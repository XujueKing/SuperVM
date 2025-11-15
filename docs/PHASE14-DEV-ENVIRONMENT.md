# Phase 14 开发环境说明

## 当前状态 (2025-11-13)

### 问题: Windows 环境缺少 cmake

shaderc-sys 需要 cmake 来编译原生库,但 Windows 开发环境未安装 cmake。

### 解决方案选项

#### 选项 A: 安装 cmake (推荐用于生产环境)

```powershell

# 使用 chocolatey

choco install cmake

# 或下载安装器

https://cmake.org/download/

```

**优势**: 

- 可以运行时编译 GLSL → SPIR-V

- 更灵活的开发体验

**劣势**:

- 需要额外安装步骤

- 首次编译时间长

---

#### 选项 B: 使用预编译 SPIR-V (当前采用) ⭐

直接使用外部工具预编译 GLSL → SPIR-V,然后在 Rust 中加载 .spv 文件。

**工具链**:
1. **glslangValidator** (Vulkan SDK 自带)
   ```bash
   glslangValidator -V shader.comp -o shader.spv
   ```

2. **在线工具**:
   - https://shader-playground.timjones.io/
   - https://www.khronos.org/spir-v/

**Rust 集成**:

```rust
// 使用 include_bytes! 嵌入预编译的 SPIR-V
let spirv_bytes = include_bytes!("../shaders/bls12_381_field.spv");
let spirv_u32 = bytemuck::cast_slice::<u8, u32>(spirv_bytes);

let shader_module = unsafe {
    device.create_shader_module_spirv(&wgpu::ShaderModuleDescriptorSpirV {
        label: Some("BLS12-381 Field Operations"),
        source: std::borrow::Cow::Borrowed(spirv_u32),
    })
};

```

**优势**:

- ✅ 无需 cmake/shaderc 运行时依赖

- ✅ 更快的编译速度

- ✅ 确定性的 SPIR-V 版本

- ✅ 可以在 CI/CD 中预编译

**劣势**:

- ⚠️ 需要手动编译 shader (可自动化)

- ⚠️ 调试稍微复杂

---

### 当前实现策略

1. **保留 spirv 模块代码** (用于未来 cmake 环境)
2. **创建 shaders/ 目录存放 GLSL 源码**
3. **提供 GLSL → SPIR-V 编译脚本**
4. **使用 include_bytes! 加载预编译 .spv**

---

## 编译 GLSL Shader (手动步骤)

### 方法 1: 使用 Vulkan SDK (推荐)

1. **安装 Vulkan SDK**:
   - Windows: https://vulkan.lunarg.com/
   - 包含 glslangValidator 工具

2. **编译 shader**:
   ```bash
   cd src/gpu-executor/shaders
   glslangValidator -V test_u64_add.comp -o test_u64_add.spv
   glslangValidator -V bls12_381_field.comp -o bls12_381_field.spv
   ```

3. **验证 SPIR-V**:
   ```bash
   spirv-val test_u64_add.spv
   ```

### 方法 2: 使用在线工具

1. 访问 https://shader-playground.timjones.io/
2. 选择 "GLSL" → "SPIR-V"
3. 粘贴 GLSL 代码
4. 下载生成的 .spv 文件

### 方法 3: Docker (跨平台)

```bash
docker run --rm -v ${PWD}:/shaders khronosgroup/glslang \
  glslangValidator -V /shaders/test_u64_add.comp -o /shaders/test_u64_add.spv

```

---

## 后续计划

### Phase 14 M14.1-M14.3 (当前阶段)

- 使用**预编译 SPIR-V**方案

- 手动编译 shader (或提供脚本)

- 专注于 BLS12-381 算法实现

### Phase 14 M14.4+ (可选优化)

- 如果需要运行时编译,安装 cmake

- 或使用 CI/CD 自动编译 shader

- 考虑使用 `spirv-builder` (rustc SPIR-V 后端)

---

## 验证当前环境

```bash

# 检查是否安装 Vulkan SDK

where glslangValidator

# 如果没有,下载安装:

# https://vulkan.lunarg.com/sdk/home#windows

```

---

**决策**: 采用**选项 B - 预编译 SPIR-V**,避免 cmake 依赖,加速开发进度。

**更新日期**: 2025-11-13
