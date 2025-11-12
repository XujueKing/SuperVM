//! Example: Hybrid scheduler demonstrating CPU-GPU fallback and adaptive routing.
//! Run with: cargo run -p gpu-executor --example hybrid_scheduler_demo --features gpu

#[cfg(feature = "gpu")]
fn main() {
    use gpu_executor::*;

    println!("=== Hybrid Scheduler Demo ===\n");

    // Initialize executors
    let cpu = CpuMapExecutor::new(|(a, b): &(Vec<f32>, Vec<f32>)| {
        a.iter().zip(b.iter()).map(|(x, y)| x + y).collect()
    });

    let gpu = create_wgpu_executor().ok();
    if gpu.is_none() {
        println!("⚠️  GPU not available, will use CPU fallback only.\n");
    } else {
        println!("✓ GPU available\n");
    }

    // Strategy: prefer GPU for batches >= 10 tasks
    let strategy = HybridStrategy {
        gpu_threshold: 10,
        max_cpu_parallelism: 4,
    };

    let mut scheduler = HybridScheduler::new(cpu, gpu, strategy);

    // Test case 1: Small batch (< threshold) -> CPU
    println!("[Test 1] Small batch (5 tasks) - expect CPU");
    let small_batch = Batch {
        tasks: (0..5)
            .map(|i| Task {
                id: i,
                payload: (vec![i as f32; 100], vec![(i + 1) as f32; 100]),
                est_cost: 100,
            })
            .collect(),
    };
    let (_, stats) = scheduler.schedule(&small_batch).expect("failed");
    println!("  → Device: {:?}, Duration: {:?}\n", stats.device, stats.duration);

    // Test case 2: Large batch (>= threshold) -> GPU (if available)
    println!("[Test 2] Large batch (20 tasks) - prefer GPU");
    let large_batch = Batch {
        tasks: (0..20)
            .map(|i| Task {
                id: 100 + i,
                payload: (vec![i as f32; 1000], vec![(i * 2) as f32; 1000]),
                est_cost: 1000,
            })
            .collect(),
    };
    let (_, stats) = scheduler.schedule(&large_batch).expect("failed");
    println!("  → Device: {:?}, Duration: {:?}\n", stats.device, stats.duration);

    println!("✅ Hybrid scheduler demo complete!");
}

#[cfg(not(feature = "gpu"))]
fn main() {
    eprintln!("[hybrid_scheduler_demo] feature 'gpu' 未启用。请使用: cargo run --example hybrid_scheduler_demo --features gpu");
}
