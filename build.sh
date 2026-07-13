#!/bin/bash
# Build script for SSH Router (Tauri v2 + Rust CLI)
#
# 双程序构建:
# - ssh-router-cli.exe: 被 sshd 调起的路由 CLI (Rust + windows crate)
# - ssh-router.exe:     托盘 GUI (Tauri v2 + React + shadcn/ui)
#
# 注意: Tauri 不建议交叉编译, 此脚本应在 Windows 上运行
# macOS 上只能构建 CLI (cargo check --target x86_64-pc-windows-msvc)
#
# 用法 (Windows):
#   ./build.sh              编译到 ./publish/
#   ./build.sh /path/to/dir 编译到指定目录

set -e

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
OUT_DIR="${1:-$SCRIPT_DIR/publish}"

echo "Building ssh-router-cli.exe..."
cargo build -p ssh-router-cli --release --target x86_64-pc-windows-msvc

echo "Building ssh-router.exe (Tauri GUI)..."
npm install
npm run build
cargo tauri build

# 汇集产物
mkdir -p "$OUT_DIR"
cp "$SCRIPT_DIR/target/x86_64-pc-windows-msvc/release/ssh-router-cli.exe" "$OUT_DIR/"
cp "$SCRIPT_DIR/src-tauri/target/release/ssh-router.exe" "$OUT_DIR/"

echo "Build succeeded:"
echo "  $OUT_DIR/ssh-router-cli.exe"
echo "  $OUT_DIR/ssh-router.exe"
