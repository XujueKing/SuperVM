// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2025 XujueKing <leadbrand@me.com>

// SuperVM 2.0 - Unified Entry and Mode Routing
// 架构师: KING XU (CHINA)
// 日期: 2025-11-04
//
// 目标：提供统一入口，根据隐私模式与对象所有权路由到快速/共识/隐私路径

use crate::mvcc::Txn;
use crate::parallel::{FastPathExecutor, FastPathStats};
use crate::parallel_mvcc::{BatchTxnResult, MvccScheduler, TxId};
#[cfg(feature = "groth16-verifier")]
use crate::zk_verifier::ZkVerifier;
use crate::adaptive_router::AdaptiveRouter; // 自适应路由器
use crate::{Address, ObjectId, OwnershipManager};
use std::sync::atomic::{AtomicU64, Ordering};
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
    FastPath,      // 独占/不可变对象，公开模式
    ConsensusPath, // 共享对象或强一致需求，公开模式
    PrivatePath,   // 隐私模式
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
    /// 快速通道执行器（Phase 5）
    fast_path: FastPathExecutor,
    /// Optional ZK verifier (feature-gated usage)
    #[cfg(feature = "groth16-verifier")]
    zk: Option<&'a dyn ZkVerifier>,
    /// 路由统计
    fast_path_txns: AtomicU64,
    consensus_path_txns: AtomicU64,
    privacy_path_txns: AtomicU64,
    /// 自适应路由器（可选）
    adaptive: Option<AdaptiveRouter>,
    /// ZK 验证统计：次数、累计耗时(ns)、最近一次耗时(ns)（feature gated）
    #[cfg(feature = "groth16-verifier")]
    zk_verify_count: AtomicU64,
    #[cfg(feature = "groth16-verifier")]
    zk_verify_total_ns: AtomicU64,
    #[cfg(feature = "groth16-verifier")]
    zk_verify_last_ns: AtomicU64,
    /// ZK 验证滑动窗口: 固定容量环形缓冲存最近 N 次耗时(ns)；仅在 feature 启用时使用
    #[cfg(feature = "groth16-verifier")]
    zk_latency_window: parking_lot::Mutex<ZkLatencyWindow>,
}

impl<'a> SuperVM<'a> {
    pub fn new(ownership: &'a OwnershipManager) -> Self {
        Self {
            ownership,
            scheduler: None,
            fast_path: FastPathExecutor::new(),
            #[cfg(feature = "groth16-verifier")]
            zk: None,
            fast_path_txns: AtomicU64::new(0),
            consensus_path_txns: AtomicU64::new(0),
            privacy_path_txns: AtomicU64::new(0),
            adaptive: None,
            #[cfg(feature = "groth16-verifier")]
            zk_verify_count: AtomicU64::new(0),
            #[cfg(feature = "groth16-verifier")]
            zk_verify_total_ns: AtomicU64::new(0),
            #[cfg(feature = "groth16-verifier")]
            zk_verify_last_ns: AtomicU64::new(0),
            #[cfg(feature = "groth16-verifier")]
            zk_latency_window: parking_lot::Mutex::new(ZkLatencyWindow::new(64)),
        }
    }

    pub fn with_scheduler(mut self, scheduler: &'a MvccScheduler) -> Self {
        self.scheduler = Some(scheduler);
        self
    }

    /// 注入可选的 ZK 验证器（最小接入）
    #[cfg(feature = "groth16-verifier")]
    pub fn with_verifier(mut self, verifier: &'a dyn crate::zk_verifier::ZkVerifier) -> Self {
        self.zk = Some(verifier);
        self
    }

    /// 注入自适应路由器
    pub fn with_adaptive_router(mut self, router: AdaptiveRouter) -> Self {
        self.adaptive = Some(router);
        self
    }

