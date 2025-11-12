param(
  [string]$DbPath = "",
  [string]$Features = "rocksdb-storage",
  [switch]$Yes
)

Write-Host "=== SuperVM Bootstrap (Windows) ==="

# 1) Resolve workspace root and default DB path
$Root = Resolve-Path -LiteralPath (Join-Path $PSScriptRoot "..")
if ([string]::IsNullOrWhiteSpace($DbPath)) {
  $DbPath = Join-Path $Root "data/rocksdb"
}

# 2) Check cargo
if (-not (Get-Command cargo -ErrorAction SilentlyContinue)) {
  Write-Warning "未检测到 cargo。请先安装 Rust 工具链 (https://rustup.rs)"
  exit 1
}

# 3) Ensure DB path
New-Item -ItemType Directory -Force -Path $DbPath | Out-Null
Write-Host "DB Path: $DbPath"

# 4) Build
$env:RUSTFLAGS = "-C target-cpu=native"
Write-Host "构建中: cargo build --release --features $Features"
cargo build --release --features $Features
if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }

# 5) Next steps
Write-Host "\n✅ 构建完成"
Write-Host "下一步:"
Write-Host "  - 运行 HTTP 指标服务: cargo run --example storage_metrics_http --features $Features --release"
Write-Host "  - 持久化一致性测试: cargo run --example persistence_consistency_test --features $Features --release"
Write-Host "  - 数据目录: $DbPath (首次运行自动产生内容)"
