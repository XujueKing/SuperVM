// SPDX-License-Identifier: GPL-3.0-or-later
// PartitionedFastPath Benchmark (prototype)
// Usage:
//   cargo run -p vm-runtime --example partitioned_fast_path_bench --release --features partitioned-fastpath -- --txs:200000 --partitions:4 --cycles:32
// Env overrides: PART_TXS, PARTITIONS, SIM_CYCLES
// Outputs synthetic TPS (simulated cycles only).

#[cfg(feature = "partitioned-fastpath")]
fn main() {
    use std::env;
    use std::time::Instant;
    use vm_runtime::partitioned_fastpath::{PartitionedFastPath, FastTask};

    // Parse args of form --txs:NUM --partitions:NUM --cycles:NUM
    let mut arg_txs: Option<usize> = None;
    let mut arg_parts: Option<usize> = None;
    let mut arg_cycles: Option<u32> = None;
    for a in std::env::args().skip(1) {
        if let Some(v) = a.strip_prefix("--txs:") { arg_txs = v.parse().ok(); }
        else if let Some(v) = a.strip_prefix("--partitions:") { arg_parts = v.parse().ok(); }
        else if let Some(v) = a.strip_prefix("--cycles:") { arg_cycles = v.parse().ok(); }
    }

    let txs: usize = arg_txs
        .or_else(|| env::var("PART_TXS").ok().and_then(|v| v.parse().ok()))
        .unwrap_or(200_000);
    let partitions: usize = arg_parts
        .or_else(|| env::var("PARTITIONS").ok().and_then(|v| v.parse().ok()))
        .unwrap_or(rayon::current_num_threads());
    let cycles: u32 = arg_cycles
        .or_else(|| env::var("SIM_CYCLES").ok().and_then(|v| v.parse().ok()))
        .unwrap_or(32);

    println!("=== PartitionedFastPath Benchmark (Prototype) ===");
    println!("Config: txs={} partitions={} cycles={}", txs, partitions, cycles);

    let mut exec = PartitionedFastPath::new(partitions);
    exec.spawn_workers();
    let start = Instant::now();
    for i in 0..txs as u64 { exec.submit(FastTask::new(i.to_be_bytes().to_vec(), cycles)); }
    // Wait until executed == submitted
    while exec.total_executed() < exec.submitted() { std::thread::yield_now(); }
    let elapsed = start.elapsed();
    let total = exec.total_executed();
    let tps = total as f64 / elapsed.as_secs_f64();
    println!("Executed={} Elapsed={:.2?} TPS≈{:.0}", total, elapsed, tps);
    println!("Per-Partition {:?}", exec.executed_per_partition());
    exec.stop();
}

#[cfg(not(feature = "partitioned-fastpath"))]
fn main() { println!("partitioned-fastpath feature 未启用，跳过基准测试。"); }
