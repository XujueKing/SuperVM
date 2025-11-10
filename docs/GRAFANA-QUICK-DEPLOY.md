# RingCT 并行证明 Grafana 监控 - 快速部署指南

## 前置条件

1. **Prometheus** - 时序数据库和指标收集器
2. **Grafana** - 可视化仪表板
3. **RingCT HTTP Bench** - 运行中的基准测试服务器

## 部署步骤

### 1. 启动 RingCT HTTP 基准测试

```powershell
# 在独立终端窗口运行
cargo run -p vm-runtime --features groth16-verifier `
  --example zk_parallel_http_bench --release

# 验证服务器运行
Invoke-WebRequest http://localhost:9090/metrics
```

### 2. 安装并配置 Prometheus

#### 下载 Prometheus (Windows)
```powershell
# 下载最新版本
Invoke-WebRequest -Uri "https://github.com/prometheus/prometheus/releases/download/v2.48.0/prometheus-2.48.0.windows-amd64.zip" `
  -OutFile "prometheus.zip"

# 解压
Expand-Archive prometheus.zip -DestinationPath .
cd prometheus-*
```

#### 使用项目配置文件
```powershell
# 复制配置文件到Prometheus目录
Copy-Item ..\prometheus-ringct.yml .\prometheus.yml

# 复制告警规则文件
Copy-Item ..\prometheus-zk-alerts.yml .\prometheus-zk-alerts.yml
```

#### 启动 Prometheus
```powershell
.\prometheus.exe --config.file=prometheus.yml
```

访问 http://localhost:9090 验证Prometheus运行。

#### 验证抓取目标
1. 打开 http://localhost:9090/targets
2. 检查 `ringct-parallel-prover` 目标状态为 **UP**

### 3. 安装并配置 Grafana

#### 下载 Grafana (Windows)
```powershell
# 下载最新版本
Invoke-WebRequest -Uri "https://dl.grafana.com/oss/release/grafana-10.2.2.windows-amd64.zip" `
  -OutFile "grafana.zip"

# 解压
Expand-Archive grafana.zip -DestinationPath .
cd grafana-*\bin
```

#### 启动 Grafana
```powershell
.\grafana-server.exe
```

访问 http://localhost:3000 (默认账号: admin/admin)

### 4. 配置 Grafana 数据源

1. 登录 Grafana (http://localhost:3000)
2. 点击 **Configuration → Data Sources**
3. 点击 **Add data source**
4. 选择 **Prometheus**
5. 配置:
   - **Name**: `Prometheus`
   - **URL**: `http://localhost:9090`
   - **Access**: `Server (default)`
6. 点击 **Save & Test**

### 5. 导入 RingCT 仪表板

#### 方法 1: UI 导入
1. 点击 **+ → Import**
2. 点击 **Upload JSON file**
3. 选择项目根目录下的 `grafana-ringct-dashboard.json`
4. 选择数据源为刚创建的 `Prometheus`
5. 点击 **Import**

#### 方法 2: Provisioning (自动化)
```powershell
# 创建 provisioning 目录
mkdir grafana-*\conf\provisioning\dashboards
mkdir grafana-*\conf\provisioning\datasources

# 复制仪表板配置
Copy-Item ..\grafana-ringct-dashboard.json `
  grafana-*\conf\provisioning\dashboards\ringct.json

# 创建数据源配置
@"
apiVersion: 1
datasources:
  - name: Prometheus
    type: prometheus
    access: proxy
    url: http://localhost:9090
    isDefault: true
"@ | Out-File grafana-*\conf\provisioning\datasources\prometheus.yml -Encoding utf8

