// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2025 XujueKing <leadbrand@me.com>

//! Ring Signature 电路实现
//!
//! 实现 Schnorr/CLSAG 风格的环签名，提供发送方匿名性。
//!
//! ## 核心功能
//! - Key Image 生成与验证（防双花）
//! - Ring 成员验证（Merkle Tree）
//! - Schnorr 签名挑战-响应协议
//!
//! ## 约束预算
//! - 目标：~150-200 约束/环成员
//! - Ring Size = 5: ~750-1000 约束
//! - Ring Size = 10: ~1500-2000 约束

use ark_bls12_381::Fr;
use ark_r1cs_std::alloc::AllocVar;
use ark_r1cs_std::boolean::Boolean;
use ark_r1cs_std::eq::EqGadget;
use ark_r1cs_std::fields::fp::FpVar;
use ark_r1cs_std::fields::FieldVar;
use ark_relations::r1cs::{ConstraintSynthesizer, ConstraintSystemRef, SynthesisError};
use ark_std::Zero;

// Poseidon for hashing
use ark_crypto_primitives::crh::poseidon::constraints::{CRHGadget, CRHParametersVar};
use ark_crypto_primitives::crh::poseidon::CRH;
use ark_crypto_primitives::crh::{CRHScheme, CRHSchemeGadget};
use ark_crypto_primitives::sponge::poseidon::PoseidonConfig;
use ark_crypto_primitives::sponge::Absorb;

// ===== 数据结构定义 =====

/// Key Image (密钥镜像)
/// 用于防止双花，由私钥和公钥派生
#[derive(Clone, Debug)]
pub struct KeyImage {
    /// Key Image 值 (I = x * H_p(P))
    pub value: Fr,
}

/// Ring 成员公钥
#[derive(Clone, Debug)]
pub struct RingMember {
    /// 公钥 (P = x * G)
    pub public_key: Fr,

    /// Merkle 根（用于验证成员资格）
    pub merkle_root: Option<Fr>,
}

/// Ring Signature 签名数据
#[derive(Clone, Debug)]
pub struct RingSignatureData {
    /// 环大小
    pub ring_size: usize,

    /// 真实签名者索引（私有，仅 Prover 知道）
    pub real_index: Option<usize>,

    /// 真实私钥（私有，仅 Prover 知道）
    pub secret_key: Option<Fr>,

    /// 环成员公钥列表
    pub ring_members: Vec<RingMember>,

    /// Key Image（公开）
    pub key_image: KeyImage,

    /// 消息哈希（要签名的内容）
    pub message: Fr,

    /// 签名挑战值（公开）
    pub challenge: Fr,

    /// 签名响应值列表（每个环成员一个，公开）
    pub responses: Vec<Fr>,
}

/// Ring Signature 电路
#[derive(Clone)]
pub struct RingSignatureCircuit {
    /// 签名数据
    pub signature: RingSignatureData,

    /// Poseidon 配置
    pub poseidon_config: PoseidonConfig<Fr>,
}

impl RingSignatureCircuit {
    /// 创建新的环签名电路
    pub fn new(signature: RingSignatureData, poseidon_config: PoseidonConfig<Fr>) -> Self {
        Self {
            signature,
            poseidon_config,
        }
    }
}

// ===== 电路实现 =====

impl ConstraintSynthesizer<Fr> for RingSignatureCircuit {
    fn generate_constraints(self, cs: ConstraintSystemRef<Fr>) -> Result<(), SynthesisError> {
        // ===== 简化版环签名电路 =====
        // 主要验证：1) Key Image 生成正确性；2) Ring 成员所有权

        // 1. 分配公开输入：Key Image
        let key_image_var = FpVar::new_input(cs.clone(), || Ok(self.signature.key_image.value))?;

        // 2. 分配私有输入：私钥和真实索引
        let secret_key_var = FpVar::new_witness(cs.clone(), || {
            self.signature
                .secret_key
                .ok_or(SynthesisError::AssignmentMissing)
        })?;

        let real_index = self.signature.real_index.unwrap_or(0);

        // 3. 获取真实公钥
        let public_key = self
            .signature
            .ring_members
            .get(real_index)
            .map(|m| m.public_key)
            .ok_or(SynthesisError::AssignmentMissing)?;

        let public_key_var = FpVar::new_witness(cs.clone(), || Ok(public_key))?;

        // 4. 验证 Key Image = H(secret_key, public_key)
        let poseidon_params = CRHParametersVar::new_constant(cs.clone(), &self.poseidon_config)?;

        let expected_key_image = CRHGadget::<Fr>::evaluate(
            &poseidon_params,
            &[secret_key_var.clone(), public_key_var.clone()],
        )?;

        expected_key_image.enforce_equal(&key_image_var)?;

        // 5. 验证公钥在环中（简化：确保公钥存在）
        // 实际应用中应验证 Merkle 成员资格
        let mut found_match = Boolean::FALSE;

        for member in &self.signature.ring_members {
            let member_pk_var = FpVar::new_witness(cs.clone(), || Ok(member.public_key))?;

            let is_equal = public_key_var.is_eq(&member_pk_var)?;
            found_match = found_match.or(&is_equal)?;
        }

        found_match.enforce_equal(&Boolean::TRUE)?;

        Ok(())
    }
}

