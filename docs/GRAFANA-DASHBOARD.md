
---

## ⚠️ Prometheus告警规则

SuperVM提供12个关键告警规则 (`prometheus-supervm-alerts.yml`):

### 核心性能告警

1. **LowTPS** - TPS < 50K持续5分钟
  - **严重性**: warning | **组件**: mvcc
  - **操作**: 检查系统负载,RocksDB性能,GC频率

2. **LowSuccessRate** - 事务成功率 < 80%持续5分钟
  - **严重性**: critical | **组件**: mvcc
  - **操作**: 检查冲突率,调整并发控制策略,减少热点键访问

### 三通道路由告警

3. **HighFastPathFallbackRate** - FastPath回退率 > 10%持续5分钟
  - **严重性**: warning | **组件**: routing
  - **操作**: 检查所有权预测准确性,调整AdaptiveRouter参数

4. **LowFastPathSuccessRate** - FastPath成功率 < 95%持续5分钟
  - **严重性**: warning | **组件**: routing
  - **操作**: 优化所有权分析算法,检查owned_ratio设置

5. **AdaptiveRouterAdjustmentFrequency** - 自适应调整频率 > 10次/秒持续10分钟
  - **严重性**: info | **组件**: routing
  - **操作**: 调整conflict_high/low阈值,增加update_every间隔

### ZK隐私验证告警

6. **LowZKProofTPS** - RingCT证明TPS < 30持续5分钟
  - **严重性**: warning | **组件**: privacy
  - **操作**: 检查验证器性能,优化批量验证配置,考虑GPU加速

7. **HighZKVerificationFailureRate** - ZK验证失败率 > 5%持续5分钟
  - **严重性**: critical | **组件**: privacy
  - **操作**: 检查证明生成器实现,验证器参数配置,曲线参数正确性

8. **HighZKVerificationLatency** - P95延迟 > 15ms持续5分钟
  - **严重性**: warning | **组件**: privacy
  - **操作**: 优化验证算法,检查CPU/GPU利用率,考虑批量验证

### 存储与GC告警

9. **HighGCFrequency** - GC运行频率 > 100次/秒持续10分钟
  - **严重性**: info | **组件**: storage
  - **操作**: 调整GC阈值,增加版本清理间隔

10. **LowVersionCleaningRate** - GC版本清理速率 < 10个/秒持续10分钟
   - **严重性**: info | **组件**: storage
   - **操作**: 检查长事务,优化GC策略

### 系统健康告警

11. **HighTransactionAbortRate** - 事务中止率 > 20%持续5分钟
   - **严重性**: warning | **组件**: mvcc
   - **操作**: 分析工作负载,减少冲突,检查热点键

12. **PrometheusMetricsStale** - SuperVM指标停止更新2分钟
   - **严重性**: critical | **组件**: monitoring
   - **操作**: 检查SuperVM实例状态,/metrics端点可用性,Prometheus配置

---

## 🔧 AdaptiveRouter 环境变量配置

AdaptiveRouter 支持通过环境变量动态调整参数:

```powershell
# 初始FastPath目标比例 (默认0.7 = 70%)
$env:SUPERVM_ADAPTIVE_INITIAL_FAST_RATIO = "0.75"

# FastPath比例最小值 (默认0.3 = 30%)
$env:SUPERVM_ADAPTIVE_MIN_RATIO = "0.4"

# FastPath比例最大值 (默认0.95 = 95%)
$env:SUPERVM_ADAPTIVE_MAX_RATIO = "0.9"

# 上调步长 (默认0.02 = 2%)
$env:SUPERVM_ADAPTIVE_STEP_UP = "0.01"

# 下调步长 (默认0.05 = 5%)
$env:SUPERVM_ADAPTIVE_STEP_DOWN = "0.03"

# 高冲突率阈值 (默认0.25 = 25%)
$env:SUPERVM_ADAPTIVE_CONFLICT_HIGH = "0.2"

# 低冲突率阈值 (默认0.1 = 10%)
$env:SUPERVM_ADAPTIVE_CONFLICT_LOW = "0.05"

# 低成功率保护阈值 (默认0.8 = 80%)
$env:SUPERVM_ADAPTIVE_SUCCESS_LOW = "0.85"

# 调整频率 (默认100,每100次调用maybe_update执行一次)
$env:SUPERVM_ADAPTIVE_UPDATE_EVERY = "50"
```

**调优建议**:
- **高冲突场景**: 降低`conflict_high`至0.15-0.2,加快下调响应
- **低冲突场景**: 提高`conflict_low`至0.15,减少不必要调整
- **稳定性优先**: 增加`update_every`至200,减少抖动
- **快速响应**: 减小`step_down`至0.02,降低上调激进度

# Grafana Dashboard 使用指南

本文档说明如何导入并使用 SuperVM 的 Grafana Dashboard 监控 MVCC、三通道路由和 ZK 隐私验证性能指标。

---

## 📦 Dashboard 文件列表

SuperVM 提供以下 Dashboard 文件:

1. **grafana-supervm-unified-dashboard.json** - ⭐ **推荐使用** - 统一Dashboard,包含所有性能指标
   - 📊 核心性能指标 (TPS, 延迟, 成功率)
   - 🚀 三通道路由性能 (FastPath/Consensus/Privacy)
   - 🔒 ZK隐私验证 (Groth16, RingCT批量验证)
   - 💾 存储与GC (MVCC垃圾回收, Flush统计)
   - ⚠️ 内置告警 (TPS过低, 回退率过高, ZK失败率)

2. **grafana-dashboard.json** - 传统MVCC Dashboard (8个面板)
3. **grafana-phase5-dashboard.json** - 三通道路由专用 (12个面板)
4. **grafana-ringct-dashboard.json** - RingCT隐私验证 (7个面板)

---

## 📋 前置要求

1. **Prometheus** - 用于采集和存储指标数据
2. **Grafana** - 用于可视化展示
3. **SuperVM metrics服务** - 提供 /metrics 端点 (例如 mixed_path_bench 或 metrics_http_demo)

---

## 🚀 快速开始

### 1. 启动 Prometheus (含告警规则)

创建 `prometheus.yml` 配置文件:

```yaml
global:
  scrape_interval: 5s
  evaluation_interval: 5s

# 加载告警规则
rule_files:
  - 'prometheus-supervm-alerts.yml'  # SuperVM 统一告警规则

# Alertmanager 配置 (可选)
alerting:
  alertmanagers:
    - static_configs:
        - targets: ['localhost:9093']  # 如果使用 Alertmanager

scrape_configs:
  - job_name: 'supervm'
    static_configs:
      - targets: ['localhost:8080']  # mixed_path_bench 或 metrics_http_demo
    metrics_path: '/metrics'
```

启动 Prometheus:

```bash
# Linux/macOS
prometheus --config.file=prometheus.yml

# Windows
prometheus.exe --config.file=prometheus.yml
```

访问 http://localhost:9090 验证 Prometheus 已启动。检查 **Status → Rules** 确认告警规则已加载。

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

### 4. 导入统一Dashboard

1. 点击左侧菜单 **Dashboards** → **Import**
2. 选择 **Upload JSON file**
3. 上传 `grafana-supervm-unified-dashboard.json` 文件 ⭐ **推荐**
4. 选择 Prometheus 数据源
5. 点击 **Import**

> **提示**: 如果需要分别查看各模块,也可以导入 grafana-dashboard.json, grafana-phase5-dashboard.json, grafana-ringct-dashboard.json

### 5. 启动 SuperVM 性能测试

**选项 1: 三通道混合路径测试 (推荐)**
```powershell
cargo run --example mixed_path_bench --release -- --iterations 100000 --owned-ratio 0.7
```

**选项 2: MVCC 基础测试**
```powershell
cargo run -p vm-runtime --example metrics_http_demo --release
```

**选项 3: 端到端三通道验证**
```powershell
cargo test --package vm-runtime --test e2e_three_channel_test --release
```

此时 Grafana Dashboard 应开始显示实时性能指标。访问 http://localhost:3000 查看 "SuperVM Unified Dashboard"。

---

## 📊 统一Dashboard面板说明

### 📊 核心性能指标 (Row 1)

#### 1. **系统总TPS (MVCC + 三通道)**
- **指标**: `mvcc_tps` + `rate(vm_routing_fast_total + vm_routing_consensus_total + vm_routing_privacy_total)`
- **说明**: MVCC事务TPS + 三通道路由总TPS
- **目标**: ≥ 100K TPS (低竞争), ≥ 85K TPS (高竞争)
- **告警**: < 50K TPS 触发警告

#### 2. **事务成功率**
- **指标**: `mvcc_success_rate`
- **说明**: MVCC事务提交成功率百分比
- **阈值**:
  - 🟢 Green: ≥ 95% (正常)
  - 🟡 Yellow: 80-95% (警告)
  - 🔴 Red: < 80% (异常)
- **告警**: < 80% 触发严重警告

#### 3. **FastPath 成功率**
- **指标**: `(vm_fast_path_success_total / vm_fast_path_attempts_total) * 100`
- **说明**: FastPath零冲突路径成功率
- **目标**: ≥ 99% (理想), ≥ 95% (可接受)
- **告警**: < 95% 触发优化建议

#### 4. **Fast→Consensus 回退统计**
- **指标**: `vm_fast_fallback_total`
- **说明**: FastPath因冲突回退到Consensus通道的累计次数
- **阈值**:
  - 🟢 Green: < 1000 (低回退)
  - 🟡 Yellow: 1000-10000 (中等)
  - 🔴 Red: > 10000 (高回退)

#### 5. **事务延迟百分位 (P50/P90/P99)**
- **指标**: `mvcc_txn_latency_ms{quantile="0.5|0.9|0.99"}`
- **说明**: 事务处理延迟分布
- **目标**: P99 < 10ms (高性能), P50 < 2ms (理想)

