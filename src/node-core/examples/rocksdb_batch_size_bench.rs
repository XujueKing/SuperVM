// SPDX-License-Identifier: GPL-3.0-or-later
// 批量大小对比基准（无 Criterion 依赖）

use anyhow::Result;
use std::fs;
use std::time::Instant;
use vm_runtime::{RocksDBConfig, RocksDBStorage};

fn stats(vals: &[f64]) -> (f64, f64, f64, f64) {
    // returns (avg, best, stddev, rsd_percent)
    if vals.is_empty() {
        return (0.0, 0.0, 0.0, 0.0);
    }
    let sum: f64 = vals.iter().copied().sum();
    let n = vals.len() as f64;
    let avg = sum / n;
    let best = vals.iter().copied().fold(0.0_f64, f64::max);
    let mut var = 0.0;
    for &v in vals {
        var += (v - avg) * (v - avg);
    }
    var /= n.max(1.0);
    let stddev = var.sqrt();
    let rsd = if avg > 0.0 { stddev / avg * 100.0 } else { 0.0 };
    (avg, best, stddev, rsd)
}
use std::fs;

fn run_case_mono(path: &str, batch_size: usize, disable_wal: bool) -> Result<f64> {
    // 清理旧目录，确保每次运行环境一致
    let _ = fs::remove_dir_all(path);
    let cfg = RocksDBConfig::production_optimized().with_path(path.to_string());
    let storage = RocksDBStorage::new(cfg)?;

    // 预生成一批数据
    let mut batch = Vec::with_capacity(batch_size);
    for i in 0..batch_size {
        let k = format!("bench_key_{}", i).into_bytes();
        let v = vec![0u8; 256];
        batch.push((k, Some(v)));
    }

    // 写入测量（整批写）
    let start = Instant::now();
    storage.write_batch_with_options(batch, disable_wal, false)?;
    let dur = start.elapsed();

    let qps = batch_size as f64 / dur.as_secs_f64();
    Ok(qps)
}

fn run_case_chunked(
    path: &str,
    batch_size: usize,
    chunk_size: usize,
    disable_wal: bool,
) -> Result<f64> {
    // 清理旧目录，确保每次运行环境一致
    let _ = fs::remove_dir_all(path);
    let cfg = RocksDBConfig::production_optimized().with_path(path.to_string());
    let storage = RocksDBStorage::new(cfg)?;

    // 预生成一批数据（整体生成，内部按 chunk 写）
    let mut batch = Vec::with_capacity(batch_size);
    for i in 0..batch_size {
        let k = format!("bench_key_{}", i).into_bytes();
        let v = vec![0u8; 256];
        batch.push((k, Some(v)));
    }

    // 写入测量（分批写）
    let start = Instant::now();
    storage.write_batch_chunked(batch, chunk_size.max(1), disable_wal, false)?;
    let dur = start.elapsed();

    let qps = batch_size as f64 / dur.as_secs_f64();
    Ok(qps)
}

fn main() -> Result<()> {
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
            // 多次运行取统计（整批）
            let mut mono = Vec::new();
            for _ in 0..5 {
                mono.push(run_case_mono(&path, sz, disable_wal)?);
            }
            let (avg_m, best_m, std_m, rsd_m) = stats(&mono);

            // 分批：chunk_size = sz/10（下限 1000）
            let chunk = (sz / 10).max(1000);
            let mut ch = Vec::new();
            for _ in 0..5 {
                ch.push(run_case_chunked(&path, sz, chunk, disable_wal)?);
            }
            let (avg_c, best_c, std_c, rsd_c) = stats(&ch);

            let gain_best = if best_m > 0.0 {
                (best_c / best_m - 1.0) * 100.0
            } else {
                0.0
            };
            let gain_avg = if avg_m > 0.0 {
                (avg_c / avg_m - 1.0) * 100.0
            } else {
                0.0
            };

            println!(
                "  批量 {:>6} 条 | 整批: AVG={:>9.2} (RSD={:>5.1}%) BEST={:>9.2} | 分批({:>5}) : AVG={:>9.2} (RSD={:>5.1}%) BEST={:>9.2} | GAIN AVG={:>6.2}% BEST={:>6.2}%",
                sz, avg_m, rsd_m, best_m, chunk, avg_c, rsd_c, best_c, gain_avg, gain_best
            );
        }
    }
    Ok(())
}
