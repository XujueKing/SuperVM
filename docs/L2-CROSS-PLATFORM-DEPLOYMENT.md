# L2 Executor 跨平台部署指南

## 🎯 设计原则

L2 executor 采用**多层 fallback 机制**,确保在任何平台上都能正常工作:

```
Platform Detection → Feature Selection → Backend Loading
     ↓                      ↓                    ↓
  Windows            default (无 feature)    TraceZkVm
  Linux/WSL          risc0-poc              Risc0Backend
  Production         risc0-poc + halo2      可选择最优后端
```

---

## 📦 Cargo.toml 平台条件编译

### 当前配置
```toml
[features]
default = []                      # Windows 默认不启用任何 zkVM 后端
risc0-poc = ["dep:risc0-zkvm"]   # Linux/WSL 可选启用

[dependencies]
# 核心依赖 (跨平台)
anyhow = "1.0"
sha2 = "0.10"
serde = { version = "1.0", features = ["derive"] }
bincode = "1.3"

# RISC0 依赖 (仅非 Windows)
[target.'cfg(not(windows))'.dependencies]
risc0-zkvm = { version = "1.0", optional = true, ... }

[target.'cfg(not(windows))'.build-dependencies]
risc0-build = "1.0"
```

### 自动区分逻辑
```bash
# Windows 上编译
cargo build -p l2-executor
# → 不会尝试安装 risc0-zkvm
# → 只构建 TraceZkVm

# Linux/WSL 上编译
cargo build -p l2-executor --features risc0-poc
# → 安装 risc0-zkvm
# → 构建 TraceZkVm + Risc0Backend
```

---

## 🛡️ 编译时保护机制

### lib.rs 保护
```rust
#[cfg(all(feature = "risc0-poc", target_os = "windows"))]
compile_error!("`risc0-poc` feature requires a non-Windows host; please build on Linux or WSL");
```

**作用**: 防止用户在 Windows 上误启用 `risc0-poc` feature

### build.rs 保护
```rust
#[cfg(all(feature = "risc0-poc", not(windows)))]
fn main() {
    risc0_build::embed_methods();
}

#[cfg(any(not(feature = "risc0-poc"), windows))]
fn main() {
    // No-op on Windows or when risc0-poc is disabled
}
```

**作用**: 确保 Windows 上构建脚本安全跳过

---

## 🚀 部署场景

### Scenario 1: Windows 开发环境
**用例**: 开发者在 Windows 上编写业务逻辑

```powershell
# 1. 克隆项目
git clone https://github.com/XujueKing/SuperVM
cd SuperVM

# 2. 构建 L2 executor (默认 feature)
cargo build -p l2-executor

# 3. 运行测试 (不包含 RISC0)
cargo test -p l2-executor
# ✅ fibonacci_proof_roundtrip ... ok
# ✅ aggregator_combines_proofs ... ok
# ✅ sha256_proof_roundtrip ... ok
```

**可用功能**:
- ✅ `TraceZkVm` 默认虚拟机
- ✅ `MerkleAggregator` 证明聚合
- ✅ `ZkVmBackend` trait 接口定义
- ❌ `Risc0Backend` 不可用 (需 WSL)

---

### Scenario 2: Windows + WSL 混合开发
**用例**: 开发者需要测试 RISC0 集成

```powershell
# Windows PowerShell - 业务逻辑开发
cargo build -p l2-executor
cargo test -p l2-executor

# WSL - RISC0 性能测试
wsl
cd /mnt/d/WEB3_AI开发/虚拟机开发
cargo build -p l2-executor --features risc0-poc
cargo test -p l2-executor --features risc0-poc
# ✅ 包含所有 7 个测试 (含 RISC0)
```

**优势**: 无需重启,两个环境并行使用

---

### Scenario 3: Linux 生产环境
**用例**: 服务器部署,使用最优 zkVM 后端

```bash
# 1. 安装依赖
sudo apt-get update
sudo apt-get install build-essential libssl-dev

# 2. 构建生产版本
cargo build --release -p l2-executor --features risc0-poc

# 3. 运行服务 (自动选择 Risc0Backend)
./target/release/supervm-l2-executor
```

**配置文件** (可选):
```toml
# config/l2.toml
[zkvm]
backend = "risc0"      # 或 "trace" (fallback)
proof_mode = "groth16" # 或 "stark"
```

---

### Scenario 4: Docker 容器化部署
**用例**: 跨平台一致性部署

```dockerfile
# Dockerfile
FROM rust:1.85-bookworm

# 安装 RISC0 依赖
RUN apt-get update && apt-get install -y \
    build-essential \
    libssl-dev \
    pkg-config

# 复制源码
WORKDIR /app
COPY . .

# 构建 L2 executor (自动启用 risc0-poc)
RUN cargo build --release -p l2-executor --features risc0-poc

# 运行服务
CMD ["./target/release/supervm-l2-executor"]
```

**部署命令**:
```bash
docker build -t supervm-l2 .
docker run -p 8080:8080 supervm-l2
```

---

## 🔧 运行时后端选择

### 动态后端加载 (推荐实现)

