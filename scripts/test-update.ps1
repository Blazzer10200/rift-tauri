# Local end-to-end auto-update test harness.
#
# WHY. The in-app update apply chain (download -> swap -> relaunch) has never
# been validated end-to-end without burning a real GitHub release, because the
# RIFT_UPDATE_FEED local-feed escape hatch was compiled out of release builds
# (the only build type that can apply). This drives the WHOLE chain locally.
#
# WHAT IT DOES.
#   1. Builds ONE release binary with the `update-test-feed` feature (so the
#      release binary honors RIFT_UPDATE_FEED).
#   2. Packs it under the ISOLATED pack id `RiftUpdateTest` at TWO versions:
#        - the current version  -> installed as the baseline ("from")
#        - $ToVersion           -> placed in a local feed dir ("to")
#      Velopack keys install location by pack id, so this installs to
#      %LocalAppData%\RiftUpdateTest and CANNOT touch your real `Rift` install.
#   3. Launches the installed baseline with RIFT_UPDATE_FEED pointed at the feed.
#      The updater sees $ToVersion; you click Update (or it auto-checks), and the
#      real apply path runs: reap rift-tauri.exe children -> swap current\ ->
#      relaunch. Velopack's release metadata (not the compiled version string)
#      drives the swap, so a single build proves the mechanics.
#   4. Polls for the swap and prints PASS/FAIL.
#
# This validates the SWAP MECHANICS (the T9 child-lock fix). The compiled
# app_version() still reports the baseline version because it's one build -- if
# you want the in-UI version number to visibly change too, run with -TwoBuilds.
#
# Usage (Windows PowerShell 5.1 local; `pwsh` 7 also works if installed):
#   powershell -File scripts/test-update.ps1                 # to = current patch + 1
#   powershell -File scripts/test-update.ps1 -ToVersion 9.9.9
#   powershell -File scripts/test-update.ps1 -Manual         # set up, then let me click
#   powershell -File scripts/test-update.ps1 -Cleanup        # uninstall the test app + feed

param(
    [string]$ToVersion,
    [switch]$Manual,
    [switch]$TwoBuilds,
    [switch]$Cleanup
)

$ErrorActionPreference = 'Stop'
$repoRoot = Resolve-Path "$PSScriptRoot\.."
Set-Location $repoRoot

$PackId    = 'RiftUpdateTest'
$installDir = Join-Path $env:LOCALAPPDATA $PackId
$feedDir   = Join-Path $repoRoot "Releases/test-feed"
$stageRoot = Join-Path $repoRoot "Releases/test-stage"

function Step($m) { Write-Host "=== $m ===" -ForegroundColor Cyan }

# --- Cleanup mode --------------------------------------------------------
if ($Cleanup) {
    Step "Cleanup"
    $uninst = Join-Path $installDir 'Update.exe'
    if (Test-Path $uninst) {
        Write-Host "Uninstalling $PackId (Velopack owns its install dir) ..." -ForegroundColor Yellow
        # Wait for Update.exe to finish: it kills the app, removes shortcuts +
        # registry, then schedules a delayed `rmdir` of the install dir. Racing
        # it with our own Remove-Item triggers "access denied" (the exe is still
        # held). Let it own the install dir; we only clear the repo scratch.
        $p = Start-Process -FilePath $uninst -ArgumentList '--uninstall', '--silent' -PassThru -Wait
        # Poll for Velopack's scheduled rmdir (cmd `choice /T 3` + rmdir) to land.
        $deadline = (Get-Date).AddSeconds(15)
        while ((Test-Path $installDir) -and (Get-Date) -lt $deadline) { Start-Sleep -Milliseconds 500 }
    }
    # Repo scratch dirs are ours, not Velopack's -- always force-remove.
    foreach ($d in @($feedDir, $stageRoot)) {
        if (Test-Path $d) { Remove-Item -Recurse -Force $d -ErrorAction SilentlyContinue; Write-Host "  removed $d" -ForegroundColor DarkGray }
    }
    # Fallback: if Velopack's deferred rmdir didn't complete, force it now.
    if (Test-Path $installDir) {
        try { Remove-Item -Recurse -Force $installDir -ErrorAction Stop; Write-Host "  removed $installDir (fallback)" -ForegroundColor DarkGray }
        catch { Write-Host "  WARN: $installDir still locked -- rerun -Cleanup or delete manually" -ForegroundColor Yellow }
    } else { Write-Host "  removed $installDir (Velopack)" -ForegroundColor DarkGray }
    Write-Host "Done." -ForegroundColor Green
    return
}

# --- Preflight -----------------------------------------------------------
foreach ($t in @('npm', 'vpk')) {
    if (-not (Get-Command $t -ErrorAction SilentlyContinue)) { throw "$t not found on PATH" }
}

$pkg = Get-Content package.json -Raw | ConvertFrom-Json
$fromVersion = $pkg.version
if (-not $ToVersion) {
    if ($fromVersion -notmatch '^(\d+)\.(\d+)\.(\d+)') { throw "cannot derive +1 from version '$fromVersion'" }
    $ToVersion = "{0}.{1}.{2}" -f $matches[1], $matches[2], ([int]$matches[3] + 1)
}
Write-Host "Test update: $fromVersion  ->  $ToVersion  (pack id: $PackId)" -ForegroundColor Green

