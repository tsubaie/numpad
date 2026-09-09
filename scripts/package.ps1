$ErrorActionPreference = 'Stop'
$taskRoot = Split-Path -Parent $PSScriptRoot
Push-Location -LiteralPath $taskRoot
try {
    cargo build --release --locked
    if ($LASTEXITCODE -ne 0) { throw 'Build failed' }
    $taskRelease = Join-Path $taskRoot 'release'
    New-Item -ItemType Directory -Path $taskRelease -Force | Out-Null
    Copy-Item -LiteralPath (Join-Path $taskRoot 'target\release\numpad.exe') -Destination (Join-Path $taskRoot 'NumPad.exe')
    Compress-Archive -LiteralPath (Join-Path $taskRoot 'NumPad.exe'), (Join-Path $taskRoot 'README.md'), (Join-Path $taskRoot 'LICENSE'), (Join-Path $taskRoot 'docs') -DestinationPath (Join-Path $taskRelease 'NumPad-1.3.0-windows-x64.zip') -Force
} finally { Pop-Location }
