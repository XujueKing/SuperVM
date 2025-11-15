// SPDX-License-Identifier: GPL-3.0-or-later
// 批量验证测试：失败率、禁用回退、并发场景

#[cfg(test)]
#[cfg(feature = "groth16-verifier")]
mod batch_verify_extended_tests {
    use vm_runtime::supervm::*;
    use vm_runtime::{OwnershipManager, OwnershipType, ObjectMetadata};
    use vm_runtime::parallel_mvcc::MvccScheduler;
    use vm_runtime::zk_verifier::{ZkVerifier, MockVerifier};

    type Address = [u8;32];
    type ObjectId = [u8;32];
    fn addr(id: u8) -> Address { let mut a=[0u8;32]; a[0]=id; a }
    fn obj(id: u8) -> ObjectId { let mut o=[0u8;32]; o[0]=id; o }

    fn reg_owned(om: &OwnershipManager, id: ObjectId, owner: Address) {
        let meta = ObjectMetadata { id, version:0, ownership: OwnershipType::Owned(owner), object_type: "Test".into(), created_at:0, updated_at:0, size:0, is_deleted:false};
        om.register_object(meta).unwrap();
    }

    #[test]
    fn batch_verify_partial_failure() {
        // 混入 50% 失败 proof，验证失败计数
        std::env::set_var("ZK_BATCH_ENABLE", "1");
        std::env::set_var("ZK_BATCH_SIZE", "4");
        std::env::set_var("ZK_BATCH_FLUSH_INTERVAL_MS", "1000");

        let ownership = OwnershipManager::new();
        reg_owned(&ownership, obj(1), addr(1));
        let scheduler = MvccScheduler::new();

        // 创建混合验证器：前两个成功，后两个失败
        let mixed_verifier: &'static dyn ZkVerifier = Box::leak(Box::new(MockVerifier::new_always_succeed()));
        let fail_verifier: &'static dyn ZkVerifier = Box::leak(Box::new(MockVerifier::new_always_fail()));

        let vm_success = SuperVM::new(&ownership)
            .with_scheduler(&scheduler)
            .with_verifier(mixed_verifier)
            .from_env();

        // Push 4 proofs (模拟混合场景：使用 MockVerifier 始终成功，后续手动触发失败)
        for i in 0..4 {
            let p = vec![i as u8; 32];
            let pi = vec![i as u8; 8];
            assert!(vm_success.verify_zk_proof(Some(&p), Some(&pi))); // 触发 flush
        }

        // 验证批量指标：total=4, failed 应为 0（MockVerifier 总是成功）
        let prom = scheduler.store().get_metrics().unwrap().export_prometheus();
        assert!(prom.contains("vm_privacy_zk_batch_verify_total 4"));
        assert!(prom.contains("vm_privacy_zk_batch_verify_failed_total 0"));

        // 创建失败场景：使用 fail_verifier
        let vm_fail = SuperVM::new(&ownership)
            .with_scheduler(&scheduler)
            .with_verifier(fail_verifier)
            .from_env();

        for i in 0..4 {
            let p = vec![(i+10) as u8; 32];
            let pi = vec![(i+10) as u8; 8];
            let _ = vm_fail.verify_zk_proof(Some(&p), Some(&pi));
        }

        // 第二批应全部失败（MockVerifier::new_always_fail）
        let prom2 = scheduler.store().get_metrics().unwrap().export_prometheus();
        assert!(prom2.contains("vm_privacy_zk_batch_verify_total 8")); // 累计 4+4
        assert!(prom2.contains("vm_privacy_zk_batch_verify_failed_total 4")); // 新增 4 个失败
    }

    #[test]
    fn batch_disabled_fallback_to_single() {
        // ZK_BATCH_ENABLE=0 时，应回退到单次验证
        std::env::set_var("ZK_BATCH_ENABLE", "0");

        let ownership = OwnershipManager::new();
        reg_owned(&ownership, obj(2), addr(1));
        let scheduler = MvccScheduler::new();
        let verifier: &'static dyn ZkVerifier = Box::leak(Box::new(MockVerifier::new_always_succeed()));

        let vm = SuperVM::new(&ownership)
            .with_scheduler(&scheduler)
            .with_verifier(verifier)
            .from_env();

        // Push 多个 proof，但不会触发批量
        for i in 0..10 {
            let p = vec![i as u8; 32];
            let pi = vec![i as u8; 8];
            assert!(vm.verify_zk_proof(Some(&p), Some(&pi)));
        }

        // 验证批量指标应为 0（未启用批量）
        let prom = scheduler.store().get_metrics().unwrap().export_prometheus();
        assert!(prom.contains("vm_privacy_zk_batch_verify_batches_total 0"));
        // 单次验证指标应有记录（每次都单独验证）
        // 注意：当前实现未启用批量时走原始单次路径，不会增加 batch verify 指标
    }

    #[test]
    fn concurrent_flush_safety() {
        // 多线程并发调用 flush，验证无 panic 和数据竞争
        std::env::set_var("ZK_BATCH_ENABLE", "1");
        std::env::set_var("ZK_BATCH_SIZE", "10");
        std::env::set_var("ZK_BATCH_FLUSH_INTERVAL_MS", "100");

        let ownership = Box::leak(Box::new(OwnershipManager::new()));
        reg_owned(ownership, obj(3), addr(1));
        let scheduler = Box::leak(Box::new(MvccScheduler::new()));
        let verifier: &'static dyn ZkVerifier = Box::leak(Box::new(MockVerifier::new_always_succeed()));

        let vm: &'static SuperVM = Box::leak(Box::new(
            SuperVM::new(ownership)
                .with_scheduler(scheduler)
                .with_verifier(verifier)
                .from_env()
        ));

        // 并发 Push 20 个 proof
        let handles: Vec<_> = (0..4).map(|tid| {
            std::thread::spawn(move || {
                for i in 0..5 {
                    let p = vec![(tid * 10 + i) as u8; 32];
                    let pi = vec![(tid * 10 + i) as u8; 8];
                    let _ = vm.verify_zk_proof(Some(&p), Some(&pi));
                }
            })
        }).collect();

        for h in handles { h.join().unwrap(); }

        // 手动 flush 剩余
        let (total, failed) = vm.flush_zk_batch();
        println!("Final flush: total={}, failed={}", total, failed);

        // 验证无 panic，且批量总数 >= 20（可能分多批）
        let prom = scheduler.store().get_metrics().unwrap().export_prometheus();
        assert!(prom.contains("vm_privacy_zk_batch_verify"));
        // 总量应为 20（4线程 × 5 proofs）
        let total_str = prom.lines()
            .find(|l| l.starts_with("vm_privacy_zk_batch_verify_total "))
            .unwrap();
        let count: u64 = total_str.split_whitespace().nth(1).unwrap().parse().unwrap();
        assert!(count >= 20, "Expected >=20 proofs verified, got {}", count);
    }
}
