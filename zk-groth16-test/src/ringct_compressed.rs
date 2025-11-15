// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2025 XujueKing <leadbrand@me.com>

//! 压缩承诺版本的 RingCT 电路
//!
//! 优化策略：
//! - 使用 Poseidon(commitment) 代替完整的 Pedersen 点验证
//! - 大幅减少椭圆曲线相关约束
//! - 预期约束数：~1500（从 4755 降低约 70%）

use ark_bls12_381::Fr;
use ark_ff::PrimeField;
use ark_r1cs_std::alloc::AllocVar;
use ark_r1cs_std::eq::EqGadget;
use ark_r1cs_std::fields::fp::FpVar;
use ark_r1cs_std::fields::FieldVar;
use ark_relations::r1cs::{ConstraintSynthesizer, ConstraintSystemRef, SynthesisError};
use std::vec::Vec;

// Pedersen commitment (native only)
use ark_crypto_primitives::commitment::pedersen as pedersen_commit;
use ark_crypto_primitives::commitment::pedersen::Window;
use ark_crypto_primitives::commitment::CommitmentScheme;
use ark_ed_on_bls12_381_bandersnatch::EdwardsProjective as PedersenCurve;

// Poseidon for hashing commitments and Merkle tree
use ark_crypto_primitives::crh::poseidon as poseidon_crh;
use ark_crypto_primitives::crh::poseidon::constraints as poseidon_constraints;
use ark_crypto_primitives::crh::{
    CRHScheme, CRHSchemeGadget, TwoToOneCRHScheme, TwoToOneCRHSchemeGadget,
};
use ark_crypto_primitives::sponge::poseidon::PoseidonConfig;

// ===== 数据结构定义 =====

/// 压缩版 UTXO（使用哈希代替坐标）
#[derive(Clone, Debug)]
pub struct CompressedUTXO {
    /// 承诺哈希 H(C) = Poseidon(commitment_x, commitment_y)（公开）
    pub commitment_hash: Fr,

    /// 原始承诺坐标（私有，仅用于 Prover 生成哈希）
    pub commitment_x: Option<Fr>,
    pub commitment_y: Option<Fr>,

    /// 金额（私有）
    pub value: Option<u64>,

    /// 盲因子（私有）
    pub blinding: Option<[u8; 32]>,
}

impl CompressedUTXO {
    /// 创建新的压缩 UTXO
    pub fn new(
        value: u64,
        blinding: [u8; 32],
        params: &pedersen_commit::Parameters<PedersenCurve>,
        poseidon_cfg: &PoseidonConfig<Fr>,
    ) -> Self {
        // 1. 生成 Pedersen 承诺（原生）
        let mut msg = value.to_le_bytes().to_vec();
        let required = CompressedPedersenWindow::WINDOW_SIZE;
        if msg.len() < required {
            msg.resize(required, 0u8);
        }
        msg.truncate(required);

        let blind_scalar = ark_ed_on_bls12_381_bandersnatch::Fr::from_le_bytes_mod_order(&blinding);
        let randomness = pedersen_commit::Randomness::<PedersenCurve>(blind_scalar);

        let aff = pedersen_commit::Commitment::<PedersenCurve, CompressedPedersenWindow>::commit(
            params,
            &msg,
            &randomness,
        )
        .expect("pedersen commit");

        // 2. 计算承诺哈希 H(x, y)
        let commitment_hash = poseidon_crh::CRH::<Fr>::evaluate(poseidon_cfg, vec![aff.x, aff.y])
            .expect("poseidon hash");

        Self {
            commitment_hash,
            commitment_x: Some(aff.x),
            commitment_y: Some(aff.y),
            value: Some(value),
            blinding: Some(blinding),
        }
    }

    /// 创建公开 UTXO（仅 Verifier 视角）
    pub fn public(commitment_hash: Fr) -> Self {
        Self {
            commitment_hash,
            commitment_x: None,
            commitment_y: None,
            value: None,
            blinding: None,
        }
    }
}

/// 压缩版 Pedersen 窗口（更小以减少约束）
#[derive(Clone, Default)]
pub struct CompressedPedersenWindow;
impl pedersen_commit::Window for CompressedPedersenWindow {
    const WINDOW_SIZE: usize = 2;
    const NUM_WINDOWS: usize = 8; // 进一步减小
}

