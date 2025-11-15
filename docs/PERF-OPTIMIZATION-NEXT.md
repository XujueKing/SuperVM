# 后续性能优化清单 (PERF-OPTIMIZATION-NEXT)

> **文档状态**: 活跃维护 | **最后更新**: 2025-11-12  
> **关联里程碑**: L0 Core Engine 100% 完成后的增量优化  
> **优先级原则**: 不影响当前里程碑判定，渐进式提升性能上限与生产可靠性

---

## 📋 概述

本文档面向已达成"功能完成 + 当前阶段性能目标"的 L0 子系统（**FastPath** 与 **Parallel Prover**），收纳后续可选优化事项。这些优化不阻塞当前发布，但能在真实复杂负载、资源亲和、观测性和可靠性方面进一步提升体验与性能上限。

### 适用范围

| 子系统 | 当前状态 | 源码路径 | 优化方向 |
|--------|---------|---------|---------|
| **FastPath 执行器** | ✅ 100% | `src/vm-runtime/src/parallel.rs` (`FastPathExecutor`) | 自适应调优、NUMA 亲和、拥塞控制、真实负载矩阵 |
| **Parallel Prover** | ✅ 100% | `src/vm-runtime/src/privacy/parallel_prover.rs` | 聚合验证、池化复用、GPU 探索、证明缓存 |

### 核心原则

- ✅ **兼容性优先**: 不破坏已公布 API 的向后兼容性

- 🎯 **灰度可控**: 通过 feature flag 或环境变量控制启用，默认保守

- 📊 **可观测性**: 新增优化必须配套指标与回归测试

- 🔄 **渐进迭代**: 优先轻量级改进，避免大规模重构

---

---

## 🚀 FastPath 优化路线图

> **当前基准**: 多核分区 2.58M → 5.96M → 6.92M TPS (2/4/8 分区)  
> **参考代码**: `src/vm-runtime/src/parallel.rs` (`FastPathExecutor`)  
> **基准脚本**: `examples/partitioned_fast_path_bench.rs`, `mixed_path_bench.rs`

### 1️⃣ 自适应分区与线程缩放 【优先级: 高】

**目标**: 根据实时吞吐与冲突率动态调整 worker 数量与批次大小，避免固定配置的次优状态。

**实现要点**:

- 监控指标: 每分区 TPS、队列长度、冲突重试率、CPU 利用率

- 调整策略:
  - 冲突率 > 15% → 增加分区数（减少热点竞争）
  - 队列积压 > 阈值 → 增加 worker 线程
  - CPU 利用率 < 50% → 减少线程（避免调度开销）

- 灰度方案: 环境变量 `FASTPATH_ADAPTIVE_PARTITION=1`

**预期收益**: 峰值 TPS +10-20%，平均延迟降低 15-25%

**验证方式**:

```bash

# 自适应 vs 固定分区对比

FASTPATH_ADAPTIVE_PARTITION=1 cargo run --example partitioned_fast_path_bench --release --features partitioned-fastpath

```

---

### 2️⃣ NUMA 亲和与绑核策略 【优先级: 中】

**目标**: 在多 NUMA 节点服务器上避免跨节点内存访问，降低延迟方差。

**实现要点**:

- 使用 `libnuma` 或 `hwloc` 探测拓扑

- 策略:
  - 每个分区绑定到同一 NUMA 节点的核心
  - 工作队列使用本地内存分配（`numa_alloc_local`）

- Feature gate: `numa-affinity` (可选依赖)

**预期收益**: p99 延迟降低 20-30%，吞吐稳定性提升

**注意事项**: 仅在 ≥2 NUMA 节点且高负载场景显著；单节点或低负载可能无增益甚至劣化

---

### 3️⃣ 拥塞控制与自适应退避 【优先级: 高】

**目标**: 高冲突/热键场景下防止重试风暴，平滑降级。

**实现要点**:

- 指数退避 + 抖动: `delay = min(base * 2^retry_count + rand(), max_delay)`

- 热键检测: 统计冲突 top-K 键，对热键降低优先级或路由到专用队列

