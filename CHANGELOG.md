# Changelog

All notable changes to SuperVM will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### [L0.8 Performance] 拥塞控制与热键检测 (2025-11-12)

**Added** - 新增功能
- `FastPathExecutor`: 拥塞检测 `is_congested()` 基于队列长度/阈值比例
- 热键跟踪: `track_key_access(key)` 记录访问频率,`get_hot_keys(top_k)` 返回 Top-K
- 智能重试: `execute_with_congestion_control()` 拥塞感知自适应退避 (1x → 10x)
- 阈值配置: `set_congestion_threshold(threshold)` 动态调整拥塞阈值
- 统计清空: `reset_hot_keys()` 支持周期性重置热键统计
- 新增示例: `examples/congestion_control_demo.rs` 验证拥塞控制效果

**Changed** - 行为变更
- 拥塞退避倍数: 根据 `queue_length / threshold` 线性增加 (上限 10x)
- 抖动机制: ±100ms 随机延迟防止雷鸣群 (thundering herd)
- 保留指数退避: 1ms → 2ms → 4ms → ... → 1s 上限（基础机制）

**Performance** - 性能改进
- **正常负载** (队列 500/1000): 重试 2 次 3.961ms (1x 退避)
- **拥塞场景** (队列 5000/1000): 重试 2 次 15.44ms (5x 自适应退避)
- **热键检测**: 1000 次访问,Top-3 热键准确率 100%
- **预期 TPS 提升**: 15-20% (避免无效重试风暴)

**Configuration** - 配置示例
```bash
# 运行拥塞控制演示
cargo run --release --example congestion_control_demo
```

**Metrics** - 拥塞级别映射
- 队列 500/1000  → 无拥塞 (1x 退避)
- 队列 2000/1000 → 轻度拥塞 (2x 退避)
- 队列 5000/1000 → 中度拥塞 (5x 退避)
- 队列 10000/1000 → 重度拥塞 (10x 退避, 上限)

**Usage Recommendation** - 使用建议
```rust
let executor = FastPathExecutor::new();
executor.set_congestion_threshold(1000); // 设置队列阈值

// 拥塞感知重试 (自动检测拥塞并调整退避)
let result = executor.execute_with_congestion_control(tx_id, || {
    // your operation
}, max_retries);

// 热键检测
executor.track_key_access(key);
let top_keys = executor.get_hot_keys(10); // Top-10 热键
```

---

### [L0.7 Performance] ProvingKey 全局缓存优化 (2025-11-12)

**Added** - 新增功能
- Multiply Circuit: 全局 ProvingKey 缓存 `MULTIPLY_PROVING_KEY` 单例
- RingCT Circuit: 已有 `RINGCT_PROVING_KEY` 缓存（v0.2.0 引入）
- `ParallelProver::with_shared_setup(config)`: 使用全局缓存的推荐构造方法
- `RingCtParallelProver::with_shared_setup(config)`: 同上（已存在,保持一致性）
- 新增示例: `examples/proving_key_cache_demo.rs` 验证缓存效果

**Changed** - 行为变更
- `ParallelProver`: 推荐使用 `with_shared_setup()` 替代 `new()` 以复用全局 ProvingKey
- 全局缓存使用 `once_cell::sync::Lazy` 实现延迟初始化（首次访问时 setup）
- `Arc<ProvingKey>` 支持线程安全共享，零拷贝引用计数

**Performance** - 性能改进
- **Multiply Circuit**: Prover 创建加速 **144x** (14.10ms → 0.098ms)
- **RingCT Circuit**: Prover 创建加速 **1312x** (54.34ms → 0.041ms)
- **内存优化**: 单一全局实例,每电路类型节省 ~500KB × N provers
- **Setup 开销**: 一次性初始化（首次访问时）,后续零开销
- **吞吐验证**: Multiply 证明 TPS 855.20 (5 proofs / 5.85ms)

**Documentation** - 文档更新
- 示例 `proving_key_cache_demo.rs` 对比首次创建 vs 复用性能
- 输出统计：创建延迟、加速倍数、内存节省、TPS 验证
- 使用建议：推荐 `with_shared_setup()` 作为默认方法

