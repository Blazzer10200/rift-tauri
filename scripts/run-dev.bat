@echo off
rem Rift dev launcher - keeps console open for vite + tauri logs.
rem Also enables WebView2 CDP on localhost:9222 so Claude can introspect
rem the running UI via scripts/cdp/*.ps1. Cost: nothing in dev, port only
rem listens on localhost. See scripts/cdp/README.md.
title Rift Dev
cd /d "%~dp0.."
set "WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS=--remote-debugging-port=9222"
echo Starting Rift dev (vite + tauri)...
echo Repo: %CD%
echo CDP:  http://localhost:9222/json (WebView2 DevTools Protocol)
echo.
call npm run tauri dev
echo.
echo Dev server exited with code %ERRORLEVEL%.
pause
