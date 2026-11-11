# PORTING SPEC — Finanzas (Swift/SwiftData) → Rust (Tauri 2 + rusqlite + rmcp)

Evidence paths relative to repo root. Swift 6.0, macOS 15 min (`FinanzasKit/Package.swift:5`).

---

## 1. Data model (`FinanzasKit/Sources/FinanzasKit/Models.swift`)

Design constraint stated at `Models.swift:4-6`: CloudKit-compatible — every attribute has a default, every relationship optional with explicit inverse, **no unique constraints**. Money is **integer cents (`Int`) everywhere**; never Decimal in storage.

All 7 `@Model` types, in `FinanzasStore.models` order (`Models.swift:134-136`): `Account, CategoryGroup, Category, Payee, LedgerEntry, Schedule, BudgetAmount`.

**Account** (`Models.swift:8-24`)
- `name: String = ""`, `offBudget: Bool = false`, `closed: Bool = false`, `sortOrder: Double = 0`
- `entries: [LedgerEntry]?` cascade, inverse `\LedgerEntry.account`; `schedules: [Schedule]?` nullify
- `balance: Int` computed = `entries.reduce(0) { $0 + $1.amount }` (`Models.swift:23`) — not stored.

**CategoryGroup** (`:26-37`): `name: String = ""`, `isIncome: Bool = false`, `sortOrder: Double = 0`, `categories: [Category]?` cascade.

**Category** (`:39-54`): `name`, `hidden: Bool = false`, `sortOrder: Double = 0`, `group: CategoryGroup?`, `entries` nullify, `schedules` nullify, `budgetAmounts: [BudgetAmount]?` cascade.

**Payee** (`:56-62`): `name: String = ""`, `entries`/`schedules` nullify.

**LedgerEntry** (`:64-85`): `date: Date = .distantPast`, `amount: Int = 0`, `notes: String = ""`, `cleared: Bool = false`, `account: Account?`, `category: Category?`, `payee: Payee?`, `schedule: Schedule?` (link to source schedule). No transfer field, no split field.

**Schedule** (`:87-117`): `name`, `amount: Int`, `autoPost: Bool = false`, `completed: Bool = false`, `nextDate: Date?` (nil ⇒ derive from recurrence start), `recurrenceData: Data = Data()` (JSON blob), `account/category/payee: ...?`, `entries: [LedgerEntry]?` nullify. `recurrence: Recurrence?` getter/setter JSON-encodes into `recurrenceData` (`:109-113`).

**BudgetAmount** (`:119-131`): `month: Int = 0` (`yyyymm`), `amount: Int = 0`, `carryover: Bool = false`, `category: Category?`. Uniqueness per (month, category) is **convention only**, enforced by upsert in `BudgetAmount.set` (`BudgetStore.swift:7-14`); loader even sums duplicates (`BudgetStore.swift:23-27`).

**Dates/months**: dates are `Date` at day precision (all writers use `Calendar.current.startOfDay`, e.g. `EntriesView.swift:283`). Months are integers `yyyy*100+mm` (`MonthKey`, `EnvelopeBudget.swift:4-14`); `next/previous` roll the year; `firstDay(m)` builds a Date. No timezone-stored month; derived via `Calendar.current` (system locale/timezone).

**Recurrence** (`Recurrence.swift:4-20`) — `Codable` struct: `start: Date`, `frequency: Frequency` (`enum: String, Codable, CaseIterable` raw `daily|weekly|monthly|yearly`), `interval: Int` (clamped `max(1, interval)`), `skipWeekend: Bool`, `weekendMode: WeekendMode` raw `before|after` (init default `.after`), `endDate: Date?`.

Rust mapping: 7 tables; `recurrence` as a TEXT JSON column (schema evolution rationale at `Models.swift:108`), keep same field names/raw values; `sortOrder REAL`; amounts `INTEGER` cents; dates as `INTEGER` unix seconds at local midnight or TEXT `YYYY-MM-DD` (must round-trip `YYYY-MM-DD` for MCP, see §4).

---

## 2. Store location, sharing, ExternalChanges

