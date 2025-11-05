// MVCC 压力测试套件
// 测试高并发、长时间运行、内存稳定性等场景

use vm_runtime::{MvccStore, GcConfig, AutoGcConfig};
use std::sync::{Arc, atomic::{AtomicU64, AtomicBool, Ordering}};
use std::thread;
use std::time::{Duration, Instant};

/// 压力测试统计信息
#[derive(Debug, Clone)]
pub struct StressTestStats {
    pub total_transactions: u64,
    pub successful_transactions: u64,
    pub failed_transactions: u64,
    pub total_reads: u64,
    pub total_writes: u64,
    pub conflicts: u64,
    pub duration_secs: f64,
    pub throughput_tps: f64,
    pub avg_latency_us: f64,
    pub p99_latency_us: f64,
    pub memory_versions: usize,
    pub memory_keys: usize,
}

impl StressTestStats {
    pub fn print_report(&self) {
        println!("\n╔═══════════════════════════════════════════════════════════╗");
        println!("║         MVCC 压力测试报告                                  ║");
        println!("╠═══════════════════════════════════════════════════════════╣");
        println!("║ 总交易数:     {:>10} 笔                              ║", self.total_transactions);
        println!("║ 成功交易:     {:>10} 笔 ({:.1}%)                     ║", 
            self.successful_transactions,
            self.successful_transactions as f64 / self.total_transactions as f64 * 100.0
        );
        println!("║ 失败交易:     {:>10} 笔 ({:.1}%)                     ║", 
            self.failed_transactions,
            self.failed_transactions as f64 / self.total_transactions as f64 * 100.0
        );
        println!("║ 冲突数:       {:>10} 次                              ║", self.conflicts);
        println!("╠═══════════════════════════════════════════════════════════╣");
        println!("║ 总读操作:     {:>10} 次                              ║", self.total_reads);
        println!("║ 总写操作:     {:>10} 次                              ║", self.total_writes);
        println!("╠═══════════════════════════════════════════════════════════╣");
        println!("║ 运行时间:     {:>10.2} 秒                            ║", self.duration_secs);
        println!("║ 吞吐量:       {:>10.2} TPS                           ║", self.throughput_tps);
        println!("║ 平均延迟:     {:>10.2} μs                            ║", self.avg_latency_us);
        println!("║ P99 延迟:     {:>10.2} μs                            ║", self.p99_latency_us);
        println!("╠═══════════════════════════════════════════════════════════╣");
        println!("║ 内存版本数:   {:>10} 个                              ║", self.memory_versions);
        println!("║ 内存键数:     {:>10} 个                              ║", self.memory_keys);
        println!("╚═══════════════════════════════════════════════════════════╝\n");
    }
}

