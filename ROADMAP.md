# SuperVM - Development Roadmap

> **开发者**: king | **架构师**: KING XU (CHINA) | **最后更新**: 2025-11-06

> **ZK 隐私专项计划**: 详见 [ROADMAP-ZK-Privacy.md](./ROADMAP-ZK-Privacy.md)

---

## 📖 项目概述

SuperVM 是一个高性能的 WASM-first 区块链虚拟机，采用 Rust 实现，支持并行执行、MVCC 并发控制和可选的隐私保护特性。

### 🎯 核心目标
- ✅ **高性能执行**: MVCC 并发控制，187K+ TPS (低竞争), 85K+ TPS (高竞争)
- ✅ **并行调度**: 工作窃取算法 + 多版本存储引擎
- 🚧 **多语言支持**: Solidity (via Solang), AssemblyScript, Rust [设计完成,待实现]
- 📋 **EVM 兼容**: 支持现有以太坊合约迁移
- 🔒 **隐私保护**: 可选的 ZK 证明与隐私交易 (专项计划)
- 🌐 **四层网络**: L1超算 → L2矿机 → L3边缘 → L4移动 [设计完成,待实现]
- 🔄 **跨链编译器**: 一次开发多链部署 (WODA) [设计完成,待实现]

### 🏗️ 技术架构

```
┌──────────────────────────────────────────────────────────────┐
│                    SuperVM 完整技术栈                        │
├──────────────────────────────────────────────────────────────┤
│  应用层: DeFi │ Games │ Social                               │
│  (Phase 3: 跨链编译器支持 Solidity/Rust/Move)               │ ⚠️ 设计完成
├──────────────────────────────────────────────────────────────┤
│  执行层: 三通道路由 (Phase 5)                               │
│  ┌──────────────┬──────────────┬─────────────────┐          │
│  │ 快速通道     │ 共识通道     │ 隐私通道        │          │
│  │ (独占对象)   │ (共享对象)   │ (ZK证明)        │          │ 🚧 进行中 41%
│  └──────────────┴──────────────┴─────────────────┘          │
├──────────────────────────────────────────────────────────────┤
│  对象所有权模型 (Sui-Inspired) - Phase 5                    │ ✅ 已完成
├──────────────────────────────────────────────────────────────┤
│  并行调度层 (Phase 4)                                        │
│  MVCC 并行调度器 │ 工作窃取算法 │ 自动 GC                   │ ✅ 已完成
├──────────────────────────────────────────────────────────────┤
│  运行时层 (Phase 2)                                          │
│  WASM Runtime (wasmtime 17.0) │ Storage 抽象 │ Host Funcs   │ ✅ 已完成
├──────────────────────────────────────────────────────────────┤
│  网络层 (Phase 6: 四层神经网络)                             │
│  L1 超算 → L2 矿机 → L3 边缘 → L4 移动                       │ ⚠️ 设计完成
└──────────────────────────────────────────────────────────────┘

图例: ✅ 已实现  🚧 进行中  ⚠️ 设计完成待实现  📋 待规划
```

---

## 📊 开发进度总览

### 整体进度: 🎯 40% (3/10 阶段完成)

| 阶段 | 名称 | 状态 | 完成度 | 周期 |
|------|------|------|--------|------|
| Phase 1 | 项目基础设施 | ✅ 已完成 | 100% | 周0 |
| Phase 2 | WASM 运行时 PoC | ✅ 已完成 | 100% | 周1-3 |
| Phase 3 | 编译器适配 | 📋 规划中 | 0% | 周4-8 |
| Phase 4 | 并行执行引擎 | ✅ 已完成 | 100% | 周9-14 |
| **Phase 4.1** | **MVCC 高竞争优化** | 📋 规划中 | 0% | 4-6周 |
| **Phase 4.2** | **自适应性能调优 (AutoTuner)** | ✅ 已完成 | 100% | 2024-11-07 |
| **Phase 4.3** | **持久化存储集成** | 📋 规划中 | 0% | 3-4周 |
| **Phase 6** | **四层神经网络** | 📋 规划中 | 0% | 16周 (6.1-6.5) |
| Phase 7 | EVM 兼容层 | 📋 规划中 | 0% | 周15-22 |
| Phase 8 | 生产环境准备 | 📋 规划中 | 0% | 周23-36 |

---

## 🚀 Phase 1: 项目基础设施 (✅ 已完成)

**目标**: 搭建项目基础设施和开发流程

**时间**: 周0 | **完成度**: 100%

- ✅ Cargo workspace 配置
- ✅ 开发规范与贡献指南 (CONTRIBUTING.md)
- ✅ GitHub 问题模板 (bug_report, feature_request)
- ✅ GitHub PR 模板
- ✅ .editorconfig 编辑器配置
- ✅ 初始 ROADMAP.md

**交付物**: 
- ✅ 规范化的项目结构
- ✅ 完整的协作流程和模板
- ✅ 标准化的代码风格配置

---

## ⚡ Phase 2: WASM 运行时 PoC (✅ 已完成)

**目标**: 实现基础 WASM 执行能力和核心 Host Functions

**时间**: 周1-3 | **完成度**: 100%

### 核心功能

**vm-runtime crate (v0.1.0)**:
- ✅ wasmtime 17.0 集成
- ✅ Storage 抽象层
  - ✅ `Storage` trait 定义
  - ✅ `MemoryStorage` 实现 (BTreeMap 后端)
- ✅ Host Functions 架构
  - ✅ `storage_api` 模块: get, read_value, set, delete
  - ✅ `chain_api` 模块: block_number, timestamp, emit_event, events_len, read_event
- ✅ 公共 API:
  - ✅ `Runtime::new(storage)`
  - ✅ `Runtime::execute_add()` (demo)
  - ✅ `Runtime::execute_with_context()` (核心 API)
- ✅ 单元测试覆盖 (6/6 通过):
  - ✅ test_memory_storage
  - ✅ test_execute_add_via_wat
  - ✅ test_storage
  - ✅ test_host_functions
  - ✅ test_emit_event
  - ✅ test_execute_with_context

**node-core crate (v0.1.0)**:
- ✅ CLI 程序框架
- ✅ `--once` 标志支持 (自动化测试)
- ✅ 日志集成 (tracing + tracing_subscriber)
- ✅ Demo 程序:
  - ✅ Demo 1: 基础 add 函数演示
  - ✅ Demo 2: 完整事件系统演示

**文档**:
- ✅ README.md (完整使用指南)
- ✅ CHANGELOG.md (版本记录)
- ✅ API 参考表格

**测试验证**:
```bash
cargo test -p vm-runtime    # 6/6 通过
cargo run -p node-core --once  # 端到端验证成功
```

**交付物**: 
- ✅ 可运行的 WASM 虚拟机 (wasmtime 17.0)
- ✅ 完整的存储和事件系统
- ✅ 2 个 Demo 程序
- ✅ 详细的开发文档

---

## 🔧 Phase 3: 编译器适配 (📋 规划中)

**目标**: 支持主流智能合约语言编译到 WASM

**时间**: 周4-8 | **完成度**: 0%

⚠️ **重要说明**: 本阶段的完整跨链编译器设计已存在于 `docs/compiler-and-gas-innovation.md` (52KB, 1561行),包括 WODA (一次开发多链部署) 的完整架构,但代码实现尚未开始。

### 第一阶段: 基础编译器集成

**Solidity 支持 (via Solang)**:
- [ ] 集成 Solang 编译器
- [ ] Solidity 标准库适配
- [ ] Contract ABI 生成
- [ ] 部署脚本工具
- [ ] ERC20 示例合约

**AssemblyScript 支持**:
- [ ] AssemblyScript 编译配置
- [ ] 标准库绑定
- [ ] TypeScript 类型定义
- [ ] 示例合约模板

**开发工具**:
- [ ] compiler-adapter crate
- [ ] 自动化构建脚本
- [ ] WASM 优化流程 (wasm-opt)
- [ ] 元数据打包工具

**JS SDK**:
- [ ] npm 包结构
- [ ] 合约部署 API
- [ ] 合约调用封装
- [ ] Event 监听接口
- [ ] Hardhat 插件

### 第二阶段: 跨链编译器 (WODA - 设计已完成)

**核心功能** (详见 `docs/compiler-and-gas-innovation.md`):
- [ ] 统一中间表示 (SuperVM IR)
- [ ] Solidity/Rust/Move 前端解析器
- [ ] 多目标后端 (SuperVM/EVM/SVM/Move)
- [ ] CLI工具: `supervm-compiler`

**期望命令**:
```bash
# 编译到 SuperVM
supervm-compiler compile Token.sol --target supervm

# 跨链部署 (一次编译部署到所有链)
supervm-compiler compile Token.sol --target all

# 跨链转换
supervm-compiler transpile program.rs --source solana --target ethereum
```

**测试与文档**:
- [ ] Solidity 集成测试
- [ ] AssemblyScript 示例
- [ ] 跨链编译测试
- [ ] 开发者指南
- [ ] API 文档网站
- ✅ 完整设计文档 (已完成)

**交付物**:
- [ ] `compiler-adapter` crate (基础编译器集成)
- [ ] `supervm-ir` crate (统一中间表示)
- [ ] `supervm-compiler` CLI (跨链编译器)
- [ ] `js-sdk` (JavaScript/TypeScript 开发套件)
- [ ] 示例 dApp 项目
- [ ] 开发者指南和 API 文档

---

## 🚄 Phase 4: 并行执行引擎 (✅ 已完成)

**目标**: 实现高性能并行交易执行和 MVCC 并发控制

**时间**: 周9-14 | **完成度**: 100%

### 核心功能 ✅

**调度系统**:
- ✅ 交易依赖分析 (DependencyGraph)
- ✅ 账户访问模式提取 (ReadWriteSet)
- ✅ 并行执行调度器 (ParallelScheduler)
- ✅ 工作窃取算法 (WorkStealingScheduler - v0.3.0)

**冲突检测**:
- ✅ 读写集收集 (ReadWriteSet::add_read/add_write)
- ✅ 冲突检测算法 (ConflictDetector)
- ✅ MVCC 内置冲突检测（写写冲突自动处理）
- [ ] 重试机制优化
- [ ] 性能优化

**状态管理**:
- ✅ 快照与回滚 (StorageSnapshot + StateManager)
- ✅ 事务保护执行 (execute_with_snapshot)
- ✅ 嵌套快照支持
- ✅ MVCC 存储引擎 (MvccStore - v0.5.0)
- ✅ MVCC 调度器集成 (ParallelScheduler::new_with_mvcc)
- ✅ MVCC 只读快速路径优化
- ✅ MVCC 垃圾回收 (GC - v0.6.0)
- ✅ MVCC 自动垃圾回收 (Auto GC - v0.7.0)
- ✅ MvccScheduler 并行调度器 (v0.9.0)
- [ ] 批量提交优化
- [ ] 内存池管理

**性能测试**:
- [ ] 基准测试框架
- [ ] 吞吐量测试
- [ ] 延迟测试
- ✅ 并发正确性验证 (8 个单元测试通过)

**演示程序**:
- ✅ Demo 5: 并行冲突检测演示
- ✅ Demo 6: 状态快照与回滚演示
- ✅ Demo 7: 工作窃取调度器演示
- ✅ Demo 9: MVCC 多版本并发控制演示
- ✅ Demo 10: MVCC 自动垃圾回收演示
- ✅ Demo 11: MVCC 并行转账示例
- ✅ Demo 12: 热点计数器冲突测试

**测试覆盖**:
- ✅ 并行执行测试 (6/6 通过)
- ✅ 状态快照测试 (5/5 通过)
- ✅ 调度器集成测试 (3/3 通过)
- ✅ 工作窃取测试 (3/3 通过)
- ✅ MVCC 核心测试 (10/10 通过)
- ✅ MVCC 调度器集成测试 (3/3 通过)
- ✅ MVCC 垃圾回收测试 (4/4 通过)
- ✅ MVCC 自动 GC 测试 (2/2 通过)
- ✅ MvccScheduler 测试 (4/4 通过)
- ✅ MVCC 压力测试套件 (5/5 通过)
- ✅ 总计: 50+ 测试通过

**交付物**:
- ✅ parallel 模块 (vm-runtime::parallel)
- ✅ ReadWriteSet, ConflictDetector, DependencyGraph
- ✅ ParallelScheduler 集成 StateManager
- ✅ WorkStealingScheduler (工作窃取调度器)
- ✅ mvcc 模块 (vm-runtime::mvcc)
- ✅ MvccStore (多版本并发控制存储)
- ✅ MVCC GC + Auto GC (垃圾回收系统)
- ✅ parallel_mvcc 模块 (MvccScheduler)
- ✅ 并行执行设计文档 (docs/parallel-execution.md)
- ✅ MVCC 压力测试指南 (docs/stress-testing-guide.md)
- ✅ GC 可观测性文档 (docs/gc-observability.md)
- [ ] 性能测试报告

### 性能指标 📈
- ✅ **低竞争场景**: 187K TPS
- ✅ **高竞争场景**: 85K TPS (热点计数器测试)
- ✅ **测试覆盖**: 50+ 单元测试通过
- ✅ **压力测试**: 完整的 MVCC 压力测试套件

