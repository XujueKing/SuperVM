use criterion::{black_box, criterion_group, criterion_main, Criterion};
use rand::rngs::OsRng;
use zk_groth16_test::{MultiplyCircuit, range_proof::RangeProofCircuit, pedersen::PedersenCommitmentCircuit};
use ark_bls12_381::{Bls12_381, Fr};
use ark_groth16::{Groth16, prepare_verifying_key};
use ark_snark::SNARK;
use zk_groth16_test::ringct::SimpleRingCTCircuit;

fn bench_multiply_setup(c: &mut Criterion) {
    let rng = &mut OsRng;

    c.bench_function("multiply_setup", |b| {
        b.iter(|| {
            let circuit = MultiplyCircuit { a: None, b: None };
            black_box(Groth16::<Bls12_381>::generate_random_parameters_with_reduction(circuit, rng).unwrap())
        })
    });
}

fn bench_multiply_prove(c: &mut Criterion) {
    let rng = &mut OsRng;
    let circuit = MultiplyCircuit { a: None, b: None };
    let params = Groth16::<Bls12_381>::generate_random_parameters_with_reduction(circuit, rng).unwrap();

    c.bench_function("multiply_prove", |bch| {
        bch.iter(|| {
            let a = Fr::from(3u64);
            let b = Fr::from(5u64);
            let circuit = MultiplyCircuit { a: Some(a), b: Some(b) };
            black_box(Groth16::<Bls12_381>::prove(&params, circuit, rng).unwrap())
        })
    });
}

fn bench_multiply_verify(c: &mut Criterion) {
    let rng = &mut OsRng;
    let circuit = MultiplyCircuit { a: None, b: None };
    let params = Groth16::<Bls12_381>::generate_random_parameters_with_reduction(circuit, rng).unwrap();

    let a = Fr::from(3u64);
    let b = Fr::from(5u64);
    let c_val = a * b;

    let circuit = MultiplyCircuit { a: Some(a), b: Some(b) };
    let proof = Groth16::<Bls12_381>::prove(&params, circuit, rng).unwrap();
    let pvk = prepare_verifying_key(&params.vk);

    c.bench_function("multiply_verify", |bch| {
        bch.iter(|| {
            black_box(Groth16::<Bls12_381>::verify_proof(&pvk, &proof, &[c_val]).unwrap())
        })
    });
}

fn bench_range_proof_8bit(c: &mut Criterion) {
    let rng = &mut OsRng;
    let params = Groth16::<Bls12_381>::generate_random_parameters_with_reduction(
        RangeProofCircuit::new(None, 8),
        rng,
    ).unwrap();

    c.bench_function("range_8bit_prove", |bch| {
        bch.iter(|| {
            black_box(Groth16::<Bls12_381>::prove(&params, RangeProofCircuit::new(Some(42), 8), rng).unwrap())
        })
    });
}

fn bench_range_proof_64bit_setup(c: &mut Criterion) {
    let rng = &mut OsRng;

    c.bench_function("range_64bit_setup", |bch| {
        bch.iter(|| {
            black_box(Groth16::<Bls12_381>::generate_random_parameters_with_reduction(
                RangeProofCircuit::new(None, 64),
                rng,
            ).unwrap())
        })
    });
}

fn bench_range_proof_64bit(c: &mut Criterion) {
    let rng = &mut OsRng;
    let test_value = 12345678901234u64;
    let params = Groth16::<Bls12_381>::generate_random_parameters_with_reduction(
        RangeProofCircuit::new(None, 64),
        rng,
    ).unwrap();

    c.bench_function("range_64bit_prove", |bch| {
        bch.iter(|| {
            black_box(Groth16::<Bls12_381>::prove(
                &params,
                RangeProofCircuit::new(Some(test_value), 64),
                rng,
            ).unwrap())
        })
    });
}

fn bench_pedersen_prove(c: &mut Criterion) {
    let rng = &mut OsRng;
    let k = 7u64;
    let params = Groth16::<Bls12_381>::generate_random_parameters_with_reduction(
        PedersenCommitmentCircuit::new(None, None, k),
        rng,
    ).unwrap();

    c.bench_function("pedersen_prove", |bch| {
        bch.iter(|| {
            black_box(Groth16::<Bls12_381>::prove(
                &params,
                PedersenCommitmentCircuit::new(Some(100), Some(42), k),
                rng,
            ).unwrap())
        })
    });
}

fn bench_combined_setup(c: &mut Criterion) {
    use zk_groth16_test::combined::CombinedCircuit;
    let rng = &mut OsRng;
    let k = 7u64;

    c.bench_function("combined_setup", |bch| {
        bch.iter(|| {
            black_box(Groth16::<Bls12_381>::generate_random_parameters_with_reduction(
                CombinedCircuit::new(None, None, k),
                rng,
            ).unwrap())
        })
    });
}

fn bench_combined_prove(c: &mut Criterion) {
    use zk_groth16_test::combined::CombinedCircuit;
    let rng = &mut OsRng;
    let k = 7u64;
    let v = 123456789012345u64;
    let r = Fr::from(987654321u64);

    let params = Groth16::<Bls12_381>::generate_random_parameters_with_reduction(
        CombinedCircuit::new(None, None, k),
        rng,
    ).unwrap();

    c.bench_function("combined_prove", |bch| {
        bch.iter(|| {
            black_box(Groth16::<Bls12_381>::prove(
                &params,
                CombinedCircuit::new(Some(v), Some(r), k),
                rng,
            ).unwrap())
        })
    });
}

fn bench_ringct_setup(c: &mut Criterion) {
    let rng = &mut OsRng;
    c.bench_function("ringct_setup", |bch| {
        bch.iter(|| {
            let circuit = SimpleRingCTCircuit::example();
            black_box(Groth16::<Bls12_381>::generate_random_parameters_with_reduction(circuit, rng).unwrap())
        })
    });
}

fn bench_ringct_prove(c: &mut Criterion) {
    let rng = &mut OsRng;
    let params = Groth16::<Bls12_381>::generate_random_parameters_with_reduction(
        SimpleRingCTCircuit::example(),
        rng,
    ).unwrap();

    c.bench_function("ringct_prove", |bch| {
        bch.iter(|| {
            black_box(Groth16::<Bls12_381>::prove(&params, SimpleRingCTCircuit::example(), rng).unwrap())
        })
    });
}

fn bench_ringct_verify(c: &mut Criterion) {
    let rng = &mut OsRng;
    let circuit = SimpleRingCTCircuit::example();
    let params = Groth16::<Bls12_381>::generate_random_parameters_with_reduction(circuit.clone(), rng).unwrap();
    let proof = Groth16::<Bls12_381>::prove(&params, circuit.clone(), rng).unwrap();
    let pvk = prepare_verifying_key(&params.vk);

    let public_inputs = vec![
        circuit.input.commitment_x,
        circuit.input.commitment_y,
        circuit.output.commitment_x,
        circuit.output.commitment_y,
        circuit.merkle_proof.root,
    ];

    c.bench_function("ringct_verify", |bch| {
        bch.iter(|| {
            black_box(Groth16::<Bls12_381>::verify_proof(&pvk, &proof, &public_inputs).unwrap())
        })
    });
}

criterion_group!(
    benches,
    bench_multiply_setup,
    bench_multiply_prove,
    bench_multiply_verify,
    bench_range_proof_8bit,
    bench_range_proof_64bit_setup,
    bench_range_proof_64bit,
    bench_pedersen_prove,
    bench_combined_setup,
    bench_combined_prove,
    bench_ringct_setup,
    bench_ringct_prove,
    bench_ringct_verify
);
criterion_main!(benches);

