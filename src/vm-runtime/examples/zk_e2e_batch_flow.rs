// SPDX-License-Identifier: GPL-3.0-or-later
//! Phase 2.2 End-to-End RingCT Batch Verification Flow
//! 端到端 ZK 批量验证工作流示例（使用 MultiplyCircuit 演示）
//!
//! 完整演示:
//!   1. 证明生成（使用 zk-groth16-test MultiplyCircuit）
//!   2. SuperVM 批量验证（通过 verify_zk_proof API）
//!   3. 批量验证统计输出
//!
//! 运行命令:
//!   cargo run --example zk_e2e_batch_flow --release --features groth16-verifier
//!
//! 环境变量:
//!   ZK_BATCH_ENABLE=1              # 启用批量验证
//!   ZK_BATCH_SIZE=32               # 批量大小
//!   ZK_BATCH_FLUSH_INTERVAL_MS=50  # 刷新间隔（毫秒）

#![cfg(feature = "groth16-verifier")]

use std::sync::Arc;
use std::time::Instant;

use ark_bls12_381::{Bls12_381, Fr};
use ark_groth16::{Groth16, Proof, ProvingKey};
use ark_serialize::CanonicalSerialize;
use ark_snark::SNARK;
use ark_std::UniformRand;

use vm_runtime::{OwnershipManager, MvccScheduler, SuperVM};
use zk_groth16_test::MultiplyCircuit;

// 环境变量解析辅助函数
fn env_bool(key: &str, default: bool) -> bool {
    std::env::var(key)
        .ok()
        .map(|v| matches!(v.as_str(), "1" | "true" | "TRUE"))
        .unwrap_or(default)
}

fn env_usize(key: &str, default: usize) -> usize {
    std::env::var(key).ok().and_then(|v| v.parse().ok()).unwrap_or(default)
}

fn env_u64(key: &str, default: u64) -> u64 {
    std::env::var(key).ok().and_then(|v| v.parse().ok()).unwrap_or(default)
}

// 生成单个 RingCT 证明（简化版：金额隐藏 + Ring Signature）
// 生成单个 Multiply 证明（a * b = c）
fn generate_multiply_proof(pk: &ProvingKey<Bls12_381>) -> (Proof<Bls12_381>, Vec<Fr>) {
    let mut rng = rand::rngs::OsRng;

    // 1. 生成随机 a, b 并计算 c = a * b
    let a = Fr::rand(&mut rng);
    let b = Fr::rand(&mut rng);
    let c = a * b;

    // 2. 构造电路
    let circuit = MultiplyCircuit { a: Some(a), b: Some(b) };

    // 3. 生成证明
    let proof = Groth16::<Bls12_381>::prove(pk, circuit, &mut rng).expect("Proof generation failed");

    // 4. 公开输入 c
    let public_inputs = vec![c];

    (proof, public_inputs)
}

fn main() {
    println!("=== ZK Batch Verification E2E Flow ===\n");

    // Step 0: 环境配置
    let batch_enable = env_bool("ZK_BATCH_ENABLE", true);
    let batch_size = env_usize("ZK_BATCH_SIZE", 32);
    let flush_interval_ms = env_u64("ZK_BATCH_FLUSH_INTERVAL_MS", 50);
    let total_proofs = 100; // 生成 100 个 RingCT 证明

    println!("[Config]");
    println!("  Batch enabled: {}", batch_enable);
    println!("  Batch size: {}", batch_size);
    println!("  Flush interval: {} ms", flush_interval_ms);
    println!("  Total proofs: {}\n", total_proofs);

    // Step 1: Setup - 生成 RingCT proving key & verifying key
    println!("[Step 1] Generating circuit parameters...");
    let mut rng = rand::rngs::OsRng;
    
    let dummy_circuit = MultiplyCircuit { a: None, b: None };

    let params = Groth16::<Bls12_381>::generate_random_parameters_with_reduction(dummy_circuit, &mut rng)
        .expect("Setup failed");
    println!("  ✓ Parameters generated\n");

    // Step 2: 生成证明
    println!("[Step 2] Generating {} proofs...", total_proofs);
    let start_gen = Instant::now();
    let mut proof_data: Vec<(Vec<u8>, Vec<u8>)> = Vec::with_capacity(total_proofs);

    for i in 0..total_proofs {
    let (proof, public_inputs) = generate_multiply_proof(&params);

        // 序列化为字节
        let mut proof_bytes = Vec::new();
        proof.serialize_compressed(&mut proof_bytes).expect("proof serialization failed");

        let mut public_input_bytes = Vec::new();
        for inp in &public_inputs {
            inp.serialize_compressed(&mut public_input_bytes).expect("input serialization failed");
        }

        proof_data.push((proof_bytes, public_input_bytes));

        if (i + 1) % 20 == 0 {
            println!("  Generated {}/{} proofs", i + 1, total_proofs);
        }
    }
    let gen_duration = start_gen.elapsed();
    println!("  ✓ All proofs generated in {:.2}s\n", gen_duration.as_secs_f64());

    // Step 3: SuperVM 批量验证
    println!("[Step 3] Setting up SuperVM with batch verification...");
    let ownership = Box::leak(Box::new(OwnershipManager::new()));
    let scheduler = Box::leak(Box::new(MvccScheduler::new()));

    // 使用 from_env_with_deps 创建 SuperVM（会读取 ZK_BATCH_* 环境变量）
    std::env::set_var("ZK_BATCH_ENABLE", if batch_enable { "1" } else { "0" });
    std::env::set_var("ZK_BATCH_SIZE", batch_size.to_string());
    std::env::set_var("ZK_BATCH_FLUSH_INTERVAL_MS", flush_interval_ms.to_string());

    let vm = Box::leak(Box::new(
        SuperVM::new(ownership)
            .with_scheduler(scheduler)
            .from_env()
    ));
    println!("  ✓ SuperVM initialized\n");

    // Step 4: 验证证明
    println!("[Step 4] Verifying {} proofs via SuperVM batch buffer...", total_proofs);
    let start_verify = Instant::now();
    let mut failed_count = 0;

    for (proof_bytes, public_input_bytes) in &proof_data {
        if !vm.verify_zk_proof(Some(proof_bytes), Some(public_input_bytes)) {
            failed_count += 1;
        }
    }

    // 手动刷新剩余批次
    let (flushed_total, flushed_failed) = vm.flush_zk_batch();
    failed_count += flushed_failed as usize;

    let verify_duration = start_verify.elapsed();
    let verify_ms = verify_duration.as_millis();
    let tps = if verify_ms > 0 {
        (total_proofs as u64 * 1000) / verify_ms as u64
    } else {
        0
    };

    println!("  ✓ Verification completed in {:.2}s", verify_duration.as_secs_f64());
    println!("  Total: {}, Failed: {}, Success rate: {:.2}%", 
             total_proofs, failed_count, 
             100.0 * (total_proofs - failed_count) as f64 / total_proofs as f64);
    println!("  Throughput: {} TPS\n", tps);

    // Step 5: 导出 Prometheus 指标
    println!("\n=== ZK E2E Batch Flow Completed ===");
    println!("Next steps:");
    println!("  1. Import grafana-dashboard.json into Grafana");
    println!("  2. View 'ZK Batch Verification' panels for real-time metrics");
    println!("  3. Adjust ZK_BATCH_SIZE/FLUSH_INTERVAL_MS for optimal performance");
}
