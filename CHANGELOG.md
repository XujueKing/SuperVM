# Changelog

All notable changes to SuperVM will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### [WHITEPAPER V1.0] 白皮书发布与内容营销素材 (2025-11-XX)

**Summary:**
- **专业白皮书创作**:
  - 中英文双语白皮书 (WHITEPAPER.md + WHITEPAPER_EN.md)
  - 神经网络生物学类比贯穿全文 (感知/自主/协作)
  - 四大创新: 242K TPS / 多链融合 / 内置隐私 / 自组织通信
  - 三大革命性场景: 灾难应急 / 审查抵抗 / 普惠金融
- **多渠道营销素材**:
  - 社交媒体模板 (Twitter/Medium/Reddit/LinkedIn/YouTube)
  - 投资者 Pitch Deck (18 页专业演示)
  - PDF 生成指南 (Pandoc 自动化)
  - 视觉资产指南 (Mermaid/Graphviz/Chart.js)

**Files Added:**
- `WHITEPAPER.md`: 中文白皮书 (~1000 行)
  - 核心定位: Web3 操作系统 (非跨链桥)
  - 技术数据: 242K TPS, 99.3% Gas 减少, $2B 桥被盗对比
  - 经济模型: 1B 供应量, 50% Gas 燃烧, 8-12% 质押 APY
  - 路线图: 2024-2026 分阶段实施
- `WHITEPAPER_EN.md`: 英文白皮书 (~800 行)
  - 完整翻译中文版本
  - 适配国际受众 (idioms/metaphors 本地化)
- `docs/SOCIAL-MEDIA-TEMPLATES.md`: 社交媒体发布素材
  - Twitter/X Thread (10 条推文串)
  - Medium 长文章模板 (2000+ 字)
  - Reddit 发布 (r/CryptoCurrency + r/ethereum)
  - Discord/LinkedIn/YouTube 脚本
  - 数据可视化建议 + 发布检查清单
- `docs/INVESTOR-PITCH-DECK.md`: 投资者演示文稿 (18 页)
  - Slide 1-3: 问题/愿景/解决方案
  - Slide 4-6: 架构/多链融合/市场机会
  - Slide 7-9: 竞争格局/商业模式/代币经济
  - Slide 10-13: 路线图/团队/增长数据/场景
  - Slide 14-16: 融资需求 ($5M Seed) / 风险 / 时机
  - Slide 17-18: 总结 + Appendix (技术细节)
- `docs/PDF-GENERATION-GUIDE.md`: PDF 生成完整指南
  - Pandoc 安装与配置 (Windows/macOS/Linux)
  - 中英文白皮书转换命令
  - 专业版模板 (封面/页眉页脚/水印)
  - 自动化脚本 (PowerShell + Bash)
  - 质量检查清单
- `docs/VISUAL-ASSETS-GUIDE.md`: 视觉资产创作指南
  - Mermaid 架构图 (四层神经网络)
  - Graphviz 网络拓扑图 (自组织通信)
  - Chart.js 性能对比图 (TPS/Gas 费用)
  - Python 可视化脚本 (代币分配饼图)
  - ASCII 灾难场景示意图
  - 品牌色彩规范 + 设计工具推荐

**Documentation Updates:**
- `docs/INDEX.md`: 新增 "白皮书与宣传材料" 章节
  - 链接到 6 个新文档 (白皮书/社交/Pitch/PDF/视觉)
- `README.md`: 添加白皮书导航链接 (已在之前更新)

**Content Highlights:**
- **神经网络类比**: L1=大脑皮层, L2=脊髓, L3=神经节, L4=感觉神经元
- **自愈合能力**: 3 秒重连, 30 秒 Mesh 切换, 72 小时离线容忍
- **市场定位**: $85B TAM (多链基础设施), 99.3% Gas 成本优势
- **融资目标**: $5M Seed, $20M 估值, 18 个月达到主网

**Next Steps (营销执行)**:
- [ ] 生成所有 PDF 版本 (运行 `scripts/generate-pdfs.ps1`)
- [ ] 创建视觉资产 (运行 `scripts/generate-visuals.ps1`)
- [ ] 发布社交媒体 Thread (Twitter/X)
- [ ] 投递 Medium/CoinDesk/The Block
- [ ] 联系 KOL/influencer 预热
- [ ] 安排 AMA (Ask Me Anything) 时间

### [PHASE 9.5] 原生监控客户端规划 - DRAFT (2025-11-10)

**Summary:**
- **零依赖监控解决方案**:
  - 使用 egui (纯 Rust GUI 框架) 开发跨平台原生客户端
  - 替代 Grafana + Prometheus 浏览器方案
  - 单一可执行文件, 无需 Docker/Node.js/浏览器
  - 目标性能: < 50MB 内存, < 5% CPU, < 500ms 启动时间

**Files Added:**
- `docs/NATIVE-MONITOR-DESIGN.md`: 原生监控客户端完整技术方案
  - GUI 框架选型 (egui vs Tauri)
  - 系统架构设计 (UI/数据采集/存储/通信)
  - 实施路径 (5个阶段, 共7周)
  - UI/UX 设计原则 (类 VS Code 风格)

