#!/usr/bin/env bash
# Drives the real debug app with a QA script on a throwaway demo database; never touches real data.
#   pnpm --dir desktop dev &            # vite on :1420
#   desktop/qa/run.sh desktop/qa/movimientos.js [screenshot.png]
# Scripts report PASS/FAIL lines through the debug-only qa_log command; exits 1 on any FAIL or timeout.
set -euo pipefail
QA=$(cd "$(dirname "$0")" && pwd); APP="$QA/../src-tauri"
DB=$(mktemp -d)
trap 'kill "${PID:-0}" 2>/dev/null || true; rm -rf "$DB"' EXIT
(cd "$APP" && cargo build -q && cargo run -q --example demo_db -- "$DB/finanzas.db" >/dev/null)
FINANZAS_DB="$DB/finanzas.db" FINANZAS_DEV_EVAL="$(cat "$1")" "$APP/target/debug/finanzas-desktop" >"$DB/app.log" 2>&1 &
PID=$!
for _ in $(seq 1 120); do grep -q "^QA DONE" "$DB/app.log" && break; sleep 1; done
if [[ -n "${2:-}" ]]; then screencapture -x -o -l"$(swift "$QA/winshot.swift" finanzas-desktop)" "$2" || true; fi
grep "^QA " "$DB/app.log" | sed 's/^QA //'
grep -iE "panic|error|bandeja" "$DB/app.log" | grep -v "^QA " || true
grep -q "^QA DONE" "$DB/app.log" || { echo "TIMEOUT: the script never finished"; exit 1; }
! grep -q "^QA FAIL" "$DB/app.log"
