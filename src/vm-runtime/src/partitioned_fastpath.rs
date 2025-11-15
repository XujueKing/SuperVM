// SPDX-License-Identifier: GPL-3.0-or-later
// PartitionedFastPath Executor (prototype revised)
// Feature: partitioned-fastpath
// 目标: 按 CPU 核数分区执行, 减少共享结构争用。当前原型仅使用全局 Injector + 独立本地队列, 后续可分区化状态访问。

#![allow(dead_code)]

use std::sync::{Arc, atomic::{AtomicU64, Ordering}};
use std::thread::{self, JoinHandle};
use crossbeam_deque::{Injector, Worker, Steal};

/// 交易任务原型：模拟执行耗时
pub struct FastTask {
    pub key: Vec<u8>,            // 未来用于分区哈希 (当前未使用, 简化实现)
    pub simulated_cycles: u32,   // 模拟执行步数
}

impl FastTask { pub fn new(key: Vec<u8>, simulated_cycles: u32) -> Self { Self { key, simulated_cycles } } }

struct PartitionThread { executed: AtomicU64 }

pub struct PartitionedFastPath {
    injector: Arc<Injector<FastTask>>, // Arc 以便在线程中安全共享
    num_partitions: usize,
    threads: Vec<JoinHandle<()>>,
    stats: Vec<Arc<PartitionThread>>,
    running: Arc<AtomicU64>,
    submitted: AtomicU64,
}

impl PartitionedFastPath {
    pub fn new(num_partitions: usize) -> Self {
        let stats = (0..num_partitions).map(|_| Arc::new(PartitionThread { executed: AtomicU64::new(0) })).collect::<Vec<_>>();
    Self { injector: Arc::new(Injector::new()), num_partitions, threads: Vec::new(), stats, running: Arc::new(AtomicU64::new(1)), submitted: AtomicU64::new(0) }
    }

    pub fn submit(&self, task: FastTask) { self.injector.push(task); self.submitted.fetch_add(1, Ordering::Relaxed); }

    pub fn spawn_workers(&mut self) {
        for idx in 0..self.num_partitions {
            let injector_ref = self.injector.clone();
            let running = self.running.clone();
            let stat = self.stats[idx].clone();
            // 每个线程拥有自己的本地 Worker
            let local = Worker::new_fifo();
            let handle = thread::spawn(move || {
                while running.load(Ordering::Relaxed) == 1 {
                    match local.pop() {
                        Some(task) => { exec_task(task); stat.executed.fetch_add(1, Ordering::Relaxed); }
                        None => match injector_ref.steal_batch_and_pop(&local) {
                            Steal::Success(task) => { exec_task(task); stat.executed.fetch_add(1, Ordering::Relaxed); }
                            Steal::Empty => { thread::yield_now(); }
                            Steal::Retry => {}
                        }
                    }
                }
                // Drain remaining tasks when stopping
                loop { match local.pop() { Some(task) => { exec_task(task); stat.executed.fetch_add(1, Ordering::Relaxed); } None => break } }
            });
            self.threads.push(handle);
        }
    }

    pub fn stop(mut self) { self.running.store(0, Ordering::Relaxed); for h in self.threads.drain(..) { let _ = h.join(); } }

    pub fn executed_per_partition(&self) -> Vec<u64> { self.stats.iter().map(|s| s.executed.load(Ordering::Relaxed)).collect() }
    pub fn total_executed(&self) -> u64 { self.executed_per_partition().into_iter().sum() }
    pub fn submitted(&self) -> u64 { self.submitted.load(Ordering::Relaxed) }
}

fn exec_task(task: FastTask) {
    let mut acc: u64 = 0;
    for _ in 0..task.simulated_cycles { acc = acc.wrapping_add(1); }
    if acc == 0xFFFF_FFFF { std::hint::black_box(acc); }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn basic_partition_exec() {
        let mut exec = PartitionedFastPath::new(4);
        exec.spawn_workers();
    for i in 0u64..10_000 { exec.submit(FastTask::new(i.to_be_bytes().to_vec(), 32)); }
        // 等待全部提交执行完 (带超时防止卡死)
        let start = std::time::Instant::now();
        while exec.total_executed() < exec.submitted() && start.elapsed().as_secs_f32() < 5.0 {
            std::thread::yield_now();
        }
        let total = exec.total_executed();
        assert!(total >= 9_000, "executed={} too small", total);
        exec.stop();
    }
}
