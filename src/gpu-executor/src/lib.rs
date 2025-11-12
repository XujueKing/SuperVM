//! Phase 13: GPU-Executor & HybridScheduler skeleton
//! Goals:
//! - Provide stable traits for CPU/GPU hybrid execution.
//! - Buildable on machines without a GPU (CPU fallback).
//! - Zero external dependencies for default `cpu` feature.
//! - Optional `gpu` feature adds wgpu-based compute shader backend.

#![forbid(unsafe_code)]
#![deny(warnings)]

use std::fmt::Debug;
use std::time::Duration;

#[cfg(feature = "gpu")]
pub mod gpu_backend;

#[cfg(feature = "gpu")]
pub use gpu_backend::{WgpuExecutor, BufferPool, create_wgpu_executor};

/// Minimal error type for executor operations.
#[derive(Debug, Clone)]
pub struct ExecError {
    pub kind: ExecErrorKind,
    pub message: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExecErrorKind {
    ResourceBusy,
    Timeout,
    InvalidTask,
    BackendUnavailable,
    Internal,
}

impl ExecError {
    pub fn new(kind: ExecErrorKind, message: impl Into<String>) -> Self {
        Self { kind, message: message.into() }
    }
}

/// A portable task descriptor; callers can embed their own payload via generics.
#[derive(Debug, Clone)]
pub struct Task<T> {
    pub id: u64,
    pub payload: T,
    pub est_cost: u64, // abstract cost units for scheduling decisions
}

#[derive(Debug, Clone)]
pub struct Batch<T> {
    pub tasks: Vec<Task<T>>,
}

#[derive(Debug, Clone)]
pub struct ExecStats {
    pub device: DeviceKind,
    pub duration: Duration,
    pub tasks: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeviceKind { Cpu, Gpu }

/// Unified execution result per task.
#[derive(Debug, Clone)]
pub struct TaskResult<R> {
    pub id: u64,
    pub output: R,
}

/// Trait for concrete GPU backends (or CPU fallback) to execute a batch.
pub trait GpuExecutor<T, R> {
    /// Execute a batch on a specific device, returning per-task results and stats.
    fn execute(&mut self, batch: &Batch<T>) -> Result<(Vec<TaskResult<R>>, ExecStats), ExecError>;

    /// Whether this executor is currently available and healthy.
    fn is_available(&self) -> bool { true }

