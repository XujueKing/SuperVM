// SPDX-License-Identifier: GPL-3.0-or-later
// 快速验证 metrics 导出功能(无需 HTTP 服务器)

use std::sync::Arc;
use vm_runtime::{
    MvccScheduler, ObjectId, ObjectMetadata, OwnershipManager, OwnershipType, Privacy, SuperVM,
    VmTransaction,
};

fn main() {
    println!("=== SuperVM Routing Metrics 快速验证 ===\n");

    // 初始化最小环境
    let ownership = Arc::new(OwnershipManager::new());
    let scheduler = Arc::new(MvccScheduler::new());
    let supervm = Arc::new(SuperVM::new(&ownership).with_scheduler(&scheduler));

    let addr: [u8; 32] = [1u8; 32];

    // 注册 owned 对象
    let owned_id: ObjectId = [0u8; 32];
    let owned_meta = ObjectMetadata {
        id: owned_id,
        version: 0,
        ownership: OwnershipType::Owned(addr),
        object_type: "TestObj".to_string(),
        created_at: 0,
        updated_at: 0,
        size: 64,
        is_deleted: false,
    };
    ownership.register_object(owned_meta).expect("register owned");

    // 注册 shared 对象
    let shared_id: ObjectId = [1u8; 32];
    let shared_meta = ObjectMetadata {
        id: shared_id,
        version: 0,
        ownership: OwnershipType::Shared,
        object_type: "TestObj".to_string(),
        created_at: 0,
        updated_at: 0,
        size: 64,
        is_deleted: false,
    };
    ownership.register_object(shared_meta).expect("register shared");

    // 模拟路由
    println!("🚀 模拟路由执行...");
    for _ in 0..5 {
        let tx = VmTransaction { from: addr, objects: vec![owned_id], privacy: Privacy::Public };
        let _ = supervm.route(&tx);
    }
    for _ in 0..3 {
        let tx = VmTransaction { from: addr, objects: vec![shared_id], privacy: Privacy::Public };
        let _ = supervm.route(&tx);
    }
    for _ in 0..2 {
        let tx = VmTransaction { from: addr, objects: vec![owned_id], privacy: Privacy::Private };
        let _ = supervm.route(&tx);
    }
    println!("✅ 路由完成: Fast=5, Consensus=3, Privacy=2\n");

    // 导出 Prometheus 格式（SuperVM 路由 + MetricsCollector）
    println!("=== Prometheus Metrics (SuperVM 路由) ===\n");
    let routing_prom = supervm.export_routing_prometheus();
    println!("{}", routing_prom);

    // 人工模拟一次 Fast→Consensus 回退计数，便于观察指标（仅演示，不影响核心逻辑）
    if let Some(mc) = scheduler.store().get_metrics() {
        mc.inc_fast_fallback();
        mc.inc_fast_fallback();
    }

    println!("=== Prometheus Metrics (MetricsCollector) ===\n");
    let collector_prom = scheduler
        .store()
        .get_metrics()
        .map(|m| m.export_prometheus())
        .unwrap_or_else(|| "# no metrics collector available\n".to_string());
    println!("{}", collector_prom);

    // 检查关键指标
    println!("\n=== 验证关键指标 ===");
    let checks = vec![
        ("vm_routing_fast_total", "Fast 路由计数"),
        ("vm_routing_consensus_total", "Consensus 路由计数"),
        ("vm_routing_privacy_total", "Privacy 路由计数"),
        ("vm_routing_fast_ratio", "Fast 路由比例"),
        ("vm_fast_fallback_total", "Fast 回退计数"),
        ("vm_fast_fallback_ratio", "Fast 回退比例"),
    ];

    // 合并文本后检查关键指标是否出现
    let combined = format!("{}\n{}", routing_prom, collector_prom);
    let mut all_present = true;
    for (metric, desc) in checks {
        if combined.contains(metric) {
            println!("✅ {} 存在", desc);
        } else {
            println!("❌ {} 缺失", desc);
            all_present = false;
        }
    }

    if all_present {
        println!("\n🎉 所有关键 metrics 均已导出!");
    } else {
        println!("\n⚠️  部分 metrics 缺失,请检查实现");
    }
}
