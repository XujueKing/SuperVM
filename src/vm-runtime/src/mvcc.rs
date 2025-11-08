// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2025 XujueKing <leadbrand@me.com>

use crate::metrics::MetricsCollector;
use dashmap::DashMap;
use parking_lot::RwLock;
use std::collections::HashMap;
use std::sync::{
    atomic::{AtomicBool, AtomicU64, Ordering},
    Arc, Mutex,
};
use std::thread;
use std::time::{Duration, Instant};

/// GC 配置选项
#[derive(Debug, Clone)]
pub struct GcConfig {
    /// 保留版本数量限制（每个键最多保留的版本数）
    pub max_versions_per_key: usize,
    /// 是否启用基于时间的 GC（清理过期版本）
    pub enable_time_based_gc: bool,
    /// 版本过期时间（秒），超过此时间的版本可被清理
    pub version_ttl_secs: u64,
    /// 自动 GC 配置
    pub auto_gc: Option<AutoGcConfig>,
}

/// 自动 GC 配置
#[derive(Debug, Clone)]
pub struct AutoGcConfig {
    /// GC 间隔时间（秒）
    pub interval_secs: u64,
    /// 版本数阈值：超过此数量触发 GC（0 表示不启用）
    pub version_threshold: usize,
    /// 是否在启动时立即执行一次 GC
    pub run_on_start: bool,
    /// 是否启用自适应 GC（根据负载动态调整参数）
    pub enable_adaptive: bool,
}

impl Default for AutoGcConfig {
    fn default() -> Self {
        Self {
            interval_secs: 60,       // 每 60 秒
            version_threshold: 1000, // 超过 1000 个版本
            run_on_start: false,
            enable_adaptive: false,
        }
    }
}

/// 自适应 GC 策略（占位类型，当前实现中未启用动态调整）
#[derive(Debug, Clone)]
pub struct AdaptiveGcStrategy {
    pub base_interval_secs: u64,
    pub min_interval_secs: u64,
    pub max_interval_secs: u64,
    pub base_threshold: usize,
    pub min_threshold: usize,
    pub max_threshold: usize,
}

impl Default for AdaptiveGcStrategy {
    fn default() -> Self {
        Self {
            base_interval_secs: 60,
            min_interval_secs: 10,
            max_interval_secs: 300,
            base_threshold: 1000,
            min_threshold: 500,
            max_threshold: 5000,
        }
    }
}

impl Default for GcConfig {
    fn default() -> Self {
        Self {
            max_versions_per_key: 10, // 默认保留 10 个版本
            enable_time_based_gc: false,
            version_ttl_secs: 3600, // 1小时
            auto_gc: None,          // 默认不启用自动 GC
        }
    }
}

/// 自动刷新配置
#[derive(Debug, Clone)]
pub struct AutoFlushConfig {
    /// 刷新间隔时间（秒），0 表示禁用基于时间的刷新
    pub interval_secs: u64,
    /// 区块数阈值：每 N 个区块触发一次刷新，0 表示禁用基于区块的刷新
    pub blocks_per_flush: u64,
    /// 保留最近多少个版本在内存中
    pub keep_recent_versions: usize,
    /// 是否在启动时立即执行一次刷新
    pub flush_on_start: bool,
}

impl Default for AutoFlushConfig {
    fn default() -> Self {
        Self {
            interval_secs: 10,       // 每 10 秒刷新一次
            blocks_per_flush: 100,   // 每 100 个区块刷新一次
            keep_recent_versions: 3, // 保留最近 3 个版本在内存
            flush_on_start: false,
        }
    }
}

/// MVCC 存储实现（优化版 + GC + 自动 GC）：
/// - 使用 DashMap 实现每键粒度的并发控制，减少全局锁竞争
/// - 每个键的版本链使用 RwLock 保护，允许并发读
/// - 使用 AtomicU64 管理时间戳，避免锁竞争
/// - 提交时仅锁定写集合涉及的键，最小化锁持有范围
/// - 支持垃圾回收，清理不再需要的旧版本
/// - 支持后台自动 GC，定期或按阈值触发
pub struct MvccStore {
    /// 每个 key 的版本链（按 ts 升序存放），使用 RwLock 允许并发读
    data: DashMap<Vec<u8>, RwLock<Vec<Version>>>,
    /// 全局递增时间戳（原子操作，无锁）
    ts: AtomicU64,
    /// 活跃事务的最小 start_ts（水位线）
    /// 用于 GC 决策：低于此时间戳的版本可能被清理
    active_txns: Arc<Mutex<Vec<u64>>>,
    /// GC 配置
    gc_config: Arc<Mutex<GcConfig>>,
    /// GC 统计信息
    gc_stats: Arc<Mutex<GcStats>>,
    /// 自动 GC 运行标志
    auto_gc_running: Arc<AtomicBool>,
    /// 自动 GC 停止信号
    auto_gc_stop: Arc<AtomicBool>,
    /// 近期提交事务计数（用于估算 TPS）
    recent_tx_count: Arc<AtomicU64>,
    /// 近期 GC 清理的版本数
    recent_gc_cleaned: Arc<AtomicU64>,
    /// 自适应 GC 策略
    adaptive_strategy: Arc<Mutex<AdaptiveGcStrategy>>,
    /// 当前运行中的 GC 间隔（秒）
    current_gc_interval_secs: Arc<AtomicU64>,
    /// 当前运行中的版本阈值
    current_gc_threshold: Arc<AtomicU64>,
    /// 全局提交锁（确保多键事务的原子性）
    /// 所有写事务在 commit 时必须持有此锁
    #[allow(dead_code)]
    commit_lock: Arc<Mutex<()>>,
    /// 每键粒度提交锁：用于并行提交时对写集合加锁，避免写写竞争
    key_locks: DashMap<Vec<u8>, Arc<Mutex<()>>>,
    /// 提交中的计数：>0 时阻塞新事务 begin，确保新事务不会看到部分提交
    commit_in_progress: AtomicU64,

    // ===== 自动刷新相关字段 =====
    /// 自动刷新配置
    auto_flush_config: Arc<Mutex<Option<AutoFlushConfig>>>,
    /// 自动刷新运行标志
    auto_flush_running: Arc<AtomicBool>,
    /// 自动刷新停止信号
    auto_flush_stop: Arc<AtomicBool>,
    /// 当前区块号（用于基于区块的刷新）
    current_block: Arc<AtomicU64>,
    /// 刷新统计信息
    flush_stats: Arc<Mutex<FlushStats>>,

    // ===== 性能指标收集器 =====
    /// 性能指标收集器（可选）
    metrics: Option<Arc<MetricsCollector>>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Version {
    pub ts: u64,
    pub value: Option<Vec<u8>>, // None 表示删除
}

/// 刷新统计信息
#[derive(Debug, Clone, Default)]
pub struct FlushStats {
    /// 刷新执行次数
    pub flush_count: u64,
    /// 刷新的键总数
    pub keys_flushed: u64,
    /// 刷新的总字节数
    pub bytes_flushed: usize,
    /// 最后一次刷新时间戳
    pub last_flush_ts: u64,
    /// 最后一次刷新的区块号
    pub last_flush_block: u64,
}

/// GC 统计信息
#[derive(Debug, Clone, Default)]
pub struct GcStats {
    /// GC 执行次数
    pub gc_count: u64,
    /// 清理的版本总数
    pub versions_cleaned: u64,
    /// 清理的键总数
    pub keys_cleaned: u64,
    /// 最后一次 GC 时间戳
    pub last_gc_ts: u64,
}

/// 自动 GC 运行时参数的快照
#[derive(Debug, Clone)]
pub struct AutoGcRuntime {
    pub enable_adaptive: bool,
    pub interval_secs: u64,
    pub version_threshold: usize,
}

impl MvccStore {
    /// 状态裁剪：批量清理每个 key 的历史版本，仅保留最新 keep_versions 个版本
    /// 返回清理的版本数和键数 (cleaned_versions, cleaned_keys)
    #[cfg(feature = "rocksdb-storage")]
    pub fn prune_old_versions(
        &self,
        keep_versions: usize,
        rocksdb: &crate::storage::RocksDBStorage,
    ) -> (u64, u64) {
        let mut total_cleaned = 0u64;
        let mut keys_cleaned = 0u64;
        let mut batch = Vec::new();
        for entry in self.data.iter() {
            let key = entry.key();
            let versions_lock = entry.value();
            let mut versions = versions_lock.write();
            if versions.len() <= keep_versions {
                continue;
            }
            // 需裁剪的历史版本
            let to_remove = versions.len() - keep_versions;
            for v in versions.drain(0..to_remove) {
                if let Some(ref _value) = v.value {
                    // 仅删除有值的历史版本（如有版本号可拼接 key，需按实际 key 设计）
                    // 这里假设 key 格式为 key|ts，实际项目可调整
                    let mut db_key = key.clone();
                    db_key.extend_from_slice(&v.ts.to_be_bytes());
                    batch.push((db_key, None));
                }
                total_cleaned += 1;
            }
            if to_remove > 0 {
                keys_cleaned += 1;
            }
        }
        // 批量删除 RocksDB 历史状态
        if !batch.is_empty() {
            let _ = rocksdb.write_batch(batch);
        }
        (total_cleaned, keys_cleaned)
    }
    pub fn new() -> Arc<Self> {
        Self::new_with_config(GcConfig::default())
    }

