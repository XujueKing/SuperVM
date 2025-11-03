//! 并行执行引擎
//! 
//! 提供交易并行执行、冲突检测、状态管理等功能

use std::collections::{HashMap, HashSet};
use std::sync::{Arc, Mutex};
use std::sync::atomic::{AtomicU64, Ordering};
use crossbeam_deque::{Injector, Stealer, Worker};
use rayon::prelude::*;

/// 交易标识符
pub type TxId = u64;

/// 账户地址
pub type Address = Vec<u8>;

/// 存储键
pub type StorageKey = Vec<u8>;

/// 交易执行的读写集
#[derive(Debug, Clone, Default)]
pub struct ReadWriteSet {
    /// 读取的键集合
    pub read_set: HashSet<StorageKey>,
    /// 写入的键集合
    pub write_set: HashSet<StorageKey>,
}

impl ReadWriteSet {
    pub fn new() -> Self {
        Self {
            read_set: HashSet::new(),
            write_set: HashSet::new(),
        }
    }
    
    /// 记录读操作
    pub fn add_read(&mut self, key: StorageKey) {
        self.read_set.insert(key);
    }
    
    /// 记录写操作
    pub fn add_write(&mut self, key: StorageKey) {
        self.write_set.insert(key);
    }
    
    /// 检查与另一个读写集是否冲突
    /// 
    /// 冲突条件: 
    /// - 一个事务写入的键,另一个事务读取或写入
    /// - WAW (Write-After-Write) 冲突
    /// - RAW (Read-After-Write) 冲突
    /// - WAR (Write-After-Read) 冲突
    pub fn conflicts_with(&self, other: &ReadWriteSet) -> bool {
        // WAW: 两者都写同一个键
        if !self.write_set.is_disjoint(&other.write_set) {
            return true;
        }
        
        // RAW: self 读, other 写
        if !self.read_set.is_disjoint(&other.write_set) {
            return true;
        }
        
        // WAR: self 写, other 读
        if !self.write_set.is_disjoint(&other.read_set) {
            return true;
        }
        
        false
    }
}

/// 交易执行结果
#[derive(Debug, Clone)]
pub struct ExecutionResult {
    /// 交易 ID
    pub tx_id: TxId,
    /// 执行返回值
    pub return_value: i32,
    /// 读写集
    pub read_write_set: ReadWriteSet,
    /// 生成的事件
    pub events: Vec<Vec<u8>>,
    /// 是否成功
    pub success: bool,
    /// 错误信息 (如果失败)
    pub error: Option<String>,
}

/// 交易依赖图
/// 
/// 用于分析交易之间的依赖关系,确定哪些交易可以并行执行
#[derive(Debug)]
pub struct DependencyGraph {
    /// 交易 ID -> 依赖的交易 ID 列表
    dependencies: HashMap<TxId, Vec<TxId>>,
}

impl DependencyGraph {
    pub fn new() -> Self {
        Self {
            dependencies: HashMap::new(),
        }
    }
    
    /// 添加依赖关系: tx_id 依赖 depends_on
    pub fn add_dependency(&mut self, tx_id: TxId, depends_on: TxId) {
        self.dependencies
            .entry(tx_id)
            .or_insert_with(Vec::new)
            .push(depends_on);
    }
    
    /// 获取指定交易的所有依赖
    pub fn get_dependencies(&self, tx_id: TxId) -> Vec<TxId> {
        self.dependencies
            .get(&tx_id)
            .cloned()
            .unwrap_or_default()
    }
    
    /// 获取所有无依赖的交易(可以立即执行)
    pub fn get_ready_transactions(&self, all_txs: &[TxId], completed: &HashSet<TxId>) -> Vec<TxId> {
        all_txs
            .iter()
            .filter(|&tx_id| {
                // 如果已完成,跳过
                if completed.contains(tx_id) {
                    return false;
                }
                
                // 检查所有依赖是否已完成
                let deps = self.get_dependencies(*tx_id);
                deps.iter().all(|dep| completed.contains(dep))
            })
            .copied()
            .collect()
    }
}

/// 冲突检测器
/// 
/// 分析交易的读写集,构建依赖图
pub struct ConflictDetector {
    /// 已分析的交易读写集
    analyzed: HashMap<TxId, ReadWriteSet>,
}

impl ConflictDetector {
    pub fn new() -> Self {
        Self {
            analyzed: HashMap::new(),
        }
    }
    
    /// 记录交易的读写集
    pub fn record(&mut self, tx_id: TxId, rw_set: ReadWriteSet) {
        self.analyzed.insert(tx_id, rw_set);
    }
    
