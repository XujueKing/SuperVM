//! L2 Executor 端到端集成示例
//!
//! 展示完整的 zkVM 执行流程:
//! 1. Runtime 初始化 (自动选择后端)
//! 2. 程序执行 + 证明生成
//! 3. 证明验证
//! 4. 证明聚合

use anyhow::Result;
use l2_executor::{
    AggregatedProof, BackendType, FibonacciProgram, L2Runtime, MerkleAggregator, Proof,
    Sha256Program, TraceZkVm,
};

/// 示例 1: 单个程序执行流程
fn example_single_program_execution() -> Result<()> {
    println!("\n=== Example 1: Single Program Execution ===");
    
    // 1. 初始化运行时 (自动选择后端)
    let runtime = L2Runtime::auto_select()?;
    println!("Backend: {}", runtime.backend_type());
    
    // 2. 创建 VM 实例
    let vm = runtime.create_trace_vm();
    
    // 3. 定义程序和输入
    let program = FibonacciProgram::new(10);
    let witness = vec![0, 1]; // 初始值
    
    // 4. 生成证明
    println!("Generating proof for Fibonacci(10)...");
    let proof = vm.prove(&program, &witness)?;
    println!("  Proof generated: {} steps", proof.steps);
    println!("  Public outputs: {:?}", proof.public_outputs);
    
    // 5. 验证证明
    println!("Verifying proof...");
    let verified = vm.verify(&program, &proof, &witness)?;
    println!("  Verification result: {}", verified);
    
    assert!(verified);
    assert_eq!(proof.public_outputs, vec![55]); // fib(10) = 55
    
    Ok(())
}

/// 示例 2: 多程序批处理
fn example_batch_processing() -> Result<()> {
    println!("\n=== Example 2: Batch Processing ===");
    
    let runtime = L2Runtime::auto_select()?;
    let vm = runtime.create_trace_vm();
    
    // 定义多个程序
    let programs = vec![
        ("fib_5", FibonacciProgram::new(5)),
        ("fib_10", FibonacciProgram::new(10)),
        ("fib_15", FibonacciProgram::new(15)),
    ];
    
    let mut proofs = Vec::new();
    
    // 批量生成证明
    println!("Processing {} programs...", programs.len());
    for (name, program) in &programs {
        let proof = vm.prove(program, &[0, 1])?;
        println!("  {} -> output: {:?}, steps: {}", name, proof.public_outputs, proof.steps);
        proofs.push(proof);
    }
    
    // 验证所有证明
    println!("Verifying all proofs...");
    for (i, (name, program)) in programs.iter().enumerate() {
        let verified = vm.verify(program, &proofs[i], &[0, 1])?;
        assert!(verified, "{} verification failed", name);
    }
    println!("  All proofs verified ✓");
    
    Ok(())
}

/// 示例 3: 证明聚合
fn example_proof_aggregation() -> Result<()> {
    println!("\n=== Example 3: Proof Aggregation ===");
    
    let runtime = L2Runtime::auto_select()?;
    let vm = runtime.create_trace_vm();
    
    // 生成多个证明
    let proofs = vec![
        vm.prove(&FibonacciProgram::new(5), &[0, 1])?,
        vm.prove(&FibonacciProgram::new(10), &[0, 1])?,
        vm.prove(&Sha256Program::default(), &[0x6162636465666768])?,
    ];
    
    println!("Generated {} proofs", proofs.len());
    
    // 聚合证明
    println!("Aggregating proofs...");
    let mut aggregator = MerkleAggregator::new();
    for proof in &proofs {
        aggregator.add_proof(proof);
    }
    
    let aggregated = aggregator.finalize();
    println!("  Aggregated proof:");
    println!("    Proof count: {}", aggregated.proof_count);
    println!("    Merkle root: {}", hex::encode(&aggregated.root));
    
    assert_eq!(aggregated.proof_count, 3);
    assert_ne!(aggregated.root, [0u8; 32]);
    
    Ok(())
}

