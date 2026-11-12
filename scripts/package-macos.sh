#!/usr/bin/env bash
# Builds dist/Finanzas-macos-<arch>.dmg (stable name, so "latest" download links keep working) from the release binaries.
# Unsigned: on first launch macOS asks to open it with right click → Abrir.
set -euo pipefail
VERSION="${1:?usage: package-macos.sh <version>}"
ARCH="$(uname -m)"
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
STAGE="$(mktemp -d)"
trap 'rm -rf "$STAGE"' EXIT
APP="$STAGE/Finanzas.app"
mkdir -p "$APP/Contents/MacOS" "$APP/Contents/Resources" "$ROOT/dist"
cp "$ROOT/packaging/macos/Finanzas.icns" "$APP/Contents/Resources/"
cp "$ROOT/target/release/finanzas" "$ROOT/target/release/finanzas-mcp" "$APP/Contents/MacOS/"
sed "s/@VERSION@/$VERSION/g" "$ROOT/packaging/macos/Info.plist" > "$APP/Contents/Info.plist"
# Ad-hoc signature only, so Apple Silicon runs it; this is not a Developer ID signature.
codesign --force --deep --sign - "$APP"
ln -s /Applications "$STAGE/Applications"
OUT="$ROOT/dist/Finanzas-macos-$ARCH.dmg"
rm -f "$OUT"
hdiutil create -volname Finanzas -srcfolder "$STAGE" -ov -format UDZO "$OUT" >/dev/null
echo "$OUT"
