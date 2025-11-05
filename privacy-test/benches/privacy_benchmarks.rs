use criterion::{black_box, criterion_group, criterion_main, Criterion};
use curve25519_dalek::ristretto::RistrettoPoint;
use curve25519_dalek::scalar::Scalar;
use curve25519_dalek::constants::RISTRETTO_BASEPOINT_POINT;
use rand::rngs::OsRng;

fn bench_scalar_multiplication(c: &mut Criterion) {
    let scalar = Scalar::random(&mut OsRng);
    let point = RISTRETTO_BASEPOINT_POINT;
    
    c.bench_function("scalar_multiplication", |b| {
        b.iter(|| {
            black_box(scalar * point)
        })
    });
}

fn bench_multiscalar_multiplication(c: &mut Criterion) {
    let scalars: Vec<Scalar> = (0..10).map(|_| Scalar::random(&mut OsRng)).collect();
    let points: Vec<RistrettoPoint> = (0..10).map(|_| RistrettoPoint::random(&mut OsRng)).collect();
    
    c.bench_function("multiscalar_multiplication_10", |b| {
        b.iter(|| {
            black_box(RistrettoPoint::multiscalar_mul(&scalars, &points))
        })
    });
}

fn bench_point_compression(c: &mut Criterion) {
    let point = RistrettoPoint::random(&mut OsRng);
    
    c.bench_function("point_compression", |b| {
        b.iter(|| {
            black_box(point.compress())
        })
    });
}

fn bench_point_decompression(c: &mut Criterion) {
    let point = RistrettoPoint::random(&mut OsRng);
    let compressed = point.compress();
    
    c.bench_function("point_decompression", |b| {
        b.iter(|| {
            black_box(compressed.decompress())
        })
    });
}

criterion_group!(
    benches,
    bench_scalar_multiplication,
    bench_multiscalar_multiplication,
    bench_point_compression,
    bench_point_decompression
);
criterion_main!(benches);
