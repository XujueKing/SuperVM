// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2025 XujueKing <leadbrand@me.com>

//! Real ZK Verifier Integration
//!
//! 基于 ark-groth16 的真实 ZK 验证器

use ark_bls12_381::{Bls12_381, Fr};
use ark_groth16::{prepare_verifying_key, Groth16, PreparedVerifyingKey, Proof, ProvingKey};
use ark_serialize::{CanonicalDeserialize, CanonicalSerialize};
use ark_snark::SNARK;
use std::sync::Arc;

/// ZK 证明数据（序列化后的字节）
pub type ProofBytes = Vec<u8>;

/// ZK 公开输入（序列化后的字节）
pub type PublicInputBytes = Vec<u8>;

/// ZK 验证器特征
pub trait ZkVerifier: Send + Sync {
    /// 验证证明
    ///
    /// # Arguments
    /// * `proof_bytes` - 序列化的 Groth16 proof
    /// * `public_inputs_bytes` - 序列化的公开输入
    ///
    /// # Returns
    /// * `Ok(true)` - 验证成功
    /// * `Ok(false)` - 验证失败
    /// * `Err(_)` - 反序列化错误或其他系统错误
    fn verify(&self, proof_bytes: &[u8], public_inputs_bytes: &[u8]) -> Result<bool, ZkError>;
    
    /// 获取验证器类型
    fn verifier_type(&self) -> &str;
}

/// ZK 错误类型
#[derive(Debug, thiserror::Error)]
pub enum ZkError {
    #[error("Proof deserialization failed: {0}")]
    ProofDeserializationError(String),
    
    #[error("Public input deserialization failed: {0}")]
    PublicInputDeserializationError(String),
    
    #[error("Verification failed: {0}")]
    VerificationError(String),
    
    #[error("Setup not initialized")]
    SetupNotInitialized,
}

/// Groth16 验证器（基于 BLS12-381 曲线）
pub struct Groth16Verifier {
    /// 预处理的验证密钥
    pvk: Arc<PreparedVerifyingKey<Bls12_381>>,
}

impl Groth16Verifier {
    /// 从验证密钥创建验证器
    pub fn new(vk: &ark_groth16::VerifyingKey<Bls12_381>) -> Self {
        let pvk = prepare_verifying_key(vk);
        Self {
            pvk: Arc::new(pvk),
        }
    }
    
    /// 从 CRS（Common Reference String）创建验证器
    ///
    /// 生产环境应使用预生成的可信设置参数
    pub fn from_proving_key(pk: &ProvingKey<Bls12_381>) -> Self {
        Self::new(&pk.vk)
    }
    
    /// 创建用于测试的验证器（使用简单电路）
    ///
    /// 警告：仅用于演示，生产环境需要真实的 Trusted Setup
    pub fn new_for_testing() -> Result<Self, ZkError> {
        use rand::rngs::OsRng;
        use zk_groth16_test::MultiplyCircuit;
        
        let rng = &mut OsRng;
        let circuit = MultiplyCircuit { a: None, b: None };
        
        let params = Groth16::<Bls12_381>::generate_random_parameters_with_reduction(circuit, rng)
            .map_err(|e| ZkError::SetupNotInitialized)?;
        
        Ok(Self::from_proving_key(&params))
    }
}

impl ZkVerifier for Groth16Verifier {
    fn verify(&self, proof_bytes: &[u8], public_inputs_bytes: &[u8]) -> Result<bool, ZkError> {
        // 1. 反序列化 Proof
        let proof = Proof::<Bls12_381>::deserialize_compressed(proof_bytes)
            .map_err(|e| ZkError::ProofDeserializationError(e.to_string()))?;
        
        // 2. 反序列化公开输入（Vec<Fr>）
        let public_inputs: Vec<Fr> = Vec::<Fr>::deserialize_compressed(public_inputs_bytes)
            .map_err(|e| ZkError::PublicInputDeserializationError(e.to_string()))?;
        
        // 3. 验证
        let result = Groth16::<Bls12_381>::verify_proof(&self.pvk, &proof, &public_inputs)
            .map_err(|e| ZkError::VerificationError(e.to_string()))?;
        
        Ok(result)
    }
    
    fn verifier_type(&self) -> &str {
        "Groth16-BLS12-381"
    }
}

