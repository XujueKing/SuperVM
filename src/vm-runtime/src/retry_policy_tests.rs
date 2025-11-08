// SPDX-License-Identifier: GPL-3.0-or-later
// RetryPolicy 行为测试
// 要点:
// 1. 指数退避: 延迟按 backoff_factor 增长并被 max_delay 限制
// 2. Fatal 分类应立即停止，不继续重试
// 3. 抖动: 启用 jitter 后的实际 sleep 时间分布在允许区间内 (这里不精确测量真实时间，
//    而是通过自定义 classifier 和可控参数，将 base_delay/max_delay 设置极小，
//    只验证执行路径与 attempts 计数)

use crate::parallel::{ParallelScheduler, RetryClass, RetryPolicy};
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

// 构造一个始终返回冲突错误若干次后成功的 operation
fn flaky_op_builder(failures_before_success: u32) -> impl FnMut(&crate::parallel::StateManager) -> Result<i32, String> {
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

    // 自定义策略: 最多 5 次重试, base=1ms, factor=2, max=5ms, jitter=0
    let policy = RetryPolicy::default()
        .with_max_retries(5)
        .with_base_delay(Duration::from_millis(1))
        .with_max_delay(Duration::from_millis(5))
        .with_backoff_factor(2.0)
        .with_jitter(0.0)
        .with_classifier(|e| if e.contains("conflict") { RetryClass::Retryable } else { RetryClass::Fatal });

    let mut op = flaky_op_builder(3); // 前 3 次失败, 第 4 次成功
    let start = Instant::now();
    let result = scheduler.execute_with_retry_policy(&mut op, &policy);
    let elapsed = start.elapsed();

    assert_eq!(result.unwrap(), 99);
    // 期望总共尝试 4 次 (3 失败 + 1 成功)。无法直接从 API 得到 attempts，这里利用时间下限校验:
    // 理论 sleep: 1ms + 2ms + 4ms ( capped 到 5ms => 实际 1 + 2 + 5 ) = ~8ms
    // 给一定裕度: 至少 5ms (考虑计时不精确)
    assert!(elapsed >= Duration::from_millis(5), "elapsed {:?} too short, backoff maybe not applied", elapsed);
}

#[test]
fn test_retry_policy_fatal_stops_immediately() {
    let scheduler = ParallelScheduler::new();
    let policy = RetryPolicy::default()
        .with_max_retries(5)
        .with_base_delay(Duration::from_millis(2))
        .with_jitter(0.0)
        .with_classifier(|_e| RetryClass::Fatal); // 所有错误直接 Fatal

    let mut attempts = 0u32;
    let mut op = |_: &crate::parallel::StateManager| {
        attempts += 1;
        Err("any error".to_string())
    };
    let start = Instant::now();
    let res = scheduler.execute_with_retry_policy(&mut op, &policy);
    let elapsed = start.elapsed();
    assert!(res.is_err());
    assert_eq!(attempts, 1, "Fatal error should not retry");
    // 不应有明显 sleep
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
        .with_jitter(0.5) // 启用 jitter
        .with_classifier(|e| if e.contains("conflict") { RetryClass::Retryable } else { RetryClass::Fatal });

    let mut op = flaky_op_builder(2); // 2 次失败, 第 3 次成功, 但 max_retries=2 => 允许尝试 3 次
    let res = scheduler.execute_with_retry_policy(&mut op, &policy);
    assert!(res.is_ok());
}
