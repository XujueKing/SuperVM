# L0.6 三通道路由性能验证脚本
# 日期: 2025-11-11

Write-Host "=== L0.6 三通道路由性能验证 ===" -ForegroundColor Cyan
Write-Host ""

# 任务1: 运行混合路径性能基准
Write-Host "[1/2] 运行 mixed_path_bench (目标: FastPath 28M TPS)..." -ForegroundColor Yellow
Write-Host "参数: MIXED_ITERS=200000, OWNED_RATIO=0.80" -ForegroundColor Gray
Write-Host ""

$env:MIXED_ITERS = "200000"
$env:OWNED_RATIO = "0.80"
$env:SEED = "2025"

try {
    $start = Get-Date
    cargo run --release --example mixed_path_bench 2>&1 | Tee-Object -FilePath "bench_mixed_path_output.txt"
    $end = Get-Date
    $duration = ($end - $start).TotalSeconds
    
    Write-Host ""
    Write-Host "✓ mixed_path_bench 完成 (耗时: $duration 秒)" -ForegroundColor Green
    Write-Host "  输出已保存至: bench_mixed_path_output.txt" -ForegroundColor Gray
} catch {
    Write-Host "✗ mixed_path_bench 失败: $_" -ForegroundColor Red
    exit 1
}

Write-Host ""
Write-Host "─────────────────────────────────────────────────" -ForegroundColor Gray
Write-Host ""

# 任务2: 运行端到端三通道测试
Write-Host "[2/2] 运行 e2e_three_channel_test (端到端稳定性验证)..." -ForegroundColor Yellow
Write-Host ""

try {
    $start = Get-Date
    cargo run --release --example e2e_three_channel_test 2>&1 | Tee-Object -FilePath "e2e_three_channel_output.txt"
    $end = Get-Date
    $duration = ($end - $start).TotalSeconds
    
    Write-Host ""
    Write-Host "✓ e2e_three_channel_test 完成 (耗时: $duration 秒)" -ForegroundColor Green
    Write-Host "  输出已保存至: e2e_three_channel_output.txt" -ForegroundColor Gray
} catch {
    Write-Host "✗ e2e_three_channel_test 失败: $_" -ForegroundColor Red
    exit 1
}

Write-Host ""
Write-Host "─────────────────────────────────────────────────" -ForegroundColor Gray
Write-Host ""

# 总结
Write-Host "【L0.6 性能验证总结】" -ForegroundColor Cyan
Write-Host ""
Write-Host "✓ 所有测试完成" -ForegroundColor Green
Write-Host ""
Write-Host "验收标准检查:" -ForegroundColor Yellow
Write-Host "  [ ] FastPath 独占对象: ≥28M TPS" -ForegroundColor Gray
Write-Host "  [ ] Consensus 共享对象: ≥290K TPS" -ForegroundColor Gray
Write-Host "  [ ] AdaptiveRouter 自适应调整正常" -ForegroundColor Gray
Write-Host "  [ ] 无运行时错误或panic" -ForegroundColor Gray
Write-Host ""
Write-Host "请检查输出文件中的TPS数据:" -ForegroundColor Yellow
Write-Host "  • bench_mixed_path_output.txt" -ForegroundColor Cyan
Write-Host "  • e2e_three_channel_output.txt" -ForegroundColor Cyan
Write-Host ""
Write-Host "如果测试通过，L0.6进度将更新至100%" -ForegroundColor Green
