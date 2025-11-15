# Autoscale Orchestrator v0

目标：在混合模式下（FullNode / LightClient / ComputeOnly / StorageProxy / Hybrid）根据四层网络指标自动切换链子模块的运行模式。

本版本（v0）的范围：

- 仅定义指标采集与上报接口

- 不做复杂策略，仅提供阈值占位与决策钩子

- 输出 Prometheus 友好指标和本地事件总线事件

## 指标

- catch_up_lag_blocks: 与主网最新高度的滞后区块数

- cpu_load: 子模块进程 CPU 使用率（0-1）

- storage_pressure: 存储压力（已用/总量）

- cache_hit_rate: 镜像层热点命中率（0-1）

## 决策钩子（示例）

- lag > 阈值 → 提升为 FullNode 或增加同步线程

- cpu_load > 阈值 → 降级 Compute workers 或延迟任务

- storage_pressure > 阈值 → 触发 L1/L2 归档或切换 StorageProxy

## 输出

- docs/autoscale/metrics.md：指标字段定义

- docs/autoscale/interfaces.md：采集/上报接口示例
