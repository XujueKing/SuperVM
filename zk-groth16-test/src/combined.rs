// Pedersen + Range 组合电路：隐藏金额的完整范围证明
// 
// 公开输入：承诺 C
// 私有输入：金额 v ∈ [0, 2^64-1]，盲化因子 r
// 约束：
//   1. C = v + r*k  （Pedersen 简化承诺）
//   2. 0 ≤ v < 2^64 （64-bit 范围证明）

use ark_ff::PrimeField;
use ark_relations::r1cs::{
    ConstraintSynthesizer, ConstraintSystemRef, LinearCombination, SynthesisError, Variable,
};

/// 组合电路：Pedersen 承诺 + 64-bit 范围证明
/// 
/// 这是隐私交易中隐藏金额范围证明的核心组件
#[derive(Clone)]
pub struct CombinedCircuit<F: PrimeField> {
    /// 金额（私有）
    pub value: Option<u64>,
    /// 盲化因子（私有）
    pub blinding: Option<F>,
    /// 承诺常数 k（公开参数，嵌入电路）
    pub k: u64,
}

impl<F: PrimeField> CombinedCircuit<F> {
    /// 创建新的组合电路实例
    /// 
    /// # 参数
    /// * `value` - 金额 v ∈ [0, 2^64-1]（None 用于 setup）
    /// * `blinding` - 盲化因子 r（None 用于 setup）
    /// * `k` - 承诺常数（公开参数）
    pub fn new(value: Option<u64>, blinding: Option<F>, k: u64) -> Self {
        Self {
            value,
            blinding,
            k,
        }
    }

    /// 计算承诺 C = v + r*k
    pub fn compute_commitment(&self) -> Option<F> {
        match (self.value, &self.blinding) {
            (Some(v), Some(r)) => {
                let v_field = F::from(v);
                let k_field = F::from(self.k);
                let rk = *r * k_field;
                Some(v_field + rk)
            }
            _ => None,
        }
    }
}

