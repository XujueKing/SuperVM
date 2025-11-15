# SuperVM 指标合同 (Metrics Contract)

本文档描述 SuperVM 运行时导出的所有 Prometheus 指标、标签、类型及推荐 PromQL 查询。

## 指标概览

SuperVM 导出以下几大类指标：
1. **MVCC 事务指标** - 事务启动/提交/回滚统计
2. **路由指标** - Fast / Consensus / Privacy 路径路由分布
3. **Fast 回退指标** - Fast→Consensus 回退次数与比率
4. **GC 与 Flush 指标** - 内存版本清理与持久化统计
5. **RocksDB 操作指标** - 持久层读写删除计数
6. **并行 ZK 证明指标** - ZK 验证批次、成功/失败、延迟
7. **ZK 验证延迟窗口** - 滑动窗口 P50/P95 百分位

---

## 1. MVCC 事务指标

### `mvcc_txn_started_total` (counter)

- **描述**: 启动的事务总数

- **类型**: counter

- **示例值**: 1234567

- **PromQL**:
  ```promql
  # 每秒启动的事务数 (QPS)
  rate(mvcc_txn_started_total[1m])
  
  # 最近 5 分钟总启动数
  increase(mvcc_txn_started_total[5m])
  ```

### `mvcc_txn_committed_total` (counter)

- **描述**: 成功提交的事务总数

- **类型**: counter

- **示例值**: 1200000

- **PromQL**:
  ```promql
  # 提交 TPS
  rate(mvcc_txn_committed_total[1m])
  
  # 提交事务总数（全时间）
  mvcc_txn_committed_total
  ```

### `mvcc_txn_aborted_total` (counter)

- **描述**: 回滚/中止的事务总数

- **类型**: counter

- **示例值**: 34567

- **PromQL**:
  ```promql
  # 回滚 TPS
  rate(mvcc_txn_aborted_total[1m])
  ```

### `mvcc_success_rate` (gauge)

- **描述**: 事务成功率（百分比，0-100）

- **类型**: gauge

- **示例值**: 97.21

- **PromQL**:
  ```promql
  # 当前成功率
  mvcc_success_rate
  
  # 最近 5 分钟平均成功率
  avg_over_time(mvcc_success_rate[5m])
  
  # 告警：成功率低于 95%
  mvcc_success_rate < 95
  ```

### `mvcc_txn_latency_ms` (summary)

- **描述**: 事务延迟百分位（ms）

- **类型**: summary

- **标签**: `quantile` = "0.5" | "0.9" | "0.99"

- **示例值**: 
  - `mvcc_txn_latency_ms{quantile="0.5"}` → 2.34
  - `mvcc_txn_latency_ms{quantile="0.9"}` → 12.56
  - `mvcc_txn_latency_ms{quantile="0.99"}` → 45.78

- **PromQL**:
  ```promql
  # P50 延迟
  mvcc_txn_latency_ms{quantile="0.5"}
  
  # P99 延迟
  mvcc_txn_latency_ms{quantile="0.99"}
  
  # 告警：P99 延迟超过 100ms
  mvcc_txn_latency_ms{quantile="0.99"} > 100
  ```

### `mvcc_tps_total` / `mvcc_tps_window` / `mvcc_peak_tps` (gauge)

- **描述**: 
  - `mvcc_tps_total`: 全局平均 TPS
  - `mvcc_tps_window`: 当前滑动窗口 TPS
  - `mvcc_peak_tps`: 观测到的峰值 TPS

- **类型**: gauge

- **示例值**: 50000.12

- **PromQL**:
  ```promql
  # 当前窗口 TPS
  mvcc_tps_window
  
  # 峰值 TPS
  max_over_time(mvcc_peak_tps[1h])
  
  # 告警：TPS 低于 10K
  mvcc_tps_window < 10000
  ```

---

## 2. 路由指标 (SuperVM Routing)

### `vm_routing_fast_total` (counter)

- **描述**: 路由到 Fast Path 的事务总数

- **类型**: counter

- **示例值**: 800000

- **PromQL**:
  ```promql
  # Fast 路由 TPS
  rate(vm_routing_fast_total[1m])
  
  # Fast 路由占比（需结合 total）
  vm_routing_fast_total / vm_routing_total
  ```

