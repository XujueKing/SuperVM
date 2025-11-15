# 跨分片事务架构设计

**版本**: v1.0  
**作者**: KING XU  
**日期**: 2025-11-08  
**状态**: 设计中 (Phase 6)

---

## 1. 架构概览

### 1.1 目标

将 SuperVM 从单节点 MVCC 扩展到**水平可扩展的分片架构**：

- 支持跨分片原子事务（2PC 协议）

- 保持 MVCC 并发控制语义

- 最小化跨分片通信开销

- 提供线性扩展能力（N 分片 → N 倍吞吐）

### 1.2 核心组件

```

┌─────────────────────────────────────────────────────────┐
│                  Transaction Coordinator                 │
│  (ShardCoordinator - 事务协调器，运行 2PC 协议)           │
└────────────┬───────────────────────────┬─────────────────┘
             │                           │
   ┌─────────▼──────────┐      ┌────────▼─────────┐
   │   Shard 0          │      │   Shard 1        │
   │ ┌────────────────┐ │      │ ┌──────────────┐ │
   │ │ MvccScheduler  │ │      │ │MvccScheduler │ │
   │ │ OwnershipMgr   │ │      │ │OwnershipMgr  │ │
   │ │ RocksDB Store  │ │      │ │RocksDB Store │ │
   │ └────────────────┘ │      │ └──────────────┘ │
   └────────────────────┘      └──────────────────┘
            │                           │
   ┌────────▼────────────────────────────▼─────────┐
   │         Network Layer (gRPC/tarpc)            │
   │  prepare_txn() / commit_txn() / abort_txn()  │
   └───────────────────────────────────────────────┘

```

---

## 2. 分片策略

### 2.1 对象分片规则

使用**一致性哈希**将对象映射到分片：

```rust
fn shard_for_object(object_id: &ObjectId, num_shards: usize) -> ShardId {
    let hash = xxhash::xxh64(object_id, 0);
    (hash % num_shards as u64) as ShardId
}

```

**特性**:

- 负载均衡：均匀分布对象

- 确定性：相同对象始终路由到同一分片

- 可扩展：增加分片时最小化数据迁移

### 2.2 分片类型

| 分片角色 | 职责 | 数据 |
|---------|-----|-----|
| **Home Shard** | 对象的主分片 | 存储对象完整数据 + 元数据 |
| **Participant Shard** | 参与跨分片事务的分片 | 临时锁定对象，不存储 |
| **Coordinator Shard** | 事务发起者 | 运行 2PC，协调所有参与者 |

---

## 3. 两阶段提交协议 (2PC)

### 3.1 协议流程

```

Client → Coordinator: Submit Txn(read_set, write_set)
          │
          ├──> Phase 1: PREPARE
          │    ├─> Shard 0: prepare_txn(local_reads, local_writes)
          │    │    └─> Check conflicts, lock objects → Vote YES/NO
          │    └─> Shard 1: prepare_txn(...)
          │         └─> Vote YES/NO
          │
          ├──> Collect Votes
          │    └─> All YES → Decision: COMMIT
          │         Any NO  → Decision: ABORT
          │
          └──> Phase 2: COMMIT/ABORT
               ├─> Shard 0: commit_txn() / abort_txn()
               │    └─> Apply writes & release locks OR rollback
               └─> Shard 1: commit_txn() / abort_txn()

```

### 3.2 消息定义

```rust
// Phase 1: 准备阶段
struct PrepareRequest {
    txn_id: TxnId,
    shard_id: ShardId,
    read_set: Vec<(ObjectId, Version)>,
    write_set: Vec<(ObjectId, Vec<u8>)>, // (id, new_data)
    timestamp: u64,
}

enum PrepareResponse {
    VoteYes { txn_id: TxnId },
    VoteNo { txn_id: TxnId, reason: ConflictReason },
}

// Phase 2: 提交阶段
struct CommitRequest {
    txn_id: TxnId,
    decision: Decision, // COMMIT or ABORT
}

struct CommitResponse {
    txn_id: TxnId,
    status: CommitStatus, // Success or Failed
}

```

### 3.3 状态机

```

[INIT] → prepare_all_shards() → [PREPARED]
         ↓ (any NO)              ↓ (all YES)
     [ABORTED]              [COMMITTING] → commit_all() → [COMMITTED]
                                ↓ (timeout/failure)
                            [ABORTED]

```

---

## 4. 冲突检测扩展

### 4.1 跨分片读写集验证

