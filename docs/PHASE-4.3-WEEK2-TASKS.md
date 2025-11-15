# Phase 4.3 - Week 2: 配置优化与批量操作

**时间**: Week 2 (预计 11月11日-11月17日)  
**目标**: 性能调优、WriteBatch 优化、监控指标集成  
**预期完成度**: 40% → 60%

---

## 📋 任务清单

### 🎯 核心任务

#### 1. WriteBatch 高级优化 (2天)

**当前状态**: ✅ 基础实现完成
**优化目标**: 提升批量写入性能至 200K+ ops/s

**任务清单**:

- [ ] **批量大小优化**
  - [ ] 测试不同批量大小 (100, 500, 1K, 5K, 10K)
  - [ ] 找到最优批量大小阈值
  - [ ] 实现自适应批量大小
  
- [ ] **内存预分配**
  - [ ] 为 WriteBatch 预分配容量
  - [ ] 减少动态扩容开销
  
- [ ] **事务语义增强**
  - [ ] 实现 `begin_batch()` / `commit_batch()` API
  - [ ] 支持批量回滚 (rollback)
  - [ ] 错误处理优化
  
- [ ] **性能测试**
  - [ ] 小批量测试 (100条)
  - [ ] 中批量测试 (1K条)
  - [ ] 大批量测试 (10K条)
  - [ ] 巨量测试 (100K条)
  - [ ] 对比单条写入 vs 批量写入

**代码示例**:

```rust
// src/vm-runtime/src/storage/rocksdb_storage.rs

impl RocksDBStorage {
    /// 带预分配的批量写入
    pub fn write_batch_optimized(&self, batch: Vec<(Vec<u8>, Option<Vec<u8>>)>) -> Result<()> {
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
    
    /// 事务式批量操作
    pub struct BatchTransaction {
        batch: WriteBatch,
        committed: bool,
    }
    
    pub fn begin_batch(&self) -> BatchTransaction {
        BatchTransaction {
            batch: WriteBatch::default(),
            committed: false,
        }
    }
}

```

**验收标准**:

- ✅ 100K条批量写入 < 0.5秒

- ✅ 批量 QPS ≥ 200K ops/s

- ✅ 内存占用稳定 (无泄漏)

---

#### 2. 配置调优与压缩策略 (1.5天)

**目标**: 找到最优配置组合,最大化吞吐量和压缩比

**任务清单**:

- [ ] **压缩算法对比**
  - [ ] LZ4 压缩 (默认,快速)
  - [ ] Snappy 压缩 (平衡)
  - [ ] Zstd 压缩 (高压缩比)
  - [ ] 无压缩 (基线)
  - [ ] 对比测试: 写入QPS、压缩比、磁盘占用
  
- [ ] **Block Cache 调优**
  - [ ] 测试不同缓存大小 (256MB, 512MB, 1GB, 2GB)
  - [ ] 缓存命中率监控
  - [ ] 找到最佳缓存/内存比例
  
- [ ] **Write Buffer 调优**
  - [ ] 测试不同缓冲区大小 (64MB, 128MB, 256MB, 512MB)
  - [ ] Flush 频率分析
  - [ ] 内存 vs 写入速度权衡
  
- [ ] **后台任务配置**
  - [ ] max_background_jobs (2, 4, 8, 16)
  - [ ] max_background_compactions
  - [ ] 压缩开销 vs CPU 利用率
  
- [ ] **多级压缩策略**
  - [ ] L0-L1: 无压缩 (热数据)
  - [ ] L2-L4: LZ4 (温数据)
  - [ ] L5+: Zstd (冷数据)

**代码示例**:

```rust
// 压缩策略配置
pub struct CompressionStrategy {
    pub level_0: DBCompressionType,
    pub level_1: DBCompressionType,
    pub level_2_plus: DBCompressionType,
}

impl RocksDBConfig {
    pub fn with_compression_strategy(mut self, strategy: CompressionStrategy) -> Self {
        self.compression_strategy = Some(strategy);
        self
    }
    
    pub fn production_optimized() -> Self {
        Self {
            path: "./data/rocksdb".to_string(),
            max_open_files: 10000,
            write_buffer_size: 256 * 1024 * 1024,  // 256MB (提升)
            block_cache_size: 1024 * 1024 * 1024,  // 1GB (提升)
            enable_compression: true,
            max_background_jobs: 8,  // 提升
            compression_strategy: Some(CompressionStrategy {
                level_0: DBCompressionType::None,
                level_1: DBCompressionType::Lz4,
                level_2_plus: DBCompressionType::Zstd,
            }),
        }
    }
}

```