### `vm_routing_consensus_total` (counter)

- **描述**: 路由到 Consensus Path 的事务总数

- **类型**: counter

- **示例值**: 150000

- **PromQL**:
  ```promql
  # Consensus 路由 TPS
  rate(vm_routing_consensus_total[1m])
  ```

### `vm_routing_privacy_total` (counter)

- **描述**: 路由到 Privacy Path 的事务总数

- **类型**: counter

- **示例值**: 50000

- **PromQL**:
  ```promql
  # Privacy 路由 TPS
  rate(vm_routing_privacy_total[1m])
  ```

### `vm_routing_total` (counter)

- **描述**: 路由事务总数（Fast + Consensus + Privacy）

- **类型**: counter

- **示例值**: 1000000

- **PromQL**:
  ```promql
  # 总路由 TPS
  rate(vm_routing_total[1m])
  ```

### `vm_routing_fast_ratio` / `vm_routing_consensus_ratio` / `vm_routing_privacy_ratio` (gauge)

- **描述**: 各路径占比（0.0 ~ 1.0）

- **类型**: gauge

- **示例值**: 0.80, 0.15, 0.05

- **PromQL**:
  ```promql
  # Fast 路径占比
  vm_routing_fast_ratio
  
  # 告警：Fast 比例低于 70%（若期望高快速路径占比）
  vm_routing_fast_ratio < 0.70
  
  # 趋势：最近 1 小时 Consensus 占比趋势
  avg_over_time(vm_routing_consensus_ratio[1h])
  ```

---

## 3. Fast 回退指标

### `vm_fast_fallback_total` (counter)

- **描述**: Fast Path 失败后回退到 Consensus Path 的总次数

- **类型**: counter

- **示例值**: 1234

- **PromQL**:
  ```promql
  # 回退 TPS
  rate(vm_fast_fallback_total[1m])
  
  # 总回退数
  vm_fast_fallback_total
  ```

### `vm_fast_fallback_ratio` (gauge)

- **描述**: Fast 回退占已提交事务的比例（0.0 ~ 1.0）

- **类型**: gauge

- **示例值**: 0.001234

- **PromQL**:
  ```promql
  # 当前回退率
  vm_fast_fallback_ratio
  
  # 告警：回退率超过 1%
  vm_fast_fallback_ratio > 0.01
  
  # 回退率 24 小时趋势
  avg_over_time(vm_fast_fallback_ratio[24h])
  ```

---

## 4. GC 与 Flush 指标

### `mvcc_gc_runs_total` (counter)

- **描述**: GC 执行次数

- **类型**: counter

- **示例值**: 345

- **PromQL**:
  ```promql
  # GC 频率（次/分钟）
  rate(mvcc_gc_runs_total[1m]) * 60
  ```

### `mvcc_gc_versions_cleaned_total` (counter)

- **描述**: GC 清理的版本总数

- **类型**: counter

- **示例值**: 123456

- **PromQL**:
  ```promql
  # 每次 GC 平均清理版本数
  rate(mvcc_gc_versions_cleaned_total[5m]) / rate(mvcc_gc_runs_total[5m])
  ```

### `mvcc_gc_duration_ms_total` (counter)

- **描述**: GC 累计耗时（ms）

- **类型**: counter

- **示例值**: 45678

- **PromQL**:
  ```promql
  # 平均 GC 延迟（ms）
  rate(mvcc_gc_duration_ms_total[5m]) / rate(mvcc_gc_runs_total[5m])
  ```

### `mvcc_flush_runs_total` / `mvcc_flush_keys_total` / `mvcc_flush_bytes_total` (counter)

- **描述**: Flush 执行次数、刷新键数、刷新字节数

- **类型**: counter

- **示例值**: 123, 456789, 1234567890

- **PromQL**:
  ```promql
  # Flush 频率（次/分钟）
  rate(mvcc_flush_runs_total[1m]) * 60
  
  # 平均每次 Flush 键数
  rate(mvcc_flush_keys_total[5m]) / rate(mvcc_flush_runs_total[5m])
  
  # Flush 吞吐量（MB/s）
  rate(mvcc_flush_bytes_total[1m]) / 1024 / 1024
  ```