**Configuration** - 配置示例
```bash
# 运行缓存验证演示
cargo run --release --example proving_key_cache_demo --features groth16-verifier
```

**Metrics** - 性能数据
- Multiply 首次创建: 14.10ms (包含 setup)
- Multiply 复用创建: 0.098ms (144x 加速)
- RingCT 首次创建: 54.34ms (包含 setup)
- RingCT 复用创建: 0.041ms (1312x 加速)
- 内存节省: ~500KB × (N-1) provers

**Usage Recommendation** - 使用建议
```rust
// ✅ 推荐：使用全局缓存
let prover = ParallelProver::with_shared_setup(config);
let ringct_prover = RingCtParallelProver::with_shared_setup(config);

// ⚠️  仅在需要自定义 ProvingKey 时使用
let custom_prover = ParallelProver::new(custom_pk, config);
```

---

### [L0.6 Performance] Parallel Prover 线程池复用优化 (2025-11-12)

**Added** - 新增功能
- Parallel Prover: 全局线程池单例 `GLOBAL_PROVER_POOL` 持久化复用
- Parallel Prover: 环境变量支持 `PROVER_THREADS=N` 覆盖默认线程数
- Parallel Prover: 线程命名 `prover-worker-{i}` 便于调试与性能分析
- Parallel Prover: 池统计追踪 (`POOL_TASK_COUNT`, `POOL_TOTAL_DURATION_NS`)
- Parallel Prover: `get_pool_stats()` 函数查询累计任务数与平均延迟
- `ParallelProver` / `RingCtParallelProver`: `with_custom_pool(pool)` 方法支持高级用户自定义池
- 新增示例: `examples/prover_pool_demo.rs` 演示线程池复用性能提升

**Changed** - 行为变更
- `ParallelProver::prove_batch()`: 使用全局池替代临时池，消除每次调用的创建/销毁开销
- `RingCtParallelProver::prove_batch()`: 同上，统一使用全局池
- `ParallelProver` / `RingCtParallelProver`: 新增 `custom_pool` 字段用于可选自定义池

**Performance** - 性能改进
- **延迟降低**: 15-25% (消除池创建开销 ~5-10ms/batch)
- **内存优化**: 峰值内存降低 30-40% (全局 ProvingKey + 单一池实例)
- **吞吐量**: 50 proofs 总耗时 0.99s，平均 TPS 50.42 (~20ms/proof)
- **池复用效率**: 100% (零临时池分配，持久化线程池)
- **扩展性**: 支持 `PROVER_THREADS` 环境变量动态配置并发度

**Documentation** - 文档更新
- 示例 `prover_pool_demo.rs` 演示 5 批次 × 10 proofs = 50 proofs
- 输出统计：批次延迟、TPS、线程池任务数、平均延迟
- 优化收益说明：线程池复用、ProvingKey 缓存、环境变量配置

**Next Steps** - 后续计划
- FastPath 拥塞控制与热 key 检测（Q1 2025 高优先级）
- Parallel Prover 批量验证聚合优化
- 创建 `tests/perf_matrix.rs` 回归测试框架
- 更新 Grafana 面板支持新指标

**Configuration** - 配置示例
```bash
# 设置 Prover 线程数（默认：CPU 核心数）
export PROVER_THREADS=16

# 运行演示
cargo run --release --example prover_pool_demo --features groth16-verifier
```

**Metrics** - 指标参考
- 线程池统计: `POOL_TASK_COUNT`, `POOL_TOTAL_DURATION_NS`
- 调用接口: `get_pool_stats() -> (task_count: u64, avg_ms: f64)`
- 演示输出:
  ```
  Total Tasks Processed: 50
  Avg Duration per Task: 19.83ms
  Pool Reuse Efficiency: 100%
  Overall TPS: 50.42
  ```

---

### [L0.5 Performance] FastPath 延迟分位增强 (2025-11-12)

