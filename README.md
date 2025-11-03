# SuperVM - WASM Runtime with Event System

开发者: 
Rainbow Haruko(CHINA) / king(CHINA) / NoahX(CHINA)
/ Alan Tang(CHINA) / Xuxu(CHINA)

SuperVM 是一个高性能的 WASM-first 虚拟机运行时,支持存储操作、链上下文访问和事件系统。

## 功能特性

### ✨ vm-runtime

- **WASM 执行引擎**: 基于 wasmtime 17.0 的高性能 WASM 运行时
- **存储抽象层**: 可插拔的存储后端(trait-based 设计)
- **Host Functions**: 
  - 📦 Storage API: get/set/delete/scan 操作
  - ⛓️ Chain Context API: block_number, timestamp
  - 📣 Event System: emit_event, events_len, read_event
  - 🔐 Crypto API: SHA-256, Keccak-256, ECDSA, Ed25519, 地址派生
- **并行执行引擎**:
  - 🚀 并行交易调度器 (ParallelScheduler)
  - ⚡ 工作窃取调度器 (WorkStealingScheduler)
  - 📦 批量操作优化 (batch_write/read/delete/execute)
  - 🔐 MVCC 多版本并发控制 (MvccStore) - NEW
  - 🔍 冲突检测与依赖分析 (ConflictDetector)
  - 📊 执行统计 (ExecutionStats)
  - 🔄 自动重试机制 (execute_with_retry)
  - 💾 状态快照与回滚 (StateManager)
- **execute_with_context API**: 执行 WASM 函数并返回结果、事件和上下文

### 🚀 node-core

- **CLI 工具**: 带 `--once` 标志支持自动化测试
- **演示程序**: 
  - Demo 1: 简单的 add 函数
  - Demo 2: 完整的事件系统展示(存储 + 事件 + 链上下文)
  - Demo 3: 密码学功能演示 (SHA-256, Keccak-256)
  - Demo 4: 以太坊地址派生
  - Demo 5: 并行执行与冲突检测
  - Demo 6: 状态快照与回滚
  - Demo 7: 工作窃取调度器
  - Demo 8: 批量操作优化
  - Demo 9: MVCC 多版本并发控制 (NEW �)

## 快速开始

### 环境要求

