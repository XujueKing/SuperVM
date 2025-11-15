//! Multi-UTXO RingCT 性能测试 (2-in-2-out)
//!
//! 测试双输入双输出场景的性能表现

use ark_bls12_381::{Bls12_381, Fr};
use ark_crypto_primitives::commitment::pedersen as pedersen_commit;
use ark_crypto_primitives::commitment::CommitmentScheme;
use ark_crypto_primitives::crh::poseidon as poseidon_crh;
use ark_crypto_primitives::crh::CRHScheme;
use ark_crypto_primitives::crh::TwoToOneCRHScheme;
use ark_crypto_primitives::sponge::poseidon::PoseidonConfig;
use ark_ed_on_bls12_381_bandersnatch::EdwardsProjective as PedersenCurve;
use ark_groth16::Groth16;
use ark_snark::SNARK;
use rand::rngs::OsRng;
use rand::RngCore;
use std::time::Instant;
use zk_groth16_test::ringct_multi_utxo::{
    MerkleProof, MultiUTXOPedersenWindow, MultiUTXORingCTCircuit, UTXO,
};

fn main() {
    println!("🚀 Multi-UTXO RingCT 性能测试 (2-in-2-out)\n");

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
        pedersen_commit::Commitment::<PedersenCurve, MultiUTXOPedersenWindow>::setup(&mut rng)
            .expect("pedersen setup");

    // 创建 2 个输入 UTXO (总额 1500)
    let values_in = [1000u64, 500u64];
    let inputs: [UTXO; 2] = std::array::from_fn(|i| {
        let mut r = [0u8; 32];
        rng.fill_bytes(&mut r);
        UTXO::new(values_in[i], r, &pedersen_params, &poseidon_cfg)
    });

    // 创建 2 个输出 UTXO (总额 1500)
    let values_out = [800u64, 700u64];
    let outputs: [UTXO; 2] = std::array::from_fn(|i| {
        let mut r = [0u8; 32];
        rng.fill_bytes(&mut r);
        UTXO::new(values_out[i], r, &pedersen_params, &poseidon_cfg)
    });

    // 创建 2 个 Merkle 证明
    let merkle_proofs: [MerkleProof; 2] = std::array::from_fn(|i| {
        let leaf = Fr::from((100 + i) as u64);
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

        MerkleProof {
            leaf,
            path,
            directions,
            root,
        }
    });

    // ===== Setup =====
    let start = Instant::now();
    // 构造环签名授权（每个输入一个，ring_size=3）
    use ark_std::UniformRand;
    let ring_auths: [zk_groth16_test::ringct_multi_utxo::RingAuth; 2] = std::array::from_fn(|_| {
        let ring_size = 3usize;
        let real_index = 1usize;
        let secret_key = Fr::rand(&mut rng);
        let mut ring_members: Vec<zk_groth16_test::ring_signature::RingMember> =
            Vec::with_capacity(ring_size);
        for j in 0..ring_size {
            let pk = if j == real_index {
                secret_key
            } else {
                Fr::rand(&mut rng)
            };
            ring_members.push(zk_groth16_test::ring_signature::RingMember {
                public_key: pk,
                merkle_root: None,
            });
        }
        let public_key = ring_members[real_index].public_key;
        let key_image =
            poseidon_crh::CRH::<Fr>::evaluate(&poseidon_cfg, vec![secret_key, public_key]).unwrap();
        zk_groth16_test::ringct_multi_utxo::RingAuth {
            ring_members,
            real_index,
            secret_key,
            key_image,
        }
    });

    let mut circuit_setup = MultiUTXORingCTCircuit {
        inputs: inputs.clone(),
        outputs: outputs.clone(),
        merkle_proofs: merkle_proofs.clone(),
        ring_auths: ring_auths.clone(),
        poseidon_cfg: poseidon_cfg.clone(),
    };

    // 清空私有见证用于 setup
    for i in 0..2 {
        circuit_setup.inputs[i] = UTXO::public(inputs[i].commitment_hash);
        circuit_setup.outputs[i] = UTXO::public(outputs[i].commitment_hash);
    }

    let (pk, vk) = Groth16::<Bls12_381>::circuit_specific_setup(circuit_setup, &mut rng)
        .expect("setup failed");
    let setup_time = start.elapsed();

    // ===== Prove =====
    let start = Instant::now();
    let circuit_prove = MultiUTXORingCTCircuit {
        inputs: inputs.clone(),
        outputs: outputs.clone(),
        merkle_proofs: merkle_proofs.clone(),
        ring_auths: ring_auths.clone(),
        poseidon_cfg: poseidon_cfg.clone(),
    };

    let proof = Groth16::<Bls12_381>::prove(&pk, circuit_prove, &mut rng).expect("prove failed");
    let prove_time = start.elapsed();

    // ===== Verify =====
    let start = Instant::now();
    let mut public_inputs = Vec::new();
    // 输入承诺哈希
    for i in 0..2 {
        public_inputs.push(inputs[i].commitment_hash);
    }
    // 输出承诺哈希
    for i in 0..2 {
        public_inputs.push(outputs[i].commitment_hash);
    }
    // Merkle 根
    for i in 0..2 {
        public_inputs.push(merkle_proofs[i].root);
    }
    // Key Images
    for i in 0..2 {
        public_inputs.push(ring_auths[i].key_image);
    }

    let valid = Groth16::<Bls12_381>::verify(&vk, &public_inputs, &proof).expect("verify failed");
    let verify_time = start.elapsed();

    // ===== 输出结果 =====
    println!("📊 性能结果 (2-in-2-out):");
    println!("  Setup:   {:.2?}", setup_time);
    println!("  Prove:   {:.2?}", prove_time);
    println!("  Verify:  {:.2?}", verify_time);
    println!("  Total:   {:.2?}", setup_time + prove_time + verify_time);
    println!();

    // 与单 UTXO 版本对比
    let single_constraints = 309;
    let multi_constraints = 747;
    let single_prove_ms = 21.3;
    let multi_prove_ms = prove_time.as_secs_f64() * 1000.0;

    println!("📈 vs. 单 UTXO (1-in-1-out) 对比:");
    println!(
        "  约束数: {} → {} (×{:.2})",
        single_constraints,
        multi_constraints,
        multi_constraints as f64 / single_constraints as f64
    );
    println!(
        "  证明时间: {:.2}ms → {:.2}ms (×{:.2})",
        single_prove_ms,
        multi_prove_ms,
        multi_prove_ms / single_prove_ms
    );
    println!("  平均每 UTXO 约束: {:.0}", multi_constraints as f64 / 4.0);
    println!("  平均每 UTXO 时间: {:.2}ms", multi_prove_ms / 4.0);
    println!();

    // 可扩展性分析
    println!("🔍 可扩展性分析:");
    println!(
        "  约束扩展效率: {:.1}% (理想 200%)",
        (multi_constraints as f64 / single_constraints as f64) * 100.0
    );
    println!(
        "  时间扩展效率: {:.1}% (理想 200%)",
        (multi_prove_ms / single_prove_ms) * 100.0
    );

    // 预测更大规模
    let pred_4in4out = multi_constraints as f64 * 2.0;
    let pred_8in8out = multi_constraints as f64 * 4.0;
    println!("\n📊 预测更大规模:");
    println!(
        "  4-in-4-out: ~{:.0} 约束, ~{:.0}ms 证明",
        pred_4in4out,
        multi_prove_ms * 2.0
    );
    println!(
        "  8-in-8-out: ~{:.0} 约束, ~{:.0}ms 证明",
        pred_8in8out,
        multi_prove_ms * 4.0
    );
    println!();

    assert!(valid, "❌ Proof verification failed!");
    println!("✅ 验证通过！Multi-UTXO 支持成功实现！");
}
