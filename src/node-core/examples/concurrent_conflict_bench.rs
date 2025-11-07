// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2025 XujueKing <leadbrand@me.com>

//! 并发冲突场景基准测试
//! 
//! 目标：验证 Bloom Filter 在真实并发写冲突下的性能提升
//! 
//! 场景：
//! - 多线程同时提交批量事务
//! - 80% 事务写共享热键（产生真实冲突）
//! - 20% 事务写独立键（无冲突）
//! 
//! 预期：
//! - Bloom 分组能识别可并行事务，提升批量提交并行度
//! - 验收标准：有 Bloom TPS ≥ 无 Bloom +15%

use std::sync::{Arc, Barrier};
use std::thread;
use std::time::{Duration, Instant};
use vm_runtime::{OptimizedMvccScheduler, OptimizedSchedulerConfig, mvcc::GcConfig};

const NUM_THREADS: usize = 10;
const TXNS_PER_THREAD: usize = 100;
const NUM_HOT_KEYS: usize = 5;
const HOT_KEY_RATIO: f64 = 0.8;
const WARMUP_RUNS: usize = 2;
const BENCH_RUNS: usize = 5;

fn main() {
    println!("=== Concurrent Conflict Benchmark ===");
    println!("Threads: {}, Txns/Thread: {}, Hot Keys: {}", 
        NUM_THREADS, TXNS_PER_THREAD, NUM_HOT_KEYS);
    println!("Hot Key Ratio: {:.0}%, Warmup: {}, Runs: {}\n", 
        HOT_KEY_RATIO * 100.0, WARMUP_RUNS, BENCH_RUNS);
    
    // Warmup
    println!("Warming up...");
    for _ in 0..WARMUP_RUNS {
        run_concurrent_test(false);
        run_concurrent_test(true);
    }
    
    println!("\nBenchmarking...");
    
    // 无 Bloom
    let mut durations_no_bloom = Vec::new();
    let mut stats_no_bloom = Vec::new();
    
    for run in 1..=BENCH_RUNS {
        println!("  Run {}/{} (No Bloom)...", run, BENCH_RUNS);
        let (duration, stats) = run_concurrent_test(false);
        durations_no_bloom.push(duration);
        stats_no_bloom.push(stats);
    }
    
    // 有 Bloom
    let mut durations_with_bloom = Vec::new();
    let mut stats_with_bloom = Vec::new();
    
    for run in 1..=BENCH_RUNS {
        println!("  Run {}/{} (With Bloom)...", run, BENCH_RUNS);
        let (duration, stats) = run_concurrent_test(true);
        durations_with_bloom.push(duration);
        stats_with_bloom.push(stats);
    }
    
    // 统计分析
    println!("\n=== Results ===\n");
    
    let total_txns = NUM_THREADS * TXNS_PER_THREAD;
    
    let stats_no = compute_stats(&durations_no_bloom, total_txns);
    let stats_with = compute_stats(&durations_with_bloom, total_txns);
    
    println!("Without Bloom Filter:");
    println!("  TPS: {:.0} ± {:.0} (min: {:.0}, max: {:.0})", 
        stats_no.mean_tps, stats_no.stddev_tps, stats_no.min_tps, stats_no.max_tps);
    println!("  Duration: {:.2}ms ± {:.2}ms", 
        stats_no.mean_duration_ms, stats_no.stddev_duration_ms);
    
    let avg_conflicts_no = stats_no_bloom.iter()
        .map(|s| s.conflicts as f64).sum::<f64>() / stats_no_bloom.len() as f64;
    let avg_success_no = stats_no_bloom.iter()
        .map(|s| s.successful as f64).sum::<f64>() / stats_no_bloom.len() as f64;
    
    println!("  Avg Successful: {:.1}", avg_success_no);
    println!("  Avg Conflicts: {:.1}", avg_conflicts_no);
    
    println!("\nWith Bloom Filter:");
    println!("  TPS: {:.0} ± {:.0} (min: {:.0}, max: {:.0})", 
        stats_with.mean_tps, stats_with.stddev_tps, stats_with.min_tps, stats_with.max_tps);
    println!("  Duration: {:.2}ms ± {:.2}ms", 
        stats_with.mean_duration_ms, stats_with.stddev_duration_ms);
    
    let avg_conflicts_with = stats_with_bloom.iter()
        .map(|s| s.conflicts as f64).sum::<f64>() / stats_with_bloom.len() as f64;
    let avg_success_with = stats_with_bloom.iter()
        .map(|s| s.successful as f64).sum::<f64>() / stats_with_bloom.len() as f64;
    let avg_bloom_hits = stats_with_bloom.iter()
        .map(|s| s.bloom_hits as f64).sum::<f64>() / stats_with_bloom.len() as f64;
    let avg_bloom_misses = stats_with_bloom.iter()
        .map(|s| s.bloom_misses as f64).sum::<f64>() / stats_with_bloom.len() as f64;
    
    println!("  Avg Successful: {:.1}", avg_success_with);
    println!("  Avg Conflicts: {:.1}", avg_conflicts_with);
    println!("  Avg Bloom Hits: {:.1}", avg_bloom_hits);
    println!("  Avg Bloom Misses: {:.1}", avg_bloom_misses);
    
    if avg_bloom_hits + avg_bloom_misses > 0.0 {
        let bloom_eff = avg_bloom_hits / (avg_bloom_hits + avg_bloom_misses) * 100.0;
        println!("  Bloom Efficiency: {:.2}%", bloom_eff);
    }
    
    let improvement = ((stats_with.mean_tps / stats_no.mean_tps) - 1.0) * 100.0;
    println!("\nImprovement: {:.2}%", improvement);
    
    if improvement >= 15.0 {
        println!("✅ SUCCESS: Bloom Filter optimization validated (+15% target met)");
    } else if improvement >= 5.0 {
        println!("⚠️  PARTIAL: Some improvement but below +15% target");
    } else if improvement >= 0.0 {
        println!("⚠️  MARGINAL: Minimal improvement");
    } else {
        println!("❌ REGRESSION: Performance decreased (may need further tuning)");
    }
}

