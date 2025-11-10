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

    // ================= Batch ZK Verification (SuperVM-level buffering) =================
    // 通过在隐私路径收集 proof 与 public inputs，按批次触发验证以降低整体均值开销。
    // 原型阶段：仍使用单个 ZkVerifier 逐个验证（暂未做真正聚合算法），但一次 Flush 批量处理并输出批量指标。
    #[cfg(feature = "groth16-verifier")]
    batch_enabled: bool,
    #[cfg(feature = "groth16-verifier")]
    batch_size: usize,
    #[cfg(feature = "groth16-verifier")]
    batch_flush_interval_ms: u64,
    #[cfg(feature = "groth16-verifier")]
    batch_buffer: parking_lot::Mutex<Vec<(Vec<u8>, Vec<u8>)>>, // (proof_bytes, public_input_bytes)
    #[cfg(feature = "groth16-verifier")]
    batch_last_flush: std::sync::Mutex<std::time::Instant>,

    /// Fast→Consensus 回退开关
    fallback_enabled: bool,
    /// 允许回退的错误关键词白名单（简单包含匹配）
    fallback_error_whitelist: Vec<&'static str>,
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
            #[cfg(feature = "groth16-verifier")]
            batch_enabled: false,
            #[cfg(feature = "groth16-verifier")]
            batch_size: 32,
            #[cfg(feature = "groth16-verifier")]
            batch_flush_interval_ms: 50, // 默认 50ms 刷新窗口
            #[cfg(feature = "groth16-verifier")]
            batch_buffer: parking_lot::Mutex::new(Vec::with_capacity(64)),
            #[cfg(feature = "groth16-verifier")]
            batch_last_flush: std::sync::Mutex::new(std::time::Instant::now()),
            fallback_enabled: false,
            fallback_error_whitelist: Vec::new(),
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

    /// 启用 Fast→Consensus 回退
    pub fn with_fallback(mut self, enabled: bool) -> Self {
        self.fallback_enabled = enabled;
        self
    }

    /// 设置回退错误白名单（字符串匹配）
    pub fn with_fallback_whitelist(mut self, errs: Vec<&'static str>) -> Self {
        self.fallback_error_whitelist = errs;
        self
    }

    /// 从环境变量注入回退配置
    /// SUPERVM_ENABLE_FAST_FALLBACK=true|1
    /// SUPERVM_FALLBACK_ON_ERRORS=Conflict,LockBusy,NotOwned
    pub fn from_env(mut self) -> Self {
        if let Ok(v) = std::env::var("SUPERVM_ENABLE_FAST_FALLBACK") {
            if v.eq_ignore_ascii_case("true") || v == "1" { self.fallback_enabled = true; }
        }
        if let Ok(v) = std::env::var("SUPERVM_FALLBACK_ON_ERRORS") {
            if !v.trim().is_empty() {
                let list: Vec<&'static str> = v
                    .split(',')
                    .map(|s| s.trim())
                    .filter(|s| !s.is_empty())
                    .map(|s| {
                        // Box::leak 返回 &'static mut str，这里协变为不可变引用
                        let leaked: &'static mut str = Box::leak(s.to_string().into_boxed_str());
                        let imm: &'static str = leaked;
                        imm
                    })
                    .collect();
                self.fallback_error_whitelist = list;
            }
        }
        // 批量 ZK 验证环境变量 (feature gated)
        #[cfg(feature = "groth16-verifier")]
        {
            if let Ok(v) = std::env::var("ZK_BATCH_ENABLE") {
                if v.eq_ignore_ascii_case("true") || v == "1" { self.batch_enabled = true; }
            }
            if let Ok(v) = std::env::var("ZK_BATCH_SIZE") {
                if let Ok(sz) = v.parse::<usize>() { if sz > 0 { self.batch_size = sz; } }
            }
            if let Ok(v) = std::env::var("ZK_BATCH_FLUSH_INTERVAL_MS") {
                if let Ok(iv) = v.parse::<u64>() { if iv > 0 { self.batch_flush_interval_ms = iv; } }
            }
        }
        self
    }

    fn fallback_allowed(&self, err: &str) -> bool {
        if !self.fallback_enabled { return false; }
        if self.fallback_error_whitelist.is_empty() { return true; }
        self.fallback_error_whitelist.iter().any(|e| err.contains(e))
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
            // Batch 逻辑：若启用批量并提供 proof/public_input，则进入缓冲
            if let (Some(verifier), Some(proof), Some(public_input)) = (self.zk, proof_bytes, public_input_bytes) {
                if self.batch_enabled {
                    // Push into buffer
                    {
                        let mut buf = self.batch_buffer.lock();
                        buf.push((proof.to_vec(), public_input.to_vec()));
                    }
                    // 检查是否需要 Flush
                    let should_flush = {
                        let buf_len = { self.batch_buffer.lock().len() };
                        if buf_len >= self.batch_size { true } else {
                            let last_guard = self.batch_last_flush.lock().unwrap();
                            last_guard.elapsed().as_millis() as u64 >= self.batch_flush_interval_ms && buf_len > 0
                        }
                    };
                    if should_flush {
                        let batch_items = {
                            let mut buf = self.batch_buffer.lock();
                            let items = std::mem::take(&mut *buf); // drain
                            items
                        };
                        // 更新 flush 时间
                        {
                            let mut last = self.batch_last_flush.lock().unwrap();
                            *last = std::time::Instant::now();
                        }
                        // 执行批量验证（逐个调用 verifier.verify）
                        let start_batch = std::time::Instant::now();
                        let mut results: Vec<bool> = Vec::with_capacity(batch_items.len());
                        for (p_bytes, pi_bytes) in &batch_items {
                            let r = verifier.verify(p_bytes, pi_bytes).unwrap_or(false);
                            // 记录单次验证指标（保持与非批量一致）
                            let single_elapsed = start_batch.elapsed(); // 这里不精准；保持简单不重复测每个单次耗时
                            let single_ns = single_elapsed.as_nanos() as u64;
                            self.zk_verify_count.fetch_add(1, Ordering::Relaxed);
                            self.zk_verify_total_ns.fetch_add(single_ns, Ordering::Relaxed);
                            self.zk_verify_last_ns.store(single_ns, Ordering::Relaxed);
                            if let Some(mut w) = self.zk_latency_window.try_lock() { w.push(single_ns); }
                            results.push(r);
                        }
                        let batch_elapsed = start_batch.elapsed();
                        // 计算统计并写入 MetricsCollector（通过 scheduler -> store -> metrics）
                        if let Some(scheduler) = self.scheduler {
                            if let Some(mc) = scheduler.store().get_metrics() {
                                let total = results.len() as u64;
                                let failed = results.iter().filter(|&&ok| !ok).count() as u64;
                                let batch_ms = batch_elapsed.as_secs_f64() * 1000.0;
                                let avg_latency_ms = if total > 0 { batch_ms / total as f64 } else { 0.0 };
                                let tps = if batch_elapsed.as_secs_f64() > 0.0 { (total - failed) as f64 / batch_elapsed.as_secs_f64() } else { 0.0 };
                                mc.record_zk_batch_verify(total, failed, batch_ms, avg_latency_ms, tps);
                            }
                        }
                        // 当前调用的结果是最后一个加入的 proof
                        if let Some(last) = results.last() { return *last; } else { return false; }
                    } else {
                        // 未到 flush 条件，执行单次验证以避免延迟接受（保持语义安全）
                        let start = std::time::Instant::now();
                        let res = verifier.verify(proof, public_input).unwrap_or(false);
                        let elapsed = start.elapsed().as_nanos() as u64;
                        self.zk_verify_count.fetch_add(1, Ordering::Relaxed);
                        self.zk_verify_total_ns.fetch_add(elapsed, Ordering::Relaxed);
                        self.zk_verify_last_ns.store(elapsed, Ordering::Relaxed);
                        if let Some(mut w) = self.zk_latency_window.try_lock() { w.push(elapsed); }
                        // 将当前项从缓冲移除，避免后续批量重复验证
                        {
                            let mut buf = self.batch_buffer.lock();
                            let _ = buf.pop();
                        }
                        return res;
                    }
                } else {
                    // 未启用批量，走原始单次验证路径
                    let start = std::time::Instant::now();
                    let res = verifier.verify(proof, public_input).unwrap_or(false);
                    let elapsed = start.elapsed().as_nanos() as u64;
                    self.zk_verify_count.fetch_add(1, Ordering::Relaxed);
                    self.zk_verify_total_ns.fetch_add(elapsed, Ordering::Relaxed);
                    self.zk_verify_last_ns.store(elapsed, Ordering::Relaxed);
                    if let Some(mut w) = self.zk_latency_window.try_lock() { w.push(elapsed); }
                    return res;
                }
            }
        }
        
        // Fallback：占位逻辑（无 ZK 验证器或未提供 proof）
        // 如需模拟延迟，可启用以下代码
        // std::thread::sleep(std::time::Duration::from_millis(0));
        true
    }

    /// 手动触发批量 ZK 验证 Flush（用于测试或定时器外部触发）
    /// 返回 (total, failed)
    #[cfg(feature = "groth16-verifier")]
    pub fn flush_zk_batch(&self) -> (u64, u64) {
        if !self.batch_enabled { return (0, 0); }
        let verifier = match self.zk { Some(v) => v, None => return (0, 0) };
        let batch_items = {
            let mut buf = self.batch_buffer.lock();
            if buf.is_empty() { return (0, 0); }
            std::mem::take(&mut *buf)
        };
        {
            let mut last = self.batch_last_flush.lock().unwrap();
            *last = std::time::Instant::now();
        }
        let start_batch = std::time::Instant::now();
        let mut success = 0u64;
        let mut total_ns_accum: u64 = 0;
        for (p, pi) in &batch_items {
            let s = std::time::Instant::now();
            let ok = verifier.verify(p, pi).unwrap_or(false);
            let ns = s.elapsed().as_nanos() as u64;
            total_ns_accum = total_ns_accum.saturating_add(ns);
            self.zk_verify_count.fetch_add(1, Ordering::Relaxed);
            self.zk_verify_total_ns.fetch_add(ns, Ordering::Relaxed);
            self.zk_verify_last_ns.store(ns, Ordering::Relaxed);
            if let Some(mut w) = self.zk_latency_window.try_lock() { w.push(ns); }
            if ok { success += 1; }
        }
        let batch_elapsed = start_batch.elapsed();
        if let Some(scheduler) = self.scheduler {
            if let Some(mc) = scheduler.store().get_metrics() {
                let total = batch_items.len() as u64;
                let failed = total.saturating_sub(success);
                let batch_ms = batch_elapsed.as_secs_f64() * 1000.0;
                let avg_latency_ms = if total > 0 { batch_ms / total as f64 } else { 0.0 };
                let tps = if batch_elapsed.as_secs_f64() > 0.0 { success as f64 / batch_elapsed.as_secs_f64() } else { 0.0 };
                mc.record_zk_batch_verify(total, failed, batch_ms, avg_latency_ms, tps);
                return (total, failed);
            }
        }
        (batch_items.len() as u64, batch_items.len() as u64 - success)
    }

    /// 启动后台定时 flush 线程（需外部保证生命周期安全）
    /// 
    /// # Safety
    /// 调用方必须确保 SuperVM 实例在后台线程运行期间保持有效（通常用 Arc/Box::leak 或进程级 static）。
    /// interval_ms: 轮询间隔毫秒；仅在 batch_enabled 时实际执行 flush
    #[cfg(feature = "groth16-verifier")]
    pub fn start_batch_flush_loop(&self, interval_ms: u64) {
        if interval_ms == 0 { return; }
        if !self.batch_enabled { return; }
        // 复制必要配置到线程本地
        let batch_size = self.batch_size;
        let flush_interval_ms = self.batch_flush_interval_ms;
        // 使用原始指针跨线程，调用方必须保证 SuperVM 实例生命周期覆盖后台线程
        let self_ptr: usize = self as *const _ as usize; // 转为 usize 规避 Send 检查
        std::thread::spawn(move || {
            let sleep_dur = std::time::Duration::from_millis(interval_ms);
            loop {
                std::thread::sleep(sleep_dur);
                unsafe {
                    let vm = &*(self_ptr as *const SuperVM);
                    if !vm.batch_enabled { continue; }
                    // 检查是否需要 flush（尺寸或时间条件）
                    let len = vm.batch_buffer.lock().len();
                    if len == 0 { continue; }
                    let last_guard = vm.batch_last_flush.lock().unwrap();
                    let elapsed_ms = last_guard.elapsed().as_millis() as u64;
                    drop(last_guard);
                    if len >= batch_size || elapsed_ms >= flush_interval_ms {
                        let _ = vm.flush_zk_batch();
                    }
                }
            }
        });
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
                Err(e) => {
                    // FastPath 执行失败，考虑回退
                    if self.fallback_allowed(&e) {
                        // 记录回退指标（需要上层将 MetricsCollector 注入并暴露接口，这里假设有全局或单例访问）
                        if let Some(scheduler) = self.scheduler {
                            // 通过 MVCC Store 暴露的 get_metrics() 访问指标收集器
                            if let Some(store_metrics) = scheduler.store().get_metrics() {
                                store_metrics.inc_fast_fallback();
                            }
                        }
                        // 回退到共识路径执行
                        let scheduler = self
                            .scheduler
                            .expect("SuperVM: scheduler not configured, call with_scheduler()");
                        let r = scheduler.execute_txn(tx_id, |txn| consensus_op(txn));
                        ExecutionReceipt {
                            path: ExecutionPath::ConsensusPath,
                            accepted: true,
                            reason: r.error.clone(),
                            success: r.success,
                            fallback_to_consensus: true,
                            return_value: r.return_value,
                            latency_ms: start.elapsed().as_millis() as u64,
                        }
                    } else {
                        ExecutionReceipt {
                            path,
                            accepted: false,
                            reason: Some(e),
                            success: false,
                            fallback_to_consensus: false,
                            return_value: None,
                            latency_ms: start.elapsed().as_millis() as u64,
                        }
                    }
                }
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
    ) -> (BatchTxnResult, BatchTxnResult, u64, Vec<TxId>) {
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
    let mut fallback_ids: Vec<TxId> = Vec::new();
        if !fast_items.is_empty() {
            let failed_ids: std::collections::HashSet<TxId> = fast_result
                .results
                .iter()
                .filter(|r| !r.success)
                .map(|r| r.tx_id)
                .collect();
            if !failed_ids.is_empty() {
                fast_fallbacks = failed_ids.len() as u64;
                fallback_ids = failed_ids.iter().copied().collect();
                // 增加回退计数到 MetricsCollector
                if let Some(mc) = scheduler.store().get_metrics() {
                    mc.inc_fast_fallback_by(fast_fallbacks);
                }
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

    (fast_result, consensus_result, fast_fallbacks, fallback_ids)
    }

    /// 标准批量执行入口（带路由与回退）
    pub fn execute_batch(&self, txs: Vec<TxnTuple>) -> (BatchTxnResult, BatchTxnResult, u64, Vec<TxId>) {
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

        // 新增：合并回退指标（如有调度器和指标收集器）
        if let Some(scheduler) = self.scheduler {
            if let Some(mc) = scheduler.store().get_metrics() {
                out.push_str("# HELP vm_fast_fallback_total Total number of fast path fallbacks to consensus\n");
                out.push_str("# TYPE vm_fast_fallback_total counter\n");
                out.push_str(&format!("vm_fast_fallback_total {}\n", mc.get_fast_fallback_total()));
                out.push_str("# HELP vm_fast_fallback_ratio Ratio of fast fallbacks over total committed transactions\n");
                out.push_str("# TYPE vm_fast_fallback_ratio gauge\n");
                out.push_str(&format!("vm_fast_fallback_ratio {:.6}\n", mc.fast_fallback_ratio()));
            }
        }

        // 新增：fast 路由尝试回退比率（基于 fast 路由总数）
        if let Some(scheduler) = self.scheduler {
            if let Some(mc) = scheduler.store().get_metrics() {
                let fast_total = stats.fast_path_count;
                let fallback_total = mc.get_fast_fallback_total();
                let ratio = if fast_total > 0 { fallback_total as f64 / fast_total as f64 } else { 0.0 };
                out.push_str("# HELP vm_routing_fast_fallback_ratio Ratio of fast fallbacks over fast path attempts\n");
                out.push_str("# TYPE vm_routing_fast_fallback_ratio gauge\n");
                out.push_str(&format!("vm_routing_fast_fallback_ratio {:.6}\n", ratio));
            }
        }

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

// ===========================================================
// 内部路由与执行冒烟测试（迁移自 tests/ 以规避外部 0 tests 问题）
// ===========================================================
#[cfg(test)]
mod routing_smoke_tests {
    use super::*;
    use crate::{OwnershipManager, OwnershipType, ObjectMetadata};
    use crate::parallel_mvcc::MvccScheduler;

    type Address = [u8;32];
    type ObjectId = [u8;32];
    fn addr(id: u8) -> Address { let mut a=[0u8;32]; a[0]=id; a }
    fn obj(id: u8) -> ObjectId { let mut o=[0u8;32]; o[0]=id; o }

    fn register_owned(om: &OwnershipManager, id: ObjectId, owner: Address) {
        let meta = ObjectMetadata { id, version:0, ownership: OwnershipType::Owned(owner), object_type: "Test".into(), created_at:0, updated_at:0, size:0, is_deleted:false};
        om.register_object(meta).unwrap();
    }
    fn register_shared(om: &OwnershipManager, id: ObjectId) {
        let meta = ObjectMetadata { id, version:0, ownership: OwnershipType::Shared, object_type: "Test".into(), created_at:0, updated_at:0, size:0, is_deleted:false};
        om.register_object(meta).unwrap();
    }

    fn tx(from: Address, objects: Vec<ObjectId>, privacy: Privacy) -> Transaction { Transaction { from, objects, privacy } }

    #[test]
    fn route_owned_public_goes_fast_internal() {
        let ownership = OwnershipManager::new();
        register_owned(&ownership, obj(1), addr(1));
    let sched = MvccScheduler::new();
    let vm = SuperVM::new(&ownership).with_scheduler(&sched);
        let t = tx(addr(1), vec![obj(1)], Privacy::Public);
        assert!(matches!(vm.route(&t), ExecutionPath::FastPath));
    }

    #[test]
    fn route_shared_public_goes_consensus_internal() {
        let ownership = OwnershipManager::new();
        register_shared(&ownership, obj(2));
    let sched = MvccScheduler::new();
    let vm = SuperVM::new(&ownership).with_scheduler(&sched);
        let t = tx(addr(1), vec![obj(2)], Privacy::Public);
        assert!(matches!(vm.route(&t), ExecutionPath::ConsensusPath));
    }

    #[test]
    fn route_private_goes_privacy_internal() {
        let ownership = OwnershipManager::new();
        register_owned(&ownership, obj(3), addr(1));
    let sched = MvccScheduler::new();
    let vm = SuperVM::new(&ownership).with_scheduler(&sched);
        let t = tx(addr(1), vec![obj(3)], Privacy::Private);
        assert!(matches!(vm.route(&t), ExecutionPath::PrivatePath));
    }

    #[test]
    fn execute_fast_success_and_prom_metrics_present_internal() {
        let ownership = OwnershipManager::new();
        register_owned(&ownership, obj(4), addr(1));
    let sched = MvccScheduler::new();
    let vm = SuperVM::new(&ownership).with_scheduler(&sched);
        let t = tx(addr(1), vec![obj(4)], Privacy::Public);
        let receipt = vm.execute_transaction_routed(10, &t, || Ok(42), |_txn| Ok(7));
        assert!(matches!(receipt.path, ExecutionPath::FastPath));
        assert!(receipt.success);
        assert_eq!(receipt.return_value, Some(42));
        let prom = vm.export_routing_prometheus();
        assert!(prom.contains("vm_routing_fast_total"));
    }

    #[test]
    fn fast_fail_fallback_increments_metric_internal() {
        let ownership = OwnershipManager::new();
        register_owned(&ownership, obj(5), addr(1)); // owner is 1
        let scheduler = MvccScheduler::new();
        let vm = SuperVM::new(&ownership)
            .with_scheduler(&scheduler)
            .with_fallback(true)
            .with_fallback_whitelist(vec!["Object", "not owned"]);
        // sender addr(2) different → fast path ownership校验失败 → fallback 执行成功
        let t = tx(addr(2), vec![obj(5)], Privacy::Public);
        let receipt = vm.execute_transaction_routed(11, &t, || Ok(1), |txn| { txn.write(b"k".to_vec(), b"v".to_vec()); Ok(9) });
        assert!(matches!(receipt.path, ExecutionPath::ConsensusPath));
        assert!(receipt.fallback_to_consensus);
        let metrics_text = scheduler.store().get_metrics().unwrap().export_prometheus();
        assert!(metrics_text.contains("vm_fast_fallback_total 1"));
    }

    #[test]
    fn routing_counts_and_ratios_accumulate_internal() {
        let ownership = OwnershipManager::new();
        register_owned(&ownership, obj(6), addr(1)); // fast
        register_shared(&ownership, obj(7));        // consensus
        register_owned(&ownership, obj(8), addr(2)); // fast with different owner for second address
        register_owned(&ownership, obj(9), addr(3)); // fast
        register_owned(&ownership, obj(10), addr(4)); // fast
    let sched = MvccScheduler::new();
    let vm = SuperVM::new(&ownership).with_scheduler(&sched);
        // Execute mixed routes: 4 fast (6,8,9,10 as their senders), 1 consensus (7), 1 privacy (6 again but private)
        let _ = vm.route(&tx(addr(1), vec![obj(6)], Privacy::Public));
        let _ = vm.route(&tx(addr(2), vec![obj(8)], Privacy::Public));
        let _ = vm.route(&tx(addr(3), vec![obj(9)], Privacy::Public));
        let _ = vm.route(&tx(addr(4), vec![obj(10)], Privacy::Public));
        let _ = vm.route(&tx(addr(1), vec![obj(7)], Privacy::Public)); // shared → consensus
        let _ = vm.route(&tx(addr(1), vec![obj(6)], Privacy::Private)); // privacy
        let stats = vm.routing_stats();
        assert_eq!(stats.fast_path_count, 4);
        assert_eq!(stats.consensus_path_count, 1);
        assert_eq!(stats.privacy_path_count, 1);
        let prom = vm.export_routing_prometheus();
        assert!(prom.contains("vm_routing_fast_total 4"));
        assert!(prom.contains("vm_routing_consensus_total 1"));
        assert!(prom.contains("vm_routing_privacy_total 1"));
    }
}

// ===========================================================
// E2E 路由与回退批量执行测试
// ===========================================================
#[cfg(test)]
mod routing_e2e_tests {
    use super::*;
    use crate::{OwnershipManager, OwnershipType, ObjectMetadata};
    use crate::parallel_mvcc::MvccScheduler;
    use crate::mvcc::Txn;

    type Address = [u8;32];
    type ObjectId = [u8;32];
    fn addr(id: u8) -> Address { let mut a=[0u8;32]; a[0]=id; a }
    fn obj(id: u8) -> ObjectId { let mut o=[0u8;32]; o[0]=id; o }

    fn reg(om:&OwnershipManager, id:ObjectId, own:OwnershipType) {
        let meta = ObjectMetadata { id, version:0, ownership: own, object_type:"Test".into(), created_at:0, updated_at:0, size:0, is_deleted:false};
        om.register_object(meta).unwrap();
    }

    fn consensus_write(txn:&mut Txn, k:&[u8], v:&[u8]) -> anyhow::Result<i32> { txn.write(k.to_vec(), v.to_vec()); Ok(v.len() as i32) }

    #[test]
    fn e2e_mixed_individual_routed_execution() {
        // Setup ownership landscape
        let ownership = OwnershipManager::new();
        // Fast path objects (owned by different senders)
        reg(&ownership, obj(10), OwnershipType::Owned(addr(1)));
        reg(&ownership, obj(11), OwnershipType::Owned(addr(2)));
        // Shared object for consensus
        reg(&ownership, obj(12), OwnershipType::Shared);
        // Privacy object (owned)
        reg(&ownership, obj(13), OwnershipType::Owned(addr(3)));

        let scheduler = MvccScheduler::new();
        let vm = SuperVM::new(&ownership)
            .with_scheduler(&scheduler)
            .with_fallback(true)
            .with_fallback_whitelist(vec!["Object", "not owned"]);

        // 1) Fast success
        let tx_fast_ok = Transaction { from: addr(1), objects: vec![obj(10)], privacy: Privacy::Public };
        let r_fast_ok = vm.execute_transaction_routed(100, &tx_fast_ok, || Ok(99), |txn| consensus_write(txn, b"f1", b"v1"));
        assert!(r_fast_ok.success && matches!(r_fast_ok.path, ExecutionPath::FastPath));
        assert_eq!(r_fast_ok.return_value, Some(99));

        // 2) Fast failure → fallback (wrong sender for owned object)
        let tx_fast_fail = Transaction { from: addr(9), objects: vec![obj(11)], privacy: Privacy::Public }; // owner is addr(2)
        let r_fast_fail = vm.execute_transaction_routed(101, &tx_fast_fail, || Ok(1), |txn| consensus_write(txn, b"ff", b"vf"));
        assert!(r_fast_fail.success && r_fast_fail.fallback_to_consensus);
        assert!(matches!(r_fast_fail.path, ExecutionPath::ConsensusPath));

        // 3) Consensus direct (shared object)
        let tx_cons = Transaction { from: addr(1), objects: vec![obj(12)], privacy: Privacy::Public };
        let r_cons = vm.execute_transaction_routed(102, &tx_cons, || Ok(1), |txn| consensus_write(txn, b"c1", b"cval"));
        assert!(r_cons.success && matches!(r_cons.path, ExecutionPath::ConsensusPath));

        // 4) Privacy (placeholder verify true) -> PrivatePath
        let tx_priv = Transaction { from: addr(3), objects: vec![obj(13)], privacy: Privacy::Private };
        let r_priv = vm.execute_transaction_routed(103, &tx_priv, || Ok(7), |txn| consensus_write(txn, b"p1", b"pval"));
        assert!(r_priv.success && matches!(r_priv.path, ExecutionPath::PrivatePath));

        // Metrics assertions: fallback counter should be 1
        let prom = scheduler.store().get_metrics().unwrap().export_prometheus();
        assert!(prom.contains("vm_fast_fallback_total 1"));

        // Routing stats: we expect fast routed (1 fast success + 1 fast attempt that failed but counted as fast route), plus one consensus route (shared), plus one privacy route.
        let stats = vm.routing_stats();
        assert_eq!(stats.fast_path_count, 2); // route() increments before execution
        assert_eq!(stats.consensus_path_count, 1); // shared
        assert_eq!(stats.privacy_path_count, 1); // privacy
        let routing_prom = vm.export_routing_prometheus();
        assert!(routing_prom.contains("vm_routing_fast_total 2"));
        assert!(routing_prom.contains("vm_routing_consensus_total 1"));
        assert!(routing_prom.contains("vm_routing_privacy_total 1"));
    }

    #[test]
    fn e2e_batch_routed_with_fallback_counts() {
        // Ownership setup for batch
        let ownership = OwnershipManager::new();
        reg(&ownership, obj(20), OwnershipType::Owned(addr(1))); // fast success
        reg(&ownership, obj(21), OwnershipType::Owned(addr(2))); // fast fail (sender mismatch) -> fallback
        reg(&ownership, obj(22), OwnershipType::Shared);        // consensus
        reg(&ownership, obj(23), OwnershipType::Owned(addr(3))); // privacy
        let scheduler = MvccScheduler::new();
        let vm = SuperVM::new(&ownership)
            .with_scheduler(&scheduler)
            .with_fallback(true)
            .with_fallback_whitelist(vec!["Object", "not owned"]);

        // Build tuple batch: (TxId, Transaction, Arc<dyn Fn(&mut Txn)>)
    use std::sync::Arc;
    let mk_ok = |k: &'static [u8], v: &'static [u8]| Arc::new(move |txn: &mut Txn| { txn.write(k.to_vec(), v.to_vec()); Ok(v.len() as i32) }) as Arc<dyn Fn(&mut Txn) -> anyhow::Result<i32> + Send + Sync>;
    let mk_err = || Arc::new(move |_txn: &mut Txn| { Err(anyhow::anyhow!("retryable fast failure")) }) as Arc<dyn Fn(&mut Txn) -> anyhow::Result<i32> + Send + Sync>;

        let batch = vec![
            (200u64, Transaction { from: addr(1), objects: vec![obj(20)], privacy: Privacy::Public }, mk_ok(b"b1", b"v")),
            (201u64, Transaction { from: addr(9), objects: vec![obj(21)], privacy: Privacy::Public }, mk_err()), // force fast failure -> fallback
            (202u64, Transaction { from: addr(1), objects: vec![obj(22)], privacy: Privacy::Public }, mk_ok(b"b3", b"vvv")), // shared
            (203u64, Transaction { from: addr(3), objects: vec![obj(23)], privacy: Privacy::Private }, mk_ok(b"b4", b"vvvv")), // privacy
        ];

    let (_fast_res, consensus_res, fast_fallbacks, fallback_ids) = vm.execute_batch(batch);

    // Successful counts: fast_res should contain attempts (包含失败)，失败的被回退到 consensus_res。
    assert_eq!(fast_fallbacks, 1, "One fast failure should have been rerouted");
    assert_eq!(fallback_ids.len(), 1, "Fallback IDs should match fallback count");
        // 现在批量接口也会增加 MetricsCollector 的 fallback 计数
        let prom = scheduler.store().get_metrics().unwrap().export_prometheus();
        assert!(prom.contains("vm_fast_fallback_total 1"), "Batch fallback should increment metrics collector");

        // Routing totals (4 routed: 2 fast attempts, 1 consensus, 1 privacy)
        let stats = vm.routing_stats();
        assert_eq!(stats.fast_path_count, 2);
        assert_eq!(stats.consensus_path_count, 1);
        assert_eq!(stats.privacy_path_count, 1);
        let routing_prom = vm.export_routing_prometheus();
        assert!(routing_prom.contains("vm_routing_fast_total 2"));
        assert!(routing_prom.contains("vm_routing_consensus_total 1"));
        assert!(routing_prom.contains("vm_routing_privacy_total 1"));

    // Batch result sanity: consensus_res 应包含原始 consensus + fallback 重试 + privacy（privacy 目前也走 consensus 执行）
    assert!(consensus_res.successful >= 2, "Consensus group should succeed for shared + privacy; fallback 重试的失败项仍可能失败（本实现对失败项未更换闭包）");
    }
}

// ===========================================================
// ZK 批量验证缓冲 - 基本测试（feature: groth16-verifier）
// ===========================================================
#[cfg(test)]
#[cfg(feature = "groth16-verifier")]
mod zk_batch_buffer_tests {
    use super::*;
    use crate::{OwnershipManager, OwnershipType, ObjectMetadata};
    use crate::parallel_mvcc::MvccScheduler;
    use crate::zk_verifier::{ZkVerifier, MockVerifier};

    type Address = [u8;32];
    type ObjectId = [u8;32];
    fn addr(id: u8) -> Address { let mut a=[0u8;32]; a[0]=id; a }
    fn obj(id: u8) -> ObjectId { let mut o=[0u8;32]; o[0]=id; o }

    fn reg_owned(om: &OwnershipManager, id: ObjectId, owner: Address) {
        let meta = ObjectMetadata { id, version:0, ownership: OwnershipType::Owned(owner), object_type: "Test".into(), created_at:0, updated_at:0, size:0, is_deleted:false};
        om.register_object(meta).unwrap();
    }

    #[test]
    fn batch_flushes_on_size_and_records_metrics() {
        // Prepare env for batch
        std::env::set_var("ZK_BATCH_ENABLE", "1");
        std::env::set_var("ZK_BATCH_SIZE", "2");
        std::env::set_var("ZK_BATCH_FLUSH_INTERVAL_MS", "1000");

        let ownership = OwnershipManager::new();
        // any object to allow route private
        reg_owned(&ownership, obj(1), addr(1));
        let scheduler = MvccScheduler::new();

        // Leak a mock verifier to satisfy lifetime
        let leaked: &'static dyn ZkVerifier = Box::leak(Box::new(MockVerifier::new_always_succeed()));

        let vm = SuperVM::new(&ownership)
            .with_scheduler(&scheduler)
            .with_verifier(leaked)
            .from_env();

        // Push two proofs; MockVerifier ignores content
        let p1 = vec![1u8; 32];
        let i1 = vec![2u8; 8];
        let p2 = vec![3u8; 32];
        let i2 = vec![4u8; 8];

        assert!(vm.verify_zk_proof(Some(&p1), Some(&i1))); // buffered, not flushed yet
        assert!(vm.verify_zk_proof(Some(&p2), Some(&i2))); // triggers flush by size

        // Metrics should include batch verify totals
        let prom = scheduler.store().get_metrics().unwrap().export_prometheus();
        assert!(prom.contains("vm_privacy_zk_batch_verify_total 2"));
        assert!(prom.contains("vm_privacy_zk_batch_verify_batches_total 1"));
    }
}
