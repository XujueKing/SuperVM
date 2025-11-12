#![cfg(feature = "hybrid-exec")]
use vm_runtime::{Runtime, MemoryStorage};

#[test]
fn hybrid_exec_basic_correctness() {
    let mut rt: Runtime<MemoryStorage> = Runtime::new_with_routing(MemoryStorage::default());
    rt.init_hybrid();
    let ops: Vec<(u64, Box<dyn Fn() -> i32 + Send + Sync>)> = (0..32u64)
        .map(|i| (i, Box::new(move || (i as i32) * 2) as Box<dyn Fn() -> i32 + Send + Sync>))
        .collect();

    // adapt boxes into concrete F where needed by using Fn() -> i32 trait object erased to Arc in implementation
    let res = rt.execute_with_hybrid(ops.into_iter().map(|(id, f)| (id, move || f())).collect());
    let stats = rt.hybrid_stats().expect("stats available");
    assert!(stats.gpu_threshold >= 8);
    assert_eq!(res.len(), 32);
    for (i, v) in res {
        assert_eq!(v, (i as i32) * 2);
    }
}
