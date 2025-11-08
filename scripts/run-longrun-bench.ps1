Param(
    [int]$DurationSecs = 600,
    [int]$IntervalSecs = 10,
    [int]$Threads = 8,
    [int]$KeySpace = 10000,
    [int]$WriteRatio = 100,
    [string]$OutDir = 'data/longrun'
)

$env:DURATION_SECS = $DurationSecs
$env:INTERVAL_SECS = $IntervalSecs
$env:NUM_THREADS = $Threads
$env:KEY_SPACE = $KeySpace
$env:WRITE_RATIO = $WriteRatio
$env:OUT_DIR = $OutDir

$timestamp = [int][double]::Parse((Get-Date -UFormat %s))
$env:OUT_FILE = "$OutDir/longrun_$timestamp.csv"

Write-Host "[LongRun] Duration=$DurationSecs Interval=$IntervalSecs Threads=$Threads KeySpace=$KeySpace WriteRatio=$WriteRatio Output=$env:OUT_FILE"

if (!(Test-Path $OutDir)) { New-Item -ItemType Directory -Path $OutDir | Out-Null }

# Build (release) then run example
cargo run -p vm-runtime --example mvcc_longrun_bench --release