    /// 构建依赖图
    /// 
    /// 对于每个交易,找出它依赖的其他交易(即与它冲突的先前交易)
    pub fn build_dependency_graph(&self, tx_order: &[TxId]) -> DependencyGraph {
        let mut graph = DependencyGraph::new();
        
        for (i, &tx_id) in tx_order.iter().enumerate() {
            if let Some(rw_set) = self.analyzed.get(&tx_id) {
                // 检查与之前所有交易的冲突
                for &prev_tx_id in &tx_order[..i] {
                    if let Some(prev_rw_set) = self.analyzed.get(&prev_tx_id) {
                        if rw_set.conflicts_with(prev_rw_set) {
                            graph.add_dependency(tx_id, prev_tx_id);
                        }
                    }
                }
            }
        }
        
        graph
    }
    
    /// 检测两个交易是否冲突
    pub fn has_conflict(&self, tx1: TxId, tx2: TxId) -> bool {
        if let (Some(rw1), Some(rw2)) = (self.analyzed.get(&tx1), self.analyzed.get(&tx2)) {
            rw1.conflicts_with(rw2)
        } else {
            false
        }
    }
}

/// 执行统计信息
#[derive(Debug, Clone, Default)]
pub struct ExecutionStats {
    /// 执行成功的交易数
    pub successful_txs: u64,
    /// 执行失败的交易数
    pub failed_txs: u64,
    /// 回滚的交易数
    pub rollback_count: u64,
    /// 重试的交易数
    pub retry_count: u64,
    /// 检测到的冲突数
    pub conflict_count: u64,
}

impl ExecutionStats {
    pub fn new() -> Self {
        Self::default()
    }
    
    /// 获取总交易数
    pub fn total_txs(&self) -> u64 {
        self.successful_txs + self.failed_txs
    }
    
    /// 计算成功率
    pub fn success_rate(&self) -> f64 {
        let total = self.total_txs();
        if total == 0 {
            0.0
        } else {
            self.successful_txs as f64 / total as f64
        }
    }
    
    /// 计算回滚率
    pub fn rollback_rate(&self) -> f64 {
        let total = self.total_txs();
        if total == 0 {
            0.0
        } else {
            self.rollback_count as f64 / total as f64
        }
    }
}

/// 并行执行调度器
/// 
/// 管理交易的并行执行,确保正确性
pub struct ParallelScheduler {
    /// 冲突检测器
    detector: Arc<Mutex<ConflictDetector>>,
    /// 已完成的交易
    completed: Arc<Mutex<HashSet<TxId>>>,
    /// 状态管理器
    state_manager: Arc<Mutex<StateManager>>,
    /// 执行统计 (原子计数器)
    stats_successful: Arc<AtomicU64>,
    stats_failed: Arc<AtomicU64>,
    stats_rollback: Arc<AtomicU64>,
    stats_retry: Arc<AtomicU64>,
    stats_conflict: Arc<AtomicU64>,
}

impl ParallelScheduler {
    pub fn new() -> Self {
        Self {
            detector: Arc::new(Mutex::new(ConflictDetector::new())),
            completed: Arc::new(Mutex::new(HashSet::new())),
            state_manager: Arc::new(Mutex::new(StateManager::new())),
            stats_successful: Arc::new(AtomicU64::new(0)),
            stats_failed: Arc::new(AtomicU64::new(0)),
            stats_rollback: Arc::new(AtomicU64::new(0)),
            stats_retry: Arc::new(AtomicU64::new(0)),
            stats_conflict: Arc::new(AtomicU64::new(0)),
        }
    }
    
    /// 获取执行统计信息
    pub fn get_stats(&self) -> ExecutionStats {
        ExecutionStats {
            successful_txs: self.stats_successful.load(Ordering::Relaxed),
            failed_txs: self.stats_failed.load(Ordering::Relaxed),
            rollback_count: self.stats_rollback.load(Ordering::Relaxed),
            retry_count: self.stats_retry.load(Ordering::Relaxed),
            conflict_count: self.stats_conflict.load(Ordering::Relaxed),
        }
    }
    
    /// 重置统计信息
    pub fn reset_stats(&self) {
        self.stats_successful.store(0, Ordering::Relaxed);
        self.stats_failed.store(0, Ordering::Relaxed);
        self.stats_rollback.store(0, Ordering::Relaxed);
        self.stats_retry.store(0, Ordering::Relaxed);
        self.stats_conflict.store(0, Ordering::Relaxed);
    }
    
    /// 记录交易完成
    pub fn mark_completed(&self, tx_id: TxId) {
        self.completed.lock().unwrap().insert(tx_id);
    }
    
    /// 记录交易的读写集
    pub fn record_rw_set(&self, tx_id: TxId, rw_set: ReadWriteSet) {
        self.detector.lock().unwrap().record(tx_id, rw_set);
    }
    
