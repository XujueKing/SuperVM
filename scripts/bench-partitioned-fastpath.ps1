# Partitioned FastPath benchmark runner
# Runs vm-runtime example with different partition counts and collects TPS

param(
    [Parameter(ValueFromRemainingArguments=$true)]
    $Partitions = @(2,4,8),
    [int]$Txs = 200000,
    [int]$Cycles = 32
)

# Normalize partitions into an int array
if ($Partitions -is [string]) {
    $Partitions = ($Partitions -split ',') | ForEach-Object { [int]$_ }
}
elseif ($Partitions -isnot [array]) {
    $Partitions = @([int]$Partitions)
}

$ErrorActionPreference = 'Stop'

$root = Split-Path -Parent $MyInvocation.MyCommand.Path
$repo = Split-Path -Parent $root
Set-Location $repo

$resultsCsv = Join-Path $repo "bench_partitioned_fastpath_results.csv"
"timestamp,partitions,txs,cycles,tps,elapsed_ms" | Out-File -FilePath $resultsCsv -Encoding UTF8

foreach ($p in $Partitions) {
    Write-Host "[Run] partitions=$p txs=$Txs cycles=$Cycles" -ForegroundColor Cyan
    $outFile = Join-Path $repo ("bench_partitioned_fastpath_p{0}.txt" -f $p)
    $start = Get-Date
    $env:PART_TXS = "$Txs"
    $env:PARTITIONS = "$p"
    $env:SIM_CYCLES = "$Cycles"
    cargo run -p vm-runtime --example partitioned_fast_path_bench --release --features partitioned-fastpath 2>&1 |
        Tee-Object -FilePath $outFile | Out-Null
    $end = Get-Date
    $elapsedMs = [int](($end - $start).TotalMilliseconds)

    # Try to parse TPS from the output file
    $tps = 0
    if (Test-Path $outFile) {
        $line = (Select-String -Path $outFile -Pattern "TPS≈([0-9]+)" -SimpleMatch:$false | Select-Object -Last 1).Line
        if ($line) {
            if ($line -match "TPS≈([0-9]+)") { $tps = [int]$Matches[1] }
        }
    }

    $ts = (Get-Date).ToString("s")
    "$ts,$p,$Txs,$Cycles,$tps,$elapsedMs" | Add-Content -Path $resultsCsv
    Write-Host "  → TPS=$tps elapsed=${elapsedMs}ms (log: $outFile)" -ForegroundColor Green
}

Write-Host "\nResults saved to: $resultsCsv" -ForegroundColor Yellow
