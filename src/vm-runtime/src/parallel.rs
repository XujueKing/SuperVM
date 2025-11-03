//! 并行执行引擎
//! 
//! 提供交易并行执行、冲突检测、状态管理等功能

use std::collections::{HashMap, HashSet};
use std::sync::{Arc, Mutex};

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

/// 并行执行调度器
/// 
/// 管理交易的并行执行,确保正确性
pub struct ParallelScheduler {
    /// 冲突检测器
    detector: Arc<Mutex<ConflictDetector>>,
    /// 已完成的交易
    completed: Arc<Mutex<HashSet<TxId>>>,
}

impl ParallelScheduler {
    pub fn new() -> Self {
        Self {
            detector: Arc::new(Mutex::new(ConflictDetector::new())),
            completed: Arc::new(Mutex::new(HashSet::new())),
        }
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
}
