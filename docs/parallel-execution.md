# 并行执行引擎设计文档# 骞惰鎵ц寮曟搸璁捐鏂囨。



作者: king  浣滆€? king  

版本: v0.7.0  鐗堟湰: v0.6.0  

日期: 2025-11-04鏃ユ湡: 2025-11-04



## 目录## 鐩綍



- [概述](#概述)- [姒傝堪](#姒傝堪)

- [架构设计](#架构设计)- [鏋舵瀯璁捐](#鏋舵瀯璁捐)

- [核心组件](#核心组件)- [鏍稿績缁勪欢](#鏍稿績缁勪欢)

- [关键 API](#关键-api)- [API 鍙傝€僝(#api-鍙傝€?

- [并行调度](#并行调度)- [浣跨敤绀轰緥](#浣跨敤绀轰緥)

- [示例](#示例)- [鎬ц兘浼樺寲](#鎬ц兘浼樺寲)

- [性能优化](#性能优化)- [娴嬭瘯楠岃瘉](#娴嬭瘯楠岃瘉)

- [测试与基准](#测试与基准)

---

---

## 姒傝堪

## 概述

SuperVM 骞惰鎵ц寮曟搸鏃ㄥ湪鎻愰珮鍖哄潡閾句氦鏄撳鐞嗗悶鍚愰噺锛岄€氳繃鏅鸿兘鍐茬獊妫€娴嬪拰渚濊禆鍒嗘瀽锛屽湪淇濊瘉姝ｇ‘鎬х殑鍓嶆彁涓嬫渶澶у寲骞惰鎵ц鏁堢巼銆?

SuperVM 并行执行引擎旨在提高区块链交易处理吞吐量，通过智能冲突检测和依赖分析，在保证正确性的前提下最大化并行执行效率。

### 璁捐鐩爣

### 设计目标

- 鉁?**姝ｇ‘鎬т紭鍏?*: 纭繚浜ゆ槗鎵ц椤哄簭姝ｇ‘鎬?

- ✅ **正确性优先**: 确保交易执行顺序正确性- 鉁?**楂樺悶鍚愰噺**: 鏈€澶у寲骞惰鎵ц鏁堢巼

- ✅ **高吞吐量**: 最大化并行执行效率- 鉁?**鑷姩鎭㈠**: 澶辫触浜ゆ槗鑷姩鍥炴粴

- ✅ **自动恢复**: 失败交易自动回滚# 并行执行引擎设计文档



---- [架构设计](#架构设计)

- [核心组件](#核心组件)

## 架构设计- [关键 API](#关键-api)

- [并行调度](#并行调度)

### 总体流程- [示例](#示例)

- [性能优化](#性能优化)

```- [测试与基准](#测试与基准)

交易输入 → 读写集提取 → 冲突检测 → 依赖图构建 → 并行调度

```SuperVM 并行执行引擎用于在保证正确性的前提下，最大化吞吐并最小化延迟；通过智能冲突检测与依赖分析，实现安全可控的并行化。



```- 正确性优先：严格保证事务执行顺序和一致性

      ┌────────────┐      ┌──────────────┐      ┌──────────────┐- 高吞吐：充分并行化与批处理优化

      │  Tx Input  │ ──▶ │  RWSet Build │ ──▶ │ Conflict Check│- 自动重试：冲突时自动回退与重试

      └────────────┘      └──────────────┘      └──────────────┘- 可观测性：完整执行与调度统计

                │                               │

                ▼                               ▼## 架构设计

           ┌──────────────┐               ┌──────────────┐

           │Dependency DAG│ ───────────▶  │Parallel Sched│### 总体流程

           └──────────────┘               └──────────────┘

                                                    │交易输入 → 读写集提取 → 冲突检测 → 依赖图构建 → 并行调度

                                                    ▼

                                         提交或自动重试/回退```

```      ┌────────────┐      ┌──────────────┐      ┌──────────────┐

      │  Tx Input  │ ──▶ │  RWSet Build │ ──▶ │ Conflict Check│

### 核心原理      └────────────┘      └──────────────┘      └──────────────┘

                │                               │

1. **读写集分析**: 记录每个交易的读写键集合                ▼                               ▼

2. **冲突检测**:            ┌──────────────┐               ┌──────────────┐

   - RAW (Read-After-Write): 读后写冲突           │Dependency DAG│ ───────────▶  │Parallel Sched│

   - WAR (Write-After-Read): 写后读冲突           └──────────────┘               └──────────────┘

   - WAW (Write-After-Write): 写后写冲突                                                    │

3. **依赖图**: 构建交易之间的依赖关系                                                    ▼

4. **并行执行**: 根据依赖关系安排并行批次                                         提交或自动重试/回退

5. **自动回滚**: 失败时自动回退并可选重试```



---## 核心组件



## 核心组件1) RW 集合（RWSet）

- RAW（Read-After-Write）、WAR（Write-After-Read）冲突类型识别

### 1. ReadWriteSet (读写集)- 键级别冲突粒度，支持前缀或精确键



**用途**: 记录交易的读写键集合2) 依赖图（DependencyGraph）

- build_dependency_graph(tx_order): 构建依赖关系图

```rust- 记录需等待的依赖与可并行的交易批次

pub struct ReadWriteSet {

    pub reads: HashSet<Vec<u8>>,3) 状态管理（StateManager）

    pub writes: HashSet<Vec<u8>>,- 负责快照与回滚；线程安全（Arc<Mutex> 等）

}- commit(): 提交并释放快照

```

4) 执行统计（ExecutionStats）

**方法**:- 字段：successful_txs, failed_txs, conflicts, retries, elapsed

- `new()`: 创建空的读写集- total_txs(), success_rate() 等便捷方法

- `add_read(key)`: 记录读操作

- `add_write(key)`: 记录写操作5) 并行调度器（ParallelScheduler）

- `has_conflict(&other)`: 检测与另一个读写集是否冲突- 统一编排所有批次执行，追踪并行度与回退

- 支持按优先级调度与工作窃取

---

## 关键 API

### 2. ConflictDetector (冲突检测器)

- build_dependency_graph(tx_order)

**用途**: 检测交易之间的冲突关系- execute_with_snapshot<F>(...)：在快照下执行

- execute_with_retry<F>(max_retries)：自动重试

```rust- execute_batch(ops)：单批并行执行（子事务级提交/回滚）

pub struct ConflictDetector {- batch_write/read/delete(...)：批量存取降低锁竞争

    rw_sets: HashMap<TxId, ReadWriteSet>,- get_stats()：获取执行统计

}

```## 并行调度



**方法**:调度策略（概述）：

- `new()`: 创建检测器- 每个工作线程有本地队列（FIFO）；优先从本地取任务

- `record(tx_id, rw_set)`: 记录交易的读写集- 本地为空时，从全局队列批量获取，或对其他线程进行工作窃取

- `has_conflict(tx1, tx2)`: 检测两个交易是否冲突- 任务支持 priority: u8（0-255，越大优先级越高）

- `build_dependency_graph(tx_order)`: 构建依赖图

核心接口：

---- submit_task(task) / submit_tasks(tasks)

- execute_all<F>(executor) 执行所有待处理任务

### 3. DependencyGraph (依赖图)- get_stats() 获取调度与执行统计



**用途**: 表示交易之间的依赖关系## 示例



```rust1) Alice 向 Bob 转账（无冲突路径）

pub struct DependencyGraph {- 独立键访问，无 RW 冲突，可直接并行提交

    dependencies: HashMap<TxId, HashSet<TxId>>,  // tx -> depends_on

}2) 读写冲突示例（tx1/tx2/tx3）

```- tx1 写 bob；tx2 读 bob；tx3 写 alice

- 依赖图可使 tx1 与 tx3 并行，tx2 等待 tx1 完成再执行

**方法**:

- `add_dependency(tx, depends_on)`: 添加依赖关系3) 嵌套事务（Nested Transactions）

- `get_ready_txs(completed)`: 获取就绪的交易集合- 在单个事务里包含子操作；每个子操作拥有自身快照；失败则子回滚

- `is_ready(tx, completed)`: 检查交易是否就绪

## 性能优化

---

1) 最小化锁持有时间

### 4. StateManager (状态管理器)- 仅在关键路径持锁；分解为“准备-提交”两阶段；

- 尽量读写在快照中进行，提交阶段短暂切入临界区

**用途**: 管理状态快照和回滚

2) 批量操作

```rust- 批量构建读写集、批量快照创建/释放、批量提交，减少切换与锁竞争

pub struct StateManager {

    storage: Arc<Mutex<HashMap<Vec<u8>, Vec<u8>>>>,3) 快照隔离

    snapshots: Arc<Mutex<Vec<HashMap<Vec<u8>, Vec<u8>>>>>,- execute_with_snapshot() 降低共享状态争用

}

```4) 线程安全与配置

- 较重路径使用 Arc<Mutex>/RwLock；结合无锁结构（如队列）

**方法**:- 可调并行度、批大小、重试次数等

- `create_snapshot()`: 创建当前状态快照

- `rollback()`: 回滚到上一个快照## 测试与基准

- `commit()`: 提交当前快照

- `get_storage()`: 获取存储引用测试覆盖（建议）：

- `snapshot_depth()`: 获取快照深度- 状态管理（5 项）：快照/回滚/提交边界

- 调度器（3 项）：队列、工作窃取、优先级

---- 统计与重试（3 项）：冲突重试与成功率



### 5. ExecutionStats (执行统计)测试场景：

1) 无冲突：10/100/1000 条数据

**用途**: 收集和报告执行指标2) 中度/高度冲突：按比例制造 RAW/WAR 冲突

3) 并行批次大小调优

```rust

pub struct ExecutionStats {Criterion 报告：

    pub successful_txs: u64,    // 成功交易数- 打开：target/criterion/report/index.html

    pub failed_txs: u64,        // 失败交易数- 指标释义：mean/median（平均/中位）、slope、std_dev（标准差）、confidence_interval（默认 95%）

    pub rollback_count: u64,    // 回滚次数- 单位：ns/iter（Criterion 默认）

    pub retry_count: u64,       // 重试次数

    pub conflict_count: u64,    // 冲突次数示例基准指标（参考初测）：

}- 并行批次 get_parallel_batch/100：≈350,045 ns/批

```- 无冲突 non_conflicting/100：≈396,673 ns

- 50% 冲突/100：≈460,675 ns

**计算指标**:- 快照创建 create_snapshot/1000：≈224,712 ns

- `total_txs()`: 总交易数- 依赖图 build_and_query/100：≈344,862 ns

- `success_rate()`: 成功率

- `rollback_rate()`: 回滚率—— 以上为 UTF-8 修正版，保留原设计意图与术语，并统一为 SuperVM 命名 ——

- `snapshot_depth()`: 鑾峰彇蹇収娣卞害

---

---

### 6. ParallelScheduler (并行调度器)

### 5. ExecutionStats (鎵ц缁熻)

**用途**: 协调所有组件，管理并行执行

**鐢ㄩ€?*: 鏀堕泦鍜屾姤鍛婃墽琛屾寚鏍?

```rust

pub struct ParallelScheduler {```rust

    detector: Arc<Mutex<ConflictDetector>>,pub struct ExecutionStats {

    completed: Arc<Mutex<HashSet<TxId>>>,    pub successful_txs: u64,    // 鎴愬姛浜ゆ槗鏁?

    state_manager: Arc<Mutex<StateManager>>,    pub failed_txs: u64,        // 澶辫触浜ゆ槗鏁?

    // 原子统计计数器    pub rollback_count: u64,    // 鍥炴粴娆℃暟

    stats_successful: Arc<AtomicU64>,    pub retry_count: u64,       // 閲嶈瘯娆℃暟

    stats_failed: Arc<AtomicU64>,    pub conflict_count: u64,    // 鍐茬獊娆℃暟

    stats_rollback: Arc<AtomicU64>,}

    stats_retry: Arc<AtomicU64>,```

    stats_conflict: Arc<AtomicU64>,

}**璁＄畻鎸囨爣**:

```- `total_txs()`: 鎬讳氦鏄撴暟

- `success_rate()`: 鎴愬姛鐜?

**核心方法**:- `rollback_rate()`: 鍥炴粴鐜?

- `execute_with_snapshot<F>()`: 快照保护执行

- `execute_with_retry<F>(max_retries)`: 自动重试执行---

- `execute_batch(ops)`: 批量执行一组交易，原子提交/回滚

- `batch_write/read/delete(...)`: 批量存储操作，降低锁争用### 6. ParallelScheduler (骞惰璋冨害鍣?

- `get_parallel_batch()`: 获取可并行交易

- `get_stats()`: 获取执行统计**鐢ㄩ€?*: 鍗忚皟鎵€鏈夌粍浠讹紝绠＄悊骞惰鎵ц



---```rust

pub struct ParallelScheduler {

### 7. WorkStealingScheduler (工作窃取调度器)    detector: Arc<Mutex<ConflictDetector>>,

    completed: Arc<Mutex<HashSet<TxId>>>,

**用途**: 使用工作窃取算法实现负载均衡的并行调度    state_manager: Arc<Mutex<StateManager>>,

    // 鍘熷瓙缁熻璁℃暟鍣?

```rust    stats_successful: Arc<AtomicU64>,

pub struct WorkStealingScheduler {    stats_failed: Arc<AtomicU64>,

    injector: Arc<Injector<Task>>,       // 全局任务队列    stats_rollback: Arc<AtomicU64>,

    stealers: Vec<Stealer<Task>>,        // 窃取器列表    stats_retry: Arc<AtomicU64>,

    scheduler: Arc<ParallelScheduler>,   // 底层调度器    stats_conflict: Arc<AtomicU64>,

    num_workers: usize,                  // 工作线程数}

}```



pub struct Task {**鏍稿績鏂规硶**:

    pub tx_id: TxId,- `execute_with_snapshot<F>()`: 蹇収淇濇姢鎵ц

    pub priority: u8,  // 0-255,越大优先级越高- `execute_with_retry<F>(max_retries)`: 鑷姩閲嶈瘯鎵ц

}- `execute_batch(ops)`: 鎵归噺鎵ц涓€缁勪氦鏄擄紝鍘熷瓙鎻愪氦/鍥炴粴

```- `batch_write/read/delete(...)`: 鎵归噺瀛樺偍鎿嶄綔锛岄檷浣庨攣浜夌敤

- `get_parallel_batch()`: 鑾峰彇鍙苟琛屼氦鏄?

**工作原理**:- `get_stats()`: 鑾峰彇鎵ц缁熻

1. 每个工作线程有自己的**本地队列** (FIFO)

2. 线程首先从本地队列获取任务---

3. 本地队列为空时，从**全局队列**批量获取

4. 全局队列也为空时，从其他线程**窃取**任务### 7. WorkStealingScheduler (宸ヤ綔绐冨彇璋冨害鍣?

5. 使用 Rayon 线程池实现并行执行

**鐢ㄩ€?*: 浣跨敤宸ヤ綔绐冨彇绠楁硶瀹炵幇璐熻浇鍧囪　鐨勫苟琛岃皟搴?

**核心方法**:

- `new(num_workers)`: 创建调度器```rust

- `submit_task(task)`: 提交单个任务pub struct WorkStealingScheduler {

- `submit_tasks(tasks)`: 批量提交任务    injector: Arc<Injector<Task>>,       // 鍏ㄥ眬浠诲姟闃熷垪

- `execute_all<F>(executor)`: 并行执行所有任务    stealers: Vec<Stealer<Task>>,        // 绐冨彇鍣ㄥ垪琛?

- `get_scheduler()`: 获取底层 ParallelScheduler    scheduler: Arc<ParallelScheduler>,   // 搴曞眰璋冨害鍣?

- `get_stats()`: 获取执行统计    num_workers: usize,                  // 宸ヤ綔绾跨▼鏁?

}

**优势**:

- ✅ **负载均衡**: 自动平衡线程间的工作量pub struct Task {

- ✅ **高吞吐量**: 减少线程空闲时间    pub tx_id: TxId,

- ✅ **可扩展性**: 支持任意数量的工作线程    pub priority: u8,  // 0-255,瓒婂ぇ浼樺厛绾ц秺楂?

- ✅ **优先级支持**: 可按优先级调度任务}

```

---

**宸ヤ綔鍘熺悊**:

### 8. Batch Operations (批量操作)1. 姣忎釜宸ヤ綔绾跨▼鏈夎嚜宸辩殑**鏈湴闃熷垪** (FIFO)

2. 绾跨▼棣栧厛浠庢湰鍦伴槦鍒楄幏鍙栦换鍔?

**动机**: 批量化减少锁获取与快照创建/提交的次数，提升高并发场景下的吞吐量。3. 鏈湴闃熷垪涓虹┖鏃?浠?*鍏ㄥ眬闃熷垪**鎵归噺鑾峰彇

4. 鍏ㄥ眬闃熷垪涔熶负绌烘椂,浠庡叾浠栫嚎绋?*绐冨彇**浠诲姟

**StateManager 批量 API**:5. 浣跨敤 Rayon 绾跨▼姹犲疄鐜板苟琛屾墽琛?

- `batch_write(Vec<(Vec<u8>, Vec<u8>)>) -> usize`

- `batch_read(&[Vec<u8>]) -> Vec<(Vec<u8>, Vec<u8>)>`**鏍稿績鏂规硶**:

- `batch_delete(&[Vec<u8>]) -> usize`- `new(num_workers)`: 鍒涘缓璋冨害鍣?

- `batch_emit_events(Vec<Vec<u8>>) -> usize`- `submit_task(task)`: 鎻愪氦鍗曚釜浠诲姟

- `submit_tasks(tasks)`: 鎵归噺鎻愪氦浠诲姟

**ParallelScheduler 批量 API**:- `execute_all<F>(executor)`: 骞惰鎵ц鎵€鏈変换鍔?

- `execute_batch<Vec<T>>(Vec<F>)`: 在单一快照中执行多笔交易，任一失败则整批回滚- `get_scheduler()`: 鑾峰彇搴曞眰 ParallelScheduler

- 直通批量存储接口：`batch_write/read/delete`- `get_stats()`: 鑾峰彇鎵ц缁熻



**示例**:**浼樺娍**:

```rust- 鉁?**璐熻浇鍧囪　**: 鑷姩骞宠　绾跨▼闂寸殑宸ヤ綔閲?

// 批量执行三笔转账，任一失败则整批回滚- 鉁?**楂樺悶鍚愰噺**: 鍑忓皯绾跨▼绌洪棽鏃堕棿

let results = scheduler.execute_batch(vec![- 鉁?**鍙墿灞曟€?*: 鏀寔浠绘剰鏁伴噺鐨勫伐浣滅嚎绋?

    Box::new(|m: &StateManager| { /* tx1 */ Ok(1) }) as Box<dyn FnOnce(&StateManager) -> Result<i32, String>>,- 鉁?**浼樺厛绾ф敮鎸?*: 鍙寜浼樺厛绾ц皟搴︿换鍔?

    Box::new(|m: &StateManager| { /* tx2 */ Ok(2) }),

    Box::new(|m: &StateManager| { /* tx3 */ Ok(3) }),---

])?;

```### 8. Batch Operations (鎵归噺鎿嶄綔)



---**鍔ㄦ満**: 鎵归噺鍖栧噺灏戦攣鑾峰彇涓庡揩鐓у垱寤?鎻愪氦鐨勬鏁帮紝鎻愬崌楂樺苟鍙戝満鏅笅鐨勫悶鍚愰噺銆?



## 关键 API**StateManager 鎵归噺 API**:

- `batch_write(Vec<(Vec<u8>, Vec<u8>)>) -> usize`

### 基础使用- `batch_read(&[Vec<u8>]) -> Vec<(Vec<u8>, Vec<u8>)>`

- `batch_delete(&[Vec<u8>]) -> usize`

```rust- `batch_emit_events(Vec<Vec<u8>>) -> usize`

use vm_runtime::ParallelScheduler;

**ParallelScheduler 鎵归噺 API**:

// 创建调度器- `execute_batch<Vec<T>>(Vec<F>)`: 鍦ㄥ崟涓€蹇収涓墽琛屽绗斾氦鏄擄紝浠讳竴澶辫触鍒欐暣鎵瑰洖婊?

let scheduler = ParallelScheduler::new();- 鐩撮€氭壒閲忓瓨鍌ㄦ帴鍙ｏ細`batch_write/read/delete`



// 使用快照保护执行交易**绀轰緥**:

let result = scheduler.execute_with_snapshot(|manager| {```rust

    let storage = manager.get_storage();// 鎵归噺鎵ц涓夌瑪杞处锛屼换涓€澶辫触鍒欐暣鎵瑰洖婊?

    let mut storage = storage.lock().unwrap();let results = scheduler.execute_batch(vec![

        Box::new(|m: &StateManager| { /* tx1 */ Ok(1) }) as Box<dyn FnOnce(&StateManager) -> Result<i32, String>>,

    // 执行交易逻辑    Box::new(|m: &StateManager| { /* tx2 */ Ok(2) }),

    storage.insert(b"balance".to_vec(), b"100".to_vec());    Box::new(|m: &StateManager| { /* tx3 */ Ok(3) }),

    ])?;

    Ok(()) // 返回 Ok 则提交，Err 则回滚```

})?;

