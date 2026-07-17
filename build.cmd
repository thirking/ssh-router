@echo off
setlocal

set "ROOT=%~dp0"
set "OUT_DIR=%~1"
if "%OUT_DIR%"=="" set "OUT_DIR=%ROOT%publish"
set "TARGET=x86_64-pc-windows-msvc"

for /f "tokens=2 delims== " %%V in ('findstr /b "version =" "%ROOT%crates\gpui-app\Cargo.toml"') do set "VERSION=%%~V"
set "VERSION_QUAD=%VERSION%.0"

where makensis.exe >nul 2>nul
if errorlevel 1 (
    echo makensis.exe not found. Install NSIS first.
    exit /b 1
)

cargo build -p ssh-router-cli -p ssh-router-gpui --release --target %TARGET%
if errorlevel 1 exit /b %ERRORLEVEL%

if not exist "%OUT_DIR%\resources" mkdir "%OUT_DIR%\resources"
copy /y "%ROOT%target\%TARGET%\release\ssh-router.exe" "%OUT_DIR%\ssh-router.exe" >nul
copy /y "%ROOT%target\%TARGET%\release\ssh-router-cli.exe" "%OUT_DIR%\resources\ssh-router-cli.exe" >nul

makensis.exe /DVERSION=%VERSION% /DVERSION_QUAD=%VERSION_QUAD% /DSOURCE_DIR="%OUT_DIR%" /DOUTPUT_FILE="%OUT_DIR%\SSH-Router_%VERSION%_x64-setup.exe" "%ROOT%installer\ssh-router.nsi"
if errorlevel 1 exit /b %ERRORLEVEL%

echo Build succeeded: "%OUT_DIR%"
