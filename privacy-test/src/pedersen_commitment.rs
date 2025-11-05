// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2025 XujueKing <leadbrand@me.com>

//! # Pedersen Commitment 实现
//! 
//! 用于在 RingCT 交易中隐藏金额
//! 
//! ## 核心公式
//! C = v*H + r*G
//! - v: 金额 (保密)
//! - r: 盲化因子 (随机, 保密)
//! - H, G: 基点 (公开)
//! - C: 承诺 (公开)
//! 
//! ## 同态性
//! C₁ + C₂ = (v₁ + v₂)*H + (r₁ + r₂)*G
//! 
//! ## 交易验证
//! Σ C_in = Σ C_out + fee*H

use curve25519_dalek::ristretto::RistrettoPoint;
use curve25519_dalek::scalar::Scalar;
use curve25519_dalek::constants::RISTRETTO_BASEPOINT_POINT;
use rand::rngs::OsRng;

/// Pedersen Commitment 生成器
pub struct PedersenCommitment {
    /// 基点 G (公开)
    pub g: RistrettoPoint,
    /// 基点 H (公开, 独立于 G)
    pub h: RistrettoPoint,
}

impl PedersenCommitment {
    /// 创建 Pedersen Commitment 生成器
    /// 
    /// 注意: H 必须是随机选择的点, 且无人知道 log_G(H)
    /// 在实际应用中, H 通常通过 Hash-to-Point 生成
    pub fn new() -> Self {
        let g = RISTRETTO_BASEPOINT_POINT;
        
        // 生成独立的基点 H
        // 在 Monero 中, H 是通过特定协议生成的公开常数
        let h = {
            use crate::hash_to_point::hash_to_point;
            hash_to_point(b"Pedersen_H_basepoint")
        };
        
        PedersenCommitment { g, h }
    }
    
    /// 生成承诺: C = v*H + r*G
    /// 
    /// # 参数
    /// - `value`: 要承诺的金额
    /// - `blinding_factor`: 盲化因子 (随机)
    pub fn commit(&self, value: u64, blinding_factor: &Scalar) -> RistrettoPoint {
        let value_scalar = Scalar::from(value);
        value_scalar * self.h + blinding_factor * self.g
    }
    
    /// 生成随机盲化因子
    pub fn random_blinding_factor() -> Scalar {
        Scalar::random(&mut OsRng)
    }
    
    /// 验证承诺打开 (Opening)
    /// 
    /// 知道 v 和 r 的情况下, 验证 C = v*H + r*G
    pub fn verify_opening(
        &self,
        commitment: &RistrettoPoint,
        value: u64,
        blinding_factor: &Scalar,
    ) -> bool {
        let expected = self.commit(value, blinding_factor);
        *commitment == expected
    }
}

/// 示例 1: 基本 Pedersen Commitment
pub fn example_basic_commitment() {
    println!("\n=== 示例 1: 基本 Pedersen Commitment ===");
    
    let pc = PedersenCommitment::new();
    
    // Alice 承诺金额 100
    let amount = 100u64;
    let blinding = PedersenCommitment::random_blinding_factor();
    let commitment = pc.commit(amount, &blinding);
    
    println!("金额: {} (保密)", amount);
    println!("盲化因子: {:?} (保密)", blinding);
    println!("承诺 C: {:?} (公开)", commitment.compress());
    
    // 验证: 知道 v 和 r 可以重建承诺
    let is_valid = pc.verify_opening(&commitment, amount, &blinding);
    assert!(is_valid);
    println!("✅ 承诺验证成功");
    
    // 验证: 不知道 v 和 r 无法重建承诺
    let wrong_amount = 200u64;
    let is_invalid = pc.verify_opening(&commitment, wrong_amount, &blinding);
    assert!(!is_invalid);
    println!("✅ 错误的金额无法通过验证");
}

/// 示例 2: 同态加法
/// 
/// C₁ + C₂ = (v₁ + v₂)*H + (r₁ + r₂)*G
pub fn example_homomorphic_addition() {
    println!("\n=== 示例 2: 同态加法 ===");
    
    let pc = PedersenCommitment::new();
    
    // Alice: 100 XMR
    let v1 = 100u64;
    let r1 = PedersenCommitment::random_blinding_factor();
    let C1 = pc.commit(v1, &r1);
    
    // Bob: 200 XMR
    let v2 = 200u64;
    let r2 = PedersenCommitment::random_blinding_factor();
    let C2 = pc.commit(v2, &r2);
    
    println!("C₁ (100 XMR): {:?}", C1.compress());
    println!("C₂ (200 XMR): {:?}", C2.compress());
    
    // 方法 1: 先加金额和盲化因子, 再承诺
    let v_sum = v1 + v2;
    let r_sum = r1 + r2;
    let C_sum_1 = pc.commit(v_sum, &r_sum);
    
    // 方法 2: 直接加承诺
    let C_sum_2 = C1 + C2;
    
    // 验证: 两种方法等价 (同态性)
    assert_eq!(C_sum_1, C_sum_2);
    println!("✅ 同态性: C₁ + C₂ = Commit(v₁+v₂, r₁+r₂)");
    println!("C₁ + C₂: {:?}", C_sum_2.compress());
}

