# SuperVM 开发者信息

## 开发团队

- **Rainbow Haruko** - iscrbank@gmail.com
- **King Xu (XujueKing)** - leadbrand@me.com
- **Alan Tang**
- **Xuxu**
- **Noah X**

## 项目信息

- **GitHub**: https://github.com/XujueKing/SuperVM
- **许可协议**: GPL-3.0-or-later
- **版权所有**: XujueKing <leadbrand@me.com>

## ZK Verifier 接入说明（vm-runtime privacy）

- 可插拔抽象：`privacy::zksnark::{ZkVerifier, ZkCircuitId, ZkError}`，默认提供 `NoopVerifier`。
- 可选后端：启用 feature `groth16-verifier` 时，提供 `privacy::groth16_verifier::Groth16Verifier` 适配器（当前为占位实现，后续接入 arkworks 验证）。

使用与测试：
- 默认构建/测试（不启用 ZK 后端）：
	- 单元/集成测试会使用 `NoopVerifier`，其默认返回 `false`，避免误接受证明。
- 启用 Groth16 适配器：
	- 测试：在 vm-runtime 下启用 feature 运行
		- cargo test -p vm-runtime --features groth16-verifier
	- 预期：占位实现对未知电路返回 `ZkError::UnknownCircuit`。

L1 扩展提交流程提示（涉及 `src/vm-runtime/src/supervm.rs` 等 L1 路径时）：
- 确认以下事项再提交：
	1) 功能开关（feature flag）已接入（已完成：`groth16-verifier`）。
	2) 对应测试已覆盖（已新增：`tests/privacy_verifier_tests.rs`）。
	3) 文档已更新（本节即为对应更新）。
	4) 本地执行特性测试通过：`cargo test -p vm-runtime --features groth16-verifier`。
	5) 根据需要创建 L1 修改申请 Issue（参见仓库 Issue Templates）。

## 联系方式

如需参与贡献或有任何问题，请：
1. 在 GitHub 上提交 Issue 或 Pull Request
2. 通过邮箱联系项目维护者

---

*此文件用于统一标注本仓库的主要开发者信息。*
