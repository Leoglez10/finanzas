# Native Mac finance app (Actual Budget replacement)
Objective: native SwiftUI macOS app covering accounts, monthly envelope planning, upcoming payments, automations and a native MCP server; no charts. Branch: `feat/native-mac-app`.
Delivery: exception-ok (local repo, no remote/PRs yet). Test runner: `swift test --package-path FinanzasKit` (logic), `xcodebuild -scheme Finanzas build` (app).

## Specs
- S1 Accounts: "quiero que tenga lo de las carteras". On-budget / off-budget, closed flag, balance = sum of transactions.
- S2 Planning: "las planeaciones". Monthly envelope budget per category: assigned, spent, available, carryover to next month.
- S3 No charts: "las graficas no me sirven tanto". No dashboard/report screens.
- S4 MCP: "quiero que tenga mcp nativo para conectar a ia". Tools at least equivalent to the current actual-mcp (accounts, transactions CRUD, categories, budget month, set budget, spending by category, schedules, skip schedule) plus upcoming payments.
- S5 Upcoming payments: "lo de las planeaciones de proximos pagos". Recurring scheduled payments (frequency, interval, weekend handling), next 30 days list, projected balance per account, skip / post actions.
- S6 Automations: "automatizaciones". Auto-post due scheduled payments when enabled, notification N days before a payment, payee → category rule.
- S7 UI: "una interfaz mas nativa de ios y rapida y facil". SwiftUI, NavigationSplitView, keyboard shortcuts, quick add.
- S8 Platform: "arranca solo con la Mac". Assumption: data model stays CloudKit-compatible ("si hay sinc con icloud podria ser buenisimo") so iCloud + iPhone can be enabled later with a paid developer account: no unique constraints, all attributes defaulted or optional, optional relationships.
- S9 Assumption (proposed in plan, not objected): one-shot importer from Actual's exported `db.sqlite` (amounts are integer cents, dates are yyyymmdd integers).

## Tasks
- [x] T1 S1,S5,S8 — FinanzasKit package: integer-cent amounts, recurrence next-date logic, SwiftData models; xcodegen app shell builds. Route: inline. Evidence: RED (missing types) → GREEN 7/7 swift test; app BUILD SUCCEEDED, launches, store at ~/Library/Application Support/Finanzas. Commit: see Log L6.
- [ ] T2 S9 — Actual db.sqlite importer. Route: inline.
- [ ] T3 S1,S7 — Accounts + transactions UI with quick add. Route: inline.
- [ ] T4 S2 — Monthly envelope budget calc + view. Route: inline.
- [ ] T5 S5 — Upcoming payments: 30-day projection, skip/post. Route: inline.
- [ ] T6 S4 — MCP server. Open decision: transport (in-app localhost HTTP vs stdio helper sharing the store); verify swift-sdk support first. Route: inline.
- [ ] T7 S6 — Automations: auto-post, notifications, payee rules. Route: inline.

## Log
- L1 (2026-10-15) "utiliza rea para investigar como funciona mi app de actual, quiero hacerla pero nativa en mi mac, y no con todo lo que tiene, quiero que tenga lo de las carteras, las planeaciones, pero las graficas no me sirven tanto, quir¿ero que tenga mcp nativo para conectar a ia, quiero que tenga lo de las planeaciones de proximos pagos, automatizaciones, una interfaz mas nativa de ios y rapida y facil, puedes investigarla y proponerme un plan nuevo de crear la app?"
- L2 (2026-10-15) "si hay sinc con icloud podria ser buenisimo, porque si tabien la podemos hacer app seria buenisimo para cel, se puede hacer app de cel tambien?"
- L3 (2026-10-15) "no tengo cuenta, arranca solo con la Mac"
- L4 REA evidence (ev_d62f79b2…): Actual budget uses 4 open accounts (3 on-budget, 1 off), 92 txns, no splits/transfers, 26 categories / 9 groups, envelope budgets, 4 monthly schedules (not auto-posting), 0 custom rules, dateFormat dd/MM/yyyy.
- L5 Routes: all inline — no parallel units, no writer trigger reason.
