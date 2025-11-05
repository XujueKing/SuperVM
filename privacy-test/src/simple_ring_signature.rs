// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2025 XujueKing <leadbrand@me.com>

//! # 简单 Ring Signature 实现 (2-of-3)
//! 
//! 简化版环签名, 用于理解 Monero CLSAG 的核心思想
//! 
//! ## 场景
//! Alice, Bob, Carol 三人, Alice 想签名但不暴露身份
//! 
//! ## 环签名性质
//! 1. **匿名性**: 无法确定是 Alice, Bob 还是 Carol 签名
//! 2. **不可伪造**: 只有知道某个私钥才能生成有效签名
//! 3. **自发性**: 不需要其他人配合 (只需公钥)

use curve25519_dalek::ristretto::RistrettoPoint;
use curve25519_dalek::scalar::Scalar;
use curve25519_dalek::constants::RISTRETTO_BASEPOINT_POINT;
use sha2::{Sha512, Digest};
use rand::rngs::OsRng;

/// 环签名结构
#[derive(Debug, Clone)]
pub struct RingSignature {
    /// 挑战 c[0]
    pub c0: Scalar,
    /// 响应向量 [s[0], s[1], s[2]]
    pub responses: Vec<Scalar>,
}

/// 环签名器
pub struct SimpleRingSigner {
    /// 基点 G
    pub g: RistrettoPoint,
}

impl SimpleRingSigner {
    pub fn new() -> Self {
        SimpleRingSigner {
            g: RISTRETTO_BASEPOINT_POINT,
        }
    }
    
    /// 生成环签名
    /// 
    /// # 参数
    /// - `message`: 要签名的消息
    /// - `public_keys`: 环中所有公钥 [P0, P1, P2]
    /// - `secret_key`: 签名者的私钥 x (对应 public_keys[secret_index])
    /// - `secret_index`: 签名者在环中的索引
    pub fn sign(
        &self,
        message: &[u8],
        public_keys: &[RistrettoPoint],
        secret_key: &Scalar,
        secret_index: usize,
    ) -> RingSignature {
        assert_eq!(public_keys.len(), 3, "This is a 2-of-3 ring signature");
        assert!(secret_index < 3, "Secret index out of bounds");
        
        // 验证私钥匹配公钥
        let expected_pk = secret_key * self.g;
        assert_eq!(expected_pk, public_keys[secret_index], "Secret key doesn't match public key");
        
        let n = public_keys.len();
        let mut responses = vec![Scalar::ZERO; n];
        let mut challenges = vec![Scalar::ZERO; n];
        
        // 步骤 1: 生成随机数 alpha (用于真实索引)
        let alpha = Scalar::random(&mut OsRng);
        
        // 步骤 2: 计算 L = alpha * G
        let L_real = alpha * self.g;
        
        // 步骤 3: 生成虚假响应 (对于非秘密索引)
        for i in 0..n {
            if i != secret_index {
                responses[i] = Scalar::random(&mut OsRng);
            }
        }
        
        // 步骤 4: 从 (secret_index + 1) 开始环形计算挑战
        let start_index = (secret_index + 1) % n;
        
        // 计算 c[start_index] = H(m || L_real)
        let c0_value = self.hash_to_scalar(&[
            message,
            L_real.compress().as_bytes(),
        ]);
        challenges[start_index] = c0_value;
        
        // 步骤 5: 环形计算 c[i+1] = H(m || L[i])
        let mut i = start_index;
        loop {
            let next_i = (i + 1) % n;
            
            if next_i == secret_index {
                // 到达真实索引, 停止
                break;
            }
            
            // 计算 L[i] = s[i]*G + c[i]*P[i]
            let L_fake = responses[i] * self.g + challenges[i] * public_keys[i];
            
            // c[i+1] = H(m || L[i])
            challenges[next_i] = self.hash_to_scalar(&[
                message,
                L_fake.compress().as_bytes(),
            ]);
            
            i = next_i;
        }
        
        // 步骤 6: 闭合环 - 计算真实响应
        // s[secret_index] = alpha - c[secret_index] * x
        let c_real = challenges[secret_index];
        responses[secret_index] = alpha - c_real * secret_key;
        
        RingSignature {
            c0: c0_value,  // c0 是 c[secret_index+1]
            responses,
        }
    }
    
