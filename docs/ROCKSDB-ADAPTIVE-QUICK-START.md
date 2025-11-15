# RocksDB 自适应批量写入 - 快速开始指南

> Phase 4.3 Week 2: 持久化存储批量写入性能优化

## 📦 环境准备

### 1. 安装依赖

**Windows 环境**:

```powershell

# 安装 LLVM 17（RocksDB 编译需要）

# 下载地址: https://github.com/llvm/llvm-project/releases/download/llvmorg-17.0.1/LLVM-17.0.1-win64.exe

# 安装后设置环境变量

$env:LIBCLANG_PATH = "C:\Program Files\LLVM\bin"

```

**Linux/macOS**:

```bash

# Ubuntu/Debian

sudo apt-get install llvm-17 libclang-17-dev

# macOS

brew install llvm@17
export LIBCLANG_PATH="/opt/homebrew/opt/llvm@17/lib"

```

### 2. 编译项目

```powershell

# 完整编译（包含 RocksDB 支持）

cargo build --release --features rocksdb-storage

# 仅编译示例（更快）

cargo build --release -p vm-runtime --features rocksdb-storage --example rocksdb_adaptive_batch_bench
cargo build --release -p vm-runtime --features rocksdb-storage --example rocksdb_adaptive_compare

```

---

## 🚀 快速运行

### 示例 1: 自适应批量基准

**默认参数运行**:

```powershell

# Windows

.\target\release\examples\rocksdb_adaptive_batch_bench.exe

# Linux/macOS

./target/release/examples/rocksdb_adaptive_batch_bench

```

**输出示例**:

```

=== 自适应分批基准 ===
配置: min_chunk=1000, max_chunk=60000, target_rsd=8.0%, adjust_up=0.15, adjust_down=0.30, window=6
N= 10000 | total_qps= 896185.83 | chunks= 7 final_chunk= 1000 | window_avg_qps=2513155.39 (RSD=14.55%) BEST=3067484.66
N= 50000 | total_qps= 903141.67 | chunks=16 final_chunk= 3215 | window_avg_qps=2282792.12 (RSD=20.29%) BEST=2674041.93
N=100000 | total_qps= 860176.58 | chunks=36 final_chunk= 2660 | window_avg_qps=2429536.47 (RSD=24.79%) BEST=2960405.81

✓ 结果已追加到: adaptive_bench_results.csv

```

**自定义参数（环境变量）**:

```powershell

# 设置自定义参数

$env:ADAPT_WINDOW = 8              # 加大窗口平滑（默认 6）
$env:ADAPT_DOWN_PCT = 0.25         # 更保守的缩小（默认 0.30）
$env:ADAPT_TARGET_RSD = 6.0        # 更严格的稳定性目标（默认 8.0）
$env:ADAPT_MIN_CHUNK = 2000        # 提高最小 chunk（默认 1000）
$env:ADAPT_MAX_CHUNK = 80000       # 提高最大 chunk（默认 60000）
$env:ADAPT_CSV = "my_results.csv"  # 自定义输出文件

# 运行

.\target\release\examples\rocksdb_adaptive_batch_bench.exe

# 清除环境变量

Remove-Item Env:\ADAPT_*

```

---

### 示例 2: 策略对比基准

**运行对比测试**:

```powershell

# 对比 Monolithic / Chunked / Adaptive 三种策略

# 每种策略运行 3 次取平均，WAL on/off 两组配置

.\target\release\examples\rocksdb_adaptive_compare.exe

```

**输出示例**:

```

=== RocksDB 批量写入策略对比 ===

┌─────────────────────────────────────────────────────────────────────────────
│ WAL OFF 配置
└─────────────────────────────────────────────────────────────────────────────

批量大小: N = 50000
  [Monolithic] AVG:  714568.17 | BEST:  885948.33 | STD: 121220.10 | RSD: 16.96%
  [Chunked-5K] AVG:  563423.43 | BEST:  581628.68 | STD: 12892.82 | RSD:  2.29% | Gain:  -21.2%
  [Adaptive]    AVG:  767317.76 | BEST:  893153.80 | STD: 127537.87 | RSD: 16.62% | Gain:   +7.4%
                ↳ chunks=27, final_chunk=1150, window_rsd=36.32%

  📊 策略对比（相对 Monolithic）:
     Chunked:  吞吐  -21.2% | RSD  -14.7%
     Adaptive: 吞吐   +7.4% | RSD   -0.3%
...
✓ 结果已保存到: adaptive_compare_results.csv

```

---

### 示例 3: 运行时监控

**实时查看 RocksDB 统计**:

```powershell

# 每 5 秒输出统计信息（需手动 Ctrl+C 停止）

.\target\release\examples\rocksdb_monitor.exe

```

**输出示例**:

```

=== RocksDB Stats ===
Keys: 150000
SST Size: 42.35 MB
Block Cache Hit Rate: 87.53%

```

