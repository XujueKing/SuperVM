// SPDX-License-Identifier: GPL-3.0-or-later
//! RingCT 并行证明 HTTP Bench 示例
//!
//! 运行:
//!   cargo run -p vm-runtime --features groth16-verifier --example zk_parallel_http_bench --release
//! 环境变量:
//!   RINGCT_PAR_BATCH=32            批次大小
//!   RINGCT_PAR_INTERVAL_MS=1000    两批之间休眠间隔
//!   RINGCT_PAR_THREADS=0           自定义线程数 (0 或未设置=rayon 默认)
//!   RINGCT_PAR_SETUP_BATCH=4       初始预热批次数
//! HTTP 端点:
//!   /metrics  导出 Prometheus 指标 (包含并行证明指标 + VM 路由指标可选)
//!   /summary  文本摘要 (最近批次性能)

use std::sync::Arc;
use std::thread;
use std::time::Duration;
use tiny_http::{Header, Response, Server};
use vm_runtime::{MetricsCollector, SuperVM, OwnershipManager};
#[cfg(feature = "groth16-verifier")]
use vm_runtime::privacy::{RingCtParallelProver, RingCtWitness, ParallelProveConfig};

#[cfg(not(feature = "groth16-verifier"))]
fn main() {
    eprintln!("groth16-verifier feature required");
}

#[cfg(feature = "groth16-verifier")]
fn main() {
    println!("=== RingCT 并行证明 HTTP Bench ===");

    // 解析环境变量
    let batch_size: usize = std::env::var("RINGCT_PAR_BATCH").ok().and_then(|v| v.parse().ok()).unwrap_or(32);
    let interval_ms: u64 = std::env::var("RINGCT_PAR_INTERVAL_MS").ok().and_then(|v| v.parse().ok()).unwrap_or(1000);
    let threads_opt: Option<usize> = std::env::var("RINGCT_PAR_THREADS").ok().and_then(|v| v.parse().ok()).filter(|n| *n > 0);
    let setup_batches: usize = std::env::var("RINGCT_PAR_SETUP_BATCH").ok().and_then(|v| v.parse().ok()).unwrap_or(4);

    let metrics = Arc::new(MetricsCollector::new());
    let ownership = OwnershipManager::new();
    let supervm = SuperVM::new(&ownership); // 路由指标可选使用

    // 初始化 RingCT proving key
    println!("[setup] 生成 RingCT ProvingKey...");
    let config = ParallelProveConfig { batch_size, num_threads: threads_opt, collect_individual_latency: true };
    let prover = RingCtParallelProver::with_shared_setup(config.clone()).with_metrics(metrics.clone());
    println!("[setup] 完成");

    // 预热: 运行若干批增量填充 metrics
    println!("[warmup] 预热批次: {}", setup_batches);
    for _ in 0..setup_batches {
        let witnesses: Vec<_> = (0..batch_size).map(|_| RingCtWitness::example()).collect();
        let stats = prover.prove_batch(&witnesses);
        println!("[warmup] ok={} failed={} tps={:.2} avg_ms={:.2}", stats.ok, stats.failed, stats.tps, stats.avg_latency_ms);
    }

    // 后台线程: 周期性执行批次
    let prover_bg = Arc::new(prover); // 共享引用
    thread::spawn(move || {
        loop {
            let witnesses: Vec<_> = (0..batch_size).map(|_| RingCtWitness::example()).collect();
            let stats = prover_bg.prove_batch(&witnesses);
            println!("[batch] total={} ok={} failed={} latency_ms={:.2} tps={:.2}", stats.total, stats.ok, stats.failed, stats.total_duration.as_secs_f64()*1000.0, stats.tps);
            thread::sleep(Duration::from_millis(interval_ms));
        }
    });

    // HTTP 服务
    let server = Server::http("0.0.0.0:9090").expect("bind 9090");
    println!("[HTTP] listening on http://127.0.0.1:9090 (endpoints: /metrics /summary)");

    for req in server.incoming_requests() {
        let url = req.url().to_string();
        if url.starts_with("/metrics") {
            let mut body = metrics.export_prometheus();
            body.push_str("\n");
            body.push_str(&supervm.export_routing_prometheus());
            let header = Header::from_bytes(&b"Content-Type"[..], &b"text/plain; version=0.0.4"[..]).unwrap();
            let _ = req.respond(Response::from_string(body).with_header(header));
        } else if url.starts_with("/summary") {
            // 最近批次指标直接来源于 metrics 的 last batch 字段
            let last_batch_ms = metrics.parallel_last_batch_latency_ms.load(std::sync::atomic::Ordering::Relaxed) as f64 / 1000.0;
            let avg_ms = metrics.parallel_last_batch_avg_latency_ms.load(std::sync::atomic::Ordering::Relaxed) as f64 / 1000.0;
            let tps = metrics.parallel_last_batch_tps.load(std::sync::atomic::Ordering::Relaxed) as f64 / 1000.0;
            let total = metrics.parallel_proof_total.load(std::sync::atomic::Ordering::Relaxed);
            let failed = metrics.parallel_proof_failed.load(std::sync::atomic::Ordering::Relaxed);
            let batches = metrics.parallel_proof_batches.load(std::sync::atomic::Ordering::Relaxed);
            let body = format!(
                "=== RingCT Parallel Proof Summary ===\nTotal Proofs: {}\nFailed Proofs: {}\nBatches: {}\nLast Batch Total Latency (ms): {:.2}\nLast Batch Avg Per-Proof (ms): {:.2}\nLast Batch TPS: {:.2}\n",
                total, failed, batches, last_batch_ms, avg_ms, tps
            );
            let header = Header::from_bytes(&b"Content-Type"[..], &b"text/plain; charset=utf-8"[..]).unwrap();
            let _ = req.respond(Response::from_string(body).with_header(header));
        } else {
            let _ = req.respond(Response::from_string("OK"));
        }
    }
}