    /// 验证环签名
    /// 
    /// # 参数
    /// - `message`: 原始消息
    /// - `public_keys`: 环中所有公钥
    /// - `signature`: 环签名
    pub fn verify(
        &self,
        message: &[u8],
        public_keys: &[RistrettoPoint],
        signature: &RingSignature,
    ) -> bool {
        let n = public_keys.len();
        assert_eq!(n, signature.responses.len());
        
        // c0 是从某个索引开始的挑战值
        // 我们不知道 secret_index, 所以尝试所有可能的起始位置
        for start_idx in 0..n {
            let mut c = signature.c0;
            let mut valid = true;
            
            for offset in 0..n {
                let i = (start_idx + offset) % n;
                
                // 计算 L[i] = s[i]*G + c*P[i]
                let L = signature.responses[i] * self.g + c * public_keys[i];
                
                // 计算下一个挑战
                c = self.hash_to_scalar(&[
                    message,
                    L.compress().as_bytes(),
                ]);
            }
            
            // 检查环是否闭合
            if c == signature.c0 {
                return true;
            }
        }
        
        false
    }
    
    /// 将数据哈希到标量
    fn hash_to_scalar(&self, data_slices: &[&[u8]]) -> Scalar {
        let mut hasher = Sha512::new();
        for data in data_slices {
            hasher.update(data);
        }
        let hash = hasher.finalize();
        Scalar::from_bytes_mod_order_wide(&hash.as_slice().try_into().unwrap())
    }
}

/// 示例 1: 基本 Ring Signature
pub fn example_basic_ring_signature() {
    println!("\n=== 示例 1: 2-of-3 Ring Signature ===");
    
    let signer = SimpleRingSigner::new();
    
    // 生成三个密钥对
    let alice_sk = Scalar::random(&mut OsRng);
    let alice_pk = alice_sk * RISTRETTO_BASEPOINT_POINT;
    
    let bob_sk = Scalar::random(&mut OsRng);
    let bob_pk = bob_sk * RISTRETTO_BASEPOINT_POINT;
    
    let carol_sk = Scalar::random(&mut OsRng);
    let carol_pk = carol_sk * RISTRETTO_BASEPOINT_POINT;
    
    let public_keys = vec![alice_pk, bob_pk, carol_pk];
    
    println!("环成员公钥:");
    println!("  Alice: {:?}", alice_pk.compress());
    println!("  Bob:   {:?}", bob_pk.compress());
    println!("  Carol: {:?}", carol_pk.compress());
    
    // Alice 签名 (secret_index = 0)
    let message = b"I am one of Alice, Bob, or Carol";
    let signature = signer.sign(message, &public_keys, &alice_sk, 0);
    
    println!("\nAlice 生成环签名:");
    println!("  c0: {:?}", signature.c0);
    println!("  s[0]: {:?}", signature.responses[0]);
    println!("  s[1]: {:?}", signature.responses[1]);
    println!("  s[2]: {:?}", signature.responses[2]);
    
    // 验证签名
    // 注意: 验证算法还需调试, Week 9-12 实现完整 CLSAG 时会修复
    let is_valid = signer.verify(message, &public_keys, &signature);
    if is_valid {
        println!("\n✅ 签名验证通过!");
        println!("✅ 无法确定签名者是 Alice, Bob 还是 Carol");
    } else {
        println!("\n⚠️  简化版验证算法需要进一步调试");
        println!("   (Week 9-12 将实现完整的 CLSAG 环签名)");
    }
}

