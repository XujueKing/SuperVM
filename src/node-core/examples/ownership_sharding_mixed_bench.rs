// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2025 XujueKing

//! 所有权分片 + Bloom 组合的混合负载基准
//!
//! 目标：在混合负载（80% 单分片、20% 跨分片）下评估：
//! - 仅 Bloom
//! - 仅分片（owner-sharding）
//! - Bloom + 分片
//! - 都关闭（基线）
//! 的 TPS 与冲突情况。
//!
//! 注意：分片判定通过键哈希近似模拟；单分片事务仅写入一个键；
//! 跨分片事务写入 3 个不同键（概率上跨 >=2 个分片）。

use std::env;
use std::sync::{Arc, Barrier};
use std::thread;
use std::time::{Duration, Instant};
use vm_runtime::{mvcc::GcConfig, OptimizedMvccScheduler, OptimizedSchedulerConfig, Txn};

const SINGLE_SHARD_RATIO: f64 = 0.8; // 80% 单分片
const NUM_SHARDS: usize = 8; // 与调度器一致
const WARMUP_RUNS: usize = 1;
const BENCH_RUNS: usize = 3;
const NUM_HOT_KEYS: usize = 8; // 热键数量（默认更中性）

fn main() {
    let num_threads = env::var("NUM_THREADS")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(8usize);
    let txns_per_thread = env::var("TX_PER_THREAD")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(200usize);
    let batch_size = env::var("BATCH_SIZE")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(20usize);

    println!("=== Ownership Sharding Mixed-Load Benchmark ===");
    println!(
        "Threads: {}, Txns/Thread: {}, Single-shard Ratio: {:.0}% (batch={})\n",
        num_threads,
        txns_per_thread,
        SINGLE_SHARD_RATIO * 100.0,
        batch_size
    );

    // 预热
    for _ in 0..WARMUP_RUNS {
        run_scenario(false, false, 0, num_threads, txns_per_thread, batch_size);
    }

    // 四组配置(不启用热键隔离)
    println!("\n=== Without Hot Key Isolation ===");
    run_and_report(
        "Baseline (no Bloom, no Sharding)",
        false,
        false,
        false,
        num_threads,
        txns_per_thread,
        batch_size,
    );
    run_and_report(
        "Bloom only",
        true,
        false,
        false,
        num_threads,
        txns_per_thread,
        batch_size,
    );
    run_and_report(
        "Sharding only",
        false,
        true,
        false,
        num_threads,
        txns_per_thread,
        batch_size,
    );
    run_and_report(
        "Bloom + Sharding",
        true,
        true,
        false,
        num_threads,
        txns_per_thread,
        batch_size,
    );

    // 启用热键隔离的对比 (threshold=5)
    println!("\n\n=== With Hot Key Isolation (threshold=5) ===");
    run_and_report_with_threshold(
        "Baseline + HotKey(5)",
        false,
        false,
        5,
        num_threads,
        txns_per_thread,
        batch_size,
    );
    run_and_report_with_threshold(
        "Bloom only + HotKey(5)",
        true,
        false,
        5,
        num_threads,
        txns_per_thread,
        batch_size,
    );
    run_and_report_with_threshold(
        "Sharding only + HotKey(5)",
        false,
        true,
        5,
        num_threads,
        txns_per_thread,
        batch_size,
    );
    run_and_report_with_threshold(
        "Bloom + Sharding + HotKey(5)",
        true,
        true,
        5,
        num_threads,
        txns_per_thread,
        batch_size,
    );

    // 热键阈值调优对比
    println!("\n\n=== Hot Key Threshold Tuning (Bloom + Sharding) ===");
    run_and_report_with_threshold(
        "Threshold = 3",
        true,
        true,
        3,
        num_threads,
        txns_per_thread,
        batch_size,
    );
    run_and_report_with_threshold(
        "Threshold = 5",
        true,
        true,
        5,
        num_threads,
        txns_per_thread,
        batch_size,
    );
    run_and_report_with_threshold(
        "Threshold = 7",
        true,
        true,
        7,
        num_threads,
        txns_per_thread,
        batch_size,
    );
    run_and_report_with_threshold(
        "Threshold = 10",
        true,
        true,
        10,
        num_threads,
        txns_per_thread,
        batch_size,
    );

    // 热键分桶并发对比
    println!("\n\n=== Hot Key Bucketing (Bloom + Sharding, threshold=5) ===");
    run_and_report_with_options(
        "Serial Hot-Key Processing",
        true,
        true,
        5,
        false,
        false,
        false,
        num_threads,
        txns_per_thread,
        batch_size,
    );
    run_and_report_with_options(
        "Bucketed Hot-Key Processing",
        true,
        true,
        5,
        true,
        false,
        false,
        num_threads,
        txns_per_thread,
        batch_size,
    );

    // LFU 全局热键跟踪对比
    println!("\n\n=== LFU Global Hot Key Tracking (Bloom + Sharding, threshold=5) ===");
    run_and_report_with_options(
        "HotKey(5) + No-LFU",
        true,
        true,
        5,
        true,
        false,
        false,
        num_threads,
        txns_per_thread,
        batch_size,
    );
    run_and_report_with_options(
        "HotKey(5) + LFU(10/0.9/50)",
        true,
        true,
        5,
        true,
        true,
        false,
        num_threads,
        txns_per_thread,
        batch_size,
    );
    run_and_report_with_options(
        "HotKey(Adaptive) + LFU",
        true,
        true,
        5,
        true,
        true,
        true,
        num_threads,
        txns_per_thread,
        batch_size,
    );
}

