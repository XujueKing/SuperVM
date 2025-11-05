# curve25519-dalek 学习笔记

开发者/作者：King Xujue

**研究周期**: Week 1-2 (2025-02-17 至 2025-03-02)  
**官方仓库**: https://github.com/dalek-cryptography/curve25519-dalek  
**文档**: https://docs.rs/curve25519-dalek/  
**学习目标**: 掌握 Ristretto Point API, 为 Ring Signature 实现做准备

---

## 📋 学习清单

- [ ] RistrettoPoint 基础操作
- [ ] Scalar 运算
- [ ] 示例代码运行
- [ ] 性能基准测试

---

## 🔧 1. 依赖添加

### 1.1 Cargo.toml 配置

```toml
[dependencies]
curve25519-dalek = { version = "4.1", features = ["serde"] }
sha2 = "0.10"
rand = "0.8"
```

**TODO**: 在 Week 9 实现阶段添加到 `vm-runtime/Cargo.toml`

---

## 📐 2. RistrettoPoint 基础

### 2.1 什么是 Ristretto?

**Ristretto** 是 Curve25519 的"群抽象层":
- 解决 Curve25519 cofactor = 8 的问题
- 提供唯一编码 (每个点只有一种表示)
- 防止小子群攻击

**为什么不用 EdwardsPoint?**
- EdwardsPoint 有 cofactor 问题
- 同一个点可能有多种编码 (安全隐患)
- Ristretto 保证唯一性 + 素数阶群

### 2.2 基础 API

```rust
use curve25519_dalek::ristretto::{RistrettoPoint, CompressedRistretto};
use curve25519_dalek::scalar::Scalar;
use curve25519_dalek::constants::RISTRETTO_BASEPOINT_POINT;

// 1. 基点
let G = RISTRETTO_BASEPOINT_POINT;

// 2. 标量乘法 (生成公钥)
let secret = Scalar::random(&mut rng);
let public = secret * G;

// 3. 点加法
let point1 = Scalar::random(&mut rng) * G;
let point2 = Scalar::random(&mut rng) * G;
let sum = point1 + point2;

// 4. 压缩/解压缩 (32 字节)
let compressed: CompressedRistretto = public.compress();
let bytes: [u8; 32] = compressed.to_bytes();
let decompressed: Option<RistrettoPoint> = compressed.decompress();
```

### 2.3 示例代码

**TODO**: 创建并运行示例

```rust
// examples/ristretto_demo.rs
use curve25519_dalek::ristretto::RistrettoPoint;
use curve25519_dalek::scalar::Scalar;
use curve25519_dalek::constants::RISTRETTO_BASEPOINT_POINT;
use rand::rngs::OsRng;

fn main() {
    let mut rng = OsRng;
    
    // 生成密钥对
    let secret_key = Scalar::random(&mut rng);
    let public_key = secret_key * RISTRETTO_BASEPOINT_POINT;
    
    println!("Secret key: {:?}", secret_key);
    println!("Public key (compressed): {:?}", public_key.compress().to_bytes());
    
    // 测试同态性 (a+b)*G = a*G + b*G
    let a = Scalar::random(&mut rng);
    let b = Scalar::random(&mut rng);
    let lhs = (a + b) * RISTRETTO_BASEPOINT_POINT;
    let rhs = a * RISTRETTO_BASEPOINT_POINT + b * RISTRETTO_BASEPOINT_POINT;
    assert_eq!(lhs, rhs);
    println!("Homomorphism test passed!");
}
```

---

## 🔢 3. Scalar 运算

### 3.1 Scalar 类型

**Scalar = 整数 mod l** (l = 2^252 + 27742317777372353535851937790883648493)

```rust
use curve25519_dalek::scalar::Scalar;

// 1. 生成随机数
let s = Scalar::random(&mut rng);

// 2. 从字节转换
let bytes: [u8; 32] = [1u8; 32];
let s = Scalar::from_bytes_mod_order(bytes); // 模 l 化简

// 3. 四则运算
let a = Scalar::random(&mut rng);
let b = Scalar::random(&mut rng);
let sum = a + b;
let diff = a - b;
let prod = a * b;
let inv = a.invert();  // 模逆元 (如果 a != 0)
let quot = a * inv;    // 除法 = a * b^(-1)

// 4. 特殊值
let zero = Scalar::zero();
let one = Scalar::one();
```

