// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2025 XujueKing <leadbrand@me.com>

//! 自适应性能调优器 (Adaptive Performance Auto-Tuner)
//!
//! 目标: 让 MVCC 内核自动学习运行时特征并动态调整配置以最大化 TPS。
//!
//! 调优策略:
//! 1. **批量大小 (min_batch_size)**: 启动时快速探测,运行时根据吞吐反馈微调
//! 2. **Bloom Filter**: 根据批量大小 + 冲突率 + 读写集大小自动决策启用/禁用
//! 3. **分片数 (num_shards)**: 根据并发度和跨分片比例调整
//! 4. **热键阈值 (hot_key_threshold)**: 已有 adaptive 逻辑,默认启用
//! 5. **密度回退阈值**: 根据历史 Bloom 收益动态调整
//!
//! 使用方式:
//! ```rust
//! let mut config = OptimizedSchedulerConfig::default();
//! config.enable_auto_tuning = true;  // 启用自动调优
//! let scheduler = OptimizedMvccScheduler::new_with_config(config);
//! ```

use std::collections::VecDeque;
use std::sync::atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

/// 自适应调优器状态
pub struct AutoTuner {
    /// 是否启用自动调优
    enabled: AtomicBool,
    
    /// 批量大小历史窗口 (batch_size, tps)
    batch_history: Mutex<VecDeque<(usize, f64)>>,
    
    /// Bloom 收益历史 (启用时TPS, 禁用时TPS)
    bloom_history: Mutex<VecDeque<(f64, f64)>>,
    
    /// 当前推荐的批量大小
    recommended_batch_size: AtomicUsize,
    
    /// 当前推荐的分片数
    recommended_num_shards: AtomicUsize,
    
    /// 当前推荐的 Bloom 开关
    recommended_bloom_enabled: AtomicBool,
    
    /// 当前推荐的密度回退阈值 (x10000 存储)
    recommended_density_threshold: AtomicU64,
    
    /// 总批次计数
    total_batches: AtomicU64,
    
    /// 上次调优的批次号
    last_tuning_batch: AtomicU64,
    
    /// 调优间隔 (每 N 个批次触发一次评估)
    tuning_interval: usize,
}

impl AutoTuner {
    /// 创建新的自动调优器
    pub fn new(tuning_interval: usize) -> Self {
        Self {
            enabled: AtomicBool::new(true),
            batch_history: Mutex::new(VecDeque::with_capacity(10)),
            bloom_history: Mutex::new(VecDeque::with_capacity(10)),
            recommended_batch_size: AtomicUsize::new(10),
            recommended_num_shards: AtomicUsize::new(8),
            recommended_bloom_enabled: AtomicBool::new(false), // 默认关闭,根据场景判断
            recommended_density_threshold: AtomicU64::new(500), // 0.05 * 10000
            total_batches: AtomicU64::new(0),
            last_tuning_batch: AtomicU64::new(0),
            tuning_interval,
        }
    }
    
    /// 启用/禁用自动调优
    pub fn set_enabled(&self, enabled: bool) {
        self.enabled.store(enabled, Ordering::Relaxed);
    }
    
    /// 是否启用
    pub fn is_enabled(&self) -> bool {
        self.enabled.load(Ordering::Relaxed)
    }
    
    /// 获取推荐的批量大小
    pub fn recommended_batch_size(&self) -> usize {
        self.recommended_batch_size.load(Ordering::Relaxed)
    }
    
    /// 获取推荐的分片数
    pub fn recommended_num_shards(&self) -> usize {
        self.recommended_num_shards.load(Ordering::Relaxed)
    }
    
    /// 获取推荐的 Bloom 开关
    pub fn recommended_bloom_enabled(&self) -> bool {
        self.recommended_bloom_enabled.load(Ordering::Relaxed)
    }
    
    /// 获取推荐的密度回退阈值
    pub fn recommended_density_threshold(&self) -> f64 {
        (self.recommended_density_threshold.load(Ordering::Relaxed) as f64) / 10000.0
    }
    
    /// 记录一次批处理执行结果
    ///
    /// # 参数
    /// - `batch_size`: 本次批量大小
    /// - `duration_secs`: 执行耗时(秒)
    /// - `total_txns`: 总交易数
    /// - `conflict_rate`: 冲突率 (0.0-1.0)
    /// - `bloom_enabled`: 是否启用了 Bloom
    /// - `avg_read_write_set_size`: 平均读写集大小
    pub fn record_batch(
        &self,
        batch_size: usize,
        duration_secs: f64,
        total_txns: usize,
        conflict_rate: f64,
        bloom_enabled: bool,
        avg_read_write_set_size: usize,
    ) {
        if !self.is_enabled() {
            return;
        }
        
        let tps = if duration_secs > 0.0 {
            (total_txns as f64) / duration_secs
        } else {
            0.0
        };
        
        // 记录批量大小 -> TPS 映射
        {
            let mut history = self.batch_history.lock().unwrap();
            history.push_back((batch_size, tps));
            if history.len() > 10 {
                history.pop_front();
            }
        }
        
        let current_batch = self.total_batches.fetch_add(1, Ordering::Relaxed) + 1;
        
        // 定期触发调优评估
        let last = self.last_tuning_batch.load(Ordering::Relaxed);
        if current_batch - last >= self.tuning_interval as u64 {
            self.evaluate_and_tune(conflict_rate, avg_read_write_set_size);
            self.last_tuning_batch.store(current_batch, Ordering::Relaxed);
        }
    }
    
