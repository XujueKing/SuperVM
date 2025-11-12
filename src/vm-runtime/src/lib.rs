// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2025 XujueKing <leadbrand@me.com>

// 开发者：king
// Developer: king
use anyhow::Result;
use std::cell::RefCell;
use std::rc::Rc;
use wasmtime::{Engine, Instance, Linker, Module, Store};

pub mod auto_tuner; // Phase 4.2: 自适应性能调优器 (智能参数调节)
pub mod bloom_filter; // Phase 4.1: 布隆过滤器 (冲突检测优化)
pub mod cross_shard_mvcc; // Phase 6: 跨分片 MVCC 扩展
mod crypto;
pub mod execution_trait; // L1: 统一执行引擎接口 (WASM/EVM)
mod host;
pub mod metrics;
pub mod mvcc;
pub mod optimized_mvcc; // Phase 4.1: 优化的 MVCC 调度器 (集成布隆过滤器)
pub mod ownership; // v2.0: Sui-Inspired 对象所有权模型
pub mod parallel;
pub mod parallel_mvcc; // v0.9.0: 新的基于 MVCC 的并行调度器
pub mod privacy; // Phase 2.0: Privacy Layer (Ring Signatures, Stealth Addresses, etc.)
pub mod shard_coordinator; // Phase 6: 分片协调器 (2PC)
pub mod shard_types; // Phase 6: 跨分片事务类型定义
#[cfg(feature = "partitioned-fastpath")]
pub mod partitioned_fastpath; // Expose partitioned_fastpath module when feature is enabled
#[cfg(feature = "partitioned-fastpath")]
pub mod multi_core_consensus; // 多核共识调度器（初始原型）
#[cfg(feature = "partitioned-fastpath")]
pub mod two_phase_consensus; // 2PC 原型（多分区提交）
#[cfg(feature = "cross-shard")]
pub mod shard; // Phase B: gRPC ShardService (proto + server skeleton)
mod storage;
pub mod supervm; // v2.0: 统一入口与模式路由 // Phase 4.3: 性能指标收集器 (Prometheus 格式)
#[cfg(feature = "groth16-verifier")]
pub mod zk_verifier; // Phase 6: 真实 ZK 验证器集成
pub mod adaptive_router; // Phase 5+: 自适应路由器（动态调整 Fast/Consensus 比例）

pub use auto_tuner::{AutoTuner, AutoTunerSummary};
pub use bloom_filter::{BloomFilter, BloomFilterCache, BloomFilterCacheStats};
pub use cross_shard_mvcc::{CrossShardMvccExt, CrossShardScheduler};
pub use execution_trait::{
    ContractResult, EngineType, ExecutionContext, ExecutionEngine, Log, StateChange,
};
use host::{chain_api, crypto_api, storage_api, HostState};
pub use metrics::{LatencyHistogram, MetricsCollector};
pub use mvcc::{
    AdaptiveGcStrategy, AutoFlushConfig, AutoGcConfig, AutoGcRuntime, FlushStats, GcConfig,
    GcStats, MvccStore, Txn, Version,
};
pub use optimized_mvcc::{
    OptimizedMvccScheduler, OptimizedSchedulerConfig, OptimizedSchedulerStats,
};
pub use ownership::{
    AccessType, Address, Object, ObjectId, ObjectMetadata, OwnershipManager, OwnershipStats,
    OwnershipType,
};
pub use parallel::{
    ConflictDetector, DependencyGraph, ExecutionResult, ExecutionStats, ParallelScheduler,
    ReadWriteSet, StateManager, StorageSnapshot, Task, TxId, WorkStealingScheduler,
};
pub use parallel_mvcc::{
    BatchTxnResult, MvccScheduler, MvccSchedulerConfig, MvccSchedulerStats, TxnResult,
};
pub use shard_coordinator::{CoordinatorError, ShardCoordinator};
pub use shard_types::{
    CommitRequest, CommitResponse, CommitStatus, ConflictReason, CrossShardTxn, Decision,
    PrepareRequest, PrepareResponse, ShardConfig, ShardId, TxnId, TxnState, VersionRequest,
    VersionResponse, shard_for_object,
};
#[cfg(feature = "cross-shard")]
pub use shard::proto as cross_shard_proto;
#[cfg(feature = "rocksdb-storage")]
pub use storage::{AdaptiveBatchConfig, AdaptiveBatchResult, RocksDBConfig, RocksDBStorage, RocksDBMetrics};
pub use storage::{MemoryStorage, Storage};
pub use supervm::{
    ExecutionPath, ExecutionReceipt, Privacy, SuperVM, Transaction as VmTransaction,
};
#[cfg(feature = "groth16-verifier")]
pub use zk_verifier::{Groth16Verifier, ProofBytes, PublicInputBytes, ZkError, ZkVerifier};

// Phase 4.3: 单元测试模块
#[cfg(all(test, feature = "rocksdb-storage"))]
mod auto_flush_tests;
#[cfg(test)]
mod metrics_tests;
#[cfg(all(test, feature = "rocksdb-storage"))]
mod state_pruning_tests;

// Type alias for complex transaction tuple in Runtime API
type RuntimeTxnTuple = (
    TxId,
    VmTransaction,
    std::sync::Arc<dyn Fn(&mut Txn) -> Result<i32> + Send + Sync>,
);

