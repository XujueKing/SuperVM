# CPU-GPU 双内核异构计算架构（SuperVM Phase 13）

状态: 规划中（Phase 13，原路线图中自 Phase 8 独立拆分）  
关联: ROADMAP.md · L1.1 ExecutionEngine · L0 Pandora Core · L3 gpu-executor/HybridScheduler · L2 ZK/证明聚合 (Phase 8)

## 目标与范围
- 目标: 在不污染 L0 内核的前提下，引入 GPU 执行能力，显著提升 ZK/密码学等重计算场景性能，并支持 CPU/GPU 混合调度与自动降级。
- 范围:
  - L1.1: 在 ExecutionEngine 能力模型中确认 `EngineType::Gpu` 与能力查询接口。
  - L3: 实现 `gpu-executor`（CUDA/OpenCL 后端）与 `HybridScheduler`（CPU/GPU 调度）。
  - L2: 支持将 ZK/哈希/签名/Merkle 等工作负载路由到 GPU。
- 非目标:
  - 不在 L0 引入任何 GPU 运行库依赖；L0 保持 CPU 主路径与纯净可移植性。

## 架构总览（协同图）
```
[L2 工作负载: ZK/哈希/签名/Merkle]
              │ 提交任务
              ▼
[L3 HybridScheduler] ──→ [GPU Executor] (CUDA/OpenCL)
        │
        └────────────→ [L0 CPU Executor] (WASM+MVCC)

  ↑ L1.1 ExecutionEngine 统一接口对接（EngineType::Wasm/Evm/Gpu）
```

## 关键组件
- L0 CPU Executor（既有）
  - WASM Runtime、MVCC、并行调度、FastPath 优化。
  - 不包含 GPU 代码与依赖。
- L1.1 ExecutionEngine 接口（既有/小扩展）
  - EngineType: Wasm/Evm/Gpu（已在 ROADMAP 列出）。
  - 能力探测: `supports(engine: EngineType) -> bool`、`hardware_caps() -> Caps`（建议）。
- L3 gpu-executor（新增）
  - 后端: CUDA（NVIDIA）/ OpenCL（AMD/Intel），feature-gate 控制。
  - 模块: `sha256_batch`、`ecdsa_batch`、`merkle_builder`、`zk_prover`。
  - 设备管理: 枚举/选择/初始化/上下文复用；内存优化: pinned memory、批量提交。
- L3 HybridScheduler（新增）
  - 策略: Auto / CpuOnly / GpuOnly / LoadBalance。
  - 自动降级: GPU 不可用或失败时回退 CPU；幂等与重试。
  - 监控: 任务队列、吞吐/延迟、GPU 利用率、失败率。
- L2 工作负载适配（增强）
  - ZK Prove/Verify、MSM/FFT、RingCT 电路可走 GPU 分支。

## 接口契约（草案）
```rust
pub enum EngineType { Wasm, Evm, Gpu }

pub struct HardwareCaps {
    pub has_gpu: bool,
    pub gpu_memory_gb: usize,
    pub backend: Option<GpuBackend>, // Cuda / OpenCL
}

pub trait ExecutionEngine {
    fn engine_type(&self) -> EngineType; // 现有: Wasm/Evm；新增: Gpu
    fn supports(&self, task: TaskType) -> bool;   // 哈希/签名/ZK/Merkle
    fn submit(&self, task: Task) -> Result<TaskHandle>;
}

pub enum TaskType { Sha256Batch, EcdsaBatch, MerkleBuild, ZkProve, ZkVerify }
```

- 错误与降级
  - submit 返回 GPU 异常 -> 调度器捕获 -> 切换 CPU 实现 -> 输出一致性校验（可选抽样）。
  - 超时/资源不足 -> 降低 batch 或改发 CPU。

## 调度策略（示例）
- Auto: 首选 GPU；GPU 忙/故障则回退 CPU。
- LoadBalance: 基于历史 P50/P90 延迟动态调整 CPU/GPU 比例。
- CpuOnly/GpuOnly: 诊断/实验模式。

