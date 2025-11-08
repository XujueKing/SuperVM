# Grafana Dashboard 使用指南

本文档说明如何导入并使用 SuperVM 的 Grafana Dashboard 监控 MVCC 和 RocksDB 性能指标。

---

## 📋 前置要求

1. **Prometheus** - 用于采集和存储指标数据
2. **Grafana** - 用于可视化展示
3. **SuperVM metrics_http_demo** - 提供 /metrics 端点

---

## 🚀 快速开始

### 1. 启动 Prometheus

创建 `prometheus.yml` 配置文件：

```yaml
global:
  scrape_interval: 5s
  evaluation_interval: 5s

scrape_configs:
  - job_name: 'supervm'
    static_configs:
      - targets: ['localhost:8080']
    metrics_path: '/metrics'
```

启动 Prometheus：

```bash
# Linux/macOS
prometheus --config.file=prometheus.yml

# Windows
prometheus.exe --config.file=prometheus.yml
```

访问 http://localhost:9090 验证 Prometheus 已启动。

### 2. 启动 Grafana

```bash
# Linux/macOS
grafana-server

# Windows
grafana-server.exe

# Docker
docker run -d -p 3000:3000 grafana/grafana-oss
```

访问 http://localhost:3000（默认用户名/密码：admin/admin）。

### 3. 配置 Prometheus 数据源

1. 登录 Grafana
2. 点击左侧菜单 **Configuration** → **Data Sources**
3. 点击 **Add data source**
4. 选择 **Prometheus**
5. 配置 URL: `http://localhost:9090`
6. 点击 **Save & Test**

### 4. 导入 Dashboard

1. 点击左侧菜单 **Dashboards** → **Import**
2. 选择 **Upload JSON file**
3. 上传 `grafana-dashboard.json` 文件
4. 选择 Prometheus 数据源
5. 点击 **Import**

### 5. 启动 SuperVM metrics_http_demo

```powershell
cargo run -p vm-runtime --example metrics_http_demo --release
```

此时 Grafana Dashboard 应开始显示实时性能指标。

---

## 📊 Dashboard 面板说明

### 1. **MVCC Transactions Per Second (TPS)**
- **指标**: `mvcc_tps`
- **说明**: 当前每秒事务处理量
- **目标**: ≥ 100K TPS (低竞争), ≥ 85K TPS (高竞争)

### 2. **Transaction Success Rate**
- **指标**: `mvcc_success_rate`
- **说明**: 事务提交成功率百分比
- **阈值**:
  - 🟢 Green: ≥ 95% (正常)
  - 🟡 Yellow: 80-95% (警告)
  - 🔴 Red: < 80% (异常)

### 3. **Transaction Latency Percentiles**
- **指标**: `mvcc_txn_latency_ms{quantile="0.5|0.9|0.99"}`
- **说明**: 事务延迟 P50/P90/P99 百分位
- **目标**: P50 < 1ms, P90 < 5ms, P99 < 10ms

### 4. **Transaction Rates (1m avg)**
- **指标**: `mvcc_txn_started_total`, `mvcc_txn_committed_total`, `mvcc_txn_aborted_total`
- **说明**: 1 分钟内启动/提交/中止事务的平均速率

### 5. **MVCC Garbage Collection**
- **指标**: `mvcc_gc_runs_total`, `mvcc_gc_versions_cleaned_total`
- **说明**: GC 执行次数和清理的版本数

### 6. **MVCC Flush Statistics**
- **指标**: `mvcc_flush_count_total`, `mvcc_flush_keys_total`
- **说明**: 刷新到 RocksDB 的次数和键数

### 7. **MVCC Flush Bytes**
- **指标**: `mvcc_flush_bytes_total`
- **说明**: 刷新到 RocksDB 的总字节数

---

## 🔍 监控最佳实践

### 告警规则建议

在 Prometheus 中配置告警规则（`prometheus_alerts.yml`）：

```yaml
groups:
  - name: supervm
    interval: 10s
    rules:
      # TPS 过低告警
      - alert: LowTPS
        expr: mvcc_tps < 50000
        for: 1m
        labels:
          severity: warning
        annotations:
          summary: "SuperVM TPS 过低"
          description: "当前 TPS {{ $value }} 低于 50K 阈值"

      # 成功率过低告警
      - alert: LowSuccessRate
        expr: mvcc_success_rate < 80
        for: 1m
        labels:
          severity: critical
        annotations:
          summary: "SuperVM 事务成功率过低"
          description: "成功率 {{ $value }}% 低于 80%"

      # P99 延迟过高告警
      - alert: HighLatency
        expr: mvcc_txn_latency_ms{quantile="0.99"} > 50
        for: 1m
        labels:
          severity: warning
        annotations:
          summary: "SuperVM P99 延迟过高"
          description: "P99 延迟 {{ $value }}ms 超过 50ms"
```

### 性能基线参考

| 指标 | 低竞争场景 | 高竞争场景 | 告警阈值 |
|------|-----------|-----------|---------|
| TPS | ≥ 187K | ≥ 85K | < 50K |
| 成功率 | ≥ 99% | ≥ 95% | < 80% |
| P50 延迟 | < 0.5ms | < 1ms | > 5ms |
| P90 延迟 | < 2ms | < 5ms | > 20ms |
| P99 延迟 | < 5ms | < 10ms | > 50ms |

---

## 🛠️ 故障排查

### Dashboard 无数据

1. 检查 Prometheus 是否正常抓取指标：
   ```bash
   curl http://localhost:9090/api/v1/targets
   ```

2. 检查 SuperVM metrics_http_demo 是否运行：
   ```bash
   curl http://localhost:8080/metrics
   ```

3. 检查 Grafana 数据源配置是否正确

### 指标不更新

1. 确认 Prometheus scrape_interval 配置（建议 5s）
2. 确认 Grafana Dashboard 自动刷新已启用（右上角刷新图标）
3. 检查时间范围是否合适（建议 Last 15 minutes）

---

## 📚 扩展阅读

- [Prometheus 官方文档](https://prometheus.io/docs/)
- [Grafana 官方文档](https://grafana.com/docs/)
- [SuperVM Metrics Collector 文档](./docs/METRICS-COLLECTOR.md)
- [SuperVM Phase 4.3 总结](./docs/PHASE-4.3-WEEK3-4-SUMMARY.md)

---

## 💡 提示

- Dashboard 默认 5 秒自动刷新，可根据需要调整
- 支持时间范围选择（Last 5m/15m/1h/6h/24h）
- 支持变量模板（未来版本可添加实例/节点筛选）
- 支持告警集成（通过 Prometheus Alertmanager）

如有问题或建议，请提交 Issue 或 PR！
