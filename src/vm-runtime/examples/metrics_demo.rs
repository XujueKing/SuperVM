// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2025 XujueKing <leadbrand@me.com>

//! Metrics Collection Demo
//! 演示如何使用 MetricsCollector 收集和导出性能指标

use std::thread;
use std::time::Duration;
use vm_runtime::MvccStore;

fn main() {
    println!("=== MVCC Store Metrics Collection Demo ===\n");

    // 1. 创建 MVCC Store (默认启用指标收集)
    let store = MvccStore::new();

    // 2. 执行一些事务
    println!("📝 执行测试事务...");

    // 成功的事务
    for i in 0..50 {
        let tx = store.begin();
        let mut tx = tx;
        tx.write(
            format!("key_{}", i).into_bytes(),
            format!("value_{}", i).into_bytes(),
        );
        match tx.commit() {
            Ok(commit_ts) => {
                if i % 10 == 0 {
                    println!("✅ 事务 {} 提交成功, commit_ts={}", i, commit_ts);
                }
            }
            Err(e) => println!("❌ 事务 {} 失败: {}", i, e),
        }
    }

    // 只读事务
    println!("\n📖 执行只读事务...");
    for _i in 0..20 {
        let tx = store.begin_read_only();
        let mut tx = tx;
        let _ = tx.read(b"key_0");
        let _ = tx.commit();
    }

    // 模拟冲突事务
    println!("\n⚔️ 模拟冲突事务...");
    let tx1 = store.begin();
    let tx2 = store.begin();

    let mut tx1 = tx1;
    let mut tx2 = tx2;

    tx1.write(b"conflict_key".to_vec(), b"value1".to_vec());
    tx2.write(b"conflict_key".to_vec(), b"value2".to_vec());

    // tx1 先提交
    match tx1.commit() {
        Ok(_) => println!("✅ tx1 提交成功"),
        Err(e) => println!("❌ tx1 失败: {}", e),
    }

    // tx2 会冲突
    match tx2.commit() {
        Ok(_) => println!("✅ tx2 提交成功"),
        Err(e) => println!("❌ tx2 失败: {}", e),
    }

    // 等待一小段时间让指标稳定
    thread::sleep(Duration::from_millis(100));

    // 3. 导出指标
    if let Some(metrics) = store.get_metrics() {
        println!("\n=== 性能指标摘要 ===");
        metrics.print_summary();

        println!("\n=== Prometheus 格式导出 ===");
        println!("{}", metrics.export_prometheus());
    } else {
        println!("⚠️ 指标收集未启用");
    }
}
