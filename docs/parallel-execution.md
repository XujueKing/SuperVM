# 并行执行引擎设计文档

作者: king  
版本: v0.6.0  
日期: 2025-11-04

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

### 4. 常见瓶颈分析

- 锁竞争（Mutex 争用）
    - 症状: 高并发下延迟抖动、尾延迟上升
    - 缓解: 采用批量写/读、缩短持锁区间、必要时细化锁粒度
- 快照创建/回滚开销
    - 症状: 大事务或深度嵌套时耗时上升
    - 缓解: 合理分批、减少不必要的嵌套、将只读路径移出快照
- 依赖图过度密集
    - 症状: 可并行度下降、批次变小
    - 缓解: 通过读写集设计减少交叉访问、对热门键做分片
- 调度开销（工作窃取）
    - 症状: 任务极短时调度成本相对偏高
    - 缓解: 合并小任务为批处理、提升每个任务的工作量

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

#### 如何阅读报告

- 打开 HTML 报告: `target/criterion/report/index.html`
- estimates.json 字段:
    - mean/median: 平均/中位数耗时
    - slope: 线性拟合的趋势估计
    - std_dev: 标准差（抖动）
    - confidence_interval: 置信区间（默认 95%）
- 单位: ns/iter（Criterion 默认单位）

#### 示例指标（节选）

- 并行调度 get_parallel_batch/100: 平均约 350,045 ns/批
- 冲突检测 non_conflicting/100: 平均约 396,673 ns
- 冲突检测 50% 冲突/100: 平均约 460,675 ns
- 快照创建 create_snapshot/1000: 平均约 224,712 ns
- 依赖图 build_and_query/100: 平均约 344,862 ns

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

## MVCC 存储后端 (v0.5.0) 🔐

### 什么是 MVCC？

MVCC (Multi-Version Concurrency Control，多版本并发控制) 是一种并发控制方法，允许多个事务同时访问数据库而不互相阻塞。每个键维护多个版本，事务读取其启动时刻的快照，写入创建新版本。

### 何时使用 MVCC？

**推荐使用 MVCC 的场景**:
- ✅ 高并发读写混合负载
- ✅ 长事务与短事务混合
- ✅ 需要快照隔离语义
- ✅ 查询密集型应用（使用只读事务优化）

**推荐使用 Snapshot 的场景**:
- ✅ 简单串行执行
- ✅ 短事务为主
- ✅ 内存敏感场景（MVCC 会保留多版本）
- ✅ 不需要高并发

### 创建 MVCC 调度器

```rust
use vm_runtime::{ParallelScheduler, MvccStore};
use std::sync::Arc;

// 创建 MVCC 存储
let mvcc_store = Arc::new(MvccStore::new());

// 创建使用 MVCC 后端的调度器
let scheduler = ParallelScheduler::new_with_mvcc(Arc::clone(&mvcc_store));

// 执行读写事务
scheduler.execute_with_mvcc(|txn| {
    // 读取数据
    if let Some(balance) = txn.read(b"balance") {
        println!("Balance: {:?}", balance);
    }
    
    // 写入数据（本地缓存）
    txn.write(b"balance".to_vec(), b"100".to_vec());
    
    // 成功返回，自动提交
    Ok(())
})?;

// 执行只读事务（快速路径，无冲突检测）
let result = scheduler.execute_with_mvcc_read_only(|txn| {
    let balance = txn.read(b"balance")?
        .ok_or("Balance not found")?;
    
    Ok(balance)
})?;
```

### MVCC 特性

**快照隔离 (Snapshot Isolation)**:
- 每个事务看到启动时刻的数据快照
- 读取不会被写入阻塞
- 写入不会阻塞读取

**写写冲突检测**:
```rust
let store = Arc::new(MvccStore::new());

// 事务 1 和 2 并发写同一键
let mut t1 = store.begin();
let mut t2 = store.begin();

t1.write(b"key".to_vec(), b"value1".to_vec());
t2.write(b"key".to_vec(), b"value2".to_vec());

// 第一个提交成功
t1.commit()?;

// 第二个提交失败（写写冲突）
assert!(t2.commit().is_err());
```

