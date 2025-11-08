# Phase 4.3: RocksDB 持久化存储集成

**开发者**: king  
**开始时间**: 2024-11-07  
**状态**: 🚧 Week 1 进行中  
**预计完成**: 2024-12-05 (4周)

---

## 📋 目标

为 SuperVM 集成 RocksDB 持久化存储后端,替代当前的 MemoryStorage,实现生产级状态管理能力。

### 🎯 核心目标
- ✅ 持久化存储: 重启后数据不丢失
- ✅ 高性能: 随机读 ≥100K ops/s, 批量写 ≥200K ops/s
- ✅ 快照管理: 支持 Checkpoint 创建和恢复
- ✅ 状态裁剪: 支持历史数据清理
- ✅ 生产就绪: 完整测试 + 24小时稳定性验证

---

## 🗓️ 实施计划

### Week 1: RocksDB 基础集成 (✅ 进行中)

**任务清单**:
- [x] 添加 rocksdb 依赖 (v0.22)
- [x] 创建 `RocksDBStorage` 结构体
- [x] 实现 `Storage` trait
  - [x] `get(&self, key: &[u8]) -> Result<Option<Vec<u8>>>`
  - [x] `set(&mut self, key: &[u8], value: &[u8]) -> Result<()>`
  - [x] `delete(&mut self, key: &[u8]) -> Result<()>`
  - [x] `scan(&self, prefix: &[u8]) -> Result<Vec<(Vec<u8>, Vec<u8>)>>`
- [x] 基础配置
  - [x] `max_open_files = 10000`
  - [x] `compression = LZ4`
  - [x] `block_cache = 512MB`
  - [x] `write_buffer_size = 128MB`
- [x] 基础单元测试
  - [x] `test_rocksdb_basic_operations`
  - [x] `test_rocksdb_scan`
  - [x] `test_rocksdb_persistence`
- [x] 创建演示示例 (`rocksdb_demo.rs`)
- [ ] 编译验证
- [ ] 运行基础测试

**交付物**:
- `src/vm-runtime/src/storage/rocksdb_storage.rs` - 核心实现
- `src/node-core/examples/rocksdb_demo.rs` - 使用示例
- 基础单元测试覆盖

---

### Week 2: 配置优化与批量操作 (📋 待开始)

**任务清单**:
- [ ] WriteBatch 实现
  - [ ] `write_batch(batch: Vec<(Vec<u8>, Option<Vec<u8>>)>) -> Result<()>`
  - [ ] 原子性保证
  - [ ] 性能测试
- [ ] 配置调优
  - [ ] 压缩策略优化 (LZ4 vs Snappy vs Zstd)
  - [ ] Block Cache 大小调优
  - [ ] Write Buffer 大小调优
  - [ ] Background Jobs 配置
- [ ] 性能基准测试
  - [ ] 随机读 QPS 测试
  - [ ] 随机写 QPS 测试
  - [ ] 批量写 QPS 测试
  - [ ] 扫描性能测试
- [ ] 监控指标
  - [ ] `get_property()` 集成
  - [ ] 统计数据输出
  - [ ] 性能仪表板设计

**交付物**:
- WriteBatch 高性能实现
- 性能基准测试报告
- 配置调优指南

**性能目标验证**:
- 随机读: ≥ 100K ops/s ✅
- 随机写: ≥ 50K ops/s ✅
- 批量写: ≥ 200K ops/s ✅
- 扫描: ≥ 500 MB/s ✅

---

### Week 3: 快照与状态管理 (📋 待开始)

**任务清单**:
- [ ] Checkpoint 快照
  - [ ] `create_checkpoint(path) -> Result<()>`
  - [ ] 快照恢复测试
  - [ ] 增量快照支持
- [ ] 状态裁剪 (Pruning)
  - [ ] `prune_before(timestamp) -> Result<()>`
  - [ ] 保留策略配置
  - [ ] 空间回收验证
- [ ] 历史查询
  - [ ] 时间点查询 API
  - [ ] 历史版本管理
- [ ] 压缩管理
  - [ ] `compact_range()` 实现
  - [ ] 自动压缩策略
  - [ ] 手动压缩工具

**交付物**:
- Checkpoint 完整实现
- 状态裁剪机制
- 压缩管理工具

---

### Week 4: 测试与文档 (📋 待开始)

**任务清单**:
- [ ] 完整单元测试
  - [ ] Storage trait 兼容性测试
  - [ ] 边界条件测试
  - [ ] 并发安全测试
  - [ ] 错误处理测试
- [ ] 集成测试
  - [ ] 与 MVCC 调度器集成
  - [ ] 与 Runtime 集成
  - [ ] 端到端测试
- [ ] 稳定性测试
  - [ ] 24 小时压力测试
  - [ ] 内存泄漏检测
  - [ ] 崩溃恢复测试
  - [ ] 数据完整性验证
- [ ] 文档编写
  - [ ] API 文档
  - [ ] 使用指南
  - [ ] 配置参考
  - [ ] 迁移指南 (MemoryStorage → RocksDB)
  - [ ] 性能调优指南
  - [ ] 故障排查手册

**交付物**:
- 完整测试套件 (覆盖率 ≥ 90%)
- 24小时稳定性测试报告
- 完整使用文档

---

## 📊 技术方案

### 1. 架构设计

