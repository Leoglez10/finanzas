#!/usr/bin/env bash
# Drives the real debug app with a script and screenshots its window. Never touches real data.
#   pnpm dev &                                   # vite on :1420
#   desktop/qa/run.sh desktop/qa/movimientos.js /tmp/out.png
# The script logs PASS/FAIL lines in a panel inside the window; read them from the screenshot.
set -euo pipefail
QA=$(cd "$(dirname "$0")" && pwd); APP="$QA/../src-tauri"
DB=$(mktemp -d)
trap 'rm -rf "$DB"' EXIT
(cd "$APP" && cargo build -q && cargo run -q --example demo_db -- "$DB/finanzas.db" >/dev/null)
FINANZAS_DB="$DB/finanzas.db" FINANZAS_DEV_EVAL="$(cat "$1")" "$APP/target/debug/finanzas-desktop" >/dev/null 2>&1 &
PID=$!
sleep "${3:-16}"
screencapture -x -o -l"$(swift "$QA/winshot.swift" finanzas-desktop)" "$2"
kill "$PID"
echo "screenshot: $2"