**只读事务优化**:
```rust
// 只读事务使用快速路径
let ro_txn = store.begin_read_only();

// 可以读取多个键
let val1 = ro_txn.read(b"key1");
let val2 = ro_txn.read(b"key2");

// 提交无需冲突检测，直接返回
let start_ts = ro_txn.commit()?;

// ❌ 只读事务不能写入（会 panic）
// ro_txn.write(...); // panic!
```

**细粒度并发控制**:
- DashMap 无锁哈希表，减少全局锁竞争
- 每键 RwLock，允许多个事务并发读取同一键
- 提交时按键排序加锁，避免死锁
- 原子时间戳分配，消除时间戳瓶颈

### MVCC vs Snapshot 性能对比

运行 MVCC 基准测试:
```bash
cargo bench --bench parallel_benchmark -- mvcc
```

**典型性能特征**:
- **只读事务**: MVCC 快速路径比 Snapshot 快 2-5 倍
- **并发读取**: MVCC 允许无锁并发，Snapshot 需要锁
- **写入性能**: 无冲突时性能相近，MVCC 略有开销（版本管理）
- **冲突场景**: MVCC 在提交时检测，Snapshot 在锁获取时阻塞

---

## 未来优化

### MVCC 垃圾回收 (v0.6.0) 🗑️

#### 为什么需要 GC？

MVCC 为每个键维护多个版本，随着事务的执行，版本数会不断增长。如果不清理旧版本：
- **内存占用**持续增加
- **查找性能**下降（版本链过长）
- **存储开销**失控

#### GC 配置

```rust
use vm_runtime::{MvccStore, GcConfig};

let config = GcConfig {
    max_versions_per_key: 10,      // 每个键最多保留 10 个版本
    enable_time_based_gc: false,   // 基于时间的 GC（未来功能）
    version_ttl_secs: 3600,        // 版本过期时间（秒）
};

let store = MvccStore::new_with_config(config);
```

#### 手动触发 GC

```rust
// 执行一次 GC
let cleaned_count = store.gc()?;
println!("清理了 {} 个旧版本", cleaned_count);

// 获取 GC 统计
let stats = store.get_gc_stats();
println!("GC 执行次数: {}", stats.gc_count);
println!("总清理版本数: {}", stats.versions_cleaned);
println!("清理的键数: {}", stats.keys_cleaned);
println!("最后 GC 时间戳: {}", stats.last_gc_ts);

// 监控存储状态
println!("当前总版本数: {}", store.total_versions());
println!("当前键数量: {}", store.total_keys());
println!("最小活跃事务时间戳: {:?}", store.get_min_active_ts());
```

#### GC 清理策略

**保留规则**（优先级从高到低）:
1. **最新版本**: 每个键的最新版本永远保留
2. **活跃事务可见版本**: 所有活跃事务可能读到的版本必须保留
3. **版本数量限制**: 根据 `max_versions_per_key` 清理超量旧版本

**清理流程**:
```
对每个键的版本链:
  1. 找到最小活跃事务 start_ts (水位线)
  2. 保留 ts <= start_ts 的第一个版本及之后的所有版本
  3. 在此基础上，根据 max_versions_per_key 限制进一步清理
  4. 最新版本无条件保留
```

**示例**:
```rust
let store = MvccStore::new_with_config(GcConfig {
    max_versions_per_key: 3,
    ..Default::default()
});

// 写入 5 个版本: ts=1,2,3,4,5
for i in 1..=5 {
    let mut txn = store.begin();
    txn.write(b"key".to_vec(), format!("v{}", i).into_bytes());
    txn.commit()?;
}

// 开启一个长事务（start_ts=6，能看到 ts<=6 的版本，即所有版本）
let long_txn = store.begin();

// 再写入 v6, v7
for i in 6..=7 {
    let mut txn = store.begin();
    txn.write(b"key".to_vec(), format!("v{}", i).into_bytes());
    txn.commit()?;
}

// 此时有 7 个版本，最小活跃 ts=6
store.gc()?;

// GC 后:
// - 保留 ts=1 (long_txn 的水位线内第一个可见版本)
// - 保留 ts=2,3,4,5,6,7 (都 >= min_active_ts)
// - 所有版本都被保留，因为 long_txn 仍活跃

drop(long_txn); // 结束长事务

store.gc()?;

// GC 后:
// - 没有活跃事务，根据 max_versions_per_key=3
// - 保留最新的 3 个版本: ts=5,6,7
// - 清理 ts=1,2,3,4
```

