// SPDX-License-Identifier: GPL-3.0-or-later
// RetryPolicy 行为测试 (集成测试层)
// 要点:
// 1. 指数退避: 延迟按 backoff_factor 增长并被 max_delay 限制
// 2. Fatal 分类应立即停止，不继续重试
// 3. 抖动路径不 panic，允许成功重试
// Note: 这里通过时间下限与次数估计，不直接暴露内部 attempts 计数

use vm_runtime::parallel::{ParallelScheduler, RetryClass, RetryPolicy};
use std::time::{Duration, Instant};
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;

fn flaky_op_builder(failures_before_success: u32) -> impl FnMut(&vm_runtime::parallel::StateManager) -> Result<i32, String> {
    let counter = Arc::new(AtomicU32::new(0));
    move |_m| {
        let n = counter.fetch_add(1, Ordering::SeqCst);
        if n < failures_before_success {
            Err("conflict: simulated".to_string())
        } else {
            Ok(99)
        }
    }
}

#[test]
fn test_retry_policy_basic_backoff() {
    let scheduler = ParallelScheduler::new();
    let policy = RetryPolicy::default()
        .with_max_retries(5)
        .with_base_delay(Duration::from_millis(1))
        .with_max_delay(Duration::from_millis(5))
        .with_backoff_factor(2.0)
        .with_jitter(0.0)
        .with_classifier(|e| if e.contains("conflict") { RetryClass::Retryable } else { RetryClass::Fatal });

    let mut op = flaky_op_builder(3); // 前 3 次失败，第 4 次成功
    let start = Instant::now();
    let result = scheduler.execute_with_retry_policy(&mut op, &policy);
    let elapsed = start.elapsed();
    assert_eq!(result.unwrap(), 99);
    assert!(elapsed >= Duration::from_millis(5), "elapsed {:?} too short, backoff maybe not applied", elapsed);
}

#[test]
fn test_retry_policy_fatal_stops_immediately() {
    let scheduler = ParallelScheduler::new();
    let policy = RetryPolicy::default()
        .with_max_retries(5)
        .with_base_delay(Duration::from_millis(2))
        .with_jitter(0.0)
        .with_classifier(|_e| RetryClass::Fatal);

    let mut attempts = 0u32;
    let mut op = |_: &vm_runtime::parallel::StateManager| {
        attempts += 1;
        Err("any error".to_string())
    };
    let start = Instant::now();
    let res: Result<i32, String> = scheduler.execute_with_retry_policy(&mut op, &policy);
    let elapsed = start.elapsed();
    assert!(res.is_err());
    assert_eq!(attempts, 1, "Fatal error should not retry");
    assert!(elapsed < Duration::from_millis(2));
}

#[test]
fn test_retry_policy_jitter_path() {
    let scheduler = ParallelScheduler::new();
    let policy = RetryPolicy::default()
        .with_max_retries(2)
        .with_base_delay(Duration::from_millis(3))
        .with_max_delay(Duration::from_millis(10))
        .with_backoff_factor(2.0)
        .with_jitter(0.5)
        .with_classifier(|e| if e.contains("conflict") { RetryClass::Retryable } else { RetryClass::Fatal });

    let mut op = flaky_op_builder(2); // 2 次失败，第 3 次成功 (允许尝试 3 次)
    let res: Result<i32, String> = scheduler.execute_with_retry_policy(&mut op, &policy);
    assert!(res.is_ok());
}
