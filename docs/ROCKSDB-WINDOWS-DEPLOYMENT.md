# RocksDB 持久化存储 - Windows 部署指南

> 说明：SuperVM 使用 `rust-rocksdb` 绑定在构建时自动编译并静态/半静态链接 RocksDB（默认静态链接其核心 C++ 库）。因此**不需要单独“安装” RocksDB**：只要启用 `rocksdb-storage` feature，`cargo build` 会自动完成依赖获取与编译。打包发行版本时：
> - 若使用 `cargo build --release` 生成二进制，RocksDB 代码已被编译进最终可执行文件或伴随少量动态库（根据平台）。
> - 部署时只需携带：SuperVM 可执行文件 + 数据目录（首次启动自动创建）。无需在目标机器额外安装 RocksDB。
> - 若希望完全静态（Linux 可选），可在 CI 中开启 `RUSTFLAGS="-C target-feature=+crt-static"`。
> - Windows 下通常采用 MSVC 链接，默认即可；不建议自行手动编译外部 RocksDB，除非要启用高级特性（ZSTD 压缩、定制 LRU 缓存策略等）。


## 概述

本文档说明如何在 Windows 环境下配置和使用 SuperVM 的 RocksDB 持久化存储功能。

## 前置条件

### 系统要求

- **操作系统**: Windows 10/11 或 Windows Server 2019/2022
- **Rust 工具链**: 1.70.0 或更高版本
- **Visual Studio**: 需要 MSVC 编译器 (推荐 VS 2019/2022)
  - 安装 "C++ 生成工具" 或完整的 Visual Studio
  - 包含 "使用 C++ 的桌面开发" 工作负载

### 依赖项

RocksDB 在 Windows 上的编译依赖于 CMake 和 MSVC。确保已安装：

```powershell
# 检查 CMake (RocksDB Rust binding 会自动调用)
cmake --version

# 检查 MSVC 编译器
cl.exe
```

## 一键引导（推荐）

在仓库根目录执行：

```powershell
powershell -ExecutionPolicy Bypass -File .\scripts\bootstrap.ps1
```

可选参数：`-DbPath` 指定数据目录，`-Features` 指定 Cargo 特性（默认 `rocksdb-storage`）。

## 安装 / 构建步骤（无需单独安装 RocksDB）

### 1. 启用 RocksDB 特性

在 `Cargo.toml` 中启用 `rocksdb-storage` feature:

```toml
[features]
default = []
rocksdb-storage = ["rocksdb"]

[dependencies]
rocksdb = { version = "0.21", optional = true }
```

### 2. 编译项目

```powershell
# 编译库 + RocksDB 支持
cargo build --release --features rocksdb-storage

# 编译特定示例
cargo build --example storage_metrics_http --features rocksdb-storage --release
cargo build --example persistence_consistency_test --features rocksdb-storage --release
```

**首次编译注意事项**:
- 首次编译会下载并编译 RocksDB C++ 库，耗时较长（5-10分钟）
- 需要约 1-2GB 临时磁盘空间
- 确保网络连接稳定（下载 RocksDB 源码）

### 3. 配置数据库路径

**Windows 路径格式**:

```rust
use vm_runtime::{RocksDBConfig, RocksDBStorage};

// 方式1: 使用反斜杠（需要转义）
let config = RocksDBConfig::default()
    .with_path("D:\\\\SuperVM\\\\data\\\\rocksdb");

// 方式2: 使用正斜杠（推荐）
let config = RocksDBConfig::default()
    .with_path("D:/SuperVM/data/rocksdb");

// 方式3: 使用相对路径
let config = RocksDBConfig::default()
    .with_path("./data/rocksdb");

let mut storage = RocksDBStorage::new(config)?;
```

**推荐路径配置**:

| 环境 | 路径 | 说明 |
|------|------|------|
| 开发环境 | `./data/rocksdb` | 项目根目录下 |
| 测试环境 | `./data/test_rocksdb` | 独立测试数据 |
| 生产环境 | `D:/SuperVM/rocksdb` | 独立驱动器（推荐 SSD） |

### 4. 性能优化配置

#### 生产环境配置