```rust
// src/l2-executor/src/runtime.rs
use crate::backend_trait::ZkVmBackend;

pub enum BackendType {
    Trace,      // 默认 (跨平台)
    Risc0,      // Linux/WSL only
    Halo2,      // 未来支持
}

pub struct L2Runtime {
    backend: Box<dyn ZkVmBackend<...>>,
}

impl L2Runtime {
    pub fn new(backend_type: BackendType) -> Result<Self> {
        let backend: Box<dyn ZkVmBackend<...>> = match backend_type {
            BackendType::Trace => {
                Box::new(crate::zkvm::TraceZkVm::default())
            }
            
            #[cfg(all(feature = "risc0-poc", not(windows)))]
            BackendType::Risc0 => {
                Box::new(crate::risc0_backend::Risc0Backend::new())
            }
            
            #[cfg(not(all(feature = "risc0-poc", not(windows))))]
            BackendType::Risc0 => {
                return Err(anyhow::anyhow!(
                    "RISC0 backend requires Linux/WSL and risc0-poc feature"
                ));
            }
            
            BackendType::Halo2 => {
                todo!("Halo2 backend not yet implemented")
            }
        };
        
        Ok(Self { backend })
    }
    
    pub fn auto_select() -> Result<Self> {
        #[cfg(all(feature = "risc0-poc", not(windows)))]
        {
            log::info!("Auto-selecting RISC0 backend (Linux detected)");
            Self::new(BackendType::Risc0)
        }
        
        #[cfg(not(all(feature = "risc0-poc", not(windows))))]
        {
            log::info!("Auto-selecting Trace backend (Windows or no RISC0 feature)");
            Self::new(BackendType::Trace)
        }
    }
}
```

### 使用示例

```rust
// 业务代码 (跨平台)
use l2_executor::L2Runtime;

fn main() -> Result<()> {
    // 自动选择最佳后端
    let runtime = L2Runtime::auto_select()?;
    
    // 或手动指定
    let runtime = L2Runtime::new(BackendType::Trace)?;
    
    // 使用统一接口
    let proof = runtime.prove(program_id, inputs)?;
    let verified = runtime.verify(proof)?;
    
    Ok(())
}
```

---

## 📊 功能对比表

| 功能模块 | Windows (默认) | Linux/WSL (risc0-poc) | 生产环境 |
|---------|---------------|----------------------|---------|
| `TraceZkVm` | ✅ | ✅ | ✅ |
| `MerkleAggregator` | ✅ | ✅ | ✅ |
| `ZkVmBackend` trait | ✅ | ✅ | ✅ |
| `Risc0Backend` | ❌ | ✅ | ✅ |
| `Halo2Backend` | 📋 计划 | 📋 计划 | 📋 计划 |
| 性能基准测试 | ❌ | ✅ | ✅ |
| 单元测试 (基础) | ✅ 3/3 | ✅ 7/7 | ✅ 7/7 |

**图例**:
- ✅ 完全支持
- ❌ 不支持 (平台限制)
- 📋 规划中

---

## 🎯 最佳实践

### 开发阶段
```bash
# Windows 开发者
1. 使用默认 feature 开发业务逻辑
2. 在 WSL 中测试 RISC0 集成
3. 提交前运行 WSL 完整测试套件

# Linux 开发者
1. 直接启用 risc0-poc feature
2. 运行完整测试 (包含性能基准)
3. 提交前验证 Windows 兼容性 (禁用 feature)
```

### CI/CD 配置
```yaml
# .github/workflows/ci.yml
name: CI

on: [push, pull_request]

jobs:
  test-windows:
    runs-on: windows-latest
    steps:
      - uses: actions/checkout@v4
      - name: Test default features
        run: cargo test -p l2-executor
      # 不启用 risc0-poc
  
  test-linux:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - name: Install RISC0 dependencies
        run: |
          sudo apt-get update
          sudo apt-get install -y build-essential
      - name: Test with RISC0
        run: cargo test -p l2-executor --features risc0-poc
      - name: Run benchmarks
        run: cargo bench -p zkvm-bench --features risc0-bench
```

### 生产部署检查清单
- [ ] 确认目标平台 (Linux/Docker)
- [ ] 启用 `risc0-poc` feature
- [ ] 运行完整测试套件
- [ ] 执行性能基准测试
- [ ] 配置日志级别 (`RUST_LOG=info`)
- [ ] 设置 `RISC0_DEV_MODE=0` (生产模式)

---

## 🔍 故障排查

### 问题 1: Windows 上启用 risc0-poc 失败
```powershell
PS> cargo build --features risc0-poc
error: `risc0-poc` feature requires a non-Windows host
```

**解决方案**:
```powershell
# 方案 1: 移除 feature (使用默认 TraceZkVm)
cargo build -p l2-executor

# 方案 2: 使用 WSL
wsl
cd /mnt/d/WEB3_AI开发/虚拟机开发
cargo build -p l2-executor --features risc0-poc
```

### 问题 2: 运行时找不到 Risc0Backend
```rust
Error: RISC0 backend requires Linux/WSL and risc0-poc feature
```

**解决方案**:
```rust
// 使用 auto_select() 自动降级
let runtime = L2Runtime::auto_select()?;

// 或手动指定 Trace backend
let runtime = L2Runtime::new(BackendType::Trace)?;
```

### 问题 3: Docker 构建失败
```dockerfile
error: failed to run custom build command for `l2-executor`
```

**解决方案**:
```dockerfile
# 确保安装 RISC0 依赖
RUN apt-get update && apt-get install -y \
    build-essential \
    libssl-dev \
    pkg-config \
    clang       # RISC0 需要
```

---

## 📚 相关文档
- `src/l2-executor/README.md` - L2 执行层概览
- `zkvm-bench/README.md` - 性能测试指南
- `RISC0-POC-README.md` - RISC0 集成详解
- `docs/L2-ZKVM-POC-COMPLETION-REPORT.md` - PoC 完成报告

---

**更新时间**: 2025-11-14  
**适用版本**: l2-executor v0.1.0+  
**维护者**: king@example.com
