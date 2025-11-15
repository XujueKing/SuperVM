# Phase 14: 原生 SPIR-V 与完整 BLS12-381 实现

**阶段**: Phase 14 - Native SPIR-V & Complete BLS12-381 Implementation  
**开始日期**: 2025-11-13  
**预估完成**: 2025-11-27 (2 周)  
**状态**: 📋 规划中  
**依赖**: Phase 13 完成 ✅

---

## 1. 背景与动机

### 1.1 Phase 13 遗留问题

在 Phase 13 M13.9 验证 Vulkan 后端时,发现 **Naga 27.0.3 不支持 u64 类型**:

```wgsl
let a = u64(input[idx]);  // ❌ Naga validation 失败
// error: naga::ir::Expression - Unable to cast

```

**根本原因**:

- WGSL 规范不支持 64 位整数 (u64/i64)

- Naga v0.11 (2023-01) 起移除非 32 位整数支持

- 限制在 Naga 编译器层,而非 GPU 硬件或 SPIR-V 标准

**影响**:

- 无法用 WGSL 实现完整的 BLS12-381 field operations (需要 381-bit = 6×u64)

- 阻塞 ZK 证明生成的核心计算路径

### 1.2 技术方案选择

| 方案 | 路径 | 优势 | 劣势 | 时间 |
|------|------|------|------|------|
| **A: 原生 SPIR-V** ⭐ | GLSL → glslang → SPIR-V | ✅ 完整 u64<br>✅ 高性能<br>✅ 标准方案 | ⚠️ 额外工具链 | 1-2 周 |
| B: U32 模拟 | 双 u32 模拟 u64 | ✅ 纯 wgpu | ❌ 复杂<br>❌ 性能损失 20-30% | 3-5 天 |
| C: 等待上游 | 等待 naga 支持 | ✅ 零成本 | ❌ 时间未知(数月) | 未知 |

**决策**: 采用 **方案 A - 原生 SPIR-V**

---

## 2. Phase 14 目标

### 2.1 核心目标

1. **完整 u64 支持**: 绕过 Naga 限制,使用原生 SPIR-V
2. **BLS12-381 Field Operations**: 完整实现 381-bit 有限域算术
3. **跨平台验证**: Vulkan (Linux/Windows), Metal (macOS), DX12 (Windows)
4. **性能基准**: GPU 实现达到或超过 CPU arkworks-rs 性能

### 2.2 非目标

- ❌ 不修改 wgpu/naga (依赖上游)

- ❌ 不实现完整 ZK prover (留待后续)

- ❌ 不优化内存布局 (先保证正确性)

---

## 3. 技术架构

### 3.1 整体架构

```

┌─────────────────────────────────────────────────────────┐
│          Phase 14: Native SPIR-V Pipeline               │
└─────────────────────────────────────────────────────────┘
                        │
        ┌───────────────┼───────────────┐
        │               │               │
        ▼               ▼               ▼
┌──────────────┐ ┌─────────────┐ ┌────────────────┐
│ GLSL Shaders │ │  shaderc    │ │ SPIR-V Binary  │
│              │ │  Compiler   │ │   Modules      │
│ • field.comp │ │             │ │                │
│ • mont.comp  │ │ GLSL → SPV  │ │ OpInt64        │
│ • curve.comp │ │             │ │ u64 ops        │
└──────────────┘ └─────────────┘ └────────────────┘
        │               │               │
        └───────────────┼───────────────┘
                        ▼
┌─────────────────────────────────────────────────────────┐
│  wgpu::create_shader_module_spirv()                     │
│  • 直接加载 SPIR-V 二进制                                 │
│  • 绕过 naga 验证                                        │
│  • unsafe API (需要验证 SPIR-V 正确性)                   │
└─────────────────────────────────────────────────────────┘
                        │
        ┌───────────────┼───────────────┐
        │               │               │
        ▼               ▼               ▼
┌──────────────┐ ┌─────────────┐ ┌────────────────┐
│   Vulkan     │ │    Metal    │ │     DX12       │
│  (Linux/Win) │ │   (macOS)   │ │   (Windows)    │
│              │ │             │ │                │
│ Native u64   │ │ Native u64  │ │ Native u64     │
└──────────────┘ └─────────────┘ └────────────────┘

```

