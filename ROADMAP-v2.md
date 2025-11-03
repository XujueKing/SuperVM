# SuperVM - Development Roadmap

开发者: king

最后更新: 2025-01-03

## 项目目标

- 使用 Rust 实现高性能 WASM-first runtime
- 支持 Solidity (via Solang) 与 JS/TS (AssemblyScript) 合约开发体验
- 可选 EVM 兼容层(revm/evmone 集成或 EVM->WASM 转译)
- 并行执行引擎以提升吞吐量

## 里程碑进度

### ✅ 阶段 1: 准备工作 (周0) - 已完成

**目标**: 搭建项目基础设施

- ✅ Cargo workspace 配置
- ✅ 开发规范与贡献指南 (CONTRIBUTING.md)
- ✅ GitHub 问题模板 (bug_report, feature_request)
- ✅ GitHub PR 模板
- ✅ .editorconfig 编辑器配置
- ✅ 初始 ROADMAP.md

**产出**: 规范化的项目结构和协作流程

---

### ✅ 阶段 2: PoC - WASM 运行时 (周1-3) - 已完成

**目标**: 实现基础 WASM 执行能力和核心 Host Functions

#### 已完成功能

**vm-runtime crate (v0.1.0)**:
- ✅ wasmtime 17.0 集成
- ✅ Storage 抽象层
  - ✅ `Storage` trait 定义
  - ✅ `MemoryStorage` 实现 (BTreeMap 后端)
- ✅ Host Functions 架构
  - ✅ `storage_api` 模块: get, read_value, set, delete
  - ✅ `chain_api` 模块: block_number, timestamp, emit_event, events_len, read_event
- ✅ 公共 API:
  - ✅ `Runtime::new(storage)`
  - ✅ `Runtime::execute_add()` (demo)
  - ✅ `Runtime::execute_with_context()` (核心 API)
- ✅ 单元测试覆盖 (6/6 通过):
  - ✅ test_memory_storage
  - ✅ test_execute_add_via_wat
  - ✅ test_storage
  - ✅ test_host_functions
  - ✅ test_emit_event
  - ✅ test_execute_with_context

**node-core crate (v0.1.0)**:
- ✅ CLI 程序框架
- ✅ `--once` 标志支持 (自动化测试)
- ✅ 日志集成 (tracing + tracing_subscriber)
- ✅ Demo 程序:
  - ✅ Demo 1: 基础 add 函数演示
  - ✅ Demo 2: 完整事件系统演示

**文档**:
- ✅ README.md (完整使用指南)
- ✅ CHANGELOG.md (版本记录)
- ✅ API 参考表格

**测试验证**:
```bash
cargo test -p vm-runtime    # 6/6 通过
cargo run -p node-core --once  # 端到端验证成功
```

**产出**: 
- 可运行的 WASM 虚拟机
- 完整的存储和事件系统
- 详细的开发文档

---

### 🚧 阶段 3: 编译器适配 (周4-8) - 规划中

**目标**: 支持主流智能合约语言编译到 WASM

#### 待实现功能

**Solidity 支持 (via Solang)**:
- [ ] 集成 Solang 编译器
- [ ] Solidity 标准库适配
- [ ] Contract ABI 生成
- [ ] 部署脚本工具
- [ ] ERC20 示例合约

**AssemblyScript 支持**:
- [ ] AssemblyScript 编译配置
- [ ] 标准库绑定
- [ ] TypeScript 类型定义
- [ ] 示例合约模板

**开发工具**:
- [ ] compiler-adapter crate
- [ ] 自动化构建脚本
- [ ] WASM 优化流程 (wasm-opt)
- [ ] 元数据打包工具

**JS SDK**:
- [ ] npm 包结构
- [ ] 合约部署 API
- [ ] 合约调用封装
- [ ] Event 监听接口
- [ ] Hardhat 插件

