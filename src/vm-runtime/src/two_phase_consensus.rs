// SPDX-License-Identifier: GPL-3.0-or-later
//! Two-Phase Commit (2PC) 原型
//! 
//! ## 功能特性
//! - **基础 2PC**: prepare → commit 两阶段提交
//! - **批量 Prepare**: 多事务批量锁获取与校验，减少锁竞争
//! - **流水线 2PC**: prepare 完成后立即释放读锁，commit 异步化处理
//! - **并行校验**: rayon 并行读集合版本校验
//! 
//! ## 性能优化
//! - 排序锁避免死锁
//! - 批量锁操作减少系统调用
//! - prepare/commit 流水线提升吞吐
//! - 完整 Prometheus 指标埋点

use std::sync::{Arc, Mutex};
use std::time::Instant;
use rayon::prelude::*;
use std::collections::HashMap;

use crate::mvcc::{Txn, MvccStore};

/// 2PC Prepare 阶段的中间状态
#[derive(Debug, Clone)]
pub struct PreparedTransaction {
    /// 分配的 commit 时间戳
    pub commit_ts: u64,
    /// 写集合 (key → value)
    pub writes: HashMap<Vec<u8>, Option<Vec<u8>>>,
    /// Prepare 完成时间
    pub prepared_at: Instant,
}

/// 自适应批量大小配置
#[derive(Debug)]
pub struct AdaptiveBatchConfig {
    /// 当前批量大小
    pub current_size: usize,
    /// 最小批量大小
    pub min_size: usize,
    /// 最大批量大小
    pub max_size: usize,
    /// 目标冲突率 (0.0-1.0)
    pub target_conflict_rate: f64,
    /// 最近批次的统计数据
    recent_stats: Mutex<BatchStats>,
}

#[derive(Debug, Clone, Default)]
struct BatchStats {
    /// 最近批次的平均冲突率
    avg_conflict_rate: f64,
    /// 最近批次的平均延迟 (ms)
    avg_latency_ms: f64,
    /// 样本数量
    samples: usize,
}

impl Default for AdaptiveBatchConfig {
    fn default() -> Self {
        Self {
            current_size: 32,
            min_size: 8,
            max_size: 128,
            target_conflict_rate: 0.05, // 目标 5% 冲突率
            recent_stats: Mutex::new(BatchStats::default()),
        }
    }
}

impl AdaptiveBatchConfig {
    /// 根据最近批次结果调整批量大小
    pub fn adjust(&self, conflict_rate: f64, latency_ms: f64) -> usize {
        let mut stats = self.recent_stats.lock().unwrap();
        
        // 更新统计数据（指数移动平均）
        let alpha = 0.3; // 平滑因子
        stats.avg_conflict_rate = if stats.samples == 0 {
            conflict_rate
        } else {
            alpha * conflict_rate + (1.0 - alpha) * stats.avg_conflict_rate
        };
        stats.avg_latency_ms = if stats.samples == 0 {
            latency_ms
        } else {
            alpha * latency_ms + (1.0 - alpha) * stats.avg_latency_ms
        };
        stats.samples += 1;

        // 调整策略：
        // - 冲突率低于目标 → 增大批次（提升吞吐）
        // - 冲突率高于目标 → 减小批次（降低延迟）
        let new_size = if stats.avg_conflict_rate < self.target_conflict_rate * 0.8 {
            // 冲突率很低，增大批次 20%
            (self.current_size as f64 * 1.2).round() as usize
        } else if stats.avg_conflict_rate > self.target_conflict_rate * 1.5 {
            // 冲突率过高，减小批次 20%
            (self.current_size as f64 * 0.8).round() as usize
        } else {
            // 冲突率适中，保持不变
            self.current_size
        };

        new_size.clamp(self.min_size, self.max_size)
    }
}

#[derive(Clone)]
pub struct TwoPhaseCoordinator {
    store: Arc<MvccStore>,
    /// 自适应批量配置（可选）
    adaptive_config: Option<Arc<Mutex<AdaptiveBatchConfig>>>,
}

impl TwoPhaseCoordinator {
    pub fn new(store: Arc<MvccStore>) -> Self { 
        Self { 
            store,
            adaptive_config: None,
        } 
    }