- 限速策略: 超过冲突阈值时暂停新任务提交 10-50ms

**预期收益**: 极端热点场景 TPS 降幅从 -60% 收窄到 -30%，避免雪崩

**配置示例**:

```bash
FASTPATH_CONGESTION_CONTROL=exponential \
FASTPATH_MAX_RETRIES=5 \
cargo run --example mixed_path_bench --release

```

---

### 4️⃣ 真实复杂工作负载矩阵 【优先级: 高】

**目标**: 补充当前合成基准，引入生产级混合负载测试。

**负载维度**:

- 事务类型: 转账/铸造/销毁/合约调用/跨分片（比例可配）

- 读写比: 纯读 / 读写混合 (90:10, 50:50, 10:90)

- 长尾分布: Zipf 热键分布 (α=0.8, 1.2, 1.5)

- 事务大小: 短事务 (<10 指令) vs 长事务 (>100 指令)

**实施步骤**:
1. 新增 `examples/realistic_workload_bench.rs`
2. 参数化生成器: `--workload-type=defi|game|nft --zipf-alpha=1.2 --read-ratio=0.8`
3. 与现有基准对比，生成性能矩阵报告

**预期产出**: 

- 发现极端场景的性能瓶颈（如 Zipf α=1.5 时 TPS 下降 40%）

- 为自适应调优提供真实参数

---

### 5️⃣ 预测调度与优先级路由 【优先级: 中】

**目标**: 基于历史冲突/命中率预判，提前路由低冲突事务到 FastPath，高冲突事务降级 Consensus。

**实现要点**:

- 训练轻量模型（如简单贝叶斯或查表）: 键前缀 → 冲突概率

- 路由决策: 冲突预测 > 阈值 → 直接走 Consensus 通道

- 冷启动: 前 1000 笔事务随机采样，累积统计后启用预测

**预期收益**: FastPath 命中率 +5-10%，整体延迟降低 8-12%

**风险**: 模型误判导致本可 FastPath 的事务走慢路径；需 A/B 测试验证

---

### 6️⃣ 延迟分位强化与慢请求追踪 【优先级: 中】

**目标**: 补充现有平均延迟指标，暴露长尾问题。

**新增指标**:

- `fastpath_latency_ms{quantile="0.5|0.9|0.95|0.99"}`

- `fastpath_slow_requests_total` (延迟 > p99 阈值的计数)

- 追踪 ID: 为 p99 以上的请求生成 trace ID，记录详细执行路径

**工具集成**:

- Prometheus Histogram (已有 LatencyHistogram 基础)

- 可选: Jaeger/OpenTelemetry 集成 (feature gate)

**验证命令**:

```bash
curl http://localhost:8080/metrics | grep fastpath_latency

```

---

### 7️⃣ 可观测性增强 【优先级: 高】

**补充指标**:

- 队列指标: `fastpath_queue_length`, `fastpath_queue_wait_time_ms`

- 批次指标: `fastpath_batch_size{partition}` (当前批次大小)

- 资源指标: `fastpath_memory_usage_bytes`, `fastpath_worker_cpu_usage`

- 重试/丢弃: `fastpath_retries_total`, `fastpath_dropped_total`

**可视化**:

- Grafana 新增 FastPath 专用 dashboard (参考现有 `grafana-2pc-cross-shard-dashboard.json`)

- 面板: TPS 趋势、延迟热力图、分区负载均衡、重试率告警

---

---

## 🔐 Parallel Prover 优化路线图

> **当前基准**: RingCT 并行证明 50.8 proofs/sec，批量验证 104.6 verifications/sec  
> **参考代码**: `src/vm-runtime/src/privacy/parallel_prover.rs` (`ParallelProver`, `RingCtParallelProver`)  
> **演示脚本**: `examples/generate_ringct_sol_verifier.rs`, `examples/zk_parallel_http_bench.rs`

### 1️⃣ 聚合验证与批量优化 【优先级: 高】

**目标**: 多个 proof 合并验证，摊薄配对运算开销，大幅提升吞吐。

**实现要点**:

- Groth16 批量验证: 利用随机线性组合，单次配对检查 N 个 proof

