// SPDX-License-Identifier: GPL-3.0-or-later
// 批量大小对比基准（无 Criterion 依赖）

use anyhow::Result;
use std::time::Instant;

#[cfg(feature = "rocksdb-storage")]
use vm_runtime::RocksDBStorage;

#[cfg(feature = "rocksdb-storage")]
fn run_case(path: &str, batch_size: usize, disable_wal: bool) -> Result<f64> {
    let mut storage = RocksDBStorage::new_with_path(path)?;

    // 预生成一批数据
    let mut batch = Vec::with_capacity(batch_size);
    for i in 0..batch_size {
        let k = format!("bench_key_{}", i).into_bytes();
        let v = vec![0u8; 256];
        batch.push((k, Some(v)));
    }

    // 写入测量
    let start = Instant::now();
    storage.write_batch_with_options(batch, disable_wal, false)?;
    let dur = start.elapsed();

    let qps = batch_size as f64 / dur.as_secs_f64();
    Ok(qps)
}

fn main() -> Result<()> {
    #[cfg(feature = "rocksdb-storage")]
    {
        println!("=== RocksDB 批量大小基准 ===");
        let sizes = [100usize, 500, 1_000, 5_000, 10_000, 100_000];
        for &disable_wal in &[false, true] {
            println!("\n-- 配置: disable_wal = {} --", disable_wal);
            for &sz in &sizes {
                // 使用独立目录避免互相影响
                let path = format!(
                    "./data/batch_bench_{}_{}",
                    sz,
                    if disable_wal { "nowal" } else { "wal" }
                );
                let qps = run_case(&path, sz, disable_wal)?;
                println!("  批量 {} 条: QPS = {:.2} ops/s", sz, qps);
            }
        }
    }

    #[cfg(not(feature = "rocksdb-storage"))]
    {
        eprintln!("rocksdb-storage feature not enabled");
    }

    Ok(())
}
