// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2025 XujueKing <leadbrand@me.com>

//! MVCC Scheduler Extensions for Cross-Shard Transactions
//!
//! 为 MvccScheduler 添加跨分片事务支持

use crate::ownership::ObjectId;
use crate::parallel_mvcc::MvccScheduler;
use crate::shard_types::*;
use anyhow::Result;
use parking_lot::RwLock;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;

// 简化复杂类型别名，降低类型复杂度
type ActiveLocks = Arc<RwLock<HashMap<TxnId, (HashSet<ObjectId>, u64)>>>;
type WaitGraph = Arc<RwLock<HashMap<TxnId, HashSet<TxnId>>>>;

/// 跨分片 MVCC 扩展（为 MvccScheduler 添加远程验证能力）
pub struct CrossShardMvccExt {
    /// 本地分片 ID（保留用于未来扩展，暂时未使用）
    _local_shard_id: ShardId,
    
    /// 活跃的跨分片事务锁
    /// txn_id -> (locked_objects, prepare_timestamp)
    active_locks: ActiveLocks,
    
    /// 等待图（用于死锁检测）
    /// txn_id -> 等待的事务集合
    wait_graph: WaitGraph,
}

impl CrossShardMvccExt {
    /// 创建跨分片扩展
    pub fn new(local_shard_id: ShardId) -> Self {
        Self {
            _local_shard_id: local_shard_id,
            active_locks: Arc::new(RwLock::new(HashMap::new())),
            wait_graph: Arc::new(RwLock::new(HashMap::new())),
        }
    }
    
    /// 处理 Prepare 请求（Phase 1）
    ///
    /// 验证读写集冲突并锁定对象
    pub fn handle_prepare(
        &self,
        scheduler: &MvccScheduler,
        request: PrepareRequest,
    ) -> PrepareResponse {
        let PrepareRequest {
            txn_id,
            shard_id: _,
            read_set,
            write_set,
            timestamp,
        } = request;
        
        // 1. 检查本地冲突
        if let Some(conflict) = self.check_local_conflicts(scheduler, &read_set, &write_set) {
            return PrepareResponse::VoteNo {
                txn_id,
                reason: conflict,
            };
        }
        
        // 2. 检查死锁风险
        if let Some(cycle) = self.detect_deadlock(txn_id) {
            return PrepareResponse::VoteNo {
                txn_id,
                reason: ConflictReason {
                    object_id: [0u8; 32],
                    expected_version: 0,
                    actual_version: 0,
                    description: format!("Deadlock detected in cycle: {:?}", cycle),
                },
            };
        }
        
        // 3. 锁定写集合中的对象
        let locked_objects: HashSet<_> = write_set.iter().map(|(obj_id, _)| *obj_id).collect();
        
        self.active_locks.write().insert(
            txn_id,
            (locked_objects, timestamp),
        );
        
        // 4. 投票同意
        PrepareResponse::VoteYes { txn_id }
    }
    
    /// 处理 Commit 请求（Phase 2）
    ///
    /// 应用写入并释放锁
    pub fn handle_commit(
        &self,
        _scheduler: &MvccScheduler,
        request: CommitRequest,
    ) -> CommitResponse {
        let CommitRequest { txn_id, decision } = request;
        
        match decision {
            Decision::Commit => {
                // 应用写入（通过 MVCC 提交）
                // 注意：实际写入应在 prepare 阶段缓存，这里只是示例
                self.release_locks(txn_id);
                
                CommitResponse {
                    txn_id,
                    status: CommitStatus::Success,
                }
            }
            Decision::Abort => {
                // 回滚并释放锁
                self.release_locks(txn_id);
                
                CommitResponse {
                    txn_id,
                    status: CommitStatus::Success,
                }
            }
        }
    }
    
    /// 检查本地冲突
    fn check_local_conflicts(
        &self,
        scheduler: &MvccScheduler,
        read_set: &[(ObjectId, u64)],
        write_set: &[(ObjectId, Vec<u8>)],
    ) -> Option<ConflictReason> {
        // 检查读集合版本冲突
        for (obj_id, expected_version) in read_set {
            // 从 MVCC store 读取当前版本
            let key = format!("obj_{}", hex::encode(obj_id));
            
            // 简化版本检查（实际需要从 MVCC metadata 获取版本）
            // TODO: 扩展 MvccStore API 暴露 get_version()
            
            // 临时模拟：假设版本号存储在特殊键中
            let version_key = format!("{}_version", key);
            
            let mut txn = scheduler.store().begin();
            if let Some(version_bytes) = txn.read(version_key.as_bytes()) {
                if let Ok(version_str) = String::from_utf8(version_bytes) {
                    if let Ok(actual_version) = version_str.parse::<u64>() {
                        if actual_version != *expected_version {
                            return Some(ConflictReason {
                                object_id: *obj_id,
                                expected_version: *expected_version,
                                actual_version,
                                description: "Read version mismatch".to_string(),
                            });
                        }
                    }
                }
            }
        }
        
        // 检查写写冲突（对象是否被其他事务锁定）
        let active_locks = self.active_locks.read();
        for (obj_id, _) in write_set {
            for (other_txn_id, (locked_objs, _)) in active_locks.iter() {
                if locked_objs.contains(obj_id) {
                    return Some(ConflictReason {
                        object_id: *obj_id,
                        expected_version: 0,
                        actual_version: 0,
                        description: format!("Object locked by txn {}", other_txn_id),
                    });
                }
            }
        }
        
        None
    }
    