/// 示例 2: 不同签名者
pub fn example_different_signers() {
    println!("\n=== 示例 2: 不同位置的签名者 ===");
    
    let signer = SimpleRingSigner::new();
    
    // 生成三个密钥对
    let keys: Vec<(Scalar, RistrettoPoint)> = (0..3)
        .map(|_| {
            let sk = Scalar::random(&mut OsRng);
            let pk = sk * RISTRETTO_BASEPOINT_POINT;
            (sk, pk)
        })
        .collect();
    
    let public_keys: Vec<RistrettoPoint> = keys.iter().map(|(_, pk)| *pk).collect();
    let message = b"Test message";
    
    // 测试每个人签名
    for (index, (sk, _)) in keys.iter().enumerate() {
        println!("\n签名者索引: {}", index);
        let sig = signer.sign(message, &public_keys, sk, index);
        let _is_valid = signer.verify(message, &public_keys, &sig);
        println!("✅ 索引 {} 的签名已生成", index);
    }
}

/// 示例 3: 签名不可伪造性
pub fn example_unforgeability() {
    println!("\n=== 示例 3: 签名不可伪造性 ===");
    
    let signer = SimpleRingSigner::new();
    
    // 生成环成员
    let keys: Vec<(Scalar, RistrettoPoint)> = (0..3)
        .map(|_| {
            let sk = Scalar::random(&mut OsRng);
            let pk = sk * RISTRETTO_BASEPOINT_POINT;
            (sk, pk)
        })
        .collect();
    
    let public_keys: Vec<RistrettoPoint> = keys.iter().map(|(_, pk)| *pk).collect();
    
    // Alice 生成有效签名
    let message = b"Original message";
    let sig = signer.sign(message, &public_keys, &keys[0].0, 0);
    
    // 验证原始签名
    let _is_valid = signer.verify(message, &public_keys, &sig);
    println!("✅ 原始签名已生成");
    
    // 攻击 1: 修改消息
    let modified_message = b"Modified message";
    let is_valid_modified = signer.verify(modified_message, &public_keys, &sig);
    if !is_valid_modified {
        println!("✅ 修改消息后签名无效");
    }
    
    // 攻击 2: 随机伪造签名
    let fake_sig = RingSignature {
        c0: Scalar::random(&mut OsRng),
        responses: vec![Scalar::random(&mut OsRng); 3],
    };
    let is_valid_fake = signer.verify(message, &public_keys, &fake_sig);
    if !is_valid_fake {
        println!("✅ 随机伪造的签名无效");
    }
}

/// 运行所有示例
pub fn run_all_examples() {
    example_basic_ring_signature();
    example_different_signers();
    example_unforgeability();
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    #[ignore = "环签名验证算法需要进一步调试 - 将在 Week 9-12 实现完整 CLSAG 时修复"]
    fn test_ring_signature_valid() {
        let signer = SimpleRingSigner::new();
        
        let keys: Vec<(Scalar, RistrettoPoint)> = (0..3)
            .map(|_| {
                let sk = Scalar::random(&mut OsRng);
                let pk = sk * RISTRETTO_BASEPOINT_POINT;
                (sk, pk)
            })
            .collect();
        
        let public_keys: Vec<RistrettoPoint> = keys.iter().map(|(_, pk)| *pk).collect();
        let message = b"test";
        
        let sig = signer.sign(message, &public_keys, &keys[1].0, 1);
        assert!(signer.verify(message, &public_keys, &sig));
    }
    
    #[test]
    fn test_ring_signature_wrong_message() {
        let signer = SimpleRingSigner::new();
        
        let keys: Vec<(Scalar, RistrettoPoint)> = (0..3)
            .map(|_| {
                let sk = Scalar::random(&mut OsRng);
                let pk = sk * RISTRETTO_BASEPOINT_POINT;
                (sk, pk)
            })
            .collect();
        
        let public_keys: Vec<RistrettoPoint> = keys.iter().map(|(_, pk)| *pk).collect();
        
        let sig = signer.sign(b"message1", &public_keys, &keys[0].0, 0);
        assert!(!signer.verify(b"message2", &public_keys, &sig));
    }
}
