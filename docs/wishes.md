# Wishes and reserved funds

A wish stores a name, positive target price, optional HTTP(S) link, notes, and an optional expense category. Amounts in the core are integer cents. Archived wishes remain in history and release their funds.

## Create or edit a wish in the app

1. In **Deseos**, choose **Nuevo deseo** (or **Editar** on an existing wish). **La compra** collects the name and target price.
2. Choose **Continuar** to review the interpreted price in **Contexto y revisión**. Select an optional expense category and expand **Enlace y detalles** if needed; notes support multiple lines. **Volver** preserves those values.
3. Choose **Guardar deseo** to save. Advancing to the second step does not create or modify a wish. A visible saving state prevents duplicate submissions; a failed save keeps the draft for correction/retry.

The target-price field accepts `11000`, `11,000`, `11,000.50` and a pasted `$11,000` (comma thousands, period decimal, optional leading `$`). The displayed interpretation makes the monetary unit explicit. Malformed grouping such as `11,00`, non-positive amounts and more than two decimal places produce a field-level error instead of silently disabling the action. Optional links may be empty; otherwise use a complete HTTP(S) product link.

Cancel, close and Escape ask before discarding changed values. Unchanged drafts close without confirmation. Saving does not record a purchase or move money. Reservation fields and MCP numeric amount conventions are unchanged by this editor.

## Reserve without recording a purchase

A reservation claims money already available in an expense category. It does not move account balances, create an expense, change category assignments, or reduce the original envelope's reported available amount. Partial reserves are allowed. Set the absolute reserved amount to zero to release it.

Reservation increases are checked against the current month's category availability minus other active wishes' reservations, inside a SQLite immediate transaction. App and MCP writers cannot claim the same category funds twice. Changing a wish's category releases its reservation. Lowering its price below its reservation is refused until funds are released.

Reservations persist across months. Later spending or budget reductions can leave them underfunded: evaluation reports a reservation deficit, never silently moves money, and still permits reductions/releases. Recording a purchase uses the existing transaction flow; release/archive the wish explicitly afterward. Archiving does not record a transaction.

## Read the evaluation

- **Budget:** available category funds minus other wishes' claims; the current wish's own reservation is not subtracted twice. A missing category produces an unknown result. An overassigned overall budget prevents an affordable verdict.
- **Cash:** net balances of open, on-budget accounts, less recorded outgoing payments due through 30 days from today (including overdue occurrences) and other active reservations. Future income and off-budget/closed accounts are excluded. Payments with no account make liquidity uncertain.
- **Shortfall:** price minus usable funds, floored at zero, separately for budget and cash. Scheduled costs constrain cash, not category availability a second time.

This is a conservative estimate based on recorded data, not a guarantee: unrecorded expenses, bank availability, future price changes, currency conversion and account-specific payment routing are not modeled. Price and budget must use the profile's existing monetary unit.

## MCP example

Use `save_wish` with `{"name":"PlayStation","price":500,"url":"https://example.com/ps5","notes":"With controller","categoryId":"Category/p3"}`.
The result contains a typed id such as `Wish/p1`. Call `reserve_wish` with `{"id":"Wish/p1","amount":150}` to set aside 150 currency units, then `evaluate_wish` with `{"id":"Wish/p1"}`.
All monetary inputs/outputs use decimal currency units at the MCP boundary (maximum two decimals); `horizonDays` remains an integer day count. Add `perfil` to select a different existing profile.

`save_wish` replaces the editable details, not a partial patch: `name` and `price` are required on edits too; omitted URL/notes become empty and omitted/null category clears it (releasing the reservation). `list_wishes` returns active wishes by default; `includeArchived: true` includes history. `archive_wish` releases reserved funds but never records the purchase.

## Storage

Schema version 2 adds the `wish` table transactionally to version 1 databases without rewriting ledger records. New and reopened databases use the same migration. Back up your database before upgrading; older binaries do not understand version 2. Each profile has its own wishes and reservations.
