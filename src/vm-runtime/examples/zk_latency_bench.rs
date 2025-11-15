// SPDX-License-Identifier: GPL-3.0-or-later
// ZK Latency Bench: run real Groth16 verification in a loop and expose metrics via /metrics

#[cfg(feature = "groth16-verifier")]
fn main() -> Result<(), Box<dyn std::error::Error>> {
    use std::sync::atomic::{AtomicU64, Ordering};
    use std::sync::{Arc, Mutex};
    use std::thread;
    use std::time::{Duration, Instant};
    use tiny_http::{Header, Response, Server};
    use vm_runtime::zk_verifier::{generate_test_proof, Groth16Verifier, ZkVerifier};
    use std::collections::VecDeque;

    println!("=== SuperVM ZK Latency Bench (Groth16) ===\n");

    // 1) 初始化验证器与测试用 proof
    let verifier = Arc::new(Groth16Verifier::new_for_testing()?);
    let (proof_bytes, public_input_bytes) = generate_test_proof()?;

    // 2) 简化实现：直接用验证器而非通过 SuperVM（避免生命周期复杂度）
    //    在后台线程持续验证并记录指标到 Arc<AtomicU64> 共享计数器
    let zk_count = Arc::new(AtomicU64::new(0));
    let zk_total_ns = Arc::new(AtomicU64::new(0));
    let zk_last_ns = Arc::new(AtomicU64::new(0));

    // 3) 后台线程：持续执行验证
    let qps: u64 = std::env::var("SUPERVM_ZK_BENCH_QPS")
        .ok()
        .and_then(|v| v.parse::<u64>().ok())
        .unwrap_or(50);
    let interval = if qps > 0 {
        Duration::from_secs_f64(1.0 / qps as f64)
    } else {
        Duration::from_millis(0)
    };

    // 滑动窗口配置：用于计算 p50/p95（单位：ns）
    let win_cap: usize = std::env::var("SUPERVM_ZK_LAT_WIN")
        .ok()
        .and_then(|v| v.parse::<usize>().ok())
        .unwrap_or(64);
    let zk_window: Arc<Mutex<VecDeque<u64>>> = Arc::new(Mutex::new(VecDeque::with_capacity(win_cap.max(1))));

    let proof = proof_bytes.clone();
    let pubinp = public_input_bytes.clone();
    let ver_bg = verifier.clone();
    let count_bg = zk_count.clone();
    let total_bg = zk_total_ns.clone();
    let last_bg = zk_last_ns.clone();
    let win_bg = zk_window.clone();

    thread::spawn(move || loop {
        let t0 = Instant::now();
        let _ = ver_bg.verify(&proof, &pubinp);
        let elapsed = t0.elapsed().as_nanos() as u64;
        count_bg.fetch_add(1, Ordering::Relaxed);
        total_bg.fetch_add(elapsed, Ordering::Relaxed);
        last_bg.store(elapsed, Ordering::Relaxed);
        // 推入滑动窗口
        if let Ok(mut w) = win_bg.lock() {
            if w.len() >= win_cap && win_cap > 0 { w.pop_front(); }
            w.push_back(elapsed);
        }
        let spent = t0.elapsed();
        if spent < interval {
            thread::sleep(interval - spent);
        }
    });

    // 4) 前台导出指标（手工 Prometheus 格式）
    // 端口可通过 SUPERVM_ZK_BENCH_PORT 配置，默认 8083
    let port: u16 = std::env::var("SUPERVM_ZK_BENCH_PORT")
        .ok()
        .and_then(|v| v.parse::<u16>().ok())
        .unwrap_or(8083);
    let bind_addr = format!("0.0.0.0:{}", port);
    let server = Server::http(&bind_addr).unwrap();
    println!("[HTTP] Metrics on http://127.0.0.1:{}/metrics (Ctrl+C 退出) ...", port);

    for request in server.incoming_requests() {
        let url = request.url().to_string();
        if url.starts_with("/metrics") {
            let count = zk_count.load(Ordering::Relaxed);
            let total = zk_total_ns.load(Ordering::Relaxed);
            let last = zk_last_ns.load(Ordering::Relaxed);
            let avg_ms = if count > 0 {
                (total as f64) / (count as f64) / 1_000_000.0
            } else {
                0.0
            };
            let last_ms = last as f64 / 1_000_000.0;

            // 计算 p50 / p95 与窗口大小
            let (p50_ms, p95_ms, win_len) = if let Ok(w) = zk_window.lock() {
                let mut v: Vec<u64> = w.iter().copied().collect();
                let n = v.len();
                if n == 0 {
                    (0.0, 0.0, 0)
                } else {
                    v.sort_unstable();
                    let idx50 = ((n as f64) * 0.5).floor() as usize;
                    let idx95 = (((n as f64) * 0.95).ceil() as usize).saturating_sub(1).min(n - 1);
                    let p50_ns = v[idx50.min(n-1)];
                    let p95_ns = v[idx95];
                    (p50_ns as f64 / 1_000_000.0, p95_ns as f64 / 1_000_000.0, n)
                }
            } else { (0.0, 0.0, 0) };

            let mut body = String::new();
            body.push_str("# HELP vm_privacy_zk_verify_count_total Total ZK verifications\n");
            body.push_str("# TYPE vm_privacy_zk_verify_count_total counter\n");
            body.push_str(&format!("vm_privacy_zk_verify_count_total {}\n", count));
            body.push_str("# HELP vm_privacy_zk_verify_avg_latency_ms Average latency (ms)\n");
            body.push_str("# TYPE vm_privacy_zk_verify_avg_latency_ms gauge\n");
            body.push_str(&format!("vm_privacy_zk_verify_avg_latency_ms {:.6}\n", avg_ms));
            body.push_str("# HELP vm_privacy_zk_verify_last_latency_ms Last latency (ms)\n");
            body.push_str("# TYPE vm_privacy_zk_verify_last_latency_ms gauge\n");
            body.push_str(&format!("vm_privacy_zk_verify_last_latency_ms {:.6}\n", last_ms));
            // 额外导出 p50 / p95 / window_size（与 SuperVM 命名保持一致）
            body.push_str("# HELP vm_privacy_zk_verify_p50_latency_ms p50 (median) ZK verification latency over recent window\n");
            body.push_str("# TYPE vm_privacy_zk_verify_p50_latency_ms gauge\n");
            body.push_str(&format!("vm_privacy_zk_verify_p50_latency_ms {:.6}\n", p50_ms));
            body.push_str("# HELP vm_privacy_zk_verify_p95_latency_ms p95 ZK verification latency over recent window\n");
            body.push_str("# TYPE vm_privacy_zk_verify_p95_latency_ms gauge\n");
            body.push_str(&format!("vm_privacy_zk_verify_p95_latency_ms {:.6}\n", p95_ms));
            body.push_str("# HELP vm_privacy_zk_verify_window_size Number of samples in ZK latency sliding window\n");
            body.push_str("# TYPE vm_privacy_zk_verify_window_size gauge\n");
            body.push_str(&format!("vm_privacy_zk_verify_window_size {}\n", win_len));

            let header = Header::from_bytes(&b"Content-Type"[..], &b"text/plain; version=0.0.4"[..]).unwrap();
            let response = Response::from_string(body).with_header(header);
            let _ = request.respond(response);
        } else {
            let body = "OK. Endpoint: /metrics (Prometheus)";
            let _ = request.respond(Response::from_string(body).with_status_code(200));
        }
    }

    Ok(())
}

#[cfg(not(feature = "groth16-verifier"))]
fn main() {
    eprintln!("❌ This bench requires the 'groth16-verifier' feature.");
    eprintln!("   Run with: cargo run -p vm-runtime --example zk_latency_bench --features groth16-verifier --release");
    std::process::exit(1);
}