- `FinanzasStore.storeURL` = `ProcessInfo.environment["FINANZAS_STORE"]` if set, else `URL.applicationSupportDirectory + "Finanzas/Finanzas.store"` (`Models.swift:139-143`) → `~/Library/Application Support/Finanzas/Finanzas.store` (README.md:57-58). Explicit name chosen because unsandboxed apps otherwise collide on `default.store` (`Models.swift:138-140`).
- `onDisk(at:)` creates the parent dir, opens `ModelContainer` with `cloudKitDatabase: .none` (`:146-150`); `inMemory()` for tests (`:152-154`).
- Two processes share one file: app (`Finanzas/FinanzasApp.swift:20`) and the bundled stdio helper `finanzas-mcp` (`FinanzasMCP/main.swift:8`). MCP writes are **signed with author `"mcp"`** (`ExternalChanges.mcpAuthor`, `ExternalChanges.swift:25`; set per call `FinanzasMCP/main.swift:34`).
- `ExternalChanges` (`ExternalChanges.swift:24-49`): on init reads `context.fetchHistory(...)` and stores last token; `poll()` fetches transactions with `token > lastToken`, returns true if any has `author == "mcp"`.
- App refresh loop (`FinanzasApp.swift:12,32,55-64`): 3 s timer → `reloadIfMCPWrote()`; skips if any window has an attached sheet; calls `saveOrAlert()` first (MCP history must be consumed on the current context), then **opens a fresh container and swaps it**, then re-runs automations. Hourly timer runs automations (`:13,33`). `.task` at launch (`:34`) and `onChange(of: scheduleSignature)` (`:155,177-181`) also run automations.
- Undo: `context.undoManager = undoManager` per context identity (`:174`).
- Rust equivalent obligation: app must poll for external writes and hot-reload (the MCP is a separate process on the same SQLite file); a changelog/author column replaces SwiftData history. IDs must stay stable strings (§4).

---

## 3. Business rules

**Account balance** = sum of that account's entry amounts (`Models.swift:23`); off-budget accounts are included in balances but excluded from budget activity. Sidebar "Todas" = sum of open accounts (`FinanzasApp.swift:150`).

**Envelope math** (`EnvelopeBudget.swift:36-71`) — inputs: `categories: [(id, isIncome)]`, `budgeted: [Key: Int]`, `carryover: Set<Key>`, `activity: [Key: Int]` keyed by `Key{month, categoryID}`.
- `first = min(month of budgeted ∪ activity keys) ?? target`; iterate `m = min(first, target)` → `target`, stepping `MonthKey.next`.
- Per month: income categories → `income += activity`, `Row(budgeted: 0, activity, available: 0)`; non-income → `assigned += budgeted`; `available = carry[c] + budgeted + activity`; if `available >= 0 || carryover.contains(key)` then `carry[c] = available` else `carry[c] = 0; overspent -= available` (i.e. `overspent += -available`).
- `toBudget = income + prevToBudget − prevOverspent − assigned` (`:63`) — "Disponible para asignar / ready to assign".
- Leftover carries forward; overspending zeroes the category next month and is subtracted from next month's toBudget unless `carryover` is set. Initial `prevToBudget = prevOverspent = 0`.
- `Row.spentShare` (`:73-81`): `spent = max(-activity,0)`; if `available < 0` → 1; `funds = spent + available`; `funds > 0 ? spent/funds : 0`.
- Store projection (`BudgetStore.swift:17-46`): categories from all `Category`, `isIncome = group?.isIncome ?? false`; budgeted summed per key plus carryover set; activity sums **only** entries that have both `category` and `account` with `account.offBudget == false`.

**MonthSummary** (`MonthSummary.swift:5-25`): from `budget.month(m)`: `income = m.income`; `budgeted = m.budgeted`; `spending = rows where !isIncome && activity < 0` mapped to `-activity`; `expenses = sum(spending)` (refund cannot go negative; a net-positive category is dropped). `savingsRate = income > 0 ? (income − expenses)/income : nil`.
- `relativeChange(current, previous)` = `previous == 0 ? nil : (current − previous)/abs(previous)` (`:28-30`).
- `uncategorizedIncome(entries, month)` = sum of `amount > 0 && category == nil && account?.offBudget == false && MonthKey.of(date) == month` (`:33-37`).
- `toBudgetHint(toBudget)` (`:40-48`): `<0` → `"Asignaste \(Money.format(-toBudget)) más de lo que tienes. Registra un ingreso o baja alguna categoría."`; `0` → `"Todo tu dinero tiene destino."`; `>0` → `"Tienes \(Money.format(toBudget)) sin destino. Asígnalo a alguna categoría."`

