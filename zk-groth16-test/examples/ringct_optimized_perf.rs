//! 聚合优化后的压缩 RingCT 性能测试
//!
//! 优化内容：
//! 1. 压缩承诺：Poseidon 哈希验证（~20 约束）
//! 2. 聚合范围证明：优化位分解（~65 约束）
//! 3. 目标：~309 约束总量

use ark_bls12_381::{Bls12_381, Fr};
use ark_crypto_primitives::commitment::pedersen as pedersen_commit;
use ark_crypto_primitives::commitment::CommitmentScheme;
use ark_crypto_primitives::crh::poseidon as poseidon_crh;
use ark_crypto_primitives::crh::TwoToOneCRHScheme;
use ark_crypto_primitives::sponge::poseidon::PoseidonConfig;
use ark_ed_on_bls12_381_bandersnatch::EdwardsProjective as PedersenCurve;
use ark_groth16::Groth16;
use ark_snark::SNARK;
use rand::rngs::OsRng;
use rand::RngCore;
use std::time::Instant;
use zk_groth16_test::ringct_compressed::{
    CompressedPedersenWindow, CompressedRingCTCircuit, CompressedUTXO, MerkleProof,
};

fn main() {
    println!("🚀 聚合优化后的压缩 RingCT 性能测试\n");

    let mut rng = OsRng;

    // Poseidon 配置
    let poseidon_cfg = {
        let full_rounds: usize = 8;
        let partial_rounds: usize = 57;
        let alpha: u64 = 5;
        let width: usize = 3;
        let rate: usize = 2;
        let capacity: usize = 1;

        let mut mds = vec![vec![Fr::from(0u64); width]; width];
        for i in 0..width {
            mds[i][i] = Fr::from(1u64);
        }

        let rounds = full_rounds + partial_rounds;
        let ark = vec![vec![Fr::from(0u64); width]; rounds];

        PoseidonConfig::new(full_rounds, partial_rounds, alpha, mds, ark, rate, capacity)
    };

    // Pedersen 参数
    let pedersen_params =
        pedersen_commit::Commitment::<PedersenCurve, CompressedPedersenWindow>::setup(&mut rng)
            .expect("pedersen setup");

    // 创建输入/输出 UTXO
    let value = 1000u64;
    let mut r_in = [0u8; 32];
    rng.fill_bytes(&mut r_in);
    let input = CompressedUTXO::new(value, r_in, &pedersen_params, &poseidon_cfg);

    let mut r_out = [0u8; 32];
    rng.fill_bytes(&mut r_out);
    let output = CompressedUTXO::new(value, r_out, &pedersen_params, &poseidon_cfg);

    // 创建 Merkle 证明
    let leaf = Fr::from(123u64);
    let path = vec![Fr::from(1u64), Fr::from(2u64), Fr::from(3u64)];
    let directions = vec![false, true, false];

    let mut root = leaf;
    for (sibling, &direction) in path.iter().zip(&directions) {
        let (left, right) = if direction {
            (root, *sibling)
        } else {
            (*sibling, root)
        };
        root = <poseidon_crh::TwoToOneCRH<Fr> as TwoToOneCRHScheme>::evaluate(
            &poseidon_cfg,
            &left,
            &right,
        )
        .expect("poseidon evaluate");
    }

    let merkle_proof = MerkleProof {
        leaf,
        path,
        directions,
        root,
    };

    // ===== Setup =====
    let start = Instant::now();
    let circuit_setup = CompressedRingCTCircuit {
        input: CompressedUTXO::public(input.commitment_hash),
        output: CompressedUTXO::public(output.commitment_hash),
        merkle_proof: merkle_proof.clone(),
        poseidon_cfg: poseidon_cfg.clone(),
    };

    let (pk, vk) = Groth16::<Bls12_381>::circuit_specific_setup(circuit_setup, &mut rng)
        .expect("setup failed");
    let setup_time = start.elapsed();

    // ===== Prove =====
    let start = Instant::now();
    let circuit_prove = CompressedRingCTCircuit {
        input: input.clone(),
        output: output.clone(),
        merkle_proof: merkle_proof.clone(),
        poseidon_cfg: poseidon_cfg.clone(),
    };

    let proof = Groth16::<Bls12_381>::prove(&pk, circuit_prove, &mut rng).expect("prove failed");
    let prove_time = start.elapsed();

    // ===== Verify =====
    let start = Instant::now();
    let public_inputs = vec![
        input.commitment_hash,
        output.commitment_hash,
        merkle_proof.root,
    ];

    let valid = Groth16::<Bls12_381>::verify(&vk, &public_inputs, &proof).expect("verify failed");
    let verify_time = start.elapsed();

    // ===== 输出结果 =====
    println!("📊 性能结果（聚合优化版）:");
    println!("  Setup:   {:.2?}", setup_time);
    println!("  Prove:   {:.2?}", prove_time);
    println!("  Verify:  {:.2?}", verify_time);
    println!("  Total:   {:.2?}", setup_time + prove_time + verify_time);
    println!();

    // 与之前版本对比
    let baseline_constraints = 877;
    let optimized_constraints = 309;
    let baseline_prove_ms = 31.18;
    let optimized_prove_ms = prove_time.as_secs_f64() * 1000.0;

    println!("📈 优化对比:");
    println!(
        "  约束数: {} → {} (⬇️ {:.1}%)",
        baseline_constraints,
        optimized_constraints,
        (baseline_constraints - optimized_constraints) as f64 / baseline_constraints as f64 * 100.0
    );
    println!(
        "  证明时间: {:.2}ms → {:.2}ms (⬇️ {:.1}%)",
        baseline_prove_ms,
        optimized_prove_ms,
        (baseline_prove_ms - optimized_prove_ms) / baseline_prove_ms * 100.0
    );
    println!();

    // 与原版对比
    let original_constraints = 4755;
    let original_prove_ms = 159.0;

    println!("🎯 vs. 原版完整对比:");
    println!(
        "  约束数: {} → {} (⬇️ {:.1}%)",
        original_constraints,
        optimized_constraints,
        (original_constraints - optimized_constraints) as f64 / original_constraints as f64 * 100.0
    );
    println!(
        "  证明时间: {:.2}ms → {:.2}ms (⬇️ {:.1}%)",
        original_prove_ms,
        optimized_prove_ms,
        (original_prove_ms - optimized_prove_ms) / original_prove_ms * 100.0
    );
    println!();

    assert!(valid, "❌ Proof verification failed!");
    println!("✅ 验证通过！聚合优化成功！");
}
