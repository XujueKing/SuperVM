# GPU 密码学批量加速

**Phase 13 Milestone M13.3: Cryptographic Batch Acceleration**

## 概述

GPU 密码学批量加速模块提供了高性能的批量哈希计算能力，使用 WGPU 计算着色器在 GPU 上并行处理大量密码学操作。

### 性能目标

- **SHA256**: 10x+ 吞吐量提升（相比纯 CPU）

- **Keccak256**: 10x+ 吞吐量提升

- **批量阈值**: 批量大小 ≥ 10 时启用 GPU 加速

### 架构特点

1. **统一接口**: `CryptoExecutor` trait 提供 CPU/GPU 统一抽象
2. **自动降级**: GPU 不可用或失败时自动回退到 CPU
3. **灵活配置**: 可调整 GPU 批量阈值和后端选择策略
4. **零外部依赖**: 默认 CPU 模式不依赖 GPU 硬件

---

## 快速开始

### 基础使用（vm-runtime 集成）

```rust
use vm_runtime::crypto::CryptoService;

// 创建密码学服务
let mut service = CryptoService::new();

// 批量计算 SHA256（自动选择 CPU/GPU）
let messages = vec![
    b"hello".to_vec(),
    b"world".to_vec(),
    b"test".to_vec(),
];

let hashes = service.sha256_batch_accelerated(messages)?;
for hash in hashes {
    println!("SHA256: {}", hex::encode(hash));
}

```

### 启用 GPU 加速

```rust
// 异步初始化 GPU 执行器
service.init_gpu().await?;

// 批量计算（>=10 条消息时自动使用 GPU）
let messages: Vec<Vec<u8>> = (0..100)
    .map(|i| format!("message_{}", i).into_bytes())
    .collect();

let hashes = service.sha256_batch_accelerated(messages)?;
// GPU 自动加速，10x+ 性能提升

```

### 调整配置

```rust
// 设置 GPU 批量阈值（默认 10）
service.set_gpu_batch_threshold(50);

// 检查 GPU 是否可用
if service.is_gpu_available() {
    println!("GPU acceleration enabled");
}

```

---

## 架构设计

### 模块结构

```

src/gpu-executor/src/crypto/
├── mod.rs              # CryptoExecutor trait 定义
├── cpu.rs              # CPU 参考实现（rayon 并行）
├── gpu.rs              # GPU 实现（WGPU）
└── shaders/
    ├── sha256.wgsl     # SHA256 计算着色器
    └── keccak256.wgsl  # Keccak256 计算着色器

```

### CryptoExecutor Trait

```rust
pub trait CryptoExecutor {
    /// 批量计算哈希
    fn hash_batch(
        &mut self,
        request: HashBatchRequest,
    ) -> Result<(HashBatchResult, CryptoStats), ExecError>;

    /// 批量验证签名
    fn verify_signature_batch(
        &mut self,
        request: SignatureVerifyBatchRequest,
    ) -> Result<(SignatureVerifyBatchResult, CryptoStats), ExecError>;

    fn is_available(&self) -> bool;
    fn device_kind(&self) -> DeviceKind;
}

```

### 数据结构

```rust
/// 哈希算法
pub enum HashAlgorithm {
    Sha256,
    Keccak256,
}

/// 批量哈希请求
pub struct HashBatchRequest {
    pub inputs: Vec<Vec<u8>>,
    pub algorithm: HashAlgorithm,
}

/// 批量哈希结果
pub struct HashBatchResult {
    pub hashes: Vec<[u8; 32]>,
}

/// 执行统计
pub struct CryptoStats {
    pub batch_size: usize,
    pub elapsed_us: u64,
}

```

---

## GPU 实现细节

### SHA256 WGSL 着色器

- **算法**: FIPS 180-4 标准 SHA-256

- **并行度**: 每个工作组处理一条消息

- **块大小**: 512 位（64 字节）

- **输出**: 256 位（32 字节）哈希值

