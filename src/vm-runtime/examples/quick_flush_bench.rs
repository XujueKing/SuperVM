// SPDX-License-Identifier: GPL-3.0-or-later
// Quick benchmark for MVCC flush batch performance

use std::time::Instant;
use std::env;
use vm_runtime::mvcc::MvccStore;

fn main() {
    println!("=== Quick MVCC Flush Batch Benchmark ===\n");
    
    let store = MvccStore::new();
    
    const NUM_KEYS: usize = 1000;
    const NUM_VERSIONS: usize = 5;
    
    // Populate store with test data
    println!("📝 Populating {} keys with {} versions each...", NUM_KEYS, NUM_VERSIONS);
    for key_idx in 0..NUM_KEYS {
        for ver_idx in 0..NUM_VERSIONS {
            let mut txn = store.begin();
            let key = format!("key_{:06}", key_idx).into_bytes();
            let value = format!("value_{}_{}", key_idx, ver_idx).into_bytes();
            txn.write(key, value);
            let _ = txn.commit();
        }
    }
    
    println!("✅ Store populated with {} total versions\n", NUM_KEYS * NUM_VERSIONS);
    
    // Benchmark: Transaction commit performance (configurable via FLUSH_TXNS)
    println!("📊 MVCC Transaction Performance:");
    let total_txns: usize = env::var("FLUSH_TXNS").ok().and_then(|v| v.parse::<usize>().ok()).unwrap_or(1000);
    let start = Instant::now();
    let mut committed = 0;
    for i in 0..total_txns {
        let mut txn = store.begin();
        let key = format!("bench_key_{}", i).into_bytes();
        let value = format!("bench_value_{}", i).into_bytes();
        txn.write(key, value);
        if txn.commit().is_ok() {
            committed += 1;
        }
    }
    let duration = start.elapsed();
    
    println!("  Total transactions: {}", total_txns);
    println!("  Committed: {}", committed);
    println!("  Duration: {:.2?}", duration);
    println!("  Throughput: {:.0} txn/sec", total_txns as f64 / duration.as_secs_f64());
    println!("  Avg latency: {:.2?}", duration / total_txns as u32);
    
    // Get flush stats
    let stats = store.get_flush_stats();
    println!("\n📈 Flush Statistics:");
    println!("  Total flushes: {}", stats.flush_count);
    println!("  Total keys flushed: {}", stats.keys_flushed);
    println!("  Total bytes flushed: {}", stats.bytes_flushed);
    
    println!("\n✅ Benchmark completed successfully");
}