### 🚀 三通道路由性能 (Row 2)

#### 6. **三通道吞吐量 (FastPath/Consensus/Privacy)**
- **指标**: 
  - FastPath: `rate(vm_routing_fast_total[1m])`
  - Consensus: `rate(vm_routing_consensus_total[1m])`
  - Privacy: `rate(vm_routing_privacy_total[1m])`
- **说明**: 三通道每秒路由的事务数
- **目标**: FastPath占比 > 70% (owned_ratio=0.7时)

#### 7. **路由比例分布 (饼图)**
- **指标**: `vm_routing_fast_total`, `vm_routing_consensus_total`, `vm_routing_privacy_total`
- **说明**: 三通道事务量占比可视化
- **分析**: 
  - FastPath占比过低 → 检查所有权分析准确性
  - Privacy占比异常高 → 检查隐私交易生成器

#### 8. **AdaptiveRouter目标FastPath比例**
- **指标**: `vm_routing_target_fast_ratio`
- **说明**: 自适应路由器动态调整的目标FastPath占比
- **范围**: 0.0 - 1.0 (对应0%-100%)
- **分析**: 该值随冲突率和成功率动态调整

#### 9. **FastPath 延迟 (ns)**
- **指标**: `vm_fast_path_avg_latency_ns`, `vm_fast_path_last_latency_ns`
- **说明**: FastPath零冲突路径的纳秒级延迟
- **目标**: < 500ns (超低延迟)

#### 10. **Fast→Consensus 回退率**
- **指标**: `vm_fast_fallback_ratio`
- **说明**: FastPath回退到Consensus的比例
- **阈值**:
  - 🟢 Green: < 5% (正常)
  - 🟡 Yellow: 5-10% (轻微问题)
  - 🔴 Red: > 10% (严重问题)
- **告警**: > 10% 触发路由配置检查

#### 11. **AdaptiveRouter 自适应调整次数**
- **指标**: `vm_routing_adaptive_adjustments_total`
- **说明**: 自适应路由器累计调整FastPath目标比例的次数
- **分析**: 调整过于频繁 (>10次/分钟持续10分钟) 触发信息告警,建议调优conflict_high/low阈值

### 🔒 ZK隐私验证 (Row 3)

#### 12. **RingCT 证明吞吐量**
- **指标**: `vm_privacy_zk_parallel_tps`, `rate(vm_privacy_zk_parallel_proof_total[1m])`
- **说明**: ZK证明并行验证的TPS
- **目标**: ≥ 50 proofs/sec (目标), ≥ 30 proofs/sec (可接受)
- **告警**: < 30 触发性能检查

#### 13. **ZK验证延迟 (Groth16)**
- **指标**: `vm_privacy_zk_verify_avg_latency_ms`, `vm_privacy_zk_verify_p50/p95_latency_ms`
- **说明**: Groth16 BLS12-381证明验证延迟
- **目标**: P95 < 10ms, P50 < 5ms
- **告警**: P95 > 15ms 触发警告

#### 14. **ZK批量验证延迟**
- **指标**: `vm_privacy_zk_batch_verify_batch_latency_ms`, `vm_privacy_zk_batch_verify_avg_latency_ms`
- **说明**: 批量验证总延迟和单个证明平均延迟
- **优化**: 批量大小 = 批量总延迟 / 单个平均延迟

#### 15. **ZK验证失败率**
- **指标**: `vm_zk_verify_failure_rate`
- **说明**: ZK证明验证失败比例
- **阈值**:
  - 🟢 Green: < 1% (正常)
  - 🟡 Yellow: 1-5% (警告)
  - 🔴 Red: > 5% (严重)
- **告警**: > 5% 触发证明生成器/验证器检查

#### 16. **ZK后端类型分布 (饼图)**
- **指标**: `vm_zk_backend_count{backend="groth16-bls12-381|plonk|mock"}`
- **说明**: 使用的ZK后端类型分布
- **生产环境**: 应100%为groth16-bls12-381,避免使用mock

#### 17. **ZK验证统计**
- **指标**: `vm_privacy_zk_verify_count_total`, `vm_privacy_zk_verify_window_size`
- **说明**: 累计验证总数和统计窗口大小

### 💾 存储与GC (Row 4)

#### 18. **MVCC垃圾回收**
- **指标**: `rate(mvcc_gc_runs_total[1m])`, `rate(mvcc_gc_versions_cleaned_total[1m])`
- **说明**: GC运行频率和版本清理速率
- **告警**: GC频率 > 100次/秒持续10分钟,建议调整GC阈值

#### 19. **MVCC Flush统计**
- **指标**: `rate(mvcc_flush_count_total[1m])`, `rate(mvcc_flush_keys_total[1m])`
- **说明**: Flush到RocksDB的频率和键数量

#### 20. **MVCC Flush字节数**
- **指标**: `rate(mvcc_flush_bytes_total[1m])`
- **说明**: 每秒Flush到持久化存储的字节数
- **分析**: 持续高Flush速率可能影响整体性能

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
