// SPDX-License-Identifier: GPL-3.0-or-later
//! Multi-core Consensus Scheduler (初始版本)
//! 目标: 将只涉及单分区的写集合事务并行执行, 保持全局时间戳单调。
//! 策略:
//! - 按 key 哈希分区 (当前使用 ahash 或 fxhash; 先用内置海量简单哈希,可替换)
//! - 如果事务写集合全部落在同一分区,投递到该分区队列,使用局部批量时间戳预分配
//! - 多分区写集合则回退到同步单核提交路径
//! - 时间戳分配: 全局 AtomicU64 按批(默认 512)分配给每个分区的本地缓存
//! - 冲突模型: 单分区内串行执行保持与原实现一致; 跨分区事务因回退不会产生写写并发冲突
//! 后续扩展: 跨分区调和、读集合跨分区并发、版本链分区化。

use std::sync::{Arc, atomic::{AtomicU64, Ordering}};
use std::thread::{self, JoinHandle};
use std::collections::VecDeque;
use std::time::Instant;
use parking_lot::Mutex;


use crate::mvcc::{MvccStore, Txn};
use crate::two_phase_consensus::TwoPhaseCoordinator;
// MetricsCollector 通过 store.get_metrics() 访问, 此处无需直接导入

/// 分区内任务: 拆出需要提交的 Txn
struct PartitionTask { txn: Txn }

/// 分区执行器: 单线程串行执行其分区内的事务
struct PartitionExecutor {
    queue: Mutex<VecDeque<PartitionTask>>, // 简化: FIFO
    // 本地时间戳批次缓存 (start..end)
    ts_next: AtomicU64,
    ts_end: AtomicU64,
    // 诊断：空轮询次数（无任务时）
    empty_polls: AtomicU64,
    // 诊断：最近一次 commit 耗时 (ms*1000)
    last_commit_latency_ms: AtomicU64,
}

impl PartitionExecutor {
    fn new(_id: usize) -> Self { Self { queue: Mutex::new(VecDeque::new()), ts_next: AtomicU64::new(0), ts_end: AtomicU64::new(0), empty_polls: AtomicU64::new(0), last_commit_latency_ms: AtomicU64::new(0) } }
    fn push(&self, t: PartitionTask) { self.queue.lock().push_back(t); }
    fn queue_len(&self) -> usize { self.queue.lock().len() }
}

/// 多核共识调度器
pub struct MultiCoreConsensus {
    store: Arc<MvccStore>,
    partitions: Vec<Arc<PartitionExecutor>>,
    workers: Vec<JoinHandle<()>>,
    global_ts: Arc<AtomicU64>,
    running: Arc<AtomicU64>,
    batch_size: u64,
    routed: AtomicU64,
    fallback: AtomicU64,
    executed: Arc<AtomicU64>,
    // 路由延迟最近一次 (ms*1000)
    route_last_latency_ms: AtomicU64,
    // 分区分配批次次数（触发全局原子抓取）
    batch_refills: AtomicU64,
}

impl MultiCoreConsensus {
    pub fn new(store: Arc<MvccStore>, num_partitions: usize, batch_size: u64) -> Self {
        let partitions = (0..num_partitions).map(|i| Arc::new(PartitionExecutor::new(i))).collect();
        Self { store, partitions, workers: Vec::new(), global_ts: Arc::new(AtomicU64::new(1)), running: Arc::new(AtomicU64::new(1)), batch_size, routed: AtomicU64::new(0), fallback: AtomicU64::new(0), executed: Arc::new(AtomicU64::new(0)), route_last_latency_ms: AtomicU64::new(0), batch_refills: AtomicU64::new(0) }
    }


    pub fn start(&mut self) {
        for p in &self.partitions {
            let p_clone = p.clone();
            let running = self.running.clone();
            let executed_ctr = self.executed.clone();
            let handle = thread::spawn(move || {
                while running.load(Ordering::Relaxed) == 1 {
                    let task_opt = { let mut q = p_clone.queue.lock(); q.pop_front() };
                    if let Some(task) = task_opt {
                        let start = Instant::now();
                        let metrics = task.txn.metrics(); // 提前获取 metrics 引用（在 commit 消耗 txn 前）
                        if task.txn.commit().is_ok() {
                            let dur_ms = start.elapsed().as_secs_f64() * 1000.0;
                            p_clone.last_commit_latency_ms.store((dur_ms * 1000.0) as u64, Ordering::Relaxed);
                            executed_ctr.fetch_add(1, Ordering::Relaxed);
                            if let Some(mc) = metrics { mc.record_commit_latency(start.elapsed()); mc.inc_consensus_executed(); }
                        }
                    } else {
                        p_clone.empty_polls.fetch_add(1, Ordering::Relaxed);
                        std::thread::yield_now();
                    }
                }
            });
            self.workers.push(handle);
        }
    }

