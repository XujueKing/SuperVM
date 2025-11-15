// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2025 XujueKing <leadbrand@me.com>

//! Persistence Consistency Test
//! 测试流程: Write → Restart → Verify
//! 验证 RocksDB 持久化的正确性与一致性

use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

#[cfg(not(feature = "rocksdb-storage"))]
fn main() {
    println!("⚠️  此示例需要启用 rocksdb-storage 特性");
    println!("请运行: cargo run --example persistence_consistency_test --features rocksdb-storage");
}

#[cfg(feature = "rocksdb-storage")]
fn main() {
    use vm_runtime::{GcConfig, MvccStore, RocksDBConfig, RocksDBStorage, Storage};

    println!("=== Persistence Consistency Test ===\n");
    println!("测试流程: Write → Restart → Verify\n");

    let test_db_path = "data/persistence_test";
    let test_iterations = 100;
    let test_key_prefix = "consistency_test";

    // ==================== Phase 1: Write ====================
    println!("📝 Phase 1: 写入阶段");
    println!("   数据库路径: {}", test_db_path);
    println!("   测试迭代: {} 个键值对\n", test_iterations);

    // 清理旧数据
    let _ = std::fs::remove_dir_all(test_db_path);
    std::fs::create_dir_all(test_db_path).unwrap();

    let write_start = SystemTime::now();

    {
        // 1.1 初始化 RocksDB + MVCC
        let mut rocksdb =
            RocksDBStorage::new(RocksDBConfig::default().with_path(test_db_path))
                .expect("RocksDB init failed");
        let gc_config = GcConfig {
            max_versions_per_key: 5,
            enable_time_based_gc: false,
            version_ttl_secs: 3600,
            auto_gc: None,
        };
        let mvcc = Arc::new(MvccStore::new_with_config(gc_config));

        println!("   ✅ 数据库初始化成功");

        // 1.2 写入测试数据 (MVCC + RocksDB 双写)
        let mut expected_data = Vec::new();
        for i in 0..test_iterations {
            let key = format!("{}_{}", test_key_prefix, i);
            let value = format!("value_{}_{}", i, write_start.duration_since(UNIX_EPOCH).unwrap().as_micros());

            // MVCC 写入
            let tx = mvcc.begin();
            let mut tx = tx;
            tx.write(key.clone().into_bytes(), value.clone().into_bytes());
            match tx.commit() {
                Ok(commit_ts) => {
                    // RocksDB 持久化
                    let storage_key = format!("{}@{}", key, commit_ts);
                    let storage_key_bytes = storage_key.as_bytes();
                    let value_bytes = value.as_bytes();
                    
                    if let Err(e) = rocksdb.set(storage_key_bytes, value_bytes) {
                        eprintln!("      ❌ RocksDB write failed: {}", e);
                    } else {
                        expected_data.push((storage_key, value.clone()));
                    }

                    if (i + 1) % 20 == 0 {
                        println!("      写入进度: {}/{} (ts={})", i + 1, test_iterations, commit_ts);
                    }
                }
                Err(e) => {
                    eprintln!("      ❌ 事务 {} 提交失败: {}", i, e);
                }
            }
        }

        // 1.3 保存预期数据清单到文件
        let manifest_path = format!("{}/expected_manifest.txt", test_db_path);
        let manifest_content = expected_data
            .iter()
            .map(|(k, v)| format!("{}={}", k, v))
            .collect::<Vec<_>>()
            .join("\n");
        std::fs::write(&manifest_path, manifest_content).unwrap();

        println!("\n   ✅ 写入完成:");
        println!("      - 成功写入: {} 条记录", expected_data.len());
        println!("      - 预期清单: {}", manifest_path);

        // 1.4 采集写入后的 RocksDB 指标
        let metrics_after_write = rocksdb.collect_metrics();
        mvcc.update_rocksdb_metrics(&metrics_after_write);

        if let Some(mc) = mvcc.get_metrics() {
            use std::sync::atomic::Ordering;
            println!("\n   📊 写入阶段统计:");
            println!("      - MVCC 提交: {}", mc.txn_committed.load(Ordering::Relaxed));
            println!("      - MVCC 回滚: {}", mc.txn_aborted.load(Ordering::Relaxed));
            println!("      - RocksDB Keys: {}", mc.rocksdb_estimate_num_keys.load(Ordering::Relaxed));
            println!(
                "      - RocksDB SST Size: {:.2} KB",
                mc.rocksdb_total_sst_size_bytes.load(Ordering::Relaxed) as f64 / 1024.0
            );
        }

        // 显式关闭数据库（Drop）
        drop(rocksdb);
        drop(mvcc);
        println!("\n   🔒 数据库已关闭 (模拟进程重启)\n");
    }

    // ==================== Phase 2: Restart ====================
    println!("🔄 Phase 2: 重启阶段");
    println!("   模拟系统重启，等待 2 秒...\n");
    std::thread::sleep(Duration::from_secs(2));

    // ==================== Phase 3: Verify ====================
    println!("🔍 Phase 3: 验证阶段");

    {
        // 3.1 重新打开数据库
        let rocksdb =
            RocksDBStorage::new(RocksDBConfig::default().with_path(test_db_path))
                .expect("RocksDB reopen failed");
        println!("   ✅ 数据库重新打开成功");

        // 3.2 加载预期数据清单
        let manifest_path = format!("{}/expected_manifest.txt", test_db_path);
        let manifest_content = std::fs::read_to_string(&manifest_path).unwrap();
        let expected_data: Vec<(String, String)> = manifest_content
            .lines()
            .filter_map(|line| {
                let parts: Vec<&str> = line.splitn(2, '=').collect();
                if parts.len() == 2 {
                    Some((parts[0].to_string(), parts[1].to_string()))
                } else {
                    None
                }
            })
            .collect();

        println!("   📋 预期清单加载: {} 条记录\n", expected_data.len());

        // 3.3 逐条验证数据
        let mut success_count = 0;
        let mut failure_count = 0;
        let mut missing_count = 0;

        for (i, (key, expected_value)) in expected_data.iter().enumerate() {
            match rocksdb.get(&key.as_bytes()) {
                Ok(Some(actual_value)) => {
                    let actual_str = String::from_utf8_lossy(&actual_value);
                    if actual_str.as_ref() == expected_value {
                        success_count += 1;
                    } else {
                        failure_count += 1;
                        println!(
                            "      ❌ 验证失败 [{}]: 键={} 预期={} 实际={}",
                            i, key, expected_value, actual_str
                        );
                    }
                }
                Ok(None) => {
                    missing_count += 1;
                    println!("      ⚠️  数据丢失 [{}]: 键={}", i, key);
                }
                Err(e) => {
                    failure_count += 1;
                    println!("      ❌ 读取错误 [{}]: 键={} 错误={}", i, key, e);
                }
            }

            if (i + 1) % 20 == 0 {
                println!("      验证进度: {}/{}", i + 1, expected_data.len());
            }
        }

        // 3.4 采集重启后的 RocksDB 指标
        let metrics_after_restart = rocksdb.collect_metrics();
        let gc_config = GcConfig::default();
        let mvcc_verify = Arc::new(MvccStore::new_with_config(gc_config));
        mvcc_verify.update_rocksdb_metrics(&metrics_after_restart);

        println!("\n   📊 验证结果:");
        println!("      ✅ 成功匹配: {}/{}", success_count, expected_data.len());
        println!("      ❌ 值不匹配: {}", failure_count);
        println!("      ⚠️  数据丢失: {}", missing_count);

        if let Some(mc) = mvcc_verify.get_metrics() {
            use std::sync::atomic::Ordering;
            println!("\n   📊 重启后 RocksDB 统计:");
            println!("      - Estimated Keys: {}", mc.rocksdb_estimate_num_keys.load(Ordering::Relaxed));
            println!(
                "      - SST Size: {:.2} KB",
                mc.rocksdb_total_sst_size_bytes.load(Ordering::Relaxed) as f64 / 1024.0
            );
            let cache_hit = mc.rocksdb_cache_hit.load(Ordering::Relaxed);
            let cache_miss = mc.rocksdb_cache_miss.load(Ordering::Relaxed);
            let cache_total = cache_hit + cache_miss;
            if cache_total > 0 {
                println!(
                    "      - Cache Hit Rate: {:.2}%",
                    (cache_hit as f64 / cache_total as f64) * 100.0
                );
            }
        }

        // 3.5 生成测试报告
        println!("\n📄 生成测试报告...");
        let report_path = format!("{}/consistency_test_report.txt", test_db_path);
        let report = format!(
            r#"=== Persistence Consistency Test Report ===
Database Path: {}
Test Time: {}
Iterations: {}

Write Phase:
  - Records Written: {}

Verify Phase:
  - Success: {}
  - Failures: {}
  - Missing: {}
  - Success Rate: {:.2}%

Conclusion: {}
"#,
            test_db_path,
            chrono::Local::now().format("%Y-%m-%d %H:%M:%S"),
            test_iterations,
            expected_data.len(),
            success_count,
            failure_count,
            missing_count,
            (success_count as f64 / expected_data.len() as f64) * 100.0,
            if success_count == expected_data.len() && failure_count == 0 && missing_count == 0 {
                "✅ PASS - 持久化一致性验证通过"
            } else {
                "❌ FAIL - 检测到数据丢失或不一致"
            }
        );

        std::fs::write(&report_path, &report).unwrap();
        println!("   💾 报告已保存: {}", report_path);

        drop(rocksdb);
    }

    // ==================== Summary ====================
    println!("\n✅ 测试完成!");
    println!("💡 下一步:");
    println!("   1. 检查测试报告: {}/consistency_test_report.txt", test_db_path);
    println!("   2. 集成到 CI/CD 流程进行自动化验证");
    println!("   3. 扩展测试场景: 并发写入、大数据量、异常中断恢复");
}
