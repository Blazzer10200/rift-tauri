# Rift release pipeline -- ONE-TIME BRIDGE for v0.4.32-alpha.
#
# v0.4.32 is the transition release: v0.4.31 users discover it via Velopack's
# GithubSource feed (releases.win.json + nupkgs), but v0.4.32+ clients use
# tauri-plugin-updater (latest.json + Setup.exe). This script produces BOTH
# sets of artifacts and uploads them to a single GitHub release.
#
# After v0.4.32 ships AND both machines are confirmed on it (gate: HANDOFF
# "second-machine sync gate" \u2014 do NOT ship v0.4.33 until verified), retire this
# script. v0.4.33+ uses scripts/release.ps1 (Tauri-only, no vpk).
#
# Failure mode you cannot dodge: v0.4.31's apply phase still hangs 5-10 min on
# the Velopack Update.exe bug. Document loudly in the CHANGELOG entry for
# v0.4.32; offer a manual Setup.exe download link as the escape hatch.
#
# Prereqs on PATH: npm, vpk (`dotnet tool install -g vpk`), gh. Plus the
# Tauri signing key env (TAURI_SIGNING_PRIVATE_KEY_PATH).
#
# Usage:  pwsh ./scripts/release-bridge.ps1
#         pwsh ./scripts/release-bridge.ps1 -Force

param(
    [switch]$Force
)

$ErrorActionPreference = 'Stop'

$repoRoot = Resolve-Path "$PSScriptRoot\.."
Set-Location $repoRoot

Write-Host '=== Rift release pipeline (BRIDGE: velopack + tauri-updater) ===' -ForegroundColor Cyan

# --- Preflight: this script is for v0.4.32-alpha only -------------------
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
if ($version -ne '0.4.32-alpha') {
    Write-Host "WARNING: release-bridge is intended for v0.4.32-alpha only. Current version is $version." -ForegroundColor Yellow
    Write-Host "Use scripts/release.ps1 for clean Tauri-only releases." -ForegroundColor Yellow
    if (-not $Force) {
        throw "Refusing to run bridge release for $version (pass -Force to override)"
    }
}
Write-Host "Version: $version (tag $tag)" -ForegroundColor Green

# --- Preflight: signing key ----------------------------------------------
if (-not $env:TAURI_SIGNING_PRIVATE_KEY_PATH -and -not $env:TAURI_SIGNING_PRIVATE_KEY) {
    throw 'TAURI_SIGNING_PRIVATE_KEY[_PATH] not set. Source .secrets/env.sh first.'
}
if ($env:TAURI_SIGNING_PRIVATE_KEY_PATH -and -not (Test-Path $env:TAURI_SIGNING_PRIVATE_KEY_PATH)) {
    throw "TAURI_SIGNING_PRIVATE_KEY_PATH points at $($env:TAURI_SIGNING_PRIVATE_KEY_PATH) but no file there."
}

# --- Preflight: release notes (same ASCII-safe helper as release.ps1) ---
function Convert-ToAsciiSafe([string]$s) {
    $s = $s -replace '\u2014', '--'
    $s = $s -replace '\u2013', '-'
    $s = $s -replace '\u2212', '-'
    $s = $s -replace '\u2192', '->'
    $s = $s -replace '\u2190', '<-'
    $s = $s -replace '\u2194', '<->'
    $s = $s -replace '\u00D7', 'x'
    $s = $s -replace '\u00F7', '/'
    $s = $s -replace '\u2026', '...'
    $s = $s -replace '\u2018', "'"
    $s = $s -replace '\u2019', "'"
    $s = $s -replace '\u201C', '"'
    $s = $s -replace '\u201D', '"'
    $s = $s -replace '\u00A0', ' '
    $s = $s -replace '\u00B7', '*'
    $s = $s -replace '\u2022', '*'
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
    if ($m.Success -and $m.Groups['ver'].Value -eq $version) {
        $body = $m.Groups['body'].Value.Trim()
        if ($body) {
            $notesBodyAscii = Convert-ToAsciiSafe $body
            $releaseNotesFile = Join-Path ([System.IO.Path]::GetTempPath()) "rift-release-notes-$version.md"
            [System.IO.File]::WriteAllText($releaseNotesFile, $notesBodyAscii)
            Write-Host "Release notes: top CHANGELOG entry ($($notesBodyAscii.Length) chars)" -ForegroundColor Green
        }
    }
}

# --- Preflight: tools ----------------------------------------------------
foreach ($t in @('npm', 'vpk', 'gh')) {
    if (-not (Get-Command $t -ErrorAction SilentlyContinue)) {
        throw "$t not found on PATH"
    }
}

# --- Preflight: clean git ------------------------------------------------
$dirty = git status --porcelain
if ($dirty -and -not $Force) {
    Write-Host 'Working tree dirty:' -ForegroundColor Yellow
    Write-Host $dirty
    throw 'release-bridge.ps1: working tree dirty (pass -Force to override)'
}

# --- Preflight: tag does not already exist ------------------------------
$releaseRepo = 'Blazzer10200/rift-releases'
try {
    $null = gh release view $tag --repo $releaseRepo --json tagName 2>&1
    if ($LASTEXITCODE -eq 0) {
        throw "GitHub release $tag already exists in $releaseRepo."
    }
} catch [System.Management.Automation.RuntimeException] {
    if ($_.Exception.Message -like '*already exists*') { throw }
}

