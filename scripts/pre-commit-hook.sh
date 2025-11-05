#!/bin/bash
# SuperVM Pre-commit Hook
# 自动检测 L0/L1 内核修改并提醒开发者

# 颜色定义
RED='\033[0;31m'
YELLOW='\033[1;33m'
GREEN='\033[0;32m'
BLUE='\033[0;34m'
NC='\033[0m'

# 允许造物主/架构师覆盖的通道（本地，仅限维护者）
# 1) 环境变量：SUPERVM_OVERRIDE=1
# 2) Git 配置：git config supervm.override true
# 3) 覆盖文件：在仓库根目录创建 .kernel-override （临时）
# 4) 分支名自动放行：king/* 或直接在 main 分支（仅限维护者）

# 维护者白名单文件
MAINTAINERS_FILE=".github/MAINTAINERS"

# 当前提交作者信息
GIT_EMAIL=$(git config user.email || echo "")
GIT_NAME=$(git config user.name || echo "")
CURRENT_BRANCH=$(git rev-parse --abbrev-ref HEAD 2>/dev/null || echo "")

is_maintainer() {
    if [ -f "$MAINTAINERS_FILE" ]; then
        # 简单匹配邮箱或名字
        if grep -qi "$GIT_EMAIL" "$MAINTAINERS_FILE" || grep -qi "$GIT_NAME" "$MAINTAINERS_FILE"; then
            return 0
        fi
    fi
    return 1
}

# 检查是否为上帝分支（king/* 或 main）
is_god_branch() {
    if [[ "$CURRENT_BRANCH" =~ ^king/ ]] || [ "$CURRENT_BRANCH" = "main" ]; then
        return 0
    fi
    return 1
}

# 覆盖开关检测
OVERRIDE_ENABLED=0
OVERRIDE_REASON=""

if [ "$SUPERVM_OVERRIDE" = "1" ]; then
    OVERRIDE_ENABLED=1
    OVERRIDE_REASON="env SUPERVM_OVERRIDE=1"
elif [ "$(git config --get supervm.override)" = "true" ]; then
    OVERRIDE_ENABLED=1
    OVERRIDE_REASON="git config supervm.override=true"
elif [ -f ".kernel-override" ]; then
    OVERRIDE_ENABLED=1
    OVERRIDE_REASON="file .kernel-override"
elif is_god_branch; then
    OVERRIDE_ENABLED=1
    OVERRIDE_REASON="god branch: $CURRENT_BRANCH"
fi

if [ "$OVERRIDE_ENABLED" = "1" ]; then
    if is_maintainer; then
        echo -e "${YELLOW}⚠️  OVERRIDE ENABLED by maintainer (${GIT_NAME} <${GIT_EMAIL}>)${NC}"
        echo -e "${YELLOW}Reason: ${OVERRIDE_REASON}${NC}"
        echo -e "${YELLOW}Skipping kernel protection checks for this commit...${NC}"
        exit 0
    else
        echo -e "${RED}❌ OVERRIDE DENIED: current user is not in .github/MAINTAINERS${NC}"
        echo -e "${BLUE}Tip:${NC} Ask an architect/core maintainer to perform the override, or remove override flags."
        exit 1
    fi
fi

# L0 核心文件模式
L0_PATTERNS=(
    "src/vm-runtime/src/lib.rs"
    "src/vm-runtime/src/runtime.rs"
    "src/vm-runtime/src/wasm_executor.rs"
    "src/vm-runtime/src/storage.rs"
    "src/vm-runtime/src/storage_api.rs"
    "src/vm-runtime/src/chain_api.rs"
    "src/vm-runtime/src/parallel/"
    "src/vm-runtime/src/mvcc/"
    "src/vm-runtime/src/parallel_mvcc/"
)

# L1 扩展文件模式
L1_PATTERNS=(
    "src/vm-runtime/src/ownership.rs"
    "src/vm-runtime/src/supervm.rs"
    "src/vm-runtime/src/execution_trait.rs"
)

