# WSL2 + Linux Vulkan GPU 测试环境配置指南

## 前置要求

- Windows 10 版本 2004+ 或 Windows 11

- 管理员权限

## 步骤 1: 安装 WSL2 和 Ubuntu

**以管理员身份打开 PowerShell**（右键开始菜单 → "Windows PowerShell (管理员)"）

```powershell

# 安装 WSL2 和 Ubuntu 24.04 LTS

wsl --install -d Ubuntu-24.04

# 如果已安装 WSL 但需要启用 WSL2，执行：

wsl --set-default-version 2

```

安装完成后会自动启动 Ubuntu，首次运行需要：
1. 设置 Linux 用户名（小写，无特殊字符）
2. 设置密码（输入时不显示，输两次）

## 步骤 2: 在 WSL Ubuntu 中安装开发环境

进入 WSL（在 PowerShell 执行 `wsl` 或打开 Ubuntu 应用）后：

```bash

# 更新包管理器

sudo apt update && sudo apt upgrade -y

# 安装编译工具链

sudo apt install -y build-essential pkg-config libssl-dev

# 安装 Rust

curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# 选择默认安装（输入 1）

source "$HOME/.cargo/env"

# 验证安装

rustc --version
cargo --version

```

## 步骤 3: 安装 Vulkan 运行时支持

```bash

# 安装 Vulkan 开发包和工具

sudo apt install -y \
    vulkan-tools \
    mesa-vulkan-drivers \
    libvulkan-dev

# 验证 Vulkan 可用（如果有 GPU 驱动会显示设备信息）

vulkaninfo --summary

```

**重要说明**：

- WSL2 中的 GPU 支持需要 **Windows 11** 或 **Windows 10 21H2+** 且安装了最新显卡驱动

- 如果 `vulkaninfo` 报错或未找到设备，可能需要：
  1. 更新 Windows 到最新版本
  2. 更新显卡驱动（NVIDIA/AMD/Intel）
  3. 检查 WSL2 内核版本：`wsl --version`（需要 5.10.16.3+）

## 步骤 4: 在 WSL 中运行 GPU 阈值扫描

```bash

# 进入项目目录（从 Windows 挂载）

cd /mnt/d/WEB3_AI开发/虚拟机开发

# 首次编译（需要几分钟）

cargo build -p vm-runtime --example gpu_threshold_scan_demo \
    --features hybrid-exec,hybrid-gpu-wgpu

# 运行 Vulkan 扫描

./scripts/run-gpu-threshold-scan.sh -b vulkan

# 输出文件位置

ls -lh data/gpu_threshold_scan/linux_vulkan_*.csv

```

## 步骤 5: 生成 Linux 对比数据并更新跨平台汇总

**回到 Windows PowerShell**（非管理员也可）：

```powershell

# 找到最新的 Linux Vulkan CSV

$linuxCsv = (Get-ChildItem "data/gpu_threshold_scan/linux_vulkan_*.csv" | 
             Sort-Object LastWriteTime -Descending | 
             Select-Object -First 1).FullName

# 生成 Linux 对比 CSV（如果需要单独 GLES vs Vulkan 对比，需修改脚本）

# 当前简化版直接转换：

powershell -NoProfile -ExecutionPolicy Bypass -File "scripts/generate-linux-compare.ps1" `
    -LinuxVulkan $linuxCsv `
    -OutFile "data/gpu_threshold_scan/linux_compare_$(Get-Date -Format 'yyyyMMdd').csv"

# 更新跨平台汇总

powershell -NoProfile -ExecutionPolicy Bypass -File "scripts/unify-gpu-platforms.ps1" `
    -WindowsCompare "data/gpu_threshold_scan/windows_compare_20251112.csv" `
    -LinuxCompare "data/gpu_threshold_scan/linux_compare_$(Get-Date -Format 'yyyyMMdd').csv"

# 更新 HTML 图表

powershell -NoProfile -ExecutionPolicy Bypass -File "scripts/generate-gpu-platform-compare-html.ps1"

```

## 常见问题

### WSL 安装失败

```powershell

# 手动启用必需功能

dism.exe /online /enable-feature /featurename:Microsoft-Windows-Subsystem-Linux /all /norestart
dism.exe /online /enable-feature /featurename:VirtualMachinePlatform /all /norestart

# 重启后再执行

wsl --install -d Ubuntu-24.04

```

### Vulkan 不可用

如果 WSL2 内 `vulkaninfo` 无输出或报错：
1. **检查 Windows 版本**：`winver`（需要 Windows 11 或 10 build 21H2+）
2. **更新 WSL 内核**：
   ```powershell
   wsl --update
   wsl --shutdown
   wsl
   ```
3. **使用 CPU-only 模式**：虽然无法测试真实 GPU，但可验证流程
   ```bash
   # 设置环境变量强制 CPU（仅测试用）
   export SUPERVM_WGPU_BACKENDS=cpu
   ./scripts/run-gpu-threshold-scan.sh -b vulkan  # 实际走 CPU 路径
   ```

### 编译错误

```bash

# 清理并重试

cargo clean
cargo build -p vm-runtime --example gpu_threshold_scan_demo \
    --features hybrid-exec,hybrid-gpu-wgpu

```

## 快速验证命令（无 GPU 环境）

如果无法启用 WSL2 GPU 支持，可以在 WSL 内生成模拟数据：

```bash
cd /mnt/d/WEB3_AI开发/虚拟机开发
cat > data/gpu_threshold_scan/linux_vulkan_test_$(date +%Y%m%d).csv << 'EOF'
size,device,duration_ms,first,backend_mask_hint
20000,Cpu,0,0,vulkan
20000,Gpu,15,0,vulkan
50000,Cpu,1,0,vulkan
50000,Gpu,2,0,vulkan
100000,Cpu,2,0,vulkan
100000,Gpu,1,0,vulkan
250000,Cpu,5,0,vulkan
250000,Gpu,2,0,vulkan
500000,Cpu,10,0,vulkan
500000,Gpu,5,0,vulkan
1000000,Cpu,21,0,vulkan
1000000,Gpu,10,0,vulkan
EOF

```

然后在 Windows 运行后续脚本即可测试流程。

## 性能调优提示

WSL2 内 I/O 性能优化：

- 代码放在 WSL 文件系统内（`~/projects/`）而非 `/mnt/d/` 可提速 2-10 倍

- 使用 `WSL2 + Docker Desktop` 可共享 GPU 环境

- 大规模测试建议使用 `--release` 编译减少开销

---
生成时间：2025-11-12