```---



### 自动重试## API 鍙傝€?



```rust### 鍩虹浣跨敤

// 失败时自动重试

let result = scheduler.execute_with_retry(```rust

    |manager| {use vm_runtime::ParallelScheduler;

        // 可能失败的操作

        if some_condition() {// 鍒涘缓璋冨害鍣?

            return Err("Temporary failure".to_string());let scheduler = ParallelScheduler::new();

        }

        Ok(42)// 浣跨敤蹇収淇濇姢鎵ц浜ゆ槗

    },let result = scheduler.execute_with_snapshot(|manager| {

    max_retries: 3  // 最多重试 3 次    let storage = manager.get_storage();

)?;    let mut storage = storage.lock().unwrap();

```    

    // 鎵ц浜ゆ槗閫昏緫

### 获取统计    storage.insert(b"balance".to_vec(), b"100".to_vec());

    

```rust    Ok(()) // 杩斿洖 Ok 鍒欐彁浜わ紝Err 鍒欏洖婊?

let stats = scheduler.get_stats();})?;

```

println!("总交易数: {}", stats.total_txs());

println!("成功率: {:.2}%", stats.success_rate() * 100.0);### 鑷姩閲嶈瘯

println!("回滚率: {:.2}%", stats.rollback_rate() * 100.0);

println!("重试次数: {}", stats.retry_count);```rust

```// 澶辫触鏃惰嚜鍔ㄩ噸璇?

let result = scheduler.execute_with_retry(

### 并行批次调度    |manager| {

        // 鍙兘澶辫触鐨勬搷浣?

```rust        if some_condition() {

use vm_runtime::{ReadWriteSet, ConflictDetector};            return Err("Temporary failure".to_string());

        }

let scheduler = ParallelScheduler::new();        Ok(42)

    },

// 记录交易读写集    max_retries: 3  // 鏈€澶氶噸璇?3 娆?

for (tx_id, rw_set) in transactions {)?;

    scheduler.record_rw_set(tx_id, rw_set);```

}

### 鑾峰彇缁熻

// 获取可并行执行的交易

let all_txs: Vec<u64> = vec![1, 2, 3, 4, 5];```rust

let ready_txs = scheduler.get_parallel_batch(&all_txs);let stats = scheduler.get_stats();



// ready_txs 包含所有可以并行执行的交易println!("鎬讳氦鏄撴暟: {}", stats.total_txs());

println!("可并行执行: {:?}", ready_txs);println!("鎴愬姛鐜? {:.2}%", stats.success_rate() * 100.0);

```println!("鍥炴粴鐜? {:.2}%", stats.rollback_rate() * 100.0);

println!("閲嶈瘯娆℃暟: {}", stats.retry_count);

### 工作窃取调度```



```rust### 骞惰鎵规璋冨害

use vm_runtime::{WorkStealingScheduler, Task};

```rust

// 创建工作窃取调度器(4 个工作线程)use vm_runtime::{ReadWriteSet, ConflictDetector};

let scheduler = WorkStealingScheduler::new(Some(4));

let scheduler = ParallelScheduler::new();

// 提交任务

let tasks = vec![// 璁板綍浜ゆ槗璇诲啓闆?

    Task::new(1, 255),  // 高优先级for (tx_id, rw_set) in transactions {

    Task::new(2, 128),  // 中优先级    scheduler.record_rw_set(tx_id, rw_set);

    Task::new(3, 50),   // 低优先级}

];

scheduler.submit_tasks(tasks);// 鑾峰彇鍙苟琛屾墽琛岀殑浜ゆ槗

let all_txs: Vec<u64> = vec![1, 2, 3, 4, 5];

// 并行执行所有任务let ready_txs = scheduler.get_parallel_batch(&all_txs);

let result = scheduler.execute_all(|tx_id| {

    // 执行任务逻辑// ready_txs 鍖呭惈鎵€鏈夊彲浠ュ苟琛屾墽琛岀殑浜ゆ槗

    println!("Processing transaction {}", tx_id);println!("鍙苟琛屾墽琛? {:?}", ready_txs);

    Ok(())```

})?;

### 宸ヤ綔绐冨彇璋冨害

println!("Executed: {:?}", result);

``````rust

use vm_runtime::{WorkStealingScheduler, Task};

---

// 鍒涘缓宸ヤ綔绐冨彇璋冨害鍣?(4 涓伐浣滅嚎绋?

## 示例let scheduler = WorkStealingScheduler::new(Some(4));



### 示例 1: 转账交易// 鎻愪氦浠诲姟

let tasks = vec![

```rust    Task::new(1, 255),  // 楂樹紭鍏堢骇

use vm_runtime::ParallelScheduler;    Task::new(2, 128),  // 涓紭鍏堢骇

    Task::new(3, 50),   // 浣庝紭鍏堢骇

let scheduler = ParallelScheduler::new();];

scheduler.submit_tasks(tasks);

// Alice 转账给 Bob

let result = scheduler.execute_with_snapshot(|manager| {// 骞惰鎵ц鎵€鏈変换鍔?

    let storage = manager.get_storage();let result = scheduler.execute_all(|tx_id| {

    let mut storage = storage.lock().unwrap();    // 鎵ц浠诲姟閫昏緫

        println!("Processing transaction {}", tx_id);

    // 读取 Alice 余额    Ok(())

    let alice_balance: u64 = storage.get(b"alice")})?;

        .and_then(|b| String::from_utf8(b.clone()).ok())

        .and_then(|s| s.parse().ok())println!("Executed: {:?}", result);

        .unwrap_or(0);```

    

    // 检查余额---

    if alice_balance < 50 {

        return Err("Insufficient balance".to_string());## 浣跨敤绀轰緥

    }

    ### 绀轰緥 1: 杞处浜ゆ槗

    // 更新余额

    storage.insert(b"alice".to_vec(), (alice_balance - 50).to_string().into_bytes());```rust

    use vm_runtime::ParallelScheduler;

    let bob_balance: u64 = storage.get(b"bob")

        .and_then(|b| String::from_utf8(b.clone()).ok())let scheduler = ParallelScheduler::new();

        .and_then(|s| s.parse().ok())

        .unwrap_or(0);// Alice 杞处缁?Bob

    let result = scheduler.execute_with_snapshot(|manager| {

    storage.insert(b"bob".to_vec(), (bob_balance + 50).to_string().into_bytes());    let storage = manager.get_storage();

        let mut storage = storage.lock().unwrap();

    Ok(())    

})?;    // 璇诲彇 Alice 浣欓

```    let alice_balance: u64 = storage.get(b"alice")

        .and_then(|b| String::from_utf8(b.clone()).ok())

### 示例 2: 冲突检测        .and_then(|s| s.parse().ok())

        .unwrap_or(0);

```rust    

use vm_runtime::{ReadWriteSet, ConflictDetector};    // 妫€鏌ヤ綑棰?

    if alice_balance < 50 {

let mut detector = ConflictDetector::new();        return Err("Insufficient balance".to_string());

    }

// 交易 1: Alice -> Bob    

let mut tx1_rw = ReadWriteSet::new();    // 鏇存柊浣欓

tx1_rw.add_read(b"alice".to_vec());    storage.insert(b"alice".to_vec(), (alice_balance - 50).to_string().into_bytes());

tx1_rw.add_write(b"alice".to_vec());    

tx1_rw.add_write(b"bob".to_vec());    let bob_balance: u64 = storage.get(b"bob")

detector.record(1, tx1_rw);        .and_then(|b| String::from_utf8(b.clone()).ok())

        .and_then(|s| s.parse().ok())

// 交易 2: Bob -> Charlie (与 tx1 冲突)        .unwrap_or(0);

let mut tx2_rw = ReadWriteSet::new();    

tx2_rw.add_read(b"bob".to_vec());   // 读 bob，与 tx1 写冲突    storage.insert(b"bob".to_vec(), (bob_balance + 50).to_string().into_bytes());

tx2_rw.add_write(b"bob".to_vec());    

tx2_rw.add_write(b"charlie".to_vec());    Ok(())

detector.record(2, tx2_rw);})?;

```

// 交易 3: David -> Eve (无冲突)

let mut tx3_rw = ReadWriteSet::new();### 绀轰緥 2: 鍐茬獊妫€娴?

tx3_rw.add_write(b"david".to_vec());

tx3_rw.add_write(b"eve".to_vec());```rust

detector.record(3, tx3_rw);use vm_runtime::{ReadWriteSet, ConflictDetector};



// 检测冲突let mut detector = ConflictDetector::new();

assert!(detector.has_conflict(1, 2));  // tx1 和 tx2 冲突

assert!(!detector.has_conflict(1, 3)); // tx1 和 tx3 不冲突// 浜ゆ槗 1: Alice -> Bob

assert!(!detector.has_conflict(2, 3)); // tx2 和 tx3 不冲突let mut tx1_rw = ReadWriteSet::new();

tx1_rw.add_read(b"alice".to_vec());

// tx1 和 tx3 可以并行执行，tx2 必须等待 tx1tx1_rw.add_write(b"alice".to_vec());

```tx1_rw.add_write(b"bob".to_vec());

detector.record(1, tx1_rw);

### 示例 3: 嵌套快照

// 浜ゆ槗 2: Bob -> Charlie (涓?tx1 鍐茬獊)

```rustlet mut tx2_rw = ReadWriteSet::new();

let scheduler = ParallelScheduler::new();tx2_rw.add_read(b"bob".to_vec());   // 璇?bob锛屼笌 tx1 鍐欏啿绐?

tx2_rw.add_write(b"bob".to_vec());

// 外层交易tx2_rw.add_write(b"charlie".to_vec());

scheduler.execute_with_snapshot(|manager| {detector.record(2, tx2_rw);

    let storage = manager.get_storage();

    let mut storage = storage.lock().unwrap();// 浜ゆ槗 3: David -> Eve (鏃犲啿绐?

    storage.insert(b"level".to_vec(), b"1".to_vec());let mut tx3_rw = ReadWriteSet::new();

    tx3_rw.add_write(b"david".to_vec());

    // 可以在这里执行更多嵌套交易tx3_rw.add_write(b"eve".to_vec());

    // 每个都有自己的快照detector.record(3, tx3_rw);

    

    Ok(())// 妫€娴嬪啿绐?

})?;assert!(detector.has_conflict(1, 2));  // tx1 鍜?tx2 鍐茬獊

```assert!(!detector.has_conflict(1, 3)); // tx1 鍜?tx3 涓嶅啿绐?

assert!(!detector.has_conflict(2, 3)); // tx2 鍜?tx3 涓嶅啿绐?

---

// tx1 鍜?tx3 鍙互骞惰鎵ц锛宼x2 蹇呴』绛夊緟 tx1

## 性能优化```



### 1. 最小化锁争用### 绀轰緥 3: 宓屽蹇収



```rust```rust

// ❌ 不好 - 长时间持有锁let scheduler = ParallelScheduler::new();

let mut storage = manager.get_storage().lock().unwrap();

expensive_computation();// 澶栧眰浜ゆ槗

storage.insert(...);scheduler.execute_with_snapshot(|manager| {

    let storage = manager.get_storage();

// ✅ 好 - 只在必要时持有锁    let mut storage = storage.lock().unwrap();

let data = expensive_computation();    storage.insert(b"level".to_vec(), b"1".to_vec());

{    

    let storage = manager.get_storage();    // 鍙互鍦ㄨ繖閲屾墽琛屾洿澶氬祵濂椾氦鏄?

    let mut storage = storage.lock().unwrap();    // 姣忎釜閮芥湁鑷繁鐨勫揩鐓?

    storage.insert(...);    

}    Ok(())

```})?;

```

### 2. 批量操作

---

```rust

// 批量记录读写集## 鎬ц兘浼樺寲

for (tx_id, rw_set) in transactions.iter() {

    scheduler.record_rw_set(*tx_id, rw_set.clone());### 1. 鏈€灏忓寲閿佷簤鐢?

}

```rust

// 一次性获取可并行批次// 鉂?涓嶅ソ - 闀挎椂闂存寔鏈夐攣

let ready_batch = scheduler.get_parallel_batch(&all_tx_ids);let mut storage = manager.get_storage().lock().unwrap();

```expensive_computation();

storage.insert(...);

### 3. 避免不必要的快照

// 鉁?濂?- 鍙湪蹇呰鏃舵寔鏈夐攣

```rustlet data = expensive_computation();

// 只读操作不需要快照{

let storage = scheduler.get_storage();    let storage = manager.get_storage();

let storage = storage.lock().unwrap();    let mut storage = storage.lock().unwrap();

let value = storage.get(b"key");    storage.insert(...);

}

// 写操作才需要快照保护```

scheduler.execute_with_snapshot(|manager| {

    // 修改状态### 2. 鎵归噺鎿嶄綔

    Ok(())

})?;```rust

```// 鎵归噺璁板綍璇诲啓闆?

for (tx_id, rw_set) in transactions.iter() {

### 4. 常见瓶颈分析    scheduler.record_rw_set(*tx_id, rw_set.clone());

}

- **锁竞争（Mutex 争用）**

    - 症状: 高并发下延迟抖动、尾延迟上升// 涓€娆℃€ц幏鍙栧彲骞惰鎵规

    - 缓解: 采用批量读写、缩短持锁区间、必要时细化锁粒度let ready_batch = scheduler.get_parallel_batch(&all_tx_ids);

- **快照创建/回滚开销**```

    - 症状: 大事务或深度嵌套时耗时上升

    - 缓解: 合理分批、减少不必要的嵌套、将只读路径移出快照### 3. 閬垮厤涓嶅繀瑕佺殑蹇収

- **依赖图过度密集**

    - 症状: 可并行度下降、批次变小```rust

    - 缓解: 通过读写集设计减少交叉访问、对热门键做分片// 鍙鎿嶄綔涓嶉渶瑕佸揩鐓?

- **调度开销（工作窃取）**let storage = scheduler.get_storage();

    - 症状: 任务极短时调度成本相对偏高let storage = storage.lock().unwrap();

    - 缓解: 合并小任务为批处理、提升每个任务的工作量let value = storage.get(b"key");



---// 鍐欐搷浣滄墠闇€瑕佸揩鐓т繚鎶?

scheduler.execute_with_snapshot(|manager| {

## 测试与基准    // 淇敼鐘舵€?

    Ok(())

### 单元测试覆盖})?;

```

**冲突检测** (6 个测试):

- ✅ test_read_write_set_conflicts### 4. 甯歌鐡堕鍒嗘瀽

- ✅ test_no_conflict

- ✅ test_dependency_graph- 閿佺珵浜夛紙Mutex 浜夌敤锛?

- ✅ test_conflict_detector    - 鐥囩姸: 楂樺苟鍙戜笅寤惰繜鎶栧姩銆佸熬寤惰繜涓婂崌

    - 缂撹В: 閲囩敤鎵归噺鍐?璇汇€佺缉鐭寔閿佸尯闂淬€佸繀瑕佹椂缁嗗寲閿佺矑搴?

**状态快照** (5 个测试):- 蹇収鍒涘缓/鍥炴粴寮€閿€

- ✅ test_snapshot_creation    - 鐥囩姸: 澶т簨鍔℃垨娣卞害宓屽鏃惰€楁椂涓婂崌

- ✅ test_rollback    - 缂撹В: 鍚堢悊鍒嗘壒銆佸噺灏戜笉蹇呰鐨勫祵濂椼€佸皢鍙璺緞绉诲嚭蹇収

- ✅ test_nested_snapshots- 渚濊禆鍥捐繃搴﹀瘑闆?

- ✅ test_commit    - 鐥囩姸: 鍙苟琛屽害涓嬮檷銆佹壒娆″彉灏?

- ✅ test_snapshot_with_events    - 缂撹В: 閫氳繃璇诲啓闆嗚璁″噺灏戜氦鍙夎闂€佸鐑棬閿仛鍒嗙墖

- 璋冨害寮€閿€锛堝伐浣滅獌鍙栵級

**调度器集成** (3 个测试):    - 鐥囩姸: 浠诲姟鏋佺煭鏃惰皟搴︽垚鏈浉瀵瑰亸楂?

- ✅ test_scheduler_with_snapshot    - 缂撹В: 鍚堝苟灏忎换鍔′负鎵瑰鐞嗐€佹彁鍗囨瘡涓换鍔＄殑宸ヤ綔閲?

- ✅ test_scheduler_rollback_on_error

- ✅ test_scheduler_nested_transactions---



**统计与重试** (3 个测试):## 娴嬭瘯楠岃瘉

- ✅ test_execution_stats

- ✅ test_retry_mechanism### 鍗曞厓娴嬭瘯瑕嗙洊

- ✅ test_retry_exhausted

**鍐茬獊妫€娴?* (6 涓祴璇?:

### 基准测试- 鉁?test_read_write_set_conflicts

- 鉁?test_no_conflict

运行基准测试:- 鉁?test_dependency_graph

```bash- 鉁?test_conflict_detector

cargo bench --bench parallel_benchmark

```**鐘舵€佸揩鐓?* (5 涓祴璇?:

- 鉁?test_snapshot_creation

测试场景:- 鉁?test_rollback

1. **冲突检测性能**: 10/50/100/500 交易- 鉁?test_nested_snapshots

2. **快照操作性能**: 10/100/1000 数据项- 鉁?test_commit

3. **依赖图构建**: 不同冲突率- 鉁?test_snapshot_with_events

4. **并行调度**: 批次大小优化

**璋冨害鍣ㄩ泦鎴?* (3 涓祴璇?:

#### 如何阅读报告- 鉁?test_scheduler_with_snapshot

- 鉁?test_scheduler_rollback_on_error

- 打开 HTML 报告: `target/criterion/report/index.html`- 鉁?test_scheduler_nested_transactions

- estimates.json 字段:

    - mean/median: 平均/中位数耗时**缁熻涓庨噸璇?* (3 涓祴璇?:

    - slope: 线性拟合的趋势估计- 鉁?test_execution_stats

    - std_dev: 标准差（抖动）- 鉁?test_retry_mechanism

    - confidence_interval: 置信区间（默认 95%）- 鉁?test_retry_exhausted

- 单位: ns/iter（Criterion 默认单位）

### 鍩哄噯娴嬭瘯

#### 示例指标（节选）

杩愯鍩哄噯娴嬭瘯:

- 并行调度 get_parallel_batch/100: 平均约 350,045 ns/批```bash

- 冲突检测 non_conflicting/100: 平均约 396,673 nscargo bench --bench parallel_benchmark

- 冲突检测 50% 冲突/100: 平均约 460,675 ns```

- 快照创建 create_snapshot/1000: 平均约 224,712 ns

- 依赖图 build_and_query/100: 平均约 344,862 ns娴嬭瘯鍦烘櫙:

1. **鍐茬獊妫€娴嬫€ц兘**: 10/50/100/500 浜ゆ槗

---2. **蹇収鎿嶄綔鎬ц兘**: 10/100/1000 鏁版嵁椤?

3. **渚濊禆鍥炬瀯寤?*: 涓嶅悓鍐茬獊鐜?

## 最佳实践4. **骞惰璋冨害**: 鎵规澶у皬浼樺寲



### 1. 错误处理#### 濡備綍闃呰鎶ュ憡



```rust- 鎵撳紑 HTML 鎶ュ憡: `target/criterion/report/index.html`

match scheduler.execute_with_snapshot(|manager| {- estimates.json 瀛楁:

    // 交易逻辑    - mean/median: 骞冲潎/涓綅鏁拌€楁椂

    Ok(())    - slope: 绾挎€ф嫙鍚堢殑瓒嬪娍浼拌

}) {    - std_dev: 鏍囧噯宸紙鎶栧姩锛?

    Ok(_) => println!("✅ 交易成功"),    - confidence_interval: 缃俊鍖洪棿锛堥粯璁?95%锛?

    Err(e) => eprintln!("❌ 交易失败: {}", e),- 鍗曚綅: ns/iter锛圕riterion 榛樿鍗曚綅锛?

}

```#### 绀轰緥鎸囨爣锛堣妭閫夛級



### 2. 监控统计- 骞惰璋冨害 get_parallel_batch/100: 骞冲潎绾?350,045 ns/鎵?

- 鍐茬獊妫€娴?non_conflicting/100: 骞冲潎绾?396,673 ns

```rust- 鍐茬獊妫€娴?50% 鍐茬獊/100: 骞冲潎绾?460,675 ns

// 定期检查统计信息- 蹇収鍒涘缓 create_snapshot/1000: 骞冲潎绾?224,712 ns

let stats = scheduler.get_stats();- 渚濊禆鍥?build_and_query/100: 骞冲潎绾?344,862 ns

if stats.rollback_rate() > 0.5 {

    eprintln!("⚠️  高回滚率: {:.2}%", stats.rollback_rate() * 100.0);---

}

```## 鏈€浣冲疄璺?



### 3. 重试策略### 1. 閿欒澶勭悊



```rust```rust

// 根据错误类型决定是否重试match scheduler.execute_with_snapshot(|manager| {

let result = scheduler.execute_with_retry(    // 浜ゆ槗閫昏緫

    |manager| {    Ok(())

        match try_transaction(manager) {}) {

            Ok(r) => Ok(r),    Ok(_) => println!("鉁?浜ゆ槗鎴愬姛"),

            Err(e) if e.is_retriable() => Err(e.to_string()),    Err(e) => eprintln!("鉂?浜ゆ槗澶辫触: {}", e),

            Err(e) => return Err(e.to_string()), // 不可重试错误}

        }```

    },

    max_retries: 5### 2. 鐩戞帶缁熻

);

``````rust

// 瀹氭湡妫€鏌ョ粺璁′俊鎭?

---let stats = scheduler.get_stats();

if stats.rollback_rate() > 0.5 {

## MVCC 存储后端 (v0.5.0) 🔒    eprintln!("鈿狅笍  楂樺洖婊氱巼: {:.2}%", stats.rollback_rate() * 100.0);

}

### 什么是 MVCC？```



MVCC (Multi-Version Concurrency Control，多版本并发控制) 是一种并发控制方法，允许多个事务同时访问数据库而不互相阻塞。每个键维护多个版本，事务读取其启动时刻的快照，写入创建新版本。### 3. 閲嶈瘯绛栫暐



### 何时使用 MVCC？```rust

// 鏍规嵁閿欒绫诲瀷鍐冲畾鏄惁閲嶈瘯

**推荐使用 MVCC 的场景**:let result = scheduler.execute_with_retry(

- ✅ 高并发读写混合负载    |manager| {

- ✅ 长事务与短事务混合        match try_transaction(manager) {

- ✅ 需要快照隔离语义            Ok(r) => Ok(r),

- ✅ 查询密集型应用（使用只读事务优化）            Err(e) if e.is_retriable() => Err(e.to_string()),

            Err(e) => return Err(e.to_string()), // 涓嶅彲閲嶈瘯閿欒

**推荐使用 Snapshot 的场景**:        }

- ✅ 简单串行执行    },

- ✅ 短事务为主    max_retries: 5

- ✅ 内存敏感场景（MVCC 会保留多版本）);

- ✅ 不需要高并发```



### 创建 MVCC 调度器---



```rust## MVCC 瀛樺偍鍚庣 (v0.5.0) 馃攼

use vm_runtime::{ParallelScheduler, MvccStore};

use std::sync::Arc;### 浠€涔堟槸 MVCC锛?



// 创建 MVCC 存储MVCC (Multi-Version Concurrency Control锛屽鐗堟湰骞跺彂鎺у埗) 鏄竴绉嶅苟鍙戞帶鍒舵柟娉曪紝鍏佽澶氫釜浜嬪姟鍚屾椂璁块棶鏁版嵁搴撹€屼笉浜掔浉闃诲銆傛瘡涓敭缁存姢澶氫釜鐗堟湰锛屼簨鍔¤鍙栧叾鍚姩鏃跺埢鐨勫揩鐓э紝鍐欏叆鍒涘缓鏂扮増鏈€?

let mvcc_store = Arc::new(MvccStore::new());

### 浣曟椂浣跨敤 MVCC锛?

// 创建使用 MVCC 后端的调度器

let scheduler = ParallelScheduler::new_with_mvcc(Arc::clone(&mvcc_store));**鎺ㄨ崘浣跨敤 MVCC 鐨勫満鏅?*:

- 鉁?楂樺苟鍙戣鍐欐贩鍚堣礋杞?

// 执行读写事务- 鉁?闀夸簨鍔′笌鐭簨鍔℃贩鍚?

scheduler.execute_with_mvcc(|txn| {- 鉁?闇€瑕佸揩鐓ч殧绂昏涔?

    // 读取数据- 鉁?鏌ヨ瀵嗛泦鍨嬪簲鐢紙浣跨敤鍙浜嬪姟浼樺寲锛?

    if let Some(balance) = txn.read(b"balance") {

        println!("Balance: {:?}", balance);**鎺ㄨ崘浣跨敤 Snapshot 鐨勫満鏅?*:

    }- 鉁?绠€鍗曚覆琛屾墽琛?

    - 鉁?鐭簨鍔′负涓?

    // 写入数据（本地缓存）- 鉁?鍐呭瓨鏁忔劅鍦烘櫙锛圡VCC 浼氫繚鐣欏鐗堟湰锛?

    txn.write(b"balance".to_vec(), b"100".to_vec());- 鉁?涓嶉渶瑕侀珮骞跺彂

    

    // 成功返回，自动提交### 鍒涘缓 MVCC 璋冨害鍣?

    Ok(())

})?;```rust

use vm_runtime::{ParallelScheduler, MvccStore};

// 执行只读事务（快速路径，无冲突检测）use std::sync::Arc;

let result = scheduler.execute_with_mvcc_read_only(|txn| {

    let balance = txn.read(b"balance")?// 鍒涘缓 MVCC 瀛樺偍

        .ok_or("Balance not found")?;let mvcc_store = Arc::new(MvccStore::new());

    

    Ok(balance)// 鍒涘缓浣跨敤 MVCC 鍚庣鐨勮皟搴﹀櫒

})?;let scheduler = ParallelScheduler::new_with_mvcc(Arc::clone(&mvcc_store));

```

// 鎵ц璇诲啓浜嬪姟

### MVCC 特性scheduler.execute_with_mvcc(|txn| {

    // 璇诲彇鏁版嵁

**快照隔离 (Snapshot Isolation)**:    if let Some(balance) = txn.read(b"balance") {

- 每个事务看到启动时刻的数据快照        println!("Balance: {:?}", balance);

- 读取不会被写入阻塞    }

- 写入不会阻塞读取    

    // 鍐欏叆鏁版嵁锛堟湰鍦扮紦瀛橈級

**写写冲突检测**:    txn.write(b"balance".to_vec(), b"100".to_vec());

```rust    

let store = Arc::new(MvccStore::new());    // 鎴愬姛杩斿洖锛岃嚜鍔ㄦ彁浜?

    Ok(())

// 事务 1 和 2 并发写同一键})?;

let mut t1 = store.begin();

let mut t2 = store.begin();// 鎵ц鍙浜嬪姟锛堝揩閫熻矾寰勶紝鏃犲啿绐佹娴嬶級

let result = scheduler.execute_with_mvcc_read_only(|txn| {

t1.write(b"key".to_vec(), b"value1".to_vec());    let balance = txn.read(b"balance")?

t2.write(b"key".to_vec(), b"value2".to_vec());        .ok_or("Balance not found")?;

    

// 第一个提交成功    Ok(balance)

t1.commit()?;})?;

```

// 第二个提交失败（写写冲突）

assert!(t2.commit().is_err());### MVCC 鐗规€?

```

**蹇収闅旂 (Snapshot Isolation)**:

**只读事务优化**:- 姣忎釜浜嬪姟鐪嬪埌鍚姩鏃跺埢鐨勬暟鎹揩鐓?

```rust- 璇诲彇涓嶄細琚啓鍏ラ樆濉?

// 只读事务使用快速路径- 鍐欏叆涓嶄細闃诲璇诲彇

let ro_txn = store.begin_read_only();

**鍐欏啓鍐茬獊妫€娴?*:

// 可以读取多个键```rust

let val1 = ro_txn.read(b"key1");let store = Arc::new(MvccStore::new());

let val2 = ro_txn.read(b"key2");

// 浜嬪姟 1 鍜?2 骞跺彂鍐欏悓涓€閿?

// 提交无需冲突检测，直接返回let mut t1 = store.begin();

let start_ts = ro_txn.commit()?;let mut t2 = store.begin();



// ❌ 只读事务不能写入（会 panic）t1.write(b"key".to_vec(), b"value1".to_vec());

// ro_txn.write(...); // panic!t2.write(b"key".to_vec(), b"value2".to_vec());

```

// 绗竴涓彁浜ゆ垚鍔?

**细粒度并发控制**:t1.commit()?;

- DashMap 无锁哈希表，减少全局锁竞争

- 每键 RwLock，允许多个事务并发读取同一键// 绗簩涓彁浜ゅけ璐ワ紙鍐欏啓鍐茬獊锛?

- 提交时按键排序加锁，避免死锁assert!(t2.commit().is_err());

- 原子时间戳分配，消除时间戳瓶颈```



### MVCC vs Snapshot 性能对比**鍙浜嬪姟浼樺寲**:

```rust

运行 MVCC 基准测试:// 鍙浜嬪姟浣跨敤蹇€熻矾寰?

```bashlet ro_txn = store.begin_read_only();

cargo bench --bench parallel_benchmark -- mvcc

```// 鍙互璇诲彇澶氫釜閿?

let val1 = ro_txn.read(b"key1");

**典型性能特征**:let val2 = ro_txn.read(b"key2");

- **只读事务**: MVCC 快速路径比 Snapshot 快 2-5 倍

- **并发读取**: MVCC 允许无锁并发，Snapshot 需要锁// 鎻愪氦鏃犻渶鍐茬獊妫€娴嬶紝鐩存帴杩斿洖

- **写入性能**: 无冲突时性能相近，MVCC 略有开销（版本管理）let start_ts = ro_txn.commit()?;

- **冲突场景**: MVCC 在提交时检测，Snapshot 在锁获取时阻塞

// 鉂?鍙浜嬪姟涓嶈兘鍐欏叆锛堜細 panic锛?

---// ro_txn.write(...); // panic!

```

## 未来优化

**缁嗙矑搴﹀苟鍙戞帶鍒?*:

### MVCC 垃圾回收 (v0.6.0) 🗑️- DashMap 鏃犻攣鍝堝笇琛紝鍑忓皯鍏ㄥ眬閿佺珵浜?

- 姣忛敭 RwLock锛屽厑璁稿涓簨鍔″苟鍙戣鍙栧悓涓€閿?

#### 为什么需要 GC？- 鎻愪氦鏃舵寜閿帓搴忓姞閿侊紝閬垮厤姝婚攣

- 鍘熷瓙鏃堕棿鎴冲垎閰嶏紝娑堥櫎鏃堕棿鎴崇摱棰?

MVCC 为每个键维护多个版本，随着事务的执行，版本数会不断增长。如果不清理旧版本会：

- **内存占用**持续增加### MVCC vs Snapshot 鎬ц兘瀵规瘮

- **查找性能**下降（版本链过长）

- **存储开销**失控杩愯 MVCC 鍩哄噯娴嬭瘯:

```bash

#### GC 配置cargo bench --bench parallel_benchmark -- mvcc

```

```rust

use vm_runtime::{MvccStore, GcConfig};**鍏稿瀷鎬ц兘鐗瑰緛**:

- **鍙浜嬪姟**: MVCC 蹇€熻矾寰勬瘮 Snapshot 蹇?2-5 鍊?

let config = GcConfig {- **骞跺彂璇诲彇**: MVCC 鍏佽鏃犻攣骞跺彂锛孲napshot 闇€瑕侀攣

    max_versions_per_key: 10,      // 每个键最多保留 10 个版本- **鍐欏叆鎬ц兘**: 鏃犲啿绐佹椂鎬ц兘鐩歌繎锛孧VCC 鐣ユ湁寮€閿€锛堢増鏈鐞嗭級

    enable_time_based_gc: false,   // 基于时间的 GC（未来功能）- **鍐茬獊鍦烘櫙**: MVCC 鍦ㄦ彁浜ゆ椂妫€娴嬶紝Snapshot 鍦ㄩ攣鑾峰彇鏃堕樆濉?

    version_ttl_secs: 3600,        // 版本过期时间（秒）

};---



let store = MvccStore::new_with_config(config);## 鏈潵浼樺寲

```

### MVCC 鍨冨溇鍥炴敹 (v0.6.0) 馃棏锔?

#### 手动触发 GC

#### 涓轰粈涔堥渶瑕?GC锛?

```rust

// 执行一次 GCMVCC 涓烘瘡涓敭缁存姢澶氫釜鐗堟湰锛岄殢鐫€浜嬪姟鐨勬墽琛岋紝鐗堟湰鏁颁細涓嶆柇澧為暱銆傚鏋滀笉娓呯悊鏃х増鏈細

let cleaned_count = store.gc()?;- **鍐呭瓨鍗犵敤**鎸佺画澧炲姞

println!("清理了 {} 个旧版本", cleaned_count);- **鏌ユ壘鎬ц兘**涓嬮檷锛堢増鏈摼杩囬暱锛?

- **瀛樺偍寮€閿€**澶辨帶

// 获取 GC 统计

let stats = store.get_gc_stats();#### GC 閰嶇疆

println!("GC 执行次数: {}", stats.gc_count);

println!("总清理版本数: {}", stats.versions_cleaned);```rust

println!("清理的键数: {}", stats.keys_cleaned);use vm_runtime::{MvccStore, GcConfig};

println!("最后 GC 时间戳: {}", stats.last_gc_ts);

let config = GcConfig {

// 监控存储状态    max_versions_per_key: 10,      // 姣忎釜閿渶澶氫繚鐣?10 涓増鏈?

println!("当前总版本数: {}", store.total_versions());    enable_time_based_gc: false,   // 鍩轰簬鏃堕棿鐨?GC锛堟湭鏉ュ姛鑳斤級

println!("当前键数量: {}", store.total_keys());    version_ttl_secs: 3600,        // 鐗堟湰杩囨湡鏃堕棿锛堢锛?

println!("最小活跃事务时间戳: {:?}", store.get_min_active_ts());};

```

let store = MvccStore::new_with_config(config);

#### GC 清理策略```



**保留规则**（优先级从高到低）:#### 鎵嬪姩瑙﹀彂 GC

1. **最新版本**: 每个键的最新版本永远保留

2. **活跃事务可见版本**: 所有活跃事务可能读到的版本必须保留```rust

3. **版本数量限制**: 根据 `max_versions_per_key` 清理超量旧版本// 鎵ц涓€娆?GC

let cleaned_count = store.gc()?;

**清理流程**:println!("娓呯悊浜?{} 涓棫鐗堟湰", cleaned_count);

```

对每个键的版本链:// 鑾峰彇 GC 缁熻

  1. 找到最小活跃事务 start_ts (水位线)let stats = store.get_gc_stats();

  2. 保留 ts <= start_ts 的第一个版本及之后的所有版本println!("GC 鎵ц娆℃暟: {}", stats.gc_count);

  3. 在此基础上，根据 max_versions_per_key 限制进一步清理println!("鎬绘竻鐞嗙増鏈暟: {}", stats.versions_cleaned);

  4. 最新版本无条件保留println!("娓呯悊鐨勯敭鏁? {}", stats.keys_cleaned);

```println!("鏈€鍚?GC 鏃堕棿鎴? {}", stats.last_gc_ts);



**示例**:// 鐩戞帶瀛樺偍鐘舵€?

```rustprintln!("褰撳墠鎬荤増鏈暟: {}", store.total_versions());

let store = MvccStore::new_with_config(GcConfig {println!("褰撳墠閿暟閲? {}", store.total_keys());

    max_versions_per_key: 3,println!("鏈€灏忔椿璺冧簨鍔℃椂闂存埑: {:?}", store.get_min_active_ts());

    ..Default::default()```

});

#### GC 娓呯悊绛栫暐

// 写入 5 个版本: ts=1,2,3,4,5

for i in 1..=5 {**淇濈暀瑙勫垯**锛堜紭鍏堢骇浠庨珮鍒颁綆锛?

    let mut txn = store.begin();1. **鏈€鏂扮増鏈?*: 姣忎釜閿殑鏈€鏂扮増鏈案杩滀繚鐣?

    txn.write(b"key".to_vec(), format!("v{}", i).into_bytes());2. **娲昏穬浜嬪姟鍙鐗堟湰**: 鎵€鏈夋椿璺冧簨鍔″彲鑳借鍒扮殑鐗堟湰蹇呴』淇濈暀

    txn.commit()?;3. **鐗堟湰鏁伴噺闄愬埗**: 鏍规嵁 `max_versions_per_key` 娓呯悊瓒呴噺鏃х増鏈?

}

**娓呯悊娴佺▼**:

// 开启一个长事务（start_ts=6，能看到 ts<=6 的版本，即所有版本）```

let long_txn = store.begin();瀵规瘡涓敭鐨勭増鏈摼:

  1. 鎵惧埌鏈€灏忔椿璺冧簨鍔?start_ts (姘翠綅绾?

// 再写入 v6, v7  2. 淇濈暀 ts <= start_ts 鐨勭涓€涓増鏈強涔嬪悗鐨勬墍鏈夌増鏈?

for i in 6..=7 {  3. 鍦ㄦ鍩虹涓婏紝鏍规嵁 max_versions_per_key 闄愬埗杩涗竴姝ユ竻鐞?

    let mut txn = store.begin();  4. 鏈€鏂扮増鏈棤鏉′欢淇濈暀

    txn.write(b"key".to_vec(), format!("v{}", i).into_bytes());```

    txn.commit()?;

}**绀轰緥**:

```rust

// 此时有 7 个版本，最小活跃 ts=6let store = MvccStore::new_with_config(GcConfig {

store.gc()?;    max_versions_per_key: 3,

    ..Default::default()

// GC 后:});

// - 保留 ts=1 (long_txn 的水位线内第一个可见版本)

// - 保留 ts=2,3,4,5,6,7 (都 >= min_active_ts)// 鍐欏叆 5 涓増鏈? ts=1,2,3,4,5

// - 所有版本都被保留，因为 long_txn 仍活跃for i in 1..=5 {

    let mut txn = store.begin();

drop(long_txn); // 结束长事务    txn.write(b"key".to_vec(), format!("v{}", i).into_bytes());

    txn.commit()?;

store.gc()?;}



// GC 后:// 寮€鍚竴涓暱浜嬪姟锛坰tart_ts=6锛岃兘鐪嬪埌 ts<=6 鐨勭増鏈紝鍗虫墍鏈夌増鏈級

// - 没有活跃事务，根据 max_versions_per_key=3let long_txn = store.begin();

// - 保留最新的 3 个版本: ts=5,6,7

// - 清理 ts=1,2,3,4// 鍐嶅啓鍏?v6, v7

```for i in 6..=7 {

    let mut txn = store.begin();

#### 活跃事务跟踪    txn.write(b"key".to_vec(), format!("v{}", i).into_bytes());

    txn.commit()?;

MVCC 自动跟踪活跃事务:}

```rust

// 开始事务时自动注册// 姝ゆ椂鏈?7 涓増鏈紝鏈€灏忔椿璺?ts=6

let txn1 = store.begin();store.gc()?;

let txn2 = store.begin_read_only();

// GC 鍚?

// 查询活跃事务水位线// - 淇濈暀 ts=1 (long_txn 鐨勬按浣嶇嚎鍐呯涓€涓彲瑙佺増鏈?

let min_ts = store.get_min_active_ts();// - 淇濈暀 ts=2,3,4,5,6,7 (閮?>= min_active_ts)

println!("最小活跃 ts: {:?}", min_ts);// - 鎵€鏈夌増鏈兘琚繚鐣欙紝鍥犱负 long_txn 浠嶆椿璺?



// 事务结束时自动注销（drop trait）drop(long_txn); // 缁撴潫闀夸簨鍔?

drop(txn1);

drop(txn2);store.gc()?;



// 现在没有活跃事务// GC 鍚?

assert_eq!(store.get_min_active_ts(), None);// - 娌℃湁娲昏穬浜嬪姟锛屾牴鎹?max_versions_per_key=3

```// - 淇濈暀鏈€鏂扮殑 3 涓増鏈? ts=5,6,7

// - 娓呯悊 ts=1,2,3,4

#### GC 最佳实践```



**1. 定期触发 GC**:#### 娲昏穬浜嬪姟璺熻釜

```rust

// 简单策略：每 N 个事务触发一次MVCC 鑷姩璺熻釜娲昏穬浜嬪姟:

let mut tx_count = 0;```rust

loop {// 寮€濮嬩簨鍔℃椂鑷姩娉ㄥ唽

    // 执行事务...let txn1 = store.begin();

    tx_count += 1;let txn2 = store.begin_read_only();

    

    if tx_count % 100 == 0 {// 鏌ヨ娲昏穬浜嬪姟姘翠綅绾?

        store.gc()?;let min_ts = store.get_min_active_ts();

    }println!("鏈€灏忔椿璺?ts: {:?}", min_ts);

}

```// 浜嬪姟缁撴潫鏃惰嚜鍔ㄦ敞閿€锛圖rop trait锛?

drop(txn1);

**2. 基于版本数触发**:drop(txn2);

```rust

// 版本数超过阈值时触发// 鐜板湪娌℃湁娲昏穬浜嬪姟

if store.total_versions() > 10000 {assert_eq!(store.get_min_active_ts(), None);

    println!("版本数过多，触发 GC");```

    let cleaned = store.gc()?;

    println!("清理了 {} 个版本", cleaned);#### GC 鏈€浣冲疄璺?

}

```**1. 瀹氭湡瑙﹀彂 GC**:

```rust

**3. 监控 GC 效果**:// 绠€鍗曠瓥鐣ワ細姣?N 涓簨鍔¤Е鍙戜竴娆?

```rustlet mut tx_count = 0;

let before_versions = store.total_versions();loop {

let cleaned = store.gc()?;    // 鎵ц浜嬪姟...

let after_versions = store.total_versions();    tx_count += 1;

    

println!("GC 前: {} 版本", before_versions);    if tx_count % 100 == 0 {

println!("清理: {} 版本", cleaned);        store.gc()?;

println!("GC 后: {} 版本", after_versions);    }

println!("压缩率: {:.2}%", }

    cleaned as f64 / before_versions as f64 * 100.0);```

```

**2. 鍩轰簬鐗堟湰鏁拌Е鍙?*:

**4. 避免在事务中触发 GC**:```rust

```rust// 鐗堟湰鏁拌秴杩囬槇鍊兼椂瑙﹀彂

// ❌ 不好 - 可能清理当前事务需要的版本if store.total_versions() > 10000 {

let txn = store.begin();    println!("鐗堟湰鏁拌繃澶氾紝瑙﹀彂 GC");

store.gc()?; // 危险！    let cleaned = store.gc()?;

txn.read(b"key");    println!("娓呯悊浜?{} 涓増鏈?, cleaned);

}

// ✅ 好 - 在事务之间触发```

drop(txn);

store.gc()?;**3. 鐩戞帶 GC 鏁堟灉**:

let txn2 = store.begin();```rust

```let before_versions = store.total_versions();

let cleaned = store.gc()?;

#### GC 性能影响let after_versions = store.total_versions();



运行 GC 基准测试:println!("GC 鍓? {} 鐗堟湰", before_versions);

```bashprintln!("娓呯悊: {} 鐗堟湰", cleaned);

cargo bench --bench parallel_benchmark -- mvcc_gcprintln!("GC 鍚? {} 鐗堟湰", after_versions);

```println!("鍘嬬缉鐜? {:.2}%", 

    cleaned as f64 / before_versions as f64 * 100.0);

**典型性能特征**:```

- **GC 吞吐量**: 每次 GC 可清理数千到数万个版本（毫秒级）

- **读取影响**: GC 使用写锁，不阻塞读操作（并发读取不受影响）**4. 閬垮厤鍦ㄤ簨鍔′腑瑙﹀彂 GC**:

- **写入影响**: GC 期间新写入需要等待（但 GC 通常很快）```rust

- **活跃事务影响**: 活跃事务越多，可清理的版本越少// 鉂?涓嶅ソ - 鍙兘娓呯悊褰撳墠浜嬪姟闇€瑕佺殑鐗堟湰

let txn = store.begin();

---store.gc()?; // 鍗遍櫓锛?

txn.read(b"key");

## MVCC 自动垃圾回收 (v0.7.0 🎉)

// 鉁?濂?- 鍦ㄤ簨鍔′箣闂磋Е鍙?

### 功能概述drop(txn);

store.gc()?;

自动 GC 在后台线程周期性清理过期版本，无需手动调用 `gc()` 方法。let txn2 = store.begin();

```

### 配置

#### GC 鎬ц兘褰卞搷

```rust

use vm_runtime::{MvccStore, GcConfig, AutoGcConfig};杩愯 GC 鍩哄噯娴嬭瘯:

use std::sync::Arc;```bash

cargo bench --bench parallel_benchmark -- mvcc_gc

let config = GcConfig {```

    max_versions_per_key: 10,

    enable_time_based_gc: false,**鍏稿瀷鎬ц兘鐗瑰緛**:

    version_ttl_secs: 3600,- **GC 鍚炲悙閲?*: 姣忔 GC 鍙竻鐞嗘暟鍗冨埌鏁颁竾涓増鏈紙姣绾э級

    auto_gc: Some(AutoGcConfig {- **璇诲彇褰卞搷**: GC 浣跨敤鍐欓攣锛屼笉闃诲璇绘搷浣滐紙骞跺彂璇诲彇涓嶅彈褰卞搷锛?

        interval_secs: 60,            // 每 60 秒执行一次- **鍐欏叆褰卞搷**: GC 鏈熼棿鏂板啓鍏ラ渶瑕佺瓑寰咃紙浣?GC 閫氬父寰堝揩锛?

        version_threshold: 1000,      // 版本数 ≥ 1000 时触发- **娲昏穬浜嬪姟褰卞搷**: 娲昏穬浜嬪姟瓒婂锛屽彲娓呯悊鐨勭増鏈秺灏?

        run_on_start: false,          // 启动时不立即执行

    }),## MVCC 鑷姩鍨冨溇鍥炴敹 (v0.7.0 馃帀)

};

### 鍔熻兘姒傝堪

let store = Arc::new(MvccStore::new_with_config(config));

// 自动 GC 后台线程已自动启动鑷姩 GC 鍦ㄥ悗鍙扮嚎绋嬪懆鏈熸€ф竻鐞嗚繃鏈熺増鏈紝鏃犻渶鎵嬪姩璋冪敤 `gc()` 鏂规硶銆?

```

### 閰嶇疆

### 触发策略

```rust

1. **周期性触发**: 每隔 `interval_secs` 秒执行一次 GCuse vm_runtime::{MvccStore, GcConfig, AutoGcConfig};

2. **阈值触发**: 当 `total_versions() >= version_threshold` 时立即执行use std::sync::Arc;

   - 设置 `version_threshold = 0` 禁用阈值检查，仅使用周期触发

3. **启动触发**: `run_on_start = true` 时立即执行一次let config = GcConfig {

    max_versions_per_key: 10,

### 手动控制    enable_time_based_gc: false,

    version_ttl_secs: 3600,

```rust    auto_gc: Some(AutoGcConfig {

// 停止自动 GC        interval_secs: 60,            // 姣?60 绉掓墽琛屼竴娆?

store.stop_auto_gc();        version_threshold: 1000,      // 鐗堟湰鏁?鈮?1000 鏃惰Е鍙?

        run_on_start: false,          // 鍚姩鏃朵笉绔嬪嵆鎵ц

// 重新启动    }),

store.start_auto_gc();};



// 检查运行状态let store = Arc::new(MvccStore::new_with_config(config));

if store.is_auto_gc_running() {// 鑷姩 GC 鍚庡彴绾跨▼宸茶嚜鍔ㄥ惎鍔?

    println!("自动 GC 正在运行");```

}

### 瑙﹀彂绛栫暐

// 动态更新配置

store.update_auto_gc_config(Some(AutoGcConfig {1. **鍛ㄦ湡鎬цЕ鍙?*: 姣忛殧 `interval_secs` 绉掓墽琛屼竴娆?GC

    interval_secs: 30,       // 改为 30 秒2. **闃堝€艰Е鍙?*: 褰?`total_versions() >= version_threshold` 鏃剁珛鍗虫墽琛?

    version_threshold: 500,  // 降低阈值   - 璁剧疆 `version_threshold = 0` 绂佺敤闃堝€兼鏌ワ紝浠呬娇鐢ㄥ懆鏈熻Е鍙?

    run_on_start: false,3. **鍚姩瑙﹀彂**: `run_on_start = true` 鏃剁珛鍗虫墽琛屼竴娆?

}));

### 鎵嬪姩鎺у埗

// 禁用自动 GC

store.update_auto_gc_config(None);```rust

```// 鍋滄鑷姩 GC

store.stop_auto_gc();

### 线程安全

// 閲嶆柊鍚姩

- 自动 GC 与事务并发执行是安全的（读写锁保护）store.start_auto_gc();

- GC 线程使用可中断休眠(100ms 粒度)，快速响应停止信号

- Drop 时自动停止 GC 线程并等待退出(最多 2 秒)// 妫€鏌ヨ繍琛岀姸鎬?

if store.is_auto_gc_running() {

### 性能影响    println!("鑷姩 GC 姝ｅ湪杩愯");

}

运行基准测试对比自动 GC 的开销:

```bash// 鍔ㄦ€佹洿鏂伴厤缃?

cargo bench --bench parallel_benchmark -- auto_gcstore.update_auto_gc_config(Some(AutoGcConfig {

```    interval_secs: 30,       // 鏀逛负 30 绉?

    version_threshold: 500,  // 闄嶄綆闃堝€?

**典型结果**:    run_on_start: false,

- **写入开销**: < 5%（自动 GC 在后台运行，不阻塞写操作）}));

- **读取开销**: 无明显影响（GC 使用写锁，不阻塞并发读）

- **CPU 占用**: 休眠时几乎为 0，GC 执行时短暂峰值// 绂佺敤鑷姩 GC

store.update_auto_gc_config(None);

### 最佳实践```



1. **生产环境**: 启用自动 GC，设置合理的 `interval_secs` (30-60s)### 绾跨▼瀹夊叏

2. **高写入场景**: 降低 `version_threshold` (500-1000) 以更频繁清理

3. **低负载场景**: 提高 `interval_secs` (120-300s) 以减少开销- 鑷姩 GC 涓庝簨鍔″苟鍙戞墽琛屾槸瀹夊叏鐨勶紙璇诲啓閿佷繚鎶わ級

4. **测试环境**: 可禁用自动 GC (`auto_gc: None`) 便于调试- GC 绾跨▼浣跨敤鍙腑鏂紤鐪?(100ms 绮掑害)锛屽揩閫熷搷搴斿仠姝俊鍙?

- Drop 鏃惰嚜鍔ㄥ仠姝?GC 绾跨▼骞剁瓑寰呴€€鍑?(鏈€澶?2 绉?

---

### 鎬ц兘褰卞搷

## 路线图

杩愯鍩哄噯娴嬭瘯瀵规瘮鑷姩 GC 鐨勫紑閿€:

### 短期 (v0.8.0)```bash

- [ ] MVCC 压力测试与调优cargo bench --bench parallel_benchmark -- auto_gc

- [ ] 交易优先级调度策略强化```

- [ ] 自动 GC 自适应策略（根据负载动态调整间隔）

**鍏稿瀷缁撴灉**:

### 中期 (v0.9.0)- **鍐欏叆寮€閿€**: < 5%锛堣嚜鍔?GC 鍦ㄥ悗鍙拌繍琛岋紝涓嶉樆濉炲啓鎿嶄綔锛?

- [ ] 乐观并发控制（OCC）集成- **璇诲彇寮€閿€**: 鏃犳槑鏄惧奖鍝嶏紙GC 浣跨敤鍐欓攣锛屼笉闃诲骞跺彂璇伙級

- [ ] 跨分片/分区的并行调度探索- **CPU 鍗犵敤**: 浼戠湢鏃跺嚑涔庝负 0锛孏C 鎵ц鏃剁煭鏆傚嘲鍊?

- [ ] MVCC 与 Snapshot 自动选择策略

### 鏈€浣冲疄璺?

### 长期 (v1.0.0)

- [ ] 分布式并行执行1. **鐢熶骇鐜**: 鍚敤鑷姩 GC锛岃缃悎鐞嗙殑 `interval_secs` (30-60s)

- [ ] GPU 加速冲突检测2. **楂樺啓鍏ュ満鏅?*: 闄嶄綆 `version_threshold` (500-1000) 浠ユ洿棰戠箒娓呯悊

- [ ] 机器学习优化调度3. **浣庤礋杞藉満鏅?*: 鎻愰珮 `interval_secs` (120-300s) 浠ュ噺灏戝紑閿€

4. **娴嬭瘯鐜**: 鍙鐢ㄨ嚜鍔?GC (`auto_gc: None`) 渚夸簬璋冭瘯

---

### 鐭湡 (v0.8.0)

## 参考资料- [ ] MVCC 鍘嬪姏娴嬭瘯涓庤皟浼?

- [ ] 浜ゆ槗浼樺厛绾ц皟搴︾瓥鐣ュ己鍖?

- [Solana Sealevel 并行执行](https://medium.com/solana-labs/sealevel-parallel-processing-thousands-of-smart-contracts-d814b378192)- [ ] 鑷姩 GC 鑷€傚簲绛栫暐锛堟牴鎹礋杞藉姩鎬佽皟鏁撮棿闅旓級

- [Aptos Block-STM](https://medium.com/aptoslabs/block-stm-how-we-execute-over-160k-transactions-per-second-on-the-aptos-blockchain-3b003657e4ba)

- [Sui 并行执行模型](https://docs.sui.io/learn/sui-execution)### 涓湡 (v0.9.0)

- [PostgreSQL MVCC](https://www.postgresql.org/docs/current/mvcc.html)- [ ] 涔愯骞跺彂鎺у埗锛圤CC锛夐泦鎴?

- [CockroachDB Transaction Layer](https://www.cockroachlabs.com/docs/stable/architecture/transaction-layer.html)- [ ] 璺ㄥ垎鐗?鍒嗗尯鐨勫苟琛岃皟搴︽帰绱?

- [ ] MVCC 涓?Snapshot 鑷姩閫夋嫨绛栫暐

---

### 闀挎湡 (v1.0.0)

## 更新历史- [ ] 鍒嗗竷寮忓苟琛屾墽琛?

- [ ] GPU 鍔犻€熷啿绐佹娴?

- **v0.7.0 (2025-11-04)**: 添加 MVCC 自动 GC（后台线程）- [ ] 鏈哄櫒瀛︿範浼樺寲璋冨害

- **v0.6.0 (2025-11-04)**: 添加 MVCC 垃圾回收

- **v0.5.0 (2025-11-04)**: MVCC 核心实现 + 只读优化 + 调度器集成---

- **v0.4.0 (2025-11-04)**: 批量操作优化

- **v0.3.0 (2025-11-03)**: 工作窃取调度器## 鍙傝€冭祫鏂?

- **v0.2.0 (2025-11-03)**: 执行统计 + 自动重试

- **v0.1.0 (2025-11-02)**: 并行执行引擎初版- [Solana Sealevel 骞惰鎵ц](https://medium.com/solana-labs/sealevel-parallel-processing-thousands-of-smart-contracts-d814b378192)

- [Aptos Block-STM](https://medium.com/aptoslabs/block-stm-how-we-execute-over-160k-transactions-per-second-on-the-aptos-blockchain-3b003657e4ba)

---- [Sui 骞惰鎵ц妯″瀷](https://docs.sui.io/learn/sui-execution)

- [PostgreSQL MVCC](https://www.postgresql.org/docs/current/mvcc.html)

*最后更新: 2025-11-04*- [CockroachDB Transaction Layer](https://www.cockroachlabs.com/docs/stable/architecture/transaction-layer.html)


---

## 鏇存柊鍘嗗彶

- **v0.7.0 (2025-11-04)**: 娣诲姞 MVCC 鑷姩 GC锛堝悗鍙扮嚎绋嬶級
- **v0.6.0 (2025-11-04)**: 娣诲姞 MVCC 鍨冨溇鍥炴敹
- **v0.5.0 (2025-11-04)**: MVCC 鏍稿績瀹炵幇 + 鍙浼樺寲 + 璋冨害鍣ㄩ泦鎴?
- **v0.4.0 (2025-11-04)**: 鎵归噺鎿嶄綔浼樺寲
- **v0.3.0 (2025-11-03)**: 宸ヤ綔绐冨彇璋冨害鍣?
- **v0.2.0 (2025-11-03)**: 鎵ц缁熻 + 鑷姩閲嶈瘯
- **v0.1.0 (2025-11-02)**: 骞惰鎵ц寮曟搸鍒濈増

---

*鏈€鍚庢洿鏂? 2025-11-04*




