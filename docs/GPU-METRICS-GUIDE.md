# GPU 加速指标与监控系统指南

> **Phase 13 M13.6** | GPU Metrics & Observability  
> 完成度：60% | 创建时间：2025-11-11

---

## 📊 概述

SuperVM GPU 加速指标系统提供全面的 GPU 执行监控，覆盖 Crypto、Merkle、ZK 三大模块。通过 Prometheus + Grafana 技术栈实现实时监控、性能分析和故障告警。

### 核心功能

- **✅ 35+ 指标维度**：执行计数、延迟分布、失败率、回退率、队列深度、内存使用

- **✅ 三模块覆盖**：Crypto (SHA256/Keccak256)、Merkle (树构建)、ZK (证明生成)

- **✅ 实时监控**：5 秒刷新，毫秒级延迟跟踪

- **✅ 多级告警**：14 条告警规则 (Critical/Warning/Info)

- **✅ 可视化**：19 个 Grafana 面板，覆盖总览、模块、资源、统计

---

## 🏗️ 架构设计

```

┌──────────────────────────────────────────────────────────────┐
│                    SuperVM Runtime                            │
│  ┌────────────┐  ┌────────────┐  ┌────────────┐            │
│  │ Crypto Ops │  │ Merkle Ops │  │  ZK Prove  │            │
│  └─────┬──────┘  └─────┬──────┘  └─────┬──────┘            │
│        │                │                │                    │
│        └────────────────┼────────────────┘                    │
│                         ▼                                     │
│         ┌───────────────────────────────┐                    │
│         │   GpuMetrics (gpu_metrics.rs) │                    │
│         │  - record_crypto_gpu()         │                    │
│         │  - record_merkle_gpu()         │                    │
│         │  - record_zk_gpu()             │                    │
│         │  - export_prometheus()         │                    │
│         └───────────────┬───────────────┘                    │
│                         │                                     │
└─────────────────────────┼─────────────────────────────────────┘
                          │ HTTP /metrics
                          ▼
         ┌────────────────────────────────┐
         │       Prometheus Server        │
         │  - Scrape metrics every 15s    │
         │  - Evaluate alert rules        │
         │  - Store time-series data      │
         └────────────┬───────────────────┘
                      │ PromQL Query
                      ▼
         ┌────────────────────────────────┐
         │       Grafana Dashboard        │
         │  - 19 panels across 6 rows     │
         │  - Real-time visualization     │
         │  - Alert integration           │
         └────────────────────────────────┘

```

---

## 📦 指标定义

### 全局指标

| 指标名称 | 类型 | 描述 | 单位 |
|---------|------|------|------|
| `gpu_available` | Gauge | GPU 设备是否可用 (0/1) | bool |
| `gpu_device_name` | Label | GPU 设备名称 | string |
| `gpu_total_executions` | Counter | 总执行次数 (所有模块) | count |
| `gpu_total_failures` | Counter | 总失败次数 | count |
| `gpu_failure_rate` | Gauge | 失败率 (failures / total) | ratio |
| `gpu_cpu_fallback_rate` | Gauge | CPU 回退率 | ratio |
| `gpu_queue_depth` | Gauge | 当前队列深度 | count |
| `gpu_memory_used_bytes` | Gauge | GPU 内存使用量 | bytes |
| `gpu_total_exec_time_seconds` | Counter | 累计执行时间 | seconds |

### Crypto 模块指标

| 指标名称 | 类型 | 描述 |
|---------|------|------|
| `gpu_crypto_executions_total` | Counter | Crypto GPU 执行总数 |
| `gpu_crypto_failures_total` | Counter | Crypto GPU 失败总数 |
| `gpu_crypto_cpu_fallback_total` | Counter | Crypto CPU 回退总数 |
| `gpu_crypto_last_batch_size` | Gauge | 最后一次批量大小 |
| `gpu_crypto_last_latency_ms` | Gauge | 最后一次延迟 (ms) |
| `gpu_crypto_latency_avg_ms` | Gauge | 平均延迟 (ms) |
| `gpu_crypto_latency_min_ms` | Gauge | 最小延迟 (ms) |
| `gpu_crypto_latency_max_ms` | Gauge | 最大延迟 (ms) |

