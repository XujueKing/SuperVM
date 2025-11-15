use ark_bls12_381::Fr;
use ark_relations::r1cs::{
    ConstraintSynthesizer, ConstraintSystemRef, LinearCombination, SynthesisError, Variable,
};

/// Pedersen 承诺电路：
/// 公开：承诺 C（作为公开输入）
/// 私有：金额 v，盲化因子 r
/// 约束：C = v*H + r*G （这里用简化版：C = v + r*k，k 为公开参数）
///
/// 注：完整 Pedersen 需要椭圆曲线群运算；这里用域乘法模拟线性承诺
#[derive(Clone)]
pub struct PedersenCommitmentCircuit {
    pub value: Option<Fr>,    // 私有：金额 v
    pub blinding: Option<Fr>, // 私有：盲化因子 r
    pub commitment_param: Fr, // 公开参数 k（模拟 H/G 比例）
}

impl PedersenCommitmentCircuit {
    pub fn new(value: Option<u64>, blinding: Option<u64>, commitment_param: u64) -> Self {
        Self {
            value: value.map(Fr::from),
            blinding: blinding.map(Fr::from),
            commitment_param: Fr::from(commitment_param),
        }
    }
}

impl ConstraintSynthesizer<Fr> for PedersenCommitmentCircuit {
    fn generate_constraints(self, cs: ConstraintSystemRef<Fr>) -> Result<(), SynthesisError> {
        // 见证私有输入 v, r
        let v = cs.new_witness_variable(|| self.value.ok_or(SynthesisError::AssignmentMissing))?;
        let r =
            cs.new_witness_variable(|| self.blinding.ok_or(SynthesisError::AssignmentMissing))?;

        // 公开输入：承诺 C = v + r*k
        let commitment = cs.new_input_variable(|| {
            let v_val = self.value.ok_or(SynthesisError::AssignmentMissing)?;
            let r_val = self.blinding.ok_or(SynthesisError::AssignmentMissing)?;
            Ok(v_val + r_val * self.commitment_param)
        })?;

        // 约束：r * k = r_times_k
        let r_times_k = cs.new_witness_variable(|| {
            let r_val = self.blinding.ok_or(SynthesisError::AssignmentMissing)?;
            Ok(r_val * self.commitment_param)
        })?;

        // 使用 LinearCombination 内联常数系数
        cs.enforce_constraint(
            LinearCombination::from(r),
            LinearCombination::zero() + (self.commitment_param, Variable::One),
            LinearCombination::from(r_times_k),
        )?;

        // 约束：v + r_times_k = commitment
        // => (v + r_times_k - commitment) * 1 = 0
        cs.enforce_constraint(
            LinearCombination::from(v) + LinearCombination::from(r_times_k)
                - LinearCombination::from(commitment),
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
    use ark_groth16::{prepare_verifying_key, Groth16};
    use ark_snark::SNARK;
    use rand::rngs::OsRng;

    #[test]
    fn test_pedersen_commitment() {
        let rng = &mut OsRng;
        let k = 7u64; // 公开参数

        // Setup（无见证）
        let params = Groth16::<Bls12_381>::generate_random_parameters_with_reduction(
            PedersenCommitmentCircuit::new(None, None, k),
            rng,
        )
        .expect("setup failed");

        // 生成证明：v=100, r=42 => C=100+42*7=394
        let v = 100u64;
        let r = 42u64;
        let c = v + r * k;

        let proof = Groth16::<Bls12_381>::prove(
            &params,
            PedersenCommitmentCircuit::new(Some(v), Some(r), k),
            rng,
        )
        .expect("proving failed");

        // 验证证明（正确承诺）
        let pvk = prepare_verifying_key(&params.vk);
        let c_fr = Fr::from(c);
        assert!(
            Groth16::<Bls12_381>::verify_proof(&pvk, &proof, &[c_fr]).unwrap(),
            "proof should verify"
        );

        // 验证证明（错误承诺）
        let wrong_c = Fr::from(999u64);
        assert!(
            !Groth16::<Bls12_381>::verify_proof(&pvk, &proof, &[wrong_c]).unwrap(),
            "wrong commitment should fail"
        );
    }
}
