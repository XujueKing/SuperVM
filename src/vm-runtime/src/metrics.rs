// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2025 XujueKing <leadbrand@me.com>

//! 性能指标收集器
//!
//! 为 MVCC Store 和 RocksDB 存储提供轻量级的性能监控
//! 支持导出为 Prometheus 格式的指标

use parking_lot::Mutex;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

/// 延迟直方图 - 记录操作延迟分布
#[derive(Debug)]
pub struct LatencyHistogram {
    buckets: Vec<(f64, AtomicU64)>, // (上限ms, 计数)
    total_count: AtomicU64,
    total_sum_ms: AtomicU64, // 总延迟(ms * 1000 避免浮点)
}

impl Clone for LatencyHistogram {
    fn clone(&self) -> Self {
        let buckets = self
            .buckets
            .iter()
            .map(|(upper, count)| (*upper, AtomicU64::new(count.load(Ordering::Relaxed))))
            .collect();

        Self {
            buckets,
            total_count: AtomicU64::new(self.total_count.load(Ordering::Relaxed)),
            total_sum_ms: AtomicU64::new(self.total_sum_ms.load(Ordering::Relaxed)),
        }
    }
}

impl LatencyHistogram {
    pub fn new() -> Self {
        // 定义延迟桶: <1ms, <5ms, <10ms, <50ms, <100ms, <500ms, <1s, >1s
        let buckets = vec![
            (1.0, AtomicU64::new(0)),
            (5.0, AtomicU64::new(0)),
            (10.0, AtomicU64::new(0)),
            (50.0, AtomicU64::new(0)),
            (100.0, AtomicU64::new(0)),
            (500.0, AtomicU64::new(0)),
            (1000.0, AtomicU64::new(0)),
            (f64::INFINITY, AtomicU64::new(0)),
        ];

        Self {
            buckets,
            total_count: AtomicU64::new(0),
            total_sum_ms: AtomicU64::new(0),
        }
    }

    /// 记录一次操作的延迟
    pub fn observe(&self, duration: Duration) {
        let ms = duration.as_secs_f64() * 1000.0;

        // 找到对应的桶
        for (upper, count) in &self.buckets {
            if ms <= *upper {
                count.fetch_add(1, Ordering::Relaxed);
                break;
            }
        }

        self.total_count.fetch_add(1, Ordering::Relaxed);
        self.total_sum_ms
            .fetch_add((ms * 1000.0) as u64, Ordering::Relaxed);
    }

    /// 计算 P50/P90/P99 百分位延迟
    pub fn percentiles(&self) -> (f64, f64, f64) {
        let total = self.total_count.load(Ordering::Relaxed);
        if total == 0 {
            return (0.0, 0.0, 0.0);
        }

        let p50_target = total / 2;
        let p90_target = (total * 90) / 100;
        let p99_target = (total * 99) / 100;

        let mut cumulative = 0u64;
        let mut p50 = 0.0;
        let mut p90 = 0.0;
        let mut p99 = 0.0;

        for (upper, count) in &self.buckets {
            cumulative += count.load(Ordering::Relaxed);

            if p50 == 0.0 && cumulative >= p50_target {
                p50 = *upper;
            }
            if p90 == 0.0 && cumulative >= p90_target {
                p90 = *upper;
            }
            if p99 == 0.0 && cumulative >= p99_target {
                p99 = *upper;
            }
        }

        (p50, p90, p99)
    }

    /// 获取平均延迟
    pub fn avg(&self) -> f64 {
        let total = self.total_count.load(Ordering::Relaxed);
        if total == 0 {
            return 0.0;
        }

        let sum = self.total_sum_ms.load(Ordering::Relaxed);
        (sum as f64) / (total as f64 * 1000.0)
    }
}

/// 指标收集器
pub struct MetricsCollector {
    // MVCC 事务指标
    pub txn_started: AtomicU64,
    pub txn_committed: AtomicU64,
    pub txn_aborted: AtomicU64,
    pub txn_latency: LatencyHistogram,

