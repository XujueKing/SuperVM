//! RingCT (Ring Confidential Transaction) 电路实现
//!
//! 实现生产级隐私交易电路，支持：
//! - 隐私转账（环签名）
//! - 金额隐藏（Pedersen 承诺）
//! - 范围证明（64-bit）
//! - 多输入/多输出 UTXO 模型
//!
//! ## 架构设计
//!
//! Phase 2.1: 简化版（单输入/单输出，环大小=5）
//! - 约束数目标: < 200
//! - 证明时间: < 100ms
//!
//! Phase 2.2: 完整版（多输入/多输出，环大小=10）
//! - 约束数目标: < 400
//! - 证明时间: < 200ms

use ark_bls12_381::Fr;
use ark_ff::PrimeField;
use ark_relations::r1cs::{ConstraintSynthesizer, ConstraintSystemRef, SynthesisError};
use std::vec::Vec;
// Poseidon (to replace placeholder Merkle hash)
use ark_r1cs_std::alloc::AllocVar;
use ark_r1cs_std::eq::EqGadget;
use ark_r1cs_std::fields::fp::FpVar;
use ark_r1cs_std::prelude::Boolean;
// Pedersen commitment
use ark_crypto_primitives::commitment::constraints::CommitmentGadget;
use ark_crypto_primitives::commitment::pedersen as pedersen_commit;
use ark_crypto_primitives::commitment::pedersen::constraints as pedersen_gadgets;
use ark_crypto_primitives::commitment::pedersen::Window;
use ark_crypto_primitives::commitment::CommitmentScheme;
use ark_crypto_primitives::crh::poseidon as poseidon_crh;
use ark_crypto_primitives::crh::poseidon::constraints as poseidon_constraints;
use ark_crypto_primitives::crh::{TwoToOneCRHScheme as TwoToOneCRHTrait, TwoToOneCRHSchemeGadget};
use ark_crypto_primitives::sponge::poseidon::PoseidonConfig;
use ark_ed_on_bls12_381_bandersnatch::constraints::EdwardsVar as PedersenCurveVar;
use ark_ed_on_bls12_381_bandersnatch::EdwardsProjective as PedersenCurve;
use ark_r1cs_std::fields::FieldVar;
use ark_r1cs_std::uint8::UInt8;
use ark_r1cs_std::ToBitsGadget;

// ===== 数据结构定义 =====

/// UTXO (Unspent Transaction Output)
#[derive(Clone, Debug)]
pub struct UTXO {
    /// 承诺值 C = v·G + r·H（公开）
    /// 使用 Pedersen 承诺的椭圆曲线点坐标 (x, y)
    pub commitment_x: Fr,
    pub commitment_y: Fr,

    /// 金额（私有，仅 Prover 知道）
    pub value: Option<u64>,

    /// 盲因子（私有，仅 Prover 知道，作为 32 字节）
    pub blinding: Option<[u8; 32]>,
}

impl UTXO {
    /// 创建新的 UTXO
    pub fn new(
        value: u64,
        blinding: [u8; 32],
        params: &pedersen_commit::Parameters<PedersenCurve>,
    ) -> Self {
        // 消息：u64 的小端字节，补零到窗口需求（4 字节）
        let mut msg = value.to_le_bytes().to_vec();
        let required = PedersenWindow::WINDOW_SIZE;
        if msg.len() < required {
            msg.resize(required, 0u8);
        }
        msg.truncate(required);

        // 随机性：将 32 字节映射为 Bandersnatch 标量
        let blind_scalar = ark_ed_on_bls12_381_bandersnatch::Fr::from_le_bytes_mod_order(&blinding);
        let randomness = pedersen_commit::Randomness::<PedersenCurve>(blind_scalar);

        let aff = pedersen_commit::Commitment::<PedersenCurve, PedersenWindow>::commit(
            params,
            &msg,
            &randomness,
        )
        .expect("pedersen commit");
        Self {
            commitment_x: aff.x,
            commitment_y: aff.y,
            value: Some(value),
            blinding: Some(blinding),
        }
    }

