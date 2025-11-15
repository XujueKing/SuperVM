# ZK Acceleration Technical Plan (Phase 13 M13.5)

**目标**: GPU 加速 ZK 证明生成，实现 20x+ 性能提升

## 背景调研

### 现有基础

根据项目已有研究 (`docs/research/`, `zk-groth16-test/`, `halo2-eval/`):

1. **Groth16 (arkworks)** ✅
   - CPU 基线: ~10ms prove (Combined circuit, 72 约束)
   - 证明大小: 128 bytes
   - 验证时间: ~3.6ms
   - 代码: `zk-groth16-test/`

2. **Halo2 (halo2_proofs)** ✅
   - CPU 基线: ~106ms prove (k=8, 256 rows)
   - 证明大小: ~1.7KB
   - 验证时间: ~4.8ms
   - 代码: `halo2-eval/`

### ZK 证明瓶颈

ZK 证明生成的计算瓶颈主要在：

1. **MSM (Multi-Scalar Multiplication)** - 70-80% 时间
   - 计算: `Σ(scalar_i × point_i)` (椭圆曲线点的标量乘法求和)
   - 规模: Groth16 需要 2 次 MSM (G1 和 G2)
   - CPU: O(n log n) Pippenger 算法
   - GPU 优化: 并行计算 + bucket method

2. **FFT (Fast Fourier Transform)** - 10-20% 时间
   - 计算: 多项式乘法和插值
   - 规模: O(n log n)
   - GPU 优化: Cooley-Tukey radix-2 FFT

## 技术方案选择

### 方案对比

| 方案 | 优点 | 缺点 | 推荐度 |
|-----|------|------|--------|
| **A. bellman-cuda** | 成熟的 Groth16 CUDA 实现，Filecoin 使用 | 1) CUDA 专用(NVIDIA only) 2) C++/CUDA 集成复杂 | ⭐⭐⭐ |
| **B. sppark (supranational)** | 高性能 MSM，支持多曲线 | 需要 CUDA，集成工作量大 | ⭐⭐⭐⭐ |
| **C. WGSL MSM/FFT** | 跨平台(WGPU)，与现有架构一致 | 需要自研实现，性能可能不如 CUDA | ⭐⭐⭐⭐⭐ |
| **D. arkworks + sppark FFI** | 利用现有 arkworks 代码，FFI 调用 GPU | FFI 开销，维护成本高 | ⭐⭐⭐ |

### **推荐方案: C (WGSL 自研)**

**理由**:
1. **架构一致性**: 复用 Phase 13 M13.3/M13.4 的 WGPU 基础设施
2. **跨平台**: DX12/Vulkan/Metal 通用，不依赖 CUDA
3. **渐进式**: 先实现 CPU 加速(rayon)，再优化 GPU kernel
4. **可控性**: 完全掌握优化空间，无黑盒依赖

**性能预期**:

- MSM GPU: 10-20x vs CPU (保守估计，CUDA 可达 50-100x)

- FFT GPU: 5-10x vs CPU

- 整体 prove: 5-10x (初期), 20x+ (优化后)

## 实现路线

### Phase 1: CPU 优化 (Week 1)

**目标**: 建立性能基线，优化 CPU 路径

1. **并行 MSM (rayon)**
   ```rust
   // src/gpu-executor/src/zk/msm_cpu.rs
   pub fn msm_parallel(scalars: &[Fr], points: &[G1Affine]) -> G1Projective {
       points.par_iter()
           .zip(scalars.par_iter())
           .map(|(point, scalar)| point.mul(scalar))
           .reduce(|| G1Projective::zero(), |a, b| a + b)
   }
   ```

2. **FFT 优化**
   - 使用 arkworks 内置的并行 FFT
   - 或实现 radix-2 并行 Cooley-Tukey

3. **Benchmark 基线**
   - 测试不同电路规模 (10K - 1M 约束)
   - 记录 MSM/FFT 占比

### Phase 2: GPU MSM 实现 (Week 2-3)

**算法: Pippenger Bucket Method**

```wgsl
// src/gpu-executor/src/zk/shaders/msm.wgsl

// Stage 1: 分桶 (Bucket Assignment)
@compute @workgroup_size(256)
fn msm_bucket_assign(
    @builtin(global_invocation_id) global_id: vec3<u32>,
) {
    let idx = global_id.x;
    if (idx >= params.n) { return; }
    
    let scalar = scalars[idx];
    let bucket_idx = (scalar >> params.window_start) & params.window_mask;
    
    // 原子增加 bucket 计数
    atomicAdd(&bucket_counts[bucket_idx], 1u);
}

// Stage 2: 桶内累加 (Bucket Accumulation)
@compute @workgroup_size(256)
fn msm_bucket_accumulate(
    @builtin(global_invocation_id) global_id: vec3<u32>,
) {
    let bucket_idx = global_id.x;
    if (bucket_idx >= params.num_buckets) { return; }
    
    var sum = G1_ZERO;
    let start = bucket_offsets[bucket_idx];
    let end = bucket_offsets[bucket_idx + 1];
    
    for (var i = start; i < end; i++) {
        let point_idx = bucket_indices[i];
        sum = ec_add(sum, points[point_idx]);
    }
    
    bucket_sums[bucket_idx] = sum;
}

// Stage 3: 桶间归约 (Bucket Reduction)
@compute @workgroup_size(256)
fn msm_bucket_reduce(
    @builtin(global_invocation_id) global_id: vec3<u32>,
) {
    // 计算 sum = Σ(bucket_idx * bucket_sum)
    // 使用倍增策略: result = 2*result + bucket_sum
}

```

