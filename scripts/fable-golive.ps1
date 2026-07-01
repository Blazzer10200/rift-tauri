# fable-golive.ps1 — one-command Fable 5 re-enable, gated on a REAL live probe.
#
# Fable 5 was pulled 2026-06-30 (Fable/Mythos access gate). This script flips the
# kill-switch back ON *only* after confirming the API actually answers a Fable turn
# — never on a calendar guess. If the probe says the gate is still up, it refuses to
# flip (a premature flip un-gates a model that hard-errors on every turn) and exits
# non-zero so you know it's not live yet. Re-run it as often as you like; it's a
# no-op until Fable actually opens.
#
# What it does when Fable IS available:
#   1. Flips FABLE_DISABLED true→false in BOTH lockstep files:
#        src-tauri/src/assistant/config.rs   (backend)
#        src/lib/state/assistant/helpers.ts  (frontend)
#   2. Verifies: cargo check + npm run check + the Fable-related vitest.
#   3. (default) STOPS there and prints the exact ship commands, so you eyeball the
#      diff before a public release. Pass -Ship to also bump+commit+tag+release.
#
#   pwsh scripts/fable-golive.ps1            # probe → flip files → verify → STOP (review, then ship)
#   pwsh scripts/fable-golive.ps1 -Ship      # probe → flip → verify → bump → commit → tag → CI release
#   pwsh scripts/fable-golive.ps1 -Version 0.83.0 -Ship   # explicit version (default: patch-bump current)
#   pwsh scripts/fable-golive.ps1 -Force     # flip even if the probe says GATED (NOT recommended)

[CmdletBinding()]
param(
  [switch]$Ship,
  [switch]$Force,
  [string]$Version
)

$ErrorActionPreference = 'Stop'
$repoRoot = Resolve-Path "$PSScriptRoot\.."
Set-Location $repoRoot

$configRs  = 'src-tauri/src/assistant/config.rs'
$helpersTs = 'src/lib/state/assistant/helpers.ts'

function Say($msg, $color = 'Cyan') { Write-Host $msg -ForegroundColor $color }

# ── 1. LIVE PROBE ──────────────────────────────────────────────────────────
Say '=== Fable 5 go-live: probing the API (real turn) ==='
$probe = & pwsh -NoProfile -File "$PSScriptRoot/fable-watch.ps1"
$probeExit = $LASTEXITCODE
Write-Host $probe
# fable-watch.ps1 exit: 0 = AVAILABLE, 1 = GATED, 2 = ERROR
if ($probeExit -ne 0 -and -not $Force) {
  $why = switch ($probeExit) { 1 { 'still GATED ("currently unavailable")' } 2 { 'probe ERROR (auth/CLI/network)' } default { "unexpected exit $probeExit" } }
  Say "REFUSING to flip: Fable is $why." Yellow
  Say "Fable is NOT live yet — re-run this script when it opens. (Override with -Force, not recommended.)" Yellow
  exit $probeExit
}
if ($Force -and $probeExit -ne 0) {
  Say "-Force: flipping despite a non-AVAILABLE probe (exit $probeExit). You own this." Red
}
Say 'Fable probe: AVAILABLE — proceeding to flip the kill-switch.' Green

# ── 2. FLIP BOTH FLAGS (lockstep) ──────────────────────────────────────────
function Flip-Flag($path, $pattern, $replacement, $label) {
  $text = [System.IO.File]::ReadAllText($path)
  if ($text -notmatch $pattern) {
    throw "Flip failed: pattern for $label not found in $path (already flipped, or the file drifted — check manually)."
  }
  $new = [regex]::Replace($text, $pattern, $replacement)
  if ($new -eq $text) { throw "Flip made no change in $path ($label) — likely already false." }
  [System.IO.File]::WriteAllText($path, $new)
  Say "  flipped $label → false ($path)" Green
}

Say '=== flipping FABLE_DISABLED true → false (both files) ==='
# Backend: `pub(super) const FABLE_DISABLED: bool = true;`
Flip-Flag $configRs 'const FABLE_DISABLED: bool = true;' 'const FABLE_DISABLED: bool = false;' 'config.rs FABLE_DISABLED'
# Frontend: `export const FABLE_DISABLED = true;`
Flip-Flag $helpersTs 'export const FABLE_DISABLED = true;' 'export const FABLE_DISABLED = false;' 'helpers.ts FABLE_DISABLED'

# ── 3. VERIFY ──────────────────────────────────────────────────────────────
Say '=== verify: cargo check ==='
& cargo check --manifest-path src-tauri/Cargo.toml --quiet
if ($LASTEXITCODE -ne 0) { throw 'cargo check FAILED after flip — NOT shipping. Revert with: git checkout ' + $configRs + ' ' + $helpersTs }

Say '=== verify: npm run check (svelte) ==='
& npm run check
if ($LASTEXITCODE -ne 0) { throw 'svelte-check FAILED after flip — NOT shipping.' }

Say '=== verify: vitest (full suite — Fable touches modelMatrix/effort) ==='
& npx vitest run
if ($LASTEXITCODE -ne 0) { throw 'vitest FAILED after flip — NOT shipping.' }

Say 'All green: cargo + svelte + vitest.' Green

if (-not $Ship) {
  Say ''
  Say '=== FLIPPED + VERIFIED (not shipped) ===' Green
  Say 'Review the diff, then ship with:' Cyan
  Say '  git diff'
  Say '  pwsh scripts/fable-golive.ps1 -Ship        # or re-run with -Ship to do it all'
  Say 'Or ship manually: bump.ps1 <ver> → edit CHANGELOG → commit → git tag vX.Y.Z → git push --tags'
  exit 0
}

# ── 4. SHIP (bump → commit → tag → CI release) ─────────────────────────────
if (-not $Version) {
  $pkg = Get-Content package.json -Raw | ConvertFrom-Json
  $parts = $pkg.version.Split('.')
  $parts[2] = [string]([int]$parts[2] + 1)   # patch bump
  $Version = $parts -join '.'
}
Say "=== ship: bump → $Version ===" Cyan
& pwsh -NoProfile -File "$PSScriptRoot/bump.ps1" $Version
if ($LASTEXITCODE -ne 0) { throw 'bump.ps1 failed' }

Say 'NOTE: update docs/CHANGELOG.md top entry to a `## v' + $Version + '` block for release notes (else notes are skipped).' Yellow

& git add -A
& git commit -m "v$Version -- re-enable Fable 5 (access gate lifted)"
if ($LASTEXITCODE -ne 0) { throw 'git commit failed' }
& git push origin main
if ($LASTEXITCODE -ne 0) { throw 'git push main failed' }
& git tag -a "v$Version" -m "v$Version -- Fable 5 re-enabled"
& git push origin "v$Version"
if ($LASTEXITCODE -ne 0) { throw 'tag push failed' }

Say "=== v$Version tagged + pushed — CI release triggered ===" Green
Say 'Watch: gh run list --repo Blazzer10200/rift-tauri --workflow release.yml --limit 1' Cyan
