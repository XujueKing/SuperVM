// 生成 RingCT BN254 Solidity 验证器 (用于 EVM 链部署)
// 使用 BN254 曲线,利用 EVM 原生预编译 (0x06/0x07/0x08),实现低 Gas 成本验证
// 仅在启用 `groth16-verifier` 特性时可用；否则提供占位 main。

#[cfg(feature = "groth16-verifier")]
use ark_bn254::{Bn254, Fr};
#[cfg(feature = "groth16-verifier")]
use ark_relations::r1cs::{ConstraintSynthesizer, ConstraintSystemRef, SynthesisError};
#[cfg(feature = "groth16-verifier")]
use ark_groth16::Groth16;
#[cfg(feature = "groth16-verifier")]
use ark_snark::SNARK;
#[cfg(feature = "groth16-verifier")]
use ark_std::rand::{rngs::StdRng, SeedableRng};

#[cfg(feature = "groth16-verifier")]
use vm_runtime::privacy::solidity_verifier::{SolidityVerifierGenerator, CurveKind};

/// 简化的 RingCT 电路 (BN254 版本)
/// 证明: commitment = value + blinding_factor (Pedersen 承诺简化版)
#[cfg(feature = "groth16-verifier")]
#[derive(Clone)]
struct RingCTCircuitBn254 {
    // 见证值 (私有)
    pub value: Option<Fr>,            // 交易金额
    pub blinding_factor: Option<Fr>,  // 致盲因子

    // 公共输入
    pub commitment: Option<Fr>,       // Pedersen 承诺 C = value*G + blinding_factor*H
}

#[cfg(feature = "groth16-verifier")]
impl ConstraintSynthesizer<Fr> for RingCTCircuitBn254 {
    fn generate_constraints(self, cs: ConstraintSystemRef<Fr>) -> Result<(), SynthesisError> {
        // 分配见证变量
        let value_var = cs.new_witness_variable(|| {
            self.value.ok_or(SynthesisError::AssignmentMissing)
        })?;

        let blinding_var = cs.new_witness_variable(|| {
            self.blinding_factor.ok_or(SynthesisError::AssignmentMissing)
        })?;

        // 分配公共输入
        let commitment_var = cs.new_input_variable(|| {
            self.commitment.ok_or(SynthesisError::AssignmentMissing)
        })?;

        // 约束: commitment = value + blinding_factor
        // 实现: value * 1 + blinding_factor * 1 = commitment
        let computed_commitment = cs.new_witness_variable(|| {
            let val = self.value.ok_or(SynthesisError::AssignmentMissing)?;
            let blind = self.blinding_factor.ok_or(SynthesisError::AssignmentMissing)?;
            Ok(val + blind)
        })?;

        // computed_commitment = commitment_var (equality constraint)
        cs.enforce_constraint(
            ark_relations::lc!() + value_var + blinding_var,
            ark_relations::lc!() + ark_relations::r1cs::Variable::One,
            ark_relations::lc!() + commitment_var,
        )?;

        Ok(())
    }
}

#[cfg(feature = "groth16-verifier")]
fn main() {
    println!("=== RingCT BN254 Solidity Verifier Generator ===\n");

    let mut rng = StdRng::seed_from_u64(42u64);

    // 1. 电路 Setup (生成 Proving Key + Verifying Key)
    println!("1. Generating circuit parameters (BN254)...");
    let circuit = RingCTCircuitBn254 {
        value: None,
        blinding_factor: None,
        commitment: None,
    };

    let (pk, vk) = Groth16::<Bn254>::circuit_specific_setup(circuit.clone(), &mut rng)
        .expect("Setup failed");

    println!("   ✓ Proving Key generated");
    println!("   ✓ Verifying Key generated\n");

    // 2. 生成 BN254 Solidity 验证器
    println!("2. Generating BN254 Solidity verifier contract...");
    let gen = SolidityVerifierGenerator::new("RingCTVerifierBN254")
        .with_curve(CurveKind::BN254);

    // 公共输入数量: 1 (commitment)
    let num_public_inputs = 1;
    let contract_path = "contracts/RingCTVerifierBN254.sol";

    gen.save_to_file_bn(&vk, num_public_inputs, contract_path)
        .expect("Failed to save contract");

    let contract_size = std::fs::metadata(contract_path)
        .expect("Failed to read contract")
        .len();

    println!("   ✓ Saved: {} ({} bytes)\n", contract_path, contract_size);

    // 3. 生成示例证明 (验证电路正确性)
    println!("3. Generating sample proof (verification test)...");
    let value = Fr::from(1000u32);              // 交易金额 1000
    let blinding_factor = Fr::from(42u32);      // 致盲因子
    let commitment = value + blinding_factor;   // 简化 Pedersen 承诺

    let circuit_with_inputs = RingCTCircuitBn254 {
        value: Some(value),
        blinding_factor: Some(blinding_factor),
        commitment: Some(commitment),
    };

    let proof = Groth16::<Bn254>::prove(&pk, circuit_with_inputs, &mut rng)
        .expect("Prove failed");

    // 验证证明
    let public_inputs = vec![commitment];
    let valid = Groth16::<Bn254>::verify(&vk, &public_inputs, &proof)
        .expect("Verify failed");

    println!("   ✓ Proof generated and verified: {}\n", valid);

    // 4. 显示合约部署说明
    println!("4. Deployment instructions:");
    println!("   # Compile with Foundry");
    println!("   forge build\n");
    println!("   # Deploy to Sepolia testnet");
    println!("   forge create \\");
    println!("     --rpc-url https://sepolia.infura.io/v3/YOUR_KEY \\");
    println!("     --private-key $PRIVATE_KEY \\");
    println!("     contracts/RingCTVerifierBN254.sol:RingCTVerifierBN254\n");
    println!("   # Call verifyProof()");
    println!("   # Public inputs: [commitment]");
    println!("   # Expected gas cost: ~150K-180K (1 public input)\n");

    println!("✅ BN254 RingCT verifier generation complete!");
    println!("📖 See docs/DUAL-CURVE-VERIFIER-GUIDE.md for usage details");
}

#[cfg(not(feature = "groth16-verifier"))]
fn main() {
    eprintln!("[generate_ringct_bn254_verifier] feature 'groth16-verifier' 未启用，示例被跳过。");
}

