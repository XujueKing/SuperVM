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

/// 基准测试: MVCC 性能对比
fn bench_mvcc_operations(c: &mut Criterion) {
    use vm_runtime::MvccStore;
    use std::sync::Arc;
    
    let mut group = c.benchmark_group("mvcc_operations");
    
    // 基准 1: 只读事务 vs 读写事务
    group.bench_function("read_only_transaction", |b| {
        let store = Arc::new(MvccStore::new());
        
        // 预填充数据
        for i in 0..100 {
            let mut txn = store.begin();
            txn.write(format!("key_{}", i).into_bytes(), b"value".to_vec());
            txn.commit().unwrap();
        }
        
        b.iter(|| {
            let txn = store.begin_read_only();
            for i in 0..10 {
                black_box(txn.read(&format!("key_{}", i).into_bytes()));
            }
            let _ = txn.commit();
        });
    });
    
    group.bench_function("read_write_transaction", |b| {
        let store = Arc::new(MvccStore::new());
        
        // 预填充数据
        for i in 0..100 {
            let mut txn = store.begin();
            txn.write(format!("key_{}", i).into_bytes(), b"value".to_vec());
            txn.commit().unwrap();
        }
        
        b.iter(|| {
            let txn = store.begin();
            for i in 0..10 {
                black_box(txn.read(&format!("key_{}", i).into_bytes()));
            }
            let _ = txn.commit();
        });
    });
    
    // 基准 2: 无冲突写入性能
    group.bench_function("mvcc_non_conflicting_writes", |b| {
        let store = Arc::new(MvccStore::new());
        let mut counter = 0;
        
        b.iter(|| {
            let mut txn = store.begin();
            let key = format!("key_{}", counter).into_bytes();
            txn.write(key, b"value".to_vec());
            let _ = txn.commit();
            counter += 1;
        });
    });
    
    // 基准 3: 冲突写入性能（写同一个键）
    group.bench_function("mvcc_conflicting_writes", |b| {
        let store = Arc::new(MvccStore::new());
        
        b.iter(|| {
            let mut txn = store.begin();
            txn.write(b"hot_key".to_vec(), b"value".to_vec());
            let _ = txn.commit(); // 可能成功也可能冲突
        });
    });
    
    group.finish();
}

/// 基准测试: MVCC 调度器集成
fn bench_mvcc_scheduler(c: &mut Criterion) {
    use vm_runtime::MvccStore;
    use std::sync::Arc;
    
    let mut group = c.benchmark_group("mvcc_scheduler");
    
    // 基准 1: Snapshot 后端 vs MVCC 后端（只读）
    group.bench_function("snapshot_backend_read", |b| {
        let scheduler = ParallelScheduler::new();
        
        // 预填充数据
        for i in 0..100 {
            scheduler.execute_with_snapshot(|manager| {
                let storage = manager.get_storage();
                let mut storage = storage.lock().unwrap();
                storage.insert(format!("key_{}", i).into_bytes(), b"value".to_vec());
                Ok(())
            }).unwrap();
        }
        
        b.iter(|| {
            scheduler.execute_with_snapshot(|manager| {
                let storage = manager.get_storage();
                let storage = storage.lock().unwrap();
                for i in 0..10 {
                    black_box(storage.get(&format!("key_{}", i).into_bytes()));
                }
                Ok(())
            }).unwrap();
        });
    });
    
    group.bench_function("mvcc_backend_read_only", |b| {
        let store = Arc::new(MvccStore::new());
        let scheduler = ParallelScheduler::new_with_mvcc(Arc::clone(&store));
        
        // 预填充数据
        for i in 0..100 {
            scheduler.execute_with_mvcc(|txn| {
                txn.write(format!("key_{}", i).into_bytes(), b"value".to_vec());
                Ok(())
            }).unwrap();
        }
        
        b.iter(|| {
            scheduler.execute_with_mvcc_read_only(|txn| {
                for i in 0..10 {
                    black_box(txn.read(&format!("key_{}", i).into_bytes()));
                }
                Ok(())
            }).unwrap();
        });
    });
    
    // 基准 2: 写入性能对比
    group.bench_function("snapshot_backend_write", |b| {
        let scheduler = ParallelScheduler::new();
        let mut counter = 0;
        
        b.iter(|| {
            scheduler.execute_with_snapshot(|manager| {
                let storage = manager.get_storage();
                let mut storage = storage.lock().unwrap();
                storage.insert(format!("key_{}", counter).into_bytes(), b"value".to_vec());
                Ok(())
            }).unwrap();
            counter += 1;
        });
    });
    
    group.bench_function("mvcc_backend_write", |b| {
        let store = Arc::new(MvccStore::new());
        let scheduler = ParallelScheduler::new_with_mvcc(Arc::clone(&store));
        let mut counter = 0;
        
        b.iter(|| {
            scheduler.execute_with_mvcc(|txn| {
                txn.write(format!("key_{}", counter).into_bytes(), b"value".to_vec());
                Ok(())
            }).unwrap();
            counter += 1;
        });
    });
    
    group.finish();
}

