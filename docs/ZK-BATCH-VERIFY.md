# ZK 批量验证与观测（Groth16）

最后更新：2025-11-10

本页介绍 SuperVM 的 ZK 批量验证缓冲与指标观测：如何启用、触发策略、Prometheus 指标以及 Grafana 面板。

---

## 1. 功能概览

- 批量缓冲：在隐私路径中收集 `(proof_bytes, public_inputs)`，按“批量大小”或“刷新间隔”触发一次性验证。

- 语义安全：若未到 flush 条件，为避免延迟积压，当前交易会立即单次验证，并从缓冲移除，保证请求不被重复验证。

- 指标观测：每次 flush 会记录批量指标（总耗时、均值、TPS、失败数等），便于性能画像与告警配置。

适用场景：有持续隐私交易流量、希望降低单笔均值开销、需要批量维度的吞吐/延迟观测。

---

## 2. 启用与配置

环境变量（进程启动时读取）：

- `ZK_BATCH_ENABLE`：`true|1` 开启批量缓冲（默认 `false`）

- `ZK_BATCH_SIZE`：批量大小（默认 `32`，建议 16~128 之间按延迟预算试验）

- `ZK_BATCH_FLUSH_INTERVAL_MS`：刷新时间窗口（默认 `50ms`，建议 20~100ms 的量级）

Windows PowerShell 示例：

```powershell
$env:ZK_BATCH_ENABLE = "1"
$env:ZK_BATCH_SIZE = "32"
$env:ZK_BATCH_FLUSH_INTERVAL_MS = "50"

```

> 提示：也可在仅观测而不改变行为的情况下，保持 `ZK_BATCH_ENABLE=0`，系统仍会导出单次验证的指标。

---

## 3. 触发策略（Flush Policy）

- 尺寸触发：当缓冲条目数 `>= ZK_BATCH_SIZE` 时立即 flush。

- 时间触发：若距离上次 flush 的间隔 `>= ZK_BATCH_FLUSH_INTERVAL_MS` 且缓冲非空，则 flush。

- 提前单次验证：若未满足 flush 条件，为避免用户请求被积压，当前交易会立即执行单次验证，并从缓冲移除，防止后续重复验证。

- 手动触发：`SuperVM::flush_zk_batch() -> (total, failed)`，可配合外部调度（cron/timer）在低谷或收尾主动清空。

> 现阶段批量验证仍为“逐项验证”的批处理（已做并行优化于验证器层），后续将考虑引入聚合证明或更高效的批量校验算法（如 SnarkPack）。

---

## 4. 指标（Prometheus）

验证侧批量指标（由 `vm-runtime/src/metrics.rs` 导出）：

- `vm_privacy_zk_batch_verify_total`：批量验证总 proof 数

- `vm_privacy_zk_batch_verify_failed_total`：批量验证失败的 proof 数

- `vm_privacy_zk_batch_verify_batches_total`：已处理的批次数

- `vm_privacy_zk_batch_verify_batch_latency_ms`：最近一批总耗时（ms）

- `vm_privacy_zk_batch_verify_avg_latency_ms`：最近一批每个 proof 的平均验证耗时（ms）

- `vm_privacy_zk_batch_verify_tps`：最近一批的验证吞吐（proofs/sec）

单次验证指标（供对比）：

- `vm_zk_verify_total`、`vm_zk_verify_failures_total`、`vm_zk_verify_failure_rate`

- `vm_zk_verify_latency_avg_ms`、`vm_zk_verify_latency_p50_ms/p90_ms/p99_ms`

- `vm_zk_backend_count{backend=...}`（后端分布）

---

## 5. Grafana 面板

已在根目录 `grafana-dashboard.json` 添加以下面板（分组在 ZK 区域之后）：

- "ZK Batch Verification Latency"：`batch_latency_ms` 与 `avg_latency_ms`

- "ZK Batch Verification Throughput"：`batch_verify_tps`

- "ZK Batch Verification Volume & Failures"：`total`/`failed`/`batches`

导入方式：
1) 打开 Grafana → Dashboards → Import → 上传 `grafana-dashboard.json`
2) 选择 Prometheus 数据源（可使用变量 `${DS_PROMETHEUS}`）

---

## 6. 后台定时器（推荐接入）

当前版本提供两种触发：尺寸/时间与手动 flush。为了让 flush 更规律、降低延迟抖动，SuperVM 提供了 `start_batch_flush_loop` 方法，可一键启动后台线程：

```rust
// 生命周期安全要求：SuperVM 实例必须在后台线程运行期间保持有效
// 方式1：Box::leak（进程级静态）
let vm = Box::leak(Box::new(
    SuperVM::new(&ownership)
        .with_scheduler(&scheduler)
        .with_verifier(&verifier)
        .from_env()
));
#[cfg(feature = "groth16-verifier")]
vm.start_batch_flush_loop(50); // 每 50ms 轮询一次

// 方式2：Arc 管理（推荐用于结构化应用）
let vm = Arc::new(SuperVM::new(&ownership)...);
let vm_clone = Arc::clone(&vm);
std::thread::spawn(move || {
    let mut interval = std::time::Duration::from_millis(50);
    loop {
        std::thread::sleep(interval);
        #[cfg(feature = "groth16-verifier")]
        { let _ = vm_clone.flush_zk_batch(); }
    }
});

```

若使用异步运行时（Tokio）：

```rust
// Tokio 定时器示例（需 vm 是 Arc<SuperVM> 或通过 Arc::from_raw 包装）
tokio::spawn(async move {
    let mut interval = tokio::time::interval(Duration::from_millis(50));
    loop {
        interval.tick().await;
        #[cfg(feature = "groth16-verifier")]
        { let _ = vm_clone.flush_zk_batch(); }
    }
});

```

> **注意**：`start_batch_flush_loop` 使用 unsafe 原始指针，要求调用方保证 SuperVM 实例生命周期覆盖后台线程（通常用 Box::leak 或进程级 static）；生产环境推荐在运行时（Tokio/Actix 等）内集中管理后台任务的启动与结束，或使用 Arc + 手动 spawn 方式以保持安全性。

---

## 7. 故障与调优建议

- 若批量间隔过大：可能导致尾延迟上升。可下调 `ZK_BATCH_FLUSH_INTERVAL_MS`。

- 若失败率提升：优先检查 proof/inputs 绑定、序列化格式与验证密钥一致性；必要时降低批量以便快速定位问题。

- TPS 波动大：结合单次与批量面板，观察“批次数量/批量大小/耗时”的耦合关系，调整 `ZK_BATCH_SIZE` 与间隔。

---

## 8. 参考实现

- `vm-runtime/src/supervm.rs`：批量缓冲、策略与 flush 实现

- `vm-runtime/src/metrics.rs`：Prometheus 指标导出

- `examples/zk_batch_vs_single_bench.rs`：单笔 vs 批量对比示例

如需进一步自动化：

- 在应用入口根据环境变量自动启动“后台 flush 定时器”（参见第 6 节），并与现有的 `/metrics` HTTP 端点整合。
