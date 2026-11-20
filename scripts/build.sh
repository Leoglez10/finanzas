#!/usr/bin/env bash
# Release build of the desktop app with the MCP server inside it.
# Bundles land in target/release/bundle/. Updater artifacts are signed with TAURI_SIGNING_PRIVATE_KEY
# (CI secret); locally it defaults to ~/.tauri/finanzas-updater.key when that file exists.
set -euo pipefail
cd "$(dirname "${BASH_SOURCE[0]}")/.."

triple="$(rustc -vV | sed -n 's/^host: //p')"
ext=""; [[ "$triple" == *windows* ]] && ext=".exe"
cargo build --release -p finanzas-mcp
mkdir -p desktop/src-tauri/binaries
cp "target/release/finanzas-mcp$ext" "desktop/src-tauri/binaries/finanzas-mcp-$triple$ext"

if [[ -z "${TAURI_SIGNING_PRIVATE_KEY:-}" && -f "$HOME/.tauri/finanzas-updater.key" ]]; then
  export TAURI_SIGNING_PRIVATE_KEY="$HOME/.tauri/finanzas-updater.key" TAURI_SIGNING_PRIVATE_KEY_PASSWORD="${TAURI_SIGNING_PRIVATE_KEY_PASSWORD:-}"
fi
pnpm --dir desktop install --frozen-lockfile
pnpm --dir desktop tauri build --config src-tauri/tauri.release.json "$@"
echo "Bundles: $PWD/target/release/bundle/"