---

## 5. RocksDB 操作指标

### `rocksdb_get_total` / `rocksdb_put_total` / `rocksdb_delete_total` (counter)

- **描述**: RocksDB 读取/写入/删除操作总数

- **类型**: counter

- **示例值**: 5000000, 3000000, 100000

- **PromQL**:
  ```promql
  # RocksDB 读取 QPS
  rate(rocksdb_get_total[1m])
  
  # RocksDB 写入 QPS
  rate(rocksdb_put_total[1m])
  
  # 读写比
  rate(rocksdb_get_total[5m]) / rate(rocksdb_put_total[5m])
  ```

---

## 6. 并行 ZK 证明指标

### `zk_parallel_proof_total` / `zk_parallel_proof_failed_total` (counter)

- **描述**: 并行证明总数与失败数

- **类型**: counter

- **示例值**: 100000, 123

- **PromQL**:
  ```promql
  # ZK 证明成功率
  (zk_parallel_proof_total - zk_parallel_proof_failed_total) / zk_parallel_proof_total
  
  # ZK 证明 TPS
  rate(zk_parallel_proof_total[1m])
  ```

### `zk_parallel_batches_total` (counter)

- **描述**: 并行批次总数

- **类型**: counter

- **示例值**: 1234

- **PromQL**:
  ```promql
  # 批次频率（批/分钟）
  rate(zk_parallel_batches_total[1m]) * 60
  ```

### `zk_parallel_last_batch_latency_ms` / `zk_parallel_last_batch_avg_latency_ms` / `zk_parallel_last_batch_tps` (gauge)

- **描述**: 最近一次批次的总延迟、平均延迟、TPS

- **类型**: gauge

- **示例值**: 123.45, 12.34, 8100.00

- **PromQL**:
  ```promql
  # 最近批次延迟
  zk_parallel_last_batch_latency_ms
  
  # 最近批次平均单证明延迟
  zk_parallel_last_batch_avg_latency_ms
  
  # 最近批次 TPS
  zk_parallel_last_batch_tps
  ```

---

## 7. ZK 验证延迟窗口（SuperVM Privacy）

*注：仅在启用 `groth16-verifier` feature 且发生过真实 ZK 验证时导出*

### `vm_privacy_zk_verify_count_total` (counter)

- **描述**: 真实 ZK 证明验证总次数

- **类型**: counter

- **示例值**: 12345

- **PromQL**:
  ```promql
  # ZK 验证 QPS
  rate(vm_privacy_zk_verify_count_total[1m])
  ```

### `vm_privacy_zk_verify_avg_latency_ms` (gauge)

- **描述**: ZK 验证平均延迟（ms，全局累计）

- **类型**: gauge

- **示例值**: 8.765

- **PromQL**:
  ```promql
  # 当前平均 ZK 验证延迟
  vm_privacy_zk_verify_avg_latency_ms
  
  # 告警：平均延迟超过 20ms
  vm_privacy_zk_verify_avg_latency_ms > 20
  ```

### `vm_privacy_zk_verify_last_latency_ms` (gauge)

- **描述**: 最近一次 ZK 验证延迟（ms）

- **类型**: gauge

- **示例值**: 9.123

- **PromQL**:
  ```promql
  # 最近一次延迟
  vm_privacy_zk_verify_last_latency_ms
  ```

### `vm_privacy_zk_verify_p50_latency_ms` / `vm_privacy_zk_verify_p95_latency_ms` (gauge)

- **描述**: 滑动窗口内 P50 / P95 ZK 验证延迟（ms）

- **类型**: gauge

- **示例值**: 7.8, 15.6

- **PromQL**:
  ```promql
  # P50 窗口延迟
  vm_privacy_zk_verify_p50_latency_ms
  
  # P95 窗口延迟
  vm_privacy_zk_verify_p95_latency_ms
  
  # 告警：P95 延迟超过 30ms
  vm_privacy_zk_verify_p95_latency_ms > 30
  ```

### `vm_privacy_zk_verify_window_size` (gauge)

- **描述**: 滑动窗口样本数量

- **类型**: gauge

- **示例值**: 64

- **PromQL**:
  ```promql
  # 窗口样本数
  vm_privacy_zk_verify_window_size
  ```

