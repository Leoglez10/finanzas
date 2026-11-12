# Dashboard redesign (Rust/Rune app)
Objective: adopt the visual language of the user's reference dashboard in the Rust/Rune app, in Spanish, using only existing data. Branch: `feat/dashboard-redesign` from `origin/main`.
Delivery: work-unit commits on the feature branch; no push/PR unless asked. Checks: `cargo test -p finanzas-app` (scratch `FINANZAS_DB`), `--scene` snapshots light/dark.

## Specs
S1. "podemos usar esta ui para esto? tomar lo mejor de eso y los diseños pero en español." Target: "en la de rust, la de main."
S2. Agreed scope (assistant proposal, accepted by S1 target answer): projected-balance hero with large amount, small cents and stacked bar (ingresos / gastado / por pagar / sobra); "Pagos por hacer" card with day chips and pending list from scheduled payments; month header with arrows and a primary "+ Agregar" button (gasto / ingreso); airy card grid; progress bars with a reference marker. All copy in Spanish.
S3. Exclusions (assumption, from proposal): no couple-split or credit-card invoice features, no third-party bank branding, no licensed 3D art — flat in-app SVG icons instead.
S4. Presentation only: existing calculations, persistence, guards and actions unchanged.

## Tasks
- T1 [S1–S4] inline: shared type scale, big-money helper, SVG icons; redesigned Resumen (header, saldo previsto, pagos por hacer, categorías, gráfico, presupuesto, recientes); upcoming data loaded on Resumen; sidebar with icons and sections. Status: done (commit below).

## Log
L1. User request verbatim: "podemos usar esta ui para esto? tomar lo mejor de eso y los diseños pero en español. estya bien no?" (reference image: "Duo Finn" Portuguese couple-finance dashboard).
L2. User answer verbatim: "en la de rust, la de main."
L3. Route: inline, no writer trigger (one coherent presentation change). Rune exposes `TextStyle::new(size, weight)` for large figures and SVG `Icon { name, view, body }` for icons. Upcoming payments currently load only on the Próximos route; Resumen list will be read-only with a link to Próximos so schedule action guards stay untouched.
L4. Theme toggle dropped: `sheet.rs` uses `s.dark.is_some()` to pin the scene date, so a runtime toggle would change new-entry default dates. System appearance still applies.
L5. Checks: `cargo test --workspace` with scratch `FINANZAS_DB` all pass (67 app tests incl. 3 new forecast/split tests); clippy clean; `git diff --check` clean. Snapshots demo-light, demo-dark (1440x1000/1500) and empty (1200x800) inspected: no overlap; dark contrast fixed for primary button and wallet art. Test `previous_month_changes_the_displayed_header` kept unchanged by rendering the title as one keyed text. Resumen payments list is read-only; "Ver próximos pagos" links to the actionable screen. Not verified: live window interaction and narrow widths below 1200.
