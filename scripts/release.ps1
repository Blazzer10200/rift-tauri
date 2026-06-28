# Rift release pipeline -- Velopack path (v0.4.47+).
#
# Builds the app, packs it with Velopack (`vpk pack`), and publishes to the
# public `rift-releases` GitHub repo (`vpk upload github`). Installed clients
# run Velopack's UpdateManager over the native GithubSource: check on launch +
# every 6h, one-click download, then unattended apply-on-exit + relaunch.
# See src-tauri/src/update_service.rs (arc: git log -- docs/design/velopack-auto-update.md).
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
#         pwsh ./scripts/release.ps1 -Ci      # CI mode (tag-driven, see release.yml)
#
# CI mode (-Ci): driven by `.github/workflows/release.yml` on a `v*` tag push.
# Skips the interactive dirty-tree refusal (a fresh checkout is clean) and, when
# `GITHUB_REF_NAME` is set, asserts the pushed tag matches the bumped version --
# a half-bumped tag can never produce a broken release. Auth comes from -Token
# (the RELEASES_TOKEN PAT scoped to rift-releases) instead of `gh auth token`.

param(
    [switch]$Force,
    [switch]$Ci,
    [string]$Token
)

$ErrorActionPreference = 'Stop'

$repoRoot = Resolve-Path "$PSScriptRoot\.."
Set-Location $repoRoot

# In CI, gh + vpk authenticate against rift-releases via the passed PAT. A GitHub
# PAT is pure printable ASCII; a stray non-ASCII/control char (BOM, zero-width
# space, NBSP, smart-quote) pasted into the RELEASES_TOKEN secret survives .Trim()
# and (a) makes Octokit throw "Request headers must contain only ASCII characters"
# at vpk upload AND (b) silently breaks `gh` CLI auth -- which masks the "already
# exists" preflight and no-ops the portable-asset drop (both seen on the v0.8.8
# ship). Strip to printable ASCII ONCE here so EVERY downstream auth path (gh +
# vpk) is clean. Export as GH_TOKEN so the `gh release view`/`delete-asset` calls
# below pick it up without a separate `gh auth login`.
if ($Token) {
    $clean = ($Token -replace '[^\x21-\x7E]', '')
    if ($clean.Length -ne $Token.Length) {
        Write-Host "  WARNING: stripped $($Token.Length - $clean.Length) non-ASCII/whitespace char(s) from RELEASES_TOKEN (copy-paste artifact) -- re-set the secret cleanly." -ForegroundColor Yellow
    }
    $Token = $clean
    $env:GH_TOKEN = $Token
}

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

