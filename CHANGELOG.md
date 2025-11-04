# Changelog

All notable changes to SuperVM will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added - vm-runtime v0.8.0 (2025-11-04)

#### MVCC Stress Testing & Adaptive GC 🔬🤖
- **压力测试套件**:
  - `test_high_concurrency_mixed_workload`: 高并发混合读写（8线程，8000交易，70%读/30%写）
  - `test_high_contention_hotspot`: 高冲突热点键测试（16线程，5个热点键，验证极端冲突场景）
  - `test_memory_growth_control`: 内存增长监控（50键，20迭代，验证 GC 效果）
  - `test_long_running_stability`: 长时间稳定性测试（60秒+，可配置数小时）
  - `test_adaptive_gc`: 自适应 GC 行为验证
- **压力测试统计信息**:
  - `StressTestStats`: 详细的性能报告（TPS、延迟、冲突率、内存使用）
  - 实时监控：TPS、版本数、GC 频率
  - P99 延迟分析
- **自适应 GC 策略** 🎯:
  - **AdaptiveGcStrategy**: 可配置的自适应策略
    - `base_interval_secs`: 基准 GC 间隔（默认 60秒）
    - `min_interval_secs`: 最小间隔（高负载，默认 10秒）
    - `max_interval_secs`: 最大间隔（低负载，默认 300秒）
    - `base_threshold`: 基准版本阈值（默认 1000）
    - `min_threshold`: 最小阈值（更激进，默认 500）
    - `max_threshold`: 最大阈值（更宽松，默认 5000）
  - **自适应调整逻辑**:
    - **高负载检测**: TPS 激增或版本快速增长 → 缩短间隔、降低阈值
    - **低效 GC 检测**: 清理率 < 10% → 延长间隔、提高阈值
    - **正常负载**: 逐渐回归基准值
  - **AutoGcConfig** 新增字段:
    - `enable_adaptive: bool` - 启用/禁用自适应 GC（默认 false）
- **内部优化**:
  - `MvccStore` 新增字段：
    - `adaptive_strategy`: 自适应策略配置
    - `recent_tx_count`: 事务计数器（用于计算 TPS）
    - `recent_gc_cleaned`: 最近 GC 清理数（用于评估效果）
  - 事务提交时自动更新计数器
  - GC 线程根据负载动态调整参数

#### Documentation 📖
- 新增 `docs/stress-testing-guide.md`: 完整的压力测试与调优指南
  - 测试套件使用说明
  - 各测试场景详解
  - 自适应 GC 配置指南
  - 性能调优建议（4 种典型场景）
  - 故障排查手册（4 个常见问题）
- 更新 `README.md`: 添加压力测试使用示例
- 更新 `CHANGELOG.md`: v0.8.0 特性说明

#### API Changes 🔧
- **Breaking**: `AutoGcConfig` 新增 `enable_adaptive: bool` 字段
  - 向后兼容：现有代码添加 `enable_adaptive: false` 即可
- **New**: `AdaptiveGcStrategy` 结构体
- **New**: `StressTestStats` 结构体（测试专用）
- **Export**: `AdaptiveGcStrategy` 导出到公共 API

---

### Added - vm-runtime v0.7.0 (2025-11-04)

#### MVCC Automatic Garbage Collection 🤖🗑️
- **AutoGcConfig**: 自动 GC 配置
  - `interval_secs`: GC 执行间隔（秒，默认 60）
  - `version_threshold`: 触发阈值（版本数，默认 1000，0 表示仅周期触发）
  - `run_on_start`: 启动时立即执行（默认 false）
- **自动 GC 功能**:
  - `start_auto_gc()`: 启动后台 GC 线程（自动启动，无需手动调用）
  - `stop_auto_gc()`: 停止后台 GC 线程
  - `is_auto_gc_running()`: 检查 GC 线程运行状态
  - `update_auto_gc_config()`: 动态更新自动 GC 配置
- **后台线程特性**:
  - 可中断休眠 (100ms 粒度)，快速响应停止信号
  - 双重触发策略：周期性 + 阈值触发
  - Drop 时自动停止并等待线程退出 (最多 2 秒)
  - 原子标志控制，线程安全
- **触发策略**:
  - **周期性**: 每隔 `interval_secs` 秒执行一次
  - **阈值触发**: 当 `total_versions() >= version_threshold` 时立即执行
  - **启动触发**: `run_on_start = true` 时启动时立即执行