    // 读写操作指标
    pub reads: AtomicU64,
    pub writes: AtomicU64,
    pub read_latency: LatencyHistogram,
    pub write_latency: AtomicU64, // 简化版，仅记录总延迟

    // GC 指标
    pub gc_runs: AtomicU64,
    pub gc_versions_cleaned: AtomicU64,
    pub gc_duration_ms: AtomicU64,

    // 刷新指标
    pub flush_runs: AtomicU64,
    pub flush_keys: AtomicU64,
    pub flush_bytes: AtomicU64,

    // RocksDB 指标
    pub rocksdb_gets: AtomicU64,
    pub rocksdb_puts: AtomicU64,
    pub rocksdb_deletes: AtomicU64,

    // 并行 ZK 证明指标
    pub parallel_proof_total: AtomicU64,
    pub parallel_proof_failed: AtomicU64,
    pub parallel_proof_batches: AtomicU64,
    pub parallel_last_batch_latency_ms: AtomicU64,       // ms * 1000
    pub parallel_last_batch_avg_latency_ms: AtomicU64,   // ms * 1000
    pub parallel_last_batch_tps: AtomicU64,              // tps * 1000

    // 时间窗口统计 (用于计算窗口 TPS 及峰值)
    window_stats: Arc<Mutex<WindowStats>>,
}

#[derive(Debug)]
struct WindowStats {
    // 全局起始时间，用于计算总体 TPS
    start_time: Instant,
    // 最近一次窗口起点
    last_reset: Instant,
    // 窗口时长（秒）
    window_secs: u64,
    // 最近窗口起点时的提交计数快照
    committed_at_last_reset: u64,
    // 最近一个完整窗口计算得到的 TPS
    last_window_tps: f64,
    // 观测到的峰值 TPS（基于窗口）
    peak_tps: f64,
}

impl Default for MetricsCollector {
    fn default() -> Self {
        Self::new()
    }
}

impl MetricsCollector {
    pub fn new() -> Self {
        let now = Instant::now();
        Self {
            txn_started: AtomicU64::new(0),
            txn_committed: AtomicU64::new(0),
            txn_aborted: AtomicU64::new(0),
            txn_latency: LatencyHistogram::new(),

            reads: AtomicU64::new(0),
            writes: AtomicU64::new(0),
            read_latency: LatencyHistogram::new(),
            write_latency: AtomicU64::new(0),

            gc_runs: AtomicU64::new(0),
            gc_versions_cleaned: AtomicU64::new(0),
            gc_duration_ms: AtomicU64::new(0),

            flush_runs: AtomicU64::new(0),
            flush_keys: AtomicU64::new(0),
            flush_bytes: AtomicU64::new(0),

            rocksdb_gets: AtomicU64::new(0),
            rocksdb_puts: AtomicU64::new(0),
            rocksdb_deletes: AtomicU64::new(0),

            parallel_proof_total: AtomicU64::new(0),
            parallel_proof_failed: AtomicU64::new(0),
            parallel_proof_batches: AtomicU64::new(0),
            parallel_last_batch_latency_ms: AtomicU64::new(0),
            parallel_last_batch_avg_latency_ms: AtomicU64::new(0),
            parallel_last_batch_tps: AtomicU64::new(0),

            window_stats: Arc::new(Mutex::new(WindowStats {
                start_time: now,
                last_reset: now,
                window_secs: 1,
                committed_at_last_reset: 0,
                last_window_tps: 0.0,
                peak_tps: 0.0,
            })),
        }
    }

    /// 刷新时间窗口（当窗口到期时计算窗口 TPS，并更新峰值）
    fn update_window(&self) {
        let committed = self.txn_committed.load(Ordering::Relaxed);
        let mut stats = self.window_stats.lock();
        let elapsed = stats.last_reset.elapsed();
        if elapsed.as_secs() >= stats.window_secs {
            let secs = elapsed.as_secs_f64().max(1.0);
            let committed_delta = committed.saturating_sub(stats.committed_at_last_reset) as f64;
            let window_tps = committed_delta / secs;
            stats.last_window_tps = window_tps;
            if window_tps > stats.peak_tps {
                stats.peak_tps = window_tps;
            }
            // 轮转窗口
            stats.last_reset = Instant::now();
            stats.committed_at_last_reset = committed;
        }
    }

