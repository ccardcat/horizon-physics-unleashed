@echo off
setlocal
chcp 65001 >nul
cd /d "%~dp0"
powershell.exe -NoProfile -ExecutionPolicy Bypass -STA -File "%~dp0Start-Atomic-Age.ps1"
if errorlevel 1 pause
endlocal
