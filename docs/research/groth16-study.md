# Groth16 zkSNARK 深度学习笔记
开发者/作者：King Xujue
**Phase 2 Week 3 - Day 1**

## 概览

Groth16 是 2016 年 Jens Groth 提出的 zkSNARK 证明系统，相比 Pinocchio 有显著性能提升：
- **证明大小**: 仅 3 个群元素 (2个G1 + 1个G2) ≈ 128 bytes
- **验证速度**: 仅需 3 次配对运算 (vs Pinocchio 的 12 次)
- **应用**: Zcash Sapling, Filecoin, Loopring 等

---

## 核心概念

### 1. R1CS (Rank-1 Constraint System)
将计算转换为约束系统：

**约束格式**: `A · w ⊙ B · w = C · w`
- `w`: 见证向量 (witness vector) = `(1, w₁, w₂, ..., wₘ)`
- `A, B, C`: 约束矩阵
- `⊙`: Hadamard 积 (逐元素乘法)

**示例**: 证明 `x³ + x + 5 = 35`
```
变量: w = (1, out, x, v₁)
其中:
  v₁ = x * x      (中间变量)
  out = v₁ * x + x + 5

约束1 (v₁ = x²):
  A = [0, 0, 1, 0]  // x
  B = [0, 0, 1, 0]  // x  
  C = [0, 0, 0, 1]  // v₁
  => x · x = v₁

约束2 (out = v₁·x + x + 5):
  A = [0, 0, 1, 1]  // x + v₁
  B = [1, 0, 1, 0]  // 1 + x
  C = [5, 1, 0, 0]  // 5 + out
  => (x + v₁) · (1 + x) = 5 + out
```

### 2. QAP (Quadratic Arithmetic Program)
将 R1CS 转换为多项式形式：

**转换公式**:
```
A·w ⊙ B·w = C·w  (n个约束)
    ↓
A(τ) · B(τ) = C(τ) + H(τ) · Z(τ)

其中:
- A(X) = Σ wᵢ · Aᵢ(X)
- B(X) = Σ wᵢ · Bᵢ(X)  
- C(X) = Σ wᵢ · Cᵢ(X)
- Z(X) = (X-1)(X-2)(X-3)···(X-n)  (目标多项式)
- H(X) = [A(X)·B(X) - C(X)] / Z(X)  (商多项式)
```

**关键性质**:
- 如果约束满足, 则 `A(X)·B(X) - C(X)` 必被 `Z(X)` 整除
- 在随机点 `τ` 求值验证 (Schwartz-Zippel 引理)

### 3. Trusted Setup (CRS)
生成公共参考字符串, 包含"毒性废料" (toxic waste):

**秘密参数**: `α, β, γ, δ, τ`
- **必须销毁**: 任何人知道这些值都能伪造证明
- **一次性**: 每个电路需要单独 Setup

**Proving Key (证明密钥)**:
- G1 元素: 
  ```
  {α, δ, 1, τ, τ², τ³, ..., τⁿ⁻¹,
   Lₗ₊₁(τ)/δ, Lₗ₊₂(τ)/δ, ..., Lₘ(τ)/δ,
   Z(τ)/δ, τ·Z(τ)/δ, τ²·Z(τ)/δ, ..., τⁿ⁻²·Z(τ)/δ}₁
  ```
- G2 元素:
  ```
  {β, δ, 1, τ, τ², τ³, ..., τⁿ⁻¹}₂
  ```
- 电路多项式系数: `A₁(X), A₂(X), ..., B₁(X), B₂(X), ..., C₁(X), C₂(X), ...`

**Verification Key (验证密钥)**:
- G1 元素: `{1, L₀(τ)/γ, L₁(τ)/γ, ..., Lₗ(τ)/γ}₁`
- G2 元素: `{1, γ, δ}₂`
- Gt 元素 (预计算配对): `α₁ * β₂`

---

## Groth16 证明系统

### 证明生成

**输入**:
- 见证向量 `w = (1, w₁, w₂, ..., wₘ)`
- 随机数 `r, s ∈ 𝔽ₚ`

