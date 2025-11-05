# SuperVM 内核保护 - 开发者快速参考

> **警告**: 修改 L0/L1 层代码前必读

开发者/作者：King Xujue

---

## 核心定义

### L0 (Core Kernel) - 核心内核
**文件清单** (20个核心文件):
```
src/vm-runtime/src/
├── lib.rs                    # 模块根
├── runtime.rs                # 核心运行时
├── wasm_executor.rs          # WASM 执行器
├── storage.rs                # 存储抽象
├── storage_api.rs            # 存储 API
├── chain_api.rs              # 链 API
├── parallel/                 # 并行调度
│   ├── mod.rs
│   ├── scheduler.rs
│   ├── executor.rs
│   ├── work_stealing.rs
│   └── channel.rs
└── mvcc/                     # MVCC 引擎
    ├── mod.rs
    ├── transaction.rs
    ├── version_chain.rs
    ├── read_set.rs
    ├── write_set.rs
    ├── conflict_detector.rs
    ├── gc.rs
    └── storage_backend.rs
```

### L1 (Kernel Extensions) - 内核扩展
```
src/vm-runtime/src/
├── ownership.rs              # 所有权转移扩展
├── supervm.rs                # 高级 API 封装
└── execution_trait.rs        # 执行引擎 trait
```

---

## 修改前检查清单

### L0 修改 (CRITICAL)
- [ ] 无法在 L1/L2/L3 实现?
- [ ] 已填写 L0 修改申请表?
- [ ] 已获得架构师 + 2名核心开发者批准?
- [ ] 已运行完整测试套件?
- [ ] 已运行性能基准测试?
- [ ] 性能退化 < 5%?
- [ ] 已更新 CHANGELOG 添加 `[L0-CRITICAL]` 标签?

**命令**:
```bash
# 测试
cargo test --workspace

# 性能测试
cargo bench --bench parallel_execution
cargo bench --bench mvcc_throughput

# 纯净构建
cargo build -p vm-runtime --no-default-features
```

---

### L1 修改 (WARNING)
- [ ] 已通过 feature flag 控制?
- [ ] 已填写 L1 修改申请表?
- [ ] 已获得 1名核心开发者批准?
- [ ] feature 关闭时零性能开销?
- [ ] 已添加 Rustdoc 文档?
- [ ] 已更新 CHANGELOG 添加 `[L1-CORE]` 标签?

**命令**:
```bash
# Feature 测试
cargo test --features your-feature
cargo test --no-default-features

# 文档测试
cargo test --doc
```

---

## 造物主/维护者覆盖 (Override)

> 仅限 `.github/MAINTAINERS` 白名单内的 owner/architect 使用

### 本地覆盖方式（任选其一）

```powershell
# 方式1: 临时环境变量（仅当前会话）
$env:SUPERVM_OVERRIDE = "1"

# 方式2: Git 配置（仓库级）
git config supervm.override true
# 关闭: git config --unset supervm.override

# 方式3: 临时覆盖文件（提交前删除）
New-Item -ItemType File .kernel-override -Force | Out-Null

# 方式4: 上帝分支（自动放行，推荐！）
git checkout -b king/hotfix-critical
# 或直接在 main 分支工作
```

### CI 覆盖方式

- **上帝分支**: PR 源分支名为 `king/*` 或 `main` 时自动放行（仅维护者）
- **PR 标签**: 添加标签 `override-l0`（仅维护者）
- **覆盖文件**: 在分支加入 `.github/OVERRIDE-L0` 或 `.kernel-override`

### 注意事项

- Override 只跳过"拦截"检查，不豁免审批与测试义务
- 建议事后补齐审批表与性能基准报告
- 上帝分支（`king/*`）最适合架构师直接操作，无需手动配置

---

## 审批要求

| 层级 | 审批人数 | 要求 | 提交前缀 |
|------|---------|------|---------|
| L0 | 3人 | 架构师 + 2核心开发者 | `[L0-CRITICAL]` |
| L1 | 1人 | 1核心开发者 | `[L1-CORE]` |
| L2 | 标准 | 标准 Code Review | `feat:` / `fix:` |
| L3 | 独立 | 独立仓库审批 | `plugin:` |

