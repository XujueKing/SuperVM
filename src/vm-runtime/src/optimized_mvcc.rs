// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2025 XujueKing <leadbrand@me.com>

//! 优化的 MVCC 并行调度器 (Phase 4.1)
//!
//! 集成布隆过滤器进行快速冲突检测,大幅减少精确冲突检查开销。
//!
//! # 性能优化
//! - **布隆过滤器**: 快速排除无冲突交易 (纳秒级)
//! - **批量提交**: 识别可并行提交的交易组
//! - **冲突预测**: 基于历史冲突模式优化调度
//!
//! # 预期性能提升
//! - 高竞争场景: 85K TPS → 120K+ TPS (提升 41%)
//! - 冲突检测: 减少 80% 的精确检查开销
//! - 误报率: < 1% (布隆过滤器)

use anyhow::Result;
use rayon::prelude::*;
use std::collections::{HashMap, HashSet, VecDeque};
use std::sync::{
    atomic::{AtomicU64, Ordering},
    Arc, Mutex,
};

use crate::bloom_filter::BloomFilterCache;
use crate::mvcc::{GcConfig, MvccStore, Txn};
use crate::parallel_mvcc::{BatchTxnResult, MvccSchedulerStats, TxId, TxnResult};

/// LFU 热键跟踪器
#[derive(Debug)]
struct HotKeyTracker {
    /// 键访问频率统计 (key -> access_count)
    frequencies: HashMap<Vec<u8>, u64>,
    /// 总批次数
    total_batches: u64,
    /// 衰减周期 (每 N 批次衰减一次)
    decay_period: u64,
    /// 衰减因子 (0.0-1.0, 例如 0.9 表示每次衰减保留 90%)
    decay_factor: f64,
}

impl HotKeyTracker {
    fn new(decay_period: u64, decay_factor: f64) -> Self {
        Self {
            frequencies: HashMap::new(),
            total_batches: 0,
            decay_period,
            decay_factor,
        }
    }

    /// 记录一批事务的键访问（仅统计写集，聚焦写写/读写冲突）
    fn record_batch(&mut self, txn_contexts: &[Option<(TxId, &Txn, &Result<i32>)>]) {
        self.total_batches += 1;

        // 统计本批次的键访问
        for ctx_opt in txn_contexts.iter() {
            if let Some((_, txn, _)) = ctx_opt.as_ref() {
                for k in txn.write_set().into_iter() {
                    *self.frequencies.entry(k).or_insert(0) += 1;
                }
            }
        }

        // 周期性衰减
        if self.total_batches.is_multiple_of(self.decay_period) {
            self.decay();
        }
    }

    /// 衰减所有频率计数
    fn decay(&mut self) {
        self.frequencies.retain(|_, count| {
            *count = (*count as f64 * self.decay_factor) as u64;
            *count > 0 // 移除计数为 0 的键
        });
    }

    /// 获取全局热键列表 (频率 >= threshold)
    fn get_hot_keys(&self, threshold: u64) -> HashSet<String> {
        self.frequencies
            .iter()
            .filter(|(_, &count)| count >= threshold)
            .map(|(key, _)| String::from_utf8_lossy(key).to_string())
            .collect()
    }

    /// 获取键的访问频率
    #[allow(dead_code)]
    fn get_frequency(&self, key: &[u8]) -> u64 {
        self.frequencies.get(key).copied().unwrap_or(0)
    }

    /// 清空统计
    #[allow(dead_code)]
    fn clear(&mut self) {
        self.frequencies.clear();
        self.total_batches = 0;
    }
}

/// 优化的调度器配置
#[derive(Debug, Clone)]
pub struct OptimizedSchedulerConfig {
    /// MVCC 配置
    pub mvcc_config: GcConfig,
    /// 最大重试次数
    pub max_retries: u64,
    /// 并行度
    pub num_workers: usize,
    /// 启用布隆过滤器优化
    pub enable_bloom_filter: bool,
    /// 每个交易的预期读写键数量
    pub expected_keys_per_txn: usize,
    /// 布隆过滤器误报率
    pub bloom_false_positive_rate: f64,
    /// 启用批量提交优化
    pub enable_batch_commit: bool,
    /// 批量提交的最小批次大小
    pub min_batch_size: usize,
    /// 自适应 Bloom 阈值：冲突率低于此值时自动禁用 Bloom
    pub adaptive_bloom_disable_threshold: f64,
    /// 自适应 Bloom 阈值：冲突率高于此值时重新启用 Bloom
    pub adaptive_bloom_enable_threshold: f64,
    /// 使用键索引冲突图（替代 O(n^2) 贪心）
    pub use_key_index_grouping: bool,
    /// 启用所有权/分片感知的调度（单分片事务走快路径）
    pub enable_owner_sharding: bool,
    /// 分片数量（用于 owner 映射：hash(key)%num_shards）
    pub num_shards: usize,
    /// 当估算的候选边密度超过阈值时，动态回退（跳过 Bloom 分组）
    pub density_fallback_threshold: f64,
    /// 启用热键隔离：将高频键的事务单独串行处理
    pub enable_hot_key_isolation: bool,
    /// 热键阈值：批内某键的访问次数超过此值则视为热键
    pub hot_key_threshold: usize,
    /// 启用热键分桶并发：将热键事务按键分组，桶内串行、桶间并行
    pub enable_hot_key_bucketing: bool,
    /// 启用自适应热键阈值：根据冲突率/候选密度动态调整阈值
    pub enable_adaptive_hot_key: bool,
    /// 自适应热键阈值下限/上限/步长
    pub hot_key_min: usize,
    pub hot_key_max: usize,
    pub hot_key_step: usize,
    /// 自适应窗口：统计最近 N 个批次的指标
    pub adaptive_window_batches: usize,
    /// 冲突率低/高阈值（平均）
    pub adaptive_conflict_rate_low: f64,
    pub adaptive_conflict_rate_high: f64,
    /// 候选密度低/高阈值（平均）
    pub adaptive_density_low: f64,
    pub adaptive_density_high: f64,
    /// 启用 LFU 热键跟踪：跨批次维护键访问频率
    pub enable_lfu_tracking: bool,
    /// LFU 衰减周期：每 N 批次衰减一次
    pub lfu_decay_period: u64,
    /// LFU 衰减因子：每次衰减保留的比例 (0.0-1.0)
    pub lfu_decay_factor: f64,
    /// LFU 全局热键阈值：全局频率超过此值视为热键
    pub lfu_hot_key_threshold: u64,
    /// LFU 中热键阈值（兼容：lfu_hot_key_threshold 作为中热键阈值使用）
    pub lfu_hot_key_threshold_medium: u64,
    /// LFU 极热键阈值（高于该值的键视为极热键）
    pub lfu_hot_key_threshold_high: u64,
    /// 启用自动性能调优 (AutoTuner)
    pub enable_auto_tuning: bool,
    /// 自动调优评估间隔 (每 N 个批次触发一次)
    pub auto_tuning_interval: usize,
}

impl Default for OptimizedSchedulerConfig {
    fn default() -> Self {
        Self {
            mvcc_config: GcConfig::default(),
            max_retries: 3,
            num_workers: rayon::current_num_threads(),
            enable_bloom_filter: true,
            expected_keys_per_txn: 50,
            bloom_false_positive_rate: 0.01,
            enable_batch_commit: true,
            min_batch_size: 10,
            adaptive_bloom_disable_threshold: 0.05, // 冲突率 < 5% 时禁用（提升禁用阈值）
            adaptive_bloom_enable_threshold: 0.10,  // 冲突率 > 10% 时启用
            use_key_index_grouping: true,           // 默认使用键索引分组
            enable_owner_sharding: true,            // 默认启用所有权感知
            num_shards: 8,                          // 默认 8 分片，可根据核心数调整
            density_fallback_threshold: 0.05,       // 候选边密度 >5% 时回退
            enable_hot_key_isolation: false,        // 热键隔离默认关闭（需测试验证）
            hot_key_threshold: 5,                   // 批内某键访问次数 ≥5 次视为热键
            enable_hot_key_bucketing: false,        // 热键分桶默认关闭
            enable_adaptive_hot_key: false,         // 自适应热键阈值默认关闭
            hot_key_min: 3,
            hot_key_max: 12,
            hot_key_step: 1,
            adaptive_window_batches: 5,
            adaptive_conflict_rate_low: 0.01,  // <1%
            adaptive_conflict_rate_high: 0.05, // >5%
            adaptive_density_low: 0.01,        // <1%
            adaptive_density_high: 0.08,       // >8%
            enable_lfu_tracking: false,        // LFU 跟踪默认关闭
            lfu_decay_period: 10,              // 每 10 批次衰减一次
            lfu_decay_factor: 0.9,             // 衰减后保留 90%
            lfu_hot_key_threshold: 20,         // 兼容字段：作为中热键阈值
            lfu_hot_key_threshold_medium: 20,  // 中热键阈值（降低以展示分层效果）
            lfu_hot_key_threshold_high: 50,    // 极热键阈值（降低以展示分层效果）
            enable_auto_tuning: true,          // 默认启用自动调优
            auto_tuning_interval: 10,          // 每 10 批次评估一次
        }
    }
}

