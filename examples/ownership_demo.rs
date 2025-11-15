// SuperVM 2.0 - Object Ownership Example
// 架构师: KING XU (CHINA)

//
// 演示对象所有权系统的使用

use vm_runtime::{AccessType, ObjectMetadata, OwnershipManager, OwnershipType};

fn main() {
    println!("=== SuperVM 2.0 Object Ownership Demo ===\n");

    let manager = OwnershipManager::new();

    // 创建两个用户
    let alice = [1u8; 32];
    let bob = [2u8; 32];

    println!("👤 Alice: {:?}...", &alice[0..4]);
    println!("👤 Bob:   {:?}...\n", &bob[0..4]);

    // ========================================================================
    // 场景 1: NFT 交易（独占对象 - 快速路径）
    // ========================================================================

    println!("📦 Scenario 1: NFT Transfer (Fast Path)");
    println!("----------------------------------------");

    // Alice 创建一个 NFT
    let mut nft_id = [0u8; 32];
    nft_id[0] = 100;

    let nft = ObjectMetadata {
        id: nft_id,
        version: 0,
        ownership: OwnershipType::Owned(alice),
        object_type: "NFT::CryptoArt".to_string(),
        created_at: 0,
        updated_at: 0,
        size: 1024,
        is_deleted: false,
    };

    manager.register_object(nft.clone()).expect("Register NFT");
    println!("✅ Alice created NFT: {:?}...", &nft_id[0..4]);

    // 验证：Alice 拥有 NFT
    assert!(manager.is_owned_by(&nft_id, &alice));
    println!("✅ Alice owns the NFT");

    // Bob 尝试访问 Alice 的 NFT（读取可以，写入不行）
    assert!(manager
        .verify_access(&nft_id, &bob, AccessType::Read)
        .is_ok());
    assert!(manager
        .verify_access(&nft_id, &bob, AccessType::Write)
        .is_err());
    println!("✅ Bob can read but cannot write");

    // Alice 将 NFT 转给 Bob
    manager
        .transfer_ownership(&nft_id, &alice, &bob)
        .expect("Transfer NFT");
    println!("🔄 Alice transferred NFT to Bob");

    // 验证：现在 Bob 拥有 NFT
    assert!(!manager.is_owned_by(&nft_id, &alice));
    assert!(manager.is_owned_by(&nft_id, &bob));
    println!("✅ Bob now owns the NFT");

    // 路径判断：NFT 交易走快速路径
    assert!(manager.should_use_fast_path(&[nft_id]));
    manager.record_transaction_path(true);
    println!("🚀 Transaction uses FAST PATH (no consensus needed)\n");

    // ========================================================================
    // 场景 2: DEX 流动性池（共享对象 - 共识路径）
    // ========================================================================

    println!("💱 Scenario 2: DEX Liquidity Pool (Consensus Path)");
    println!("---------------------------------------------------");

    // 创建一个共享的流动性池
    let mut pool_id = [0u8; 32];
    pool_id[0] = 200;

    let pool = ObjectMetadata {
        id: pool_id,
        version: 0,
        ownership: OwnershipType::Shared,
        object_type: "DEX::LiquidityPool".to_string(),
        created_at: 0,
        updated_at: 0,
        size: 4096,
        is_deleted: false,
    };

    manager
        .register_object(pool.clone())
        .expect("Register pool");
    println!("✅ Created shared liquidity pool: {:?}...", &pool_id[0..4]);

    // 验证：这是一个共享对象
    assert!(manager.is_shared(&pool_id));
    println!("✅ Pool is a shared object");

    // Alice 和 Bob 都可以访问共享对象
    assert!(manager
        .verify_access(&pool_id, &alice, AccessType::Read)
        .is_ok());
    assert!(manager
        .verify_access(&pool_id, &alice, AccessType::Write)
        .is_ok());
    assert!(manager
        .verify_access(&pool_id, &bob, AccessType::Read)
        .is_ok());
    assert!(manager
        .verify_access(&pool_id, &bob, AccessType::Write)
        .is_ok());
    println!("✅ Both Alice and Bob can read/write");

    // 但不能转移共享对象的所有权
    assert!(manager.transfer_ownership(&pool_id, &alice, &bob).is_err());
    println!("✅ Cannot transfer shared object ownership");

    // 路径判断：DEX 交易走共识路径
    assert!(!manager.should_use_fast_path(&[pool_id]));
    manager.record_transaction_path(false);
    println!("🔗 Transaction uses CONSENSUS PATH (MVCC + BFT)\n");

    // ========================================================================
    // 场景 3: 游戏地图数据（不可变对象 - 超快路径）
    // ========================================================================

    println!("🗺️  Scenario 3: Game Map Data (Immutable Path)");
    println!("------------------------------------------------");

    // Alice 创建游戏地图数据
    let mut map_id = [0u8; 32];
    map_id[0] = 150;

    let map = ObjectMetadata {
        id: map_id,
        version: 0,
        ownership: OwnershipType::Owned(alice),
        object_type: "Game::MapData".to_string(),
        created_at: 0,
        updated_at: 0,
        size: 10240,
        is_deleted: false,
    };

    manager.register_object(map.clone()).expect("Register map");
    println!("✅ Alice created map data: {:?}...", &map_id[0..4]);

    // Alice 将地图冻结为不可变对象
    manager.make_immutable(&map_id, &alice).expect("Freeze map");
    println!("❄️  Alice froze the map as immutable");

    // 验证：现在是不可变对象
    assert!(manager.is_immutable(&map_id));
    println!("✅ Map is now immutable");

    // 任何人都可以读取，但没人能修改
    assert!(manager
        .verify_access(&map_id, &bob, AccessType::Read)
        .is_ok());
    assert!(manager
        .verify_access(&map_id, &alice, AccessType::Write)
        .is_err());
    assert!(manager
        .verify_access(&map_id, &bob, AccessType::Write)
        .is_err());
    println!("✅ Everyone can read, nobody can write");

    // 路径判断：不可变对象走超快路径
    assert!(manager.should_use_fast_path(&[map_id]));
    manager.record_transaction_path(true);
    println!("⚡ Immutable reads: ZERO-COST, NO-LOCK\n");

    // ========================================================================
    // 场景 4: 混合交易（包含共享对象 → 共识路径）
    // ========================================================================

    println!("🔀 Scenario 4: Mixed Transaction (Forced to Consensus Path)");
    println!("------------------------------------------------------------");

    // 一个交易同时访问 NFT 和 DEX 池
    let objects = vec![nft_id, pool_id];

    // 只要包含一个共享对象，就必须走共识路径
    assert!(!manager.should_use_fast_path(&objects));
    manager.record_transaction_path(false);
    println!("✅ Transaction with NFT (owned) + Pool (shared)");
    println!("🔗 Must use CONSENSUS PATH (due to shared object)\n");

    // ========================================================================
    // 统计报告
    // ========================================================================

    println!("📊 Final Statistics");
    println!("-------------------");

    let stats = manager.get_stats();
    println!("Owned objects:     {}", stats.owned_count);
    println!("Shared objects:    {}", stats.shared_count);
    println!("Immutable objects: {}", stats.immutable_count);
    println!("Fast path txs:     {}", stats.fast_path_txs);
    println!("Consensus txs:     {}", stats.consensus_path_txs);
    println!("Transfers:         {}", stats.transfer_count);

    let fast_ratio = manager.get_fast_path_ratio();
    println!("\n🚀 Fast Path Ratio: {:.1}%", fast_ratio * 100.0);

    println!("\n✨ Estimated Performance:");
    println!("  - Fast path:      200K+ TPS, < 1ms latency");
    println!("  - Consensus path: 10-20K TPS, 2-5s latency");
    println!("  - Immutable read: Unlimited TPS, < 0.1ms latency");

    // ========================================================================
    // 预期：70% 交易走快速路径
    // ========================================================================

    println!("\n💡 SuperVM 2.0 Design Goal:");
    println!("  - 70% transactions → Fast Path (NFT, game items, owned assets)");
    println!("  - 30% transactions → Consensus Path (DEX, lending, governance)");
    println!("  - Average network TPS: 170K+ (vs Ethereum 15 TPS = 11,000x faster)");

    println!("\n=== Demo Complete ===");
}
