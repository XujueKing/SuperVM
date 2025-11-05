use ark_bls12_381::Fr;
use ark_relations::r1cs::{ConstraintSynthesizer, ConstraintSystemRef, LinearCombination, SynthesisError, Variable};

/// 简单位分解 Range 证明电路：
/// 证明者持有私有值 v，公开输入 c，应满足：
/// 1) v = c
/// 2) v 在 [0, 2^n_bits) 范围内（通过二进制位分解与布尔约束保证）
#[derive(Clone)]
pub struct RangeProofCircuit {
    pub value: Option<Fr>,
    pub value_u64: Option<u64>,
    pub n_bits: usize,
}

impl RangeProofCircuit {
    pub fn new(value: Option<u64>, n_bits: usize) -> Self {
        let value_fr = value.map(|v| Fr::from(v as u64));
        Self { value: value_fr, value_u64: value, n_bits }
    }
}

impl ConstraintSynthesizer<Fr> for RangeProofCircuit {
    fn generate_constraints(self, cs: ConstraintSystemRef<Fr>) -> Result<(), SynthesisError> {
        // 见证 v
        let v = cs.new_witness_variable(|| self.value.ok_or(SynthesisError::AssignmentMissing))?;

        // 公开输入 c = v
        let c = cs.new_input_variable(|| self.value.ok_or(SynthesisError::AssignmentMissing))?;

        // 约束 v = c: (v - c) * 1 = 0
        cs.enforce_constraint(
            LinearCombination::from(v) - LinearCombination::from(c),
            LinearCombination::from(Variable::One),
            LinearCombination::zero(),
        )?;

        // 位分解：v = sum_{i=0}^{n_bits-1} b_i * 2^i，且每个 b_i ∈ {0,1}
        let mut sum_lc = LinearCombination::<Fr>::zero();
        let mut coeff = Fr::from(1u64);

        for i in 0..self.n_bits {
            let bit_val = self
                .value_u64
                .map(|v| ((v >> i) & 1) as u64)
                .unwrap_or(0u64);

            let bit = cs.new_witness_variable(|| Ok(Fr::from(bit_val)))?; // 见证位按值分配

            // 布尔约束：bit * (bit - 1) = 0
            cs.enforce_constraint(
                LinearCombination::from(bit),
                LinearCombination::from(bit) - LinearCombination::from(Variable::One),
                LinearCombination::zero(),
            )?;

            // 累加 sum += bit * coeff
            sum_lc = sum_lc + (coeff, bit);

            // coeff <<= 1
            coeff += coeff;
        }

        // 约束：sum == v => (sum - v) * 1 = 0
        cs.enforce_constraint(
            sum_lc - LinearCombination::from(v),
            LinearCombination::from(Variable::One),
            LinearCombination::zero(),
        )?;

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
    fn test_range_proof_8_bits() {
        let rng = &mut OsRng;
        let params = Groth16::<Bls12_381>::generate_random_parameters_with_reduction(
            RangeProofCircuit::new(None, 8),
            rng,
        ).expect("setup failed");

        // v = 42 < 2^8
        let proof = Groth16::<Bls12_381>::prove(&params, RangeProofCircuit::new(Some(42), 8), rng)
            .expect("proving failed");
        let pvk = prepare_verifying_key(&params.vk);

        // 公开输入 c = 42
        let c = Fr::from(42u64);
        assert!(Groth16::<Bls12_381>::verify_proof(&pvk, &proof, &[c]).unwrap());

        // 错误公开输入
        let wrong_c = Fr::from(255u64);
        assert!(!Groth16::<Bls12_381>::verify_proof(&pvk, &proof, &[wrong_c]).unwrap());
    }

    #[test]
    fn test_range_proof_64_bits() {
        let rng = &mut OsRng;
        
        println!("\n=== 64-bit 范围证明测试 ===");
        println!("1. 执行 Trusted Setup (64 个约束)...");
        let params = Groth16::<Bls12_381>::generate_random_parameters_with_reduction(
            RangeProofCircuit::new(None, 64),
            rng,
        ).expect("setup failed");
        println!("   ✓ Setup 完成");

        // v = 12345678901234 < 2^64
        let test_value = 12345678901234u64;
        println!("\n2. 生成证明 (v={} < 2^64)...", test_value);
        let proof = Groth16::<Bls12_381>::prove(
            &params,
            RangeProofCircuit::new(Some(test_value), 64),
            rng,
        ).expect("proving failed");
        println!("   ✓ 证明生成成功");

        // 验证证明
        println!("3. 验证证明...");
        let pvk = prepare_verifying_key(&params.vk);
        let c = Fr::from(test_value);
        assert!(
            Groth16::<Bls12_381>::verify_proof(&pvk, &proof, &[c]).unwrap(),
            "proof should verify"
        );
        println!("   ✓ 证明验证通过!");

        // 错误公开输入
        println!("4. 测试错误公开输入...");
        let wrong_c = Fr::from(999999999999u64);
        assert!(
            !Groth16::<Bls12_381>::verify_proof(&pvk, &proof, &[wrong_c]).unwrap(),
            "wrong value should fail"
        );
        println!("   ✓ 错误证明被正确拒绝!");
    }
}
