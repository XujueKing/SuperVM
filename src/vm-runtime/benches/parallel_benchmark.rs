use criterion::{black_box, criterion_group, criterion_main, Criterion, BenchmarkId};
use vm_runtime::{ReadWriteSet, ConflictDetector, ParallelScheduler};
use std::collections::HashSet;

/// 生成不冲突的交易读写集
fn generate_non_conflicting_txs(count: usize) -> Vec<(u64, ReadWriteSet)> {
    (0..count)
        .map(|i| {
            let mut rw_set = ReadWriteSet::new();
            // 每个交易访问不同的账户
            let account = format!("account_{}", i);
            rw_set.add_write(account.into_bytes());
            (i as u64, rw_set)
        })
        .collect()
}

/// 生成部分冲突的交易读写集
fn generate_conflicting_txs(count: usize, conflict_rate: f64) -> Vec<(u64, ReadWriteSet)> {
    (0..count)
        .map(|i| {
            let mut rw_set = ReadWriteSet::new();
            
            // 根据冲突率决定是否访问热点账户
            if (i as f64 / count as f64) < conflict_rate {
                // 热点账户 - 会产生冲突
                rw_set.add_write(b"hot_account".to_vec());
            } else {
                // 独立账户 - 不会冲突
                let account = format!("account_{}", i);
                rw_set.add_write(account.into_bytes());
            }
            
            (i as u64, rw_set)
        })
        .collect()
}

/// 基准测试: 冲突检测性能
fn bench_conflict_detection(c: &mut Criterion) {
    let mut group = c.benchmark_group("conflict_detection");
    
    for tx_count in [10, 50, 100, 500].iter() {
        group.bench_with_input(
            BenchmarkId::new("non_conflicting", tx_count),
            tx_count,
            |b, &count| {
                let txs = generate_non_conflicting_txs(count);
                
                b.iter(|| {
                    let mut detector = ConflictDetector::new();
                    for (tx_id, rw_set) in &txs {
                        detector.record(*tx_id, rw_set.clone());
                    }
                    
                    let tx_ids: Vec<u64> = (0..count as u64).collect();
                    black_box(detector.build_dependency_graph(&tx_ids));
                });
            },
        );
        
        group.bench_with_input(
            BenchmarkId::new("50%_conflict", tx_count),
            tx_count,
            |b, &count| {
                let txs = generate_conflicting_txs(count, 0.5);
                
                b.iter(|| {
                    let mut detector = ConflictDetector::new();
                    for (tx_id, rw_set) in &txs {
                        detector.record(*tx_id, rw_set.clone());
                    }
                    
                    let tx_ids: Vec<u64> = (0..count as u64).collect();
                    black_box(detector.build_dependency_graph(&tx_ids));
                });
            },
        );
    }
    
    group.finish();
}

/// 基准测试: 快照创建和回滚性能
fn bench_snapshot_operations(c: &mut Criterion) {
    let mut group = c.benchmark_group("snapshot_operations");
    
    for data_size in [10, 100, 1000].iter() {
        group.bench_with_input(
            BenchmarkId::new("create_snapshot", data_size),
            data_size,
            |b, &size| {
                let scheduler = ParallelScheduler::new();
                
                // 预填充数据
                {
                    let storage_arc = scheduler.get_storage();
                    let mut storage = storage_arc.lock().unwrap();
                    for i in 0..size {
                        let key = format!("key_{}", i).into_bytes();
                        let value = format!("value_{}", i).into_bytes();
                        storage.insert(key, value);
                    }
                }
                
                b.iter(|| {
                    scheduler.execute_with_snapshot(|_manager| {
                        // 仅测试快照创建和提交的开销
                        Ok(())
                    }).unwrap();
                });
            },
        );
        
        group.bench_with_input(
            BenchmarkId::new("rollback", data_size),
            data_size,
            |b, &size| {
                let scheduler = ParallelScheduler::new();
                
                // 预填充数据
                {
                    let storage_arc = scheduler.get_storage();
                    let mut storage = storage_arc.lock().unwrap();
                    for i in 0..size {
                        let key = format!("key_{}", i).into_bytes();
                        let value = format!("value_{}", i).into_bytes();
                        storage.insert(key, value);
                    }
                }
                
                b.iter(|| {
                    let _result: Result<(), String> = scheduler.execute_with_snapshot(|manager| {
                        let storage_arc = manager.get_storage();
                        let mut storage = storage_arc.lock().unwrap();
                        
                        // 修改所有数据
                        for i in 0..size {
                            let key = format!("key_{}", i).into_bytes();
                            storage.insert(key, b"modified".to_vec());
                        }
                        
                        // 触发回滚
                        Err("rollback test".to_string())
                    });
                });
            },
        );
    }
    
    group.finish();
}

/// 基准测试: 依赖图构建性能
fn bench_dependency_graph(c: &mut Criterion) {
    let mut group = c.benchmark_group("dependency_graph");
    
    for tx_count in [10, 50, 100, 500].iter() {
        group.bench_with_input(
            BenchmarkId::new("build_and_query", tx_count),
            tx_count,
            |b, &count| {
                let txs = generate_conflicting_txs(count, 0.3);
                let mut detector = ConflictDetector::new();
                
                for (tx_id, rw_set) in &txs {
                    detector.record(*tx_id, rw_set.clone());
                }
                
                let tx_ids: Vec<u64> = (0..count as u64).collect();
                
                b.iter(|| {
                    let graph = detector.build_dependency_graph(&tx_ids);
                    let completed = HashSet::new();
                    black_box(graph.get_ready_transactions(&tx_ids, &completed));
                });
            },
        );
    }
    
    group.finish();
}

/// 基准测试: 并行批次调度
fn bench_parallel_scheduling(c: &mut Criterion) {
    let mut group = c.benchmark_group("parallel_scheduling");
    
    for tx_count in [10, 50, 100].iter() {
        group.bench_with_input(
            BenchmarkId::new("get_parallel_batch", tx_count),
            tx_count,
            |b, &count| {
                let scheduler = ParallelScheduler::new();
                let txs = generate_conflicting_txs(count, 0.2);
                
                // 记录所有交易的读写集
                for (tx_id, rw_set) in &txs {
                    scheduler.record_rw_set(*tx_id, rw_set.clone());
                }
                
                let all_txs: Vec<u64> = (0..count as u64).collect();
                
                b.iter(|| {
                    black_box(scheduler.get_parallel_batch(&all_txs));
                });
            },
        );
    }
    
    group.finish();
}

criterion_group!(
    benches,
    bench_conflict_detection,
    bench_snapshot_operations,
    bench_dependency_graph,
    bench_parallel_scheduling
);
criterion_main!(benches);
