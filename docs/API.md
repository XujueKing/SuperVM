# vm-runtime API Documentation

版本: v0.9.0  
最后更新: 2025-11-05

开发者/作者：King Xujue

## 概述

`vm-runtime` 是一个基于 wasmtime 的 WebAssembly 运行时库，提供存储抽象、链上下文访问和事件系统。

## 核心类型

### Runtime&lt;S: Storage&gt;

WASM 虚拟机运行时，支持自定义存储后端。

#### 泛型参数

- `S`: 实现 `Storage` trait 的存储后端类型

#### 构造函数

**new**

```rust
pub fn new(storage: S) -> Self

```

创建新的运行时实例。

**参数**: `storage` - 存储后端实现

**示例**:

```rust
use vm_runtime::{Runtime, MemoryStorage};
let runtime = Runtime::new(MemoryStorage::new());

```

**new_with_routing (v2.0+)**

```rust
pub fn new_with_routing(storage: S) -> Self

```

创建带路由能力的运行时（支持对象所有权和MVCC调度）。

---

## Storage Trait

存储抽象接口。

```rust
pub trait Storage {
    fn get(&amp;self, key: &amp;[u8]) -> Result&lt;Option&lt;Vec&lt;u8&gt;&gt;&gt;;
    fn set(&amp;mut self, key: &amp;[u8], value: &amp;[u8]) -> Result&lt;()&gt;;
    fn delete(&amp;mut self, key: &amp;[u8]) -> Result&lt;()&gt;;
}

```

### 方法

**get** - 读取键值  
**set** - 写入键值对  
**delete** - 删除键值对  

---

## MVCC Store (v0.7.0+)

多版本并发控制存储。

### MvccStore

提供快照隔离的事务支持。

**方法**:

- `new()` - 创建新的 MVCC 存储实例

- `begin()` - 开始新事务，返回事务句柄

- `enable_auto_gc(config)` - 启用自动垃圾回收

- `gc_now()` - 立即执行垃圾回收

**示例**:

```rust
use vm_runtime::MvccStore;

let store = MvccStore::new();
let mut txn = store.begin();
txn.write(b"key", b"value")?;
txn.commit()?;

```

### Txn

事务句柄。

**方法**:

- `read(&amp;mut self, key: &amp;[u8])` - 读取键值（v0.9.0+ 需要 &amp;mut self）

- `write(&amp;mut self, key, value)` - 写入键值

- `commit(self)` - 提交事务

- `abort(self)` - 放弃事务

---

## 并行调度器 (v0.9.0+)

### MvccScheduler

基于 MVCC 的并行事务调度器。

**方法**:

- `new()` - 创建默认配置的调度器

- `execute_batch(store, transactions)` - 批量并行执行事务

- `stats()` - 获取调度器统计信息

**示例**:

```rust
let scheduler = MvccScheduler::new();
let store = MvccStore::new();

let txns = vec![
    (1, |txn: &amp;mut Txn| {
        txn.write(b"key1", b"value1")?;
        Ok(0)
    }),
];

let result = scheduler.execute_batch(&amp;store, txns);

```

---

## 对象所有权模型 (v2.0+)

### OwnershipManager

Sui 风格的对象所有权管理。

**方法**:

- `create_object(id, owner, ownership_type)` - 创建新对象

- `transfer_object(object_id, from, to)` - 转移对象所有权

- `access_object(object_id, accessor, access_type)` - 检查对象访问权限

- `freeze_object(object_id, owner)` - 冻结对象为不可变

- `share_object(object_id, owner)` - 将对象转为共享

---

## SuperVM 统一接口 (v2.0+)

### SuperVM

统一的虚拟机入口，支持公开/私有模式路由。

**方法**:

```rust
pub fn execute_transaction(
    &amp;self,
    tx: VmTransaction,
    privacy: Privacy,
) -> Result&lt;ExecutionReceipt&gt;

```

执行交易（根据隐私模式路由）。

**示例**:

```rust
use vm_runtime::{SuperVM, Privacy, VmTransaction};

let vm = SuperVM::new(MemoryStorage::new());
let receipt = vm.execute_transaction(tx, Privacy::Public)?;

```

---

## Host Functions

WASM 模块可导入的 host 函数。

### storage_api

- `storage_get(key_ptr, key_len)` - 读取存储值

- `storage_set(key_ptr, key_len, value_ptr, value_len)` - 写入存储值

- `storage_delete(key_ptr, key_len)` - 删除存储值

### chain_api

- `block_number()` - 获取当前区块号

- `timestamp()` - 获取当前时间戳

- `emit_event(data_ptr, data_len)` - 发出事件

### crypto_api

- `sha256(input_ptr, input_len, output_ptr)` - 计算 SHA256 哈希

- `verify_ed25519(msg_ptr, msg_len, sig_ptr, pubkey_ptr)` - 验证 Ed25519 签名

---

## Phase 4.3 新增 API (2025-11)

### MvccStore::prune_old_versions

状态裁剪功能，批量清理历史版本。

```rust
pub fn prune_old_versions(
    &self,
    rocksdb: &mut RocksDBStorage,
    keep_versions: usize,
) -> Result<(u64, u64), String>

```

**参数**:

- `rocksdb` - RocksDB 存储实例

- `keep_versions` - 保留的最近版本数量

**返回值**: `(清理的版本数, 涉及的键数)`

**示例**:

```rust
use vm_runtime::{MvccStore, RocksDBStorage};
use std::sync::Arc;

let mut rocksdb = RocksDBStorage::new("data/db", false)?;
let mvcc = Arc::new(MvccStore::new());

// 刷新到 RocksDB
mvcc.flush_to_storage(&mut rocksdb)?;

// 裁剪,保留最近 10 个版本
let (pruned_versions, pruned_keys) = mvcc.prune_old_versions(&mut rocksdb, 10)?;
println!("清理 {} 版本, {} 键", pruned_versions, pruned_keys);

```

---

### HTTP /metrics Endpoint

Prometheus 格式指标导出端点。

**启动方式 (MVCC + 路由合并输出)**:

```powershell
cargo run -p vm-runtime --example metrics_http_demo --features rocksdb-storage --release

```

**访问地址**: `http://127.0.0.1:8080/metrics`  (MVCC + Routing 指标合并)

**导出指标 (MVCC)**:

- `mvcc_tps` - 当前每秒事务处理量

- `mvcc_success_rate` - 事务成功率 (%)

- `mvcc_txn_started_total` - 启动的事务总数

- `mvcc_txn_committed_total` - 提交的事务总数

- `mvcc_txn_aborted_total` - 中止的事务总数

- `mvcc_txn_latency_ms{quantile="0.5|0.9|0.99"}` - 事务延迟百分位

- `mvcc_gc_runs_total` - GC 运行次数

- `mvcc_gc_versions_cleaned_total` - GC 清理的版本数

- `mvcc_flush_count_total` - 刷新次数

- `mvcc_flush_keys_total` - 刷新的键数

- `mvcc_flush_bytes_total` - 刷新的字节数

**导出指标 (Routing + Adaptive + ZK)**:

- `vm_routing_fast_total`        - 路由到快速通道的事务数

- `vm_routing_consensus_total`   - 路由到共识通道的事务数

- `vm_routing_privacy_total`     - 路由到隐私通道的事务数

- `vm_routing_total`             - 路由总事务数

- `vm_routing_fast_ratio`        - 快速通道路由比例 (0-1)

- `vm_routing_consensus_ratio`   - 共识通道路由比例 (0-1)

- `vm_routing_privacy_ratio`     - 隐私通道路由比例 (0-1)
    
自适应路由新增指标 (AdaptiveRouter):

- `vm_routing_target_fast_ratio`             - 自适应算法当前目标 Fast 占比

- `vm_routing_adaptive_adjustments_total`    - 自适应比例累计调整次数

ZK 验证延迟指标（启用 `groth16-verifier` feature 且发生过验证后导出）:

- `vm_privacy_zk_verify_count_total`         - 已执行的真实 ZK 验证次数

- `vm_privacy_zk_verify_avg_latency_ms`      - 平均验证延迟（毫秒）

- `vm_privacy_zk_verify_last_latency_ms`     - 最近一次验证延迟（毫秒）

- `vm_privacy_zk_verify_p50_latency_ms`      - 最近滑动窗口验证延迟 P50（毫秒）

- `vm_privacy_zk_verify_p95_latency_ms`      - 最近滑动窗口验证延迟 P95（毫秒）

- `vm_privacy_zk_verify_window_size`         - 滑动窗口中样本数量 (默认 64)

此外，示例 `metrics_http_demo` 现已合并导出 SuperVM 路由指标（无需单独进程），同一端点会包含以下 Routing 指标：

- `vm_routing_fast_total`、`vm_routing_consensus_total`、`vm_routing_privacy_total`、`vm_routing_total`

- `vm_routing_fast_ratio`、`vm_routing_consensus_ratio`、`vm_routing_privacy_ratio`

也可使用 `routing_metrics_http_demo`（端口 8081）单独查看路由指标。

#### 路由指标（Phase 5 新增）

若使用示例 `routing_metrics_http_demo`（端口 8081），将额外暴露 SuperVM 路由统计：

启动示例:

```powershell
cargo run -p vm-runtime --example routing_metrics_http_demo --release

```

访问地址: `http://127.0.0.1:8081/metrics`

导出指标（Routing + Adaptive + ZK）:

- `vm_routing_fast_total`        - 路由到快速通道的事务数

- `vm_routing_consensus_total`   - 路由到共识通道的事务数

- `vm_routing_privacy_total`     - 路由到隐私通道的事务数

- `vm_routing_total`             - 路由总事务数

- `vm_routing_fast_ratio`        - 快速通道路由比例 (0-1)

- `vm_routing_consensus_ratio`   - 共识通道路由比例 (0-1)

- `vm_routing_privacy_ratio`     - 隐私通道路由比例 (0-1)

- `vm_routing_target_fast_ratio` - 自适应当前目标 Fast 占比

- `vm_routing_adaptive_adjustments_total` - 自适应调整次数

- （若启用并发生验证）`vm_privacy_zk_verify_*` 一组延迟与次数指标

自适应路由环境变量覆盖（可选）：

在 `routing_metrics_http_demo` 与集成环境中，AdaptiveRouter 支持通过环境变量覆盖默认参数（未设置的项使用默认值）。

- `SUPERVM_ADAPTIVE_INIT`          初始 Fast 比例 (默认 0.70)

- `SUPERVM_ADAPTIVE_MIN`           最小 Fast 比例 (默认 0.10)

- `SUPERVM_ADAPTIVE_MAX`           最大 Fast 比例 (默认 0.90)

- `SUPERVM_ADAPTIVE_STEP_UP`       上调步长 (默认 0.05)

- `SUPERVM_ADAPTIVE_STEP_DOWN`     下调步长 (默认 0.05)

- `SUPERVM_ADAPTIVE_CONFLICT_LOW`  冲突低阈值 (默认 0.05)

- `SUPERVM_ADAPTIVE_CONFLICT_HIGH` 冲突高阈值 (默认 0.25)

- `SUPERVM_ADAPTIVE_SUCCESS_LOW`   成功率低阈值 (默认 0.80)

- `SUPERVM_ADAPTIVE_UPDATE_EVERY`  更新周期（次调用）(默认 100)

Windows PowerShell 示例（调整初始 Fast 比例与步长）:

```powershell
$env:SUPERVM_ADAPTIVE_INIT = "0.55"
$env:SUPERVM_ADAPTIVE_STEP_UP = "0.02"; $env:SUPERVM_ADAPTIVE_STEP_DOWN = "0.04"
cargo run -p vm-runtime --example routing_metrics_http_demo --release

```

Prometheus 抓取配置示例（增加一个 job）:

```yaml
scrape_configs:
    - job_name: 'supervm-mvcc'
        static_configs:
            - targets: ['localhost:8080']
        metrics_path: '/metrics'
    - job_name: 'supervm-routing'
        static_configs:
            - targets: ['localhost:8081']
        metrics_path: '/metrics'

```

Grafana 面板建议:

- Fast/Consensus/Privacy 路由比例三线趋势图: `vm_routing_fast_ratio`, `vm_routing_consensus_ratio`, `vm_routing_privacy_ratio`

- 路由总数累积: `vm_routing_total`

- 通道占比饼图 (Transform)

- 目标 Fast 比例 vs 实际 Fast 比例对比折线: `vm_routing_target_fast_ratio` vs `vm_routing_fast_ratio`

- 自适应调整次数累积柱状: `vm_routing_adaptive_adjustments_total`

- ZK 验证延迟趋势 (last/avg/p95): `vm_privacy_zk_verify_last_latency_ms`, `vm_privacy_zk_verify_avg_latency_ms`, `vm_privacy_zk_verify_p95_latency_ms`

- ZK 验证次数累计: `vm_privacy_zk_verify_count_total`

指标来源: `SuperVM::export_routing_prometheus()`。

**Prometheus 配置 (单端点抓取)**:

```yaml
scrape_configs:
  - job_name: 'supervm'
    static_configs:
      - targets: ['localhost:8080']
    metrics_path: '/metrics'

```

#### 混合负载基准指标服务（Phase 5 新增）

边跑基准边暴露指标：`mixed_path_bench` 支持 `--serve-metrics[:PORT]` 参数，默认端口 8082。

启动示例:

```powershell
cargo run -p vm-runtime --example mixed_path_bench --release -- --serve-metrics:8082

```

访问地址: `http://127.0.0.1:8082/metrics`、`http://127.0.0.1:8082/summary`

新增 ZK 延迟基准 (Groth16)：

```powershell
$env:SUPERVM_ZK_BENCH_QPS = "50"
$env:SUPERVM_ZK_LAT_WIN = "64"           # 可选，滑动窗口样本数（默认64）
$env:SUPERVM_ZK_BENCH_PORT = "8083"      # 可选，HTTP 端口（默认8083）
cargo run -p vm-runtime --example zk_latency_bench --features groth16-verifier --release

```

访问地址: `http://127.0.0.1:<PORT>/metrics`（默认 `<PORT>=8083`）

导出指标（除 Routing + Adaptive + ZK 全套外，增加 Fast / Consensus / Privacy 基准指标）:

- FastPath:
    - `bench_fastpath_executed_total`    快速通道执行成功数
    - `bench_fastpath_failed_total`      快速通道失败数
    - `bench_fastpath_avg_latency_ns`    快速通道平均延迟（纳秒）
    - `bench_fastpath_estimated_tps`     快速通道估算 TPS（1e9 / avg_latency_ns）

- PrivacyPath:
    - `bench_privacy_executed_total`     隐私路径执行成功数
    - `bench_privacy_failed_total`       隐私路径失败数
    - `bench_privacy_avg_latency_ns`     隐私路径平均延迟（纳秒）
    - `bench_privacy_estimated_tps`      隐私路径估算 TPS（1e9 / avg_latency_ns）

- ConsensusPath:
    - `bench_consensus_success_rate`     共识成功率（0~1）
    - `bench_consensus_conflict_rate`    共识冲突率（0~1）

路由指标（已合并端点统一导出）:

- `vm_routing_fast_total` / `_ratio`

- `vm_routing_consensus_total` / `_ratio`

- `vm_routing_privacy_total` / `_ratio`

使用隐私事务：添加参数 `--privacy-ratio:FLOAT` 或环境变量 `PRIVACY_RATIO` 例如：

```powershell
cargo run -p vm-runtime --example mixed_path_bench --release -- --txs 80000 --owned-ratio 0.7 --privacy-ratio:0.15 --serve-metrics:8082

```

示例输出关键字段：