**测试脚本**:

```rust
// examples/rocksdb_compression_bench.rs
fn main() -> Result<()> {
    let strategies = vec![
        ("LZ4", DBCompressionType::Lz4),
        ("Snappy", DBCompressionType::Snappy),
        ("Zstd", DBCompressionType::Zstd),
        ("None", DBCompressionType::None),
    ];
    
    for (name, compression) in strategies {
        println!("Testing compression: {}", name);
        
        let config = RocksDBConfig {
            compression,
            ..Default::default()
        };
        
        let mut storage = RocksDBStorage::new(config)?;
        
        // 写入 100K 条记录
        let start = Instant::now();
        for i in 0..100_000 {
            storage.set(&format!("key_{}", i).into_bytes(), 
                       &vec![0u8; 1024])?;  // 1KB value
        }
        let duration = start.elapsed();
        
        // 统计磁盘占用
        let disk_usage = get_directory_size(&config.path)?;
        
        println!("  Time: {:?}", duration);
        println!("  QPS: {:.2}", 100_000.0 / duration.as_secs_f64());
        println!("  Disk: {} MB", disk_usage / 1024 / 1024);
        println!("  Compression ratio: {:.2}x\n", 
                 (100_000 * 1024) as f64 / disk_usage as f64);
    }
    
    Ok(())
}

```

**验收标准**:

- ✅ 找到最优压缩算法 (性能 vs 压缩比)

- ✅ 压缩比 ≥ 2x

- ✅ 配置文档更新

---

#### 3. 监控指标集成 (1.5天)

**目标**: 实现完整的性能监控和可观测性

**任务清单**:

- [ ] **RocksDB 统计信息**
  - [ ] `rocksdb.stats` - 总体统计
  - [ ] `rocksdb.block.cache.hit` - 缓存命中率
  - [ ] `rocksdb.block.cache.miss` - 缓存未命中
  - [ ] `rocksdb.compaction.times.micros` - 压缩耗时
  - [ ] `rocksdb.write.stall` - 写入停顿
  
- [ ] **性能指标结构**
  - [ ] 读取 QPS
  - [ ] 写入 QPS
  - [ ] 批量写入 QPS
  - [ ] 平均延迟
  - [ ] P99 延迟
  - [ ] 缓存命中率
  - [ ] 磁盘占用
  - [ ] 压缩比
  
- [ ] **实时监控 API**
  - [ ] `get_stats()` - 获取统计信息
  - [ ] `get_performance_metrics()` - 性能指标
  - [ ] `print_stats()` - 打印统计
  
- [ ] **监控示例**
  - [ ] 实时监控 dashboard
  - [ ] 性能趋势图表
  - [ ] 告警阈值设置

**代码示例**:

```rust
// src/vm-runtime/src/storage/rocksdb_storage.rs

#[derive(Debug, Clone)]
pub struct RocksDBStats {
    // 基础统计
    pub num_keys: u64,
    pub total_size: u64,
    
    // 缓存统计
    pub cache_hit_count: u64,
    pub cache_miss_count: u64,
    pub cache_hit_rate: f64,
    
    // 性能统计
    pub read_qps: f64,
    pub write_qps: f64,
    pub avg_read_latency_us: f64,
    pub avg_write_latency_us: f64,
    pub p99_latency_us: f64,
    
    // 压缩统计
    pub num_compactions: u64,
    pub compaction_time_sec: f64,
    pub compression_ratio: f64,
    
    // 磁盘统计
    pub disk_usage_bytes: u64,
    pub num_files: u32,
}

impl RocksDBStorage {
    /// 获取统计信息
    pub fn get_stats(&self) -> Result<RocksDBStats> {
        let num_keys = self.get_property_u64("rocksdb.estimate-num-keys")?;
        let total_size = self.get_property_u64("rocksdb.total-sst-files-size")?;
        
        let cache_hit = self.get_property_u64("rocksdb.block.cache.hit")?;
        let cache_miss = self.get_property_u64("rocksdb.block.cache.miss")?;
        let cache_hit_rate = cache_hit as f64 / (cache_hit + cache_miss) as f64;
        
        Ok(RocksDBStats {
            num_keys,
            total_size,
            cache_hit_count: cache_hit,
            cache_miss_count: cache_miss,
            cache_hit_rate,
            // ... 其他字段
        })
    }
    
    /// 打印统计信息
    pub fn print_stats(&self) -> Result<()> {
        let stats = self.get_stats()?;
        
        println!("=== RocksDB Statistics ===");
        println!("Keys: {}", stats.num_keys);
        println!("Total Size: {} MB", stats.total_size / 1024 / 1024);
        println!("Cache Hit Rate: {:.2}%", stats.cache_hit_rate * 100.0);
        println!("Read QPS: {:.2}", stats.read_qps);
        println!("Write QPS: {:.2}", stats.write_qps);
        println!("P99 Latency: {:.2} ms", stats.p99_latency_us / 1000.0);
        println!("Compression Ratio: {:.2}x", stats.compression_ratio);
        
        Ok(())
    }
    
    fn get_property_u64(&self, property: &str) -> Result<u64> {
        self.db.property_value(property)?
            .and_then(|s| s.parse().ok())
            .ok_or_else(|| anyhow!("Failed to get property: {}", property))
    }
}

```

**监控示例**:

```rust
// examples/rocksdb_monitor.rs
fn main() -> Result<()> {
    let storage = RocksDBStorage::new_default()?;
    
    // 启动监控循环
    loop {
        storage.print_stats()?;
        std::thread::sleep(Duration::from_secs(5));
    }
}

```

**验收标准**:

- ✅ 完整的统计信息 API

- ✅ 实时监控示例

- ✅ 性能指标文档

---

#### 4. 性能基准测试套件 (2天)

**目标**: 建立完整的性能基准测试,验证目标达成

**任务清单**:

- [ ] **基准测试框架**
  - [ ] 使用 Criterion 集成
  - [ ] 多场景测试矩阵
  - [ ] 自动化报告生成
  
- [ ] **读取基准**
  - [ ] 顺序读取
  - [ ] 随机读取
  - [ ] 范围扫描
  - [ ] 缓存命中 vs 未命中
  
- [ ] **写入基准**
  - [ ] 顺序写入
  - [ ] 随机写入
  - [ ] 批量写入 (不同批量大小)
  - [ ] 并发写入
  
- [ ] **混合负载**
  - [ ] 80% 读 + 20% 写
  - [ ] 50% 读 + 50% 写
  - [ ] YCSB 工作负载模拟
  
- [ ] **压力测试**
  - [ ] 持续写入 1小时
  - [ ] 持续读取 1小时
  - [ ] 混合负载 24小时

**代码示例**:

```rust
// benches/rocksdb_benchmark.rs

use criterion::{black_box, criterion_group, criterion_main, Criterion, BenchmarkId};
use vm_runtime::{RocksDBStorage, Storage};

fn random_write_benchmark(c: &mut Criterion) {
    let mut group = c.benchmark_group("rocksdb_write");
    
    for size in [100, 1_000, 10_000, 100_000].iter() {
        group.bench_with_input(BenchmarkId::from_parameter(size), size, |b, &size| {
            let mut storage = RocksDBStorage::new_with_path("./bench_db").unwrap();
            
            b.iter(|| {
                for i in 0..size {
                    let key = format!("key_{}", i).into_bytes();
                    let value = format!("value_{}", i).into_bytes();
                    storage.set(&key, &value).unwrap();
                }
            });
        });
    }
    
    group.finish();
}

fn batch_write_benchmark(c: &mut Criterion) {
    let mut group = c.benchmark_group("rocksdb_batch_write");
    
    for size in [100, 1_000, 10_000, 100_000].iter() {
        group.bench_with_input(BenchmarkId::from_parameter(size), size, |b, &size| {
            let storage = RocksDBStorage::new_with_path("./bench_db").unwrap();
            
            b.iter(|| {
                let mut batch = Vec::new();
                for i in 0..*size {
                    let key = format!("batch_key_{}", i).into_bytes();
                    let value = format!("batch_value_{}", i).into_bytes();
                    batch.push((key, Some(value)));
                }
                storage.write_batch(batch).unwrap();
            });
        });
    }
    
    group.finish();
}

fn random_read_benchmark(c: &mut Criterion) {
    let storage = setup_benchmark_data();
    
    c.bench_function("rocksdb_random_read", |b| {
        b.iter(|| {
            for i in 0..10_000 {
                let key = format!("key_{}", i).into_bytes();
                black_box(storage.get(&key).unwrap());
            }
        });
    });
}

criterion_group!(benches, random_write_benchmark, batch_write_benchmark, random_read_benchmark);
criterion_main!(benches);

```

