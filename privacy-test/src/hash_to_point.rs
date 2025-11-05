// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2025 XujueKing <leadbrand@me.com>

//! # Hash-to-Point 实现
//! 
//! 用于 Monero Key Image 生成: I = x * Hp(P)
//! 
//! ## 核心思想
//! 将任意数据 (如公钥) 哈希到椭圆曲线点
//! 
//! ## Monero 使用的方法
//! 1. 哈希数据到 32 字节: h = Hash(data)
//! 2. 使用 Elligator 或 Ristretto 将 h 映射到曲线点
//! 3. 乘以 cofactor 8 确保在素数阶子群

use curve25519_dalek::ristretto::RistrettoPoint;
use curve25519_dalek::scalar::Scalar;
use curve25519_dalek::constants::RISTRETTO_BASEPOINT_POINT;
use sha2::{Sha512, Digest};
use rand::rngs::OsRng;

/// Hash-to-Point 实现
/// 
/// 使用 Ristretto 的 from_uniform_bytes 方法
/// 输入: 任意长度的数据
/// 输出: RistrettoPoint
pub fn hash_to_point(data: &[u8]) -> RistrettoPoint {
    // 步骤 1: 哈希数据到 64 字节 (Ristretto 需要 64 字节均匀输入)
    let mut hasher = Sha512::new();
    hasher.update(data);
    let hash = hasher.finalize();
    
    // 步骤 2: 将 64 字节哈希映射到 Ristretto 点
    // from_uniform_bytes 保证输出点的均匀分布
    let mut bytes = [0u8; 64];
    bytes.copy_from_slice(&hash);
    RistrettoPoint::from_uniform_bytes(&bytes)
}

/// 示例 1: 基本 Hash-to-Point
pub fn example_hash_to_point() {
    println!("\n=== 示例 1: Hash-to-Point ===");
    
    // 哈希不同数据到点
    let data1 = b"Hello, Monero!";
    let data2 = b"Different data";
    
    let point1 = hash_to_point(data1);
    let point2 = hash_to_point(data2);
    
    println!("Hash('Hello, Monero!'): {:?}", point1.compress());
    println!("Hash('Different data'): {:?}", point2.compress());
    
    // 验证: 不同数据产生不同点
    assert_ne!(point1, point2);
    println!("✅ 验证: 不同数据产生不同点");
    
    // 验证: 相同数据产生相同点 (确定性)
    let point1_again = hash_to_point(data1);
    assert_eq!(point1, point1_again);
    println!("✅ 验证: Hash-to-Point 是确定性的");
}

/// 示例 2: Key Image 生成 (Monero 防双花机制)
/// 
/// Key Image: I = x * Hp(P)
/// - x: 私钥 (标量)
/// - P: 公钥 (点), P = x*G
/// - Hp: Hash-to-Point 函数
/// - I: Key Image (唯一标识输出, 可公开)
pub fn example_key_image() {
    println!("\n=== 示例 2: Key Image 生成 ===");
    
    // 生成密钥对
    let secret_key = Scalar::random(&mut OsRng);
    let public_key = secret_key * RISTRETTO_BASEPOINT_POINT;
    
    println!("私钥: {:?}", secret_key);
    println!("公钥: {:?}", public_key.compress());
    
    // 生成 Key Image
    // I = x * Hp(P)
    let public_key_bytes = public_key.compress().to_bytes();
    let hash_point = hash_to_point(&public_key_bytes);
    let key_image = secret_key * hash_point;
    
    println!("Hash-to-Point(P): {:?}", hash_point.compress());
    println!("Key Image I = x*Hp(P): {:?}", key_image.compress());
    
    // 验证: 知道私钥才能生成正确的 Key Image
    // 攻击者不知道 x, 无法计算 I = x * Hp(P)
    println!("✅ Key Image 绑定到私钥, 但不泄露私钥");
}

