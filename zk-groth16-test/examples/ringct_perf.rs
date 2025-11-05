//! RingCT 性能测试示例
//! 
//! 快速测量 setup/prove/verify 的实际耗时

use ark_bls12_381::{Bls12_381, Fr};
use ark_groth16::Groth16;
use ark_snark::SNARK;
use rand::rngs::OsRng;
use std::time::Instant;
use zk_groth16_test::ringct::SimpleRingCTCircuit;

fn main() {
    println!("=== RingCT Performance Test ===\n");
    
    let mut rng = OsRng;
    let circuit = SimpleRingCTCircuit::example();
    
    // Setup
    println!("🔧 Running setup...");
    let start = Instant::now();
    let (pk, vk) = Groth16::<Bls12_381>::circuit_specific_setup(circuit.clone(), &mut rng)
        .expect("Setup failed");
    let setup_time = start.elapsed();
    println!("   ✅ Setup time: {:?}\n", setup_time);
    
    // Prove
    println!("🔐 Generating proof...");
    let start = Instant::now();
    let proof = Groth16::<Bls12_381>::prove(&pk, circuit.clone(), &mut rng)
        .expect("Prove failed");
    let prove_time = start.elapsed();
    println!("   ✅ Prove time: {:?}\n", prove_time);
    
    // Verify
    println!("✓ Verifying proof...");
    let public_inputs = vec![
        circuit.input.commitment_x,
        circuit.input.commitment_y,
        circuit.output.commitment_x,
        circuit.output.commitment_y,
        circuit.merkle_proof.root,
    ];
    
    let start = Instant::now();
    let valid = Groth16::<Bls12_381>::verify(&vk, &public_inputs, &proof)
        .expect("Verify failed");
    let verify_time = start.elapsed();
    println!("   ✅ Verify time: {:?}", verify_time);
    println!("   ✅ Proof valid: {}\n", valid);
    
    // Summary
    println!("=== Summary ===");
    println!("Setup:  {:>8.2?}", setup_time);
    println!("Prove:  {:>8.2?}", prove_time);
    println!("Verify: {:>8.2?}", verify_time);
    println!("Total:  {:>8.2?}", setup_time + prove_time + verify_time);
}
