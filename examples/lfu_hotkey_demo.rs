// SPDX-License-Identifier: GPL-3.0-or-later
//! Minimal LFU + tiered hot key demo
//! Reads env vars:
//!   LFU_MEDIUM, LFU_HIGH, LFU_DECAY_PERIOD, LFU_DECAY_FACTOR, LFU_BATCHES
//! Prints one summary line with counts and TPS.

use std::env;
use std::sync::{Arc, Barrier};
use std::thread;
use std::time::Instant;
use vm_runtime::{mvcc::GcConfig, OptimizedMvccScheduler, OptimizedSchedulerConfig, Txn};

const NUM_THREADS: usize = 4;
const TXNS_PER_THREAD: usize = 160;
const BATCH_SIZE: usize = 20;
const NUM_HOT_KEYS: usize = 6; // frequency gradient

fn main() {
    let medium = env::var("LFU_MEDIUM")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(40);
    let high = env::var("LFU_HIGH")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(120);
    let decay_period = env::var("LFU_DECAY_PERIOD")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(10);
    let decay_factor = env::var("LFU_DECAY_FACTOR")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(0.9);
    let batches = env::var("LFU_BATCHES")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(3);

    let mut cfg = OptimizedSchedulerConfig::default();
    cfg.enable_bloom_filter = true;
    cfg.use_key_index_grouping = true;
    cfg.enable_batch_commit = true;
    cfg.min_batch_size = 10;
    cfg.enable_owner_sharding = true;
    cfg.num_shards = 8;
    cfg.enable_hot_key_isolation = true;
    cfg.hot_key_threshold = 5;
    cfg.enable_hot_key_bucketing = true;
    cfg.enable_lfu_tracking = true;
    cfg.lfu_hot_key_threshold_medium = medium as u64;
    cfg.lfu_hot_key_threshold_high = high as u64;
    cfg.lfu_hot_key_threshold = medium as u64;
    cfg.lfu_decay_period = decay_period as u64;
    cfg.lfu_decay_factor = decay_factor;
    cfg.mvcc_config = GcConfig {
        max_versions_per_key: 2000,
        enable_time_based_gc: false,
        version_ttl_secs: 3600,
        auto_gc: None,
    };
    let sched = Arc::new(OptimizedMvccScheduler::new_with_config(cfg));
    let barrier = Arc::new(Barrier::new(NUM_THREADS));
    let start = Instant::now();
    for _ in 0..batches {
        // run a few batches
        let handles: Vec<_> = (0..NUM_THREADS)
            .map(|tid| {
                let s = Arc::clone(&sched);
                let b = barrier.clone();
                thread::spawn(move || {
                    b.wait();
                    for batch_start in (0..TXNS_PER_THREAD).step_by(BATCH_SIZE) {
                        let batch_end = (batch_start + BATCH_SIZE).min(TXNS_PER_THREAD);
                        let txs: Vec<_> = (batch_start..batch_end)
                            .map(|i| {
                                let tx_id = (tid * TXNS_PER_THREAD + i) as u64;
                                let is_single = ((i as f64) / (TXNS_PER_THREAD as f64)) < 0.5;
                                let keys: Vec<Vec<u8>> = if is_single {
                                    vec![format!("k_{}_{}", tid, i).into_bytes()]
                                } else {
                                    let hot_idx = (tx_id as usize) % NUM_HOT_KEYS;
                                    vec![
                                        format!("hot_k_{}", hot_idx).into_bytes(),
                                        format!("a2_{}_{}", tid, i).into_bytes(),
                                        format!("a3_{}_{}", tid, i).into_bytes(),
                                    ]
                                };
                                let keys_arc = Arc::new(keys);
                                let payload = Arc::new(format!("v{}_{}", tid, i).into_bytes());
                                (tx_id, move |txn: &mut Txn| {
                                    for k in keys_arc.iter() {
                                        txn.write(k.clone(), (*payload).clone());
                                    }
                                    Ok(0)
                                })
                            })
                            .collect();
                        let _ = s.execute_batch(txs);
                    }
                })
            })
            .collect();
        for h in handles {
            h.join().unwrap();
        }
    }
    let dur = start.elapsed();
    let stats = sched.get_stats();
    let tps = (batches as f64 * (NUM_THREADS * TXNS_PER_THREAD) as f64) / dur.as_secs_f64();
    let diag = stats.diagnostics;
    if let Some(d) = diag {
        println!(
            "TPS: {:.0} Duration(ms): {:.2} extreme {} medium {} batch {}",
            tps,
            dur.as_secs_f64() * 1000.0,
            d.extreme_hot_count,
            d.medium_hot_count,
            d.batch_hot_count
        );
    } else {
        println!(
            "TPS: {:.0} Duration(ms): {:.2} extreme 0 medium 0 batch 0",
            tps,
            dur.as_secs_f64() * 1000.0
        );
    }
}
