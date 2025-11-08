// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2025 XujueKing <leadbrand@me.com>

//! 增强版 Bloom Filter 性能基准测试
//!
//! 改进：
//! - Warmup 预热
//! - 重复运行 N 次
//! - 统计分析（均值、标准差、最小/最大）
//! - 更大规模测试
//! - 更多冲突率场景

use std::time::{Duration, Instant};
use vm_runtime::{mvcc::GcConfig, OptimizedMvccScheduler, OptimizedSchedulerConfig};

const WARMUP_RUNS: usize = 3;
const BENCH_RUNS: usize = 10;

fn main() {
    println!("=== Enhanced Bloom Filter Optimization Benchmark ===");
    println!(
        "Warmup: {} runs, Benchmark: {} runs\n",
        WARMUP_RUNS, BENCH_RUNS
    );

    // 场景 1: 低竞争 (无共享键) - 10K 交易
    println!("Scenario 1: Low Contention (10K txns, unique keys)");
    benchmark_scenario("Low", 10000, Contention::Low);

    println!("\n");

    // 场景 2: 高竞争 (单热键) - 1K 交易
    println!("Scenario 2: High Contention (1K txns, single hot key)");
    benchmark_scenario("High", 1000, Contention::High);

    println!("\n");

    // 场景 3: 中等竞争 (10 个共享键) - 5K 交易
    println!("Scenario 3: Medium Contention (5K txns, 10 shared keys)");
    benchmark_scenario("Medium", 5000, Contention::Medium(10));

    println!("\n");

    // 场景 4: 更大规模低竞争 - 50K 交易
    println!("Scenario 4: Large Scale Low Contention (50K txns)");
    benchmark_scenario("LargeLow", 50000, Contention::Low);

    println!("\n");

    // 场景 5: 更细颗粒度中等竞争 (100 个共享键) - 10K 交易
    println!("Scenario 5: Fine-Grained Medium Contention (10K txns, 100 keys)");
    benchmark_scenario("FineMedium", 10000, Contention::Medium(100));
}

#[derive(Clone)]
enum Contention {
    Low,           // 每个交易唯一键
    High,          // 所有交易写同一个键
    Medium(usize), // n 个共享键
}

fn benchmark_scenario(name: &str, num_txns: usize, contention: Contention) {
    // Warmup
    for _ in 0..WARMUP_RUNS {
        run_once(num_txns, &contention, false);
        run_once(num_txns, &contention, true);
    }

    // 无 Bloom
    let mut durations_no_bloom = Vec::new();
    let mut stats_no_bloom_list = Vec::new();

    for _ in 0..BENCH_RUNS {
        let (duration, stats) = run_once(num_txns, &contention, false);
        durations_no_bloom.push(duration);
        stats_no_bloom_list.push(stats);
    }

    // 有 Bloom
    let mut durations_with_bloom = Vec::new();
    let mut stats_with_bloom_list = Vec::new();

    for _ in 0..BENCH_RUNS {
        let (duration, stats) = run_once(num_txns, &contention, true);
        durations_with_bloom.push(duration);
        stats_with_bloom_list.push(stats);
    }

    // 统计分析
    let stats_no = compute_stats(&durations_no_bloom, num_txns);
    let stats_with = compute_stats(&durations_with_bloom, num_txns);

    println!("  Without Bloom Filter:");
    println!(
        "    TPS: {:.0} ± {:.0} (min: {:.0}, max: {:.0})",
        stats_no.mean_tps, stats_no.stddev_tps, stats_no.min_tps, stats_no.max_tps
    );
    println!(
        "    Duration: {:.2}ms ± {:.2}ms",
        stats_no.mean_duration_ms, stats_no.stddev_duration_ms
    );

    let avg_conflicts_no = stats_no_bloom_list
        .iter()
        .map(|s| s.conflicts as f64)
        .sum::<f64>()
        / stats_no_bloom_list.len() as f64;
    let avg_retries_no = stats_no_bloom_list
        .iter()
        .map(|s| s.retries as f64)
        .sum::<f64>()
        / stats_no_bloom_list.len() as f64;

    println!("    Avg Conflicts: {:.1}", avg_conflicts_no);
    println!("    Avg Retries: {:.1}", avg_retries_no);

    println!("  With Bloom Filter:");
    println!(
        "    TPS: {:.0} ± {:.0} (min: {:.0}, max: {:.0})",
        stats_with.mean_tps, stats_with.stddev_tps, stats_with.min_tps, stats_with.max_tps
    );
    println!(
        "    Duration: {:.2}ms ± {:.2}ms",
        stats_with.mean_duration_ms, stats_with.stddev_duration_ms
    );

    let avg_conflicts_with = stats_with_bloom_list
        .iter()
        .map(|s| s.conflicts as f64)
        .sum::<f64>()
        / stats_with_bloom_list.len() as f64;
    let avg_retries_with = stats_with_bloom_list
        .iter()
        .map(|s| s.retries as f64)
        .sum::<f64>()
        / stats_with_bloom_list.len() as f64;
    let avg_bloom_hits = stats_with_bloom_list
        .iter()
        .map(|s| s.bloom_hits as f64)
        .sum::<f64>()
        / stats_with_bloom_list.len() as f64;
    let avg_bloom_misses = stats_with_bloom_list
        .iter()
        .map(|s| s.bloom_misses as f64)
        .sum::<f64>()
        / stats_with_bloom_list.len() as f64;

    println!("    Avg Conflicts: {:.1}", avg_conflicts_with);
    println!("    Avg Retries: {:.1}", avg_retries_with);
    println!("    Avg Bloom Hits: {:.1}", avg_bloom_hits);
    println!("    Avg Bloom Misses: {:.1}", avg_bloom_misses);

    if avg_bloom_hits + avg_bloom_misses > 0.0 {
        let bloom_eff = avg_bloom_hits / (avg_bloom_hits + avg_bloom_misses) * 100.0;
        println!("    Bloom Efficiency: {:.2}%", bloom_eff);
    }

    let improvement = ((stats_with.mean_tps / stats_no.mean_tps) - 1.0) * 100.0;
    println!("  Improvement: {:.2}% (mean TPS)", improvement);
}

