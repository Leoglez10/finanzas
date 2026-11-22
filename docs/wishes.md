# Goals (wishes) and reserved funds

A goal stores a name, a type (**Compra**, **Viaje**, **Experiencia** or **Fondo**), a positive target price, an optional target date, optional line items, an optional HTTP(S) link, notes, and an optional expense category. Amounts in the core are integer cents. Archived goals remain in history and release their funds; completed ones are kept as achievements.

## Create or edit a wish in the app

1. In **Deseos y metas**, choose **Nueva meta** (or the edit button on a goal). Step 1 collects the type, name, price, an optional expense category, line items, target date, link and note.
2. **Continuar** shows the interpreted price before anything is saved; **Atrás** keeps what you typed.
3. **Guardar deseo** / **Guardar meta** saves. A failed save keeps the draft so you can correct it and retry.

The target-price field accepts `11000`, `11,000`, `11,000.50` and a pasted `$11,000` (comma thousands, period decimal). Malformed grouping such as `11,00`, non-positive amounts and more than two decimals show an error on the field. Links may be empty; otherwise use a complete HTTP(S) address.

Esc, the close button and **Cancelar** close the editor without saving and without asking. Saving does not record a purchase or move money. Every change in Deseos (saving, setting money aside, releasing, archiving, buying) can be undone from the **Deshacer** notice or with ⌘Z.

## Target date and monthly amount

With a target date, each goal shows how much to set aside per month to reach it on time: what is left (price − paid items − set aside) divided by the monthly dates from today (same day of the month) up to the target, rounded up. A goal in the target's month asks for the rest at once; a past date shows it as overdue. **Aparta este mes** adds up the monthly amounts of active goals.

## Line items

A goal can be split into line items (flight, hotel, food…). When it has items, its price is their sum. Each item can be marked as paid: paid items count as progress, lower what can still be set aside, and release money already set aside that they cover (the payment used it). Items keep their ids across edits, so tags on movements stay attached.

## History

Every change to the set-aside amount is logged with its date: setting money aside, releasing it, a category change, archiving, completing, undo, and the release when a paid item covers it. The log always adds up to what is set aside. Cards show the latest changes and, from the second day with changes, a step chart.

## Complete a goal

**Cumplida** (or the type's button: **Lo compré**, **Ya viajé**, **Lo viví**, **Meta cumplida**) records the expense in one transaction on the first open on-budget account, in the goal's category: one movement per unpaid line item, or one for the price when there are none. Paid items are assumed already recorded. The goal is archived with the date it was achieved and shows as a **Logro** in the archive. Undo removes the movements and restores the goal.

## Tag movements with a goal

In the movement dialog, **Meta** tags a movement with a goal and, optionally, one of its line items. Goal cards then show what tagged movements add up to, per item (red when over plan) and in total, and the ledger shows a chip with the goal. Movements recorded by **Cumplida** are tagged automatically. Deleting a movement removes its tag.

## Reserve without recording a purchase

A reservation claims money already available in an expense category. It does not move account balances, create an expense, change category assignments, or reduce the original envelope's reported available amount. Partial reserves are allowed. Set the absolute reserved amount to zero to release it.

Reservation increases are checked against the current month's category availability minus other active wishes' reservations, inside a SQLite immediate transaction. App and MCP writers cannot claim the same category funds twice. Changing a wish's category releases its reservation. Lowering its price below its reservation is refused until funds are released.

Reservations persist across months. Later spending or budget reductions can leave them underfunded: evaluation reports a reservation deficit, never silently moves money, and still permits reductions/releases. Archiving alone does not record a transaction.

## Read the evaluation

- **Budget:** available category funds minus other wishes' claims; the current wish's own reservation is not subtracted twice. A missing category produces an unknown result. An overassigned overall budget prevents an affordable verdict.
- **Cash:** net balances of open, on-budget accounts, less recorded outgoing payments due through 30 days from today (including overdue occurrences) and other active reservations. Future income and off-budget/closed accounts are excluded. Payments with no account make liquidity uncertain.
- **Shortfall:** price minus paid items minus usable funds, floored at zero, separately for budget and cash. Scheduled costs constrain cash, not category availability a second time.

This is a conservative estimate based on recorded data, not a guarantee: unrecorded expenses, bank availability, future price changes, currency conversion and account-specific payment routing are not modeled. Price and budget must use the profile's existing monetary unit.

## MCP example

Use `save_wish` with `{"name":"PlayStation","price":500,"url":"https://example.com/ps5","notes":"With controller","categoryId":"Category/p3"}`.
The result contains a typed id such as `Wish/p1`. Call `reserve_wish` with `{"id":"Wish/p1","amount":150}` to set aside 150 currency units, then `evaluate_wish` with `{"id":"Wish/p1"}`.
All monetary inputs/outputs use decimal currency units at the MCP boundary (maximum two decimals); `horizonDays` remains an integer day count. Add `perfil` to select a different existing profile.

`save_wish` replaces the editable details, not a partial patch: `name` and `price` are required on edits too; omitted URL/notes become empty, omitted/null category clears it (releasing the reservation), omitted `kind` becomes `compra` and omitted `targetDate` clears it. `items` is the exception: omitted keeps the current line items, `[]` removes them, and a list replaces them (send an item's `id`, such as `WishItem/p4`, to keep it); with items, `price` is their sum. Example: `{"name":"Cancún","price":1,"kind":"viaje","targetDate":"2027-06-15","items":[{"name":"Vuelo","amount":4200,"paid":true},{"name":"Hotel","amount":3600}]}`.

`list_wishes` returns active goals by default; `includeArchived: true` includes history. Each entry has `wish` (with `kind`, `targetDate`, `paid`, `achievedDate`), `evaluation`, `plan` (`monthsLeft`, `monthly`, `remaining`, `overdue`; null without a date), `items`, `contributions` (dated history) and `spending` (tagged movements per item). `archive_wish` releases reserved funds but never records the purchase.

## Storage

Migrations are additive and transactional; none rewrites ledger records. Version 2 adds the `wish` table; 3 adds `kind` and `target_date`; 4 adds `wish_item`; 5 adds `wish_contribution` (opened with money already set aside); 6 adds `achieved_date`; 7 adds `wish_entry` (movement tags, removed with their movement). Back up your database before upgrading: an older app refuses a newer file without changing it. Each profile has its own goals and reservations.
