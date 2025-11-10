# Phase C: FastPath 性能画像与 1M TPS 优化计划

**目标**: 使用 perf/Flamegraph 分析 FastPath 热点，优化至 1M TPS

**开发者**: king  
**开始时间**: 2025-11-10  
**预计周期**: 2-3 周

---

## 🎯 总体目标

当前 FastPath 性能基线（基于 Phase 5 路由实现）：
- ✅ **FastPath 纯吞吐**: **28.57M TPS** (2857万 TPS, Release, Windows)
- ✅ **FastPath 延迟**: **34-35 纳秒** (avg)
- ✅ **混合负载 (80% Fast)**: **1.20M TPS**
- **多线程热键冲突 (Consensus)**: ~290K TPS (10 线程, MVCC 路径)
- **Consensus 纯吞吐**: 377K TPS (100% 共享对象)

**Phase C 目标**（已基本达成，需进一步验证与优化）：
- ✅ FastPath 单核吞吐 **已达到 28.57M TPS**（超越目标 57倍）
- ✅ P99 延迟 **已达到 34-35ns**（比 100μs 目标低 3000倍）
- 🎯 **新目标**: Consensus 路径优化至 **1M TPS**（当前 377K）
- 🎯 **新目标**: 多核并行扩展至 **50M TPS**（8 核）
- 🎯 **新目标**: 跨分片隐私验证吞吐 **> 10K TPS**（当前 ~200 TPS）

---

## 📊 Phase C.1: 新性能瓶颈识别 (Week 1)

### 当前性能全景

基于 Phase 5 实测数据（PHASE5-METRICS-2025-11-10.md）：

| 路径 | 当前 TPS | 延迟 | 成功率 | 瓶颈分析 |
|------|----------|------|--------|----------|
| **FastPath** | 28.57M | 34-35ns | 100% | ✅ **已达极限**（零锁/零分配） |
| **Consensus (纯)** | 377K | ~2.7μs | 100% | ⚠️ **MVCC 版本管理** |
| **混合 (80% Fast)** | 1.20M | 34ns/~2.7μs | 100% | ⚠️ **Consensus 拖累** |
| **跨分片隐私** | ~200 | ~5ms | - | ⚠️ **ZK 验证开销** |

**新优化方向**（FastPath 已优化到位，聚焦其他路径）：

1. **Consensus 路径**: 377K → **1M TPS**
   - MVCC 版本链优化（使用 `smallvec` 减少堆分配）
   - 无锁哈希表替换 `RwLock<HashMap>`（`dashmap`）
   - 批量提交优化（分组 flush）

2. **跨分片隐私验证**: 200 TPS → **10K TPS**
   - SuperVM 批量验证并行化（当前串行）
   - GPU 加速 ZK 验证（CUDA/OpenCL）
   - 分片间版本查询并行化（已实现 `get_remote_versions`）

3. **多核扩展**: 28.57M → **50M TPS**
   - FastPath 分区执行器（16/32 分区）
   - NUMA 感知调度
   - CPU 亲和性绑定

### 工具链准备（保持不变）

#### Linux 环境 (推荐)
```bash
# 安装 perf (Linux 内核性能分析工具)
sudo apt-get install linux-tools-common linux-tools-generic

# 安装 Flamegraph 工具
git clone https://github.com/brendangregg/FlameGraph
export PATH=$PATH:$(pwd)/FlameGraph

# Rust 符号优化编译
cargo build --release --features parallel-mvcc,cross-shard
```

#### Windows 环境 (替代方案)
```powershell
# 使用 cargo-flamegraph (跨平台)
cargo install flamegraph

# Windows Performance Analyzer (WPA)
# https://docs.microsoft.com/en-us/windows-hardware/test/wpt/
```

### 基准测试准备（更新）

重点测试 **Consensus 路径** 和 **跨分片隐私**（FastPath 已达极限）：