    /// 获取可并行执行的交易
    pub fn get_parallel_batch(&self, all_txs: &[TxId]) -> Vec<TxId> {
        let detector = self.detector.lock().unwrap();
        let completed = self.completed.lock().unwrap();
        
        let graph = detector.build_dependency_graph(all_txs);
        graph.get_ready_transactions(all_txs, &completed)
    }
    
    /// 获取状态管理器的引用
    pub fn get_state_manager(&self) -> Arc<Mutex<StateManager>> {
        Arc::clone(&self.state_manager)
    }
    
    /// 在快照保护下执行操作
    /// 
    /// 该方法会:
    /// 1. 创建状态快照
    /// 2. 执行提供的操作
    /// 3. 如果操作成功,提交快照并更新统计
    /// 4. 如果操作失败,回滚到快照并更新统计
    /// 
    /// # 参数
    /// - `operation`: 要执行的操作,返回 Result<T, String>
    /// 
    /// # 返回
    /// - `Ok(T)`: 操作成功的结果
    /// - `Err(String)`: 操作失败的错误信息
    pub fn execute_with_snapshot<T, F>(&self, operation: F) -> Result<T, String>
    where
        F: FnOnce(&StateManager) -> Result<T, String>,
    {
        let mut manager = self.state_manager.lock().unwrap();
        
        // 创建快照
        manager.create_snapshot()?;
        
        // 执行操作
        let result = operation(&manager);
        
        match result {
            Ok(value) => {
                // 操作成功,提交快照
                manager.commit()?;
                self.stats_successful.fetch_add(1, Ordering::Relaxed);
                Ok(value)
            }
            Err(err) => {
                // 操作失败,回滚
                manager.rollback()?;
                self.stats_failed.fetch_add(1, Ordering::Relaxed);
                self.stats_rollback.fetch_add(1, Ordering::Relaxed);
                Err(err)
            }
        }
    }
    
    /// 带重试机制的事务执行
    /// 
    /// 在交易失败时自动重试,最多重试 max_retries 次
    /// 
    /// # 参数
    /// - `operation`: 要执行的操作
    /// - `max_retries`: 最大重试次数
    /// 
    /// # 返回
    /// - `Ok(T)`: 操作成功的结果
    /// - `Err(String)`: 所有重试都失败后的错误信息
    pub fn execute_with_retry<T, F>(&self, mut operation: F, max_retries: u32) -> Result<T, String>
    where
        F: FnMut(&StateManager) -> Result<T, String>,
    {
        let mut attempts = 0;
        let mut last_error = String::new();
        
        while attempts <= max_retries {
            if attempts > 0 {
                self.stats_retry.fetch_add(1, Ordering::Relaxed);
            }
            
            let result = self.execute_with_snapshot(|manager| operation(manager));
            
            match result {
                Ok(value) => return Ok(value),
                Err(err) => {
                    last_error = err;
                    attempts += 1;
                }
            }
        }
        
        Err(format!("Failed after {} attempts: {}", attempts, last_error))
    }
    
    /// 获取当前存储状态
    pub fn get_storage(&self) -> Arc<Mutex<HashMap<Vec<u8>, Vec<u8>>>> {
        let manager = self.state_manager.lock().unwrap();
        manager.get_storage()
    }
    
    /// 获取当前事件列表
    pub fn get_events(&self) -> Arc<Mutex<Vec<Vec<u8>>>> {
        let manager = self.state_manager.lock().unwrap();
        manager.get_events()
    }
}

/// 工作窃取任务
#[derive(Debug, Clone)]
pub struct Task {
    pub tx_id: TxId,
    pub priority: u8,  // 任务优先级 (0-255, 值越大优先级越高)
}

impl Task {
    pub fn new(tx_id: TxId, priority: u8) -> Self {
        Self { tx_id, priority }
    }
}

/// 工作窃取调度器
/// 
/// 使用工作窃取算法进行负载均衡:
/// - 每个工作线程有自己的本地队列 (Worker)
/// - 空闲线程可以从其他线程的队列"窃取"任务 (Stealer)
/// - 全局队列 (Injector) 用于任务注入和负载均衡
pub struct WorkStealingScheduler {
    /// 全局任务队列
    injector: Arc<Injector<Task>>,
    /// 底层并行调度器
    scheduler: Arc<ParallelScheduler>,
    /// 工作线程数量
    num_workers: usize,
}

impl WorkStealingScheduler {
    /// 创建新的工作窃取调度器
    /// 
    /// # 参数
    /// - `num_workers`: 工作线程数量,默认使用 CPU 核心数
    pub fn new(num_workers: Option<usize>) -> Self {
        let num_workers = num_workers.unwrap_or_else(|| num_cpus::get());
        
        Self {
            injector: Arc::new(Injector::new()),
            scheduler: Arc::new(ParallelScheduler::new()),
            num_workers,
        }
    }
    
    /// 提交任务到全局队列
    pub fn submit_task(&self, task: Task) {
        self.injector.push(task);
    }
    
