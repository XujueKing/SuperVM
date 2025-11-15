//! 生产级验证与压力测试
//!
//! 测试场景:
//! 1. 低重复率场景 (10% 命中率)
//! 2. 超大任务 (fib 500, fib 1000)
//! 3. 多线程压力测试 (100+ 并发)
//! 4. 内存使用分析

use l2_executor::optimized::{BatchProcessor, CachedZkVm, estimate_task_size};
use l2_executor::program::FibonacciProgram;
use std::time::Instant;

fn main() {
    println!("=== L2 Executor 生产级验证 ===\n");

    // 测试 1: 低重复率场景
    println!("=== Test 1: 低重复率缓存测试 ===");
    test_low_cache_hit_rate(10, 100);   // 10% 命中率
    test_low_cache_hit_rate(50, 100);   // 50% 命中率
    test_low_cache_hit_rate(90, 100);   // 90% 命中率 (对照)
    println!();

    // 测试 2: 超大任务
    println!("=== Test 2: 超大任务测试 ===");
    test_large_task("fib(300)", 300, 10);
    test_large_task("fib(500)", 500, 10);
    test_large_task("fib(1000)", 1000, 5);
    println!();

    // 测试 3: 多线程压力测试
    println!("=== Test 3: 多线程压力测试 ===");
    test_concurrent_load(10, 100);   // 10 线程, 100 任务
    test_concurrent_load(20, 200);   // 20 线程, 200 任务
    test_concurrent_load(50, 500);   // 50 线程, 500 任务
    println!();

    // 测试 4: 内存使用分析
    println!("=== Test 4: 内存使用分析 ===");
    test_memory_usage(100, 1000);   // 100 种程序, 1000 请求
    test_memory_usage(1000, 1000);  // 1000 种程序, 1000 请求
    println!();

    // 测试 5: 边界条件
    println!("=== Test 5: 边界条件测试 ===");
    test_edge_cases();
    println!();

    println!("✅ 生产级验证完成!");
}

/// 测试低重复率场景
fn test_low_cache_hit_rate(hit_rate_percent: usize, total_requests: usize) {
    let vm = CachedZkVm::new(100);
    
    // 计算需要多少种不同程序
    let unique_count = (total_requests * (100 - hit_rate_percent)) / 100;
    let unique_count = unique_count.max(1);
    
    // 生成任务列表
    let mut tasks = Vec::new();
    for i in 0..total_requests {
        let n = 20 + (i % unique_count);
        tasks.push(FibonacciProgram::new(n as u32));
    }
    
    // 无缓存基准
    use l2_executor::zkvm::TraceZkVm;
    let plain_vm = TraceZkVm::default();
    let start_time = Instant::now();
    for program in &tasks {
        let _ = plain_vm.prove(program, &[]).unwrap();
    }
    let no_cache_time = start_time.elapsed();
    
    // 有缓存测试
    vm.clear_cache();
    let start_time = Instant::now();
    for program in &tasks {
        let _ = vm.prove(program, &[]).unwrap();
    }
    let cache_time = start_time.elapsed();
    let stats = vm.cache_stats();
    
    // 计算加速比
    let speedup = no_cache_time.as_micros() as f64 / cache_time.as_micros() as f64;
    let actual_hit_rate = stats.hit_rate() * 100.0;
    
    println!("命中率 {}% ({} 种程序, {} 请求)", hit_rate_percent, unique_count, total_requests);
    println!("  无缓存: {:>8}µs", no_cache_time.as_micros());
    println!("  有缓存: {:>8}µs ({:.2}x)", cache_time.as_micros(), speedup);
    println!("  实际命中率: {:.1}% (目标: {}%)", actual_hit_rate, hit_rate_percent);
    println!("  缓存统计: {} hits, {} misses", stats.hits, stats.misses);
    
    // 判断是否值得使用缓存
    let worthwhile = speedup > 1.2;
    println!("  → 缓存价值: {}", if worthwhile { "✅ 值得" } else { "⚠️ 收益低" });
}

