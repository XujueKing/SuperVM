# 快速执行脚本: L0.6 + L0.7 验证
# 一键运行所有测试

Write-Host @"
╔══════════════════════════════════════════════════════════════╗
║  SuperVM L0.6 & L0.7 集成验证                               ║
║  日期: 2025-11-11                                            ║
╚══════════════════════════════════════════════════════════════╝
"@ -ForegroundColor Cyan

Write-Host ""

# 检查当前目录
if (-not (Test-Path "Cargo.toml")) {
    Write-Host "✗ 错误: 请在项目根目录运行此脚本" -ForegroundColor Red
    exit 1
}

$totalStart = Get-Date

# ========================================
# Part 1: L0.7 Bulletproofs 验证
# ========================================
Write-Host "【Part 1/2】L0.7 Bulletproofs Range Proof 验证" -ForegroundColor Yellow
Write-Host "─────────────────────────────────────────────────" -ForegroundColor Gray
Write-Host ""

try {
    & ".\scripts\verify_l07_bulletproofs.ps1"
    if ($LASTEXITCODE -ne 0) {
        Write-Host "✗ Bulletproofs验证失败" -ForegroundColor Red
        exit 1
    }
} catch {
    Write-Host "✗ Bulletproofs验证异常: $_" -ForegroundColor Red
    exit 1
}

Write-Host ""
Write-Host "════════════════════════════════════════════════════" -ForegroundColor Gray
Write-Host ""

# ========================================
# Part 2: L0.6 三通道路由验证
# ========================================
Write-Host "【Part 2/2】L0.6 三通道路由性能验证" -ForegroundColor Yellow
Write-Host "─────────────────────────────────────────────────" -ForegroundColor Gray
Write-Host ""

try {
    & ".\scripts\verify_l06_performance.ps1"
    if ($LASTEXITCODE -ne 0) {
        Write-Host "✗ 三通道路由验证失败" -ForegroundColor Red
        exit 1
    }
} catch {
    Write-Host "✗ 三通道路由验证异常: $_" -ForegroundColor Red
    exit 1
}

Write-Host ""
Write-Host "════════════════════════════════════════════════════" -ForegroundColor Gray
Write-Host ""

# ========================================
# 总结
# ========================================
$totalEnd = Get-Date
$totalDuration = ($totalEnd - $totalStart).TotalMinutes

Write-Host @"
╔══════════════════════════════════════════════════════════════╗
║  🎉 所有验证完成!                                           ║
╚══════════════════════════════════════════════════════════════╝
"@ -ForegroundColor Green

Write-Host ""
Write-Host "总耗时: $([math]::Round($totalDuration, 2)) 分钟" -ForegroundColor Cyan
Write-Host ""
Write-Host "输出文件:" -ForegroundColor Yellow
Write-Host "  ✓ bulletproofs_test_output.txt" -ForegroundColor Gray
Write-Host "  ✓ bulletproofs_compare_output.txt" -ForegroundColor Gray
Write-Host "  ✓ bench_mixed_path_output.txt" -ForegroundColor Gray
Write-Host "  ✓ e2e_three_channel_output.txt" -ForegroundColor Gray
Write-Host ""
Write-Host "进度报告:" -ForegroundColor Yellow
Write-Host "  ✓ L06-L07-PROGRESS-2025-11-11.md" -ForegroundColor Cyan
Write-Host ""
Write-Host "下一步:" -ForegroundColor Yellow
Write-Host "  1. 检查输出文件中的性能数据" -ForegroundColor Gray
Write-Host "  2. 验证所有测试通过" -ForegroundColor Gray
Write-Host "  3. 更新ROADMAP.md进度:" -ForegroundColor Gray
Write-Host "     • L0.6: 92% → 100%" -ForegroundColor Gray
Write-Host "     • L0.7: 95% → 98%" -ForegroundColor Gray
Write-Host "     • L0整体: 96% → 98%" -ForegroundColor Gray
Write-Host "  4. 提交代码到Git" -ForegroundColor Gray
Write-Host ""
