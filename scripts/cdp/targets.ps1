# targets.ps1 - List CDP targets. Sanity-check that WebView2 is exposing CDP.
# Usage: powershell -NoProfile -File scripts/cdp/targets.ps1
param()
$ErrorActionPreference = 'Stop'
. "$PSScriptRoot/_cdp.ps1"

try {
    $list = Invoke-RestMethod -Uri "$script:CdpHost/json" -TimeoutSec 5
} catch {
    Write-Error "CDP unreachable at $script:CdpHost. Did you start dev via scripts/run-dev.bat?"
    exit 1
}

$list | ForEach-Object {
    [PSCustomObject]@{
        type  = $_.type
        title = $_.title
        url   = $_.url
        id    = $_.id
    }
} | Format-Table -AutoSize
