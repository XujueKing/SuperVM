# SuperVM 内核保护 - 5分钟上手指南

> **你是架构师/Owner，这是为你量身定制的最简使用指南**

---

## 当前状态检查

你的 Git 身份（已自动识别）:
- 用户名: `KingAI`
- 邮箱: `146935787+XujueKing@users.noreply.github.com`
- 维护者白名单: ✅ 已登记

---

## 最简单的使用方式（3选1）

### 方式1: 上帝分支（推荐，零配置）

```powershell
# 创建 king/ 开头的分支，自动放行
git checkout -b king/your-feature

# 正常开发...
git add .
git commit -m "[L0-CRITICAL] perf: optimize scheduler"

# ✅ 不会被拦截！
```

**适用场景**: 紧急修复、大型重构、架构师日常工作

---

### 方式2: 直接在 main 分支工作

```powershell
# 确保在 main 分支
git checkout main

# 正常提交
git add .
git commit -m "fix: critical bug"

# ✅ main 分支也自动放行
```

**适用场景**: 简单的快速修复

---

### 方式3: 临时环境变量

```powershell
# 仅当前 PowerShell 会话生效
$env:SUPERVM_OVERRIDE = "1"

# 在任意分支提交
git add .
git commit -m "..."

# ✅ 本次会话内所有提交都放行
```

**适用场景**: 在 feature 分支测试时临时使用

---

## 快速测试（可选）

想验证 hook 是否正常工作？试试这个：

```powershell
# 1. 创建测试分支
git checkout -b test-hook

# 2. 随便改个 L0 文件（不要真的改，只是触发检测）
Add-Content src\vm-runtime\src\runtime.rs "`n# test"

# 3. 尝试提交
git add src\vm-runtime\src\runtime.rs
git commit -m "test: trigger L0 warning"

# 应该会看到红色警告并要求确认

# 4. 清理测试
git reset HEAD~1
git checkout src\vm-runtime\src\runtime.rs
git checkout main
git branch -D test-hook
```

---

## 现在就用上帝分支测试

```powershell
# 创建上帝分支
git checkout -b king/test-override

# 同样改个 L0 文件
Add-Content src\vm-runtime\src\runtime.rs "`n# test"

# 提交
git add src\vm-runtime\src\runtime.rs
git commit -m "test: god branch auto-pass"

# ✅ 应该看到黄色提示: "OVERRIDE ENABLED by maintainer"
# ✅ 并自动通过！

# 清理
git reset HEAD~1
git checkout src\vm-runtime\src\runtime.rs
git checkout main
git branch -D king/test-override
```

---

## 身份识别机制

**不是** VS Code 登录账号，而是你的 **Git 配置**:

```powershell
# 查看当前配置
git config user.name
git config user.email

# 如果需要修改（一般不需要）
git config user.name "KingAI"
git config user.email "146935787+XujueKing@users.noreply.github.com"
```

Hook 会读取这两个信息，然后在 `.github/MAINTAINERS` 中匹配。
✅ 你的三个邮箱和两个用户名都已登记，无论哪个都能识别。

---

## 常见场景

### 场景1: 紧急热修复
```powershell
git checkout -b king/hotfix-$(Get-Date -Format 'MMdd')
# 修改代码
git add -A
git commit -m "[L0-CRITICAL] hotfix: xxx"
git push origin king/hotfix-$(Get-Date -Format 'MMdd')
# ✅ 自动放行
```

### 场景2: 日常开发（非内核）
```powershell
# 普通分支也可以用，只要不改 L0/L1 文件就不会触发检查
git checkout -b feature/new-api
# 修改 node-core 或其他非内核代码
git add -A
git commit -m "feat: add new API"
# ✅ 不触发检查，正常提交
```

### 场景3: 架构重构
```powershell
git checkout -b king/refactor-mvcc
# 大规模修改 L0 内核
git add -A
git commit -m "[L0-CRITICAL] refactor: redesign MVCC storage"
# ✅ 自动放行，事后补充性能报告即可
```

---

## 如果识别不了怎么办？

1. **检查 Git 配置**
   ```powershell
   git config user.name
   git config user.email
   ```

2. **检查是否在白名单**
   ```powershell
   Select-String -Path .github\MAINTAINERS -Pattern "$(git config user.email)"
   ```

3. **临时解决（紧急情况）**
   ```powershell
   $env:SUPERVM_OVERRIDE = "1"
   git commit -m "..."
   ```

4. **永久解决（联系我）**
   - 把你的新邮箱/用户名告诉我
   - 我会加到 `.github/MAINTAINERS`

---

## 总结

**你只需记住一条**: 用 `king/*` 分支，什么都不用配置！

```powershell
git checkout -b king/任意名字
# 然后随便改，随便提交
```

就这么简单！🎉
