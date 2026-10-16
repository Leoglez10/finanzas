#!/bin/bash
# Build the Debug app first. Render its real views without constructing FinanzasApp.
set -euo pipefail
cd "$(dirname "$0")/.."
products="$PWD/build/Build/Products/Debug"
work="$PWD/build/design-check"
mkdir -p "$work"
python3 - "$work" <<'PY'
from pathlib import Path
import sys
source = Path('Finanzas/FinanzasApp.swift').read_text()
assert source.count('@main\n') == 1
Path(sys.argv[1], 'AppWithoutEntrypoint.swift').write_text(source.replace('@main\n', '', 1))
PY
sources=()
for source in Finanzas/*.swift; do
    [[ "$source" == "Finanzas/FinanzasApp.swift" ]] || sources+=("$source")
done
xcrun swiftc -parse-as-library -swift-version 6 \
    -target "$(uname -m)-apple-macos15.0" \
    -module-cache-path "$work/module-cache" -I "$products" \
    "${sources[@]}" "$work/AppWithoutEntrypoint.swift" \
    scripts/check-design.swift "$products/FinanzasKit.o" -o "$work/check-design"
"$work/check-design" "$work"
