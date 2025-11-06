// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2025 XujueKing <leadbrand@me.com>

// SuperVM 2.0 - Unified Entry and Mode Routing
// 架构师: KING XU (CHINA)
// 日期: 2025-11-04
//
// 目标：提供统一入口，根据隐私模式与对象所有权路由到快速/共识/隐私路径

use crate::{OwnershipManager, ObjectId, Address};
#[cfg(feature = "groth16-verifier")]
use crate::privacy::{ZkVerifier, ZkCircuitId, ZkError};
use crate::parallel_mvcc::{MvccScheduler, BatchTxnResult, TxId};
use crate::mvcc::Txn;
use std::sync::Arc;

// Type aliases for complex transaction types
type TxnOperation = Arc<dyn Fn(&mut Txn) -> anyhow::Result<i32> + Send + Sync>;
type TxnTuple = (TxId, Transaction, TxnOperation);
type TxnItem = (TxId, TxnOperation);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Privacy {
    Public,
    Private,
}

#[derive(Debug, Clone)]
pub struct Transaction {
    pub from: Address,
    pub objects: Vec<ObjectId>,
    pub privacy: Privacy,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExecutionPath {
    FastPath,        // 独占/不可变对象，公开模式
    ConsensusPath,   // 共享对象或强一致需求，公开模式
    PrivatePath,     // 隐私模式
}

#[derive(Debug, Clone)]
pub struct ExecutionReceipt {
    pub path: ExecutionPath,
    pub accepted: bool,
    pub reason: Option<String>,
    pub success: bool,
    pub fallback_to_consensus: bool,
    pub return_value: Option<i32>,
    pub latency_ms: u64,
}

pub struct SuperVM<'a> {
    ownership: &'a OwnershipManager,
    scheduler: Option<&'a MvccScheduler>,
    /// Optional ZK verifier (feature-gated usage)
    #[cfg(feature = "groth16-verifier")]
    zk: Option<&'a dyn ZkVerifier>,
}

impl<'a> SuperVM<'a> {
    pub fn new(ownership: &'a OwnershipManager) -> Self {
        Self {
            ownership,
            scheduler: None,
            #[cfg(feature = "groth16-verifier")]
            zk: None,
        }
    }

    pub fn with_scheduler(mut self, scheduler: &'a MvccScheduler) -> Self {
        self.scheduler = Some(scheduler);
        self
    }

    /// 注入可选的 ZK 验证器（最小接入）
    #[cfg(feature = "groth16-verifier")]
    pub fn with_verifier(mut self, verifier: &'a dyn ZkVerifier) -> Self {
        self.zk = Some(verifier);
        self
    }

    /// 供上层在进入隐私路径前主动调用的验证入口（可选）
    #[cfg(feature = "groth16-verifier")]
    pub fn verify_with(&self, circuit: &ZkCircuitId, proof: &[u8], public_inputs: &[u8]) -> Result<bool, ZkError> {
        if let Some(v) = self.zk {
            v.verify_proof(circuit, proof, public_inputs)
        } else {
            // 未配置验证器时，默认返回 false（由调用方决定是否放行）
            Ok(false)
        }
    }

    /// 路由到执行路径（不执行）
    pub fn route(&self, tx: &Transaction) -> ExecutionPath {
        match tx.privacy {
            Privacy::Private => ExecutionPath::PrivatePath,
            Privacy::Public => {
                if self.ownership.should_use_fast_path(&tx.objects) {
                    ExecutionPath::FastPath
                } else {
                    ExecutionPath::ConsensusPath
                }
            }
        }
    }

