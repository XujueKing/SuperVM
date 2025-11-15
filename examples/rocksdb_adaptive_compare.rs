// SPDX-License-Identifier: GPL-3.0-or-later
// RocksDB 批量写入策略对比基准：Monolithic vs Chunked vs Adaptive

use anyhow::Result;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::time::Instant;
use vm_runtime::{AdaptiveBatchConfig, RocksDBStorage};

fn gen_batch(n: usize) -> Vec<(Vec<u8>, Option<Vec<u8>>)> {
    let mut batch = Vec::with_capacity(n);
    for i in 0..n {
        let k = format!("cmp_key_{}", i).into_bytes();
        let v = vec![0u8; 256];
        batch.push((k, Some(v)));
    }
    batch
}

// 统计多次运行的 QPS
fn stats(qps_list: &[f64]) -> (f64, f64, f64, f64) {
    if qps_list.is_empty() {
        return (0.0, 0.0, 0.0, 0.0);
    }
    let n = qps_list.len() as f64;
    let sum: f64 = qps_list.iter().sum();
    let avg = sum / n;
    let best = qps_list.iter().fold(0.0_f64, |m, &v| m.max(v));
    let mut var = 0.0;
    for &v in qps_list {
        var += (v - avg) * (v - avg);
    }
    var /= n.max(1.0);
    let stddev = var.sqrt();
    let rsd = if avg > 0.0 { stddev / avg * 100.0 } else { 0.0 };
    (avg, best, stddev, rsd)
}

fn main() -> Result<()> {
    let path = "./data/adaptive_compare";
    let _ = fs::remove_dir_all(path);

    let storage = RocksDBStorage::new_with_path(path)?;
    let sizes = [10_000usize, 50_000, 100_000];
    let iterations = 3; // 每种策略运行 3 次取平均
    let csv_path = "adaptive_compare_results.csv";

    // 准备 CSV
    let csv_exists = std::path::Path::new(&csv_path).exists();
    let mut csv_file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(&csv_path)?;
    if !csv_exists {
        writeln!(csv_file, "timestamp,batch_size,wal_enabled,strategy,iter,total_qps,avg_qps,best_qps,stddev_qps,rsd_pct,chunks,final_chunk")?;
    }

    println!("=== RocksDB 批量写入策略对比 ===\n");

    for wal_enabled in [false, true] {
        let wal_str = if wal_enabled { "WAL ON " } else { "WAL OFF" };
        println!("┌─────────────────────────────────────────────────────────────────────────────");
        println!("│ {} 配置", wal_str);
        println!("└─────────────────────────────────────────────────────────────────────────────");

        for &n in &sizes {
            println!("\n批量大小: N = {}", n);

            // === 策略 1: Monolithic (单体批量) ===
            let mut mono_qps_list = Vec::new();
            for iter in 0..iterations {
                let batch = gen_batch(n);
                let start = Instant::now();
                storage.write_batch_with_options(batch, !wal_enabled, false)?;
                let dur = start.elapsed();
                let qps = n as f64 / dur.as_secs_f64();
                mono_qps_list.push(qps);

                let timestamp = chrono::Utc::now().format("%Y-%m-%d %H:%M:%S").to_string();
                writeln!(
                    csv_file,
                    "{},{},{},monolithic,{},{:.2},0.0,0.0,0.0,0.0,1,{}",
                    timestamp, n, wal_enabled, iter, qps, n
                )?;
            }
            let (mono_avg, mono_best, mono_std, mono_rsd) = stats(&mono_qps_list);
            println!(
                "  [Monolithic] AVG: {:>10.2} | BEST: {:>10.2} | STD: {:>8.2} | RSD: {:>5.2}%",
                mono_avg, mono_best, mono_std, mono_rsd
            );

            // === 策略 2: Chunked (固定分块 = n/10) ===
            let chunk_size = (n / 10).max(1000);
            let mut chunked_qps_list = Vec::new();
            for iter in 0..iterations {
                let batch = gen_batch(n);
                let start = Instant::now();
                storage.write_batch_chunked(batch, chunk_size, !wal_enabled, false)?;
                let dur = start.elapsed();
                let qps = n as f64 / dur.as_secs_f64();
                chunked_qps_list.push(qps);

                let timestamp = chrono::Utc::now().format("%Y-%m-%d %H:%M:%S").to_string();
                let chunks = (n + chunk_size - 1) / chunk_size;
                writeln!(
                    csv_file,
                    "{},{},{},chunked,{},{:.2},0.0,0.0,0.0,0.0,{},{}",
                    timestamp, n, wal_enabled, iter, qps, chunks, chunk_size
                )?;
            }
            let (chunked_avg, chunked_best, chunked_std, chunked_rsd) = stats(&chunked_qps_list);
            println!("  [Chunked-{}K] AVG: {:>10.2} | BEST: {:>10.2} | STD: {:>8.2} | RSD: {:>5.2}% | Gain: {:>+6.1}%",
                chunk_size / 1000, chunked_avg, chunked_best, chunked_std, chunked_rsd,
                (chunked_avg - mono_avg) / mono_avg * 100.0);

            // === 策略 3: Adaptive (自适应分块) ===
            let mut adaptive_qps_list = Vec::new();
            let mut adaptive_results = Vec::new();
            for iter in 0..iterations {
                let batch = gen_batch(n);
                let cfg = AdaptiveBatchConfig::default_for(&batch);
                let start = Instant::now();
                let res =
                    storage.write_batch_adaptive_with_config(batch, cfg, !wal_enabled, false)?;
                let dur = start.elapsed();
                let qps = n as f64 / dur.as_secs_f64();
                adaptive_qps_list.push(qps);
                adaptive_results.push(res.clone());

                let timestamp = chrono::Utc::now().format("%Y-%m-%d %H:%M:%S").to_string();
                writeln!(
                    csv_file,
                    "{},{},{},adaptive,{},{:.2},{:.2},{:.2},{:.2},{:.2},{},{}",
                    timestamp,
                    n,
                    wal_enabled,
                    iter,
                    qps,
                    res.avg_qps,
                    res.best_qps,
                    res.stddev_qps,
                    res.rsd_pct,
                    res.chunks,
                    res.final_chunk
                )?;
            }
            let (adaptive_avg, adaptive_best, adaptive_std, adaptive_rsd) =
                stats(&adaptive_qps_list);
            let avg_res = &adaptive_results[adaptive_results.len() / 2]; // 取中位数结果
            println!("  [Adaptive]    AVG: {:>10.2} | BEST: {:>10.2} | STD: {:>8.2} | RSD: {:>5.2}% | Gain: {:>+6.1}%",
                adaptive_avg, adaptive_best, adaptive_std, adaptive_rsd,
                (adaptive_avg - mono_avg) / mono_avg * 100.0);
            println!(
                "                ↳ chunks={}, final_chunk={}, window_rsd={:.2}%",
                avg_res.chunks, avg_res.final_chunk, avg_res.rsd_pct
            );

            // 对比总结
            println!("\n  📊 策略对比（相对 Monolithic）:");
            println!(
                "     Chunked:  吞吐 {:>+6.1}% | RSD {:>+6.1}%",
                (chunked_avg - mono_avg) / mono_avg * 100.0,
                chunked_rsd - mono_rsd
            );
            println!(
                "     Adaptive: 吞吐 {:>+6.1}% | RSD {:>+6.1}%",
                (adaptive_avg - mono_avg) / mono_avg * 100.0,
                adaptive_rsd - mono_rsd
            );
        }

        println!();
    }

    println!("✓ 结果已保存到: {}", csv_path);
    Ok(())
}