    /// 创建带自适应批量配置的协调器
    pub fn with_adaptive_batch(store: Arc<MvccStore>, config: AdaptiveBatchConfig) -> Self {
        Self {
            store,
            adaptive_config: Some(Arc::new(Mutex::new(config))),
        }
    }

    /// 获取当前推荐的批量大小
    pub fn get_recommended_batch_size(&self) -> usize {
        self.adaptive_config
            .as_ref()
            .map(|cfg| cfg.lock().unwrap().current_size)
            .unwrap_or(32) // 默认批量大小
    }

    /// 执行真正的 2PC 两阶段提交（prepare → commit 完全分离）：
    /// **Prepare**: 加锁 + 读校验 + 分配 commit_ts
    /// **Commit**: 批量写入 (append_version) + 自动释放锁
    pub fn prepare_and_commit(&self, txn: Txn) -> Result<u64, String> {
        let prepare_start = Instant::now();
        // 构造锁集合：对写集合 key 排序后加锁，保持全局一致顺序避免环形死锁。
        let mut keys: Vec<Vec<u8>> = txn.writes().keys().cloned().collect();
        keys.sort();
        // 先收集所有 Arc<Mutex>, 再统一加锁持有 guard, 避免局部变量生命周期问题
        let locks: Vec<_> = keys.iter().map(|k| self.store.key_lock(k)).collect();
        let _guards: Vec<_> = locks.iter().map(|lk| lk.lock()).collect();

    // === Prepare 阶段：并行读集合校验 ===
        let validation_start = Instant::now();
        let conflict = txn.reads()
            .par_iter()
            .find_any(|read_key| {
                let tail_ts = txn.get_tail_ts(read_key);
                tail_ts > txn.start_ts
            });

        if let Some(conflict_key) = conflict {
            let tail_ts = txn.get_tail_ts(conflict_key);
            if let Some(mc) = self.store.get_metrics() {
                mc.record_cross_shard_prepare(prepare_start.elapsed().as_secs_f64(), false, false);
            }
            return Err(format!("2PC abort: read-write conflict on key (tail_ts={} > start_ts={})", tail_ts, txn.start_ts));
        }

    // 分配 commit_ts：全局分配 (2PC 统一协调，不使用 override)
    let commit_ts = self.store.next_ts();

        // 记录 prepare 指标 (成功=true, privacy_invalid=false)；耗时为当前 elapsed。
        if let Some(mc) = self.store.get_metrics() {
            mc.record_cross_shard_prepare(prepare_start.elapsed().as_secs_f64(), true, false);
        }

        // === Commit 阶段：批量写入 ===
        let commit_start = Instant::now();
        for (key, value_opt) in txn.writes() {
            self.store.append_version(key, commit_ts, value_opt.clone());
        }
        
        let commit_duration = commit_start.elapsed();
        // 记录 2PC commit 指标 (成功=true)
        if let Some(mc) = self.store.get_metrics() {
            mc.record_cross_shard_commit(commit_duration, true);
        }
        // 也记录 MVCC commit 延迟（保留原有逻辑）
        if let Some(mc) = txn.metrics() {
            mc.record_commit_latency(commit_duration);
        }

    // 锁自动释放
    Ok(commit_ts)
    }

