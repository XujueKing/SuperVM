# RocksDB 持久化存储 - macOS 部署指南

> 适用于: macOS 12 Monterey / 13 Ventura / 14 Sonoma (Apple Silicon 与 Intel x86_64 均已验证)
> SuperVM 通过 `rust-rocksdb` 编译集成 RocksDB, 无需手工安装核心库; 仅需开发工具与可选压缩依赖。

## 1. 环境准备

### 1.1 安装 Xcode 命令行工具

```bash
xcode-select --install

```

### 1.2 安装 Homebrew (若未安装)

```bash
/bin/bash -c "$(curl -fsSL https://raw.githubusercontent.com/Homebrew/install/HEAD/install.sh)"

```

### 1.3 安装构建依赖

```bash
brew install cmake llvm pkg-config

# 可选压缩库:

brew install snappy lz4 zstd

```

### 1.4 安装/更新 Rust

```bash
curl https://sh.rustup.rs -sSf | sh
source "$HOME/.cargo/env"
rustup update stable

```

## 2. 构建 SuperVM (含 RocksDB)

```bash

# Apple Silicon 优化: target-cpu=native 会启用 NEON/CRC 指令

RUSTFLAGS="-C target-cpu=native" cargo build --release --features rocksdb-storage

```

## 3. 运行示例

```bash

# 指标服务

cargo run --release --example storage_metrics_http --features rocksdb-storage

# 持久化一致性测试

cargo run --release --example persistence_consistency_test --features rocksdb-storage

```

访问:

- http://localhost:9091/metrics

- http://localhost:9091/summary

## 4. 常见问题 (FAQ)

### Q1: 编译提示找不到 `libclang`

```bash
brew install llvm
export LIBCLANG_PATH="$(brew --prefix llvm)/lib"

```

### Q2: M1/M2 架构性能偏低

- 确认已加 `RUSTFLAGS="-C target-cpu=native"`

- 避免在 Rosetta 转译 shell 中运行 (使用原生 arm64 终端)

### Q3: 需要全静态吗?

- macOS 环境一般不建议强行全静态；默认动态链接系统 libc 即可。

### Q4: 提升 I/O 性能建议

- 使用 APFS SSD (NVMe 优先)

- 避免 Spotlight 索引数据库目录: `mdutil -i off /path/to/rocksdb`

## 5. 性能参考 (Apple M2 Pro)

| 项目 | 数值 (示例) |
|------|-------------|
| 单键写延迟 | ~40–70 µs |
| 顺序读吞吐 | 150K ops/s |
| 随机读命中率 (1GB Cache) | 60%+ |

## 6. 与 Linux / Windows 区别

| 维度 | macOS | Linux | Windows |
|------|-------|-------|---------|
| 构建链 | Xcode CLT + brew | build-essential/clang | MSVC + CMake |
| 全静态可行性 | 弱 (不推荐) | 可选 (crt-static) | 一般不需要 |
| 压缩库装法 | brew | apt/dnf | 手工或关闭 |

## 7. 一键引导 (推荐)

```bash
./scripts/bootstrap.sh

# 或指定自定义目录:

DB_PATH=./data/rocksdb_dev FEATURES=rocksdb-storage ./scripts/bootstrap.sh

```

## 8. 下一步

- 支持多 Column Family 进行冷热数据分层

- 集成 WAL 策略可配置 (fsync 间隔)

- 自动后台 Compaction 监控与调参建议

## 9. 参考

- https://github.com/facebook/rocksdb

- https://github.com/rust-rocksdb/rust-rocksdb

- docs/ROCKSDB-LINUX-DEPLOYMENT.md

- docs/ROCKSDB-WINDOWS-DEPLOYMENT.md