**输出**: 证明 `π = (A, B, C)` (3个群元素)

**计算公式**:
```rust
// A ∈ G1
A₁ = α₁ + Σ wᵢ·Aᵢ(τ)₁ + r·δ₁

// B ∈ G2  
B₂ = β₂ + Σ wᵢ·Bᵢ(τ)₂ + s·δ₂

// C ∈ G1
C₁ = Σ(wᵢ·Lᵢ(τ)/δ)₁  (l+1 到 m)
   + H(τ)·(Z(τ)/δ)₁
   + s·A₁ + r·B₁ - r·s·δ₁

其中:
- Lᵢ(X) = β·Aᵢ(X) + α·Bᵢ(X) + Cᵢ(X)
- H(X) = [A(X)·B(X) - C(X)] / Z(X)
```

**步骤**:
1. 从见证 `w` 计算多项式 `A(X), B(X), C(X)`
2. 计算商多项式 `H(X) = [A(X)·B(X) - C(X)] / Z(X)`
3. 使用 CRS 中的加密值计算 `A, B, C`
4. 添加随机盲化因子 `r, s` (零知识性)

### 证明验证

**输入**:
- 公开输入 `w₀, w₁, ..., wₗ`
- 证明 `π = (A, B, C)`

**验证方程**:
```
e(A, B) = e(α, β) · e(Σ wᵢ·Lᵢ(τ)/γ, γ) · e(C, δ)
              ↑          ↑                    ↑
            预计算    公开输入部分          证明部分
```

**仅需 3 次配对**:
- `e(A, B)` - 左边
- `e(公开输入, γ)` - 右边第2项
- `e(C, δ)` - 右边第3项
- `e(α, β)` - 预计算, 不计入

**正确性证明**:
```
左边:
e(A, B) = e(α + A(τ) + r·δ, β + B(τ) + s·δ)
        = A(τ)·B(τ) + α·β + α·B(τ) + β·A(τ) 
          + s·α·δ + s·A(τ)·δ + r·β·δ + r·B(τ)·δ + s·r·δ²

右边:
e(α,β) · e(L(τ)/γ, γ) · e(C, δ)
= α·β + L(τ) + [H(τ)·Z(τ) + s·A(τ) + r·B(τ) + r·s·δ - r·s·δ]·δ
= α·β + β·A(τ) + α·B(τ) + C(τ) + H(τ)·Z(τ)
  + s·α·δ + s·A(τ)·δ + r·β·δ + r·B(τ)·δ + s·r·δ²

如果 A(τ)·B(τ) = C(τ) + H(τ)·Z(τ), 则两边相等 ✓
```

---

## 关键创新

### vs Pinocchio
1. **不使用 "Knowledge of Coefficient"**
   - Pinocchio: 每个多项式需要 2 个群元素证明 "知道系数"
   - Groth16: 使用 α, β 强制 A, B, C 使用相同的 w

2. **分离公开输入**
   - 使用 γ, δ 使公开输入独立于私有见证
   - 验证者只需处理公开输入部分

3. **证明更小**
   - Pinocchio: 7×G1 + 1×G2 ≈ 256 bytes
   - Groth16: 2×G1 + 1×G2 ≈ 128 bytes

---

## 配对友好曲线

### BN254 (aka BN128)
```rust
// 基域 𝔽ₚ (素数 p)
p = 21888242871839275222246405745257275088696311157297823662689037894645226208583

// 嵌入度 k=12
// G1: E(𝔽ₚ) - 普通椭圆曲线点
// G2: E(𝔽ₚ¹²) - 扩域上的点
// Gt: 𝔽ₚ¹² - 配对结果

// 曲线方程
E: y² = x³ + 3
```

**性能**:
- G1 点大小: 32 bytes (压缩)
- G2 点大小: 64 bytes (压缩)
- 配对运算: ~2-3ms (优化实现)

