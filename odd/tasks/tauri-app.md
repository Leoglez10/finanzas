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
- T1 [S1,S2,S4] inline — Shell: restore `desktop/` base, design tokens + themes, sidebar, routing, Resumen with real data. Status: in progress.
- T2 [S1,S5] inline — Movimientos/cuenta: grouped ledger, search/filters, add/edit dialog, inline edit, delete with undo. Status: pending.
- T3 [S1] inline — Presupuesto. Status: pending.
- T4 [S1,S5] inline — Próximos pagos (register/skip/schedules) + reminders as native notifications + tray. Status: pending.
- T5 [S1] inline — Deseos (reserve, release, buy, archive, two-step editor). Status: pending.
- T6 [S3,S4] inline — Ajustes sub-tabs: Perfil (profiles), General (theme/mode/tray/autostart), Avisos, Datos, Actualizaciones, Atajos; persisted settings. Status: pending.
- T7 [S5] inline — ⌘K palette, native menu + shortcuts, quick-capture window + global shortcut. Status: pending.
- T8 [S5] inline — CSV drag-and-drop import. Status: pending.
- T9 [S6,S7] — Packaging/release/cask switch, retire Rune app (needs user decision). Status: pending.

## Log
L1. 2026-11-18 user request verbatim: "ya empieza la implementacion".
L2. Prior art: Tauri 2 + Svelte app removed in e652984 ("remove Tauri desktop app in favor of the Rune app"); restored `desktop/` from e652984^ as the base for the Rust command layer, icons and config. Frontend components are rewritten to the new design.
L3. Route: inline; no writer reason (units are sequential and depend on the shell). Risk: item 5 (new JS/Rust dependencies: Tauri plugins, Svelte toolchain) — independent verifier at feature end.
