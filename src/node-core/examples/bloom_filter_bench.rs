// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2025 XujueKing <leadbrand@me.com>

//! Bloom Filter 优化性能基准测试
//! 
//! 测试目标:
//! - 验证布隆过滤器能否提升高竞争场景性能
//! - 对比启用/禁用布隆过滤器的 TPS 差异
//! - 测量冲突检测的加速效果

use std::time::Instant;
use vm_runtime::{OptimizedMvccScheduler, OptimizedSchedulerConfig, mvcc::GcConfig};

fn main() {
    println!("=== Bloom Filter Optimization Benchmark ===\n");
    
    // 测试场景 1: 低竞争 (无共享键)
    println!("Scenario 1: Low Contention (No Shared Keys)");
    benchmark_low_contention();
    
    println!("\n");
    
    // 测试场景 2: 高竞争 (所有交易写同一个键)
    println!("Scenario 2: High Contention (All Transactions Write Same Key)");
    benchmark_high_contention();
    
    println!("\n");
    
    // 测试场景 3: 中等竞争 (部分共享键)
    println!("Scenario 3: Medium Contention (Partial Shared Keys)");
    benchmark_medium_contention();
}

fn benchmark_low_contention() {
    let num_txns = 10000;
    
    // 不启用布隆过滤器
    let mut config_no_bloom = OptimizedSchedulerConfig::default();
    config_no_bloom.enable_bloom_filter = false;
    config_no_bloom.mvcc_config = GcConfig {
        max_versions_per_key: 100,
        enable_time_based_gc: false,
        version_ttl_secs: 3600,
        auto_gc: None,
    };
    
    let scheduler_no_bloom = OptimizedMvccScheduler::new_with_config(config_no_bloom);
    
    let start = Instant::now();
    for i in 0..num_txns {
        let key = format!("key_{}", i);
        let _ = scheduler_no_bloom.execute_txn(i as u64, |txn| {
            txn.write(key.as_bytes().to_vec(), b"value".to_vec());
            Ok(1)
        });
    }
    let duration_no_bloom = start.elapsed();
    let tps_no_bloom = num_txns as f64 / duration_no_bloom.as_secs_f64();
    
    // 启用布隆过滤器
    let mut config_with_bloom = OptimizedSchedulerConfig::default();
    config_with_bloom.enable_bloom_filter = true;
    config_with_bloom.mvcc_config = GcConfig {
        max_versions_per_key: 100,
        enable_time_based_gc: false,
        version_ttl_secs: 3600,
        auto_gc: None,
    };
    
    let scheduler_with_bloom = OptimizedMvccScheduler::new_with_config(config_with_bloom);
    
    let start = Instant::now();
    for i in 0..num_txns {
        let key = format!("key_{}", i);
        let _ = scheduler_with_bloom.execute_txn(i as u64, |txn| {
            txn.write(key.as_bytes().to_vec(), b"value".to_vec());
            Ok(1)
        });
    }
    let duration_with_bloom = start.elapsed();
    let tps_with_bloom = num_txns as f64 / duration_with_bloom.as_secs_f64();
    
    println!("  Without Bloom Filter:");
    println!("    Duration: {:?}", duration_no_bloom);
    println!("    TPS: {:.0}", tps_no_bloom);
    
    println!("  With Bloom Filter:");
    println!("    Duration: {:?}", duration_with_bloom);
    println!("    TPS: {:.0}", tps_with_bloom);
    
    let improvement = ((tps_with_bloom / tps_no_bloom) - 1.0) * 100.0;
    println!("  Improvement: {:.2}%", improvement);
    
    let stats = scheduler_with_bloom.get_stats();
    println!("\n  Bloom Filter Stats:");
    println!("    Successful: {}", stats.basic.successful_txs);
    println!("    Conflicts: {}", stats.basic.conflict_count);
}