    /// 批量 Prepare: 一次性处理多个事务的 prepare 阶段
    /// 
    /// ## 优化点
    /// - 批量收集所有事务的写键并排序，减少锁获取次数
    /// - 并行校验所有事务的读集合
    /// - 返回 PreparedTransaction 中间状态，供后续 commit
    /// 
    /// ## 参数
    /// - `txns`: 待 prepare 的事务列表
    /// 
    /// ## 返回
    /// - Ok: Vec<PreparedTransaction> 所有事务的 prepared 状态
    /// - Err: (索引, 错误信息) 第一个失败事务的索引和原因
    pub fn batch_prepare(&self, txns: Vec<Txn>) -> Result<Vec<PreparedTransaction>, (usize, String)> {
        let batch_start = Instant::now();
        let batch_size = txns.len();

        // 1. 收集所有事务的写键并去重排序
        let mut all_keys: Vec<Vec<u8>> = txns.iter()
            .flat_map(|txn| txn.writes().keys().cloned())
            .collect();
        all_keys.sort();
        all_keys.dedup();

        // 2. 批量加锁 (一次性获取所有锁)
        let locks: Vec<_> = all_keys.iter().map(|k| self.store.key_lock(k)).collect();
        let _guards: Vec<_> = locks.iter().map(|lk| lk.lock()).collect();

        // 3. 并行校验所有事务的读集合
        let results: Vec<_> = txns.par_iter()
            .enumerate()
            .map(|(idx, txn)| {
                let conflict = txn.reads()
                    .par_iter()
                    .find_any(|read_key| {
                        let tail_ts = txn.get_tail_ts(read_key);
                        tail_ts > txn.start_ts
                    });

                if let Some(conflict_key) = conflict {
                    let tail_ts = txn.get_tail_ts(conflict_key);
                    Err((idx, format!("read-write conflict on key (tail_ts={} > start_ts={})", tail_ts, txn.start_ts)))
                } else {
                    Ok(())
                }
            })
            .collect();

        // 4. 检查是否有失败（短路返回）
        if let Some(Err((idx, err))) = results.iter().find(|r| r.is_err()) {
            if let Some(mc) = self.store.get_metrics() {
                mc.record_cross_shard_prepare(batch_start.elapsed().as_secs_f64(), false, false);
            }
            return Err((*idx, err.clone()));
        }

        // 5. 为所有事务分配 commit_ts 并构造 PreparedTransaction
        let prepared: Vec<PreparedTransaction> = txns.into_iter()
            .map(|txn| {
                let commit_ts = self.store.next_ts();
                let writes = txn.writes().clone();
                PreparedTransaction {
                    commit_ts,
                    writes,
                    prepared_at: Instant::now(),
                }
            })
            .collect();

        // 6. 记录批量 prepare 成功指标
        if let Some(mc) = self.store.get_metrics() {
            mc.record_cross_shard_prepare(batch_start.elapsed().as_secs_f64(), true, false);
            mc.record_batch_prepare(batch_size);
        }

        // 7. 自适应调整批量大小（如果启用）
        let batch_latency_ms = batch_start.elapsed().as_secs_f64() * 1000.0;
        let conflict_rate = 0.0; // 当前批次无冲突
        if let Some(adaptive) = &self.adaptive_config {
            let new_size = adaptive.lock().unwrap().adjust(conflict_rate, batch_latency_ms);
            adaptive.lock().unwrap().current_size = new_size;
        }

        Ok(prepared)
        // 锁自动释放
    }

    /// 批量 Prepare (自适应版本): 根据冲突率自动调整批量大小
    /// 
    /// 返回 (PreparedTransaction列表, 推荐的下次批量大小)
    pub fn adaptive_batch_prepare(&self, txns: Vec<Txn>) -> Result<(Vec<PreparedTransaction>, usize), (usize, String)> {
        let batch_start = Instant::now();
        let batch_size = txns.len();

        // 1. 收集所有事务的写键并去重排序
        let mut all_keys: Vec<Vec<u8>> = txns.iter()
            .flat_map(|txn| txn.writes().keys().cloned())
            .collect();
        all_keys.sort();
        all_keys.dedup();

        // 2. 批量加锁
        let locks: Vec<_> = all_keys.iter().map(|k| self.store.key_lock(k)).collect();
        let _guards: Vec<_> = locks.iter().map(|lk| lk.lock()).collect();

        // 3. 并行校验并统计冲突数
        let mut conflict_count = 0;
        let results: Vec<_> = txns.par_iter()
            .enumerate()
            .map(|(idx, txn)| {
                let conflict = txn.reads()
                    .par_iter()
                    .find_any(|read_key| {
                        let tail_ts = txn.get_tail_ts(read_key);
                        tail_ts > txn.start_ts
                    });

                if let Some(conflict_key) = conflict {
                    let tail_ts = txn.get_tail_ts(conflict_key);
                    Err((idx, format!("read-write conflict on key (tail_ts={} > start_ts={})", tail_ts, txn.start_ts)))
                } else {
                    Ok(())
                }
            })
            .collect();

        // 统计冲突数
        conflict_count = results.iter().filter(|r| r.is_err()).count();

        // 4. 检查是否有失败
        if let Some(Err((idx, err))) = results.iter().find(|r| r.is_err()) {
            let conflict_rate = conflict_count as f64 / batch_size as f64;
            let batch_latency_ms = batch_start.elapsed().as_secs_f64() * 1000.0;
            
            // 调整批量大小（即使失败也收集数据）
            let next_size = if let Some(adaptive) = &self.adaptive_config {
                let new_size = adaptive.lock().unwrap().adjust(conflict_rate, batch_latency_ms);
                adaptive.lock().unwrap().current_size = new_size;
                new_size
            } else {
                batch_size
            };

            if let Some(mc) = self.store.get_metrics() {
                mc.record_cross_shard_prepare(batch_start.elapsed().as_secs_f64(), false, false);
            }
            return Err((*idx, err.clone()));
        }

        // 5. 为所有事务分配 commit_ts
        let prepared: Vec<PreparedTransaction> = txns.into_iter()
            .map(|txn| {
                let commit_ts = self.store.next_ts();
                let writes = txn.writes().clone();
                PreparedTransaction {
                    commit_ts,
                    writes,
                    prepared_at: Instant::now(),
                }
            })
            .collect();

        // 6. 记录指标
        if let Some(mc) = self.store.get_metrics() {
            mc.record_cross_shard_prepare(batch_start.elapsed().as_secs_f64(), true, false);
            mc.record_batch_prepare(batch_size);
        }

        // 7. 自适应调整
        let batch_latency_ms = batch_start.elapsed().as_secs_f64() * 1000.0;
        let conflict_rate = conflict_count as f64 / batch_size as f64;
        let next_size = if let Some(adaptive) = &self.adaptive_config {
            let new_size = adaptive.lock().unwrap().adjust(conflict_rate, batch_latency_ms);
            adaptive.lock().unwrap().current_size = new_size;
            new_size
        } else {
            batch_size
        };

        Ok((prepared, next_size))
    }