### Merkle 模块指标

| 指标名称 | 类型 | 描述 |
|---------|------|------|
| `gpu_merkle_executions_total` | Counter | Merkle GPU 执行总数 |
| `gpu_merkle_failures_total` | Counter | Merkle GPU 失败总数 |
| `gpu_merkle_cpu_fallback_total` | Counter | Merkle CPU 回退总数 |
| `gpu_merkle_last_tree_size` | Gauge | 最后一次树大小 (叶子数) |
| `gpu_merkle_last_latency_ms` | Gauge | 最后一次延迟 (ms) |
| `gpu_merkle_latency_avg_ms` | Gauge | 平均延迟 (ms) |
| `gpu_merkle_latency_min_ms` | Gauge | 最小延迟 (ms) |
| `gpu_merkle_latency_max_ms` | Gauge | 最大延迟 (ms) |

### ZK 模块指标

| 指标名称 | 类型 | 描述 |
|---------|------|------|
| `gpu_zk_executions_total` | Counter | ZK GPU 执行总数 |
| `gpu_zk_failures_total` | Counter | ZK GPU 失败总数 |
| `gpu_zk_cpu_fallback_total` | Counter | ZK CPU 回退总数 |
| `gpu_zk_last_constraints` | Gauge | 最后一次约束数量 |
| `gpu_zk_last_latency_ms` | Gauge | 最后一次延迟 (ms) |
| `gpu_zk_latency_avg_ms` | Gauge | 平均延迟 (ms) |
| `gpu_zk_latency_min_ms` | Gauge | 最小延迟 (ms) |
| `gpu_zk_latency_max_ms` | Gauge | 最大延迟 (ms) |

---

## 🚀 快速部署

### 1. 启用指标收集 (代码集成)

在你的 GPU executor 模块中集成指标收集：

```rust
use gpu_executor::gpu_metrics::GpuMetrics;

// 初始化指标
let metrics = GpuMetrics::new();
metrics.set_device_available(true, "NVIDIA RTX 4090");

// Crypto 操作记录
let start = std::time::Instant::now();
let result = crypto_executor.execute_gpu(batch);
let latency = start.elapsed();

if result.is_ok() {
    metrics.record_crypto_gpu(batch.len(), latency, false);
} else {
    metrics.record_crypto_gpu(batch.len(), latency, true);
}

// Merkle 操作记录
let start = std::time::Instant::now();
let result = merkle_executor.execute_gpu(leaves);
let latency = start.elapsed();

if result.is_ok() {
    metrics.record_merkle_gpu(leaves.len(), latency, false);
} else {
    // CPU 回退
    metrics.record_merkle_cpu_fallback(leaves.len());
}

// ZK 证明记录
let start = std::time::Instant::now();
let result = zk_prover.prove_gpu(constraints);
let latency = start.elapsed();

metrics.record_zk_gpu(constraints, latency, result.is_err());

// 导出 Prometheus 格式
let prometheus_output = metrics.export_prometheus();

```

### 2. 暴露 /metrics HTTP 端点

在 `vm-runtime` 中添加 metrics 端点：

```rust
// src/metrics_server.rs
use axum::{routing::get, Router};
use gpu_executor::gpu_metrics::GpuMetrics;

pub async fn start_metrics_server(
    metrics: Arc<GpuMetrics>,
    port: u16,
) -> Result<()> {
    let app = Router::new()
        .route("/metrics", get({
            let metrics = metrics.clone();
            move || async move {
                metrics.export_prometheus()
            }
        }))
        .route("/health", get(|| async { "OK" }));

    let addr = format!("0.0.0.0:{}", port);
    axum::Server::bind(&addr.parse().unwrap())
        .serve(app.into_make_service())
        .await?;
    
    Ok(())
}

```

配置启动（默认端口 9090）：

```rust
// main.rs
let metrics = Arc::new(GpuMetrics::new());
tokio::spawn(start_metrics_server(metrics.clone(), 9090));

```

### 3. 配置 Prometheus

