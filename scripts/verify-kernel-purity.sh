#!/bin/bash
# SuperVM 内核纯净性验证脚本
# 用于验证代码修改是否违反内核保护规则

set -e

# 颜色定义
RED='\033[0;31m'
YELLOW='\033[1;33m'
GREEN='\033[0;32m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

echo -e "${BLUE}╔════════════════════════════════════════════════════════╗${NC}"
echo -e "${BLUE}║  🔍 SuperVM Kernel Purity Verification               ║${NC}"
echo -e "${BLUE}╚════════════════════════════════════════════════════════╝${NC}"
echo ""

# L0 核心文件列表
L0_FILES=(
    "src/vm-runtime/src/lib.rs"
    "src/vm-runtime/src/runtime.rs"
    "src/vm-runtime/src/wasm_executor.rs"
    "src/vm-runtime/src/storage.rs"
    "src/vm-runtime/src/storage_api.rs"
    "src/vm-runtime/src/chain_api.rs"
    "src/vm-runtime/src/parallel"
    "src/vm-runtime/src/mvcc"
    "src/vm-runtime/src/parallel_mvcc"
)

# L1 扩展文件列表
L1_FILES=(
    "src/vm-runtime/src/ownership.rs"
    "src/vm-runtime/src/supervm.rs"
    "src/vm-runtime/src/execution_trait.rs"
)

# 检查结果
HAS_L0_CHANGES=0
HAS_L1_CHANGES=0
HAS_DEPENDENCY_CHANGES=0
WARNINGS=0
ERRORS=0

echo -e "${BLUE}📂 Step 1: Checking file modifications...${NC}"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"

# 检查 L0 文件修改
for file in "${L0_FILES[@]}"; do
    if [ -d "$file" ]; then
        # 目录,检查目录下所有文件
        MODIFIED=$(git diff --name-only HEAD | grep "^$file/" || true)
    else
        # 单个文件
        MODIFIED=$(git diff --name-only HEAD | grep "^$file$" || true)
    fi
    
    if [ -n "$MODIFIED" ]; then
        echo -e "${RED}⚠️  L0 CRITICAL: $file${NC}"
        HAS_L0_CHANGES=1
        WARNINGS=$((WARNINGS + 1))
    fi
done

# 检查 L1 文件修改
for file in "${L1_FILES[@]}"; do
    MODIFIED=$(git diff --name-only HEAD | grep "^$file" || true)
    if [ -n "$MODIFIED" ]; then
        echo -e "${YELLOW}⚠️  L1 CORE: $file${NC}"
        HAS_L1_CHANGES=1
        WARNINGS=$((WARNINGS + 1))
    fi
done

if [ "$HAS_L0_CHANGES" -eq 0 ] && [ "$HAS_L1_CHANGES" -eq 0 ]; then
    echo -e "${GREEN}✅ No kernel file modifications detected${NC}"
fi

echo ""
echo -e "${BLUE}📦 Step 2: Checking dependencies...${NC}"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"

# 检查 Cargo.toml 修改
if git diff --name-only HEAD | grep -q "src/vm-runtime/Cargo.toml"; then
    echo -e "${RED}⚠️  vm-runtime/Cargo.toml modified!${NC}"
    echo "   Dependencies changes require L0 approval"
    HAS_DEPENDENCY_CHANGES=1
    WARNINGS=$((WARNINGS + 1))
    
    # 显示具体修改
    echo ""
    echo "   Modified dependencies:"
    git diff HEAD src/vm-runtime/Cargo.toml | grep "^[+-]" | grep -v "^[+-][+-][+-]" || true
    echo ""
fi

# 检查是否引入禁止的依赖
echo "Checking for forbidden dependencies..."
if cargo tree -p vm-runtime 2>/dev/null | grep -q "revm"; then
    echo -e "${RED}❌ FORBIDDEN: revm dependency found in vm-runtime!${NC}"
    echo "   EVM adapter must be in separate crate"
    ERRORS=$((ERRORS + 1))
fi

if cargo tree -p vm-runtime 2>/dev/null | grep -q "tokio"; then
    echo -e "${YELLOW}⚠️  WARNING: tokio found in vm-runtime${NC}"
    echo "   Consider if async is necessary in kernel"
    WARNINGS=$((WARNINGS + 1))
fi

# 统计依赖数量
CORE_DEPS=$(cargo tree -p vm-runtime --depth 1 2>/dev/null | wc -l || echo 0)
if [ "$CORE_DEPS" -gt 20 ]; then
    echo -e "${YELLOW}⚠️  WARNING: Too many dependencies ($CORE_DEPS > 20)${NC}"
    echo "   Kernel should have minimal dependencies"
    WARNINGS=$((WARNINGS + 1))
