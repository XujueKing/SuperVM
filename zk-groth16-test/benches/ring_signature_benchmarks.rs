// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2025 XujueKing <leadbrand@me.com>

//! Ring Signature 性能基准测试
//!
//! 测试不同环大小下的约束数和性能

use criterion::{black_box, criterion_group, criterion_main, Criterion, BenchmarkId};
use zk_groth16_test::ring_signature::*;
use ark_bls12_381::{Bls12_381, Fr};
use ark_groth16::Groth16;
use ark_snark::SNARK;
use ark_std::UniformRand;
use rand::rngs::OsRng;
use ark_crypto_primitives::sponge::poseidon::PoseidonConfig;
use ark_relations::r1cs::ConstraintSystem;

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
    
    let ark = vec![vec![Fr::from(10u64), Fr::from(11u64), Fr::from(12u64)]; full_rounds + partial_rounds];
    
    PoseidonConfig::new(full_rounds, partial_rounds, alpha, mds, ark, rate, capacity)
}

fn bench_ring_signature_prove(c: &mut Criterion) {
    let mut group = c.benchmark_group("ring_signature_prove");
    let config = setup_poseidon_config();
    let rng = &mut OsRng;
    
    for ring_size in [3, 5, 7, 11].iter() {
        let secret_key = Fr::rand(rng);
        let real_index = ring_size / 2;
        
        let mut ring_members = vec![];
        for i in 0..*ring_size {
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
            &config,
            rng,
        ).expect("generate signature");
        
        let circuit = RingSignatureCircuit::new(signature, config.clone());
        
        // Setup
        let (pk, _vk) = Groth16::<Bls12_381>::circuit_specific_setup(circuit.clone(), rng)
            .expect("setup");
        
        group.bench_with_input(
            BenchmarkId::from_parameter(ring_size),
            ring_size,
            |b, _| {
                b.iter(|| {
                    let _proof = Groth16::<Bls12_381>::prove(&pk, circuit.clone(), rng)
                        .expect("prove");
                })
            },
        );
    }
    
    group.finish();
}

fn bench_ring_signature_verify(c: &mut Criterion) {
    let mut group = c.benchmark_group("ring_signature_verify");
    let config = setup_poseidon_config();
    let rng = &mut OsRng;
    
    for ring_size in [3, 5, 7, 11].iter() {
        let secret_key = Fr::rand(rng);
        let real_index = ring_size / 2;
        
        let mut ring_members = vec![];
        for i in 0..*ring_size {
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
            &config,
            rng,
        ).expect("generate signature");
        
        let circuit = RingSignatureCircuit::new(signature, config.clone());
        
        // Setup
        let (pk, vk) = Groth16::<Bls12_381>::circuit_specific_setup(circuit.clone(), rng)
            .expect("setup");
        
        let public_inputs = vec![circuit.signature.key_image.value];
        
        let proof = Groth16::<Bls12_381>::prove(&pk, circuit, rng)
            .expect("prove");
        
        group.bench_with_input(
            BenchmarkId::from_parameter(ring_size),
            ring_size,
            |b, _| {
                b.iter(|| {
                    let _is_valid = Groth16::<Bls12_381>::verify(&vk, &public_inputs, &proof)
                        .expect("verify");
                })
            },
        );
    }
    
    group.finish();
}

fn count_constraints(c: &mut Criterion) {
    let config = setup_poseidon_config();
    let rng = &mut OsRng;
    
    println!("\n=== Ring Signature Constraint Count Analysis ===");
    
    for ring_size in [3, 5, 7, 11, 15].iter() {
        let secret_key = Fr::rand(rng);
        let real_index = ring_size / 2;
        
        let mut ring_members = vec![];
        for i in 0..*ring_size {
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
            &config,
            rng,
        ).expect("generate signature");
        
        let circuit = RingSignatureCircuit::new(signature, config.clone());
        
        // 计算约束数
        let cs = ConstraintSystem::<Fr>::new_ref();
        circuit.generate_constraints(cs.clone()).expect("generate constraints");
        
        let num_constraints = cs.num_constraints();
        let constraints_per_member = num_constraints / ring_size;
        
        println!("Ring Size: {:2} | Constraints: {:4} | Per Member: {:3}", 
                 ring_size, num_constraints, constraints_per_member);
    }
    
    println!("================================================\n");
}

criterion_group!(
    benches,
    count_constraints,
    bench_ring_signature_prove,
    bench_ring_signature_verify
);
criterion_main!(benches);