/// 优化的 MVCC 调度器
pub struct OptimizedMvccScheduler {
    /// MVCC 存储引擎
    store: Arc<MvccStore>,
    /// 布隆过滤器缓存
    bloom_cache: Option<Arc<BloomFilterCache>>,
    /// 配置
    config: OptimizedSchedulerConfig,
    /// 统计信息
    stats_successful: Arc<AtomicU64>,
    stats_failed: Arc<AtomicU64>,
    stats_conflict: Arc<AtomicU64>,
    stats_retry: Arc<AtomicU64>,
    stats_bloom_hits: Arc<AtomicU64>,   // 布隆过滤器命中次数
    stats_bloom_misses: Arc<AtomicU64>, // 布隆过滤器误报次数
    /// 运行期自适应开关: 当冲突率极低时自动绕过 Bloom 以避免额外开销
    adaptive_bloom_runtime: Arc<std::sync::atomic::AtomicBool>,
    // --- 诊断指标（跨分片/分组路径）---
    diag_bloom_may_conflict_total: Arc<AtomicU64>,
    diag_bloom_may_conflict_true: Arc<AtomicU64>,
    diag_precise_checks: Arc<AtomicU64>,
    diag_precise_conflicts_added: Arc<AtomicU64>,
    diag_groups_built: Arc<AtomicU64>,
    diag_grouped_txns_total: Arc<AtomicU64>,
    diag_group_max_size: Arc<AtomicU64>,
    diag_candidate_density: Arc<AtomicU64>, // 候选密度 * 10000 (存储为整数)
    /// LFU 热键跟踪器 (可选)
    hot_key_tracker: Option<Arc<Mutex<HotKeyTracker>>>,
    /// 当前生效的热键阈值（支持自适应在线调整）
    current_hot_key_threshold: Arc<std::sync::atomic::AtomicUsize>,
    /// 自适应状态窗口（冲突率、候选密度）
    adaptive_state: Option<std::sync::Mutex<AdaptiveState>>,
    // --- 分层热键诊断 ---
    diag_extreme_hot_count: Arc<AtomicU64>,
    diag_medium_hot_count: Arc<AtomicU64>,
    diag_batch_hot_count: Arc<AtomicU64>,
    // --- 自动调优器 (Phase 4.2) ---
    auto_tuner: Option<Arc<crate::auto_tuner::AutoTuner>>,
}

struct AdaptiveState {
    window: VecDeque<(f64, f64)>, // (conflict_rate, density)
}

impl Default for OptimizedMvccScheduler {
    fn default() -> Self {
        Self::new()
    }
}

impl OptimizedMvccScheduler {
    /// 创建新的优化调度器
    pub fn new() -> Self {
        Self::new_with_config(OptimizedSchedulerConfig::default())
    }

    /// 使用指定配置创建调度器
    pub fn new_with_config(config: OptimizedSchedulerConfig) -> Self {
        let store = MvccStore::new_with_config(config.mvcc_config.clone());

        let bloom_cache = if config.enable_bloom_filter {
            Some(Arc::new(BloomFilterCache::new(
                1000, // 初始容量,会动态增长
                config.expected_keys_per_txn,
                config.bloom_false_positive_rate,
            )))
        } else {
            None
        };

        let hot_key_tracker = if config.enable_lfu_tracking {
            Some(Arc::new(Mutex::new(HotKeyTracker::new(
                config.lfu_decay_period,
                config.lfu_decay_factor,
            ))))
        } else {
            None
        };

        let adaptive_state = if config.enable_adaptive_hot_key {
            Some(std::sync::Mutex::new(AdaptiveState {
                window: std::collections::VecDeque::new(),
            }))
        } else {
            None
        };

        let current_hot_key_threshold = Arc::new(std::sync::atomic::AtomicUsize::new(
            config.hot_key_threshold,
        ));
        let enable_auto_tuning = config.enable_auto_tuning;
        let auto_tuning_interval = config.auto_tuning_interval;

        Self {
            store,
            bloom_cache,
            config,
            stats_successful: Arc::new(AtomicU64::new(0)),
            stats_failed: Arc::new(AtomicU64::new(0)),
            stats_conflict: Arc::new(AtomicU64::new(0)),
            stats_retry: Arc::new(AtomicU64::new(0)),
            stats_bloom_hits: Arc::new(AtomicU64::new(0)),
            stats_bloom_misses: Arc::new(AtomicU64::new(0)),
            adaptive_bloom_runtime: Arc::new(std::sync::atomic::AtomicBool::new(true)),
            diag_bloom_may_conflict_total: Arc::new(AtomicU64::new(0)),
            diag_bloom_may_conflict_true: Arc::new(AtomicU64::new(0)),
            diag_precise_checks: Arc::new(AtomicU64::new(0)),
            diag_precise_conflicts_added: Arc::new(AtomicU64::new(0)),
            diag_groups_built: Arc::new(AtomicU64::new(0)),
            diag_grouped_txns_total: Arc::new(AtomicU64::new(0)),
            diag_group_max_size: Arc::new(AtomicU64::new(0)),
            diag_candidate_density: Arc::new(AtomicU64::new(0)),
            hot_key_tracker,
            current_hot_key_threshold,
            adaptive_state,
            diag_extreme_hot_count: Arc::new(AtomicU64::new(0)),
            diag_medium_hot_count: Arc::new(AtomicU64::new(0)),
            diag_batch_hot_count: Arc::new(AtomicU64::new(0)),
            auto_tuner: if enable_auto_tuning {
                Some(Arc::new(crate::auto_tuner::AutoTuner::new(
                    auto_tuning_interval,
                )))
            } else {
                None
            },
        }
    }

    #[inline]
    fn effective_hot_key_threshold(&self) -> usize {
        self.current_hot_key_threshold.load(Ordering::Relaxed)
    }

    fn post_batch_adaptive_update(&self, pre_conflicts: u64, batch_total: usize) {
        if !self.config.enable_adaptive_hot_key || batch_total == 0 {
            return;
        }

        let post_conflicts = self.stats_conflict.load(Ordering::Relaxed);
        let batch_conflicts = post_conflicts.saturating_sub(pre_conflicts) as f64;
        let conflict_rate = (batch_conflicts / batch_total as f64).clamp(0.0, 1.0);
        let density = (self.diag_candidate_density.load(Ordering::Relaxed) as f64) / 10000.0;

        if let Some(state_mutex) = &self.adaptive_state {
            let mut st = state_mutex.lock().unwrap();
            st.window.push_back((conflict_rate, density));
            while st.window.len() > self.config.adaptive_window_batches {
                st.window.pop_front();
            }
            let (mut c_sum, mut d_sum) = (0.0, 0.0);
            for (c, d) in st.window.iter() {
                c_sum += *c;
                d_sum += *d;
            }
            let n = st.window.len() as f64;
            let c_avg = if n > 0.0 { c_sum / n } else { 0.0 };
            let d_avg = if n > 0.0 { d_sum / n } else { 0.0 };

            let mut thr = self.current_hot_key_threshold.load(Ordering::Relaxed);
            if c_avg >= self.config.adaptive_conflict_rate_high
                || d_avg >= self.config.adaptive_density_high
            {
                // 提高隔离力度：降低阈值
                if thr > self.config.hot_key_min {
                    thr = thr.saturating_sub(self.config.hot_key_step);
                }
            } else if c_avg <= self.config.adaptive_conflict_rate_low
                && d_avg <= self.config.adaptive_density_low
            {
                // 降低隔离力度：提升阈值
                if thr < self.config.hot_key_max {
                    thr = thr.saturating_add(self.config.hot_key_step);
                }
            }
            thr = thr.clamp(self.config.hot_key_min, self.config.hot_key_max);
            self.current_hot_key_threshold.store(thr, Ordering::Relaxed);
        }
    }

    /// 获取底层 MVCC 存储
    pub fn store(&self) -> &Arc<MvccStore> {
        &self.store
    }

    /// 获取自动调优器摘要 (如果启用)
    pub fn get_auto_tuner_summary(&self) -> Option<crate::auto_tuner::AutoTunerSummary> {
        self.auto_tuner.as_ref().map(|t| t.summary())
    }

    /// 获取统计信息
    pub fn get_stats(&self) -> OptimizedSchedulerStats {
        OptimizedSchedulerStats {
            basic: MvccSchedulerStats {
                successful_txs: self.stats_successful.load(Ordering::Relaxed),
                failed_txs: self.stats_failed.load(Ordering::Relaxed),
                conflict_count: self.stats_conflict.load(Ordering::Relaxed),
                retry_count: self.stats_retry.load(Ordering::Relaxed),
            },
            bloom_hits: self.stats_bloom_hits.load(Ordering::Relaxed),
            bloom_misses: self.stats_bloom_misses.load(Ordering::Relaxed),
            bloom_filter_stats: self.bloom_cache.as_ref().map(|c| c.stats()),
            diagnostics: Some(OptimizedDiagnosticsStats {
                bloom_may_conflict_total: self
                    .diag_bloom_may_conflict_total
                    .load(Ordering::Relaxed),
                bloom_may_conflict_true: self.diag_bloom_may_conflict_true.load(Ordering::Relaxed),
                precise_checks: self.diag_precise_checks.load(Ordering::Relaxed),
                precise_conflicts_added: self.diag_precise_conflicts_added.load(Ordering::Relaxed),
                groups_built: self.diag_groups_built.load(Ordering::Relaxed),
                grouped_txns_total: self.diag_grouped_txns_total.load(Ordering::Relaxed),
                group_max_size: self.diag_group_max_size.load(Ordering::Relaxed),
                candidate_density: self.diag_candidate_density.load(Ordering::Relaxed) as f64
                    / 10000.0,
                current_hot_key_threshold: self.effective_hot_key_threshold(),
                adaptive_avg_conflict_rate: self
                    .adaptive_state
                    .as_ref()
                    .map(|m| {
                        let st = m.lock().unwrap();
                        if st.window.is_empty() {
                            0.0
                        } else {
                            st.window.iter().map(|(c, _d)| *c).sum::<f64>() / st.window.len() as f64
                        }
                    })
                    .unwrap_or(0.0),
                adaptive_avg_density: self
                    .adaptive_state
                    .as_ref()
                    .map(|m| {
                        let st = m.lock().unwrap();
                        if st.window.is_empty() {
                            0.0
                        } else {
                            st.window.iter().map(|(_c, d)| *d).sum::<f64>() / st.window.len() as f64
                        }
                    })
                    .unwrap_or(0.0),
                extreme_hot_count: self.diag_extreme_hot_count.load(Ordering::Relaxed),
                medium_hot_count: self.diag_medium_hot_count.load(Ordering::Relaxed),
                batch_hot_count: self.diag_batch_hot_count.load(Ordering::Relaxed),
            }),
        }
    }

