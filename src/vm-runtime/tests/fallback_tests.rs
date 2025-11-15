// SPDX-License-Identifier: GPL-3.0-or-later
// Fallback behavior tests for SuperVM fast->consensus path

use vm_runtime::supervm::{SuperVM, Transaction, Privacy, ExecutionPath};
use vm_runtime::{OwnershipManager, OwnershipType, ObjectMetadata};
use vm_runtime::parallel_mvcc::MvccScheduler;
use vm_runtime::mvcc::Txn;

type Address = [u8;32];
type ObjectId = [u8;32];

fn addr(id: u8) -> Address { let mut a = [0u8;32]; a[0]=id; a }
fn obj(id: u8) -> ObjectId { let mut o = [0u8;32]; o[0]=id; o }

#[test]
fn fast_path_failure_whitelisted_fallback_to_consensus() {
    // Setup ownership: object owned by address 1
    let ownership = OwnershipManager::new();
    let meta = ObjectMetadata {
        id: obj(10),
        version: 0,
        ownership: OwnershipType::Owned(addr(1)),
        object_type: "Test".into(),
        created_at: 0,
        updated_at: 0,
        size: 0,
        is_deleted: false,
    };
    ownership.register_object(meta).expect("register object");

    // Scheduler for consensus path
    let scheduler = MvccScheduler::new();

    // SuperVM with fallback enabled and whitelist matching error substring
    let vm = SuperVM::new(&ownership)
        .with_scheduler(&scheduler)
        .with_fallback(true)
        .with_fallback_whitelist(vec!["not owned"]); // match error "not owned"

    // Transaction from different address -> fast path route but execute_fast_path will fail
    let tx = Transaction { from: addr(2), objects: vec![obj(10)], privacy: Privacy::Public };

    // fast_op returns Ok but ownership check triggers earlier failure; we keep simple op
    let receipt = vm.execute_transaction_routed(100, &tx, || Ok(1), |txn: &mut Txn| {
        txn.write(b"k".to_vec(), b"v".to_vec());
        Ok(7)
    });

    assert!(matches!(receipt.path, ExecutionPath::ConsensusPath), "Should end on consensus path after fallback");
    assert!(receipt.fallback_to_consensus, "Fallback flag should be true");
    assert!(receipt.success, "Consensus execution should succeed");
    assert_eq!(receipt.return_value, Some(7));

    // Metrics should record one fallback
    let metrics_text = scheduler.store().get_metrics().unwrap().export_prometheus();
    assert!(metrics_text.contains("vm_fast_fallback_total 1"), "Fallback metric should be incremented to 1");
}

#[test]
fn fast_path_failure_not_whitelisted_no_fallback() {
    let ownership = OwnershipManager::new();
    let meta = ObjectMetadata {
        id: obj(11),
        version: 0,
        ownership: OwnershipType::Owned(addr(1)),
        object_type: "Test".into(),
        created_at: 0,
        updated_at: 0,
        size: 0,
        is_deleted: false,
    };
    ownership.register_object(meta).expect("register object");
    let scheduler = MvccScheduler::new();

    // Whitelist does NOT contain substring -> fallback disallowed
    let vm = SuperVM::new(&ownership)
        .with_scheduler(&scheduler)
        .with_fallback(true)
        .with_fallback_whitelist(vec!["Conflict"]); // does not match "not owned"

    let tx = Transaction { from: addr(2), objects: vec![obj(11)], privacy: Privacy::Public };

    let receipt = vm.execute_transaction_routed(101, &tx, || Ok(1), |txn: &mut Txn| {
        txn.write(b"k2".to_vec(), b"v2".to_vec());
        Ok(9)
    });

    assert!(matches!(receipt.path, ExecutionPath::FastPath), "Receipt path stays fast path when no fallback");
    assert!(!receipt.fallback_to_consensus, "Fallback flag should be false");
    assert!(!receipt.success, "Fast path failure without fallback should not succeed");
    assert!(receipt.return_value.is_none());

    let metrics_text = scheduler.store().get_metrics().unwrap().export_prometheus();
    assert!(metrics_text.contains("vm_fast_fallback_total 0"), "Fallback metric should remain 0");
}
