use std::sync::Arc;
use std::thread;
use std::time::Duration;
use vm_runtime::{AutoGcConfig, GcConfig, MvccStore};

fn main() {
    println!("=== Demo 10: MVCC 自动垃圾回收 ===\n");

    demo_auto_gc_periodic();
    demo_auto_gc_threshold();
    demo_auto_gc_control();
    demo_auto_gc_vs_manual();
}

fn demo_auto_gc_periodic() {
    println!("1️⃣  周期性自动 GC");

    // 配置每 2 秒执行一次 GC
    let config = GcConfig {
        max_versions_per_key: 5,
        enable_time_based_gc: false,
        version_ttl_secs: 3600,
        auto_gc: Some(AutoGcConfig {
            interval_secs: 2,     // 每 2 秒
            version_threshold: 0, // 禁用阈值触发
            run_on_start: false,
            enable_adaptive: false,
        }),
    };

    let store = Arc::new(MvccStore::new_with_config(config));
    println!("  ⚙️  配置: 每 2 秒自动 GC");
    println!("  🚀 自动 GC 已启动: {}", store.is_auto_gc_running());

    // 生成一些旧版本
    for i in 0..20 {
        let mut txn = store.begin();
        txn.write(b"counter".to_vec(), format!("{}", i).into_bytes());
        txn.commit().unwrap();
    }
    println!("  📝 已写入 20 个版本");
    println!("  📊 当前总版本数: {}", store.total_versions());

    // 等待 GC 执行
    println!("  ⏳ 等待 2.5 秒让 GC 执行...");
    thread::sleep(Duration::from_millis(2500));

    let stats = store.get_gc_stats();
    println!("  🗑️  GC 统计:");
    println!("     - 执行次数: {}", stats.gc_count);
    println!("     - 清理版本数: {}", stats.versions_cleaned);
    println!("     - 当前版本数: {}", store.total_versions());

    store.stop_auto_gc();
    println!("  ⏹️  已停止自动 GC\n");
}

fn demo_auto_gc_threshold() {
    println!("2️⃣  阈值触发自动 GC");

    // 配置版本数超过 15 时触发
    let config = GcConfig {
        max_versions_per_key: 3,
        enable_time_based_gc: false,
        version_ttl_secs: 3600,
        auto_gc: Some(AutoGcConfig {
            interval_secs: 10,     // 周期较长
            version_threshold: 15, // 阈值触发
            run_on_start: false,
            enable_adaptive: false,
        }),
    };

    let store = Arc::new(MvccStore::new_with_config(config));
    println!("  ⚙️  配置: 版本数 ≥ 15 时触发 GC");

    // 快速写入到达阈值
    println!("  📝 写入版本...");
    for i in 0..20 {
        let mut txn = store.begin();
        txn.write(
            format!("key{}", i % 5).into_bytes(),
            format!("{}", i).into_bytes(),
        );
        txn.commit().unwrap();

        if i == 14 {
            println!("  📊 第 15 个版本写入，版本数: {}", store.total_versions());
        }
    }

    // 等待 GC 触发和执行
    thread::sleep(Duration::from_millis(500));

    let stats = store.get_gc_stats();
    println!("  🗑️  GC 自动触发:");
    println!("     - 执行次数: {}", stats.gc_count);
    println!("     - 清理版本数: {}", stats.versions_cleaned);
    println!("     - 当前版本数: {}", store.total_versions());

    store.stop_auto_gc();
    println!();
}

fn demo_auto_gc_control() {
    println!("3️⃣  动态控制自动 GC");

    let config = GcConfig {
        max_versions_per_key: 5,
        enable_time_based_gc: false,
        version_ttl_secs: 3600,
        auto_gc: Some(AutoGcConfig {
            interval_secs: 1,
            version_threshold: 10,
            run_on_start: false,
            enable_adaptive: false,
        }),
    };

    let store = Arc::new(MvccStore::new_with_config(config));
    println!(
        "  🚀 初始状态: 自动 GC 运行中 = {}",
        store.is_auto_gc_running()
    );

    // 停止
    store.stop_auto_gc();
    thread::sleep(Duration::from_millis(100));
    println!(
        "  ⏹️  停止后: 自动 GC 运行中 = {}",
        store.is_auto_gc_running()
    );

    // 手动写入
    for i in 0..10 {
        let mut txn = store.begin();
        txn.write(b"data".to_vec(), format!("{}", i).into_bytes());
        txn.commit().unwrap();
    }
    println!("  📝 写入 10 个版本 (GC 已停止)");
    println!("  📊 版本数: {}", store.total_versions());

    // 重新启动
    let _ = store.start_auto_gc();
    println!(
        "  ▶️  重新启动: 自动 GC 运行中 = {}",
        store.is_auto_gc_running()
    );

    thread::sleep(Duration::from_millis(1500));
    let stats = store.get_gc_stats();
    println!("  🗑️  重启后 GC 执行:");
    println!("     - 执行次数: {}", stats.gc_count);
    println!("     - 当前版本数: {}", store.total_versions());

    // 动态更新配置
    store.update_auto_gc_config(Some(AutoGcConfig {
        interval_secs: 30,      // 改为 30 秒
        version_threshold: 100, // 提高阈值
        run_on_start: false,
        enable_adaptive: false,
    }));
    println!("  🔧 已更新配置: interval=30s, threshold=100");

    store.stop_auto_gc();
    println!();
}

fn demo_auto_gc_vs_manual() {
    println!("4️⃣  自动 GC vs 手动 GC 对比\n");

    // 手动 GC
    println!("  📋 手动 GC 模式:");
    let manual_store = Arc::new(MvccStore::new());

    for i in 0..50 {
        let mut txn = manual_store.begin();
        txn.write(b"counter".to_vec(), format!("{}", i).into_bytes());
        txn.commit().unwrap();

        // 手动触发 GC
        if i % 10 == 0 {
            let cleaned = manual_store.gc().unwrap();
            println!("     - 第 {} 次写入，手动 GC 清理 {} 版本", i, cleaned);
        }
    }
    println!("     ✅ 需要手动调用 gc() 方法");
    println!("     ❌ 需要开发者选择合适时机");
    println!("     ❌ 可能遗忘导致内存增长\n");

    // 自动 GC
    println!("  🤖 自动 GC 模式:");
    let config = GcConfig {
        max_versions_per_key: 5,
        enable_time_based_gc: false,
        version_ttl_secs: 3600,
        auto_gc: Some(AutoGcConfig {
            interval_secs: 1,
            version_threshold: 20,
            run_on_start: false,
            enable_adaptive: false,
        }),
    };
    let auto_store = Arc::new(MvccStore::new_with_config(config));

    for i in 0..50 {
        let mut txn = auto_store.begin();
        txn.write(b"counter".to_vec(), format!("{}", i).into_bytes());
        txn.commit().unwrap();
    }

    thread::sleep(Duration::from_millis(1500));
    let stats = auto_store.get_gc_stats();
    println!("     - 写入 50 次，自动 GC 执行 {} 次", stats.gc_count);
    println!("     - 清理了 {} 个版本", stats.versions_cleaned);
    println!("     ✅ 后台自动执行，无需干预");
    println!("     ✅ 基于时间和阈值智能触发");
    println!("     ✅ Drop 时自动清理线程\n");

    auto_store.stop_auto_gc();

    println!("  💡 建议: 生产环境使用自动 GC，测试环境可用手动 GC");
    println!();
}