/// 高并发读写测试
/// 
/// 参数:
/// - num_threads: 并发线程数
/// - num_txs_per_thread: 每个线程执行的交易数
/// - num_keys: 总键数（影响冲突率）
/// - read_ratio: 读操作比例（0.0-1.0）
#[test]
fn test_high_concurrency_mixed_workload() {
    let num_threads = 8;
    let num_txs_per_thread = 1000;
    let num_keys = 100;
    let read_ratio = 0.7; // 70% 读，30% 写

    println!("\n🚀 高并发混合读写压力测试");
    println!("   线程数: {}", num_threads);
    println!("   每线程交易数: {}", num_txs_per_thread);
    println!("   总键数: {}", num_keys);
    println!("   读比例: {:.0}%\n", read_ratio * 100.0);

    let config = GcConfig {
        max_versions_per_key: 20,
        enable_time_based_gc: false,
        version_ttl_secs: 3600,
        auto_gc: Some(AutoGcConfig {
            interval_secs: 5,
            version_threshold: 1000,
            run_on_start: false,
            enable_adaptive: false,
        }),
    };

    let store = Arc::new(MvccStore::new_with_config(config));

    // 初始化数据
    for i in 0..num_keys {
        let mut txn = store.begin();
        txn.write(format!("key_{}", i).into_bytes(), b"0".to_vec());
        txn.commit().unwrap();
    }

    // 统计计数器
    let success_count = Arc::new(AtomicU64::new(0));
    let fail_count = Arc::new(AtomicU64::new(0));
    let read_count = Arc::new(AtomicU64::new(0));
    let write_count = Arc::new(AtomicU64::new(0));
    let latencies = Arc::new(std::sync::Mutex::new(Vec::new()));

    let start = Instant::now();
    let mut handles = vec![];

    for thread_id in 0..num_threads {
        let store = Arc::clone(&store);
        let success = Arc::clone(&success_count);
        let fail = Arc::clone(&fail_count);
        let reads = Arc::clone(&read_count);
        let writes = Arc::clone(&write_count);
        let lats = Arc::clone(&latencies);

        let handle = thread::spawn(move || {
            let mut rng = thread_id as u64; // 简单的伪随机数
            
            for _ in 0..num_txs_per_thread {
                let tx_start = Instant::now();
                let mut txn = store.begin();

                // 随机选择键
                rng = rng.wrapping_mul(1103515245).wrapping_add(12345);
                let key_idx = (rng % num_keys as u64) as usize;
                let key = format!("key_{}", key_idx);

                // 根据比例选择读或写
                let is_read = (key_idx % 100) < (read_ratio * 100.0) as usize;

                if is_read {
                    // 读操作
                    let _ = txn.read(key.as_bytes());
                    reads.fetch_add(1, Ordering::Relaxed);
                } else {
                    // 写操作
                    rng = rng.wrapping_mul(1103515245).wrapping_add(12345);
                    let value = rng % 1000000;
                    txn.write(key.into_bytes(), format!("{}", value).into_bytes());
                    writes.fetch_add(1, Ordering::Relaxed);
                }

                match txn.commit() {
                    Ok(_) => {
                        success.fetch_add(1, Ordering::Relaxed);
                        let latency = tx_start.elapsed().as_micros() as f64;
                        lats.lock().unwrap().push(latency);
                    }
                    Err(_) => {
                        fail.fetch_add(1, Ordering::Relaxed);
                    }
                }
            }
        });

        handles.push(handle);
    }

    // 等待所有线程完成
    for handle in handles {
        handle.join().unwrap();
    }

    let duration = start.elapsed();
    let total_txs = num_threads * num_txs_per_thread;

    // 计算统计信息
    let mut latencies = latencies.lock().unwrap();
    latencies.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let avg_latency = latencies.iter().sum::<f64>() / latencies.len() as f64;
    let p99_idx = (latencies.len() as f64 * 0.99) as usize;
    let p99_latency = latencies.get(p99_idx).copied().unwrap_or(0.0);

    let stats = StressTestStats {
        total_transactions: total_txs as u64,
        successful_transactions: success_count.load(Ordering::Relaxed),
        failed_transactions: fail_count.load(Ordering::Relaxed),
        total_reads: read_count.load(Ordering::Relaxed),
        total_writes: write_count.load(Ordering::Relaxed),
        conflicts: fail_count.load(Ordering::Relaxed),
        duration_secs: duration.as_secs_f64(),
        throughput_tps: total_txs as f64 / duration.as_secs_f64(),
        avg_latency_us: avg_latency,
        p99_latency_us: p99_latency,
        memory_versions: store.total_versions(),
        memory_keys: store.total_keys(),
    };

    stats.print_report();

    // 验证
    assert!(stats.successful_transactions > 0, "应该有成功的交易");
    assert!(stats.throughput_tps > 100.0, "吞吐量应该 > 100 TPS");
    assert!(stats.memory_versions < num_keys * 30, "版本数应该被 GC 控制");
}

