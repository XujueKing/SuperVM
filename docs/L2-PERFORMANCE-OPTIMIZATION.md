# L2 Executor 性能优化建议

## 📊 当前性能基准 (Windows, Trace backend)

### Fibonacci 程序性能
| 复杂度 | 输出结果 | 步骤数 | 执行时间 | 吞吐量 (步骤/秒) |
|--------|---------|--------|---------|-----------------|
| fib(5) | 5 | 7 | 31.1µs | ~225K steps/s |
| fib(10) | 55 | 12 | 23.9µs | ~502K steps/s |
| fib(20) | 6765 | 22 | 48µs | ~458K steps/s |
| fib(50) | 12586269025 | 52 | 94.8µs | ~549K steps/s |

**观察**: 
- 吞吐量稳定在 200K-550K steps/s
- 小规模任务 (fib 5-10) 受启动开销影响
- 中大规模任务 (fib 20-50) 性能更稳定

---

## 🎯 优化建议 (按优先级)

### P0: 高优先级 (立即可做)

#### 1. 批量处理优化
**问题**: 当前每次 `prove()` 都有固定开销  
**建议**: 实现批量证明生成接口

```rust
impl TraceZkVm {
    pub fn prove_batch<P: TraceProgram>(
        &self,
        programs: &[&P],
        witnesses: &[&[u64]]
    ) -> Result<Vec<Proof>> {
        // 共享 VM 状态,减少重复初始化
        programs.iter()
            .zip(witnesses)
            .map(|(p, w)| self.prove(p, w))
            .collect()
    }
}
```

**预期提升**: 10-20% (减少初始化开销)

---

#### 2. 并行证明生成
**问题**: 多个独立证明顺序生成,未利用多核  
**建议**: 使用 rayon 并行化

```rust
use rayon::prelude::*;

pub fn prove_parallel<P: TraceProgram + Sync>(
    &self,
    programs: &[&P],
    witnesses: &[&[u64]]
) -> Result<Vec<Proof>> {
    programs.par_iter()
        .zip(witnesses.par_iter())
        .map(|(p, w)| self.prove(p, w))
        .collect()
}
```

**预期提升**: 3-4x (4 核 CPU)

---

#### 3. 证明缓存
**问题**: 相同程序+输入重复计算  
**建议**: 实现 LRU 缓存

```rust
use lru::LruCache;

pub struct CachedZkVm {
    vm: TraceZkVm,
    cache: Arc<Mutex<LruCache<ProofKey, Proof>>>,
}

impl CachedZkVm {
    pub fn prove_cached<P: TraceProgram>(
        &self,
        program: &P,
        witness: &[u64]
    ) -> Result<Proof> {
        let key = ProofKey::new(program, witness);
        
        if let Some(cached) = self.cache.lock().get(&key) {
            return Ok(cached.clone()); // 缓存命中
        }
        
        let proof = self.vm.prove(program, witness)?;
        self.cache.lock().put(key, proof.clone());
        Ok(proof)
    }
}
```

**预期提升**: 100x (缓存命中时)

---

### P1: 中优先级 (本周完成)

#### 4. 聚合器优化
**问题**: MerkleAggregator 使用 Vec 动态扩容  
**建议**: 预分配容量

```rust
impl MerkleAggregator {
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            proofs: Vec::with_capacity(capacity),
            ..Default::default()
        }
    }
}
```

**预期提升**: 5-10% (大批量聚合)

---

#### 5. 序列化优化
**问题**: bincode 序列化未配置压缩  
**建议**: 启用压缩选项

```rust
use bincode::Options;

fn serialize_proof(proof: &Proof) -> Result<Vec<u8>> {
    bincode::DefaultOptions::new()
        .with_varint_encoding()
        .with_little_endian()
        .serialize(proof)
}
```

**预期提升**: 20-30% (证明大小)

---

#### 6. 内存池复用
**问题**: 每次 prove 分配新 Vec  
**建议**: 使用对象池

```rust
use pool::Pool;

pub struct ZkVmPool {
    vm_pool: Pool<TraceZkVm>,
}

impl ZkVmPool {
    pub fn prove<P: TraceProgram>(
        &self,
        program: &P,
        witness: &[u64]
    ) -> Result<Proof> {
        let vm = self.vm_pool.get()?;
        vm.prove(program, witness)
    }
}
```

**预期提升**: 15-25% (减少 GC 压力)

---

### P2: 低优先级 (未来迭代)

#### 7. SIMD 加速
**问题**: Merkle tree 哈希计算未使用 SIMD  
**建议**: 使用 SHA256 SIMD 实现

