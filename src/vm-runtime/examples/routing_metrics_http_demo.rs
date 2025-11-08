// SPDX-License-Identifier: GPL-3.0-or-later
// Routing metrics HTTP demo: expose SuperVM routing stats via /metrics

use std::sync::Arc;
use tiny_http::{Header, Response, Server};
use vm_runtime::{
    MvccScheduler, ObjectId, ObjectMetadata, OwnershipManager, OwnershipType, Privacy, SuperVM,
    VmTransaction, adaptive_router::{AdaptiveRouter, AdaptiveRouterConfig},
};

fn main() {
    println!("=== SuperVM Routing Metrics HTTP Demo ===\n");

    // 初始化 Ownership 与 Scheduler（最小可运行环境）
    let ownership = Arc::new(OwnershipManager::new());
    let scheduler = Arc::new(MvccScheduler::new());
    // 启用自适应路由（演示）：支持通过环境变量覆盖配置
    // 若希望调整初始 fast 比例：$env:SUPERVM_ADAPTIVE_INIT="0.55" 等
    let adaptive_cfg = AdaptiveRouterConfig::from_env();
    let adaptive = AdaptiveRouter::new_with_config(adaptive_cfg);
    let supervm = Arc::new(SuperVM::new(&ownership).with_scheduler(&scheduler).with_adaptive_router(adaptive));

    // 为了演示，预热一些路由计数
    // - 模拟 fast/consensus/privacy 三种路由各 10 次
    {
    let addr: [u8; 32] = [1u8; 32];
        // Owned 对象
        let owned_id: ObjectId = [0u8; 32];
        let owned_meta = ObjectMetadata {
            id: owned_id,
            version: 0,
            ownership: OwnershipType::Owned(addr),
            object_type: "BenchObj".to_string(),
            created_at: 0,
            updated_at: 0,
            size: 128,
            is_deleted: false,
        };
        ownership
            .register_object(owned_meta)
            .expect("register owned failed");

        // Shared 对象
        let shared_id: ObjectId = [1u8; 32];
        let shared_meta = ObjectMetadata {
            id: shared_id,
            version: 0,
            ownership: OwnershipType::Shared,
            object_type: "BenchObj".to_string(),
            created_at: 0,
            updated_at: 0,
            size: 128,
            is_deleted: false,
        };
        ownership
            .register_object(shared_meta)
            .expect("register shared failed");

        // fast: 10 次
        for _ in 0..10u32 {
            let tx = VmTransaction { from: addr, objects: vec![owned_id], privacy: Privacy::Public };
            let _ = supervm.route(&tx);
        }
        // consensus: 10 次
        for _ in 0..10u32 {
            let tx = VmTransaction { from: addr, objects: vec![shared_id], privacy: Privacy::Public };
            let _ = supervm.route(&tx);
        }
        // privacy: 10 次
        for _ in 0..10u32 {
            let tx = VmTransaction { from: addr, objects: vec![owned_id], privacy: Privacy::Private };
            let _ = supervm.route(&tx);
        }
    }

    // 前台启动 HTTP 服务 (避免跨线程生命周期问题)
    let server = Server::http("0.0.0.0:8081").unwrap();
    println!("[HTTP] 监听 http://127.0.0.1:8081/metrics (Ctrl+C 退出) ...");
    for request in server.incoming_requests() {
        let url = request.url().to_string();
        if url.starts_with("/metrics") {
            let body = supervm.export_routing_prometheus();
            let header = Header::from_bytes(&b"Content-Type"[..], &b"text/plain; version=0.0.4"[..]).unwrap();
            let response = Response::from_string(body).with_header(header);
            let _ = request.respond(response);
        } else if url.starts_with("/summary") {
            let stats = supervm.routing_stats();
            let body = format!(
                "=== Routing Stats Summary ===\nFast: {}\nConsensus: {}\nPrivacy: {}\nRatios: fast={:.2} consensus={:.2} privacy={:.2}\nEndpoints: /metrics (Prometheus), /summary (this)\n",
                stats.fast_path_count,
                stats.consensus_path_count,
                stats.privacy_path_count,
                stats.fast_path_ratio(),
                stats.consensus_path_ratio(),
                stats.privacy_path_ratio(),
            );
            let header = Header::from_bytes(&b"Content-Type"[..], &b"text/plain; charset=utf-8"[..]).unwrap();
            let response = Response::from_string(body).with_header(header);
            let _ = request.respond(response);
        } else {
            let body = "OK. Endpoints: /metrics (Prometheus), /summary (text)";
            let _ = request.respond(Response::from_string(body).with_status_code(200));
        }
    }
}
