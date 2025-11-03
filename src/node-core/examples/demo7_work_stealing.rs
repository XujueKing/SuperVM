//! Demo 7: 工作窃取调度器演示
//! 
//! 展示如何使用 WorkStealingScheduler 进行高效的并行任务调度
//! 
//! 运行: cargo run --example demo7_work_stealing

use vm_runtime::{WorkStealingScheduler, Task, ParallelScheduler};
use std::sync::{Arc, Mutex};
use std::time::Instant;

fn main() {
    println!("=== Demo 7: 工作窃取调度器演示 ===\n");
    
    // 示例 1: 基础工作窃取
    demo_basic_work_stealing();
    
    // 示例 2: 优先级调度
    demo_priority_scheduling();
    
    // 示例 3: 大规模任务处理
    demo_large_scale_tasks();
    
    // 示例 4: 与 ParallelScheduler 集成
    demo_integration_with_parallel_scheduler();
}

/// 示例 1: 基础工作窃取
fn demo_basic_work_stealing() {
    println!("📌 示例 1: 基础工作窃取\n");
    
    // 创建工作窃取调度器 (4 个工作线程)
    let scheduler = WorkStealingScheduler::new(Some(4));
    
    // 提交 20 个任务
    let tasks: Vec<Task> = (1..=20)
        .map(|i| Task::new(i, 100))
        .collect();
    
    println!("提交 {} 个任务到工作队列", tasks.len());
    scheduler.submit_tasks(tasks);
    
    // 执行任务
    let start = Instant::now();
    let executed_tasks = Arc::new(Mutex::new(Vec::new()));
    let executed_clone = Arc::clone(&executed_tasks);
    
    let result = scheduler.execute_all(move |tx_id| {
        // 模拟任务处理
        std::thread::sleep(std::time::Duration::from_millis(10));
        executed_clone.lock().unwrap().push(tx_id);
        println!("  ✓ 执行任务 {}", tx_id);
        Ok(())
    });
    
    let duration = start.elapsed();
    
    match result {
        Ok(executed) => {
            println!("\n✅ 成功执行 {} 个任务", executed.len());
            println!("⏱️  总耗时: {:?}", duration);
            println!("⚡ 平均每任务: {:?}", duration / executed.len() as u32);
        }
        Err(e) => println!("❌ 执行失败: {}", e),
    }
    
    println!("\n{}\n", "=".repeat(60));
}

/// 示例 2: 优先级调度
fn demo_priority_scheduling() {
    println!("📌 示例 2: 优先级调度\n");
    
    let scheduler = WorkStealingScheduler::new(Some(2));
    
    // 提交不同优先级的任务
    println!("提交任务:");
    scheduler.submit_task(Task::new(1, 255)); // 高优先级
    println!("  - 任务 1 (优先级: 255 - 高)");
    
    scheduler.submit_task(Task::new(2, 128)); // 中优先级
    println!("  - 任务 2 (优先级: 128 - 中)");
    
    scheduler.submit_task(Task::new(3, 50));  // 低优先级
    println!("  - 任务 3 (优先级: 50 - 低)");
    
    scheduler.submit_task(Task::new(4, 200)); // 高优先级
    println!("  - 任务 4 (优先级: 200 - 高)");
    
    let executed_order = Arc::new(Mutex::new(Vec::new()));
    let order_clone = Arc::clone(&executed_order);
    
    let result = scheduler.execute_all(move |tx_id| {
        order_clone.lock().unwrap().push(tx_id);
        println!("  ✓ 执行任务 {}", tx_id);
        Ok(())
    });
    
    if result.is_ok() {
        let order = executed_order.lock().unwrap();
        println!("\n执行顺序: {:?}", *order);
        println!("✅ 所有任务执行完成");
    }
    
    println!("\n{}\n", "=".repeat(60));
}

/// 示例 3: 大规模任务处理
fn demo_large_scale_tasks() {
    println!("📌 示例 3: 大规模任务处理\n");
    
    let num_tasks = 1000;
    let num_workers = 8;
    
    println!("配置: {} 个工作线程处理 {} 个任务", num_workers, num_tasks);
    
    let scheduler = WorkStealingScheduler::new(Some(num_workers));
    
    // 提交大量任务
    let tasks: Vec<Task> = (1..=num_tasks)
        .map(|i| Task::new(i, (i % 256) as u8))
        .collect();
    
    scheduler.submit_tasks(tasks);
    
    // 执行任务并统计
    let start = Instant::now();
    let success_count = Arc::new(std::sync::atomic::AtomicU64::new(0));
    let success_clone = Arc::clone(&success_count);
    
    let result = scheduler.execute_all(move |tx_id| {
        // 模拟计算密集型任务
        let _ = (0..100).fold(0u64, |acc, i| acc.wrapping_add(i * tx_id));
        success_clone.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        Ok(())
    });
    
    let duration = start.elapsed();
    
    match result {
        Ok(executed) => {
            println!("\n✅ 性能统计:");
            println!("  - 执行任务数: {}", executed.len());
            println!("  - 总耗时: {:?}", duration);
            println!("  - 吞吐量: {:.2} 任务/秒", 
                executed.len() as f64 / duration.as_secs_f64());
            
            // 获取统计
            let stats = scheduler.get_stats();
            println!("\n📊 执行统计:");
            println!("  - 成功: {}", stats.successful_txs);
            println!("  - 失败: {}", stats.failed_txs);
            println!("  - 成功率: {:.2}%", stats.success_rate() * 100.0);
        }
        Err(e) => println!("❌ 执行失败: {}", e),
    }
    
    println!("\n{}\n", "=".repeat(60));
}