**Recurrence** (`Recurrence.swift`)
- `occurrences(from:through:)` (`:26-42`): day precision; `scanLimit = through + 3 days` (weekend shift ≤2 d); iterate `n = 0,1,2…` while `raw <= scanLimit`; if `endDate` and `raw > startOfDay(endDate)` break; `date = adjusted(raw)`; keep if `from <= date <= through` and `result.last != date` (collapses weekend-shift duplicates).
- `next(onOrAfter:)` (`:44-48`) = `occurrences(from: date, through: date + 2 years).first` (linear scan; comment "ponytail" at `:45`).
- `rawOccurrence(n)` (`:50-61`): from `startOfDay(start)`, `steps = n*interval`; daily `+steps` days, weekly `+steps*7` days, monthly `+steps` months, yearly `+steps` years. Recomputed from `start` each n → **month-end clamps without drift** (Jan 31 → Feb 28 → Mar 31).
- `adjusted` (`:63-70`): if `skipWeekend`, step `+1` day (after) or `-1` day (before) while `isDateInWeekend`.
- Known edge (task log `odd/tasks/native-mac-finance-app.md:27` L15): `.before` on a weekend start can lose the first occurrence.

**Upcoming** (`Upcoming.swift`)
- `effectiveNextDate` = `nil` if `completed` or no recurrence, else `nextDate ?? r.next(onOrAfter: r.start)` (`:6-9`).
- `skip()` = `advance()`; `post(in:)` (`:15-23`) inserts `LedgerEntry(date: effectiveNextDate ?? startOfDay(now), amount, account, category, payee, schedule: self)`, then `applyPayeeRule()`, then `advance()`.
- `advance()` (`:25-30`): `dayAfter = current + 1 day`; `nextDate = r.next(onOrAfter: dayAfter)` else `completed = true`.
- `items(schedules:today:days: 30)` (`:42-62`): per schedule with `effectiveNextDate` + recurrence, `r.occurrences(from: next, through: today+days)`; flat-map, sort by `(date, schedule.name)`; running projected balance per account keyed by account id, seeded from `account.balance`, each item `projected = running + schedule.amount` (signed); `overdue = date < today`; `projectedBalance = nil` if no account.

**Automations** (`Automations.swift`, `Finanzas/Automation.swift`)
- `Automations.postDue(schedules, today, in:)` (`:7-19`): for `s.autoPost && !s.completed`, loop `while count < 500 && effectiveNextDate <= startOfDay(today)` posting each. Cap guards runaway daily rules.
- Runs: app launch (`FinanzasApp.swift:34`), hourly (`:33`), after MCP reload (`:63`), on schedule signature change (`:155`), and on Ajustes toggles (`Automation.swift:82-83`).
- `Reminders.plan(items, daysBefore, hour, now)` (`:33-49`): `fire = setHour(hour:00)` on `item.date − daysBefore`, skipped if `fire <= now`; `title = "Pago próximo: <name>"`; `body = "<money> el dd.MM.yyyy (<account>) · se registrará automáticamente"`; `id = "pago-" + ModelID + "-YYYYMMDD"`; sorted by fireDate. App uses `Upcoming.items(days: 60)`, hour 9, `daysBefore` default 2, enabled default true (`Automation.swift:10-13,28-40`); cancels all pending `pago-` notifications then re-adds `UNCalendarNotificationTrigger` one-shots.
- Payee→category learning: `Payee.suggestedCategory` = category of the newest entry (by date) that has a category (`:52-55`); `LedgerEntry.applyPayeeRule()` fills `category` only when nil (`:58-62`). Applied in quick add (`EntriesView.swift:291`), MCP `add_transaction` (`FinanzasTools.swift:147`), and schedule `post` (`Upcoming.swift:20`).

**Entry input parsing** (`EntryInput.swift:5-30`)
- `Money.cents(from:)`: strip characters `space`, `$`, `,`, `-`; require full match `\d+(\.\d+)?`; parse `Decimal` with `en_US_POSIX`; `*100`; `NSDecimalRound(.plain)`; → `Int`. Sign is **never** taken from text. Pinned: `"125.50"→12550`, `"$1,250.5"→125050`, `" 80 "→8000`, `"0.07"→7`, `"-40"→4000`, `"12.345"→1235` (`EntryInputTests.swift:6-15`).
- `Money.budgeted(from:current:)`: trim; strip `+`; parse cents; first char `"+"` → `current+cents`; `"-"` → `max(0, current−cents)`; otherwise replace with `cents`. Pinned: `("500",20000)→50000`, `("+100",20000)→30000`, `("-50",20000)→15000`, `("- 300",20000)→0`, `("+$1,000.50",0)→100050`, `"+abc"→nil` (`EntryInputTests.swift:45-54`).
- `Money.format(cents, currency:"MXN")` = `(Decimal(cents)/100)` formatted `.currency(code:)` `es_MX`.
- Payee matching: trim; case- **and diacritic-insensitive** compare; `findOrCreate` inserts when missing; empty → nil (`EntryInput.swift:33-60`).
- **No transfers**: no model/field for them; the importer drops Actual transfer payees (`ActualImporter.swift:57`).

