@echo off
REM Build script for SshRouter
REM Requires .NET Framework 4.0+ (included with Windows)

set CSC=C:\Windows\Microsoft.NET\Framework64\v4.0.30319\csc.exe
set OUT=%~dp0SshRouter.exe
set SRC=%~dp0SshRouter.cs

if not exist "%CSC%" (
    echo Error: csc.exe not found at %CSC%
    echo Please install .NET Framework 4.0+
    exit /b 1
)

echo Compiling SshRouter.exe...
"%CSC%" /nologo /out:"%OUT%" "%SRC%"

if %ERRORLEVEL% equ 0 (
    echo Build succeeded: %OUT%
) else (
    echo Build failed with error %ERRORLEVEL%
)

exit /b %ERRORLEVEL%