/// 示例 4: 与 ParallelScheduler 集成
fn demo_integration_with_parallel_scheduler() {
    println!("📌 示例 4: 与 ParallelScheduler 集成\n");
    
    let ws_scheduler = WorkStealingScheduler::new(Some(4));
    let parallel_scheduler = ws_scheduler.get_scheduler();
    
    // 使用底层 ParallelScheduler 进行状态管理
    println!("模拟账户转账场景:");
    
    // 初始化账户
    {
        let storage = parallel_scheduler.get_storage();
        let mut storage = storage.lock().unwrap();
        storage.insert(b"alice".to_vec(), b"1000".to_vec());
        storage.insert(b"bob".to_vec(), b"500".to_vec());
        storage.insert(b"charlie".to_vec(), b"200".to_vec());
    }
    
    println!("初始余额:");
    print_balances(&parallel_scheduler);
    
    // 提交转账任务
    ws_scheduler.submit_task(Task::new(1, 100)); // Alice -> Bob: 100
    ws_scheduler.submit_task(Task::new(2, 100)); // Bob -> Charlie: 50
    ws_scheduler.submit_task(Task::new(3, 100)); // Charlie -> Alice: 30
    
    println!("\n执行转账:");
    
    let ps_clone = Arc::clone(&parallel_scheduler);
    let result = ws_scheduler.execute_all(move |tx_id| {
        match tx_id {
            1 => transfer(&ps_clone, "alice", "bob", 100),
            2 => transfer(&ps_clone, "bob", "charlie", 50),
            3 => transfer(&ps_clone, "charlie", "alice", 30),
            _ => Ok(()),
        }
    });
    
    match result {
        Ok(_) => {
            println!("\n✅ 所有转账完成");
            println!("\n最终余额:");
            print_balances(&parallel_scheduler);
            
            // 显示统计
            let stats = parallel_scheduler.get_stats();
            println!("\n📊 统计:");
            println!("  - 成功交易: {}", stats.successful_txs);
            println!("  - 失败交易: {}", stats.failed_txs);
            println!("  - 回滚次数: {}", stats.rollback_count);
        }
        Err(e) => println!("❌ 转账失败: {}", e),
    }
    
    println!("\n{}\n", "=".repeat(60));
}

/// 辅助函数: 转账
fn transfer(scheduler: &Arc<ParallelScheduler>, from: &str, to: &str, amount: u64) -> Result<(), String> {
    scheduler.execute_with_snapshot(|manager| {
        let storage = manager.get_storage();
        let mut storage = storage.lock().unwrap();
        
        // 读取余额
        let from_balance: u64 = storage.get(from.as_bytes())
            .and_then(|b| String::from_utf8(b.clone()).ok())
            .and_then(|s| s.parse().ok())
            .unwrap_or(0);
        
        if from_balance < amount {
            return Err(format!("{} 余额不足", from));
        }
        
        let to_balance: u64 = storage.get(to.as_bytes())
            .and_then(|b| String::from_utf8(b.clone()).ok())
            .and_then(|s| s.parse().ok())
            .unwrap_or(0);
        
        // 更新余额
        storage.insert(from.as_bytes().to_vec(), 
            (from_balance - amount).to_string().into_bytes());
        storage.insert(to.as_bytes().to_vec(), 
            (to_balance + amount).to_string().into_bytes());
        
        println!("  ✓ {} -> {}: {} (余额: {} -> {})", 
            from, to, amount, from_balance, from_balance - amount);
        
        Ok(())
    })
}

/// 辅助函数: 打印余额
fn print_balances(scheduler: &Arc<ParallelScheduler>) {
    let storage = scheduler.get_storage();
    let storage = storage.lock().unwrap();
    
    for account in &["alice", "bob", "charlie"] {
        let balance = storage.get(account.as_bytes())
            .and_then(|b| String::from_utf8(b.clone()).ok())
            .unwrap_or_else(|| "0".to_string());
        println!("  - {}: {}", account, balance);
    }
}
