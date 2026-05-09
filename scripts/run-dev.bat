@echo off
rem Rift dev launcher - keeps console open for vite + tauri logs.
title Rift Dev
cd /d "%~dp0.."
echo Starting Rift dev (vite + tauri)...
echo Repo: %CD%
echo.
call npm run tauri dev
echo.
echo Dev server exited with code %ERRORLEVEL%.
pause
