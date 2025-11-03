# Changelog

All notable changes to SuperVM will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added - vm-runtime v0.2.0 (2025-11-03)

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
