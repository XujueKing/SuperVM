// SPDX-License-Identifier: GPL-3.0-or-later
// Benchmark: RetryPolicy vs No-Retry execution cost under simulated conflicts
// Criterion-based micro benchmark

use criterion::{criterion_group, criterion_main, Criterion, BatchSize};
use std::time::{Duration, Instant};
use vm_runtime::parallel::{ParallelScheduler, RetryClass, RetryPolicy};

fn make_retry_policy() -> RetryPolicy {
    RetryPolicy::default()
        .with_max_retries(5)
        .with_base_delay(Duration::from_micros(50))
        .with_max_delay(Duration::from_millis(2))
        .with_backoff_factor(2.0)
        .with_jitter(0.1)
        .with_classifier(|e| if e.contains("conflict") { RetryClass::Retryable } else { RetryClass::Fatal })
}

fn bench_retry_policy(c: &mut Criterion) {
    let scheduler = ParallelScheduler::new();
    let policy = make_retry_policy();

    // Simulate an operation that fails N times before success
    c.bench_function("retry_policy_conflict_3_failures", |b| {
        b.iter_batched(
            || {
                let mut attempts = 0u32;
                move |_: &vm_runtime::parallel::StateManager| {
                    if attempts < 3 { attempts += 1; Err("conflict".to_string()) } else { Ok(42) }
                }
            },
            |mut op| {
                let _ = scheduler.execute_with_retry_policy(&mut op, &policy);
            },
            BatchSize::SmallInput,
        );
    });

    c.bench_function("no_retry_conflict_3_failures", |b| {
        b.iter_batched(
            || {
                let mut attempts = 0u32;
                move |_: &vm_runtime::parallel::StateManager| {
                    if attempts < 3 { attempts += 1; Err("conflict".to_string()) } else { Ok(42) }
                }
            },
            |mut op| {
                // Manual single attempt loop without policy
                let mut last = Err("init".to_string());
                for _ in 0..6 {
                    match op(&scheduler.get_state_manager().lock().unwrap()) {
                        Ok(v) => { last = Ok(v); break; }
                        Err(e) => { last = Err(e); }
                    }
                }
                let _ = last;
            },
            BatchSize::SmallInput,
        );
    });
}

criterion_group!(retry_policy, bench_retry_policy);
criterion_main!(retry_policy);