else
    echo -e "${GREEN}✅ Dependency count OK ($CORE_DEPS <= 20)${NC}"
fi

echo ""
echo -e "${BLUE}🔨 Step 3: Building pure kernel...${NC}"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"

if cargo build -p vm-runtime --no-default-features --quiet 2>&1; then
    echo -e "${GREEN}✅ Pure kernel builds successfully${NC}"
else
    echo -e "${RED}❌ FAILED: Pure kernel build failed${NC}"
    ERRORS=$((ERRORS + 1))
fi

echo ""
echo -e "${BLUE}🧪 Step 4: Running kernel tests...${NC}"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"

if cargo test -p vm-runtime --no-default-features --quiet 2>&1; then
    echo -e "${GREEN}✅ Kernel tests passed${NC}"
else
    echo -e "${RED}❌ FAILED: Kernel tests failed${NC}"
    ERRORS=$((ERRORS + 1))
fi

# 性能基准测试(可选,仅当有修改时运行)
if [ "$HAS_L0_CHANGES" -eq 1 ]; then
    echo ""
    echo -e "${BLUE}⚡ Step 5: Performance check recommended${NC}"
    echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
    echo -e "${YELLOW}⚠️  L0 modifications detected${NC}"
    echo "   Please run: cargo bench --bench parallel_execution"
    echo "   Verify no performance regression (< 5%)"
    WARNINGS=$((WARNINGS + 1))
fi

# 生成报告
echo ""
echo -e "${BLUE}╔════════════════════════════════════════════════════════╗${NC}"
echo -e "${BLUE}║  📊 Verification Report                               ║${NC}"
echo -e "${BLUE}╚════════════════════════════════════════════════════════╝${NC}"
echo ""

if [ "$HAS_L0_CHANGES" -eq 1 ]; then
    echo -e "${RED}⚠️  L0 KERNEL MODIFICATIONS DETECTED${NC}"
    echo ""
    echo "   ❗ REQUIRED ACTIONS:"
    echo "   1. Fill out L0 modification request form"
    echo "   2. Get approval from:"
    echo "      - Architect: KING XU"
    echo "      - Core Developer 1: ___________"
    echo "      - Core Developer 2: ___________"
    echo "   3. Run full test suite:"
    echo "      cargo test --workspace"
    echo "   4. Run benchmarks:"
    echo "      cargo bench"
    echo "   5. Verify no performance regression"
    echo ""
    echo "   📖 See: docs/KERNEL-DEFINITION.md Section 4"
    echo ""
fi

if [ "$HAS_L1_CHANGES" -eq 1 ]; then
    echo -e "${YELLOW}⚠️  L1 EXTENSION MODIFICATIONS DETECTED${NC}"
    echo ""
    echo "   ❗ REQUIRED ACTIONS:"
    echo "   1. Fill out L1 modification request form"
    echo "   2. Ensure feature flag control"
    echo "   3. Get approval from 1 core developer"
    echo "   4. Update documentation"
    echo ""
    echo "   📖 See: docs/KERNEL-DEFINITION.md Section 3.1"
    echo ""
fi

if [ "$HAS_DEPENDENCY_CHANGES" -eq 1 ]; then
    echo -e "${RED}⚠️  DEPENDENCY CHANGES DETECTED${NC}"
    echo ""
    echo "   ❗ JUSTIFICATION REQUIRED:"
    echo "   - Why is this dependency necessary?"
    echo "   - Can it be moved to a plugin?"
    echo "   - What is the impact on compile time?"
    echo "   - What is the impact on binary size?"
    echo ""
fi

# 总结
echo ""
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo -e "Warnings: ${YELLOW}${WARNINGS}${NC}"
echo -e "Errors:   ${RED}${ERRORS}${NC}"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"

if [ "$ERRORS" -gt 0 ]; then
    echo ""
    echo -e "${RED}❌ VERIFICATION FAILED${NC}"
    echo "   Please fix errors before committing"
    exit 1
elif [ "$WARNINGS" -gt 0 ]; then
    echo ""
    echo -e "${YELLOW}⚠️  VERIFICATION PASSED WITH WARNINGS${NC}"
    echo "   Review warnings and complete approval process"
    exit 0
else
    echo ""
    echo -e "${GREEN}✅ VERIFICATION PASSED${NC}"
    echo "   No kernel modifications detected"
    exit 0
fi