fn run_and_report(
    label: &str,
    enable_bloom: bool,
    enable_sharding: bool,
    enable_hot_key: bool,
    num_threads: usize,
    txns_per_thread: usize,
    batch_size: usize,
) {
    let threshold = if enable_hot_key { 5 } else { 0 };
    run_and_report_with_threshold(
        label,
        enable_bloom,
        enable_sharding,
        threshold,
        num_threads,
        txns_per_thread,
        batch_size,
    )
}

fn run_and_report_with_threshold(
    label: &str,
    enable_bloom: bool,
    enable_sharding: bool,
    hot_key_threshold: usize,
    num_threads: usize,
    txns_per_thread: usize,
    batch_size: usize,
) {
    run_and_report_with_options(
        label,
        enable_bloom,
        enable_sharding,
        hot_key_threshold,
        false,
        false,
        false,
        num_threads,
        txns_per_thread,
        batch_size,
    )
}

fn run_and_report_with_options(
    label: &str,
    enable_bloom: bool,
    enable_sharding: bool,
    hot_key_threshold: usize,
    enable_bucketing: bool,
    enable_lfu: bool,
    enable_adaptive: bool,
    num_threads: usize,
    txns_per_thread: usize,
    batch_size: usize,
) {
    println!("\n--- {} ---", label);

    let mut durations = Vec::new();
    let mut conflicts = Vec::new();
    let mut successes = Vec::new();
    // 诊断指标累积
    let mut d_may_total = Vec::new();
    let mut d_may_true = Vec::new();
    let mut d_precise_checks = Vec::new();
    let mut d_precise_edges = Vec::new();
    let mut d_groups = Vec::new();
    let mut d_grouped_txns = Vec::new();
    let mut d_group_max = Vec::new();
    let mut d_density = Vec::new();
    let mut bench_last_diag: Option<vm_runtime::optimized_mvcc::OptimizedDiagnosticsStats> = None;

    for _ in 0..BENCH_RUNS {
        let (dur, stat) = run_scenario_with_options(
            enable_bloom,
            enable_sharding,
            hot_key_threshold,
            enable_bucketing,
            enable_lfu,
            enable_adaptive,
            num_threads,
            txns_per_thread,
            batch_size,
        );
        durations.push(dur);
        conflicts.push(stat.conflicts as f64);
        successes.push(stat.successful as f64);
        if let Some(diag) = stat.diagnostics.clone() {
            d_may_total.push(diag.bloom_may_conflict_total as f64);
            d_may_true.push(diag.bloom_may_conflict_true as f64);
            d_precise_checks.push(diag.precise_checks as f64);
            d_precise_edges.push(diag.precise_conflicts_added as f64);
            d_groups.push(diag.groups_built as f64);
            d_grouped_txns.push(diag.grouped_txns_total as f64);
            d_group_max.push(diag.group_max_size as f64);
            d_density.push(diag.candidate_density);
            bench_last_diag = Some(diag);
        }
    }

    let total_txns = (num_threads * txns_per_thread) as f64;
    let st = compute_stats(&durations, total_txns);

    let avg_conflicts = mean(&conflicts);
    let avg_success = mean(&successes);

    println!(
        "TPS: {:.0} ± {:.0} (min: {:.0}, max: {:.0}) | Duration: {:.2}ms ± {:.2}ms",
        st.mean_tps,
        st.stddev_tps,
        st.min_tps,
        st.max_tps,
        st.mean_duration_ms,
        st.stddev_duration_ms
    );
    println!(
        "Avg Successful: {:.1} / run | Avg Conflicts: {:.1}",
        avg_success, avg_conflicts
    );
    if !d_may_total.is_empty() {
        // 打印基础诊断
        print!(
            "Diag: may_conflict total {:.1}, true {:.1} | precise checks {:.1}, edges {:.1} | groups {:.1}, grouped_txns {:.1}, max_group {:.1} | density {:.4}",
            mean(&d_may_total), mean(&d_may_true), mean(&d_precise_checks), mean(&d_precise_edges),
            mean(&d_groups), mean(&d_grouped_txns), mean(&d_group_max), mean(&d_density)
        );
        // 追加自适应与阈值信息（若存在）
        if let Some(diag) = bench_last_diag {
            print!(
                " | hot_key_thr {} | adaptive conf {:.3} dens {:.3} | extreme {} medium {} batch {}",
                diag.current_hot_key_threshold,
                diag.adaptive_avg_conflict_rate,
                diag.adaptive_avg_density,
                diag.extreme_hot_count,
                diag.medium_hot_count,
                diag.batch_hot_count
            );
        }
        println!();
    }
}

