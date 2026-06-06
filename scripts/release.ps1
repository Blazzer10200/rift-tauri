# Rift release pipeline -- Velopack path (v0.4.47+).
#
# Builds the app, packs it with Velopack (`vpk pack`), and publishes to the
# public `rift-releases` GitHub repo (`vpk upload github`). Installed clients
# run Velopack's UpdateManager over the native GithubSource: check on launch +
# every 6h, one-click download, then unattended apply-on-exit + relaunch.
# See src-tauri/src/update_service.rs + docs/design/velopack-auto-update.md.
#
# Two-repo split: source lives in private `rift-tauri`; releases publish to
# public `rift-releases` so unauthenticated GithubSource fetches succeed.
#
# CRITICAL -- vpk CLI version MUST equal the `velopack` crate version (both
# pinned to 1.2.0). Bump together: `dotnet tool update -g vpk` + the `=x.y.z`
# pin in src-tauri/Cargo.toml. A mismatch packs an Update runtime incompatible
# with the linked client runtime.
#
# Prereqs on PATH: npm, vpk (`dotnet tool install -g vpk`), gh (logged in).
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

Write-Host '=== Rift release pipeline (Velopack) ===' -ForegroundColor Cyan

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

# --- Preflight: vpk version == velopack crate version --------------------
# Velopack requires the packing CLI and the linked runtime to match exactly.
$crateVer = $null
if ($cargoText -match '(?m)^\s*velopack\s*=\s*"=?([0-9]+\.[0-9]+\.[0-9]+)"') {
    $crateVer = $matches[1]
}
$vpkBanner = (& vpk -h 2>&1 | Select-String 'Velopack CLI ([0-9.]+)')
$vpkVer = if ($vpkBanner) { $vpkBanner.Matches[0].Groups[1].Value } else { $null }
if ($crateVer -and $vpkVer -and ($crateVer -ne $vpkVer)) {
    throw "vpk/crate version mismatch: vpk=$vpkVer, velopack crate=$crateVer. Run ``dotnet tool update -g vpk`` and align the Cargo.toml pin."
}
Write-Host "Velopack: crate=$crateVer, vpk=$vpkVer" -ForegroundColor Green

# --- Preflight: extract release notes from CHANGELOG ---------------------
# ASCII-only source -- \uXXXX in -replace patterns resolves at .NET regex
# compile time, NOT at PS string-parse time, so the source bytes stay ASCII.
# BOM-less .ps1 read as Win-1252 by PS5.1 would mojibake literal multi-byte
# chars. vpk pack embeds the notes into the nuspec <releaseNotes>; Latin-1
# chars trip the XmlReader on read-back (v0.4.26 `1.15x` XmlException), so
# pre-strip to pure ASCII.
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
                $bodyAscii = Convert-ToAsciiSafe $body
                # Sanity: synthesize the nuspec releaseNotes element + parse as
                # XML before handing to vpk. Catches char hazards fast, not
                # after a 5-min build burns.
                $probeXml = "<?xml version='1.0' encoding='utf-8'?><r>$([System.Security.SecurityElement]::Escape($bodyAscii))</r>"
                try {
                    [xml]$null = $probeXml
                } catch {
                    throw "Release notes failed XML sanity probe: $($_.Exception.Message)"
                }
                $releaseNotesFile = Join-Path ([System.IO.Path]::GetTempPath()) "rift-release-notes-$version.md"
                [System.IO.File]::WriteAllText($releaseNotesFile, $bodyAscii)
                $delta = $body.Length - $bodyAscii.Length
                Write-Host "Release notes: top CHANGELOG entry ($($bodyAscii.Length) chars, $delta non-ASCII stripped)" -ForegroundColor Green
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
# `npm run tauri build` builds the frontend (vite) + the release exe (which
# embeds those assets). Velopack wraps the self-contained exe directly and
# produces its own Setup.exe, so Tauri's NSIS bundle output is unused here.
Write-Host '=== tauri build ===' -ForegroundColor Cyan
& npm run tauri build
if ($LASTEXITCODE -ne 0) { throw 'tauri build failed' }

$targetRoot = if ($env:CARGO_TARGET_DIR) { $env:CARGO_TARGET_DIR } else { 'src-tauri/target' }
$exePath = Join-Path $targetRoot 'release/rift-tauri.exe'
if (-not (Test-Path $exePath)) { throw "exe not produced: $exePath" }

# --- Stage a clean directory for vpk pack -------------------------------
# vpk packs every file in -p verbatim. Copy ONLY the exe + window icon.
# If a future Tauri release bundles a WebView2 redistributable, sidecar, or
# any *.dll next to the exe, IT MUST BE ADDED HERE or it'll be missing on
# installed clients.
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
    '-c', 'win',
    '--packTitle', 'Rift',
    '--packAuthors', 'Blazzer',
    '--icon', "$staging/icon.ico",
    '-o', 'Releases'
)
if ($releaseNotesFile) {
    $packArgs += @('--releaseNotes', $releaseNotesFile)
}
# Optional themed installer splash -- drop a ~560x140 PNG/GIF here to swap the
# bland default.
$splashPath = 'src-tauri/installer-splash.png'
if (Test-Path $splashPath) {
    Write-Host "  splash: $splashPath" -ForegroundColor DarkGray
    $packArgs += @('--splashImage', $splashPath)
}
& vpk @packArgs
if ($LASTEXITCODE -ne 0) { throw 'vpk pack failed' }

# --- Drop the portable zip before upload ---------------------------------
# vpk always emits a *-Portable.zip alongside Setup.exe. We publish Setup.exe
# only so a new user has one obvious "download + run" path; the portable build
# just adds a "which one do I click?" fork. Safe to delete -- it's standalone,
# not referenced by releases.win.json / the Velopack update manifest.
$portable = Get-ChildItem -Path 'Releases' -Filter '*-Portable.zip' -ErrorAction SilentlyContinue
foreach ($p in $portable) {
    Write-Host "  dropping portable build: $($p.Name)" -ForegroundColor DarkGray
    Remove-Item -Force $p.FullName
}

# --- Upload to GitHub ----------------------------------------------------
# vpk uploads Setup.exe + .nupkg + delta + releases.win.json as release assets
# and creates/publishes the release. --channel win matches the pack channel +
# the client manifest. --pre for alpha/beta/rc: the client's
# GithubSource(prerelease:true) reads the prerelease list, so pre-releases are
# visible (unlike the old GH-release-API `/latest` path, which excluded them).
Write-Host '=== vpk upload github ===' -ForegroundColor Cyan
$ghToken = (gh auth token).Trim()
if (-not $ghToken) { throw 'gh auth token returned empty -- run `gh auth login` first' }

$uploadArgs = @(
    'upload', 'github',
    '-o', 'Releases',
    '--repoUrl', "https://github.com/$releaseRepo",
    '--channel', 'win',
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