**EC Point 算术 (BLS12-381)**

```wgsl
// 椭圆曲线点加法 (Projective 坐标)
fn ec_add(a: G1Projective, b: G1Projective) -> G1Projective {
    // Projective 坐标: (X, Y, Z) 表示 (X/Z, Y/Z)
    // 加法公式: https://www.hyperelliptic.org/EFD/g1p/auto-shortw-projective.html
    
    let t0 = a.x * b.x;  // t0 = X1*X2
    let t1 = a.y * b.y;  // t1 = Y1*Y2
    let t2 = a.z * b.z;  // t2 = Z1*Z2
    
    // ... (完整公式约 15 行)
    
    return G1Projective {
        x: x3,
        y: y3,
        z: z3,
    };
}

// 标量乘法 (Double-and-Add)
fn ec_mul(point: G1Affine, scalar: Fr) -> G1Projective {
    var result = G1_ZERO;
    var temp = point_to_projective(point);
    
    for (var i = 0u; i < 255u; i++) {
        if ((scalar >> i) & 1u == 1u) {
            result = ec_add(result, temp);
        }
        temp = ec_double(temp);
    }
    
    return result;
}

```

**性能优化**:

- Window size: 8-16 bits (平衡 bucket 数量和深度)

- Workgroup size: 256 (GPU 占用率优化)

- Memory coalescing: 对齐访问模式

### Phase 3: GPU FFT 实现 (Week 4)

**算法: Cooley-Tukey Radix-2 FFT**

```wgsl
// src/gpu-executor/src/zk/shaders/fft.wgsl

// Radix-2 DIT (Decimation In Time) FFT
@compute @workgroup_size(256)
fn fft_radix2(
    @builtin(global_invocation_id) global_id: vec3<u32>,
) {
    let idx = global_id.x;
    if (idx >= params.n / 2) { return; }
    
    let stage = params.current_stage;
    let block_size = 1u << (stage + 1);
    let block_id = idx / (block_size / 2);
    let in_block_idx = idx % (block_size / 2);
    
    let offset = block_id * block_size + in_block_idx;
    let pair_offset = offset + block_size / 2;
    
    // Butterfly 操作
    let a = data[offset];
    let b = data[pair_offset];
    let twiddle = twiddle_factors[in_block_idx << (log2_n - stage - 1)];
    
    let t = field_mul(b, twiddle);
    data[offset] = field_add(a, t);
    data[pair_offset] = field_sub(a, t);
}

// 有限域算术 (Fr field)
fn field_mul(a: Fr, b: Fr) -> Fr {
    // Montgomery 乘法
    // 参考: arkworks ff crate
}

fn field_add(a: Fr, b: Fr) -> Fr {
    // 模加法
}

```

**优化策略**:

- Shared memory: 减少全局内存访问

- Bank conflict 避免: 交错访问模式

- Bit-reversal: 预处理或后处理

### Phase 4: ZkProver 抽象层 (Week 5)

```rust
// src/gpu-executor/src/zk/mod.rs

pub trait ZkProver {
    type Circuit;
    type Proof;
    type ProvingKey;
    
    fn prove(
        &self,
        pk: &Self::ProvingKey,
        circuit: Self::Circuit,
    ) -> Result<Self::Proof, ExecError>;
    
    fn device_kind(&self) -> DeviceKind;
    fn stats(&self) -> ZkStats;
}

// CPU 实现 (基于 arkworks)
pub struct CpuZkProver {
    stats: Arc<Mutex<ZkStats>>,
}

impl ZkProver for CpuZkProver {
    type Circuit = impl ark_relations::r1cs::ConstraintSynthesizer<Fr>;
    type Proof = ark_groth16::Proof<Bls12_381>;
    type ProvingKey = ark_groth16::ProvingKey<Bls12_381>;
    
    fn prove(&self, pk: &Self::ProvingKey, circuit: Self::Circuit) -> Result<Self::Proof, ExecError> {
        let start = Instant::now();
        
        let proof = ark_groth16::Groth16::<Bls12_381>::prove(pk, circuit, &mut rng)?;
        
        let elapsed = start.elapsed();
        self.stats.lock().record_prove(elapsed);
        
        Ok(proof)
    }
}

// GPU 实现
pub struct GpuZkProver {
    device: wgpu::Device,
    queue: wgpu::Queue,
    msm_pipeline: wgpu::ComputePipeline,
    fft_pipeline: wgpu::ComputePipeline,
    stats: Arc<Mutex<ZkStats>>,
}

impl GpuZkProver {
    pub async fn new() -> Result<Self, ExecError> {
        // 初始化 WGPU
        // 加载 MSM/FFT shader
        // 创建 pipeline
    }
}

impl ZkProver for GpuZkProver {
    fn prove(&self, pk: &Self::ProvingKey, circuit: Self::Circuit) -> Result<Self::Proof, ExecError> {
        let start = Instant::now();
        
        // 1. 约束求解 (CPU)
        let cs = ConstraintSystem::new_ref();
        circuit.generate_constraints(cs.clone())?;
        
        // 2. R1CS → QAP (CPU)
        let (a, b, c, z) = cs.to_matrices()?;
        
        // 3. MSM (GPU)
        let proof_a = self.msm_gpu(&pk.a_query, &z)?;
        let proof_b = self.msm_gpu(&pk.b_query, &z)?;
        let proof_c = self.msm_gpu(&pk.c_query, &z)?;
        
        // 4. 组装证明
        let proof = Proof { a: proof_a, b: proof_b, c: proof_c };
        
        let elapsed = start.elapsed();
        self.stats.lock().record_prove(elapsed);
        
        Ok(proof)
    }
}

```