    pub fn new_with_config(config: GcConfig) -> Arc<Self> {
        let auto_gc_enabled = config.auto_gc.is_some();

        let store = Arc::new(Self {
            data: DashMap::new(),
            ts: AtomicU64::new(0),
            active_txns: Arc::new(Mutex::new(Vec::new())),
            gc_config: Arc::new(Mutex::new(config.clone())),
            gc_stats: Arc::new(Mutex::new(GcStats::default())),
            auto_gc_running: Arc::new(AtomicBool::new(false)),
            auto_gc_stop: Arc::new(AtomicBool::new(false)),
            recent_tx_count: Arc::new(AtomicU64::new(0)),
            recent_gc_cleaned: Arc::new(AtomicU64::new(0)),
            adaptive_strategy: Arc::new(Mutex::new(AdaptiveGcStrategy::default())),
            current_gc_interval_secs: Arc::new(AtomicU64::new(
                config
                    .auto_gc
                    .as_ref()
                    .map(|c| c.interval_secs)
                    .unwrap_or(0),
            )),
            current_gc_threshold: Arc::new(AtomicU64::new(
                config
                    .auto_gc
                    .as_ref()
                    .map(|c| c.version_threshold as u64)
                    .unwrap_or(0),
            )),
            commit_lock: Arc::new(Mutex::new(())),
            key_locks: DashMap::new(),
            commit_in_progress: AtomicU64::new(0),

            // 自动刷新字段初始化
            auto_flush_config: Arc::new(Mutex::new(None)),
            auto_flush_running: Arc::new(AtomicBool::new(false)),
            auto_flush_stop: Arc::new(AtomicBool::new(false)),
            current_block: Arc::new(AtomicU64::new(0)),
            flush_stats: Arc::new(Mutex::new(FlushStats::default())),

            // 默认启用指标收集器
            metrics: Some(Arc::new(MetricsCollector::new())),
        });

        // 如果配置了自动 GC，启动后台线程
        if auto_gc_enabled {
            let _ = Self::start_auto_gc_internal(Arc::clone(&store));
        }

        store
    }

    /// 开启一个事务，分配 start_ts（快照版本）
    pub fn begin(self: &Arc<Self>) -> Txn {
        // 简单屏障：若有提交进行中，等待其完成，避免新事务看到部分提交
        while self.commit_in_progress.load(Ordering::SeqCst) > 0 {
            std::thread::yield_now();
        }
        let start_ts = self.ts.fetch_add(1, Ordering::SeqCst) + 1;

        // 注册活跃事务
        self.active_txns.lock().unwrap().push(start_ts);

        Txn {
            store: Arc::clone(self),
            start_ts,
            writes: HashMap::new(),
            reads: std::collections::HashSet::new(),
            committed: false,
            read_only: false,
        }
    }

    /// 开启一个只读事务（快速路径）
    ///
    /// 只读事务优化：
    /// - 不维护写集合
    /// - commit() 直接返回，无需冲突检测
    /// - 性能更优，适合大量只读查询场景
    pub fn begin_read_only(self: &Arc<Self>) -> Txn {
        let start_ts = self.ts.fetch_add(1, Ordering::SeqCst) + 1;

        // 注册活跃事务（只读事务也需要注册，防止 GC 清理它可见的版本）
        self.active_txns.lock().unwrap().push(start_ts);

        Txn {
            store: Arc::clone(self),
            start_ts,
            writes: HashMap::new(),
            reads: std::collections::HashSet::new(),
            committed: false,
            read_only: true,
        }
    }

    fn next_ts(&self) -> u64 {
        self.ts.fetch_add(1, Ordering::SeqCst) + 1
    }

    /// 只读接口：按给定 start_ts 查询可见版本（测试/调试辅助）
    /// 使用读锁，允许多个事务并发读取
    pub fn read_at(&self, key: &[u8], start_ts: u64) -> Option<Vec<u8>> {
        self.data.get(key).and_then(|entry| {
            let versions = entry.value().read();
            versions
                .iter()
                .rev()
                .find(|v| v.ts <= start_ts)
                .and_then(|v| v.value.clone())
        })
    }

    /// 注销活跃事务
    fn unregister_txn(&self, start_ts: u64) {
        let mut active = self.active_txns.lock().unwrap();
        if let Some(pos) = active.iter().position(|&ts| ts == start_ts) {
            active.remove(pos);
        }
    }

    /// 获取活跃事务的最小 start_ts（水位线）
    ///
    /// 返回 None 表示没有活跃事务
    pub fn get_min_active_ts(&self) -> Option<u64> {
        let active = self.active_txns.lock().unwrap();
        active.iter().min().copied()
    }

    /// 更新 GC 配置
    pub fn set_gc_config(&self, config: GcConfig) {
        *self.gc_config.lock().unwrap() = config;
    }

    /// 获取 GC 统计信息
    pub fn get_gc_stats(&self) -> GcStats {
        self.gc_stats.lock().unwrap().clone()
    }

    /// 返回自动 GC 的运行时参数（当前 interval 与阈值），若未配置自动 GC 则返回 None
    pub fn get_auto_gc_runtime(&self) -> Option<AutoGcRuntime> {
        let cfg = self.gc_config.lock().unwrap().auto_gc.clone();
        cfg.map(|c| AutoGcRuntime {
            enable_adaptive: c.enable_adaptive,
            interval_secs: self.current_gc_interval_secs.load(Ordering::Relaxed),
            version_threshold: self.current_gc_threshold.load(Ordering::Relaxed) as usize,
        })
    }

    /// 执行垃圾回收
    ///
    /// 清理策略：
    /// 1. 保留每个键的最新版本（无论是否有活跃事务）
    /// 2. 对于有多个版本的键，根据配置清理旧版本：
    ///    - 基于版本数量：超过 max_versions_per_key 的旧版本
    ///    - 基于活跃事务：低于最小活跃事务 start_ts 的版本可被清理
    ///
    /// 返回清理的版本总数
    pub fn gc(&self) -> Result<u64, String> {
        let config = self.gc_config.lock().unwrap().clone();
        let min_active_ts = self.get_min_active_ts();

        let mut total_cleaned = 0u64;
        let mut keys_cleaned = 0u64;

        // 遍历所有键，清理旧版本
        for entry in self.data.iter() {
            let _key = entry.key();
            let versions_lock = entry.value();
            let mut versions = versions_lock.write();

            if versions.len() <= 1 {
                // 只有一个版本，不清理
                continue;
            }

            // 计算需要保留的版本数
            let mut keep_count = versions.len();

            // 策略 1: 基于版本数量限制
            if keep_count > config.max_versions_per_key {
                keep_count = config.max_versions_per_key;
            }

            // 策略 2: 基于活跃事务水位线
            // 如果有活跃事务，保留它们可见的所有版本（ts <= min_active_ts 的最新版本必须保留）
            if let Some(min_ts) = min_active_ts {
                // 找到最老的活跃事务可见的版本索引
                // 活跃事务能看到 ts <= start_ts 的版本，所以要保留第一个 ts <= min_ts 的版本及之后的所有版本
                if let Some(first_visible_idx) = versions.iter().position(|v| v.ts <= min_ts) {
                    let versions_needed = versions.len() - first_visible_idx;
                    if versions_needed > keep_count {
                        keep_count = versions_needed;
                    }
                }
            }

            // 执行清理：保留最后 keep_count 个版本
            let to_remove = versions.len().saturating_sub(keep_count);
            if to_remove > 0 {
                versions.drain(0..to_remove);
                total_cleaned += to_remove as u64;
                keys_cleaned += 1;
            }
        }

        // 更新统计信息
        let mut stats = self.gc_stats.lock().unwrap();
        stats.gc_count += 1;
        stats.versions_cleaned += total_cleaned;
        stats.keys_cleaned += keys_cleaned;
        stats.last_gc_ts = self.ts.load(Ordering::SeqCst);

        Ok(total_cleaned)
    }

