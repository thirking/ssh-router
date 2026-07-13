@echo off
REM Build script for SSH Router (Tauri v2 + Rust CLI)
REM
REM 双程序构建:
REM - ssh-router-cli.exe: 被 sshd 调起的路由 CLI
REM - ssh-router.exe:     托盘 GUI (Tauri v2 + React)
REM
REM 用法:
REM   build.cmd                 编译到 .\publish\
REM   build.cmd "D:\deploy"     编译到指定目录

set OUT_DIR=%~1
if "%OUT_DIR%"=="" set OUT_DIR=%~dp0publish

echo Building ssh-router-cli.exe...
cargo build -p ssh-router-cli --release --target x86_64-pc-windows-msvc
if %ERRORLEVEL% neq 0 (
    echo CLI build failed
    exit /b %ERRORLEVEL%
)

echo Building ssh-router.exe (Tauri GUI)...
call npm install
call npm run build
cargo tauri build
if %ERRORLEVEL% neq 0 (
    echo Tauri build failed
    exit /b %ERRORLEVEL%
)

REM 汇集产物
if not exist "%OUT_DIR%" mkdir "%OUT_DIR%"
copy "%~dp0target\x86_64-pc-windows-msvc\release\ssh-router-cli.exe" "%OUT_DIR%\"
copy "%~dp0src-tauri\target\release\ssh-router.exe" "%OUT_DIR%\"

echo Build succeeded:
echo   %OUT_DIR%\ssh-router-cli.exe
echo   %OUT_DIR%\ssh-router.exe