```rust
// Storage Trait (已存在)
pub trait Storage {
    fn get(&self, key: &[u8]) -> Result<Option<Vec<u8>>>;
    fn set(&mut self, key: &[u8], value: &[u8]) -> Result<()>;
    fn delete(&mut self, key: &[u8]) -> Result<()>;
    fn scan(&self, prefix: &[u8]) -> Result<Vec<(Vec<u8>, Vec<u8>)>>;
}

// RocksDB 实现
pub struct RocksDBStorage {
    db: Arc<DB>,
    config: RocksDBConfig,
}

// 配置
pub struct RocksDBConfig {
    pub path: String,
    pub max_open_files: i32,
    pub write_buffer_size: usize,
    pub block_cache_size: usize,
    pub enable_compression: bool,
    pub max_background_jobs: i32,
}
```

### 2. 特性开关

```toml
[features]
rocksdb-storage = ["dep:rocksdb"]
```

### 3. 使用示例

```rust
use vm_runtime::{RocksDBStorage, RocksDBConfig, Storage};

// 使用默认配置
let storage = RocksDBStorage::new_default()?;

// 自定义配置
let config = RocksDBConfig {
    path: "./data/supervm".to_string(),
    max_open_files: 10000,
    write_buffer_size: 256 * 1024 * 1024,  // 256MB
    block_cache_size: 1024 * 1024 * 1024,  // 1GB
    enable_compression: true,
    max_background_jobs: 8,
};
let storage = RocksDBStorage::new(config)?;

// 基础操作
storage.set(b"key", b"value")?;
let value = storage.get(b"key")?;

// 批量写入
let batch = vec![
    (b"key1".to_vec(), Some(b"value1".to_vec())),
    (b"key2".to_vec(), Some(b"value2".to_vec())),
    (b"key3".to_vec(), None),  // 删除
];
storage.write_batch(batch)?;

// 快照
storage.create_checkpoint("./snapshots/checkpoint_001")?;
```

---

## 🎯 性能目标

### 基准环境
- CPU: Intel Core i7-9750H @ 2.60GHz (6 核 12 线程)
- OS: Windows 11
- 磁盘: NVMe SSD

### 目标 QPS

| 操作类型 | 目标 QPS | 验收标准 |
|---------|----------|---------|
| 随机读 | ≥ 100K ops/s | SSD 环境 |
| 随机写 | ≥ 50K ops/s | SSD 环境 |
| 批量写 | ≥ 200K ops/s | WriteBatch |
| 扫描 | ≥ 500 MB/s | 顺序读取 |
| P99 延迟 | < 10 ms | 99分位 |

### 压缩比
- 目标: 2-5x (LZ4 压缩)
- 验收: 实测压缩比 ≥ 2x

---

## ✅ 当前进度

### Week 1: RocksDB 基础集成 (✅ 90%)

**已完成**:
- ✅ 添加 rocksdb 依赖 (v0.22)
- ✅ 创建 RocksDBStorage 结构体
- ✅ 实现 Storage trait (get/set/delete/scan)
- ✅ 基础配置 (压缩、缓存、写缓冲)
- ✅ WriteBatch 实现
- ✅ Checkpoint 快照实现
- ✅ 5个单元测试
- ✅ 演示示例 (rocksdb_demo.rs)

**进行中**:
- 🚧 编译验证 (RocksDB C++ 依赖编译中...)

**待完成**:
- ⏳ 运行单元测试
- ⏳ 运行演示示例
- ⏳ 性能基准测试

---

## 📝 开发日志

### 2024-11-07

**上午**:
- ✅ 创建 Phase 4.3 实施计划
- ✅ 添加 rocksdb 依赖到 Cargo.toml
- ✅ 创建 `src/vm-runtime/src/storage/rocksdb_storage.rs`
- ✅ 实现 Storage trait 所有方法
- ✅ 实现 WriteBatch 高性能批量写入
- ✅ 实现 Checkpoint 快照管理
- ✅ 添加 5 个单元测试
- ✅ 创建 `rocksdb_demo.rs` 演示示例
- 🚧 启动编译 (RocksDB C++ 依赖编译中...)

**下一步**:
- ⏳ 等待编译完成
- ⏳ 运行单元测试验证
- ⏳ 运行演示示例
- ⏳ 收集性能数据

---

## 🔗 相关资源

### 文档
- RocksDB Wiki: https://github.com/facebook/rocksdb/wiki
- rust-rocksdb: https://github.com/rust-rocksdb/rust-rocksdb

### 参考实现
- Ethereum Geth: LevelDB → RocksDB 迁移
- Sui Move: RocksDB 存储层
- Solana: RocksDB 账本存储

---

## 📌 注意事项

### 1. 编译要求
- RocksDB 需要 C++ 编译器 (MSVC on Windows)
- 首次编译时间较长 (5-10 分钟)
- 需要 ~500MB 磁盘空间

### 2. 配置建议
- SSD 环境: 启用 LZ4 压缩
- HDD 环境: 考虑使用 Snappy 或无压缩
- 高并发: 增加 max_background_jobs
- 内存充足: 增加 block_cache_size

### 3. 迁移路径
- MemoryStorage → RocksDB 兼容 Storage trait
- 无需修改上层代码
- 只需在创建 Runtime 时切换存储后端

---

## 🎉 预期成果

### 功能成果
- ✅ 生产级持久化存储
- ✅ 100K+ 读 QPS, 200K+ 批量写 QPS
- ✅ 快照管理能力
- ✅ 状态裁剪机制
- ✅ 完整测试覆盖

### 文档成果
- ✅ API 文档
- ✅ 使用指南
- ✅ 性能调优指南
- ✅ 迁移指南

### 影响
- 🚀 SuperVM 具备生产就绪能力
- 🚀 可运行长期稳定的节点
- 🚀 支持大规模状态管理
- 🚀 为 Phase 6 (四层网络) 奠定基础
