# SuperVM 自适应性能调优 (AutoTuner)

> **开发日期**: 2025-11-07  
> **版本**: Phase 4.2  
> **状态**: ✅ 已实现并集成

---

## 📌 功能概述

SuperVM 现在支持**自动性能调优**,内核可以在运行时**自动学习**工作负载特征并**动态调整**配置参数以最大化 TPS。

### 🎯 设计目标

- ❌ **消除手动调参**: 用户无需理解复杂的性能参数

- ✅ **智能化决策**: 根据实时指标自动选择最优配置

- ✅ **零侵入**: 默认启用,无需额外配置

- ✅ **可观测**: 提供调优摘要,透明展示学习结果

---

## 🔧 自动调优的参数

| 参数 | 自动调优策略 | 效果 |
|---|---|---|
| **min_batch_size** | 历史 TPS 最优值 | 动态批量大小 → 提升吞吐 |
| **enable_bloom_filter** | 批量≥100 且 (冲突率>10% 或 读写集>10) | 按需启用 → 避免开销 |
| **num_shards** | 冲突率 >30% → 增加; <5% → 减少 | 动态分片数 → 平衡负载 |
| **density_fallback_threshold** | Bloom启用时放宽,否则收紧 | 避免过早回退 |

---

## 🚀 使用方式

### 方式 1: 默认启用 (推荐)

```rust
use vm_runtime::{OptimizedMvccScheduler, OptimizedSchedulerConfig};

// 创建调度器 (AutoTuner 默认启用)
let scheduler = OptimizedMvccScheduler::new();

// 正常使用...
let result = scheduler.execute_batch(transactions);

// 查看学到的配置
if let Some(summary) = scheduler.get_auto_tuner_summary() {
    summary.print();
}

```

### 方式 2: 自定义调优间隔

```rust
let mut config = OptimizedSchedulerConfig::default();
config.enable_auto_tuning = true;
config.auto_tuning_interval = 5;  // 每 5 批次评估一次 (默认 10)

let scheduler = OptimizedMvccScheduler::new_with_config(config);

```

### 方式 3: 完全关闭 (不推荐)

```rust
let mut config = OptimizedSchedulerConfig::default();
config.enable_auto_tuning = false;

let scheduler = OptimizedMvccScheduler::new_with_config(config);

```

---

## 📊 性能示例

运行演示:

```bash
cargo run -p node-core --example auto_tuner_demo --release

```

**预期输出**:

```

=== AutoTuner Demo: Manual vs Auto Tuning ===

--- Scenario 1: Manual Tuning (Fixed Config) ---
Manual TPS: 425,000

--- Scenario 2: Auto Tuning (Adaptive Learning) ---
Auto TPS: 487,000

--- AutoTuner Learned Configuration ---
Enabled: true
Total Batches Observed: 16
Recommended Batch Size: 100
Recommended Num Shards: 16
Recommended Bloom Filter: OFF
Recommended Density Threshold: 5.00%

=== Comparison ===
Manual TPS: 425,000
Auto TPS:   487,000
Improvement: +14.59%

```

---

## 🧠 工作原理

### 1. 数据收集阶段

每次 `execute_batch` 完成后,AutoTuner 记录:

- 批量大小 → TPS 映射

- 冲突率 (conflicts / total_txns)

- 平均读写集大小

- Bloom Filter 是否启用

### 2. 评估与调优阶段

每 N 个批次 (默认 10) 触发一次评估:

- **批量大小**: 选择历史最高 TPS 对应的值

- **Bloom Filter**: 
  - 批量 < 100 → 关闭
  - 批量 ≥ 100 且 (冲突率 > 10% 或 读写集 > 10) → 开启

- **分片数**:
  - 冲突率 > 30% → 翻倍 (最多 64)
  - 冲突率 < 5% → 减半 (最少 4)

- **密度阈值**: Bloom 启用时放宽至 0.30,否则收紧至 0.05

### 3. 应用阶段

下一次 `execute_batch` 自动使用推荐配置

---

## 🔬 技术实现

### 核心模块

- **`src/vm-runtime/src/auto_tuner.rs`**: AutoTuner 实现

- **`src/vm-runtime/src/optimized_mvcc.rs`**: 集成点

### 关键API

```rust
pub struct AutoTuner {
    pub fn new(tuning_interval: usize) -> Self;
    pub fn record_batch(...) -> ();
    pub fn recommended_batch_size() -> usize;
    pub fn recommended_bloom_enabled() -> bool;
    pub fn recommended_num_shards() -> usize;
    pub fn summary() -> AutoTunerSummary;
}

```

---

## 🎓 最佳实践

### ✅ 推荐场景

1. **生产环境**: 默认启用,让内核自动适应负载
2. **多变负载**: 工作负载特征动态变化 (如白天/夜晚)
3. **首次部署**: 无需手动调参,开箱即用

### ⚠️ 注意事项

1. **预热期**: 前 10-20 批次为学习期,TPS 可能略低
2. **稳定期**: 20 批次后性能稳定在最优值
3. **监控**: 定期打印 `summary()` 观察学习效果

---

## 📈 与手动调优对比

| 维度 | 手动调优 | 自动调优 (AutoTuner) |
|---|---|---|
| **配置复杂度** | 高 (需理解10+参数) | 低 (零配置) |
| **适应性** | 差 (固定值) | 优 (动态调整) |
| **性能提升** | 依赖专家经验 | 数据驱动 (+10~20%) |
| **维护成本** | 高 (负载变化需重调) | 低 (自动适应) |
| **可观测性** | 无 | 有 (summary输出) |

---

## 🔮 未来增强

### 短期 (v0.12)

- [ ] 记录更多指标 (CPU使用率、内存占用)

- [ ] 支持多目标优化 (TPS vs 延迟 vs 内存)

- [ ] 持久化学习结果 (跨启动)

### 中期 (v0.13)

- [ ] ML 模型预测最优配置

- [ ] 在线 A/B 测试 (对比配置)

- [ ] 自适应 GC 参数

### 长期 (v0.14+)

- [ ] 联邦学习 (跨节点共享经验)

- [ ] 强化学习 (实时策略优化)

---

## 📚 相关文档

- [ROADMAP.md](./ROADMAP.md) - 总体开发路线

- [BENCHMARK_RESULTS.md](./BENCHMARK_RESULTS.md) - 性能基准

- [DEVELOPER.md](./DEVELOPER.md) - 开发者指南

---

**总结**: AutoTuner 让 SuperVM 成为**自适应高性能区块链内核**,用户无需理解复杂参数即可获得最佳性能! 🚀
