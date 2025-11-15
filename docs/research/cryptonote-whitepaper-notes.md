# CryptoNote 白皮书学习笔记

开发者/作者：King Xujue

**论文标题**: CryptoNote v2.0  
**作者**: Nicolas van Saberhagen  
**日期**: 2013-10-17  
**下载链接**: https://cryptonote.org/whitepaper.pdf  
**学习周期**: Week 1-2 (2025-02-03 至 2025-02-16)

---

## 📋 学习清单

- [ ] 第 1-2 章: 背景与目标

- [ ] 第 3 章: Untraceable Transactions (不可追踪交易)

- [ ] 第 4 章: One-time Keys (一次性密钥)

- [ ] 第 5 章: Ring Signatures (环签名)

- [ ] 第 6 章: Double-Spending Prevention (防双花)

- [ ] 第 7 章: 协议细节

---

## 🎯 1. 核心目标

### 1.1 CryptoNote 解决的问题

**Bitcoin 的隐私问题**:
1. ❌ **可追踪性**: 所有交易公开可见
2. ❌ **地址关联**: 重复使用地址泄露身份
3. ❌ **金额透明**: 所有金额明文可见

**CryptoNote 的解决方案**:
1. ✅ **不可追踪**: Ring Signatures 隐藏发送方
2. ✅ **不可关联**: One-time Keys 每次生成新地址
3. ✅ **金额隐藏**: RingCT (v2.0 后引入, 不在原白皮书)

### 1.2 设计原则

- **隐私默认**: 不是可选特性, 而是强制隐私

- **去中心化**: PoW 挖矿 (CryptoNight 算法)

- **可审计性**: 接收方可选择性公开交易 (View Key)

---

## 🔑 2. One-time Keys (一次性密钥)

### 2.1 核心思想

**问题**: Bitcoin 地址重复使用导致隐私泄露

**解决**: 每笔交易生成唯一的一次性地址

### 2.2 密钥体系

**接收方钱包**:

```

私钥对:

- (a, A): Spend key pair (花费密钥)
  A = aG (公钥, 公开)
  
- (b, B): View key pair (视图密钥)
  B = bG (公钥, 公开)

钱包地址 = (A, B) 的编码

```

**为什么需要两个密钥对?**

- **Spend key (a)**: 签名交易 (花费权限)

- **View key (b)**: 扫描交易 (只读权限)

- 分离 → 可以安全授权第三方审计 (只给 View Key)

### 2.3 一次性地址生成

**发送方流程**:

```

输入:

- 接收方地址: (A, B)

- 输出索引: n (第几个输出)

步骤:
1. 随机生成 r (交易私钥, ephemeral)
2. 计算 R = rG (交易公钥, 放在链上)
3. 计算共享秘密: S = rA (ECDH)
4. 派生一次性私钥哈希: h = H_s(S, n)
5. 计算一次性公钥: P = H_s(S, n)G + B

输出:

- 链上公开: (R, P)

- 接收方可见: P 对应金额

```

**接收方识别**:

```

输入:

- 链上数据: (R, P)

- 自己的私钥: (a, b)

步骤:
1. 计算共享秘密: S = aR (= arG = rA, ECDH)
2. 派生哈希: h = H_s(S, n)
3. 计算期望公钥: P' = H_s(S, n)G + B
4. 检查: P' == P?
   - 如果相等 → 这笔输出属于我!
   - 记录 (P, h) 用于后续花费

花费:

- 一次性私钥: p = h + b (推导出来的)

- 验证: pG = hG + bG = hG + B = P ✓

```

### 2.4 数学证明

**正确性证明**:

```

发送方: P = H_s(rA, n)G + B
接收方: p = H_s(aR, n) + b

验证 pG = P:
  pG = (H_s(aR, n) + b)G
     = H_s(aR, n)G + bG
     = H_s(arG, n)G + B    (因为 aR = arG = rA)
     = H_s(rA, n)G + B
     = P ✓

```

**安全性**:

- 外部观察者只能看到随机公钥 P (无法关联接收方)

- 只有知道 a 的人才能计算 S = aR

---

## 🎭 3. Ring Signatures (环签名)

### 3.1 核心概念

**目标**: 隐藏交易的真实输入

**方法**: 将真实输入混入多个"诱饵"输入 (decoys)

### 3.2 环签名定义

**签名者**:

- 拥有私钥 x_s (真实输入)

- 公钥 P_s = x_s G

**环成员**:

- 公钥集合: {P_0, P_1, ..., P_n}

- 其中 P_s 是真实公钥, 其他是诱饵

**签名**:

- 证明: "我知道环中某一个私钥"

- 但不透露是哪一个!

### 3.3 白皮书中的环签名构造 (简化版)

**Traceable Ring Signature** (可追踪环签名):

