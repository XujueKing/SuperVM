# Update roadmap progress percentages and last-updated metadata
# Usage:
#   powershell -ExecutionPolicy Bypass -File scripts/update-roadmaps.ps1 -Phase5Percent 35

param(
    [Parameter(Mandatory=$false)] [int] $Phase5Percent = 35,
    [Parameter(Mandatory=$false)] [string] $RoadmapPath = "ROADMAP.md"
)

$ErrorActionPreference = 'Stop'

function Update-LastUpdated {
    param([string] $Path)
    $content = Get-Content -Path $Path -Raw -Encoding UTF8
    $today = (Get-Date).ToString('yyyy-MM-dd')
    $pattern = '(?<prefix>>\s*\*\*开发者\*\*:\s*king\s*\|\s*\*\*架构师\*\*:\s*KING XU \(CHINA\)\s*\|\s*\*\*最后更新\*\*:\s*)(\d{4}-\d{2}-\d{2})'
    $replacement = "`${prefix}$today"
    $new = [regex]::Replace($content, $pattern, $replacement)
    if ($new -ne $content) { Set-Content -Path $Path -Value $new -Encoding UTF8 }
}

function Update-Phase5-Percent {
    param([string] $Path, [int] $Percent)
    $content = Get-Content -Path $Path -Raw -Encoding UTF8
    # table row replacement (Phase 5 line)
    $pattern1 = '(\|\s*\*\*Phase 5\*\*\s*\|[^|]*\|[^|]*\|\s*)(\d+%)(\s*\|)'
    $new = [regex]::Replace($content, $pattern1, "${1}$Percent%${3}")
    # ASCII diagram replacement (进行中 NN%)
    $pattern2 = '(进行中\s*)(\d+)%'
    $new2 = [regex]::Replace($new, $pattern2, "`${1}$Percent%")
    if ($new2 -ne $content) { Set-Content -Path $Path -Value $new2 -Encoding UTF8 }
}

Write-Host "Updating $RoadmapPath to Phase5=$Phase5Percent% ..."
Update-LastUpdated -Path $RoadmapPath
Update-Phase5-Percent -Path $RoadmapPath -Percent $Phase5Percent
Write-Host "Done."
