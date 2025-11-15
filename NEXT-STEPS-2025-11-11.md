# SuperVM 下一步开发计划
**日期**: 2025-11-11  
**当前分支**: king/l0-mvcc-privacy-verification  
**整体进度**: 54% (L0: 96% | L1: 50% | L2: 0% | L3: 5% | L4: 10%)

---

## 📊 当前状态总览

### ✅ 已完成的核心能力 (L0层 96%)
- **MVCC并发控制**: 单核242K TPS，多核495K TPS (2PC混合负载)
- **批量+流水线2PC**: 多线程1.19M TPS (+76.5% 性能提升)
- **ZK隐私验证**: Groth16 RingCT，批量验证200+ TPS，延迟P95<10ms
- **三通道路由**: AdaptiveRouter自适应路由 (FastPath/Consensus/Privacy)
- **可观测性**: Prometheus指标 + Grafana统一Dashboard + 12条告警规则 ✅
- **跨分片协议**: 2PC prepare/commit 完整指标，并行读校验 (+56% TPS)

### 🔧 最近更新 (2025-11-11)
1. **L0.9 可观测性 100%完成**:
   - 统一Grafana Dashboard (`grafana-supervm-unified-dashboard.json`)
   - Prometheus告警规则 (`prometheus-supervm-alerts.yml`)
   - 完整文档 (`docs/GRAFANA-DASHBOARD.md`)

2. **L0.6 三通道路由 92%**:
   - AdaptiveRouter核心实现完成
   - 9个环境变量配置 (SUPERVM_ADAPTIVE_*)
   - 待验证: 性能基准测试 (28M TPS目标)

---

## 🎯 下一步开发路线 (按优先级排序)

### **优先级1: L0层收尾与验证** (预计1周)

#### 任务1.1: 三通道路由性能验证 (L0.6: 92% → 100%)
**目标**: 验证FastPath性能达到28M TPS，确保三通道稳定运行

**执行步骤**:
```powershell
# Step 1: 运行FastPath性能基准测试
cargo run --release --example mixed_path_bench

# Step 2: 端到端三通道稳定性测试
cargo test --release e2e_three_channel_test

# Step 3: 验证AdaptiveRouter自适应调整
# 检查日志中的路由切换行为和冲突率统计
```

**验收标准**:
- ✅ FastPath独占对象: ≥28M TPS
- ✅ Consensus共享对象: ≥290K TPS
- ✅ AdaptiveRouter自适应调整正常 (冲突率<5%时路由到FastPath)
- ✅ 无运行时错误或panic

**预期产出**:
- 性能基准报告 (更新至 `BENCHMARK_RESULTS.md`)
- L0.6进度更新至100%

---

### **优先级2: L1协议适配层设计** (预计2-3周)

#### 任务2.1: ChainAdapter统一接口设计 (L1.2: 40% → 80%)
**目标**: 定义多链统一抽象层，为外部链适配器插件提供标准接口

**核心文件结构**:
```
src/chain_adapter/
├── mod.rs              # 公共导出模块
├── traits.rs           # ChainAdapter trait定义
├── ir.rs               # TxIR/BlockIR/StateIR统一IR
├── registry.rs         # ChainAdapterRegistry注册表
├── svm_native.rs       # SvmNativeAdapter (SuperVM原生WASM)
└── tests/
    ├── ir_tests.rs     # IR转换测试
    └── registry_tests.rs # 注册表测试
```

**核心代码设计**:

```rust
// src/chain_adapter/traits.rs
use crate::chain_adapter::ir::{TxIR, BlockIR, StateIR};
use std::sync::Arc;

/// 链标识符
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ChainId {
    SuperVM,
    Ethereum,
    BSC,
    Polygon,
    Bitcoin,
    Solana,
    TRON,
}

/// 多链适配器统一接口
pub trait ChainAdapter: Send + Sync {
    /// 链标识
    fn chain_id(&self) -> ChainId;
    
    /// 交易IR转换 (原链交易 → SuperVM TxIR)
    fn translate_tx(&self, raw_tx: &[u8]) -> Result<TxIR, AdapterError>;
    
    /// 区块IR转换 (原链区块 → SuperVM BlockIR)
    fn translate_block(&self, raw_block: &[u8]) -> Result<BlockIR, AdapterError>;
    
    /// 状态映射 (原链状态 → SuperVM StateIR)
    fn map_state(&self, chain_state: &[u8]) -> Result<StateIR, AdapterError>;
    
    /// 签名验证 (使用原链验证逻辑)
    fn verify_signature(&self, tx: &TxIR) -> Result<bool, AdapterError>;
    
    /// Gas模型转换 (原链Gas → SuperVM Gas)
    fn convert_gas(&self, chain_gas: u64) -> u64;
}

#[derive(Debug, thiserror::Error)]
pub enum AdapterError {
    #[error("Invalid transaction format")]
    InvalidTransaction,
    #[error("Unsupported chain: {0:?}")]
    UnsupportedChain(ChainId),
    #[error("Signature verification failed")]
    InvalidSignature,
    #[error("Decode error: {0}")]
    DecodeError(String),
}
```