**关键优化**:
1. 预计算轮常量 K[64]
2. 大端序转换在 GPU 侧完成
3. 消息填充自动处理

### Keccak256 WGSL 着色器

- **算法**: Keccak-f[1600] 置换

- **速率**: 1088 位（136 字节）

- **容量**: 512 位

- **轮数**: 24 轮

**关键优化**:
1. 64位运算用 `vec2<u32>` 模拟
2. θ/ρ/π/χ/ι 五步置换优化
3. 吸收-挤出模式高效实现

### GPU 缓冲区布局

```

输入缓冲区:
  [len_u32, data_u32...] × batch_size
  每条消息: 4字节长度 + 数据（4字节对齐）

输出缓冲区:
  [hash_u32 × 8] × batch_size
  每个哈希: 32字节（8个 u32，大端序）

配置缓冲区:
  [batch_size: u32, max_msg_len: u32]

```

---

## 性能数据

### CPU 基准（参考）

| 算法 | 批量大小 | 消息大小 | 吞吐量 (MH/s) |
|------|---------|---------|--------------|
| SHA256 | 1 | 32B | 0.009 |
| SHA256 | 100 | 128B | 1.27 |
| SHA256 | 1000 | 256B | 1.52 |
| Keccak256 | 1 | 32B | 1.0 |
| Keccak256 | 100 | 128B | 1.19 |
| Keccak256 | 1000 | 256B | 1.84 |

### GPU 加速（目标）

| 算法 | 批量大小 | 预期加速比 | 预期吞吐量 (MH/s) |
|------|---------|-----------|------------------|
| SHA256 | 100 | 10-15x | 12-19 |
| SHA256 | 1000 | 15-20x | 23-30 |
| Keccak256 | 100 | 10-15x | 12-18 |
| Keccak256 | 1000 | 15-20x | 27-37 |

*注: 实际性能取决于 GPU 硬件*

### 性能测试工具

```bash

# CPU 基准测试

cargo run --example crypto_batch_bench --release

# GPU 基准测试（DX12 后端）

cargo run --example crypto_batch_bench --release --features wgpu-backend-dx12

# 快速验证（正确性测试）

cargo run --example crypto_validate --release --features wgpu-backend-dx12

```

---

## API 参考

### vm-runtime 集成

#### CryptoService

主要密码学服务类，提供 CPU/GPU 混合加速。

```rust
impl CryptoService {
    /// 创建新服务
    pub fn new() -> Self;

    /// 初始化 GPU 执行器（异步）
    pub async fn init_gpu(&mut self) -> Result<()>;

    /// 批量 SHA256（自动加速）
    pub fn sha256_batch_accelerated(
        &mut self,
        messages: Vec<Vec<u8>>
    ) -> Result<Vec<[u8; 32]>>;

    /// 批量 Keccak256（自动加速）
    pub fn keccak256_batch_accelerated(
        &mut self,
        messages: Vec<Vec<u8>>
    ) -> Result<Vec<[u8; 32]>>;

    /// 设置 GPU 批量阈值
    pub fn set_gpu_batch_threshold(&mut self, threshold: usize);

    /// 检查 GPU 可用性
    pub fn is_gpu_available(&self) -> bool;
}

```

### gpu-executor 直接使用

#### CPU 执行器

```rust
use gpu_executor::crypto::*;

let mut cpu_exec = cpu::CpuCryptoExecutor::new();

let request = HashBatchRequest {
    inputs: vec![b"test".to_vec()],
    algorithm: HashAlgorithm::Sha256,
};

let (result, stats) = cpu_exec.hash_batch(request)?;
println!("CPU time: {} μs", stats.elapsed_us);

```

#### GPU 执行器

```rust
use gpu_executor::crypto::*;

let mut gpu_exec = gpu::GpuCryptoExecutor::new().await?;

let request = HashBatchRequest {
    inputs: vec![b"test".to_vec()],
    algorithm: HashAlgorithm::Sha256,
};

let (result, stats) = gpu_exec.hash_batch(request)?;
println!("GPU time: {} μs", stats.elapsed_us);

```