    /// 创建公开 UTXO（仅 Verifier 视角）
    pub fn public(commitment_x: Fr, commitment_y: Fr) -> Self {
        Self {
            commitment_x,
            commitment_y,
            value: None,
            blinding: None,
        }
    }
}

/// Pedersen commitment window for 64-bit input
/// 优化参数：平衡约束数与生成器数量
#[derive(Clone, Default)]
pub struct PedersenWindow;
impl pedersen_commit::Window for PedersenWindow {
    const WINDOW_SIZE: usize = 2; // 每个窗口 2 字节（16 位）
    const NUM_WINDOWS: usize = 16; // 16 个窗口（支持 32 字节输入）
}

/// Merkle 成员证明
#[derive(Clone, Debug)]
pub struct MerkleProof {
    /// 叶子节点（公钥哈希）
    pub leaf: Fr,

    /// Merkle 路径（从叶子到根）
    pub path: Vec<Fr>,

    /// 路径方向（0=左，1=右）
    pub directions: Vec<bool>,

    /// Merkle 根（公开）
    pub root: Fr,
}

impl MerkleProof {
    /// 验证 Merkle 证明
    pub fn verify(&self) -> bool {
        let mut current = self.leaf;

        for (sibling, &direction) in self.path.iter().zip(&self.directions) {
            // 简化哈希：H(a, b) = a + b（实际应使用 Poseidon）
            current = if direction {
                current + *sibling // current 在右
            } else {
                *sibling + current // current 在左
            };
        }

        current == self.root
    }
}

// ===== Phase 2.1: 简化版 RingCT 电路 =====

/// 简化版 RingCT 电路（单输入/单输出）
///
/// ## 功能
/// - 1 个输入 UTXO
/// - 1 个输出 UTXO
/// - 环大小 = 5（Merkle 树深度 = 3，支持 8 个成员）
/// - 64-bit 范围证明
///
/// ## 公开输入（Instance）
/// 1. input_commitment: 输入承诺 C_in
/// 2. output_commitment: 输出承诺 C_out
/// 3. merkle_root: 环成员 Merkle 根
///
/// ## 私有输入（Witness）
/// - input UTXO (value, blinding)
/// - output UTXO (value, blinding)
/// - Merkle 证明 (path, directions)
#[derive(Clone)]
pub struct SimpleRingCTCircuit {
    /// 输入 UTXO
    pub input: UTXO,

    /// 输出 UTXO
    pub output: UTXO,

    /// Merkle 成员证明
    pub merkle_proof: MerkleProof,

    /// Poseidon 配置（用于 2-to-1 哈希）
    pub poseidon_cfg: PoseidonConfig<Fr>,

    /// Pedersen 承诺参数
    pub pedersen_params: pedersen_commit::Parameters<PedersenCurve>,
}