**测试与文档**:
- [ ] Solidity 集成测试
- [ ] AssemblyScript 示例
- [ ] 开发者指南
- [ ] API 文档网站

**交付物**:
- compiler-adapter: 编译器集成工具
- js-sdk: JavaScript/TypeScript 开发套件
- 示例 dApp 项目
- 完整开发文档

---

### � 阶段 4: 并行执行引擎 (周9-14) - 进行中

**目标**: 实现基于账户的并行交易执行

#### 已完成功能 ✅

**调度系统**:
- ✅ 交易依赖分析 (DependencyGraph)
- ✅ 账户访问模式提取 (ReadWriteSet)
- ✅ 并行执行调度器 (ParallelScheduler)
- [ ] 工作窃取算法

**冲突检测**:
- ✅ 读写集收集 (ReadWriteSet::add_read/add_write)
- ✅ 冲突检测算法 (ConflictDetector)
- [ ] 重试机制
- [ ] 性能优化

**状态管理**:
- ✅ 快照与回滚 (StorageSnapshot + StateManager)
- ✅ 事务保护执行 (execute_with_snapshot)
- ✅ 嵌套快照支持
- [ ] MVCC 实现研究
- [ ] 批量提交优化
- [ ] 内存池管理

**性能测试**:
- [ ] 基准测试框架
- [ ] 吞吐量测试
- [ ] 延迟测试
- ✅ 并发正确性验证 (8 个单元测试通过)

**演示程序**:
- ✅ Demo 5: 并行冲突检测演示
- ✅ Demo 6: 状态快照与回滚演示

**测试覆盖**:
- ✅ 并行执行测试 (6/6 通过)
- ✅ 状态快照测试 (5/5 通过)
- ✅ 调度器集成测试 (3/3 通过)
- ✅ 总计: 29/29 测试通过

**交付物**:
- ✅ parallel 模块 (vm-runtime::parallel)
- ✅ ReadWriteSet, ConflictDetector, DependencyGraph
- ✅ ParallelScheduler 集成 StateManager
- [ ] 性能测试报告
- [ ] 并行执行设计文档

**进度**: 🎯 约 65% 完成

---

### 📋 阶段 5: EVM 兼容层 (周15-22)

**目标**: 支持现有以太坊合约迁移

#### 计划功能

**EVM 集成**:
- [ ] revm/evmone 评估
- [ ] EVM 后端插件化
- [ ] EVM->WASM 转译器研究
- [ ] Opcode 映射表

**兼容性**:
- [ ] Ethereum JSON-RPC API
- [ ] Gas 计量系统
- [ ] Precompiled 合约
- [ ] ERC 标准支持 (20/721/1155)

**测试验证**:
- [ ] Ethereum 测试套件
- [ ] DeFi 协议测试 (Uniswap, AAVE)
- [ ] NFT 市场测试
- [ ] 跨合约调用测试

**交付物**:
- evm-compat crate
- 兼容性测试报告
- 迁移指南

---

### 📋 阶段 6: 生产准备 (周23-36)

**目标**: 完善功能,达到生产可用标准

#### 计划功能

**网络层**:
- [ ] P2P 网络实现
- [ ] 区块同步
- [ ] 交易广播
- [ ] 节点发现

**共识系统**:
- [ ] 共识插件接口
- [ ] PoW/PoS 示例实现
- [ ] 区块验证
- [ ] 最终性确认

**监控与运维**:
- [ ] Prometheus 指标
- [ ] Grafana 仪表盘
- [ ] 日志聚合
- [ ] 告警系统

**安全审计**:
- [ ] 代码审计
- [ ] 模糊测试
- [ ] 安全加固
- [ ] 漏洞赏金计划

**文档完善**:
- [ ] 架构设计文档
- [ ] 运维手册
- [ ] API 完整文档
- [ ] 最佳实践指南

**交付物**:
- 生产级区块链节点
- 完整监控系统
- 审计报告
- 全面文档

