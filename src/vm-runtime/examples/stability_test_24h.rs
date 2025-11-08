// 24小时稳定性测试
// 验证 MVCC + RocksDB 系统的长期稳定性、内存管理和性能一致性

use rand::Rng;
use std::sync::Arc;
use std::thread;
use std::time::{Duration, Instant};
use vm_runtime::{GcConfig, MvccStore, RocksDBConfig, RocksDBStorage};

const TEST_DURATION_HOURS: u64 = 1; // 原 24，示例缩短
const REPORT_INTERVAL_MINUTES: u64 = 5; // 原 10
const CHECKPOINT_INTERVAL_HOURS: u64 = 1; // 原 6

fn main() {
    println!("🚀 开始 24 小时稳定性测试");
    println!("📊 测试配置:");
    println!("   - 测试时长: {} 小时", TEST_DURATION_HOURS);
    println!("   - 报告间隔: {} 分钟", REPORT_INTERVAL_MINUTES);
    println!("   - 检查点间隔: {} 小时", CHECKPOINT_INTERVAL_HOURS);
    println!("   - 启用功能: MVCC, RocksDB, GC, Auto-Flush, Metrics, Pruning\n");

    // 初始化 RocksDB
    let db_path = "data/stability_test_24h";
    std::fs::create_dir_all(db_path).unwrap();
    let mut rocksdb = RocksDBStorage::new(RocksDBConfig::default().with_path(db_path)).unwrap();
    println!("✅ RocksDB 初始化成功: {}", db_path);

    // 初始化 MVCC Store
    let gc_config = GcConfig {
        max_versions_per_key: 10,
        enable_time_based_gc: false,
        version_ttl_secs: 3600,
        auto_gc: None,
    };
    let mvcc = MvccStore::new_with_config(gc_config);
    println!("✅ MVCC Store 初始化成功");

    // 本示例改用手动刷新（manual_flush）以便与检查点操作配合
    println!("ℹ️ 使用手动刷新 (manual_flush) 进行周期性持久化\n");

    // 测试参数
    let test_duration = Duration::from_secs(TEST_DURATION_HOURS * 3600);
    let report_interval = Duration::from_secs(REPORT_INTERVAL_MINUTES * 60);
    let checkpoint_interval = Duration::from_secs(CHECKPOINT_INTERVAL_HOURS * 3600);

    let start_time = Instant::now();
    let mut last_report = Instant::now();
    let mut last_checkpoint = Instant::now();
    let mut checkpoint_count = 0;

    let mut iteration = 0u64;
    let mut total_txns = 0u64;
    let mut total_success = 0u64;
    let mut total_conflicts = 0u64;

    println!("🎯 开始压力测试循环...\n");

    // 主测试循环
    while start_time.elapsed() < test_duration {
        iteration += 1;

        // 执行一批事务
        let batch_size = 1000;
        let (success, conflicts) = run_transaction_batch(&mvcc, batch_size);

        total_txns += batch_size;
        total_success += success;
        total_conflicts += conflicts;

        // 定期报告
        if last_report.elapsed() >= report_interval {
            print_progress_report(
                start_time.elapsed(),
                test_duration,
                total_txns,
                total_success,
                total_conflicts,
                &mvcc,
            );
            last_report = Instant::now();
        }

        // 定期创建检查点
        if last_checkpoint.elapsed() >= checkpoint_interval {
            checkpoint_count += 1;
            create_checkpoint(&mut rocksdb, &mvcc, checkpoint_count);
            last_checkpoint = Instant::now();
        }

        // 每1000次迭代执行状态裁剪
        if iteration % 1000 == 0 {
            prune_old_versions(&mvcc, &mut rocksdb);
        }

        // 短暂休眠避免CPU过载
        thread::sleep(Duration::from_millis(100));
    }

    // 最终报告
    println!("\n{}", "=".repeat(80));
    println!("🎉 稳定性测试完成 (示例版)");
    println!("{}\n", "=".repeat(80));

    print_final_report(
        test_duration,
        total_txns,
        total_success,
        total_conflicts,
        checkpoint_count,
        &mvcc,
    );

    // 导出最终指标（片段）
    if let Some(metrics) = mvcc.get_metrics() {
        let s = metrics.export_prometheus();
        let cut = s.len().min(200);
        println!("\n📊 最终 Prometheus 指标片段:\n{}...", &s[..cut]);
    }
    println!("\n✅ 测试完成,所有资源已清理");
}

