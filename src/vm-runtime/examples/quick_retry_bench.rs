// SPDX-License-Identifier: GPL-3.0-or-later
// Quick benchmark for RetryPolicy performance validation

use std::time::{Duration, Instant};
use std::env;
use vm_runtime::parallel::{ParallelScheduler, RetryClass, RetryPolicy};

fn main() {
    println!("=== Quick RetryPolicy Benchmark ===\n");
    
    let scheduler = ParallelScheduler::new();
    let policy = RetryPolicy::default()
        .with_max_retries(5)
        .with_base_delay(Duration::from_micros(50))
        .with_max_delay(Duration::from_millis(2))
        .with_backoff_factor(2.0)
        .with_jitter(0.1)
        .with_classifier(|e| if e.contains("conflict") { RetryClass::Retryable } else { RetryClass::Fatal });

    let iterations: usize = env::var("RETRY_ITERS")
        .ok()
        .and_then(|v| v.parse::<usize>().ok())
        .unwrap_or(1000);
    
    // Test 1: RetryPolicy with 3 failures before success
    let start = Instant::now();
    let mut successes = 0;
    for _ in 0..iterations {
        let mut attempt_count = 0;
        let result = scheduler.execute_with_retry_policy(
            |_sm| {
                attempt_count += 1;
                if attempt_count < 4 {
                    Err("conflict".to_string())
                } else {
                    Ok(42)
                }
            },
            &policy
        );
        if result.is_ok() {
            successes += 1;
        }
    }
    let elapsed_policy = start.elapsed();
    
    println!("📊 RetryPolicy (3 failures before success):");
    println!("  Total iterations: {}", iterations);
    println!("  Successes: {}", successes);
    println!("  Total time: {:.2?}", elapsed_policy);
    println!("  Avg per op: {:.2?}", elapsed_policy / iterations as u32);
    println!("  Throughput: {:.0} ops/sec\n", iterations as f64 / elapsed_policy.as_secs_f64());
    
    // Test 2: Manual retry loop (baseline)
    let start = Instant::now();
    let mut successes = 0;
    for _ in 0..iterations {
        let mut attempt_count = 0;
        let mut result = Err("init".to_string());
        for _ in 0..6 {
            attempt_count += 1;
            if attempt_count < 4 {
                result = Err("conflict".to_string());
            } else {
                result = Ok(42);
                break;
            }
        }
        if result.is_ok() {
            successes += 1;
        }
    }
    let elapsed_manual = start.elapsed();
    
    println!("📊 Manual Retry Loop (baseline):");
    println!("  Total iterations: {}", iterations);
    println!("  Successes: {}", successes);
    println!("  Total time: {:.2?}", elapsed_manual);
    println!("  Avg per op: {:.2?}", elapsed_manual / iterations as u32);
    println!("  Throughput: {:.0} ops/sec\n", iterations as f64 / elapsed_manual.as_secs_f64());
    
    let overhead_pct = ((elapsed_policy.as_secs_f64() / elapsed_manual.as_secs_f64()) - 1.0) * 100.0;
    println!("⚖️  RetryPolicy overhead: {:.2}%", overhead_pct);
    
    println!("\n=== Execution Statistics ===");
    let stats = scheduler.get_stats();
    println!("Total transactions: {}", stats.total_txs());
    println!("Successful: {}", stats.successful_txs);
    println!("Failed: {}", stats.failed_txs);
    println!("Retry count: {}", stats.retry_count);
}