```rust
// benches/consensus_1m_tps_bench.rs (新目标)
use vm_runtime::{SuperVM, OwnershipManager, MvccScheduler, Transaction, Privacy};
use criterion::{black_box, criterion_group, criterion_main, Criterion, BenchmarkId};

fn consensus_path_benchmark(c: &mut Criterion) {
    let ownership = OwnershipManager::new();
    let scheduler = MvccScheduler::new();
    let vm = SuperVM::new(&ownership).with_scheduler(&scheduler);
    
    // 注册 1K 共享对象（高竞争）
    for i in 0..1000u64 {
        let obj_id = i.to_le_bytes().repeat(4);
        let addr_a = 1u64.to_le_bytes().repeat(4);
        let addr_b = 2u64.to_le_bytes().repeat(4);
        // 注册为共享对象（多个所有者）
        ownership.register_shared(&obj_id, &[addr_a, addr_b]);
    }
    
    c.bench_function("consensus_high_contention", |b| {
        let mut counter = 0u64;
        b.iter(|| {
            let obj_id = (counter % 1000).to_le_bytes().repeat(4);
            let addr = (counter % 2 + 1).to_le_bytes().repeat(4);
            let tx = Transaction {
                from: addr,
                objects: vec![obj_id],
                privacy: Privacy::Public,
            };
            // 强制走 Consensus 路径
            vm.execute_consensus_path(counter, &tx, || Ok(42));
            counter += 1;
        });
    });
}

fn cross_shard_privacy_benchmark(c: &mut Criterion) {
    // 测试跨分片隐私验证吞吐
    // 目标: 从 200 TPS 提升到 10K TPS
    // 使用 SuperVM::verify_zk_proof_batch 批量验证
}

criterion_group!(benches, consensus_path_benchmark, cross_shard_privacy_benchmark);
criterion_main!(benches);
```

### 性能画像采集

#### 方法 1: perf + Flamegraph (Linux)
```bash
# 采集 CPU 性能数据（60 秒，99Hz 采样）
perf record -F 99 -g -- cargo run --release --example fastpath_pressure_test

# 生成 Flamegraph
perf script | stackcollapse-perf.pl | flamegraph.pl > fastpath.svg

# 查看火焰图（浏览器打开）
firefox fastpath.svg
```

#### 方法 2: cargo-flamegraph (跨平台)
```bash
cargo flamegraph --bench fastpath_1m_tps_bench -- --bench
# 输出: flamegraph.svg
```

### 预期热点分析（更新）

基于 FastPath 已达 28.57M TPS，现聚焦 **Consensus 路径** 瓶颈：

**Consensus 路径预期热点**（377K TPS → 1M TPS）：

1. **MVCC 版本链分配** (~40%): `Box::new(VersionNode)` 频繁堆分配
2. **锁竞争** (~25%): `RwLock::write` 在版本链更新时阻塞
3. **版本链遍历** (~15%): 线性扫描查找可见版本
4. **冲突检测** (~10%): `check_conflicts` 比较读集版本
5. **哈希计算** (~10%): `HashMap::get` / `BTreeMap::insert`

**跨分片隐私验证预期热点**（200 TPS → 10K TPS）：

1. **ZK 证明验证** (~70%): `arkworks::verify` 单次耗时 ~4-5ms
2. **串行验证** (~15%): 逐个证明验证，未利用并行性
3. **网络 RPC** (~10%): 分片间版本查询延迟
4. **内存分配** (~5%): proof_bytes 复制开销

### 关键指标收集

使用 `perf stat` 获取硬件计数器：

```bash
perf stat -e cache-misses,cache-references,L1-dcache-loads,L1-dcache-load-misses \
    cargo run --release --example fastpath_pressure_test
```

**关注指标**:
- Cache Miss Rate < 5%
- Instructions Per Cycle (IPC) > 2.0
- Branch Mispredict Rate < 1%

---

## ⚡ Phase C.2: 针对性优化实施 (Week 2)

### 优化方向 1: Consensus 路径 - MVCC 版本链优化

**问题**: 频繁 `Box::new(VersionNode)` 导致碎片化，版本链遍历慢

**方案 1.1**: 使用 `smallvec` 内联前 4 个版本

```rust
use smallvec::SmallVec;

pub struct MvccVersionChain {
    // 前 4 个版本栈内存，超过才堆分配
    versions: SmallVec<[VersionNode; 4]>,
}

impl MvccVersionChain {
    pub fn add_version(&mut self, node: VersionNode) {
        self.versions.push(node); // 大多数对象 ≤ 4 版本，零堆分配
    }
    
    pub fn find_visible(&self, tx_id: TxId) -> Option<&VersionNode> {
        // 逆序查找（最新版本优先）
        self.versions.iter().rev().find(|v| v.committed && v.tx_id < tx_id)
    }
}
```

**预期收益**: 减少 30-40% 版本链堆分配

**方案 1.2**: 无锁版本链（`crossbeam::epoch` RCU）