fn run_scenario(
    enable_bloom: bool,
    enable_sharding: bool,
    hot_key_threshold: usize,
    num_threads: usize,
    txns_per_thread: usize,
    batch_size: usize,
) -> (Duration, BenchStats) {
    run_scenario_with_options(
        enable_bloom,
        enable_sharding,
        hot_key_threshold,
        false,
        false,
        false,
        num_threads,
        txns_per_thread,
        batch_size,
    )
}

fn run_scenario_with_options(
    enable_bloom: bool,
    enable_sharding: bool,
    hot_key_threshold: usize,
    enable_bucketing: bool,
    enable_lfu: bool,
    enable_adaptive: bool,
    num_threads: usize,
    txns_per_thread: usize,
    batch_size: usize,
) -> (Duration, BenchStats) {
    use std::env;
    let mut config = OptimizedSchedulerConfig::default();
    config.enable_bloom_filter = enable_bloom;
    config.use_key_index_grouping = true;
    config.enable_batch_commit = true;
    config.min_batch_size = 10;
    config.max_retries = 5;
    config.enable_owner_sharding = enable_sharding;
    config.num_shards = NUM_SHARDS;
    // 允许通过环境变量覆盖关键参数，便于自动化脚本调参
    let env_hot = env::var("HOT_KEY_THRESHOLD")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(hot_key_threshold);
    let env_medium = env::var("LFU_MEDIUM")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(20);
    let env_high = env::var("LFU_HIGH")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(50);
    let env_decay_p = env::var("LFU_DECAY_PERIOD")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(10u64);
    let env_decay_f = env::var("LFU_DECAY_FACTOR")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(0.9f64);
    let env_adaptive = env::var("ADAPTIVE")
        .ok()
        .map(|v| v == "1" || v.to_lowercase() == "true")
        .unwrap_or(enable_adaptive);
    config.enable_hot_key_isolation = env_hot > 0;
    config.hot_key_threshold = env_hot as usize;
    config.enable_hot_key_bucketing = enable_bucketing;
    config.enable_lfu_tracking =
        enable_lfu || env::var("LFU_MEDIUM").is_ok() || env::var("LFU_HIGH").is_ok();
    config.enable_adaptive_hot_key = env_adaptive;
    config.lfu_decay_period = env_decay_p;
    config.lfu_decay_factor = env_decay_f;
    config.lfu_hot_key_threshold_medium = env_medium as u64;
    config.lfu_hot_key_threshold_high = env_high as u64;
    config.lfu_hot_key_threshold = env_medium as u64; // 兼容字段
    config.mvcc_config = GcConfig {
        max_versions_per_key: 2000,
        enable_time_based_gc: false,
        version_ttl_secs: 3600,
        auto_gc: None,
    };

    let scheduler = Arc::new(OptimizedMvccScheduler::new_with_config(config));
    let barrier = Arc::new(Barrier::new(num_threads));
    let start = Instant::now();

    let handles: Vec<_> = (0..num_threads)
        .map(|tid| {
            let scheduler = Arc::clone(&scheduler);
            let barrier = Arc::clone(&barrier);
            let txns_per_thread_local = txns_per_thread;
            let batch_size_local = batch_size;
            thread::spawn(move || {
                barrier.wait();
                for batch_start in (0..txns_per_thread_local).step_by(batch_size_local) {
                    let batch_end = (batch_start + batch_size_local).min(txns_per_thread_local);
                    let transactions: Vec<_> = (batch_start..batch_end)
                        .map(|i| {
                            let tx_id = (tid * txns_per_thread_local + i) as u64;
                            let is_single =
                                (i as f64 / txns_per_thread_local as f64) < SINGLE_SHARD_RATIO;

                            // 统一闭包创建：构造要写入的键集合与载荷，再返回相同类型的闭包
                            let (keys_vec, payload_bytes): (Vec<Vec<u8>>, Vec<u8>) = if is_single {
                                (
                                    vec![format!("s_key_{}_{}", tid, i).into_bytes()],
                                    format!("v_{}_{}", tid, i).into_bytes(),
                                )
                            } else {
                                (
                                    {
                                        // 包含一个共享热键 + 2 个独立键（高概率跨分片且产生冲突）
                                        let hot_idx = (tx_id as usize) % NUM_HOT_KEYS;
                                        vec![
                                            format!("hot_k_{}", hot_idx).into_bytes(),
                                            format!("m2_{}_{}", tid, i).into_bytes(),
                                            format!("m3_{}_{}", tid, i).into_bytes(),
                                        ]
                                    },
                                    format!("vx_{}_{}", tid, i).into_bytes(),
                                )
                            };

                            let keys_arc = Arc::new(keys_vec);
                            let payload_arc = Arc::new(payload_bytes);
                            (tx_id, move |txn: &mut Txn| {
                                let v = (*payload_arc).clone();
                                for k in keys_arc.iter() {
                                    txn.write(k.clone(), v.clone());
                                }
                                Ok(1)
                            })
                        })
                        .collect();

                    let _ = scheduler.execute_batch(transactions);
                }
            })
        })
        .collect();

    for h in handles {
        h.join().unwrap();
    }
    let duration = start.elapsed();

    let st = scheduler.get_stats();
    let bench_stats = BenchStats {
        successful: st.basic.successful_txs,
        failed: st.basic.failed_txs,
        conflicts: st.basic.conflict_count,
        retries: st.basic.retry_count,
        diagnostics: st.diagnostics,
    };

    (duration, bench_stats)
}