```

公开参数:

- 环成员: {P_0, P_1, ..., P_n}

- 消息: m

签名者 (知道 x_s):
1. 计算 Key Image: I = x_s * H_p(P_s)
   (防双花标记)

2. 选择随机数: α
   计算: L = αG, R = αH_p(P_s)

3. 对每个非秘密索引 i != s:
   - 随机选择 q_i, w_i
   - 计算 L_i = q_i G + w_i P_i
   - 计算 R_i = q_i H_p(P_i) + w_i I

4. 计算挑战: c = H(m, L_0, ..., L_n, R_0, ..., R_n)

5. 环形求解:
   - c_s = c - Σ(w_i) mod l
   - q_s = α - c_s * x_s mod l

输出签名:
σ = (I, c_0, q_0, ..., c_n, q_n)

```

**验证**:

```

输入: 签名 σ, 环 {P_0, ..., P_n}, 消息 m

步骤:
1. 重新计算 L_i, R_i:
   L_i = q_i G + c_i P_i
   R_i = q_i H_p(P_i) + c_i I

2. 验证挑战:
   H(m, L_0, ..., L_n, R_0, ..., R_n) == Σ(c_i) mod l

3. 接受 iff 验证通过

```

### 3.4 与 Monero CLSAG 的区别

**白皮书签名** (2013):

- 基础可追踪环签名

- 签名大小: O(n) (n = ring size)

- 每个环成员需要 2 个标量 (q_i, c_i)

**Monero CLSAG** (2020):

- 优化的简洁环签名

- 签名大小: O(n) 但常数更小

- 验证速度更快 (~2x)

**SuperVM 应选择**: CLSAG (更现代, Monero 已验证)

---

## 🚫 4. Double-Spending Prevention (防双花)

### 4.1 Key Image 机制

**问题**: 环签名隐藏真实输入, 如何防止重复花费?

**解决**: Key Image 作为"支票编号"

### 4.2 Key Image 生成

```

输入:

- 一次性私钥: p (属于某个输出 P)

- 一次性公钥: P = pG

计算:
I = p * H_p(P)

性质:
1. 确定性: 同样的 p 生成同样的 I
2. 唯一性: 不同的 p 生成不同的 I (概率上)
3. 不可伪造: 不知道 p 无法计算有效的 I
4. 无关联性: 从 (P, I) 无法推出 p

```

### 4.3 双花检测

**验证节点维护 Key Image 集合**:

```

全局状态: key_image_set = {}

验证交易:
1. 提取签名中的 Key Image: I
2. 检查: I ∈ key_image_set?
   - 是 → 拒绝 (双花!)
   - 否 → 继续验证

3. 验证环签名有效性:
   - 验证 I 确实由环中某个私钥生成
   
4. 接受交易:
   - 添加 I 到 key_image_set

```

### 4.4 安全性分析

**攻击场景 1: 重复花费同一输出**

- 防御: Key Image 相同 → 被拒绝 ✓

**攻击场景 2: 伪造 Key Image**

- 防御: 环签名验证失败 (无法证明知道私钥) ✓

**攻击场景 3: 用不同 Key Image 花费同一输出**

- 防御: 数学上不可行 (I = p * H_p(P) 是确定性的) ✓

---

## 🔐 5. 协议细节

### 5.1 交易结构

```

Transaction:
├── version
├── unlock_time
├── inputs: [TxIn]
│   └── TxIn:
│       ├── ring: [PublicKey]  (环成员)
│       ├── key_offsets        (存储优化)
│       └── signature          (环签名 + Key Image)
└── outputs: [TxOut]
    └── TxOut:
        ├── amount             (金额, RingCT 前明文)
        └── target: PublicKey  (一次性公钥 P)

```

### 5.2 地址编码

**标准地址**: Base58(network_byte + A + B + checksum)

- network_byte: 主网/测试网标识

- A: Spend public key (32 bytes)

- B: View public key (32 bytes)

- checksum: 前 4 字节哈希

### 5.3 挖矿与 PoW

**CryptoNight 算法** (白皮书提出):

- ASIC 抗性: 需要 2 MB 内存

- CPU 友好: 普通电脑可挖矿

- Monero 后来改用 RandomX

---

## 📊 6. 性能与权衡

### 6.1 交易大小

**对比 Bitcoin**:

| 指标 | Bitcoin | CryptoNote (ring=11) |
|------|---------|---------------------|
| 单输入大小 | ~150 bytes | ~1.5 KB (10x) |
| 单输出大小 | ~40 bytes | ~100 bytes (2.5x) |
| 典型交易 | ~250 bytes | ~3 KB |

**权衡**: 隐私 ↔ 存储/带宽

### 6.2 验证时间

**环签名验证**: O(n) 时间复杂度

- ring_size = 11 → ~5ms

- ring_size = 64 → ~30ms

**批量验证优化**: 可并行验证多个签名

---

## 🎯 7. 应用到 SuperVM

### 7.1 核心设计决策

**从白皮书学到的经验**:

1. **密钥体系**: 采用双密钥 (Spend + View)
   - ✅ 允许只读审计 (View Key)
   - ✅ 冷钱包友好 (Spend Key 离线)

2. **一次性地址**: 必须实现
   - ✅ 防止地址关联
   - 实现: `stealth_address.rs`

3. **环签名**: 选择 CLSAG (不是白皮书原版)
   - 原因: Monero 7 年实战验证
   - 实现: `ring_signature.rs`

