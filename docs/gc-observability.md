# GC 运行时可观测性

适用版本: v0.8.0+

本文档介绍如何在运行时观察 Auto GC 的关键参数与状态，帮助你在压测或生产中快速诊断与调优。

## 能力概览

- 实时获取当前 GC 周期与阈值（包含自适应调整后的值）
- 确认是否启用了自适应 GC
- 搭配 GC 统计（次数、清理版本数）进行效果评估

## API

通过 `MvccStore::get_auto_gc_runtime()` 获取快照：

```rust
use vm_runtime::{MvccStore, GcConfig, AutoGcConfig};

let store = MvccStore::new_with_config(GcConfig {
    max_versions_per_key: 20,
    enable_time_based_gc: false,
    version_ttl_secs: 3600,
    auto_gc: Some(AutoGcConfig {
        interval_secs: 60,
        version_threshold: 1000,
        run_on_start: true,
        enable_adaptive: true,
    }),
});

if let Some(rt) = store.get_auto_gc_runtime() {
    println!(
        "adaptive: {}, interval_secs: {}, threshold: {}",
        rt.enable_adaptive, rt.interval_secs, rt.version_threshold
    );
}
```

返回结构（只读快照）：

```rust
pub struct AutoGcRuntime {
    pub enable_adaptive: bool,
    pub interval_secs: u64,       // 当前生效的间隔（可能被自适应调整）
    pub version_threshold: u64,   // 当前生效的阈值（可能被自适应调整）
}
```

> 注：当未配置 `auto_gc` 时，`get_auto_gc_runtime()` 返回 `None`。

## 与 GC 统计联合使用

```rust
let stats = store.get_gc_stats();
println!(
    "gc_count={}, versions_cleaned={}, keys_cleaned={}, last_gc_ts={}",
    stats.gc_count, stats.versions_cleaned, stats.keys_cleaned, stats.last_gc_ts
);
```

结合 `AutoGcRuntime` 可以判断：
- 调整后的间隔/阈值是否与期望一致
- 在某个区间内 GC 是否有效（清理率）

## 典型用法：压测观测点

在压测循环或阶段性 sleep 处插入观测：

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
  - 检查 `interval_secs` 是否过大、`version_threshold` 是否过高
  - 确认是否启用了 `enable_adaptive`
- GC 清理率过低：
  - 适当降低阈值或缩短间隔
  - 检查是否存在长事务阻挡历史版本回收

## 相关文档

- 压力测试与调优指南: ./stress-testing-guide.md
- 并行执行设计: ./parallel-execution.md