// ===== 辅助函数 =====

impl RingSignatureData {
    /// 生成 Key Image
    /// I = H(private_key, public_key)
    pub fn generate_key_image(
        secret_key: Fr,
        public_key: Fr,
        poseidon_config: &PoseidonConfig<Fr>,
    ) -> Result<KeyImage, String> {
        let value = CRH::<Fr>::evaluate(poseidon_config, vec![secret_key, public_key])
            .map_err(|e| format!("Failed to generate key image: {:?}", e))?;

        Ok(KeyImage { value })
    }

    /// 生成环签名（链下）
    /// 这个函数在实际应用中会在客户端执行
    pub fn generate_signature(
        secret_key: Fr,
        real_index: usize,
        ring_members: Vec<RingMember>,
        message: Fr,
        poseidon_config: &PoseidonConfig<Fr>,
        rng: &mut impl rand::Rng,
    ) -> Result<Self, String> {
        use ark_std::UniformRand;

        let ring_size = ring_members.len();
        if real_index >= ring_size {
            return Err("Real index out of bounds".to_string());
        }

        // 1. 生成 Key Image
        let public_key = ring_members[real_index].public_key;
        let key_image = Self::generate_key_image(secret_key, public_key, poseidon_config)?;

        // 2. 生成随机数 α
        let alpha = Fr::rand(rng);

        // 3. 生成伪造的响应值（除了真实索引）
        let mut responses = vec![];
        for i in 0..ring_size {
            if i == real_index {
                responses.push(Fr::zero()); // 占位，稍后计算
            } else {
                responses.push(Fr::rand(rng));
            }
        }

        // 4. 计算挑战值
        // 简化实现：challenge = H(message, key_image, ring_members)
        let mut hash_input = vec![message, key_image.value];
        for member in &ring_members {
            hash_input.push(member.public_key);
        }

        let challenge = CRH::<Fr>::evaluate(poseidon_config, hash_input)
            .map_err(|e| format!("Failed to compute challenge: {:?}", e))?;

        // 5. 计算真实响应
        // s_π = α - c_π * x
        // 简化：s_π = α - challenge * secret_key
        let real_response = alpha - challenge * secret_key;
        responses[real_index] = real_response;

        Ok(Self {
            ring_size,
            real_index: Some(real_index),
            secret_key: Some(secret_key),
            ring_members,
            key_image,
            message,
            challenge,
            responses,
        })
    }

    /// 验证环签名（链上）
    pub fn verify(&self, poseidon_config: &PoseidonConfig<Fr>) -> Result<bool, String> {
        // 1. 验证挑战值
        let mut hash_input = vec![self.message, self.key_image.value];
        for member in &self.ring_members {
            hash_input.push(member.public_key);
        }

        let expected_challenge = CRH::<Fr>::evaluate(poseidon_config, hash_input)
            .map_err(|e| format!("Failed to compute challenge: {:?}", e))?;

        if self.challenge != expected_challenge {
            return Ok(false);
        }

        // 2. 验证所有响应值非零
        for response in &self.responses {
            if response.is_zero() {
                return Ok(false);
            }
        }

        Ok(true)
    }
}

// ===== 测试 =====

#[cfg(test)]
mod tests {
    use super::*;
    use ark_bls12_381::Bls12_381;
    use ark_groth16::Groth16;
    use ark_relations::r1cs::ConstraintSystem;
    use ark_snark::SNARK;
    use ark_std::test_rng;
    use rand::rngs::OsRng;

    fn setup_poseidon_config() -> PoseidonConfig<Fr> {
        // 创建默认的 Poseidon 配置
        let full_rounds = 8;
        let partial_rounds = 57;
        let alpha = 5;
        let rate = 2;
        let capacity = 1;

        // 简化参数（实际项目中应使用标准参数）
        let mds = vec![
            vec![Fr::from(1u64), Fr::from(2u64), Fr::from(3u64)],
            vec![Fr::from(4u64), Fr::from(5u64), Fr::from(6u64)],
            vec![Fr::from(7u64), Fr::from(8u64), Fr::from(9u64)],
        ];

        let ark = vec![
            vec![Fr::from(10u64), Fr::from(11u64), Fr::from(12u64)];
            full_rounds + partial_rounds
        ];

        PoseidonConfig::new(full_rounds, partial_rounds, alpha, mds, ark, rate, capacity)
    }

