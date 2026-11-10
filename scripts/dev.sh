#!/usr/bin/env bash
set -euo pipefail

cd "$(dirname "${BASH_SOURCE[0]}")/.."
pnpm --dir desktop install
pnpm --dir desktop tauri dev