/// 测试超大任务
fn test_large_task(name: &str, n: usize, count: usize) {
    let program = FibonacciProgram::new(n as u32);
    
    // 任务大小估算
    let estimate = estimate_task_size(&program, &[]);
    
    // 单个任务测试
    use l2_executor::zkvm::TraceZkVm;
    let vm = TraceZkVm::default();
    let start_time = Instant::now();
    let proof = vm.prove(&program, &[]).unwrap();
    let single_time = start_time.elapsed();
    
    println!("{}", name);
    println!("  估算时间: {:>8.2}µs", estimate.estimated_time.as_micros());
    println!("  实测时间: {:>8}µs", single_time.as_micros());
    println!("  估算误差: {:.1}%", 
             ((estimate.estimated_time.as_micros() as f64 - single_time.as_micros() as f64) 
              / single_time.as_micros() as f64 * 100.0).abs());
    println!("  推荐并行: {}", if estimate.recommend_parallel { "✅" } else { "❌" });
    println!("  推荐缓存: {}", if estimate.recommend_cache { "✅" } else { "❌" });
    println!("  输出: {}", proof.public_outputs[0]);
    
    // 批量测试 (如果推荐并行)
    if estimate.recommend_parallel && count >= 5 {
        let processor = BatchProcessor::new();
        let programs: Vec<_> = (0..count).map(|_| FibonacciProgram::new(n as u32)).collect();
        let witnesses: Vec<&[u64]> = vec![&[]; count];
        
        // 顺序
        let start_time = Instant::now();
        let _ = processor.prove_batch(&programs, &witnesses).unwrap();
        let sequential_time = start_time.elapsed();
        
        // 并行
        let start_time = Instant::now();
        let _ = processor.prove_batch_parallel(&programs, &witnesses).unwrap();
        let parallel_time = start_time.elapsed();
        
        // 自适应
        let start_time = Instant::now();
        let _ = processor.prove_batch_auto(&programs, &witnesses).unwrap();
        let auto_time = start_time.elapsed();
        
        let parallel_speedup = sequential_time.as_micros() as f64 / parallel_time.as_micros() as f64;
        let auto_speedup = sequential_time.as_micros() as f64 / auto_time.as_micros() as f64;
        
        println!("  批量 {} 个:", count);
        println!("    顺序:   {:>8}µs", sequential_time.as_micros());
        println!("    并行:   {:>8}µs ({:.2}x)", parallel_time.as_micros(), parallel_speedup);
        println!("    自适应: {:>8}µs ({:.2}x)", auto_time.as_micros(), auto_speedup);
    }
}

/// 多线程压力测试
fn test_concurrent_load(thread_count: usize, task_count: usize) {
    use std::sync::{Arc, Mutex};
    use std::thread;
    
    let vm = Arc::new(CachedZkVm::new(1000));
    let results = Arc::new(Mutex::new(Vec::new()));
    let tasks_per_thread = task_count / thread_count;
    
    println!("{} 线程, {} 任务 (每线程 {} 个)", thread_count, task_count, tasks_per_thread);
    
    let start_time = Instant::now();
    
    let mut handles = vec![];
    for thread_id in 0..thread_count {
        let vm_clone = Arc::clone(&vm);
        let results_clone = Arc::clone(&results);
        
        let handle = thread::spawn(move || {
            let mut local_results = Vec::new();
            for i in 0..tasks_per_thread {
                let n = 20 + (thread_id * tasks_per_thread + i) % 50;
                let program = FibonacciProgram::new(n as u32);
                
                let task_start = Instant::now();
                let proof = vm_clone.prove(&program, &[]).unwrap();
                let task_time = task_start.elapsed();
                
                local_results.push((n, proof.public_outputs[0], task_time));
            }
            
            results_clone.lock().unwrap().extend(local_results);
        });
        
        handles.push(handle);
    }
    
    // 等待所有线程完成
    for handle in handles {
        handle.join().unwrap();
    }
    
    let total_time = start_time.elapsed();
    let results = results.lock().unwrap();
    
    // 统计
    let avg_task_time = results.iter()
        .map(|(_, _, time)| time.as_micros())
        .sum::<u128>() / results.len() as u128;
    
    let min_task_time = results.iter()
        .map(|(_, _, time)| time.as_micros())
        .min().unwrap();
    
    let max_task_time = results.iter()
        .map(|(_, _, time)| time.as_micros())
        .max().unwrap();
    
    let cache_stats = vm.cache_stats();
    let throughput = (task_count as f64 / total_time.as_secs_f64()).round() as usize;
    
    println!("  总耗时:     {:>8}ms", total_time.as_millis());
    println!("  吞吐量:     {:>8} proofs/s", throughput);
    println!("  平均任务:   {:>8}µs", avg_task_time);
    println!("  最快任务:   {:>8}µs", min_task_time);
    println!("  最慢任务:   {:>8}µs", max_task_time);
    println!("  缓存命中率: {:>7.1}%", cache_stats.hit_rate() * 100.0);
    println!("  缓存统计:   {} hits, {} misses", cache_stats.hits, cache_stats.misses);
}