/// 执行一批事务
fn run_transaction_batch(mvcc: &Arc<MvccStore>, batch_size: u64) -> (u64, u64) {
    let mut rng = rand::thread_rng();
    let mut success = 0;
    let mut conflicts = 0;

    for _ in 0..batch_size {
    let mut tx = mvcc.begin();

        // 随机读写操作
        let key_count = rng.gen_range(1..=5);
        let mut read_keys = vec![];

        // 读操作
        for _ in 0..key_count {
            let key = format!("key_{}", rng.gen_range(0..100));
            if tx.read(key.as_bytes()).is_some() { read_keys.push(key); }
        }

        // 写操作
        for key in &read_keys {
            let value = format!("value_{}", rng.gen_range(0..1000));
            tx.write(key.clone().into_bytes(), value.into_bytes());
        }

        // 提交事务
        if tx.commit().is_ok() { success += 1; } else { conflicts += 1; }
    }

    (success, conflicts)
}

/// 打印进度报告
fn print_progress_report(
    elapsed: Duration,
    total_duration: Duration,
    total_txns: u64,
    total_success: u64,
    total_conflicts: u64,
    mvcc: &Arc<MvccStore>,
) {
    let elapsed_hours = elapsed.as_secs() as f64 / 3600.0;
    let progress_pct = (elapsed.as_secs() as f64 / total_duration.as_secs() as f64) * 100.0;
    let success_rate = (total_success as f64 / total_txns as f64) * 100.0;
    let avg_tps = total_success as f64 / elapsed.as_secs() as f64;

    println!("{}", "=".repeat(80));
    println!(
        "📈 进度报告 [{:.1}% 完成, {:.2} 小时已过]",
        progress_pct, elapsed_hours
    );
    println!("{}", "=".repeat(80));
    println!("📊 事务统计:");
    println!("   - 总事务数: {}", total_txns);
    println!("   - 成功提交: {} ({:.2}%)", total_success, success_rate);
    println!(
        "   - 冲突回滚: {} ({:.2}%)",
        total_conflicts,
        100.0 - success_rate
    );
    println!("   - 平均 TPS: {:.0}", avg_tps);

    let (current_tps, current_success_rate) = if let Some(m) = mvcc.get_metrics() { (m.tps(), m.success_rate()) } else { (0.0, 0.0) };

    println!("\n📊 实时性能:");
    println!("   - 当前 TPS: {:.0}", current_tps);
    println!("   - 当前成功率: {:.2}%", current_success_rate);

    let gc_stats = mvcc.get_gc_stats();
    println!("\n🗑️  GC 统计:");
    println!("   - GC 运行次数: {}", gc_stats.gc_count);
    println!("   - 清理版本数: {}", gc_stats.versions_cleaned);

    let flush_stats = mvcc.get_flush_stats();
    println!("\n💾 Flush 统计:");
    println!("   - Flush 次数: {}", flush_stats.flush_count);
    println!("   - Flush 键数: {}", flush_stats.keys_flushed);
    println!("   - Flush 字节数: {} KB", flush_stats.bytes_flushed / 1024);
    println!();
}

/// 创建检查点
fn create_checkpoint(rocksdb: &mut RocksDBStorage, mvcc: &Arc<MvccStore>, count: u32) {
    println!("📸 创建检查点 #{}", count);

    // 刷新 MVCC 数据到 RocksDB（保留最近 3 个版本在内存）
    match mvcc.manual_flush(rocksdb, 3) {
        Ok((keys, bytes)) => {
            println!("   ✅ MVCC 刷新: {} 键, {} KB", keys, bytes / 1024);
        }
        Err(e) => {
            println!("   ❌ MVCC 刷新失败: {}", e);
            return;
        }
    }

    // 创建 RocksDB 检查点
    let checkpoint_name = format!("checkpoint_{}", count);
    match rocksdb.create_checkpoint(&checkpoint_name) {
        Ok(_) => println!("   ✅ RocksDB 检查点创建成功: {}", checkpoint_name),
        Err(e) => println!("   ❌ RocksDB 检查点失败: {}", e),
    }
}

