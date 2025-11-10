// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2025 XujueKing <leadbrand@me.com>

//! Shard Coordinator - 跨分片事务协调器
//!
//! 实现两阶段提交协议 (2PC)

use crate::cross_shard_mvcc::CrossShardMvccExt;
use crate::shard_types::*;
use crate::ownership::ObjectId;
use parking_lot::RwLock;
use std::collections::HashMap;
#[cfg(feature = "cross-shard")]
use crate::shard::proto::shard_service_client::ShardServiceClient;
#[cfg(feature = "cross-shard")]
use crate::shard::proto::*;
#[cfg(feature = "cross-shard")]
use tonic::transport::Channel;
use std::sync::Arc;

/// 分片协调器（运行在事务发起节点）
pub struct ShardCoordinator {
    /// 分片配置
    config: ShardConfig,
    
    /// 活跃的跨分片事务（txn_id -> 元数据）
    active_txns: Arc<RwLock<HashMap<TxnId, CrossShardTxn>>>,
    
    /// 事务 ID 生成器（原子计数器）
    next_txn_id: Arc<parking_lot::Mutex<TxnId>>,
    
    /// MVCC 扩展（处理本地 prepare/commit）
    #[allow(dead_code)]
    mvcc_ext: Arc<CrossShardMvccExt>,
    
    /// RPC 客户端
    #[cfg(feature = "cross-shard")]
    rpc_clients: HashMap<ShardId, ShardServiceClient<Channel>>,
    #[cfg(not(feature = "cross-shard"))]
    _rpc_clients: HashMap<ShardId, ()>,
}

impl ShardCoordinator {
    /// 创建新的协调器实例
    pub fn new(config: ShardConfig) -> Self {
        let mvcc_ext = Arc::new(CrossShardMvccExt::new(config.local_shard_id));
        
        Self {
            config,
            active_txns: Arc::new(RwLock::new(HashMap::new())),
            next_txn_id: Arc::new(parking_lot::Mutex::new(1)),
            mvcc_ext,
            #[cfg(feature = "cross-shard")]
            rpc_clients: HashMap::new(),
            #[cfg(not(feature = "cross-shard"))]
            _rpc_clients: HashMap::new(),
        }
    }

    /// 建立到所有分片节点的 gRPC 连接（在 cross-shard 启用时可用）
    #[cfg(feature = "cross-shard")]
    pub async fn connect_all(&mut self) -> anyhow::Result<()> {
        for (sid, endpoint) in &self.config.shard_endpoints {
            let client = ShardServiceClient::connect(format!("http://{}", endpoint)).await?;
            self.rpc_clients.insert(*sid, client);
        }
        Ok(())
    }

    /// 并行 prepare 所有参与分片（在 cross-shard 启用时可用）
    #[cfg(feature = "cross-shard")]
    pub async fn prepare_all(&mut self, reqs: Vec<(ShardId, PrepareRequest)>) -> anyhow::Result<Vec<(ShardId, PrepareResponse)>> {
        use futures::future::join_all;
        let mut futs = Vec::with_capacity(reqs.len());
        for (sid, req) in reqs.into_iter() {
            let client = self.rpc_clients.get_mut(&sid).expect("client connected");
            futs.push(async move {
                let resp = client.prepare_txn(req).await;
                (sid, resp)
            });
        }
        let results = join_all(futs).await;
        let mut out = Vec::with_capacity(results.len());
        for (sid, r) in results {
            let resp = r?.into_inner();
            out.push((sid, resp));
        }
        Ok(out)
    }

