//! Integration test for GPU backend with wgpu.
//! Only runs when `gpu` feature is enabled.

#![cfg(feature = "gpu")]

use gpu_executor::*;

#[test]
fn wgpu_vector_add_smoke() {
    let mut executor = create_wgpu_executor().expect("GPU init failed");

    let batch = Batch {
        tasks: vec![
            Task {
                id: 1,
                payload: (vec![1.0, 2.0, 3.0], vec![4.0, 5.0, 6.0]),
                est_cost: 1,
            },
            Task {
                id: 2,
                payload: (vec![10.0, 20.0], vec![5.0, 15.0]),
                est_cost: 1,
            },
        ],
    };

    let (results, stats) = executor.execute(&batch).expect("execute failed");

    assert_eq!(results.len(), 2);
    assert_eq!(results[0].id, 1);
    assert_eq!(results[0].output, vec![5.0, 7.0, 9.0]);
    assert_eq!(results[1].output, vec![15.0, 35.0]);
    assert_eq!(stats.device, DeviceKind::Gpu);
    assert!(stats.duration.as_millis() < 5000); // reasonable upper bound
}

#[test]
fn hybrid_scheduler_with_gpu() {
    let cpu = CpuMapExecutor::new(|(a, b): &(Vec<f32>, Vec<f32>)| {
        a.iter().zip(b.iter()).map(|(x, y)| x + y).collect::<Vec<f32>>()
    });

    let gpu = create_wgpu_executor().ok(); // may fail if no GPU
    let strategy = HybridStrategy {
        gpu_threshold: 1, // prefer GPU even for small batches
        max_cpu_parallelism: 1,
    };

    let mut scheduler = HybridScheduler::new(cpu, gpu, strategy);

    let batch = Batch {
        tasks: vec![Task {
            id: 42,
            payload: (vec![1.0, 2.0, 3.0], vec![10.0, 20.0, 30.0]),
            est_cost: 1,
        }],
    };

    let (results, stats) = scheduler.schedule(&batch).expect("schedule failed");
    assert_eq!(results[0].output, vec![11.0, 22.0, 33.0]);
    // Device could be CPU or GPU depending on availability
    println!("Executed on: {:?}", stats.device);
}