```rust
use crossbeam::epoch::{self, Atomic, Owned, Shared};

pub struct LockFreeVersionChain {
    head: Atomic<VersionNode>,
}

impl LockFreeVersionChain {
    pub fn add_version(&self, node: VersionNode) {
        let guard = epoch::pin();
        loop {
            let head = self.head.load(Ordering::Acquire, &guard);
            let mut new_node = Owned::new(node.clone());
            new_node.next = head;
            if self.head.compare_exchange(head, new_node, Ordering::Release, &guard).is_ok() {
                break;
            }
        }
    }
}
```

**预期收益**: 消除版本链更新锁竞争，+20-30% 并发吞吐

### 优化方向 2: 跨分片隐私验证并行化

**问题**: SuperVM 批量验证当前串行处理，单次 ZK 验证 ~4-5ms

**方案 2.1**: 证明验证多线程并行（Rayon）

```rust
use rayon::prelude::*;

impl SuperVM {
    pub fn verify_zk_proof_batch_parallel(
        &self,
        proofs: &[PrivacyProof],
        curve: CurveType,
    ) -> Vec<bool> {
        // 并行验证（利用多核）
        proofs.par_iter().map(|proof| {
            self.verify_zk_proof(&proof.proof_bytes, &proof.public_inputs, curve)
        }).collect()
    }
}
```

**预期收益**: 8 核下吞吐提升 6-7×（200 TPS → 1400 TPS）

**方案 2.2**: GPU 加速 ZK 验证（CUDA）

```rust
#[cfg(feature = "gpu-accel")]
use cudarc::driver::*;

impl SuperVM {
    pub fn verify_zk_proof_gpu(
        &self,
        proofs: &[PrivacyProof],
    ) -> Result<Vec<bool>, GpuError> {
        // 批量传输到 GPU
        let gpu_proofs = self.cuda_ctx.htod_copy(proofs)?;
        
        // GPU 并行验证（1000+ 核）
        let results = self.cuda_verify_kernel.launch(gpu_proofs)?;
        
        // 传回 CPU
        Ok(self.cuda_ctx.dtoh_copy(results)?)
    }
}
```

**预期收益**: GPU 加速下吞吐提升 50-100×（200 TPS → 10K-20K TPS）

**备注**: Phase 8 GPU 加速已规划，可提前实施 PoC

### 优化方向 3: FastPath 多核扩展（28.57M → 50M TPS）

**问题**: 当前 FastPath 单核 28.57M TPS，需多核并行突破 50M

**方案 3.1**: 分区并行执行器

```rust
pub struct PartitionedFastPath {
    partitions: Vec<FastPathExecutor>,
    num_partitions: usize,
}

impl PartitionedFastPath {
    pub fn new(num_partitions: usize) -> Self {
        let partitions = (0..num_partitions)
            .map(|_| FastPathExecutor::new())
            .collect();
        Self { partitions, num_partitions }
    }
    
    fn get_partition(&self, obj_id: &[u8; 32]) -> &FastPathExecutor {
        let hash = obj_id[0] as usize; // 简化哈希
        &self.partitions[hash % self.num_partitions]
    }
    
    pub fn execute_parallel<F>(&self, tx: &Transaction, op: F) -> Result<i32, String>
    where F: FnOnce() -> Result<i32, String> + Send
    {
        let partition = self.get_partition(&tx.objects[0]);
        partition.execute(tx.id, op)
    }
}
```

**预期收益**: 8 核下 FastPath 达到 50M+ TPS（当前单核 28.57M × 多核系数 ~1.8）

**方案 3.2**: NUMA 感知调度

```rust
use numa::{NodeMask, get_cpu_node};

impl PartitionedFastPath {
    pub fn new_numa_aware(cores_per_node: usize) -> Self {
        let num_nodes = numa::get_max_node() + 1;
        let partitions: Vec<_> = (0..num_nodes * cores_per_node)
            .map(|i| {
                let node = i / cores_per_node;
                // 绑定线程到 NUMA 节点
                let mut executor = FastPathExecutor::new();
                executor.pin_to_numa_node(node);
                executor
            })
            .collect();
        Self { partitions, num_partitions: partitions.len() }
    }
}
```

**预期收益**: 多 NUMA 节点服务器上提升 10-20%（减少跨节点内存访问）

### 优化方向 4: Consensus 路径批量提交优化

**问题**: 当前 Consensus 路径 377K TPS，每笔事务独立提交

