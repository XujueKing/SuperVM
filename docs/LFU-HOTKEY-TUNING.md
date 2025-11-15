# LFU 与分层热键调优指南

> 日期：2025-11-07  
> 适用组件：`OptimizedMvccScheduler` 热键隔离 / LFU 全局频率跟踪 / 自适应阈值 / 分层执行路径

## 目标与背景

高并发事务执行中，少数高频键会造成严重写冲突。传统批内热键检测仅看到“当前批次”访问模式，无法识别跨批次持续热点；同时全部热键统一处理（串行或分桶）会牺牲并行度。为此实现：
1. 全局 LFU 频率跟踪（跨批次累积 + 衰减）
2. 分层热键分类：极热 (Extreme) / 中热 (Medium) / 批次局部热键 (Batch) / 冷键 (Cold)
3. 分层执行策略：
   - 极热：严格串行（避免级联冲突与缓存抖动）
   - 中热：按键分桶（桶内串行，桶间并行）
   - 批次局部热键：分桶（仅批内频繁）
   - 冷键：Bloom + 普通并行提交
4. 自适应阈值：根据窗口内冲突率与候选密度微调批次热键阈值。

## 关键配置字段概览

| 字段 | 作用 | 建议初始值 | 调优方向 |
|------|------|------------|----------|
| `enable_hot_key_isolation` | 是否启用批次热键隔离 | true | 基准对比可局部关闭|
| `hot_key_threshold` | 批内热键阈值（候选扫描用，亦用于自适应） | 5~10 | 随冲突率动态调优 |
| `enable_lfu_tracking` | 启用跨批次全局频率 | true（需要分层） | 若负载高度均匀可关闭 |
| `lfu_decay_period` | 每 N 批次衰减一次 | 10~30 | 热点持续性强→增大 |
| `lfu_decay_factor` | 衰减保留比例（0-1） | 0.9~0.97 | 热点短暂→降低，持久→升高 |
| `lfu_hot_key_threshold_medium` | LFU 中热键阈值 | 30~60 | 过多中热→升高，捕捉不足→降低 |
| `lfu_hot_key_threshold_high` | LFU 极热键阈值 | 80~150 | 极热过多→升高，极热=0且确有冲突→降低 |
| `enable_hot_key_bucketing` | 批内热键是否按键并发 | true | 若桶内串行开销 > 冲突节省，可关闭 |
| `enable_adaptive_hot_key` | 自适应批内阈值 | 可选：true | 高度动态负载建议开启 |
| `adaptive_window_batches` | 自适应统计窗口大小 | 8~16 | 变化快→减小；噪声大→增大 |

## 分层分类逻辑简述

1. 先从 LFU tracker 中提取满足 `medium` 与 `high` 阈值的全局热键集合（HashSet）。
2. 扫描当前批次事务写集：
   - 写集包含任一 `high` 键 → 事务归类为极热
   - 否则写集包含任一 `medium` 键 → 事务归类为中热
   - 否则统计批内局部频率 >= `hot_key_threshold` 的键 → 事务归类为批次热键
   - 否则归类为冷键
3. 依次执行：极热串行 → 中热桶并发 → 批次热键桶并发 → 冷键 Bloom/普通并行。

## 诊断指标解读 (`OptimizedDiagnosticsStats`)

| 字段 | 含义 | 关注点 |
|------|------|--------|
| `current_hot_key_threshold` | 自适应后生效的批内阈值 | 是否频繁震荡（过度调优） |
| `adaptive_conflict_rate_avg` | 窗口平均冲突率 | 持续高 → 考虑降低阈值或提升极热串行比例 |
| `adaptive_candidate_density_avg` | 候选分组密度 | 高密度但低冲突 → 可能 Bloom 分组收益不大 |
| `extreme_hot_count` | 极热事务数量 | 极热过大 → 检查 `lfu_hot_key_threshold_high` |
| `medium_hot_count` | 中热事务数量 | 若为 0 且存在热点扩散 → 降低 medium 阈值 |
| `batch_hot_count` | 批次局部热键事务数 | 大但冲突率低 → 阈值可升高减少隔离成本 |

## 调优流程推荐