```rust
let config = RocksDBConfig {
    path: "D:/SuperVM/rocksdb".to_string(),
    max_open_files: 20000,
    write_buffer_size: 256 * 1024 * 1024,  // 256MB
    block_cache_size: 1024 * 1024 * 1024,  // 1GB
    enable_compression: false,              // Windows 上禁用压缩避免依赖
    create_if_missing: true,
    max_background_jobs: 8,
};
```

#### 开发/测试配置

```rust
let config = RocksDBConfig::default()  // 使用默认值
    .with_path("./data/rocksdb_dev");
```

## 运行示例

### 1. 存储指标 HTTP 服务

启动集成 SuperVM 路由 + MVCC + RocksDB 指标的 HTTP 服务：

```powershell
cargo run --example storage_metrics_http --features rocksdb-storage --release
```

**访问端点**:
- `http://localhost:9091/metrics` - Prometheus 格式指标
- `http://localhost:9091/summary` - 文本格式统计摘要
- `http://localhost:9091/healthz` - 健康检查
- `http://localhost:9091/trigger?count=100&type=fast` - 触发测试事务

**指标示例**:
```
# RocksDB 内部指标
rocksdb_estimate_num_keys 1250
rocksdb_total_sst_size_bytes 524288
rocksdb_cache_hit 8423
rocksdb_cache_miss 157
rocksdb_compaction_cpu_micros 125000
rocksdb_write_stall_micros 0
rocksdb_num_files_level0 3
rocksdb_num_immutable_mem_table 0
```

### 2. 持久化一致性测试

验证 write → restart → verify 流程：

```powershell
cargo run --example persistence_consistency_test --features rocksdb-storage --release
```

**测试输出**:
```
=== Persistence Consistency Test ===
测试流程: Write → Restart → Verify

📝 Phase 1: 写入阶段
   ✅ 数据库初始化成功
   ✅ 写入完成: 100 条记录

🔄 Phase 2: 重启阶段
   模拟系统重启，等待 2 秒...

🔍 Phase 3: 验证阶段
   ✅ 数据库重新打开成功
   📊 验证结果:
      ✅ 成功匹配: 100/100
      ❌ 值不匹配: 0
      ⚠️  数据丢失: 0

✅ PASS - 持久化一致性验证通过
```

**生成的文件**:
- `data/persistence_test/consistency_test_report.txt` - 测试报告
- `data/persistence_test/expected_manifest.txt` - 预期数据清单
- `data/persistence_test/*.sst` - RocksDB SST 文件

### 3. RocksDB 指标采集演示

```powershell
cargo run --example rocksdb_metrics_demo --features rocksdb-storage
```

周期性采集 RocksDB 内部指标并更新到 MetricsCollector。

## 常见问题 (FAQ)

### Q1: 编译时出现 "link.exe not found"

**原因**: 未安装 MSVC 编译器或未加载环境变量。

**解决方案**:
```powershell
# 方式1: 使用 Visual Studio Developer PowerShell
# 从开始菜单启动 "Developer PowerShell for VS 2022"

# 方式2: 手动加载 MSVC 环境
& "C:\Program Files\Microsoft Visual Studio\2022\Community\Common7\Tools\Launch-VsDevShell.ps1"

# 方式3: 安装 Build Tools for Visual Studio 2022
# https://visualstudio.microsoft.com/downloads/#build-tools-for-visual-studio-2022
```

### Q2: 编译时 RocksDB 长时间无响应

**原因**: 首次编译 RocksDB C++ 库需要下载源码并完整编译。

**解决方案**:
- 等待编译完成（5-10分钟）
- 确保网络连接稳定
- 使用 `--verbose` 查看详细进度:
  ```powershell
  cargo build --features rocksdb-storage --verbose
  ```

### Q3: 运行时出现 "Access Denied" 或路径权限错误

**原因**: Windows 文件权限或路径包含特殊字符。

**解决方案**:
```powershell
# 检查路径权限
icacls "D:\SuperVM\data\rocksdb"

# 授予完全控制权限
icacls "D:\SuperVM\data\rocksdb" /grant Users:F /T

# 避免路径中包含中文或特殊字符
# ❌ 错误: "./数据/rocksdb"
# ✅ 正确: "./data/rocksdb"
```

### Q4: 数据库文件被锁定无法删除

**原因**: RocksDB 实例未正确关闭。

