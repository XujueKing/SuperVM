// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2025 XujueKing <leadbrand@me.com>

//! 对抗性测试：双花攻击、伪造签名、边界情况
//!
//! 测试 RingCT Multi-UTXO 电路对恶意输入的抵抗能力。

#[cfg(test)]
mod adversarial_tests {
    use ark_bls12_381::Fr;
    use ark_crypto_primitives::commitment::pedersen as pedersen_commit;
    use ark_crypto_primitives::commitment::CommitmentScheme;
    use ark_crypto_primitives::crh::poseidon as poseidon_crh;
    use ark_crypto_primitives::crh::{CRHScheme, TwoToOneCRHScheme};
    use ark_crypto_primitives::sponge::poseidon::PoseidonConfig;
    use ark_ed_on_bls12_381_bandersnatch::EdwardsProjective as PedersenCurve;
    use ark_relations::r1cs::{ConstraintSynthesizer, ConstraintSystem};
    use ark_std::UniformRand;
    use rand::rngs::OsRng;
    use rand::RngCore;
    use zk_groth16_test::ring_signature::RingMember as RSMember;
    use zk_groth16_test::ringct_multi_utxo::{
        MerkleProof, MultiUTXOPedersenWindow, MultiUTXORingCTCircuit, RingAuth, UTXO,
    };

    fn setup_poseidon_config() -> PoseidonConfig<Fr> {
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
    }

    #[test]
    #[should_panic(expected = "Unsatisfiable")]
    fn test_double_spend_same_key_image() {
        // 测试：两个输入使用相同的 Key Image 应该失败（反双花约束）
        let mut rng = OsRng;
        let poseidon_cfg = setup_poseidon_config();

        let pedersen_params =
            pedersen_commit::Commitment::<PedersenCurve, MultiUTXOPedersenWindow>::setup(&mut rng)
                .expect("pedersen setup");

        // 创建 2 个输入 UTXO
        let values_in = [1000u64, 500u64];
        let inputs: [UTXO; 2] = std::array::from_fn(|i| {
            let mut r = [0u8; 32];
            rng.fill_bytes(&mut r);
            UTXO::new(values_in[i], r, &pedersen_params, &poseidon_cfg)
        });

        // 创建 2 个输出 UTXO
        let values_out = [800u64, 700u64];
        let outputs: [UTXO; 2] = std::array::from_fn(|i| {
            let mut r = [0u8; 32];
            rng.fill_bytes(&mut r);
            UTXO::new(values_out[i], r, &pedersen_params, &poseidon_cfg)
        });

        // 创建 Merkle 证明
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

        // 恶意：两个输入使用相同的 Key Image（即相同私钥和公钥）
        let secret_key = Fr::rand(&mut rng);
        let ring_size = 3usize;
        let real_index = 1usize;

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
            poseidon_crh::CRH::<Fr>::evaluate(&poseidon_cfg, vec![secret_key, public_key]).unwrap();

        // 两个输入都用同样的 key_image
        let ring_auths: [RingAuth; 2] = std::array::from_fn(|_| RingAuth {
            ring_members: ring_members.clone(),
            real_index,
            secret_key,
            key_image,
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

        // 应该 unsatisfied（因为 key_images 相同，反双花约束会尝试计算 (ki0-ki1)*inv=1，但 ki0=ki1 导致 inv 计算失败）
        assert!(!cs.is_satisfied().unwrap(), "Double spend should fail");
    }

    #[test]
    fn test_forged_signature_wrong_secret_key() {
        // 测试：使用错误的私钥应该导致 Key Image 不匹配，约束失败
        let mut rng = OsRng;
        let poseidon_cfg = setup_poseidon_config();

        let pedersen_params =
            pedersen_commit::Commitment::<PedersenCurve, MultiUTXOPedersenWindow>::setup(&mut rng)
                .expect("pedersen setup");

        let values_in = [1000u64, 500u64];
        let inputs: [UTXO; 2] = std::array::from_fn(|i| {
            let mut r = [0u8; 32];
            rng.fill_bytes(&mut r);
            UTXO::new(values_in[i], r, &pedersen_params, &poseidon_cfg)
        });

        let values_out = [800u64, 700u64];
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
                .unwrap();
            }
            MerkleProof {
                leaf,
                path,
                directions,
                root,
            }
        });

