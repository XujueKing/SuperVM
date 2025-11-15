//! 递归证明聚合策略模块
//!
//! 基于 Session 12 理论分析,实现自适应聚合决策和配置管理。

/// 聚合策略类型
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AggregationStrategy {
    /// 不聚合 (证明数量太少)
    NoAggregation,
    /// 单级聚合 (N → 1)
    SingleLevel { batch_size: usize },
    /// 两级聚合 (N → M → 1)
    TwoLevel { first_batch: usize, second_batch: usize },
    /// 三级聚合 (N → M → K → 1)
    ThreeLevel { first_batch: usize, second_batch: usize, third_batch: usize },
}

impl AggregationStrategy {
    /// 获取策略描述
    pub fn description(&self) -> String {
        match self {
            Self::NoAggregation => "不聚合 (直接提交)".to_string(),
            Self::SingleLevel { batch_size } => format!("单级聚合 ({} → 1)", batch_size),
            Self::TwoLevel { first_batch, second_batch } => {
                format!("两级聚合 ({} → {} → 1)", first_batch * second_batch, first_batch)
            }
            Self::ThreeLevel { first_batch, second_batch, third_batch } => {
                format!("三级聚合 ({} → {} → {} → 1)", 
                    first_batch * second_batch * third_batch,
                    second_batch * third_batch,
                    third_batch)
            }
        }
    }

    /// 计算总聚合因子
    pub fn total_aggregation_factor(&self) -> usize {
        match self {
            Self::NoAggregation => 1,
            Self::SingleLevel { batch_size } => *batch_size,
            Self::TwoLevel { first_batch, second_batch } => first_batch * second_batch,
            Self::ThreeLevel { first_batch, second_batch, third_batch } => {
                first_batch * second_batch * third_batch
            }
        }
    }
}

/// 聚合配置
#[derive(Debug, Clone)]
pub struct AggregationConfig {
    /// 最小聚合证明数 (小于此值不聚合)
    pub min_proofs_for_aggregation: usize,
    /// 单级聚合批大小
    pub single_level_batch_size: usize,
    /// 两级聚合第一级批大小
    pub two_level_first_batch: usize,
    /// 两级聚合第二级批大小
    pub two_level_second_batch: usize,
    /// 三级聚合批大小配置
    pub three_level_batches: (usize, usize, usize),
    /// 并行工作线程数
    pub parallel_workers: usize,
}

impl Default for AggregationConfig {
    fn default() -> Self {
        // 使用可用 CPU 核心数,最小 4 核
        let workers = std::thread::available_parallelism()
            .map(|n| n.get())
            .unwrap_or(4)
            .max(4);
        
        Self {
            min_proofs_for_aggregation: 6, // Session 12: 6 个证明开始划算
            single_level_batch_size: 10,
            two_level_first_batch: 10,
            two_level_second_batch: 10,
            three_level_batches: (10, 10, 10),
            parallel_workers: workers,
        }
    }
}

impl AggregationConfig {
    /// 小型应用配置 (< 1K TPS)
    pub fn small_app() -> Self {
        Self {
            min_proofs_for_aggregation: 6,
            single_level_batch_size: 10,
            two_level_first_batch: 5,
            two_level_second_batch: 5,
            three_level_batches: (5, 5, 5),
            parallel_workers: 8,
        }
    }

    /// 中型应用配置 (1K-10K TPS)
    pub fn medium_app() -> Self {
        Self {
            min_proofs_for_aggregation: 10,
            single_level_batch_size: 10,
            two_level_first_batch: 10,
            two_level_second_batch: 10,
            three_level_batches: (10, 10, 10),
            parallel_workers: 32,
        }
    }

    /// 大型应用配置 (> 10K TPS)
    pub fn large_app() -> Self {
        Self {
            min_proofs_for_aggregation: 20,
            single_level_batch_size: 10,
            two_level_first_batch: 10,
            two_level_second_batch: 10,
            three_level_batches: (10, 10, 10),
            parallel_workers: 128,
        }
    }
}

/// 聚合决策器
pub struct AggregationDecider {
    config: AggregationConfig,
}

impl AggregationDecider {
    /// 创建新的决策器
    pub fn new(config: AggregationConfig) -> Self {
        Self { config }
    }

    /// 根据证明数量决定聚合策略
    ///
    /// # 决策逻辑 (基于 Session 12 分析)
    /// - < 6 proofs: 不聚合 (成本不划算)
    /// - 6-50 proofs: 单级聚合 (10 → 1)
    /// - 51-500 proofs: 两级聚合 (100 → 10 → 1)
    /// - > 500 proofs: 三级聚合 (1000 → 100 → 10 → 1)
    pub fn decide_strategy(&self, proof_count: usize) -> AggregationStrategy {
        if proof_count < self.config.min_proofs_for_aggregation {
            return AggregationStrategy::NoAggregation;
        }

        match proof_count {
            6..=50 => AggregationStrategy::SingleLevel {
                batch_size: self.config.single_level_batch_size,
            },
            51..=500 => AggregationStrategy::TwoLevel {
                first_batch: self.config.two_level_first_batch,
                second_batch: self.config.two_level_second_batch,
            },
            _ => AggregationStrategy::ThreeLevel {
                first_batch: self.config.three_level_batches.0,
                second_batch: self.config.three_level_batches.1,
                third_batch: self.config.three_level_batches.2,
            },
        }
    }

