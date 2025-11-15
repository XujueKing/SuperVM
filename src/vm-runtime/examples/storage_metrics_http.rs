// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2025 XujueKing <leadbrand@me.com>

//! Storage Metrics HTTP Server Demo
//! 集成展示：SuperVM 路由 + MVCC + RocksDB 存储层指标通过 /metrics 端点暴露

use std::sync::Arc;
use std::thread;
use std::time::Duration;
use tiny_http::{Header, Response, Server};
use vm_runtime::{
    GcConfig, MvccScheduler, MvccStore, ObjectId, ObjectMetadata, OwnershipManager,
    OwnershipType, Privacy, SuperVM, VmTransaction,
    adaptive_router::{AdaptiveRouter, AdaptiveRouterConfig},
};

#[cfg(not(feature = "rocksdb-storage"))]
fn main() {
    println!("⚠️  此示例需要启用 rocksdb-storage 特性");
    println!("请运行: cargo run --example storage_metrics_http --features rocksdb-storage --release");
}

#[cfg(feature = "rocksdb-storage")]
fn main() {
    use vm_runtime::{RocksDBConfig, RocksDBStorage, Storage};

    println!("=== Storage Metrics HTTP Server ===\n");

    // 1. 初始化 RocksDB
    let db_path = "data/storage_metrics_http";
    std::fs::create_dir_all(db_path).unwrap();
    let rocksdb = Arc::new(
        RocksDBStorage::new(RocksDBConfig::default().with_path(db_path))
            .expect("RocksDB init failed"),
    );
    println!("✅ RocksDB 初始化成功: {}", db_path);

    // 2. 初始化 MVCC Store (启用指标收集 + RocksDB 持久化)
    let gc_config = GcConfig {
        max_versions_per_key: 10,
        enable_time_based_gc: false,
        version_ttl_secs: 3600,
        auto_gc: None,
    };
    let mvcc = Arc::new(MvccStore::new_with_config(gc_config));
    println!("✅ MVCC Store 初始化成功 (指标收集已启用)");

    // 3. 初始化 SuperVM (路由 + 所有权)
    let ownership = Arc::new(OwnershipManager::new());
    let scheduler = Arc::new(MvccScheduler::new_with_store(Arc::clone(&mvcc)));
    let adaptive_cfg = AdaptiveRouterConfig::from_env();
    let adaptive = AdaptiveRouter::new_with_config(adaptive_cfg);
    let supervm = Arc::new(
        SuperVM::new(&ownership)
            .with_scheduler(&scheduler)
            .with_adaptive_router(adaptive),
    );
    println!("✅ SuperVM 初始化成功");

    // 4. 注册测试对象 (Owned + Shared)
    let addr: [u8; 32] = [42u8; 32];
    let owned_id: ObjectId = [0u8; 32];
    let shared_id: ObjectId = [1u8; 32];

    ownership
        .register_object(ObjectMetadata {
            id: owned_id,
            version: 0,
            ownership: OwnershipType::Owned(addr),
            object_type: "TestObj".to_string(),
            created_at: 0,
            updated_at: 0,
            size: 128,
            is_deleted: false,
        })
        .expect("register owned failed");

    ownership
        .register_object(ObjectMetadata {
            id: shared_id,
            version: 0,
            ownership: OwnershipType::Shared,
            object_type: "TestObj".to_string(),
            created_at: 0,
            updated_at: 0,
            size: 128,
            is_deleted: false,
        })
        .expect("register shared failed");

    println!("✅ 测试对象已注册 (Owned + Shared)");

    // 5. 预热路由 & 执行一些事务
    println!("\n📝 预热: 执行 30 个事务...");
    for i in 0..30u32 {
        let route_type = i % 3;
        let tx = match route_type {
            0 => VmTransaction {
                from: addr,
                objects: vec![owned_id],
                privacy: Privacy::Public,
            }, // FastPath
            1 => VmTransaction {
                from: addr,
                objects: vec![shared_id],
                privacy: Privacy::Public,
            }, // Consensus
            _ => VmTransaction {
                from: addr,
                objects: vec![owned_id],
                privacy: Privacy::Private,
            }, // Privacy
        };
        let _ = supervm.route(&tx);

        // 执行 MVCC 写入
        let mvcc_tx = mvcc.begin();
        let mut mvcc_tx = mvcc_tx;
        let key = format!("warmup_key_{}", i);
        let value = format!("warmup_value_{}", i);
        mvcc_tx.write(key.into_bytes(), value.into_bytes());
        if let Ok(commit_ts) = mvcc_tx.commit() {
            // 持久化到 RocksDB
            if i % 10 == 0 {
                let _ = rocksdb.set(&format!("commit_{}", commit_ts).into_bytes(), &value.into_bytes());
            }
        }
    }
    println!("   ✅ 预热完成 (30 txs)");

    // 6. 启动后台指标采集线程
    let rocksdb_clone = Arc::clone(&rocksdb);
    let mvcc_clone = Arc::clone(&mvcc);
    let metrics_thread = thread::spawn(move || {
        loop {
            thread::sleep(Duration::from_secs(5));
            let metrics = rocksdb_clone.collect_metrics();
            mvcc_clone.update_rocksdb_metrics(&metrics);
        }
    });

    println!("\n🚀 HTTP Server 启动中...");
    let server = Server::http("0.0.0.0:9091").unwrap();
    println!("📡 监听 http://127.0.0.1:9091/metrics");
    println!("   端点:");
    println!("      /metrics    - Prometheus 格式指标 (SuperVM + MVCC + RocksDB)");
    println!("      /healthz    - 健康检查");
    println!("      /summary    - 文本格式统计摘要");
    println!("      /trigger    - 手动触发事务 (query: ?count=N&type=fast|consensus|privacy)");
    println!("\n按 Ctrl+C 退出...\n");

    // 7. HTTP 请求处理循环
    for request in server.incoming_requests() {
        let url = request.url().to_string();

        if url.starts_with("/metrics") {
            // 组合导出: SuperVM 路由 + MVCC + RocksDB
            let mut body = String::new();

            // SuperVM 路由指标
            body.push_str(&supervm.export_routing_prometheus());
            body.push('\n');

            // MVCC + RocksDB 指标
            if let Some(mc) = mvcc.get_metrics() {
                body.push_str(&mc.export_prometheus());
            }

            let header = Header::from_bytes(
                &b"Content-Type"[..],
                &b"text/plain; version=0.0.4"[..],
            )
            .unwrap();
            let response = Response::from_string(body).with_header(header);
            let _ = request.respond(response);
        } else if url.starts_with("/healthz") {
            let _ = request.respond(Response::from_string("ok").with_status_code(200));
        } else if url.starts_with("/summary") {
            let stats = supervm.routing_stats();
            let mut body = String::new();
            body.push_str("=== SuperVM Storage Metrics Summary ===\n\n");

            // 路由统计
            body.push_str(&format!("Routing Stats:\n"));
            body.push_str(&format!("  FastPath:   {} ({:.2}%)\n", stats.fast_path_count, stats.fast_path_ratio() * 100.0));
            body.push_str(&format!("  Consensus:  {} ({:.2}%)\n", stats.consensus_path_count, stats.consensus_path_ratio() * 100.0));
            body.push_str(&format!("  Privacy:    {} ({:.2}%)\n\n", stats.privacy_path_count, stats.privacy_path_ratio() * 100.0));

            // MVCC 统计
            if let Some(mc) = mvcc.get_metrics() {
                use std::sync::atomic::Ordering;
                let committed = mc.txn_committed.load(Ordering::Relaxed);
                let aborted = mc.txn_aborted.load(Ordering::Relaxed);
                body.push_str(&format!("MVCC Stats:\n"));
                body.push_str(&format!("  Committed:  {}\n", committed));
                body.push_str(&format!("  Aborted:    {}\n", aborted));
                body.push_str(&format!("  Success Rate: {:.2}%\n", mc.success_rate()));
                body.push_str(&format!("  TPS:        {:.0}\n\n", mc.tps()));

                // RocksDB 统计
                let num_keys = mc.rocksdb_estimate_num_keys.load(Ordering::Relaxed);
                let sst_size = mc.rocksdb_total_sst_size_bytes.load(Ordering::Relaxed);
                let cache_hit = mc.rocksdb_cache_hit.load(Ordering::Relaxed);
                let cache_miss = mc.rocksdb_cache_miss.load(Ordering::Relaxed);
                let cache_total = cache_hit + cache_miss;
                let hit_rate = if cache_total > 0 {
                    (cache_hit as f64 / cache_total as f64) * 100.0
                } else {
                    0.0
                };

                body.push_str(&format!("RocksDB Stats:\n"));
                body.push_str(&format!("  Estimated Keys: {}\n", num_keys));
                body.push_str(&format!("  SST Size:       {:.2} MB\n", sst_size as f64 / 1024.0 / 1024.0));
                body.push_str(&format!("  Cache Hit Rate: {:.2}%\n", hit_rate));
                body.push_str(&format!("  Cache Hit:      {}\n", cache_hit));
                body.push_str(&format!("  Cache Miss:     {}\n", cache_miss));
            }

            let header = Header::from_bytes(
                &b"Content-Type"[..],
                &b"text/plain; charset=utf-8"[..],
            )
            .unwrap();
            let response = Response::from_string(body).with_header(header);
            let _ = request.respond(response);
        } else if url.starts_with("/trigger") {
            // /trigger?count=N&type=fast|consensus|privacy
            let parts: Vec<&str> = url.splitn(2, '?').collect();
            let query = if parts.len() == 2 { parts[1] } else { "" };

            let mut count = 10u32;
            let mut tx_type = "fast";

            for pair in query.split('&') {
                let kv: Vec<&str> = pair.splitn(2, '=').collect();
                if kv.len() == 2 {
                    match kv[0] {
                        "count" => count = kv[1].parse().unwrap_or(10),
                        "type" => tx_type = kv[1],
                        _ => {}
                    }
                }
            }

            for i in 0..count {
                let tx = match tx_type {
                    "consensus" => VmTransaction {
                        from: addr,
                        objects: vec![shared_id],
                        privacy: Privacy::Public,
                    },
                    "privacy" => VmTransaction {
                        from: addr,
                        objects: vec![owned_id],
                        privacy: Privacy::Private,
                    },
                    _ => VmTransaction {
                        from: addr,
                        objects: vec![owned_id],
                        privacy: Privacy::Public,
                    },
                };
                let _ = supervm.route(&tx);

                // MVCC 写入 + RocksDB 持久化
                let mvcc_tx = mvcc.begin();
                let mut mvcc_tx = mvcc_tx;
                let key = format!("trigger_{}_{}", tx_type, i);
                let value = format!("value_{}", i);
                mvcc_tx.write(key.clone().into_bytes(), value.clone().into_bytes());
                if let Ok(commit_ts) = mvcc_tx.commit() {
                    let _ = rocksdb.set(&format!("{}_{}", key, commit_ts).into_bytes(), &value.into_bytes());
                }
            }

            let body = format!("ok: triggered {} {} transactions", count, tx_type);
            let _ = request.respond(Response::from_string(body).with_status_code(200));
        } else {
            let body = "Storage Metrics HTTP Server\nEndpoints: /metrics, /summary, /healthz, /trigger?count=N&type=fast|consensus|privacy";
            let _ = request.respond(Response::from_string(body).with_status_code(200));
        }
    }

    // 清理 (实际不会到达，因为 Ctrl+C 会中断)
    drop(metrics_thread);
}
