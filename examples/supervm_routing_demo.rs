fn main() {// SuperVM 2.0 - SuperVM Routing Demo

    println!("supervm_routing_demo placeholder - build check only");// 架构师: KING XU (CHINA)

}

use vm_runtime::{OwnershipManager, OwnershipType, ObjectMetadata, SuperVM, VmTransaction, Privacy, Address};

fn main() {
    let manager = OwnershipManager::new();
    let supervm = SuperVM::new(&manager);

    let alice: Address = [0xAA; 32];
    let bob: Address = [0xBB; 32];

    // 注册一个独占对象 (owned by Alice)
    let mut obj1 = [0u8; 32]; obj1[0] = 1;
    let meta1 = ObjectMetadata { id: obj1, version: 0, ownership: OwnershipType::Owned(alice), object_type: "Asset::Coin".into(), created_at: 0, updated_at: 0, size: 128, is_deleted: false };
    manager.register_object(meta1).unwrap();

    // 注册一个共享对象
    let mut obj2 = [0u8; 32]; obj2[0] = 2;
    let meta2 = ObjectMetadata { id: obj2, version: 0, ownership: OwnershipType::Shared, object_type: "DEX::Pool".into(), created_at: 0, updated_at: 0, size: 1024, is_deleted: false };
    manager.register_object(meta2).unwrap();

    // 交易 1：Alice 操作自己的对象（公开）→ Fast Path
    let tx1 = VmTransaction { from: alice, objects: vec![obj1], privacy: Privacy::Public };
    let r1 = supervm.execute_transaction(&tx1);
    println!("TX1: {:?}", r1);

    // 交易 2：包含共享对象（公开）→ Consensus Path
    let tx2 = VmTransaction { from: alice, objects: vec![obj1, obj2], privacy: Privacy::Public };
    let r2 = supervm.execute_transaction(&tx2);
    println!("TX2: {:?}", r2);

    // 交易 3：隐私模式 → Private Path
    let tx3 = VmTransaction { from: bob, objects: vec![obj1], privacy: Privacy::Private };
    let r3 = supervm.execute_transaction(&tx3);
    println!("TX3: {:?}", r3);
}
