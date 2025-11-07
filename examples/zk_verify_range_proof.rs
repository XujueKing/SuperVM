// SPDX-License-Identifier: GPL-3.0-or-later
// Demo: Groth16 range_proof_v1 (64-bit) verification with serialization + reload
// Requires feature: groth16-verifier

use std::fs::{create_dir_all, File};
use std::io::{Read, Write};
use std::path::PathBuf;

use vm_runtime::privacy::{Groth16Verifier, ZkCircuitId, ZkVerifier};

use ark_bls12_381::{Bls12_381, Fr};
use ark_groth16::{Groth16, prepare_verifying_key, VerifyingKey};
use ark_serialize::{CanonicalSerialize, CanonicalDeserialize};
use ark_snark::SNARK;
use rand::rngs::OsRng;
use zk_groth16_test::range_proof::RangeProofCircuit;

fn main() {
    // Resolve artifact directory: target/tmp/zk_range_proof_demo
    let out_dir = std::env::var("CARGO_TARGET_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|_| std::env::current_dir().unwrap().join("target"));
    let dir = out_dir.join("tmp").join("zk_range_proof_demo");
    create_dir_all(&dir).expect("create artifact dir");

    let rng = &mut OsRng;

    // 1) Trusted setup for 64-bit range proof circuit
    let params = Groth16::<Bls12_381>::generate_random_parameters_with_reduction(
        RangeProofCircuit::new(None, 64),
        rng,
    ).expect("setup");

    let vk_path = dir.join("vk.bin");
    {
        let mut vk_bytes = Vec::new();
        params.vk.serialize_uncompressed(&mut vk_bytes).expect("serialize vk");
        let mut f = File::create(&vk_path).expect("open vk file");
        f.write_all(&vk_bytes).expect("write vk");
        println!("Wrote VK to {} ({} bytes)", vk_path.display(), vk_bytes.len());
    }

    // 2) Prove value in range: v = 12345678901234
    let value: u64 = 12345678901234u64;
    let proof = Groth16::<Bls12_381>::prove(&params, RangeProofCircuit::new(Some(value), 64), rng)
        .expect("prove");

    let proof_path = dir.join("proof.bin");
    let c_path = dir.join("c.bin");

    {
        let mut proof_bytes = Vec::new();
        proof.serialize_uncompressed(&mut proof_bytes).expect("serialize proof");
        let mut f = File::create(&proof_path).expect("open proof file");
        f.write_all(&proof_bytes).expect("write proof");
        println!("Wrote Proof to {} ({} bytes)", proof_path.display(), proof_bytes.len());

        let c = Fr::from(value);
        let mut c_bytes = Vec::new();
        c.serialize_uncompressed(&mut c_bytes).expect("serialize c");
        let mut f = File::create(&c_path).expect("open c file");
        f.write_all(&c_bytes).expect("write c");
        println!("Wrote public input c to {} ({} bytes)", c_path.display(), c_bytes.len());
    }

    // 3) Reload VK/Proof/c from disk and verify
    let mut vk_bytes = Vec::new();
    File::open(&vk_path).expect("open vk").read_to_end(&mut vk_bytes).expect("read vk");
    let vk: VerifyingKey<Bls12_381> = VerifyingKey::deserialize_uncompressed_unchecked(&vk_bytes[..])
        .expect("deserialize vk");
    let pvk = prepare_verifying_key(&vk);

    let mut proof_bytes = Vec::new();
    File::open(&proof_path).expect("open proof").read_to_end(&mut proof_bytes).expect("read proof");

    let mut c_bytes = Vec::new();
    File::open(&c_path).expect("open c").read_to_end(&mut c_bytes).expect("read c");

    let verifier = Groth16Verifier::new();
    verifier.register_range_proof_v1_with_pvk(pvk);

    let id = ZkCircuitId::from("range_proof_v1");
    let ok = verifier.verify_proof(&id, &proof_bytes, &c_bytes).expect("verify true");
    println!("Verify with correct c => {}", ok);

    // 4) Verify with wrong public input
    let wrong_c = Fr::from(value + 1);
    let mut wrong_bytes = Vec::new();
    wrong_c.serialize_uncompressed(&mut wrong_bytes).unwrap();
    let not_ok = verifier.verify_proof(&id, &proof_bytes, &wrong_bytes).expect("verify false");
    println!("Verify with wrong c => {}", not_ok);
}