    /// 执行交易（当前：做基本校验与路径判定；不附带业务执行）
    pub fn execute_transaction(&self, tx: &Transaction) -> ExecutionReceipt {
        let path = self.route(tx);
        let mut accepted = true;
        let mut reason = None;

        // 基础权限校验（最小原型）：公开模式写共享对象无需校验；
        // 对独占对象的写入需调用者拥有。这里先假设所有访问为写。
        if matches!(path, ExecutionPath::FastPath) {
            // 快速路径：所有对象必须为独占或不可变
            for obj in &tx.objects {
                if let Some(owner) = self.ownership.get_ownership_type(obj) {
                    match owner {
                        crate::OwnershipType::Owned(addr) => {
                            if addr != tx.from {
                                accepted = false;
                                reason = Some("fast path requires owner to be sender".into());
                                break;
                            }
                        }
                        crate::OwnershipType::Immutable => {
                            // 只读，允许（最小原型不区分读写）
                        }
                        crate::OwnershipType::Shared => {
                            accepted = false;
                            reason = Some("shared object cannot go fast path".into());
                            break;
                        }
                    }
                } else {
                    accepted = false;
                    reason = Some("object not found".into());
                    break;
                }
            }
        }

        ExecutionReceipt { path, accepted, reason, success: false, fallback_to_consensus: false, return_value: None, latency_ms: 0 }
    }

    /// 单笔执行（带业务闭包）并按路径执行；Fast 失败时回退到共识重试一次
    pub fn execute_transaction_with<F>(&self, tx_id: TxId, tx: &Transaction, f: F) -> ExecutionReceipt
    where
        F: Fn(&mut Txn) -> anyhow::Result<i32>,
    {
        let start = std::time::Instant::now();
        let path = self.route(tx);
        let scheduler = self.scheduler.expect("SuperVM: scheduler not configured, call with_scheduler()");

        match path {
            ExecutionPath::FastPath => {
                // Fast 通道（MVP：先复用 MVCC 执行器）。失败则单次回退 Consensus。
                let r = scheduler.execute_txn(tx_id, |txn| f(txn));
                if r.success {
                    return ExecutionReceipt {
                        path,
                        accepted: true,
                        reason: None,
                        success: true,
                        fallback_to_consensus: false,
                        return_value: r.return_value,
                        latency_ms: start.elapsed().as_millis() as u64,
                    };
                }
                // 回退一次到共识
                let r2 = scheduler.execute_txn(tx_id, |txn| f(txn));
                ExecutionReceipt {
                    path: ExecutionPath::ConsensusPath,
                    accepted: true,
                    reason: r.error.or_else(|| r2.error.clone()),
                    success: r2.success,
                    fallback_to_consensus: true,
                    return_value: r2.return_value,
                    latency_ms: start.elapsed().as_millis() as u64,
                }
            }
            ExecutionPath::ConsensusPath => {
                let r = scheduler.execute_txn(tx_id, |txn| f(txn));
                ExecutionReceipt {
                    path,
                    accepted: true,
                    reason: r.error.clone(),
                    success: r.success,
                    fallback_to_consensus: false,
                    return_value: r.return_value,
                    latency_ms: start.elapsed().as_millis() as u64,
                }
            }
            ExecutionPath::PrivatePath => {
                // Phase 1.3: 隐私通道占位（暂按共识执行，标注路径并统计）
                // TODO Phase 2: 接入环签名、隐形地址、zkProof 验证
                self.ownership.record_transaction_path(false); // 记录为非快速路径
                let r = scheduler.execute_txn(tx_id, |txn| f(txn));
                ExecutionReceipt {
                    path,
                    accepted: true,
                    reason: r.error.clone(),
                    success: r.success,
                    fallback_to_consensus: false,
                    return_value: r.return_value,
                    latency_ms: start.elapsed().as_millis() as u64,
                }
            }
        }
    }