# --- Preflight: pushed tag must match the bumped version (CI guard) -------
# In a tag-driven CI release the trigger is `git push --tags`. If the tag was
# cut before the three version files were bumped (the classic lockstep miss),
# the build would publish under a tag that disagrees with its own binary. Fail
# fast here instead of shipping a mismatched release.
if ($env:GITHUB_REF_NAME) {
    $tagVer = $env:GITHUB_REF_NAME -replace '^v', ''
    if ($tagVer -ne $version) {
        throw "Tag mismatch: pushed tag $($env:GITHUB_REF_NAME) (v$tagVer) != bumped version v$version. Re-bump (scripts/bump.ps1 $tagVer) or re-tag."
    }
    Write-Host "Tag guard: $($env:GITHUB_REF_NAME) matches bumped version" -ForegroundColor Green
}

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
$releaseTitle = $null
$changelogPath = 'docs/CHANGELOG.md'
if (Test-Path $changelogPath) {
    $clText = [System.IO.File]::ReadAllText($changelogPath)
    $entryPattern = '(?ms)^## v(?<ver>[^\s]+)(?<titleline>[^\r\n]*)\r?\n(?<body>.*?)(?=^## v|\z)'
    $entryRx = New-Object System.Text.RegularExpressions.Regex $entryPattern
    $m = $entryRx.Match($clText)
    if ($m.Success) {
        $topVer = $m.Groups['ver'].Value
        if ($topVer -eq $version) {
            # Release title from the header tail. Header is "vX.Y.Z — YYYY-MM-DD — Title"
            # (date optional); strip the leading separator AND the date segment via an
            # ASCII-only pattern -> GitHub release name. Titles start with a letter.
            $titleClean = ($m.Groups['titleline'].Value -replace '^[^A-Za-z0-9]+(?:\d{4}-\d{2}-\d{2}[^A-Za-z0-9]+)?', '').Trim()
            # Strip embedded double-quotes: PS 5.1 mangles native-exe args that
            # contain them, so vpk's --releaseName splits the title mid-string
            # ("'/' was not matched" on a quoted title). Belt for any future title.
            $titleClean = $titleClean -replace '"', ''
            if ($titleClean) { $releaseTitle = Convert-ToAsciiSafe $titleClean }
            $body = $m.Groups['body'].Value.Trim()
            # Strip relative markdown links (../src-tauri/...) -- they 404 on the
            # public releases repo, which carries no source tree.
            $body = [regex]::Replace($body, '\[([^\]]+)\]\((?:\.{1,2}[\\/])[^)]*\)', '$1')
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

# --- Preflight: R2 creds present in CI (silent-stale-feed guard) ----------
# The app's live update feed is R2 ONLY (update_service.rs UPDATE_FEED_URL) --
# GitHub is NOT a fallback. The R2 dual-publish below is conditional, so a CI run
# missing these secrets would build + pack + publish to GitHub fully green while
# the feed clients actually read goes stale and no update ever reaches them. Fail
# loud here -- BEFORE the expensive build -- rather than skip silently at upload.
# (mega-audit cont.228 F2; arc: git log -- docs/design/self-hosted-distribution.md)
if ($Ci) {
    $missingR2 = @('R2_ACCESS_KEY_ID', 'R2_SECRET_ACCESS_KEY', 'R2_ENDPOINT') |
        Where-Object { -not (Get-Item "env:$_" -ErrorAction SilentlyContinue) }
    if ($missingR2) {
        throw "release.ps1: CI run is missing R2 secret(s): $($missingR2 -join ', '). The live update feed is R2-only -- shipping without them silently strands clients on a stale feed. Set them as repo secrets and re-run."
    }
}

# --- Preflight: clean git ------------------------------------------------
# Skipped in CI: a tag-push checkout is clean by definition, and the build step
# may have already written node_modules/build artifacts before this runs.
if (-not $Ci) {
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
}

# --- Preflight: tag does not already exist ------------------------------
$releaseRepo = 'Blazzer10200/rift'
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
# produces its own Setup.exe, so Tauri's NSIS bundle output is unused here --
# `--no-bundle` skips the makensis step (and its per-run NSIS downloads), which
# only ever produced a discarded installer. vpk packs release/rift-tauri.exe.
Write-Host '=== tauri build ===' -ForegroundColor Cyan
& npm run tauri build -- --no-bundle
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
    # No delta packages: keeps the release asset list lean (one fewer .nupkg per
    # release). Clients just download the full package -- a non-issue at this
    # app size + user base. Revert to the default by dropping this flag.
    '--delta', 'None',
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

# --- Upload to GitHub ----------------------------------------------------
# vpk uploads Setup.exe + full .nupkg + releases.win.json as release assets
# and creates/publishes the release. --channel win matches the pack channel +
# the client manifest. --pre for alpha/beta/rc: the client's
# GithubSource(prerelease:true) reads the prerelease list, so pre-releases are
# visible (unlike the old GH-release-API `/latest` path, which excluded them).
Write-Host '=== vpk upload github ===' -ForegroundColor Cyan
# In CI, -Token carries the rift-releases PAT; locally, fall back to the gh
# session token.
# $Token was already stripped to printable ASCII up top (shared with GH_TOKEN);
# locally, fall back to the gh session token.
$ghToken = if ($Token) { $Token } else { (gh auth token).Trim() }
if (-not $ghToken) { throw 'no GitHub token -- pass -Token <pat> (CI) or run `gh auth login` (local)' }

$uploadArgs = @(
    'upload', 'github',
    '-o', 'Releases',
    '--repoUrl', "https://github.com/$releaseRepo",
    '--channel', 'win',
    '--publish',
    '--releaseName', $(if ($releaseTitle) { "$tag $([char]0x2014) $releaseTitle" } else { $tag }),
    '--tag', $tag,
    '--token', $ghToken
)
if ($version -match '-(alpha|beta|rc)') {
    $uploadArgs += '--pre'
}
& vpk @uploadArgs
if ($LASTEXITCODE -ne 0) { throw 'vpk upload failed' }

# --- Optional: dual-publish to Cloudflare R2 (self-hosted feed) -------------
# Fires only when R2 creds are present (CI secrets). WARNING: the app's live
# update feed IS R2 only (update_service.rs UPDATE_FEED_URL) -- GitHub is NOT a
# fallback. If R2 creds are absent here, the R2 feed goes stale and shipped
# clients see no new update. Fix R2_ACCESS_KEY_ID / R2_SECRET_ACCESS_KEY /
# R2_ENDPOINT in CI secrets. (arc: git log -- docs/design/self-hosted-distribution.md)
if ($env:R2_ACCESS_KEY_ID -and $env:R2_SECRET_ACCESS_KEY -and $env:R2_ENDPOINT) {
    Write-Host '=== vpk upload s3 (Cloudflare R2) ===' -ForegroundColor Cyan
    $r2Args = @(
        'upload', 's3',
        '-o', 'Releases',
        '--bucket', $(if ($env:R2_BUCKET) { $env:R2_BUCKET } else { 'rift-releases' }),
        '--endpoint', $env:R2_ENDPOINT,
        '--keyId', $env:R2_ACCESS_KEY_ID,
        '--secret', $env:R2_SECRET_ACCESS_KEY,
        '--channel', 'win'
    )
    & vpk @r2Args
    if ($LASTEXITCODE -ne 0) { throw 'vpk upload s3 (R2) failed' }
} else {
    # In CI this branch is unreachable -- the R2 preflight above hard-fails on
    # missing creds. Only a local (-Ci-less) run lands here, where skipping the
    # dual-publish is fine (local builds aren't the shipped feed).
    Write-Host 'R2 env not set -- skipping S3 dual-publish (local run; GitHub feed only).' -ForegroundColor DarkGray
}

# --- Drop the portable zip from the published release --------------------
# vpk's pack manifest lists the portable as an upload asset, so the file must
# exist at upload time -- we remove it from the release afterward. We publish
# Setup.exe only so a new user has one obvious "download + run" path. Safe:
# the portable isn't part of the Velopack update feed (releases.win.json
# references the .nupkgs). Non-fatal -- a failure here doesn't unship.
Write-Host '=== dropping portable asset from release ===' -ForegroundColor Cyan
& gh release delete-asset $tag 'Rift-win-Portable.zip' --repo $releaseRepo -y
if ($LASTEXITCODE -ne 0) { Write-Host '  (portable asset not present or already removed)' -ForegroundColor DarkGray }
$global:LASTEXITCODE = 0

# --- Verify --------------------------------------------------------------
Write-Host '=== Release published ===' -ForegroundColor Green
gh release view $tag --repo $releaseRepo

# --- Cleanup -------------------------------------------------------------
Remove-Item -Recurse -Force $staging
if ($releaseNotesFile -and (Test-Path $releaseNotesFile)) {
    Remove-Item -Force $releaseNotesFile
}
Write-Host "Done. Tag: $tag" -ForegroundColor Green
