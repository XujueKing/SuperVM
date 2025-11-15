# Migrate existing scattered asset folders into unified assets/ hierarchy
# Usage: powershell -ExecutionPolicy Bypass -File scripts/migrate-assets.ps1

$ErrorActionPreference = 'Stop'

# Use script location to determine repository root for consistent behavior
$scriptDir = Split-Path -Parent $MyInvocation.MyCommand.Path
$root = Split-Path -Parent $scriptDir
$assetsRoot = Join-Path $root 'assets'
$targets = @(
    @{ Source = 'visuals'; Dest = 'visuals' },
    @{ Source = 'pdf-output'; Dest = 'pdf' },
    @{ Source = 'pitch-deck-source.md'; Dest = 'ppt' } # single file if existed at root
)

Write-Host "Migrating asset folders into assets/ ..." -ForegroundColor Cyan

if (-not (Test-Path $assetsRoot)) { New-Item -ItemType Directory -Force -Path $assetsRoot | Out-Null }

foreach ($t in $targets) {
    $srcPath = Join-Path $root $t.Source
    if (Test-Path $srcPath) {
        $destDir = Join-Path $assetsRoot $t.Dest
        if (-not (Test-Path $destDir)) { New-Item -ItemType Directory -Force -Path $destDir | Out-Null }
        Write-Host "  -> $($t.Source) → assets/$($t.Dest)" -ForegroundColor Yellow
        if ((Get-Item $srcPath).PSIsContainer) {
            Get-ChildItem -Path $srcPath -Recurse | ForEach-Object {
                $rel = $_.FullName.Substring($srcPath.Length).TrimStart('\','/')
                $targetPath = Join-Path $destDir $rel
                if ($_.PSIsContainer) {
                    if (-not (Test-Path $targetPath)) { New-Item -ItemType Directory -Force -Path $targetPath | Out-Null }
                } else {
                    $targetParent = Split-Path -Parent $targetPath
                    if (-not (Test-Path $targetParent)) { New-Item -ItemType Directory -Force -Path $targetParent | Out-Null }
                    Copy-Item -Path $_.FullName -Destination $targetPath -Force
                }
            }
        } else {
            Copy-Item -Path $srcPath -Destination (Join-Path $destDir (Split-Path -Leaf $srcPath)) -Force
        }
    } else {
        Write-Host "  (skip missing) $($t.Source)" -ForegroundColor DarkGray
    }
}

Write-Host "Migration complete." -ForegroundColor Green
Write-Host "Resulting tree (top-level):" -ForegroundColor Cyan
Get-ChildItem $assetsRoot | ForEach-Object { Write-Host "  - $($_.Name)" }
