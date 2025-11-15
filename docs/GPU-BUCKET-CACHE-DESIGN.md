# GPU Executor 多任务批次优化设计

## 概述

本文档描述基于**尺寸分桶**的 pipeline 和 bind group 缓存优化方案，用于减少多任务批次执行时的对象创建开销。

---

## 1. 尺寸分桶策略

### 分桶算法

使用 2 的幂次方分桶，最小桶为 16K：

```rust
fn size_bucket(n: usize) -> usize {
    const MIN_BUCKET: usize = 16_384; // 16K
    if n <= MIN_BUCKET {
        return MIN_BUCKET;
    }
    // Round up to next power of 2
    let bits = (n - 1).leading_zeros();
    1usize << (usize::BITS - bits)
}

```

**桶划分示例**：

- 0-16K → 16K 桶

- 16K-32K → 32K 桶

- 32K-64K → 64K 桶

- 64K-128K → 128K 桶

- 128K-256K → 256K 桶

- 256K-512K → 512K 桶

- 512K-1M → 1M 桶

- 1M-2M → 2M 桶

**优势**：

- 减少桶数量（对数级而非线性）

- 同桶内任务可共享 pipeline 和 bind group layout

- 缓存命中率高（常见规模集中在少数桶）

---

## 2. Pipeline 缓存结构

### BucketCache 实现

```rust
struct BucketCache {
    cache: HashMap<usize, Arc<BucketResources>>,
    device: Arc<Device>,
}

struct BucketResources {
    pipeline: ComputePipeline,
    bind_group_layout: BindGroupLayout,
}

```

**关键特性**：

- 每个桶创建一次 pipeline 和 bind group layout

- 使用 Arc 共享所有权，避免重复编译

- 延迟创建（lazy initialization）：首次使用桶时编译

### 创建流程

```rust
fn get_or_create(&mut self, bucket_size: usize, shader_code: &str) -> Arc<BucketResources> {
    if let Some(res) = self.cache.get(&bucket_size) {
        return res.clone(); // 缓存命中
    }
    
    // 缓存未命中：编译 shader + 创建 pipeline/layout
    let shader = self.device.create_shader_module(...);
    let bind_group_layout = self.device.create_bind_group_layout(...);
    let pipeline_layout = self.device.create_pipeline_layout(...);
    let pipeline = self.device.create_compute_pipeline(...);
    
    let resources = Arc::new(BucketResources { pipeline, bind_group_layout });
    self.cache.insert(bucket_size, resources.clone());
    resources
}

```

---

## 3. Bind Group 重用策略

### 单任务批次（已实现）

使用 `va_cache: HashMap<usize, CachedVectorAdd>`：

- Key：精确尺寸（如 100,000）

- Value：持久化 a/b/out/staging buffers + bind_group

- 适用：相同规模重复调用的稳态场景

### 多任务批次（新增优化）

**挑战**：不同任务可能有不同尺寸，无法直接共享 buffers。

**方案 A：按桶分组重用 pipeline**

```rust
// 1. 按桶分组任务
let mut buckets: HashMap<usize, Vec<&Task>> = HashMap::new();
for task in &batch.tasks {
    let n = task.payload.0.len();
    let bucket = size_bucket(n);
    buckets.entry(bucket).or_default().push(task);
}

// 2. 对每个桶使用对应的 pipeline
for (bucket_size, tasks) in buckets {
    let resources = self.bucket_cache.get_or_create(bucket_size, VECTOR_ADD_SHADER);
    cpass.set_pipeline(&resources.pipeline);
    
    for task in tasks {
        // 为每个任务创建 buffers（仍从 pool 获取）
        // 创建 bind_group（使用缓存的 layout，但新 buffers）
        let bind_group = self.device.create_bind_group(...);
        cpass.set_bind_group(0, &bind_group, &[]);
        cpass.dispatch_workgroups(...);
    }
}

```

**收益**：

- 减少 pipeline 切换次数（桶数量 << 任务数量）

- 共享 bind group layout 降低验证开销

- Pool 仍复用 buffers

**方案 B：同桶同尺寸任务完全重用 bind group**

```rust
// 进一步细分：同桶内按精确尺寸分组
let mut bucket_size_groups: HashMap<(usize, usize), Vec<&Task>> = HashMap::new();
for task in &batch.tasks {
    let n = task.payload.0.len();
    let bucket = size_bucket(n);
    bucket_size_groups.entry((bucket, n)).or_default().push(task);
}

for ((bucket, exact_size), tasks) in bucket_size_groups {
    let resources = self.bucket_cache.get_or_create(bucket, VECTOR_ADD_SHADER);
    cpass.set_pipeline(&resources.pipeline);
    
    if tasks.len() == 1 {
        // 单任务：从 va_cache 重用
        ...
    } else {
        // 多任务但同尺寸：创建一组 buffers 循环使用
        // 需要同步机制确保前一任务 GPU 完成再重用
        // 或为每个任务创建独立 bind_group（更安全）
    }
}

```

