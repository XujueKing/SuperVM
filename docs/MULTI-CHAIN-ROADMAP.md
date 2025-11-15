# SuperVM 多链支持路线图

> **最后更新**: 2025-11-14  
> **状态**: L1 骨架完成，L3 插件规划中

## 📋 概述

SuperVM 采用分层架构实现多链聚合，通过 **统一接口 + 热插拔插件** 模式支持 12+ 条主流公链。

### 架构原则

```
┌──────────────────────────────────────────────┐
│ L1 协议适配层 (骨架层)                      │
│  • ChainAdapter 统一接口                    │
│  • ChainType 枚举定义                       │
│  • 最小可行骨架实现                         │
│  • 工厂模式创建                             │
└──────────────────────────────────────────────┘
                    ▼
┌──────────────────────────────────────────────┐
│ L3 应用层 (插件层)                          │
│  • L3.2 外部链适配器插件                    │
│     - 真实协议实现 (revm/TVM/Move VM)       │
│     - IR 翻译层                             │
│  • L3.4 多链节点热插拔                      │
│     - 节点生命周期管理                      │
│     - 数据同步与健康监控                    │
└──────────────────────────────────────────────┘
```

---

## 🌐 支持的公链 (12+)

### ✅ 已实现骨架 (L1 层)

| 链名称 | ChainType | 骨架文件 | 状态 | 测试 |
|--------|-----------|----------|------|------|
| SuperVM | `SuperVM` | `wasm_adapter.rs` | 100% ✅ | ✅ |
| EVM 通用 | `EVM` | `evm_adapter.rs` | 90% ✅ | ✅ |
| TRON | `TRON` | `tron_adapter.rs` | 90% ✅ | ✅ |
| Bitcoin | `Bitcoin` | `bitcoin_submodule.rs` | 85% ✅ | ✅ |
| Solana | `Solana` | `solana_submodule.rs` | 85% ✅ | ✅ |

### 📋 规划中 (L1 枚举已定义 + L3 插件待实现)

#### EVM 系列

| 链名称 | ChainType | 特性 | 优先级 |
|--------|-----------|------|--------|
| Polygon | `Polygon` | Layer 2, PoS 共识, Plasma Bridge | Phase 7 |
| BNB Chain | `BNB` | Parlia PoSA, 快速最终性, 双链架构 | Phase 7 |
| Avalanche | `Avalanche` | 雪崩共识, 子网架构, 三链协同 | Phase 7 |

**共同点**: 均 EVM 兼容，可复用 `EvmAdapter` 骨架，差异化处理共识层与 Gas 模型。

#### Move 系列

| 链名称 | ChainType | 特性 | 优先级 |
|--------|-----------|------|--------|
| Sui | `Sui` | Move VM, 对象中心模型, Narwhal & Bullshark 共识 | Phase 7 |
| Aptos | `Aptos` | Move VM, BlockSTM 并行执行, AptosBFT 共识 | Phase 7 |

**共同点**: 基于 Move 语言，需要 Move VM 桥接层；可共享基础 Move IR 翻译逻辑。

#### 其他公链

| 链名称 | ChainType | 特性 | 优先级 |
|--------|-----------|------|--------|
| Cardano | `Cardano` | Plutus 智能合约, EUTXO 模型, Ouroboros PoS | Phase 7-10 |
| TON | `TON` | TVM, 异步架构, 无限分片 | Phase 7-10 |
| 科图链 (KOT) | `KOT` | NOS 共识, 国产公链生态 | Phase 7-10 |

---

## 🏗️ 实现路线图

### Phase 7: L3.2 外部链适配器插件

#### 第一批 (优先级高)

**1. EVM 通用适配器** (基础设施)
- [ ] revm 5.0 集成
- [ ] MvccEvmDatabase 桥接层
- [ ] ERC20/ERC721 合约测试
- [ ] Ethereum/BSC 验证

**2. TRON 适配器**
- [ ] TVM 集成
- [ ] 能量/带宽模型映射
- [ ] TRC20 代币支持

**3. Bitcoin 适配器**
- [ ] UTXO → 账户模型映射
- [ ] Bitcoin P2P 网络集成
- [ ] SPV 轻节点支持

**4. Solana 适配器**
- [ ] QUIC gossip 网络集成
- [ ] Solana Runtime 桥接
- [ ] SPL Token 支持

#### 第二批 (EVM 系列扩展)

