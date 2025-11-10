# SuperVM - Next-Generation Decentralized Virtual Machine

> **潘多拉星核 (Pandora Core)**: Web3 基础设施操作系统  
> 📄 **[白皮书 (中文)](./WHITEPAPER.md)** | 📄 **[Whitepaper (EN)](./WHITEPAPER_EN.md)** | 🗺️ **[路线图](./ROADMAP.md)** | 📚 **[开发文档](./DEVELOPER.md)**  
> 🎨 **[资产生成指南](./ASSETS-README.md)** | 🚀 **[快速开始](./QUICK-START-ASSETS.md)**

**开发者**: Rainbow Haruko(CHINA) / king(CHINA) / NoahX(CHINA) / Alan Tang(CHINA) / Xuxu(CHINA)

---

## 项目概述

SuperVM 是一个高性能的 WASM-first 区块链虚拟机，聚焦内核纯净与并行执行：
- ⚡ 并行执行 + MVCC 并发控制：单线程 242K TPS（Windows 本地），多线程高竞争 ~290K TPS（本地基准）；批量写入峰值 754K–860K ops/s（存储微基准，非 TPS）
- 🧠 内核分级保护：L0（核心运行时/调度/MVCC），L1（内核扩展），L2+（接口/插件/应用）
- 🔌 插件化兼容：EVM 通过适配器在插件层实现，零入侵内核
- 🔒 隐私专项：ZK/环签等在独立模块推进（参见 ROADMAP-ZK-Privacy）

**核心定位**: 不是"跨链桥"，而是**多链聚合器** + **Web3 操作系统**

当前工作区版本：0.5.0（活跃开发）

## 🚩 最新进展亮点（2025-11-09）

- 🆕 **双曲线 Solidity 验证器 (Phase 2.2)**：
  - **BLS12-381** (128-bit 安全,未来 EVM 2.0) + **BN254** (100-bit,当前 EVM 原生支持)
  - 统一架构 CurveKind 枚举,两条曲线完全并行,互不影响
  - BN254 合约 3474 字节,使用 EVM 预编译 0x08 (低 Gas ~150K-200K)
  - BLS12-381 合约 5574 字节,面向 zkEVM 2.0 与长期安全
  - Gas 优化: external+calldata 签名,gamma_abc 内联展开,移除动态数组
  - 示例: `generate_bn254_multiply_sol_verifier.rs` (BN254) + 测试 (BLS12-381)
  - 详见: [DUAL-CURVE-VERIFIER-GUIDE.md](docs/DUAL-CURVE-VERIFIER-GUIDE.md)
- 🆕 **RingCT 并行证明与批量验证 (Phase 2.3)**：
  - 全局 ProvingKey 缓存(once_cell),消除重复setup开销(节省1-2秒/实例)
  - RingCT 并行证明: 50.8 proofs/sec (批次32,延迟19.7ms,100%成功率)
  - 批量验证: 104.6 verifications/sec (8倍提升vs逐个验证)
  - HTTP基准测试: :9090/metrics (Prometheus), /summary (人类可读)
  - Grafana监控: 7个面板,3条告警规则,完整部署指南
  - Fast→Consensus回退: 环境变量配置,自动路由降级
- 🆕 **快照管理/恢复/自动清理**：支持 create_checkpoint、restore_from_checkpoint、maybe_create_snapshot、cleanup_old_snapshots，3 个测试用例全部通过
- 🆕 **MVCC 自动刷新机制**：flush_to_storage、load_from_storage，支持双触发器（时间+区块数），demo 稳定运行
- 🆕 **Prometheus 指标集成**：metrics.rs 模块（MetricsCollector + LatencyHistogram），集成到 MVCC commit/commit_parallel，export_prometheus 导出，metrics_demo 运行成功（TPS≈669, 成功率≈98.61%，该 demo 仅用于健康检查，不代表性能上限）
- 🆕 **HTTP /metrics 端点**：metrics_http_demo 提供 Prometheus 监控接口，支持 GET http://127.0.0.1:8080/metrics
- 🆕 **状态裁剪功能**：prune_old_versions 批量清理历史版本，state_pruning_demo 成功清理 150 版本（10 键 × 15 旧版本）
- 🆕 **文档/编码规范升级**：90 个 Markdown 文件批量转换为 UTF-8，.vscode/settings.json 强制 UTF-8 编码
- 🆕 **新文档**：`docs/METRICS-COLLECTOR.md`（指标收集器）、`docs/PHASE-4.3-WEEK3-4-SUMMARY.md`（阶段总结）、`docs/ROCKSDB-ADAPTIVE-QUICK-START.md`（批量写入指南）
 - 🆕 **Phase 5 三通道路由**：Fast/Consensus/Private 路径落地，新增基准与 E2E 示例（见下）

### ⏳ 待补充/优化
- [ ] Grafana Dashboard 配置（性能可视化）
- [ ] 24小时稳定性测试（长期运行验证）
- [ ] 单元测试/集成测试补充
- [ ] API.md 文档补全（新 API 汇总）

---

## 🚀 快速演示命令

```powershell
# 双曲线 Solidity 验证器生成 (Phase 2.2)
# BN254 (当前 EVM 链,使用预编译 0x08,低 Gas ~150K-200K)
cargo run -p vm-runtime --features groth16-verifier --example generate_bn254_multiply_sol_verifier --release
# 输出: contracts/BN254MultiplyVerifier.sol (3474 bytes)

# BLS12-381 测试 (未来 EVM 2.0,高安全 128-bit)
cargo test -p vm-runtime --features groth16-verifier privacy::solidity_verifier --lib -- --nocapture
# 输出: target/contracts/MultiplyVerifier.sol (5574 bytes)

# RingCT 并行证明 HTTP 基准测试 (Phase 2.3)
cargo run -p vm-runtime --features groth16-verifier --example zk_parallel_http_bench --release
# 访问: http://localhost:9090/metrics (Prometheus) 和 /summary (摘要)

# Phase 5：Fast Path 基准（可设置 FAST_PATH_ITERS/FAST_PATH_OBJECTS）
cargo run -p vm-runtime --example fast_path_bench --release

# Phase 5：混合负载基准（可设置 MIXED_ITERS/OWNED_RATIO/OWNED_OBJECTS/SHARED_OBJECTS）
cargo run -p vm-runtime --example mixed_path_bench --release

# Phase 5：混合负载 + /metrics（可选：边跑边抓路由/FastPath/Consensus 指标）
cargo run -p vm-runtime --example mixed_path_bench --release -- --serve-metrics:8082

# Phase 5：三通道 E2E 验证
cargo run -p vm-runtime --example e2e_three_channel_test --release

# 快照/恢复/自动清理功能演示
cargo run -p vm-runtime --example mvcc_auto_flush_demo --release --features rocksdb-storage

# Prometheus 指标采集演示
cargo run -p vm-runtime --example metrics_demo --release

# HTTP /metrics 端点演示 (监听 http://127.0.0.1:8080/metrics)
cargo run -p vm-runtime --example metrics_http_demo --release

# 状态裁剪演示 (清理历史版本)
cargo run -p vm-runtime --example state_pruning_demo --release --features rocksdb-storage

# RocksDB 批量写入基准测试
cargo run -p node-core --example rocksdb_adaptive_batch_bench --release --features rocksdb-storage
```

