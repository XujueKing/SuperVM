# Contributing

开发者：king

欢迎贡献！非常感谢你愿意为本项目做出贡献。以下是快速指南，帮助你更顺利地提交 PR。

1. 提交类型
   - Bug 修复：包含复现步骤与测试用例
   - 新功能：包含设计说明与兼容性影响
   - 文档：清晰、可复现的说明

2. 本地开发
   - 使用 rustup 安装工具链（stable channel）
   - 使用 cargo fmt 格式化代码：
     - cargo fmt --all
   - 使用 cargo clippy 检查：
     - cargo clippy --all-targets --all-features -- -D warnings

3. 提交规范
   - 使用清晰的提交信息，格式例如：
     - chore: …
     - feat(module): …
     - fix(module): …
   - 每次 PR 应包含说明、如何测试以及关联 Issue（如果有）

4. Pull Request 流程
   - Fork 仓库并创建分支（feature/xxx 或 fix/xxx）
   - 提交代码并推送到你的远程分支
   - 打开 PR 并在描述中包含复现步骤与测试说明
   - CI 会自动检查格式、clippy、构建与测试，请确保本地先通过这些检查

5. 风险与兼容性
   - 在引入破坏性变更时，请在 PR 描述中明确说明兼容性/迁移步骤

6. 代码审查
   - 维护者会审阅你的 PR，可能会请求修改或补充测试
   - 请在 PR 中回应审查意见并保持分支更新

感谢你的贡献！
