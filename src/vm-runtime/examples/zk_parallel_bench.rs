// SPDX-License-Identifier: GPL-3.0-or-later
//! 并行 ZK 证明吞吐基准 (占位 MultiplyCircuit) 
//! 运行方式:
//!   cargo run -p vm-runtime --release --example zk_parallel_bench --features groth16-verifier
//! 可选环境变量:
//!   ZK_PAR_BATCHES=5    (运行批次数)
//!   ZK_PAR_SIZES=8,32,64,128 (批量大小列表)
//!   ZK_PAR_THREADS=0    (0 表示使用 rayon 默认)

use std::sync::Arc;
use vm_runtime::privacy::parallel_prover::{ParallelProver, ParallelProveConfig, CircuitInput};
use vm_runtime::metrics::MetricsCollector;
use ark_groth16::Groth16;
use ark_bls12_381::Bls12_381;
use zk_groth16_test::MultiplyCircuit;

fn parse_sizes() -> Vec<usize> {
    std::env::var("ZK_PAR_SIZES")
        .unwrap_or_else(|_| "8,32,64".to_string())
        .split(',')
        .filter_map(|s| s.trim().parse::<usize>().ok())
        .collect()
}

fn main() {
    println!("🚀 并行 ZK 证明基准开始");
    let batches: usize = std::env::var("ZK_PAR_BATCHES").ok().and_then(|v| v.parse().ok()).unwrap_or(5);
    let thread_override: Option<usize> = std::env::var("ZK_PAR_THREADS").ok().and_then(|v| v.parse().ok()).filter(|n| *n>0);
    let sizes = parse_sizes();

    // Setup (一次 C R S)
    let rng = &mut rand::rngs::OsRng;
    let params = Groth16::<Bls12_381>::generate_random_parameters_with_reduction(MultiplyCircuit { a: None, b: None }, rng).expect("setup fail");
    let metrics = Arc::new(MetricsCollector::new());

    for &size in &sizes {
        println!("\n== 批量大小: {size} ==");
        let mut total_ok = 0usize;
        let mut total_failed = 0usize;
        let mut total_proofs_time_ms = 0.0f64;
        for b in 0..batches {
            let config = ParallelProveConfig { batch_size: size, num_threads: thread_override, collect_individual_latency: false };
            let prover = ParallelProver::new(params.clone(), config).with_metrics(metrics.clone());
            let inputs: Vec<_> = (0..size).map(|i| CircuitInput { a: (i as u64 + 2).into(), b: 3u64.into() }).collect();
            let stats = prover.prove_batch(&inputs);
            println!("批次 {}/{}: total={} ok={} failed={} batch_ms={:.2} tps={:.2}", b+1, batches, stats.total, stats.ok, stats.failed, stats.total_duration.as_secs_f64()*1000.0, stats.tps);
            total_ok += stats.ok;
            total_failed += stats.failed;
            total_proofs_time_ms += stats.total_duration.as_secs_f64()*1000.0;
        }
        let avg_batch_ms = total_proofs_time_ms / batches as f64;
        let overall_tps = (total_ok as f64) / (total_proofs_time_ms/1000.0);
        println!("汇总: proofs_ok={} failed={} avg_batch_ms={:.2} overall_tps={:.2}", total_ok, total_failed, avg_batch_ms, overall_tps);
    }

    // 导出并行指标片段
    let expo = metrics.export_prometheus();
    for line in expo.lines().filter(|l| l.contains("vm_privacy_zk_parallel")) {
        println!("METRIC {}", line);
    }
    println!("✅ 并行基准结束");
}
