# RocksDB 持久化存储 - Linux 部署指南

> 适用于: Ubuntu 20.04/22.04, Debian 12, RockyLinux 9, AlmaLinux 9, CentOS Stream 9
> SuperVM 通过 `rust-rocksdb` 在构建时自动编译并链接 RocksDB, 无需系统级安装。只需准备编译环境与可选优化依赖即可。

## 1. 快速结论
- 不需要 `apt install rocksdb` (系统包往往版本落后)
- 只需安装: `build-essential cmake pkg-config clang` (或 `gcc`)
- 启用 feature: `--features rocksdb-storage` 自动完成编译
- 数据目录默认: `./data/rocksdb` (可通过 `RocksDBConfig::with_path()` 调整)

```bash
# 一行完成构建 (Release + RocksDB)
RUSTFLAGS="-C target-cpu=native" cargo build --release --features rocksdb-storage
```

## 2. 环境准备

### 2.1 基础依赖 (Ubuntu/Debian)
```bash
sudo apt update
sudo apt install -y build-essential cmake pkg-config clang git
```

### 2.2 可选性能依赖
| 组件 | 作用 | 是否必需 |
|------|------|----------|
| `zstd` | 压缩算法 (若启用 RocksDB 压缩) | 可选 |
| `libsnappy-dev` | Snappy 压缩 | 可选 |
| `liblz4-dev` | LZ4 压缩 | 可选 |

> 若未安装这些库且启用了相关压缩选项，RocksDB 会降级为无压缩或构建失败。SuperVM 默认 `enable_compression=false`。

```bash
sudo apt install -y libsnappy-dev zstd libzstd-dev liblz4-dev
```

### 2.3 Rust 工具链
```bash
curl https://sh.rustup.rs -sSf | sh
source "$HOME/.cargo/env"
rustup update stable
```

## 3. 一键引导（可选）

```bash
chmod +x scripts/bootstrap.sh
./scripts/bootstrap.sh
# 自定义: DB_PATH=/var/lib/supervm/rocksdb FEATURES=rocksdb-storage YES=1 ./scripts/bootstrap.sh
```

## 4. 构建与运行

### 3.1 构建
```bash
# Release 构建 + RocksDB 特性
cargo build --release --features rocksdb-storage

# 指定目标架构优化 (提升 RocksDB 内部 MemTable/压缩效率)
RUSTFLAGS="-C target-cpu=native" cargo build --release --features rocksdb-storage
```

### 3.2 运行示例
```bash
# 指标导出 HTTP 服务
cargo run --release --example storage_metrics_http --features rocksdb-storage
# 持久化一致性测试
cargo run --release --example persistence_consistency_test --features rocksdb-storage
# RocksDB 指标采集演示
cargo run --example rocksdb_metrics_demo --features rocksdb-storage
```

### 3.3 数据目录管理
```bash
# 默认目录 (相对路径)
mkdir -p ./data/rocksdb
# 使用绝对路径
export SUPERVM_DB_PATH="/var/lib/supervm/rocksdb"
```

在代码中：
```rust
let config = RocksDBConfig::default().with_path(
    std::env::var("SUPERVM_DB_PATH").unwrap_or_else(|_| "./data/rocksdb".into())
);
```

## 5. 生产优化建议

| 项目 | 建议值 | 说明 |
|------|--------|------|
| `max_open_files` | 20000 | 减少文件句柄回收压力 |
| `write_buffer_size` | 256MB | 增加 MemTable 合并批次 |
| `block_cache_size` | 1GB | 提升随机读取命中率 |
| `max_background_jobs` | 8 | 增强 Compaction 并行度 |
| 压缩 | 关闭或仅 LZ4 | 减少 CPU 占用 |

### 4.1 静态链接 (可选)
```bash
# 生成近似全静态二进制 (glibc 环境仍有动态部分)
RUSTFLAGS="-C target-feature=+crt-static" cargo build --release --features rocksdb-storage
```

### 4.2 NUMA/IO 调优
```bash
# 绑定进程到 NUMA 节点 0 (多 NUMA 环境可分离执行与存储线程)
numactl --cpunodebind=0 --membind=0 ./target/release/storage_metrics_http

# 使用 ionice 提升后台 Compaction 优先级
ionice -c2 -n4 ./target/release/storage_metrics_http
```

