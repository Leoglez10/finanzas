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
- S9 UI framework change: "cambio de planes, hazlo en esto https://github.com/anomalyco/rune". The desktop UI moves from Tauri + Svelte to Rune (native GPU-rendered Rust UI; macOS/Windows/Linux/web). Core and MCP stay.

## Architecture

- Cargo workspace: `crates/finanzas-core` (models, `domain/` pure rules, `store/` SQLite via rusqlite bundled), `crates/finanzas-mcp` (rmcp stdio), `desktop/` (Tauri 2 + Svelte 5 + Vite + TS, `desktop/src-tauri`).
- DB: `FINANZAS_DB` env or `<platform data dir>/Finanzas/finanzas.db`; WAL + busy_timeout; dates TEXT `YYYY-MM-DD`, months INTEGER yyyymm, money INTEGER cents, recurrence TEXT JSON. External ids `"<Entity>/<n>"`.
- App and MCP share the file; the app detects external writes with `PRAGMA data_version` (no history table).

## Tasks

- [x] T8 S2,S5,S9 delegated — Rune shell/sidebar (d01b563), overview and asynchronous refresh (1965a47); writer observed 114 tests, clippy and release build pass; parent inspected light/empty overview PNGs. Tauri retained pending parity.
- [x] T9 S2,S4,S5,S9 delegated — ledger (dc48b17) + entry sheet, create/edit/confirmed delete, pending-mutation guards, contextual selection, no implicit default-DB open — 8ad7954. 25 app / 133 workspace tests + clippy under scratch FINANZAS_DB. Independent verify found 2 high issues (fixed with RED/GREEN); re-verify unavailable (quota) → parent inline: pointer tap+Delete shows identified confirmation; real startup opens only scratch DB. Gaps: failed-save error rendering not shown; release build first timed out at 120s (retried). Cross-process conflicts unchanged.
- [x] T10 S5,S7,S8,S9 delegated — T10a Presupuesto bf985cb: 138 isolated tests, clippy, release build; parent inspected light PNG. Gaps: Asignado commits on Enter only (no blur hook), no group/category context menus, "Nuevo grupo", "Mostrar ocultas". Pending: T10b Próximos pagos, T10c Ajustes/import/reminders. T10b Próximos pagos 8c242d9: 152 isolated tests, clippy, release; parent inspected list + sheet PNGs. TDD caveat: tests written after code, validated by deliberate mutation (guards/freshness/visibility/post-vs-skip). Schedule edit is new design (Swift has none): changing rule resets next_date/completed. Hourly timer/day rollover untested; auto-post failure path untested. T10c Ajustes/Nueva cuenta/import/reminders 469210d: tests-first RED (0/20) then GREEN, 172 isolated tests, clippy, release 2m49s; parent inspected ajustes-dark + recordatorios PNGs. Import source bytes hash-checked unchanged in tests. Gaps: native macOS file dialog not exercised by hand; Actual .zip via UI untested; reminder dismissals session-only (Tauri persisted `sent`); settings write sync on UI thread (tiny file); positive opening balance creates uncategorized income nag.
- [x] T11 S3,S9 inline — desktop/ (Tauri+Svelte) removed; README, scripts/dev.sh, scripts/build.sh, CI (Linux deps libxkbcommon-dev + libwayland-dev, FINANZAS_DB scratch, 3-OS release build) and .gitignore updated — e652984. Isolated: 172 tests, clippy clean, scripts/build.sh release OK (2m41s). CI not run (branch not pushed).
- [x] T0 S1 inline — workspace + models skeleton — 723a434
- [x] T1 S2,S4 delegated — `domain/` pure rules, 36 tests — 5342d6e (Swift wins over spec on conflicts: reminder date dd/MM/yyyy, UpcomingItem.auto_post)
- [x] T2a S2 delegated — schema, balances (incl. empty accounts), envelope projection, default path — 238a5a0
- [x] T2b S2,S4 delegated — CRUD for all models, entries filter/search, payee helpers, uncategorized income, 52 tests — f57ccd0 (suggested_category RED was a FK error, not an assertion; its test asserts newest-wins + missing)
- [x] T2c S7 delegated — SwiftData import via read-only open + in-memory backup, 55 tests — 8844484. Live check (in-memory target): 4 accounts, 15 categories, 5 payees, 96 entries, 3 schedules, 17 budgets; balances match sqlite3 (BBVA Leo -49.00, others 0); source .store and -wal sha1 unchanged
- [x] T2d S8 delegated — Actual import + budgets + schedule dates, 6 Swift tests ported, 66 core tests — 0cf282a, afc71ab (no live Actual DB tested)
- [~] T3 S2,S4 delegated — service layer, 69 core tests — 85aaf37. Partial: only 3 combined service tests and RED was compile-only; the per-behavior parity is pinned by T4's ported FinanzasToolsTests (end-to-end through service)
- [x] T4a S6 delegated — MCP crate: arg validation, 12 specs, 7 read-only tools, 82 workspace tests — 7d444b0 (integer error wording follows my wrong prompt, fixed in T4b to Swift's 'expected an integer from A to B')
- [x] T4b S6 delegated — 5 write tools (9b73e24) + rmcp stdio binary + spawn-the-binary JSON-RPC test, 94 workspace tests — ec1429a
- [x] T5a S3,S5 delegated — Tauri 2 + Svelte shell, sidebar, palette, data_version polling (own cargo workspace) — 1415fca; pnpm check/build + cargo check pass; not run by hand yet
- [x] T5b S2,S5 delegated — Resumen + Movimientos + entry sheet, async commands, startup/hourly auto-post — c1feda3; check/build/clippy pass; not exercised in a running app
- [x] T5c S5,S7,S8 delegated — Presupuesto, Próximos pagos, Ajustes, Nueva cuenta, in-app reminders, SwiftData/Actual import UI, 3 desktop tests — e8a03f8; not exercised in a running app
- [x] T7 S4,S5 delegated — overview parity: first-run counts open accounts only; budget card = top 5 positive assignments with -activity — ea550e2, 98 tests
- [x] T6 S3 delegated — README, scripts/dev.sh, scripts/build.sh, .github/workflows/rust.yml (test + macOS/Windows/Ubuntu tauri build) — 12258d9; CI not run yet (needs push)

Route evidence: context backstop fired at T0 (153k tokens); T1/T2 are independent units of several files each → parallel background writers.

## Log

- L1 2026-11-10 user: "si quiero rehacer la app en rust se puede?"
- L2 2026-11-10 user: "es que es muy lenta, aparte si la podemos hacer con mas plataformas seria increible, hazlo en un nuevo worktree, me haciendo commit"
- L3 Data volume on the live store is tiny (96 entries, 4 accounts, 15 categories, 3 schedules, 17 budget rows): slowness comes from SwiftUI/SwiftData patterns (spec §8), not volume.
- L5 Route correction: T1 and T2 share one crate, so T1's RED tests break T2's test build. T2 paused (schema + 3 store tests done, uncommitted) and resumes after T1 is GREEN; future parallel units must live in separate crates.
- L6 Writers stop after a few behaviors; T2 split into T2a–T2d so each run has a bounded scope. Delivery: slice count includes the 13 Swift commits already on feat/native-mac-app; PR slicing decided when the user asks for a PR.
- L7 T5b review found core follow-ups: `service::overview` picks budget-progress rows from spending instead of Swift's selection; `first_run_checks` counts closed accounts; entry search filters in Rust after SQL fetch (acceptable for S2, backend-side). Fix overview parity after T4 lands.
- L8 Smoke test 2026-11-10: `tauri build --bundles app` OK (Finanzas.app 14.3 MiB, built in 1m07s). Live SwiftData store imported read-only into a scratch FINANZAS_DB (4/15/5/96/3/17; source .store/-wal sha1 unchanged before and after). Release app launched against it and stayed up (~98 MB RSS); screenshot unavailable (no screen-recording permission) so screens were NOT visually verified. Release finanzas-mcp over stdio against the same DB: list_accounts matches (BBVA Leo -49.00, others 0) and get_budget_month 2026-11 returns groups. CI workflow not run (branch not pushed). Final suite: 98 workspace tests + 3 desktop tests, clippy clean.
- L9 2026-11-10 user: "cambio de planes, hazlo en esto https://github.com/anomalyco/rune" — replace the Tauri+Svelte desktop with a Rune app (pinned git rev of `dev`, pre-release). Rune renders headless to PNG, which finally allows visual verification of screens. Tauri desktop/ is kept until the Rune app reaches parity, then removed.
- L10 Independent verify (Risk item 1): imports PASS; found Store::open could mutate a foreign DB via FINANZAS_DB. Fixed cc69b2d (Core Data/unversioned refusal) then 8f932f1 (allowlist: supported user_version + all Finanzas tables, inspected read-only first). Re-verify of cc69b2d flagged the versioned-foreign gap that 8f932f1 closes. Live: legacy path refused, sha1 unchanged; imported scratch DB still opens. Final verify of 8f932f1: PASS on all 4 items (40 store tests). Residual: inspect→reopen race (unverified); malformed files return SQLite/IO errors (still before any write).
- L11 Recovery: earlier task-row insertion did not match, leaving Rune tasks absent from this document. T8–T11 rows reconciled from commits, writer evidence and visible tasks; T9 remains unverified pending independent checks. For T9 use this worktree, not the original checkout.
- L12 Independent T9 verification BLOCKED despite 131 isolated passing tests: failed-save test reaches the default DB opener (scratch DB created under explicit FINANZAS_DB; production effects unknown, not inspected); stale selection can delete an invisible prior-account entry (source-established, not runtime-reproduced). Form light labels ~4.21:1 and missing explicit accessible labels. Correction worker mwbix67a-14-cmqx addressing these with regression tests; T9 remains open. Every test/app command now requires explicit scratch FINANZAS_DB; TMPDIR alone is insufficient. Duplicate mutation assertions requested. Cross-process conflicts and pre-existing stale-error handling remain unchanged.
- L13 Re-verifier mwbj7mok-15-3hrr failed on provider quota; parent ran bounded inline runtime checks instead (scratch DB, PNGs inspected).
- L14 Rune reached parity on all screens (T8–T10); Tauri desktop removed (T11). Architecture section above still describes desktop/ historically.
- L4 Swift app stays in the repo untouched until the Rust app reaches parity; removal is a later user decision.