# 检查暂存的文件
STAGED_FILES=$(git diff --cached --name-only)

# 检测 L0 修改
L0_MODIFIED=0
L0_FILES_MODIFIED=""

for pattern in "${L0_PATTERNS[@]}"; do
    MATCHES=$(echo "$STAGED_FILES" | grep "^$pattern" || true)
    if [ -n "$MATCHES" ]; then
        L0_MODIFIED=1
        L0_FILES_MODIFIED="$L0_FILES_MODIFIED
$MATCHES"
    fi
done

# 检测 L1 修改
L1_MODIFIED=0
L1_FILES_MODIFIED=""

for pattern in "${L1_PATTERNS[@]}"; do
    MATCHES=$(echo "$STAGED_FILES" | grep "^$pattern" || true)
    if [ -n "$MATCHES" ]; then
        L1_MODIFIED=1
        L1_FILES_MODIFIED="$L1_FILES_MODIFIED
$MATCHES"
    fi
done

# 检测依赖修改
CARGO_MODIFIED=0
if echo "$STAGED_FILES" | grep -q "src/vm-runtime/Cargo.toml"; then
    CARGO_MODIFIED=1
fi

# 如果有 L0 修改,显示严重警告
if [ "$L0_MODIFIED" -eq 1 ]; then
    echo ""
    echo -e "${RED}╔════════════════════════════════════════════════════════╗${NC}"
    echo -e "${RED}║  ⚠️  CRITICAL: L0 KERNEL MODIFICATION DETECTED  ⚠️      ║${NC}"
    echo -e "${RED}╚════════════════════════════════════════════════════════╝${NC}"
    echo ""
    echo -e "${YELLOW}You are about to commit changes to L0 core kernel files:${NC}"
    echo "$L0_FILES_MODIFIED" | sed 's/^/  📄 /'
    echo ""
    echo -e "${RED}❗ MANDATORY REQUIREMENTS:${NC}"
    echo "  1. ✍️  Fill out L0 modification request form"
    echo "  2. ✅ Get approval from architect + 2 core developers"
    echo "  3. 🧪 Run full test suite:"
    echo "      cargo test --workspace"
    echo "  4. ⚡ Run benchmarks:"
    echo "      cargo bench --bench parallel_execution"
    echo "      cargo bench --bench mvcc_throughput"
    echo "  5. 📊 Verify no performance regression (< 5%)"
    echo "  6. 📝 Update CHANGELOG.md with [L0-CRITICAL] tag"
    echo ""
    echo -e "${BLUE}📖 Documentation: docs/KERNEL-DEFINITION.md${NC}"
    echo ""
    
    # 强制确认
    read -p "$(echo -e ${YELLOW}Have you completed ALL L0 approval requirements? [yes/no]: ${NC})" answer
    
    if [ "$answer" != "yes" ]; then
        echo ""
        echo -e "${RED}❌ COMMIT BLOCKED${NC}"
        echo "   Complete L0 approval process before committing"
        echo "   See: docs/KERNEL-DEFINITION.md Section 4.1"
        echo ""
        exit 1
    fi
    
    # 二次确认
    read -p "$(echo -e ${RED}Are you ABSOLUTELY SURE? This modifies core kernel [yes/no]: ${NC})" confirm
    
    if [ "$confirm" != "yes" ]; then
        echo ""
        echo -e "${YELLOW}⚠️  COMMIT CANCELLED${NC}"
        exit 1
    fi
    
    echo ""
    echo -e "${GREEN}✅ L0 approval confirmed, proceeding...${NC}"
fi

