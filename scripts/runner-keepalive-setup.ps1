# Run ON the proxmox-win CI runner VM to (re)install the RunnerKeepAlive
# startup task — load-bearing for tag-driven releases (cont.90): without it a
# cold boot can leave the actions-runner service stopped and CI queues forever.
$ErrorActionPreference='Continue'
$svc='actions.runner.Blazzer10200-rift-tauri.proxmox-win'

# keepalive script: at startup, retry-start the runner service until it stays
# Running (covers the cold-boot race where network isn't up yet and the runner
# exits cleanly before SCM/delayed-start would recover it).
$ka = @'
$svc='actions.runner.Blazzer10200-rift-tauri.proxmox-win'
for($i=0; $i -lt 30; $i++){
  try { if((Get-Service $svc -EA Stop).Status -ne 'Running'){ Start-Service $svc -EA Stop } } catch {}
  Start-Sleep -Seconds 10
  try { if((Get-Service $svc -EA Stop).Status -eq 'Running'){ break } } catch {}
}
'@
Set-Content 'C:\runner-keepalive.ps1' -Value $ka -Encoding UTF8

# register the startup task (runs as SYSTEM at boot)
schtasks /create /tn 'RunnerKeepAlive' /tr 'powershell.exe -NoProfile -ExecutionPolicy Bypass -WindowStyle Hidden -File C:\runner-keepalive.ps1' /sc onstart /ru SYSTEM /rl HIGHEST /f | Out-Null

# also let SCM restart the service on a CLEAN exit (failureflag 1), not just crashes
sc.exe failureflag $svc 1 | Out-Null

'task exists: ' + ((schtasks /query /tn 'RunnerKeepAlive' /fo csv -EA SilentlyContinue) -ne $null)
'keepalive script: ' + (Test-Path 'C:\runner-keepalive.ps1')
'service start type: ' + (((sc.exe qc $svc) -join ' ') -replace '.*START_TYPE\s*:\s*(\S+\s+\S+(\s+\(\S+\))?).*','$1')
