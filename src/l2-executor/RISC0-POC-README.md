# L2 Executor - RISC0 zkVM PoC

本目录包含 SuperVM L2 执行层的 RISC0 zkVM 概念验证实现。

## 组件

- **`src/risc0_backend.rs`**: RISC0 prover/verifier 后端封装
- **`methods/fibonacci/`**: RISC0 guest program（Fibonacci 计算示例）
- **`build.rs`**: 构建脚本，自动编译 guest 并生成 ELF/ID 常量

## 快速开始

### 在 Linux/WSL 环境运行测试

```bash
# 1. 确保已安装 RISC0 工具链
curl -L https://risczero.com/install | bash
source ~/.bashrc
rzup install

# 2. 运行测试（开发模式，快速验证）
cd /mnt/d/WEB3_AI开发/虚拟机开发
RISC0_DEV_MODE=1 cargo test -p l2-executor --features risc0-poc

# 或使用提供的脚本
./scripts/test-risc0-poc.sh
```

### Windows 限制

由于 RISC0 工具链依赖 RISC-V 目标和 GNU 工具链，Windows 原生环境无法编译。请使用 WSL 或 Linux 环境。

## 架构说明

```
l2-executor
├── src/
│   ├── risc0_backend.rs   # Host 端 API（prove/verify）
│   └── lib.rs             # 条件导出 (cfg feature risc0-poc)
└── methods/
    └── fibonacci/
        ├── src/main.rs    # Guest 端入口（no_std）
        └── Cargo.toml     # 独立 workspace（使用 risc0 工具链）
```

编译流程：
1. `build.rs` 调用 `risc0-build::embed_methods()`
2. RISC0 工具链编译 guest → ELF 二进制
3. 生成 `methods.rs`，包含 `L2_EXECUTOR_METHODS_FIBONACCI_ELF` 等常量
4. Host 代码通过 `include!` 引入并调用

## 下一步

- [ ] 实现通用 zkVM trait，抽象 RISC0/SP1/Jolt 后端
- [ ] 接入 L2ExecutionBridge 演示端到端流程
- [ ] 添加性能基准与实际电路约束分析
