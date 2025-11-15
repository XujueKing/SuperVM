// SPDX-License-Identifier: GPL-3.0-or-later
// Phase 5 Mixed Workload Benchmark
// 目标: 测试 FastPath (Owned/Immutable) + ConsensusPath (Shared) 混合负载下整体吞吐与比例
// 环境变量:
//   MIXED_ITERS       - 事务总数 (默认 200_000)
//   OWNED_OBJECTS     - 独占对象数量 (默认 10_000)
//   SHARED_OBJECTS    - 共享对象数量 (默认 2_000)
//   OWNED_RATIO       - 独占对象事务比例 (默认 0.80) (0.0-1.0)
//   SEED              - 随机种子 (默认 2025)
// 输出指标:
//   Fast Path 成功数, Consensus 成功数, 总吞吐 TPS, FastPath 比例, 冲突率, 成功率

use std::time::Instant;
use std::sync::Arc;
use std::thread;
use tiny_http::{Server, Response, Header};
use rand::{Rng, SeedableRng};
use rand::rngs::StdRng;
use vm_runtime::{OwnershipManager, OwnershipType, ObjectMetadata, Address, ObjectId};
use vm_runtime::{SuperVM, MvccScheduler, Privacy, VmTransaction as Transaction};
use vm_runtime::{Txn, MvccSchedulerConfig};

fn make_addr(id: u64) -> Address {
    let mut a = [0u8;32];
    a[0..8].copy_from_slice(&id.to_le_bytes());
    a
}

fn make_obj(id: u64) -> ObjectId {
    let mut o = [0u8;32];
    o[0..8].copy_from_slice(&id.to_le_bytes());
    o
}

