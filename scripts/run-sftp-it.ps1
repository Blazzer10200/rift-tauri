# run-sftp-it.ps1 - run the live-SFTP #[ignore] integration tests against the
# Proxmox LXC 121 test target (see scripts/sftp-test-target.sh + docs/design/
# proxmox-sftp-test-target.md). One command instead of six lines of env setup.
#
#   powershell -NoProfile -File scripts/run-sftp-it.ps1
#   powershell -NoProfile -File scripts/run-sftp-it.ps1 -Host_ 192.168.1.16  # skip IP resolve
#   powershell -NoProfile -File scripts/run-sftp-it.ps1 -Filter sftp_it_batch
#
# Tests are #[ignore]d so a plain `cargo test` never runs them; this opts in.
param(
    [string]$Host_   = "",                                   # explicit IP; blank => resolve via bash helper
    [string]$User    = "rift",
    [int]   $Port    = 22,
    [string]$Key     = "C:/AI Workflow/.secrets/rift-sftp-test",
    [string]$Filter  = "sftp_it"
)

$ErrorActionPreference = "Stop"
$root = Split-Path -Parent $PSScriptRoot   # repo root (scripts/ is one level down)
Set-Location $root

if ([string]::IsNullOrWhiteSpace($Host_)) {
    Write-Host "resolving test-target IP via scripts/sftp-test-target.sh ..."
    $Host_ = (bash scripts/sftp-test-target.sh ip).Trim()
    if ([string]::IsNullOrWhiteSpace($Host_)) {
        Write-Error "could not resolve target IP (container down? bash on PATH?). Pass -Host_ <ip> to override."
        exit 1
    }
}

if (-not (Test-Path $Key)) {
    Write-Error "SSH key not found: $Key  (pass -Key <path> in Windows C:/ form - Rust std::fs can't read /c/ paths)"
    exit 1
}

Write-Host "target: $User@$Host_`:$Port  key: $Key  filter: $Filter"
$env:RIFT_TEST_SFTP_HOST = $Host_
$env:RIFT_TEST_SFTP_PORT = "$Port"
$env:RIFT_TEST_SFTP_USER = $User
$env:RIFT_TEST_SFTP_KEY  = $Key

cargo test --manifest-path src-tauri/Cargo.toml $Filter -- --ignored --nocapture
exit $LASTEXITCODE
