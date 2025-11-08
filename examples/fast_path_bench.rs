// SPDX-License-Identifier: GPL-3.0-or-later
// Phase 5 Fast Path Benchmark
// 目标: 验证 FastPathExecutor 在独占对象场景下的吞吐与延迟

use std::time::{Instant, Duration};
use vm_runtime::{OwnershipManager};
use vm_runtime::supervm::{SuperVM, Transaction, Privacy};
use vm_runtime::parallel::FastPathStats;
use rand::{Rng, SeedableRng};
use rand::rngs::StdRng;

fn make_address(id: u64) -> [u8;32] {
    let mut arr = [0u8;32];
    arr[0..8].copy_from_slice(&id.to_le_bytes());
    arr
}

fn main() {
    println!("=== Fast Path Benchmark (Phase 5) ===\n");

    // 配置参数
    let iterations: usize = std::env::var("FAST_PATH_ITERS")
        .ok().and_then(|v| v.parse::<usize>().ok()).unwrap_or(200_000);
    let owned_objects: usize = std::env::var("FAST_PATH_OBJECTS")
        .ok().and_then(|v| v.parse::<usize>().ok()).unwrap_or(10_000);

    let mut ownership = OwnershipManager::new();
    let sender = make_address(42);

    // 注册 owned 对象
    for i in 0..owned_objects {
        let obj_id = {
            let mut id = [0u8;32];
            id[0..8].copy_from_slice(&i.to_le_bytes());
            id
        };
        let metadata = vm_runtime::ObjectMetadata {
            id: obj_id,
            version: 0,
            ownership: vm_runtime::OwnershipType::Owned(sender),
            object_type: "BenchmarkObject".to_string(),
            created_at: 0,
            updated_at: 0,
            size: 0,
            is_deleted: false,
        };
        ownership.register_object(metadata).unwrap();
    }

    let mut vm = SuperVM::new(&ownership);

    let mut rng = StdRng::seed_from_u64(2025);

    let start = Instant::now();
    let mut success = 0u64;

    for tx_id in 0..iterations as u64 {
        // 随机选择 1 个 owned 对象
        let obj_index = rng.gen_range(0..owned_objects as u64);
        let obj_id = {
            let mut id = [0u8;32];
            id[0..8].copy_from_slice(&obj_index.to_le_bytes());
            id
        };
        let tx = Transaction { from: sender, objects: vec![obj_id], privacy: Privacy::Public };
        let r = vm.execute_fast_path(tx_id, &tx, || {
            // 模拟极轻量业务逻辑（CPU 计算 + 内存写）
            let mut acc = 0u64;
            for _ in 0..4 { acc = acc.wrapping_add(1); }
            Ok((acc % 7) as i32)
        });
        if r.is_ok() { success += 1; }
    }

    let elapsed = start.elapsed();
    let stats: FastPathStats = vm.fast_path_stats();

    println!("📊 Benchmark Summary:");
    println!("  Iterations: {}", iterations);
    println!("  Successes: {}", success);
    println!("  Elapsed: {:.2?}", elapsed);
    println!("  Avg latency (ns): {}", stats.avg_latency_ns);
    println!("  Estimated TPS: {:.0}", stats.estimated_tps());
    println!("  Success Rate: {:.2}%", stats.success_rate() * 100.0);
}
