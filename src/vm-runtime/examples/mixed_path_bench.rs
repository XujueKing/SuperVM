// SPDX-License-Identifier: GPL-3.0-or-later
// Phase 5 Mixed Workload Benchmark with optional /metrics
// Usage:
//   cargo run -p vm-runtime --example mixed_path_bench --release -- --serve-metrics:8082
// Env:
//   MIXED_ITERS, OWNED_OBJECTS, SHARED_OBJECTS, OWNED_RATIO, SEED

use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};
use std::thread;
use std::time::Instant;
use tiny_http::{Header, Response, Server};
use vm_runtime::{
    GcConfig, MvccScheduler, MvccSchedulerConfig, OwnershipManager, OwnershipType, SuperVM, Txn,
    adaptive_router::AdaptiveRouter,
};
use vm_runtime::{ObjectId, ObjectMetadata, Privacy, VmTransaction as Transaction};

use parking_lot::Mutex;
use std::sync::Arc;

#[derive(Clone, Default)]
struct BenchSnapshot {
    fast_executed: u64,
    fast_failed: u64,
    fast_avg_latency_ns: u64,
    fast_estimated_tps: f64,
    privacy_executed: u64,
    privacy_failed: u64,
    privacy_avg_latency_ns: u64,
    privacy_estimated_tps: f64,
    consensus_success_rate: f64,
    consensus_conflict_rate: f64,
    routing_fast: u64,
    routing_consensus: u64,
    routing_privacy: u64,
}

fn make_addr(id: u64) -> [u8; 32] {
    let mut a = [0u8; 32];
    a[0..8].copy_from_slice(&id.to_le_bytes());
    a
}

fn make_obj(id: u64) -> ObjectId {
    let mut o = [0u8; 32];
    o[0..8].copy_from_slice(&id.to_le_bytes());
    o
}

