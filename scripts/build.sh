#!/usr/bin/env bash
set -euo pipefail

cd "$(dirname "${BASH_SOURCE[0]}")/.."
pnpm --dir desktop install
pnpm --dir desktop tauri build
cargo build --release -p finanzas-mcp
mcp="$PWD/target/release/finanzas-mcp"
if [[ -f "$mcp.exe" ]]; then mcp="$mcp.exe"; fi
printf 'Desktop bundles: %s\nMCP binary: %s\n' \
  "$PWD/desktop/src-tauri/target/release/bundle/" "$mcp"
