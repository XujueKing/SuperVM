# 双曲线 Solidity 验证器指南

## 概述

SuperVM 的 Groth16 验证器生成器支持双曲线后端:
- **BLS12-381**: 128-bit 安全级别,面向未来 (EVM 2.0, zkEVM 2.0)
- **BN254 (alt_bn128)**: 100-bit 安全级别,当前 EVM 链原生支持

两条曲线完全并行,根据部署目标选择,不影响核心电路逻辑。

---

## 📊 曲线对比

| 特性 | BLS12-381 | BN254 (alt_bn128) |
|------|-----------|-------------------|
| **安全级别** | 128-bit | 100-bit |
| **EVM 原生支持** | ❌ (EVM 2.0 路线图) | ✅ (预编译 0x06/0x07/0x08) |
| **Gas 成本** | ⚠️ 需自定义实现 | ✅ 低成本 (~150K-200K gas) |
| **场素数 (q)** | 4002409...891 (381-bit) | 21888242...8583 (254-bit) |
| **配对友好性** | Type-3 (G1, G2 分离) | Type-3 |
| **Arkworks 包** | `ark-bls12-381` | `ark-bn254` |
| **适用场景** | 长期安全 / zkEVM 2.0 / 研究 | 现有 EVM 链部署 |

---

## 🚀 快速开始

### 1. 生成 BN254 验证器合约 (当前 EVM 链部署)

```rust
use vm_runtime::privacy::solidity_verifier::{SolidityVerifierGenerator, CurveKind};
use ark_bn254::{Bn254, Fr};
use ark_groth16::Groth16;

// 1. 定义电路 (使用 BN254 标量域)
#[derive(Clone)]
struct MyCircuitBn254 {
    pub a: Option<Fr>,
    pub b: Option<Fr>,
}

impl ConstraintSynthesizer<Fr> for MyCircuitBn254 {
    fn generate_constraints(self, cs: ConstraintSystemRef<Fr>) -> Result<(), SynthesisError> {
        let a = cs.new_witness_variable(|| self.a.ok_or(SynthesisError::AssignmentMissing))?;
        let b = cs.new_witness_variable(|| self.b.ok_or(SynthesisError::AssignmentMissing))?;
        let c = cs.new_input_variable(|| {
            let a_val = self.a.ok_or(SynthesisError::AssignmentMissing)?;
            let b_val = self.b.ok_or(SynthesisError::AssignmentMissing)?;
            Ok(a_val * b_val)
        })?;
        cs.enforce_constraint(lc!() + a, lc!() + b, lc!() + c)?;
        Ok(())
    }
}

// 2. 生成 Proving Key + Verifying Key
let mut rng = test_rng();
let circuit = MyCircuitBn254 { a: None, b: None };
let (pk, vk) = Groth16::<Bn254>::circuit_specific_setup(circuit, &mut rng).unwrap();

// 3. 生成 BN254 Solidity 合约
let gen = SolidityVerifierGenerator::new("MyVerifierBN254")
    .with_curve(CurveKind::BN254);

gen.save_to_file_bn(&vk, 1, "contracts/MyVerifierBN254.sol").unwrap();
```

**输出**: `contracts/MyVerifierBN254.sol` (~3.5KB, 使用 EVM 预编译 0x08)

### 2. 生成 BLS12-381 验证器合约 (未来链 / 研究用途)

```rust
use ark_bls12_381::{Bls12_381, Fr};

// 1. 定义电路 (使用 BLS12-381 标量域)
#[derive(Clone)]
struct MyCircuitBls {
    pub a: Option<Fr>,
    pub b: Option<Fr>,
}

// ... (ConstraintSynthesizer 实现相同)

// 2. Setup
let (pk, vk) = Groth16::<Bls12_381>::circuit_specific_setup(circuit, &mut rng).unwrap();

// 3. 生成 BLS12-381 Solidity 合约
let gen = SolidityVerifierGenerator::new("MyVerifierBLS")
    .with_curve(CurveKind::BLS12_381); // 默认值,可省略

gen.save_to_file(&vk, 1, "contracts/MyVerifierBLS.sol").unwrap();
```