    /// 计算 TPS (每秒事务数)
    pub fn tps(&self) -> f64 {
        let stats = self.window_stats.lock();
        let elapsed = stats.start_time.elapsed().as_secs_f64();
        if elapsed == 0.0 {
            return 0.0;
        }

        let committed = self.txn_committed.load(Ordering::Relaxed);
        committed as f64 / elapsed
    }

    /// 当前窗口 TPS（滚动计算，每 window_secs 秒刷新）
    pub fn tps_window(&self) -> f64 {
        self.update_window();
        self.window_stats.lock().last_window_tps
    }

    /// 峰值 TPS（窗口口径）
    pub fn peak_tps(&self) -> f64 {
        self.update_window();
        self.window_stats.lock().peak_tps
    }

    /// 获取成功率
    pub fn success_rate(&self) -> f64 {
        let committed = self.txn_committed.load(Ordering::Relaxed);
        let aborted = self.txn_aborted.load(Ordering::Relaxed);
        let total = committed + aborted;

        if total == 0 {
            return 100.0;
        }

        (committed as f64 / total as f64) * 100.0
    }

    /// 导出为 Prometheus 格式
    pub fn export_prometheus(&self) -> String {
        let mut output = String::new();

        // MVCC 事务指标
        output.push_str("# HELP mvcc_txn_started_total Total number of transactions started\n");
        output.push_str("# TYPE mvcc_txn_started_total counter\n");
        output.push_str(&format!(
            "mvcc_txn_started_total {}\n",
            self.txn_started.load(Ordering::Relaxed)
        ));

        output.push_str("# HELP mvcc_txn_committed_total Total number of transactions committed\n");
        output.push_str("# TYPE mvcc_txn_committed_total counter\n");
        output.push_str(&format!(
            "mvcc_txn_committed_total {}\n",
            self.txn_committed.load(Ordering::Relaxed)
        ));

        output.push_str("# HELP mvcc_txn_aborted_total Total number of transactions aborted\n");
        output.push_str("# TYPE mvcc_txn_aborted_total counter\n");
        output.push_str(&format!(
            "mvcc_txn_aborted_total {}\n",
            self.txn_aborted.load(Ordering::Relaxed)
        ));

    // TPS（总体）
        output.push_str("# HELP mvcc_tps Current transactions per second\n");
        output.push_str("# TYPE mvcc_tps gauge\n");
        output.push_str(&format!("mvcc_tps {:.2}\n", self.tps()));

    // 窗口 TPS
    output.push_str("# HELP mvcc_tps_window Current window TPS (rolling)\n");
    output.push_str("# TYPE mvcc_tps_window gauge\n");
    output.push_str(&format!("mvcc_tps_window {:.2}\n", self.tps_window()));

    // 峰值 TPS（窗口）
    output.push_str("# HELP mvcc_tps_peak Peak TPS observed over windows\n");
    output.push_str("# TYPE mvcc_tps_peak gauge\n");
    output.push_str(&format!("mvcc_tps_peak {:.2}\n", self.peak_tps()));

        // 成功率
        output.push_str("# HELP mvcc_success_rate Transaction success rate percentage\n");
        output.push_str("# TYPE mvcc_success_rate gauge\n");
        output.push_str(&format!("mvcc_success_rate {:.2}\n", self.success_rate()));

        // 延迟百分位
        let (p50, p90, p99) = self.txn_latency.percentiles();
        output.push_str(
            "# HELP mvcc_txn_latency_ms Transaction latency percentiles in milliseconds\n",
        );
        output.push_str("# TYPE mvcc_txn_latency_ms summary\n");
        output.push_str(&format!(
            "mvcc_txn_latency_ms{{quantile=\"0.5\"}} {:.2}\n",
            p50
        ));
        output.push_str(&format!(
            "mvcc_txn_latency_ms{{quantile=\"0.9\"}} {:.2}\n",
            p90
        ));
        output.push_str(&format!(
            "mvcc_txn_latency_ms{{quantile=\"0.99\"}} {:.2}\n",
            p99
        ));

        // GC 指标
        output.push_str("# HELP mvcc_gc_runs_total Total number of GC runs\n");
        output.push_str("# TYPE mvcc_gc_runs_total counter\n");
        output.push_str(&format!(
            "mvcc_gc_runs_total {}\n",
            self.gc_runs.load(Ordering::Relaxed)
        ));

        output.push_str(
            "# HELP mvcc_gc_versions_cleaned_total Total number of versions cleaned by GC\n",
        );
        output.push_str("# TYPE mvcc_gc_versions_cleaned_total counter\n");
        output.push_str(&format!(
            "mvcc_gc_versions_cleaned_total {}\n",
            self.gc_versions_cleaned.load(Ordering::Relaxed)
        ));

        // 刷新指标
        output.push_str("# HELP mvcc_flush_runs_total Total number of flush operations\n");
        output.push_str("# TYPE mvcc_flush_runs_total counter\n");
        output.push_str(&format!(
            "mvcc_flush_runs_total {}\n",
            self.flush_runs.load(Ordering::Relaxed)
        ));

        output.push_str("# HELP mvcc_flush_keys_total Total number of keys flushed\n");
        output.push_str("# TYPE mvcc_flush_keys_total counter\n");
        output.push_str(&format!(
            "mvcc_flush_keys_total {}\n",
            self.flush_keys.load(Ordering::Relaxed)
        ));

        output.push_str("# HELP mvcc_flush_bytes_total Total number of bytes flushed\n");
        output.push_str("# TYPE mvcc_flush_bytes_total counter\n");
        output.push_str(&format!(
            "mvcc_flush_bytes_total {}\n",
            self.flush_bytes.load(Ordering::Relaxed)
        ));

        // RocksDB 指标
        output.push_str("# HELP rocksdb_gets_total Total number of RocksDB get operations\n");
        output.push_str("# TYPE rocksdb_gets_total counter\n");
        output.push_str(&format!(
            "rocksdb_gets_total {}\n",
            self.rocksdb_gets.load(Ordering::Relaxed)
        ));

        output.push_str("# HELP rocksdb_puts_total Total number of RocksDB put operations\n");
        output.push_str("# TYPE rocksdb_puts_total counter\n");
        output.push_str(&format!(
            "rocksdb_puts_total {}\n",
            self.rocksdb_puts.load(Ordering::Relaxed)
        ));

        output.push_str("# HELP rocksdb_deletes_total Total number of RocksDB delete operations\n");
        output.push_str("# TYPE rocksdb_deletes_total counter\n");
        output.push_str(&format!(
            "rocksdb_deletes_total {}\n",
            self.rocksdb_deletes.load(Ordering::Relaxed)
        ));

        // 并行证明指标
        output.push_str("# HELP vm_privacy_zk_parallel_proof_total Total parallel ZK proofs attempted\n");
        output.push_str("# TYPE vm_privacy_zk_parallel_proof_total counter\n");
        output.push_str(&format!(
            "vm_privacy_zk_parallel_proof_total {}\n",
            self.parallel_proof_total.load(Ordering::Relaxed)
        ));

        output.push_str("# HELP vm_privacy_zk_parallel_proof_failed_total Total parallel ZK proofs failed\n");
        output.push_str("# TYPE vm_privacy_zk_parallel_proof_failed_total counter\n");
        output.push_str(&format!(
            "vm_privacy_zk_parallel_proof_failed_total {}\n",
            self.parallel_proof_failed.load(Ordering::Relaxed)
        ));

        output.push_str("# HELP vm_privacy_zk_parallel_batches_total Total parallel proof batches processed\n");
        output.push_str("# TYPE vm_privacy_zk_parallel_batches_total counter\n");
        output.push_str(&format!(
            "vm_privacy_zk_parallel_batches_total {}\n",
            self.parallel_proof_batches.load(Ordering::Relaxed)
        ));

        output.push_str("# HELP vm_privacy_zk_parallel_batch_latency_ms Last batch total latency (ms)\n");
        output.push_str("# TYPE vm_privacy_zk_parallel_batch_latency_ms gauge\n");
        output.push_str(&format!(
            "vm_privacy_zk_parallel_batch_latency_ms {:.3}\n",
            self.parallel_last_batch_latency_ms.load(Ordering::Relaxed) as f64 / 1000.0
        ));

        output.push_str("# HELP vm_privacy_zk_parallel_avg_latency_ms Last batch average per-proof latency (ms)\n");
        output.push_str("# TYPE vm_privacy_zk_parallel_avg_latency_ms gauge\n");
        output.push_str(&format!(
            "vm_privacy_zk_parallel_avg_latency_ms {:.3}\n",
            self.parallel_last_batch_avg_latency_ms.load(Ordering::Relaxed) as f64 / 1000.0
        ));

        output.push_str("# HELP vm_privacy_zk_parallel_tps Last batch throughput proofs/sec\n");
        output.push_str("# TYPE vm_privacy_zk_parallel_tps gauge\n");
        output.push_str(&format!(
            "vm_privacy_zk_parallel_tps {:.3}\n",
            self.parallel_last_batch_tps.load(Ordering::Relaxed) as f64 / 1000.0
        ));

        output
    }