fn main() {
    println!("=== Mixed Path Benchmark (Phase 5) ===\n");
    let mut iterations: usize = std::env::var("MIXED_ITERS")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(200_000);
    let owned_objects: usize = std::env::var("OWNED_OBJECTS")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(10_000);
    let shared_objects: usize = std::env::var("SHARED_OBJECTS")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(2_000);
    let owned_ratio: f64 = std::env::var("OWNED_RATIO")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(0.80_f64);
    let seed: u64 = std::env::var("SEED")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(2025);
    // 可选隐私事务比例（默认 0.0）
    let mut privacy_ratio: f64 = std::env::var("PRIVACY_RATIO")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(0.0);
    // 可选隐私延迟（毫秒，默认 0）用于模拟 ZK 验证时间
    let mut privacy_latency_ms: u64 = std::env::var("PRIVACY_LATENCY_MS")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(0);

    // Optional: --serve-metrics[:PORT]
    let mut serve_metrics = false;
    let mut metrics_port: u16 = 8082;
    for arg in std::env::args().skip(1) {
        if let Some(rest) = arg.strip_prefix("--serve-metrics") {
            serve_metrics = true;
            if let Some(p) = rest.strip_prefix(":") {
                if let Ok(v) = p.parse::<u16>() {
                    metrics_port = v;
                }
            }
        } else if let Some(rest) = arg.strip_prefix("--txs") {
            if let Some(val) = rest.strip_prefix(":") {
                if let Ok(n) = val.parse::<usize>() { iterations = n; }
            } else if let Ok(n) = rest.parse::<usize>() { iterations = n; }
        } else if let Some(rest) = arg.strip_prefix("--privacy-ratio") {
            if let Some(val) = rest.strip_prefix(":") {
                if let Ok(r) = val.parse::<f64>() { privacy_ratio = r; }
            }
        } else if let Some(rest) = arg.strip_prefix("--privacy-latency-ms") {
            if let Some(val) = rest.strip_prefix(":") {
                if let Ok(ms) = val.parse::<u64>() { privacy_latency_ms = ms; }
            }
        }
    }

    println!(
        "Config: iterations={} owned_objects={} shared_objects={} owned_ratio={:.2} privacy_ratio={:.2} privacy_latency_ms={} seed={} serve_metrics={} port={}",
        iterations, owned_objects, shared_objects, owned_ratio, privacy_ratio, privacy_latency_ms, seed, serve_metrics, metrics_port
    );

    let ownership = OwnershipManager::new();
    let alice = make_addr(1);

    // Owned objects
    for i in 0..owned_objects as u64 {
        let meta = ObjectMetadata {
            id: make_obj(i),
            version: 0,
            ownership: OwnershipType::Owned(alice),
            object_type: "OwnedAsset".into(),
            created_at: 0,
            updated_at: 0,
            size: 64,
            is_deleted: false,
        };
        ownership.register_object(meta).unwrap();
    }

    // Shared objects
    for i in 0..shared_objects as u64 {
        let meta = ObjectMetadata {
            id: make_obj(1_000_000 + i),
            version: 0,
            ownership: OwnershipType::Shared,
            object_type: "SharedPool".into(),
            created_at: 0,
            updated_at: 0,
            size: 256,
            is_deleted: false,
        };
        ownership.register_object(meta).unwrap();
    }

    // Scheduler
    let scheduler = MvccScheduler::new_with_config(MvccSchedulerConfig {
        max_retries: 3,
        num_workers: 0,
        mvcc_config: GcConfig {
            max_versions_per_key: 20,
            enable_time_based_gc: false,
            version_ttl_secs: 3600,
            auto_gc: None,
        },
    });
    // 启用自适应路由（最小集成）
    let adaptive = AdaptiveRouter::new();
    let vm = SuperVM::new(&ownership)
        .with_scheduler(&scheduler)
        .with_adaptive_router(adaptive);

    // Optional metrics server via snapshot
    let mut snap_opt: Option<Arc<Mutex<BenchSnapshot>>> = None;
    if serve_metrics {
        let snap = Arc::new(Mutex::new(BenchSnapshot::default()));
        let snap_reader = snap.clone();
        snap_opt = Some(snap);
        thread::spawn(move || {
            let addr = format!("0.0.0.0:{}", metrics_port);
            let server = Server::http(&addr).expect("start metrics server");
            println!(
                "[metrics] Listening on http://127.0.0.1:{}/metrics (Mixed Bench)",
                metrics_port
            );
            for request in server.incoming_requests() {
                let url = request.url().to_string();
                if url.starts_with("/metrics") {
                    let s = snap_reader.lock();
                    let total = s.routing_fast + s.routing_consensus + s.routing_privacy;
                    let fr = if total > 0 { s.routing_fast as f64 / total as f64 } else { 0.0 };
                    let cr = if total > 0 { s.routing_consensus as f64 / total as f64 } else { 0.0 };
                    let pr = if total > 0 { s.routing_privacy as f64 / total as f64 } else { 0.0 };
                    let mut body = String::new();
                    body.push_str("# HELP vm_routing_fast_total Total number of transactions routed to fast path\n");
                    body.push_str("# TYPE vm_routing_fast_total counter\n");
                    body.push_str(&format!("vm_routing_fast_total {}\n", s.routing_fast));
                    body.push_str("# HELP vm_routing_consensus_total Total number of transactions routed to consensus path\n");
                    body.push_str("# TYPE vm_routing_consensus_total counter\n");
                    body.push_str(&format!("vm_routing_consensus_total {}\n", s.routing_consensus));
                    body.push_str("# HELP vm_routing_privacy_total Total number of transactions routed to privacy path\n");
                    body.push_str("# TYPE vm_routing_privacy_total counter\n");
                    body.push_str(&format!("vm_routing_privacy_total {}\n", s.routing_privacy));
                    body.push_str("# HELP vm_routing_fast_ratio Ratio of fast path routed transactions\n");
                    body.push_str("# TYPE vm_routing_fast_ratio gauge\n");
                    body.push_str(&format!("vm_routing_fast_ratio {:.6}\n", fr));
                    body.push_str("# HELP vm_routing_consensus_ratio Ratio of consensus path routed transactions\n");
                    body.push_str("# TYPE vm_routing_consensus_ratio gauge\n");
                    body.push_str(&format!("vm_routing_consensus_ratio {:.6}\n", cr));
                    body.push_str("# HELP vm_routing_privacy_ratio Ratio of privacy path routed transactions\n");
                    body.push_str("# TYPE vm_routing_privacy_ratio gauge\n");
                    body.push_str(&format!("vm_routing_privacy_ratio {:.6}\n", pr));
                    body.push_str("# HELP bench_fastpath_executed_total Fast path executed count (benchmark)\n");
                    body.push_str("# TYPE bench_fastpath_executed_total counter\n");
                    body.push_str(&format!("bench_fastpath_executed_total {}\n", s.fast_executed));
                    body.push_str("# HELP bench_fastpath_failed_total Fast path failed count (benchmark)\n");
                    body.push_str("# TYPE bench_fastpath_failed_total counter\n");
                    body.push_str(&format!("bench_fastpath_failed_total {}\n", s.fast_failed));
                    body.push_str("# HELP bench_fastpath_avg_latency_ns Average fast path latency (ns)\n");
                    body.push_str("# TYPE bench_fastpath_avg_latency_ns gauge\n");
                    body.push_str(&format!("bench_fastpath_avg_latency_ns {}\n", s.fast_avg_latency_ns));
                    body.push_str("# HELP bench_fastpath_estimated_tps Estimated TPS from avg latency\n");
                    body.push_str("# TYPE bench_fastpath_estimated_tps gauge\n");
                    body.push_str(&format!("bench_fastpath_estimated_tps {:.2}\n", s.fast_estimated_tps));
                    body.push_str("# HELP bench_privacy_executed_total Privacy path executed count (benchmark)\n");
                    body.push_str("# TYPE bench_privacy_executed_total counter\n");
                    body.push_str(&format!("bench_privacy_executed_total {}\n", s.privacy_executed));
                    body.push_str("# HELP bench_privacy_failed_total Privacy path failed count (benchmark)\n");
                    body.push_str("# TYPE bench_privacy_failed_total counter\n");
                    body.push_str(&format!("bench_privacy_failed_total {}\n", s.privacy_failed));
                    body.push_str("# HELP bench_privacy_avg_latency_ns Average privacy path latency (ns)\n");
                    body.push_str("# TYPE bench_privacy_avg_latency_ns gauge\n");
                    body.push_str(&format!("bench_privacy_avg_latency_ns {}\n", s.privacy_avg_latency_ns));
                    body.push_str("# HELP bench_privacy_estimated_tps Estimated privacy path TPS from avg latency\n");
                    body.push_str("# TYPE bench_privacy_estimated_tps gauge\n");
                    body.push_str(&format!("bench_privacy_estimated_tps {:.2}\n", s.privacy_estimated_tps));
                    body.push_str("# HELP bench_consensus_conflict_rate Consensus conflict rate (benchmark)\n");
                    body.push_str("# TYPE bench_consensus_conflict_rate gauge\n");
                    body.push_str(&format!("bench_consensus_conflict_rate {:.6}\n", s.consensus_conflict_rate));
                    body.push_str("# HELP bench_consensus_success_rate Consensus success rate (benchmark)\n");
                    body.push_str("# TYPE bench_consensus_success_rate gauge\n");
                    body.push_str(&format!("bench_consensus_success_rate {:.6}\n", s.consensus_success_rate));
                    let header = Header::from_bytes(&b"Content-Type"[..], &b"text/plain; version=0.0.4"[..]).unwrap();
                    let _ = request.respond(Response::from_string(body).with_header(header));
                } else if url.starts_with("/summary") {
                    let s = snap_reader.lock();
                    let total = s.routing_fast + s.routing_consensus + s.routing_privacy;
                    let fr = if total > 0 { s.routing_fast as f64 / total as f64 } else { 0.0 };
                    let cr = if total > 0 { s.routing_consensus as f64 / total as f64 } else { 0.0 };
                    let pr = if total > 0 { s.routing_privacy as f64 / total as f64 } else { 0.0 };
                    let body = format!(
                        "=== Mixed Bench Summary ===\nFast executed={} failed={} avg_latency_ns={} est_tps={:.0}\nPrivacy executed={} failed={} avg_latency_ns={} est_tps={:.0}\nConsensus success_rate={:.2}% conflict_rate={:.2}%\nRouting: fast={} consensus={} privacy={} | ratio f={:.2} c={:.2} p={:.2}\n",
                        s.fast_executed, s.fast_failed, s.fast_avg_latency_ns, s.fast_estimated_tps,
                        s.privacy_executed, s.privacy_failed, s.privacy_avg_latency_ns, s.privacy_estimated_tps,
                        s.consensus_success_rate * 100.0, s.consensus_conflict_rate * 100.0,
                        s.routing_fast, s.routing_consensus, s.routing_privacy, fr, cr, pr
                    );
                    let header = Header::from_bytes(&b"Content-Type"[..], &b"text/plain; charset=utf-8"[..]).unwrap();
                    let _ = request.respond(Response::from_string(body).with_header(header));
                } else {
                    let _ = request.respond(Response::from_string("OK: /metrics /summary").with_status_code(200));
                }
            }
        });
        // Attach a sampler to update snapshot in the main loop (below)
    }

    let mut rng = StdRng::seed_from_u64(seed);

    let mut fast_success = 0u64;
    let mut fast_attempt = 0u64;
    let mut consensus_success = 0u64;
    let mut consensus_attempt = 0u64;

    // Privacy path local stats (since we don't have a built-in collector yet)
    let mut privacy_executed = 0u64;
    let mut privacy_failed = 0u64;
    let mut privacy_lat_ns_sum: u128 = 0;

    let start = Instant::now();

    for tx_id in 0..iterations as u64 {
        let roll: f64 = rng.gen();
        let roll_priv: f64 = rng.gen();
        if roll_priv < privacy_ratio {
            // Privacy 路径事务（对象选择对路由无影响，这里选 1 个 owned 对象）
            let idx = rng.gen_range(0..owned_objects as u64);
            let obj = make_obj(idx);
            let tx = Transaction { from: alice, objects: vec![obj], privacy: Privacy::Private };
            // 走共识执行器分支（当前版本 Private 复用共识执行器）
            consensus_attempt += 1;
            let p_start = Instant::now();
            let receipt = vm.execute_transaction_routed(
                tx_id,
                &tx,
                || Ok(0),
                |txn: &mut Txn| {
                    // 模拟隐私写入
                    if privacy_latency_ms > 0 { std::thread::sleep(std::time::Duration::from_millis(privacy_latency_ms)); }
                    let key = format!("zk_{}", tx_id).into_bytes();
                    txn.write(key, b"proof_ok".to_vec());
                    Ok(3)
                },
            );
            let p_dur = p_start.elapsed();
            privacy_lat_ns_sum = privacy_lat_ns_sum.saturating_add(p_dur.as_nanos() as u128);
            if receipt.success { 
                consensus_success += 1; 
                privacy_executed += 1;
            } else {
                privacy_failed += 1;
            }
        } else if roll < owned_ratio {
            // FastPath
            let idx = rng.gen_range(0..owned_objects as u64);
            let obj = make_obj(idx);
            let tx = Transaction { from: alice, objects: vec![obj], privacy: Privacy::Public };
            fast_attempt += 1;
            let receipt = vm.execute_transaction_routed(
                tx_id,
                &tx,
                || {
                    let mut v = 0u64;
                    for _ in 0..4 {
                        v = v.wrapping_add(1);
                    }
                    Ok((v % 11) as i32)
                },
                |txn: &mut Txn| {
                    txn.write(b"fp".to_vec(), b"v".to_vec());
                    Ok(1)
                },
            );
            if receipt.success {
                fast_success += 1;
            }
        } else {
            // Consensus
            let owned_idx = rng.gen_range(0..owned_objects as u64);
            let shared_idx = rng.gen_range(0..shared_objects as u64);
            let obj_owned = make_obj(owned_idx);
            let obj_shared = make_obj(1_000_000 + shared_idx);
            let tx = Transaction {
                from: alice,
                objects: vec![obj_owned, obj_shared],
                privacy: Privacy::Public,
            };
            consensus_attempt += 1;
            let receipt = vm.execute_transaction_routed(
                tx_id,
                &tx,
                || { Ok(0) },
                |txn: &mut Txn| {
                    let key = format!("key_{}", tx_id).into_bytes();
                    txn.write(key.clone(), b"val".to_vec());
                    let _ = txn.read(&key);
                    Ok(2)
                },
            );
            if receipt.success {
                consensus_success += 1;
            }
        }

        // Update metrics snapshot every 1000 txs
        if serve_metrics && tx_id % 1000 == 0 {
            if let Some(ref snap_arc) = snap_opt {
                let mut s = snap_arc.lock();
                let fp_stats = vm.fast_path_stats();
                let cons_stats = scheduler.get_stats();
                let routing_stats = vm.routing_stats();
                s.fast_executed = fp_stats.executed_count;
                s.fast_failed = fp_stats.failed_count;
                s.fast_avg_latency_ns = fp_stats.avg_latency_ns;
                s.fast_estimated_tps = fp_stats.estimated_tps();
                let p_count = privacy_executed + privacy_failed;
                s.privacy_executed = privacy_executed;
                s.privacy_failed = privacy_failed;
                s.privacy_avg_latency_ns = if p_count > 0 { (privacy_lat_ns_sum / p_count as u128) as u64 } else { 0 };
                s.privacy_estimated_tps = if s.privacy_avg_latency_ns > 0 { 1_000_000_000.0f64 / (s.privacy_avg_latency_ns as f64) } else { 0.0 };
                s.consensus_success_rate = cons_stats.success_rate();
                s.consensus_conflict_rate = cons_stats.conflict_rate();
                s.routing_fast = routing_stats.fast_path_count;
                s.routing_consensus = routing_stats.consensus_path_count;
                s.routing_privacy = routing_stats.privacy_path_count;
            }
        }
    }

    let elapsed = start.elapsed();
    let fast_stats = vm.fast_path_stats();
    let consensus_stats = scheduler.get_stats();
    let routing = vm.routing_stats();

    if let Some(ref snap_arc) = snap_opt {
        let mut s = snap_arc.lock();
        s.fast_executed = fast_stats.executed_count;
        s.fast_failed = fast_stats.failed_count;
        s.fast_avg_latency_ns = fast_stats.avg_latency_ns;
        s.fast_estimated_tps = fast_stats.estimated_tps();
        let p_count = privacy_executed + privacy_failed;
        s.privacy_executed = privacy_executed;
        s.privacy_failed = privacy_failed;
        s.privacy_avg_latency_ns = if p_count > 0 { (privacy_lat_ns_sum / p_count as u128) as u64 } else { 0 };
        s.privacy_estimated_tps = if s.privacy_avg_latency_ns > 0 { 1_000_000_000.0f64 / (s.privacy_avg_latency_ns as f64) } else { 0.0 };
        s.consensus_success_rate = consensus_stats.success_rate();
        s.consensus_conflict_rate = consensus_stats.conflict_rate();
        s.routing_fast = routing.fast_path_count;
        s.routing_consensus = routing.consensus_path_count;
        s.routing_privacy = routing.privacy_path_count;
    }

    let total_tx = fast_attempt + consensus_attempt;
    let tps = total_tx as f64 / elapsed.as_secs_f64();

    println!("\n=== Result ===");
    println!("Total Txns: {}", total_tx);
    println!(
        "FastPath Attempt / Success: {} / {}",
        fast_attempt, fast_success
    );
    println!(
        "Consensus Attempt / Success: {} / {}",
        consensus_attempt, consensus_success
    );
    println!("Elapsed: {:.2?}", elapsed);
    println!("Throughput (TPS): {:.0}", tps);
    println!("FastPath Avg Latency (ns): {}", fast_stats.avg_latency_ns);
    println!(
        "FastPath Estimated TPS: {:.0}",
        fast_stats.estimated_tps()
    );
    println!(
        "Consensus Success Rate: {:.2}%",
        consensus_stats.success_rate() * 100.0
    );
    println!(
        "Consensus Conflict Rate: {:.2}%",
        consensus_stats.conflict_rate() * 100.0
    );
    println!(
        "Routing Fast/Consensus/Privacy Ratio: {:.2}/{:.2}/{:.2}",
        routing.fast_path_ratio(),
        routing.consensus_path_ratio(),
        routing.privacy_path_ratio()
    );
    if serve_metrics {
        println!(
            "Metrics server active on :{} -> /metrics /summary",
            metrics_port
        );
        println!("Press Ctrl+C to stop...");
        loop {
            std::thread::sleep(std::time::Duration::from_secs(10));
        }
    }
}