在 `MvccScheduler` 中增加远程验证接口：

```rust
impl MvccScheduler {
    // 本地验证（现有逻辑）
    fn validate_local(&self, txn: &Transaction) -> bool;
    
    // 跨分片验证（新增）
    async fn validate_remote(
        &self, 
        txn: &Transaction,
        remote_reads: HashMap<ShardId, Vec<ObjectId>>
    ) -> Result<bool, RemoteError> {
        for (shard_id, object_ids) in remote_reads {
            let versions = self.rpc_client
                .get_object_versions(shard_id, object_ids)
                .await?;
            
            // 检查是否发生跨分片写入冲突
            if !self.check_version_consistency(versions) {
                return Ok(false);
            }
        }
        Ok(true)
    }
}

```

### 4.2 死锁检测

使用**等待图 (Wait-For Graph)** 检测跨分片死锁：

```rust
struct DeadlockDetector {
    wait_graph: HashMap<TxnId, HashSet<TxnId>>,
}

impl DeadlockDetector {
    fn add_wait_edge(&mut self, waiter: TxnId, holder: TxnId) {
        self.wait_graph.entry(waiter).or_default().insert(holder);
    }
    
    fn detect_cycle(&self) -> Option<Vec<TxnId>> {
        // Tarjan's algorithm 或 DFS
    }
    
    fn break_deadlock(&mut self, cycle: Vec<TxnId>) -> TxnId {
        // 选择代价最小的事务中止（如：最晚开始的事务）
        cycle.into_iter().max_by_key(|txn_id| txn_id.timestamp).unwrap()
    }
}

```

---

## 5. 网络通信层

### 5.1 技术选型

| 方案 | 优点 | 缺点 | 推荐度 |
|-----|------|-----|-------|
| **tonic (gRPC)** | 工业标准、跨语言、流式支持 | 依赖多、编译慢 | ⭐⭐⭐⭐⭐ |
| **tarpc** | 纯 Rust、轻量、类型安全 | 生态较小 | ⭐⭐⭐⭐ |
| **自定义 TCP** | 灵活、低开销 | 维护成本高 | ⭐⭐ |

**选择**: **tonic (gRPC)** - 便于未来集成监控、负载均衡等生产特性。

### 5.2 gRPC 服务定义

```protobuf
// shard_service.proto
service ShardService {
    rpc PrepareTxn(PrepareRequest) returns (PrepareResponse);
    rpc CommitTxn(CommitRequest) returns (CommitResponse);
    rpc AbortTxn(AbortRequest) returns (AbortResponse);
    rpc GetObjectVersions(VersionRequest) returns (VersionResponse);
}

```

### 5.3 Rust 实现框架

```rust
use tonic::{transport::Server, Request, Response, Status};

#[tonic::async_trait]
impl ShardService for ShardNode {
    async fn prepare_txn(
        &self,
        request: Request<PrepareRequest>,
    ) -> Result<Response<PrepareResponse>, Status> {
        let req = request.into_inner();
        
        // 1. 检查本地冲突
        let conflicts = self.mvcc_scheduler.check_conflicts(&req.read_set, &req.write_set);
        
        if conflicts.is_empty() {
            // 2. 锁定对象（预写日志）
            self.mvcc_scheduler.lock_objects(&req.write_set);
            Ok(Response::new(PrepareResponse::VoteYes { txn_id: req.txn_id }))
        } else {
            Ok(Response::new(PrepareResponse::VoteNo { 
                txn_id: req.txn_id, 
                reason: conflicts[0].clone() 
            }))
        }
    }
    
    async fn commit_txn(
        &self,
        request: Request<CommitRequest>,
    ) -> Result<Response<CommitResponse>, Status> {
        let req = request.into_inner();
        
        match req.decision {
            Decision::Commit => {
                // 应用写入并释放锁
                self.mvcc_scheduler.commit(&req.txn_id)?;
            },
            Decision::Abort => {
                // 回滚并释放锁
                self.mvcc_scheduler.abort(&req.txn_id)?;
            }
        }
        
        Ok(Response::new(CommitResponse { 
            txn_id: req.txn_id, 
            status: CommitStatus::Success 
        }))
    }
}

```

---

## 6. 性能优化

### 6.1 批量提交

合并多个事务的 prepare 请求，减少网络往返：

```rust
struct BatchPrepareRequest {
    txns: Vec<PrepareRequest>,
}

// 单次 RPC 处理 100 个事务
const BATCH_SIZE: usize = 100;

```