### 技术债与后续优化 🛠️
- [ ] ParallelScheduler 重试机制优化（MvccScheduler 已有内置重试）
- [ ] 热路径性能优化和缓存机制
- [ ] 批量提交优化
- [ ] 内存池管理
- [ ] 系统性吞吐量/延迟基准测试
- [ ] 性能测试报告生成
- [ ] Prometheus/Grafana 监控集成

---

## 🚀 Phase 4.1: MVCC 高竞争性能优化专项 (📋 规划中)

**目标**: 将高竞争场景下的 TPS 从 85K 提升到 120K+,与 Aptos Block-STM 持平或超越

**时间**: 预计 4-6 周 | **完成度**: 0% | **优先级**: 🔴 高

### 📊 当前性能基线
- ✅ **低竞争场景**: 187K TPS (全球领先)
- ⚠️ **高竞争场景**: 85K TPS (vs Aptos 120K,落后 29%)
- ✅ **读延迟**: 2.1 μs (全球最低)
- ✅ **写延迟**: 6.5 μs (全球最低)
- ✅ **GC 开销**: < 2% (业界最低)

### 🎯 性能目标
- 🎯 **高竞争 TPS**: 120K+ (提升 41%)
- 🎯 **线程利用率**: 98%+ (当前 95%)
- 🎯 **锁竞争开销**: < 5% (当前 ~15%)
- 🎯 **误报率**: < 1% (冲突检测)

### 🔧 优化方案

(详细内容见原 Phase 4.1 章节,包含 6 大优化方案、实施计划、测试验证等)

---

## 🧠 Phase 4.2: 自适应性能调优 (AutoTuner) (✅ 已完成)

**目标**: 让内核自动学习工作负载特征并动态调整配置参数以最大化 TPS

**时间**: 2024-11-07 | **完成度**: 100% | **优先级**: � 高

### 📊 实现成果

**核心功能**:
- ✅ 自动调整批量大小 (`min_batch_size`) - 基于历史 TPS
- ✅ 自动启用/禁用 Bloom Filter - 基于批量 + 冲突率 + 读写集大小
- ✅ 自动调整分片数 (`num_shards`) - 基于冲突率动态伸缩
- ✅ 自动调整密度回退阈值 - 避免 Bloom 过早回退

**性能提升**:
- 🎯 **零配置**: 默认启用,无需手动调参
- 🎯 **TPS 提升**: +10-20% vs 固定配置
- 🎯 **自适应**: 动态适应负载变化

**文档与示例**:
- ✅ `docs/AUTO-TUNER.md` - 完整使用指南
- ✅ `docs/bloom-filter-optimization-report.md` - Bloom Filter 优化分析
- ✅ `src/vm-runtime/src/auto_tuner.rs` - 核心实现
- ✅ `src/node-core/examples/auto_tuner_demo.rs` - AutoTuner 对比演示
- ✅ `src/node-core/examples/bloom_fair_bench.rs` - Bloom Filter 公平基准(支持 AUTO_BATCH)

**使用示例**:
```rust
// 创建调度器 (AutoTuner 默认启用)
let scheduler = OptimizedMvccScheduler::new();

// 查看学到的配置
if let Some(summary) = scheduler.get_auto_tuner_summary() {
    summary.print();
}
```

详见: `docs/AUTO-TUNER.md`

---

## �💾 Phase 4.3: 持久化存储集成专项 (📋 规划中)

**目标**: 集成 RocksDB 持久化存储,实现生产级状态管理

**时间**: 预计 3-4 周 | **完成度**: 0% | **优先级**: 🟡 中

### 📊 当前状态

**已实现**:
- ✅ Storage Trait 抽象层 (get/set/delete/scan)
- ✅ MemoryStorage (BTreeMap,仅用于测试)
- ✅ MVCC Store (内存多版本,187K TPS)
- ✅ Host Functions 集成 (storage_get/set/delete)

**问题**:
- ❌ 无持久化: 重启丢失所有状态
- ❌ 内存受限: 无法处理大规模状态
- ❌ 无快照: 无法回滚到历史状态
- ❌ 无归档: 无法查询历史数据

### 🎯 目标与验收标准

**功能目标**:
- ✅ RocksDB 后端集成 (替代 MemoryStorage)
- ✅ 持久化状态存储 (重启恢复)
- ✅ 批量写入优化 (WriteBatch)
- ✅ 快照管理 (Checkpoint)
- ✅ 状态裁剪 (Pruning)
- ✅ 监控指标 (读写 QPS,延迟,缓存命中率)

**性能目标**:
- 🎯 **随机读**: ≥ 100K ops/s (SSD)
- 🎯 **随机写**: ≥ 50K ops/s (SSD)
- 🎯 **批量写**: ≥ 200K ops/s (WriteBatch)
- 🎯 **扫描**: ≥ 500 MB/s
- 🎯 **压缩比**: 2-5x (LZ4)
- 🎯 **延迟 P99**: < 10 ms

**验收标准**:
- ✅ 所有单元测试通过 (RocksDBStorage impl Storage)
- ✅ 兼容性测试通过 (与 MemoryStorage 行为一致)
- ✅ 性能测试通过 (达到目标 QPS)
- ✅ 长时间稳定性测试 (24 小时无崩溃)
- ✅ 数据完整性测试 (重启恢复验证)
- ✅ 文档完整 (使用指南 + API 文档)

### 🔧 实现方案

#### 1. **RocksDB 集成** (Week 1)

**任务清单**:
- [ ] 添加依赖: `rocksdb = { version = "0.21", optional = true }`
- [ ] 创建 `src/vm-runtime/src/storage/rocksdb_storage.rs`
- [ ] 实现 `RocksDBStorage` 结构体
- [ ] 实现 `Storage` trait for `RocksDBStorage`
  - [ ] `get(&self, key: &[u8]) -> Result<Option<Vec<u8>>>`
  - [ ] `set(&mut self, key: &[u8], value: &[u8]) -> Result<()>`
  - [ ] `delete(&mut self, key: &[u8]) -> Result<()>`
  - [ ] `scan(&self, prefix: &[u8]) -> Result<Vec<(Vec<u8>, Vec<u8>)>>`
- [ ] 配置优化
  - [ ] `max_open_files = 10000`
  - [ ] `compression = LZ4`
  - [ ] `block_cache = 512MB`
  - [ ] `write_buffer_size = 128MB`

**代码框架**:
```rust
// src/vm-runtime/src/storage/rocksdb_storage.rs

use crate::Storage;
use rocksdb::{DB, Options, WriteBatch, IteratorMode};
use anyhow::Result;

pub struct RocksDBStorage {
    db: DB,
    path: String,
}

impl RocksDBStorage {
    pub fn open(path: &str) -> Result<Self> {
        let mut opts = Options::default();
        opts.create_if_missing(true);
        opts.set_max_open_files(10000);
        opts.set_compression_type(rocksdb::DBCompressionType::Lz4);
        opts.set_write_buffer_size(128 * 1024 * 1024); // 128MB
        
        let db = DB::open(&opts, path)?;
        Ok(Self { 
            db, 
            path: path.to_string() 
        })
    }
    
    pub fn batch_write(&self, writes: &[(Vec<u8>, Vec<u8>)]) -> Result<()> {
        let mut batch = WriteBatch::default();
        for (key, value) in writes {
            batch.put(key, value);
        }
        self.db.write(batch)?;
        Ok(())
    }
    
    pub fn create_checkpoint(&self, checkpoint_path: &str) -> Result<()> {
        let checkpoint = rocksdb::checkpoint::Checkpoint::new(&self.db)?;
        checkpoint.create_checkpoint(checkpoint_path)?;
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

#### 2. **MVCC + RocksDB 集成** (Week 2)

**挑战**: MVCC Store 是内存多版本,RocksDB 是持久化单版本

**方案 A: 两层架构** (推荐)
```
智能合约
    ↓
MVCC Store (内存缓存 + 版本控制)
    ↓ flush
RocksDB (持久化 + 单版本)
```

**实现**:
- [ ] MVCC Store 添加 `flush_to_storage(storage: &mut dyn Storage)` 方法
- [ ] 定期刷新 (每 N 个区块或每 M 秒)
- [ ] 仅刷新已提交版本 (Committed)
- [ ] 保留最近 K 个版本在内存 (热数据)

**方案 B: RocksDB 原生多版本** (高级)
```
使用 RocksDB Column Families 实现多版本:
- CF 0: 默认 (最新版本)
- CF 1: version_1
- CF 2: version_2
- ...
```

**选择**: 优先实现方案 A (简单),方案 B 作为后续优化

#### 3. **批量操作优化** (Week 2)

**目标**: 利用 WriteBatch 提升写入性能

**任务**:
- [ ] 在 Storage Trait 添加 `batch_write()` 方法
- [ ] MVCC Scheduler 集成批量提交
- [ ] 配置批量大小 (默认 1000)
- [ ] 性能测试: 批量 vs 单条

**预期提升**: 单条 50K ops/s → 批量 200K ops/s (4× 提升)

#### 4. **快照与裁剪** (Week 3)

**快照管理**:
- [ ] 实现 `create_checkpoint(path)` 方法
- [ ] 定期快照 (每 1000 区块)
- [ ] 快照恢复测试
- [ ] 快照导出/导入工具

**状态裁剪**:
- [ ] 配置保留窗口 (默认最近 10000 区块)
- [ ] 后台裁剪任务 (异步)
- [ ] 保留 Merkle Root (验证历史)
- [ ] 归档历史数据 (可选)

#### 5. **监控与调优** (Week 3-4)

**监控指标**:
- [ ] 读写 QPS (每秒操作数)
- [ ] 延迟分布 (P50/P90/P99/P999)
- [ ] 缓存命中率
- [ ] 压缩比
- [ ] 磁盘使用量
- [ ] 写入放大 (Write Amplification)

**工具集成**:
- [ ] Prometheus metrics exporter
- [ ] Grafana dashboard
- [ ] 性能分析工具 (perf/flamegraph)

**调优参数**:
```rust
// 针对不同场景的配置预设

// 高吞吐配置 (适合批量写入)
pub fn high_throughput_config() -> Options {
    let mut opts = Options::default();
    opts.set_max_background_jobs(8);
    opts.set_max_write_buffer_number(4);
    opts.set_write_buffer_size(256 * 1024 * 1024); // 256MB
    opts.set_target_file_size_base(128 * 1024 * 1024); // 128MB
    opts
}

// 低延迟配置 (适合读密集)
pub fn low_latency_config() -> Options {
    let mut opts = Options::default();
    opts.set_block_cache(512 * 1024 * 1024); // 512MB
    opts.set_bloom_filter(10.0, true);
    opts.set_compression_type(rocksdb::DBCompressionType::None); // 牺牲空间换延迟
    opts
}

// 均衡配置 (默认)
pub fn balanced_config() -> Options {
    // 当前实现
}
```

#### 6. **测试与文档** (Week 4)

**单元测试**:
- [ ] `test_rocksdb_basic_operations()`
- [ ] `test_rocksdb_batch_write()`
- [ ] `test_rocksdb_scan_prefix()`
- [ ] `test_rocksdb_checkpoint()`
- [ ] `test_rocksdb_recovery()`

**集成测试**:
- [ ] 与 MVCC Scheduler 集成测试
- [ ] 多线程并发测试
- [ ] 大数据量测试 (100GB+)
- [ ] 断电恢复测试

**性能基准测试**:
```bash
# 随机读写
cargo bench --bench storage_bench -- rocksdb_random

# 顺序扫描
cargo bench --bench storage_bench -- rocksdb_scan

# 批量写入
cargo bench --bench storage_bench -- rocksdb_batch
```

**文档**:
- [ ] 使用指南: `docs/storage-guide.md`
- [ ] API 文档: 更新 `docs/API.md`
- [ ] 配置指南: `docs/rocksdb-tuning.md`
- [ ] 迁移指南: `docs/migration-to-rocksdb.md`

### 📈 预期效果

**性能提升**:
```
内存存储 (MemoryStorage):
✅ 读: 无限制 (内存速度)
❌ 写: 无限制 (但重启丢失)
❌ 容量: 受限于内存 (~16GB)

