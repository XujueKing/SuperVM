# SuperVM - Roadmap (初始)

开发者：king

## 目标
- 使用 Rust 实现高性能 WASM-first runtime
- 支持 Solidity (via Solang) 与 JS/TS (AssemblyScript) 合约开发体验
- 可选 EVM 兼容层（revm/evmone 集成或 EVM->WASM 转译）

## 里程碑
1. 准备（周0）
   - 仓库骨架、开发规范、CI 初始
2. PoC（周1-3）
   - vm-runtime 最小运行时（wasmtime/wasmi），node-core demo（单节点、交易、执行）
3. 编译器适配（周4-8）
   - 集成 Solang、AssemblyScript；JS SDK + 部署脚本
4. 并行执行 PoC（周9-14）
   - 账户并行调度器、冲突检测、回滚机制
5. EVM 兼容（周15-22）
   - 集成 revm/evmone 或实现转译层，运行 ERC20 测试
6. 预发布与生产准备（周23-36）
   - 完整网络、共识插件、监控、审计、文档与开发者体验优化

## 交付物（每里程碑）
- vm-runtime crate: wasm runtime + host API
- node crate: 本地测试链 binary
- compiler-adapter: 自动化编译/打包流程 (WASM + metadata)
- evm-compat: 可插拔 EVM 后端
- js-sdk + hardhat plugin、示例 dApp、benchmark 报告

## 风险与缓解
- Solidity->WASM 语义差异：限制不支持特性、提供兼容层说明
- 并行执行复杂性：从可回滚的简单模型开始、增加冲突检测与重试
- 性能开销：选择高性能 runtime、对热路径优化与缓存

## 短期下一步（可选执行）
- PoC: 在 vm-runtime 集成 wasmi/wasmtime 并运行示例合约
- 集成 Solang demo：Solidity -> WASM -> 在本地 runtime 执行
- 生成 CI、ISSUE/PR 模板、Roadmap（本文件）

## 联系与贡献
请在 Issues 中提交 bug/feature 请求，或通过 PR 提交代码。遵循 CONTRIBUTING 指南。
