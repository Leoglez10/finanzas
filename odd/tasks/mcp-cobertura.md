Objective: the MCP server can do (almost) everything the app does with data.
Branch: feat/mcp-deudas · Delivery: ask-on-risk · Runner: `cargo test -p finanzas-core -p finanzas-mcp`; `cargo test -p finanzas-desktop`; QA `desktop/qa/cuentas.js`, `presupuesto.js`

## Specs

- S1 "haz una investigacion de la app, y dime que partes no tienen mcp, estaria bien que agregues para que pueda hacer la mayoria de cosas si no es que todo en la app"
- S2 Accounts: create an account with optional opening balance, off-budget flag and card terms (same as Nueva cuenta).
- S3 Categories: create a category, rename/hide it, create/rename a group (same as Presupuesto editing).
- S4 Schedules: create/edit, register the next occurrence, delete.
- S5 Goals: mark a line item paid, complete a goal (records its expenses), tag a transaction with a goal/item.
- S6 Resumen summary for a period, and adding many transactions at once, all or none (same as CSV import).
- S7 List profiles (the `perfil` argument already selects one).
- Out of scope (app-only): appearance, notifications, updates, Actual/legacy import file pickers, create/delete profiles.
- Invariant: app and MCP share the same core function for each write (create account, categories); no second implementation.

## Tasks

- T1 [S2] inline — move create_account to core service, app uses it; MCP create_account. Risk: none (app behavior unchanged). Status: done.
- T2 [S3] inline — move category/group writes to core; MCP create_category, update_category, save_category_group. Risk: none. Status: done.
- T3 [S4] inline — MCP save_schedule, post_schedule, delete_schedule. Status: done.
- T4 [S5] inline — MCP set_wish_item_paid, complete_wish, tag_transaction. Status: done.
- T5 [S6] inline — MCP get_overview, add_transactions. Status: done.
- T6 [S7] inline — MCP list_profiles; README tool table. Status: done.

## Log

- L1 (2026-11-23) User: "haz una investigacuon de la app, y dime que partes no tienen mc, estaria bien que agregues para que pueda hacer la mayoria de cosas si no es que tpdp en la app"
- L2 Gap table given to the user; same branch as the card/installment tools (cafd725).
- L3 Finding: the app UI never calls create_category/change_category/change_group (backend-only commands); checked them directly against the real app after moving them to core (6/6 incl. Spanish errors).
- L4 Evidence: core 96 + catalog 2 + debts 8, MCP 34 + stdio, desktop 4, clippy -D warnings clean; QA cuentas, presupuesto, deudas green.
