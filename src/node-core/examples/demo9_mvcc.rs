use vm_runtime::MvccStore;
use std::thread;
use std::sync::Arc;

fn main() {
    println!("=== Demo 9: MVCC 多版本并发控制 ===\n");

    demo_basic_mvcc();
    demo_read_only_fast_path();
    demo_snapshot_isolation();
    demo_write_conflict();
    demo_concurrent_performance();
}

fn demo_basic_mvcc() {
    println!("1️⃣  基础 MVCC 操作");
    let store = MvccStore::new();

    // 事务 1: 写入初始数据
    let mut t1 = store.begin();
    t1.write(b"account_alice".to_vec(), b"1000".to_vec());
    t1.write(b"account_bob".to_vec(), b"500".to_vec());
    let ts1 = t1.commit().expect("t1 commit failed");
    println!("  ✅ T1 提交成功 (ts={}): Alice=1000, Bob=500", ts1);

    // 事务 2: 读取数据
    let mut t2 = store.begin();
    let alice_balance = t2.read(b"account_alice").unwrap();
    let bob_balance = t2.read(b"account_bob").unwrap();
    println!("  📖 T2 读取: Alice={}, Bob={}", 
        String::from_utf8_lossy(&alice_balance),
        String::from_utf8_lossy(&bob_balance));
    println!();
}

fn demo_read_only_fast_path() {
    println!("2️⃣  只读事务快速路径");
    let store = MvccStore::new();

    // 初始化数据
    let mut t0 = store.begin();
    for i in 0..10 {
        t0.write(format!("product_{}", i).into_bytes(), format!("price_{}", i * 100).into_bytes());
    }
    t0.commit().unwrap();
    println!("  💾 初始化 10 个产品");

    // 使用只读事务查询（快速路径）
    let mut ro_txn = store.begin_read_only();
    println!("  🔍 只读事务查询 (is_read_only={})", ro_txn.is_read_only());
    
    for i in 0..5 {
        let key = format!("product_{}", i).into_bytes();
        if let Some(price) = ro_txn.read(&key) {
            println!("     - product_{}: {}", i, String::from_utf8_lossy(&price));
        }
    }

    // 只读事务提交（快速路径，无冲突检测开销）
    let ts = ro_txn.commit().unwrap();
    println!("  ✅ 只读事务提交 (ts={}, 快速路径)", ts);
    println!("     优势: 无需冲突检测、无需分配 commit_ts、无锁争用");
    println!();
}

fn demo_snapshot_isolation() {
    println!("3️⃣  快照隔离演示");
    let store = MvccStore::new();

    // 初始化
    let mut t0 = store.begin();
    t0.write(b"counter".to_vec(), b"0".to_vec());
    t0.commit().unwrap();

    // T1 开启快照
    let mut t1 = store.begin();
    let v1 = t1.read(b"counter").unwrap();
    println!("  📸 T1 快照: counter={}", String::from_utf8_lossy(&v1));

    // T2 更新计数器
    let mut t2 = store.begin();
    t2.write(b"counter".to_vec(), b"10".to_vec());
    t2.commit().unwrap();
    println!("  ✅ T2 提交: counter=10");

    // T1 仍然看到旧值（快照隔离）
    let v1_after = t1.read(b"counter").unwrap();
    println!("  🔍 T1 读取（提交后）: counter={} (仍为旧值)", 
        String::from_utf8_lossy(&v1_after));

    // 新事务 T3 看到新值
    let mut t3 = store.begin();
    let v3 = t3.read(b"counter").unwrap();
    println!("  ✨ T3 读取（新快照）: counter={} (最新值)", 
        String::from_utf8_lossy(&v3));
    println!();
}

fn demo_write_conflict() {
    println!("4️⃣  写写冲突检测");
    let store = MvccStore::new();

    // 初始化账户
    let mut t0 = store.begin();
    t0.write(b"account".to_vec(), b"100".to_vec());
    t0.commit().unwrap();
    println!("  💰 初始余额: 100");

    // T1 和 T2 同时尝试修改同一账户
    let mut t1 = store.begin();
    let mut t2 = store.begin();

    t1.write(b"account".to_vec(), b"150".to_vec());
    t2.write(b"account".to_vec(), b"200".to_vec());

    // T1 先提交成功
    match t1.commit() {
        Ok(ts) => println!("  ✅ T1 提交成功 (ts={}): 150", ts),
        Err(e) => println!("  ❌ T1 提交失败: {}", e),
    }

    // T2 提交失败（写写冲突）
    match t2.commit() {
        Ok(ts) => println!("  ✅ T2 提交成功 (ts={}): 200", ts),
        Err(e) => println!("  ❌ T2 提交失败: {} (预期行为)", e),
    }

    // 验证最终值
    let t3 = store.begin();
    let final_balance = t3.read(b"account").unwrap();
    println!("  🔍 最终余额: {}", String::from_utf8_lossy(&final_balance));
    println!();
}

fn demo_concurrent_performance() {
    println!("5️⃣  并发性能测试");
    let store = MvccStore::new();

    // 初始化 100 个账户
    let mut t0 = store.begin();
    for i in 0..100 {
        t0.write(format!("account_{}", i).into_bytes(), b"1000".to_vec());
    }
    t0.commit().unwrap();
    println!("  💾 初始化 100 个账户");

    // 8 个线程并发读取
    println!("  🔄 启动 8 个线程并发读取...");
    let start = std::time::Instant::now();
    let handles: Vec<_> = (0..8)
        .map(|tid| {
            let store_clone = Arc::clone(&store);
            thread::spawn(move || {
                let mut txn = store_clone.begin();
                let mut sum = 0u64;
                for i in 0..100 {
                    let key = format!("account_{}", i).into_bytes();
                    if let Some(val) = txn.read(&key) {
                        sum += String::from_utf8_lossy(&val).parse::<u64>().unwrap_or(0);
                    }
                }
                sum
            })
        })
        .collect();

    let results: Vec<_> = handles.into_iter().map(|h| h.join().unwrap()).collect();
    let elapsed = start.elapsed();
    
    println!("  ✅ 并发读取完成:");
    println!("     - 总耗时: {:?}", elapsed);
    println!("     - 每线程总额: {:?}", results);
    println!("     - 无锁竞争，性能优秀！");
    
    // 并发写入不同账户
    println!("\n  🔄 启动 8 个线程并发写入不同账户...");
    let start = std::time::Instant::now();
    let handles: Vec<_> = (0..8)
        .map(|tid| {
            let store_clone = Arc::clone(&store);
            thread::spawn(move || {
                let mut txn = store_clone.begin();
                for i in 0..10 {
                    let key = format!("new_account_{}_{}", tid, i).into_bytes();
                    txn.write(key, format!("{}", tid * 100 + i).into_bytes());
                }
                txn.commit()
            })
        })
        .collect();

    let results: Vec<_> = handles.into_iter().map(|h| h.join().unwrap()).collect();
    let elapsed = start.elapsed();
    let success_count = results.iter().filter(|r| r.is_ok()).count();

    println!("  ✅ 并发写入完成:");
    println!("     - 总耗时: {:?}", elapsed);
    println!("     - 成功事务数: {}/8", success_count);
    println!("     - 每键粒度锁，高并发友好！");
    println!();
}