**Documentation Updates:**
- `ROADMAP.md`: 新增 Phase 9.5 (原生监控客户端, 7周)
  - M1: MVP 基础 (2周) - 基础 Dashboard + /metrics 拉取
  - M2: 实时图表与本地存储 (2周) - egui_plot + RocksDB 时序存储
  - M3: 节点管理与多连接 (1周) - 多节点支持
  - M4: 告警引擎与通知 (1周) - 规则引擎 + 系统通知
  - M5: 跨平台打包与优化 (1周) - Windows/Linux/macOS 打包
- `docs/INDEX.md`: 新增 `NATIVE-MONITOR-DESIGN.md` 链接

**Next Steps (Phase 9.5 M1)**:
- [ ] 创建 `native-monitor/` crate
- [ ] 搭建 egui + eframe 项目结构
- [ ] 实现 HTTP 客户端拉取 /metrics
- [ ] 开发基础 Dashboard UI (TPS/Latency/Success Rate)

### [PHASE 10 M1] 插件规范 v0 发布 - DRAFT (2025-11-10)

**Summary:**
- **插件系统规范草案**:
  - 定义热插拔子模块/插件接口规范（Native ABI + gRPC 双模式）
  - 支持原链节点（Bitcoin Core/Geth/Solana）作为可插拔子模块运行
  - 提供三级运行策略（Strict/Permissive/Dev）与沙箱隔离机制
  - 统一 IR 镜像层（TxIR/BlockIR/StateIR）用于跨链状态查询

**Files Added:**
- `docs/plugins/README.md`: 插件架构总览与快速开始指南
- `docs/plugins/PLUGIN-SPEC.md`: 插件规范草案（生命周期/ABI/安全策略）
- `docs/plugins/example-plugin.yaml`: 插件清单示例（Bitcoin 子模块）
- `proto/plugin_host.proto`: gRPC 数据平面与控制 RPC 定义（Register/StreamBlocks/SubmitTx/Health）
- `sdk/plugin-sdk-rs/README.md`: Rust SDK 占位说明

**Documentation Updates:**
- `docs/INDEX.md`: 新增 `🔌 plugins/` 插件系统规范章节
- `ROADMAP.md Phase 10 M1`: 标记插件规范 v0 相关交付物（已完成 4/6 项）

**Next Steps (Pending):**
- [ ] 补全 `docs/plugins/plugin-manifest.schema.json` (JSON Schema 校验)
- [ ] 添加 `docs/plugins/submodule-adapter.md` (SubmoduleAdapter trait 详细说明)
- [ ] 生成 Rust protobuf 绑定并集成到 SDK
- [ ] 创建 Native Plugin 与 gRPC Plugin 的完整示例代码

### [PHASE 2.3] RingCT 并行证明与批量验证 - VERIFIED (2025-01-XX)

**Summary:**
- **RingCT 并行证明优化**:
  - 实现全局 ProvingKey 单例缓存(once_cell),消除重复setup开销(节省1-2秒/实例)
  - 添加 RingCtParallelProver 支持真实 MultiUTXORingCTCircuit witness
  - 创建 zk_parallel_http_bench.rs HTTP基准测试(端口9090,端点/metrics和/summary)
  - 新增 vm_privacy_zk_parallel_* 系列指标: proof_total/failed, batches_total, latency_ms, tps
- **批量验证模块**:
  - 新增 batch_verifier.rs 支持并行验证多个Groth16证明
  - 实现 PreparedVerifyingKey 优化验证性能
  - 验证性能提升8倍: 13.1 → 104.6 verifications/sec (32批次)
- **Fast→Consensus Fallback**:
  - 添加环境变量控制: SUPERVM_ENABLE_FAST_FALLBACK, SUPERVM_FALLBACK_ON_ERRORS
  - SuperVM::with_fallback() 方法支持可配置错误白名单
  - 新增 vm_fast_fallback_total 指标记录回退次数
- **Grafana 监控集成**:
  - 创建 prometheus-ringct.yml 配置文件(抓取:9090/metrics)
  - grafana-ringct-dashboard.json 仪表板模板(7个面板)
  - GRAFANA-RINGCT-PANELS.md 详细面板配置文档
  - GRAFANA-QUICK-DEPLOY.md 快速部署指南(Windows)
  - prometheus-zk-alerts.yml 3条告警规则(失败率/TPS/延迟)

**Performance Baseline** (Release mode, Windows, BLS12-381):
- **RingCT Proving**: 50.8 proofs/sec (批次32, 平均19.7ms/proof, 100%成功率)
- **Batch Verification**: 104.6 verifications/sec (8x faster than individual)
- **峰值TPS**: 53.01 proofs/sec, 最佳延迟: 18.86ms/proof

**Verification Results:**
- ✅ **所有测试通过**: parallel_prover (3/3), batch_verifier (3/3), fallback (2/2)
- ✅ **代码质量**: cargo fix清理所有unused imports/variables,零警告
- ✅ **HTTP基准**: 832+ proofs generated, 26+ batches, 0 failures
- ✅ **Prometheus集成**: /metrics端点输出23个指标(MVCC/RingCT/路由)

