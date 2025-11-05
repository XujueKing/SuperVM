// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2025 XujueKing <leadbrand@me.com>

//! 基于 MVCC 的并行执行引擎 (v0.9.0)
//! 
//! 这个模块将 MVCC 作为底层存储引擎，实现高性能的并行事务执行。
//! 相比旧版本的 ParallelScheduler，这个版本：
//! - 使用 MVCC 的内置冲突检测，无需手动 ConflictDetector
//! - 使用 MVCC 的快照隔离，无需 StateManager
//! - 自动垃圾回收，支持自适应 GC
//! - 更好的并发性能和正确性保证

use std::sync::{Arc, atomic::{AtomicU64, Ordering}};
use anyhow::Result;
use rayon::prelude::*;

use crate::mvcc::{MvccStore, Txn, GcConfig, AutoGcConfig};

/// 交易标识符
pub type TxId = u64;

/// 交易执行函数
/// 
/// 接收一个 MVCC 事务，执行业务逻辑，返回结果
pub type TxnFn = Box<dyn FnOnce(&mut Txn) -> Result<i32> + Send>;

/// 交易执行结果
#[derive(Debug, Clone)]
pub struct TxnResult {
    /// 交易 ID
    pub tx_id: TxId,
    /// 执行返回值
    pub return_value: Option<i32>,
    /// 是否成功
    pub success: bool,
    /// 错误信息 (如果失败)
    pub error: Option<String>,
    /// 提交时间戳 (如果成功)
    pub commit_ts: Option<u64>,
}

/// 批量交易执行结果
#[derive(Debug, Clone)]
pub struct BatchTxnResult {
    /// 成功的交易数
    pub successful: u64,
    /// 失败的交易数
    pub failed: u64,
    /// 因冲突回滚的交易数
    pub conflicts: u64,
    /// 各交易的详细结果
    pub results: Vec<TxnResult>,
}

/// 执行统计信息
#[derive(Debug, Clone, Default)]
pub struct MvccSchedulerStats {
    /// 执行成功的交易数
    pub successful_txs: u64,
    /// 执行失败的交易数
    pub failed_txs: u64,
    /// 因冲突回滚的交易数
    pub conflict_count: u64,
    /// 重试次数
    pub retry_count: u64,
}

impl MvccSchedulerStats {
    pub fn new() -> Self {
        Self::default()
    }
    
    /// 获取总交易数
    pub fn total_txs(&self) -> u64 {
        self.successful_txs + self.failed_txs
    }
    
    /// 计算成功率
    pub fn success_rate(&self) -> f64 {
        let total = self.total_txs();
        if total == 0 {
            0.0
        } else {
            self.successful_txs as f64 / total as f64
        }
    }
    
    /// 计算冲突率
    pub fn conflict_rate(&self) -> f64 {
        let total = self.total_txs();
        if total == 0 {
            0.0
        } else {
            self.conflict_count as f64 / total as f64
        }
    }
}

/// 调度器配置
#[derive(Debug, Clone)]
pub struct MvccSchedulerConfig {
    /// MVCC 配置
    pub mvcc_config: GcConfig,
    /// 最大重试次数 (冲突时)
    pub max_retries: u64,
    /// 并行度 (工作线程数)
    pub num_workers: usize,
}

impl Default for MvccSchedulerConfig {
    fn default() -> Self {
        Self {
            mvcc_config: GcConfig {
                max_versions_per_key: 20,
                enable_time_based_gc: false,
                version_ttl_secs: 3600,
                auto_gc: Some(AutoGcConfig {
                    interval_secs: 60,
                    version_threshold: 1000,
                    run_on_start: false,
                    enable_adaptive: true,  // 默认启用自适应 GC
                }),
            },
            max_retries: 3,
            num_workers: rayon::current_num_threads(),
        }
    }
}

/// 基于 MVCC 的并行调度器
/// 
/// 核心设计：
/// - 使用 MvccStore 作为唯一的存储引擎
/// - 每个交易在独立的 MVCC 事务中执行
/// - 冲突检测由 MVCC 自动处理（写写冲突）
/// - 快照隔离保证事务一致性
/// - 自适应 GC 自动管理内存
pub struct MvccScheduler {
    /// MVCC 存储引擎
    store: Arc<MvccStore>,
    /// 配置
    config: MvccSchedulerConfig,
    /// 统计信息 (原子计数器)
    stats_successful: Arc<AtomicU64>,
    stats_failed: Arc<AtomicU64>,
    stats_conflict: Arc<AtomicU64>,
    stats_retry: Arc<AtomicU64>,
}

impl MvccScheduler {
    /// 创建新的调度器 (使用默认配置)
    pub fn new() -> Self {
        Self::new_with_config(MvccSchedulerConfig::default())
    }
    
