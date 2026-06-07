<#
  Phase 3b prototype - edit-applying-swarm safety harness (proof-of-mechanism).

  Proves the load-bearing Windows mechanic for the worktree+verify safety layer
  (see docs/design/edit-swarm-safety-layer.md):

    git worktree (isolation) -> node_modules junction -> apply edit
      -> verify gate (npm run check) -> verdict (accept / auto-revert)
      -> SAFE cleanup (rmdir junction, then worktree remove) -> main tree untouched

  Spawns two "agents":
    agent-clean  : harmless comment edit   -> gate PASS  -> would-merge (shows diff)
    agent-break  : injected TS type error  -> gate FAIL  -> auto-revert

  This is a PROTOTYPE, not production code. It does NOT modify the main checkout
  (no merge-back) and is safe to run while `tauri dev` is alive: the gate is the
  read-only frontend check only, never `cargo check` on the main tree.

  Run:  powershell -NoProfile -File scripts/proto/swarm-harness.ps1
        pwsh -File scripts/proto/swarm-harness.ps1 -KeepWorktrees   # debug: skip cleanup
#>
[CmdletBinding()]
param(
  [string]$Repo = (Resolve-Path "$PSScriptRoot\..\..").Path,
  [switch]$KeepWorktrees
)

# NOT 'Stop': native git/npm write informational text to stderr; under Stop that
# becomes a terminating NativeCommandError (PS5.1 trap). Check exit codes instead.
$ErrorActionPreference = 'Continue'
$protoRoot = Join-Path $env:TEMP 'rift-swarm-proto'

function Write-Step($msg) { Write-Host "  $msg" -ForegroundColor DarkCyan }

# Remove a junction WITHOUT recursing into / deleting its target.
# `cmd /c rmdir` deletes the reparse point only; Remove-Item -Recurse would
# follow the link and wipe the real node_modules. This is the load-bearing
# safety detail (see design doc section 4.2).
function Remove-Junction($path) {
  if (Test-Path $path) { cmd /c rmdir "$path" | Out-Null }
}

function Invoke-SwarmAgent {
  param([string]$Name, [string]$TargetRel, [ValidateSet('clean','break')][string]$EditKind)

  $wt = Join-Path $protoRoot $Name
  $nm = Join-Path $wt 'node_modules'
  $result = [ordered]@{ agent = $Name; target = $TargetRel; edit = $EditKind; gate = $null; verdict = $null }

  Write-Host "`n[$Name] target=$TargetRel edit=$EditKind" -ForegroundColor Yellow
  try {
    # 1. isolation
    Write-Step '1. git worktree add --detach'
    git -C $Repo worktree add --detach $wt HEAD 2>$null | Out-Null
    if ($LASTEXITCODE -ne 0) { throw "worktree add failed (exit $LASTEXITCODE)" }

    # 2. junction node_modules so the gate can run without a reinstall
    Write-Step '2. junction node_modules'
    New-Item -ItemType Junction -Path $nm -Target (Join-Path $Repo 'node_modules') | Out-Null

    # 3. apply the edit
    Write-Step "3. apply $EditKind edit to $TargetRel"
    $target = Join-Path $wt $TargetRel
    if ($EditKind -eq 'clean') {
      Add-Content -LiteralPath $target -Value "`n// [proto] swarm-harness no-op edit"
    } else {
      Add-Content -LiteralPath $target -Value "`nexport const __protoTypeError: number = 'definitely not a number';"
    }

    # 4. verify gate (the same command `/check` runs)
    Write-Step '4. verify gate: npm run check'
    Push-Location $wt
    try {
      $gateOut = & npm run check 2>&1 | Out-String
      $code = $LASTEXITCODE
    } finally { Pop-Location }
    $result.gate = if ($code -eq 0) { 'PASS' } else { "FAIL (exit $code)" }

    # show the last few gate lines for evidence
    ($gateOut -split "`n" | Where-Object { $_ -match 'svelte-check|error|Error|warning|found \d' } |
      Select-Object -Last 4) | ForEach-Object { Write-Host "       | $_" -ForegroundColor DarkGray }

    # 5. verdict
    if ($code -eq 0) {
      $result.verdict = 'ACCEPT (would cherry-pick)'
      Write-Step "5. gate PASS -> ACCEPT. Diff that would merge:"
      $diff = git -C $wt --no-pager diff -- $TargetRel | Out-String
      ($diff -split "`n" | Select-Object -Last 6) | ForEach-Object { Write-Host "       $_" -ForegroundColor DarkGreen }
    } else {
      $result.verdict = 'AUTO-REVERT (discard worktree)'
      Write-Step '5. gate FAIL -> AUTO-REVERT, flag for human. Main tree untouched.'
    }
  }
  finally {
    if (-not $KeepWorktrees) {
      Write-Step '6. cleanup: rmdir junction, then worktree remove'
      Remove-Junction $nm
      git -C $Repo worktree remove --force $wt 2>$null | Out-Null
    } else {
      Write-Step "   (kept worktree at $wt)"
    }
  }
  [pscustomobject]$result
}

# --- run ---------------------------------------------------------------------
Write-Host "rift edit-swarm safety harness - proof of mechanism" -ForegroundColor Cyan
Write-Host "repo: $Repo" -ForegroundColor DarkGray
New-Item -ItemType Directory -Force -Path $protoRoot | Out-Null

$before = (git -C $Repo status --porcelain | Measure-Object).Count

$results = @(
  Invoke-SwarmAgent -Name 'agent-clean' -TargetRel 'src/lib/state/assistant/sessionLog.ts' -EditKind 'clean'
  Invoke-SwarmAgent -Name 'agent-break' -TargetRel 'src/lib/state/assistant/sessionLog.ts' -EditKind 'break'
)

$after = (git -C $Repo status --porcelain | Measure-Object).Count

Write-Host "`n=== summary ===" -ForegroundColor Cyan
$results | Format-Table -AutoSize | Out-String | Write-Host
$isoOk = ($before -eq $after)
$isoMsg = if ($isoOk) { 'OK - main checkout unchanged' } else { 'VIOLATED' }
Write-Host ("main-tree isolation: {0} (porcelain entries before={1} after={2})" -f $isoMsg, $before, $after) -ForegroundColor $(if ($isoOk) { 'Green' } else { 'Red' })
git -C $Repo worktree prune