**Files Changed:**
- `src/vm-runtime/src/privacy/parallel_prover.rs`: 全局ProvingKey缓存, RingCtParallelProver
- `src/vm-runtime/src/privacy/batch_verifier.rs`: 新增批量验证模块
- `src/vm-runtime/src/metrics.rs`: record_parallel_batch(), inc_fast_fallback()
- `src/vm-runtime/src/supervm.rs`: with_fallback(), from_env(), 回退逻辑
- `src/vm-runtime/examples/zk_parallel_http_bench.rs`: 新增HTTP基准测试
- `src/vm-runtime/tests/fallback_tests.rs`: 新增2个回退行为测试
- `docs/GRAFANA-RINGCT-PANELS.md`: 新增Grafana面板配置
- `docs/GRAFANA-QUICK-DEPLOY.md`: 新增快速部署指南
- `docs/RINGCT-PERFORMANCE-BASELINE.md`: 新增性能基准报告
- `prometheus-ringct.yml`: Prometheus抓取配置
- `grafana-ringct-dashboard.json`: Grafana仪表板JSON

**Dependencies:**
- 添加 `once_cell = "1.20"` 用于全局ProvingKey缓存

**Risk Assessment:** LOW
- 所有更改都是功能扩展,无breaking changes
- 现有API保持向后兼容(parallel_prover保留with_default_setup,标记deprecated)
- 性能优化不影响正确性(全局PK初始化只发生一次)

**Recommendations:**
1. 长期压测: 24小时稳定性测试观察内存/CPU趋势
2. 批次大小调优: A/B测试32/64/128对TPS影响
3. Grafana生产部署: 配置Alertmanager邮件/Slack通知
4. 批量验证集成: 将batch_verifier集成到隐私路由验证流程

---

### [L0-CRITICAL] Kernel core MVCC and privacy verifier updates - VERIFIED (2025-11-07)

**Summary:**
- Updated kernel core modules under `src/vm-runtime/`:
  - `mvcc.rs`: Added `enable_adaptive` field to `AutoGcConfig` for future self-tuning GC support
  - `optimized_mvcc.rs`: Minor code cleanup (unused mut warning)
  - `privacy/mod.rs`: Enhanced ZK verifier integration structure
- Fixed compilation errors in examples (demo9_mvcc mutability, mixed_workload_test duplicate main, lfu_hotkey_demo return type)
- Added feature gates for optional ZK examples (`groth16-verifier` feature)

**Verification Results:**
- ✅ **Full workspace tests PASSED**: 118 tests passed (97 vm-runtime unit + 11 integration + 12 privacy-test + others)
  - Key tests: MVCC concurrent read/write, snapshot isolation, auto GC lifecycle, bloom filter optimization, ownership routing
  - Stress tests: high concurrency mixed workload (23s), hotspot contention, memory growth control
  - 1 ignored: `test_long_running_stability` (deferred to CI)
- ✅ **No regressions**: All existing functionality intact; backward compatible
- ✅ **Compilation clean**: No errors across all workspace crates (halo2-eval, node-core, privacy-test, zk-groth16-test, vm-runtime)
- ⚠️ **Performance benchmarks**: Deferred to next run due to file lock contention; recommend CI baseline comparison

**Risk Assessment:** LOW
- Changes are additive (new field with default value)
- No modifications to critical execution paths
- All test coverage maintained

**Next Actions (Optional):**
- Run `cargo bench --bench parallel_benchmark` in CI to establish TPS baseline post-merge
- Consider enabling `test_long_running_stability` in nightly CI runs


### Added - zk-groth16-test v0.1.0 (2025-06-20)

#### Ring Signature 电路与测试 ✅
- 新增模块：`zk-groth16-test/src/ring_signature.rs`
  - 功能：Key Image 生成与验证、环成员存在性验证（简化版环签名）
  - 约束：ring_size=3 → 253 约束（≈84 约束/成员）
  - 公开输入：Key Image（Poseidon 哈希）
- 单元测试（4/4 通过）：
  - `test_key_image_generation`
  - `test_ring_signature_generation_and_verification`
  - `test_ring_signature_circuit_constraints`
  - `test_ring_signature_end_to_end`
- 基准脚本：`zk-groth16-test/benches/ring_signature_benchmarks.rs`
- 报告文档：`zk-groth16-test/RING_SIGNATURE_REPORT.md`

#### RingCT 多 UTXO 集成 ✅
- 更新 `zk-groth16-test/src/ringct_multi_utxo.rs`
  - 集成环签名：Key Image 公开输入（每个输入 1 个）、成员资格验证、输入间 Key Image 去重（反双花约束）
  - 兼容原有：承诺哈希验证、金额平衡、范围证明、Merkle 成员证明
  - 所有相关单元测试通过（集成后）
- 更新 `zk-groth16-test/examples/ringct_multi_utxo_perf.rs`
  - 构造 `ring_auths` 并将 Key Image 纳入公开输入

#### 对抗性测试套件 🛡️
- 新增 `zk-groth16-test/tests/adversarial_tests.rs`（5/5 通过）
  - ✅ `test_double_spend_same_key_image`：相同 Key Image 的两笔交易触发约束失败（Unsatisfiable）
  - ✅ `test_forged_signature_wrong_secret_key`：错误私钥导致 Key Image 不匹配，约束失败
  - ✅ `test_ring_membership_validation`：公钥在环中时约束满足（正常流程验证）
  - ✅ `test_max_ring_size`：ring_size=10 正常工作，约束数=735
  - ✅ `test_zero_value_transaction`：零值交易边界情况正常工作
