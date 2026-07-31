$ErrorActionPreference = 'Stop'

function Invoke-Checked([string]$Command, [string[]]$Arguments) {
  & $Command @Arguments
  if ($LASTEXITCODE -ne 0) {
    throw "$Command failed with exit code $LASTEXITCODE"
  }
}

Invoke-Checked 'npm' @('run', 'check')
Invoke-Checked 'npm' @('test', '--', '--run', 'src/lib/state/assistant.playback.test.ts', 'src/lib/components/assistant/composer/modelMatrix.test.ts')

Push-Location (Join-Path $PSScriptRoot '..\src-tauri')
try {
  Invoke-Checked 'cargo' @('test', 'openai', '--lib')
} finally {
  Pop-Location
}
