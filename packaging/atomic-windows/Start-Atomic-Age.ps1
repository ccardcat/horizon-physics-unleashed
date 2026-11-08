$ErrorActionPreference = 'Stop'
$root = $PSScriptRoot
$version = (Get-Content -LiteralPath (Join-Path $root 'research-lock.json') -Raw | ConvertFrom-Json).minecraft.version
$candidates = @((Join-Path $root 'resources'), (Join-Path $root ('.local\horizon-client\' + $version)))
$launcher = $env:HORIZON_LAUNCHER_HOME
if (-not $launcher) { $launcher = Join-Path $env:APPDATA 'horizon-launcher' }
$versions = Join-Path $launcher 'versions'
if (Test-Path -LiteralPath $versions) {
    $candidates += @(Get-ChildItem -LiteralPath $versions -Directory | Sort-Object LastWriteTime -Descending | ForEach-Object {
        Join-Path $_.FullName ('.local\horizon-client\' + $version)
    })
}
$resources = $candidates | Where-Object { Test-Path -LiteralPath (Join-Path $_ 'horizon-resources.json') } | Select-Object -First 1
if (-not $resources) {
    Add-Type -AssemblyName System.Windows.Forms
    $dialog = New-Object System.Windows.Forms.FolderBrowserDialog
    $dialog.Description = 'Select prepared Horizon resources ' + $version + ' (contains horizon-resources.json)'
    if ($dialog.ShowDialog() -eq 'OK') { $resources = $dialog.SelectedPath }
}
if (-not $resources -or -not (Test-Path -LiteralPath (Join-Path $resources 'horizon-resources.json'))) {
    Write-Host 'Prepared Minecraft resources were not found. Launch ordinary Horizon once and retry. See README-WINDOWS.txt.'
    exit 2
}
# Keep this build's worlds and options in its own folder; only read the launcher's resources.
$env:HORIZON_HOME = Join-Path $root 'atomic-data'
New-Item -ItemType Directory -Force $env:HORIZON_HOME | Out-Null
Set-Location $env:HORIZON_HOME
& (Join-Path $root 'horizon-client.exe') '--resources' $resources '--world' 'AtomicAge' '--creative' '--dev-mod' (Join-Path $root 'atomic_age.hmod')
exit $LASTEXITCODE
