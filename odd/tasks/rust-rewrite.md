Objective: rewrite Finanzas in Rust as a fast, cross-platform app (macOS, Windows, Linux) with the same features, data and MCP tools.
Branch: feat/rust-rewrite (worktree ../finanzas-worktrees/rust-rewrite, from a9b683c) · Delivery: ask-on-risk · TDD: strict (global config) · Runner: `cargo test --workspace`; desktop UI: `pnpm --dir desktop build` + `pnpm --dir desktop check`
Parity reference: `odd/specs/rust-porting-spec.md` (§1–§8, mapped from the Swift app with path:line evidence).

## Specs

- S1 Rewrite in Rust in a new worktree, committing as we go: "hazlo en un nuevo worktree, me haciendo commit".
- S2 Fast: "es que es muy lenta". No full-history recomputation per render: balances and budget activity come from SQL aggregates; the envelope walk runs once per request (one walk serves every month the screen needs); heavy work stays off the UI thread. Avoid every hotspot in spec §8.
- S3 More platforms: "si la podemos hacer con mas plataformas seria increible". One codebase building on macOS, Windows and Linux (Tauri 2).
- S4 Same business rules as spec §3 (envelope math, month summary, recurrence, upcoming, automations, payee→category learning, amount input parsing), with the Swift tests in spec §7 ported.
- S5 Same screens, Spanish copy, shortcuts and palette as spec §6.
- S6 MCP server with the same 12 tools, params, validation and output shapes as spec §4 (stdio).
- S7 Existing data migrates: import the current SwiftData store (`~/Library/Application Support/Finanzas/Finanzas.store`, Core Data SQLite) read-only, never modifying it. Core Data dates are seconds since 2001-01-01 UTC stored at local midnight; `ZRECURRENCEDATA` is the Swift Recurrence JSON with dates in the same epoch.
- S8 Actual Budget import as spec §5.

## Architecture

- Cargo workspace: `crates/finanzas-core` (models, `domain/` pure rules, `store/` SQLite via rusqlite bundled), `crates/finanzas-mcp` (rmcp stdio), `desktop/` (Tauri 2 + Svelte 5 + Vite + TS, `desktop/src-tauri`).
- DB: `FINANZAS_DB` env or `<platform data dir>/Finanzas/finanzas.db`; WAL + busy_timeout; dates TEXT `YYYY-MM-DD`, months INTEGER yyyymm, money INTEGER cents, recurrence TEXT JSON. External ids `"<Entity>/<n>"`.
- App and MCP share the file; the app detects external writes with `PRAGMA data_version` (no history table).

## Tasks

- [ ] T0 S1 inline — workspace + models skeleton — commit pending
- [ ] T1 S2,S4 delegated (writer A, parallel with T2) — `domain/` pure rules + ported tests
- [ ] T2 S2,S7,S8 delegated (writer B, parallel with T1) — `store/` schema, queries, SwiftData migration, Actual import
- [ ] T3 S2,S4 — service layer (post/skip/auto-post, budget month, summaries) used by MCP and desktop
- [ ] T4 S6 — MCP server (parallel with T5)
- [ ] T5 S2,S3,S5 — Tauri desktop app + Svelte UI (parallel with T4)
- [ ] T6 S3 — README, install scripts, CI build matrix (macOS/Windows/Linux)

Route evidence: context backstop fired at T0 (153k tokens); T1/T2 are independent units of several files each → parallel background writers.

## Log

- L1 2026-11-10 user: "si quiero rehacer la app en rust se puede?"
- L2 2026-11-10 user: "es que es muy lenta, aparte si la podemos hacer con mas plataformas seria increible, hazlo en un nuevo worktree, me haciendo commit"
- L3 Data volume on the live store is tiny (96 entries, 4 accounts, 15 categories, 3 schedules, 17 budget rows): slowness comes from SwiftUI/SwiftData patterns (spec §8), not volume.
- L4 Swift app stays in the repo untouched until the Rust app reaches parity; removal is a later user decision.
