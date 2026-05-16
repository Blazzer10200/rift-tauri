# _cdp.ps1 - Shared helper: connect to WebView2 CDP, send command, get result.
# Dot-source from sibling scripts. ASCII only (PS5.1 BOM trap).

$ErrorActionPreference = 'Stop'
$script:CdpPort = if ($env:RIFT_CDP_PORT) { $env:RIFT_CDP_PORT } else { '9222' }
$script:CdpHost = "http://localhost:$script:CdpPort"

function Get-RiftTarget {
    # Find the Rift page target. WebView2 may also list service workers.
    try {
        $list = Invoke-RestMethod -Uri "$script:CdpHost/json" -TimeoutSec 5
    } catch {
        throw "CDP unreachable at $script:CdpHost. Is run-dev.bat running and is WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS set?"
    }
    $page = $list | Where-Object { $_.type -eq 'page' } | Select-Object -First 1
    if (-not $page) { throw "No 'page' target in CDP list. Targets: $($list | ConvertTo-Json -Compress)" }
    return $page
}

function Invoke-CdpCommand {
    param(
        [Parameter(Mandatory)] [string] $WsUrl,
        [Parameter(Mandatory)] [string] $Method,
        [hashtable] $Params = @{},
        [int] $TimeoutMs = 30000
    )
    Add-Type -AssemblyName System.Net.Http
    $ws = New-Object System.Net.WebSockets.ClientWebSocket
    $cts = New-Object System.Threading.CancellationTokenSource
    $cts.CancelAfter($TimeoutMs)
    try {
        $ws.ConnectAsync([Uri]$WsUrl, $cts.Token).GetAwaiter().GetResult()

        $payload = @{
            id     = 1
            method = $Method
            params = $Params
        } | ConvertTo-Json -Compress -Depth 10

        $bytes = [System.Text.Encoding]::UTF8.GetBytes($payload)
        $seg = New-Object System.ArraySegment[byte] -ArgumentList @(,$bytes)
        $ws.SendAsync($seg, [System.Net.WebSockets.WebSocketMessageType]::Text, $true, $cts.Token).GetAwaiter().GetResult()

        # Drain frames until we see {"id":1,...} (skip async events).
        $buf = New-Object byte[] 65536
        $sb = New-Object System.Text.StringBuilder
        while ($true) {
            $sb.Clear() | Out-Null
            do {
                $segIn = New-Object System.ArraySegment[byte] -ArgumentList @(,$buf)
                $r = $ws.ReceiveAsync($segIn, $cts.Token).GetAwaiter().GetResult()
                $sb.Append([System.Text.Encoding]::UTF8.GetString($buf, 0, $r.Count)) | Out-Null
            } while (-not $r.EndOfMessage)
            $frame = $sb.ToString()
            $obj = $frame | ConvertFrom-Json
            if ($obj.id -eq 1) { return $obj }
        }
    } finally {
        try { $ws.CloseAsync([System.Net.WebSockets.WebSocketCloseStatus]::NormalClosure, '', $cts.Token).GetAwaiter().GetResult() } catch {}
        $ws.Dispose()
        $cts.Dispose()
    }
}
