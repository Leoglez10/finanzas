#!/usr/bin/env bash
# Captures every screen in light and dark from the real debug app on a throwaway demo database.
#   pnpm --dir desktop dev &            # vite on :1420
#   desktop/qa/shots.sh <out-dir>       # writes <screen>-<light|dark>.png
# Compare two runs (before/after a change) side by side. Never touches real data.
set -euo pipefail
OUT=${1:?usage: shots.sh <out-dir>}; mkdir -p "$OUT"
QA=$(cd "$(dirname "$0")" && pwd); APP="$QA/../src-tauri"
DB=$(mktemp -d)
trap 'kill "${PID:-0}" 2>/dev/null || true; rm -rf "$DB"' EXIT
(cd "$APP" && cargo build -q && cargo run -q --example demo_db -- "$DB/finanzas.db" >/dev/null)
FINANZAS_DB="$DB/finanzas.db" FINANZAS_DEV_EVAL="$(cat "$QA/shots.js")" "$APP/../../target/debug/finanzas-desktop" >"$DB/app.log" 2>&1 &
PID=$!
done_shots=0
for _ in $(seq 1 240); do
  # ponytail: polls the log every 0.5s; a SHOT line stays valid 1.5s, enough for one capture.
  shots=$(grep '^QA SHOT ' "$DB/app.log" || true)
  count=$(printf '%s' "$shots" | grep -c . || true)
  while (( done_shots < count )); do
    done_shots=$((done_shots + 1))
    name=$(printf '%s\n' "$shots" | sed -n "${done_shots}p" | sed 's/^QA SHOT //')
    screencapture -x -o -l"$(swift "$QA/winshot.swift" finanzas-desktop)" "$OUT/$name.png" && echo "$OUT/$name.png"
  done
  grep -q "^QA DONE" "$DB/app.log" && break
  sleep 0.5
done
grep "^QA FAIL" "$DB/app.log" && exit 1
grep -q "^QA DONE" "$DB/app.log" || { echo "TIMEOUT: the walk never finished"; exit 1; }