**Added** - 新增功能
- FastPath: 延迟分位统计 (p50/p90/p95/p99) 基于 `LatencyHistogram`
- FastPath: 带指数退避的 `execute_with_retry` 方法（支持最大重试次数与抖动）
- FastPath: 队列长度追踪 (`set_queue_length` / `get_queue_length`)
- FastPath: Prometheus 格式指标导出 (`export_prometheus`)
- `LatencyHistogram::percentile(pct)`: 计算任意百分位延迟（例如 P95）
- 新增示例: `examples/fastpath_latency_demo.rs` 演示延迟分位与重试统计

**Changed** - 行为变更
- `FastPathExecutor`: 内部集成 `LatencyHistogram`，自动记录每次执行延迟
- `FastPathStats`: 扩展字段 `retry_count`, `queue_length`, `p50/p90/p95/p99_latency_ms`
- `FastPathStats::summary()`: 新增人类可读格式化输出（包含延迟分位与重试率）

**Performance** - 性能改进
- 延迟追踪开销: 每次事务 +1 次 atomic fetch_add + histogram 桶查找（~50ns）
- 重试机制支持指数退避（1ms → 2ms → 4ms → ... → 1s 上限），减少重试风暴
- 可观测性增强：暴露 p99 慢请求，便于识别长尾问题

**Documentation** - 文档更新
- `docs/PERF-OPTIMIZATION-NEXT.md`: 完整优化路线图（FastPath + Parallel Prover）
- README.md: 新增 `docs/PERF-OPTIMIZATION-NEXT.md` 引用
- docs/INDEX.md: 索引补充 PERF-OPTIMIZATION-NEXT.md

**配置示例**:
```rust
// 使用默认直方图
let executor = FastPathExecutor::new();

// 使用外部共享直方图（集成到全局 MetricsCollector）
let histogram = Arc::new(LatencyHistogram::new());
let executor = FastPathExecutor::with_histogram(histogram);

// 带重试执行
executor.execute_with_retry(tx_id, || { /* ... */ }, 3)?;

// 导出 Prometheus 指标
println!("{}", executor.export_prometheus("fastpath"));
```

**指标示例**:
```promql
# P99 延迟（毫秒）
fastpath_latency_ms{quantile="0.99"}

# 重试率
rate(fastpath_retries_total[5m]) / rate(fastpath_txns_total[5m])

# 队列积压告警
fastpath_queue_length > 1000
```

**下一步**:
- [ ] 拥塞控制与热键检测（防止重试风暴）
- [ ] 真实复杂工作负载矩阵（DeFi/GameFi/NFT 场景）
- [ ] Parallel Prover 线程池复用 + ProvingKey 全局缓存

---

### [L0.7 True 2PC Prepare Phase] 读集合校验与冲突检测 (2025-11-13)

Summary:
- **实现真实 prepare 阶段**（`TwoPhaseCoordinator::prepare_and_commit`）：
  - 写集合锁定：按字典序排序 key 后加锁，避免死锁（已有）
  - **读集合校验（NEW）**：对每个读 key 检查 `tail_ts`（最新提交版本时间戳），若 `tail_ts > start_ts` 则 abort
  - 冲突检测与 abort 协议：prepare 失败时记录 `cross_shard_prepare(success=false)` 指标，返回 `Err` 含冲突详情
  - 持锁期间执行 commit，锁自动释放（当前简化实现；后续拆分 prepare/commit 双阶段）
- **MVCC 扩展**：
  - `Txn::reads()`: 暴露读集合引用供 2PC prepare 校验
  - `MvccStore::get_tail_ts(key)`: 获取 key 的最新提交版本时间戳（若不存在返回 0）
- **单元测试**:
  - `two_pc_read_write_conflict_abort`: 验证 T1 读取 key → T2 提交修改 key → T1 2PC prepare 因 read-write conflict abort

Files Changed:
- Modified: `src/vm-runtime/src/two_phase_consensus.rs` (真实 prepare 阶段实现 + conflict abort 测试)
- Modified: `src/vm-runtime/src/mvcc.rs` (新增 `Txn::reads()`, `MvccStore::get_tail_ts()`)

