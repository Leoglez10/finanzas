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
- S10 Language: "toda la app en español". All user-facing UI text in Spanish; development region `es`. Code identifiers stay English.
- S9 Assumption (proposed in plan, not objected): one-shot importer from Actual's exported `db.sqlite` (amounts are integer cents, dates are yyyymmdd integers).

## Tasks
- [x] T1 S1,S5,S8 — FinanzasKit package: integer-cent amounts, recurrence next-date logic, SwiftData models; xcodegen app shell builds. Route: inline. Evidence: RED (missing types) → GREEN 7/7 swift test; app BUILD SUCCEEDED, launches, store at ~/Library/Application Support/Finanzas. Commit: see Log L6.
- [x] T2 S9 — Actual db.sqlite importer + menu "Importar desde Actual…" (sqlite or zip/blob). Route: inline. Evidence: RED (missing ActualImporter) → GREEN 11/11; real db import 4 accounts/15 categories/5 payees/92 entries/4 schedules = Actual view counts; app BUILD SUCCEEDED. Budget amounts (zero_budgets) deferred to T4.
- [x] T3 S1,S7,S10 — Accounts sidebar with balances, entries Table (search, delete, context menu), quick add sheet ⌘N. Route: inline. Evidence: RED (Money/findOrCreate missing) → GREEN 14/14; app BUILD SUCCEEDED; screenshot with real imported data verified. Quick-add sheet not exercised by automation.
- [x] T4 S2 — Envelope budget: BudgetAmount model (additive schema change), EnvelopeBudget calc with Actual rules, budget import (full + importBudgets by name), BudgetView with month nav (⌘←/⌘→) and inline assign. Route: inline; HIGH RISK item 1 (persisted schema change) → independent verifier. Evidence: RED → GREEN 25/25; real store migrated with counts preserved (4/15/95/3) and 16 budgets imported (= Actual SQL count); BUILD SUCCEEDED; screenshot verified. Oct 2026 toBudget -$139.01 not cross-checked against Actual UI (server down).
- [ ] T5 S5 — Upcoming payments: 30-day projection, skip/post. Route: inline.
- [ ] T6 S4 — MCP server. Open decision: transport (in-app localhost HTTP vs stdio helper sharing the store); verify swift-sdk support first. Route: inline.
- [ ] T7 S6 — Automations: auto-post, notifications, payee rules. Route: inline.

## Log
- L1 (2026-10-15) "utiliza rea para investigar como funciona mi app de actual, quiero hacerla pero nativa en mi mac, y no con todo lo que tiene, quiero que tenga lo de las carteras, las planeaciones, pero las graficas no me sirven tanto, quir¿ero que tenga mcp nativo para conectar a ia, quiero que tenga lo de las planeaciones de proximos pagos, automatizaciones, una interfaz mas nativa de ios y rapida y facil, puedes investigarla y proponerme un plan nuevo de crear la app?"
- L2 (2026-10-15) "si hay sinc con icloud podria ser buenisimo, porque si tabien la podemos hacer app seria buenisimo para cel, se puede hacer app de cel tambien?"
- L3 (2026-10-15) "no tengo cuenta, arranca solo con la Mac"
- L4 REA evidence (ev_d62f79b2…): Actual budget uses 4 open accounts (3 on-budget, 1 off), 92 txns, no splits/transfers, 26 categories / 9 groups, envelope budgets, 4 monthly schedules (not auto-posting), 0 custom rules, dateFormat dd/MM/yyyy.
- L5 Routes: all inline — no parallel units, no writer trigger reason.
- L6 T1 committed as ddc85f9.
- L7 (2026-10-15) "toda la app en español" → added S10; T1 shell labels translated and development region set to es.
- L8 T1 follow-up commit cebf927 (Spanish UI). RDD assess: risk medium, review_due false (under_budget). Next: T2 Actual importer.
- L9 T2 committed as 216b695. RDD assess: "risk": "medium" "review_due_reason": "under_budget" 
- L10 (2026-10-15) "importa lo de mi app y despues continua" → imported ~/.actual-mcp cache (2026-09-29, fresher than Aug blob) into app store via scratchpad importer: 4 accounts / 15 categories / 5 payees / 95 entries / 3 schedules. All balances are $0 in Actual too (income == spending per account). Note: Category name clashes with ObjC Category → typealias in app target; do not rename the model (store already has data).
- L11 T3 committed as 7b12066. RDD assess: "risk": "medium" "review_due": false "review_due_reason": "under_budget" 
- L12 (2026-10-15) "si" (proceed with T4). Real store backed up to scratchpad backup-pre-t4 before migration.
- L13 T4 verification: independent verifier PASS S2/S8/S10, 0 blockers (36/36 incl. 11 probes; old-schema on-disk store migrated with all rows). Parent spot check 25/25. RDD: assess high (escalated item 1) → user granted review → lenses risk+reliability → approved, acknowledged (gentle-ai.review-acknowledged/v1, lineage review-27e71edf2818abd3). Advisories (not fixed): importBudgets keeps first match on duplicate group/category names (user data has 0 duplicates); rows without category_mapping skipped silently; Actual "hold for next month" not modelled; BudgetView assign uses try? + autosave. Next: T5 upcoming payments.