### BLS12-381 (Zcash 新标准)
```rust
p = 4002409555221667393417789825735904156556882819939007885332058136124031650490837864442687629129015664037894272559787

// 更高安全性 (~128-bit vs BN254 的 ~100-bit)
// 更适合递归证明 (Halo2)
```

---

## 实际应用示例

### 电路: 证明知道哈希原像
```rust
// 公开: hash = SHA256(secret)
// 私有: secret
// 电路: 验证 SHA256(secret) == hash

// R1CS 约束:
// SHA256 = ~25,000 个约束
// (每个布尔运算/加法/异或 → 约束)

约束数量估算:
- SHA256: ~25K
- Pedersen Hash: ~5K  
- ECDSA 签名验证: ~150K
- Merkle Tree (深度20): ~20K
```

### 性能数据 (BN254 曲线)
| 电路复杂度 | Setup 时间 | 证明时间 | 验证时间 | 证明大小 |
|-----------|-----------|---------|---------|---------|
| 10K 约束 | ~5s | ~2s | ~5ms | 128 bytes |
| 100K 约束 | ~30s | ~10s | ~5ms | 128 bytes |
| 1M 约束 | ~5min | ~60s | ~5ms | 128 bytes |

**关键**: 验证时间和证明大小**不随电路规模增长** ✨

---

## SuperVM 应用场景

### 场景 1: 隐藏转账金额
```rust
// 公开: 
// - 输入承诺: C_in = v_in·H + r_in·G
// - 输出承诺: C_out1, C_out2
// - 手续费: fee (明文)

// 私有:
// - 输入金额: v_in
// - 输出金额: v_out1, v_out2
// - 盲化因子: r_in, r_out1, r_out2

// 电路约束:
// 1. v_in = v_out1 + v_out2 + fee  (平衡)
// 2. 0 ≤ v_in < 2^64                (范围证明)
// 3. 0 ≤ v_out1 < 2^64
// 4. 0 ≤ v_out2 < 2^64
// 5. C_in = v_in·H + r_in·G         (承诺打开)
// 6. C_out1 = v_out1·H + r_out1·G
// 7. C_out2 = v_out2·H + r_out2·G

约束数量: ~15K (3个范围证明 + 承诺验证)
证明时间: ~3s
验证时间: ~5ms ✓ (可接受)
```

**对比 Bulletproofs**:
- Groth16: 证明 128 bytes, 验证 ~5ms
- Bulletproofs: 证明 ~700 bytes, 验证 ~10ms
- **Groth16 更适合链上验证!**

### 场景 2: 私有智能合约
```rust
// 公开:
// - 合约状态根: state_root
// - 状态转换: state_root' (新根)

// 私有:
// - 合约代码执行 trace
// - 输入参数

// 电路: zkVM 执行验证
// 约束数量: ~1M+ (复杂合约)
// 证明时间: ~60s
// 验证时间: ~5ms ✓
```

---

## Trusted Setup 问题

### 风险
如果秘密参数 `(α, β, γ, δ, τ)` 泄露:
- 攻击者可伪造任意证明
- 破坏整个系统安全性

### 解决方案: MPC Ceremony (多方计算仪式)
```
参与者 1: 生成 τ₁, 计算 {τ₁, τ₁², ...} → 删除 τ₁
参与者 2: 使用上一步结果, 生成 τ₂, 计算 {τ₁·τ₂, (τ₁·τ₂)², ...} → 删除 τ₂
...
参与者 N: 最终 τ = τ₁·τ₂·...·τₙ

只要有 1 个诚实参与者删除了秘密, Setup 就是安全的!
```

**实例**:
- Zcash Powers of Tau: 176 参与者
- Filecoin: 2000+ 参与者

### 通用 Setup (PLONK 改进)
- Groth16: 每个电路需要独立 Setup
- PLONK: 一次 Setup 可用于所有电路
- **SuperVM 选择**: 如果频繁更新电路, 考虑 PLONK

---

## 实现库对比

### bellman (Rust)
```toml
[dependencies]
bellman = "0.14"
pairing = "0.23"  # BN254 / BLS12-381 曲线
```

