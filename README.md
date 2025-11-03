# SuperVM - WASM Runtime with Event System

开发者: king

SuperVM 是一个高性能的 WASM-first 虚拟机运行时,支持存储操作、链上下文访问和事件系统。

## 功能特性

### ✨ vm-runtime

- **WASM 执行引擎**: 基于 wasmtime 17.0 的高性能 WASM 运行时
- **存储抽象层**: 可插拔的存储后端(trait-based 设计)
- **Host Functions**: 
  - 📦 Storage API: get/set/delete/scan 操作
  - ⛓️ Chain Context API: block_number, timestamp
  - 📣 Event System: emit_event, events_len, read_event
- **execute_with_context API**: 执行 WASM 函数并返回结果、事件和上下文

### 🚀 node-core

- **CLI 工具**: 带 `--once` 标志支持自动化测试
- **演示程序**: 
  - Demo 1: 简单的 add 函数
  - Demo 2: 完整的事件系统展示(存储 + 事件 + 链上下文)

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

**测试覆盖:**
- ✅ test_memory_storage - 存储实现测试
- ✅ test_execute_add_via_wat - 基础 WASM 执行
- ✅ test_storage - 存储 API 测试
- ✅ test_host_functions - Host 函数调用
- ✅ test_emit_event - 事件发送与读取
- ✅ test_execute_with_context - 完整上下文执行

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

## 开发状态

当前版本: **v0.1.0** (PoC 阶段)

- ✅ 基础 WASM 执行
- ✅ 存储抽象与实现
- ✅ Host functions (存储 + 链上下文 + 事件)
- ✅ execute_with_context API
- ✅ 完整单元测试覆盖
- 🚧 编译器集成 (Solidity/AssemblyScript)
- 📋 并行执行引擎
- 📋 EVM 兼容层

详见 [CHANGELOG.md](CHANGELOG.md) 和 [ROADMAP.md](ROADMAP.md)。

## 贡献指南

欢迎贡献!请参阅 [CONTRIBUTING.md](CONTRIBUTING.md)。

## 许可证

MIT OR Apache-2.0

## 联系方式

- 开发者: king
- Email: king@example.com
- 问题反馈: [GitHub Issues](https://github.com/XujueKing/SuperVM/issues)