#### Testing 🧪
- 新增 5 个自动 GC 测试:
  - `test_auto_gc_periodic`: 周期性自动清理
  - `test_auto_gc_threshold`: 阈值触发自动清理
  - `test_auto_gc_run_on_start`: 启动时立即清理
  - `test_auto_gc_start_stop`: 启动/停止控制
  - `test_auto_gc_concurrent_safety`: 并发安全性
- 总测试数: **64/64 通过** ✅ (+5 from v0.6.0)

#### Benchmarks 📊
- 新增 `auto_gc_impact` 基准组:
  - `write_without_auto_gc` vs `write_with_auto_gc`: 写入性能对比
  - `read_without_auto_gc` vs `read_with_auto_gc`: 读取性能对比
- 性能影响: 写入开销 < 5%，读取无明显影响

#### API Changes 🔧
- **Breaking**: `GcConfig` 新增 `auto_gc: Option<AutoGcConfig>` 字段
  - 向后兼容：现有代码添加 `auto_gc: None` 即可
- **New**: `AutoGcConfig` 结构体
- **New**: `MvccStore::start_auto_gc()` - 启动自动 GC
- **New**: `MvccStore::stop_auto_gc()` - 停止自动 GC
- **New**: `MvccStore::is_auto_gc_running()` - 检查运行状态
- **New**: `MvccStore::update_auto_gc_config()` - 动态更新配置
- **New**: `impl Drop for MvccStore` - 自动清理资源

#### Documentation 📖
- 更新 `README.md`: 添加自动 GC 使用示例
- 更新 `docs/parallel-execution.md`: 添加"MVCC 自动垃圾回收"章节
- 测试计数更新: 59 → 64

---

### Added - vm-runtime v0.6.0 (2025-11-04)

#### MVCC Garbage Collection 🗑️
- **GcConfig**: 可配置的垃圾回收策略
  - `max_versions_per_key`: 每个键最多保留的版本数（默认 10）
  - `enable_time_based_gc`: 是否启用基于时间的 GC（默认 false）
  - `version_ttl_secs`: 版本过期时间（秒）
- **MvccStore GC 功能**:
  - `gc()`: 手动触发垃圾回收，清理不再需要的旧版本
  - `get_gc_stats()`: 获取 GC 统计信息（执行次数、清理版本数、清理键数）
  - `get_min_active_ts()`: 获取活跃事务的最小时间戳（水位线）
  - `set_gc_config()`: 动态更新 GC 配置
  - `total_versions()`: 获取当前总版本数（监控用）
  - `total_keys()`: 获取当前键数量（监控用）
- **活跃事务跟踪**:
  - 自动注册和注销活跃事务（通过 begin/drop）
  - GC 保护活跃事务可见的所有版本
  - 基于水位线的智能清理策略
- **GC 清理策略**:
  - 保留每个键的最新版本（无条件）
  - 保留所有活跃事务可见的版本（基于 min_active_ts）
  - 根据 max_versions_per_key 限制清理超量版本
  - 避免清理仍在使用的版本，确保正确性

#### Testing 🧪
- 新增 5 个 GC 测试:
  - `test_gc_version_cleanup`: 版本清理正确性
  - `test_gc_preserves_active_transaction_visibility`: 保护活跃事务可见性
  - `test_gc_no_active_transactions`: 无活跃事务时的清理
  - `test_gc_multiple_keys`: 多键 GC
  - `test_gc_stats_accumulation`: GC 统计累计
- 总测试数: **59/59 通过** ✅

#### Benchmarks 📊
- 新增 `mvcc_gc` 基准组:
  - `gc_throughput`: 不同版本数下的 GC 吞吐量
  - `read_with_gc`: GC 对读取性能的影响
  - `write_with_gc`: GC 对写入性能的影响
  - `gc_with_active_transactions`: 活跃事务对 GC 的影响

#### API Changes 🔧
- `MvccStore::new_with_config(config: GcConfig)`: 创建带 GC 配置的存储
- 导出新类型: `GcConfig`, `GcStats`
- `Txn` 自动在 Drop 时注销活跃事务

#### Performance 🚀
- **内存控制**: 通过定期 GC 控制内存增长
- **智能清理**: 仅清理不再需要的版本，不影响活跃事务
- **低开销**: GC 使用写锁，不阻塞读操作

## [0.5.0] - 2025-11-04

### Added - vm-runtime v0.5.0

#### MVCC Multi-Version Concurrency Control 🔐
- **MvccStore**: 多版本并发控制存储实现
  - 快照隔离 (Snapshot Isolation) 语义
  - 每个键维护版本链,按时间戳升序存储
  - 原子时间戳分配 (AtomicU64),消除瓶颈
  - **细粒度并发控制**:
    - DashMap 无锁哈希表,减少全局锁争用
    - 每键 RwLock 读写锁,允许并发读取
    - 提交时按键排序加锁,避免死锁
    - 仅锁定写集合涉及的键,最小化锁持有范围
