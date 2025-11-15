use anyhow::Result;
use l2_executor::{
    BatchProcessor, CachedZkVm, FibonacciProgram, TraceZkVm,
};
use std::time::Instant;

fn main() -> Result<()> {
    env_logger::init();

    println!("\n╔═══════════════════════════════════════════════════════╗");
    println!("║  L2 Executor - Performance Optimization Demo         ║");
    println!("╚═══════════════════════════════════════════════════════╝\n");

    // 测试 1: 批量处理 vs 单个处理
    benchmark_batch_vs_single()?;

    // 测试 2: 并行 vs 顺序
    benchmark_parallel_vs_sequential()?;

    // 测试 3: 缓存效果
    benchmark_cache_performance()?;

    // 测试 4: 综合性能提升
    benchmark_combined_optimizations()?;

    println!("\n✅ All performance benchmarks completed!\n");

    Ok(())
}

/// 测试 1: 批量处理 vs 单个处理
fn benchmark_batch_vs_single() -> Result<()> {
    println!("═══ Test 1: Batch Processing vs Individual ═══\n");

    let count: usize = 20;
    let programs: Vec<_> = (0..count).map(|i| FibonacciProgram::new(10 + i as u32)).collect();
    let witnesses: Vec<&[u64]> = vec![&[]; count];

    // 单个处理
    let vm = TraceZkVm::default();
    let start = Instant::now();
    for prog in &programs {
        vm.prove(prog, &[])?;
    }
    let individual_time = start.elapsed();

    // 批量处理
    let processor = BatchProcessor::new();
    let start = Instant::now();
    let _proofs = processor.prove_batch(&programs, &witnesses)?;
    let batch_time = start.elapsed();

    println!("  Individual processing ({} proofs):", count);
    println!("    Time: {:?}", individual_time);
    println!("    Avg per proof: {:?}", individual_time / count as u32);
    println!();
    println!("  Batch processing ({} proofs):", count);
    println!("    Time: {:?}", batch_time);
    println!("    Avg per proof: {:?}", batch_time / count as u32);
    println!();
    println!("  Speedup: {:.2}x", individual_time.as_secs_f64() / batch_time.as_secs_f64());
    println!();

    Ok(())
}

/// 测试 2: 并行 vs 顺序
fn benchmark_parallel_vs_sequential() -> Result<()> {
    println!("═══ Test 2: Parallel vs Sequential ═══\n");

    let count: usize = 20;
    let programs: Vec<_> = (0..count).map(|i| FibonacciProgram::new(20 + i as u32)).collect();
    let witnesses: Vec<&[u64]> = vec![&[]; count];

    let processor = BatchProcessor::new();

    // 顺序执行
    let start = Instant::now();
    let _proofs = processor.prove_batch(&programs, &witnesses)?;
    let sequential_time = start.elapsed();

    // 并行执行
    let start = Instant::now();
    let _proofs = processor.prove_batch_parallel(&programs, &witnesses)?;
    let parallel_time = start.elapsed();

    println!("  Sequential processing ({} proofs):", count);
    println!("    Time: {:?}", sequential_time);
    println!("    Avg per proof: {:?}", sequential_time / count as u32);
    println!();
    println!("  Parallel processing ({} proofs):", count);
    println!("    Time: {:?}", parallel_time);
    println!("    Avg per proof: {:?}", parallel_time / count as u32);
    println!();
    println!("  Speedup: {:.2}x", sequential_time.as_secs_f64() / parallel_time.as_secs_f64());
    println!("  CPU cores utilized: ~{}", num_cpus::get());
    println!();

    Ok(())
}

/// 测试 3: 缓存效果
fn benchmark_cache_performance() -> Result<()> {
    println!("═══ Test 3: Cache Performance ═══\n");

    let vm = CachedZkVm::new(100);
    let program = FibonacciProgram::new(50);

    // 第一次 - 缓存未命中
    let start = Instant::now();
    let _proof1 = vm.prove(&program, &[])?;
    let miss_time = start.elapsed();

    // 第二次 - 缓存命中
    let start = Instant::now();
    let _proof2 = vm.prove(&program, &[])?;
    let hit_time = start.elapsed();

    println!("  Cache miss (first call):");
    println!("    Time: {:?}", miss_time);
    println!();
    println!("  Cache hit (second call):");
    println!("    Time: {:?}", hit_time);
    println!();
    println!("  Speedup: {:.2}x", miss_time.as_secs_f64() / hit_time.as_secs_f64());
    println!();

    // 批量测试缓存命中率
    println!("  Cache hit rate test (100 requests, 10 unique programs):");
    vm.clear_cache();

    let programs: Vec<_> = (0..10).map(|i| FibonacciProgram::new(10 + i * 5)).collect();
    
    for i in 0..100 {
        let prog = &programs[i % 10]; // 重复使用 10 个程序
        vm.prove(prog, &[])?;
    }

    let stats = vm.cache_stats();
    println!("    {}", stats);
    println!("    Expected hit rate: ~90%");
    println!();

    Ok(())
}

/// 测试 4: 综合优化效果
fn benchmark_combined_optimizations() -> Result<()> {
    println!("═══ Test 4: Combined Optimizations ═══\n");

    let count: usize = 30;
    let programs: Vec<_> = (0..count).map(|i| FibonacciProgram::new(20 + (i % 10) as u32)).collect();

    // 基准: 单个无缓存
    let vm = TraceZkVm::default();
    let start = Instant::now();
    for prog in &programs {
        vm.prove(prog, &[])?;
    }
    let baseline_time = start.elapsed();

    // 优化 1: 批量 + 顺序
    let processor = BatchProcessor::new();
    let witnesses: Vec<&[u64]> = vec![&[]; count];
    let start = Instant::now();
    let _proofs = processor.prove_batch(&programs, &witnesses)?;
    let batch_time = start.elapsed();

    // 优化 2: 批量 + 并行
    let start = Instant::now();
    let _proofs = processor.prove_batch_parallel(&programs, &witnesses)?;
    let batch_parallel_time = start.elapsed();

    // 优化 3: 缓存 (有重复程序)
    let cached_vm = CachedZkVm::new(100);
    let start = Instant::now();
    for prog in &programs {
        cached_vm.prove(prog, &[])?;
    }
    let cache_time = start.elapsed();

    println!("  Baseline (individual, no cache, {} proofs):", count);
    println!("    Time: {:?}", baseline_time);
    println!();
    println!("  Optimization 1 (batch sequential):");
    println!("    Time: {:?}", batch_time);
    println!("    Speedup: {:.2}x", baseline_time.as_secs_f64() / batch_time.as_secs_f64());
    println!();
    println!("  Optimization 2 (batch parallel):");
    println!("    Time: {:?}", batch_parallel_time);
    println!("    Speedup: {:.2}x", baseline_time.as_secs_f64() / batch_parallel_time.as_secs_f64());
    println!();
    println!("  Optimization 3 (cache, ~67% hit rate):");
    println!("    Time: {:?}", cache_time);
    println!("    Speedup: {:.2}x", baseline_time.as_secs_f64() / cache_time.as_secs_f64());
    let stats = cached_vm.cache_stats();
    println!("    {}", stats);
    println!();

    println!("  📊 Summary:");
    println!("    Batch:    {:.2}x faster", baseline_time.as_secs_f64() / batch_time.as_secs_f64());
    println!("    Parallel: {:.2}x faster", baseline_time.as_secs_f64() / batch_parallel_time.as_secs_f64());
    println!("    Cache:    {:.2}x faster", baseline_time.as_secs_f64() / cache_time.as_secs_f64());
    println!();

    Ok(())
}