```

bench_privacy_executed_total 29941
bench_privacy_avg_latency_ns 2659
bench_privacy_estimated_tps 376081.23
vm_routing_privacy_ratio 0.1497

```

#### 隐私延迟模拟与迭代控制（Phase 5 增强）

为更贴近真实 ZK / 隐私证明验证开销，可注入固定延迟，并显式控制总迭代数：

参数/环境变量:

- `--privacy-latency-ms:NUM` 或 `PRIVACY_LATENCY_MS=NUM`
    - 对每一笔走隐私路径的事务人工注入 `NUM` 毫秒延迟（阻塞 sleep），用于模拟证明验证或解密成本。
    - 会直接抬升 `bench_privacy_avg_latency_ns`（理论上在原始纳秒平均值基础上 + `NUM * 1_000_000`）。
    - 仅影响隐私路径，不影响 fast / consensus 事务。

- `--txs NUM` 或 `--txs:NUM`
    - 覆盖默认迭代次数（代码支持空格或冒号两种形式）。
    - 便于进行小样本快速验证或固定规模基准。

示例：2000 笔事务，20% 隐私，单笔隐私模拟 5ms 延迟并暴露指标：

```powershell
cargo run -p vm-runtime --example mixed_path_bench --release -- --txs 2000 --owned-ratio 0.6 --privacy-ratio:0.2 --privacy-latency-ms:5 --serve-metrics:8082

```

（截取）示例指标变化：

```

bench_privacy_avg_latency_ns 5002341   # ≈ 5ms 注入 + 原始执行耗时
bench_privacy_estimated_tps 199123.4   # 因平均延迟上升而下降

```

调优指引：

- 通过多组 `--privacy-latency-ms`（如 0 / 2 / 5 / 10 / 20）测量曲线，评估系统吞吐对隐私验证成本的敏感度。

- 将结果追加到性能报告（如 `PHASE5-METRICS.md`）以便可视化趋势。

- 若未来接入真实 ZK 验证器，可去掉该模拟参数或作为 fallback。

说明: 启用后每 1000 事务刷新快照；隐私路径当前复用共识执行器（后续可替换为真实隐私执行器和 ZK 验证延迟）。Privacy 平均延迟与估算 TPS 可用于评估 ZK 成本。

> 如果仍需分离路由指标，可运行 `routing_metrics_http_demo`（端口 8081）并增加第二个 job。

---

### Grafana Dashboard

预配置的 Grafana Dashboard 用于可视化监控。

**导入方式**:
1. 在 Grafana 中添加 Prometheus 数据源 (`http://localhost:9090`)
2. Import → Upload JSON file
3. 选择 `grafana-dashboard.json`

**Dashboard 面板**:

- MVCC Transactions Per Second (TPS)

- Transaction Success Rate (%)

- Transaction Latency Percentiles (P50/P90/P99)

- Transaction Rates (1m avg)

- MVCC Garbage Collection

- MVCC Flush Statistics

- MVCC Flush Bytes
 - Routing Path Ratios (fast/consensus/privacy)
 - Routing Path Counts (counters)
 - Routing Path Pie (Transform)

**详细文档**: 参见 [docs/GRAFANA-DASHBOARD.md](./GRAFANA-DASHBOARD.md)

---

### AutoFlushConfig

自动刷新配置结构。

```rust
pub struct AutoFlushConfig {
    pub enable: bool,
    pub interval_seconds: u64,
    pub block_trigger_threshold: u64,
}

```

**字段**:

- `enable` - 是否启用自动刷新

- `interval_seconds` - 时间触发器间隔 (秒)

- `block_trigger_threshold` - 区块触发器阈值

**示例**:

```rust
use vm_runtime::{MvccStore, RocksDBStorage, AutoFlushConfig};
use std::sync::Arc;

let rocksdb = RocksDBStorage::new("data/db", false)?;
let mvcc = Arc::new(MvccStore::new());

let config = AutoFlushConfig {
    enable: true,
    interval_seconds: 300,  // 5分钟
    block_trigger_threshold: 10000,
};

mvcc.start_auto_flush(rocksdb.clone(), config);

// ... 运行事务 ...

mvcc.stop_auto_flush();

```

