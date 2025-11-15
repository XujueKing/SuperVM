# SuperVM 多链统一架构愿景

**版本**: v0.2 (2025-11-09 更新)  
**目标**: 
1. 让 SuperVM 在部署后以"伪装兼容节点"的形式同时接入 BTC / Ethereum / Solana / 其他链的网络协议, 对外看似原生节点, 对内使用统一高性能 + 隐私增强内核与存储。
2. 提供去中心化 Web 存储与寻址层，让传统网站通过热插拔硬盘接入 SuperVM，用户通过 SuperVM Web3 浏览器访问区块链背后的存储空间，实现真正的去中心化互联网。

---

> 实操入口：如果你想直接验证跨链执行最小路径，请参考《[Cross-Chain Executor 使用指南](./cross-executor-usage.md)》。

## 1. 愿景概述

SuperVM = 一个可插拔、多协议兼容、隐私强化、统一数据结构的主网运行环境 + 去中心化 Web 基础设施。

外部视角:

- 其他公链的节点认为 SuperVM 子模块是它们的正常同伴(peer)。

- 它按协议握手、同步区块、广播交易、返回查询结果。

内部视角:

- 接收到的原生区块/交易被转换为统一 IR(Intermediate Representation)。

- 在统一 IR 上执行高性能执行引擎 + 隐私转换 (承诺/Nullifier/加密索引)。

- 存储两份: `raw_original` (原链格式) + `privacy_extended` (SuperVM 新格式)。

用户视角:

- 使用原生协议客户端 (MetaMask / Bitcoin Core / Solana CLI) 仍可与 SuperVM 通信, 资产不丢失。

- 切换到 SuperVM 原生协议获得: 更低 Gas, 更高 TPS, 增强隐私保护, 跨链资产统一引用。

迁移路径:

- 初期: 透明代理模式 (SuperVM 仅做镜像 & 提供额外私有接口)。

- 中期: Encouraged Mode (用户在 SuperVM 原生协议内进行同样操作, 并有性能/费用优势)。

- 后期: Native Dominant (大部分用户使用 SuperVM 原生协议, 老协议保留兼容层)。

---

## 2. 核心原则

| 原则 | 说明 |
|------|------|
| 不破坏兼容性 | 原链可随时回退, 数据保留原始结构副本 |
| 插件化适配 | 每条链一个 Adapter 模块, 可热插拔与升级 |
| 统一抽象层 | 交易/区块/状态/资产均抽象为统一 IR, 便于调度与索引 |
| 隐私优先 | 引入承诺树、Nullifier 集、加密标签、可选零知识证明缓存 |
| 可回滚与影子状态 | 面对多链 reorg, 保持影子状态回滚与最终性缓冲区 |
| 性能调度 | 执行分级: 高频路径 (内核内存态) / 低频查询 (延迟索引) |
| 资产同构 | 不同链资产映射为统一 ID (ChainID + 原生AssetRef → Hash) |
| 渐进式采用 | 不强制迁移, 通过成本优化进行吸引 |

---

## 3. 模块分层

```

+--------------------------------------------------------------+
|                      SuperVM P2P Orchestrator               |
|  - Multi-Chain Peer Manager  - Identity Masquerade          |
|  - Protocol Dispatch         - Reorg Event Bus              |
+--------------------+--------------------+--------------------+
|   BTC Adapter      |   EVM Adapter      |  Solana Adapter    | ...
|  - P2P(P2PFront)   |  - DevP2P/ETHWire  |  - QUIC/Gossip     |     
|  - BlockTranslator |  - TxDecoder       |  - SlotAssembler   |     
|  - SPV/Headersync  |  - Beacon/Exec     |  - FinalityTracker |     
+--------------------+--------------------+--------------------+
|               Unified IR & Privacy Pipeline                 |
|  - TxIR / BlockIR / State Snapshot                          |
|  - Commitment Builder / Nullifier Manager                   |
|  - Aggregated Index (Merkle / Sparse / Accumulators)        |
+--------------------------------------------------------------+
|                 Execution & Scheduling Core                 |
|  - High-Perf WASM/Native VM (SuperVM Runtime)               |
|  - Cross-Chain Asset Router                                 |
|  - Batch Verification (Proof Pools)                         |
+--------------------------------------------------------------+
|                     Storage Namespaces                      |
|  raw_original | unified_ir | privacy_extended | cache_index  |
+--------------------------------------------------------------+
|                     Observability & Control                 |
|  Metrics | Tracing | Auditing | Governance                  |
+--------------------------------------------------------------+

```

---

## 4. Adapter 接口定义 (草案)

```rust
trait ChainAdapter {
    fn chain_id(&self) -> ChainId;                // 唯一标识
    fn protocol_caps(&self) -> ProtocolCaps;      // 支持的功能 (TxRelay/Blocks/Events)

    // 网络接入
    fn start_p2p(&mut self, cfg: P2PConfig) -> Result<()>;
    fn poll_network(&mut self) -> Vec<RawInboundEvent>; // headers / tx / blocks

    // 数据翻译
    fn translate_block(&self, raw: RawBlock) -> BlockIR; // 归一化
    fn translate_tx(&self, raw: RawTx) -> TxIR;

    // 最终性 / reorg 处理
    fn finality_window(&self) -> FinalityPolicy; // BTC: 6 conf, ETH: epochs, Solana: slots
    fn detect_reorg(&self, chain_state: &ChainState) -> Option<ReorgEvent>;

    // 状态访问
    fn state_snapshot(&self) -> StateIR;         // Address balances / UTXO sets / Accounts

    // 资源与健康度
    fn metrics(&self) -> AdapterMetrics;
}

```

### TxIR 统一字段示例

```json
{
  "tx_hash": "0x...",
  "chain_id": "EVM:1",             // or BTC:0 / SOL:101
  "nonce_or_seq": 12345,
  "from": "0x..." ,
  "to": "0x..." ,                  // BTC multi-output → normalized as outputs[]
  "value_list": ["100000000"],      // BTC 多输出时多值
  "fee": {
    "gas_price_or_rate": "5000000000",
    "limit_or_weight": 21000
  },
  "payload": {
    "kind": "evm_call",             // evm_deploy / btc_transfer / sol_invoke
    "raw": "0x...."                 // 原始输入数据
  },
  "timestamp": 1731180000,
  "privacy_tags": {
    "commitment": "0x...",          // 可选
    "nullifiers": ["0x..."]
  }
}

```

---

## 5. 数据存储策略

| Namespace | 内容 | 写入来源 | 回滚需求 | 压缩/裁剪 |
|-----------|------|----------|----------|-----------|
| raw_original | 原始区块/交易二进制 | Adapter 网络层 | 是 (跟随原链) | 中等 (区块剪枝) |
| unified_ir | 归一化 IR 表 | 翻译器 | 是 (影子状态) | 可行 (列式存储) |
| privacy_extended | 承诺、Nullifier、加密索引 | 隐私流水线 | 是 (随 IR 回滚) | 长期保留 |
| cache_index | 二级加速索引 (Bloom/Sparse) | 查询优化器 | 可再生 | 可定期重建 |

回滚策略:

- 维护 `ShadowState(height, root_hash, validity_window)`

- 只在原链确认度低 (< finality threshold) 时保留多分支

- 后台任务清理旧分支并合并承诺树

---

## 6. 隐私转换流水线

1. 接收 RawTx → TxIR
2. 生成 Commitment (value + salt) 与 Nullifier (spent 标记)
3. 构建 Merkle / Sparse Merkle Tree → 更新根
4. 预生成零知识证明缓存 (可选提前计算批量证明)  
5. 存储扩展记录 → 等待用户通过 SuperVM 原生协议查询

批量加速点:

- 将多条相同类型交易聚合在一个证明内 (批量转账批处理)

- 提前维护 `pending_proof_pool` → 定时或大小阈值触发验证

---

## 7. 跨链资产建模与路由

统一资产 ID = `hash(OriginChainID || NativeAssetRef || EncodingVersion)`

示例:

- BTC UTXO: `BTC:txhash:vout_index`

- ETH ERC20: `ETH:contract:tokenId` 或 `ETH:contract` (fungible)

- Solana SPL: `SOL:mint_address`

路由流程:

```

User (SuperVM Protocol) → Asset Router → Adapter(chain_id) → 原链 RPC/P2P → Result → 转换/签名 → 返回

```

跨链价值操作:

- 读取余额：使用 Adapter 的 state_snapshot 统一映射

- 发起转账：构造原生链交易结构 → 签名（用户原生私钥或 SuperVM 代理密钥）→ 广播

- 监听确认：FinalityTracker 推送事件 → 触发隐私扩展写入

---

## 8. 最终性与 Reorg 处理

| 链 | 最终性参考 | SuperVM 策略 |
|----|-----------|---------------|
| BTC | ~6 确认块 | 缓冲窗口=6, 超出后写入稳定层 |
| Ethereum PoS | 2 epochs (~12.8 min) | 进入稳定后合并 shadow state |
| Solana | Slot finality (~1-2s, 乐观) | 维护 slot DAG 与乐观确认 → 回滚冲突 slot |

Reorg 事件:

- Adapter 提交 `ReorgEvent{from_height, to_height, orphan_blocks}`

- 执行 `Rollback(unified_ir, privacy_extended)` → 重新应用新分支数据

---

## 9. 安全与威胁模型 (初稿)

| 威胁 | 描述 | 缓解 |
|------|------|------|
| 伪装节点被识别 | 适配器实现不完全符合协议细节, 导致断连 | 严格兼容测试 + 协议差异模拟器 |
| 重放攻击 | 跨协议交易被重复执行 | 引入 nonce / nullifier 集合全局查重 |
| Reorg DoS | 高频回滚消耗资源 | ShadowState 层限速 + 分支深度阈值 |
| 桥欺骗 | 虚假跨链资产映射 | 资产 ID + 多来源验证 + 轻客户端校验根 |
| 信息泄露 | 原链明文与 SuperVM 隐私映射关联被反推出 | 引入盐、加密标签、可选混淆层 |
| 预编译滥用 | BN254/多曲线点输入异常消耗 Gas | 结构化校验 + 点域限制 + 黑名单策略 |

---

## 9.1 决策确认与路线细化（2025-11-09）

### 首发组合

- **EVM + BTC** 作为 M1-M2 阶段目标，优先打通最大生态与最成熟协议。

### 路线优先级

- **先 RPC 伪装**：所有链先实现 JSON-RPC/REST/gRPC 兼容，优先满足钱包/应用/工具链对接，P2P 网络逐步补全。

### 存储分层与四层神经网络映射

- **L1-L2**：全量原始区块/交易存储（raw_original），保证可审计、可回放、可归档。

- **L3-L4**：SuperVM 原生协议流转与高效索引（unified_ir、privacy_extended、cache_index），支持高性能调度与隐私流水线。

- **流转机制**：L4-L3 层可直接用 SuperVM 原生协议进行资产/状态流转，L1-L2 层负责历史全量与归档。