    /// The device kind represented by this executor.
    fn device_kind(&self) -> DeviceKind;
}

/// Simple strategy knobs for hybrid scheduling.
#[derive(Debug, Clone)]
pub struct HybridStrategy {
    pub gpu_threshold: usize, // if batch size >= threshold, prefer GPU
    pub max_cpu_parallelism: usize,
    pub adaptive_enabled: bool, // 是否开启动态阈值调整
    pub min_gpu_threshold: usize, // GPU 阈值下限
    pub max_gpu_threshold: usize, // GPU 阈值上限
    pub adjust_step: usize,       // 每次调整的步长
}

impl Default for HybridStrategy {
    fn default() -> Self {
        Self {
            gpu_threshold: 32,
            max_cpu_parallelism: 1,
            adaptive_enabled: true,
            min_gpu_threshold: 8,
            max_gpu_threshold: 512,
            adjust_step: 8,
        }
    }
}

/// A minimal HybridScheduler that chooses between CPU and GPU executors.
pub struct HybridScheduler<Cpu, Gpu, T, R>
where
    Cpu: GpuExecutor<T, R>,
    Gpu: GpuExecutor<T, R>,
{
    cpu: Cpu,
    gpu: Option<Gpu>,
    strategy: HybridStrategy,
    _phantom_t: std::marker::PhantomData<T>,
    _phantom_r: std::marker::PhantomData<R>,
    // 运行时统计，用于动态调节策略
    recent_cpu_time: Duration,
    recent_gpu_time: Duration,
    recent_cpu_batches: usize,
    recent_gpu_batches: usize,
    gpu_busy_events: usize,
}

impl<Cpu, Gpu, T, R> HybridScheduler<Cpu, Gpu, T, R>
where
    Cpu: GpuExecutor<T, R>,
    Gpu: GpuExecutor<T, R>,
{
    pub fn new(cpu: Cpu, gpu: Option<Gpu>, strategy: HybridStrategy) -> Self {
        Self {
            cpu,
            gpu,
            strategy,
            _phantom_t: Default::default(),
            _phantom_r: Default::default(),
            recent_cpu_time: Duration::ZERO,
            recent_gpu_time: Duration::ZERO,
            recent_cpu_batches: 0,
            recent_gpu_batches: 0,
            gpu_busy_events: 0,
        }
    }

    pub fn schedule(&mut self, batch: &Batch<T>) -> Result<(Vec<TaskResult<R>>, ExecStats), ExecError> {
        // 决策：先决定目标设备
        let target = self.decide_route(batch);
        if matches!(target, DeviceKind::Gpu) {
            if let Some(gpu) = self.gpu.as_mut() {
                if gpu.is_available() {
                    match gpu.execute(batch) {
                        Ok((res, mut stats)) => {
                            self.recent_gpu_time += stats.duration;
                            self.recent_gpu_batches += 1;
                            self.maybe_adjust_gpu_threshold();
                            stats.device = DeviceKind::Gpu;
                            return Ok((res, stats));
                        }
                        Err(e) if matches!(e.kind, ExecErrorKind::ResourceBusy) => {
                            // 记录忙事件并升高阈值，回退到 CPU
                            self.gpu_busy_events += 1;
                            if self.strategy.gpu_threshold < self.strategy.max_gpu_threshold {
                                self.strategy.gpu_threshold = (self.strategy.gpu_threshold + self.strategy.adjust_step)
                                    .min(self.strategy.max_gpu_threshold);
                            }
                        }
                        Err(e) if matches!(e.kind, ExecErrorKind::BackendUnavailable) => {
                            // 后端不可用，后续可能恢复；不调整 busy
                        }
                        Err(e) => return Err(e),
                    }
                }
            }
        }
        // CPU 执行（默认或回退）
        let r = self.cpu.execute(batch);
        if let Ok((_, stats)) = &r {
            self.recent_cpu_time += stats.duration;
            self.recent_cpu_batches += 1;
            self.maybe_adjust_gpu_threshold();
        }
        r
    }

    fn maybe_adjust_gpu_threshold(&mut self) {
        if !self.strategy.adaptive_enabled { return; }
        // 简单策略：比较平均耗时比例，如果 GPU 更快且批次经常受限，则降低阈值；如果 GPU 变慢则升高阈值
        let avg_gpu = if self.recent_gpu_batches > 0 { self.recent_gpu_time / self.recent_gpu_batches as u32 } else { Duration::MAX };
        let avg_cpu = if self.recent_cpu_batches > 0 { self.recent_cpu_time / self.recent_cpu_batches as u32 } else { Duration::MAX };
        // 如果 GPU 平均耗时明显低于 CPU（< 70%），尝试降低阈值；反之提高
        if avg_gpu < avg_cpu.mul_f32(0.7) {
            if self.strategy.gpu_threshold > self.strategy.min_gpu_threshold {
                self.strategy.gpu_threshold = self.strategy.gpu_threshold.saturating_sub(self.strategy.adjust_step).max(self.strategy.min_gpu_threshold);
            }
        } else if avg_cpu < avg_gpu.mul_f32(0.85) {
            if self.strategy.gpu_threshold < self.strategy.max_gpu_threshold {
                self.strategy.gpu_threshold = (self.strategy.gpu_threshold + self.strategy.adjust_step).min(self.strategy.max_gpu_threshold);
            }
        }
        // 如果近期 GPU 频繁忙，则保守升阈（避免过度选择 GPU）
        if self.gpu_busy_events > 3 {
            self.gpu_busy_events = 0; // 衰减
            if self.strategy.gpu_threshold < self.strategy.max_gpu_threshold {
                self.strategy.gpu_threshold = (self.strategy.gpu_threshold + self.strategy.adjust_step).min(self.strategy.max_gpu_threshold);
            }
        }
    }

    /// 依据 batch 大小、任务估算成本与历史耗时，给出当前批次的优先设备选择
    pub fn decide_route(&self, batch: &Batch<T>) -> DeviceKind {
        // 无 GPU 直接返回 CPU
        if self.gpu.is_none() { return DeviceKind::Cpu; }
        let gpu_available = self.gpu.as_ref().map(|g| g.is_available()).unwrap_or(false);
        if !gpu_available { return DeviceKind::Cpu; }

        let n = batch.tasks.len();
        if n == 0 { return DeviceKind::Cpu; }
        let total_cost: u64 = batch.tasks.iter().map(|t| t.est_cost).sum();
        let avg_cost = total_cost as f64 / n as f64;

        // 基线：阈值判断
        let mut prefer_gpu = n >= self.strategy.gpu_threshold;

        // 轻/重任务启发：重任务更倾向 GPU，轻任务更倾向 CPU
        const LOW_COST: f64 = 2.0;
        const HIGH_COST: f64 = 100.0;
        if avg_cost <= LOW_COST { prefer_gpu = false; }
        if avg_cost >= HIGH_COST { prefer_gpu = true; }

        // 历史耗时：若有统计，用相对快的设备
        if self.recent_cpu_batches > 0 && self.recent_gpu_batches > 0 {
            let avg_gpu = self.recent_gpu_time / self.recent_gpu_batches as u32;
            let avg_cpu = self.recent_cpu_time / self.recent_cpu_batches as u32;
            if avg_gpu < avg_cpu.mul_f32(0.8) { prefer_gpu = true; }
            if avg_cpu < avg_gpu.mul_f32(0.8) { prefer_gpu = false; }
        }
        if prefer_gpu { DeviceKind::Gpu } else { DeviceKind::Cpu }
    }

    // ---- 配置与观测接口 ----
    pub fn strategy(&self) -> &HybridStrategy { &self.strategy }
    pub fn strategy_mut(&mut self) -> &mut HybridStrategy { &mut self.strategy }
    pub fn set_strategy(&mut self, s: HybridStrategy) { self.strategy = s; }
    pub fn stats_snapshot(&self) -> SchedulerStats {
        SchedulerStats {
            recent_cpu_batches: self.recent_cpu_batches,
            recent_gpu_batches: self.recent_gpu_batches,
            recent_cpu_time: self.recent_cpu_time,
            recent_gpu_time: self.recent_gpu_time,
            gpu_busy_events: self.gpu_busy_events,
            gpu_threshold: self.strategy.gpu_threshold,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct SchedulerStats {
    pub recent_cpu_batches: usize,
    pub recent_gpu_batches: usize,
    pub recent_cpu_time: Duration,
    pub recent_gpu_time: Duration,
    pub gpu_busy_events: usize,
    pub gpu_threshold: usize,
}

/// A trivial CPU fallback executor that maps payload via a pure function.
#[cfg(feature = "cpu")]
pub struct CpuMapExecutor<F> {
    map: F,
}

#[cfg(feature = "cpu")]
impl<F> CpuMapExecutor<F> {
    pub fn new(map: F) -> Self { Self { map } }
}

#[cfg(feature = "cpu")]
impl<T, R, F> GpuExecutor<T, R> for CpuMapExecutor<F>
where
    F: Fn(&T) -> R,
{
    fn execute(&mut self, batch: &Batch<T>) -> Result<(Vec<TaskResult<R>>, ExecStats), ExecError> {
        let start = std::time::Instant::now();
        let mut out = Vec::with_capacity(batch.tasks.len());
        for t in &batch.tasks {
            let r = (self.map)(&t.payload);
            out.push(TaskResult { id: t.id, output: r });
        }
        let stats = ExecStats { device: DeviceKind::Cpu, duration: start.elapsed(), tasks: out.len() };
        Ok((out, stats))
    }

    fn device_kind(&self) -> DeviceKind { DeviceKind::Cpu }
}

/// Placeholder GPU executor that always reports unavailable unless `gpu` feature in future adds a real backend.
pub struct UnavailableGpu;

impl<T, R> GpuExecutor<T, R> for UnavailableGpu {
    fn execute(&mut self, _batch: &Batch<T>) -> Result<(Vec<TaskResult<R>>, ExecStats), ExecError> {
        Err(ExecError::new(ExecErrorKind::BackendUnavailable, "GPU backend not available"))
    }

    fn is_available(&self) -> bool { false }

    fn device_kind(&self) -> DeviceKind { DeviceKind::Gpu }
}

// ---------------- Parallel CPU Executor (Rayon) ----------------
// Feature `parallel` adds a data-parallel map implementation using rayon.
// Provides higher throughput for large batches; falls back to sequential if batch small.
#[cfg(feature = "parallel")]
pub struct ParallelCpuExecutor<F> {
    map: F,
    adaptive_min_parallel: usize, // 小于该值时直接走顺序，避免线程池开销
}

#[cfg(feature = "parallel")]
impl<F> ParallelCpuExecutor<F> {
    pub fn new(map: F) -> Self { Self { map, adaptive_min_parallel: 4 } }
    pub fn with_adaptive(mut self, min_parallel: usize) -> Self { self.adaptive_min_parallel = min_parallel; self }
}

#[cfg(feature = "parallel")]
impl<T, R, F> GpuExecutor<T, R> for ParallelCpuExecutor<F>
where
    F: Fn(&T) -> R + Sync + Send,
    T: Sync,
    R: Send,
{
    fn execute(&mut self, batch: &Batch<T>) -> Result<(Vec<TaskResult<R>>, ExecStats), ExecError> {
        let start = std::time::Instant::now();
        let out: Vec<TaskResult<R>> = if batch.tasks.len() < self.adaptive_min_parallel {
            // 顺序执行，避免 rayon 调度开销
            batch.tasks.iter().map(|t| {
                let r = (self.map)(&t.payload);
                TaskResult { id: t.id, output: r }
            }).collect()
        } else {
            use rayon::prelude::*;
            batch.tasks.par_iter().map(|t| {
                let r = (self.map)(&t.payload);
                TaskResult { id: t.id, output: r }
            }).collect()
        };
        let stats = ExecStats { device: DeviceKind::Cpu, duration: start.elapsed(), tasks: out.len() };
        Ok((out, stats))
    }

    fn device_kind(&self) -> DeviceKind { DeviceKind::Cpu }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cpu_fallback_executes() {
        let cpu = CpuMapExecutor::new(|x: &u32| x + 1);
        let mut scheduler = HybridScheduler::new(cpu, Some(UnavailableGpu), HybridStrategy::default());
        let batch = Batch { tasks: vec![
            Task { id: 1, payload: 41u32, est_cost: 1 },
            Task { id: 2, payload: 1u32, est_cost: 1 },
        ]};
        let (res, stats) = scheduler.schedule(&batch).expect("cpu fallback works");
        assert_eq!(res.len(), 2);
        assert_eq!(res[0].id, 1);
        assert_eq!(res[0].output, 42);
        assert_eq!(stats.device, DeviceKind::Cpu);
    }

    #[test]
    fn gpu_path_attempts_then_falls_back_on_unavailable() {
        let cpu = CpuMapExecutor::new(|x: &u32| x * 2);
    let mut scheduler = HybridScheduler::new(cpu, Some(UnavailableGpu), HybridStrategy { gpu_threshold: 1, max_cpu_parallelism: 1, adaptive_enabled: false, min_gpu_threshold: 8, max_gpu_threshold: 512, adjust_step: 8 });
        let batch = Batch { tasks: vec![ Task { id: 7, payload: 3u32, est_cost: 1 } ] };
        let (res, stats) = scheduler.schedule(&batch).expect("fallback works");
        assert_eq!(res[0].output, 6);
        assert_eq!(stats.device, DeviceKind::Cpu);
    }

    #[cfg(feature = "parallel")]
    #[test]
    fn parallel_executor_produces_same_results() {
        let mut exec = ParallelCpuExecutor::new(|x: &u64| x * 3);
        let batch = Batch { tasks: (0..64).map(|i| Task { id: i as u64, payload: i as u64, est_cost: 1 }).collect() };
        let (res_parallel, stats_parallel) = exec.execute(&batch).expect("parallel ok");

        // sequential for comparison
        let mut seq = CpuMapExecutor::new(|x: &u64| x * 3);
        let (res_seq, _stats_seq) = seq.execute(&batch).expect("seq ok");

        assert_eq!(res_parallel.len(), res_seq.len());
        for (a, b) in res_parallel.iter().zip(res_seq.iter()) {
            assert_eq!(a.id, b.id);
            assert_eq!(a.output, b.output);
        }
        assert!(stats_parallel.duration <= Duration::from_secs(1)); // sanity upper bound
    }

    #[test]
    fn adaptive_threshold_moves() {
        let cpu = CpuMapExecutor::new(|x: &u32| x + 1);
        let mut scheduler = HybridScheduler::new(cpu, Some(UnavailableGpu), HybridStrategy::default());
        // 因为 GPU 不可用，始终走 CPU；avg_cpu < avg_gpu (MAX) 会导致阈值上升直到上限
        let original = scheduler.strategy.gpu_threshold;
        for i in 0..10 {
            let batch = Batch { tasks: (0..16).map(|j| Task { id: (i*100 + j) as u64, payload: j as u32, est_cost: 1 }).collect() };
            let _ = scheduler.schedule(&batch).unwrap();
        }
        assert!(scheduler.strategy.gpu_threshold >= original); // 阈值不下降
    }
}
