<#
  SuperVM Workspace Cache Cleanup
  Usage:
    powershell -NoProfile -ExecutionPolicy Bypass -File scripts/cleanup-cache.ps1
      [-DoCleanup] [-Aggressive] [-GitGC]

  Notes:
  - Default is dry-run (report only). Add -DoCleanup to actually delete.
  - -Aggressive will also clean large data/ benches and pdf-output (destructive).
  - Safe by default: removes Cargo target/, __pycache__/, .ipynb_checkpoints/.
#>

param(
  [switch]$DoCleanup,
  [switch]$Aggressive,
  [switch]$GitGC
)

$ErrorActionPreference = 'Stop'

function Get-DirSizeBytes($path) {
  if (-not (Test-Path $path)) { return 0 }
  $sum = (Get-ChildItem -Path $path -Recurse -File -ErrorAction SilentlyContinue | Measure-Object -Property Length -Sum).Sum
  if (-not $sum) { return 0 } else { return [int64]$sum }
}

function Format-Size($bytes) {
  if ($bytes -ge 1GB) { return ([math]::Round($bytes/1GB,2).ToString() + ' GB') }
  elseif ($bytes -ge 1MB) { return ([math]::Round($bytes/1MB,2).ToString() + ' MB') }
  elseif ($bytes -ge 1KB) { return ([math]::Round($bytes/1KB,2).ToString() + ' KB') }
  else { return ($bytes.ToString() + ' B') }
}

# Ensure we operate from repo root
$scriptDir = Split-Path -Parent $MyInvocation.MyCommand.Path
$repoRoot = Split-Path -Parent $scriptDir
Set-Location $repoRoot

Write-Host 'SuperVM Cache Cleanup (dry-run by default)' -ForegroundColor Cyan
Write-Host "Repo: $repoRoot" -ForegroundColor DarkCyan
 $modeParts = @()
 if ($DoCleanup) { $modeParts += 'cleanup' } else { $modeParts += 'report' }
 if ($Aggressive) { $modeParts += 'aggressive' } else { $modeParts += 'safe' }
 Write-Host ('Mode: ' + ($modeParts -join ', ')) -ForegroundColor Yellow
Write-Host ''

# Collect candidate directories
$candidates = New-Object System.Collections.ArrayList

# 1) Root target/
if (Test-Path 'target') { [void]$candidates.Add((Resolve-Path 'target').Path) }

# 2) Nested target/ directories in workspace
Get-ChildItem -Directory -Recurse -Filter target -ErrorAction SilentlyContinue |
  ForEach-Object { [void]$candidates.Add($_.FullName) }

# 3) Python caches
Get-ChildItem -Directory -Recurse -Filter __pycache__ -ErrorAction SilentlyContinue |
  ForEach-Object { [void]$candidates.Add($_.FullName) }

# 4) Jupyter checkpoints
Get-ChildItem -Directory -Recurse -Filter .ipynb_checkpoints -ErrorAction SilentlyContinue |
  ForEach-Object { [void]$candidates.Add($_.FullName) }

# 5) Legacy pdf-output
if (Test-Path 'pdf-output') { [void]$candidates.Add((Resolve-Path 'pdf-output').Path) }

# 6) Aggressive: data/ (benches & demos) top-level subdirs
if ($Aggressive -and (Test-Path 'data')) {
  Get-ChildItem -Path 'data' -Directory -ErrorAction SilentlyContinue |
    ForEach-Object { [void]$candidates.Add($_.FullName) }
}

# Deduplicate
$candidates = $candidates | Sort-Object -Unique

# Compute sizes
$report = @()
$totalBytes = 0
foreach ($c in $candidates) {
  $size = Get-DirSizeBytes $c
  $totalBytes += $size
  $report += [PSCustomObject]@{ Path = $c; SizeBytes = $size; Size = (Format-Size $size) }
}

if ($report.Count -eq 0) {
  Write-Host 'No cleanup candidates found.' -ForegroundColor Green
  exit 0
}

Write-Host 'Candidates:' -ForegroundColor Cyan
$report | Sort-Object SizeBytes -Descending | Format-Table -AutoSize Path, Size
Write-Host ''
Write-Host ('Total potential reclaim: ' + (Format-Size $totalBytes)) -ForegroundColor Magenta
Write-Host ''

if (-not $DoCleanup) {
  Write-Host 'Dry-run only. Re-run with -DoCleanup to delete the above paths.' -ForegroundColor Yellow
  Write-Host 'Example: powershell -NoProfile -ExecutionPolicy Bypass -File scripts/cleanup-cache.ps1 -DoCleanup -Aggressive' -ForegroundColor DarkYellow
  exit 0
}

# Perform deletion (sorted largest-first)
Write-Host 'Deleting candidates...' -ForegroundColor Yellow
foreach ($item in ($report | Sort-Object SizeBytes -Descending)) {
  $p = $item.Path
  if (Test-Path $p) {
    try {
      Write-Host ('  Removing: ' + $p + '  (' + $item.Size + ')') -ForegroundColor DarkYellow
      Remove-Item -LiteralPath $p -Recurse -Force -ErrorAction Stop
    } catch {
      Write-Host ('  Failed to remove: ' + $p + '  -> ' + $_.Exception.Message) -ForegroundColor Red
    }
  }
}

# Optional: git gc
if ($GitGC) {
  if (Test-Path '.git') {
    Write-Host ''
    Write-Host 'Running git gc --prune=now --aggressive ...' -ForegroundColor Yellow
    try {
      git --no-pager gc --prune=now --aggressive | Out-Host
    } catch {
      Write-Host 'git gc failed or git not found.' -ForegroundColor DarkGray
    }
  }
}

Write-Host ''
Write-Host 'Cleanup completed.' -ForegroundColor Green
