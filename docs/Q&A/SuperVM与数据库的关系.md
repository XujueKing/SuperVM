# SuperVM 与数据库的关系

> **作者**: KING XU (CHINA) | **创建时间**: 2025-11-06

---

## 📋 目录

1. [核心概念](#核心概念)
2. [存储架构](#存储架构)
3. [当前实现](#当前实现)
4. [数据库集成方案](#数据库集成方案)
5. [与传统数据库的对比](#与传统数据库的对比)
6. [未来规划](#未来规划)

---

## 🎯 核心概念

### SuperVM 是什么?

```

SuperVM = 区块链虚拟机 (Blockchain VM)

- 执行智能合约

- 管理区块链状态

- 处理交易

≠ 数据库管理系统 (DBMS)

```

### 与数据库的关系

```

┌─────────────────────────────────────────────────┐
│              应用层 (DApp)                      │
├─────────────────────────────────────────────────┤
│         SuperVM (虚拟机执行层)                  │
│  - WASM 执行                                    │
│  - MVCC 并发控制                                │
│  - 交易调度                                     │
├─────────────────────────────────────────────────┤
│      Storage Trait (存储抽象层) 🔑              │
│  trait Storage {                                │
│    fn get(&self, key: &[u8]) -> Option<Vec<u8>>│
│    fn set(&mut self, key: &[u8], value: &[u8]) │
│    fn delete(&mut self, key: &[u8])            │
│  }                                              │
├─────────────────────────────────────────────────┤
│       持久化存储层 (可选)                       │
│  - RocksDB (推荐)      ← 数据库在这里           │
│  - LevelDB             ← 数据库在这里           │
│  - LMDB                ← 数据库在这里           │
│  - PostgreSQL          ← 数据库在这里           │
│  - MemoryStorage (测试)                         │
└─────────────────────────────────────────────────┘

关系: SuperVM 通过 Storage Trait 使用数据库
      数据库是存储后端,VM 是执行引擎

```

---

## 🏗️ 存储架构

### 1. **Storage Trait - 存储抽象层**

SuperVM 定义了统一的存储接口:

```rust
// 文件: src/vm-runtime/src/storage.rs

/// 存储接口,定义了虚拟机可以使用的存储操作
pub trait Storage {
    /// 根据键获取值
    fn get(&self, key: &[u8]) -> Result<Option<Vec<u8>>>;

    /// 设置键值对
    fn set(&mut self, key: &[u8], value: &[u8]) -> Result<()>;

    /// 删除键值对
    fn delete(&mut self, key: &[u8]) -> Result<()>;

    /// 根据前缀扫描键值对
    fn scan(&self, prefix: &[u8]) -> Result<Vec<(Vec<u8>, Vec<u8>)>>;
}

```

**设计优势**:

- ✅ **抽象解耦**: VM 逻辑与存储实现完全解耦

- ✅ **可插拔**: 可切换任意符合 Trait 的存储后端

- ✅ **类型安全**: Rust Trait 保证编译时类型检查

- ✅ **可测试**: 可用内存存储进行单元测试

### 2. **Runtime 与 Storage 的集成**

```rust
// 文件: src/vm-runtime/src/lib.rs

/// VM 运行时的主要接口
pub struct Runtime<S: Storage = MemoryStorage> {
    engine: Engine,
    storage: Rc<RefCell<S>>,  // ← 存储后端
    ownership_manager: Option<Arc<OwnershipManager>>,
    scheduler: Option<Arc<MvccScheduler>>,
}

impl<S: Storage + 'static> Runtime<S> {
    /// 创建新的运行时实例
    pub fn new(storage: S) -> Self {
        Self {
            engine: Engine::default(),
            storage: Rc::new(RefCell::new(storage)),
            ownership_manager: None,
            scheduler: None,
        }
    }
    
    /// 获取存储接口
    pub fn storage(&self) -> Rc<RefCell<S>> {
        self.storage.clone()
    }
}

```

**使用示例**:

```rust
// 使用内存存储 (测试)
let runtime = Runtime::new(MemoryStorage::new());

// 使用 RocksDB (生产)
let db = RocksDBStorage::open("/path/to/db")?;
let runtime = Runtime::new(db);

```

### 3. **Host Functions - WASM 与存储交互**

```rust
// 文件: src/vm-runtime/src/host.rs

pub mod storage_api {
    /// storage_get(key_ptr: i32, key_len: i32) -> i64
    pub fn storage_get(
        mut caller: Caller<'_, HostState<impl Storage>>,
        key_ptr: i32,
        key_len: i32,
    ) -> Result<i64> {
        let key = read_memory(&memory, &caller, key_ptr, key_len)?;
        
        // 追踪读操作 (用于并行执行)
        caller.data_mut().read_write_set.add_read(key.clone());
        
        // 查询存储
        match caller.data().storage.borrow().get(&key)? {
            Some(value) => {
                caller.data_mut().last_get = Some(value);
                Ok(value.len() as i64)
            }
            None => Ok(0),
        }
    }
    
    /// storage_set(key_ptr: i32, key_len: i32, value_ptr: i32, value_len: i32) -> i32
    pub fn storage_set(
        mut caller: Caller<'_, HostState<impl Storage>>,
        key_ptr: i32,
        key_len: i32,
        value_ptr: i32,
        value_len: i32,
    ) -> Result<i32> {
        let key = read_memory(&memory, &caller, key_ptr, key_len)?;
        let value = read_memory(&memory, &caller, value_ptr, value_len)?;
        
        // 追踪写操作
        caller.data_mut().read_write_set.add_write(key.clone());
        
        // 写入存储
        caller.data_mut().storage.borrow_mut().set(&key, &value)?;
        Ok(0)
    }
}

```

**调用流程**:

```

WASM 智能合约
    ↓ storage_get/set (Host Function)
HostState<Storage>
    ↓ storage.get/set()
Storage Trait 实现
    ↓
底层数据库 (RocksDB/LevelDB/...)

```

---

## 💾 当前实现

### 1. **MemoryStorage - 内存存储**

**用途**: 测试和开发

```rust
// 文件: src/vm-runtime/src/storage.rs

#[derive(Default)]
pub struct MemoryStorage {
    data: std::collections::BTreeMap<Vec<u8>, Vec<u8>>,
}

impl Storage for MemoryStorage {
    fn get(&self, key: &[u8]) -> Result<Option<Vec<u8>>> {
        Ok(self.data.get(key).cloned())
    }

    fn set(&mut self, key: &[u8], value: &[u8]) -> Result<()> {
        self.data.insert(key.to_vec(), value.to_vec());
        Ok(())
    }

    fn delete(&mut self, key: &[u8]) -> Result<()> {
        self.data.remove(key);
        Ok(())
    }

    fn scan(&self, prefix: &[u8]) -> Result<Vec<(Vec<u8>, Vec<u8>)>> {
        Ok(self.data
            .range(prefix.to_vec()..)
            .take_while(|(k, _)| k.starts_with(prefix))
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect())
    }
}

```

**特点**:

- ✅ **零依赖**: 仅使用 Rust 标准库

- ✅ **快速**: 纯内存操作

- ⚠️ **非持久化**: 重启丢失数据

- ⚠️ **无法扩展**: 受限于内存大小

### 2. **MVCC Store - 多版本并发控制存储**

```rust
// 文件: src/vm-runtime/src/mvcc.rs

pub struct MvccStore {
    // DashMap: 每键粒度并发控制
    data: DashMap<Vec<u8>, RwLock<Vec<Version>>>,
    
    // 时间戳分配器
    ts: AtomicU64,
    
    // 垃圾回收配置
    gc_config: Arc<Mutex<GcConfig>>,
    auto_gc_handle: Arc<Mutex<Option<AutoGcHandle>>>,
}

pub struct Version {
    pub ts: u64,           // 时间戳
    pub value: Vec<u8>,    // 值
    pub txn: Txn,          // 事务状态
}

```

**特点**:

- ✅ **多版本**: 支持 MVCC 并发控制

- ✅ **高性能**: 键级锁定,187K TPS (低竞争)

- ✅ **自动 GC**: 后台清理旧版本

- ⚠️ **内存型**: 当前未持久化到磁盘

**与数据库的关系**:

```

MvccStore 是内存中的多版本缓存
    ↓ (未来可集成)
持久化层 (RocksDB/LevelDB)

```

---

## 🔧 数据库集成方案

### 方案 1: RocksDB 集成 (推荐) ⭐

**RocksDB** 是 Facebook 开发的高性能 KV 数据库,广泛用于区块链项目。

#### 实现示例

```rust
// 创建新文件: src/vm-runtime/src/storage/rocksdb_storage.rs

use crate::Storage;
use rocksdb::{DB, Options, WriteBatch};
use anyhow::Result;

pub struct RocksDBStorage {
    db: DB,
}

impl RocksDBStorage {
    pub fn open(path: &str) -> Result<Self> {
        let mut opts = Options::default();
        opts.create_if_missing(true);
        opts.set_max_open_files(10000);
        opts.set_use_fsync(false);
        opts.set_compression_type(rocksdb::DBCompressionType::Lz4);
        
        let db = DB::open(&opts, path)?;
        Ok(Self { db })
    }
    
    pub fn batch_write(&self, writes: &[(Vec<u8>, Vec<u8>)]) -> Result<()> {
        let mut batch = WriteBatch::default();
        for (key, value) in writes {
            batch.put(key, value);
        }
        self.db.write(batch)?;
        Ok(())
    }
}

impl Storage for RocksDBStorage {
    fn get(&self, key: &[u8]) -> Result<Option<Vec<u8>>> {
        Ok(self.db.get(key)?)
    }

    fn set(&mut self, key: &[u8], value: &[u8]) -> Result<()> {
        self.db.put(key, value)?;
        Ok(())
    }

    fn delete(&mut self, key: &[u8]) -> Result<()> {
        self.db.delete(key)?;
        Ok(())
    }

    fn scan(&self, prefix: &[u8]) -> Result<Vec<(Vec<u8>, Vec<u8>)>> {
        let mut results = Vec::new();
        let iter = self.db.prefix_iterator(prefix);
        for item in iter {
            let (key, value) = item?;
            if !key.starts_with(prefix) {
                break;
            }
            results.push((key.to_vec(), value.to_vec()));
        }
        Ok(results)
    }
}

```

#### 依赖配置

```toml

# Cargo.toml

[dependencies]
rocksdb = { version = "0.21", optional = true }

[features]
default = []
rocksdb-storage = ["rocksdb"]

```

#### 使用示例

```rust
use vm_runtime::{Runtime, RocksDBStorage};

fn main() -> anyhow::Result<()> {
    // 打开数据库
    let storage = RocksDBStorage::open("./supervm_data")?;
    
    // 创建运行时
    let runtime = Runtime::new(storage);
    
    // 执行合约
    let wasm_code = std::fs::read("contract.wasm")?;
    let result = runtime.execute(&wasm_code, "main", 1000, 1704067500)?;
    
    Ok(())
}

```

**优势**:

- ✅ **高性能**: 针对 SSD 优化

- ✅ **压缩**: 支持 LZ4/Snappy 压缩

- ✅ **成熟**: 以太坊 Geth、Solana 等都在使用

- ✅ **Rust 绑定**: `rust-rocksdb` crate 稳定

**性能指标**:

```

随机读: ~100K ops/s (SSD)
随机写: ~50K ops/s (SSD)
扫描: ~500MB/s
压缩比: 2-5x (取决于数据)

```

---

### 方案 2: LevelDB 集成

```rust
// 类似 RocksDB,但性能稍低
use leveldb::{database::Database, options::Options};

pub struct LevelDBStorage {
    db: Database<Vec<u8>>,
}

impl Storage for LevelDBStorage {
    // 实现类似...
}

```

**对比**:
| 特性 | RocksDB | LevelDB |
|------|---------|---------|
| 性能 | ⭐⭐⭐⭐⭐ | ⭐⭐⭐⭐ |
| 压缩 | 多种算法 | Snappy |
| 维护 | 活跃 | 较少 |
| 推荐度 | ✅ 高 | ⚠️ 中 |

---

### 方案 3: LMDB 集成 (内存映射)

```rust
use lmdb::{Environment, Database};

pub struct LMDBStorage {
    env: Environment,
    db: Database,
}

impl Storage for LMDBStorage {
    // 实现...
}

```

**特点**:

- ✅ **零拷贝**: 内存映射文件

- ✅ **ACID**: 完整事务支持

- ⚠️ **内存限制**: 需预分配地址空间

---

### 方案 4: PostgreSQL 集成 (关系型)

```rust
use sqlx::{PgPool, postgres::PgPoolOptions};

pub struct PostgresStorage {
    pool: PgPool,
}

impl Storage for PostgresStorage {
    fn get(&self, key: &[u8]) -> Result<Option<Vec<u8>>> {
        // SELECT value FROM kv_store WHERE key = $1
    }
    
    fn set(&mut self, key: &[u8], value: &[u8]) -> Result<()> {
        // INSERT INTO kv_store (key, value) VALUES ($1, $2)
        // ON CONFLICT (key) DO UPDATE SET value = $2
    }
}

```

**场景**:

- ✅ 需要 SQL 查询能力

- ✅ 需要复杂索引

- ⚠️ 性能低于 KV 数据库

- ⚠️ 部署复杂度高

---

## 📊 与传统数据库的对比

### SuperVM Storage vs 传统数据库

| 维度 | SuperVM Storage | 传统数据库 (PostgreSQL/MySQL) |
|------|-----------------|------------------------------|
| **数据模型** | Key-Value | 关系型 (表/行/列) |
| **查询能力** | get/set/scan | SQL (JOIN/GROUP BY/...) |
| **事务模型** | MVCC (乐观锁) | ACID (悲观锁可选) |
| **性能** | 187K TPS (单机) | ~5-20K TPS (单机) |
| **延迟** | 2-7 μs | 1-10 ms |
| **扩展性** | 水平扩展 (分片) | 垂直扩展为主 |
| **一致性** | 最终一致性 | 强一致性 |
| **用途** | 区块链状态存储 | 通用数据管理 |

### 为什么 SuperVM 不使用 SQL 数据库?

```

区块链状态存储的特点:
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
1. 简单 KV 访问      → 不需要 JOIN/聚合
2. 高频读写          → 需要极低延迟
3. 确定性执行        → 不需要复杂查询
4. 版本控制 (MVCC)   → KV 数据库更适合
5. 水平扩展          → 分片友好
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

SQL 数据库的开销:
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
❌ 查询解析          → 增加延迟
❌ 查询优化器        → 不确定性
❌ 复杂索引维护      → 写入慢
❌ 行锁/表锁         → 并发性差
❌ WAL 日志          → 额外 I/O
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

结论: KV 数据库 (RocksDB) 更适合区块链场景

```

---

## 🚀 未来规划

### Phase 6: 四层网络与存储分层

```

L1 (超算节点) - 完整状态存储
├── RocksDB (10TB+)
├── 完整历史数据
└── BFT 共识

L2 (矿机节点) - 轻量状态
├── RocksDB (1TB)
├── 最近 N 个区块
└── MVCC 批量执行

L3 (边缘节点) - 区域缓存
├── LRU Cache (100GB)
├── 热点数据
└── <10ms 延迟

L4 (移动节点) - 本地客户端
├── SQLite (1GB)
├── 即时反馈
└── 批量同步

```

### 存储优化路线图

**短期 (Q1 2026)**:

- [ ] 集成 RocksDB 持久化存储

- [ ] 实现批量写入优化

- [ ] 添加存储层监控指标

**中期 (Q2-Q3 2026)**:

- [ ] 实现 MVCC + RocksDB 集成

- [ ] 状态裁剪 (Pruning)

- [ ] 快照导出/导入

**长期 (2026+)**:

- [ ] 分布式存储集群

- [ ] 跨节点状态同步

- [ ] 存储层 sharding

---

## 📚 参考资料

### 数据库选型参考

**RocksDB**:

- 官网: https://rocksdb.org/

- Rust 绑定: https://github.com/rust-rocksdb/rust-rocksdb

- 使用者: Ethereum (Geth), Solana, CockroachDB

**LevelDB**:

- 官网: https://github.com/google/leveldb

- Rust 绑定: https://github.com/skade/leveldb

- 使用者: Bitcoin Core, Ethereum (早期)

**LMDB**:

- 官网: https://www.symas.com/lmdb

- Rust 绑定: https://github.com/danburkert/lmdb-rs

- 使用者: OpenLDAP, Monero

### 相关文档

- [SuperVM 存储接口设计](../API.md#storage-trait)

- [MVCC 并发控制](../parallel-execution.md)

- [四层网络架构](../phase1-implementation.md)

- [性能测试报告](../../BENCHMARK_RESULTS.md)

---

## 💡 总结

### 核心要点

1. **SuperVM ≠ 数据库**
   - SuperVM 是虚拟机执行引擎
   - 数据库是持久化存储后端
   - 通过 Storage Trait 解耦

2. **当前状态**
   - ✅ MemoryStorage (测试)
   - ✅ MVCC Store (内存多版本)
   - 📋 RocksDB (规划中)

3. **推荐方案**
   - 生产环境: RocksDB
   - 测试环境: MemoryStorage
   - 特殊场景: LMDB/PostgreSQL

4. **设计优势**
   - 抽象解耦 (Storage Trait)
   - 可插拔后端
   - 高性能 (187K TPS)
   - 类型安全 (Rust)

### 下一步行动

```bash

# 1. 添加 RocksDB 依赖

cargo add rocksdb --optional

# 2. 实现 RocksDBStorage

# 创建 src/vm-runtime/src/storage/rocksdb_storage.rs

# 3. 集成测试

cargo test --features rocksdb-storage

# 4. 性能基准测试

cargo bench --features rocksdb-storage

```

---

**文档版本**: v1.0  
**最后更新**: 2025-11-06  
**维护者**: KING XU (CHINA)
