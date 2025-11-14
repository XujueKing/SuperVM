//! 自适应优化演示
//!
//! 演示智能任务大小估算和自适应优化策略:
//! 1. 小任务 (fib 20): 只用批量,不用并行/缓存
//! 2. 中等任务 (fib 100): 批量 + 并行
//! 3. 大任务 (fib 500): 批量 + 并行 + 缓存

use l2_executor::optimized::{BatchProcessor, CachedZkVm, estimate_task_size};
use l2_executor::program::FibonacciProgram;
use std::time::Instant;

fn main() {
    println!("=== L2 Executor 自适应优化演示 ===\n");

    // 测试 1: 任务大小估算
    println!("=== Test 1: 任务大小估算 ===");
    
    let tasks = vec![
        ("fib(20)", FibonacciProgram::new(20)),
        ("fib(50)", FibonacciProgram::new(50)),
        ("fib(100)", FibonacciProgram::new(100)),
        ("fib(200)", FibonacciProgram::new(200)),
    ];
    
    for (name, program) in &tasks {
        let estimate = estimate_task_size(program, &[]);
        println!(
            "{:12} - 预估: {:>6.2}µs, 并行: {:5}, 缓存: {:5}",
            name,
            estimate.estimated_time.as_micros(),
            if estimate.recommend_parallel { "✅" } else { "❌" },
            if estimate.recommend_cache { "✅" } else { "❌" }
        );
    }
    println!();

    // 测试 2: 自适应批量处理
    println!("=== Test 2: 自适应批量处理 ===");
    
    test_adaptive_batch("小任务 (fib 20-30)", 20, 30, 20);
    test_adaptive_batch("中等任务 (fib 100-110)", 100, 110, 20);
    test_adaptive_batch("大任务 (fib 200-210)", 200, 210, 20);
    println!();

    // 测试 3: 自适应缓存
    println!("=== Test 3: 自适应缓存 ===");
    
    test_adaptive_cache("小任务 (fib 20)", FibonacciProgram::new(20), 100);
    test_adaptive_cache("中等任务 (fib 100)", FibonacciProgram::new(100), 100);
    test_adaptive_cache("大任务 (fib 200)", FibonacciProgram::new(200), 100);
    println!();

    // 测试 4: 综合对比
    println!("=== Test 4: 综合性能对比 ===");
    
    compare_strategies("小任务", 20, 30);
    compare_strategies("中等任务", 100, 110);
    println!();

    println!("✅ 自适应优化演示完成!");
}

/// 测试自适应批量处理
fn test_adaptive_batch(name: &str, start: usize, end: usize, count: usize) {
    let processor = BatchProcessor::new();
    
    // 生成任务
    let mut programs = Vec::new();
    for i in 0..count {
        let n = start + i % (end - start + 1);
        programs.push(FibonacciProgram::new(n as u32));
    }
    let witnesses: Vec<&[u64]> = vec![&[]; count];
    
    // 方式 1: 顺序批量
    let start_time = Instant::now();
    let _proofs = processor.prove_batch(&programs, &witnesses).unwrap();
    let sequential_time = start_time.elapsed();
    
    // 方式 2: 并行批量
    let start_time = Instant::now();
    let _proofs = processor.prove_batch_parallel(&programs, &witnesses).unwrap();
    let parallel_time = start_time.elapsed();
    
    // 方式 3: 自适应批量
    let start_time = Instant::now();
    let _proofs = processor.prove_batch_auto(&programs, &witnesses).unwrap();
    let auto_time = start_time.elapsed();
    
    // 计算加速比
    let parallel_speedup = sequential_time.as_micros() as f64 / parallel_time.as_micros() as f64;
    let auto_speedup = sequential_time.as_micros() as f64 / auto_time.as_micros() as f64;
    
    println!("{}", name);
    println!("  顺序: {:>8.2}µs", sequential_time.as_micros());
    println!("  并行: {:>8.2}µs ({}x)", parallel_time.as_micros(), format!("{:.2}", parallel_speedup));
    println!("  自动: {:>8.2}µs ({}x) ← 推荐", auto_time.as_micros(), format!("{:.2}", auto_speedup));
}