### 轻客户端集成

- **立即接入**：BTC（SPV）、ETH（信标 sync committee）、Solana（slot proofs）等轻客户端机制，确保跨链资产与最终性安全。

### 资产映射与命名规则

- **统一资产ID编码方案**：
  - 结构：`AssetID = hash(OriginChainID || NativeAssetRef || EncodingVersion)`
  - 例：
    - BTC UTXO: `BTC:txhash:vout_index`
    - ETH ERC20: `ETH:contract:tokenId` 或 `ETH:contract` (fungible)
    - Solana SPL: `SOL:mint_address`
    - SuperVM 原生资产：`SVM:asset_name[:subtype]`
  - **命名映射**：
    - SuperVM 网络内可用友好别名（如 USDT(web3)、BTC(web3)），对外仍可还原原链格式。
    - 资产注册表支持多语言/多标准别名，便于未来主导标准。
  - **标准制定展望**：
    - 资产ID与命名规范可开放为 EIP/SIP/BIP 提案，推动行业采纳。

---

## 10. 渐进式里程碑 (建议)

| 阶段 | 目标 | 产出 |
|------|------|------|
| M1 | 单 EVM Adapter 注入 | 同步区块 + TxIR 转换 + 基本存储 |
| M2 | BTC SPV 头同步 + UTXO 映射 | 双链 IR 合并 + 资产路由 V1 |
| M3 | 隐私转换流水线上线 | Commitment/Nullifier/Proof 缓存 |
| M4 | 批量验证 + Gas 优化 | Batch verify 接口 + 性能指标 |
| M5 | Solana Adapter (QUIC gossip) | Slot 转换 + 乐观最终性集成 |
| M6 | 统一资产跨链转账 | Router + 资产映射持久化 |
| M7 | 预编译/内核加速实验 | verify_groth16 native 实现 |
| M8 | 安全与审计 | 威胁模型定稿 + 测试集 |
| M9 | 主网 Beta | 多链接入测压 + 文档生态 |

---

## 11. Web3 原生存储与寻址层

### 11.1 愿景：去中心化 Web 热插拔存储

**目标**：让传统互联网网站通过热插拔硬盘或开放磁盘空间接入 SuperVM，用户通过 SuperVM Web3 浏览器（非 www 协议）直接访问区块链背后的存储空间，实现去中心化网站托管与访问。

### 11.2 核心组件

#### 存储层：热插拔与分布式存储池

- **物理接入**：
  - 节点可挂载外置硬盘、NAS、云存储作为存储资源池。
  - 支持热插拔：新设备接入后自动注册到 SuperVM 存储网络，分配存储配额与寻址空间。
  - 本地磁盘开放空间：节点运营者可设定磁盘空间上限（如 100GB），SuperVM 自动分片管理。

- **数据分片与冗余**：
  - 网站内容（HTML/CSS/JS/图片/视频）按内容哈希分片存储（类似 IPFS）。
  - 支持可配置冗余因子（如 3 副本），分散到不同地理位置节点。
  - 使用 erasure coding（如 Reed-Solomon）优化存储效率与可恢复性。

- **存储证明与激励**：
  - 定期挑战机制（Proof of Storage）：验证节点是否真实保存数据。
  - 奖励机制：提供存储空间的节点获得原生代币奖励。
  - 惩罚机制：未通过挑战的节点被剔除并扣除质押。

#### 寻址层：去中心化域名与路由系统

**SuperVM Name Service (SNS)**：类似 DNS + ENS + IPFS 的混合体

- **命名规范**：
  - 格式：`<name>.svm` 或 `<name>.web3`
  - 示例：`myapp.svm`、`decentralized-blog.web3`
  - 支持子域名：`api.myapp.svm`、`cdn.myapp.svm`

- **注册与解析**：
  - 通过 SuperVM 智能合约注册域名，链上记录所有权与解析记录。
  - 解析记录包含：
    - `content_hash`：网站根目录的 Merkle root 或 IPFS CID
    - `storage_nodes`：分布式存储节点地址列表
    - `routing_policy`：负载均衡策略（就近访问、随机、权重）
    - `version`：支持多版本部署与回滚

- **路由机制**：
  - 用户输入 `myapp.svm` → SuperVM 浏览器查询链上注册表 → 获取 content_hash → 从存储节点拉取内容 → 渲染展示。
  - 智能路由：优先从地理位置最近、延迟最低的节点拉取。
  - 缓存策略：浏览器本地缓存 + 边缘节点 CDN 缓存（SuperVM 节点可选启用 CDN 模式）。

#### 浏览器层：SuperVM Web3 Browser

**非 www 协议栈**：

- **协议**：`svm://` 或 `web3://`
  - 示例：`svm://myapp.svm`、`web3://0x1234...abcd`（直接用哈希访问）

- **核心功能**：
  - **去中心化解析**：内置 SNS 客户端，直接查询链上记录与存储节点。
  - **内容验证**：下载内容后验证哈希，防止恶意节点篡改。
  - **隐私保护**：支持零知识证明身份登录、加密通信（TLS over SuperVM）。
  - **资产集成**：浏览器内置 Web3 钱包，网站可直接调用 SuperVM 资产接口。
  - **WASM/WebGPU 加速**：支持高性能 dApp 运行（游戏、3D、AI 推理）。

- **兼容性桥接**：
  - 支持传统 IPFS/Arweave 内容（通过网关转换）。
  - 可选启用 www 网关：传统浏览器通过 `https://gateway.supervm.io/svm/myapp.svm` 访问（中心化入口，降级体验）。