**方案**: 分组批量 flush（减少 MVCC 写放大）

```rust
pub struct BatchedConsensusExecutor {
    pending_commits: DashMap<TxId, VersionNode>,
    batch_size: usize,
}

impl BatchedConsensusExecutor {
    pub fn execute_batch(&self, txs: Vec<Transaction>) -> Vec<Result<i32, String>> {
        let mut results = Vec::with_capacity(txs.len());
        let mut batch_buffer = Vec::with_capacity(self.batch_size);
        
        for tx in txs {
            // 执行逻辑
            let res = self.execute_single(&tx);
            batch_buffer.push((tx.id, res.clone()));
            results.push(res);
            
            // 达到批次大小,批量 flush
            if batch_buffer.len() >= self.batch_size {
                self.flush_batch(&batch_buffer);
                batch_buffer.clear();
            }
        }
        
        // flush 剩余事务
        if !batch_buffer.is_empty() {
            self.flush_batch(&batch_buffer);
        }
        
        results
    }
    
    fn flush_batch(&self, batch: &[(TxId, Result<i32, String>)]) {
        // 单次锁获取,批量写入版本链
        for (tx_id, result) in batch {
            if result.is_ok() {
                self.mvcc.commit_batch(tx_id);
            }
        }
    }
}
```

**预期收益**: Consensus 吞吐提升至 600K-800K TPS（批次大小 32-64）

---

## 🧪 Phase C.3: 更新基准测试与验证 (Week 3)

### 基准测试矩阵（更新）

| 场景 | 线程数 | 对象数 | 冲突率 | 当前 TPS | 目标 TPS | 优化后 TPS |
|------|-------|--------|--------|---------|---------|-----------|
| **FastPath (单核)** | 1 | 10K | 0% | **28.57M** | 50M | ? |
| **FastPath (8核)** | 8 | 100K | 0% | ~35M 估算 | 50M | ? |
| **Consensus (纯)** | 1 | 2K | 10% | 377K | 1M | ? |
| **Consensus (批量)** | 1 | 2K | 10% | 377K | 800K | ? |
| **混合 (80% Fast)** | 4 | 50K | 1% | 1.20M | 3M | ? |
| **跨分片隐私 (CPU)** | 8 | - | - | ~200 | 1.5K | ? |
| **跨分片隐私 (GPU)** | - | - | - | ~200 | 10K | ? |

### 性能回归测试（更新）

```bash
# Consensus 路径基准测试
cargo bench --bench consensus_1m_tps_bench

# 跨分片隐私验证基准测试
cargo bench --bench cross_shard_privacy_bench

# FastPath 多核扩展测试
cargo bench --bench fastpath_multicore_bench -- --threads 1,2,4,8

# 保存优化后基线
cargo bench -- --save-baseline phase_c_optimized

# 对比优化前后
cargo bench -- --baseline phase_c_optimized
```

### 延迟分布验证

```rust
// 确保 P99 < 100μs
let latencies = collect_latencies(1_000_000);
latencies.sort();
let p50 = latencies[500_000];
let p90 = latencies[900_000];
let p99 = latencies[990_000];

assert!(p50 < Duration::from_micros(50), "P50: {:?}", p50);
assert!(p90 < Duration::from_micros(80), "P90: {:?}", p90);
assert!(p99 < Duration::from_micros(100), "P99: {:?}", p99);
```

---

## 📈 成功指标（更新）

### 性能指标
- ✅ **FastPath 单核 TPS**: 28.57M (已达成)
- ✅ **FastPath 延迟**: 34-35ns (已达成)
- 🎯 **FastPath 8核 TPS** > 50M（新目标）
- 🎯 **Consensus 路径 TPS** > 1M（当前 377K）
- 🎯 **跨分片隐私 CPU** > 1.5K TPS（当前 ~200）
- 🎯 **跨分片隐私 GPU** > 10K TPS（PoC）
- 🎯 **混合负载 (80% Fast)** > 3M TPS（当前 1.20M）

### 可观测性指标
- ✅ FastPath Flamegraph 热点 < 10%（零锁/零分配已证明）
- 🎯 Consensus Flamegraph 显示版本链分配 < 20%（当前预估 40%）
- 🎯 跨分片隐私 Flamegraph 显示 ZK 验证并行化效果
- 🎯 perf stat IPC > 2.0（CPU 利用率优化）

