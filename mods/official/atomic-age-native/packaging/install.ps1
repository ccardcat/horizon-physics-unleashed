param([string]$LauncherPath = '', [switch]$NoPause)
$ErrorActionPreference = 'Stop'
[Console]::OutputEncoding = New-Object System.Text.UTF8Encoding
Write-Host 'Atomic Age Native 0.5.0 | Horizon nightly fdc87ae'
Write-Host 'This imports an unsigned native build and creates a separate Atomic Age instance.'
Write-Host 'Your existing worlds and mods are kept in their current instance.'
$launcher = $LauncherPath
if ([string]::IsNullOrWhiteSpace($launcher)) {
    $launcher = Join-Path ([Environment]::GetFolderPath('Desktop')) 'horizon-launcher.exe'
}
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
    # The launcher is a Windows GUI executable. Wait for the actual process and
    # read both pipes; PowerShell's ambient LASTEXITCODE can be null or stale.
    $arguments = @('--json') + $Command
    $quoted = foreach ($argument in $arguments) {
        if ($argument.Length -gt 0 -and $argument -notmatch '[\s"]') {
            $argument
        } else {
            $escaped = [regex]::Replace($argument, '(\\*)"', '$1$1\"')
            $escaped = [regex]::Replace($escaped, '(\\+)$', '$1$1')
            '"' + $escaped + '"'
        }
    }
    $start = New-Object System.Diagnostics.ProcessStartInfo
    $start.FileName = $launcher
    $start.Arguments = $quoted -join ' '
    $start.UseShellExecute = $false
    $start.CreateNoWindow = $true
    $start.RedirectStandardOutput = $true
    $start.RedirectStandardError = $true
    $start.StandardOutputEncoding = New-Object System.Text.UTF8Encoding($false)
    $start.StandardErrorEncoding = New-Object System.Text.UTF8Encoding($false)
    $process = New-Object System.Diagnostics.Process
    $process.StartInfo = $start
    try {
        if (!$process.Start()) { throw 'Could not start Horizon Launcher.' }
        $stdoutTask = $process.StandardOutput.ReadToEndAsync()
        $stderrTask = $process.StandardError.ReadToEndAsync()
        $process.WaitForExit()
        $stdout = $stdoutTask.GetAwaiter().GetResult()
        $stderr = $stderrTask.GetAwaiter().GetResult()
        $code = $process.ExitCode
    } finally {
        $process.Dispose()
    }
    if ($code -ne 0) {
        $details = ($stderr + "`n" + $stdout).Trim()
        if (!$details) { $details = 'The launcher returned no diagnostic text.' }
        if ($details -match 'os error 112') {
            $details = "Not enough free space in Horizon's version storage. The ZIP location does not control it.`nIn Horizon Launcher, use Settings > Data folder > Move... to choose an empty folder on a drive with enough space, or free space on the current drive. Then retry.`n`n$details"
        }
        throw "Horizon Launcher exit code ${code}:`n$details`nCommand: $($Command -join ' ')"
    }
    if ([string]::IsNullOrWhiteSpace($stdout)) {
        throw "Horizon Launcher returned no JSON output.`n$stderr"
    }
    try { return ($stdout | ConvertFrom-Json) } catch {
        throw "Could not read Horizon Launcher JSON:`n$stdout`n$stderr"
    }
}
# Use the player's currently selected installed Nightly as the resource donor.
$import = Invoke-LauncherJson -Command @('import-build', (Join-Path $PSScriptRoot 'horizon-client.exe'), '--trust-unsigned')
if (!$import.imported) { throw 'The launcher did not return an imported build id.' }
$name = 'Atomic Age 0.5 ' + (Get-Date -Format 'yyyyMMdd-HHmmss')
$instance = Invoke-LauncherJson -Command @('instance', 'create', $name)
if (!$instance.id -or !$instance.home) { throw 'The launcher did not return the new instance folder.' }
$null = Invoke-LauncherJson -Command @('instance', 'version', $instance.id, $import.imported)
$modDir = Join-Path $instance.home 'horizon-mods'
New-Item -ItemType Directory -Force -Path $modDir | Out-Null
Copy-Item -LiteralPath (Join-Path $PSScriptRoot 'atomic_age_content.hmod') -Destination (Join-Path $modDir 'atomic_age.hmod')
[IO.File]::WriteAllText((Join-Path $instance.home 'horizon-mods.txt'), "mod.atomic_age:true`n", (New-Object Text.UTF8Encoding($false)))
$null = Invoke-LauncherJson -Command @('instance', 'select', $instance.id)
Write-Host "Installed: $name"
Write-Host "Instance: $($instance.home)"
Write-Host 'Restart Horizon Launcher, select the new Atomic Age profile, and press Play.'
Write-Host 'Inventory: Atomic Age bombs / weapons & portals.'
if (!$NoPause) { Read-Host 'Press Enter to close' }
