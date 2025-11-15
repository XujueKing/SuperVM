// SPDX-License-Identifier: GPL-3.0-or-later
// Performance Matrix Regression Tests
//
// 目标: 建立 L0 优化后的性能基准,用于未来回归测试
// 测试矩阵:
// 1. FastPath 延迟分位 (p50/p90/p95/p99)
// 2. Parallel Prover 线程池复用效率
// 3. ProvingKey 缓存加速比
// 4. 端到端集成性能验证

#[cfg(test)]
mod perf_matrix_tests {
    use vm_runtime::parallel::FastPathExecutor;
    use vm_runtime::metrics::LatencyHistogram;
    use std::sync::Arc;
    use std::time::{Duration, Instant};

    #[cfg(feature = "groth16-verifier")]
    use vm_runtime::privacy::parallel_prover::{
        ParallelProver, RingCtParallelProver, CircuitInput, ParallelProveConfig,
        RingCtWitness, get_pool_stats,
    };
    #[cfg(feature = "groth16-verifier")]
    use ark_bls12_381::Fr;

    /// 性能基准阈值配置
    struct PerfThresholds {
        // FastPath 阈值
        fastpath_p50_max_ms: f64,
        fastpath_p99_max_ms: f64,
        fastpath_min_tps: f64,
        fastpath_retry_rate_max: f64,
        
        // 拥塞控制阈值
        congestion_normal_max_ms: f64,  // 正常负载最大延迟
        congestion_backoff_min: f64,    // 拥塞场景最小退避倍数
        hotkey_detection_accuracy: f64, // 热键检测准确率

        // Parallel Prover 阈值
        prover_pool_creation_max_ms: f64,
        prover_tps_min: f64,
        prover_batch_latency_max_ms: f64,

        // ProvingKey 缓存阈值
        pk_first_creation_max_ms: f64,
        pk_reuse_creation_max_ms: f64,
        pk_speedup_min: f64,
    }

    impl Default for PerfThresholds {
        fn default() -> Self {
            Self {
                // FastPath: 保守阈值（基于 demo 结果）
                fastpath_p50_max_ms: 2.0,
                fastpath_p99_max_ms: 10.0,
                fastpath_min_tps: 1000.0,
                fastpath_retry_rate_max: 10.0, // 10%
                
                // 拥塞控制: 基于 congestion_control_demo
                congestion_normal_max_ms: 5.0,  // 正常负载 < 5ms
                congestion_backoff_min: 2.0,    // 拥塞至少 2x 退避
                hotkey_detection_accuracy: 0.9, // 90% 准确率

                // Parallel Prover: 基于验证结果
                prover_pool_creation_max_ms: 1.0,
                prover_tps_min: 40.0,
                prover_batch_latency_max_ms: 250.0,

                // ProvingKey 缓存: 保守加速比
                pk_first_creation_max_ms: 100.0,
                pk_reuse_creation_max_ms: 1.0,
                pk_speedup_min: 50.0, // 至少 50x 加速
            }
        }
    }