    /// 评估并调整参数
    fn evaluate_and_tune(&self, conflict_rate: f64, avg_rw_set_size: usize) {
        // 1. 调优批量大小: 选择历史最高 TPS 对应的 batch_size
        {
            let history = self.batch_history.lock().unwrap();
            if !history.is_empty() {
                let best = history.iter()
                    .max_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));
                if let Some((best_batch, _tps)) = best {
                    self.recommended_batch_size.store(*best_batch, Ordering::Relaxed);
                }
            }
        }
        
        // 2. Bloom Filter 自动决策:
        //    - 批量 < 100 → 关闭
        //    - 批量 ≥ 100 且 (冲突率 > 10% 或 读写集 > 10) → 开启
        //    - 否则关闭
        let current_batch = self.recommended_batch_size.load(Ordering::Relaxed);
        let should_enable_bloom = current_batch >= 100
            && (conflict_rate > 0.10 || avg_rw_set_size > 10);
        self.recommended_bloom_enabled.store(should_enable_bloom, Ordering::Relaxed);
        
        // 3. 密度回退阈值: 若 Bloom 启用,放宽阈值避免过早回退;否则收紧
        let new_threshold = if should_enable_bloom { 0.30 } else { 0.05 };
        self.recommended_density_threshold.store((new_threshold * 10000.0) as u64, Ordering::Relaxed);
        
        // 4. 分片数: 根据冲突率调整 (高冲突 → 增加分片)
        let current_shards = self.recommended_num_shards.load(Ordering::Relaxed);
        let new_shards = if conflict_rate > 0.30 {
            (current_shards * 2).min(64)  // 最多 64 分片
        } else if conflict_rate < 0.05 {
            (current_shards / 2).max(4)   // 最少 4 分片
        } else {
            current_shards
        };
        self.recommended_num_shards.store(new_shards, Ordering::Relaxed);
    }
    
    /// 快速批量大小探测 (仅在初始化时调用)
    ///
    /// 返回探测到的最优批量大小
    pub fn quick_probe_batch_size<F>(
        &self,
        probe_fn: F,
    ) -> usize
    where
        F: Fn(usize) -> f64,  // (batch_size) -> tps
    {
        let candidates = [10, 50, 100, 200, 500];
        let mut best = (10usize, 0.0f64);
        
        for &cand in &candidates {
            let tps = probe_fn(cand);
            if tps > best.1 {
                best = (cand, tps);
            }
            
            // 提前终止: 若 TPS 下降超过 20%,停止探测更大批量
            if best.1 > 0.0 && tps < best.1 * 0.8 {
                break;
            }
        }
        
        self.recommended_batch_size.store(best.0, Ordering::Relaxed);
        best.0
    }
    
    /// 获取调优统计摘要
    pub fn summary(&self) -> AutoTunerSummary {
        AutoTunerSummary {
            enabled: self.is_enabled(),
            total_batches: self.total_batches.load(Ordering::Relaxed),
            recommended_batch_size: self.recommended_batch_size(),
            recommended_num_shards: self.recommended_num_shards(),
            recommended_bloom_enabled: self.recommended_bloom_enabled(),
            recommended_density_threshold: self.recommended_density_threshold(),
        }
    }
}

/// 调优器统计摘要
#[derive(Debug, Clone)]
pub struct AutoTunerSummary {
    pub enabled: bool,
    pub total_batches: u64,
    pub recommended_batch_size: usize,
    pub recommended_num_shards: usize,
    pub recommended_bloom_enabled: bool,
    pub recommended_density_threshold: f64,
}

impl AutoTunerSummary {
    pub fn print(&self) {
        println!("=== Auto-Tuner Summary ===");
        println!("Enabled: {}", self.enabled);
        println!("Total Batches Observed: {}", self.total_batches);
        println!("Recommended Batch Size: {}", self.recommended_batch_size);
        println!("Recommended Num Shards: {}", self.recommended_num_shards);
        println!("Recommended Bloom Filter: {}", if self.recommended_bloom_enabled { "ON" } else { "OFF" });
        println!("Recommended Density Threshold: {:.2}%", self.recommended_density_threshold * 100.0);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_auto_tuner_basic() {
        let tuner = AutoTuner::new(5);
        assert!(tuner.is_enabled());
        
        // 模拟几次批处理
        tuner.record_batch(50, 0.1, 5000, 0.05, false, 3);
        tuner.record_batch(100, 0.08, 10000, 0.10, false, 5);
        tuner.record_batch(200, 0.12, 20000, 0.15, true, 8);
        
        let summary = tuner.summary();
        assert_eq!(summary.total_batches, 3);
    }
    
    #[test]
    fn test_quick_probe() {
        let tuner = AutoTuner::new(10);
        
        // 模拟探测函数: batch=100 时 TPS 最高
        let best = tuner.quick_probe_batch_size(|batch| {
            match batch {
                10 => 100000.0,
                50 => 150000.0,
                100 => 200000.0,
                200 => 180000.0,
                500 => 150000.0,
                _ => 0.0,
            }
        });
        
        assert_eq!(best, 100);
        assert_eq!(tuner.recommended_batch_size(), 100);
    }
}