/// VM 运行时的主要接口
pub struct Runtime<S: Storage = MemoryStorage> {
    engine: Engine,
    storage: Rc<RefCell<S>>,
    /// Phase 1.3: 集成对象所有权管理
    ownership_manager: Option<std::sync::Arc<OwnershipManager>>,
    /// Phase 1.3: 集成 MVCC 调度器
    scheduler: Option<std::sync::Arc<MvccScheduler>>,
    #[cfg(feature = "hybrid-exec")]
    hybrid: Option<HybridComponents>,
}

impl<S: Storage + 'static> Runtime<S> {
    /// 创建新的运行时实例，storage 将被内部 Rc 包装以便在 host 中共享
    pub fn new(storage: S) -> Self {
        Self {
            engine: Engine::default(),
            storage: Rc::new(RefCell::new(storage)),
            ownership_manager: None,
            scheduler: None,
            #[cfg(feature = "hybrid-exec")]
            hybrid: None, // 由调用方显式启用 hybrid 初始化（避免 Debug derive 复杂度）
        }
    }

    /// Phase 1.3: 创建带路由能力的运行时
    pub fn new_with_routing(storage: S) -> Self {
        Self {
            engine: Engine::default(),
            storage: Rc::new(RefCell::new(storage)),
            ownership_manager: Some(std::sync::Arc::new(OwnershipManager::new())),
            scheduler: Some(std::sync::Arc::new(MvccScheduler::new())),
            #[cfg(feature = "hybrid-exec")]
            hybrid: Some(HybridComponents::new_default()),
        }
    }

    /// 获取存储接口的不可变引用（内部为 Rc<RefCell>）
    pub fn storage(&self) -> Rc<RefCell<S>> {
        self.storage.clone()
    }
        /// 初始化 Hybrid 组件（延迟初始化以避免默认构造开销）
        pub fn init_hybrid(&mut self) {
            if self.hybrid.is_none() {
                self.hybrid = Some(HybridComponents::new_default());
            }
        }

        #[cfg(feature = "hybrid-exec")]
        pub fn init_hybrid_low_overhead(&mut self) {
            self.hybrid = Some(HybridComponents::new_low_overhead());
        }

    /// Phase 1.3: 获取所有权管理器
    pub fn ownership_manager(&self) -> Option<&std::sync::Arc<OwnershipManager>> {
        self.ownership_manager.as_ref()
    }

    /// Phase 1.3: 获取调度器
    pub fn scheduler(&self) -> Option<&std::sync::Arc<MvccScheduler>> {
        self.scheduler.as_ref()
    }

    #[cfg(feature = "hybrid-exec")]
    pub fn hybrid(&self) -> Option<&HybridComponents> { self.hybrid.as_ref() }

    /// 注册 host functions 到 linker
    fn register_host_functions(&self, linker: &mut Linker<HostState<S>>) -> Result<()> {
        // 注册存储相关函数
        linker.func_wrap("storage_api", "storage_get", storage_api::storage_get)?;
        linker.func_wrap(
            "storage_api",
            "storage_read_value",
            storage_api::storage_read_value,
        )?;
        linker.func_wrap("storage_api", "storage_set", storage_api::storage_set)?;
        linker.func_wrap("storage_api", "storage_delete", storage_api::storage_delete)?;
        // 注册链/事件相关函数
        linker.func_wrap("chain_api", "block_number", chain_api::block_number)?;
        linker.func_wrap("chain_api", "timestamp", chain_api::timestamp)?;
        linker.func_wrap("chain_api", "emit_event", chain_api::emit_event)?;
        linker.func_wrap("chain_api", "events_len", chain_api::events_len)?;
        linker.func_wrap("chain_api", "read_event", chain_api::read_event)?;
        // 注册密码学相关函数
        linker.func_wrap("crypto_api", "sha256", crypto_api::sha256)?;
        linker.func_wrap("crypto_api", "keccak256", crypto_api::keccak256)?;
        linker.func_wrap(
            "crypto_api",
            "verify_secp256k1",
            crypto_api::verify_secp256k1,
        )?;
        linker.func_wrap("crypto_api", "verify_ed25519", crypto_api::verify_ed25519)?;
        linker.func_wrap(
            "crypto_api",
            "recover_secp256k1_pubkey",
            crypto_api::recover_secp256k1_pubkey,
        )?;
        linker.func_wrap(
            "crypto_api",
            "derive_eth_address",
            crypto_api::derive_eth_address,
        )?;
        Ok(())
    }

    /// 在给定 store 上实例化模块（会注册 host functions）
    fn instantiate(&self, store: &mut Store<HostState<S>>, module: &Module) -> Result<Instance> {
        let mut linker = Linker::new(&self.engine);
        self.register_host_functions(&mut linker)?;
        let instance = linker.instantiate(store, module)?;
        Ok(instance)
    }

    /// 加载并调用导出函数 `add(i32, i32) -> i32`，返回结果
    pub fn execute_add(&self, module_bytes: &[u8], a: i32, b: i32) -> Result<i32> {
        let module = Module::new(&self.engine, module_bytes)?;

        // 创建 Store，并将 storage 的 Rc 克隆到 HostState 中
        let mut store = Store::new(
            &self.engine,
            HostState {
                storage: self.storage.clone(),
                memory: None,
                last_get: None,
                events: Vec::new(),
                block_number: 0,
                timestamp: 0,
                read_write_set: ReadWriteSet::new(),
            },
        );

        let instance = self.instantiate(&mut store, &module)?;

        // 获取导出的内存并保存
        if let Some(memory) = instance.get_memory(&mut store, "memory") {
            store.data_mut().memory = Some(memory);
        }

        let add = instance.get_typed_func::<(i32, i32), i32>(&mut store, "add")?;
        let res = add.call(&mut store, (a, b))?;
        Ok(res)
    }

    /// 执行 WASM 模块并返回结果与事件
    ///
    /// 调用指定的导出函数（无参数 -> i32），并返回：
    /// - 函数返回值
    /// - 执行过程中收集的事件列表
    /// - 区块号与时间戳（从 HostState 中获取）
    pub fn execute_with_context(
        &self,
        module_bytes: &[u8],
        func_name: &str,
        block_number: u64,
        timestamp: u64,
    ) -> Result<(i32, Vec<Vec<u8>>, u64, u64)> {
        let module = Module::new(&self.engine, module_bytes)?;

        let mut store = Store::new(
            &self.engine,
            HostState {
                storage: self.storage.clone(),
                memory: None,
                last_get: None,
                events: Vec::new(),
                block_number,
                timestamp,
                read_write_set: ReadWriteSet::new(),
            },
        );

        let instance = self.instantiate(&mut store, &module)?;

        // 获取导出的内存并保存
        if let Some(memory) = instance.get_memory(&mut store, "memory") {
            store.data_mut().memory = Some(memory);
        }

        // 调用指定的导出函数
        let func = instance.get_typed_func::<(), i32>(&mut store, func_name)?;
        let result = func.call(&mut store, ())?;

        // 提取事件与上下文
        let events = store.data().events.clone();
        let block_num = store.data().block_number;
        let ts = store.data().timestamp;

        Ok((result, events, block_num, ts))
    }

    /// 执行 WASM 模块并返回完整的执行结果 (包括读写集)
    ///
    /// 用于并行执行场景
    pub fn execute_with_rw_tracking(
        &self,
        module_bytes: &[u8],
        func_name: &str,
        block_number: u64,
        timestamp: u64,
    ) -> Result<ExecutionResult> {
        let module = Module::new(&self.engine, module_bytes)?;

        let mut store = Store::new(
            &self.engine,
            HostState {
                storage: self.storage.clone(),
                memory: None,
                last_get: None,
                events: Vec::new(),
                block_number,
                timestamp,
                read_write_set: ReadWriteSet::new(),
            },
        );

        let instance = self.instantiate(&mut store, &module)?;

        // 获取导出的内存并保存
        if let Some(memory) = instance.get_memory(&mut store, "memory") {
            store.data_mut().memory = Some(memory);
        }

        // 调用指定的导出函数
        let func = instance.get_typed_func::<(), i32>(&mut store, func_name)?;
        let result = func.call(&mut store, ());

        // 提取所有状态
        let events = store.data().events.clone();
        let read_write_set = store.data().read_write_set.clone();

        match result {
            Ok(return_value) => Ok(ExecutionResult {
                tx_id: 0, // 由调用者设置
                return_value,
                read_write_set,
                events,
                success: true,
                error: None,
            }),
            Err(e) => Ok(ExecutionResult {
                tx_id: 0,
                return_value: -1,
                read_write_set,
                events,
                success: false,
                error: Some(e.to_string()),
            }),
        }
    }

    /// Phase 1.3: 带路由的交易执行入口
    ///
    /// 根据交易的隐私模式和对象所有权自动路由到 Fast/Consensus/Private 路径
    pub fn execute_with_routing(
        &self,
        tx_id: TxId,
        tx: &VmTransaction,
        func: impl Fn(&mut Txn) -> Result<i32>,
    ) -> Result<ExecutionReceipt> {
        let ownership = self.ownership_manager.as_ref().ok_or_else(|| {
            anyhow::anyhow!("Runtime not configured with routing, use new_with_routing()")
        })?;
        let scheduler = self
            .scheduler
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("Runtime not configured with scheduler"))?;

        let supervm = SuperVM::new(ownership).with_scheduler(scheduler);
        Ok(supervm.execute_transaction_with(tx_id, tx, func))
    }

    /// Phase 1.3: 带路由的批量交易执行
    pub fn execute_batch_with_routing(
        &self,
        txs: Vec<RuntimeTxnTuple>,
    ) -> Result<(BatchTxnResult, BatchTxnResult, u64, Vec<u64>)> {
        let ownership = self
            .ownership_manager
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("Runtime not configured with routing"))?;
        let scheduler = self
            .scheduler
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("Runtime not configured with scheduler"))?;

        let supervm = SuperVM::new(ownership).with_scheduler(scheduler);
        Ok(supervm.execute_batch(txs))
    }

    /// Phase 1.3: 获取路由统计
    pub fn get_routing_stats(&self) -> Option<OwnershipStats> {
        self.ownership_manager.as_ref().map(|m| m.get_stats())
    }

    // init_hybrid 已在后续 hybrid-exec impl 模块提供，避免重复定义
}

