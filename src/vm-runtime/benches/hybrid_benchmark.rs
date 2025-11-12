use criterion::{criterion_group, criterion_main, Criterion, BenchmarkId, Throughput, BatchSize};
use std::time::Duration;
use vm_runtime::{Runtime, MemoryStorage};
use once_cell::sync::OnceCell;

#[inline(always)]
fn busy_work(x: u64, iters: usize) -> i32 {
    let mut v = x;
    for _ in 0..iters { v = v.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407); }
    (v >> 33) as i32
}

fn bench_hybrid(c: &mut Criterion) {
    let mut group = c.benchmark_group("hybrid_vs_seq");
    group.sample_size(10)
        .warm_up_time(Duration::from_millis(200))
        .measurement_time(Duration::from_millis(600));

    // 支持多批量规模扫描
    let sizes = [10_000usize, 50_000usize, 100_000usize];
    let iters = 512usize;
    for &batch_size in &sizes {
        let label = format!("n{}_it{}", batch_size, iters);
        group.throughput(Throughput::Elements(batch_size as u64));

        // 1. 直接顺序 (纯函数循环，不创建 Runtime)
        group.bench_with_input(BenchmarkId::new("seq_direct", &label), &label, |b, _| {
            let data: Vec<u64> = (0..batch_size as u64).collect();
            b.iter_batched(
                || (),
                |_| {
                let mut acc = 0i64;
                for &x in &data { acc += busy_work(x, iters) as i64; }
                acc
                },
                BatchSize::SmallInput,
            )
        });

        // 2. Runtime 顺序 (使用 execute_with_hybrid 的 fallback，不初始化 hybrid)
        group.bench_with_input(BenchmarkId::new("runtime_seq", &label), &label, |b, _| {
            let rt_seq: Runtime<MemoryStorage> = Runtime::new_with_routing(MemoryStorage::default());
            let data: Vec<u64> = (0..batch_size as u64).collect();
            b.iter_batched(
                || (),
                |_| {
                let ops: Vec<(u64, Box<dyn Fn() -> i32 + Send + Sync>)> = data.iter().map(|&i| {
                    (i, Box::new(move || busy_work(i, iters)) as Box<dyn Fn() -> i32 + Send + Sync>)
                }).collect();
                let res = rt_seq.execute_with_hybrid(ops.into_iter().map(|(id,f)| (id, move || f())).collect());
                res.iter().map(|(_,v)| *v as i64).sum::<i64>()
                },
                BatchSize::SmallInput,
            )
        });

        // 3. Runtime Hybrid Dyn (原始动态分发)
        group.bench_with_input(BenchmarkId::new("runtime_hybrid_dyn", &label), &label, |b, _| {
            let mut rt_hybrid: Runtime<MemoryStorage> = Runtime::new_with_routing(MemoryStorage::default());
            rt_hybrid.init_hybrid();
            let data: Vec<u64> = (0..batch_size as u64).collect();
            b.iter_batched(
                || (),
                |_| {
                    let ops: Vec<(u64, Box<dyn Fn() -> i32 + Send + Sync>)> = data.iter().map(|&i| {
                        (i, Box::new(move || busy_work(i, iters)) as Box<dyn Fn() -> i32 + Send + Sync>)
                    }).collect();
                    let res = rt_hybrid.execute_with_hybrid(ops.into_iter().map(|(id,f)| (id, move || f())).collect());
                    res.iter().map(|(_,v)| *v as i64).sum::<i64>()
                },
                BatchSize::SmallInput,
            )
        });

        // 4. Runtime Hybrid FnPtr (低开销函数指针路径 - 使用低开销初始化)
        const FN_PTR_ITERS: usize = 512; // 固定迭代次数避免 fn 捕获环境
        #[inline(always)]
        fn bw_fnptr(x: u64) -> i32 { busy_work(x, FN_PTR_ITERS) }
        group.bench_with_input(BenchmarkId::new("runtime_hybrid_fnptr", &label), &label, |b, _| {
            let mut rt_hybrid: Runtime<MemoryStorage> = Runtime::new_with_routing(MemoryStorage::default());
            rt_hybrid.init_hybrid_low_overhead();
            let data: Vec<u64> = (0..batch_size as u64).collect();
            b.iter_batched(
                || (),
                |_| {
                    let ops: Vec<(u64, u64, fn(u64)->i32)> = data.iter().map(|&i| (i, i, bw_fnptr as fn(u64)->i32)).collect();
                    let res = rt_hybrid.execute_with_hybrid_fn(ops);
                    res.iter().map(|(_,v)| *v as i64).sum::<i64>()
                },
                BatchSize::SmallInput,
            )
        });

        // 5. Runtime Hybrid Dyn Chunked (减少任务数量以观察调度/锁开销下降效果)
        group.bench_with_input(BenchmarkId::new("runtime_hybrid_dyn_chunked", &label), &label, |b, _| {
            let mut rt_hybrid: Runtime<MemoryStorage> = Runtime::new_with_routing(MemoryStorage::default());
            rt_hybrid.init_hybrid();
            let data: Vec<u64> = (0..batch_size as u64).collect();
            let chunk_size = 1_000usize; // 可后续参数化
            let num_chunks = (batch_size + chunk_size - 1) / chunk_size;
            b.iter_batched(
                || (),
                |_| {
                    let mut ops: Vec<(u64, Box<dyn Fn() -> i32 + Send + Sync>)> = Vec::with_capacity(num_chunks);
                    for chunk_id in 0..num_chunks {
                        let start = chunk_id * chunk_size;
                        let end = (start + chunk_size).min(batch_size);
                        let chunk_vec: Vec<u64> = data[start..end].to_vec(); // 拥有数据避免借用生命周期问题
                        ops.push((chunk_id as u64, Box::new(move || {
                            let mut acc: i32 = 0;
                            for &x in &chunk_vec { acc = acc.wrapping_add(busy_work(x, iters)); }
                            acc
                        }) as Box<dyn Fn() -> i32 + Send + Sync>));
                    }
                    let res = rt_hybrid.execute_with_hybrid(ops.into_iter().map(|(id,f)| (id, move || f())).collect());
                    // 汇总时使用 i64 便于跨路径比较
                    res.iter().map(|(_,v)| *v as i64).sum::<i64>()
                },
                BatchSize::SmallInput,
            )
        });

        // 6. Runtime Hybrid FnPtr Chunked (函数指针 + 分块，公平对比)
        static CHUNK_DATA: OnceCell<Vec<u64>> = OnceCell::new();
        static CHUNK_OFFSETS: OnceCell<Vec<(usize, usize)>> = OnceCell::new();
        #[inline(always)]
        fn bw_fnptr_chunked(chunk_id: u64) -> i32 {
            let data = CHUNK_DATA.get().expect("chunk data init");
            let offsets = CHUNK_OFFSETS.get().expect("chunk offsets init");
            let (start, end) = offsets[chunk_id as usize];
            let mut acc: i32 = 0;
            for &x in &data[start..end] { acc = acc.wrapping_add(busy_work(x, FN_PTR_ITERS)); }
            acc
        }
        group.bench_with_input(BenchmarkId::new("runtime_hybrid_fnptr_chunked", &label), &label, |b, _| {
            let mut rt_hybrid: Runtime<MemoryStorage> = Runtime::new_with_routing(MemoryStorage::default());
            rt_hybrid.init_hybrid_low_overhead();
            let data: Vec<u64> = (0..batch_size as u64).collect();
            let chunk_size = 1_000usize;
            let num_chunks = (batch_size + chunk_size - 1) / chunk_size;
            // 预先构建偏移表
            let mut offsets = Vec::with_capacity(num_chunks);
            for chunk_id in 0..num_chunks {
                let start = chunk_id * chunk_size;
                let end = (start + chunk_size).min(batch_size);
                offsets.push((start, end));
            }
            // 将数据放入全局 OnceCell，避免生命周期/捕获问题
            let _ = CHUNK_DATA.get_or_init(|| data.clone());
            let _ = CHUNK_OFFSETS.get_or_init(|| offsets.clone());

            b.iter_batched(
                || (),
                |_| {
                    let ops: Vec<(u64, u64, fn(u64)->i32)> = (0..num_chunks)
                        .map(|chunk_id| (chunk_id as u64, chunk_id as u64, bw_fnptr_chunked as fn(u64)->i32))
                        .collect();
                    let res = rt_hybrid.execute_with_hybrid_fn(ops);
                    res.iter().map(|(_,v)| *v as i64).sum::<i64>()
                },
                BatchSize::SmallInput,
            )
        });

        // 7. Runtime Hybrid Auto Chunked (调用 Runtime 内置自动分块 API)
        group.bench_with_input(BenchmarkId::new("runtime_hybrid_auto_chunked", &label), &label, |b, _| {
            let mut rt_hybrid: Runtime<MemoryStorage> = Runtime::new_with_routing(MemoryStorage::default());
            rt_hybrid.init_hybrid_low_overhead();
            let data: Vec<u64> = (0..batch_size as u64).collect();
            b.iter_batched(
                || (),
                |_| {
                    let ops: Vec<(u64, u64, fn(u64)->i32)> = data.iter().map(|&i| (i, i, bw_fnptr as fn(u64)->i32)).collect();
                    let res = rt_hybrid.execute_with_hybrid_auto_chunked(ops);
                    res.iter().map(|(_,v)| *v as i64).sum::<i64>()
                },
                BatchSize::SmallInput,
            )
        });

        // 8. Chunk Size Sweep (explicit chunk size) - 500
        group.bench_with_input(BenchmarkId::new("runtime_hybrid_chunked_cs500", &label), &label, |b, _| {
            let mut rt_hybrid: Runtime<MemoryStorage> = Runtime::new_with_routing(MemoryStorage::default());
            rt_hybrid.init_hybrid_low_overhead();
            let data: Vec<u64> = (0..batch_size as u64).collect();
            let chunk_size = 500usize;
            b.iter_batched(|| (), |_| {
                let ops: Vec<(u64,u64,fn(u64)->i32)> = data.iter().map(|&i| (i, i, bw_fnptr as fn(u64)->i32)).collect();
                let res = rt_hybrid.execute_with_hybrid_chunked_with(ops, chunk_size);
                res.iter().map(|(_,v)| *v as i64).sum::<i64>()
            }, BatchSize::SmallInput)
        });

        // 9. Chunk Size Sweep - 750
        group.bench_with_input(BenchmarkId::new("runtime_hybrid_chunked_cs750", &label), &label, |b, _| {
            let mut rt_hybrid: Runtime<MemoryStorage> = Runtime::new_with_routing(MemoryStorage::default());
            rt_hybrid.init_hybrid_low_overhead();
            let data: Vec<u64> = (0..batch_size as u64).collect();
            let chunk_size = 750usize;
            b.iter_batched(|| (), |_| {
                let ops: Vec<(u64,u64,fn(u64)->i32)> = data.iter().map(|&i| (i, i, bw_fnptr as fn(u64)->i32)).collect();
                let res = rt_hybrid.execute_with_hybrid_chunked_with(ops, chunk_size);
                res.iter().map(|(_,v)| *v as i64).sum::<i64>()
            }, BatchSize::SmallInput)
        });

        // 10. Chunk Size Sweep - 1000
        group.bench_with_input(BenchmarkId::new("runtime_hybrid_chunked_cs1000", &label), &label, |b, _| {
            let mut rt_hybrid: Runtime<MemoryStorage> = Runtime::new_with_routing(MemoryStorage::default());
            rt_hybrid.init_hybrid_low_overhead();
            let data: Vec<u64> = (0..batch_size as u64).collect();
            let chunk_size = 1000usize;
            b.iter_batched(|| (), |_| {
                let ops: Vec<(u64,u64,fn(u64)->i32)> = data.iter().map(|&i| (i, i, bw_fnptr as fn(u64)->i32)).collect();
                let res = rt_hybrid.execute_with_hybrid_chunked_with(ops, chunk_size);
                res.iter().map(|(_,v)| *v as i64).sum::<i64>()
            }, BatchSize::SmallInput)
        });

        // 11. Chunk Size Sweep - 1250
        group.bench_with_input(BenchmarkId::new("runtime_hybrid_chunked_cs1250", &label), &label, |b, _| {
            let mut rt_hybrid: Runtime<MemoryStorage> = Runtime::new_with_routing(MemoryStorage::default());
            rt_hybrid.init_hybrid_low_overhead();
            let data: Vec<u64> = (0..batch_size as u64).collect();
            let chunk_size = 1250usize;
            b.iter_batched(|| (), |_| {
                let ops: Vec<(u64,u64,fn(u64)->i32)> = data.iter().map(|&i| (i, i, bw_fnptr as fn(u64)->i32)).collect();
                let res = rt_hybrid.execute_with_hybrid_chunked_with(ops, chunk_size);
                res.iter().map(|(_,v)| *v as i64).sum::<i64>()
            }, BatchSize::SmallInput)
        });

        // 12. Chunk Size Sweep - 1500
        group.bench_with_input(BenchmarkId::new("runtime_hybrid_chunked_cs1500", &label), &label, |b, _| {
            let mut rt_hybrid: Runtime<MemoryStorage> = Runtime::new_with_routing(MemoryStorage::default());
            rt_hybrid.init_hybrid_low_overhead();
            let data: Vec<u64> = (0..batch_size as u64).collect();
            let chunk_size = 1500usize;
            b.iter_batched(|| (), |_| {
                let ops: Vec<(u64,u64,fn(u64)->i32)> = data.iter().map(|&i| (i, i, bw_fnptr as fn(u64)->i32)).collect();
                let res = rt_hybrid.execute_with_hybrid_chunked_with(ops, chunk_size);
                res.iter().map(|(_,v)| *v as i64).sum::<i64>()
            }, BatchSize::SmallInput)
        });

        // 13. Chunk Size Sweep - 1750
        group.bench_with_input(BenchmarkId::new("runtime_hybrid_chunked_cs1750", &label), &label, |b, _| {
            let mut rt_hybrid: Runtime<MemoryStorage> = Runtime::new_with_routing(MemoryStorage::default());
            rt_hybrid.init_hybrid_low_overhead();
            let data: Vec<u64> = (0..batch_size as u64).collect();
            let chunk_size = 1750usize;
            b.iter_batched(|| (), |_| {
                let ops: Vec<(u64,u64,fn(u64)->i32)> = data.iter().map(|&i| (i, i, bw_fnptr as fn(u64)->i32)).collect();
                let res = rt_hybrid.execute_with_hybrid_chunked_with(ops, chunk_size);
                res.iter().map(|(_,v)| *v as i64).sum::<i64>()
            }, BatchSize::SmallInput)
        });

        // 14. Chunk Size Sweep - 2000
        group.bench_with_input(BenchmarkId::new("runtime_hybrid_chunked_cs2000", &label), &label, |b, _| {
            let mut rt_hybrid: Runtime<MemoryStorage> = Runtime::new_with_routing(MemoryStorage::default());
            rt_hybrid.init_hybrid_low_overhead();
            let data: Vec<u64> = (0..batch_size as u64).collect();
            let chunk_size = 2000usize;
            b.iter_batched(|| (), |_| {
                let ops: Vec<(u64,u64,fn(u64)->i32)> = data.iter().map(|&i| (i, i, bw_fnptr as fn(u64)->i32)).collect();
                let res = rt_hybrid.execute_with_hybrid_chunked_with(ops, chunk_size);
                res.iter().map(|(_,v)| *v as i64).sum::<i64>()
            }, BatchSize::SmallInput)
        });
    } // end for sizes

    group.finish();
}

criterion_group!(benches, bench_hybrid);
criterion_main!(benches);
