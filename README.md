# Finanzas

Local personal finance app for macOS, Windows and Linux, built with Rust and Tauri 2.
Manage accounts, envelope budgets and scheduled payments, and connect AI assistants through MCP.
The interface is in Spanish.

## Features

- **Resumen** — balances, monthly income and expenses, spending and budget progress.
- **Cuentas** — on-budget and off-budget accounts with balances.
- **Movimientos** — searchable ledger and transaction editing.
- **Presupuesto** — monthly envelope budget with assignments, activity and carryover.
- **Próximos pagos** — scheduled payments and projected balances.
- **Automations** and a local **MCP server** (see below).

Everything stays local: no account, cloud or telemetry.

## Interface

The Spanish interface uses a light/dark adaptive palette: petroleum navigation, emerald income,
coral expenses and amber payment warnings. The sidebar connects the overview, accounts, ledger,
budget and upcoming payments.

## Requirements

- Rust stable, Node.js 20.19+ and pnpm.
- Platform dependencies ([Tauri 2 prerequisites](https://v2.tauri.app/start/prerequisites/)):

| Platform | Dependencies |
|---|---|
| macOS | Xcode Command Line Tools (`xcode-select --install`) |
| Windows | Microsoft C++ Build Tools with Desktop development with C++, and WebView2 |
| Debian/Ubuntu | `libwebkit2gtk-4.1-dev`, `libayatana-appindicator3-dev`, `librsvg2-dev`, `build-essential`, `curl`, `wget`, `file`, `libxdo-dev`, `libssl-dev` |

## Install

Install dependencies and start the app in development mode from the repository root:

```sh
pnpm --dir desktop install
pnpm --dir desktop tauri dev
```

Alternatively, run `bash scripts/dev.sh`.

## Build

```sh
pnpm --dir desktop install
pnpm --dir desktop tauri build
cargo test --workspace
```

Tauri writes platform bundles under `desktop/src-tauri/target/release/bundle/`.
`bash scripts/build.sh` also builds the release MCP server.

## Data location

The app and MCP server share a SQLite database. Set `FINANZAS_DB` to use a different file;
otherwise the default is the platform data directory:

| Platform | Default database |
|---|---|
| macOS | `~/Library/Application Support/Finanzas/finanzas.db` |
| Windows | `%APPDATA%\Finanzas\finanzas.db` |
| Linux | `~/.local/share/Finanzas/finanzas.db` (or `$XDG_DATA_HOME/Finanzas/finanzas.db`) |

## MCP server

Build the stdio server and register the resulting binary with your MCP client:

```sh
cargo build --release -p finanzas-mcp
```

Use the absolute path to `target/release/finanzas-mcp` as the client command
(`finanzas-mcp.exe` on Windows). Optionally set `FINANZAS_DB` in the client's environment
to use the same non-default database as the app. The server also works while the app is closed.

Amounts are decimal currency units (negative for outflows); dates use `YYYY-MM-DD` and months use `YYYY-MM`.

| Tool | Purpose |
|---|---|
| `list_accounts` | List accounts and balances |
| `get_transactions` | Query transactions |
| `list_categories` | List category groups and categories |
| `get_budget_month` | Get a monthly envelope budget |
| `spending_by_category` | Summarize outflows by category |
| `add_transaction` | Add a transaction |
| `update_transaction` | Update a transaction |
| `delete_transaction` | Delete a transaction |
| `set_budget_amount` | Assign a monthly category amount |
| `list_schedules` | List scheduled payments |
| `skip_schedule` | Skip a scheduled payment |
| `get_upcoming` | Get upcoming payments and projected balances |

## Import

On first run, **Importar mis datos de la app anterior** imports the old SwiftData store
read-only, without modifying it. Actual Budget data can also be imported into an empty store
from `db.sqlite`, an export zip or a server `.blob`.

## Legacy Swift app

The SwiftUI app under `Finanzas/`, `FinanzasKit/` and `FinanzasMCP/` still builds with the old
macOS instructions until it is removed. It requires macOS 15+, Xcode 16+ and XcodeGen:

```sh
xcodegen
xcodebuild -project Finanzas.xcodeproj -scheme Finanzas -configuration Release -derivedDataPath build build
cp -R build/Build/Products/Release/Finanzas.app /Applications/
swift test --package-path FinanzasKit
```

Its store is `~/Library/Application Support/Finanzas/Finanzas.store`; `FINANZAS_STORE` overrides it.
The ad-hoc signed app may require right-click › Open on first launch.

## License

[MIT](LICENSE)
