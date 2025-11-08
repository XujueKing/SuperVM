// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2025 XujueKing <leadbrand@me.com>

//! Integration tests for privacy verifier abstraction

use vm_runtime::privacy::{NoopVerifier, ZkCircuitId, ZkVerifier};

#[test]
fn noop_verifier_is_false_by_default() {
    let v = NoopVerifier::default();
    let ok = v
        .verify_proof(&ZkCircuitId::from("ring_signature_v1"), b"proof", b"inputs")
        .expect("noop should not error");
    assert!(!ok, "NoopVerifier must return false by default");
}

// Feature-gated test for the optional Groth16 adapter
#[cfg(feature = "groth16-verifier")]
mod groth16_feature_tests {
    use super::*;
    use ark_bls12_381::{Bls12_381, Fr};
    use ark_groth16::{prepare_verifying_key, Groth16};
    use ark_serialize::CanonicalSerialize;
    use ark_snark::SNARK;
    use rand::rngs::OsRng;
    use vm_runtime::privacy::{Groth16Verifier, ZkError};
    use zk_groth16_test::MultiplyCircuit;

    #[test]
    fn groth16_unknown_circuit_is_error() {
        let v = Groth16Verifier::new();
        let err = v
            .verify_proof(&ZkCircuitId::from("nonexistent_circuit"), b"p", b"i")
            .expect_err("should error for unknown circuit id");

        match err {
            ZkError::UnknownCircuit(id) => assert_eq!(id, "nonexistent_circuit"),
            other => panic!("unexpected error: {other:?}"),
        }
    }

    #[test]
    fn groth16_registered_circuit_handler_is_called() {
        let v = Groth16Verifier::new();
        // Register a dummy circuit that returns true only if proof == "ok"
        v.register("dummy_v1", |proof, _inputs| {
            if proof == b"ok" {
                Ok(true)
            } else {
                Ok(false)
            }
        });

        let id = ZkCircuitId::from("dummy_v1");
        let ok = v.verify_proof(&id, b"ok", b"_").expect("no error");
        assert!(ok);

        let not_ok = v.verify_proof(&id, b"bad", b"_").expect("no error");
        assert!(!not_ok);
    }

    #[test]
    fn groth16_multiply_v1_end_to_end_true_and_false() {
        let v = Groth16Verifier::new();

        // Setup (trusted): generate parameters and PVK for multiply circuit
        let rng = &mut OsRng;
        let params = Groth16::<Bls12_381>::generate_random_parameters_with_reduction(
            MultiplyCircuit { a: None, b: None },
            rng,
        )
        .expect("setup");
        let pvk = prepare_verifying_key(&params.vk);

        // Register circuit handler with PVK captured
        v.register_multiply_v1_with_pvk(pvk);

        // Prove a=3, b=5 => c=15 using the same params
        let a = Fr::from(3u64);
        let b = Fr::from(5u64);
        let c = a * b;
        let proof = Groth16::<Bls12_381>::prove(
            &params,
            MultiplyCircuit {
                a: Some(a),
                b: Some(b),
            },
            rng,
        )
        .expect("prove");

        // Serialize proof and public input `c`
        let mut proof_bytes = Vec::new();
        proof.serialize_uncompressed(&mut proof_bytes).unwrap();

        let mut c_bytes = Vec::new();
        c.serialize_uncompressed(&mut c_bytes).unwrap();

        let id = ZkCircuitId::from("multiply_v1");
        // Should verify true
        let ok = v.verify_proof(&id, &proof_bytes, &c_bytes).expect("verify");
        assert!(ok, "multiply_v1 should verify with correct c");

        // Wrong public input
        let wrong_c = Fr::from(999u64);
        let mut wrong_bytes = Vec::new();
        wrong_c.serialize_uncompressed(&mut wrong_bytes).unwrap();
        let not_ok = v
            .verify_proof(&id, &proof_bytes, &wrong_bytes)
            .expect("verify false");
        assert!(!not_ok, "multiply_v1 should not verify with wrong c");
    }