创建 `prometheus-gpu.yml`：

```yaml
global:
  scrape_interval: 15s
  evaluation_interval: 15s

scrape_configs:
  - job_name: 'supervm-gpu'
    static_configs:
      - targets: ['localhost:9090']
        labels:
          instance: 'supervm-gpu-node1'
          environment: 'production'

rule_files:
  - 'prometheus-gpu-alerts.yml'

alerting:
  alertmanagers:
    - static_configs:
        - targets: ['localhost:9093']

```

启动 Prometheus：

```bash

# 下载 Prometheus (Windows)

wget https://github.com/prometheus/prometheus/releases/download/v2.45.0/prometheus-2.45.0.windows-amd64.zip
Expand-Archive prometheus-2.45.0.windows-amd64.zip
cd prometheus-2.45.0.windows-amd64

# 复制配置文件

cp d:\WEB3_AI开发\虚拟机开发\prometheus-gpu.yml .
cp d:\WEB3_AI开发\虚拟机开发\prometheus-gpu-alerts.yml .

# 启动服务

.\prometheus.exe --config.file=prometheus-gpu.yml --web.listen-address=:9091

```

验证：访问 http://localhost:9091/targets 确认 SuperVM target 为 UP

### 4. 导入 Grafana Dashboard

1. **启动 Grafana**：

```bash

# 下载 Grafana (Windows)

wget https://dl.grafana.com/oss/release/grafana-10.0.0.windows-amd64.zip
Expand-Archive grafana-10.0.0.windows-amd64.zip
cd grafana-10.0.0

# 启动服务

.\bin\grafana-server.exe

```

访问 http://localhost:3000 (默认用户名/密码: admin/admin)

2. **添加数据源**：

- 进入 `Configuration > Data Sources`

- 添加 Prometheus

- URL: `http://localhost:9091`

- 点击 `Save & Test`

3. **导入 Dashboard**：

- 进入 `Create > Import`

- 上传文件：`d:\WEB3_AI开发\虚拟机开发\grafana-gpu-acceleration-dashboard.json`

- 选择 Prometheus 数据源

- 点击 `Import`

---

## 📈 Dashboard 面板说明

### Row 1: GPU 总览

- **GPU 可用性 (Stat)**: 设备状态 (可用/不可用)，背景色指示

- **GPU 总执行次数 (Stat)**: 累计执行数，含趋势图

- **GPU 失败率 (Gauge)**: 0-100% 量表，阈值：绿色 < 1% < 黄色 < 5% < 红色

- **CPU 回退率 (Gauge)**: 0-100% 量表，阈值：绿色 < 10% < 黄色 < 30% < 红色

### Row 2: Crypto 模块

- **Crypto GPU 执行吞吐量 (Timeseries)**: GPU 执行/秒 vs CPU 回退/秒

- **Crypto 延迟分布 (Timeseries)**: 最后一次延迟 vs 平均延迟

### Row 3: Merkle 模块

- **Merkle GPU 执行吞吐量 (Timeseries)**: GPU 执行/秒 vs CPU 回退/秒

- **Merkle 延迟与树大小 (Timeseries)**: 双 Y 轴 (延迟 ms / 树大小叶子数)

### Row 4: ZK 模块

- **ZK GPU 证明吞吐量 (Timeseries)**: GPU 证明/秒 vs CPU 回退/秒

- **ZK 延迟与约束规模 (Timeseries)**: 双 Y 轴 (延迟 ms / 约束数 K)

### Row 5: 队列与资源

- **GPU 队列深度 (Timeseries)**: 实时队列深度，阈值线 (10/50)

- **GPU 内存使用 (Timeseries)**: 内存使用 MB

### Row 6: 综合统计

- **模块执行分布 (Pie Chart)**: Crypto / Merkle / ZK 占比

- **失败统计 (Stat)**: 各模块失败计数，背景色告警

- **GPU 总执行时间 (Stat)**: 累计执行时间秒

---

## 🚨 告警规则说明

### Critical 级别 (4 条)

