<p align="center"><img src="packaging/icon/finanzas-1024.png" width="128" alt="Finanzas icon"></p>

# Finanzas

[![Latest release](https://img.shields.io/github/v/release/Leoglez10/finanzas?label=versi%C3%B3n)](https://github.com/Leoglez10/finanzas/releases/latest)
[![Rust CI](https://github.com/Leoglez10/finanzas/actions/workflows/rust.yml/badge.svg)](https://github.com/Leoglez10/finanzas/actions/workflows/rust.yml)
[![Release](https://github.com/Leoglez10/finanzas/actions/workflows/release.yml/badge.svg)](https://github.com/Leoglez10/finanzas/actions/workflows/release.yml)
[![Downloads](https://img.shields.io/github/downloads/Leoglez10/finanzas/total)](https://github.com/Leoglez10/finanzas/releases)
[![Website](https://img.shields.io/badge/web-leoglez10.github.io%2Ffinanzas-155E75)](https://leoglez10.github.io/finanzas/)

Local personal finance app for macOS, Windows and Linux, built in Rust with the [Rune](https://github.com/anomalyco/rune) GPU UI framework.
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

- Rust stable (edition 2024).
- Platform dependencies:

| Platform | Dependencies |
|---|---|
| macOS | Xcode Command Line Tools (`xcode-select --install`) |
| Windows | Microsoft C++ Build Tools with Desktop development with C++ |
| Debian/Ubuntu | Build: `libxkbcommon-dev`, `libwayland-dev`. Run: `libxkbcommon-x11-0` and a Vulkan driver (`mesa-vulkan-drivers` without a GPU) |

The file dialogs used by import go through the XDG desktop portal on Linux.

## Install

These links always download the latest release:

| Platform | Download | Notes |
|---|---|---|
| macOS (Apple Silicon) | [Finanzas-macos-arm64.dmg](https://github.com/Leoglez10/finanzas/releases/latest/download/Finanzas-macos-arm64.dmg) | Drag Finanzas to Applications; the first time, right-click the app and choose Open. |
| Windows (x64) | [Finanzas-windows-x64-setup.exe](https://github.com/Leoglez10/finanzas/releases/latest/download/Finanzas-windows-x64-setup.exe) | Installs for the current user; if SmartScreen appears, choose More info → Run anyway. |
| Linux (Debian/Ubuntu) | [finanzas-linux-amd64.deb](https://github.com/Leoglez10/finanzas/releases/latest/download/finanzas-linux-amd64.deb) | `sudo apt install ./finanzas-linux-amd64.deb` |
| Linux (other) | [finanzas-linux-x86_64.tar.gz](https://github.com/Leoglez10/finanzas/releases/latest/download/finanzas-linux-x86_64.tar.gz) | Extract and run `./finanzas`. |

The installers are not signed yet. Each one also ships the MCP server (`finanzas-mcp`). Your data
stays in the data location below, so reinstalling or upgrading keeps it.

### Releasing

Releases are automatic: bump `version` in the root `Cargo.toml` and merge to `main`.
`.github/workflows/release.yml` sees that `v<version>` has no release yet, builds the three
installers, creates the tag and publishes the release. Pushing a `v*` tag by hand also works.
`bash scripts/package-macos.sh <version>` builds the `.dmg` locally after a release build.

### From source

Start the app from the repository root:

```sh
cargo run --release -p finanzas-app
```

Alternatively, run `bash scripts/dev.sh`.

## Build

```sh
cargo build --release -p finanzas-app -p finanzas-mcp
cargo test --workspace
```

The app binary is `target/release/finanzas` (`finanzas.exe` on Windows); `bash scripts/build.sh`
builds it together with the MCP server. The app also renders headless for checks without a window,
for example `target/release/finanzas --scene demo-light --size 1200x800 --snap resumen.png`;
the `--scene` demos use in-memory data. Run tests with `FINANZAS_DB` pointing at a scratch file.

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

**Ajustes › Importar datos** copies data into an empty database. **Importar desde Finanzas
(SwiftData)** reads the old app's store (`~/Library/Application Support/Finanzas/Finanzas.store`)
read-only, without modifying it. **Importar desde Actual** accepts `db.sqlite`, an export zip or a
server `.blob`. A confirmation names the source file and the destination database first.

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

## Built with
[![Built with Gentle-AI](https://raw.githubusercontent.com/Gentleman-Programming/gentle-ai/main/docs/assets/brand/built-with-gentle-ai.png)](https://github.com/Gentleman-Programming/gentle-ai)

## License

[MIT](LICENSE)