### 6.2 读取优化

- **Read-Only 事务**: 跳过 2PC，直接读取快照

- **本地读优化**: 优先读取本地分片对象

- **版本缓存**: 缓存远程对象版本信息

### 6.3 延迟优化

| 场景 | 单分片延迟 | 跨分片延迟 | 优化方法 |
|-----|-----------|-----------|---------|
| 纯本地事务 | 0.5ms | N/A | 快速路径 |
| 2-分片事务 | - | 3-5ms | 并行 prepare |
| 3+ 分片事务 | - | 5-10ms | 批量 + 流水线 |

---

## 7. 监控指标

### 7.1 新增 Prometheus 指标

```rust
// 跨分片事务统计
cross_shard_txn_total{shards="2"}        // 涉及 2 个分片的事务数
cross_shard_txn_total{shards="3+"}       // 涉及 3+ 分片

// 2PC 阶段延迟
cross_shard_prepare_latency_ms          // prepare 阶段耗时
cross_shard_commit_latency_ms           // commit 阶段耗时

// 成功率与中止率
cross_shard_commit_rate                 // 提交成功率
cross_shard_abort_rate                  // 中止率（冲突 + 超时）

// 网络通信
shard_rpc_bytes_sent                    // 发送字节数
shard_rpc_bytes_received                // 接收字节数
shard_rpc_errors_total                  // RPC 错误数

```

### 7.2 Grafana 面板

新增 "Cross-Shard Transactions" 仪表盘：

- 跨分片事务比例饼图

- 2PC 延迟分布直方图

- 分片间通信流量热力图

---

## 8. 实现里程碑

### Phase 1: 基础框架 (Week 1-2)

- [ ] 定义 `ShardId`、`ShardConfig` 类型

- [ ] 实现 `ShardCoordinator` 骨架

- [ ] 集成 `tonic` 依赖并定义 `.proto` 文件

### Phase 2: 2PC 协议 (Week 2-3)

- [ ] 实现 prepare/commit/abort RPC 处理器

- [ ] 扩展 `MvccScheduler` 支持远程验证

- [ ] 添加事务状态持久化（WAL）

### Phase 3: 测试与优化 (Week 3-4)

- [ ] 跨分片基准测试工具 (`mixed_path_bench --shards:4`)

- [ ] 集成测试：2-shard / 4-shard 场景

- [ ] 性能调优：批量提交、流水线

### Phase 4: 监控与文档 (Week 4)

- [ ] Prometheus 指标导出

- [ ] 更新 `docs/API.md`

- [ ] 编写跨分片最佳实践文档

---

## 9. 风险与缓解

| 风险 | 影响 | 缓解措施 |
|-----|------|---------|
| **网络分区** | 事务永久阻塞 | 引入超时机制 + Paxos 恢复 |
| **协调器单点故障** | 系统不可用 | 协调器高可用（Raft 集群） |
| **跨分片热点** | 性能退化 | 动态负载均衡 + 对象迁移 |
| **2PC 死锁** | 事务饥饿 | Wait-For Graph 检测 + 优先级中止 |

---

## 10. 参考文献

1. **Google Spanner**: TrueTime + 2PC  
   - https://research.google/pubs/pub39966/

2. **Sui Blockchain**: 对象所有权模型  
   - https://docs.sui.io/concepts/object-ownership

3. **Calvin**: 确定性数据库  
   - http://cs.yale.edu/homes/thomson/publications/calvin-sigmod12.pdf

4. **CockroachDB**: 分布式事务实现  
   - https://www.cockroachlabs.com/docs/stable/architecture/transaction-layer.html

---

## 附录: 数据结构定义

```rust
// src/vm-runtime/src/shard_types.rs

pub type ShardId = u16;
pub type TxnId = u64;

#[derive(Debug, Clone)]
pub struct ShardConfig {
    pub num_shards: usize,
    pub shard_endpoints: HashMap<ShardId, String>, // "127.0.0.1:5000"
    pub timeout_ms: u64,
}

#[derive(Debug, Clone)]
pub enum Decision {
    Commit,
    Abort,
}

#[derive(Debug, Clone)]
pub struct ConflictReason {
    pub object_id: ObjectId,
    pub expected_version: Version,
    pub actual_version: Version,
}

```

---

## 11. 隐私跨分片 (ZK) 设计

### 11.1 目标与约束

- 目标：在 2PC 语义下实现跨分片的隐私交易（不泄漏明文数据），保持原子性与一致性。