    /// 隐私验证（支持真实 ZK 验证器）
    ///
    /// 如果配置了 ZK 验证器且提供了 proof，使用真实验证；
    /// 否则使用占位逻辑（始终返回 true，或模拟延迟）
    ///
    /// # Arguments
    /// * `proof_bytes` - 可选的序列化 proof
    /// * `public_input_bytes` - 可选的公开输入
    pub fn verify_zk_proof(
        &self,
        proof_bytes: Option<&[u8]>,
        public_input_bytes: Option<&[u8]>,
    ) -> bool {
        #[cfg(feature = "groth16-verifier")]
        {
            // 如果提供了验证器和 proof，执行真实验证
            if let (Some(verifier), Some(proof), Some(public_input)) =
                (self.zk, proof_bytes, public_input_bytes)
            {
                let start = std::time::Instant::now();
                let res = match verifier.verify(proof, public_input) {
                    Ok(valid) => valid,
                    Err(_) => false,
                };
                let elapsed = start.elapsed().as_nanos() as u64;
                self.zk_verify_count.fetch_add(1, Ordering::Relaxed);
                self.zk_verify_total_ns.fetch_add(elapsed, Ordering::Relaxed);
                self.zk_verify_last_ns.store(elapsed, Ordering::Relaxed);
                // 记录滑动窗口
                if let Some(mut w) = self.zk_latency_window.try_lock() {
                    w.push(elapsed);
                }
                return res;
            }
        }
        
        // Fallback：占位逻辑（无 ZK 验证器或未提供 proof）
        // 如需模拟延迟，可启用以下代码
        // std::thread::sleep(std::time::Duration::from_millis(0));
        true
    }

    /// 供上层在进入隐私路径前主动调用的验证入口（带明确错误返回）
    #[cfg(feature = "groth16-verifier")]
    pub fn verify_with_error(
        &self,
        proof: &[u8],
        public_inputs: &[u8],
    ) -> Result<bool, crate::zk_verifier::ZkError> {
        if let Some(v) = self.zk {
            v.verify(proof, public_inputs)
        } else {
            // 未配置验证器时，返回错误
            Err(crate::zk_verifier::ZkError::SetupNotInitialized)
        }
    }

    /// 路由到执行路径（不执行）
    pub fn route(&self, tx: &Transaction) -> ExecutionPath {
        let base_path = match tx.privacy {
            Privacy::Private => ExecutionPath::PrivatePath,
            Privacy::Public => {
                if self.ownership.should_use_fast_path(&tx.objects) {
                    ExecutionPath::FastPath
                } else {
                    ExecutionPath::ConsensusPath
                }
            }
        };

        // 如果启用自适应且是公共交易：应用软配额策略
        let path = if matches!(base_path, ExecutionPath::FastPath) {
            if let (Some(ad), Some(scheduler)) = (self.adaptive.as_ref(), self.scheduler) {
                // 更新自适应状态（节流内部控制）
                let stats = scheduler.get_stats();
                ad.maybe_update(&stats);

                let target = ad.target_fast_ratio();
                let routed_stats = self.routing_stats();
                let current_total = routed_stats.total();
                let current_fast_ratio = if current_total > 0 {
                    routed_stats.fast_path_count as f64 / current_total as f64
                } else {
                    0.0
                };
                // 若当前 fast 比例已超过目标，则将该交易降级到共识路径 (软配额)
                if current_fast_ratio > target {
                    ExecutionPath::ConsensusPath
                } else {
                    ExecutionPath::FastPath
                }
            } else {
                base_path
            }
        } else {
            base_path
        };

        // 统计路由
        match path {
            ExecutionPath::FastPath => self.fast_path_txns.fetch_add(1, Ordering::Relaxed),
            ExecutionPath::ConsensusPath => self.consensus_path_txns.fetch_add(1, Ordering::Relaxed),
            ExecutionPath::PrivatePath => self.privacy_path_txns.fetch_add(1, Ordering::Relaxed),
        };

        path
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

        ExecutionReceipt {
            path,
            accepted,
            reason,
            success: false,
            fallback_to_consensus: false,
            return_value: None,
            latency_ms: 0,
        }
    }