1. 基线对比：关闭 `enable_lfu_tracking` 与热键隔离，记录 TPS/冲突率。  
2. 启用批内热键隔离（仅 `hot_key_threshold`），观察：
   - TPS 是否下降（隔离调度开销）
   - 冲突率是否下降（隔离收益）
3. 启用 LFU：
   - 初始 `lfu_hot_key_threshold_medium=40`, `lfu_hot_key_threshold_high=120`
   - 检查是否产生 `medium` 与 `extreme` 分层；没有则降低 medium 或提高极热阈值使梯度显现。
4. 调整衰减：
   - 若热点“粘性”强（同一键长时间高频）但频率刚好掉到 medium → 增大 `lfu_decay_period` 或提升 `lfu_decay_factor`。
   - 若陈旧热点长期占用极热集合 → 降低 `lfu_decay_factor` 或减小 `lfu_decay_period`。
5. 自适应批内阈值：
   - 开启后观察 `current_hot_key_threshold` 的调节幅度；若频繁跳变且收益不明显，可关闭改为手动设定较稳阈值。
6. 最终收敛：
   - 目标：极热事务占总事务 < 5%，中热占 10%~25%，批次热键隔离不超过 30%，冲突率相较基线显著降低（>15%）。

## 负载设计要点（用于验证分层）

| 手段 | 目的 | 示例 |
|------|------|------|
| 减少热键总数 | 增大单键频率，触发极热 | `NUM_HOT_KEYS=3` 触发 extreme=40 |
| 增加热键总数 | 稀释频率，制造 medium 层 | `NUM_HOT_KEYS=6` 得到 medium=40 extreme=0 |
| 扩大批大小 | 提高单批频率累积效率 | `BATCH_SIZE=40` 而不是 20 |
| 增加总事务数 | 提供跨批累计时间 | `TXNS_PER_THREAD=400` |
| 调整衰减慢速 | 保持历史热点身份 | `decay_period=20, factor=0.95` |

## 示例基准结果摘录

```

HotKey(5) + LFU(10/0.9/50): extreme 40 medium 0 batch 0 (集中极热)
HotKey(5) + LFU + 调整阈值 medium=40 high=120 + NUM_HOT_KEYS=6: extreme 0 medium 40 batch 0 (梯度分层成功)

```

备注：不同工作负载下绝对数值会变化，关注分层比例与冲突率下降趋势。

## 推荐默认值（通用场景）

```rust
lfu_decay_period = 16
lfu_decay_factor = 0.92
lfu_hot_key_threshold_medium = 50
lfu_hot_key_threshold_high = 130
hot_key_threshold (batch) = 6
adaptive_window_batches = 12

```

适合：中等规模（数千事务/秒）且热点键约占总键空间 <2%。

## 常见问题 (FAQ)

1. 中热始终为 0？
   - 可能极热阈值过低或热键太少；提高 `lfu_hot_key_threshold_high` 或增加 `NUM_HOT_KEYS`。
2. 极热事务太多导致串行瓶颈？
   - 升高 `lfu_hot_key_threshold_high`；或降低衰减保留（factor）。
3. Bloom 分组统计密度高但冲突率下降不明显？
   - 可能键访问高度集中且分组成本抵消收益，考虑临时关闭 Bloom。
4. 自适应阈值频繁振荡？
   - 增大 `adaptive_window_batches`，或设定更宽的高低阈值区间避免抖动。
5. 频率无增长，全局集合始终空？
   - 工作负载短暂且衰减过快；减少 `lfu_decay_period` 不会改善，需增大它或提高总事务数。

## 快速检查清单

- [ ] 基线数据已记录 (TPS/冲突率)

- [ ] 启用 LFU 后是否出现 medium/extreme 集合

- [ ] 串行极热事务占比是否可控 (<5%)

- [ ] 分桶中热是否提升并行度（观察TPS变化）

- [ ] 批次热键隔离是否减少重试次数

- [ ] 自适应阈值是否稳定（无剧烈震荡）

## 后续扩展建议

- 引入“写写冲突强度”二级指标：区分高频但互不冲突的键（只读或不同字段）

- 支持周期性导出 LFU 状态用于离线分析与预测预热

- 结合分片信息：不同分片的局部极热键可拆分并行串行队列

---
如需进一步场景落地或自动化调参脚本，可新增脚本：`scripts/tune-lfu-hotkeys.ps1` 收集多轮指标并输出建议配置。
