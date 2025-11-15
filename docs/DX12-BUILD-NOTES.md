## GLES vs DX12 对比（2025-11-12）

初步结论：
 - 50k 以下：GLES 更快或持平（DX12 初始化/提交成本偏高）。
 - ≥100k：DX12 明显占优，250k 以上优势稳定。
 - 进一步优化方向与 DX12 单后端一致（合并提交、持久化资源、异步回读等）。

## 小规模延迟优化（2025-11-12 更新）

本次在 `WgpuExecutor` 中落地了两项低风险优化：

1) 单次 ComputePass 批处理：将每任务单独 begin/end 的 compute pass 合并为单个 pass，循环 set_bind_group + dispatch，减少 DX12/GLES 的 pass 开销。

2) StagingBuffer 复用池：新增独立 `staging_pool`（MAP_READ|COPY_DST），去除每任务的临时占位分配与频繁创建；在读回后将 staging buffer 归还池中复用。

改动位置：`src/gpu-executor/src/gpu_backend.rs`

- 结构：`WgpuExecutor` 增加 `staging_pool: BufferPool`

- 逻辑：`Pending.staging` 改为 `Option<Buffer>`；调度后从 `staging_pool` 获取并复制，读回后归还。

实测变化（Windows/DX12）：

- 20k：GPU 15ms → 14ms

- 50k：GPU 4ms → 3ms（与 GLES 3ms 打平）

- 100k：GPU 保持 1ms；250k：2ms；500k：3→2ms；1000k：4ms

最新对比输出（节选）：`data/gpu_threshold_scan/windows_compare_20251112.csv`

```

"size","cpu_ms_gles","gpu_ms_gles","cpu_ms_dx12","gpu_ms_dx12","faster_backend"
"20000","0","25","0","14","dx12"
"50000","0","3","0","3","tie"
"100000","0","9","0","1","dx12"
"250000","1","9","0","2","dx12"
"500000","1","15","0","2","dx12"
"1000000","3","30","2","4","dx12"

```

后续方向：

- 进一步持久化 bind group/pipeline（按尺寸 bucketing），减少对象创建。

- 合并回读与异步 map：在较大批次下压缩 host 等待。

- 视需要扩展到 Vulkan（Linux）以完成三后端统一基线。

---

## 单任务缓存与轮询读回（2025-11-12 补充）

- 新增 per-size 缓存（单任务批次）：为相同 size 的向量加任务持久化 a/b/out/staging 与 bind_group，避免重复创建/绑定；多任务批次仍走池化路径，避免共享冲突。

- 将一次性 `PollType::Wait` 改为轮询 `PollType::Poll` + try_recv，分段读回已就绪的映射，搭建简单主机侧流水线；保持接口同步返回不变。

- 代码位置：`src/gpu-executor/src/gpu_backend.rs`（`va_cache` 与轮询逻辑）。

- 备注：缓存对“相同规模多轮调用”的稳态收益更明显；单次测量可能受驱动/噪声影响波动，建议以多次采样的分位数评估。

# Windows DX12 构建与运行说明（当前阻塞）

本文记录在 Windows 上以 wgpu DX12 后端构建/运行 SuperVM GPU 示例时遇到的已知问题、根因分析和可选解决路径，便于团队复现与规避。

## 环境前提

- OS: Windows 10/11（PowerShell v5.1）

- Rust: stable（release 构建）

- 代码：SuperVM 仓库，分支 king/l0-mvcc-privacy-verification（2025-11-12）

## 现象与复现

- 目标：以 DX12 后端运行 `examples/gpu_threshold_scan_demo.rs`。

- 触发命令（示例其一）：
  ```powershell
  $env:SUPERVM_WGPU_BACKENDS="dx12"
  cargo run -p vm-runtime --example gpu_threshold_scan_demo --features "hybrid-exec,hybrid-gpu-wgpu,gpu-executor/wgpu-backend-dx12" --release
  ```

- 期望：编译通过并输出 CSV 风格时延对比；

- 实际：编译失败于 `wgpu-hal` DX12 路径，错误类型以 E0308/E0277 为主。

## 典型错误片段

