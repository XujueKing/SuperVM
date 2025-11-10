# Phase D: 多链统一插件系统 - ChainAdapter 架构实现

**目标**: 实现多链统一架构的 ChainAdapter 插件系统（EVM/WASM/BTC/Solana 等作为可插拔模块）

**开发者**: king  
**开始时间**: 2025-11-10  
**预计周期**: 4-6 周

---

## 🎯 设计理念重构

### ⚠️ 架构调整说明

**原计划**: Phase D 定位为"EVM 适配器研究"，将 revm 深度集成到 SuperVM 核心。

**新架构**: 根据 **MULTICHAIN-ARCHITECTURE-VISION.md（多链统一架构愿景）**，调整为：

```
SuperVM 核心 (L0/L1 - 纯净内核)
    ↓ 统一抽象层
ChainAdapter 插件层 (L2 - 可插拔)
    ├── EVM Adapter (Ethereum/BSC/Polygon)
    ├── WASM Adapter (原生高性能执行)
    ├── BTC Adapter (UTXO 模型适配)
    ├── Solana Adapter (账户模型适配)
    └── ... (可扩展其他链)
```

**核心原则**:
- ✅ **零侵入**: EVM 不进入 `vm-runtime` 核心
- ✅ **插件化**: 所有链适配器平等对待，可热插拔
- ✅ **统一抽象**: 通过 `ChainAdapter` trait 归一化所有链
- ✅ **性能隔离**: FastPath/Consensus 路径不受任何适配器影响

### 新目标

1. **设计 ChainAdapter 统一接口**（TxIR/BlockIR/StateIR 翻译层）
2. **实现 EVM Adapter** 作为第一个参考实现（基于 revm）
3. **实现 WASM Adapter** 作为原生高性能路径
4. **设计 BTC/Solana Adapter 接口**（规划，Phase E 实施）
5. **实现 Adapter 热插拔机制**（动态加载/卸载）

---

## � Phase D.1: ChainAdapter 统一接口设计 (Week 1)

### 核心 Trait 定义

参考 `MULTICHAIN-ARCHITECTURE-VISION.md` 的接口草案：

```rust
// src/chain-adapters/src/lib.rs

use anyhow::Result;

/// 链标识符
#[derive(Debug, Clone, Hash, Eq, PartialEq)]
pub enum ChainId {
    EVM(u64),        // chain_id (1=Mainnet, 56=BSC, etc.)
    Bitcoin,         // BTC 主网
    Solana(u8),      // 101=Mainnet, 102=Testnet
    Wasm,            // SuperVM 原生 WASM
}

/// 统一交易中间表示 (TxIR)
#[derive(Debug, Clone)]
pub struct TxIR {
    pub tx_hash: [u8; 32],
    pub chain_id: ChainId,
    pub nonce_or_seq: u64,
    pub from: Vec<u8>,          // 归一化地址（EVM 20字节 / BTC 可变长 / Solana 32字节）
    pub to: Vec<u8>,
    pub value_list: Vec<u128>,  // BTC 多输出支持
    pub fee: FeeInfo,
    pub payload: TxPayload,
    pub timestamp: u64,
    pub privacy_tags: Option<PrivacyTags>,
}

/// 交易载荷（区分不同链的执行语义）
#[derive(Debug, Clone)]
pub enum TxPayload {
    EvmCall { data: Vec<u8> },
    EvmDeploy { bytecode: Vec<u8> },
    BtcTransfer { script: Vec<u8> },
    SolanaInvoke { program_id: [u8; 32], accounts: Vec<AccountMeta>, data: Vec<u8> },
    WasmInvoke { module: Vec<u8>, function: String, args: Vec<u8> },
}

/// 统一区块中间表示 (BlockIR)
#[derive(Debug, Clone)]
pub struct BlockIR {
    pub block_hash: [u8; 32],
    pub chain_id: ChainId,
    pub height: u64,
    pub parent_hash: [u8; 32],
    pub timestamp: u64,
    pub transactions: Vec<TxIR>,
    pub state_root: [u8; 32],
}

/// ChainAdapter 核心接口
pub trait ChainAdapter: Send + Sync {
    /// 链标识
    fn chain_id(&self) -> ChainId;
    
    /// 协议能力（支持的功能）
    fn protocol_caps(&self) -> ProtocolCaps;
    
    // ========== 网络层 ==========
    /// 启动 P2P 网络（连接原链节点）
    fn start_p2p(&mut self, cfg: P2PConfig) -> Result<()>;
    
    /// 轮询网络事件（接收区块/交易）
    fn poll_network(&mut self) -> Vec<RawInboundEvent>;
    
    // ========== 数据翻译层 ==========
    /// 原始区块 → BlockIR
    fn translate_block(&self, raw: RawBlock) -> Result<BlockIR>;
    
    /// 原始交易 → TxIR
    fn translate_tx(&self, raw: RawTx) -> Result<TxIR>;
    
    /// TxIR → 原生链格式（用于广播到原链网络）
    fn encode_tx(&self, ir: &TxIR) -> Result<RawTx>;
    
    // ========== 状态同步 ==========
    /// 获取当前状态快照（账户余额/UTXO 集/合约存储）
    fn state_snapshot(&self) -> Result<StateIR>;
    
    /// 应用 TxIR 到状态（返回状态变更）
    fn apply_tx(&self, state: &mut StateIR, tx: &TxIR) -> Result<Vec<StateChange>>;
    
    // ========== 最终性与重组 ==========
    /// 最终性策略（BTC: 6 确认，ETH: epoch，Solana: slot）
    fn finality_window(&self) -> FinalityPolicy;
    
    /// 检测链重组
    fn detect_reorg(&self, chain_state: &ChainState) -> Option<ReorgEvent>;
    
    // ========== 资源监控 ==========
    /// 适配器性能指标
    fn metrics(&self) -> AdapterMetrics;
}
```

### TxIR 设计要点

**关键挑战**: 如何归一化不同链的交易模型？