    /// 单笔执行（带业务闭包）并按路径执行；Fast 失败时回退到共识重试一次
    pub fn execute_transaction_with<F>(
        &self,
        tx_id: TxId,
        tx: &Transaction,
        f: F,
    ) -> ExecutionReceipt
    where
        F: Fn(&mut Txn) -> anyhow::Result<i32>,
    {
        let start = std::time::Instant::now();
        let path = self.route(tx);
        let scheduler = self
            .scheduler
            .expect("SuperVM: scheduler not configured, call with_scheduler()");

        match path {
            ExecutionPath::FastPath => {
                // Fast 通道：物理分离，使用 FastPathExecutor
                // 由于该接口要求 Fn(&mut Txn)，这里继续使用共识执行器，提示用户改用 execute_transaction_routed
                // 以传入 fast_op（无事务闭包）实现真正快速路径。
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
                // 隐私通道：ZK 证明验证（如未提供 proof，fallback 到占位逻辑）
                if !self.verify_zk_proof(None, None) {
                    return ExecutionReceipt {
                        path,
                        accepted: false,
                        reason: Some("zk proof invalid".into()),
                        success: false,
                        fallback_to_consensus: false,
                        return_value: None,
                        latency_ms: start.elapsed().as_millis() as u64,
                    };
                }

                // 当前阶段：仍按共识执行，后续切换到隐私执行器
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

    /// 新接口：根据路径物理分离执行
    /// - FastPath 使用 FastPathExecutor（无事务闭包）
    /// - Consensus/Private 使用 MvccScheduler（带事务闭包）
    pub fn execute_transaction_routed<Ffast, Fcons>(
        &self,
        tx_id: TxId,
        tx: &Transaction,
        fast_op: Ffast,
        consensus_op: Fcons,
    ) -> ExecutionReceipt
    where
        Ffast: FnOnce() -> Result<i32, String>,
        Fcons: Fn(&mut Txn) -> anyhow::Result<i32>,
    {
        let start = std::time::Instant::now();
        let path = self.route(tx);
        match path {
            ExecutionPath::FastPath => match self.execute_fast_path(tx_id, tx, fast_op) {
                Ok(val) => ExecutionReceipt {
                    path,
                    accepted: true,
                    reason: None,
                    success: true,
                    fallback_to_consensus: false,
                    return_value: Some(val),
                    latency_ms: start.elapsed().as_millis() as u64,
                },
                Err(e) => ExecutionReceipt {
                    path,
                    accepted: false,
                    reason: Some(e),
                    success: false,
                    fallback_to_consensus: false,
                    return_value: None,
                    latency_ms: start.elapsed().as_millis() as u64,
                },
            },
            ExecutionPath::ConsensusPath | ExecutionPath::PrivatePath => {
                // 对于隐私路径，先执行 ZK 验证
                if matches!(path, ExecutionPath::PrivatePath) && !self.verify_zk_proof(None, None) {
                    return ExecutionReceipt {
                        path,
                        accepted: false,
                        reason: Some("zk proof invalid".into()),
                        success: false,
                        fallback_to_consensus: false,
                        return_value: None,
                        latency_ms: start.elapsed().as_millis() as u64,
                    };
                }
                let scheduler = self
                    .scheduler
                    .expect("SuperVM: scheduler not configured, call with_scheduler()");
                let r = scheduler.execute_txn(tx_id, |txn| consensus_op(txn));
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

    /// 批量执行（路径物理分离版本）：
    /// - fast_ops 走 FastPathExecutor
    /// - consensus_ops 走 MvccScheduler
    pub fn execute_batch_split<Ffast, Fcons>(
        &self,
        fast_ops: Vec<(TxId, Transaction, Ffast)>,
        consensus_ops: Vec<(TxId, Transaction, Fcons)>,
    ) -> (Vec<Result<i32, String>>, BatchTxnResult)
    where
        Ffast: FnOnce() -> Result<i32, String> + Send,
        Fcons: Fn(&mut Txn) -> anyhow::Result<i32> + Send + Sync,
    {
        // FastPath 执行（并行）
        let fast_validated: Vec<(TxId, Ffast)> = fast_ops
            .into_iter()
            .filter_map(|(id, tx, f)| {
                // 验证路由是否确为 FastPath
                if matches!(self.route(&tx), ExecutionPath::FastPath) {
                    Some((id, f))
                } else {
                    None
                }
            })
            .collect();
        let fast_res = self.fast_path.execute_batch(fast_validated);

        // Consensus 执行（并行）
        let scheduler = self
            .scheduler
            .expect("SuperVM: scheduler not configured, call with_scheduler()");
        let consensus_items: Vec<(TxId, Fcons)> = consensus_ops
            .into_iter()
            .map(|(id, _tx, f)| (id, f))
            .collect();
        let consensus_res = scheduler.execute_batch(consensus_items);

        (fast_res, consensus_res)
    }

    /// 根据对象所有权为批量交易路由路径并分别执行
    /// 注意：当前最小实现——Fast/Consensus 均使用同一 MvccScheduler 执行，
    /// 区别仅在于路由与统计；后续可将 Fast Path 切换为无共识快速通道。
    pub fn execute_batch_routed<F>(
        &self,
        txs: Vec<(TxId, Transaction, F)>,
    ) -> (BatchTxnResult, BatchTxnResult)
    where
        F: Fn(&mut Txn) -> anyhow::Result<i32> + Send + Sync,
    {
        let scheduler = self
            .scheduler
            .expect("SuperVM: scheduler not configured, call with_scheduler()");

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
            BatchTxnResult {
                successful: 0,
                failed: 0,
                conflicts: 0,
                results: vec![],
            }
        } else {
            scheduler.execute_batch(fast_group)
        };

        let consensus_result = if consensus_group.is_empty() {
            BatchTxnResult {
                successful: 0,
                failed: 0,
                conflicts: 0,
                results: vec![],
            }
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
        let scheduler = self
            .scheduler
            .expect("SuperVM: scheduler not configured, call with_scheduler()");

        // 分桶
        let mut fast_items: Vec<TxnItem> = Vec::new();
        let mut consensus_items: Vec<TxnItem> = Vec::new();
        for (id, tx, f) in txs.into_iter() {
            match self.route(&tx) {
                ExecutionPath::FastPath => fast_items.push((id, f)),
                ExecutionPath::ConsensusPath | ExecutionPath::PrivatePath => {
                    consensus_items.push((id, f))
                }
            }
        }

        // 执行 fast
        let fast_result = if fast_items.is_empty() {
            BatchTxnResult {
                successful: 0,
                failed: 0,
                conflicts: 0,
                results: vec![],
            }
        } else {
            let cloned: Vec<(TxId, _)> = fast_items
                .iter()
                .map(|(id, f)| {
                    (*id, {
                        let f = f.clone();
                        move |txn: &mut Txn| (f)(txn)
                    })
                })
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
            BatchTxnResult {
                successful: 0,
                failed: 0,
                conflicts: 0,
                results: vec![],
            }
        } else {
            let cloned: Vec<(TxId, _)> = consensus_items
                .iter()
                .map(|(id, f)| {
                    (*id, {
                        let f = f.clone();
                        move |txn: &mut Txn| (f)(txn)
                    })
                })
                .collect();
            scheduler.execute_batch(cloned)
        };

        (fast_result, consensus_result, fast_fallbacks)
    }

    /// 标准批量执行入口（带路由与回退）
    pub fn execute_batch(&self, txs: Vec<TxnTuple>) -> (BatchTxnResult, BatchTxnResult, u64) {
        self.execute_batch_routed_with_fallback(txs)
    }

    // ========================================================================
    // Phase 5: 快速通道专用接口
    // ========================================================================

    /// 快速通道执行（仅限独占对象，零冲突）
    ///
    /// # 说明
    /// - 不经过 MVCC 调度器
    /// - 直接执行，无版本控制
    /// - 目标 TPS: 500K+
    ///
    /// # 参数
    /// - `tx_id`: 事务 ID
    /// - `tx`: 事务对象（必须仅包含 Owned 对象）
    /// - `operation`: 业务逻辑闭包
    ///
    /// # 返回
    /// - `Ok(return_value)`: 执行成功
    /// - `Err(msg)`: 执行失败或权限校验失败
    pub fn execute_fast_path<F>(&self, tx_id: TxId, tx: &Transaction, operation: F) -> Result<i32, String>
    where
        F: FnOnce() -> Result<i32, String>,
    {
        // 1. 验证所有对象为 Owned 且属于发送者
        for obj in &tx.objects {
            if let Some(owner_type) = self.ownership.get_ownership_type(obj) {
                match owner_type {
                    crate::OwnershipType::Owned(addr) => {
                        if addr != tx.from {
                            return Err(format!("Object {:?} not owned by sender", obj));
                        }
                    }
                    crate::OwnershipType::Shared => {
                        return Err("Shared object cannot use fast path".into());
                    }
                    crate::OwnershipType::Immutable => {
                        // 不可变对象允许（只读访问）
                    }
                }
            } else {
                return Err(format!("Object {:?} not found", obj));
            }
        }

        // 2. 快速通道执行
        self.fast_path.execute(tx_id, operation)
    }

    /// 批量快速通道执行（并行）
    pub fn execute_fast_path_batch<F>(&self, operations: Vec<(TxId, Transaction, F)>) -> Vec<Result<i32, String>>
    where
        F: FnOnce() -> Result<i32, String> + Send,
    {
        // 验证 + 执行
        let validated_ops: Vec<(TxId, F)> = operations
            .into_iter()
            .filter_map(|(tx_id, tx, op)| {
                // 简单验证：检查第一个对象
                if let Some(first_obj) = tx.objects.first() {
                    if let Some(owner_type) = self.ownership.get_ownership_type(first_obj) {
                        if matches!(owner_type, crate::OwnershipType::Owned(addr) if addr == tx.from) {
                            return Some((tx_id, op));
                        }
                    }
                }
                None
            })
            .collect();

        self.fast_path.execute_batch(validated_ops)
    }

    /// 获取快速通道统计信息
    pub fn fast_path_stats(&self) -> FastPathStats {
        self.fast_path.stats()
    }

    /// 重置快速通道统计
    pub fn reset_fast_path_stats(&self) {
        self.fast_path.reset_stats();
    }

    /// 获取路由统计信息
    pub fn routing_stats(&self) -> RoutingStats {
        RoutingStats {
            fast_path_count: self.fast_path_txns.load(Ordering::Relaxed),
            consensus_path_count: self.consensus_path_txns.load(Ordering::Relaxed),
            privacy_path_count: self.privacy_path_txns.load(Ordering::Relaxed),
        }
    }

    /// 重置路由统计
    pub fn reset_routing_stats(&self) {
        self.fast_path_txns.store(0, Ordering::Relaxed);
        self.consensus_path_txns.store(0, Ordering::Relaxed);
        self.privacy_path_txns.store(0, Ordering::Relaxed);
    }

    /// 导出路由统计为 Prometheus 文本格式
    ///
    /// 指标集合:
    /// - vm_routing_fast_total        (counter)
    /// - vm_routing_consensus_total   (counter)
    /// - vm_routing_privacy_total     (counter)
    /// - vm_routing_fast_ratio        (gauge)
    /// - vm_routing_consensus_ratio   (gauge)
    /// - vm_routing_privacy_ratio     (gauge)
    pub fn export_routing_prometheus(&self) -> String {
        let stats = self.routing_stats();
        let total = stats.total();

        let mut out = String::new();
        out.push_str("# HELP vm_routing_fast_total Total number of transactions routed to fast path\n");
        out.push_str("# TYPE vm_routing_fast_total counter\n");
        out.push_str(&format!("vm_routing_fast_total {}\n", stats.fast_path_count));

        out.push_str("# HELP vm_routing_consensus_total Total number of transactions routed to consensus path\n");
        out.push_str("# TYPE vm_routing_consensus_total counter\n");
        out.push_str(&format!("vm_routing_consensus_total {}\n", stats.consensus_path_count));

        out.push_str("# HELP vm_routing_privacy_total Total number of transactions routed to privacy path\n");
        out.push_str("# TYPE vm_routing_privacy_total counter\n");
        out.push_str(&format!("vm_routing_privacy_total {}\n", stats.privacy_path_count));

        out.push_str("# HELP vm_routing_fast_ratio Ratio of fast path routed transactions\n");
        out.push_str("# TYPE vm_routing_fast_ratio gauge\n");
        out.push_str(&format!("vm_routing_fast_ratio {:.6}\n", stats.fast_path_ratio()));

        out.push_str("# HELP vm_routing_consensus_ratio Ratio of consensus path routed transactions\n");
        out.push_str("# TYPE vm_routing_consensus_ratio gauge\n");
        out.push_str(&format!("vm_routing_consensus_ratio {:.6}\n", stats.consensus_path_ratio()));

        out.push_str("# HELP vm_routing_privacy_ratio Ratio of privacy path routed transactions\n");
        out.push_str("# TYPE vm_routing_privacy_ratio gauge\n");
        out.push_str(&format!("vm_routing_privacy_ratio {:.6}\n", stats.privacy_path_ratio()));

        out.push_str("# HELP vm_routing_total Total number of routed transactions (all paths)\n");
        out.push_str("# TYPE vm_routing_total counter\n");
        out.push_str(&format!("vm_routing_total {}\n", total));

        // 自适应扩展指标
        if let Some(ad) = &self.adaptive {
            out.push_str(&ad.export_prometheus());
        }

        // ZK 验证指标（仅在启用 feature 且发生过验证时输出）
        #[cfg(feature = "groth16-verifier")]
        {
            let count = self.zk_verify_count.load(Ordering::Relaxed);
            if count > 0 {
                let total_ns = self.zk_verify_total_ns.load(Ordering::Relaxed);
                let avg_ms = (total_ns as f64) / (count as f64) / 1_000_000.0;
                let last_ms = self.zk_verify_last_ns.load(Ordering::Relaxed) as f64 / 1_000_000.0;
                let (p50, p95, window_len) = {
                    let w = self.zk_latency_window.lock();
                    let (p50, p95) = w.percentiles();
                    (p50 / 1_000_000.0, p95 / 1_000_000.0, w.len() as u64)
                };
                out.push_str("# HELP vm_privacy_zk_verify_count_total Total number of real ZK proof verifications\n");
                out.push_str("# TYPE vm_privacy_zk_verify_count_total counter\n");
                out.push_str(&format!("vm_privacy_zk_verify_count_total {}\n", count));
                out.push_str("# HELP vm_privacy_zk_verify_avg_latency_ms Average ZK verification latency (ms)\n");
                out.push_str("# TYPE vm_privacy_zk_verify_avg_latency_ms gauge\n");
                out.push_str(&format!("vm_privacy_zk_verify_avg_latency_ms {:.6}\n", avg_ms));
                out.push_str("# HELP vm_privacy_zk_verify_last_latency_ms Last ZK verification latency (ms)\n");
                out.push_str("# TYPE vm_privacy_zk_verify_last_latency_ms gauge\n");
                out.push_str(&format!("vm_privacy_zk_verify_last_latency_ms {:.6}\n", last_ms));
                out.push_str("# HELP vm_privacy_zk_verify_p50_latency_ms p50 (median) ZK verification latency over recent window\n");
                out.push_str("# TYPE vm_privacy_zk_verify_p50_latency_ms gauge\n");
                out.push_str(&format!("vm_privacy_zk_verify_p50_latency_ms {:.6}\n", p50));
                out.push_str("# HELP vm_privacy_zk_verify_p95_latency_ms p95 ZK verification latency over recent window\n");
                out.push_str("# TYPE vm_privacy_zk_verify_p95_latency_ms gauge\n");
                out.push_str(&format!("vm_privacy_zk_verify_p95_latency_ms {:.6}\n", p95));
                out.push_str("# HELP vm_privacy_zk_verify_window_size Number of samples in ZK latency sliding window\n");
                out.push_str("# TYPE vm_privacy_zk_verify_window_size gauge\n");
                out.push_str(&format!("vm_privacy_zk_verify_window_size {}\n", window_len));
            }
        }

        out
    }
}

// ================= ZK 验证滑动窗口实现（feature gated） ==================
#[cfg(feature = "groth16-verifier")]
struct ZkLatencyWindow {
    buf: Vec<u64>,      // 存储 ns
    cap: usize,
    pos: usize,
    filled: bool,
}

#[cfg(feature = "groth16-verifier")]
impl ZkLatencyWindow {
    fn new(cap: usize) -> Self {
        Self { buf: vec![0; cap], cap, pos: 0, filled: false }
    }
    fn push(&mut self, v: u64) {
        self.buf[self.pos] = v;
        self.pos = (self.pos + 1) % self.cap;
        if self.pos == 0 { self.filled = true; }
    }
    fn len(&self) -> usize { if self.filled { self.cap } else { self.pos } }
    fn iter_values(&self) -> Vec<u64> {
        let mut out = Vec::with_capacity(self.len());
        if self.filled {
            out.extend_from_slice(&self.buf[self.pos..]);
            out.extend_from_slice(&self.buf[..self.pos]);
        } else {
            out.extend_from_slice(&self.buf[..self.pos]);
        }
        out
    }
    fn percentiles(&self) -> (f64, f64) {
        let mut vals = self.iter_values();
        if vals.is_empty() { return (0.0, 0.0); }
        vals.sort_unstable();
        let idx50 = (vals.len() as f64 * 0.50).floor().clamp(0.0, (vals.len()-1) as f64) as usize;
        let idx95 = (vals.len() as f64 * 0.95).floor().clamp(0.0, (vals.len()-1) as f64) as usize;
        (vals[idx50] as f64, vals[idx95] as f64)
    }
}

/// 路由统计信息
#[derive(Debug, Clone, Copy)]
pub struct RoutingStats {
    pub fast_path_count: u64,
    pub consensus_path_count: u64,
    pub privacy_path_count: u64,
}

impl RoutingStats {
    /// 总事务数
    pub fn total(&self) -> u64 {
        self.fast_path_count + self.consensus_path_count + self.privacy_path_count
    }

    /// 快速通道比例
    pub fn fast_path_ratio(&self) -> f64 {
        let total = self.total();
        if total > 0 {
            self.fast_path_count as f64 / total as f64
        } else {
            0.0
        }
    }

    /// 共识通道比例
    pub fn consensus_path_ratio(&self) -> f64 {
        let total = self.total();
        if total > 0 {
            self.consensus_path_count as f64 / total as f64
        } else {
            0.0
        }
    }

    /// 隐私通道比例
    pub fn privacy_path_ratio(&self) -> f64 {
        let total = self.total();
        if total > 0 {
            self.privacy_path_count as f64 / total as f64
        } else {
            0.0
        }
    }
}