    /// 释放事务持有的锁
    fn release_locks(&self, txn_id: TxnId) {
        self.active_locks.write().remove(&txn_id);
        self.wait_graph.write().remove(&txn_id);
    }
    
    /// 死锁检测（使用 DFS 检测等待图中的环）
    fn detect_deadlock(&self, txn_id: TxnId) -> Option<Vec<TxnId>> {
        let wait_graph = self.wait_graph.read();
        
        // 简化版死锁检测：只检查直接等待
        if let Some(waiting_for) = wait_graph.get(&txn_id) {
            // 检查是否有反向等待（A 等待 B，B 等待 A）
            for &other_txn in waiting_for {
                if let Some(other_waiting) = wait_graph.get(&other_txn) {
                    if other_waiting.contains(&txn_id) {
                        return Some(vec![txn_id, other_txn]);
                    }
                }
            }
        }
        
        None
    }
    
    /// 添加等待边（txn_a 等待 txn_b）
    pub fn add_wait_edge(&self, waiter: TxnId, holder: TxnId) {
        self.wait_graph
            .write()
            .entry(waiter)
            .or_default()
            .insert(holder);
    }
    
    /// 获取活跃锁数量（用于监控）
    pub fn active_lock_count(&self) -> usize {
        self.active_locks.read().len()
    }
}

/// 为 MvccScheduler 添加跨分片方法（通过扩展 trait）
pub trait CrossShardScheduler {
    /// 验证远程读集合（跨分片）
    fn validate_remote_reads(
        &self,
        remote_shard_id: ShardId,
        object_ids: Vec<ObjectId>,
    ) -> Result<HashMap<ObjectId, u64>>;
    
    /// 锁定对象（用于 2PC prepare 阶段）
    fn lock_objects(&self, txn_id: TxnId, objects: Vec<ObjectId>) -> Result<()>;
    
    /// 释放对象锁（用于 2PC commit/abort 阶段）
    fn unlock_objects(&self, txn_id: TxnId) -> Result<()>;
}

impl CrossShardScheduler for MvccScheduler {
    fn validate_remote_reads(
        &self,
        _remote_shard_id: ShardId,
        object_ids: Vec<ObjectId>,
    ) -> Result<HashMap<ObjectId, u64>> {
        let mut versions = HashMap::new();
        
        let mut txn = self.store().begin();
        
        for obj_id in object_ids {
            let key = format!("obj_{}", hex::encode(obj_id));
            let version_key = format!("{}_version", key);
            
            // 读取版本号
            if let Some(version_bytes) = txn.read(version_key.as_bytes()) {
                if let Ok(version_str) = String::from_utf8(version_bytes) {
                    if let Ok(version) = version_str.parse::<u64>() {
                        versions.insert(obj_id, version);
                    }
                }
            } else {
                // 对象不存在，版本为 0
                versions.insert(obj_id, 0);
            }
        }
        
        Ok(versions)
    }
    
    fn lock_objects(&self, _txn_id: TxnId, _objects: Vec<ObjectId>) -> Result<()> {
        // TODO: 实现对象级锁定（可能需要扩展 MvccStore）
        // 当前 MVCC 使用乐观锁，这里可以添加悲观锁支持
        Ok(())
    }
    
    fn unlock_objects(&self, _txn_id: TxnId) -> Result<()> {
        // TODO: 释放对象锁
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parallel_mvcc::MvccScheduler;

    #[test]
    fn test_cross_shard_ext_creation() {
        let ext = CrossShardMvccExt::new(0);
        assert_eq!(ext.active_lock_count(), 0);
    }
    
    #[test]
    fn test_prepare_no_conflict() {
        let ext = CrossShardMvccExt::new(0);
        let scheduler = MvccScheduler::new();
        
        let request = PrepareRequest {
            txn_id: 100,
            shard_id: 0,
            read_set: vec![],
            write_set: vec![([1u8; 32], vec![0x42])],
            timestamp: 1000,
        };
        
        let response = ext.handle_prepare(&scheduler, request);
        
        // 应该投票 Yes
        assert!(matches!(response, PrepareResponse::VoteYes { .. }));
        
        // 锁应该被记录
        assert_eq!(ext.active_lock_count(), 1);
    }
    
    #[test]
    fn test_commit_releases_locks() {
        let ext = CrossShardMvccExt::new(0);
        let scheduler = MvccScheduler::new();
        
        // 先 prepare
        let prepare_req = PrepareRequest {
            txn_id: 100,
            shard_id: 0,
            read_set: vec![],
            write_set: vec![([1u8; 32], vec![0x42])],
            timestamp: 1000,
        };
        ext.handle_prepare(&scheduler, prepare_req);
        
        assert_eq!(ext.active_lock_count(), 1);
        
        // 然后 commit
        let commit_req = CommitRequest {
            txn_id: 100,
            decision: Decision::Commit,
        };
        ext.handle_commit(&scheduler, commit_req);
        
        // 锁应该被释放
        assert_eq!(ext.active_lock_count(), 0);
    }
    
    #[test]
    fn test_deadlock_detection() {
        let ext = CrossShardMvccExt::new(0);
        
        // 创建循环等待：txn 1 等待 txn 2，txn 2 等待 txn 1
        ext.add_wait_edge(1, 2);
        ext.add_wait_edge(2, 1);
        
        // 应该检测到死锁
        let cycle = ext.detect_deadlock(1);
        assert!(cycle.is_some());
        
        let cycle = cycle.unwrap();
        assert_eq!(cycle.len(), 2);
    }
}