### 11.3 网站部署流程

**开发者视角**：

1. **打包网站**：
   ```bash
   svm-cli build ./my-website
   # 输出: content_hash = 0xabc123..., manifest.json
   ```

2. **上传到存储网络**：
   ```bash
   svm-cli deploy --name myapp.svm --content ./my-website
   # SuperVM 自动分片上传到存储节点
   # 返回: 交易哈希 + 域名注册确认
   ```

3. **域名注册/更新**：
   ```bash
   svm-cli register myapp.svm --content-hash 0xabc123... --nodes 5 --redundancy 3
   # 链上写入注册记录, 指定 5 个存储节点, 3 副本
   ```

4. **访问与测试**：
   ```bash
   svm-browser svm://myapp.svm
   # 或通过 Web3 浏览器直接输入 myapp.svm
   ```

**用户视角**：

1. 安装 SuperVM Web3 Browser（桌面版/移动版/浏览器插件）。
2. 输入地址：`myapp.svm`
3. 浏览器：
   - 查询链上 SNS 合约 → 获取 `content_hash` + 存储节点列表
   - 从存储节点下载内容（自动选择最快节点）
   - 验证哈希 → 渲染网站
4. 交互：网站可调用 SuperVM 钱包、资产接口、隐私计算等原生能力。

### 11.4 技术栈与协议

| 层级 | 技术选型 | 说明 |
|------|----------|------|
| 寻址 | SNS 智能合约 (Solidity/WASM) | 链上域名注册表 |
| 存储 | 分布式哈希表 (DHT) + Merkle DAG | 类似 IPFS/Filecoin 混合 |
| 传输 | QUIC/HTTP3 + libp2p | 低延迟、多路复用、NAT 穿透 |
| 验证 | Merkle proof + 内容哈希校验 | 防篡改 |
| 激励 | Proof of Storage + 代币奖励 | 经济驱动存储提供 |
| 浏览器 | Electron/Tauri + Chromium/WebKit | 跨平台 Web3 浏览器 |
| 开发者工具 | CLI + SDK (JS/Rust/Python) | 打包、部署、管理 |

### 11.5 与现有方案对比

| 方案 | 中心化程度 | 性能 | 隐私 | 成本 | SuperVM 优势 |
|------|-----------|------|------|------|--------------|
| 传统 Web (www) | 高（依赖 DNS/服务器） | 高 | 低 | 中 | 去中心化、抗审查 |
| IPFS | 中（依赖网关/pinning） | 中 | 中 | 低 | 激励层 + 链上寻址 |
| Arweave | 低 | 中 | 中 | 高（一次性付费） | 动态更新 + 跨链集成 |
| ENS + IPFS | 中 | 中 | 中 | 中 | 统一协议 + 隐私增强 |
| **SuperVM** | **低** | **高（L3-L4 加速）** | **高（隐私流水线）** | **低（存储激励）** | 多链资产 + 原生计算 + 热插拔 |

### 11.6 安全与治理

- **内容审核**：
  - 可选社区治理：恶意内容（钓鱼/非法）可通过 DAO 投票下架。
  - 节点自主选择：存储节点可设置内容过滤策略（拒绝存储特定类型）。

- **版本控制与回滚**：
  - 每次更新生成新 `content_hash`，链上保留历史版本。
  - 用户可指定访问特定版本：`myapp.svm@v1.2.3` 或 `myapp.svm@0xabc123...`

- **抗 DDoS**：
  - 存储节点分布式，单点攻击无效。
  - 浏览器自动切换节点，保证可用性。

### 11.7 里程碑扩展

| 阶段 | 目标 | 产出 |
|------|------|------|
| M10 | SNS 智能合约 + 域名注册 | 链上域名系统原型 |
| M11 | 存储层 MVP（单节点热插拔） | 外置硬盘接入 + 分片存储 |
| M12 | SuperVM Web3 Browser Alpha | 支持 svm:// 协议 + 基本渲染 |
| M13 | 分布式存储网络（多节点） | DHT + 副本 + Proof of Storage |
| M14 | 开发者工具链 (CLI/SDK) | 一键部署 + 本地测试 |
| M15 | CDN 模式 + 就近路由 | 边缘节点加速 + 地理路由 |
| M16 | 内容市场与激励完善 | 存储奖励 + 流量分成 |

---

## 12. 热插拔子模块:原链节点聚合架构

### 12.1 核心理念

**SuperVM 不是"跨链桥",而是"多链节点聚合器"**:

- 每个子模块**就是原链的完整节点实现** (Bitcoin Core / Geth / Solana Validator)

- 子模块**遵守原链共识协议**,参与原链网络(挖矿/验证/同步)

- SuperVM 统一路由层**双重映射**资产状态:
  - 子模块维护原链真实状态 (UTXO / State Trie / Account DB)
  - 统一账本映射镜像状态 (Unified IR 格式)

### 12.2 架构示意

