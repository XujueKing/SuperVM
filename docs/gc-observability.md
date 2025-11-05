# GC 运行时可观测性

版本: v0.8.0+  
最后更新: 2025-11-05

## 概述

本文档介绍如何在运行时观察 Auto GC 的关键参数与状态，便于监控、调试和性能调优。

SuperVM 的 MVCC 存储系统支持配置式自动垃圾回收（Auto GC），可定期清理不再需要的旧版本数据，控制内存增长。通过运行时可观测性 API，您可以实时监控 GC 行为并调整策略。

---

## 可观测性 API

### 1. get_auto_gc_runtime()

获取当前自动 GC 的运行时参数快照。

**方法签名**:
```rust
pub fn get_auto_gc_runtime(&self) -> Option<AutoGcRuntime>
```

**返回值**:
- `Some(AutoGcRuntime)` - 如果启用了自动 GC
- `None` - 如果未配置自动 GC

**AutoGcRuntime 结构**:
```rust
pub struct AutoGcRuntime {
    /// 是否启用自适应 GC
    pub enable_adaptive: bool,
    /// 当前 GC 间隔时间（秒）
    pub interval_secs: u64,
    /// 当前版本数阈值
    pub version_threshold: usize,
}
```

**用途**:
- 监控自适应 GC 是否根据负载动态调整了参数
- 调试 GC 触发频率问题
- 性能分析时了解当前 GC 策略

**示例**:
```rust
use vm_runtime::MvccStore;

let store = MvccStore::new_with_config(config);

// 查询当前 GC 运行参数
if let Some(runtime) = store.get_auto_gc_runtime() {
    println!("自适应模式: {}", runtime.enable_adaptive);
    println!("GC 间隔: {} 秒", runtime.interval_secs);
    println!("版本阈值: {} 个", runtime.version_threshold);
} else {
    println!("未启用自动 GC");
}
```

---

### 2. get_gc_stats()

获取累计 GC 统计信息。

**方法签名**:
```rust
pub fn get_gc_stats(&self) -> GcStats
```

**返回值**: `GcStats` 结构体

**GcStats 结构**:
```rust
pub struct GcStats {
    /// GC 执行次数
    pub gc_count: u64,
    /// 清理的版本总数
    pub versions_cleaned: u64,
    /// 清理的键总数
    pub keys_cleaned: u64,
    /// 最后一次 GC 时间戳
    pub last_gc_ts: u64,
}
```

**用途**:
- 监控 GC 执行频率
- 评估 GC 回收效果
- 诊断内存泄漏或版本堆积问题

**示例**:
```rust
let stats = store.get_gc_stats();

println!("╔═══════════════════════════════════════╗");
println!("║         GC 统计报告                   ║");
println!("╠═══════════════════════════════════════╣");
println!("║ 执行次数:   {:>10}            ║", stats.gc_count);
println!("║ 清理版本:   {:>10}            ║", stats.versions_cleaned);
println!("║ 清理键数:   {:>10}            ║", stats.keys_cleaned);
println!("║ 最后执行:   {:>10}            ║", stats.last_gc_ts);
println!("╚═══════════════════════════════════════╝");
```

---

### 3. total_versions() / total_keys()

获取当前存储的总版本数和键数。

**方法签名**:
```rust
pub fn total_versions(&self) -> usize
pub fn total_keys(&self) -> usize
```

**用途**:
- 监控内存使用情况
- 判断 GC 是否有效控制版本增长
- 压力测试时的关键指标

**示例**:
```rust
let versions = store.total_versions();
let keys = store.total_keys();

println!("当前版本数: {}", versions);
println!("当前键数: {}", keys);

// 计算平均版本数
let avg_versions = if keys > 0 {
    versions as f64 / keys as f64
} else {
    0.0
};
println!("平均版本/键: {:.2}", avg_versions);
```

---

## 监控最佳实践

### 1. 定期轮询监控

在生产环境中，建议定期采集 GC 统计信息：

```rust
use std::thread;
use std::time::Duration;

fn gc_monitor_loop(store: Arc<MvccStore>) {
    thread::spawn(move || {
        loop {
            thread::sleep(Duration::from_secs(30));
            
            let stats = store.get_gc_stats();
            let versions = store.total_versions();
            let keys = store.total_keys();
            
            // 输出到监控系统
            log::info!("GC Stats: count={}, cleaned={}, versions={}, keys={}",
                stats.gc_count, stats.versions_cleaned, versions, keys);
            
            // 检查异常情况
            if versions > 100_000 {
                log::warn!("版本数过高: {}", versions);
            }
        }
    });
}
```

### 2. 自适应 GC 观察

启用自适应 GC 时，观察参数变化：

```rust
let mut last_interval = 0u64;

loop {
    thread::sleep(Duration::from_secs(10));
    
    if let Some(runtime) = store.get_auto_gc_runtime() {
        if runtime.interval_secs != last_interval {
            println!("⚡ GC 间隔已调整: {} -> {} 秒",
                last_interval, runtime.interval_secs);
            last_interval = runtime.interval_secs;
        }
    }
}
```