- 公式: `e(∑ rᵢ·Aᵢ, B) = e(C, ∑ rᵢ·Vᵢ)` (r 为随机数)

- 集成到 `verify_batch()` 现有接口，自动切换单验证/批验证

**预期收益**: 

- 批量验证吞吐 +150-300% (从 104 → 300+ verifications/sec)

- Gas 成本降低 40-60% (EVM 环境)

**风险缓解**: 随机数生成使用密码学安全 RNG，防止伪造攻击

**验证方式**:

```bash

# 批量验证基准

cargo run --example zk_batch_verify_bench --release --features groth16-verifier -- --batch-size=16,32,64

```

---

### 2️⃣ 线程池与内存池复用 【优先级: 高】

**目标**: 消除频繁分配 proving key、工作缓冲与线程创建的开销。

**实现要点**:

- **ProvingKey 全局池**: 使用 `once_cell::sync::Lazy` 缓存常用电路的 proving key

- **工作缓冲池**: `Vec<Scalar>` 等临时内存通过 `ArrayVec` 或对象池复用

- **线程池**: 替换当前的 `rayon::spawn` 为持久化线程池 (`rayon::ThreadPoolBuilder`)

**当前问题**:

- 每次 `ParallelProver::new()` 重新分配 ~500KB proving key

- 工作缓冲频繁 malloc/free 导致内存碎片与 GC 压力

**预期收益**:

- 证明延迟降低 15-25% (减少分配耗时)

- 内存峰值降低 30-40%，GC 暂停减少

**配置示例**:

```rust
// Feature gate: prover-pool-reuse
#[cfg(feature = "prover-pool-reuse")]
static PROVING_KEY_POOL: Lazy<HashMap<CircuitId, Arc<ProvingKey>>> = ...;

```

---

### 3️⃣ GPU/SIMD 加速探索 【优先级: 中】

**目标**: 可选 GPU 加速（CUDA/OpenCL）用于 MSM/FFT，保持 CPU 回退路径。

**实施策略**:

- **阶段 1** (短期): SIMD 优化标量乘法（AVX2/NEON）
  - 使用 `packed_simd` 或 `wide` crate
  - 目标: CPU 证明速度 +20-30%

- **阶段 2** (中期): GPU MSM (Multi-Scalar Multiplication)
  - FFI 集成 `bellman-cuda` 或 `zprize-msm`
  - Feature gate: `gpu-acceleration` (可选依赖 CUDA/OpenCL)
  - 自动降级: GPU 不可用时回退 CPU 路径

- **阶段 3** (长期): 完整 GPU 电路优化

**预期收益**:

- SIMD: CPU 证明速度 +20-30%

- GPU MSM: 大环 (ring size > 64) 证明速度 +200-400%

**风险**:

- GPU 依赖增加部署复杂度 → 必须保持 CPU 路径可用

- CUDA 许可限制 → 优先 OpenCL 或 Vulkan Compute

**验证方式**:

```bash

# CPU vs SIMD vs GPU 对比

cargo bench --features simd-acceleration -- parallel_prover
cargo bench --features gpu-acceleration -- parallel_prover  # 需 GPU 环境

```

---

### 4️⃣ 证明缓存与 Memoization 【优先级: 中】

**目标**: 重复输入快速路径，避免重新生成已有 proof。

**实现要点**:

- 缓存策略: LRU 缓存 `(circuit_input_hash) → proof`

- 命中判断: SHA256 哈希输入向量，查表返回

- 缓存大小: 环境变量配置 (默认 1000 条)

- 过期策略: TTL 或基于区块高度失效

**适用场景**:

- 隐私交易中重复金额范围证明 (如频繁的 1 ETH, 10 ETH 转账)

- 测试/演示环境

**预期收益**: 

- 缓存命中时延迟 <1ms (vs 原 20ms 证明时间)

- 极端场景 (90% 命中率) 吞吐 +900%

**注意**: 生产环境需评估缓存命中率，低命中场景可能无收益

---

### 5️⃣ 超大环规模自适应分块 【优先级: 低】

