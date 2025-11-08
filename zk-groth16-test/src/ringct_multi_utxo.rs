// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2025 XujueKing <leadbrand@me.com>

//! Multi-UTXO RingCT 电路 (2-in-2-out)
//!
//! 扩展压缩承诺和聚合范围证明技术到多输入输出场景
//! 预期约束数：~531 (线性扩展)

use ark_bls12_381::Fr;
use ark_ff::{Field, PrimeField};
use ark_r1cs_std::alloc::AllocVar;
use ark_r1cs_std::boolean::Boolean;
use ark_r1cs_std::eq::EqGadget;
use ark_r1cs_std::fields::fp::FpVar;
use ark_r1cs_std::fields::FieldVar;
use ark_relations::r1cs::{ConstraintSynthesizer, ConstraintSystemRef, SynthesisError};
use ark_std::Zero;

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
// Reuse Ring Signature types
use crate::ring_signature::RingMember as RSMember;

// ===== 数据结构定义 =====

/// 多 UTXO Pedersen 窗口参数
#[derive(Clone, Default)]
pub struct MultiUTXOPedersenWindow;
impl pedersen_commit::Window for MultiUTXOPedersenWindow {
    const WINDOW_SIZE: usize = 2;
    const NUM_WINDOWS: usize = 8;
}

/// 单个 UTXO（压缩版）
#[derive(Clone, Debug)]
pub struct UTXO {
    /// 承诺哈希 H(C) = Poseidon(commitment_x, commitment_y)（公开）
    pub commitment_hash: Fr,

    /// 原始承诺坐标（私有，仅用于 Prover）
    pub commitment_x: Option<Fr>,
    pub commitment_y: Option<Fr>,

    /// 金额（私有）
    pub value: Option<u64>,

    /// 盲因子（私有）
    pub blinding: Option<[u8; 32]>,
}