    #[test]
    fn groth16_generic_vec_encoding_one_input() {
        let v = Groth16Verifier::new();

        // Setup PVK
        let rng = &mut OsRng;
        let params = Groth16::<Bls12_381>::generate_random_parameters_with_reduction(
            MultiplyCircuit { a: None, b: None },
            rng,
        )
        .expect("setup");
        let pvk = prepare_verifying_key(&params.vk);
        v.register_circuit_with_pvk_fr_vec("multiply_generic_v1", pvk);

        // Prove
        let a = Fr::from(7u64);
        let b = Fr::from(6u64);
        let c = a * b;
        let proof = Groth16::<Bls12_381>::prove(
            &params,
            MultiplyCircuit {
                a: Some(a),
                b: Some(b),
            },
            rng,
        )
        .expect("prove");

        let mut proof_bytes = Vec::new();
        proof.serialize_uncompressed(&mut proof_bytes).unwrap();

        // Encode public inputs vec with length prefix = 1
        let mut inputs_bytes = Vec::new();
        inputs_bytes.extend_from_slice(&(1u32.to_le_bytes()));
        c.serialize_uncompressed(&mut inputs_bytes).unwrap();

        let id = ZkCircuitId::from("multiply_generic_v1");
        let ok = v
            .verify_proof(&id, &proof_bytes, &inputs_bytes)
            .expect("verify ok");
        assert!(ok);

        // Wrong length prefix (0) should fail verification (invalid public inputs)
        let mut wrong_inputs = Vec::new();
        wrong_inputs.extend_from_slice(&(0u32.to_le_bytes()));
        let err = v.verify_proof(&id, &proof_bytes, &wrong_inputs);
        assert!(matches!(
            err,
            Err(ZkError::Other(_)) | Err(ZkError::InvalidPublicInputs)
        ));
    }

    #[test]
    fn groth16_ring_signature_v1_end_to_end() {
        use ark_crypto_primitives::sponge::poseidon::PoseidonConfig;
        use ark_std::UniformRand as _;
        use zk_groth16_test::ring_signature::{
            RingMember, RingSignatureCircuit, RingSignatureData,
        };

        let v = Groth16Verifier::new();
        let rng = &mut OsRng;

        // Setup Poseidon config (same as in zk-groth16-test)
        let poseidon_config = {
            let full_rounds = 8;
            let partial_rounds = 57;
            let alpha = 5;
            let rate = 2;
            let capacity = 1;
            let mds = vec![
                vec![Fr::from(1u64), Fr::from(2u64), Fr::from(3u64)],
                vec![Fr::from(4u64), Fr::from(5u64), Fr::from(6u64)],
                vec![Fr::from(7u64), Fr::from(8u64), Fr::from(9u64)],
            ];
            let ark = vec![
                vec![Fr::from(10u64), Fr::from(11u64), Fr::from(12u64)];
                full_rounds + partial_rounds
            ];
            PoseidonConfig::new(full_rounds, partial_rounds, alpha, mds, ark, rate, capacity)
        };

        // Generate a ring signature
        let ring_size = 3;
        let real_index = 1;
        let secret_key = Fr::rand(rng);
        let mut ring_members = vec![];
        for i in 0..ring_size {
            let pk = if i == real_index {
                secret_key
            } else {
                Fr::rand(rng)
            };
            ring_members.push(RingMember {
                public_key: pk,
                merkle_root: None,
            });
        }
        let message = Fr::rand(rng);

        let signature = RingSignatureData::generate_signature(
            secret_key,
            real_index,
            ring_members,
            message,
            &poseidon_config,
            rng,
        )
        .expect("generate signature");

        let circuit = RingSignatureCircuit::new(signature.clone(), poseidon_config);

        // Setup (circuit-specific)
        let params =
            Groth16::<Bls12_381>::generate_random_parameters_with_reduction(circuit.clone(), rng)
                .expect("setup");
        let pvk = prepare_verifying_key(&params.vk);
        v.register_ring_signature_v1_with_pvk(pvk);

        // Prove
        let proof = Groth16::<Bls12_381>::prove(&params, circuit, rng).expect("prove");

        // Serialize proof and key_image
        let mut proof_bytes = Vec::new();
        proof.serialize_uncompressed(&mut proof_bytes).unwrap();

        let mut key_image_bytes = Vec::new();
        signature
            .key_image
            .value
            .serialize_uncompressed(&mut key_image_bytes)
            .unwrap();

        let id = ZkCircuitId::from("ring_signature_v1");
        let ok = v
            .verify_proof(&id, &proof_bytes, &key_image_bytes)
            .expect("verify");
        assert!(ok, "ring_signature_v1 should verify with correct key_image");

        // Wrong key_image should fail
        let wrong_ki = Fr::rand(rng);
        let mut wrong_bytes = Vec::new();
        wrong_ki.serialize_uncompressed(&mut wrong_bytes).unwrap();
        let not_ok = v
            .verify_proof(&id, &proof_bytes, &wrong_bytes)
            .expect("verify false");
        assert!(
            !not_ok,
            "ring_signature_v1 should not verify with wrong key_image"
        );
    }