/// 示例 3: Key Image 防双花
/// 
/// 场景: 用户试图花费同一输出两次
pub fn example_double_spend_prevention() {
    println!("\n=== 示例 3: Key Image 防双花 ===");
    
    // Alice 的密钥对
    let alice_secret = Scalar::random(&mut OsRng);
    let alice_public = alice_secret * RISTRETTO_BASEPOINT_POINT;
    
    // Alice 收到一笔输出, 生成 Key Image
    let output1_hash_point = hash_to_point(&alice_public.compress().to_bytes());
    let key_image_1 = alice_secret * output1_hash_point;
    
    println!("Alice 第一次花费, Key Image: {:?}", key_image_1.compress());
    
    // Alice 试图再次花费同一输出
    let key_image_2 = alice_secret * output1_hash_point;
    
    println!("Alice 第二次花费, Key Image: {:?}", key_image_2.compress());
    
    // 验证: 两次花费产生相同的 Key Image
    assert_eq!(key_image_1, key_image_2);
    println!("✅ 相同输出产生相同 Key Image → 全网可检测双花!");
    
    // 验证: 不同输出产生不同 Key Image
    let bob_public = Scalar::random(&mut OsRng) * RISTRETTO_BASEPOINT_POINT;
    let output2_hash_point = hash_to_point(&bob_public.compress().to_bytes());
    let key_image_3 = alice_secret * output2_hash_point;
    
    assert_ne!(key_image_1, key_image_3);
    println!("✅ 不同输出产生不同 Key Image");
}

/// 示例 4: Hash-to-Point 与域分离
/// 
/// 不同用途的哈希应使用域分离标签
pub fn example_domain_separation() {
    println!("\n=== 示例 4: 域分离 ===");
    
    let data = b"Same data";
    
    // 不同域的 Hash-to-Point
    let point_domain1 = {
        let mut hasher = Sha512::new();
        hasher.update(b"DOMAIN1:");  // 域标签
        hasher.update(data);
        let hash = hasher.finalize();
        let mut bytes = [0u8; 64];
        bytes.copy_from_slice(&hash);
        RistrettoPoint::from_uniform_bytes(&bytes)
    };
    
    let point_domain2 = {
        let mut hasher = Sha512::new();
        hasher.update(b"DOMAIN2:");  // 不同域标签
        hasher.update(data);
        let hash = hasher.finalize();
        let mut bytes = [0u8; 64];
        bytes.copy_from_slice(&hash);
        RistrettoPoint::from_uniform_bytes(&bytes)
    };
    
    // 验证: 相同数据在不同域产生不同点
    assert_ne!(point_domain1, point_domain2);
    println!("✅ 域分离防止跨协议攻击");
    println!("Domain1: {:?}", point_domain1.compress());
    println!("Domain2: {:?}", point_domain2.compress());
}

/// 运行所有示例
pub fn run_all_examples() {
    example_hash_to_point();
    example_key_image();
    example_double_spend_prevention();
    example_domain_separation();
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_hash_to_point_deterministic() {
        let data = b"test data";
        let point1 = hash_to_point(data);
        let point2 = hash_to_point(data);
        assert_eq!(point1, point2);
    }
    
    #[test]
    fn test_hash_to_point_different_inputs() {
        let point1 = hash_to_point(b"input1");
        let point2 = hash_to_point(b"input2");
        assert_ne!(point1, point2);
    }
    
    #[test]
    fn test_key_image_uniqueness() {
        // 相同私钥 + 相同公钥 = 相同 Key Image
        let secret = Scalar::random(&mut OsRng);
        let public = secret * RISTRETTO_BASEPOINT_POINT;
        
        let hash_point = hash_to_point(&public.compress().to_bytes());
        let ki1 = secret * hash_point;
        let ki2 = secret * hash_point;
        
        assert_eq!(ki1, ki2);
    }
    
    #[test]
    fn test_key_image_binding() {
        // 不同私钥 + 相同公钥哈希 = 不同 Key Image
        let secret1 = Scalar::random(&mut OsRng);
        let secret2 = Scalar::random(&mut OsRng);
        let public = secret1 * RISTRETTO_BASEPOINT_POINT;
        
        let hash_point = hash_to_point(&public.compress().to_bytes());
        let ki1 = secret1 * hash_point;
        let ki2 = secret2 * hash_point;
        
        assert_ne!(ki1, ki2);
    }
}