**解决方案**:
```rust
// 确保显式 Drop
{
    let mut storage = RocksDBStorage::new(config)?;
    // ... 使用 storage
} // storage 在此处自动 Drop

// 或手动 Drop
drop(storage);
```

### Q5: 如何迁移现有数据库到新路径

```powershell
# 1. 停止所有使用数据库的进程

# 2. 复制整个数据库目录
xcopy /E /I /H "D:\old_path\rocksdb" "D:\new_path\rocksdb"

# 3. 更新配置中的路径
# let config = RocksDBConfig::default().with_path("D:/new_path/rocksdb");

# 4. 验证新路径可用
cargo run --example persistence_consistency_test --features rocksdb-storage --release
```

## 性能基准 (Windows)

**测试环境**:
- CPU: Intel i7-10700 @ 2.9GHz (8核16线程)
- RAM: 32GB DDR4
- SSD: NVMe PCIe 3.0

**持久化写入性能**:
- 单次写入延迟: ~50-100μs
- 批量写入 (100条): ~5-10ms
- 吞吐量: ~10K-20K writes/sec

**持久化读取性能**:
- 单次读取延迟: ~10-30μs
- 随机读取: ~30K-50K reads/sec
- 顺序读取: ~100K-200K reads/sec

**一致性测试**:
- Write → Restart → Verify: 100/100 通过
- 数据完整性: 0 丢失, 0 损坏

## 集成到 CI/CD

### GitHub Actions 示例

```yaml
name: RocksDB Persistence Test

on: [push, pull_request]

jobs:
  test-windows:
    runs-on: windows-latest
    steps:
      - uses: actions/checkout@v3
      
      - name: Setup Rust
        uses: actions-rs/toolchain@v1
        with:
          toolchain: stable
          override: true
      
      - name: Build with RocksDB
        run: cargo build --features rocksdb-storage --release
      
      - name: Run Persistence Test
        run: cargo run --example persistence_consistency_test --features rocksdb-storage --release
      
      - name: Upload Test Report
        uses: actions/upload-artifact@v3
        with:
          name: persistence-test-report
          path: data/persistence_test/consistency_test_report.txt
```

## 监控与维护

### Prometheus + Grafana 集成

**1. 启动指标服务**:
```powershell
cargo run --example storage_metrics_http --features rocksdb-storage --release
```

**2. 配置 Prometheus** (`prometheus.yml`):
```yaml
scrape_configs:
  - job_name: 'supervm_storage'
    static_configs:
      - targets: ['localhost:9091']
    metrics_path: '/metrics'
    scrape_interval: 5s
```

**3. Grafana 面板示例指标**:
- `rocksdb_estimate_num_keys` - 键数量
- `rocksdb_cache_hit / (rocksdb_cache_hit + rocksdb_cache_miss)` - 缓存命中率
- `rocksdb_total_sst_size_bytes` - 存储大小
- `rate(rocksdb_compaction_cpu_micros[1m])` - Compaction CPU 使用率

### 定期备份

```powershell
# 创建快照（需要实现 Checkpoint 功能）
# 当前可手动复制数据库文件
$timestamp = Get-Date -Format "yyyyMMdd_HHmmss"
xcopy /E /I /H "D:\SuperVM\rocksdb" "D:\Backup\rocksdb_$timestamp"
```

## 下一步

1. **扩展测试场景**:
   - 并发写入压力测试
   - 大数据量测试 (百万级键)
   - 异常中断恢复测试（模拟断电）

2. **性能调优**:
   - 启用 Bloom Filter 优化读取
   - 调整 Compaction 策略
   - 实现预写日志 (WAL) 持久化

3. **高可用部署**:
   - 主从复制方案
   - Checkpoint 自动备份
   - 灾难恢复流程

## 参考资料

- [RocksDB 官方文档](https://github.com/facebook/rocksdb/wiki)
- [rust-rocksdb](https://github.com/rust-rocksdb/rust-rocksdb)
- [SuperVM ROADMAP.md](../ROADMAP.md) - L0.2 存储抽象层
- [ARCHITECTURE.md](./ARCHITECTURE.md) - 架构设计文档
 - [RocksDB Linux 部署指南](./ROCKSDB-LINUX-DEPLOYMENT.md)
