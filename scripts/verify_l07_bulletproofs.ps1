# L0.7 Bulletproofs Range Proof 集成验证脚本
# 日期: 2025-11-11

Write-Host "=== L0.7 Bulletproofs Range Proof 集成验证 ===" -ForegroundColor Cyan
Write-Host ""

# 任务1: 编译并运行单元测试
Write-Host "[1/3] 运行 Bulletproofs 单元测试..." -ForegroundColor Yellow
Write-Host ""

try {
    $start = Get-Date
    Set-Location zk-groth16-test
    cargo test --lib bulletproofs --release 2>&1 | Tee-Object -FilePath "../bulletproofs_test_output.txt"
    $end = Get-Date
    $duration = ($end - $start).TotalSeconds
    
    Write-Host ""
    Write-Host "✓ 单元测试完成 (耗时: $duration 秒)" -ForegroundColor Green
    Write-Host "  输出已保存至: bulletproofs_test_output.txt" -ForegroundColor Gray
} catch {
    Write-Host "✗ 单元测试失败: $_" -ForegroundColor Red
    Set-Location ..
    exit 1
}

Write-Host ""
Write-Host "─────────────────────────────────────────────────" -ForegroundColor Gray
Write-Host ""

# 任务2: 运行性能对比示例
Write-Host "[2/3] 运行 Groth16 vs Bulletproofs 性能对比..." -ForegroundColor Yellow
Write-Host ""

try {
    $start = Get-Date
    cargo run --release --example compare_range_proofs 2>&1 | Tee-Object -FilePath "../bulletproofs_compare_output.txt"
    $end = Get-Date
    $duration = ($end - $start).TotalSeconds
    
    Write-Host ""
    Write-Host "✓ 性能对比完成 (耗时: $duration 秒)" -ForegroundColor Green
    Write-Host "  输出已保存至: bulletproofs_compare_output.txt" -ForegroundColor Gray
} catch {
    Write-Host "✗ 性能对比失败: $_" -ForegroundColor Red
    Set-Location ..
    exit 1
}

Write-Host ""
Write-Host "─────────────────────────────────────────────────" -ForegroundColor Gray
Write-Host ""

# 任务3: 运行性能基准测试
Write-Host "[3/3] 运行 Bulletproofs 性能基准 (Criterion)..." -ForegroundColor Yellow
Write-Host ""

try {
    # 检查是否有bulletproofs基准
    if (Test-Path "benches/bulletproofs_bench.rs") {
        $start = Get-Date
        cargo bench bulletproofs 2>&1 | Tee-Object -FilePath "../bulletproofs_bench_output.txt"
        $end = Get-Date
        $duration = ($end - $start).TotalSeconds
        
        Write-Host ""
        Write-Host "✓ 基准测试完成 (耗时: $duration 秒)" -ForegroundColor Green
        Write-Host "  输出已保存至: bulletproofs_bench_output.txt" -ForegroundColor Gray
    } else {
        Write-Host "⊘ 跳过基准测试 (benches/bulletproofs_bench.rs 不存在)" -ForegroundColor Yellow
    }
} catch {
    Write-Host "⚠ 基准测试失败 (非致命): $_" -ForegroundColor Yellow
}

Set-Location ..

Write-Host ""
Write-Host "─────────────────────────────────────────────────" -ForegroundColor Gray
Write-Host ""

# 总结
Write-Host "【L0.7 Bulletproofs 集成总结】" -ForegroundColor Cyan
Write-Host ""
Write-Host "✓ 所有核心测试完成" -ForegroundColor Green
Write-Host ""
Write-Host "验收标准检查:" -ForegroundColor Yellow
Write-Host "  [ ] Bulletproofs依赖编译通过" -ForegroundColor Gray
Write-Host "  [ ] 64-bit Range Proof生成/验证功能正常" -ForegroundColor Gray
Write-Host "  [ ] 批量验证性能优于单个验证" -ForegroundColor Gray
Write-Host "  [ ] 与Groth16性能对比基准完成" -ForegroundColor Gray
Write-Host "  [ ] 所有单元测试通过" -ForegroundColor Gray
Write-Host ""
Write-Host "请检查输出文件:" -ForegroundColor Yellow
Write-Host "  • bulletproofs_test_output.txt (单元测试)" -ForegroundColor Cyan
Write-Host "  • bulletproofs_compare_output.txt (性能对比)" -ForegroundColor Cyan
Write-Host "  • bulletproofs_bench_output.txt (基准测试)" -ForegroundColor Cyan
Write-Host ""
Write-Host "关键性能指标 (预期):" -ForegroundColor Yellow
Write-Host "  • 证明时间: <10ms" -ForegroundColor Gray
Write-Host "  • 验证时间: <15ms" -ForegroundColor Gray
Write-Host "  • 证明大小: ~672-736 bytes" -ForegroundColor Gray
Write-Host "  • 批量验证加速: >3x" -ForegroundColor Gray
Write-Host ""
Write-Host "如果测试通过，L0.7进度将更新至98%" -ForegroundColor Green
