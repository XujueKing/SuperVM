// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2025 XujueKing <leadbrand@me.com>

//! FastPath 延迟分位增强演示
//!
//! 验证新增的 p50/p90/p95/p99 延迟指标、重试统计与 Prometheus 导出
//!
//! 运行:
//! ```bash
//! cargo run -p vm-runtime --example fastpath_latency_demo --release
//! ```

use vm_runtime::parallel::{FastPathExecutor, TxId};
use std::time::Duration;
use std::thread;

fn main() {
    println!("=== FastPath 延迟分位增强演示 ===\n");

    let executor = FastPathExecutor::new();

    // 模拟不同延迟的事务
    println!("执行 1000 笔事务(模拟不同延迟)...");
    
    for i in 0..1000 {
        let delay_us = match i % 10 {
            0 => 50,    // P10: 50μs
            1..=4 => 100,  // P40: 100μs
            5..=7 => 200,  // P30: 200μs
            8 => 500,   // P10: 500μs
            _ => 1000,  // P10: 1ms
        };

        executor.execute(i as TxId, || {
            thread::sleep(Duration::from_micros(delay_us));
            Ok(42)
        }).ok();
    }

    // 模拟需要重试的事务
    println!("执行 50 笔带重试的事务...");
    for i in 1000..1050 {
        let mut attempt = 0;
        executor.execute_with_retry(i as TxId, || {
            attempt += 1;
            if attempt < 2 {
                Err("模拟失败需要重试".to_string())
            } else {
                thread::sleep(Duration::from_micros(150));
                Ok(42)
            }
        }, 3).ok();
    }

    // 获取统计信息
    let stats = executor.stats();

    println!("\n{}", "=".repeat(60));
    println!("{}", stats.summary());
    println!("{}", "=".repeat(60));

    // 导出 Prometheus 指标
    println!("\n=== Prometheus 导出 ===\n");
    println!("{}", executor.export_prometheus("fastpath"));

    // 验证关键指标
    println!("\n=== 关键指标验证 ===");
    println!("✓ 总执行: {}", stats.executed_count);
    println!("✓ 重试次数: {}", stats.retry_count);
    println!("✓ 重试率: {:.2}%", stats.retry_rate() * 100.0);
    println!("✓ P50 延迟: {:.3}ms", stats.p50_latency_ms);
    println!("✓ P90 延迟: {:.3}ms", stats.p90_latency_ms);
    println!("✓ P95 延迟: {:.3}ms", stats.p95_latency_ms);
    println!("✓ P99 延迟: {:.3}ms", stats.p99_latency_ms);

    // 性能预测
    println!("\n=== 性能预测 ===");
    println!("基于平均延迟的估算 TPS: {:.0}", stats.estimated_tps());
}
