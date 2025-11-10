# Autoscale Interfaces (v0)

## 采集接口（示意）

```rust
pub struct MetricsSnapshot {
    pub chain_id: String,
    pub height: u64,
    pub catch_up_lag_blocks: u64,
    pub cpu_load: f32,
    pub storage_pressure: f32,
    pub cache_hit_rate: f32,
    pub updated_at: i64,
}

pub trait MetricsCollector {
    fn collect(&self) -> anyhow::Result<MetricsSnapshot>;
}
```

## 决策钩子（示意）

```rust
pub enum Mode { FullNode, LightClient, ComputeOnly, StorageProxy, Hybrid }

pub trait Autoscaler {
    fn evaluate(&self, m: &MetricsSnapshot) -> Mode;
}
```

## 上报/导出
- Prometheus Exporter（/metrics）
- 本地事件总线（autoscale.mode.changed）