/// 示例 3: RingCT 交易验证
/// 
/// 验证: Σ输入承诺 = Σ输出承诺 + 手续费承诺
pub fn example_transaction_verification() {
    println!("\n=== 示例 3: RingCT 交易验证 ===");
    
    let pc = PedersenCommitment::new();
    
    // 输入: Alice 有 1000 XMR
    let input_amount = 1000u64;
    let input_blinding = PedersenCommitment::random_blinding_factor();
    let input_commitment = pc.commit(input_amount, &input_blinding);
    
    println!("输入: 1000 XMR (隐藏)");
    println!("输入承诺: {:?}", input_commitment.compress());
    
    // 输出 1: 给 Bob 600 XMR
    let output1_amount = 600u64;
    let output1_blinding = PedersenCommitment::random_blinding_factor();
    let output1_commitment = pc.commit(output1_amount, &output1_blinding);
    
    // 输出 2: 找零给 Alice 390 XMR
    let output2_amount = 390u64;
    let output2_blinding = PedersenCommitment::random_blinding_factor();
    let output2_commitment = pc.commit(output2_amount, &output2_blinding);
    
    // 手续费: 10 XMR (公开)
    let fee = 10u64;
    let fee_commitment = pc.commit(fee, &Scalar::ZERO);  // 手续费盲化因子为0
    
    println!("输出1: 600 XMR (隐藏)");
    println!("输出2: 390 XMR (隐藏)");
    println!("手续费: 10 XMR (公开)");
    
    // 验证方程: C_in = C_out1 + C_out2 + C_fee
    // 需要调整盲化因子: r_in = r_out1 + r_out2 + 0
    // 但我们作弊了, 实际交易中需要 range proof
    
    // 正确的验证 (需要 proof of zero balance)
    // 这里我们演示如果盲化因子满足关系, 验证会通过
    
    // 重新生成输出承诺, 使盲化因子平衡
    let output1_blinding_correct = PedersenCommitment::random_blinding_factor();
    let output2_blinding_correct = input_blinding - output1_blinding_correct;  // 确保 r_in = r_out1 + r_out2
    
    let output1_correct = pc.commit(output1_amount, &output1_blinding_correct);
    let output2_correct = pc.commit(output2_amount, &output2_blinding_correct);
    
    // 验证: 输入 = 输出1 + 输出2 + 手续费
    let outputs_sum = output1_correct + output2_correct + fee_commitment;
    assert_eq!(input_commitment, outputs_sum);
    println!("✅ 交易平衡验证通过!");
    println!("验证方程: C_in = C_out1 + C_out2 + C_fee");
}

/// 示例 4: 隐私性
/// 
/// 承诺隐藏了金额, 但保持可验证性
pub fn example_privacy() {
    println!("\n=== 示例 4: 隐私性 ===");
    
    let pc = PedersenCommitment::new();
    
    // Alice 承诺 100 XMR
    let amount = 100u64;
    let blinding = PedersenCommitment::random_blinding_factor();
    let commitment = pc.commit(amount, &blinding);
    
    println!("Alice 的承诺: {:?}", commitment.compress());
    
    // 攻击者尝试猜测金额
    for guess in [50, 100, 150, 200] {
        // 攻击者不知道盲化因子, 无法验证猜测
        let attacker_blinding = PedersenCommitment::random_blinding_factor();
        let attacker_guess = pc.commit(guess, &attacker_blinding);
        
        if attacker_guess == commitment {
            println!("猜对了: {} XMR", guess);
        } else {
            println!("❌ 猜测 {} XMR 失败 (盲化因子不匹配)", guess);
        }
    }
    
    println!("✅ 承诺隐藏了金额, 即使猜对金额也无法验证");
    println!("(需要知道盲化因子才能验证)");
}

/// 运行所有示例
pub fn run_all_examples() {
    example_basic_commitment();
    example_homomorphic_addition();
    example_transaction_verification();
    example_privacy();
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_commitment_opening() {
        let pc = PedersenCommitment::new();
        let value = 12345u64;
        let blinding = PedersenCommitment::random_blinding_factor();
        let commitment = pc.commit(value, &blinding);
        
        assert!(pc.verify_opening(&commitment, value, &blinding));
        assert!(!pc.verify_opening(&commitment, value + 1, &blinding));
    }
    
    #[test]
    fn test_homomorphic_property() {
        let pc = PedersenCommitment::new();
        
        let v1 = 100u64;
        let r1 = PedersenCommitment::random_blinding_factor();
        let C1 = pc.commit(v1, &r1);
        
        let v2 = 200u64;
        let r2 = PedersenCommitment::random_blinding_factor();
        let C2 = pc.commit(v2, &r2);
        
        let C_sum = C1 + C2;
        let expected = pc.commit(v1 + v2, &(r1 + r2));
        
        assert_eq!(C_sum, expected);
    }
    
    #[test]
    fn test_transaction_balance() {
        let pc = PedersenCommitment::new();
        
        // 输入: 1000
        let input = 1000u64;
        let r_in = PedersenCommitment::random_blinding_factor();
        let C_in = pc.commit(input, &r_in);
        
        // 输出: 600 + 390 + 10 (fee)
        let out1 = 600u64;
        let out2 = 390u64;
        let fee = 10u64;
        
        let r_out1 = PedersenCommitment::random_blinding_factor();
        let r_out2 = r_in - r_out1;  // 平衡盲化因子
        
        let C_out1 = pc.commit(out1, &r_out1);
        let C_out2 = pc.commit(out2, &r_out2);
        let C_fee = pc.commit(fee, &Scalar::ZERO);
        
        assert_eq!(C_in, C_out1 + C_out2 + C_fee);
    }
}