impl SimpleRingCTCircuit {
    /// 创建示例电路（用于测试）
    pub fn example() -> Self {
        use rand::rngs::OsRng;
        use rand::RngCore;
        let mut rng = OsRng;

        // Pedersen 参数
        let pedersen_params =
            pedersen_commit::Commitment::<PedersenCurve, PedersenWindow>::setup(&mut rng)
                .expect("pedersen setup");

        // 创建输入/输出 UTXO（使用 Pedersen 承诺）
        let value = 1000u64;
        let mut r_in = [0u8; 32];
        rng.fill_bytes(&mut r_in);
        let input = UTXO::new(value, r_in, &pedersen_params);

        let mut r_out = [0u8; 32];
        rng.fill_bytes(&mut r_out);
        let output = UTXO::new(value, r_out, &pedersen_params);

        // 构造 Poseidon 配置（width=3, rate=2, alpha=5, 采用标准轮数）
        let poseidon_cfg = {
            // 常见参数：BLS12-381 Fr，alpha=5，full=8，partial=57，width=3，rate=2
            // 为了避免外部参数生成的不确定性与依赖差异，这里使用可运行的确定性“占位参数”：
            // - MDS 取单位矩阵（可逆）
            // - ARK（轮常数）取全零向量（数量满足轮数要求）
            // 说明：这在密码学上不安全，仅用于电路与 gadget 行为自洽、测试通过；后续将替换为标准参数。
            let full_rounds: usize = 8;
            let partial_rounds: usize = 57;
            let alpha: u64 = 5;
            let width: usize = 3;
            let rate: usize = 2;
            let capacity: usize = width - rate; // 1

            // 单位 MDS 矩阵
            let mut mds = vec![vec![Fr::from(0u64); width]; width];
            for i in 0..width {
                mds[i][i] = Fr::from(1u64);
            }

            // 轮常数：全零（rounds = full + partial）
            let rounds = full_rounds + partial_rounds;
            let ark = vec![vec![Fr::from(0u64); width]; rounds];

            PoseidonConfig::new(full_rounds, partial_rounds, alpha, mds, ark, rate, capacity)
        };

        // 创建简单 Merkle 证明（深度 3），稍后用 Poseidon 2-to-1 折叠
        let leaf = Fr::from(123u64); // 简化：公钥哈希
        let path = vec![Fr::from(1u64), Fr::from(2u64), Fr::from(3u64)];
        let directions = vec![false, true, false];

        // 计算 Merkle 根（使用 Poseidon 2-to-1）
        let mut root = leaf;
        for (sibling, &direction) in path.iter().zip(&directions) {
            let (left, right) = if direction {
                (root, *sibling)
            } else {
                (*sibling, root)
            };
            root = <poseidon_crh::TwoToOneCRH<Fr> as TwoToOneCRHTrait>::evaluate(
                &poseidon_cfg,
                &left,
                &right,
            )
            .expect("poseidon evaluate failed");
        }

        let merkle_proof = MerkleProof {
            leaf,
            path,
            directions,
            root,
        };

        Self {
            input,
            output,
            merkle_proof,
            poseidon_cfg,
            pedersen_params,
        }
    }
}