        // 构造伪造签名：正确的 Key Image，但私钥是错误的
        let ring_auths: [RingAuth; 2] = std::array::from_fn(|_| {
            let ring_size = 3usize;
            let real_index = 1usize;
            let correct_secret_key = Fr::rand(&mut rng);
            let wrong_secret_key = Fr::rand(&mut rng); // 错误的私钥

            let mut ring_members: Vec<RSMember> = Vec::with_capacity(ring_size);
            for j in 0..ring_size {
                let pk = if j == real_index {
                    correct_secret_key
                } else {
                    Fr::rand(&mut rng)
                };
                ring_members.push(RSMember {
                    public_key: pk,
                    merkle_root: None,
                });
            }
            let public_key = ring_members[real_index].public_key;
            // 用正确的 sk 生成 Key Image
            let key_image = poseidon_crh::CRH::<Fr>::evaluate(
                &poseidon_cfg,
                vec![correct_secret_key, public_key],
            )
            .unwrap();

            // 但电路内用错误的 sk
            RingAuth {
                ring_members,
                real_index,
                secret_key: wrong_secret_key,
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

        // Key Image 约束会失败：H(wrong_sk, pk) != key_image
        assert!(!cs.is_satisfied().unwrap(), "Forged signature should fail");
        println!("✅ 伪造签名测试通过：错误私钥被正确拒绝");
    }

    #[test]
    fn test_ring_membership_validation() {
        // 测试：环成员验证正常工作（公钥从环中取出应该通过）
        let mut rng = OsRng;
        let poseidon_cfg = setup_poseidon_config();

        let pedersen_params =
            pedersen_commit::Commitment::<PedersenCurve, MultiUTXOPedersenWindow>::setup(&mut rng)
                .expect("pedersen setup");

        let values_in = [1000u64, 500u64];
        let inputs: [UTXO; 2] = std::array::from_fn(|i| {
            let mut r = [0u8; 32];
            rng.fill_bytes(&mut r);
            UTXO::new(values_in[i], r, &pedersen_params, &poseidon_cfg)
        });

        let values_out = [800u64, 700u64];
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
                .unwrap();
            }
            MerkleProof {
                leaf,
                path,
                directions,
                root,
            }
        });

        // 正常构造：公钥在环中，环成员验证应该通过
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

        // 环成员验证应该成功
        assert!(
            cs.is_satisfied().unwrap(),
            "Ring membership validation should pass"
        );
        println!("✅ 环成员验证测试通过：公钥在环中时约束满足");
    }

    #[test]
    fn test_max_ring_size() {
        // 测试：更大的环大小（ring_size=10）应该仍然正常工作
        let mut rng = OsRng;
        let poseidon_cfg = setup_poseidon_config();

        let pedersen_params =
            pedersen_commit::Commitment::<PedersenCurve, MultiUTXOPedersenWindow>::setup(&mut rng)
                .expect("pedersen setup");

        let values_in = [1000u64, 500u64];
        let inputs: [UTXO; 2] = std::array::from_fn(|i| {
            let mut r = [0u8; 32];
            rng.fill_bytes(&mut r);
            UTXO::new(values_in[i], r, &pedersen_params, &poseidon_cfg)
        });

        let values_out = [800u64, 700u64];
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
                .unwrap();
            }
            MerkleProof {
                leaf,
                path,
                directions,
                root,
            }
        });

        // 构造 ring_size=10
        let ring_auths: [RingAuth; 2] = std::array::from_fn(|_| {
            let ring_size = 10usize;
            let real_index = 5usize;
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

        assert!(cs.is_satisfied().unwrap(), "Max ring size should work");
        let num_constraints = cs.num_constraints();
        println!(
            "✅ 最大环大小测试通过：ring_size=10, 约束数={}",
            num_constraints
        );
    }

    #[test]
    fn test_zero_value_transaction() {
        // 测试：零值交易应该正常工作（边界情况）
        let mut rng = OsRng;
        let poseidon_cfg = setup_poseidon_config();

        let pedersen_params =
            pedersen_commit::Commitment::<PedersenCurve, MultiUTXOPedersenWindow>::setup(&mut rng)
                .expect("pedersen setup");

        // 所有金额为 0
        let values_in = [0u64, 0u64];
        let inputs: [UTXO; 2] = std::array::from_fn(|i| {
            let mut r = [0u8; 32];
            rng.fill_bytes(&mut r);
            UTXO::new(values_in[i], r, &pedersen_params, &poseidon_cfg)
        });

        let values_out = [0u64, 0u64];
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
                .unwrap();
            }
            MerkleProof {
                leaf,
                path,
                directions,
                root,
            }
        });

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

        assert!(
            cs.is_satisfied().unwrap(),
            "Zero value transaction should work"
        );
        println!("✅ 零值交易测试通过");
    }
}