    /// 获取存储的总版本数（用于监控）
    pub fn total_versions(&self) -> usize {
        self.data
            .iter()
            .map(|entry| entry.value().read().len())
            .sum()
    }

    /// 获取存储的键数量
    pub fn total_keys(&self) -> usize {
        self.data.len()
    }

    /// 启动自动 GC（内部方法）
    fn start_auto_gc_internal(store: Arc<Self>) -> Result<(), String> {
        // 检查是否已经在运行
        if store.auto_gc_running.swap(true, Ordering::SeqCst) {
            return Err("Auto GC is already running".to_string());
        }

        let config = store.gc_config.lock().unwrap().clone();
        let auto_gc_config = match config.auto_gc {
            Some(cfg) => cfg,
            None => {
                store.auto_gc_running.store(false, Ordering::SeqCst);
                return Err("Auto GC is not configured".to_string());
            }
        };

        // 重置停止信号
        store.auto_gc_stop.store(false, Ordering::SeqCst);

        // 启动后台线程
        let store_clone = Arc::clone(&store);
        thread::spawn(move || {
            let mut threshold = auto_gc_config.version_threshold;
            let adaptive = auto_gc_config.enable_adaptive;
            let mut current_interval = auto_gc_config.interval_secs;
            let _strategy = store_clone.adaptive_strategy.lock().unwrap().clone();

            // 上一次观测值
            let mut last_tx = store_clone.recent_tx_count.load(Ordering::Relaxed);
            let mut last_versions = store_clone.total_versions() as u64;

            // 初始化导出观测值
            store_clone
                .current_gc_interval_secs
                .store(current_interval, Ordering::Relaxed);
            store_clone
                .current_gc_threshold
                .store(threshold as u64, Ordering::Relaxed);

            // 如果配置了启动时运行，先执行一次
            if auto_gc_config.run_on_start {
                if let Ok(cleaned) = store_clone.gc() {
                    store_clone
                        .recent_gc_cleaned
                        .fetch_add(cleaned, Ordering::Relaxed);
                }
            }

            // 循环执行 GC
            while !store_clone.auto_gc_stop.load(Ordering::SeqCst) {
                // 等待间隔时间（可中断）
                for _ in 0..(current_interval * 10) {
                    if store_clone.auto_gc_stop.load(Ordering::SeqCst) {
                        break;
                    }
                    thread::sleep(Duration::from_millis(100));
                }

                if store_clone.auto_gc_stop.load(Ordering::SeqCst) {
                    break;
                }

                // 自适应：根据近期 TPS 与版本增长调整 interval 与 threshold
                if adaptive {
                    let strat = store_clone.adaptive_strategy.lock().unwrap().clone();
                    let tx_now = store_clone.recent_tx_count.load(Ordering::Relaxed);
                    let tx_delta = tx_now.saturating_sub(last_tx);
                    last_tx = tx_now;

                    let versions_now = store_clone.total_versions() as u64;
                    let versions_delta = versions_now.saturating_sub(last_versions);
                    last_versions = versions_now;

                    let cleaned = store_clone.recent_gc_cleaned.swap(0, Ordering::Relaxed);

                    // 简单启发式：
                    // - 高 TPS 或版本增长快 -> 缩短间隔，降低阈值
                    // - 低 TPS 且增长慢且 GC 清理少 -> 拉长间隔，提升阈值
                    let high_load = tx_delta > 1_000 || versions_delta > 5_000;
                    let low_load = tx_delta < 100 && versions_delta < 500 && cleaned < 100;

                    if high_load {
                        current_interval = current_interval
                            .saturating_sub(1)
                            .max(strat.min_interval_secs);
                        threshold = threshold.saturating_sub(100).max(strat.min_threshold);
                    } else if low_load {
                        current_interval = (current_interval + 1).min(strat.max_interval_secs);
                        threshold = (threshold + 100).min(strat.max_threshold);
                    } else {
                        // 回归基线的微调
                        if current_interval > strat.base_interval_secs {
                            current_interval -=
                                (current_interval - strat.base_interval_secs).min(1);
                        } else if current_interval < strat.base_interval_secs {
                            current_interval +=
                                (strat.base_interval_secs - current_interval).min(1);
                        }
                        if threshold > strat.base_threshold {
                            threshold -= (threshold - strat.base_threshold).min(50);
                        } else if threshold < strat.base_threshold {
                            threshold += (strat.base_threshold - threshold).min(50);
                        }
                    }
                    // 刷新观测值
                    store_clone
                        .current_gc_interval_secs
                        .store(current_interval, Ordering::Relaxed);
                    store_clone
                        .current_gc_threshold
                        .store(threshold as u64, Ordering::Relaxed);
                }

                // 检查是否需要触发 GC（threshold=0 表示总是定时执行）
                let should_run = if threshold > 0 {
                    store_clone.total_versions() >= threshold
                } else {
                    true
                };

                if should_run {
                    if let Ok(cleaned) = store_clone.gc() {
                        store_clone
                            .recent_gc_cleaned
                            .fetch_add(cleaned, Ordering::Relaxed);
                    }
                }
            }

            // 标记为已停止
            store_clone.auto_gc_running.store(false, Ordering::SeqCst);
        });

        Ok(())
    }

    /// 启动自动 GC
    ///
    /// 根据配置的 auto_gc 参数启动后台 GC 线程
    ///
    /// # 返回
    /// - `Ok(())`: 成功启动
    /// - `Err(String)`: 启动失败（已在运行或未配置）
    pub fn start_auto_gc(self: &Arc<Self>) -> Result<(), String> {
        Self::start_auto_gc_internal(Arc::clone(self))
    }

    /// 停止自动 GC
    ///
    /// 发送停止信号给后台 GC 线程，线程会在当前 GC 完成后退出
    pub fn stop_auto_gc(&self) {
        self.auto_gc_stop.store(true, Ordering::SeqCst);
    }

    /// 刷新内存中的已提交版本到持久化存储
    ///
    /// 实现方案 A: 两层架构
    /// - MVCC Store 保留在内存中进行并发控制
    /// - 定期将已提交的最新版本刷新到 RocksDB
    /// - 热数据保留策略: 保留最近 K 个版本在内存
    ///
    /// 参数:
    /// - storage: 持久化存储后端 (实现 Storage trait)
    /// - keep_recent_versions: 保留最近多少个版本在内存 (默认 3)
    ///
    /// 返回: (刷新的键数量, 刷新的总字节数)
    pub fn flush_to_storage(
        &self,
        storage: &mut dyn crate::Storage,
        keep_recent_versions: usize,
    ) -> Result<(usize, usize), String> {
        let mut flushed_keys = 0;
        let mut flushed_bytes = 0;
        // 统一聚合为批，后端自行决定是否原子批量处理
        let mut batch: Vec<(Vec<u8>, Option<Vec<u8>>)> = Vec::new();

        // 获取最小活跃事务时间戳（低于此时间戳的版本已被所有活跃事务可见）
        let min_active_ts = self.get_min_active_ts();

        // 遍历所有键，刷新已提交版本
        for entry in self.data.iter() {
            let key = entry.key();
            let mut versions = entry.value().write();

            // 筛选出可以安全刷新的版本:
            // 1. 必须是已提交的版本 (value.is_some())
            // 2. 如果有活跃事务，版本时间戳必须 < min_active_ts (所有活跃事务都能看到)
            // 3. 保留最新的 keep_recent_versions 个版本在内存

            if versions.is_empty() {
                continue;
            }

            // 找到最新的已提交版本
            let latest_committed = versions.iter().rev().find(|v| v.value.is_some());

            if let Some(latest) = latest_committed {
                // 检查是否可以安全刷新
                let can_flush = match min_active_ts {
                    Some(min_ts) => latest.ts < min_ts,
                    None => true, // 没有活跃事务，可以安全刷新
                };

                if can_flush {
                    if let Some(ref value) = latest.value {
                        batch.push((key.clone(), Some(value.clone())));
                        flushed_keys += 1;
                        flushed_bytes += key.len() + value.len();
                    }

                    // 清理旧版本，保留最新的 keep_recent_versions 个
                    if versions.len() > keep_recent_versions {
                        let keep_from = versions.len().saturating_sub(keep_recent_versions);
                        versions.drain(..keep_from);
                    }
                }
            }
        }

        // 尝试批量提交；若后端不支持则逐条写入
        if !batch.is_empty() {
            // 克隆一份用于尝试批量写入；避免所有权被消费后无法回退
            let batch_for_attempt = batch.clone();
            match storage.write_batch_if_supported(batch_for_attempt) {
                Ok(true) => { /* 已批量处理 */ }
                Ok(false) => {
                    // 回退：逐条写入（MemoryStorage 不会走到这里；为未来后端保留路径）
                    for (k, v_opt) in batch.into_iter() {
                        match v_opt {
                            Some(v) => storage.set(&k, &v).map_err(|e| format!("fallback set failed: {}", e))?,
                            None => storage.delete(&k).map_err(|e| format!("fallback delete failed: {}", e))?,
                        }
                    }
                }
                Err(e) => return Err(format!("Failed to flush batch: {}", e)),
            }
        }

        Ok((flushed_keys, flushed_bytes))
    }