**输出**: `contracts/MyVerifierBLS.sol` (~5.5KB, 需自定义预编译或链支持)

---

## 🔧 API 参考

### `SolidityVerifierGenerator`

**构造方法**:
```rust
pub fn new(contract_name: &str) -> Self
```

**曲线选择**:
```rust
pub fn with_curve(self, curve: CurveKind) -> Self
```
- `CurveKind::BLS12_381` (默认)
- `CurveKind::BN254`

**BLS12-381 合约生成**:
```rust
pub fn generate_bls(
    &self,
    vk: &VerifyingKey<Bls12_381>,
    num_public_inputs: usize
) -> String

pub fn save_to_file(
    &self,
    vk: &VerifyingKey<Bls12_381>,
    num_public_inputs: usize,
    path: &str
) -> std::io::Result<()>
```

**BN254 合约生成**:
```rust
pub fn generate_bn254(
    &self,
    vk: &VerifyingKey<Bn254>,
    num_public_inputs: usize
) -> String

pub fn save_to_file_bn(
    &self,
    vk: &VerifyingKey<Bn254>,
    num_public_inputs: usize,
    path: &str
) -> std::io::Result<()>
```

---

## 📦 部署流程

### 快速开始

详细部署步骤请参考: **[DEPLOYMENT-GUIDE.md](./DEPLOYMENT-GUIDE.md)**

我们提供三种部署方案:
1. **Remix IDE** - 最简单,无需本地环境,适合快速测试
2. **Hardhat** - JavaScript/TypeScript 生态,适合 Web3 开发者
3. **Foundry** - Rust 生态,适合 Solidity 高级开发

### BN254 部署 (Ethereum / Polygon / Arbitrum / Optimism)

**1. 使用 Foundry 编译合约**:
```bash
forge build
```

**2. 部署到测试网**:
```bash
# 示例: Sepolia 测试网
forge create \
  --rpc-url https://sepolia.infura.io/v3/YOUR_KEY \
  --private-key $PRIVATE_KEY \
  contracts/MyVerifierBN254.sol:MyVerifierBN254
```

**3. 验证合约 (可选)**:
```bash
forge verify-contract \
  --chain sepolia \
  --etherscan-api-key $ETHERSCAN_KEY \
  <CONTRACT_ADDRESS> \
  contracts/MyVerifierBN254.sol:MyVerifierBN254
```

**4. 调用验证接口**:
```solidity
// 准备证明数据 (从 Rust 生成的 JSON 格式化)
uint256[2] memory a = [proofA_x, proofA_y];
uint256[2][2] memory b = [[proofB_x0, proofB_x1], [proofB_y0, proofB_y1]];
uint256[2] memory c = [proofC_x, proofC_y];
uint256[1] memory input = [public_input_0];

bool valid = verifier.verifyProof(a, b, c, input);
```

### BLS12-381 部署 (zkEVM 2.0 / 自定义链)

⚠️ **当前主流 EVM 链不支持 BLS12-381 预编译**,需满足以下条件之一:
1. 目标链实现 EIP-XXXX (BLS12-381 预编译提案)
2. 使用 zkEVM 2.0+ (如 Scroll 2.0, Polygon zkEVM 2.0)
3. 自定义链部署并提供 0x0A-0x0E 预编译接口

**适用场景**:
- 长期安全性要求 (128-bit)
- 与 Ethereum 2.0 验证器兼容
- 研究与原型验证

---

## 🧪 测试验证

### 运行 BN254 示例
```bash
cargo run -p vm-runtime --features groth16-verifier \
  --example generate_bn254_multiply_sol_verifier --release
```

**输出**:
```
=== BN254 Solidity Verifier Generator (Multiply) ===
saved: contracts/BN254MultiplyVerifier.sol (3474 bytes)
```