/// 用于生成测试证明的辅助函数（示例也需要访问，因此移除 cfg(test)）
pub fn generate_test_proof() -> Result<(ProofBytes, PublicInputBytes), ZkError> {
    use rand::rngs::OsRng;
    use zk_groth16_test::MultiplyCircuit;
    
    let rng = &mut OsRng;
    
    // Trusted Setup
    let circuit_template = MultiplyCircuit { a: None, b: None };
    let params = Groth16::<Bls12_381>::generate_random_parameters_with_reduction(
        circuit_template,
        rng,
    )
    .map_err(|_| ZkError::SetupNotInitialized)?;
    
    // 生成证明：a=7, b=11, c=77
    let a = Fr::from(7u64);
    let b = Fr::from(11u64);
    let c = a * b; // 77
    
    let proof = Groth16::<Bls12_381>::prove(
        &params,
        MultiplyCircuit {
            a: Some(a),
            b: Some(b),
        },
        rng,
    )
    .map_err(|e| ZkError::VerificationError(e.to_string()))?;
    
    // 序列化
    let mut proof_bytes = Vec::new();
    proof
        .serialize_compressed(&mut proof_bytes)
        .map_err(|e| ZkError::ProofDeserializationError(e.to_string()))?;
    
    let mut public_input_bytes = Vec::new();
    vec![c]
        .serialize_compressed(&mut public_input_bytes)
        .map_err(|e| ZkError::PublicInputDeserializationError(e.to_string()))?;
    
    Ok((proof_bytes, public_input_bytes))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_groth16_verifier_valid_proof() {
        // 使用统一的 setup 参数生成 proving key 与 verifying key，避免不一致
        use rand::rngs::OsRng;
        use zk_groth16_test::MultiplyCircuit;

        let rng = &mut OsRng;

        // Trusted Setup（一次性）
        let circuit_template = MultiplyCircuit { a: None, b: None };
        let params = Groth16::<Bls12_381>::generate_random_parameters_with_reduction(
            circuit_template,
            rng,
        ).expect("setup failed");

        // 构建验证器（来自同一 params）
        let verifier = Groth16Verifier::from_proving_key(&params);

        // 构造 witness a,b 与公开输入 c=a*b
        let a = Fr::from(13u64);
        let b = Fr::from(17u64);
        let c = a * b; // 221

        // 生成 proof（使用与验证器相同的 params）
        let proof = Groth16::<Bls12_381>::prove(
            &params,
            MultiplyCircuit { a: Some(a), b: Some(b) },
            rng,
        ).expect("prove failed");

        // 序列化 proof
        let mut proof_bytes = Vec::new();
        proof.serialize_compressed(&mut proof_bytes).expect("serialize proof failed");

        // 序列化公开输入 Vec<Fr>
        let mut public_input_bytes = Vec::new();
        vec![c].serialize_compressed(&mut public_input_bytes).expect("serialize public input failed");

        // 验证应该成功
        let result = verifier.verify(&proof_bytes, &public_input_bytes).expect("verify call failed");
        assert!(result, "Valid proof should verify");
    }
    
    #[test]
    fn test_groth16_verifier_invalid_public_input() {
        let verifier = Groth16Verifier::new_for_testing().unwrap();
        let (proof_bytes, _) = generate_test_proof().unwrap();
        
        // 使用错误的公开输入
        let wrong_input = Fr::from(999u64);
        let mut wrong_input_bytes = Vec::new();
        vec![wrong_input]
            .serialize_compressed(&mut wrong_input_bytes)
            .unwrap();
        
        // 验证应该失败
        let result = verifier.verify(&proof_bytes, &wrong_input_bytes);
        assert!(result.is_ok());
        assert!(!result.unwrap(), "Invalid proof should not verify");
    }
    
    #[test]
    fn test_groth16_verifier_corrupted_proof() {
        let verifier = Groth16Verifier::new_for_testing().unwrap();
        
        // 使用损坏的证明字节
        let corrupted_proof = vec![0u8; 100];
        let (_, public_input_bytes) = generate_test_proof().unwrap();
        
        // 应该返回反序列化错误
        let result = verifier.verify(&corrupted_proof, &public_input_bytes);
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), ZkError::ProofDeserializationError(_)));
    }
    
    #[test]
    fn test_verifier_type() {
        let verifier = Groth16Verifier::new_for_testing().unwrap();
        assert_eq!(verifier.verifier_type(), "Groth16-BLS12-381");
    }
}