# 重启 Grafana
```

### 6. 查看仪表板

1. 打开 http://localhost:3000
2. 点击 **Dashboards → RingCT Parallel Proof Monitoring**
3. 观察实时指标:
   - **TPS**: 实时吞吐量
   - **Latency**: 证明生成延迟
   - **Success Rate**: 成功率百分比
   - **Total Proofs**: 累计证明数
   - **Failed Proofs**: 失败证明数

## 面板说明

### Panel 1: RingCT Proof Throughput (TPS)
- **指标**: `vm_privacy_zk_parallel_tps`
- **含义**: 每秒生成的证明数
- **告警**: TPS < 30 持续5分钟

### Panel 2: RingCT Proof Latency
- **指标**: `vm_privacy_zk_parallel_avg_latency_ms`
- **含义**: 平均每个证明的生成时间
- **目标**: < 25ms

### Panel 3: RingCT Success Rate
- **计算**: `100 * (1 - failed/total)`
- **阈值**:
  - 🔴 Red: < 95%
  - 🟡 Yellow: 95-99%
  - 🟢 Green: > 99%

### Panel 4-6: 累计统计
- **Total Proofs**: 总证明数(带趋势线)
- **Failed Proofs**: 失败数(背景色告警)
- **Batches Processed**: 处理批次数

### Panel 7: Proof Generation Rate
- **5分钟平均**: 成功率 vs 失败率
- **用途**: 观察长期趋势和异常

## 告警配置

告警规则已在 `prometheus-zk-alerts.yml` 中定义:

1. **HighRingCTFailureRate**: 失败率 > 5% 持续5分钟
2. **LowRingCTThroughput**: TPS < 30 持续5分钟
3. **HighRingCTLatency**: 平均延迟 > 50ms 持续5分钟

查看告警: http://localhost:9090/alerts

## 环境变量优化

调整基准测试性能:

```powershell
# 增大批次大小 (可能提升TPS)
$env:RINGCT_PAR_BATCH = "64"

# 减少批次间隔 (更频繁的证明生成)
$env:RINGCT_PAR_INTERVAL_MS = "500"

# 指定线程数
$env:RINGCT_PAR_THREADS = "8"

# 重新启动基准测试
cargo run -p vm-runtime --features groth16-verifier `
  --example zk_parallel_http_bench --release
```

## 故障排查

### Prometheus 无法抓取指标
```powershell
# 检查 HTTP Bench 是否运行
curl http://localhost:9090/summary

# 检查防火墙
netsh advfirewall firewall add rule name="Prometheus" dir=in action=allow protocol=TCP localport=9090

# 检查 Prometheus targets
# 访问 http://localhost:9090/targets
```

### Grafana 无数据
1. 检查数据源连接: Configuration → Data Sources → Test
2. 检查查询语句: Explore → 输入 `vm_privacy_zk_parallel_tps`
3. 检查时间范围: 仪表板右上角选择 "Last 15 minutes"

### 指标不更新
```powershell
# 检查 HTTP Bench 是否仍在生成证明
Get-Process | Where-Object { $_.ProcessName -like "*zk_parallel*" }

# 查看终端输出是否有 [batch] 日志
```

## Docker 快速启动 (可选)

如果已安装Docker:

```powershell
# 启动 Prometheus
docker run -d -p 9090:9090 `
  -v ${PWD}/prometheus-ringct.yml:/etc/prometheus/prometheus.yml `
  -v ${PWD}/prometheus-zk-alerts.yml:/etc/prometheus/alerts.yml `
  prom/prometheus

# 启动 Grafana
docker run -d -p 3000:3000 `
  -e "GF_SECURITY_ADMIN_PASSWORD=admin" `
  grafana/grafana
```

## 下一步

1. **长期存储**: 配置 Prometheus remote_write 到 VictoriaMetrics/Thanos
2. **告警通知**: 集成 Alertmanager 发送邮件/Slack通知
3. **多实例监控**: 添加生产环境的 RingCT 节点到抓取目标
4. **性能基线**: 建立TPS/延迟基线,用于容量规划

## 参考文档

- [GRAFANA-RINGCT-PANELS.md](./GRAFANA-RINGCT-PANELS.md) - 详细面板配置
- [RINGCT-PERFORMANCE-BASELINE.md](./RINGCT-PERFORMANCE-BASELINE.md) - 性能基准数据
- [prometheus-zk-alerts.yml](../prometheus-zk-alerts.yml) - 完整告警规则
- [Prometheus 文档](https://prometheus.io/docs/)
- [Grafana 文档](https://grafana.com/docs/)
