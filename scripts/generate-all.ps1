# SuperVM 资产生成总控脚本
# 用法: .\scripts\generate-all.ps1

$ErrorActionPreference = "Stop"

Write-Host ""
Write-Host "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━" -ForegroundColor Magenta
Write-Host "   🚀 SuperVM 内容资产生成器" -ForegroundColor Cyan
Write-Host "   潘多拉星核 (Pandora Core) - Web3 操作系统" -ForegroundColor White
Write-Host "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━" -ForegroundColor Magenta
Write-Host ""

$startTime = Get-Date

# 检查依赖
Write-Host "🔍 检查依赖工具..." -ForegroundColor Cyan
Write-Host ""

$hasPandoc = Get-Command pandoc -ErrorAction SilentlyContinue
$hasPython = Get-Command python -ErrorAction SilentlyContinue
$hasMermaid = Get-Command mmdc -ErrorAction SilentlyContinue

if ($hasPandoc) {
    Write-Host "  ✅ Pandoc: 已安装" -ForegroundColor Green
} else {
    Write-Host "  ❌ Pandoc: 未安装 (PDF/PPT 生成将跳过)" -ForegroundColor Yellow
}

if ($hasPython) {
    Write-Host "  ✅ Python: 已安装" -ForegroundColor Green
} else {
    Write-Host "  ⚠️  Python: 未安装 (图表生成将跳过)" -ForegroundColor Yellow
}

if ($hasMermaid) {
    Write-Host "  ✅ Mermaid CLI: 已安装" -ForegroundColor Green
} else {
    Write-Host "  ⚠️  Mermaid CLI: 未安装 (架构图 PNG 将跳过)" -ForegroundColor Yellow
}

Write-Host ""
Start-Sleep -Seconds 2

# 1. 生成视觉资产
Write-Host "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━" -ForegroundColor Cyan
Write-Host "📊 第 1 步: 生成视觉资产" -ForegroundColor Yellow
Write-Host "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━" -ForegroundColor Cyan
Write-Host ""

if (Test-Path ".\scripts\generate-visuals.ps1") {
    & .\scripts\generate-visuals.ps1
} else {
    Write-Host "❌ 未找到 generate-visuals.ps1" -ForegroundColor Red
}

Write-Host ""
Write-Host "按任意键继续..." -ForegroundColor Gray
$null = $Host.UI.RawUI.ReadKey("NoEcho,IncludeKeyDown")
Write-Host ""

# 2. 生成 PDF 文档
Write-Host "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━" -ForegroundColor Cyan
Write-Host "📄 第 2 步: 生成 PDF 文档" -ForegroundColor Yellow
Write-Host "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━" -ForegroundColor Cyan
Write-Host ""

if ($hasPandoc -and (Test-Path ".\scripts\generate-pdfs.ps1")) {
    & .\scripts\generate-pdfs.ps1
} else {
    Write-Host "⚠️  跳过 PDF 生成 (需要 Pandoc)" -ForegroundColor Yellow
}

Write-Host ""
Write-Host "按任意键继续..." -ForegroundColor Gray
$null = $Host.UI.RawUI.ReadKey("NoEcho,IncludeKeyDown")
Write-Host ""

# 3. 生成 PowerPoint
Write-Host "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━" -ForegroundColor Cyan
Write-Host "📊 第 3 步: 生成 PowerPoint 演示" -ForegroundColor Yellow
Write-Host "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━" -ForegroundColor Cyan
Write-Host ""

if ($hasPandoc -and (Test-Path ".\scripts\generate-ppt.ps1")) {
    & .\scripts\generate-ppt.ps1
} else {
    Write-Host "⚠️  跳过 PPT 生成 (需要 Pandoc)" -ForegroundColor Yellow
}

Write-Host ""

# 计算耗时
$endTime = Get-Date
$duration = $endTime - $startTime
$minutes = [math]::Floor($duration.TotalMinutes)
$seconds = $duration.Seconds

# 最终总结
Write-Host ""
Write-Host "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━" -ForegroundColor Magenta
Write-Host "   🎉 所有资产生成完成！" -ForegroundColor Green
Write-Host "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━" -ForegroundColor Magenta
Write-Host ""
Write-Host "⏱️  总耗时: $minutes 分 $seconds 秒" -ForegroundColor Cyan
Write-Host ""
Write-Host "📂 生成的资产:" -ForegroundColor Cyan
Write-Host ""

