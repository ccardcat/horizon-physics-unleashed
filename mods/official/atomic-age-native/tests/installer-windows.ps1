$ErrorActionPreference = 'Stop'
$source = Join-Path $PSScriptRoot '..\packaging\install.ps1'
$tokens = $null; $parseErrors = $null
$ast = [System.Management.Automation.Language.Parser]::ParseFile($source, [ref]$tokens, [ref]$parseErrors)
if ($parseErrors.Count) { throw ($parseErrors -join "`n") }
$function = $ast.Find({ param($node) $node -is [System.Management.Automation.Language.FunctionDefinitionAst] -and $node.Name -eq 'Invoke-LauncherJson' }, $true)
Invoke-Expression $function.Extent.Text
$root = Join-Path ([IO.Path]::GetTempPath()) ('atomic-installer-' + [guid]::NewGuid().ToString('N'))
New-Item -ItemType Directory -Path $root | Out-Null
$env:ATOMIC_INSTALL_TEST_ROOT = $root
$launcher = Join-Path $root 'GUI launcher.exe'
$stub = @'
using System;
using System.IO;
using System.Text;
using System.Web.Script.Serialization;
public static class LauncherFixture {
    [STAThread]
    public static int Main(string[] args) {
        var output = new StreamWriter(Console.OpenStandardOutput(), new UTF8Encoding(false));
        var error = new StreamWriter(Console.OpenStandardError(), new UTF8Encoding(false));
        output.AutoFlush = true; error.AutoFlush = true;
        var json = new JavaScriptSerializer();
        var root = Environment.GetEnvironmentVariable("ATOMIC_INSTALL_TEST_ROOT");
        File.AppendAllText(Path.Combine(root,"calls.jsonl"),json.Serialize(args)+"\n");
        string command = args.Length > 1 ? args[1] : "";
        if (command == "fail") { error.WriteLine("native importer: source Nightly missing \u041e\u0448\u0438\u0431\u043a\u0430"); return 17; }
        if (command == "diskfull") { error.WriteLine("Could not copy C:\\Horizon\\versions\\nightly\\.local: disk full (os error 112)"); return 1; }
        if (command == "empty") { error.WriteLine("no JSON fixture"); return 0; }
        if (command == "invalid") { output.WriteLine("not JSON fixture"); return 0; }
        if (command == "warning") { error.Write(new string('w',200000)); }
        if (command == "import-build") { output.Write(json.Serialize(new {imported="imported-fixture",prepared=true})); return 0; }
        if (command == "instance") { output.Write(json.Serialize(new {id="atomic-fixture",home=Path.Combine(root,"fresh-profile")})); return 0; }
        output.Write(json.Serialize(new {arguments=args})); return 0;
    }
}
'@
Add-Type -TypeDefinition $stub -ReferencedAssemblies 'System.Web.Extensions' -OutputAssembly $launcher -OutputType WindowsApplication
$LASTEXITCODE = 73
$unicode = [string][char]0x041F + [char]0x043E + [char]0x0440
$arguments = @('roundtrip', 'F:\folder with spaces\', 'x"y', 'back\"quote', $unicode, '')
$result = Invoke-LauncherJson -Command $arguments
if ($result.arguments.Count -ne $arguments.Count + 1) { throw 'Argument count changed.' }
for ($i=0; $i -lt $arguments.Count; $i++) {
    if ($result.arguments[$i+1] -cne $arguments[$i]) { throw "Argument $i changed." }
}
$null = Invoke-LauncherJson -Command @('warning')
$thrown = $null
try { $null = Invoke-LauncherJson -Command @('fail') } catch { $thrown = $_.Exception.Message }
if (!$thrown -or !$thrown.Contains('exit code 17') -or !$thrown.Contains('source Nightly missing')) { throw 'Native stderr/exit code was lost.' }
$nativeUnicode = [string][char]0x041E + [char]0x0448 + [char]0x0438 + [char]0x0431 + [char]0x043A + [char]0x0430
if (!$thrown.Contains($nativeUnicode)) { throw 'UTF-8 native error text was corrupted.' }
$thrown = $null
try { $null = Invoke-LauncherJson -Command @('diskfull') } catch { $thrown = $_.Exception.Message }
if (!$thrown -or !$thrown.Contains('Not enough free space') -or !$thrown.Contains('os error 112')) { throw 'Disk-full error lost its cause or recovery instructions.' }
foreach ($case in @('empty','invalid')) {
    $thrown = $null
    try { $null = Invoke-LauncherJson -Command @($case) } catch { $thrown = $_.Exception.Message }
    if (!$thrown -or !$thrown.Contains($(if ($case -eq 'empty') { 'no JSON' } else { 'not JSON fixture' }))) { throw "Missing diagnostic for $case output." }
}
# Run the complete installer against the GUI fixture and verify it touches only the new profile.
Copy-Item -LiteralPath $source -Destination (Join-Path $root 'install.ps1')
[IO.File]::WriteAllText((Join-Path $root 'horizon-client.exe'),'MZfixture')
[IO.File]::WriteAllText((Join-Path $root 'atomic_age_content.hmod'),'fixture-content')
$sentinel = Join-Path $root 'existing-world.sentinel'
[IO.File]::WriteAllText($sentinel,'keep me')
function Read-Host { param([string]$Prompt) 'YES' }
& (Join-Path $root 'install.ps1') -LauncherPath $launcher -NoPause
if ([IO.File]::ReadAllText((Join-Path $root 'fresh-profile\horizon-mods\atomic_age.hmod')) -cne 'fixture-content') { throw 'Content installation failed.' }
if ([IO.File]::ReadAllText((Join-Path $root 'fresh-profile\horizon-mods.txt')) -cne "mod.atomic_age:true`n") { throw 'Mod was not enabled.' }
if ([IO.File]::ReadAllText($sentinel) -cne 'keep me') { throw 'Existing data changed.' }
if ([IO.File]::ReadAllBytes($source) | Where-Object { $_ -gt 127 }) { throw 'Installer must remain ASCII-safe for Windows PowerShell 5.1.' }
Write-Host 'PASS: Windows GUI process wait, stale LASTEXITCODE, quoting, Unicode, stderr, missing JSON and fresh-profile installation.'