fn run_concurrent_test(enable_bloom: bool) -> (Duration, BenchStats) {
    let mut config = OptimizedSchedulerConfig::default();
    config.enable_bloom_filter = enable_bloom;
    config.use_key_index_grouping = true;
    config.enable_batch_commit = true;
    config.min_batch_size = 10;
    config.max_retries = 5;
    config.mvcc_config = GcConfig {
        max_versions_per_key: 1000,
        enable_time_based_gc: false,
        version_ttl_secs: 3600,
        auto_gc: None,
    };
    
    let scheduler = Arc::new(OptimizedMvccScheduler::new_with_config(config));
    let barrier = Arc::new(Barrier::new(NUM_THREADS));
    
    let start = Instant::now();
    
    // 启动多个线程，每个线程提交一批事务
    let handles: Vec<_> = (0..NUM_THREADS)
        .map(|thread_id| {
            let scheduler_clone = Arc::clone(&scheduler);
            let barrier_clone = Arc::clone(&barrier);
            
            thread::spawn(move || {
                // 等待所有线程就绪
                barrier_clone.wait();
                
                // 使用小批量构造交易并走批量执行路径，以触发 Bloom 分组
                const BATCH_SIZE: usize = 20;
                for batch_start in (0..TXNS_PER_THREAD).step_by(BATCH_SIZE) {
                    let batch_end = (batch_start + BATCH_SIZE).min(TXNS_PER_THREAD);
                    let transactions: Vec<_> = (batch_start..batch_end)
                        .map(|i| {
                            let tx_id = (thread_id * TXNS_PER_THREAD + i) as u64;
                            let is_hot = (i as f64 / TXNS_PER_THREAD as f64) < HOT_KEY_RATIO;
                            // 统一闭包创建点：先计算 key 和 value，再用一个 move 闭包
                            let key = if is_hot {
                                let hot_key_idx = (tx_id as usize) % NUM_HOT_KEYS;
                                format!("hot_key_{}", hot_key_idx)
                            } else {
                                format!("cold_key_{}_{}", thread_id, i)
                            };
                            let value_vec: Vec<u8> = if is_hot {
                                format!("value_{}_{}", thread_id, i).into_bytes()
                            } else {
                                b"value".to_vec()
                            };
                            let value_arc = std::sync::Arc::new(value_vec);
                            (tx_id, move |txn: &mut vm_runtime::mvcc::Txn| {
                                // 使用克隆避免消费 captured 变量，使闭包实现 Fn
                                let v = (*value_arc).clone();
                                txn.write(key.as_bytes().to_vec(), v);
                                Ok(1)
                            })
                        })
                        .collect();
                    let _batch_result = scheduler_clone.execute_batch(transactions);
                }
            })
        })
        .collect();
    
    // 等待所有线程完成
    for handle in handles {
        handle.join().unwrap();
    }
    
    let duration = start.elapsed();
    
    let stats_data = scheduler.get_stats();
    let bench_stats = BenchStats {
        successful: stats_data.basic.successful_txs,
        failed: stats_data.basic.failed_txs,
        conflicts: stats_data.basic.conflict_count,
        retries: stats_data.basic.retry_count,
        bloom_hits: stats_data.bloom_hits,
        bloom_misses: stats_data.bloom_misses,
    };
    
    (duration, bench_stats)
}

#[derive(Clone, Debug)]
struct BenchStats {
    successful: u64,
    failed: u64,
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
    let tps_values: Vec<f64> = durations.iter()
        .map(|d| num_txns as f64 / d.as_secs_f64())
        .collect();
    
    let duration_ms: Vec<f64> = durations.iter()
        .map(|d| d.as_secs_f64() * 1000.0)
        .collect();
    
    let mean_tps = tps_values.iter().sum::<f64>() / tps_values.len() as f64;
    let variance_tps = tps_values.iter()
        .map(|v| (v - mean_tps).powi(2))
        .sum::<f64>() / tps_values.len() as f64;
    let stddev_tps = variance_tps.sqrt();
    
    let mean_duration_ms = duration_ms.iter().sum::<f64>() / duration_ms.len() as f64;
    let variance_duration = duration_ms.iter()
        .map(|v| (v - mean_duration_ms).powi(2))
        .sum::<f64>() / duration_ms.len() as f64;
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