// ================= Phase 13: Hybrid Executor Integration =================
#[cfg(feature = "hybrid-exec")]
mod hybrid_integration {
    use super::*;
    use gpu_executor::{Batch as HybridBatch, Task as HybridTask, ParallelCpuExecutor, HybridScheduler, HybridStrategy, UnavailableGpu, SchedulerStats};
    use std::sync::Mutex;

    pub struct HybridComponents {
        pub scheduler: Mutex<HybridScheduler<ParallelCpuExecutor<fn(&HybridWork) -> i32>, UnavailableGpu, HybridWork, i32>>,
    }

    #[derive(Clone)]
    pub enum HybridOp {
        Dyn(std::sync::Arc<dyn Fn() -> i32 + Send + Sync>),
           Fn { f: fn(u64) -> i32, arg: u64 },
           Fn2 { f: fn(u64, u64) -> i32, a: u64, b: u64 },
    }

    #[derive(Clone)]
    pub struct HybridWork {
        pub tx_id: TxId,
        pub cost: u64,
        pub op: HybridOp,
    }

    impl HybridComponents {
        pub fn new_default() -> Self {
            fn exec_fn(w: &HybridWork) -> i32 {
                match &w.op {
                    HybridOp::Dyn(f) => (f)(),
                    HybridOp::Fn { f, arg } => (f)(*arg),
                    HybridOp::Fn2 { f, a, b } => (f)(*a, *b),
                }
            }
            let parallel = ParallelCpuExecutor::new(exec_fn as fn(&HybridWork) -> i32).with_adaptive(4);
            let gpu = UnavailableGpu;
            let strategy = HybridStrategy::default();
            let sched = HybridScheduler::new(parallel, Some(gpu), strategy);
            Self { scheduler: Mutex::new(sched) }
        }