### 3.2 核心组件

#### 3.2.1 GLSL Shaders (src/gpu-executor/shaders/)

**bls12_381_field.comp** - BLS12-381 有限域运算

```glsl
#version 450
#extension GL_ARB_gpu_shader_int64 : enable
#extension GL_EXT_shader_explicit_arithmetic_types_int64 : enable

layout(local_size_x = 256) in;

// BLS12-381 381-bit 模数 (6x u64)
const uint64_t BLS_MODULUS[6] = uint64_t[](
    0x00000001ffffffffUL,   // limb 0 (least significant)
    0xac96341c4ffffffcUL,
    0x36fc7695920927b6UL,
    0xf3b4c437e8c64b44UL,
    0x2b7a3f1c7ced5d8aUL,
    0x1a0111ea397fe69aUL    // limb 5 (most significant)
);

// Montgomery R = 2^384 mod p (用于 Montgomery 形式)
const uint64_t MONTGOMERY_R[6] = uint64_t[](
    0x760900000002fffdUL,
    0xebf4000bc40c0002UL,
    0x5f48985753c758baUL,
    0x77ce585370525745UL,
    0x5c071a97a256ec6dUL,
    0x15f65ec3fa80e493UL
);

// Montgomery R^2 mod p
const uint64_t MONTGOMERY_R2[6] = uint64_t[](
    0xf4df1f341c341746UL,
    0x0a76e6a609d104f1UL,
    0x8de5476c4c95b6d5UL,
    0x67eb88a9939d83c0UL,
    0x9a793e85b519952dUL,
    0x11988fe592cae3aaUL
);

// Montgomery INV = -p^{-1} mod 2^64
const uint64_t MONTGOMERY_INV = 0xfffffffeffffffffUL;

// Buffer bindings
layout(set = 0, binding = 0) buffer InputA {
    uint64_t data[];
} input_a;

layout(set = 0, binding = 1) buffer InputB {
    uint64_t data[];
} input_b;

layout(set = 0, binding = 2) buffer Output {
    uint64_t data[];
} output;

// 381-bit 加法 (6x u64)
void field_add(out uint64_t result[6], uint64_t a[6], uint64_t b[6]) {
    uint64_t carry = 0;
    for (int i = 0; i < 6; i++) {
        uint64_t sum = a[i] + b[i] + carry;
        carry = (sum < a[i]) ? 1 : 0;
        result[i] = sum;
    }
    
    // Reduce if result >= modulus
    bool needs_reduce = false;
    for (int i = 5; i >= 0; i--) {
        if (result[i] > BLS_MODULUS[i]) {
            needs_reduce = true;
            break;
        } else if (result[i] < BLS_MODULUS[i]) {
            break;
        }
    }
    
    if (needs_reduce) {
        carry = 0;
        for (int i = 0; i < 6; i++) {
            uint64_t diff = result[i] - BLS_MODULUS[i] - carry;
            carry = (result[i] < BLS_MODULUS[i] + carry) ? 1 : 0;
            result[i] = diff;
        }
    }
}

// 381-bit 减法 (6x u64)
void field_sub(out uint64_t result[6], uint64_t a[6], uint64_t b[6]) {
    uint64_t borrow = 0;
    for (int i = 0; i < 6; i++) {
        uint64_t diff = a[i] - b[i] - borrow;
        borrow = (a[i] < b[i] + borrow) ? 1 : 0;
        result[i] = diff;
    }
    
    // Add modulus if result is negative
    if (borrow != 0) {
        uint64_t carry = 0;
        for (int i = 0; i < 6; i++) {
            uint64_t sum = result[i] + BLS_MODULUS[i] + carry;
            carry = (sum < result[i]) ? 1 : 0;
            result[i] = sum;
        }
    }
}

// Montgomery 乘法: result = (a * b * R^{-1}) mod p
void montgomery_mul(out uint64_t result[6], uint64_t a[6], uint64_t b[6]) {
    uint64_t temp[12] = uint64_t[12](0,0,0,0,0,0,0,0,0,0,0,0);
    
    // 1. Multiplication: temp = a * b (12 limbs)
    for (int i = 0; i < 6; i++) {
        uint64_t carry = 0;
        for (int j = 0; j < 6; j++) {
            // 128-bit multiplication simulation
            uint64_t a_lo = a[i] & 0xFFFFFFFFUL;
            uint64_t a_hi = a[i] >> 32;
            uint64_t b_lo = b[j] & 0xFFFFFFFFUL;
            uint64_t b_hi = b[j] >> 32;
            
            uint64_t p0 = a_lo * b_lo;
            uint64_t p1 = a_lo * b_hi;
            uint64_t p2 = a_hi * b_lo;
            uint64_t p3 = a_hi * b_hi;
            
            uint64_t mid = p1 + p2 + (p0 >> 32);
            uint64_t prod_lo = (mid << 32) | (p0 & 0xFFFFFFFFUL);
            uint64_t prod_hi = p3 + (mid >> 32);
            
            uint64_t sum = temp[i + j] + prod_lo + carry;
            carry = prod_hi + ((sum < temp[i + j]) ? 1 : 0);
            temp[i + j] = sum;
        }
        temp[i + 6] = carry;
    }
    
    // 2. Montgomery Reduction: result = temp * R^{-1} mod p
    for (int i = 0; i < 6; i++) {
        uint64_t m = temp[i] * MONTGOMERY_INV;
        uint64_t carry = 0;
        
        for (int j = 0; j < 6; j++) {
            uint64_t prod = m * BLS_MODULUS[j] + temp[i + j] + carry;
            carry = prod >> 64;  // High 64 bits
            temp[i + j] = prod;   // Low 64 bits
        }
        
        for (int j = 6; j < 12 - i; j++) {
            uint64_t sum = temp[i + j] + carry;
            carry = (sum < temp[i + j]) ? 1 : 0;
            temp[i + j] = sum;
        }
    }
    
    // 3. Copy upper 6 limbs to result
    for (int i = 0; i < 6; i++) {
        result[i] = temp[i + 6];
    }
    
    // 4. Final reduction if needed
    bool needs_reduce = false;
    for (int i = 5; i >= 0; i--) {
        if (result[i] > BLS_MODULUS[i]) {
            needs_reduce = true;
            break;
        } else if (result[i] < BLS_MODULUS[i]) {
            break;
        }
    }
    
    if (needs_reduce) {
        uint64_t borrow = 0;
        for (int i = 0; i < 6; i++) {
            uint64_t diff = result[i] - BLS_MODULUS[i] - borrow;
            borrow = (result[i] < BLS_MODULUS[i] + borrow) ? 1 : 0;
            result[i] = diff;
        }
    }
}

void main() {
    uint idx = gl_GlobalInvocationID.x;
    
    uint64_t a[6], b[6], result[6];
    
    // Load inputs (6 limbs per element)
    for (int i = 0; i < 6; i++) {
        a[i] = input_a.data[idx * 6 + i];
        b[i] = input_b.data[idx * 6 + i];
    }
    
    // Perform Montgomery multiplication
    montgomery_mul(result, a, b);
    
    // Store output
    for (int i = 0; i < 6; i++) {
        output.data[idx * 6 + i] = result[i];
    }
}

```