    #[test]
    fn groth16_range_proof_v1_end_to_end() {
        use zk_groth16_test::range_proof::RangeProofCircuit;

        let v = Groth16Verifier::new();
        let rng = &mut OsRng;

        // Setup for 64-bit range
        let params = Groth16::<Bls12_381>::generate_random_parameters_with_reduction(
            RangeProofCircuit::new(None, 64),
            rng,
        )
        .expect("setup");
        let pvk = prepare_verifying_key(&params.vk);
        v.register_range_proof_v1_with_pvk(pvk);

        // Prove value = 42
        let proof = Groth16::<Bls12_381>::prove(&params, RangeProofCircuit::new(Some(42), 64), rng)
            .expect("prove");

        // Serialize proof and public input c=42
        let mut proof_bytes = Vec::new();
        proof.serialize_uncompressed(&mut proof_bytes).unwrap();
        let c = Fr::from(42u64);
        let mut c_bytes = Vec::new();
        c.serialize_uncompressed(&mut c_bytes).unwrap();

        let id = ZkCircuitId::from("range_proof_v1");
        let ok = v.verify_proof(&id, &proof_bytes, &c_bytes).expect("verify");
        assert!(ok, "range_proof_v1 should verify with correct c");

        // Wrong public input
        let wrong_c = Fr::from(43u64);
        let mut wrong_bytes = Vec::new();
        wrong_c.serialize_uncompressed(&mut wrong_bytes).unwrap();
        let not_ok = v
            .verify_proof(&id, &proof_bytes, &wrong_bytes)
            .expect("verify false");
        assert!(!not_ok, "range_proof_v1 should not verify with wrong c");
    }

    #[test]
    fn groth16_ringct_v1_end_to_end() {
        use zk_groth16_test::ringct::SimpleRingCTCircuit;

        let v = Groth16Verifier::new();
        let rng = &mut OsRng;

        // Generate example circuit
        let circuit = SimpleRingCTCircuit::example();

        // Setup
        let params =
            Groth16::<Bls12_381>::generate_random_parameters_with_reduction(circuit.clone(), rng)
                .expect("setup");
        let pvk = prepare_verifying_key(&params.vk);
        v.register_ringct_v1_with_pvk(pvk);

        // Prove
        let proof = Groth16::<Bls12_381>::prove(&params, circuit.clone(), rng).expect("prove");

        // Serialize proof
        let mut proof_bytes = Vec::new();
        proof.serialize_uncompressed(&mut proof_bytes).unwrap();

        // Encode public inputs: [length=5, input_commit_x, input_commit_y, output_commit_x, output_commit_y, merkle_root]
        let public_inputs = vec![
            circuit.input.commitment_x,
            circuit.input.commitment_y,
            circuit.output.commitment_x,
            circuit.output.commitment_y,
            circuit.merkle_proof.root,
        ];
        let mut inputs_bytes = Vec::new();
        inputs_bytes.extend_from_slice(&(5u32.to_le_bytes()));
        for inp in &public_inputs {
            inp.serialize_uncompressed(&mut inputs_bytes).unwrap();
        }

        let id = ZkCircuitId::from("ringct_v1");
        let ok = v
            .verify_proof(&id, &proof_bytes, &inputs_bytes)
            .expect("verify");
        assert!(ok, "ringct_v1 should verify with correct public inputs");

        // Wrong public inputs (flip merkle_root)
        let mut wrong_inputs = vec![
            circuit.input.commitment_x,
            circuit.input.commitment_y,
            circuit.output.commitment_x,
            circuit.output.commitment_y,
            Fr::from(999u64), // wrong merkle_root
        ];
        let mut wrong_bytes = Vec::new();
        wrong_bytes.extend_from_slice(&(5u32.to_le_bytes()));
        for inp in &wrong_inputs {
            inp.serialize_uncompressed(&mut wrong_bytes).unwrap();
        }
        let not_ok = v
            .verify_proof(&id, &proof_bytes, &wrong_bytes)
            .expect("verify false");
        assert!(
            !not_ok,
            "ringct_v1 should not verify with wrong public inputs"
        );
    }
}
