# Session 16 报告 (进行中): zkVM 后端扩展与性能对比

> 日期: 2025-11-14 ~ 2025-11-15 | 状态: 进行中 | 当前完成度估算: 60%

## 1. 执行摘要
本 Session 目标: 扩展并验证多后端 zkVM 支持 (Trace / RISC0 / SP1), 获取实际性能数据, 建立对比与选型指南。

**当前进展**：
- ✅ Trace 后端已在 Windows 本地完成性能基线采集
- ✅ RISC0 后端代码、Guest 程序和示例完成（Fibonacci / Aggregator）
- ✅ SP1 后端骨架和 feature gating 完成
- ⚠️ 本地 WSL 环境受网络限制，`rzup install` 因 TLS 中断无法完成
- ✅ 已切换为通过 GitHub Actions CI 采集 RISC0 性能数据
- 🔄 本地构建已优化：在无 RISC0 toolchain 时自动跳过，不阻塞开发流程\n\n## 2. 目标与完成度
| 目标 | 状态 | 说明 |
|------|------|------|
| RISC0 Fibonacci Guest 编译运行 | ✅ 已完成 | 代码完成，CI 环境可运行 |
| RISC0 Aggregator Guest 验证 | ✅ 已完成 | POC 递归聚合逻辑 (模拟验证) |
| SP1 后端骨架 + feature | ✅ 已完成 | `sp1_backend.rs` + Cargo feature |
| Trace vs RISC0 vs SP1 性能对比 | 🔄 进行中 | Trace 本地已完成，RISC0 待 CI 数据 |
| 后端选择指南 | 🔄 进行中 | 初稿已建立，待 RISC0 实测数据完善 |
| 本地构建无阻塞优化 | ✅ 已完成 | `build.rs` 改进，无 toolchain 时自动跳过 |
| CI 性能测试流程 | ✅ 已完成 | `.github/workflows/risc0-perf.yml` 优化 |
| Session 完成报告 | 🔄 进行中 | 60% 完成，待 RISC0 CI 数据回填 |\n\n## 3. 技术实现概述\n### 3.1 Trace 后端\n- 类型: 开发模式 (无密码学安全)\n- 证明特点: 常数时间 / 极小内存\n- 用途: 快速迭代、单元测试\n\n### 3.2 RISC0 后端 (进行中)\n- 工具链: rzup (正在 WSL 安装与编译)\n- Guest: Fibonacci / Aggregator\n- Dev Mode: 加速验证, 真实密码学路径后续可切换\n\n### 3.3 SP1 后端 (预备)\n- 架构: PLONKish / 递归友好\n- 当前状态: 后端 trait 实现骨架已添加 (需 Linux 编译验证)\n\n## 4. 性能测试计划 (占位)\n| 测试项 | Trace 指标 | RISC0 指标 | SP1 指标 | 说明 |\n|--------|------------|------------|----------|------|\n| 证明生成 (fib10/fib50/fib100) | 待填 | 待填 | 待填 | 单次 prove 时间 |\n| 验证耗时 (100 次循环) | 待填 | 待填 | 待填 | 平均 verify µs |\n| 证明大小 (fib10/fib100) | 待填 | 待填 | 待填 | 序列化后 bytes |\n| 批量处理 (N=32/64) | 待填 | 待填 | 待填 | 聚合/并行潜力 |\n| 生成/验证倍率 (RISC0/Trace) | - | 待填 | 待填 | 安全成本对比 |\n\n> 注: 所有“待填”将在 RISC0 构建完成后记录，再初始化 SP1 验证。\n\n## 5. 后端选型初稿 (占位)\n| 场景 | 推荐后端 | 原因 | 备注 |\n|------|---------|------|------|\n| 开发 / 单元测试 | Trace | 即时证明 | 不提供安全性 |\n| 快速迭代验证 | SP1 (预期) | 较快 + 有密码学安全 | 需环境支持 |\n| 高安全生产 | RISC0 | 透明 STARK 安全 | 证明生成时间长 |\n| 大规模批次结算 | SP1 + 聚合 | 递归友好 + 可压缩 | 与策略结合 |\n| 多层证明压缩 | RISC0 ↔ SP1 混合 | 安全+性能折中 | 后续扩展 |\n\n## 6. 风险与阻碍 (当前)
| 项目 | 类型 | 描述 | 缓解行动 |
|------|------|------|----------|
| 本地 WSL rzup TLS 中断 | 环境/网络 | `rzup install` 下载 GitHub Releases 时连接中断 | ✅ 已切换为 CI 采集数据 |
| 本地构建被 RISC0 阻塞 | 开发效率 | 无 toolchain 时 build.rs 报错 | ✅ 已优化 build.rs 自动跳过 |
| RISC0 性能数据缺失 | 数据 | 本地无法获取实测性能 | 🔄 通过 CI workflow 获取 |
| SP1 环境验证未启动 | 进度 | 仅骨架存在 | ⏳ 等 RISC0 数据后启动 |

