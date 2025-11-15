# EVM 适配器架构设计

开发者/作者：King Xujue

> **设计原则**: 保持 SuperVM 核心纯净,EVM 兼容层作为可插拔模块

## 🎯 设计目标

1. **零侵入**: 不修改 `vm-runtime` 核心执行逻辑
2. **可插拔**: EVM 功能通过 feature flag 控制,可随时移除
3. **性能隔离**: WASM 执行路径不受 EVM 影响
4. **清晰边界**: 通过 trait 抽象实现多引擎支持

## 🏗️ 架构设计

### 1. 模块划分

```

SuperVM/
├── src/
│   ├── vm-runtime/              # 核心运行时 (纯净)
│   │   ├── execution_trait.rs   # ✅ 已实现: L1 统一执行引擎 trait
│   │   ├── wasm_executor.rs     # WASM 执行器 (实现 trait)
│   │   ├── parallel_mvcc.rs     # 并行调度
│   │   └── storage.rs           # 存储抽象
│   │
│   ├── evm-adapter/             # EVM 适配器 (独立 crate)
│   │   ├── Cargo.toml           # 独立依赖管理
│   │   ├── src/
│   │   │   ├── lib.rs           # 适配器入口
│   │   │   ├── evm_executor.rs  # EVM 执行器 (实现 trait)
│   │   │   ├── revm_backend.rs  # revm 封装
│   │   │   ├── precompiles.rs   # EVM 预编译合约
│   │   │   └── gas_mapping.rs   # Gas 计量映射
│   │   └── tests/               # 独立测试
│   │
│   └── node-core/               # 节点核心
│       ├── engine_selector.rs   # 引擎选择器
│       └── config.rs            # 配置管理
│
└── Cargo.toml                   # Workspace 配置

```

### 2. 接口设计

#### 2.1 统一执行引擎接口

```rust
// vm-runtime/src/execution_trait.rs

use anyhow::Result;

/// 执行引擎类型
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EngineType {
    Wasm,
    Evm,
}

/// 执行上下文
pub struct ExecutionContext {
    pub caller: [u8; 20],
    pub contract: [u8; 20],
    pub value: u128,
    pub gas_limit: u64,
    pub block_number: u64,
    pub timestamp: u64,
}

/// 执行结果
pub struct ExecutionResult {
    pub success: bool,
    pub return_data: Vec<u8>,
    pub gas_used: u64,
    pub logs: Vec<Log>,
    pub state_changes: Vec<StateChange>,
}

/// 统一执行引擎 trait
pub trait ExecutionEngine: Send + Sync {
    /// 执行合约代码
    fn execute(
        &self,
        code: &[u8],
        input: &[u8],
        context: &ExecutionContext,
    ) -> Result<ExecutionResult>;

    /// 获取引擎类型
    fn engine_type(&self) -> EngineType;

    /// 验证代码格式
    fn validate_code(&self, code: &[u8]) -> Result<()>;
}

```

#### 2.2 WASM 执行器实现 (最小修改)

```rust
// vm-runtime/src/wasm_executor.rs

use crate::execution_trait::*;

pub struct WasmExecutor {
    runtime: Runtime,  // 现有的 Runtime
}

impl WasmExecutor {
    // 现有方法保持不变
    pub fn new() -> Self { ... }
    
    // 现有执行逻辑
    pub fn execute_wasm(&self, ...) -> Result<...> {
        // 保持原有实现不变
    }
}

// 新增: 实现 ExecutionEngine trait (仅添加,不修改)
impl ExecutionEngine for WasmExecutor {
    fn execute(&self, code: &[u8], input: &[u8], ctx: &ExecutionContext) 
        -> Result<ExecutionResult> 
    {
        // 适配层: 调用现有的 execute_wasm 方法
        let result = self.execute_wasm(code, input)?;
        
        // 转换为统一的 ExecutionResult
        Ok(ExecutionResult {
            success: true,
            return_data: result.data,
            gas_used: result.gas,
            logs: result.events.into_iter().map(|e| e.into()).collect(),
            state_changes: vec![],
        })
    }

    fn engine_type(&self) -> EngineType {
        EngineType::Wasm
    }

    fn validate_code(&self, code: &[u8]) -> Result<()> {
        wasmtime::Module::validate(&self.runtime.engine(), code)?;
        Ok(())
    }
}

```

#### 2.3 EVM 适配器实现 (完全独立)

```rust
// evm-adapter/src/evm_executor.rs

use vm_runtime::execution_trait::*;
use revm::{Database, EVM};

pub struct EvmExecutor {
    evm: EVM<EvmDatabase>,
}

impl EvmExecutor {
    pub fn new() -> Self {
        Self {
            evm: EVM::new(),
        }
    }
}

impl ExecutionEngine for EvmExecutor {
    fn execute(&self, code: &[u8], input: &[u8], ctx: &ExecutionContext) 
        -> Result<ExecutionResult> 
    {
        // EVM 执行逻辑 (完全隔离)
        let result = self.evm.transact(code, input, ctx)?;
        
        Ok(ExecutionResult {
            success: !result.is_error(),
            return_data: result.output,
            gas_used: result.gas_used,
            logs: result.logs,
            state_changes: result.state_changes,
        })
    }

    fn engine_type(&self) -> EngineType {
        EngineType::Evm
    }

    fn validate_code(&self, code: &[u8]) -> Result<()> {
        // 验证 EVM 字节码
        evm::validate_bytecode(code)?;
        Ok(())
    }
}

```