```rust
// src/chain_adapter/ir.rs
use primitive_types::{H256, U256};

/// 统一交易IR (Transaction Intermediate Representation)
#[derive(Debug, Clone)]
pub struct TxIR {
    /// 发送者地址 (统一为20字节)
    pub from: Address,
    
    /// 接收者地址 (None表示合约创建)
    pub to: Option<Address>,
    
    /// 转账金额
    pub value: U256,
    
    /// 调用数据/合约字节码
    pub data: Vec<u8>,
    
    /// Nonce (防重放)
    pub nonce: u64,
    
    /// Gas限制
    pub gas_limit: u64,
    
    /// Gas价格
    pub gas_price: U256,
    
    /// 原始签名 (便于跨链验证)
    pub signature: Option<Signature>,
    
    /// 来源链标识
    pub source_chain: ChainId,
}

/// 统一地址 (20字节, 兼容EVM)
pub type Address = [u8; 20];

/// 统一签名
#[derive(Debug, Clone)]
pub struct Signature {
    pub r: U256,
    pub s: U256,
    pub v: u8,
}

/// 统一区块IR
#[derive(Debug, Clone)]
pub struct BlockIR {
    /// 区块号
    pub number: u64,
    
    /// 时间戳
    pub timestamp: u64,
    
    /// 父区块哈希
    pub parent_hash: H256,
    
    /// 状态根
    pub state_root: H256,
    
    /// 交易列表
    pub transactions: Vec<TxIR>,
    
    /// 来源链标识
    pub source_chain: ChainId,
}

/// 统一状态IR
#[derive(Debug, Clone)]
pub struct StateIR {
    /// 账户余额
    pub balance: U256,
    
    /// Nonce
    pub nonce: u64,
    
    /// 合约代码哈希
    pub code_hash: Option<H256>,
    
    /// 存储根
    pub storage_root: H256,
}
```

```rust
// src/chain_adapter/registry.rs
use super::traits::{ChainAdapter, ChainId};
use std::collections::HashMap;
use std::sync::{Arc, RwLock};

/// 链适配器注册表 (全局单例)
pub struct ChainAdapterRegistry {
    adapters: RwLock<HashMap<ChainId, Arc<dyn ChainAdapter>>>,
}

impl ChainAdapterRegistry {
    pub fn new() -> Self {
        Self {
            adapters: RwLock::new(HashMap::new()),
        }
    }
    
    /// 注册适配器
    pub fn register(&self, adapter: Arc<dyn ChainAdapter>) -> Result<(), String> {
        let chain_id = adapter.chain_id();
        let mut adapters = self.adapters.write().unwrap();
        
        if adapters.contains_key(&chain_id) {
            return Err(format!("Adapter for {:?} already registered", chain_id));
        }
        
        adapters.insert(chain_id, adapter);
        Ok(())
    }
    
    /// 获取适配器
    pub fn get(&self, chain_id: ChainId) -> Option<Arc<dyn ChainAdapter>> {
        self.adapters.read().unwrap().get(&chain_id).cloned()
    }
    
    /// 列出所有已注册链
    pub fn list_chains(&self) -> Vec<ChainId> {
        self.adapters.read().unwrap().keys().copied().collect()
    }
}

// 全局注册表
lazy_static::lazy_static! {
    pub static ref GLOBAL_REGISTRY: ChainAdapterRegistry = ChainAdapterRegistry::new();
}
```

