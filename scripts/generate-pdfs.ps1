# SuperVM PDF Generator
# Usage: .\scripts\generate-pdfs.ps1

$ErrorActionPreference = "Stop"

# Ensure working directory is repository root so 'assets/pdf' resolves correctly
$scriptDir = Split-Path -Parent $MyInvocation.MyCommand.Path
$repoRoot = Split-Path -Parent $scriptDir
Set-Location $repoRoot

Write-Host "Starting SuperVM PDF generation..." -ForegroundColor Cyan
Write-Host ""

# Resolve Pandoc path and ensure availability in current session
function Resolve-PandocPath {
    $cmd = Get-Command pandoc -ErrorAction SilentlyContinue
    if ($cmd -and $cmd.Source) { return $cmd.Source }
    $candidates = @(
        'C:\\Program Files\\Pandoc\\pandoc.exe',
        'C:\\Program Files (x86)\\Pandoc\\pandoc.exe',
        "$env:USERPROFILE\\scoop\\apps\\pandoc\\current\\pandoc.exe",
        "$env:LOCALAPPDATA\\Pandoc\\pandoc.exe"
    )
    foreach ($p in $candidates) { if (Test-Path $p) { return $p } }
    return $null
}

# Resolve XeLaTeX path if not found in PATH (MiKTeX/TeXLive common locations)
function Resolve-XeLaTeXPath {
    $cmd = Get-Command xelatex -ErrorAction SilentlyContinue
    if ($cmd -and $cmd.Source) { return $cmd.Source }
    $candidates = @(
        'C:\\Program Files\\MiKTeX\\miktex\\bin\\x64\\xelatex.exe',
        "$env:LOCALAPPDATA\\Programs\\MiKTeX\\miktex\\bin\\x64\\xelatex.exe",
        'C:\\texlive\\2024\\bin\\win32\\xelatex.exe',
        'C:\\texlive\\2023\\bin\\win32\\xelatex.exe'
    )
    foreach ($p in $candidates) { if (Test-Path $p) { return $p } }
    return $null
}

# Pick available PDF engine (prefer xelatex; fallback to tectonic, wkhtmltopdf)
function Select-PdfEngine {
    $xel = Get-Command xelatex -ErrorAction SilentlyContinue
    if (-not $xel) {
        $xelPath = Resolve-XeLaTeXPath
        if ($xelPath) {
            $xelDir = Split-Path -Parent $xelPath
            if ($env:Path -notlike "*$xelDir*") { $env:Path += ";$xelDir" }
            $xel = Get-Command xelatex -ErrorAction SilentlyContinue
        }
    }
    if ($xel) { return 'xelatex' }
    $tectonic = Get-Command tectonic -ErrorAction SilentlyContinue
    if ($tectonic) { return 'tectonic' }
    $wk = Get-Command wkhtmltopdf -ErrorAction SilentlyContinue
    if ($wk) { return 'wkhtmltopdf' }
    return $null
}

$pandocPath = Resolve-PandocPath
if (-not $pandocPath) {
    Write-Host "Error: Pandoc not found" -ForegroundColor Red
    Write-Host "Hint: reopen terminal or add C:\\Program Files\\Pandoc to PATH" -ForegroundColor Yellow
    Write-Host "Docs: https://pandoc.org/installing.html" -ForegroundColor Yellow
    exit 1
}

# 将 pandoc 目录加入当前会话 PATH，避免后续命令找不到
$pandocDir = Split-Path -Parent $pandocPath
if ($env:Path -notlike "*$pandocDir*") { $env:Path += ";$pandocDir" }

Write-Host "Pandoc detected: $pandocPath" -ForegroundColor Green
& $pandocPath --version | Select-Object -First 1 | Out-Host
Write-Host ""

# Create output directory
$outputDir = (Join-Path 'assets' 'pdf')
New-Item -ItemType Directory -Force -Path $outputDir | Out-Null
Write-Host "Output directory: $outputDir" -ForegroundColor Cyan
Write-Host ""

# CN Whitepaper
Write-Host "[1/4] Generating CN Whitepaper..." -ForegroundColor Yellow
try {
    $cnOut = Join-Path $outputDir 'SuperVM_Whitepaper_CN_v1.0.pdf'
    $pdfEngine = Select-PdfEngine
    if (-not $pdfEngine) { throw 'No PDF engine found (xelatex/tectonic/wkhtmltopdf). Please install MiKTeX, Tectonic or wkhtmltopdf.' }
    $pandocArgs = @(
        'WHITEPAPER.md',
        '-o', $cnOut,
        "--pdf-engine=$pdfEngine",
        '--toc',
        '--toc-depth=2',
        '--number-sections',
        '-V','CJKmainfont=SimSun',
        '-V','CJKsansfont=SimHei',
        '-V','geometry:margin=1in',
        '-V','fontsize=11pt',
        '-V','colorlinks=true',
        '-V','linkcolor=blue',
        '-V','urlcolor=blue',
        '--metadata','title=SuperVM Whitepaper (CN)',
        '--metadata','author=SuperVM Foundation',
        '--metadata','date=2025-01'
    )
    & $pandocPath @pandocArgs
    Write-Host "   CN whitepaper generated" -ForegroundColor Green
} catch {
    Write-Host "   CN whitepaper failed: $_" -ForegroundColor Red
}
Write-Host ""