        /// 低开销版本：关闭自适应 / GPU 逻辑，只使用并行 CPU。
        pub fn new_low_overhead() -> Self {
            fn exec_fn(w: &HybridWork) -> i32 {
                match &w.op {
                    HybridOp::Dyn(f) => (f)(),
                    HybridOp::Fn { f, arg } => (f)(*arg),
                    HybridOp::Fn2 { f, a, b } => (f)(*a, *b),
                }
            }
            let parallel = ParallelCpuExecutor::new(exec_fn as fn(&HybridWork) -> i32).with_adaptive(4);
            let gpu = UnavailableGpu;
            let mut strategy = HybridStrategy::default();
            strategy.adaptive_enabled = false;
            strategy.gpu_threshold = usize::MAX / 2; // 实际不触发 GPU
            let sched = HybridScheduler::new(parallel, Some(gpu), strategy);
            Self { scheduler: Mutex::new(sched) }
        }

        pub fn execute_batch(&self, works: Vec<HybridWork>) -> Vec<(TxId, i32)> {
            let batch = HybridBatch { tasks: works.into_iter().enumerate().map(|(i, w)| HybridTask { id: i as u64, payload: w.clone(), est_cost: w.cost }).collect() };
            let mut sched = self.scheduler.lock().unwrap();
            let (results, _stats) = sched.schedule(&batch).expect("hybrid schedule ok");
            results.into_iter().map(|tr| {
                let hw = &batch.tasks[tr.id as usize].payload; // id 对应索引
                (hw.tx_id, tr.output)
            }).collect()
        }

        pub fn execute_batch_fn(&self, items: Vec<(TxId, u64, fn(u64)->i32)>) -> Vec<(TxId,i32)> {
            let works: Vec<HybridWork> = items.into_iter().map(|(tx_id, arg, f)| HybridWork { tx_id, cost: 10, op: HybridOp::Fn { f, arg } }).collect();
            self.execute_batch(works)
        }

        pub fn execute_batch_fn2(&self, items: Vec<(TxId, u64, u64, fn(u64,u64)->i32)>) -> Vec<(TxId,i32)> {
            let works: Vec<HybridWork> = items.into_iter().map(|(tx_id, a, b, f)| HybridWork { tx_id, cost: 10, op: HybridOp::Fn2 { f, a, b } }).collect();
            self.execute_batch(works)
        }

        pub fn stats(&self) -> SchedulerStats {
            let sched = self.scheduler.lock().unwrap();
            sched.stats_snapshot()
        }
    }
}

#[cfg(feature = "hybrid-exec")]
pub use hybrid_integration::{HybridComponents, HybridWork};

