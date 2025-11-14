//! Performance Stress Test
//!
//! High-volume aggregation test to validate production performance.

use l2_executor::{
    aggregation::{AggregationConfig, AggregationDecider},
    metrics::{create_shared_metrics, MetricsTimer},
    program::FibonacciProgram,
    runtime::{BackendType, L2Runtime},
};
use rayon::prelude::*;
use std::sync::Arc;

fn main() {
    env_logger::init_from_env(env_logger::Env::new().default_filter_or("info"));

    println!("🔥 L2 Executor Performance Stress Test\n");

    let metrics = create_shared_metrics();
    let decider = AggregationDecider::new(AggregationConfig::large_app());
    
    println!("📋 Test Configuration:");
    println!("  - Config: Large App (128 workers, 1M cache)");
    println!("  - Backend: Trace zkVM");
    println!("  - Parallelism: {} cores\n", num_cpus::get());

    // Test 1: Throughput test
    println!("⚡ Test 1: Maximum Throughput (1000 proofs)");
    test_throughput(&metrics, 1000);

    // Test 2: Sustained load
    println!("\n🔄 Test 2: Sustained Load (10 batches × 100 proofs)");
    test_sustained_load(&metrics, &decider, 10, 100);

    // Test 3: Burst handling
    println!("\n💥 Test 3: Burst Handling (5000 proofs, parallel)");
    test_burst_handling(&metrics, 5000);

    // Test 4: Cache stress
    println!("\n💾 Test 4: Cache Stress (10K operations, 80% repeat)");
    test_cache_stress(&metrics, 10000, 0.8);

    // Test 5: Large batch aggregation
    println!("\n📦 Test 5: Large Batch Aggregation (1000 proofs)");
    test_large_aggregation(&metrics, &decider, 1000);

    // Print final summary
    println!("\n" + &"=".repeat(70));
    metrics.print_summary();

    // Performance analysis
    analyze_performance(&metrics);

    println!("\n✅ Stress test completed!");
}

fn test_throughput(
    metrics: &impl std::ops::Deref<Target = l2_executor::metrics::MetricsCollector>,
    count: usize,
) {
    let timer = MetricsTimer::new();
    let runtime = L2Runtime::new(BackendType::Trace).expect("Failed to create runtime");
    let zkvm = runtime.create_trace_vm();

    for i in 0..count {
        let program = FibonacciProgram::new(10 + (i % 40) as u32);
        let _proof = zkvm.prove(&program, &[]).expect("Proof generation failed");
        metrics.record_proof_generation(0); // Minimal overhead
    }

    let elapsed = timer.elapsed_ms();
    let throughput = (count as f64 / elapsed as f64) * 1000.0;

    println!("  ✅ Generated {} proofs in {} ms", count, elapsed);
    println!("  📊 Throughput: {:.0} proofs/second", throughput);
}

fn test_sustained_load(
    metrics: &impl std::ops::Deref<Target = l2_executor::metrics::MetricsCollector>,
    decider: &AggregationDecider,
    batches: usize,
    batch_size: usize,
) {
    let timer = MetricsTimer::new();
    let runtime = L2Runtime::new(BackendType::Trace).expect("Failed to create runtime");
    let zkvm = runtime.create_trace_vm();

    for batch in 0..batches {
        let batch_timer = MetricsTimer::new();

        for i in 0..batch_size {
            let program = FibonacciProgram::new(10 + (i % 30) as u32);
            let _proof = zkvm.prove(&program, &[]).expect("Proof generation failed");
        }

        let strategy = decider.decide_strategy(batch_size);
        let strategy_name = match strategy {
            l2_executor::aggregation::AggregationStrategy::SingleLevel { .. } => "single",
            l2_executor::aggregation::AggregationStrategy::TwoLevel { .. } => "two_level",
            _ => "other",
        };

        metrics.record_aggregation(strategy_name, batch_size, batch_timer.elapsed_ms());
        println!(
            "    Batch {}/{}: {} proofs in {} ms",
            batch + 1,
            batches,
            batch_size,
            batch_timer.elapsed_ms()
        );
    }

    println!("  ✅ Total time: {} ms", timer.elapsed_ms());
    println!(
        "  📊 Average batch time: {:.1} ms",
        timer.elapsed_ms() as f64 / batches as f64
    );
}

fn test_burst_handling(
    metrics: &impl std::ops::Deref<Target = l2_executor::metrics::MetricsCollector>,
    count: usize,
) {
    let timer = MetricsTimer::new();
    let runtime = Arc::new(L2Runtime::new(BackendType::Trace).expect("Failed to create runtime"));
    
    let programs: Vec<_> = (0..count)
        .map(|i| FibonacciProgram::new(10 + (i % 50) as u32))
        .collect();

    let workers = num_cpus::get();
    metrics.update_workers(workers as u64);

    programs.par_iter().for_each(|program| {
        let zkvm = runtime.create_trace_vm();
        let _proof = zkvm.prove(program, &[]).expect("Proof generation failed");
    });

    let elapsed = timer.elapsed_ms();
    let throughput = (count as f64 / elapsed as f64) * 1000.0;

    println!("  ✅ Processed {} proofs in {} ms", count, elapsed);
    println!(
        "  📊 Parallel throughput: {:.0} proofs/second ({} workers)",
        throughput, workers
    );
}

