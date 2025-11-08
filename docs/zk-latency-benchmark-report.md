# SuperVM ZK 验证延迟基准报告 (初始草稿)

日期: 2025-11-08  
分支: king/l0-mvcc-privacy-verification  
示例: `zk_latency_bench` (Groth16)

## 1. 目标
评估真实 Groth16 验证在 SuperVM 集成下的平均延迟、滑动窗口百分位 (p50/p95) 与可调 QPS 负载行为，为后续隐私路径调度策略 (Phase B) 提供参考。

## 2. 方法
使用新示例（直接调用 Groth16 验证器以降低样例复杂度）：
```powershell
$env:SUPERVM_ZK_BENCH_QPS = "50"
$env:SUPERVM_ZK_LAT_WIN = "64"
$env:SUPERVM_ZK_BENCH_PORT = "8084"
cargo run -p vm-runtime --example zk_latency_bench --features groth16-verifier --release
```
示例行为：
- 按设定 QPS 周期调用真实 Groth16 验证器（示例中未嵌入 SuperVM，避免生命周期复杂度）
- 滑动窗口容量: 64 (环形缓冲)
- 通过 `/metrics` 暴露以下指标：
  - `vm_privacy_zk_verify_count_total`
  - `vm_privacy_zk_verify_avg_latency_ms`
  - `vm_privacy_zk_verify_last_latency_ms`
  - `vm_privacy_zk_verify_p50_latency_ms`
  - `vm_privacy_zk_verify_p95_latency_ms`
  - `vm_privacy_zk_verify_window_size`

## 3. 指标抓取建议 (Prometheus)
```yaml
scrape_configs:
  - job_name: 'supervm-zk-latency'
    static_configs:
      - targets: ['localhost:8084']
    metrics_path: '/metrics'
    scrape_interval: 2s
```

## 4. 初步结果
> 机器: Windows; 构建: release; 曲线基于窗口 64 样本
| QPS | avg_latency_ms | p50_ms | p95_ms | last_ms | samples | notes |
|-----|----------------|-------|-------|---------|---------|-------|
| 50  | 3.746          | 3.353 | 9.772 | 2.496   | 64      | 端口=8084 |
| 100 | 2.754          | 2.475 | 6.592 | 3.406   | 64      | 端口=8085 |
| 200 | 2.464          | 2.239 | 4.036 | 4.392   | 64      | 端口=8086 |

## 5. 观察点 & 后续分析
1. p95 是否显著高于平均值 (长尾验证).  
2. 窗口大小是否足够平滑 (考虑参数化 SUPERVM_ZK_LAT_WIN).  
3. 验证吞吐与延迟关系 (可估 TPS = QPS * 成功率).  
4. 与交易执行路径整合：后续可在 PrivacyPath 执行前动态评估验证队列长度。  

## 6. 下一步
- [x] 采集真实数据填表（QPS=50）
- [x] 增加窗口容量环境变量支持 (SUPERVM_ZK_LAT_WIN)
- [x] 采集 QPS=100 / 200 数据
- [ ] 将 ZK 延迟曲线加入 Grafana Dashboard
- [ ] 设计 PrivacyPath 降级策略：高延迟时转入延迟隐藏批处理

## 7. 结论 (待数据)
真实验证延迟分布与比例指标将用于驱动 Phase B 的“延迟感知隐私路由”策略。最终目标：在高验证负载时保持公共路径吞吐，同时保障隐私交易体验。

---
GPL-3.0-or-later | SuperVM Project