**目标**: 处理 ring size > 128 的退化场景，防止单个 proof 耗时过长。

**实现要点**:

- 分块策略: ring size > 阈值 → 拆分为多个子 proof

- 聚合: 使用递归 SNARKs (Halo2) 或简单拼接

- 自适应: 根据可用内存与 CPU 核心数动态调整分块大小

**预期收益**: 超大环 (256+) 证明时间从 >10s 降至 2-3s

**风险**: 增加 proof 大小与验证复杂度；需权衡

---

### 6️⃣ 容错与重试机制 【优先级: 中】

**目标**: 单批次局部失败时隔离并重试，避免整批丢弃。

**实现要点**:

- 异常隔离: `try_prove()` 包装，捕获电路构造/证明生成错误

- 重试策略: 失败任务单独重试 3 次（指数退避）

- 降级路径: 重试失败 → 记录日志 + 返回错误（不阻塞其他 proof）

- 监控: `prover_failures_total{reason}`, `prover_retries_total`

**当前问题**: 批量证明中一个失败导致整批丢弃

**预期收益**: 可用性 +99.9% → 99.99%，失败率降低 90%

---

### 7️⃣ 可观测性增强 【优先级: 高】

**补充指标**:

- 内存指标: `prover_memory_usage_bytes`, `prover_proving_key_cache_size`

- 批次指标: `prover_batch_size{circuit}` (实际批次大小)

- 耗时分位: `prover_latency_ms{quantile, phase="setup|prove|verify"}`

- 失败统计: `prover_failures_total{reason="oom|invalid_input|timeout"}`

- 重试率: `prover_retry_rate` (重试次数 / 总任务数)

**Grafana 面板**:

- 证明吞吐趋势 (proofs/sec)

- 验证延迟热力图 (p50/p95/p99)

- 内存占用曲线（峰值/平均）

- 失败率告警 (>1% 触发)

**示例查询**:

```promql
rate(prover_proofs_total[5m])  # 5分钟平均吞吐
histogram_quantile(0.99, prover_latency_ms)  # p99延迟

```

---

---

## 📊 度量与回归测试框架

### 基准维度矩阵

| 维度 | 指标 | 工具 | 阈值示例 |
|------|------|------|---------|
| **吞吐** | TPS, ops/sec | cargo bench | ±5% 回归告警 |
| **延迟** | p50/p90/p95/p99 (ms) | Criterion, Prometheus | p99 < 100ms |
| **资源** | CPU 利用率, 内存峰值 | perf, heaptrack | 内存 < 2GB |
| **可靠性** | 失败率, 重试率 | 自定义统计 | 失败率 < 0.1% |
| **存储** | RocksDB 写放大, 压缩率 | RocksDB metrics | WAF < 10 |

### Feature 组合测试

必须覆盖的 feature 组合（防止组合爆炸导致未测功能上线）:

```bash

# 基础组合

cargo test --all-features
cargo test --no-default-features

# 关键组合

cargo test --features "rocksdb-storage,partitioned-fastpath"
cargo test --features "groth16-verifier,privacy-enhanced"
cargo test --features "rocksdb-storage,partitioned-fastpath,groth16-verifier"

# 可选组合（CI 中周期性运行）

cargo test --features "numa-affinity,prover-pool-reuse"
cargo test --features "gpu-acceleration"  # 需 GPU runner

```

### 新增测试套件建议

#### 1. 性能回归矩阵测试

**文件**: `tests/perf_matrix.rs`

**目标**: 轻量 e2e 测试，快速检测性能回归

**实现**:

```rust
#[test]
fn perf_matrix_fastpath() {
    let configs = vec![
        (2, 10000),  // (partitions, txs)
        (4, 10000),
        (8, 10000),
    ];
    for (p, n) in configs {
        let tps = run_bench(p, n);
        // 回归检测: TPS 不应低于基线 -5%
        assert!(tps >= BASELINE_TPS * 0.95);
    }
}

```

**集成**: CI 在 PR merge 前必跑

---

#### 2. 24小时稳定性测试

**文件**: `scripts/stability_test_24h.sh`

