# Session 16 快速开始指南

## 本地验证 Trace Backend（Windows）

在本机 Windows 环境下，Trace backend 可以直接运行，无需任何额外依赖。

### 运行 Trace 性能基线

```powershell
cd "D:\WEB3_AI开发\虚拟机开发\src\l2-executor"
cargo run --release --example backend_comparison
```

**预期输出**：
- Fibonacci 计算结果验证
- Trace backend prove/verify 时间（Dev Mode，约 0ms）
- 证明大小（minimal overhead）

---

## 通过 CI 获取 RISC0 性能数据

由于本地 WSL 环境受网络限制，RISC0 性能测试通过 GitHub Actions 完成。

### 步骤 1: 触发 Workflow

1. 访问仓库 GitHub Actions 页面：
   ```
   https://github.com/XujueKing/SuperVM/actions
   ```

2. 在左侧列表中选择 `RISC0 Performance (Dev Mode)`

3. 点击右上角 `Run workflow` 按钮

4. 选择分支：`king/l0-mvcc-privacy-verification`

5. 点击 `Run workflow` 确认

### 步骤 2: 等待执行完成

- 预计耗时：10-15 分钟
- 主要步骤：
  - 安装 Rust toolchain
  - 安装 RISC0 toolchain (`rzup install`)
  - 编译依赖（首次较慢）
  - 运行 `risc0_performance_comparison` 示例

### 步骤 3: 下载性能日志

1. Workflow 完成后，进入该次运行详情页

2. 滚动到页面底部 `Artifacts` 区域

3. 下载 `risc0-perf-logs` 压缩包

4. 解压得到 `risc0_ci_run.log`

### 步骤 4: 解析关键数据

在 `risc0_ci_run.log` 中查找以下关键信息：

```
=== RISC0 Performance Comparison ===
Fibonacci(10) prove time: XXX ms
Fibonacci(10) verify time: XXX µs
Proof size: XXX bytes

Fibonacci(50) prove time: XXX ms
...
```

### 步骤 5: 回填报告

将上述数据填入 `docs/SESSION-16-COMPLETION-REPORT.md` 第 4 节性能表格。

---

## 本地构建说明

### 不启用 RISC0（默认）

```powershell
cd src/l2-executor
cargo build --release
```

✅ 不需要 RISC0 toolchain，可以正常编译和运行 Trace backend

### 启用 RISC0（需要 Linux + toolchain）

```bash
# 仅在 Linux/WSL 且已安装 rzup 的情况下
export RISC0_DEV_MODE=1
source "$HOME/.risc0/bin/env"
cargo build --release --features risc0-poc
```

⚠️ 如果没有 RISC0 toolchain，`build.rs` 会自动跳过，输出警告但不阻塞编译

### 强制跳过 RISC0 构建

如果你想在启用 `risc0-poc` feature 时跳过实际的 guest 编译：

```bash
export RISC0_SKIP_BUILD=1
cargo build --release --features risc0-poc
```

---

## 常见问题

**Q: 本地 Windows 可以运行 RISC0 吗？**

A: 不行。RISC0 toolchain 仅支持 Linux/macOS。Windows 用户：
- 本地：使用 Trace backend 进行开发和测试
- 性能验证：通过 CI 获取 RISC0 数据

**Q: 为什么不在本机 WSL 跑 RISC0？**

A: 尝试过，但 `rzup install` 在下载 GitHub Releases 时多次遭遇 TLS 中断。判断为本地网络环境限制，CI 环境更稳定。

**Q: CI 跑一次要多久？**

A: 首次约 10-15 分钟（下载 + 编译依赖）。后续如果有缓存可能更快。

**Q: 如何确认本地构建不会被 RISC0 阻塞？**

A: 测试命令：
```powershell
cd src/l2-executor
cargo clean
cargo build --release
```
如果成功完成且没有报错 "risc0 toolchain could not be found"，说明改进生效。

---

## 相关文档

- [SESSION-16-COMPLETION-REPORT.md](./SESSION-16-COMPLETION-REPORT.md) - 完整报告和性能表格
- [SESSION-16-TROUBLESHOOTING.md](./SESSION-16-TROUBLESHOOTING.md) - 环境问题和解决方案
- [.github/workflows/risc0-perf.yml](../.github/workflows/risc0-perf.yml) - CI workflow 定义