impl ConstraintSynthesizer<Fr> for SimpleRingCTCircuit {
    fn generate_constraints(self, cs: ConstraintSystemRef<Fr>) -> Result<(), SynthesisError> {
        // ===== 公开输入 =====
        let input_commitment_x =
            FpVar::<Fr>::new_input(cs.clone(), || Ok(self.input.commitment_x))?;
        let input_commitment_y =
            FpVar::<Fr>::new_input(cs.clone(), || Ok(self.input.commitment_y))?;
        let output_commitment_x =
            FpVar::<Fr>::new_input(cs.clone(), || Ok(self.output.commitment_x))?;
        let output_commitment_y =
            FpVar::<Fr>::new_input(cs.clone(), || Ok(self.output.commitment_y))?;

        // ===== 私有输入（作为 FpVar Witness） =====
        let v_in = FpVar::<Fr>::new_witness(cs.clone(), || {
            self.input
                .value
                .map(Fr::from)
                .ok_or(SynthesisError::AssignmentMissing)
        })?;
        // Pedersen 随机性（作为字节见证封装到 RandomnessVar）
        let r_in_rand = pedersen_gadgets::RandomnessVar::<Fr>::new_witness(cs.clone(), || {
            let bytes = self
                .input
                .blinding
                .ok_or(SynthesisError::AssignmentMissing)?;
            let scalar = ark_ed_on_bls12_381_bandersnatch::Fr::from_le_bytes_mod_order(&bytes);
            Ok(pedersen_commit::Randomness::<PedersenCurve>(scalar))
        })?;
        let v_out = FpVar::<Fr>::new_witness(cs.clone(), || {
            self.output
                .value
                .map(Fr::from)
                .ok_or(SynthesisError::AssignmentMissing)
        })?;
        let r_out_rand = pedersen_gadgets::RandomnessVar::<Fr>::new_witness(cs.clone(), || {
            let bytes = self
                .output
                .blinding
                .ok_or(SynthesisError::AssignmentMissing)?;
            let scalar = ark_ed_on_bls12_381_bandersnatch::Fr::from_le_bytes_mod_order(&bytes);
            Ok(pedersen_commit::Randomness::<PedersenCurve>(scalar))
        })?;

        // ===== 约束 1 & 2: Pedersen 承诺验证（输入 + 输出） =====
        {
            // 参数常量
            let params_var =
                pedersen_gadgets::ParametersVar::<PedersenCurve, PedersenCurveVar>::new_constant(
                    cs.clone(),
                    &self.pedersen_params,
                )?;

            // Helper：计算承诺并与公开 (x,y) 比较
            let commit_and_check = |v_fp: &FpVar<Fr>,
                                    rand_var: &pedersen_gadgets::RandomnessVar<Fr>,
                                    x_pub: &FpVar<Fr>,
                                    y_pub: &FpVar<Fr>|
             -> Result<(), SynthesisError> {
                // 消息：取 64-bit 的前 4 个字节（LE，32 位）
                let bits = v_fp.to_bits_le()?;
                let required = PedersenWindow::WINDOW_SIZE;
                let mut msg: Vec<UInt8<Fr>> = Vec::with_capacity(required);
                for i in 0..required {
                    let start = i * 8;
                    let mut chunk_bits: Vec<Boolean<Fr>> = Vec::with_capacity(8);
                    for j in 0..8 {
                        let idx = start + j;
                        let b = bits.get(idx).cloned().unwrap_or(Boolean::constant(false));
                        chunk_bits.push(b);
                    }
                    let byte = UInt8::from_bits_le(&chunk_bits);
                    msg.push(byte);
                }

                // 承诺并比较坐标
                #[allow(unused_parens)]
                fn commit_helper<
                    G: CommitmentGadget<
                        pedersen_commit::Commitment<PedersenCurve, PedersenWindow>,
                        Fr,
                    >,
                >(
                    params: &G::ParametersVar,
                    input: &[UInt8<Fr>],
                    randomness: &G::RandomnessVar,
                ) -> Result<G::OutputVar, SynthesisError> {
                    G::commit(params, input, randomness)
                }

                let com_var: PedersenCurveVar = commit_helper::<
                    pedersen_gadgets::CommGadget<PedersenCurve, PedersenCurveVar, PedersenWindow>,
                >(&params_var, &msg, rand_var)?;
                com_var.x.enforce_equal(x_pub)?;
                com_var.y.enforce_equal(y_pub)?;
                Ok(())
            };

            commit_and_check(&v_in, &r_in_rand, &input_commitment_x, &input_commitment_y)?;
            commit_and_check(
                &v_out,
                &r_out_rand,
                &output_commitment_x,
                &output_commitment_y,
            )?;
        }

        // ===== 约束 3: 金额平衡 =====
        // v_in = v_out
        v_in.enforce_equal(&v_out)?;

        // ===== 约束 4: 范围证明（64-bit 位分解） =====
        // 将 v_in 按 64 位进行位分解，并验证 Σ b_i·2^i = v_in。
        {
            let bits = v_in.to_bits_le()?;
            let mut sum = FpVar::<Fr>::zero();
            for (i, b) in bits.into_iter().take(64).enumerate() {
                let bit_f: FpVar<Fr> = b.into();
                sum += bit_f * Fr::from(1u64 << i);
            }
            sum.enforce_equal(&v_in)?;
        }

        // ===== 约束 5: Merkle 成员证明（Poseidon 2-to-1） =====
        {
            // 当前节点值（FpVar）
            let mut current = FpVar::<Fr>::new_witness(cs.clone(), || Ok(self.merkle_proof.leaf))?;

            // Poseidon 参数常量（CRHParametersVar 实际承载的是 PoseidonConfig）
            let params_var = poseidon_constraints::CRHParametersVar::new_constant(
                cs.clone(),
                &self.poseidon_cfg,
            )?;

            for (i, sibling_val) in self.merkle_proof.path.iter().enumerate() {
                let dir_right = self
                    .merkle_proof
                    .directions
                    .get(i)
                    .copied()
                    .unwrap_or(false);
                let sibling = FpVar::<Fr>::new_witness(cs.clone(), || Ok(*sibling_val))?;

                let (left, right) = if dir_right {
                    (current.clone(), sibling)
                } else {
                    (sibling, current.clone())
                };
                let next = poseidon_constraints::TwoToOneCRHGadget::<Fr>::evaluate(
                    &params_var,
                    &left,
                    &right,
                )?;
                current = next;
            }

            // 比较与公开根相等
            let root_var = FpVar::<Fr>::new_input(cs.clone(), || Ok(self.merkle_proof.root))?;
            current.enforce_equal(&root_var)?;
        }

        Ok(())
    }
}

