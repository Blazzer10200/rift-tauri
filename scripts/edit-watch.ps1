#!/usr/bin/env pwsh
# Live watcher for the rift-edit-swarm (command-prompt / PowerShell version).
# Usage from cmd:  scripts\edit-watch.cmd  [run_id]
#       or direct: powershell -ExecutionPolicy Bypass -File scripts\edit-watch.ps1 [run_id]
param([string]$RunId = "")

$Root = "$env:USERPROFILE\.claude\projects\c--AI-Workflow-projects-rift-tauri"

function Find-Dir {
    $pat = if ($RunId) { $RunId } else { "wf_*" }
    Get-ChildItem -Path $Root -Directory -ErrorAction SilentlyContinue |
        ForEach-Object { Join-Path $_.FullName "subagents\workflows" } |
        Where-Object { Test-Path $_ } |
        ForEach-Object { Get-ChildItem -Path $_ -Directory -Filter $pat -ErrorAction SilentlyContinue } |
        Sort-Object LastWriteTime -Descending |
        Select-Object -First 1
}

while ($true) {
    Clear-Host
    $dir = Find-Dir
    if (-not $dir) { Write-Host "waiting for a workflow run..."; Start-Sleep 2; continue }
    $j = Join-Path $dir.FullName "journal.jsonl"
    Write-Host "=== EDIT SWARM - $($dir.Name) ==="
    if (-not (Test-Path $j)) { Write-Host " waiting for journal..."; Start-Sleep 2; continue }

    $lines = Get-Content $j -ErrorAction SilentlyContinue
    function C([string]$pat) { ($lines | Select-String -SimpleMatch $pat).Count }

    $st = C '"type":"started"'
    $rs = C '"type":"result"'
    Write-Host (" started:{0}  done:{1}  in-flight:{2}" -f $st, $rs, ($st - $rs))
    Write-Host " ---"
    Write-Host (" PLAN   patches proposed:{0}   deferred:{1}" -f (C '"action":"fix"'), (C '"action":"defer"'))
    Write-Host (" VERIFY checked:{0}   safe:{1}" -f (C '"old_string_matches"'), (C '"safe":true'))
    Write-Host " ---"
    if ($lines.Count -gt 0) {
        $lines | Select-Object -Last 2 | ForEach-Object {
            $s = $_ -replace '\{"type":"', '' -replace '","key.*agentId":"', '  ' -replace '".*$', ''
            Write-Host "  $s"
        }
    }
    Start-Sleep 2
}