---

## 开发工具

### 1. 安装 Git Hooks
```powershell
# 自动检测 L0/L1 修改
.\scripts\install-git-hooks.ps1
```

### 2. 手动验证脚本
```bash
# 运行纯净度检查
bash scripts/verify-kernel-purity.sh
```

### 3. CI/CD 自动检查
- GitHub Actions: `.github/workflows/kernel-purity-check.yml`
- 每次 PR 自动触发

---

## 禁止依赖

**L0 内核绝对禁止**:
```toml
# ❌ 禁止
revm = "*"         # EVM 执行引擎
ethers = "*"       # 以太坊库
web3 = "*"         # Web3 库
tokio = "*"        # 异步运行时 (内核同步)
async-std = "*"    # 异步运行时

# ✅ 允许
wasmtime = "17.0"  # WASM 运行时
parking_lot = "*"  # 同步原语
crossbeam = "*"    # 并发工具
```

---

## 性能基准

**硬件**: Intel Xeon 8255C 64核128线程

| 场景 | 基准 TPS | 容忍退化 |
|------|---------|---------|
| 低竞争 (8通道) | 187,000 | < 5% (178K+) |
| 高竞争 (单通道) | 85,000 | < 5% (81K+) |
| WASM 执行 | ~10μs | < 10% |
| MVCC 读延迟 | <1μs | < 20% |

---

## Commit Message 格式

### L0 修改
```
[L0-CRITICAL] <type>: <subject>

<body>

Performance Impact:
- Low Contention: 187K → 185K TPS (-1.07%)
- High Contention: 85K → 84K TPS (-1.18%)

Approval:
- Architect: @username
- Core Dev 1: @username
- Core Dev 2: @username

Refs: #123
```

### L1 修改
```
[L1-CORE] <type>: <subject>

<body>

Feature: feature-name
Performance: No regression when feature disabled

Approval:
- Core Dev: @username

Refs: #456
```

---

## 相关文档

- **完整定义**: [docs/KERNEL-DEFINITION.md](../docs/KERNEL-DEFINITION.md)
- **L0 申请表**: [.github/ISSUE_TEMPLATE/L0-modification-request.md](../.github/ISSUE_TEMPLATE/L0-modification-request.md)
- **L1 申请表**: [.github/ISSUE_TEMPLATE/L1-modification-request.md](../.github/ISSUE_TEMPLATE/L1-modification-request.md)
- **EVM 插件设计**: [docs/evm-adapter-design.md](../docs/evm-adapter-design.md)
- **架构文档**: [docs/architecture-2.0.md](../docs/architecture-2.0.md)

---

## 常见问题

### Q: 如何判断是否需要修改 L0?
**A**: 只有当功能必须在 runtime/parallel/mvcc 核心路径实现时才修改 L0。
如果可以通过扩展/插件实现,应该使用 L1/L2/L3 层。

### Q: 添加新功能应该放在哪层?
```
判断树:
1. 需要修改 WASM 执行/并行调度/MVCC? → L0 (需要审批)
2. 需要内核 API 且通用性强? → L1 (用 feature flag)
3. 是执行引擎变体(如 EVM)? → L3 (独立插件)
4. 是业务逻辑? → L4 (应用层)
```

### Q: 性能优化算 L0 修改吗?
**A**: 是的。任何修改 L0 文件的 PR 都需要走 L0 审批流程,
即使是优化也要证明无性能退化。

### Q: 我是架构师，每次都要填申请表吗?
**A**: 不需要！使用"上帝分支"即可：
```powershell
git checkout -b king/your-feature
# 自动放行，无需手动配置
```

### Q: 如何快速验证纯净度?
```bash
# 快速检查
cargo build -p vm-runtime --no-default-features

# 完整验证
bash scripts/verify-kernel-purity.sh
```

---

## 需要帮助?

- 📖 阅读: [docs/KERNEL-DEFINITION.md](../docs/KERNEL-DEFINITION.md)
- 💬 讨论: GitHub Discussions
- 🐛 报告: GitHub Issues

---

<div align="center">

**保持内核纯净,性能才能持续优化!** 🚀

</div>
