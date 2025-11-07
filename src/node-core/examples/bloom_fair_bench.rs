// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2025 XujueKing

//! 公平版 Bloom Filter 基准测试
//!
//! 目标：验证在“真实可受益”的条件下（大批量/高并发/批量提交路径）
//! Bloom Filter 对吞吐的实际影响。
//!
//! 关键点：
//! - 使用 execute_batch() 批量路径（非单条 execute_txn）
//! - 可配置：线程数、每线程交易数、批量大小、冲突模式
//! - 对比：禁用 Bloom vs 启用 Bloom 的 TPS、冲突率、Bloom 命中效率
//!
//! 环境变量：
//! - THREADS            (默认 16)
//! - TXNS_PER_THREAD    (默认 2000)
//! - BATCH_SIZE         (默认 200)
//! - CONTENTION         (low | medium10 | medium100 | high，默认 medium10)
//! - HOT_KEY_RATIO      (高竞争时热点比例，默认 0.6)
//!
//! 运行示例（PowerShell）：
//!   cargo run -p node-core --example bloom_fair_bench --release
//!   $env:THREADS=16; $env:TXNS_PER_THREAD=2000; $env:BATCH_SIZE=200; cargo run -p node-core --example bloom_fair_bench --release

use std::sync::{Arc, Barrier};
use std::thread;
use std::time::Instant;
use std::{env};

use vm_runtime::{OptimizedMvccScheduler, OptimizedSchedulerConfig, OptimizedSchedulerStats, mvcc::GcConfig};

#[derive(Clone, Copy, Debug)]
enum ContentionMode {
    Low,
    Medium(usize), // 共享键数量
    High,          // 单一热点键，按 HOT_KEY_RATIO 控制热键占比
}

fn parse_contention() -> ContentionMode {
    match env::var("CONTENTION").unwrap_or_else(|_| "medium10".to_string()).to_lowercase().as_str() {
        "low" => ContentionMode::Low,
        "high" => ContentionMode::High,
        s if s.starts_with("medium") => {
            let n = s.strip_prefix("medium").and_then(|v| v.parse::<usize>().ok()).unwrap_or(10);
            ContentionMode::Medium(n)
        }
        _ => ContentionMode::Medium(10),
    }
}

fn build_scheduler(enable_bloom: bool) -> OptimizedMvccScheduler {
    let mut cfg = OptimizedSchedulerConfig::default();
    cfg.enable_bloom_filter = enable_bloom;
    cfg.use_key_index_grouping = true;
    cfg.enable_batch_commit = true;
    // 为了专注评估 Bloom 分组效果，禁用 owner/sharding 快路径，避免绕过 Bloom
    cfg.enable_owner_sharding = false;
    cfg.min_batch_size = 10; // 总批次大小通过调用方控制；达到阈值即可走批量路径
    // 放宽候选密度回退阈值，避免高密度场景过早绕过 Bloom
    cfg.density_fallback_threshold = 0.50;
    cfg.mvcc_config = GcConfig {
        max_versions_per_key: 100,
        enable_time_based_gc: false,
        version_ttl_secs: 3600,
        auto_gc: None,
    };
    OptimizedMvccScheduler::new_with_config(cfg)
}

fn report(label: &str, start: Instant, total_tx: usize, stats: &OptimizedSchedulerStats) {
    let elapsed = start.elapsed().as_secs_f64();
    let tps = (total_tx as f64) / elapsed;
    let conflict_rate = stats.basic.conflict_rate() * 100.0;
    let bloom_eff = stats.bloom_efficiency() * 100.0;
    println!("{}:", label);
    println!("  Duration: {:.3}s  TPS: {:.0}", elapsed, tps);
    println!("  Success: {}  Failed: {}  Conflicts: {} ({:.2}%)  Retries: {}",
        stats.basic.successful_txs, stats.basic.failed_txs, stats.basic.conflict_count, conflict_rate, stats.basic.retry_count);
    if stats.bloom_hits > 0 || stats.bloom_misses > 0 {
        println!("  Bloom: hits={} misses={} eff={:.2}%", stats.bloom_hits, stats.bloom_misses, bloom_eff);
        if let Some(ref bf) = stats.bloom_filter_stats {
            println!("  Bloom cache: total_txns={} total_reads={} total_writes={} avg_fpr_r={:.4} avg_fpr_w={:.4}",
                bf.total_txns, bf.total_reads, bf.total_writes, bf.avg_false_positive_rate_read, bf.avg_false_positive_rate_write);
        }
    } else {
        println!("  Bloom: (no activity recorded)");
    }
}