#### 3.2.2 SPIR-V 编译器 (src/gpu-executor/src/spirv/)

**compiler.rs** - GLSL → SPIR-V 编译

```rust
use shaderc::{Compiler, CompileOptions, ShaderKind, TargetEnv, EnvVersion};
use std::path::Path;

pub struct SpirvCompiler {
    compiler: Compiler,
}

#[derive(Debug, thiserror::Error)]
pub enum CompileError {
    #[error("Shaderc initialization failed: {0}")]
    InitError(String),
    
    #[error("Compilation failed: {0}")]
    CompilationError(String),
    
    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),
}

impl SpirvCompiler {
    pub fn new() -> Result<Self, CompileError> {
        let compiler = Compiler::new()
            .ok_or_else(|| CompileError::InitError("Failed to create shaderc compiler".into()))?;
        Ok(Self { compiler })
    }
    
    pub fn compile_glsl_file(
        &self,
        path: &Path,
        entry_point: &str,
    ) -> Result<Vec<u32>, CompileError> {
        let source = std::fs::read_to_string(path)?;
        self.compile_glsl(&source, entry_point, path.to_str().unwrap())
    }
    
    pub fn compile_glsl(
        &self,
        source: &str,
        entry_point: &str,
        filename: &str,
    ) -> Result<Vec<u32>, CompileError> {
        let mut options = CompileOptions::new()
            .ok_or_else(|| CompileError::InitError("Failed to create compile options".into()))?;
        
        // Enable 64-bit integer support
        options.add_macro_definition("GL_ARB_gpu_shader_int64", Some("1"));
        options.add_macro_definition("GL_EXT_shader_explicit_arithmetic_types_int64", Some("1"));
        
        // Target Vulkan 1.2 SPIR-V
        options.set_target_env(TargetEnv::Vulkan, EnvVersion::Vulkan1_2 as u32);
        options.set_target_spirv(shaderc::SpirvVersion::V1_5);
        
        // Optimization level
        options.set_optimization_level(shaderc::OptimizationLevel::Performance);
        
        // Generate debug info (optional)
        options.set_generate_debug_info();
        
        let artifact = self.compiler
            .compile_into_spirv(
                source,
                ShaderKind::Compute,
                filename,
                entry_point,
                Some(&options),
            )
            .map_err(|e| CompileError::CompilationError(e.to_string()))?;
        
        Ok(artifact.as_binary().to_vec())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_compiler_creation() {
        let compiler = SpirvCompiler::new();
        assert!(compiler.is_ok());
    }
    
    #[test]
    fn test_simple_shader_compilation() {
        let compiler = SpirvCompiler::new().unwrap();
        
        let source = r#"
            #version 450
            #extension GL_ARB_gpu_shader_int64 : enable
            
            layout(local_size_x = 1) in;
            layout(set = 0, binding = 0) buffer Output { uint64_t value; } output;
            
            void main() {
                output.value = 42UL;
            }
        "#;
        
        let result = compiler.compile_glsl(source, "main", "test.comp");
        assert!(result.is_ok());
        
        let spirv = result.unwrap();
        assert!(!spirv.is_empty());
        
        // Check SPIR-V magic number (0x07230203)
        assert_eq!(spirv[0], 0x07230203);
    }
}

```