    /// 批量提交任务
    pub fn submit_tasks(&self, tasks: Vec<Task>) {
        for task in tasks {
            self.injector.push(task);
        }
    }
    
    /// 执行所有任务
    /// 
    /// 使用 rayon 线程池并行执行任务,每个线程:
    /// 1. 从自己的本地队列获取任务
    /// 2. 如果本地队列为空,尝试从全局队列获取
    /// 3. 如果全局队列也为空,从其他线程窃取任务
    pub fn execute_all<F>(&self, executor: F) -> Result<Vec<TxId>, String>
    where
        F: Fn(TxId) -> Result<(), String> + Send + Sync,
    {
        let executed = Arc::new(Mutex::new(Vec::new()));
        let errors = Arc::new(Mutex::new(Vec::new()));
        
        // 创建工作线程和它们的本地队列
        let workers: Vec<Worker<Task>> = (0..self.num_workers)
            .map(|_| Worker::new_fifo())
            .collect();
        
        // 收集窃取器
        let stealers: Vec<Stealer<Task>> = workers
            .iter()
            .map(|w| w.stealer())
            .collect();
        
        let injector = Arc::clone(&self.injector);
        let executor = Arc::new(executor);
        
        // 使用 rayon 并行执行
        workers.into_par_iter().enumerate().for_each(|(worker_id, worker)| {
            let executed = Arc::clone(&executed);
            let errors = Arc::clone(&errors);
            let executor = Arc::clone(&executor);
            
            loop {
                // 尝试从本地队列获取任务
                let task = worker.pop().or_else(|| {
                    // 本地队列为空,尝试从全局队列获取
                    loop {
                        match injector.steal_batch_and_pop(&worker) {
                            crossbeam_deque::Steal::Success(t) => return Some(t),
                            crossbeam_deque::Steal::Empty => break,
                            crossbeam_deque::Steal::Retry => continue,
                        }
                    }
                    
                    // 全局队列也为空,尝试从其他线程窃取
                    stealers.iter().enumerate()
                        .filter(|(id, _)| *id != worker_id)
                        .find_map(|(_, stealer)| {
                            loop {
                                match stealer.steal_batch_and_pop(&worker) {
                                    crossbeam_deque::Steal::Success(t) => return Some(t),
                                    crossbeam_deque::Steal::Empty => return None,
                                    crossbeam_deque::Steal::Retry => continue,
                                }
                            }
                        })
                });
                
                match task {
                    Some(task) => {
                        // 执行任务
                        match executor(task.tx_id) {
                            Ok(()) => {
                                executed.lock().unwrap().push(task.tx_id);
                            }
                            Err(e) => {
                                errors.lock().unwrap().push((task.tx_id, e));
                            }
                        }
                    }
                    None => break, // 没有更多任务
                }
            }
        });
        
        // 检查是否有错误
        let error_list = errors.lock().unwrap();
        if !error_list.is_empty() {
            return Err(format!("Execution failed for {} tasks", error_list.len()));
        }
        
        let result = executed.lock().unwrap().clone();
        Ok(result)
    }
    
    /// 获取底层并行调度器
    pub fn get_scheduler(&self) -> Arc<ParallelScheduler> {
        Arc::clone(&self.scheduler)
    }
    
    /// 获取执行统计
    pub fn get_stats(&self) -> ExecutionStats {
        self.scheduler.get_stats()
    }
}

// 添加 num_cpus 的简单实现 (如果没有依赖)
mod num_cpus {
    use std::thread;
    