    /// 使用指定配置创建调度器
    pub fn new_with_config(config: MvccSchedulerConfig) -> Self {
        let store = MvccStore::new_with_config(config.mvcc_config.clone());
        
        Self {
            store,
            config,
            stats_successful: Arc::new(AtomicU64::new(0)),
            stats_failed: Arc::new(AtomicU64::new(0)),
            stats_conflict: Arc::new(AtomicU64::new(0)),
            stats_retry: Arc::new(AtomicU64::new(0)),
        }
    }
    
    /// 获取底层 MVCC 存储的引用
    pub fn store(&self) -> &Arc<MvccStore> {
        &self.store
    }
    
    /// 获取执行统计信息
    pub fn get_stats(&self) -> MvccSchedulerStats {
        MvccSchedulerStats {
            successful_txs: self.stats_successful.load(Ordering::Relaxed),
            failed_txs: self.stats_failed.load(Ordering::Relaxed),
            conflict_count: self.stats_conflict.load(Ordering::Relaxed),
            retry_count: self.stats_retry.load(Ordering::Relaxed),
        }
    }
    
    /// 重置统计信息
    pub fn reset_stats(&self) {
        self.stats_successful.store(0, Ordering::Relaxed);
        self.stats_failed.store(0, Ordering::Relaxed);
        self.stats_conflict.store(0, Ordering::Relaxed);
        self.stats_retry.store(0, Ordering::Relaxed);
    }
    
