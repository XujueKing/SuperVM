// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2025 XujueKing <leadbrand@me.com>

//! HTTP /metrics 端点 Demo
//! 通过 tiny_http 暴露 Prometheus 格式指标

use std::sync::Arc;
use tiny_http::{Header, Response, Server};
use vm_runtime::{
    MvccStore, ObjectId, ObjectMetadata, OwnershipManager, OwnershipType, Privacy, SuperVM,
    VmTransaction,
};

fn main() {
    println!("=== MVCC Store Metrics HTTP Demo ===\n");
    let store = Arc::new(MvccStore::new());

    // ==== 路由指标：构造 SuperVM + 预热统计 ====
    let ownership = Arc::new(OwnershipManager::new());
    let supervm = Arc::new(SuperVM::new(&ownership)); // 此示例仅使用路由统计，不需要 scheduler

    // 注册 owned/shared 对象并进行路由，以便打开页面就有可见数据
    {
        let addr: [u8; 32] = [7u8; 32];
        // Owned 对象
        let owned_id: ObjectId = [11u8; 32];
        let owned_meta = ObjectMetadata {
            id: owned_id,
            version: 0,
            ownership: OwnershipType::Owned(addr),
            object_type: "DemoObj".to_string(),
            created_at: 0,
            updated_at: 0,
            size: 64,
            is_deleted: false,
        };
        ownership
            .register_object(owned_meta)
            .expect("register owned failed");

        // Shared 对象
        let shared_id: ObjectId = [12u8; 32];
        let shared_meta = ObjectMetadata {
            id: shared_id,
            version: 0,
            ownership: OwnershipType::Shared,
            object_type: "DemoObj".to_string(),
            created_at: 0,
            updated_at: 0,
            size: 64,
            is_deleted: false,
        };
        ownership
            .register_object(shared_meta)
            .expect("register shared failed");

        // 预热路由计数
        for _ in 0..10u32 {
            let tx = VmTransaction {
                from: addr,
                objects: vec![owned_id],
                privacy: Privacy::Public,
            };
            let _ = supervm.route(&tx);
        }
        for _ in 0..10u32 {
            let tx = VmTransaction {
                from: addr,
                objects: vec![shared_id],
                privacy: Privacy::Public,
            };
            let _ = supervm.route(&tx);
        }
        for _ in 0..10u32 {
            let tx = VmTransaction {
                from: addr,
                objects: vec![owned_id],
                privacy: Privacy::Private,
            };
            let _ = supervm.route(&tx);
        }
    }

    // 启动 HTTP 服务器（前台）
    let server = Server::http("0.0.0.0:8080").unwrap();
    println!("[HTTP] 监听 http://127.0.0.1:8080/metrics ...");

    // 主线程执行事务，产生指标
    println!("📝 执行测试事务以生成指标...");
    for i in 0..50 {
        let mut tx = store.begin();
        tx.write(
            format!("key_{}", i).into_bytes(),
            format!("value_{}", i).into_bytes(),
        );
        let _ = tx.commit();
    }
    println!("[主线程] 事务写入完成，开始提供 HTTP 服务: \n  - http://127.0.0.1:8080/metrics  (Prometheus: MVCC + Routing)\n  - http://127.0.0.1:8080/summary  (文本摘要: TPS 窗口/峰值 + P50/P90/P99 + 路由比例)\n  - http://127.0.0.1:8080/routing/reset (重置路由计数)\n");

    for request in server.incoming_requests() {
        let url = request.url().to_string();
        if url.starts_with("/metrics") {
            if let Some(metrics) = store.get_metrics() {
                let mut body = metrics.export_prometheus();
                body.push_str("\n");
                body.push_str(&supervm.export_routing_prometheus());
                let header = Header::from_bytes(&b"Content-Type"[..], &b"text/plain; version=0.0.4"[..]).unwrap();
                let response = Response::from_string(body).with_header(header);
                let _ = request.respond(response);
            } else {
                let _ = request
                    .respond(Response::from_string("metrics not enabled").with_status_code(500));
            }
        } else if url.starts_with("/summary") {
            if let Some(metrics) = store.get_metrics() {
                let tps_overall = metrics.tps();
                let tps_window = metrics.tps_window();
                let tps_peak = metrics.peak_tps();
                let p50 = metrics.latency_p50();
                let p90 = metrics.latency_p90();
                let p99 = metrics.latency_p99();
                let success = metrics.success_rate();

                let rstats = supervm.routing_stats();
                let body = format!(
                    "=== MVCC + Routing Metrics Summary ===\n\
TPS(overall): {:.0}\n\
TPS(window):  {:.0}\n\
TPS(peak):    {:.0}\n\
Success Rate: {:.2}%\n\
Latency(ms):  P50={:.2} P90={:.2} P99={:.2}\n\
Routing:      fast={} consensus={} privacy={} | ratios: f={:.2} c={:.2} p={:.2}\n\
Endpoints: /metrics (Prometheus), /summary (this), /routing/reset (reset routing counters)\n",
                    tps_overall,
                    tps_window,
                    tps_peak,
                    success,
                    p50,
                    p90,
                    p99,
                    rstats.fast_path_count,
                    rstats.consensus_path_count,
                    rstats.privacy_path_count,
                    rstats.fast_path_ratio(),
                    rstats.consensus_path_ratio(),
                    rstats.privacy_path_ratio()
                );
                let header = Header::from_bytes(&b"Content-Type"[..], &b"text/plain; charset=utf-8"[..]).unwrap();
                let response = Response::from_string(body).with_header(header);
                let _ = request.respond(response);
            } else {
                let _ = request
                    .respond(Response::from_string("metrics not enabled").with_status_code(500));
            }
        } else if url.starts_with("/routing/reset") {
            supervm.reset_routing_stats();
            let _ = request
                .respond(Response::from_string("routing stats reset OK").with_status_code(200));
        } else {
            let body = "OK. Endpoints: /metrics (Prometheus), /summary (text)";
            let _ = request
                .respond(Response::from_string(body).with_status_code(200));
        }
    }
}
