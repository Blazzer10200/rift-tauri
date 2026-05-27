# Rift release pipeline -- Tauri-only path (v0.4.33+).
#
# Migrated from velopack 2026-05-26 (see docs/design/updater-migration.md).
# Produces a single NSIS Setup.exe + .sig, generates latest.json, and uploads
# all three to the public rift-releases GitHub repo. v0.4.32+ clients poll
# latest.json via tauri-plugin-updater.
#
# For the ONE-TIME v0.4.32 bridge release (also produces velopack artifacts so
# v0.4.31 clients can update), use scripts/release-bridge.ps1 instead.
#
# Prereqs on PATH: npm, gh (logged in). Tauri signing key env required --
# TAURI_SIGNING_PRIVATE_KEY_PATH (loaded from .secrets/env.sh) must point at
# C:/Users/BLAZZER/.tauri/rift.key. If unset, `tauri build` produces no .sig
# file and clients will reject the update.
#
# Bump versions BEFORE running -- use `scripts/bump.ps1 <version>` to write the
# three lockstep files (package.json + Cargo.toml + tauri.conf.json).
#
# Release notes are pulled from the top entry of `docs/CHANGELOG.md`. Top entry
# version must match the bumped version or notes are skipped.
#
# Usage:  pwsh ./scripts/release.ps1
#         pwsh ./scripts/release.ps1 -Force   # bypass dirty-tree refusal

param(
    [switch]$Force
)

$ErrorActionPreference = 'Stop'

$repoRoot = Resolve-Path "$PSScriptRoot\.."
Set-Location $repoRoot

Write-Host '=== Rift release pipeline (Tauri-updater path) ===' -ForegroundColor Cyan

# --- Preflight: version sync ---------------------------------------------
$pkg = Get-Content package.json -Raw | ConvertFrom-Json
$cargoText = Get-Content src-tauri/Cargo.toml -Raw
if ($cargoText -notmatch '(?ms)\[package\][^\[]*?^\s*version\s*=\s*"([^"]+)"') {
    throw 'Cargo.toml: cannot parse [package] version field'
}
$cargoVer = $matches[1]
$tauriCfg = Get-Content src-tauri/tauri.conf.json -Raw | ConvertFrom-Json
if ($pkg.version -ne $cargoVer -or $pkg.version -ne $tauriCfg.version) {
    throw "Version mismatch: package.json=$($pkg.version), Cargo.toml=$cargoVer, tauri.conf.json=$($tauriCfg.version)"
}
$version = $pkg.version
$tag = "v$version"
Write-Host "Version: $version (tag $tag)" -ForegroundColor Green

# --- Preflight: signing key ----------------------------------------------
if (-not $env:TAURI_SIGNING_PRIVATE_KEY_PATH -and -not $env:TAURI_SIGNING_PRIVATE_KEY) {
    throw 'TAURI_SIGNING_PRIVATE_KEY[_PATH] not set. Source .secrets/env.sh or set TAURI_SIGNING_PRIVATE_KEY_PATH=C:/Users/BLAZZER/.tauri/rift.key before releasing.'
}
if ($env:TAURI_SIGNING_PRIVATE_KEY_PATH -and -not (Test-Path $env:TAURI_SIGNING_PRIVATE_KEY_PATH)) {
    throw "TAURI_SIGNING_PRIVATE_KEY_PATH points at $($env:TAURI_SIGNING_PRIVATE_KEY_PATH) but no file there."
}

# --- Preflight: extract release notes from CHANGELOG ---------------------
# ASCII-only source -- \uXXXX in -replace patterns resolves at .NET regex
# compile time, NOT at PS string-parse time, so the source bytes stay ASCII.
# BOM-less .ps1 read as Win-1252 by PS5.1 would mojibake any literal
# multi-byte chars.
function Convert-ToAsciiSafe([string]$s) {
    $s = $s -replace '\u2014', '--'   # em-dash
    $s = $s -replace '\u2013', '-'    # en-dash
    $s = $s -replace '\u2212', '-'    # minus sign
    $s = $s -replace '\u2192', '->'   # right arrow
    $s = $s -replace '\u2190', '<-'   # left arrow
    $s = $s -replace '\u2194', '<->'  # left-right arrow
    $s = $s -replace '\u00D7', 'x'    # multiplication sign (v0.4.26 culprit)
    $s = $s -replace '\u00F7', '/'    # division sign
    $s = $s -replace '\u2026', '...'  # horizontal ellipsis
    $s = $s -replace '\u2018', "'"    # left single quote
    $s = $s -replace '\u2019', "'"    # right single quote
    $s = $s -replace '\u201C', '"'    # left double quote
    $s = $s -replace '\u201D', '"'    # right double quote
    $s = $s -replace '\u00A0', ' '    # non-breaking space
    $s = $s -replace '\u00B7', '*'    # middle dot
    $s = $s -replace '\u2022', '*'    # bullet
    # Belt-and-suspenders: drop any remaining non-ASCII.
    $sb = New-Object System.Text.StringBuilder
    foreach ($ch in $s.ToCharArray()) {
        if ([int]$ch -lt 128) { [void]$sb.Append($ch) }
    }
    return $sb.ToString()
}

