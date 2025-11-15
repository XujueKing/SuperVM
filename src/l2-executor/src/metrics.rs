//! Performance Monitoring Module
//!
//! Provides Prometheus-compatible metrics for L2 Executor performance monitoring.

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Instant;

/// Prometheus-compatible metrics collector
pub struct MetricsCollector {
    // Aggregation metrics
    pub aggregation_total: AtomicU64,
    pub aggregation_strategy_single: AtomicU64,
    pub aggregation_strategy_two_level: AtomicU64,
    pub aggregation_strategy_three_level: AtomicU64,
    pub aggregation_duration_ms: AtomicU64,
    pub aggregation_proof_count: AtomicU64,

    // Performance metrics
    pub tps_current: AtomicU64,
    pub tps_average: AtomicU64,
    pub gas_savings_percent: AtomicU64,
    pub proof_size_savings_percent: AtomicU64,

    // Cache metrics
    pub cache_hits: AtomicU64,
    pub cache_misses: AtomicU64,
    pub cache_evictions: AtomicU64,

    // Proof generation metrics
    pub proof_generation_total: AtomicU64,
    pub proof_generation_duration_ms: AtomicU64,
    pub proof_verification_total: AtomicU64,
    pub proof_verification_duration_ms: AtomicU64,

    // System health metrics
    pub parallel_workers_active: AtomicU64,
    pub l1_submission_gas_used: AtomicU64,
    pub errors_total: AtomicU64,
}

impl MetricsCollector {
    pub fn new() -> Self {
        Self {
            aggregation_total: AtomicU64::new(0),
            aggregation_strategy_single: AtomicU64::new(0),
            aggregation_strategy_two_level: AtomicU64::new(0),
            aggregation_strategy_three_level: AtomicU64::new(0),
            aggregation_duration_ms: AtomicU64::new(0),
            aggregation_proof_count: AtomicU64::new(0),

            tps_current: AtomicU64::new(0),
            tps_average: AtomicU64::new(0),
            gas_savings_percent: AtomicU64::new(0),
            proof_size_savings_percent: AtomicU64::new(0),

            cache_hits: AtomicU64::new(0),
            cache_misses: AtomicU64::new(0),
            cache_evictions: AtomicU64::new(0),

            proof_generation_total: AtomicU64::new(0),
            proof_generation_duration_ms: AtomicU64::new(0),
            proof_verification_total: AtomicU64::new(0),
            proof_verification_duration_ms: AtomicU64::new(0),

            parallel_workers_active: AtomicU64::new(0),
            l1_submission_gas_used: AtomicU64::new(0),
            errors_total: AtomicU64::new(0),
        }
    }

    /// Record an aggregation operation
    pub fn record_aggregation(&self, strategy: &str, proof_count: usize, duration_ms: u64) {
        self.aggregation_total.fetch_add(1, Ordering::Relaxed);
        self.aggregation_proof_count
            .fetch_add(proof_count as u64, Ordering::Relaxed);
        self.aggregation_duration_ms
            .fetch_add(duration_ms, Ordering::Relaxed);

        match strategy {
            "single" => self
                .aggregation_strategy_single
                .fetch_add(1, Ordering::Relaxed),
            "two_level" => self
                .aggregation_strategy_two_level
                .fetch_add(1, Ordering::Relaxed),
            "three_level" => self
                .aggregation_strategy_three_level
                .fetch_add(1, Ordering::Relaxed),
            _ => 0,
        };
    }

    /// Update TPS metrics
    pub fn update_tps(&self, current: u64, average: u64) {
        self.tps_current.store(current, Ordering::Relaxed);
        self.tps_average.store(average, Ordering::Relaxed);
    }

    /// Update savings metrics
    pub fn update_savings(&self, gas_savings: u64, size_savings: u64) {
        self.gas_savings_percent
            .store(gas_savings, Ordering::Relaxed);
        self.proof_size_savings_percent
            .store(size_savings, Ordering::Relaxed);
    }

    /// Record cache operation
    pub fn record_cache(&self, hit: bool) {
        if hit {
            self.cache_hits.fetch_add(1, Ordering::Relaxed);
        } else {
            self.cache_misses.fetch_add(1, Ordering::Relaxed);
        }
    }

    /// Record cache eviction
    pub fn record_eviction(&self) {
        self.cache_evictions.fetch_add(1, Ordering::Relaxed);
    }

    /// Record proof generation
    pub fn record_proof_generation(&self, duration_ms: u64) {
        self.proof_generation_total
            .fetch_add(1, Ordering::Relaxed);
        self.proof_generation_duration_ms
            .fetch_add(duration_ms, Ordering::Relaxed);
    }

    /// Record proof verification
    pub fn record_proof_verification(&self, duration_ms: u64) {
        self.proof_verification_total
            .fetch_add(1, Ordering::Relaxed);
        self.proof_verification_duration_ms
            .fetch_add(duration_ms, Ordering::Relaxed);
    }

    /// Update active workers count
    pub fn update_workers(&self, count: u64) {
        self.parallel_workers_active
            .store(count, Ordering::Relaxed);
    }

