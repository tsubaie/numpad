# Portable helper tests. Run under PowerShell 7; all downloads and process
# launches are mocked, while checksum, ZIP, replacement, and rollback use real files.
param([string]$Scenario = 'success')
$ErrorActionPreference = 'Stop'
$root = Join-Path ([IO.Path]::GetTempPath()) ([guid]::NewGuid().ToString())
$work = Join-Path $root 'work'
$global:NumPadTestWork = $work
$fixture = Join-Path $root 'fixture'
New-Item -ItemType Directory -Path $work,$fixture | Out-Null
$app = Join-Path $root 'NumPad.exe'
Set-Content -LiteralPath $app -Value 'old binary'
$entryName = if ($Scenario -eq 'missing-executable') { 'unexpected.txt' } else { 'NumPad.exe' }
Set-Content -LiteralPath (Join-Path $fixture $entryName) -Value 'new binary'
$global:NumPadTestArchive = Join-Path $root 'fixture.zip'
Compress-Archive -Path (Join-Path $fixture '*') -DestinationPath $global:NumPadTestArchive
$global:Restarted = $false
$global:Requests = 0
$global:FailReplacement = $Scenario -eq 'replacement-fails'
function Invoke-WebRequest {
    param($Uri,$OutFile,$TimeoutSec,[switch]$UseBasicParsing)
    $global:Requests++
    if ($Uri -notmatch '^https://github.com/tsubaie/numpad/releases/download/v1\.3\.2/') { throw 'Unexpected download URL' }
    if ($Uri.EndsWith('/SHA256SUMS')) {
        $hash = (Get-FileHash -LiteralPath $global:NumPadTestArchive -Algorithm SHA256).Hash.ToLowerInvariant()
        if ($Scenario -eq 'bad-checksum') { $hash = '0' * 64 }
        $line = "$hash  NumPad-1.3.2-windows-x64.zip"
        if ($Scenario -eq 'duplicate-checksum') { $line = "$line`n$line" }
        Set-Content -LiteralPath $OutFile -Value $line
    } else { Copy-Item -LiteralPath $global:NumPadTestArchive -Destination $OutFile }
}
function Get-Process {
    param($Id,$ErrorAction)
    $marker = if ($Scenario -eq 'save-canceled') { 'cancel' } else { 'commit' }
    Set-Content -LiteralPath (Join-Path $global:NumPadTestWork $marker) -Value 'test'
    return $null
}
function Start-Process { param($FilePath) if ((Get-Content -LiteralPath $FilePath -Raw).Trim() -ne 'new binary') { throw 'Restarted wrong binary' }; $global:Restarted = $true }
function Read-Host { param($Prompt) return '' }
function Move-Item {
    param($LiteralPath,$Destination)
    if ($global:FailReplacement -and $LiteralPath.EndsWith('.update-424242')) {
        $global:FailReplacement = $false
        throw 'Simulated replacement failure'
    }
    Microsoft.PowerShell.Management\Move-Item -LiteralPath $LiteralPath -Destination $Destination
}
$version = if ($Scenario -eq 'invalid-version') { '1.3.2;whoami' } else { '1.3.2' }
& (Join-Path $PSScriptRoot '../scripts/update-windows.ps1') -Version $version -AppPath $app -AppPid 424242 -WorkDir $work
if ($Scenario -eq 'success') {
    if (!$global:Restarted -or (Get-Content -LiteralPath $app -Raw).Trim() -ne 'new binary') { throw 'Update failed' }
    $backups = @(Get-ChildItem -LiteralPath $root -Filter 'NumPad.exe.backup-*')
    if ($backups.Count -ne 1 -or (Get-Content -LiteralPath $backups[0].FullName -Raw).Trim() -ne 'old binary') { throw 'Backup missing' }
} else {
    if ($global:Restarted -or (Get-Content -LiteralPath $app -Raw).Trim() -ne 'old binary') { throw 'Failure changed the installed binary' }
    if (!(Test-Path -LiteralPath (Join-Path $work 'result'))) { throw 'Failure feedback missing' }
    if ($Scenario -eq 'invalid-version' -and $global:Requests -ne 0) { throw 'Invalid version requested a download' }
}
Remove-Item -LiteralPath $root -Recurse -Force
Write-Host "PASS $Scenario"
exit 0