    /// 重置统计信息
    pub fn reset_stats(&self) {
        self.stats_successful.store(0, Ordering::Relaxed);
        self.stats_failed.store(0, Ordering::Relaxed);
        self.stats_conflict.store(0, Ordering::Relaxed);
        self.stats_retry.store(0, Ordering::Relaxed);
        self.stats_bloom_hits.store(0, Ordering::Relaxed);
        self.stats_bloom_misses.store(0, Ordering::Relaxed);

        if let Some(cache) = &self.bloom_cache {
            cache.clear();
        }
        // 重置时允许 Bloom，但运行中会根据冲突率自动关闭
        self.adaptive_bloom_runtime.store(true, Ordering::Relaxed);
        // 重置诊断
        self.diag_bloom_may_conflict_total
            .store(0, Ordering::Relaxed);
        self.diag_bloom_may_conflict_true
            .store(0, Ordering::Relaxed);
        self.diag_precise_checks.store(0, Ordering::Relaxed);
        self.diag_precise_conflicts_added
            .store(0, Ordering::Relaxed);
        self.diag_groups_built.store(0, Ordering::Relaxed);
        self.diag_grouped_txns_total.store(0, Ordering::Relaxed);
        self.diag_group_max_size.store(0, Ordering::Relaxed);
        self.diag_candidate_density.store(0, Ordering::Relaxed);
    }

    /// 执行单个交易 (带布隆过滤器优化)
    pub fn execute_txn<F>(&self, tx_id: TxId, f: F) -> TxnResult
    where
        F: Fn(&mut Txn) -> Result<i32>,
    {
        let mut retries = 0;
        let bloom_index = self.bloom_cache.as_ref().map(|cache| cache.allocate_txn());

        loop {
            // 开启新事务
            let mut txn = self.store.begin();

            // 执行业务逻辑
            let result = match f(&mut txn) {
                Ok(value) => value,
                Err(e) => {
                    self.stats_failed.fetch_add(1, Ordering::Relaxed);
                    return TxnResult {
                        tx_id,
                        return_value: None,
                        success: false,
                        error: Some(e.to_string()),
                        commit_ts: None,
                    };
                }
            };

            // 记录本次事务的读写集到布隆缓存（用于后续冲突快速判断）
            if let (Some(cache), Some(idx)) = (&self.bloom_cache, bloom_index) {
                // 记录读集合
                for key in txn.read_set() {
                    cache.record_read(idx, &key);
                }
                // 记录写集合
                for key in txn.write_set() {
                    cache.record_write(idx, &key);
                }
            }

            // 尝试提交
            match txn.commit() {
                Ok(commit_ts) => {
                    self.stats_successful.fetch_add(1, Ordering::Relaxed);
                    if retries > 0 {
                        self.stats_retry.fetch_add(retries, Ordering::Relaxed);
                    }
                    return TxnResult {
                        tx_id,
                        return_value: Some(result),
                        success: true,
                        error: None,
                        commit_ts: Some(commit_ts),
                    };
                }
                Err(e) => {
                    self.stats_conflict.fetch_add(1, Ordering::Relaxed);

                    if retries >= self.config.max_retries {
                        self.stats_failed.fetch_add(1, Ordering::Relaxed);
                        return TxnResult {
                            tx_id,
                            return_value: None,
                            success: false,
                            error: Some(format!("Max retries exceeded: {}", e)),
                            commit_ts: None,
                        };
                    }

                    retries += 1;
                    // 简单的指数退避
                    std::thread::sleep(std::time::Duration::from_micros(1 << retries));
                }
            }
        }
    }

    /// 批量执行交易 (带优化)
    ///
    /// 使用布隆过滤器快速识别可并行执行的交易
    pub fn execute_batch<F>(&self, transactions: Vec<(TxId, F)>) -> BatchTxnResult
    where
        F: Fn(&mut Txn) -> Result<i32> + Send + Sync,
    {
        let start_time = std::time::Instant::now();
        let total_txns = transactions.len();

        // 自适应：若历史上冲突极低，则绕过 Bloom 以避免开销
        let should_use_bloom = if let Some(tuner) = &self.auto_tuner {
            tuner.recommended_bloom_enabled()
        } else {
            self.config.enable_bloom_filter && self.adaptive_bloom_runtime.load(Ordering::Relaxed)
        };

        let result = if self.config.enable_batch_commit
            && transactions.len() >= self.config.min_batch_size
        {
            self.execute_batch_sharding_first(transactions, should_use_bloom)
        } else {
            self.execute_batch_simple(transactions)
        };

        // 记录到 AutoTuner (用于后续调优)
        if let Some(tuner) = &self.auto_tuner {
            let duration = start_time.elapsed().as_secs_f64();
            let conflict_rate = result.conflicts as f64 / total_txns.max(1) as f64;
            // 简化: 假设平均读写集大小 = 预期值 (实际可从 txn context 计算)
            let avg_rw_set = self.config.expected_keys_per_txn;
            let batch_size = self.config.min_batch_size;
            tuner.record_batch(
                batch_size,
                duration,
                total_txns,
                conflict_rate,
                should_use_bloom,
                avg_rw_set,
            );
        }

        result
    }

