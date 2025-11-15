# Merkle Tree GPU Acceleration

**Phase 13 Milestone 13.4 (M13.4) - GPU-加速 Merkle 树构建**

## 概述

本模块提供 GPU 加速的 Merkle 树构建实现，使用 WGPU compute shader 并行计算哈希树，实现 5-20x 性能提升。

## 目标

- **性能目标**: 5-20x 加速比（相对 CPU 基线）

- **吞吐量**: 25-100K trees/sec（1K 叶子节点）

- **自动选择**: 根据树大小自动选择 CPU/GPU 路径

## 算法设计

### 核心算法

采用**自底向上层级并行构建**策略：

```

Layer 0 (叶子):  [L0, L1, L2, L3, L4, L5, L6, L7]  (8 nodes)
                  │   │   │   │   │   │   │   │
                  └─┬─┘   └─┬─┘   └─┬─┘   └─┬─┘
Layer 1:          [H01,    H23,    H45,    H67]    (4 nodes, GPU 并行)
                   │       │       │       │
                   └───┬───┘       └───┬───┘
Layer 2:              [H0123,         H4567]       (2 nodes, GPU 并行)
                        │               │
                        └───────┬───────┘
Layer 3 (root):              [ROOT]                (1 node)

```

### GPU 实现特点

1. **并行化**: 每层所有父节点并行计算
2. **SHA-256 复用**: 复用 M13.3 的 WGSL SHA-256 内核
3. **奇数节点处理**: 自动复制末尾节点
4. **内存优化**: 大端序直接映射，减少转换开销

## 性能数据

### CPU 基线 (CpuMerkleExecutor)

测试平台: Windows 11, AMD Ryzen (或同等配置)

| 树大小 | 平均时间 | 吞吐量 |
|-------|---------|--------|
| 1K 叶子 | 0.778 ms | 1,285 trees/sec |
| 4K 叶子 | 2.5 ms | 400 trees/sec |
| 16K 叶子 | 10.3 ms | 97 trees/sec |
| 64K 叶子 | 38.9 ms | 26 trees/sec |
| 256K 叶子 | 148.2 ms | 6.7 trees/sec |

**测试配置**: Release 优化, 10 次迭代取平均

### GPU 性能 (GpuMerkleExecutor)

状态: ✅ 编译通过, ✅ 初始化成功, ⏳ 性能测试待完成

**当前进展**:

- GPU 设备初始化成功

- WGSL shader 编译通过

- Compute pipeline 创建成功

- Buffer 映射机制需优化（轮询逻辑调试中）

**预期性能** (基于设计目标):

- 小树 (1K): ~5x 加速

- 中树 (16K): ~10x 加速

- 大树 (256K): ~20x 加速

## API 参考

### CpuMerkleExecutor

**CPU 串行实现**，适合小规模树 (< 1024 叶子)。

```rust
use gpu_executor::merkle::{CpuMerkleExecutor, MerkleExecutor, MerkleRequest};

let executor = CpuMerkleExecutor::new();
let leaves: Vec<[u8; 32]> = vec![
    [1u8; 32], [2u8; 32], [3u8; 32], [4u8; 32]
];

let request = MerkleRequest { leaves };
let result = executor.build_tree(request)?;

println!("Root: {:?}", result.root);
println!("Device: {:?}", result.device); // DeviceKind::Cpu

```

### GpuMerkleExecutor

**GPU 并行实现**，适合大规模树 (≥ 1024 叶子)。

```rust
use gpu_executor::merkle::{GpuMerkleExecutor, MerkleExecutor, MerkleRequest};

// 异步初始化
let executor = GpuMerkleExecutor::new().await?;

// 或同步初始化 (阻塞当前线程)
let executor = GpuMerkleExecutor::new_blocking()?;

let leaves: Vec<[u8; 32]> = (0..16384)
    .map(|i| {
        let mut leaf = [0u8; 32];
        leaf[..4].copy_from_slice(&i.to_be_bytes());
        leaf
    })
    .collect();

let request = MerkleRequest { leaves };
let result = executor.build_tree(request)?;

println!("Root: {:?}", result.root);
println!("Device: {:?}", result.device); // DeviceKind::Gpu

```

