// 将 BN254 Groth16 proof 格式化为 Solidity calldata (JSON 输出)
// 用于测试网部署后调用 verifyProof() 函数

use ark_bn254::{Bn254, Fr, G1Affine, G2Affine, Fq};
use ark_relations::r1cs::{ConstraintSynthesizer, ConstraintSystemRef, SynthesisError};
use ark_groth16::Groth16;
use ark_snark::SNARK;
use ark_std::rand::{rngs::StdRng, SeedableRng};
use serde_json::json;
use std::fs;

/// 将 Fq/Fr 转换为 BigInt 再转 bytes (不需要 PrimeField trait)
fn field_to_bytes_be(f: impl AsRef<[u64]>) -> Vec<u8> {
    let limbs = f.as_ref();
    let mut bytes = Vec::with_capacity(32);
    for limb in limbs.iter().rev() {
        bytes.extend_from_slice(&limb.to_be_bytes());
    }
    bytes
}

/// 简单乘法电路 (a * b = c)
#[derive(Clone)]
struct MultiplyCircuitBn254 {
    pub a: Option<Fr>,
    pub b: Option<Fr>,
}

impl ConstraintSynthesizer<Fr> for MultiplyCircuitBn254 {
    fn generate_constraints(self, cs: ConstraintSystemRef<Fr>) -> Result<(), SynthesisError> {
        let a = cs.new_witness_variable(|| self.a.ok_or(SynthesisError::AssignmentMissing))?;
        let b = cs.new_witness_variable(|| self.b.ok_or(SynthesisError::AssignmentMissing))?;
        let c = cs.new_input_variable(|| {
            let a_val = self.a.ok_or(SynthesisError::AssignmentMissing)?;
            let b_val = self.b.ok_or(SynthesisError::AssignmentMissing)?;
            Ok(a_val * b_val)
        })?;
        
        cs.enforce_constraint(
            ark_relations::lc!() + a,
            ark_relations::lc!() + b,
            ark_relations::lc!() + c,
        )?;
        
        Ok(())
    }
}

/// 将 G1Affine 点转换为 Solidity uint256[2] 格式
fn g1_to_solidity(point: &G1Affine) -> [String; 2] {
    use ark_ff::PrimeField;
    let x_bytes = point.x.into_bigint().to_bytes_be();
    let y_bytes = point.y.into_bigint().to_bytes_be();
    [
        format!("0x{}", hex::encode(&x_bytes)),
        format!("0x{}", hex::encode(&y_bytes)),
    ]
}

/// 将 G2Affine 点转换为 Solidity uint256[2][2] 格式
fn g2_to_solidity(point: &G2Affine) -> [[String; 2]; 2] {
    use ark_ff::PrimeField;
    let x0_bytes = point.x.c0.into_bigint().to_bytes_be();
    let x1_bytes = point.x.c1.into_bigint().to_bytes_be();
    let y0_bytes = point.y.c0.into_bigint().to_bytes_be();
    let y1_bytes = point.y.c1.into_bigint().to_bytes_be();
    
    [
        [format!("0x{}", hex::encode(&x0_bytes)), format!("0x{}", hex::encode(&x1_bytes))],
        [format!("0x{}", hex::encode(&y0_bytes)), format!("0x{}", hex::encode(&y1_bytes))],
    ]
}

/// 将公共输入转换为 Solidity uint256[] 格式
fn public_inputs_to_solidity(inputs: &[Fr]) -> Vec<String> {
    use ark_ff::PrimeField;
    inputs.iter().map(|inp| {
        let bytes = inp.into_bigint().to_bytes_be();
        format!("0x{}", hex::encode(&bytes))
    }).collect()
}

fn main() {
    println!("=== BN254 Proof → Solidity Calldata Formatter ===\n");

    let mut rng = StdRng::seed_from_u64(12345u64);

    // 1. Setup 电路
    println!("1. Setting up circuit (Multiply: a * b = c)...");
    let circuit = MultiplyCircuitBn254 { a: None, b: None };
    let (pk, vk) = Groth16::<Bn254>::circuit_specific_setup(circuit, &mut rng)
        .expect("Setup failed");
    println!("   ✓ Circuit setup complete\n");

    // 2. 生成证明 (示例: 3 * 4 = 12)
    println!("2. Generating proof (3 * 4 = 12)...");
    let a_val = Fr::from(3u32);
    let b_val = Fr::from(4u32);
    let c_val = a_val * b_val; // 12

    let circuit_with_inputs = MultiplyCircuitBn254 {
        a: Some(a_val),
        b: Some(b_val),
    };

    let proof = Groth16::<Bn254>::prove(&pk, circuit_with_inputs, &mut rng)
        .expect("Prove failed");

    let public_inputs = vec![c_val];
    
    // 验证证明有效性
    let valid = Groth16::<Bn254>::verify(&vk, &public_inputs, &proof)
        .expect("Verify failed");
    println!("   ✓ Proof generated and verified: {}\n", valid);

    // 3. 格式化为 Solidity calldata
    println!("3. Formatting proof to Solidity calldata...");
    
    let proof_a = g1_to_solidity(&proof.a);
    let proof_b = g2_to_solidity(&proof.b);
    let proof_c = g1_to_solidity(&proof.c);
    let public_inputs_hex = public_inputs_to_solidity(&public_inputs);

    let calldata = json!({
        "a": proof_a,
        "b": proof_b,
        "c": proof_c,
        "input": public_inputs_hex,
        "metadata": {
            "circuit": "Multiply (a * b = c)",
            "witness": {
                "a": a_val.to_string(),
                "b": b_val.to_string()
            },
            "public_input": {
                "c": c_val.to_string()
            }
        }
    });

    // 4. 输出到文件
    let output_path = "proof_calldata_bn254.json";
    fs::write(output_path, serde_json::to_string_pretty(&calldata).unwrap())
        .expect("Failed to write calldata");

    println!("   ✓ Calldata saved to: {}\n", output_path);

    // 5. 显示使用说明
    println!("4. Usage with cast (Foundry):");
    println!("   cast send <VERIFIER_CONTRACT_ADDRESS> \\");
    println!("     \"verifyProof(uint256[2],uint256[2][2],uint256[2],uint256[1])\" \\");
    println!("     \"[{},{}]\" \\", proof_a[0], proof_a[1]);
    println!("     \"[[{},{}],[{},{}]]\" \\", 
        proof_b[0][0], proof_b[0][1], proof_b[1][0], proof_b[1][1]);
    println!("     \"[{},{}]\" \\", proof_c[0], proof_c[1]);
    println!("     \"[{}]\" \\", public_inputs_hex[0]);
    println!("     --rpc-url https://sepolia.infura.io/v3/YOUR_KEY \\");
    println!("     --private-key $PRIVATE_KEY\n");

    println!("5. Usage with Hardhat/Ethers.js:");
    println!("   const tx = await verifier.verifyProof(");
    println!("     {}, // a", serde_json::to_string(&proof_a).unwrap());
    println!("     {}, // b", serde_json::to_string(&proof_b).unwrap());
    println!("     {}, // c", serde_json::to_string(&proof_c).unwrap());
    println!("     {}  // input", serde_json::to_string(&public_inputs_hex).unwrap());
    println!("   );");
    println!("   const receipt = await tx.wait();");
    println!("   console.log('Gas used:', receipt.gasUsed.toString());\n");

    println!("✅ Proof calldata generation complete!");
    println!("📄 Output: {}", output_path);
}
