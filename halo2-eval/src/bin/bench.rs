use halo2_eval::MulCircuit;
use halo2_proofs::dev::MockProver;
use halo2curves::bn256::Fr;
use std::time::Instant;

fn main() {
    // security param k
    let k = 8; // 2^8 rows

    // Circuit
    let a = Fr::from(3u64);
    let b = Fr::from(5u64);
    let c = a * b;
    let circuit = MulCircuit { a: Some(a), b: Some(b) };

    // MockProver (快速功能性验证 + 粗略性能)
    let t0 = Instant::now();
    let prover = MockProver::run(k, &circuit, vec![vec![c]]).expect("mock prover");
    prover.assert_satisfied();
    let t1 = Instant::now();

    println!("halo2 Mul(k=2^{}) mock-synthesize: {:.2?}", k, t1 - t0);
}