### MerkleResult

```rust
pub struct MerkleResult {
    pub root: [u8; 32],               // 树根哈希
    pub layers: Option<Vec<Vec<[u8; 32]>>>, // 所有层（可选）
    pub device: DeviceKind,           // 使用的设备
}

```

### MerkleStats

性能统计信息:

```rust
pub struct MerkleStats {
    pub cpu_builds: usize,       // CPU 构建次数
    pub gpu_builds: usize,       // GPU 构建次数
    pub last_build_ms: f64,      // 上次构建耗时(ms)
    pub avg_leaves: f64,         // 平均叶子节点数
}

// 获取统计
let stats = executor.stats();
println!("CPU builds: {}, GPU builds: {}", stats.cpu_builds, stats.gpu_builds);

```

## 架构设计

### 代码结构

```

src/gpu-executor/src/merkle/
├── mod.rs              # Trait 定义, CPU 实现, 6 单元测试
├── gpu.rs              # GPU executor (WGPU 27)
└── shaders/
    └── merkle_tree.wgsl  # Merkle compute shader (SHA-256)

```

### MerkleExecutor Trait

统一的 Merkle 树执行器接口:

```rust
pub trait MerkleExecutor {
    fn build_tree(&self, request: MerkleRequest) -> Result<MerkleResult, ExecError>;
    fn stats(&self) -> MerkleStats;
    fn device_kind(&self) -> DeviceKind;
}

```

### Feature Gates

```toml
[features]
wgpu-backend-dx12 = ["wgpu", "pollster", "bytemuck", "gpu-allocator", "wgpu/dx12", "wgpu/wgsl"]

```

## 实现细节

### WGSL Shader 设计

**文件**: `merkle/shaders/merkle_tree.wgsl`

**核心函数**:

```wgsl
// SHA-256 双哈希 (Merkle 父节点计算)
fn sha256_double_hash(left: array<u32, 8>, right: array<u32, 8>) -> array<u32, 8> {
    // 1. 拼接 left || right (64 字节)
    // 2. SHA-256(left || right)
    // 3. 返回哈希值 (32 字节 = 8 × u32)
}

// 主计算内核
@compute @workgroup_size(64)
fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let pair_index = global_id.x;
    // 处理一对叶子节点，计算父节点
}

```

**Buffer 绑定**:

- `@group(0) @binding(0)` - 输入节点 (storage, read-only)

- `@group(0) @binding(1)` - 输出节点 (storage, write)

- `@group(0) @binding(2)` - 参数 (node_count, next_node_count)

### GPU 执行流程

```rust
// 1. 初始化 WGPU
let instance = wgpu::Instance::new(...);
let adapter = instance.request_adapter(...).await?;
let (device, queue) = adapter.request_device(...).await?;

// 2. 加载 shader 并创建 pipeline
let shader = device.create_shader_module(...);
let pipeline = device.create_compute_pipeline(...);

// 3. 逐层构建树
while current_node_count > 1 {
    // 3.1 创建 buffers
    let input_buffer = device.create_buffer(...);
    let output_buffer = device.create_buffer(...);
    
    // 3.2 写入数据
    queue.write_buffer(&input_buffer, 0, current_layer_data);
    
    // 3.3 执行 compute shader
    let mut encoder = device.create_command_encoder(...);
    let mut compute_pass = encoder.begin_compute_pass(...);
    compute_pass.set_pipeline(&pipeline);
    compute_pass.set_bind_group(0, &bind_group, &[]);
    compute_pass.dispatch_workgroups(workgroup_count, 1, 1);
    
    // 3.4 读回结果
    queue.submit([encoder.finish()]);
    let staging_buffer = device.create_buffer(...);
    buffer.map_async(MapMode::Read, ...);
    device.poll(PollType::Poll);
    
    // 3.5 准备下一层
    current_node_count = (current_node_count + 1) / 2;
}

```

