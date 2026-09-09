$ErrorActionPreference = 'Stop'
$taskRoot = Split-Path -Parent $PSScriptRoot
Push-Location -LiteralPath $taskRoot
try {
    cargo build --release --locked
    if ($LASTEXITCODE -ne 0) { throw 'Build failed' }
    Copy-Item -LiteralPath (Join-Path $taskRoot 'target\release\numpad.exe') -Destination (Join-Path $taskRoot 'NumPad.exe')
    python scripts/package.py --platform windows --binary NumPad.exe --output release
    if ($LASTEXITCODE -ne 0) { throw 'Packaging failed (Python 3.11+ is required)' }
} finally { Pop-Location }
