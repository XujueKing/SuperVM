// SPDX-License-Identifier: GPL-3.0-or-later
//! 对比：单笔验证 vs 批量验证 (Groth16, BLS12-381)
//! 运行示例：
//!   cargo run -p vm-runtime --release --example zk_batch_vs_single_bench --features groth16-verifier
//! 环境变量（可选）：
//!   ZK_VERIFY_PROOFS=128          # 生成并验证的证明数量（默认 128）
//!   ZK_VERIFY_BATCH_SIZE=32       # 批量验证批大小（默认 32）
//!   ZK_VERIFY_HTTP=0|1            # 是否启动 HTTP /metrics 导出（默认 0）
//!   ZK_VERIFY_PORT=8085           # HTTP 端口（默认 8085）

#![cfg(feature = "groth16-verifier")]

use std::sync::Arc;
use std::time::Instant;

use ark_bls12_381::{Bls12_381, Fr};
use ark_groth16::{Groth16, Proof, VerifyingKey};
use ark_snark::SNARK;
use ark_std::UniformRand;

use vm_runtime::metrics::MetricsCollector;
use vm_runtime::privacy::batch_verifier::{BatchVerifier, BatchVerifyConfig};
use zk_groth16_test::MultiplyCircuit;

fn env_usize(key: &str, default: usize) -> usize {
    std::env::var(key).ok().and_then(|v| v.parse().ok()).unwrap_or(default)
}
fn env_bool(key: &str, default: bool) -> bool {
    std::env::var(key)
        .ok()
        .map(|v| matches!(v.as_str(), "1" | "true" | "TRUE"))
        .unwrap_or(default)
}
fn env_u16(key: &str, default: u16) -> u16 {
    std::env::var(key).ok().and_then(|v| v.parse().ok()).unwrap_or(default)
}

fn gen_proofs(n: usize, pk: &ark_groth16::ProvingKey<Bls12_381>) -> (Vec<Proof<Bls12_381>>, Vec<Vec<Fr>>) {
    let mut rng = rand::rngs::OsRng;
    let mut proofs = Vec::with_capacity(n);
    let mut public_inputs = Vec::with_capacity(n);

    for _ in 0..n {
        // 随机 a,b 并计算 c=a*b
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

fn verify_single(vk: &VerifyingKey<Bls12_381>, proofs: &[Proof<Bls12_381>], inputs: &[Vec<Fr>]) -> (usize, usize, f64) {
    assert_eq!(proofs.len(), inputs.len());
    let t0 = Instant::now();
    let mut ok = 0usize;
    let mut failed = 0usize;
    for (pf, inp) in proofs.iter().zip(inputs.iter()) {
        let r = Groth16::<Bls12_381>::verify(vk, inp, pf).unwrap_or(false);
        if r { ok += 1; } else { failed += 1; }
    }
    let ms = t0.elapsed().as_secs_f64() * 1000.0;
    (ok, failed, ms)
}

fn main() {
    println!("=== ZK Verify: Single vs Batch ===");
    let total = env_usize("ZK_VERIFY_PROOFS", 128);
    let batch_size = env_usize("ZK_VERIFY_BATCH_SIZE", 32); // 仅用于展示，BatchVerifier 当前逐个并行验证
    let http = env_bool("ZK_VERIFY_HTTP", false);
    let port = env_u16("ZK_VERIFY_PORT", 8085);

    // 一次性 setup
    let mut rng = rand::rngs::OsRng;
    let params = Groth16::<Bls12_381>::generate_random_parameters_with_reduction(
        MultiplyCircuit { a: None, b: None },
        &mut rng,
    ).expect("setup failed");
    let vk = params.vk.clone();

    // 生成 N 个证明
    println!("[setup] generate {} proofs...", total);
    let (proofs, public_inputs) = gen_proofs(total, &params);

    // 1) 单笔验证
    println!("\n-- Single verify --");
    let (ok1, fail1, ms1) = verify_single(&vk, &proofs, &public_inputs);
    let avg1 = if total > 0 { ms1 / total as f64 } else { 0.0 };
    let tps1 = if ms1 > 0.0 { ok1 as f64 / (ms1/1000.0) } else { 0.0 };
    println!("single: ok={} failed={} total_ms={:.2} avg_ms={:.4} tps={:.2}", ok1, fail1, ms1, avg1, tps1);

    // 2) 批量验证（并行）
    println!("\n-- Batch verify (parallel) --");
    let metrics = Arc::new(MetricsCollector::new());
    let cfg = BatchVerifyConfig { batch_size, use_prepared_vk: true };
    let verifier = BatchVerifier::new(vk.clone(), cfg).with_metrics(metrics.clone());
    let stats = verifier.verify_batch_optimized(&proofs, &public_inputs);
    println!(
        "batch: total={} ok={} failed={} total_ms={:.2} avg_ms={:.4} tps={:.2}",
        stats.total, stats.verified, stats.failed, stats.total_duration.as_secs_f64()*1000.0, stats.avg_latency_ms, stats.verifications_per_sec
    );

    // 导出 Prometheus 指标
    if http {
        use tiny_http::{Header, Response, Server};
        let addr = format!("0.0.0.0:{}", port);
        let server = Server::http(&addr).expect("bind http");
        println!("[HTTP] /metrics on http://127.0.0.1:{}/metrics", port);
        for req in server.incoming_requests() {
            let url = req.url().to_string();
            if url.starts_with("/metrics") {
                let mut body = String::new();
                // 输出 batch verify 指标
                body.push_str(&metrics.export_prometheus());
                // 额外附加单笔摘要（仅文本，非 Prometheus 格式）
                body.push_str("\n# Single verify summary (text-only)\n");
                body.push_str(&format!("# single_total_ms {:.3}\n", ms1));
                body.push_str(&format!("# single_avg_ms {:.6}\n", avg1));
                body.push_str(&format!("# single_tps {:.3}\n", tps1));
                let header = Header::from_bytes(&b"Content-Type"[..], &b"text/plain; version=0.0.4"[..]).unwrap();
                let _ = req.respond(Response::from_string(body).with_header(header));
            } else {
                let _ = req.respond(tiny_http::Response::from_string("OK"));
            }
        }
    } else {
        // 控制台输出 Prometheus 片段
        let expo = metrics.export_prometheus();
        for line in expo.lines().filter(|l| l.contains("vm_privacy_zk_batch_verify_")) {
            println!("METRIC {}", line);
        }
    }
}