---

### FlushStats

刷新统计数据结构。

```rust
pub struct FlushStats {
    pub flush_count: u64,
    pub keys_flushed: u64,
    pub bytes_flushed: u64,
}

```

**获取方式**:

```rust
let stats = mvcc.flush_stats();
println!("刷新 {} 次, {} 键, {} KB",
    stats.flush_count,
    stats.keys_flushed,
    stats.bytes_flushed / 1024);

```

---

### MetricsCollector

全局指标收集器 (线程安全单例)。

```rust
pub struct MetricsCollector { /* ... */ }

```

**方法**:

```rust
// 获取全局实例
pub fn global() -> &'static MetricsCollector

// TPS 和成功率
pub fn tps(&self) -> f64
pub fn success_rate(&self) -> f64

// 延迟百分位
pub fn latency_p50(&self) -> f64
pub fn latency_p90(&self) -> f64
pub fn latency_p99(&self) -> f64

// 事务计数
pub fn txn_started(&self) -> u64
pub fn txn_committed(&self) -> u64
pub fn txn_aborted(&self) -> u64

// 记录事件
pub fn record_txn_start(&self)
pub fn record_txn_commit(&self)
pub fn record_txn_abort(&self)
pub fn record_txn_latency(&self, latency_ms: f64)

```

**示例**:

```rust
use vm_runtime::MetricsCollector;

let metrics = MetricsCollector::global();

// 记录事务
metrics.record_txn_start();
let start = std::time::Instant::now();
// ... 执行事务 ...
metrics.record_txn_commit();
metrics.record_txn_latency(start.elapsed().as_secs_f64() * 1000.0);

// 查询统计
println!("TPS: {:.0}", metrics.tps());
println!("成功率: {:.2}%", metrics.success_rate());
println!("P99 延迟: {:.2}ms", metrics.latency_p99());

```

---

### export_prometheus

导出 Prometheus 格式指标。

```rust
pub fn export_prometheus() -> String

```

**返回值**: Prometheus 文本格式的指标数据

**示例**:

```rust
use vm_runtime::export_prometheus;

let metrics_text = export_prometheus();
println!("{}", metrics_text);

```

**输出示例**:

```

# HELP mvcc_tps Transactions per second

# TYPE mvcc_tps gauge

mvcc_tps 85432.5

# HELP mvcc_success_rate Transaction success rate (%)

# TYPE mvcc_success_rate gauge

mvcc_success_rate 98.75

# HELP mvcc_txn_latency_ms Transaction latency (milliseconds)

# TYPE mvcc_txn_latency_ms gauge

mvcc_txn_latency_ms{quantile="0.5"} 0.85
mvcc_txn_latency_ms{quantile="0.9"} 2.34
mvcc_txn_latency_ms{quantile="0.99"} 5.67

```

---

## 版本历史

### v0.10.0 (Phase 4.3, 2025-11-08)

- ✅ RocksDB 持久化存储集成

- ✅ Checkpoint 快照管理

- ✅ MVCC Auto-Flush 机制

- ✅ Prometheus Metrics 集成

- ✅ HTTP /metrics 端点

- ✅ 状态裁剪功能 (prune_old_versions)

- ✅ Grafana Dashboard 配置

### v0.9.0 (2025-06-03)

-  Write Skew 修复（读集合跟踪 + 三阶段提交）

-  金额守恒验证通过

-  性能优化：187K TPS（低竞争），85K TPS（高竞争）

### v0.7.0

-  自适应 GC（自动垃圾回收）

-  MVCC 存储实现

### v0.6.0

-  并行执行引擎

-  冲突检测与依赖分析

---

## 许可证

本项目采用 GPL-3.0-or-later 许可证。详见根目录 LICENSE 文件。

版权所有  2025 XujueKing <leadbrand@me.com>