    pub fn get() -> usize {
        thread::available_parallelism()
            .map(|n| n.get())
            .unwrap_or(4) // 默认 4 个线程
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_read_write_set_conflicts() {
        let mut rw1 = ReadWriteSet::new();
        rw1.add_read(b"key1".to_vec());
        rw1.add_write(b"key2".to_vec());
        
        let mut rw2 = ReadWriteSet::new();
        rw2.add_read(b"key2".to_vec()); // 读 key2,与 rw1 的写冲突
        
        assert!(rw1.conflicts_with(&rw2));
        assert!(rw2.conflicts_with(&rw1));
    }
    
    #[test]
    fn test_no_conflict() {
        let mut rw1 = ReadWriteSet::new();
        rw1.add_read(b"key1".to_vec());
        rw1.add_write(b"key1".to_vec());
        
        let mut rw2 = ReadWriteSet::new();
        rw2.add_read(b"key2".to_vec());
        rw2.add_write(b"key2".to_vec());
        
        assert!(!rw1.conflicts_with(&rw2));
    }
    
    #[test]
    fn test_dependency_graph() {
        let mut graph = DependencyGraph::new();
        graph.add_dependency(2, 1); // tx2 依赖 tx1
        graph.add_dependency(3, 1); // tx3 依赖 tx1
        graph.add_dependency(4, 2); // tx4 依赖 tx2
        
        let all_txs = vec![1, 2, 3, 4];
        let mut completed = HashSet::new();
        
        // 初始只有 tx1 可以执行
        let ready = graph.get_ready_transactions(&all_txs, &completed);
        assert_eq!(ready, vec![1]);
        
        // tx1 完成后,tx2 和 tx3 可以并行执行
        completed.insert(1);
        let ready = graph.get_ready_transactions(&all_txs, &completed);
        assert!(ready.contains(&2) && ready.contains(&3));
        
        // tx2 完成后,tx4 可以执行
        completed.insert(2);
        completed.insert(3);
        let ready = graph.get_ready_transactions(&all_txs, &completed);
        assert_eq!(ready, vec![4]);
    }
    
    #[test]
    fn test_conflict_detector() {
        let mut detector = ConflictDetector::new();
        
        let mut rw1 = ReadWriteSet::new();
        rw1.add_write(b"balance_alice".to_vec());
        detector.record(1, rw1);
        
        let mut rw2 = ReadWriteSet::new();
        rw2.add_write(b"balance_bob".to_vec());
        detector.record(2, rw2);
        
        let mut rw3 = ReadWriteSet::new();
        rw3.add_read(b"balance_alice".to_vec()); // 与 tx1 冲突
        detector.record(3, rw3);
        
        // 构建依赖图
        let tx_order = vec![1, 2, 3];
        let graph = detector.build_dependency_graph(&tx_order);
        
        // tx1 和 tx2 无依赖,可并行
        assert_eq!(graph.get_dependencies(1).len(), 0);
        assert_eq!(graph.get_dependencies(2).len(), 0);
        
        // tx3 依赖 tx1
        assert_eq!(graph.get_dependencies(3), vec![1]);
    }
    
    #[test]
    fn test_scheduler_with_snapshot() {
        let scheduler = ParallelScheduler::new();
        
        // 成功的操作
        let result = scheduler.execute_with_snapshot(|manager| {
            let storage_arc = manager.get_storage();
            let mut storage = storage_arc.lock().unwrap();
            storage.insert(b"key".to_vec(), b"value".to_vec());
            Ok(42)
        });
        
        assert_eq!(result, Ok(42));
        
        // 验证状态已保存
        let storage_arc = scheduler.get_storage();
        let storage = storage_arc.lock().unwrap();
        assert_eq!(storage.get(&b"key".to_vec()), Some(&b"value".to_vec()));
    }
    
    #[test]
    fn test_scheduler_rollback_on_error() {
        let scheduler = ParallelScheduler::new();
        
        // 先设置初始状态
        {
            let storage_arc = scheduler.get_storage();
            let mut storage = storage_arc.lock().unwrap();
            storage.insert(b"balance".to_vec(), b"100".to_vec());
        }
        
        // 执行会失败的操作
        let result: Result<(), String> = scheduler.execute_with_snapshot(|manager| {
            let storage_arc = manager.get_storage();
            let mut storage = storage_arc.lock().unwrap();
            storage.insert(b"balance".to_vec(), b"50".to_vec());
            
            // 模拟失败
            Err("Insufficient funds".to_string())
        });
        
        assert!(result.is_err());
        
        // 验证状态已回滚
        let storage_arc = scheduler.get_storage();
        let storage = storage_arc.lock().unwrap();
        assert_eq!(storage.get(&b"balance".to_vec()), Some(&b"100".to_vec()));
    }
    
    #[test]
    fn test_scheduler_nested_transactions() {
        let scheduler = ParallelScheduler::new();
        
        // 第一个事务成功
        scheduler.execute_with_snapshot(|manager| {
            let storage_arc = manager.get_storage();
            let mut storage = storage_arc.lock().unwrap();
            storage.insert(b"user1".to_vec(), b"data1".to_vec());
            Ok(())
        }).unwrap();
        
        // 第二个事务失败
        let result: Result<(), String> = scheduler.execute_with_snapshot(|manager| {
            let storage_arc = manager.get_storage();
            let mut storage = storage_arc.lock().unwrap();
            storage.insert(b"user2".to_vec(), b"data2".to_vec());
            Err("Transaction failed".to_string())
        });
        
        assert!(result.is_err());
        
        // 验证只有第一个事务的数据存在
        let storage_arc = scheduler.get_storage();
        let storage = storage_arc.lock().unwrap();
        assert_eq!(storage.get(&b"user1".to_vec()), Some(&b"data1".to_vec()));
        assert_eq!(storage.get(&b"user2".to_vec()), None);
    }
    
    #[test]
    fn test_execution_stats() {
        let scheduler = ParallelScheduler::new();
        
        // 执行一些成功的交易
        for i in 0..3 {
            scheduler.execute_with_snapshot(|manager| {
                let storage_arc = manager.get_storage();
                let mut storage = storage_arc.lock().unwrap();
                storage.insert(format!("key{}", i).into_bytes(), b"value".to_vec());
                Ok(())
            }).unwrap();
        }
        
        // 执行一些失败的交易
        for _ in 0..2 {
            let _: Result<(), String> = scheduler.execute_with_snapshot(|_manager| {
                Err("Test error".to_string())
            });
        }
        
        let stats = scheduler.get_stats();
        assert_eq!(stats.successful_txs, 3);
        assert_eq!(stats.failed_txs, 2);
        assert_eq!(stats.rollback_count, 2);
        assert_eq!(stats.total_txs(), 5);
        assert_eq!(stats.success_rate(), 0.6);
    }
    
    #[test]
    fn test_retry_mechanism() {
        let scheduler = ParallelScheduler::new();
        
        let mut attempt_count = 0;
        
        // 模拟前两次失败,第三次成功
        let result = scheduler.execute_with_retry(
            |manager| {
                attempt_count += 1;
                
                if attempt_count < 3 {
                    Err(format!("Attempt {} failed", attempt_count))
                } else {
                    let storage_arc = manager.get_storage();
                    let mut storage = storage_arc.lock().unwrap();
                    storage.insert(b"key".to_vec(), b"success".to_vec());
                    Ok(42)
                }
            },
            5, // max_retries
        );
        
        assert_eq!(result, Ok(42));
        assert_eq!(attempt_count, 3);
        
        let stats = scheduler.get_stats();
        assert_eq!(stats.retry_count, 2); // 重试了 2 次
        assert_eq!(stats.successful_txs, 1);
        assert_eq!(stats.rollback_count, 2); // 前两次失败回滚
    }
    
    #[test]
    fn test_retry_exhausted() {
        let scheduler = ParallelScheduler::new();
        
        // 模拟总是失败
        let result: Result<i32, String> = scheduler.execute_with_retry(
            |_manager| Err("Always fails".to_string()),
            3, // max_retries
        );
        
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Failed after 4 attempts"));
        
        let stats = scheduler.get_stats();
        assert_eq!(stats.retry_count, 3);
        assert_eq!(stats.failed_txs, 4); // 初次 + 3 次重试
        assert_eq!(stats.rollback_count, 4);
    }
}

// ============================================
// 状态快照与回滚
// ============================================

/// 存储状态快照
/// 
/// 用于在交易执行失败时回滚到之前的状态
#[derive(Debug, Clone)]
pub struct StorageSnapshot {
    /// 快照时的存储状态 (key -> value)
    storage_state: HashMap<Vec<u8>, Vec<u8>>,
    /// 快照时的事件列表
    events: Vec<Vec<u8>>,
}

impl StorageSnapshot {
    /// 创建空快照
    pub fn new() -> Self {
        Self {
            storage_state: HashMap::new(),
            events: Vec::new(),
        }
    }
    
