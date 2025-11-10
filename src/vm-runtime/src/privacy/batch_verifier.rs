// SPDX-License-Identifier: GPL-3.0-or-later
// 批量验证模块 - Groth16 Batch Verification
// Phase 2.3: 优化验证性能,支持批量验证多个证明

use std::time::{Duration, Instant};
use std::sync::Arc;
use ark_bls12_381::Bls12_381;
use ark_groth16::{Groth16, VerifyingKey, Proof, PreparedVerifyingKey};
use ark_snark::SNARK;
use ark_serialize::{CanonicalSerialize, CanonicalDeserialize};
use crate::metrics::MetricsCollector;

/// 批量验证配置
#[derive(Clone, Debug)]
pub struct BatchVerifyConfig {
    /// 批量大小
    pub batch_size: usize,
    /// 是否使用预处理的 VerifyingKey
    pub use_prepared_vk: bool,
}

impl Default for BatchVerifyConfig {
    fn default() -> Self {
        Self {
            batch_size: 32,
            use_prepared_vk: true,
        }
    }
}

/// 批量验证统计
#[derive(Clone, Debug)]
pub struct BatchVerifyStats {
    pub total: usize,
    pub verified: usize,
    pub failed: usize,
    pub total_duration: Duration,
    pub avg_latency_ms: f64,
    pub verifications_per_sec: f64,
}

/// 批量验证器
pub struct BatchVerifier {
    vk: Arc<VerifyingKey<Bls12_381>>,
    prepared_vk: Option<Arc<PreparedVerifyingKey<Bls12_381>>>,
    #[allow(dead_code)] // 保留用于未来扩展
    config: BatchVerifyConfig,
    metrics: Option<Arc<MetricsCollector>>,
}

impl BatchVerifier {
    /// 创建新的批量验证器
    pub fn new(vk: VerifyingKey<Bls12_381>, config: BatchVerifyConfig) -> Self {
        let prepared_vk = if config.use_prepared_vk {
            Some(Arc::new(PreparedVerifyingKey::from(vk.clone())))
        } else {
            None
        };
        
        Self {
            vk: Arc::new(vk),
            prepared_vk,
            config,
            metrics: None,
        }
    }

    /// 添加 metrics 收集器
    pub fn with_metrics(mut self, metrics: Arc<MetricsCollector>) -> Self {
        self.metrics = Some(metrics);
        self
    }

    /// 逐个验证证明(基线方法)
    /// 
    /// 参数:
    /// - proofs: 证明列表
    /// - public_inputs: 对应的公共输入列表
    /// 
    /// 返回: 验证统计信息
    pub fn verify_individual<F>(
        &self,
        proofs: &[Proof<Bls12_381>],
        public_inputs: &[Vec<F>],
    ) -> BatchVerifyStats
    where
        F: CanonicalSerialize + CanonicalDeserialize + Clone,
    {
        assert_eq!(proofs.len(), public_inputs.len(), "Proofs and inputs length mismatch");

        let start = Instant::now();
        let mut verified = 0usize;
        let mut failed = 0usize;

        for (proof, inputs) in proofs.iter().zip(public_inputs.iter()) {
            // 序列化公共输入用于验证
            let mut serialized_inputs = Vec::new();
            for inp in inputs.iter() {
                inp.serialize_compressed(&mut serialized_inputs)
                    .expect("Failed to serialize input");
            }

            // 反序列化为 Fr
            use ark_bls12_381::Fr;
            let fr_inputs: Vec<Fr> = inputs
                .iter()
                .map(|inp| {
                    let mut buf = Vec::new();
                    inp.serialize_compressed(&mut buf).unwrap();
                    Fr::deserialize_compressed(&buf[..]).unwrap()
                })
                .collect();

            let result = if let Some(ref pvk) = self.prepared_vk {
                Groth16::<Bls12_381>::verify_with_processed_vk(pvk, &fr_inputs, proof)
            } else {
                Groth16::<Bls12_381>::verify(&self.vk, &fr_inputs, proof)
            };

            if result.unwrap_or(false) {
                verified += 1;
            } else {
                failed += 1;
            }
        }

        let total = verified + failed;
        let total_duration = start.elapsed();
        let avg_latency_ms = if total > 0 {
            (total_duration.as_secs_f64() * 1000.0) / total as f64
        } else {
            0.0
        };
        let verifications_per_sec = if total_duration.as_secs_f64() > 0.0 {
            verified as f64 / total_duration.as_secs_f64()
        } else {
            0.0
        };

        BatchVerifyStats {
            total,
            verified,
            failed,
            total_duration,
            avg_latency_ms,
            verifications_per_sec,
        }
    }

