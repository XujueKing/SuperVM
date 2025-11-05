---
name: L1 扩展修改申请
about: 申请修改 SuperVM L1 内核扩展代码
title: '[L1-CORE] '
labels: 'L1-core, kernel-extension, needs-review'
assignees: ''
---

# L1 内核扩展修改申请表

> ⚠️ **提示**: 此申请用于修改 SuperVM 内核扩展层 (L1)  
> 📖 **文档**: 参见 `docs/KERNEL-DEFINITION.md` Section 4.2

---

## 1. 申请人信息

- **姓名**: 
- **GitHub ID**: @
- **日期**: YYYY-MM-DD
- **所属团队**: 

---

## 2. 修改概述

### 2.1 修改类型
- [ ] 新增扩展功能
- [ ] 修改现有扩展
- [ ] 性能优化
- [ ] Bug修复
- [ ] 重构
- [ ] 其他: _____________

### 2.2 涉及文件
<!-- 列出所有将要修改的 L1 文件 -->
```
src/vm-runtime/src/
├── [ ] ownership.rs           (所有权转移扩展)
├── [ ] supervm.rs            (高级 API 封装)
├── [ ] execution_trait.rs    (执行引擎 trait)
└── [ ] 新文件: ______________
```

### 2.3 Feature Flag
<!-- L1 修改必须通过 feature flag 控制 -->
- **Feature 名称**: `feature = ""`
- **默认启用**: [ ] 是 [ ] 否
- **依赖的 feature**: 

---

## 3. 修改动机

### 3.1 功能需求
<!-- 描述新增或修改的功能 -->

**用户故事**:
> 作为 _____, 我希望 _____, 以便 _____

**使用场景**:


### 3.2 为什么放在 L1 层?
<!-- 说明为什么这是内核扩展而非插件 -->

**技术判断**:
- [ ] 需要访问 L0 内核 API
- [ ] 需要与 L0 紧密集成
- [ ] 性能敏感,不能走插件路径
- [ ] 通用性强,适合作为内核扩展
- [ ] 其他原因: 

---

## 4. 技术方案

### 4.1 设计概述


### 4.2 核心 API
<!-- 列出新增或修改的公开 API -->

```rust
// 新增 API 签名
pub fn new_api_function() -> Result<(), Error> {
    // ...
}
```

### 4.3 与 L0 交互
<!-- 描述如何使用 L0 内核 API -->

**依赖的 L0 API**:
- `Runtime::execute()`
- `Storage::read()`
- 

**交互模式**:
- [ ] 只读访问 L0
- [ ] 读写访问 L0
- [ ] 扩展 L0 行为

### 4.4 Feature Flag 配置
```toml
# Cargo.toml
[features]
default = []
your-feature = []  # 你的 feature

# 可选依赖
[dependencies]
some-lib = { version = "1.0", optional = true }
```

---

## 5. 性能影响

### 5.1 性能预期
<!-- L1 扩展应该不影响 L0 核心性能 -->

| 场景 | 基准 TPS | 启用 feature 后 TPS | 变化 |
|------|---------|-------------------|------|
| 低竞争 (不使用扩展) | 187,000 | | 0% |
| 低竞争 (使用扩展) | - | | |
| 高竞争 (使用扩展) | - | | |

### 5.2 性能隔离
- [ ] **确认**: 当 feature 关闭时,零性能开销
- [ ] **确认**: 不影响 L0 核心路径性能

---

## 6. 测试计划

### 6.1 Feature 测试
- [ ] 功能测试: `cargo test --features your-feature`
- [ ] 无 feature 测试: `cargo test --no-default-features`
- [ ] 测试覆盖率: ___%

### 6.2 集成测试
```bash
# 测试 feature 组合
cargo test --features "feature-a,feature-b"
```

### 6.3 文档测试
- [ ] 示例代码测试: `cargo test --doc`

---

## 7. 文档要求

### 7.1 代码文档
- [ ] Rustdoc 注释完整
- [ ] 使用示例 (doc examples)
- [ ] 错误处理说明

### 7.2 用户文档
- [ ] 更新 `docs/API.md`
- [ ] 更新 `docs/quick-reference-2.0.md`
- [ ] 更新 `CHANGELOG.md` 添加 `[L1-CORE]` 标签

### 7.3 示例代码
- [ ] 添加 `examples/*.rs` 示例

---

## 8. 兼容性

### 8.1 向后兼容性
- [ ] 完全兼容
- [ ] 有 Breaking Change:
  - 影响范围: 
  - 迁移指南: 

### 8.2 版本号
- **当前版本**: v0.4.x
- **目标版本**: v0.__.__ (MINOR 版本号变更)

---

## 9. 审批流程

### 9.1 必须审批人员
<!-- L1 修改需要 1名核心开发者审批 -->

- [ ] **核心开发者**: @________ (必填)

### 9.2 Code Review 检查清单
- [ ] 代码质量高
- [ ] Feature flag 正确配置
- [ ] 单元测试完善
- [ ] Rustdoc 文档完整
- [ ] 性能隔离验证
- [ ] 不影响 L0 核心

---

## 10. 实施计划

### 10.1 时间线
- **开发开始**: YYYY-MM-DD
- **PR 提交**: YYYY-MM-DD
- **Review 完成**: YYYY-MM-DD
- **合并目标**: YYYY-MM-DD

### 10.2 发布计划
- **版本号**: v0.__.__ (MINOR 版本)
- **发布说明**: 

---

## 11. 附加信息

### 11.1 相关 Issue
- Closes #
- Related to #

### 11.2 依赖关系
<!-- 是否依赖其他 PR 或 Issue -->


### 11.3 备注


---

## 审批记录

### 核心开发者审批意见
- **审批人**: @
- **审批时间**: YYYY-MM-DD
- **审批结果**: [ ] 批准 [ ] 拒绝 [ ] 需要修改
- **意见**: 

---

**最终决定**: [ ] 批准合并 [ ] 拒绝 [ ] 延期讨论