    /// 从当前状态创建快照
    pub fn from_storage(storage: &HashMap<Vec<u8>, Vec<u8>>, events: &[Vec<u8>]) -> Self {
        Self {
            storage_state: storage.clone(),
            events: events.to_vec(),
        }
    }
    
    /// 获取快照的存储状态
    pub fn get_storage_state(&self) -> &HashMap<Vec<u8>, Vec<u8>> {
        &self.storage_state
    }
    
    /// 获取快照的事件列表
    pub fn get_events(&self) -> &[Vec<u8>] {
        &self.events
    }
}

/// 状态管理器
/// 
/// 管理存储状态的快照和回滚
#[derive(Debug)]
pub struct StateManager {
    /// 当前存储状态
    current_storage: Arc<Mutex<HashMap<Vec<u8>, Vec<u8>>>>,
    /// 当前事件列表
    current_events: Arc<Mutex<Vec<Vec<u8>>>>,
    /// 快照栈 (支持嵌套事务)
    snapshots: Vec<StorageSnapshot>,
}

impl StateManager {
    /// 创建新的状态管理器
    pub fn new() -> Self {
        Self {
            current_storage: Arc::new(Mutex::new(HashMap::new())),
            current_events: Arc::new(Mutex::new(Vec::new())),
            snapshots: Vec::new(),
        }
    }
    
    /// 从现有状态创建管理器
    pub fn from_storage(storage: HashMap<Vec<u8>, Vec<u8>>) -> Self {
        Self {
            current_storage: Arc::new(Mutex::new(storage)),
            current_events: Arc::new(Mutex::new(Vec::new())),
            snapshots: Vec::new(),
        }
    }
    
