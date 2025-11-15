// SPDX-License-Identifier: GPL-3.0-or-later
//! 批量 + 流水线 2PC 性能基准测试
//!
//! 对比三种模式:
//! 1. 原始 prepare_and_commit (单事务)
//! 2. batch_prepare + pipeline_commit (批量处理)
//! 3. 混合模式 (批量大小自适应)

use std::sync::Arc;
use std::time::Instant;
use vm_runtime::{MvccStore, two_phase_consensus::TwoPhaseCoordinator};

fn main() {
    println!("╔════════════════════════════════════════════════════════════════════════╗");
    println!("║        2PC Batch & Pipeline Performance Benchmark                     ║");
    println!("╚════════════════════════════════════════════════════════════════════════╝\n");

    let total_txns = std::env::args()
        .nth(1)
        .and_then(|s| s.parse().ok())
        .unwrap_or(10000);

    let batch_size = std::env::args()
        .nth(2)
        .and_then(|s| s.parse().ok())
        .unwrap_or(50);

    println!("Configuration:");
    println!("  Total Transactions: {}", total_txns);
    println!("  Batch Size: {}", batch_size);
    println!();

    // ===== 模式 1: 原始单事务 prepare_and_commit =====
    println!("╔════════════════════════════════════════════════════════════════════════╗");
    println!("║  Mode 1: Original prepare_and_commit (Single Transaction)             ║");
    println!("╚════════════════════════════════════════════════════════════════════════╝");

    let store1 = Arc::new(MvccStore::new());
    let coord1 = TwoPhaseCoordinator::new(Arc::clone(&store1)); // 使用Arc::clone避免双重包装

    let start1 = Instant::now();
    let mut success1 = 0;

    for i in 0..total_txns {
        let mut txn = store1.begin();
        let key = format!("key_{}", i % 1000); // 1000 个不同的键模拟冲突
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

    // ===== 模式 2: 批量 prepare + 流水线 commit =====
    println!("╔════════════════════════════════════════════════════════════════════════╗");
    println!("║  Mode 2: Batch Prepare + Pipeline Commit                              ║");
    println!("╚════════════════════════════════════════════════════════════════════════╝");

    let store2 = Arc::new(MvccStore::new());
    let coord2 = TwoPhaseCoordinator::new(Arc::clone(&store2)); // 使用Arc::clone避免双重包装

    let start2 = Instant::now();
    let mut success2 = 0;
    let mut batch_txns = Vec::new();

    for i in 0..total_txns {
        let mut txn = store2.begin();
        let key = format!("key_{}", i % 1000);
        let value = format!("value_{}", i);
        txn.write(key.as_bytes().to_vec(), value.as_bytes().to_vec());
        batch_txns.push(txn);

        // 当批次满或最后一批时处理
        if batch_txns.len() >= batch_size || i == total_txns - 1 {
            let current_batch = std::mem::take(&mut batch_txns); // 移动所有权避免clone
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

    // ===== 性能对比总结 =====
    println!("╔════════════════════════════════════════════════════════════════════════╗");
    println!("║                     Performance Comparison                             ║");
    println!("╚════════════════════════════════════════════════════════════════════════╝");

    let speedup = (tps2 / tps1 - 1.0) * 100.0;
    let time_saved = (elapsed1.as_secs_f64() - elapsed2.as_secs_f64()) / elapsed1.as_secs_f64() * 100.0;

    println!();
    println!("┌────────────────────────────────────────────────────────────────────┐");
    println!("│ Metric                    │ Original    │ Batch+Pipeline │ Delta  │");
    println!("├────────────────────────────────────────────────────────────────────┤");
    println!("│ Throughput (TPS)          │ {:>10.2} │ {:>14.2} │ {:>+6.1}% │", 
        tps1, tps2, speedup);
    println!("│ Time Elapsed (s)          │ {:>10.3} │ {:>14.3} │ {:>+6.1}% │", 
        elapsed1.as_secs_f64(), elapsed2.as_secs_f64(), -time_saved);
    println!("│ Avg Latency (ms)          │ {:>10.3} │ {:>14.3} │ {:>+6.1}% │",
        elapsed1.as_secs_f64() * 1000.0 / total_txns as f64,
        elapsed2.as_secs_f64() * 1000.0 / total_txns as f64,
        -speedup);
    println!("└────────────────────────────────────────────────────────────────────┘");
    println!();

    if let Some(metrics) = store2.get_metrics() {
        let prom_output = metrics.export_prometheus();
        let batch_ops: Vec<&str> = prom_output.lines()
            .filter(|line| line.contains("cross_shard_batch") || line.contains("cross_shard_pipeline"))
            .collect();
        
        if !batch_ops.is_empty() {
            println!("╔════════════════════════════════════════════════════════════════════════╗");
            println!("║                     Batch & Pipeline Metrics                           ║");
            println!("╚════════════════════════════════════════════════════════════════════════╝");
            for line in batch_ops {
                println!("  {}", line);
            }
            println!();
        }
    }

    println!("✅ Benchmark Complete!");
    println!();
    println!("Key Findings:");
    if speedup > 0.0 {
        println!("  🚀 Batch + Pipeline is {:.1}% FASTER than original", speedup);
    } else {
        println!("  ⚠️  Original is {:.1}% faster (consider smaller batch size)", -speedup);
    }
    println!("  📊 Time saved: {:.1}%", time_saved);
    println!("  💡 Optimal batch size depends on contention level");
    println!();
}
