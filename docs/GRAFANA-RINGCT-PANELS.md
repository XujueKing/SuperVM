# Grafana RingCT 并行证明指标面板建议

本文档提供新增 Prometheus 指标的 Grafana 面板配置建议，用于监控 RingCT 并行证明性能。

## 新增指标概览

| 指标名称 | 类型 | 描述 |
|---------|------|------|
| `vm_privacy_zk_parallel_proof_total` | counter | 累计尝试并行证明总数 |
| `vm_privacy_zk_parallel_proof_failed_total` | counter | 累计失败证明总数 |
| `vm_privacy_zk_parallel_batches_total` | counter | 累计处理批次数 |
| `vm_privacy_zk_parallel_batch_latency_ms` | gauge | 最近批次总延迟（毫秒）|
| `vm_privacy_zk_parallel_avg_latency_ms` | gauge | 最近批次单个证明平均延迟（毫秒）|
| `vm_privacy_zk_parallel_tps` | gauge | 最近批次吞吐量（proofs/sec）|
| `vm_fast_fallback_total` | counter | Fast→Consensus 路径回退次数 |

---

## 推荐面板配置

### 1. RingCT 并行证明吞吐量 (TPS)

**面板类型**: Time series  
**PromQL 查询**:
```promql
vm_privacy_zk_parallel_tps
```

**配置建议**:
- **单位**: `proofs/sec` (自定义)
- **Y 轴标签**: "Proofs per Second"
- **图例**: 显示 Mean / Last / Max
- **阈值**:
  - 绿色: > 100 proofs/s (健康)
  - 黄色: 50-100 proofs/s (警告)
  - 红色: < 50 proofs/s (异常)

---

### 2. RingCT 批次延迟监控

**面板类型**: Time series (双 Y 轴)  
**PromQL 查询**:
```promql
# 左 Y 轴：批次总延迟
vm_privacy_zk_parallel_batch_latency_ms

# 右 Y 轴：单证明平均延迟
vm_privacy_zk_parallel_avg_latency_ms
```

**配置建议**:
- **单位**: `ms` (毫秒)
- **Y 轴 (左)**: 批次总延迟
- **Y 轴 (右)**: 单证明平均延迟
- **图例**: Table 模式，显示 Mean / P90 / Max
- **阈值 (avg_latency)**:
  - 绿色: < 50ms
  - 黄色: 50-100ms
  - 红色: > 100ms

---

### 3. RingCT 证明成功率

**面板类型**: Stat (单值面板)  
**PromQL 查询**:
```promql
# 成功率百分比
100 * (1 - (rate(vm_privacy_zk_parallel_proof_failed_total[5m]) / rate(vm_privacy_zk_parallel_proof_total[5m])))
```

**配置建议**:
- **单位**: `percent (0-100)`
- **颜色模式**: 基于阈值
- **阈值**:
  - 绿色: > 99.5%
  - 黄色: 95-99.5%
  - 红色: < 95%
- **显示**: 大号数字 + 趋势线

---

### 4. RingCT 证明失败计数 (Counter)

**面板类型**: Time series  
**PromQL 查询**:
```promql
# 失败率 (每秒)
rate(vm_privacy_zk_parallel_proof_failed_total[1m])
```

**配置建议**:
- **单位**: `failures/sec`
- **Y 轴标签**: "Proof Failures per Second"
- **告警规则**: 当 `rate(vm_privacy_zk_parallel_proof_failed_total[5m]) > 1` 持续 2 分钟时触发

---

### 5. Fast→Consensus 路径回退监控

**面板类型**: Time series  
**PromQL 查询**:
```promql
# 回退率 (每秒)
rate(vm_fast_fallback_total[1m])
```

**配置建议**:
- **单位**: `fallbacks/sec`
- **Y 轴标签**: "Fast Path Fallbacks per Second"
- **阈值**:
  - 绿色: < 1 fallback/s (正常)
  - 黄色: 1-5 fallbacks/s (注意)
  - 红色: > 5 fallbacks/s (异常)

---

### 6. RingCT 批次处理仪表盘 (Stat Grid)

**面板类型**: Stat (统计卡片网格)  
**PromQL 查询 (多个 Query)**:

1. **总证明数**:
   ```promql
   vm_privacy_zk_parallel_proof_total
   ```
   单位: `short` / 颜色: 蓝色

2. **总失败数**:
   ```promql
   vm_privacy_zk_parallel_proof_failed_total
   ```
   单位: `short` / 颜色: 红色

3. **总批次数**:
   ```promql
   vm_privacy_zk_parallel_batches_total
   ```
   单位: `short` / 颜色: 绿色

4. **最近批次 TPS**:
   ```promql
   vm_privacy_zk_parallel_tps
   ```
   单位: `proofs/sec` / 颜色: 橙色

**布局**: 2x2 网格，每个卡片宽度 6，高度 4

---

## 推荐仪表盘布局

