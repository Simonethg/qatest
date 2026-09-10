@echo off
REM Bootstrap when fileless PowerShell install is blocked. Checksums still come from latest.json.
powershell -NoProfile -ExecutionPolicy Bypass -File "%~dp0install.ps1" %*
