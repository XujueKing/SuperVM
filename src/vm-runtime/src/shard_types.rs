// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2025 XujueKing <leadbrand@me.com>

//! Shard Types and Common Definitions
//!
//! 跨分片事务的核心数据类型定义

use crate::ownership::{ObjectId, Version};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// 分片 ID 类型
pub type ShardId = u16;

/// 事务 ID 类型（全局唯一）
pub type TxnId = u64;

/// 分片配置
#[derive(Debug, Clone)]
pub struct ShardConfig {
    /// 分片总数
    pub num_shards: usize,
    
    /// 各分片的 gRPC 端点地址
    /// 格式: "127.0.0.1:5000"
    pub shard_endpoints: HashMap<ShardId, String>,
    
    /// RPC 超时时间（毫秒）
    pub timeout_ms: u64,
    
    /// 本分片 ID
    pub local_shard_id: ShardId,
}

impl Default for ShardConfig {
    fn default() -> Self {
        Self {
            num_shards: 1,
            shard_endpoints: HashMap::new(),
            timeout_ms: 5000, // 5秒
            local_shard_id: 0,
        }
    }
}

/// 2PC 决策类型
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Decision {
    /// 提交事务
    Commit,
    /// 中止事务
    Abort,
}

/// 冲突原因详细信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConflictReason {
    /// 冲突的对象 ID
    pub object_id: ObjectId,
    
    /// 事务期望的版本
    pub expected_version: Version,
    
    /// 实际存储的版本
    pub actual_version: Version,
    
    /// 冲突描述
    pub description: String,
}

/// Phase 1: Prepare 请求
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrepareRequest {
    /// 事务 ID
    pub txn_id: TxnId,
    
    /// 目标分片 ID
    pub shard_id: ShardId,
    
    /// 读集合（对象 ID + 期望版本）
    pub read_set: Vec<(ObjectId, Version)>,
    
    /// 写集合（对象 ID + 新数据）
    pub write_set: Vec<(ObjectId, Vec<u8>)>,
    
    /// 事务时间戳
    pub timestamp: u64,
}

/// Phase 1: Prepare 响应
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PrepareResponse {
    /// 投票同意（可以提交）
    VoteYes { txn_id: TxnId },
    
    /// 投票拒绝（检测到冲突）
    VoteNo { 
        txn_id: TxnId, 
        reason: ConflictReason 
    },
}

/// Phase 2: Commit/Abort 请求
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommitRequest {
    /// 事务 ID
    pub txn_id: TxnId,
    
    /// 协调器的最终决策
    pub decision: Decision,
}

/// Commit 状态
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CommitStatus {
    /// 成功
    Success,
    /// 失败（如：事务已超时）
    Failed,
}

/// Phase 2: Commit/Abort 响应
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommitResponse {
    /// 事务 ID
    pub txn_id: TxnId,
    
    /// 执行状态
    pub status: CommitStatus,
}

/// 查询对象版本请求（用于远程验证）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VersionRequest {
    /// 要查询的对象 ID 列表
    pub object_ids: Vec<ObjectId>,
}

/// 查询对象版本响应
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VersionResponse {
    /// 对象 ID -> 当前版本
    pub versions: HashMap<ObjectId, Version>,
}

/// 事务状态（协调器侧）
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TxnState {
    /// 初始状态
    Init,
    
    /// 正在准备阶段
    Preparing,
    
    /// 所有分片已准备好
    Prepared,
    
    /// 正在提交
    Committing,
    
    /// 已提交
    Committed,
    
    /// 已中止
    Aborted,
}

/// 跨分片事务元数据
#[derive(Debug, Clone)]
pub struct CrossShardTxn {
    /// 事务 ID
    pub txn_id: TxnId,
    
    /// 参与的分片列表
    pub participant_shards: Vec<ShardId>,
    
    /// 事务状态
    pub state: TxnState,
    
    /// 各分片的投票结果
    pub votes: HashMap<ShardId, PrepareResponse>,
    
    /// 创建时间戳
    pub created_at: u64,
}

impl CrossShardTxn {
    pub fn new(txn_id: TxnId, participant_shards: Vec<ShardId>) -> Self {
        Self {
            txn_id,
            participant_shards,
            state: TxnState::Init,
            votes: HashMap::new(),
            created_at: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
        }
    }
    
    /// 检查是否所有分片都投了 Yes
    pub fn all_votes_yes(&self) -> bool {
        if self.votes.len() != self.participant_shards.len() {
            return false;
        }
        
        self.votes.values().all(|vote| matches!(vote, PrepareResponse::VoteYes { .. }))
    }
    
    /// 检查是否有任何分片投了 No
    pub fn any_vote_no(&self) -> bool {
        self.votes.values().any(|vote| matches!(vote, PrepareResponse::VoteNo { .. }))
    }
}

/// 计算对象所属分片
pub fn shard_for_object(object_id: &ObjectId, num_shards: usize) -> ShardId {
    // 使用简单哈希（生产环境可替换为 xxhash）
    let hash: u64 = object_id.iter().fold(0u64, |acc, &byte| {
        acc.wrapping_mul(31).wrapping_add(byte as u64)
    });
    (hash % num_shards as u64) as ShardId
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_shard_for_object() {
        let obj_id = [1u8; 32];
        let shard = shard_for_object(&obj_id, 4);
        assert!(shard < 4);
        
        // 确定性：相同对象始终路由到同一分片
        let shard2 = shard_for_object(&obj_id, 4);
        assert_eq!(shard, shard2);
    }
    
    #[test]
    fn test_cross_shard_txn_voting() {
        let mut txn = CrossShardTxn::new(100, vec![0, 1, 2]);
        
        // 未收集完所有投票
        assert!(!txn.all_votes_yes());
        
        // 全部投 Yes
        txn.votes.insert(0, PrepareResponse::VoteYes { txn_id: 100 });
        txn.votes.insert(1, PrepareResponse::VoteYes { txn_id: 100 });
        txn.votes.insert(2, PrepareResponse::VoteYes { txn_id: 100 });
        assert!(txn.all_votes_yes());
        
        // 一个投 No
        txn.votes.insert(2, PrepareResponse::VoteNo {
            txn_id: 100,
            reason: ConflictReason {
                object_id: [0u8; 32],
                expected_version: 5,
                actual_version: 6,
                description: "version mismatch".to_string(),
            },
        });
        assert!(!txn.all_votes_yes());
        assert!(txn.any_vote_no());
    }
}