```
┌──────────────────────────────────────────────────────────────────┐
│  Row 1: RingCT 并行证明性能总览                                    │
├────────────────────────────┬─────────────────────────────────────┤
│  [1] RingCT TPS           │  [2] 批次延迟 (总/平均)               │
│  (Time series, 12w x 8h)  │  (Time series, 12w x 8h)            │
├────────────────────────────┴─────────────────────────────────────┤
│  Row 2: 统计卡片                                                  │
├────────┬────────┬────────┬────────┬───────────────────────────────┤
│ 总证明 │ 总失败 │ 总批次 │ 最近TPS│  [3] 证明成功率               │
│ 6w x4h │ 6w x4h │ 6w x4h │ 6w x4h │  (Stat, 12w x 4h)            │
├────────┴────────┴────────┴────────┴───────────────────────────────┤
│  Row 3: 异常监控                                                  │
├────────────────────────────┬─────────────────────────────────────┤
│  [4] 失败率趋势            │  [5] Fast 回退监控                   │
│  (Time series, 12w x 6h)  │  (Time series, 12w x 6h)            │
└────────────────────────────┴─────────────────────────────────────┘
```

---

## 告警规则建议 (Prometheus Alertmanager)

### 1. 高失败率告警
```yaml
- alert: RingCTProofHighFailureRate
  expr: |
    100 * (rate(vm_privacy_zk_parallel_proof_failed_total[5m]) / rate(vm_privacy_zk_parallel_proof_total[5m])) > 5
  for: 2m
  labels:
    severity: warning
  annotations:
    summary: "RingCT proof failure rate > 5%"
    description: "Current failure rate: {{ $value | humanizePercentage }}"
```

### 2. 低吞吐量告警
```yaml
- alert: RingCTProofLowThroughput
  expr: vm_privacy_zk_parallel_tps < 50
  for: 5m
  labels:
    severity: warning
  annotations:
    summary: "RingCT proof throughput below 50 proofs/sec"
    description: "Current TPS: {{ $value | printf \"%.2f\" }}"
```

### 3. Fast 回退异常
```yaml
- alert: FastPathFallbackSpike
  expr: rate(vm_fast_fallback_total[2m]) > 10
  for: 1m
  labels:
    severity: critical
  annotations:
    summary: "Fast→Consensus fallback rate exceeds 10/sec"
    description: "Current rate: {{ $value | printf \"%.2f\" }} fallbacks/sec"
```

---

## 快速导入面板 JSON 片段

将以下 JSON 片段插入 `grafana-dashboard.json` 的 `panels` 数组：

```json
{
  "datasource": {
    "type": "prometheus",
    "uid": "${DS_PROMETHEUS}"
  },
  "fieldConfig": {
    "defaults": {
      "color": { "mode": "palette-classic" },
      "custom": {
        "axisCenteredZero": false,
        "axisLabel": "Proofs/sec",
        "axisPlacement": "auto",
        "drawStyle": "line",
        "fillOpacity": 10,
        "lineWidth": 2,
        "showPoints": "never"
      },
      "unit": "short",
      "thresholds": {
        "mode": "absolute",
        "steps": [
          { "color": "red", "value": null },
          { "color": "yellow", "value": 50 },
          { "color": "green", "value": 100 }
        ]
      }
    }
  },
  "gridPos": { "h": 8, "w": 12, "x": 0, "y": 16 },
  "id": 20,
  "targets": [
    {
      "expr": "vm_privacy_zk_parallel_tps",
      "legendFormat": "RingCT TPS",
      "refId": "A"
    }
  ],
  "title": "RingCT Parallel Proof Throughput",
  "type": "timeseries"
}
```

---

## 使用说明

1. **启动 HTTP Bench 示例**:
   ```bash
   cargo run -p vm-runtime --features groth16-verifier --example zk_parallel_http_bench --release
   ```

2. **配置 Prometheus 抓取目标** (`prometheus.yml`):
   ```yaml
   scrape_configs:
     - job_name: 'supervm-ringct'
       static_configs:
         - targets: ['localhost:9090']
   ```

3. **导入 Grafana 仪表盘**:
   - 打开 Grafana → Create → Import
   - 粘贴更新后的 `grafana-dashboard.json`
   - 或使用上述 JSON 片段手动添加面板

4. **验证指标可见性**:
   - 访问 `http://localhost:9090/metrics`
   - 搜索 `vm_privacy_zk_parallel` 确认指标导出正常

---

## 参考资源

- **Prometheus 查询语法**: https://prometheus.io/docs/prometheus/latest/querying/basics/
- **Grafana 面板配置**: https://grafana.com/docs/grafana/latest/panels/
- **SuperVM HTTP Bench 示例**: `src/vm-runtime/examples/zk_parallel_http_bench.rs`
- **指标收集器实现**: `src/vm-runtime/src/metrics.rs`

---

**版本**: 1.0.0  
**日期**: 2025-11-09  
**作者**: SuperVM Team