---

## 📚 关键文档入口

- [DUAL-CURVE-VERIFIER-GUIDE.md](docs/DUAL-CURVE-VERIFIER-GUIDE.md) - 双曲线 Solidity 验证器指南 (BLS12-381 + BN254) 🔐 **NEW**
- [METRICS-COLLECTOR.md](docs/METRICS-COLLECTOR.md) - Prometheus 指标收集器文档
- [PARALLEL-PROVER-GUIDE.md](docs/PARALLEL-PROVER-GUIDE.md) - RingCT 并行证明快速参考 🔐
- [RINGCT-PERFORMANCE-BASELINE.md](docs/RINGCT-PERFORMANCE-BASELINE.md) - RingCT 性能基准数据 📊
- [GRAFANA-RINGCT-PANELS.md](docs/GRAFANA-RINGCT-PANELS.md) - Grafana RingCT 面板配置 📈
- [GRAFANA-QUICK-DEPLOY.md](docs/GRAFANA-QUICK-DEPLOY.md) - 监控系统快速部署 🚀
- [PHASE-4.3-WEEK3-4-SUMMARY.md](docs/PHASE-4.3-WEEK3-4-SUMMARY.md) - Week 3-4 阶段总结
- [ROCKSDB-ADAPTIVE-QUICK-START.md](docs/ROCKSDB-ADAPTIVE-QUICK-START.md) - RocksDB 批量写入快速指南
- [sui-smart-contract-analysis.md](docs/sui-smart-contract-analysis.md) - Sui 对象模型与 SuperVM 三通道路由（Phase 5）
- [ROADMAP.md](ROADMAP.md) - 项目进度与阶段目标
- [docs/INDEX.md](docs/INDEX.md) - 全部文档导航

---

## 📝 阶段性总结（2025-11-09）

1. **双曲线 Solidity 验证器完成 (Phase 2.2 Task 1)**：BLS12-381 (未来 EVM 2.0, 128-bit 安全) + BN254 (当前 EVM 原生,低 Gas) 双后端实现,合约生成测试通过,文档完整。
2. **RingCT 并行证明与批量验证 (Phase 2.3)**：50.8 proofs/sec (并行),104.6 verifications/sec (批量),Grafana 监控完整部署,HTTP 基准测试稳定。
3. 快照、自动刷新、Prometheus 指标、HTTP /metrics 端点、状态裁剪五大功能全部落地,demo 与测试用例均通过。
4. 性能数据对齐：单线程事务提交 242K TPS；多线程高竞争 ~290K TPS；RocksDB 批量写入 754K–860K ops/s。
5. 文档与编码规范同步升级，90+ 文档批量转换为 UTF-8，开发体验与可维护性提升。
6. 剩余任务：Gas 成本测量 (BN254 testnet 部署)、批量验证集成 SuperVM、24h 稳定性测试、Grafana 生产配置。
5. 详细进展、数据与代码示例见 `docs/PHASE-4.3-WEEK3-4-SUMMARY.md`、`docs/METRICS-COLLECTOR.md`。

### 快速入口
- 路线图与阶段规划：`ROADMAP.md`
- 内核速用指南（含上帝分支）：`docs/KERNEL-QUICK-START.md`
- 内核定义与保护机制：`docs/KERNEL-DEFINITION.md`
- 模块分级与版本索引：`docs/KERNEL-MODULES-VERSIONS.md`
- EVM 适配器设计：`docs/evm-adapter-design.md`
- 架构资料与对比：`docs/architecture-2.0.md`、`docs/tech-comparison.md`
- 热键与 LFU 分层调优：`docs/LFU-HOTKEY-TUNING.md`
- **自适应性能调优 (AutoTuner)**: `docs/AUTO-TUNER.md` ⭐ **NEW**
- Bloom Filter 优化分析：`docs/bloom-filter-optimization-report.md`
- **RocksDB 持久化存储**: `docs/PHASE-4.3-ROCKSDB-INTEGRATION.md` 🔥
- **自适应批量写入快速开始**: `docs/ROCKSDB-ADAPTIVE-QUICK-START.md` 🚀 **NEW**
- **性能指标收集 (Prometheus)**: `docs/METRICS-COLLECTOR.md` 📊 **NEW**
- **Phase 4.3 Week 3-4 总结**: `docs/PHASE-4.3-WEEK3-4-SUMMARY.md` 📝 **NEW**

### 🔬 性能调优与基准测试

#### 性能矩阵（当前验证）

- 单线程 MVCC 提交: 242K TPS（Windows 本地）
- 多线程高竞争（并行提交）: ~290K TPS（本地基准）
- RocksDB 批量写入微基准: 754K–860K ops/s（存储吞吐，非 TPS）
- 指标字段（Prometheus 导出）:
  - mvcc_tps（总体 TPS，自启动以来）
  - mvcc_tps_window（窗口 TPS，滚动计算）
  - mvcc_tps_peak（峰值 TPS，以窗口为口径）
  - mvcc_txn_latency_ms{quantile="0.5|0.9|0.99"}（事务延迟百分位，单位 ms）

注：examples/metrics_demo 与 metrics_http_demo 输出仅用于健康检测，不代表性能上限。

#### 自适应调优演示 (AutoTuner)

```powershell
# 运行自适应 vs 手动配置对比演示
cargo run -p node-core --example auto_tuner_demo --release

# 预期输出: Manual ~425K TPS, Auto ~487K TPS (+14.59%)
```

#### Bloom Filter 公平基准测试

```powershell
# 固定批次大小测试
$env:BATCH_SIZE='200'; cargo run -p node-core --example bloom_fair_bench --release

# 自动探测最优批次大小 (推荐)
$env:AUTO_BATCH='1'; cargo run -p node-core --example bloom_fair_bench --release
```

#### RocksDB 持久化存储演示 (Phase 4.3)

