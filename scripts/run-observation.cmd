@echo off
setlocal
"%SystemRoot%\System32\WindowsPowerShell\v1.0\powershell.exe" -NoLogo -NoProfile -ExecutionPolicy RemoteSigned -File "%~dp0run_standard_observation.ps1" %*
set "observation_exit=%errorlevel%"
echo.
echo Codigo del lanzador: %observation_exit%
pause
exit /b %observation_exit%
