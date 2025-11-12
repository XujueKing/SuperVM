# Phase 13: GPU-Executor Skeleton — Quick Start

**Phase 13: CPU-GPU 混合执行器** 的最小可编译骨架，用于在无 GPU 环境下定义接口，并提供 CPU 回退。

---

## 目录结构

```
src/gpu-executor/
├── Cargo.toml            # 独立 crate，特性：cpu (默认)、gpu (未来)
├── src/
│   └── lib.rs            # 核心 Trait、类型与占位 GPU 实现
└── tests/
    └── basic.rs          # 集成测试：CPU fallback 验证
```

---

## 核心接口

### 1. `GpuExecutor<T, R>` Trait
```rust
pub trait GpuExecutor<T, R> {
    fn execute(&mut self, batch: &Batch<T>) -> Result<(Vec<TaskResult<R>>, ExecStats), ExecError>;
    fn is_available(&self) -> bool { true }
    fn device_kind(&self) -> DeviceKind;
}
```

- **目的**: 统一 CPU 与 GPU 后端的批量执行接口。
- **参数化**: 泛型 `T` (任务输入) 与 `R` (任务输出)。

### 2. `HybridScheduler<Cpu, Gpu, T, R>`
```rust
pub struct HybridScheduler<Cpu, Gpu, T, R> {
    cpu: Cpu,
    gpu: Option<Gpu>,
    strategy: HybridStrategy,
    // ...
}
```

- **策略**: 根据批大小、GPU 可用性自动回退。
- **配置**: `HybridStrategy { gpu_threshold, max_cpu_parallelism }`。

### 3. CPU Fallback: `CpuMapExecutor<F>`
```rust
#[cfg(feature = "cpu")]
pub struct CpuMapExecutor<F> { map: F }
impl<T, R, F: Fn(&T) -> R> GpuExecutor<T, R> for CpuMapExecutor<F> { ... }
```

- **用途**: 开发与测试阶段不需要真实 GPU 时使用。
- **实现**: 简单 map 语义，同步执行每个任务。

### 4. GPU 占位符: `UnavailableGpu`
```rust
pub struct UnavailableGpu;
impl<T, R> GpuExecutor<T, R> for UnavailableGpu {
    fn execute(...) -> Result<...> { Err(ExecError::BackendUnavailable) }
    fn is_available(&self) -> bool { false }
}
```

- **目的**: 当 GPU feature 未启用时，避免编译失败。
- **行为**: 永远返回不可用错误。

---

## 快速验证

### 编译与测试
```powershell
# 构建新 crate
cargo build -p gpu-executor

# 运行单元测试与集成测试（无需 GPU）
cargo test -p gpu-executor
```

**预期输出**:
```
running 2 tests
..
test result: ok. 2 passed; 0 failed; 0 ignored
```

---

## 使用示例（CPU 模式）

```rust
use gpu_executor::*;

fn main() {
    // CPU 执行器：简单 +1 操作
    let cpu = CpuMapExecutor::new(|x: &u32| x + 1);
    
    // 构建调度器（GPU 未启用）
    let mut scheduler = HybridScheduler::new(
        cpu, 
        Some(UnavailableGpu), 
        HybridStrategy::default()
    );
    
    // 批量任务
    let batch = Batch { tasks: vec![
        Task { id: 1, payload: 41, est_cost: 1 },
        Task { id: 2, payload: 99, est_cost: 1 },
    ]};
    
    // 执行（自动回退到 CPU）
    let (results, stats) = scheduler.schedule(&batch).expect("ok");
    
    assert_eq!(results[0].output, 42);
    assert_eq!(stats.device, DeviceKind::Cpu);
}
```

---

## 下一步扩展

### A. GPU 真实后端集成（M13.2–M13.4）
1. **Feature Gate**: 启用 `gpu` 特性后引入 `wgpu` 或 `cuda-rs` 可选依赖。
2. **KernelExecutor**: 实现真实 GPU kernel 调度（WGSL/SPIR-V/PTX）。
3. **显存管理**: Buffer 分配、分页、Pool 复用。

### B. 任务调度策略（M13.5）
- **启发式**: 根据任务类型（算术/密码/混合）选择设备。
- **负载均衡**: CPU 与 GPU 并发工作，按吞吐比例动态分配。
- **自适应阈值**: 根据历史延迟自动调整 `gpu_threshold`。

### C. 与 L0/L1 集成（M13.6–M13.7）
- **MVCC 前端**: 将 `Transaction` 转换为 `Task<T>`，输出 `Result<R>` 映射回 Commit/Abort。
- **Metrics**: 复用 Prometheus 指标，增加 `gpu_utilization`, `cpu_fallback_count`。
- **Feature Toggle**: 在 `vm-runtime/Cargo.toml` 中可选依赖 `gpu-executor`。

---

## 设计原则

1. **零依赖默认**: 默认 `cpu` 特性不引入任何外部 GPU 库，确保普通开发环境可编译通过。
2. **Trait 隔离**: `GpuExecutor` 定义为纯接口，允许后续替换不同后端（Vulkan/Metal/OpenCL）。
3. **渐进式**: 先搭好类型系统与接口，后面逐步填充真实 GPU kernel 与内存管理。
4. **可测试**: CPU fallback 保证所有逻辑在无 GPU 时也能验证，降低 CI 配置门槛。

---

## 相关文档

- **架构设计**: `docs/ARCH-CPU-GPU-HYBRID.md`  
- **Roadmap**: `ROADMAP.md` (Phase 13, M13.1–M13.9)  
- **Code**: `src/gpu-executor/src/lib.rs`  

---

## 联系与反馈

如有疑问或希望参与 GPU kernel 实现，请参考 `CONTRIBUTING.md` 或提 issue。

**Phase 13 启动日期**: 2025-11-12  
**当前状态**: ✅ M13.1 完成（骨架搭建）
