$ErrorActionPreference = 'Stop'
[Console]::OutputEncoding = New-Object System.Text.UTF8Encoding
Write-Host 'Atomic Age Native 0.5.0 — Horizon nightly fdc87ae'
Write-Host 'This imports an unsigned native build and creates a separate Atomic Age instance.'
Write-Host 'Your existing worlds and mods are kept in their current instance.'
$launcher = Join-Path ([Environment]::GetFolderPath('Desktop')) 'horizon-launcher.exe'
if (!(Test-Path -LiteralPath $launcher)) {
    Add-Type -AssemblyName System.Windows.Forms
    $dialog = New-Object System.Windows.Forms.OpenFileDialog
    $dialog.Title = 'Select horizon-launcher.exe'
    $dialog.Filter = 'Horizon Launcher|*.exe'
    if ($dialog.ShowDialog() -ne 'OK') { exit 1 }
    $launcher = $dialog.FileName
}
$answer = Read-Host 'Install this native build with your user permissions? Type YES to continue'
if ($answer -cne 'YES') { exit 0 }
function Invoke-LauncherJson([string[]]$Command) {
    $lines = & $launcher --json @Command
    if ($LASTEXITCODE -ne 0) { throw "Horizon Launcher failed: $($Command -join ' ')" }
    return ($lines -join "`n" | ConvertFrom-Json)
}
# Use the player's currently selected installed Nightly as the resource donor.
$import = Invoke-LauncherJson @('import-build', (Join-Path $PSScriptRoot 'horizon-client.exe'), '--trust-unsigned')
if (!$import.imported) { throw 'The launcher did not return an imported build id.' }
$name = 'Atomic Age 0.5 ' + (Get-Date -Format 'yyyyMMdd-HHmmss')
$instance = Invoke-LauncherJson @('instance', 'create', $name)
if (!$instance.id -or !$instance.home) { throw 'The launcher did not return the new instance folder.' }
$null = Invoke-LauncherJson @('instance', 'version', $instance.id, $import.imported)
$modDir = Join-Path $instance.home 'horizon-mods'
New-Item -ItemType Directory -Force -Path $modDir | Out-Null
Copy-Item -LiteralPath (Join-Path $PSScriptRoot 'atomic_age_content.hmod') -Destination (Join-Path $modDir 'atomic_age.hmod')
[IO.File]::WriteAllText((Join-Path $instance.home 'horizon-mods.txt'), "mod.atomic_age:true`n", (New-Object Text.UTF8Encoding($false)))
$null = Invoke-LauncherJson @('instance', 'select', $instance.id)
Write-Host "Installed: $name"
Write-Host "Instance: $($instance.home)"
Write-Host 'Open Horizon Launcher, press Play, create a creative world.'
Write-Host 'Inventory: Atomic Age bombs / weapons & portals.'
Read-Host 'Press Enter to close'