Behavior:
- **Prepare 成功路径**：读集合全部 key 的 tail_ts ≤ start_ts → 继续 commit
- **Prepare 失败路径**：读集合中任一 key 的 tail_ts > start_ts → 立即 abort，释放锁，返回包含冲突信息的 Err

Performance Impact:
- 新增读集合校验开销：每个读 key 一次 DashMap 查询 + RwLock::read() 获取 tail_ts（O(reads) 复杂度）
- 对于只读事务或读集合较小的事务，开销可忽略（<1µs per key）
- Abort 后立即释放锁，避免持锁时间浪费；冲突率高时可通过指标监控优化

Constraints & Next Steps:
- **当前 commit 阶段仍为单体实现**：prepare 成功后直接调用 `txn.commit()`，未拆分为独立 commit 阶段
- **下一步（任务 4）**：拆分 commit 阶段 — 获取 commit_ts + 批量 `append_version` 写入 + 异步释放锁，独立测量 prepare/commit 延迟
- **后续优化**：
  - 并行读集合校验（当前串行遍历 `reads`）
  - 分区级并行 prepare/commit（将跨分区事务拆分为子任务）
  - Abort 率监控指标：`two_pc_abort_total{reason="read_conflict|timeout"}`

### [L0.6 Two-Phase Consensus Integration] 多分区事务 2PC 路径原型 (2025-11-13)

Summary:
- **新增模块 `two_phase_consensus`**（受 `partitioned-fastpath` feature 控制）：
  - `TwoPhaseCoordinator::prepare_and_commit`: 对多分区事务执行最小可行的 prepare 阶段：
    - 按字典序对写集合 key 加锁（`MvccStore::key_lock`），记录 `cross_shard_prepare` 指标。
    - 持锁期间同步提交，避免死锁（全局一致加锁顺序）。
  - 后续扩展：读集合校验、冲突检测、分区级并行 prepare/commit 双阶段。
- **集成到 `multi_core_consensus.rs`**：
  - `route_or_commit` 基于 `Txn::partition_set()` 计算写集合跨分区情况：
    - 单分区 → 异步路由到分区 worker（快速路径）
    - 多分区 → 调用 `TwoPhaseCoordinator::prepare_and_commit`（2PC 路径，当前占位实现为同步提交）
    - 无写集合 → 同步提交（只读事务）
  - 新增指标记录：`consensus_routed`/`fallback`/`executed` 及延迟直方图 (`route_latency`, `commit_latency`)。
- **MVCC 扩展**：
  - `Txn::partition_set(partitions)`: 计算写集合涉及的分区集合（与 `multi_core_consensus` 使用相同哈希算法 FNV-1a）。
  - `Txn::metrics()`: 暴露指标收集器引用，供 worker/2PC 记录延迟。
  - `MvccStore::key_lock(&key)`: 返回细粒度 key 级互斥锁（`Arc<Mutex<()>>`），供 2PC prepare 阶段锁定。
  - `MvccStore::append_version(&key, ts, value)`: 直接附加版本到指定 key 版本链（供未来真实 2PC commit 阶段批量写入）。
- **单元测试**:
  - `multi_core_consensus::tests::route_single_partition`: 单键事务 → 异步路由，返回占位 ts=0。
  - `multi_core_consensus::tests::multi_partition_goes_2pc_and_commits`: 多键跨分区事务 → 触发 2PC 路径，同步提交并验证数据可读。
- **新增基准测试示例 `two_pc_consensus_bench`**:
  - 混合单/多分区事务工作负载（可配置 `--multi_ratio`）；测量总吞吐与路由分布。
  - 用法: `cargo run -p vm-runtime --example two_pc_consensus_bench --release --features partitioned-fastpath -- --txs:50000 --partitions:4 --multi_ratio:0.2`