RocksDB 存储:
✅ 读: 100K ops/s (SSD)
✅ 写: 50K ops/s (单条), 200K ops/s (批量)
✅ 容量: TB 级别
✅ 持久化: 重启恢复
✅ 快照: 支持回滚
```

**生产就绪**:
- ✅ 数据持久化 (灾难恢复)
- ✅ 大规模状态 (支持 TB 级数据)
- ✅ 历史查询 (快照机制)
- ✅ 性能监控 (Prometheus + Grafana)
- ✅ 运维工具 (备份/恢复/裁剪)

### 📋 实施计划

**Week 1: RocksDB 基础集成**
- [ ] 添加依赖和 Feature Flag
- [ ] 实现 RocksDBStorage
- [ ] 实现 Storage Trait
- [ ] 单元测试
- [ ] 基准测试

**Week 2: MVCC 集成 + 批量优化**
- [ ] MVCC Store 刷新机制
- [ ] 批量写入优化
- [ ] 集成测试
- [ ] 性能对比测试

**Week 3: 快照与裁剪**
- [ ] Checkpoint 实现
- [ ] 状态裁剪
- [ ] 恢复测试
- [ ] 长时间稳定性测试

**Week 4: 监控与文档**
- [ ] Prometheus 集成
- [ ] Grafana Dashboard
- [ ] 完整文档
- [ ] 使用示例

### 🎖️ 成功标准

完成后,SuperVM 将具备:
- ✅ **生产级持久化**: 数据安全,重启恢复
- ✅ **大规模状态**: 支持 TB 级区块链状态
- ✅ **高性能**: 100K+ 读 QPS, 200K+ 批量写 QPS
- ✅ **可运维**: 快照/备份/恢复/裁剪工具链
- ✅ **可监控**: 完整的 Metrics + Dashboard

**里程碑**: 从 PoC 原型 → 生产级虚拟机存储层 🏆

### 📚 参考资料
- [RocksDB 官方文档](https://rocksdb.org/)
- [rust-rocksdb GitHub](https://github.com/rust-rocksdb/rust-rocksdb)
- [SuperVM Storage 设计文档](../Q&A/SuperVM与数据库的关系)
- [以太坊 Geth 存储架构](https://geth.ethereum.org/docs/interface/database)
- [Solana AccountsDB](https://docs.solana.com/implemented-proposals/persistent-account-storage)

---

## � Phase 5: 对象所有权与三通道路由 (🚧 进行中)

### 📊 当前性能基线
- ✅ **低竞争场景**: 187K TPS (全球领先)
- ⚠️ **高竞争场景**: 85K TPS (vs Aptos 120K,落后 29%)
- ✅ **读延迟**: 2.1 μs (全球最低)
- ✅ **写延迟**: 6.5 μs (全球最低)
- ✅ **GC 开销**: < 2% (业界最低)

### 🎯 性能目标
- 🎯 **高竞争 TPS**: 120K+ (提升 41%)
- 🎯 **线程利用率**: 98%+ (当前 95%)
- 🎯 **锁竞争开销**: < 5% (当前 ~15%)
- 🎯 **误报率**: < 1% (冲突检测)

### 🔧 优化方案

#### 1. **细粒度锁优化** (预计提升 20-30%)
```rust
// 当前: DashMap<Vec<u8>, RwLock<Vec<Version>>>
// 问题: 热点键仍有锁竞争

// 优化方案 1: 增加分片数量
// - 当前: 默认分片 (CPU 核心数)
// - 目标: 4-8x 分片 (256-512 分片)

// 优化方案 2: Lock-free 读路径
// - 使用 Arc<RwLock> 替换 RwLock
// - 读操作无需等待写锁
// - 利用 MVCC 多版本特性
```

**实现任务**:
- [ ] 添加 `MvccStore::new_with_shards(shard_count: usize)` 配置
- [ ] 实现自适应分片策略 (根据热点键动态调整)
- [ ] 重构读路径为 lock-free (使用 Arc<Version> 引用计数)
- [ ] 添加分片性能基准测试

#### 2. **冲突检测优化** (预计提升 10-15%)
```rust
// 当前: 基于 HashSet 的读写集检测
// 问题: 误报率较高,导致不必要的重试

// 优化方案: Bloom Filter 快速过滤
// - 第一阶段: Bloom Filter 快速判断无冲突
// - 第二阶段: 精确检测真实冲突
// - 减少 70-80% 的精确检测开销
```

**实现任务**:
- [ ] 集成 Bloom Filter (bloomfilter crate 或自实现)
- [ ] 实现两阶段冲突检测
- [ ] 添加误报率统计和监控
- [ ] 优化哈希函数选择 (针对账户地址)

#### 3. **批量操作优化** (预计提升 5-10%)
```rust
// 当前: 单个交易提交
// 优化: 批量提交 + 组提交

// 方案: Group Commit
// - 收集 N 个交易的写操作
// - 一次性批量写入存储
// - 减少存储层 I/O 次数
```

**实现任务**:
- [ ] 实现 `MvccStore::batch_commit(txs: &[Transaction])`
- [ ] 添加批量大小配置 (默认 100-1000)
- [ ] 实现自适应批量策略 (根据负载调整)
- [ ] 添加批量提交性能测试

#### 4. **热路径优化** (预计提升 5-10%)
```rust
// 当前: 每次读取都检查版本链
// 优化: 缓存最新版本

// 方案 1: 版本缓存
// - 缓存最常访问的版本
// - LRU 淘汰策略

// 方案 2: 内联优化
// - 标记热路径函数为 #[inline]
// - 减少函数调用开销
```

**实现任务**:
- [ ] 添加版本缓存层 (LRU Cache)
- [ ] 标记关键路径函数 `#[inline]` 或 `#[inline(always)]`
- [ ] 使用 `likely`/`unlikely` hints
- [ ] Profile 热路径 (perf/flamegraph)

#### 5. **内存池管理** (预计提升 3-5%)
```rust
// 当前: 每个版本动态分配
// 优化: 预分配内存池

// 方案: Object Pool
// - 预分配 Version 对象池
// - 复用对象,减少分配/释放开销
// - 与 GC 集成
```

**实现任务**:
- [ ] 实现 `VersionPool` 对象池
- [ ] 集成到 MvccStore 创建/销毁流程
- [ ] 配置池大小和增长策略
- [ ] 监控内存使用和碎片

#### 6. **并行 GC 优化** (预计提升 2-5%)
```rust
// 当前: 后台单线程 GC
// 优化: 并行 GC + 增量 GC

// 方案 1: 并行标记和清理
// - 多线程扫描版本链
// - 并行删除过期版本

// 方案 2: 增量 GC
// - 每次只清理一部分
// - 减少 GC 停顿时间
```

**实现任务**:
- [ ] 实现并行 GC (Rayon 并行扫描)
- [ ] 实现增量 GC (分批清理)
- [ ] 添加 GC 暂停时间监控
- [ ] 优化 GC 触发策略

### 📈 预期性能提升

| 优化项 | 当前 TPS | 预期提升 | 目标 TPS |
|--------|----------|----------|----------|
| **基线** | 85K | - | 85K |
| + 细粒度锁 | 85K | +20-30% | 102-110K |
| + 冲突检测 | 102-110K | +10-15% | 112-127K |
| + 批量操作 | 112-127K | +5-10% | 118-140K |
| + 热路径优化 | 118-140K | +5-10% | 124-154K |
| + 内存池 | 124-154K | +3-5% | 128-162K |
| + 并行 GC | 128-162K | +2-5% | **130-170K** |

**保守目标**: 120K TPS (提升 41%)  
**激进目标**: 150K TPS (提升 76%)  
**极限目标**: 170K TPS (提升 100%)

### 🧪 测试与验证

**基准测试**:
- [ ] 热点计数器测试 (100% 冲突率)
- [ ] 转账测试 (10-50% 冲突率)
- [ ] 混合负载测试 (读写比 7:3)
- [ ] 长时间稳定性测试 (24 小时)

**性能分析**:
- [ ] Flamegraph 火焰图 (CPU profiling)
- [ ] perf stat 性能计数器
- [ ] 锁竞争分析 (parking_lot tracing)
- [ ] 内存分析 (valgrind/heaptrack)

**验收标准**:
- ✅ 高竞争 TPS ≥ 120K (必须)
- ✅ 线程利用率 ≥ 98%
- ✅ 锁竞争开销 < 5%
- ✅ 24 小时稳定性测试通过
- ✅ 内存泄漏检测通过
- ✅ 低竞争性能无退化 (≥ 187K TPS)

### 📋 实施计划

**Week 1-2: 细粒度锁 + 冲突检测**
- [ ] 实现可配置分片数量
- [ ] Lock-free 读路径重构
- [ ] Bloom Filter 集成
- [ ] 基准测试验证 (目标 110K+)

**Week 3-4: 批量操作 + 热路径**
- [ ] 实现批量提交
- [ ] 版本缓存层
- [ ] 内联优化和 likely hints
- [ ] Profile 和优化热路径
- [ ] 基准测试验证 (目标 120K+)

**Week 5-6: 内存池 + GC 优化 (可选)**
- [ ] 实现对象池
- [ ] 并行/增量 GC
- [ ] 长时间稳定性测试
- [ ] 性能报告和文档

### 🎖️ 成功标准

完成后,SuperVM 将在**所有场景**下成为全球性能第一:
- ✅ **低竞争**: 187K TPS (已是全球第一)
- ✅ **高竞争**: 120K+ TPS (与 Aptos 持平或超越)
- ✅ **读写延迟**: 2-7 μs (继续保持全球最低)
- ✅ **GC 开销**: < 2% (继续保持业界最低)

**定位**: 无可争议的全球第一区块链虚拟机内核 🏆