| 告警名称 | 触发条件 | 持续时间 | 影响 |
|---------|---------|---------|------|
| GPUDeviceUnavailable | `gpu_available == 0` | 1m | 所有任务回退 CPU，性能下降 5-20x |
| GPUHighMemoryUsage | `gpu_memory_used_bytes > 3GB` | 5m | OOM 风险，buffer 分配失败 |
| GPUConsecutiveFailures | `increase(failures[1m]) > 50` | 2m | GPU 执行路径不可用 |

### Warning 级别 (7 条)

| 告警名称 | 触发条件 | 持续时间 | 影响 |
|---------|---------|---------|------|
| GPUHighFailureRate | `failure_rate > 5%` | 5m | 部分任务回退，性能下降 |
| GPUHighCPUFallbackRate | `fallback_rate > 20%` | 5m | 加速效果不达预期 |
| CryptoGPUHighFailureRate | `crypto_failure_rate > 10%` | 5m | Crypto 加速受损 |
| MerkleGPUHighFailureRate | `merkle_failure_rate > 10%` | 5m | Merkle 加速受损 |
| ZKGPUHighFailureRate | `zk_failure_rate > 10%` | 5m | ZK 加速受损 |
| CryptoGPUHighLatency | `crypto_avg_latency > 100ms` | 5m | Crypto 性能下降 |
| MerkleGPUHighLatency | `merkle_avg_latency > 500ms` | 5m | Merkle 性能下降 |
| ZKGPUHighLatency | `zk_avg_latency > 2000ms` | 5m | ZK 性能下降 |

### Info 级别 (3 条)

| 告警名称 | 触发条件 | 持续时间 | 影响 |
|---------|---------|---------|------|
| GPUHighQueueDepth | `queue_depth > 100` | 10m | 等待时间增加 |
| GPULowUtilization | `rate(total_executions) < 10` | 15m | 资源未充分利用 |
| GPUThroughputDrop | `吞吐量下降 > 50%` | 5m | 加速效果减弱 |

---

## 🔍 常见问题排查

### Q1: Prometheus target 显示 DOWN

**排查步骤**：

1. 检查 SuperVM metrics server 是否启动：
   ```bash
   curl http://localhost:9090/health
   # 预期输出: OK
   ```

2. 检查防火墙规则：
   ```powershell
   netsh advfirewall firewall add rule name="SuperVM Metrics" dir=in action=allow protocol=TCP localport=9090
   ```

3. 检查日志是否有 bind 错误：
   ```bash
   tail -f supervm.log | grep "metrics_server"
   ```

### Q2: Dashboard 面板显示 "No Data"

**排查步骤**：

1. 验证 Prometheus 是否收集到数据：
   ```
   # 访问 Prometheus UI: http://localhost:9091
   # 执行查询: gpu_available
   # 应有结果返回
   ```

2. 检查 Grafana 数据源配置：
   ```
   # Grafana > Configuration > Data Sources > Prometheus
   # 点击 "Save & Test"，确认 "Data source is working"
   ```

3. 检查指标名称是否匹配：
   ```bash
   curl http://localhost:9090/metrics | grep gpu_
   # 确认所有指标都存在
   ```

### Q3: 告警未触发

**排查步骤**：

1. 确认告警规则已加载：
   ```
   # 访问 Prometheus UI: http://localhost:9091/rules
   # 查看 gpu_acceleration_alerts 组是否存在
   ```

2. 检查告警状态：
   ```
   # Prometheus UI > Alerts
   # 查看各告警的 State (Inactive/Pending/Firing)
   ```

3. 验证 Alertmanager 配置：
   ```bash
   curl http://localhost:9093/api/v1/status
   ```

### Q4: GPU 失败率持续高于阈值

**诊断步骤**：

1. 查看 GPU executor 日志：
   ```bash
   tail -f gpu-executor.log | grep ERROR
   ```

2. 检查常见失败原因：
   - **Buffer 分配失败**: 内存不足，减少批量大小
   - **Shader 编译错误**: WGSL 语法问题，检查 shader 代码
   - **Device Lost**: GPU 驱动崩溃，重启应用或更新驱动
   - **Timeout**: 计算超时，增加 timeout 配置