Files Added/Changed:
- Added: `src/vm-runtime/src/two_phase_consensus.rs`
- Added: `src/vm-runtime/examples/two_pc_consensus_bench.rs`
- Modified: `src/vm-runtime/src/multi_core_consensus.rs` (集成 2PC 路由逻辑, 增加 2 个单元测试)
- Modified: `src/vm-runtime/src/mvcc.rs` (新增 `partition_set`, `metrics`, `key_lock`, `append_version`)
- Modified: `src/vm-runtime/src/metrics.rs` (新增 `consensus_routed_total`, `fallback_total`, `executed_total`, `route_latency`, `commit_latency` 指标及 Prometheus 输出)
- Modified: `src/vm-runtime/src/lib.rs` (暴露 `two_phase_consensus` 模块)
- Modified: `src/vm-runtime/Cargo.toml` (注册 `two_pc_consensus_bench` 示例)

Performance (2PC 路径当前为占位同步实现，吞吐与单核相当; 待真实 prepare/commit 协议后预期提升):
- 单分区事务吞吐：~636K TPS (partitions=4, batch=512, 最佳配置)
- 多分区事务吞吐：当前与单核 commit 相同（~373K TPS），因 2PC 路径仍为同步提交占位实现；真实 prepare/commit 双阶段后预期改善。

Constraints & Next Steps:
- **当前 2PC 为最小可行占位实现**：prepare 阶段仅加锁+记录指标，未实现读集合校验、冲突检测、并行 prepare/commit。
- **下一步候选**：
  - 真实 2PC prepare: 并行锁定所有分区 key, 读集合校验 tail_ts 未变（串行化检测），收集 prepare-ok 决议。
  - 真实 2PC commit: 批量调用 `append_version` 写入各分区版本链，异步完成 commit。
  - 分区级并行 prepare/commit: 将跨分区事务拆分为子任务并发执行 prepare/commit，进一步提升多分区吞吐。
  - 适配诊断指标：分区不均衡监控、2PC abort 率、prepare 延迟分位数；Prometheus 输出 `multi_consensus_*` 新指标至 `/metrics` 端点聚合器。

### [L0.5 Multi-Core Consensus] 单分区路由原型突破 500K TPS (2025-11-11)

Summary:
- 新增模块 `multi_core_consensus`（受 `partitioned-fastpath` feature 控制）实现多核共识原型：
  - 写集合按 key 哈希分区；若事务写集合全部命中同一分区，则路由到对应 worker 异步提交，否则回退同步提交（保持语义简单与安全）。
  - 每分区本地维护时间戳批次缓存（ts_next..ts_end），从全局原子按批量（默认 512）获取，降低全局争用。
  - `Txn` 支持外部时间戳注入：新增 `override_commit_ts` 与 `with_ts(ts)`，`commit()` 优先使用外部 ts。
- 新增示例 `multi_core_consensus_bench`：可配置 `--txs/--partitions/--batch`，用于测量多核吞吐。

Files Added/Changed:
- Added: `src/vm-runtime/src/multi_core_consensus.rs`
- Added: `src/vm-runtime/examples/multi_core_consensus_bench.rs`
- Modified: `src/vm-runtime/src/mvcc.rs`（`Txn::writes()`, `Txn::with_ts()`, `override_commit_ts` 字段，`commit()` 支持外部 ts；`MvccStore::next_ts` 改为 `pub(crate)`）
- Modified: `src/vm-runtime/src/lib.rs` 暴露模块；`Cargo.toml` 注册示例

Performance (200K 单键写事务，纯共识路径，Windows 本机):
- 单核 `mixed_path_bench`：~373K TPS（波动环境下的近期值；历史峰值 ~418–429K）
- 多核（单分区路由）：
  - partitions=2, batch=512: ~121K TPS（不稳定/受限，待进一步分析）
  - partitions=4, batch=512: ~636K TPS（最佳）
  - partitions=8, batch=512: ~626K TPS（略低于 4 分区，可能因调度/CPU 饱和）
  - partitions=4, batch=1024: ~607K TPS；batch=2048: ~581K TPS（批次过大略有回退）

Interpretation:
- 对于单键或写集合完全落在同一分区的事务，分区化+本地批次时间戳带来明显提升；在 4 分区时突破 500K TPS，并在本机达到 ~636K TPS。
- 批量（batch）512 在本机表现最佳；更大批量在该负载下不增反降。

