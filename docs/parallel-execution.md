# 并行执行引擎设计文档

作者: king  
版本: v0.9.0  
最后更新: 2025-11-05

## 目录

- [概述](#概述)
- [架构设计](#架构设计)
- [核心组件](#核心组件)
- [两种调度器对比](#两种调度器对比)
- [关键 API](#关键-api)
- [使用示例](#使用示例)
- [性能优化](#性能优化)
- [测试与基准](#测试与基准)
- [最佳实践](#最佳实践)

---

## 概述

SuperVM 并行执行引擎旨在提高区块链交易处理吞吐量，通过智能冲突检测和依赖分析，在保证正确性的前提下最大化并行执行效率。

### 设计目标

 **正确性优先**: 严格保证事务执行顺序和一致性  
 **高吞吐量**: 充分利用多核并行能力  
 **自动恢复**: 冲突事务自动回滚与重试  
 **可观测性**: 完整的执行统计和性能指标  
 **灵活配置**: 支持不同工作负载的调优

### 核心特性

- **双引擎架构**: 提供 ParallelScheduler 和 MvccScheduler 两种实现
- **智能冲突检测**: 自动分析读写集，构建依赖图
- **工作窃取调度**: 动态负载均衡，最大化 CPU 利用率
- **快照隔离**: 基于 MVCC 的事务隔离级别
- **自适应 GC**: 自动内存管理，无需手动调优

---

## 架构设计

### 总体流程

\\\
交易输入  读写集提取  冲突检测  依赖图构建  并行调度  执行完成
\\\

\\\
                  
        Tx Input      RWSet Build    Conflict Check
                  
                                               
                                               
                          
           Dependency DAG   Parallel Sched
                          
                                                    
                                                    
                                          
                                           Execute Results  
                                          
\\\

### 架构层次

\\\

         应用层 (Transaction Executor)            

    调度层 (ParallelScheduler / MvccScheduler)   

       存储层 (StateManager / MvccStore)         

           基础设施 (Rayon / Crossbeam)          

\\\

---

## 核心组件

### 1. ReadWriteSet (读写集)

记录事务访问的键集合，用于冲突检测。

\\\ust
pub struct ReadWriteSet {
    pub read_set: HashSet<StorageKey>,
    pub write_set: HashSet<StorageKey>,
}
\\\

**功能**:
- dd_read(key): 记录读操作
- dd_write(key): 记录写操作
- conflicts_with(other): 检测与另一个读写集的冲突

**冲突规则**:
- **WAW** (Write-After-Write): 两个事务写同一个键
- **RAW** (Read-After-Write): 一个事务读，另一个写
- **WAR** (Write-After-Read): 一个事务写，另一个读

### 2. ConflictDetector (冲突检测器)

分析交易读写集，构建依赖关系。

\\\ust
pub struct ConflictDetector {
    analyzed: HashMap<TxId, ReadWriteSet>,
}
\\\

**核心方法**:
- ecord(tx_id, rw_set): 记录交易的读写集
- uild_dependency_graph(tx_order): 构建依赖图
- has_conflict(tx1, tx2): 检测两个交易是否冲突

**依赖图构建算法**:
\\\ust
for (i, tx_id) in tx_order.iter().enumerate() {
    // 检查与之前所有交易的冲突
    for prev_tx_id in &tx_order[..i] {
        if rw_set.conflicts_with(prev_rw_set) {
            // tx_id 依赖 prev_tx_id
            graph.add_dependency(tx_id, prev_tx_id);
        }
    }
}
\\\

### 3. DependencyGraph (依赖图)

DAG (有向无环图) 表示交易之间的依赖关系。

\\\ust
pub struct DependencyGraph {
    dependencies: HashMap<TxId, Vec<TxId>>,
}
\\\

**核心方法**:
- dd_dependency(tx, depends_on): 添加依赖关系
- get_dependencies(tx): 获取指定交易的所有依赖
- get_ready_transactions(all_txs, completed): 获取可立即执行的交易

**调度逻辑**:
\\\ust
// 获取无依赖或依赖已完成的交易
ready_txs = all_txs.filter(|tx| {
    deps = graph.get_dependencies(tx)
    deps.all(|dep| completed.contains(dep))
})
\\\

### 4. StateManager (状态管理器)

管理交易执行状态的快照和回滚。

\\\ust
pub struct StateManager {
    snapshots: HashMap<SnapshotId, HashMap<StorageKey, Vec<u8>>>,
    current_state: HashMap<StorageKey, Vec<u8>>,
}
\\\

**核心方法**:
- create_snapshot(): 创建当前状态快照
- get(key): 读取状态
- set(key, value): 写入状态
- ollback_to_snapshot(id): 回滚到指定快照
- discard_snapshot(id): 丢弃快照释放内存

**快照机制**:
\\\ust
// 1. 执行前创建快照
snapshot_id = state_manager.create_snapshot();

// 2. 执行交易
result = execute_transaction(tx);

// 3. 根据结果处理
if result.success {
    state_manager.discard_snapshot(snapshot_id);
} else {
    state_manager.rollback_to_snapshot(snapshot_id);
}
\\\

### 5. ParallelScheduler (并行调度器)

管理交易的并行执行。

\\\ust
pub struct ParallelScheduler {
    detector: Arc<Mutex<ConflictDetector>>,
    completed: Arc<Mutex<HashSet<TxId>>>,
    state_manager: Arc<Mutex<StateManager>>,
    stats_successful: Arc<AtomicU64>,
    stats_failed: Arc<AtomicU64>,
    stats_conflict: Arc<AtomicU64>,
    mvcc_store: Option<Arc<MvccStore>>,
}
\\\

**核心方法**:
- execute_parallel(txs, executor): 并行执行交易列表
- execute_with_retry(tx_id, executor): 带重试的单交易执行
- get_parallel_batch(all_txs): 获取可并行执行的批次
- get_stats(): 获取执行统计

### 6. WorkStealingScheduler (工作窃取调度器)

基于工作窃取算法的高性能调度器。

\\\ust
pub struct WorkStealingScheduler {
    global_queue: Arc<Injector<TxId>>,
    local_queues: Vec<Worker<TxId>>,
    stealers: Vec<Stealer<TxId>>,
    scheduler: Arc<ParallelScheduler>,
}
\\\

**工作窃取原理**:
\\\
线程 1:  [Task1, Task2] steal 线程 2: [Task5]
线程 3:  [Task3, Task4, Task6] steal 线程 4: []
\\\

**优势**:
-  动态负载均衡
-  减少线程空闲时间
-  提高 CPU 利用率

### 7. MvccScheduler (MVCC 调度器)

基于 MVCC 存储的新一代调度器 (v0.9.0+)。

\\\ust
pub struct MvccScheduler {
    store: Arc<MvccStore>,
    config: MvccSchedulerConfig,
    stats_successful: Arc<AtomicU64>,
    stats_failed: Arc<AtomicU64>,
    stats_conflict: Arc<AtomicU64>,
}
\\\

**核心优势**:
-  使用 MVCC 内置冲突检测，无需手动 ConflictDetector
-  快照隔离自动处理，无需 StateManager
-  自适应 GC 自动管理内存
-  更好的并发性能和正确性保证

---

## 两种调度器对比

### ParallelScheduler vs MvccScheduler

| 特性 | ParallelScheduler | MvccScheduler |
|------|------------------|---------------|
| **版本** | v0.7.0 | v0.9.0+ |
| **存储后端** | StateManager (可选 MVCC) | MvccStore (必须) |
| **冲突检测** | 手动 ConflictDetector | MVCC 自动检测 |
| **快照管理** | 手动 Snapshot | MVCC 自动快照 |
| **内存管理** | 手动释放 | 自适应 GC |
| **并发模型** | 依赖图 + 批次执行 | MVCC 事务 + 并行提交 |
| **性能** | 良好 | 更优 |
| **适用场景** | 通用并行执行 | MVCC 原生场景 |

### 选择指南

**使用 ParallelScheduler**:
-  需要灵活的存储后端
-  自定义冲突检测逻辑
-  不依赖 MVCC

**使用 MvccScheduler** (推荐):
-  追求最佳性能
-  需要事务隔离
-  自动内存管理
-  生产环境部署

---

## 关键 API

### ParallelScheduler API

#### 创建调度器

\\\ust
use vm_runtime::ParallelScheduler;

// 默认配置
let scheduler = ParallelScheduler::new();

// 使用 MVCC 后端
let mvcc_store = MvccStore::new_with_config(gc_config);
let scheduler = ParallelScheduler::with_mvcc_store(mvcc_store);
\\\

#### 并行执行

\\\ust
// 定义执行器
let executor = |tx_id: TxId, state: &StateManager| -> Result<ExecutionResult> {
    // 执行交易逻辑
    let value = state.get(&key)?;
    state.set(&key, new_value)?;
    
    Ok(ExecutionResult {
        tx_id,
        return_value: 0,
        success: true,
        ..Default::default()
    })
};

// 并行执行交易列表
let tx_ids: Vec<TxId> = vec![1, 2, 3, 4, 5];
let results = scheduler.execute_parallel(&tx_ids, executor)?;

// 处理结果
for result in results {
    println!("Tx {}: {}", result.tx_id, 
        if result.success { "成功" } else { "失败" });
}
\\\

#### 获取统计信息

\\\ust
let stats = scheduler.get_stats();

println!("成功: {}", stats.successful_txs);
println!("失败: {}", stats.failed_txs);
println!("冲突: {}", stats.conflict_count);
println!("成功率: {:.2}%", stats.success_rate() * 100.0);
\\\

---

### MvccScheduler API

#### 创建调度器

\\\ust
use vm_runtime::{MvccScheduler, MvccSchedulerConfig, GcConfig, AutoGcConfig};

// 默认配置
let scheduler = MvccScheduler::new();

// 自定义配置
let config = MvccSchedulerConfig {
    mvcc_config: GcConfig {
        max_versions_per_key: 30,
        auto_gc: Some(AutoGcConfig {
            interval_secs: 60,
            version_threshold: 2000,
            enable_adaptive: true,
            ..Default::default()
        }),
        ..Default::default()
    },
    max_retries: 5,
    num_workers: 8,
};

let scheduler = MvccScheduler::with_config(config);
\\\

#### 并行执行

\\\ust
use vm_runtime::TxnFn;

// 定义交易函数
let txns: Vec<(TxId, TxnFn)> = vec![
    (1, Box::new(|txn| {
        let value = txn.read(b"key1")?;
        txn.write(b"key1".to_vec(), b"value1".to_vec());
        Ok(0)
    })),
    (2, Box::new(|txn| {
        txn.write(b"key2".to_vec(), b"value2".to_vec());
        Ok(0)
    })),
];

// 并行执行
let result = scheduler.execute_batch_parallel(txns)?;

println!("成功: {}", result.successful);
println!("失败: {}", result.failed);
println!("冲突: {}", result.conflicts);
println!("成功率: {:.2}%", 
    result.successful as f64 / (result.successful + result.failed) as f64 * 100.0
);
\\\

#### 单交易执行

\\\ust
let tx_id = 1;
let txn_fn = Box::new(|txn: &mut Txn| {
    let value = txn.read(b"counter")?
        .and_then(|v| String::from_utf8(v).ok())
        .and_then(|s| s.parse::<i32>().ok())
        .unwrap_or(0);
    
    let new_value = value + 1;
    txn.write(b"counter".to_vec(), new_value.to_string().into_bytes());
    
    Ok(new_value)
});

let result = scheduler.execute_transaction(tx_id, txn_fn)?;

if result.success {
    println!("Tx {} 成功, 返回值: {:?}", result.tx_id, result.return_value);
} else {
    println!("Tx {} 失败: {:?}", result.tx_id, result.error);
}
\\\

---

### WorkStealingScheduler API

#### 创建调度器

\\\ust
use vm_runtime::WorkStealingScheduler;

// 自动检测 CPU 核心数
let scheduler = WorkStealingScheduler::new(None);

// 指定工作线程数
let scheduler = WorkStealingScheduler::new(Some(8));
\\\

#### 执行任务

\\\ust
let tx_ids: Vec<TxId> = (1..=100).collect();

let executor = |tx_id: TxId, state: &StateManager| -> Result<ExecutionResult> {
    // 模拟交易执行
    std::thread::sleep(std::time::Duration::from_micros(100));
    
    Ok(ExecutionResult {
        tx_id,
        success: true,
        return_value: 0,
        ..Default::default()
    })
};

let results = scheduler.execute_parallel(&tx_ids, executor)?;

println!("完成 {} 个交易", results.len());
\\\

#### 获取内部调度器

\\\ust
let parallel_scheduler = scheduler.get_scheduler();
let stats = parallel_scheduler.get_stats();

println!("统计: {:?}", stats);
\\\

---

## 使用示例

### 示例 1: 简单并行执行

\\\ust
use vm_runtime::{ParallelScheduler, StateManager, ExecutionResult};

fn main() -> anyhow::Result<()> {
    let scheduler = ParallelScheduler::new();
    
    // 定义交易执行逻辑
    let executor = |tx_id: TxId, state: &StateManager| -> Result<ExecutionResult> {
        let key = format!("account_{}", tx_id);
        
        // 读取余额
        let balance = state.get(key.as_bytes())?
            .and_then(|v| String::from_utf8(v).ok())
            .and_then(|s| s.parse::<u64>().ok())
            .unwrap_or(0);
        
        // 增加余额
        let new_balance = balance + 100;
        state.set(key.as_bytes(), new_balance.to_string().as_bytes())?;
        
        Ok(ExecutionResult {
            tx_id,
            return_value: new_balance as i32,
            success: true,
            ..Default::default()
        })
    };
    
    // 执行 10 个交易
    let tx_ids: Vec<TxId> = (1..=10).collect();
    let results = scheduler.execute_parallel(&tx_ids, executor)?;
    
    // 输出结果
    for result in results {
        println!("Tx {}: 余额 = {}", result.tx_id, result.return_value);
    }
    
    // 打印统计
    let stats = scheduler.get_stats();
    println!("\n统计信息:");
    println!("  成功: {}", stats.successful_txs);
    println!("  失败: {}", stats.failed_txs);
    println!("  成功率: {:.2}%", stats.success_rate() * 100.0);
    
    Ok(())
}
\\\

### 示例 2: 冲突检测与重试

\\\ust
use vm_runtime::ParallelScheduler;

fn main() -> anyhow::Result<()> {
    let scheduler = ParallelScheduler::new();
    
    // 定义有冲突的交易
    let executor = |tx_id: TxId, state: &StateManager| -> Result<ExecutionResult> {
        // 所有交易都访问同一个键 (热点键)
        let key = b"hot_key";
        
        let value = state.get(key)?
            .and_then(|v| String::from_utf8(v).ok())
            .and_then(|s| s.parse::<i32>().ok())
            .unwrap_or(0);
        
        // 模拟计算
        std::thread::sleep(std::time::Duration::from_millis(10));
        
        let new_value = value + 1;
        state.set(key, new_value.to_string().as_bytes())?;
        
        Ok(ExecutionResult {
            tx_id,
            return_value: new_value,
            success: true,
            ..Default::default()
        })
    };
    
    // 执行 20 个冲突交易
    let tx_ids: Vec<TxId> = (1..=20).collect();
    let results = scheduler.execute_parallel(&tx_ids, executor)?;
    
    // 验证最终值
    let final_value = results.last().unwrap().return_value;
    println!("最终值: {} (预期: 20)", final_value);
    assert_eq!(final_value, 20, "最终值应该等于交易数量");
    
    // 打印统计
    let stats = scheduler.get_stats();
    println!("\n冲突统计:");
    println!("  检测到的冲突: {}", stats.conflict_count);
    println!("  重试次数: {}", stats.retry_count);
    println!("  回滚次数: {}", stats.rollback_count);
    
    Ok(())
}
\\\

### 示例 3: MVCC 调度器使用

\\\ust
use vm_runtime::{MvccScheduler, MvccSchedulerConfig, TxnFn};

fn main() -> anyhow::Result<()> {
    // 创建调度器
    let config = MvccSchedulerConfig::default();
    let scheduler = MvccScheduler::with_config(config);
    
    // 准备交易
    let mut txns: Vec<(TxId, TxnFn)> = Vec::new();
    
    for i in 1..=100 {
        let tx_id = i as TxId;
        let txn_fn = Box::new(move |txn: &mut Txn| {
            let key = format!("key_{}", i % 10); // 10 个键
            
            // 读取当前值
            let value = txn.read(key.as_bytes())?
                .and_then(|v| String::from_utf8(v).ok())
                .and_then(|s| s.parse::<i32>().ok())
                .unwrap_or(0);
            
            // 写入新值
            let new_value = value + 1;
            txn.write(key.into_bytes(), new_value.to_string().into_bytes());
            
            Ok(new_value)
        });
        
        txns.push((tx_id, txn_fn));
    }
    
    // 并行执行
    let start = std::time::Instant::now();
    let result = scheduler.execute_batch_parallel(txns)?;
    let duration = start.elapsed();
    
    // 输出结果
    println!("执行完成:");
    println!("  成功: {}", result.successful);
    println!("  失败: {}", result.failed);
    println!("  冲突: {}", result.conflicts);
    println!("  耗时: {:.2}ms", duration.as_micros() as f64 / 1000.0);
    println!("  TPS: {:.2}", 
        (result.successful + result.failed) as f64 / duration.as_secs_f64()
    );
    
    // 验证结果
    for i in 0..10 {
        let key = format!("key_{}", i);
        let value = scheduler.get_value(key.as_bytes())?
            .and_then(|v| String::from_utf8(v).ok())
            .and_then(|s| s.parse::<i32>().ok())
            .unwrap_or(0);
        println!("  {}: {}", key, value);
    }
    
    Ok(())
}
\\\

### 示例 4: 工作窃取调度器

\\\ust
use vm_runtime::WorkStealingScheduler;

fn main() -> anyhow::Result<()> {
    // 创建 8 个工作线程
    let scheduler = WorkStealingScheduler::new(Some(8));
    
    // 定义计算密集型交易
    let executor = |tx_id: TxId, state: &StateManager| -> Result<ExecutionResult> {
        let key = format!("key_{}", tx_id % 100);
        
        // 模拟计算
        let mut sum = 0u64;
        for i in 0..10000 {
            sum += i;
        }
        
        state.set(key.as_bytes(), sum.to_string().as_bytes())?;
        
        Ok(ExecutionResult {
            tx_id,
            return_value: sum as i32,
            success: true,
            ..Default::default()
        })
    };
    
    // 执行 1000 个交易
    let tx_ids: Vec<TxId> = (1..=1000).collect();
    
    let start = std::time::Instant::now();
    let results = scheduler.execute_parallel(&tx_ids, executor)?;
    let duration = start.elapsed();
    
    println!("工作窃取调度器:");
    println!("  完成交易: {}", results.len());
    println!("  耗时: {:.2}ms", duration.as_millis());
    println!("  平均延迟: {:.2}μs", 
        duration.as_micros() as f64 / results.len() as f64
    );
    
    // 获取统计
    let parallel_scheduler = scheduler.get_scheduler();
    let stats = parallel_scheduler.get_stats();
    println!("  成功率: {:.2}%", stats.success_rate() * 100.0);
    
    Ok(())
}
\\\

---

## 性能优化

### 1. 减少冲突

**策略 1: 键分片**

\\\ust
// 不好: 所有交易访问同一个键
let key = b"global_counter";

// 好: 按交易 ID 分片
let shard = tx_id % 10;
let key = format!("counter_{}", shard);
\\\

**策略 2: 批量操作**

\\\ust
// 不好: 每个键一个交易
for i in 0..100 {
    execute_transaction(|state| {
        state.set(format!("key_{}", i).as_bytes(), b"value")?;
    });
}

// 好: 批量写入
execute_transaction(|state| {
    for i in 0..100 {
        state.set(format!("key_{}", i).as_bytes(), b"value")?;
    }
});
\\\

**策略 3: 只读事务**

\\\ust
// MVCC 调度器支持只读事务优化
let txn = store.begin_read_only(); // 无冲突检测
let value = txn.read(b"key")?;
txn.commit()?; // 快速路径
\\\

### 2. 优化并行度

**策略 1: 调整工作线程数**

\\\ust
// 自动检测 CPU 核心数
let num_cores = num_cpus::get();

// 计算密集型: 使用全部核心
let scheduler = WorkStealingScheduler::new(Some(num_cores));

// IO 密集型: 可以超额配置
let scheduler = WorkStealingScheduler::new(Some(num_cores * 2));
\\\

**策略 2: 批次大小调优**

\\\ust
// 小批次: 更好的负载均衡，但调度开销大
let batch_size = 10;

// 大批次: 减少调度开销，但可能负载不均
let batch_size = 100;

// 推荐: 根据任务复杂度调整
let batch_size = if is_complex_task { 10 } else { 100 };
\\\

### 3. 内存优化

**策略 1: 及时释放快照**

\\\ust
// 不好: 快照未释放
let snapshot_id = state_manager.create_snapshot();
execute_transaction();
// 忘记释放

// 好: RAII 模式自动释放
{
    let _snapshot = state_manager.create_snapshot();
    execute_transaction();
} // 自动释放
\\\

**策略 2: 启用自适应 GC**

\\\ust
let config = MvccSchedulerConfig {
    mvcc_config: GcConfig {
        max_versions_per_key: 20,
        auto_gc: Some(AutoGcConfig {
            enable_adaptive: true, // 启用自适应
            interval_secs: 60,
            version_threshold: 1000,
            ..Default::default()
        }),
        ..Default::default()
    },
    ..Default::default()
};
\\\

### 4. 减少系统调用

**策略 1: 批量读取**

\\\ust
// 不好: 多次系统调用
for key in keys {
    let value = state.get(&key)?;
}

// 好: 批量读取
let values = state.batch_get(&keys)?;
\\\

**策略 2: 缓存频繁访问的键**

\\\ust
use std::cell::RefCell;
use std::collections::HashMap;

thread_local! {
    static CACHE: RefCell<HashMap<Vec<u8>, Vec<u8>>> = RefCell::new(HashMap::new());
}

fn cached_get(state: &StateManager, key: &[u8]) -> Result<Option<Vec<u8>>> {
    CACHE.with(|cache| {
        if let Some(value) = cache.borrow().get(key) {
            return Ok(Some(value.clone()));
        }
        
        let value = state.get(key)?;
        if let Some(ref v) = value {
            cache.borrow_mut().insert(key.to_vec(), v.clone());
        }
        Ok(value)
    })
}
\\\

---

## 测试与基准

### 单元测试

\\\ust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_conflict_detection() {
        let mut detector = ConflictDetector::new();
        
        // 交易 1: 读 key1, 写 key2
        let mut rw1 = ReadWriteSet::new();
        rw1.add_read(b"key1".to_vec());
        rw1.add_write(b"key2".to_vec());
        detector.record(1, rw1);
        
        // 交易 2: 读 key2, 写 key3
        let mut rw2 = ReadWriteSet::new();
        rw2.add_read(b"key2".to_vec());
        rw2.add_write(b"key3".to_vec());
        detector.record(2, rw2);
        
        // 应该检测到冲突 (RAW)
        assert!(detector.has_conflict(1, 2));
    }

    #[test]
    fn test_dependency_graph() {
        let mut graph = DependencyGraph::new();
        
        // 构建依赖: 2 -> 1, 3 -> 1, 3 -> 2
        graph.add_dependency(2, 1);
        graph.add_dependency(3, 1);
        graph.add_dependency(3, 2);
        
        let all_txs = vec![1, 2, 3];
        let mut completed = HashSet::new();
        
        // 初始: 只有 1 可执行
        let ready = graph.get_ready_transactions(&all_txs, &completed);
        assert_eq!(ready, vec![1]);
        
        // 1 完成后: 2 可执行
        completed.insert(1);
        let ready = graph.get_ready_transactions(&all_txs, &completed);
        assert_eq!(ready, vec![2]);
        
        // 1, 2 完成后: 3 可执行
        completed.insert(2);
        let ready = graph.get_ready_transactions(&all_txs, &completed);
        assert_eq!(ready, vec![3]);
    }

    #[test]
    fn test_parallel_execution() {
        let scheduler = ParallelScheduler::new();
        
        let executor = |tx_id: TxId, state: &StateManager| -> Result<ExecutionResult> {
            let key = format!("key_{}", tx_id);
            state.set(key.as_bytes(), b"value")?;
            
            Ok(ExecutionResult {
                tx_id,
                success: true,
                return_value: 0,
                ..Default::default()
            })
        };
        
        let tx_ids: Vec<TxId> = (1..=10).collect();
        let results = scheduler.execute_parallel(&tx_ids, executor).unwrap();
        
        assert_eq!(results.len(), 10);
        assert!(results.iter().all(|r| r.success));
    }
}
\\\

### 性能基准

\\\ust
use criterion::{black_box, criterion_group, criterion_main, Criterion};

fn bench_parallel_scheduler(c: &mut Criterion) {
    let scheduler = ParallelScheduler::new();
    
    c.bench_function("parallel_10_txs", |b| {
        b.iter(|| {
            let executor = |tx_id: TxId, state: &StateManager| {
                let key = format!("key_{}", tx_id);
                state.set(key.as_bytes(), b"value").unwrap();
                Ok(ExecutionResult::default())
            };
            
            let tx_ids: Vec<TxId> = (1..=10).collect();
            scheduler.execute_parallel(&tx_ids, executor).unwrap()
        });
    });
}

fn bench_mvcc_scheduler(c: &mut Criterion) {
    let scheduler = MvccScheduler::new();
    
    c.bench_function("mvcc_100_txs", |b| {
        b.iter(|| {
            let mut txns: Vec<(TxId, TxnFn)> = Vec::new();
            
            for i in 1..=100 {
                let txn_fn = Box::new(move |txn: &mut Txn| {
                    let key = format!("key_{}", i % 10);
                    txn.write(key.into_bytes(), b"value".to_vec());
                    Ok(0)
                });
                txns.push((i, txn_fn));
            }
            
            scheduler.execute_batch_parallel(txns).unwrap()
        });
    });
}

criterion_group!(benches, bench_parallel_scheduler, bench_mvcc_scheduler);
criterion_main!(benches);
\\\

### 压力测试

运行完整的压力测试套件:

\\\ash
# 运行并行执行相关测试
cargo test -p vm-runtime --test parallel_tests -- --nocapture

# 运行 MVCC 调度器测试
cargo test -p vm-runtime --test mvcc_parallel_tests -- --nocapture

# 运行工作窃取调度器测试
cargo test -p vm-runtime --test work_stealing_tests -- --nocapture
\\\

---

## 最佳实践

### 1. 选择合适的调度器

\\\ust
// 场景 1: 通用并行执行，需要灵活性
let scheduler = ParallelScheduler::new();

// 场景 2: 追求最佳性能，MVCC 原生支持 (推荐)
let scheduler = MvccScheduler::new();

// 场景 3: 计算密集型，需要动态负载均衡
let scheduler = WorkStealingScheduler::new(None);
\\\

### 2. 合理设置并行度

\\\ust
// 获取 CPU 核心数
let num_cores = num_cpus::get();

// 计算密集型: 使用 CPU 核心数
let config = MvccSchedulerConfig {
    num_workers: num_cores,
    ..Default::default()
};

// IO 密集型: 可以超额配置
let config = MvccSchedulerConfig {
    num_workers: num_cores * 2,
    ..Default::default()
};
\\\

### 3. 监控执行统计

\\\ust
// 定期检查统计信息
let stats = scheduler.get_stats();

println!("执行统计:");
println!("  成功率: {:.2}%", stats.success_rate() * 100.0);
println!("  冲突率: {:.2}%", stats.conflict_rate() * 100.0);
println!("  重试次数: {}", stats.retry_count);

// 告警检查
if stats.success_rate() < 0.95 {
    log::warn!("成功率过低: {:.2}%", stats.success_rate() * 100.0);
}

if stats.conflict_rate() > 0.1 {
    log::warn!("冲突率过高: {:.2}%", stats.conflict_rate() * 100.0);
}
\\\

### 4. 错误处理

\\\ust
use anyhow::{Context, Result};

fn execute_with_error_handling(
    scheduler: &MvccScheduler,
    txns: Vec<(TxId, TxnFn)>
) -> Result<BatchTxnResult> {
    // 执行交易
    let result = scheduler.execute_batch_parallel(txns)
        .context("批量执行失败")?;
    
    // 检查失败交易
    for txn_result in &result.results {
        if !txn_result.success {
            log::error!(
                "Tx {} 失败: {:?}", 
                txn_result.tx_id, 
                txn_result.error
            );
        }
    }
    
    // 检查冲突率
    let conflict_rate = result.conflicts as f64 / 
        (result.successful + result.failed) as f64;
    
    if conflict_rate > 0.2 {
        log::warn!(
            "冲突率过高: {:.2}%, 考虑优化交易访问模式",
            conflict_rate * 100.0
        );
    }
    
    Ok(result)
}
\\\

### 5. 性能调优检查清单

 **冲突优化**
- 使用键分片减少热点
- 批量操作合并写入
- 只读事务使用快速路径

 **并行度调优**
- 根据工作负载类型调整线程数
- 调整批次大小平衡调度开销

 **内存管理**
- 启用自适应 GC
- 及时释放快照
- 监控版本数增长

 **监控告警**
- 定期检查成功率和冲突率
- 记录执行统计趋势
- 设置性能基准和告警阈值

---

## 总结

SuperVM 并行执行引擎提供了三种调度器:

| 调度器 | 适用场景 | 性能 | 复杂度 |
|--------|---------|------|--------|
| **ParallelScheduler** | 通用并行执行 | 良好 | 中等 |
| **MvccScheduler** | MVCC 原生场景 (推荐) | 优秀 | 低 |
| **WorkStealingScheduler** | 计算密集型 | 很好 | 低 |

**推荐使用 MvccScheduler**:
-  更好的性能和正确性
-  自动内存管理
-  更简单的 API
-  生产环境验证

**关键要点**:
1. 正确性优先，性能其次
2. 减少冲突是提升性能的关键
3. 合理设置并行度和 GC 参数
4. 持续监控执行统计

---

## 参考资料

- **相关文档**: 
  - [GC 运行时可观测性](gc-observability.md)
  - [MVCC 压力测试与调优指南](stress-testing-guide.md)
  - [API 文档](API.md)

- **代码示例**: 
  - src/node-core/examples/demo5-parallel.rs
  - src/node-core/examples/demo7-work-stealing.rs
  - src/node-core/examples/demo9-mvcc-parallel.rs

- **测试文件**: 
  - src/vm-runtime/tests/parallel_tests.rs
  - src/vm-runtime/tests/mvcc_stress_test.rs

---

 2025 SuperVM Project | GPL-3.0 License