## 单元测试

**测试覆盖**: 6 个测试用例，全部通过 ✅

```bash
$ cargo test -p gpu-executor merkle --lib --release
running 6 tests
test merkle::tests::test_odd_number_leaves ... ok
test merkle::tests::test_power_of_two_leaves ... ok
test merkle::tests::test_deterministic_root ... ok
test merkle::tests::test_single_leaf ... ok
test merkle::tests::test_stats_tracking ... ok
test merkle::tests::test_two_leaves ... ok

test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured

```

**测试场景**:
1. 单叶子节点
2. 两个叶子节点
3. 2 的幂次方节点 (8 叶子)
4. 奇数节点 (5 叶子)
5. 确定性根哈希验证
6. 统计信息追踪

## Benchmark

**工具**: `examples/merkle_bench.rs` 和 `examples/merkle_bench_simple.rs`

**运行方式**:

```bash

# 完整 benchmark

cargo run -p gpu-executor --example merkle_bench --features wgpu-backend-dx12 --release

# 简化版本

cargo run -p gpu-executor --example merkle_bench_simple --features wgpu-backend-dx12 --release

# 使用脚本

powershell -ExecutionPolicy Bypass -File run_merkle_bench.ps1

```

**测试配置**:

- 5 种树大小: 1K, 4K, 16K, 64K, 256K 叶子

- 每种大小 10 次迭代

- Release 优化编译

- 预热运行

## 已知限制

1. **内存限制**: 当前实现将整棵树加载到 GPU 内存，对于超大树 (> 1M 叶子) 可能需要流式处理
2. **异步优化**: 当前使用阻塞式 buffer 映射，可优化为异步管道
3. **错误处理**: GPU 设备故障时的错误信息需要改进
4. **平台支持**: 当前仅在 Windows DX12 上测试，需验证 Vulkan/Metal 后端

## 未来工作

### 短期 (Phase 13)

1. **M13.5 ZK 加速**: 集成 bellman-cuda/halo2 GPU 后端
2. **M13.6 指标系统**: Prometheus metrics + Grafana dashboard
3. **M13.7 一致性验证**: CPU/GPU 结果一致性检查

### 长期优化

1. **流式处理**: 支持超大树 (> 10M 叶子)
2. **Merkle 证明优化**: 仅存储必要层以生成证明
3. **多 GPU 支持**: 跨 GPU 负载均衡
4. **自适应阈值**: 根据实时性能动态调整 CPU/GPU 切换阈值

## 技术栈

- **Rust**: 1.70+

- **WGPU**: 27.0.1 (跨平台 GPU compute API)

- **WGSL**: WebGPU Shading Language

- **SHA-256**: 复用 M13.3 crypto 模块的 WGSL 实现

- **Backend**: DX12 (Windows), Vulkan (Linux), Metal (macOS)

## 相关文档

- [GPU-CRYPTO-ACCELERATION.md](./GPU-CRYPTO-ACCELERATION.md) - M13.3 密码学加速 (SHA-256/Keccak256)

- [ARCH-CPU-GPU-HYBRID.md](./ARCH-CPU-GPU-HYBRID.md) - Hybrid 混合调度架构

- [AUTO-TUNER.md](./AUTO-TUNER.md) - 自动调优系统

- [ROADMAP.md](../ROADMAP.md) - Phase 13 完整路线图

## 开发进度

**M13.4 状态**: ⏳ 80% 完成

- ✅ CPU 实现 (6/6 测试通过)

- ✅ GPU shader (WGSL 编译通过)

- ✅ GPU executor (初始化成功)

- ⏳ GPU benchmark (轮询逻辑调试中)

- ⬜ MerkleService 集成

- ⬜ 文档完善

**预计完成时间**: 2025-11-12

---

*文档版本: v0.1.0-draft*  
*最后更新: 2025-11-12*  
*作者: XujueKing*