$releaseNotesFile = $null
$notesBodyAscii = ""
$changelogPath = 'docs/CHANGELOG.md'
if (Test-Path $changelogPath) {
    $clText = [System.IO.File]::ReadAllText($changelogPath)
    $entryPattern = '(?ms)^## v(?<ver>[^\s]+)[^\r\n]*\r?\n(?<body>.*?)(?=^## v|\z)'
    $entryRx = New-Object System.Text.RegularExpressions.Regex $entryPattern
    $m = $entryRx.Match($clText)
    if ($m.Success) {
        $topVer = $m.Groups['ver'].Value
        if ($topVer -eq $version) {
            $body = $m.Groups['body'].Value.Trim()
            if ($body) {
                $notesBodyAscii = Convert-ToAsciiSafe $body
                $releaseNotesFile = Join-Path ([System.IO.Path]::GetTempPath()) "rift-release-notes-$version.md"
                [System.IO.File]::WriteAllText($releaseNotesFile, $notesBodyAscii)
                Write-Host "Release notes: top CHANGELOG entry ($($notesBodyAscii.Length) chars)" -ForegroundColor Green
            } else {
                Write-Host "Warning: CHANGELOG entry for v$version has empty body -- skipping notes" -ForegroundColor Yellow
            }
        } else {
            Write-Host "Warning: CHANGELOG top entry is v$topVer, not v$version -- skipping notes" -ForegroundColor Yellow
        }
    } else {
        Write-Host "Warning: no '## v...' entries found in $changelogPath -- skipping notes" -ForegroundColor Yellow
    }
} else {
    Write-Host "Warning: $changelogPath not found -- skipping notes" -ForegroundColor Yellow
}

# --- Preflight: tools ----------------------------------------------------
foreach ($t in @('npm', 'gh')) {
    if (-not (Get-Command $t -ErrorAction SilentlyContinue)) {
        throw "$t not found on PATH"
    }
}

# --- Preflight: clean git ------------------------------------------------
$dirty = git status --porcelain
if ($dirty) {
    Write-Host 'Working tree dirty:' -ForegroundColor Yellow
    Write-Host $dirty
    if (-not $Force) {
        Write-Host 'Refusing to ship from a dirty working tree. Re-run with -Force to override.' -ForegroundColor Red
        throw 'release.ps1: working tree dirty (pass -Force to override)'
    }
    Write-Host '  -Force: continuing despite dirty tree' -ForegroundColor Yellow
}

# --- Preflight: tag does not already exist ------------------------------
$releaseRepo = 'Blazzer10200/rift-releases'
try {
    $null = gh release view $tag --repo $releaseRepo --json tagName 2>&1
    if ($LASTEXITCODE -eq 0) {
        throw "GitHub release $tag already exists in $releaseRepo. Bump version or delete the release first."
    }
} catch [System.Management.Automation.RuntimeException] {
    if ($_.Exception.Message -like '*already exists*') { throw }
}

# --- Build ---------------------------------------------------------------
Write-Host '=== tauri build (NSIS + .sig) ===' -ForegroundColor Cyan
npm run tauri build
if ($LASTEXITCODE -ne 0) { throw 'tauri build failed' }

# tauri build emits to either src-tauri/target/ or $env:CARGO_TARGET_DIR.
$targetRoot = if ($env:CARGO_TARGET_DIR) { $env:CARGO_TARGET_DIR } else { 'src-tauri/target' }
$nsisDir = Join-Path $targetRoot 'release/bundle/nsis'
if (-not (Test-Path $nsisDir)) { throw "NSIS bundle dir not produced: $nsisDir" }

# Bundle dir is shared across all builds; filter to the exact current version
# so we don't pick up artifacts from prior tags. Pattern matches Tauri's NSIS
# naming: <productName>_<version>_<arch>-setup.exe.
$setupPattern = "*_${version}_*-setup.exe"
$setupCandidates = @(Get-ChildItem -Path $nsisDir -Filter $setupPattern -File)
if ($setupCandidates.Count -ne 1) {
    throw "Expected exactly one $setupPattern in $nsisDir, found $($setupCandidates.Count)"
}
$setupPath = $setupCandidates[0].FullName
$sigPath = "$setupPath.sig"
if (-not (Test-Path $sigPath)) {
    throw "Signature file missing: $sigPath. Check TAURI_SIGNING_PRIVATE_KEY_PATH and that createUpdaterArtifacts=true in tauri.conf.json."
}
Write-Host "  Setup: $setupPath" -ForegroundColor DarkGray
Write-Host "  Sig:   $sigPath" -ForegroundColor DarkGray