    /// 创建当前状态的快照
    pub fn create_snapshot(&mut self) -> Result<(), String> {
        let storage = self.current_storage.lock()
            .map_err(|e| format!("Failed to lock storage: {}", e))?;
        let events = self.current_events.lock()
            .map_err(|e| format!("Failed to lock events: {}", e))?;
        
        let snapshot = StorageSnapshot::from_storage(&storage, &events);
        self.snapshots.push(snapshot);
        
        Ok(())
    }
    
    /// 回滚到最近的快照
    pub fn rollback(&mut self) -> Result<(), String> {
        if let Some(snapshot) = self.snapshots.pop() {
            // 恢复存储状态
            let mut storage = self.current_storage.lock()
                .map_err(|e| format!("Failed to lock storage: {}", e))?;
            *storage = snapshot.get_storage_state().clone();
            
            // 恢复事件列表
            let mut events = self.current_events.lock()
                .map_err(|e| format!("Failed to lock events: {}", e))?;
            *events = snapshot.get_events().to_vec();
            
            Ok(())
        } else {
            Err("No snapshot available to rollback".to_string())
        }
    }
    
    /// 提交当前状态 (丢弃最近的快照)
    pub fn commit(&mut self) -> Result<(), String> {
        if self.snapshots.pop().is_some() {
            Ok(())
        } else {
            Err("No snapshot available to commit".to_string())
        }
    }
    
    /// 获取当前存储状态的引用
    pub fn get_storage(&self) -> Arc<Mutex<HashMap<Vec<u8>, Vec<u8>>>> {
        Arc::clone(&self.current_storage)
    }
    
    /// 获取当前事件列表的引用
    pub fn get_events(&self) -> Arc<Mutex<Vec<Vec<u8>>>> {
        Arc::clone(&self.current_events)
    }
    
    /// 获取快照深度 (当前有多少个快照)
    pub fn snapshot_depth(&self) -> usize {
        self.snapshots.len()
    }
}

#[cfg(test)]
mod snapshot_tests {
    use super::*;
    
    #[test]
    fn test_snapshot_creation() {
        let mut manager = StateManager::new();
        
        // 添加一些数据
        {
            let storage_arc = manager.get_storage();
            let mut storage = storage_arc.lock().unwrap();
            storage.insert(b"key1".to_vec(), b"value1".to_vec());
            storage.insert(b"key2".to_vec(), b"value2".to_vec());
        }
        
        // 创建快照
        manager.create_snapshot().unwrap();
        assert_eq!(manager.snapshot_depth(), 1);
        
        // 修改数据
        {
            let storage_arc = manager.get_storage();
            let mut storage = storage_arc.lock().unwrap();
            storage.insert(b"key1".to_vec(), b"new_value".to_vec());
            storage.insert(b"key3".to_vec(), b"value3".to_vec());
        }
        
        // 验证修改生效
        {
            let storage_arc = manager.get_storage();
            let storage = storage_arc.lock().unwrap();
            assert_eq!(storage.get(&b"key1".to_vec()), Some(&b"new_value".to_vec()));
            assert_eq!(storage.len(), 3);
        }
    }
    
    #[test]
    fn test_rollback() {
        let mut manager = StateManager::new();
        
        // 初始状态
        {
            let storage_arc = manager.get_storage();
            let mut storage = storage_arc.lock().unwrap();
            storage.insert(b"key1".to_vec(), b"value1".to_vec());
        }
        
        // 创建快照
        manager.create_snapshot().unwrap();
        
        // 修改数据
        {
            let storage_arc = manager.get_storage();
            let mut storage = storage_arc.lock().unwrap();
            storage.insert(b"key1".to_vec(), b"modified".to_vec());
            storage.insert(b"key2".to_vec(), b"value2".to_vec());
        }
        
        // 回滚
        manager.rollback().unwrap();
        
        // 验证回滚成功
        {
            let storage_arc = manager.get_storage();
            let storage = storage_arc.lock().unwrap();
            assert_eq!(storage.get(&b"key1".to_vec()), Some(&b"value1".to_vec()));
            assert_eq!(storage.get(&b"key2".to_vec()), None); // key2 应该不存在
            assert_eq!(storage.len(), 1);
        }
        assert_eq!(manager.snapshot_depth(), 0);
    }
    
