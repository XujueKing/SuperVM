// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2025 XujueKing <leadbrand@me.com>

//! RocksDB Checkpoint 管理器演示
//! 
//! 功能:
//! - 自动检查点创建 (每 N 区块)
//! - 检查点清理 (保留最新 M 个)
//! - 检查点恢复
//! - 检查点列表

use anyhow::Result;
use std::sync::Arc;
use vm_runtime::storage::{
    RocksDBStorage, 
    RocksDBConfig, 
    CheckpointManager, 
    CheckpointManagerConfig,
    Storage,
};

fn main() -> Result<()> {
    println!("🚀 RocksDB Checkpoint Manager Demo\n");
    
    // 1. 创建 RocksDB 存储
    println!("1️⃣  创建 RocksDB 存储...");
    let config = RocksDBConfig::default().with_path("./data/demo_checkpoint");
    let storage = Arc::new(RocksDBStorage::new(config)?);
    println!("   ✅ 存储已创建: ./data/demo_checkpoint\n");
    
    // 2. 配置检查点管理器
    println!("2️⃣  配置检查点管理器...");
    let mut checkpoint_config = CheckpointManagerConfig::default();
    checkpoint_config.checkpoints_dir = "./data/demo_checkpoints".to_string();
    checkpoint_config.block_interval = 10;   // 每 10 区块创建一次
    checkpoint_config.max_checkpoints = 5;   // 最多保留 5 个
    
    let manager = CheckpointManager::new(storage.clone(), checkpoint_config);
    println!("   ✅ 检查点管理器已配置:");
    println!("      - 检查点目录: ./data/demo_checkpoints");
    println!("      - 区块间隔: 每 10 区块");
    println!("      - 最大保留: 5 个\n");
    
    // 3. 模拟区块链运行并创建检查点
    println!("3️⃣  模拟区块链运行...");
    for block_number in 0..=50 {
        // 写入一些数据
        let key = format!("block_{:04}", block_number);
        let value = format!("data_at_block_{}", block_number);
        
        // 使用 Arc 克隆来获取可变引用
        let storage_clone = storage.clone();
        let storage_mut = unsafe { &mut *(Arc::as_ptr(&storage_clone) as *mut RocksDBStorage) };
        storage_mut.set(key.as_bytes(), value.as_bytes())?;
        
        // 检查是否需要创建检查点
        if manager.should_checkpoint(block_number) {
            let checkpoint_name = manager.create_checkpoint(block_number)?;
            println!("   📸 区块 {} - 创建检查点: {}", block_number, checkpoint_name);
        }
    }
    println!();
    
    // 4. 列出所有检查点
    println!("4️⃣  列出所有检查点:");
    let checkpoints = manager.list_checkpoints()?;
    for (i, checkpoint) in checkpoints.iter().enumerate() {
        println!("   {}. {}", i + 1, checkpoint);
    }
    println!("   📊 总计: {} 个检查点\n", checkpoints.len());
    
    // 5. 从检查点恢复
    if let Some(latest_checkpoint) = checkpoints.last() {
        println!("5️⃣  从最新检查点恢复...");
        println!("   检查点: {}", latest_checkpoint);
        
        let restored_storage = manager.restore_checkpoint(
            latest_checkpoint,
            "./data/demo_restored"
        )?;
        
        // 验证恢复的数据
        let test_key = b"block_0050";
        if let Some(value) = restored_storage.get(test_key)? {
            println!("   ✅ 恢复成功! 验证数据: {} = {}", 
                String::from_utf8_lossy(test_key),
                String::from_utf8_lossy(&value)
            );
        }
        println!("   恢复位置: ./data/demo_restored\n");
    }
    
    // 6. 统计信息
    println!("6️⃣  统计信息:");
    if let Some(stats) = storage.get_property("rocksdb.estimate-num-keys") {
        println!("   键数量: {}", stats);
    }
    if let Some(stats) = storage.get_property("rocksdb.total-sst-files-size") {
        let size_mb = stats.parse::<f64>().unwrap_or(0.0) / 1024.0 / 1024.0;
        println!("   SST 文件大小: {:.2} MB", size_mb);
    }
    
    println!("\n✨ Demo 完成!");
    println!("\n💡 提示:");
    println!("   - 检查点保存在: ./data/demo_checkpoints/");
    println!("   - 恢复的数据在: ./data/demo_restored/");
    println!("   - 运行多次将看到自动清理旧检查点");
    
    Ok(())
}
