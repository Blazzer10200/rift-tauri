# screenshot.ps1 - Capture a PNG of the Rift webview. Prints saved path on stdout.
# Usage: powershell -NoProfile -File scripts/cdp/screenshot.ps1
#        powershell -NoProfile -File scripts/cdp/screenshot.ps1 -Out path.png -Quality 60
#        powershell -NoProfile -File scripts/cdp/screenshot.ps1 -Format jpeg -Quality 50
#
# Default goes to scripts/cdp/.tmp/snap-<timestamp>.png. JPEG at quality 50-70
# is the cost-saving sweet spot for Opus 4.7 token math (images tripled in
# cost vs 4.6). Use only when DOM/state inspection won't answer the question.
param(
    [string] $Out,
    [ValidateSet('png','jpeg','webp')] [string] $Format = 'png',
    [int] $Quality = 70,
    [string] $Clip,   # "x,y,w,h"
    [int] $TimeoutMs = 15000
)
$ErrorActionPreference = 'Stop'
. "$PSScriptRoot/_cdp.ps1"

if (-not $Out) {
    $tmpDir = Join-Path $PSScriptRoot '.tmp'
    if (-not (Test-Path $tmpDir)) { New-Item -ItemType Directory -Path $tmpDir | Out-Null }
    $ts = Get-Date -Format 'yyyyMMdd-HHmmss'
    $Out = Join-Path $tmpDir "snap-$ts.$Format"
}

$params = @{ format = $Format }
if ($Format -ne 'png') { $params.quality = $Quality }
if ($Clip) {
    $parts = $Clip -split ','
    if ($parts.Count -ne 4) { throw "Clip must be 'x,y,w,h'" }
    $params.clip = @{
        x = [double]$parts[0]; y = [double]$parts[1]
        width = [double]$parts[2]; height = [double]$parts[3]
        scale = 1
    }
}

$target = Get-RiftTarget
$resp = Invoke-CdpCommand -WsUrl $target.webSocketDebuggerUrl -Method 'Page.captureScreenshot' -Params $params -TimeoutMs $TimeoutMs

if (-not $resp.result.data) {
    throw "CDP returned no data. Full response: $($resp | ConvertTo-Json -Compress -Depth 10)"
}

$bytes = [Convert]::FromBase64String($resp.result.data)
[System.IO.File]::WriteAllBytes($Out, $bytes)
Write-Output $Out