```

┌─────────────────────────────────────────────────────────────┐
│           SuperVM 主网节点 (物理设备)                          │
├─────────────────────────────────────────────────────────────┤
│  统一路由层 (Phase 5 三通道路由)                               │
│  ├─ 快速通道: SuperVM 原生交易 (映射状态快速确认)               │
│  ├─ 共识通道: 转发到子模块 → 原链共识 → 回写映射               │
│  └─ 隐私通道: RingCT 混淆 (跨子模块隐私交易)                   │
├─────────────────────────────────────────────────────────────┤
│  🔌 热插拔子模块层 (每个都是完整原链节点!)                      │
│                                                              │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐      │
│  │ Bitcoin Core │  │ Geth (EVM)   │  │ Solana Node  │      │
│  │  子模块       │  │  子模块       │  │  子模块       │      │
│  ├──────────────┤  ├──────────────┤  ├──────────────┤      │
│  │• 完整 UTXO DB│  │• State Trie  │  │• Account DB  │      │
│  │• PoW 挖矿引擎│  │• PoS 验证    │  │• PoH 时钟    │      │
│  │• Mempool    │  │• EVM 执行    │  │• SVM 运行时  │      │
│  │• P2P 协议    │  │• DevP2P     │  │• Gossip 协议 │      │
│  └──────────────┘  └──────────────┘  └──────────────┘      │
│         ↕ 真实网络    ↕ 真实网络       ↕ 真实网络            │
│  Bitcoin Network  Ethereum Network  Solana Network         │
└─────────────────────────────────────────────────────────────┘
          ↕ 双重映射同步
┌─────────────────────────────────────────────────────────────┐
│  SuperVM 统一账本 (Unified IR 存储)                           │
│  • BTC(supervm) ← 实时映射自 Bitcoin 子模块 UTXO 状态        │
│  • ETH(supervm) ← 实时映射自 Geth 子模块 State Trie         │
│  • USDT(supervm) ← 映射自 Geth 子模块中的 USDT 合约状态      │
└─────────────────────────────────────────────────────────────┘

```

### 12.3 关键实现要点

#### 12.3.1 子模块基于原链源码

```rust
// Bitcoin 子模块: 基于 bitcoin-core 源码适配
pub struct BitcoinSubmodule {
    // 完整的比特币节点组件
    blockchain: BlockchainDB,      // leveldb 存储完整区块链
    chainstate: ChainstateDB,      // UTXO 集合
    mempool: Mempool,              // 未确认交易池
    consensus: ConsensusEngine,    // PoW 验证 + 难度调整
    p2p_node: P2PNode,             // 连接比特币网络
    miner: Option<Miner>,          // 可选挖矿模块
    
    // SuperVM 集成接口
    state_mirror: Arc<Mutex<UnifiedStateMirror>>,
}

impl ChainAdapter for BitcoinSubmodule {
    fn start(&self) -> Result<()> {
        // 1. 连接到比特币网络
        self.p2p_node.connect_to_peers()?;
        
        // 2. 同步区块链
        self.p2p_node.sync_blockchain()?;
        
        // 3. 启动挖矿 (如果配置)
        if let Some(miner) = &self.miner {
            miner.start_mining()?;
        }
        
        // 4. 启动状态镜像同步
        self.start_state_mirroring()?;
        
        Ok(())
    }
    
    fn process_native_transaction(&self, tx: BitcoinTx) -> Result<()> {
        // 1. 验证签名 (比特币原生验证)
        self.consensus.verify_transaction(&tx)?;
        
        // 2. 加入 mempool
        self.mempool.add_transaction(tx.clone())?;
        
        // 3. 广播到比特币网络
        self.p2p_node.broadcast_transaction(&tx)?;
        
        // 4. 同步映射到 SuperVM 统一账本
        let tx_ir = self.convert_to_ir(&tx);
        self.state_mirror.lock().unwrap().apply_transaction(tx_ir)?;
        
        Ok(())
    }
    
    fn mine_block(&self) -> Result<Block> {
        // 真正的比特币 PoW 挖矿!
        let miner = self.miner.as_ref().ok_or("Miner not enabled")?;
        
        // 1. 从 mempool 选择交易
        let txs = self.mempool.select_transactions()?;
        
        // 2. 构造 Coinbase 交易 (挖矿奖励归节点地址)
        let coinbase = self.create_coinbase_tx(self.mining_address)?;
        
        // 3. PoW 计算 (寻找 nonce)
        let block = miner.mine(txs, coinbase)?;
        
        // 4. 广播到比特币网络
        self.p2p_node.broadcast_block(&block)?;
        
        // 5. 应用到本地链状态
        self.blockchain.add_block(&block)?;
        self.chainstate.update_utxos(&block)?;
        
        // 6. 镜像到 SuperVM 统一账本
        let block_ir = self.convert_block_to_ir(&block);
        self.state_mirror.lock().unwrap().apply_block(block_ir)?;
        
        Ok(block)
    }
}

```

#### 12.3.2 Geth (Ethereum) 子模块

```rust
// Geth 子模块: 基于 go-ethereum 源码 FFI 封装
pub struct GethSubmodule {
    // Geth 核心组件 (通过 CGO FFI 调用)
    eth_backend: EthereumBackend,  // Geth 后端
    state_db: StateDB,             // 世界状态树
    txpool: TxPool,                // 交易池
    consensus: BeaconConsensus,    // PoS 共识 (Merge 后)
    validator: Option<Validator>,  // 验证者 (需 32 ETH 质押)
    
    // SuperVM 集成
    state_mirror: Arc<Mutex<UnifiedStateMirror>>,
}

impl ChainAdapter for GethSubmodule {
    fn execute_smart_contract(&self, tx: EthTx) -> Result<Receipt> {
        // 1. EVM 执行 (使用真实 Geth EVM)
        let receipt = self.eth_backend.execute_transaction(&tx)?;
        
        // 2. 更新世界状态
        self.state_db.commit(receipt.state_root)?;
        
        // 3. 广播到以太坊网络
        self.eth_backend.broadcast_transaction(&tx)?;
        
        // 4. 镜像到 SuperVM
        let tx_ir = self.convert_tx_to_ir(&tx, &receipt);
        self.state_mirror.lock().unwrap().apply_transaction(tx_ir)?;
        
        Ok(receipt)
    }
    
    fn validate_block(&self, block: EthBlock) -> Result<()> {
        // PoS 验证者职责
        let validator = self.validator.as_ref().ok_or("Not a validator")?;
        
        // 1. 验证区块
        validator.verify_block(&block)?;
        
        // 2. 参与 Attestation
        let attestation = validator.create_attestation(&block)?;
        self.eth_backend.submit_attestation(attestation)?;
        
        // 3. 镜像到 SuperVM
        let block_ir = self.convert_block_to_ir(&block);
        self.state_mirror.lock().unwrap().apply_block(block_ir)?;
        
        Ok(())
    }
}

```

