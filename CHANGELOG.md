# Changelog

All notable changes to SuperVM will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

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