#### 2.4 引擎选择器

```rust
// node-core/src/engine_selector.rs

use vm_runtime::execution_trait::*;
use vm_runtime::WasmExecutor;

#[cfg(feature = "evm-compat")]
use evm_adapter::EvmExecutor;

pub struct EngineSelector {
    wasm_engine: WasmExecutor,
    #[cfg(feature = "evm-compat")]
    evm_engine: EvmExecutor,
}

impl EngineSelector {
    pub fn new() -> Self {
        Self {
            wasm_engine: WasmExecutor::new(),
            #[cfg(feature = "evm-compat")]
            evm_engine: EvmExecutor::new(),
        }
    }

    /// 根据代码类型选择引擎
    pub fn select(&self, code: &[u8]) -> Result<&dyn ExecutionEngine> {
        // 通过魔数判断代码类型
        if code.starts_with(b"\0asm") {
            return Ok(&self.wasm_engine);
        }
        
        #[cfg(feature = "evm-compat")]
        if is_evm_bytecode(code) {
            return Ok(&self.evm_engine);
        }
        
        Err(anyhow!("Unsupported code format"))
    }

    /// 执行合约
    pub fn execute(
        &self,
        code: &[u8],
        input: &[u8],
        context: &ExecutionContext,
    ) -> Result<ExecutionResult> {
        let engine = self.select(code)?;
        engine.execute(code, input, context)
    }
}

fn is_evm_bytecode(code: &[u8]) -> bool {
    // 简单启发式判断: EVM 字节码通常以 PUSH/DUP 等操作码开始
    !code.is_empty() && code[0] >= 0x60 && code[0] <= 0x7f
}

```

### 3. 依赖管理

#### 3.1 Workspace Cargo.toml

```toml
[workspace]
members = [
    "src/vm-runtime",
    "src/evm-adapter",
    "src/node-core",
]

[workspace.dependencies]
anyhow = "1.0"
thiserror = "1.0"

```

#### 3.2 vm-runtime/Cargo.toml (核心保持纯净)

```toml
[package]
name = "vm-runtime"
version = "0.10.0"

[dependencies]

# 核心依赖 (不变)

wasmtime = "17.0"
anyhow.workspace = true

# ... 其他现有依赖

# 注意: 没有 revm 依赖!

```

#### 3.3 evm-adapter/Cargo.toml (独立依赖)

```toml
[package]
name = "evm-adapter"
version = "0.1.0"

[dependencies]
vm-runtime = { path = "../vm-runtime" }  # 仅依赖 trait 定义
revm = { version = "3.5", default-features = false }
anyhow.workspace = true

```

#### 3.4 node-core/Cargo.toml (可选集成)

```toml
[package]
name = "node-core"
version = "0.10.0"

[features]
default = []
evm-compat = ["evm-adapter"]  # 可选 EVM 功能

[dependencies]
vm-runtime = { path = "../vm-runtime" }
evm-adapter = { path = "../evm-adapter", optional = true }

```

### 4. Feature Flag 控制

#### 4.1 编译选项

```bash

# 纯净内核 (无 EVM,推荐用于生产)

cargo build --release

# 完整功能 (含 EVM 兼容)

cargo build --release --features evm-compat

# 仅测试核心功能

cargo test -p vm-runtime

# 测试 EVM 适配器

cargo test -p evm-adapter

```

#### 4.2 运行时配置

```toml

# config.toml

[execution]

# 启用的引擎

enabled_engines = ["wasm"]  # 默认仅 WASM

# 如果需要 EVM 兼容

# enabled_engines = ["wasm", "evm"]

[evm]

# EVM 相关配置 (仅在启用时有效)

chain_id = 1
london_enabled = true

```

## 🔒 核心纯净性保证

### 1. 代码侵入度分析

| 模块 | 修改类型 | 侵入程度 |
|------|---------|---------|
| `wasm_executor.rs` | ✅ 添加 trait impl | 极低 (10行代码) |
| `parallel_mvcc.rs` | ❌ 无修改 | 零 |
| `storage.rs` | ❌ 无修改 | 零 |
| `其他核心模块` | ❌ 无修改 | 零 |

### 2. 依赖树验证

```bash

# 检查 vm-runtime 依赖树 (应该不包含 revm)

cargo tree -p vm-runtime --no-default-features

# 检查 evm-adapter 依赖树 (应该包含 revm)

cargo tree -p evm-adapter

```

### 3. 性能基准测试

