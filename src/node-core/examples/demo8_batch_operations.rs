//! Demo 8: 批量操作演示
//!
//! 展示如何使用批量操作提升性能
//!
//! 运行: cargo run --example demo8_batch_operations

use std::time::Instant;
use vm_runtime::ParallelScheduler;

fn main() {
    println!("=== Demo 8: 批量操作演示 ===\n");

    // 示例 1: 批量写入 vs 单个写入
    demo_batch_write_performance();

    // 示例 2: 批量读取
    demo_batch_read();

    // 示例 3: 批量执行交易
    demo_batch_execution();

    // 示例 4: 批量失败回滚
    demo_batch_rollback();
}

/// 示例 1: 批量写入性能对比
fn demo_batch_write_performance() {
    println!("📌 示例 1: 批量写入性能对比\n");

    let scheduler = ParallelScheduler::new();
    let num_writes = 1000;

    // 方式 1: 单个写入
    let start = Instant::now();
    for i in 0..num_writes {
        let storage = scheduler.get_storage();
        let mut storage = storage.lock().unwrap();
        storage.insert(
            format!("single_key_{}", i).into_bytes(),
            format!("value_{}", i).into_bytes(),
        );
    }
    let single_duration = start.elapsed();

    // 方式 2: 批量写入
    let start = Instant::now();
    let writes: Vec<_> = (0..num_writes)
        .map(|i| {
            (
                format!("batch_key_{}", i).into_bytes(),
                format!("value_{}", i).into_bytes(),
            )
        })
        .collect();

    let count = scheduler.batch_write(writes).unwrap();
    let batch_duration = start.elapsed();

    println!("✅ 写入 {} 条记录", num_writes);
    println!("\n性能对比:");
    println!("  单个写入: {:?}", single_duration);
    println!("  批量写入: {:?}", batch_duration);
    println!(
        "  加速比: {:.2}x",
        single_duration.as_secs_f64() / batch_duration.as_secs_f64()
    );
    println!(
        "  批量写入效率提升: {:.1}%",
        ((single_duration.as_secs_f64() - batch_duration.as_secs_f64())
            / single_duration.as_secs_f64())
            * 100.0
    );

    assert_eq!(count, num_writes);

    println!("\n{}\n", "=".repeat(60));
}

/// 示例 2: 批量读取
fn demo_batch_read() {
    println!("📌 示例 2: 批量读取\n");

    let scheduler = ParallelScheduler::new();

    // 准备数据
    println!("准备测试数据...");
    let writes: Vec<_> = (0..100)
        .map(|i| {
            (
                format!("user_{}", i).into_bytes(),
                format!("{{\"balance\": {}}}", i * 100).into_bytes(),
            )
        })
        .collect();

    scheduler.batch_write(writes).unwrap();

    // 批量读取特定用户
    let keys_to_read: Vec<_> = vec![0, 10, 20, 30, 40, 50]
        .iter()
        .map(|i| format!("user_{}", i).into_bytes())
        .collect();

    println!("\n读取用户: 0, 10, 20, 30, 40, 50");

    let start = Instant::now();
    let results = scheduler.batch_read(&keys_to_read).unwrap();
    let duration = start.elapsed();

    println!("\n✅ 批量读取结果:");
    for (key, value) in results {
        let user = String::from_utf8(key).unwrap();
        let data = String::from_utf8(value).unwrap();
        println!("  - {}: {}", user, data);
    }

    println!("\n⏱️  耗时: {:?}", duration);

    println!("\n{}\n", "=".repeat(60));
}

/// 示例 3: 批量执行交易
fn demo_batch_execution() {
    println!("📌 示例 3: 批量执行交易\n");

    let scheduler = ParallelScheduler::new();

    // 初始化账户
    {
        let storage = scheduler.get_storage();
        let mut storage = storage.lock().unwrap();
        storage.insert(b"alice".to_vec(), b"1000".to_vec());
        storage.insert(b"bob".to_vec(), b"500".to_vec());
        storage.insert(b"charlie".to_vec(), b"200".to_vec());
    }

    println!("初始余额:");
    print_balances(&scheduler, &["alice", "bob", "charlie"]);

    // 批量执行多个转账
    println!("\n执行批量转账:");
    println!("  1. Alice -> Bob: 100");
    println!("  2. Bob -> Charlie: 50");
    println!("  3. Charlie -> Alice: 30");

    let operations = vec![
        // 转账 1: Alice -> Bob
        Box::new(|manager: &vm_runtime::StateManager| transfer(manager, "alice", "bob", 100))
            as Box<dyn FnOnce(&vm_runtime::StateManager) -> Result<String, String>>,
        // 转账 2: Bob -> Charlie
        Box::new(|manager: &vm_runtime::StateManager| transfer(manager, "bob", "charlie", 50)),
        // 转账 3: Charlie -> Alice
        Box::new(|manager: &vm_runtime::StateManager| transfer(manager, "charlie", "alice", 30)),
    ];

    let start = Instant::now();
    match scheduler.execute_batch(operations) {
        Ok(results) => {
            let duration = start.elapsed();
            println!("\n✅ 批量执行成功!");
            for (i, msg) in results.iter().enumerate() {
                println!("  ✓ 交易 {}: {}", i + 1, msg);
            }
            println!("\n⏱️  总耗时: {:?}", duration);
        }
        Err(e) => {
            println!("\n❌ 批量执行失败: {}", e);
        }
    }

    println!("\n最终余额:");
    print_balances(&scheduler, &["alice", "bob", "charlie"]);

    // 显示统计
    let stats = scheduler.get_stats();
    println!("\n📊 统计信息:");
    println!("  - 成功交易: {}", stats.successful_txs);
    println!("  - 失败交易: {}", stats.failed_txs);
    println!("  - 成功率: {:.2}%", stats.success_rate() * 100.0);

    println!("\n{}\n", "=".repeat(60));
}