    /// 执行单个交易 (带自动重试)
    /// 
    /// # 参数
    /// - `tx_id`: 交易 ID
    /// - `f`: 交易执行函数 (必须是 Fn 以支持重试)
    /// 
    /// # 返回
    /// 交易执行结果
    /// 
    /// # 注意
    /// 由于需要支持冲突重试，闭包必须实现 Fn 而不是 FnOnce
    pub fn execute_txn<F>(&self, tx_id: TxId, f: F) -> TxnResult
    where
        F: Fn(&mut Txn) -> Result<i32>,
    {
        let mut retries = 0;
        
        loop {
            // 开启新事务
            let mut txn = self.store.begin();
            
            // 执行业务逻辑
            let result = match f(&mut txn) {
                Ok(value) => value,
                Err(e) => {
                    // 业务逻辑错误，不重试
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
            
            // 尝试提交
            match txn.commit() {
                Ok(commit_ts) => {
                    // 提交成功
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
                    // 冲突，检查是否重试
                    self.stats_conflict.fetch_add(1, Ordering::Relaxed);
                    
                    if retries >= self.config.max_retries {
                        // 达到最大重试次数
                        self.stats_failed.fetch_add(1, Ordering::Relaxed);
                        if retries > 0 {
                            self.stats_retry.fetch_add(retries, Ordering::Relaxed);
                        }
                        return TxnResult {
                            tx_id,
                            return_value: None,
                            success: false,
                            error: Some(format!("Transaction aborted after {} retries: {}", retries, e)),
                            commit_ts: None,
                        };
                    }
                    
                    // 重试
                    retries += 1;
                    // 可选：添加短暂延迟避免活锁
                    std::thread::yield_now();
                }
            }
        }
    }
    
    /// 并行执行多个交易
    /// 
    /// 使用 rayon 并行执行，MVCC 自动处理冲突
    /// 
    /// # 参数
    /// - `txns`: 交易 ID 和执行函数的列表
    /// 
    /// # 返回
    /// 批量执行结果
    pub fn execute_batch<F>(&self, txns: Vec<(TxId, F)>) -> BatchTxnResult
    where
        F: Fn(&mut Txn) -> Result<i32> + Send + Sync,
    {
        // 使用 rayon 并行执行
        let results: Vec<TxnResult> = txns
            .into_par_iter()
            .map(|(tx_id, f)| self.execute_txn(tx_id, f))
            .collect();
        
        // 统计结果
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
    
    /// 执行只读交易 (使用快照隔离)
    /// 
    /// 只读事务不会产生冲突，性能更高
    /// 
    /// # 参数
    /// - `f`: 只读操作函数
    /// 
    /// # 返回
    /// 操作结果
    pub fn read_only<F, R>(&self, f: F) -> Result<R>
    where
        F: FnOnce(&mut Txn) -> Result<R>,
    {
        let mut txn = self.store.begin_read_only();
        f(&mut txn)
    }
    
    /// 批量写入 (单个事务)
    /// 
    /// # 参数
    /// - `writes`: 要写入的键值对列表
    /// 
    /// # 返回
    /// 写入成功返回提交时间戳
    pub fn batch_write(&self, writes: Vec<(Vec<u8>, Vec<u8>)>) -> Result<u64> {
        let mut txn = self.store.begin();
        
        for (key, value) in writes {
            txn.write(key, value);
        }
        
        txn.commit().map_err(|e| anyhow::anyhow!("Batch write failed: {}", e))
    }
    
    /// 批量读取 (快照隔离)
    /// 
    /// # 参数
    /// - `keys`: 要读取的键列表
    /// 
    /// # 返回
    /// 读取到的值列表 (Some表示存在, None表示不存在)
    pub fn batch_read(&self, keys: &[Vec<u8>]) -> Vec<Option<Vec<u8>>> {
        let mut txn = self.store.begin_read_only();
        
        keys.iter()
            .map(|key| txn.read(key).map(|v| v.to_vec()))
            .collect()
    }
    
    /// 批量删除 (单个事务)
    /// 
    /// # 参数
    /// - `keys`: 要删除的键列表
    /// 
    /// # 返回
    /// 删除成功返回提交时间戳
    pub fn batch_delete(&self, keys: Vec<Vec<u8>>) -> Result<u64> {
        let mut txn = self.store.begin();
        
        for key in keys {
            txn.delete(key);
        }
        
        txn.commit().map_err(|e| anyhow::anyhow!("Batch delete failed: {}", e))
    }
}

impl Default for MvccScheduler {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_mvcc_scheduler_basic() {
        let scheduler = MvccScheduler::new();
        
        // 执行单个交易
        let result = scheduler.execute_txn(1, |txn| {
            txn.write(b"key1".to_vec(), b"value1".to_vec());
            Ok(42)
        });
        
        assert!(result.success);
        assert_eq!(result.return_value, Some(42));
        
        // 验证写入
        let value = scheduler.read_only(|txn| {
            Ok(txn.read(b"key1").map(|v| v.to_vec()))
        }).unwrap();
        
        assert_eq!(value, Some(b"value1".to_vec()));
    }
    
    #[test]
    fn test_mvcc_scheduler_conflict() {
        let scheduler = MvccScheduler::new();
        
        // 先写入一个值
        scheduler.execute_txn(1, |txn| {
            txn.write(b"counter".to_vec(), b"0".to_vec());
            Ok(0)
        });
        
        // 并发修改同一个键
        let txns: Vec<_> = (0..10)
            .map(|i| {
                (i as TxId, |txn: &mut Txn| -> Result<i32> {
                    let val = txn
                        .read(b"counter")
                        .and_then(|v| {
                            std::str::from_utf8(v.as_ref())
                                .ok()
                                .and_then(|s| s.parse::<i32>().ok())
                        })
                        .unwrap_or(0);
                    
                    let new_val = val + 1;
                    txn.write(b"counter".to_vec(), new_val.to_string().into_bytes());
                    Ok(new_val)
                })
            })
            .collect();
        
        let batch_result = scheduler.execute_batch(txns);
        
        // 应该有一些成功，可能有一些冲突
        assert!(batch_result.successful > 0);
        println!("Successful: {}, Failed: {}, Conflicts: {}", 
            batch_result.successful, batch_result.failed, batch_result.conflicts);
    }
    
    #[test]
    fn test_mvcc_scheduler_batch_operations() {
        let scheduler = MvccScheduler::new();
        
        // 批量写入
        let writes = vec![
            (b"key1".to_vec(), b"value1".to_vec()),
            (b"key2".to_vec(), b"value2".to_vec()),
            (b"key3".to_vec(), b"value3".to_vec()),
        ];
        
        let commit_ts = scheduler.batch_write(writes).unwrap();
        assert!(commit_ts > 0);
        
        // 批量读取
        let keys = vec![b"key1".to_vec(), b"key2".to_vec(), b"key3".to_vec(), b"key4".to_vec()];
        let values = scheduler.batch_read(&keys);
        
        assert_eq!(values[0], Some(b"value1".to_vec()));
        assert_eq!(values[1], Some(b"value2".to_vec()));
        assert_eq!(values[2], Some(b"value3".to_vec()));
        assert_eq!(values[3], None);
        
        // 批量删除
        let delete_keys = vec![b"key1".to_vec(), b"key2".to_vec()];
        scheduler.batch_delete(delete_keys).unwrap();
        
        // 验证删除
        let remaining = scheduler.batch_read(&keys);
        assert_eq!(remaining[0], None);
        assert_eq!(remaining[1], None);
        assert_eq!(remaining[2], Some(b"value3".to_vec()));
    }
    
    #[test]
    fn test_mvcc_scheduler_stats() {
        let scheduler = MvccScheduler::new();
        
        // 执行一些交易
        for i in 0..10 {
            scheduler.execute_txn(i, |txn| {
                txn.write(format!("key{}", i).into_bytes(), b"value".to_vec());
                Ok(i as i32)
            });
        }
        
        let stats = scheduler.get_stats();
        assert_eq!(stats.successful_txs, 10);
        assert_eq!(stats.failed_txs, 0);
        assert_eq!(stats.total_txs(), 10);
        assert_eq!(stats.success_rate(), 1.0);
    }
}