fn main() {
    println!("=== Mixed Path Benchmark (Phase 5) ===\n");
    let iterations: usize = std::env::var("MIXED_ITERS").ok().and_then(|v| v.parse().ok()).unwrap_or(200_000);
    let owned_objects: usize = std::env::var("OWNED_OBJECTS").ok().and_then(|v| v.parse().ok()).unwrap_or(10_000);
    let shared_objects: usize = std::env::var("SHARED_OBJECTS").ok().and_then(|v| v.parse().ok()).unwrap_or(2_000);
    let owned_ratio: f64 = std::env::var("OWNED_RATIO").ok().and_then(|v| v.parse().ok()).unwrap_or(0.80_f64);
    let seed: u64 = std::env::var("SEED").ok().and_then(|v| v.parse().ok()).unwrap_or(2025);

    // 解析可选参数: --serve-metrics[:PORT]
    let mut serve_metrics = false;
    let mut metrics_port: u16 = 8082;
    for arg in std::env::args().skip(1) {
        if let Some(rest) = arg.strip_prefix("--serve-metrics") {
            serve_metrics = true;
            if let Some(p) = rest.strip_prefix(":") {
                if let Ok(v) = p.parse::<u16>() { metrics_port = v; }
            }
        }
    }

    println!("Config: iterations={} owned_objects={} shared_objects={} owned_ratio={:.2} seed={} serve_metrics={} port={}" ,
             iterations, owned_objects, shared_objects, owned_ratio, seed, serve_metrics, metrics_port);

    let mut ownership = OwnershipManager::new();
    let alice = make_addr(1);
    let bob = make_addr(2);

    // 注册独占对象 (Owned)
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

    // 注册共享对象 (Shared)
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

    // 调度器配置 (可调整重试次数与并行度)
    let scheduler = MvccScheduler::new_with_config(MvccSchedulerConfig { max_retries: 3, num_workers: 0, mvcc_config: vm_runtime::GcConfig { max_versions_per_key: 20, enable_time_based_gc: false, version_ttl_secs: 3600, auto_gc: None }});
    let vm = SuperVM::new(&ownership).with_scheduler(&scheduler);

    // 如果需要启动 /metrics 服务
    if serve_metrics {
        let vm_ref = vm.clone();
        let sched_ref = scheduler.clone();
        thread::spawn(move || {
            let addr = format!("0.0.0.0:{}", metrics_port);
            let server = Server::http(&addr).expect("start metrics server");
            println!("[metrics] Listening on http://127.0.0.1:{}/metrics", metrics_port);
            for request in server.incoming_requests() {
                let url = request.url().to_string();
                if url.starts_with("/metrics") {
                    let routing = vm_ref.routing_stats();
                    let fp = vm_ref.fast_path_stats();
                    let stats = sched_ref.get_stats();
                    let mut body = String::new();
                    body.push_str(&vm_ref.export_routing_prometheus());
                    body.push_str("# HELP bench_fastpath_executed_total Fast path executed count (benchmark)\n");
                    body.push_str("# TYPE bench_fastpath_executed_total counter\n");
                    body.push_str(&format!("bench_fastpath_executed_total {}\n", fp.executed_count));
                    body.push_str("# HELP bench_fastpath_failed_total Fast path failed count (benchmark)\n");
                    body.push_str("# TYPE bench_fastpath_failed_total counter\n");
                    body.push_str(&format!("bench_fastpath_failed_total {}\n", fp.failed_count));
                    body.push_str("# HELP bench_fastpath_avg_latency_ns Average fast path latency (ns)\n");
                    body.push_str("# TYPE bench_fastpath_avg_latency_ns gauge\n");
                    body.push_str(&format!("bench_fastpath_avg_latency_ns {}\n", fp.avg_latency_ns));
                    body.push_str("# HELP bench_fastpath_estimated_tps Estimated TPS from avg latency\n");
                    body.push_str("# TYPE bench_fastpath_estimated_tps gauge\n");
                    body.push_str(&format!("bench_fastpath_estimated_tps {:.2}\n", fp.estimated_tps()));
                    body.push_str("# HELP bench_consensus_conflict_rate Consensus conflict rate (benchmark)\n");
                    body.push_str("# TYPE bench_consensus_conflict_rate gauge\n");
                    body.push_str(&format!("bench_consensus_conflict_rate {:.6}\n", stats.conflict_rate()));
                    body.push_str("# HELP bench_consensus_success_rate Consensus success rate (benchmark)\n");
                    body.push_str("# TYPE bench_consensus_success_rate gauge\n");
                    body.push_str(&format!("bench_consensus_success_rate {:.6}\n", stats.success_rate()));
                    // 路由快速摘要 gauge
                    body.push_str("# HELP bench_routing_fast_ratio Fast path ratio snapshot\n# TYPE bench_routing_fast_ratio gauge\n");
                    body.push_str(&format!("bench_routing_fast_ratio {:.6}\n", routing.fast_path_ratio()));
                    body.push_str("# HELP bench_routing_consensus_ratio Consensus path ratio snapshot\n# TYPE bench_routing_consensus_ratio gauge\n");
                    body.push_str(&format!("bench_routing_consensus_ratio {:.6}\n", routing.consensus_path_ratio()));
                    body.push_str("# HELP bench_routing_privacy_ratio Privacy path ratio snapshot\n# TYPE bench_routing_privacy_ratio gauge\n");
                    body.push_str(&format!("bench_routing_privacy_ratio {:.6}\n", routing.privacy_path_ratio()));
                    let header = Header::from_bytes(&b"Content-Type"[..], &b"text/plain; version=0.0.4"[..]).unwrap();
                    let _ = request.respond(Response::from_string(body).with_header(header));
                } else if url.starts_with("/summary") {
                    let routing = vm_ref.routing_stats();
                    let fp = vm_ref.fast_path_stats();
                    let stats = sched_ref.get_stats();
                    let body = format!(
                        "=== Mixed Bench Summary ===\nFast executed={} failed={} avg_latency_ns={} est_tps={:.0}\nConsensus success_rate={:.2}% conflict_rate={:.2}%\nRouting: fast={} consensus={} privacy={} | ratio f={:.2} c={:.2} p={:.2}\n",
                        fp.executed_count, fp.failed_count, fp.avg_latency_ns, fp.estimated_tps(),
                        stats.success_rate()*100.0, stats.conflict_rate()*100.0,
                        routing.fast_path_count, routing.consensus_path_count, routing.privacy_path_count,
                        routing.fast_path_ratio(), routing.consensus_path_ratio(), routing.privacy_path_ratio()
                    );
                    let header = Header::from_bytes(&b"Content-Type"[..], &b"text/plain; charset=utf-8"[..]).unwrap();
                    let _ = request.respond(Response::from_string(body).with_header(header));
                } else {
                    let _ = request.respond(Response::from_string("OK: /metrics /summary").with_status_code(200));
                }
            }
        });
    }

    let mut rng = StdRng::seed_from_u64(seed);

    let mut fast_success = 0u64;
    let mut fast_attempt = 0u64;
    let mut consensus_success = 0u64;
    let mut consensus_attempt = 0u64;

    let start = Instant::now();

    for tx_id in 0..iterations as u64 {
        let roll: f64 = rng.gen();
        if roll < owned_ratio {
            // FastPath: 仅使用一个 Owned 对象
            let idx = rng.gen_range(0..owned_objects as u64);
            let obj = make_obj(idx);
            let tx = Transaction { from: alice, objects: vec![obj], privacy: Privacy::Public };
            fast_attempt += 1;
            let receipt = vm.execute_transaction_routed(
                tx_id,
                &tx,
                || {
                    // 轻量逻辑
                    let mut v = 0u64; for _ in 0..4 { v = v.wrapping_add(1); }
                    Ok((v % 11) as i32)
                },
                |txn: &mut Txn| {
                    // FastPath 不走 txn，这里不会触发
                    txn.write(b"fp".to_vec(), b"v".to_vec());
                    Ok(1)
                }
            );
            if receipt.success { fast_success += 1; }
        } else {
            // ConsensusPath: 加入一个共享对象
            let owned_idx = rng.gen_range(0..owned_objects as u64);
            let shared_idx = rng.gen_range(0..shared_objects as u64);
            let obj_owned = make_obj(owned_idx);
            let obj_shared = make_obj(1_000_000 + shared_idx);
            let tx = Transaction { from: alice, objects: vec![obj_owned, obj_shared], privacy: Privacy::Public };
            consensus_attempt += 1;
            let receipt = vm.execute_transaction_routed(
                tx_id,
                &tx,
                || {
                    // fast_op 不会执行 (路由到 consensus)
                    Ok(0)
                },
                |txn: &mut Txn| {
                    // 简单写入，制造读写集
                    let key = format!("key_{}", tx_id).into_bytes();
                    txn.write(key.clone(), b"val".to_vec());
                    // 读取回调验证
                    let _ = txn.read(&key);
                    Ok(2)
                }
            );
            if receipt.success { consensus_success += 1; }
        }
    }

    let elapsed = start.elapsed();
    let fast_stats = vm.fast_path_stats();
    let consensus_stats = scheduler.get_stats();
    let routing = vm.routing_stats();

    let total_tx = fast_attempt + consensus_attempt;
    let tps = total_tx as f64 / elapsed.as_secs_f64();

    println!("\n=== Result ===");
    println!("Total Txns: {}", total_tx);
    println!("FastPath Attempt / Success: {} / {}", fast_attempt, fast_success);
    println!("Consensus Attempt / Success: {} / {}", consensus_attempt, consensus_success);
    println!("Elapsed: {:.2?}", elapsed);
    println!("Throughput (TPS): {:.0}", tps);
    println!("FastPath Avg Latency (ns): {}", fast_stats.avg_latency_ns);
    println!("FastPath Estimated TPS: {:.0}", fast_stats.estimated_tps());
    println!("Consensus Success Rate: {:.2}%", consensus_stats.success_rate() * 100.0);
    println!("Consensus Conflict Rate: {:.2}%", consensus_stats.conflict_rate() * 100.0);
    println!("Routing Fast/Consensus/Privacy Ratio: {:.2}/{:.2}/{:.2}", routing.fast_path_ratio(), routing.consensus_path_ratio(), routing.privacy_path_ratio());
    if serve_metrics { println!("Metrics server active on :{}  -> /metrics /summary", metrics_port); }
}
