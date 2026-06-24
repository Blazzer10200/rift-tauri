# ship-watch.ps1 -- wait on the tag-driven release run and report green/red.
#
# Kills the "confirm the CI release landed green next session" ritual that ends
# every Daily log. Run it right after `git push origin main --tags`:
#
#   pwsh scripts/ship-watch.ps1            # newest v* tag
#   pwsh scripts/ship-watch.ps1 v0.32.1    # a specific tag
#
# Finds the `release` workflow run for the tag, blocks until it completes via
# `gh run watch --exit-status`, and exits non-zero if it failed -- so it's safe
# to chain (`... ; if ($?) { echo shipped }`). Needs `gh` authed to the source repo.
[CmdletBinding()]
param(
  [string]$Tag,
  [int]$TimeoutSeconds = 1800   # release runs take ~4-7m; 30m is generous slack
)

$ErrorActionPreference = 'Stop'

if (-not (Get-Command gh -ErrorAction SilentlyContinue)) {
  Write-Error "gh CLI not found on PATH -- install it or run the release check manually."
  exit 2
}

if (-not $Tag) {
  $Tag = git tag --list 'v*' --sort=-creatordate | Select-Object -First 1
  if (-not $Tag) { Write-Error "No v* tag found to watch."; exit 2 }
}
Write-Host "Watching the release run for tag $Tag ..." -ForegroundColor Cyan

# The release run is keyed on the tag ref; poll briefly for it to register (a
# fresh `git push --tags` can beat GitHub dispatching the run by a few seconds).
$runId = $null
$deadline = (Get-Date).AddSeconds(60)
while (-not $runId -and (Get-Date) -lt $deadline) {
  $runId = gh run list --workflow=release.yml --branch $Tag --limit 1 --json databaseId --jq '.[0].databaseId' 2>$null
  if (-not $runId) { Start-Sleep -Seconds 5 }
}
if (-not $runId) {
  Write-Error "No release run found for $Tag within 60s. Did the tag push trigger release.yml? Check: gh run list --workflow=release.yml"
  exit 2
}

Write-Host "Found release run $runId -- blocking until it finishes (exit status mirrors the run)..." -ForegroundColor Cyan
# --exit-status: non-zero here if the run concludes failed/cancelled.
gh run watch $runId --exit-status --interval 15
$rc = $LASTEXITCODE

if ($rc -eq 0) {
  Write-Host "`n[green] Release $Tag published successfully (run $runId)." -ForegroundColor Green
} else {
  Write-Host "`n[red] Release $Tag did NOT land green (run $runId, exit $rc)." -ForegroundColor Red
  Write-Host "       gh run view $runId --log-failed" -ForegroundColor Red
}
exit $rc
