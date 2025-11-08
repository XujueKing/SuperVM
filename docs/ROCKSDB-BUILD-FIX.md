# RocksDB 编译问题修复指南

## ❌ 问题

编译 RocksDB 时出现错误:
```
Unable to find libclang: "couldn't find any valid shared libraries matching: 
['clang.dll', 'libclang.dll'], set the `LIBCLANG_PATH` environment variable"
```

## 🔍 原因

RocksDB 的 Rust 绑定 (`rocksdb` crate) 依赖 `bindgen`,而 `bindgen` 需要 LLVM/Clang 来解析 C++ 头文件。

## ✅ 解决方案

### 方案 1: 使用预编译的 RocksDB (推荐 ⭐)

修改 `Cargo.toml`,禁用需要编译的压缩库:

```toml
[dependencies]
# 使用系统 RocksDB (如果可用) 或禁用某些压缩
rocksdb = { version = "0.22", optional = true, default-features = false, features = ["lz4"] }
```

**优点**: 避免复杂的 C++ 编译
**缺点**: 可能缺少某些压缩算法

---

### 方案 2: 安装 LLVM/Clang (完整支持)

1. **下载 LLVM**:
   - 访问: https://github.com/llvm/llvm-project/releases
   - 下载: `LLVM-<version>-win64.exe`
   - 推荐版本: LLVM 15.0+ 或 17.0+

2. **安装 LLVM**:
   - 运行安装程序
   - 选择 "Add LLVM to system PATH"
   - 记住安装路径 (如 `C:\Program Files\LLVM`)

3. **设置环境变量**:
   ```powershell
   # 临时设置 (当前会话)
   $env:LIBCLANG_PATH = "C:\Program Files\LLVM\bin"
   
   # 或者永久设置
   [Environment]::SetEnvironmentVariable("LIBCLANG_PATH", "C:\Program Files\LLVM\bin", "User")
   ```

4. **重新编译**:
   ```powershell
   cargo clean -p vm-runtime
   cargo build -p vm-runtime --features rocksdb-storage --release
   ```

**优点**: 完整功能,支持所有压缩算法
**缺点**: 需要下载安装 LLVM (~500MB)

---

### 方案 3: 使用 MemoryStorage (临时绕过)

在 Week 1 完成编译问题之前,可以继续使用 `MemoryStorage` 进行其他开发:

```rust
// 使用内存存储 (无需 RocksDB)
use vm_runtime::{MemoryStorage, Storage};

let mut storage = MemoryStorage::new();
storage.set(b"key", b"value")?;
```

等 RocksDB 编译成功后再切换。

---

### 方案 4: 使用 Docker/WSL (Linux 环境)

RocksDB 在 Linux 环境下编译更顺畅:

```bash
# WSL2 Ubuntu
sudo apt-get install clang libclang-dev
cargo build -p vm-runtime --features rocksdb-storage
```

---

## 🚀 推荐行动方案

### 快速方案 (5分钟):
**方案 1**: 简化依赖,禁用 zstd 压缩

```toml
# src/vm-runtime/Cargo.toml
[dependencies]
rocksdb = { version = "0.22", optional = true, default-features = false }
```

### 完整方案 (30分钟):
**方案 2**: 安装 LLVM,获得完整功能

---

## 📝 当前状态

编译进度:
- ✅ librocksdb-sys 编译成功 (RocksDB C++ 库)
- ❌ zstd-sys 编译失败 (缺少 libclang)
- 🚧 rocksdb crate 等待中

---

## 🔄 修复后验证

```powershell
# 1. 清理旧的编译产物
cargo clean -p vm-runtime

# 2. 重新编译
cargo build -p vm-runtime --features rocksdb-storage --release

# 3. 运行测试
cargo test -p vm-runtime --features rocksdb-storage --lib rocksdb

# 4. 运行演示
cargo run -p node-core --example rocksdb_demo --features rocksdb-storage --release
```

---

## 💡 Week 1 替代方案

如果 RocksDB 编译问题短期无法解决,可以:

1. **继续 Week 2-4 的其他工作**:
   - AutoTuner 优化 (不依赖 RocksDB)
   - MVCC 高竞争优化 (Phase 4.1)
   - 文档编写

2. **使用 MemoryStorage 验证接口**:
   - Storage trait 已经抽象好
   - 上层代码与存储后端解耦
   - RocksDB 编译成功后无缝切换

3. **在 Linux/WSL 环境中完成 RocksDB 集成**:
   - 更好的 C++ 工具链支持
   - 编译更快更稳定

---

## 🎯 决策建议

**我的建议**: 先使用 **方案 1 (简化依赖)** 快速验证功能,Week 2 再考虑安装 LLVM 获得完整功能。

```powershell
# 立即尝试方案 1
# 编辑 src/vm-runtime/Cargo.toml,修改 rocksdb 依赖
# 然后重新编译
```

需要我帮你修改 Cargo.toml 吗?