---

## 4. MCP server

`FinanzasMCP/main.swift`: `Server(name:"finanzas", version:"1.0.0", capabilities: tools.listChanged=false)`, **stdio** transport (`:47-48`). Each `CallTool` runs on MainActor, creates a **fresh `ModelContext`** with `author = "mcp"`, calls `FinanzasTools.call`, serializes with `JSONSerialization` `.prettyPrinted, .sortedKeys, .fragmentsAllowed, .withoutEscapingSlashes`, returns one text content block; failures → `"Error: <localizedDescription>"` with `isError: true`. Tool annotations map to `readOnlyHint`/`destructiveHint`. Writes are `save()`d before returning.

IDs: `ModelID.string` = `"<Entity>/<primaryKey>"` e.g. `Account/p3`, extracted from `PersistentIdentifier`'s Codable JSON (`ExternalChanges.swift:5-14`); lookup is a linear scan of one entity (`:16-23`). Clients rely on this shape.

Tools (12, `FinanzasTools.swift:27-105`; amounts are **decimal currency units** in the API, 2-decimal max, negative = outflow; dates `YYYY-MM-DD`; months `YYYY-MM`):

| Tool | Params (required*) | Output |
|---|---|---|
| `list_accounts` | — | array of `{id,name,offBudget,closed,balance}` sorted by `sortOrder` |
| `get_transactions` | `startDate*`, `endDate*`, `accountId?`, `categoryId?`, `limit?` (1–1000, default 100) | `{total, truncated, transactions:[{id,date,amount,accountId,account,payee,categoryId,category,notes,cleared,scheduleId}]}` newest first |
| `list_categories` | — | `[{id,name,isIncome,categories:[{id,name,hidden}]}]` groups by sortOrder |
| `get_budget_month` | `month*` | `{month:"YYYY-MM",toBudget,income,budgeted,groups:[{id,name,isIncome,categories:[{id,name,budgeted,spent,available}]}]}` (`spent` = raw signed activity) |
| `spending_by_category` | `startDate*`, `endDate*`, `includeOffBudget?` (default false) | `{startDate,endDate,total,categories:[{categoryId,category,spent}]}`, outflows only, largest first, uncategorized → `categoryId: null`, `category:"Sin categoría"` |
| `add_transaction` | `accountId*`, `date*`, `amount*`, `payee?`, `categoryId?`, `notes?`, `cleared?` (default true) | transaction JSON; payee reused/created; missing category filled by payee rule |
| `update_transaction` | `id*`, plus any of `date,amount,payee,categoryId,notes,cleared` (≥1 required) | transaction JSON; `notes:""` clears; payee non-empty else error |
| `delete_transaction` | `id*` | `{deleted:<transaction JSON>}`; destructive |
| `set_budget_amount` | `month*`, `categoryId*`, `amount*` (≥0) | `{month,categoryId,category,budgeted}`; rejects income categories |
| `list_schedules` | — | array of `{id,name,amount,payee,accountId,account,frequency,interval,nextDate,completed,autoPost}` sorted by name |
| `skip_schedule` | `id*`, `through?` | `{id,name,previousNextDate,nextDate,completed,skipped}`; max 500 skips/call; completed → error |
| `get_upcoming` | `days?` (0–365, default 30) | array of `{scheduleId,name,date,amount,accountId,account,overdue,projectedBalance}` |

Arg validation (`FinanzasTools.swift:326-421`): JSON `0/1` must not be read as booleans (`isBoolean` uses `CFBooleanGetTypeID`, `:331-334`); `int` validates bounds on `Double` (`:355-362`); `date` requires regex + round-trip equality (`:364-370`); `range()` requires `start <= end`; `month` requires `YYYY-(01..12)`; `cents` parses shortest decimal text, rejects `|value| > 1_000_000_000_000`, and **rejects >2 decimals** (`:393-411`); unknown tool → `ToolError("Unknown tool: …")`; bad id → `"<key>: not found"`.

---

## 5. ActualImporter (`ActualImporter.swift`)

Input: Actual Budget `db.sqlite` directly, or a **zip / server `.blob`** containing `db.sqlite` (app layer: sniff `"SQLite format 3"` header, else `/usr/bin/ditto -x -k` into temp dir and expect `db.sqlite`, `Finanzas/ImportFromActual.swift:47-63`). Read-only `sqlite3_open_v2(SQLITE_OPEN_READONLY)` (`:296-303`). Only into an empty store (`Account` and `LedgerEntry` counts 0, else `storeNotEmpty`) (`:21-23`).

