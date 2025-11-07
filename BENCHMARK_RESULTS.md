# 基准测试结果（CI 对比版）

> 本页记录 CI 自动跑的 Criterion 基准结果，并与 main 分支基线对比；作为 L0 变更合入的性能证据链。当前仓库基准文件为 `src/vm-runtime/benches/parallel_benchmark.rs`。

- 报告日期：2025-11-07
- CI 工作流：`.github/workflows/kernel-purity-check.yml`
- 运行环境：GitHub Actions ubuntu-latest（Runner 2 vCPU / 7GB 内存，虚拟化环境）
- 基线定义：origin/main 同步跑的 `--save-baseline main` 对比 `--save-baseline current`
- 判定门槛：任一关键分组回归 < 5% 视为可接受

---

## 1. 结果总览（本次 CI）

> 等待 CI 完成后，从 artifact “performance-report” 下载并打开 `target/criterion/report/index.html`，将关键分组的相对变化粘贴到下表。

| 基准分组 | 指标 | current | main(baseline) | 变化 | 判定 |
|---|---|---:|---:|---:|---|
| conflict_detection | time/iter | 待填 | 待填 | 待填 | < 5% ✅ / ≥ 5% ⚠️ |
| snapshot_operations (create_snapshot) | time/iter | 待填 | 待填 | 待填 |  |
| snapshot_operations (rollback) | time/iter | 待填 | 待填 | 待填 |  |
| dependency_graph (build_and_query) | time/iter | 待填 | 待填 | 待填 |  |
| parallel_scheduling (get_parallel_batch) | time/iter | 待填 | 待填 | 待填 |  |
| mvcc_operations (read_only_transaction) | time/iter | 待填 | 待填 | 待填 |  |
| mvcc_operations (read_write_transaction) | time/iter | 待填 | 待填 | 待填 |  |
| mvcc_operations (mvcc_non_conflicting_writes) | time/iter | 待填 | 待填 | 待填 |  |
| mvcc_operations (mvcc_conflicting_writes) | time/iter | 待填 | 待填 | 待填 |  |
| mvcc_scheduler (snapshot_backend_read) | time/iter | 待填 | 待填 | 待填 |  |
| mvcc_scheduler (mvcc_backend_read_only) | time/iter | 待填 | 待填 | 待填 |  |
| mvcc_scheduler (snapshot_backend_write) | time/iter | 待填 | 待填 | 待填 |  |
| mvcc_scheduler (mvcc_backend_write) | time/iter | 待填 | 待填 | 待填 |  |
| mvcc_gc (gc_throughput, N=5/10/20/50) | time/iter | 待填 | 待填 | 待填 |  |
| auto_gc_impact (read_with/without_auto_gc) | time/iter | 待填 | 待填 | 待填 |  |
| auto_gc_impact (write_with/without_auto_gc) | time/iter | 待填 | 待填 | 待填 |  |

结论：
- 总体回归：待填（预计 < 5%）
- 风险评估：低（新增字段默认关闭，不影响关键路径；所有测试通过）

---

## 2. 方法与可复现性

- 运行命令（CI 自动执行）：
	- 当前分支：
		- `cargo bench -p vm-runtime --bench parallel_benchmark -- --save-baseline current`
	- 基线（main）：
		- `cargo bench -p vm-runtime --bench parallel_benchmark -- --save-baseline main`
	- 对比报告：
		- `cargo criterion --baseline main`
- 报告路径：`target/criterion/report/index.html`
- 注意：Criterion 输出的是每次迭代耗时（time/iter），作对比用相对百分比（regression/improvement）。

---

## 3. 与历史版本对照（参考）

> 以下为历史一次（v0.7.0）局部结果，仅作对照参考，环境与场景不可比，不纳入本次回归判定。

- 自动 GC 性能影响（v0.7.0 本地）：
	- 写入开销 ~ +1.8%
	- 读取开销 ~ +3.7%

---

## 4. 结论与后续

- 结论：当关键分组回归 < 5% 时，满足 L0 合入性能门槛。
- 后续：
	1) CI 完成后更新上表为最终值；
	2) 在 PR 中附上本页链接；
	3) 合并到 main 后，作为新的基线继续跟踪。
