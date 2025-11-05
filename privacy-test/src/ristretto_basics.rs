// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2025 XujueKing <leadbrand@me.com>

//! # RistrettoPoint 基础操作
//! 
//! RistrettoPoint 是 Ristretto255 群的元素，提供:
//! - 无法区分的点编码 (indistinguishable encoding)
//! - 素数阶群 (prime-order group, 无 cofactor 问题)
//! - 与 Ed25519 兼容的性能

use curve25519_dalek::ristretto::RistrettoPoint;
use curve25519_dalek::scalar::Scalar;
use curve25519_dalek::constants::RISTRETTO_BASEPOINT_POINT;
use curve25519_dalek::traits::{Identity, MultiscalarMul};
use rand::rngs::OsRng;

/// 示例 1: 创建 RistrettoPoint
pub fn example_create_points() {
    println!("\n=== 示例 1: 创建 RistrettoPoint ===");
    
    // 方法 1: 使用基点 G
    let base_point = RISTRETTO_BASEPOINT_POINT;
    println!("基点 G: {:?}", base_point.compress());
    
    // 方法 2: 随机生成点 (实际是 random_scalar * G)
    let random_scalar = Scalar::random(&mut OsRng);
    let random_point = random_scalar * RISTRETTO_BASEPOINT_POINT;
    println!("随机点: {:?}", random_point.compress());
    
    // 方法 3: 零元素 (单位元)
    let identity = RistrettoPoint::identity();
    println!("单位元 (零点): {:?}", identity.compress());
}

/// 示例 2: 标量乘法 (Scalar Multiplication)
/// 
/// 这是密码学的核心操作: P = k*G
/// - 已知 k 和 G, 容易计算 P
/// - 已知 P 和 G, 难以计算 k (离散对数问题, ECDLP)
pub fn example_scalar_multiplication() {
    println!("\n=== 示例 2: 标量乘法 ===");
    
    // 生成私钥 (标量)
    let secret_key = Scalar::random(&mut OsRng);
    println!("私钥 (标量): {:?}", secret_key);
    
    // 计算公钥 P = k*G
    let public_key = secret_key * RISTRETTO_BASEPOINT_POINT;
    println!("公钥 (点): {:?}", public_key.compress());
    
    // 验证单位元性质: 0*G = Identity
    let zero = Scalar::ZERO;
    let zero_point = zero * RISTRETTO_BASEPOINT_POINT;
    assert_eq!(zero_point, RistrettoPoint::identity());
    println!("✅ 验证: 0*G = Identity");
    
    // 验证逆元性质: k*G + (-k)*G = Identity
    let neg_secret = -secret_key;
    let neg_public = neg_secret * RISTRETTO_BASEPOINT_POINT;
    let sum = public_key + neg_public;
    assert_eq!(sum, RistrettoPoint::identity());
    println!("✅ 验证: k*G + (-k)*G = Identity");
}

/// 示例 3: 点加法 (Point Addition)
/// 
/// Ristretto 群支持点加法: P₁ + P₂
/// 对应的标量关系: (k₁ + k₂)*G = k₁*G + k₂*G
pub fn example_point_addition() {
    println!("\n=== 示例 3: 点加法 ===");
    
    let k1 = Scalar::random(&mut OsRng);
    let k2 = Scalar::random(&mut OsRng);
    
    let P1 = k1 * RISTRETTO_BASEPOINT_POINT;
    let P2 = k2 * RISTRETTO_BASEPOINT_POINT;
    
    // 方法 1: 先加标量, 再乘基点
    let k_sum = k1 + k2;
    let P_sum_1 = k_sum * RISTRETTO_BASEPOINT_POINT;
    
    // 方法 2: 先乘基点, 再加点
    let P_sum_2 = P1 + P2;
    
    // 验证两种方法等价
    assert_eq!(P_sum_1, P_sum_2);
    println!("✅ 验证: (k₁ + k₂)*G = k₁*G + k₂*G");
    println!("P₁: {:?}", P1.compress());
    println!("P₂: {:?}", P2.compress());
    println!("P₁ + P₂: {:?}", P_sum_2.compress());
}