    /// 远程批量查询对象版本（按分片聚合后并行请求）
    #[cfg(feature = "cross-shard")]
    pub async fn get_remote_versions(&mut self, shard_to_objects: HashMap<ShardId, Vec<ObjectId>>) -> anyhow::Result<HashMap<ObjectId, u64>> {
        use futures::future::join_all;
        let mut futs = Vec::with_capacity(shard_to_objects.len());
        for (sid, objs) in shard_to_objects.into_iter() {
            let mut client = self.rpc_clients.get_mut(&sid).expect("client connected").clone();
            let req = VersionRequest { object_ids: objs.iter().map(|o| o.to_vec()).collect() };
            futs.push(async move {
                let resp = client.get_object_versions(req).await;
                (sid, resp)
            });
        }
        let results = join_all(futs).await;
        let mut out: HashMap<ObjectId, u64> = HashMap::new();
        for (_sid, r) in results {
            let resp = r?.into_inner();
            for ov in resp.versions {
                let mut id = [0u8;32];
                let len = ov.object_id.len().min(32);
                id[..len].copy_from_slice(&ov.object_id[..len]);
                out.insert(id, ov.version);
            }
        }
        Ok(out)
    }

    /// 并行下发最终决议（在 cross-shard 启用时可用）
    #[cfg(feature = "cross-shard")]
    pub async fn commit_all(&mut self, sid_list: Vec<ShardId>, decision: Decision, txn_id: u64, epoch: u64) -> anyhow::Result<()> {
        use futures::future::join_all;
        let mut futs = Vec::new();
        for sid in sid_list {
            let client = self.rpc_clients.get_mut(&sid).expect("client connected");
            let req = CommitRequest { txn_id, decision: decision as i32, coordinator_epoch: epoch };
            futs.push(async move { client.commit_txn(req).await });
        }
        let results = join_all(futs).await;
        for r in results { r?; }
        Ok(())
    }
    
    /// 生成新的事务 ID
    fn generate_txn_id(&self) -> TxnId {
        let mut id = self.next_txn_id.lock();
        let txn_id = *id;
        *id += 1;
        txn_id
    }
    
    /// 计算事务涉及的分片列表
    fn compute_participant_shards(
        &self,
        read_set: &[(ObjectId, u64)],
        write_set: &[(ObjectId, Vec<u8>)],
    ) -> Vec<ShardId> {
        let mut shards = std::collections::HashSet::new();
        
        for (obj_id, _) in read_set {
            let shard = shard_for_object(obj_id, self.config.num_shards);
            shards.insert(shard);
        }
        
        for (obj_id, _) in write_set {
            let shard = shard_for_object(obj_id, self.config.num_shards);
            shards.insert(shard);
        }
        
        shards.into_iter().collect()
    }
    
    /// 执行跨分片事务
    ///
    /// # Arguments
    /// * `read_set` - 读集合 (object_id, expected_version)
    /// * `write_set` - 写集合 (object_id, new_data)
    ///
    /// # Returns
    /// * `Ok(true)` - 事务提交成功
    /// * `Ok(false)` - 事务被中止（冲突）
    /// * `Err(_)` - 系统错误（网络故障、超时等）
    pub fn execute_cross_shard_txn(
        &self,
        read_set: Vec<(ObjectId, u64)>,
        write_set: Vec<(ObjectId, Vec<u8>)>,
    ) -> Result<bool, CoordinatorError> {
        // 1. 生成事务 ID
        let txn_id = self.generate_txn_id();
        
        // 2. 计算参与分片
        let participant_shards = self.compute_participant_shards(&read_set, &write_set);
        
        // 优化：单分片事务走快速路径
        if participant_shards.len() == 1 {
            return self.execute_single_shard_txn(
                participant_shards[0],
                read_set,
                write_set,
            );
        }
        
        // 3. 创建事务元数据
        let mut txn = CrossShardTxn::new(txn_id, participant_shards.clone());
        txn.state = TxnState::Init;
        self.active_txns.write().insert(txn_id, txn.clone());
        
        // 4. Phase 1: Prepare
        let prepare_result = self.phase_prepare(txn_id, &participant_shards, &read_set, &write_set)?;
        
        // 5. 根据投票决定提交或中止
        let decision = if prepare_result {
            Decision::Commit
        } else {
            Decision::Abort
        };
        
        // 6. Phase 2: Commit/Abort
        self.phase_commit(txn_id, &participant_shards, decision)?;
        
        // 7. 清理元数据
        self.active_txns.write().remove(&txn_id);
        
        Ok(decision == Decision::Commit)
    }
    
