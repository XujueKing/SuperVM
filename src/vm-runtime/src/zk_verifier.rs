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

/// ZK 公开输入(序列化后的字节)
pub type PublicInputBytes = Vec<u8>;

/// ZK 后端类型
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ZkBackend {
    /// Groth16 (BLS12-381)
    Groth16Bls12_381,
    /// Plonk (预留)
    #[allow(dead_code)]
    Plonk,
    /// 测试/Mock 后端
    #[allow(dead_code)]
    Mock,
}

impl ZkBackend {
    /// 转换为字符串标识
    pub fn as_str(&self) -> &'static str {
        match self {
            ZkBackend::Groth16Bls12_381 => "groth16-bls12-381",
            ZkBackend::Plonk => "plonk",
            ZkBackend::Mock => "mock",
        }
    }
}

/// ZK 验证器特征
pub trait ZkVerifier: Send + Sync {
    /// 验证证明
    ///
    /// # Arguments
    /// * `proof_bytes` - 序列化的 ZK proof
    /// * `public_inputs_bytes` - 序列化的公开输入
    ///
    /// # Returns
    /// * `Ok(true)` - 验证成功
    /// * `Ok(false)` - 验证失败
    /// * `Err(_)` - 反序列化错误或其他系统错误
    fn verify(&self, proof_bytes: &[u8], public_inputs_bytes: &[u8]) -> Result<bool, ZkError>;
    
    /// 获取验证器类型（字符串描述）
    fn verifier_type(&self) -> &str;
    
    /// 获取后端枚举
    fn backend(&self) -> ZkBackend;
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
            .map_err(|_e| ZkError::SetupNotInitialized)?;
        
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
    
