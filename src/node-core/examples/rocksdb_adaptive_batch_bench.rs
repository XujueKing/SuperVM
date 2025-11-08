// SPDX-License-Identifier: GPL-3.0-or-later
// 自适应分批基准示例（支持环境变量配置与 CSV 导出）

use anyhow::Result;
use std::env;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::time::Instant;
use vm_runtime::{AdaptiveBatchConfig, RocksDBStorage};

fn gen_batch(n: usize) -> Vec<(Vec<u8>, Option<Vec<u8>>)> {
    let mut batch = Vec::with_capacity(n);
    for i in 0..n {
        let k = format!("ab_key_{}", i).into_bytes();
        let v = vec![0u8; 256];
        batch.push((k, Some(v)));
    }
    batch
}

fn parse_env_usize(key: &str, default: usize) -> usize {
    env::var(key)
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(default)
}

fn parse_env_f64(key: &str, default: f64) -> f64 {
    env::var(key)
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(default)
}

fn main() -> Result<()> {
    let path = "./data/adaptive_bench";
    let _ = fs::remove_dir_all(path);

    // 使用独立路径，避免历史数据影响 compaction/flush 行为
    let storage = RocksDBStorage::new_with_path(path)?;

    // 从环境变量读取配置（可选覆盖默认值）
    let init_chunk_override = parse_env_usize("ADAPT_INIT_CHUNK", 0);
    let min_chunk = parse_env_usize("ADAPT_MIN_CHUNK", 1_000);
    let max_chunk = parse_env_usize("ADAPT_MAX_CHUNK", 60_000);
    let target_rsd = parse_env_f64("ADAPT_TARGET_RSD", 8.0);
    let adjust_up = parse_env_f64("ADAPT_UP_PCT", 0.15);
    let adjust_down = parse_env_f64("ADAPT_DOWN_PCT", 0.30);
    let window = parse_env_usize("ADAPT_WINDOW", 6);
    let csv_path =
        env::var("ADAPT_CSV").unwrap_or_else(|_| "adaptive_bench_results.csv".to_string());

    let sizes = [10_000usize, 50_000, 100_000];

    println!("=== 自适应分批基准 ===");
    println!("配置: min_chunk={}, max_chunk={}, target_rsd={:.1}%, adjust_up={:.2}, adjust_down={:.2}, window={}",
        min_chunk, max_chunk, target_rsd, adjust_up, adjust_down, window);

    // 准备 CSV 文件（追加模式，首次写入表头）
    let csv_exists = std::path::Path::new(&csv_path).exists();
    let mut csv_file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(&csv_path)?;
    if !csv_exists {
        writeln!(csv_file, "timestamp,batch_size,total_qps,chunks,final_chunk,window_avg_qps,rsd_pct,best_qps,min_chunk,max_chunk,target_rsd,adjust_up,adjust_down,window")?;
    }

    for &n in &sizes {
        let batch = gen_batch(n);
        let start = Instant::now();

        // 构建自定义配置（若有环境变量覆盖则使用，否则用默认）
        let mut cfg = AdaptiveBatchConfig::default_for(&batch);
        if init_chunk_override > 0 {
            cfg.init_chunk = init_chunk_override;
        }
        cfg.min_chunk = min_chunk;
        cfg.max_chunk = max_chunk;
        cfg.target_rsd_pct = target_rsd;
        cfg.adjust_up_pct = adjust_up;
        cfg.adjust_down_pct = adjust_down;
        cfg.window = window;

        let res = storage.write_batch_adaptive_with_config(batch, cfg, true, false)?; // disable_wal=true
        let dur = start.elapsed();
        let qps_total = n as f64 / dur.as_secs_f64();

        println!(
            "N={:>6} | total_qps={:>10.2} | chunks={:>2} final_chunk={:>5} | window_avg_qps={:>10.2} (RSD={:>5.2}%) BEST={:>10.2}",
            n, qps_total, res.chunks, res.final_chunk, res.avg_qps, res.rsd_pct, res.best_qps
        );

        // 写入 CSV
        let timestamp = chrono::Utc::now().format("%Y-%m-%d %H:%M:%S").to_string();
        writeln!(
            csv_file,
            "{},{},{:.2},{},{},{:.2},{:.2},{:.2},{},{},{:.2},{:.2},{:.2},{}",
            timestamp,
            n,
            qps_total,
            res.chunks,
            res.final_chunk,
            res.avg_qps,
            res.rsd_pct,
            res.best_qps,
            min_chunk,
            max_chunk,
            target_rsd,
            adjust_up,
            adjust_down,
            window
        )?;
    }

    println!("\n✓ 结果已追加到: {}", csv_path);
    Ok(())
}
