Param(
  [switch]$VerboseLogs
)

$ErrorActionPreference = 'Stop'

Write-Host "[info] Preparing WSL Docker-based RISC0 build..."

# Ensure Docker Desktop is installed and running (best-effort check)
try {
  docker --version | Out-Null
} catch {
  Write-Error "docker not found. Please install Docker Desktop and start it."
  Write-Host "Hint: https://docs.docker.com/desktop/install/windows-install/"
  exit 1
}

# Convert current repo path to WSL path
$repoWin = (Resolve-Path ".").Path
try {
  $repoWsl = (wsl.exe wslpath -a -u "$repoWin").Trim()
} catch {
  Write-Error "wsl.exe not available. Please enable WSL."
  Write-Host "Hint: https://learn.microsoft.com/windows/wsl/install"
  exit 1
}

$scriptWsl = "$repoWsl/scripts/local-risc0-build-docker.sh"

# Make script executable and run it inside WSL
$cmd = @(
  "set -euo pipefail",
  "cd '$repoWsl'",
  "chmod +x '$scriptWsl'",
  if ($VerboseLogs) { "export RUST_LOG=debug" } else { "true" },
  "bash '$scriptWsl'"
) -join " && "

Write-Host "[info] Invoking WSL build script..."
wsl.exe bash -lc "$cmd"

Write-Host "[done] Completed. Logs: ci_artifacts/local_build.log, ci_artifacts/local_run.log"