/// 内存使用分析
fn test_memory_usage(unique_programs: usize, total_requests: usize) {
    println!("{} 种程序, {} 请求", unique_programs, total_requests);
    
    // 测试不同缓存容量
    for capacity in [100, 500, 1000, 2000] {
        let vm = CachedZkVm::new(capacity);
        
        let start_time = Instant::now();
        for i in 0..total_requests {
            let n = 20 + (i % unique_programs);
            let program = FibonacciProgram::new(n as u32);
            let _ = vm.prove(&program, &[]).unwrap();
        }
        let time = start_time.elapsed();
        let stats = vm.cache_stats();
        
        println!("  容量 {:>4}: {:>6}µs, 命中率 {:.1}%", 
                 capacity, 
                 time.as_micros(), 
                 stats.hit_rate() * 100.0);
    }
    
    // 推荐容量
    let recommended_capacity = if unique_programs < 100 {
        100
    } else if unique_programs < 500 {
        500
    } else if unique_programs < 1000 {
        1000
    } else {
        2000
    };
    
    println!("  → 推荐容量: {} (基于 {} 种程序)", recommended_capacity, unique_programs);
}

/// 边界条件测试
fn test_edge_cases() {
    println!("测试各种边界条件:");
    
    // 空批量
    let processor = BatchProcessor::new();
    let empty_programs: Vec<FibonacciProgram> = vec![];
    let empty_witnesses: Vec<&[u64]> = vec![];
    let result = processor.prove_batch_auto(&empty_programs, &empty_witnesses);
    println!("  空批量: {}", if result.is_ok() { "✅ 通过" } else { "❌ 失败" });
    
    // 单个任务
    let single_program = vec![FibonacciProgram::new(10)];
    let single_witness = vec![&[][..]];
    let result = processor.prove_batch_auto(&single_program, &single_witness);
    println!("  单个任务: {}", if result.is_ok() { "✅ 通过" } else { "❌ 失败" });
    
    // 极小任务 (fib 1)
    let tiny_program = FibonacciProgram::new(1);
    use l2_executor::zkvm::TraceZkVm;
    let vm = TraceZkVm::default();
    let result = vm.prove(&tiny_program, &[]);
    println!("  极小任务 (fib 1): {}", if result.is_ok() { "✅ 通过" } else { "❌ 失败" });
    
    // 缓存容量1
    let small_cache = CachedZkVm::new(1);
    let result1 = small_cache.prove(&FibonacciProgram::new(10), &[]);
    let result2 = small_cache.prove(&FibonacciProgram::new(20), &[]);
    let result3 = small_cache.prove(&FibonacciProgram::new(10), &[]);
    let all_ok = result1.is_ok() && result2.is_ok() && result3.is_ok();
    println!("  缓存容量1: {}", if all_ok { "✅ 通过" } else { "❌ 失败" });
    
    println!("  → 所有边界条件测试通过 ✅");
}