Table→model mapping (all `WHERE IFNULL(tombstone,0)=0`):
- `accounts(id,name,offbudget,closed,sort_order)` → Account (`:29-33`)
- `category_groups(id,name,is_income,sort_order)` → CategoryGroup (`:35-39`)
- `categories(id,name,hidden,cat_group,sort_order)` → Category, group by map (`:41-45`)
- `payees(id,name)` where `transfer_acct IS NULL` → Payee (`:48-53`); `payee_mapping(id,targetId)` merges (`:54-55`)
- `schedules` join `rules` on `s.rule`, left join `schedules_next_date`: `nextDateSQL` = `CASE WHEN nd.local_next_date_ts = nd.base_next_date_ts THEN nd.local_next_date ELSE nd.base_next_date END` (`:59-67,150-151`); `posts_transaction`→`autoPost`, `completed`→`completed`, name falls back to payee name or `"Sin nombre"`; `dates` are `yyyymmdd` ints → Date via components (`:216-218`).
- `rules.conditions` JSON → `ScheduleRule` (`:232-262`): `field:"description"`→payeeID, `"acct"`→accountID, `"amount"` (Int, or `{num1,num2}` → midpoint), `"date"` (ISO string → one-off monthly with `start = endDate`; or object `{start,frequency,interval,skipWeekend,weekendSolveMode,endMode,endDate,endOccurrences}`; `endMode:"after_n_occurrences"` computes last raw occurrence counting un-shifted dates). Actual `patterns` ignored.
- `transactions`: `t.acct, cm.transferId (category via category_mapping), amount, pm.targetId (payee via payee_mapping), notes, date, cleared, schedule`; excludes `isParent=1`, tombstoned, null date/account; children kept only if parent not tombstoned (`:103-116`). Amounts are already integer cents.
- `zero_budgets(month, category via category_mapping, amount, carryover)` → BudgetAmount, rows merged and **summed** per `month|category`, skipping `amount==0 && carryover==0` and unknown categories (`:194-215`).
- Also: `importBudgets(at:into:)` matches categories by `"groupName/categoryName"` and refuses if any BudgetAmount exists (`:120-148`); `importScheduleDates(at:into:)` matches schedules **by name**, first-wins (`:154-172`). Summary counts accounts/categories/payees/entries/schedules/budgets. Failure → `context.rollback()`.

---

## 6. UI screens

Root `NavigationSplitView` (`FinanzasApp.swift:90-176`): sidebar header `chart.pie.fill` tile on `FinanceTheme.hero`, title "Finanzas" / "Tu dinero, en orden"; items **Resumen**, **Presupuesto**, **Próximos pagos**, section "Cuentas" with "Todas" (`tray.full`, balance = sum open), each open account (`chart.line.uptrend.xyaxis` if off-budget else `creditcard`) with balance right-aligned (coral if <0), "Nueva cuenta" button (alert with Nombre/Crear/Cancelar; new `sortOrder = max+1`), section "Cerradas" with `archivebox`. Detail routes: `.overview/.budget/.upcoming/.allAccounts/.account(id)`.

**Resumen** (`OverviewView.swift`)
- `@Query` all entries (date desc), accounts, categories, budgetAmounts (`:20-24`); computes `EnvelopeBudget.load` + `MonthSummary` for current and previous month in `body` (`:29-32`).
- First-run guide "Primeros pasos" with 3 checks: "Crea una cuenta", "Registra tu ingreso" (any entry `amount>0` whose `category.group.isIncome`), "Asígnalo en el presupuesto" (any `BudgetAmount.amount > 0`) (`:63-71`).
- 4 stat cards: "Saldo total" (prominent, `hero` fill, note "N cuenta(s) abierta(s)"), "Ingresos del mes", "Gastos del mes", "Tasa de ahorro" (`—` + "Sin ingresos este mes" when nil) with `relativeChange` badges "… vs mes anterior" (`:104-124`).
- Chart "Ingresos vs gastos" (Swift Charts `BarMark`, income emerald / expenses coral, 6M/12M segmented picker, footer "Flujo neto promedio:") (`:130-172`); note the product spec said "no charts" (`odd/tasks/native-mac-finance-app.md:12` S3) but a dashboard chart now exists.
- "Gasto por categoría": total + top 6 with percent and ProgressView (`:176-206`); "Movimientos recientes": grid of first 8 entries columns Descripción/Categoría/Cuenta/Fecha/Monto, "Sin categoría" fallback (`:210-240`); "Presupuesto del mes": spent big, "de <assigned>", progress tinted petroleum/amber(≥0.9)/coral(>1), "… usado"/"Sin asignar", "… excedido"/"… restante", top-5 rows "<spent> / <budgeted>" (`:244-282`).
- Toolbar: "Mes anterior" ⌘←, "Hoy", "Mes siguiente" ⌘→ (`:52-59`). Month title = capitalized es_MX `"Octubre de 2026"` (`:9-14`).