/// 示例 4: 批量失败回滚
fn demo_batch_rollback() {
    println!("📌 示例 4: 批量失败自动回滚\n");

    let scheduler = ParallelScheduler::new();

    // 初始化账户
    {
        let storage = scheduler.get_storage();
        let mut storage = storage.lock().unwrap();
        storage.insert(b"alice".to_vec(), b"100".to_vec());
        storage.insert(b"bob".to_vec(), b"50".to_vec());
    }

    println!("初始余额:");
    print_balances(&scheduler, &["alice", "bob"]);

    // 批量执行,其中一个会失败
    println!("\n尝试批量转账 (其中一个会失败):");
    println!("  1. Alice -> Bob: 50 ✓");
    println!("  2. Alice -> Bob: 100 ❌ (余额不足)");
    println!("  3. Bob -> Alice: 20 (不会执行)");

    let operations = vec![
        // 转账 1: 成功
        Box::new(|manager: &vm_runtime::StateManager| transfer(manager, "alice", "bob", 50))
            as Box<dyn FnOnce(&vm_runtime::StateManager) -> Result<String, String>>,
        // 转账 2: 失败 (余额不足)
        Box::new(|manager: &vm_runtime::StateManager| transfer(manager, "alice", "bob", 100)),
        // 转账 3: 不会执行
        Box::new(|manager: &vm_runtime::StateManager| transfer(manager, "bob", "alice", 20)),
    ];

    match scheduler.execute_batch(operations) {
        Ok(_) => {
            println!("\n✅ 批量执行成功 (不应该到这里)");
        }
        Err(e) => {
            println!("\n❌ 批量执行失败: {}", e);
            println!("✅ 所有交易已自动回滚");
        }
    }

    println!("\n最终余额 (应该与初始余额相同):");
    print_balances(&scheduler, &["alice", "bob"]);

    // 显示统计
    let stats = scheduler.get_stats();
    println!("\n📊 统计信息:");
    println!("  - 成功交易: {}", stats.successful_txs);
    println!("  - 失败交易: {}", stats.failed_txs);
    println!("  - 回滚次数: {}", stats.rollback_count);

    println!("\n{}\n", "=".repeat(60));
}

/// 辅助函数: 转账
fn transfer(
    manager: &vm_runtime::StateManager,
    from: &str,
    to: &str,
    amount: u64,
) -> Result<String, String> {
    let storage = manager.get_storage();
    let mut storage = storage.lock().unwrap();

    // 读取余额
    let from_balance: u64 = storage
        .get(from.as_bytes())
        .and_then(|b| String::from_utf8(b.clone()).ok())
        .and_then(|s| s.parse().ok())
        .unwrap_or(0);

    if from_balance < amount {
        return Err(format!(
            "{} 余额不足 (需要: {}, 当前: {})",
            from, amount, from_balance
        ));
    }

    let to_balance: u64 = storage
        .get(to.as_bytes())
        .and_then(|b| String::from_utf8(b.clone()).ok())
        .and_then(|s| s.parse().ok())
        .unwrap_or(0);

    // 更新余额
    storage.insert(
        from.as_bytes().to_vec(),
        (from_balance - amount).to_string().into_bytes(),
    );
    storage.insert(
        to.as_bytes().to_vec(),
        (to_balance + amount).to_string().into_bytes(),
    );

    Ok(format!("{} -> {} : {}", from, to, amount))
}

/// 辅助函数: 打印余额
fn print_balances(scheduler: &ParallelScheduler, accounts: &[&str]) {
    let storage = scheduler.get_storage();
    let storage = storage.lock().unwrap();

    for account in accounts {
        let balance = storage
            .get(account.as_bytes())
            .and_then(|b| String::from_utf8(b.clone()).ok())
            .unwrap_or_else(|| "0".to_string());
        println!("  - {}: {}", account, balance);
    }
}