**目标**: 检测内存泄漏、资源耗尽、长期性能退化

**实现**:

```bash
#!/bin/bash

# 运行 24h 混合负载，每小时采样一次指标

for i in {1..24}; do
    cargo run --example mixed_path_bench --release -- \
        --duration=3600 --metrics-port=9090 > "metrics_hour_$i.json"
    # 检查内存是否持续增长
    check_memory_leak metrics_hour_$i.json
done

```

**触发**: 每日 cron 或手动触发（发版前必跑）

---

#### 3. 极端场景压测

**场景清单**:

- **热键风暴**: 99% 事务访问同一键

- **长尾分布**: Zipf α=2.0 (极端热点)

- **大批量**: 单批次 10,000 事务

- **内存限制**: `ulimit -v 1GB` 运行

- **网络抖动**: 模拟跨分片延迟 100-500ms

**预期产出**: 发现边界条件下的崩溃/死锁/性能悬崖

---

### CI/CD 集成

#### GitHub Actions 工作流示例

```yaml
name: Performance Regression Check

on: [pull_request]

jobs:
  perf-check:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3
      - name: Run perf matrix
        run: cargo test --test perf_matrix --release
      
      - name: Benchmark baseline comparison
        run: |
          cargo bench --bench fastpath -- --save-baseline pr-${{ github.event.number }}
          cargo bench --bench parallel_prover -- --baseline main

```

#### 回归告警

- TPS 下降 >5% → 自动评论 PR，要求解释

- p99 延迟增加 >10% → 阻塞 merge

---

---

## 🚢 升级策略与发布流程

### 优化分级与灰度控制

#### 第一级：默认关闭（探索性优化）

需要 feature flag 或环境变量显式启用，适用于：

- GPU 加速（需特定硬件）

- NUMA 亲和（仅多节点服务器受益）

- 实验性算法（如预测调度）

**配置方式**:

```toml

# Cargo.toml

[features]
default = []
numa-affinity = ["libnuma"]
gpu-acceleration = ["bellman-cuda"]
experimental-predictor = []

```

```bash

# 运行时启用

FASTPATH_ADAPTIVE_PARTITION=1 cargo run --release

```

---

#### 第二级：保守默认（稳定优化）

经过充分测试，风险可控，默认启用但可关闭，适用于：

- 线程池/内存池复用

- 聚合验证

- 延迟分位统计

**配置方式**:

```bash

# 环境变量关闭（紧急回退）

DISABLE_PROVER_POOL_REUSE=1 ./supervm

```

---

#### 第三级：强制启用（关键修复）

安全/正确性修复，无开关，适用于：

- 内存泄漏修复

- 死锁修复

- 数据一致性修复

---

### 发布检查清单

#### 发版前必测（Checklist）

- [ ] `cargo test --all-features` 全部通过

- [ ] `cargo bench` 无显著回归（±5% 容差）

- [ ] `tests/perf_matrix.rs` 通过

- [ ] 24h 稳定性测试无内存泄漏/崩溃

- [ ] 文档更新（CHANGELOG.md, README.md）

- [ ] Grafana dashboard 兼容新指标

- [ ] 向后兼容性验证（旧版本数据可读取）

#### 灰度发布流程

1. **内部测试网** (1-3 天)
   - 部署到内部节点，监控指标
   - 人工触发极端场景测试
2. **公共测试网** (1 周)
   - 5% → 20% → 50% → 100% 流量灰度
   - 关键指标对比（TPS, p99延迟, 失败率）
3. **主网发布**
   - 金丝雀部署：单节点 → 单区域 → 全网
   - 回滚预案：保留前一版本二进制

---

### 变更记录规范

#### CHANGELOG.md 模板

