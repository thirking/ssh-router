#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "$0")" && pwd)"
OUT_DIR="${1:-$ROOT/publish}"
VERSION="$(sed -n 's/^version = "\([^"]*\)"/\1/p' "$ROOT/crates/gpuix-app/Cargo.toml" | head -1)"

case "$(uname -s)" in
  MINGW*|MSYS*|CYGWIN*) ;;
  *) echo "Windows installer must be built on Windows. Use apps/gpuix for local development." >&2; exit 1 ;;
esac

if ! command -v makensis.exe >/dev/null 2>&1 && ! command -v makensis >/dev/null 2>&1; then
  echo "makensis not found. Install NSIS first." >&2
  exit 1
fi
MAKENSIS="$(command -v makensis.exe || command -v makensis)"
mkdir -p "$OUT_DIR/resources"
OUT_DIR="$(cd "$OUT_DIR" && pwd)"

cd "$ROOT/apps/gpuix"
bun install --frozen-lockfile
bun run typecheck
bun run test
bun run build
cp "$ROOT/apps/gpuix/dist/ssh-router.exe" "$OUT_DIR/ssh-router.exe"
cp "$ROOT/apps/gpuix/dist/ssh-router-backend.exe" "$OUT_DIR/ssh-router-backend.exe"
cp "$ROOT/apps/gpuix/dist/resources/ssh-router-cli.exe" "$OUT_DIR/resources/ssh-router-cli.exe"
cp "$ROOT/apps/gpuix/dist/resources/ssh-router-desktop.node" "$OUT_DIR/resources/ssh-router-desktop.node"
cp "$ROOT/apps/gpuix/dist/resources/gpuix-native.node" "$OUT_DIR/resources/gpuix-native.node"
"$MAKENSIS" \
  "/DVERSION=$VERSION" \
  "/DVERSION_QUAD=$VERSION.0" \
  "/DSOURCE_DIR=$OUT_DIR" \
  "/DOUTPUT_FILE=$OUT_DIR/SSH-Router_${VERSION}_x64-setup.exe" \
  "$ROOT/installer/ssh-router.nsi"

echo "Build succeeded: $OUT_DIR"