**性能目标验证**:

```rust
// tests/performance_test.rs

#[test]
fn test_random_read_performance() {
    let mut storage = setup_test_storage();
    
    // 预填充数据
    for i in 0..100_000 {
        storage.set(&format!("key_{}", i).into_bytes(), b"value").unwrap();
    }
    
    // 测试随机读
    let start = Instant::now();
    for i in 0..100_000 {
        storage.get(&format!("key_{}", i).into_bytes()).unwrap();
    }
    let duration = start.elapsed();
    let qps = 100_000.0 / duration.as_secs_f64();
    
    assert!(qps >= 100_000.0, "Random read QPS {} < 100K target", qps);
}

#[test]
fn test_batch_write_performance() {
    let storage = setup_test_storage();
    
    let mut batch = Vec::new();
    for i in 0..100_000 {
        batch.push((format!("key_{}", i).into_bytes(), Some(b"value".to_vec())));
    }
    
    let start = Instant::now();
    storage.write_batch(batch).unwrap();
    let duration = start.elapsed();
    let qps = 100_000.0 / duration.as_secs_f64();
    
    assert!(qps >= 200_000.0, "Batch write QPS {} < 200K target", qps);
}

```

**验收标准**:

- ✅ Criterion 基准测试集成

- ✅ 完整的性能报告

- ✅ HTML 报告生成

- ✅ 所有目标达成验证

---

## 📊 验收标准总结

### 性能目标

- ✅ 随机读: ≥ 100K ops/s

- ✅ 批量写: ≥ 200K ops/s

- ✅ 压缩比: ≥ 2x

- ✅ P99 延迟: < 10ms

### 代码质量

- ✅ 单元测试覆盖 ≥ 90%

- ✅ 无编译警告

- ✅ 无内存泄漏

- ✅ 文档完整

### 交付物

- ✅ WriteBatch 优化实现

- ✅ 配置调优指南

- ✅ 监控指标 API

- ✅ Criterion 基准测试

- ✅ 性能测试报告

---

## 🎯 Week 2 时间分配

| 任务 | 时间 | 优先级 |
|------|------|--------|
| WriteBatch 优化 | 2天 | 🔴 高 |
| 配置调优 | 1.5天 | 🔴 高 |
| 监控指标 | 1.5天 | 🟡 中 |
| 基准测试 | 2天 | 🔴 高 |

**总计**: 7天 (1周工作量)

---

## 📝 开发笔记

### 关键优化点

1. **WriteBatch 预分配**: 避免动态扩容
2. **压缩分级**: 热数据无压缩,冷数据高压缩
3. **缓存调优**: 找到最佳缓存大小
4. **后台任务**: CPU核心数 * 2

### 常见陷阱

- ⚠️ 写缓冲区过大导致内存溢出

- ⚠️ 后台任务过多导致 CPU 竞争

- ⚠️ 压缩比过高导致读取变慢

- ⚠️ 缓存过小导致频繁磁盘访问

### 性能调优技巧

- 💡 使用 SSD: 性能提升 10-100x

- 💡 关闭 WAL: 批量写入时可选

- 💡 调整 bloom filter 参数

- 💡 使用 column families 分离热冷数据

---

## 🔗 相关资源

- RocksDB Tuning Guide: https://github.com/facebook/rocksdb/wiki/RocksDB-Tuning-Guide

- Performance Benchmarks: https://github.com/facebook/rocksdb/wiki/Performance-Benchmarks

- Compression: https://github.com/facebook/rocksdb/wiki/Compression

---

**准备完成! 等待 Week 1 编译完成后即可开始 Week 2 开发** 🚀