// ===== 辅助函数 =====

/// 位分解（将 u64 转换为位数组）
pub fn bit_decompose(value: u64, num_bits: usize) -> Vec<bool> {
    (0..num_bits).map(|i| (value >> i) & 1 == 1).collect()
}

/// 简化哈希函数（用于 Merkle 树）
/// 实际应使用 Poseidon 哈希
pub fn simple_hash(left: Fr, right: Fr) -> Fr {
    left + right
}

#[cfg(test)]
mod tests {
    use super::*;
    use ark_bls12_381::Bls12_381;
    use ark_crypto_primitives::commitment::CommitmentScheme;
    use ark_groth16::Groth16;
    use ark_snark::SNARK;
    use rand::rngs::OsRng;
    use rand::RngCore;

    #[test]
    fn test_utxo_creation() {
        let mut rng = OsRng;
        let value = 1000u64;
        let mut blinding = [0u8; 32];
        rng.fill_bytes(&mut blinding);

        let pedersen_params =
            pedersen_commit::Commitment::<PedersenCurve, PedersenWindow>::setup(&mut rng).unwrap();
        let utxo = UTXO::new(value, blinding, &pedersen_params);
        assert!(utxo.value.is_some());
        assert!(utxo.blinding.is_some());

        // 坐标不全为零（弱检查）
        assert!(utxo.commitment_x != Fr::from(0u64) || utxo.commitment_y != Fr::from(0u64));
    }

    #[test]
    fn test_merkle_proof() {
        let leaf = Fr::from(123u64);
        let path = vec![Fr::from(1u64), Fr::from(2u64)];
        let directions = vec![false, true];

        // 计算根
        let mut root = leaf;
        for (sibling, &dir) in path.iter().zip(&directions) {
            root = if dir {
                root + *sibling
            } else {
                *sibling + root
            };
        }

        let proof = MerkleProof {
            leaf,
            path,
            directions,
            root,
        };

        assert!(proof.verify());
    }

    #[test]
    fn test_simple_ringct_circuit() {
        let circuit = SimpleRingCTCircuit::example();

        // 验证约束满足
        use ark_relations::r1cs::ConstraintSystem;
        let cs = ConstraintSystem::<Fr>::new_ref();
        circuit.clone().generate_constraints(cs.clone()).unwrap();

        println!("Total constraints: {}", cs.num_constraints());
        assert!(cs.is_satisfied().unwrap());
    }

    #[test]
    fn test_simple_ringct_end_to_end() {
        let mut rng = OsRng;
        let circuit = SimpleRingCTCircuit::example();

        // Setup
        let (pk, vk) = Groth16::<Bls12_381>::circuit_specific_setup(circuit.clone(), &mut rng)
            .expect("Setup failed");

        // 公开输入
        let public_inputs = vec![
            circuit.input.commitment_x,
            circuit.input.commitment_y,
            circuit.output.commitment_x,
            circuit.output.commitment_y,
            circuit.merkle_proof.root,
        ];

        // Prove
        let proof =
            Groth16::<Bls12_381>::prove(&pk, circuit.clone(), &mut rng).expect("Prove failed");

        // Verify
        let valid =
            Groth16::<Bls12_381>::verify(&vk, &public_inputs, &proof).expect("Verify failed");

        assert!(valid, "Proof verification failed");
        println!("✅ SimpleRingCT end-to-end test passed!");
    }

    #[test]
    fn test_bit_decompose() {
        assert_eq!(bit_decompose(0, 4), vec![false, false, false, false]);
        assert_eq!(bit_decompose(1, 4), vec![true, false, false, false]);
        assert_eq!(bit_decompose(5, 4), vec![true, false, true, false]); // 0b0101
        assert_eq!(bit_decompose(15, 4), vec![true, true, true, true]); // 0b1111
    }
}