- 新增测试报告：`zk-groth16-test/ADVERSARIAL_TESTS_REPORT.md`
  - 详细安全性分析、约束分解、性能评估
  - 验证双花防护、签名真实性、发送方匿名等安全属性

#### 相关文档
- `ROADMAP-ZK-Privacy.md`：标记“实现环签名电路（Week 5-6）”与“集成到 Multi-UTXO 交易”为已完成，并补充约束指标与报告链接
- `docs/INDEX.md`：新增“隐私与零知识”板块，汇总研究与实现链接
 - `ROADMAP.md`：将 Phase 5 进度从 30% → 35%，并新增 `scripts/update-roadmaps.ps1` 自动化脚本
 - 新增优化报告：`zk-groth16-test/OPTIMIZATION_REPORT.md`

### Added - vm-runtime v0.9.0 (2025-06-03)

#### Critical Bug Fix: Write Skew Anomaly 🐛🔧
- **根本原因**: MVCC 并发转账出现随机金额偏差（±50-200），违反守恒定律
  - **Issue 1**: Write Skew 异常 - 事务基于过期快照读取，覆盖更新的已提交值
  - **Issue 2**: 部分写可见性 - 提交写入多个 key 时非原子性，新事务可在写入过程中 begin() 并读取部分状态
- **解决方案**:
  - **读集合跟踪** (`reads: HashSet<Vec<u8>>`): 记录事务读取的所有 key
  - **三阶段提交**:
    - Phase 0: 检测读写冲突（包括写集合的 key）
    - Phase 1: 检测写写冲突
    - Phase 2: 原子写入（持有 `commit_lock` + `active_txns` 锁）
  - **关键修复**: 在 commit 写入期间持有 `active_txns` 锁，阻止新事务开始，确保原子性
- **验证结果** ✅:
  - 所有测试通过（10/20/100/1000/10000 笔交易）
  - 金额守恒：total = expected in all cases
  - 性能影响可接受（见下方性能数据）

#### Performance Benchmarks 📊
- **低竞争场景** (50 账户, 10K 交易):
  - **186,993 TPS** (0.053s 总耗时)
  - 0.19 平均重试次数
  - 99.98% 成功率
  - ✅ 金额守恒验证通过
- **高竞争场景** (5 账户, 10K 交易):
  - **85,348 TPS** (0.117s 总耗时)
  - 36.3% 冲突率
  - 0.57 平均重试次数
  - 99.90% 成功率
  - ✅ 金额守恒验证通过

#### API Changes ⚠️
- **Breaking**: `Txn::read()` 现在需要 `&mut self` (用于记录读集合)
  - 所有调用方需更新为 `let mut txn = ...`
  - 影响文件: `parallel.rs`, `parallel_mvcc.rs`

#### Test Suite 🧪
- 新增测试文件:
  - `debug_concurrent_transfer.rs`: 10 笔转账，3 账户
  - `verify_transfer_detailed.rs`: 20 笔转账，5 账户
  - `sequential_transfer_test.rs`: 串行执行基准测试
  - `minimal_conservation_test.rs`: 最小守恒测试（2 账户）
  - `benchmark_parallel_transfer.rs`: 大规模性能测试（100/1000/10000 笔）
  - `benchmark_hotspot_transfer.rs`: 高竞争热点测试
- 所有测试金额守恒验证 ✅

#### Architecture Research 🔬
- 对比分析主流区块链架构:
  - **Solana**: 预声明 + 账户锁定，65K TPS，需预知依赖
  - **Aptos Block-STM**: 乐观并行 + 确定性验证，160K TPS，适合共识
  - **Sui**: 对象所有权 + 最小共识，120K TPS（简单转账），适合去中心化
  - **Monero**: 环签名 + 隐形地址 + RingCT，2K TPS，强隐私保护

### Added - vm-runtime v0.8.0 (2025-05-08)

#### MVCC Stress Testing & Adaptive GC 🔬🤖
- **压力测试套件**:
  - `test_high_concurrency_mixed_workload`: 高并发混合读写（8线程，8000交易，70%读/30%写）
  - `test_high_contention_hotspot`: 高冲突热点键测试（16线程，5个热点键，验证极端冲突场景）
  - `test_memory_growth_control`: 内存增长监控（50键，20迭代，验证 GC 效果）
  - `test_long_running_stability`: 长时间稳定性测试（60秒+，可配置数小时）
  - `test_adaptive_gc`: 自适应 GC 行为验证
- **压力测试统计信息**:
  - `StressTestStats`: 详细的性能报告（TPS、延迟、冲突率、内存使用）
  - 实时监控：TPS、版本数、GC 频率
  - P99 延迟分析
- **自适应 GC 策略** 🎯:
  - **AdaptiveGcStrategy**: 可配置的自适应策略
    - `base_interval_secs`: 基准 GC 间隔（默认 60秒）
    - `min_interval_secs`: 最小间隔（高负载，默认 10秒）
    - `max_interval_secs`: 最大间隔（低负载，默认 300秒）
    - `base_threshold`: 基准版本阈值（默认 1000）
    - `min_threshold`: 最小阈值（更激进，默认 500）
    - `max_threshold`: 最大阈值（更宽松，默认 5000）
  - **自适应调整逻辑**:
    - **高负载检测**: TPS 激增或版本快速增长 → 缩短间隔、降低阈值
    - **低效 GC 检测**: 清理率 < 10% → 延长间隔、提高阈值
    - **正常负载**: 逐渐回归基准值
  - **AutoGcConfig** 新增字段:
    - `enable_adaptive: bool` - 启用/禁用自适应 GC（默认 false）
