// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2025 XujueKing <leadbrand@me.com>

//! RocksDB Metrics Integration Demo
//! 演示如何使用 collect_metrics() 采集 RocksDB 内部指标并更新到 MetricsCollector

use std::sync::Arc;
use std::thread;
use std::time::Duration;
use vm_runtime::{GcConfig, MvccStore, RocksDBConfig, RocksDBStorage};

#[cfg(not(feature = "rocksdb-storage"))]
fn main() {
    println!("⚠️  此示例需要启用 rocksdb-storage 特性");
    println!("请运行: cargo run --example rocksdb_metrics_demo --features rocksdb-storage");
}

#[cfg(feature = "rocksdb-storage")]
fn main() {
    println!("=== RocksDB Metrics Integration Demo ===\n");

    // 1. 初始化 RocksDB
    let db_path = "data/metrics_demo";
    std::fs::create_dir_all(db_path).unwrap();
    let rocksdb = RocksDBStorage::new(RocksDBConfig::default().with_path(db_path)).unwrap();
    println!("✅ RocksDB 初始化成功: {}\n", db_path);

    // 2. 初始化 MVCC Store (启用指标收集)
    let gc_config = GcConfig {
        max_versions_per_key: 10,
        enable_time_based_gc: false,
        version_ttl_secs: 3600,
        auto_gc: None,
    };
    let mvcc = Arc::new(MvccStore::new_with_config(gc_config));
    println!("✅ MVCC Store 初始化成功 (指标收集已启用)\n");

    // 3. 执行一些事务操作以产生数据
    println!("📝 Step 1: 执行事务写入数据...");
    for i in 0..100 {
        let tx = mvcc.begin();
        let mut tx = tx;
        let key = format!("key_{}", i);
        let value = format!("value_{}", i);
        tx.write(key.into_bytes(), value.into_bytes());
        match tx.commit() {
            Ok(commit_ts) => {
                if i % 20 == 0 {
                    println!("   ✅ 事务 {} 提交成功 (ts={})", i, commit_ts);
                }
            }
            Err(e) => println!("   ❌ 事务 {} 失败: {}", i, e),
        }
    }
    println!("   - 完成 100 个事务写入\n");

    // 4. 采集 RocksDB 内部指标
    println!("📊 Step 2: 采集 RocksDB 内部指标...");
    let rocksdb_metrics = rocksdb.collect_metrics();
    println!("   - 估计键数量: {}", rocksdb_metrics.estimate_num_keys);
    println!(
        "   - SST 文件总大小: {:.2} MB",
        rocksdb_metrics.total_sst_size_bytes as f64 / 1024.0 / 1024.0
    );

    let cache_total = rocksdb_metrics.cache_hit + rocksdb_metrics.cache_miss;
    let cache_hit_rate = if cache_total > 0 {
        (rocksdb_metrics.cache_hit as f64 / cache_total as f64) * 100.0
    } else {
        0.0
    };
    println!("   - Block Cache 命中数: {}", rocksdb_metrics.cache_hit);
    println!("   - Block Cache 未命中数: {}", rocksdb_metrics.cache_miss);
    println!("   - Block Cache 命中率: {:.2}%", cache_hit_rate);
    println!(
        "   - Compaction CPU 时间: {:.2} ms",
        rocksdb_metrics.compaction_cpu_micros as f64 / 1000.0
    );
    println!(
        "   - Compaction 写入字节数: {:.2} KB",
        rocksdb_metrics.compaction_write_bytes as f64 / 1024.0
    );
    println!(
        "   - Write Stall 时间: {:.2} ms",
        rocksdb_metrics.write_stall_micros as f64 / 1000.0
    );
    println!("   - Level 0 文件数: {}", rocksdb_metrics.num_files_level0);
    println!(
        "   - Immutable MemTable 数: {}\n",
        rocksdb_metrics.num_immutable_mem_table
    );

    // 5. 更新 RocksDB 指标到 MetricsCollector
    println!("🔄 Step 3: 更新指标到 MetricsCollector...");
    mvcc.update_rocksdb_metrics(&rocksdb_metrics);
    println!("   ✅ RocksDB 指标已同步到 MetricsCollector\n");

    // 6. 导出 Prometheus 格式指标
    println!("📤 Step 4: 导出 Prometheus 格式指标...");
    if let Some(metrics) = mvcc.get_metrics() {
        let prom_output = metrics.export_prometheus();

        // 只打印 RocksDB 相关指标行
        let rocksdb_lines: Vec<&str> = prom_output
            .lines()
            .filter(|line| line.contains("rocksdb") && !line.starts_with('#'))
            .collect();

        println!("   📊 RocksDB Prometheus 指标:");
        for line in &rocksdb_lines {
            println!("      {}", line);
        }
        println!();

        // 保存完整指标到文件
        let metrics_file = "data/metrics_demo/prometheus_metrics.txt";
        std::fs::write(metrics_file, &prom_output).unwrap();
        println!("   💾 完整指标已保存到: {}\n", metrics_file);
    } else {
        println!("   ⚠️  MetricsCollector 未启用\n");
    }

    // 7. 模拟周期性指标采集
    println!("⏱️  Step 5: 模拟周期性指标采集 (每2秒采集一次, 持续10秒)...");
    for iteration in 1..=5 {
        thread::sleep(Duration::from_secs(2));

        // 执行更多事务
        let tx = mvcc.begin();
        let mut tx = tx;
        let key = format!("periodic_key_{}", iteration);
        let value = format!("periodic_value_{}", iteration);
        tx.write(key.into_bytes(), value.into_bytes());
        let _ = tx.commit();

        // 采集并更新指标
        let metrics = rocksdb.collect_metrics();
        mvcc.update_rocksdb_metrics(&metrics);

        println!("   🔄 迭代 {}/5:", iteration);
        println!(
            "      - Keys: {} | SST Size: {:.2} KB | Cache Hit Rate: {:.2}%",
            metrics.estimate_num_keys,
            metrics.total_sst_size_bytes as f64 / 1024.0,
            if metrics.cache_hit + metrics.cache_miss > 0 {
                (metrics.cache_hit as f64 / (metrics.cache_hit + metrics.cache_miss) as f64) * 100.0
            } else {
                0.0
            }
        );
    }
    println!();

    // 8. 最终统计
    println!("📊 Final Statistics:");
    if let Some(metrics) = mvcc.get_metrics() {
        use std::sync::atomic::Ordering;
        println!("   MVCC:");
        println!(
            "      - Total Txn Committed: {}",
            metrics.txn_committed.load(Ordering::Relaxed)
        );
        println!(
            "      - Total Txn Aborted: {}",
            metrics.txn_aborted.load(Ordering::Relaxed)
        );
        println!("      - Success Rate: {:.2}%", metrics.success_rate());
        println!("      - TPS: {:.0}", metrics.tps());

        println!("\n   RocksDB Internal:");
        println!(
            "      - Estimate Num Keys: {}",
            metrics.rocksdb_estimate_num_keys.load(Ordering::Relaxed)
        );
        println!(
            "      - Total SST Size: {:.2} MB",
            metrics.rocksdb_total_sst_size_bytes.load(Ordering::Relaxed) as f64 / 1024.0 / 1024.0
        );
        let cache_hit = metrics.rocksdb_cache_hit.load(Ordering::Relaxed);
        let cache_miss = metrics.rocksdb_cache_miss.load(Ordering::Relaxed);
        let total = cache_hit + cache_miss;
        let hit_rate = if total > 0 {
            (cache_hit as f64 / total as f64) * 100.0
        } else {
            0.0
        };
        println!("      - Cache Hit Rate: {:.2}%", hit_rate);
        println!(
            "      - Compaction CPU: {:.2} ms",
            metrics
                .rocksdb_compaction_cpu_micros
                .load(Ordering::Relaxed) as f64
                / 1000.0
        );
        println!(
            "      - Write Stall: {:.2} ms",
            metrics.rocksdb_write_stall_micros.load(Ordering::Relaxed) as f64 / 1000.0
        );
        println!(
            "      - Level 0 Files: {}",
            metrics.rocksdb_num_files_level0.load(Ordering::Relaxed)
        );
    }

    println!("\n✅ Demo 完成!");
    println!("💡 提示:");
    println!("   1. 在生产环境中,应定期调用 collect_metrics() 和 update_rocksdb_metrics()");
    println!("   2. 可通过 HTTP /metrics 端点暴露 Prometheus 格式指标");
    println!("   3. 使用 Grafana 可视化 RocksDB 性能指标");
}
