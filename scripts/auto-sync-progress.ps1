# Auto Sync Phase Progress from sub-roadmaps/checklists
# Calculates completion ratio from ROADMAP-ZK-Privacy.md and updates Phase 5 percent in ROADMAP.md
# Heuristic:
#   - Count markdown checkboxes [-] and [x] under the whole file (default) or an optional section filter
#   - percent = Base + Weight * (completed/total*100), clamped [0,100]
# Defaults are chosen so that if ~30% of privacy items done -> Phase5 ≈ 35%

param(
    [Parameter(Mandatory=$false)] [string] $PrivacyPath = "ROADMAP-ZK-Privacy.md",
    [Parameter(Mandatory=$false)] [string] $RoadmapPath = "ROADMAP.md",
    [Parameter(Mandatory=$false)] [int] $Base = 5,
    [Parameter(Mandatory=$false)] [double] $Weight = 1.0,
    [Parameter(Mandatory=$false)] [int] $CapIncrease = 15,
    [Parameter(Mandatory=$false)] [string] $SectionFilter = "Phase 2: 生产级 RingCT 实现"
)

$ErrorActionPreference = 'Stop'

function Get-CheckboxStats {
    param([string] $Markdown, [string] $SectionFilter)
    $content = Get-Content -Path $Markdown -Raw -Encoding UTF8

    if ($SectionFilter -and ($content -match [regex]::Escape($SectionFilter))) {
        # Keep everything after the section filter header
        $parts = $content -split [regex]::Escape($SectionFilter), 2
        if ($parts.Length -ge 2) { $content = $parts[1] }
    }

    $checked = ([regex]::Matches($content, "- \[x\]", 'IgnoreCase')).Count
    $unchecked = ([regex]::Matches($content, "- \[ \]")).Count
    $total = $checked + $unchecked
    if ($total -eq 0) { return @{ Checked = 0; Total = 0; Ratio = 0.0 } }
    $ratio = [double]$checked / [double]$total
    return @{ Checked = $checked; Total = $total; Ratio = $ratio }
}

function Get-CurrentPhase5Percent {
    param([string] $Path)
    $content = Get-Content -Path $Path -Raw -Encoding UTF8
    # Match the Phase 5 row in the progress table and capture the percentage column
    $m = [regex]::Match($content, "\|\s*\*\*Phase 5\*\*\s*\|[^|]*\|[^|]*\|\s*(\d+)%\s*\|")
    if ($m.Success) { return [int]$m.Groups[1].Value }
    return $null
}

function Update-Phase5 {
    param([string] $Path, [int] $NewPercent)
    # Update table row
    $content = Get-Content -Path $Path -Raw -Encoding UTF8
    $pattern1 = '(?m)^(\|\s*\*\*Phase 5\*\*\s*\|[^|]*\|[^|]*\|\s*)(\d+%)(\s*\|.*)$'
    $updated = [regex]::Replace($content, $pattern1, "${1}$NewPercent%${3}")
    # Update ASCII diagram line percentage (header area with 进行中 XX%)
    $updated = [regex]::Replace($updated, '(进行中\s*)(\d+)%', "`${1}$NewPercent%")
    # Update bottom progress bar line like: "Phase 5 三通道路由  ███ ...  30% 🚧"
    # Be robust to varied spaces, trailing content (emoji/notes), and bar glyphs; capture any non-digit run before percent
    $patternBar = '(?m)(Phase\s*5\b.*?三通道路由[^\r\n]*?\s)(\d{1,3})%(\s*.*)$'
    $updated = [regex]::Replace($updated, $patternBar, "`${1}$NewPercent%`${3}")
    # Update last-updated date to today
    $today = (Get-Date).ToString('yyyy-MM-dd')
    $updated = [regex]::Replace($updated, '(>\s*\*\*开发者\*\*:\s*king\s*\|\s*\*\*架构师\*\*:\s*KING XU \(CHINA\)\s*\|\s*\*\*最后更新\*\*:\s*)(\d{4}-\d{2}-\d{2})', "`${1}$today")
    # Always perform a line-wise pass against the just-updated content to ensure stubborn lines are updated
    $lines = $updated -split '(?:\r\n|\n)'
    $changed = $false
    for ($i = 0; $i -lt $lines.Length; $i++) {
        if ($lines[$i] -like '*Phase 5 三通道路由*') {
            $newLine = [regex]::Replace($lines[$i], '(\d{1,3})%(\s*.*)$', "$NewPercent%`${2}")
            if ($newLine -ne $lines[$i]) { Write-Host "[auto-sync] Fallback updated Phase 5 line: $($lines[$i]) -> $newLine"; $lines[$i] = $newLine; $changed = $true }
        }
        if ($lines[$i] -like '*进行中*%*') {
            $newLine2 = [regex]::Replace($lines[$i], '(进行中\s*)(\d{1,3})%', "`${1}$NewPercent%")
            if ($newLine2 -ne $lines[$i]) { Write-Host "[auto-sync] Fallback updated header line: $($lines[$i]) -> $newLine2"; $lines[$i] = $newLine2; $changed = $true }
        }
    }
    $final = if ($changed) { $lines -join "`r`n" } else { $updated }
    if ($final -ne $content) { Set-Content -Path $Path -Value $final -Encoding UTF8 }
}

Write-Host "[auto-sync] Reading $PrivacyPath ..."
$stats = Get-CheckboxStats -Markdown $PrivacyPath -SectionFilter $SectionFilter
Write-Host ("[auto-sync] Privacy checklist => {0}/{1} ({2:P0})" -f $stats.Checked, $stats.Total, $stats.Ratio)

if ($stats.Total -eq 0) {
    Write-Host "[auto-sync] No checklist items found; skipping."
    exit 0
}

$current = Get-CurrentPhase5Percent -Path $RoadmapPath
if ($null -eq $current) { $current = 0 }

$target = [math]::Round($Base + $Weight * (100.0 * $stats.Ratio))
$target = [math]::Min(100, [math]::Max(0, $target))

# Smooth: cap single-run increase
if ($CapIncrease -gt 0 -and $target -gt ($current + $CapIncrease)) {
    $target = $current + $CapIncrease
}

Write-Host "[auto-sync] Phase 5 current=$current%, target=$target%"
if ($target -ne $current) {
    Update-Phase5 -Path $RoadmapPath -NewPercent $target
    Write-Host "[auto-sync] Updated Phase 5 to $target%"
} else {
    Write-Host "[auto-sync] No change needed."
}

Write-Host "[auto-sync] Done."