Param(
    [string]$OutDir = 'data/quick_bench',
    [switch]$NoEmoji,
    [int]$RetryIters = 1000,
    [int]$FlushTxns = 1000
)

Write-Host "[QuickBenches] Output directory: $OutDir"
if (!(Test-Path $OutDir)) { New-Item -ItemType Directory -Path $OutDir | Out-Null }

# Build once (release) for consistency
Write-Host "[QuickBenches] Building examples (release)..."
cargo build -p vm-runtime --release --examples | Out-Null

# Helper to run and capture output
function Invoke-Capture {
    param(
        [string]$Cmd,
        [string]$Name
    )
    Write-Host "[QuickBenches] Running $Name ..."
    $file = Join-Path $OutDir "$Name-output.txt"
    if (Test-Path $file) { Remove-Item -Force $file }
    $psi = New-Object System.Diagnostics.ProcessStartInfo
    $psi.FileName = "powershell"
    $psi.Arguments = "-NoProfile -ExecutionPolicy Bypass -Command $Cmd"
    $psi.UseShellExecute = $false
    $psi.RedirectStandardOutput = $true
    $psi.RedirectStandardError = $true
    $proc = New-Object System.Diagnostics.Process
    $proc.StartInfo = $psi
    $null = $proc.Start()
    $stdOut = $proc.StandardOutput.ReadToEnd()
    $stdErr = $proc.StandardError.ReadToEnd()
    $proc.WaitForExit()
    $combined = $stdOut + [Environment]::NewLine + $stdErr
    if ($NoEmoji) { $combined = Remove-Emoji $combined }
    $combined | Out-File -Encoding UTF8 $file
    return [PSCustomObject]@{
        Output = $combined
        ExitCode = $proc.ExitCode
        File = $file
        Name = $Name
    }
}

# Helper to format values with default
function Format-OrDefault {
    param(
        [Parameter(Mandatory=$true)] [AllowNull()] $Value,
        [string] $Suffix
    )
    if ($null -eq $Value) { return '(not found)' }
    if ($Value -is [array]) { if ($Value.Count -eq 0) { return '(not found)' } else { $Value = $Value[0] } }
    $str = [string]$Value
    if ([string]::IsNullOrWhiteSpace($str)) { return '(not found)' }
    if ($Suffix) { return "$str $Suffix" } else { return $str }
}

# Helper: remove emoji/surrogate pairs and common symbol ranges when -NoEmoji
function Remove-Emoji {
    param([string]$Text)
    if ([string]::IsNullOrEmpty($Text)) { return $Text }
    $noSurrogates = [System.Text.RegularExpressions.Regex]::Replace($Text, "[\uD800-\uDBFF][\uDC00-\uDFFF]", "")
    $noSymbols = [System.Text.RegularExpressions.Regex]::Replace($noSurrogates, "[\u2600-\u27BF]", "")
    return $noSymbols
}

# Helper: convert string to int/double or $null
function Convert-ToIntOrNull {
    param($s)
    if ($null -eq $s) { return $null }
    $v = $s
    if ($s -is [array]) {
        if ($s.Count -eq 0) { return $null }
        $v = $s[0]
    }
    $v = ($v -replace ",", "")
    if ($v -match '^[0-9]+$') { return [int]$v } else { return $null }
}
function Convert-ToDoubleOrNull {
    param($s)
    if ($null -eq $s) { return $null }
    $v = $s
    if ($s -is [array]) {
        if ($s.Count -eq 0) { return $null }
        $v = $s[0]
    }
    $v = ($v -replace ",", "")
    if ($v -match '^[0-9]+(\.[0-9]+)?$') { return [double]$v } else { return $null }
}

# Run quick_retry_bench
$env:RETRY_ITERS = $RetryIters
$retryCmd = "cargo run -p vm-runtime --example quick_retry_bench --release"
$retryResult = Invoke-Capture -Cmd $retryCmd -Name "quick_retry_bench"
$retryOut = $retryResult.Output

# Parse metrics (simple regexes)
$retrySuccess = ($retryOut | Select-String -Pattern 'Successes: (\d+)' | ForEach-Object { $_.Matches[0].Groups[1].Value })
$retryThroughput = ($retryOut | Select-String -Pattern 'Throughput: ([\d,\.]+) ops/sec' | ForEach-Object { $_.Matches[0].Groups[1].Value })
$retryRetries = ($retryOut | Select-String -Pattern 'Retry count: (\d+)' | ForEach-Object { $_.Matches[0].Groups[1].Value })

# Run quick_flush_bench
$env:FLUSH_TXNS = $FlushTxns
$flushCmd = "cargo run -p vm-runtime --example quick_flush_bench --release --features rocksdb-storage"
$flushResult = Invoke-Capture -Cmd $flushCmd -Name "quick_flush_bench"
$flushOut = $flushResult.Output
$flushTxns = ($flushOut | Select-String -Pattern 'Total transactions: (\d+)' | ForEach-Object { $_.Matches[0].Groups[1].Value })
$flushCommitted = ($flushOut | Select-String -Pattern 'Committed: (\d+)' | ForEach-Object { $_.Matches[0].Groups[1].Value })
$flushThroughput = ($flushOut | Select-String -Pattern 'Throughput: ([\d,\.]+) txn/sec' | ForEach-Object { $_.Matches[0].Groups[1].Value })

# Markdown summary
$timestamp = (Get-Date).ToString('yyyy-MM-dd HH:mm:ss')
$mdFile = Join-Path $OutDir "summary.md"
$md = @()
$md += "# Quick Bench Summary"
$md += "Generated: $timestamp"
$md += ""
$md += "## Retry Bench"
$md += "- Successes: $(Format-OrDefault -Value $retrySuccess)"
$md += "- Retries: $(Format-OrDefault -Value $retryRetries)"
$md += "- Throughput: $(Format-OrDefault -Value $retryThroughput -Suffix 'ops/sec')"
$md += ""
$md += "## Flush Bench"
$md += "- Transactions: $(Format-OrDefault -Value $flushTxns)"
$md += "- Committed: $(Format-OrDefault -Value $flushCommitted)"
$md += "- Throughput: $(Format-OrDefault -Value $flushThroughput -Suffix 'txn/sec')"
$md += ""
$md += "Raw outputs saved under $OutDir"
$md -join "`n" | Out-File -Encoding UTF8 $mdFile

# JSON summary
$jsonObj = [PSCustomObject]@{
    generated_at = (Get-Date).ToString('o')
    no_emoji = [bool]$NoEmoji
    retry_bench = [PSCustomObject]@{
    successes = (Convert-ToIntOrNull $retrySuccess)
    retries = (Convert-ToIntOrNull $retryRetries)
    throughput_ops_sec = (Convert-ToDoubleOrNull $retryThroughput)
        raw_file = $retryResult.File
        exit_code = $retryResult.ExitCode
    }
    flush_bench = [PSCustomObject]@{
    transactions = (Convert-ToIntOrNull $flushTxns)
    committed = (Convert-ToIntOrNull $flushCommitted)
    throughput_txn_sec = (Convert-ToDoubleOrNull $flushThroughput)
        raw_file = $flushResult.File
        exit_code = $flushResult.ExitCode
    }
}
$jsonFile = Join-Path $OutDir "summary.json"
$jsonObj | ConvertTo-Json -Depth 5 | Out-File -Encoding UTF8 $jsonFile

Write-Host "[QuickBenches] Summary written: $mdFile"
Write-Host "[QuickBenches] JSON summary written: $jsonFile"