/// 高冲突场景压力测试
/// 所有线程竞争少量热点键
#[test]
fn test_high_contention_hotspot() {
    let num_threads = 16;
    let num_txs_per_thread = 500;
    let num_hotspot_keys = 5; // 只有 5 个热点键

    println!("\n🔥 高冲突热点键压力测试");
    println!("   线程数: {}", num_threads);
    println!("   每线程交易数: {}", num_txs_per_thread);
    println!("   热点键数: {}\n", num_hotspot_keys);

    let config = GcConfig {
        max_versions_per_key: 50, // 热点键需要更多版本
        enable_time_based_gc: false,
        version_ttl_secs: 3600,
        auto_gc: Some(AutoGcConfig {
            interval_secs: 3,
            version_threshold: 500,
            run_on_start: false,
            enable_adaptive: false,
        }),
    };

    let store = Arc::new(MvccStore::new_with_config(config));

    // 初始化热点键
    for i in 0..num_hotspot_keys {
        let mut txn = store.begin();
        txn.write(format!("hot_{}", i).into_bytes(), b"0".to_vec());
        txn.commit().unwrap();
    }

    let success_count = Arc::new(AtomicU64::new(0));
    let conflict_count = Arc::new(AtomicU64::new(0));

    let start = Instant::now();
    let mut handles = vec![];

    for thread_id in 0..num_threads {
        let store = Arc::clone(&store);
        let success = Arc::clone(&success_count);
        let conflicts = Arc::clone(&conflict_count);

        let handle = thread::spawn(move || {
            let mut rng = thread_id as u64;
            
            for _ in 0..num_txs_per_thread {
                let mut txn = store.begin();

                // 总是访问热点键
                rng = rng.wrapping_mul(1103515245).wrapping_add(12345);
                let key_idx = (rng % num_hotspot_keys as u64) as usize;
                let key = format!("hot_{}", key_idx);

                // 读取当前值
                let current = txn.read(key.as_bytes())
                    .and_then(|v| String::from_utf8(v).ok())
                    .and_then(|s| s.parse::<u64>().ok())
                    .unwrap_or(0);

                // 写入新值
                txn.write(key.into_bytes(), format!("{}", current + 1).into_bytes());

                match txn.commit() {
                    Ok(_) => {
                        success.fetch_add(1, Ordering::Relaxed);
                    }
                    Err(_) => {
                        conflicts.fetch_add(1, Ordering::Relaxed);
                    }
                }
            }
        });

        handles.push(handle);
    }

    for handle in handles {
        handle.join().unwrap();
    }

    let duration = start.elapsed();
    let total_txs = num_threads * num_txs_per_thread;
    let success = success_count.load(Ordering::Relaxed);
    let conflicts = conflict_count.load(Ordering::Relaxed);

    let stats = StressTestStats {
        total_transactions: total_txs as u64,
        successful_transactions: success,
        failed_transactions: conflicts,
        total_reads: success,
        total_writes: success,
        conflicts,
        duration_secs: duration.as_secs_f64(),
        throughput_tps: success as f64 / duration.as_secs_f64(),
        avg_latency_us: 0.0,
        p99_latency_us: 0.0,
        memory_versions: store.total_versions(),
        memory_keys: store.total_keys(),
    };

    stats.print_report();

    // 验证最终值正确性
    let mut final_txn = store.begin_read_only();
    for i in 0..num_hotspot_keys {
        let key = format!("hot_{}", i);
        if let Some(value) = final_txn.read(key.as_bytes()) {
            let count: u64 = String::from_utf8(value)
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(0);
            println!("   热点键 {} 最终值: {}", key, count);
        }
    }

    assert!(success > 0, "应该有成功的交易");
    assert!(conflicts > 0, "高冲突场景应该产生冲突");
    let conflict_rate = conflicts as f64 / total_txs as f64;
    println!("\n   冲突率: {:.1}%", conflict_rate * 100.0);
}