# --- Generate latest.json -----------------------------------------------
Write-Host '=== latest.json ===' -ForegroundColor Cyan
$sigContent = [System.IO.File]::ReadAllText($sigPath).Trim()
$pubDate = (Get-Date).ToUniversalTime().ToString("yyyy-MM-ddTHH:mm:ssZ")
$setupFileName = [System.IO.Path]::GetFileName($setupPath)
$downloadUrl = "https://github.com/$releaseRepo/releases/download/$tag/$setupFileName"

$latest = [ordered]@{
    version    = $version
    notes      = $notesBodyAscii
    pub_date   = $pubDate
    platforms  = [ordered]@{
        "windows-x86_64" = [ordered]@{
            signature = $sigContent
            url       = $downloadUrl
        }
    }
}
$latestJson = $latest | ConvertTo-Json -Depth 6

$releasesDir = Join-Path $repoRoot 'Releases'
if (-not (Test-Path $releasesDir)) { New-Item -ItemType Directory -Path $releasesDir | Out-Null }
$latestPath = Join-Path $releasesDir 'latest.json'
# UTF-8 NO BOM -- Tauri's JSON parser barfs on the BOM signature byte (R5 in brief).
[System.IO.File]::WriteAllText($latestPath, $latestJson, [System.Text.UTF8Encoding]::new($false))
Write-Host "  Wrote: $latestPath" -ForegroundColor DarkGray

# --- Create + upload to GitHub ------------------------------------------
Write-Host '=== gh release create ===' -ForegroundColor Cyan
$ghArgs = @(
    'release', 'create', $tag,
    '--repo', $releaseRepo,
    '--title', $tag
)
if ($releaseNotesFile) {
    $ghArgs += @('--notes-file', $releaseNotesFile)
} else {
    $ghArgs += @('--notes', '')
}
# DO NOT add --prerelease, even for alpha/beta/rc. GitHub's
# releases/latest/download/<asset> redirect EXCLUDES prereleases, which
# would 404 the tauri-updater endpoint baked into every shipped client
# (https://github.com/.../releases/latest/download/latest.json). Alpha-ness
# is communicated via the version suffix (`-alpha`), not the GH flag.
$ghArgs += @($setupPath, $sigPath, $latestPath)

& gh @ghArgs
if ($LASTEXITCODE -ne 0) { throw 'gh release create failed' }

# --- Round-trip verify ---------------------------------------------------
# Download the just-uploaded Setup.exe and SHA256-compare against the local
# pre-upload artifact. Catches corrupt/wrong-asset uploads before clients see
# a broken update (issue #18).
Write-Host '=== Round-trip verify (SHA256) ===' -ForegroundColor Cyan
$verifyDir = Join-Path $releasesDir "verify-$version"
if (Test-Path $verifyDir) { Remove-Item -Recurse -Force $verifyDir }
New-Item -ItemType Directory -Path $verifyDir | Out-Null
gh release download $tag --repo $releaseRepo --pattern "*-setup.exe" -D $verifyDir
if ($LASTEXITCODE -ne 0) { throw 'gh release download (verify) failed' }
$downloaded = @(Get-ChildItem -Path $verifyDir -Filter '*-setup.exe' -File)
if ($downloaded.Count -ne 1) {
    throw "Expected exactly one downloaded *-setup.exe in $verifyDir, found $($downloaded.Count)"
}
$localHash = (Get-FileHash -Algorithm SHA256 -Path $setupPath).Hash
$remoteHash = (Get-FileHash -Algorithm SHA256 -Path $downloaded[0].FullName).Hash
if ($localHash -ne $remoteHash) {
    throw "Round-trip verify FAILED: local SHA256=$localHash remote SHA256=$remoteHash. Setup.exe upload corrupted."
}
Write-Host "  SHA256 match: $localHash" -ForegroundColor Green
Remove-Item -Recurse -Force $verifyDir

# --- Verify --------------------------------------------------------------
Write-Host '=== Release published ===' -ForegroundColor Green
gh release view $tag --repo $releaseRepo

# --- Cleanup -------------------------------------------------------------
if ($releaseNotesFile -and (Test-Path $releaseNotesFile)) {
    Remove-Item -Force $releaseNotesFile
}
Write-Host "Done. Tag: $tag" -ForegroundColor Green
