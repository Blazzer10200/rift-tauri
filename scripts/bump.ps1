# Rift version bump -- writes every source-manifest version in one shot.
#
# package.json + package-lock.json + src-tauri/Cargo.toml + Cargo.lock +
# src-tauri/tauri.conf.json must always match. `release.ps1`'s preflight bails
# on mismatch but doesn't fix it; this is the fixer.
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

Bump-First -Path 'package-lock.json' `
    -Pattern '(?m)^  "version":\s*"[^"]+",' `
    -Replacement "  `"version`": `"$Version`"," `
    -Label 'package-lock.json top-level version'

Bump-First -Path 'package-lock.json' `
    -Pattern '(?m)^      "version":\s*"[^"]+",' `
    -Replacement "      `"version`": `"$Version`"," `
    -Label 'package-lock.json root package version'

Bump-First -Path 'src-tauri/Cargo.toml' `
    -Pattern '(?ms)(\[package\][^\[]*?^version\s*=\s*")[^"]+(")' `
    -Replacement "`${1}$Version`${2}" `
    -Label 'Cargo.toml [package] version'

Bump-First -Path 'src-tauri/Cargo.lock' `
    -Pattern '(?ms)(\[\[package\]\]\r?\nname = "rift-tauri"\r?\nversion = ")[^"]+(")' `
    -Replacement "`${1}$Version`${2}" `
    -Label 'Cargo.lock rift-tauri package version'

Bump-First -Path 'src-tauri/tauri.conf.json' `
    -Pattern '"version":\s*"[^"]+"' `
    -Replacement "`"version`": `"$Version`"" `
    -Label 'tauri.conf.json top-level version'

# Cross-check: re-read every source manifest and confirm they match.
$pkg = Get-Content package.json -Raw | ConvertFrom-Json
$pkgLock = Get-Content package-lock.json -Raw | ConvertFrom-Json -AsHashtable
$pkgLockRoot = $pkgLock['packages']['']['version']
$cargoText = Get-Content src-tauri/Cargo.toml -Raw
$null = $cargoText -match '(?ms)\[package\][^\[]*?^version\s*=\s*"([^"]+)"'
$cargoVer = $matches[1]
$cargoLockText = Get-Content src-tauri/Cargo.lock -Raw
$null = $cargoLockText -match '(?ms)\[\[package\]\]\r?\nname = "rift-tauri"\r?\nversion = "([^"]+)"'
$cargoLockVer = $matches[1]
$tauriCfg = Get-Content src-tauri/tauri.conf.json -Raw | ConvertFrom-Json
if ($pkg.version -ne $Version -or $pkgLock['version'] -ne $Version -or
    $pkgLockRoot -ne $Version -or $cargoVer -ne $Version -or
    $cargoLockVer -ne $Version -or $tauriCfg.version -ne $Version) {
    throw "Post-bump verify failed: pkg=$($pkg.version), lock=$($pkgLock['version']), lockRoot=$pkgLockRoot, cargo=$cargoVer, cargoLock=$cargoLockVer, tauri=$($tauriCfg.version)"
}

Write-Host "All source manifests at $Version." -ForegroundColor Green
Write-Host "Next: edit docs/CHANGELOG.md, commit, then create the annotated release tag." -ForegroundColor Cyan