/// Merkle 成员证明
#[derive(Clone, Debug)]
pub struct MerkleProof {
    pub leaf: Fr,
    pub path: Vec<Fr>,
    pub directions: Vec<bool>,
    pub root: Fr,
}

impl MerkleProof {
    pub fn verify(&self, poseidon_cfg: &PoseidonConfig<Fr>) -> bool {
        let mut current = self.leaf;

        for (sibling, &direction) in self.path.iter().zip(&self.directions) {
            let (left, right) = if direction {
                (current, *sibling)
            } else {
                (*sibling, current)
            };
            current = <poseidon_crh::TwoToOneCRH<Fr> as TwoToOneCRHScheme>::evaluate(
                poseidon_cfg,
                &left,
                &right,
            )
            .expect("poseidon 2-to-1");
        }

        current == self.root
    }
}

// ===== 压缩版 RingCT 电路 =====

/// 压缩版 RingCT 电路（使用哈希承诺验证）
#[derive(Clone)]
pub struct CompressedRingCTCircuit {
    pub input: CompressedUTXO,
    pub output: CompressedUTXO,
    pub merkle_proof: MerkleProof,
    pub poseidon_cfg: PoseidonConfig<Fr>,
}

impl CompressedRingCTCircuit {
    /// 创建示例电路
    pub fn example() -> Self {
        use rand::rngs::OsRng;
        use rand::RngCore;
        let mut rng = OsRng;

        // Poseidon 配置（2-to-1 CRH 和 n-to-1 CRH 共用）
        let poseidon_cfg = {
            let full_rounds: usize = 8;
            let partial_rounds: usize = 57;
            let alpha: u64 = 5;
            let width: usize = 3;
            let rate: usize = 2;
            let capacity: usize = 1;

            let mut mds = vec![vec![Fr::from(0u64); width]; width];
            for i in 0..width {
                mds[i][i] = Fr::from(1u64);
            }

            let rounds = full_rounds + partial_rounds;
            let ark = vec![vec![Fr::from(0u64); width]; rounds];

            PoseidonConfig::new(full_rounds, partial_rounds, alpha, mds, ark, rate, capacity)
        };

        // Pedersen 参数
        let pedersen_params =
            pedersen_commit::Commitment::<PedersenCurve, CompressedPedersenWindow>::setup(&mut rng)
                .expect("pedersen setup");

        // 创建输入/输出 UTXO
        let value = 1000u64;
        let mut r_in = [0u8; 32];
        rng.fill_bytes(&mut r_in);
        let input = CompressedUTXO::new(value, r_in, &pedersen_params, &poseidon_cfg);

        let mut r_out = [0u8; 32];
        rng.fill_bytes(&mut r_out);
        let output = CompressedUTXO::new(value, r_out, &pedersen_params, &poseidon_cfg);

        // 创建 Merkle 证明
        let leaf = Fr::from(123u64);
        let path = vec![Fr::from(1u64), Fr::from(2u64), Fr::from(3u64)];
        let directions = vec![false, true, false];

        let mut root = leaf;
        for (sibling, &direction) in path.iter().zip(&directions) {
            let (left, right) = if direction {
                (root, *sibling)
            } else {
                (*sibling, root)
            };
            root = <poseidon_crh::TwoToOneCRH<Fr> as TwoToOneCRHScheme>::evaluate(
                &poseidon_cfg,
                &left,
                &right,
            )
            .expect("poseidon evaluate");
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
        }
    }
}