**5. Polygon 适配器**
- [ ] 基于 EVM 通用适配器扩展
- [ ] Heimdall + Bor 共识集成
- [ ] Plasma Bridge 支持

**6. BNB Chain 适配器**
- [ ] Parlia PoSA 共识
- [ ] 双链架构 (Beacon + Smart Chain)
- [ ] BEP-20 代币标准

**7. Avalanche 适配器**
- [ ] 雪崩共识协议
- [ ] 子网架构支持
- [ ] C-Chain/X-Chain/P-Chain 协同

#### 第三批 (Move 系列)

**8. Sui 适配器**
- [ ] Move VM 桥接层
- [ ] 对象中心模型映射
- [ ] Narwhal & Bullshark 共识

**9. Aptos 适配器**
- [ ] Move VM 复用
- [ ] BlockSTM 并行执行引擎
- [ ] AptosBFT 共识支持

#### 第四批 (其他公链)

**10. Cardano 适配器**
- [ ] Plutus 智能合约解释器
- [ ] EUTXO 模型映射
- [ ] Ouroboros PoS 集成

**11. TON 适配器**
- [ ] TVM 集成
- [ ] 异步架构支持
- [ ] 无限分片桥接

**12. 科图链 (KOT) 适配器**
- [ ] NOS 共识机制集成
- [ ] 技术规格收集
- [ ] 国产公链生态对接

---

### Phase 10: L3.4 多链节点热插拔

#### 节点子模块集成

**1. Bitcoin 生态**
- [ ] Bitcoin Core 子模块
- [ ] Lightning Network 节点
- [ ] UTXO 索引服务

**2. EVM 生态**
- [ ] Geth/Reth 子模块
- [ ] Polygon Validator 节点
- [ ] BNB Chain 节点
- [ ] Avalanche 节点 (三链)

**3. Solana 生态**
- [ ] Solana Validator 子模块
- [ ] RPC 节点支持
- [ ] Gossip 协议集成

**4. TRON 生态**
- [ ] TRON Full Node 集成
- [ ] Witness 节点支持
- [ ] gRPC API 桥接

**5. Move 生态**
- [ ] Sui Full Node 集成
- [ ] Aptos Full Node 集成
- [ ] Move Prover（可选）

**6. 其他公链**
- [ ] Cardano 节点 (cardano-node)
- [ ] TON Validator 节点
- [ ] 科图链 (KOT) 节点

---

## 🔧 技术实现细节

### ChainType 枚举 (已实现)

```rust
pub enum ChainType {
    SuperVM,      // SuperVM 原生 WASM 链
    EVM,          // EVM 兼容链 (通用)
    Bitcoin,      // Bitcoin 及其派生链
    Solana,       // Solana
    TRON,         // TRON
    Polygon,      // Polygon (EVM 兼容 Layer 2)
    BNB,          // BNB Chain (Binance Smart Chain)
    Avalanche,    // Avalanche
    Cardano,      // Cardano
    Sui,          // Sui (Move-based)
    Aptos,        // Aptos (Move-based)
    TON,          // TON (The Open Network)
    KOT,          // 科图链 (NOS/KOT)
    Custom,       // 自定义链
}
```

### AdapterFactory 当前状态

```rust
pub fn create(config: ChainConfig) -> Result<Box<dyn ChainAdapter>> {
    match config.chain_type {
        ChainType::SuperVM => Ok(Box::new(WasmChainAdapter::new(config)?)),
        ChainType::EVM => Ok(Box::new(EvmAdapter::new(config)?)),
        ChainType::TRON => Ok(Box::new(TronAdapter::new(config)?)),
        
        // EVM 系列 (占位，Phase 7 实现)
        ChainType::Polygon | ChainType::BNB | ChainType::Avalanche => {
            bail!("{} adapter not yet implemented", config.chain_type.as_str())
        }
        
        // 非 EVM 系列 (占位)
        ChainType::Bitcoin | ChainType::Solana => bail!("..."),
        
        // Move 系列 (占位)
        ChainType::Sui | ChainType::Aptos => bail!("..."),
        
        // 其他公链 (占位)
        ChainType::Cardano | ChainType::TON | ChainType::KOT => bail!("..."),
        
        ChainType::Custom => bail!("Custom adapter requires explicit implementation"),
    }
}
```

---

## 📊 进度追踪

### 整体进度

| 阶段 | 完成度 | 状态 |
|------|--------|------|
| **L1 层骨架** | 95% | ✅ 接近完成 |
| **L3.2 适配器插件** | 10% | 📋 规划中 |
| **L3.4 节点热插拔** | 10% | 📋 规划中 |