impl UTXO {
    /// 创建新的 UTXO
    pub fn new(
        value: u64,
        blinding: [u8; 32],
        params: &pedersen_commit::Parameters<PedersenCurve>,
        poseidon_cfg: &PoseidonConfig<Fr>,
    ) -> Self {
        // 1. 生成 Pedersen 承诺
        let mut msg = value.to_le_bytes().to_vec();
        let required = MultiUTXOPedersenWindow::WINDOW_SIZE;
        if msg.len() < required {
            msg.resize(required, 0u8);
        }
        msg.truncate(required);

        let blind_scalar = ark_ed_on_bls12_381_bandersnatch::Fr::from_le_bytes_mod_order(&blinding);
        let randomness = pedersen_commit::Randomness::<PedersenCurve>(blind_scalar);

        let aff = pedersen_commit::Commitment::<PedersenCurve, MultiUTXOPedersenWindow>::commit(
            params,
            &msg,
            &randomness,
        )
        .expect("pedersen commit");

        // 2. 计算承诺哈希
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

// ===== Multi-UTXO RingCT 电路 =====

/// 环签名授权（每个输入一个）
#[derive(Clone, Debug)]
pub struct RingAuth {
    pub ring_members: Vec<RSMember>,
    pub real_index: usize,
    pub secret_key: Fr,
    pub key_image: Fr,
}

/// Multi-UTXO RingCT 电路 (2-in-2-out)
#[derive(Clone)]
pub struct MultiUTXORingCTCircuit {
    // 输入 UTXOs (2 个)
    pub inputs: [UTXO; 2],
    // 输出 UTXOs (2 个)
    pub outputs: [UTXO; 2],
    // Merkle 证明 (每个输入一个)
    pub merkle_proofs: [MerkleProof; 2],
    // 环签名授权（每个输入一个）
    pub ring_auths: [RingAuth; 2],
    // Poseidon 配置
    pub poseidon_cfg: PoseidonConfig<Fr>,
}

impl MultiUTXORingCTCircuit {
    /// 创建示例电路用于测试
    pub fn example() -> Self {
        use rand::rngs::OsRng;
        use rand::RngCore;
        let mut rng = OsRng;

        // Poseidon 配置
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
            pedersen_commit::Commitment::<PedersenCurve, MultiUTXOPedersenWindow>::setup(&mut rng)
                .expect("pedersen setup");

        // 创建 2 个输入 UTXO (总额 1500)
        let values_in = [1000u64, 500u64];
        let inputs: [UTXO; 2] = std::array::from_fn(|i| {
            let mut r = [0u8; 32];
            rng.fill_bytes(&mut r);
            UTXO::new(values_in[i], r, &pedersen_params, &poseidon_cfg)
        });

        // 创建 2 个输出 UTXO (总额 1500)
        let values_out = [800u64, 700u64];
        let outputs: [UTXO; 2] = std::array::from_fn(|i| {
            let mut r = [0u8; 32];
            rng.fill_bytes(&mut r);
            UTXO::new(values_out[i], r, &pedersen_params, &poseidon_cfg)
        });

        // 创建 2 个 Merkle 证明
        let merkle_proofs: [MerkleProof; 2] = std::array::from_fn(|i| {
            let leaf = Fr::from((100 + i) as u64);
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

            MerkleProof {
                leaf,
                path,
                directions,
                root,
            }
        });

        // 创建环签名授权（每个输入一个，ring_size=3）
        use ark_std::UniformRand;
        let ring_auths: [RingAuth; 2] = std::array::from_fn(|_| {
            let ring_size = 3usize;
            let real_index = (rng.next_u32() as usize) % ring_size;
            let secret_key = Fr::rand(&mut rng);
            let mut ring_members: Vec<RSMember> = Vec::with_capacity(ring_size);
            for j in 0..ring_size {
                let pk = if j == real_index {
                    secret_key
                } else {
                    Fr::rand(&mut rng)
                };
                ring_members.push(RSMember {
                    public_key: pk,
                    merkle_root: None,
                });
            }
            let public_key = ring_members[real_index].public_key;
            let key_image =
                poseidon_crh::CRH::<Fr>::evaluate(&poseidon_cfg, vec![secret_key, public_key])
                    .expect("poseidon ki");
            RingAuth {
                ring_members,
                real_index,
                secret_key,
                key_image,
            }
        });

        Self {
            inputs,
            outputs,
            merkle_proofs,
            ring_auths,
            poseidon_cfg,
        }
    }
}

impl ConstraintSynthesizer<Fr> for MultiUTXORingCTCircuit {
    fn generate_constraints(self, cs: ConstraintSystemRef<Fr>) -> Result<(), SynthesisError> {
        // ===== 公开输入 =====
        // 2 个输入承诺哈希 + 2 个输出承诺哈希 + 2 个 Merkle 根 + 2 个 Key Image
        let mut input_commitment_hashes = Vec::new();
        for i in 0..2 {
            let hash = FpVar::<Fr>::new_input(cs.clone(), || Ok(self.inputs[i].commitment_hash))?;
            input_commitment_hashes.push(hash);
        }

        let mut output_commitment_hashes = Vec::new();
        for i in 0..2 {
            let hash = FpVar::<Fr>::new_input(cs.clone(), || Ok(self.outputs[i].commitment_hash))?;
            output_commitment_hashes.push(hash);
        }

        let mut merkle_roots = Vec::new();
        for i in 0..2 {
            let root = FpVar::<Fr>::new_input(cs.clone(), || Ok(self.merkle_proofs[i].root))?;
            merkle_roots.push(root);
        }

        // Key Images 公开输入
        let mut key_images = Vec::new();
        for i in 0..2 {
            let ki = FpVar::<Fr>::new_input(cs.clone(), || Ok(self.ring_auths[i].key_image))?;
            key_images.push(ki);
        }

        // ===== 私有输入 =====
        // 输入金额和承诺坐标
        let mut input_values = Vec::new();
        let mut input_coords = Vec::new();
        for i in 0..2 {
            let v = FpVar::<Fr>::new_witness(cs.clone(), || {
                self.inputs[i]
                    .value
                    .map(Fr::from)
                    .ok_or(SynthesisError::AssignmentMissing)
            })?;
            let x = FpVar::<Fr>::new_witness(cs.clone(), || {
                self.inputs[i]
                    .commitment_x
                    .ok_or(SynthesisError::AssignmentMissing)
            })?;
            let y = FpVar::<Fr>::new_witness(cs.clone(), || {
                self.inputs[i]
                    .commitment_y
                    .ok_or(SynthesisError::AssignmentMissing)
            })?;
            input_values.push(v);
            input_coords.push((x, y));
        }

        // 输出金额和承诺坐标
        let mut output_values = Vec::new();
        let mut output_coords = Vec::new();
        for i in 0..2 {
            let v = FpVar::<Fr>::new_witness(cs.clone(), || {
                self.outputs[i]
                    .value
                    .map(Fr::from)
                    .ok_or(SynthesisError::AssignmentMissing)
            })?;
            let x = FpVar::<Fr>::new_witness(cs.clone(), || {
                self.outputs[i]
                    .commitment_x
                    .ok_or(SynthesisError::AssignmentMissing)
            })?;
            let y = FpVar::<Fr>::new_witness(cs.clone(), || {
                self.outputs[i]
                    .commitment_y
                    .ok_or(SynthesisError::AssignmentMissing)
            })?;
            output_values.push(v);
            output_coords.push((x, y));
        }

        // ===== 约束 1: 承诺哈希验证（4 个承诺）=====
        {
            let params_var = poseidon_constraints::CRHParametersVar::new_constant(
                cs.clone(),
                &self.poseidon_cfg,
            )?;

            // 验证输入承诺哈希
            for i in 0..2 {
                let (x, y) = &input_coords[i];
                let hash = poseidon_constraints::CRHGadget::<Fr>::evaluate(
                    &params_var,
                    &[x.clone(), y.clone()],
                )?;
                hash.enforce_equal(&input_commitment_hashes[i])?;
            }

            // 验证输出承诺哈希
            for i in 0..2 {
                let (x, y) = &output_coords[i];
                let hash = poseidon_constraints::CRHGadget::<Fr>::evaluate(
                    &params_var,
                    &[x.clone(), y.clone()],
                )?;
                hash.enforce_equal(&output_commitment_hashes[i])?;
            }
        }

        // ===== 约束 2: 金额平衡（sum(inputs) = sum(outputs)）=====
        {
            let mut sum_in = FpVar::<Fr>::constant(Fr::from(0u64));
            for v in &input_values {
                sum_in = &sum_in + v;
            }

            let mut sum_out = FpVar::<Fr>::constant(Fr::from(0u64));
            for v in &output_values {
                sum_out = &sum_out + v;
            }

            sum_in.enforce_equal(&sum_out)?;
        }

        // ===== 约束 3: 聚合范围证明（4 个 64-bit 范围）=====
        // 为每个输入金额做范围证明
        for i in 0..2 {
            let value_u64 = self.inputs[i].value.unwrap_or(0);

            // 手动位分解
            let mut bits = Vec::with_capacity(64);
            for j in 0..64 {
                let bit_val = ((value_u64 >> j) & 1) == 1;
                let bit = Boolean::new_witness(cs.clone(), || Ok(bit_val))?;
                bits.push(bit);
            }

            // 重建并验证
            let mut reconstructed = FpVar::<Fr>::constant(Fr::from(0u64));
            for (j, bit) in bits.iter().enumerate() {
                let bit_field: FpVar<Fr> = bit.clone().into();
                reconstructed += &bit_field * Fr::from(1u64 << j);
            }
            reconstructed.enforce_equal(&input_values[i])?;
        }

        // 为每个输出金额做范围证明
        for i in 0..2 {
            let value_u64 = self.outputs[i].value.unwrap_or(0);

            // 手动位分解
            let mut bits = Vec::with_capacity(64);
            for j in 0..64 {
                let bit_val = ((value_u64 >> j) & 1) == 1;
                let bit = Boolean::new_witness(cs.clone(), || Ok(bit_val))?;
                bits.push(bit);
            }

            // 重建并验证
            let mut reconstructed = FpVar::<Fr>::constant(Fr::from(0u64));
            for (j, bit) in bits.iter().enumerate() {
                let bit_field: FpVar<Fr> = bit.clone().into();
                reconstructed += &bit_field * Fr::from(1u64 << j);
            }
            reconstructed.enforce_equal(&output_values[i])?;
        }

        // ===== 约束 4: Merkle 成员证明（2 个证明）=====
        {
            let params_var = poseidon_constraints::CRHParametersVar::new_constant(
                cs.clone(),
                &self.poseidon_cfg,
            )?;

            for i in 0..2 {
                let mut current =
                    FpVar::<Fr>::new_witness(cs.clone(), || Ok(self.merkle_proofs[i].leaf))?;

                for (j, sibling_val) in self.merkle_proofs[i].path.iter().enumerate() {
                    let dir_right = self.merkle_proofs[i]
                        .directions
                        .get(j)
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

                current.enforce_equal(&merkle_roots[i])?;
            }
        }

        // ===== 约束 5: 环签名（Key Image 正确性 + 成员资格）=====
        {
            use ark_crypto_primitives::crh::poseidon::constraints::CRHGadget as PoseidonCRHGadget;
            use ark_crypto_primitives::crh::poseidon::constraints::CRHParametersVar as PoseidonCRHParamsVar;

            let params_var = PoseidonCRHParamsVar::new_constant(cs.clone(), &self.poseidon_cfg)?;
            let mut pk_vars: Vec<FpVar<Fr>> = Vec::new();

            for i in 0..2 {
                // witness: secret_key, real public_key
                let sk_var =
                    FpVar::<Fr>::new_witness(cs.clone(), || Ok(self.ring_auths[i].secret_key))?;
                let real_pk =
                    self.ring_auths[i].ring_members[self.ring_auths[i].real_index].public_key;
                let pk_var = FpVar::<Fr>::new_witness(cs.clone(), || Ok(real_pk))?;
                pk_vars.push(pk_var.clone());

                // Key Image correctness: KI = H(sk, pk)
                let expected_ki = PoseidonCRHGadget::<Fr>::evaluate(
                    &params_var,
                    &[sk_var.clone(), pk_var.clone()],
                )?;
                expected_ki.enforce_equal(&key_images[i])?;

                // Membership: pk in ring_members (OR over equality)
                let mut found = Boolean::FALSE;
                for m in &self.ring_auths[i].ring_members {
                    let member_pk = FpVar::<Fr>::new_witness(cs.clone(), || Ok(m.public_key))?;
                    let eq = pk_var.is_eq(&member_pk)?;
                    found = found.or(&eq)?;
                }
                found.enforce_equal(&Boolean::TRUE)?;
            }

            // Anti-double-spend: key_images must be distinct
            // Enforce (ki0 - ki1) * inv = 1
            let diff = &key_images[0] - &key_images[1];
            let inv = FpVar::<Fr>::new_witness(cs.clone(), || {
                let d = self.ring_auths[0].key_image - self.ring_auths[1].key_image;
                if d.is_zero() {
                    return Err(SynthesisError::Unsatisfiable);
                }
                Ok(d.inverse().unwrap())
            })?;
            (diff * inv).enforce_equal(&FpVar::<Fr>::constant(Fr::from(1u64)))?;
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
    fn test_multi_utxo_ringct_constraints() {
        use ark_relations::r1cs::ConstraintSystem;

        let cs = ConstraintSystem::<Fr>::new_ref();
        let circuit = MultiUTXORingCTCircuit::example();

        circuit.generate_constraints(cs.clone()).unwrap();

        let num_constraints = cs.num_constraints();
        println!(
            "✅ Multi-UTXO RingCT 约束数 (2-in-2-out): {}",
            num_constraints
        );
        println!(
            "📊 vs. 单 UTXO (309): 扩展系数 {:.2}x",
            num_constraints as f64 / 309.0
        );
        println!(
            "📊 vs. 预期 (~531): {:.1}%",
            (num_constraints as f64 / 531.0) * 100.0
        );

        assert!(
            cs.is_satisfied().unwrap(),
            "Constraints should be satisfied"
        );
    }

    #[test]
    fn test_multi_utxo_ringct_end_to_end() {
        let mut rng = OsRng;

        // Setup
        let setup_circuit = MultiUTXORingCTCircuit::example();
        let mut setup_circuit_clone = setup_circuit.clone();

        // 清空私有见证用于 setup
        for i in 0..2 {
            setup_circuit_clone.inputs[i] = UTXO::public(setup_circuit.inputs[i].commitment_hash);
            setup_circuit_clone.outputs[i] = UTXO::public(setup_circuit.outputs[i].commitment_hash);
        }

        let (pk, vk) = Groth16::<Bls12_381>::circuit_specific_setup(setup_circuit_clone, &mut rng)
            .expect("setup failed");

        // Prove
        let proof = Groth16::<Bls12_381>::prove(&pk, setup_circuit.clone(), &mut rng)
            .expect("prove failed");

        // Verify
        let mut public_inputs = Vec::new();
        // 输入承诺哈希
        for i in 0..2 {
            public_inputs.push(setup_circuit.inputs[i].commitment_hash);
        }
        // 输出承诺哈希
        for i in 0..2 {
            public_inputs.push(setup_circuit.outputs[i].commitment_hash);
        }
        // Merkle 根
        for i in 0..2 {
            public_inputs.push(setup_circuit.merkle_proofs[i].root);
        }
        // Key Images
        for i in 0..2 {
            public_inputs.push(setup_circuit.ring_auths[i].key_image);
        }

        let valid =
            Groth16::<Bls12_381>::verify(&vk, &public_inputs, &proof).expect("verify failed");

        assert!(valid, "Proof should be valid");
        println!("✅ Multi-UTXO RingCT end-to-end test passed!");
    }

    #[test]
    fn test_balance_check() {
        // 测试金额不平衡应该失败
        use ark_relations::r1cs::ConstraintSystem;
        use rand::rngs::OsRng;
        use rand::RngCore;
        let mut rng = OsRng;

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

        let pedersen_params =
            pedersen_commit::Commitment::<PedersenCurve, MultiUTXOPedersenWindow>::setup(&mut rng)
                .expect("pedersen setup");

        // 创建不平衡的交易：输入 1500，输出 1400（少了 100）
        let values_in = [1000u64, 500u64];
        let inputs: [UTXO; 2] = std::array::from_fn(|i| {
            let mut r = [0u8; 32];
            rng.fill_bytes(&mut r);
            UTXO::new(values_in[i], r, &pedersen_params, &poseidon_cfg)
        });

        let values_out = [800u64, 600u64]; // 总和 1400，不匹配
        let outputs: [UTXO; 2] = std::array::from_fn(|i| {
            let mut r = [0u8; 32];
            rng.fill_bytes(&mut r);
            UTXO::new(values_out[i], r, &pedersen_params, &poseidon_cfg)
        });

        let merkle_proofs: [MerkleProof; 2] = std::array::from_fn(|i| {
            let leaf = Fr::from((100 + i) as u64);
            let path = vec![Fr::from(1u64)];
            let directions = vec![false];

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

            MerkleProof {
                leaf,
                path,
                directions,
                root,
            }
        });

        // 构造环签名授权（使环签名部分满足）
        use ark_std::UniformRand;
        let ring_auths: [RingAuth; 2] = std::array::from_fn(|_| {
            let ring_size = 3usize;
            let real_index = 1usize;
            let secret_key = Fr::rand(&mut rng);
            let mut ring_members: Vec<RSMember> = Vec::with_capacity(ring_size);
            for j in 0..ring_size {
                let pk = if j == real_index {
                    secret_key
                } else {
                    Fr::rand(&mut rng)
                };
                ring_members.push(RSMember {
                    public_key: pk,
                    merkle_root: None,
                });
            }
            let public_key = ring_members[real_index].public_key;
            let key_image =
                poseidon_crh::CRH::<Fr>::evaluate(&poseidon_cfg, vec![secret_key, public_key])
                    .unwrap();
            RingAuth {
                ring_members,
                real_index,
                secret_key,
                key_image,
            }
        });

        let circuit = MultiUTXORingCTCircuit {
            inputs,
            outputs,
            merkle_proofs,
            ring_auths,
            poseidon_cfg,
        };

        let cs = ConstraintSystem::<Fr>::new_ref();
        circuit.generate_constraints(cs.clone()).unwrap();

        // 约束应该不满足（金额不平衡）
        assert!(
            !cs.is_satisfied().unwrap(),
            "Unbalanced transaction should fail"
        );
        println!("✅ Balance check test passed: unbalanced transaction correctly rejected");
    }
}