- **内部优化**:
  - `MvccStore` 新增字段：
    - `adaptive_strategy`: 自适应策略配置
    - `recent_tx_count`: 事务计数器（用于计算 TPS）
    - `recent_gc_cleaned`: 最近 GC 清理数（用于评估效果）
  - 事务提交时自动更新计数器
  - GC 线程根据负载动态调整参数

#### Documentation 📖
- 新增 `docs/stress-testing-guide.md`: 完整的压力测试与调优指南
  - 测试套件使用说明
  - 各测试场景详解
  - 自适应 GC 配置指南
  - 性能调优建议（4 种典型场景）
  - 故障排查手册（4 个常见问题）
- 更新 `README.md`: 添加压力测试使用示例
- 更新 `CHANGELOG.md`: v0.8.0 特性说明

#### API Changes 🔧
- **Breaking**: `AutoGcConfig` 新增 `enable_adaptive: bool` 字段
  - 向后兼容：现有代码添加 `enable_adaptive: false` 即可
- **New**: `AdaptiveGcStrategy` 结构体
- **New**: `StressTestStats` 结构体（测试专用）
- **Export**: `AdaptiveGcStrategy` 导出到公共 API

---

### Added - vm-runtime v0.7.0 (2025-04-15)

#### MVCC Automatic Garbage Collection 🤖🗑️
- **AutoGcConfig**: 自动 GC 配置
  - `interval_secs`: GC 执行间隔（秒，默认 60）
  - `version_threshold`: 触发阈值（版本数，默认 1000，0 表示仅周期触发）
  - `run_on_start`: 启动时立即执行（默认 false）
- **自动 GC 功能**:
  - `start_auto_gc()`: 启动后台 GC 线程（自动启动，无需手动调用）
  - `stop_auto_gc()`: 停止后台 GC 线程
  - `is_auto_gc_running()`: 检查 GC 线程运行状态
  - `update_auto_gc_config()`: 动态更新自动 GC 配置
- **后台线程特性**:
  - 可中断休眠 (100ms 粒度)，快速响应停止信号
  - 双重触发策略：周期性 + 阈值触发
  - Drop 时自动停止并等待线程退出 (最多 2 秒)
  - 原子标志控制，线程安全
- **触发策略**:
  - **周期性**: 每隔 `interval_secs` 秒执行一次
  - **阈值触发**: 当 `total_versions() >= version_threshold` 时立即执行
  - **启动触发**: `run_on_start = true` 时启动时立即执行

#### Testing 🧪
- 新增 5 个自动 GC 测试:
  - `test_auto_gc_periodic`: 周期性自动清理
  - `test_auto_gc_threshold`: 阈值触发自动清理
  - `test_auto_gc_run_on_start`: 启动时立即清理
  - `test_auto_gc_start_stop`: 启动/停止控制
  - `test_auto_gc_concurrent_safety`: 并发安全性
- 总测试数: **64/64 通过** ✅ (+5 from v0.6.0)

#### Benchmarks 📊
- 新增 `auto_gc_impact` 基准组:
  - `write_without_auto_gc` vs `write_with_auto_gc`: 写入性能对比
  - `read_without_auto_gc` vs `read_with_auto_gc`: 读取性能对比
- 性能影响: 写入开销 < 5%，读取无明显影响

#### API Changes 🔧
- **Breaking**: `GcConfig` 新增 `auto_gc: Option<AutoGcConfig>` 字段
  - 向后兼容：现有代码添加 `auto_gc: None` 即可
- **New**: `AutoGcConfig` 结构体
- **New**: `MvccStore::start_auto_gc()` - 启动自动 GC
- **New**: `MvccStore::stop_auto_gc()` - 停止自动 GC
- **New**: `MvccStore::is_auto_gc_running()` - 检查运行状态
- **New**: `MvccStore::update_auto_gc_config()` - 动态更新配置
- **New**: `impl Drop for MvccStore` - 自动清理资源

#### Documentation 📖
- 更新 `README.md`: 添加自动 GC 使用示例
- 更新 `docs/parallel-execution.md`: 添加"MVCC 自动垃圾回收"章节
- 测试计数更新: 59 → 64

---

### Added - vm-runtime v0.6.0 (2025-04-01)

#### MVCC Garbage Collection 🗑️
- **GcConfig**: 可配置的垃圾回收策略
  - `max_versions_per_key`: 每个键最多保留的版本数（默认 10）
  - `enable_time_based_gc`: 是否启用基于时间的 GC（默认 false）
  - `version_ttl_secs`: 版本过期时间（秒）
- **MvccStore GC 功能**:
  - `gc()`: 手动触发垃圾回收，清理不再需要的旧版本
  - `get_gc_stats()`: 获取 GC 统计信息（执行次数、清理版本数、清理键数）
  - `get_min_active_ts()`: 获取活跃事务的最小时间戳（水位线）
  - `set_gc_config()`: 动态更新 GC 配置
  - `total_versions()`: 获取当前总版本数（监控用）
  - `total_keys()`: 获取当前键数量（监控用）