fn main() {
    let num_threads = env::var("THREADS").ok().and_then(|v| v.parse().ok()).unwrap_or(16usize);
    let txns_per_thread = env::var("TXNS_PER_THREAD").ok().and_then(|v| v.parse().ok()).unwrap_or(2000usize);
    let mut batch_size = env::var("BATCH_SIZE").ok().and_then(|v| v.parse().ok()).unwrap_or(200usize);
    let hot_key_ratio = env::var("HOT_KEY_RATIO").ok().and_then(|v| v.parse::<f64>().ok()).unwrap_or(0.6);
    let contention = parse_contention();
    let auto_batch = env::var("AUTO_BATCH").ok().map(|v| v == "1" || v.eq_ignore_ascii_case("true") ).unwrap_or(false)
        || env::var("BATCH_SIZE").ok().map(|v| v.eq_ignore_ascii_case("auto")).unwrap_or(false);

    println!("=== Bloom Filter Fair Benchmark ===");
    println!("Threads: {}  Txns/Thread: {}  Batch: {}{}  Contention: {:?}  HotKeyRatio: {:.0}%", 
        num_threads, txns_per_thread, batch_size, if auto_batch { " (auto)" } else { "" }, contention, hot_key_ratio * 100.0);

    // Auto tune batch size (quick measurement, no Bloom)
    if auto_batch {
        let candidates = [50usize, 100, 200, 500, 1000];
        let probe_txns = txns_per_thread.min(500);
        let mut best = (0usize, 0.0);
        println!("\n[AutoBatch] Probing batch sizes on no-Bloom ({} tx/thread)...", probe_txns);
        for &cand in &candidates {
            let (tps, _stats) = run_once(num_threads, probe_txns, cand, hot_key_ratio, contention, false, true);
            println!("  - batch={} => {:.0} TPS", cand, tps);
            if tps > best.1 { best = (cand, tps); }
        }
        batch_size = if best.0 > 0 { best.0 } else { batch_size }; 
        println!("[AutoBatch] Chosen batch size: {}\n", batch_size);
    }

    // 预热一次（禁用 Bloom）
    run_once(num_threads, txns_per_thread, batch_size, hot_key_ratio, contention, false, true);

    // 正式对比：无 Bloom
    let (tps_no, stats_no) = run_once(num_threads, txns_per_thread, batch_size, hot_key_ratio, contention, false, false);
    // 正式对比：有 Bloom
    let (tps_yes, stats_yes) = run_once(num_threads, txns_per_thread, batch_size, hot_key_ratio, contention, true, false);

    println!("\n=== Summary ===");
    println!("Without Bloom: {:.0} TPS", tps_no);
    println!("With Bloom:    {:.0} TPS", tps_yes);
    if tps_no > 0.0 {
        println!("Delta: {:+.2}%", (tps_yes / tps_no - 1.0) * 100.0);
    }

    // 额外打印一次详细统计（可读性）
    println!("\n--- Detailed (With Bloom) ---");
    stats_yes.print_detailed();
    println!("\n--- Detailed (Without Bloom) ---");
    stats_no.print_detailed();
}

fn run_once(
    num_threads: usize,
    txns_per_thread: usize,
    batch_size: usize,
    hot_key_ratio: f64,
    contention: ContentionMode,
    enable_bloom: bool,
    silent: bool,
) -> (f64, OptimizedSchedulerStats) {
    let scheduler = build_scheduler(enable_bloom);
    let barrier = Arc::new(Barrier::new(num_threads));

    let total_tx = num_threads * txns_per_thread;
    let start = Instant::now();

    thread::scope(|scope| {
        for t in 0..num_threads {
            let barrier_c = barrier.clone();
            let scheduler_c = &scheduler;
            scope.spawn(move || {
                // 对每个线程：分批创建交易闭包并调用批量执行
                barrier_c.wait();
                let mut tx_id_base: u64 = (t * txns_per_thread) as u64;
                let mut generated = 0usize;
                while generated < txns_per_thread {
                    let remain = txns_per_thread - generated;
                    let this_batch = remain.min(batch_size);
                    let mut txs: Vec<(u64, Box<dyn Fn(&mut vm_runtime::mvcc::Txn) -> anyhow::Result<i32> + Send + Sync>)> = Vec::with_capacity(this_batch);

                    for i in 0..this_batch {
                        // 按竞争模式生成键
                        let (key, value) = match contention {
                            ContentionMode::Low => {
                                let k = format!("k_{}_{}", t, generated + i);
                                (k.into_bytes(), b"v".to_vec())
                            }
                            ContentionMode::Medium(n) => {
                                let idx = (generated + i) % n;
                                let k = format!("mk{}_{}", idx, t);
                                (k.into_bytes(), (generated + i).to_string().into_bytes())
                            }
                            ContentionMode::High => {
                                // 按比例走热点键或独立键
                                let frac = (generated + i) as f64 / txns_per_thread as f64;
                                if frac < hot_key_ratio { (b"hot".to_vec(), (generated + i).to_string().into_bytes()) }
                                else { let k = format!("cold_{}_{}", t, generated + i); (k.into_bytes(), b"v".to_vec()) }
                            }
                        };

                        let id = tx_id_base;
                        tx_id_base += 1;
                        let f = move |txn: &mut vm_runtime::mvcc::Txn| -> anyhow::Result<i32> {
                            txn.write(key.clone(), value.clone());
                            Ok(1)
                        };
                        // 将闭包装箱以满足 trait 对象要求
                        txs.push((id, Box::new(f)));
                    }

                    // 转换为期望签名：Vec<(TxId, F)>，其中 F: Fn(&mut Txn)+Send+Sync
                    let txs_std: Vec<(u64, _)> = txs.into_iter().map(|(id, f)| {
                        (id, move |txn: &mut vm_runtime::mvcc::Txn| -> anyhow::Result<i32> { (f)(txn) })
                    }).collect();

                    let _result = scheduler_c.execute_batch(txs_std);
                    generated += this_batch;
                }
            });
        }
    });

    let elapsed = start.elapsed().as_secs_f64();
    let tps = (total_tx as f64) / elapsed;
    let stats = scheduler.get_stats();
    let label = if enable_bloom { "With Bloom" } else { "Without Bloom" };
    if !silent { report(label, start, total_tx, &stats); }
    (tps, stats)
}