    /// 从持久化存储加载数据到 MVCC Store
    ///
    /// 使用场景: 启动时从 RocksDB 恢复状态
    ///
    /// 参数:
    /// - storage: 持久化存储后端
    /// - keys: 要加载的键列表 (None 表示加载所有键)
    ///
    /// 返回: 加载的键数量
    pub fn load_from_storage(
        &self,
        storage: &dyn crate::Storage,
        keys: Option<&[Vec<u8>]>,
    ) -> Result<usize, String> {
        let mut loaded_keys = 0;
        let load_ts = self.next_ts();

        let keys_to_load: Vec<Vec<u8>> = match keys {
            Some(k) => k.to_vec(),
            None => {
                // 如果没有指定键，扫描存储中的所有键
                // 这里需要 Storage trait 支持前缀扫描
                // 暂时返回错误，需要用户提供键列表
                return Err("Loading all keys requires Storage::scan() support".to_string());
            }
        };

        for key in keys_to_load {
            if let Some(value) = storage
                .get(&key)
                .map_err(|e| format!("Failed to load key from storage: {}", e))?
            {
                // 创建新版本
                let version = Version {
                    ts: load_ts,
                    value: Some(value),
                };

                // 插入或更新版本链
                let entry = self
                    .data
                    .entry(key)
                    .or_insert_with(|| RwLock::new(Vec::new()));
                let mut versions = entry.write();
                versions.push(version);

                loaded_keys += 1;
            }
        }

        Ok(loaded_keys)
    }

    // ===== 自动刷新相关方法 =====

    /// 启动自动刷新后台线程
    ///
    /// 需要提供持久化存储实例的引用
    /// 注意: 由于 Storage trait 要求 &mut self，我们需要使用 Arc<Mutex<>> 包装
    pub fn start_auto_flush(
        self: &Arc<Self>,
        config: AutoFlushConfig,
        storage: Arc<Mutex<dyn crate::Storage + Send>>,
    ) -> Result<(), String> {
        // 检查是否已经在运行
        if self.auto_flush_running.load(Ordering::SeqCst) {
            return Err("Auto flush is already running".to_string());
        }

        // 保存配置
        *self.auto_flush_config.lock().unwrap() = Some(config.clone());

        // 设置运行标志
        self.auto_flush_running.store(true, Ordering::SeqCst);
        self.auto_flush_stop.store(false, Ordering::SeqCst);

        // 启动后台线程
        let store = Arc::clone(self);
        thread::spawn(move || {
            Self::auto_flush_thread(store, config, storage);
        });

        Ok(())
    }

    /// 自动刷新后台线程主循环
    fn auto_flush_thread(
        store: Arc<Self>,
        config: AutoFlushConfig,
        storage: Arc<Mutex<dyn crate::Storage + Send>>,
    ) {
        // 启动时刷新（如果配置了）
        if config.flush_on_start {
            if let Ok(mut storage_guard) = storage.lock() {
                if let Ok((keys, bytes)) =
                    store.flush_to_storage(&mut *storage_guard, config.keep_recent_versions)
                {
                    let mut stats = store.flush_stats.lock().unwrap();
                    stats.flush_count += 1;
                    stats.keys_flushed += keys as u64;
                    stats.bytes_flushed += bytes;
                    stats.last_flush_ts = store.ts.load(Ordering::Relaxed);
                    stats.last_flush_block = store.current_block.load(Ordering::Relaxed);
                }
            }
        }

        let mut last_flush_time = std::time::Instant::now();
        let mut last_flush_block = store.current_block.load(Ordering::Relaxed);

        loop {
            // 检查停止信号
            if store.auto_flush_stop.load(Ordering::SeqCst) {
                break;
            }

            let mut should_flush = false;
            let current_block = store.current_block.load(Ordering::Relaxed);

            // 检查是否满足刷新条件
            // 1. 基于时间
            if config.interval_secs > 0 {
                if last_flush_time.elapsed() >= Duration::from_secs(config.interval_secs) {
                    should_flush = true;
                }
            }

            // 2. 基于区块数
            if config.blocks_per_flush > 0 {
                if current_block >= last_flush_block + config.blocks_per_flush {
                    should_flush = true;
                }
            }

            // 执行刷新
            if should_flush {
                if let Ok(mut storage_guard) = storage.lock() {
                    match store.flush_to_storage(&mut *storage_guard, config.keep_recent_versions) {
                        Ok((keys, bytes)) => {
                            let mut stats = store.flush_stats.lock().unwrap();
                            stats.flush_count += 1;
                            stats.keys_flushed += keys as u64;
                            stats.bytes_flushed += bytes;
                            stats.last_flush_ts = store.ts.load(Ordering::Relaxed);
                            stats.last_flush_block = current_block;

                            last_flush_time = std::time::Instant::now();
                            last_flush_block = current_block;
                        }
                        Err(e) => {
                            eprintln!("Auto flush error: {}", e);
                        }
                    }
                }
            }

            // 短暂休眠，避免 CPU 空转
            thread::sleep(Duration::from_millis(100));
        }

        // 清理运行标志
        store.auto_flush_running.store(false, Ordering::SeqCst);
    }

    /// 停止自动刷新
    pub fn stop_auto_flush(&self) {
        self.auto_flush_stop.store(true, Ordering::SeqCst);
    }

    /// 检查自动刷新是否正在运行
    pub fn is_auto_flush_running(&self) -> bool {
        self.auto_flush_running.load(Ordering::SeqCst)
    }

    /// 更新当前区块号（用于基于区块的刷新触发）
    pub fn set_current_block(&self, block: u64) {
        self.current_block.store(block, Ordering::Relaxed);
    }

    /// 获取当前区块号
    pub fn get_current_block(&self) -> u64 {
        self.current_block.load(Ordering::Relaxed)
    }

    /// 获取刷新统计信息
    pub fn get_flush_stats(&self) -> FlushStats {
        self.flush_stats.lock().unwrap().clone()
    }

    /// 获取指标收集器的引用
    pub fn get_metrics(&self) -> Option<Arc<MetricsCollector>> {
        self.metrics.clone()
    }

    /// 禁用指标收集
    pub fn disable_metrics(&mut self) {
        self.metrics = None;
    }

    /// 启用指标收集
    pub fn enable_metrics(&mut self) {
        if self.metrics.is_none() {
            self.metrics = Some(Arc::new(MetricsCollector::new()));
        }
    }

    /// 手动触发一次刷新（用于测试或强制刷新）
    pub fn manual_flush(
        &self,
        storage: &mut dyn crate::Storage,
        keep_recent_versions: usize,
    ) -> Result<(usize, usize), String> {
        let result = self.flush_to_storage(storage, keep_recent_versions)?;

        // 更新统计
        let mut stats = self.flush_stats.lock().unwrap();
        stats.flush_count += 1;
        stats.keys_flushed += result.0 as u64;
        stats.bytes_flushed += result.1;
        stats.last_flush_ts = self.ts.load(Ordering::Relaxed);
        stats.last_flush_block = self.current_block.load(Ordering::Relaxed);

        Ok(result)
    }

    /// 检查自动 GC 是否正在运行
    pub fn is_auto_gc_running(&self) -> bool {
        self.auto_gc_running.load(Ordering::SeqCst)
    }

    /// 更新自动 GC 配置（需要重启才能生效）
    ///
    /// 注意：如果自动 GC 正在运行，需要先 stop_auto_gc()，
    /// 更新配置后再 start_auto_gc()
    pub fn update_auto_gc_config(&self, auto_config: Option<AutoGcConfig>) {
        let mut config = self.gc_config.lock().unwrap();
        config.auto_gc = auto_config;
    }
}

pub struct Txn {
    store: Arc<MvccStore>,
    pub start_ts: u64,
    writes: HashMap<Vec<u8>, Option<Vec<u8>>>,
    /// 读集合：记录所有读取过的键，用于检测 read-write conflict
    reads: std::collections::HashSet<Vec<u8>>,
    committed: bool,
    read_only: bool,
}

impl Txn {
    /// 读取在 start_ts 及以前可见的值
    pub fn read(&mut self, key: &[u8]) -> Option<Vec<u8>> {
        // 记录到读集合（用于后续冲突检测）
        self.reads.insert(key.to_vec());

        // 优先返回当前事务未提交的写（写读自己）
        if let Some(v) = self.writes.get(key) {
            return v.clone();
        }
        self.store.read_at(key, self.start_ts)
    }

