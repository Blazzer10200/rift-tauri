# Run Rift's Rust tests and self-heal one known persistent-runner cache failure.
#
# The self-hosted Windows runner keeps CARGO_TARGET_DIR between jobs. A runner
# interruption can leave webview2-com's CodeView data corrupt: link.exe then
# reports LNK1103 before any test binary starts. Retry only that exact linker
# failure after cleaning the affected package; all source/test failures remain
# immediately red.

$ErrorActionPreference = 'Continue'
$env:CARGO_PROFILE_TEST_DEBUG = '0'
$manifest = 'src-tauri/Cargo.toml'

$firstOutput = @(& cargo test --manifest-path $manifest 2>&1)
$firstExit = $LASTEXITCODE
$firstOutput | ForEach-Object { Write-Host "$_" }

if ($firstExit -eq 0) {
    exit 0
}

$failureText = $firstOutput -join "`n"
if ($failureText -notmatch 'LNK1103:\s+debugging information corrupt' -or
    $failureText -notmatch 'libwebview2_com-') {
    exit $firstExit
}

Write-Warning 'Corrupt webview2-com debug cache detected; cleaning that package and retrying once.'
& cargo clean --manifest-path $manifest -p webview2-com
$cleanExit = $LASTEXITCODE
if ($cleanExit -ne 0) {
    exit $cleanExit
}

& cargo test --manifest-path $manifest
exit $LASTEXITCODE
