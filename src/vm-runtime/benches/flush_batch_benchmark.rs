// SPDX-License-Identifier: GPL-3.0-or-later
// Benchmark: MVCC manual flush to RocksDB (batch write path)

use criterion::{criterion_group, criterion_main, Criterion, BatchSize};
use tempfile::tempdir;
use vm_runtime::{GcConfig, MvccStore, RocksDBConfig, RocksDBStorage, Storage};

fn prepare_store(n: usize) -> (MvccStoreHandle, RocksDBStorage) {
    let store = MvccStore::new_with_config(GcConfig::default());
    for i in 0..n {
        let mut tx = store.begin();
        let k = format!("k{:06}", i).into_bytes();
        let v = format!("v{:06}", i).into_bytes();
        tx.write(k, v);
        let _ = tx.commit();
    }
    let dir = tempdir().unwrap();
    let mut rocks = RocksDBStorage::new(
        RocksDBConfig::default().with_path(dir.path().to_string_lossy().to_string())
    ).unwrap();
    (MvccStoreHandle(store), rocks)
}

struct MvccStoreHandle(vm_runtime::MvccStore);
impl std::ops::Deref for MvccStoreHandle { type Target = vm_runtime::MvccStore; fn deref(&self) -> &Self::Target { &self.0 } }

fn bench_flush_batch(c: &mut Criterion) {
    c.bench_function("mvcc_manual_flush_100k_keys", |b| {
        b.iter_batched(
            || prepare_store(100_000),
            |(store, mut rocks)| {
                let _ = store.manual_flush(&mut rocks, 3);
            },
            BatchSize::PerIteration,
        );
    });
}

criterion_group!(flush_batch, bench_flush_batch);
criterion_main!(flush_batch);