Constraints & Next Steps:
- 仅对“单分区写集合”进行异步路由；跨分区写集合直接回退同步提交（保持简单语义，避免跨分区冲突协议）。
- 下一步候选：
  - 跨分区两阶段协调（更大改动，提升覆盖率与吞吐）
  - 分区锁分层（降低 `key_locks` 热点）
  - 参数寻优：分区数与批量大小在不同硬件上的最优点

### [L0.5 FastPath Performance Analysis] FastPath 性能分析与优化路径规划 (2025-11-11)

**Summary:**
- **FastPath 性能基线验证**:
  - 运行 fast_path_bench: **28.57M TPS, 35ns 延迟** (100万次迭代)
  - 确认 FastPath 已达近乎最优性能 (零锁/零分配/CPU L1 cache 级延迟)
- **性能瓶颈识别**:
  - FastPath 优化空间 <10% (atomic ops, ownership lookup, route decision 均已优化)
  - Consensus 路径高潜力: 377K TPS → **500K TPS** 目标 (+33%)
  - 多核扩展可行性: 28.57M → **180M TPS@8核** (6.3x scaling)
- **优化路径规划**:
  - **Phase 1**: DashMap 替换 RwLock<HashMap> (预计 +20%)
  - **Phase 2**: Smallvec 优化版本链 (预计 +10%)
  - **Phase 3**: Per-Thread 时间戳批量分配 (预计 +3%)
  - **Phase 4**: PartitionedFastPath 多核扩展 (预计 6x@8核)

**Files Added:**
- `docs/FASTPATH-PERFORMANCE-ANALYSIS.md`: 完整性能分析报告 (500+ 行)
  - FastPath 执行流程拆解 (35ns 分解到各阶段)
  - 热点路径分析 (Atomic ops 43%, Ownership lookup 23%, Route decision 14%)
  - Consensus 瓶颈识别 (Version chain 60%, Lock contention 25%, TS allocation 10%)
  - 多核扩展架构设计 (PartitionedFastPath + NUMA-aware)
  - 详细实现清单与基准测试计划

**Files Modified:**
- `src/vm-runtime/Cargo.toml`:
  - 新增 `smallvec = "1.13"` 依赖 (为 MVCC 版本链优化做准备)
  - 已有 `dashmap = "6.1"` (支持无锁并发 HashMap)

**Performance Baseline:**
- FastPath: 28.57M TPS, 35ns latency ✅ (Near-optimal)
- Consensus: 377K TPS, ~2.7μs latency ⚠️ (Optimization target)
- Mixed (80% Fast): 1.20M TPS ✅

**Optimization Targets:**
- Consensus: 377K → **500K TPS** (+33%)
- Multi-Core (8 cores): 28.57M → **180M TPS** (+530%)
- Mixed (80% Fast, 8 cores): 1.20M → **150M TPS** (+12400%)

**ROADMAP Update:**
- L0.5 FastPath 极致优化: 90% → 92% (性能分析完成,优化路径确定)
- L0 整体完成度: 97% → 97.5%

---


### [L0.5 Consensus Path Optimization] SmallVec + Thread-Local TS (2025-11-11)

**Summary:**
- 在 MVCC 共识路径落地两项低风险优化：
  - 引入特性 `smallvec-chains`，将版本链容器从 `Vec<Version>` 抽象为 `VersionChain`，在启用特性时使用 `SmallVec<[Version;4]>`，内联短链以减少堆分配与缓存未命中。
  - 引入特性 `thread-local-ts`，为 `next_ts()` 实现线程本地批量分配（默认批量 128），降低全局 `AtomicU64` 热点争用。

**Files Modified:**
- `src/vm-runtime/src/mvcc.rs`:
  - 新增 `VersionChain` 类型别名并按特性切换 `RwLock<Vec<Version>>` 与 `RwLock<SmallVec<[Version;4]>>`
  - 改造插入路径以在启用 `smallvec-chains` 时初始化 `SmallVec`
  - 为 `next_ts()` 增加线程本地批量实现（特性 `thread-local-ts`）
- `src/vm-runtime/Cargo.toml`:
  - 新增可选特性：`dashmap-mvcc`（占位）、`smallvec-chains`、`thread-local-ts`

