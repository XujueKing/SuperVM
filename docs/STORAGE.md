# Storage Layer Documentation

## 概述

SuperVM 存储层提供灵活的存储抽象,支持多种后端实现:
- **MemoryStorage**: 内存存储 (BTreeMap, 测试用)
- **RocksDBStorage**: 持久化存储 (RocksDB, 生产用)

本文档重点介绍 **RocksDBStorage** 的配置、管理、优化和最佳实践。

---

## RocksDB 配置指南

### 基本配置

```rust
use vm_runtime::{RocksDBConfig, RocksDBStorage};

let config = RocksDBConfig::default()
    .with_path("data/rocksdb")
    .with_create_if_missing(true)
    .with_max_open_files(1000)
    .with_write_buffer_size(64 * 1024 * 1024)  // 64MB
    .with_max_write_buffer_number(3)
    .with_target_file_size_base(64 * 1024 * 1024);  // 64MB

let storage = RocksDBStorage::new(config)?;
```

### 高级配置参数

| 参数 | 默认值 | 说明 | 推荐值 |
|------|--------|------|--------|
| `write_buffer_size` | 64MB | MemTable 大小 | 高吞吐: 128-256MB |
| `max_write_buffer_number` | 3 | 并发 MemTable 数量 | 写密集: 4-6 |
| `target_file_size_base` | 64MB | L1 SST 文件大小 | 大数据集: 128-256MB |
| `max_background_jobs` | 2 | 后台 compaction/flush 线程 | 多核: 4-8 |
| `block_cache_size` | 8MB | Block Cache 大小 | 读密集: 512MB-2GB |
| `enable_statistics` | false | 启用内部统计 | 生产: true (监控用) |

---

## Checkpoint 管理

### 自动检查点创建

```rust
use vm_runtime::CheckpointManager;

// 自动检查点管理 (每1000个区块创建一次)
storage.maybe_create_snapshot(block_num, &SnapshotConfig {
    snapshot_interval: 1000,
    keep_recent: 10,
    base_path: "data/snapshots".into(),
})?;

// 手动创建检查点
storage.create_checkpoint("checkpoint_2025_11_11")?;
```

### 检查点恢复

```rust
// 列出所有检查点
let checkpoints = RocksDBStorage::list_checkpoints("data/snapshots")?;

// 从检查点恢复
let storage = RocksDBStorage::restore_from_checkpoint(
    &checkpoints[0],
    RocksDBConfig::default()
)?;
```

### 检查点清理策略

```rust
// 清理旧快照 (保留最近10个)
storage.cleanup_old_snapshots(&SnapshotConfig {
    keep_recent: 10,
    ..Default::default()
})?;
```

---

## AutoFlush 机制

### MVCC 自动刷新配置

```rust
use vm_runtime::{AutoFlushConfig, MvccStore};

let flush_config = AutoFlushConfig {
    interval_secs: 60,          // 每60秒刷新一次
    blocks_per_flush: 100,      // 或每100个区块刷新一次
    keep_recent_versions: 10,   // 保留最近10个版本在内存
    flush_on_start: true,       // 启动时立即刷新一次
};

mvcc.start_auto_flush(flush_config, storage.clone())?;
```

### 手动刷新

```rust
// 手动触发刷新 (保留最近5个版本)
let (keys_flushed, bytes_flushed) = mvcc.manual_flush(&mut storage, 5)?;

println!("Flushed {} keys, {} bytes", keys_flushed, bytes_flushed);
```

### 刷新统计

```rust
let flush_stats = mvcc.get_flush_stats();

println!("Flush count: {}", flush_stats.flush_count);
println!("Keys flushed: {}", flush_stats.keys_flushed);
println!("Bytes flushed: {} MB", flush_stats.bytes_flushed / 1024 / 1024);
println!("Last flush block: {}", flush_stats.last_flush_block);
```

---

## 状态裁剪策略

### 裁剪策略选择

1. **保留最近 N 个版本** (推荐)
   ```rust
   let (versions_cleaned, keys_affected) = mvcc.prune_old_versions(10, &storage);
   ```

2. **基于时间窗口裁剪**
   ```rust
   // 保留最近7天的版本
   let cutoff_ts = current_ts - (7 * 24 * 3600);
   // (需自定义实现时间戳过滤逻辑)
   ```

3. **基于区块高度裁剪**
   ```rust
   // 保留最近1000个区块的版本
   let cutoff_block = current_block - 1000;
   // (需自定义实现区块高度过滤逻辑)
   ```

### 自动裁剪集成

