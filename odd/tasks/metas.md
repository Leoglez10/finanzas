Objective: turn Deseos into goals that fit trips and experiences, not only purchases.
Branch: feat/period-filters-dashboard (continues) · Delivery: ask-on-risk · Runner: `cargo test -p finanzas-core -p finanzas-mcp`; `pnpm --dir desktop check` + `pnpm --dir desktop test`; QA `desktop/qa/deseos.js`

## Specs

- S1 Goal type: "Tipo de meta: Compra, Viaje, Experiencia o Fondo (ahorro o emergencias). Cada tipo tendría su icono y sus textos de ejemplo." Existing wishes become Compra.
- S2 Target date and suggested contribution: "Cancún, 15 jun 2027: te faltan $8,400 en 7 meses → aparta $1,200 al mes". Optional date; a goal past its date shows as overdue ("vas atrasado").
- S3 Line items (mainly trips): "vuelo, hotel, comida, actividades. El precio total sería la suma, y cada partida se marca como pagada."
- S4 Contribution history: "cada vez que apartas dinero queda registrado con su fecha, y se ve una barra o gráfica de avance."
- S5 Completing: "Marcar como cumplida registra el gasto (o los gastos de cada partida) y la archiva como logro."
- S6 During the trip: "etiquetar movimientos con la meta para comparar lo gastado contra lo planeado por partida."
- Assumption: the MCP save_wish tool gains optional kind/targetDate with its existing replace semantics (omitted = default).

## Tasks

- T1 [S1,S2] inline — schema v3 (kind, target_date), core plan (months left, monthly amount, overdue), MCP fields, dialog + Deseos UI. Risk: item 1 (schema migration). Status: done (core 95+12, MCP 26, front 30, QA deseos.js 20/20 PASS; independent verifier: S1 pass, S2 pass, no blockers).
- T2 [S3] inline — line items table (schema v4), total = sum, paid flag; paid counts as progress and releases what it covers. Risk: item 1. Status: done (core 95+15, MCP 27, QA deseos.js 25/25).
- T3 [S4] inline — contribution log (schema v5) on reserve and on paid-item releases, history chart on cards. Risk: item 1. Status: done (core 95+16, MCP 28, front 31, QA deseos.js 27/27).
- T4 [S5] inline — complete_wish in core (schema v6 achieved_date): one expense per unpaid item or one for the price, archived as achievement; UI Cumplida + Logro chip, undo. Risk: item 1. Status: done (core 95+19, MCP 28, QA deseos.js 30/30).
- T5 [S6] inline — tag movements with a goal; spent vs planned per item. Status: pending.

## Log

- L1 (2026-11-22) User: "la parte de los deseos no obligatoreio es un obketo, tambien puede ser un viaje, entonces como mejoramos esa seccion? que mas agregarias?" → proposal of 6 items; user: "hagamos esos, empieza por pasos".
- L2 Route: all tasks inline (one writer, sequential, no parallel units). Order T1→T5 follows the proposal's value ranking.
- L3 (2026-11-22) User goal: "tu /goal es terminar hasta todas las fases, y cuando acabes y testes que funcuiona, todo loq ue hay pendiente lo sibes a gh. con pr y cuando acabe todo subes la nueva version y verificas que funcione todo. cuando avabe eso mejoras la pagina de gh pages y el readme. ese es el goal"
- L4 T1 verifier notes kept as follow-ups: an older app refuses a v3 file with "not a Finanzas database" (data safe, message misleading); end-of-month plan edge (Jan 31 → Feb 28 = 1 month) accepted; archived goals now get no plan (fixed with test).