/// 基准测试: MVCC 垃圾回收性能
fn bench_mvcc_gc(c: &mut Criterion) {
    use vm_runtime::{MvccStore, GcConfig};
    use std::sync::Arc;
    
    let mut group = c.benchmark_group("mvcc_gc");
    
    // 基准 1: GC 吞吐量（不同版本数）
    for versions_per_key in [5, 10, 20, 50].iter() {
        group.bench_with_input(
            BenchmarkId::new("gc_throughput", versions_per_key),
            versions_per_key,
            |b, &count| {
                let config = GcConfig {
                    max_versions_per_key: 3,
                    enable_time_based_gc: false,
                    version_ttl_secs: 3600,
                };
                let store = Arc::new(MvccStore::new_with_config(config));
                
                // 预填充：100 个键，每个键 count 个版本
                for key_id in 0..100 {
                    for ver in 0..count {
                        let mut txn = store.begin();
                        txn.write(
                            format!("key{}", key_id).into_bytes(),
                            format!("v{}", ver).into_bytes(),
                        );
                        txn.commit().unwrap();
                    }
                }
                
                b.iter(|| {
                    black_box(store.gc().unwrap());
                });
            },
        );
    }
    
    // 基准 2: GC 对读取性能的影响
    group.bench_function("read_with_gc", |b| {
        let config = GcConfig {
            max_versions_per_key: 5,
            enable_time_based_gc: false,
            version_ttl_secs: 3600,
        };
        let store = Arc::new(MvccStore::new_with_config(config));
        
        // 预填充
        for i in 0..100 {
            let mut txn = store.begin();
            txn.write(format!("key{}", i).into_bytes(), b"value".to_vec());
            txn.commit().unwrap();
        }
        
        let mut counter = 0;
        
        b.iter(|| {
            // 读取
            let txn = store.begin_read_only();
            for i in 0..10 {
                black_box(txn.read(&format!("key{}", i).into_bytes()));
            }
            drop(txn);
            
            // 每 10 次读取执行一次 GC
            counter += 1;
            if counter % 10 == 0 {
                store.gc().unwrap();
            }
        });
    });
    
    // 基准 3: GC 对写入性能的影响
    group.bench_function("write_with_gc", |b| {
        let config = GcConfig {
            max_versions_per_key: 5,
            enable_time_based_gc: false,
            version_ttl_secs: 3600,
        };
        let store = Arc::new(MvccStore::new_with_config(config));
        
        let mut counter = 0;
        
        b.iter(|| {
            // 写入
            let mut txn = store.begin();
            txn.write(format!("key{}", counter % 100).into_bytes(), b"value".to_vec());
            txn.commit().unwrap();
            
            // 每 20 次写入执行一次 GC
            counter += 1;
            if counter % 20 == 0 {
                store.gc().unwrap();
            }
        });
    });
    
    // 基准 4: 活跃事务对 GC 的影响
    group.bench_function("gc_with_active_transactions", |b| {
        let config = GcConfig {
            max_versions_per_key: 3,
            enable_time_based_gc: false,
            version_ttl_secs: 3600,
        };
        let store = Arc::new(MvccStore::new_with_config(config));
        
        // 预填充
        for i in 0..100 {
            for v in 0..10 {
                let mut txn = store.begin();
                txn.write(format!("key{}", i).into_bytes(), format!("v{}", v).into_bytes());
                txn.commit().unwrap();
            }
        }
        
        // 创建一些长期活跃的事务
        let _active_txns: Vec<_> = (0..5).map(|_| store.begin_read_only()).collect();
        
        b.iter(|| {
            black_box(store.gc().unwrap());
        });
    });
    
    group.finish();
}

criterion_group!(
    benches,
    bench_conflict_detection,
    bench_snapshot_operations,
    bench_dependency_graph,
    bench_parallel_scheduling,
    bench_mvcc_operations,
    bench_mvcc_scheduler,
    bench_mvcc_gc
);
criterion_main!(benches);
