#!/usr/bin/env bash
# Runs the desktop app with hot reload. FINANZAS_DB=<path> points it at another database.
set -euo pipefail
cd "$(dirname "${BASH_SOURCE[0]}")/.."
pnpm --dir desktop install
pnpm --dir desktop tauri dev
