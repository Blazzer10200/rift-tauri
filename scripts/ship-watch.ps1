# ship-watch.ps1 -- wait on the tag-driven release run and report green/red.
#
# Kills the "confirm the CI release landed green next session" ritual that ends
# every Daily log. Run it right after `git push origin main --tags`:
#
#   pwsh scripts/ship-watch.ps1            # newest v* tag
#   pwsh scripts/ship-watch.ps1 v0.32.1    # a specific tag
#
# Finds the `release` workflow run for the tag, polls until it completes, and
# exits non-zero if it failed or timed out -- so it's safe to chain
# (`... ; if ($?) { echo shipped }`). Needs `gh` authed to the source repo.
[CmdletBinding()]
param(
  [string]$Tag,
  [int]$TimeoutSeconds = 1800   # release runs take ~4-7m; 30m is generous slack
)

$ErrorActionPreference = 'Stop'

if ($TimeoutSeconds -lt 1) {
  Write-Host "TimeoutSeconds must be at least 1." -ForegroundColor Red
  exit 2
}
$overallDeadline = (Get-Date).AddSeconds($TimeoutSeconds)

if (-not (Get-Command gh -ErrorAction SilentlyContinue)) {
  Write-Host "gh CLI not found on PATH -- install it or run the release check manually." -ForegroundColor Red
  exit 2
}

if (-not $Tag) {
  $Tag = git tag --list 'v*' --sort=-creatordate | Select-Object -First 1
  if (-not $Tag) { Write-Host "No v* tag found to watch." -ForegroundColor Red; exit 2 }
}
Write-Host "Watching the release run for tag $Tag ..." -ForegroundColor Cyan

# The release run is keyed on the tag ref; poll briefly for it to register (a
# fresh `git push --tags` can beat GitHub dispatching the run by a few seconds).
$runId = $null
$registrationSeconds = [Math]::Min(60, $TimeoutSeconds)
$registrationDeadline = (Get-Date).AddSeconds($registrationSeconds)
while (-not $runId -and (Get-Date) -lt $registrationDeadline) {
  $runId = gh run list --workflow=release.yml --branch $Tag --limit 1 --json databaseId --jq '.[0].databaseId' 2>$null
  if (-not $runId) {
    $remaining = [Math]::Max(0, [Math]::Floor(($registrationDeadline - (Get-Date)).TotalSeconds))
    if ($remaining -gt 0) { Start-Sleep -Seconds ([Math]::Min(5, $remaining)) }
  }
}
if (-not $runId) {
  Write-Host "No release run found for $Tag within ${registrationSeconds}s. Did the tag push trigger release.yml? Check: gh run list --workflow=release.yml" -ForegroundColor Red
  exit 2
}

Write-Host "Found release run $runId -- polling every 15s (timeout ${TimeoutSeconds}s)..." -ForegroundColor Cyan
$rc = $null
$runUrl = $null
while ((Get-Date) -lt $overallDeadline) {
  $json = gh run view $runId --json status,conclusion,url
  if ($LASTEXITCODE -ne 0) {
    Write-Host "Could not read release run $runId. Check: gh run view $runId" -ForegroundColor Red
    exit 2
  }
  $run = $json | ConvertFrom-Json
  $runUrl = $run.url
  if ($run.status -eq 'completed') {
    $rc = if ($run.conclusion -eq 'success') { 0 } else { 1 }
    break
  }

  $remaining = [Math]::Max(0, [Math]::Ceiling(($overallDeadline - (Get-Date)).TotalSeconds))
  Write-Host "  $($run.status) -- ${remaining}s remaining" -ForegroundColor DarkGray
  if ($remaining -gt 0) { Start-Sleep -Seconds ([Math]::Min(15, $remaining)) }
}

if ($null -eq $rc) {
  Write-Host "`n[red] Timed out after ${TimeoutSeconds}s waiting for release $Tag (run $runId)." -ForegroundColor Red
  if ($runUrl) { Write-Host "       $runUrl" -ForegroundColor Red }
  exit 124
}

if ($rc -eq 0) {
  Write-Host "`n[green] Release $Tag published successfully (run $runId)." -ForegroundColor Green
} else {
  Write-Host "`n[red] Release $Tag did NOT land green (run $runId, exit $rc)." -ForegroundColor Red
  Write-Host "       gh run view $runId --log-failed" -ForegroundColor Red
}
exit $rc