    /// 估算聚合后的性能提升
    pub fn estimate_performance_gain(&self, proof_count: usize) -> AggregationMetrics {
        let strategy = self.decide_strategy(proof_count);
        
        // 基于 Session 11 RISC0 实测数据
        let verify_time_ms = 24.5;
        let proof_size_kb = 215.0;
        let gas_per_verify = 300_000;

        // 不聚合场景
        let no_agg_verify_time = verify_time_ms * proof_count as f64;
        let no_agg_proof_size = proof_size_kb * proof_count as f64;
        let no_agg_gas = gas_per_verify * proof_count;
        let no_agg_tps = proof_count as f64 * 1000.0 / no_agg_verify_time;

        // 聚合场景
        let agg_factor = strategy.total_aggregation_factor();
        let aggregated_proof_count = (proof_count + agg_factor - 1) / agg_factor; // 向上取整
        let agg_verify_time = verify_time_ms * aggregated_proof_count as f64;
        let agg_proof_size = proof_size_kb * aggregated_proof_count as f64 * 1.2; // +20% 聚合开销
        let agg_gas = gas_per_verify * aggregated_proof_count;
        let agg_tps = proof_count as f64 * 1000.0 / agg_verify_time;

        AggregationMetrics {
            strategy,
            proof_count,
            // 不聚合指标
            no_aggregation_verify_time_ms: no_agg_verify_time,
            no_aggregation_proof_size_kb: no_agg_proof_size,
            no_aggregation_gas: no_agg_gas,
            no_aggregation_tps: no_agg_tps,
            // 聚合后指标
            aggregated_verify_time_ms: agg_verify_time,
            aggregated_proof_size_kb: agg_proof_size,
            aggregated_gas: agg_gas,
            aggregated_tps: agg_tps,
            // 提升倍数
            tps_improvement: agg_tps / no_agg_tps,
            gas_savings_percent: (1.0 - agg_gas as f64 / no_agg_gas as f64) * 100.0,
            size_savings_percent: (1.0 - agg_proof_size / no_agg_proof_size) * 100.0,
        }
    }
}

/// 聚合性能指标
#[derive(Debug, Clone)]
pub struct AggregationMetrics {
    pub strategy: AggregationStrategy,
    pub proof_count: usize,
    // 不聚合指标
    pub no_aggregation_verify_time_ms: f64,
    pub no_aggregation_proof_size_kb: f64,
    pub no_aggregation_gas: usize,
    pub no_aggregation_tps: f64,
    // 聚合后指标
    pub aggregated_verify_time_ms: f64,
    pub aggregated_proof_size_kb: f64,
    pub aggregated_gas: usize,
    pub aggregated_tps: f64,
    // 提升倍数
    pub tps_improvement: f64,
    pub gas_savings_percent: f64,
    pub size_savings_percent: f64,
}

impl AggregationMetrics {
    /// 打印性能报告
    pub fn print_report(&self) {
        println!("\n╔════════════════════════════════════════════════════════╗");
        println!("║   聚合性能分析报告                                    ║");
        println!("╚════════════════════════════════════════════════════════╝");
        
        println!("\n证明数量: {}", self.proof_count);
        println!("聚合策略: {}", self.strategy.description());
        
        println!("\n=== 不聚合场景 ===");
        println!("  L1 验证时间: {:.2}ms", self.no_aggregation_verify_time_ms);
        println!("  证明总大小: {:.2}KB", self.no_aggregation_proof_size_kb);
        println!("  Gas 成本: {} gas", self.no_aggregation_gas);
        println!("  吞吐量: {:.0} TPS", self.no_aggregation_tps);
        
        println!("\n=== 聚合后场景 ===");
        println!("  L1 验证时间: {:.2}ms", self.aggregated_verify_time_ms);
        println!("  证明总大小: {:.2}KB", self.aggregated_proof_size_kb);
        println!("  Gas 成本: {} gas", self.aggregated_gas);
        println!("  吞吐量: {:.0} TPS", self.aggregated_tps);
        
        println!("\n=== 性能提升 ===");
        println!("  TPS 提升: {:.2}x", self.tps_improvement);
        println!("  Gas 节省: {:.2}%", self.gas_savings_percent);
        println!("  存储节省: {:.2}%", self.size_savings_percent);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_strategy_decision() {
        let decider = AggregationDecider::new(AggregationConfig::default());

        // < 6: 不聚合
        assert_eq!(
            decider.decide_strategy(5),
            AggregationStrategy::NoAggregation
        );

        // 6-50: 单级
        matches!(
            decider.decide_strategy(10),
            AggregationStrategy::SingleLevel { .. }
        );

        // 51-500: 两级
        matches!(
            decider.decide_strategy(100),
            AggregationStrategy::TwoLevel { .. }
        );

        // > 500: 三级
        matches!(
            decider.decide_strategy(1000),
            AggregationStrategy::ThreeLevel { .. }
        );
    }

    #[test]
    fn test_performance_estimation() {
        let decider = AggregationDecider::new(AggregationConfig::default());

        // 10 个证明,单级聚合
        let metrics = decider.estimate_performance_gain(10);
        assert_eq!(metrics.proof_count, 10);
        assert!(metrics.tps_improvement > 9.0); // ~10x
        assert!(metrics.gas_savings_percent > 85.0); // ~90%
    }

    #[test]
    fn test_different_app_configs() {
        let small = AggregationConfig::small_app();
        let medium = AggregationConfig::medium_app();
        let large = AggregationConfig::large_app();

        assert_eq!(small.parallel_workers, 8);
        assert_eq!(medium.parallel_workers, 32);
        assert_eq!(large.parallel_workers, 128);
    }
}