/// 示例 4: 跨平台兼容性检查
fn example_cross_platform_compatibility() -> Result<()> {
    println!("\n=== Example 4: Cross-Platform Compatibility ===");
    
    // 检查可用后端
    println!("Available backends:");
    for backend in L2Runtime::available_backends() {
        println!("  - {} (available: {})", backend, L2Runtime::is_backend_available(backend));
    }
    
    // 尝试创建所有可能的后端
    println!("\nTesting backend initialization:");
    
    // Trace (总是可用)
    match L2Runtime::new(BackendType::Trace) {
        Ok(runtime) => println!("  ✓ Trace backend: {}", runtime.backend_type()),
        Err(e) => println!("  ✗ Trace backend failed: {}", e),
    }
    
    // RISC0 (平台相关)
    match L2Runtime::new(BackendType::Risc0) {
        Ok(runtime) => println!("  ✓ RISC0 backend: {}", runtime.backend_type()),
        Err(e) => println!("  ✗ RISC0 backend: {}", e),
    }
    
    // Halo2 (未实现)
    match L2Runtime::new(BackendType::Halo2) {
        Ok(runtime) => println!("  ✓ Halo2 backend: {}", runtime.backend_type()),
        Err(e) => println!("  ✗ Halo2 backend: {}", e),
    }
    
    Ok(())
}

/// 示例 5: 性能对比 (简化版)
fn example_performance_comparison() -> Result<()> {
    println!("\n=== Example 5: Performance Comparison ===");
    
    let runtime = L2Runtime::auto_select()?;
    let vm = runtime.create_trace_vm();
    
    // 测试不同复杂度
    let complexities = vec![5, 10, 20, 50];
    
    println!("Testing Fibonacci with different complexities:");
    for n in complexities {
        let program = FibonacciProgram::new(n);
        
        let start = std::time::Instant::now();
        let proof = vm.prove(&program, &[0, 1])?;
        let elapsed = start.elapsed();
        
        println!("  fib({:2}) -> {:6} (steps: {:4}, time: {:?})", 
                 n, 
                 proof.public_outputs[0], 
                 proof.steps,
                 elapsed);
    }
    
    Ok(())
}

/// 示例 6: 错误处理
fn example_error_handling() -> Result<()> {
    println!("\n=== Example 6: Error Handling ===");
    
    let runtime = L2Runtime::auto_select()?;
    let vm = runtime.create_trace_vm();
    
    // 正常情况
    let program = FibonacciProgram::new(10);
    let proof = vm.prove(&program, &[0, 1])?;
    println!("✓ Normal execution successful");
    
    // 验证失败 (使用错误的 witness)
    let wrong_witness = vec![1, 2];
    match vm.verify(&program, &proof, &wrong_witness) {
        Ok(verified) => {
            if verified {
                println!("  Unexpected: verification passed with wrong witness");
            } else {
                println!("✓ Verification correctly failed with wrong witness");
            }
        }
        Err(e) => println!("  Error during verification: {}", e),
    }
    
    Ok(())
}

/// 主函数: 运行所有示例
fn main() -> Result<()> {
    println!("╔═══════════════════════════════════════════╗");
    println!("║  L2 Executor - End-to-End Integration    ║");
    println!("╚═══════════════════════════════════════════╝");
    
    example_single_program_execution()?;
    example_batch_processing()?;
    example_proof_aggregation()?;
    example_cross_platform_compatibility()?;
    example_performance_comparison()?;
    example_error_handling()?;
    
    println!("\n╔═══════════════════════════════════════════╗");
    println!("║  All examples completed successfully! ✓  ║");
    println!("╚═══════════════════════════════════════════╝");
    
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_single_program_execution() {
        example_single_program_execution().unwrap();
    }

    #[test]
    fn test_batch_processing() {
        example_batch_processing().unwrap();
    }

    #[test]
    fn test_proof_aggregation() {
        example_proof_aggregation().unwrap();
    }

    #[test]
    fn test_cross_platform_compatibility() {
        example_cross_platform_compatibility().unwrap();
    }

    #[test]
    fn test_performance_comparison() {
        example_performance_comparison().unwrap();
    }

    #[test]
    fn test_error_handling() {
        example_error_handling().unwrap();
    }
}