```rust
#[cfg(target_arch = "x86_64")]
use sha2::Sha256Simd;

#[cfg(not(target_arch = "x86_64"))]
use sha2::Sha256 as Sha256Simd;
```

**预期提升**: 10-20% (Merkle tree 构建)

---

#### 8. 异步 IO
**问题**: 配置文件同步加载阻塞  
**建议**: 使用 tokio async

```rust
pub async fn from_config_file_async(path: &str) -> Result<L2Runtime> {
    let content = tokio::fs::read_to_string(path).await?;
    let config: RuntimeConfig = toml::from_str(&content)?;
    Ok(Self::with_config(config.backend.unwrap_or_default(), config)?)
}
```

**预期提升**: 改善用户体验 (非阻塞)

---

#### 9. GPU 加速 (RISC0)
**问题**: RISC0 证明生成未使用 GPU  
**建议**: 启用 CUDA 支持

```toml
[dependencies.risc0-zkvm]
version = "1.0"
features = ["cuda", "metal"] # GPU 加速
```

**预期提升**: 10-100x (取决于 GPU)

---

## 📈 预期性能提升路线图

### 短期 (1-2 周)
1. ✅ 批量处理优化 → +10-20%
2. ✅ 并行证明生成 → +300%
3. ✅ 证明缓存 → +10000% (缓存命中)

**综合提升**: 3-4x (实际工作负载)

### 中期 (1 个月)
4. ✅ 聚合器优化 → +5-10%
5. ✅ 序列化优化 → +20-30% (大小)
6. ✅ 内存池复用 → +15-25%

**综合提升**: 4-6x

### 长期 (3 个月)
7. ✅ SIMD 加速 → +10-20%
8. ✅ 异步 IO → 改善体验
9. ✅ GPU 加速 → +10-100x (RISC0)

**综合提升**: 50-500x (GPU + 所有优化)

---

## 🔧 实现计划

### Phase 1: 低成本高收益 (本周)
```rust
// 1. 添加批量接口
impl TraceZkVm {
    pub fn prove_batch(...) -> Result<Vec<Proof>>
}

// 2. 添加 rayon 依赖
[dependencies]
rayon = "1.11"

// 3. 实现并行化
programs.par_iter().map(|p| prove(p)).collect()
```

### Phase 2: 缓存与池化 (下周)
```rust
// 4. 添加 lru 和 pool 依赖
[dependencies]
lru = "0.12"
object-pool = "0.5"

// 5. 实现缓存层
pub struct CachedZkVm { ... }

// 6. 实现对象池
pub struct ZkVmPool { ... }
```

### Phase 3: 高级优化 (未来)
```rust
// 7. SIMD
#[cfg(target_feature = "avx2")]
use fast_sha256;

// 8. 异步
pub async fn prove_async(...) -> Result<Proof>

// 9. GPU
risc0-zkvm = { features = ["cuda"] }
```

---

## 📊 性能测试建议

### 基准测试场景
1. **单个小任务** - fib(5), 测量启动开销
2. **单个大任务** - fib(100), 测量计算能力
3. **批量小任务** - 100x fib(5), 测量吞吐量
4. **批量大任务** - 10x fib(50), 测量并行效率
5. **缓存命中** - 重复 fib(10), 测量缓存效果

### 测试指标
- **时间**: 平均值, 中位数, P99
- **吞吐量**: 步骤/秒, 证明/秒
- **内存**: 峰值使用, 平均使用
- **CPU**: 利用率, 核心数缩放

---

## 🎯 成功指标

### 短期目标 (1-2 周)
- ✅ 批量处理吞吐量 > 1M steps/s
- ✅ 并行化效率 > 70% (4 核)
- ✅ 缓存命中率 > 50% (实际负载)

### 中期目标 (1 个月)
- ✅ 端到端延迟 < 100ms (fib 20)
- ✅ 内存使用 < 500MB (1000 证明)
- ✅ 证明大小 < 1KB (压缩)

### 长期目标 (3 个月)
- ✅ GPU 加速提升 > 10x
- ✅ RISC0 生产模式 < 5s (fib 50)
- ✅ 支持 10K+ 并发请求

---

## 📚 参考资源

- **Rayon**: https://docs.rs/rayon/
- **LRU Cache**: https://docs.rs/lru/
- **Object Pool**: https://docs.rs/object-pool/
- **RISC0 Performance**: https://dev.risczero.com/api/zkvm/performance
- **Criterion Benchmarks**: `zkvm-bench/README.md`

---

**更新时间**: 2025-11-14  
**当前版本**: l2-executor v0.1.0  
**下一步**: 实施 Phase 1 优化 (批量+并行)