### 3.2 应用场景

**在 Ring Signature 中的用途**:
1. **私钥**: SecretKey = Scalar
2. **挑战值**: challenge = Scalar (Fiat-Shamir)
3. **响应值**: response = Scalar (签名的一部分)
4. **盲因子**: blinding_factor = Scalar (Pedersen Commitment)

---

## 🔐 4. Hash-to-Point

### 4.1 为什么需要?

**Key Image 生成**: I = x * Hp(P)
- Hp(P) 必须是确定性的点 (从公钥 P 派生)
- 不能简单哈希到字节 (需要是曲线上的点)

### 4.2 实现方法

**方法 1: RistrettoPoint::from_uniform_bytes()**

```rust
use sha2::{Sha512, Digest};
use curve25519_dalek::ristretto::RistrettoPoint;

fn hash_to_point(data: &[u8]) -> RistrettoPoint {
    let mut hasher = Sha512::new();
    hasher.update(data);
    let hash = hasher.finalize();
    
    // 将 64 字节哈希映射到点
    RistrettoPoint::from_uniform_bytes(&hash.into())
}
```

**方法 2: RistrettoPoint::hash_from_bytes()** (需要 `digest` feature)

```rust
use sha2::Sha512;
use curve25519_dalek::ristretto::RistrettoPoint;

fn hash_to_point(data: &[u8]) -> RistrettoPoint {
    RistrettoPoint::hash_from_bytes::<Sha512>(data)
}
```

### 4.3 Key Image 生成示例

```rust
fn generate_key_image(secret_key: &Scalar, public_key: &RistrettoPoint) -> RistrettoPoint {
    // Monero: I = x * Hp(P)
    let hp = hash_to_point(&public_key.compress().to_bytes());
    secret_key * hp
}

// 验证: 知道 (x, P=xG) 可计算 I, 但从 (P, I) 无法推出 x
```

---

## 🎯 5. Pedersen Commitment

### 5.1 理论

**承诺方案**: C(a, r) = aH + rG
- a: 金额 (secret)
- r: 盲因子 (blinding factor)
- G, H: 两个独立基点

**同态性**: C(a1, r1) + C(a2, r2) = C(a1+a2, r1+r2)

### 5.2 实现

```rust
use curve25519_dalek::ristretto::RistrettoPoint;
use curve25519_dalek::scalar::Scalar;
use curve25519_dalek::constants::RISTRETTO_BASEPOINT_POINT;

lazy_static! {
    // H = hash_to_point("Pedersen_H")
    static ref H: RistrettoPoint = {
        RistrettoPoint::hash_from_bytes::<Sha512>(b"Pedersen_H")
    };
}

fn commit(amount: u64, blinding_factor: &Scalar) -> RistrettoPoint {
    let amount_scalar = Scalar::from(amount);
    amount_scalar * (*H) + blinding_factor * RISTRETTO_BASEPOINT_POINT
}

// 验证平衡: sum(C_inputs) = sum(C_outputs)
fn verify_balance(
    input_commitments: &[RistrettoPoint],
    output_commitments: &[RistrettoPoint],
) -> bool {
    let sum_inputs: RistrettoPoint = input_commitments.iter().sum();
    let sum_outputs: RistrettoPoint = output_commitments.iter().sum();
    sum_inputs == sum_outputs
}
```

### 5.3 应用到 SuperVM

**TODO**: Week 17-20 实现 Commitment 模块时参考

---

## ⚡ 6. 性能基准

### 6.1 基础操作性能

**TODO**: 运行 `cargo bench` 测试

预期性能 (Intel i7, 单核):
- Scalar 乘法: ~50-60 μs
- Point 加法: ~10 μs
- Point 压缩: ~5 μs
- Point 解压缩: ~60 μs
- Hash-to-point: ~80 μs

### 6.2 优化技巧

