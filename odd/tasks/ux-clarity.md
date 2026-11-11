Objective: make the app understandable for anyone, not only envelope-budgeting experts.
Branch: feat/native-mac-app · Delivery: ask-on-risk · Runner: `swift test --package-path FinanzasKit`, `xcodebuild … build`, `scripts/check-design.sh`

## Specs

- S1 Income without category: "cuando eliges 'Ingreso' en el formulario, que la categoría por defecto sea la de ingresos" and warn about uncategorized income ("marcar los movimientos 'Sin categoría' con un aviso para clasificarlos").
- S2 "Disponible para asignar" explains the next action, e.g. "Asignaste $500 más de lo que tienes. Registra un ingreso o baja alguna categoría" / "Tienes $X sin destino".
- S3 First-run guide on the Overview: "crear cuenta → registrar tu ingreso → asignarlo en el presupuesto", steps checked off as done.
- S4 Compact headings: smaller title, no decorative icon tile.
- S5 Ledger: no empty striped rows; hide "Cuenta" column with a single open account; "Notas" narrower than "Comercio".
- S6 Budget: per-category progress bar (spent vs. funds); the "+100 / -50" shortcut visible, not only in the tooltip.
- S7 Copy fixes: "1 cuenta abierta" plural; "100.0 %" must not wrap; long category names stay on one line in Overview cards.

## Tasks

- [x] T1 S1 inline — 013e797 (RED/GREEN: uncategorizedIncome test)
- [x] T2 S2 inline — 6c86ddb (RED/GREEN: toBudgetHint test)
- [x] T3 S3 inline — 436b5f4 (UI only; checked in design render)
- [x] T4 S4+S7 inline — 70968a6 (UI only; checked in design render)
- [x] T5 S5 inline — bf06d74 (UI only; checked at 760 px)
- [x] T6 S6 inline — 2a02460 (RED/GREEN: Row.spentShare test)

## Log

- L1 2026-10-16 user: "has el to do que cumplas todos los puntos" (points 1–7 from the UI review).
- L2 2026-10-16 verification: 66 kit tests pass; app builds; check-design PASS (20 renders). Not exercised by hand in the running app. Progress bars render gray in the offscreen probe (inactive window tint), same as the existing Overview bars.
