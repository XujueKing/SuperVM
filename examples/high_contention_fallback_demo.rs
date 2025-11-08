// SuperVM 2.0 - High Contention Fallback Demo

// 目的：构造高竞争场景，强制触发 Fast Path 失败并验证自动回退机制

use anyhow::Result;
use std::sync::Arc;
use vm_runtime::{
    Address, MvccScheduler, ObjectMetadata, OwnershipManager, OwnershipType, Privacy, SuperVM,
    VmTransaction,
};

// Type alias for complex transaction tuple
type TxnTuple = (
    u64,
    VmTransaction,
    Arc<dyn Fn(&mut vm_runtime::Txn) -> Result<i32> + Send + Sync>,
);

fn main() -> Result<()> {
    let manager = OwnershipManager::new();
    let scheduler = MvccScheduler::new();
    let supervm = SuperVM::new(&manager).with_scheduler(&scheduler);

    let alice: Address = [0xA1; 32];
    let bob: Address = [0xB2; 32];

    // 创建独占对象（Alice 和 Bob 各一个）
    let owned_a = make_id(1);
    let owned_b = make_id(2);

    manager
        .register_object(ObjectMetadata {
            id: owned_a,
            version: 0,
            ownership: OwnershipType::Owned(alice),
            object_type: "Asset::Wallet".into(),
            created_at: 0,
            updated_at: 0,
            size: 64,
            is_deleted: false,
        })
        .map_err(anyhow::Error::msg)?;

    manager
        .register_object(ObjectMetadata {
            id: owned_b,
            version: 0,
            ownership: OwnershipType::Owned(bob),
            object_type: "Asset::Wallet".into(),
            created_at: 0,
            updated_at: 0,
            size: 64,
            is_deleted: false,
        })
        .map_err(anyhow::Error::msg)?;

    println!("=== 高竞争场景：多笔交易写同一 Key ===\n");

    // 构造 100 笔交易，都写同一个共享 key（"global_counter"）
    // 即使对象是独占的，但所有交易访问同一 key 会引发 MVCC 冲突
    let n = 100;
    let mut txs: Vec<TxnTuple> = Vec::new();

    for i in 0..n {
        let from = if i % 2 == 0 { alice } else { bob };
        let obj = if i % 2 == 0 { owned_a } else { owned_b };

        txs.push((
            i,
            VmTransaction {
                from,
                objects: vec![obj],
                privacy: Privacy::Public,
            },
            Arc::new(move |txn| {
                // 所有交易读写同一个全局计数器 key
                let key = b"global_counter".to_vec();
                let val = txn
                    .read(&key)
                    .and_then(|v| std::str::from_utf8(v.as_ref()).ok()?.parse::<i32>().ok())
                    .unwrap_or(0);
                txn.write(key, (val + 1).to_string().into_bytes());
                Ok(val + 1)
            }),
        ));
    }

    // 使用统一批量执行（带回退）
    let (fast_res, cons_res, fallbacks) = supervm.execute_batch(txs);

    println!("执行结果：");
    println!(
        "  Fast:       routed={}, ok={}, failed={}, conflicts={}",
        n, fast_res.successful, fast_res.failed, fast_res.conflicts
    );
    println!(
        "  Consensus:  ok={}, failed={}, conflicts={}",
        cons_res.successful, cons_res.failed, cons_res.conflicts
    );
    println!("  **Fallbacks: fast→consensus={}**", fallbacks);

    // 验证最终值
    let final_val = scheduler.read_only(|txn| {
        Ok(txn
            .read(b"global_counter")
            .and_then(|v| std::str::from_utf8(v.as_ref()).ok()?.parse::<i32>().ok())
            .unwrap_or(0))
    })?;

    println!("\n最终计数器值: {}", final_val);
    println!("期望值: {}", n);

    if fallbacks > 0 {
        println!(
            "\n✅ 回退机制触发成功！{} 笔交易从 Fast 回退到 Consensus",
            fallbacks
        );
    } else {
        println!("\n⚠️  当前场景未触发回退（Fast Path 全部成功或直接路由到 Consensus）");
    }

    Ok(())
}

fn make_id(b0: u8) -> [u8; 32] {
    let mut id = [0u8; 32];
    id[0] = b0;
    id
}