**loader.rs** - WGPU SPIR-V 加载

```rust
use wgpu::{Device, ShaderModule, ShaderModuleDescriptorSpirV};
use std::borrow::Cow;

pub struct SpirvLoader;

impl SpirvLoader {
    pub unsafe fn load_spirv(
        device: &Device,
        spirv: &[u32],
        label: Option<&str>,
    ) -> ShaderModule {
        device.create_shader_module_spirv(&ShaderModuleDescriptorSpirV {
            label,
            source: Cow::Borrowed(spirv),
        })
    }
    
    pub unsafe fn load_spirv_from_bytes(
        device: &Device,
        bytes: &[u8],
        label: Option<&str>,
    ) -> Result<ShaderModule, String> {
        // Validate alignment
        if bytes.len() % 4 != 0 {
            return Err("SPIR-V binary must be 4-byte aligned".into());
        }
        
        // Cast to u32 slice
        let spirv = bytemuck::cast_slice::<u8, u32>(bytes);
        
        // Validate magic number
        if spirv.is_empty() || spirv[0] != 0x07230203 {
            return Err("Invalid SPIR-V magic number".into());
        }
        
        Ok(Self::load_spirv(device, spirv, label))
    }
}

```

---

## 4. 里程碑规划

### M14.1: GLSL Shader 开发 (0.5 周)

**目标**: 完成 BLS12-381 field operations GLSL 实现

**任务**:
1. ✅ 设计 6×u64 数据布局
2. ⏳ 实现 field_add (381-bit 加法)
3. ⏳ 实现 field_sub (381-bit 减法)
4. ⏳ 实现 montgomery_mul (蒙哥马利乘法)
5. ⏳ 实现 field_inv (费马小定理求逆)
6. ⏳ 单元测试 (CPU 验证)

**验收标准**:

- GLSL 编译通过 (glslangValidator)

- CPU 参考实现一致性测试 100% 通过

- 支持批量操作 (256 elements/workgroup)

---

### M14.2: shaderc 编译器集成 (0.5 周)

**目标**: 集成 shaderc,实现构建时 GLSL → SPIR-V 编译

