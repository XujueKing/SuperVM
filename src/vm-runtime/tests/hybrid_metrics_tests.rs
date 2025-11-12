#![cfg(feature = "hybrid-exec")]
use vm_runtime::{Runtime, MemoryStorage};

#[test]
fn hybrid_metrics_increments() {
    let mut rt: Runtime<MemoryStorage> = Runtime::new_with_routing(MemoryStorage::default());
    rt.init_hybrid();

    // First batch
    let ops1: Vec<(u64, Box<dyn Fn() -> i32 + Send + Sync>)> = (0..10u64)
        .map(|i| (i, Box::new(move || (i as i32) + 1) as Box<dyn Fn() -> i32 + Send + Sync>))
        .collect();
    let _ = rt.execute_with_hybrid(ops1.into_iter().map(|(id, f)| (id, move || f())).collect());

    // Second batch
    let ops2: Vec<(u64, Box<dyn Fn() -> i32 + Send + Sync>)> = (0..20u64)
        .map(|i| (i, Box::new(move || (i as i32) * 2) as Box<dyn Fn() -> i32 + Send + Sync>))
        .collect();
    let _ = rt.execute_with_hybrid(ops2.into_iter().map(|(id, f)| (id, move || f())).collect());

    // Check metrics via scheduler
    let sched = rt.scheduler().expect("scheduler available");
    let metrics = sched.store().get_metrics().expect("metrics available");
    let decisions = metrics.hybrid_routing_decisions.load(std::sync::atomic::Ordering::Relaxed);
    let last_batch_size = metrics.hybrid_batch_size.load(std::sync::atomic::Ordering::Relaxed);

    assert!(decisions >= 2, "expected at least 2 routing decisions, got {}", decisions);
    assert_eq!(last_batch_size, 20, "last batch size should be 20");
}