    /// 打印当前指标摘要
    pub fn print_summary(&self) {
        println!("=== 性能指标摘要 ===");
        println!("事务:");
        println!("  已启动: {}", self.txn_started.load(Ordering::Relaxed));
        println!("  已提交: {}", self.txn_committed.load(Ordering::Relaxed));
        println!("  已中止: {}", self.txn_aborted.load(Ordering::Relaxed));
        println!("  TPS(总体): {:.2}", self.tps());
        println!("  TPS(窗口): {:.2}", self.tps_window());
        println!("  TPS(峰值-窗口): {:.2}", self.peak_tps());
        println!("  成功率: {:.2}%", self.success_rate());

        let (p50, p90, p99) = self.txn_latency.percentiles();
        println!("延迟 (ms):");
        println!("  P50: {:.2}", p50);
        println!("  P90: {:.2}", p90);
        println!("  P99: {:.2}", p99);
        println!("  AVG: {:.2}", self.txn_latency.avg());

        println!("GC:");
        println!("  运行次数: {}", self.gc_runs.load(Ordering::Relaxed));
        println!(
            "  清理版本: {}",
            self.gc_versions_cleaned.load(Ordering::Relaxed)
        );

        println!("刷新:");
        println!("  运行次数: {}", self.flush_runs.load(Ordering::Relaxed));
        println!("  刷新键数: {}", self.flush_keys.load(Ordering::Relaxed));
        println!("  刷新字节: {}", self.flush_bytes.load(Ordering::Relaxed));
    }
}