# --- Build (tauri NSIS + .sig) ------------------------------------------
Write-Host '=== tauri build (NSIS + .sig) ===' -ForegroundColor Cyan
npm run tauri build
if ($LASTEXITCODE -ne 0) { throw 'tauri build failed' }

$targetRoot = if ($env:CARGO_TARGET_DIR) { $env:CARGO_TARGET_DIR } else { 'src-tauri/target' }
$nsisDir = Join-Path $targetRoot 'release/bundle/nsis'
$setupCandidates = @(Get-ChildItem -Path $nsisDir -Filter '*-setup.exe' -File)
if ($setupCandidates.Count -ne 1) {
    throw "Expected exactly one *-setup.exe in $nsisDir, found $($setupCandidates.Count)"
}
$setupPath = $setupCandidates[0].FullName
$sigPath = "$setupPath.sig"
if (-not (Test-Path $sigPath)) {
    throw "Signature file missing: $sigPath."
}
Write-Host "  Tauri Setup: $setupPath" -ForegroundColor DarkGray
Write-Host "  Tauri Sig:   $sigPath" -ForegroundColor DarkGray

# --- Generate latest.json (for v0.4.32+ tauri-updater clients) ----------
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
[System.IO.File]::WriteAllText($latestPath, $latestJson, [System.Text.UTF8Encoding]::new($false))
Write-Host "  Wrote: $latestPath" -ForegroundColor DarkGray

# --- Stage rift-tauri.exe for velopack pack -----------------------------
# v0.4.31's Velopack-rust client expects to find releases.win.json + a nupkg
# in the GitHub release. vpk pack wraps the same rift-tauri.exe in its own
# nupkg + Setup.exe format. v0.4.31's apply phase then runs the vpk-built
# Setup.exe (the one that hangs); after the swap it lands on disk as the
# Tauri build's v0.4.32 binary, which then takes the tauri-updater path.
$exePath = if ($env:CARGO_TARGET_DIR) {
    Join-Path $env:CARGO_TARGET_DIR 'release/rift-tauri.exe'
} else {
    'src-tauri/target/release/rift-tauri.exe'
}
if (-not (Test-Path $exePath)) { throw "exe not produced: $exePath" }

$staging = "Releases/staging-$version"
if (Test-Path $staging) { Remove-Item -Recurse -Force $staging }
New-Item -ItemType Directory -Path $staging -Force | Out-Null
Copy-Item $exePath $staging
Copy-Item 'src-tauri/icons/icon.ico' $staging

# --- vpk pack (legacy feed for v0.4.31 clients) -------------------------
Write-Host '=== vpk pack (legacy bridge for v0.4.31 clients) ===' -ForegroundColor Cyan
$packArgs = @(
    'pack',
    '-u', 'Rift',
    '-v', $version,
    '-p', $staging,
    '-e', 'rift-tauri.exe',
    '--packTitle', 'Rift',
    '--packAuthors', 'Blazzer',
    '--icon', "$staging/icon.ico",
    '-o', 'Releases'
)
if ($releaseNotesFile) {
    $packArgs += @('--releaseNotes', $releaseNotesFile)
}
& vpk @packArgs
if ($LASTEXITCODE -ne 0) { throw 'vpk pack failed' }

# --- Upload to GitHub (both legacy vpk AND new tauri-updater assets) ----
# vpk upload github creates the release. After that we attach the Tauri
# Setup.exe + .sig + latest.json via `gh release upload`.
Write-Host '=== vpk upload github ===' -ForegroundColor Cyan
$ghToken = (gh auth token).Trim()
if (-not $ghToken) { throw 'gh auth token returned empty' }

$uploadArgs = @(
    'upload', 'github',
    '--repoUrl', "https://github.com/$releaseRepo",
    '--publish',
    '--channel', 'win',
    '--releaseName', $tag,
    '--tag', $tag,
    '--token', $ghToken
)
if ($version -match '-(alpha|beta|rc)') {
    $uploadArgs += '--pre'
}
& vpk @uploadArgs
if ($LASTEXITCODE -ne 0) { throw 'vpk upload failed' }

Write-Host '=== gh release upload (Tauri assets) ===' -ForegroundColor Cyan
gh release upload $tag $setupPath $sigPath $latestPath --repo $releaseRepo --clobber
if ($LASTEXITCODE -ne 0) { throw 'gh release upload failed' }

# --- Verify --------------------------------------------------------------
Write-Host '=== Release published ===' -ForegroundColor Green
gh release view $tag --repo $releaseRepo

# --- Cleanup -------------------------------------------------------------
Remove-Item -Recurse -Force $staging
if ($releaseNotesFile -and (Test-Path $releaseNotesFile)) {
    Remove-Item -Force $releaseNotesFile
}
Write-Host "Done. Tag: $tag" -ForegroundColor Green
Write-Host "" -ForegroundColor Yellow
Write-Host "NEXT STEPS:" -ForegroundColor Yellow
Write-Host "  1. v0.4.31 user(s) update via Velopack -- expect 5-10 min 'Applying...' hang." -ForegroundColor Yellow
Write-Host "  2. Confirm BOTH machines are on v0.4.32 before shipping v0.4.33." -ForegroundColor Yellow
Write-Host "  3. v0.4.33+ uses scripts/release.ps1 (clean Tauri-only path)." -ForegroundColor Yellow