    /// Phase 1: 向所有参与分片发送 Prepare 请求
    fn phase_prepare(
        &self,
        txn_id: TxnId,
        participant_shards: &[ShardId],
        read_set: &[(ObjectId, u64)],
        write_set: &[(ObjectId, Vec<u8>)],
    ) -> Result<bool, CoordinatorError> {
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        
        // 更新状态
        if let Some(txn) = self.active_txns.write().get_mut(&txn_id) {
            txn.state = TxnState::Preparing;
        }
        
        // 向每个分片发送 prepare 请求（并行）
        let mut all_votes_yes = true;
        
        for &shard_id in participant_shards {
            // 按分片过滤读写集
            let local_reads: Vec<_> = read_set
                .iter()
                .filter(|(obj_id, _)| shard_for_object(obj_id, self.config.num_shards) == shard_id)
                .cloned()
                .collect();
            
            let local_writes: Vec<_> = write_set
                .iter()
                .filter(|(obj_id, _)| shard_for_object(obj_id, self.config.num_shards) == shard_id)
                .cloned()
                .collect();
            
            let request = PrepareRequest {
                txn_id,
                shard_id,
                read_set: local_reads,
                write_set: local_writes,
                timestamp,
            };
            
            // TODO: 发送 RPC 请求
            // 现在使用模拟逻辑（假设本地调用）
            let response = self.simulate_prepare_rpc(shard_id, request)?;
            
            // 记录投票
            if let Some(txn) = self.active_txns.write().get_mut(&txn_id) {
                txn.votes.insert(shard_id, response.clone());
            }
            
            // 检查投票结果
            match response {
                PrepareResponse::VoteYes { .. } => {},
                PrepareResponse::VoteNo { .. } => {
                    all_votes_yes = false;
                    break; // 提前终止（优化）
                }
            }
        }
        
        // 更新状态
        if let Some(txn) = self.active_txns.write().get_mut(&txn_id) {
            txn.state = if all_votes_yes {
                TxnState::Prepared
            } else {
                TxnState::Aborted
            };
        }
        
        Ok(all_votes_yes)
    }
    
    /// Phase 2: 向所有参与分片发送 Commit/Abort 请求
    fn phase_commit(
        &self,
        txn_id: TxnId,
        participant_shards: &[ShardId],
        decision: Decision,
    ) -> Result<(), CoordinatorError> {
        // 更新状态
        if let Some(txn) = self.active_txns.write().get_mut(&txn_id) {
            txn.state = match decision {
                Decision::Commit => TxnState::Committing,
                Decision::Abort => TxnState::Aborted,
            };
        }
        
        // 向所有分片发送决策
        for &shard_id in participant_shards {
            let request = CommitRequest { txn_id, decision };
            
            // TODO: 发送 RPC 请求
            let _response = self.simulate_commit_rpc(shard_id, request)?;
        }
        
        // 更新最终状态
        if let Some(txn) = self.active_txns.write().get_mut(&txn_id) {
            txn.state = match decision {
                Decision::Commit => TxnState::Committed,
                Decision::Abort => TxnState::Aborted,
            };
        }
        
        Ok(())
    }
    
    /// 单分片事务快速路径（跳过 2PC）
    fn execute_single_shard_txn(
        &self,
        _shard_id: ShardId,
        _read_set: Vec<(ObjectId, u64)>,
        _write_set: Vec<(ObjectId, Vec<u8>)>,
    ) -> Result<bool, CoordinatorError> {
        // 直接提交（TODO: 调用本地 MVCC 调度器）
        let _request = CommitRequest {
            txn_id: self.generate_txn_id(),
            decision: Decision::Commit,
        };
        
        // 模拟：假设总是成功
        Ok(true)
    }
    
