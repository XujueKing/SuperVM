# RingCT 并行证明快速参考

> **快速上手指南**: SuperVM RingCT 并行证明生成与批量验证

## 核心组件

### 1. RingCtParallelProver - 并行证明生成器

```rust
use vm_runtime::privacy::parallel_prover::{RingCtParallelProver, RingCtWitness};
use vm_runtime::metrics::MetricsCollector;
use std::sync::Arc;

// 创建证明器(推荐方式: 使用全局ProvingKey缓存)
let metrics = Arc::new(MetricsCollector::new());
let prover = RingCtParallelProver::with_shared_setup()
    .with_metrics(metrics.clone());

// 准备witness数据
let witnesses: Vec<RingCtWitness> = vec![
    RingCtWitness::example(), // 实际使用替换为真实交易数据
    // ... 更多 witnesses
];

// 生成批量证明
let stats = prover.prove_batch(&witnesses);
println!("TPS: {:.2}, Avg Latency: {:.2}ms", stats.tps, stats.avg_latency_ms);
```

**配置环境变量**:
```powershell
$env:RINGCT_PAR_BATCH = "32"         # 批次大小
$env:RINGCT_PAR_THREADS = "8"        # 线程数
$env:RINGCT_PAR_INTERVAL_MS = "1000" # 批次间隔(ms)
```

### 2. BatchVerifier - 批量验证器

```rust
use vm_runtime::privacy::batch_verifier::{BatchVerifier, BatchVerifyConfig};

// 创建验证器
let config = BatchVerifyConfig {
    batch_size: 32,
    use_prepared_vk: true, // 使用预处理VK加速
};
let verifier = BatchVerifier::new(vk, config)
    .with_metrics(metrics.clone());

// 批量验证(并行优化)
let stats = verifier.verify_batch_optimized(&proofs, &public_inputs);
println!("Verified: {}/{} ({:.2} verifications/sec)",
    stats.verified, stats.total, stats.verifications_per_sec);
```

### 3. Fast→Consensus Fallback - 回退机制

```rust
use vm_runtime::supervm::SuperVM;

// 方法1: 环境变量配置
$env:SUPERVM_ENABLE_FAST_FALLBACK = "true"
$env:SUPERVM_FALLBACK_ON_ERRORS = "GasExhausted,MemoryOverflow"

let supervm = SuperVM::from_env(mvcc, gas_pool, shard_coordinator);

// 方法2: 代码配置
let supervm = SuperVM::new(mvcc, gas_pool, shard_coordinator)
    .with_fallback(&["GasExhausted".to_string()]);

// 执行交易(自动回退)
let result = supervm.execute_transaction_routed(tx, path);
```

## 性能基准

| 指标 | 值 | 配置 |
|------|---|------|
| **Proving TPS** | 50.8 proofs/sec | 批次32, Release模式 |
| **Proving延迟** | 19.7ms/proof | 平均值(范围19-23ms) |
| **Verification TPS** | 104.6 verifications/sec | 批次32,并行优化 |
| **Verification延迟** | 9.6ms/proof | 平均值 |
| **成功率** | 100% | 832+ proofs, 0 failures |

**性能提升**:
- ProvingKey缓存: 消除1-2秒重复setup开销
- 并行验证 vs 逐个验证: **8倍提升** (13.1 → 104.6/sec)

## 监控指标

### Prometheus 指标

```promql
# RingCT 并行证明TPS
vm_privacy_zk_parallel_tps

# 总证明数
vm_privacy_zk_parallel_proof_total

# 失败率
rate(vm_privacy_zk_parallel_proof_failed_total[5m]) 
  / rate(vm_privacy_zk_parallel_proof_total[5m]) * 100

# 平均延迟
vm_privacy_zk_parallel_avg_latency_ms

# 批次吞吐
rate(vm_privacy_zk_parallel_batches_total[1m])

# Fast路径回退次数
vm_fast_fallback_total
```

### HTTP 端点

启动基准测试服务器:
```powershell
cargo run -p vm-runtime --features groth16-verifier \
  --example zk_parallel_http_bench --release
```

访问端点:
- **http://localhost:9090/metrics** - Prometheus格式指标
- **http://localhost:9090/summary** - 人类可读摘要

## Grafana 仪表板

### 快速导入

1. 安装 Prometheus:
```powershell
# 使用项目配置文件
prometheus --config.file=prometheus-ringct.yml
```

2. 安装 Grafana:
```powershell
# 启动后访问 http://localhost:3000 (admin/admin)
grafana-server.exe
```

3. 导入仪表板:
   - 打开 Grafana → Dashboards → Import
   - 上传 `grafana-ringct-dashboard.json`
   - 选择 Prometheus 数据源

### 关键面板

| 面板 | 指标 | 用途 |
|------|------|------|
| **TPS** | `vm_privacy_zk_parallel_tps` | 实时吞吐监控 |
| **延迟** | `vm_privacy_zk_parallel_avg_latency_ms` | 性能瓶颈诊断 |
| **成功率** | `100 * (1 - failed/total)` | 稳定性监控 |
| **总证明数** | `vm_privacy_zk_parallel_proof_total` | 累计统计+趋势 |

详细配置见: [GRAFANA-RINGCT-PANELS.md](./GRAFANA-RINGCT-PANELS.md)

## 告警规则

### Prometheus Alerts