- windows crate 版本分裂导致 FFI 类型不一致：
  ```text
  error[E0308]: mismatched types
    expected `ID3D12Device` (windows 0.53), found `Direct3D12::ID3D12Device` (windows 0.58)
  note: two different versions of crate `windows` are being used
  ...
  error[E0277]: the trait bound `&ID3D12Heap: Param<ID3D12Heap, InterfaceType>` is not satisfied
  note: multiple versions of `windows_core` in the dependency graph
  ```

## 根因分析

- `wgpu-hal 27.0.4` 与 `gpu-allocator 0.27.0` 间接/直接依赖了不同版本的 `windows` / `windows-core`：
  - `wgpu-hal` 使用 `windows` 0.58
  - `gpu-allocator` 使用 `windows` 0.53

- Direct3D12 接口类型（例如 `ID3D12Device`、`ID3D12Heap`、`D3D12_RESOURCE_DESC`）在不同版本的 `windows` crate 中为不同的 Rust 类型，即使 GUID 相同，也被视为不同类型，导致编译期类型不匹配与 trait bound 失败。

## 可选解决方案

1. Workspace 统一 `windows`/`windows-core` 版本（最小尝试，但存在 API 差异风险）
   - 在仓库根 `Cargo.toml` 添加：
     ```toml
     [patch.crates-io]
     windows = "=0.58.0"
     windows-core = "=0.58.0"
     ```
   - 风险：`gpu-allocator 0.27` 可能使用了仅适配 0.53 的 API；强行统一后仍可能报错。

2. Fork/升级 `gpu-allocator`（推荐技术路线，需投入）
   - 建立 fork，将 d3d12 模块的 `windows` 依赖升级至 0.58 并完成相应 API 迁移；
   - 在本仓库的 `Cargo.toml` 使用 `[patch.crates-io]` 指向 fork；
   - 重新编译并验证 DX12 示例运行。

3. 降级 `wgpu`（回退路线）
   - 查找与 `windows` 0.53 匹配的 `wgpu` 版本并适配现有代码；
   - 副作用：丢失 0.27 的 API 与修复，代码需做条件兼容或重写部分 API 调用。

4. 暂不启用 DX12（当前阶段建议）
   - 保持 Windows 上 GLES 后端验证；
   - 优先迁移 Linux + Vulkan，获取更具代表性的阈值与吞吐；
   - 等上游 `wgpu`/`gpu-allocator` 收敛 `windows` 版本后再尝试。

## 建议路线（当前阶段）

- 主线继续：Windows 走 GLES；平台对比优先补齐 Linux + Vulkan；

- 若必须拿 DX12 数据：优先推进方案 2（fork `gpu-allocator` 并升级到 `windows` 0.58），风险可控、长期更稳。

## 附：运行脚本（GLES 可用,DX12 当前失败）

- GLES：
  ```powershell
  powershell -NoProfile -ExecutionPolicy Bypass -File scripts/run-gpu-threshold-scan.ps1 -Backends gles -OutFile data/gpu_threshold_scan/windows_gles_YYYYMMDD.csv -Release
  ```

- DX12（复现实验，当前会失败）：
  ```powershell
  $env:SUPERVM_WGPU_BACKENDS="dx12"
  cargo run -p vm-runtime --example gpu_threshold_scan_demo --features "hybrid-exec,hybrid-gpu-wgpu,gpu-executor/wgpu-backend-dx12" --release
  ```

---

## 自动检测脚本（推荐第一步）

在尝试任何修复前，可以先运行自动检测脚本以确认是否存在 `windows` / `windows-core` 多版本冲突：

### ⚠️ 重要限制

**脚本默认不启用 feature flags**，无法检测条件依赖的版本冲突。对于 DX12 相关冲突，需手动验证：

```powershell

# 手动检测 DX12 feature 下的冲突

cargo tree -p gpu-executor --features wgpu-backend-dx12 -i windows

# 如果出现 "multiple `windows` packages" 错误，说明存在版本冲突

```

### 仅检测（无副作用）

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File scripts/detect-windows-crate-conflict.ps1

```

**示例输出**（存在冲突时）：

```

[detect] Scanning cargo tree for windows/windows-core versions...
[report] Detected windows crates versions:
  - windows: 0.53.0, 0.58.0
  - windows-core: 0.53.0, 0.58.0
[conflict] Multiple versions found for: windows, windows-core
[hint] To attempt auto-fix, run with: -Apply -WindowsVersion 0.58.0

