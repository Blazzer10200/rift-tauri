# run-dev-deelevated.ps1 — the ONE reliable way to launch Rift dev with working
# CDP from ANY shell (elevated or not), with zero orphan sprawl.
#
# WHY THIS EXISTS: WebView2 Runtime 150.x added a "trusted origin check" that
# refuses to bind the DevTools remote-debugging port when the host process runs
# ELEVATED (High Integrity Level) — v150 regression, MicrosoftEdge/WebView2Feedback#5640
# (filed 2026-07-06; worked on Runtime 149). Claude Code often runs elevated, so a
# plain `npm run tauri dev` inherits High IL and CDP silently never binds. This
# script de-elevates to medium IL via a one-shot scheduled task so :9222 opens.
#
#   pwsh -NoProfile -File scripts\run-dev-deelevated.ps1              # launch (fire-and-forget)
#   pwsh -NoProfile -File scripts\run-dev-deelevated.ps1 -WaitForCdp # launch + block until :9222 is up
#
# It ALWAYS kills stale dev instances first (the sprawl fix): repeated launches
# during a debugging session used to leave orphaned windowless rift-tauri.exe
# ghosts on the desktop. Now every launch starts from a known-clean slate.

param(
  [switch]$WaitForCdp,   # block until CDP :9222 responds (up to 180s), then return
  [switch]$NoKill        # skip the kill-stale-first step (advanced; normally you want the kill)
)

$ErrorActionPreference = 'Stop'
$repo = Split-Path -Parent $PSScriptRoot
$DEV_UDD_GLOB = '*Rift?EBWebView-Dev*'   # -like wildcard; ? matches the backslash
$env:WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS = "--remote-debugging-port=9222 --remote-allow-origins=*"
$env:WEBVIEW2_USER_DATA_FOLDER = "$env:LOCALAPPDATA\Rift\EBWebView-Dev"

function Test-Elevated {
  $id = [System.Security.Principal.WindowsIdentity]::GetCurrent()
  (New-Object System.Security.Principal.WindowsPrincipal($id)).IsInRole([System.Security.Principal.WindowsBuiltInRole]::Administrator)
}

# --- Kill-stale-first: remove any prior dev instance so launches never sprawl. ---
# Scope is STRICT: only rift-tauri.exe under the dev target dir (cargo-targets /
# src-tauri\target), its EBWebView-Dev WebView2 children, and vite on :1420. NEVER
# touches the user's REAL installed Rift (that lives under %LOCALAPPDATA%, a
# different path + a different WebView2 user-data-dir) — the by-PATH filter is the
# safety guard the "never kill rift-tauri by image name" rule demands.
function Stop-StaleDev {
  $killed = 0
  Get-CimInstance Win32_Process -Filter "Name='rift-tauri.exe'" |
    Where-Object { $_.ExecutablePath -like '*cargo-targets*' -or $_.ExecutablePath -like '*src-tauri\target*' } |
    ForEach-Object { Stop-Process -Id $_.ProcessId -Force -ErrorAction SilentlyContinue; $killed++ }
  Get-CimInstance Win32_Process -Filter "Name='msedgewebview2.exe'" |
    Where-Object { $_.CommandLine -like $DEV_UDD_GLOB } |
    ForEach-Object { Stop-Process -Id $_.ProcessId -Force -ErrorAction SilentlyContinue }
  # vite dev server on :1420 (child of the killed npm tree, but reap explicitly in case it detached)
  try { Get-NetTCPConnection -LocalPort 1420 -State Listen -ErrorAction Stop |
    ForEach-Object { Stop-Process -Id $_.OwningProcess -Force -ErrorAction SilentlyContinue } } catch {}
  # Wait for the WebView2 singleton to FULLY exit — a surviving browser process for
  # this user-data-dir makes the next launch attach to it (missing the CDP flag).
  $t = 0
  do {
    Start-Sleep -Milliseconds 700
    $n = @(Get-CimInstance Win32_Process -Filter "Name='msedgewebview2.exe'" | Where-Object { $_.CommandLine -like $DEV_UDD_GLOB }).Count
    $t++
  } while ($n -gt 0 -and $t -lt 20)
  if ($killed -gt 0) { Write-Output "[dev] cleaned $killed stale dev instance(s); webview singleton drained (n=$n)" }
  else { Write-Output "[dev] no stale dev instances (clean slate)" }
}

if (-not $NoKill) { Stop-StaleDev }

# The command the dev process runs. Env vars are set INSIDE the task action so
# they reach the (medium-IL) child regardless of how the task host seeds env.
$devCmd = @"
set "WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS=--remote-debugging-port=9222 --remote-allow-origins=*"
set "WEBVIEW2_USER_DATA_FOLDER=%LOCALAPPDATA%\Rift\EBWebView-Dev"
cd /d "$repo"
call npm run tauri dev
"@
$batPath = Join-Path $env:TEMP "rift-dev-deelevated.bat"
Set-Content -LiteralPath $batPath -Value $devCmd -Encoding ASCII

if (Test-Elevated) {
  Write-Output "[dev] shell IS elevated — de-elevating to medium IL (WebView2 150.x CDP fix)"
  $taskName = "RiftDevDeElevated"
  $user = "$env:USERDOMAIN\$env:USERNAME"
  # schtasks chatters on stderr (benign "not found" / "/ST earlier than now"
  # warnings); PS5.1 under EAP=Stop + redirect promotes that to a terminating
  # NativeCommandError. Relax EAP for the block; fail loud via exit codes.
  $eap = $ErrorActionPreference; $ErrorActionPreference = 'Continue'
  schtasks /Delete /TN $taskName /F *>$null
  # /RL LIMITED = run as the interactive user at their DEFAULT (medium) level.
  schtasks /Create /TN $taskName /TR "cmd.exe /c `"$batPath`"" /SC ONCE /ST 00:00 /RL LIMITED /F /RU $user *>$null
  if ($LASTEXITCODE -ne 0) { throw "schtasks /Create failed (exit $LASTEXITCODE)" }
  schtasks /Run /TN $taskName *>$null
  if ($LASTEXITCODE -ne 0) { throw "schtasks /Run failed (exit $LASTEXITCODE)" }
  Start-Sleep -Seconds 3
  schtasks /Delete /TN $taskName /F *>$null
  $ErrorActionPreference = $eap
  Write-Output "[dev] launched at medium IL via scheduled task; task entry cleaned up."
} else {
  Write-Output "[dev] shell is NOT elevated — launching directly at medium IL"
  # Detached so this script can return / proceed to the wait loop.
  Start-Process -FilePath "cmd.exe" -ArgumentList "/c", "`"$batPath`"" -WindowStyle Minimized
  Write-Output "[dev] launched (minimized console)."
}

if ($WaitForCdp) {
  Write-Output "[dev] waiting for CDP :9222 to bind (WebView2 must finish first paint)..."
  $bound = 0
  for ($i = 0; $i -lt 60; $i++) {
    try {
      $r = Invoke-WebRequest -Uri "http://127.0.0.1:9222/json/version" -UseBasicParsing -TimeoutSec 2 -ErrorAction Stop
      if ($r.StatusCode -eq 200) { $bound++; if ($bound -ge 2) { break } }
    } catch { $bound = 0 }
    Start-Sleep -Seconds 3
  }
  if ($bound -ge 2) { Write-Output "[dev] OK - CDP :9222 is UP (~$($i*3)s). Start the wrapper (npm run cdp:serve) if not already running." }
  else { Write-Output "[dev] FAIL - CDP :9222 did NOT bind in ~180s. Run: bash scripts/cdp/c.sh doctor" }
}