**任务**:
1. ⏳ 添加 shaderc Cargo 依赖
2. ⏳ 实现 SpirvCompiler (compiler.rs)
3. ⏳ 构建脚本 (build.rs) 自动编译 shaders
4. ⏳ 生成 .spv 二进制文件
5. ⏳ 单元测试编译流程

**验收标准**:

- `cargo build` 自动编译所有 GLSL shaders

- 生成的 .spv 文件通过 spirv-val 验证

- 编译错误有清晰的错误信息

**依赖项**:

```toml
[dependencies]
shaderc = "0.8"
bytemuck = "1.14"

[build-dependencies]
shaderc = "0.8"

```

---

### M14.3: WGPU SPIR-V 直接加载 (0.5 周)

**目标**: 使用 wgpu unsafe API 加载 SPIR-V 二进制

**任务**:
1. ⏳ 实现 SpirvLoader (loader.rs)
2. ⏳ 更新 GpuZkProver 使用 SPIR-V shaders
3. ⏳ Pipeline 创建与绑定
4. ⏳ Buffer 管理与数据传输
5. ⏳ 集成测试

**验收标准**:

- 成功加载 SPIR-V shader modules

- Pipeline 创建无错误

- 能够执行简单的 u64 运算 (加法测试)

**代码示例**:

```rust
// Load pre-compiled SPIR-V
let spirv_bytes = include_bytes!("../shaders/bls12_381_field.spv");
let spirv_u32 = bytemuck::cast_slice::<u8, u32>(spirv_bytes);

let shader_module = unsafe {
    device.create_shader_module_spirv(&wgpu::ShaderModuleDescriptorSpirV {
        label: Some("BLS12-381 Field Operations"),
        source: std::borrow::Cow::Borrowed(spirv_u32),
    })
};

```

---

### M14.4: 跨平台测试与优化 (0.5 周)

**目标**: 验证 Vulkan/Metal/DX12 三平台正确性

**任务**:
1. ⏳ Vulkan (Linux/Windows) 测试
2. ⏳ Metal (macOS) 测试
3. ⏳ DX12 (Windows) 测试
4. ⏳ 性能基准测试
5. ⏳ 与 arkworks-rs CPU 实现对比

**验收标准**:

- 所有平台测试通过 (正确性 100%)

- GPU 性能 ≥ CPU arkworks-rs

- 批量处理 (N=1024) 延迟 < 10ms

**测试矩阵**:
| Platform | Backend | U64 Support | Status |
|----------|---------|-------------|--------|
| Linux | Vulkan | ✅ | ⏳ 待测试 |
| Windows | Vulkan | ✅ | ⏳ 待测试 |
| Windows | DX12 | ✅ | ⏳ 待测试 |
| macOS | Metal | ✅ | ⏳ 待测试 |

---

### M14.5: 文档与代码审查 (0.5 周)

**目标**: 完善文档,代码审查,准备合并

**任务**:
1. ⏳ API 文档 (Rustdoc)
2. ⏳ 技术文档 (SPIR-V 集成指南)
3. ⏳ 性能报告
4. ⏳ 代码审查与优化
5. ⏳ Phase 14 完成报告

**交付物**:

- `docs/PHASE14-COMPLETION-REPORT.md`

- `docs/SPIRV-INTEGRATION-GUIDE.md`

- `docs/BLS12-381-PERFORMANCE-REPORT.md`

- 更新 `ROADMAP.md`

---

## 5. 技术风险

### 5.1 已知风险

| 风险 | 影响 | 概率 | 缓解措施 |
|------|------|------|---------|
| **shaderc 依赖冲突** | 编译失败 | 中 | 使用 vendored feature |
| **SPIR-V 验证失败** | GPU 无法执行 | 低 | 使用 spirv-val 预验证 |
| **Metal u64 限制** | macOS 不支持 | 低 | Metal 已支持 64-bit |
| **DX12 u64 限制** | Windows 不支持 | 低 | DX12 SM 6.0+ 支持 |
| **性能不及预期** | 需要优化 | 中 | 使用 shared memory 优化 |

### 5.2 回退方案

如果原生 SPIR-V 遇到不可解决的问题:
1. **回退到 U32 模拟**: 使用 Phase 13 已实现的双 u32 模拟
2. **性能权衡**: 接受 20-30% 性能损失
3. **时间影响**: 增加 1 周优化时间

---

## 6. 性能目标