# 统计文件
$pdfCount = (Get-ChildItem "pdf-output" -Filter *.pdf -ErrorAction SilentlyContinue).Count
$pptCount = (Get-ChildItem "pdf-output" -Filter *.pptx -ErrorAction SilentlyContinue).Count
$chartCount = (Get-ChildItem "visuals/charts" -Filter *.png -ErrorAction SilentlyContinue).Count
$diagramCount = (Get-ChildItem "visuals/diagrams" -ErrorAction SilentlyContinue).Count

Write-Host "  📄 PDF 文档: $pdfCount 个" -ForegroundColor White
Write-Host "  📊 PowerPoint: $pptCount 个" -ForegroundColor White
Write-Host "  📈 图表: $chartCount 个" -ForegroundColor White
Write-Host "  🏗️  架构图: $diagramCount 个源文件" -ForegroundColor White
Write-Host ""
Write-Host "📍 文件位置:" -ForegroundColor Cyan
Write-Host "   • PDF/PPT: $(Resolve-Path 'pdf-output' -ErrorAction SilentlyContinue)" -ForegroundColor White
Write-Host "   • 视觉资产: $(Resolve-Path 'visuals' -ErrorAction SilentlyContinue)" -ForegroundColor White
Write-Host ""

# 下一步建议
Write-Host "🚀 下一步行动:" -ForegroundColor Yellow
Write-Host ""
Write-Host "  1️⃣  审查生成的 PDF 白皮书" -ForegroundColor Cyan
Write-Host "     打开: pdf-output\SuperVM_Whitepaper_CN_v1.0.pdf" -ForegroundColor Gray
Write-Host ""
Write-Host "  2️⃣  编辑 PowerPoint 演示" -ForegroundColor Cyan
Write-Host "     打开: pdf-output\SuperVM_Investor_Pitch_Deck.pptx" -ForegroundColor Gray
Write-Host "     添加: Logo, 品牌配色, 图表" -ForegroundColor Gray
Write-Host ""
Write-Host "  3️⃣  准备社交媒体发布" -ForegroundColor Cyan
Write-Host "     参考: docs\SOCIAL-MEDIA-TEMPLATES.md" -ForegroundColor Gray
Write-Host "     配图: visuals\charts\*.png" -ForegroundColor Gray
Write-Host ""
Write-Host "  4️⃣  设置官方网站" -ForegroundColor Cyan
Write-Host "     上传: 白皮书 PDF, Pitch Deck" -ForegroundColor Gray
Write-Host "     链接: supervm.io/whitepaper, supervm.io/deck" -ForegroundColor Gray
Write-Host ""
Write-Host "  5️⃣  联系投资者" -ForegroundColor Cyan
Write-Host "     发送: SuperVM_Investor_Pitch_Deck.pptx" -ForegroundColor Gray
Write-Host "     附件: 白皮书, GitHub 链接" -ForegroundColor Gray
Write-Host ""

# 缺失工具提示
if (-not $hasPandoc) {
    Write-Host "⚠️  安装 Pandoc 以生成 PDF/PPT:" -ForegroundColor Yellow
    Write-Host "   choco install pandoc" -ForegroundColor Cyan
    Write-Host "   或访问: https://pandoc.org/installing.html" -ForegroundColor Cyan
    Write-Host ""
}

if (-not $hasPython) {
    Write-Host "⚠️  安装 Python 以生成图表:" -ForegroundColor Yellow
    Write-Host "   下载: https://www.python.org/downloads/" -ForegroundColor Cyan
    Write-Host "   安装: pip install matplotlib" -ForegroundColor Cyan
    Write-Host ""
}

if (-not $hasMermaid) {
    Write-Host "⚠️  安装 Mermaid CLI 以生成架构图 PNG:" -ForegroundColor Yellow
    Write-Host "   npm install -g @mermaid-js/mermaid-cli" -ForegroundColor Cyan
    Write-Host ""
}

Write-Host "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━" -ForegroundColor Magenta
Write-Host "   准备好打开 Web3 的潘多拉魔盒了吗? 🚀" -ForegroundColor White
Write-Host "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━" -ForegroundColor Magenta
Write-Host ""
