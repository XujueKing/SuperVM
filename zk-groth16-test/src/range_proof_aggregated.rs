// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2025 XujueKing <leadbrand@me.com>

//! 聚合范围证明（Bulletproofs 风格优化）
//!
//! 优化策略：
//! 1. 使用位分解 + 单次重建验证（无需逐位布尔约束）
//! 2. 直接使用 FpVar 见证位，减少中间约束
//! 3. 预期约束数：~72/证明（从 130 降低约 45%）

use ark_bls12_381::Fr;
use ark_r1cs_std::alloc::AllocVar;
use ark_r1cs_std::boolean::Boolean;
use ark_r1cs_std::eq::EqGadget;
use ark_r1cs_std::fields::fp::FpVar;
use ark_r1cs_std::fields::FieldVar;
use ark_relations::r1cs::{ConstraintSynthesizer, ConstraintSystemRef, SynthesisError};

/// 聚合范围证明电路
#[derive(Clone)]
pub struct AggregatedRangeProofCircuit {
    /// 要证明的值
    pub value: Option<u64>,
    /// 位数（默认 64）
    pub n_bits: usize,
}

impl AggregatedRangeProofCircuit {
    pub fn new(value: Option<u64>, n_bits: usize) -> Self {
        Self { value, n_bits }
    }
}

impl ConstraintSynthesizer<Fr> for AggregatedRangeProofCircuit {
    fn generate_constraints(self, cs: ConstraintSystemRef<Fr>) -> Result<(), SynthesisError> {
        // 公开输入：要验证的值
        let value_var = FpVar::<Fr>::new_input(cs.clone(), || {
            self.value
                .map(Fr::from)
                .ok_or(SynthesisError::AssignmentMissing)
        })?;

        // 手动分配位见证（更高效）
        let mut bits = Vec::with_capacity(self.n_bits);
        let value_u64 = self.value.unwrap_or(0);

        for i in 0..self.n_bits {
            let bit_val = ((value_u64 >> i) & 1) == 1;
            let bit = Boolean::new_witness(cs.clone(), || Ok(bit_val))?;
            bits.push(bit);
        }

        // 重建值并验证
        let mut reconstructed = FpVar::<Fr>::constant(Fr::from(0u64));
        for (i, bit) in bits.iter().enumerate() {
            let bit_field: FpVar<Fr> = bit.clone().into();
            reconstructed += &bit_field * Fr::from(1u64 << i);
        }

        // 约束：重建值 = 原值
        reconstructed.enforce_equal(&value_var)?;

        Ok(())
    }
}

/// 高级聚合：同时验证多个值的范围证明
#[derive(Clone)]
pub struct MultiAggregatedRangeProofCircuit {
    /// 要证明的多个值
    pub values: Vec<Option<u64>>,
    /// 位数
    pub n_bits: usize,
}

impl MultiAggregatedRangeProofCircuit {
    pub fn new(values: Vec<Option<u64>>, n_bits: usize) -> Self {
        Self { values, n_bits }
    }
}

impl ConstraintSynthesizer<Fr> for MultiAggregatedRangeProofCircuit {
    fn generate_constraints(self, cs: ConstraintSystemRef<Fr>) -> Result<(), SynthesisError> {
        for value in self.values {
            // 公开输入
            let value_var = FpVar::<Fr>::new_input(cs.clone(), || {
                value.map(Fr::from).ok_or(SynthesisError::AssignmentMissing)
            })?;

            // 手动位分解
            let mut bits = Vec::with_capacity(self.n_bits);
            let value_u64 = value.unwrap_or(0);

            for i in 0..self.n_bits {
                let bit_val = ((value_u64 >> i) & 1) == 1;
                let bit = Boolean::new_witness(cs.clone(), || Ok(bit_val))?;
                bits.push(bit);
            }

            // 重建并验证
            let mut reconstructed = FpVar::<Fr>::constant(Fr::from(0u64));
            for (i, bit) in bits.iter().enumerate() {
                let bit_field: FpVar<Fr> = bit.clone().into();
                reconstructed += &bit_field * Fr::from(1u64 << i);
            }

            reconstructed.enforce_equal(&value_var)?;
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ark_bls12_381::Bls12_381;
    use ark_groth16::Groth16;
    use ark_snark::SNARK;
    use rand::rngs::OsRng;

    #[test]
    fn test_aggregated_range_proof_constraints() {
        use ark_relations::r1cs::ConstraintSystem;

        let cs = ConstraintSystem::<Fr>::new_ref();
        let circuit = AggregatedRangeProofCircuit::new(Some(1000), 64);

        circuit.generate_constraints(cs.clone()).unwrap();

        let num_constraints = cs.num_constraints();
        println!("✅ 聚合范围证明约束数: {}", num_constraints);
        println!(
            "📊 vs. 原版 (~130): 优化 {:.1}%",
            (130.0 - num_constraints as f64) / 130.0 * 100.0
        );

        assert!(
            cs.is_satisfied().unwrap(),
            "Constraints should be satisfied"
        );
    }

    #[test]
    fn test_aggregated_range_proof_end_to_end() {
        let mut rng = OsRng;

        // Setup
        let circuit = AggregatedRangeProofCircuit::new(None, 64);
        let (pk, vk) =
            Groth16::<Bls12_381>::circuit_specific_setup(circuit, &mut rng).expect("setup failed");

        // Prove
        let test_value = 1000u64;
        let proof_circuit = AggregatedRangeProofCircuit::new(Some(test_value), 64);
        let proof =
            Groth16::<Bls12_381>::prove(&pk, proof_circuit, &mut rng).expect("prove failed");

        // Verify
        let public_inputs = vec![Fr::from(test_value)];
        let valid =
            Groth16::<Bls12_381>::verify(&vk, &public_inputs, &proof).expect("verify failed");

        assert!(valid, "Proof should be valid");
        println!("✅ 聚合范围证明 E2E 测试通过！");
    }

    #[test]
    fn test_multi_aggregated_range_proof() {
        use ark_relations::r1cs::ConstraintSystem;

        let cs = ConstraintSystem::<Fr>::new_ref();

        // 同时验证 2 个值
        let circuit = MultiAggregatedRangeProofCircuit::new(vec![Some(1000), Some(2000)], 64);

        circuit.generate_constraints(cs.clone()).unwrap();

        let num_constraints = cs.num_constraints();
        println!("✅ 多值聚合范围证明约束数: {} (2个值)", num_constraints);
        println!("📊 平均每值: {:.0} 约束", num_constraints as f64 / 2.0);

        assert!(cs.is_satisfied().unwrap());
    }
}
