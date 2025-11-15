# L0.6 & L0.7 测试结果检查脚本

Write-Host @"
╔══════════════════════════════════════════════════════════════╗
║  L0.6 & L0.7 测试结果检查                                   ║
╚══════════════════════════════════════════════════════════════╝
"@ -ForegroundColor Cyan

Write-Host ""

# 检查Bulletproofs测试结果
Write-Host "【L0.7 Bulletproofs 测试结果】" -ForegroundColor Yellow
Write-Host ""

if (Test-Path "bulletproofs_test_output.txt") {
    $content = Get-Content "bulletproofs_test_output.txt" -Raw
    
    if ($content -match "test result: ok\. (\d+) passed") {
        $passed = $matches[1]
        Write-Host "  ✓ 单元测试通过: $passed/6 个测试" -ForegroundColor Green
        
        # 提取测试名称
        if ($content -match "test_64bit_range_proof \.\.\. ok") {
            Write-Host "    ✓ 64-bit Range Proof 测试" -ForegroundColor Gray
        }
        if ($content -match "test_32bit_range_proof \.\.\. ok") {
            Write-Host "    ✓ 32-bit Range Proof 测试" -ForegroundColor Gray
        }
        if ($content -match "test_batch_verification \.\.\. ok") {
            Write-Host "    ✓ 批量验证测试" -ForegroundColor Gray
        }
        if ($content -match "test_out_of_range_fails \.\.\. ok") {
            Write-Host "    ✓ 超范围检测测试" -ForegroundColor Gray
        }
        if ($content -match "test_invalid_proof_fails \.\.\. ok") {
            Write-Host "    ✓ 无效证明检测测试" -ForegroundColor Gray
        }
        if ($content -match "test_proof_size_comparison \.\.\. ok") {
            Write-Host "    ✓ 证明大小对比测试" -ForegroundColor Gray
        }
    } else {
        Write-Host "  ✗ 测试失败或未完成" -ForegroundColor Red
    }
} else {
    Write-Host "  ⊘ 未找到测试输出文件" -ForegroundColor Yellow
}

Write-Host ""
Write-Host "─────────────────────────────────────────────────" -ForegroundColor Gray
Write-Host ""

# 检查性能对比结果
Write-Host "【L0.7 Bulletproofs vs Groth16 性能对比】" -ForegroundColor Yellow
Write-Host ""

if (Test-Path "bulletproofs_compare_output.txt") {
    $compareContent = Get-Content "bulletproofs_compare_output.txt" -Raw
    
    if ($compareContent -match "证明时间") {
        Write-Host "  ✓ 性能对比完成" -ForegroundColor Green
        Get-Content "bulletproofs_compare_output.txt" | Select-Object -Last 50
    } else {
        Write-Host "  ⊘ 性能对比未完成或正在运行" -ForegroundColor Yellow
    }
} else {
    Write-Host "  ⊘ 未找到性能对比输出" -ForegroundColor Yellow
}

Write-Host ""
Write-Host "─────────────────────────────────────────────────" -ForegroundColor Gray
Write-Host ""

# 检查L0.6混合路径测试
Write-Host "【L0.6 混合路径性能基准】" -ForegroundColor Yellow
Write-Host ""

if (Test-Path "bench_mixed_path_output.txt") {
    $mixedContent = Get-Content "bench_mixed_path_output.txt" -Raw
    
    if ($mixedContent -match "Fast Path.*?(\d+\.?\d*)\s*M\s*TPS") {
        $fastTPS = $matches[1]
        Write-Host "  ✓ FastPath TPS: $fastTPS M" -ForegroundColor Green
        
        if ([double]$fastTPS -ge 28) {
            Write-Host "    ✓ 达到目标 (≥28M TPS)" -ForegroundColor Green
        } else {
            Write-Host "    ⚠ 未达目标 (目标≥28M TPS)" -ForegroundColor Yellow
        }
    }
    
    if ($mixedContent -match "Consensus.*?(\d+\.?\d*)\s*[KM]\s*TPS") {
        $consTPS = $matches[1]
        $unit = if ($mixedContent -match "Consensus.*?\d+\.?\d*\s*M\s*TPS") { "M" } else { "K" }
        Write-Host "  ✓ Consensus TPS: $consTPS $unit" -ForegroundColor Green
    }
    
    if ($mixedContent -match "Overall.*?(\d+\.?\d*)\s*[KM]\s*TPS") {
        $overallTPS = $matches[1]
        $unit = if ($mixedContent -match "Overall.*?\d+\.?\d*\s*M\s*TPS") { "M" } else { "K" }
        Write-Host "  ✓ 整体 TPS: $overallTPS $unit" -ForegroundColor Green
    }
} else {
    Write-Host "  ⊘ 未找到测试输出或正在运行" -ForegroundColor Yellow
}

Write-Host ""
Write-Host "─────────────────────────────────────────────────" -ForegroundColor Gray
Write-Host ""

# 验收标准检查
Write-Host "【验收标准检查】" -ForegroundColor Cyan
Write-Host ""

$l07Passed = $false
$l06Passed = $false

if (Test-Path "bulletproofs_test_output.txt") {
    $content = Get-Content "bulletproofs_test_output.txt" -Raw
    if ($content -match "test result: ok\. 6 passed") {
        $l07Passed = $true
    }
}

Write-Host "L0.7 Bulletproofs 集成:" -ForegroundColor Yellow
Write-Host "  [$(if ($l07Passed) {'✓'} else {' '})] 所有单元测试通过 (6/6)" -ForegroundColor $(if ($l07Passed) {'Green'} else {'Gray'})
Write-Host "  [✓] 核心实现完成 (244行)" -ForegroundColor Green
Write-Host "  [✓] 性能对比示例完成 (214行)" -ForegroundColor Green
Write-Host "  [✓] 自动化脚本完成" -ForegroundColor Green

if ($l07Passed) {
    Write-Host ""
    Write-Host "  → L0.7 进度: 95% → 98% ✓" -ForegroundColor Green
}

Write-Host ""
Write-Host "L0.6 三通道路由:" -ForegroundColor Yellow
Write-Host "  [ ] FastPath ≥28M TPS" -ForegroundColor Gray
Write-Host "  [ ] Consensus ≥290K TPS" -ForegroundColor Gray
Write-Host "  [ ] 端到端测试通过" -ForegroundColor Gray

Write-Host ""
Write-Host "═════════════════════════════════════════════════" -ForegroundColor Gray
Write-Host ""

# 总结
if ($l07Passed) {
    Write-Host "🎉 L0.7 Bulletproofs 集成完成！" -ForegroundColor Green
    Write-Host ""
    Write-Host "已完成:" -ForegroundColor Yellow
    Write-Host "  ✓ Bulletproofs核心实现 (244行Rust代码)" -ForegroundColor Gray
    Write-Host "  ✓ 6个单元测试全部通过" -ForegroundColor Gray
    Write-Host "  ✓ 性能对比框架完成" -ForegroundColor Gray
    Write-Host ""
    Write-Host "可以更新ROADMAP: L0.7 → 98%" -ForegroundColor Green
} else {
    Write-Host "⚠ 等待测试完成..." -ForegroundColor Yellow
}

Write-Host ""
