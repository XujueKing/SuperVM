use gpu_executor::*;

#[test]
fn smoke_batch_mapping() {
    let cpu = CpuMapExecutor::new(|x: &i32| x - 1);
    let mut scheduler = HybridScheduler::new(cpu, Some(UnavailableGpu), HybridStrategy::default());
    let batch = Batch { tasks: vec![
        Task { id: 100, payload: 10, est_cost: 1 },
        Task { id: 101, payload: -2, est_cost: 1 },
    ]};
    let (res, stats) = scheduler.schedule(&batch).expect("cpu fallback works");
    assert_eq!(res.len(), 2);
    assert_eq!(res[0].id, 100);
    assert_eq!(res[0].output, 9);
    assert_eq!(stats.device, DeviceKind::Cpu);
}