3. 启用详细日志：
   ```bash
   RUST_LOG=gpu_executor=debug cargo run
   ```

### Q5: Merkle GPU 延迟高于 CPU

**优化建议**：

1. 调整批量大小阈值（当前建议 >= 1024 叶子）：
   ```rust
   if tree_size < 1024 {
       return cpu_executor.execute(leaves);
   }
   ```

2. 检查 layer 迭代逻辑：
   - 是否有不必要的 buffer mapping
   - 是否能合并多个 layer 到一个 dispatch

3. 对比 benchmark 数据：
   ```bash
   cargo run --release --example merkle_bench
   # 查看 CPU vs GPU 性能对比
   ```

---

## 📊 性能基准

### M13.3 Crypto 加速效果 (已完成)

| 操作 | CPU Baseline | GPU (1K batch) | 加速比 |
|-----|-------------|----------------|-------|
| SHA-256 | 15.2 ms | 2.8 ms | **5.4x** |
| Keccak-256 | 18.7 ms | 3.1 ms | **6.0x** |

### M13.4 Merkle 加速效果 (80% 完成)

| 树大小 | CPU Baseline | GPU 目标 | 加速比目标 |
|-------|-------------|---------|----------|
| 1K 叶子 | 0.778 ms | ~0.15 ms | **5x** |
| 16K 叶子 | 13.2 ms | ~2.0 ms | **6.5x** |
| 256K 叶子 | 220 ms | ~30 ms | **7x** |

### M13.5 ZK 加速效果 (30% 完成)

| 约束规模 | CPU Baseline | GPU 目标 | 加速比目标 |
|---------|-------------|---------|----------|
| 10K | 45 ms | ~5 ms | **9x** |
| 100K | 520 ms | ~30 ms | **17x** |
| 1M | 8100 ms | ~350 ms | **23x** |

---

## 🛠️ 高级配置

### 自定义指标导出间隔

```rust
// 在 metrics_server 中配置
pub struct MetricsConfig {
    pub export_interval: Duration,  // 默认 15s
    pub aggregation_window: Duration,  // 默认 1m
}

```

### 集成 Alertmanager 通知

编辑 `alertmanager.yml`：

```yaml
route:
  group_by: ['alertname', 'severity']
  group_wait: 10s
  group_interval: 10s
  repeat_interval: 12h
  receiver: 'gpu-alerts-webhook'

receivers:
  - name: 'gpu-alerts-webhook'
    webhook_configs:
      - url: 'http://your-webhook-endpoint/alerts'
        send_resolved: true

```

### 多 GPU 设备标签

```rust
metrics.set_device_available(true, "GPU-0:NVIDIA-RTX-4090");
metrics.set_device_available(true, "GPU-1:NVIDIA-RTX-3090");

```

Dashboard 查询更新：

```promql
gpu_available{device=~"GPU-.*"}

```

---

## 📚 下一步工作

### M13.6 剩余任务 (40% → 100%)

- [ ] **集成测试**: 端到端测试 metrics 收集与导出

- [ ] **Dashboard 截图**: 生成示例图表添加到本文档

- [ ] **性能测试**: 验证 metrics 开销 < 1% 执行时间

- [ ] **文档完善**: 添加 PromQL 查询示例库

### M13.7-M13.9 后续工作

- [ ] **一致性监控**: 添加 CPU vs GPU 结果对比指标

- [ ] **多 GPU 监控**: 设备级别的负载均衡指标

- [ ] **跨平台指标**: Vulkan/Metal backend 性能对比

---

## 📖 参考资料

- [Prometheus 查询语法](https://prometheus.io/docs/prometheus/latest/querying/basics/)

- [Grafana Dashboard 最佳实践](https://grafana.com/docs/grafana/latest/best-practices/)

- [WGPU Profiling Guide](https://wgpu.rs/doc/wgpu/struct.Device.html#method.create_query_set)

- [Phase 13 技术路线](docs/ARCHITECTURE-INTEGRATION-ANALYSIS.md#phase-13)

---

**文档版本**: 1.0  
**最后更新**: 2025-11-11  
**维护者**: SuperVM GPU Acceleration Team
