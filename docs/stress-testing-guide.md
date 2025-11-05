# MVCC 压力测试与调优指南 (v0.8.0)

版本: v0.8.0  
最后更新: 2025-11-05

## 目录

- [概述](#概述)
- [压力测试套件](#压力测试套件)
- [测试场景详解](#测试场景详解)
- [GC 配置参数](#gc-配置参数)
- [自适应 GC](#自适应-gc)
- [性能调优建议](#性能调优建议)
- [故障排查](#故障排查)
- [监控最佳实践](#监控最佳实践)

---

## 概述

本指南介绍如何使用 MVCC 压力测试套件来评估系统性能，以及如何根据测试结果进行调优。

SuperVM 的 MVCC 存储系统提供了完整的压力测试工具，涵盖高并发、高冲突、内存控制、长时间稳定性等场景，帮助您在生产部署前充分验证系统能力。

### 为什么需要压力测试？

 **验证稳定性**: 确保高负载下系统不会崩溃或内存泄漏  
 **评估性能**: 了解系统的吞吐量（TPS）和延迟特性  
 **优化配置**: 找到最适合您工作负载的 GC 参数  
 **发现瓶颈**: 识别性能瓶颈和优化机会  
 **调优指导**: 为生产环境提供配置依据

---

## 压力测试套件

### 快速开始

`ash
# 运行所有快速测试（排除长时间测试）
cargo test -p vm-runtime --test mvcc_stress_test -- --test-threads=1 --nocapture

# 运行包括长时间测试
cargo test -p vm-runtime --test mvcc_stress_test -- --test-threads=1 --nocapture --ignored

# 运行特定测试
cargo test -p vm-runtime --test mvcc_stress_test test_high_concurrency_mixed_workload -- --nocapture
`

### 可用测试套件

| 测试名称 | 描述 | 运行时间 | 关键指标 |
|---------|------|---------|---------|
| 	est_high_concurrency_mixed_workload | 高并发混合读写（8线程，1000交易） | ~1秒 | TPS, 成功率, 延迟 |
| 	est_high_contention_hotspot | 高冲突热点键（16线程，5个热点键） | ~1秒 | 冲突率, 数据正确性 |
| 	est_memory_growth_control | 内存增长监控（50键，20迭代） | ~10秒 | 版本数, GC效果 |
| 	est_adaptive_gc | 自适应 GC 行为验证 | ~20秒 | 参数调整, 清理效果 |
| 	est_long_running_stability | 长时间稳定性（60秒，可配置） | 60秒+ | 内存稳定性, TPS稳定性 |

---

## 测试场景详解

### 1. 高并发混合读写测试

**测试目的**  
评估混合读写负载下的系统性能，模拟真实生产环境中的典型工作负载。

**测试配置**
`ust
线程数: 8
每线程交易数: 1000
总键数: 100
读操作比例: 70%
写操作比例: 30%
`

**GC 配置**
`ust
let config = GcConfig {
    max_versions_per_key: 20,
    enable_time_based_gc: false,
    version_ttl_secs: 3600,
    auto_gc: Some(AutoGcConfig {
        interval_secs: 5,
        version_threshold: 1000,
        run_on_start: false,
        enable_adaptive: false,
    }),
};
`

**预期结果**
-  吞吐量: > 100,000 TPS
-  成功率: > 99%
-  冲突率: < 1%
-  平均延迟: < 20 μs
-  P99 延迟: < 100 μs

**示例输出**
`

         MVCC 压力测试报告                                  

 总交易数:           8000 笔                              
 成功交易:           7983 笔 (99.8%)                     
 失败交易:             17 笔 (0.2%)                      
 冲突数:               17 次                              

 总读操作:           5600 次                              
 总写操作:           2400 次                              

 运行时间:           0.01 秒                              
 吞吐量:        729634.09 TPS                           
 平均延迟:           8.02 μs                            
 P99 延迟:          59.00 μs                            

 内存版本数:          850 个                              
 内存键数:            100 个                              

`

---

### 2. 高冲突热点键测试

**测试目的**  
评估在极端冲突场景下的表现，验证 MVCC 冲突检测和重试机制。

**测试配置**
`ust
线程数: 16
每线程交易数: 500
热点键数: 5 (所有线程竞争同样的键)
访问模式: 读-修改-写
`

**GC 配置**
`ust
let config = GcConfig {
    max_versions_per_key: 50,  // 热点键需要更多版本
    enable_time_based_gc: false,
    version_ttl_secs: 3600,
    auto_gc: Some(AutoGcConfig {
        interval_secs: 3,
        version_threshold: 500,
        run_on_start: false,
        enable_adaptive: false,
    }),
};
`

**预期结果**
-  吞吐量: > 50,000 TPS
-  成功率: > 95%
-  冲突率: 1-5%
-  最终数据正确性: 100%

**示例输出**
`
 高冲突热点键压力测试
   线程数: 16
   每线程交易数: 500
   热点键数: 5

   热点键 hot_0 最终值: 522 (预期: 16*500 成功次数)
   热点键 hot_1 最终值: 489
   热点键 hot_2 最终值: 507
   热点键 hot_3 最终值: 495
   热点键 hot_4 最终值: 512

    所有热点键数据正确
   冲突率: 1.9%
   吞吐量: 68,342 TPS
`

---

### 3. 内存增长控制测试

**测试目的**  
验证 GC 能有效控制内存增长，防止版本数无限膨胀。

**测试配置**
`ust
键数: 50
迭代次数: 20
每次迭代更新所有键
GC 间隔: 2秒
版本阈值: 500
`

**GC 配置**
`ust
let config = GcConfig {
    max_versions_per_key: 10,
    enable_time_based_gc: false,
    version_ttl_secs: 3600,
    auto_gc: Some(AutoGcConfig {
        interval_secs: 2,
        version_threshold: 500,
        run_on_start: false,
        enable_adaptive: false,
    }),
};
`

**预期结果**
-  版本数稳定在 键数  max_versions_per_key 附近
-  GC 定期执行（至少 2 次）
-  内存不会无限增长
-  GC 清理率 > 10%

**示例输出**
`
 内存增长控制测试

迭代  1: 版本数 =   50, 键数 =  50, 平均版本/键 = 1.00
迭代  2: 版本数 =  100, 键数 =  50, 平均版本/键 = 2.00
迭代  3: 版本数 =  150, 键数 =  50, 平均版本/键 = 3.00
...
迭代 14: 版本数 =  550, 键数 =  50, 平均版本/键 = 11.00   接近阈值
迭代 15: 版本数 =  520, 键数 =  50, 平均版本/键 = 10.40  GC 已清理
迭代 16: 版本数 =  540, 键数 =  50, 平均版本/键 = 10.80
...

 测试完成
   最终版本数: 650 (预期最大: 750)
   最终键数: 50
   GC 执行次数: 3
   GC 清理版本: 450
`

---

### 4. 自适应 GC 测试

**测试目的**  
验证自适应 GC 能根据负载动态调整参数。

**测试阶段**
1. **阶段 1 - 高负载写入**: 快速产生大量版本
2. **阶段 2 - 观察调整**: 等待自适应 GC 响应
3. **阶段 3 - 低负载**: 验证参数回归

**GC 配置**
`ust
let config = GcConfig {
    max_versions_per_key: 20,
    enable_time_based_gc: false,
    version_ttl_secs: 3600,
    auto_gc: Some(AutoGcConfig {
        interval_secs: 10,       // 初始间隔
        version_threshold: 1000,
        run_on_start: false,
        enable_adaptive: true,   //  启用自适应
    }),
};
`

**预期结果**
-  高负载时 GC 更频繁
-  版本数被有效控制
-  低负载时 GC 频率降低
-  参数动态调整可观测

**示例输出**
`
 自适应 GC 测试
   初始间隔: 10 秒
   初始阈值: 1000 版本
   自适应模式: 已启用

[阶段 1] 高负载写入...
   写入 5000 个版本
   版本数: 4850
   GC 次数: 1
   清理版本: 150

[阶段 2] 等待自适应调整...
   3s: 版本数 = 4820, GC 次数 = 2
   6s: 版本数 = 4790, GC 次数 = 3  GC 间隔缩短
   9s: 版本数 = 4760, GC 次数 = 4
   12s: 版本数 = 4730, GC 次数 = 5
   15s: 版本数 = 4700, GC 次数 = 6

[阶段 3] 低负载写入...
   写入 10 个版本

 测试完成
   最终版本数: 4510
   总 GC 次数: 8
   总清理版本: 490
    自适应 GC 根据负载自动调整了参数
`

---

### 5. 长时间稳定性测试

**测试目的**  
验证长时间运行的稳定性和内存稳定性。

**测试配置**
`ust
运行时长: 60秒 (可配置为数小时)
线程数: 4
键数: 200
读写比例: 50/50
`

**监控指标**
-  实时 TPS
-  版本数变化
-  GC 执行频率
-  内存占用趋势

**示例输出**
`
  长时间稳定性测试 (60 秒)

[10s]  TPS: 5810, 版本数: 58301, 键数: 200, GC 次数: 0, 成功: 58100
[20s]  TPS: 5824, 版本数: 58302, 键数: 200, GC 次数: 1, 成功: 116480
[30s]  TPS: 5831, 版本数: 58305, 键数: 200, GC 次数: 2, 成功: 174930
[40s]  TPS: 5819, 版本数: 58298, 键数: 200, GC 次数: 3, 成功: 232760
[50s]  TPS: 5827, 版本数: 58301, 键数: 200, GC 次数: 4, 成功: 291375
[60s]  TPS: 5833, 版本数: 58299, 键数: 200, GC 次数: 5, 成功: 349980

 稳定性测试完成
   平均 TPS: 5824.2
   TPS 标准差: 8.4 (稳定)
   最终版本数: 58299
   版本数波动: < 0.1% (稳定)
   GC 执行: 5 次 (均匀分布)
`

---

## GC 配置参数

### GcConfig 结构

`ust
pub struct GcConfig {
    /// 每个键最多保留的版本数
    pub max_versions_per_key: usize,
    
    /// 是否启用基于时间的 GC
    pub enable_time_based_gc: bool,
    
    /// 版本过期时间（秒）
    pub version_ttl_secs: u64,
    
    /// 自动 GC 配置
    pub auto_gc: Option<AutoGcConfig>,
}
`

### AutoGcConfig 结构

`ust
pub struct AutoGcConfig {
    /// GC 间隔时间（秒）
    pub interval_secs: u64,
    
    /// 版本数阈值：超过此数量触发 GC（0 表示不启用）
    pub version_threshold: usize,
    
    /// 是否在启动时立即执行一次 GC
    pub run_on_start: bool,
    
    /// 是否启用自适应 GC（根据负载动态调整参数）
    pub enable_adaptive: bool,
}
`

### 参数说明

| 参数 | 默认值 | 说明 | 调优建议 |
|------|--------|------|---------|
| max_versions_per_key | 10 | 每个键保留的最大版本数 | 增加可减少冲突，但会占用更多内存 |
| interval_secs | 60 | GC 执行间隔（秒） | 缩短可更及时回收，但会增加 CPU 开销 |
| ersion_threshold | 1000 | 触发 GC 的版本数阈值 | 降低可更激进回收，提高可延迟触发 |
| un_on_start | false | 启动时立即执行 GC | 适合从快照恢复的场景 |
| enable_adaptive | false | 启用自适应调整 | 推荐启用，自动优化参数 |

---

## 自适应 GC

### 什么是自适应 GC？

自适应 GC 根据系统负载和内存压力动态调整 GC 参数，无需手动调优。

**核心能力**:
-  根据 TPS 和版本增长速度调整间隔
-  根据 GC 清理效率调整阈值
-  负载变化时自动响应
-  长期趋势跟踪和优化

### 配置自适应 GC

`ust
use vm_runtime::{MvccStore, GcConfig, AutoGcConfig};

let config = GcConfig {
    max_versions_per_key: 20,
    enable_time_based_gc: false,
    version_ttl_secs: 3600,
    auto_gc: Some(AutoGcConfig {
        interval_secs: 60,          // 基准间隔
        version_threshold: 1000,    // 基准阈值
        run_on_start: false,
        enable_adaptive: true,      //  启用自适应
    }),
};

let store = MvccStore::new_with_config(config);
`

### 自适应策略

`ust
pub struct AdaptiveGcStrategy {
    pub base_interval_secs: 60,    // 基准间隔
    pub min_interval_secs: 10,     // 最小间隔（高负载）
    pub max_interval_secs: 300,    // 最大间隔（低负载）
    pub base_threshold: 1000,      // 基准阈值
    pub min_threshold: 500,        // 最小阈值（更激进）
    pub max_threshold: 5000,       // 最大阈值（更宽松）
}
`

### 调整逻辑

#### 1. 高负载检测

**触发条件** (满足任一):
- 高 TPS (近期事务数 > 阈值)
- 版本快速增长 (增长率 > 10%/周期)

**调整策略**:
-  缩短 GC 间隔 (最低至 min_interval_secs)
-  降低触发阈值 (最低至 min_threshold)
-  结果: 更频繁、更激进的 GC

#### 2. 低效 GC 检测

**触发条件**:
- GC 清理率 < 10% (清理版本数/总版本数)
- 连续 3 次低效 GC

**调整策略**:
-  延长 GC 间隔 (最高至 max_interval_secs)
-  提高触发阈值 (最高至 max_threshold)
-  结果: 减少无效 GC，节省资源

#### 3. 正常负载

**触发条件**:
- TPS 适中
- 版本增长平稳
- GC 清理效率正常 (> 10%)

**调整策略**:
-  逐渐回归基准值
-  结果: 保持稳定状态

### 运行时观察

**查看当前 GC 参数** (v0.8.0+):

`ust
if let Some(runtime) = store.get_auto_gc_runtime() {
    println!("自适应模式: {}", runtime.enable_adaptive);
    println!("当前间隔: {} 秒", runtime.interval_secs);
    println!("当前阈值: {} 个版本", runtime.version_threshold);
}
`

**结合 GC 统计评估效果**:

`ust
let stats = store.get_gc_stats();
let runtime = store.get_auto_gc_runtime().unwrap();

println!("");
println!("      自适应 GC 运行状态               ");
println!("");
println!(" GC 执行次数:   {:>10}        ", stats.gc_count);
println!(" 清理版本数:   {:>10}        ", stats.versions_cleaned);
println!(" 当前间隔:     {:>10} 秒     ", runtime.interval_secs);
println!(" 当前阈值:     {:>10} 版本   ", runtime.version_threshold);
println!("");

// 计算清理效率
if stats.gc_count > 0 {
    let avg_cleaned = stats.versions_cleaned / stats.gc_count;
    println!("平均每次清理: {} 版本", avg_cleaned);
}
`

>  详细说明请参见 [GC 运行时可观测性文档](gc-observability.md)

---

## 性能调优建议

### 基于测试结果调优

#### 场景 1: 高吞吐量要求

**业务需求**: 最大化 TPS，内存充足

**推荐配置**:
`ust
GcConfig {
    max_versions_per_key: 30,      //  增加版本限制
    enable_time_based_gc: false,
    version_ttl_secs: 3600,
    auto_gc: Some(AutoGcConfig {
        interval_secs: 120,        //  延长间隔
        version_threshold: 2000,   //  提高阈值
        run_on_start: false,
        enable_adaptive: true,     //  启用自适应
    }),
}
`

**调优原理**: 
- 减少 GC 频率，降低 GC 开销
- 保留更多版本，减少冲突重试
- 自适应机制应对负载波动

**预期效果**:
-  TPS 提升 10-20%
-  内存占用增加 20-30%

---

#### 场景 2: 内存敏感场景

**业务需求**: 内存有限，需要严格控制

**推荐配置**:
`ust
GcConfig {
    max_versions_per_key: 10,      //  降低版本限制
    enable_time_based_gc: false,
    version_ttl_secs: 3600,
    auto_gc: Some(AutoGcConfig {
        interval_secs: 30,         //  缩短间隔
        version_threshold: 500,    //  降低阈值
        run_on_start: false,
        enable_adaptive: true,     //  启用自适应
    }),
}
`

**调优原理**:
- 更激进的 GC，快速回收内存
- 严格控制版本数量
- 自适应避免过度 GC

**预期效果**:
-  内存占用降低 40-50%
-  TPS 可能下降 5-10%

---

#### 场景 3: 高冲突场景

**业务需求**: 存在热点键，冲突率高

**推荐配置**:
`ust
GcConfig {
    max_versions_per_key: 50,      //  热点键需要更多版本
    enable_time_based_gc: false,
    version_ttl_secs: 3600,
    auto_gc: Some(AutoGcConfig {
        interval_secs: 60,
        version_threshold: 1000,
        run_on_start: false,
        enable_adaptive: true,     //  让自适应处理
    }),
}
`

**应用层优化**:
`ust
// 1. 键分片（减少热点）
let shard = hash(key) % num_shards;
let sharded_key = format!("{}_{}", key, shard);

// 2. 批量操作（减少冲突）
let keys: Vec<_> = (0..100).map(|i| format!("key_{}", i)).collect();
batch_update(&store, &keys);

// 3. 只读事务（使用快速路径）
let ro_txn = store.begin_read_only(); // 不需要冲突检测
let value = ro_txn.read(b"hot_key")?;
ro_txn.commit()?; // 直接返回，无开销
`

**预期效果**:
-  冲突率降低 50-70%
-  成功率提升至 > 99%

---

#### 场景 4: 长事务场景

**业务需求**: 有长时间运行的事务（如报表生成）

**推荐配置**:
`ust
GcConfig {
    max_versions_per_key: 100,     //  大幅增加
    enable_time_based_gc: false,
    version_ttl_secs: 3600,
    auto_gc: Some(AutoGcConfig {
        interval_secs: 180,        //  延长间隔
        version_threshold: 5000,   //  提高阈值
        run_on_start: false,
        enable_adaptive: false,    //  禁用（避免误判）
    }),
}
`

**调优原理**:
- 保留足够多的版本供长事务读取
- 避免 GC 过早清理长事务需要的版本
- 禁用自适应防止误判为低效 GC

**预期效果**:
-  长事务不会因版本被清理而失败
-  内存占用显著增加

---

### 监控指标基准

运行压力测试时关注以下指标:

#### 1. 吞吐量 (TPS)

| 场景 | 目标 TPS | 优秀 | 良好 | 需优化 |
|------|---------|------|------|--------|
| 高并发混合 | > 100K | > 500K | 100K-500K | < 100K |
| 高冲突热点 | > 50K | > 200K | 50K-200K | < 50K |
| 长时间稳定 | 稳定 | 波动 < 5% | 波动 < 10% | 波动 > 10% |

#### 2. 延迟

| 指标 | 优秀 | 良好 | 需优化 |
|------|------|------|--------|
| 平均延迟 | < 10 μs | 10-20 μs | > 20 μs |
| P99 延迟 | < 50 μs | 50-100 μs | > 100 μs |
| P999 延迟 | < 200 μs | 200-500 μs | > 500 μs |

#### 3. 成功率

| 场景 | 目标成功率 | 说明 |
|------|-----------|------|
| 混合负载 | > 99% | 低冲突场景应该接近 100% |
| 高冲突 | > 95% | 热点键会导致更多冲突 |
| 长事务 | > 98% | 避免版本被清理导致失败 |

#### 4. 内存

| 指标 | 计算公式 | 健康范围 |
|------|----------|---------|
| 版本数 | 	otal_versions() | < 键数  max_versions  1.5 |
| 平均版本/键 | versions / keys | < max_versions_per_key  1.2 |
| GC 清理率 | cleaned / (cleaned + versions) | > 10% |

---

## 故障排查

### 问题 1: 内存持续增长

**症状**  
版本数不断增加，GC 执行但无效。

**诊断步骤**

`ust
// 1. 检查 GC 是否启用
if store.get_auto_gc_runtime().is_none() {
    println!(" 未启用自动 GC");
}

// 2. 检查 GC 统计
let stats = store.get_gc_stats();
println!("GC 执行次数: {}", stats.gc_count);
println!("GC 清理版本: {}", stats.versions_cleaned);

if stats.gc_count > 0 && stats.versions_cleaned == 0 {
    println!("  GC 执行但未清理任何版本");
}

// 3. 检查活跃事务
let min_ts = store.get_min_active_ts();
println!("最小活跃事务 TS: {:?}", min_ts);

if min_ts.is_some() {
    println!("  有长时间未提交的事务阻止 GC");
}

// 4. 查看版本分布
let versions = store.total_versions();
let keys = store.total_keys();
let avg = versions as f64 / keys as f64;
println!("平均版本/键: {:.2}", avg);
`

**解决方案**

`ust
// 方案 1: 调整 GC 参数
let config = GcConfig {
    max_versions_per_key: 10,    //  降低限制
    auto_gc: Some(AutoGcConfig {
        interval_secs: 30,       //  缩短间隔
        version_threshold: 500,  //  降低阈值
        enable_adaptive: true,
        ..Default::default()
    }),
    ..Default::default()
};
store.set_gc_config(config);

// 方案 2: 立即手动触发 GC
store.gc().unwrap();

// 方案 3: 确保所有事务及时提交
// 检查代码中是否有未提交的事务
`

---

### 问题 2: 吞吐量低于预期

**症状**  
TPS 远低于基准值，系统响应缓慢。

**诊断步骤**

`ust
// 1. 检查 GC 频率
let stats = store.get_gc_stats();
let runtime = store.get_auto_gc_runtime().unwrap();

let gc_per_minute = stats.gc_count as f64 / (elapsed_secs / 60.0);
println!("GC 频率: {:.2} 次/分钟", gc_per_minute);

if gc_per_minute > 10.0 {
    println!("  GC 过于频繁，影响性能");
}

// 2. 检查冲突率
let conflict_rate = failed_txs as f64 / total_txs as f64 * 100.0;
println!("冲突率: {:.2}%", conflict_rate);

if conflict_rate > 5.0 {
    println!("  冲突率过高");
}

// 3. 检查延迟分布
println!("P50 延迟: {:.2} μs", p50_latency);
println!("P99 延迟: {:.2} μs", p99_latency);

if p99_latency > p50_latency * 10.0 {
    println!("  延迟分布不均，存在长尾");
}
`

**解决方案**

`ust
// 方案 1: 减少 GC 频率
let config = GcConfig {
    max_versions_per_key: 30,
    auto_gc: Some(AutoGcConfig {
        interval_secs: 120,      //  延长间隔
        version_threshold: 2000, //  提高阈值
        enable_adaptive: true,
        ..Default::default()
    }),
    ..Default::default()
};

// 方案 2: 使用只读事务
let ro_txn = store.begin_read_only();
// 只读事务无需冲突检测，性能更高

// 方案 3: 优化事务粒度
// 将大事务拆分为多个小事务
for chunk in data.chunks(100) {
    let mut txn = store.begin();
    for item in chunk {
        txn.write(item.key, item.value);
    }
    txn.commit()?;
}
`

---

### 问题 3: 高冲突率

**症状**  
大量事务因冲突失败，重试次数多。

**诊断步骤**

`ust
// 1. 识别热点键
let mut key_access_count = HashMap::new();
for txn in &transactions {
    for key in &txn.write_set {
        *key_access_count.entry(key).or_insert(0) += 1;
    }
}

let mut hotspots: Vec<_> = key_access_count.into_iter().collect();
hotspots.sort_by_key(|(_, count)| std::cmp::Reverse(*count));

println!("Top 10 热点键:");
for (i, (key, count)) in hotspots.iter().take(10).enumerate() {
    println!("{}. {} - {} 次访问", i+1, String::from_utf8_lossy(key), count);
}

// 2. 分析冲突模式
let conflict_rate = conflicts as f64 / total_txs as f64;
println!("总冲突率: {:.2}%", conflict_rate * 100.0);

// 3. 检查版本数
let versions_per_key = store.total_versions() / store.total_keys();
println!("平均版本/键: {}", versions_per_key);
`

**解决方案**

`ust
// 方案 1: 增加版本数限制
let config = GcConfig {
    max_versions_per_key: 50,  //  热点键需要更多版本
    ..Default::default()
};

// 方案 2: 键分片
fn shard_key(key: &str, num_shards: usize) -> Vec<u8> {
    let hash = hash_function(key);
    let shard = hash % num_shards;
    format!("{}_{}", key, shard).into_bytes()
}

// 使用
let sharded_key = shard_key("hot_key", 10);
txn.write(sharded_key, value);

// 方案 3: 应用层队列
// 对热点键的写入通过队列串行化
let queue = Arc::new(Mutex::new(VecDeque::new()));
// 单独线程处理队列，避免并发冲突

// 方案 4: 读写分离
// 对热点键使用只读事务
let ro_txn = store.begin_read_only();
let value = ro_txn.read(b"hot_key")?; // 不会冲突
`

---

### 问题 4: P99 延迟高

**症状**  
平均延迟正常，但 P99/P999 很高。

**诊断步骤**

`ust
// 1. 记录 GC 执行时刻
let mut gc_timestamps = Vec::new();
let mut last_gc_count = 0;

loop {
    let stats = store.get_gc_stats();
    if stats.gc_count > last_gc_count {
        gc_timestamps.push(Instant::now());
        last_gc_count = stats.gc_count;
    }
    thread::sleep(Duration::from_millis(100));
}

// 2. 关联延迟峰值与 GC
// 检查高延迟请求是否发生在 GC 期间

// 3. 分析版本链长度
for entry in store.data.iter() {
    let versions = entry.value().read();
    if versions.len() > 20 {
        println!("键 {:?} 版本链过长: {} 个版本",
            String::from_utf8_lossy(entry.key()),
            versions.len()
        );
    }
}
`

**解决方案**

`ust
// 方案 1: 更激进的 GC
let config = GcConfig {
    max_versions_per_key: 15,    //  减少版本限制
    auto_gc: Some(AutoGcConfig {
        interval_secs: 45,       //  缩短间隔
        version_threshold: 800,  //  降低阈值
        enable_adaptive: true,
        ..Default::default()
    }),
    ..Default::default()
};

// 方案 2: 监控 GC 暂停时间
let gc_start = Instant::now();
store.gc()?;
let gc_duration = gc_start.elapsed();
println!("GC 耗时: {:.2} ms", gc_duration.as_micros() as f64 / 1000.0);

if gc_duration.as_millis() > 10 {
    println!("  GC 暂停时间过长");
}

// 方案 3: 异步 GC（未来特性）
// 在后台线程执行 GC，减少对前台事务的影响
`

---

## 监控最佳实践

### 1. 生产环境监控

建议在生产环境中持续监控以下指标:

`ust
use std::time::{Duration, Instant};
use std::thread;

fn production_monitor(store: Arc<MvccStore>) {
    thread::spawn(move || {
        let mut last_check = Instant::now();
        let mut last_stats = store.get_gc_stats();
        
        loop {
            thread::sleep(Duration::from_secs(30));
            
            let stats = store.get_gc_stats();
            let versions = store.total_versions();
            let keys = store.total_keys();
            let elapsed = last_check.elapsed().as_secs_f64();
            
            // 计算速率
            let gc_rate = (stats.gc_count - last_stats.gc_count) as f64 / elapsed * 60.0;
            let clean_rate = (stats.versions_cleaned - last_stats.versions_cleaned) as f64 / elapsed;
            
            // 输出到监控系统（如 Prometheus）
            log::info!(
                "MVCC Monitor: versions={}, keys={}, gc_rate={:.2}/min, clean_rate={:.2}/s",
                versions, keys, gc_rate, clean_rate
            );
            
            // 告警检查
            if versions > 100_000 {
                log::warn!("版本数过高: {}", versions);
            }
            
            if gc_rate > 20.0 {
                log::warn!("GC 频率过高: {:.2} 次/分钟", gc_rate);
            }
            
            if let Some(runtime) = store.get_auto_gc_runtime() {
                log::info!(
                    "Adaptive GC: interval={}s, threshold={}",
                    runtime.interval_secs,
                    runtime.version_threshold
                );
            }
            
            last_check = Instant::now();
            last_stats = stats;
        }
    });
}

// 启动监控
production_monitor(Arc::clone(&store));
`

### 2. 压力测试监控

在压力测试期间，实时输出详细统计:

`ust
fn stress_test_monitor(store: Arc<MvccStore>, duration: Duration) {
    let start = Instant::now();
    let mut last_check = Instant::now();
    
    while start.elapsed() < duration {
        thread::sleep(Duration::from_secs(5));
        
        let stats = store.get_gc_stats();
        let versions = store.total_versions();
        let keys = store.total_keys();
        let elapsed = start.elapsed().as_secs_f64();
        
        println!(
            "[{:>5.1}s] 版本数: {:>6}, 键数: {:>4}, GC 次数: {:>3}, 清理版本: {:>6}",
            elapsed, versions, keys, stats.gc_count, stats.versions_cleaned
        );
        
        // 检查异常
        let avg_versions = versions as f64 / keys as f64;
        if avg_versions > 50.0 {
            println!("        平均版本/键过高: {:.2}", avg_versions);
        }
        
        last_check = Instant::now();
    }
}
`

### 3. 性能基准记录

定期运行基准测试并记录结果:

`ust
// 创建基准测试记录
struct BenchmarkResult {
    date: String,
    version: String,
    tps: f64,
    avg_latency_us: f64,
    p99_latency_us: f64,
    success_rate: f64,
}

fn record_benchmark(result: BenchmarkResult) {
    let json = serde_json::to_string(&result).unwrap();
    
    // 追加到文件
    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open("benchmark_history.jsonl")
        .unwrap();
    
    writeln!(file, "{}", json).unwrap();
}

// 比较历史基准
fn compare_with_baseline(current: &BenchmarkResult, baseline: &BenchmarkResult) {
    let tps_change = (current.tps - baseline.tps) / baseline.tps * 100.0;
    let latency_change = (current.avg_latency_us - baseline.avg_latency_us) 
        / baseline.avg_latency_us * 100.0;
    
    println!("与基准对比:");
    println!("  TPS: {:+.1}%", tps_change);
    println!("  延迟: {:+.1}%", latency_change);
    
    if tps_change < -10.0 {
        println!("    性能显著下降！");
    }
}
`

---

## 总结

### 关键要点

 **压力测试是必需的**  
在生产环境部署前，务必运行完整的压力测试套件，确保系统能够承受预期负载。

 **自适应 GC 是首选**  
对于大多数场景，启用自适应 GC 可以自动优化性能，减少手动调优工作。

 **监控是关键**  
持续监控 TPS、延迟、版本数、GC 统计等指标，及时发现和解决问题。

 **根据工作负载调优**  
不同场景需要不同的配置，使用本指南作为起点，根据实际情况调整参数。

 **长期跟踪基准**  
定期运行基准测试并记录结果，用于性能回归检测和优化效果评估。

### 快速参考

| 优化目标 | 关键参数 | 推荐值 |
|---------|---------|--------|
| 最大化 TPS | max_versions_per_key | 30 |
| | interval_secs | 120 |
| | ersion_threshold | 2000 |
| 最小化内存 | max_versions_per_key | 10 |
| | interval_secs | 30 |
| | ersion_threshold | 500 |
| 降低冲突 | max_versions_per_key | 50 |
| | 应用层分片 |  |
| | 只读事务 |  |
| 长事务支持 | max_versions_per_key | 100 |
| | enable_adaptive | false |

---

## 下一步

-  查看 [GC 运行时可观测性文档](gc-observability.md)
-  查看 [并行执行设计文档](parallel-execution.md)
-  查看 [Demo 示例](../../src/node-core/examples/)
-  查看 [基准测试报告](../../BENCHMARK_RESULTS.md)
-  查看 [API 文档](API.md)

---

 2025 SuperVM Project | GPL-3.0 License