- **活跃事务跟踪**:
  - 自动注册和注销活跃事务（通过 begin/drop）
  - GC 保护活跃事务可见的所有版本
  - 基于水位线的智能清理策略
- **GC 清理策略**:
  - 保留每个键的最新版本（无条件）
  - 保留所有活跃事务可见的版本（基于 min_active_ts）
  - 根据 max_versions_per_key 限制清理超量版本
  - 避免清理仍在使用的版本，确保正确性

#### Testing 🧪
- 新增 5 个 GC 测试:
  - `test_gc_version_cleanup`: 版本清理正确性
  - `test_gc_preserves_active_transaction_visibility`: 保护活跃事务可见性
  - `test_gc_no_active_transactions`: 无活跃事务时的清理
  - `test_gc_multiple_keys`: 多键 GC
  - `test_gc_stats_accumulation`: GC 统计累计
- 总测试数: **59/59 通过** ✅

#### Benchmarks 📊
- 新增 `mvcc_gc` 基准组:
  - `gc_throughput`: 不同版本数下的 GC 吞吐量
  - `read_with_gc`: GC 对读取性能的影响
  - `write_with_gc`: GC 对写入性能的影响
  - `gc_with_active_transactions`: 活跃事务对 GC 的影响

#### API Changes 🔧
- `MvccStore::new_with_config(config: GcConfig)`: 创建带 GC 配置的存储
- 导出新类型: `GcConfig`, `GcStats`
- `Txn` 自动在 Drop 时注销活跃事务

#### Performance 🚀
- **内存控制**: 通过定期 GC 控制内存增长
- **智能清理**: 仅清理不再需要的版本，不影响活跃事务
- **低开销**: GC 使用写锁，不阻塞读操作

## [0.5.0] - 2025-03-15

### Added - vm-runtime v0.5.0

#### MVCC Multi-Version Concurrency Control 🔐
- **MvccStore**: 多版本并发控制存储实现
  - 快照隔离 (Snapshot Isolation) 语义
  - 每个键维护版本链,按时间戳升序存储
  - 原子时间戳分配 (AtomicU64),消除瓶颈
  - **细粒度并发控制**:
    - DashMap 无锁哈希表,减少全局锁争用
    - 每键 RwLock 读写锁,允许并发读取
    - 提交时按键排序加锁,避免死锁
    - 仅锁定写集合涉及的键,最小化锁持有范围
- **Txn**: 事务接口
  - `begin()`: 开启读写事务,分配快照版本 (start_ts)
  - `begin_read_only()`: 开启只读事务 (快速路径)
  - `read()`: 读取 start_ts 及之前的可见版本
  - `write()` / `delete()`: 本地缓存写操作 (只读事务会 panic)
  - `commit()`: 提交事务,进行写写冲突检测 (只读无需检测,直接返回 start_ts)
  - `abort()`: 放弃事务
- **只读事务优化** ⚡:
  - `begin_read_only()` 标记事务为只读
  - 提交时跳过冲突检测和锁获取
  - 无写集合,直接返回快照时间戳
  - 显著降低只读查询开销
- **冲突检测**:
  - 提交时检测写写冲突 (Write-Write Conflict)
  - 若发现 ts > start_ts 的已提交版本则拒绝提交
  - 保证可串行化 (Serializability)

#### Scheduler Integration with MVCC 🔗
- **ParallelScheduler MVCC 支持**:
  - `new_with_mvcc(store: Arc<MvccStore>)`: 创建 MVCC 后端调度器
  - `execute_with_mvcc<F>(&self, operation: F)`: 执行读写事务
    - 自动开启事务、执行操作、提交或回滚
    - 更新统计信息 (successful/failed/rollback)
  - `execute_with_mvcc_read_only<F>(&self, operation: F)`: 执行只读事务
    - 使用快速路径,无冲突检测开销
    - 适用于查询密集型场景
  - 非破坏性集成: 保留原有 snapshot 机制,可选使用 MVCC

#### Testing 🧪
- 新增 10 个 MVCC 核心测试:
  - `test_mvcc_write_write_conflict`: 写写冲突检测
  - `test_mvcc_snapshot_isolation_visibility`: 快照隔离可见性
  - `test_mvcc_version_visibility_multiple_versions`: 多版本可见性
  - `test_mvcc_concurrent_reads`: 并发读取性能
  - `test_mvcc_concurrent_writes_different_keys`: 不同键并发写
  - `test_mvcc_concurrent_writes_same_key_conflicts`: 同键冲突检测
  - `test_mvcc_read_only_transaction`: 只读事务快速路径
  - `test_mvcc_read_only_cannot_write`: 只读事务写入保护
  - `test_mvcc_read_only_cannot_delete`: 只读事务删除保护
  - `test_mvcc_read_only_performance`: 只读性能对比
- 新增 3 个 MVCC 调度器集成测试:
  - `test_scheduler_mvcc_basic_commit`: MVCC调度器基础提交
  - `test_scheduler_mvcc_abort_on_error`: MVCC调度器错误回滚
  - `test_scheduler_mvcc_read_only_fast_path`: MVCC调度器只读路径
- 总测试数: **54/54 通过** ✅ (v0.5.0 基础)