    /// 尝试路由事务到单分区: 若写集合全部同一分区 -> 入队, 多分区则走 2PC，占位实现。
    pub fn route_or_commit(&self, txn: Txn) -> Result<u64, String> {
        let route_start = Instant::now();
        // 基于 Txn::partition_set 计算分区集合（与 fast_hash 一致）
        let parts = txn.partition_set(self.partitions.len());
        match parts.len() {
            0 => {
                // 无写集合 (只读) 直接同步提交
                self.fallback.fetch_add(1, Ordering::Relaxed);
                if let Some(mc) = self.store.get_metrics() { mc.inc_consensus_fallback(); mc.record_route_latency(route_start.elapsed()); }
                txn.commit()
            }
            1 => {
                // 单分区 -> 投递到对应分区执行器，分配本地批次 ts
                let pid = *parts.iter().next().unwrap() as usize;
                let part = &self.partitions[pid];
                if part.ts_next.load(Ordering::Relaxed) >= part.ts_end.load(Ordering::Relaxed) {
                    let start = self.global_ts.fetch_add(self.batch_size, Ordering::Relaxed);
                    part.ts_next.store(start, Ordering::Relaxed);
                    part.ts_end.store(start + self.batch_size, Ordering::Relaxed);
                    self.batch_refills.fetch_add(1, Ordering::Relaxed);
                }
                let assigned_ts = part.ts_next.fetch_add(1, Ordering::Relaxed);
                self.partitions[pid].push(PartitionTask { txn: txn.with_ts(assigned_ts) });
                self.routed.fetch_add(1, Ordering::Relaxed);
                let dur_ms = route_start.elapsed().as_secs_f64() * 1000.0;
                self.route_last_latency_ms.store((dur_ms * 1000.0) as u64, Ordering::Relaxed);
                if let Some(mc) = self.store.get_metrics() { mc.inc_consensus_routed(); mc.record_route_latency(route_start.elapsed()); }
                Ok(0)
            }
            _ => {
                // 多分区 -> 进入 2PC 路径（当前占位实现：同步提交并记录 prepare 指标）
                self.fallback.fetch_add(1, Ordering::Relaxed);
                if let Some(mc) = self.store.get_metrics() { mc.inc_consensus_fallback(); mc.record_route_latency(route_start.elapsed()); }
                let coord = TwoPhaseCoordinator::new(self.store.clone());
                coord.prepare_and_commit(txn)
            }
        }
    }

    pub fn routed(&self) -> u64 { self.routed.load(Ordering::Relaxed) }
    pub fn fallback_count(&self) -> u64 { self.fallback.load(Ordering::Relaxed) }
    pub fn executed(&self) -> u64 { self.executed.load(Ordering::Relaxed) }
    pub fn batch_refills(&self) -> u64 { self.batch_refills.load(Ordering::Relaxed) }
    pub fn route_last_latency_ms(&self) -> f64 { self.route_last_latency_ms.load(Ordering::Relaxed) as f64 / 1000.0 }