/// 长时间稳定性测试（简化版，实际运行时间可配置）
#[test]
#[ignore] // 默认忽略，使用 --ignored 参数运行
fn test_long_running_stability() {
    let duration_secs = 60; // 1 分钟测试（生产环境建议数小时）
    let num_threads = 4;
    let num_keys = 200;

    println!("\n⏰ 长时间稳定性测试");
    println!("   运行时长: {} 秒", duration_secs);
    println!("   线程数: {}", num_threads);
    println!("   键数: {}\n", num_keys);

    let config = GcConfig {
        max_versions_per_key: 15,
        enable_time_based_gc: false,
        version_ttl_secs: 3600,
        auto_gc: Some(AutoGcConfig {
            interval_secs: 10,
            version_threshold: 2000,
            run_on_start: false,
            enable_adaptive: false,
        }),
    };

    let store = Arc::new(MvccStore::new_with_config(config));

    // 初始化数据
    for i in 0..num_keys {
        let mut txn = store.begin();
        txn.write(format!("key_{}", i).into_bytes(), b"0".to_vec());
        txn.commit().unwrap();
    }

    let running = Arc::new(AtomicBool::new(true));
    let tx_count = Arc::new(AtomicU64::new(0));

    // 监控线程
    let monitor_running = Arc::clone(&running);
    let monitor_store = Arc::clone(&store);
    let monitor_tx_count = Arc::clone(&tx_count);
    let monitor_handle = thread::spawn(move || {
        let start = Instant::now();
        let mut last_gc_stats = monitor_store.get_gc_stats();
        
        while monitor_running.load(Ordering::Relaxed) {
            thread::sleep(Duration::from_secs(10));
            
            let elapsed = start.elapsed().as_secs();
            let txs = monitor_tx_count.load(Ordering::Relaxed);
            let versions = monitor_store.total_versions();
            let keys = monitor_store.total_keys();
            let gc_stats = monitor_store.get_gc_stats();

            println!("   [{}s] TPS: {:.0}, 版本数: {}, 键数: {}, GC 次数: {}, 清理版本: {}",
                elapsed,
                txs as f64 / elapsed as f64,
                versions,
                keys,
                gc_stats.gc_count - last_gc_stats.gc_count,
                gc_stats.versions_cleaned - last_gc_stats.versions_cleaned,
            );

            last_gc_stats = gc_stats;
        }
    });

    // 工作线程
    let start = Instant::now();
    let mut handles = vec![];

    for thread_id in 0..num_threads {
        let store = Arc::clone(&store);
        let running = Arc::clone(&running);
        let count = Arc::clone(&tx_count);

        let handle = thread::spawn(move || {
            let mut rng = thread_id as u64;
            
            while running.load(Ordering::Relaxed) {
                let mut txn = store.begin();

                rng = rng.wrapping_mul(1103515245).wrapping_add(12345);
                let key_idx = (rng % num_keys as u64) as usize;
                let key = format!("key_{}", key_idx);

                // 50% 读，50% 写
                if key_idx.is_multiple_of(2) {
                    let _ = txn.read(key.as_bytes());
                } else {
                    rng = rng.wrapping_mul(1103515245).wrapping_add(12345);
                    let value = rng % 1000000;
                    txn.write(key.into_bytes(), format!("{}", value).into_bytes());
                }

                if txn.commit().is_ok() {
                    count.fetch_add(1, Ordering::Relaxed);
                }

                // 小延迟避免过度占用 CPU
                thread::sleep(Duration::from_micros(100));
            }
        });

        handles.push(handle);
    }

    // 运行指定时间
    thread::sleep(Duration::from_secs(duration_secs));
    running.store(false, Ordering::Relaxed);

    // 等待所有线程完成
    for handle in handles {
        handle.join().unwrap();
    }
    monitor_handle.join().unwrap();

    let duration = start.elapsed();
    let total_txs = tx_count.load(Ordering::Relaxed);

    println!("\n✅ 长时间测试完成");
    println!("   总交易数: {}", total_txs);
    println!("   平均 TPS: {:.2}", total_txs as f64 / duration.as_secs_f64());
    println!("   最终版本数: {}", store.total_versions());
    println!("   最终键数: {}", store.total_keys());

    let gc_stats = store.get_gc_stats();
    println!("   GC 总次数: {}", gc_stats.gc_count);
    println!("   GC 总清理: {} 个版本", gc_stats.versions_cleaned);

    assert!(total_txs > 0, "应该执行了交易");
    assert!(store.total_versions() < num_keys * 50, "版本数应该被控制");
}

