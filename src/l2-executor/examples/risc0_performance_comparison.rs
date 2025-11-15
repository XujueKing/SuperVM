//! Session 11: RISC0 Backend 性能对比测试
//!
//! 对比 Trace vs RISC0 backend 的性能差异:
//! - 证明生成时间
//! - 证明验证时间
//! - 证明大小
//! - 自适应策略兼容性
//!
//! 运行方式 (需要 WSL):
//! ```bash
//! # 在 WSL 中编译并运行
//! wsl bash -c "cd /mnt/d/WEB3_AI开发/虚拟机开发 && cargo build --release --features risc0-poc --example risc0_performance_comparison"
//! wsl bash -c "cd /mnt/d/WEB3_AI开发/虚拟机开发 && cargo run --release --features risc0-poc --example risc0_performance_comparison"
//! ```

#[cfg(feature = "risc0-poc")]
use l2_executor::{
    Risc0Backend, ZkVmBackend, 
    FibonacciProgram, TraceZkVm,
    risc0_backend::L2_EXECUTOR_METHODS_FIBONACCI_ID,
};

use std::time::Instant;

#[cfg(feature = "risc0-poc")]
fn main() {
    println!("╔════════════════════════════════════════════════════════════╗");
    println!("║  Session 11: RISC0 Backend 性能对比测试                   ║");
    println!("╚════════════════════════════════════════════════════════════╝\n");

    // 测试 1: 证明生成性能对比
    println!("【测试 1】证明生成性能对比");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n");

    test_proof_generation();

    println!("\n");

    // 测试 2: 证明验证性能对比
    println!("【测试 2】证明验证性能对比");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n");

    test_proof_verification();

    println!("\n");

    // 测试 3: 证明大小对比
    println!("【测试 3】证明大小对比");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n");

    test_proof_size();

    println!("\n");

    // 测试 4: 批量处理性能
    println!("【测试 4】批量处理性能 (RISC0)");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n");

    test_batch_processing();

    println!("\n");

    // 测试 5: 安全性验证
    println!("【测试 5】安全性验证 (伪造检测)");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n");

    test_security();

    println!("\n╔════════════════════════════════════════════════════════════╗");
    println!("║  Session 11 测试完成!                                      ║");
    println!("╚════════════════════════════════════════════════════════════╝");
}

#[cfg(feature = "risc0-poc")]
fn test_proof_generation() {
    let risc0 = Risc0Backend::new();
    let trace = TraceZkVm::default();

    let test_cases = vec![
        ("fib(10)", 0u64, 1u64, 10u32),
        ("fib(20)", 0, 1, 20),
        ("fib(50)", 0, 1, 50),
        ("fib(100)", 0, 1, 100),
    ];

    println!("| 任务 | Trace 时间 | RISC0 时间 | 倍数 (RISC0/Trace) |");
    println!("|------|-----------|-----------|-------------------|");

    for (name, a0, a1, rounds) in test_cases {
        // Trace backend
        let trace_program = FibonacciProgram::new(rounds);
        let witness = vec![a0, a1];
        
        let trace_start = Instant::now();
        let _trace_proof = trace.prove(&trace_program, &witness).expect("trace prove");
        let trace_micros = trace_start.elapsed().as_micros();

        // RISC0 backend
        let risc0_start = Instant::now();
        let _risc0_proof = risc0.prove_fibonacci(a0, a1, rounds).expect("risc0 prove");
        let risc0_micros = risc0_start.elapsed().as_micros();

        let ratio = risc0_micros as f64 / trace_micros as f64;

        println!(
            "| {} | {}µs | {}µs | {:.2}x |",
            name, trace_micros, risc0_micros, ratio
        );
    }

    println!("\n💡 分析:");
    println!("   - RISC0 使用真正的 zk-SNARK (STARK),安全性高");
    println!("   - Trace 是模拟 backend,速度快但无安全性");
    println!("   - 预期 RISC0 慢 100-10000x (取决于任务大小)");
}

#[cfg(feature = "risc0-poc")]
fn test_proof_verification() {
    let risc0 = Risc0Backend::new();
    let trace = TraceZkVm::default();

    println!("生成测试证明 (fib 20)...");

    // 生成证明
    let trace_program = FibonacciProgram::new(20);
    let witness = vec![0u64, 1u64];
    let trace_proof = trace.prove(&trace_program, &witness).expect("trace prove");
    let risc0_proof = risc0.prove_fibonacci(0, 1, 20).expect("risc0 prove");

    // 验证性能测试
    let n = 100; // 验证次数

    // Trace verification
    let trace_start = Instant::now();
    for _ in 0..n {
        trace.verify(&trace_program, &trace_proof, &witness).expect("verify");
    }
    let trace_total = trace_start.elapsed().as_micros();
    let trace_avg = trace_total / n;

    // RISC0 verification
    let risc0_start = Instant::now();
    for _ in 0..n {
        risc0.verify_fibonacci(&risc0_proof).expect("verify");
    }
    let risc0_total = risc0_start.elapsed().as_micros();
    let risc0_avg = risc0_total / n;

    println!("\n| Backend | 单次验证 | 100 次总计 | TPS (1/验证时间) |");
    println!("|---------|---------|-----------|-----------------|");
    println!("| Trace   | {}µs | {}µs | {:.0} proofs/s |", 
        trace_avg, trace_total, 1_000_000.0 / trace_avg as f64);
    println!("| RISC0   | {}µs | {}µs | {:.0} proofs/s |",
        risc0_avg, risc0_total, 1_000_000.0 / risc0_avg as f64);

    let ratio = risc0_avg as f64 / trace_avg as f64;
    println!("\n倍数 (RISC0/Trace): {:.2}x", ratio);

    println!("\n💡 分析:");
    println!("   - RISC0 验证包含椭圆曲线、哈希等密码学运算");
    println!("   - Trace 验证仅检查签名,极快");
    println!("   - 验证速度决定了链上吞吐量上限");
}