```markdown

## [Unreleased]

### Added - 新增功能

- FastPath: 自适应分区调整（环境变量 `FASTPATH_ADAPTIVE_PARTITION`）#1234

### Changed - 行为变更

- Parallel Prover: 默认启用线程池复用，可通过 `DISABLE_PROVER_POOL_REUSE=1` 关闭 #1235

### Performance - 性能改进

- FastPath: NUMA 亲和优化，p99 延迟降低 25%（仅多 NUMA 节点场景）#1236

- Parallel Prover: 批量验证吞吐 +180% (104 → 290 verifications/sec) #1237

### Fixed - 修复

- 修复高并发下 proving key 缓存竞态条件 #1238

### Deprecated - 废弃警告

- `ParallelProver::new()` 将在 v0.7 移除，请使用 `ParallelProver::with_pool()` #1239

```

---

### 文档同步要求

每个优化必须同步更新：
1. **技术文档**（本文档 + 相关设计文档）
2. **API 文档**（Rustdoc + `docs/API.md`）
3. **部署指南**（环境变量/feature 配置说明）
4. **README.md**（性能数据更新）
5. **Grafana Dashboard**（新增指标的面板）

---

---

## 🔗 快速验证入口

### FastPath 基准测试

#### 多核分区基准

```bash

# 基础基准（2/4/8 分区对比）

cargo run -p vm-runtime --example partitioned_fast_path_bench \
  --release --features partitioned-fastpath -- \
  --txs=200000 --partitions=2,4,8 --cycles=32

# 自适应分区测试（需实现后）

FASTPATH_ADAPTIVE_PARTITION=1 \
cargo run -p vm-runtime --example partitioned_fast_path_bench \
  --release --features partitioned-fastpath

```

#### 混合路径基准

```bash

# 调整隐私比例（0.0 = 纯 FastPath, 1.0 = 纯 Privacy）

cargo run -p vm-runtime --example mixed_path_bench \
  --release --features partitioned-fastpath,groth16-verifier -- \
  --privacy-ratio=0.2 --serve-metrics=9090

# 访问指标

curl http://localhost:9090/metrics | grep bench_

```

#### 真实负载基准（规划中）

```bash

# DeFi 场景模拟（高读低写）

cargo run --example realistic_workload_bench --release -- \
  --workload-type=defi --zipf-alpha=1.2 --read-ratio=0.9

# GameFi 场景（高冲突热键）

cargo run --example realistic_workload_bench --release -- \
  --workload-type=game --zipf-alpha=1.8 --read-ratio=0.5

```

---

### Parallel Prover 基准测试

#### RingCT 并行证明

```bash

# HTTP 基准（提供 Prometheus 导出）

cargo run -p vm-runtime --features groth16-verifier \
  --example zk_parallel_http_bench --release

# 访问端点

curl http://localhost:9090/metrics       # Prometheus 格式
curl http://localhost:9090/summary       # 人类可读摘要

```

#### 批量验证基准（规划中）

```bash

# 对比批次大小（8, 16, 32, 64）

cargo bench --bench parallel_prover -- --batch-size=8,16,32,64

```

#### Solidity 验证器生成

```bash

# BN254（当前 EVM 兼容，Gas 优化）

cargo run -p vm-runtime --features groth16-verifier \
  --example generate_bn254_multiply_sol_verifier --release

# 输出: contracts/BN254MultiplyVerifier.sol

# BLS12-381（未来 EVM 2.0，高安全）

cargo test -p vm-runtime --features groth16-verifier \
  privacy::solidity_verifier --lib -- --nocapture

# 输出: target/contracts/MultiplyVerifier.sol

```

---

### 性能监控与可视化

#### 本地 Prometheus + Grafana 快速启动

```bash

# 1. 启动带指标导出的演示服务

cargo run -p vm-runtime --example storage_metrics_http --release

# 监听: http://localhost:8080/metrics

# 2. 启动 Prometheus（需预先安装）

prometheus --config.file=prometheus-supervm-alerts.yml

# 3. 导入 Grafana Dashboard

# 文件: grafana-2pc-cross-shard-dashboard.json, grafana-ringct-dashboard.json

# 访问: http://localhost:3000

```

#### 关键指标查询示例

```promql

# FastPath TPS 趋势

rate(fastpath_txns_total[5m])

# p99 延迟

histogram_quantile(0.99, rate(fastpath_latency_ms_bucket[5m]))

# Parallel Prover 吞吐

rate(prover_proofs_total[1m])

# 内存占用趋势

process_resident_memory_bytes{job="supervm"}

```

