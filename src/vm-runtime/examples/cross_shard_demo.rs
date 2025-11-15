// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2025 XujueKing <leadbrand@me.com>

//! Cross-Shard Transaction Demo
//!
//! 演示跨分片事务的基本用法

use std::collections::HashMap;
use vm_runtime::{
    shard_for_object, CoordinatorError, ShardConfig, ShardCoordinator, ShardId,
};

fn main() -> Result<(), CoordinatorError> {
    println!("=== Cross-Shard Transaction Demo (Phase 6) ===\n");

    // 1. 配置 4 个分片
    let config = ShardConfig {
        num_shards: 4,
        shard_endpoints: create_shard_endpoints(4),
        timeout_ms: 5000,
        local_shard_id: 0,
    };

    println!("📦 Shard Configuration:");
    println!("   Num Shards: {}", config.num_shards);
    println!("   Timeout: {}ms", config.timeout_ms);
    println!("   Local Shard: {}\n", config.local_shard_id);

    // 2. 创建协调器
    let coordinator = ShardCoordinator::new(config.clone());

    // 3. 准备测试对象
    let obj1 = create_object_id(1);
    let obj2 = create_object_id(100);
    let obj3 = create_object_id(200);

    // 查看对象分片映射
    println!("🗂️  Object Shard Mapping:");
    println!("   obj1 -> Shard {}", shard_for_object(&obj1, config.num_shards));
    println!("   obj2 -> Shard {}", shard_for_object(&obj2, config.num_shards));
    println!("   obj3 -> Shard {}\n", shard_for_object(&obj3, config.num_shards));

    // 4. 测试场景1：单分片事务（快速路径）
    println!("🚀 Scenario 1: Single-Shard Transaction (Fast Path)");
    let read_set = vec![(obj1, 1)];
    let write_set = vec![(obj1, vec![0x42])];

    match coordinator.execute_cross_shard_txn(read_set, write_set) {
        Ok(true) => println!("   ✅ Transaction COMMITTED\n"),
        Ok(false) => println!("   ❌ Transaction ABORTED (conflict)\n"),
        Err(e) => println!("   ⚠️  Error: {}\n", e),
    }

    // 5. 测试场景2：跨分片事务（2PC 协议）
    println!("🔀 Scenario 2: Cross-Shard Transaction (2PC)");
    let read_set = vec![(obj1, 1), (obj2, 1)];
    let write_set = vec![(obj2, vec![0x43]), (obj3, vec![0x44])];

    let shard1 = shard_for_object(&obj1, config.num_shards);
    let shard2 = shard_for_object(&obj2, config.num_shards);
    let shard3 = shard_for_object(&obj3, config.num_shards);

    println!(
        "   Participants: Shard {} (read), Shard {} (write), Shard {} (write)",
        shard1, shard2, shard3
    );

    match coordinator.execute_cross_shard_txn(read_set, write_set) {
        Ok(true) => println!("   ✅ Transaction COMMITTED\n"),
        Ok(false) => println!("   ❌ Transaction ABORTED (conflict detected in prepare phase)\n"),
        Err(e) => println!("   ⚠️  Error: {}\n", e),
    }

    // 6. 统计信息
    println!("📊 Statistics:");
    println!("   Active Transactions: {}", coordinator.active_txn_count());

    println!("\n✨ Demo completed!");
    Ok(())
}

/// 创建分片端点配置（模拟）
fn create_shard_endpoints(num_shards: usize) -> HashMap<ShardId, String> {
    (0..num_shards as ShardId)
        .map(|id| (id, format!("127.0.0.1:{}", 5000 + id)))
        .collect()
}

/// 创建测试对象 ID
fn create_object_id(seed: u8) -> [u8; 32] {
    let mut id = [0u8; 32];
    id[0] = seed;
    id
}