```rust
// src/chain_adapter/svm_native.rs
use super::traits::{ChainAdapter, ChainId, AdapterError};
use super::ir::{TxIR, BlockIR, StateIR};

/// SuperVM 原生WASM适配器 (零开销)
pub struct SvmNativeAdapter;

impl ChainAdapter for SvmNativeAdapter {
    fn chain_id(&self) -> ChainId {
        ChainId::SuperVM
    }
    
    fn translate_tx(&self, raw_tx: &[u8]) -> Result<TxIR, AdapterError> {
        // SuperVM原生交易已经是标准IR格式，直接反序列化
        bincode::deserialize(raw_tx)
            .map_err(|e| AdapterError::DecodeError(e.to_string()))
    }
    
    fn translate_block(&self, raw_block: &[u8]) -> Result<BlockIR, AdapterError> {
        bincode::deserialize(raw_block)
            .map_err(|e| AdapterError::DecodeError(e.to_string()))
    }
    
    fn map_state(&self, chain_state: &[u8]) -> Result<StateIR, AdapterError> {
        bincode::deserialize(chain_state)
            .map_err(|e| AdapterError::DecodeError(e.to_string()))
    }
    
    fn verify_signature(&self, tx: &TxIR) -> Result<bool, AdapterError> {
        // SuperVM原生验证逻辑
        Ok(true) // TODO: 实现ed25519/secp256k1验证
    }
    
    fn convert_gas(&self, chain_gas: u64) -> u64 {
        chain_gas // SuperVM原生Gas，无需转换
    }
}
```

**单元测试示例**:
```rust
// src/chain_adapter/tests/registry_tests.rs
#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_register_and_get_adapter() {
        let registry = ChainAdapterRegistry::new();
        let svm_adapter = Arc::new(SvmNativeAdapter);
        
        // 注册适配器
        registry.register(svm_adapter.clone()).unwrap();
        
        // 获取适配器
        let adapter = registry.get(ChainId::SuperVM).unwrap();
        assert_eq!(adapter.chain_id(), ChainId::SuperVM);
        
        // 重复注册应报错
        assert!(registry.register(svm_adapter).is_err());
    }
    
    #[test]
    fn test_list_chains() {
        let registry = ChainAdapterRegistry::new();
        registry.register(Arc::new(SvmNativeAdapter)).unwrap();
        
        let chains = registry.list_chains();
        assert_eq!(chains.len(), 1);
        assert!(chains.contains(&ChainId::SuperVM));
    }
}
```

**验收标准**:
- ✅ ChainAdapter trait编译通过
- ✅ TxIR/BlockIR/StateIR结构定义完整
- ✅ ChainAdapterRegistry单元测试全部通过
- ✅ SvmNativeAdapter功能验证通过

**预期产出**:
- `src/chain_adapter/` 完整模块
- L1.2进度更新至80%
- 技术文档 `docs/CHAIN-ADAPTER-DESIGN.md`

---

### **优先级3: L3.2 EVM Adapter插件开发** (预计3-4周)

#### 任务3.1: EVM适配器基础实现 (10% → 60%)
**目标**: 实现EVM链适配器，支持Ethereum/BSC/Polygon等EVM兼容链

**技术栈**:
- `revm 5.0`: Rust EVM执行引擎
- `alloy-primitives`: EVM类型库
- `ethers-core`: 交易解析

**核心文件结构**:
```
src/adapters/evm/
├── mod.rs              # EVM适配器公共导出
├── adapter.rs          # EvmAdapter实现ChainAdapter
├── database.rs         # MvccEvmDatabase (MVCC ↔ revm状态桥接)
├── translator.rs       # EVM Tx → TxIR转换
├── gas.rs              # EVM Gas模型转换
└── tests/
    ├── erc20_test.rs   # ERC20合约测试
    └── erc721_test.rs  # ERC721合约测试
```

**依赖配置**:
```toml
# Cargo.toml
[dependencies]
revm = { version = "5.0", features = ["std", "serde"] }
alloy-primitives = "0.7"
ethers-core = "2.0"
rlp = "0.5"
```

**核心代码实现**:

```rust
// src/adapters/evm/database.rs
use revm::{Database, DatabaseRef};
use revm::primitives::{Address, U256, AccountInfo, Bytecode, B256};
use crate::mvcc::{MvccStore, Storage};
use std::sync::Arc;

/// MVCC ↔ EVM状态桥接层
pub struct MvccEvmDatabase<S: Storage> {
    mvcc: Arc<MvccStore<S>>,
    start_ts: u64,
}

impl<S: Storage> MvccEvmDatabase<S> {
    pub fn new(mvcc: Arc<MvccStore<S>>, start_ts: u64) -> Self {
        Self { mvcc, start_ts }
    }
}

impl<S: Storage> DatabaseRef for MvccEvmDatabase<S> {
    type Error = String;
    
    /// 读取账户基本信息
    fn basic_ref(&self, address: Address) -> Result<Option<AccountInfo>, Self::Error> {
        let key = format!("evm:account:{:?}", address);
        
        match self.mvcc.read(&key, self.start_ts) {
            Ok(Some(data)) => {
                // 反序列化账户信息
                let account: AccountInfo = bincode::deserialize(&data)
                    .map_err(|e| format!("Failed to decode account: {}", e))?;
                Ok(Some(account))
            }
            Ok(None) => Ok(None),
            Err(e) => Err(format!("MVCC read error: {:?}", e)),
        }
    }
    
    /// 读取合约代码
    fn code_by_hash_ref(&self, code_hash: B256) -> Result<Bytecode, Self::Error> {
        let key = format!("evm:code:{:?}", code_hash);
        
        match self.mvcc.read(&key, self.start_ts) {
            Ok(Some(data)) => Ok(Bytecode::new_raw(data.into())),
            Ok(None) => Ok(Bytecode::default()),
            Err(e) => Err(format!("Code read error: {:?}", e)),
        }
    }
    
    /// 读取存储槽位
    fn storage_ref(&self, address: Address, index: U256) -> Result<U256, Self::Error> {
        let key = format!("evm:storage:{:?}:{:?}", address, index);
        
        match self.mvcc.read(&key, self.start_ts) {
            Ok(Some(data)) => {
                let value: [u8; 32] = data.try_into()
                    .map_err(|_| "Invalid storage value")?;
                Ok(U256::from_be_bytes(value))
            }
            Ok(None) => Ok(U256::ZERO),
            Err(e) => Err(format!("Storage read error: {:?}", e)),
        }
    }
    
    /// 读取区块哈希
    fn block_hash_ref(&self, number: u64) -> Result<B256, Self::Error> {
        let key = format!("evm:block_hash:{}", number);
        
        match self.mvcc.read(&key, self.start_ts) {
            Ok(Some(data)) => {
                let hash: [u8; 32] = data.try_into()
                    .map_err(|_| "Invalid block hash")?;
                Ok(B256::from(hash))
            }
            Ok(None) => Ok(B256::default()),
            Err(e) => Err(format!("Block hash read error: {:?}", e)),
        }
    }
}
```

```rust
// src/adapters/evm/translator.rs
use crate::chain_adapter::ir::{TxIR, Address, Signature};
use crate::chain_adapter::traits::ChainId;
use ethers_core::types::Transaction as EthTx;
use rlp::Rlp;

/// EVM交易 → SuperVM TxIR转换器
pub struct EvmTranslator;

impl EvmTranslator {
    /// 解析RLP编码的EVM交易
    pub fn decode_transaction(raw_tx: &[u8]) -> Result<TxIR, String> {
        // 解析RLP交易
        let eth_tx: EthTx = rlp::decode(raw_tx)
            .map_err(|e| format!("RLP decode error: {}", e))?;
        
        // 转换为TxIR
        Ok(TxIR {
            from: eth_tx.from.0,
            to: eth_tx.to.map(|addr| addr.0),
            value: eth_tx.value.into(),
            data: eth_tx.input.to_vec(),
            nonce: eth_tx.nonce.as_u64(),
            gas_limit: eth_tx.gas.as_u64(),
            gas_price: eth_tx.gas_price.unwrap_or_default().into(),
            signature: Some(Signature {
                r: eth_tx.r.into(),
                s: eth_tx.s.into(),
                v: eth_tx.v.as_u64() as u8,
            }),
            source_chain: ChainId::Ethereum,
        })
    }
}
```

