# 并行执行引擎设计文档

作者: king  
版本: v0.2.0  
日期: 2025-11-03

## 目录

- [概述](#概述)
- [架构设计](#架构设计)
- [核心组件](#核心组件)
- [API 参考](#api-参考)
- [使用示例](#使用示例)
- [性能优化](#性能优化)
- [测试验证](#测试验证)

---

## 概述

SuperVM 并行执行引擎旨在提高区块链交易处理吞吐量，通过智能冲突检测和依赖分析，在保证正确性的前提下最大化并行执行效率。

### 设计目标

- ✅ **正确性优先**: 确保交易执行顺序正确性
- ✅ **高吞吐量**: 最大化并行执行效率
- ✅ **自动恢复**: 失败交易自动回滚
- ✅ **监控友好**: 完整的执行统计信息

### 核心特性

1. **冲突检测**: 基于读写集的智能冲突分析
2. **依赖管理**: 动态构建交易依赖图
3. **状态快照**: 支持嵌套的快照与回滚
4. **自动重试**: 可配置的重试策略
5. **执行统计**: 实时性能监控指标

---

## 架构设计

### 整体架构

```
┌─────────────────────────────────────────────────────────────┐
│                    ParallelScheduler                        │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐      │
│  │  Conflict    │  │ Dependency   │  │   State      │      │
│  │  Detector    │  │   Graph      │  │  Manager     │      │
│  └──────┬───────┘  └──────┬───────┘  └──────┬───────┘      │
│         │                  │                  │              │
│         └──────────────────┼──────────────────┘              │
│                            │                                 │
│                ┌───────────▼────────────┐                    │
│                │  Execution Statistics   │                    │
│                │  - Success/Fail Count   │                    │
│                │  - Retry Count          │                    │
│                │  - Conflict Rate        │                    │
│                └─────────────────────────┘                    │
└─────────────────────────────────────────────────────────────┘
```

### 数据流

```
交易输入 → 读写集提取 → 冲突检测 → 依赖图构建 → 并行调度
                                                    │
                                                    ▼
                                            快照创建 → 执行
                                                    │
                                    ┌───────────────┴───────────────┐
                                    │                               │
                                    ▼                               ▼
                                 成功提交                        失败回滚
                                    │                               │
                                    └───────────────┬───────────────┘
                                                    ▼
                                              更新统计信息
```

---

## 核心组件

### 1. ReadWriteSet (读写集)

**用途**: 记录交易访问的存储键

```rust
pub struct ReadWriteSet {
    pub read_set: HashSet<StorageKey>,   // 读取的键
    pub write_set: HashSet<StorageKey>,  // 写入的键
}
```

**冲突规则**:
- **WAW** (Write-After-Write): 两个交易写同一个键
- **RAW** (Read-After-Write): 一个读，另一个写
- **WAR** (Write-After-Read): 一个写，另一个读

**方法**:
- `add_read(key)`: 记录读操作
- `add_write(key)`: 记录写操作
- `conflicts_with(other)`: 检测是否与另一个读写集冲突

---

### 2. ConflictDetector (冲突检测器)

**用途**: 分析交易之间的冲突关系

```rust
pub struct ConflictDetector {
    analyzed: HashMap<TxId, ReadWriteSet>,
}
```

**工作流程**:
1. 记录每个交易的读写集
2. 比较读写集检测冲突
3. 构建依赖关系图

**方法**:
- `record(tx_id, rw_set)`: 记录交易读写集
- `has_conflict(tx1, tx2)`: 检查两个交易是否冲突
- `build_dependency_graph(tx_order)`: 构建依赖图

---

### 3. DependencyGraph (依赖图)

**用途**: 管理交易之间的依赖关系

```rust
pub struct DependencyGraph {
    dependencies: HashMap<TxId, Vec<TxId>>,
}
```

**功能**:
- 记录哪些交易必须等待哪些交易完成
- 识别可以并行执行的交易批次

**方法**:
- `add_dependency(tx, depends_on)`: 添加依赖
- `get_dependencies(tx)`: 获取依赖列表
- `get_ready_transactions(all_txs, completed)`: 获取可执行交易

---

### 4. StateManager (状态管理器)

**用途**: 管理状态快照和回滚

```rust
pub struct StateManager {
    current_storage: Arc<Mutex<HashMap<Vec<u8>, Vec<u8>>>>,
    current_events: Arc<Mutex<Vec<Vec<u8>>>>,
    snapshots: Vec<StorageSnapshot>,
}
```

**特性**:
- ✅ 支持嵌套快照
- ✅ 原子回滚操作
- ✅ 线程安全 (Arc<Mutex>)

**方法**:
- `create_snapshot()`: 创建快照
- `rollback()`: 回滚到最近快照
- `commit()`: 提交并丢弃快照
- `snapshot_depth()`: 获取快照深度

---

### 5. ExecutionStats (执行统计)

**用途**: 收集和报告执行指标

```rust
pub struct ExecutionStats {
    pub successful_txs: u64,    // 成功交易数
    pub failed_txs: u64,        // 失败交易数
    pub rollback_count: u64,    // 回滚次数
    pub retry_count: u64,       // 重试次数
    pub conflict_count: u64,    // 冲突次数
}
```

**计算指标**:
- `total_txs()`: 总交易数
- `success_rate()`: 成功率
- `rollback_rate()`: 回滚率

---

### 6. ParallelScheduler (并行调度器)

**用途**: 协调所有组件，管理并行执行

```rust
pub struct ParallelScheduler {
    detector: Arc<Mutex<ConflictDetector>>,
    completed: Arc<Mutex<HashSet<TxId>>>,
    state_manager: Arc<Mutex<StateManager>>,
    // 原子统计计数器
    stats_successful: Arc<AtomicU64>,
    stats_failed: Arc<AtomicU64>,
    stats_rollback: Arc<AtomicU64>,
    stats_retry: Arc<AtomicU64>,
    stats_conflict: Arc<AtomicU64>,
}
```

**核心方法**:
- `execute_with_snapshot<F>()`: 快照保护执行
- `execute_with_retry<F>(max_retries)`: 自动重试执行
- `execute_batch(ops)`: 批量执行一组交易，原子提交/回滚
- `batch_write/read/delete(...)`: 批量存储操作，降低锁争用
- `get_parallel_batch()`: 获取可并行交易
- `get_stats()`: 获取执行统计

---

### 7. WorkStealingScheduler (工作窃取调度器)

**用途**: 使用工作窃取算法实现负载均衡的并行调度

```rust
pub struct WorkStealingScheduler {
    injector: Arc<Injector<Task>>,       // 全局任务队列
    stealers: Vec<Stealer<Task>>,        // 窃取器列表
    scheduler: Arc<ParallelScheduler>,   // 底层调度器
    num_workers: usize,                  // 工作线程数
}

pub struct Task {
    pub tx_id: TxId,
    pub priority: u8,  // 0-255,越大优先级越高
}
```

**工作原理**:
1. 每个工作线程有自己的**本地队列** (FIFO)
2. 线程首先从本地队列获取任务
3. 本地队列为空时,从**全局队列**批量获取
4. 全局队列也为空时,从其他线程**窃取**任务
5. 使用 Rayon 线程池实现并行执行

**核心方法**:
- `new(num_workers)`: 创建调度器
- `submit_task(task)`: 提交单个任务
- `submit_tasks(tasks)`: 批量提交任务
- `execute_all<F>(executor)`: 并行执行所有任务
- `get_scheduler()`: 获取底层 ParallelScheduler
- `get_stats()`: 获取执行统计

**优势**:
- ✅ **负载均衡**: 自动平衡线程间的工作量
- ✅ **高吞吐量**: 减少线程空闲时间
- ✅ **可扩展性**: 支持任意数量的工作线程
- ✅ **优先级支持**: 可按优先级调度任务

---

### 8. Batch Operations (批量操作)

**动机**: 批量化减少锁获取与快照创建/提交的次数，提升高并发场景下的吞吐量。

**StateManager 批量 API**:
- `batch_write(Vec<(Vec<u8>, Vec<u8>)>) -> usize`
- `batch_read(&[Vec<u8>]) -> Vec<(Vec<u8>, Vec<u8>)>`
- `batch_delete(&[Vec<u8>]) -> usize`
- `batch_emit_events(Vec<Vec<u8>>) -> usize`

**ParallelScheduler 批量 API**:
- `execute_batch<Vec<T>>(Vec<F>)`: 在单一快照中执行多笔交易，任一失败则整批回滚
- 直通批量存储接口：`batch_write/read/delete`

**示例**:
```rust
// 批量执行三笔转账，任一失败则整批回滚
let results = scheduler.execute_batch(vec![
    Box::new(|m: &StateManager| { /* tx1 */ Ok(1) }) as Box<dyn FnOnce(&StateManager) -> Result<i32, String>>,
    Box::new(|m: &StateManager| { /* tx2 */ Ok(2) }),
    Box::new(|m: &StateManager| { /* tx3 */ Ok(3) }),
])?;
```

---

## API 参考

### 基础使用

```rust
use vm_runtime::ParallelScheduler;

// 创建调度器
let scheduler = ParallelScheduler::new();

// 使用快照保护执行交易
let result = scheduler.execute_with_snapshot(|manager| {
    let storage = manager.get_storage();
    let mut storage = storage.lock().unwrap();
    
    // 执行交易逻辑
    storage.insert(b"balance".to_vec(), b"100".to_vec());
    
    Ok(()) // 返回 Ok 则提交，Err 则回滚
})?;
```

### 自动重试

```rust
// 失败时自动重试
let result = scheduler.execute_with_retry(
    |manager| {
        // 可能失败的操作
        if some_condition() {
            return Err("Temporary failure".to_string());
        }
        Ok(42)
    },
    max_retries: 3  // 最多重试 3 次
)?;
```

### 获取统计

```rust
let stats = scheduler.get_stats();

println!("总交易数: {}", stats.total_txs());
println!("成功率: {:.2}%", stats.success_rate() * 100.0);
println!("回滚率: {:.2}%", stats.rollback_rate() * 100.0);
println!("重试次数: {}", stats.retry_count);
```

### 并行批次调度

```rust
use vm_runtime::{ReadWriteSet, ConflictDetector};

let scheduler = ParallelScheduler::new();

// 记录交易读写集
for (tx_id, rw_set) in transactions {
    scheduler.record_rw_set(tx_id, rw_set);
}

// 获取可并行执行的交易
let all_txs: Vec<u64> = vec![1, 2, 3, 4, 5];
let ready_txs = scheduler.get_parallel_batch(&all_txs);

// ready_txs 包含所有可以并行执行的交易
println!("可并行执行: {:?}", ready_txs);
```

### 工作窃取调度

```rust
use vm_runtime::{WorkStealingScheduler, Task};

// 创建工作窃取调度器 (4 个工作线程)
let scheduler = WorkStealingScheduler::new(Some(4));

// 提交任务
let tasks = vec![
    Task::new(1, 255),  // 高优先级
    Task::new(2, 128),  // 中优先级
    Task::new(3, 50),   // 低优先级
];
scheduler.submit_tasks(tasks);

// 并行执行所有任务
let result = scheduler.execute_all(|tx_id| {
    // 执行任务逻辑
    println!("Processing transaction {}", tx_id);
    Ok(())
})?;

println!("Executed: {:?}", result);
```

---

## 使用示例

### 示例 1: 转账交易

```rust
use vm_runtime::ParallelScheduler;

let scheduler = ParallelScheduler::new();

// Alice 转账给 Bob
let result = scheduler.execute_with_snapshot(|manager| {
    let storage = manager.get_storage();
    let mut storage = storage.lock().unwrap();
    
    // 读取 Alice 余额
    let alice_balance: u64 = storage.get(b"alice")
        .and_then(|b| String::from_utf8(b.clone()).ok())
        .and_then(|s| s.parse().ok())
        .unwrap_or(0);
    
    // 检查余额
    if alice_balance < 50 {
        return Err("Insufficient balance".to_string());
    }
    
    // 更新余额
    storage.insert(b"alice".to_vec(), (alice_balance - 50).to_string().into_bytes());
    
    let bob_balance: u64 = storage.get(b"bob")
        .and_then(|b| String::from_utf8(b.clone()).ok())
        .and_then(|s| s.parse().ok())
        .unwrap_or(0);
    
    storage.insert(b"bob".to_vec(), (bob_balance + 50).to_string().into_bytes());
    
    Ok(())
})?;
```

### 示例 2: 冲突检测

```rust
use vm_runtime::{ReadWriteSet, ConflictDetector};

let mut detector = ConflictDetector::new();

// 交易 1: Alice -> Bob
let mut tx1_rw = ReadWriteSet::new();
tx1_rw.add_read(b"alice".to_vec());
tx1_rw.add_write(b"alice".to_vec());
tx1_rw.add_write(b"bob".to_vec());
detector.record(1, tx1_rw);

// 交易 2: Bob -> Charlie (与 tx1 冲突)
let mut tx2_rw = ReadWriteSet::new();
tx2_rw.add_read(b"bob".to_vec());   // 读 bob，与 tx1 写冲突
tx2_rw.add_write(b"bob".to_vec());
tx2_rw.add_write(b"charlie".to_vec());
detector.record(2, tx2_rw);

// 交易 3: David -> Eve (无冲突)
let mut tx3_rw = ReadWriteSet::new();
tx3_rw.add_write(b"david".to_vec());
tx3_rw.add_write(b"eve".to_vec());
detector.record(3, tx3_rw);

// 检测冲突
assert!(detector.has_conflict(1, 2));  // tx1 和 tx2 冲突
assert!(!detector.has_conflict(1, 3)); // tx1 和 tx3 不冲突
assert!(!detector.has_conflict(2, 3)); // tx2 和 tx3 不冲突

// tx1 和 tx3 可以并行执行，tx2 必须等待 tx1
```

### 示例 3: 嵌套快照

```rust
let scheduler = ParallelScheduler::new();

// 外层交易
scheduler.execute_with_snapshot(|manager| {
    let storage = manager.get_storage();
    let mut storage = storage.lock().unwrap();
    storage.insert(b"level".to_vec(), b"1".to_vec());
    
    // 可以在这里执行更多嵌套交易
    // 每个都有自己的快照
    
    Ok(())
})?;
```

---

## 性能优化

### 1. 最小化锁争用

```rust
// ❌ 不好 - 长时间持有锁
let mut storage = manager.get_storage().lock().unwrap();
expensive_computation();
storage.insert(...);

// ✅ 好 - 只在必要时持有锁
let data = expensive_computation();
{
    let storage = manager.get_storage();
    let mut storage = storage.lock().unwrap();
    storage.insert(...);
}
```

### 2. 批量操作

```rust
// 批量记录读写集
for (tx_id, rw_set) in transactions.iter() {
    scheduler.record_rw_set(*tx_id, rw_set.clone());
}

// 一次性获取可并行批次
let ready_batch = scheduler.get_parallel_batch(&all_tx_ids);
```

### 3. 避免不必要的快照

```rust
// 只读操作不需要快照
let storage = scheduler.get_storage();
let storage = storage.lock().unwrap();
let value = storage.get(b"key");

// 写操作才需要快照保护
scheduler.execute_with_snapshot(|manager| {
    // 修改状态
    Ok(())
})?;
```

---

## 测试验证

### 单元测试覆盖

**冲突检测** (6 个测试):
- ✅ test_read_write_set_conflicts
- ✅ test_no_conflict
- ✅ test_dependency_graph
- ✅ test_conflict_detector

**状态快照** (5 个测试):
- ✅ test_snapshot_creation
- ✅ test_rollback
- ✅ test_nested_snapshots
- ✅ test_commit
- ✅ test_snapshot_with_events

**调度器集成** (3 个测试):
- ✅ test_scheduler_with_snapshot
- ✅ test_scheduler_rollback_on_error
- ✅ test_scheduler_nested_transactions

**统计与重试** (3 个测试):
- ✅ test_execution_stats
- ✅ test_retry_mechanism
- ✅ test_retry_exhausted

### 基准测试

运行基准测试:
```bash
cargo bench --bench parallel_benchmark
```

测试场景:
1. **冲突检测性能**: 10/50/100/500 交易
2. **快照操作性能**: 10/100/1000 数据项
3. **依赖图构建**: 不同冲突率
4. **并行调度**: 批次大小优化

---

## 最佳实践

### 1. 错误处理

```rust
match scheduler.execute_with_snapshot(|manager| {
    // 交易逻辑
    Ok(())
}) {
    Ok(_) => println!("✅ 交易成功"),
    Err(e) => eprintln!("❌ 交易失败: {}", e),
}
```

### 2. 监控统计

```rust
// 定期检查统计信息
let stats = scheduler.get_stats();
if stats.rollback_rate() > 0.5 {
    eprintln!("⚠️  高回滚率: {:.2}%", stats.rollback_rate() * 100.0);
}
```

### 3. 重试策略

```rust
// 根据错误类型决定是否重试
let result = scheduler.execute_with_retry(
    |manager| {
        match try_transaction(manager) {
            Ok(r) => Ok(r),
            Err(e) if e.is_retriable() => Err(e.to_string()),
            Err(e) => return Err(e.to_string()), // 不可重试错误
        }
    },
    max_retries: 5
);
```

---

## 未来优化

### 短期 (v0.3.0)
- [ ] 工作窃取调度算法
- [ ] 批量提交优化
- [ ] 性能基准测试报告

### 中期 (v0.4.0)
- [ ] MVCC (多版本并发控制)
- [ ] 乐观并发控制
- [ ] 交易优先级调度

### 长期 (v1.0.0)
- [ ] 分布式并行执行
- [ ] GPU 加速冲突检测
- [ ] 机器学习优化调度

---

## 参考资料

- [Solana Sealevel 并行执行](https://medium.com/solana-labs/sealevel-parallel-processing-thousands-of-smart-contracts-d814b378192)
- [Aptos Block-STM](https://medium.com/aptoslabs/block-stm-how-we-execute-over-160k-transactions-per-second-on-the-aptos-blockchain-3b003657e4ba)
- [Sui 并行执行模型](https://docs.sui.io/learn/sui-execution)

---

*最后更新: 2025-11-03*