impl<F: PrimeField> ConstraintSynthesizer<F> for CombinedCircuit<F> {
    fn generate_constraints(self, cs: ConstraintSystemRef<F>) -> Result<(), SynthesisError> {
        // ========== 第一部分：Pedersen 承诺约束 ==========
        
        // 1. 分配私有输入：金额 v
        let v_var = cs.new_witness_variable(|| {
            self.value
                .map(F::from)
                .ok_or(SynthesisError::AssignmentMissing)
        })?;

        // 2. 分配私有输入：盲化因子 r
        let r_var = cs.new_witness_variable(|| {
            self.blinding.ok_or(SynthesisError::AssignmentMissing)
        })?;

        // 3. 计算 r * k
        let k_value = F::from(self.k);
        let rk = match &self.blinding {
            Some(r) => Some(*r * k_value),
            None => None,
        };
        let rk_var = cs.new_witness_variable(|| {
            rk.ok_or(SynthesisError::AssignmentMissing)
        })?;

        // 约束：rk = r * k
        cs.enforce_constraint(
            LinearCombination::from(r_var),
            LinearCombination::zero() + (k_value, Variable::One),
            LinearCombination::from(rk_var),
        )?;

        // 4. 计算承诺 C = v + r*k
        let c_value = self.compute_commitment();
        let c_var = cs.new_input_variable(|| {
            c_value.ok_or(SynthesisError::AssignmentMissing)
        })?;

        // 约束：C = v + rk
        cs.enforce_constraint(
            LinearCombination::from(Variable::One),
            LinearCombination::from(v_var) + rk_var,
            LinearCombination::from(c_var),
        )?;

        // ========== 第二部分：64-bit 范围证明约束 ==========
        
        // 将金额 v 分解为 64 个比特
        let value_u64 = self.value.unwrap_or(0);
        
        // 为每个比特创建变量
        let mut bit_vars = Vec::with_capacity(64);
        for i in 0..64 {
            let bit_value = ((value_u64 >> i) & 1) == 1;
            let bit_var = cs.new_witness_variable(|| {
                Ok(if bit_value { F::one() } else { F::zero() })
            })?;
            bit_vars.push(bit_var);
        }

        // 布尔约束：每个 bit b_i 必须满足 b_i * (b_i - 1) = 0
        for &bit_var in &bit_vars {
            cs.enforce_constraint(
                LinearCombination::from(bit_var),
                LinearCombination::from(bit_var) - LinearCombination::from(Variable::One),
                LinearCombination::zero(),
            )?;
        }

        // 位求和约束：v = Σ(b_i * 2^i)
        let mut sum_lc = LinearCombination::zero();
        for (i, &bit_var) in bit_vars.iter().enumerate() {
            let coeff = F::from(1u64 << i);
            sum_lc = sum_lc + (coeff, bit_var);
        }

        // 约束：sum = v
        cs.enforce_constraint(
            LinearCombination::from(Variable::One),
            sum_lc,
            LinearCombination::from(v_var),
        )?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ark_bls12_381::{Bls12_381, Fr};
    use ark_groth16::{Groth16, prepare_verifying_key};
    use ark_snark::SNARK;
    use rand::rngs::OsRng;

    #[test]
    fn test_combined_circuit_small_value() {
        let rng = &mut OsRng;
        let k = 7u64;

        println!("\n=== Pedersen + Range 组合电路测试（小金额）===");
        
        // 1. Trusted Setup
        println!("1. 执行 Trusted Setup...");
        let params = Groth16::<Bls12_381>::generate_random_parameters_with_reduction(
            CombinedCircuit::new(None, None, k),
            rng,
        )
        .expect("setup failed");
        println!("   ✓ Setup 完成");

        // 2. 生成证明（v=100, r=42）
        let v = 100u64;
        let r = Fr::from(42u64);
        println!("\n2. 生成证明（v={}, r=42, k={}）...", v, k);
        
        let circuit = CombinedCircuit::new(Some(v), Some(r), k);
        let c = circuit.compute_commitment().unwrap();
        println!("   承诺 C = v + r*k = {} + 42*{} = {}", v, k, c);

        let proof = Groth16::<Bls12_381>::prove(&params, circuit, rng)
            .expect("proving failed");
        println!("   ✓ 证明生成成功");

        // 3. 验证证明
        println!("3. 验证证明...");
        let pvk = prepare_verifying_key(&params.vk);
        assert!(
            Groth16::<Bls12_381>::verify_proof(&pvk, &proof, &[c]).unwrap(),
            "proof should verify"
        );
        println!("   ✓ 证明验证通过!");

        // 4. 测试错误承诺
        println!("4. 测试错误承诺...");
        let wrong_c = Fr::from(999u64);
        assert!(
            !Groth16::<Bls12_381>::verify_proof(&pvk, &proof, &[wrong_c]).unwrap(),
            "wrong commitment should fail"
        );
        println!("   ✓ 错误承诺被正确拒绝!");
    }

    #[test]
    fn test_combined_circuit_large_value() {
        let rng = &mut OsRng;
        let k = 7u64;

        println!("\n=== Pedersen + Range 组合电路测试（大金额）===");
        
        // 1. Trusted Setup
        println!("1. 执行 Trusted Setup...");
        let params = Groth16::<Bls12_381>::generate_random_parameters_with_reduction(
            CombinedCircuit::new(None, None, k),
            rng,
        )
        .expect("setup failed");
        println!("   ✓ Setup 完成");

        // 2. 生成证明（真实场景大金额）
        let v = 123456789012345u64; // ~123 万亿
        let r = Fr::from(987654321u64);
        println!("\n2. 生成证明（v={}，真实场景大金额）...", v);
        
        let circuit = CombinedCircuit::new(Some(v), Some(r), k);
        let c = circuit.compute_commitment().unwrap();

        let proof = Groth16::<Bls12_381>::prove(&params, circuit, rng)
            .expect("proving failed");
        println!("   ✓ 证明生成成功");

        // 3. 验证证明
        println!("3. 验证证明...");
        let pvk = prepare_verifying_key(&params.vk);
        assert!(
            Groth16::<Bls12_381>::verify_proof(&pvk, &proof, &[c]).unwrap(),
            "proof should verify"
        );
        println!("   ✓ 大金额证明验证通过!");
    }

    #[test]
    fn test_combined_circuit_edge_cases() {
        let rng = &mut OsRng;
        let k = 7u64;

        println!("\n=== Pedersen + Range 组合电路边界测试 ===");
        
        let params = Groth16::<Bls12_381>::generate_random_parameters_with_reduction(
            CombinedCircuit::new(None, None, k),
            rng,
        )
        .expect("setup failed");

        let pvk = prepare_verifying_key(&params.vk);

        // 测试 1: v=0（最小值）
        println!("\n测试 1: v=0（最小合法金额）");
        let circuit = CombinedCircuit::new(Some(0), Some(Fr::from(1u64)), k);
        let c = circuit.compute_commitment().unwrap();
        let proof = Groth16::<Bls12_381>::prove(&params, circuit, rng).unwrap();
        assert!(Groth16::<Bls12_381>::verify_proof(&pvk, &proof, &[c]).unwrap());
        println!("   ✓ v=0 测试通过");

        // 测试 2: v=2^32-1（32位最大值）
        println!("\n测试 2: v=2^32-1（32位最大值）");
        let v_32bit = u32::MAX as u64;
        let circuit = CombinedCircuit::new(Some(v_32bit), Some(Fr::from(2u64)), k);
        let c = circuit.compute_commitment().unwrap();
        let proof = Groth16::<Bls12_381>::prove(&params, circuit, rng).unwrap();
        assert!(Groth16::<Bls12_381>::verify_proof(&pvk, &proof, &[c]).unwrap());
        println!("   ✓ v=2^32-1 测试通过");

        // 测试 3: v=2^63-1（接近最大值）
        println!("\n测试 3: v=2^63-1（接近 64-bit 最大值）");
        let v_near_max = (1u64 << 63) - 1;
        let circuit = CombinedCircuit::new(Some(v_near_max), Some(Fr::from(3u64)), k);
        let c = circuit.compute_commitment().unwrap();
        let proof = Groth16::<Bls12_381>::prove(&params, circuit, rng).unwrap();
        assert!(Groth16::<Bls12_381>::verify_proof(&pvk, &proof, &[c]).unwrap());
        println!("   ✓ v=2^63-1 测试通过");
    }
}
