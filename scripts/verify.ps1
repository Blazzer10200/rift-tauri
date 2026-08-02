# One deterministic local verification entry point for Rift.

[CmdletBinding()]
param(
    [switch]$FrontendOnly,
    [switch]$RustOnly
)

$ErrorActionPreference = 'Stop'

if ($FrontendOnly -and $RustOnly) {
    throw 'Choose either -FrontendOnly or -RustOnly, not both.'
}

$repoRoot = (Resolve-Path "$PSScriptRoot\..").Path
Set-Location $repoRoot

function Invoke-Gate {
    param([string]$Label, [scriptblock]$Command)
    Write-Host "`n=== $Label ===" -ForegroundColor Cyan
    & $Command
    if ($LASTEXITCODE -ne 0) { throw "$Label failed with exit code $LASTEXITCODE" }
}

if (-not $FrontendOnly) {
    $repoPattern = [regex]::Escape($repoRoot)
    $devProcesses = @(Get-CimInstance Win32_Process -ErrorAction SilentlyContinue | Where-Object {
        ($_.CommandLine -and $_.CommandLine -match $repoPattern -and
            ($_.CommandLine -match 'tauri(\.js|\.exe)?["'']?\s+dev' -or $_.CommandLine -match 'run-dev.*\.ps1')) -or
        ($_.Name -eq 'rift-tauri.exe' -and $_.ExecutablePath -match '\\cargo-targets\\debug\\rift-tauri\.exe$')
    })
    if ($devProcesses.Count -gt 0) {
        $pids = ($devProcesses.ProcessId | Sort-Object -Unique) -join ', '
        Write-Error "Repo-scoped Tauri dev is running (PID $pids). Stop it before Rust gates, or use npm run verify:frontend during live UI work."
        exit 2
    }

}

# Static analysis first: fail before paying for either test suite.
if (-not $RustOnly) {
    Invoke-Gate 'Svelte and design tokens' { npm run check }
}
if (-not $FrontendOnly) {
    Invoke-Gate 'Clippy' { cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings }
}

if (-not $RustOnly) {
    Invoke-Gate 'Vitest' { npm test }
}
if (-not $FrontendOnly) {
    Invoke-Gate 'Cargo tests' { cargo test --manifest-path src-tauri/Cargo.toml }
}

Write-Host "`nAll requested Rift verification gates passed." -ForegroundColor Green