#### Dependencies 📦
- 新增 `dashmap ^6.1`: 高性能并发哈希表
- 新增 `parking_lot ^0.12`: 更快的 RwLock 实现

#### Performance 🚀
- **并发读取**: 多事务可同时读取不同键 (无锁竞争)
- **并发写入**: 不同键的写入可并发执行
- **时间戳分配**: 原子操作,避免锁开销
- **锁粒度**: 从全局锁优化为每键锁,大幅降低争用

## [0.4.0] - 2025-03-01

### Added - vm-runtime v0.4.0

#### Batch Operations Optimization 📦
- **StateManager 批量操作**:
  - `batch_write()`: 批量写入,减少锁争用
  - `batch_read()`: 批量读取,一次性获取多个键
  - `batch_delete()`: 批量删除
  - `batch_emit_events()`: 批量发送事件
  - **性能提升**: 相比单个操作,批量写入可提升数倍性能
- **ParallelScheduler 批量执行**:
  - `execute_batch()`: 批量执行交易,共享一个快照
  - 原子性保证: 批次中任何交易失败,整个批次回滚
  - `batch_write()` / `batch_read()` / `batch_delete()`: 直接批量操作接口
  - 减少快照创建/提交开销
  
#### Testing 🧪
- 新增 6 个批量操作测试:
  - `test_batch_write`: 批量写入
  - `test_batch_read`: 批量读取
  - `test_batch_delete`: 批量删除
  - `test_batch_emit_events`: 批量事件
  - `test_execute_batch`: 批量执行成功
  - `test_execute_batch_rollback`: 批量失败回滚
- 总测试数: **41/41 通过** ✅

#### Documentation 📚
- 更新文档说明批量操作 API

#### Examples 💡
- **Demo 8**: 批量操作演示 (`demo8_batch_operations.rs`)
  - 批量写入性能对比 (1000 条记录)
  - 批量读取示例
  - 批量执行交易
  - 批量失败自动回滚

## [0.3.0] - 2025-11-03

### Added - vm-runtime v0.3.0

#### Work-Stealing Scheduler ⚡
- **WorkStealingScheduler**: 工作窃取调度器
  - 基于 crossbeam-deque 和 rayon 的高性能任务调度
  - 自动负载均衡: 空闲线程从忙碌线程窃取任务
  - `submit_task()` / `submit_tasks()`: 提交任务到全局队列
  - `execute_all()`: 并行执行所有任务
  - 支持任务优先级 (0-255)
  - 集成 ParallelScheduler 进行状态管理
- **Task**: 任务定义
  - `tx_id`: 交易标识符
  - `priority`: 任务优先级
- **性能提升**:
  - 减少线程空闲时间
  - 提高 CPU 利用率
  - 支持大规模任务处理 (测试 1000+ 任务)

#### Testing 🧪
- 新增 3 个工作窃取测试:
  - `test_work_stealing_basic`: 基础工作窃取
  - `test_work_stealing_with_priorities`: 优先级调度
  - `test_work_stealing_with_errors`: 错误处理
- 总测试数: **35/35 通过** ✅

#### Documentation 📚
- 更新 `docs/parallel-execution.md`:
  - 添加 WorkStealingScheduler 详细说明
  - 工作窃取算法原理
  - API 使用示例
  - 性能优化建议

#### Examples 💡
- **Demo 7**: 工作窃取调度器演示 (`demo7_work_stealing.rs`)
  - 基础工作窃取
  - 优先级调度
  - 大规模任务处理 (1000 任务)
  - 与 ParallelScheduler 集成

## [0.2.0] - 2025-11-03

### Added - vm-runtime v0.2.0

#### Parallel Execution Engine 🚀
- **ParallelScheduler**: 并行交易调度器
  - `execute_with_snapshot()`: 快照保护的事务执行
  - `execute_with_retry()`: 带自动重试的事务执行
  - `get_stats()`: 获取执行统计信息
- **ConflictDetector**: 冲突检测器
  - `record()`: 记录交易读写集
  - `has_conflict()`: 检测两个交易是否冲突
  - `build_dependency_graph()`: 构建依赖关系图
- **DependencyGraph**: 依赖图管理
  - `add_dependency()`: 添加依赖关系
  - `get_ready_transactions()`: 获取可并行执行的交易
- **StateManager**: 状态管理器
  - `create_snapshot()`: 创建状态快照
  - `rollback()`: 回滚到快照状态
  - `commit()`: 提交并丢弃快照
  - 支持嵌套快照
- **ExecutionStats**: 执行统计
  - 成功/失败交易计数
  - 回滚/重试次数统计
  - 冲突检测计数
  - 成功率/回滚率计算

#### Crypto API (`crypto_api` module)
- `sha256(data_ptr, data_len, output_ptr) -> i32`: SHA-256 哈希
- `keccak256(data_ptr, data_len, output_ptr) -> i32`: Keccak-256 哈希
- `ed25519_verify(msg_ptr, msg_len, sig_ptr, pubkey_ptr) -> i32`: Ed25519 签名验证
- `secp256k1_verify(msg_ptr, msg_len, sig_ptr, pubkey_ptr) -> i32`: ECDSA 签名验证
- `derive_eth_address(pubkey_ptr, pubkey_len, output_ptr) -> i32`: 以太坊地址派生