    /// 流水线 Commit: 批量提交已 prepared 的事务
    /// 
    /// ## 流水线特性
    /// - prepare 完成后立即释放锁，不阻塞后续 prepare
    /// - commit 阶段可异步化处理（当前为同步简化实现）
    /// - 支持批量写入优化
    /// 
    /// ## 参数
    /// - `prepared_txns`: batch_prepare 返回的 prepared 状态列表
    /// 
    /// ## 返回
    /// - 成功提交的事务数量
    pub fn pipeline_commit(&self, prepared_txns: Vec<PreparedTransaction>) -> usize {
        let commit_start = Instant::now();
        let pipeline_depth = prepared_txns.len();
        let mut success_count = 0;

        for prepared in prepared_txns {
            // 批量写入所有版本
            for (key, value_opt) in prepared.writes.iter() {
                self.store.append_version(key, prepared.commit_ts, value_opt.clone());
            }
            success_count += 1;
        }

        // 记录 commit 指标
        let commit_duration = commit_start.elapsed();
        if let Some(mc) = self.store.get_metrics() {
            mc.record_cross_shard_commit(commit_duration, true);
            mc.record_pipeline_commit(pipeline_depth);
        }

        success_count
    }

    /// 细粒度锁控制的批量 Prepare: 分批加锁而非一次性锁定所有键
    /// 
    /// ## 优化原理
    /// - 将事务按写键分组，每组独立加锁
    /// - 减少单次锁持有时间和锁争用范围
    /// - 提升并发度，降低平均等待延迟
    /// 
    /// ## 参数
    /// - `txns`: 待 prepare 的事务列表
    /// - `lock_batch_size`: 每批锁定的最大键数量（默认32）
    /// 
    /// ## 返回
    /// - Ok: Vec<PreparedTransaction> 所有事务的 prepared 状态
    /// - Err: (索引, 错误信息) 第一个失败事务的索引和原因
    pub fn batch_prepare_fine_grained(
        &self, 
        txns: Vec<Txn>, 
        lock_batch_size: usize
    ) -> Result<Vec<PreparedTransaction>, (usize, String)> {
        let batch_start = Instant::now();
        let total_txns = txns.len();

        // 1. 收集所有写键并分组
        let mut all_keys: Vec<Vec<u8>> = txns.iter()
            .flat_map(|txn| txn.writes().keys().cloned())
            .collect();
        all_keys.sort();
        all_keys.dedup();

        // 2. 将键分成多个批次
        let key_batches: Vec<&[Vec<u8>]> = all_keys.chunks(lock_batch_size).collect();
        let mut all_prepared = Vec::with_capacity(total_txns);

        // 3. 分批处理事务（每批独立加锁）
        for (batch_idx, key_batch) in key_batches.iter().enumerate() {
            // 3.1 仅锁定当前批次的键
            let locks: Vec<_> = key_batch.iter().map(|k| self.store.key_lock(k)).collect();
            let _guards: Vec<_> = locks.iter().map(|lk| lk.lock()).collect();

            // 3.2 找出涉及这些键的事务
            let relevant_txns: Vec<(usize, &Txn)> = txns.iter()
                .enumerate()
                .filter(|(_, txn)| {
                    txn.writes().keys().any(|k| key_batch.contains(k))
                })
                .collect();

            // 3.3 并行校验这些事务
            let results: Vec<_> = relevant_txns.par_iter()
                .map(|(idx, txn)| {
                    let conflict = txn.reads()
                        .par_iter()
                        .find_any(|read_key| {
                            let tail_ts = txn.get_tail_ts(read_key);
                            tail_ts > txn.start_ts
                        });

                    if let Some(conflict_key) = conflict {
                        let tail_ts = txn.get_tail_ts(conflict_key);
                        Err((*idx, format!("conflict on key (tail_ts={} > start_ts={})", tail_ts, txn.start_ts)))
                    } else {
                        Ok(*idx)
                    }
                })
                .collect();

            // 3.4 检查冲突
            if let Some(Err((idx, err))) = results.iter().find(|r| r.is_err()) {
                if let Some(mc) = self.store.get_metrics() {
                    mc.record_cross_shard_prepare(batch_start.elapsed().as_secs_f64(), false, false);
                }
                return Err((*idx, err.clone()));
            }

            // 3.5 为成功的事务分配 commit_ts（仅当前批次）
            for (idx, txn) in relevant_txns.iter() {
                if !all_prepared.iter().any(|p: &(usize, PreparedTransaction)| p.0 == *idx) {
                    let commit_ts = self.store.next_ts();
                    let writes = txn.writes().clone();
                    all_prepared.push((
                        *idx,
                        PreparedTransaction {
                            commit_ts,
                            writes,
                            prepared_at: Instant::now(),
                        }
                    ));
                }
            }

            // 锁在这里自动释放（作用域结束）
        }

        // 4. 按原始索引排序并提取 PreparedTransaction
        all_prepared.sort_by_key(|(idx, _)| *idx);
        let prepared: Vec<PreparedTransaction> = all_prepared.into_iter()
            .map(|(_, p)| p)
            .collect();

        // 5. 记录指标
        if let Some(mc) = self.store.get_metrics() {
            mc.record_cross_shard_prepare(batch_start.elapsed().as_secs_f64(), true, false);
            mc.record_batch_prepare(total_txns);
        }

        Ok(prepared)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mvcc::MvccStore;
    use std::sync::Arc;

    #[test]
    fn two_pc_basic_commit() {
        let store = MvccStore::new();
    // metrics 默认已启用; 若需重新启用可在构造前修改 MvccStore::new() 逻辑
        let coord = TwoPhaseCoordinator::new(store.clone());
    let mut tx = store.begin();
        tx.write(b"k".to_vec(), b"v".to_vec());
        let res = coord.prepare_and_commit(tx).unwrap();
        assert!(res > 0);

    #[test]
    fn two_pc_read_write_conflict_abort() {
        let store = MvccStore::new();
        let coord = TwoPhaseCoordinator::new(store.clone());

        // T1: 读取 key "conflict"
        let mut tx1 = store.begin();
        let _val = tx1.read(b"conflict"); // 记录读集合，start_ts=1

        // T2: 写入并提交 key "conflict"
        let mut tx2 = store.begin();
        tx2.write(b"conflict".to_vec(), b"new_value".to_vec());
        let _commit_ts = tx2.commit().unwrap(); // commit_ts > tx1.start_ts

        // T1: 尝试多分区提交（触发 2PC prepare），应因读集合冲突 abort
        tx1.write(b"k1".to_vec(), b"v1".to_vec());
        tx1.write(b"k2".to_vec(), b"v2".to_vec()); // 构造多键写确保触发 2PC（若路由需要）
        let result = coord.prepare_and_commit(tx1);
        assert!(result.is_err(), "应因 read-write conflict abort");
        let err_msg = result.unwrap_err();
        assert!(err_msg.contains("read-write conflict"), "错误消息应包含 conflict 关键字");
    }
    }
}