    /// 写入（缓存在本地事务中）
    ///
    /// 注意：只读事务调用此方法会 panic
    pub fn write(&mut self, key: Vec<u8>, value: Vec<u8>) {
        if self.read_only {
            panic!("cannot write in read-only transaction");
        }
        self.writes.insert(key, Some(value));
    }

    /// 删除（缓存在本地事务中）
    ///
    /// 注意：只读事务调用此方法会 panic
    pub fn delete(&mut self, key: Vec<u8>) {
        if self.read_only {
            panic!("cannot delete in read-only transaction");
        }
        self.writes.insert(key, None);
    }

    /// 检查是否为只读事务
    pub fn is_read_only(&self) -> bool {
        self.read_only
    }

    /// 提交：
    /// - 分配 commit_ts
    /// - 对每个写入键做写写冲突检测：如果存在 ts > start_ts 的已提交版本，则冲突
    /// - 无冲突则将本地写入附加为新版本
    ///
    /// 优化点：
    /// - 仅对写集合中的键加锁，而非全局锁
    /// - 使用写锁（RwLock::write）保护版本链的修改
    /// - 按键排序后加锁，避免死锁
    /// - **只读事务快速路径**: 直接返回，跳过所有冲突检测和写入
    pub fn commit(mut self) -> Result<u64, String> {
        if self.committed {
            return Err("txn already committed".into());
        }

        let start_time = Instant::now();

        // 记录事务开始 (如果有 metrics)
        if let Some(ref metrics) = self.store.metrics {
            metrics
                .txn_started
                .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        }

        // 只读事务快速路径：直接返回 start_ts，无需任何操作
        if self.read_only {
            self.committed = true;
            // 计数 TPS
            self.store.recent_tx_count.fetch_add(1, Ordering::Relaxed);

            // 记录提交成功
            if let Some(ref metrics) = self.store.metrics {
                metrics
                    .txn_committed
                    .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                metrics.txn_latency.observe(start_time.elapsed());
                // 更新窗口统计（滚动 TPS 与峰值）
                metrics.tps_window(); // 调用以触发窗口刷新
            }

            return Ok(self.start_ts);
        }

        // 细粒度提交锁方案：对写集合内的键按序加锁，避免写写冲突并允许不同键集并行提交
        // 1) 按键排序（字节序）
        // 2) 为每个键获取/创建独立互斥锁，并按序加锁，避免死锁
        // 3) 在持锁状态下执行最终验证与写入，确保原子性

        // 按键排序以避免死锁
        let mut sorted_keys: Vec<_> = self.writes.keys().cloned().collect();
        sorted_keys.sort();

        // 获取各键的互斥锁（Arc clone），随后按顺序加锁
        let mut key_mutexes: Vec<Arc<Mutex<()>>> = Vec::with_capacity(sorted_keys.len());
        for k in &sorted_keys {
            let lock_arc = self
                .store
                .key_locks
                .entry(k.clone())
                .or_insert_with(|| Arc::new(Mutex::new(())))
                .clone();
            key_mutexes.push(lock_arc);
        }

        // 按排序顺序逐个加锁，持有到提交结束
        let mut _guards = Vec::with_capacity(key_mutexes.len());
        for m in &key_mutexes {
            _guards.push(m.lock().unwrap());
        }

        // 为本次提交分配提交时间戳（在持锁之后分配，保证随后的检测与写入一致）
        let commit_ts = self.store.next_ts();

        // **三阶段提交**：检测读冲突 → 检测写冲突 → 写入
        // 阶段0：检测读集合中**所有键**是否被修改（包括写集合里的键！）
        // 这是防止 Write Skew 的关键：事务基于旧读值计算新写值时，如果读值已过期则拒绝
        for key in &self.reads {
            if let Some(entry) = self.store.data.get(key) {
                let versions = entry.value().read();
                if versions.iter().rev().any(|v| v.ts > self.start_ts) {
                    // 记录中止
                    if let Some(ref metrics) = self.store.metrics {
                        metrics
                            .txn_aborted
                            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                    }
                    return Err(format!(
                        "read-write conflict on key {:?}",
                        String::from_utf8_lossy(key)
                    ));
                }
            }
        }

        // 阶段1：检测写集合的 write-write conflict（冗余检查，但保持一致性）
        // 注意：写集合中的键已经在阶段0检测过读写冲突，这里再检测一次写写冲突
        for key in &sorted_keys {
            if let Some(entry) = self.store.data.get(key) {
                let versions = entry.value().read();
                if versions.iter().rev().any(|v| v.ts > self.start_ts) {
                    // 记录中止
                    if let Some(ref metrics) = self.store.metrics {
                        metrics
                            .txn_aborted
                            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                    }
                    return Err(format!(
                        "write-write conflict on key {:?}",
                        String::from_utf8_lossy(key)
                    ));
                }
            }
        }

        // 阶段2：写入新版本（在持有每键锁的情况下，确保原子性）
        for key in &sorted_keys {
            let entry = self
                .store
                .data
                .entry(key.clone())
                .or_insert_with(|| RwLock::new(Vec::new()));

            let mut versions = entry.value().write();
            let value = self.writes.get(key).unwrap().clone();
            versions.push(Version {
                ts: commit_ts,
                value,
            });
        }

        self.committed = true;
        // 计数 TPS
        self.store.recent_tx_count.fetch_add(1, Ordering::Relaxed);

        // 记录提交成功和延迟
        if let Some(ref metrics) = self.store.metrics {
            metrics
                .txn_committed
                .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
            metrics.txn_latency.observe(start_time.elapsed());
            metrics.tps_window();
        }

        Ok(commit_ts)
        // 全局提交锁在此处自动释放
    }

    /// 放弃事务（丢弃本地写集合）
    pub fn abort(self) {
        // 记录中止
        if let Some(ref metrics) = self.store.metrics {
            metrics
                .txn_aborted
                .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        }
        // Drop trait 会自动注销
    }
}

impl Txn {
    /// 获取事务的读集合（键集合，克隆返回，避免借用生命周期问题）
    /// 用于调度器在执行后期统计和冲突快速判断（Bloom 记录）
    pub fn read_set(&self) -> std::collections::HashSet<Vec<u8>> {
        self.reads.clone()
    }

    /// 获取事务的写集合（仅返回键集合，克隆返回）
    /// 调度器仅需要键即可用于冲突快速判断
    pub fn write_set(&self) -> std::collections::HashSet<Vec<u8>> {
        self.writes.keys().cloned().collect()
    }