```rust
// 在 AutoFlush 回调中集成裁剪
mvcc.start_auto_flush(AutoFlushConfig {
    interval_secs: 300,  // 每5分钟
    ..Default::default()
}, storage.clone())?;

// 后台线程定期裁剪
std::thread::spawn(move || {
    loop {
        std::thread::sleep(Duration::from_secs(600)); // 每10分钟
        let (cleaned, _) = mvcc.prune_old_versions(10, &storage);
        if cleaned > 0 {
            println!("✂️ Pruned {} versions", cleaned);
        }
    }
});
```

---

## Prometheus 指标监控

### RocksDB 内部指标

```rust
// 采集 RocksDB 内部指标
let rocksdb_metrics = storage.collect_metrics();

// 同步到 MetricsCollector
mvcc.update_rocksdb_metrics(&rocksdb_metrics);

// 导出 Prometheus 格式
if let Some(metrics) = mvcc.get_metrics() {
    let prometheus_output = metrics.export_prometheus();
    // HTTP /metrics 端点返回 prometheus_output
}
```

### 关键指标说明

| 指标名称 | 类型 | 说明 | 告警阈值 |
|---------|------|------|----------|
| `rocksdb_estimate_num_keys` | gauge | 估计键数量 | - |
| `rocksdb_total_sst_size_bytes` | gauge | SST 文件总大小 | >10GB 考虑裁剪 |
| `rocksdb_cache_hit_rate` | gauge | Block Cache 命中率 | <80% 增大cache |
| `rocksdb_compaction_cpu_micros` | counter | Compaction CPU时间 | 增长过快调整配置 |
| `rocksdb_write_stall_micros` | counter | 写入停顿时间 | >1000ms 优化写入 |
| `rocksdb_num_files_level0` | gauge | Level 0 文件数 | >10 触发compaction |
| `rocksdb_num_immutable_mem_table` | gauge | Immutable MemTable数 | >3 写入压力大 |

### Grafana Dashboard 配置

```json
{
  "panels": [
    {
      "title": "RocksDB Cache Hit Rate",
      "targets": [
        {
          "expr": "rocksdb_cache_hit_rate"
        }
      ],
      "alert": {
        "conditions": [
          {
            "evaluator": { "params": [80], "type": "lt" },
            "query": { "params": ["A", "5m", "now"] }
          }
        ]
      }
    },
    {
      "title": "Write Stall Latency",
      "targets": [
        {
          "expr": "rate(rocksdb_write_stall_micros[5m]) / 1000"
        }
      ]
    }
  ]
}
```

---

## 性能调优建议

### 1. 写入优化

**问题**: 写入停顿 (`write_stall_micros` 高)

**解决方案**:
```rust
RocksDBConfig::default()
    .with_max_write_buffer_number(6)          // 增加并发MemTable
    .with_min_write_buffer_number_to_merge(2) // 减少合并阈值
    .with_max_background_jobs(8)              // 增加后台线程
```

### 2. 读取优化

**问题**: Cache 命中率低 (`rocksdb_cache_hit_rate` < 80%)

**解决方案**:
```rust
RocksDBConfig::default()
    .with_block_cache_size(2 * 1024 * 1024 * 1024)  // 增大cache到2GB
    .with_bloom_filter_bits_per_key(10)              // 启用Bloom Filter
```

### 3. Compaction 优化

**问题**: Level 0 文件过多 (`rocksdb_num_files_level0` > 10)

**解决方案**:
```rust
RocksDBConfig::default()
    .with_level0_file_num_compaction_trigger(4)  // 更早触发compaction
    .with_max_bytes_for_level_base(256 * 1024 * 1024)  // 增大L1大小
```

### 4. 空间优化

**问题**: SST 文件过大 (`rocksdb_total_sst_size_bytes` > 10GB)

**解决方案**:
- 定期执行状态裁剪 `prune_old_versions(10, &storage)`
- 启用 LZ4 压缩: `with_compression_type(CompressionType::Lz4)`
- 减少 `max_versions_per_key` 配置

---

## 故障恢复流程

### 1. 数据损坏恢复

```rust
// 尝试修复数据库
RocksDBStorage::repair("data/rocksdb")?;

// 如果修复失败,从最近检查点恢复
let checkpoints = RocksDBStorage::list_checkpoints("data/snapshots")?;
let storage = RocksDBStorage::restore_from_checkpoint(
    &checkpoints[0],
    RocksDBConfig::default()
)?;
```

### 2. 写入停顿处理

