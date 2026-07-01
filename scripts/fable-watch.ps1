# fable-watch.ps1 -- detect the moment Claude Fable 5 access is restored.
#
# Fable 5 was pulled 2026-06-14 (US-gov access gate). Anthropic announced
# 2026-06-27 they're "continuing to work with the government to make Fable 5
# available for general use again" -- future tense, not restored yet. Until the
# API actually answers, Rift hides Fable behind the kill-switch (FABLE_DISABLED
# = true in src-tauri/src/assistant/config.rs AND src/lib/state/assistant/
# helpers.ts) so users can't pick a model that hard-errors.
#
# This script spawns the `claude` CLI with the SAME flag Rift's turn path uses
# for the model (turn.rs ~955: --model claude-fable-5) and classifies the result:
#   AVAILABLE  -> the API answered a real turn (is_error:false). RE-ENABLE Fable:
#                 flip FABLE_DISABLED back to false in BOTH files (lockstep), run
#                 `npm run check` + `cargo test`, then ship.
#   GATED      -> still "currently unavailable" -- leave the kill-switch on.
#   ERROR      -> CLI/auth/other failure (not the access gate) -- see the message.
#
#   pwsh scripts/fable-watch.ps1              # one probe, prints status, exits
#   pwsh scripts/fable-watch.ps1 -Watch       # poll every 30 min until AVAILABLE
#   pwsh scripts/fable-watch.ps1 -Watch -IntervalMinutes 60
#
# Costs ~nothing while GATED (a pre-output refusal isn't billed); a real turn
# once it flips is ~a cent. Manual only -- NOT wired into CI (no Claude auth
# there, and a paid turn doesn't belong in an automated gate).
[CmdletBinding()]
param(
  [switch]$Watch,
  [ValidateRange(1, 1440)]
  [int]$IntervalMinutes = 30,
  [int]$TimeoutSeconds = 60
)

$ErrorActionPreference = 'Stop'

$modelId = 'claude-fable-5'

if (-not (Get-Command claude -ErrorAction SilentlyContinue)) {
  Write-Error "claude CLI not found on PATH -- install @anthropic-ai/claude-code (DEVELOPING.md section 3)."
  exit 2
}

# One probe. Returns a PSCustomObject { Status; Detail } where Status is one of
# 'AVAILABLE' | 'GATED' | 'ERROR'. Spawns the CLI exactly as Rift does for the
# model arg, in a throwaway scoped folder so no real workspace is touched.
function Test-FableOnce {
  $work = Join-Path ([System.IO.Path]::GetTempPath()) ("rift-fable-" + [System.IO.Path]::GetRandomFileName())
  New-Item -ItemType Directory -Path $work -Force | Out-Null
  try {
    Push-Location $work
    # stream-json + --verbose so we can read the terminal `result` envelope's
    # is_error + result fields, the same shape Rift's frontend consumes.
    $raw = & claude -p 'Reply with exactly the single word READY and nothing else.' `
      --model $modelId --output-format stream-json --verbose 2>&1 |
      Select-String -Pattern '"type":"result"' | Select-Object -First 1

    $line = if ($raw) { $raw.ToString() } else { '' }

    if ($line -match '"is_error":false') {
      return [PSCustomObject]@{ Status = 'AVAILABLE'; Detail = 'API answered a real turn.' }
    }
    if ($line -match 'currently unavailable') {
      return [PSCustomObject]@{ Status = 'GATED'; Detail = 'Still "Claude Fable 5 is currently unavailable" (access gate).' }
    }
    if ($line -match '"result":"(?<msg>[^"]*)"') {
      return [PSCustomObject]@{ Status = 'ERROR'; Detail = $Matches['msg'] }
    }
    return [PSCustomObject]@{ Status = 'ERROR'; Detail = "No result envelope (timeout or CLI failure). Raw: $line" }
  }
  finally {
    Pop-Location
    Remove-Item -Recurse -Force $work -ErrorAction SilentlyContinue
  }
}

function Show-Result([PSCustomObject]$r) {
  $stamp = (Get-Date).ToString('yyyy-MM-dd HH:mm:ss')
  switch ($r.Status) {
    'AVAILABLE' {
      Write-Host "[$stamp] Fable 5: AVAILABLE -- $($r.Detail)" -ForegroundColor Green
      Write-Host "  -> RE-ENABLE: set FABLE_DISABLED = false in BOTH:" -ForegroundColor Green
      Write-Host "       src-tauri/src/assistant/config.rs" -ForegroundColor Green
      Write-Host "       src/lib/state/assistant/helpers.ts" -ForegroundColor Green
      Write-Host "     then: npm run check  &&  cargo test --manifest-path src-tauri/Cargo.toml" -ForegroundColor Green
    }
    'GATED' { Write-Host "[$stamp] Fable 5: GATED   -- $($r.Detail)" -ForegroundColor Yellow }
    default { Write-Host "[$stamp] Fable 5: ERROR   -- $($r.Detail)" -ForegroundColor Red }
  }
}

if (-not $Watch) {
  $r = Test-FableOnce
  Show-Result $r
  # Exit code doubles as a machine signal: 0 = available, 1 = still gated, 2 = error.
  exit $(switch ($r.Status) { 'AVAILABLE' { 0 } 'GATED' { 1 } default { 2 } })
}

Write-Host "Watching Fable 5 availability (every $IntervalMinutes min). Ctrl-C to stop." -ForegroundColor Cyan
while ($true) {
  $r = Test-FableOnce
  Show-Result $r
  if ($r.Status -eq 'AVAILABLE') {
    # Audible + visible nudge -- the whole point is to catch the flip unattended.
    [Console]::Beep(880, 400); [Console]::Beep(1175, 600)
    Write-Host "`n*** Fable 5 is BACK. Re-enable the kill-switch and ship. ***`n" -ForegroundColor Green
    exit 0
  }
  Start-Sleep -Seconds ($IntervalMinutes * 60)
}