    /// 导出多核共识指标（Prometheus 片段）
    pub fn export_prometheus(&self) -> String {
        let mut out = String::new();
        out.push_str("# HELP multi_consensus_routed_total Total transactions routed to partition workers\n");
        out.push_str("# TYPE multi_consensus_routed_total counter\n");
        out.push_str(&format!("multi_consensus_routed_total {}\n", self.routed()));
        out.push_str("# HELP multi_consensus_fallback_total Total transactions fallback to single-thread commit\n");
        out.push_str("# TYPE multi_consensus_fallback_total counter\n");
        out.push_str(&format!("multi_consensus_fallback_total {}\n", self.fallback_count()));
        out.push_str("# HELP multi_consensus_executed_total Total routed transactions executed (committed)\n");
        out.push_str("# TYPE multi_consensus_executed_total counter\n");
        out.push_str(&format!("multi_consensus_executed_total {}\n", self.executed()));
        out.push_str("# HELP multi_consensus_route_last_latency_ms Last route_or_commit latency (ms)\n");
        out.push_str("# TYPE multi_consensus_route_last_latency_ms gauge\n");
        out.push_str(&format!("multi_consensus_route_last_latency_ms {:.3}\n", self.route_last_latency_ms()));
        out.push_str("# HELP multi_consensus_batch_refills_total Total local batch refill operations\n");
        out.push_str("# TYPE multi_consensus_batch_refills_total counter\n");
        out.push_str(&format!("multi_consensus_batch_refills_total {}\n", self.batch_refills()));
        for (idx, p) in self.partitions.iter().enumerate() {
            out.push_str("# HELP multi_consensus_partition_queue_depth Current queued tasks per partition\n");
            out.push_str("# TYPE multi_consensus_partition_queue_depth gauge\n");
            out.push_str(&format!("multi_consensus_partition_queue_depth{{partition=\"{}\"}} {}\n", idx, p.queue_len()));
            let last_commit_ms = p.last_commit_latency_ms.load(Ordering::Relaxed) as f64 / 1000.0;
            out.push_str("# HELP multi_consensus_partition_last_commit_latency_ms Last commit latency per partition (ms)\n");
            out.push_str("# TYPE multi_consensus_partition_last_commit_latency_ms gauge\n");
            out.push_str(&format!("multi_consensus_partition_last_commit_latency_ms{{partition=\"{}\"}} {:.3}\n", idx, last_commit_ms));
            out.push_str("# HELP multi_consensus_partition_empty_polls_total Empty poll loops per partition (diagnostic)\n");
            out.push_str("# TYPE multi_consensus_partition_empty_polls_total counter\n");
            out.push_str(&format!("multi_consensus_partition_empty_polls_total{{partition=\"{}\"}} {}\n", idx, p.empty_polls.load(Ordering::Relaxed)));
        }
        out
    }

    pub fn stop(mut self) { self.running.store(0, Ordering::Relaxed); for h in self.workers.drain(..) { let _ = h.join(); } }
}

impl Drop for MultiCoreConsensus {
    fn drop(&mut self) { self.running.store(0, Ordering::Relaxed); for h in self.workers.drain(..) { let _ = h.join(); } }
}

fn fast_hash(data: &[u8]) -> u64 { // 简单 FNV-1a 64
    let mut hash: u64 = 0xcbf29ce484222325; // offset basis
    for b in data { hash ^= *b as u64; hash = hash.wrapping_mul(0x100000001b3); }
    hash
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mvcc::MvccStore;

    #[test]
    fn route_single_partition() {
        let store = MvccStore::new();
        let mut mc = MultiCoreConsensus::new(store.clone(), 4, 512);
        mc.start();
        // 构造一个写单键事务 -> 单分区
        let mut txn = store.begin();
        txn.write(b"key1".to_vec(), b"v".to_vec());
        let res = mc.route_or_commit(txn).unwrap();
        assert_eq!(res, 0, "异步路由应返回 0 占位");
        mc.stop();
    }

    #[test]
    fn multi_partition_goes_2pc_and_commits() {
        let store = MvccStore::new();
        let mc = MultiCoreConsensus::new(store.clone(), 4, 512);
        // 多分区事务 -> 触发 2PC 路径 (同步提交返回实际 ts)
        let k1 = b"k1".to_vec();
        let mut k2 = b"k2".to_vec();
        // 调整 k2 直到与 k1 落在不同分区，避免依赖具体哈希常量猜测
        loop {
            let mut probe = store.begin();
            probe.write(k1.clone(), b"v1".to_vec());
            probe.write(k2.clone(), b"v2".to_vec());
            let parts = probe.partition_set(4);
            if parts.len() >= 2 { break; }
            k2.push(0xFF); // 变更 key 扰动哈希
        }
        let mut tx = store.begin();
        tx.write(k1.clone(), b"v1".to_vec());
        tx.write(k2.clone(), b"v2".to_vec());
        let res = mc.route_or_commit(tx).unwrap();
        assert!(res > 0, "多分区 2PC 路径应返回实际提交 ts");
        // 验证已提交：读取值
        let mut rtxn = store.begin_readonly();
        let v1 = rtxn.read(&k1).unwrap();
        let v2 = rtxn.read(&k2).unwrap();
        assert_eq!(v1, b"v1".to_vec());
        assert_eq!(v2, b"v2".to_vec());
    }
}