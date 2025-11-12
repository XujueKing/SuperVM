// SPDX-License-Identifier: GPL-3.0-or-later
//! 多线程并发批量 2PC 性能基准测试
//!
//! 对比三种模式:
//! 1. 单线程原始 prepare_and_commit
//! 2. 单线程批量 batch_prepare + pipeline_commit
//! 3. 多线程并发批量 (rayon ThreadPool)

use std::sync::{Arc, Mutex};
use std::time::Instant;
use rayon::prelude::*;
use vm_runtime::{MvccStore, two_phase_consensus::TwoPhaseCoordinator};

fn main() {
    println!("╔════════════════════════════════════════════════════════════════════════╗");
    println!("║     Concurrent Batch 2PC Performance Benchmark                        ║");
    println!("╚════════════════════════════════════════════════════════════════════════╝\n");

    let total_txns = std::env::args()
        .nth(1)
        .and_then(|s| s.parse().ok())
        .unwrap_or(100000);

    let batch_size = std::env::args()
        .nth(2)
        .and_then(|s| s.parse().ok())
        .unwrap_or(32);

    let num_threads = std::env::args()
        .nth(3)
        .and_then(|s| s.parse().ok())
        .unwrap_or(8);

    println!("Configuration:");
    println!("  Total Transactions: {}", total_txns);
    println!("  Batch Size: {}", batch_size);
    println!("  Concurrent Threads: {}", num_threads);
    println!();

    // ===== 模式 1: 单线程原始 prepare_and_commit =====
    println!("╔════════════════════════════════════════════════════════════════════════╗");
    println!("║  Mode 1: Single-Threaded Original prepare_and_commit                  ║");
    println!("╚════════════════════════════════════════════════════════════════════════╝");

    let store1 = Arc::new(MvccStore::new());
    let coord1 = TwoPhaseCoordinator::new(Arc::clone(&store1));

    let start1 = Instant::now();
    let mut success1 = 0;

    for i in 0..total_txns {
        let mut txn = store1.begin();
        let key = format!("key_{}", i % 1000); // 1000 keys for conflict simulation
        let value = format!("value_{}", i);
        txn.write(key.as_bytes().to_vec(), value.as_bytes().to_vec());

        if coord1.prepare_and_commit(txn).is_ok() {
            success1 += 1;
        }
    }

    let elapsed1 = start1.elapsed();
    let tps1 = total_txns as f64 / elapsed1.as_secs_f64();

    println!("  Time Elapsed: {:.3}s", elapsed1.as_secs_f64());
    println!("  Successful Txns: {}/{}", success1, total_txns);
    println!("  Throughput: {:.2} TPS", tps1);
    println!();

    // ===== 模式 2: 单线程批量 batch_prepare + pipeline_commit =====
    println!("╔════════════════════════════════════════════════════════════════════════╗");
    println!("║  Mode 2: Single-Threaded Batch Prepare + Pipeline Commit              ║");
    println!("╚════════════════════════════════════════════════════════════════════════╝");

    let store2 = Arc::new(MvccStore::new());
    let coord2 = TwoPhaseCoordinator::new(Arc::clone(&store2));

    let start2 = Instant::now();
    let mut success2 = 0;
    let mut batch_txns = Vec::new();

    for i in 0..total_txns {
        let mut txn = store2.begin();
        let key = format!("key_{}", i % 1000);
        let value = format!("value_{}", i);
        txn.write(key.as_bytes().to_vec(), value.as_bytes().to_vec());
        batch_txns.push(txn);

        if batch_txns.len() >= batch_size || i == total_txns - 1 {
            let current_batch = std::mem::take(&mut batch_txns);
            match coord2.batch_prepare(current_batch) {
                Ok(prepared) => {
                    success2 += coord2.pipeline_commit(prepared);
                }
                Err((idx, err)) => {
                    eprintln!("Batch failed at index {}: {}", idx, err);
                }
            }
        }
    }

    let elapsed2 = start2.elapsed();
    let tps2 = total_txns as f64 / elapsed2.as_secs_f64();

    println!("  Time Elapsed: {:.3}s", elapsed2.as_secs_f64());
    println!("  Successful Txns: {}/{}", success2, total_txns);
    println!("  Throughput: {:.2} TPS", tps2);
    println!();

    // ===== 模式 3: 多线程并发批量 =====
    println!("╔════════════════════════════════════════════════════════════════════════╗");
    println!("║  Mode 3: Multi-Threaded Concurrent Batch (rayon ThreadPool)           ║");
    println!("╚════════════════════════════════════════════════════════════════════════╝");

    let store3 = Arc::new(MvccStore::new());
    let coord3 = TwoPhaseCoordinator::new(Arc::clone(&store3));

    let start3 = Instant::now();
    let success3 = Arc::new(Mutex::new(0));

    // 配置 rayon 线程池
    let pool = rayon::ThreadPoolBuilder::new()
        .num_threads(num_threads)
        .build()
        .unwrap();

    pool.install(|| {
        // 将事务分组，每个线程处理一批
        let txns_per_thread = total_txns / num_threads;
        
        (0..num_threads).into_par_iter().for_each(|thread_id| {
            let start_idx = thread_id * txns_per_thread;
            let end_idx = if thread_id == num_threads - 1 {
                total_txns
            } else {
                (thread_id + 1) * txns_per_thread
            };

            let mut local_success = 0;
            let mut batch_txns = Vec::new();

            for i in start_idx..end_idx {
                let mut txn = store3.begin();
                let key = format!("key_{}", i % 1000);
                let value = format!("value_{}", i);
                txn.write(key.as_bytes().to_vec(), value.as_bytes().to_vec());
                batch_txns.push(txn);

                if batch_txns.len() >= batch_size || i == end_idx - 1 {
                    let current_batch = std::mem::take(&mut batch_txns);
                    match coord3.batch_prepare(current_batch) {
                        Ok(prepared) => {
                            local_success += coord3.pipeline_commit(prepared);
                        }
                        Err((idx, err)) => {
                            eprintln!("Thread {} batch failed at index {}: {}", thread_id, idx, err);
                        }
                    }
                }
            }

            *success3.lock().unwrap() += local_success;
        });
    });

    let elapsed3 = start3.elapsed();
    let final_success3 = *success3.lock().unwrap();
    let tps3 = total_txns as f64 / elapsed3.as_secs_f64();

    println!("  Time Elapsed: {:.3}s", elapsed3.as_secs_f64());
    println!("  Successful Txns: {}/{}", final_success3, total_txns);
    println!("  Throughput: {:.2} TPS", tps3);
    println!();

    // ===== 模式 4: 多线程并发批量 + 细粒度锁 =====
    println!("╔════════════════════════════════════════════════════════════════════════╗");
    println!("║  Mode 4: Multi-Threaded + Fine-Grained Lock Batching                  ║");
    println!("╚════════════════════════════════════════════════════════════════════════╝");

    let store4 = Arc::new(MvccStore::new());
    let coord4 = TwoPhaseCoordinator::new(Arc::clone(&store4));
    let lock_batch_size = 32; // 每批锁定32个键

    let start4 = Instant::now();
    let success4 = Arc::new(Mutex::new(0));

    pool.install(|| {
        let txns_per_thread = total_txns / num_threads;
        
        (0..num_threads).into_par_iter().for_each(|thread_id| {
            let start_idx = thread_id * txns_per_thread;
            let end_idx = if thread_id == num_threads - 1 {
                total_txns
            } else {
                (thread_id + 1) * txns_per_thread
            };

            let mut local_success = 0;
            let mut batch_txns = Vec::new();

            for i in start_idx..end_idx {
                let mut txn = store4.begin();
                let key = format!("key_{}", i % 1000);
                let value = format!("value_{}", i);
                txn.write(key.as_bytes().to_vec(), value.as_bytes().to_vec());
                batch_txns.push(txn);

                if batch_txns.len() >= batch_size || i == end_idx - 1 {
                    let current_batch = std::mem::take(&mut batch_txns);
                    match coord4.batch_prepare_fine_grained(current_batch, lock_batch_size) {
                        Ok(prepared) => {
                            local_success += coord4.pipeline_commit(prepared);
                        }
                        Err((idx, err)) => {
                            eprintln!("Thread {} fine-grained batch failed at index {}: {}", thread_id, idx, err);
                        }
                    }
                }
            }

            *success4.lock().unwrap() += local_success;
        });
    });

    let elapsed4 = start4.elapsed();
    let final_success4 = *success4.lock().unwrap();
    let tps4 = total_txns as f64 / elapsed4.as_secs_f64();

    println!("  Time Elapsed: {:.3}s", elapsed4.as_secs_f64());
    println!("  Successful Txns: {}/{}", final_success4, total_txns);
    println!("  Throughput: {:.2} TPS", tps4);
    println!("  Lock Batch Size: {} keys", lock_batch_size);
    println!();

    // ===== 性能对比总结 =====
    println!("╔════════════════════════════════════════════════════════════════════════╗");
    println!("║                     Performance Comparison                             ║");
    println!("╚════════════════════════════════════════════════════════════════════════╝");

    let speedup_single_batch = (tps2 / tps1 - 1.0) * 100.0;
    let speedup_concurrent = (tps3 / tps1 - 1.0) * 100.0;
    let speedup_concurrent_vs_batch = (tps3 / tps2 - 1.0) * 100.0;
    let speedup_fine_grained = (tps4 / tps1 - 1.0) * 100.0;
    let speedup_fine_vs_coarse = (tps4 / tps3 - 1.0) * 100.0;

    println!();
    println!("┌─────────────────────────────────────────────────────────────────────────────┐");
    println!("│ Mode                      │ Throughput (TPS) │ vs Mode 1 │ vs Mode 3 │");
    println!("├─────────────────────────────────────────────────────────────────────────────┤");
    println!("│ 1. Single Original        │ {:>15.2} │    0.0%   │     -     │", tps1);
    println!("│ 2. Single Batch           │ {:>15.2} │ {:>+7.1}% │     -     │", tps2, speedup_single_batch);
    println!("│ 3. Concurrent Batch ({}T) │ {:>15.2} │ {:>+7.1}% │    0.0%   │", 
        num_threads, tps3, speedup_concurrent);
    println!("│ 4. Concurrent + Fine Lock │ {:>15.2} │ {:>+7.1}% │ {:>+7.1}% │",
        tps4, speedup_fine_grained, speedup_fine_vs_coarse);
    println!("└─────────────────────────────────────────────────────────────────────────────┘");
    println!();

    // 输出批量指标
    if let Some(metrics) = store3.get_metrics() {
        let prom_output = metrics.export_prometheus();
        let batch_metrics: Vec<&str> = prom_output.lines()
            .filter(|line| line.contains("batch") || line.contains("pipeline"))
            .collect();
        
        if !batch_metrics.is_empty() {
            println!("╔════════════════════════════════════════════════════════════════════════╗");
            println!("║                     Batch & Pipeline Metrics                           ║");
            println!("╚════════════════════════════════════════════════════════════════════════╝");
            for line in batch_metrics {
                println!("  {}", line);
            }
            println!();
        }
    }

    println!("✅ Benchmark Complete!");
    println!();
    println!("Key Findings:");
    if speedup_concurrent > 0.0 {
        println!("  🚀 Concurrent batch is {:.1}% FASTER than single-threaded original", speedup_concurrent);
    } else {
        println!("  ⚠️  Single-threaded original is {:.1}% faster than concurrent batch", -speedup_concurrent);
    }
    
    if speedup_fine_vs_coarse > 0.0 {
        println!("  💡 Fine-grained locking provides {:.1}% speedup over coarse-grained", speedup_fine_vs_coarse);
    } else {
        println!("  ⚠️  Coarse-grained is {:.1}% faster (may need higher contention)", -speedup_fine_vs_coarse);
    }
    
    println!("  📊 Threads: {}, Batch Size: {}, Lock Batch: {} keys", num_threads, batch_size, lock_batch_size);
    println!();
}