fn benchmark_high_contention() {
    let num_txns = 1000; // 减少数量因为高竞争下会有很多重试
    
    // 不启用布隆过滤器
    let mut config_no_bloom = OptimizedSchedulerConfig::default();
    config_no_bloom.enable_bloom_filter = false;
    config_no_bloom.max_retries = 10;
    config_no_bloom.mvcc_config = GcConfig {
        max_versions_per_key: 100,
        enable_time_based_gc: false,
        version_ttl_secs: 3600,
        auto_gc: None,
    };
    
    let scheduler_no_bloom = OptimizedMvccScheduler::new_with_config(config_no_bloom);
    
    let start = Instant::now();
    for i in 0..num_txns {
        let _ = scheduler_no_bloom.execute_txn(i as u64, |txn| {
            txn.write(b"shared_key".to_vec(), format!("value_{}", i).into_bytes());
            Ok(1)
        });
    }
    let duration_no_bloom = start.elapsed();
    let tps_no_bloom = num_txns as f64 / duration_no_bloom.as_secs_f64();
    
    let stats_no_bloom = scheduler_no_bloom.get_stats();
    
    // 启用布隆过滤器
    let mut config_with_bloom = OptimizedSchedulerConfig::default();
    config_with_bloom.enable_bloom_filter = true;
    config_with_bloom.max_retries = 10;
    config_with_bloom.mvcc_config = GcConfig {
        max_versions_per_key: 100,
        enable_time_based_gc: false,
        version_ttl_secs: 3600,
        auto_gc: None,
    };
    
    let scheduler_with_bloom = OptimizedMvccScheduler::new_with_config(config_with_bloom);
    
    let start = Instant::now();
    for i in 0..num_txns {
        let _ = scheduler_with_bloom.execute_txn(i as u64, |txn| {
            txn.write(b"shared_key".to_vec(), format!("value_{}", i).into_bytes());
            Ok(1)
        });
    }
    let duration_with_bloom = start.elapsed();
    let tps_with_bloom = num_txns as f64 / duration_with_bloom.as_secs_f64();
    
    let stats_with_bloom = scheduler_with_bloom.get_stats();
    
    println!("  Without Bloom Filter:");
    println!("    Duration: {:?}", duration_no_bloom);
    println!("    TPS: {:.0}", tps_no_bloom);
    println!("    Conflicts: {}", stats_no_bloom.basic.conflict_count);
    println!("    Retries: {}", stats_no_bloom.basic.retry_count);
    
    println!("  With Bloom Filter:");
    println!("    Duration: {:?}", duration_with_bloom);
    println!("    TPS: {:.0}", tps_with_bloom);
    println!("    Conflicts: {}", stats_with_bloom.basic.conflict_count);
    println!("    Retries: {}", stats_with_bloom.basic.retry_count);
    
    let improvement = ((tps_with_bloom / tps_no_bloom) - 1.0) * 100.0;
    println!("  Improvement: {:.2}%", improvement);
}

fn benchmark_medium_contention() {
    let num_txns = 5000;
    let num_shared_keys = 10; // 10 个共享键
    
    // 不启用布隆过滤器
    let mut config_no_bloom = OptimizedSchedulerConfig::default();
    config_no_bloom.enable_bloom_filter = false;
    config_no_bloom.mvcc_config = GcConfig {
        max_versions_per_key: 100,
        enable_time_based_gc: false,
        version_ttl_secs: 3600,
        auto_gc: None,
    };
    
    let scheduler_no_bloom = OptimizedMvccScheduler::new_with_config(config_no_bloom);
    
    let start = Instant::now();
    for i in 0..num_txns {
        let key = format!("key_{}", i % num_shared_keys);
        let _ = scheduler_no_bloom.execute_txn(i as u64, |txn| {
            txn.write(key.as_bytes().to_vec(), format!("value_{}", i).into_bytes());
            Ok(1)
        });
    }
    let duration_no_bloom = start.elapsed();
    let tps_no_bloom = num_txns as f64 / duration_no_bloom.as_secs_f64();
    
    let stats_no_bloom = scheduler_no_bloom.get_stats();
    
    // 启用布隆过滤器
    let mut config_with_bloom = OptimizedSchedulerConfig::default();
    config_with_bloom.enable_bloom_filter = true;
    config_with_bloom.mvcc_config = GcConfig {
        max_versions_per_key: 100,
        enable_time_based_gc: false,
        version_ttl_secs: 3600,
        auto_gc: None,
    };
    
    let scheduler_with_bloom = OptimizedMvccScheduler::new_with_config(config_with_bloom);
    
    let start = Instant::now();
    for i in 0..num_txns {
        let key = format!("key_{}", i % num_shared_keys);
        let _ = scheduler_with_bloom.execute_txn(i as u64, |txn| {
            txn.write(key.as_bytes().to_vec(), format!("value_{}", i).into_bytes());
            Ok(1)
        });
    }
    let duration_with_bloom = start.elapsed();
    let tps_with_bloom = num_txns as f64 / duration_with_bloom.as_secs_f64();
    
    let stats_with_bloom = scheduler_with_bloom.get_stats();
    
    println!("  Without Bloom Filter:");
    println!("    Duration: {:?}", duration_no_bloom);
    println!("    TPS: {:.0}", tps_no_bloom);
    println!("    Conflicts: {}", stats_no_bloom.basic.conflict_count);
    
    println!("  With Bloom Filter:");
    println!("    Duration: {:?}", duration_with_bloom);
    println!("    TPS: {:.0}", tps_with_bloom);
    println!("    Conflicts: {}", stats_with_bloom.basic.conflict_count);
    
    let improvement = ((tps_with_bloom / tps_no_bloom) - 1.0) * 100.0;
    println!("  Improvement: {:.2}%", improvement);
}