    /// 批量执行（先分片快路径，再决定是否启用 Bloom 分组）
    fn execute_batch_sharding_first<F>(
        &self,
        transactions: Vec<(TxId, F)>,
        use_bloom: bool,
    ) -> BatchTxnResult
    where
        F: Fn(&mut Txn) -> Result<i32> + Send + Sync,
    {
        // 批次开始前记录冲突计数快照，用于计算本批冲突率（供自适应热键阈值使用）
        let pre_conflicts = self.stats_conflict.load(Ordering::Relaxed);
        // LFU 跟踪：记录本批次访问的键并获取全局热键
        let lfu_hot_keys: HashSet<String> = if let Some(tracker) = &self.hot_key_tracker {
            let mut tracker_guard = tracker.lock().unwrap();
            // 触发周期性衰减
            tracker_guard.decay();
            // 获取当前全局热键
            tracker_guard.get_hot_keys(self.config.lfu_hot_key_threshold)
        } else {
            HashSet::new()
        };

        // 阶段 1: 并行执行所有交易 (不提交)，收集上下文
        let mut txn_contexts: Vec<Option<(TxId, Txn, Result<i32>)>> = transactions
            .into_par_iter()
            .map(|(tx_id, f)| {
                let mut txn = self.store.begin();
                let result = f(&mut txn);
                Some((tx_id, txn, result))
            })
            .collect();

        // LFU 跟踪：记录本批次访问的键
        if let Some(tracker) = &self.hot_key_tracker {
            // 将当前批次上下文转换为只读视图以供记录
            let readonly: Vec<Option<(TxId, &Txn, &Result<i32>)>> = txn_contexts
                .iter()
                .map(|opt| opt.as_ref().map(|(id, txn, res)| (*id, txn, res)))
                .collect();
            let mut tracker_guard = tracker.lock().unwrap();
            tracker_guard.record_batch(&readonly);
        }

        // 阶段 1.5：分片快路径（无论 Bloom 是否启用都执行）
        let mut results: Vec<TxnResult> = Vec::new();
        if self.config.enable_owner_sharding {
            let (single_shard_groups, multi_shard_indices) = self.partition_by_shard(&txn_contexts);

            // 单分片事务：分片间并行提交
            let mut single_shard_ctx: Vec<Vec<(TxId, Txn, Result<i32>)>> = Vec::new();
            for (_shard, indices) in single_shard_groups.into_iter() {
                let mut ctxs = Vec::with_capacity(indices.len());
                for idx in indices {
                    if let Some(ctx) = txn_contexts[idx].take() {
                        ctxs.push(ctx);
                    }
                }
                if !ctxs.is_empty() {
                    single_shard_ctx.push(ctxs);
                }
            }

            // 分片间并行，分片内并行提交
            let mut shard_results: Vec<Vec<TxnResult>> = single_shard_ctx
                .into_par_iter()
                .map(|ctxs| {
                    ctxs.into_par_iter()
                        .map(|(tx_id, txn, result)| match result {
                            Ok(value) => match txn.commit_parallel() {
                                Ok(commit_ts) => {
                                    self.stats_successful.fetch_add(1, Ordering::Relaxed);
                                    TxnResult {
                                        tx_id,
                                        return_value: Some(value),
                                        success: true,
                                        error: None,
                                        commit_ts: Some(commit_ts),
                                    }
                                }
                                Err(e) => {
                                    self.stats_conflict.fetch_add(1, Ordering::Relaxed);
                                    self.stats_failed.fetch_add(1, Ordering::Relaxed);
                                    TxnResult {
                                        tx_id,
                                        return_value: None,
                                        success: false,
                                        error: Some(e.to_string()),
                                        commit_ts: None,
                                    }
                                }
                            },
                            Err(e) => {
                                self.stats_failed.fetch_add(1, Ordering::Relaxed);
                                TxnResult {
                                    tx_id,
                                    return_value: None,
                                    success: false,
                                    error: Some(e.to_string()),
                                    commit_ts: None,
                                }
                            }
                        })
                        .collect()
                })
                .collect();
            for mut v in shard_results.drain(..) {
                results.append(&mut v);
            }

            // 重组剩余跨分片上下文
            let mut remaining_ctx: Vec<Option<(TxId, Txn, Result<i32>)>> =
                Vec::with_capacity(multi_shard_indices.len());
            for idx in multi_shard_indices {
                remaining_ctx.push(txn_contexts[idx].take());
            }

            if remaining_ctx.iter().all(|o| o.is_none()) {
                let successful = results.iter().filter(|r| r.success).count() as u64;
                let failed = results.iter().filter(|r| !r.success).count() as u64;
                let conflicts = self.stats_conflict.load(Ordering::Relaxed);
                // 自适应：更新并返回
                self.post_batch_adaptive_update(pre_conflicts, results.len());
                return BatchTxnResult {
                    successful,
                    failed,
                    conflicts,
                    results,
                };
            }

            // 用剩余上下文继续后续逻辑
            txn_contexts = remaining_ctx;
        }

        // 阶段 2：对剩余跨分片事务，按需进行 Bloom 分组；否则直接并行提交
        let remaining: Vec<(TxId, Txn, Result<i32>)> =
            txn_contexts.into_iter().flatten().collect();

        if remaining.is_empty() {
            let successful = results.iter().filter(|r| r.success).count() as u64;
            let failed = results.iter().filter(|r| !r.success).count() as u64;
            let conflicts = self.stats_conflict.load(Ordering::Relaxed);
            // 自适应：更新并返回
            self.post_batch_adaptive_update(pre_conflicts, results.len());
            return BatchTxnResult {
                successful,
                failed,
                conflicts,
                results,
            };
        }

        if use_bloom {
            if let Some(cache) = &self.bloom_cache {
                let mut ctx_opts: Vec<Option<(TxId, Txn, Result<i32>)>> =
                    remaining.into_iter().map(Some).collect();

                // === 分层热键隔离：极热(串行) / 中热(分桶) / 批次局部热(分桶) ===
                if self.config.enable_hot_key_isolation && self.config.enable_lfu_tracking {
                    // 构建 LFU 中/高阈值集合
                    let (lfu_medium, lfu_high) = if let Some(tracker) = &self.hot_key_tracker {
                        let guard = tracker.lock().unwrap();
                        let med: HashSet<Vec<u8>> = guard
                            .get_hot_keys(self.config.lfu_hot_key_threshold_medium)
                            .into_iter()
                            .map(|s| s.into_bytes())
                            .collect();
                        let high: HashSet<Vec<u8>> = guard
                            .get_hot_keys(self.config.lfu_hot_key_threshold_high)
                            .into_iter()
                            .map(|s| s.into_bytes())
                            .collect();
                        (med, high)
                    } else {
                        (HashSet::new(), HashSet::new())
                    };

                    let (extreme_idx, medium_idx, batch_idx, mut cold_idx) =
                        self.partition_by_hot_keys_tiered(&ctx_opts, &lfu_medium, &lfu_high);

                    // 诊断写入
                    self.diag_extreme_hot_count
                        .store(extreme_idx.len() as u64, Ordering::Relaxed);
                    self.diag_medium_hot_count
                        .store(medium_idx.len() as u64, Ordering::Relaxed);
                    self.diag_batch_hot_count
                        .store(batch_idx.len() as u64, Ordering::Relaxed);

                    use std::sync::Mutex;
                    let ctx_opts_mutex = Mutex::new(ctx_opts);

                    // 1) 极热：串行
                    if !extreme_idx.is_empty() {
                        let extreme_results: Vec<TxnResult> = extreme_idx
                            .into_iter()
                            .filter_map(|i| {
                                let mut g = ctx_opts_mutex.lock().unwrap();
                                g[i].take()
                            })
                            .map(|(tx_id, txn, result)| match result {
                                Ok(value) => match txn.commit_parallel() {
                                    Ok(commit_ts) => {
                                        self.stats_successful.fetch_add(1, Ordering::Relaxed);
                                        TxnResult {
                                            tx_id,
                                            return_value: Some(value),
                                            success: true,
                                            error: None,
                                            commit_ts: Some(commit_ts),
                                        }
                                    }
                                    Err(e) => {
                                        self.stats_conflict.fetch_add(1, Ordering::Relaxed);
                                        self.stats_failed.fetch_add(1, Ordering::Relaxed);
                                        TxnResult {
                                            tx_id,
                                            return_value: None,
                                            success: false,
                                            error: Some(e.to_string()),
                                            commit_ts: None,
                                        }
                                    }
                                },
                                Err(e) => {
                                    self.stats_failed.fetch_add(1, Ordering::Relaxed);
                                    TxnResult {
                                        tx_id,
                                        return_value: None,
                                        success: false,
                                        error: Some(e.to_string()),
                                        commit_ts: None,
                                    }
                                }
                            })
                            .collect();
                        results.extend(extreme_results);
                    }

                    // 2) 中热 + 批次局部热：分桶并发
                let mut merged_bucket_idx = medium_idx;
                merged_bucket_idx.extend(batch_idx);
                    if !merged_bucket_idx.is_empty() {
                        let buckets = self.partition_hot_by_buckets(
                            &ctx_opts_mutex.lock().unwrap(),
                            &merged_bucket_idx,
                        );
                        let results_vec: Vec<Vec<TxnResult>> = buckets
                            .into_par_iter()
                            .map(|(_key, indices)| {
                                let bucket_txns: Vec<(TxId, Txn, Result<i32>)> = {
                                    let mut guard = ctx_opts_mutex.lock().unwrap();
                                    indices
                                        .into_iter()
                                        .filter_map(|i| guard[i].take())
                                        .collect()
                                };
                                bucket_txns
                                    .into_iter()
                                    .map(|(tx_id, txn, result)| match result {
                                        Ok(value) => match txn.commit_parallel() {
                                            Ok(commit_ts) => {
                                                self.stats_successful
                                                    .fetch_add(1, Ordering::Relaxed);
                                                TxnResult {
                                                    tx_id,
                                                    return_value: Some(value),
                                                    success: true,
                                                    error: None,
                                                    commit_ts: Some(commit_ts),
                                                }
                                            }
                                            Err(e) => {
                                                self.stats_conflict.fetch_add(1, Ordering::Relaxed);
                                                self.stats_failed.fetch_add(1, Ordering::Relaxed);
                                                TxnResult {
                                                    tx_id,
                                                    return_value: None,
                                                    success: false,
                                                    error: Some(e.to_string()),
                                                    commit_ts: None,
                                                }
                                            }
                                        },
                                        Err(e) => {
                                            self.stats_failed.fetch_add(1, Ordering::Relaxed);
                                            TxnResult {
                                                tx_id,
                                                return_value: None,
                                                success: false,
                                                error: Some(e.to_string()),
                                                commit_ts: None,
                                            }
                                        }
                                    })
                                    .collect()
                            })
                            .collect();
                        results.extend(results_vec.into_iter().flatten());
                    }

                    // 3) 重建冷事务上下文以继续 Bloom 分组
                    let mut guard = ctx_opts_mutex.lock().unwrap();
                    if cold_idx.is_empty() {
                        let successful = results.iter().filter(|r| r.success).count() as u64;
                        let failed = results.iter().filter(|r| !r.success).count() as u64;
                        let conflicts = self.stats_conflict.load(Ordering::Relaxed);
                        self.post_batch_adaptive_update(pre_conflicts, results.len());
                        return BatchTxnResult {
                            successful,
                            failed,
                            conflicts,
                            results,
                        };
                    }
                    ctx_opts = cold_idx
                        .drain(..)
                        .filter_map(|i| guard[i].take())
                        .map(Some)
                        .collect();
                } else if self.config.enable_hot_key_isolation {
                    // 原有单层热键隔离逻辑（作为回退）
                    let (hot_indices, cold_indices) =
                        self.partition_by_hot_keys(&ctx_opts, &lfu_hot_keys);

                    if !hot_indices.is_empty() {
                        let use_bucketing = self.config.enable_hot_key_bucketing
                            || (self.config.enable_lfu_tracking && !lfu_hot_keys.is_empty());
                        let hot_results: Vec<TxnResult> = if use_bucketing {
                            // 分桶并发处理：按热键分组，桶内串行、桶间并行
                            let buckets = self.partition_hot_by_buckets(&ctx_opts, &hot_indices);
                            use std::sync::Mutex;
                            let ctx_opts_mutex = Mutex::new(ctx_opts);

                            let results_vec: Vec<Vec<TxnResult>> = buckets
                                .into_par_iter()
                                .map(|(_key, indices)| {
                                    // 每个桶内串行处理
                                    let bucket_txns: Vec<(TxId, Txn, Result<i32>)> = {
                                        let mut guard = ctx_opts_mutex.lock().unwrap();
                                        indices
                                            .into_iter()
                                            .filter_map(|i| guard[i].take())
                                            .collect()
                                    };

                                    bucket_txns
                                        .into_iter()
                                        .map(|(tx_id, txn, result)| match result {
                                            Ok(value) => match txn.commit_parallel() {
                                                Ok(commit_ts) => {
                                                    self.stats_successful
                                                        .fetch_add(1, Ordering::Relaxed);
                                                    TxnResult {
                                                        tx_id,
                                                        return_value: Some(value),
                                                        success: true,
                                                        error: None,
                                                        commit_ts: Some(commit_ts),
                                                    }
                                                }
                                                Err(e) => {
                                                    self.stats_conflict
                                                        .fetch_add(1, Ordering::Relaxed);
                                                    self.stats_failed
                                                        .fetch_add(1, Ordering::Relaxed);
                                                    TxnResult {
                                                        tx_id,
                                                        return_value: None,
                                                        success: false,
                                                        error: Some(e.to_string()),
                                                        commit_ts: None,
                                                    }
                                                }
                                            },
                                            Err(e) => {
                                                self.stats_failed.fetch_add(1, Ordering::Relaxed);
                                                TxnResult {
                                                    tx_id,
                                                    return_value: None,
                                                    success: false,
                                                    error: Some(e.to_string()),
                                                    commit_ts: None,
                                                }
                                            }
                                        })
                                        .collect()
                                })
                                .collect();

                            ctx_opts = ctx_opts_mutex.into_inner().unwrap();
                            results_vec.into_iter().flatten().collect()
                        } else {
                            // 全部热键事务串行处理
                            hot_indices
                                .into_iter()
                                .filter_map(|i| ctx_opts[i].take())
                                .map(|(tx_id, txn, result)| match result {
                                    Ok(value) => match txn.commit_parallel() {
                                        Ok(commit_ts) => {
                                            self.stats_successful.fetch_add(1, Ordering::Relaxed);
                                            TxnResult {
                                                tx_id,
                                                return_value: Some(value),
                                                success: true,
                                                error: None,
                                                commit_ts: Some(commit_ts),
                                            }
                                        }
                                        Err(e) => {
                                            self.stats_conflict.fetch_add(1, Ordering::Relaxed);
                                            self.stats_failed.fetch_add(1, Ordering::Relaxed);
                                            TxnResult {
                                                tx_id,
                                                return_value: None,
                                                success: false,
                                                error: Some(e.to_string()),
                                                commit_ts: None,
                                            }
                                        }
                                    },
                                    Err(e) => {
                                        self.stats_failed.fetch_add(1, Ordering::Relaxed);
                                        TxnResult {
                                            tx_id,
                                            return_value: None,
                                            success: false,
                                            error: Some(e.to_string()),
                                            commit_ts: None,
                                        }
                                    }
                                })
                                .collect()
                        };
                        results.extend(hot_results);

                        // 如果所有事务都是热键事务，直接返回
                        if cold_indices.is_empty() {
                            let successful = results.iter().filter(|r| r.success).count() as u64;
                            let failed = results.iter().filter(|r| !r.success).count() as u64;
                            let conflicts = self.stats_conflict.load(Ordering::Relaxed);
                            // 自适应：更新并返回
                            self.post_batch_adaptive_update(pre_conflicts, results.len());
                            return BatchTxnResult {
                                successful,
                                failed,
                                conflicts,
                                results,
                            };
                        }

                        // 重新构建仅包含非热键事务的上下文
                        ctx_opts = cold_indices
                            .into_iter()
                            .filter_map(|i| ctx_opts[i].take())
                            .map(Some)
                            .collect();
                    }
                }

                // 建立 bloom 索引前：估算候选边密度，密度过高则回退（跳过 Bloom 分组）
                let density = self.estimate_candidate_density(&ctx_opts);
                // 记录候选密度诊断值
                self.diag_candidate_density
                    .store((density * 10000.0) as u64, Ordering::Relaxed);
                if density > self.config.density_fallback_threshold {
                    // 回退：直接并行提交剩余事务
                    let more_results: Vec<TxnResult> = ctx_opts
                        .into_par_iter()
                        .filter_map(|o| o)
                        .map(|(tx_id, txn, result)| match result {
                            Ok(value) => match txn.commit_parallel() {
                                Ok(commit_ts) => {
                                    self.stats_successful.fetch_add(1, Ordering::Relaxed);
                                    TxnResult {
                                        tx_id,
                                        return_value: Some(value),
                                        success: true,
                                        error: None,
                                        commit_ts: Some(commit_ts),
                                    }
                                }
                                Err(e) => {
                                    self.stats_conflict.fetch_add(1, Ordering::Relaxed);
                                    self.stats_failed.fetch_add(1, Ordering::Relaxed);
                                    TxnResult {
                                        tx_id,
                                        return_value: None,
                                        success: false,
                                        error: Some(e.to_string()),
                                        commit_ts: None,
                                    }
                                }
                            },
                            Err(e) => {
                                self.stats_failed.fetch_add(1, Ordering::Relaxed);
                                TxnResult {
                                    tx_id,
                                    return_value: None,
                                    success: false,
                                    error: Some(e.to_string()),
                                    commit_ts: None,
                                }
                            }
                        })
                        .collect();
                    results.extend(more_results);

                    let successful = results.iter().filter(|r| r.success).count() as u64;
                    let failed = results.iter().filter(|r| !r.success).count() as u64;
                    let conflicts = self.stats_conflict.load(Ordering::Relaxed);
                    // 自适应：更新并返回
                    self.post_batch_adaptive_update(pre_conflicts, results.len());
                    return BatchTxnResult {
                        successful,
                        failed,
                        conflicts,
                        results,
                    };
                }
                // 建立 bloom 索引并分组（沿用原逻辑）
                let bloom_indices: Vec<usize> =
                    (0..ctx_opts.len()).map(|_| cache.allocate_txn()).collect();
                for (i, ctx_opt) in ctx_opts.iter().enumerate() {
                    if let Some((_, txn, _)) = ctx_opt.as_ref() {
                        for key in txn.read_set() {
                            cache.record_read(bloom_indices[i], &key);
                        }
                        for key in txn.write_set() {
                            cache.record_write(bloom_indices[i], &key);
                        }
                    }
                }
                let groups =
                    self.build_conflict_groups_with_key_index(&ctx_opts, &bloom_indices, cache);

                for group in groups {
                    let mut owned_ctx: Vec<(TxId, Txn, Result<i32>)> =
                        Vec::with_capacity(group.len());
                    for idx in group {
                        if let Some(ctx) = ctx_opts[idx].take() {
                            owned_ctx.push(ctx);
                        }
                    }
                    let group_results: Vec<TxnResult> = owned_ctx
                        .into_par_iter()
                        .map(|(tx_id, txn, result)| match result {
                            Ok(value) => match txn.commit_parallel() {
                                Ok(commit_ts) => {
                                    self.stats_successful.fetch_add(1, Ordering::Relaxed);
                                    TxnResult {
                                        tx_id,
                                        return_value: Some(value),
                                        success: true,
                                        error: None,
                                        commit_ts: Some(commit_ts),
                                    }
                                }
                                Err(e) => {
                                    self.stats_conflict.fetch_add(1, Ordering::Relaxed);
                                    self.stats_failed.fetch_add(1, Ordering::Relaxed);
                                    TxnResult {
                                        tx_id,
                                        return_value: None,
                                        success: false,
                                        error: Some(e.to_string()),
                                        commit_ts: None,
                                    }
                                }
                            },
                            Err(e) => {
                                self.stats_failed.fetch_add(1, Ordering::Relaxed);
                                TxnResult {
                                    tx_id,
                                    return_value: None,
                                    success: false,
                                    error: Some(e.to_string()),
                                    commit_ts: None,
                                }
                            }
                        })
                        .collect();
                    results.extend(group_results);
                }

                let successful = results.iter().filter(|r| r.success).count() as u64;
                let failed = results.iter().filter(|r| !r.success).count() as u64;
                let conflicts = self.stats_conflict.load(Ordering::Relaxed);

                // 自适应开关更新
                let total = successful + failed;
                if total > 0 {
                    let conflict_rate = conflicts as f64 / total as f64;
                    if conflict_rate < self.config.adaptive_bloom_disable_threshold {
                        self.adaptive_bloom_runtime.store(false, Ordering::Relaxed);
                    } else if conflict_rate > self.config.adaptive_bloom_enable_threshold {
                        self.adaptive_bloom_runtime.store(true, Ordering::Relaxed);
                    }
                }

                return BatchTxnResult {
                    successful,
                    failed,
                    conflicts,
                    results,
                };
            }
        }

        // 不使用 Bloom：直接对剩余跨分片事务并行提交
        let more_results: Vec<TxnResult> = remaining
            .into_par_iter()
            .map(|(tx_id, txn, result)| match result {
                Ok(value) => match txn.commit_parallel() {
                    Ok(commit_ts) => {
                        self.stats_successful.fetch_add(1, Ordering::Relaxed);
                        TxnResult {
                            tx_id,
                            return_value: Some(value),
                            success: true,
                            error: None,
                            commit_ts: Some(commit_ts),
                        }
                    }
                    Err(e) => {
                        self.stats_conflict.fetch_add(1, Ordering::Relaxed);
                        self.stats_failed.fetch_add(1, Ordering::Relaxed);
                        TxnResult {
                            tx_id,
                            return_value: None,
                            success: false,
                            error: Some(e.to_string()),
                            commit_ts: None,
                        }
                    }
                },
                Err(e) => {
                    self.stats_failed.fetch_add(1, Ordering::Relaxed);
                    TxnResult {
                        tx_id,
                        return_value: None,
                        success: false,
                        error: Some(e.to_string()),
                        commit_ts: None,
                    }
                }
            })
            .collect();
        results.extend(more_results);

        let successful = results.iter().filter(|r| r.success).count() as u64;
        let failed = results.iter().filter(|r| !r.success).count() as u64;
        let conflicts = self.stats_conflict.load(Ordering::Relaxed);
        // 自适应：更新后返回
        self.post_batch_adaptive_update(pre_conflicts, results.len());
        BatchTxnResult {
            successful,
            failed,
            conflicts,
            results,
        }
    }