    /// 批量验证优化版本 (使用配对聚合)
    /// 
    /// 注意: Groth16 原生不支持批量验证,这里使用并行逐个验证模拟批量处理
    /// 未来可考虑使用 aggregated proofs (如 SnarkPack) 进一步优化
    /// 
    /// 参数:
    /// - proofs: 证明列表
    /// - public_inputs: 对应的公共输入列表
    /// 
    /// 返回: 验证统计信息
    pub fn verify_batch_optimized<F>(
        &self,
        proofs: &[Proof<Bls12_381>],
        public_inputs: &[Vec<F>],
    ) -> BatchVerifyStats
    where
        F: CanonicalSerialize + CanonicalDeserialize + Clone + Send + Sync,
    {
        assert_eq!(proofs.len(), public_inputs.len(), "Proofs and inputs length mismatch");

        let start = Instant::now();
        
        // 使用 Rayon 并行验证
        use rayon::prelude::*;
        use ark_bls12_381::Fr;

        let results: Vec<bool> = proofs
            .par_iter()
            .zip(public_inputs.par_iter())
            .map(|(proof, inputs)| {
                // 转换输入为 Fr
                let fr_inputs: Vec<Fr> = inputs
                    .iter()
                    .map(|inp| {
                        let mut buf = Vec::new();
                        inp.serialize_compressed(&mut buf).unwrap();
                        Fr::deserialize_compressed(&buf[..]).unwrap()
                    })
                    .collect();

                // 验证单个证明
                if let Some(ref pvk) = self.prepared_vk {
                    Groth16::<Bls12_381>::verify_with_processed_vk(pvk, &fr_inputs, proof)
                        .unwrap_or(false)
                } else {
                    Groth16::<Bls12_381>::verify(&self.vk, &fr_inputs, proof)
                        .unwrap_or(false)
                }
            })
            .collect();

        let verified = results.iter().filter(|&&v| v).count();
        let failed = results.len() - verified;
        let total = results.len();
        let total_duration = start.elapsed();

        let avg_latency_ms = if total > 0 {
            (total_duration.as_secs_f64() * 1000.0) / total as f64
        } else {
            0.0
        };
        let verifications_per_sec = if total_duration.as_secs_f64() > 0.0 {
            verified as f64 / total_duration.as_secs_f64()
        } else {
            0.0
        };

        let stats = BatchVerifyStats {
            total,
            verified,
            failed,
            total_duration,
            avg_latency_ms,
            verifications_per_sec,
        };

        // 记录 metrics：使用“批量验证”指标族（验证侧）
        if let Some(m) = &self.metrics {
            m.record_zk_batch_verify(
                stats.total as u64,
                stats.failed as u64,
                stats.total_duration.as_secs_f64() * 1000.0,
                stats.avg_latency_ms,
                stats.verifications_per_sec,
            );
        }

        stats
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ark_bls12_381::Fr;
    use ark_std::UniformRand;
    use rand::rngs::OsRng;
    use zk_groth16_test::MultiplyCircuit;

    #[test]
    fn test_batch_verify_individual() {
        let mut rng = OsRng;
        
        // 生成一个 setup
        let a = Fr::rand(&mut rng);
        let b = Fr::rand(&mut rng);
        let _c = a * b;
        
        let circuit = MultiplyCircuit { a: Some(a), b: Some(b) };
        let (pk, vk) = Groth16::<Bls12_381>::circuit_specific_setup(circuit.clone(), &mut rng)
            .expect("Setup failed");

        // 生成多个证明
        let num_proofs = 10;
        let mut proofs = Vec::new();
        let mut public_inputs = Vec::new();

        for _ in 0..num_proofs {
            let a = Fr::rand(&mut rng);
            let b = Fr::rand(&mut rng);
            let c = a * b;
            
            let circuit = MultiplyCircuit { a: Some(a), b: Some(b) };
            let proof = Groth16::<Bls12_381>::prove(&pk, circuit, &mut rng)
                .expect("Prove failed");
            
            proofs.push(proof);
            public_inputs.push(vec![c]);
        }

        // 批量验证
        let config = BatchVerifyConfig::default();
        let verifier = BatchVerifier::new(vk, config);
        let stats = verifier.verify_individual(&proofs, &public_inputs);

        println!("Individual verification: {:?}", stats);
        assert_eq!(stats.verified, num_proofs);
        assert_eq!(stats.failed, 0);
    }

    #[test]
    fn test_batch_verify_optimized() {
        let mut rng = OsRng;
        
        let a = Fr::rand(&mut rng);
        let b = Fr::rand(&mut rng);
        let circuit = MultiplyCircuit { a: Some(a), b: Some(b) };
        let (pk, vk) = Groth16::<Bls12_381>::circuit_specific_setup(circuit, &mut rng)
            .expect("Setup failed");

        let num_proofs = 32;
        let mut proofs = Vec::new();
        let mut public_inputs = Vec::new();

        for _ in 0..num_proofs {
            let a = Fr::rand(&mut rng);
            let b = Fr::rand(&mut rng);
            let c = a * b;
            
            let circuit = MultiplyCircuit { a: Some(a), b: Some(b) };
            let proof = Groth16::<Bls12_381>::prove(&pk, circuit, &mut rng)
                .expect("Prove failed");
            
            proofs.push(proof);
            public_inputs.push(vec![c]);
        }

        let config = BatchVerifyConfig {
            batch_size: 32,
            use_prepared_vk: true,
        };
        let verifier = BatchVerifier::new(vk, config);
        let stats = verifier.verify_batch_optimized(&proofs, &public_inputs);

        println!("Optimized verification: {:?}", stats);
        assert_eq!(stats.verified, num_proofs);
        assert_eq!(stats.failed, 0);
        assert!(stats.verifications_per_sec > 0.0);
    }

    #[test]
    fn test_batch_verify_with_invalid_proofs() {
        let mut rng = OsRng;
        
        let a = Fr::rand(&mut rng);
        let b = Fr::rand(&mut rng);
        let circuit = MultiplyCircuit { a: Some(a), b: Some(b) };
        let (pk, vk) = Groth16::<Bls12_381>::circuit_specific_setup(circuit, &mut rng)
            .expect("Setup failed");

        let num_proofs = 10;
        let mut proofs = Vec::new();
        let mut public_inputs = Vec::new();

        for i in 0..num_proofs {
            let a = Fr::rand(&mut rng);
            let b = Fr::rand(&mut rng);
            let c = a * b;
            
            let circuit = MultiplyCircuit { a: Some(a), b: Some(b) };
            let proof = Groth16::<Bls12_381>::prove(&pk, circuit, &mut rng)
                .expect("Prove failed");
            
            proofs.push(proof);
            
            // 故意让一半的公共输入错误
            if i % 2 == 0 {
                public_inputs.push(vec![c]);
            } else {
                public_inputs.push(vec![Fr::rand(&mut rng)]); // 错误的公共输入
            }
        }

        let config = BatchVerifyConfig::default();
        let verifier = BatchVerifier::new(vk, config);
        let stats = verifier.verify_batch_optimized(&proofs, &public_inputs);

        println!("Verification with invalid inputs: {:?}", stats);
        assert_eq!(stats.verified, 5); // 只有一半验证成功
        assert_eq!(stats.failed, 5);
    }
}
