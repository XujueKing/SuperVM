// Phase 4.3: RocksDB 持久化存储演示
// Developer: king
// Date: 2024-11-07

use anyhow::Result;
use std::time::Instant;
use vm_runtime::{RocksDBConfig, RocksDBStorage, Storage};

fn main() -> Result<()> {
    println!("=== RocksDB 持久化存储演示 ===\n");

    // 1. 基础操作演示
    demo_basic_operations()?;

    // 2. 持久化演示
    demo_persistence()?;

    // 3. 批量写入演示
    demo_batch_write()?;

    // 4. 性能基准
    demo_performance()?;

    Ok(())
}

fn demo_basic_operations() -> Result<()> {
    println!("1️⃣ 基础操作演示");
    println!("---");

    let config = RocksDBConfig {
        path: "./data/demo_basic".to_string(),
        ..Default::default()
    };

    let mut storage = RocksDBStorage::new(config)?;

    // 写入
    storage.set(b"user:alice", b"balance:1000")?;
    storage.set(b"user:bob", b"balance:500")?;
    println!("✅ 写入 2 条记录");

    // 读取
    if let Some(value) = storage.get(b"user:alice")? {
        println!("✅ 读取 alice: {}", String::from_utf8_lossy(&value));
    }

    // 扫描
    let results = storage.scan(b"user:")?;
    println!("✅ 扫描 'user:' 前缀: {} 条记录", results.len());

    // 删除
    storage.delete(b"user:bob")?;
    println!("✅ 删除 bob 记录");

    println!();
    Ok(())
}

fn demo_persistence() -> Result<()> {
    println!("2️⃣ 持久化演示");
    println!("---");

    let db_path = "./data/demo_persist";

    // 第一次打开: 写入数据
    {
        let mut storage = RocksDBStorage::new_with_path(db_path)?;
        storage.set(b"persistent_key", b"this_will_survive_restart")?;
        println!("✅ 写入持久化数据");
    }

    // 第二次打开: 验证数据存在
    {
        let storage = RocksDBStorage::new_with_path(db_path)?;
        if let Some(value) = storage.get(b"persistent_key")? {
            println!("✅ 重启后读取成功: {}", String::from_utf8_lossy(&value));
        }
    }

    println!();
    Ok(())
}

fn demo_batch_write() -> Result<()> {
    println!("3️⃣ 批量写入演示");
    println!("---");

    let config = RocksDBConfig {
        path: "./data/demo_batch".to_string(),
        ..Default::default()
    };

    let storage = RocksDBStorage::new(config)?;

    // 准备批量数据
    let mut batch = Vec::new();
    for i in 0..1000 {
        let key = format!("batch_key_{:04}", i).into_bytes();
        let value = format!("batch_value_{}", i).into_bytes();
        batch.push((key, Some(value)));
    }

    let start = Instant::now();
    storage.write_batch(batch)?;
    let duration = start.elapsed();

    println!("✅ 批量写入 1000 条记录");
    println!("⏱️  耗时: {:?}", duration);
    println!("📊 吞吐量: {:.2} ops/s", 1000.0 / duration.as_secs_f64());

    println!();
    Ok(())
}

fn demo_performance() -> Result<()> {
    println!("4️⃣ 性能基准测试");
    println!("---");

    let config = RocksDBConfig {
        path: "./data/demo_perf".to_string(),
        write_buffer_size: 128 * 1024 * 1024, // 128MB
        block_cache_size: 512 * 1024 * 1024,  // 512MB
        ..Default::default()
    };

    let mut storage = RocksDBStorage::new(config)?;

    // 随机写入基准
    println!("📝 随机写入基准 (10,000 条):");
    let start = Instant::now();
    for i in 0..10_000 {
        let key = format!("perf_key_{:05}", i).into_bytes();
        let value = format!("value_{}", i).into_bytes();
        storage.set(&key, &value)?;
    }
    let duration = start.elapsed();
    let write_qps = 10_000.0 / duration.as_secs_f64();
    println!("   ⏱️  耗时: {:?}", duration);
    println!("   📊 写入 QPS: {:.2}", write_qps);

    // 随机读取基准
    println!("\n📖 随机读取基准 (10,000 条):");
    let start = Instant::now();
    for i in 0..10_000 {
        let key = format!("perf_key_{:05}", i).into_bytes();
        let _ = storage.get(&key)?;
    }
    let duration = start.elapsed();
    let read_qps = 10_000.0 / duration.as_secs_f64();
    println!("   ⏱️  耗时: {:?}", duration);
    println!("   📊 读取 QPS: {:.2}", read_qps);

    // 批量写入基准
    println!("\n📝 批量写入基准 (100,000 条):");
    let mut batch = Vec::new();
    for i in 0..100_000 {
        let key = format!("batch_perf_{:06}", i).into_bytes();
        let value = format!("value_{}", i).into_bytes();
        batch.push((key, Some(value)));
    }
    let start = Instant::now();
    storage.write_batch(batch)?;
    let duration = start.elapsed();
    let batch_qps = 100_000.0 / duration.as_secs_f64();
    println!("   ⏱️  耗时: {:?}", duration);
    println!("   📊 批量 QPS: {:.2}", batch_qps);

    // 扫描基准
    println!("\n🔍 扫描基准:");
    let start = Instant::now();
    let results = storage.scan(b"perf_key_")?;
    let duration = start.elapsed();
    println!("   ⏱️  耗时: {:?}", duration);
    println!("   📊 扫描结果: {} 条记录", results.len());

    // 性能总结
    println!("\n📊 性能总结:");
    println!("   ✅ 随机写入: {:.2} ops/s", write_qps);
    println!("   ✅ 随机读取: {:.2} ops/s", read_qps);
    println!("   ✅ 批量写入: {:.2} ops/s", batch_qps);

    // 目标对比
    println!("\n🎯 目标达成情况:");
    let read_target = 100_000.0;
    let batch_target = 200_000.0;
    println!(
        "   读取目标: 100K ops/s, 实际: {:.2}K ops/s ({})",
        read_qps / 1000.0,
        if read_qps >= read_target {
            "✅ 达成"
        } else {
            "❌ 未达成"
        }
    );
    println!(
        "   批量写入目标: 200K ops/s, 实际: {:.2}K ops/s ({})",
        batch_qps / 1000.0,
        if batch_qps >= batch_target {
            "✅ 达成"
        } else {
            "❌ 未达成"
        }
    );

    println!();
    Ok(())
}
