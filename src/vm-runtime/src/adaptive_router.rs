// SPDX-License-Identifier: GPL-3.0-or-later
// 自适应路由器：基于运行时冲突/成功率动态调整 FastPath 目标占比

use std::cmp::Ordering;
use std::env;
use std::sync::atomic::{AtomicU64, Ordering as AOrd};
use std::sync::Mutex;

use crate::parallel_mvcc::MvccSchedulerStats;

/// 将 f64 比例以 1e6 放大存储到 u64，避免跨线程浮点写入争用
fn encode_ratio(v: f64) -> u64 {
    let v = v.clamp(0.0, 1.0);
    (v * 1_000_000.0) as u64
}
fn decode_ratio(v: u64) -> f64 {
    (v as f64) / 1_000_000.0
}

#[derive(Debug)]
struct AdaptiveState {
    // 上次快照（累积量）
    last_successful: u64,
    last_failed: u64,
    last_conflict: u64,
    // 调整统计
    adjustments_total: u64,
    // 节流更新
    tick: u64,
}

/// 配置：自适应路由参数
#[derive(Debug, Clone)]
pub struct AdaptiveRouterConfig {
    pub initial_fast_ratio: f64,
    pub min_ratio: f64,
    pub max_ratio: f64,
    pub step_up: f64,
    pub step_down: f64,
    pub conflict_low: f64,
    pub conflict_high: f64,
    pub success_low: f64,
    pub update_every: u64,
}

impl Default for AdaptiveRouterConfig {
    fn default() -> Self {
        Self {
            initial_fast_ratio: 0.70,
            min_ratio: 0.10,
            max_ratio: 0.90,
            step_up: 0.05,
            step_down: 0.05,
            conflict_low: 0.05,
            conflict_high: 0.25,
            success_low: 0.80,
            update_every: 100,
        }
    }
}

impl AdaptiveRouterConfig {
    /// 从环境变量读取配置，未设置的变量使用默认值。
    /// 环境变量列表（可选）：
    /// - SUPERVM_ADAPTIVE_INIT          (f64, 0..1) 初始 fast 比例，默认 0.70
    /// - SUPERVM_ADAPTIVE_MIN           (f64, 0..1) 最小 fast 比例，默认 0.10
    /// - SUPERVM_ADAPTIVE_MAX           (f64, 0..1) 最大 fast 比例，默认 0.90
    /// - SUPERVM_ADAPTIVE_STEP_UP       (f64) 上调步长，默认 0.05
    /// - SUPERVM_ADAPTIVE_STEP_DOWN     (f64) 下调步长，默认 0.05
    /// - SUPERVM_ADAPTIVE_CONFLICT_LOW  (f64) 冲突低阈值，默认 0.05
    /// - SUPERVM_ADAPTIVE_CONFLICT_HIGH (f64) 冲突高阈值，默认 0.25
    /// - SUPERVM_ADAPTIVE_SUCCESS_LOW   (f64) 成功率低阈值，默认 0.80
    /// - SUPERVM_ADAPTIVE_UPDATE_EVERY  (u64) 节流更新周期，默认 100
    pub fn from_env() -> Self {
        let mut cfg = Self::default();
        if let Ok(v) = env::var("SUPERVM_ADAPTIVE_INIT") { if let Ok(p) = v.parse::<f64>() { cfg.initial_fast_ratio = p; } }
        if let Ok(v) = env::var("SUPERVM_ADAPTIVE_MIN") { if let Ok(p) = v.parse::<f64>() { cfg.min_ratio = p; } }
        if let Ok(v) = env::var("SUPERVM_ADAPTIVE_MAX") { if let Ok(p) = v.parse::<f64>() { cfg.max_ratio = p; } }
        if let Ok(v) = env::var("SUPERVM_ADAPTIVE_STEP_UP") { if let Ok(p) = v.parse::<f64>() { cfg.step_up = p; } }
        if let Ok(v) = env::var("SUPERVM_ADAPTIVE_STEP_DOWN") { if let Ok(p) = v.parse::<f64>() { cfg.step_down = p; } }
        if let Ok(v) = env::var("SUPERVM_ADAPTIVE_CONFLICT_LOW") { if let Ok(p) = v.parse::<f64>() { cfg.conflict_low = p; } }
        if let Ok(v) = env::var("SUPERVM_ADAPTIVE_CONFLICT_HIGH") { if let Ok(p) = v.parse::<f64>() { cfg.conflict_high = p; } }
        if let Ok(v) = env::var("SUPERVM_ADAPTIVE_SUCCESS_LOW") { if let Ok(p) = v.parse::<f64>() { cfg.success_low = p; } }
        if let Ok(v) = env::var("SUPERVM_ADAPTIVE_UPDATE_EVERY") { if let Ok(p) = v.parse::<u64>() { cfg.update_every = p; } }
        // 基本有效性修正
        if cfg.min_ratio > cfg.max_ratio { std::mem::swap(&mut cfg.min_ratio, &mut cfg.max_ratio); }
        cfg.initial_fast_ratio = cfg.initial_fast_ratio.clamp(cfg.min_ratio, cfg.max_ratio);
        cfg
    }
}