## 6. systemd 服务示例

`/etc/systemd/system/supervm-storage.service`:
```ini
[Unit]
Description=SuperVM Storage Metrics HTTP
After=network.target

[Service]
Type=simple
User=supervm
Group=supervm
WorkingDirectory=/opt/supervm
ExecStart=/opt/supervm/target/release/storage_metrics_http
Restart=on-failure
Environment=SUPERVM_DB_PATH=/var/lib/supervm/rocksdb

[Install]
WantedBy=multi-user.target
```

```bash
sudo systemctl daemon-reload
sudo systemctl enable supervm-storage --now
systemctl status supervm-storage
```

## 7. Prometheus + Grafana

### 6.1 Prometheus 抓取配置
```yaml
scrape_configs:
  - job_name: 'supervm_storage'
    static_configs:
      - targets: ['localhost:9091']
    metrics_path: '/metrics'
    scrape_interval: 5s
```

### 6.2 常用监控指标
| 指标 | 说明 |
|------|------|
| `rocksdb_estimate_num_keys` | 估计键数量 |
| `rocksdb_total_sst_size_bytes` | SST 总大小 |
| `rocksdb_cache_hit` / `rocksdb_cache_miss` | Block Cache 命中/未命中 |
| `rocksdb_compaction_cpu_micros` | 压缩 CPU 时间 |
| `rocksdb_write_stall_micros` | 写阻塞时间 |

### 6.3 告警示例
```yaml
- alert: RocksDBWriteStall
  expr: increase(rocksdb_write_stall_micros[5m]) > 5e6
  for: 10m
  labels:
    severity: warning
  annotations:
    summary: "RocksDB 写阻塞过高"
    description: "过去10分钟写阻塞累计 >5s"
```

## 8. 备份与恢复

### 7.1 快照 (Checkpoint) 方案
当前实现支持通过复制数据目录实现冷备份：
```bash
TIMESTAMP=$(date +%Y%m%d_%H%M%S)
cp -r /var/lib/supervm/rocksdb /backup/rocksdb_$TIMESTAMP
```

### 7.2 恢复
```bash
systemctl stop supervm-storage
rm -rf /var/lib/supervm/rocksdb/*
cp -r /backup/rocksdb_20251113_012300/* /var/lib/supervm/rocksdb/
systemctl start supervm-storage
```

## 9. 压力测试建议
| 场景 | 命令 | 目标 |
|------|------|------|
| 单线程写 | 自定义 bench | 验证基础延迟 |
| 多线程写 | 8/16 线程 | 观察写扩展性 |
| 随机读 | 50% get 50% set | 缓存命中率 |
| 大键空间 | 百万级 key | LSM level 行为 |

## 10. 常见问题 (FAQ)
### Q1: 编译缓慢 (>15min)
- 禁用压缩特性；确认未使用低速虚拟磁盘；加 `RUSTFLAGS="-C codegen-units=16"` 提升并行度。
### Q2: 运行提示 `Too many open files`
```bash
ulimit -n 1048576
# 永久配置 /etc/security/limits.conf
supervm soft nofile 1048576
supervm hard nofile 1048576
```
### Q3: Block Cache 命中率低 (<30%)
- 增大 `block_cache_size`; 使用热点键预热；检查是否随机扫描超出缓存容量。
### Q4: Compaction CPU 时间过高
- 降低写入速率；调大 MemTable；分批写入；观察 `num_files_level0` 是否积压。
### Q5: WAL 持久化需求
- 当前默认使用 WriteBatch 原子性；未来可拓展启用 WAL + fsync（将添加配置开关）。

## 11. 下一步路线
- 支持增量 Checkpoint 与在线备份
- WAL + 可配置落盘策略
- Column Family 分层热数据/冷数据隔离
- Bloom Filter 开关与多级过滤

## 12. 参考
- RocksDB Wiki: https://github.com/facebook/rocksdb/wiki
- rust-rocksdb: https://github.com/rust-rocksdb/rust-rocksdb
- SuperVM ROADMAP L0.2 持久化子系统