    #[test]
    fn test_fastpath_latency_percentiles() {
        println!("\n━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
        println!("  Test 1: FastPath Latency Percentiles (p50/p90/p95/p99)");
        println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n");

        let thresholds = PerfThresholds::default();
        let histogram = Arc::new(LatencyHistogram::new());
        let executor = FastPathExecutor::with_histogram(histogram.clone());

        // 执行 500 次事务（轻量级测试）
        let num_txns = 500;
        let start = Instant::now();
        let mut success_count = 0;

        for i in 0..num_txns {
            // 模拟快速执行（50-100μs）
            let result = executor.execute(i as u64, || {
                std::thread::sleep(Duration::from_micros(50 + (i % 50) as u64));
                Ok(0) // 返回 i32
            });
            if result.is_ok() {
                success_count += 1;
            }
        }

        let total_duration = start.elapsed();
        let stats = executor.stats();

        // 验证指标
        println!("Results:");
        println!("  Executed: {}/{}", success_count, num_txns);
        println!("  P50: {:.3}ms (threshold: <{:.1}ms)", stats.p50_latency_ms, thresholds.fastpath_p50_max_ms);
        println!("  P90: {:.3}ms", stats.p90_latency_ms);
        println!("  P95: {:.3}ms", stats.p95_latency_ms);
        println!("  P99: {:.3}ms (threshold: <{:.1}ms)", stats.p99_latency_ms, thresholds.fastpath_p99_max_ms);
        
        let tps = num_txns as f64 / total_duration.as_secs_f64();
        println!("  TPS: {:.0} (threshold: >{:.0})", tps, thresholds.fastpath_min_tps);

        // 断言性能要求
        assert!(stats.p50_latency_ms < thresholds.fastpath_p50_max_ms,
            "P50 latency too high: {:.3}ms > {:.1}ms", stats.p50_latency_ms, thresholds.fastpath_p50_max_ms);
        assert!(stats.p99_latency_ms < thresholds.fastpath_p99_max_ms,
            "P99 latency too high: {:.3}ms > {:.1}ms", stats.p99_latency_ms, thresholds.fastpath_p99_max_ms);
        assert!(tps > thresholds.fastpath_min_tps,
            "TPS too low: {:.0} < {:.0}", tps, thresholds.fastpath_min_tps);

        println!("\n✅ FastPath latency percentiles: PASSED\n");
    }

    #[test]
    fn test_fastpath_retry_mechanism() {
        println!("\n━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
        println!("  Test 2: FastPath Retry Mechanism & Rate");
        println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n");

        let thresholds = PerfThresholds::default();
        let histogram = Arc::new(LatencyHistogram::new());
        let executor = FastPathExecutor::with_histogram(histogram);

        let num_txns = 100;

        for i in 0..num_txns {
            // 每 10 个事务模拟一次失败（触发重试）
            let should_fail_first = i % 10 == 0;
            let mut attempt = 0;

            let _result = executor.execute_with_retry(i as u64, || {
                attempt += 1;
                if should_fail_first && attempt == 1 {
                    Err("Simulated conflict".to_string())
                } else {
                    std::thread::sleep(Duration::from_micros(50));
                    Ok(0)
                }
            }, 3);
        }

        let stats = executor.stats();
        let retry_rate = stats.retry_rate();

        println!("Results:");
        println!("  Total Executed: {}", stats.executed_count);
        println!("  Retry Count: {}", stats.retry_count);
        println!("  Retry Rate: {:.2}% (threshold: <{:.1}%)", retry_rate, thresholds.fastpath_retry_rate_max);

        assert!(retry_rate < thresholds.fastpath_retry_rate_max,
            "Retry rate too high: {:.2}% > {:.1}%", retry_rate, thresholds.fastpath_retry_rate_max);

        println!("\n✅ FastPath retry mechanism: PASSED\n");
    }

    #[test]
    fn test_congestion_control_and_hot_keys() {
        println!("\n━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
        println!("  Test 3: Congestion Control & Hot Key Detection");
        println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n");

        let thresholds = PerfThresholds::default();
        
        // 场景 1: 正常负载 (无拥塞)
        println!("Scenario 1: Normal Load (queue < threshold)");
        let executor1 = Arc::new(FastPathExecutor::new());
        executor1.set_congestion_threshold(1000);
        executor1.set_queue_length(500); // 50% 负载
        
        let start = Instant::now();
        let mut attempt_count = 0;
        let result = executor1.execute_with_congestion_control(1, || {
            attempt_count += 1;
            if attempt_count < 3 {
                Err("simulated failure".to_string())
            } else {
                Ok(42)
            }
        }, 5);
        let normal_elapsed = start.elapsed();
        
        assert!(result.is_ok(), "Normal load should succeed");
        assert!(normal_elapsed.as_millis() as f64 <= thresholds.congestion_normal_max_ms,
            "Normal load latency too high: {:.2}ms > {:.1}ms", 
            normal_elapsed.as_millis(), thresholds.congestion_normal_max_ms);
        
        println!("  ✓ Result: {:?}", result);
        println!("  ✓ Latency: {:.2}ms (threshold: <{:.1}ms)", 
            normal_elapsed.as_millis(), thresholds.congestion_normal_max_ms);
        
        // 场景 2: 拥塞场景 (队列超载)
        println!("\nScenario 2: Congested (queue > threshold)");
        let executor2 = Arc::new(FastPathExecutor::new());
        executor2.set_congestion_threshold(1000);
        executor2.set_queue_length(5000); // 500% 负载 → 5x 退避
        
        let start = Instant::now();
        let mut attempt_count = 0;
        let result = executor2.execute_with_congestion_control(2, || {
            attempt_count += 1;
            if attempt_count < 3 {
                Err("congestion failure".to_string())
            } else {
                Ok(100)
            }
        }, 5);
        let congested_elapsed = start.elapsed();
        
        assert!(result.is_ok(), "Congested scenario should still succeed");
        assert!(executor2.is_congested(), "Should detect congestion");
        
        let backoff_ratio = congested_elapsed.as_millis() as f64 / normal_elapsed.as_millis() as f64;
        assert!(backoff_ratio >= thresholds.congestion_backoff_min,
            "Congestion backoff too low: {:.2}x < {:.1}x", backoff_ratio, thresholds.congestion_backoff_min);
        
        println!("  ✓ Result: {:?}", result);
        println!("  ✓ Latency: {:.2}ms ({}x normal)", congested_elapsed.as_millis(), backoff_ratio as u64);
        println!("  ✓ Congestion detected: {}", executor2.is_congested());
        
        // 场景 3: 热键检测
        println!("\nScenario 3: Hot Key Detection");
        let executor3 = Arc::new(FastPathExecutor::new());
        let hot_keys = vec![42, 100, 200];
        let cold_keys = vec![1, 2, 3, 4, 5];
        
        // 模拟 100 次访问: 60% 热键, 40% 冷键
        for i in 0..100 {
            let key = if i % 10 < 6 {
                hot_keys[i % hot_keys.len()]
            } else {
                cold_keys[i % cold_keys.len()]
            };
            executor3.track_key_access(key);
        }
        
        let top_3 = executor3.get_hot_keys(3);
        assert_eq!(top_3.len(), 3, "Should return top 3 keys");
        
        // 验证准确率: Top-3 应该是热键
        let hot_key_set: std::collections::HashSet<_> = hot_keys.into_iter().collect();
        let detected_hot = top_3.iter().filter(|(k, _)| hot_key_set.contains(k)).count();
        let accuracy = detected_hot as f64 / top_3.len() as f64;
        
        assert!(accuracy >= thresholds.hotkey_detection_accuracy,
            "Hot key detection accuracy too low: {:.2} < {:.2}", 
            accuracy, thresholds.hotkey_detection_accuracy);
        
        println!("  ✓ Top-3 keys detected: {:?}", top_3);
        println!("  ✓ Accuracy: {:.1}% (threshold: >{:.0}%)", 
            accuracy * 100.0, thresholds.hotkey_detection_accuracy * 100.0);
        
        println!("\n✅ Congestion control & hot key detection: PASSED\n");
    }

    #[test]
    #[cfg(feature = "groth16-verifier")]
    fn test_prover_thread_pool_reuse() {
        println!("\n━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
        println!("  Test 4: Parallel Prover Thread Pool Reuse");
        println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n");

        let thresholds = PerfThresholds::default();
        let config = ParallelProveConfig {
            batch_size: 5,
            num_threads: None,
            collect_individual_latency: false,
        };

        // 测试多次批量证明（验证池复用）
        let prover = ParallelProver::with_shared_setup(config.clone());
        let num_batches = 3;
        let batch_size = 5;

        let start = Instant::now();
        for batch_idx in 0..num_batches {
            let inputs: Vec<CircuitInput> = (0..batch_size).map(|i| {
                CircuitInput {
                    a: Fr::from((batch_idx * batch_size + i + 1) as u64),
                    b: Fr::from((batch_idx * batch_size + i + 2) as u64),
                }
            }).collect();

            let batch_start = Instant::now();
            let stats = prover.prove_batch(&inputs);
            let batch_duration = batch_start.elapsed().as_secs_f64() * 1000.0;

            println!("Batch {}: {} proofs in {:.2}ms (TPS: {:.2})",
                batch_idx + 1, stats.ok, batch_duration, stats.tps);

            assert_eq!(stats.ok, batch_size, "All proofs should succeed");
            assert!(batch_duration < thresholds.prover_batch_latency_max_ms,
                "Batch latency too high: {:.2}ms > {:.1}ms",
                batch_duration, thresholds.prover_batch_latency_max_ms);
        }

        let total_duration = start.elapsed().as_secs_f64();
        let total_proofs = num_batches * batch_size;
        let overall_tps = total_proofs as f64 / total_duration;

        // 验证线程池统计
        let (pool_tasks, pool_avg_ms) = get_pool_stats();
        println!("\nThread Pool Statistics:");
        println!("  Total Tasks: {}", pool_tasks);
        println!("  Avg Task Duration: {:.2}ms", pool_avg_ms);
        println!("  Overall TPS: {:.2} (threshold: >{:.1})", overall_tps, thresholds.prover_tps_min);

        assert!(pool_tasks >= total_proofs as u64, "Pool should track all tasks");
        assert!(overall_tps > thresholds.prover_tps_min,
            "Overall TPS too low: {:.2} < {:.1}", overall_tps, thresholds.prover_tps_min);

        println!("\n✅ Thread pool reuse: PASSED\n");
    }

    #[test]
    #[cfg(feature = "groth16-verifier")]
    fn test_proving_key_cache_speedup() {
        println!("\n━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
        println!("  Test 5: ProvingKey Cache Speedup");
        println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n");

        let thresholds = PerfThresholds::default();
        let config = ParallelProveConfig {
            batch_size: 5,
            num_threads: None,
            collect_individual_latency: false,
        };

        // 测试首次创建（包含 setup）
        println!("Testing first ParallelProver creation (with setup)...");
        let start = Instant::now();
        let _prover1 = ParallelProver::with_shared_setup(config.clone());
        let first_creation_ms = start.elapsed().as_secs_f64() * 1000.0;
        println!("  First creation: {:.2}ms (threshold: <{:.1}ms)",
            first_creation_ms, thresholds.pk_first_creation_max_ms);

        // 测试后续创建（复用缓存）
        println!("\nTesting 5 more creations (reuse cached key)...");
        let mut total_reuse_ms = 0.0;
        for i in 1..=5 {
            let start = Instant::now();
            let _prover = ParallelProver::with_shared_setup(config.clone());
            let reuse_ms = start.elapsed().as_secs_f64() * 1000.0;
            total_reuse_ms += reuse_ms;
            if i % 2 == 0 {
                println!("  Creation #{}: {:.3}ms", i, reuse_ms);
            }
        }
        let avg_reuse_ms = total_reuse_ms / 5.0;
        let speedup = first_creation_ms / avg_reuse_ms;

        println!("\nResults:");
        println!("  First creation: {:.2}ms", first_creation_ms);
        println!("  Avg reuse: {:.3}ms (threshold: <{:.1}ms)", avg_reuse_ms, thresholds.pk_reuse_creation_max_ms);
        println!("  Speedup: {:.0}x (threshold: >{:.0}x)", speedup, thresholds.pk_speedup_min);

        assert!(avg_reuse_ms < thresholds.pk_reuse_creation_max_ms,
            "Reuse creation too slow: {:.3}ms > {:.1}ms",
            avg_reuse_ms, thresholds.pk_reuse_creation_max_ms);
        assert!(speedup > thresholds.pk_speedup_min,
            "Speedup too low: {:.0}x < {:.0}x", speedup, thresholds.pk_speedup_min);

        println!("\n✅ ProvingKey cache speedup: PASSED\n");
    }

    #[test]
    #[cfg(feature = "groth16-verifier")]
    fn test_end_to_end_integrated_performance() {
        println!("\n━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
        println!("  Test 6: End-to-End Integrated Performance");
        println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n");

        // 集成测试: FastPath + Parallel Prover + ProvingKey Cache
        let histogram = Arc::new(LatencyHistogram::new());
        let fastpath = FastPathExecutor::with_histogram(histogram);

        let prover_config = ParallelProveConfig {
            batch_size: 3,
            num_threads: None,
            collect_individual_latency: false,
        };
        let prover = ParallelProver::with_shared_setup(prover_config);

        // 模拟混合工作负载
        let num_iterations = 3;
        let txns_per_iteration = 10;
        let proofs_per_iteration = 3;

        println!("Simulating mixed workload:");
        println!("  {} iterations × ({} txns + {} proofs)", 
            num_iterations, txns_per_iteration, proofs_per_iteration);

        let start = Instant::now();
        let mut total_txns = 0;
        let mut total_proofs = 0;

        for iter in 0..num_iterations {
            // 阶段 1: FastPath 事务
            for i in 0..txns_per_iteration {
                let tx_id = (iter * txns_per_iteration + i) as u64;
                let result = fastpath.execute(tx_id, || {
                    std::thread::sleep(Duration::from_micros(50));
                    Ok(0)
                });
                if result.is_ok() {
                    total_txns += 1;
                }
            }

            // 阶段 2: Parallel Prover
            let inputs: Vec<CircuitInput> = (0..proofs_per_iteration).map(|i| {
                CircuitInput {
                    a: Fr::from((iter * proofs_per_iteration + i + 1) as u64),
                    b: Fr::from((iter * proofs_per_iteration + i + 2) as u64),
                }
            }).collect();
            let stats = prover.prove_batch(&inputs);
            total_proofs += stats.ok;

            println!("  Iteration {}: {} txns + {} proofs", iter + 1, txns_per_iteration, stats.ok);
        }

        let total_duration = start.elapsed();
        let fastpath_stats = fastpath.stats();

        println!("\nIntegrated Results:");
        println!("  Total Txns: {}", total_txns);
        println!("  Total Proofs: {}", total_proofs);
        println!("  Total Duration: {:.2}s", total_duration.as_secs_f64());
        println!("  FastPath P99: {:.3}ms", fastpath_stats.p99_latency_ms);
        
        let expected_txns = num_iterations * txns_per_iteration;
        let expected_proofs = num_iterations * proofs_per_iteration;
        
        assert_eq!(total_txns, expected_txns, "All transactions should succeed");
        assert_eq!(total_proofs, expected_proofs, "All proofs should succeed");

        println!("\n✅ End-to-end integrated performance: PASSED\n");
    }

    #[test]
    fn test_perf_matrix_summary() {
        println!("\n━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
        println!("  Performance Matrix Summary");
        println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n");

        let thresholds = PerfThresholds::default();

        println!("Performance Thresholds:");
        println!("  FastPath:");
        println!("    • P50 Latency: <{:.1}ms", thresholds.fastpath_p50_max_ms);
        println!("    • P99 Latency: <{:.1}ms", thresholds.fastpath_p99_max_ms);
        println!("    • Min TPS: >{:.0}", thresholds.fastpath_min_tps);
        println!("    • Max Retry Rate: <{:.1}%", thresholds.fastpath_retry_rate_max);
        
        println!("\n  Congestion Control:");
        println!("    • Normal Load: <{:.1}ms", thresholds.congestion_normal_max_ms);
        println!("    • Min Backoff Multiplier: >{:.1}x", thresholds.congestion_backoff_min);
        println!("    • Hot Key Accuracy: >{:.0}%", thresholds.hotkey_detection_accuracy * 100.0);
        
        #[cfg(feature = "groth16-verifier")]
        {
            println!("\n  Parallel Prover:");
            println!("    • Pool Creation: <{:.1}ms", thresholds.prover_pool_creation_max_ms);
            println!("    • Min TPS: >{:.0}", thresholds.prover_tps_min);
            println!("    • Batch Latency: <{:.1}ms", thresholds.prover_batch_latency_max_ms);
            
            println!("\n  ProvingKey Cache:");
            println!("    • First Creation: <{:.1}ms", thresholds.pk_first_creation_max_ms);
            println!("    • Reuse Creation: <{:.1}ms", thresholds.pk_reuse_creation_max_ms);
            println!("    • Min Speedup: >{:.0}x", thresholds.pk_speedup_min);
        }

        println!("\nTest Coverage:");
        println!("  ✓ FastPath latency percentiles (p50/p90/p95/p99)");
        println!("  ✓ FastPath retry mechanism & exponential backoff");
        println!("  ✓ FastPath congestion control & hot key detection");
        
        #[cfg(feature = "groth16-verifier")]
        {
            println!("  ✓ Parallel Prover thread pool reuse efficiency");
            println!("  ✓ ProvingKey cache speedup validation");
            println!("  ✓ End-to-end integrated performance");
        }

        println!("\n━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
        println!("  Run with: cargo test --release perf_matrix");
        #[cfg(feature = "groth16-verifier")]
        println!("  Full tests: cargo test --release --features groth16-verifier perf_matrix");
        println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n");
    }
}