/// 测试自适应缓存
fn test_adaptive_cache(name: &str, program: FibonacciProgram, requests: usize) {
    let vm = CachedZkVm::new(100);
    
    // 创建一个内部 VM 用于无缓存测试
    use l2_executor::zkvm::TraceZkVm;
    let plain_vm = TraceZkVm::default();
    
    // 方式 1: 不使用缓存 (直接计算)
    let start_time = Instant::now();
    for _ in 0..requests {
        let _ = plain_vm.prove(&program, &[]).unwrap();
    }
    let no_cache_time = start_time.elapsed();
    
    // 方式 2: 使用缓存
    vm.clear_cache();
    let start_time = Instant::now();
    for _ in 0..requests {
        let _ = vm.prove(&program, &[]).unwrap();
    }
    let cache_time = start_time.elapsed();
    let cache_stats = vm.cache_stats();
    
    // 方式 3: 智能缓存
    vm.clear_cache();
    let start_time = Instant::now();
    for _ in 0..requests {
        let _ = vm.prove_smart(&program, &[]).unwrap();
    }
    let smart_time = start_time.elapsed();
    
    // 计算加速比
    let cache_speedup = no_cache_time.as_micros() as f64 / cache_time.as_micros() as f64;
    let smart_speedup = no_cache_time.as_micros() as f64 / smart_time.as_micros() as f64;
    
    println!("{}", name);
    println!("  无缓存: {:>8.2}µs", no_cache_time.as_micros());
    println!("  有缓存: {:>8.2}µs ({}x, 命中率: {:.1}%)", 
             cache_time.as_micros(), 
             format!("{:.2}", cache_speedup),
             cache_stats.hit_rate() * 100.0);
    println!("  智能:   {:>8.2}µs ({}x) ← 推荐", smart_time.as_micros(), format!("{:.2}", smart_speedup));
}

/// 对比不同策略
fn compare_strategies(name: &str, start: usize, end: usize) {
    let processor = BatchProcessor::new();
    let count = 30;
    
    // 生成任务
    let mut programs = Vec::new();
    for i in 0..count {
        let n = start + i % (end - start + 1);
        programs.push(FibonacciProgram::new(n as u32));
    }
    let witnesses: Vec<&[u64]> = vec![&[]; count];
    
    // 基准: 顺序批量
    let start_time = Instant::now();
    let _proofs = processor.prove_batch(&programs, &witnesses).unwrap();
    let baseline = start_time.elapsed();
    
    // 策略 1: 强制并行
    let start_time = Instant::now();
    let _proofs = processor.prove_batch_parallel(&programs, &witnesses).unwrap();
    let force_parallel = start_time.elapsed();
    
    // 策略 2: 自适应
    let start_time = Instant::now();
    let _proofs = processor.prove_batch_auto(&programs, &witnesses).unwrap();
    let adaptive = start_time.elapsed();
    
    // 计算加速比
    let parallel_speedup = baseline.as_micros() as f64 / force_parallel.as_micros() as f64;
    let adaptive_speedup = baseline.as_micros() as f64 / adaptive.as_micros() as f64;
    
    println!("{} ({} proofs)", name, count);
    println!("  基准 (顺序):  {:>8.2}µs", baseline.as_micros());
    println!("  强制并行:     {:>8.2}µs ({:.2}x)", force_parallel.as_micros(), parallel_speedup);
    println!("  自适应智能:   {:>8.2}µs ({:.2}x) ← 最佳", adaptive.as_micros(), adaptive_speedup);
    
    // 判断哪个更好
    let best = if adaptive < force_parallel && adaptive < baseline {
        "自适应智能 ✅"
    } else if force_parallel < baseline {
        "强制并行"
    } else {
        "顺序执行"
    };
    println!("  → 最佳策略: {}", best);
}