/// 简单的自适应路由器（Phase A）
/// 策略：根据最近窗口冲突率/成功率，上调或下调 target_fast_ratio
pub struct AdaptiveRouter {
    // 目标 fast 比例（以 1e6 缩放的原子存储）
    target_fast_ratio: AtomicU64,

    // 配置参数
    cfg: AdaptiveRouterConfig,

    // 可变状态（受保护）
    state: Mutex<AdaptiveState>,
}

impl Default for AdaptiveRouter {
    fn default() -> Self {
        Self::new()
    }
}

impl AdaptiveRouter {
    pub fn new() -> Self {
        Self::new_with_config(AdaptiveRouterConfig::default())
    }

    pub fn new_with_config(cfg: AdaptiveRouterConfig) -> Self {
        Self {
            target_fast_ratio: AtomicU64::new(encode_ratio(cfg.initial_fast_ratio)),
            cfg,
            state: Mutex::new(AdaptiveState {
                last_successful: 0,
                last_failed: 0,
                last_conflict: 0,
                adjustments_total: 0,
                tick: 0,
            }),
        }
    }

    /// 读取目标 fast 比例（0..1）
    pub fn target_fast_ratio(&self) -> f64 {
        decode_ratio(self.target_fast_ratio.load(AOrd::Relaxed))
    }

    /// 内部：设置目标 fast 比例（带 clamp）
    fn set_target_fast_ratio(&self, v: f64) {
        let clamped = v.clamp(self.cfg.min_ratio, self.cfg.max_ratio);
        self.target_fast_ratio.store(encode_ratio(clamped), AOrd::Relaxed);
    }

    /// 可能地执行一次基于调度器统计的更新（节流：每 update_every 次调用执行一次）
    pub fn maybe_update(&self, stats: &MvccSchedulerStats) {
        let mut st = self.state.lock().unwrap();
        st.tick += 1;
        if !st.tick.is_multiple_of(self.cfg.update_every) {
            return;
        }

        let total_now = stats.successful_txs + stats.failed_txs;
        let total_prev = st.last_successful + st.last_failed;
        let delta_total = total_now.saturating_sub(total_prev);
        let delta_conflict = stats.conflict_count.saturating_sub(st.last_conflict);

        // 更新快照
        st.last_successful = stats.successful_txs;
        st.last_failed = stats.failed_txs;
        st.last_conflict = stats.conflict_count;

        if delta_total == 0 {
            return; // 无新事务，不调整
        }

        let conflict_rate = (delta_conflict as f64) / (delta_total as f64);
        let success_rate = if total_now == 0 {
            1.0
        } else {
            (stats.successful_txs as f64) / (total_now as f64)
        };

        let mut target = self.target_fast_ratio();

        // 基本规则：冲突高则降，冲突低则升
        if conflict_rate > self.cfg.conflict_high {
            target -= self.cfg.step_down;
        } else if conflict_rate < self.cfg.conflict_low {
            target += self.cfg.step_up;
        }

        // 成功率保护：成功率过低且冲突偏高时更强降速
        if success_rate < self.cfg.success_low && conflict_rate > self.cfg.conflict_low {
            target -= self.cfg.step_down; // 额外再降一次
        }

        // 计算是否产生有效调整
        let before = self.target_fast_ratio();
        match target.partial_cmp(&before).unwrap_or(Ordering::Equal) {
            Ordering::Less | Ordering::Greater => {
                self.set_target_fast_ratio(target);
                st.adjustments_total += 1;
            }
            Ordering::Equal => {}
        }
    }

    /// 返回累计的自适应调整次数
    pub fn adjustments_total(&self) -> u64 {
        self.state.lock().unwrap().adjustments_total
    }

    /// 导出该路由器自身的 Prometheus 指标片段
    pub fn export_prometheus(&self) -> String {
        let mut out = String::new();
        out.push_str("# HELP vm_routing_target_fast_ratio Target ratio for fast path decided by adaptive router\n");
        out.push_str("# TYPE vm_routing_target_fast_ratio gauge\n");
        out.push_str(&format!(
            "vm_routing_target_fast_ratio {:.6}\n",
            self.target_fast_ratio()
        ));

        out.push_str("# HELP vm_routing_adaptive_adjustments_total Total number of adaptive ratio adjustments\n");
        out.push_str("# TYPE vm_routing_adaptive_adjustments_total counter\n");
        out.push_str(&format!(
            "vm_routing_adaptive_adjustments_total {}\n",
            self.adjustments_total()
        ));

        out
    }
}
