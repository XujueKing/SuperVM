<#
SuperVM Git Hooks Installer (ASCII only)
Installs the pre-commit hook to enforce kernel protection checks.
#>

Write-Host "Installing SuperVM Git pre-commit hook..." -ForegroundColor Cyan

# Ensure we are in a Git repository
if (-not (Test-Path ".git")) {
    Write-Host "Error: .git directory not found. Run this script from the repository root." -ForegroundColor Red
    exit 1
}

# Define paths
$hooksDir = ".git/hooks"
$sourceHook = "scripts/pre-commit-hook.sh"
$targetHook = Join-Path $hooksDir "pre-commit"

# Ensure hooks directory exists
if (-not (Test-Path $hooksDir)) {
    New-Item -ItemType Directory -Path $hooksDir -Force | Out-Null
}

# Ensure source hook exists
if (-not (Test-Path $sourceHook)) {
    Write-Host "Error: Source hook not found: $sourceHook" -ForegroundColor Red
    exit 1
}

# Copy hook
Copy-Item -Path $sourceHook -Destination $targetHook -Force

# Final summary
Write-Host "Hook installed: $targetHook" -ForegroundColor Green
Write-Host "Notes:" -ForegroundColor White
Write-Host "- The hook warns on L0/L1 kernel changes and may block commits without confirmation." -ForegroundColor Gray
Write-Host "- Maintainer override is available (env SUPERVM_OVERRIDE=1, git config supervm.override true, or .kernel-override)." -ForegroundColor Gray
Write-Host "- See docs/KERNEL-QUICK-REFERENCE.md for details." -ForegroundColor Gray



