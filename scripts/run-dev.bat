@echo off
rem Rift dev launcher - keeps console open for vite + tauri logs.
rem Also enables WebView2 CDP on localhost:9222 so Claude can introspect
rem the running UI via scripts/cdp/*.ps1. Cost: nothing in dev, port only
rem listens on localhost. See scripts/cdp/README.md.
title Rift Dev
cd /d "%~dp0.."
rem CDP port. NOTE: WebView2 Runtime 150.x refuses to bind this port when the
rem HOST PROCESS RUNS ELEVATED (admin / High Integrity Level) — a v150 security
rem regression (WebView2Feedback#5640). A normal double-click of this .bat runs
rem at medium IL and works fine. But if you launch it from an ELEVATED shell
rem (e.g. an admin terminal, or Claude Code running elevated), the CDP port
rem silently never binds — use scripts\run-dev-deelevated.ps1 instead, which
rem de-elevates to medium IL. (--remote-allow-origins=* is harmless belt-and-
rem suspenders, NOT the fix — elevation is.) See scripts/cdp/README.md.
set "WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS=--remote-debugging-port=9222 --remote-allow-origins=*"
rem Dedicated WebView2 user-data folder so dev can run alongside the installed
rem Rift build without 0x8007139F lock conflicts (both share bundle identifier).
set "WEBVIEW2_USER_DATA_FOLDER=%LOCALAPPDATA%\Rift\EBWebView-Dev"
echo Starting Rift dev (vite + tauri)...
echo Repo: %CD%
echo CDP:  http://localhost:9222/json (WebView2 DevTools Protocol)
echo.
call npm run tauri dev
echo.
echo Dev server exited with code %ERRORLEVEL%.
pause
