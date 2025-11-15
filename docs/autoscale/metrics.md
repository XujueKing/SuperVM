# Autoscale Metrics Schema (v0)

字段 | 类型 | 说明
----|------|----
chain_id | string | 链标识（bitcoin/ethereum/...）
height | uint64 | 当前本地高度
catch_up_lag_blocks | uint64 | 与主网最新高度的滞后
cpu_load | float | 0.0-1.0
storage_pressure | float | 0.0-1.0（已用/总量）
cache_hit_rate | float | 0.0-1.0
updated_at | timestamp | 采集时间

备注：本 schema 仅用于 v0 原型，后续将补充网络带宽、延迟、磁盘IO等指标。