```powershell
# RocksDB 自适应批量写入基准测试
cargo run -p node-core --example rocksdb_adaptive_batch_bench --release --features rocksdb-storage

# MVCC 自动刷新演示 (时间+区块双触发器)
cargo run -p vm-runtime --example mvcc_auto_flush_demo --release --features rocksdb-storage

# 性能指标收集演示 (Prometheus 格式)
cargo run -p vm-runtime --example metrics_demo --release

# 预期输出:
# - 自适应批量写入: 754K-860K ops/s (远超 200K 目标)
# - MVCC 自动刷新: 每 5 区块或 2 秒触发
# - Metrics: TPS 669, 成功率 98.61%, P50/P90/P99 延迟 <1ms
```

#### 热点调优与基准脚本

- 生成阈值对比报告(Markdown):

  ```powershell
  # 运行多组 Medium/High 阈值,收集 TPS 与 extreme/medium/batch 计数
  powershell -NoProfile -ExecutionPolicy Bypass -File scripts/generate-hotkey-report.ps1 `
    -MediumThresholds 20,40 `
    -HighThresholds 50,120 `
    -DecayPeriod 10 `
    -DecayFactor 0.9 `
    -Batches 2 `
    -Output hotkey-report.md
  ```

- 运行最小分层示例(`lfu_hotkey_demo`):

  ```powershell
  # 可选:设置环境变量以调整阈值与衰减参数
  $env:LFU_MEDIUM=40; $env:LFU_HIGH=120; $env:LFU_DECAY_PERIOD=10; $env:LFU_DECAY_FACTOR=0.9; $env:LFU_BATCHES=3

  # 运行最小示例(输出包含 TPS 与 extreme/medium/batch 计数)
  cargo run -p vm-runtime --release --example lfu_hotkey_demo
  ```

- 自定义基准测试参数(workload + LFU):

  ```powershell
  # 工作负载参数: 线程数、每线程事务数、批次大小
  $env:NUM_THREADS=4; $env:TX_PER_THREAD=100; $env:BATCH_SIZE=10
  
  # LFU 参数: Medium/High 阈值、衰减周期/因子、批次热键阈值、自适应开关
  $env:LFU_MEDIUM=30; $env:LFU_HIGH=80; $env:LFU_DECAY_PERIOD=10; $env:LFU_DECAY_FACTOR=0.9; $env:HOT_KEY_THRESHOLD=5; $env:ADAPTIVE=true

  # 运行完整基准测试
  cargo run --release --bin ownership_sharding_mixed_bench
  ```

> **环境变量完整列表**:
> - **工作负载**: `NUM_THREADS`(默认8)、`TX_PER_THREAD`(默认200)、`BATCH_SIZE`(默认20)
> - **LFU 阈值**: `LFU_MEDIUM`(默认20)、`LFU_HIGH`(默认50)
> - **LFU 衰减**: `LFU_DECAY_PERIOD`(默认10批次)、`LFU_DECAY_FACTOR`(默认0.9)
> - **批次热键**: `HOT_KEY_THRESHOLD`(默认5次访问)
> - **自适应**: `ADAPTIVE`(默认false,设为"1"或"true"启用)

> 更多调优细节与推荐默认值,见 `docs/LFU-HOTKEY-TUNING.md`。

> 架构师可直接使用 `king/*` 分支或 `main` 分支进行内核改动，自动放行；细节见“内核速用指南”。

---

## 🌐 扩展能力与场景支持

> 除内核性能与纯净之外，SuperVM 也面向更完整的链上生态能力：四层神经网络、绝对隐私、跨链兼容、游戏/DeFi 高性能、跨链编译器、多币种 Gas 等。

### 四层神经网络（L1 → L4）
- L1 超算层：高性能数据中心/云节点，负责重任务与聚合
- L2 矿机层：大规模分布式节点，负责执行与存储
- L3 边缘层：贴近用户侧的低延迟接入与加速
- L4 移动层：终端/轻客户端参与、证明与校验
参考：`docs/architecture-2.0.md`

### 绝对隐私（Monero/SNARKs 路线）
- 环签名、隐匿地址、RingCT 金额隐私
- ZK 证明电路实验：Groth16、Halo2（独立实验模块）
- 隐私交易与可验证计算分层接入，不污染内核
参考：`ROADMAP-ZK-Privacy.md`、`halo2-eval/`、`zk-groth16-test/`

### 兼容其它链（插件化）
- EVM 兼容通过“适配器插件”提供（L3 层），零侵入内核
- Solidity → WASM（编译器路线）与 EVM 字节码执行（适配路线）两条路径并存
参考：`docs/evm-adapter-design.md`

### 游戏 & DeFi 高性能支持
- 所有权路由 + MVCC 并行，减少热点冲突与串行瓶颈
- 低延迟路径适配游戏状态同步，高并发撮合/清算支持 DeFi
参考：`docs/scenario-analysis-game-defi.md`

### 跨链编译器（WODA）
- 一次开发，多目标链部署；模型与 ABI 适配在接口层完成
参考：`docs/compiler-and-gas-innovation.md`

### 多币种 Gas 与激励
- 根据资产/场景分层计费；结合四层网络的激励与路由策略
参考：`docs/gas-incentive-mechanism.md`

---

## 功能特性

### ✨ vm-runtime

- **WASM 执行引擎**: 基于 wasmtime 17.0 的高性能 WASM 运行时
- **存储抽象层**: 可插拔的存储后端(trait-based 设计)
- **Host Functions**: 
  - 📦 Storage API: get/set/delete/scan 操作
  - ⛓️ Chain Context API: block_number, timestamp
  - 📣 Event System: emit_event, events_len, read_event
  - 🔐 Crypto API: SHA-256, Keccak-256, ECDSA, Ed25519, 地址派生
- **并行执行引擎**:
  - 🚀 并行交易调度器 (ParallelScheduler)
  - ⚡ 工作窃取调度器 (WorkStealingScheduler)
  - 📦 批量操作优化 (batch_write/read/delete/execute)
  - 🔐 MVCC 多版本并发控制 (MvccStore) - NEW
  - 🔍 冲突检测与依赖分析 (ConflictDetector)
  - 📊 执行统计 (ExecutionStats)
  - 🔄 自动重试机制 (execute_with_retry)
  - 💾 状态快照与回滚 (StateManager)
- **execute_with_context API**: 执行 WASM 函数并返回结果、事件和上下文

### 🚀 node-core

- **CLI 工具**: 带 `--once` 标志支持自动化测试
- **演示程序**: 
  - Demo 1: 简单的 add 函数
  - Demo 2: 完整的事件系统展示(存储 + 事件 + 链上下文)
  - Demo 3: 密码学功能演示 (SHA-256, Keccak-256)
  - Demo 4: 以太坊地址派生
  - Demo 5: 并行执行与冲突检测
  - Demo 6: 状态快照与回滚
  - Demo 7: 工作窃取调度器
  - Demo 8: 批量操作优化
  - Demo 9: MVCC 多版本并发控制
  - Demo 10: MVCC 自动垃圾回收 (NEW 🎉)
