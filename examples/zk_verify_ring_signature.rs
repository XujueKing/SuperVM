// SPDX-License-Identifier: GPL-3.0-or-later
// Demo: Groth16 ring_signature_v1 verification with serialization + reload
// Requires feature: groth16-verifier

use std::fs::{create_dir_all, File};
use std::io::{Read, Write};
use std::path::PathBuf;

use vm_runtime::privacy::{Groth16Verifier, ZkCircuitId, ZkVerifier};

use ark_bls12_381::{Bls12_381, Fr};
use ark_crypto_primitives::sponge::poseidon::PoseidonConfig;
use ark_groth16::{prepare_verifying_key, Groth16, VerifyingKey};
use ark_serialize::{CanonicalDeserialize, CanonicalSerialize};
use ark_snark::SNARK;
use ark_std::UniformRand as _;
use rand::rngs::OsRng;
use zk_groth16_test::ring_signature::{RingMember, RingSignatureCircuit, RingSignatureData};

fn setup_poseidon_config() -> PoseidonConfig<Fr> {
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
    let ark =
        vec![vec![Fr::from(10u64), Fr::from(11u64), Fr::from(12u64)]; full_rounds + partial_rounds];
    PoseidonConfig::new(full_rounds, partial_rounds, alpha, mds, ark, rate, capacity)
}

fn main() {
    // Resolve artifact directory: target/tmp/zk_ring_signature_demo
    let out_dir = std::env::var("CARGO_TARGET_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|_| std::env::current_dir().unwrap().join("target"));
    let dir = out_dir.join("tmp").join("zk_ring_signature_demo");
    create_dir_all(&dir).expect("create artifact dir");

    // Setup Poseidon config
    let poseidon_config = setup_poseidon_config();

    // Generate ring signature (ring_size=5, real_index=2)
    let rng = &mut OsRng;
    let ring_size = 5;
    let real_index = 2;
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

    let circuit = RingSignatureCircuit::new(signature.clone(), poseidon_config.clone());

    // 1) Trusted setup
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

    // 2) Prove
    let proof = Groth16::<Bls12_381>::prove(&params, circuit, rng).expect("prove");

    let proof_path = dir.join("proof.bin");
    let key_image_path = dir.join("key_image.bin");

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

        let mut ki_bytes = Vec::new();
        signature
            .key_image
            .value
            .serialize_uncompressed(&mut ki_bytes)
            .expect("serialize key_image");
        let mut f = File::create(&key_image_path).expect("open key_image file");
        f.write_all(&ki_bytes).expect("write key_image");
        println!(
            "Wrote key_image to {} ({} bytes)",
            key_image_path.display(),
            ki_bytes.len()
        );
    }

    // 3) Reload VK/Proof/key_image from disk and verify
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

    let mut ki_bytes = Vec::new();
    File::open(&key_image_path)
        .expect("open key_image")
        .read_to_end(&mut ki_bytes)
        .expect("read key_image");

    let verifier = Groth16Verifier::new();
    verifier.register_ring_signature_v1_with_pvk(pvk);

    let id = ZkCircuitId::from("ring_signature_v1");
    let ok = verifier
        .verify_proof(&id, &proof_bytes, &ki_bytes)
        .expect("verify true");
    println!("Verify with correct key_image => {}", ok);

    // 4) Verify with wrong key_image
    let wrong_ki = Fr::rand(rng);
    let mut wrong_bytes = Vec::new();
    wrong_ki.serialize_uncompressed(&mut wrong_bytes).unwrap();
    let not_ok = verifier
        .verify_proof(&id, &proof_bytes, &wrong_bytes)
        .expect("verify false");
    println!("Verify with wrong key_image => {}", not_ok);
}