---

### CI 集成验证

#### 性能回归检测

```bash

# 本地运行 CI 测试

cargo test --test perf_matrix --release

# 基准对比（需先保存 baseline）

cargo bench --bench fastpath -- --save-baseline main
git checkout feature-branch
cargo bench --bench fastpath -- --baseline main

```

#### Feature 组合矩阵测试

```bash

# 运行所有关键组合

./scripts/test-feature-matrix.sh

# 或手动

for features in \
  "rocksdb-storage" \
  "partitioned-fastpath,groth16-verifier" \
  "rocksdb-storage,partitioned-fastpath,groth16-verifier,consensus-optimizations"
do
  cargo test --features "$features" --release
done

```

---

## 📚 参考资料

### 内部文档

- [ROADMAP.md](../ROADMAP.md) - 完整路线图与进度跟踪

- [AUTO-TUNER.md](AUTO-TUNER.md) - 自适应性能调优设计

- [METRICS-COLLECTOR.md](METRICS-COLLECTOR.md) - 性能指标收集架构

- [CROSS-SHARD-DESIGN.md](CROSS-SHARD-DESIGN.md) - 跨分片事务设计（含 2PC 优化）

- [ARCH-CPU-GPU-HYBRID.md](ARCH-CPU-GPU-HYBRID.md) - CPU-GPU 双内核架构（Phase 8）

### 外部资源

- **NUMA 优化**: [Brendan Gregg's NUMA Blog](https://www.brendangregg.com/blog/2013-12-22/linux-numa-performance.html)

- **Groth16 批量验证**: [EIP-2537 BLS12-381 Precompiles](https://eips.ethereum.org/EIPS/eip-2537)

- **Zipf 分布**: [Anna Povzner's Workload Modeling](https://www.usenix.org/conference/fast19/presentation/wu-kan)

- **GPU MSM**: [ZPrize MSM Optimizations](https://www.zprize.io/)

---

## 🗓️ 优化排期建议

### Q1 2025（高优先级 - 快速收益）

- ✅ FastPath: 拥塞控制与自适应退避

- ✅ FastPath: 延迟分位强化 (p50/p90/p95/p99)

- ✅ Parallel Prover: 线程池/内存池复用

- ✅ Parallel Prover: 聚合验证

### Q2 2025（中优先级 - 生产化）

- ⏳ FastPath: 真实复杂工作负载矩阵

- ⏳ FastPath: 自适应分区/线程缩放

- ⏳ Parallel Prover: 容错与重试机制

- ⏳ 新增: `tests/perf_matrix.rs` + 24h 稳定性测试

### Q3 2025（探索性 - 长期投资）

- 📋 FastPath: NUMA 亲和（需多节点验证环境）

- 📋 FastPath: 预测调度

- 📋 Parallel Prover: GPU/SIMD 加速探索

- 📋 Parallel Prover: 证明缓存

### Q4 2025（高级优化 - 极限性能）

- 📋 Parallel Prover: 超大环自适应分块

- 📋 FastPath: 深度可观测性（Jaeger/OpenTelemetry）

- 📋 全栈性能剖析与火焰图自动化

---

## 📞 反馈与贡献

### 优先级调整建议

如果你认为某项优化应提前/推迟，请提交 Issue 并附上：

- 场景描述（为何该优化重要/不重要）

- 预期收益估算（TPS +X%, 延迟 -Y%）

- 实施成本评估（工作量 / 风险）

### 贡献流程

1. 从优化清单选择任务
2. 创建 feature branch: `perf/fastpath-numa-affinity`
3. 实现 + 测试 + 文档更新
4. 提交 PR，附上基准对比数据
5. Code Review + 性能回归检测通过后合并

### 联系方式

- GitHub Issues: [SuperVM Repository](https://github.com/XujueKing/SuperVM/issues)

- 技术讨论: ROADMAP.md 中维护者联系方式

---

**最后更新**: 2025-11-12  
**文档版本**: v1.0  
**下次审查**: 2025-12-12（每月更新优化进度与排期）

