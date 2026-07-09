# run-dev2-deelevated.ps1 — SECOND parallel dev window for side-by-side work.
# Mirrors run-dev-deelevated.ps1's de-elevation (WebView2 150.x elevated-CDP fix,
# see that file) but does NOT run `tauri dev` — it launches the ALREADY-BUILT dev
# exe directly. Why: any `tauri dev --config` overlay changes the baked config →
# cargo relink → "Access is denied" replacing rift-tauri.exe while instance 1
# runs it. The dev exe has devUrl http://localhost:1420 compiled in, so this
# window feeds off instance 1's vite — no second vite, no cargo, instant launch.
#
#   instance 1 (primary):  vite :1420 · CDP :9222 · wrapper :9223 · EBWebView-Dev
#   instance 2 (this):     same vite  · CDP :9224 · wrapper :9225 · EBWebView-Dev2
#
# Wrapper for this instance (RIFT_CDP_HOST pin matters — WebView2 CDP is IPv4-only):
#   RIFT_CDP_HOST=127.0.0.1 RIFT_CDP_PORT=9224 RIFT_CDP_API_PORT=9225 node scripts/cdp/serve.cjs
# c.sh against this instance:
#   RIFT_CDP_API=http://127.0.0.1:9225 bash scripts/cdp/c.sh <cmd>
#
# PREREQS: instance 1's dev must be RUNNING (this window dies with its vite).
# SHARED-TREE CAVEATS: src/ edits HMR into BOTH windows; Rust rebuilds are
# off-limits while either instance runs (shared exe).
# NOTE: instance 1's cleanup (`c.sh reap`, run-dev-deelevated's kill-stale) globs
# `*EBWebView-Dev*` which MATCHES Dev2 — those will reap this window as "stale".

param(
  [switch]$WaitForCdp,   # block until CDP :9224 responds (up to 90s), then return
  [switch]$NoKill        # skip the kill-stale-first step
)

$ErrorActionPreference = 'Stop'
$repo = Split-Path -Parent $PSScriptRoot
$DEV2_UDD_GLOB = '*Rift?EBWebView-Dev2*'   # -like wildcard; ? matches the backslash
$CDP_PORT = 9224
$EXE = 'C:\cargo-targets\debug\rift-tauri.exe'

if (-not (Test-Path $EXE)) { throw "[dev2] dev exe not found at $EXE — run instance 1's dev (which builds it) first." }
try {
  $vite = Invoke-WebRequest -Uri "http://localhost:1420/" -UseBasicParsing -TimeoutSec 3 -ErrorAction Stop
} catch { throw "[dev2] vite :1420 not answering — instance 1's dev must be running (this window loads from it)." }

function Test-Elevated {
  $id = [System.Security.Principal.WindowsIdentity]::GetCurrent()
  (New-Object System.Security.Principal.WindowsPrincipal($id)).IsInRole([System.Security.Principal.WindowsBuiltInRole]::Administrator)
}

# --- Kill-stale-first, scoped to INSTANCE 2 ONLY. ---
# Instance 1 (:1420/:9222/EBWebView-Dev) may be another session's live workbench —
# never touch it. Scope: Dev2 webviews + their rift-tauri.exe parent (dev-target
# path double-checked). PID-only kills.
function Stop-StaleDev2 {
  $killed = 0
  $webviews = @(Get-CimInstance Win32_Process -Filter "Name='msedgewebview2.exe'" |
    Where-Object { $_.CommandLine -like $DEV2_UDD_GLOB })
  $hostPids = $webviews | ForEach-Object { $_.ParentProcessId } | Sort-Object -Unique
  foreach ($hp in $hostPids) {
    $p = Get-CimInstance Win32_Process -Filter "ProcessId=$hp" -ErrorAction SilentlyContinue
    if ($p -and $p.Name -eq 'rift-tauri.exe' -and
        ($p.ExecutablePath -like '*cargo-targets*' -or $p.ExecutablePath -like '*src-tauri\target*')) {
      Stop-Process -Id $hp -Force -ErrorAction SilentlyContinue; $killed++
    }
  }
  $webviews | ForEach-Object { Stop-Process -Id $_.ProcessId -Force -ErrorAction SilentlyContinue }
  # Wait for the Dev2 WebView2 singleton to FULLY exit — a survivor makes the next
  # launch attach to it (missing the CDP flag).
  $t = 0
  do {
    Start-Sleep -Milliseconds 700
    $n = @(Get-CimInstance Win32_Process -Filter "Name='msedgewebview2.exe'" | Where-Object { $_.CommandLine -like $DEV2_UDD_GLOB }).Count
    $t++
  } while ($n -gt 0 -and $t -lt 20)
  if ($killed -gt 0) { Write-Output "[dev2] cleaned $killed stale instance-2 host(s); Dev2 webview drained (n=$n)" }
  else { Write-Output "[dev2] no stale instance-2 processes (clean slate)" }
}

if (-not $NoKill) { Stop-StaleDev2 }

# Env vars set INSIDE the task action so they reach the (medium-IL) child.
$devCmd = @"
set "WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS=--remote-debugging-port=$CDP_PORT --remote-allow-origins=*"
set "WEBVIEW2_USER_DATA_FOLDER=%LOCALAPPDATA%\Rift\EBWebView-Dev2"
cd /d "$repo"
start "" "$EXE"
"@
$batPath = Join-Path $env:TEMP "rift-dev2-deelevated.bat"
Set-Content -LiteralPath $batPath -Value $devCmd -Encoding ASCII

if (Test-Elevated) {
  Write-Output "[dev2] shell IS elevated — de-elevating to medium IL (WebView2 150.x CDP fix)"
  $taskName = "RiftDev2DeElevated"
  $user = "$env:USERDOMAIN\$env:USERNAME"
  schtasks /Delete /TN $taskName /F *>$null
  schtasks /Create /TN $taskName /TR "cmd.exe /c `"$batPath`"" /SC ONCE /ST 00:00 /RL LIMITED /F /RU $user *>$null
  schtasks /Run /TN $taskName *>$null
  Start-Sleep -Seconds 3
  schtasks /Delete /TN $taskName /F *>$null
  Write-Output "[dev2] launched at medium IL via scheduled task; task entry cleaned up."
} else {
  Write-Output "[dev2] shell is NOT elevated — launching directly at medium IL"
  Start-Process -FilePath "cmd.exe" -ArgumentList "/c", "`"$batPath`"" -WindowStyle Minimized
  Write-Output "[dev2] launched (direct exe)."
}

if ($WaitForCdp) {
  Write-Output "[dev2] waiting for CDP :$CDP_PORT to bind (WebView2 must finish first paint)..."
  $bound = 0
  for ($i = 0; $i -lt 30; $i++) {
    try {
      $r = Invoke-WebRequest -Uri "http://127.0.0.1:$CDP_PORT/json/version" -UseBasicParsing -TimeoutSec 2 -ErrorAction Stop
      if ($r.StatusCode -eq 200) { $bound++; if ($bound -ge 2) { break } }
    } catch { $bound = 0 }
    Start-Sleep -Seconds 3
  }
  if ($bound -ge 2) { Write-Output "[dev2] OK: CDP :$CDP_PORT is UP (~$($i*3)s)." }
  else { Write-Output "[dev2] FAIL: CDP :$CDP_PORT did NOT bind in ~90s. Is the Dev2 window open? Check for a crash dialog." }
}