### 12.4 双重映射机制

#### 12.4.1 实时状态同步

```rust
pub struct UnifiedStateMirror {
    // SuperVM 统一账本
    accounts: HashMap<Address, Account>,
    assets: HashMap<AssetID, AssetState>,
    
    // 原链状态索引
    btc_utxo_index: HashMap<OutPoint, UnifiedAssetRef>,
    eth_account_index: HashMap<EthAddress, UnifiedAddress>,
}

impl UnifiedStateMirror {
    // Bitcoin 区块确认后触发
    pub fn sync_bitcoin_block(&mut self, block: &BitcoinBlock) {
        for tx in &block.transactions {
            // 处理 UTXO 消耗
            for input in &tx.inputs {
                if let Some(asset_ref) = self.btc_utxo_index.remove(&input.previous_output) {
                    // 从统一账本扣除
                    self.deduct_asset(asset_ref);
                }
            }
            
            // 处理 UTXO 创建
            for (vout, output) in tx.outputs.iter().enumerate() {
                let outpoint = OutPoint::new(tx.txid(), vout as u32);
                let asset_id = AssetID::btc(); // BTC(supervm)
                let amount = output.value;
                
                // 添加到统一账本
                let asset_ref = self.add_asset(asset_id, amount, output.script_pubkey);
                self.btc_utxo_index.insert(outpoint, asset_ref);
            }
        }
    }
    
    // Ethereum 状态变更后触发
    pub fn sync_ethereum_state(&mut self, receipt: &Receipt) {
        for log in &receipt.logs {
            // 监听 ERC20 Transfer 事件 (例如 USDT)
            if log.topics[0] == ERC20_TRANSFER_TOPIC {
                let from = Address::from(log.topics[1]);
                let to = Address::from(log.topics[2]);
                let amount = U256::from_big_endian(&log.data);
                
                // 映射到 USDT(supervm)
                let asset_id = AssetID::usdt();
                self.transfer_asset(from, to, asset_id, amount);
            }
        }
    }
}

```

#### 12.4.2 用户查询路由

```rust
pub struct UnifiedRouter {
    submodules: HashMap<ChainID, Box<dyn ChainAdapter>>,
    state_mirror: Arc<Mutex<UnifiedStateMirror>>,
}

impl UnifiedRouter {
    // 用户通过 SuperVM RPC 查询余额
    pub fn get_balance(&self, address: Address, asset: AssetID) -> Result<U256> {
        // 快速通道: 从映射状态读取
        let mirror = self.state_mirror.lock().unwrap();
        if let Some(balance) = mirror.get_asset_balance(address, asset) {
            return Ok(balance);
        }
        
        // 回退到子模块查询
        match asset.chain_id() {
            ChainID::Bitcoin => {
                let btc = self.submodules.get(&ChainID::Bitcoin).unwrap();
                btc.query_native_balance(address)
            }
            ChainID::Ethereum => {
                let eth = self.submodules.get(&ChainID::Ethereum).unwrap();
                eth.query_native_balance(address)
            }
            _ => Err("Unsupported chain")
        }
    }
    
    // 用户发起交易
    pub fn submit_transaction(&self, tx: UnifiedTx) -> Result<TxHash> {
        match tx.asset.chain_id() {
            ChainID::Bitcoin => {
                // 转发到 Bitcoin 子模块
                let btc = self.submodules.get(&ChainID::Bitcoin).unwrap();
                let native_tx = self.convert_to_bitcoin_tx(tx)?;
                btc.process_native_transaction(native_tx)
            }
            ChainID::Ethereum => {
                // 转发到 Geth 子模块
                let eth = self.submodules.get(&ChainID::Ethereum).unwrap();
                let native_tx = self.convert_to_eth_tx(tx)?;
                eth.process_native_transaction(native_tx)
            }
            _ => Err("Unsupported chain")
        }
    }
}

```

### 12.5 挖矿收益分配

**你的节点能挖到的真实收益**:

| 子模块 | 挖矿类型 | 收益归属 | 映射到 SuperVM |
|--------|---------|---------|----------------|
| Bitcoin Core | PoW 挖矿 | 节点 BTC 地址 | ✅ 自动映射为 BTC(supervm) |
| Geth | PoS 验证 | 验证者地址 (需 32 ETH) | ✅ 自动映射为 ETH(supervm) |
| Solana | PoH + PoS | 验证者账户 | ✅ 自动映射为 SOL(supervm) |
| SuperVM 原生 | 统一共识 | 节点 SVM 地址 | ✅ SuperVM 原生代币 SVM |

**四重收益来源**:
1. **原链挖矿奖励** (BTC 区块奖励 + 交易费)
2. **原链验证奖励** (ETH Staking 收益)
3. **SuperVM 路由手续费** (用户使用快速通道支付 SVM)
4. **跨链桥手续费** (子模块间资产转移收费)

