// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2025 XujueKing <leadbrand@me.com>

//! ZK 验证指标演示
//!
//! 展示 ZK 验证指标的收集和 Prometheus 导出

use std::time::Duration;
use vm_runtime::metrics::MetricsCollector;

#[cfg(feature = "groth16-verifier")]
use vm_runtime::zk_verifier::ZkBackend;

fn main() {
    println!("=== ZK 验证指标演示 ===\n");

    let metrics = MetricsCollector::new();

    // 模拟一些 ZK 验证操作
    #[cfg(feature = "groth16-verifier")]
    {
        println!("使用真实 ZK 后端类型...");
        
        // 模拟 Groth16 验证
        for _ in 0..5 {
            metrics.record_zk_verify(
                ZkBackend::Groth16Bls12_381,
                true,
                Duration::from_micros(12000),
            );
        }
        
        // 模拟一次失败
        metrics.record_zk_verify(
            ZkBackend::Groth16Bls12_381,
            false,
            Duration::from_micros(15000),
        );
        
        // 模拟 Plonk 验证
        for _ in 0..3 {
            metrics.record_zk_verify(
                ZkBackend::Plonk,
                true,
                Duration::from_micros(8000),
            );
        }
    }

    #[cfg(not(feature = "groth16-verifier"))]
    {
        println!("使用 mock ZK 后端...");
        
        for _ in 0..5 {
            metrics.record_zk_verify("groth16", true, Duration::from_micros(12000));
        }
        
        metrics.record_zk_verify("groth16", false, Duration::from_micros(15000));
        
        for _ in 0..3 {
            metrics.record_zk_verify("plonk", true, Duration::from_micros(8000));
        }
    }

    println!("\n--- ZK 指标摘要 ---");
    println!("总验证次数: {}", metrics.zk_verify_total.load(std::sync::atomic::Ordering::Relaxed));
    println!("失败次数: {}", metrics.zk_verify_failures.load(std::sync::atomic::Ordering::Relaxed));
    println!("失败率: {:.2}%", metrics.zk_verify_failure_rate() * 100.0);
    println!("平均延迟: {:.3} ms", metrics.zk_verify_avg_latency_ms());

    #[cfg(feature = "groth16-verifier")]
    {
        println!("\n后端类型分布:");
        println!("  Groth16: {}", metrics.zk_backend_groth16_count.load(std::sync::atomic::Ordering::Relaxed));
        println!("  Plonk: {}", metrics.zk_backend_plonk_count.load(std::sync::atomic::Ordering::Relaxed));
        println!("  Mock: {}", metrics.zk_backend_mock_count.load(std::sync::atomic::Ordering::Relaxed));
    }

    println!("\n--- Prometheus 导出 (ZK 部分) ---");
    let prom_output = metrics.export_prometheus();
    
    // 提取 ZK 相关指标
    for line in prom_output.lines() {
        if line.contains("vm_zk_") {
            println!("{}", line);
        }
    }
    
    println!("\n演示完成！");
}
