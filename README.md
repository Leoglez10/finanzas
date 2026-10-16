# Finanzas

Native macOS personal finance app (SwiftUI + SwiftData): accounts, monthly envelope budget,
upcoming scheduled payments and a bundled MCP server for AI assistants. UI in Spanish.

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