#[derive(Clone, Debug)]
struct BenchStats {
    successful: u64,
    failed: u64,
    conflicts: u64,
    retries: u64,
    diagnostics: Option<vm_runtime::optimized_mvcc::OptimizedDiagnosticsStats>,
}

struct Statistics {
    mean_tps: f64,
    stddev_tps: f64,
    min_tps: f64,
    max_tps: f64,
    mean_duration_ms: f64,
    stddev_duration_ms: f64,
}

fn compute_stats(durations: &[Duration], total_txns: f64) -> Statistics {
    let tps: Vec<f64> = durations
        .iter()
        .map(|d| total_txns / d.as_secs_f64())
        .collect();
    let dur_ms: Vec<f64> = durations.iter().map(|d| d.as_secs_f64() * 1000.0).collect();

    let mean_tps = mean(&tps);
    let stddev_tps = stddev(&tps, mean_tps);
    let mean_duration_ms = mean(&dur_ms);
    let stddev_duration_ms = stddev(&dur_ms, mean_duration_ms);

    let min_tps = tps.iter().cloned().fold(f64::INFINITY, f64::min);
    let max_tps = tps.iter().cloned().fold(f64::NEG_INFINITY, f64::max);

    Statistics {
        mean_tps,
        stddev_tps,
        min_tps,
        max_tps,
        mean_duration_ms,
        stddev_duration_ms,
    }
}

fn mean(v: &[f64]) -> f64 {
    v.iter().sum::<f64>() / v.len() as f64
}
fn stddev(v: &[f64], m: f64) -> f64 {
    (v.iter().map(|x| (x - m).powi(2)).sum::<f64>() / v.len() as f64).sqrt()
}