# 如果有 L1 修改,显示警告
if [ "$L1_MODIFIED" -eq 1 ]; then
    echo ""
    echo -e "${YELLOW}╔════════════════════════════════════════════════════════╗${NC}"
    echo -e "${YELLOW}║  ⚠️  L1 EXTENSION MODIFICATION DETECTED  ⚠️            ║${NC}"
    echo -e "${YELLOW}╚════════════════════════════════════════════════════════╝${NC}"
    echo ""
    echo "Modified L1 extension files:"
    echo "$L1_FILES_MODIFIED" | sed 's/^/  📄 /'
    echo ""
    echo -e "${YELLOW}❗ REQUIREMENTS:${NC}"
    echo "  1. Fill out L1 modification request form"
    echo "  2. Ensure feature flag control"
    echo "  3. Get approval from 1 core developer"
    echo "  4. Run tests: cargo test --features <feature-name>"
    echo "  5. Update documentation"
    echo ""
    
    read -p "$(echo -e ${YELLOW}Have you completed L1 approval process? [yes/no]: ${NC})" answer
    
    if [ "$answer" != "yes" ]; then
        echo ""
        echo -e "${YELLOW}⚠️  COMMIT CANCELLED${NC}"
        echo "   Complete L1 approval process first"
        exit 1
    fi
fi

# 如果有依赖修改
if [ "$CARGO_MODIFIED" -eq 1 ]; then
    echo ""
    echo -e "${RED}╔════════════════════════════════════════════════════════╗${NC}"
    echo -e "${RED}║  ⚠️  DEPENDENCY MODIFICATION DETECTED  ⚠️              ║${NC}"
    echo -e "${RED}╚════════════════════════════════════════════════════════╝${NC}"
    echo ""
    echo -e "${YELLOW}vm-runtime/Cargo.toml has been modified${NC}"
    echo ""
    echo "Modified dependencies:"
    git diff --cached src/vm-runtime/Cargo.toml | grep "^[+-]" | grep -v "^[+-][+-][+-]" || true
    echo ""
    echo -e "${RED}❗ JUSTIFICATION REQUIRED:${NC}"
    echo "  1. Why is this dependency necessary?"
    echo "  2. Can it be moved to a plugin?"
    echo "  3. Impact on compile time?"
    echo "  4. Impact on binary size?"
    echo "  5. Is it L0-critical?"
    echo ""
    
    read -p "$(echo -e ${RED}Dependencies justified and approved? [yes/no]: ${NC})" answer
    
    if [ "$answer" != "yes" ]; then
        echo ""
        echo -e "${RED}❌ COMMIT BLOCKED${NC}"
        echo "   Dependency changes require justification"
        exit 1
    fi
fi

# 自动运行快速测试(仅内核修改时)
if [ "$L0_MODIFIED" -eq 1 ] || [ "$L1_MODIFIED" -eq 1 ]; then
    echo ""
    echo -e "${BLUE}🧪 Running quick kernel tests...${NC}"
    
    if ! cargo test -p vm-runtime --quiet 2>&1 | tail -1; then
        echo ""
        echo -e "${RED}❌ TESTS FAILED${NC}"
        echo "   Fix tests before committing"
        exit 1
    fi
    
    echo -e "${GREEN}✅ Quick tests passed${NC}"
fi

# 提醒 commit message 格式
if [ "$L0_MODIFIED" -eq 1 ]; then
    echo ""
    echo -e "${BLUE}📝 Commit Message Reminder:${NC}"
    echo "   Use format: [L0-CRITICAL] <type>: <subject>"
    echo "   Example: [L0-CRITICAL] perf: optimize MVCC read path"
    echo ""
elif [ "$L1_MODIFIED" -eq 1 ]; then
    echo ""
    echo -e "${BLUE}📝 Commit Message Reminder:${NC}"
    echo "   Use format: [L1-CORE] <type>: <subject>"
    echo "   Example: [L1-CORE] feat: add ownership transfer API"
    echo ""
fi

echo -e "${GREEN}✅ Pre-commit checks passed${NC}"
echo ""

exit 0
