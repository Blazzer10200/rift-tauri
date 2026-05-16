# wait.ps1 - Poll a JS expression until it returns truthy (or timeout).
# Returns the final value as JSON. Critical for "wait for stream done", "wait for modal".
# Usage:
#   powershell -NoProfile -File scripts/cdp/wait.ps1 -Js "!document.querySelector('[data-streaming]')" -TimeoutMs 60000
#   powershell -NoProfile -File scripts/cdp/wait.ps1 -Js "document.querySelectorAll('.message').length >= 2"
param(
    [Parameter(Mandatory)] [string] $Js,
    [int] $TimeoutMs = 60000,
    [int] $IntervalMs = 250,
    [switch] $Quiet
)
$ErrorActionPreference = 'Stop'
. "$PSScriptRoot/_cdp.ps1"

$target = Get-RiftTarget
$deadline = [DateTime]::UtcNow.AddMilliseconds($TimeoutMs)
$polls = 0

while ([DateTime]::UtcNow -lt $deadline) {
    $polls++
    $resp = Invoke-CdpCommand -WsUrl $target.webSocketDebuggerUrl -Method 'Runtime.evaluate' -TimeoutMs 5000 -Params @{
        expression    = $Js
        returnByValue = $true
        awaitPromise  = $true
    }
    if ($resp.result.exceptionDetails) {
        @{ error = $resp.result.exceptionDetails.exception.description; polls = $polls } | ConvertTo-Json -Compress
        exit 2
    }
    $val = $resp.result.result.value
    if ($val) {
        @{ ok = $true; value = $val; polls = $polls; elapsedMs = [int]((($deadline.AddMilliseconds(-$TimeoutMs) - [DateTime]::UtcNow).TotalMilliseconds) * -1) } | ConvertTo-Json -Compress -Depth 10
        exit 0
    }
    Start-Sleep -Milliseconds $IntervalMs
}

@{ ok = $false; reason = 'timeout'; polls = $polls; timeoutMs = $TimeoutMs } | ConvertTo-Json -Compress
exit 3
