// SPDX-License-Identifier: GPL-3.0-or-later
// 长跑基准：持续执行 MVCC 事务，周期输出 TPS/成功率/延迟百分位，并写入 CSV

use std::env;
use std::fs::{create_dir_all, File};
use std::io::Write;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::{Duration, Instant};

use vm_runtime::MvccStore;

// 简易 LCG 伪随机数，避免引入 crates 依赖
struct Lcg(u64);
impl Lcg {
    fn new(seed: u64) -> Self { Self(seed) }
    fn next(&mut self) -> u64 { self.0 = self.0.wrapping_mul(6364136223846793005).wrapping_add(1); self.0 }
}

fn parse_env<T: std::str::FromStr>(key: &str, default: T) -> T {
    env::var(key).ok().and_then(|s| s.parse::<T>().ok()).unwrap_or(default)
}

fn main() {
    let duration_secs: u64 = parse_env("DURATION_SECS", 600); // 默认 10 分钟
    let interval_secs: u64 = parse_env("INTERVAL_SECS", 10);
    let num_threads: usize = parse_env("NUM_THREADS", 8);
    let key_space: u64 = parse_env("KEY_SPACE", 10_000);
    let write_ratio: u64 = parse_env("WRITE_RATIO", 100); // 0..100 全写默认 100
    let out_dir: String = env::var("OUT_DIR").unwrap_or_else(|_| "data/longrun".to_string());
    let out_file: String = env::var("OUT_FILE").unwrap_or_else(|_| {
        let ts = chrono_like_timestamp();
        format!("{}/longrun_{}.csv", out_dir, ts)
    });

    println!("=== MVCC Long-run Benchmark ===");
    println!("Duration: {}s, Interval: {}s, Threads: {}, KeySpace: {}, WriteRatio: {}%", duration_secs, interval_secs, num_threads, key_space, write_ratio);
    println!("Output: {}", out_file);

    // 确保输出目录存在
    if let Some(parent) = std::path::Path::new(&out_file).parent() { let _ = create_dir_all(parent); }

    let store = Arc::new(MvccStore::new());
    let stop = Arc::new(AtomicBool::new(false));

    // 工作线程
    let mut handles = Vec::with_capacity(num_threads);
    for tid in 0..num_threads {
        let store_cl = store.clone();
        let stop_cl = stop.clone();
        handles.push(thread::spawn(move || {
            let mut rnd = Lcg::new((tid as u64 + 1) * 0x9E3779B97F4A7C15);
            while !stop_cl.load(Ordering::Relaxed) {
                // 随机决定读/写；简单场景写为主
                let r = (rnd.next() % 100) as u64;
                let key_id = rnd.next() % key_space;
                let key = format!("key_{key_id}").into_bytes();
                if r < write_ratio {
                    // 写事务
                    let mut tx = store_cl.begin();
                    tx.write(key, format!("val_{}", rnd.next()).into_bytes());
                    let _ = tx.commit();
                } else {
                    // 读事务
                    let mut tx = store_cl.begin_read_only();
                    let _ = tx.read(&key);
                    let _ = tx.commit();
                }
            }
        }));
    }

    // CSV 头
    let mut file = File::create(&out_file).expect("cannot create csv");
    writeln!(file, "timestamp,elapsed_s,tps,tps_window,tps_peak,success_rate,p50_ms,p90_ms,p99_ms,committed,aborted").unwrap();

    // 统计线程（主线程）
    let start = Instant::now();
    let mut next_tick = Instant::now();
    loop {
        if start.elapsed().as_secs() >= duration_secs { break; }
        if next_tick.elapsed().as_secs() >= interval_secs {
            next_tick = Instant::now();
        }
        thread::sleep(Duration::from_secs(1));
        if let Some(m) = store.get_metrics() {
            // 触发窗口刷新
            let tps_window = m.tps_window();
            let tps_peak = m.peak_tps();
            let tps_overall = m.tps();
            let success = m.success_rate();
            let (p50, p90, p99) = m.txn_latency.percentiles();
            let committed = m.txn_committed.load(Ordering::Relaxed);
            let aborted = m.txn_aborted.load(Ordering::Relaxed);

            println!(
                "[{:>5.0}s] TPS(overall): {:>8.0} | TPS(win): {:>8.0} | Peak: {:>8.0} | SR: {:>5.1}% | P50/P90/P99(ms): {:>4.2}/{:>4.2}/{:>4.2} | committed={} aborted={}",
                start.elapsed().as_secs_f64(), tps_overall, tps_window, tps_peak, success, p50, p90, p99, committed, aborted
            );

            let ts = chrono_like_timestamp();
            writeln!(file, "{},{},{:.2},{:.2},{:.2},{:.2},{:.2},{:.2},{:.2},{},{}",
                ts,
                start.elapsed().as_secs(),
                tps_overall,
                tps_window,
                tps_peak,
                success,
                p50, p90, p99,
                committed, aborted
            ).unwrap();
            let _ = file.flush();
        }
    }

    // 停止 & 汇总
    stop.store(true, Ordering::Relaxed);
    for h in handles { let _ = h.join(); }

    if let Some(m) = store.get_metrics() {
        println!("\n=== Summary ===");
        m.print_summary();
    }

    println!("CSV saved: {}", out_file);
}

// 简单时间戳：yyyyMMdd_HHmmss（不依赖 chrono crate）
fn chrono_like_timestamp() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();
    // 仅作为文件名区分；真正格式化需外部工具
    format!("{}", now.as_secs())
}