    /// 简单批量执行 (无优化)
    fn execute_batch_simple<F>(&self, transactions: Vec<(TxId, F)>) -> BatchTxnResult
    where
        F: Fn(&mut Txn) -> Result<i32> + Send + Sync,
    {
        let results: Vec<TxnResult> = transactions
            .into_par_iter()
            .map(|(tx_id, f)| self.execute_txn(tx_id, f))
            .collect();

        let successful = results.iter().filter(|r| r.success).count() as u64;
        let failed = results.iter().filter(|r| !r.success).count() as u64;
        let conflicts = self.stats_conflict.load(Ordering::Relaxed);

        BatchTxnResult {
            successful,
            failed,
            conflicts,
            results,
        }
    }

    /// 优化的批量执行 (使用键索引冲突图分组)
    #[allow(dead_code)]
    fn execute_batch_optimized<F>(&self, transactions: Vec<(TxId, F)>) -> BatchTxnResult
    where
        F: Fn(&mut Txn) -> Result<i32> + Send + Sync,
    {
        // 阶段 1: 并行执行所有交易 (不提交)
        let mut txn_contexts: Vec<Option<(TxId, Txn, Result<i32>)>> = transactions
            .into_par_iter()
            .map(|(tx_id, f)| {
                let mut txn = self.store.begin();
                let result = f(&mut txn);
                Some((tx_id, txn, result))
            })
            .collect();

        // 可选阶段 1.5：基于所有权/分片的快速路径
        // 将仅触达单一分片的事务分离出来，以绕过 Bloom 与冲突分组开销
        if self.config.enable_owner_sharding {
            let (single_shard_groups, multi_shard_indices) = self.partition_by_shard(&txn_contexts);

            // 先处理单分片事务：不同分片之间可以并行，分片内直接并行提交（依靠每键锁保障安全）
            let mut results: Vec<TxnResult> = Vec::new();

            // 收集并消费单分片上下文
            type ShardContextGroup = Vec<(usize, Vec<(TxId, Txn, Result<i32>)>)>;
            let mut single_shard_ctx: ShardContextGroup = Vec::new();
            for (shard, indices) in single_shard_groups.into_iter() {
                let mut ctxs = Vec::with_capacity(indices.len());
                for idx in indices {
                    if let Some(ctx) = txn_contexts[idx].take() {
                        ctxs.push(ctx);
                    }
                }
                if !ctxs.is_empty() {
                    single_shard_ctx.push((shard, ctxs));
                }
            }

            // 分片间并行；分片内并行提交
            let mut shard_results: Vec<Vec<TxnResult>> = single_shard_ctx
                .into_par_iter()
                .map(|(_shard, ctxs)| {
                    ctxs.into_par_iter()
                        .map(|(tx_id, txn, result)| match result {
                            Ok(value) => match txn.commit_parallel() {
                                Ok(commit_ts) => {
                                    self.stats_successful.fetch_add(1, Ordering::Relaxed);
                                    TxnResult {
                                        tx_id,
                                        return_value: Some(value),
                                        success: true,
                                        error: None,
                                        commit_ts: Some(commit_ts),
                                    }
                                }
                                Err(e) => {
                                    self.stats_conflict.fetch_add(1, Ordering::Relaxed);
                                    self.stats_failed.fetch_add(1, Ordering::Relaxed);
                                    TxnResult {
                                        tx_id,
                                        return_value: None,
                                        success: false,
                                        error: Some(e.to_string()),
                                        commit_ts: None,
                                    }
                                }
                            },
                            Err(e) => {
                                self.stats_failed.fetch_add(1, Ordering::Relaxed);
                                TxnResult {
                                    tx_id,
                                    return_value: None,
                                    success: false,
                                    error: Some(e.to_string()),
                                    commit_ts: None,
                                }
                            }
                        })
                        .collect()
                })
                .collect();

            for mut v in shard_results.drain(..) {
                results.append(&mut v);
            }

            // 对剩余跨分片事务，继续走布隆+分组路径
            // 将未消费的 multi_shard_indices 组装回一个紧凑的 ctx 数组以复用后续逻辑
            let mut remaining_ctx: Vec<Option<(TxId, Txn, Result<i32>)>> =
                Vec::with_capacity(multi_shard_indices.len());
            for idx in multi_shard_indices {
                remaining_ctx.push(txn_contexts[idx].take());
            }

            // 若没有剩余，直接返回
            let remaining_ctx: Vec<Option<(TxId, Txn, Result<i32>)>> =
                remaining_ctx.into_iter().collect();
            if remaining_ctx.iter().all(|o| o.is_none()) {
                let successful = results.iter().filter(|r| r.success).count() as u64;
                let failed = results.iter().filter(|r| !r.success).count() as u64;
                let conflicts = self.stats_conflict.load(Ordering::Relaxed);
                return BatchTxnResult {
                    successful,
                    failed,
                    conflicts,
                    results,
                };
            } else {
                // 将 remaining_ctx 替换回主上下文，继续按 Bloom 分组处理
                txn_contexts = remaining_ctx;
            }
        }

        // 阶段 2: 使用键索引构建冲突图并分组
        if let Some(cache) = &self.bloom_cache {
            // 为每个事务分配一个布隆索引并记录其读写集
            let bloom_indices: Vec<usize> = (0..txn_contexts.len())
                .map(|_| cache.allocate_txn())
                .collect();

            for (i, ctx_opt) in txn_contexts.iter().enumerate() {
                if let Some((_, txn, _)) = ctx_opt.as_ref() {
                    for key in txn.read_set() {
                        cache.record_read(bloom_indices[i], &key);
                    }
                    for key in txn.write_set() {
                        cache.record_write(bloom_indices[i], &key);
                    }
                }
            }

            // 键索引冲突图分组：O(触键数 + 边数) 复杂度
            let groups =
                self.build_conflict_groups_with_key_index(&txn_contexts, &bloom_indices, cache);

            // 阶段 3: 按分组顺序提交（组内并行，组间顺序执行）
            let mut results = Vec::with_capacity(txn_contexts.len());
            for group in groups {
                // 先顺序取出本组的上下文，避免在并行闭包中可变借用 txn_contexts
                let mut owned_ctx: Vec<(TxId, Txn, Result<i32>)> = Vec::with_capacity(group.len());
                for idx in group {
                    if let Some(ctx) = txn_contexts[idx].take() {
                        owned_ctx.push(ctx);
                    }
                }

                let group_results: Vec<TxnResult> = owned_ctx
                    .into_par_iter()
                    .map(|(tx_id, txn, result)| match result {
                        Ok(value) => match txn.commit_parallel() {
                            Ok(commit_ts) => {
                                self.stats_successful.fetch_add(1, Ordering::Relaxed);
                                TxnResult {
                                    tx_id,
                                    return_value: Some(value),
                                    success: true,
                                    error: None,
                                    commit_ts: Some(commit_ts),
                                }
                            }
                            Err(e) => {
                                self.stats_conflict.fetch_add(1, Ordering::Relaxed);
                                self.stats_failed.fetch_add(1, Ordering::Relaxed);
                                TxnResult {
                                    tx_id,
                                    return_value: None,
                                    success: false,
                                    error: Some(e.to_string()),
                                    commit_ts: None,
                                }
                            }
                        },
                        Err(e) => {
                            self.stats_failed.fetch_add(1, Ordering::Relaxed);
                            TxnResult {
                                tx_id,
                                return_value: None,
                                success: false,
                                error: Some(e.to_string()),
                                commit_ts: None,
                            }
                        }
                    })
                    .collect();
                results.extend(group_results);
            }

            let successful = results.iter().filter(|r| r.success).count() as u64;
            let failed = results.iter().filter(|r| !r.success).count() as u64;
            let conflicts = self.stats_conflict.load(Ordering::Relaxed);

            // 自适应 Bloom 开关：根据实测冲突率动态启停
            let total = successful + failed;
            if total > 0 {
                let conflict_rate = conflicts as f64 / total as f64;
                if conflict_rate < self.config.adaptive_bloom_disable_threshold {
                    // 冲突率过低，禁用 Bloom 避免开销
                    self.adaptive_bloom_runtime.store(false, Ordering::Relaxed);
                } else if conflict_rate > self.config.adaptive_bloom_enable_threshold {
                    // 冲突率较高，重新启用 Bloom
                    self.adaptive_bloom_runtime.store(true, Ordering::Relaxed);
                }
            }

            return BatchTxnResult {
                successful,
                failed,
                conflicts,
                results,
            };
        }

        // Fallback: 如果布隆过滤器不可用
        // 若未启用 Bloom，则回退为简单批量执行
        self.execute_batch_simple(
            txn_contexts
                .into_iter()
                .filter_map(|opt| {
                    opt.map(|(id, _txn, result)| {
                        (
                            id,
                            move |_: &mut Txn| if let Ok(v) = result { Ok(v) } else { Ok(0) },
                        )
                    })
                })
                .collect(),
        )
    }

