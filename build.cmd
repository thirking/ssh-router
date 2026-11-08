@echo off
setlocal

set "ROOT=%~dp0"
if "%~1"=="" (
    set "OUT_DIR=%ROOT%publish"
) else (
    set "OUT_DIR=%~f1"
)
for /f "tokens=2 delims== " %%V in ('findstr /b "version =" "%ROOT%crates\gpuix-app\Cargo.toml"') do set "VERSION=%%~V"
set "VERSION_QUAD=%VERSION%.0"

where makensis.exe >nul 2>nul
if errorlevel 1 (
    echo makensis.exe not found. Install NSIS first.
    exit /b 1
)

cd /d "%ROOT%apps\gpuix"
call bun install --frozen-lockfile
if errorlevel 1 exit /b %ERRORLEVEL%
call bun run typecheck
if errorlevel 1 exit /b %ERRORLEVEL%
call bun run test
if errorlevel 1 exit /b %ERRORLEVEL%
call bun run build
if errorlevel 1 exit /b %ERRORLEVEL%

if not exist "%OUT_DIR%\resources" mkdir "%OUT_DIR%\resources"
copy /y "%ROOT%apps\gpuix\dist\ssh-router.exe" "%OUT_DIR%\ssh-router.exe" >nul
if errorlevel 1 exit /b %ERRORLEVEL%
copy /y "%ROOT%apps\gpuix\dist\ssh-router-backend.exe" "%OUT_DIR%\ssh-router-backend.exe" >nul
if errorlevel 1 exit /b %ERRORLEVEL%
copy /y "%ROOT%apps\gpuix\dist\resources\ssh-router-cli.exe" "%OUT_DIR%\resources\ssh-router-cli.exe" >nul
if errorlevel 1 exit /b %ERRORLEVEL%
copy /y "%ROOT%apps\gpuix\dist\resources\ssh-router-desktop.node" "%OUT_DIR%\resources\ssh-router-desktop.node" >nul
if errorlevel 1 exit /b %ERRORLEVEL%
copy /y "%ROOT%apps\gpuix\dist\resources\gpuix-native.node" "%OUT_DIR%\resources\gpuix-native.node" >nul
if errorlevel 1 exit /b %ERRORLEVEL%

makensis.exe /DVERSION=%VERSION% /DVERSION_QUAD=%VERSION_QUAD% /DSOURCE_DIR="%OUT_DIR%" /DOUTPUT_FILE="%OUT_DIR%\SSH-Router_%VERSION%_x64-setup.exe" "%ROOT%installer\ssh-router.nsi"
if errorlevel 1 exit /b %ERRORLEVEL%

echo Build succeeded: "%OUT_DIR%"