### 12.6 安全隔离

#### 12.6.1 子模块沙箱

```rust
// 每个子模块在独立进程中运行
pub struct SubmoduleSandbox {
    process: Child,              // 子进程
    ipc_channel: IpcChannel,     // IPC 通信
    resource_limits: ResourceLimits, // CPU/内存限制
}

impl SubmoduleSandbox {
    pub fn start_bitcoin_module(&self) -> Result<()> {
        // 启动隔离的 Bitcoin Core 进程
        let process = Command::new("bitcoin-core-wrapper")
            .arg("--datadir=/var/supervm/bitcoin")
            .arg("--ipc-socket=/var/supervm/bitcoin.sock")
            .spawn()?;
        
        // 限制资源使用
        self.resource_limits.set_cpu_quota(0.5)?; // 最多 50% CPU
        self.resource_limits.set_memory_limit(4_000_000_000)?; // 4GB RAM
        
        Ok(())
    }
}

```

#### 12.6.2 故障隔离

- 子模块崩溃不影响其他链

- 统一路由层自动重启失败模块

- 映射状态保留在 SuperVM,子模块可重新同步

### 12.7 部署与运维

#### 12.7.1 节点启动流程

```bash

# 初始化 SuperVM 主网节点

supervm init --config /etc/supervm/config.toml

# 启用子模块 (热插拔)

supervm module enable bitcoin --mining-address=bc1q...
supervm module enable ethereum --validator-key=/keys/eth_validator.json
supervm module enable solana --identity=/keys/solana_identity.json

# 启动统一路由

supervm start

```

#### 12.7.2 配置示例

```toml
[node]
network = "mainnet"
data_dir = "/var/supervm"

[modules.bitcoin]
enabled = true
mode = "full-node"  # full-node | light-client
mining = true
mining_threads = 4
mining_address = "bc1qxy2kgdygjrsqtzq2n0yrf2493p83kkfjhx0wlh"

[modules.ethereum]
enabled = true
mode = "validator"  # full-node | validator | light-client
consensus = "pos"
validator_key = "/keys/eth_validator.json"
execution_endpoint = "http://localhost:8551"  # Engine API

[modules.solana]
enabled = false  # 可选模块

[router]
fast_lane_enabled = true  # SuperVM 快速通道
consensus_lane_timeout = "30s"  # 原链共识超时
privacy_lane_ringct = true  # 隐私通道

[mirror]
sync_interval = "100ms"  # 状态镜像同步频率
storage_backend = "rocksdb"  # unified IR 存储

```

### 12.8 与现有设计的整合

| 现有模块 | 角色变化 | 整合方式 |
|---------|---------|---------|
| Phase 5 三通道路由 | ✅ 直接复用 | 共识通道 = 转发到子模块 |
| Phase 7 EVM Adapter | ✅ 升级为子模块 | EVM Adapter → Geth 子模块 |
| Phase 4 MVCC 执行 | ✅ 用于快速通道 | 映射状态的高性能执行 |
| Phase 2.2 ZK 隐私 | ✅ 用于隐私通道 | RingCT 跨子模块混币 |
| Phase 6 四层网络 | ✅ 存储层复用 | L1-L2 存储子模块原始数据 |

### 12.9 技术挑战

| 挑战 | 解决方案 |
|------|---------|
| 子模块资源消耗大 | 分级部署: 全节点/轻客户端/仅镜像 |
| 状态同步延迟 | 乐观映射 + 延迟确认 (6 区块后最终确认) |
| 跨模块事务一致性 | 两阶段提交 + 回滚补偿 |
| 原链协议升级 | 子模块独立升级,不影响统一层 |
| 法律合规风险 | 开源协议声明 + 节点运营者自主选择启用模块 |

### 12.10 运行模式与首批支持链 (已决策)

#### 12.10.1 运行模式定义

| 模式 | 描述 | 典型用途 | 占用 | 数据来源 |
|------|------|----------|------|----------|
| FullNode | 完整账本 + 共识 + 执行 | BTC 挖矿 / ETH 验证 / 索引构建 | 高 | 本地全量 | 
| LightClient | 仅区块头 + 按需证明 | 低资源节点/移动部署 | 低 | 原链远程 + 轻证明 |
| ComputeOnly | 只执行统一 IR / 不保留原链全账本 | 高并发快速通道 | 中 | 来自主模块镜像层 | 
| StorageProxy | 仅存储账本(归档)不参与执行 | 冷归档/容灾 | 高(存储) | 原链同步 | 
| Hybrid(Auto) | 动态在 FullNode/Light/ComputeOnly 之间切换 | 此次决策采用 | 自适应 | 策略混合 |

#### 12.10.2 混合模式调度策略

依据“四层网络”度量 (示例假设):
| 层级 | 角色 | 数据类型 | 目标延迟 | 触发切换指标 |
|------|------|----------|----------|--------------|
| L1 | 冷归档 | 全量原始区块 | >1s 可接受 | 磁盘利用率 > 85% → 触发裁剪/外部归档 |
| L2 | 近期热区 | 最近 N 周区块/状态 | 200–500ms | 最近高度追平滞后 > 50 区块 → 升级 Light→Full |
| L3 | 热执行缓存 | 统一 IR + 热账户/UTXO | <50ms | Cache 命中率 < 80% → 重新分级 | 
| L4 | 内存瞬态 | 即时调度/交易池 | <10ms | 内存压力 > 70% → 降级部分 ComputeOnly 实例 |

调度核心伪代码:

```rust
fn autoscale_cycle(metrics: MetricsSnapshot) {
  for chain in supported_chains() {
    let m = metrics.chain(&chain);
    match (m.catch_up_lag, m.cpu_load, m.storage_pressure) {
      (lag, _, _) if lag > LAG_THRESHOLD_BLOCKS => promote_to_full(chain),
      (_, cpu, _) if cpu > 0.85 => demote_compute_workers(chain),
      (_, _, sp) if sp > 0.90 => offload_to_storage_proxy(chain),
      _ => maintain(chain)
    }
  }
}

```

#### 12.10.3 首批支持链 (已锁定)

| 链 | 范式 | 启动默认模式 | 说明 |
|----|------|--------------|------|
| Bitcoin | UTXO + PoW | FullNode (可降级 Light) | 需要挖矿/UTXO 索引 | 
| Ethereum | Account + PoS + EVM | FullNode 或 Light(执行分离) | 可选验证者 (32 ETH) | 
| Solana | 并行账户 + PoH+PoS | Light + ComputeOnly 扩展 | 重度带宽/并行执行 | 
| TRON | Account + DPoS | Light → Hybrid | 资源租赁/能量模型| 

#### 12.10.4 ERC20 / SPL / TRC20 资产映射范围

| 类别 | 采集方式 | 过滤策略 | 索引键 | 频率 |
|------|----------|----------|--------|------|
| ERC20 | 监听 Transfer 事件 | Top-N 市值 + 白名单 | (chain,contract,address) | 实时 | 
| SPL Token | 账户状态变更 | 热度排名 + 请求触发 | (mint,address) | 批量(Δ slot) |
| TRC20 | 事件日志 | 兼容 ERC20 事件模式 | (contract,address) | 实时 |

#### 12.10.5 跨链一致性保证

| 场景 | 策略 | 降级路径 |
|------|------|----------|
| Bitcoin 重组 | 维护影子 UTXO 分支 → 回滚镜像 | 暂停相关资产快速通道 |
| Ethereum Finality 延迟 | 标记映射状态为乐观 | 读取回退到子模块查询 |
| Solana Slot Skip | 延迟确认 fast-lane 更新 | 仅读旧镜像快照 |
| TRON 节点故障 | 切换 Light → 备用 RPC | 冻结该链入账映射 |

#### 12.10.6 对现有 Phase 的影响

| Phase | 影响 | 调整 |
|-------|------|------|
| Phase 5 (路由) | 需接入 autoscale 决策器 | 增加 chain_mode 查询接口 |
| Phase 6 (四层网络) | 指标输入源 | 增加层级压力导出 | 
| Phase 10 (多链适配) | M1 范围扩大 (包含 Solana / TRON 骨架) | 细分 M1a/M1b | 
| Phase 11 (存储与寻址) | 映射层热度分级策略 | 加入冷热资产分层 | 

#### 12.10.7 后续需要实现的组件

| 组件 | 描述 | 优先级 |
|------|------|--------|
| Autoscale Orchestrator | 采集指标 + 决策模式切换 | 高 |
| Chain Capability Registry | 记录每条链支持的模式矩阵 | 高 |
| Mirror Consistency Guard | 检测/修复镜像与原链差异 | 高 |
| Solana Adapter Skeleton | Slot 订阅 + Program 执行代理 | 高 |
| TRON Adapter Skeleton | gRPC/HTTP API 封装 + 资源模型支持 | 中 |
| Token Indexer Framework | 统一 ERC20/SPL/TRC20 事件转 IR | 高 |
| Storage Offload Manager | L1/L2 冷热迁移调度 | 中 |

> 注: 以上决策已写入,Section 14 中对应开放问题将部分转为“已解决”标记。

---

## 13. 后续研究列表

- **子模块 FFI 封装**（Bitcoin Core C++、Geth Go、Solana Rust 的统一接口）

- **进程隔离与资源限制**（cgroups、namespace、seccomp 沙箱）

- **子模块热升级机制**（原链协议升级时不停机更新）

---

## 13. 决策需明确的开放问题

| 问题 | 影响 | 决策选项 |
|------|------|----------|
| 是否持久保存全部原始区块 | 存储成本 | 全量 / 近期 + 归档 / 摘要化 |
| 统一协议是否自带签名层 | 客户端改造复杂度 | 继续用原私钥 / 引入统一身份层 |
| 适配器运行模式 | 异步线程 vs 协程调度 | 简化实现 / 最小延迟 |
| 资产跨链校验深度 | 安全 vs 速度 | 乐观 + 延迟校验 / 即时校验 |
| 是否支持主动分叉模拟 | 攻击与压力测试 | 有 / 无 |

---

## 13. 行动建议 (短期)

1. 选定首个双链组合: EVM + BTC (成本最低, 生态最大)。
2. 定义 TxIR & BlockIR 精简字段集，准备原型转换器。
3. 编写 `ChainAdapter` trait & EVM Adapter 草案 (仅 headers + tx→IR)。
4. 建立 `storage/namespace` 原型：使用 RocksDB 分区或 Postgres schema。
5. 集成 BN254 verifier (已进行) 作为隐私流水线的第一类证明。
6. 撰写威胁模型初稿并留空待补充的攻击场景。

---

## 14. 下一步可交付 (如果确认继续)

- Adapter Trait + EVM Adapter 骨架代码文件

- TxIR / BlockIR Schema JSON 文件

- 初步数据落盘示例 (1 个区块转换后的 unified_ir JSON)

- 隐私流水线 stub (commitment/nullifier builder)

> 回答“继续”则进入 M1 原型准备；回答“调整”我会修改本愿景文档。