    /// 估算候选边密度（基于键索引对数上界）：用于动态决定是否回退跳过 Bloom 分组
    fn estimate_candidate_density(&self, txn_contexts: &[Option<(TxId, Txn, Result<i32>)>]) -> f64 {
        use std::collections::HashMap;
        let n = txn_contexts.iter().filter(|o| o.is_some()).count();
        if n <= 1 {
            return 0.0;
        }

        let mut key_readers: HashMap<Vec<u8>, usize> = HashMap::new();
        let mut key_writers: HashMap<Vec<u8>, usize> = HashMap::new();
        for ctx_opt in txn_contexts.iter() {
            if let Some((_, txn, _)) = ctx_opt.as_ref() {
                for k in txn.read_set() {
                    *key_readers.entry(k).or_insert(0) += 1;
                }
                for k in txn.write_set() {
                    *key_writers.entry(k).or_insert(0) += 1;
                }
            }
        }
        let mut candidates: u128 = 0;
        let mut add_pairs = |cnt: usize| {
            if cnt >= 2 {
                candidates += (cnt as u128) * ((cnt as u128) - 1) / 2;
            }
        };
        for &cnt in key_readers.values() {
            add_pairs(cnt);
        }
        for &cnt in key_writers.values() {
            add_pairs(cnt);
        }

        let total_pairs: u128 = (n as u128) * ((n as u128) - 1) / 2;
        if total_pairs == 0 {
            return 0.0;
        }
        let ratio = (candidates as f64) / (total_pairs as f64);
        if ratio > 1.0 {
            1.0
        } else {
            ratio
        }
    }