    fn backend(&self) -> ZkBackend {
        ZkBackend::Groth16Bls12_381
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

/// Mock ZK 验证器（用于测试和 CI）
/// 
/// 可配置的验证器，支持：
/// - 总是返回成功/失败
/// - 模拟延迟
/// - 验证调用计数
#[derive(Debug, Clone)]
pub struct MockVerifier {
    /// 验证结果（true=成功, false=失败）
    always_succeed: bool,
    /// 模拟延迟（微秒）
    simulated_delay_us: u64,
    /// 验证调用计数（用于测试断言）
    #[allow(dead_code)]
    call_count: Arc<std::sync::atomic::AtomicU64>,
}

impl MockVerifier {
    /// 创建总是成功的 mock verifier
    pub fn new_always_succeed() -> Self {
        Self {
            always_succeed: true,
            simulated_delay_us: 0,
            call_count: Arc::new(std::sync::atomic::AtomicU64::new(0)),
        }
    }
    
    /// 创建总是失败的 mock verifier
    pub fn new_always_fail() -> Self {
        Self {
            always_succeed: false,
            simulated_delay_us: 0,
            call_count: Arc::new(std::sync::atomic::AtomicU64::new(0)),
        }
    }
    
    /// 创建带延迟的 mock verifier
    pub fn new_with_delay(succeed: bool, delay_us: u64) -> Self {
        Self {
            always_succeed: succeed,
            simulated_delay_us: delay_us,
            call_count: Arc::new(std::sync::atomic::AtomicU64::new(0)),
        }
    }
    
    /// 获取调用次数
    pub fn call_count(&self) -> u64 {
        self.call_count.load(std::sync::atomic::Ordering::Relaxed)
    }
}

impl ZkVerifier for MockVerifier {
    fn verify(&self, _proof_bytes: &[u8], _public_inputs_bytes: &[u8]) -> Result<bool, ZkError> {
        // 增加调用计数
        self.call_count.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        
        // 模拟延迟
        if self.simulated_delay_us > 0 {
            std::thread::sleep(std::time::Duration::from_micros(self.simulated_delay_us));
        }
        
        // 返回配置的结果
        Ok(self.always_succeed)
    }
    
    fn verifier_type(&self) -> &str {
        "Mock"
    }
    
    fn backend(&self) -> ZkBackend {
        ZkBackend::Mock
    }
}

/// 根据环境变量创建 ZK 验证器
/// 
/// 环境变量：
/// - `ZK_VERIFIER_MODE`: "mock" | "real" (默认 "real")
/// - `ZK_MOCK_ALWAYS_SUCCEED`: "true" | "false" (默认 "true")
/// - `ZK_MOCK_DELAY_US`: 延迟微秒数 (默认 "0")
/// 
/// # Examples
/// ```no_run
/// // 设置环境变量使用 mock verifier
/// std::env::set_var("ZK_VERIFIER_MODE", "mock");
/// std::env::set_var("ZK_MOCK_ALWAYS_SUCCEED", "false");
/// 
/// let verifier = vm_runtime::zk_verifier::create_verifier_from_env();
/// ```
pub fn create_verifier_from_env() -> Arc<dyn ZkVerifier> {
    let mode = std::env::var("ZK_VERIFIER_MODE").unwrap_or_else(|_| "real".to_string());
    
    match mode.to_lowercase().as_str() {
        "mock" => {
            let always_succeed = std::env::var("ZK_MOCK_ALWAYS_SUCCEED")
                .unwrap_or_else(|_| "true".to_string())
                .to_lowercase() == "true";
            
            let delay_us = std::env::var("ZK_MOCK_DELAY_US")
                .unwrap_or_else(|_| "0".to_string())
                .parse::<u64>()
                .unwrap_or(0);
            
            if delay_us > 0 {
                Arc::new(MockVerifier::new_with_delay(always_succeed, delay_us))
            } else if always_succeed {
                Arc::new(MockVerifier::new_always_succeed())
            } else {
                Arc::new(MockVerifier::new_always_fail())
            }
        }
        _ => {
            // 默认使用真实 verifier（测试模式）
            Arc::new(Groth16Verifier::new_for_testing()
                .expect("Failed to create Groth16Verifier for testing"))
        }
    }
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
    
    #[test]
    fn test_backend_enum() {
        let verifier = Groth16Verifier::new_for_testing().unwrap();
        assert_eq!(verifier.backend(), ZkBackend::Groth16Bls12_381);
        assert_eq!(verifier.backend().as_str(), "groth16-bls12-381");
    }
    
    #[test]
    fn test_backend_enum_values() {
        assert_eq!(ZkBackend::Groth16Bls12_381.as_str(), "groth16-bls12-381");
        assert_eq!(ZkBackend::Plonk.as_str(), "plonk");
        assert_eq!(ZkBackend::Mock.as_str(), "mock");
    }
    
    #[test]
    fn test_mock_verifier_always_succeed() {
        let verifier = MockVerifier::new_always_succeed();
        let result = verifier.verify(&[], &[]).unwrap();
        assert!(result, "Mock verifier should always succeed");
        assert_eq!(verifier.verifier_type(), "Mock");
        assert_eq!(verifier.backend(), ZkBackend::Mock);
    }
    
    #[test]
    fn test_mock_verifier_always_fail() {
        let verifier = MockVerifier::new_always_fail();
        let result = verifier.verify(&[], &[]).unwrap();
        assert!(!result, "Mock verifier should always fail");
    }
    
    #[test]
    fn test_mock_verifier_with_delay() {
        let verifier = MockVerifier::new_with_delay(true, 1000); // 1ms delay
        let start = std::time::Instant::now();
        let result = verifier.verify(&[], &[]).unwrap();
        let elapsed = start.elapsed();
        
        assert!(result, "Mock verifier should succeed");
        assert!(elapsed.as_micros() >= 1000, "Should have delay of at least 1ms");
    }
    
    #[test]
    fn test_mock_verifier_call_count() {
        let verifier = MockVerifier::new_always_succeed();
        assert_eq!(verifier.call_count(), 0);
        
        verifier.verify(&[], &[]).unwrap();
        assert_eq!(verifier.call_count(), 1);
        
        verifier.verify(&[], &[]).unwrap();
        verifier.verify(&[], &[]).unwrap();
        assert_eq!(verifier.call_count(), 3);
    }
    
    #[test]
    fn test_create_verifier_from_env_mock() {
        // 设置为 mock 模式
        std::env::set_var("ZK_VERIFIER_MODE", "mock");
        std::env::set_var("ZK_MOCK_ALWAYS_SUCCEED", "true");
        
        let verifier = create_verifier_from_env();
        assert_eq!(verifier.verifier_type(), "Mock");
        assert_eq!(verifier.backend(), ZkBackend::Mock);
        
        let result = verifier.verify(&[], &[]).unwrap();
        assert!(result, "Mock verifier should succeed");
        
        // 清理环境变量
        std::env::remove_var("ZK_VERIFIER_MODE");
        std::env::remove_var("ZK_MOCK_ALWAYS_SUCCEED");
    }
    
    #[test]
    fn test_create_verifier_from_env_mock_fail() {
        std::env::set_var("ZK_VERIFIER_MODE", "mock");
        std::env::set_var("ZK_MOCK_ALWAYS_SUCCEED", "false");
        
        let verifier = create_verifier_from_env();
        let result = verifier.verify(&[], &[]).unwrap();
        assert!(!result, "Mock verifier should fail");
        
        std::env::remove_var("ZK_VERIFIER_MODE");
        std::env::remove_var("ZK_MOCK_ALWAYS_SUCCEED");
    }
    
    #[test]
    fn test_create_verifier_from_env_real() {
        // 默认或显式设置为 real
        std::env::set_var("ZK_VERIFIER_MODE", "real");
        
        let verifier = create_verifier_from_env();
        assert_eq!(verifier.verifier_type(), "Groth16-BLS12-381");
        assert_eq!(verifier.backend(), ZkBackend::Groth16Bls12_381);
        
        std::env::remove_var("ZK_VERIFIER_MODE");
    }
}
