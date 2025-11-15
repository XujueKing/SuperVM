//! Example: GPU-accelerated vector addition with performance comparison.
//! Run with: cargo run -p gpu-executor --example vector_add_demo --features gpu

#[cfg(feature = "gpu")]
fn main() {
    use gpu_executor::*;
    use std::time::Instant;

    println!("=== GPU Vector Addition Demo ===\n");

    // Initialize GPU executor
    println!("[1/4] Initializing GPU backend...");
    let mut gpu_exec = match create_wgpu_executor() {
        Ok(e) => e,
        Err(err) => {
            eprintln!("GPU unavailable: {:?}. Exiting.", err);
            return;
        }
    };
    println!("✓ GPU ready\n");

    // Generate test data
    let sizes = vec![1_000, 10_000, 100_000, 1_000_000];

    for &n in &sizes {
        println!("[2/4] Testing vector size: {} elements ({} KB)", n, n * 4 / 1024);

        let a: Vec<f32> = (0..n).map(|i| i as f32).collect();
        let b: Vec<f32> = (0..n).map(|i| (i * 2) as f32).collect();

        // CPU baseline
        let cpu_exec = CpuMapExecutor::new(|(a, b): &(Vec<f32>, Vec<f32>)| {
            a.iter().zip(b.iter()).map(|(x, y)| x + y).collect()
        });
        let batch = Batch {
            tasks: vec![Task {
                id: 1,
                payload: (a.clone(), b.clone()),
                est_cost: n as u64,
            }],
        };

        let mut cpu_exec_mut = cpu_exec;
        let start = Instant::now();
        let (cpu_results, _) = cpu_exec_mut.execute(&batch).expect("CPU failed");
        let cpu_time = start.elapsed();

        // GPU
        let batch_gpu = Batch {
            tasks: vec![Task {
                id: 2,
                payload: (a.clone(), b.clone()),
                est_cost: n as u64,
            }],
        };
        let start = Instant::now();
        let (gpu_results, _) = gpu_exec.execute(&batch_gpu).expect("GPU failed");
        let gpu_time = start.elapsed();

        // Verify correctness
        assert_eq!(cpu_results[0].output.len(), gpu_results[0].output.len());
        let correct = cpu_results[0]
            .output
            .iter()
            .zip(gpu_results[0].output.iter())
            .all(|(c, g)| (c - g).abs() < 1e-5);

        println!(
            "  CPU: {:>8.2} ms | GPU: {:>8.2} ms | Speedup: {:.2}x | Correct: {}",
            cpu_time.as_secs_f64() * 1000.0,
            gpu_time.as_secs_f64() * 1000.0,
            cpu_time.as_secs_f64() / gpu_time.as_secs_f64(),
            if correct { "✓" } else { "✗" }
        );
    }

    println!("\n✅ Demo complete!");
}

#[cfg(not(feature = "gpu"))]
fn main() {
    eprintln!("[vector_add_demo] feature 'gpu' 未启用。请使用: cargo run --example vector_add_demo --features gpu");
}
