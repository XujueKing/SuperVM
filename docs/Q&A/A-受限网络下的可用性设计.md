# 受限网络下的可用性设计（合规版）

> 本指南旨在在“合法合规”前提下，提升 SuperVM 在受限或不稳定网络环境中的可用性、隐私最小化与可观测性，不涉及规避或绕开监管的行为或建议。

## 1. 设计目标

- 可用性：网络受限或丢包时，系统不致停摆，用户仍可读取、排队提交与恢复后对账
- 合规性：支持区域策略、数据驻留、保留期与审计，默认最小化元数据
- 可观测性：在不泄露敏感信息的前提下，提供可追溯与 SLA 监测

## 2. 灰度降级策略

- 只读模式（ReadOnly）：上游不可达时，对外只提供读取接口；修改请求进入队列
- 离线队列（Store-and-Forward）：
  - L4：将交易/操作排队到本地持久化队列（SQLite/嵌入式 KV）
  - L3：区域只读缓存命中（不出广域网），减少跨域依赖
  - 恢复后幂等回放（基于幂等键，重复提交无副作用）
- 优雅超时：指数退避、快速失败与用户友好提示

## 3. 跨层协同（L4↔L3↔L2/L1）

- L4：
  - 本地缓存与签名、离线队列、局域协作（L4↔L4 在 LAN 内同步）
  - 与 L3 断连时进入只读 + 队列模式
- L3：
  - 区域热点只读缓存（命中 80-95%），提供低延迟查询
  - 与 L2 连通恢复后补齐缓存
- L2/L1：
  - 可配置的数据驻留与裁剪策略；跨域访问需授权与审计

## 4. 策略与配置（示例）

```toml
[compliance]
mode = "regional"                 # enterprise|regional|global
geo_fencing = ["CN", "!EU"]       # 允许/禁止区域（示例）
metadata_minimization = "strict"   # strict|standard
retention_days = 7

[data_residency]
required_region = "CN-North"
cross_region_write = false

[network.policy]
fallback_order = ["lan", "regional", "global"]
allowed_transports = ["tcp", "tls", "websocket"]
rate_limit_bps = 1_048_576
burst_bytes = 262_144

[degrade]
read_only_on_unreachable = true
offline_queue = true
max_queue_age_min = 1440
idempotent_keys = "sha256(tx)"

[observability]
audit_log = true
pii_redaction = "on"
```

## 5. 最小 API 骨架（文档示例）

```rust
pub enum Decision { Allow, Deny { reason: String }, Degrade(DegradeMode) }

pub enum DegradeMode { Normal, ReadOnly, QueueOnly }

pub trait PolicyEngine {
    fn decide_write(&self, region: &str, key: &str) -> Decision;
    fn decide_transport(&self, t: &str) -> Decision; // tcp/tls/ws
}

pub trait TransportAdapter {
    fn name(&self) -> &'static str;
    fn is_allowed(&self, policy: &dyn PolicyEngine) -> bool;
    fn send(&self, bytes: &[u8]) -> anyhow::Result<()>;
}

pub struct OfflineQueue {
    pub max_age: std::time::Duration,
}

impl OfflineQueue {
    pub fn enqueue(&self, idempotent_key: &[u8], item: Vec<u8>) -> anyhow::Result<()> { Ok(()) }
    pub async fn replay(&self) -> anyhow::Result<()> { Ok(()) }
}
```

## 6. 幂等与对账

- 幂等键：`sha256(tx)` 或业务定义的复合键，保证重复提交只生效一次
- 回放流程：
  1. 出队前先本地幂等查重
  2. 上游 ACK 失败则重试（最大 N 次 + 指数退避）
  3. 成功后写入对账表，更新本地状态
- 对账：
  - L4 与 L3/L2 定期对账（Bloom 过滤器快速差异发现）
  - 异常条目人工或批量修复

## 7. 可观测性与审计

- 指标（Prometheus）：
  - `availability_percent`, `degrade_count`, `offline_queue_depth`, `replay_lag_seconds`
  - `regional_hit_ratio`, `cross_region_denied_total`
- 日志：
  - 审计日志默认脱敏（PII/密钥不落盘）
  - 策略拒绝的原因码与上下文可审计

## 8. 测试场景清单（E2E）

- A：上游阻断 → L4 切只读+队列；恢复后回放成功且无重复副作用
- B：跨域写被策略禁止 → 本地拒绝并记录审计原因
- C：允许传输中断 → 在白名单内切换到备选传输
- D：速率整形 → 峰值被平滑且不触发上游丢包
- E：区域只读缓存 → 在跨域受限时仍可快速查询热点数据

## 9. 典型落地建议

- 企业/区域部署：默认开启 `compliance.mode=enterprise|regional`，明确 `required_region`
- 移动端（L4）：默认启用离线队列与只读降级；蜂窝网络下扩大回放间隔
- 边缘（L3）：热点只读缓存 + 区域优先；禁用跨域写

---

更多上下文：
- 《ROADMAP.md》Phase 6.x 合规与抗干扰（并行专项）
- 《四层网络硬件部署与算力调度》：L4 本地缓存与区域协作
