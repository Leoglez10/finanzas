# Wishes with reserved funds
Objective: save desired purchases, reserve envelope funds, and explain affordability in the app and MCP.
Branch: feat/wishlist-reservations; base: 318b24a3bb04788e65bc18620d41173b64831487; delivery: auto-chain / feature-branch-chain (local work units only); runners: cargo test --workspace; cargo clippy --workspace --all-targets -- -D warnings.

## Specs
S1. "Goals, o cosas que quiero cmprar"; "por ejemplo una PlayStation"; "poner el precio, poner un poco el link, poner más detalles de lo que quiero comprar". Check: create/list/edit/archive wishes with price, URL, notes and optional budget category; app entry point and MCP tools.
S2. "que la application intelligentemente diga que te alcanza, no te alcanza, va a dentro de tus presupuestos, no sabes tus presupuestos, te hace falta tanto". Check: shared deterministic evaluation reports category fit, shortfall, liquidity and upcoming recorded commitments, with unknown states when evidence is missing.
S3. "modificar el MCP for that controller to the list of desers, all the list of things that you can see." Check: profile-aware MCP list/save/archive/reserve/evaluate calls use the shared core.
S4. "puede ser apartar d euna vez". Check: partial reservation/release, no double allocation, validated amounts, atomic cross-connection enforcement and persistent migration.

## Tasks
T1 | S1,S2,S4 | inline | done | Core wishes, schema migration, evaluation and atomic reservations; cargo test -p finanzas-core and clippy passed; commit pending.
T2 | S1-S4 | inline | in_progress | MCP tools and docs; public-interface tests; commit pending.
T3 | S1,S2,S4 | inline | pending | App navigation, forms, list/evaluation/reservation interactions; commit pending.
T4 | S1-S4 | independent gentle-ai-verify | pending | Full suite, clippy, visual/interaction evidence, independent verification and due native reviews.

## Log
L1. Original request verbatim: oye ayudame a pensar en una nueva impolementacion, imaginate que quiero comrar algo, quisiera poner en la app asi como Goals, o cosas que quiero cmprar, por Si me quiero comprar por ejemplo una PlayStation que existe dentro de la placa o un botón o no sé cómo ir con un objeto a mi lista de compras y poner el precio, poner un poco el link, poner más detalles de lo que quiero comprar, que la application intelligentemente diga que te alcanza, no te alcanza, va a dentro de tus presupuestos, no sabes tus presupuestos, te hace falta tanto, modificar el MCP for that controller to the list of desers, all the list of things that you can see.
L2. Implementation approval: "si. hazlo". Scope clarification: "puede ser apartar d euna vez".
L3. Design assumptions: reservations claim existing expense-category funds, do not move accounts or create expenses, and persist across months. Partial reservations allowed. Evaluation uses current month and 30-day recorded-payment horizon; open on-budget accounts only; future income is not counted as cash. Existing overspending is reported, never silently repaired. Archived wishes release reservations. Purchase recording remains the existing transaction workflow: release/archive explicitly rather than inventing or duplicating a transaction.
L5. T1 RED: missing wish APIs (exit 101). GREEN: core suite and clippy passed (job-4). Two fixture errors corrected: schedules require recurrence; uncategorized deposits do not fund the envelope engine. Migration preserves existing ledger and reopens wishes; concurrent writers test admits exactly one claim. MCP RED: Unknown tool: save_wish (exit 101).
L4. Forecast ~1000 authored changed lines across three coherent local work units (core, MCP, UI); split by behavior, not line-count golf. No publishing. Preserve unrelated untracked odd/tasks/app-wide-visual-refresh.md. Risk: item 1 (persistent schema migration) and item 4 (app/MCP reservation concurrency); independent verifier required. Core rollback boundary: new wish table/services only; no existing transaction data rewritten.