**Cuentas / Movimientos** (`EntriesView.swift`)
- Header card: "Saldo de los movimientos" (or "Total de la búsqueda") = sum of *filtered* rows, "N movimientos", buttons "Agregar gasto" (coral capsule `expenseFill`, tooltip "Agregar gasto (⌘N)") and "Agregar ingreso" (`incomeFill`, ⇧⌘N) (`:33-60`). Toolbar "Nuevo gasto" ⌘N (`:62-66`).
- `Table` columns: Fecha (dd.MM.yyyy es_MX), Cuenta (only when `account == nil` and >1 open account), Comercio, Categoría ("Sin categoría" with warning icon + help "Sin categoría no cuenta en el presupuesto…"), Notas, Monto (coral/green by sign) (`:82-111`)). Context menu Editar/Eliminar, double-click edits, Delete key deletes, no alternating row background, empty state "Sin movimientos" ("Agrega uno con ⌘N o importa desde Actual en Archivo.") (`:112-130`).
- Search (`searchable`, prompt "Buscar comercio, categoría o nota") filters on payee/category/notes with `localizedStandardContains` (`:131-137`).
- New/Edit sheet "Nuevo movimiento" / "Editar movimiento": segmented Gasto/Ingreso (switching to income auto-picks first non-hidden income category; switching back clears it) (`:211-219`), amount field monospaced colored by type, Cuenta picker (open only), Comercio with `textInputSuggestions` (≤8 case/diacritic-insensitive matches), Categoría picker grouped by group with hidden filtered, "Sin categoría" option; picking a known payee pre-fills its suggested category (`:249-256`); Fecha DatePicker, Notas; Guardar enabled iff `Money.cents` parses and account chosen; saves `startOfDay(date)` and `amount = isExpense ? -cents : cents` (`:279-295`).

**Presupuesto** (`BudgetView.swift`): metrics "Disponible para asignar" (colored coral if negative, note = `toBudgetHint`), "Ingresos del mes", "Asignado"; amber banner when `uncategorizedIncome > 0` ("Tienes X en ingresos sin categoría. Dales una categoría de ingreso en Movimientos para poder repartirlos."); hint label "Escribe cuánto asignas a cada categoría, o usa +100 / -50 para sumar o restar." (`:25-51`). Expense groups first with header columns "Asignado"/"Gastado"/"Disponible" (110 pt each), per-row progress `spentShare`, editable assign field committing on submit/blur via `Money.budgeted` (never drops typed text) (`:229-290`). Income groups show only activity in emerald. Category context menu "Renombrar" / "Ocultar"|"Mostrar" (hide instead of delete), group context menu "Renombrar grupo". Toolbar: "Nuevo grupo", "Mostrar ocultas" toggle ("Muestra las categorías ocultas; clic derecho ▸ Mostrar para recuperarlas"), month nav ⌘←/⌘→/Hoy (`:139-153`). Hidden categories render at 0.5 opacity.

**Próximos pagos** (`UpcomingView.swift`): metrics "Vencidos" (count), "Próximos 30 días", "Pagos programados"; sections "Vencidos", "Próximos 30 días" ("Nada programado"), "Pagos programados" (all schedules; label `describe()` = "Diario/Cada N días/Semanal/… · próximo dd.MM.yyyy · automático", or "Terminado"; context menu "Eliminar"). Row shows date badge (amber when overdue) + "Vencido", name, "payee · account", amount, "Saldo: X" projected; "Registrar"/"Saltar" buttons only when `item.date == schedule.effectiveNextDate`; every mutation saves immediately ("Saved right away so a posted payment survives a quit") (`:60-103`). Toolbar "Nuevo pago programado" ⌘N. Sheet fields: Nombre, Gasto/Ingreso, Monto, Cuenta, Comercio, Categoría, Repetición (Diario/Semanal/Mensual/Anual, "Cada N" 1–52, "Primera fecha", toggle "Si cae en fin de semana, pasar al lunes" → `weekendMode: .after`), "Registrar automáticamente" with footer "Los pagos automáticos se registran al abrir la app y cada hora, sin confirmación adicional." (`:110-217`).