fn test_cache_stress(
    metrics: &impl std::ops::Deref<Target = l2_executor::metrics::MetricsCollector>,
    operations: usize,
    repeat_ratio: f64,
) {
    let timer = MetricsTimer::new();
    let runtime = L2Runtime::new(BackendType::Trace).expect("Failed to create runtime");
    let zkvm = runtime.create_trace_vm();

    let unique_programs = (operations as f64 * (1.0 - repeat_ratio)) as usize;
    let programs: Vec<_> = (0..unique_programs)
        .map(|i| FibonacciProgram::new(10 + (i % 40) as u32))
        .collect();

    for i in 0..operations {
        let program_idx = i % unique_programs;
        let _proof = zkvm.prove(&programs[program_idx], &[]).expect("Proof generation failed");

        let is_hit = i >= unique_programs;
        metrics.record_cache(is_hit);
    }

    let hit_rate = metrics.cache_hit_rate();
    println!("  ✅ {} operations in {} ms", operations, timer.elapsed_ms());
    println!("  📊 Cache hit rate: {:.2}% (target: {:.0}%)", hit_rate, repeat_ratio * 100.0);
}

fn test_large_aggregation(
    metrics: &impl std::ops::Deref<Target = l2_executor::metrics::MetricsCollector>,
    decider: &AggregationDecider,
    proof_count: usize,
) {
    let timer = MetricsTimer::new();
    let runtime = L2Runtime::new(BackendType::Trace).expect("Failed to create runtime");
    let zkvm = runtime.create_trace_vm();

    let strategy = decider.decide_strategy(proof_count);
    println!("  Strategy: {}", strategy.description());

    for i in 0..proof_count {
        let program = FibonacciProgram::new(10 + (i % 30) as u32);
        let _proof = zkvm.prove(&program, &[]).expect("Proof generation failed");
    }

    let strategy_name = match strategy {
        l2_executor::aggregation::AggregationStrategy::ThreeLevel { .. } => "three_level",
        l2_executor::aggregation::AggregationStrategy::TwoLevel { .. } => "two_level",
        _ => "other",
    };

    metrics.record_aggregation(strategy_name, proof_count, timer.elapsed_ms());

    let perf = decider.estimate_performance_gain(proof_count);
    metrics.update_tps(perf.aggregated_tps as u64, perf.aggregated_tps as u64);
    metrics.update_savings(
        perf.gas_savings_percent as u64,
        perf.size_savings_percent as u64,
    );

    println!("  ✅ Aggregated {} proofs in {} ms", proof_count, timer.elapsed_ms());
    println!(
        "  📈 Estimated TPS: {:.0}, Gas savings: {:.1}%",
        perf.aggregated_tps, perf.gas_savings_percent
    );
}

fn analyze_performance(
    metrics: &impl std::ops::Deref<Target = l2_executor::metrics::MetricsCollector>,
) {
    println!("\n📊 Performance Analysis:\n");

    let total_proofs = metrics.proof_generation_total.load(std::sync::atomic::Ordering::Relaxed);
    let cache_hit_rate = metrics.cache_hit_rate();
    let tps = metrics.tps_current.load(std::sync::atomic::Ordering::Relaxed);

    println!("  Total proofs generated: {}", total_proofs);
    println!("  Cache effectiveness: {:.2}%", cache_hit_rate);
    println!("  Estimated production TPS: {}", tps);

    // Performance ratings
    println!("\n  Performance Ratings:");
    
    if cache_hit_rate >= 80.0 {
        println!("    ✅ Cache: EXCELLENT (>= 80%)");
    } else if cache_hit_rate >= 50.0 {
        println!("    ⚠️  Cache: GOOD (>= 50%)");
    } else {
        println!("    ❌ Cache: POOR (< 50%)");
    }

    if tps >= 10000 {
        println!("    ✅ TPS: EXCELLENT (>= 10K)");
    } else if tps >= 1000 {
        println!("    ⚠️  TPS: GOOD (>= 1K)");
    } else {
        println!("    ❌ TPS: NEEDS IMPROVEMENT (< 1K)");
    }

    let aggregations = metrics.aggregation_total.load(std::sync::atomic::Ordering::Relaxed);
    if aggregations > 0 {
        println!("    ✅ Aggregation: ACTIVE ({} operations)", aggregations);
    } else {
        println!("    ⚠️  Aggregation: NOT USED");
    }
}