### 运行 BLS12-381 测试
```bash
cargo test -p vm-runtime --features groth16-verifier \
  privacy::solidity_verifier --lib -- --nocapture
```

**预期输出**:
```
running 2 tests
Generated Solidity verifier (5574 bytes):
// SPDX-License-Identifier: MIT
pragma solidity ^0.8.0;
contract MultiplyVerifier { ... }
test test_generate_solidity_verifier ... ok
test test_save_solidity_verifier ... ok
```

---

## 🎯 选择建议

### 选择 BN254 (推荐用于现阶段部署)

✅ **适用场景**:
- 部署到现有 EVM 链 (Ethereum, Polygon, BSC, Arbitrum, Optimism)
- 需要低 Gas 成本 (~150K-200K gas/验证)
- 跨链桥接 (L1→L2 验证)
- 生产环境上链

❌ **不适用**:
- 极高安全性要求 (金融级应用建议使用 128-bit)

### 选择 BLS12-381 (面向未来)

✅ **适用场景**:
- 长期归档与安全性要求 (128-bit)
- EVM 2.0 / zkEVM 2.0 链
- 研究原型与技术验证
- 与 Ethereum 2.0 验证器互操作

❌ **不适用**:
- 当前主流 EVM 链 (无预编译支持)
- Gas 敏感型应用 (需自定义实现,成本高)

---

## 📋 Gas 成本对比

### BN254 (实测数据,Sepolia 测试网)

| 操作 | Gas 成本 | 备注 |
|------|---------|------|
| 合约部署 | ~800K | 一次性 |
| 单次验证 (1 公共输入) | ~150K-180K | 使用预编译 0x08 |
| 单次验证 (10 公共输入) | ~200K-250K | 线性增长 |

### BLS12-381 (理论估算)

⚠️ **无 EVM 原生支持,需自定义实现**:
- 预编译方式: ~150K-200K (需链支持)
- 纯 Solidity 实现: ~5M-10M gas (不推荐)
- Yul 优化实现: ~1M-2M gas (中等成本)

---

## 🔗 相关资源

- **EVM 预编译地址**:
  - 0x06: `ecAdd` (G1 点加法)
  - 0x07: `ecMul` (G1 标量乘法)
  - 0x08: `ecPairing` (配对检查) ← 验证器使用
- **EIP-2537**: BLS12-381 预编译提案 (未正式激活)
- **Foundry 部署文档**: https://book.getfoundry.sh/forge/deploying
- **Arkworks Groth16**: https://docs.rs/ark-groth16/0.4.0/

---

## ❓ FAQ

**Q: 为什么提供两条曲线?**  
A: BN254 满足当前 EVM 链部署需求 (低 Gas, 原生支持),BLS12-381 面向未来高安全性场景 (EVM 2.0, zkEVM),两者不冲突。

**Q: 哪条曲线更安全?**  
A: BLS12-381 (128-bit) > BN254 (100-bit),但 BN254 对绝大多数应用足够安全。

**Q: 可以在同一项目中同时使用两条曲线吗?**  
A: 可以,为不同电路生成不同合约,根据部署目标选择。例如:核心资产用 BLS12-381 (高安全),辅助功能用 BN254 (低 Gas)。

**Q: BN254 验证器在 zkEVM 上能运行吗?**  
A: 可以,zkEVM 向后兼容 EVM 预编译,BN254 合约可直接部署。

**Q: 如何处理证明数据格式化?**  
A: 使用 `examples/format_ringct_proof_for_solidity.rs` (开发中) 导出 JSON 格式,前端/脚本调用 Solidity 接口。

---

## 📝 版本记录

- **v0.5.0** (2025-11-09): 双曲线后端实现,BLS12-381 + BN254 完整支持
- **v0.4.0** (2025-11-08): 初版 BLS12-381 验证器生成器

---

**开发团队**: Rainbow Haruko / king / NoahX / Alan Tang / Xuxu  
**许可证**: MIT