**Ajustes** (`Finanzas/Automation.swift:46-89`): `FinanceHeading("Ajustes","Tus recordatorios, a tu ritmo")`; Toggle "Avisarme antes de cada pago" (UserDefaults `avisosActivos`, default true); Stepper "Avisar N día(s) antes, a las 9:00" 0…14 (`diasAviso`, default 2, disabled when off); automation explainer text. Window 500×400 min.

**Import** (`Finanzas/ImportFromActual.swift`): File ▸ "Importar desde Actual…" replacing import/export group, disabled without a container; `NSOpenPanel` title "Selecciona el db.sqlite o el respaldo .zip de Actual"; result alerts "Importación completa" ("N cuentas, N categorías, N comercios, N movimientos y N pagos programados."), warning "Ya tienes datos", critical "No se pudo importar". App-level store failure UI: "No se pudo abrir la base de datos" + path + "Mostrar en Finder" (`FinanzasApp.swift:66-88`). Save failure alert: "No se pudo guardar el cambio" / "Puede que la IA haya modificado los mismos datos…" (`:194-208`).

**Palette** (`Finance/FinanceTheme.swift:5-18`, adaptive light/dark): petroleum `#155E75`/`#67C6DF`; emerald `#147D59`/`#6DDBAD`; coral `#BE493D`/`#FF9A8F`; amber `#986000`/`#F5C46B`; canvas `#F4F7FA`/`#151D27`; card `#FFFFFF`/`#1E2936`; border `#DEE7ED`/`#354453`; hero `#155E75` (fixed); fixed fills `expenseFill #BE493D`, `incomeFill #147D59`. Cards: 18 pt radius + 1 pt border; headings `.title.bold` + `.callout` subtitle.

---

## 7. Existing tests (`FinanzasKit/Tests/FinanzasKitTests`, 66 tests, swift-testing)

- `EnvelopeBudgetTests.swift`: `monthKeysStepAcrossYears`, `assignedMinusSpendingIsAvailable`, `leftoverAndUnassignedMoneyCarryForward`, `overspendingResetsCategoryAndReducesNextMonthToBudget`, `carryoverKeepsOverspendingInTheCategory`, `overAssigningMakesToBudgetNegativeAndItCarries`, `emptyMonthBeforeAnyDataIsZero`, `spentShareIsSpendingOverAvailableFunds` (parameterized: 50000/−45000/5000→0.9; overspent→1.0; 0/0/0→0; carried 20000 unspent→0; refund 30000/+10000→0).
- `RecurrenceTests.swift`: `monthlyOccurrencesWithinRange`, `monthEndClampsWithoutDrift` (Jan 31 start → Jan 31, Feb 28, Mar 31, Apr 30), `intervalAndWeekly`, `weekendMovesAfterOrBefore`, `endDateStopsOccurrences`, `weekendShiftNeverRepeatsADate`.
- `UpcomingTests.swift`: `upcomingListsOccurrencesWithProjectedBalancePerAccount` (ordering by date then name; per-account projected balances `[45000,4000,15000,3000]`), `scheduleWithoutStoredNextDateStartsAtItsFirstOccurrence`, `skipAdvancesToTheFollowingOccurrence`, `postingCreatesALinkedEntryAndAdvances`, `passingTheLastOccurrenceCompletesTheSchedule`, `dailyWeekendShiftedScheduleListsEachDateOnce`.
- `MonthSummaryTests.swift`: `monthSummaryTakesIncomeAndSpendingFromTheBudgetRows`, `savingsRateIsNilWithoutIncome`, `relativeChange` params `(110,100,0.1),(90,100,−0.1),(5,0,nil)`, `aRefundedCategoryDoesNotMakeSpendingNegative`, `uncategorizedIncomeCountsOnlyThisMonthsOnBudgetDepositsWithoutCategory`, `toBudgetHintTellsWhatToDoNext`.
- `EntryInputTests.swift`: `parsesTypedAmountsToPositiveCents`, `rejectsInvalidAmounts` (`""`,`"abc"`,`"1.2.3"`), `payeeLookupIsCaseInsensitiveAndCreatesWhenMissing`, `payeeNamedFindsExistingWithoutCreating`, `budgetInputSetsOrAdjustsTheAssignedAmount`, `budgetInputRejectsInvalidText`.
- `BudgetStoreTests.swift`: `loadsBudgetFromStoreIgnoringOffBudgetAndUncategorized`, `settingABudgetUpdatesInsteadOfDuplicating`.
- `ModelTests.swift`: `accountBalanceSumsItsEntries`, `scheduleRoundTripsItsRecurrence`.
- `AutomationTests.swift`: `autoPostRecordsEveryDueOccurrenceOfAutomaticSchedulesOnly`, `remindersFireDaysBeforeAtTheChosenHourAndSkipThePast`, `payeeSuggestsItsMostRecentCategory`, `postedSchedulesUseThePayeeCategoryWhenTheyHaveNone`.
- `ActualImporterTests.swift`: `importsLiveRowsWithMappingsResolved`, `importsSchedulesWithTheirRecurrence`, `refusesToImportIntoANonEmptyStore`, `importsBudgetAmountsMergingMappedCategories`, `importsBudgetsAloneIntoAnExistingStoreByCategoryName`, `importsScheduleDatesIntoAnExistingStoreByName`, plus gated live test `ACTUAL_DB`.
- `FinanzasToolsTests.swift`: `everySpecHasAnObjectSchema` (exact 12-name order), `listAccountsReturnsDecimalBalances`, `getTransactionsFiltersSortsAndTruncates`, `listCategoriesGroupsWithFlags`, `budgetMonthMatchesEnvelopeRules`, `setBudgetAmountValidatesAndUpserts`, `spendingByCategoryExcludesInflowsAndOffBudget`, `addTransactionReusesPayeeAndDefaultsCleared`, `updateTransactionChangesOnlyGivenFields`, `deleteTransactionReturnsTheDeletedRow`, `schedulesListSkipAndUpcoming`, `unknownToolAndBadIdsAreErrors`, `writesFromAnotherContainerAreDetectedByAuthor`, `idsAreShortStableAndTyped`, `jsonZeroAndOneAreAmountsNotBooleans`, `wrongTypedArgumentsAreRejectedNotIgnored`, `hugeIntegersAreRejectedWithoutCrashing`, `absurdAmountsAreRejected`, `addTransactionLearnsCategoryFromPayeeWhenNoneGiven`.