4. **Key Image**: 必须实现
   - ✅ 防双花核心机制
   - 存储: 需要高效索引 (Week 7-8 设计)

### 7.2 API 设计草案

```rust
// 基于 CryptoNote 理论设计

pub struct WalletKeys {
    pub spend_secret: SecretKey,   // a
    pub spend_public: PublicKey,   // A = aG
    pub view_secret: SecretKey,    // b
    pub view_public: PublicKey,    // B = bG
}

pub fn generate_one_time_address(
    recipient_spend_public: &PublicKey,
    recipient_view_public: &PublicKey,
    tx_secret_key: &SecretKey,
    output_index: u32,
) -> (PublicKey, PublicKey) {
    // 返回: (R, P)
    // R = rG (交易公钥)
    // P = H_s(rA, n)G + B (一次性公钥)
    todo!()
}

pub fn scan_transaction(
    tx_public_key: &PublicKey,      // R
    output_public_key: &PublicKey,  // P
    view_secret_key: &SecretKey,    // b
    spend_public_key: &PublicKey,   // B
    output_index: u32,
) -> Option<SecretKey> {
    // 如果属于自己, 返回一次性私钥 p
    todo!()
}

```

### 7.3 实现优先级

**Week 9-12** (Phase 2.2.1):

- [x] 理论学习 ✓ (本笔记)

- [ ] 实现 Key Image 生成

- [ ] 实现 CLSAG 签名/验证

**Week 13-16** (Phase 2.2.2):

- [ ] 实现一次性地址生成

- [ ] 实现交易扫描

- [ ] 钱包集成

---

## 📚 8. 延伸阅读

### 8.1 必读资源

- [x] **CryptoNote v2.0 Whitepaper** (本笔记)

- [ ] **Zero to Monero 2.0** - 更详细的数学推导
  - https://web.getmonero.org/library/Zero-to-Monero-2-0-0.pdf
  - 章节 3-5 重点阅读

- [ ] **MRL 论文** (Monero Research Lab)
  - MRL-0005: Ring CT 2.0
  - MRL-0011: CLSAG

### 8.2 相关技术

- **Bulletproofs** - 范围证明 (不在原白皮书)

- **Stealth Addresses** - 更详细的实现

- **Subaddresses** - Monero 扩展 (方便多地址管理)

### 8.3 对比研究

| 项目 | 环签名 | 隐身地址 | 金额隐藏 | zkSNARK |
|------|-------|---------|---------|---------|
| CryptoNote | ✅ | ✅ | ❌ | ❌ |
| Monero | ✅ (CLSAG) | ✅ | ✅ (RingCT) | ❌ |
| Zcash | ❌ | ❌ | ✅ | ✅ |
| SuperVM | ✅ (计划) | ✅ (计划) | ✅ (计划) | ? (Week 3-4 评估) |

---

## ✅ 学习进度

### Week 1 (2025-02-03 至 2025-02-09)

**Day 1 (2025-02-03)**:

- [x] 下载白皮书

- [x] 创建学习笔记框架

- [ ] 阅读第 1-3 章 (背景 + 不可追踪性)

**Day 2**:

- [ ] 阅读第 4 章 (One-time Keys) - 详细推导

- [ ] 手动验证数学公式

**Day 3**:

- [ ] 阅读第 5 章 (Ring Signatures) - 签名构造

- [ ] 对比 Monero CLSAG 改进

**Day 4-5**:

- [ ] 阅读第 6 章 (Double-Spending) - Key Image

- [ ] 阅读第 7 章 (协议细节)

- [ ] 完成白皮书全文

**Day 6-7**:

- [ ] 结合 Monero 源码验证理论

- [ ] 总结核心概念

- [ ] 准备 Week 2 实践

### Week 2 (2025-02-10 至 2025-02-16)

**Day 8-10**:

- [ ] 阅读 Zero to Monero Ch3-5

- [ ] 深入 RingCT 数学推导

**Day 11-14**:

- [ ] 编写技术选型报告

- [ ] 设计 SuperVM 隐私 API

- [ ] 准备 Week 3-4 zkSNARK 评估

---

## 💡 问题与思考

### 关键问题

1. **为什么需要 H_p (Hash-to-Point)?**
   - 答: 确保 Key Image 在曲线上, 且无法预测

2. **如何选择 Ring Members?**
   - 白皮书: 随机选择
   - Monero: Gamma 分布 (模拟真实花费模式)

3. **View Key 泄露的风险?**
   - 可以看到所有交易金额和接收记录
   - 但无法花费 (需要 Spend Key)
   - 用途: 审计, 交易所合规

### 个人理解

**CryptoNote 的天才设计**:
1. 环签名 → 隐藏发送方
2. 一次性地址 → 隐藏接收方
3. Key Image → 防双花 (不泄露身份)

三者结合 = 完整隐私层!

**TODO**: 每日记录新的理解

---

## 🔗 相关笔记

- `monero-study-notes.md` - Monero 源码实现

- `curve25519-dalek-notes.md` - 密码学库学习

- `ring-signature-deep-dive.md` (Week 2 创建)

---

**最后更新**: 2025-11-04  
**下次更新**: 每日同步学习进度
