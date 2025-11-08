// SPDX-License-Identifier: GPL-3.0-or-later
// 并行证明生成模块 (Groth16)
// Phase 2.2.X: 目标 4 核 > 400 TPS (批量 prove)
// 初始骨架: 后续将接入具体 RingCT / RangeProof 电路输入类型

use std::time::{Duration, Instant};
use std::sync::Arc;
use rayon::prelude::*;
use ark_bls12_381::Bls12_381;
use ark_groth16::{Groth16, ProvingKey};
use ark_snark::SNARK;
use ark_bls12_381::Fr;
use crate::metrics::MetricsCollector;

/// 单个电路输入占位结构 (后续替换为真实 RingCT 交易上下文)
#[derive(Clone)]
pub struct CircuitInput {
    pub a: Fr,
    pub b: Fr,
}

/// 并行证明配置
#[derive(Clone, Debug)]
pub struct ParallelProveConfig {
    /// 批量大小 (一次提交多少证明任务)
    pub batch_size: usize,
    /// 自定义线程池大小 (None 使用全局默认)
    pub num_threads: Option<usize>,
    /// 是否收集单个证明耗时
    pub collect_individual_latency: bool,
}

impl Default for ParallelProveConfig {
    fn default() -> Self {
        Self {
            batch_size: 32,
            num_threads: None,
            collect_individual_latency: true,
        }
    }
}

/// 并行证明统计结果
#[derive(Clone, Debug)]
pub struct ParallelProofStats {
    pub total: usize,
    pub ok: usize,
    pub failed: usize,
    pub total_duration: Duration,
    pub avg_latency_ms: f64,
    pub tps: f64,
    pub per_proof_latency_ms: Vec<f64>,
}

/// 并行 prover
pub struct ParallelProver {
    pk: Arc<ProvingKey<Bls12_381>>,
    config: ParallelProveConfig,
    metrics: Option<Arc<MetricsCollector>>,
}

impl ParallelProver {
    pub fn new(pk: ProvingKey<Bls12_381>, config: ParallelProveConfig) -> Self {
        Self { pk: Arc::new(pk), config, metrics: None }
    }

    pub fn with_metrics(mut self, metrics: Arc<MetricsCollector>) -> Self {
        self.metrics = Some(metrics);
        self
    }

    /// 批量生成证明 (占位实现: 使用 MultiplyCircuit 风格的简单电路)
    pub fn prove_batch(&self, inputs: &[CircuitInput]) -> ParallelProofStats {
        let start = Instant::now();
        let mut per_latency = Vec::with_capacity(inputs.len());

        // 可选自定义线程池
        let pool_opt = self.config.num_threads.map(|n| rayon::ThreadPoolBuilder::new().num_threads(n).build().expect("threadpool build"));
        let pk_ref = self.pk.clone();

        // 闭包: 生成单个证明 (后续替换为实际 RingCT 电路)
        let prove_one = |inp: &CircuitInput| -> (bool, Duration) {
            let local_start = Instant::now();
            // 占位: 生成简单乘法电路证明
            use zk_groth16_test::MultiplyCircuit;
            let circuit = MultiplyCircuit { a: Some(inp.a), b: Some(inp.b) };
            let rng = &mut rand::rngs::OsRng;
            let res = Groth16::<Bls12_381>::prove(&pk_ref, circuit, rng);
            let dur = local_start.elapsed();
            (res.is_ok(), dur)
        };

        let results: Vec<(bool, Duration)> = if let Some(pool) = pool_opt.as_ref() {
            pool.install(|| {
                inputs.par_iter().map(prove_one).collect()
            })
        } else {
            inputs.par_iter().map(prove_one).collect()
        };

        let total_duration = start.elapsed();
        let mut ok = 0usize;
        let mut failed = 0usize;
        for (succ, dur) in results.into_iter() {
            if succ { ok += 1; } else { failed += 1; }
            if self.config.collect_individual_latency {
                per_latency.push(dur.as_secs_f64() * 1000.0);
            }
        }
        let total = ok + failed;
        let avg_latency_ms = if ok > 0 { (total_duration.as_secs_f64() * 1000.0) / ok as f64 } else { 0.0 };
        // tps 基于成功证明数除以总耗时
        let tps = if total_duration.as_secs_f64() > 0.0 { ok as f64 / total_duration.as_secs_f64() } else { 0.0 };

        let stats = ParallelProofStats {
            total,
            ok,
            failed,
            total_duration,
            avg_latency_ms,
            tps,
            per_proof_latency_ms: per_latency,
        };

        // 记录 metrics
        if let Some(m) = &self.metrics {
            m.record_parallel_batch(
                stats.total as u64,
                stats.failed as u64,
                stats.total_duration.as_secs_f64() * 1000.0,
                stats.avg_latency_ms,
                stats.tps,
            );
        }

        stats
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ark_groth16::Groth16;
    use ark_bls12_381::Bls12_381;
    use zk_groth16_test::MultiplyCircuit;

    #[test]
    fn test_parallel_prover_basic() {
        let rng = &mut rand::rngs::OsRng;
        let params = Groth16::<Bls12_381>::generate_random_parameters_with_reduction(MultiplyCircuit { a: None, b: None }, rng).expect("setup fail");
        let metrics = Arc::new(MetricsCollector::new());
        let prover = ParallelProver::new(params, ParallelProveConfig::default()).with_metrics(metrics.clone());
        let inputs: Vec<CircuitInput> = (0..prover.config.batch_size).map(|i| CircuitInput { a: Fr::from((i+1) as u64), b: Fr::from(3u64) }).collect();
        let stats = prover.prove_batch(&inputs);
        assert_eq!(stats.total, prover.config.batch_size);
        assert_eq!(stats.failed, 0);
        assert!(stats.tps > 0.0);
        // metrics 应已记录一批
        assert_eq!(metrics.parallel_proof_batches.load(std::sync::atomic::Ordering::Relaxed), 1);
    }
}