```

### 尝试自动修复（统一到 0.58.0）

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File scripts/detect-windows-crate-conflict.ps1 -Apply -WindowsVersion 0.58.0

```

脚本行为：

- 在根 `Cargo.toml` 添加或更新 `[patch.crates-io]` 段落，强制 pin `windows = "=0.58.0"` 和 `windows-core = "=0.58.0"`。

- 生成备份文件 `Cargo.toml.bak.dx12-<timestamp>` 便于回滚。

- 提示执行 `cargo clean` 并重试 DX12 编译。

**后续步骤**：

```powershell
cargo clean
$env:SUPERVM_WGPU_BACKENDS="dx12"
cargo run -p vm-runtime --example gpu_threshold_scan_demo --features "hybrid-exec,hybrid-gpu-wgpu,gpu-executor/wgpu-backend-dx12" --release

```

**注意**：

- 若 `gpu-allocator` 0.27 在 `windows` 0.58 下出现 API 不兼容，仍会编译失败；这时需要继续按"方案 2"（fork `gpu-allocator`）推进。

- 统一版本只是最简尝试；若仍失败，建议回退到 GLES 或等待上游收敛。

---

## 当前状态（2025-11-12 更新）

- 我们已将 DX12 后端编译打通：
  - 将 `gpu-allocator` 升级到 0.28，并在 `gpu-executor` 中为其启用 `std` 特性（0.28 要求 `std` 或 `hashbrown` 至少启用其一）。
  - 扩展 GPU 后端源码的条件编译：原本仅在 `wgpu-backend` 下编译的模块，现同时适配 `wgpu-backend-dx12`，避免 DX12 特性打开时符号被裁剪导致的编译错误。
  - 验证：
    - 构建：`cargo build -p gpu-executor --features wgpu-backend-dx12` 成功；
    - 运行（最小 Demo）：
      ```powershell
      $env:SUPERVM_WGPU_BACKENDS="dx12"
      cargo run -p vm-runtime --example hybrid_gpu_vector_add_demo --features "hybrid-exec,hybrid-gpu-wgpu,hybrid-gpu-wgpu-dx12" --release
      ```
      观测输出（示例）：
      ```
      [gpu] WGPU backends mask: Backends(DX12)
      Executed on: Gpu; elements: 1000000; duration_ms: 18; first3: [0.0, 3.0, 6.0]
      ```

- 仍存在的事项（信息透明）：
  - 由于 crates.io 上的 `wgpu-hal 27.0.4` 仍间接依赖 `gpu-allocator 0.27`，当前构建会同时出现 `gpu-allocator 0.27` 与 `0.28` 两个版本并存（多版本无二义冲突，但会增大依赖图）。
  - 要彻底消除双版本，有两条路线：
    1) 等上游 `wgpu` 在 27.x 补丁/28 版本将 `gpu-allocator` 统一至 0.28（推荐，最稳妥）。
    2) 使用 `[patch.crates-io]` 指向一个 fork 的 `gpu-allocator`（名称仍为 `gpu-allocator`，适配默认特性或提供 `hashbrown`），并在 fork 中维持与 `wgpu-hal` 期望的 API 兼容；这需要我们维护一个兼容分支。

- 建议：
  - 现阶段 DX12 已可编译并运行 Demo，功能验证路径可通；
  - 如无强需求压缩依赖体积/去重，可暂缓处理双版本，待上游自然对齐；
  - 若需要在 CI 上全面开启 DX12 跑阈值扫描，可直接复用 GLES 流程，替换 `-Backends dx12` 并收集 CSV 对比。

## 阈值扫描数据（DX12）

最新扫描文件：`data/gpu_threshold_scan/windows_dx12_20251112.csv`

采集命令：

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File scripts/run-gpu-threshold-scan.ps1 -Backends dx12 -OutFile data/gpu_threshold_scan/windows_dx12_20251112.csv -Release

```

示例片段：

```

size,device,duration_ms,first,backend_mask_hint
20000,Cpu,0,0,dx12
20000,Gpu,13,0,dx12
50000,Cpu,0,0,dx12
50000,Gpu,4,0,dx12
100000,Cpu,0,0,dx12
100000,Gpu,1,0,dx12
250000,Cpu,0,0,dx12
250000,Gpu,2,0,dx12
500000,Cpu,0,0,dx12
500000,Gpu,2,0,dx12
1000000,Cpu,1,0,dx12
1000000,Gpu,4,0,dx12

