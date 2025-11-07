// SuperVM 2.0 - Fallback Demo (Fast → Consensus)

// 演示：execute_transaction_with() 单笔执行与快速路径回退

use anyhow::Result;
use vm_runtime::{OwnershipManager, OwnershipType, ObjectMetadata, SuperVM, VmTransaction, Privacy, Address, MvccScheduler};

fn main() -> Result<()> {
    let manager = OwnershipManager::new();
    let scheduler = MvccScheduler::new();
    let supervm = SuperVM::new(&manager).with_scheduler(&scheduler);

    let alice: Address = [0xA1; 32];
    let bob: Address = [0xB2; 32];

    // 场景 1：独占对象（Fast 路径成功）
    let owned = make_id(1);
    manager.register_object(ObjectMetadata {
        id: owned,
        version: 0,
        ownership: OwnershipType::Owned(alice),
        object_type: "Asset::Token".into(),
        created_at: 0,
        updated_at: 0,
        size: 64,
        is_deleted: false,
    }).map_err(anyhow::Error::msg)?;

    let tx1 = VmTransaction { from: alice, objects: vec![owned], privacy: Privacy::Public };
    let receipt1 = supervm.execute_transaction_with(1, &tx1, |txn| {
        txn.write(b"balance_alice".to_vec(), b"100".to_vec());
        Ok(100)
    });
    
    println!("\n=== 场景 1: Fast Path Success ===");
    println!("Path: {:?}", receipt1.path);
    println!("Success: {}", receipt1.success);
    println!("Fallback: {}", receipt1.fallback_to_consensus);
    println!("Return: {:?}", receipt1.return_value);
    println!("Latency: {} ms", receipt1.latency_ms);

    // 场景 2：共享对象（直接走 Consensus）
    let shared = make_id(100);
    manager.register_object(ObjectMetadata {
        id: shared,
        version: 0,
        ownership: OwnershipType::Shared,
        object_type: "DEX::Pool".into(),
        created_at: 0,
        updated_at: 0,
        size: 256,
        is_deleted: false,
    }).map_err(anyhow::Error::msg)?;

    let tx2 = VmTransaction { from: alice, objects: vec![shared], privacy: Privacy::Public };
    let receipt2 = supervm.execute_transaction_with(2, &tx2, |txn| {
        let key = b"pool_liquidity".to_vec();
        let val = txn.read(&key)
            .and_then(|v| std::str::from_utf8(v.as_ref()).ok()?.parse::<i32>().ok())
            .unwrap_or(0);
        txn.write(key, (val + 50).to_string().into_bytes());
        Ok(val + 50)
    });

    println!("\n=== 场景 2: Consensus Path (Shared Object) ===");
    println!("Path: {:?}", receipt2.path);
    println!("Success: {}", receipt2.success);
    println!("Fallback: {}", receipt2.fallback_to_consensus);
    println!("Return: {:?}", receipt2.return_value);
    println!("Latency: {} ms", receipt2.latency_ms);

    // 场景 3：模拟 Fast 失败（冲突）并自动回退到 Consensus
    // 通过两个并发交易访问同一独占对象的 key 来制造读写冲突
    let owned2 = make_id(2);
    manager.register_object(ObjectMetadata {
        id: owned2,
        version: 0,
        ownership: OwnershipType::Owned(bob),
        object_type: "Asset::NFT".into(),
        created_at: 0,
        updated_at: 0,
        size: 64,
        is_deleted: false,
    }).map_err(anyhow::Error::msg)?;

    // 预先写入一个值
    supervm.execute_transaction_with(998, &VmTransaction {
        from: bob,
        objects: vec![owned2],
        privacy: Privacy::Public,
    }, |txn| {
        txn.write(b"nft_owner".to_vec(), b"bob".to_vec());
        Ok(0)
    });

    // 尝试读写同一 key（可能引发冲突，取决于调度器并发）
    // 这里简化为单笔，实际 Fast 回退是内部重试逻辑；我们通过强制错误触发回退
    let tx3 = VmTransaction { from: bob, objects: vec![owned2], privacy: Privacy::Public };
    let receipt3 = supervm.execute_transaction_with(3, &tx3, |txn| {
        let key = b"nft_owner".to_vec();
        let current = txn.read(&key)
            .map(|v| String::from_utf8_lossy(v.as_ref()).to_string())
            .unwrap_or_else(|| "".to_string());
        txn.write(key.clone(), format!("{}_v2", current).into_bytes());
        Ok(42)
    });

    println!("\n=== 场景 3: Fast (可能回退到 Consensus) ===");
    println!("Path: {:?}", receipt3.path);
    println!("Success: {}", receipt3.success);
    println!("Fallback: {} (如果 Fast 失败，自动重试 Consensus)", receipt3.fallback_to_consensus);
    println!("Return: {:?}", receipt3.return_value);
    println!("Latency: {} ms", receipt3.latency_ms);

    println!("\n✅ 单笔回退演示完成。");
    Ok(())
}

fn make_id(b0: u8) -> [u8; 32] {
    let mut id = [0u8; 32];
    id[0] = b0;
    id
}
