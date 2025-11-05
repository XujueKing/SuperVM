# Monero 源码学习笔记
开发者/作者：King Xujue

**研究周期**: 2025-03-03 开始（持续更新）  
**参考仓库**: [Monero Project](https://github.com/monero-project/monero)  
**学习目标**: 理解 Ring Signature, Stealth Address, Key Image 实现细节  
**完整引用**: 详见 [ATTRIBUTIONS.md](../ATTRIBUTIONS.md)

---

## 📋 学习清单

- [x] Ring Signature 基本原理（已应用于 zk-groth16-test）
- [x] Key Image 防双花机制（已实现）
- [ ] Stealth Address 生成机制（进行中）
- [ ] RingCT 完整交易流程（计划中）

---

## 🔍 1. Ring Signature (环签名)

### 1.1 核心文件定位

**关键文件**:
- `src/ringct/rctSigs.cpp` - RingCT 签名实现
- `src/ringct/rctTypes.h` - RingCT 类型定义
- `src/cryptonote_core/cryptonote_tx_utils.cpp` - 交易构造
- `src/crypto/crypto.cpp` - 基础加密原语

### 1.2 Ring Signature 类型

Monero 支持多种环签名算法:

| 算法 | 引入版本 | 签名大小 | 验证速度 | 特点 |
|------|---------|---------|---------|------|
| MLSAG | v7 (2016) | ~1.7 KB (ring=11) | 慢 | 多层链接匿名群签名 |
| CLSAG | v12 (2020) | ~1.5 KB (ring=11) | 快 | 简洁链接匿名群签名 |
| Triptych | 提案中 | ~1.2 KB | 更快 | 基于对数大小证明 |

**当前实现**: CLSAG (自 v12 Monero 协议)

### 1.3 CLSAG 签名结构

```cpp
// src/ringct/rctTypes.h (Monero)
struct clsag {
    rct::keyV s; // scalars (responses), length = ring_size
    rct::key c1; // initial challenge
    rct::key I;  // signing key image (prevents double-spend)
    rct::key D;  // commitment key image (binds to commitment relation)
};
```

字段解释:
- s: 响应标量向量，长度等于环大小 N。对应每个环成员的响应值，用于闭合挑战环。
- c1: 第一个挑战标量，作为整个 Fiat-Shamir 挑战链的起点。
- I: 密钥镜像 (Key Image)，由真实私钥 x 和其公钥 P 经 Hp(P) 生成，确保同一输出被花费时可检测重复，且不暴露真实输入。
- D: 承诺镜像 (Commitment Key Image)，将承诺关系绑定进签名，抵御“Janus/组合”类攻击，确保签名同时链接到承诺的盲因子关系。

注意:
- I 在 prunable 序列化中不保存，可依据输入与环成员重建；D 会被序列化以供验证。

### 1.4 CLSAG 签名算法流程

#### 签名生成 (`CLSAG_Gen` + `proveRctCLSAGSimple`)

**函数定位**: `src/ringct/rctSigs.cpp`
- 核心函数: `CLSAG_Gen()` (L1100-1300)
- 简化接口: `proveRctCLSAGSimple()` (L1800+)

**算法步骤**:

```cpp
// 输入:
// - message: 交易消息哈希
// - P: 环成员公钥向量 [P_0, P_1, ..., P_n]
// - p: 真实私钥 (对应 P[l])
// - C: 承诺向量 [C_0, C_1, ..., C_n] (已减去 C_offset)
// - z: 承诺盲因子
// - C_nonzero: 原始承诺向量 (用于哈希)
// - C_offset: 承诺偏移量 (通常是输出承诺)
// - l: 真实密钥索引 (secret index)

clsag CLSAG_Gen(...) {
    size_t n = P.size(); // 环大小
    
    // 步骤 1: 生成密钥镜像
    ge_p3 H_p3;
    hash_to_p3(H_p3, P[l]);              // H = Hp(P[l])
    key H;
    ge_p3_tobytes(H.bytes, &H_p3);
    
    key D;                                // 承诺密钥镜像
    key I;                                // 签名密钥镜像
    
    // 步骤 2: 初始化随机值 (由硬件设备或软件生成)
    key a, aG, aH;
    hwdev.clsag_prepare(p, z, I, D, H, a, aG, aH);
    // 其中: I = p * H, D = z * H, aG = a*G, aH = a*H
    
    // 步骤 3: 预计算密钥镜像
    geDsmp I_precomp, D_precomp;
    precomp(I_precomp.k, I);
    precomp(D_precomp.k, D);
    
    sig.D = scalarmult8(D);               // D' = 8*D (cofactor 清除)
    
    // 步骤 4: 计算聚合哈希 mu_P, mu_C (域分离)
    keyV mu_P_to_hash(2*n+4);
    keyV mu_C_to_hash(2*n+4);
    
    // 域分离标签
    sc_0(mu_P_to_hash[0].bytes);
    memcpy(mu_P_to_hash[0].bytes, config::HASH_KEY_CLSAG_AGG_0, ...);
    sc_0(mu_C_to_hash[0].bytes);
    memcpy(mu_C_to_hash[0].bytes, config::HASH_KEY_CLSAG_AGG_1, ...);
    
    // 填充公钥和承诺
    for (size_t i = 1; i < n+1; ++i) {
        mu_P_to_hash[i] = P[i-1];
        mu_C_to_hash[i] = P[i-1];
        mu_P_to_hash[i+n] = C_nonzero[i-n-1];
        mu_C_to_hash[i+n] = C_nonzero[i-n-1];
    }
    mu_P_to_hash[2*n+1] = I;
    mu_P_to_hash[2*n+2] = sig.D;
    mu_P_to_hash[2*n+3] = C_offset;
    // mu_C_to_hash 同理...
    
    key mu_P = hash_to_scalar(mu_P_to_hash);
    key mu_C = hash_to_scalar(mu_C_to_hash);
    
    // 步骤 5: 计算初始挑战 c[l+1]
    keyV c_to_hash(2*n+5);  // domain, P, C, C_offset, message, L, R
    sc_0(c_to_hash[0].bytes);
    memcpy(c_to_hash[0].bytes, config::HASH_KEY_CLSAG_ROUND, ...);
    
    for (size_t i = 1; i < n+1; ++i) {
        c_to_hash[i] = P[i-1];
        c_to_hash[i+n] = C_nonzero[i-1];
    }
    c_to_hash[2*n+1] = C_offset;
    c_to_hash[2*n+2] = message;
    c_to_hash[2*n+3] = aG;  // L_initial
    c_to_hash[2*n+4] = aH;  // R_initial
    
    key c;
    hwdev.clsag_hash(c_to_hash, c);
    
    // 步骤 6: 环形计算挑战和响应
    sig.s = keyV(n);
    size_t i = (l + 1) % n;
    if (i == 0) copy(sig.c1, c);  // 保存 c1
    
    key c_new, L, R, c_p, c_c;
    geDsmp P_precomp, C_precomp, H_precomp;
    ge_p3 Hi_p3;
    
    while (i != l) {
        // 为非秘密索引生成随机响应
        sig.s[i] = skGen();
        
        sc_mul(c_p.bytes, mu_P.bytes, c.bytes);  // c_p = c * mu_P
        sc_mul(c_c.bytes, mu_C.bytes, c.bytes);  // c_c = c * mu_C
        
        // 预计算点
        precomp(P_precomp.k, P[i]);
        precomp(C_precomp.k, C[i]);
        
        // 计算 L = s[i]*G + c_p*P[i] + c_c*C[i]
        addKeys_aGbBcC(L, sig.s[i], c_p, P_precomp.k, c_c, C_precomp.k);
        
        // 计算 R = s[i]*Hp(P[i]) + c_p*I + c_c*D
        hash_to_p3(Hi_p3, P[i]);
        ge_dsm_precomp(H_precomp.k, &Hi_p3);
        addKeys_aAbBcC(R, sig.s[i], H_precomp.k, c_p, I_precomp.k, c_c, D_precomp.k);
        
        // 计算下一个挑战
        c_to_hash[2*n+3] = L;
        c_to_hash[2*n+4] = R;
        hwdev.clsag_hash(c_to_hash, c_new);
        copy(c, c_new);
        
        i = (i + 1) % n;
        if (i == 0) copy(sig.c1, c);  // 保存环起点
    }
    
    // 步骤 7: 计算真实索引的响应 (闭合环)
    // s[l] = a - c*(p*mu_P + z*mu_C) mod l
    hwdev.clsag_sign(c, a, p, z, mu_P, mu_C, sig.s[l]);
    
    // 清理敏感数据
    memwipe(&a, sizeof(key));
    
    return sig;  // 返回 (s, c1, I, D)
}
```

#### 签名验证 (`verRctCLSAGSimple`)

**函数定位**: `src/ringct/rctSigs.cpp:L2900+`

**验证步骤**:

```cpp
bool verRctCLSAGSimple(const key &message, const clsag &sig, 
                       const ctkeyV &pubs, const key &C_offset) {
    const size_t n = pubs.size();
    
    // 步骤 1: 数据完整性检查
    CHECK(n >= 1);
    CHECK(n == sig.s.size());
    for (size_t i = 0; i < n; ++i)
        CHECK(sc_check(sig.s[i].bytes) == 0);  // 标量合法性
    CHECK(sc_check(sig.c1.bytes) == 0);
    CHECK(!(sig.I == identity()));  // Key Image 不能是单位元
    
    // 步骤 2: 预处理承诺偏移
    ge_p3 C_offset_p3;
    ge_frombytes_vartime(&C_offset_p3, C_offset.bytes);
    ge_cached C_offset_cached;
    ge_p3_to_cached(&C_offset_cached, &C_offset_p3);
    
    // 步骤 3: 预计算密钥镜像
    key D_8 = scalarmult8(sig.D);
    CHECK(!(D_8 == identity()));
    
    geDsmp I_precomp, D_precomp;
    precomp(I_precomp.k, sig.I);
    precomp(D_precomp.k, D_8);
    
    // 步骤 4: 重建聚合哈希 mu_P, mu_C (与签名时相同)
    keyV mu_P_to_hash(2*n+4);
    keyV mu_C_to_hash(2*n+4);
    // ... (填充逻辑同签名)
    key mu_P = hash_to_scalar(mu_P_to_hash);
    key mu_C = hash_to_scalar(mu_C_to_hash);
    
    // 步骤 5: 设置轮次哈希
    keyV c_to_hash(2*n+5);
    // ... (填充逻辑同签名)
    c_to_hash[2*n+2] = message;
    
    // 步骤 6: 从 c1 开始重建挑战环
    key c = copy(sig.c1);
    key c_p, c_c, c_new, L, R;
    geDsmp P_precomp, C_precomp, hash_precomp;
    ge_p3 hash8_p3, temp_p3;
    ge_p1p1 temp_p1;
    
    size_t i = 0;
    while (i < n) {
        sc_mul(c_p.bytes, mu_P.bytes, c.bytes);
        sc_mul(c_c.bytes, mu_C.bytes, c.bytes);
        
        // 预计算
        precomp(P_precomp.k, pubs[i].dest);
        
        // 计算 C[i] - C_offset
        ge_frombytes_vartime(&temp_p3, pubs[i].mask.bytes);
        ge_sub(&temp_p1, &temp_p3, &C_offset_cached);
        ge_p1p1_to_p3(&temp_p3, &temp_p1);
        ge_dsm_precomp(C_precomp.k, &temp_p3);
        
        // 重建 L 和 R (验证方程)
        // L = s[i]*G + c_p*P[i] + c_c*(C[i] - C_offset)
        addKeys_aGbBcC(L, sig.s[i], c_p, P_precomp.k, c_c, C_precomp.k);
        
        // R = s[i]*Hp(P[i]) + c_p*I + c_c*D
        hash_to_p3(hash8_p3, pubs[i].dest);
        ge_dsm_precomp(hash_precomp.k, &hash8_p3);
        addKeys_aAbBcC(R, sig.s[i], hash_precomp.k, c_p, I_precomp.k, c_c, D_precomp.k);
        
        // 计算下一个挑战
        c_to_hash[2*n+3] = L;
        c_to_hash[2*n+4] = R;
        c_new = hash_to_scalar(c_to_hash);
        CHECK(!(c_new == zero()));
        
        copy(c, c_new);
        i = i + 1;
    }
    
    // 步骤 7: 验证环闭合 (c 应该回到 c1)
    sc_sub(c_new.bytes, c.bytes, sig.c1.bytes);
    return sc_isnonzero(c_new.bytes) == 0;  // c == c1 则通过
}
```

#### 关键数学关系

**签名正确性证明**:

对于真实索引 l, 响应 s[l] 的计算:
```
s[l] = a - c[l]*(p*mu_P + z*mu_C) mod l
```

验证时重建:
```
L[l] = s[l]*G + c_p[l]*P[l] + c_c[l]*C[l]
     = (a - c[l]*(p*mu_P + z*mu_C))*G + c[l]*mu_P*P[l] + c[l]*mu_C*C[l]
     = a*G + c[l]*mu_P*(P[l] - p*G) + c[l]*mu_C*(C[l] - z*G)
     = a*G  (因为 P[l] = p*G, C[l] = z*G)
     = aG (初始值)

R[l] = s[l]*Hp(P[l]) + c_p[l]*I + c_c[l]*D
     = ... (类似推导)
     = aH (初始值)
```

因此验证时会重建出 (L[l], R[l]) = (aG, aH), 从而重建出 c[l+1], 最终闭合环回到 c1.

**关键问题解答**:
- ✅ **Ring members 选择**: 由钱包通过 `get_outs` RPC 从区块链获取, 使用 gamma 分布选择 decoys
- ✅ **Key Image 生成**: I = p * Hp(P), 其中 Hp(P) = hash_to_p3(P) 将公钥哈希到曲线点
- ✅ **验证有效性**: 重建挑战环, 检查 c_final == c1 (环闭合)

### 1.5 代码片段分析

#### 片段 1: 密钥镜像生成 (Key Image Generation)

```cpp
// src/ringct/rctSigs.cpp:L1120-1130
// 从公钥生成 Hash-to-Point
ge_p3 H_p3;
hash_to_p3(H_p3, P[l]);  // H = Hp(P[l])
key H;
ge_p3_tobytes(H.bytes, &H_p3);

// 硬件设备计算 I = p * H, D = z * H
key a, aG, aH;
hwdev.clsag_prepare(p, z, sig.I, D, H, a, aG, aH);
```

**原理**: 
- `hash_to_p3(P)` 将 32 字节公钥 P 确定性映射到椭圆曲线点 Hp(P)
- Key Image I = x * Hp(P) 绑定到私钥 x, 但不暴露 x
- 同一输出再次花费会产生相同的 I (全网可检测双花)

#### 片段 2: 聚合哈希计算 (Aggregation Hashes)

```cpp
// src/ringct/rctSigs.cpp:L1150-1180
// 域分离 (防止跨协议/跨上下文攻击)
keyV mu_P_to_hash(2*n+4);
sc_0(mu_P_to_hash[0].bytes);
memcpy(mu_P_to_hash[0].bytes, 
       config::HASH_KEY_CLSAG_AGG_0,
       sizeof(config::HASH_KEY_CLSAG_AGG_0)-1);

// 绑定所有公钥和承诺到哈希
for (size_t i = 1; i < n+1; ++i) {
    mu_P_to_hash[i] = P[i-1];
    mu_P_to_hash[i+n] = C_nonzero[i-n-1];
}
mu_P_to_hash[2*n+1] = sig.I;
mu_P_to_hash[2*n+2] = sig.D;
mu_P_to_hash[2*n+3] = C_offset;

key mu_P = hash_to_scalar(mu_P_to_hash);
key mu_C = hash_to_scalar(mu_C_to_hash); // 类似
```

**作用**:
- mu_P, mu_C 将所有环成员和承诺绑定到签名
- 防止"混合环"攻击 (不同签名的环成员混淆)
- 域分离标签 `HASH_KEY_CLSAG_AGG_0/1` 防止哈希重用

#### 片段 3: 环形挑战-响应计算 (Ring Challenge-Response Loop)

```cpp
// src/ringct/rctSigs.cpp:L1220-1260
while (i != l) {
    sig.s[i] = skGen();  // 非秘密索引: 随机响应
    
    sc_mul(c_p.bytes, mu_P.bytes, c.bytes);
    sc_mul(c_c.bytes, mu_C.bytes, c.bytes);
    
    precomp(P_precomp.k, P[i]);
    precomp(C_precomp.k, C[i]);
    
    // L = s[i]*G + c_p*P[i] + c_c*C[i]
    addKeys_aGbBcC(L, sig.s[i], c_p, P_precomp.k, c_c, C_precomp.k);
    
    // R = s[i]*Hp(P[i]) + c_p*I + c_c*D
    hash_to_p3(Hi_p3, P[i]);
    ge_dsm_precomp(H_precomp.k, &Hi_p3);
    addKeys_aAbBcC(R, sig.s[i], H_precomp.k, c_p, I_precomp.k, c_c, D_precomp.k);
    
    // 计算下一个挑战
    c_to_hash[2*n+3] = L;
    c_to_hash[2*n+4] = R;
    hwdev.clsag_hash(c_to_hash, c_new);
    
    copy(c, c_new);
    i = (i + 1) % n;
}
```

**Fiat-Shamir 变换**:
- 对于 decoys (非秘密索引), 先选随机 s[i], 再计算 L, R
- 挑战 c 通过哈希链传递: c[i+1] = H(L[i], R[i], ...)
- 真实索引 l 的 s[l] 在环闭合时反推: s[l] = a - c[l]*(...)

#### 片段 4: 真实响应计算 (Signing Index Response)

```cpp
// src/ringct/rctSigs.cpp:L1270
// 环闭合: 计算 s[l] 使得验证方程成立
hwdev.clsag_sign(c, a, p, z, mu_P, mu_C, sig.s[l]);

// 软件实现 (hw::device_default::clsag_sign):
// s[l] = a - c*(p*mu_P + z*mu_C) mod l
sc_mul(tmp1, mu_P, p);         // tmp1 = mu_P * p
sc_mul(tmp2, mu_C, z);         // tmp2 = mu_C * z
sc_add(tmp3, tmp1, tmp2);      // tmp3 = mu_P*p + mu_C*z
sc_mul(tmp4, c, tmp3);         // tmp4 = c * (mu_P*p + mu_C*z)
sc_sub(s, a, tmp4);            // s = a - c*(mu_P*p + mu_C*z)
```

**零知识性**:
- s[l] 混合了随机数 a 和秘密 (p, z)
- 验证者无法从 s[l] 反推 p 或 z
- 只能验证 s[l] 满足签名方程

#### 片段 5: 验证环闭合 (Verification Ring Closure)

```cpp
// src/ringct/rctSigs.cpp:L3100-3150 (verRctCLSAGSimple)
key c = copy(sig.c1);
size_t i = 0;

while (i < n) {
    // 重建 L[i] 和 R[i]
    sc_mul(c_p.bytes, mu_P.bytes, c.bytes);
    sc_mul(c_c.bytes, mu_C.bytes, c.bytes);
    
    addKeys_aGbBcC(L, sig.s[i], c_p, P_precomp.k, c_c, C_precomp.k);
    addKeys_aAbBcC(R, sig.s[i], hash_precomp.k, c_p, I_precomp.k, c_c, D_precomp.k);
    
    // 重建下一个挑战
    c_to_hash[2*n+3] = L;
    c_to_hash[2*n+4] = R;
    c_new = hash_to_scalar(c_to_hash);
    
    copy(c, c_new);
    i++;
}

// 验证环闭合: c 应该回到 c1
sc_sub(c_new.bytes, c.bytes, sig.c1.bytes);
return sc_isnonzero(c_new.bytes) == 0;  // c == c1?
```

**验证逻辑**:
- 从 c1 开始, 依次重建所有 (L[i], R[i])
- 每个 L[i], R[i] 生成下一个挑战 c[i+1]
- 如果签名有效, 最后的 c 会等于起点 c1 (环闭合)
- 否则 c != c1, 签名无效

#### 性能优化技巧

```cpp
// 1. 预计算 (Precomputation)
geDsmp P_precomp;
precomp(P_precomp.k, P[i]);  // 将点转换为 DSM 形式, 加速多标量乘法

// 2. 批量操作 (Batch Operations)
addKeys_aGbBcC(L, s, c_p, P_precomp.k, c_c, C_precomp.k);
// 等价于: L = s*G + c_p*P + c_c*C (一次计算)

// 3. Cofactor 清除
key D_8 = scalarmult8(sig.D);  // D' = 8*D, 确保在素数阶子群
CHECK(!(D_8 == identity()));    // 拒绝小子群攻击
```

**学习笔记**:
- CLSAG 使用 Fiat-Shamir 变换将交互式协议转为非交互式
- 聚合哈希 (mu_P, mu_C) 是 CLSAG 相比 MLSAG 的关键改进
- 承诺密钥镜像 D 防止"Janus/组合"类攻击
- 域分离标签防止跨协议攻击 (如 Monero vs SuperVM 签名混淆) 

---

## 🎭 2. Stealth Address (隐身地址)

### 2.1 核心概念

**隐身地址 = 每笔交易生成唯一的一次性地址**

- **发送方**: 使用接收方公钥生成一次性地址
- **接收方**: 使用私钥扫描区块链识别属于自己的交易

### 2.2 关键文件

- `src/cryptonote_basic/cryptonote_format_utils.cpp`
  - `generate_key_derivation()` - 派生密钥
  - `derive_public_key()` - 派生公钥
- `src/wallet/wallet2.cpp`
  - 钱包扫描逻辑

### 2.3 地址生成流程

```
接收方钱包密钥:
- Spend key pair: (a, A = aG)  - 花费密钥
- View key pair: (b, B = bG)   - 视图密钥

发送方生成一次性地址:
1. 随机生成 r (交易密钥)
2. 计算 R = rG (交易公钥)
3. 计算共享秘密: S = rA = raG
4. 计算一次性公钥: P = H(S, n)G + B
   其中 n 是输出索引

接收方识别:
1. 扫描区块链中的 R
2. 计算共享秘密: S = aR = arG
3. 计算期望公钥: P' = H(S, n)G + B
4. 如果 P' == P, 则该输出属于自己
```

### 2.4 代码片段

**TODO**: 提取 `generate_key_derivation()` 实现

```cpp
// src/cryptonote_basic/cryptonote_format_utils.cpp

```

**关键问题**:
- [ ] 如何高效扫描大量交易?
- [ ] 视图密钥 vs 花费密钥的分离作用?
- [ ] 多个输出如何索引 (n)?

---

## 🔑 2. Stealth Address (隐身地址)

### 2.1 基本概念

**目的**: 每笔交易生成唯一的一次性地址, 防止交易关联, 保护接收方隐私。

**核心思想**: 
- 发送方使用接收方的公钥 + 随机数生成一次性地址
- 接收方通过扫描区块链, 使用私钥恢复属于自己的输出
- 区块链上每个输出地址都不同, 但接收方可以花费

### 2.2 Monero 地址结构

Monero 标准地址包含两对密钥:

```
Address = (A, B)
  A = a*G  (View Public Key, 视图公钥)
  B = b*G  (Spend Public Key, 花费公钥)

用户持有:
  a = view secret key (视图私钥, 用于扫描)
  b = spend secret key (花费私钥, 用于签名)
```

**职责分离**:
- `a` (view key): 只能查看交易, 不能花费 (可分享给审计员)
- `b` (spend key): 花费资金 (绝对保密)

### 2.3 一次性地址生成 (发送方)

发送方要给地址 `(A, B)` 发送资金时:

#### 步骤 1: 生成交易密钥对

```cpp
// src/crypto/crypto.cpp:L150
secret_key r;  // 交易私钥 (transaction secret key)
random_scalar(r);  // 生成随机 256-bit 标量

public_key R;  // 交易公钥 (transaction public key)
secret_key_to_public_key(r, R);  // R = r*G
```

`R` 会被写入交易的 `tx_extra` 字段, 公开可见。

#### 步骤 2: 计算共享密钥 (Diffie-Hellman)

```cpp
// src/crypto/crypto.cpp:L237 - generate_key_derivation()
bool generate_key_derivation(const public_key &A,     // 接收方视图公钥
                              const secret_key &r,     // 交易私钥
                              key_derivation &derivation) {
    ge_p3 point;
    ge_p2 point2;
    ge_p1p1 point3;
    
    if (ge_frombytes_vartime(&point, &A) != 0)
        return false;
    
    // 计算 r*A (Diffie-Hellman 密钥协商)
    ge_scalarmult(&point2, &r, &point);
    
    // 乘以 cofactor 8 (防止小子群攻击)
    ge_mul8(&point3, &point2);
    ge_p1p1_to_p2(&point2, &point3);
    ge_tobytes(&derivation, &point2);
    
    return true;
}
```

**数学原理** (ECDH):
```
derivation = 8 * r * A
           = 8 * r * (a*G)
           = 8 * a * (r*G)
           = 8 * a * R
```

发送方使用 `(r, A)` 计算, 接收方使用 `(a, R)` 计算, 结果相同!

#### 步骤 3: 派生一次性输出公钥

```cpp
// src/crypto/crypto.cpp:L258 - derive_public_key()
bool derive_public_key(const key_derivation &derivation,
                       size_t output_index,     // 输出索引 n
                       const public_key &B,     // 接收方花费公钥
                       public_key &P_out) {     // 一次性公钥
    ec_scalar scalar;
    ge_p3 point1, point2;
    ge_cached point3;
    ge_p1p1 point4;
    ge_p2 point5;
    
    if (ge_frombytes_vartime(&point1, &B) != 0)
        return false;
    
    // 计算 Hs(derivation || output_index)
    derivation_to_scalar(derivation, output_index, scalar);
    
    // 计算 Hs(...)*G
    ge_scalarmult_base(&point2, &scalar);
    
    // P_out = Hs(...)*G + B
    ge_p3_to_cached(&point3, &point2);
    ge_add(&point4, &point1, &point3);
    ge_p1p1_to_p2(&point5, &point4);
    ge_tobytes(&P_out, &point5);
    
    return true;
}
```

**数学公式**:
```
P_out = Hs(8*r*A || n)*G + B
```

其中 `Hs()` 是 hash-to-scalar 函数.

### 2.4 扫描输出 (接收方)

接收方扫描区块链, 对每笔交易:

#### 步骤 1: 提取交易公钥 R

```cpp
// src/cryptonote_basic/cryptonote_format_utils.cpp
crypto::public_key tx_pub_key = get_tx_pub_key_from_extra(tx);
```

#### 步骤 2: 重建共享密钥

```cpp
crypto::key_derivation derivation;
acc.get_device().generate_key_derivation(R, acc.m_view_secret_key, derivation);
// derivation = 8*a*R = 8*r*A (与发送方相同)
```

#### 步骤 3: 逐个检查输出

```cpp
for (size_t n = 0; n < tx.vout.size(); ++n) {
    crypto::public_key P_blockchain = get_output_public_key(tx.vout[n]);
    
    // 重建期望的公钥
    crypto::public_key P_expected;
    acc.get_device().derive_public_key(derivation, n, 
                                       acc.m_spend_public_key, 
                                       P_expected);
    
    if (P_blockchain == P_expected) {
        // 这个输出属于我!
        outs.push_back(n);
    }
}
```

### 2.5 花费输出 (派生一次性私钥)

```cpp
// src/crypto/crypto.cpp:L278 - derive_secret_key()
void derive_secret_key(const key_derivation &derivation,
                       size_t output_index,
                       const secret_key &b,      // 花费私钥
                       secret_key &p_out) {      // 一次性私钥
    ec_scalar scalar;
    
    derivation_to_scalar(derivation, output_index, scalar);
    sc_add(&p_out, &b, &scalar);  // p_out = Hs(...) + b
}
```

**数学验证**:
```
P_out = p_out * G
      = (Hs(8*r*A || n) + b) * G
      = Hs(8*r*A || n)*G + b*G
      = Hs(8*r*A || n)*G + B  ✅
```

### 2.6 View Tags 优化 (Monero v15+)

为减少扫描计算量 (256倍提速):

```cpp
// src/crypto/crypto.cpp:L650 - derive_view_tag()
void derive_view_tag(const key_derivation &derivation,
                     size_t output_index,
                     view_tag &view_tag) {  // 只有 1 字节!
    struct {
        char salt[8];  // "view_tag"
        key_derivation derivation;
        char output_index_varint[...];
    } buf;
    
    memcpy(buf.salt, "view_tag", 8);
    buf.derivation = derivation;
    tools::write_varint(end, output_index);
    
    hash view_tag_full;
    cn_fast_hash(&buf, ..., view_tag_full);
    
    // 只取前 1 字节
    memcpy(&view_tag, &view_tag_full, 1);
}
```

**优化原理**:
1. 发送方计算 view tag 并附加到输出 (1字节)
2. 接收方先检查 view tag (简单比较)
3. 只有匹配 (1/256 概率) 才做完整的 `derive_public_key`
4. 平均减少 99.6% 的计算量

### 2.7 关键问题解答

✅ **为什么需要两对密钥?**
- `a` (view key): 轻钱包/审计员可扫描, 但不能花费
- `b` (spend key): 冷钱包保管, 只在签名时需要

✅ **为什么要乘以 cofactor 8?**
- Ed25519 曲线 cofactor=8, 确保结果在素数阶子群
- 防止小子群攻击 (Lim-Lee attack)

✅ **如何防止地址重用?**
- 每笔交易生成新的随机 `r`
- 即使同一接收方, 每个 `P_out` 都不同
- 区块链分析无法关联输出

---

## 🔑 3. Key Image (密钥镜像)

### 3.1 作用

**防止双花攻击**: Key Image 是从私钥派生的唯一值,全网可见

- 每个输出有唯一的 Key Image
- 花费输出时必须提供 Key Image
- 网络拒绝重复的 Key Image

### 3.2 生成算法

```
输入: 
- x: 一次性私钥
- P: 对应的一次性公钥 (P = xG)

输出:
- I: Key Image

计算:
I = x * Hp(P)

其中 Hp(P) 是 "hash-to-point" 函数
```

### 3.3 关键函数实现

```cpp
// src/crypto/crypto.cpp:L620 - generate_key_image()
void generate_key_image(const public_key &pub,
                        const secret_key &sec,
                        key_image &image) {
    ge_p3 point;
    ge_p2 point2;
    
    // 计算 Hp(P)
    hash_to_ec(pub, point);  // 将公钥哈希到椭圆曲线点
    
    // 计算 x * Hp(P)
    ge_scalarmult(&point2, &sec, &point);
    ge_tobytes(&image, &point2);
}

// hash_to_ec 实现:
static void hash_to_ec(const public_key &key, ge_p3 &res) {
    hash h;
    ge_p2 point;
    ge_p1p1 point2;
    
    // 哈希公钥
    cn_fast_hash(&key, sizeof(public_key), h);
    
    // 解码哈希到点 (Elligator)
    ge_fromfe_frombytes_vartime(&point, (const unsigned char*)&h);
    
    // 乘以 cofactor 8
    ge_mul8(&point2, &point);
    ge_p1p1_to_p3(&res, &point2);
}
```

**数学性质**:
- `I = x * Hp(P)` 绑定到私钥 `x`, 但不泄露 `x`
- 同一输出再次花费会产生相同的 `I` (全网可检测)

### 3.4 安全性

**为什么不能直接用公钥?**
- 公钥会泄露环签名中的真实输出
- Key Image 通过 hash-to-point 打破关联性

**为什么攻击者不能伪造?**
- 只有知道私钥 x 才能计算 I = x * Hp(P)
- 环签名同时证明签名者知道某个私钥

---

## � 4. Bulletproofs 范围证明

### 4.1 基本概念

**目的**: 在不泄露金额的情况下, 证明交易金额 `v` 在合法范围内 (0 ≤ v < 2^64)。

**核心思想**:
- Pedersen Commitment 隐藏金额: `C = v*H + gamma*G`
- Bulletproofs 证明 `v ∈ [0, 2^N)` 且不泄露 `v` 或 `gamma`
- 聚合证明: 多个输出共享一个证明, 指数级减少大小

**关键特性**:
- **证明大小**: 2*log₂(n*m) + 9 个椭圆曲线点 (~700 bytes for 2 outputs)
- **验证复杂度**: 批量验证 O(n + m*log(m)), 单个 O(n*log(n))
- **无需可信设置** (相比 zk-SNARKs)

### 4.2 Pedersen Commitment 基础

#### 承诺计算

```rust
// Pedersen Commitment: C = v*H + gamma*G
// v: 金额 (保密)
// gamma: 盲化因子 (随机, 保密)
// H, G: 基点 (公开)

let commitment = v * H + gamma * G;
```

**同态性** (Homomorphic Property):
```
C₁ + C₂ = (v₁*H + γ₁*G) + (v₂*H + γ₂*G)
        = (v₁ + v₂)*H + (γ₁ + γ₂)*G
```

**应用**: 交易验证
```
输入承诺之和 = 输出承诺之和 + 手续费*H
Σ C_in = Σ C_out + fee*H
```

### 4.3 Bulletproofs 证明生成

Monero 使用的是 **aggregated range proof** (多输出聚合证明)。

#### 步骤 1: 初始化 (PAPER LINES 41-44)

```cpp
// src/ringct/bulletproofs.cc:L800 - bulletproof_PROVE()

constexpr size_t N = 64;    // 比特位数 (2^64 范围)
size_t M = outputs.size();  // 输出数量
size_t MN = M * N;

// 将金额编码为比特向量 aL, aR
for (size_t j = 0; j < M; ++j) {
    for (size_t i = 0; i < N; ++i) {
        if (v[j] & (1 << i)) {
            aL[j*N + i] = 1;   // 比特为 1
            aR[j*N + i] = 0;
        } else {
            aL[j*N + i] = 0;   // 比特为 0
            aR[j*N + i] = -1;  // aR = aL - 1
        }
    }
}

// 生成承诺 V = v*H + gamma*G
for (size_t i = 0; i < M; ++i) {
    V[i] = addKeys2(gamma[i] / 8, v[i] / 8, H);  // 除以8是cofactor处理
}
```

**aL, aR 关系**:
```
aL[i] ∈ {0, 1}         (比特值)
aR[i] = aL[i] - 1 ∈ {-1, 0}
aL ⊙ aR = 0            (Hadamard 积为0)
Σ(aL[i] * 2^i) = v     (二进制重建金额)
```

#### 步骤 2: 向量承诺 A, S (PAPER LINES 43-47)

```cpp
// 生成随机向量 sL, sR (用于零知识)
sL = random_vector(MN);
sR = random_vector(MN);

// A = aL*G + aR*H + alpha*G (第一个承诺)
alpha = random_scalar();
A = vector_exponent(aL, aR) + alpha * G;

// S = sL*G + sR*H + rho*G (第二个承诺, 用于多项式)
rho = random_scalar();
S = vector_exponent(sL, sR) + rho * G;
```

#### 步骤 3: Fiat-Shamir 挑战 (PAPER LINES 48-50)

```cpp
// 从 V, A, S 派生挑战 (非交互式)
y = H(V || A || S)
z = H(y)
```

#### 步骤 4: 多项式构造 (PAPER LINES 58-63)

```cpp
// 构造多项式 l(X), r(X)
// l(X) = (aL - z*1) + sL*X
// r(X) = y^n ⊙ (aR + z*1 + sR*X) + z²*2^n

l0 = aL - z;
l1 = sL;

y_powers = [1, y, y², ..., y^(MN-1)];
r0 = (aR + z) ⊙ y_powers + z² * [2⁰, 2¹, ..., 2^(N-1)];
r1 = y_powers ⊙ sR;

// 计算多项式系数
// t(X) = <l(X), r(X)> = t₀ + t₁*X + t₂*X²
t1 = <l0, r1> + <l1, r0>;
t2 = <l1, r1>;
```

#### 步骤 5: 多项式承诺 T1, T2 (PAPER LINES 52-53)

```cpp
tau1 = random_scalar();
tau2 = random_scalar();

T1 = t1*H / 8 + tau1*G / 8;
T2 = t2*H / 8 + tau2*G / 8;
```

#### 步骤 6: 挑战与响应 (PAPER LINES 54-63)

```cpp
// 新挑战
x = H(z || T1 || T2);

// 计算 taux (盲化因子的线性组合)
taux = tau1*x + tau2*x² + Σ(z^(j+2) * gamma[j]);

// 计算 mu (用于内积证明)
mu = x*rho + alpha;

// 计算 l, r (在 x 处求值)
l = l0 + l1*x;
r = r0 + r1*x;

// 计算 t (内积)
t = <l, r>;
```

#### 步骤 7: 内积证明 (Inner Product Argument)

这是 Bulletproofs 的核心递归算法:

```cpp
// src/ringct/bulletproofs.cc:L1100 - 内积证明循环

nprime = MN;  // 初始向量长度
L[], R[] = [];  // 左右承诺数组

while (nprime > 1) {
    nprime /= 2;
    
    // 计算交叉内积
    cL = <a[0:nprime], b[nprime:2*nprime]>;
    cR = <a[nprime:2*nprime], b[0:nprime]>;
    
    // 计算左右承诺
    L = Σ(a[0:nprime] * G[nprime:2*nprime]) 
      + Σ(b[nprime:2*nprime] * H[0:nprime]) 
      + cL * x_ip * H;
      
    R = Σ(a[nprime:2*nprime] * G[0:nprime]) 
      + Σ(b[0:nprime] * H[nprime:2*nprime]) 
      + cR * x_ip * H;
    
    // Fiat-Shamir 挑战
    w = H(L || R);
    
    // 折叠向量 (递归压缩)
    a' = w*a[0:nprime] + w⁻¹*a[nprime:2*nprime];
    b' = w⁻¹*b[0:nprime] + w*b[nprime:2*nprime];
    
    // 折叠基点
    G' = w⁻¹*G[0:nprime] + w*G[nprime:2*nprime];
    H' = w*H[0:nprime] + w⁻¹*H[nprime:2*nprime];
    
    a = a'; b = b'; G = G'; H = H';
}

// 最终返回标量 a, b (长度为1)
```

**证明结构**:
```rust
struct Bulletproof {
    V: Vec<Point>,       // 承诺向量 (M个)
    A: Point,            // 向量承诺 A
    S: Point,            // 向量承诺 S
    T1: Point,           // 多项式承诺 T1
    T2: Point,           // 多项式承诺 T2
    taux: Scalar,        // 盲化因子
    mu: Scalar,          // 内积盲化因子
    L: Vec<Point>,       // 左承诺 (log₂(MN)个)
    R: Vec<Point>,       // 右承诺 (log₂(MN)个)
    a: Scalar,           // 最终 a
    b: Scalar,           // 最终 b
    t: Scalar,           // 最终内积
}
```

**大小计算**:
```
M=2 outputs (128 bits total):
- V: 2 * 32 = 64 bytes
- A, S, T1, T2: 4 * 32 = 128 bytes
- taux, mu, a, b, t: 5 * 32 = 160 bytes
- L, R: 2 * log₂(128) * 32 = 2 * 7 * 32 = 448 bytes
Total: ~800 bytes

对比: 原始 RingCT (non-Bulletproofs): ~7 KB
节省: 89% 空间
```

### 4.4 Bulletproofs 验证

验证方程 (PAPER LINE 62-65):

```cpp
// src/ringct/bulletproofs.cc:L1400 - bulletproof_VERIFY()

// 重建挑战
y = H(V || A || S);
z = H(y);
x = H(z || T1 || T2);
x_ip = H(x || taux || mu || t);
w[] = [H(L[0] || R[0]), H(L[1] || R[1]), ...];

// 验证方程 1: Pedersen Commitment 平衡
// g^(-z) * h^(z*y^n + z²*2^n) * V₁^(z²) * V₂^(z⁴) == g^(-taux) * h^t * T1^x * T2^(x²)

lhs = -z*G + (z*y^n + z²*2^n)*H 
    + z²*V[0] + z⁴*V[1]  // 多输出聚合
    
rhs = -taux*G + t*H + x*T1 + x²*T2;

CHECK(lhs == rhs);  // 方程 1

// 验证方程 2: 内积证明
// 重建基点 G', H'
for (i = 0; i < log₂(MN); i++) {
    G' = w[i]⁻¹*G[left] + w[i]*G[right];
    H' = w[i]*H[left] + w[i]⁻¹*H[right];
}

// 检查最终内积
lhs = a*G' + b*H' + (a*b)*x_ip*H;
rhs = mu*G + Σ(w[i]²*L[i]) + Σ(w[i]⁻²*R[i]) + ...;

CHECK(lhs == rhs);  // 方程 2
```

### 4.5 批量验证优化

Monero 支持批量验证多个 Bulletproofs (区块中所有交易):

```cpp
// src/ringct/bulletproofs.cc:L1300 - bulletproof_VERIFY(batch)

// 为每个证明生成随机权重
weight_y[] = random();
weight_z[] = random();

// 聚合所有验证方程 (加权和)
aggregate_lhs = Σ(weight_y[i] * lhs[i]) + Σ(weight_z[i] * lhs2[i]);
aggregate_rhs = Σ(weight_y[i] * rhs[i]) + Σ(weight_z[i] * rhs2[i]);

// 单次多标量乘法检查
CHECK(aggregate_lhs == aggregate_rhs);
```

**性能提升**:
- 单个验证: ~5ms (1 output)
- 批量验证 1000 proofs: ~1.2s (平均 1.2ms/proof)
- **提速**: 4倍

### 4.6 关键问题解答

✅ **为什么需要两个向量 aL, aR?**
- `aL ∈ {0,1}` 表示比特值
- `aR = aL - 1 ∈ {-1,0}` 确保 `aL ⊙ aR = 0`
- 这个约束隐式证明了 aL 是二进制

✅ **为什么要递归折叠?**
- 初始向量长度 MN (例如 128)
- 每轮折叠减半: 128 → 64 → 32 → ... → 1
- 证明大小: O(log MN) 而非 O(MN)

✅ **Bulletproofs vs Bulletproofs+?**
- Bulletproofs+: Monero v15+ 使用
- 改进: 减少 1 个标量 (weighted norm argument)
- 节省: ~32 bytes/proof

✅ **如何防止负数?**
- 范围证明强制 `v ∈ [0, 2^N)`
- 负数的二进制表示会溢出 N 位
- 验证方程会失败

---

## �🔄 5. RingCT 完整交易流程

### 4.1 交易结构

```cpp
// src/ringct/rctTypes.h
struct rctSig {
    rctSigBase base;        // 基础签名
    vector<clsag> p;        // 环签名 (每个输入一个)
    vector<rangeSig> rangeSigs; // 范围证明 (Bulletproofs)
    // ...
};
```

### 4.2 交易构造步骤

**TODO**: 详细分析 `construct_tx_and_get_tx_key()`

```
1. 选择输入 (UTXOs)
2. 为每个输入选择 ring members
3. 生成输出的隐身地址
4. 生成 Pedersen Commitments (隐藏金额)
5. 生成 Bulletproofs (证明金额 ≥ 0)
6. 生成 CLSAG 签名 (证明拥有某个输入)
7. 验证 Commitment 平衡 (输入 = 输出 + 手续费)
```

### 4.3 承诺方案

**Pedersen Commitment**:
```
C(a, r) = aH + rG

其中:
- a: 金额 (secret)
- r: 盲因子 (blinding factor)
- H, G: 基点
```

**平衡验证**:
```
sum(C_inputs) = sum(C_outputs) + fee * H
```

### 4.4 关键问题

- [ ] Ring size 如何选择? (当前默认 16)
- [ ] Ring members 选择算法? (gamma 分布)
- [ ] Bulletproofs 聚合如何工作?
- [ ] 手续费如何计算?

---

## 📊 5. 性能数据

### 5.1 签名/验证时间

**TODO**: 运行 Monero 基准测试

```bash
# Clone Monero repository and run performance tests
git clone https://github.com/monero-project/monero.git
cd monero
# 编译并运行性能测试
```

**预期数据** (Ring Size = 16):
- 签名生成: ~50-100ms
- 签名验证: ~5-10ms
- Bulletproofs 生成: ~200-300ms
- Bulletproofs 验证: ~5-10ms (批量验证更快)

### 5.2 交易大小

| Ring Size | CLSAG Size | Bulletproofs | 总大小 (估算) |
|-----------|-----------|--------------|---------------|
| 11 | ~1.5 KB | ~1.5 KB | ~3 KB |
| 16 | ~2 KB | ~1.5 KB | ~3.5 KB |
| 64 | ~7 KB | ~1.5 KB | ~8.5 KB |

---

## 🎯 6. 应用到 SuperVM

### 6.1 设计决策

**需要决定**:
1. **Ring Size**: Monero 用 16, 我们用多少?
   - 更大 = 更匿名, 但更慢/更大
   - 建议: 11 (默认), 支持 3-64 可配置

2. **签名算法**: CLSAG vs MLSAG?
   - 选择: CLSAG (更快, 更小)

3. **Range Proof**: Bulletproofs vs Bulletproofs+?
   - 选择: Bulletproofs (curve25519-dalek 已支持)

4. **zkSNARK**: 是否额外集成?
   - 待评估: Week 3-4 决策

### 6.2 API 设计草案

```rust
// 基于 Monero 学习成果设计

pub struct RingSigner {
    ring_size: usize,
    ring_members: Vec<PublicKey>,
    secret_index: usize,
    secret_key: SecretKey,
}

impl RingSigner {
    pub fn sign(&self, message: &[u8]) -> Result<RingSignature> {
        // 1. 生成 Key Image
        let key_image = self.generate_key_image();
        
        // 2. 执行 CLSAG 签名算法
        // (参考 Monero proveRctCLSAGSimple)
        
        todo!()
    }
}
```

### 6.3 实现路线图

**Week 9-12** (Phase 2.2.1):
- [ ] 实现 `generate_key_image()` (基于 Monero)
- [ ] 实现 CLSAG 签名算法
- [ ] 实现 CLSAG 验证算法
- [ ] 性能测试: 目标 <50ms 签名, <5ms 验证

---

## 📚 7. 参考资料

### 7.1 必读论文

- [ ] **Zero to Monero 2.0** (450 页完整教程)
  - https://web.getmonero.org/library/Zero-to-Monero-2-0-0.pdf
  - 章节重点: Ch3 (Ring Signatures), Ch4 (Stealth Addresses), Ch5 (RingCT)

- [ ] **CryptoNote v2.0 Whitepaper**
  - https://cryptonote.org/whitepaper.pdf
  - 原始隐私币设计

- [ ] **Triptych: Logarithmic-Sized Linkable Ring Signatures**
  - https://eprint.iacr.org/2020/018
  - 下一代环签名算法

### 7.2 Monero 文档

- **官方文档**: https://www.getmonero.org/resources/developer-guides/
- **Moneropedia**: https://www.getmonero.org/resources/moneropedia/
- **StackExchange**: https://monero.stackexchange.com/

> **引用说明**: 本文档引用的所有外部资料（论文、项目、文档）已在 [ATTRIBUTIONS.md](../ATTRIBUTIONS.md) 中详细列出，包括版权声明与致谢。

### 7.3 代码导航

**核心目录**:
```
monero-research/
├── src/
│   ├── ringct/              ← RingCT 实现 (重点!)
│   │   ├── rctSigs.cpp      ← CLSAG/MLSAG 签名
│   │   ├── rctTypes.h       ← 数据结构
│   │   └── bulletproofs.cc  ← 范围证明
│   ├── crypto/              ← 基础密码学
│   │   ├── crypto.cpp       ← Ed25519, Key Image
│   │   └── hash-ops.h       ← 哈希函数
│   ├── cryptonote_basic/    ← CryptoNote 核心
│   │   └── cryptonote_format_utils.cpp ← 地址生成
│   └── wallet/              ← 钱包逻辑
│       └── wallet2.cpp      ← 交易构造, 扫描
```

---

## ✅ 学习进度

### Week 1 (2025-03-03 至 2025-03-09)

**Day 1-3 (2025-03-03 ~ 2025-03-05)**:
- [x] 克隆 Monero 仓库
- [x] 创建学习笔记框架
- [x] 阅读 `rctTypes.h` 了解数据结构
- [x] 研究 Ring Signature 基本原理
- [x] 实现 Key Image 机制（应用于 zk-groth16-test）
- [x] 完成环签名电路实现与测试

**Day 2-3**:
- [ ] 深入 `rctSigs.cpp` - CLSAG 实现
- [ ] 提取关键代码片段到笔记
- [ ] 画出 CLSAG 签名流程图

**Day 4-5**:
- [ ] 研究 Stealth Address 实现
- [ ] 研究 Key Image 生成
- [ ] 运行 Monero 测试用例

**Day 6-7**:
- [ ] 总结 Week 1 学习成果
- [ ] 准备 Week 2 深入研究计划

### Week 2 (2025-03-10 至 2025-03-16)

**Day 8-10**:
- [ ] 研究 Bulletproofs 实现
- [ ] 研究 RingCT 完整交易流程
- [ ] 编写 C++ 测试代码验证理解

**Day 11-12**:
- [ ] 设计 SuperVM 的 Ring Signature API
- [ ] 编写技术选型报告
- [ ] 确定实现细节 (ring size, 算法选择)

**Day 13-14**:
- [ ] 完成 Monero 学习总结报告
- [ ] 准备 Week 3 zkSNARK 评估

---

## 💡 问题与思考

### 待解决问题

1. **Ring Member 选择算法**
   - Monero 使用 gamma 分布选择 decoys
   - 如何防止统计分析攻击?

2. **性能优化**
   - 批量验证如何实现?
   - 是否需要预计算表?

3. **存储优化**
   - Key Image 索引结构?
   - 如何高效检测双花?

### 个人理解

**TODO**: 每天记录学习心得

---

## 🔗 相关笔记

- `curve25519-dalek-notes.md` (Week 1-2 并行学习)
- `cryptonote-whitepaper-notes.md` (Week 1-2 并行学习)
- `phase2-implementation-decisions.md` (Week 7-8 架构设计)

---

**创建日期**: 2025-03-03  
**最后更新**: 2025-11-06  
**维护说明**: 本文档为学习笔记框架，随研究进展持续更新