**权衡**：

- 完全重用需要更复杂的生命周期管理

- 简化方案：仅重用 pipeline/layout，每任务独立 bind_group

---

## 4. 性能预期

### 基线（当前实现）

- 单任务批次：✅ va_cache 完全重用 buffers + bind_group

- 多任务批次：❌ 每任务重新创建 bind_group（但 pipeline 共享）

### 优化后（分桶缓存）

**小批次（2-10 任务，不同尺寸）**：

- Pipeline 创建：从 N 次 → 桶数量次（通常 1-3 个桶）

- Bind group layout 验证：降低（共享 layout）

- 预期提升：5-15%（减少驱动验证开销）

**大批次（100+ 任务，尺寸分散）**：

- Pipeline 切换：从无序 → 按桶批量

- 预期提升：10-25%（更好的缓存局部性）

**稳态场景（重复调用相似规模批次）**：

- Pipeline 命中率：接近 100%

- 预期提升：20-40%（完全避免编译开销）

### 测试验证

运行以下基准测试对比：

```powershell

# 基线：无分桶缓存

cargo bench --bench gpu_multi_task_baseline

# 优化：启用分桶缓存

cargo bench --bench gpu_multi_task_bucketed

# 对比分析

scripts/analyze-bucket-cache-perf.ps1

```

---

## 5. 实现步骤

### Phase 1: 基础设施（已完成）

- [x] `size_bucket()` 函数

- [x] `BucketCache` 结构体

- [x] `get_or_create()` 方法

### Phase 2: 集成到 execute_vector_add

- [ ] 在多任务批次分支中启用分桶

- [ ] 按桶分组并设置对应 pipeline

- [ ] 保持现有 pool 逻辑不变

### Phase 3: 测试与验证

- [ ] 创建多任务批次基准测试

- [ ] 对比启用/禁用分桶缓存的性能

- [ ] 验证正确性（输出一致性）

### Phase 4: 调优

- [ ] 动态调整桶大小策略（监控命中率）

- [ ] 考虑 LRU 淘汰（限制缓存总量）

- [ ] 性能计数器（pipeline 创建次数、命中率）

---

## 6. 安全性与正确性

### Bind Group 生命周期

**问题**：Bind group 引用 buffers，buffers 归还 pool 后可能被重用。

**解决**：
1. **方案 A**（当前）：Pending 跟踪所有 buffers，submit 后才归还 pool
2. **方案 B**（未来）：Bind group 持有 Arc<Buffer>，自动生命周期管理

### Pipeline 安全性

Pipeline 不直接引用数据，仅需编译一次，Arc 共享安全。

### 并发考虑

单线程 executor：无并发问题。  
多线程版本：BucketCache 需要 `Arc<Mutex<...>>` 或 `RwLock`。

---

## 7. 监控与诊断

### 添加统计信息

```rust
impl WgpuExecutor {
    pub fn bucket_cache_stats(&self) -> (usize, Vec<usize>) {
        self.bucket_cache.stats()
    }
    
    pub fn print_stats(&self) {
        let (num_buckets, bucket_sizes) = self.bucket_cache_stats();
        eprintln!("[gpu] Bucket cache: {} buckets cached: {:?}", num_buckets, bucket_sizes);
        eprintln!("[gpu] va_cache: {} exact-size entries", self.va_cache.len());
        eprintln!("[gpu] Buffer pool hit rate: {:.2}%", self.buffer_pool.hit_rate() * 100.0);
    }
}

```

### 日志示例

```

[gpu] Bucket cache: 3 buckets cached: [16384, 65536, 262144]
[gpu] va_cache: 5 exact-size entries
[gpu] Buffer pool hit rate: 78.50%

```

---

## 8. 后续优化方向

1. **动态桶大小**：根据实际负载调整桶边界
2. **预热（Warm-up）**：启动时预创建常用桶的 pipeline
3. **多 shader 支持**：扩展到矩阵乘法、归约等操作
4. **异步编译**：Pipeline 创建放入后台线程
5. **持久化缓存**：序列化编译结果，跨进程复用

---

## 9. 参考资料

- [wgpu Pipeline Caching](https://github.com/gfx-rs/wgpu/wiki/Pipeline-Caching)

- [Vulkan Best Practices - Pipeline Caching](https://arm-software.github.io/vulkan-sdk/best_practices.html)

- [DirectX 12 Pipeline State Objects](https://learn.microsoft.com/en-us/windows/win32/direct3d12/pipelines-and-shaders)

---

生成时间：2025-11-12
作者：KING XU / Rainbow Haruko