**示例代码**:
```rust
use bellman::{Circuit, ConstraintSystem, SynthesisError};
use pairing::bn256::{Bn256, Fr};

struct MyCircuit {
    x: Option<Fr>,  // 私有输入
}

impl Circuit<Bn256> for MyCircuit {
    fn synthesize<CS: ConstraintSystem<Bn256>>(
        self,
        cs: &mut CS,
    ) -> Result<(), SynthesisError> {
        // 分配变量
        let x = cs.alloc(|| "x", || {
            self.x.ok_or(SynthesisError::AssignmentMissing)
        })?;
        
        // 添加约束: x² = x (x=0 or x=1)
        cs.enforce(
            || "x * x = x",
            |lc| lc + x,
            |lc| lc + x,
            |lc| lc + x,
        );
        
        Ok(())
    }
}
```

**优点**:
- 成熟稳定 (Zcash 使用)
- 文档较完善
- 性能优化好

**缺点**:
- API 较底层 (手动构建约束)
- Trusted Setup 复杂

### arkworks (Rust)
```toml
[dependencies]
ark-groth16 = "0.4"
ark-bn254 = "0.4"
ark-std = "0.4"
```

**优点**:
- 模块化设计
- 支持多种证明系统 (Groth16, Marlin, GM17)
- 高性能

**缺点**:
- 学习曲线陡峭
- 文档不如 bellman 完善

---

## 下一步计划

### Week 3 Day 2-3
1. ✅ 完成 Groth16 原理学习
2. ✅ 原 bellman 路线尝试（pairing 特性/模块在 0.20/0.21/0.23 版本上存在不一致，导入与 API 对齐成本较高）
3. ✅ 切换 arkworks 路线：最小电路 a*b=c 实现与测试通过（ark-groth16 + ark-bls12-381）
4. ✅ 实现简易 Range 证明电路（位分解 + 布尔约束），测试通过
5. ⏳ 运行基准测试（已接入 setup/prove/verify 与 range_prove，首次运行中）

### Week 3 Day 4-7
1. ✅ 研究 arkworks 实现并完成 PoC
2. ⏳ 对比 bellman vs arkworks 性能（以相同电路在两套库跑 benchmark）
3. ⏳ 准备进入 PLONK 学习 (Week 4)

---

## 从 bellman 迁移到 arkworks 的原因与最小示例

在 Windows/当前工具链下，pairing 0.20/0.21/0.23 的曲线模块导出（`pairing::bn256` / `pairing::bls12_381`）与特性开关存在不一致，导致 bellman 示例在曲线导入与 `verify_proof` 返回类型期望之间需要较多版本对齐工作。为加快验证闭环，我们切换到 arkworks 生态（`ark-groth16`、`ark-bls12-381`），接口更稳定清晰，能快速得到可用的端到端结果与基准。

最小电路（a*b=c）示例位置：`zk-groth16-test/src/lib.rs`
- Setup: `Groth16::<Bls12_381>::generate_random_parameters_with_reduction`
- Prove: `Groth16::<Bls12_381>::prove(&params, circuit, rng)`
- Verify: `Groth16::<Bls12_381>::verify_proof(&pvk, &proof, &[c])`

Range 证明（位分解）示例位置：`zk-groth16-test/src/range_proof.rs`
- 约束：`v = c`，以及 `v = sum(b_i * 2^i)` 且每个 `b_i` 满足布尔约束 `b_i*(b_i-1)=0`
- 用于演示范围约束的构建方式；下一步将结合 Pedersen 承诺实现隐藏值的范围证明。

---

## 参考资料

1. **Groth16 论文**: "On the Size of Pairing-based Non-interactive Arguments" (Eurocrypt 2016)
2. **Zero Knowledge Blog**: https://www.zeroknowledgeblog.com/index.php/groth16
3. **bellman 源码**: https://github.com/zkcrypto/bellman
4. **Zcash Sapling**: https://z.cash/upgrade/sapling/
5. **Powers of Tau**: https://github.com/ZcashFoundation/powersoftau-attestations

---

*最后更新: 2025-11-04*