/// 示例 4: 多标量乘法 (Multiscalar Multiplication)
/// 
/// 高效计算: a₁*P₁ + a₂*P₂ + ... + aₙ*Pₙ
/// curve25519-dalek 使用 Straus 算法优化
pub fn example_multiscalar_multiplication() {
    println!("\n=== 示例 4: 多标量乘法 ===");
    
    // 生成多个标量和点
    let scalars: Vec<Scalar> = (0..5).map(|_| Scalar::random(&mut OsRng)).collect();
    let points: Vec<RistrettoPoint> = (0..5)
        .map(|_| Scalar::random(&mut OsRng) * RISTRETTO_BASEPOINT_POINT)
        .collect();
    
    // 方法 1: 朴素计算 (逐个乘加)
    use std::time::Instant;
    let start = Instant::now();
    let mut result_naive = RistrettoPoint::identity();
    for (scalar, point) in scalars.iter().zip(points.iter()) {
        result_naive += scalar * point;
    }
    let naive_time = start.elapsed();
    
    // 方法 2: 多标量乘法 (Straus 算法)
    let start = Instant::now();
    let result_optimized = RistrettoPoint::multiscalar_mul(&scalars, &points);
    let optimized_time = start.elapsed();
    
    // 验证结果相同
    assert_eq!(result_naive, result_optimized);
    println!("✅ 验证: 朴素计算 = 多标量乘法");
    println!("朴素计算耗时: {:?}", naive_time);
    println!("Straus 算法耗时: {:?}", optimized_time);
    println!("提速: {:.2}x", naive_time.as_nanos() as f64 / optimized_time.as_nanos() as f64);
}

/// 示例 5: 压缩与解压缩
/// 
/// RistrettoPoint 可以压缩为 32 字节传输/存储
pub fn example_compression() {
    println!("\n=== 示例 5: 压缩与解压缩 ===");
    
    let point = Scalar::random(&mut OsRng) * RISTRETTO_BASEPOINT_POINT;
    
    // 压缩为 32 字节
    let compressed = point.compress();
    println!("压缩点 (32 bytes): {:?}", compressed);
    println!("十六进制: {}", hex::encode(compressed.as_bytes()));
    
    // 解压缩
    let decompressed = compressed.decompress().expect("解压缩失败");
    
    // 验证
    assert_eq!(point, decompressed);
    println!("✅ 验证: 压缩/解压缩保持点不变");
}

/// 运行所有示例
pub fn run_all_examples() {
    example_create_points();
    example_scalar_multiplication();
    example_point_addition();
    example_multiscalar_multiplication();
    example_compression();
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_scalar_multiplication() {
        let k = Scalar::from(42u64);
        let P = k * RISTRETTO_BASEPOINT_POINT;
        
        // k*G != Identity (除非 k=0)
        assert_ne!(P, RistrettoPoint::identity());
        
        // 2*(k*G) = (2k)*G
        let P2 = Scalar::from(2u64) * P;
        let k2 = Scalar::from(84u64);
        let P2_alt = k2 * RISTRETTO_BASEPOINT_POINT;
        assert_eq!(P2, P2_alt);
    }
    
    #[test]
    fn test_point_addition_associativity() {
        let P1 = Scalar::random(&mut OsRng) * RISTRETTO_BASEPOINT_POINT;
        let P2 = Scalar::random(&mut OsRng) * RISTRETTO_BASEPOINT_POINT;
        let P3 = Scalar::random(&mut OsRng) * RISTRETTO_BASEPOINT_POINT;
        
        // 验证结合律: (P1 + P2) + P3 = P1 + (P2 + P3)
        let left = (P1 + P2) + P3;
        let right = P1 + (P2 + P3);
        assert_eq!(left, right);
    }
    
    #[test]
    fn test_compression_roundtrip() {
        for _ in 0..10 {
            let point = Scalar::random(&mut OsRng) * RISTRETTO_BASEPOINT_POINT;
            let compressed = point.compress();
            let decompressed = compressed.decompress().unwrap();
            assert_eq!(point, decompressed);
        }
    }
}
