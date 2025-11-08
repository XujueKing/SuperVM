// SPDX-License-Identifier: GPL-3.0-or-later
// 自适应路由器单元测试：验证高冲突下降与低冲突上升逻辑

use vm_runtime::adaptive_router::{AdaptiveRouter, AdaptiveRouterConfig};
use vm_runtime::parallel_mvcc::MvccSchedulerStats;

// 构造一个 stats 快照
fn make_stats(success: u64, failed: u64, conflict: u64) -> MvccSchedulerStats {
    MvccSchedulerStats {
        successful_txs: success,
        failed_txs: failed,
        conflict_count: conflict,
        retry_count: 0,
    }
}

#[test]
fn test_adaptive_router_low_conflict_increase() {
    let router = AdaptiveRouter::new();
    let initial = router.target_fast_ratio();
    // 模拟 3 个窗口，每窗口 100 个新事务，冲突率极低 (<0.05)
    // 每次窗口：成功 +100，冲突 +0
    for w in 0..3 {
        for i in 0..100 { // tick 达到 update_every 触发 maybe_update
            let stats = make_stats((w * 100 + i + 1) as u64, 0, (w * 0) as u64);
            router.maybe_update(&stats);
        }
    }
    let after = router.target_fast_ratio();
    assert!(after > initial, "fast ratio should increase on sustained low conflict");
}

#[test]
fn test_adaptive_router_high_conflict_decrease() {
    let router = AdaptiveRouter::new();
    let initial = router.target_fast_ratio();
    // 模拟高冲突窗口：100 笔事务里冲突 40 （冲突率 0.4 > 0.25 阈值）
    let mut success_acc = 0u64;
    let mut conflict_acc = 0u64;
    for i in 0..100 {
        success_acc += 1; // 近似，把所有非失败视为成功
        // 分配冲突：前 40 次标记冲突累积
        if i < 40 { conflict_acc += 1; }
        let stats = make_stats(success_acc, 0, conflict_acc);
        router.maybe_update(&stats);
    }
    let after = router.target_fast_ratio();
    assert!(after < initial, "fast ratio should decrease on high conflict");
}

#[test]
fn test_adaptive_router_adjustments_counter() {
    let router = AdaptiveRouter::new();
#[test]
fn test_adaptive_router_config_initial_ratio() {
    let cfg = AdaptiveRouterConfig { initial_fast_ratio: 0.42, ..AdaptiveRouterConfig::default() };
    let router = AdaptiveRouter::new_with_config(cfg);
    assert!((router.target_fast_ratio() - 0.42).abs() < 1e-6, "initial ratio should follow config");
}
    // 低冲突窗口 → 提升一次
    for i in 0..100 {
        let stats = make_stats(i as u64 + 1, 0, 0);
        router.maybe_update(&stats);
    }
    let adj1 = router.adjustments_total();
    assert!(adj1 >= 1, "should have at least one adjustment after first window");
}