---

## 技术栈总结

### 核心组件
- **Runtime**: wasmtime (JIT 编译)
- **存储**: 抽象层 + 可插拔后端
- **语言支持**: Rust (原生), Solidity (Solang), AssemblyScript
- **网络**: libp2p
- **共识**: 插件化设计
- **监控**: Prometheus + Grafana

### 依赖版本
- Rust: stable (1.70+)
- wasmtime: 17.0
- tokio: 1.35 (异步运行时)
- tracing: 0.1 (日志)
- clap: 4.4 (CLI)

---

## 风险评估与缓解

### 技术风险

**Solidity->WASM 语义差异**
- 风险: Solidity 特性可能无法完美映射到 WASM
- 缓解: 
  - 明确标注不支持的特性
  - 提供兼容层说明文档
  - 与 Solang 社区保持沟通

**并行执行复杂性**
- 风险: 并发 bug 难以调试和复现
- 缓解:
  - 从简单的快照-回滚模型开始
  - 完善的测试套件
  - 形式化验证工具

**性能开销**
- 风险: WASM 执行和 Host 调用可能成为瓶颈
- 缓解:
  - 选择高性能 runtime (wasmtime JIT)
  - 热路径优化
  - 缓存机制
  - 定期性能基准测试

### 生态风险

**开发者采用**
- 风险: 开发者习惯现有工具链
- 缓解:
  - 提供熟悉的开发体验 (Hardhat 插件)
  - 完善文档和示例
  - 社区支持和教程

**EVM 兼容性**
- 风险: 完全兼容 EVM 成本高
- 缓解:
  - 采用分阶段策略
  - 先支持核心功能
  - 根据需求逐步完善

---

## 当前状态总结 (2025-01-03)

### ✅ 已完成
- [x] 基础设施搭建
- [x] WASM 运行时 (wasmtime)
- [x] 存储抽象层
- [x] Host Functions (storage + chain + events + crypto)
- [x] execute_with_context API
- [x] 单元测试覆盖
- [x] CLI 工具和演示
- [x] 开发文档
- [x] 并行执行核心功能 (冲突检测、依赖分析、调度器)
- [x] 状态快照与回滚系统

### 🚧 进行中
- [x] 并行执行引擎 (65% 完成)
  - ✅ 核心调度系统
  - ✅ 冲突检测
  - ✅ 状态管理 (快照/回滚)
  - ⏳ 性能优化与测试

### 📋 待开始
- [ ] 并行执行完善 (性能优化、工作窃取)
- [ ] 编译器适配
- [ ] EVM 兼容
- [ ] 生产准备

### 📊 进度
- **阶段 1**: ✅ 100%
- **阶段 2**: ✅ 100%
- **阶段 3**: 📋 0%
- **阶段 4**: 🚧 65%
- **整体进度**: 🎯 44% (2.65/6 阶段完成)

---

## 下一步行动 (Next Sprint)

1. **立即**:
   - 推送代码到远程仓库 (feat/vm-events-clean)
   - 创建 PR 到 main 分支
   - 发布 v0.1.0 release

2. **本周**:
   - 调研 Solang 编译器集成方案
   - 设计 compiler-adapter 架构
   - 开始 JS SDK 原型开发

3. **本月**:
   - 完成 Solidity 编译器集成
   - 实现基础 JS SDK
   - 编写 ERC20 示例合约

---

## 参与贡献

欢迎贡献! 请参阅:
- [CONTRIBUTING.md](CONTRIBUTING.md) - 贡献指南
- [DEVELOPER.md](DEVELOPER.md) - 开发者文档
- [GitHub Issues](https://github.com/XujueKing/SuperVM/issues) - 问题反馈

## 联系方式

- 开发者: king
- Email: king@example.com
- GitHub: [@XujueKing](https://github.com/XujueKing)

---

*Roadmap 会根据开发进度和社区反馈持续更新*