```

初步观察：

- DX12 在当前实现下中小规模 vector-add 未明显优于 CPU（GPU 初始化/提交成本占比高）。

- ≥250k 后 GPU 延迟下降到 1–2ms 级，具备批量并行优势，仍需与 GLES/Vulkan 对比确认是否有进一步阈值偏移。

- 1000000 元素耗时 4ms（DX12），与 GLES 初次测试接近，可后续并列记录对比曲线。

后续改进方向：
1. 复用/持久化 command encoder 与 bind group，减少每任务创建成本。
2. 引入批量合并（多个任务拼接为一个 dispatch，减少 submit 次数）。
3. 加入异步 staging 回读与 pipeline overlap，提升长批性能。
4. 扩展更多算子（哈希、字段乘法等）验证 DX12 后端通用性。
5. 与 Vulkan (Linux) / GLES (Windows) 三后端作并行基线对比，更新综合曲线。

依赖去重计划（gpu-allocator 双版本）：

- 当前保持双版本运行（功能已验证）。

- 已尝试：在根 Cargo.toml 用 `[patch.crates-io]` 指向 `gpu-allocator` Git tag `0.28.0`，并保留默认特性（std）。
  - 结果：工作区自有 crate 统一到 0.28，但 crates.io 上的 `wgpu-hal 27.0.4` 仍保留对 `gpu-allocator 0.27.0` 的依赖，导致双版本并存。
  - 结论：仅依赖 patch 无法让 wgpu-hal 放弃 0.27（其发布版本锁定了 semver），需等待上游发布将其提升到 0.28 的版本。
  - 如果必须去重：可 fork `gpu-allocator` 或 `wgpu-hal`，通过 `[patch.crates-io]` 指向 fork 源，但此方案需要我们自行维护兼容性与更新。


  ## GLES vs DX12 对比（2025-11-12）

  我们已在同一台 Windows 机器上分别跑通 GLES 与 DX12 的阈值扫描，并生成对比 CSV：

  - GLES 阈值扫描：`data/gpu_threshold_scan/windows_gles_20251112.csv`
  - DX12 阈值扫描：`data/gpu_threshold_scan/windows_dx12_20251112.csv`
  - 自动对比输出：`data/gpu_threshold_scan/windows_compare_20251112.csv`

  生成对比文件的命令（示例）：
  ```powershell
  powershell -NoProfile -ExecutionPolicy Bypass -File scripts/compare-gpu-backends.ps1 `
    -GLES data/gpu_threshold_scan/windows_gles_20251112.csv `
    -DX12 data/gpu_threshold_scan/windows_dx12_20251112.csv `
    -OutFile data/gpu_threshold_scan/windows_compare_20251112.csv
  ```

  对比文件示例前几行：
  ```
  "size","cpu_ms_gles","gpu_ms_gles","cpu_ms_dx12","gpu_ms_dx12","faster_backend"
  "20000","0","25","0","16","dx12"
  "50000","0","3","0","4","gles"
  "100000","0","9","0","1","dx12"
  "250000","1","9","0","2","dx12"
  "500000","1","15","0","3","dx12"
  "1000000","3","30","1","5","dx12"
  ```

  初步结论：
  - 50k 以下：GLES 更快或持平（DX12 初始化/提交成本偏高）。
  - ≥100k：DX12 明显占优，250k 以上优势稳定。
  - 进一步优化方向与 DX12 单后端一致（合并提交、持久化资源、异步回读等）。


  ## PowerShell 执行策略与调用方式（Windows 5.1 提示）

  在 VS Code 的 PowerShell 终端内，直接使用 `& .\scripts\*.ps1` 可能触发执行策略限制（PSSecurityException）。建议统一使用以下方式绕过，不修改全局策略：

  ```powershell
  powershell -NoProfile -ExecutionPolicy Bypass -File scripts/<your-script>.ps1 <args>
  ```

  例如对比脚本：
  ```powershell
  powershell -NoProfile -ExecutionPolicy Bypass -File scripts/compare-gpu-backends.ps1 `
    -GLES data/gpu_threshold_scan/windows_gles_20251112.csv `
    -DX12 data/gpu_threshold_scan/windows_dx12_20251112.csv `
    -OutFile data/gpu_threshold_scan/windows_compare_20251112.csv
  ```

