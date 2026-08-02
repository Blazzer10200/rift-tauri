# Read-only developer environment diagnostic for Rift.

$ErrorActionPreference = 'Stop'

$repoRoot = (Resolve-Path "$PSScriptRoot\..").Path
Set-Location $repoRoot

$rows = New-Object System.Collections.Generic.List[object]
$requiredMissing = $false

function Add-Result {
    param(
        [string]$Name,
        [string]$State,
        [string]$Detail,
        [bool]$Required = $false
    )
    $script:rows.Add([pscustomobject]@{ Check = $Name; State = $State; Detail = $Detail })
    if ($Required -and $State -ne 'ready') { $script:requiredMissing = $true }
}

function Probe-Command {
    param(
        [string]$Name,
        [string]$Command,
        [string[]]$Arguments,
        [bool]$Required = $false
    )
    $resolved = Get-Command $Command -ErrorAction SilentlyContinue
    if (-not $resolved) {
        Add-Result $Name 'missing' "$Command is not on PATH" $Required
        return
    }
    try {
        $output = @(& $resolved.Source @Arguments 2>&1)
        $line = ($output | Where-Object { $_ -and $_.ToString().Trim() } | Select-Object -First 1)
        if ($LASTEXITCODE -eq 0) {
            Add-Result $Name 'ready' (($line | Out-String).Trim()) $Required
        } else {
            Add-Result $Name 'error' (($line | Out-String).Trim()) $Required
        }
    } catch {
        Add-Result $Name 'error' $_.Exception.Message $Required
    }
}

Write-Host '=== Rift project doctor ===' -ForegroundColor Cyan
Write-Host "Repo: $repoRoot"

Probe-Command 'Git' 'git' @('--version') $true
Probe-Command 'Node' 'node' @('--version') $true
Probe-Command 'npm' 'npm' @('--version') $true
Probe-Command 'Rust' 'rustc' @('--version') $true
Probe-Command 'Cargo' 'cargo' @('--version') $true
Probe-Command 'Claude CLI' 'claude' @('--version') $false
Probe-Command 'Codex CLI' 'codex' @('--version') $false
Probe-Command 'GitHub CLI' 'gh' @('--version') $false

$branch = (git branch --show-current 2>$null).Trim()
$dirtyCount = @(git status --short 2>$null).Count
Add-Result 'Repository' 'ready' "branch=$branch; changed files=$dirtyCount" $true

$repoPattern = [regex]::Escape($repoRoot)
$devProcesses = @(Get-CimInstance Win32_Process -ErrorAction SilentlyContinue | Where-Object {
    ($_.CommandLine -and $_.CommandLine -match $repoPattern -and
        ($_.CommandLine -match 'tauri(\.js|\.exe)?["'']?\s+dev' -or $_.CommandLine -match 'run-dev.*\.ps1')) -or
    ($_.Name -eq 'rift-tauri.exe' -and $_.ExecutablePath -match '\\cargo-targets\\debug\\rift-tauri\.exe$')
})
if ($devProcesses.Count -gt 0) {
    Add-Result 'Tauri dev' 'running' ("PID " + (($devProcesses.ProcessId | Sort-Object -Unique) -join ', '))
} else {
    Add-Result 'Tauri dev' 'stopped' 'No repo-scoped Tauri dev launcher detected'
}

try {
    $cdp = Invoke-RestMethod -Uri 'http://127.0.0.1:9223/health' -TimeoutSec 2
    $cdpDetail = if ($cdp.ok) { 'wrapper and WebView bridge responding' } else { 'wrapper responded but is not ready' }
    Add-Result 'Rift CDP' $(if ($cdp.ok) { 'ready' } else { 'warning' }) $cdpDetail
} catch {
    Add-Result 'Rift CDP' 'stopped' 'Run npm run cdp:dev when live UI inspection is needed'
}

$rows | Format-Table -AutoSize -Wrap

if ($requiredMissing) {
    Write-Error 'One or more required development tools are missing or unhealthy.'
    exit 1
}

Write-Host 'Doctor complete. Optional provider/CDP tools may be stopped without making the project unhealthy.' -ForegroundColor Green
