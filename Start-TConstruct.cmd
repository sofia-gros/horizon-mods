@echo off
setlocal
powershell.exe -NoLogo -NoProfile -ExecutionPolicy Bypass -File "%~dp0Start-TConstruct.ps1" %*
if errorlevel 1 (
  echo.
  echo The game could not start. Check the error above.
  pause
)
endlocal
