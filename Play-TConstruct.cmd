@echo off
setlocal
powershell.exe -NoLogo -NoProfile -ExecutionPolicy Bypass -File "%~dp0tools\play-windows.ps1"
if errorlevel 1 (
  echo.
  echo The game did not start. Read the message above and press any key to close.
  pause >nul
)
endlocal
