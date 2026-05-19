# Rift release pipeline -- local execution.
#
# Reads version from package.json (must match Cargo.toml), runs `npm run tauri
# build`, packs with Velopack, publishes to GitHub releases. Unsigned for now
# (audit H4 -- signing deferred until cert + AAS budget is in place).
#
# Two-repo split: source lives in private `rift-tauri`, releases publish to
# public `rift-releases` so unauthenticated AutoSource clients can fetch
# updates. Velopack-rust 0.0.1298 has no auth in AutoSource -- the public
# releases repo is the only no-fork path.
#
# Prereqs on PATH: npm, vpk (`dotnet tool install -g vpk`), gh (logged in).
#
# Bump versions BEFORE running this script -- use `scripts/bump.ps1 <version>`
# to write the three lockstep files (package.json + Cargo.toml + tauri.conf.json)
# in one shot. This script never auto-bumps; use `/git-ship` for the full
# bump-and-publish pipeline.
#
# Release notes are pulled from the top entry of `docs/CHANGELOG.md`. The top
# entry's version must match the bumped version or the notes are skipped.
#
# Usage:  pwsh ./scripts/release.ps1

$ErrorActionPreference = 'Stop'

$repoRoot = Resolve-Path "$PSScriptRoot\.."
Set-Location $repoRoot

Write-Host '=== Rift release pipeline ===' -ForegroundColor Cyan

# --- Preflight: version sync ---------------------------------------------
$pkg = Get-Content package.json -Raw | ConvertFrom-Json
$cargoText = Get-Content src-tauri/Cargo.toml -Raw
if ($cargoText -notmatch '(?ms)^\s*version\s*=\s*"([^"]+)"') {
    throw 'Cargo.toml: cannot parse version field'
}
$cargoVer = $matches[1]
$tauriCfg = Get-Content src-tauri/tauri.conf.json -Raw | ConvertFrom-Json
if ($pkg.version -ne $cargoVer -or $pkg.version -ne $tauriCfg.version) {
    throw "Version mismatch: package.json=$($pkg.version), Cargo.toml=$cargoVer, tauri.conf.json=$($tauriCfg.version)"
}
$version = $pkg.version
$tag = "v$version"
Write-Host "Version: $version (tag $tag)" -ForegroundColor Green

# --- Preflight: extract release notes from CHANGELOG --------------------
# Pull the top `## v<version>` entry body. Only used if the top entry's
# version matches the bumped version -- otherwise we'd ship stale notes from
# a prior release. Silently skipped (warn only) if missing; release still ships.
$releaseNotesFile = $null
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
                $releaseNotesFile = Join-Path ([System.IO.Path]::GetTempPath()) "rift-release-notes-$version.md"
                [System.IO.File]::WriteAllText($releaseNotesFile, $body)
                Write-Host "Release notes: top CHANGELOG entry ($($body.Length) chars)" -ForegroundColor Green
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
foreach ($t in @('npm', 'vpk', 'gh')) {
    if (-not (Get-Command $t -ErrorAction SilentlyContinue)) {
        throw "$t not found on PATH"
    }
}

# --- Preflight: clean git ------------------------------------------------
$dirty = git status --porcelain
if ($dirty) {
    Write-Host 'Working tree dirty:' -ForegroundColor Yellow
    Write-Host $dirty
    $ans = Read-Host 'Continue anyway? (y/N)'
    if ($ans -ne 'y') { exit 1 }
}

# --- Preflight: tag does not already exist ------------------------------
# `gh release view` exits non-zero + writes to stderr when the release isn't
# found, which trips ErrorAction=Stop on PS5.1. Wrap to swallow the not-found
# case and only throw if the release actually exists (exit 0).
$releaseRepo = 'Blazzer10200/rift-releases'
try {
    $null = gh release view $tag --repo $releaseRepo --json tagName 2>&1
    if ($LASTEXITCODE -eq 0) {
        throw "GitHub release $tag already exists in $releaseRepo. Bump version or delete the release first."
    }
} catch [System.Management.Automation.RuntimeException] {
    if ($_.Exception.Message -like '*already exists*') { throw }
    # else: not-found -- proceed
}

# --- Build ---------------------------------------------------------------
Write-Host '=== tauri build ===' -ForegroundColor Cyan
npm run tauri build
if ($LASTEXITCODE -ne 0) { throw 'tauri build failed' }

# CARGO_TARGET_DIR (if set globally) redirects the exe out of src-tauri/target/.
# Resolve against $env:CARGO_TARGET_DIR first, then fall back to the in-tree path.
$exePath = if ($env:CARGO_TARGET_DIR) {
    Join-Path $env:CARGO_TARGET_DIR 'release/rift-tauri.exe'
} else {
    'src-tauri/target/release/rift-tauri.exe'
}
if (-not (Test-Path $exePath)) { throw "exe not produced: $exePath" }

# --- Stage a clean directory for vpk pack -------------------------------
# vpk pack ships every file in -p verbatim. target/release/ contains build
# artifacts we don't want in the package; copy only the exe + icon.
$staging = "Releases/staging-$version"
if (Test-Path $staging) { Remove-Item -Recurse -Force $staging }
New-Item -ItemType Directory -Path $staging -Force | Out-Null
Copy-Item $exePath $staging
Copy-Item 'src-tauri/icons/icon.ico' $staging

# --- vpk pack ------------------------------------------------------------
Write-Host '=== vpk pack ===' -ForegroundColor Cyan
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
# Optional: themed splash for the native Velopack installer dialog. Active
# when `src-tauri/installer-splash.png` exists — drop a 560x140-ish PNG/GIF
# matching the in-app theme there to swap the bland default.
$splashPath = 'src-tauri/installer-splash.png'
if (Test-Path $splashPath) {
    Write-Host "  splash: $splashPath" -ForegroundColor DarkGray
    $packArgs += @('--splashImage', $splashPath)
}
& vpk @packArgs
if ($LASTEXITCODE -ne 0) { throw 'vpk pack failed' }

# --- Upload to GitHub ----------------------------------------------------
# vpk uploads Setup.exe + .nupkg + delta files as release assets and creates
# the release/tag. --publish marks it published (not draft).
Write-Host '=== vpk upload github ===' -ForegroundColor Cyan
$ghToken = (gh auth token).Trim()
if (-not $ghToken) { throw 'gh auth token returned empty -- run `gh auth login` first' }

$uploadArgs = @(
    'upload', 'github',
    '--repoUrl', "https://github.com/$releaseRepo",
    '--publish',
    '--releaseName', $tag,
    '--tag', $tag,
    '--token', $ghToken
)
if ($version -match '-(alpha|beta|rc)') {
    $uploadArgs += '--pre'
}
& vpk @uploadArgs
if ($LASTEXITCODE -ne 0) { throw 'vpk upload failed' }

# --- Verify --------------------------------------------------------------
Write-Host '=== Release published ===' -ForegroundColor Green
gh release view $tag --repo $releaseRepo

# --- Cleanup -------------------------------------------------------------
Remove-Item -Recurse -Force $staging
if ($releaseNotesFile -and (Test-Path $releaseNotesFile)) {
    Remove-Item -Force $releaseNotesFile
}
Write-Host "Done. Tag: $tag" -ForegroundColor Green