    /// Record L1 submission
    pub fn record_l1_submission(&self, gas_used: u64) {
        self.l1_submission_gas_used
            .fetch_add(gas_used, Ordering::Relaxed);
    }

    /// Record error
    pub fn record_error(&self) {
        self.errors_total.fetch_add(1, Ordering::Relaxed);
    }

    /// Get cache hit rate (0-100%)
    pub fn cache_hit_rate(&self) -> f64 {
        let hits = self.cache_hits.load(Ordering::Relaxed) as f64;
        let misses = self.cache_misses.load(Ordering::Relaxed) as f64;
        let total = hits + misses;

        if total > 0.0 {
            (hits / total) * 100.0
        } else {
            0.0
        }
    }

    /// Export metrics in Prometheus text format
    pub fn export_prometheus(&self) -> String {
        let mut output = String::new();

        // Aggregation metrics
        output.push_str("# HELP l2_aggregation_total Total number of aggregation operations\n");
        output.push_str("# TYPE l2_aggregation_total counter\n");
        output.push_str(&format!(
            "l2_aggregation_total {}\n",
            self.aggregation_total.load(Ordering::Relaxed)
        ));

        output.push_str(
            "# HELP l2_aggregation_strategy_total Aggregation operations by strategy\n",
        );
        output.push_str("# TYPE l2_aggregation_strategy_total counter\n");
        output.push_str(&format!(
            "l2_aggregation_strategy_total{{strategy=\"single\"}} {}\n",
            self.aggregation_strategy_single.load(Ordering::Relaxed)
        ));
        output.push_str(&format!(
            "l2_aggregation_strategy_total{{strategy=\"two_level\"}} {}\n",
            self.aggregation_strategy_two_level.load(Ordering::Relaxed)
        ));
        output.push_str(&format!(
            "l2_aggregation_strategy_total{{strategy=\"three_level\"}} {}\n",
            self.aggregation_strategy_three_level
                .load(Ordering::Relaxed)
        ));

        output.push_str(
            "# HELP l2_aggregation_duration_ms_total Total aggregation duration in milliseconds\n",
        );
        output.push_str("# TYPE l2_aggregation_duration_ms_total counter\n");
        output.push_str(&format!(
            "l2_aggregation_duration_ms_total {}\n",
            self.aggregation_duration_ms.load(Ordering::Relaxed)
        ));

        // Performance metrics
        output.push_str("# HELP l2_tps_current Current transactions per second\n");
        output.push_str("# TYPE l2_tps_current gauge\n");
        output.push_str(&format!(
            "l2_tps_current {}\n",
            self.tps_current.load(Ordering::Relaxed)
        ));

        output.push_str("# HELP l2_tps_average Average transactions per second\n");
        output.push_str("# TYPE l2_tps_average gauge\n");
        output.push_str(&format!(
            "l2_tps_average {}\n",
            self.tps_average.load(Ordering::Relaxed)
        ));

        output.push_str("# HELP l2_gas_savings_percent Gas cost savings percentage\n");
        output.push_str("# TYPE l2_gas_savings_percent gauge\n");
        output.push_str(&format!(
            "l2_gas_savings_percent {}\n",
            self.gas_savings_percent.load(Ordering::Relaxed)
        ));

        // Cache metrics
        output.push_str("# HELP l2_cache_hits_total Total cache hits\n");
        output.push_str("# TYPE l2_cache_hits_total counter\n");
        output.push_str(&format!(
            "l2_cache_hits_total {}\n",
            self.cache_hits.load(Ordering::Relaxed)
        ));

        output.push_str("# HELP l2_cache_misses_total Total cache misses\n");
        output.push_str("# TYPE l2_cache_misses_total counter\n");
        output.push_str(&format!(
            "l2_cache_misses_total {}\n",
            self.cache_misses.load(Ordering::Relaxed)
        ));

        output.push_str("# HELP l2_cache_hit_rate Cache hit rate percentage\n");
        output.push_str("# TYPE l2_cache_hit_rate gauge\n");
        output.push_str(&format!("l2_cache_hit_rate {:.2}\n", self.cache_hit_rate()));

        // Proof metrics
        output.push_str("# HELP l2_proof_generation_total Total proofs generated\n");
        output.push_str("# TYPE l2_proof_generation_total counter\n");
        output.push_str(&format!(
            "l2_proof_generation_total {}\n",
            self.proof_generation_total.load(Ordering::Relaxed)
        ));

        output.push_str("# HELP l2_proof_verification_total Total proofs verified\n");
        output.push_str("# TYPE l2_proof_verification_total counter\n");
        output.push_str(&format!(
            "l2_proof_verification_total {}\n",
            self.proof_verification_total.load(Ordering::Relaxed)
        ));

        // System health
        output.push_str("# HELP l2_parallel_workers_active Active parallel workers\n");
        output.push_str("# TYPE l2_parallel_workers_active gauge\n");
        output.push_str(&format!(
            "l2_parallel_workers_active {}\n",
            self.parallel_workers_active.load(Ordering::Relaxed)
        ));

        output.push_str("# HELP l2_errors_total Total errors encountered\n");
        output.push_str("# TYPE l2_errors_total counter\n");
        output.push_str(&format!(
            "l2_errors_total {}\n",
            self.errors_total.load(Ordering::Relaxed)
        ));

        output
    }

