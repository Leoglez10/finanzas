Objective: credit-card details (all optional), installment purchases, and a "Deudas" page enabled from Ajustes that gathers every debt with payment advice; plus a dashboard widget.
Branch: feat/deudas-tarjetas · Delivery: ask-on-risk · Runner: `cargo test -p finanzas-core`; `cargo test -p finanzas-desktop` (settings); `pnpm --dir desktop check` + `pnpm --dir desktop test`

## Specs

- S1 Credit-card data, optional: "lo de la tc quisiera que los datos fueran opcionales y que no limiten". An account can be marked as credit card with limit, cut day, due day, annual rate and minimum-payment %; every field may stay empty and nothing is blocked.
- S2 Notice when data is missing: "obviamente si lo pones que salga un aviso de si lo pones puede la app recomendarte mejores opciones y pagos".
- S3 Installment purchases (from the first message): "si hiciste un pago a meses como dividirlo si es una tarjeta, como hacer los calculos". Plan = total, months, start date, optional rate (0 = MSI); app shows monthly charge, months left, amount left.
- S4 Smart calculations (first message): "que sea un sistema inteligente de mira debes tanto y tu interes de tanto". Debt, available credit, utilization, payment to avoid interest, minimum payment, estimated interest if paying only the minimum (includes 16% IVA), next cut and due dates.
- S5 Debts page: "una nueva pestaña seleccionable que se seleccione en ajustes que se llame deudas y junte todas las deudas, un nuevo apartado en ajustes que habilite esa pagina". Off by default; switch in a new Ajustes section; sidebar shows "Deudas" only when on.
- S6 Dashboard widget (first message): "un nuevo widget en el dashboard".
- S7 Rates: entered by hand with a hint of where to find them on the statement; no researched rate table (rates are per customer).
- Assumption: a debt is any open credit-card account, plus any open account with a negative balance (loans).
- Assumption: MCP tool for debts deferred.

## Tasks

- T1 [S1,S3,S4] inline — schema v8 (`credit_card`, `installment`), store CRUD, `service::debts` with calculations and tests. Risk: item 1 (schema migration).
- T2 [S1,S2,S3] inline — Tauri commands + card/installment dialogs, AccountDialog toggle. Risk: none expected.
- T3 [S2,S4,S5,S7] inline — `debts_page` setting, Ajustes "Deudas" section, sidebar entry, Deudas screen with advice. Risk: none expected.
- T4 [S6] inline — "Deudas" Resumen widget. Risk: none expected.

## Log

- L1 (2026-11-23) User: "se me ocurre hacer como una division en las cuentas o añadirle mas opciones como la de cuando es una tarjeta de credito ponerle los limites de saldo, cuanto llevas, porcentaje de cobros o intereses, si hiciste un pago a meses como dividirlo si es una tarjeta, como hacer los calculos, quizas poner en el plan md investigar las tasas u ponerlas en la app, o que se pongan a mano, pero apartados en esas tarjetas de Credito para ponerle de que limite y dinero y todo lo demas. que sea un sistema inteligente de mira debes tanto y tu interes de tanto y que te sirva como para llevar cuentas tambien un nuevo widget en el dashboard, que plan me propones"
- L2 (2026-11-23) User: "lo de la tc quisiera que los datos fueran opcionales y que no limiten, obviamente si lo pones que salga un aviso de si lo pones puede la app recomendarte mejores opciones y pagos. me gustaria una nueva pestaña seleccionable que se seleccione en ajustes que se llame deudas y junte todas las deudas, un nuevo apartado en ajustes que habilite esa pagina. crea el plan y empieza a implementarlo"
- L3 Route: all inline, sequential (units depend on T1). Money stored as cents, rates as basis points (4500 = 45.00%).