- Rust toolchain (stable) - [安装 rustup](https://rustup.rs/)
- 操作系统: Windows / Linux / macOS

### 运行演示

```powershell
# 运行完整演示(包含事件系统)
cargo run -p node-core

# 运行一次后退出(适合 CI/自动化测试)
cargo run -p node-core -- --once
```

**预期输出:**
```
INFO node_core: Starting node (PoC) with config: config.toml
INFO node_core: Demo 1: add(7,8) => 15
INFO node_core: Demo 2: execute_with_context results:
INFO node_core:   Function returned: 1704079545
INFO node_core:   Block number: 12345, Timestamp: 1704067200
INFO node_core:   Events collected: 2 events
INFO node_core:     Event 1: UserAction
INFO node_core:     Event 2: BlockProcessed
```

### 运行测试

```powershell
# 运行所有测试
cargo test -p vm-runtime

# 运行特定测试
cargo test -p vm-runtime test_execute_with_context
```

**测试覆盖 (59/59 通过):**

**核心功能:**
- ✅ test_memory_storage - 存储实现测试
- ✅ test_execute_add_via_wat - 基础 WASM 执行
- ✅ test_storage - 存储 API 测试
- ✅ test_host_functions - Host 函数调用
- ✅ test_emit_event - 事件发送与读取
- ✅ test_execute_with_context - 完整上下文执行

**密码学功能:**
- ✅ test_sha256 - SHA-256 哈希
- ✅ test_keccak256 - Keccak-256 哈希
- ✅ test_ed25519_verify - Ed25519 签名验证
- ✅ test_secp256k1_verify - ECDSA 签名验证
- ✅ test_derive_eth_address - 以太坊地址派生

**并行执行引擎:**
- ✅ test_read_write_set_conflicts - 读写集冲突检测
- ✅ test_dependency_graph - 依赖图构建
- ✅ test_conflict_detector - 冲突检测器
- ✅ test_snapshot_creation - 快照创建
- ✅ test_rollback - 状态回滚
- ✅ test_nested_snapshots - 嵌套快照
- ✅ test_commit - 快照提交
- ✅ test_execution_stats - 执行统计
- ✅ test_retry_mechanism - 自动重试
- ✅ test_scheduler_with_snapshot - 调度器集成
- ✅ test_work_stealing_basic - 工作窃取基础
- ✅ test_work_stealing_with_priorities - 优先级调度
- ✅ test_work_stealing_with_errors - 错误处理
- ✅ test_batch_write - 批量写入
- ✅ test_batch_read - 批量读取
- ✅ test_batch_delete - 批量删除
- ✅ test_batch_emit_events - 批量事件
- ✅ test_execute_batch - 批量执行
- ✅ test_execute_batch_rollback - 批量回滚

**MVCC 多版本并发控制:**
- ✅ test_mvcc_write_write_conflict - 写写冲突检测
- ✅ test_mvcc_snapshot_isolation_visibility - 快照隔离可见性
- ✅ test_mvcc_version_visibility_multiple_versions - 多版本可见性
- ✅ test_mvcc_concurrent_reads - 并发读取测试
- ✅ test_mvcc_concurrent_writes_different_keys - 不同键并发写
- ✅ test_mvcc_concurrent_writes_same_key_conflicts - 同键冲突检测
- ✅ test_mvcc_read_only_transaction - 只读事务快速路径
- ✅ test_mvcc_read_only_cannot_write - 只读事务写入保护
- ✅ test_mvcc_read_only_cannot_delete - 只读事务删除保护
- ✅ test_mvcc_read_only_performance - 只读性能对比

**MVCC 调度器集成:**
- ✅ test_scheduler_mvcc_basic_commit - MVCC调度器基础提交
- ✅ test_scheduler_mvcc_abort_on_error - MVCC调度器错误回滚
- ✅ test_scheduler_mvcc_read_only_fast_path - MVCC调度器只读路径

**MVCC 垃圾回收:**
- ✅ test_gc_version_cleanup - 版本清理正确性
- ✅ test_gc_preserves_active_transaction_visibility - 保护活跃事务可见性
- ✅ test_gc_no_active_transactions - 无活跃事务时的清理
- ✅ test_gc_multiple_keys - 多键 GC
- ✅ test_gc_stats_accumulation - GC 统计累计

**基准测试:**
```powershell
# 运行性能基准测试
cargo bench --bench parallel_benchmark
```

### 性能摘要 (Criterion)

- 并行调度 get_parallel_batch/100: 平均约 350,045 ns/批
- 冲突检测 non_conflicting/100: 平均约 396,673 ns
- 冲突检测 50% 冲突/100: 平均约 460,675 ns
- 快照创建 create_snapshot/1000: 平均约 224,712 ns
- 依赖图 build_and_query/100: 平均约 344,862 ns

说明:
- 单位为 ns/iter（Criterion 默认），不同机器的绝对值会有差异，请以相对对比为主。
- 完整 HTML 报告路径: target/criterion/report/index.html

## 使用示例

### 基础 WASM 执行

```rust
use vm_runtime::{Runtime, MemoryStorage};

let runtime = Runtime::new(MemoryStorage::new());
let wat = r#"
(module
  (func $add (export "add") (param i32 i32) (result i32)
    local.get 0
    local.get 1
    i32.add)
)
"#;
let wasm = wat::parse_str(wat)?;
let result = runtime.execute_add(&wasm, 7, 8)?;
assert_eq!(result, 15);
```

### 并行执行与状态管理

```rust
use vm_runtime::{ParallelScheduler, ExecutionStats};

// 创建并行调度器
let scheduler = ParallelScheduler::new();

// 使用快照保护执行交易
let result = scheduler.execute_with_snapshot(|manager| {
    let storage = manager.get_storage();
    let mut storage = storage.lock().unwrap();
    storage.insert(b"balance".to_vec(), b"100".to_vec());
    Ok(()) // 成功则提交
})?;

// 使用自动重试机制
let result = scheduler.execute_with_retry(
    |manager| {
        // 可能失败的操作
        Ok(42)
    },
    max_retries: 3
)?;

// 获取执行统计
let stats = scheduler.get_stats();
println!("成功率: {:.2}%", stats.success_rate() * 100.0);
println!("重试次数: {}", stats.retry_count);
```

### 工作窃取调度器

```rust
use vm_runtime::{WorkStealingScheduler, Task};

// 创建工作窃取调度器 (4 个工作线程)
let scheduler = WorkStealingScheduler::new(Some(4));

// 提交任务 (支持优先级)
let tasks = vec![
    Task::new(1, 255),  // 高优先级
    Task::new(2, 128),  // 中优先级
    Task::new(3, 50),   // 低优先级
];
scheduler.submit_tasks(tasks);

// 并行执行所有任务
let result = scheduler.execute_all(|tx_id| {
    println!("Processing transaction {}", tx_id);
    Ok(())
})?;

// 获取统计信息
let stats = scheduler.get_stats();
println!("成功: {}, 失败: {}", stats.successful_txs, stats.failed_txs);
```

### 批量操作

```rust
use vm_runtime::ParallelScheduler;

let scheduler = ParallelScheduler::new();

// 批量写入 (减少锁争用)
let writes = vec![
    (b"key1".to_vec(), b"value1".to_vec()),
    (b"key2".to_vec(), b"value2".to_vec()),
    (b"key3".to_vec(), b"value3".to_vec()),
];
scheduler.batch_write(writes)?;

// 批量读取
let keys = vec![b"key1".to_vec(), b"key2".to_vec()];
let results = scheduler.batch_read(&keys)?;

// 批量执行交易 (原子性: 全部成功或全部回滚)
let operations = vec![
    Box::new(|manager| { /* 交易 1 */ Ok(1) }),
    Box::new(|manager| { /* 交易 2 */ Ok(2) }),
    Box::new(|manager| { /* 交易 3 */ Ok(3) }),
];
let results = scheduler.execute_batch(operations)?;
```

### MVCC 多版本并发控制

```rust
use vm_runtime::MvccStore;

let store = MvccStore::new();

// 事务 1：写入并提交
let mut t1 = store.begin();
t1.write(b"balance".to_vec(), b"100".to_vec());
let ts1 = t1.commit()?;

// 事务 2：快照隔离读取
let t2 = store.begin();
assert_eq!(t2.read(b"balance").as_deref(), Some(b"100".as_ref()));

// 并发更新同一键会触发写写冲突检测
let mut t3 = store.begin();
let mut t4 = store.begin();
t3.write(b"balance".to_vec(), b"200".to_vec());
t4.write(b"balance".to_vec(), b"300".to_vec());

// 第一个提交成功
t3.commit()?;
// 第二个提交失败（写写冲突）
assert!(t4.commit().is_err());
```

**优化特性**:
- ✅ 每键粒度读写锁 (RwLock)，允许并发读取
- ✅ DashMap 无锁哈希表，降低全局锁竞争
- ✅ 原子时间戳 (AtomicU64)，消除时间戳分配瓶颈
- ✅ 提交时按键排序加锁，避免死锁
- ✅ 快照隔离 (Snapshot Isolation) 语义
- ✅ 垃圾回收 (GC)：自动清理旧版本，控制内存增长

**垃圾回收 (v0.6.0 NEW)**:
```rust
use vm_runtime::{MvccStore, GcConfig};

// 创建带 GC 配置的 MVCC 存储
let config = GcConfig {
    max_versions_per_key: 10,      // 每个键最多保留 10 个版本
    enable_time_based_gc: false,   // 暂不启用基于时间的 GC
    version_ttl_secs: 3600,        // 版本过期时间（秒）
};
let store = MvccStore::new_with_config(config);

// ... 执行一些事务 ...

// 手动触发 GC
let cleaned = store.gc()?;
println!("清理了 {} 个旧版本", cleaned);

// 获取 GC 统计
let stats = store.get_gc_stats();
println!("GC 执行次数: {}", stats.gc_count);
println!("总清理版本数: {}", stats.versions_cleaned);

// 监控存储状态
println!("当前总版本数: {}", store.total_versions());
println!("当前键数量: {}", store.total_keys());
```

**GC 策略**:
- 保留每个键的最新版本（无论配置如何）
- 保留所有活跃事务可见的版本（基于水位线）
- 根据 `max_versions_per_key` 限制清理超量版本
- 自动跟踪活跃事务，防止清理仍在使用的版本

### 使用事件系统

```rust
use vm_runtime::{Runtime, MemoryStorage};

let runtime = Runtime::new(MemoryStorage::new());
let wat = r#"
(module
  (import "chain_api" "emit_event" (func $emit_event (param i32 i32) (result i32)))
  (memory (export "memory") 1)
  (data (i32.const 0) "Hello, World!")
  
  (func (export "greet") (result i32)
    i32.const 0
    i32.const 13
    call $emit_event
    drop
    i32.const 42
  )
)
"#;
let wasm = wat::parse_str(wat)?;
let (result, events, block_num, timestamp) = runtime.execute_with_context(
    &wasm,
    "greet",
    12345,  // block_number
    1704067200  // timestamp
)?;

assert_eq!(result, 42);
assert_eq!(events.len(), 1);
assert_eq!(events[0], b"Hello, World!");
```

### 自定义存储后端

```rust
use vm_runtime::Storage;
use anyhow::Result;

struct MyStorage {
    // your implementation
}

impl Storage for MyStorage {
    fn get(&self, key: &[u8]) -> Result<Option<Vec<u8>>> {
        // your logic
    }
    
    fn set(&mut self, key: &[u8], value: &[u8]) -> Result<()> {
        // your logic
    }
    
    fn delete(&mut self, key: &[u8]) -> Result<()> {
        // your logic
    }
    
    fn scan(&self, prefix: &[u8], limit: usize) -> Result<Vec<(Vec<u8>, Vec<u8>)>> {
        // your logic
    }
}

let runtime = Runtime::new(MyStorage::new());
```

## Host Functions 参考

### Storage API (`storage_api`)

| 函数 | 签名 | 说明 |
|------|------|------|
| `storage_get` | `(key_ptr: i32, key_len: i32) -> i64` | 读取键值,返回长度(缓存到 last_get) |
| `storage_read_value` | `(ptr: i32, len: i32) -> i32` | 从缓存读取值到内存 |
| `storage_set` | `(key_ptr: i32, key_len: i32, value_ptr: i32, value_len: i32) -> i32` | 写入键值对 |
| `storage_delete` | `(key_ptr: i32, key_len: i32) -> i32` | 删除键 |

### Chain API (`chain_api`)

| 函数 | 签名 | 说明 |
|------|------|------|
| `block_number` | `() -> i64` | 获取当前区块号 |
| `timestamp` | `() -> i64` | 获取当前时间戳 |
| `emit_event` | `(data_ptr: i32, data_len: i32) -> i32` | 发送事件 |
| `events_len` | `() -> i32` | 获取事件总数 |
| `read_event` | `(index: i32, ptr: i32, len: i32) -> i32` | 读取指定事件 |

## 项目结构

```
SuperVM/
├── src/
│   ├── vm-runtime/          # WASM 运行时核心
│   │   ├── src/
│   │   │   ├── lib.rs       # 公共 API
│   │   │   ├── storage.rs   # 存储抽象
│   │   │   └── host.rs      # Host functions
│   │   └── Cargo.toml
│   └── node-core/           # CLI 演示程序
│       ├── src/
│       │   └── main.rs
│       └── Cargo.toml
├── docs/
│   └── plans/
│       └── vm-runtime-extension.md
├── CHANGELOG.md             # 更新日志
├── ROADMAP.md               # 开发路线图
└── Cargo.toml               # Workspace 配置
```

## 架构设计

```
┌─────────────────────────────────────────────┐
│             node-core (CLI)                 │
│  ┌──────────────────────────────────────┐   │
│  │  Demo 1: Basic execution             │   │
│  │  Demo 2: Events + Storage + Context  │   │
│  └────────────┬─────────────────────────┘   │
└───────────────┼─────────────────────────────┘
                │
                ▼
┌───────────────────────────────────────────────┐
│           vm-runtime Crate                    │
│  ┌─────────────────────────────────────────┐  │
│  │  Runtime<S: Storage>                    │  │
│  │  - execute_add()                        │  │
│  │  - execute_with_context()               │  │
│  └──────────┬──────────────────────────────┘  │
│             │                                  │
│  ┌──────────▼──────────┐  ┌────────────────┐ │
│  │   Storage Trait     │  │  Host Functions│ │
│  │  - get/set/delete   │  │  - storage_api │ │
│  │  - scan             │  │  - chain_api   │ │
│  └─────────────────────┘  └────────────────┘ │
│             │                      │           │
│  ┌──────────▼──────────┐          │           │
│  │  MemoryStorage      │          │           │
│  │  (BTreeMap backend) │          │           │
│  └─────────────────────┘          │           │
└────────────────────────────────────┼───────────┘
                                     │
                                     ▼
                          ┌──────────────────┐
                          │   wasmtime 17.0  │
                          │   WASM Engine    │
                          └──────────────────┘
```

## 性能特性

- ⚡ **Zero-copy**: 使用指针传递避免不必要的内存复制
- 🔒 **安全性**: Rust 内存安全 + WASM 沙箱隔离
- 🚀 **高性能**: wasmtime JIT 编译优化
- 📦 **模块化**: 可插拔存储后端,易于扩展

提示: 想快速了解本项目的性能表现？请直接查看下方的“[性能摘要 (Criterion)](#性能摘要-criterion)”小节，或打开本地基准报告 HTML：`target/criterion/report/index.html`。

## 开发状态

当前版本: **v0.5.0** (活跃开发)

**已完成 ✅:**
- ✅ 基础 WASM 执行引擎
- ✅ 存储抽象与实现
- ✅ Host Functions (存储 + 链上下文 + 事件 + 密码学)
- ✅ execute_with_context API
- ✅ 并行执行引擎
    - ✅ 冲突检测与依赖分析
    - ✅ 状态快照与回滚
    - ✅ 执行统计与监控
    - ✅ 自动重试机制
    - ✅ 工作窃取调度器
    - ✅ 批量操作优化（batch_write/read/delete/execute）
    - ✅ MVCC 多版本并发控制（每键粒度读写锁 + DashMap）
- ✅ 完整单元测试覆盖 (47 个测试)
- ✅ 性能基准测试框架（Criterion）

**进行中 🚧:**
- 🚧 性能基准测试报告总结与文档化
- 🚧 MVCC 与 ParallelScheduler 集成

**计划中 📋:**
- 📋 编译器集成 (Solidity/AssemblyScript)
- 📋 EVM 兼容层
- 📋 乐观并发控制（OCC）
- 📋 生产环境部署

详见 [CHANGELOG.md](CHANGELOG.md) 和 [ROADMAP.md](ROADMAP.md)。

## 贡献指南

欢迎贡献!请参阅 [CONTRIBUTING.md](CONTRIBUTING.md)。

## 许可证

MIT OR Apache-2.0

## 联系方式

- 开发者: Rainbow Haruko / king
- Email: iscrbank@gmail.com / leadbrand@me.com
- 问题反馈: [GitHub Issues](https://github.com/XujueKing/SuperVM/issues)