fn run_once(
    num_txns: usize,
    contention: &Contention,
    enable_bloom: bool,
) -> (Duration, BenchStats) {
    let mut config = OptimizedSchedulerConfig::default();
    config.enable_bloom_filter = enable_bloom;
    config.use_key_index_grouping = true;
    config.enable_batch_commit = true;
    config.min_batch_size = 10;
    config.mvcc_config = GcConfig {
        max_versions_per_key: 100,
        enable_time_based_gc: false,
        version_ttl_secs: 3600,
        auto_gc: None,
    };

    let scheduler = OptimizedMvccScheduler::new_with_config(config);

    let start = Instant::now();

    // 简单串行执行（暂时），后续可换成批量
    match contention {
        Contention::Low => {
            for i in 0..num_txns {
                let key = format!("key_{}", i);
                let _ = scheduler.execute_txn(i as u64, |txn| {
                    txn.write(key.as_bytes().to_vec(), b"value".to_vec());
                    Ok(1)
                });
            }
        }
        Contention::High => {
            for i in 0..num_txns {
                let _ = scheduler.execute_txn(i as u64, |txn| {
                    txn.write(b"shared_key".to_vec(), format!("value_{}", i).into_bytes());
                    Ok(1)
                });
            }
        }
        Contention::Medium(num_keys) => {
            for i in 0..num_txns {
                let key = format!("key_{}", i % num_keys);
                let _ = scheduler.execute_txn(i as u64, |txn| {
                    txn.write(key.as_bytes().to_vec(), format!("value_{}", i).into_bytes());
                    Ok(1)
                });
            }
        }
    }

    let duration = start.elapsed();

    let stats_data = scheduler.get_stats();
    let bench_stats = BenchStats {
        conflicts: stats_data.basic.conflict_count,
        retries: stats_data.basic.retry_count,
        bloom_hits: stats_data.bloom_hits,
        bloom_misses: stats_data.bloom_misses,
    };

    (duration, bench_stats)
}

#[derive(Clone)]
struct BenchStats {
    conflicts: u64,
    retries: u64,
    bloom_hits: u64,
    bloom_misses: u64,
}

struct Statistics {
    mean_tps: f64,
    stddev_tps: f64,
    min_tps: f64,
    max_tps: f64,
    mean_duration_ms: f64,
    stddev_duration_ms: f64,
}

fn compute_stats(durations: &[Duration], num_txns: usize) -> Statistics {
    let tps_values: Vec<f64> = durations
        .iter()
        .map(|d| num_txns as f64 / d.as_secs_f64())
        .collect();

    let duration_ms: Vec<f64> = durations.iter().map(|d| d.as_secs_f64() * 1000.0).collect();

    let mean_tps = tps_values.iter().sum::<f64>() / tps_values.len() as f64;
    let variance_tps = tps_values
        .iter()
        .map(|v| (v - mean_tps).powi(2))
        .sum::<f64>()
        / tps_values.len() as f64;
    let stddev_tps = variance_tps.sqrt();

    let mean_duration_ms = duration_ms.iter().sum::<f64>() / duration_ms.len() as f64;
    let variance_duration = duration_ms
        .iter()
        .map(|v| (v - mean_duration_ms).powi(2))
        .sum::<f64>()
        / duration_ms.len() as f64;
    let stddev_duration_ms = variance_duration.sqrt();

    let min_tps = tps_values.iter().cloned().fold(f64::INFINITY, f64::min);
    let max_tps = tps_values.iter().cloned().fold(f64::NEG_INFINITY, f64::max);

    Statistics {
        mean_tps,
        stddev_tps,
        min_tps,
        max_tps,
        mean_duration_ms,
        stddev_duration_ms,
    }
}
