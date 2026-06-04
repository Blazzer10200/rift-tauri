@echo off
REM Live watcher for the rift-edit-swarm. Run from a command prompt at the repo root:
REM    scripts\edit-watch.cmd            (newest run)
REM    scripts\edit-watch.cmd wf_xxxxx   (a specific run id)
powershell -NoProfile -ExecutionPolicy Bypass -File "%~dp0edit-watch.ps1" %*