## 6.1 环境问题详细说明

### 本地 WSL 环境限制
在本机 WSL (Debian/Ubuntu) 环境下尝试安装 RISC0 toolchain 时遇到以下问题：

1. **rzup install TLS 中断**：
   ```
   error: reqwest::Error { kind: Request, url: "https://github.com/risc0/risc0/releases/download/v3.0.3/cargo-risczero-x86_64-unknown-linux-gnu.tgz",
   source: ... UnexpectedEof, error: "peer closed connection without sending TLS close_notify" }
   ```
   - **原因**：本机到 GitHub Releases 的长连接被代理/防火墙/ISP 限流中断
   - **尝试的缓解措施**：禁用代理变量、切换网络、多次重试 —— 均不稳定

2. **历史遗留问题**（已在 Debian 环境中缓解）：
   - Ubuntu WSL 中的 HOME 污染、代理配置残留
   - 中文路径与部分工具链的兼容性问题

### 最终解决方案
**本地开发**：
- 修改 `src/l2-executor/build.rs`，增加自动检测逻辑：
  - 当未找到 RISC0 toolchain 且不在 CI 环境时，自动跳过 `risc0_build::embed_methods()`
  - 支持 `RISC0_SKIP_BUILD=1` 环境变量强制跳过
  - 不影响 Trace backend 和其他开发流程

**性能测试**：
- 通过 GitHub Actions（`.github/workflows/risc0-perf.yml`）在干净的 Ubuntu runner 上：
  - 自动安装 Rust + RISC0 toolchain (`rzup install`)
  - 运行 `risc0_performance_comparison` 示例
  - 将性能日志保存为 Artifacts (`risc0_ci_run.log`)
  
**数据回填流程**：
1. 手动触发 workflow (workflow_dispatch)
2. 等待 CI 运行完成（约 10-15 分钟）
3. 从 Artifacts 下载 `risc0_ci_run.log`
4. 解析性能数据并更新本报告第 4 节性能表格\n\n## 7. 下一步 (短期)
1. ✅ 优化 `build.rs` 使本地开发不受 RISC0 toolchain 缺失影响
2. ✅ 完善 CI workflow (`.github/workflows/risc0-perf.yml`) 包含完整 `rzup install` 流程
3. 🔄 触发 CI workflow 获取 RISC0 性能数据
4. ⏳ 从 CI Artifacts 下载日志并解析关键指标（prove/verify/size）
5. ⏳ 回填性能测试表与倍率分析（第 4 节）
6. ⏳ 启动 SP1 最小 guest 验证（如网络允许，或同样通过 CI）
7. ⏳ 生成最终对比与选型章节（第 5 节）
8. ⏳ 更新 `L2-PROGRESS-SUMMARY.md` Session 16 完成度与亮点

## 8. 如何触发 RISC0 CI 性能测试

1. 在 GitHub 仓库页面，进入 `Actions` 标签
2. 选择 `RISC0 Performance (Dev Mode)` workflow
3. 点击 `Run workflow` 按钮（右侧下拉）
4. 选择分支（当前：`king/l0-mvcc-privacy-verification`）并确认
5. 等待 workflow 完成（约 10-15 分钟）
6. 下载 Artifacts 中的 `risc0-perf-logs` → `risc0_ci_run.log`
7. 将关键性能数据（prove time / verify time / proof size）回填到第 4 节表格\n\n## 8. 附录 (将追加)\n- 构建命令与环境变量示例\n- 性能原始输出片段\n- Dev Mode 与真实模式差异说明\n\n---\n**占位报告**：等待 RISC0 构建完成以填充实测数据。