# type.ps1 - Set a textarea/input value + fire input events so Svelte bindings update.
# Optionally press a key after (e.g. Enter to submit).
# Usage:
#   powershell -NoProfile -File scripts/cdp/type.ps1 -Selector 'textarea' -Text 'hello'
#   powershell -NoProfile -File scripts/cdp/type.ps1 -Selector 'textarea' -Text 'hi' -PressKey Enter
param(
    [Parameter(Mandatory)] [string] $Selector,
    [Parameter(Mandatory)] [string] $Text,
    [ValidateSet('None','Enter','Tab','Escape','CtrlEnter')] [string] $PressKey = 'None',
    [int] $TimeoutMs = 10000
)
$ErrorActionPreference = 'Stop'
. "$PSScriptRoot/_cdp.ps1"

# JSON-encode the text so embedded quotes / newlines / unicode survive intact
# through the JS string literal.
$jsText = $Text | ConvertTo-Json -Compress

$keyCode = switch ($PressKey) {
    'Enter'     { 13 }
    'Tab'       { 9 }
    'Escape'    { 27 }
    'CtrlEnter' { -1 }
    default     { 0 }
}

$js = @"
(() => {
    const el = document.querySelector($($Selector | ConvertTo-Json -Compress));
    if (!el) return { error: 'selector not found' };
    el.focus();
    // Use the native value setter so React/Svelte detect the change.
    const proto = el instanceof HTMLTextAreaElement ? HTMLTextAreaElement.prototype : HTMLInputElement.prototype;
    const setter = Object.getOwnPropertyDescriptor(proto, 'value').set;
    setter.call(el, $jsText);
    el.dispatchEvent(new Event('input', { bubbles: true }));
    el.dispatchEvent(new Event('change', { bubbles: true }));
    const key = $keyCode;
    if (key === 13) {
        el.dispatchEvent(new KeyboardEvent('keydown', { key: 'Enter', code: 'Enter', keyCode: 13, which: 13, bubbles: true }));
        el.dispatchEvent(new KeyboardEvent('keyup',   { key: 'Enter', code: 'Enter', keyCode: 13, which: 13, bubbles: true }));
    } else if (key === -1) {
        el.dispatchEvent(new KeyboardEvent('keydown', { key: 'Enter', code: 'Enter', keyCode: 13, which: 13, ctrlKey: true, bubbles: true }));
        el.dispatchEvent(new KeyboardEvent('keyup',   { key: 'Enter', code: 'Enter', keyCode: 13, which: 13, ctrlKey: true, bubbles: true }));
    } else if (key === 9) {
        el.dispatchEvent(new KeyboardEvent('keydown', { key: 'Tab', code: 'Tab', keyCode: 9, which: 9, bubbles: true }));
    } else if (key === 27) {
        el.dispatchEvent(new KeyboardEvent('keydown', { key: 'Escape', code: 'Escape', keyCode: 27, which: 27, bubbles: true }));
    }
    return { ok: true, len: el.value?.length ?? 0 };
})()
"@

$target = Get-RiftTarget
$resp = Invoke-CdpCommand -WsUrl $target.webSocketDebuggerUrl -Method 'Runtime.evaluate' -TimeoutMs $TimeoutMs -Params @{
    expression    = $js
    returnByValue = $true
    awaitPromise  = $true
}

if ($resp.result.exceptionDetails) {
    @{ error = $resp.result.exceptionDetails.exception.description } | ConvertTo-Json -Compress
    exit 2
}
$resp.result.result.value | ConvertTo-Json -Compress