- **Txn**: 事务接口
  - `begin()`: 开启读写事务,分配快照版本 (start_ts)
  - `begin_read_only()`: 开启只读事务 (快速路径)
  - `read()`: 读取 start_ts 及之前的可见版本
  - `write()` / `delete()`: 本地缓存写操作 (只读事务会 panic)
  - `commit()`: 提交事务,进行写写冲突检测 (只读无需检测,直接返回 start_ts)
  - `abort()`: 放弃事务
- **只读事务优化** ⚡:
  - `begin_read_only()` 标记事务为只读
  - 提交时跳过冲突检测和锁获取
  - 无写集合,直接返回快照时间戳
  - 显著降低只读查询开销
- **冲突检测**:
  - 提交时检测写写冲突 (Write-Write Conflict)
  - 若发现 ts > start_ts 的已提交版本则拒绝提交
  - 保证可串行化 (Serializability)

#### Scheduler Integration with MVCC 🔗
- **ParallelScheduler MVCC 支持**:
  - `new_with_mvcc(store: Arc<MvccStore>)`: 创建 MVCC 后端调度器
  - `execute_with_mvcc<F>(&self, operation: F)`: 执行读写事务
    - 自动开启事务、执行操作、提交或回滚
    - 更新统计信息 (successful/failed/rollback)
  - `execute_with_mvcc_read_only<F>(&self, operation: F)`: 执行只读事务
    - 使用快速路径,无冲突检测开销
    - 适用于查询密集型场景
  - 非破坏性集成: 保留原有 snapshot 机制,可选使用 MVCC

#### Testing 🧪
- 新增 10 个 MVCC 核心测试:
  - `test_mvcc_write_write_conflict`: 写写冲突检测
  - `test_mvcc_snapshot_isolation_visibility`: 快照隔离可见性
  - `test_mvcc_version_visibility_multiple_versions`: 多版本可见性
  - `test_mvcc_concurrent_reads`: 并发读取性能
  - `test_mvcc_concurrent_writes_different_keys`: 不同键并发写
  - `test_mvcc_concurrent_writes_same_key_conflicts`: 同键冲突检测
  - `test_mvcc_read_only_transaction`: 只读事务快速路径
  - `test_mvcc_read_only_cannot_write`: 只读事务写入保护
  - `test_mvcc_read_only_cannot_delete`: 只读事务删除保护
  - `test_mvcc_read_only_performance`: 只读性能对比
- 新增 3 个 MVCC 调度器集成测试:
  - `test_scheduler_mvcc_basic_commit`: MVCC调度器基础提交
  - `test_scheduler_mvcc_abort_on_error`: MVCC调度器错误回滚
  - `test_scheduler_mvcc_read_only_fast_path`: MVCC调度器只读路径
- 总测试数: **54/54 通过** ✅ (v0.5.0 基础)

#### Dependencies 📦
- 新增 `dashmap ^6.1`: 高性能并发哈希表
- 新增 `parking_lot ^0.12`: 更快的 RwLock 实现

#### Performance 🚀
- **并发读取**: 多事务可同时读取不同键 (无锁竞争)
- **并发写入**: 不同键的写入可并发执行
- **时间戳分配**: 原子操作,避免锁开销
- **锁粒度**: 从全局锁优化为每键锁,大幅降低争用

## [0.4.0] - 2025-11-04

### Added - vm-runtime v0.4.0

#### Batch Operations Optimization 📦
- **StateManager 批量操作**:
  - `batch_write()`: 批量写入,减少锁争用
  - `batch_read()`: 批量读取,一次性获取多个键
  - `batch_delete()`: 批量删除
  - `batch_emit_events()`: 批量发送事件
  - **性能提升**: 相比单个操作,批量写入可提升数倍性能
- **ParallelScheduler 批量执行**:
  - `execute_batch()`: 批量执行交易,共享一个快照
  - 原子性保证: 批次中任何交易失败,整个批次回滚
  - `batch_write()` / `batch_read()` / `batch_delete()`: 直接批量操作接口
  - 减少快照创建/提交开销
  
#### Testing 🧪
- 新增 6 个批量操作测试:
  - `test_batch_write`: 批量写入
  - `test_batch_read`: 批量读取
  - `test_batch_delete`: 批量删除
  - `test_batch_emit_events`: 批量事件
  - `test_execute_batch`: 批量执行成功
  - `test_execute_batch_rollback`: 批量失败回滚