---

## 特性开关

### Cargo Features

```toml
[dependencies]
gpu-executor = { path = "../gpu-executor", optional = true, features = ["cpu"] }

[features]

# 启用混合执行器

hybrid-exec = ["dep:gpu-executor"]

# 启用 GPU 后端（需要 GPU 硬件）

hybrid-gpu-wgpu = ["hybrid-exec", "gpu-executor/wgpu-backend"]
hybrid-gpu-wgpu-dx12 = ["hybrid-exec", "gpu-executor/wgpu-backend-dx12"]

```

### 编译选项

```bash

# 仅 CPU（默认）

cargo build

# CPU + GPU 混合（GLES 后端）

cargo build --features hybrid-exec,hybrid-gpu-wgpu

# CPU + GPU 混合（DX12 后端，Windows 推荐）

cargo build --features hybrid-exec,hybrid-gpu-wgpu-dx12

```

---

## 限制与已知问题

### 当前限制

1. **签名验证**: GPU 签名验证未实现，使用 CPU fallback
2. **消息大小**: 单条消息最大 512 字节（SHA256）/ 256 字节（Keccak256）
3. **批量大小**: 建议 ≥10 条消息以充分发挥 GPU 优势

### 兼容性

- **支持平台**: Windows (DX12), Linux (Vulkan), macOS (Metal)

- **WGPU 版本**: 27.0+

- **最低 GPU**: 支持计算着色器的任何 GPU

### 错误处理

所有 GPU 错误自动降级到 CPU，保证高可用性：

```rust
// GPU 初始化失败 -> 继续使用 CPU
service.init_gpu().await.ok();

// GPU 执行失败 -> 自动回退 CPU
let hashes = service.sha256_batch_accelerated(messages)?;
// 无论 GPU 是否可用，都能正常工作

```

---

## 开发指南

### 添加新哈希算法

1. 在 `HashAlgorithm` 枚举添加变体
2. 实现 CPU 版本（`cpu.rs`）
3. 编写 WGSL 着色器（`shaders/`）
4. 在 `gpu.rs` 添加管线创建
5. 更新测试和基准

### 调试技巧

```bash

# 启用 WGPU 日志

export RUST_LOG=wgpu=debug

# 强制使用 CPU（测试降级）

# 不初始化 GPU 或设置高阈值

service.set_gpu_batch_threshold(999999);

# 验证正确性

cargo test --features hybrid-exec

```

### 性能分析

```bash

# 使用 Prometheus 指标（如果启用）

cargo run --example crypto_batch_bench --release --features metrics

# GPU 探查器（Windows）

# 使用 PIX 或 RenderDoc 分析 GPU 性能

```

---

## 未来改进

### 短期目标

- [ ] 实现 GPU 签名验证（ECDSA/Ed25519）

- [ ] 支持更大消息（流式处理）

- [ ] 添加 Prometheus 指标导出

### 长期目标

- [ ] 自适应批量阈值（根据硬件动态调整）

- [ ] 多 GPU 支持（数据并行）

- [ ] 更多哈希算法（BLAKE2, SHA3-512 等）

---

## 参考资料

### 标准与规范

- [FIPS 180-4: SHA-256](https://nvlpubs.nist.gov/nistpubs/FIPS/NIST.FIPS.180-4.pdf)

- [Keccak/SHA-3](https://keccak.team/keccak.html)

- [WGSL Specification](https://www.w3.org/TR/WGSL/)

### 实现参考

- [sha2 crate](https://docs.rs/sha2/)

- [tiny-keccak crate](https://docs.rs/tiny-keccak/)

- [wgpu documentation](https://docs.rs/wgpu/)

---

**版本**: Phase 13 M13.3  
**最后更新**: 2025-11-12  
**作者**: SuperVM Development Team