| 链 | 交易模型 | TxIR 映射策略 |
|---|----------|--------------|
| **Ethereum** | 账户模型 (from/to/value/data) | 直接映射，`payload = EvmCall` |
| **Bitcoin** | UTXO 模型 (inputs/outputs) | `from = inputs[0]`, `to = outputs[0]`, `value_list = outputs[*].value` |
| **Solana** | 账户模型 + 多指令 | `payload = SolanaInvoke`, 单 tx 可包含多 instruction |
| **WASM** | 函数调用 | `payload = WasmInvoke` |

**统一原则**:
- `from/to` 可变长字节数组（适配不同地址格式）
- `value_list` 支持多输出（BTC UTXO）
- `payload` 枚举保留链特定语义
- `privacy_tags` 可选字段（SuperVM 隐私增强）
│  ├─ gas_used (实际 Gas 消耗)              │
│  ├─ logs (事件日志)                       │
│  └─ state_diff (状态变更集)               │
└───────────────────────────────────────────┘
```

### revm 依赖分析

```toml
[dependencies]
# 核心 EVM
revm = { version = "5.0", default-features = false, features = ["std"] }
revm-primitives = "2.0"  # 基础类型定义（Address, U256, Bytes）

# 可选特性
# revm = { version = "5.0", features = ["serde", "memory_limit", "optimism"] }
```

### Database Trait 接口

```rust
use revm::primitives::{Address, U256, Bytes, B256};
use revm::Database;

/// revm Database trait（核心抽象）
pub trait Database {
    type Error;

    /// 获取账户基本信息（nonce, balance, code_hash）
    fn basic(&mut self, address: Address) -> Result<Option<AccountInfo>, Self::Error>;

    /// 获取合约代码
    fn code_by_hash(&mut self, code_hash: B256) -> Result<Bytecode, Self::Error>;

    /// 读取存储槽
    fn storage(&mut self, address: Address, index: U256) -> Result<U256, Self::Error>;

    /// 获取区块哈希
    fn block_hash(&mut self, number: U256) -> Result<B256, Self::Error>;
}

/// 账户信息结构
pub struct AccountInfo {
    pub balance: U256,
    pub nonce: u64,
    pub code_hash: B256,
    pub code: Option<Bytecode>,
}
```

### 典型使用流程

```rust
use revm::{EVM, primitives::{TransactTo, ExecutionResult, Output}};

// 1. 创建 EVM 实例（注入自定义 Database）
let mut evm = EVM::new();
evm.database(MyCustomDB::new());

// 2. 配置交易参数
evm.env.tx.caller = Address::from([0x01; 20]);
evm.env.tx.transact_to = TransactTo::Call(contract_address);
evm.env.tx.data = calldata.into();
evm.env.tx.value = U256::from(0);
evm.env.tx.gas_limit = 1_000_000;

// 3. 执行交易
let result = evm.transact_commit()?;

// 4. 处理结果
match result {
    ExecutionResult::Success { output, gas_used, logs, .. } => {
        println!("Gas used: {}", gas_used);
        for log in logs { println!("Event: {:?}", log); }
    }
    ExecutionResult::Revert { output, gas_used } => {
        println!("Reverted: {:?}", output);
    }
    ExecutionResult::Halt { reason, gas_used } => {
        println!("Halted: {:?}", reason);
    }
}
```

---

---

## 🔌 Phase D.2: EVM Adapter 参考实现 (Week 2-3)

### 模块结构

```
src/
├── chain-adapters/           # 适配器框架（新 crate）
│   ├── Cargo.toml
│   ├── src/
│   │   ├── lib.rs           # ChainAdapter trait 定义
│   │   ├── types.rs         # TxIR/BlockIR/StateIR
│   │   └── traits.rs        # 辅助 trait
│   └── tests/
│
├── evm-adapter/             # EVM 适配器实现（新 crate）
│   ├── Cargo.toml           # 依赖: revm, alloy-primitives
│   ├── src/
│   │   ├── lib.rs           # EVM Adapter 入口
│   │   ├── translator.rs    # EVM Tx/Block → TxIR/BlockIR
│   │   ├── executor.rs      # revm 执行封装
│   │   ├── database.rs      # revm Database trait 实现
│   │   └── p2p.rs           # DevP2P/ETHWire 网络层（可选）
│   └── examples/
│       └── erc20_demo.rs
│
└── wasm-adapter/            # WASM 适配器实现（新 crate）
    ├── Cargo.toml
    ├── src/
    │   ├── lib.rs
    │   └── executor.rs      # Wasmtime 执行封装
    └── tests/
```

### EVM Adapter 核心实现

#### 1. EVM Transaction Translator

```rust
// evm-adapter/src/translator.rs

use chain_adapters::{TxIR, ChainId, TxPayload, FeeInfo};
use alloy_primitives::{Address, U256, Bytes};

pub struct EvmTranslator {
    chain_id: u64,
}

impl EvmTranslator {
    pub fn translate_tx(&self, raw: &RawEvmTx) -> Result<TxIR> {
        Ok(TxIR {
            tx_hash: raw.hash().0,
            chain_id: ChainId::EVM(self.chain_id),
            nonce_or_seq: raw.nonce,
            from: raw.from.0.to_vec(),
            to: raw.to.map(|a| a.0.to_vec()).unwrap_or_default(),
            value_list: vec![raw.value.try_into()?],
            fee: FeeInfo {
                gas_price: raw.gas_price.try_into()?,
                gas_limit: raw.gas_limit,
            },
            payload: if raw.to.is_none() {
                TxPayload::EvmDeploy { bytecode: raw.input.to_vec() }
            } else {
                TxPayload::EvmCall { data: raw.input.to_vec() }
            },
            timestamp: 0, // 从区块获取
            privacy_tags: None,
        })
    }
    