- **压力测试与调优** (NEW 🔬):
  - 高并发混合读写测试 (8线程，8000交易)
  - 高冲突热点键测试 (16线程，5个热点键)
  - 内存增长监控测试
  - 长时间稳定性测试 (60秒+)
  - 自适应 GC 行为验证
  - 详细性能报告 (TPS, 延迟, 冲突率)

## 快速开始

### 环境要求

- Rust toolchain (stable) - [安装 rustup](https://rustup.rs/)
- 操作系统: Windows / Linux / macOS

### 运行演示

```powershell
# 运行完整演示(包含事件系统)
cargo run -p node-core

# 运行一次后退出(适合 CI/自动化测试)
cargo run -p node-core -- --once
```

**预期输出:**
```
INFO node_core: Starting node (PoC) with config: config.toml
INFO node_core: Demo 1: add(7,8) => 15
INFO node_core: Demo 2: execute_with_context results:
INFO node_core:   Function returned: 1704079545
INFO node_core:   Block number: 12345, Timestamp: 1704067200
INFO node_core:   Events collected: 2 events
INFO node_core:     Event 1: UserAction
INFO node_core:     Event 2: BlockProcessed
```

### 运行测试

```powershell
# 运行所有测试
cargo test -p vm-runtime

# 运行特定测试
cargo test -p vm-runtime test_execute_with_context
```

**测试覆盖 (64/64 通过):**

**核心功能:**
- ✅ test_memory_storage - 存储实现测试
- ✅ test_execute_add_via_wat - 基础 WASM 执行
- ✅ test_storage - 存储 API 测试
- ✅ test_host_functions - Host 函数调用
- ✅ test_emit_event - 事件发送与读取
- ✅ test_execute_with_context - 完整上下文执行

**密码学功能:**
- ✅ test_sha256 - SHA-256 哈希
- ✅ test_keccak256 - Keccak-256 哈希
- ✅ test_ed25519_verify - Ed25519 签名验证
- ✅ test_secp256k1_verify - ECDSA 签名验证
- ✅ test_derive_eth_address - 以太坊地址派生

**并行执行引擎:**
- ✅ test_read_write_set_conflicts - 读写集冲突检测
- ✅ test_dependency_graph - 依赖图构建
- ✅ test_conflict_detector - 冲突检测器
- ✅ test_snapshot_creation - 快照创建
- ✅ test_rollback - 状态回滚
- ✅ test_nested_snapshots - 嵌套快照
- ✅ test_commit - 快照提交
- ✅ test_execution_stats - 执行统计
- ✅ test_retry_mechanism - 自动重试
- ✅ test_scheduler_with_snapshot - 调度器集成
- ✅ test_work_stealing_basic - 工作窃取基础
- ✅ test_work_stealing_with_priorities - 优先级调度
- ✅ test_work_stealing_with_errors - 错误处理
- ✅ test_batch_write - 批量写入
- ✅ test_batch_read - 批量读取
- ✅ test_batch_delete - 批量删除
- ✅ test_batch_emit_events - 批量事件
- ✅ test_execute_batch - 批量执行
- ✅ test_execute_batch_rollback - 批量回滚

**MVCC 多版本并发控制:**
- ✅ test_mvcc_write_write_conflict - 写写冲突检测
- ✅ test_mvcc_snapshot_isolation_visibility - 快照隔离可见性
- ✅ test_mvcc_version_visibility_multiple_versions - 多版本可见性
- ✅ test_mvcc_concurrent_reads - 并发读取测试
- ✅ test_mvcc_concurrent_writes_different_keys - 不同键并发写
- ✅ test_mvcc_concurrent_writes_same_key_conflicts - 同键冲突检测
- ✅ test_mvcc_read_only_transaction - 只读事务快速路径
- ✅ test_mvcc_read_only_cannot_write - 只读事务写入保护
- ✅ test_mvcc_read_only_cannot_delete - 只读事务删除保护
- ✅ test_mvcc_read_only_performance - 只读性能对比

**MVCC 调度器集成:**
- ✅ test_scheduler_mvcc_basic_commit - MVCC调度器基础提交
- ✅ test_scheduler_mvcc_abort_on_error - MVCC调度器错误回滚
- ✅ test_scheduler_mvcc_read_only_fast_path - MVCC调度器只读路径

**MVCC 垃圾回收:**
- ✅ test_gc_version_cleanup - 版本清理正确性
- ✅ test_gc_preserves_active_transaction_visibility - 保护活跃事务可见性
- ✅ test_gc_no_active_transactions - 无活跃事务时的清理
- ✅ test_gc_multiple_keys - 多键 GC
- ✅ test_gc_stats_accumulation - GC 统计累计

**MVCC 自动垃圾回收 (NEW 🎉):**
- ✅ test_auto_gc_periodic - 周期性自动清理
- ✅ test_auto_gc_threshold - 阈值触发自动清理
- ✅ test_auto_gc_run_on_start - 启动时立即清理
- ✅ test_auto_gc_start_stop - 启动/停止控制
- ✅ test_auto_gc_concurrent_safety - 并发安全性

**MVCC 压力测试 (NEW 🔬):**
- ✅ test_high_concurrency_mixed_workload - 高并发混合读写 (8线程，8000交易)
- ✅ test_high_contention_hotspot - 高冲突热点键 (16线程，5个热点键)
- ✅ test_memory_growth_control - 内存增长监控 (50键，20迭代)
- ✅ test_adaptive_gc - 自适应 GC 验证
- ✅ test_long_running_stability - 长时间稳定性 (60秒+)

**运行压力测试:**
```powershell
# 快速压力测试（排除长时间测试）
cargo test -p vm-runtime --test mvcc_stress_test -- --test-threads=1 --nocapture

# 包括长时间测试
cargo test -p vm-runtime --test mvcc_stress_test -- --test-threads=1 --nocapture --ignored
```

**基准测试:**
```powershell
# 运行性能基准测试
cargo bench --bench parallel_benchmark
```

### 性能摘要 (Criterion)

- 并行调度 get_parallel_batch/100: 平均约 350,045 ns/批
- 冲突检测 non_conflicting/100: 平均约 396,673 ns
- 冲突检测 50% 冲突/100: 平均约 460,675 ns
- 快照创建 create_snapshot/1000: 平均约 224,712 ns
- 依赖图 build_and_query/100: 平均约 344,862 ns

说明:
- 单位为 ns/iter（Criterion 默认），不同机器的绝对值会有差异，请以相对对比为主。
- 完整 HTML 报告路径: target/criterion/report/index.html

## 使用示例

### 基础 WASM 执行

```rust
use vm_runtime::{Runtime, MemoryStorage};

let runtime = Runtime::new(MemoryStorage::new());
let wat = r#"
(module
  (func $add (export "add") (param i32 i32) (result i32)
    local.get 0
    local.get 1
    i32.add)
)
"#;
let wasm = wat::parse_str(wat)?;
let result = runtime.execute_add(&wasm, 7, 8)?;
assert_eq!(result, 15);
```

### 并行执行与状态管理

```rust
use vm_runtime::{ParallelScheduler, ExecutionStats};

// 创建并行调度器
let scheduler = ParallelScheduler::new();

// 使用快照保护执行交易
let result = scheduler.execute_with_snapshot(|manager| {
    let storage = manager.get_storage();
    let mut storage = storage.lock().unwrap();
    storage.insert(b"balance".to_vec(), b"100".to_vec());
    Ok(()) // 成功则提交
})?;

// 使用自动重试机制
let result = scheduler.execute_with_retry(
    |manager| {
        // 可能失败的操作
        Ok(42)
    },
    max_retries: 3
)?;

// 获取执行统计
let stats = scheduler.get_stats();
println!("成功率: {:.2}%", stats.success_rate() * 100.0);
println!("重试次数: {}", stats.retry_count);
```

### 基于 MVCC 的并行调度器 (v0.9.0 NEW 🎯)

无需手动冲突检测与快照管理，使用 MVCC 原生事务隔离与写写冲突检测，支持自动重试与批量操作。

```rust
use vm_runtime::{MvccScheduler, MvccSchedulerConfig};
use anyhow::Result;

// 创建带自适应 GC 的 MVCC 调度器
let scheduler = MvccScheduler::new_with_config(MvccSchedulerConfig::default());

// 执行单个事务（自动重试）
let result = scheduler.execute_txn(1, |txn| {
    txn.write(b"key".to_vec(), b"value".to_vec());
    Ok(42)
});
assert!(result.success);

// 并行批量事务
let txns: Vec<_> = (0..8)
    .map(|i| (i as u64, |txn: &mut vm_runtime::Txn| -> Result<i32> {
        let key = format!("k{}", i).into_bytes();
        txn.write(key, b"v".to_vec());
        Ok(i as i32)
    }))
    .collect();

let batch = scheduler.execute_batch(txns);
println!("successful={}, failed={}, conflicts={}", batch.successful, batch.failed, batch.conflicts);

// 快照只读
let value = scheduler.read_only(|txn| Ok(txn.read(b"key").map(|v| v.to_vec())) )?;
assert_eq!(value, Some(b"value".to_vec()));

// 批量写/读/删
let ts = scheduler.batch_write(vec![(b"a".to_vec(), b"1".to_vec())])?;
let vals = scheduler.batch_read(&[b"a".to_vec()]);
let _ = scheduler.batch_delete(vec![b"a".to_vec()])?;
```

### 工作窃取调度器

```rust
use vm_runtime::{WorkStealingScheduler, Task};

// 创建工作窃取调度器 (4 个工作线程)
let scheduler = WorkStealingScheduler::new(Some(4));

// 提交任务 (支持优先级)
let tasks = vec![
    Task::new(1, 255),  // 高优先级
    Task::new(2, 128),  // 中优先级
    Task::new(3, 50),   // 低优先级
];
scheduler.submit_tasks(tasks);

// 并行执行所有任务
let result = scheduler.execute_all(|tx_id| {
    println!("Processing transaction {}", tx_id);
    Ok(())
})?;

// 获取统计信息
let stats = scheduler.get_stats();
println!("成功: {}, 失败: {}", stats.successful_txs, stats.failed_txs);
```

### 批量操作

```rust
use vm_runtime::ParallelScheduler;

let scheduler = ParallelScheduler::new();

// 批量写入 (减少锁争用)
let writes = vec![
    (b"key1".to_vec(), b"value1".to_vec()),
    (b"key2".to_vec(), b"value2".to_vec()),
    (b"key3".to_vec(), b"value3".to_vec()),
];
scheduler.batch_write(writes)?;

// 批量读取
let keys = vec![b"key1".to_vec(), b"key2".to_vec()];
let results = scheduler.batch_read(&keys)?;

// 批量执行交易 (原子性: 全部成功或全部回滚)
let operations = vec![
    Box::new(|manager| { /* 交易 1 */ Ok(1) }),
    Box::new(|manager| { /* 交易 2 */ Ok(2) }),
    Box::new(|manager| { /* 交易 3 */ Ok(3) }),
];
let results = scheduler.execute_batch(operations)?;
```

### MVCC 多版本并发控制

```rust
use vm_runtime::MvccStore;

let store = MvccStore::new();

// 事务 1：写入并提交
let mut t1 = store.begin();
t1.write(b"balance".to_vec(), b"100".to_vec());
let ts1 = t1.commit()?;

// 事务 2：快照隔离读取
let t2 = store.begin();
assert_eq!(t2.read(b"balance").as_deref(), Some(b"100".as_ref()));

// 并发更新同一键会触发写写冲突检测
let mut t3 = store.begin();
let mut t4 = store.begin();
t3.write(b"balance".to_vec(), b"200".to_vec());
t4.write(b"balance".to_vec(), b"300".to_vec());

// 第一个提交成功

---

## License

本项目自有代码采用 GPL-3.0-or-later 许可协议发布。详情参见仓库根目录的 `LICENSE` 文件。

第三方组件说明：
- `solana/` 目录为第三方参考代码，不属于本项目的一部分，保持其原有许可证约束（Apache-2.0，见 `solana/LICENSE`）。本项目的构建与发布不会包含该目录。
t3.commit()?;
// 第二个提交失败（写写冲突）
assert!(t4.commit().is_err());
```

**优化特性**:
- ✅ 每键粒度读写锁 (RwLock)，允许并发读取
- ✅ DashMap 无锁哈希表，降低全局锁竞争
- ✅ 原子时间戳 (AtomicU64)，消除时间戳分配瓶颈
- ✅ 提交时按键排序加锁，避免死锁
- ✅ 快照隔离 (Snapshot Isolation) 语义
- ✅ 垃圾回收 (GC)：自动清理旧版本，控制内存增长

**垃圾回收 (v0.6.0 NEW)**:
```rust
use vm_runtime::{MvccStore, GcConfig};

// 创建带 GC 配置的 MVCC 存储
let config = GcConfig {
    max_versions_per_key: 10,      // 每个键最多保留 10 个版本
    enable_time_based_gc: false,   // 暂不启用基于时间的 GC
    version_ttl_secs: 3600,        // 版本过期时间（秒）
};
let store = MvccStore::new_with_config(config);

// ... 执行一些事务 ...

// 手动触发 GC
let cleaned = store.gc()?;
println!("清理了 {} 个旧版本", cleaned);

// 获取 GC 统计
let stats = store.get_gc_stats();
println!("GC 执行次数: {}", stats.gc_count);
println!("总清理版本数: {}", stats.versions_cleaned);

// 监控存储状态
println!("当前总版本数: {}", store.total_versions());
println!("当前键数量: {}", store.total_keys());
```

**GC 策略**:
- 保留每个键的最新版本（无论配置如何）
- 保留所有活跃事务可见的版本（基于水位线）
- 根据 `max_versions_per_key` 限制清理超量版本
- 自动跟踪活跃事务，防止清理仍在使用的版本

**自动 GC (v0.7.0 NEW 🎉)**:
```rust
use vm_runtime::{MvccStore, GcConfig, AutoGcConfig};
use std::sync::Arc;

// 创建启用自动 GC 的 MVCC 存储
let config = GcConfig {
    max_versions_per_key: 10,
    enable_time_based_gc: false,
    version_ttl_secs: 3600,
    auto_gc: Some(AutoGcConfig {
        interval_secs: 60,            // 每 60 秒执行一次 GC
        version_threshold: 1000,      // 当总版本数超过 1000 时触发
        run_on_start: false,          // 启动时不立即运行
    }),
};
let store = Arc::new(MvccStore::new_with_config(config));

// 自动 GC 后台线程已启动，无需手动调用 gc()

// 动态控制自动 GC
store.stop_auto_gc();                // 停止自动 GC
store.start_auto_gc();               // 重新启动自动 GC
assert!(store.is_auto_gc_running()); // 检查运行状态

// 更新自动 GC 配置（运行时动态调整）
store.update_auto_gc_config(Some(AutoGcConfig {
    interval_secs: 30,      // 改为 30 秒
    version_threshold: 500, // 降低阈值
    run_on_start: false,
}));

// Drop 时会自动停止 GC 线程并等待退出
```

**自动 GC 触发策略**:
- **周期性触发**: 每隔 `interval_secs` 秒执行一次 GC
- **阈值触发**: 当总版本数 ≥ `version_threshold` 时立即触发（如果配置了阈值）
- **启动触发**: 如果 `run_on_start = true`，启动时立即执行一次
- **优雅停止**: Drop 时自动停止后台线程，最多等待 2 秒

**性能影响** (基准测试):
- 写入开销: 自动 GC 对写入操作的影响 < 5%
- 读取开销: 对读取操作无明显影响
- 后台线程: 采用可中断休眠 (100ms 粒度)，响应快速

**自适应 GC (v0.8.0 NEW 🎯)**:
```rust
use vm_runtime::{MvccStore, GcConfig, AutoGcConfig};

// 启用自适应 GC，根据负载自动调整参数
let config = GcConfig {
    max_versions_per_key: 20,
    enable_time_based_gc: false,
    version_ttl_secs: 3600,
    auto_gc: Some(AutoGcConfig {
        interval_secs: 60,          // 基准间隔
        version_threshold: 1000,    // 基准阈值
        run_on_start: false,
        enable_adaptive: true,      // 🎯 启用自适应模式
    }),
};
let store = Arc::new(MvccStore::new_with_config(config));

// 自适应 GC 会根据负载自动调整：
// - 高负载时：缩短间隔（最小 10秒），降低阈值（最小 500）
// - 低效 GC：延长间隔（最大 300秒），提高阈值（最大 5000）
// - 正常负载：逐渐回归基准值
```

**自适应策略**:
- **高负载检测** (TPS 激增或版本快速增长):
  - 缩短 GC 间隔 (基准 60s → 最小 10s)
  - 降低触发阈值 (基准 1000 → 最小 500)
  - 更频繁、更激进的 GC
- **低效 GC 检测** (清理率 < 10%):
  - 延长 GC 间隔 (基准 60s → 最大 300s)
  - 提高触发阈值 (基准 1000 → 最大 5000)
  - 减少无效 GC，节省资源

**运行时观测** (v0.8.0+):
```rust
// 实时查看当前 GC 参数（包括自适应调整后的值）
if let Some(runtime) = store.get_auto_gc_runtime() {
    println!("自适应模式: {}", runtime.enable_adaptive);
    println!("当前间隔: {}s", runtime.interval_secs);
    println!("当前阈值: {}", runtime.version_threshold);
}

// 结合 GC 统计评估效果
let stats = store.get_gc_stats();
println!("GC 执行次数: {}", stats.gc_count);
println!("清理版本数: {}", stats.versions_cleaned);
```

> 📖 详细说明请参考: [GC 运行时可观测性文档](docs/gc-observability.md)
- **正常负载**:
  - 逐渐回归基准值
  - 保持稳定状态

**压力测试与调优指南**: 查看 [docs/stress-testing-guide.md](docs/stress-testing-guide.md)

### 使用事件系统

```rust
use vm_runtime::{Runtime, MemoryStorage};

let runtime = Runtime::new(MemoryStorage::new());
let wat = r#"
(module
  (import "chain_api" "emit_event" (func $emit_event (param i32 i32) (result i32)))
  (memory (export "memory") 1)
  (data (i32.const 0) "Hello, World!")
  
  (func (export "greet") (result i32)
    i32.const 0
    i32.const 13
    call $emit_event
    drop
    i32.const 42
  )
)
"#;
let wasm = wat::parse_str(wat)?;
let (result, events, block_num, timestamp) = runtime.execute_with_context(
    &wasm,
    "greet",
    12345,  // block_number
    1704067200  // timestamp
)?;

assert_eq!(result, 42);
assert_eq!(events.len(), 1);
assert_eq!(events[0], b"Hello, World!");
```

### 自定义存储后端

```rust
use vm_runtime::Storage;
use anyhow::Result;

struct MyStorage {
    // your implementation
}

impl Storage for MyStorage {
    fn get(&self, key: &[u8]) -> Result<Option<Vec<u8>>> {
        // your logic
    }
    
    fn set(&mut self, key: &[u8], value: &[u8]) -> Result<()> {
        // your logic
    }
    
    fn delete(&mut self, key: &[u8]) -> Result<()> {
        // your logic
    }
    
    fn scan(&self, prefix: &[u8], limit: usize) -> Result<Vec<(Vec<u8>, Vec<u8>)>> {
        // your logic
    }
}

let runtime = Runtime::new(MyStorage::new());
```

## Host Functions 参考

### Storage API (`storage_api`)

| 函数 | 签名 | 说明 |
|------|------|------|
| `storage_get` | `(key_ptr: i32, key_len: i32) -> i64` | 读取键值,返回长度(缓存到 last_get) |
| `storage_read_value` | `(ptr: i32, len: i32) -> i32` | 从缓存读取值到内存 |
| `storage_set` | `(key_ptr: i32, key_len: i32, value_ptr: i32, value_len: i32) -> i32` | 写入键值对 |
| `storage_delete` | `(key_ptr: i32, key_len: i32) -> i32` | 删除键 |

### Chain API (`chain_api`)

| 函数 | 签名 | 说明 |
|------|------|------|
| `block_number` | `() -> i64` | 获取当前区块号 |
| `timestamp` | `() -> i64` | 获取当前时间戳 |
| `emit_event` | `(data_ptr: i32, data_len: i32) -> i32` | 发送事件 |
| `events_len` | `() -> i32` | 获取事件总数 |
| `read_event` | `(index: i32, ptr: i32, len: i32) -> i32` | 读取指定事件 |

## 项目结构

```
SuperVM/
├── src/
│   ├── vm-runtime/          # WASM 运行时核心
│   │   ├── src/
│   │   │   ├── lib.rs           # 公共 API
│   │   │   ├── runtime.rs       # 核心运行时 (L0)
│   │   │   ├── wasm_executor.rs # WASM 执行器 (L0)
│   │   │   ├── storage.rs       # 存储抽象 (L0)
│   │   │   ├── parallel/        # 并行调度器 (L0)
│   │   │   ├── mvcc/            # MVCC 引擎 (L0)
│   │   │   ├── ownership.rs     # 所有权扩展 (L1)
│   │   │   ├── supervm.rs       # 高级 API (L1)
│   │   │   └── execution_trait.rs # 执行引擎 trait (L1)
│   │   └── Cargo.toml
│   └── node-core/           # CLI 演示程序 (L4)
│       ├── src/
│       │   └── main.rs
│       └── Cargo.toml
├── privacy-test/            # 隐私密码学实验 (L3)
│   ├── src/
│   │   ├── simple_ring_signature.rs
│   │   ├── pedersen_commitment.rs
│   │   └── hash_to_point.rs
│   └── Cargo.toml
├── halo2-eval/              # Halo2 ZK 评估 (L3)
│   ├── src/
│   └── Cargo.toml
├── zk-groth16-test/         # Groth16 ZK 实验 (L3)
│   ├── src/
│   ├── benches/
│   └── Cargo.toml
├── examples/                # 示例程序 (L4)
│   ├── ownership_demo.rs
│   ├── supervm_routing_demo.rs
│   ├── routed_batch_demo.rs
│   └── tps_compare_demo.rs
├── scripts/                 # 开发与部署脚本
│   ├── install-git-hooks.ps1
│   ├── pre-commit-hook.sh
│   └── verify-kernel-purity.sh
├── .github/                 # GitHub 配置
│   ├── MAINTAINERS          # 维护者白名单
│   ├── workflows/
│   │   └── kernel-purity-check.yml
│   └── ISSUE_TEMPLATE/
├── docs/
│   ├── KERNEL-DEFINITION.md     # 内核保护定义 (600+ 行)
│   ├── KERNEL-QUICK-START.md    # 架构师快速上手
│   ├── KERNEL-QUICK-REFERENCE.md # 开发者参考卡
│   ├── KERNEL-MODULES-VERSIONS.md # 模块分级 (L0-L4)
│   ├── architecture-2.0.md      # 完整架构文档
│   ├── evm-adapter-design.md    # EVM 适配器设计
│   ├── parallel-execution.md    # 并行执行机制
│   ├── gas-incentive-mechanism.md # GAS 激励机制
│   ├── scenario-analysis-game-defi.md # 游戏/DeFi 场景分析
│   └── plans/
│       ├── phase2-privacy-layer.md
│       └── vm-runtime-extension.md
├── CHANGELOG.md             # 更新日志
├── ROADMAP.md               # 主开发路线图
├── ROADMAP-ZK-Privacy.md    # 隐私专项路线图
└── Cargo.toml               # Workspace 配置
```

## 架构设计

```
┌─────────────────────────────────────────────┐
│             node-core (CLI)                 │
│  ┌──────────────────────────────────────┐   │
│  │  Demo 1: Basic execution             │   │
│  │  Demo 2: Events + Storage + Context  │   │
│  └────────────┬─────────────────────────┘   │
└───────────────┼─────────────────────────────┘
                │
                ▼
┌───────────────────────────────────────────────────────────────┐
│                    vm-runtime Crate                           │
│  ┌──────────────────────────────────────────────────────────┐ │
│  │  L4 应用层 (Application Layer)                          │ │
│  │  - Cross-Chain Compiler (跨链编译器)                    │ │
│  │  - DApps (游戏/DeFi/NFT 去中心化应用)                   │ │
│  │  - node-core CLI                                         │ │
│  │  - examples/ 示例程序                                    │ │
│  └──────────────────────┬───────────────────────────────────┘ │
│                         │                                      │
│  ┌──────────────────────▼───────────────────────────────────┐ │
│  │  L3 插件层 (Plugin Layer)                               │ │
│  │  - EVM Adapter (solidity → WASM)                        │ │
│  │  - Neural Network Engine (神经网络推理引擎)            │ │
│  │  - privacy-test (RingCT, Pedersen)                      │ │
│  │  - zk-groth16-test, halo2-eval                          │ │
│  └──────────────────────┬───────────────────────────────────┘ │
│                         │                                      │
│  ┌──────────────────────▼───────────────────────────────────┐ │
│  │  L1 扩展层 (Extension Layer) - 桥接 L0 与 L2           │ │
│  │  - ownership.rs (所有权管理)                            │ │
│  │  - supervm.rs (高级 API)                                │ │
│  │  - execution_trait.rs (统一执行引擎接口) ✅            │ │
│  │    ↑ 向下封装 L0 核心 | 向上服务 L2 适配器 ↓           │ │
│  └──────────────────────┬───────────────────────────────────┘ │
│                         │                                      │
│  ┌──────────────────────▼───────────────────────────────────┐ │
│  │  L0 核心内核 (Core Kernel) - 受保护                     │ │
│  │  ┌────────────────────────────────────────────────────┐  │ │
│  │  │  ParallelScheduler (并行调度器)                   │  │ │
│  │  │  - WorkStealingScheduler 工作窃取                 │  │ │
│  │  │  - 依赖检测与冲突解析                             │  │ │
│  │  └────────────┬───────────────────────────────────────┘  │ │
│  │               │                                            │ │
│  │  ┌────────────▼───────────────────────────────────────┐  │ │
│  │  │  MVCC Engine (多版本并发控制)                     │  │ │
│  │  │  - MvccStore (每键版本链)                         │  │ │
│  │  │  - 快照隔离 (Snapshot Isolation)                  │  │ │
│  │  │  - GC (垃圾回收) + Observability                  │  │ │
│  │  └────────────┬───────────────────────────────────────┘  │ │
│  │               │                                            │ │
│  │  ┌────────────▼───────────────────────────────────────┐  │ │
│  │  │  Runtime<S: Storage>                               │  │ │
│  │  │  - WasmExecutor (WASM 执行器)                      │  │ │
│  │  │  - Storage Trait (存储抽象)                        │  │ │
│  │  │  - Host Functions (链上下文/密码学/事件)           │  │ │
│  │  └────────────┬───────────────────────────────────────┘  │ │
│  └───────────────┼────────────────────────────────────────────┘ │
└──────────────────┼──────────────────────────────────────────────┘
                   │
                   ▼
        ┌──────────────────┐
        │  wasmtime 17.0   │
        │  WASM JIT Engine │
        └──────────────────┘
```

## 性能特性

### 🏆 核心性能指标

- **并行TPS**: 187,000+ TPS (低冲突场景) | 85,000+ TPS (高冲突场景)
- **内核保护**: 零侵入式内核隔离 (L0/L1 分级保护)
- **并发模型**: MVCC 快照隔离 + 每键粒度版本链
- **调度器**: Work-Stealing 工作窃取 + 自动依赖检测

### ⚡ 性能优化技术

- **Zero-copy 设计**: 指针传递避免内存复制,存储层直接引用
- **MVCC 多版本控制**: 
  - 每键独立版本链 (DashMap),读写并发无阻塞
  - 快照隔离 (Snapshot Isolation),事务级一致性保证
  - 自适应 GC,后台垃圾回收过期版本
- **并行调度优化**:
  - 工作窃取调度器 (WorkStealingScheduler),自动负载均衡
  - 依赖检测与冲突解析,最大化并行度
  - 批量操作优化 (batch_write/read/execute),减少系统调用
- **JIT 编译加速**: wasmtime 17.0 实时编译优化,接近原生代码性能

### 🔒 安全特性

- **Rust 内存安全**: 编译期所有权检查,零成本抽象
- **WASM 沙箱隔离**: 字节码验证 + 线性内存隔离,恶意合约无法逃逸
- **内核保护机制**:
  - L0 核心内核只读保护 (CI + pre-commit hook)
  - 5 种覆盖方法 (环境变量/Git 配置/上帝分支/标签/文件)
  - 维护者白名单验证 (`.github/MAINTAINERS`)

### 📦 架构特性

- **模块化设计**: L0-L4 分层架构,可插拔存储/调度/执行引擎
- **跨链兼容**: EVM Adapter 支持 Solidity → WASM 编译,兼容以太坊生态
- **可观测性**: 完整 GC 可视化 + 执行统计,实时监控系统状态

提示: 完整性能报告请查看 `BENCHMARK_RESULTS.md`,或运行 `cargo bench` 查看本地基准测试 (`target/criterion/report/index.html`)。详细压测指南见 [`docs/stress-testing-guide.md`](docs/stress-testing-guide.md)。

## 开发状态

当前版本: **v0.5.0** (活跃开发)

**已完成 ✅:**
- ✅ 基础 WASM 执行引擎
- ✅ 存储抽象与实现
- ✅ Host Functions (存储 + 链上下文 + 事件 + 密码学)
- ✅ execute_with_context API
- ✅ 并行执行引擎
    - ✅ 冲突检测与依赖分析
    - ✅ 状态快照与回滚
    - ✅ 执行统计与监控
    - ✅ 自动重试机制
    - ✅ 工作窃取调度器
    - ✅ 批量操作优化（batch_write/read/delete/execute）
    - ✅ MVCC 多版本并发控制（每键粒度读写锁 + DashMap）
- ✅ 完整单元测试覆盖 (47 个测试)
- ✅ 性能基准测试框架（Criterion）

**进行中 🚧:**
- 🚧 性能基准测试报告总结与文档化
- 🚧 MVCC 与 ParallelScheduler 集成

**计划中 📋:**
- 📋 编译器集成 (Solidity/AssemblyScript)
- 📋 EVM 兼容层
- 📋 乐观并发控制（OCC）
- 📋 生产环境部署

详见 [CHANGELOG.md](CHANGELOG.md) 和 [ROADMAP.md](ROADMAP.md)。

## 文档资源

- 📖 [API 文档](docs/API.md) - Host Functions API 参考
- 📖 [并行执行设计](docs/parallel-execution.md) - 并行调度器与冲突检测
- 📖 [压力测试与调优指南](docs/stress-testing-guide.md) - MVCC 压力测试与自适应 GC (v0.8.0)
- 📖 [GC 运行时可观测性](docs/gc-observability.md) - 实时监控 GC 参数 (v0.8.0)
- 📖 [游戏与 DeFi 场景分析](docs/scenario-analysis-game-defi.md) - 面向业务场景的性能路径
- 📖 [跨链编译器与多币种 Gas](docs/compiler-and-gas-innovation.md) - 编译与计费创新
- 📖 [Gas 激励机制](docs/gas-incentive-mechanism.md) - 四层网络下的激励设计
- 📖 [ZK 隐私专项计划](ROADMAP-ZK-Privacy.md) - 隐私路线与里程碑
- 📖 [内核速用指南](docs/KERNEL-QUICK-START.md) - 架构师/Owner 上帝分支与覆盖
- 📖 [内核定义与保护](docs/KERNEL-DEFINITION.md) - L0/L1/L2/L3/L4 规则
- 📖 [模块分级与版本索引](docs/KERNEL-MODULES-VERSIONS.md) - 模块层级与版本策略
- 📖 [EVM 适配器设计](docs/evm-adapter-design.md) - 插件化零入侵方案
- 📖 [变更日志](CHANGELOG.md) - 版本历史与更新
- 📖 [贡献指南](CONTRIBUTING.md) - 如何参与开发
- 📖 [开发者文档](DEVELOPER.md) - 开发流程与规范
- 📖 [项目路线图](ROADMAP.md) - 未来规划与进展

## 贡献指南

欢迎贡献!请参阅 [CONTRIBUTING.md](CONTRIBUTING.md)。

## 许可证

本项目自有代码采用 GPL-3.0-or-later 许可协议发布，详见根目录 `LICENSE`。

第三方组件说明：
- `solana/` 目录为第三方参考代码，不属于本项目的一部分，保持其原有许可证约束（Apache-2.0，见 `solana/LICENSE`）。本项目的构建与发布不会包含该目录。

## 联系方式

- 开发者: Rainbow Haruko / king
- Email: iscrbank@gmail.com / leadbrand@me.com
- 问题反馈: [GitHub Issues](https://github.com/XujueKing/SuperVM/issues)
