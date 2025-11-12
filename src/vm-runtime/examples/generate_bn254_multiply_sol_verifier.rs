// SPDX-License-Identifier: GPL-3.0-or-later
// 使用 BN254 曲线生成最小乘法电路的 Solidity 验证器（EVM 预编译兼容）
// 仅在启用 `groth16-verifier` 特性时可用；否则提供占位 main 防止编译失败。

#[cfg(feature = "groth16-verifier")]
mod real {
    use ark_bn254::{Bn254, Fr as Fp};
    use ark_groth16::Groth16;
    use ark_relations::r1cs::{ConstraintSynthesizer, ConstraintSystemRef, LinearCombination, SynthesisError};
    use ark_snark::SNARK;
    use rand::rngs::OsRng;
    use vm_runtime::privacy::solidity_verifier::SolidityVerifierGenerator;

    #[derive(Clone)]
    pub struct MultiplyCircuitBn254 { pub a: Option<Fp>, pub b: Option<Fp> }

    impl ConstraintSynthesizer<Fp> for MultiplyCircuitBn254 {
        fn generate_constraints(self, cs: ConstraintSystemRef<Fp>) -> Result<(), SynthesisError> {
            let a = cs.new_witness_variable(|| self.a.ok_or(SynthesisError::AssignmentMissing))?;
            let b = cs.new_witness_variable(|| self.b.ok_or(SynthesisError::AssignmentMissing))?;
            let c = cs.new_input_variable(|| {
                let a = self.a.ok_or(SynthesisError::AssignmentMissing)?;
                let b = self.b.ok_or(SynthesisError::AssignmentMissing)?;
                Ok(a * b)
            })?;
            cs.enforce_constraint(LinearCombination::from(a), LinearCombination::from(b), LinearCombination::from(c))?;
            Ok(())
        }
    }

    pub fn run() {
        println!("=== BN254 Solidity Verifier Generator (Multiply) ===\n");
        let mut rng = OsRng;
        let circuit = MultiplyCircuitBn254 { a: None, b: None };
        let params = Groth16::<Bn254>::generate_random_parameters_with_reduction(circuit, &mut rng).expect("setup");
        let num_public_inputs = 1usize; // c = a*b
        let gen = SolidityVerifierGenerator::new("BN254MultiplyVerifier");
        std::fs::create_dir_all("contracts").ok();
        let out = "contracts/BN254MultiplyVerifier.sol";
        gen.save_to_file_bn(&params.vk, num_public_inputs, out).expect("save");
        let code = gen.generate_bn254(&params.vk, num_public_inputs);
        println!("saved: {} ({} bytes)", out, code.len());
    }
}

#[cfg(feature = "groth16-verifier")]
fn main() { real::run(); }

#[cfg(not(feature = "groth16-verifier"))]
fn main() {
    eprintln!("[generate_bn254_multiply_sol_verifier] feature 'groth16-verifier' 未启用，示例被跳过。");
}