- 总测试数: **41/41 通过** ✅

#### Documentation 📚
- 更新文档说明批量操作 API

#### Examples 💡
- **Demo 8**: 批量操作演示 (`demo8_batch_operations.rs`)
  - 批量写入性能对比 (1000 条记录)
  - 批量读取示例
  - 批量执行交易
  - 批量失败自动回滚

## [0.3.0] - 2025-11-03

### Added - vm-runtime v0.3.0

#### Work-Stealing Scheduler ⚡
- **WorkStealingScheduler**: 工作窃取调度器
  - 基于 crossbeam-deque 和 rayon 的高性能任务调度
  - 自动负载均衡: 空闲线程从忙碌线程窃取任务
  - `submit_task()` / `submit_tasks()`: 提交任务到全局队列
  - `execute_all()`: 并行执行所有任务
  - 支持任务优先级 (0-255)
  - 集成 ParallelScheduler 进行状态管理
- **Task**: 任务定义
  - `tx_id`: 交易标识符
  - `priority`: 任务优先级
- **性能提升**:
  - 减少线程空闲时间
  - 提高 CPU 利用率
  - 支持大规模任务处理 (测试 1000+ 任务)

#### Testing 🧪
- 新增 3 个工作窃取测试:
  - `test_work_stealing_basic`: 基础工作窃取
  - `test_work_stealing_with_priorities`: 优先级调度
  - `test_work_stealing_with_errors`: 错误处理
- 总测试数: **35/35 通过** ✅

#### Documentation 📚
- 更新 `docs/parallel-execution.md`:
  - 添加 WorkStealingScheduler 详细说明
  - 工作窃取算法原理
  - API 使用示例
  - 性能优化建议

#### Examples 💡
- **Demo 7**: 工作窃取调度器演示 (`demo7_work_stealing.rs`)
  - 基础工作窃取
  - 优先级调度
  - 大规模任务处理 (1000 任务)
  - 与 ParallelScheduler 集成

## [0.2.0] - 2025-11-03

### Added - vm-runtime v0.2.0

#### Parallel Execution Engine 🚀
- **ParallelScheduler**: 并行交易调度器
  - `execute_with_snapshot()`: 快照保护的事务执行
  - `execute_with_retry()`: 带自动重试的事务执行
  - `get_stats()`: 获取执行统计信息
- **ConflictDetector**: 冲突检测器
  - `record()`: 记录交易读写集
  - `has_conflict()`: 检测两个交易是否冲突
  - `build_dependency_graph()`: 构建依赖关系图
- **DependencyGraph**: 依赖图管理
  - `add_dependency()`: 添加依赖关系
  - `get_ready_transactions()`: 获取可并行执行的交易
- **StateManager**: 状态管理器
  - `create_snapshot()`: 创建状态快照
  - `rollback()`: 回滚到快照状态
  - `commit()`: 提交并丢弃快照
  - 支持嵌套快照
- **ExecutionStats**: 执行统计
  - 成功/失败交易计数
  - 回滚/重试次数统计
  - 冲突检测计数
  - 成功率/回滚率计算

#### Crypto API (`crypto_api` module)
- `sha256(data_ptr, data_len, output_ptr) -> i32`: SHA-256 哈希
- `keccak256(data_ptr, data_len, output_ptr) -> i32`: Keccak-256 哈希
- `ed25519_verify(msg_ptr, msg_len, sig_ptr, pubkey_ptr) -> i32`: Ed25519 签名验证
- `secp256k1_verify(msg_ptr, msg_len, sig_ptr, pubkey_ptr) -> i32`: ECDSA 签名验证
- `derive_eth_address(pubkey_ptr, pubkey_len, output_ptr) -> i32`: 以太坊地址派生

#### Performance Benchmarks
- 添加 criterion 基准测试框架
- 4 组基准测试:
  - 冲突检测性能 (10/50/100/500 交易)
  - 快照操作性能 (10/100/1000 数据项)
  - 依赖图构建性能
  - 并行调度性能

#### Testing
- ✅ 32/32 单元测试通过
  - 11 个并行执行测试
  - 5 个密码学测试
  - 5 个状态快照测试
  - 3 个调度器集成测试
  - 8 个核心功能测试

### Added - node-core v0.2.0 (2025-11-03)

#### Demo Programs
- **Demo 3**: 密码学功能演示
  - SHA-256 和 Keccak-256 哈希计算
  - 哈希验证
- **Demo 4**: 以太坊地址派生
  - 从公钥派生以太坊地址