```rust
// benches/execution_overhead.rs

#[bench]
fn bench_wasm_execution_pure(b: &mut Bencher) {
    // 纯 WASM 执行 (无 EVM feature)
    let executor = WasmExecutor::new();
    b.iter(|| executor.execute_wasm(...));
}

#[bench]
fn bench_wasm_execution_with_trait(b: &mut Bencher) {
    // 通过 trait 执行 (验证零开销抽象)
    let executor = WasmExecutor::new();
    b.iter(|| executor.execute(...));  // trait 方法
}

// 预期结果: 两者性能应相同 (编译器内联优化)

```

### 4. 编译产物大小对比

```bash

# 纯净版本

cargo build --release
ls -lh target/release/node-core  # 记录大小

# EVM 版本

cargo clean
cargo build --release --features evm-compat
ls -lh target/release/node-core  # 对比大小

# 预期: EVM 版本仅增加 ~2-3MB (revm 库大小)

```

## � 子模块化升级：EVM Adapter → Geth 子模块（MVP 定稿）

为对齐“热插拔子模块 = 原链节点”的总体路线，本文件在保持现有适配器设计不变的前提下，新增首选实现路径：优先以“Geth 子模块”对接真实以太坊节点能力，原基于 revm 的适配器作为纯兼容/测试路径保留。

### 子模块接口（SubmoduleAdapter）最小契约

```rust
pub trait SubmoduleAdapter {
    fn start(&self) -> anyhow::Result<()>;                 // 启动/连接原链
    fn stop(&self) -> anyhow::Result<()>;                  // 平滑停止
    fn process_native_transaction(&self, tx: NativeTx) -> anyhow::Result<TxHash>; // 提交原生交易
    fn execute_smart_contract(&self, tx: NativeTx) -> anyhow::Result<Receipt>;     // 合约执行（账户链）
    fn query_native_state(&self, q: StateQuery) -> anyhow::Result<StateResult>;    // 原生状态查询
    fn sync_to_unified_mirror(&self, mirror: &mut UnifiedStateMirror) -> anyhow::Result<()>; // 写入统一镜像
}

```

### Geth 子模块（优先）

- 集成方式：Engine API（首选）或 FFI 桥接

- 能力范围：区块/交易同步、EVM 执行、账户与 ERC20 事件监听

- 与统一层衔接：将 Receipt/Logs 转为 TxIR/StateIR，写入镜像层

### 与原“EVM 适配器（revm）”的关系

- 保留：作为无外部进程依赖的轻量兼容路径

- 优先级：Geth 子模块 > revm 适配器

- 选择逻辑：运行时由配置/探测决定（优先启用子模块）

### MVP 范围（Phase 10 M1）

- 定义 SubmoduleAdapter 契约

- 实现 Geth 子模块最小骨架（同步 + 执行 + 事件→IR 写镜像）

- ERC20 Indexer v0（Transfer 事件 → IR）

- 与 go-ethereum 节点互联验证

---

## �🚀 实施路线图

### Phase 1: 接口定义 ✅ **已完成** (2025-11-05)

- [x] 创建 `execution_trait.rs` ✅

- [x] 定义 `ExecutionEngine` trait ✅

- [x] 定义 `ExecutionContext`, `ContractResult` 等数据结构 ✅

- [x] 编写单元测试 `test_execution_trait` ✅

- [x] 集成到 `lib.rs` 并导出公共 API ✅

**完成详情**:

- 文件: `src/vm-runtime/src/execution_trait.rs` (76 行)

- 层级: L1 扩展层 (连接 L0 核心与 L2 适配器)

- 测试: ✅ 通过

- 编译: ✅ 通过

### Phase 2: EVM 适配器开发 (Week 2-3)

- [ ] 创建 `evm-adapter` crate

- [ ] 集成 revm

- [ ] 实现 `ExecutionEngine` trait

- [ ] Gas 映射实现

- [ ] Precompiles 支持

### Phase 3: 引擎选择器 (Week 4)

- [ ] 实现 `EngineSelector`

- [ ] 代码类型检测逻辑

- [ ] Feature flag 配置

- [ ] 端到端测试

### Phase 4: 测试与优化 (Week 5-6)

- [ ] 以太坊测试套件

- [ ] 性能基准测试

- [ ] 文档完善

- [ ] 发布 v0.11.0

## 📊 成功标准

1. ✅ `vm-runtime` 核心代码修改少于 50 行
2. ✅ 不启用 `evm-compat` feature 时,无 revm 依赖
3. ✅ WASM 执行性能与之前版本一致 (误差 < 1%)
4. ✅ 编译时可完全移除 EVM 代码
5. ✅ EVM 适配器可独立测试和发布

## 🔧 维护策略

### 独立开发

- EVM 适配器由专门团队维护

- 核心团队专注于 WASM 性能优化

- 两个模块独立发版

### 升级隔离

- SuperVM 核心升级不影响 EVM 适配器

- EVM 适配器升级 (如 revm 新版本) 不影响核心

- 通过 trait 接口保持兼容性

### 未来扩展

- 同样的模式可用于其他 VM (如 Move VM)

- 保持核心的纯净性和高性能

- 通过插件生态支持多种执行环境

---

**总结**: 通过插件化架构,SuperVM 可以在保持核心纯净高效的同时,支持 EVM 兼容性。这是一个**可选、可插拔、零污染**的解决方案。
