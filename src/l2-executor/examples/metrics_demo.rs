//! Performance Monitoring Demo
//!
//! Demonstrates integration of Prometheus metrics with aggregation operations.

use l2_executor::aggregation::{AggregationConfig, AggregationDecider};
use l2_executor::metrics::{create_shared_metrics, MetricsTimer, SharedMetrics};
use std::thread;
use std::time::Duration;

fn main() {
    println!("🚀 L2 Executor Performance Monitoring Demo\n");

    // Create shared metrics collector
    let metrics = create_shared_metrics();

    // Initialize aggregation decider
    let config = AggregationConfig::default();
    let decider = AggregationDecider::new(config);

    println!("📊 Simulating aggregation operations with metrics...\n");

    // Simulate different aggregation scenarios
    simulate_aggregation(&metrics, &decider, 6);
    simulate_aggregation(&metrics, &decider, 25);
    simulate_aggregation(&metrics, &decider, 150);
    simulate_aggregation(&metrics, &decider, 800);

    // Simulate cache operations
    println!("💾 Simulating cache operations...");
    for i in 0..100 {
        let hit = i % 3 != 0; // ~67% hit rate
        metrics.record_cache(hit);
    }
    println!("  Recorded 100 cache operations\n");

    // Simulate proof operations
    println!("🔐 Simulating proof generation/verification...");
    for _ in 0..50 {
        metrics.record_proof_generation(85);
        metrics.record_proof_verification(1);
    }
    println!("  Generated and verified 50 proofs\n");

    // Update system metrics
    metrics.update_workers(8);
    metrics.record_l1_submission(300_000);

    // Print summary
    metrics.print_summary();

    // Export Prometheus format
    println!("📤 Prometheus Metrics Export:\n");
    println!("{}", metrics.export_prometheus());

    // Demonstrate HTTP endpoint simulation
    println!("\n🌐 Simulated Prometheus scrape endpoint:");
    println!("   GET http://localhost:9090/metrics\n");
    println!("   (In production, use actix-web or warp to serve this)");
}

fn simulate_aggregation(metrics: &SharedMetrics, decider: &AggregationDecider, proof_count: usize) {
    let timer = MetricsTimer::new();

    // Decide strategy
    let strategy = decider.decide_strategy(proof_count);
    let strategy_name = match strategy {
        l2_executor::aggregation::AggregationStrategy::NoAggregation => "none",
        l2_executor::aggregation::AggregationStrategy::SingleLevel { .. } => "single",
        l2_executor::aggregation::AggregationStrategy::TwoLevel { .. } => "two_level",
        l2_executor::aggregation::AggregationStrategy::ThreeLevel { .. } => "three_level",
    };

    // Simulate aggregation work
    thread::sleep(Duration::from_millis(10));

    // Record metrics
    let duration = timer.elapsed_ms();
    metrics.record_aggregation(strategy_name, proof_count, duration);

    // Estimate performance
    let perf = decider.estimate_performance_gain(proof_count);
    metrics.update_tps(perf.aggregated_tps as u64, perf.aggregated_tps as u64);
    metrics.update_savings(
        perf.gas_savings_percent as u64,
        perf.size_savings_percent as u64,
    );

    println!(
        "  ✅ Aggregated {} proofs using {} strategy ({} ms)",
        proof_count, strategy_name, duration
    );
}
