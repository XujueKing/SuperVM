// SPDX-License-Identifier: GPL-3.0-or-later
// Smoke tests for SuperVM routing and basic execution

mod test_helpers;
use test_helpers::*;
use vm_runtime::supervm::ExecutionPath;

#[test]
fn route_owned_public_goes_fast() {
    let ctx = VmTestContext::new();
    let a1 = addr(1);
    let o1 = obj(1);
    ctx.register_owned(o1, a1);

    let vm = ctx.build_vm();
    let t = tx(a1, vec![o1], vm_runtime::supervm::Privacy::Public);
    let path = vm.route(&t);
    assert!(matches!(path, ExecutionPath::FastPath));
}

#[test]
fn route_shared_public_goes_consensus() {
    let ctx = VmTestContext::new();
    let a1 = addr(1);
    let o2 = obj(2);
    ctx.register_shared(o2);

    let vm = ctx.build_vm();
    let t = tx(a1, vec![o2], vm_runtime::supervm::Privacy::Public);
    let path = vm.route(&t);
    assert!(matches!(path, ExecutionPath::ConsensusPath));
}

#[test]
fn route_private_goes_privacy() {
    let ctx = VmTestContext::new();
    let a1 = addr(1);
    let o3 = obj(3);
    ctx.register_owned(o3, a1);

    let vm = ctx.build_vm();
    let t = tx(a1, vec![o3], vm_runtime::supervm::Privacy::Private);
    let path = vm.route(&t);
    assert!(matches!(path, ExecutionPath::PrivatePath));
}

#[test]
fn execute_routed_fast_success_and_metrics() {
    let ctx = VmTestContext::new();
    let a1 = addr(1);
    let o4 = obj(4);
    ctx.register_owned(o4, a1);
    let vm = ctx.build_vm();

    // Fast op returns value 42
    let t = tx(a1, vec![o4], vm_runtime::supervm::Privacy::Public);
    let receipt = vm.execute_transaction_routed(1, &t, || Ok(42), |txn| write_key(txn, b"k", b"v"));

    assert!(matches!(receipt.path, ExecutionPath::FastPath));
    assert!(receipt.success);
    assert_eq!(receipt.return_value, Some(42));

    // Routing counters export should include fast total >= 1
    let metrics = vm.export_routing_prometheus();
    assert!(metrics.contains("vm_routing_fast_total"));
}

#[test]
fn execute_routed_fast_fail_then_fallback_increments_metric() {
    let ctx = VmTestContext::new();
    let a1 = addr(1);
    let a2 = addr(2);
    let o5 = obj(5);
    ctx.register_owned(o5, a1);
    let vm = ctx.build_vm()
        .with_fallback(true)
        .with_fallback_whitelist(vec!["not owned", "Object"]); // include substring from error

    // Sender a2 does not own o5 => fast path will fail, fallback to consensus
    let t = tx(a2, vec![o5], vm_runtime::supervm::Privacy::Public);
    let receipt = vm.execute_transaction_routed(2, &t, || Ok(1), |txn| write_key(txn, b"k2", b"v2"));

    assert!(matches!(receipt.path, ExecutionPath::ConsensusPath));
    assert!(receipt.fallback_to_consensus);
    assert!(receipt.success);

    // Check mvcc metrics for fallback counter
    let prom = ctx.scheduler.store().get_metrics().unwrap().export_prometheus();
    assert!(prom.contains("vm_fast_fallback_total 1"));
}
