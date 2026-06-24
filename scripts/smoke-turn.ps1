# smoke-turn.ps1 -- prove a REAL Claude turn completes end-to-end before shipping.
#
# Every Daily log defers the live turn check ("needs a paid turn + folder"). CDP
# proves render; it never proves a real round-trip. This is that missing smoke:
# it spawns the `claude` CLI with the SAME flags Rift's turn path uses
# (turn.rs ~751: -p / --setting-sources project,local / --model) against a
# throwaway temp folder, and asserts an assistant reply + exit 0.
#
#   pwsh scripts/smoke-turn.ps1                 # opus, default prompt
#   pwsh scripts/smoke-turn.ps1 -Model sonnet   # cheaper
#
# Costs ~a cent or two of quota. Manual / pre-ship only -- NOT wired into CI (CI
# has no Claude auth, and a paid turn doesn't belong in an automated gate). What
# it exercises: CLI reachability + auth (keychain or browser login) + the
# `--setting-sources project,local` flag still yielding a reply. The exact class
# of breakage ("you broke something, hello takes forever") chased live before.
[CmdletBinding()]
param(
  [ValidateSet('opus','sonnet','haiku')]
  [string]$Model = 'opus',
  [string]$Prompt = 'Reply with exactly the single word READY and nothing else.',
  [int]$TimeoutSeconds = 60
)

$ErrorActionPreference = 'Stop'

$modelId = @{ opus = 'claude-opus-4-8'; sonnet = 'claude-sonnet-4-6'; haiku = 'claude-haiku-4-5' }[$Model]

if (-not (Get-Command claude -ErrorAction SilentlyContinue)) {
  Write-Error "claude CLI not found on PATH -- install @anthropic-ai/claude-code (DEVELOPING.md section 3)."
  exit 2
}

# Throwaway scoped folder, mirroring Rift opening a workspace.
$work = Join-Path ([System.IO.Path]::GetTempPath()) ("rift-smoke-" + [System.IO.Path]::GetRandomFileName())
New-Item -ItemType Directory -Path $work -Force | Out-Null
try {
  Write-Host "Smoke turn: $Model ($modelId) in $work" -ForegroundColor Cyan
  Push-Location $work

  # Mirror turn.rs's spawn: -p, --setting-sources project,local, --model. Plain
  # text output keeps the assertion simple -- we only need to confirm a non-empty
  # assistant reply came back, not parse the stream-json NDJSON frames.
  $started = Get-Date
  $job = Start-Job -ScriptBlock {
    $env:ANTHROPIC_API_KEY = $null   # force the CLI's own auth path, as Rift does
    # Quote 'project,local' -- bare commas are PowerShell's array operator and
    # would split it into two args (the CLI then rejects "project local").
    claude -p $using:Prompt --setting-sources 'project,local' --model $using:modelId 2>&1
  }

  if (-not (Wait-Job $job -Timeout $TimeoutSeconds)) {
    Stop-Job $job; Remove-Job $job -Force
    Write-Host "[red] Smoke turn TIMED OUT after ${TimeoutSeconds}s -- no reply." -ForegroundColor Red
    Write-Host "      A real turn is not completing. This is the 'hello takes forever' class -- check auth (/status), not effort." -ForegroundColor Red
    exit 1
  }
  $out = (Receive-Job $job) -join "`n"
  $rc = if ($job.State -eq 'Completed') { 0 } else { 1 }
  Remove-Job $job -Force
  $elapsed = [math]::Round(((Get-Date) - $started).TotalSeconds, 1)

  # The CLI can exit 0 while printing a flag/auth error to its output, so exit
  # code alone is not proof of a real turn -- reject known CLI-error signatures.
  $errorSig = $out -match 'Error processing|Invalid setting|Invalid API|Invalid model|not found|Please run|credit balance|rate limit|Unauthorized'
  if ($rc -ne 0 -or [string]::IsNullOrWhiteSpace($out) -or $errorSig) {
    Write-Host "[red] Smoke turn FAILED (exit $rc, ${elapsed}s). Output:" -ForegroundColor Red
    Write-Host $out
    exit 1
  }

  Write-Host "[green] Real turn completed in ${elapsed}s. Reply:" -ForegroundColor Green
  Write-Host ($out.Trim())
  exit 0
}
finally {
  Pop-Location -ErrorAction SilentlyContinue
  Remove-Item -Recurse -Force $work -ErrorAction SilentlyContinue
}