### 链支持进度

| 类别 | 骨架 | 插件 | 节点 |
|------|------|------|------|
| **SuperVM** | ✅ 100% | ✅ 100% | ✅ 100% |
| **EVM 通用** | ✅ 90% | 📋 0% | 📋 0% |
| **TRON** | ✅ 90% | 📋 0% | 📋 0% |
| **Bitcoin** | ✅ 85% | 📋 0% | 📋 0% |
| **Solana** | ✅ 85% | 📋 0% | 📋 0% |
| **Polygon** | ✅ 枚举 | 📋 0% | 📋 0% |
| **BNB** | ✅ 枚举 | 📋 0% | 📋 0% |
| **Avalanche** | ✅ 枚举 | 📋 0% | 📋 0% |
| **Sui** | ✅ 枚举 | 📋 0% | 📋 0% |
| **Aptos** | ✅ 枚举 | 📋 0% | 📋 0% |
| **Cardano** | ✅ 枚举 | 📋 0% | 📋 0% |
| **TON** | ✅ 枚举 | 📋 0% | 📋 0% |
| **KOT** | ✅ 枚举 | 📋 0% | 📋 0% |

---

## 🎯 里程碑

### M1: L1 骨架完成 ✅ (2025-11-14)

- [x] ChainType 枚举扩展至 14 种链
- [x] EVM/TRON/Bitcoin/Solana 骨架实现
- [x] AdapterFactory 工厂模式集成
- [x] 199 个单元测试全部通过
- [x] ROADMAP 文档更新

### M2: EVM 系列插件完成 📋 (Phase 7)

- [ ] revm 5.0 集成
- [ ] Polygon/BNB/Avalanche 适配器
- [ ] ERC20/BEP-20 代币支持
- [ ] 共识层差异化处理

### M3: Move 系列插件完成 📋 (Phase 7)

- [ ] Move VM 桥接层
- [ ] Sui/Aptos 适配器
- [ ] 对象模型映射
- [ ] BlockSTM 集成

### M4: 全链节点热插拔 📋 (Phase 10)

- [ ] 12+ 条链节点子模块集成
- [ ] 统一健康监控
- [ ] 热重载与版本升级
- [ ] 跨链原子事务协议

---

## 📚 相关文档

- `ROADMAP.md`: 完整项目路线图
- `docs/PHASE-D-EVM-ADAPTER-PLAN.md`: EVM 适配器详细设计
- `docs/CROSS-CHAIN-ARCHITECTURE.md`: 跨链架构文档
- `src/vm-runtime/src/adapter/chain_adapter.rs`: ChainAdapter 接口定义
- `src/vm-runtime/src/adapter/*.rs`: 各链适配器实现

---

## 🤝 贡献指南

### 添加新链支持

1. **扩展 ChainType 枚举**
   ```rust
   // src/vm-runtime/src/adapter/chain_adapter.rs
   pub enum ChainType {
       // ...
       YourChain,  // 新增链
   }
   ```

2. **实现骨架适配器** (L1 层)
   ```rust
   // src/vm-runtime/src/adapter/your_chain_adapter.rs
   pub struct YourChainAdapter { /* ... */ }
   impl ChainAdapter for YourChainAdapter { /* ... */ }
   ```

3. **更新工厂模式**
   ```rust
   ChainType::YourChain => {
       Ok(Box::new(YourChainAdapter::new(config)?))
   }
   ```

4. **实现真实插件** (L3.2 层)
   - 集成原生运行时 (VM/执行引擎)
   - IR 翻译层
   - 共识协议桥接

5. **节点热插拔** (L3.4 层)
   - 节点生命周期管理
   - 数据同步与健康监控

---

## ⚠️ 注意事项

1. **架构边界**
   - L1 层仅包含接口定义与最小骨架
   - L3.2 层实现真实协议逻辑
   - L3.4 层管理节点生命周期

2. **性能隔离**
   - 外部链插件不影响 L0 核心性能
   - FastPath 保持 28.57M TPS 不变

3. **测试覆盖**
   - 每个适配器需提供冒烟测试
   - CI 使用 feature 门控避免外部依赖

4. **文档同步**
   - 新增链需同步更新 ROADMAP.md
   - 重要特性需补充设计文档

---

**最后更新**: 2025-11-14  
**维护者**: KING XU (CHINA)  
**项目**: SuperVM - 潘多拉计划
