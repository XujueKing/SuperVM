param(
  [string]$Features = "rocksdb-storage",
  [string]$OutDir = "dist",
  [string]$Name = "SuperVM"
)

$ErrorActionPreference = "Stop"
Write-Host "=== SuperVM Release Packaging (Windows) ==="

# Resolve root
$Root = Resolve-Path -LiteralPath (Join-Path $PSScriptRoot "..")
Set-Location $Root

# Build
$env:RUSTFLAGS = "-C target-cpu=native"
Write-Host "Building release with features: $Features"
cargo build --release --features $Features

# Prepare staging
$stamp = Get-Date -Format "yyyyMMdd_HHmmss"
$arch = "win64"
$pkgName = "$Name-$arch-$stamp"
$stage = Join-Path $Root "dist/$pkgName"
New-Item -ItemType Directory -Force -Path $stage | Out-Null
New-Item -ItemType Directory -Force -Path (Join-Path $stage "bin") | Out-Null
New-Item -ItemType Directory -Force -Path (Join-Path $stage "docs") | Out-Null

# Collect binaries (examples)
$bin = Join-Path $Root "target/release/examples"
$targets = @(
  "storage_metrics_http.exe",
  "persistence_consistency_test.exe",
  "zk_parallel_http_bench.exe",
  "routing_metrics_http_demo.exe"
)
foreach ($t in $targets) {
  $src = Join-Path $bin $t
  if (Test-Path $src) { Copy-Item $src (Join-Path $stage "bin/$t") }
}

# Optionally collect main crates if any bin exists
$mainBin = Join-Path $Root "target/release"
Get-ChildItem $mainBin -Filter "*.exe" | Where-Object { $_.Name -notmatch "(build|example)" } | ForEach-Object {
  Copy-Item $_.FullName (Join-Path $stage "bin/$($_.Name)") -ErrorAction SilentlyContinue
}

# Copy docs
$docs = @(
  "README.md","ROADMAP.md","LICENSE",
  "docs/ROCKSDB-WINDOWS-DEPLOYMENT.md",
  "docs/ROCKSDB-LINUX-DEPLOYMENT.md",
  "docs/ROCKSDB-MACOS-DEPLOYMENT.md"
)
foreach ($d in $docs) {
  if (Test-Path $d) {
    $dest = Join-Path $stage $d
    $destDir = Split-Path $dest -Parent
    New-Item -ItemType Directory -Force -Path $destDir | Out-Null
    Copy-Item $d $dest
  }
}

# Make zip
New-Item -ItemType Directory -Force -Path $OutDir | Out-Null
$zipPath = Join-Path $OutDir ("$pkgName.zip")
if (Test-Path $zipPath) { Remove-Item $zipPath -Force }
Compress-Archive -Path (Join-Path $stage "*") -DestinationPath $zipPath

Write-Host "\n✅ Package created: $zipPath"
