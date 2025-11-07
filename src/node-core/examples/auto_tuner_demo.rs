// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2025 XujueKing

//! AutoTuner 演示示例
//!
//! 展示内核如何自动学习并调整性能参数以最大化 TPS。
//!
//! 运行:
//!   cargo run -p node-core --example auto_tuner_demo --release
//!
//! 对比:
//!   - 手动调优 (固定参数)
//!   - 自动调优 (动态学习)

use std::sync::{Arc, Barrier};
use std::thread;
use std::time::Instant;
use vm_runtime::{OptimizedMvccScheduler, OptimizedSchedulerConfig, mvcc::GcConfig};

fn main() {
    println!("=== AutoTuner Demo: Manual vs Auto Tuning ===\n");
    
    // 场景 1: 手动配置 (固定参数)
    println!("--- Scenario 1: Manual Tuning (Fixed Config) ---");
    let mut manual_config = OptimizedSchedulerConfig::default();
    manual_config.enable_auto_tuning = false;
    manual_config.min_batch_size = 10;  // 固定批量
    manual_config.enable_bloom_filter = false;  // 固定关闭
    manual_config.num_shards = 8;       // 固定分片数
    manual_config.mvcc_config = GcConfig {
        max_versions_per_key: 100,
        enable_time_based_gc: false,
        version_ttl_secs: 3600,
        auto_gc: None,
    };
    
    let manual_scheduler = OptimizedMvccScheduler::new_with_config(manual_config);
    let manual_tps = run_workload(&manual_scheduler, 16, 2000);
    println!("Manual TPS: {:.0}\n", manual_tps);
    
    // 场景 2: 自动调优 (动态学习)
    println!("--- Scenario 2: Auto Tuning (Adaptive Learning) ---");
    let mut auto_config = OptimizedSchedulerConfig::default();
    auto_config.enable_auto_tuning = true;   // 启用自动调优
    auto_config.auto_tuning_interval = 5;    // 每 5 批次评估
    auto_config.enable_adaptive_hot_key = true;  // 启用热键自适应
    auto_config.mvcc_config = GcConfig {
        max_versions_per_key: 100,
        enable_time_based_gc: false,
        version_ttl_secs: 3600,
        auto_gc: None,
    };
    
    let auto_scheduler = OptimizedMvccScheduler::new_with_config(auto_config);
    let auto_tps = run_workload(&auto_scheduler, 16, 2000);
    println!("Auto TPS: {:.0}\n", auto_tps);
    
    // 打印 AutoTuner 学到的配置
    if let Some(summary) = auto_scheduler.get_auto_tuner_summary() {
        println!("\n--- AutoTuner Learned Configuration ---");
        summary.print();
    }
    
    // 对比
    println!("\n=== Comparison ===");
    println!("Manual TPS: {:.0}", manual_tps);
    println!("Auto TPS:   {:.0}", auto_tps);
    if manual_tps > 0.0 {
        println!("Improvement: {:+.2}%", (auto_tps / manual_tps - 1.0) * 100.0);
    }
}

fn run_workload(scheduler: &OptimizedMvccScheduler, num_threads: usize, txns_per_thread: usize) -> f64 {
    let barrier = Arc::new(Barrier::new(num_threads));
    let start = Instant::now();
    let total_txns = num_threads * txns_per_thread;
    
    thread::scope(|scope| {
        for t in 0..num_threads {
            let barrier_c = barrier.clone();
            scope.spawn(move || {
                barrier_c.wait();
                
                for i in 0..txns_per_thread {
                    let tx_id = (t * txns_per_thread + i) as u64;
                    // 50% 热键冲突
                    let key = if i % 2 == 0 {
                        format!("hot_key_{}", i % 10)
                    } else {
                        format!("cold_key_{}_{}", t, i)
                    };
                    
                    let _result = scheduler.execute_txn(tx_id, |txn| {
                        txn.write(key.into_bytes(), b"value".to_vec());
                        Ok(1)
                    });
                }
            });
        }
    });
    
    let elapsed = start.elapsed().as_secs_f64();
    (total_txns as f64) / elapsed
}
