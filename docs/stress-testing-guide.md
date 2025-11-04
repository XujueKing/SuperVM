# MVCC 压力测试与调优指南 (v0.8.0)

版本: v0.8.0  
日期: 2025-11-04

## 目录

- [概述](#概述)
- [压力测试套件](#压力测试套件)
- [测试场景](#测试场景)
- [自适应 GC](#自适应-gc)
- [性能调优建议](#性能调优建议)
- [故障排查](#故障排查)

---

## 概述

本指南介绍如何使用 MVCC 压力测试套件来评估系统性能，以及如何根据测试结果进行调优。

### 为什么需要压力测试？

- **验证稳定性**: 确保高负载下系统不会崩溃或内存泄漏
- **评估性能**: 了解系统的吞吐量和延迟特性
- **优化配置**: 找到最适合您工作负载的 GC 参数
- **发现瓶颈**: 识别性能瓶颈和优化机会

---

## 压力测试套件

### 运行所有测试

```bash
# 运行所有快速测试（排除长时间测试）
cargo test -p vm-runtime --test mvcc_stress_test -- --test-threads=1 --nocapture

# 运行包括长时间测试
cargo test -p vm-runtime --test mvcc_stress_test -- --test-threads=1 --nocapture --ignored
```

### 可用测试

| 测试名称 | 描述 | 运行时间 |
|---------|------|---------|
| `test_high_concurrency_mixed_workload` | 高并发混合读写（8线程，8000交易） | ~1秒 |
| `test_high_contention_hotspot` | 高冲突热点键（16线程，5个热点键） | ~1秒 |
| `test_memory_growth_control` | 内存增长监控（50键，20迭代） | ~10秒 |
| `test_adaptive_gc` | 自适应 GC 行为验证 | ~20秒 |
| `test_long_running_stability` | 长时间稳定性（60秒，可配置） | 60秒+ |

---

## 测试场景

### 1. 高并发混合读写测试

**目的**: 评估混合读写负载下的性能

**配置**:
```rust
- 线程数: 8
- 每线程交易数: 1000
- 总键数: 100
- 读比例: 70%
- 写比例: 30%
```

**预期结果**:
- 吞吐量: > 100,000 TPS
- 成功率: > 99%
- 冲突率: < 1%
- 平均延迟: < 20 μs

**示例输出**:
```
╔═══════════════════════════════════════════════════════════╗
║         MVCC 压力测试报告                                  ║
╠═══════════════════════════════════════════════════════════╣
║ 总交易数:           8000 笔                              ║
║ 成功交易:           7983 笔 (99.8%)                     ║
║ 失败交易:             17 笔 (0.2%)                     ║
║ 冲突数:               17 次                              ║
╠═══════════════════════════════════════════════════════════╣
║ 吞吐量:        729634.09 TPS                           ║
║ 平均延迟:           8.02 μs                            ║
║ P99 延迟:          59.00 μs                            ║
╚═══════════════════════════════════════════════════════════╝
```

### 2. 高冲突热点键测试

**目的**: 评估在极端冲突场景下的表现

**配置**:
```rust
- 线程数: 16
- 每线程交易数: 500
- 热点键数: 5
- 读写模式: 读-修改-写
```

**预期结果**:
- 吞吐量: > 50,000 TPS
- 成功率: > 95%
- 冲突率: 1-5%
- 最终数据正确性: 100%

**示例输出**:
```
热点键 hot_0 最终值: 522
热点键 hot_1 最终值: 589
热点键 hot_2 最终值: 507
...
冲突率: 1.9%
```

### 3. 内存增长控制测试

**目的**: 验证 GC 能有效控制内存增长

**配置**:
```rust
- 键数: 50
- 迭代次数: 20
- GC 间隔: 2秒
- 版本阈值: 500
```

**预期结果**:
- 版本数稳定在 `键数 × max_versions_per_key` 附近
- GC 定期执行（至少 2 次）
- 内存不会无限增长

**示例输出**:
```
迭代  1: 版本数 =   50, 键数 =  50, 平均版本/键 = 1.00
迭代  2: 版本数 =  100, 键数 =  50, 平均版本/键 = 2.00
...
迭代 14: 版本数 =  550, 键数 =  50, 平均版本/键 = 11.00
迭代 15: 版本数 =  600, 键数 =  50, 平均版本/键 = 12.00

✅ 测试完成
最终版本数: 650
GC 执行次数: 2
GC 清理版本: 350
```

### 4. 长时间稳定性测试

**目的**: 验证长时间运行的稳定性

**配置**:
```rust
- 运行时长: 60秒（可配置为数小时）
- 线程数: 4
- 键数: 200
- 读写比例: 50/50
```

**监控指标**:
- 实时 TPS
- 版本数变化
- GC 执行频率
- 内存占用

**示例输出**:
```
[10s] TPS: 5810, 版本数: 58301, 键数: 200, GC 次数: 0
[20s] TPS: 5824, 版本数: 58302, 键数: 200, GC 次数: 1
[30s] TPS: 5831, 版本数: 58305, 键数: 200, GC 次数: 2
...
```

---

## 自适应 GC

### 什么是自适应 GC？

自适应 GC 根据系统负载和内存压力动态调整 GC 参数，无需手动调优。

### 配置自适应 GC

```rust
use vm_runtime::{MvccStore, GcConfig, AutoGcConfig, AdaptiveGcStrategy};

let config = GcConfig {
    max_versions_per_key: 20,
    enable_time_based_gc: false,
    version_ttl_secs: 3600,
    auto_gc: Some(AutoGcConfig {
        interval_secs: 60,          // 基准间隔
        version_threshold: 1000,    // 基准阈值
        run_on_start: false,
        enable_adaptive: true,      // 🎯 启用自适应
    }),
};

let store = MvccStore::new_with_config(config);
```

### 自适应策略

```rust
pub struct AdaptiveGcStrategy {
    pub base_interval_secs: 60,    // 基准间隔
    pub min_interval_secs: 10,     // 最小间隔（高负载）
    pub max_interval_secs: 300,    // 最大间隔（低负载）
    pub base_threshold: 1000,      // 基准阈值
    pub min_threshold: 500,        // 最小阈值（更激进）
    pub max_threshold: 5000,       // 最大阈值（更宽松）
}
```

### 调整逻辑

1. **高负载检测**（高 TPS 或版本快速增长）:
   - ✅ 缩短 GC 间隔
   - ✅ 降低触发阈值
   - 🎯 更频繁、更激进的 GC

2. **低效 GC 检测**（清理率 < 10%）:
   - ✅ 延长 GC 间隔
   - ✅ 提高触发阈值
   - 🎯 减少无效 GC，节省资源

3. **正常负载**:
   - ✅ 逐渐回归基准值
   - 🎯 保持稳定状态

### 自适应 GC 测试

运行自适应 GC 测试:
```bash
cargo test -p vm-runtime --test mvcc_stress_test test_adaptive_gc -- --nocapture
```

观察输出，确认:
- GC 在高负载时更频繁执行
- 版本数被有效控制
- 低负载时 GC 频率降低

---

## 性能调优建议

### 基于测试结果调优

#### 场景 1: 高吞吐量要求

**问题**: 需要最大化 TPS

**建议**:
```rust
GcConfig {
    max_versions_per_key: 30,      // ⬆️ 增加版本限制
    auto_gc: Some(AutoGcConfig {
        interval_secs: 120,        // ⬆️ 延长间隔
        version_threshold: 2000,   // ⬆️ 提高阈值
        enable_adaptive: true,     // ✅ 启用自适应
    }),
}
```

**原理**: 减少 GC 频率，降低开销

---

#### 场景 2: 内存敏感场景

**问题**: 内存有限，需要严格控制

**建议**:
```rust
GcConfig {
    max_versions_per_key: 10,      // ⬇️ 降低版本限制
    auto_gc: Some(AutoGcConfig {
        interval_secs: 30,         // ⬇️ 缩短间隔
        version_threshold: 500,    // ⬇️ 降低阈值
        enable_adaptive: true,     // ✅ 启用自适应
    }),
}
```

**原理**: 更激进的 GC，快速回收内存

---

#### 场景 3: 高冲突场景

**问题**: 热点键导致大量冲突

**建议**:
```rust
GcConfig {
    max_versions_per_key: 50,      // ⬆️ 热点键需要更多版本
    auto_gc: Some(AutoGcConfig {
        interval_secs: 60,
        version_threshold: 1000,
        enable_adaptive: true,     // ✅ 让自适应处理
    }),
}

// 同时考虑应用层优化：
// 1. 键分片（减少热点）
// 2. 批量操作（减少冲突）
// 3. 只读事务（使用快速路径）
```

---

#### 场景 4: 长事务场景

**问题**: 有长时间运行的事务

**建议**:
```rust
GcConfig {
    max_versions_per_key: 100,     // ⬆️⬆️ 大幅增加
    auto_gc: Some(AutoGcConfig {
        interval_secs: 180,        // ⬆️ 延长间隔
        version_threshold: 5000,   // ⬆️ 提高阈值
        enable_adaptive: false,    // ❌ 禁用（避免误判）
    }),
}
```

**原理**: 保留足够多的版本供长事务读取

---

### 监控指标

运行压力测试时关注:

1. **吞吐量 (TPS)**:
   - 高并发: > 100K TPS
   - 高冲突: > 50K TPS

2. **延迟**:
   - 平均: < 20 μs
   - P99: < 100 μs

3. **成功率**:
   - 混合负载: > 99%
   - 高冲突: > 95%

4. **内存**:
   - 版本数: < 键数 × max_versions_per_key × 1.5
   - GC 清理率: > 10%

---

## 故障排查

### 问题 1: 内存持续增长

**症状**: 版本数不断增加，GC 无效

**可能原因**:
- GC 间隔太长
- 版本阈值太高
- 有长时间未提交的事务

**解决方案**:
```rust
// 1. 降低 GC 间隔和阈值
interval_secs: 30,
version_threshold: 500,

// 2. 检查活跃事务
let min_ts = store.get_min_active_ts();
println!("最小活跃事务: {:?}", min_ts);

// 3. 确保所有事务及时提交或回滚
```

---

### 问题 2: 吞吐量下降

**症状**: TPS 远低于预期

**可能原因**:
- GC 太频繁
- 高冲突率
- 锁竞争

**解决方案**:
```rust
// 1. 延长 GC 间隔
interval_secs: 120,

// 2. 启用自适应 GC
enable_adaptive: true,

// 3. 使用只读事务优化读操作
let ro_txn = store.begin_read_only();
```

---

### 问题 3: 高冲突率

**症状**: 大量事务失败

**可能原因**:
- 热点键访问
- 版本数不足
- 事务粒度太大

**解决方案**:
```rust
// 1. 增加版本限制
max_versions_per_key: 50,

// 2. 应用层分片
let shard = hash(key) % num_shards;
let sharded_key = format!("{}_{}", key, shard);

// 3. 缩小事务范围
// 将大事务拆分为多个小事务
```

---

### 问题 4: P99 延迟高

**症状**: 平均延迟正常，但 P99 很高

**可能原因**:
- GC 暂停
- 锁等待
- 版本链过长

**解决方案**:
```rust
// 1. 更激进的 GC
max_versions_per_key: 15,
version_threshold: 800,

// 2. 监控 GC 执行
let stats = store.get_gc_stats();
println!("GC 频率: {} 次/分钟", 
    stats.gc_count as f64 / elapsed_minutes);
```

---

## 总结

✅ **压力测试是必需的**  
在生产环境部署前，务必运行完整的压力测试套件

✅ **自适应 GC 是首选**  
对于大多数场景，启用自适应 GC 可以自动优化性能

✅ **监控是关键**  
持续监控 TPS、延迟、版本数和 GC 统计

✅ **根据工作负载调优**  
不同场景需要不同的配置，使用本指南作为起点

---

## 下一步

- 查看 [并行执行设计文档](parallel-execution.md)
- 查看 [Demo 示例](../../src/node-core/examples/)
- 查看 [基准测试报告](../../BENCHMARK_RESULTS.md)

---

*最后更新: 2025-11-04*