```yaml
# prometheus-zk-alerts.yml
groups:
  - name: ringct_alerts
    interval: 30s
    rules:
      - alert: HighRingCTFailureRate
        expr: rate(vm_privacy_zk_parallel_proof_failed_total[5m]) 
              / rate(vm_privacy_zk_parallel_proof_total[5m]) > 0.05
        for: 5m
        labels:
          severity: critical
        annotations:
          summary: "RingCT证明失败率超过5%"

      - alert: LowRingCTThroughput
        expr: vm_privacy_zk_parallel_tps < 30
        for: 5m
        labels:
          severity: warning
        annotations:
          summary: "RingCT证明TPS低于30"

      - alert: HighRingCTLatency
        expr: vm_privacy_zk_parallel_avg_latency_ms > 50
        for: 5m
        labels:
          severity: warning
        annotations:
          summary: "RingCT证明延迟超过50ms"
```

## 故障排查

### TPS低于预期

1. **检查线程配置**:
```powershell
$env:RINGCT_PAR_THREADS = "16" # 尝试增加线程数
```

2. **优化批次大小**:
```powershell
$env:RINGCT_PAR_BATCH = "64" # 尝试64或128
```

3. **检查CPU使用率**:
```powershell
Get-Process | Where-Object { $_.Name -like "*zk_parallel*" } | 
  Select-Object Name, CPU, PM
```

### 验证失败率高

1. **检查公共输入**:
```rust
// 确保公共输入与witness匹配
let c = witness.a * witness.b; // MultiplyCircuit示例
```

2. **验证ProvingKey/VerifyingKey一致性**:
```rust
// 使用相同的setup结果
let (pk, vk) = Groth16::circuit_specific_setup(circuit, &mut rng)?;
```

### 内存占用过高

1. **减小批次大小**:
```powershell
$env:RINGCT_PAR_BATCH = "16" # 降低并发度
```

2. **监控内存趋势**:
```promql
# Grafana查询
process_resident_memory_bytes{job="ringct-parallel-prover"}
```

## 最佳实践

### 1. ProvingKey管理

✅ **推荐**: 使用全局单例
```rust
RingCtParallelProver::with_shared_setup() // 自动使用RINGCT_PROVING_KEY静态变量
```

❌ **避免**: 重复setup
```rust
// 每次都setup会浪费1-2秒
RingCtParallelProver::with_default_setup() // 已标记deprecated
```

### 2. 批量验证策略

- **小批次(<10)**: 使用 `verify_individual`
- **大批次(≥32)**: 使用 `verify_batch_optimized`(并行加速)
- **PreparedVK**: 始终启用(默认),提升20-30%性能

### 3. 环境变量优先级

```
代码硬编码 < from_env()环境变量 < 运行时参数
```

建议生产环境使用环境变量,开发环境使用代码配置。

### 4. Metrics收集

始终在生产环境附加metrics:
```rust
let prover = RingCtParallelProver::with_shared_setup()
    .with_metrics(metrics.clone()); // 必须!

let verifier = BatchVerifier::new(vk, config)
    .with_metrics(metrics.clone()); // 必须!
```

## 集成示例

### 完整隐私交易流程

```rust
use vm_runtime::privacy::parallel_prover::{RingCtParallelProver, RingCtWitness};
use vm_runtime::privacy::batch_verifier::{BatchVerifier, BatchVerifyConfig};
use vm_runtime::metrics::MetricsCollector;
use std::sync::Arc;

fn process_private_transactions(txs: Vec<PrivateTx>) -> Result<(), Error> {
    let metrics = Arc::new(MetricsCollector::new());
    
    // 1. 生成证明
    let prover = RingCtParallelProver::with_shared_setup()
        .with_metrics(metrics.clone());
    
    let witnesses: Vec<RingCtWitness> = txs.iter()
        .map(|tx| RingCtWitness::from_transaction(tx))
        .collect();
    
    let proof_stats = prover.prove_batch(&witnesses);
    println!("Generated {} proofs at {:.2} TPS", 
        proof_stats.ok, proof_stats.tps);
    
    // 2. 批量验证
    let vk = get_verifying_key(); // 从配置加载
    let config = BatchVerifyConfig::default();
    let verifier = BatchVerifier::new(vk, config)
        .with_metrics(metrics.clone());
    
    let proofs = prover.get_proofs(); // 伪代码
    let public_inputs = extract_public_inputs(&txs);
    
    let verify_stats = verifier.verify_batch_optimized(&proofs, &public_inputs);
    println!("Verified {}/{} proofs at {:.2} verifications/sec",
        verify_stats.verified, verify_stats.total,
        verify_stats.verifications_per_sec);
    
    // 3. 导出 Prometheus 指标
    let prometheus_text = metrics.export_prometheus();
    serve_metrics(prometheus_text); // 在/metrics端点提供
    
    Ok(())
}
```

## 参考文档

- [RINGCT-PERFORMANCE-BASELINE.md](./RINGCT-PERFORMANCE-BASELINE.md) - 性能基准详细数据
- [GRAFANA-RINGCT-PANELS.md](./GRAFANA-RINGCT-PANELS.md) - Grafana面板配置
- [GRAFANA-QUICK-DEPLOY.md](./GRAFANA-QUICK-DEPLOY.md) - 监控系统快速部署
- [API.md](./API.md) - 完整API文档
- [CHANGELOG.md](../CHANGELOG.md) - 版本变更记录

## 联系与支持

- **Issues**: 在 GitHub 提交问题
- **文档**: docs/ 目录下的详细文档
- **示例**: examples/ 目录下的可运行示例

---

**最后更新**: 2025-01-XX | **版本**: Phase 2.3
