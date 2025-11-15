# Phase 4.3 Week 2 - 快速参考

## 🎯 本周目标

**配置优化 + 批量操作 + 监控 + 基准测试**

## 📅 时间安排

- **Day 1-2**: WriteBatch 高级优化

- **Day 3-4**: 配置调优 + 监控指标

- **Day 5-7**: 性能基准测试套件

## ⚡ 核心任务速查

### Task 1: WriteBatch 优化

```rust
// 目标: 200K+ ops/s

// 1. 预分配容量
let mut batch = WriteBatch::with_capacity(batch_size);

// 2. 事务语义
pub fn begin_batch() -> BatchTransaction;
pub fn commit_batch(tx: BatchTransaction) -> Result<()>;

// 3. 测试矩阵
[100, 1K, 10K, 100K] 条记录

```

### Task 2: 压缩策略

```rust
// 对比算法

- LZ4    (快速)

- Snappy (平衡)

- Zstd   (高压缩比)

- None   (基线)

// 目标: 压缩比 ≥ 2x

```

### Task 3: 配置调优

```rust
// Block Cache: 512MB → 1GB (测试)
// Write Buffer: 128MB → 256MB (测试)
// Background Jobs: 4 → 8 (测试)

```

### Task 4: 监控 API

```rust
pub struct RocksDBStats {
    pub read_qps: f64,
    pub write_qps: f64,
    pub cache_hit_rate: f64,
    pub compression_ratio: f64,
    pub p99_latency_us: f64,
}

storage.get_stats()?;
storage.print_stats()?;

```

### Task 5: 基准测试

```bash

# Criterion 集成

cargo bench --bench rocksdb_benchmark

# 生成 HTML 报告

target/criterion/report/index.html

```

## 🎯 验收标准

- ✅ 随机读 ≥ 100K ops/s

- ✅ 批量写 ≥ 200K ops/s

- ✅ 压缩比 ≥ 2x

- ✅ P99 延迟 < 10ms

- ✅ Criterion 报告完整

## 📝 快速命令

```powershell

# 编译带 RocksDB 特性

cargo build -p vm-runtime --features rocksdb-storage --release

# 运行压缩基准测试

cargo run -p node-core --example rocksdb_compression_bench --release

# 运行监控示例

cargo run -p node-core --example rocksdb_monitor --release

# 运行 Criterion 基准

cargo bench --bench rocksdb_benchmark

# 运行性能测试

cargo test -p vm-runtime --features rocksdb-storage test_performance --release

```

## 📊 需要创建的文件

Week 2 新增文件:

- [ ] `examples/rocksdb_compression_bench.rs` - 压缩算法对比

- [ ] `examples/rocksdb_monitor.rs` - 实时监控

- [ ] `benches/rocksdb_benchmark.rs` - Criterion 基准

- [ ] `tests/performance_test.rs` - 性能测试

- [ ] `docs/rocksdb-tuning-guide.md` - 调优指南

## 🔧 关键代码片段

### WriteBatch 优化

```rust
impl RocksDBStorage {
    pub fn write_batch_optimized(&self, 
        batch: Vec<(Vec<u8>, Option<Vec<u8>>)>) -> Result<()> {
        let mut write_batch = WriteBatch::with_capacity(batch.len());
        
        for (key, value) in batch {
            match value {
                Some(v) => write_batch.put(&key, &v),
                None => write_batch.delete(&key),
            }
        }
        
        self.db.write(write_batch)?;
        Ok(())
    }
}

```

### 压缩配置

```rust
opts.set_compression_type(DBCompressionType::Lz4);
opts.set_compression_options(0, 0, 0, 1024);

```

### 监控 API

```rust
pub fn get_stats(&self) -> Result<RocksDBStats> {
    let cache_hit = self.get_property_u64("rocksdb.block.cache.hit")?;
    let cache_miss = self.get_property_u64("rocksdb.block.cache.miss")?;
    let hit_rate = cache_hit as f64 / (cache_hit + cache_miss) as f64;
    // ...
}

```

### Criterion 基准

```rust
fn batch_write_benchmark(c: &mut Criterion) {
    c.bench_function("rocksdb_batch_100k", |b| {
        b.iter(|| {
            // 测试代码
        });
    });
}

```

## 💡 优化技巧

1. **WriteBatch**: 预分配容量,减少扩容
2. **压缩**: 热数据无压缩,冷数据 Zstd
3. **缓存**: 内存 * 0.3 ~ 0.5 作为 block cache
4. **后台任务**: CPU 核心数 * 2
5. **Write Buffer**: 增大可提升写入,但占内存

## ⚠️ 注意事项

- 压缩会增加 CPU 占用

- 缓存过大导致内存不足

- 后台任务过多导致竞争

- SSD vs HDD 配置差异大

## 📈 预期成果

Week 2 结束时:

- ✅ WriteBatch 性能优化 (200K+ ops/s)

- ✅ 最优配置组合确定

- ✅ 完整监控指标

- ✅ Criterion 基准报告

- ✅ 性能调优文档

---

**详细任务清单**: `docs/PHASE-4.3-WEEK2-TASKS.md`  
**实施文档**: `docs/PHASE-4.3-ROCKSDB-INTEGRATION.md`