    /// 基于 key 的哈希将事务划分到分片：
    /// - 若一个事务触达的所有键都映射到同一分片 => 返回该分片编号
    /// - 否则 => 归入跨分片集合
    fn partition_by_shard(
        &self,
        txn_contexts: &[Option<(TxId, Txn, Result<i32>)>],
    ) -> (std::collections::HashMap<usize, Vec<usize>>, Vec<usize>) {
        use std::collections::{HashMap, HashSet};
        let mut single_shard: HashMap<usize, Vec<usize>> = HashMap::new();
        let mut multi_shard: Vec<usize> = Vec::new();

        for (i, ctx_opt) in txn_contexts.iter().enumerate() {
            let Some((_, txn, _)) = ctx_opt.as_ref() else {
                continue;
            };

            let mut seen: HashSet<usize> = HashSet::new();
            for k in txn
                .read_set()
                .into_iter()
                .chain(txn.write_set().into_iter())
            {
                let shard = self.key_to_shard(&k);
                seen.insert(shard);
                if seen.len() > 1 {
                    break;
                }
            }
            match seen.len() {
                0 => {
                    multi_shard.push(i);
                }
                1 => {
                    let shard = *seen.iter().next().unwrap();
                    single_shard.entry(shard).or_default().push(i);
                }
                _ => multi_shard.push(i),
            }
        }
        (single_shard, multi_shard)
    }

    /// 分层热键隔离：将事务分为极热/中热/批次热/冷四类
    /// 返回：(极热索引, 中热索引, 批次热索引, 冷索引)
    fn partition_by_hot_keys_tiered(
        &self,
        txn_contexts: &[Option<(TxId, Txn, Result<i32>)>],
        lfu_medium: &HashSet<Vec<u8>>,
        lfu_high: &HashSet<Vec<u8>>,
    ) -> (Vec<usize>, Vec<usize>, Vec<usize>, Vec<usize>) {
        use std::collections::HashMap;
        let mut key_access_count: HashMap<Vec<u8>, usize> = HashMap::new();

        // 仅统计写集，聚焦可引发冲突的键
        for ctx_opt in txn_contexts.iter() {
            if let Some((_, txn, _)) = ctx_opt.as_ref() {
                for k in txn.write_set().into_iter() {
                    *key_access_count.entry(k).or_insert(0) += 1;
                }
            }
        }

        // 批次内局部热键
        let batch_hot_keys: HashSet<Vec<u8>> = key_access_count
            .into_iter()
            .filter(|(_, count)| *count >= self.effective_hot_key_threshold())
            .map(|(key, _)| key)
            .collect();

        let mut extreme = Vec::new();
        let mut medium = Vec::new();
        let mut batch_hot = Vec::new();
        let mut cold = Vec::new();

        for (i, ctx_opt) in txn_contexts.iter().enumerate() {
            if let Some((_, txn, _)) = ctx_opt.as_ref() {
                // 只看写集的热键触碰
                let ws = txn.write_set();
                let touches_high = ws.iter().any(|k| lfu_high.contains(k));
                if touches_high {
                    extreme.push(i);
                    continue;
                }

                let touches_medium = ws.iter().any(|k| lfu_medium.contains(k));
                if touches_medium {
                    medium.push(i);
                    continue;
                }

                let touches_batch = ws.iter().any(|k| batch_hot_keys.contains(k));
                if touches_batch {
                    batch_hot.push(i);
                } else {
                    cold.push(i);
                }
            }
        }

        (extreme, medium, batch_hot, cold)
    }

    /// 热键隔离：识别批内高频键并分离涉及热键的事务
    /// 返回：(热键事务索引列表, 非热键事务索引列表)
    fn partition_by_hot_keys(
        &self,
        txn_contexts: &[Option<(TxId, Txn, Result<i32>)>],
        lfu_hot_keys: &HashSet<String>,
    ) -> (Vec<usize>, Vec<usize>) {
        use std::collections::HashMap;
        let mut key_access_count: HashMap<Vec<u8>, usize> = HashMap::new();

        // 统计每个键的访问次数
        for ctx_opt in txn_contexts.iter() {
            if let Some((_, txn, _)) = ctx_opt.as_ref() {
                for k in txn
                    .read_set()
                    .into_iter()
                    .chain(txn.write_set().into_iter())
                {
                    *key_access_count.entry(k).or_insert(0) += 1;
                }
            }
        }

        // 识别批次内热键(访问次数 >= 阈值)
        let batch_hot_keys: std::collections::HashSet<Vec<u8>> = key_access_count
            .into_iter()
            .filter(|(_, count)| *count >= self.effective_hot_key_threshold())
            .map(|(key, _)| key)
            .collect();

        // 合并 LFU 全局热键和批次内热键
        let mut hot_keys = batch_hot_keys;
        for lfu_key in lfu_hot_keys {
            hot_keys.insert(lfu_key.as_bytes().to_vec());
        }

        if hot_keys.is_empty() {
            // 无热键，全部走非热键路径
            let all_indices: Vec<usize> = (0..txn_contexts.len())
                .filter(|&i| txn_contexts[i].is_some())
                .collect();
            return (Vec::new(), all_indices);
        }

        let mut hot_txns = Vec::new();
        let mut cold_txns = Vec::new();

        for (i, ctx_opt) in txn_contexts.iter().enumerate() {
            if let Some((_, txn, _)) = ctx_opt.as_ref() {
                let touches_hot = txn
                    .read_set()
                    .into_iter()
                    .chain(txn.write_set().into_iter())
                    .any(|k| hot_keys.contains(&k));

                if touches_hot {
                    hot_txns.push(i);
                } else {
                    cold_txns.push(i);
                }
            }
        }

        (hot_txns, cold_txns)
    }

    /// 热键分桶：将热键事务按其访问的热键分配到不同的桶
    /// 返回: HashMap<热键, 访问该热键的事务索引列表>
    fn partition_hot_by_buckets(
        &self,
        txn_contexts: &[Option<(TxId, Txn, Result<i32>)>],
        hot_txn_indices: &[usize],
    ) -> std::collections::HashMap<Vec<u8>, Vec<usize>> {
        use std::collections::{HashMap, HashSet};

        // 先识别所有热键
        let mut key_access_count: HashMap<Vec<u8>, usize> = HashMap::new();
        for &idx in hot_txn_indices {
            if let Some((_, txn, _)) = txn_contexts[idx].as_ref() {
                for k in txn
                    .read_set()
                    .into_iter()
                    .chain(txn.write_set().into_iter())
                {
                    *key_access_count.entry(k).or_insert(0) += 1;
                }
            }
        }

        let hot_keys: HashSet<Vec<u8>> = key_access_count
            .into_iter()
            .filter(|(_, count)| *count >= self.config.hot_key_threshold)
            .map(|(key, _)| key)
            .collect();

        // 将事务分配到各热键桶（一个事务可能访问多个热键，分配给首个热键）
        let mut buckets: HashMap<Vec<u8>, Vec<usize>> = HashMap::new();
        for &idx in hot_txn_indices {
            if let Some((_, txn, _)) = txn_contexts[idx].as_ref() {
                let first_hot_key = txn
                    .read_set()
                    .into_iter()
                    .chain(txn.write_set().into_iter())
                    .find(|k| hot_keys.contains(k));

                if let Some(key) = first_hot_key {
                    buckets.entry(key).or_default().push(idx);
                }
            }
        }

        buckets
    }

    #[inline]
    fn key_to_shard(&self, key: &[u8]) -> usize {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        let mut hasher = DefaultHasher::new();
        key.hash(&mut hasher);
        (hasher.finish() as usize) % self.config.num_shards.max(1)
    }

