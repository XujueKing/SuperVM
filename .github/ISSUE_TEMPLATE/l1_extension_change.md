---
name: "L1 Extension Change Request"
about: "提交 L1 扩展（非 L0 核心）修改的审批申请"
title: "[L1] <简要描述变更>"
labels: ["L1", "approval-needed"]
assignees: []
---

## 变更摘要

- 变更范围（文件/目录）：
- 变更类型（feat/fix/refactor/doc/test/chore）：
- 背景/动机：

## 功能开关（Feature Flag）

- Feature 名称：
- 默认状态（开启/关闭）：
- 如何启用：`cargo build/test --features <feature>`

## 测试与验证

- [ ] 单元测试覆盖（列出文件/用例）：
- [ ] 集成/端到端验证（如适用）：
- [ ] 本地运行结果：
  - 默认：`cargo test -p vm-runtime`
  - 启用特性：`cargo test -p vm-runtime --features <feature>`

## 性能/安全影响评估

- 性能影响（预计/实测）：
- 安全边界影响（内核纯净、隔离、DoS 风险等）：
- 回滚方案：

## 文档

- [ ] 开发文档已更新（文件名/链接）：
- [ ] 用户文档（如适用）：

## 审批

- 审批人 King：
- 备注：