## 度量与可观测性
- 指标: GPU 利用率、任务排队/执行时间、batch 大小、fallback 次数、成功率。
- 日志: 关键阶段（分发、执行、回传、降级）。
- 追踪: 单任务 trace-id，便于跨层排查。

## 交付与里程碑（映射 Phase 13）
- 13.1: `gpu-executor` crate 骨架 + 设备检测（CUDA/OpenCL）
- 13.2: 密码学加速: SHA256/签名/Merkle 批量
- 13.3: ZK 加速: 接入 bellman-cuda，MSM/FFT 优化
- 13.4: HybridScheduler 策略与自动降级
- 13.5: 评测与对齐: 性能对比/一致性校验
- 13.6: 多 GPU 支持与负载均衡
- 13.7: 研究性方向（合约执行、MVCC 读优化等）

## 风险与缓释
- CUDA 依赖兼容: 预留 OpenCL 后端；以 feature gate 控制可选依赖。
- CPU-GPU 传输瓶颈: pinned memory、批处理；必要时压缩中间数据。
- 结果一致性: 与 CPU 路径做随机抽样比对；单元测试 + 集成测试覆盖。

## 测试计划
- 单测: 各任务内核正确性（CPU vs GPU）。
- 压测: 不同混合占比下的 TPS/延迟；GPU 利用率曲线。
- 故障演练: GPU 异常、设备丢失、内存不足；验证自动降级。

## 目录与依赖建议
- `gpu-executor/`
  - `src/cuda/`、`src/opencl/`
- 依赖: `cudarc`, `opencl3`, `bellman-cuda`, 可选 feature: `gpu-cuda`/`gpu-opencl`/`gpu-all`。

## 验收标准（建议）
- GPU 设备检测成功率 > 99%
- GPU ZK 证明加速 > 50×；批量签名 > 20×；批量哈希 > 10×
- 高负载 GPU 利用率 > 70% ；CPU-only 模式可独立编译通过
- 失败自动降级到 CPU；单元测试覆盖率 > 80%

---
更多细节请参见 ROADMAP.md 的 Phase 13 专章与 L1/L2/L3 对应小节（Phase 8 保留用于 zkVM 与证明聚合，不再混用）。

---

## 📑 目录 (TOC)

1. 术语对照表
2. 数据与任务流细化
3. 调度策略与决策矩阵
4. 性能基线 & 目标指标
5. 安全与一致性 (降级/校验/侧信道)
6. 风险矩阵与缓释策略
7. Sprint 规划 (前两周 PoC)
8. 实施里程碑与验收重复确认

## 1. 术语对照表
| 名称 | 含义 | 备注 |
|------|------|------|
| HybridScheduler | 根据策略在 CPU/GPU 之间分派批量计算任务的调度器 | 支持 Auto/Balance/CpuOnly/GpuOnly |
| gpu-executor | 实际封装 CUDA/OpenCL 内核调用的执行模块 | 通过 feature gate 启用 |
| Batch | 一次提交到 GPU 的同类型任务集合 | 控制传输/内核启动开销摊薄 |
| Fallback | GPU 失败或超时时回退到 CPU 路径 | 保持功能正确性 |
| Key Image | RingCT 防双花标识（与 Phase 8 证明聚合区分） | 来自隐私层 L0.7 |
| MSM/FFT | ZK 证明生成的核心算子 | 重点 GPU 加速目标 |

## 2. 数据与任务流细化
```
Txn / ZK Request / Crypto Batch
    │ classify(TaskType)
    ▼
  HybridScheduler(strategy, metrics)
      ├─ supports(Gpu)? & GPU Healthy → enqueue GPU Batch
      │     └─ gpu-executor.launch(kernel, device_ctx)
      │           └─ result buffer (pinned) → completion callback
      └─ else → CPU path (sync or rayon parallel)
        └─ optional sample cross-check (N%)
```
关键点：
- 分类：由 TaskType 决定可 GPU 化能力；部分任务（小批量）直接 CPU。
- 回传：GPU 结果写入预分配缓冲，避免重复分配；完成后记录延迟与大小。
- 采样一致性：按比例抽样 GPU vs CPU 重算，发现偏差立即标记设备 unhealthy。

