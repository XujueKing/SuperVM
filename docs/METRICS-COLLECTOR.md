# MVCC Metrics 收集器文档

## 概述

Metrics 收集器 (`MetricsCollector`) 为 MVCC Store 和 RocksDB 存储提供轻量级的性能监控和指标收集功能,支持导出为 Prometheus 格式。

## 功能特性

### 1. 轻量级设计

- 基于 `AtomicU64` 实现,无锁高性能

- 可选启用/禁用,默认启用

- 最小化对业务逻辑的性能影响

### 2. 指标类型

#### MVCC 事务指标

- `mvcc_txn_started_total`: 已启动事务总数 (Counter)

- `mvcc_txn_committed_total`: 已提交事务总数 (Counter)

- `mvcc_txn_aborted_total`: 已中止事务总数 (Counter)

- `mvcc_tps`: 当前每秒事务数 (Gauge)

- `mvcc_success_rate`: 事务成功率百分比 (Gauge)

- `mvcc_txn_latency_ms`: 事务延迟分布 P50/P90/P99 (Summary)

#### 读写操作指标

- `reads`: 读操作次数 (Counter)

- `writes`: 写操作次数 (Counter)

- `read_latency`: 读延迟直方图

- `write_latency`: 写延迟 (Counter)

#### GC 指标

- `mvcc_gc_runs_total`: GC 运行次数 (Counter)

- `mvcc_gc_versions_cleaned_total`: GC 清理版本总数 (Counter)

- `gc_duration_ms`: GC 持续时间

#### 刷新指标

- `mvcc_flush_runs_total`: Flush 运行次数 (Counter)

- `mvcc_flush_keys_total`: Flush 键总数 (Counter)

- `mvcc_flush_bytes_total`: Flush 字节总数 (Counter)

#### RocksDB 指标

- `rocksdb_gets_total`: RocksDB Get 操作次数 (Counter)

- `rocksdb_puts_total`: RocksDB Put 操作次数 (Counter)

- `rocksdb_deletes_total`: RocksDB Delete 操作次数 (Counter)

### 3. 延迟直方图

`LatencyHistogram` 使用桶分布记录延迟:

- <1ms

- <5ms

- <10ms

- <50ms

- <100ms

- <500ms

- <1s

- \>1s

支持计算 P50/P90/P99 百分位延迟和平均延迟。

## API 使用

### 基本用法

```rust
use vm_runtime::{MvccStore, MetricsCollector};

// 创建 MVCC Store (默认启用指标收集)
let store = MvccStore::new();

// 执行事务...
let tx = store.begin();
// ... 事务操作 ...
tx.commit().unwrap();

// 获取指标
if let Some(metrics) = store.get_metrics() {
    // 打印摘要
    metrics.print_summary();
    
    // 导出 Prometheus 格式
    let prometheus_text = metrics.export_prometheus();
    println!("{}", prometheus_text);
}

```

### 控制指标收集

```rust
// 禁用指标收集
store.disable_metrics();

// 启用指标收集
store.enable_metrics();

```

### 手动记录指标

如果需要在自定义代码中记录指标:

```rust
use std::time::Instant;

if let Some(metrics) = store.get_metrics() {
    let start = Instant::now();
    
    // ... 执行操作 ...
    
    // 记录延迟
    metrics.txn_latency.observe(start.elapsed());
    
    // 更新计数器
    metrics.reads.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
}

```

## Prometheus 格式示例

```prometheus

# HELP mvcc_txn_started_total Total number of transactions started

# TYPE mvcc_txn_started_total counter

mvcc_txn_started_total 50

# HELP mvcc_txn_committed_total Total number of transactions committed

# TYPE mvcc_txn_committed_total counter

mvcc_txn_committed_total 49

# HELP mvcc_txn_aborted_total Total number of transactions aborted

# TYPE mvcc_txn_aborted_total counter

mvcc_txn_aborted_total 1

# HELP mvcc_tps Current transactions per second

# TYPE mvcc_tps gauge

mvcc_tps 1234.56

# HELP mvcc_success_rate Transaction success rate percentage

# TYPE mvcc_success_rate gauge

mvcc_success_rate 98.00

# HELP mvcc_txn_latency_ms Transaction latency percentiles in milliseconds

# TYPE mvcc_txn_latency_ms summary

mvcc_txn_latency_ms{quantile="0.5"} 1.23
mvcc_txn_latency_ms{quantile="0.9"} 5.67
mvcc_txn_latency_ms{quantile="0.99"} 10.45

```

## HTTP Metrics Endpoint (未来实现)

计划添加 HTTP 服务器,暴露 `/metrics` 端点供 Prometheus 抓取:

```rust
// 未来 API 设计
use vm_runtime::metrics::MetricsServer;

let metrics_server = MetricsServer::new("0.0.0.0:9090", store.get_metrics().unwrap());
metrics_server.start(); // 启动 HTTP 服务器

```

## Grafana Dashboard

可以使用导出的 Prometheus 指标创建 Grafana Dashboard。推荐面板:

1. **事务吞吐量**: TPS 时间序列图
2. **事务成功率**: 成功率百分比图
3. **延迟分布**: P50/P90/P99 延迟时间序列
4. **GC 性能**: GC 运行次数和清理版本数
5. **Flush 统计**: Flush 频率和数据量
6. **RocksDB 操作**: Get/Put/Delete 操作频率

## 注意事项

1. **性能影响**: 指标收集使用原子操作,性能开销极小 (<1%)
2. **内存使用**: 每个 `MetricsCollector` 约占 1KB 内存
3. **线程安全**: 所有操作都是线程安全的,无需额外加锁
4. **可选性**: 可以通过 `disable_metrics()` 完全禁用指标收集

## 示例程序

运行 metrics_demo 查看完整示例:

```bash
cargo run --example metrics_demo --release

```

## 未来改进

- [ ] 添加 HTTP `/metrics` 端点

- [ ] 支持自定义指标标签 (labels)

- [ ] 添加更细粒度的 RocksDB 指标

- [ ] 支持指标聚合和滑动窗口统计

- [ ] 集成 OpenTelemetry 标准