    /// 并行提交版本：
    /// - 不使用全局提交锁；使用每键锁 + 提交屏障，允许不同键集合的事务并发提交
    /// - 提交开始前自增 commit_in_progress，阻止新 begin；结束后自减
    /// - 仍按键排序获取锁，避免死锁
    pub fn commit_parallel(mut self) -> Result<u64, String> {
        if self.committed {
            return Err("txn already committed".into());
        }

        let start_time = Instant::now();

        // 记录事务开始
        if let Some(ref metrics) = self.store.metrics {
            metrics
                .txn_started
                .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        }

        // 只读事务快速路径
        if self.read_only {
            self.committed = true;
            self.store.recent_tx_count.fetch_add(1, Ordering::Relaxed);

            // 记录提交成功
            if let Some(ref metrics) = self.store.metrics {
                metrics
                    .txn_committed
                    .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                metrics.txn_latency.observe(start_time.elapsed());
                metrics.tps_window();
            }

            return Ok(self.start_ts);
        }

        // 标记有提交进行中，阻止新 begin
        self.store.commit_in_progress.fetch_add(1, Ordering::SeqCst);

        // 排序写键并获取每键锁
        let mut sorted_keys: Vec<_> = self.writes.keys().cloned().collect();
        sorted_keys.sort();

        // 先收集锁的 Arc，确保后续 guard 的借用生命周期统一
        let mut lock_arcs: Vec<Arc<Mutex<()>>> = Vec::with_capacity(sorted_keys.len());
        for key in &sorted_keys {
            let arc = self
                .store
                .key_locks
                .entry(key.clone())
                .or_insert_with(|| Arc::new(Mutex::new(())))
                .clone();
            lock_arcs.push(arc);
        }
        // 然后获取所有锁的 guard，保持到提交结束
        let mut _key_guards: Vec<std::sync::MutexGuard<'_, ()>> =
            Vec::with_capacity(lock_arcs.len());
        for arc in &lock_arcs {
            _key_guards.push(arc.lock().unwrap());
        }

        // 验证阶段（无全局锁）：读冲突与写写冲突检查
        for key in &self.reads {
            if let Some(entry) = self.store.data.get(key) {
                let versions = entry.value().read();
                if versions.iter().rev().any(|v| v.ts > self.start_ts) {
                    // 记录中止
                    if let Some(ref metrics) = self.store.metrics {
                        metrics
                            .txn_aborted
                            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                    }
                    // 释放提交屏障
                    self.store.commit_in_progress.fetch_sub(1, Ordering::SeqCst);
                    return Err(format!(
                        "read-write conflict on key {:?}",
                        String::from_utf8_lossy(key)
                    ));
                }
            }
        }
        for key in &sorted_keys {
            if let Some(entry) = self.store.data.get(key) {
                let versions = entry.value().read();
                if versions.iter().rev().any(|v| v.ts > self.start_ts) {
                    // 记录中止
                    if let Some(ref metrics) = self.store.metrics {
                        metrics
                            .txn_aborted
                            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                    }
                    self.store.commit_in_progress.fetch_sub(1, Ordering::SeqCst);
                    return Err(format!(
                        "write-write conflict on key {:?}",
                        String::from_utf8_lossy(key)
                    ));
                }
            }
        }

        // 分配提交 ts 并写入
        let commit_ts = self.store.next_ts();
        for key in &sorted_keys {
            let entry = self
                .store
                .data
                .entry(key.clone())
                .or_insert_with(|| RwLock::new(Vec::new()));
            let mut versions = entry.value().write();
            let value = self.writes.get(key).unwrap().clone();
            versions.push(Version {
                ts: commit_ts,
                value,
            });
        }

        self.committed = true;
        self.store.recent_tx_count.fetch_add(1, Ordering::Relaxed);

        // 记录提交成功和延迟
        if let Some(ref metrics) = self.store.metrics {
            metrics
                .txn_committed
                .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
            metrics.txn_latency.observe(start_time.elapsed());
            metrics.tps_window();
        }

        // 提交结束，解除屏障
        self.store.commit_in_progress.fetch_sub(1, Ordering::SeqCst);
        Ok(commit_ts)
    }
}

/// 在事务结束时自动注销活跃事务
impl Drop for Txn {
    fn drop(&mut self) {
        self.store.unregister_txn(self.start_ts);
    }
}

/// MvccStore 销毁时自动停止 GC 线程
impl Drop for MvccStore {
    fn drop(&mut self) {
        self.stop_auto_gc();
        // 等待 GC 线程退出（最多等待 2 秒）
        for _ in 0..20 {
            if !self.is_auto_gc_running() {
                break;
            }
            thread::sleep(Duration::from_millis(100));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mvcc_write_write_conflict() {
        let store = MvccStore::new();

        // T1 写 key1 并提交
        let mut t1 = store.begin();
        t1.write(b"key1".to_vec(), b"A".to_vec());
        let _c1 = t1.commit().expect("t1 commit ok");

        // T2 在 T1 之后开始，但 start_ts < T1 提交的 ts 不成立（因为 begin 也递增），
        // 为了制造冲突：让 T2 在 T1 之前开始，再由 T1 先提交。我们改为：
        let store = MvccStore::new();
        let mut t2 = store.begin(); // start_ts = 1
        let mut t1 = store.begin(); // start_ts = 2

        t1.write(b"key1".to_vec(), b"A".to_vec());
        let _ = t1.commit().unwrap();

        t2.write(b"key1".to_vec(), b"B".to_vec());
        let e = t2.commit().expect_err("t2 should conflict");
        assert!(e.contains("write-write conflict"));
    }

    #[test]
    fn test_mvcc_snapshot_isolation_visibility() {
        let store = MvccStore::new();

        // 初始写入：T0 提交 V0
        let mut t0 = store.begin();
        t0.write(b"k".to_vec(), b"v0".to_vec());
        let ts0 = t0.commit().unwrap();

        // T1 在看到 ts0 后开始，读取应为 v0
        let mut t1 = store.begin();
        assert_eq!(t1.read(b"k").as_deref(), Some(b"v0".as_ref()));

        // T2 先开始（拿到更早的 start_ts），此时读取仍是 v0
        let mut t2 = store.begin();
        assert_eq!(t2.read(b"k").as_deref(), Some(b"v0".as_ref()));

        // T3 写入 v1 并提交
        let mut t3 = store.begin();
        t3.write(b"k".to_vec(), b"v1".to_vec());
        let _ts3 = t3.commit().unwrap();

        // 在 T3 提交后：
        // - T1、T2 由于 start_ts 更早，仍应看到 v0（快照隔离）
        assert_eq!(t1.read(b"k").as_deref(), Some(b"v0".as_ref()));
        assert_eq!(t2.read(b"k").as_deref(), Some(b"v0".as_ref()));

        // 新开 T4，应看到最新 v1
        let mut t4 = store.begin();
        assert_eq!(t4.read(b"k").as_deref(), Some(b"v1".as_ref()));

        // 直接用读接口校验不同时间点的可见性
        assert_eq!(store.read_at(b"k", ts0).as_deref(), Some(b"v0".as_ref()));
    }

    #[test]
    fn test_mvcc_version_visibility_multiple_versions() {
        let store = MvccStore::new();

        // v1
        let mut t1 = store.begin();
        t1.write(b"k".to_vec(), b"v1".to_vec());
        let ts1 = t1.commit().unwrap();

        // v2
        let mut t2 = store.begin();
        t2.write(b"k".to_vec(), b"v2".to_vec());
        let ts2 = t2.commit().unwrap();

        // v3 (删除)
        let mut t3 = store.begin();
        t3.delete(b"k".to_vec());
        let ts3 = t3.commit().unwrap();

        // 不同快照读取
        assert_eq!(store.read_at(b"k", ts1).as_deref(), Some(b"v1".as_ref()));
        assert_eq!(store.read_at(b"k", ts2).as_deref(), Some(b"v2".as_ref()));
        assert_eq!(store.read_at(b"k", ts3), None);
    }

    #[test]
    fn test_mvcc_concurrent_reads() {
        use std::thread;
        let store = MvccStore::new();

        // 初始化数据
        let mut t0 = store.begin();
        for i in 0..10 {
            t0.write(
                format!("key{}", i).into_bytes(),
                format!("value{}", i).into_bytes(),
            );
        }
        t0.commit().unwrap();

        // 并发读取：8 个线程同时读取不同键
        let handles: Vec<_> = (0..8)
            .map(|_tid| {
                let store_clone = Arc::clone(&store);
                thread::spawn(move || {
                    let mut txn = store_clone.begin();
                    for i in 0..10 {
                        let key = format!("key{}", i).into_bytes();
                        let val = txn.read(&key);
                        assert_eq!(val.as_deref(), Some(format!("value{}", i).as_bytes()));
                    }
                })
            })
            .collect();

        for h in handles {
            h.join().unwrap();
        }
    }

    #[test]
    fn test_mvcc_concurrent_writes_different_keys() {
        use std::thread;
        let store = MvccStore::new();

        // 并发写入不同键：8 个线程各写 5 个不重叠的键
        let handles: Vec<_> = (0..8)
            .map(|tid| {
                let store_clone = Arc::clone(&store);
                thread::spawn(move || {
                    let mut txn = store_clone.begin();
                    for i in 0..5 {
                        let key = format!("key_{}_{}", tid, i).into_bytes();
                        txn.write(key, format!("value_{}_{}", tid, i).into_bytes());
                    }
                    txn.commit().unwrap();
                })
            })
            .collect();

        for h in handles {
            h.join().unwrap();
        }

        // 验证：所有写入都成功
        let mut verify_txn = store.begin();
        for tid in 0..8 {
            for i in 0..5 {
                let key = format!("key_{}_{}", tid, i).into_bytes();
                let val = verify_txn.read(&key);
                assert_eq!(
                    val.as_deref(),
                    Some(format!("value_{}_{}", tid, i).as_bytes())
                );
            }
        }
    }

    #[test]
    fn test_mvcc_concurrent_writes_same_key_conflicts() {
        use std::thread;
        let store = MvccStore::new();

        // 初始值
        let mut t0 = store.begin();
        t0.write(b"shared".to_vec(), b"init".to_vec());
        t0.commit().unwrap();

        // 并发写入同一键：期望只有一个成功，其他冲突
        let handles: Vec<_> = (0..4)
            .map(|tid| {
                let store_clone = Arc::clone(&store);
                thread::spawn(move || {
                    let mut txn = store_clone.begin();
                    txn.write(b"shared".to_vec(), format!("value{}", tid).into_bytes());
                    txn.commit()
                })
            })
            .collect();

        let results: Vec<_> = handles.into_iter().map(|h| h.join().unwrap()).collect();

        // 至少有一个成功，其余应冲突
        let success_count = results.iter().filter(|r| r.is_ok()).count();
        let conflict_count = results.iter().filter(|r| r.is_err()).count();

        assert!(success_count >= 1, "至少有一个事务应成功");
        assert_eq!(success_count + conflict_count, 4);
    }

    #[test]
    fn test_mvcc_read_only_transaction() {
        let store = MvccStore::new();

        // 初始化数据
        let mut t0 = store.begin();
        t0.write(b"k1".to_vec(), b"v1".to_vec());
        t0.write(b"k2".to_vec(), b"v2".to_vec());
        t0.commit().unwrap();

        // 只读事务可以读取
        let mut ro_txn = store.begin_read_only();
        assert!(ro_txn.is_read_only());
        let start_ts = ro_txn.start_ts;
        assert_eq!(ro_txn.read(b"k1").as_deref(), Some(b"v1".as_ref()));
        assert_eq!(ro_txn.read(b"k2").as_deref(), Some(b"v2".as_ref()));

        // 只读事务提交（快速路径）
        let ts = ro_txn.commit().unwrap();
        assert_eq!(ts, start_ts); // start_ts 直接返回，无需分配新 commit_ts
    }

    #[test]
    #[should_panic(expected = "cannot write in read-only transaction")]
    fn test_mvcc_read_only_cannot_write() {
        let store = MvccStore::new();
        let mut ro_txn = store.begin_read_only();
        ro_txn.write(b"k".to_vec(), b"v".to_vec()); // 应 panic
    }

    #[test]
    #[should_panic(expected = "cannot delete in read-only transaction")]
    fn test_mvcc_read_only_cannot_delete() {
        let store = MvccStore::new();
        let mut ro_txn = store.begin_read_only();
        ro_txn.delete(b"k".to_vec()); // 应 panic
    }

    #[test]
    fn test_mvcc_read_only_performance() {
        use std::thread;
        use std::time::Instant;

        let store = MvccStore::new();

        // 初始化 100 个键
        let mut t0 = store.begin();
        for i in 0..100 {
            t0.write(
                format!("key{}", i).into_bytes(),
                format!("value{}", i).into_bytes(),
            );
        }
        t0.commit().unwrap();

        // 测试只读事务性能（无提交开销）
        let start = Instant::now();
        let handles: Vec<_> = (0..8)
            .map(|_| {
                let store_clone = Arc::clone(&store);
                thread::spawn(move || {
                    for _ in 0..100 {
                        let mut txn = store_clone.begin_read_only();
                        for i in 0..10 {
                            let _ = txn.read(&format!("key{}", i).into_bytes());
                        }
                        txn.commit().unwrap(); // 快速路径，无开销
                    }
                })
            })
            .collect();

        for h in handles {
            h.join().unwrap();
        }
        let ro_elapsed = start.elapsed();

        // 对比：普通读写事务（即使不写，也有提交开销）
        let start = Instant::now();
        let handles: Vec<_> = (0..8)
            .map(|_| {
                let store_clone = Arc::clone(&store);
                thread::spawn(move || {
                    for _ in 0..100 {
                        let mut txn = store_clone.begin();
                        for i in 0..10 {
                            let _ = txn.read(&format!("key{}", i).into_bytes());
                        }
                        txn.commit().unwrap(); // 需要分配 commit_ts
                    }
                })
            })
            .collect();

        for h in handles {
            h.join().unwrap();
        }
        let rw_elapsed = start.elapsed();

        println!("只读事务耗时: {:?}", ro_elapsed);
        println!("读写事务耗时: {:?}", rw_elapsed);
        // 只读事务应更快（通常快 20-50%）
        assert!(ro_elapsed < rw_elapsed * 2, "只读事务应有性能优势");
    }

    // ====================
    // GC 测试
    // ====================

    #[test]
    fn test_gc_version_cleanup() {
        let config = GcConfig {
            max_versions_per_key: 3,
            enable_time_based_gc: false,
            version_ttl_secs: 3600,
            auto_gc: None,
        };
        let store = MvccStore::new_with_config(config);

        // 写入 5 个版本
        for i in 0..5 {
            let mut txn = store.begin();
            txn.write(b"key".to_vec(), format!("v{}", i).into_bytes());
            txn.commit().unwrap();
        }

        // GC 前应有 5 个版本
        assert_eq!(store.total_versions(), 5);

        // 执行 GC（max_versions_per_key=3，应清理 2 个旧版本）
        let _cleaned = store.gc().unwrap();
        assert_eq!(_cleaned, 2);

        // GC 后应剩 3 个版本
        assert_eq!(store.total_versions(), 3);

        // 验证 GC 统计
        let stats = store.get_gc_stats();
        assert_eq!(stats.gc_count, 1);
        assert_eq!(stats.versions_cleaned, 2);
        assert_eq!(stats.keys_cleaned, 1);
    }

    #[test]
    fn test_gc_preserves_active_transaction_visibility() {
        let config = GcConfig {
            max_versions_per_key: 2,
            enable_time_based_gc: false,
            version_ttl_secs: 3600,
            auto_gc: None,
        };
        let store = MvccStore::new_with_config(config);

        // v1
        let mut t1 = store.begin();
        t1.write(b"key".to_vec(), b"v1".to_vec());
        t1.commit().unwrap();

        // 开启一个长事务（持有 v1 的快照）
        let mut long_txn = store.begin();
        assert_eq!(long_txn.read(b"key").as_deref(), Some(b"v1".as_ref()));

        // v2, v3, v4
        for i in 2..=4 {
            let mut txn = store.begin();
            txn.write(b"key".to_vec(), format!("v{}", i).into_bytes());
            txn.commit().unwrap();
        }

        // 现在有 4 个版本
        assert_eq!(store.total_versions(), 4);

        // 执行 GC：虽然 max_versions=2，但因为 long_txn 仍活跃，不应清理它可见的 v1
        let _cleaned = store.gc().unwrap();

        // 应保留 long_txn.start_ts 可见的版本（v1） + 最新的版本
        // 根据实现，应保留所有 ts >= min_active_ts 的版本
        assert!(store.total_versions() >= 1, "至少保留活跃事务可见的版本");

        // long_txn 仍能读取 v1
        assert_eq!(long_txn.read(b"key").as_deref(), Some(b"v1".as_ref()));

        // 提交 long_txn，它不再活跃
        drop(long_txn);

        // 再次 GC，现在可以更激进地清理
        let _cleaned2 = store.gc().unwrap();
        // 应清理到 max_versions_per_key=2
        assert_eq!(store.total_versions(), 2);
    }

    #[test]
    fn test_gc_no_active_transactions() {
        let config = GcConfig {
            max_versions_per_key: 2,
            enable_time_based_gc: false,
            version_ttl_secs: 3600,
            auto_gc: None,
        };
        let store = MvccStore::new_with_config(config);

        // 写入 5 个版本（每次都提交并结束事务）
        for i in 0..5 {
            let mut txn = store.begin();
            txn.write(b"key".to_vec(), format!("v{}", i).into_bytes());
            txn.commit().unwrap();
        }

        // 没有活跃事务
        assert_eq!(store.get_min_active_ts(), None);

        // 执行 GC，应清理到 max_versions=2
        let cleaned = store.gc().unwrap();
        assert_eq!(cleaned, 3); // 清理 3 个旧版本
        assert_eq!(store.total_versions(), 2);
    }

    #[test]
    fn test_gc_multiple_keys() {
        let config = GcConfig {
            max_versions_per_key: 2,
            enable_time_based_gc: false,
            version_ttl_secs: 3600,
            auto_gc: None,
        };
        let store = MvccStore::new_with_config(config);

        // 3 个键，每个写入 5 个版本
        for key_id in 0..3 {
            for ver in 0..5 {
                let mut txn = store.begin();
                txn.write(
                    format!("key{}", key_id).into_bytes(),
                    format!("v{}", ver).into_bytes(),
                );
                txn.commit().unwrap();
            }
        }

        // 总共 15 个版本
        assert_eq!(store.total_versions(), 15);
        assert_eq!(store.total_keys(), 3);

        // 执行 GC
        let cleaned = store.gc().unwrap();
        assert_eq!(cleaned, 9); // 每个键清理 3 个，共 9 个

        // 剩余 6 个版本 (3 键 * 2 版本/键)
        assert_eq!(store.total_versions(), 6);

        // GC 统计
        let stats = store.get_gc_stats();
        assert_eq!(stats.keys_cleaned, 3);
        assert_eq!(stats.versions_cleaned, 9);
    }

    #[test]
    fn test_gc_stats_accumulation() {
        let config = GcConfig {
            max_versions_per_key: 2,
            enable_time_based_gc: false,
            version_ttl_secs: 3600,
            auto_gc: None,
        };
        let store = MvccStore::new_with_config(config);

        // 第一轮写入并 GC
        for i in 0..4 {
            let mut txn = store.begin();
            txn.write(b"key".to_vec(), format!("v{}", i).into_bytes());
            txn.commit().unwrap();
        }
        let cleaned1 = store.gc().unwrap();

        // 第二轮写入并 GC
        for i in 4..8 {
            let mut txn = store.begin();
            txn.write(b"key".to_vec(), format!("v{}", i).into_bytes());
            txn.commit().unwrap();
        }
        let cleaned2 = store.gc().unwrap();

        // 验证统计累计
        let stats = store.get_gc_stats();
        assert_eq!(stats.gc_count, 2);
        assert_eq!(stats.versions_cleaned, cleaned1 + cleaned2);
    }

    // ====================
    // 自动 GC 测试
    // ====================

    #[test]
    fn test_auto_gc_periodic() {
        let config = GcConfig {
            max_versions_per_key: 2,
            enable_time_based_gc: false,
            version_ttl_secs: 3600,
            auto_gc: Some(AutoGcConfig {
                interval_secs: 1,     // 每 1 秒
                version_threshold: 0, // 不使用阈值，定时执行
                run_on_start: false,
                enable_adaptive: false,
            }),
        };
        let store = MvccStore::new_with_config(config);

        // 验证自动 GC 已启动
        assert!(store.is_auto_gc_running());

        // 写入一些版本
        for i in 0..5 {
            let mut txn = store.begin();
            txn.write(b"key".to_vec(), format!("v{}", i).into_bytes());
            txn.commit().unwrap();
        }

        // 初始有 5 个版本
        assert_eq!(store.total_versions(), 5);

        // 等待 GC 执行（至少 1.5 秒）
        thread::sleep(Duration::from_millis(1500));

        // GC 应该已经执行，版本数应减少到 max_versions_per_key=2
        assert!(
            store.total_versions() <= 2,
            "Auto GC should have cleaned old versions"
        );

        // 验证统计
        let stats = store.get_gc_stats();
        assert!(stats.gc_count >= 1, "At least one GC should have run");

        // 停止自动 GC
        store.stop_auto_gc();

        // 等待停止
        thread::sleep(Duration::from_millis(200));

        // 验证已停止
        assert!(!store.is_auto_gc_running());
    }

    #[test]
    fn test_auto_gc_threshold() {
        let config = GcConfig {
            max_versions_per_key: 2,
            enable_time_based_gc: false,
            version_ttl_secs: 3600,
            auto_gc: Some(AutoGcConfig {
                interval_secs: 1,      // 每 1 秒检查
                version_threshold: 10, // 超过 10 个版本触发
                run_on_start: false,
                enable_adaptive: false,
            }),
        };
        let store = MvccStore::new_with_config(config);

        // 写入少量版本（不触发阈值）
        for i in 0..5 {
            let mut txn = store.begin();
            txn.write(b"key".to_vec(), format!("v{}", i).into_bytes());
            txn.commit().unwrap();
        }

        // 等待 1.5 秒
        thread::sleep(Duration::from_millis(1500));

        // 版本数应该没有变化（未达到阈值）
        let versions_before = store.total_versions();
        assert_eq!(versions_before, 5);

        // 写入更多版本，超过阈值
        for i in 5..15 {
            let mut txn = store.begin();
            txn.write(format!("key{}", i).into_bytes(), b"value".to_vec());
            txn.commit().unwrap();
        }

        // 现在应该有超过 10 个版本
        assert!(store.total_versions() > 10);

        // 等待 GC 执行
        thread::sleep(Duration::from_millis(1500));

        // GC 应该已清理
        let versions_after = store.total_versions();
        assert!(
            versions_after < versions_before + 10,
            "Auto GC should have cleaned when threshold exceeded"
        );

        store.stop_auto_gc();
    }

    #[test]
    fn test_auto_gc_run_on_start() {
        let config = GcConfig {
            max_versions_per_key: 2,
            enable_time_based_gc: false,
            version_ttl_secs: 3600,
            auto_gc: Some(AutoGcConfig {
                interval_secs: 60, // 长间隔，不会在测试期间再次运行
                version_threshold: 0,
                run_on_start: true, // 启动时立即运行
                enable_adaptive: false,
            }),
        };

        // 先创建存储并写入数据（不启用自动 GC）
        let config_without_auto = GcConfig {
            max_versions_per_key: 2,
            enable_time_based_gc: false,
            version_ttl_secs: 3600,
            auto_gc: None,
        };
        let store_temp = MvccStore::new_with_config(config_without_auto);

        for i in 0..5 {
            let mut txn = store_temp.begin();
            txn.write(b"key".to_vec(), format!("v{}", i).into_bytes());
            txn.commit().unwrap();
        }

        drop(store_temp);

        // 创建启用 run_on_start 的存储
        let store = MvccStore::new_with_config(config);

        // 写入数据
        for i in 0..5 {
            let mut txn = store.begin();
            txn.write(b"key".to_vec(), format!("v{}", i).into_bytes());
            txn.commit().unwrap();
        }

        // 等待启动时 GC 完成
        thread::sleep(Duration::from_millis(500));

        // 验证 GC 已执行
        let stats = store.get_gc_stats();
        assert!(stats.gc_count >= 1, "run_on_start should trigger GC");

        store.stop_auto_gc();
    }

    #[test]
    fn test_auto_gc_start_stop() {
        let config = GcConfig {
            max_versions_per_key: 2,
            enable_time_based_gc: false,
            version_ttl_secs: 3600,
            auto_gc: None, // 不自动启动
        };
        let store = MvccStore::new_with_config(config);

        // 初始未运行
        assert!(!store.is_auto_gc_running());

        // 手动配置并启动
        store.update_auto_gc_config(Some(AutoGcConfig {
            interval_secs: 1,
            version_threshold: 0,
            run_on_start: false,
            enable_adaptive: false,
        }));

        let result = store.start_auto_gc();
        assert!(result.is_ok());
        assert!(store.is_auto_gc_running());

        // 重复启动应失败
        let result2 = store.start_auto_gc();
        assert!(result2.is_err());

        // 停止
        store.stop_auto_gc();
        thread::sleep(Duration::from_millis(200));
        assert!(!store.is_auto_gc_running());

        // 可以再次启动
        let result3 = store.start_auto_gc();
        assert!(result3.is_ok());
        assert!(store.is_auto_gc_running());

        store.stop_auto_gc();
    }

    #[test]
    fn test_auto_gc_concurrent_safety() {
        use std::sync::Arc;
        use std::thread;

        let config = GcConfig {
            max_versions_per_key: 5,
            enable_time_based_gc: false,
            version_ttl_secs: 3600,
            auto_gc: Some(AutoGcConfig {
                interval_secs: 1,
                version_threshold: 20,
                run_on_start: false,
                enable_adaptive: false,
            }),
        };
        let store = Arc::new(MvccStore::new_with_config(config));

        // 多线程并发写入
        let handles: Vec<_> = (0..4)
            .map(|tid| {
                let store_clone = Arc::clone(&store);
                thread::spawn(move || {
                    for i in 0..10 {
                        let mut txn = store_clone.begin();
                        txn.write(
                            format!("key_{}_{}", tid, i).into_bytes(),
                            format!("value_{}_{}", tid, i).into_bytes(),
                        );
                        let _ = txn.commit();
                        thread::sleep(Duration::from_millis(10));
                    }
                })
            })
            .collect();

        // 等待写入完成
        for h in handles {
            h.join().unwrap();
        }

        // 等待 GC 运行
        thread::sleep(Duration::from_millis(2000));

        // 验证：系统仍正常运行，没有 panic
        let stats = store.get_gc_stats();
        println!("GC stats: {:?}", stats);

        // 可以正常读取
        let mut txn = store.begin();
        let _ = txn.read(b"key_0_0");

        store.stop_auto_gc();
    }
}
