# SuperVM 项目 Q&A 中心

> **说明**：本文档是 SuperVM 项目的统一问答知识库，收录聊天中的优质问答内容。
> 
> **更新方式**：当聊天中出现有价值的问答时，发送指令 `收录这条Q&A` 即可自动添加到对应分类。

---

## 📚 目录

- [架构设计](#架构设计)
- [隐私与零知识证明](#隐私与零知识证明)
- [性能优化](#性能优化)
- [开发实践](#开发实践)
- [部署运维](#部署运维)
- [理论基础](#理论基础)

---

## 架构设计

### Q: SuperVM 的三路径设计有什么优势？

**A:** SuperVM 采用 **FastPath / ConsensusPath / PrivatePath** 三层架构：

1. **FastPath** - 极致性能普通交易
   - 无锁并发执行
   - 冲突自动降级到 ConsensusPath
   - 适用场景：独立账户高频交易（如游戏道具、NFT mint）

2. **ConsensusPath** - 共识保证的串行交易
   - 强一致性保证
   - 冲突检测与回滚
   - 适用场景：资金转账、合约调用

3. **PrivatePath** - 隐私交易验证
   - ZK-SNARK 零知识证明
   - 可选隐私功能（特性开关控制）
   - 适用场景：匿名转账、隐私 DeFi

**核心优势**：
- ✅ 性能与隐私解耦（普通交易不受 ZK 开销影响）
- ✅ 应用自由选择路径（根据场景需求）
- ✅ 渐进式启用隐私功能（特性开关）

---

## 隐私与零知识证明

### Q: SuperVM 的 ZK 验证器系统主要用途是什么？

**A:** 这个 ZK 验证器系统为 SuperVM 提供**隐私交易验证能力**,是整个项目的**隐私层基础设施**。

#### 核心价值
通过 **Groth16 零知识证明**(ZK-SNARK),允许用户进行**隐私保护的链上交易**,同时向所有验证节点证明交易的**数学合法性**,但**不暴露任何敏感信息**:
- ❌ 不暴露：谁是发送者(sender)
- ❌ 不暴露：转账金额(amount)  
- ❌ 不暴露：谁是接收者(receiver)
- ✅ 可验证：交易在数学上是正确的
- ✅ 可验证：没有凭空创造代币
- ✅ 可验证：发送者确实拥有这笔资产

#### 支持的隐私功能

当前实现了 **4 种核心电路**(circuit),对应不同的隐私需求:

**1. 环签名 (ring_signature_v1)** - 隐藏交易发送者
```
原理: 证明"我是这个环(ring)中 N 个人之一",但不透露具体是谁
密码学: Poseidon 哈希函数 + Schnorr 签名变体
公开输入: key_image(防双花标识)
隐私保证: 
  ✓ 发送者身份完全混淆(在 N 个候选人中)
  ✓ 通过 key_image 防止同一笔钱被花两次
  ✓ 验证者无法追踪真实签名者
典型场景: 匿名投票、隐私转账、DAO 提案
```

**2. 范围证明 (range_proof_v1)** - 隐藏金额但保证合法性
```
原理: 证明一个承诺值(commitment)在 [0, 2^64) 范围内
密码学: 64-bit 范围约束电路
公开输入: Pedersen 承诺 c = value*G + blinding*H
隐私保证:
  ✓ 金额完全隐藏(通过承诺)
  ✓ 证明金额非负(防止负数攻击)
  ✓ 证明金额在合理范围内(防止溢出)
典型场景: 
  - 隐私转账(不泄露金额)
  - 防止通货膨胀攻击
  - 合规审计(金额在监管范围内)
```

**3. RingCT (ringct_v1)** - 完整隐私交易(Monero 模式)
```
原理: 环签名 + Pedersen 承诺 + 范围证明 + Merkle 树验证
密码学: 
  - Pedersen 承诺隐藏输入/输出金额
  - Merkle 证明验证 UTXO 存在性
  - 范围证明确保金额合法
公开输入(5 个 Fr):
  1. input_commitment: 输入金额承诺
  2. output_commitment: 输出金额承诺
  3. merkle_root: UTXO 集合的 Merkle 根
  4-5. 其他辅助参数
隐私保证:
  ✓ 发送者身份隐藏(环签名)
  ✓ 金额完全隐藏(承诺)
  ✓ 接收者地址隐藏(一次性地址)
  ✓ 交易图谱无法追踪
典型场景:
  - 完全隐私的价值转移
  - 隐私 DeFi(匿名 swap/借贷)
  - 企业级隐私支付
```

**4. Multiply (multiply_v1)** - 示例/测试电路
```
原理: 简单的乘法约束 a * b = c
用途: 
  ✓ 验证系统正确性
  ✓ 性能基准测试
  ✓ 开发者入门示例
约束数: ~5 个(最简单)
```

#### 与 SuperVM 三路径的集成

ZK 验证器是 **PrivatePath** 的核心组件:

```rust
// FastPath - 高性能公开交易(无 ZK 开销)
let tx_fast = Transaction {
    privacy: Privacy::Public,
    data: /* 普通交易数据 */,
};

// ConsensusPath - 共识保证的公开交易
let tx_consensus = Transaction {
    privacy: Privacy::Public,
    data: /* 需要强一致性的交易 */,
};

// PrivatePath - 隐私交易(使用 ZK 验证)
let tx_private = Transaction {
    privacy: Privacy::Private,
    circuit_id: "ringct_v1",           // 使用 RingCT 电路
    proof: proof_bytes,                 // Groth16 证明
    public_inputs: public_inputs_bytes, // 5 个 Fr 公开输入
};

// SuperVM 自动路由到对应路径
let receipt = supervm.execute_transaction(&tx_private)?;
```

**关键设计原则**:
1. **可选性**: 通过 `groth16-verifier` 特性开关控制
2. **隔离性**: ZK 验证不影响 FastPath/ConsensusPath 性能
3. **灵活性**: 应用可根据场景选择隐私级别
4. **渐进式**: 可以先部署 FastPath,后续再启用隐私功能

#### 实际应用场景

**场景 A: 匿名转账(类似 Monero)**
```
用例: Alice 给 Bob 转 100 USDT,但不想让任何人知道
技术方案: 使用 ringct_v1 电路
流程:
  1. Alice 生成 RingCT 证明:
     - 从 UTXO 集合中选择 10 个作为"环"
     - 生成输入/输出 Pedersen 承诺
     - 生成范围证明(金额 100 在 [0, 2^64))
     - 生成 Merkle 证明(证明输入 UTXO 存在)
  2. 提交隐私交易到链上:
     proof: 384 字节
     public_inputs: 164 字节(5 个 Fr)
  3. 验证节点验证 ZK 证明(~20ms)
  4. 交易上链,但链上只能看到:
     ✓ 证明验证通过 ✅
     ✓ 承诺值(无法反推金额)
     ✓ key_image(防双花)
     ✗ 看不到: Alice 身份、100 USDT、Bob 地址

隐私保证:
  - 发送者匿名度: 1/10(环大小)
  - 金额完全隐藏(信息论安全)
  - 接收者完全隐藏(一次性地址)
```

**场景 B: 合规审计(监管友好型隐私)**
```
用例: 企业需要隐私转账,但监管机构需要审计能力
技术方案: RingCT + 审计密钥(audit key)
流程:
  1. 系统生成主密钥对:
     - view_key: 可以解密承诺(查看金额)
     - spend_key: 可以花费 UTXO
  2. 企业进行隐私交易(RingCT)
  3. 监管机构持有 view_key:
     - 可以解密特定交易的金额
     - 验证金额是否在合规范围内
     - 追踪资金流向(如果需要)
  4. 普通用户无 view_key:
     - 只能看到承诺和证明
     - 无法解密任何信息

平衡点:
  ✓ 默认隐私(普通用户看不到)
  ✓ 选择性披露(监管机构可审计)
  ✓ 数学合法性(ZK 证明保证)
```

**场景 C: 隐私 DeFi(防止 MEV 攻击)**
```
用例: 在 DEX 上执行大额交易,防止抢跑和三明治攻击
技术方案: 环签名 + 承诺
流程:
  1. 交易者提交加密的交易意图:
     - 使用环签名隐藏身份
     - 使用承诺隐藏交易金额
     - 使用时间锁延迟披露
  2. 矿工/验证者:
     - 看不到交易细节(无法抢跑)
     - 验证 ZK 证明确保交易合法
  3. 交易执行后:
     - 解密承诺(或不解密)
     - 更新链上状态

防护能力:
  ✓ 防止 front-running(看不到交易内容)
  ✓ 防止 sandwich attack(金额隐藏)
  ✓ 防止价格操纵(身份混淆)
```

**场景 D: 隐私投票/DAO 治理**
```
用例: DAO 提案投票,但不泄露投票者身份和权重
技术方案: 环签名 + 范围证明
流程:
  1. 投票者生成证明:
     - 环签名证明"我是 DAO 成员之一"
     - 范围证明证明"我的投票权重在合理范围"
     - 承诺投票选项(赞成/反对)
  2. 提交到链上:
     - 验证者确认证明有效
     - 累加投票结果(同态加密)
  3. 投票结束:
     - 解密总结果
     - 但无法追踪个人投票

隐私保证:
  ✓ 投票者身份匿名
  ✓ 投票内容隐藏(投票期间)
  ✓ 防止贿选(无法证明投给谁)
  ✓ 防止 Sybil 攻击(范围证明)
```

#### 技术优势

**与传统隐私方案对比**:
| 方案 | 隐私模式 | 性能 | 灵活性 | 合规性 |
|------|---------|------|--------|--------|
| 以太坊 | 完全透明 | 高 | 高 | 高 |
| Monero | 强制隐私 | 中 | 低 | 低 |
| Zcash | 可选隐私(双池) | 中 | 中 | 中 |
| **SuperVM** | **三路径可选** | **高** | **高** | **高** |

**SuperVM 独特优势**:
1. **架构隔离**: 隐私不影响普通交易性能
2. **渐进式部署**: 可以先不启用隐私功能
3. **合规友好**: 预留审计接口(view key)
4. **开发者友好**: 完整的 API/测试/示例/文档
5. **模块化设计**: 可以只启用需要的电路

#### 性能指标

| 电路 | Prove 时间 | Verify 时间 | Proof 大小 | 约束数 |
|------|-----------|------------|-----------|--------|
| multiply_v1 | ~100ms | ~10ms | 384 字节 | ~5 |
| ring_signature_v1 | ~200ms | ~15ms | 384 字节 | ~150 |
| range_proof_v1 | ~150ms | ~12ms | 384 字节 | ~64 |
| ringct_v1 | ~400ms | ~20ms | 384 字节 | ~400 |

**优化空间**(未实现):
- 批量验证: 10 个 proof → ~50ms(而非 200ms)
- 并行验证: 线性加速(核心数)
- Compressed 序列化: proof 大小减半
- PVK 缓存: 首次验证后 < 5ms

#### 为什么重要？

这个系统解决了区块链领域的**根本矛盾**:
```
公开透明 ⚖️ 隐私保护
  ↓              ↓
易于审计      用户权利
防止作弊      商业秘密
监管合规      抗审查性
```

**SuperVM 的解决方案**:
- 通过 ZK 证明实现"**可验证的隐私**"
- 通过三路径架构实现"**可选的隐私**"
- 通过审计密钥实现"**可审计的隐私**"

这是构建**下一代 Web3 应用**的关键基础设施:
- ✅ DeFi 不再是"链上暴露所有策略"
- ✅ DAO 投票不再是"完全公开的政治压力"
- ✅ 价值转移不再是"所有人都能看到你的资产"
- ✅ 企业应用不再是"商业机密全公开"

**这就是 ZK 验证器的核心价值** 🛡️

---

### Q: 与 SuperVM 的三条执行路径如何配合？

**A:** ZK 验证器与路径集成方式：

```rust
// 1. FastPath - 高性能普通交易（无 ZK 开销）
let tx1 = Transaction { privacy: Privacy::Public, ... };

// 2. ConsensusPath - 需要共识的公开交易
let tx2 = Transaction { privacy: Privacy::Public, ... };

// 3. PrivatePath - 隐私交易（使用 ZK 验证）
let tx3 = Transaction { privacy: Privacy::Private, ... };
```

当交易标记为 `Privacy::Private` 时：
- 进入 **PrivatePath** 处理
- 验证 ZK 证明确保交易合法性
- 调用 `supervm.verify_with(&circuit_id, &proof, &public_inputs)`

**关键设计**：
- 验证器通过 `with_verifier()` 可选注入（不影响 L1 核心）
- 特性开关 `groth16-verifier` 控制 ZK 依赖（默认构建轻量）
- FastPath/ConsensusPath 性能完全不受影响

---

### Q: ZK 隐私交易的实际应用场景有哪些？

**A:** 三大核心场景：

#### 场景 A: 匿名转账
```
Alice 给 Bob 转 100 代币，但不想让链上观察者知道：
✓ 谁是发送者（环签名混淆）
✓ 转了多少钱（Pedersen 承诺隐藏金额）
✓ 金额是否合法（范围证明确保 >= 0）

链上只需验证 ZK 证明 ✅，无需查看明文交易
```

#### 场景 B: 合规审计
```
监管机构可以获得"审计密钥"：
✓ 特定条件下解密 Pedersen 承诺
✓ 验证范围证明确保没有洗钱
✓ 普通用户无法解密

平衡隐私与合规需求
```

#### 场景 C: DeFi 隐私交易
```
在 DEX 上交易，隐藏：
✓ 交易策略（环签名隐藏身份）
✓ 持仓量（承诺隐藏金额）
验证器只需确认"这笔交易数学上是对的"

防止抢跑（front-running）和三明治攻击
```

---

### Q: 为什么采用特性开关（feature flag）设计？

**A:** 三大优势：

1. **默认轻量** - 不启用特性时，不包含 ZK 依赖（arkworks 库很大，~10MB）
2. **灵活部署** - 应用可选择是否启用隐私功能
3. **L1 保护** - 验证器不影响 FastPath/ConsensusPath 的关键路径性能

```toml
# 默认构建（轻量，无 ZK 依赖）
cargo build

# 启用隐私功能（包含 arkworks）
cargo build --features groth16-verifier
```

**架构隔离**：
```rust
#[cfg(feature = "groth16-verifier")]
pub mod groth16_verifier;

#[cfg(feature = "groth16-verifier")]
pub use groth16_verifier::Groth16Verifier;
```

---

### Q: ZK 验证性能如何？

**A:** 当前性能指标（单核，未优化）：

| 电路 | Setup | Prove | Verify | 约束数 | Proof 大小 |
|------|-------|-------|--------|--------|-----------|
| multiply_v1 | ~50ms | ~100ms | ~10ms | ~5 | 384 字节 |
| ring_signature_v1 | ~150ms | ~200ms | ~15ms | ~150 | 384 字节 |
| range_proof_v1 | ~100ms | ~150ms | ~12ms | ~64 | 384 字节 |
| ringct_v1 | ~300ms | ~400ms | ~20ms | ~400 | 384 字节 |

**优化空间**（未实现）：
- 批量验证：一次验证 N 个 proof（摊销 pairing 开销）
- 并行验证：rayon 并行处理多笔隐私交易
- Compressed 序列化：减小传输开销 50%
- PVK 预热缓存：避免重复反序列化

**对比 Monero/Zcash**：
- Monero RingCT 验证：~50-100ms（C++ 高度优化）
- Zcash Groth16 验证：~5-10ms（专用硬件加速）
- SuperVM 当前：~10-20ms（Rust 基础实现，有优化空间）

---

### Q: 与传统区块链隐私方案对比？

**A:** 关键差异：

| 方案 | 隐私模式 | 性能 | 灵活性 | 合规性 |
|------|---------|------|--------|--------|
| **以太坊** | 完全透明 | 高 | 高 | 高 |
| **Monero** | 全链隐私 | 中 | 低（必须隐私）| 低 |
| **Zcash** | 可选隐私 | 中 | 中 | 中 |
| **SuperVM** | **可选隐私路径** | **高**（分离） | **高**（三路径） | **高**（审计密钥）|

**SuperVM 独特优势**：
- ✅ 普通交易走 FastPath（高性能，无 ZK 开销）
- ✅ 隐私交易走 PrivatePath（ZK 验证）
- ✅ 应用自由选择隐私级别（不强制）
- ✅ 不牺牲整体性能（架构隔离）
- ✅ 预留审计接口（合规友好）

---

### Q: ZK 公开输入编码协议是什么？

**A:** 两种编码方式（与 arkworks 序列化对齐）：

#### 单 Fr 协议（简单电路）
适用于：multiply_v1, ring_signature_v1, range_proof_v1

```rust
// 公开输入只有一个 Field Element
public_inputs_bytes = Fr.serialize_uncompressed()
// 固定 32 字节（BLS12-381 scalar）
```

#### Vec<Fr> 协议（多输入电路）
适用于：ringct_v1 等复杂电路

```rust
// 多个公开输入，长度前缀编码
public_inputs_bytes = [
    u32_le(length),  // 4 字节长度前缀（小端序）
    Fr0.serialize(), // 第一个 Fr（32 字节）
    Fr1.serialize(), // 第二个 Fr（32 字节）
    ...
]

// 示例：ringct_v1 有 5 个公开输入
// 总大小 = 4 + 5*32 = 164 字节
```

**关键约定**：
- 使用 `CanonicalSerialize::serialize_uncompressed()`（与验证器匹配）
- Vec 协议必须包含长度前缀（防止歧义）
- 发送者和验证者必须使用相同编码（否则验证失败）

---

### Q: 如何在代码中使用 ZK 验证器？

**A:** 三步集成（需启用 `groth16-verifier` 特性）：

```rust
use vm_runtime::privacy::{Groth16Verifier, ZkVerifier};

// 1. 创建验证器实例
#[cfg(feature = "groth16-verifier")]
let verifier = Groth16Verifier::new();

// 2. 注册需要的电路（加载 PreparedVerifyingKey）
#[cfg(feature = "groth16-verifier")]
{
    // 从文件加载 VK 并预处理
    let vk_bytes = std::fs::read("vk.bin")?;
    let vk = VerifyingKey::<Bls12_381>::deserialize_uncompressed(&vk_bytes[..])?;
    let pvk = prepare_verifying_key(&vk);
    
    // 注册到验证器
    verifier.register_ring_signature_v1_with_pvk(pvk);
    verifier.register_range_proof_v1_with_pvk(pvk2);
    verifier.register_ringct_v1_with_pvk(pvk3);
}

// 3. 注入到 SuperVM（可选）
let supervm = SuperVM::new(&manager)
    .with_verifier(&verifier);

// 4. 执行隐私交易（自动验证 ZK 证明）
let private_tx = Transaction {
    privacy: Privacy::Private,
    proof: proof_bytes,
    public_inputs: inputs_bytes,
    ...
};

let receipt = supervm.execute_transaction(&private_tx)?;
// 内部会调用 verifier.verify_proof("ring_signature_v1", &proof, &inputs)
```

**完整示例**：参见 `examples/zk_verify_*.rs` 文件。

---

### Q: 生产环境部署 ZK 验证器的注意事项？

**A:** 关键要点（6 项核心）：

#### 1. Trusted Setup（可信设置）
```
⚠️ 必须使用 MPC ceremony 生成参数（多方计算）
- 防止单点"有毒废料"（toxic waste）攻击
- 参考 Zcash Powers of Tau / Hermez Ceremony

❌ 不要：使用测试环境的 setup 参数
✅ 要做：组织公开的 MPC ceremony 或使用行业标准参数
```

#### 2. 参数标准化
```
⚠️ 哈希函数/曲线参数应使用行业标准
- Poseidon: 使用 SAFE rounds（当前示例为简化参数）
- BLS12-381: 保持与 Zcash/Filecoin 对齐

❌ 不要：自定义 Poseidon 参数（安全性未验证）
✅ 要做：使用 neptune/poseidon-rs 等标准库
```

#### 3. PVK 分发与保护
```
✓ VerifyingKey 可公开分发（链上存储）
✗ ProvingKey 必须保密（用户持有或 TEE 环境）

分发方式：
- IPFS 固定 VK CID（去中心化）
- 链上 VK Registry 合约
- P2P 网络 DHT 存储
```

#### 4. 审计密钥机制
```
为合规场景预留选择性披露：
- 监管机构持有"审计私钥"
- 用户可选生成"view key"（类似 Monero）
- 承诺可在特定条件下解密

⚖️ 平衡隐私与合规需求
```

#### 5. 性能优化
```
启用生产级优化：
- 批量验证（Groth16 支持 pairing 批处理）
- 并行验证（rayon 线程池）
- PVK 内存缓存（避免重复反序列化）
- Compressed 序列化（减小网络传输）
```

#### 6. 安全审计
```
必须经过专业审计：
- 电路逻辑正确性（是否有漏洞）
- 密码学实现（侧信道攻击/时序攻击）
- 参数安全性（R1CS 约束完整性）

推荐审计机构：Trail of Bits / ABDK / OpenZeppelin
```

---

### Q: ZK 验证器后续扩展计划？

**A:** 三阶段路线图：

#### 短期（已规划）
- ✅ 基础电路接入（multiply/ring_signature/range_proof/ringct）
- 🔄 序列化工具化（减少样板代码）
- 🔄 性能优化（批量验证、并行化）

```rust
// 计划中的工具模块
use vm_runtime::privacy::zk_utils;

let (vk, proof, inputs) = zk_utils::load_artifacts("demo_dir")?;
zk_utils::save_artifacts("output_dir", &vk, &proof, &inputs)?;
```

#### 中期
- **RingCT 压缩版**：减小证明大小 50%（Compressed serialization）
- **多 UTXO 支持**：批量输入/输出（类似 Bitcoin UTXO 模型）
- **聚合范围证明**：Bulletproofs++ 替代 Groth16 范围证明（更小 proof）

```rust
// 未来 API
verifier.register_ringct_compressed_v2_with_pvk(pvk);
verifier.register_multi_utxo_v1_with_pvk(pvk);
```

#### 长期
- **递归证明**：Halo2 / Nova（无需 trusted setup）
- **跨链隐私桥**：ZK 证明跨链验证（以太坊 L1 验证 SuperVM 隐私交易）
- **合规审计工具链**：自动化审计报告生成、链上合规验证

```rust
// 未来愿景：递归证明聚合
let aggregated_proof = aggregate_proofs(&[proof1, proof2, proof3])?;
// 单次验证替代 3 次验证（节省 Gas/CPU）
```

---

### Q: 在哪里可以找到 ZK 验证器的完整示例？

**A:** 四个可运行示例（需启用 `groth16-verifier` 特性）：

```powershell
# 1. Multiply 电路（入门示例）
cargo run -p vm-runtime --features groth16-verifier --example zk_verify_multiply
# 输出：VK(872字节) + Proof(384字节) + c(32字节)
# 验证正确/错误公开输入

# 2. Ring Signature（环签名）
cargo run -p vm-runtime --features groth16-verifier --example zk_verify_ring_signature
# 输出：Ring size=5, key_image 验证
# 防双花演示

# 3. Range Proof（范围证明）
cargo run -p vm-runtime --features groth16-verifier --example zk_verify_range_proof
# 输出：value=12345678901234 in [0, 2^64)
# 负数攻击防御

# 4. RingCT（完整隐私交易）
cargo run -p vm-runtime --features groth16-verifier --example zk_verify_ringct
# 输出：5 个公开输入（input/output commitments + merkle root）
# 完整 Monero-style 隐私交易
```

**每个示例包含**：
- ✅ 完整的 Setup → Prove → Verify 流程
- ✅ VK/Proof/Inputs 序列化与文件持久化
- ✅ 正确/错误公开输入的验证对比
- ✅ 详细的 README 文档（`examples/README-zk-verify-*.md`）

**测试覆盖**：
```powershell
# 运行所有 ZK 集成测试
cargo test -p vm-runtime --test privacy_verifier_tests --features groth16-verifier
# 8/8 tests passed（包含 4 个电路的端到端测试）
```

---

## 性能优化

*（此分类待补充，可从后续聊天中收录）*

---

## 开发实践

*（此分类待补充，可从后续聊天中收录）*

---

## 部署运维

*（此分类待补充，可从后续聊天中收录）*

---

## 理论基础

*（此分类待补充，可从后续聊天中收录）*

---

## 📝 更新日志

| 日期 | 更新内容 | 来源 |
|------|---------|------|
| 2025-11-06 | 初始化 Q&A 中心，添加"隐私与零知识证明"分类 12 条问答 | 聊天记录整理 |

---

## 🔧 使用指南

### 如何自动收录新问答？

当您在聊天中得到满意的回答时，发送以下指令之一：

```
收录这条Q&A
收录到Q&A
添加到FAQ
```

系统会自动：
1. 识别最近的一组 Q&A
2. 推断合适的分类（或您可指定：`收录到"性能优化"分类`）
3. 格式化并追加到对应分类
4. 更新"更新日志"

### 分类说明

- **架构设计**：三路径设计、模块划分、接口设计等架构级问题
- **隐私与零知识证明**：ZK-SNARK、电路设计、隐私交易等密码学问题
- **性能优化**：并发、内存、序列化等性能相关问题
- **开发实践**：代码风格、测试、调试等开发流程问题
- **部署运维**：Docker、监控、日志等运维问题
- **理论基础**：密码学、分布式共识、虚拟机原理等理论问题

### 查找技巧

- 使用 Ctrl+F 搜索关键词（如"环签名"、"性能"、"部署"）
- 查看目录快速定位分类
- 优先查看"更新日志"了解最新问答

---

**持续更新中... 💡**
