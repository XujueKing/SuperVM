//! zkVM Backend Performance Comparison
//!
//! This example demonstrates the pluggable zkVM backend architecture
//! and compares the development-mode Trace backend with production backends.
//!
//! Run with:
//! ```bash
//! # Trace-only (works on Windows)
//! cargo run --example backend_comparison
//!
//! # With RISC0 support (requires Linux/WSL)
//! cargo run --example backend_comparison --features risc0-poc
//!
//! # With SP1 support (requires Linux/WSL)  
//! cargo run --example backend_comparison --features sp1-poc
//! ```

use l2_executor::*;
use std::time::Instant;

fn main() {
    println!("🚀 SuperVM L2 Executor - zkVM Backend Comparison\n");
    println!("================================================\n");

    // Test program: Fibonacci(10)
    let program = FibonacciProgram::new(10);
    // TraceZkVm 期望 witness 为 &[u64]
    let witness: Vec<u64> = vec![0, 1];
    let iterations = 10;

    println!("📊 Test Configuration:");
    println!("  Program: Fibonacci(10)");
    println!("  Iterations: {}", iterations);
    println!("  Expected output: 55\n");

    // 1. Trace Backend (always available)
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("Test 1: Trace Backend (Development Mode)");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n");
    
    benchmark_trace_backend(&program, &witness, iterations);

    // 2. RISC0 Backend (if feature enabled)
    #[cfg(feature = "risc0-poc")]
    {
        println!("\n━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
        println!("Test 2: RISC0 Backend (STARK-based)");
        println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n");
        println!("⚠️  RISC0 backend requires actual guest ELF compilation");
        println!("   Run in Linux/WSL environment for full functionality\n");
    }

    // 3. SP1 Backend (if feature enabled)
    #[cfg(feature = "sp1-poc")]
    {
        println!("\n━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
        println!("Test 3: SP1 Backend (PLONKish)");
        println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n");
        println!("⚠️  SP1 backend requires actual guest program compilation");
        println!("   Run in Linux/WSL environment for full functionality\n");
    }

    // Print comparison summary
    print_comparison_summary();

    println!("✅ Backend comparison completed successfully!");
}

fn benchmark_trace_backend(program: &FibonacciProgram, witness: &[u64], iterations: usize) {
    let vm = TraceZkVm::default();

    // Warm-up run
    let _proof = vm.prove(program, &witness).expect("prove");

    // Measure proving time
    let prove_start = Instant::now();
    // Trace 模式下无需统计证明大小，聚焦时间开销

    for _ in 0..iterations {
        let _proof = vm.prove(program, &witness).expect("prove");
    }
    let prove_time_ms = prove_start.elapsed().as_millis() / iterations as u128;

    // Generate one proof for verification test
    let proof = vm.prove(program, &witness).expect("prove");

    // Measure verification time
    let verify_start = Instant::now();
    for _ in 0..iterations {
        assert!(vm.verify(program, &proof, &witness).expect("verify"));
    }
    let verify_time_ms = verify_start.elapsed().as_millis() / iterations as u128;

    println!("✅ Trace Backend Results:");
    println!("   • Backend: Trace (Development Mode)");
    println!("   • Prove time: {} ms", prove_time_ms);
    println!("   • Verify time: {} ms", verify_time_ms);
    // Trace 模式下证明为内部结构，不输出大小
    println!("   • Output: {:?}", proof.public_outputs);
    println!("   • Status: ✅ All tests passed");
    println!("\n💡 Analysis:");
    println!("   • Instant proving (<1ms typical)");
    println!("   • Perfect for development and testing");
    println!("   • No cryptographic security (traces only)");
    println!("   • Recommended: Local development & unit tests");
}

fn print_comparison_summary() {
    println!("\n━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("📊 Backend Comparison Summary");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n");

    println!("Available Backends:");
    println!("  ✅ Trace     - Development mode (always available)");
    
    #[cfg(feature = "risc0-poc")]
    println!("  ✅ RISC0    - STARK-based (enabled)");
    #[cfg(not(feature = "risc0-poc"))]
    println!("  ⚪ RISC0    - Enable with --features risc0-poc");

    #[cfg(feature = "sp1-poc")]
    println!("  ✅ SP1      - PLONKish (enabled)");
    #[cfg(not(feature = "sp1-poc"))]
    println!("  ⚪ SP1      - Enable with --features sp1-poc");

    println!("\n━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("💡 Backend Selection Guide");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n");

    println!("🔧 Development & Testing:");
    println!("   Use: Trace backend");
    println!("   Why: Instant proving, no setup required");
    println!("   Trade-off: No cryptographic security\n");

    println!("⚡ Production (Performance Priority):");
    println!("   Use: SP1 backend");
    println!("   Why: Faster proving, smaller proofs");
    println!("   Trade-off: PLONKish security assumptions\n");

    println!("🔒 Production (Maximum Security):");
    println!("   Use: RISC0 backend");
    println!("   Why: STARK-based, transparent setup");
    println!("   Trade-off: Slower proving, larger proofs\n");

    println!("🔀 Hybrid Approach:");
    println!("   Use: Aggregation to combine multiple proofs");
    println!("   Why: Best of both worlds - prove fast, settle securely");
    println!("   How: Fast backend → Aggregator → RISC0/SP1 final proof\n");

    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("📚 Next Steps");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n");

    println!("1. Enable RISC0 backend:");
    println!("   cargo run --example backend_comparison --features risc0-poc\n");

    println!("2. Enable SP1 backend:");
    println!("   cargo run --example backend_comparison --features sp1-poc\n");

    println!("3. Run in Linux/WSL for full zkVM support:");
    println!("   wsl");
    println!("   cd /mnt/d/WEB3_AI开发/虚拟机开发");
    println!("   cargo run --example backend_comparison --features risc0-poc,sp1-poc\n");

    println!("4. Explore aggregation:");
    println!("   cargo run --example aggregation_strategy_demo\n");
}

