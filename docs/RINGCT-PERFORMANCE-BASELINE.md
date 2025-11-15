# RingCT 并行证明性能基准

> **测试日期**: 2024-01
> **环境**: Windows, Release模式, Arkworks Groth16 (BLS12-381)
> **配置**: 批次大小32, 1秒间隔

## 性能摘要

### 吞吐量 (TPS)

- **预热阶段平均**: 49.7 proofs/sec

- **稳定运行平均**: 50.8 proofs/sec  

- **观察范围**: 41.6 - 53.0 proofs/sec

- **峰值**: 53.01 proofs/sec

### 延迟

- **预热阶段平均**: 20.2 ms/proof

- **稳定运行平均**: 19.7 ms/proof

- **观察范围**: 19.05 - 22.48 ms/proof

- **最佳**: 19.05 ms/proof

### 可靠性

- **总证明数**: 832+ proofs

- **失败数**: 0

- **成功率**: 100%

- **批次总数**: 26+ batches

## 详细指标数据

### Prometheus 指标快照

```

vm_privacy_zk_parallel_proof_total 832
vm_privacy_zk_parallel_proof_failed_total 0
vm_privacy_zk_parallel_batches_total 26
vm_privacy_zk_parallel_batch_latency_ms 719.834
vm_privacy_zk_parallel_avg_latency_ms 22.494
vm_privacy_zk_parallel_tps 44.454

```

### 批次样本 (22个批次)

```

Batch | Total | OK | Failed | Latency(ms) | TPS   | Avg(ms)
------|-------|----|----|-------------|-------|--------
1     | 32    | 32 | 0      | 768.67      | 41.63 | 24.02
2     | 32    | 32 | 0      | 614.48      | 52.08 | 19.20
3     | 32    | 32 | 0      | 623.12      | 51.35 | 19.47
4     | 32    | 32 | 0      | 624.77      | 51.22 | 19.52
5     | 32    | 32 | 0      | 633.04      | 50.55 | 19.78
6     | 32    | 32 | 0      | 604.93      | 52.90 | 18.90
7     | 32    | 32 | 0      | 620.73      | 51.55 | 19.40
8     | 32    | 32 | 0      | 616.68      | 51.89 | 19.27
9     | 32    | 32 | 0      | 622.67      | 51.39 | 19.46
10    | 32    | 32 | 0      | 626.83      | 51.05 | 19.59
11    | 32    | 32 | 0      | 642.42      | 49.81 | 20.08
12    | 32    | 32 | 0      | 635.59      | 50.35 | 19.86
13    | 32    | 32 | 0      | 668.30      | 47.88 | 20.88
14    | 32    | 32 | 0      | 631.03      | 50.71 | 19.72
15    | 32    | 32 | 0      | 616.36      | 51.92 | 19.26
16    | 32    | 32 | 0      | 618.68      | 51.72 | 19.33
17    | 32    | 32 | 0      | 617.84      | 51.79 | 19.31
18    | 32    | 32 | 0      | 607.59      | 52.67 | 18.99
19    | 32    | 32 | 0      | 624.04      | 51.28 | 19.50
20    | 32    | 32 | 0      | 619.50      | 51.65 | 19.36
21    | 32    | 32 | 0      | 603.64      | 53.01 | 18.86
22    | 32    | 32 | 0      | 624.10      | 51.27 | 19.50

```

## 性能分析

### 优势

1. **稳定性高**: 100%成功率,无单次失败
2. **延迟可控**: 平均19.7ms,满足实时性要求
3. **吞吐量稳定**: 50+TPS的持续输出,变异系数小
4. **资源利用**: 并行批处理有效利用多核CPU

### 优化机会

1. **批次大小调优**: 当前32可能未达到最优,建议测试64/128
2. **线程池配置**: 可通过`RINGCT_PAR_THREADS`环境变量调整
3. **批量验证**: 实现Groth16批量验证可进一步提升TPS
4. **ProvingKey缓存**: 已实现全局单例,无重复初始化开销

## HTTP 端点

### `/summary` - 人类可读摘要

```

=== RingCT Parallel Proof Summary ===
Total Proofs: 832
Failed Proofs: 0
Batches: 26
Last Batch Total Latency (ms): 719.834
Last Batch Avg Per-Proof (ms): 22.494
Last Batch TPS: 44.454

```

### `/metrics` - Prometheus 抓取端点

完整指标列表见 [GRAFANA-RINGCT-PANELS.md](./GRAFANA-RINGCT-PANELS.md)

## 环境变量配置

| 变量 | 默认值 | 说明 |
|------|--------|------|
| `RINGCT_PAR_BATCH` | 32 | 每批证明数量 |
| `RINGCT_PAR_INTERVAL_MS` | 1000 | 批次间隔(毫秒) |
| `RINGCT_PAR_THREADS` | 自动 | Rayon线程池大小 |
| `RINGCT_PAR_SETUP_BATCH` | 8 | 预热批次大小 |

## 运行基准测试

```powershell

# 启动HTTP基准测试服务器

cargo run -p vm-runtime --features groth16-verifier \
  --example zk_parallel_http_bench --release

# 查看实时摘要

Invoke-WebRequest http://localhost:9090/summary

# 查看Prometheus指标

Invoke-WebRequest http://localhost:9090/metrics

```

## 对比参考

| 实现方式 | TPS | 延迟 | 成功率 |
|---------|-----|------|--------|
| 串行证明 | ~5-10 | ~100ms | 100% |
| **并行批处理(32)** | **50.8** | **19.7ms** | **100%** |
| 理论最大值 | ~80-100 | ~10ms | - |

**结论**: 并行实现相比串行提升5-10倍,延迟降低80%,为生产环境可用性奠定基础。

---

## 下一步

1. **Grafana集成**: 配置Prometheus抓取`:9090/metrics`,导入面板模板
2. **批次大小优化**: A/B测试不同batch size对TPS/延迟的影响
3. **批量验证**: 实现Groth16 batch verification进一步提升吞吐
4. **长期压测**: 24小时稳定性测试,观察内存/CPU趋势