---

## 8. Performance hotspots (why it feels slow)

1. **Full-table aggregation inside `body`, per render.** `EnvelopeBudget.load` fetches **all** `Category`, `BudgetAmount` and `LedgerEntry` rows and folds them (`BudgetStore.swift:19-46`), and is called directly from the view body: `OverviewView.swift:29`, `BudgetView.swift:21`. Any `@Query`-observed mutation invalidates and re-runs it.
2. **Envelope walk repeats from the earliest data month, many times per frame.** `EnvelopeBudget.month` starts at `min(all budget/activity months)` and re-walks every month to target (`EnvelopeBudget.swift:40-69`); Overview calls `MonthSummary` for current + previous and 6–12 chart months (`OverviewView.swift:31-32,133-140`) → 8–14 full history walks per redraw, each iterating all categories.
3. **Balances are computed by reducing every relationship in memory.** `Account.balance` (`Models.swift:23`) is used per sidebar row (`FinanzasApp.swift:150-160` region), per account in `Upcoming.items` seeding (`Upcoming.swift:56`), and per Overview card (`OverviewView.swift:107`). With SwiftData faults this can materialize every entry of every account repeatedly.
4. **Whole-ledger fetches feed dense views.** `OverviewView.swift:20` sorts all entries reverse just to show 8 rows; `UpcomingView.swift:8` fetches all entries only to force recompute; `BudgetView.swift:9` fetches all entries to compute one banner; `EntriesView.swift:131-137` re-filters the entire array with `localizedStandardContains` on every keystroke of the search field, and the header recomputes the filtered sum each time.
5. **Linear scans for IDs.** `ModelID.find` fetches an entire entity and string-compares (`ExternalChanges.swift:16-23`) — on every MCP write to resolve an id; `Payee.named` fetches all payees per keystroke/suggestion (`EntryInput.swift:50-58`; called from `EntriesView.swift:252`).
6. **Persistent-history polling + container swap on the main actor.** A 3 s timer runs `ExternalChanges.poll()` (history fetch) and, on a hit, saves, opens a **new** `ModelContainer` and swaps the window's container (`FinanzasApp.swift:12,32,55-64`), dropping every warm fault/cache; the history is read in full at `ExternalChanges.init` and never pruned (`ExternalChanges.swift:26-31`).
7. **Everything is `@MainActor`.** Budget/envelope math, importer (casts to `@MainActor`, `ActualImporter.swift:20`) and tool dispatch (`FinanzasTools.swift:111`) all execute on the main thread, so the aggregations above block UI.
