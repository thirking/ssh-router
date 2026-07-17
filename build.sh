#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "$0")" && pwd)"
OUT_DIR="${1:-$ROOT/publish}"
TARGET=x86_64-pc-windows-msvc
VERSION="$(sed -n 's/^version = "\([^"]*\)"/\1/p' "$ROOT/crates/gpui-app/Cargo.toml" | head -1)"

if ! command -v makensis.exe >/dev/null 2>&1 && ! command -v makensis >/dev/null 2>&1; then
  echo "makensis not found. Install NSIS first." >&2
  exit 1
fi
MAKENSIS="$(command -v makensis.exe || command -v makensis)"

cargo build -p ssh-router-cli -p ssh-router-gpui --release --target "$TARGET"
mkdir -p "$OUT_DIR/resources"
cp "$ROOT/target/$TARGET/release/ssh-router.exe" "$OUT_DIR/ssh-router.exe"
cp "$ROOT/target/$TARGET/release/ssh-router-cli.exe" "$OUT_DIR/resources/ssh-router-cli.exe"
"$MAKENSIS" \
  "/DVERSION=$VERSION" \
  "/DVERSION_QUAD=$VERSION.0" \
  "/DSOURCE_DIR=$OUT_DIR" \
  "/DOUTPUT_FILE=$OUT_DIR/SSH-Router_${VERSION}_x64-setup.exe" \
  "$ROOT/installer/ssh-router.nsi"

echo "Build succeeded: $OUT_DIR"
