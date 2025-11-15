# Session 16 构建排障与绕过指南

日期: 2025-11-14 | 适用范围: RISC0 Dev Mode 构建/运行（WSL）

## 症状
- `risc0-circuit-recursion v1.2.6` 构建阶段下载 ZKR 工件后校验失败：
  - 错误片段: `verification: FAILED … status 200`（S3 可达但校验未通过）

## 可能原因
- 历史代理缓存/链路中间件改写（导致内容被替换）
- 断点续传损坏（zip 不完整）
- 中文路径兼容性边角（个别工具链对非 ASCII 路径处理不一致）

## 快速绕过（推荐）
1) 将仓库复制到 WSL 家目录的 ASCII 路径后构建
```bash
wsl
set -e
rm -rf "$HOME/supervm_ascii" && mkdir -p "$HOME/supervm_ascii"
cp -r "/mnt/d/WEB3_AI开发/虚拟机开发" "$HOME/supervm_ascii/repo"
export PATH="$HOME/.risc0/bin:$PATH"; export RISC0_DEV_MODE=1
cd "$HOME/supervm_ascii/repo/src/l2-executor"
cargo run --release --features risc0-poc --example risc0_performance_comparison
```

2) 确认 WSL 侧无代理残留
```bash
echo http_proxy=${http_proxy:-unset}
echo https_proxy=${https_proxy:-unset}
```

3) 如仍失败，清理缓存并重试
```bash
rm -rf "$HOME/.cargo/registry" "$HOME/.cargo/git" "$HOME/.cache"
cargo clean
cargo build -vv --features risc0-poc
```

## 备用路径（网络/镜像持续异常时）
- 将 `src/l2-executor/Cargo.toml` 的 `risc0-zkvm` 暂定锁为 `=1.1.x` 再尝试构建。
- 或待网络稳定后重试 `1.2.6`（当前自动解析版本）。

## Trace 基线（已采集/Windows）
- 样例: Fibonacci(10)，期望输出 55
- 结果: 证明/验证均为 ~0ms（开发模式，非密码学安全）
- 命令:
  - `cd d:\WEB3_AI开发\虚拟机开发\src\l2-executor`
  - `cargo run --release --example backend_comparison`

## 附录：一键脚本（WSL）
- 可使用 `scripts/risc0-build-workaround.sh` 自动复制到 ASCII 路径并构建。

---

## WSL 常见环境错误修复

### A. getpwnam/getpwuid failed 5（无法解析默认用户）
症状：执行 `wsl bash -lc ...` 输出 `getpwnam(...) failed 5` 或 `getpwuid(0) failed 5`。

1) 列出发行版确认名称
```powershell
wsl -l -v
```

2) 以 root 进入发行版，检查用户与家目录（将 <Distro> 替换为上一步名称）
```powershell
wsl -d <Distro> -u root bash -lc "id -u; getent passwd | head -n 5; ls -la /home"
```

3) 如缺少常用用户，创建并设为默认
```bash
# 在 root shell 中执行
adduser <your_user>
usermod -aG sudo <your_user>
mkdir -p /home/<your_user>
chown -R <your_user>:<your_user> /home/<your_user>
printf "[user]\ndefault=%s\n" <your_user> > /etc/wsl.conf
```

4) 退出并重启 WSL，然后设定默认用户（某些发行版也支持专用命令）
```powershell
wsl --shutdown
ubuntu config --default-user <your_user>  # 若为 Ubuntu 系列
# 或下次直接： wsl -d <Distro> -u <your_user> bash -lc "whoami"
```

### B. locale 文件缺失（fopen(/etc/default/locale) failed 5）
在 root shell 中修复本地化配置：
```bash
apt-get update
apt-get install -y locales
locale-gen en_US.UTF-8
update-locale LANG=en_US.UTF-8
```

---

## 通过 CI 采集 RISC0 指标（本地 WSL 异常时的替代方案）
当本地 WSL 障碍无法短时修复，可使用 GitHub Actions 在 Linux 云环境构建与运行：

1) 使用仓库内工作流 `.github/workflows/risc0-perf.yml`（manual dispatch）
2) 触发后自动：
  - 安装 Rust 与 RISC0 工具链（Dev Mode）
  - 构建并运行 `risc0_performance_comparison`
  - 上传运行日志与性能输出为构建产物（Artifacts）
3) 下载 Artifacts 并回填 `SESSION-16-COMPLETION-REPORT.md`

---

## 使用 Debian 临时分发无损构建（推荐验证 WSL 引擎）
当现有 Ubuntu 分发无法启动且希望“无损”验证 WSL 引擎是否正常时，可安装一个干净的 Debian 分发进行对照，不影响现有数据：

1) 安装并初始化 Debian（管理员 PowerShell）
```powershell
wsl --install -d Debian
```
首次启动 Debian 时按提示创建用户（建议加入 sudo 组）。

2) 在 Debian 内运行一键脚本，产生日志到工作区根目录
```powershell
wsl -d Debian bash -lc 'chmod +x "/mnt/d/WEB3_AI开发/虚拟机开发/scripts/wsl-debian-risc0-run.sh"; \
"/mnt/d/WEB3_AI开发/虚拟机开发/scripts/wsl-debian-risc0-run.sh"'
```

3) 查看日志并回填报告
- 日志路径：`d:\WEB3_AI开发\虚拟机开发\risc0_debian_run.log`
- 若成功，将包含 prove/verify/size 的输出片段；将其用于 `SESSION-16-COMPLETION-REPORT.md`

---

## D. rzup install TLS 中断（UnexpectedEof）

**症状**：在本机 WSL (Debian/Ubuntu) 执行 `rzup install` 时报错：
```
error: reqwest::Error { kind: Request, url: "https://github.com/risc0/risc0/releases/download/v3.0.3/cargo-risczero-x86_64-unknown-linux-gnu.tgz", 
source: hyper_util::client::legacy::Error(SendRequest, hyper::Error(Io, Custom { kind: UnexpectedEof, error: "peer closed connection without sending TLS close_notify" })) }
```

**原因**：
- 本机到 GitHub Releases 的长连接质量不稳定（代理/防火墙/ISP 限流等）；
- TLS 握手或数据传输中途被中间设备中断，导致下载失败。

**临时缓解（不保证成功）**：
```bash
# 禁用可能干扰的代理变量
unset http_proxy https_proxy HTTP_PROXY HTTPS_PROXY
# 重试
source "$HOME/.risc0/bin/env"
rzup install --verbose
```

**最终解决方案（推荐）**：
- **本地构建**：设置 `RISC0_SKIP_BUILD=1` 环境变量或在非 CI 环境下自动跳过 RISC0 build（已在 `src/l2-executor/build.rs` 实现）；
- **性能测试**：通过 GitHub Actions（`.github/workflows/risc0-perf.yml`）在干净的 Ubuntu runner 上安装 RISC0 toolchain 并运行性能基准；
- **数据回填**：从 CI Artifacts 下载 `risc0_ci_run.log`，解析性能数据并更新 `SESSION-16-COMPLETION-REPORT.md`。

**本地开发不受影响**：
- 在不开启 `risc0-poc` feature 时，或在 Windows 上，`build.rs` 完全绕过 RISC0；
- 只要不启用 `risc0-poc` feature，本地构建和 Trace backend 测试不受任何影响。