### 3. 压力测试监控

在压力测试中，持续监控内存与 GC 效果：

```rust
let start = Instant::now();
let mut last_check = Instant::now();

// 压力测试循环
for i in 0..100_000 {
    // 执行事务...
    
    // 每 5 秒检查一次
    if last_check.elapsed().as_secs() >= 5 {
        let versions = store.total_versions();
        let stats = store.get_gc_stats();
        
        println!("[{:.1}s] 版本数: {}, GC 次数: {}, 清理: {}",
            start.elapsed().as_secs_f64(),
            versions,
            stats.gc_count,
            stats.versions_cleaned
        );
        
        last_check = Instant::now();
    }
}
```

---

## 调试场景

### 场景 1: GC 未按预期触发

**症状**: 版本数持续增长，GC 执行次数为 0

**诊断**:
```rust
// 1. 检查是否启用了自动 GC
if store.get_auto_gc_runtime().is_none() {
    println!("❌ 未启用自动 GC");
}

// 2. 检查配置参数
if let Some(runtime) = store.get_auto_gc_runtime() {
    println!("间隔: {} 秒", runtime.interval_secs);
    println!("阈值: {} 版本", runtime.version_threshold);
    
    let versions = store.total_versions();
    if versions < runtime.version_threshold {
        println!("版本数 {} 未达阈值 {}", versions, runtime.version_threshold);
    }
}
```

### 场景 2: GC 清理效果不佳

**症状**: GC 执行多次，但版本数仍然很高

**诊断**:
```rust
let stats = store.get_gc_stats();
let versions = store.total_versions();

println!("GC 执行次数: {}", stats.gc_count);
println!("清理版本总数: {}", stats.versions_cleaned);
println!("当前版本数: {}", versions);

if stats.gc_count > 0 {
    let avg_cleaned = stats.versions_cleaned / stats.gc_count;
    println!("平均每次清理: {} 版本", avg_cleaned);
    
    if avg_cleaned < 10 {
        println!("⚠️  清理效果不佳，可能是:");
        println!("   - 有大量活跃事务阻止清理");
        println!("   - max_versions_per_key 设置过高");
        println!("   - 写入速度超过 GC 清理速度");
    }
}
```

### 场景 3: 性能抖动

**症状**: TPS 周期性下降

**诊断**:
```rust
// 记录 GC 执行时刻
let mut last_gc_count = 0u64;

loop {
    thread::sleep(Duration::from_secs(1));
    
    let stats = store.get_gc_stats();
    if stats.gc_count > last_gc_count {
        println!("⏱️  GC 执行于 {} 时刻", Instant::now());
        last_gc_count = stats.gc_count;
        
        // 与 TPS 监控关联，确定 GC 是否导致性能下降
    }
    }
}
```

---

## 配置调优建议

基于监控数据，调整 GC 配置：

### 1. 版本数过高

如果 `total_versions()` 持续增长：
- 减小 `interval_secs`（更频繁 GC）
- 降低 `version_threshold`（更早触发）
- 减小 `max_versions_per_key`（保留更少版本）

### 2. GC 过于频繁

如果 `gc_count` 增长很快但版本数很少：
- 增大 `interval_secs`
- 提高 `version_threshold`
- 启用 `enable_adaptive` 自动调节

### 3. 自适应调优

启用自适应 GC 后，系统会根据负载自动调整参数。监控 `get_auto_gc_runtime()` 返回的参数变化，确认自适应策略是否符合预期。

---

## 参考

- **配置详解**: 参见 `stress-testing-guide.md` 中的"GC 配置参数"章节
- **压力测试**: 参见 `src/vm-runtime/tests/mvcc_stress_test.rs`
- **API 文档**: 参见 `docs/API.md`

---

© 2025 SuperVM Project | GPL-3.0 License

## �?GC 统计联合使用

```rust
let stats = store.get_gc_stats();
println!(
    "gc_count={}, versions_cleaned={}, keys_cleaned={}, last_gc_ts={}",
    stats.gc_count, stats.versions_cleaned, stats.keys_cleaned, stats.last_gc_ts
);
```

结合 `AutoGcRuntime` 可以判断�?
- 调整后的间隔/阈值是否与期望一�?
- 在某个区间内 GC 是否有效（清理率�?

## 典型用法：压测观测点

在压测循环或阶段�?sleep 处插入观测：

```rust
if let Some(rt) = store.get_auto_gc_runtime() {
    let stats = store.get_gc_stats();
    println!(
        "[obs] interval={}s, threshold={}, gc_count={}, cleaned_versions={}",
        rt.interval_secs, rt.version_threshold, stats.gc_count, stats.versions_cleaned
    );
}
```

## 排错建议

- 版本数居高不下：
  - 检�?`interval_secs` 是否过大、`version_threshold` 是否过高
  - 确认是否启用�?`enable_adaptive`
- GC 清理率过低：
  - 适当降低阈值或缩短间隔
  - 检查是否存在长事务阻挡历史版本回收

## 相关文档

- 压力测试与调优指�? ./stress-testing-guide.md
- 并行执行设计: ./parallel-execution.md