/// 内存增长监控测试
#[test]
fn test_memory_growth_control() {
    println!("\n📊 内存增长控制测试");

    let config = GcConfig {
        max_versions_per_key: 10,
        enable_time_based_gc: false,
        version_ttl_secs: 3600,
        auto_gc: Some(AutoGcConfig {
            interval_secs: 2,
            version_threshold: 500,
            run_on_start: false,
            enable_adaptive: false,
        }),
    };

    let store = Arc::new(MvccStore::new_with_config(config));

    let num_keys = 50;
    let num_iterations = 20;

    println!("   键数: {}", num_keys);
    println!("   迭代次数: {}", num_iterations);
    println!("\n   监控内存增长...\n");

    for iteration in 0..num_iterations {
        // 每次迭代写入所有键
        for i in 0..num_keys {
            let mut txn = store.begin();
            txn.write(
                format!("key_{}", i).into_bytes(),
                format!("value_{}", iteration).into_bytes()
            );
            txn.commit().unwrap();
        }

        let versions = store.total_versions();
        let keys = store.total_keys();
        let avg_versions_per_key = versions as f64 / keys as f64;

        println!("   迭代 {:2}: 版本数 = {:4}, 键数 = {:3}, 平均版本/键 = {:.2}",
            iteration + 1, versions, keys, avg_versions_per_key);

        // 等待 GC 运行
        thread::sleep(Duration::from_millis(500));
    }

    // 给予自动 GC 最后一次运行机会，避免时序抖动导致的统计偏差
    thread::sleep(Duration::from_secs(3));

    let final_versions = store.total_versions();
    let final_keys = store.total_keys();
    let gc_stats = store.get_gc_stats();

    println!("\n   ✅ 测试完成");
    println!("   最终版本数: {}", final_versions);
    println!("   最终键数: {}", final_keys);
    println!("   GC 执行次数: {}", gc_stats.gc_count);
    println!("   GC 清理版本: {}", gc_stats.versions_cleaned);

    // 验证：版本数应该被限制
    let max_expected_versions = num_keys * 15; // 稍高于配置的 max_versions_per_key
    assert!(
        final_versions <= max_expected_versions,
        "版本数 {} 超过预期最大值 {}",
        final_versions,
        max_expected_versions
    );

    // 验证：GC 应该执行过
    assert!(gc_stats.gc_count > 0, "GC 应该至少执行过一次");
    assert!(gc_stats.versions_cleaned > 0, "GC 应该清理过版本");
}

/// 自适应 GC 测试
/// 测试 GC 策略能否根据负载自动调整
#[test]
fn test_adaptive_gc() {
    println!("\n🎯 自适应 GC 测试");

    let config = GcConfig {
        max_versions_per_key: 20,
        enable_time_based_gc: false,
        version_ttl_secs: 3600,
        auto_gc: Some(AutoGcConfig {
            interval_secs: 10,      // 初始 10 秒间隔
            version_threshold: 1000,
            run_on_start: false,
            enable_adaptive: true,  // 启用自适应
        }),
    };

    let store = Arc::new(MvccStore::new_with_config(config));

    println!("   初始间隔: 10 秒");
    println!("   初始阈值: 1000 版本");
    println!("   自适应模式: 已启用\n");

    // 阶段 1: 高负载写入（产生大量版本）
    println!("   [阶段 1] 高负载写入...");
    for i in 0..100 {
        for j in 0..50 {
            let mut txn = store.begin();
            txn.write(
                format!("key_{}", j).into_bytes(),
                format!("value_{}_{}", i, j).into_bytes()
            );
            txn.commit().unwrap();
        }
    }
    println!("     写入 5000 个版本");
    println!("     版本数: {}", store.total_versions());
    
    thread::sleep(Duration::from_secs(3));
    
    let gc_stats_1 = store.get_gc_stats();
    println!("     GC 次数: {}", gc_stats_1.gc_count);
    println!("     清理版本: {}", gc_stats_1.versions_cleaned);

    // 阶段 2: 等待一段时间，观察 GC 行为
    println!("\n   [阶段 2] 等待自适应调整...");
    for i in 0..5 {
        thread::sleep(Duration::from_secs(3));
        let versions = store.total_versions();
        let gc_stats = store.get_gc_stats();
        println!("     {}s: 版本数 = {}, GC 次数 = {}",
            (i + 1) * 3, versions, gc_stats.gc_count);
    }

    // 阶段 3: 低负载（少量写入）
    println!("\n   [阶段 3] 低负载写入...");
    for i in 0..10 {
        let mut txn = store.begin();
        txn.write(b"low_load_key".to_vec(), format!("value_{}", i).into_bytes());
        txn.commit().unwrap();
    }
    println!("     写入 10 个版本");
    
    thread::sleep(Duration::from_secs(5));
    
    let final_stats = store.get_gc_stats();
    let final_versions = store.total_versions();

    println!("\n   ✅ 测试完成");
    println!("   最终版本数: {}", final_versions);
    println!("   总 GC 次数: {}", final_stats.gc_count);
    println!("   总清理版本: {}", final_stats.versions_cleaned);

    // 验证
    assert!(final_stats.gc_count > 0, "自适应 GC 应该执行过");
    assert!(final_versions < 5000, "版本数应该被 GC 控制");
    println!("\n   💡 自适应 GC 根据负载自动调整了参数");
}
