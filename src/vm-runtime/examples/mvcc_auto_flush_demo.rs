// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2025 XujueKing <leadbrand@me.com>

//! MVCC 自动刷新示例
//!
//! 演示如何使用自动刷新功能将 MVCC Store 的数据定期刷新到 RocksDB
//!
//! 运行方式:
//! ```bash
//! cargo run --example mvcc_auto_flush_demo --features rocksdb-storage
//! ```

use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;
use vm_runtime::{AutoFlushConfig, MvccStore, RocksDBStorage, Storage};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== MVCC 自动刷新示例 ===\n");

    // 1. 创建 RocksDB 存储
    let storage_path = "./data/demo_auto_flush";
    println!("1. 创建 RocksDB 存储: {}", storage_path);

    // 清理旧数据
    if std::path::Path::new(storage_path).exists() {
        std::fs::remove_dir_all(storage_path)?;
    }

    let storage = RocksDBStorage::new_with_path(storage_path)?;
    let storage_arc: Arc<Mutex<dyn Storage + Send>> = Arc::new(Mutex::new(storage));

    // 2. 创建 MVCC Store
    println!("2. 创建 MVCC Store");
    let store = MvccStore::new();

    // 3. 配置自动刷新
    println!("3. 配置自动刷新:");
    let flush_config = AutoFlushConfig {
        interval_secs: 2,        // 每 2 秒刷新一次
        blocks_per_flush: 5,     // 每 5 个区块刷新一次
        keep_recent_versions: 2, // 保留最近 2 个版本在内存
        flush_on_start: true,    // 启动时立即刷新一次
    };
    println!("   - 时间间隔: {} 秒", flush_config.interval_secs);
    println!("   - 区块间隔: {} 个区块", flush_config.blocks_per_flush);
    println!("   - 内存保留版本数: {}", flush_config.keep_recent_versions);

    // 4. 启动自动刷新
    println!("4. 启动自动刷新后台线程");
    store.start_auto_flush(flush_config, Arc::clone(&storage_arc))?;

    println!("\n开始模拟交易写入...\n");

    // 5. 模拟交易写入
    for block in 1..=15 {
        // 更新当前区块号
        store.set_current_block(block);

        // 在每个区块中执行多个交易
        for tx_id in 1..=3 {
            let key = format!("account_{}", block * 100 + tx_id);
            let value = format!("balance_{}_v{}", block * 1000, tx_id);

            // 开启事务
            let mut tx = store.begin();
            tx.write(key.as_bytes().to_vec(), value.as_bytes().to_vec());
            tx.commit()?;

            println!(
                "  区块 {:2} | 交易 {} | 写入: {} = {}",
                block, tx_id, key, value
            );
        }

        // 获取刷新统计
        let stats = store.get_flush_stats();
        println!(
            "  区块 {:2} | 刷新统计: count={}, keys={}, bytes={}, last_block={}",
            block,
            stats.flush_count,
            stats.keys_flushed,
            stats.bytes_flushed,
            stats.last_flush_block
        );

        // 短暂休眠模拟区块间隔
        thread::sleep(Duration::from_millis(500));
    }

    println!("\n等待最后一次刷新...");
    thread::sleep(Duration::from_secs(3));

    // 6. 显示最终统计
    let final_stats = store.get_flush_stats();
    println!("\n=== 最终刷新统计 ===");
    println!("  总刷新次数: {}", final_stats.flush_count);
    println!("  总刷新键数: {}", final_stats.keys_flushed);
    println!("  总刷新字节数: {}", final_stats.bytes_flushed);
    println!("  最后刷新区块: {}", final_stats.last_flush_block);
    println!("  最后刷新时间戳: {}", final_stats.last_flush_ts);

    // 7. 停止自动刷新
    println!("\n7. 停止自动刷新");
    store.stop_auto_flush();

    // 等待后台线程退出
    thread::sleep(Duration::from_millis(500));

    if !store.is_auto_flush_running() {
        println!("   ✓ 自动刷新已成功停止");
    }

    // 8. 验证数据持久化
    println!("\n8. 验证数据持久化");
    println!("   重新打开 RocksDB...");

    drop(storage_arc); // 释放旧的存储实例

    let verify_storage = RocksDBStorage::new_with_path(storage_path)?;

    // 检查几个键
    let test_keys = vec!["account_101", "account_505", "account_1503"];
    for key in test_keys {
        if let Some(value) = verify_storage.get(key.as_bytes())? {
            println!("   ✓ {} = {}", key, String::from_utf8_lossy(&value));
        } else {
            println!("   ✗ {} 不存在", key);
        }
    }

    println!("\n=== 演示完成 ===");

    Ok(())
}
