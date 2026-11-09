@echo off
setlocal
cd /d "%~dp0.."
if not exist "start.ps1" (
  echo [ERROR] No start.ps1 found in %CD%
  exit /b 1
)
powershell.exe -NoProfile -ExecutionPolicy Bypass -File "%CD%\start.ps1" %*
if errorlevel 1 (
  echo.
  echo [formcraft-start.bat] exited with error - window held open, instacrash guard
  pause
  exit /b 1
)
endlocal
exit /b 0