impl ConstraintSynthesizer<Fr> for CompressedRingCTCircuit {
    fn generate_constraints(self, cs: ConstraintSystemRef<Fr>) -> Result<(), SynthesisError> {
        // ===== 公开输入 =====
        let input_commitment_hash =
            FpVar::<Fr>::new_input(cs.clone(), || Ok(self.input.commitment_hash))?;
        let output_commitment_hash =
            FpVar::<Fr>::new_input(cs.clone(), || Ok(self.output.commitment_hash))?;

        // ===== 私有输入 =====
        let v_in = FpVar::<Fr>::new_witness(cs.clone(), || {
            self.input
                .value
                .map(Fr::from)
                .ok_or(SynthesisError::AssignmentMissing)
        })?;
        let v_out = FpVar::<Fr>::new_witness(cs.clone(), || {
            self.output
                .value
                .map(Fr::from)
                .ok_or(SynthesisError::AssignmentMissing)
        })?;

        // 承诺坐标（私有）
        let input_x = FpVar::<Fr>::new_witness(cs.clone(), || {
            self.input
                .commitment_x
                .ok_or(SynthesisError::AssignmentMissing)
        })?;
        let input_y = FpVar::<Fr>::new_witness(cs.clone(), || {
            self.input
                .commitment_y
                .ok_or(SynthesisError::AssignmentMissing)
        })?;
        let output_x = FpVar::<Fr>::new_witness(cs.clone(), || {
            self.output
                .commitment_x
                .ok_or(SynthesisError::AssignmentMissing)
        })?;
        let output_y = FpVar::<Fr>::new_witness(cs.clone(), || {
            self.output
                .commitment_y
                .ok_or(SynthesisError::AssignmentMissing)
        })?;

        // ===== 约束 1: 承诺哈希验证（使用 Poseidon）=====
        // 验证 H(input_x, input_y) = input_commitment_hash
        {
            let params_var = poseidon_constraints::CRHParametersVar::new_constant(
                cs.clone(),
                &self.poseidon_cfg,
            )?;

            let input_hash =
                poseidon_constraints::CRHGadget::<Fr>::evaluate(&params_var, &[input_x, input_y])?;
            input_hash.enforce_equal(&input_commitment_hash)?;

            let output_hash = poseidon_constraints::CRHGadget::<Fr>::evaluate(
                &params_var,
                &[output_x, output_y],
            )?;
            output_hash.enforce_equal(&output_commitment_hash)?;
        }

        // ===== 约束 2: 金额平衡 =====
        v_in.enforce_equal(&v_out)?;

        // ===== 约束 3: 聚合范围证明（64-bit，优化版）=====
        // 使用高效位分解验证（~65 约束 vs ~130）
        {
            use ark_r1cs_std::boolean::Boolean;

            // 手动位分解输入金额
            let mut bits = Vec::with_capacity(64);
            let value_u64 = self.input.value.unwrap_or(0);

            for i in 0..64 {
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
            reconstructed.enforce_equal(&v_in)?;
        }

        // ===== 约束 4: Merkle 成员证明 =====
        {
            let mut current = FpVar::<Fr>::new_witness(cs.clone(), || Ok(self.merkle_proof.leaf))?;
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
                let next =
                    <poseidon_constraints::TwoToOneCRHGadget<Fr> as TwoToOneCRHSchemeGadget<
                        _,
                        _,
                    >>::evaluate(&params_var, &left, &right)?;
                current = next;
            }
            let root_var = FpVar::<Fr>::new_input(cs.clone(), || Ok(self.merkle_proof.root))?;
            current.enforce_equal(&root_var)?;
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
    fn test_compressed_ringct_circuit() {
        let circuit = CompressedRingCTCircuit::example();

        use ark_relations::r1cs::ConstraintSystem;
        let cs = ConstraintSystem::<Fr>::new_ref();
        circuit.clone().generate_constraints(cs.clone()).unwrap();

        println!("Total constraints (Compressed): {}", cs.num_constraints());
        assert!(cs.is_satisfied().unwrap());
    }

    #[test]
    fn test_compressed_ringct_end_to_end() {
        let mut rng = OsRng;
        let circuit = CompressedRingCTCircuit::example();

        // Setup
        let (pk, vk) = Groth16::<Bls12_381>::circuit_specific_setup(circuit.clone(), &mut rng)
            .expect("Setup failed");

        // 公开输入（3 个：两个承诺哈希 + merkle_root）
        let public_inputs = vec![
            circuit.input.commitment_hash,
            circuit.output.commitment_hash,
            circuit.merkle_proof.root,
        ];

        // Prove
        let proof =
            Groth16::<Bls12_381>::prove(&pk, circuit.clone(), &mut rng).expect("Prove failed");

        // Verify
        let valid =
            Groth16::<Bls12_381>::verify(&vk, &public_inputs, &proof).expect("Verify failed");

        assert!(valid, "Proof verification failed");
        println!("✅ CompressedRingCT end-to-end test passed!");
    }
}