impl MetricsCollector {
    /// 便捷方法：获取 P50（ms）
    pub fn latency_p50(&self) -> f64 {
        let (p50, _, _) = self.txn_latency.percentiles();
        p50
    }
    /// 便捷方法：获取 P90（ms）
    pub fn latency_p90(&self) -> f64 {
        let (_, p90, _) = self.txn_latency.percentiles();
        p90
    }
    /// 便捷方法：获取 P99（ms）
    pub fn latency_p99(&self) -> f64 {
        let (_, _, p99) = self.txn_latency.percentiles();
        p99
    }

    /// 记录一次并行批量证明统计
    pub fn record_parallel_batch(
        &self,
        total: u64,
        failed: u64,
        batch_latency_ms: f64,
        avg_latency_ms: f64,
        tps: f64,
    ) {
        self.parallel_proof_total.fetch_add(total, Ordering::Relaxed);
        self.parallel_proof_failed.fetch_add(failed, Ordering::Relaxed);
        self.parallel_proof_batches.fetch_add(1, Ordering::Relaxed);
        self.parallel_last_batch_latency_ms
            .store((batch_latency_ms * 1000.0) as u64, Ordering::Relaxed);
        self.parallel_last_batch_avg_latency_ms
            .store((avg_latency_ms * 1000.0) as u64, Ordering::Relaxed);
        self.parallel_last_batch_tps
            .store((tps * 1000.0) as u64, Ordering::Relaxed);
    }
}
