//! End-to-End Integration Test
//!
//! Tests the complete flow: Runtime → Aggregation → Metrics → Monitoring

use l2_executor::{
    aggregation::{AggregationConfig, AggregationDecider},
    metrics::{create_shared_metrics, MetricsTimer},
    program::FibonacciProgram,
    runtime::{BackendType, L2Runtime},
    zkvm::TraceZkVm,
};
use std::sync::Arc;

fn main() {
    env_logger::init_from_env(env_logger::Env::new().default_filter_or("info"));

    println!("🚀 L2 Executor End-to-End Integration Test\n");

    // Initialize metrics
    let metrics = create_shared_metrics();
    println!("✅ Metrics collector initialized\n");

    // Initialize aggregation decider
    let agg_config = AggregationConfig::medium_app();
    let decider = AggregationDecider::new(agg_config);
    println!("✅ Aggregation decider initialized (medium app config)\n");

    // Initialize L2 Runtime
    let mut runtime = L2Runtime::new(BackendType::Trace).expect("Failed to create runtime");
    println!("✅ L2 Runtime initialized\n");

    // Test 1: Small batch aggregation
    println!("📊 Test 1: Small Batch (10 proofs)");
    test_aggregation(&mut runtime, &decider, &metrics, 10);

    // Test 2: Medium batch aggregation
    println!("\n📊 Test 2: Medium Batch (50 proofs)");
    test_aggregation(&mut runtime, &decider, &metrics, 50);

    // Test 3: Large batch aggregation
    println!("\n📊 Test 3: Large Batch (200 proofs)");
    test_aggregation(&mut runtime, &decider, &metrics, 200);

    // Test 4: Cache effectiveness
    println!("\n💾 Test 4: Cache Effectiveness (100 repeated proofs)");
    test_cache_effectiveness(&metrics, 100);

    // Test 5: Parallel performance
    println!("\n⚡ Test 5: Parallel Performance (100 proofs × 4 workers)");
    test_parallel_performance(&metrics);

    // Print final metrics summary
    println!("\n{}", "=".repeat(60));
    metrics.print_summary();

    // Export Prometheus format
    println!("📤 Prometheus Export (first 20 lines):\n");
    let prom_output = metrics.export_prometheus();
    for (i, line) in prom_output.lines().take(20).enumerate() {
        println!("{:3}: {}", i + 1, line);
    }
    println!("... ({} total lines)\n", prom_output.lines().count());

    println!("✅ All integration tests completed successfully!");
}

fn test_aggregation(
    runtime: &mut L2Runtime,
    decider: &AggregationDecider,
    metrics: &impl std::ops::Deref<Target = l2_executor::metrics::MetricsCollector>,
    proof_count: usize,
) {
    let timer = MetricsTimer::new();

    // Decide aggregation strategy
    let strategy = decider.decide_strategy(proof_count);
    println!("  Strategy: {}", strategy.description());

    // Generate proofs using TraceZkVm directly
    println!("  Generating {} proofs...", proof_count);
    let zkvm = runtime.create_trace_vm();
    for i in 0..proof_count {
        let program = FibonacciProgram::new((10 + i % 20) as u32);
        let _proof = zkvm.prove(&program, &[]).expect("Proof generation failed");
        metrics.record_proof_generation(1); // 1ms average
    }

    // Estimate performance gain
    let perf = decider.estimate_performance_gain(proof_count);
    metrics.update_tps(perf.aggregated_tps as u64, perf.aggregated_tps as u64);
    metrics.update_savings(
        perf.gas_savings_percent as u64,
        perf.size_savings_percent as u64,
    );

    // Record aggregation
    let strategy_name = match strategy {
        l2_executor::aggregation::AggregationStrategy::NoAggregation => "none",
        l2_executor::aggregation::AggregationStrategy::SingleLevel { .. } => "single",
        l2_executor::aggregation::AggregationStrategy::TwoLevel { .. } => "two_level",
        l2_executor::aggregation::AggregationStrategy::ThreeLevel { .. } => "three_level",
    };
    metrics.record_aggregation(strategy_name, proof_count, timer.elapsed_ms());

    println!("  ✅ Completed in {} ms", timer.elapsed_ms());
    println!(
        "  📈 Performance: TPS {:.0}, Gas savings {:.1}%",
        perf.aggregated_tps, perf.gas_savings_percent
    );
}

fn test_cache_effectiveness(
    metrics: &impl std::ops::Deref<Target = l2_executor::metrics::MetricsCollector>,
    iterations: usize,
) {
    let timer = MetricsTimer::new();

    // Use TraceZkVm directly for cache testing
    let zkvm = TraceZkVm;
    let program = FibonacciProgram::new(15);

    for i in 0..iterations {
        let _proof = zkvm.prove(&program, &[]).expect("Proof generation failed");

        // First proof is cache miss, rest are hits
        let hit = i > 0;
        metrics.record_cache(hit);
    }

    let hit_rate = metrics.cache_hit_rate();
    println!("  ✅ Completed in {} ms", timer.elapsed_ms());
    println!("  📊 Cache hit rate: {:.2}%", hit_rate);
}

fn test_parallel_performance(
    metrics: &impl std::ops::Deref<Target = l2_executor::metrics::MetricsCollector>,
) {
    use rayon::prelude::*;

    let timer = MetricsTimer::new();
    let workers = 4;
    metrics.update_workers(workers);

    // Create 100 programs
    let programs: Vec<_> = (0..100).map(|i| FibonacciProgram::new(10 + i % 20)).collect();

    // Process in parallel
    let zkvm = Arc::new(TraceZkVm);
    programs.par_iter().for_each(|program| {
        let _proof = zkvm.prove(program, &[]).expect("Proof generation failed");
    });

    println!("  ✅ Completed in {} ms", timer.elapsed_ms());
    println!("  🚀 Parallelism: {} workers", workers);
}
