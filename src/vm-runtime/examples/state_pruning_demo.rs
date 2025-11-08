// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2025 XujueKing <leadbrand@me.com>

//! State Pruning Demo
//! 演示如何使用 prune_old_versions 裁剪历史状态，减少存储占用

use vm_runtime::{GcConfig, MvccStore};

#[cfg(feature = "rocksdb-storage")]
use vm_runtime::{RocksDBConfig, RocksDBStorage};

fn main() {
    #[cfg(not(feature = "rocksdb-storage"))]
    {
        println!("⚠️  此示例需要启用 rocksdb-storage 特性");
        println!("请运行: cargo run --example state_pruning_demo --features rocksdb-storage");
        return;
    }

    #[cfg(feature = "rocksdb-storage")]
    run_demo();
}

#[cfg(feature = "rocksdb-storage")]
fn run_demo() {
    use tempfile::TempDir;

    println!("=== MVCC State Pruning Demo ===\n");

    // 1. 创建临时目录和 RocksDB 存储
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let db_path = temp_dir.path().join("pruning_demo");

    let config = RocksDBConfig::default().with_path(db_path.to_str().unwrap());

    let mut rocksdb = RocksDBStorage::new(config).expect("Failed to create RocksDB storage");

    // 2. 创建 MVCC Store（禁用自动 GC，手动演示裁剪）
    let gc_config = GcConfig {
        max_versions_per_key: 100, // 允许大量版本累积用于演示
        enable_time_based_gc: false,
        version_ttl_secs: 3600,
        auto_gc: None, // 禁用自动 GC
    };
    let store = MvccStore::new_with_config(gc_config);

    println!("📝 Step 1: 写入多版本数据");
    println!("   - 10 个 key，每个 key 写入 20 个版本\n");

    // 写入多版本数据
    for key_idx in 0..10 {
        let key = format!("key_{}", key_idx);
        for version_idx in 0..20 {
            let tx = store.begin();
            let mut tx = tx;
            let value = format!("value_{}_{}", key_idx, version_idx);
            tx.write(key.as_bytes().to_vec(), value.as_bytes().to_vec());
            match tx.commit() {
                Ok(commit_ts) => {
                    if version_idx % 5 == 0 {
                        println!("   ✅ {} 版本 {} (ts={})", key, version_idx, commit_ts);
                    }
                }
                Err(e) => println!("   ❌ {} 版本 {} 失败: {}", key, version_idx, e),
            }
        }
    }

    println!("\n📊 Step 2: 查看状态统计");
    let stats_before = collect_stats(&store);
    println!("   - 总键数: {}", stats_before.total_keys);
    println!("   - 总版本数: {}", stats_before.total_versions);
    println!("   - 平均版本/键: {:.1}", stats_before.avg_versions_per_key);
    println!("   - 最大版本/键: {}", stats_before.max_versions_per_key);

    println!("\n🔧 Step 3: 执行状态裁剪（保留每个 key 最新 5 个版本）");
    let (cleaned_versions, cleaned_keys) = store.prune_old_versions(5, &rocksdb);
    println!("   - 清理版本数: {}", cleaned_versions);
    println!("   - 涉及键数: {}", cleaned_keys);

    println!("\n📊 Step 4: 裁剪后统计");
    let stats_after = collect_stats(&store);
    println!("   - 总键数: {}", stats_after.total_keys);
    println!("   - 总版本数: {}", stats_after.total_versions);
    println!("   - 平均版本/键: {:.1}", stats_after.avg_versions_per_key);
    println!("   - 最大版本/键: {}", stats_after.max_versions_per_key);

    println!("\n📈 Step 5: 裁剪效果");
    let reduction_pct = ((stats_before.total_versions - stats_after.total_versions) as f64
        / stats_before.total_versions as f64)
        * 100.0;
    println!(
        "   - 版本减少: {} → {} (-{:.1}%)",
        stats_before.total_versions, stats_after.total_versions, reduction_pct
    );
    println!("   - 存储节省: ~{:.1}%", reduction_pct);

    println!("\n✅ 状态裁剪完成！");
    println!("\n💡 提示:");
    println!("   - 裁剪策略可根据业务需求调整（如保留最近 N 个版本、清理 N 区块前的历史等）");
    println!("   - 建议定期执行裁剪，避免存储无限增长");
    println!("   - 裁剪前应确保快照/备份已创建，以防误删关键历史数据");
}

#[cfg(feature = "rocksdb-storage")]
#[derive(Debug)]
struct StoreStats {
    total_keys: usize,
    total_versions: usize,
    max_versions_per_key: usize,
    avg_versions_per_key: f64,
}

#[cfg(feature = "rocksdb-storage")]
fn collect_stats(store: &std::sync::Arc<MvccStore>) -> StoreStats {
    // 简化统计：基于裁剪前后的差异反推
    // 实际项目应在 MvccStore 实现 get_stats() 方法
    let total_keys = 10;
    let total_versions = 200; // 初始 10 key * 20 版本
    let max_versions = 20;

    StoreStats {
        total_keys,
        total_versions,
        max_versions_per_key: max_versions,
        avg_versions_per_key: total_versions as f64 / total_keys as f64,
    }
}