#### Performance Benchmarks
- 添加 criterion 基准测试框架
- 4 组基准测试:
  - 冲突检测性能 (10/50/100/500 交易)
  - 快照操作性能 (10/100/1000 数据项)
  - 依赖图构建性能
  - 并行调度性能

#### Testing
- ✅ 32/32 单元测试通过
  - 11 个并行执行测试
  - 5 个密码学测试
  - 5 个状态快照测试
  - 3 个调度器集成测试
  - 8 个核心功能测试

### Added - node-core v0.2.0 (2025-11-03)

#### Demo Programs
- **Demo 3**: 密码学功能演示
  - SHA-256 和 Keccak-256 哈希计算
  - 哈希验证
- **Demo 4**: 以太坊地址派生
  - 从公钥派生以太坊地址
- **Demo 5**: 并行执行演示
  - 3 笔交易的冲突检测
  - 依赖关系分析
  - 并行调度展示
- **Demo 6**: 状态快照与回滚 ✨
  - 场景 1: 成功的交易提交
  - 场景 2: 失败的交易自动回滚
  - 场景 3: 嵌套交易部分回滚

---

## [0.1.0] - 2025-11-02

### Added - vm-runtime v0.1.0

#### Core Runtime
- **WASM Execution Engine**: Integrated wasmtime 17.0 for WebAssembly execution
- **Storage Abstraction**: `Storage` trait with `MemoryStorage` implementation
- **Host Functions Architecture**: Modular host function registration system

#### Storage API (`storage_api` module)
- `storage_get(key_ptr, key_len) -> i64`: Get value by key, cache to `last_get`
- `storage_read_value(ptr, len) -> i32`: Read cached value from last get
- `storage_set(key_ptr, key_len, value_ptr, value_len) -> i32`: Write key-value pair
- `storage_delete(key_ptr, key_len) -> i32`: Delete key from storage

#### Chain Context API (`chain_api` module)
- `block_number() -> i64`: Get current block number
- `timestamp() -> i64`: Get current block timestamp
- `emit_event(data_ptr, data_len) -> i32`: Emit an event to host
- `events_len() -> i32`: Get total number of emitted events
- `read_event(index, ptr, len) -> i32`: Read event data by index

#### Public APIs
- `Runtime::new(storage: S)`: Create runtime with custom storage backend
- `Runtime::execute_add(&self, module_bytes, a, b) -> Result<i32>`: Execute add function (demo)
- `Runtime::execute_with_context(&self, module_bytes, func_name, block_number, timestamp) -> Result<(i32, Vec<Vec<u8>>, u64, u64)>`: Execute function with block context and return events

#### Testing
- ✅ 6/6 unit tests passing:
  - `test_memory_storage`: Storage trait implementation
  - `test_execute_add_via_wat`: Basic WASM execution
  - `test_storage`: Storage operations via runtime
  - `test_host_functions`: Host function calls from WASM
  - `test_emit_event`: Event emission and reading
  - `test_execute_with_context`: Full context execution with events

### Added - node-core v0.1.0

#### CLI Features
- `--once` flag: Run once and exit without waiting for Ctrl-C (for automated testing)
- **Demo 1**: Simple add(7,8) demonstration
- **Demo 2**: Full event system showcase
  - Emits "UserAction" and "BlockProcessed" events
  - Uses storage API to write key-value pairs
  - Demonstrates block context (block_number, timestamp) access
  - Pretty-prints collected events to console

#### Logging
- Integrated tracing + tracing_subscriber for structured logging
- INFO-level output for demo results

### Changed

#### Project Structure
- Workspace resolver set to "2" (eliminates Cargo warnings)
- .gitignore updated with UTF-8 comments
- /solana/ directory excluded from version control (local reference only)

### Technical Details

#### Memory Management
- Host functions use `Rc<RefCell<Storage>>` for shared mutable state
- Memory handle cloning pattern to avoid borrow checker conflicts
- Safe memory access via `read_memory` and `write_memory` helpers

#### Module Naming
- Host functions registered under proper namespaces:
  - `storage_api::*` for storage operations
  - `chain_api::*` for blockchain context and events
- WAT imports must match these module names exactly

#### Performance Considerations
- Storage operations use BTreeMap (O(log n) complexity)
- Event collection uses Vec (append-only, no reallocation concerns for typical use)
- Memory operations validated with bounds checking

## [0.0.0] - 2025-01-XX (Initial PoC)

### Added
- Initial repository structure
- Basic Cargo workspace setup
- wasmtime integration proof-of-concept
- Simple WAT example execution

---

## Development Timeline

- **Week 1**: PoC - Basic WASM runtime with wasmtime
- **Week 2**: Storage abstraction and host function architecture
- **Week 3**: Chain context, event system, and execute_with_context API
- **Next**: Compiler adapter for Solidity/AssemblyScript

## Contributors

- king <king@example.com> - Initial development

## Notes

### Breaking Changes
None yet (pre-1.0.0)

### Migration Guide
N/A (first release)

### Known Issues
- Push to remote repository blocked by network issues (large history)
- solana/ directory remains in local filesystem (gitignored)

### Upcoming Features (Roadmap)
See [ROADMAP.md](ROADMAP.md) for planned features:
- Solidity compiler integration (Solang)
- AssemblyScript support
- Parallel execution engine
- EVM compatibility layer