    pub fn translate_block(&self, raw: &RawEvmBlock) -> Result<BlockIR> {
        let txs = raw.transactions.iter()
            .map(|tx| self.translate_tx(tx))
            .collect::<Result<Vec<_>>>()?;
        
        Ok(BlockIR {
            block_hash: raw.hash.0,
            chain_id: ChainId::EVM(self.chain_id),
            height: raw.number.try_into()?,
            parent_hash: raw.parent_hash.0,
            timestamp: raw.timestamp,
            transactions: txs,
            state_root: raw.state_root.0,
        })
    }
}
```

#### 2. revm Database 适配器

```rust
// evm-adapter/src/database.rs

use revm::primitives::{AccountInfo, Bytecode, B256, U256};
use revm::Database;
use chain_adapters::StateIR;

pub struct MvccEvmDatabase {
    state: Arc<RwLock<StateIR>>,
}

impl Database for MvccEvmDatabase {
    type Error = anyhow::Error;
    
    fn basic(&mut self, address: Address) -> Result<Option<AccountInfo>, Self::Error> {
        let state = self.state.read().unwrap();
        let key = format!("evm_account_{}", hex::encode(address.as_slice()));
        
        if let Some(data) = state.get(&key) {
            let info: AccountInfo = bincode::deserialize(&data)?;
            Ok(Some(info))
        } else {
            Ok(Some(AccountInfo::default()))
        }
    }
    
    fn code_by_hash(&mut self, code_hash: B256) -> Result<Bytecode, Self::Error> {
        let state = self.state.read().unwrap();
        let key = format!("evm_code_{}", hex::encode(code_hash));
        
        if let Some(data) = state.get(&key) {
            Ok(Bytecode::new_raw(data.into()))
        } else {
            Ok(Bytecode::default())
        }
    }
    
    fn storage(&mut self, address: Address, index: U256) -> Result<U256, Self::Error> {
        let state = self.state.read().unwrap();
        let key = format!("evm_storage_{}_{}", 
            hex::encode(address.as_slice()), 
            hex::encode(index.to_be_bytes::<32>())
        );
        
        if let Some(data) = state.get(&key) {
            Ok(U256::from_be_slice(&data))
        } else {
            Ok(U256::ZERO)
        }
    }
    
    fn block_hash(&mut self, number: u64) -> Result<B256, Self::Error> {
        let state = self.state.read().unwrap();
        let key = format!("block_hash_{}", number);
        
        if let Some(data) = state.get(&key) {
            Ok(B256::from_slice(&data))
        } else {
            Ok(B256::ZERO)
        }
    }
}
```

#### 3. EVM Adapter 实现

```rust
// evm-adapter/src/lib.rs

use chain_adapters::{ChainAdapter, ChainId, TxIR, BlockIR, StateIR};
use revm::{Evm, InMemoryDB};

pub struct EvmAdapter {
    chain_id: u64,
    translator: EvmTranslator,
    db: Arc<RwLock<MvccEvmDatabase>>,
}

impl ChainAdapter for EvmAdapter {
    fn chain_id(&self) -> ChainId {
        ChainId::EVM(self.chain_id)
    }
    
    fn protocol_caps(&self) -> ProtocolCaps {
        ProtocolCaps {
            tx_relay: true,
            block_sync: true,
            state_query: true,
        }
    }
    
    fn translate_tx(&self, raw: RawTx) -> Result<TxIR> {
        self.translator.translate_tx(&raw.as_evm()?)
    }
    
    fn translate_block(&self, raw: RawBlock) -> Result<BlockIR> {
        self.translator.translate_block(&raw.as_evm()?)
    }
    
    fn apply_tx(&self, state: &mut StateIR, tx: &TxIR) -> Result<Vec<StateChange>> {
        let mut evm = Evm::builder()
            .with_db(self.db.clone())
            .build();
        
        // 执行 EVM 交易（详细实现见 executor.rs）
        let result = evm.transact()?;
        
        // 提取状态变更
        let changes = self.extract_state_changes(&result)?;
        Ok(changes)
    }
    
    // ... 其他方法实现
}
```

### WASM Adapter 对比实现

```rust
// wasm-adapter/src/lib.rs

use chain_adapters::{ChainAdapter, ChainId, TxPayload};
use wasmtime::{Engine, Module, Store};

pub struct WasmAdapter {
    engine: Engine,
}

impl ChainAdapter for WasmAdapter {
    fn chain_id(&self) -> ChainId {
        ChainId::Wasm
    }
    
    fn apply_tx(&self, state: &mut StateIR, tx: &TxIR) -> Result<Vec<StateChange>> {
        if let TxPayload::WasmInvoke { module, function, args } = &tx.payload {
            let module = Module::new(&self.engine, module)?;
            let mut store = Store::new(&self.engine, ());
            let instance = Instance::new(&mut store, &module, &[])?;
            
            // 调用 WASM 函数
            let func = instance.get_func(&mut store, function)
                .ok_or(anyhow!("function not found"))?;
            
            let result = func.call(&mut store, args, &mut [])?;
            
            // 提取状态变更
            Ok(vec![/* ... */])
        } else {
            Err(anyhow!("invalid payload for WASM adapter"))
        }
    }
}
```

### 架构设计

```
┌─────────────────────────────────────────────────────────┐
│               SuperVM 统一执行层                        │
├─────────────────────────────────────────────────────────┤
│  ExecutionEngine Trait                                  │
│  ├─ WasmEngine (现有)                                   │
│  ├─ EvmEngine (新增) ◄─┐                                │
│  └─ GpuEngine (规划)   │                                │
├────────────────────────┼────────────────────────────────┤
│  EVM Adapter           │                                │
│  ├─ EvmExecutor        │                                │
│  ├─ MvccDatabase ◄─────┘ (核心桥接层)                   │
│  └─ StateMapper                                         │
├─────────────────────────────────────────────────────────┤
│  MVCC Store (统一状态后端)                              │
│  ├─ Account State (balance, nonce, code)                │
│  ├─ Storage Slots (contract storage)                    │
│  └─ Block Context (block_hash, timestamp, etc.)         │
└─────────────────────────────────────────────────────────┘
```

### MvccDatabase 实现

核心挑战：将 EVM 的 **账户状态树** 映射到 SuperVM 的 **键值 MVCC Store**

```rust
use revm::{Database, primitives::*};
use vm_runtime::{MvccStore, Txn};