---

## 推荐告警规则

### 高优先级告警

```yaml
groups:
  - name: supervm_critical
    interval: 30s
    rules:
      - alert: LowSuccessRate
        expr: mvcc_success_rate < 95
        for: 2m
        labels:
          severity: critical
        annotations:
          summary: "MVCC 事务成功率低于 95%"
          description: "当前成功率 {{ $value }}%"

      - alert: HighP99Latency
        expr: mvcc_txn_latency_ms{quantile="0.99"} > 100
        for: 3m
        labels:
          severity: warning
        annotations:
          summary: "P99 延迟超过 100ms"
          description: "当前 P99 延迟 {{ $value }}ms"

      - alert: HighFastFallbackRatio
        expr: vm_fast_fallback_ratio > 0.05
        for: 5m
        labels:
          severity: warning
        annotations:
          summary: "Fast 回退率超过 5%"
          description: "回退率 {{ $value | humanizePercentage }}"

      - alert: LowFastPathRatio
        expr: vm_routing_fast_ratio < 0.50
        for: 5m
        labels:
          severity: info
        annotations:
          summary: "Fast 路径占比低于 50%"
          description: "当前占比 {{ $value | humanizePercentage }}"

      - alert: ZkVerifyHighLatency
        expr: vm_privacy_zk_verify_p95_latency_ms > 30
        for: 3m
        labels:
          severity: warning
        annotations:
          summary: "ZK 验证 P95 延迟超过 30ms"
          description: "当前 P95 {{ $value }}ms"

```

---

## 示例 Grafana 仪表盘查询

### 面板 1: 总体 TPS 与成功率

```promql

# TPS

rate(mvcc_txn_committed_total[1m])

# 成功率

mvcc_success_rate

```

### 面板 2: 路由分布（饼图）

```promql

# Fast

vm_routing_fast_total

# Consensus

vm_routing_consensus_total

# Privacy

vm_routing_privacy_total

```

### 面板 3: 延迟百分位时间序列

```promql

# P50

mvcc_txn_latency_ms{quantile="0.5"}

# P90

mvcc_txn_latency_ms{quantile="0.9"}

# P99

mvcc_txn_latency_ms{quantile="0.99"}

```

### 面板 4: Fast 回退趋势

```promql

# 回退 TPS

rate(vm_fast_fallback_total[1m])

# 回退率

vm_fast_fallback_ratio

```

### 面板 5: GC 与 Flush 性能

```promql

# GC 频率（次/分钟）

rate(mvcc_gc_runs_total[1m]) * 60

# Flush 吞吐量（MB/s）

rate(mvcc_flush_bytes_total[1m]) / 1024 / 1024

# 平均 GC 延迟

rate(mvcc_gc_duration_ms_total[5m]) / rate(mvcc_gc_runs_total[5m])

```

### 面板 6: ZK 验证延迟分布

```promql

# P50

vm_privacy_zk_verify_p50_latency_ms

# P95

vm_privacy_zk_verify_p95_latency_ms

# 平均

vm_privacy_zk_verify_avg_latency_ms

```

---

## 导出端点

**HTTP 端点**: `/metrics` (需启动 metrics HTTP 服务器)

**示例**:

```bash
curl http://localhost:9090/metrics

```

**集成到 Prometheus**:

```yaml

# prometheus.yml

scrape_configs:
  - job_name: 'supervm'
    static_configs:
      - targets: ['localhost:9090']
    scrape_interval: 15s

```

---

## 版本历史

- **v0.1.0** (2025-11-10): 初始指标合同
  - MVCC 事务指标
  - 路由指标（Fast / Consensus / Privacy）
  - Fast 回退指标
  - GC / Flush 指标
  - RocksDB 操作指标
  - 并行 ZK 证明指标
  - ZK 验证延迟窗口指标

---

## 未来扩展

计划增加：

- 跨分片事务指标（Shard ID 标签）

- 自适应路由器调整事件计数

- 热键（Hotkey）检测指标

- 网络层 P2P 指标（当 SuperVM 集成分布式网络时）

---

**文档维护**: SuperVM 开发团队  
**最后更新**: 2025-11-10  
**联系**: leadbrand@me.com
