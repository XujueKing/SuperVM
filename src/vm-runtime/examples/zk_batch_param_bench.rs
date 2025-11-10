// Phase 2.2 Batch Verification - Parameterized Benchmark
// 参数化批量验证性能基准测试
// 测试不同 batch_size (8/16/32/64/128) 对批量验证吞吐量与延迟的影响
//
// 运行命令:
//   cargo run --example zk_batch_param_bench --release --features groth16-verifier
//
// CSV 输出:
//   data/batch_param_bench/results.csv

#![cfg(feature = "groth16-verifier")]

use std::time::Instant;
use std::fs;
use std::io::Write;
use std::sync::Arc;

use ark_bls12_381::{Bls12_381, Fr};
use ark_groth16::{Groth16, Proof};
use ark_snark::SNARK;
use ark_std::UniformRand;

use vm_runtime::metrics::MetricsCollector;
use vm_runtime::privacy::batch_verifier::{BatchVerifier, BatchVerifyConfig};
use zk_groth16_test::MultiplyCircuit;

fn gen_proofs(n: usize, pk: &ark_groth16::ProvingKey<Bls12_381>) -> (Vec<Proof<Bls12_381>>, Vec<Vec<Fr>>) {
    let mut rng = rand::rngs::OsRng;
    let mut proofs = Vec::with_capacity(n);
    let mut public_inputs = Vec::with_capacity(n);

    for _ in 0..n {
        let a = Fr::rand(&mut rng);
        let b = Fr::rand(&mut rng);
        let c = a * b;
        let circuit = MultiplyCircuit { a: Some(a), b: Some(b) };
        let proof = Groth16::<Bls12_381>::prove(pk, circuit, &mut rng).expect("prove failed");
        proofs.push(proof);
        public_inputs.push(vec![c]);
    }
    (proofs, public_inputs)
}

fn main() {
    println!("=== ZK Batch Verification Parameterized Benchmark ===\n");

    // 准备输出目录
    let output_dir = "data/batch_param_bench";
    fs::create_dir_all(output_dir).expect("无法创建输出目录");
    let csv_path = format!("{}/results.csv", output_dir);
    let mut csv_file = fs::File::create(&csv_path).expect("无法创建 CSV 文件");
    writeln!(csv_file, "batch_size,total_proofs,duration_ms,tps,avg_latency_ms").expect("CSV 写入失败");

    // 一次性 setup
    let mut rng = rand::rngs::OsRng;
    let params = Groth16::<Bls12_381>::generate_random_parameters_with_reduction(
        MultiplyCircuit { a: None, b: None },
        &mut rng,
    ).expect("setup failed");
    let vk = params.vk.clone();

    // 测试参数
    let batch_sizes = vec![8, 16, 32, 64, 128];
    let total_proofs = 1000; // 每组测试 1000 个证明

    // 生成证明
    println!("[setup] Generating {} proofs...\n", total_proofs);
    let (proofs, public_inputs) = gen_proofs(total_proofs, &params);

    for &batch_size in &batch_sizes {
        println!("测试 batch_size = {} ...", batch_size);

        println!("Testing batch_size = {} ...", batch_size);

        // 创建批量验证器
        let metrics = Arc::new(MetricsCollector::new());
        let cfg = BatchVerifyConfig { batch_size, use_prepared_vk: true };
        let verifier = BatchVerifier::new(vk.clone(), cfg).with_metrics(metrics.clone());

        // 批量验证
        let start = Instant::now();
        let stats = verifier.verify_batch_optimized(&proofs, &public_inputs);
        let duration = start.elapsed();

        let duration_ms = duration.as_millis() as u64;
        let tps = if duration_ms > 0 {
            (total_proofs as u64 * 1000) / duration_ms
        } else {
            0
        };
        let avg_latency_ms = if total_proofs > 0 {
            duration_ms as f64 / total_proofs as f64
        } else {
            0.0
        };


        println!(
            "  Completed: {} proofs, {} ms, {} TPS, avg {:.2} ms/proof, failed: {}",
            total_proofs, duration_ms, tps, avg_latency_ms, stats.failed
        );

        // 写入 CSV
        writeln!(
            csv_file,
            "{},{},{},{},{:.2}",
            batch_size, total_proofs, duration_ms, tps, avg_latency_ms
        ).expect("CSV 写入失败");
    }


    println!("\nResults exported to: {}", csv_path);
    println!("Open CSV file with Excel/Google Sheets for visualization");
}
