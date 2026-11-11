#!/usr/bin/env bash
set -euo pipefail

cd "$(dirname "${BASH_SOURCE[0]}")/.."
cargo build --release -p finanzas-app -p finanzas-mcp
app="$PWD/target/release/finanzas"
mcp="$PWD/target/release/finanzas-mcp"
if [[ -f "$app.exe" ]]; then app="$app.exe"; mcp="$mcp.exe"; fi
printf 'App binary: %s\nMCP binary: %s\n' "$app" "$mcp"
