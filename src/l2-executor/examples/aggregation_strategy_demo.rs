// Session 13: 聚合策略演示
//
// 展示自适应聚合决策和性能预测

use l2_executor::aggregation::{AggregationConfig, AggregationDecider};

fn main() {
    println!("╔══════════════════════════════════════════════════════════╗");
    println!("║  SuperVM L2 Executor - Session 13                       ║");
    println!("║  自适应聚合策略演示                                     ║");
    println!("╚══════════════════════════════════════════════════════════╝");

    // 测试不同规模的证明批次
    let test_cases = vec![
        ("极小批次", 3),
        ("小批次 (阈值边缘)", 6),
        ("中等批次", 25),
        ("大批次", 150),
        ("超大批次", 800),
    ];

    println!("\n=== 默认配置决策 ===\n");
    let decider = AggregationDecider::new(AggregationConfig::default());

    for (name, proof_count) in &test_cases {
        let strategy = decider.decide_strategy(*proof_count);
        println!("{} ({} proofs):", name, proof_count);
        println!("  策略: {}", strategy.description());
        println!("  聚合因子: {}x\n", strategy.total_aggregation_factor());
    }

    // 性能估算示例
    println!("\n╔══════════════════════════════════════════════════════════╗");
    println!("║  性能估算示例                                           ║");
    println!("╚══════════════════════════════════════════════════════════╝");

    for (name, proof_count) in &test_cases[1..] {
        // 跳过极小批次
        println!("\n--- {} ({} proofs) ---", name, proof_count);
        let metrics = decider.estimate_performance_gain(*proof_count);
        metrics.print_report();
    }

    // 不同应用配置对比
    println!("\n╔══════════════════════════════════════════════════════════╗");
    println!("║  不同应用规模配置对比                                   ║");
    println!("╚══════════════════════════════════════════════════════════╝");

    let configs = vec![
        ("小型应用", AggregationConfig::small_app()),
        ("中型应用", AggregationConfig::medium_app()),
        ("大型应用", AggregationConfig::large_app()),
    ];

    for (name, config) in configs {
        println!("\n{}", name);
        println!("  最小聚合证明数: {}", config.min_proofs_for_aggregation);
        println!("  并行工作线程: {}", config.parallel_workers);
        println!("  单级批大小: {}", config.single_level_batch_size);
        println!("  两级配置: {} × {}", config.two_level_first_batch, config.two_level_second_batch);
        println!("  三级配置: {} × {} × {}", 
            config.three_level_batches.0,
            config.three_level_batches.1,
            config.three_level_batches.2
        );
    }

    // 成本效益分析
    println!("\n╔══════════════════════════════════════════════════════════╗");
    println!("║  成本效益分析                                           ║");
    println!("╚══════════════════════════════════════════════════════════╝");

    println!("\n假设 Gas Price = 30 gwei, ETH = $3000\n");

    let proof_counts = vec![10, 50, 100, 500, 1000];
    
    for &count in &proof_counts {
        let metrics = decider.estimate_performance_gain(count);
        let gas_saved = metrics.no_aggregation_gas - metrics.aggregated_gas;
        let cost_saved_usd = gas_saved as f64 * 30.0 * 1e-9 * 3000.0;
        
        println!("{} proofs:", count);
        println!("  策略: {}", metrics.strategy.description());
        println!("  Gas 节省: {} gas ({:.2}%)", gas_saved, metrics.gas_savings_percent);
        println!("  成本节省: ${:.2} USD", cost_saved_usd);
        println!("  TPS 提升: {:.2}x\n", metrics.tps_improvement);
    }

    // Rollup 场景模拟
    println!("\n╔══════════════════════════════════════════════════════════╗");
    println!("║  Rollup 场景模拟                                        ║");
    println!("╚══════════════════════════════════════════════════════════╝");

    println!("\n场景: L2 每 10s 向 L1 提交 1 个区块");
    println!("批大小: 100 tx/batch");
    println!("批数量: 10 batches/block = 1000 tx/block\n");

    let rollup_proof_count = 10; // 每个 batch 1 个证明
    let rollup_metrics = decider.estimate_performance_gain(rollup_proof_count);

    println!("不聚合策略:");
    println!("  每区块 L1 验证: {:.2}ms", rollup_metrics.no_aggregation_verify_time_ms);
    println!("  吞吐量: {:.0} TPS", rollup_metrics.no_aggregation_tps);
    println!("  每区块 Gas: {} gas", rollup_metrics.no_aggregation_gas);

    println!("\n聚合策略 ({}→1):", rollup_proof_count);
    println!("  每区块 L1 验证: {:.2}ms", rollup_metrics.aggregated_verify_time_ms);
    println!("  吞吐量: {:.0} TPS", rollup_metrics.aggregated_tps);
    println!("  每区块 Gas: {} gas", rollup_metrics.aggregated_gas);

    println!("\n✨ 提升: {:.2}x TPS, {:.2}% Gas 节省",
        rollup_metrics.tps_improvement,
        rollup_metrics.gas_savings_percent
    );

    println!("\n╔══════════════════════════════════════════════════════════╗");
    println!("║  结论                                                   ║");
    println!("╠══════════════════════════════════════════════════════════╣");
    println!("║  1. 证明数量 ≥ 6 时聚合开始划算                        ║");
    println!("║  2. 单级聚合可提供 ~10x TPS 提升                       ║");
    println!("║  3. 多级聚合适合大批量场景 (100x - 1000x)              ║");
    println!("║  4. Gas 成本可节省 85%-99.9%                           ║");
    println!("║  5. 自适应策略可根据批量自动优化                       ║");
    println!("╚══════════════════════════════════════════════════════════╝");
}
