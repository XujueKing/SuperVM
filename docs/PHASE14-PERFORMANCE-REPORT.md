# Phase 14 Performance Report

更新日期: 2025-11-13
适用范围: M14.4–M14.6（性能基线、Persistent Backend、批处理流水线）

## 硬件与环境

- 机器: Intel Core i7-9750H @ 2.60GHz（6C/12T）

- OS: Windows

- GPU 后端: 原生 Vulkan SPIR-V（`spirv-vulkan`）

- 构建: `--release`（默认不启用 `shader-compile`，使用预编译 .spv）

- 重要环境变量:
  - `GPU_STREAM_CHUNK_ELEMS`（默认 524,288；可调）
  - `GPU_DEVICE_LOCAL`（1 启用设备本地路径）

## 方法学

- Backend: `PersistentVulkanBackend`（复用 instance/device/pipeline/descriptor/command pool）

- Kernel: BLS12-381 field add（6×u64 limbs）

- 数据路径: Host-Visible 与 Device-Local 双路径对比；按规模选择更优路径

- 批处理模式:
  - 顺序批处理：`run_field_add_batch(...)`
  - 流水线批处理：`run_field_add_batch_pipelined(...)`（Ping-Pong 双缓冲 + 跨队列同步）

- 可复现示例：
  - `examples/bench_persistent.rs`
  - `examples/bench_batch_pipelined.rs`
  - `examples/smoke_batch.rs`（顺序 vs 流水线一致性冒烟）

## 结果摘要

### Persistent Backend（M14.5）

- 消除初始化开销（~110ms），小规模显著收益

- 代表性结果（64K elements）：one-shot 130.9ms → persistent 17.22ms（7.6x）

- 小规模（64 elements）：104.87ms → 3.90ms（26.9x）

### 批处理流水化（M14.6）

- 输入合并（A||B）与双缓冲流水线

- 代表性结果（16K elements）：
  - 2 jobs: 1.13x（27ms → 23ms）
  - 3 jobs: 1.64x（41ms → 25ms）
  - 4 jobs: 1.79x（53ms → 29ms）

- 代表性结果（64K elements）：
  - 3 jobs: 1.40x（82ms → 59ms）
  - 4 jobs: 1.48x（104ms → 70ms）

- 代表性结果（256K elements）：
  - 4 jobs: 1.20x（262ms → 217ms）

### Host-Visible vs Device-Local（选路建议）

- ≤64K：Host-Visible 更快（避免额外 Copy+Barrier）

- ≥1M：Device-Local 更有潜力（带宽更高，计算占比更高）

## 复现步骤（可选）

PowerShell

```

# 1) 冒烟：验证顺序与流水线一致性

cargo run -p gpu-executor --example smoke_batch --features spirv-vulkan --release

# 2) 持久化基线

cargo run -p gpu-executor --example bench_persistent --features spirv-vulkan --release

# 3) 批处理流水线（可调分块与路径）

$env:GPU_STREAM_CHUNK_ELEMS='524288'
$env:GPU_DEVICE_LOCAL='0'
cargo run -p gpu-executor --example bench_batch_pipelined --features spirv-vulkan --release

```

注意

- 如需构建期自动编译 shader（非默认）：`cargo build -p gpu-executor --features shader-compile`

- Windows/CI 未安装 CMake 时建议保持默认（预编译 .spv），功能完整可用

## 参考资料

- 详细日志与图表：
  - `docs/M14.6-PROGRESS.md`
  - `docs/M14.6-BATCH-PIPELINING-SUMMARY.md`
  - `BENCHMARK_RESULTS.md`（新增 Phase 14 GPU 小节）

- API 文档：`docs/API-GPU-EXECUTOR.md`

## 结论

- M14.5 显著改善小规模开销并成为后续优化的基础

- M14.6 在多作业批次下提供 1.4x–1.8x 的稳定收益，且规模越大收益趋缓

- 建议：根据规模启发式选择数据路径，增量调优 `GPU_STREAM_CHUNK_ELEMS`，并在 ≥3 作业时优先使用流水线批处理