/// EVM Database 适配器（基于 SuperVM MVCC）
pub struct MvccDatabase<'a> {
    txn: &'a mut Txn,
    cache: HashMap<Address, AccountInfo>, // 事务内缓存
}

impl<'a> MvccDatabase<'a> {
    pub fn new(txn: &'a mut Txn) -> Self {
        Self { txn, cache: HashMap::new() }
    }

    /// 键格式: evm:account:{address}:balance
    fn account_balance_key(addr: &Address) -> Vec<u8> {
        format!("evm:account:{}:balance", hex::encode(addr)).into_bytes()
    }

    /// 键格式: evm:account:{address}:nonce
    fn account_nonce_key(addr: &Address) -> Vec<u8> {
        format!("evm:account:{}:nonce", hex::encode(addr)).into_bytes()
    }

    /// 键格式: evm:account:{address}:code
    fn account_code_key(addr: &Address) -> Vec<u8> {
        format!("evm:account:{}:code", hex::encode(addr)).into_bytes()
    }

    /// 键格式: evm:storage:{address}:{slot}
    fn storage_key(addr: &Address, slot: &U256) -> Vec<u8> {
        format!("evm:storage:{}:{}", hex::encode(addr), slot).into_bytes()
    }
}

impl<'a> Database for MvccDatabase<'a> {
    type Error = anyhow::Error;

    fn basic(&mut self, address: Address) -> Result<Option<AccountInfo>, Self::Error> {
        // 优先查缓存
        if let Some(info) = self.cache.get(&address) {
            return Ok(Some(info.clone()));
        }

        // 从 MVCC 读取
        let balance_key = Self::account_balance_key(&address);
        let nonce_key = Self::account_nonce_key(&address);
        let code_key = Self::account_code_key(&address);

        let balance = self.txn.read(&balance_key)
            .and_then(|b| bincode::deserialize::<U256>(&b).ok())
            .unwrap_or(U256::ZERO);

        let nonce = self.txn.read(&nonce_key)
            .and_then(|b| bincode::deserialize::<u64>(&b).ok())
            .unwrap_or(0);

        let code = self.txn.read(&code_key)
            .map(|b| Bytecode::new_raw(b.into()));

        let code_hash = code.as_ref()
            .map(|c| keccak256(c.bytecode()))
            .unwrap_or(KECCAK_EMPTY);

        let info = AccountInfo { balance, nonce, code_hash, code };
        self.cache.insert(address, info.clone());
        Ok(Some(info))
    }

    fn code_by_hash(&mut self, _code_hash: B256) -> Result<Bytecode, Self::Error> {
        // 暂不实现 code_hash 索引（直接通过 basic 获取）
        Ok(Bytecode::new())
    }

    fn storage(&mut self, address: Address, index: U256) -> Result<U256, Self::Error> {
        let key = Self::storage_key(&address, &index);
        let value = self.txn.read(&key)
            .and_then(|b| bincode::deserialize::<U256>(&b).ok())
            .unwrap_or(U256::ZERO);
        Ok(value)
    }

    fn block_hash(&mut self, number: U256) -> Result<B256, Self::Error> {
        // 从 MVCC 读取历史区块哈希
        let key = format!("evm:block_hash:{}", number).into_bytes();
        let hash = self.txn.read(&key)
            .and_then(|b| B256::try_from(b.as_slice()).ok())
            .unwrap_or_default();
        Ok(hash)
    }
}
```

### EvmEngine 实现

```rust
use vm_runtime::{ExecutionEngine, ExecutionContext, ContractResult};
use revm::{EVM, primitives::*};

pub struct EvmEngine;

impl ExecutionEngine for EvmEngine {
    fn execute(&self, ctx: &ExecutionContext) -> ContractResult {
        // 1. 创建 MVCC 事务
        let mut mvcc_txn = ctx.storage.begin_txn();

        // 2. 注入 MvccDatabase
        let mut evm = EVM::new();
        evm.database(MvccDatabase::new(&mut mvcc_txn));

        // 3. 配置 EVM 环境
        evm.env.tx.caller = Address::from_slice(&ctx.sender);
        evm.env.tx.transact_to = TransactTo::Call(Address::from_slice(&ctx.contract_address));
        evm.env.tx.data = ctx.input.clone().into();
        evm.env.tx.value = U256::from(ctx.value);
        evm.env.tx.gas_limit = ctx.gas_limit;

        // 4. 执行
        let result = evm.transact_commit().map_err(|e| e.to_string())?;

        // 5. 提交 MVCC 事务
        mvcc_txn.commit()?;

        // 6. 转换结果
        match result {
            ExecutionResult::Success { output, gas_used, logs, .. } => {
                ContractResult {
                    success: true,
                    return_data: output.into_data().to_vec(),
                    gas_used: gas_used as u64,
                    logs: logs.into_iter().map(convert_log).collect(),
                    ..Default::default()
                }
            }
            ExecutionResult::Revert { output, gas_used } => {
                ContractResult {
                    success: false,
                    return_data: output.to_vec(),
                    gas_used: gas_used as u64,
                    error: Some("EVM Revert".into()),
                    ..Default::default()
                }
            }
            ExecutionResult::Halt { reason, gas_used } => {
                ContractResult {
                    success: false,
                    gas_used: gas_used as u64,
                    error: Some(format!("EVM Halt: {:?}", reason)),
                    ..Default::default()
                }
            }
        }
    }

    fn engine_type(&self) -> EngineType {
        EngineType::Evm
    }
}
```

---

## 🧪 Phase D.3: 适配器热插拔与动态加载 (Week 4)

### 适配器注册与管理

```rust
// src/chain-adapters/src/registry.rs

use std::collections::HashMap;
use std::sync::{Arc, RwLock};

pub struct AdapterRegistry {
    adapters: RwLock<HashMap<ChainId, Box<dyn ChainAdapter>>>,
}

impl AdapterRegistry {
    pub fn new() -> Self {
        Self {
            adapters: RwLock::new(HashMap::new()),
        }
    }
    
    /// 注册适配器
    pub fn register(&self, adapter: Box<dyn ChainAdapter>) -> Result<()> {
        let chain_id = adapter.chain_id();
        let mut adapters = self.adapters.write().unwrap();
        
        if adapters.contains_key(&chain_id) {
            return Err(anyhow!("adapter already registered for chain {:?}", chain_id));
        }
        
        adapters.insert(chain_id, adapter);
        Ok(())
    }
    
    /// 卸载适配器
    pub fn unregister(&self, chain_id: &ChainId) -> Result<()> {
        let mut adapters = self.adapters.write().unwrap();
        adapters.remove(chain_id)
            .ok_or(anyhow!("adapter not found"))?;
        Ok(())
    }
    
    /// 获取适配器
    pub fn get(&self, chain_id: &ChainId) -> Option<Arc<dyn ChainAdapter>> {
        let adapters = self.adapters.read().unwrap();
        adapters.get(chain_id).map(|a| Arc::clone(a))
    }
    
    /// 列出所有已注册适配器
    pub fn list_adapters(&self) -> Vec<ChainId> {
        let adapters = self.adapters.read().unwrap();
        adapters.keys().cloned().collect()
    }
}
```

### 启动配置

```rust
// examples/multi_chain_demo.rs

use chain_adapters::{AdapterRegistry, ChainId};
use evm_adapter::EvmAdapter;
use wasm_adapter::WasmAdapter;
use btc_adapter::BtcAdapter; // Phase E 实现

fn main() -> Result<()> {
    let registry = AdapterRegistry::new();
    
    // 注册 EVM 适配器（Ethereum 主网）
    let evm_eth = EvmAdapter::new(1)?; // chain_id = 1
    registry.register(Box::new(evm_eth))?;
    
    // 注册 EVM 适配器（BSC）
    let evm_bsc = EvmAdapter::new(56)?; // chain_id = 56
    registry.register(Box::new(evm_bsc))?;
    
    // 注册 WASM 适配器
    let wasm = WasmAdapter::new()?;
    registry.register(Box::new(wasm))?;
    
    // 注册 BTC 适配器（可选）
    #[cfg(feature = "btc-adapter")]
    {
        let btc = BtcAdapter::new()?;
        registry.register(Box::new(btc))?;
    }
    
    println!("Registered adapters: {:?}", registry.list_adapters());
    
    // 动态卸载适配器（例如禁用 BSC）
    registry.unregister(&ChainId::EVM(56))?;
    
    Ok(())
}
```

### Feature Flag 控制

```toml
# Cargo.toml (workspace)

[workspace]
members = [
    "src/vm-runtime",
    "src/chain-adapters",
    "src/evm-adapter",
    "src/wasm-adapter",
    "src/btc-adapter",    # 可选
    "src/solana-adapter", # 可选
]

[features]
default = ["wasm-adapter"]
evm-adapter = ["evm-adapter/default"]
btc-adapter = ["btc-adapter/default"]
solana-adapter = ["solana-adapter/default"]
all-adapters = ["evm-adapter", "btc-adapter", "solana-adapter", "wasm-adapter"]
```

构建示例：

```bash
# 仅 WASM（最小核心）
cargo build --no-default-features --features wasm-adapter

# WASM + EVM
cargo build --features evm-adapter

# 全部适配器
cargo build --features all-adapters
```

### 示例 1: 部署 ERC20 合约

```rust
// examples/evm_erc20_deploy.rs
use vm_runtime::{Runtime, EvmEngine};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let storage = MemoryStorage::new();
    let runtime = Runtime::new(storage);

    // ERC20 bytecode (编译自 Solidity)
    let erc20_bytecode = include_bytes!("../contracts/ERC20.bin");

    // 部署合约
    let deploy_ctx = ExecutionContext {
        sender: [0x01; 20],
        contract_address: [0x00; 20], // 部署时为空
        input: erc20_bytecode.to_vec(),
        value: 0,
        gas_limit: 5_000_000,
        ..Default::default()
    };

    let engine = EvmEngine;
    let result = engine.execute(&deploy_ctx)?;

    println!("Contract deployed at: {:?}", result.contract_address);
    Ok(())
}
```

### 示例 2: 调用 ERC20.transfer

```rust
// examples/evm_erc20_transfer.rs
use revm::primitives::Bytes;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let runtime = Runtime::new(MemoryStorage::new());
    let engine = EvmEngine;

    // ERC20.transfer(address recipient, uint256 amount) 函数签名
    let selector = keccak256(b"transfer(address,uint256)")[..4].to_vec();
    let recipient = Address::from([0x02; 20]);
    let amount = U256::from(1000);

    // ABI 编码
    let mut calldata = selector;
    calldata.extend_from_slice(&[0u8; 12]); // padding
    calldata.extend_from_slice(recipient.as_slice());
    calldata.extend_from_slice(&amount.to_be_bytes::<32>());

    // 调用合约
    let ctx = ExecutionContext {
        sender: [0x01; 20],
        contract_address: [0xAB; 20], // 已部署合约地址
        input: calldata,
        gas_limit: 100_000,
        ..Default::default()
    };

    let result = engine.execute(&ctx)?;
    assert!(result.success);
    println!("Transfer successful, gas used: {}", result.gas_used);
    Ok(())
}
```

### 测试覆盖

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mvcc_database_account_basic() {
        let store = MvccStore::new();
        let mut txn = store.begin();
        let mut db = MvccDatabase::new(&mut txn);

        let addr = Address::from([0x01; 20]);
        let info = db.basic(addr).unwrap();

        assert_eq!(info.balance, U256::ZERO);
        assert_eq!(info.nonce, 0);
    }

    #[test]
    fn test_mvcc_database_storage() {
        let store = MvccStore::new();
        let mut txn = store.begin();
        let mut db = MvccDatabase::new(&mut txn);

        let addr = Address::from([0x01; 20]);
        let slot = U256::from(42);

        // 写入
        let key = MvccDatabase::storage_key(&addr, &slot);
        txn.write(key, bincode::serialize(&U256::from(999)).unwrap());

        // 读取
        let value = db.storage(addr, slot).unwrap();
        assert_eq!(value, U256::from(999));
    }

    #[test]
    fn test_evm_engine_simple_call() {
        // 部署简单合约并调用
        let engine = EvmEngine;
        let ctx = ExecutionContext {
            sender: [0x01; 20],
            contract_address: [0x02; 20],
            input: vec![0x60, 0x42], // PUSH1 0x42
            gas_limit: 50_000,
            ..Default::default()
        };

        let result = engine.execute(&ctx).unwrap();
        assert!(result.success);
    }
}
```

---

## 🎯 Phase D.4: BTC/Solana Adapter 接口设计（规划）(Week 5-6)

### BTC Adapter 设计要点

**挑战**: UTXO 模型 vs 账户模型归一化

```rust
// src/btc-adapter/src/lib.rs

pub struct BtcAdapter {
    network: bitcoin::Network,
    node_client: Arc<BitcoinRpcClient>,
}

impl ChainAdapter for BtcAdapter {
    fn chain_id(&self) -> ChainId {
        ChainId::Bitcoin
    }
    
    fn translate_tx(&self, raw: RawTx) -> Result<TxIR> {
        let btc_tx: bitcoin::Transaction = raw.as_btc()?;
        
        // UTXO → 账户模型归一化
        Ok(TxIR {
            tx_hash: btc_tx.txid().as_byte_array().clone(),
            chain_id: ChainId::Bitcoin,
            nonce_or_seq: 0, // BTC 无 nonce，使用序列号
            from: btc_tx.input[0].previous_output.to_bytes(), // 简化
            to: btc_tx.output[0].script_pubkey.to_bytes(),
            value_list: btc_tx.output.iter().map(|o| o.value.to_sat() as u128).collect(),
            fee: self.calculate_fee(&btc_tx)?,
            payload: TxPayload::BtcTransfer {
                script: btc_tx.output[0].script_pubkey.to_bytes(),
            },
            timestamp: 0, // 从区块获取
            privacy_tags: None,
        })
    }
    
    fn apply_tx(&self, state: &mut StateIR, tx: &TxIR) -> Result<Vec<StateChange>> {
        // 将 UTXO 支出/生成转换为状态变更
        if let TxPayload::BtcTransfer { script } = &tx.payload {
            let changes = vec![
                StateChange::BtcUtxoSpent { /* ... */ },
                StateChange::BtcUtxoCreated { /* ... */ },
            ];
            Ok(changes)
        } else {
            Err(anyhow!("invalid payload for BTC adapter"))
        }
    }
    
    // P2P 网络层: 实现 Bitcoin P2P 协议
    fn start_p2p(&mut self, cfg: P2PConfig) -> Result<()> {
        // 连接到 Bitcoin 节点网络
        // 实现 version/verack/getdata/block/tx 消息处理
        todo!("Bitcoin P2P protocol implementation")
    }
}
```

### Solana Adapter 设计要点

**挑战**: 多指令事务 + 账户模型差异

```rust
// src/solana-adapter/src/lib.rs

pub struct SolanaAdapter {
    rpc_client: Arc<RpcClient>,
}

impl ChainAdapter for SolanaAdapter {
    fn chain_id(&self) -> ChainId {
        ChainId::Solana(101) // 主网
    }
    
    fn translate_tx(&self, raw: RawTx) -> Result<TxIR> {
        let sol_tx: solana_transaction::Transaction = raw.as_solana()?;
        
        // Solana 单个 tx 包含多个 instruction
        // 简化为只取第一个 instruction
        let ix = &sol_tx.message.instructions[0];
        
        Ok(TxIR {
            tx_hash: sol_tx.signatures[0].as_ref().try_into()?,
            chain_id: ChainId::Solana(101),
            nonce_or_seq: 0, // Solana 使用 recent_blockhash，无 nonce
            from: sol_tx.message.account_keys[0].to_bytes().to_vec(),
            to: sol_tx.message.account_keys[ix.program_id_index as usize]
                .to_bytes().to_vec(),
            value_list: vec![0], // Solana 转账金额在 instruction data 中
            fee: FeeInfo {
                gas_price: 5000, // Lamports per signature
                gas_limit: 1,
            },
            payload: TxPayload::SolanaInvoke {
                program_id: sol_tx.message.account_keys[ix.program_id_index as usize]
                    .to_bytes(),
                accounts: ix.accounts.clone(),
                data: ix.data.clone(),
            },
            timestamp: 0,
            privacy_tags: None,
        })
    }
    
    fn apply_tx(&self, state: &mut StateIR, tx: &TxIR) -> Result<Vec<StateChange>> {
        if let TxPayload::SolanaInvoke { program_id, accounts, data } = &tx.payload {
            // 调用 Solana Runtime (SVM) 执行指令
            // 这里需要集成 solana-program-runtime
            todo!("Solana instruction execution")
        } else {
            Err(anyhow!("invalid payload for Solana adapter"))
        }
    }
}
```

### 适配器对比表

| 适配器 | 交易模型 | 状态模型 | P2P 协议 | 最终性 | 实施优先级 |
|--------|----------|----------|----------|--------|-----------|
| **EVM** | 账户 | Merkle Patricia Trie | DevP2P | Epoch (12.8分钟) | **Week 2-3** |
| **WASM** | 函数调用 | Key-Value | 无（原生） | 即时 | **Week 2-3** |
| **BTC** | UTXO | UTXO Set | Bitcoin P2P | 6 确认 (~1小时) | **Phase E** |
| **Solana** | 多指令 | 账户 | QUIC/Turbine | Slot (~400ms) | **Phase E** |

### 优化方向

