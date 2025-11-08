// SPDX-License-Identifier: GPL-3.0-or-later
// SuperVM Routing + optional ZK verifier wiring demo

use vm_runtime::{
    Address, ObjectId, ObjectMetadata, OwnershipManager, OwnershipType, Privacy, SuperVM,
    VmTransaction as Transaction,
};

#[cfg(feature = "groth16-verifier")]
use ark_bls12_381::{Bls12_381, Fr};
#[cfg(feature = "groth16-verifier")]
use ark_groth16::{prepare_verifying_key, Groth16};
#[cfg(feature = "groth16-verifier")]
use ark_serialize::CanonicalSerialize;
#[cfg(feature = "groth16-verifier")]
use ark_snark::SNARK;
#[cfg(feature = "groth16-verifier")]
use rand::rngs::OsRng;
#[cfg(feature = "groth16-verifier")]
use vm_runtime::privacy::{Groth16Verifier, ZkCircuitId, ZkVerifier};
#[cfg(feature = "groth16-verifier")]
use zk_groth16_test::MultiplyCircuit;

fn mk_id(n: u8) -> ObjectId {
    let mut id = [0u8; 32];
    id[0] = n;
    id
}

fn main() {
    let manager = OwnershipManager::new();
    let supervm = SuperVM::new(&manager);

    let alice: Address = [0xAA; 32];
    let bob: Address = [0xBB; 32];

    // 注册对象
    let obj1 = mk_id(1);
    let meta1 = ObjectMetadata {
        id: obj1,
        version: 0,
        ownership: OwnershipType::Owned(alice),
        object_type: "Asset::Coin".into(),
        created_at: 0,
        updated_at: 0,
        size: 128,
        is_deleted: false,
    };
    manager.register_object(meta1).unwrap();

    let obj2 = mk_id(2);
    let meta2 = ObjectMetadata {
        id: obj2,
        version: 0,
        ownership: OwnershipType::Shared,
        object_type: "DEX::Pool".into(),
        created_at: 0,
        updated_at: 0,
        size: 1024,
        is_deleted: false,
    };
    manager.register_object(meta2).unwrap();

    // 交易 1：Alice 操作自己的对象（公开）→ Fast Path
    let tx1 = Transaction {
        from: alice,
        objects: vec![obj1],
        privacy: Privacy::Public,
    };
    let r1 = supervm.execute_transaction(&tx1);
    println!("TX1: {:?}", r1);

    // 交易 2：包含共享对象（公开）→ Consensus Path
    let tx2 = Transaction {
        from: alice,
        objects: vec![obj1, obj2],
        privacy: Privacy::Public,
    };
    let r2 = supervm.execute_transaction(&tx2);
    println!("TX2: {:?}", r2);

    // 交易 3：隐私模式 → Private Path
    let tx3 = Transaction {
        from: bob,
        objects: vec![obj1],
        privacy: Privacy::Private,
    };
    let r3 = supervm.execute_transaction(&tx3);
    println!("TX3: {:?}", r3);

    // 可选：ZK 验证器接入演示（启用 feature 时）
    #[cfg(feature = "groth16-verifier")]
    {
        // Setup a tiny multiply proof
        let rng = &mut OsRng;
        let params = Groth16::<Bls12_381>::generate_random_parameters_with_reduction(
            MultiplyCircuit { a: None, b: None },
            rng,
        )
        .expect("setup");
        let pvk = prepare_verifying_key(&params.vk);

        let a = Fr::from(2u64);
        let b = Fr::from(9u64);
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

        // Serialize inputs per register_multiply_v1_with_pvk contract
        let mut proof_bytes = Vec::new();
        proof.serialize_uncompressed(&mut proof_bytes).unwrap();
        let mut c_bytes = Vec::new();
        c.serialize_uncompressed(&mut c_bytes).unwrap();

        // Wire verifier into SuperVM and verify
        let verifier = Groth16Verifier::new();
        verifier.register_multiply_v1_with_pvk(pvk);
        let supervm2 = supervm.with_verifier(&verifier);
        let circuit = ZkCircuitId::from("multiply_v1");
        match supervm2.verify_with(&circuit, &proof_bytes, &c_bytes) {
            Ok(ok) => println!("verify_with(multiply_v1) => {}", ok),
            Err(e) => println!("verify_with error: {}", e),
        }
    }
}
