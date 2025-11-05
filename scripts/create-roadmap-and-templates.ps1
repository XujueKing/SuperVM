# 开发者：king
# Set working directory to script's parent directory (project root)
$projectRoot = Split-Path -Parent $PSScriptRoot
Set-Location -Path $projectRoot

# Ensure directories
$paths = @(
    ".github",
    ".github\ISSUE_TEMPLATE",
    "scripts"
)
foreach ($p in $paths) {
    if (-not (Test-Path $p)) { New-Item -ItemType Directory -Path $p | Out-Null }
}

# Write ROADMAP.md
@"
# SuperVM - Roadmap (初始)

## 目标
- 使用 Rust 实现高性能 WASM-first runtime
- 支持 Solidity (via Solang) 与 JS/TS (AssemblyScript) 合约开发体验
- 可选 EVM 兼容层（revm/evmone 集成或 EVM->WASM 转译）

## 里程碑
1. 准备（周0）
   - 仓库骨架、开发规范、CI 初始
2. PoC（周1-3）
   - vm-runtime 最小运行时（wasmtime/wasmi），node-core demo（单节点、交易、执行）
3. 编译器适配（周4-8）
   - 集成 Solang、AssemblyScript；JS SDK + 部署脚本
4. 并行执行 PoC（周9-14）
   - 账户并行调度器、冲突检测、回滚机制
5. EVM 兼容（周15-22）
   - 集成 revm/evmone 或实现转译层，运行 ERC20 测试
6. 预发布与生产准备（周23-36）
   - 完整网络、共识插件、监控、审计、文档与开发者体验优化

## 交付物（每里程碑）
- vm-runtime crate: wasm runtime + host API
- node crate: 本地测试链 binary
- compiler-adapter: 自动化编译/打包流程 (WASM + metadata)
- evm-compat: 可插拔 EVM 后端
- js-sdk + hardhat plugin、示例 dApp、benchmark 报告

## 风险与缓解
- Solidity->WASM 语义差异：限制不支持特性、提供兼容层说明
- 并行执行复杂性：从可回滚的简单模型开始、增加冲突检测与重试
- 性能开销：选择高性能 runtime、对热路径优化与缓存

## 短期下一步（可选执行）
- PoC: 在 vm-runtime 集成 wasmi/wasmtime 并运行示例合约
- 集成 Solang demo：Solidity -> WASM -> 在本地 runtime 执行
- 生成 CI、ISSUE/PR 模板、Roadmap（本文件）

## 联系与贡献
请在 Issues 中提交 bug/feature 请求，或通过 PR 提交代码。遵循 CONTRIBUTING 指南。
"@ | Set-Content -Path "ROADMAP.md" -Encoding UTF8

# Write bug_report.md
@"
---
name: Bug report
about: 报告 bug 或不可预期行为
title: "[bug] <简短描述>"
labels: bug
assignees: ""
---

**描述**
请简要描述出现的问题是什么，以及期望行为。

**复现步骤**
复现问题的最小步骤，包括示例命令或代码片段：
1. ...
2. ...
3. ...

**环境**
- 操作系统: Windows/Linux/macOS
- Rust 版本: `rustc --version`
- 分支/提交: `git rev-parse --short HEAD`

**日志/错误信息**
请粘贴相关日志或错误堆栈（如有）。

**附加信息**
可选：截图、回滚步骤、是否可在干净仓库复现等。
"@ | Set-Content -Path ".github\ISSUE_TEMPLATE\bug_report.md" -Encoding UTF8

# Write feature_request.md
@"
---
name: Feature request
about: 提议新功能或改进
title: "[feature] <简短描述>"
labels: enhancement
assignees: ""
---

**目标**
说明你希望新增的功能或改进是什么，解决了什么痛点。

**用例**
说明具体使用场景、示例以及期望的交互/API。

**替代方案**
若有现有变通方案，请描述并说明不足之处。

**优先级**
- 必要 / 重要 / 可选

**备注**
可选：实现建议、相关参考、兼容影响。
"@ | Set-Content -Path ".github\ISSUE_TEMPLATE\feature_request.md" -Encoding UTF8

# Write PULL_REQUEST_TEMPLATE.md
@"
# Pull Request

感谢贡献！请使用以下模板填写信息以加快审核。

## 变更类型
- [ ] Bug 修复
- [ ] 新功能
- [ ] 文档
- [ ] 测试
- [ ] 其他（描述）

## 说明
简要描述本次变更的目的、实现思路及关键改动点。

## 如何测试
提供复现或测试步骤，包含命令或示例：
1. ...
2. ...

## 关联 Issue
Fixes #<issue-number> （如适用）

## Checklist
- [ ] 代码通过 `cargo fmt` 和 `cargo clippy` 检查
- [ ] 添加/更新相关测试
- [ ] 更新文档（如需要）

## 注意事项
如有破坏性变更或迁移步骤，请在此说明。
"@ | Set-Content -Path ".github\PULL_REQUEST_TEMPLATE.md" -Encoding UTF8

# Write .editorconfig
@"
root = true

[*]
charset = utf-8
end_of_line = lf
insert_final_newline = true
"@ | Set-Content -Path ".editorconfig" -Encoding UTF8

# Git add & commit
if (-not (Test-Path ".git")) {
    git init
}
git add -A
try {
    git commit -m "chore: add ROADMAP and GitHub issue/PR templates" -q
    Write-Host "Files created and committed successfully."
} catch {
    Write-Host "Commit skipped or failed: $($_.Exception.Message)"
}

Write-Host "完成：ROADMAP.md 与 GitHub 模板已创建。"