    /// 根据对象所有权为批量交易路由路径并分别执行
    /// 注意：当前最小实现——Fast/Consensus 均使用同一 MvccScheduler 执行，
    /// 区别仅在于路由与统计；后续可将 Fast Path 切换为无共识快速通道。
    pub fn execute_batch_routed<F>(&self, txs: Vec<(TxId, Transaction, F)>) -> (BatchTxnResult, BatchTxnResult)
    where
        F: Fn(&mut Txn) -> anyhow::Result<i32> + Send + Sync,
    {
        let scheduler = self.scheduler.expect("SuperVM: scheduler not configured, call with_scheduler()");

        // 划分 fast 与 consensus 两组
        let mut fast_group: Vec<(TxId, F)> = Vec::new();
        let mut consensus_group: Vec<(TxId, F)> = Vec::new();

        for (id, tx, f) in txs {
            let path = self.route(&tx);
            match path {
                ExecutionPath::FastPath => fast_group.push((id, f)),
                ExecutionPath::ConsensusPath => consensus_group.push((id, f)),
                ExecutionPath::PrivatePath => {
                    // 最小原型：隐私路径暂时按共识路径处理（后续接入隐私验证与专用执行引擎）
                    consensus_group.push((id, f));
                }
            }
        }

        let fast_result = if fast_group.is_empty() {
            BatchTxnResult { successful: 0, failed: 0, conflicts: 0, results: vec![] }
        } else {
            scheduler.execute_batch(fast_group)
        };

        let consensus_result = if consensus_group.is_empty() {
            BatchTxnResult { successful: 0, failed: 0, conflicts: 0, results: vec![] }
        } else {
            scheduler.execute_batch(consensus_group)
        };

        (fast_result, consensus_result)
    }

    /// 带回退统计的批量执行（使用可复用闭包）
    pub fn execute_batch_routed_with_fallback(
        &self,
        txs: Vec<TxnTuple>,
    ) -> (BatchTxnResult, BatchTxnResult, u64) {
        let scheduler = self.scheduler.expect("SuperVM: scheduler not configured, call with_scheduler()");

        // 分桶
        let mut fast_items: Vec<TxnItem> = Vec::new();
        let mut consensus_items: Vec<TxnItem> = Vec::new();
        for (id, tx, f) in txs.into_iter() {
            match self.route(&tx) {
                ExecutionPath::FastPath => fast_items.push((id, f)),
                ExecutionPath::ConsensusPath | ExecutionPath::PrivatePath => consensus_items.push((id, f)),
            }
        }

        // 执行 fast
        let fast_result = if fast_items.is_empty() {
            BatchTxnResult { successful: 0, failed: 0, conflicts: 0, results: vec![] }
        } else {
            let cloned: Vec<(TxId, _)> = fast_items
                .iter()
                .map(|(id, f)| (*id, {
                    let f = f.clone();
                    move |txn: &mut Txn| (f)(txn)
                }))
                .collect();
            scheduler.execute_batch(cloned)
        };

        // 将 fast 失败的回退到 consensus
        let mut fast_fallbacks: u64 = 0;
        if !fast_items.is_empty() {
            let failed_ids: std::collections::HashSet<TxId> = fast_result
                .results
                .iter()
                .filter(|r| !r.success)
                .map(|r| r.tx_id)
                .collect();
            if !failed_ids.is_empty() {
                fast_fallbacks = failed_ids.len() as u64;
                for (id, f) in fast_items.into_iter() {
                    if failed_ids.contains(&id) {
                        consensus_items.push((id, f));
                    }
                }
            }
        }

        // 执行 consensus（含原始 + 回退）
        let consensus_result = if consensus_items.is_empty() {
            BatchTxnResult { successful: 0, failed: 0, conflicts: 0, results: vec![] }
        } else {
            let cloned: Vec<(TxId, _)> = consensus_items
                .iter()
                .map(|(id, f)| (*id, {
                    let f = f.clone();
                    move |txn: &mut Txn| (f)(txn)
                }))
                .collect();
            scheduler.execute_batch(cloned)
        };

        (fast_result, consensus_result, fast_fallbacks)
    }

    /// 标准批量执行入口（带路由与回退）
    pub fn execute_batch(
        &self,
        txs: Vec<TxnTuple>,
    ) -> (BatchTxnResult, BatchTxnResult, u64) {
        self.execute_batch_routed_with_fallback(txs)
    }
}

