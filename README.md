# Finanzas

Native macOS personal finance app (SwiftUI + SwiftData): accounts, monthly envelope budget,
upcoming scheduled payments and a bundled MCP server for AI assistants. UI in Spanish.

![macOS 15+](https://img.shields.io/badge/macOS-15%2B-blue) ![Swift 6](https://img.shields.io/badge/Swift-6-orange)

## Features

- **Resumen** — dashboard with total balance, monthly income and expenses, savings rate,
  income vs expenses chart (Swift Charts), spending by category, recent movements and budget progress.
- **Cuentas** — on-budget and off-budget accounts with live balances; create accounts from the sidebar.
- **Movimientos** — searchable ledger per account or across all; quick add with ⌘N, double-click to edit any field.
- **Presupuesto** — monthly envelope budget (assigned, spent, available, carryover). Amount cells accept
  `500`, `+100` or `-50`; add, rename and hide categories and groups.
- **Próximos pagos** — recurring scheduled payments, next 30 days with projected balances, post or skip.
- **Automations** and a bundled **MCP server** (see below).

Everything stays local: no account, no cloud, no telemetry.

## Interface

All screens share a light/dark adaptive palette: petroleum for navigation and primary accents,
white or dark slate cards, emerald for income, coral for expenses, and amber for payment warnings.
The budget highlights amounts available to assign; the ledger keeps native table selection and search;
upcoming payments show date badges and separate post/skip actions. Entry and schedule sheets group
related fields while retaining native controls and keyboard behavior. Import and error dialogs remain native.

After a Debug build, `bash scripts/check-design.sh` renders the real views with synthetic in-memory data
into `build/design-check/` in light and dark mode. It does not construct `FinanzasApp`, open the real
store, or start launch-time payment automation. These renders support manual visual review; they are
not automated assertions for clipping, contrast, or keyboard interaction.

## Requirements

- macOS 15 or later, Xcode 16+
- [XcodeGen](https://github.com/yonaskolb/XcodeGen) (`brew install xcodegen`)

## Install

```sh
xcodegen
xcodebuild -project Finanzas.xcodeproj -scheme Finanzas -configuration Release -derivedDataPath build build
cp -R build/Build/Products/Release/Finanzas.app /Applications/
```

The app is ad-hoc signed; on first launch use right-click › Open if Gatekeeper complains.

## Build

```sh
xcodegen
xcodebuild -project Finanzas.xcodeproj -scheme Finanzas -destination 'platform=macOS' build
swift test --package-path FinanzasKit
```

Data lives in `~/Library/Application Support/Finanzas/Finanzas.store`.
Set `FINANZAS_STORE=/path/to/other.store` to point the app and the MCP server at another store.

## Import from Actual Budget

*Archivo › Importar desde Actual…* accepts Actual's `db.sqlite` or its export zip / server `.blob`.
It only imports into an empty store.

## Automations

- Scheduled payments marked *Registrar automáticamente* are recorded on their date, at launch and hourly.
- A macOS notification arrives N days before each payment at 09:00 (*Finanzas › Ajustes*, default 2 days).
- A movement saved without a category (quick add, `add_transaction`, posted schedules) takes the
  category last used with the same payee.

## MCP server

`Finanzas.app/Contents/MacOS/finanzas-mcp` is a stdio MCP server over the same store. It works
while the app is closed; when the app is open it refreshes within a few seconds of an MCP write.

Claude Code:

```sh
claude mcp add finanzas -- /Applications/Finanzas.app/Contents/MacOS/finanzas-mcp
```

Claude Desktop (`claude_desktop_config.json`):

```json
{ "mcpServers": { "finanzas": { "command": "/Applications/Finanzas.app/Contents/MacOS/finanzas-mcp" } } }
```

Tools (amounts in decimal currency units, negative = outflow; dates `YYYY-MM-DD`, months `YYYY-MM`;
ids look like `Account/p3`):

| Tool | Purpose |
|---|---|
| `list_accounts` | Accounts with balance and flags |
| `get_transactions` | Transactions in a date range, optional account/category, `limit` (default 100) |
| `list_categories` | Category groups and categories |
| `get_budget_month` | Envelope budget of a month |
| `spending_by_category` | Outflows per category in a range |
| `add_transaction` / `update_transaction` / `delete_transaction` | Edit transactions |
| `set_budget_amount` | Assign an amount to an expense category for a month |
| `list_schedules` / `skip_schedule` | Scheduled payments; skip once or `through` a date |
| `get_upcoming` | Overdue and next `days` (default 30) payments with projected balances |

## License

[MIT](LICENSE)