### 📚 参考资料
- [Aptos Block-STM 论文](https://arxiv.org/abs/2203.06871)
- [DashMap 性能优化](https://docs.rs/dashmap)
- [Lock-free 数据结构](https://preshing.com/20120612/an-introduction-to-lock-free-programming/)
- [Bloom Filter 原理](https://en.wikipedia.org/wiki/Bloom_filter)
- [Group Commit 技术](https://dev.mysql.com/doc/refman/8.0/en/group-commit.html)

---

## � Phase 5: 对象所有权与三通道路由 (🚧 进行中)

**目标**: 实现 Sui-Inspired 对象所有权模型和快速/共识/隐私三通道路由

**启动时间**: 2025-09-01 | **完成度**: 30%

### 已完成 ✅

**对象所有权模型** (Sui-Inspired):
- ✅ 实现文件: `src/vm-runtime/src/ownership.rs`
- ✅ 支持类型: 独占(Owned) / 共享(Shared) / 不可变(Immutable)
- ✅ 功能: 权限校验、所有权转移、冻结、路径路由
- ✅ Demo: `cargo run --example ownership_demo`
- ✅ 设计参考: `docs/sui-smart-contract-analysis.md`

**统一入口与路由**:
- ✅ 实现文件: `src/vm-runtime/src/supervm.rs`
- ✅ 路由类型: Privacy::{Public, Private}
- ✅ 核心 API: `execute_transaction()`
- ✅ Demo: `cargo run --example supervm_routing_demo`

### 进行中 🚧

**快速/共识路径打通**:
- [ ] 在调度器前置路由
- [ ] 集成 OwnershipManager
- [ ] 快速通道: 独占对象直接执行 (无需共识)
- [ ] 共识通道: 共享对象进入 MVCC 调度器

**私有路径细化**:
- [ ] 添加隐私验证占位
- [ ] 统计和可观测性
- [ ] 集成 ZK 隐私层 (详见 ROADMAP-ZK-Privacy.md)

### 下一步 📋
- [ ] 在并行执行器中集成 OwnershipManager 路由
- [ ] 增加 E2E 校验样例
- [ ] 完善三通道路由文档

**交付物**:
- [ ] 完整的三通道路由系统
- [ ] 性能测试报告
- [ ] 开发者使用文档

---

## 🌐 Phase 6: 四层神经网络 (📋 规划中)

**目标**: 实现 L1超算 → L2矿机 → L3边缘 → L4移动 的全球分布式网络架构

**时间**: 16 周 (Phase 6.1-6.5) | **完成度**: 0% (设计已完成)

### 📚 设计文档
- 📄 `docs/architecture-2.0.md` - 完整的四层网络设计 (983行)
- 📄 `docs/phase1-implementation.md` - Week 4 接口设计
- 📄 `docs/scenario-analysis-game-defi.md` - 游戏场景应用
- 📄 `docs/four-layer-network-deployment-and-compute-scheduling.md` - **硬件适配与部署策略** ✨ NEW

### 🎯 核心理念

**传统区块链**:
- ❌ 所有节点运行相同软件,执行相同任务
- ❌ 浪费资源 (高性能服务器做简单查询)
- ❌ 无法扩展 (受限于最弱节点)
- ❌ 成本高昂 (所有节点需高端硬件)

**SuperVM 四层网络**:
- ✅ 根据硬件能力,自动分配不同任务
- ✅ 资源优化 (充分利用每个节点的能力)
- ✅ 水平扩展 (弱节点处理简单任务)
- ✅ 成本降低 (不需要所有节点都是高配)
- ✅ 全网协同 (任务自动路由到合适节点)

**设计原则**:
1. **一核多态**: 同一 SuperVM 内核,根据硬件自动调整功能
2. **任务分层**: 复杂任务(共识/ZK)→强节点,简单任务(查询/转发)→弱节点
3. **存储分级**: 全量状态→L1,部分状态→L2,热数据→L3,本地缓存→L4
4. **算力池化**: 所有节点贡献算力,系统智能调度
5. **自动降级**: 硬件不足时自动降级功能(完整节点→轻节点)

### 🖥️ 四层硬件规格

#### L1: 超算节点 (Supercomputing Nodes)

**角色**: 共识参与者、完整状态存储、复杂计算

```yaml
推荐配置:
  CPU: 64-128 核心 (Intel Xeon Platinum / AMD EPYC 9654)
  RAM: 512 GB - 1 TB DDR5
  存储: 10 TB NVMe SSD (RAID 0)
  网络: 25-100 Gbps
  GPU: NVIDIA H100 (可选,用于 ZK 加速)

工作负载:
  - BFT 共识 (10-20K TPS)
  - 完整状态验证
  - ZK 证明生成 (可选 GPU)
  - 历史数据归档
  - 复杂查询 (聚合/分析)

预期性能:
  TPS: 10-20K (共识受限)
  存储: 10-100 TB 全量状态
  查询延迟: 10-50 ms
  区块时间: 1-3 秒
```

#### L2: 矿机节点 (Mining Nodes)

**角色**: 交易执行、区块打包、MVCC 并行调度

```yaml
推荐配置:
  CPU: 32-64 核心 (高主频)
  RAM: 128-256 GB
  存储: 2 TB NVMe SSD
  网络: 10 Gbps
  GPU: RTX 4090 (可选,用于密码学)

工作负载:
  - 交易执行 (MVCC)
  - 交易验证
  - 区块构建
  - 状态更新
  - 游戏状态更新/物理模拟 (游戏场景)

预期性能:
  TPS: 100-200K (MVCC 并行)
  存储: 500 GB - 2 TB (最近 10000 区块)
  查询延迟: 1-5 ms
  区块打包: < 100 ms
```

#### L3: 边缘节点 (Edge Nodes)

**角色**: 区域缓存、交易转发、快速响应

```yaml
推荐配置:
  CPU: 8-16 核心
  RAM: 16-32 GB
  存储: 256 GB SSD
  网络: 1 Gbps
  GPU: 无

工作负载:
  - 区域缓存 (LRU)
  - 交易路由/转发
  - 查询响应
  - 状态同步
  - CDN 功能 (资产缓存/内容分发)

预期性能:
  TPS: 1M+ (缓存命中)
  存储: 100 GB - 1 TB (热数据)
  查询延迟: < 10 ms
  缓存命中率: 80-95%
```

#### L4: 移动节点 (Mobile/IoT Nodes)

**角色**: 轻客户端、本地缓存、即时反馈

```yaml
移动设备配置:
  CPU: 4-8 核心 (ARM)
  RAM: 4-8 GB
  存储: 64-256 GB
  网络: 4G/5G/WiFi

工作负载:
  - 本地缓存
  - 交易签名/提交
  - 余额查询
  - 离线队列
  - 本地状态预测 (游戏客户端)

预期性能:
  TPS: 本地操作 (无限制)
  存储: 1-10 GB (用户数据)
  查询延迟: < 1 ms (本地)
  同步周期: 1-10 分钟
```

### 🔧 内核安装与适配

#### 统一内核,多重配置

**核心理念**: 同一个 SuperVM 内核二进制,根据硬件自动适配

```rust
// src/node-core/src/main.rs

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // 1. 检测硬件能力
    let hardware = HardwareDetector::detect()?;
    
    // 2. 自动决定节点类型
    let node_type = NodeType::auto_detect(&hardware)?;
    
    // 3. 加载对应配置
    let config = Config::load_for_node_type(node_type)?;
    
    // 4. 启动节点
    let node = SuperVMNode::new(hardware, config)?;
    node.start().await?;
    
    Ok(())
}
```

#### 硬件检测

```rust
// src/node-core/src/hardware_detector.rs

pub struct HardwareCapability {
    pub cpu_cores: usize,
    pub memory_gb: usize,
    pub disk_gb: usize,
    pub network_mbps: usize,
    pub has_gpu: bool,
    pub gpu_memory_gb: usize,
    pub arch: Architecture,  // x86_64, ARM64, ...
}

impl HardwareDetector {
    pub fn detect() -> Result<HardwareCapability> {
        let cpu_cores = num_cpus::get();
        let memory_gb = Self::detect_memory()?;
        let disk_gb = Self::detect_disk()?;
        let network_mbps = Self::detect_network()?;
        let (has_gpu, gpu_memory_gb) = Self::detect_gpu()?;
        let arch = Self::detect_arch();
        
        Ok(HardwareCapability {
            cpu_cores,
            memory_gb,
            disk_gb,
            network_mbps,
            has_gpu,
            gpu_memory_gb,
            arch,
        })
    }
}
```

#### 节点类型自动决策

```rust
// src/node-core/src/node_type.rs

#[derive(Debug, Clone, Copy)]
pub enum NodeType {
    L1Supernode,    // 超算节点
    L2Miner,        // 矿机节点
    L3Edge,         // 边缘节点
    L4Mobile,       // 移动节点
}

impl NodeType {
    pub fn auto_detect(hw: &HardwareCapability) -> Result<Self> {
        // 决策树算法
        if hw.cpu_cores >= 32 && hw.memory_gb >= 128 && hw.disk_gb >= 2000 {
            Ok(NodeType::L1Supernode)
        } else if hw.cpu_cores >= 16 && hw.memory_gb >= 64 && hw.disk_gb >= 500 {
            Ok(NodeType::L2Miner)
        } else if hw.cpu_cores >= 4 && hw.memory_gb >= 8 && hw.disk_gb >= 100 {
            Ok(NodeType::L3Edge)
        } else {
            Ok(NodeType::L4Mobile)
        }
    }
}
```

#### 配置文件模板

每层节点都有独立的配置文件模板:

```toml
# config/l1_supernode.toml
[node]
type = "L1Supernode"
[consensus]
enable = true
algorithm = "BFT"
[storage]
backend = "RocksDB"
enable_pruning = false  # 保留完整历史

# config/l2_miner.toml
[node]
type = "L2Miner"
[consensus]
enable = false  # L2 不参与共识
[storage]
backend = "RocksDB"
enable_pruning = true
prune_keep_blocks = 10000

# config/l3_edge.toml
[node]
type = "L3Edge"
[storage]
backend = "LRU"  # 仅内存缓存
cache_gb = 4

# config/l4_mobile.toml
[node]
type = "L4Mobile"
[storage]
backend = "SQLite"  # 轻量级数据库
cache_mb = 100
```

### 🎯 任务分工机制

#### 智能任务路由

```rust
// src/node-core/src/task_router.rs

pub struct TaskRouter {
    local_capability: HardwareCapability,
    node_type: NodeType,
    peers: Vec<PeerNode>,
}

impl TaskRouter {
    /// 决定任务应该在哪里执行
    pub async fn route_task(&self, task: Task) -> TaskDestination {
        match task {
            // 本地可处理的任务
            Task::SimpleQuery(_) if self.can_handle_locally(&task) => {
                TaskDestination::Local
            }
            
            // 需要转发到更强节点
            Task::ZkProof(_) if self.node_type != NodeType::L1Supernode => {
                let best_l1 = self.find_best_peer(NodeType::L1Supernode);
                TaskDestination::Remote(best_l1)
            }
            
            // 需要分布式执行
            Task::LargeComputation(_) => {
                let workers = self.find_available_workers();
                TaskDestination::Distributed(workers)
            }
            
            _ => TaskDestination::Local,
        }
    }
}
```

#### 任务类型定义

```rust
// src/node-core/src/task.rs

#[derive(Debug, Clone)]
pub enum Task {
    // L1 专属任务
    Consensus(ConsensusTask),           // 复杂度: 90
    ZkProof(ZkProofTask),               // 复杂度: 95
    StateValidation(StateValidationTask), // 复杂度: 85
    
    // L2 专属任务
    TxExecution(TxExecutionTask),       // 复杂度: 60
    BlockBuilding(BlockBuildingTask),   // 复杂度: 70
    StateUpdate(StateUpdateTask),       // 复杂度: 50
    
    // L3 专属任务
    Query(QueryTask),                   // 复杂度: 20
    TxForwarding(TxForwardingTask),     // 复杂度: 15
    CacheUpdate(CacheUpdateTask),       // 复杂度: 25
    
    // L4 专属任务
    LocalOp(LocalOpTask),               // 复杂度: 10
    TxSigning(TxSigningTask),           // 复杂度: 30
}
```

#### 负载均衡

```rust
// src/node-core/src/load_balancer.rs

pub struct LoadBalancer {
    nodes: DashMap<NodeId, NodeInfo>,
}

impl LoadBalancer {
    /// 选择最佳节点执行任务
    pub fn select_node(&self, task: &Task) -> Option<NodeId> {
        let required_type = task.required_node_type();
        
        // 1. 过滤符合条件的节点
        let candidates: Vec<_> = self.nodes
            .iter()
            .filter(|n| n.node_type >= required_type)
            .filter(|n| n.current_load.load(Ordering::Relaxed) < 80)
            .collect();
        
        // 2. 计算每个节点的得分
        let mut best_node = None;
        let mut best_score = f64::NEG_INFINITY;
        
        for node in candidates {
            let score = self.calculate_score(node, task);
            if score > best_score {
                best_score = score;
                best_node = Some(*node.key());
            }
        }
        
        best_node
    }
}
```

### 💾 存储分层管理

#### 四层存储策略

```
L1: 完整状态 (100%)
├── RocksDB (10-100 TB)
├── 所有历史区块
├── 所有历史交易
└── 所有状态变更

L2: 部分状态 (最近 N 个区块)
├── RocksDB (500 GB - 2 TB)
├── 最近 10000 区块
├── 活跃账户状态
└── 定期从 L1 裁剪

L3: 热点数据 (高频访问)
├── LRU Cache (100 GB - 1 TB)
├── 热门账户余额
├── NFT 元数据
└── 游戏实时状态

L4: 本地缓存 (用户专属)
├── SQLite (1-10 GB)
├── 用户账户
├── 最近交易
└── 离线队列
```

#### 状态同步协议

```rust
// src/node-core/src/state_sync.rs

pub struct StateSyncProtocol {
    local_node_type: NodeType,
    peers: HashMap<NodeType, Vec<PeerConnection>>,
}

impl StateSyncProtocol {
    /// L4 → L3 同步
    pub async fn sync_l4_to_l3(&self, user_data: UserData) -> Result<()> {
        let l3_peer = self.find_nearest_l3()?;
        
        // 1. 批量提交交易
        if user_data.pending_txs.len() > 0 {
            l3_peer.batch_submit(user_data.pending_txs).await?;
        }
        
        // 2. 获取最新状态
        let latest_state = l3_peer.query_user_state(user_data.address).await?;
        
        // 3. 更新本地缓存
        self.update_local_cache(latest_state)?;
        
        Ok(())
    }
    
    /// L3 → L2 同步
    pub async fn sync_l3_to_l2(&self, cache_miss: Vec<Key>) -> Result<()> {
        let l2_peer = self.find_best_l2()?;
        let data = l2_peer.batch_query(cache_miss).await?;
        self.update_cache(data)?;
        Ok(())
    }
    
    /// L2 → L1 同步
    pub async fn sync_l2_to_l1(&self, block: Block) -> Result<()> {
        let l1_peer = self.find_l1_validator()?;
        l1_peer.submit_block(block).await?;
        
        if self.should_prune() {
            self.prune_old_blocks().await?;
        }
        
        Ok(())
    }
}
```

#### 智能缓存策略

```rust
// src/node-core/src/cache.rs

pub struct SmartCache {
    lru: LruCache<Key, Value>,
    access_freq: DashMap<Key, AtomicU64>,
    prefetch_enabled: bool,
}

impl SmartCache {
    /// 预取热点数据
    pub async fn prefetch_hot_data(&self) -> Result<()> {
        if !self.prefetch_enabled {
            return Ok(());
        }
        
        // 1. 分析访问频率,预取 Top 1000
        let hot_keys: Vec<_> = self.access_freq
            .iter()
            .map(|entry| (entry.key().clone(), entry.value().load(Ordering::Relaxed)))
            .sorted_by(|a, b| b.1.cmp(&a.1))
            .take(1000)
            .map(|(k, _)| k)
            .collect();
        
        // 2. 批量从上层获取
        let data = self.batch_fetch_from_upper_layer(hot_keys).await?;
        
        // 3. 更新缓存
        for (key, value) in data {
            self.lru.put(key, value);
        }
        
        Ok(())
    }
}
```

### ⚡ 算力调度策略

#### 全网算力池

```rust
// src/node-core/src/compute_pool.rs

pub struct ComputePool {
    nodes: DashMap<NodeId, ComputeNode>,
    task_queue: Arc<Mutex<VecDeque<ComputeTask>>>,
}

impl ComputePool {
    /// 提交计算任务到全网算力池
    pub async fn submit_task(&self, task: ComputeTask) -> Result<TaskId> {
        let task_id = TaskId::new();
        
        // 1. 评估任务需求
        let requirement = task.compute_requirement();
        
        // 2. 查找合适的节点
        let suitable_nodes = self.find_suitable_nodes(&requirement)?;
        
        if suitable_nodes.is_empty() {
            self.task_queue.lock().await.push_back(task);
            return Ok(task_id);
        }
        
        // 3. 选择最佳节点并分配任务
        let best_node = self.select_best_node(&suitable_nodes);
        self.assign_task(best_node, task_id, task).await?;
        
        Ok(task_id)
    }
    
    /// 分布式并行计算 (MapReduce)
    pub async fn distributed_compute<T, R>(
        &self,
        data: Vec<T>,
        map_fn: fn(T) -> R,
        reduce_fn: fn(Vec<R>) -> R,
    ) -> Result<R> {
        // 1. 数据分片
        let chunk_size = (data.len() + self.nodes.len() - 1) / self.nodes.len();
        let chunks: Vec<_> = data.chunks(chunk_size).collect();
        
        // 2. 分发到各节点 (Map 阶段)
        let futures: Vec<_> = chunks.iter().enumerate()
            .map(|(i, chunk)| {
                let node = self.nodes.iter().nth(i % self.nodes.len()).unwrap();
                node.execute_map(chunk, map_fn)
            })
            .collect();
        
        // 3. 等待所有节点完成
        let results = futures::future::join_all(futures).await;
        
        // 4. Reduce 阶段
        Ok(reduce_fn(results))
    }
}
```

#### ZK 证明的 GPU 加速调度

```rust
// src/node-core/src/zk_scheduler.rs

pub struct ZkProofScheduler {
    gpu_nodes: Vec<NodeId>,  // 有 GPU 的 L1 节点
    cpu_fallback: Vec<NodeId>,
}

impl ZkProofScheduler {
    /// 调度 ZK 证明任务
    pub async fn schedule_proof(&self, proof_task: ZkProofTask) -> Result<Proof> {
        // 1. 优先尝试 GPU 节点
        if let Some(gpu_node) = self.find_available_gpu_node() {
            match self.submit_to_gpu(gpu_node, proof_task.clone()).await {
                Ok(proof) => return Ok(proof),
                Err(e) => warn!("GPU proof failed: {}, fallback to CPU", e),
            }
        }
        
        // 2. GPU 不可用,fallback 到 CPU
        let cpu_node = self.find_available_cpu_node()?;
        let proof = self.submit_to_cpu(cpu_node, proof_task).await?;
        Ok(proof)
    }
    
    /// 批量 ZK 证明 (充分利用 GPU)
    pub async fn batch_prove(&self, tasks: Vec<ZkProofTask>) -> Result<Vec<Proof>> {
        let gpu_nodes: Vec<_> = self.gpu_nodes
            .iter()
            .filter(|id| self.is_node_available(id))
            .collect();
        
        if gpu_nodes.is_empty() {
            return self.cpu_batch_prove(tasks).await;
        }
        
        // 任务分片并行提交
        let chunk_size = (tasks.len() + gpu_nodes.len() - 1) / gpu_nodes.len();
        let futures: Vec<_> = tasks.chunks(chunk_size).enumerate()
            .map(|(i, chunk)| {
                let node = gpu_nodes[i % gpu_nodes.len()];
                self.submit_batch_to_gpu(*node, chunk.to_vec())
            })
            .collect();
        
        let results = futures::future::try_join_all(futures).await?;
        Ok(results.into_iter().flatten().collect())
    }
}
```

#### 动态负载调整

```rust
// src/node-core/src/load_adjuster.rs

pub struct LoadAdjuster {
    metrics: Arc<Mutex<NodeMetrics>>,
}

impl LoadAdjuster {
    /// 根据负载动态调整节点行为
    pub async fn adjust(&self) -> Result<()> {
        let metrics = self.metrics.lock().await;
        
        if metrics.cpu_usage > 0.9 {
            self.reduce_parallelism().await?;
            self.reject_new_tasks().await?;
        }
        
        if metrics.memory_usage > 0.85 {
            self.clear_cache().await?;
            self.trigger_gc().await?;
        }
        
        if metrics.task_queue_length > 1000 {
            self.request_help_from_peers().await?;
        }
        
        Ok(())
    }
}
```

### 📅 实施路线图

#### Phase 6.1: 四层网络基础框架 (4 周)

**Week 1: 硬件检测与节点类型决策**
- [ ] 实现 `HardwareDetector` (CPU/内存/磁盘/GPU检测)
- [ ] 实现 `NodeType::auto_detect()` (决策树算法)
- [ ] 创建配置文件模板 (L1/L2/L3/L4)
- [ ] 实现命令行参数解析 (`--node-type`, `--config`)
- [ ] 单元测试 (覆盖各种硬件配置)

**Week 2: 任务路由与分发**
- [ ] 实现 `TaskRouter` (路由决策引擎)
- [ ] 定义 `Task` 枚举和属性 (复杂度/最低节点类型)
- [ ] 实现任务复杂度评估算法
- [ ] 实现任务路由决策树 (本地/远程/分布式)
- [ ] 集成测试 (模拟任务路由场景)

**Week 3: 负载均衡与调度**
- [ ] 实现 `LoadBalancer` (节点选择算法)
- [ ] 实现节点得分算法 (能力-负载-队列)
- [ ] 实现心跳和健康检查机制
- [ ] 实现动态负载调整 (CPU/内存/磁盘/网络监控)
- [ ] 压力测试 (1000+ 节点模拟)

**Week 4: 测试与文档**
- [ ] 单元测试 (覆盖率 > 80%)
- [ ] 集成测试 (4 层网络模拟)
- [ ] 性能基准测试 (任务路由延迟/负载均衡效率)
- [ ] 部署文档 (安装指南/配置说明)
- [ ] API 文档 (Rust doc)

#### Phase 6.2: 存储分层管理 (3 周)

**Week 1: L1/L2 存储实现**
- [ ] L1 RocksDB 完整状态存储 (与 Phase 4.2 集成)
- [ ] L2 RocksDB 裁剪策略 (保留最近 10000 区块)
- [ ] 状态同步协议 (L2→L1)
- [ ] 区块归档机制 (压缩/导出)
- [ ] 存储性能测试

**Week 2: L3/L4 缓存实现**
- [ ] L3 LRU 缓存实现 (基于 `lru` crate)
- [ ] L3 预取策略 (热点数据分析)
- [ ] L4 SQLite 轻量存储 (用户数据/离线队列)
- [ ] 状态同步协议 (L4→L3, L3→L2)
- [ ] 缓存命中率测试

**Week 3: 测试与优化**
- [ ] 存储性能测试 (各层读写 QPS)
- [ ] 缓存命中率测试 (L3 目标 80-95%)
- [ ] 数据一致性测试 (跨层同步验证)
- [ ] 同步延迟测试 (L4→L3→L2→L1 端到端)
- [ ] 优化报告与文档

#### Phase 6.3: 算力池与分布式计算 (4 周)

**Week 1: 计算池框架**
- [ ] 实现 `ComputePool` (全网算力管理)
- [ ] 实现 `ComputeNode` (节点能力描述)
- [ ] 任务队列管理 (优先级队列)
- [ ] 节点注册与发现 (动态上下线)
- [ ] 框架单元测试

**Week 2: 任务调度**
- [ ] 任务分配算法 (最佳节点选择)
- [ ] 分布式 MapReduce 实现
- [ ] 任务失败重试机制 (最多 3 次)
- [ ] 结果汇总与验证
- [ ] 调度性能测试

**Week 3: GPU 加速集成**
- [ ] ZK 证明 GPU 调度器 (`ZkProofScheduler`)
- [ ] GPU 节点管理 (能力检测/负载监控)
- [ ] CPU fallback 机制
- [ ] 批量证明优化 (充分利用 GPU 并行)
- [ ] GPU 加速效果验证 (对比 CPU)

**Week 4: 测试与优化**
- [ ] 算力池性能测试 (任务吞吐量/延迟)
- [ ] 分布式计算测试 (MapReduce 正确性/性能)
- [ ] GPU 加速效果验证 (10× 以上提升)
- [ ] 负载均衡测试 (节点利用率均衡性)
- [ ] 完整文档与示例

#### Phase 6.4: P2P 网络与通信 (3 周)

**Week 1: 神经网络寻址系统 (基础架构)** ⭐ **核心创新**
- [ ] 实现 `NodeAddress` 和地址系统
  - [ ] `NodeAddress` 结构体 (PeerId + 硬件能力 + NAT类型 + 区域)
  - [ ] `Region` 枚举和延迟估计
  - [ ] `NatType` 检测 (STUN 协议集成)
- [ ] 实现四层路由表 (类 DNS 分层寻址)
  - [ ] `L1RootRoutingTable` (RocksDB 持久化 + 完整索引)
  - [ ] `L2GlobalRoutingTable` (LRU 缓存 10万节点)
  - [ ] `L3RegionalRoutingTable` (区域缓存 1万节点)
  - [ ] `L4LocalRoutingTable` (本地缓存 100节点)
- [ ] 实现 `RoutingTable` trait (注册/查询/心跳/删除)
- [ ] 单元测试 (路由表基本操作)

**Week 2: 智能路由与快速穿透** ⭐ **核心创新**
- [ ] 实现 `AddressingService` 寻址协议
  - [ ] `AddressQuery` 查询请求 (支持过滤条件)
  - [ ] `AddressResponse` 响应 (返回节点 + 连接提示)
  - [ ] 智能节点选择算法 (延迟 + 负载 + 能力评分)
- [ ] 实现 NAT 穿透增强
  - [ ] `NatTraversalService` (NAT 类型检测)
  - [ ] ICE 协议打洞 (候选地址收集 + 连接性检查)
  - [ ] L3 中继服务 (自动选择最近 L3 作为 relay)
- [ ] 实现 `ConnectionHint` 生成
  - [ ] 直连提示 (公网 IP)
  - [ ] 打洞提示 (STUN 地址 + NAT 类型)
  - [ ] 中继提示 (L3 节点地址)
- [ ] 集成测试 (不同 NAT 场景穿透测试)

**Week 3: libp2p 集成与优化**
- [ ] libp2p 网络初始化 (transport + noise + yamux)
- [ ] 节点发现优化
  - [ ] mDNS (本地网络快速发现)
  - [ ] Kademlia DHT (全局发现 + 备份)
  - [ ] **神经网络寻址 (主要方式,取代传统 DHT)** ⭐
- [ ] 连接管理
  - [ ] 连接池 (复用连接)
  - [ ] 心跳机制 (10秒一次,更新负载)
  - [ ] 自动重连 (连接断开自动恢复)
- [ ] 消息协议
  - [ ] Protobuf 序列化 (寻址查询/响应)
  - [ ] 请求/响应模式 (RPC)
  - [ ] 发布/订阅模式 (心跳广播)
- [ ] 性能测试与优化
  - [ ] 寻址延迟测试 (目标: L3 < 10ms, L2 < 50ms, L1 < 100ms)
  - [ ] 缓存命中率测试 (目标: L3 80%+, L2 60%+)
  - [ ] NAT 穿透成功率测试 (目标: 95%+)
  - [ ] 跨区域连接测试 (全球节点模拟)
  - [ ] 网络分区恢复测试
  - [ ] 带宽优化 (压缩 + 批量传输)

#### Phase 6.5: 生产部署 (2 周)

**Week 1: 部署工具**
- [ ] 一键安装脚本 (`install.sh`)
- [ ] Docker 镜像 (L1/L2/L3/L4 多架构)
- [ ] Kubernetes 配置 (Helm Chart)
- [ ] 监控 Dashboard (Prometheus + Grafana)
- [ ] 自动化测试

**Week 2: 文档与培训**
- [ ] 部署指南 (快速开始/生产部署)
- [ ] 运维手册 (监控/升级/备份)
- [ ] 故障排查 (常见问题/日志分析)
- [ ] 用户培训材料 (视频/PPT)
- [ ] 社区发布

#### Phase 6.x: 合规与抗干扰（并行专项，2 周）

说明：本专项与 Phase 6.4-6.5 并行推进，聚焦“在合法合规前提下”的可用性、隐私最小化与抗干扰能力建设，不涉及规避或绕开监管的内容。

**目标**
- 合规模式开关与策略下发（区域/企业/全球）
- 数据主权与驻留（Region 优先，跨域需授权与审计）
- 在受限网络下的灰度降级：只读、延迟提交（store-and-forward）、局域协作
- 可插拔传输与流量整形（在允许策略内选择更稳健的传输）
- 可观测性与审计（敏感信息脱敏、可追溯但最小化元数据）

**里程碑（2 周）**
- Week 1：策略与配置
  - [ ] Policy Engine 配置模型与本地校验
  - [ ] 合规模式开关与地理围栏（Geo Fencing）
  - [ ] 数据驻留/保留期/元数据最小化策略
  - [ ] 受限网络降级策略（只读/队列/回放）
- Week 2：实现与验证
  - [ ] 可插拔传输抽象与白名单（tcp/tls/ws）
  - [ ] 速率整形与拥塞自适应
  - [ ] 审计日志与脱敏、SLA 指标暴露
  - [ ] 端到端场景测试（阻断→降级→恢复幂等对账）

**验收标准（Acceptance Criteria）**
- 受限网络场景：
  - 阻断时 L4 进入只读或离线队列模式；L3 区域只读缓存可用
  - 恢复后队列按幂等语义回放，重复写不产生副作用
  - 跨域写在合规策略禁止时被本地拒绝并记录可审计原因
- 数据主权：
  - 指定 Region 的数据不跨域落盘；跨域访问需策略授权且可审计
  - 数据保留期与删除策略可配置并自动执行
- 可观测性：
  - 暴露 Prometheus 指标：可用性、降级次数、队列深度、回放滞后
  - 审计日志默认脱敏，PII/密钥不落盘

**配置示例（TOML）**

```toml
[compliance]
mode = "regional"            # enterprise|regional|global
geo_fencing = ["CN", "!EU"]  # 允许/禁止的区域（示例）
metadata_minimization = "strict"  # strict|standard
retention_days = 7

[data_residency]
required_region = "CN-North"
cross_region_write = false

[network.policy]
fallback_order = ["lan", "regional", "global"]
allowed_transports = ["tcp", "tls", "websocket"]
rate_limit_bps = 1_048_576      # 1 MB/s
burst_bytes = 262_144           # 256 KB

[degrade]
read_only_on_unreachable = true
offline_queue = true
max_queue_age_min = 1440        # 24h
idempotent_keys = "sha256(tx)"  # 幂等键策略（文档化约定）

[observability]
audit_log = true
pii_redaction = "on"
```

**接口骨架（Rust，文档示例）**

```rust
// src/node-core/src/policy.rs（文档示例，后续落地实现）
#[derive(Clone)]
pub struct CompliancePolicy {
    pub mode: Mode,               // Enterprise/Regional/Global
    pub geo_fencing: Vec<String>, // 允许/禁止区域表达式
    pub residency_region: String, // 数据驻留区域
    pub metadata_min: Level,      // Strict/Standard
    pub retention_days: u32,
}

pub enum Decision { Allow, Deny { reason: String }, Degrade(DegradeMode) }

pub enum DegradeMode { Normal, ReadOnly, QueueOnly }

pub trait PolicyEngine {
    fn decide_write(&self, region: &str, key: &str) -> Decision;
    fn decide_transport(&self, t: &str) -> Decision; // tcp/tls/ws
}

// 受限网络下的传输可插拔抽象（白名单内选择稳健传输）
pub trait TransportAdapter {
    fn name(&self) -> &'static str;
    fn is_allowed(&self, policy: &dyn PolicyEngine) -> bool;
    fn send(&self, bytes: &[u8]) -> anyhow::Result<()>;
}

pub struct OfflineQueue {
    // 队列持久化、最大龄、幂等键
    pub max_age: std::time::Duration,
}

impl OfflineQueue {
    pub fn enqueue(&self, idempotent_key: &[u8], item: Vec<u8>) -> anyhow::Result<()> { Ok(()) }
    pub async fn replay(&self) -> anyhow::Result<()> { Ok(()) }
}
```

**测试计划（E2E 场景）**
- 场景 A：上游不可达 → L4 进入只读+队列；恢复后回放并对账
- 场景 B：跨域写被策略禁止 → 本地拒绝并记录审计原因
- 场景 C：允许的传输中断 → 切换到备选传输（tcp→tls→ws）
- 场景 D：速率整形 → 峰值流量被平滑且不触发上游丢包

更多实现细节与指南见《docs/restricted-network-availability.md》。

### 技术栈

**核心模块**:
- [ ] `src/node-core/src/hardware_detector.rs` - 硬件检测
- [ ] `src/node-core/src/node_type.rs` - 节点类型决策
- [ ] `src/node-core/src/task_router.rs` - 任务路由
- [ ] `src/node-core/src/load_balancer.rs` - 负载均衡
- [ ] `src/node-core/src/state_sync.rs` - 状态同步
- [ ] `src/node-core/src/cache.rs` - 智能缓存
- [ ] `src/node-core/src/compute_pool.rs` - 算力池
- [ ] `src/node-core/src/zk_scheduler.rs` - ZK 调度
- [ ] `src/node-core/src/load_adjuster.rs` - 负载调整

**神经网络寻址系统** ⭐ **新增**:
- [ ] `src/node-core/src/addressing.rs` - 节点地址系统
- [ ] `src/node-core/src/routing_table.rs` - 四层路由表
- [ ] `src/node-core/src/addressing_protocol.rs` - 寻址协议
- [ ] `src/node-core/src/nat_traversal.rs` - NAT 穿透增强
- [ ] `src/node-core/src/connection_hint.rs` - 连接提示生成

**网络层**:
- [ ] `src/node-core/src/network/l1_supernode.rs` - L1 节点
- [ ] `src/node-core/src/network/l2_miner.rs` - L2 节点
- [ ] `src/node-core/src/network/l3_edge.rs` - L3 节点
- [ ] `src/node-core/src/network/l4_mobile.rs` - L4 节点
- [ ] `src/node-core/src/network/protocol.rs` - 通信协议
- [ ] `src/node-core/src/network/router.rs` - 网络路由
- [ ] `config/l1_supernode.toml` - L1 配置模板
- [ ] `config/l2_miner.toml` - L2 配置模板
- [ ] `config/l3_edge.toml` - L3 配置模板
- [ ] `config/l4_mobile.toml` - L4 配置模板
- [ ] `scripts/install.sh` - 一键安装脚本
- [ ] `docker/Dockerfile.l1` - L1 Docker 镜像
- [ ] `docker/Dockerfile.l2` - L2 Docker 镜像
- [ ] `docker/Dockerfile.l3` - L3 Docker 镜像
- [ ] `docker/Dockerfile.l4` - L4 Docker 镜像
- [ ] `k8s/helm/supervm/` - Kubernetes Helm Chart

### 📊 预期效果

**性能提升**:
```
单机 SuperVM (Phase 4 完成后):
- TPS: 187K (低竞争) → 120K+ (高竞争优化后)
- 扩展性: 受限于单机硬件
- 成本: 高 (需高端服务器)

四层网络 SuperVM (Phase 6 完成后):
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
L1 (10 节点):      10-20K TPS × 10  = 100-200K TPS
L2 (100 节点):     100-200K TPS × 100 = 10-20M TPS
L3 (1000 节点):    查询响应 1M+ QPS
L4 (无限):         本地操作无限制
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
总吞吐量: 10-20M TPS (理论)
查询 QPS: 1M+
全球延迟: < 100 ms (跨洲)
           < 10 ms (同区域)
```

**成本优化**:
```
传统方案 (所有节点高配):
100 节点 × $5000/月 = $500K/月

四层网络方案:
L1 (10 节点):    $10K/月 × 10  = $100K/月
L2 (100 节点):   $2K/月 × 100  = $200K/月
L3 (1000 节点):  $100/月 × 1000 = $100K/月
L4 (用户设备):   $0
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
总成本: $400K/月 (节省 20%)
```

**算力利用率**:
```
传统方案:
- 平均算力利用率: 30-50%
- 峰值浪费: 50-70% 算力闲置

四层网络方案:
- 平均算力利用率: 70-90%
- 峰值调度: 动态借用全网算力
- 算力共享: 95%+ 利用率
```

### 交付物
- [ ] 四层网络完整实现 (16 周)
- [ ] 硬件检测与自动适配系统
- [ ] 任务路由与负载均衡框架
- [ ] 存储分层管理系统
- [ ] 全网算力池与分布式计算
- [ ] P2P 网络与通信协议
- [ ] 一键安装脚本 (Linux/Windows/macOS)
- [ ] Docker 镜像 (多架构支持)
- [ ] Kubernetes 部署配置
- [ ] 监控 Dashboard (Prometheus + Grafana)
- [ ] 部署文档 (安装/配置/运维)
- [ ] API 文档 (Rust doc + 用户手册)
- [ ] 性能测试报告 (各层性能指标)
- [ ] 故障排查指南

---

## 🔗 Phase 7: EVM 兼容层 (📋 规划中)

**目标**: 支持现有以太坊合约无缝迁移

**时间**: 周15-22 | **完成度**: 0%

⚠️ **重要说明**: 本阶段与 Phase 3 (Solidity 编译器) 采用**不同技术路线**:
- **Phase 3 路线**: Solidity → WASM (通过 Solang) - 原生 SuperVM 执行
- **Phase 5 路线**: 已编译的 EVM 字节码 → 直接执行 (通过 revm) - 以太坊合约迁移

🏗️ **架构隔离原则**: EVM 兼容层作为**可选插件**,**完全独立**于核心引擎:
- ✅ **零侵入**: 不修改 `vm-runtime` 核心代码
- ✅ **可插拔**: 独立 crate `evm-adapter`,可随时移除
- ✅ **性能隔离**: EVM 执行不影响 WASM 执行路径
- ✅ **清晰边界**: 通过统一的 `ExecutionEngine` trait 接口集成

### 架构设计: 插件化隔离

```
SuperVM 核心架构 (保持纯净)
├── vm-runtime (核心引擎)
│   ├── wasm_executor.rs        ← 核心 WASM 执行器 (不变)
│   ├── parallel_mvcc.rs        ← 并行调度器 (不变)
│   ├── storage.rs              ← 存储抽象 (不变)
│   └── execution_trait.rs      ← 新增: 统一执行接口
│
├── evm-adapter (独立插件 - 可选依赖)
│   ├── Cargo.toml              ← 独立的 crate
│   ├── evm_executor.rs         ← 实现 ExecutionEngine trait
│   ├── revm_backend.rs         ← revm 封装
│   └── precompiles.rs          ← EVM 预编译合约
│
└── node-core (节点层)
    └── engine_selector.rs      ← 根据合约类型选择引擎
```

**关键设计原则**:
1. **Feature Flag 控制**: `evm-adapter` 作为可选 feature
2. **Trait 抽象**: 定义 `ExecutionEngine` trait,WASM 和 EVM 都实现它
3. **运行时选择**: 根据合约类型动态选择引擎,互不干扰
4. **依赖隔离**: revm 仅在启用 `evm-compat` feature 时编译

### 技术路线选择

**方案A: EVM 解释器集成** (推荐 - 架构最清晰):
- [ ] 集成 revm (Rust EVM 实现)
- [ ] 作为独立 `evm-adapter` crate
- [ ] 通过 `ExecutionEngine` trait 接入
- [ ] 支持直接运行已有的 EVM 字节码
- [ ] 适合快速迁移现有以太坊 DApp
- [ ] **架构优势**: 完全隔离,零污染核心

**方案B: EVM→WASM 转译** (性能优先,但实验性):
- [ ] 研究 EVM Opcode → WASM 指令映射
- [ ] 实现转译器工具
- [ ] 转译后以 WASM 运行,性能更高
- [ ] 兼容性可能有限
- [ ] **架构优势**: 无需修改运行时

**方案C: 双模式支持** (终极方案):
- [ ] 同时支持方案A和方案B
- [ ] 开发者可选择最佳路径
- [ ] 实现复杂度高
- [ ] **架构要求**: 严格的模块隔离

### 实现计划: 保持核心纯净

**Step 1: 定义统一接口** (在 `vm-runtime`):
```rust
// vm-runtime/src/execution_trait.rs (新增文件)
pub trait ExecutionEngine {
    fn execute(&self, code: &[u8], context: &Context) -> Result<ExecutionResult>;
    fn engine_type(&self) -> EngineType; // WASM / EVM
}

// 现有 WASM 执行器实现该 trait (零修改,仅添加 trait impl)
impl ExecutionEngine for WasmExecutor { ... }
```

**Step 2: EVM 适配器独立开发** (新增 crate):
```rust
// evm-adapter/src/lib.rs (独立 crate)
pub struct EvmExecutor {
    revm: Revm,  // revm 依赖仅在此 crate 中
}

impl ExecutionEngine for EvmExecutor {
    fn execute(&self, bytecode: &[u8], ctx: &Context) -> Result<ExecutionResult> {
        // EVM 执行逻辑,完全隔离
    }
}
```

**Step 3: Feature Flag 控制** (在 `Cargo.toml`):
```toml
[features]
default = []
evm-compat = ["evm-adapter"]  # 可选功能

[dependencies]
evm-adapter = { path = "../evm-adapter", optional = true }
```

**Step 4: 运行时选择引擎** (在 `node-core`):
```rust
// node-core/src/engine_selector.rs
fn select_engine(contract: &Contract) -> Box<dyn ExecutionEngine> {
    match contract.code_type {
        CodeType::Wasm => Box::new(WasmExecutor::new()),
        #[cfg(feature = "evm-compat")]
        CodeType::Evm => Box::new(EvmExecutor::new()),
    }
}
```

### 计划功能

**阶段 1: 接口设计** (对核心零影响):
| **持久化存储集成** |  完整 |  0% | Phase 4.2 |  中 |
- [ ] 在 `vm-runtime` 中定义 `ExecutionEngine` trait
- [ ] 为现有 `WasmExecutor` 实现 trait (仅添加,不修改)
- [ ] 设计 `Context` 和 `ExecutionResult` 统一结构

**阶段 2: EVM 适配器开发** (完全独立):
- [ ] 创建独立 crate: `evm-adapter`
- [ ] 评估 revm/evmone 性能
- [ ] 实现 `ExecutionEngine` trait for EVM
- [ ] Ethereum JSON-RPC API 实现
- [ ] Gas 计量系统对接
- [ ] Precompiled 合约支持
- [ ] ERC 标准支持 (20/721/1155)

**阶段 3: 集成层开发** (node-core 层面):
- [ ] 实现引擎选择器 (根据合约类型路由)
- [ ] 添加 feature flag 控制
- [ ] 配置文件支持 (可禁用 EVM 模式)

**测试验证**:
- [ ] Ethereum 官方测试套件
- [ ] DeFi 协议兼容测试 (Uniswap, AAVE)
- [ ] NFT 市场测试
- [ ] 跨合约调用测试

**交付物**:
- [x] `execution_trait.rs` - 统一执行引擎接口 (在 vm-runtime) ✅ **已完成**
  - L1 扩展层，连接 L0 核心与 L2 适配器
  - 76 行代码，包含 `ExecutionEngine` trait
  - 测试通过: `test_execution_trait` ✅
- [ ] `evm-adapter` crate - 独立的 EVM 适配器
- [ ] `engine_selector.rs` - 引擎选择器 (在 node-core)
- [ ] 兼容性测试报告
- [ ] 合约迁移指南
- [ ] 性能对比报告 (EVM vs WASM路径)

### 核心纯净性保证 ✅

| 关注点 | 保证措施 |
|--------|---------|
| **代码侵入** | `vm-runtime` 核心代码零修改,仅添加 trait 定义 |
| **依赖污染** | revm 依赖仅在 `evm-adapter`,不进入核心 |
| **性能影响** | WASM 执行路径完全独立,无额外开销 |
| **编译体积** | feature flag 控制,不启用则不编译 EVM 代码 |
| **可维护性** | EVM 代码在独立 crate,可单独开发/测试/删除 |
| **升级隔离** | 核心升级不受 EVM 影响,EVM 升级不影响核心 |

**验证方式**:
```bash
# 纯净内核编译 (无 EVM)
cargo build -p vm-runtime --no-default-features

# 完整功能编译 (含 EVM)
cargo build --features evm-compat

# 性能基准测试 (验证零开销)
cargo bench --bench wasm_execution  # 应与之前结果一致
```

---

## 🔄 Phase 8: CPU-GPU 双内核异构计算架构 (📋 规划中)

**目标**: 实现 CPU+GPU 混合执行,大幅提升密码学计算性能

**时间**: 预计 17 周 (约 4 个月) | **完成度**: 0%

⚠️ **专项说明**: 本阶段为 GPU 加速专项,完整设计见 `docs/Q&A/双内核异构计算架构`

### 背景与动机

当前 SuperVM L0 内核基于 CPU 多线程架构,普通交易执行已达 187K TPS。但在密码学密集型场景存在瓶颈:

| 场景 | CPU 性能 | 瓶颈 | 目标 |
|------|---------|------|------|
| **ZK 证明生成** | 0.4 TPS (2.5s/proof) | 椭圆曲线运算 | **20-50 TPS** (100-1000× 加速) |
| **批量签名验证** | 2K TPS | 密码学计算 | **40-200K TPS** (20-100× 加速) |
| **批量哈希** | 10K TPS | 串行计算 | **100-300K TPS** (10-30× 加速) |
| **Merkle 树构建** | 5K TPS | 树结构遍历 | **25-100K TPS** (5-20× 加速) |

### 架构设计: 保持 L0 纯净

```
┌─────────────────────────────────────────────────────────┐
│         L4 应用层 - 混合调度器 (HybridScheduler)        │
│         - 智能任务分发 (CPU/GPU/混合)                   │
│         - 自动降级 (GPU 不可用时使用 CPU)                │
├─────────────────────────────────────────────────────────┤
│         L3 插件层 - 双内核实现                          │
│  ┌──────────────────┐     ┌──────────────────┐         │
│  │  CPU Executor    │     │  GPU Executor    │         │
│  │  (L0 WASM+MVCC)  │     │  (CUDA/OpenCL)   │         │
│  │  - 187K TPS      │     │  - ZK Proof      │         │
│  │  - 通用计算      │     │  - Batch Verify  │         │
│  └──────────────────┘     └──────────────────┘         │
├─────────────────────────────────────────────────────────┤
│         L1 统一接口 (execution_trait.rs 扩展)           │
│         - EngineType::Gpu 🆕                            │
│         - TaskType 任务分类 🆕                          │
│         - GPU 能力查询 API 🆕                           │
├─────────────────────────────────────────────────────────┤
│         L0 核心内核 (完全不修改!) ✅                     │
│         - WASM Runtime                                  │
│         - MVCC Store                                    │
│         - Parallel Scheduler                            │
└─────────────────────────────────────────────────────────┘
```

**核心原则**:
1. ✅ **L0 纯净**: CPU 内核完全不修改
2. ✅ **插件化**: GPU 作为 L3 可选插件,独立编译
3. ✅ **统一抽象**: 通过 L1 `execution_trait.rs` 统一接口
4. ✅ **自动降级**: 无 GPU 环境自动回退到 CPU

### 实施计划 (17周)

#### Phase 8.1: 基础框架 (2周)
- [ ] 扩展 L1 `execution_trait.rs` 接口
  - [ ] 添加 `EngineType::Gpu` 和 `TaskType` 枚举
  - [ ] 添加 `supports_task()` 和 `estimated_speedup()` 方法
- [ ] 创建 `gpu-executor` crate 骨架
  - [ ] 目录结构和 Cargo.toml 配置
  - [ ] Feature flags: `cuda` (NVIDIA) / `opencl` (AMD/Intel)
- [ ] GPU 设备检测与初始化
  - [ ] CUDA 设备枚举和能力检测
  - [ ] 错误处理与自动降级

**验收**: 编译通过,GPU 设备成功检测

#### Phase 8.2: GPU 密码学加速 (4周)
- [ ] GPU SHA256 批量计算
  - [ ] CUDA kernel 实现
  - [ ] 目标: 10-30× 加速
- [ ] GPU ECDSA/Ed25519 批量验证
  - [ ] 椭圆曲线点运算 GPU 实现
  - [ ] 目标: 20-100× 加速
- [ ] GPU Merkle 树构建
  - [ ] 并行哈希树算法
  - [ ] 目标: 5-20× 加速

**验收**: 所有功能达到目标加速比,集成测试通过

#### Phase 8.3: GPU ZK 证明加速 (6周)
- [ ] 集成 bellman-cuda 库
  - [ ] 依赖配置与编译
  - [ ] API 适配
- [ ] 实现 GPU Groth16 Prove
  - [ ] MSM (Multi-Scalar Multiplication) GPU 加速
  - [ ] FFT GPU 加速
  - [ ] 批量证明优化
- [ ] RingCT 电路 GPU 加速
  - [ ] 适配现有 RingCT 电路
  - [ ] 目标: 单个证明 < 50ms (vs CPU 2.5s)

**验收**: ZK 证明生成 100-1000× 加速,结果与 CPU 一致

#### Phase 8.4: 混合调度器 (3周)
- [ ] 实现 `HybridScheduler`
  - [ ] 任务分类逻辑 (Transaction/ZkProof/BatchVerify/...)
  - [ ] 自动调度策略 (Auto/CpuOnly/GpuOnly/LoadBalance)
  - [ ] CPU+GPU 协同执行 (隐私交易场景)
- [ ] 统计与监控
  - [ ] CPU/GPU 任务计数,执行时间统计
  - [ ] 加速比计算
- [ ] 批量混合执行
  - [ ] CPU 和 GPU 任务并行处理

**验收**: 混合工作负载 TPS > 200K,GPU 利用率 > 80%

#### Phase 8.5: 优化与测试 (2周)
- [ ] 性能优化
  - [ ] CPU-GPU 数据传输优化 (pinned memory)
  - [ ] GPU 内核优化 (occupancy, register usage)
  - [ ] 批处理大小调优
- [ ] 压力测试
  - [ ] 100K+ 交易混合负载测试
  - [ ] 24小时稳定性测试
  - [ ] 内存泄漏检测
- [ ] 文档完善
  - [ ] 架构文档,API 文档,使用示例

**验收**: 24小时无崩溃,性能达标,文档完整

### 性能预期

| 工作负载类型 | CPU-only | CPU+GPU | 提升 |
|-------------|----------|---------|------|
| **80% 普通 + 20% 隐私** | 150K TPS | 154K TPS | +3% |
| **50% 普通 + 50% 隐私** | 94K TPS | 103K TPS | +10% |
| **30% 普通 + 70% 隐私** | 56K TPS | 70K TPS | +25% |

**结论**: 隐私交易占比越高,GPU 加速收益越明显。

### 验收标准

**功能验收**:
- [ ] ✅ GPU 设备检测成功率 > 99%
- [ ] ✅ GPU 密码学计算正确性 100%
- [ ] ✅ GPU ZK 证明与 CPU 结果一致
- [ ] ✅ 混合调度器任务分发正确率 > 95%
- [ ] ✅ CPU-only 模式编译通过 (无 GPU 依赖)

**性能验收**:
- [ ] ✅ GPU ZK 证明加速 > 50×
- [ ] ✅ GPU 批量签名验证加速 > 20×
- [ ] ✅ GPU 批量哈希加速 > 10×
- [ ] ✅ 混合工作负载 TPS > 200K
- [ ] ✅ GPU 利用率 > 70% (高负载场景)

**稳定性验收**:
- [ ] ✅ 24小时压力测试无崩溃
- [ ] ✅ 内存使用增长 < 1% / 小时
- [ ] ✅ GPU 失败自动降级到 CPU

**代码质量验收**:
- [ ] ✅ L0 内核代码零修改
- [ ] ✅ L1 接口扩展通过代码审查
- [ ] ✅ GPU 执行器单元测试覆盖率 > 80%
- [ ] ✅ 文档完整性 > 90%

### 技术栈

**新建 crate**:
- `gpu-executor/` - GPU 执行器插件
  - `src/executor.rs` - 执行器主逻辑
  - `src/cuda/` - NVIDIA CUDA 后端
  - `src/opencl/` - OpenCL 后端 (AMD/Intel)

**依赖**:
```toml
cudarc = { version = "0.10", optional = true }       # CUDA 绑定
bellman-cuda = { version = "0.4", optional = true }  # ZK GPU 加速
sha2-cuda = { version = "0.1", optional = true }     # SHA256 GPU
opencl3 = { version = "0.9", optional = true }       # OpenCL
```

**Feature flags**:
```toml
[features]
default = []
gpu-cuda = ["cudarc", "bellman-cuda", "sha2-cuda"]  # NVIDIA
gpu-opencl = ["opencl3"]                             # AMD/Intel
gpu-all = ["gpu-cuda", "gpu-opencl"]                 # 全部支持
```

**修改文件**:
- `src/vm-runtime/src/execution_trait.rs` (L1 扩展)
- `node-core/src/hybrid_scheduler.rs` (新建)

### 风险评估

| 风险 | 概率 | 影响 | 缓解措施 |
|------|------|------|---------|
| **CUDA 库兼容性** | 🟡 中 | 高 | 提前验证 POC,准备 OpenCL 方案 |
| **CPU-GPU 数据传输瓶颈** | 🟡 中 | 中 | 使用 pinned memory,批处理优化 |
| **GPU 内存不足** | 🟢 低 | 中 | 动态调整 batch size,支持多 GPU |
| **性能未达预期** | 🟢 低 | 高 | 充分基准测试,算法优化 |
| **L0 内核污染** | 🟢 低 | 高 | 严格 feature flag 隔离,代码审查 |

### 后续扩展 (可选)

- [ ] **Phase 8.6**: 多 GPU 支持
  - 多 GPU 设备管理
  - 跨 GPU 任务调度
  - GPU 间负载均衡

- [ ] **Phase 8.7**: 其他 GPU 加速场景 (研究性质)
  - GPU 智能合约执行
  - GPU MVCC 读优化
  - GPU 网络数据包处理

### 交付物

- [ ] `gpu-executor` crate - GPU 执行器插件
- [ ] `HybridScheduler` - CPU+GPU 混合调度器
- [ ] 性能测试报告 (加速比,吞吐量,延迟)
- [ ] 架构文档和使用指南
- [ ] GPU 环境配置文档 (CUDA/OpenCL 安装)
- [ ] 示例代码 (隐私交易 GPU 加速)

**参考文档**:
- `docs/Q&A/双内核异构计算架构` - 完整架构设计 (本文档)
- `docs/Q&A/关于内核对GPU和CPU的适配` - GPU 适配 Q&A

---

## 🏭 Phase 9: 生产环境准备 (📋 规划中)

**目标**: 完善功能，达到生产可用标准

**时间**: 周40-53 | **完成度**: 0%

### 计划功能

**网络层**:
- [ ] P2P 网络实现
- [ ] 区块同步
- [ ] 交易广播
- [ ] 节点发现

**共识系统**:
- [ ] 共识插件接口
- [ ] PoW/PoS 示例实现
- [ ] 区块验证
- [ ] 最终性确认

**监控与运维**:
- [ ] Prometheus 指标
- [ ] Grafana 仪表盘
- [ ] 日志聚合
- [ ] 告警系统

**安全审计**:
- [ ] 代码审计
- [ ] 模糊测试
- [ ] 安全加固
- [ ] 漏洞赏金计划

**文档完善**:
- [ ] 架构设计文档
- [ ] 运维手册
- [ ] API 完整文档
- [ ] 最佳实践指南

**交付物**:
- [ ] 生产级区块链节点
- [ ] 完整监控系统
- [ ] 安全审计报告
- [ ] 全面运维文档

---

## 🔧 技术栈

### 核心技术栈
| 组件 | 技术选型 | 版本 | 状态 |
|------|---------|------|------|
| **Runtime** | wasmtime (JIT) | 17.0 | ✅ |
| **并发模型** | MVCC + 工作窃取 | v0.9.0 | ✅ |
| **存储引擎** | Storage Trait 抽象 | - | ✅ |
| **持久化存储** | RocksDB (可选) | 0.21 | 📋 Phase 4.2 |
| **异步运行时** | tokio | 1.35 | ✅ |
| **语言支持** | Rust, Solidity, AssemblyScript | - | 🚧 |
| **GPU 加速** | CUDA/OpenCL (可选) | - | 📋 Phase 8 |
| **网络层** | libp2p | - | 📋 |
| **共识** | 插件化设计 | - | 📋 |
| **Gas 机制** | 多币种 Gas + PGAT | [设计完成](./docs/gas-incentive-mechanism.md) | 📋 |
| **监控** | Prometheus + Grafana | - | 📋 |
| **日志** | tracing | 0.1 | ✅ |
| **CLI** | clap | 4.4 | ✅ |

### 依赖要求
- **Rust**: stable 1.70+
- **操作系统**: Linux, macOS, Windows
- **内存**: 建议 8GB+

---

## 📊 开发状态对照表

| 功能模块 | 设计文档 | 实现状态 | 阶段 | 优先级 |
|---------|---------|---------|------|--------|
| 项目基础设施 | ✅ | ✅ 100% | Phase 1 | - |
| WASM运行时 | ✅ | ✅ 100% | Phase 2 | - |
| **跨链编译器(WODA)** | ✅ 1561行 | ❌ 0% | Phase 3 | 🟡 中 |
| MVCC并行执行 | ✅ | ✅ 100% | Phase 4 | - |
| **MVCC高竞争优化** |  完整 |  0% | Phase 4.1 |  高 |
| **三通道路由** | ✅ | 🚧 41% | Phase 5 | � 低 |
| **四层网络(L1-L4)** | ✅ 983行 | ❌ 0% | Phase 6 | � 高 |
| EVM兼容层 | ✅ | ❌ 0% | Phase 7 | � 中 |
| **CPU-GPU双内核** | ✅ 完整 | ❌ 0% | Phase 8 | 🟡 中 |
| 生产环境准备 | 📋 | ❌ 0% | Phase 9 | � 低 |
| ZK隐私层 | ✅ | 🚧 进行中 | 专项 | 🟢 低 |

### 说明
- **Phase 6 (四层网络)**: 完整设计见 `docs/architecture-2.0.md`，但网络层代码完全未实现
- **Phase 3 (跨链编译器)**: 完整设计见 `docs/compiler-and-gas-innovation.md`，需先实现基础编译器集成
- **Phase 5 (三通道路由)**: 对象所有权已完成，路由集成进行中

---

## 📊 风险评估与缓解

### 技术风险

**设计与实现差距 (新增)**
- 风险: 四层网络和跨链编译器设计完整但未实现,可能误导用户
- 缓解:
  - ✅ ROADMAP中明确标注 "设计完成,待实现"
  - 建议: 创建 GitHub Issues 跟踪实现进度
  - 建议: 在README中添加功能状态说明

**Solidity->WASM 语义差异**
- 风险: Solidity 特性可能无法完美映射到 WASM
- 缓解: 
  - 明确标注不支持的特性
  - 提供兼容层说明文档
  - 与 Solang 社区保持沟通

**并行执行复杂性**
- 风险: 并发 bug 难以调试和复现
- 缓解:
  - 从简单的快照-回滚模型开始
  - 完善的测试套件
  - 形式化验证工具

**性能开销**
- 风险: WASM 执行和 Host 调用可能成为瓶颈
- 缓解:
  - 选择高性能 runtime (wasmtime JIT)
  - 热路径优化
  - 缓存机制
  - 定期性能基准测试

### 生态风险

**开发者采用**
- 风险: 开发者习惯现有工具链
- 缓解:
  - 提供熟悉的开发体验 (Hardhat 插件)
  - 完善文档和示例
  - 社区支持和教程

**EVM 兼容性**
- 风险: EVM 兼容可能污染核心架构
- 缓解:
  - ✅ 采用插件化架构,完全隔离 (详见 `docs/evm-adapter-design.md`)
  - ✅ Feature flag 控制,可选编译
  - ✅ 通过 trait 抽象,零侵入核心代码
  - ✅ 独立 crate 开发和测试

---

## 📈 当前状态总结

**更新时间**: 2025-11-05

### ✅ 已完成的里程碑
1. **Phase 1**: 项目基础设施 (100%)
   - Cargo workspace、CI/CD、代码规范

2. **Phase 2**: WASM 运行时 PoC (100%)
   - wasmtime 集成、Storage 抽象层、Host Functions

3. **Phase 4**: 并行执行引擎 (100%)
   - ParallelScheduler + WorkStealingScheduler
   - MVCC 存储引擎 (v0.5.0 → v0.9.0)
   - 自动垃圾回收 (GC + Auto GC)
   - MvccScheduler 并行调度器
   - 50+ 测试通过，完整文档

4. **Phase 7**: 三通道路由 (30%)
   - 对象所有权模型 (Sui-Inspired)
   - 统一路由入口
   - 快速/共识/隐私三通道

### 🚧 进行中的工作
- **Phase 7**: 三通道路由集成 (30%)
  - 对象所有权模型已完成
  - 快速/共识/隐私路径打通进行中
- **ZK隐私层**: 环签名、RingCT、零知识证明

### 📋 待启动的阶段
- **Phase 3**: 编译器适配 (设计完成52KB，需实现)
- **Phase 5**: EVM 兼容层
- **Phase 6**: 生产环境准备
- **Phase 8**: 四层神经网络 (设计完成983行，需实现)

### 📊 总体进度
```
Phase 1 基础设施        ████████████████████ 100% ✅
Phase 2 WASM运行时      ████████████████████ 100% ✅
Phase 3 编译器适配      ░░░░░░░░░░░░░░░░░░░░   0% 📋
Phase 4 并行执行        ████████████████████ 100% ✅
Phase 4.1 高竞争优化    ░░░░░░░░░░░░░░░░░░░░   0% 📋 (新增)
Phase 4.2 持久化存储    ░░░░░░░░░░░░░░░░░░░░   0% 📋 (新增)
Phase 5 三通道路由      ████████░░░░░░░░░░░░  41% 🚧
Phase 6 四层网络        ░░░░░░░░░░░░░░░░░░░░   0% 📋
Phase 7 EVM兼容         ░░░░░░░░░░░░░░░░░░░░   0% 📋
Phase 8 CPU-GPU双内核   ░░░░░░░░░░░░░░░░░░░░   0% 📋
Phase 9 生产环境        ░░░░░░░░░░░░░░░░░░░░   0% 📋
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
Overall 整体进度        ██████░░░░░░░░░░░░░░  34% 
```

---

## 🎯 下一步行动计划

### 本周 (Week 1)
- [x] ✅ 重构 ROADMAP 结构，移除"2.0架构升级"概念
- [x] ✅ 新增 Phase 8: CPU-GPU 双内核异构计算架构专项
- [x] ✅ 新增 Phase 4.1: MVCC 高竞争性能优化专项
- [x] ✅ 新增 Phase 4.2: 持久化存储集成专项 (RocksDB)
- [ ] **Phase 5**: 完成三通道路由集成
- [ ] 在并行执行器中集成 OwnershipManager
- [ ] 增加 E2E 校验样例

### 本月 (November 2025)
- [ ] **Phase 4.2**: 启动 RocksDB 持久化存储集成 (优先)
  - [ ] 实现 RocksDBStorage
  - [ ] 集成测试与性能基准
- [ ] **Phase 4.1**: 启动 MVCC 高竞争性能优化 (目标 120K+ TPS)
  - [ ] 细粒度锁优化
  - [ ] Bloom Filter 冲突检测
- [ ] **Phase 5**: 完成快速/共识/隐私三通道打通
- [ ] **Phase 3**: 调研 Solang 编译器集成方案
- [ ] **Phase 3**: 设计 compiler-adapter 架构
- [ ] **Phase 6**: 评估四层网络实现优先级
- [ ] **Phase 8**: 评估 GPU 加速专项优先级和硬件需求
- [ ] 开始 JS SDK 原型开发
- [ ] 集成 ZK 隐私层基础设施

### 下个季度 (Q1 2026)

#### 🔥 优先级 1: 核心性能与存储 (7-10周)
- [ ] **Phase 4.2**: 完成 RocksDB 持久化存储集成 (3-4周)
  - Week 1: RocksDB 基础集成 + Storage Trait 实现
  - Week 2: MVCC 集成 + 批量写入优化
  - Week 3: 快照管理 + 状态裁剪
  - Week 4: 监控指标 + 完整文档
- [ ] **Phase 4.1**: 完成 MVCC 高竞争性能优化 (4-6周)
  - Week 1-2: 细粒度锁 + Bloom Filter (目标 110K TPS)
  - Week 3-4: 批量操作 + 热路径优化 (目标 120K+ TPS)
  - Week 5-6: 内存池 + 并行 GC (目标 130-150K TPS)
  - 性能报告和文档

#### 🌐 优先级 2: 四层网络基础 (16周,可与优先级1并行)
- [ ] **Phase 6.1**: 四层网络基础框架 (4周)
  - Week 1: 硬件检测 + 节点类型决策
  - Week 2: 任务路由 + 分发机制
  - Week 3: 负载均衡 + 调度算法
  - Week 4: 测试 + 文档
- [ ] **Phase 6.2**: 存储分层管理 (3周)
  - Week 1: L1/L2 RocksDB 实现
  - Week 2: L3/L4 缓存实现
  - Week 3: 测试 + 优化
- [ ] **Phase 6.3**: 算力池与分布式计算 (4周)
  - Week 1: 计算池框架
  - Week 2: 任务调度
  - Week 3: GPU 加速集成
  - Week 4: 测试 + 优化
- [ ] **Phase 6.4**: P2P 网络与通信 (3周)
  - Week 1: libp2p 集成
  - Week 2: 协议实现
  - Week 3: 测试 + 优化
- [ ] **Phase 6.5**: 生产部署 (2周)
  - Week 1: 部署工具 (Docker/K8s)
  - Week 2: 文档 + 培训

#### 🔧 优先级 3: 编译器与跨链 (5周,Phase 6 后启动)
- [ ] **Phase 3**: 完成 Solidity 编译器集成 (3周)
- [ ] **Phase 3**: 启动跨链编译器 (WODA) 实现 (2周原型)
  - 实现 SuperVM IR 中间表示
  - 实现基础前端解析器

#### ⚡ 可选: GPU 加速专项 (如有硬件支持)
- [ ] **Phase 8**: GPU 加速专项
  - Phase 8.1: 基础框架 (2周)
  - Phase 8.2: GPU 密码学加速 (4周)

#### 📦 生态建设
- [ ] 实现完整 JS SDK
- [ ] 编写 ERC20/ERC721 示例合约
- [ ] 开始 EVM 兼容层开发 (Phase 7)

---

## 🤝 参与贡献

欢迎贡献! 请参阅:
- [CONTRIBUTING.md](CONTRIBUTING.md) - 贡献指南
- [DEVELOPER.md](DEVELOPER.md) - 开发者文档
- [GitHub Issues](https://github.com/XujueKing/SuperVM/issues) - 问题反馈

## 📬 联系方式

- **开发者**: king
- **架构师**: KING XU (CHINA)
- **GitHub**: [@XujueKing](https://github.com/XujueKing)
- **项目**: [SuperVM](https://github.com/XujueKing/SuperVM)

---

## 📚 相关文档

### 核心文档
- [README.md](./README.md) - 项目简介和快速开始
- [CHANGELOG.md](./CHANGELOG.md) - 版本更新日志
- [CONTRIBUTING.md](./CONTRIBUTING.md) - 贡献指南
- [DEVELOPER.md](./DEVELOPER.md) - 开发者文档
- [ROADMAP-ZK-Privacy.md](./ROADMAP-ZK-Privacy.md) - ZK 隐私专项计划

### 架构与设计
- [docs/architecture-2.0.md](./docs/architecture-2.0.md) - SuperVM 2.0 完整架构 (四层网络)
- [docs/parallel-execution.md](./docs/parallel-execution.md) - 并行执行引擎设计
- [docs/compiler-and-gas-innovation.md](./docs/compiler-and-gas-innovation.md) - 跨链编译器 & 多币种 Gas
- [docs/evm-adapter-design.md](./docs/evm-adapter-design.md) - EVM 适配器插件化设计
- [docs/KERNEL-DEFINITION.md](./docs/KERNEL-DEFINITION.md) - **内核定义与保护机制** ⚠️ 重要
- [docs/KERNEL-MODULES-VERSIONS.md](./docs/KERNEL-MODULES-VERSIONS.md) - **模块分级与版本索引** 🧭
- [docs/sui-smart-contract-analysis.md](./docs/sui-smart-contract-analysis.md) - Sui 智能合约分析
- [docs/scenario-analysis-game-defi.md](./docs/scenario-analysis-game-defi.md) - 游戏与 DeFi 场景分析
- [docs/Q&A/双内核异构计算架构](./docs/Q&A/双内核异构计算架构) - **CPU-GPU 双内核设计** 🆕 重要
- [docs/Q&A/superVM内核部分的技术水平](./docs/Q&A/superVM内核部分的技术水平) - **内核技术评估** 🆕 参考
- [docs/Q&A/SuperVM与数据库的关系](./docs/Q&A/SuperVM与数据库的关系) - **存储架构设计** 🆕 重要
- [docs/four-layer-network-deployment-and-compute-scheduling.md](./docs/four-layer-network-deployment-and-compute-scheduling.md) - **四层网络部署策略** ✨ 最新 重要

### 经济模型与激励
- [docs/gas-incentive-mechanism.md](./docs/gas-incentive-mechanism.md) - Gas 激励机制设计

### 测试与运维
- [docs/stress-testing-guide.md](./docs/stress-testing-guide.md) - 压力测试指南
- [docs/gc-observability.md](./docs/gc-observability.md) - GC 可观测性指南

---

<div align="center">

**SuperVM - 高性能 WASM 区块链虚拟机**

*Roadmap 会根据开发进度和社区反馈持续更新*

[![License](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)
[![Rust](https://img.shields.io/badge/rust-1.70%2B-orange.svg)](https://www.rust-lang.org)
[![Status](https://img.shields.io/badge/status-active-success.svg)](https://github.com/XujueKing/SuperVM)

</div>



