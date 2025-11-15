Param(
  [switch]$SkipInstall,
  [switch]$VerboseLogs
)

$ErrorActionPreference = 'Stop'
Write-Host "[info] Native RISC0 build (non-Docker) via WSL wrapper"

# Resolve repo root
$repoWin = (Resolve-Path ".").Path
try { $repoWsl = (wsl.exe wslpath -a -u $repoWin).Trim() } catch { Write-Error "WSL not available"; exit 1 }

$nativeScript = "$repoWsl/scripts/local-risc0-build-native.sh"

# Ensure script has execute bit
wsl.exe bash -lc "chmod +x '$nativeScript'" | Out-Null

$cmdParts = @()
$cmdParts += "set -euo pipefail"
$cmdParts += "cd '$repoWsl'"
if ($SkipInstall) { $cmdParts += "export SKIP_RZUP_INSTALL=1" }
if ($VerboseLogs) { $cmdParts += "export RUST_LOG=debug" }
$cmdParts += "bash '$nativeScript'"

$fullCmd = $cmdParts -join " && "
Write-Host "[info] Executing in WSL: $fullCmd"
wsl.exe bash -lc "$fullCmd"

Write-Host "[done] See ci_artifacts/native_build.log and native_run.log"