#[cfg(feature = "hybrid-exec")]
impl<S: Storage + 'static> Runtime<S> {
    /// 批量事务执行（Hybrid）: 将一组 (TxId, Txn闭包) 转换为 HybridWork 并交给并行执行器
    pub fn execute_with_hybrid<F>(&self, operations: Vec<(TxId, F)>) -> Vec<(TxId, i32)>
    where
        F: Fn() -> i32 + Send + Sync + 'static,
    {
        if let Some(h) = &self.hybrid {
            #[cfg(not(feature = "hybrid-lite"))]
            let start = std::time::Instant::now();
            use crate::hybrid_integration::HybridOp;
            let works: Vec<HybridWork> = operations.into_iter().map(|(id, f)| HybridWork { tx_id: id, cost: 10, op: HybridOp::Dyn(std::sync::Arc::new(f)) }).collect();
            #[cfg(not(feature = "hybrid-lite"))]
            let batch_size = works.len();
            let results = h.execute_batch(works);
            // 记录指标（若 metrics collector 存在），在 lite 模式下跳过
            #[cfg(not(feature = "hybrid-lite"))]
            if let Some(sched) = &self.scheduler {
                if let Some(mc) = sched.store().get_metrics() {
                    mc.record_hybrid_batch(batch_size, start.elapsed(), rayon::current_num_threads());
                    mc.inc_hybrid_routing_decision();
                }
            }
            results
        } else {
            // fallback: 顺序执行
            operations.into_iter().map(|(id, f)| (id, f())).collect()
        }
    }

    /// 获取混合调度器统计（若未初始化返回 None）
    pub fn hybrid_stats(&self) -> Option<gpu_executor::SchedulerStats> {
        self.hybrid.as_ref().map(|h| h.stats())
    }

    /// 函数指针快速路径：减少 Box/Arc 动态分发开销
    pub fn execute_with_hybrid_fn(&self, operations: Vec<(TxId, u64, fn(u64)->i32)>) -> Vec<(TxId,i32)> {
        if let Some(h) = &self.hybrid {
            #[cfg(not(feature = "hybrid-lite"))]
            let start = std::time::Instant::now();
            #[cfg(not(feature = "hybrid-lite"))]
            let batch_size = operations.len();
            let results = h.execute_batch_fn(operations);
            #[cfg(not(feature = "hybrid-lite"))]
            if let Some(sched) = &self.scheduler {
                if let Some(mc) = sched.store().get_metrics() {
                    mc.record_hybrid_batch(batch_size, start.elapsed(), rayon::current_num_threads());
                    mc.inc_hybrid_routing_decision();
                }
            }
            results
        } else {
            // fallback: 顺序
            operations.into_iter().map(|(id,arg,f)| (id, f(arg))).collect()
        }
    }

    /// 自动分块 + 函数指针快速路径：根据任务总数决定是否启用分块（粗化）以降低调度开销。
    /// 策略：若 tasks > auto_chunk_threshold (默认 2_000) 则按 chunk_size (默认 1_000) 分块。
    pub fn execute_with_hybrid_auto_chunked(&self, operations: Vec<(TxId, u64, fn(u64)->i32)>) -> Vec<(TxId,i32)> {
        // 通过环境变量允许覆盖阈值与分块大小：
        // HYBRID_AUTO_CHUNK_THRESHOLD（默认 2000），HYBRID_CHUNK_SIZE（默认 1000）
        use once_cell::sync::OnceCell;
        struct AutoChunkCfg { threshold: usize, chunk: usize }
        static CFG: OnceCell<AutoChunkCfg> = OnceCell::new();
        let cfg = CFG.get_or_init(|| {
            let th = std::env::var("HYBRID_AUTO_CHUNK_THRESHOLD").ok().and_then(|s| s.parse().ok()).unwrap_or(2_000usize);
            let ch = std::env::var("HYBRID_CHUNK_SIZE").ok().and_then(|s| s.parse().ok()).unwrap_or(1_000usize);
            AutoChunkCfg { threshold: th.max(1), chunk: ch.max(1) }
        });
        let auto_threshold = cfg.threshold;
        // 自适应 chunk 选择：若未通过环境变量显式指定，按经验公式 chunk ~= round_to_500(total/50)，并限制在 [500,2000]
        let total = operations.len();
        let chunk_size = if std::env::var("HYBRID_CHUNK_SIZE").is_ok() {
            cfg.chunk
        } else {
            let approx = (total.max(1) / 50).max(1);
            let rounded = ((approx + 249) / 500) * 500; // 向最近 500 步进对齐（上取整）
            rounded.clamp(500, 2000)
        };
        if total <= auto_threshold { return self.execute_with_hybrid_fn(operations); }
        if let Some(h) = &self.hybrid {
            #[cfg(not(feature = "hybrid-lite"))]
            let start = std::time::Instant::now();
            // 将原始操作聚合成块：每块返回其内部所有调用结果的 wrapping sum，保留第一个 tx_id 作为块 ID 引用
            use crate::hybrid_integration::HybridOp; // 保证可用
            let mut works: Vec<crate::hybrid_integration::HybridWork> = Vec::new();
            for chunk_start in (0..total).step_by(chunk_size) {
                let chunk_end = (chunk_start + chunk_size).min(total);
                let (first_tx, _, _) = operations[chunk_start];
                // 直接聚合闭包，显著减少任务数。
                let mut acc_ops: Vec<(TxId, u64, fn(u64)->i32)> = Vec::with_capacity(chunk_end - chunk_start);
                for i in chunk_start..chunk_end { acc_ops.push(operations[i]); }
                // 构造一个动态聚合闭包
                let dyn_closure = move || {
                    let mut acc: i32 = 0;
                    for &(_, arg, f) in &acc_ops { acc = acc.wrapping_add(f(arg)); }
                    acc
                };
                works.push(crate::hybrid_integration::HybridWork { tx_id: first_tx, cost: 10, op: HybridOp::Dyn(std::sync::Arc::new(dyn_closure)) });
            }
            #[cfg(not(feature = "hybrid-lite"))]
            let batch_size = works.len();
            let results = h.execute_batch(works);
            #[cfg(not(feature = "hybrid-lite"))]
            if let Some(sched) = &self.scheduler { if let Some(mc) = sched.store().get_metrics() { mc.record_hybrid_batch(batch_size, start.elapsed(), rayon::current_num_threads()); mc.inc_hybrid_routing_decision(); } }
            results
        } else {
            // fallback 顺序：同样分块计算，以保持语义一致（返回每块聚合值）
            let mut out = Vec::new();
            for chunk_start in (0..total).step_by(chunk_size) {
                let chunk_end = (chunk_start + chunk_size).min(total);
                let (first_tx, _, _) = operations[chunk_start];
                let mut acc: i32 = 0;
                for i in chunk_start..chunk_end { let (_, arg, f) = operations[i]; acc = acc.wrapping_add(f(arg)); }
                out.push((first_tx, acc));
            }
            out
        }
    }

    /// 显式分块版本：调用方指定 chunk_size，按块聚合执行。
    pub fn execute_with_hybrid_chunked_with(&self, operations: Vec<(TxId, u64, fn(u64)->i32)>, chunk_size: usize) -> Vec<(TxId,i32)> {
        let chunk = chunk_size.max(1);
        let total = operations.len();
        if total == 0 { return Vec::new(); }
        if let Some(h) = &self.hybrid {
            #[cfg(not(feature = "hybrid-lite"))]
            let start = std::time::Instant::now();
            use crate::hybrid_integration::HybridOp;
            let mut works: Vec<crate::hybrid_integration::HybridWork> = Vec::new();
            for chunk_start in (0..total).step_by(chunk) {
                let chunk_end = (chunk_start + chunk).min(total);
                let (first_tx, _, _) = operations[chunk_start];
                let mut acc_ops: Vec<(TxId,u64,fn(u64)->i32)> = Vec::with_capacity(chunk_end - chunk_start);
                for i in chunk_start..chunk_end { acc_ops.push(operations[i]); }
                let dyn_closure = move || {
                    let mut acc: i32 = 0;
                    for &(_, arg, f) in &acc_ops { acc = acc.wrapping_add(f(arg)); }
                    acc
                };
                works.push(crate::hybrid_integration::HybridWork { tx_id: first_tx, cost: 10, op: HybridOp::Dyn(std::sync::Arc::new(dyn_closure)) });
            }
            #[cfg(not(feature = "hybrid-lite"))]
            let batch_size = works.len();
            let results = h.execute_batch(works);
            #[cfg(not(feature = "hybrid-lite"))]
            if let Some(sched) = &self.scheduler { if let Some(mc) = sched.store().get_metrics() { mc.record_hybrid_batch(batch_size, start.elapsed(), rayon::current_num_threads()); mc.inc_hybrid_routing_decision(); } }
            results
        } else {
            // fallback：顺序按块聚合
            let mut out = Vec::new();
            for chunk_start in (0..total).step_by(chunk) {
                let chunk_end = (chunk_start + chunk).min(total);
                let (first_tx, _, _) = operations[chunk_start];
                let mut acc: i32 = 0;
                for i in chunk_start..chunk_end { let (_, arg, f) = operations[i]; acc = acc.wrapping_add(f(arg)); }
                out.push((first_tx, acc));
            }
            out
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::MemoryStorage;

    #[test]
    fn test_execute_add_via_wat() {
        // 一个简单的 WAT 模块，导出 add 函数
        let wat = r#"
        (module
          (func $add (export "add") (param i32 i32) (result i32)
            local.get 0
            local.get 1
            i32.add)
        )
        "#;

        let wasm = wat::parse_str(wat).expect("failed to parse wat");
        let rt = Runtime::new(MemoryStorage::new());
        let result = rt.execute_add(&wasm, 2, 3).expect("execution failed");
        assert_eq!(result, 5);
    }

    #[test]
    fn test_storage() -> Result<()> {
        let rt = Runtime::new(MemoryStorage::new());

        // 测试存储操作通过直接操作存储接口
        rt.storage().borrow_mut().set(b"test_key", b"test_value")?;
        assert_eq!(
            rt.storage().borrow().get(b"test_key")?.unwrap(),
            b"test_value"
        );

        Ok(())
    }

    #[test]
    fn test_host_functions() -> Result<()> {
        let rt = Runtime::new(MemoryStorage::new());

        // 一个使用存储 API 的 WAT 模块
        let wat = r#"
        (module
          ;; 导入存储相关函数
          (import "storage_api" "storage_get" (func $storage_get (param i32 i32) (result i64)))
          (import "storage_api" "storage_set" (func $storage_set (param i32 i32 i32 i32) (result i32)))
          
          ;; 导出内存
          (memory (export "memory") 1)
          
          ;; 存储一些常量字符串
          (data (i32.const 100) "test_key")
          (data (i32.const 200) "test_value")
          
                    ;; 导出的测试函数
                    (func (export "test_storage") (result i32)
                        ;; 写入键值对
                        (call $storage_set
                            (i32.const 100)    ;; key_ptr
                            (i32.const 8)      ;; key_len
                            (i32.const 200)    ;; value_ptr
                            (i32.const 10))    ;; value_len
                        drop
                        (i32.const 0)
                    )
        )
        "#;

        let wasm = wat::parse_str(wat)?;
        let mut store = Store::new(
            &rt.engine,
            HostState {
                storage: rt.storage.clone(),
                memory: None,
                last_get: None,
                events: Vec::new(),
                block_number: 0,
                timestamp: 0,
                read_write_set: ReadWriteSet::new(),
            },
        );
        let module = Module::new(&rt.engine, &wasm)?;
        let instance = rt.instantiate(&mut store, &module)?;

        if let Some(memory) = instance.get_memory(&mut store, "memory") {
            store.data_mut().memory = Some(memory);
        }

        let test_fn = instance.get_typed_func::<(), i32>(&mut store, "test_storage")?;
        let result = test_fn.call(&mut store, ())?;

        assert_eq!(result, 0); // 0 表示成功
        assert_eq!(
            store.data().storage.borrow().get(b"test_key")?.unwrap(),
            b"test_value"
        );

        Ok(())
    }

    #[test]
    fn test_emit_event() -> Result<()> {
        let rt = Runtime::new(MemoryStorage::new());
        let wat = r#"
                (module
                    (import "chain_api" "emit_event" (func $emit_event (param i32 i32) (result i32)))
                    (import "chain_api" "events_len" (func $events_len (result i32)))
                    (import "chain_api" "read_event" (func $read_event (param i32 i32 i32) (result i32)))
                    (memory (export "memory") 1)
                    (data (i32.const 100) "my_event")
                    ;; 200..207 用作读取缓冲区
                    (func (export "test_emit") (result i32)
                        (call $emit_event (i32.const 100) (i32.const 8))
                        drop
                        (call $events_len)
                        drop
                        ;; 从索引 0 读取事件到地址 200
                        (call $read_event (i32.const 0) (i32.const 200) (i32.const 8))
                        drop
                        (i32.const 0)
                    )
                )
        "#;
        let wasm = wat::parse_str(wat)?;
        let mut store = Store::new(
            &rt.engine,
            HostState {
                storage: rt.storage.clone(),
                memory: None,
                last_get: None,
                events: Vec::new(),
                block_number: 0,
                timestamp: 0,
                read_write_set: ReadWriteSet::new(),
            },
        );
        let module = Module::new(&rt.engine, &wasm)?;
        let instance = rt.instantiate(&mut store, &module)?;
        if let Some(memory) = instance.get_memory(&mut store, "memory") {
            store.data_mut().memory = Some(memory);
        }
        let test_fn = instance.get_typed_func::<(), i32>(&mut store, "test_emit")?;
        let result = test_fn.call(&mut store, ())?;
        assert_eq!(result, 0);
        assert_eq!(store.data().events.len(), 1);
        assert_eq!(store.data().events[0].as_slice(), b"my_event");
        Ok(())
    }

    #[test]
    fn test_execute_with_context() -> Result<()> {
        let rt = Runtime::new(MemoryStorage::new());

        // WAT 模块：发射两个事件并返回 42
        let wat = r#"
        (module
            (import "chain_api" "emit_event" (func $emit_event (param i32 i32) (result i32)))
            (import "chain_api" "block_number" (func $block_number (result i64)))
            (import "chain_api" "timestamp" (func $timestamp (result i64)))
            (memory (export "memory") 1)
            (data (i32.const 100) "event_one")
            (data (i32.const 200) "event_two")
            
            (func (export "run") (result i32)
                ;; 发射第一个事件
                (call $emit_event (i32.const 100) (i32.const 9))
                drop
                
                ;; 发射第二个事件
                (call $emit_event (i32.const 200) (i32.const 9))
                drop
                
                ;; 返回 42
                (i32.const 42)
            )
        )
        "#;

        let wasm = wat::parse_str(wat)?;
        let (result, events, block_num, ts) =
            rt.execute_with_context(&wasm, "run", 12345, 67890)?;

        // 验证返回值
        assert_eq!(result, 42);

        // 验证事件
        assert_eq!(events.len(), 2);
        assert_eq!(events[0].as_slice(), b"event_one");
        assert_eq!(events[1].as_slice(), b"event_two");

        // 验证上下文
        assert_eq!(block_num, 12345);
        assert_eq!(ts, 67890);

        Ok(())
    }

    #[test]
    fn test_crypto_sha256() -> Result<()> {
        let rt = Runtime::new(MemoryStorage::new());

        let wat = r#"
            (module
                (import "crypto_api" "sha256" (func $sha256 (param i32 i32 i32) (result i32)))
                (memory (export "memory") 1)
                (data (i32.const 0) "hello world")
            
                (func (export "hash") (result i32)
                    ;; 调用 sha256("hello world")
                    ;; 输入在地址 0, 长度 11
                    ;; 输出写入地址 100
                    (call $sha256 (i32.const 0) (i32.const 11) (i32.const 100))
                )
            )
            "#;

        let wasm = wat::parse_str(wat)?;
        let (result, _, _, _) = rt.execute_with_context(&wasm, "hash", 0, 0)?;

        assert_eq!(result, 0); // 成功

        // 验证哈希结果
        let storage = rt.storage();
        let store_ref = storage.borrow();
        // 注意: 实际应该从 WASM 内存读取结果,这里只验证调用成功
        drop(store_ref);

        Ok(())
    }

    #[test]
    fn test_crypto_keccak256() -> Result<()> {
        let rt = Runtime::new(MemoryStorage::new());

        let wat = r#"
            (module
                (import "crypto_api" "keccak256" (func $keccak256 (param i32 i32 i32) (result i32)))
                (memory (export "memory") 1)
                (data (i32.const 0) "hello world")
            
                (func (export "hash") (result i32)
                    ;; 调用 keccak256("hello world")
                    (call $keccak256 (i32.const 0) (i32.const 11) (i32.const 100))
                )
            )
            "#;

        let wasm = wat::parse_str(wat)?;
        let (result, _, _, _) = rt.execute_with_context(&wasm, "hash", 0, 0)?;

        assert_eq!(result, 0); // 成功

        Ok(())
    }

    #[test]
    fn test_crypto_verify_signatures() -> Result<()> {
        let rt = Runtime::new(MemoryStorage::new());

        // 测试 secp256k1 验证 (用无效数据测试错误处理)
        let wat = r#"
            (module
                (import "crypto_api" "verify_secp256k1" 
                    (func $verify_secp256k1 (param i32 i32 i32 i32) (result i32)))
                (memory (export "memory") 1)
            
                (func (export "verify") (result i32)
                    ;; 验证签名 (全零数据,应该返回错误或失败)
                    ;; msg_ptr=0 (32字节), sig_ptr=32 (64字节), pubkey_ptr=96 (33字节)
                    (call $verify_secp256k1 
                        (i32.const 0) 
                        (i32.const 32) 
                        (i32.const 96) 
                        (i32.const 33))
                )
            )
            "#;

        let wasm = wat::parse_str(wat)?;
        let (result, _, _, _) = rt.execute_with_context(&wasm, "verify", 0, 0)?;

        // 应该返回 -1 (错误) 或 0 (验证失败)
        assert!(result <= 0);

        Ok(())
    }

    #[test]
    fn test_crypto_derive_eth_address() -> Result<()> {
        let rt = Runtime::new(MemoryStorage::new());

        let wat = r#"
        (module
            (import "crypto_api" "derive_eth_address" 
                (func $derive_eth_address (param i32 i32 i32) (result i32)))
            (memory (export "memory") 1)
            
            (func (export "derive") (result i32)
                ;; 测试用公钥 (33 字节压缩格式,全零为无效公钥)
                ;; pubkey_ptr=0 (33字节), output_ptr=100 (20字节)
                (call $derive_eth_address 
                    (i32.const 0) 
                    (i32.const 33) 
                    (i32.const 100))
            )
        )
        "#;

        let wasm = wat::parse_str(wat)?;
        let (result, _, _, _) = rt.execute_with_context(&wasm, "derive", 0, 0)?;

        // 应该返回 -1 (无效公钥)
        assert_eq!(result, -1);

        Ok(())
    }

    #[test]
    fn test_parallel_read_write_tracking() -> Result<()> {
        let rt = Runtime::new(MemoryStorage::new());

        let wat = r#"
        (module
            (import "storage_api" "storage_set" (func $storage_set (param i32 i32 i32 i32) (result i32)))
            (import "storage_api" "storage_get" (func $storage_get (param i32 i32) (result i64)))
            (memory (export "memory") 1)
            (data (i32.const 0) "alice_balance")
            (data (i32.const 20) "100")
            
            (func (export "test") (result i32)
                ;; 写入 alice_balance
                i32.const 0
                i32.const 13
                i32.const 20
                i32.const 3
                call $storage_set
                drop
                
                ;; 读取 alice_balance
                i32.const 0
                i32.const 13
                call $storage_get
                drop
                
                i32.const 0
            )
        )
        "#;

        let wasm = wat::parse_str(wat)?;
        let exec_result = rt.execute_with_rw_tracking(&wasm, "test", 1, 1000)?;

        // 验证读写集
        assert!(exec_result.success);
        assert!(exec_result
            .read_write_set
            .write_set
            .contains(&b"alice_balance".to_vec()));
        assert!(exec_result
            .read_write_set
            .read_set
            .contains(&b"alice_balance".to_vec()));

        Ok(())
    }

    #[test]
    fn test_parallel_conflict_detection() -> Result<()> {
        use crate::parallel::{ConflictDetector, ReadWriteSet};

        let mut detector = ConflictDetector::new();

        // TX1: 写 alice_balance
        let mut rw1 = ReadWriteSet::new();
        rw1.add_write(b"alice_balance".to_vec());
        detector.record(1, rw1);

        // TX2: 写 bob_balance (无冲突)
        let mut rw2 = ReadWriteSet::new();
        rw2.add_write(b"bob_balance".to_vec());
        detector.record(2, rw2);

        // TX3: 读 alice_balance (与 TX1 冲突)
        let mut rw3 = ReadWriteSet::new();
        rw3.add_read(b"alice_balance".to_vec());
        detector.record(3, rw3);

        // 构建依赖图
        let tx_order = vec![1, 2, 3];
        let graph = detector.build_dependency_graph(&tx_order);

        // TX1 和 TX2 可以并行执行
        assert_eq!(graph.get_dependencies(1).len(), 0);
        assert_eq!(graph.get_dependencies(2).len(), 0);

        // TX3 必须等待 TX1
        assert_eq!(graph.get_dependencies(3), vec![1]);

        Ok(())
    }

    #[test]
    fn test_execution_trait() {
        use crate::execution_trait::*;

        // 测试 EngineType
        assert_eq!(EngineType::Wasm, EngineType::Wasm);
        assert_ne!(EngineType::Wasm, EngineType::Evm);

        // 测试 ExecutionContext
        let ctx = ExecutionContext {
            caller: [1u8; 20],
            contract: [2u8; 20],
            value: 1000,
            gas_limit: 100000,
            block_number: 12345,
            timestamp: 1234567890,
        };
        assert_eq!(ctx.value, 1000);
        assert_eq!(ctx.gas_limit, 100000);

        // 测试 ContractResult
        let result = ContractResult {
            success: true,
            return_data: vec![1, 2, 3],
            gas_used: 5000,
            logs: vec![],
            state_changes: vec![],
        };
        assert!(result.success);
        assert_eq!(result.gas_used, 5000);
    }
}
