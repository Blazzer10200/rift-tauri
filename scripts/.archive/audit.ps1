# audit.ps1 — local CI-shaped check. No code mutation.
#
# Runs the same gate a release-eve CI job would: dep advisories, unused deps,
# npm prod-side advisories, svelte-check, cargo check. Exits non-zero on any
# failure so the script can be wrapped by a pre-push hook or a GH Actions job
# once #14 lands.
#
# Usage: pwsh scripts/audit.ps1
# From git-bash: powershell -NoProfile -File scripts/audit.ps1

$ErrorActionPreference = 'Stop'
$root = Split-Path -Parent $PSScriptRoot
Push-Location $root

$failed = @()

function Step($name, [scriptblock]$block) {
    Write-Host ""
    Write-Host "=== $name ===" -ForegroundColor Cyan
    $exit = 0
    try {
        & $block
        $exit = $LASTEXITCODE
    } catch {
        Write-Host $_ -ForegroundColor Red
        $exit = 1
    }
    if ($exit -ne 0) {
        Write-Host "FAIL ($name) exit=$exit" -ForegroundColor Red
        $script:failed += $name
    } else {
        Write-Host "OK ($name)" -ForegroundColor Green
    }
}

Step 'cargo audit' {
    cargo audit --file src-tauri/Cargo.lock --ignore RUSTSEC-2023-0071
}

Step 'cargo machete' {
    cargo machete src-tauri
}

Step 'npm audit (prod only)' {
    npm audit --omit=dev
}

Step 'svelte-check' {
    npm run check
}

Step 'cargo check' {
    cargo check --manifest-path src-tauri/Cargo.toml
}

Pop-Location

Write-Host ""
if ($failed.Count -gt 0) {
    Write-Host "AUDIT FAILED: $($failed -join ', ')" -ForegroundColor Red
    exit 1
} else {
    Write-Host "AUDIT GREEN" -ForegroundColor Green
    exit 0
}
