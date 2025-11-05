# SuperVM 模块分级与版本索引

> 本文档统一标注仓库内各模块的层级（L0/L1/L2/L3/L4）与版本号，并给出版本号演进规则。
>
> 版本来源：使用 Cargo 工作区版本或各 crate 自身的 Cargo.toml。

开发者/作者：King Xujue

---

## 版本号策略（SemVer + 层级约束）

- L0（核心内核）
  - MAJOR：任何 Breaking Change（包括性能语义变化）
  - MINOR：向后兼容的功能增强（极少数情况）
  - PATCH：Bug 修复、微优化、文档
- L1（内核扩展）
  - MAJOR：公开 API 的 Breaking Change
  - MINOR：新增 API（feature flag 控制）
  - PATCH：修复/内部优化
- L2（接口层 Trait/Adapter）
  - MAJOR：接口契约变更
  - MINOR：新增可选接口
  - PATCH：文档/实现修正
- L3（外部插件/研究组件）
  - 实验性质，遵循 SemVer，但允许更快迭代
- L4（应用/节点）
  - 正常遵循 SemVer，按发布节奏演进

> 注意：工作区当前版本为 0.1.0（见根 `Cargo.toml`），个别 crate 可能有自己的版本。

---

## 模块清单

| 模块 | 路径 | 层级 | 当前版本 | 说明 |
|------|------|------|----------|------|
| vm-runtime | `src/vm-runtime` | L0/L1 | 0.1.0 | 核心 WASM 运行时 + 并行调度 + MVCC（L0）；扩展 API（L1） |
| node-core | `src/node-core` | L4 | 0.1.0 | 节点/应用层，依赖 tokio、CLI 等 |
| privacy-test | `privacy-test` | L3 | 0.1.0 | 隐私密码学练习与基准（curve25519、RingSig） |
| halo2-eval | `halo2-eval` | L3 | 0.1.0 | Halo2 评估实验（独立版本） |
| zk-groth16-test | `zk-groth16-test` | L3 | 0.1.0 | Groth16 证明实验与基准 |
| examples | `examples/*.rs` | L4 | N/A | 示例程序，不单独发版 |

---

## vm-runtime 子模块分级明细

- L0（核心路径）
  - `src/vm-runtime/src/runtime.rs`
  - `src/vm-runtime/src/wasm_executor.rs`
  - `src/vm-runtime/src/storage.rs`
  - `src/vm-runtime/src/storage_api.rs`
  - `src/vm-runtime/src/chain_api.rs`
  - `src/vm-runtime/src/parallel/**/*`
  - `src/vm-runtime/src/mvcc/**/*`
- L1（扩展层 - 连接 L0 与 L2）
  - `src/vm-runtime/src/ownership.rs` - 对象所有权管理
  - `src/vm-runtime/src/supervm.rs` - 统一路由入口
  - `src/vm-runtime/src/execution_trait.rs` - **统一执行引擎接口** ✅
    - 作用: 桥接 L0 核心层与 L2 适配器层
    - 向下封装: L0 的 WASM 执行能力
    - 向上暴露: 统一的 ExecutionEngine trait
    - 支持引擎: WASM, EVM (可扩展)

版本演进（建议）：
- L0 发生 Breaking → vm-runtime 直接提升 MAJOR 版本（例如 1.x → 2.0.0）
- 仅新增 L1 扩展 API → MINOR+1（例如 1.2.0 → 1.3.0）
- 修复/优化 → PATCH+1（例如 1.2.3 → 1.2.4）

**最新更新 (v0.1.0)**:
- ✅ 新增 `execution_trait.rs` (L1) - 统一执行引擎接口
- 76 行代码，包含 `ExecutionEngine` trait 定义
- 测试覆盖: `test_execution_trait` ✅

---

## 版本来源与验证

- 工作区版本：根 `Cargo.toml` → `[workspace.package] version = "0.1.0"`
- 继承版本：`src/vm-runtime/Cargo.toml`, `src/node-core/Cargo.toml` 使用 `version.workspace = true`
- 独立版本：`halo2-eval/Cargo.toml` 使用 `version = "0.1.0"`

验证命令（可选）：
```bash
# 列出所有 crate 版本
cargo metadata --no-deps --format-version 1 | jq -r '.packages[] | "\(.name) \(.version)"'
```

---

## 兼容性与依赖约束（摘要）

- L0 禁止依赖：`revm`, `ethers`, `web3`, `tokio`, `async-std` 等
- L1 必须通过 feature flag 暴露功能
- L3 插件/实验不得反向依赖 L0 内核私有实现

完整规则见：`docs/KERNEL-DEFINITION.md`

---

## 版本升级流程（快速清单）

1. 确认修改所属层级（L0/L1/...）
2. 根据层级确定版本位（MAJOR/MINOR/PATCH）
3. 更新对应 `Cargo.toml` 版本号
4. 更新 `CHANGELOG.md`
5. 运行 CI（单测/基准/纯净度）
6. 打标签发布（tag: `crate-name@x.y.z`）

---

## 关联文档
- 内核定义与保护机制：`docs/KERNEL-DEFINITION.md`
- 开发者速查：`docs/KERNEL-QUICK-REFERENCE.md`
- 项目路线图：`ROADMAP.md`
