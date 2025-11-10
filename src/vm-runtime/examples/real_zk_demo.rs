// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2025 XujueKing <leadbrand@me.com>

//! Real ZK Verifier Demo
//!
//! 演示 SuperVM 集成真实 Groth16 验证器

#[cfg(feature = "groth16-verifier")]
fn main() -> Result<(), Box<dyn std::error::Error>> {
    use vm_runtime::{
        zk_verifier::{generate_test_proof, Groth16Verifier, ZkVerifier},
        OwnershipManager, SuperVM,
    };

    println!("=== Real ZK Verifier Demo ===\n");

    // 1. 创建 ZK 验证器（使用测试用 Trusted Setup）
    println!("🔑 Step 1: Initialize ZK Verifier");
    let verifier = Groth16Verifier::new_for_testing()?;
    println!("   Verifier Type: {}\n", verifier.verifier_type());

    // 2. 生成测试证明（a=7, b=11, c=77）
    println!("📝 Step 2: Generate Test Proof");
    let (proof_bytes, public_input_bytes) = generate_test_proof()?;
    println!("   Proof Size: {} bytes", proof_bytes.len());
    println!("   Public Input Size: {} bytes\n", public_input_bytes.len());

    // 3. 直接验证证明
    println!("✅ Step 3: Direct Verification");
    let valid = verifier.verify(&proof_bytes, &public_input_bytes)?;
    println!("   Verification Result: {}\n", if valid { "VALID ✓" } else { "INVALID ✗" });

    // 4. 创建 SuperVM 并注入验证器
    println!("🚀 Step 4: Integrate with SuperVM");
    let ownership = OwnershipManager::new();
    let mut supervm = SuperVM::new(&ownership);
    supervm = supervm.with_verifier(&verifier);
    println!("   SuperVM configured with ZK verifier\n");

    // 5. 测试带真实验证的隐私事务
    println!("🔒 Step 5: Privacy Transaction with Real ZK");
    
    // 场景 A：提供有效 proof
    println!("   Scenario A: Valid Proof");
    let result_valid = supervm.verify_zk_proof(Some(&proof_bytes), Some(&public_input_bytes));
    println!("      Result: {}\n", if result_valid { "ACCEPTED ✓" } else { "REJECTED ✗" });

    // 场景 B：提供无效 proof（错误的公开输入）
    println!("   Scenario B: Invalid Public Input");
    let wrong_input = vec![0u8; public_input_bytes.len()]; // 全零输入
    let result_invalid = supervm.verify_zk_proof(Some(&proof_bytes), Some(&wrong_input));
    println!("      Result: {}\n", if result_invalid { "ACCEPTED ✓" } else { "REJECTED ✗" });

    // 场景 C：Fallback（未提供 proof，使用占位逻辑）
    println!("   Scenario C: Fallback (No Proof Provided)");
    let result_fallback = supervm.verify_zk_proof(None, None);
    println!("      Result: {} (占位逻辑)\n", if result_fallback { "ACCEPTED ✓" } else { "REJECTED ✗" });

    // 6. 性能测试：验证延迟
    println!("⚡ Step 6: Performance Benchmark");
    let iterations = 1000;
    let start = std::time::Instant::now();
    
    for _ in 0..iterations {
        let _ = verifier.verify(&proof_bytes, &public_input_bytes)?;
    }
    
    let elapsed = start.elapsed();
    let avg_latency_us = elapsed.as_micros() / iterations;
    let tps = (iterations as f64 / elapsed.as_secs_f64()) as u64;
    
    println!("   Iterations: {}", iterations);
    println!("   Total Time: {:.2?}", elapsed);
    println!("   Average Latency: {} µs", avg_latency_us);
    println!("   Estimated TPS: {} txns/sec\n", tps);

    // 7. 内存占用
    println!("📊 Step 7: Memory Footprint");
    println!("   Proof Size: {} bytes ({:.2} KB)", 
        proof_bytes.len(), 
        proof_bytes.len() as f64 / 1024.0
    );
    println!("   Public Input Size: {} bytes\n", public_input_bytes.len());

    println!("✨ Demo completed successfully!");
    Ok(())
}

#[cfg(not(feature = "groth16-verifier"))]
fn main() {
    eprintln!("❌ This demo requires the 'groth16-verifier' feature.");
    eprintln!("   Run with: cargo run --example real_zk_demo --features groth16-verifier");
    std::process::exit(1);
}
