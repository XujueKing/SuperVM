//! Demonstration of `ParallelCpuExecutor` vs sequential `CpuMapExecutor`.
//! Run with: `cargo run -p gpu-executor --no-default-features --features cpu,parallel --example parallel_cpu_demo`

#[cfg(all(feature = "cpu", feature = "parallel"))]
fn main() {
    use gpu_executor::{Batch, Task, CpuMapExecutor, ParallelCpuExecutor, GpuExecutor};
    use std::time::Instant;

    const N: usize = 50_000;
    let batch = Batch { tasks: (0..N as u64).map(|i| Task { id: i, payload: i, est_cost: 1 }).collect() };

    // sequential
    let mut seq = CpuMapExecutor::new(|x: &u64| x.wrapping_mul(3).wrapping_add(1));
    let t0 = Instant::now();
    let (seq_res, seq_stats) = seq.execute(&batch).expect("seq ok");
    let seq_time = t0.elapsed();

    // parallel
    let mut par = ParallelCpuExecutor::new(|x: &u64| x.wrapping_mul(3).wrapping_add(1));
    let t1 = Instant::now();
    let (par_res, par_stats) = par.execute(&batch).expect("par ok");
    let par_time = t1.elapsed();

    assert_eq!(seq_res.len(), par_res.len());
    for (a, b) in seq_res.iter().zip(par_res.iter()) { assert_eq!(a.output, b.output); }

    println!("Sequential: {:?} (device={:?})", seq_time, seq_stats.device);
    println!("Parallel:   {:?} (device={:?})", par_time, par_stats.device);
    if par_time < seq_time { println!("Speedup: {:.2}x", seq_time.as_secs_f64() / par_time.as_secs_f64()); }
}

#[cfg(not(all(feature = "cpu", feature = "parallel")))]
fn main() {
    eprintln!("Enable features 'cpu,parallel' to run this example.");
}
