# Cross-Chain Executor 使用指南（unstable-examples）

本页介绍如何在 `vm-runtime` 中使用跨链执行能力（`CrossChainExecutor`）。当前实现为占位/演示版本，便于验证 API 与执行路径；完整的余额/nonce/gas 及跨链证明校验会在后续阶段完善。

## 前置条件

- Windows PowerShell 环境（示例命令基于 PowerShell）
- 已安装 Rust stable toolchain

## 快速运行

仅运行库测试（避开 examples 构建依赖）：

```powershell
$env:RUSTFLAGS='-C debuginfo=0'; cargo test -p vm-runtime --features unstable-examples --lib -- --test-threads=1
```

只运行 cross_executor 冒烟测试：

```powershell
$env:RUSTFLAGS='-C debuginfo=0'; cargo test -p vm-runtime --features unstable-examples --lib cross_executor_smoke -- --test-threads=1
```

## 示例一：跨链转账（零金额冒烟）

```rust
use vm_runtime::adapter::{AdapterRegistry, ChainConfig, WasmChainAdapter, TxIR, CrossChainExecutor};

let registry = AdapterRegistry::new();

// 注册两条 SuperVM 链（链1、链2）
let adapter1 = WasmChainAdapter::new(ChainConfig::supervm(1))?;
let adapter2 = WasmChainAdapter::new(ChainConfig::supervm(2))?;
registry.register(Box::new(adapter1))?;
registry.register(Box::new(adapter2))?;

let executor = CrossChainExecutor::new(registry);

// 构造零金额跨链转账，避免余额校验阻塞冒烟
let mut tx = TxIR::cross_chain_transfer(vec![0x11; 20], vec![0x22; 20], 0, 0, 1, 2);
tx.compute_hash();

let receipt = executor.execute_cross_chain_transfer(&tx)?;
assert_eq!(receipt.locked_amount, 0);
assert_eq!(receipt.proof.len(), 32);
```

## 示例二：跨链合约调用（最小示例）

```rust
use vm_runtime::adapter::{AdapterRegistry, ChainConfig, WasmChainAdapter, TxIR, TxType, StateIR, CrossChainExecutor};

let registry = AdapterRegistry::new();
let adapter1 = WasmChainAdapter::new(ChainConfig::supervm(10))?;
let adapter2 = WasmChainAdapter::new(ChainConfig::supervm(20))?;
registry.register(Box::new(adapter1))?;
registry.register(Box::new(adapter2))?;

let executor = CrossChainExecutor::new(registry);

let tx = TxIR {
    hash: vec![],
    from: vec![0x33; 20],
    to: Some(vec![0x44; 20]),
    value: 0,
    gas_limit: 0, // 冒烟：设置为 0，避免 gas 相关校验
    gas_price: 1,
    nonce: 0,
    data: vec![],
    signature: vec![],
    chain_id: 10,
    tx_type: TxType::CrossChainCall,
    source_chain: Some(10),
    target_chain: Some(20),
};

let mut state = StateIR::default();
executor.execute_cross_chain_call(&tx, &mut state)?; // 接口返回 Result<()>
```

## 备注

- 当前 `ChainAdapter::execute_transaction` 基于 `&mut StateIR` 执行；`CrossChainExecutor` 会在调用前确保 `from` 账户存在于 `state`（无余额占位）。
- 未来将引入余额/nonce/gas 完整校验与跨链证明（Merkle 或 ZK）验证逻辑，并与存储层（MVCC/RocksDB）对接。