```rust
// src/adapters/evm/adapter.rs
use crate::chain_adapter::traits::{ChainAdapter, ChainId, AdapterError};
use crate::chain_adapter::ir::{TxIR, BlockIR, StateIR};
use super::translator::EvmTranslator;

/// EVM链适配器 (支持Ethereum/BSC/Polygon)
pub struct EvmAdapter {
    chain_id: ChainId,
}

impl EvmAdapter {
    pub fn new(chain_id: ChainId) -> Self {
        assert!(matches!(chain_id, ChainId::Ethereum | ChainId::BSC | ChainId::Polygon));
        Self { chain_id }
    }
}

impl ChainAdapter for EvmAdapter {
    fn chain_id(&self) -> ChainId {
        self.chain_id
    }
    
    fn translate_tx(&self, raw_tx: &[u8]) -> Result<TxIR, AdapterError> {
        EvmTranslator::decode_transaction(raw_tx)
            .map_err(|e| AdapterError::DecodeError(e))
    }
    
    fn translate_block(&self, raw_block: &[u8]) -> Result<BlockIR, AdapterError> {
        // TODO: 实现区块解析
        todo!("Block translation not implemented")
    }
    
    fn map_state(&self, chain_state: &[u8]) -> Result<StateIR, AdapterError> {
        // TODO: 实现状态映射
        todo!("State mapping not implemented")
    }
    
    fn verify_signature(&self, tx: &TxIR) -> Result<bool, AdapterError> {
        // TODO: 实现secp256k1签名验证
        Ok(true)
    }
    
    fn convert_gas(&self, evm_gas: u64) -> u64 {
        // EVM Gas → SuperVM Gas (1:1映射)
        evm_gas
    }
}
```

**ERC20合约测试**:
```rust
// src/adapters/evm/tests/erc20_test.rs
#[cfg(test)]
mod tests {
    use super::*;
    use revm::{EVM, InMemoryDB};
    
    #[test]
    fn test_erc20_transfer() {
        // 1. 部署ERC20合约
        let mut evm = EVM::new();
        let contract_bytecode = include_bytes!("../../contracts/ERC20.bin");
        
        // 2. 调用transfer()
        let transfer_data = encode_transfer(recipient, amount);
        
        // 3. 执行交易
        let result = evm.transact_commit();
        
        // 4. 验证余额变化
        assert_eq!(get_balance(recipient), amount);
    }
}
```

**验收标准**:
- ✅ EvmAdapter实现ChainAdapter trait
- ✅ MvccEvmDatabase桥接层功能正常
- ✅ EVM交易→TxIR转换正确
- ✅ ERC20合约转账测试通过
- ✅ Gas消耗与Geth误差<5%

**预期产出**:
- `src/adapters/evm/` 完整模块
- L3.2进度更新至60%
- EVM适配器文档 `docs/EVM-ADAPTER.md`

---

### **优先级4: L0.7 ZK隐私层优化** (95% → 98%)

#### 任务4.1: Bulletproofs Range Proof集成
**目标**: 替换当前Groth16 64-bit Range Proof，降低约束数和证明时间

**依赖库**:
```toml
# Cargo.toml
[dependencies]
bulletproofs = "4.0"
curve25519-dalek = "4.1"
merlin = "3.0"
```

**性能对比**:
| 方案 | 约束数 | 证明时间 | 证明大小 | Setup |
|------|--------|---------|---------|-------|
| Groth16 64-bit | 64 | ~4ms | 128B | Trusted |
| Bulletproofs 64-bit | ~60 | ~8ms | ~672B | Transparent |

**实现步骤**:
1. 集成Bulletproofs库到`zk-groth16-test/`
2. 实现64-bit Range Proof生成与验证
3. 对比Groth16与Bulletproofs性能
4. 根据场景选择方案 (链上用Groth16，链下用Bulletproofs)

**注意事项**:
- Bulletproofs证明更大，但Setup透明 (无需Trusted Ceremony)
- 适合链下聚合场景，不适合链上验证 (Gas高)

---

### **优先级5: L4.1 四层网络架构PoC** (10% → 30%)

#### 任务5.1: 分层通信协议原型
**参考文档**: `docs/four-layer-network-deployment-and-compute-scheduling.md`

**最小可行原型**:
```
src/network/
├── layers/
│   ├── l4_super_compute.rs  # 超算层节点 (高性能服务器)
│   ├── l4_miner.rs           # 矿机层节点 (通用计算)
│   ├── l4_edge.rs            # 边缘层节点 (轻节点/IoT)
│   └── l4_mobile.rs          # 移动层节点 (手机/浏览器)
├── protocol.rs               # 分层通信协议
├── scheduler.rs              # 任务调度器
└── node_registry.rs          # 节点注册与发现
```

