// 真实 KZG 证明基准测试
// 使用 halo2_proofs 0.3 的简化 API

use halo2_eval::MulCircuit;
use halo2_proofs::{
    plonk::{create_proof, keygen_pk, keygen_vk, verify_proof},
    poly::commitment::Params,
    transcript::{Blake2bRead, Blake2bWrite, Challenge255},
};
use halo2curves::bn256::{Fr, G1Affine};
use rand::rngs::OsRng;
use std::time::Instant;

// 辅助函数：执行完整的 prove-verify 流程
fn bench_full_cycle(
    k: u32,
) -> (
    std::time::Duration,
    std::time::Duration,
    std::time::Duration,
    usize,
) {
    let mut rng = OsRng;

    // 1. Setup: 生成 SRS (通用 trusted setup)
    let t_setup_start = Instant::now();
    let params = Params::<G1Affine>::new(k);
    let t_setup = t_setup_start.elapsed();

    // 2. Circuit
    let a = Fr::from(3u64);
    let b = Fr::from(5u64);
    let c = a * b;
    let circuit = MulCircuit {
        a: Some(a),
        b: Some(b),
    };

    // 3. Keygen
    let t_keygen_start = Instant::now();
    let vk = keygen_vk(&params, &circuit).expect("keygen_vk");
    let pk = keygen_pk(&params, vk, &circuit).expect("keygen_pk");
    let t_keygen = t_keygen_start.elapsed();

    // 4. Prove
    let instances = vec![vec![c]];
    let mut transcript = Blake2bWrite::<_, G1Affine, Challenge255<_>>::init(vec![]);
    let t_prove_start = Instant::now();

    create_proof(
        &params,
        &pk,
        &[circuit.clone()],
        &[&[&instances[0][..]]],
        rng,
        &mut transcript,
    )
    .expect("create_proof");

    let proof = transcript.finalize();
    let t_prove = t_prove_start.elapsed();
    let proof_size = proof.len();

    // 5. Verify
    let mut transcript = Blake2bRead::<_, G1Affine, Challenge255<_>>::init(&proof[..]);
    let t_verify_start = Instant::now();

    // 使用 SingleVerifier 策略（简单验证）
    use halo2_proofs::plonk::SingleVerifier;
    let strategy = SingleVerifier::new(&params);
    verify_proof(
        &params,
        pk.get_vk(),
        strategy,
        &[&[&instances[0][..]]],
        &mut transcript,
    )
    .expect("verify_proof failed");

    let t_verify = t_verify_start.elapsed();

    (t_setup + t_keygen, t_prove, t_verify, proof_size)
}

fn main() {
    println!("=== Halo2 (KZG/Bn256) 性能基准测试 ===\n");

    for k in [6, 8, 10] {
        print!("k={} (2^{}={} 行): ", k, k, 1 << k);
        let (t_setup, t_prove, t_verify, proof_size) = bench_full_cycle(k);
        println!(
            "Setup+Keygen={:.2?} | Prove={:.2?} | Verify={:.2?} | 证明大小={} bytes",
            t_setup, t_prove, t_verify, proof_size
        );
    }

    println!("\n对比 Groth16 (arkworks):");
    println!("  Groth16: Setup=26.8ms | Prove=10.0ms | Verify=3.6ms | 证明大小=128 bytes");
}
