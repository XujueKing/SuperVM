// SuperVM 2.0 - Mixed Workload Test (70% Fast + 30% Consensus)
// Phase 1.3: 测试混合路径,输出分层统计与路由比例
// 架构师: KING XU (CHINA)

use anyhow::Result;
use std::sync::Arc;
use std::time::Instant;
use vm_runtime::{MemoryStorage, ObjectMetadata, OwnershipType, Privacy, Runtime, VmTransaction};

// Type alias for complex transaction tuple
type TxnTuple = (
    u64,
    VmTransaction,
    Arc<dyn Fn(&mut vm_runtime::Txn) -> Result<i32> + Send + Sync>,
);

fn main() -> Result<()> {
    println!("=== SuperVM 2.0 - Mixed Workload Test ===");
    println!("配置: 70% Fast Path + 30% Consensus Path\n");

    // 使用带路由能力的 Runtime
    let runtime = Runtime::new_with_routing(MemoryStorage::new());
    let ownership = runtime.ownership_manager().unwrap();
    let _scheduler = runtime.scheduler().unwrap();

    // 创建对象
    let num_owned = 700; // 用于 Fast
    let num_shared = 30; // 用于 Consensus

    println!(
        "初始化 {} 个独占对象, {} 个共享对象...",
        num_owned, num_shared
    );

    for i in 0..num_owned {
        let owner = [(i % 256) as u8; 32];
        let obj_id = make_id(i as u64);
        ownership
            .register_object(ObjectMetadata {
                id: obj_id,
                version: 0,
                ownership: OwnershipType::Owned(owner),
                object_type: "Asset::Token".into(),
                created_at: 0,
                updated_at: 0,
                size: 64,
                is_deleted: false,
            })
            .map_err(anyhow::Error::msg)?;
    }

    for i in 0..num_shared {
        let obj_id = make_id(10000 + i as u64);
        ownership
            .register_object(ObjectMetadata {
                id: obj_id,
                version: 0,
                ownership: OwnershipType::Shared,
                object_type: "DEX::Pool".into(),
                created_at: 0,
                updated_at: 0,
                size: 256,
                is_deleted: false,
            })
            .map_err(anyhow::Error::msg)?;
    }

    // 构造混合工作负载：7000 Fast + 3000 Consensus
    let n_fast = 7000u64;
    let n_consensus = 3000u64;
    let total = n_fast + n_consensus;

    println!(
        "构造 {} 笔交易 (Fast={}, Consensus={})...\n",
        total, n_fast, n_consensus
    );

    let mut txs: Vec<TxnTuple> = Vec::new();

    // Fast 交易（分散 key，低竞争）
    for i in 0..n_fast {
        let obj_idx = (i % num_owned as u64) as usize;
        let owner = [(obj_idx % 256) as u8; 32];
        let obj_id = make_id(obj_idx as u64);

        txs.push((
            i,
            VmTransaction {
                from: owner,
                objects: vec![obj_id],
                privacy: Privacy::Public,
            },
            Arc::new(move |txn| {
                let key = format!("fast_{}", i).into_bytes();
                txn.write(key, format!("val_{}", i).into_bytes());
                Ok(i as i32)
            }),
        ));
    }

    // Consensus 交易（高竞争，每个 pool ~100 笔）
    for i in 0..n_consensus {
        let pool_idx = (i % num_shared as u64) as usize;
        let obj_id = make_id(10000 + pool_idx as u64);
        let sender = [(i % 256) as u8; 32];

        txs.push((
            100_000 + i,
            VmTransaction {
                from: sender,
                objects: vec![obj_id],
                privacy: Privacy::Public,
            },
            Arc::new(move |txn| {
                let key = format!("pool_{}_counter", pool_idx).into_bytes();
                let val = txn
                    .read(&key)
                    .and_then(|v| std::str::from_utf8(v.as_ref()).ok()?.parse::<i32>().ok())
                    .unwrap_or(0);
                txn.write(key, (val + 1).to_string().into_bytes());
                Ok(val + 1)
            }),
        ));
    }

    println!("开始执行 {} 笔交易...\n", txs.len());

    // 执行
    let t0 = Instant::now();
    let (fast_res, cons_res, fallbacks) = runtime.execute_batch_with_routing(txs)?;
    let dt = t0.elapsed();

    // 统计
    let total_ms = dt.as_secs_f64() * 1000.0;
    let total_success = fast_res.successful + cons_res.successful;
    let overall_tps = if total_ms > 0.0 {
        (total_success as f64) / (total_ms / 1000.0)
    } else {
        0.0
    };

    println!("=== 执行结果 ===\n");

    println!("Fast Path:");
    println!(
        "  路由: {} ({:.1}%)",
        n_fast,
        (n_fast as f64 / total as f64) * 100.0
    );
    println!("  成功: {}", fast_res.successful);
    println!("  失败: {}", fast_res.failed);
    println!("  冲突: {}", fast_res.conflicts);
    println!(
        "  成功率: {:.2}%",
        (fast_res.successful as f64 / n_fast as f64) * 100.0
    );

    println!("\nConsensus Path:");
    println!(
        "  路由: {} ({:.1}%)",
        n_consensus,
        (n_consensus as f64 / total as f64) * 100.0
    );
    println!("  成功: {}", cons_res.successful);
    println!("  失败: {}", cons_res.failed);
    println!("  冲突: {}", cons_res.conflicts);
    println!(
        "  成功率: {:.2}%",
        (cons_res.successful as f64 / n_consensus as f64) * 100.0
    );

    println!("\n回退统计:");
    println!("  Fast→Consensus: {}", fallbacks);

    println!("\n性能指标:");
    println!("  总耗时: {:.2} ms", total_ms);
    println!("  总成功: {} / {}", total_success, total);
    println!("  总 TPS: {:.0}", overall_tps);
    println!(
        "  总成功率: {:.2}%",
        (total_success as f64 / total as f64) * 100.0
    );

    // 路由统计
    if let Some(stats) = runtime.get_routing_stats() {
        println!("\n所有权统计:");
        println!("  独占对象: {}", stats.owned_count);
        println!("  共享对象: {}", stats.shared_count);
        println!("  不可变对象: {}", stats.immutable_count);
        println!("  快速路径交易: {}", stats.fast_path_txs);
        println!("  共识路径交易: {}", stats.consensus_path_txs);
        let total_routed = stats.fast_path_txs + stats.consensus_path_txs;
        if total_routed > 0 {
            println!(
                "  快速路径占比: {:.1}%",
                (stats.fast_path_txs as f64 / total_routed as f64) * 100.0
            );
        }
    }

    #[cfg(debug_assertions)]
    println!("\n⚠️  Debug 构建。使用 `cargo run --release --example mixed_workload_test` 获取生产级性能。");

    #[cfg(not(debug_assertions))]
    println!("\n✅ Release 构建，生产级性能数据");

    Ok(())
}

fn make_id(n: u64) -> [u8; 32] {
    let mut id = [0u8; 32];
    id[0..8].copy_from_slice(&n.to_le_bytes());
    id
}