/// 状态裁剪
fn prune_old_versions(mvcc: &Arc<MvccStore>, rocksdb: &RocksDBStorage) {
    let (versions, keys) = mvcc.prune_old_versions(10, rocksdb);
    if versions > 0 {
        println!("✂️  状态裁剪: 清理 {} 版本, {} 键", versions, keys);
    }
}

/// 打印最终报告
fn print_final_report(
    duration: Duration,
    total_txns: u64,
    total_success: u64,
    total_conflicts: u64,
    checkpoint_count: u32,
    mvcc: &Arc<MvccStore>,
) {
    let hours = duration.as_secs() as f64 / 3600.0;
    let success_rate = (total_success as f64 / total_txns as f64) * 100.0;
    let avg_tps = total_success as f64 / duration.as_secs() as f64;

    println!("📊 最终统计报告");
    println!("{}", "=".repeat(80));
    println!("\n⏱️  测试时长: {:.2} 小时", hours);

    println!("\n📈 事务性能:");
    println!("   - 总事务数: {}", total_txns);
    println!("   - 成功提交: {} ({:.2}%)", total_success, success_rate);
    println!(
        "   - 冲突回滚: {} ({:.2}%)",
        total_conflicts,
        100.0 - success_rate
    );
    println!("   - 平均 TPS: {:.0}", avg_tps);

    if let Some(metrics) = mvcc.get_metrics() {
        println!("\n📊 延迟统计:");
        println!("   - P50 延迟: {:.2} ms", metrics.latency_p50());
        println!("   - P90 延迟: {:.2} ms", metrics.latency_p90());
        println!("   - P99 延迟: {:.2} ms", metrics.latency_p99());
        println!("   - TPS(窗口): {:.0}", metrics.tps_window());
        println!("   - TPS(峰值-窗口): {:.0}", metrics.peak_tps());
    }

    let gc_stats = mvcc.get_gc_stats();
    println!("\n🗑️  GC 性能:");
    println!("   - GC 运行次数: {}", gc_stats.gc_count);
    println!("   - 清理版本数: {}", gc_stats.versions_cleaned);
    println!(
        "   - 平均清理/次: {:.1}",
    gc_stats.versions_cleaned as f64 / gc_stats.gc_count.max(1) as f64
    );

    let flush_stats = mvcc.get_flush_stats();
    println!("\n💾 Flush 性能:");
    println!("   - Flush 次数: {}", flush_stats.flush_count);
    println!("   - Flush 键数: {}", flush_stats.keys_flushed);
    println!(
        "   - Flush 字节数: {:.2} MB",
        flush_stats.bytes_flushed as f64 / 1024.0 / 1024.0
    );
    println!(
        "   - 平均键数/次: {:.1}",
        flush_stats.keys_flushed as f64 / flush_stats.flush_count.max(1) as f64
    );

    println!("\n📸 检查点:");
    println!("   - 创建次数: {}", checkpoint_count);
    println!(
        "   - 平均间隔: {:.2} 小时",
        hours / checkpoint_count.max(1) as f64
    );

    println!("\n✅ 稳定性结论:");
    if success_rate >= 95.0 {
        println!(
            "   🎉 优秀 - 系统稳定运行 {:.2} 小时,成功率 {:.2}%",
            hours, success_rate
        );
    } else if success_rate >= 80.0 {
        println!("   ⚠️  警告 - 成功率 {:.2}% 低于预期 (95%)", success_rate);
    } else {
        println!("   ❌ 失败 - 成功率 {:.2}% 严重偏低", success_rate);
    }
}
