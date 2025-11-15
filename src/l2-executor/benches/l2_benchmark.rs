use criterion::{black_box, criterion_group, criterion_main, Criterion, BenchmarkId};
use l2_executor::{L2Runtime, BackendType, FibonacciProgram, Sha256Program};

/// Benchmark: 单个 Fibonacci 证明生成
fn bench_single_fibonacci(c: &mut Criterion) {
    let runtime = L2Runtime::new(BackendType::Trace).unwrap();
    let program = FibonacciProgram::new(10);
    
    c.bench_function("fib_10_prove", |b| {
        b.iter(|| {
            runtime.prove(&program, black_box(&[])).unwrap()
        })
    });
}

/// Benchmark: 不同复杂度 Fibonacci
fn bench_fibonacci_scaling(c: &mut Criterion) {
    let runtime = L2Runtime::new(BackendType::Trace).unwrap();
    let mut group = c.benchmark_group("fibonacci_scaling");
    
    for n in [5, 10, 20, 50, 100].iter() {
        let program = FibonacciProgram::new(*n);
        group.bench_with_input(BenchmarkId::from_parameter(n), n, |b, _| {
            b.iter(|| {
                runtime.prove(&program, black_box(&[])).unwrap()
            })
        });
    }
    
    group.finish();
}

/// Benchmark: 批量证明生成
fn bench_batch_proving(c: &mut Criterion) {
    let runtime = L2Runtime::new(BackendType::Trace).unwrap();
    let programs: Vec<_> = (0..10).map(|i| FibonacciProgram::new(5 + i)).collect();
    
    c.bench_function("batch_10_proofs", |b| {
        b.iter(|| {
            for prog in &programs {
                runtime.prove(prog, black_box(&[])).unwrap();
            }
        })
    });
}

/// Benchmark: 证明验证速度
fn bench_verification(c: &mut Criterion) {
    let runtime = L2Runtime::new(BackendType::Trace).unwrap();
    let program = FibonacciProgram::new(20);
    let proof = runtime.prove(&program, &[]).unwrap();
    
    c.bench_function("verify_proof", |b| {
        b.iter(|| {
            runtime.verify(black_box(&proof)).unwrap()
        })
    });
}

/// Benchmark: SHA256 程序
fn bench_sha256(c: &mut Criterion) {
    let runtime = L2Runtime::new(BackendType::Trace).unwrap();
    let mut group = c.benchmark_group("sha256");
    
    for size in [32, 64, 128].iter() {
        let data = vec![0u8; *size];
        let program = Sha256Program::new(&data);
        
        group.bench_with_input(BenchmarkId::from_parameter(size), size, |b, _| {
            b.iter(|| {
                runtime.prove(&program, black_box(&[])).unwrap()
            })
        });
    }
    
    group.finish();
}

/// Benchmark: 端到端流程
fn bench_end_to_end(c: &mut Criterion) {
    c.bench_function("e2e_prove_verify", |b| {
        b.iter(|| {
            let runtime = L2Runtime::new(BackendType::Trace).unwrap();
            let program = FibonacciProgram::new(black_box(15));
            let proof = runtime.prove(&program, &[]).unwrap();
            runtime.verify(&proof).unwrap()
        })
    });
}

criterion_group!(
    benches,
    bench_single_fibonacci,
    bench_fibonacci_scaling,
    bench_batch_proving,
    bench_verification,
    bench_sha256,
    bench_end_to_end
);
criterion_main!(benches);