    #[test]
    fn test_nested_snapshots() {
        let mut manager = StateManager::new();
        
        // 第一层状态
        {
            let storage_arc = manager.get_storage();
            let mut storage = storage_arc.lock().unwrap();
            storage.insert(b"key".to_vec(), b"v1".to_vec());
        }
        manager.create_snapshot().unwrap();
        
        // 第二层状态
        {
            let storage_arc = manager.get_storage();
            let mut storage = storage_arc.lock().unwrap();
            storage.insert(b"key".to_vec(), b"v2".to_vec());
        }
        manager.create_snapshot().unwrap();
        
        // 第三层状态
        {
            let storage_arc = manager.get_storage();
            let mut storage = storage_arc.lock().unwrap();
            storage.insert(b"key".to_vec(), b"v3".to_vec());
        }
        
        assert_eq!(manager.snapshot_depth(), 2);
        
        // 回滚到第二层
        manager.rollback().unwrap();
        {
            let storage_arc = manager.get_storage();
            let storage = storage_arc.lock().unwrap();
            assert_eq!(storage.get(&b"key".to_vec()), Some(&b"v2".to_vec()));
        }
        
        // 回滚到第一层
        manager.rollback().unwrap();
        {
            let storage_arc = manager.get_storage();
            let storage = storage_arc.lock().unwrap();
            assert_eq!(storage.get(&b"key".to_vec()), Some(&b"v1".to_vec()));
        }
        
        assert_eq!(manager.snapshot_depth(), 0);
    }
    
    #[test]
    fn test_commit() {
        let mut manager = StateManager::new();
        
        {
            let storage_arc = manager.get_storage();
            let mut storage = storage_arc.lock().unwrap();
            storage.insert(b"key".to_vec(), b"value1".to_vec());
        }
        manager.create_snapshot().unwrap();
        
        {
            let storage_arc = manager.get_storage();
            let mut storage = storage_arc.lock().unwrap();
            storage.insert(b"key".to_vec(), b"value2".to_vec());
        }
        
        // 提交 (丢弃快照但保留当前状态)
        manager.commit().unwrap();
        assert_eq!(manager.snapshot_depth(), 0);
        
        // 当前状态应该保持 value2
        {
            let storage_arc = manager.get_storage();
            let storage = storage_arc.lock().unwrap();
            assert_eq!(storage.get(&b"key".to_vec()), Some(&b"value2".to_vec()));
        }
        
        // 无法回滚
        assert!(manager.rollback().is_err());
    }
    
    #[test]
    fn test_snapshot_with_events() {
        let mut manager = StateManager::new();
        
        // 添加事件
        {
            let events_arc = manager.get_events();
            let mut events = events_arc.lock().unwrap();
            events.push(b"event1".to_vec());
        }
        
        manager.create_snapshot().unwrap();
        
        // 添加更多事件
        {
            let events_arc = manager.get_events();
            let mut events = events_arc.lock().unwrap();
            events.push(b"event2".to_vec());
            events.push(b"event3".to_vec());
        }
        
        // 回滚
        manager.rollback().unwrap();
        
        // 验证只有 event1 存在
        {
            let events_arc = manager.get_events();
            let events = events_arc.lock().unwrap();
            assert_eq!(events.len(), 1);
            assert_eq!(events[0], b"event1");
        }
    }
    
    #[test]
    fn test_work_stealing_basic() {
        let ws_scheduler = WorkStealingScheduler::new(Some(4));
        
        // 提交 10 个任务
        let tasks: Vec<Task> = (1..=10)
            .map(|i| Task::new(i, 100))
            .collect();
        
        ws_scheduler.submit_tasks(tasks);
        
        // 执行所有任务
        let executed_count = Arc::new(std::sync::atomic::AtomicU64::new(0));
        let executed_count_clone = Arc::clone(&executed_count);
        
        let result = ws_scheduler.execute_all(move |tx_id| {
            executed_count_clone.fetch_add(1, Ordering::Relaxed);
            println!("Executing tx {}", tx_id);
            Ok(())
        });
        
        assert!(result.is_ok());
        assert_eq!(executed_count.load(Ordering::Relaxed), 10);
    }
    
    #[test]
    fn test_work_stealing_with_priorities() {
        let ws_scheduler = WorkStealingScheduler::new(Some(2));
        
        // 提交不同优先级的任务
        ws_scheduler.submit_task(Task::new(1, 255)); // 高优先级
        ws_scheduler.submit_task(Task::new(2, 128)); // 中优先级
        ws_scheduler.submit_task(Task::new(3, 50));  // 低优先级
        
        let executed = Arc::new(Mutex::new(Vec::new()));
        let executed_clone = Arc::clone(&executed);
        
        let result = ws_scheduler.execute_all(move |tx_id| {
            executed_clone.lock().unwrap().push(tx_id);
            Ok(())
        });
        
        assert!(result.is_ok());
        assert_eq!(executed.lock().unwrap().len(), 3);
    }
    
    #[test]
    fn test_work_stealing_with_errors() {
        let ws_scheduler = WorkStealingScheduler::new(Some(2));
        
        // 提交会失败的任务
        ws_scheduler.submit_task(Task::new(1, 100));
        ws_scheduler.submit_task(Task::new(2, 100));
        
        let result = ws_scheduler.execute_all(|tx_id| {
            if tx_id == 2 {
                Err("Simulated error".to_string())
            } else {
                Ok(())
            }
        });
        
        // 应该返回错误
        assert!(result.is_err());
    }
}
