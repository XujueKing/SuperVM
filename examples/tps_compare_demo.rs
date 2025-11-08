// SuperVM 2.0 - TPS Compare Demo (Fast vs Consensus)

use anyhow::Result;
use std::time::Instant;
use vm_runtime::{Address, MvccScheduler, VmTransaction};
use vm_runtime::{ObjectMetadata, OwnershipManager, OwnershipType, Privacy, SuperVM};

// Type alias for complex transaction tuple
type TxnTuple = (
    u64,
    VmTransaction,
    Box<dyn Fn(&mut vm_runtime::Txn) -> Result<i32> + Send + Sync>,
);

fn main() -> Result<()> {
    // Setup
    let manager = OwnershipManager::new();
    let scheduler = MvccScheduler::new();
    let supervm = SuperVM::new(&manager).with_scheduler(&scheduler);

    // Participants
    let alice: Address = [0xA1; 32];
    let bob: Address = [0xB2; 32];

    // Objects
    let owned_a = make_id(1);
    let owned_b = make_id(2);
    let shared = make_id(100);

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
    manager
        .register_object(ObjectMetadata {
            id: shared,
            version: 0,
            ownership: OwnershipType::Shared,
            object_type: "DEX::Pool".into(),
            created_at: 0,
            updated_at: 0,
            size: 256,
            is_deleted: false,
        })
        .map_err(anyhow::Error::msg)?;

    // Workloads
    let n_fast = 200u64;
    let n_consensus = 200u64;

    // Build fast-path txs (Owned-only)
    let mut fast_txs: Vec<(
        u64,
        VmTransaction,
        Box<dyn Fn(&mut vm_runtime::Txn) -> Result<i32> + Send + Sync>,
    )> = Vec::new();
    for i in 0..n_fast {
        let from = if i % 2 == 0 { alice } else { bob };
        let obj = if i % 2 == 0 { owned_a } else { owned_b };
        fast_txs.push((
            i,
            VmTransaction {
                from,
                objects: vec![obj],
                privacy: Privacy::Public,
            },
            Box::new(move |txn| {
                let key = format!("fast:{}", i).into_bytes();
                let val = txn
                    .read(&key)
                    .and_then(|v| std::str::from_utf8(v.as_ref()).ok()?.parse::<i32>().ok())
                    .unwrap_or(0);
                txn.write(key, (val + 1).to_string().into_bytes());
                Ok(val + 1)
            }),
        ));
    }

    // Build consensus-path txs (Shared)
    let mut cons_txs: Vec<TxnTuple> = Vec::new();
    for i in 0..n_consensus {
        let from = if i % 2 == 0 { alice } else { bob };
        cons_txs.push((
            10_000 + i,
            VmTransaction {
                from,
                objects: vec![shared],
                privacy: Privacy::Public,
            },
            Box::new(move |txn| {
                let key = b"pool_shares".to_vec();
                let val = txn
                    .read(&key)
                    .and_then(|v| std::str::from_utf8(v.as_ref()).ok()?.parse::<i32>().ok())
                    .unwrap_or(0);
                txn.write(key, (val + 1).to_string().into_bytes());
                Ok(val + 1)
            }),
        ));
    }

    // Run fast group
    let t0 = Instant::now();
    let (fast_res, _dummy) = supervm.execute_batch_routed(fast_txs);
    let dt_fast = t0.elapsed();

    // Run consensus group
    let t1 = Instant::now();
    let (_dummy2, cons_res) = supervm.execute_batch_routed(cons_txs);
    let dt_cons = t1.elapsed();

    // Stats
    let fast_ms = dt_fast.as_secs_f64() * 1000.0;
    let cons_ms = dt_cons.as_secs_f64() * 1000.0;
    let fast_tps = if fast_ms > 0.0 {
        (n_fast as f64) / (fast_ms / 1000.0)
    } else {
        0.0
    };
    let cons_tps = if cons_ms > 0.0 {
        (n_consensus as f64) / (cons_ms / 1000.0)
    } else {
        0.0
    };

    println!("\n=== TPS Compare ===");
    println!(
        "Fast:       txs={}, ok={}, conflicts={}, time={:.2} ms, TPS≈{:.0}",
        n_fast, fast_res.successful, fast_res.conflicts, fast_ms, fast_tps
    );
    println!(
        "Consensus:  txs={}, ok={}, conflicts={}, time={:.2} ms, TPS≈{:.0}",
        n_consensus, cons_res.successful, cons_res.conflicts, cons_ms, cons_tps
    );

    Ok(())
}

fn make_id(b0: u8) -> [u8; 32] {
    let mut id = [0u8; 32];
    id[0] = b0;
    id
}