#### 活跃事务跟踪

MVCC 自动跟踪活跃事务:
```rust
// 开始事务时自动注册
let txn1 = store.begin();
let txn2 = store.begin_read_only();

// 查询活跃事务水位线
let min_ts = store.get_min_active_ts();
println!("最小活跃 ts: {:?}", min_ts);

// 事务结束时自动注销（Drop trait）
drop(txn1);
drop(txn2);

// 现在没有活跃事务
assert_eq!(store.get_min_active_ts(), None);
```

#### GC 最佳实践

**1. 定期触发 GC**:
```rust
// 简单策略：每 N 个事务触发一次
let mut tx_count = 0;
loop {
    // 执行事务...
    tx_count += 1;
    
    if tx_count % 100 == 0 {
        store.gc()?;
    }
}
```

**2. 基于版本数触发**:
```rust
// 版本数超过阈值时触发
if store.total_versions() > 10000 {
    println!("版本数过多，触发 GC");
    let cleaned = store.gc()?;
    println!("清理了 {} 个版本", cleaned);
}
```

**3. 监控 GC 效果**:
```rust
let before_versions = store.total_versions();
let cleaned = store.gc()?;
let after_versions = store.total_versions();

println!("GC 前: {} 版本", before_versions);
println!("清理: {} 版本", cleaned);
println!("GC 后: {} 版本", after_versions);
println!("压缩率: {:.2}%", 
    cleaned as f64 / before_versions as f64 * 100.0);
```

**4. 避免在事务中触发 GC**:
```rust
// ❌ 不好 - 可能清理当前事务需要的版本
let txn = store.begin();
store.gc()?; // 危险！
txn.read(b"key");

// ✅ 好 - 在事务之间触发
drop(txn);
store.gc()?;
let txn2 = store.begin();
```

#### GC 性能影响

运行 GC 基准测试:
```bash
cargo bench --bench parallel_benchmark -- mvcc_gc
```

**典型性能特征**:
- **GC 吞吐量**: 每次 GC 可清理数千到数万个版本（毫秒级）
- **读取影响**: GC 使用写锁，不阻塞读操作（并发读取不受影响）
- **写入影响**: GC 期间新写入需要等待（但 GC 通常很快）
- **活跃事务影响**: 活跃事务越多，可清理的版本越少

### 短期 (v0.7.0)
- [ ] MVCC 自动 GC（后台线程定期清理）
- [ ] MVCC 压力测试与调优
- [ ] 交易优先级调度策略强化

### 中期 (v0.7.0)
- [ ] 乐观并发控制（OCC）集成
- [ ] 跨分片/分区的并行调度探索
- [ ] MVCC 与 Snapshot 自动选择策略

### 长期 (v1.0.0)
- [ ] 分布式并行执行
- [ ] GPU 加速冲突检测
- [ ] 机器学习优化调度

---

## 参考资料

- [Solana Sealevel 并行执行](https://medium.com/solana-labs/sealevel-parallel-processing-thousands-of-smart-contracts-d814b378192)
- [Aptos Block-STM](https://medium.com/aptoslabs/block-stm-how-we-execute-over-160k-transactions-per-second-on-the-aptos-blockchain-3b003657e4ba)
- [Sui 并行执行模型](https://docs.sui.io/learn/sui-execution)
- [PostgreSQL MVCC](https://www.postgresql.org/docs/current/mvcc.html)
- [CockroachDB Transaction Layer](https://www.cockroachlabs.com/docs/stable/architecture/transaction-layer.html)

---

## 更新历史

- **v0.6.0 (2025-11-04)**: 添加 MVCC 垃圾回收
- **v0.5.0 (2025-11-04)**: MVCC 核心实现 + 只读优化 + 调度器集成
- **v0.4.0 (2025-11-04)**: 批量操作优化
- **v0.3.0 (2025-11-03)**: 工作窃取调度器
- **v0.2.0 (2025-11-03)**: 执行统计 + 自动重试
- **v0.1.0 (2025-11-02)**: 并行执行引擎初版

---

*最后更新: 2025-11-04*