### 文档指标
- ✅ Phase 5 性能报告已完成（PHASE5-METRICS-2025-11-10.md）
- 🎯 Phase C 优化报告（优化前后对比 + Flamegraph）
- 🎯 跨分片隐私优化最佳实践（CPU vs GPU 选型指南）

---

## 🔧 工具与依赖（更新）

### Cargo 依赖
```toml
[dependencies]
dashmap = "5.5"          # 无锁并发哈希表（Consensus 优化）
smallvec = "1.11"        # 栈内联向量（MVCC 版本链）
crossbeam = "0.8"        # 无锁数据结构（RCU 版本链）
rayon = "1.8"            # 数据并行（ZK 验证并行化）
parking_lot = "0.12"     # 高性能锁（已有）
numa = "0.2"             # NUMA 感知调度（可选）

[dev-dependencies]
criterion = "0.5"        # 基准测试框架
flamegraph = "0.6"       # 火焰图生成

[features]
gpu-accel = ["cudarc"]   # GPU 加速（Phase 8 提前实施）

[dependencies.cudarc]
version = "0.10"
optional = true
```

### 系统工具
```bash
# Linux
sudo apt-get install linux-tools-common linux-tools-generic

# macOS
brew install flamegraph

# Windows
cargo install flamegraph
```

---

## 📋 任务清单（更新）

### Week 1: 新瓶颈识别
- [x] ~~FastPath 性能画像~~（已完成,28.57M TPS）
- [ ] Consensus 路径 Flamegraph 分析（识别版本链热点）
- [ ] 跨分片隐私验证性能剖析（CPU vs GPU 对比）
- [ ] 创建 `consensus_1m_tps_bench` 基准测试
- [ ] 创建 `cross_shard_privacy_bench` 基准测试
- [ ] 分析 `perf stat` 硬件计数器（Consensus 路径）

### Week 2: 针对性优化
- [ ] **Consensus 优化 1**: 引入 `smallvec` 内联版本链
- [ ] **Consensus 优化 2**: 无锁版本链（`crossbeam::epoch`）
- [ ] **Consensus 优化 3**: 批量提交优化（分组 flush）
- [ ] **跨分片隐私优化 1**: Rayon 并行验证（8 核）
- [ ] **跨分片隐私优化 2**: GPU 加速 PoC（cudarc）
- [ ] **FastPath 优化**: 分区并行执行器（16/32 分区）
- [ ] 每项优化后运行 micro-benchmark

### Week 3: 验证与文档
- [ ] 完整基准测试矩阵（Consensus/隐私/FastPath 多核）
- [ ] 延迟分布验证（P50/P90/P99）
- [ ] 对比优化前后 Flamegraph
- [ ] 编写 Phase C 性能优化报告
- [ ] 更新 ROADMAP.md Phase C 进度
- [ ] 归档 Flamegraph 和基准数据

---

## 🚀 未来扩展方向（更新）

### Phase C+: GPU 加速 ZK 验证（提前实施）
- ✅ 已规划在 Phase 8，可提前 PoC
- 使用 cudarc/CUDA 批量验证 Groth16 证明
- 目标: 跨分片隐私验证达到 10K-20K TPS
- 预期收益: GPU 并行验证 50-100× CPU

### Phase C++: 自适应批量大小调优
- 根据负载动态调整 Consensus 批次大小（16/32/64/128）
- A/B 测试不同批次对延迟/吞吐的影响
- 实现自适应策略（低延迟 vs 高吞吐模式切换）

### Phase C+++: NUMA 优化与 CPU 亲和性
- 多 NUMA 节点服务器分片绑定
- CPU 核心亲和性绑定（减少上下文切换）
- 本地内存访问优化（跨节点访问 penalty ~50%）

### Phase C++++: FastPath 极限挑战（100M TPS）
- 当前 28.57M TPS 单核，8 核理论上限 ~200M
- 实际目标: 50M-100M TPS（考虑多核扩展系数 ~1.8-3.5）
- 技术路径: 分区 + NUMA + 零拷贝 + 内存预分配

---

**下一步**: 
1. **立即**: 运行 Consensus 路径 Flamegraph，识别版本链热点
2. **本周**: 实施 `smallvec` 优化，测试 Consensus 吞吐提升
3. **下周**: 启动跨分片隐私 Rayon 并行化，目标 1.5K TPS
4. **Phase 8 提前**: GPU 加速 ZK 验证 PoC，突破 10K TPS！�
