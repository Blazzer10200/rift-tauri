# eval.ps1 - Run JS in the Rift page, print the result as JSON.
# Usage: powershell -NoProfile -File scripts/cdp/eval.ps1 -Js "document.title"
#        powershell -NoProfile -File scripts/cdp/eval.ps1 -JsFile path/to/snippet.js
# Returns: { value | error } as JSON on stdout.
param(
    [string] $Js,
    [string] $JsFile,
    [int] $TimeoutMs = 30000
)
$ErrorActionPreference = 'Stop'
. "$PSScriptRoot/_cdp.ps1"

if ($JsFile) {
    if (-not (Test-Path $JsFile)) { throw "JsFile not found: $JsFile" }
    $Js = [System.IO.File]::ReadAllText($JsFile, [System.Text.UTF8Encoding]::new($false))
}
if (-not $Js) { throw "Provide -Js '<expr>' or -JsFile <path>" }

$target = Get-RiftTarget

# awaitPromise=true lets us return values from async fns.
# returnByValue=true serializes the result instead of returning an objectId.
$resp = Invoke-CdpCommand -WsUrl $target.webSocketDebuggerUrl -Method 'Runtime.evaluate' -TimeoutMs $TimeoutMs -Params @{
    expression    = $Js
    returnByValue = $true
    awaitPromise  = $true
    timeout       = $TimeoutMs
}

if ($resp.result.exceptionDetails) {
    $err = $resp.result.exceptionDetails
    $msg = $err.exception.description
    if (-not $msg) { $msg = $err.text }
    @{ error = $msg } | ConvertTo-Json -Depth 10 -Compress
    exit 2
}

$value = $resp.result.result.value
@{ value = $value } | ConvertTo-Json -Depth 20 -Compress
