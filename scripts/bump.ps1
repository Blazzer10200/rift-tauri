# Rift version bump -- writes the THREE lockstep files in one shot.
#
# package.json + src-tauri/Cargo.toml + src-tauri/tauri.conf.json must always
# match. `release.ps1`'s preflight bails on mismatch but doesn't fix it; this
# is the fixer. Most-common ship-failure mode pre-this-script: bumping two of
# three by hand and tripping the preflight on `vpk pack`.
#
# Leaves CHANGELOG.md alone -- curated content is `/git-ship` territory.
#
# Usage:
#   pwsh ./scripts/bump.ps1 0.4.12-alpha
#   pwsh ./scripts/bump.ps1 -Version 0.4.12

param(
    [Parameter(Mandatory = $true, Position = 0)]
    [string]$Version
)

$ErrorActionPreference = 'Stop'

if ($Version -notmatch '^\d+\.\d+\.\d+(-[a-zA-Z0-9.]+)?$') {
    throw "Version '$Version' must be semver (e.g. 0.4.12 or 0.4.12-alpha)"
}

$repoRoot = Resolve-Path "$PSScriptRoot\.."
Set-Location $repoRoot

function Bump-First {
    param(
        [string]$Path,
        [string]$Pattern,
        [string]$Replacement,
        [string]$Label
    )
    if (-not (Test-Path $Path)) { throw "$Path : not found" }
    $text = [System.IO.File]::ReadAllText($Path)
    $rx = [regex]$Pattern
    if (-not $rx.IsMatch($text)) {
        throw "$Path : '$Label' line not found (pattern: $Pattern)"
    }
    $new = $rx.Replace($text, $Replacement, 1)
    if ($new -eq $text) {
        Write-Host "  $Path  (already at $Version)" -ForegroundColor DarkGray
        return
    }
    [System.IO.File]::WriteAllText($Path, $new)
    Write-Host "  $Path" -ForegroundColor Green
}

Write-Host "=== Bump to $Version ===" -ForegroundColor Cyan

Bump-First -Path 'package.json' `
    -Pattern '"version":\s*"[^"]+"' `
    -Replacement "`"version`": `"$Version`"" `
    -Label 'package.json top-level version'

Bump-First -Path 'src-tauri/Cargo.toml' `
    -Pattern '(?m)^version\s*=\s*"[^"]+"' `
    -Replacement "version = `"$Version`"" `
    -Label 'Cargo.toml [package] version'

Bump-First -Path 'src-tauri/tauri.conf.json' `
    -Pattern '"version":\s*"[^"]+"' `
    -Replacement "`"version`": `"$Version`"" `
    -Label 'tauri.conf.json top-level version'

# Cross-check: re-read all three and confirm they match.
$pkg = Get-Content package.json -Raw | ConvertFrom-Json
$cargoText = Get-Content src-tauri/Cargo.toml -Raw
$null = $cargoText -match '(?m)^version\s*=\s*"([^"]+)"'
$cargoVer = $matches[1]
$tauriCfg = Get-Content src-tauri/tauri.conf.json -Raw | ConvertFrom-Json
if ($pkg.version -ne $Version -or $cargoVer -ne $Version -or $tauriCfg.version -ne $Version) {
    throw "Post-bump verify failed: pkg=$($pkg.version), cargo=$cargoVer, tauri=$($tauriCfg.version)"
}

Write-Host "All three at $Version." -ForegroundColor Green
Write-Host "Next: edit docs/CHANGELOG.md, then pwsh scripts/release.ps1" -ForegroundColor Cyan