$targetRoot = if ($env:CARGO_TARGET_DIR) { $env:CARGO_TARGET_DIR } else { 'src-tauri/target' }
$exePath = Join-Path $targetRoot 'release/rift-tauri.exe'

# --- Build (release, with update-test-feed) ------------------------------
# `tauri build -- <args>` forwards extra args to `cargo build`, so the feature
# lands on the actual release binary that vpk wraps.
function Build-WithFeature {
    Step "tauri build (release, --features update-test-feed)"
    & npm run tauri build -- --features update-test-feed
    if ($LASTEXITCODE -ne 0) { throw 'tauri build failed' }
    if (-not (Test-Path $exePath)) { throw "exe not produced: $exePath" }
}

# Pack the freshly-built exe under $PackId at $ver into $out.
function Pack-Version([string]$ver, [string]$out) {
    $stage = Join-Path $stageRoot $ver
    if (Test-Path $stage) { Remove-Item -Recurse -Force $stage }
    New-Item -ItemType Directory -Path $stage -Force | Out-Null
    Copy-Item $exePath $stage
    Copy-Item 'src-tauri/icons/icon.ico' $stage
    if (-not (Test-Path $out)) { New-Item -ItemType Directory -Path $out -Force | Out-Null }
    Step "vpk pack $PackId $ver"
    & vpk pack -u $PackId -v $ver -p $stage -e 'rift-tauri.exe' -c win `
        --packTitle 'Rift (update test)' --packAuthors 'Blazzer' `
        --icon "$stage/icon.ico" --delta None -o $out
    if ($LASTEXITCODE -ne 0) { throw "vpk pack ($ver) failed" }
}

if (Test-Path $feedDir) { Remove-Item -Recurse -Force $feedDir }

Build-WithFeature
# Baseline (from): pack into a temp dir, install via its Setup.exe.
$fromOut = Join-Path $stageRoot 'from-release'
Pack-Version $fromVersion $fromOut

Step "Install baseline v$fromVersion to $installDir"
$setup = Get-ChildItem -Path $fromOut -Filter "*Setup.exe" | Select-Object -First 1
if (-not $setup) { throw "no Setup.exe produced in $fromOut" }
& $setup.FullName --silent
# Velopack Setup returns immediately; wait for the install to settle.
$deadline = (Get-Date).AddSeconds(60)
while (-not (Test-Path (Join-Path $installDir 'current')) -and (Get-Date) -lt $deadline) {
    Start-Sleep -Milliseconds 500
}
if (-not (Test-Path (Join-Path $installDir 'current'))) { throw "baseline install did not appear at $installDir" }

# Target (to): optionally rebuild so app_version() visibly changes, then pack
# into the feed.
if ($TwoBuilds) {
    Step "Re-stamp version files to $ToVersion for second build"
    & "$PSScriptRoot/bump.ps1" $ToVersion
    try { Build-WithFeature } finally {
        Step "Restoring version files to $fromVersion"
        & "$PSScriptRoot/bump.ps1" $fromVersion
    }
}
Pack-Version $ToVersion $feedDir

# --- Drive the update ----------------------------------------------------
$installedExe = Join-Path $installDir 'current/rift-tauri.exe'
Write-Host ""
Write-Host "Feed ready: $feedDir (has v$ToVersion)" -ForegroundColor Green
Write-Host "Baseline installed: $installedExe (v$fromVersion)" -ForegroundColor Green
Write-Host ""
Step "Launching baseline with RIFT_UPDATE_FEED -> feed"
$env:RIFT_UPDATE_FEED = $feedDir
Start-Process -FilePath $installedExe

if ($Manual) {
    Write-Host "MANUAL: in the launched app, trigger a check + Update. It should" -ForegroundColor Yellow
    Write-Host "download v$ToVersion, close, swap, and relaunch. Then run:" -ForegroundColor Yellow
    Write-Host "  powershell -File scripts/test-update.ps1 -Cleanup" -ForegroundColor Yellow
    return
}

# Auto-verify: the swap drops a RiftUpdateTest-<to>-full.nupkg into packages\
# and replaces current\. Poll for the target nupkg as the success signal.
Step "Waiting for apply (download + swap) -- click Update in the app if not automatic"
$toNupkg = "*$PackId-$ToVersion-*full.nupkg"
$pkgsDir = Join-Path $installDir 'packages'
$deadline = (Get-Date).AddSeconds(180)
$applied = $false
while ((Get-Date) -lt $deadline) {
    if ((Test-Path $pkgsDir) -and (Get-ChildItem $pkgsDir -Filter $toNupkg -ErrorAction SilentlyContinue)) {
        $applied = $true; break
    }
    Start-Sleep -Seconds 2
}

Write-Host ""
if ($applied) {
    Write-Host "PASS: v$ToVersion package staged + applied -- the swap chain works." -ForegroundColor Green
    Write-Host "  (run 'powershell -File scripts/test-update.ps1 -Cleanup' to remove the test install)" -ForegroundColor DarkGray
} else {
    Write-Host "FAIL/TIMEOUT: no v$ToVersion package after 180s." -ForegroundColor Red
    Write-Host "  Check %LOCALAPPDATA%\$PackId and rift.log. The update may not have been triggered" -ForegroundColor Red
    Write-Host "  in-app, or the apply silently no-opped (the historical child-lock bug)." -ForegroundColor Red
}
