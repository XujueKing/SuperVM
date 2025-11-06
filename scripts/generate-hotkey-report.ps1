Param(
  [int[]]$MediumThresholds = @(20,40),
  [int[]]$HighThresholds = @(50,120),
  [int]$DecayPeriod = 10,
  [double]$DecayFactor = 0.9,
  [int]$Batches = 3,
  [string]$Output = "hotkey-report.md",
  [int]$HotKeyThreshold = 5,
  [switch]$Adaptive,
  [int]$TxPerThread = 200,
  [int]$Threads = 8,
  [int]$BatchSize = 20
)

# 使用 ownership_sharding_mixed_bench 基准，通过环境变量注入参数，多组运行生成对比表

Write-Host "[HotKeyReport] Generating report..." -ForegroundColor Cyan

if (!(Test-Path $Output)) { New-Item -ItemType File -Path $Output -Force | Out-Null }
"# Hot Key LFU Report (ownership_sharding_mixed_bench)`n" | Out-File $Output -Encoding UTF8
"参数: decay_period=$DecayPeriod decay_factor=$DecayFactor batches=$Batches hot_key_thr=$HotKeyThreshold adaptive=$Adaptive tx_per_thread=$TxPerThread threads=$Threads batch_size=$BatchSize`n" | Add-Content $Output
"| Medium | High | ExtremeTx | MediumTx | BatchTx | TPS | Conflicts | HotKeyThr | AdaptiveConf | Duration(ms) |" | Add-Content $Output
"|--------|------|----------|---------|---------|-----|-----------|-----------|-------------|-------------|" | Add-Content $Output

foreach ($m in $MediumThresholds) {
  foreach ($h in $HighThresholds) {
    # 注入环境变量（如需，可改为读取配置文件）
    $env:LFU_MEDIUM = $m
    $env:LFU_HIGH = $h
    $env:LFU_DECAY_PERIOD = $DecayPeriod
    $env:LFU_DECAY_FACTOR = $DecayFactor
    $env:HOT_KEY_THRESHOLD = $HotKeyThreshold
    $env:LFU_BATCHES = $Batches
    $env:TX_PER_THREAD = $TxPerThread
    $env:NUM_THREADS = $Threads
    $env:BATCH_SIZE = $BatchSize
    $env:ADAPTIVE = if ($Adaptive) { "1" } else { "0" }

    $run = cargo run --release --bin ownership_sharding_mixed_bench 2>$null
    if (-not $run) { Write-Warning "No output captured from bench. Skipping row for medium=$m high=$h"; continue }

    $diagLine = ($run | Select-String -Pattern 'Diag:' | Select-Object -Last 1).Line
    if (-not $diagLine) { Write-Warning "No diagnostics line found for medium=$m high=$h"; continue }
    $extreme = if ($diagLine -match 'extreme (\d+)') { $Matches[1] } else { '0' }
    $mediumTx = if ($diagLine -match 'medium (\d+)') { $Matches[1] } else { '0' }
    $batchTx = if ($diagLine -match 'batch (\d+)') { $Matches[1] } else { '0' }
    $hotThr = if ($diagLine -match 'hot_key_thr (\d+)') { $Matches[1] } else { $HotKeyThreshold }
    $adaptiveConf = if ($diagLine -match 'adaptive conf ([0-9\.]+)') { $Matches[1] } else { '0' }
    $tpsLine = ($run | Select-String -Pattern 'TPS:' | Select-Object -Last 1).Line
    $tps = if ($tpsLine) { ($tpsLine -split '\s+')[1] } else { 'NA' }
    $confLine = ($run | Select-String -Pattern 'Avg Conflicts:' | Select-Object -Last 1).Line
    $conflicts = if ($confLine) { ($confLine -split ':')[1].Trim() } else { 'NA' }
    $durMs = 'NA'

    "| $m | $h | $extreme | $mediumTx | $batchTx | $tps | $conflicts | $hotThr | $adaptiveConf | $durMs |" | Add-Content $Output
    Write-Host "[Done] medium=$m high=$h -> extreme=$extreme medium=$mediumTx batch=$batchTx TPS=$tps conflicts=$conflicts" -ForegroundColor Green
  }
}

Write-Host "Report written to $Output" -ForegroundColor Yellow
