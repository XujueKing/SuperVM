// SuperVM 2.0 - Routed Batch Execution Demo

use vm_runtime::{OwnershipManager, OwnershipType, ObjectMetadata, SuperVM, Privacy};
use vm_runtime::{MvccScheduler, VmTransaction, Address};
use anyhow::Result;

fn main() -> Result<()> {
    // 1) 初始化组件
    let manager = OwnershipManager::new();
    let scheduler = MvccScheduler::new();
    let supervm = SuperVM::new(&manager).with_scheduler(&scheduler);

    // 2) 创建对象
    let alice: Address = [0x11; 32];
    let bob: Address = [0x22; 32];

    let mut owned1 = [0u8; 32]; owned1[0] = 10;
    let mut owned2 = [0u8; 32]; owned2[0] = 11;
    let mut shared = [0u8; 32]; shared[0] = 20;

    manager.register_object(ObjectMetadata{ id: owned1, version:0, ownership: OwnershipType::Owned(alice), object_type:"Asset::Coin".into(), created_at:0, updated_at:0, size:64, is_deleted:false }).unwrap();
    manager.register_object(ObjectMetadata{ id: owned2, version:0, ownership: OwnershipType::Owned(bob), object_type:"Game::Item".into(), created_at:0, updated_at:0, size:64, is_deleted:false }).unwrap();
    manager.register_object(ObjectMetadata{ id: shared, version:0, ownership: OwnershipType::Shared, object_type:"DEX::Pool".into(), created_at:0, updated_at:0, size:256, is_deleted:false }).unwrap();

    // 3) 构造批量交易（包含 fast 与 consensus）
    let mut txs: Vec<(u64, VmTransaction, Box<dyn Fn(&mut vm_runtime::Txn) -> Result<i32> + Send + Sync>)> = Vec::new();

    // Fast: Alice 操作自己对象（加 1）
    txs.push((1, VmTransaction{ from: alice, objects: vec![owned1], privacy: Privacy::Public }, Box::new(|txn| {
        let key = b"alice_counter".to_vec();
        let val = txn.read(&key).and_then(|v| std::str::from_utf8(v.as_ref()).ok()?.parse::<i32>().ok()).unwrap_or(0);
        txn.write(key, (val+1).to_string().into_bytes());
        Ok(val+1)
    })));

    // Fast: Bob 操作自己对象（加 2）
    txs.push((2, VmTransaction{ from: bob, objects: vec![owned2], privacy: Privacy::Public }, Box::new(|txn| {
        let key = b"bob_counter".to_vec();
        let val = txn.read(&key).and_then(|v| std::str::from_utf8(v.as_ref()).ok()?.parse::<i32>().ok()).unwrap_or(0);
        txn.write(key, (val+2).to_string().into_bytes());
        Ok(val+2)
    })));

    // Consensus: 访问共享对象（池子份额 +1）
    txs.push((3, VmTransaction{ from: alice, objects: vec![shared], privacy: Privacy::Public }, Box::new(|txn| {
        let key = b"pool_shares".to_vec();
        let val = txn.read(&key).and_then(|v| std::str::from_utf8(v.as_ref()).ok()?.parse::<i32>().ok()).unwrap_or(0);
        txn.write(key, (val+1).to_string().into_bytes());
        Ok(val+1)
    })));

    // 4) 路由并执行
    let (fast_result, consensus_result) = supervm.execute_batch_routed(txs);

    println!("Fast: success={}, failed={}, conflicts={}", fast_result.successful, fast_result.failed, fast_result.conflicts);
    println!("Consensus: success={}, failed={}, conflicts={}", consensus_result.successful, consensus_result.failed, consensus_result.conflicts);

    Ok(())
}