### Phase 5: Benchmark & 优化 (Week 6)

**测试矩阵**:

| 电路规模 | 约束数 | CPU 时间 | GPU 时间 | 加速比 | 目标 |
|---------|--------|----------|----------|--------|------|
| Small | 1K | 5ms | 2ms | 2.5x | - |
| Medium | 10K | 50ms | 10ms | 5x | ✅ |
| Large | 100K | 500ms | 25ms | 20x | ✅ |
| XLarge | 1M | 5s | 200ms | 25x | 🎯 |

**优化重点**:
1. MSM bucket 大小调优
2. FFT twiddle factor 预计算
3. CPU-GPU 数据传输重叠
4. 批量证明优化

## 集成路径

### vm-runtime 集成

```rust
// src/vm-runtime/src/zk.rs

pub struct ZkService {
    cpu_prover: CpuZkProver,
    #[cfg(feature = "zk-gpu")]
    gpu_prover: Option<GpuZkProver>,
    config: ZkConfig,
}

impl ZkService {
    pub fn prove<C: ConstraintSynthesizer<Fr>>(
        &self,
        pk: &ProvingKey<Bls12_381>,
        circuit: C,
    ) -> Result<Proof<Bls12_381>> {
        #[cfg(feature = "zk-gpu")]
        if let Some(gpu) = &self.gpu_prover {
            if self.should_use_gpu(circuit_size) {
                return gpu.prove(pk, circuit);
            }
        }
        
        self.cpu_prover.prove(pk, circuit)
    }
    
    fn should_use_gpu(&self, size: usize) -> bool {
        size >= self.config.gpu_threshold  // 默认: 10K 约束
    }
}

```

### Feature Gates

```toml

# Cargo.toml

[features]
default = ["cpu"]
cpu = []
zk-gpu = ["wgpu", "pollster", "bytemuck"]

```

## 风险与备选方案

### 风险

1. **WGSL 性能不及 CUDA** (可能只达到 5-10x vs 20x+ 目标)
   - **缓解**: 先实现功能验证，后续可选集成 sppark/bellman-cuda

2. **椭圆曲线算术复杂** (BLS12-381 有限域运算)
   - **缓解**: 参考 arkworks-rs 的 ff/ec 实现，移植到 WGSL

3. **内存带宽瓶颈** (大规模 MSM 需要大量点数据传输)
   - **缓解**: 使用 staging buffer + 流水线

### 备选方案

**Plan B: arkworks + sppark FFI**

如果 WGSL 性能不达标，回退到：
1. 保留 arkworks CPU 路径
2. 通过 FFI 调用 sppark CUDA MSM
3. Feature gate: `zk-cuda` (NVIDIA 专用)

**优点**: 50-100x 加速比 (Filecoin 实测数据)  
**缺点**: CUDA 依赖，跨平台性差

## 时间表

| 周次 | 任务 | 交付物 |
|-----|------|--------|
| Week 1 | CPU 优化 + 基线 | msm_cpu.rs, fft_cpu.rs, benchmark |
| Week 2 | GPU MSM 实现 | msm.wgsl, msm_gpu.rs |
| Week 3 | GPU MSM 优化 | Bucket method, 测试通过 |
| Week 4 | GPU FFT 实现 | fft.wgsl, fft_gpu.rs |
| Week 5 | ZkProver 抽象层 | trait + CPU/GPU 实现 |
| Week 6 | Benchmark + 优化 | 性能报告, ROADMAP 更新 |

## 成功标准

- ✅ CPU 优化: 5x+ vs naive 实现

- ✅ GPU MSM: 10x+ vs CPU parallel

- ✅ GPU FFT: 5x+ vs CPU parallel

- ✅ 整体 prove: 20x+ vs CPU baseline (100K+ 约束)

- ✅ 正确性: 100% 证明验证通过

- ✅ 集成: ZkService 统一接口

---

*文档版本: v0.1.0-draft*  
*作者: XujueKing*  
*日期: 2025-11-12*