**核心功能**:
1. **节点分层注册**: 节点启动时向注册中心报告所属层级
2. **任务下发**: L4-Sub1向L4-Sub2/Sub3下发计算任务
3. **结果上报**: L4-Sub2/Sub3完成后上报结果
4. **工作量证明**: 简单的PoW防止Sybil攻击

**PoC目标**:
- 实现4层节点类型
- 任务分发与结果收集
- 分层通信协议 (HTTP/gRPC)

---

## 📅 时间规划建议

### 第1周 (2025-11-11 ~ 11-17)
- ✅ 更新ROADMAP进度 (已完成)
- 🔧 L0.6 三通道路由性能验证
- 📚 L1.2 ChainAdapter接口设计文档

### 第2-3周 (2025-11-18 ~ 12-01)
- 🔨 L1.2 ChainAdapter核心实现
- 🧪 单元测试与集成测试
- 📝 技术文档编写

### 第4-6周 (2025-12-02 ~ 12-22)
- 🔨 L3.2 EVM Adapter插件开发
- 🧪 ERC20/ERC721合约测试
- 📊 性能基准测试

### 第7周+ (2025-12-23 ~)
- 🔐 L0.7 Bulletproofs集成
- 🌐 L4.1 四层网络PoC
- 📈 持续性能优化

---

## 🎯 关键里程碑

| 里程碑 | 时间 | 交付物 |
|--------|------|--------|
| **L0层完成** | 2025-11-17 | L0.6验证完成, L0进度100% |
| **L1层框架** | 2025-12-01 | ChainAdapter接口, SvmNativeAdapter |
| **L3 EVM插件** | 2025-12-22 | EvmAdapter, ERC20测试通过 |
| **L0 ZK优化** | 2026-01-05 | Bulletproofs集成 |
| **L4 网络PoC** | 2026-01-19 | 四层网络原型 |

---

## 📚 参考资料

### 核心文档
- `ROADMAP.md` - 总体开发路线图
- `ROADMAP-ZK-Privacy.md` - ZK隐私层专项路线图
- `L0-UPGRADE-REPORT-2025-11-11.md` - L0层最新进展
- `docs/GRAFANA-DASHBOARD.md` - 可观测性完整指南

### 技术规范
- `docs/compiler-and-gas-innovation.md` - WODA编译器设计
- `docs/four-layer-network-deployment-and-compute-scheduling.md` - 四层网络架构
- `docs/ARCH-CPU-GPU-HYBRID.md` - CPU-GPU异构计算

### 外部依赖
- [revm](https://github.com/bluealloy/revm) - Rust EVM执行引擎
- [arkworks](https://github.com/arkworks-rs) - ZK证明库
- [bulletproofs](https://github.com/dalek-cryptography/bulletproofs) - Range Proof

---

## ✅ 检查清单

### 开发前准备
- [ ] 阅读相关设计文档
- [ ] 了解现有代码结构
- [ ] 配置开发环境 (Rust 1.75+)

### 开发中
- [ ] 遵循Rust最佳实践
- [ ] 编写单元测试 (覆盖率>80%)
- [ ] 添加性能基准测试
- [ ] 更新技术文档

### 开发后
- [ ] 代码审查 (Code Review)
- [ ] 集成测试通过
- [ ] 性能指标达标
- [ ] 更新ROADMAP进度

---

## 🚀 快速开始

### 验证L0.6三通道路由
```powershell
# 性能基准测试
cargo run --release --example mixed_path_bench

# 端到端测试
cargo test --release e2e_three_channel_test
```

### 开发L1.2 ChainAdapter
```powershell
# 创建模块
mkdir -p src/chain_adapter/tests

# 运行测试
cargo test --package supervm --lib chain_adapter
```

### 开发L3.2 EVM Adapter
```powershell
# 创建插件目录
mkdir -p src/adapters/evm/tests

# 添加依赖
cargo add revm alloy-primitives ethers-core

# 运行EVM测试
cargo test --package supervm-evm-adapter
```

---

**下一步行动**: 建议从 **L0.6三通道路由性能验证** 开始，确保L0层100%完成后再推进L1/L3层开发。
