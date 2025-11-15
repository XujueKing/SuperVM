# GPU Executor API (PersistentVulkanBackend)

最后更新: 2025-11-13

本页文档化 `gpu-executor` 的原生 Vulkan SPIR-V 执行接口，涵盖持久化后端与批处理 API。

## 概览

- 模块: `gpu_executor::spirv`

- 主要类型: `PersistentVulkanBackend`

- 主要能力:
  - 复用 Vulkan 实例/设备/管线/描述符，消除每次调用 100ms+ 初始化开销
  - 支持 Host-Visible 与 Device-Local 两条数据路径
  - 批处理执行（顺序版与流水线版），提升吞吐与资源利用率

## 关键类型

### PersistentVulkanBackend

```rust
pub struct PersistentVulkanBackend { /* 内部资源句柄与缓存 */ }

impl PersistentVulkanBackend {
    pub fn new() -> anyhow::Result<Self>;

    /// 顺序批处理：按序执行多个同构作业，重用已编译的 pipeline
    pub fn run_field_add_batch(
        &mut self,
        spirv_bytes: &[u8],
        jobs: &[(&[u64], &[u64])],
        elements: usize,
        use_device_local: bool,
    ) -> anyhow::Result<Vec<Vec<u64>>>;

    /// 流水线批处理：Ping-Pong 双缓冲，重叠 N 与 N+1 的传输与计算
    pub fn run_field_add_batch_pipelined(
        &mut self,
        spirv_bytes: &[u8],
        jobs: &[(&[u64], &[u64])],
        elements: usize,
    ) -> anyhow::Result<Vec<Vec<u64>>>;
}

```

输入/输出约定：

- 输入 `jobs`: 每个作业包含两段等长的 6×u64 limbs 数组切片 `(A, B)`；`elements` 表示元素个数（每元素=6×u64）；

- 输出：返回每个作业的结果 C（同样以 6×u64 limbs 扁平数组表示，长度 = elements×6）；

- 错误：参数不一致、缓冲区长度不匹配、Vulkan 资源/同步失败等将返回 `anyhow::Error`；

## 使用示例

```rust
use gpu_executor::spirv::PersistentVulkanBackend;
use std::fs;

fn main() -> anyhow::Result<()> {
    // 建议使用预编译 .spv；如需构建期自动编译，启用 feature `shader-compile`
    let spirv = fs::read("src/gpu-executor/shaders/compiled/bls12_381_field_add.spv")?;

    // 构造批次（2 个作业作为示例）
    let a1: Vec<u64> = vec![0; 6*16384];
    let b1: Vec<u64> = vec![0; 6*16384];
    let a2: Vec<u64> = vec![1; 6*16384];
    let b2: Vec<u64> = vec![2; 6*16384];

    let jobs: Vec<(&[u64], &[u64])> = vec![(&a1, &b1), (&a2, &b2)];

    let mut backend = PersistentVulkanBackend::new()?;

    // 顺序批处理（可选开启 device-local 路径）
    let _c_seq = backend.run_field_add_batch(&spirv, &jobs, 16384, true)?;

    // 流水线批处理（自动选择最佳路径）
    let _c_pipe = backend.run_field_add_batch_pipelined(&spirv, &jobs, 16384)?;
    Ok(())
}

```

## 环境变量

- `GPU_STREAM_CHUNK_ELEMS`：异步流式路径分块大小（元素为单位，默认 524_288）。

- `GPU_DEVICE_LOCAL`：优先选择设备本地路径（1 启用）。

- `SHADER_OPT`：构建期 shader 优化等级（`zero|size|performance`）。

- `SHADER_STRICT`：构建期严格模式（1 = warnings as errors）。

说明：`SHADER_*` 仅在启用 `--features shader-compile` 时生效。

## 性能提示

- 小规模（≤64K elements）：Host-Visible 通常更快（避免额外拷贝/屏障）。

- 中大规模（≥1M elements）：Device-Local 可能更优（更高带宽）。

- 多作业批次（≥3）：`run_field_add_batch_pipelined` 更易取得可观收益（1.4x~1.8x）。

## 附录：测试与基准

- 示例：
    - `src/gpu-executor/examples/bench_persistent.rs`
    - `src/gpu-executor/examples/bench_batch_pipelined.rs`
    - `src/gpu-executor/examples/smoke_batch.rs`（最小冒烟：顺序 vs 流水线结果一致性）

- 结果：见 `docs/M14.6-PROGRESS.md` 与 `docs/M14.6-BATCH-PIPELINING-SUMMARY.md`，以及 `BENCHMARK_RESULTS.md` 的 Phase 14 小节。
