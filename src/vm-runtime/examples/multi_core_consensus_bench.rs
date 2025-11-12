// SPDX-License-Identifier: GPL-3.0-or-later
// 简单多核共识路径基准：单键写事务，路由到分区并行提交

#[cfg(feature = "partitioned-fastpath")]
fn main() {
    use std::sync::Arc;
    use std::time::Instant;
    use vm_runtime::{mvcc::MvccStore, multi_core_consensus::MultiCoreConsensus};

    let args: Vec<String> = std::env::args().collect();
    let txs: usize = args.iter().position(|a| a == "--txs").and_then(|i| args.get(i+1)).and_then(|s| s.parse().ok()).unwrap_or(200_000);
    let parts: usize = args.iter().position(|a| a == "--partitions").and_then(|i| args.get(i+1)).and_then(|s| s.parse().ok()).unwrap_or_else(|| std::thread::available_parallelism().map(|n| n.get()).unwrap_or(4));
    let batch: u64 = args.iter().position(|a| a == "--batch").and_then(|i| args.get(i+1)).and_then(|s| s.parse().ok()).unwrap_or(512);

    println!("=== Multi-Core Consensus Bench ===\nTxns={} Partitions={} Batch={} ", txs, parts, batch);

    let store = MvccStore::new();
    let mut mc = MultiCoreConsensus::new(store.clone(), parts, batch);
    mc.start();

    let start = Instant::now();
    for i in 0..txs {
        let mut txn = store.begin();
        // 单键写，值很小，强调提交路径
        let key = format!("k{:08}", i).into_bytes();
        txn.write(key, b"v".to_vec());
        let _ = mc.route_or_commit(txn);
    }
    // 等待异步执行完成
    while mc.executed() < mc.routed() { std::thread::yield_now(); }
    let elapsed = start.elapsed();
    let tps = (txs as f64) / elapsed.as_secs_f64();
    println!("Elapsed: {:?}\nThroughput (TPS): {:.0}", elapsed, tps);
}

#[cfg(not(feature = "partitioned-fastpath"))]
fn main() { println!("partitioned-fastpath feature 未启用，跳过基准测试。"); }
