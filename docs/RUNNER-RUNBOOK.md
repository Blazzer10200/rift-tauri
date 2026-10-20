# Self-hosted runner runbook (`proxmox-win`)

`check.yml`, `audit.yml`, and `release.yml` use the single Windows runner labeled
`[self-hosted, windows, tauri-build]`. Its warm Cargo cache keeps builds fast,
but a wedged listener blocks the entire queue. Pull requests do not automatically
run the self-hosted check workflow; it runs on trusted repository events.

## Normal release monitoring

```powershell
pwsh scripts/ship-watch.ps1 vX.Y.Z -TimeoutSeconds 1800
```

The helper locates the tag's release run, polls every 15 seconds, fails on a red
conclusion, and exits `124` on timeout.

## Queue-stall symptom

A tag or main push remains `queued` and never starts. Workflow
`timeout-minutes` begins only after a runner accepts the job, so it does not end
a dispatch stall.

Check the registered runner:

```powershell
gh api repos/Blazzer10200/rift-tauri/actions/runners `
  --jq '.runners[] | {name,status,busy}'
```

`online` is not sufficient: a stale listener may continue heartbeating. An
online, non-busy runner with a job queued for more than a minute is probably
wedged.

## Recovery

On `proxmox-win`, restart the runner service:

```powershell
Restart-Service "actions.runner.*"
Get-Service "actions.runner.*" | Format-Table Name,Status
```

For a console installation, stop `run.cmd` with Ctrl+C and launch it again. The
existing job should dispatch; do not create or rewrite a tag.

If the queue entry itself is stuck, cancel and rerun the same workflow:

```powershell
gh run cancel <run-id>
gh run rerun <run-id>
gh run view <run-id> --json status,conclusion,url
```

## Determine whether a release shipped

Publication is feed-first. A red or interrupted run can leave R2 live before the
GitHub half completes. Check both independently:

1. R2 `releases.win.json` lists the version and the public package returns 200.
2. The GitHub release is final and contains the installer, full/delta packages,
   `releases.win.json`, and `RELEASES`.
3. Public package sizes and SHA-256 digests match.

An R2 entry proves clients can see that package, but it does not make the human
download release complete. Repair the missing half by rerunning the same tagged
workflow after confirming the existing public state.

## Prevention

No independent watchdog currently exists: a hosted watcher consumes unavailable
minutes, while a watcher on the same runner cannot detect that runner's failure.
If a second host becomes available, monitor listener/log freshness there and
restart the service only after repeated stale checks.
