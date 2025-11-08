// SPDX-License-Identifier: GPL-3.0-or-later
// Demo: Groth16 ringct_v1 (SimpleRingCT) verification with serialization + reload
// Requires feature: groth16-verifier

use std::fs::{create_dir_all, File};
use std::io::{Read, Write};
use std::path::PathBuf;

use vm_runtime::privacy::{Groth16Verifier, ZkCircuitId, ZkVerifier};

use ark_bls12_381::{Bls12_381, Fr};
use ark_groth16::{prepare_verifying_key, Groth16, VerifyingKey};
use ark_serialize::{CanonicalDeserialize, CanonicalSerialize};
use ark_snark::SNARK;
use rand::rngs::OsRng;
use zk_groth16_test::ringct::SimpleRingCTCircuit;

fn main() {
    // Resolve artifact directory: target/tmp/zk_ringct_demo
    let out_dir = std::env::var("CARGO_TARGET_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|_| std::env::current_dir().unwrap().join("target"));
    let dir = out_dir.join("tmp").join("zk_ringct_demo");
    create_dir_all(&dir).expect("create artifact dir");

    let rng = &mut OsRng;

    // 1) Generate example RingCT circuit (single input/output, ring size=5)
    let circuit = SimpleRingCTCircuit::example();
    println!("Generated SimpleRingCT circuit:");
    println!(
        "  Input commitment:  ({}, {})",
        circuit.input.commitment_x, circuit.input.commitment_y
    );
    println!(
        "  Output commitment: ({}, {})",
        circuit.output.commitment_x, circuit.output.commitment_y
    );
    println!("  Merkle root:       {}", circuit.merkle_proof.root);

    // 2) Trusted setup
    let params =
        Groth16::<Bls12_381>::generate_random_parameters_with_reduction(circuit.clone(), rng)
            .expect("setup");

    let vk_path = dir.join("vk.bin");
    {
        let mut vk_bytes = Vec::new();
        params
            .vk
            .serialize_uncompressed(&mut vk_bytes)
            .expect("serialize vk");
        let mut f = File::create(&vk_path).expect("open vk file");
        f.write_all(&vk_bytes).expect("write vk");
        println!(
            "Wrote VK to {} ({} bytes)",
            vk_path.display(),
            vk_bytes.len()
        );
    }

    // 3) Prove
    let proof = Groth16::<Bls12_381>::prove(&params, circuit.clone(), rng).expect("prove");

    let proof_path = dir.join("proof.bin");
    let inputs_path = dir.join("public_inputs.bin");

    {
        let mut proof_bytes = Vec::new();
        proof
            .serialize_uncompressed(&mut proof_bytes)
            .expect("serialize proof");
        let mut f = File::create(&proof_path).expect("open proof file");
        f.write_all(&proof_bytes).expect("write proof");
        println!(
            "Wrote Proof to {} ({} bytes)",
            proof_path.display(),
            proof_bytes.len()
        );

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
        let mut f = File::create(&inputs_path).expect("open inputs file");
        f.write_all(&inputs_bytes).expect("write inputs");
        println!(
            "Wrote public inputs to {} ({} bytes)",
            inputs_path.display(),
            inputs_bytes.len()
        );
    }

    // 4) Reload VK/Proof/inputs from disk and verify
    let mut vk_bytes = Vec::new();
    File::open(&vk_path)
        .expect("open vk")
        .read_to_end(&mut vk_bytes)
        .expect("read vk");
    let vk: VerifyingKey<Bls12_381> =
        VerifyingKey::deserialize_uncompressed_unchecked(&vk_bytes[..]).expect("deserialize vk");
    let pvk = prepare_verifying_key(&vk);

    let mut proof_bytes = Vec::new();
    File::open(&proof_path)
        .expect("open proof")
        .read_to_end(&mut proof_bytes)
        .expect("read proof");

    let mut inputs_bytes = Vec::new();
    File::open(&inputs_path)
        .expect("open inputs")
        .read_to_end(&mut inputs_bytes)
        .expect("read inputs");

    let verifier = Groth16Verifier::new();
    verifier.register_ringct_v1_with_pvk(pvk);

    let id = ZkCircuitId::from("ringct_v1");
    let ok = verifier
        .verify_proof(&id, &proof_bytes, &inputs_bytes)
        .expect("verify true");
    println!("Verify with correct public inputs => {}", ok);

    // 5) Verify with wrong public inputs (flip merkle_root)
    let wrong_inputs = vec![
        circuit.input.commitment_x,
        circuit.input.commitment_y,
        circuit.output.commitment_x,
        circuit.output.commitment_y,
        Fr::from(9999u64), // wrong merkle_root
    ];
    let mut wrong_bytes = Vec::new();
    wrong_bytes.extend_from_slice(&(5u32.to_le_bytes()));
    for inp in &wrong_inputs {
        inp.serialize_uncompressed(&mut wrong_bytes).unwrap();
    }
    let not_ok = verifier
        .verify_proof(&id, &proof_bytes, &wrong_bytes)
        .expect("verify false");
    println!("Verify with wrong public inputs => {}", not_ok);
}
