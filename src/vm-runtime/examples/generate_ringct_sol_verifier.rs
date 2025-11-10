// SPDX-License-Identifier: GPL-3.0-or-later
// 生成 RingCT Groth16 Solidity 验证器示例

use vm_runtime::privacy::solidity_verifier::SolidityVerifierGenerator;
use zk_groth16_test::ringct_multi_utxo::MultiUTXORingCTCircuit;
use ark_bls12_381::Bls12_381;
use ark_groth16::Groth16;
use ark_snark::SNARK;
use rand::rngs::OsRng;

fn main() {
    println!("=== RingCT Solidity Verifier Generator ===\n");

    // 1. 生成 RingCT 电路的 setup
    println!("[1/3] Generating RingCT circuit setup...");
    let mut rng = OsRng;
    let circuit = MultiUTXORingCTCircuit::example();
    
    let (pk, vk) = Groth16::<Bls12_381>::circuit_specific_setup(circuit.clone(), &mut rng)
        .expect("Failed to setup circuit");
    
    println!("   ✓ Setup complete");
    println!("   - Proving key size: {} bytes", std::mem::size_of_val(&pk));
    println!("   - Verifying key points: {} gamma_abc", vk.gamma_abc_g1.len());

    // 2. 确定公共输入数量
    // MultiUTXORingCTCircuit 的公共输入: [amount_commitment, key_image, ring_member_pubkeys...]
    let num_public_inputs = vk.gamma_abc_g1.len() - 1; // gamma_abc[0] 是常量项
    println!("\n[2/3] Public inputs count: {}", num_public_inputs);

    // 3. 生成 Solidity 验证器
    println!("\n[3/3] Generating Solidity verifier...");
    let generator = SolidityVerifierGenerator::new("RingCTVerifier");
    
    // 保存到文件
    let output_path = "contracts/RingCTVerifier.sol";
    std::fs::create_dir_all("contracts")
        .expect("Failed to create contracts directory");
    
    generator.save_to_file(&vk, num_public_inputs, output_path)
        .expect("Failed to save Solidity file");
    
    println!("   ✓ Verifier saved to: {}", output_path);

    // 4. 显示合约信息
    let contract_code = generator.generate(&vk, num_public_inputs);
    println!("\n=== Contract Info ===");
    println!("Contract size: {} bytes", contract_code.len());
    println!("Public inputs: {}", num_public_inputs);
    println!("\nFirst 50 lines:");
    for (i, line) in contract_code.lines().take(50).enumerate() {
        println!("{:3}: {}", i + 1, line);
    }

    println!("\n=== Gas Estimation ===");
    println!("Estimated deployment gas: ~3,000,000 - 5,000,000");
    println!("Estimated verification gas: ~150,000 - 300,000");
    println!("\nNote: Actual gas costs depend on:");
    println!("  - Number of public inputs: {}", num_public_inputs);
    println!("  - EVM version and optimizations");
    println!("  - Network conditions");

    println!("\n=== Next Steps ===");
    println!("1. Deploy to test network (Ganache/Hardhat/Anvil):");
    println!("   forge create contracts/RingCTVerifier.sol:RingCTVerifier");
    println!("\n2. Test with real proofs:");
    println!("   - Generate proof using RingCtParallelProver");
    println!("   - Format proof/inputs for Solidity");
    println!("   - Call verifyProof() function");
    println!("\n3. Optimize gas:");
    println!("   - Enable Solidity optimizer");
    println!("   - Consider batch verification for multiple proofs");
    println!("   - Use calldata instead of memory where possible");
}
