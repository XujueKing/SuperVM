//! Strategy decisions demo: observe routing and adaptive threshold changes.
//! Run with CPU-only + parallel: 
//! cargo run -p gpu-executor --no-default-features --features cpu,parallel --example strategy_decisions_demo

use gpu_executor::{Batch, Task, CpuMapExecutor, HybridScheduler, HybridStrategy, UnavailableGpu, GpuExecutor};

fn main() {
    // CPU 处理函数：模拟中等开销，避免超轻任务导致并行不划算。
    fn work(x: &u64) -> u64 {
        // 简单忙等，模拟计算负载
        let mut v = *x;
        for _ in 0..50 { v = v.wrapping_mul(1664525).wrapping_add(1013904223); }
        v
    }

    let cpu = CpuMapExecutor::new(work);
    let strategy = HybridStrategy { gpu_threshold: 32, max_cpu_parallelism: 8, adaptive_enabled: true, min_gpu_threshold: 8, max_gpu_threshold: 256, adjust_step: 8 };
    let mut sched = HybridScheduler::new(cpu, Some(UnavailableGpu), strategy);

    // 构造不同 cost 的批次：轻/中/重（用 est_cost 代表，让调度器倾向不同）
    let batches: Vec<Batch<u64>> = vec![
        make_batch(16, 1),   // 小批+轻任务 -> CPU
        make_batch(64, 1),   // 大批+轻任务 -> 仍可能 CPU（轻任务阈值提升）
        make_batch(24, 200), // 中批+重任务 -> 倾向 GPU（但这里 GPU 不可用，回退 CPU）
        make_batch(128, 5),  // 大批+中等 -> 可能倾向 GPU（回退 CPU）
    ];

    for (i, b) in batches.iter().enumerate() {
        let target = sched.decide_route(b);
        let (.., stats) = sched.schedule(b).expect("ok");
        let snap = sched.stats_snapshot();
        println!(
            "batch#{i}: size={} est_cost≈{} target={:?} executed={:?} gpu_threshold={} cpu_batches={} gpu_batches={} busy_events={}",
            b.tasks.len(), avg_est_cost(b), target, stats.device, snap.gpu_threshold, snap.recent_cpu_batches, snap.recent_gpu_batches, snap.gpu_busy_events
        );
    }
}

fn make_batch(n: usize, est_cost: u64) -> Batch<u64> {
    Batch { tasks: (0..n as u64).map(|i| Task { id: i, payload: i, est_cost }).collect() }
}

fn avg_est_cost<T>(b: &Batch<T>) -> u64 { let s: u64 = b.tasks.iter().map(|t| t.est_cost).sum(); s / (b.tasks.len() as u64).max(1) }
