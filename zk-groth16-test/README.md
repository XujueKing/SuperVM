# zk-groth16-test

开发者/作者：King Xujue

Groth16 zkSNARK 原型测试项目（arkworks 生态）

## 快速开始

### 环境要求
- Rust 1.70+
- Windows/Linux/macOS

### 安装依赖
```bash
cd zk-groth16-test
cargo build --release
```

### 运行测试
```bash
# 运行所有单元测试
cargo test

# 运行单个电路测试
cargo test test_multiply_circuit_end_to_end
cargo test test_range_proof_8_bits
cargo test test_pedersen_commitment
```

### 运行基准测试
```bash
# 运行所有基准
cargo bench

# 运行单个基准
cargo bench multiply_setup
cargo bench multiply_prove
cargo bench multiply_verify
cargo bench range_8bit_prove
cargo bench pedersen_prove
```

## 电路说明

### MultiplyCircuit（最小示例）
证明知道 a, b 使得 a * b = c（c 为公开输入）

```rust
use zk_groth16_test::MultiplyCircuit;

let circuit = MultiplyCircuit {
    a: Some(Fr::from(3u64)),
    b: Some(Fr::from(5u64)),
};
// 公开输入: c = 15
```

### RangeProofCircuit（范围证明）
证明知道 v 使得 v = c 且 0 ≤ v < 2^n_bits

**8-bit 范围证明示例**：
```rust
use zk_groth16_test::range_proof::RangeProofCircuit;

let circuit = RangeProofCircuit::new(Some(42), 8);
// 证明: 42 < 2^8
// 公开输入: c = 42
```

**64-bit 范围证明示例**（真实金额场景）：
```rust
use zk_groth16_test::range_proof::RangeProofCircuit;

let circuit = RangeProofCircuit::new(Some(12345678901234), 64);
// 证明: 12345678901234 < 2^64
// 公开输入: c = 12345678901234
// 约束数: ~70 个（位分解 + 布尔约束 + 求和）
```

### PedersenCommitmentCircuit（承诺证明）
证明知道 v, r 使得 C = v + r*k（简化线性承诺）

```rust
use zk_groth16_test::pedersen::PedersenCommitmentCircuit;

let circuit = PedersenCommitmentCircuit::new(Some(100), Some(42), 7);
// 证明: 知道 v=100, r=42 使得 C=100+42*7=394
// 公开输入: C = 394
```

## 端到端示例

```rust
use ark_bls12_381::{Bls12_381, Fr};
use ark_groth16::{Groth16, prepare_verifying_key};
use ark_snark::SNARK;
use zk_groth16_test::MultiplyCircuit;
use rand::rngs::OsRng;

fn main() {
    let rng = &mut OsRng;

    // 1. Trusted Setup（无见证电路）
    let params = Groth16::<Bls12_381>::generate_random_parameters_with_reduction(
        MultiplyCircuit { a: None, b: None },
        rng,
    ).unwrap();

    // 2. 生成证明（a=3, b=5）
    let a = Fr::from(3u64);
    let b = Fr::from(5u64);
    let c = a * b; // c = 15

    let proof = Groth16::<Bls12_381>::prove(
        &params,
        MultiplyCircuit { a: Some(a), b: Some(b) },
        rng,
    ).unwrap();

    // 3. 验证证明
    let pvk = prepare_verifying_key(&params.vk);
    let valid = Groth16::<Bls12_381>::verify_proof(&pvk, &proof, &[c]).unwrap();
    
    assert!(valid);
    println!("✅ 证明验证通过!");
}
```

## 性能数据（参考）

| 电路 | Setup | Prove | Verify | 约束数 | 说明 |
|------|-------|-------|--------|-------|------|
| Multiply | 31.1ms | 5.2ms | 3.6ms | ~1 | 最小示例 |
| Range-8bit | - | 4.4ms | ~3.6ms | ~10 | 8位范围 |
| **Range-64bit** | **19.6ms** | **7.4ms** | **~3.6ms** | **~70** | **真实金额范围** ✨ |
| Pedersen | - | 3.8ms | ~3.6ms | ~2 | 简化承诺 |
| Combined (Pedersen+Range-64bit) | 26.8ms | 10.0ms | ~3.6ms | ~72 | 承诺 + 范围组合 ✨ |

**关键发现**:
- **扩展性验证**: 约束数 ×7（10→70），证明时间仅 ×1.7（4.4ms→7.4ms），**亚线性增长**
- **验证成本固定**: 无论电路复杂度，验证时间恒定 ~3.6ms ✨
- **证明大小**: 128 bytes（恒定，无论约束数）

**环境**: Windows 10, Rust release build  
详细数据见: `docs/research/zk-evaluation.md`

## 项目结构

```
zk-groth16-test/
├── src/
│   ├── lib.rs              # MultiplyCircuit
│   ├── range_proof.rs      # RangeProofCircuit
│   └── pedersen.rs         # PedersenCommitmentCircuit
├── benches/
│   └── groth16_benchmarks.rs  # Criterion 基准测试
├── Cargo.toml
└── README.md
```

## 依赖

```toml
[dependencies]
ark-ff = "0.4"
ark-relations = "0.4"
ark-groth16 = "0.4"
ark-bls12-381 = "0.4"
ark-snark = "0.4"
rand = "0.8"

[dev-dependencies]
criterion = "0.5"
```

## 相关文档

### 本项目（Groth16）
- [Groth16 原理学习笔记](../docs/research/groth16-study.md)
- [zkSNARK 技术评估](../docs/research/zk-evaluation.md)
- [Groth16 PoC 总结](../docs/research/groth16-poc-summary.md)
- [Monero 隐私技术研究](../docs/research/monero-study-notes.md)
- [Ring Signature 实现报告](./RING_SIGNATURE_REPORT.md)
 - [RingCT Multi-UTXO 实现报告](./MULTI_UTXO_REPORT.md)
 - [RingCT Multi-UTXO 对抗性测试报告](./ADVERSARIAL_TESTS_REPORT.md)
 - [RingCT 约束优化报告](./OPTIMIZATION_REPORT.md)

### Halo2 评估（对比参考）
- [Halo2 项目 README](../halo2-eval/README.md) - Halo2 快速入门与性能数据
- [Halo2 评估总结](../docs/research/halo2-eval-summary.md) - Groth16 vs Halo2 全面对比

### 开发路线图
- [ZK 隐私层路线图](../ROADMAP-ZK-Privacy.md) - SuperVM 隐私层开发计划（Phase 1-4）

## License

MIT
