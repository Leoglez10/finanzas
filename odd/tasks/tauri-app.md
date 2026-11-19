# Tauri app (redesign)
Objective: replace the Rune UI with a Tauri 2 + Svelte 5 app that implements the approved prototype `design/redesign/index.html` on top of `finanzas-core`. Branch `feat/tauri-app`. Delivery: ask-on-risk; push/PR/merge are the user's decision.
Test runner: `cargo test` (core + desktop crate), `pnpm --dir desktop check` + `node design/redesign/quick-parse.test.cjs` for the frontend parser.

## Specs
S1. "ya empieza la implementacion" — implement the redesign approved in this conversation as a Tauri app. Source of truth for look and behavior: `design/redesign/index.html` (screens Resumen, Movimientos/cuenta, Presupuesto, Próximos pagos, Deseos, Ajustes; dialogs; themes).
S2. Visual rules from the user: "no me gustan los colores graidentes para una interfaz asi, deja colores fijos" — no gradients (the donut's hard-stop segments are not gradients).
S3. "que ajustes tenga distinfras sub pestañas y que cada una tenga lo distinto"; profile lives in Ajustes (avatar bubble next to Ajustes in the sidebar opens Ajustes › Perfil).
S4. Themes: "si, agrégalos todos" — Petróleo, Bosque, Ciruela, Grafito, Arena, Sistema (macOS accent), each light/dark; income/expense/warning/category colors fixed.
S5. Tauri-native features accepted with "agrega todo eso": quick capture with a global shortcut ("uber 120 ayer"), ⌘K palette, keyboard shortcuts + native menu, menu-bar tray, native notifications for payment reminders even with the window closed, undo instead of confirmation dialogs, inline edit in the ledger, CSV drag-and-drop import with preview and duplicate detection, interactive charts, in-app updates.
S6. Data safety: real data stays in the existing per-profile SQLite stores via `finanzas-core`; the MCP server keeps working unchanged. Assumption: the Rune app (`crates/finanzas-app`) stays until the Tauri app reaches parity; removing it is a separate user decision.
S7. Open decision (not blocking early tasks): in-app updater vs Homebrew cask as the update channel.

## Tasks
- T1 [S1,S2,S4] inline — Shell: restore `desktop/` base, design tokens + themes, sidebar, routing, Resumen with real data. Status: done (pnpm check 0 errors, cargo test 4 passed, app launched against demo DB and Resumen screenshot inspected). Commit db7f6f6.
- T2 [S1,S5] inline — Movimientos/cuenta: grouped ledger, search/filters, add/edit dialog, inline edit, delete with undo. Status: done (desktop/qa/movimientos.js 13/13 PASS on the real debug app with a demo DB; quick-parse 6 tests; pnpm check 0 errors).
- T3 [S1] inline — Presupuesto. Status: done (desktop/qa/presupuesto.js 9/9 PASS on the real debug app; pnpm check 0 errors).
- T4 [S1,S5] inline — Próximos pagos (register/skip/schedules) + reminders as native notifications + tray. Status: done (desktop/qa/proximos.js 13/13 PASS; movimientos 13/13 and presupuesto 9/9 re-run; workspace cargo test + clippy clean after the core move). Commits 457ec03 (core refactor, Risk item 3), ad23261 (flaky test fix), next feat commit.
- T5 [S1] inline — Deseos (reserve, release, buy, archive, two-step editor). Status: done (desktop/qa/deseos.js 17/17 PASS; core restore_wish with test). 
- T6 [S3,S4] inline — Ajustes sub-tabs: Perfil (profiles), General (theme/mode/tray/autostart), Avisos, Datos, Actualizaciones, Atajos; persisted settings. Status: done (desktop/qa/ajustes.js 16/16 PASS three runs in a row; earlier QA scripts re-run).
- T7 [S5] inline — ⌘K palette, native menu + shortcuts, quick-capture window + global shortcut. Status: pending.
- T8 [S5] inline — CSV drag-and-drop import. Status: pending.
- T9 [S6,S7] — Packaging/release/cask switch, retire Rune app (needs user decision). Status: pending.

## Log
L1. 2026-11-18 user request verbatim: "ya empieza la implementacion".
L2. Prior art: Tauri 2 + Svelte app removed in e652984 ("remove Tauri desktop app in favor of the Rune app"); restored `desktop/` from e652984^ as the base for the Rust command layer, icons and config. Frontend components are rewritten to the new design.
L3. Route: inline; no writer reason (units are sequential and depend on the shell). Risk: item 5 (new JS/Rust dependencies: Tauri plugins, Svelte toolchain) — independent verifier at feature end.
L4. T2 verification: real app driven through FINANZAS_DEV_EVAL (debug builds only) via desktop/qa/run.sh; System Events accessibility was not permitted, so no permission was requested. Undo of a delete re-adds the movement with a new id (schedule link not restored).
L5. T4: schedule draft rules moved to finanzas-core (shared with the Rune app, re-exported; 217 workspace tests + clippy green). Undo snapshots the schedule before post/skip/delete and restores it. Tray: native menu (balance title, next payment, open, quit); closing the window hides it while the tray exists so reminders keep firing. Native notifications have no action buttons on desktop (tauri-plugin-notification limitation), so "Registrar" from a notification is not available; the tray item opens Próximos instead.
L6. QA runner hardened: covered windows suspend WKWebView, so QA scripts wait through a debug-only qa_sleep command, report through qa_log, and the window is created from config with background throttling disabled only when FINANZAS_DEV_EVAL is set (debug builds). Two script bugs were found and fixed (selectors matched the schedules list); the app behaved correctly. Settings tests were flaky (shared scratch dir) and are fixed.
L7. T5: core wish errors are English, translated in desktop/src/lib/api.ts errorText. "Ver producto" uses tauri-plugin-opener (new dependency, Risk item 5). "Lo compré" records the expense in the first open on-budget account and archives the wish; undo removes both.
L8. T6: profiles switch by swapping the shared Store handle; the active profile is remembered in settings.json (`profile`). Deleting a profile is irreversible (deletes the database), so it keeps a confirmation dialog with focus on "Conservar" instead of undo. Found and fixed: hiding a profile or changing its color did not refresh the list (settings do not bump the data version). "Elegir foto" from the prototype is not implemented (no photo storage). Updates tab shows the version and points to Homebrew until S7 is decided. New dependency tauri-plugin-autostart (Risk item 5); QA never toggles it because it would register a LaunchAgent on the real Mac.
