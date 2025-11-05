// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2025 XujueKing <leadbrand@me.com>

//! Groth16 zkSNARK 实践测试（arkworks + BLS12-381）
//!
//! 目标：最小可行电路（a*b=c）端到端：Trusted Setup → 生成证明 → 验证证明。

use ark_bls12_381::Fr;
use ark_relations::r1cs::{ConstraintSynthesizer, ConstraintSystemRef, LinearCombination, SynthesisError};

#[derive(Clone)]
pub struct MultiplyCircuit {
    pub a: Option<Fr>, // 私有输入
    pub b: Option<Fr>, // 私有输入
}

impl ConstraintSynthesizer<Fr> for MultiplyCircuit {
    fn generate_constraints(self, cs: ConstraintSystemRef<Fr>) -> Result<(), SynthesisError> {
        // 分配私有输入 a, b
        let a = cs.new_witness_variable(|| self.a.ok_or(SynthesisError::AssignmentMissing))?;
        let b = cs.new_witness_variable(|| self.b.ok_or(SynthesisError::AssignmentMissing))?;

        // 分配公开输入 c = a*b
        let c = cs.new_input_variable(|| {
            let a = self.a.ok_or(SynthesisError::AssignmentMissing)?;
            let b = self.b.ok_or(SynthesisError::AssignmentMissing)?;
            Ok(a * b)
        })?;

        // 约束：a * b = c
        cs.enforce_constraint(LinearCombination::from(a), LinearCombination::from(b), LinearCombination::from(c))?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ark_bls12_381::Bls12_381;
    use ark_groth16::{Groth16, prepare_verifying_key};
    use ark_snark::SNARK;
    use rand::rngs::OsRng;

    #[test]
    fn test_multiply_circuit_end_to_end() {
        // 1) Trusted Setup（无见证电路）
        let rng = &mut OsRng;
        let params = {
            let circuit = MultiplyCircuit { a: None, b: None };
            Groth16::<Bls12_381>::generate_random_parameters_with_reduction(circuit, rng)
                .expect("setup failed")
        };

        // 2) 生成证明：证明知道 a=3, b=5 使得 c=15
        let a = Fr::from(3u64);
        let b = Fr::from(5u64);
        let c = a * b;

        let proof = Groth16::<Bls12_381>::prove(&params, MultiplyCircuit { a: Some(a), b: Some(b) }, rng)
            .expect("proving failed");

        // 3) 验证证明：正确公开输入
        let pvk = prepare_verifying_key(&params.vk);
        assert!(Groth16::<Bls12_381>::verify_proof(&pvk, &proof, &[c]).unwrap(), "should verify");

        // 4) 验证证明：错误公开输入
    let wrong = Fr::from(999u64);
    assert!(!Groth16::<Bls12_381>::verify_proof(&pvk, &proof, &[wrong]).unwrap(), "should not verify");
    }
}

// 导出电路模块
pub mod range_proof;
pub mod range_proof_aggregated;
pub mod ringct;
pub mod ringct_compressed;
pub mod ringct_multi_utxo;