---

## 💻 代码集成示例

### 基础用法

```rust
use vm_runtime::RocksDBStorage;
use anyhow::Result;

fn main() -> Result<()> {
    // 创建存储实例
    let storage = RocksDBStorage::new_with_path("./data/my_db")?;
    
    // 准备批量数据
    let mut batch = Vec::new();
    for i in 0..100_000 {
        let key = format!("key_{}", i).into_bytes();
        let value = vec![0u8; 256];
        batch.push((key, Some(value)));
    }
    
    // 方式 1: 单体批量（最简单，但 RSD 可能较高）
    storage.write_batch_with_options(batch.clone(), true, false)?;
    
    // 方式 2: 固定分块（稳定性好，吞吐略降）
    storage.write_batch_chunked(batch.clone(), 5_000, true, false)?;
    
    // 方式 3: 自适应（推荐，自动调优）
    storage.write_batch_adaptive(batch, true, false)?;
    
    Ok(())
}

```

### 高级配置

```rust
use vm_runtime::{RocksDBStorage, AdaptiveBatchConfig};
use anyhow::Result;

fn main() -> Result<()> {
    let storage = RocksDBStorage::new_with_path("./data/my_db")?;
    let batch = prepare_batch(100_000);
    
    // 自定义自适应参数
    let mut cfg = AdaptiveBatchConfig::default_for(&batch);
    cfg.window = 8;                 // 大批量场景加大窗口
    cfg.target_rsd_pct = 6.0;       // 更严格的稳定性目标
    cfg.adjust_down_pct = 0.25;     // 更保守的缩小策略
    cfg.min_chunk = 2_000;          // 提高最小 chunk
    cfg.max_chunk = 80_000;         // 提高最大 chunk
    
    // 执行自适应写入
    let result = storage.write_batch_adaptive_with_config(
        batch,
        cfg,
        true,   // disable_wal: 禁用 WAL 提升吞吐
        false   // sync: 不强制同步
    )?;
    
    // 查看结果统计
    println!("Chunks: {}", result.chunks);
    println!("Final chunk size: {}", result.final_chunk);
    println!("AVG QPS: {:.2}", result.avg_qps);
    println!("RSD: {:.2}%", result.rsd_pct);
    
    Ok(())
}

fn prepare_batch(n: usize) -> Vec<(Vec<u8>, Option<Vec<u8>>)> {
    (0..n).map(|i| {
        (format!("key_{}", i).into_bytes(), Some(vec![0u8; 256]))
    }).collect()
}

```

---

## 📊 性能调优建议

### 场景 1: 小批量高频写入（< 10K）

**推荐策略**: Chunked 固定分块

```rust
// chunk_size = 1000 左右，RSD < 1%
storage.write_batch_chunked(batch, 1_000, true, false)?;

```

**预期性能**:

- QPS: 750K+

- RSD: < 1%

- 稳定性: ⭐⭐⭐⭐⭐

---

### 场景 2: 中批量混合负载（10K - 50K）

**推荐策略**: Adaptive 自适应

```rust
// 默认配置即可，自动探测最优 chunk
storage.write_batch_adaptive(batch, true, false)?;

```

**预期性能**:

- QPS: 650K - 900K

- RSD: 12% - 20%

- 稳定性: ⭐⭐⭐⭐

- 灵活性: ⭐⭐⭐⭐⭐

---

### 场景 3: 大批量一次性导入（> 100K）

**推荐策略**: Adaptive + 自定义参数

```rust
let mut cfg = AdaptiveBatchConfig::default_for(&batch);
cfg.window = 10;              // 加大窗口平滑 RocksDB flush/compaction
cfg.target_rsd_pct = 10.0;    // 放宽 RSD 目标，优先吞吐
cfg.adjust_down_pct = 0.20;   // 更保守的缩小，避免过度震荡
cfg.max_chunk = 100_000;      // 提高上限，允许更大 chunk

storage.write_batch_adaptive_with_config(batch, cfg, true, false)?;

```

**预期性能**:

- QPS: 800K+

- RSD: 15% - 25%

- 稳定性: ⭐⭐⭐

- 吞吐: ⭐⭐⭐⭐⭐

---

## 🔍 数据分析

### 查看 CSV 结果

**自适应基准数据**:

```powershell

# 查看最新 10 条记录

Get-Content adaptive_bench_results.csv | Select-Object -Last 10

# 导入 Excel 分析

# adaptive_bench_results.csv 包含列:

# timestamp, batch_size, total_qps, chunks, final_chunk, window_avg_qps, rsd_pct, best_qps, min_chunk, max_chunk, target_rsd, adjust_up, adjust_down, window

```

**策略对比数据**:

```powershell

# 查看对比结果

Get-Content adaptive_compare_results.csv | Select-Object -Last 10

# adaptive_compare_results.csv 包含列:

# timestamp, batch_size, wal_enabled, strategy, iter, total_qps, avg_qps, best_qps, stddev_qps, rsd_pct, chunks, final_chunk

```

### 性能对比分析

**按策略筛选**:

```powershell

# 查看所有 Adaptive 策略结果

Import-Csv adaptive_compare_results.csv | Where-Object { $_.strategy -eq "adaptive" } | Format-Table

```

**计算平均值**:

```powershell

# 计算 100K 批量下 Adaptive 策略的平均 QPS

Import-Csv adaptive_compare_results.csv | 
    Where-Object { $_.batch_size -eq 100000 -and $_.strategy -eq "adaptive" } | 
    Measure-Object -Property total_qps -Average

```

---

## 🛠️ 故障排查

### 问题 1: 编译失败 - 找不到 libclang

**错误信息**:

```

error: failed to run custom build command for `librocksdb-sys`
Unable to find libclang

```

**解决方案**:

```powershell

# 1. 安装 LLVM 17

# 下载: https://github.com/llvm/llvm-project/releases/download/llvmorg-17.0.1/LLVM-17.0.1-win64.exe

# 2. 设置环境变量（临时）

$env:LIBCLANG_PATH = "C:\Program Files\LLVM\bin"

# 3. 设置环境变量（永久）

[System.Environment]::SetEnvironmentVariable("LIBCLANG_PATH", "C:\Program Files\LLVM\bin", "User")

# 4. 重新编译

cargo clean
cargo build --release --features rocksdb-storage

```

---

### 问题 2: 磁盘空间不足

**错误信息**:

```

error: 磁盘空间不足。 (os error 112)

```

**解决方案**:

```powershell

# 清理 target 目录（释放 ~18GB）

cargo clean

# 或仅清理 release 构建

Remove-Item -Recurse -Force target\release

# 禁用增量编译（减少磁盘占用）

$env:CARGO_INCREMENTAL = 0
cargo build --release --features rocksdb-storage

```

---

### 问题 3: RSD 过高（> 30%）

**现象**: 大批量写入时 RSD 持续 > 30%

**调优方案**:

**方案 A - 加大窗口平滑**:

```powershell
$env:ADAPT_WINDOW = 10
$env:ADAPT_DOWN_PCT = 0.20
.\target\release\examples\rocksdb_adaptive_batch_bench.exe

```

**方案 B - 降低目标 RSD**:

```powershell
$env:ADAPT_TARGET_RSD = 10.0   # 放宽目标，优先吞吐
.\target\release\examples\rocksdb_adaptive_batch_bench.exe

```

**方案 C - 使用固定分块**:

```rust
// 改用 Chunked 策略，RSD 更低但吞吐略降
storage.write_batch_chunked(batch, 5_000, true, false)?;

```

---

## 📖 相关文档

- **集成文档**: `docs/PHASE-4.3-ROCKSDB-INTEGRATION.md`

- **构建修复**: `docs/ROCKSDB-BUILD-FIX.md`

- **性能报告**: `BENCHMARK_RESULTS.md` (Section 4)

- **API 文档**: `docs/API.md`

---

## 🎯 性能目标参考

| 批量大小 | 推荐策略 | 目标 QPS | 目标 RSD | 实测达成 |
|---------|---------|----------|----------|----------|
| 10K | Chunked | > 200K | < 5% | ✅ 754K / 0.26% |
| 50K | Adaptive | > 200K | < 15% | ✅ 646K / 12.2% |
| 100K | Adaptive | > 200K | < 25% | ✅ 860K / 24.8% |

---

## 💡 最佳实践

1. **小批量（< 10K）**: 使用 `write_batch_chunked`，chunk=1000，获得极致稳定性
2. **中批量（10-50K）**: 使用 `write_batch_adaptive`，默认配置即可
3. **大批量（> 100K）**: 自定义 `AdaptiveBatchConfig`，加大 window 和 max_chunk
4. **生产环境**: WAL 按需启用（数据重要性 vs 吞吐权衡）
5. **持续监控**: 定期运行 `rocksdb_monitor` 观察 cache hit rate
6. **参数调优**: 使用环境变量快速迭代，无需重编译

---

## 🚦 快速检查清单

- [ ] LLVM 17 已安装且 LIBCLANG_PATH 已设置

- [ ] `cargo build --release --features rocksdb-storage` 编译通过

- [ ] 运行 `rocksdb_adaptive_batch_bench` 输出 CSV 文件

- [ ] 运行 `rocksdb_adaptive_compare` 查看策略对比

- [ ] QPS > 200K 且 RSD 符合场景预期

- [ ] 根据场景选择合适策略（Chunked / Adaptive）

- [ ] 生产部署前在真实数据上跑对比基准

---

**需要帮助？** 查看 `BENCHMARK_RESULTS.md` Section 4 或联系开发团队。