    /// 模拟 Prepare RPC（临时实现）
    fn simulate_prepare_rpc(
        &self,
        _shard_id: ShardId,
        request: PrepareRequest,
    ) -> Result<PrepareResponse, CoordinatorError> {
        // TODO: 替换为真实 gRPC 调用
        
        // 模拟：90% 概率投 Yes
        let vote_yes = rand::random::<f64>() > 0.1;
        
        if vote_yes {
            Ok(PrepareResponse::VoteYes {
                txn_id: request.txn_id,
            })
        } else {
            Ok(PrepareResponse::VoteNo {
                txn_id: request.txn_id,
                reason: ConflictReason {
                    object_id: [0u8; 32],
                    expected_version: 1,
                    actual_version: 2,
                    description: "simulated conflict".to_string(),
                },
            })
        }
    }
    
    /// 模拟 Commit RPC（临时实现）
    fn simulate_commit_rpc(
        &self,
        _shard_id: ShardId,
        _request: CommitRequest,
    ) -> Result<CommitResponse, CoordinatorError> {
        // TODO: 替换为真实 gRPC 调用
        Ok(CommitResponse {
            txn_id: _request.txn_id,
            status: CommitStatus::Success,
        })
    }
    
    /// 获取活跃事务数量
    pub fn active_txn_count(&self) -> usize {
        self.active_txns.read().len()
    }
    
    /// 获取事务状态（用于监控）
    pub fn get_txn_state(&self, txn_id: TxnId) -> Option<TxnState> {
        self.active_txns.read().get(&txn_id).map(|txn| txn.state)
    }
}

/// 协调器错误类型
#[derive(Debug, thiserror::Error)]
pub enum CoordinatorError {
    #[error("RPC timeout for shard {0}")]
    RpcTimeout(ShardId),
    
    #[error("Network error: {0}")]
    NetworkError(String),
    
    #[error("Transaction {0} not found")]
    TxnNotFound(TxnId),
    
    #[error("Invalid state transition")]
    InvalidState,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_coordinator_basic() {
        let config = ShardConfig {
            num_shards: 4,
            shard_endpoints: HashMap::new(),
            timeout_ms: 5000,
            local_shard_id: 0,
        };
        
        let coordinator = ShardCoordinator::new(config);
        
        // 生成事务 ID
        let txn_id1 = coordinator.generate_txn_id();
        let txn_id2 = coordinator.generate_txn_id();
        assert_ne!(txn_id1, txn_id2);
        assert!(txn_id2 > txn_id1);
    }
    
    #[test]
    fn test_compute_participant_shards() {
        let config = ShardConfig {
            num_shards: 4,
            ..Default::default()
        };
        
        let coordinator = ShardCoordinator::new(config);
        
        let obj1 = [1u8; 32];
        let obj2 = [2u8; 32];
        
        let read_set = vec![(obj1, 1)];
        let write_set = vec![(obj2, vec![0x42])];
        
        let shards = coordinator.compute_participant_shards(&read_set, &write_set);
        
        // 应该涉及 1-2 个分片（取决于哈希结果）
        assert!(!shards.is_empty());
        assert!(shards.len() <= 2);
    }
    
    #[test]
    fn test_cross_shard_txn_simulation() {
        let config = ShardConfig {
            num_shards: 2,
            ..Default::default()
        };
        
        let coordinator = ShardCoordinator::new(config);
        
        let obj1 = [1u8; 32];
        let obj2 = [255u8; 32]; // 确保路由到不同分片
        
        let read_set = vec![(obj1, 1)];
        let write_set = vec![(obj2, vec![0x42, 0x43])];
        
        // 执行跨分片事务（可能成功也可能中止）
        let result = coordinator.execute_cross_shard_txn(read_set, write_set);
        
        assert!(result.is_ok());
        
        // 事务完成后应该被清理
        assert_eq!(coordinator.active_txn_count(), 0);
    }
}