#[cfg(feature = "risc0-poc")]
fn test_proof_size() {
    let risc0 = Risc0Backend::new();
    let trace = TraceZkVm::default();

    let test_cases = vec![10u32, 20, 50, 100];

    println!("| 任务 | Trace 大小 | RISC0 大小 | 倍数 |");
    println!("|------|-----------|-----------|------|");

    for rounds in test_cases {
        // Trace proof (估算大小,因为没有 Serialize)
        let trace_size_estimate = 100; // program_id + digest + outputs ≈ 100 bytes

        // RISC0 proof
        let risc0_proof = risc0.prove_fibonacci(0, 1, rounds).expect("prove");
        let risc0_size = bincode::serialize(&risc0_proof).expect("serialize").len();

        let ratio = risc0_size as f64 / trace_size_estimate as f64;

        println!(
            "| fib({}) | ~{} bytes | {} bytes | {:.2}x |",
            rounds, trace_size_estimate, risc0_size, ratio
        );
    }

    println!("\n💡 分析:");
    println!("   - RISC0 证明包含 STARK proof + journal");
    println!("   - Trace 仅包含执行摘要");
    println!("   - 证明大小影响链上存储成本和网络传输");
}

#[cfg(feature = "risc0-poc")]
fn test_batch_processing() {
    let risc0 = Risc0Backend::new();

    println!("测试场景: 生成 10 个 fib(20) 证明\n");

    // 顺序执行
    let sequential_start = Instant::now();
    for _ in 0..10 {
        risc0.prove_fibonacci(0, 1, 20).expect("prove");
    }
    let sequential_micros = sequential_start.elapsed().as_micros();

    // 并行执行 (使用 rayon)
    use rayon::prelude::*;
    let parallel_start = Instant::now();
    (0..10).into_par_iter().for_each(|_| {
        risc0.prove_fibonacci(0, 1, 20).expect("prove");
    });
    let parallel_micros = parallel_start.elapsed().as_micros();

    let speedup = sequential_micros as f64 / parallel_micros as f64;

    println!("| 策略 | 耗时 | 吞吐量 |");
    println!("|------|------|--------|");
    println!("| 顺序 | {}µs | {:.0} proofs/s |", 
        sequential_micros, 10_000_000.0 / sequential_micros as f64);
    println!("| 并行 | {}µs | {:.0} proofs/s |",
        parallel_micros, 10_000_000.0 / parallel_micros as f64);

    println!("\n加速比: {:.2}x", speedup);

    println!("\n💡 分析:");
    println!("   - RISC0 证明生成 CPU 密集,适合并行");
    println!("   - 理想加速比 ≈ CPU 核心数");
    println!("   - 实际加速比受内存带宽、锁竞争影响");
}

#[cfg(feature = "risc0-poc")]
fn test_security() {
    let risc0 = Risc0Backend::new();

    println!("测试 1: 正确证明验证通过 ✓\n");

    let proof = risc0.prove_fibonacci(0, 1, 10).expect("prove");
    match risc0.verify_fibonacci(&proof) {
        Ok(_) => println!("✅ 正确证明验证通过"),
        Err(e) => println!("❌ 验证失败: {}", e),
    }

    println!("\n测试 2: 篡改输出检测 (通过 trait 接口)\n");

    // 生成正确证明 (fib(10) = 89)
    let mut private_inputs = Vec::new();
    private_inputs.extend_from_slice(&0u64.to_le_bytes());
    private_inputs.extend_from_slice(&1u64.to_le_bytes());
    private_inputs.extend_from_slice(&10u32.to_le_bytes());

    let (proof, outputs) = risc0
        .prove(&L2_EXECUTOR_METHODS_FIBONACCI_ID, &private_inputs, &vec![])
        .expect("prove");

    assert_eq!(outputs, vec![89]);

    // 尝试用错误输出验证
    let fake_outputs = vec![100u64]; // 伪造输出
    let result = risc0.verify(
        &L2_EXECUTOR_METHODS_FIBONACCI_ID,
        &proof,
        &vec![],
        &fake_outputs,
    );

    match result {
        Ok(false) => println!("✅ 成功检测到输出篡改"),
        Ok(true) => println!("❌ 未检测到篡改 (安全漏洞!)"),
        Err(e) => println!("❌ 验证错误: {}", e),
    }

    println!("\n测试 3: 错误 program ID 检测\n");

    let fake_id = [0u32; 8]; // 伪造 program ID
    let result = risc0.verify(
        &fake_id,
        &proof,
        &vec![],
        &outputs,
    );

    match result {
        Ok(false) => println!("✅ 成功检测到 program ID 不匹配"),
        Ok(true) => println!("❌ 未检测到不匹配 (安全漏洞!)"),
        Err(e) => println!("❌ 验证错误: {}", e),
    }

    println!("\n💡 安全性总结:");
    println!("   - RISC0 提供密码学级别的安全性");
    println!("   - 无法伪造证明或篡改输出");
    println!("   - 适合生产环境的信任最小化应用");
}

#[cfg(not(feature = "risc0-poc"))]
fn main() {
    eprintln!("❌ 此示例需要 risc0-poc feature");
    eprintln!("请在 WSL 中运行:");
    eprintln!("  wsl bash -c \"cd /mnt/d/WEB3_AI开发/虚拟机开发 && cargo run --release --features risc0-poc --example risc0_performance_comparison\"");
    std::process::exit(1);
}
