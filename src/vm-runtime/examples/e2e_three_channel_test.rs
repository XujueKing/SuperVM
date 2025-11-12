// SPDX-License-Identifier: GPL-3.0-or-later
// E2E test for three-channel routing: Fast (Owned), Consensus (Shared), Private (Mock)
// Run: cargo run --example e2e_three_channel_test --release

use vm_runtime::{SuperVM, OwnershipManager, OwnershipType, ObjectMetadata};
use vm_runtime::{Address, ObjectId, Privacy, VmTransaction as Transaction, MvccScheduler};
use vm_runtime::Txn;

fn mk_addr(v: u8) -> Address { [v; 32] }
fn mk_id(v: u64) -> ObjectId { let mut id = [0u8;32]; id[0..8].copy_from_slice(&v.to_le_bytes()); id }

fn main() {
    println!("=== E2E Three-Channel Test ===\n");

    // 1) 准备对象与所有权
    let manager = OwnershipManager::new();
    let alice = mk_addr(0xAA);
    let bob = mk_addr(0xBB);

    // Owned NFT 属于 Alice（快速通道）
    let nft_id = mk_id(100);
    manager.register_object(ObjectMetadata {
        id: nft_id,
        version: 0,
        ownership: OwnershipType::Owned(alice),
        object_type: "NFT::Collectible".into(),
        created_at: 0,
        updated_at: 0,
        size: 128,
        is_deleted: false,
    }).unwrap();

    // 共享 DeFi 池（共识通道）
    let pool_id = mk_id(200);
    manager.register_object(ObjectMetadata {
        id: pool_id,
        version: 0,
        ownership: OwnershipType::Shared,
        object_type: "DeFi::Pool".into(),
        created_at: 0,
        updated_at: 0,
        size: 2048,
        is_deleted: false,
    }).unwrap();

    // 私密账户（隐私通道，当前走验证占位+共识执行）
    let priv_id = mk_id(300);
    manager.register_object(ObjectMetadata {
        id: priv_id,
        version: 0,
        ownership: OwnershipType::Owned(bob), // 所有者 Bob，但交易使用 Privacy::Private
        object_type: "Privacy::Account".into(),
        created_at: 0,
        updated_at: 0,
        size: 512,
        is_deleted: false,
    }).unwrap();

    let scheduler = MvccScheduler::new();
    let vm = SuperVM::new(&manager).with_scheduler(&scheduler);

    // 2) Fast: Alice 操作自己的 NFT（应走 FastPath）
    let tx_fast = Transaction { from: alice, objects: vec![nft_id], privacy: Privacy::Public };
    let r_fast = vm.execute_transaction_routed(1, &tx_fast, || Ok(1), |txn: &mut Txn| {
        txn.write(b"fast_key".to_vec(), b"v".to_vec());
        Ok(1)
    });
    println!("Fast Receipt: {:?}", r_fast);

    // 3) Consensus: 操作共享池（应走 ConsensusPath）
    let tx_cons = Transaction { from: alice, objects: vec![nft_id, pool_id], privacy: Privacy::Public };
    let r_cons = vm.execute_transaction_routed(2, &tx_cons, || Ok(0), |txn: &mut Txn| {
        txn.write(b"pool_key".to_vec(), b"v".to_vec());
        let _ = txn.read(b"pool_key");
        Ok(2)
    });
    println!("Consensus Receipt: {:?}", r_cons);

    // 4) Private: Bob 发起隐私转账（应走 PrivatePath + ZK 验证占位通过）
    let tx_priv = Transaction { from: bob, objects: vec![priv_id], privacy: Privacy::Private };
    let r_priv = vm.execute_transaction_routed(3, &tx_priv, || Ok(7), |txn: &mut Txn| {
        txn.write(b"priv_key".to_vec(), b"v".to_vec());
        Ok(3)
    });
    println!("Private Receipt: {:?}", r_priv);

    // 5) 汇总统计
    let routing = vm.routing_stats();
    println!("Routing: fast={} consensus={} privacy={} (ratio: {:.2}/{:.2}/{:.2})",
        routing.fast_path_count, routing.consensus_path_count, routing.privacy_path_count,
        routing.fast_path_ratio(), routing.consensus_path_ratio(), routing.privacy_path_ratio());

    // 简单断言（非严格）
    assert!(r_fast.success && matches!(r_fast.path, vm_runtime::ExecutionPath::FastPath));
    assert!(r_cons.success && matches!(r_cons.path, vm_runtime::ExecutionPath::ConsensusPath));
    assert!(r_priv.success && matches!(r_priv.path, vm_runtime::ExecutionPath::PrivatePath));

    println!("\nE2E test passed.");
}