#### 1. 状态缓存
```rust
pub struct CachedMvccDatabase<'a> {
    inner: MvccDatabase<'a>,
    account_cache: LruCache<Address, AccountInfo>,
    storage_cache: LruCache<(Address, U256), U256>,
}

impl<'a> Database for CachedMvccDatabase<'a> {
    fn storage(&mut self, address: Address, index: U256) -> Result<U256, Self::Error> {
        if let Some(&value) = self.storage_cache.get(&(address, index)) {
            return Ok(value);
        }
        let value = self.inner.storage(address, index)?;
        self.storage_cache.put((address, index), value);
        Ok(value)
    }
}
```

#### 2. 并行 EVM 执行

```rust
use rayon::prelude::*;

pub fn execute_evm_batch(
    contexts: Vec<ExecutionContext>,
) -> Vec<ContractResult> {
    contexts.par_iter()
        .map(|ctx| EvmEngine.execute(ctx).unwrap())
        .collect()
}
```

#### 3. Precompile 优化

```rust
// 自定义高性能 Precompile（如 blake2b）
pub fn register_custom_precompiles(evm: &mut EVM) {
    evm.precompiles.insert(
        Address::from([0xFF; 20]),
        Box::new(Blake2bPrecompile),
    );
}
```

### SuperVM 集成

```rust
// supervm.rs 扩展
pub enum EngineType {
    Wasm,
    Evm, // 新增
    Gpu,
}

impl SuperVM {
    pub fn execute_with_engine(
        &self,
        engine_type: EngineType,
        ctx: ExecutionContext,
    ) -> ContractResult {
        match engine_type {
            EngineType::Wasm => WasmEngine.execute(&ctx),
            EngineType::Evm => EvmEngine.execute(&ctx),
            EngineType::Gpu => unimplemented!(),
        }
    }
}
```

---

---

## 📊 成功指标（更新）

### 架构指标
- ✅ **零侵入**: `vm-runtime` 核心无任何 EVM 依赖
- ✅ **插件化**: 所有适配器通过 `ChainAdapter` trait 统一接口
- ✅ **热插拔**: 支持运行时注册/卸载适配器（AdapterRegistry）
- ✅ **Feature Gating**: 可选编译任意适配器组合

### 功能指标（Phase D）
- ✅ `ChainAdapter` trait 设计完成（TxIR/BlockIR/StateIR）
- ✅ **EVM Adapter** 实现完成（revm 集成 + MVCC Database）
- ✅ **WASM Adapter** 实现完成（Wasmtime 集成）
- ✅ 跨适配器交易路由（根据 ChainId 自动分发）
- 📋 **BTC Adapter** 接口设计完成（Phase E 实施）
- 📋 **Solana Adapter** 接口设计完成（Phase E 实施）

### 性能指标（EVM Adapter）
- 🎯 EVM 合约执行 TPS > 10K（单核，简单转账）
- 🎯 EVM 存储读写延迟 < 1μs（MVCC 缓存命中）
- 🎯 ERC20 transfer 吞吐 > 5K TPS
- 🎯 相对 revm 原生性能 > 60%（MVCC 映射开销 < 40%）

### 兼容性指标
- ✅ 支持标准 ERC20/ERC721 合约
- ✅ 通过 revm 官方测试套件
- ✅ 兼容 Solidity 0.8.x 编译输出
- 📋 支持 EVM 预编译合约（ecrecover/sha256/etc.）

### 文档指标
- ✅ ChainAdapter 接口文档（本文档）
- ✅ EVM Adapter 使用指南
- ✅ WASM Adapter 使用指南
- 📋 BTC/Solana Adapter 设计文档（Phase E）
- 📋 多链统一架构白皮书更新

### 功能指标
- ✅ 成功部署 ERC20 合约
- ✅ 成功调用 ERC20.transfer
- ✅ 通过 OpenZeppelin 测试套件
- ✅ 支持 Solidity 0.8+ 特性

### 性能指标
- ✅ EVM 执行性能 > 50% revm 原生性能（考虑 MVCC 开销）
- ✅ 并行 EVM 执行线性扩展至 4 核
- ✅ 状态缓存命中率 > 80%

### 可观测性
- ✅ Prometheus 指标：evm_calls_total, evm_gas_used, evm_errors
- ✅ 区分 WASM 与 EVM 执行路径统计

---

## 📚 参考资源

