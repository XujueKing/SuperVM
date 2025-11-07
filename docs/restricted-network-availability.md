# Availability Design under Restricted Networks (Compliance)

> This guide focuses on improving availability, metadata minimization, and observability of SuperVM under restricted or unstable networks in a lawful and compliant manner. It does not provide guidance on circumventing or bypassing regulation.

## 1. Objectives

- Availability: when networks are restricted or lossy, the system remains usable — reads continue, writes are queued and reconciled after recovery
- Compliance: regional policy, data residency, retention and audit; metadata minimization by default
- Observability: SLA and auditability without exposing sensitive data

## 2. Graceful Degradation

- ReadOnly mode: when upstream is unreachable, only reads are served; mutations are queued
- Store-and-Forward queue:
  - L4: queue transactions/ops into a persistent local queue (SQLite/embedded KV)
  - L3: regional read-only cache hits (no WAN), reducing cross-region dependency
  - After recovery, idempotent replay (based on idempotent keys; duplicate-safe)
- Friendly timeouts: exponential backoff, fast-fail, user-friendly hints

## 3. Cross-layer Cooperation (L4↔L3↔L2/L1)

- L4:
  - Local cache & signing, offline queue, LAN cooperation (L4↔L4 sync in LAN)
  - Switch to ReadOnly + Queue when L3 is unreachable
- L3:
  - Regional hot read-only cache (80–95% hit), low-latency queries
  - Resync from L2 after connectivity recovers
- L2/L1:
  - Configurable data residency & pruning; cross-region access requires authorization and audit

## 4. Policy & Configuration (Example)

```toml
[compliance]
mode = "regional"                 # enterprise|regional|global
geo_fencing = ["CN", "!EU"]
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

## 5. Minimal API Skeleton (Doc Sample)

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

## 6. Idempotency & Reconciliation

- Idempotent key: `sha256(tx)` or business-defined composite keys, ensuring single-effect semantics
- Replay flow:
  1. Local de-dup check before dequeuing
  2. Retry with exponential backoff on upstream ACK failure (max N times)
  3. On success, write reconciliation record and update local state
- Reconciliation:
  - Periodic reconciliation between L4 and L3/L2 (Bloom filter for quick diff)
  - Repair abnormal entries manually or in batch

## 7. Observability & Audit

- Metrics (Prometheus):
  - `availability_percent`, `degrade_count`, `offline_queue_depth`, `replay_lag_seconds`
  - `regional_hit_ratio`, `cross_region_denied_total`
- Logs:
  - Audit logs redact PII/keys by default
  - Policy-denied reason codes and context are auditable

## 8. E2E Test Scenarios

- A: Upstream blocked → L4 switches to ReadOnly+Queue; recovery replay succeeds without side effects
- B: Cross-region write is denied by policy → locally rejected with an auditable reason
- C: Allowed transport interrupted → fallback to another transport in whitelist
- D: Rate shaping → peak traffic is smoothed without triggering upstream drops
- E: Regional read-only cache → fast queries under cross-region restrictions

## 9. Deployment Tips

- Enterprise/Regional: enable `compliance.mode=enterprise|regional`, set `required_region`
- Mobile (L4): enable offline queue & ReadOnly by default; increase replay intervals on cellular
- Edge (L3): hot read-only cache + regional priority; disable cross-region writes

---

See also:
- `ROADMAP.md` Phase 6.x Compliance & Anti-Interference (parallel workstream)
- Four-layer Hardware Deployment & Compute Scheduling: L4 local cache and regional cooperation