- **Demo 5**: 并行执行演示
  - 3 笔交易的冲突检测
  - 依赖关系分析
  - 并行调度展示
- **Demo 6**: 状态快照与回滚 ✨
  - 场景 1: 成功的交易提交
  - 场景 2: 失败的交易自动回滚
  - 场景 3: 嵌套交易部分回滚

---

## [0.1.0] - 2025-11-02

### Added - vm-runtime v0.1.0

#### Core Runtime
- **WASM Execution Engine**: Integrated wasmtime 17.0 for WebAssembly execution
- **Storage Abstraction**: `Storage` trait with `MemoryStorage` implementation
- **Host Functions Architecture**: Modular host function registration system

#### Storage API (`storage_api` module)
- `storage_get(key_ptr, key_len) -> i64`: Get value by key, cache to `last_get`
- `storage_read_value(ptr, len) -> i32`: Read cached value from last get
- `storage_set(key_ptr, key_len, value_ptr, value_len) -> i32`: Write key-value pair
- `storage_delete(key_ptr, key_len) -> i32`: Delete key from storage

#### Chain Context API (`chain_api` module)
- `block_number() -> i64`: Get current block number
- `timestamp() -> i64`: Get current block timestamp
- `emit_event(data_ptr, data_len) -> i32`: Emit an event to host
- `events_len() -> i32`: Get total number of emitted events
- `read_event(index, ptr, len) -> i32`: Read event data by index

#### Public APIs
- `Runtime::new(storage: S)`: Create runtime with custom storage backend
- `Runtime::execute_add(&self, module_bytes, a, b) -> Result<i32>`: Execute add function (demo)
- `Runtime::execute_with_context(&self, module_bytes, func_name, block_number, timestamp) -> Result<(i32, Vec<Vec<u8>>, u64, u64)>`: Execute function with block context and return events

#### Testing
- ✅ 6/6 unit tests passing:
  - `test_memory_storage`: Storage trait implementation
  - `test_execute_add_via_wat`: Basic WASM execution
  - `test_storage`: Storage operations via runtime
  - `test_host_functions`: Host function calls from WASM
  - `test_emit_event`: Event emission and reading
  - `test_execute_with_context`: Full context execution with events

### Added - node-core v0.1.0

#### CLI Features
- `--once` flag: Run once and exit without waiting for Ctrl-C (for automated testing)
- **Demo 1**: Simple add(7,8) demonstration
- **Demo 2**: Full event system showcase
  - Emits "UserAction" and "BlockProcessed" events
  - Uses storage API to write key-value pairs
  - Demonstrates block context (block_number, timestamp) access
  - Pretty-prints collected events to console

#### Logging
- Integrated tracing + tracing_subscriber for structured logging
- INFO-level output for demo results

### Changed

#### Project Structure
- Workspace resolver set to "2" (eliminates Cargo warnings)
- .gitignore updated with UTF-8 comments
- /solana/ directory excluded from version control (local reference only)

### Technical Details

#### Memory Management
- Host functions use `Rc<RefCell<Storage>>` for shared mutable state
- Memory handle cloning pattern to avoid borrow checker conflicts
- Safe memory access via `read_memory` and `write_memory` helpers

#### Module Naming
- Host functions registered under proper namespaces:
  - `storage_api::*` for storage operations
  - `chain_api::*` for blockchain context and events
- WAT imports must match these module names exactly

#### Performance Considerations
- Storage operations use BTreeMap (O(log n) complexity)
- Event collection uses Vec (append-only, no reallocation concerns for typical use)
- Memory operations validated with bounds checking

## [0.0.0] - 2025-01-XX (Initial PoC)

### Added
- Initial repository structure
- Basic Cargo workspace setup
- wasmtime integration proof-of-concept
- Simple WAT example execution

---

## Development Timeline

- **Week 1**: PoC - Basic WASM runtime with wasmtime
- **Week 2**: Storage abstraction and host function architecture
- **Week 3**: Chain context, event system, and execute_with_context API
- **Next**: Compiler adapter for Solidity/AssemblyScript

## Contributors

- king <king@example.com> - Initial development

## Notes

### Breaking Changes
None yet (pre-1.0.0)

### Migration Guide
N/A (first release)

### Known Issues
- Push to remote repository blocked by network issues (large history)
- solana/ directory remains in local filesystem (gitignored)

### Upcoming Features (Roadmap)
See [ROADMAP.md](ROADMAP.md) for planned features:
- Solidity compiler integration (Solang)
- AssemblyScript support
- Parallel execution engine
- EVM compatibility layer