    /// 使用键索引构建冲突图并分组 (O(触键数 + 冲突边数))
    ///
    /// 算法：
    /// 1. 构建 Key→(读事务列表, 写事务列表) 倒排索引
    /// 2. 对每个 Key，在其关联的事务间添加冲突边（RW/WR/WW）
    /// 3. 使用贪心图着色算法得到可并行批次（同颜色=同批）
    fn build_conflict_groups_with_key_index(
        &self,
        txn_contexts: &[Option<(TxId, Txn, Result<i32>)>],
        bloom_indices: &[usize],
        cache: &BloomFilterCache,
    ) -> Vec<Vec<usize>> {
        use std::collections::{HashMap, HashSet};

        // 步骤 1: 构建键索引 Key→(读集, 写集)
        let mut key_readers: HashMap<Vec<u8>, Vec<usize>> = HashMap::new();
        let mut key_writers: HashMap<Vec<u8>, Vec<usize>> = HashMap::new();

        for (i, ctx_opt) in txn_contexts.iter().enumerate() {
            if let Some((_, txn, _)) = ctx_opt.as_ref() {
                for key in txn.read_set() {
                    key_readers.entry(key.clone()).or_default().push(i);
                }
                for key in txn.write_set() {
                    key_writers.entry(key.clone()).or_default().push(i);
                }
            }
        }

        // 步骤 2: 构建冲突图 (邻接表)
        let n = txn_contexts.len();
        let mut conflicts: Vec<HashSet<usize>> = vec![HashSet::new(); n];

        // 先用 Bloom 快速剪枝：只在 Bloom 提示可能冲突时才精确构建边
        for key_data in [&key_readers, &key_writers] {
            for (key, txns) in key_data.iter() {
                // 同一键的事务：先用 Bloom 过滤
                for &i in txns {
                    for &j in txns {
                        if i >= j {
                            continue;
                        }

                        // Bloom 快速检查
                        self.diag_bloom_may_conflict_total
                            .fetch_add(1, Ordering::Relaxed);
                        if cache.may_conflict(bloom_indices[i], bloom_indices[j]) {
                            self.diag_bloom_may_conflict_true
                                .fetch_add(1, Ordering::Relaxed);
                            // 精确验证 RW/WR/WW 冲突
                            self.diag_precise_checks.fetch_add(1, Ordering::Relaxed);
                            if let (Some((_, tx_i, _)), Some((_, tx_j, _))) =
                                (txn_contexts[i].as_ref(), txn_contexts[j].as_ref())
                            {
                                let r1 = tx_i.read_set();
                                let w1 = tx_i.write_set();
                                let r2 = tx_j.read_set();
                                let w2 = tx_j.write_set();

                                if r1.contains(key) && w2.contains(key)
                                    || w1.contains(key) && r2.contains(key)
                                    || w1.contains(key) && w2.contains(key)
                                {
                                    conflicts[i].insert(j);
                                    conflicts[j].insert(i);
                                    self.stats_bloom_hits.fetch_add(1, Ordering::Relaxed);
                                    self.diag_precise_conflicts_added
                                        .fetch_add(1, Ordering::Relaxed);
                                } else {
                                    self.stats_bloom_misses.fetch_add(1, Ordering::Relaxed);
                                }
                            }
                        }
                    }
                }
            }
        }

        // 步骤 3: 贪心图着色 → 分批次
        let mut colors: Vec<Option<usize>> = vec![None; n];
        let mut color_groups: HashMap<usize, Vec<usize>> = HashMap::new();
        let mut next_color = 0;

        for i in 0..n {
            if colors[i].is_some() {
                continue;
            }

            // 找到邻居已使用的颜色
            let mut used_colors: HashSet<usize> = HashSet::new();
            for &neighbor in &conflicts[i] {
                if let Some(c) = colors[neighbor] {
                    used_colors.insert(c);
                }
            }

            // 分配最小可用颜色
            let assigned_color = (0..next_color)
                .find(|c| !used_colors.contains(c))
                .unwrap_or_else(|| {
                    let c = next_color;
                    next_color += 1;
                    c
                });

            colors[i] = Some(assigned_color);
            color_groups.entry(assigned_color).or_default().push(i);
        }

        // 按颜色（批次）返回分组
        let mut groups: Vec<Vec<usize>> = color_groups.into_values().collect();
        groups.sort_by_key(|g| std::cmp::Reverse(g.len())); // 大组优先

        // 诊断：记录分组数与规模
        let group_count = groups.len() as u64;
        let total_tx = n as u64;
        if group_count > 0 {
            self.diag_groups_built
                .fetch_add(group_count, Ordering::Relaxed);
        }
        self.diag_grouped_txns_total
            .fetch_add(total_tx, Ordering::Relaxed);
        if let Some(max_len) = groups.iter().map(|g| g.len() as u64).max() {
            // 原子地更新最大值
            loop {
                let cur = self.diag_group_max_size.load(Ordering::Relaxed);
                if max_len <= cur {
                    break;
                }
                if self
                    .diag_group_max_size
                    .compare_exchange(cur, max_len, Ordering::Relaxed, Ordering::Relaxed)
                    .is_ok()
                {
                    break;
                }
            }
        }
        groups
    }
}

/// 优化调度器的统计信息
#[derive(Debug, Clone)]
pub struct OptimizedSchedulerStats {
    /// 基础统计
    pub basic: MvccSchedulerStats,
    /// 布隆过滤器命中次数 (成功避免精确检查)
    pub bloom_hits: u64,
    /// 布隆过滤器误报次数
    pub bloom_misses: u64,
    /// 布隆过滤器详细统计
    pub bloom_filter_stats: Option<crate::bloom_filter::BloomFilterCacheStats>,
    /// 诊断指标（可选）
    pub diagnostics: Option<OptimizedDiagnosticsStats>,
}
/// 分组与 Bloom 剪枝的详细诊断指标
#[derive(Debug, Clone)]
pub struct OptimizedDiagnosticsStats {
    pub bloom_may_conflict_total: u64,
    pub bloom_may_conflict_true: u64,
    pub precise_checks: u64,
    pub precise_conflicts_added: u64,
    pub groups_built: u64,
    pub grouped_txns_total: u64,
    pub group_max_size: u64,
    pub candidate_density: f64, // 候选边密度(0.0-1.0)
    // --- 自适应热键阈值诊断 ---
    pub current_hot_key_threshold: usize,
    pub adaptive_avg_conflict_rate: f64,
    pub adaptive_avg_density: f64,
    // --- 分层热键诊断 ---
    pub extreme_hot_count: u64,
    pub medium_hot_count: u64,
    pub batch_hot_count: u64,
}

impl OptimizedSchedulerStats {
    /// 计算布隆过滤器效率
    pub fn bloom_efficiency(&self) -> f64 {
        let total = self.bloom_hits + self.bloom_misses;
        if total == 0 {
            0.0
        } else {
            self.bloom_hits as f64 / total as f64
        }
    }

    /// 打印详细统计
    pub fn print_detailed(&self) {
        println!("=== Optimized MVCC Scheduler Statistics ===");
        println!("Successful transactions: {}", self.basic.successful_txs);
        println!("Failed transactions: {}", self.basic.failed_txs);
        println!("Conflicts: {}", self.basic.conflict_count);
        println!("Retries: {}", self.basic.retry_count);
        println!("Success rate: {:.2}%", self.basic.success_rate() * 100.0);
        println!("Conflict rate: {:.2}%", self.basic.conflict_rate() * 100.0);

        if self.bloom_hits > 0 || self.bloom_misses > 0 {
            println!("\n--- Bloom Filter Performance ---");
            println!("Bloom hits: {}", self.bloom_hits);
            println!("Bloom misses: {}", self.bloom_misses);
            println!("Bloom efficiency: {:.2}%", self.bloom_efficiency() * 100.0);

            if let Some(ref stats) = self.bloom_filter_stats {
                println!("Total txns cached: {}", stats.total_txns);
                println!("Total reads cached: {}", stats.total_reads);
                println!("Total writes cached: {}", stats.total_writes);
                println!("Avg FPR (read): {:.4}", stats.avg_false_positive_rate_read);
                println!(
                    "Avg FPR (write): {:.4}",
                    stats.avg_false_positive_rate_write
                );
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_optimized_scheduler_basic() {
        let scheduler = OptimizedMvccScheduler::new();

        // 执行一些交易
        let result1 = scheduler.execute_txn(1, |txn| {
            txn.write(b"key1".to_vec(), b"value1".to_vec());
            Ok(1)
        });

        let result2 = scheduler.execute_txn(2, |txn| {
            let _ = txn.read(b"key1");
            txn.write(b"key2".to_vec(), b"value2".to_vec());
            Ok(2)
        });

        assert!(result1.success);
        assert!(result2.success);

        let stats = scheduler.get_stats();
        assert_eq!(stats.basic.successful_txs, 2);
    }

    #[test]
    fn test_optimized_batch_execution() {
        let scheduler = OptimizedMvccScheduler::new();

        // 创建一批交易
        let transactions: Vec<_> = (0..10)
            .map(|i| {
                let key = format!("key{}", i);
                (i as u64, move |txn: &mut Txn| {
                    txn.write(key.as_bytes().to_vec(), b"value".to_vec());
                    Ok(i)
                })
            })
            .collect();

        let result = scheduler.execute_batch(transactions);

        assert_eq!(result.successful, 10);
        assert_eq!(result.failed, 0);
    }

    #[test]
    fn test_bloom_filter_optimization() {
        let mut config = OptimizedSchedulerConfig::default();
        config.enable_bloom_filter = true;
        let scheduler = OptimizedMvccScheduler::new_with_config(config);

        // 执行一些有冲突的交易
        for i in 0..20 {
            let _ = scheduler.execute_txn(i, |txn| {
                txn.write(b"shared_key".to_vec(), format!("value{}", i).into_bytes());
                Ok(i as i32)
            });
        }

        let stats = scheduler.get_stats();
        stats.print_detailed();

        assert!(stats.basic.successful_txs > 0);
    }
}