# EN Whitepaper
Write-Host "[2/4] Generating EN Whitepaper..." -ForegroundColor Yellow
try {
    $enOut = Join-Path $outputDir 'SuperVM_Whitepaper_EN_v1.0.pdf'
    $pdfEngine = Select-PdfEngine
    if (-not $pdfEngine) { throw 'No PDF engine found (xelatex/tectonic/wkhtmltopdf). Please install MiKTeX, Tectonic or wkhtmltopdf.' }
    $pandocArgs = @(
        'WHITEPAPER_EN.md',
        '-o', $enOut,
        "--pdf-engine=$pdfEngine",
        '--toc',
        '--toc-depth=2',
        '--number-sections',
        '-V','mainfont=Times New Roman',
        '-V','sansfont=Arial',
        '-V','monofont=Courier New',
        '-V','geometry:margin=1in',
        '-V','fontsize=11pt',
        '-V','colorlinks=true',
        '-V','linkcolor=NavyBlue',
        '-V','urlcolor=RoyalBlue',
        '--metadata','title=SuperVM Technical Whitepaper',
        '--metadata','author=SuperVM Foundation',
        '--metadata','date=2025-01'
    )
    & $pandocPath @pandocArgs
    Write-Host "   EN whitepaper generated" -ForegroundColor Green
} catch {
    Write-Host "   EN whitepaper failed: $_" -ForegroundColor Red
}
Write-Host ""

# Investor Deck (PDF)
Write-Host "[3/4] Generating Investor Deck (PDF)..." -ForegroundColor Yellow
try {
    $deckMd = Join-Path 'docs' 'INVESTOR-PITCH-DECK.md'
    $deckOut = Join-Path $outputDir 'SuperVM_Investor_Deck_v1.0.pdf'
    $pdfEngine = Select-PdfEngine
    if (-not $pdfEngine) { throw 'No PDF engine found (xelatex/tectonic/wkhtmltopdf). Please install MiKTeX, Tectonic or wkhtmltopdf.' }
    $pandocArgs = @(
        $deckMd,
        '-o', $deckOut,
        "--pdf-engine=$pdfEngine",
        '-V','CJKmainfont=SimSun',
        '-V','CJKsansfont=SimHei',
        '-V','geometry:margin=0.75in',
        '-V','fontsize=12pt',
        '-V','colorlinks=true',
        '-V','linkcolor=NavyBlue',
        '--metadata','title=SuperVM Investor Deck',
        '--metadata','author=SuperVM Team',
        '--metadata','date=January 2025'
    )
    & $pandocPath @pandocArgs
    Write-Host "   Investor Deck generated" -ForegroundColor Green
} catch {
    Write-Host "   Investor Deck failed: $_" -ForegroundColor Red
}
Write-Host ""

# Social Media Templates (PDF)
Write-Host "[4/4] Generating Social Media Templates (PDF)..." -ForegroundColor Yellow
try {
    $smtMd = Join-Path 'docs' 'SOCIAL-MEDIA-TEMPLATES.md'
    $smtOut = Join-Path $outputDir 'SuperVM_Social_Media_Templates_v1.0.pdf'
    $pdfEngine = Select-PdfEngine
    if (-not $pdfEngine) { throw 'No PDF engine found (xelatex/tectonic/wkhtmltopdf). Please install MiKTeX, Tectonic or wkhtmltopdf.' }
    $pandocArgs = @(
        $smtMd,
        '-o', $smtOut,
        "--pdf-engine=$pdfEngine",
        '-V','CJKmainfont=SimSun',
        '-V','geometry:margin=1in',
        '-V','fontsize=10pt',
        '-V','colorlinks=true',
        '--metadata','title=SuperVM Social Media Templates',
        '--metadata','author=SuperVM Marketing',
        '--metadata','date=January 2025'
    )
    & $pandocPath @pandocArgs
    Write-Host "   Social media templates generated" -ForegroundColor Green
} catch {
    Write-Host "   Social media templates failed: $_" -ForegroundColor Red
}
Write-Host ""

# Summary
Write-Host "------------------------------" -ForegroundColor Cyan
Write-Host "PDF generation completed" -ForegroundColor Green
Write-Host "------------------------------" -ForegroundColor Cyan
Write-Host ""
Write-Host "Generated files:" -ForegroundColor Cyan

if (Test-Path $outputDir) {
    Get-ChildItem $outputDir -Filter *.pdf | ForEach-Object {
        $size = [math]::Round($_.Length / 1MB, 2)
    Write-Host "  - $($_.Name) " -NoNewline -ForegroundColor White
        Write-Host "($size MB)" -ForegroundColor Gray
    }
}

Write-Host ""
Write-Host "Location: $(Resolve-Path $outputDir)" -ForegroundColor Cyan
Write-Host ""
Write-Host "Hint: Use Adobe Reader or SumatraPDF to view" -ForegroundColor Yellow
Write-Host ""
