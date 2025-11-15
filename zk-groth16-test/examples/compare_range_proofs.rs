// SPDX-License-Identifier: GPL-3.0-or-later
// Groth16 vs Bulletproofs Range Proof 性能对比
// 运行: cargo run --release --example compare_range_proofs

use std::time::Instant;
use zk_groth16_test::bulletproofs_range_proof::BulletproofsRangeProver;
use zk_groth16_test::range_proof::RangeProofCircuit;

// Groth16导入
use ark_bls12_381::Bls12_381;
use ark_groth16::{prepare_verifying_key, Groth16};
use ark_snark::SNARK;
use rand::rngs::OsRng;

fn main() {
    println!("=== Groth16 vs Bulletproofs Range Proof 性能对比 ===\n");

    // 测试参数
    let test_value = 12345678u64;
    let n_bits = 64;

    println!("测试参数:");
    println!("  Value: {}", test_value);
    println!("  Range: [0, 2^{})\n", n_bits);

    // ============================================
    // Groth16 64-bit Range Proof
    // ============================================
    println!("【Groth16 Range Proof】");
    
    let rng = &mut OsRng;
    
    // Trusted Setup
    let setup_start = Instant::now();
    let groth16_params = Groth16::<Bls12_381>::generate_random_parameters_with_reduction(
        RangeProofCircuit::new(None, n_bits),
        rng,
    )
    .expect("Groth16 setup failed");
    let setup_time = setup_start.elapsed();
    
    // 证明生成
    let prove_start = Instant::now();
    let groth16_proof = Groth16::<Bls12_381>::prove(
        &groth16_params,
        RangeProofCircuit::new(Some(test_value), n_bits),
        rng,
    )
    .expect("Groth16 prove failed");
    let prove_time = prove_start.elapsed();
    
    // 证明验证
    let pvk = prepare_verifying_key(&groth16_params.vk);
    let public_input = vec![ark_bls12_381::Fr::from(test_value)];
    
    let verify_start = Instant::now();
    let groth16_valid = Groth16::<Bls12_381>::verify_with_processed_vk(&pvk, &public_input, &groth16_proof)
        .expect("Groth16 verify failed");
    let verify_time = verify_start.elapsed();
    
    // 证明大小 (Groth16恒定128字节)
    let groth16_proof_size = 128; // 3个G1点 (48B each) = 144B压缩后约128B
    
    println!("  Setup时间:    {:?}", setup_time);
    println!("  证明时间:     {:?}", prove_time);
    println!("  验证时间:     {:?}", verify_time);
    println!("  证明大小:     {} bytes", groth16_proof_size);
    println!("  验证结果:     {}", if groth16_valid { "✓ 通过" } else { "✗ 失败" });
    println!("  Setup类型:    Trusted Setup (需要MPC仪式)\n");

    // ============================================
    // Bulletproofs Range Proof
    // ============================================
    println!("【Bulletproofs Range Proof】");
    
    // 创建证明器 (无需Setup!)
    let bp_prover = BulletproofsRangeProver::new(64);
    
    // 证明生成
    let prove_start = Instant::now();
    let (bp_proof, bp_commitment, _blinding) = bp_prover
        .prove_range_auto_blinding(test_value, n_bits)
        .expect("Bulletproofs prove failed");
    let bp_prove_time = prove_start.elapsed();
    
    // 证明验证
    let verify_start = Instant::now();
    let bp_valid = bp_prover
        .verify_range(&bp_proof, &bp_commitment, n_bits)
        .is_ok();
    let bp_verify_time = verify_start.elapsed();
    
    // 证明大小
    let bp_proof_size = BulletproofsRangeProver::proof_size(&bp_proof);
    
    println!("  Setup时间:    0ms (透明Setup, 无需Trusted Ceremony)");
    println!("  证明时间:     {:?}", bp_prove_time);
    println!("  验证时间:     {:?}", bp_verify_time);
    println!("  证明大小:     {} bytes", bp_proof_size);
    println!("  验证结果:     {}", if bp_valid { "✓ 通过" } else { "✗ 失败" });
    println!("  Setup类型:    透明Setup (无需信任假设)\n");

    // ============================================
    // 性能对比总结
    // ============================================
    println!("【性能对比总结】");
    println!("┌─────────────────┬──────────────┬────────────────┬──────────┐");
    println!("│ 指标            │ Groth16      │ Bulletproofs   │ 倍数     │");
    println!("├─────────────────┼──────────────┼────────────────┼──────────┤");
    println!("│ Setup时间       │ {:>9.2?}  │ {:>11}   │ {:>8} │", 
             setup_time, "0ms", "N/A");
    println!("│ 证明时间        │ {:>9.2?}  │ {:>11.2?}   │ {:>7.2}x │", 
             prove_time, bp_prove_time, bp_prove_time.as_secs_f64() / prove_time.as_secs_f64());
    println!("│ 验证时间        │ {:>9.2?}  │ {:>11.2?}   │ {:>7.2}x │", 
             verify_time, bp_verify_time, bp_verify_time.as_secs_f64() / verify_time.as_secs_f64());
    println!("│ 证明大小        │ {:>9} B │ {:>11} B │ {:>7.2}x │", 
             groth16_proof_size, bp_proof_size, bp_proof_size as f64 / groth16_proof_size as f64);
    println!("└─────────────────┴──────────────┴────────────────┴──────────┘\n");

    // ============================================
    // 批量验证性能测试
    // ============================================
    println!("【批量验证性能测试】(Bulletproofs优势)");
    
    let batch_sizes = [10, 50, 100];
    
    for &batch_size in &batch_sizes {
        // 生成批量证明
        let values: Vec<u64> = (0..batch_size).map(|i| 1000 + i as u64).collect();
        let mut proofs = Vec::new();
        let mut commitments = Vec::new();
        
        for &value in &values {
            let (proof, commitment, _) = bp_prover
                .prove_range_auto_blinding(value, n_bits)
                .expect("Prove failed");
            proofs.push(proof);
            commitments.push(commitment);
        }
        
        // 批量验证
        let batch_start = Instant::now();
        let batch_valid = bp_prover.verify_batch(&proofs, &commitments, n_bits).is_ok();
        let batch_time = batch_start.elapsed();
        
        // 逐个验证对比
        let individual_start = Instant::now();
        for (proof, commitment) in proofs.iter().zip(commitments.iter()) {
            bp_prover.verify_range(proof, commitment, n_bits).expect("Verify failed");
        }
        let individual_time = individual_start.elapsed();
        
        let speedup = individual_time.as_secs_f64() / batch_time.as_secs_f64();
        
        println!("  批次大小 {}: 批量={:?} vs 逐个={:?} | 加速 {:.2}x | 均摊 {:?}/个 | {}", 
                 batch_size, 
                 batch_time, 
                 individual_time, 
                 speedup,
                 batch_time / batch_size,
                 if batch_valid { "✓" } else { "✗" });
    }

    println!("\n【使用场景建议】");
    println!("  Groth16:");
    println!("    ✓ 链上验证 (证明小128B, Gas成本低)");
    println!("    ✓ 验证速度优先 (约3-4ms)");
    println!("    ✗ 需要Trusted Setup (信任假设)");
    println!("    ✗ 每个电路需要独立Setup");
    
    println!("\n  Bulletproofs:");
    println!("    ✓ 透明Setup (无需信任假设)");
    println!("    ✓ 链下聚合 (批量验证快3-5倍)");
    println!("    ✓ 灵活范围大小 (无需重新Setup)");
    println!("    ✗ 证明大 (约672-736B)");
    println!("    ✗ 链上验证Gas高 (不适合EVM)");

    println!("\n【推荐策略】混合使用:");
    println!("  • 链上隐私交易 → Groth16");
    println!("  • 链下批量聚合 → Bulletproofs");
    println!("  • zkVM开发迭代 → Bulletproofs (无需Setup)");
}