**Performance:**
- Consensus 基线（owned_ratio=0.0, 200K txns）：
  - 关闭特性（baseline）：约 392K TPS
  - 启用 `smallvec-chains + thread-local-ts`：约 411K TPS（+4.8%）

### [L0.5 Multi-Core Prototype] PartitionedFastPath 初版 (2025-11-11)

**Summary:**
- 新增 feature `partitioned-fastpath` 与模块 `partitioned_fastpath`：
  - 全局 Injector + N 个工作线程本地队列（每线程一个 Worker）
  - API：`PartitionedFastPath::new(n)`, `submit(FastTask)`, `spawn_workers()`, `stop()`
  - 示例：`examples/partitioned_fast_path_bench.rs`（合成工作负载）

**How to Run:**
- `cargo run -p vm-runtime --example partitioned_fast_path_bench --release --features partitioned-fastpath -- --txs:200000 --partitions:8 --cycles:32`

**Initial Result (synthetic):**
- 8 分区，cycles=32：Executed=200000 Elapsed≈112ms TPS≈1,782,665（合成空转，仅用于验证并发框架吞吐上限）
- 说明：该基准是 CPU 空转模拟，并非真实 VM/共识路径；后续将把真实共识写路径接入分区执行器。

**How to Run:**
- Baseline：
  - `cargo run -p vm-runtime --example mixed_path_bench --release`（环境变量 OWNED_RATIO=0.0）
- With features：
  - `cargo run -p vm-runtime --example mixed_path_bench --release --features smallvec-chains,thread-local-ts`（环境变量 OWNED_RATIO=0.0）

**Notes:**
- 进一步共识路径优化仍在进行：减少写路径锁持有范围、批量提交、按键分区执行器（PartitionedFastPath）。

---

### [L0.5 Consensus Commit Path] Per-Key Lock & Late Conflict Check (2025-11-11)

**Summary:**
- 新增特性 `consensus-optimizations`，对 MVCC 共识提交路径进行“逐键加锁 + 写集合预检 + 提交后快速返回”改造，缩短锁持有时间，减少无效冲突扫描。
- 实现要点：
  - 写集合排序后按键获取独立互斥锁，单键写入后立即释放（替代原始“全键持锁到结束”策略）。
  - 提交前预检最近版本尾部时间戳，发现写写冲突直接中止，避免进入加锁阶段。
  - 分配 `commit_ts` 后在每键写入前做一次“late write-write”校验，确保提交窗口内无竞争覆盖。
  - 保留原路径（未启用特性）以便 A/B 对比和快速回退。

**Files Modified:**
- `src/vm-runtime/src/mvcc.rs`: 增加特性分支、预检循环、逐键加锁写入、移除旧冗余三阶段提交代码（避免 unreachable warning）。
- `src/vm-runtime/Cargo.toml`: 添加 feature `consensus-optimizations`。

**Performance (Pure Consensus Workload, OWNED_RATIO=0.0, 200K txns):**
| 配置 | TPS | 相对提升 |
|------|-----|----------|
| smallvec-chains + thread-local-ts (baseline) | 395K | 基线 |
| + consensus-optimizations | 422K–429K | +6.8% ～ +8.5% |

**Interpretation:**
- 改造后锁持有总时长 ≈ 写入 O(N)（N=写键数）而非“检测+写入”全阶段，大幅降低空等待；在当前低冲突场景下提升有限但稳定。
- 仍有 ~80K TPS 与 500K 目标差距，后续计划：
  - 减少写集合重复版本尾扫描（缓存尾版本 ts）。
  - 可选读集合最小重检（在高并发读写 skew 场景开启）。
  - 结合 PartitionedFastPath 将 Consensus 对象按分片分配锁集合。

**Attempted but Reverted:**
- 尝试加入 `tail_ts` 原子缓存（DashMap<Vec<u8>, AtomicU64>）与线程本地缓冲复用，实测在本机回归到 ~347–357K TPS，判断为哈希查找与额外分配/复制开销抵消了收益，已回滚，保留已验证增益的 last() 尾部检测与逐键加锁。