- 约束：
    - 分片节点不可见敏感明文；
    - 参与分片能够独立验证证明正确性；
    - 协调器只做决议，不做明文计算；
    - 与现有 SuperVM ZK 批量验证机制无缝集成（ZK_BATCH_* 配置与指标）。

### 11.2 数据模型与消息扩展

在 PrepareRequest 增加隐私证明载荷：

```rust
// 仅展示新增/相关字段
struct PrivacyProof {
        system: ZkSystem,              // Groth16_BLS12_381 | Groth16_BN254
        proof_bytes: Vec<u8>,          // 证明本体（压缩）
        public_inputs: Vec<Vec<u8>>,   // 公开输入序列化（字段元素）
        commitments: Vec<Vec<u8>>,     // 承诺/承诺打开 (可选)
}

struct PrepareRequest {
        // ... 原字段
        trace_id: u128,                // 分布式追踪
        coordinator_epoch: u64,        // 协调器任期
        retry_count: u32,              // 重试次数
        privacy: Option<PrivacyProof>, // 隐私交易附加字段
}

```

对应 Proto（摘要，详见 proto/cross_shard.proto）：

```protobuf
message PrivacyProof {
    enum ZkSystem { ZK_GROTH16_BLS12_381 = 0; ZK_GROTH16_BN254 = 1; }
    ZkSystem system = 1;
    bytes proof_bytes = 2;
    repeated bytes public_inputs = 3;
    repeated bytes commitments = 4; // optional
}

message PrepareRequest {
    // ...
    uint128 trace_id = 100;         // 若不支持，拆为两段 uint64 高低位
    uint64 coordinator_epoch = 101;
    uint32 retry_count = 102;
    optional PrivacyProof privacy = 200;
}

```

### 11.3 验证策略

- 方案 A：协调器统一验证（一次验证）
    - 优点：开销最小；
    - 缺点：参与分片无法独立信任验证结果。

- 方案 B：各参与分片独立验证（推荐）
    - 优点：每个分片对本地写入安全性有独立背书；
    - 缺点：多次验证，需优化（使用 SuperVM 批量缓冲）。

采用方案 B，并引入 SuperVM 批量验证：

```rust
// 伪代码：在 ShardService.prepare_txn 中
if let Some(p) = req.privacy.as_ref() {
        // 将 proof/public_inputs 交给 SuperVM 批量缓冲
        if !supervm.verify_zk_proof(Some(&p.proof_bytes), Some(&concat_inputs(&p.public_inputs))) {
                return VoteNo(Reason::InvalidProof);
        }
}

```

### 11.4 执行流程（隐私场景）

```

Client → Coordinator: Submit Txn(read_set, write_set, privacy_proof)
                 │
                 ├─ Phase 1: PREPARE
                 │   ├─> Shard A: verify_zk_proof() + lock → YES/NO
                 │   └─> Shard B: verify_zk_proof() + lock → YES/NO
                 │
                 ├─ Collect Votes (all YES ? COMMIT : ABORT)
                 │
                 └─ Phase 2: COMMIT/ABORT (两边一致)

```

### 11.5 失败与恢复

- 证明失败：参与分片直接投 NO，协调器决议 ABORT。

- 超时：协调器记录 `cross_shard_prepare_timeout_total`，并发起 ABORT。

- 协调器故障恢复：新主节点根据 WAL/状态机重放决议（见第 3.3 状态机的 RECOVERING）。

### 11.6 监控与指标映射

- 新增：
    - `cross_shard_privacy_txn_total`（隐私跨分片事务数）
    - `cross_shard_privacy_abort_total{reason="invalid_proof|timeout|conflict"}`

- 复用：
    - `vm_privacy_zk_batch_verify_*`（批量验证吞吐/延迟/失败率）

### 11.7 与 SuperVM 集成

- 配置：遵循 `ZK_BATCH_ENABLE|SIZE|FLUSH_INTERVAL_MS`；协调器与参与分片均可独立配置。

- 推荐：在参与分片启用批量模式，减少多事务并发下的平均验证开销。

- 安全：各分片必须独立完成验证成功后才可进行本地锁与 prepare。

---

## 12. Proto 说明

已新增 `proto/cross_shard.proto` 草案，覆盖 Prepare/Commit/Abort 及隐私字段（PrivacyProof）。
后续将配合 tonic 代码生成与服务骨架实现。


---

**下一步**: 创建 `src/vm-runtime/src/shard_coordinator.rs` 实现 2PC 协调器。