**批量操作**:
```rust
// 慢: 逐个计算
let mut sum = RistrettoPoint::identity();
for scalar in scalars {
    sum += scalar * RISTRETTO_BASEPOINT_POINT;
}

// 快: 使用 Straus 算法 (多标量乘法)
use curve25519_dalek::traits::VartimeMultiscalarMul;
let sum = RistrettoPoint::vartime_multiscalar_mul(
    &scalars,
    &points
);
```

**预计算表**:
```rust
use curve25519_dalek::traits::MultiscalarMul;

// 预计算基点表 (加速重复乘法)
let precomputed = RISTRETTO_BASEPOINT_POINT.precompute();
let result = precomputed.multiply(&scalar);
```

---

## 🧪 7. 实战练习

### 7.1 练习 1: Ed25519 签名

**TODO**: 实现简单的 Schnorr 签名

```rust
// Schnorr 签名: (R, s) where R = rG, s = r + H(R||P||m)*x
struct SchnorrSignature {
    R: RistrettoPoint,
    s: Scalar,
}

fn sign(message: &[u8], secret_key: &Scalar) -> SchnorrSignature {
    // TODO: 实现
    todo!()
}

fn verify(message: &[u8], public_key: &RistrettoPoint, sig: &SchnorrSignature) -> bool {
    // TODO: 验证 sG = R + H(R||P||m)*P
    todo!()
}
```

### 7.2 练习 2: 简单 Ring Signature

**目标**: 实现 2-of-3 环签名 (简化版)

```rust
struct SimpleRingSignature {
    ring: [RistrettoPoint; 3],
    c: [Scalar; 3],      // 挑战值
    r: [Scalar; 3],      // 响应值
}

// TODO: 实现 sign() 和 verify()
```

---

## 📚 8. 参考资料

### 8.1 官方文档

- **curve25519-dalek docs**: https://docs.rs/curve25519-dalek/
- **Ristretto 论文**: https://ristretto.group/
- **Mike Hamburg's paper**: https://eprint.iacr.org/2015/673

### 8.2 示例项目

- **bulletproofs**: https://github.com/dalek-cryptography/bulletproofs
  - 使用 curve25519-dalek 实现范围证明
- **ed25519-dalek**: https://github.com/dalek-cryptography/ed25519-dalek
  - EdDSA 签名实现

### 8.3 学习路径

**推荐顺序**:
1. 阅读 docs.rs API 文档 (2-3 小时)
2. 运行官方示例 (1 小时)
3. 实现 Schnorr 签名 (半天)
4. 实现简单 Ring Signature (1 天)
5. 研究 bulletproofs 源码 (2-3 天)

---

## ✅ 学习进度

### Week 1 (2025-02-17 至 2025-02-23)

**Day 1 (2025-02-17)**:
- [x] 创建学习笔记
- [ ] 添加依赖到测试项目
- [ ] 运行基础示例 (RistrettoPoint, Scalar)

**Day 2-3**:
- [ ] 实现 Hash-to-Point
- [ ] 实现 Pedersen Commitment
- [ ] 性能基准测试

**Day 4-5**:
- [ ] 实现 Schnorr 签名
- [ ] 实现简单 Ring Signature (2-of-3)
- [ ] 单元测试

**Day 6-7**:
- [ ] 阅读 bulletproofs 源码
- [ ] 总结学习成果
- [ ] 准备 Week 2 高级主题

### Week 2 (2025-02-24 至 2025-03-02)

**Day 8-10**:
- [ ] 深入多标量乘法优化
- [ ] 批量验证技术
- [ ] 内存安全实践 (zeroize)

**Day 11-14**:
- [ ] 结合 Monero 源码理解应用
- [ ] 设计 SuperVM Ring Signature API
- [ ] 编写技术选型报告

---

## 💡 问题与思考

### 常见问题

1. **RistrettoPoint vs EdwardsPoint?**
   - 答: 永远用 Ristretto (除非你知道自己在做什么)

2. **性能瓶颈在哪?**
   - 答: Scalar 乘法 (占 80% 时间)

3. **如何安全处理私钥?**
   - 答: 使用 `zeroize` crate 自动清零

### 个人理解

**TODO**: 每日记录学习心得

---

**最后更新**: 2025-11-04  
**下次更新**: 每日同步学习进度