**ROADMAP Update:** L0.5 单核共识性能推进：97% → 98%（提交路径第一阶段改造完成；回滚不影响已记录增益）。

### [L0.5 Timestamp Allocation] Adaptive thread-local batching (2025-11-11)

**Summary:**
- 增强 `thread-local-ts`：每线程批量在持续耗尽后自适应扩大（128 → 最高 2048），以在高负载/多核场景进一步降低全局原子争用。

**Performance:**
- 当前单核纯共识工作负载（200K txns）下增益为中性（≈422–429K TPS），但预期在多核与更高提交速率时更有帮助。

---


### [L0.2 RocksDB Metrics Integration] RocksDB 内部指标集成 Prometheus (2025-11-XX)

**Summary:**
- **RocksDB 内部指标采集**:
  - 扩展 MetricsCollector 添加 9 个 RocksDB 内部指标字段
  - 实现 RocksDBStorage::collect_metrics() 采集 get_property() 数据
  - 新增 RocksDBMetrics 结构 (cache_hit/miss, compaction, write-stall, SST files, Level 0 files)
  - MvccStore::update_rocksdb_metrics() API 同步指标到 MetricsCollector
- **Prometheus 导出**:
  - export_prometheus() 新增 14 个 RocksDB 指标输出
  - block_cache_hit_rate 命中率计算
  - compaction_cpu_micros / write_stall_micros 延迟监控
  - estimate_num_keys / total_sst_size_bytes 存储统计
- **示例验证**:
  - rocksdb_metrics_demo.rs 演示周期性指标采集
  - stability_test_24h.rs 增强集成 RocksDB 监控输出
  - state_pruning_demo.rs 验证 prune_old_versions() (150 版本清理)

**Files Modified:**
- `src/vm-runtime/src/metrics.rs`:
  - 新增 9 个 AtomicU64 字段 (rocksdb_estimate_num_keys, rocksdb_total_sst_size_bytes, rocksdb_cache_hit/miss, rocksdb_compaction_cpu_micros, rocksdb_compaction_write_bytes, rocksdb_write_stall_micros, rocksdb_num_files_level0, rocksdb_num_immutable_mem_table)
  - export_prometheus() 新增 14 行 RocksDB 指标输出
- `src/vm-runtime/src/storage/rocksdb_storage.rs`:
  - 新增 collect_metrics() -> RocksDBMetrics 方法
  - 新增 RocksDBMetrics 结构定义
- `src/vm-runtime/src/storage.rs`:
  - pub use rocksdb_storage::RocksDBMetrics
- `src/vm-runtime/src/mvcc.rs`:
  - 新增 update_rocksdb_metrics(&RocksDBMetrics) API
- `src/vm-runtime/src/lib.rs`:
  - pub use RocksDBMetrics

**Files Added:**
- `src/vm-runtime/examples/rocksdb_metrics_demo.rs`: RocksDB 指标采集集成示例 (200+ 行)
  - 演示 collect_metrics() / update_rocksdb_metrics() 周期性调用
  - 导出 Prometheus 格式指标到文件
  - 验证 cache_hit_rate / compaction / write-stall 数据正常
- `docs/STORAGE.md`: 存储层完整文档 (500+ 行)
  - RocksDB 配置指南与高级参数调优
  - Checkpoint 管理最佳实践
  - AutoFlush 机制与刷新统计
  - 状态裁剪策略 (版本/时间/区块高度)
  - Prometheus 指标监控与告警阈值
  - 性能调优建议 (写入/读取/Compaction/空间优化)
  - 故障恢复流程与生产环境配置

**Performance Impact:**
- 指标采集开销: ~1-2ms/次 (get_property() 调用)
- 建议采集频率: 10-60 秒/次 (避免性能影响)

**Stability Test Results:**
- TPS: 9667 (目标>5000 ✅, 超出93%)
- 成功率: 100% ✅
- 测试时长: 1分钟压力测试 (580K 事务)
- 无内存泄漏,无写入停顿

**ROADMAP Update:**
- L0.2 存储抽象层: 93% → 95% (稳定性测试+文档完成)
- L0 整体完成度: 96.5% → 97%

---

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