### revm 文档
- [revm GitHub](https://github.com/bluealloy/revm)
- [revm Book](https://bluealloy.github.io/revm/)
- [Ethereum Yellow Paper](https://ethereum.github.io/yellowpaper/paper.pdf)

### 示例项目
- [reth](https://github.com/paradigmxyz/reth): 使用 revm 的 Ethereum 客户端
- [Foundry](https://github.com/foundry-rs/foundry): Solidity 开发框架，内置 revm

### Solidity 工具链
- [solc](https://docs.soliditylang.org/en/latest/installing-solidity.html): Solidity 编译器
- [OpenZeppelin Contracts](https://github.com/OpenZeppelin/openzeppelin-contracts)

---

---

## 📋 任务清单（更新）

### Week 1: ChainAdapter 框架设计
- [ ] 创建 `chain-adapters` crate（统一接口定义）
- [ ] 设计 `ChainAdapter` trait（参考 MULTICHAIN-ARCHITECTURE-VISION.md）
- [ ] 定义 TxIR/BlockIR/StateIR 统一中间表示
- [ ] 设计 `TxPayload` 枚举（支持 EVM/WASM/BTC/Solana）
- [ ] 实现 `AdapterRegistry`（注册/卸载/查询）
- [ ] 编写 ChainAdapter 接口文档

### Week 2-3: EVM & WASM Adapter 实现
- [ ] **EVM Adapter**:
  - [ ] 创建 `evm-adapter` crate（依赖 revm 5.0）
  - [ ] 实现 `EvmTranslator`（EVM Tx/Block → TxIR/BlockIR）
  - [ ] 实现 `MvccEvmDatabase`（revm Database trait）
  - [ ] 实现 `EvmAdapter`（ChainAdapter trait）
  - [ ] 测试 ERC20 部署与调用
- [ ] **WASM Adapter**:
  - [ ] 创建 `wasm-adapter` crate（依赖 wasmtime）
  - [ ] 实现 `WasmAdapter`（ChainAdapter trait）
  - [ ] 集成 SuperVM WASM 执行器
  - [ ] 测试简单 WASM 合约执行

### Week 4: 热插拔与集成测试
- [ ] 实现 Feature Flag 控制（default/evm-adapter/btc-adapter/etc.）
- [ ] 创建 `multi_chain_demo` 示例（同时运行 EVM + WASM）
- [ ] 测试运行时注册/卸载适配器
- [ ] 跨适配器交易路由测试
- [ ] 性能基准测试（EVM vs WASM TPS 对比）

### Week 5-6: BTC/Solana Adapter 设计（规划）
- [ ] **BTC Adapter** 接口设计:
  - [ ] UTXO → TxIR 映射策略
  - [ ] Bitcoin P2P 协议集成方案
  - [ ] SPV/头部同步策略
- [ ] **Solana Adapter** 接口设计:
  - [ ] 多指令 tx → TxIR 映射
  - [ ] QUIC/Turbine 网络协议集成
  - [ ] 账户模型差异处理
- [ ] 编写 BTC/Solana Adapter 设计文档

### 文档与优化
- [ ] 更新 ROADMAP.md Phase D 进度
- [ ] 更新 MULTICHAIN-ARCHITECTURE-VISION.md（实施细节）
- [ ] 编写"多链统一架构"博客文章
- [ ] 性能优化报告（EVM Adapter MVCC 映射开销分析）

### Week 1: revm 调研
- [x] 安装 revm 依赖
- [ ] 运行 revm 官方示例
- [ ] 阅读 Database trait 文档
- [ ] 分析 reth 的 Database 实现
- [ ] 编写技术调研报告

### Week 2: 适配器设计
- [ ] 实现 MvccDatabase 基础结构
- [ ] 实现 EvmEngine trait
- [ ] 设计键格式规范（文档化）
- [ ] 创建 evm-adapter crate

### Week 3: PoC 实现
- [ ] 部署简单合约测试
- [ ] 部署 ERC20 合约
- [ ] 调用 ERC20.transfer
- [ ] 编写单元测试（覆盖率 > 80%）

### Week 4: 优化与集成
- [ ] 实现状态缓存
- [ ] 并行 EVM 执行测试
- [ ] 集成到 SuperVM 路由器
- [ ] 性能基准测试
- [ ] 更新 ROADMAP Phase D 进度

---

## 🚀 未来扩展方向（更新）

### Phase E: BTC & Solana Adapter 完整实现（4-6 周）
- **BTC Adapter**:
  - 完整 Bitcoin P2P 协议实现（version/verack/getdata/block/tx）
  - SPV 轻客户端模式（仅同步区块头）
  - UTXO 集状态管理与重组处理
  - Lightning Network 二层支持（可选）
  
- **Solana Adapter**:
  - QUIC/Turbine 网络协议集成
  - Solana Runtime (SVM) 集成
  - 账户租金机制映射
  - Wormhole 跨链桥集成（可选）

### Phase F: 多链统一隐私层（Phase B 扩展）
- 跨链承诺树（统一 Merkle Root）
- 多链 Nullifier 集（防止跨链双花）
- 加密索引（跨链资产查询）
- ZK 证明聚合（批量验证多链交易）

### Phase G: P2P 网络层统一调度
- 多协议 P2P Orchestrator（同时运行 DevP2P/Bitcoin P2P/QUIC）
- 身份伪装（对外呈现为原链节点）
- 协议路由（根据消息类型分发到适配器）
- Reorg 事件总线（统一处理所有链的重组）

### Phase H: Web3 存储与寻址层
- 去中心化 Web 存储（基于 IPFS/Arweave）
- 域名系统（基于 ENS/Handshake）
- SuperVM Web3 浏览器（访问链上存储空间）
- 热插拔硬盘接入（传统网站迁移到区块链）

---

## 📖 总结与架构定位

### 核心理念调整

**原计划**: Phase D 作为"EVM 适配器"单一模块  
**新定位**: Phase D 作为"多链统一插件系统"的**基础设施实现**

```
多链统一架构全景:
├── Phase D: ChainAdapter 框架 + EVM/WASM 参考实现 ← 当前
├── Phase E: BTC/Solana Adapter 完整实现
├── Phase F: 多链统一隐私层
├── Phase G: P2P 网络层统一调度
└── Phase H: Web3 存储与寻址层
```

### 核心价值

1. **EVM 不是核心，而是插件之一**
   - 与 WASM/BTC/Solana 平等对待
   - 可选编译，零侵入内核
   - 热插拔，支持运行时动态加载

2. **统一抽象层（ChainAdapter）**
   - TxIR/BlockIR/StateIR 归一化所有链
   - ChainAdapter trait 标准化适配器接口
   - 隐私增强层叠加（privacy_tags 可选字段）

3. **性能不妥协**
   - FastPath 28.57M TPS 不受任何适配器影响
   - EVM Adapter 通过 MVCC 映射达到 >60% revm 性能
   - WASM Adapter 复用 SuperVM 原生执行器（零开销）

4. **渐进式采用（参考 MULTICHAIN-ARCHITECTURE-VISION.md）**
   - 初期: 透明代理模式（SuperVM 镜像原链 + 提供私有接口）
   - 中期: Encouraged Mode（用户获得性能/费用优势）
   - 后期: Native Dominant（SuperVM 原生协议主导）

---

**下一步行动**: 
1. **Week 1**: 设计 `ChainAdapter` trait 详细接口（参考 MULTICHAIN-ARCHITECTURE-VISION.md）
2. **Week 2-3**: 实现 EVM Adapter + WASM Adapter 参考实现
3. **Week 4**: 热插拔机制 + 跨适配器路由测试
4. **Phase E**: 启动 BTC/Solana Adapter 设计与实施！🚀
