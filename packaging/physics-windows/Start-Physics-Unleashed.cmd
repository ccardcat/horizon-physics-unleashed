@echo off
setlocal
chcp 65001 >nul
cd /d "%~dp0"
rem Use existing prepared resources, but keep this demo's saves away from a real player's worlds.
set "HORIZON_HOME=%~dp0demo-data"
powershell.exe -NoProfile -STA -Command "$ErrorActionPreference='Stop'; $root=(Get-Location).Path; $version=(Get-Content -LiteralPath (Join-Path $root 'research-lock.json') -Raw | ConvertFrom-Json).minecraft.version; $candidates=@((Join-Path $root 'resources'),(Join-Path $root ('.local\horizon-client\'+$version))); $launcher=$env:HORIZON_LAUNCHER_HOME; if (-not $launcher) { $launcher=Join-Path $env:APPDATA 'horizon-launcher' }; $versions=Join-Path $launcher 'versions'; if (Test-Path -LiteralPath $versions) { $candidates+=@(Get-ChildItem -LiteralPath $versions -Directory | Sort-Object LastWriteTime -Descending | ForEach-Object { Join-Path $_.FullName ('.local\horizon-client\'+$version) }) }; $resources=$candidates | Where-Object { Test-Path -LiteralPath (Join-Path $_ 'horizon-resources.json') } | Select-Object -First 1; if (-not $resources) { Add-Type -AssemblyName System.Windows.Forms; $dialog=New-Object System.Windows.Forms.FolderBrowserDialog; $dialog.Description='Выберите папку готовых ресурсов Horizon '+$version+' (в ней есть horizon-resources.json). Инструкция: README-WINDOWS.txt'; if ($dialog.ShowDialog() -eq 'OK') { $resources=$dialog.SelectedPath } }; if (-not $resources -or -not (Test-Path -LiteralPath (Join-Path $resources 'horizon-resources.json'))) { Write-Host 'Ресурсы Minecraft не найдены. Откройте README-WINDOWS.txt: там описана подготовка через обычный Horizon Launcher.'; exit 2 }; & (Join-Path $root 'horizon-client.exe') '--resources' $resources '--world' 'PhysicsVisual' '--creative'; exit $LASTEXITCODE"
if errorlevel 1 pause
endlocal
