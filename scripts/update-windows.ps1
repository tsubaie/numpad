param(
    [Parameter(Mandatory=$true)][string]$Version,
    [Parameter(Mandatory=$true)][string]$AppPath,
    [Parameter(Mandatory=$true)][int]$AppPid,
    [Parameter(Mandatory=$true)][string]$WorkDir
)
$ErrorActionPreference = 'Stop'
$stagePath = $null
$backupPath = $null
try {
    if (Test-Path -LiteralPath (Join-Path $WorkDir 'cancel')) { throw 'Update canceled.' }
    Set-Content -LiteralPath (Join-Path $WorkDir 'result.started') -Value 'started' -Encoding UTF8
    if ($Version -notmatch '^\d+\.\d+\.\d+$') { throw 'Invalid release version.' }
    [Net.ServicePointManager]::SecurityProtocol = [Net.SecurityProtocolType]::Tls12
    $asset = "NumPad-$Version-windows-x64.zip"
    $base = "https://github.com/tsubaie/numpad/releases/download/v$Version"
    $archive = Join-Path $WorkDir $asset
    Write-Host "Downloading NumPad $Version..."
    Invoke-WebRequest -UseBasicParsing -Uri "$base/$asset" -OutFile $archive -TimeoutSec 300
    $sumsPath = Join-Path $WorkDir "SHA256SUMS"
    Invoke-WebRequest -UseBasicParsing -Uri "$base/SHA256SUMS" -OutFile $sumsPath -TimeoutSec 60
    $sums = Get-Content -LiteralPath $sumsPath -Raw
    $matchesFound = @($sums -split "`n" | Where-Object { $_ -match ('^[0-9a-f]{64}\s+\*?' + [regex]::Escape($asset) + '\s*$') })
    if ($matchesFound.Count -ne 1) { throw 'Missing or ambiguous release checksum.' }
    $expected = ($matchesFound[0] -split '\s+')[0]
    if ((Get-FileHash -LiteralPath $archive -Algorithm SHA256).Hash.ToLowerInvariant() -ne $expected) { throw 'Checksum mismatch. Nothing was installed.' }
    Add-Type -AssemblyName System.IO.Compression.FileSystem
    $zip = [IO.Compression.ZipFile]::OpenRead($archive)
    try {
        $entry = $zip.GetEntry('NumPad.exe')
        if ($null -eq $entry) { throw 'The release archive does not contain NumPad.exe.' }
        $stagePath = "$AppPath.update-$AppPid"
        [IO.Compression.ZipFileExtensions]::ExtractToFile($entry,$stagePath,$false)
    } finally { $zip.Dispose() }
    # Only this known entry is extracted; archive paths cannot escape the destination.
    Set-Content -LiteralPath (Join-Path $WorkDir 'ready') -Value 'ready' -Encoding UTF8
    Write-Host 'Checksum verified. Waiting for NumPad to save its session and close...'
    $deadline = [DateTime]::UtcNow.AddMinutes(2)
    while (($null -ne (Get-Process -Id $AppPid -ErrorAction SilentlyContinue)) -or !(Test-Path -LiteralPath (Join-Path $WorkDir 'commit'))) {
        if (Test-Path -LiteralPath (Join-Path $WorkDir 'cancel')) { throw 'NumPad could not save its session. Update canceled.' }
        if ([DateTime]::UtcNow -gt $deadline) { throw 'NumPad did not close. The original application is unchanged.' }
        Start-Sleep -Milliseconds 200
    }
    $backupPath = "$AppPath.backup-$(Get-Date -Format yyyyMMddHHmmss)-$AppPid"
    Move-Item -LiteralPath $AppPath -Destination $backupPath
    try { Move-Item -LiteralPath $stagePath -Destination $AppPath }
    catch { Move-Item -LiteralPath $backupPath -Destination $AppPath; throw }
    Write-Host "Installed NumPad $Version. Previous binary preserved at $backupPath"
    Start-Process -FilePath $AppPath
    Set-Content -LiteralPath (Join-Path $WorkDir 'result') -Value '0' -Encoding UTF8
    Remove-Item -LiteralPath $WorkDir -Recurse -Force -ErrorAction SilentlyContinue
} catch {
    $message = $_.Exception.Message
    Set-Content -LiteralPath (Join-Path $WorkDir 'result') -Value $message -Encoding UTF8
    if ($stagePath -and (Test-Path -LiteralPath $stagePath)) { Remove-Item -LiteralPath $stagePath -Force }
    Write-Host "Update failed: $message"
    Read-Host 'Press Enter to close'
    exit 1
}