```bash
# 检查告警
curl http://localhost:9090/metrics | grep write_stall

# 紧急措施: 停止写入,手动触发 compaction
# (RocksDB 会自动触发,但可能需要时间)

# 长期方案: 调整配置参数
```

### 3. 磁盘空间不足

```bash
# 1. 检查 SST 文件大小
du -sh data/rocksdb

# 2. 执行状态裁剪
cargo run --example state_pruning_demo --features rocksdb-storage

# 3. 清理旧检查点
# (保留最近5个)
```

---

## 最佳实践

### 生产环境配置

```rust
let config = RocksDBConfig::default()
    .with_path("/var/lib/supervm/rocksdb")
    .with_create_if_missing(true)
    .with_write_buffer_size(128 * 1024 * 1024)  // 128MB
    .with_max_write_buffer_number(4)
    .with_target_file_size_base(128 * 1024 * 1024)
    .with_max_background_jobs(8)
    .with_block_cache_size(2 * 1024 * 1024 * 1024)  // 2GB
    .with_enable_statistics(true)  // 启用监控
    .with_bloom_filter_bits_per_key(10);
```

### 监控检查清单

- [ ] `rocksdb_cache_hit_rate` >= 80%
- [ ] `rocksdb_write_stall_micros` < 100ms/分钟
- [ ] `rocksdb_num_files_level0` < 8
- [ ] `rocksdb_num_immutable_mem_table` < 3
- [ ] 定期检查 SST 文件大小增长趋势
- [ ] 每周执行一次状态裁剪
- [ ] 每天创建检查点快照

### 容量规划

| 负载类型 | 预估 TPS | 建议配置 | 磁盘空间 |
|---------|---------|---------|---------|
| 轻量级 | <1000 | 默认配置 | 10-50GB |
| 中等负载 | 1K-10K | write_buffer_size=128MB | 50-200GB |
| 高负载 | 10K-100K | write_buffer_size=256MB, cache=4GB | 200GB-1TB |
| 超高负载 | >100K | 分片存储, 多RocksDB实例 | 1TB+ |

---

## 示例代码

### 完整初始化流程

```rust
use vm_runtime::{
    RocksDBConfig, RocksDBStorage, MvccStore, GcConfig, AutoFlushConfig
};

fn initialize_storage() -> anyhow::Result<(RocksDBStorage, Arc<MvccStore>)> {
    // 1. 创建 RocksDB 存储
    let rocksdb_config = RocksDBConfig::default()
        .with_path("data/production")
        .with_write_buffer_size(128 * 1024 * 1024)
        .with_enable_statistics(true);
    
    let mut storage = RocksDBStorage::new(rocksdb_config)?;
    
    // 2. 创建 MVCC Store
    let gc_config = GcConfig {
        max_versions_per_key: 10,
        enable_time_based_gc: false,
        version_ttl_secs: 3600,
        auto_gc: None,
    };
    let mvcc = Arc::new(MvccStore::new_with_config(gc_config));
    
    // 3. 启动自动刷新
    mvcc.start_auto_flush(AutoFlushConfig {
        interval_secs: 60,
        blocks_per_flush: 100,
        keep_recent_versions: 10,
        flush_on_start: true,
    }, Arc::new(Mutex::new(storage.clone())))?;
    
    // 4. 启动定期裁剪
    let mvcc_clone = mvcc.clone();
    let storage_clone = storage.clone();
    std::thread::spawn(move || {
        loop {
            std::thread::sleep(Duration::from_secs(600));
            let (cleaned, _) = mvcc_clone.prune_old_versions(10, &storage_clone);
            println!("✂️ Pruned {} versions", cleaned);
        }
    });
    
    Ok((storage, mvcc))
}
```

### 指标监控示例

```rust
// 周期性采集指标
loop {
    std::thread::sleep(Duration::from_secs(30));
    
    // 采集 RocksDB 指标
    let metrics = storage.collect_metrics();
    mvcc.update_rocksdb_metrics(&metrics);
    
    // 导出 Prometheus 格式
    if let Some(collector) = mvcc.get_metrics() {
        let prom = collector.export_prometheus();
        // 通过 HTTP /metrics 端点暴露
        serve_metrics(prom);
    }
}
```

---

## 参考资料

- [RocksDB Wiki](https://github.com/facebook/rocksdb/wiki)
- [RocksDB Tuning Guide](https://github.com/facebook/rocksdb/wiki/RocksDB-Tuning-Guide)
- [Prometheus Metrics Best Practices](https://prometheus.io/docs/practices/naming/)

---

**最后更新**: 2025-11-11  
**维护者**: XujueKing <leadbrand@me.com>