## 3. 调度策略与决策矩阵
| 条件 | GPU 选择 | CPU 回退 | 调整行为 |
|------|----------|----------|----------|
| GPU 利用率 < 70% & 队列深度 < 阈值 | 是 | 否 | 增大 batch (×1.2) |
| GPU 队列深度 > 高水位 | 否 | 是 | 减小 batch (×0.8) / Balance 模式切换 |
| 最近 N 次失败率 > p | 否 | 是 | 标记 unhealthy，进入降级窗口 |
| 单任务大小 < min_batch_size | 否 | 是 | 直接 CPU（避免启动开销） |
| 需要严格低延迟 (p99 SLA) | 视模式 | 是 | 强制小批+CPU |

策略微调：每次评估窗口 1s 或 100 批次，使用 EMA 平滑指标。

## 4. 性能基线 & 目标指标
| 指标 | 基线（CPU） | 目标（GPU） | 说明 |
|------|-------------|-------------|------|
| SHA256 1K inputs | X ops/s (待测) | ≥10× | 批量 kernel 合并 |
| ECDSA verify 1K | X ops/s | ≥20× | 同步曲线批处理 |
| Merkle 1M leaves | 构建耗时 T | ≤ T/5 | 并行哈希 + 内存共用 |
| MSM (ZK) | CPU 基线 t_ms | ≤ t_ms/20 | 依赖 bellman-cuda |
| FFT (ZK) | CPU 基线 t_ms | ≤ t_ms/15 | 蝶形并行 |
| Fallback 成功率 | 100% | 100% | 正确性优先 |
| 结果一致性抽样 | 100% 匹配 | 100% 匹配 | 失败率=0 触发告警 |

后续会将实际数值填充至表格（M13.5 测试完成后）。

## 5. 安全与一致性
- 内存：避免在 GPU 上驻留敏感私钥材料，敏感数据只传入哈希结果或公钥。
- 降级：GPU 任何内核错误/超时/设备丢失 → 标记 unhealthy → 一段观察窗口后再尝试恢复。
- 校验：N% 抽样（默认 1-5%）重新在 CPU 上验证结果；连续两次失败直接熔断 GPU。
- 侧信道：避免根据秘密数据决定批大小或调度策略；使用恒定分支内核。
- zeroize：批处理结束后清理 host pinned buffer。

## 6. 风险矩阵
| 风险 | 等级 | 缓释 |
|------|------|------|
| CUDA 版本兼容问题 | 中 | feature gate + OpenCL 后端兜底 |
| 设备争用（多进程） | 中 | 独占模式/显式上下文管理 |
| 批量太大导致延迟尾部升高 | 高 | 自适应批调整 + p99 监控 |
| 内存传输瓶颈 | 高 | pinned memory + 合并提交 |
| 不一致结果（GPU bug） | 高 | 抽样比对 + 快速熔断 |
| 驱动崩溃 | 中 | 自动降级 + 恢复冷却时间 |
| 指标缺失导致决策失效 | 中 | 启动自检 + 指标完整性探针 |

## 7. Sprint 规划（前两周 PoC）
| 周次 | 目标 | 任务 | 验收 |
|------|------|------|------|
| Week 1 | 设备检测与执行骨架 | 创建设备枚举 + submit 原型 + CPU fallback | GPU 有/无环境均可运行 |
| Week 2 | 哈希批处理 PoC | SHA256 kernel + batch builder + 指标导出 | ≥5× CPU 单线程，结果对齐 |

Week 3 开始扩展签名与 Merkle；Week 4 接入 ZK MSM/FFT。

## 8. 验收重复确认
完成后与 ROADMAP M13.x 对照：所有表格打勾 + 指标面板在线 + 抽样一致性日志零失败。

