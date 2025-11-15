# Phase 3 Preview: Intra-batch Pipelining

日期: 2025-11-12
状态: 预研验证完成（可选开关，默认关闭）

## 背景

在 Phase 2（Bucket Cache）基础上，我们探索 Phase 3 的第一步：在单次批处理中，将上一段的读回（map/poll/CPU 侧工作）与下一段的 GPU compute 进行重叠，以降低端到端时延。

本轮实现的是“最小可行”版本：

- 不改变对外 API；

- 在 `execute_vector_add()` 内部引入“两段流水线”模式；

- 仅当 `SUPERVM_GPU_PIPELINE=1` 且批大小>1 时启用；

- 默认保持原有路径（`SUPERVM_GPU_PIPELINE` 未设置或为 `0`）。

## 运行方法

PowerShell（Windows）：

```powershell

# 关闭流水线（基线）

$env:SUPERVM_GPU_PIPELINE = "0"
cargo run -p gpu-executor --features gpu,wgpu-backend,wgpu-backend-dx12 --example batch_pipeline_bench --quiet

# 开启流水线（两段重叠）

$env:SUPERVM_GPU_PIPELINE = "1"
cargo run -p gpu-executor --features gpu,wgpu-backend,wgpu-backend-dx12 --example batch_pipeline_bench --quiet

```

可选：观察桶命中率与分布

```powershell
$env:SUPERVM_GPU_STATS = "1"

```

## 基准设置

- 批大小：24 任务/批

- 每任务元素：32,768（32K）

- 后端：DX12 可用（wgpu 0.27）；其余后端按环境可用性自动回退

- 轮次：5

## 实测结果（当前机器）

### 样本 1（初始测试）

- 基线（pipeline=0）：avg=23.50ms, p50=22.35ms, p90=28.26ms

- 流水线（pipeline=1）：avg=20.95ms, p50=20.31ms, p90=23.66ms

收益：

- 平均时延 ≈ +10.9%

- P50 ≈ +9.1%

- P90 ≈ +16.3%

### 样本 2（验证运行，2025-11-12）

- 基线（pipeline=0）：avg=23.21ms, p50=22.86ms, p90=26.38ms

- 流水线（pipeline=1）：avg=21.86ms, p50=21.01ms, p90=24.91ms

收益：

- 平均时延 ≈ +5.8%

- P50 ≈ +8.1%

- P90 ≈ +5.6%

**小结**：两次运行展现一致的正向收益（5-16%），具体值受系统负载与后端状态影响。

注：收益与批大小、元素规模、后端实现以及系统环境有关，建议结合自身工作负载复测。

## 实现要点

- 在 `execute_vector_add()` 中拆分为两个 chunk；

- 每段：记录 compute → 安排拷贝 → submit → 开始 map_async；

- 通过 `device.poll(PollType::Poll)` 轮询，将第一段读回与第二段 compute 重叠；

- 资源管理仍复用现有 `BufferPool`/`staging_pool`，不改变外部契约。

## 风险与边界

- 小批量/小任务下，重叠收益可能有限；

- 不同后端（GL/Vulkan/DX12）和驱动行为差异会影响收益；

- 目前为“单批两段”重叠，未跨多批；跨批需要引入更通用的队列与异步接口（属于完整 Phase 3）。

## 下一步（建议）

- 扩大 sweep：任务规模（16K~1M）、批大小（8/16/24/32）、不同后端对比；

- 设计“跨批次”流水线队列（N 批 compute 与 N-1 批读回并行），纳入调度器；

- 指标化：统计提交耗时、读回时延分布，结合 `BucketCache::stats()` 观察命中率。