    /// Print human-readable metrics summary
    pub fn print_summary(&self) {
        println!("\n=== L2 Executor Metrics Summary ===\n");

        println!("📊 Aggregation Metrics:");
        println!(
            "  Total aggregations: {}",
            self.aggregation_total.load(Ordering::Relaxed)
        );
        println!(
            "    Single-level: {}",
            self.aggregation_strategy_single.load(Ordering::Relaxed)
        );
        println!(
            "    Two-level: {}",
            self.aggregation_strategy_two_level.load(Ordering::Relaxed)
        );
        println!(
            "    Three-level: {}",
            self.aggregation_strategy_three_level
                .load(Ordering::Relaxed)
        );
        println!(
            "  Total proofs aggregated: {}",
            self.aggregation_proof_count.load(Ordering::Relaxed)
        );
        println!(
            "  Total duration: {} ms",
            self.aggregation_duration_ms.load(Ordering::Relaxed)
        );

        println!("\n⚡ Performance Metrics:");
        println!(
            "  Current TPS: {}",
            self.tps_current.load(Ordering::Relaxed)
        );
        println!(
            "  Average TPS: {}",
            self.tps_average.load(Ordering::Relaxed)
        );
        println!(
            "  Gas savings: {}%",
            self.gas_savings_percent.load(Ordering::Relaxed)
        );
        println!(
            "  Proof size savings: {}%",
            self.proof_size_savings_percent.load(Ordering::Relaxed)
        );

        println!("\n💾 Cache Metrics:");
        println!(
            "  Cache hits: {}",
            self.cache_hits.load(Ordering::Relaxed)
        );
        println!(
            "  Cache misses: {}",
            self.cache_misses.load(Ordering::Relaxed)
        );
        println!("  Hit rate: {:.2}%", self.cache_hit_rate());
        println!(
            "  Evictions: {}",
            self.cache_evictions.load(Ordering::Relaxed)
        );

        println!("\n🔐 Proof Metrics:");
        println!(
            "  Proofs generated: {}",
            self.proof_generation_total.load(Ordering::Relaxed)
        );
        println!(
            "  Proofs verified: {}",
            self.proof_verification_total.load(Ordering::Relaxed)
        );

        println!("\n🖥️  System Health:");
        println!(
            "  Active workers: {}",
            self.parallel_workers_active.load(Ordering::Relaxed)
        );
        println!(
            "  Total errors: {}",
            self.errors_total.load(Ordering::Relaxed)
        );

        println!("\n===================================\n");
    }
}

impl Default for MetricsCollector {
    fn default() -> Self {
        Self::new()
    }
}

/// Shared metrics instance (thread-safe)
pub type SharedMetrics = Arc<MetricsCollector>;

/// Create a new shared metrics collector
pub fn create_shared_metrics() -> SharedMetrics {
    Arc::new(MetricsCollector::new())
}

/// Timer for measuring operation duration
pub struct MetricsTimer {
    start: Instant,
}

impl MetricsTimer {
    pub fn new() -> Self {
        Self {
            start: Instant::now(),
        }
    }

    pub fn elapsed_ms(&self) -> u64 {
        self.start.elapsed().as_millis() as u64
    }
}

impl Default for MetricsTimer {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_metrics_collector() {
        let metrics = MetricsCollector::new();

        // Record some operations
        metrics.record_aggregation("single", 10, 100);
        metrics.record_aggregation("two_level", 100, 500);
        metrics.record_cache(true);
        metrics.record_cache(true);
        metrics.record_cache(false);

        assert_eq!(metrics.aggregation_total.load(Ordering::Relaxed), 2);
        assert_eq!(
            metrics.aggregation_strategy_single.load(Ordering::Relaxed),
            1
        );
        assert_eq!(
            metrics.aggregation_strategy_two_level.load(Ordering::Relaxed),
            1
        );
        assert_eq!(metrics.aggregation_proof_count.load(Ordering::Relaxed), 110);
        assert_eq!(metrics.cache_hits.load(Ordering::Relaxed), 2);
        assert_eq!(metrics.cache_misses.load(Ordering::Relaxed), 1);
        assert!((metrics.cache_hit_rate() - 66.67).abs() < 0.1);
    }

    #[test]
    fn test_prometheus_export() {
        let metrics = MetricsCollector::new();
        metrics.record_aggregation("single", 5, 50);
        metrics.update_tps(1000, 950);

        let output = metrics.export_prometheus();

        assert!(output.contains("l2_aggregation_total 1"));
        assert!(output.contains("l2_tps_current 1000"));
        assert!(output.contains("l2_tps_average 950"));
    }

    #[test]
    fn test_timer() {
        let timer = MetricsTimer::new();
        std::thread::sleep(std::time::Duration::from_millis(10));
        let elapsed = timer.elapsed_ms();
        assert!(elapsed >= 10);
    }
}
