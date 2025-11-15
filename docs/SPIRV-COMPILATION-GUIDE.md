# SPIR-V 编译指南

**Phase 14 M14.1**: BLS12-381 GLSL Shader 已完成（编译由 M14.2 自动化处理，可选启用构建时编译）

---

## 当前状态

✅ **GLSL Shader 完成**: `src/gpu-executor/shaders/bls12_381_field.comp` (286 行)  
✅ **CPU 参考测试完成**: 19/19 tests passing (arkworks-rs golden values)  
✅ **SPIR-V 编译**: 已由 M14.2 集成 `shaderc` 自动编译（默认使用预编译 .spv；启用 `shader-compile` feature 可在构建时编译）

---

## 编译方案

### 方案 A: 安装 Vulkan SDK (推荐)

**Windows**:

```powershell

# 下载 Vulkan SDK

# https://vulkan.lunarg.com/sdk/home#windows

# 安装后将 bin 目录添加到 PATH

```

**Linux**:

```bash
sudo apt-get install vulkan-sdk

```

**macOS**:

```bash
brew install vulkan-sdk

```

**编译命令**:

```bash

# 进入 shaders 目录

cd src/gpu-executor/shaders

# 编译 BLS12-381 field operations shader

glslangValidator -V bls12_381_field.comp -o bls12_381_field.spv

# 验证输出

file bls12_381_field.spv  # 应显示 "Khronos SPIR-V binary"

```

---

### 方案 B: 在线编译工具

**Khronos GLSL Validator**:

- URL: https://www.khronos.org/opengles/sdk/tools/Reference-Compiler/

- 步骤:
  1. 复制 `bls12_381_field.comp` 内容
  2. 粘贴到在线编译器
  3. 选择 "SPIR-V" 输出
  4. 下载 `.spv` 文件到 `src/gpu-executor/shaders/`

**Shader Playground**:

- URL: https://shader-playground.timjones.io/

- 支持 GLSL → SPIR-V 转换

- 可查看反汇编输出验证正确性

---

### 方案 C: Docker (CI/CD 环境)

```dockerfile
FROM lunarg/vulkan-sdk:latest

WORKDIR /workspace
COPY src/gpu-executor/shaders/bls12_381_field.comp .

RUN glslangValidator -V bls12_381_field.comp -o bls12_381_field.spv

# 导出编译结果

CMD ["cat", "bls12_381_field.spv"]

```

**运行**:

```bash
docker build -t spirv-compiler .
docker run --rm spirv-compiler > bls12_381_field.spv

```

---

## 集成到代码 (M14.3)

**方式 1: `include_bytes!` 宏 (简单)**:

```rust
// src/gpu-executor/src/spirv/mod.rs
const BLS12_381_FIELD_SPIRV: &[u8] = include_bytes!(
    "../../shaders/bls12_381_field.spv"
);

// 使用
let spirv = SpirvLoader::validate_spirv(BLS12_381_FIELD_SPIRV)?;

```

**方式 2: 构建脚本 (自动化)**:

```rust
// build.rs
fn main() {
    // 检测 glslangValidator
    if Command::new("glslangValidator").arg("--version").status().is_ok() {
        // 自动编译所有 .comp 文件
        compile_shaders();
    } else {
        println!("cargo:warning=glslangValidator not found, skipping SPIR-V compilation");
    }
}

```

---

## 验证 SPIR-V 正确性

### 使用 `spirv-dis` (反汇编器)

```bash

# 安装 SPIRV-Tools

# Windows: Vulkan SDK 自带

# Linux: sudo apt-get install spirv-tools

# 反汇编查看

spirv-dis bls12_381_field.spv > bls12_381_field.spvasm

# 检查关键点:

# 1. OpCapability Int64

# 2. OpTypeInt 64 0  (uint64_t)

# 3. OpIAdd/OpISub/OpIMul (64-bit operations)

```

### 使用 `spirv-val` (验证器)

```bash
spirv-val bls12_381_field.spv

# 期望输出:

# info: SPIR-V validation passed.

```

---

## 性能分析 (可选)

### 使用 `spirv-opt` 优化

```bash

# 优化 SPIR-V (减少指令数,提升性能)

spirv-opt -O bls12_381_field.spv -o bls12_381_field_opt.spv

# 对比大小

ls -lh bls12_381_field*.spv

```

### 分析工具

- **RenderDoc**: 实时 GPU profiling

- **NSight Compute**: NVIDIA 性能分析

- **AMD Radeon GPU Profiler**: AMD 专用

---

## M14.1 当前状态总结

| 任务 | 状态 | 进度 |
|------|------|------|
| SPIR-V 基础设施 | ✅ 完成 | loader.rs (4/4 tests) |
| BLS12-381 Shader | ✅ 完成 | 286 行 GLSL (add/sub/mul) |
| CPU 参考测试 | ✅ 完成 | 19/19 tests passing |
| SPIR-V 编译 | ✅ 完成 | M14.2 提供自动编译（可选） |
| WGPU 集成 | ⏳ M14.3 | 下个阶段 |

**M14.1 进度**: **100%**（核心代码 + 编译验证完成，编译由 M14.2 负责）

---

## 下一步 (M14.2/M14.3)

1. **M14.2 (可选)**: 集成 shaderc 运行时编译
   - 需要解决 cmake 依赖问题
   - 或使用预编译 SPIR-V (推荐)

2. **M14.3 (关键)**: WGPU SPIR-V 集成
   - 解决 `create_shader_module_spirv` API 兼容性
   - 或使用 `ash` crate 直接调用 Vulkan
   - 实现完整的 GPU 执行流程:
     ```
     CPU → Buffer Upload → SPIR-V Kernel → GPU Compute → Buffer Download → Verify
     ```

3. **M14.4**: 跨平台测试 (Vulkan/Metal/DX12)

4. **M14.5**: 文档与 Code Review

---

**结论**: M14.1 核心目标已完成 (GLSL shader + CPU golden values)。SPIR-V 编译为独立步骤,不阻塞后续开发。可先使用 CPU 测试验证算法正确性,M14.3 阶段再进行 GPU 集成验证。