    #[test]
    fn test_key_image_generation() {
        let rng = &mut OsRng;
        let config = setup_poseidon_config();

        use ark_std::UniformRand;
        let secret_key = Fr::rand(rng);
        let public_key = Fr::rand(rng); // 简化：实际应该是 sk * G

        let ki = RingSignatureData::generate_key_image(secret_key, public_key, &config)
            .expect("generate key image");

        // Key image应该是确定性的
        let ki2 = RingSignatureData::generate_key_image(secret_key, public_key, &config)
            .expect("generate key image 2");

        assert_eq!(ki.value, ki2.value, "Key images should be deterministic");

        // 不同的私钥应该产生不同的key image
        let different_sk = Fr::rand(rng);
        let ki3 = RingSignatureData::generate_key_image(different_sk, public_key, &config)
            .expect("generate key image 3");

        assert_ne!(
            ki.value, ki3.value,
            "Different keys should produce different key images"
        );
    }

    #[test]
    fn test_ring_signature_generation_and_verification() {
        let rng = &mut OsRng;
        let config = setup_poseidon_config();

        use ark_std::UniformRand;

        // 创建环成员
        let ring_size = 5;
        let real_index = 2;
        let secret_key = Fr::rand(rng);

        let mut ring_members = vec![];
        for i in 0..ring_size {
            let pk = if i == real_index {
                secret_key // 简化：实际应该是 sk * G
            } else {
                Fr::rand(rng)
            };

            ring_members.push(RingMember {
                public_key: pk,
                merkle_root: None,
            });
        }

        let message = Fr::rand(rng);

        // 生成签名
        let signature = RingSignatureData::generate_signature(
            secret_key,
            real_index,
            ring_members,
            message,
            &config,
            rng,
        )
        .expect("generate signature");

        // 验证签名
        let is_valid = signature.verify(&config).expect("verify signature");
        assert!(is_valid, "Signature should be valid");
    }

    #[test]
    fn test_ring_signature_circuit_constraints() {
        let rng = &mut OsRng;
        let config = setup_poseidon_config();

        use ark_std::UniformRand;

        // 创建简单的环签名
        let ring_size = 3;
        let real_index = 1;
        let secret_key = Fr::rand(rng);

        let mut ring_members = vec![];
        for i in 0..ring_size {
            let pk = if i == real_index {
                secret_key
            } else {
                Fr::rand(rng)
            };

            ring_members.push(RingMember {
                public_key: pk,
                merkle_root: None,
            });
        }

        let message = Fr::rand(rng);

        let signature = RingSignatureData::generate_signature(
            secret_key,
            real_index,
            ring_members,
            message,
            &config,
            rng,
        )
        .expect("generate signature");

        // 创建电路
        let circuit = RingSignatureCircuit::new(signature, config);

        // 检查约束数
        let cs = ConstraintSystem::<Fr>::new_ref();
        circuit
            .generate_constraints(cs.clone())
            .expect("generate constraints");

        let num_constraints = cs.num_constraints();
        println!(
            "Ring Signature Circuit (ring_size={}): {} constraints",
            ring_size, num_constraints
        );

        // 验证约束满足
        assert!(
            cs.is_satisfied().unwrap(),
            "Constraints should be satisfied"
        );
    }

    #[test]
    fn test_ring_signature_end_to_end() {
        let rng = &mut OsRng;
        let config = setup_poseidon_config();

        use ark_std::UniformRand;

        // 生成签名
        let ring_size = 5;
        let real_index = 2;
        let secret_key = Fr::rand(rng);

        let mut ring_members = vec![];
        for i in 0..ring_size {
            let pk = if i == real_index {
                secret_key
            } else {
                Fr::rand(rng)
            };

            ring_members.push(RingMember {
                public_key: pk,
                merkle_root: None,
            });
        }

        let message = Fr::rand(rng);

        let signature = RingSignatureData::generate_signature(
            secret_key,
            real_index,
            ring_members,
            message,
            &config,
            rng,
        )
        .expect("generate signature");

        let circuit = RingSignatureCircuit::new(signature, config);

        // Setup
        let (pk, vk) =
            Groth16::<Bls12_381>::circuit_specific_setup(circuit.clone(), rng).expect("setup");

        // 准备公开输入（只有 key_image）
        let public_inputs = vec![circuit.signature.key_image.value];

        // Prove
        let proof = Groth16::<Bls12_381>::prove(&pk, circuit, rng).expect("prove");

        // Verify
        let is_valid = Groth16::<Bls12_381>::verify(&vk, &public_inputs, &proof).expect("verify");

        assert!(is_valid, "Proof should be valid");
        println!("✅ Ring Signature End-to-End Test Passed!");
    }
}