### 6.1 基准测试

**测试配置**:

- BLS12-381 field multiplication

- Batch size: 1024 operations

- Input: random field elements

- GPU: NVIDIA RTX 3060 / AMD RX 6800

**性能指标**:
| 操作 | CPU (arkworks) | GPU 目标 | 加速比 |
|------|---------------|---------|-------|
| Field Add | 100 ns | 10 ns | 10× |
| Field Mul | 500 ns | 50 ns | 10× |
| Field Inv | 5 μs | 500 ns | 10× |
| Batch 1024 | 500 μs | 50 μs | 10× |

### 6.2 优化策略

**阶段 1: 正确性** (M14.1-M14.3)

- 简单实现,保证正确

- 不优化内存访问

**阶段 2: 性能优化** (M14.4)

- Shared memory 优化

- Coalesced memory access

- Workgroup size 调优

**阶段 3: 高级优化** (Phase 15+)

- Kernel fusion

- Pipeline 优化

- Multi-GPU 调度

---

## 7. 依赖项与环境

### 7.1 Cargo 依赖

```toml
[dependencies]

# 现有依赖

wgpu = "0.18"
pollster = "0.3"
bytemuck = { version = "1.14", features = ["derive"] }

# 新增依赖

shaderc = "0.8"                # GLSL → SPIR-V 编译器
spirv-reflect = "0.2"          # SPIR-V 反射 (可选)

[build-dependencies]
shaderc = "0.8"                # 构建时编译 shaders

[dev-dependencies]
ark-bls12-381 = "0.4"          # CPU 参考实现
criterion = "0.5"              # 性能基准测试

```

### 7.2 系统依赖

**Linux**:

```bash

# Vulkan SDK

sudo apt-get install vulkan-sdk

# GLSL 工具链

sudo apt-get install glslang-tools spirv-tools

```

**macOS**:

```bash

# Vulkan SDK (MoltenVK)

brew install vulkan-sdk

# GLSL 工具链

brew install glslang spirv-tools

```

**Windows**:

```powershell

# Vulkan SDK

# 下载并安装 https://vulkan.lunarg.com/

# shaderc (included in Vulkan SDK)

```

---

## 8. 进度跟踪

### 8.1 每日站会

- **时间**: 每天 10:00 AM

- **内容**: 昨日完成、今日计划、阻塞问题

- **记录**: `docs/phase14-daily-standup.md`

### 8.2 里程碑检查点

| 日期 | 里程碑 | 检查内容 |
|------|--------|---------|
| Week 1 Day 3 | M14.1 | GLSL shader 正确性测试 |
| Week 1 Day 5 | M14.2 | shaderc 编译集成完成 |
| Week 2 Day 2 | M14.3 | WGPU 加载测试通过 |
| Week 2 Day 4 | M14.4 | 跨平台性能达标 |
| Week 2 Day 5 | M14.5 | 文档审查完成 |

---

## 9. 成功标准

Phase 14 成功完成的标准:

1. ✅ **功能完整性**
   - BLS12-381 field operations 正确实现
   - 支持 add, sub, mul, inv 操作
   - 所有单元测试通过

2. ✅ **跨平台支持**
   - Vulkan (Linux/Windows) 验证通过
   - Metal (macOS) 验证通过
   - DX12 (Windows) 验证通过

3. ✅ **性能目标**
   - GPU 性能 ≥ CPU arkworks-rs
   - Batch 1024 延迟 < 10ms
   - 内存带宽利用率 > 70%

4. ✅ **代码质量**
   - 代码覆盖率 > 80%
   - 无 clippy warnings
   - 通过代码审查

5. ✅ **文档完整**
   - API 文档完整
   - 集成指南清晰
   - 性能报告详细

---

## 10. 后续规划

### Phase 15: 完整 ZK Prover

在 Phase 14 完成后:
1. **MSM (Multi-Scalar Multiplication)**: 基于 field ops 实现
2. **FFT (Fast Fourier Transform)**: Polynomial 运算
3. **Groth16 Prover**: 完整 ZK 证明生成
4. **多 GPU 调度**: 利用 Phase 13 LoadBalancer

---

**文档版本**: v1.0  
**创建日期**: 2025-11-13  
**最后更新**: 2025-11-13  
**维护者**: GPU Executor Team
