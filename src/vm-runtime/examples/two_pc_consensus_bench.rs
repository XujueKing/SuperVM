// SPDX-License-Identifier: GPL-3.0-or-later
//! 2PC Consensus Benchmark: 混合单/多分区事务工作负载测量
//! 用法: cargo run -p vm-runtime --example two_pc_consensus_bench --release --features partitioned-fastpath -- --txs:50000 --partitions:4 --multi_ratio:0.2
#[cfg(feature = "partitioned-fastpath")]
fn main() {
    use std::sync::Arc;
    use vm_runtime::{mvcc::MvccStore, multi_core_consensus::MultiCoreConsensus};
    use std::time::Instant;

    let args: Vec<String> = std::env::args().collect();
    let txs = args.iter().find_map(|a| a.strip_prefix("--txs:").and_then(|v| v.parse::<usize>().ok())).unwrap_or(50000);
    let partitions = args.iter().find_map(|a| a.strip_prefix("--partitions:").and_then(|v| v.parse::<usize>().ok())).unwrap_or(4);
    let multi_ratio = args.iter().find_map(|a| a.strip_prefix("--multi_ratio:").and_then(|v| v.parse::<f64>().ok())).unwrap_or(0.2);
    println!("╭─────────────────────────────────────────────╮");
    println!("│ 2PC Consensus Benchmark (Mixed Workload)   │");
    println!("├─────────────────────────────────────────────┤");
    println!("│ Txs: {:<9} | Partitions: {:<3}           │", txs, partitions);
    println!("│ Multi-Partition Ratio: {:.1}%                │", multi_ratio * 100.0);
    println!("╰─────────────────────────────────────────────╯");

    let store = MvccStore::new(); // MvccStore::new() 返回 Arc<MvccStore>
    let mut mc = MultiCoreConsensus::new(store.clone(), partitions, 512);
    mc.start();

    // 生成混合事务：单分区 (1-ratio)，多分区 (ratio)
    let multi_count = (txs as f64 * multi_ratio).round() as usize;
    let single_count = txs - multi_count;

    let start = Instant::now();
    for i in 0..single_count {
        let mut tx = store.begin();
        let key = format!("sk_{}", i).into_bytes();
        tx.write(key, b"v".to_vec());
        let _ = mc.route_or_commit(tx);
    }
    for i in 0..multi_count {
        let mut tx = store.begin();
        // 构造跨多个分区的写集合：简单策略为递增 key 直至分区集不同
        let k1 = format!("mk1_{}", i).into_bytes();
        let mut k2 = format!("mk2_{}", i).into_bytes();
        loop {
            let mut probe = store.begin();
            probe.write(k1.clone(), b"v1".to_vec());
            probe.write(k2.clone(), b"v2".to_vec());
            if probe.partition_set(partitions).len() >= 2 { break; }
            k2.push(0xFF);
        }
        tx.write(k1, b"v1".to_vec());
        tx.write(k2, b"v2".to_vec());
        let _ = mc.route_or_commit(tx);
    }
    let elapsed = start.elapsed().as_secs_f64();
    let tps = txs as f64 / elapsed;

    // 等待异步 worker 处理完队列（简化：短暂休眠）
    std::thread::sleep(std::time::Duration::from_millis(500));

    println!("\n╭─────────────────────────────────────────────╮");
    println!("│               Benchmark Results             │");
    println!("├─────────────────────────────────────────────┤");
    println!("│ Total Txs: {:<9} | Time: {:.3}s         │", txs, elapsed);
    println!("│ Throughput: {:.2} TPS                  │", tps);
    println!("│ Routed (Single-Partition): {:<10}       │", mc.routed());
    println!("│ Fallback (Read-Only/Multi-Partition): {:<5}│", mc.fallback_count());
    println!("│ Executed (Committed Workers): {:<10}     │", mc.executed());
    println!("╰─────────────────────────────────────────────╯");

    mc.stop();
}

#[cfg(not(feature = "partitioned-fastpath"))]
fn main() { println!("partitioned-fastpath feature 未启用，跳过基准测试。"); }
