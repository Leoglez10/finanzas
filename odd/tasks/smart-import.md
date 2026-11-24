Objective: importing works with most bank CSV and Excel files: columns detected by header and content, ambiguities resolved from the data or asked once in the preview, and remembered per bank.
Branch: feat/smart-import · Delivery: ask-on-risk · Runner: `pnpm --dir desktop test` + `pnpm --dir desktop check`; `cargo test -p finanzas` (desktop/src-tauri)

## Specs

- S1 Excel: "que detecte la mayoria de excwl o cvs que tenga, si por ejemplo qu eponga un ecvel que ellos tengan ya caserom" — .xlsx, .xls and .ods import like .csv (first non-empty sheet; Excel dates become ISO dates).
- S2 Column detection: "que detecte la mayoria de columnas" — header row found among the first rows (title rows before it are skipped); each column scored by header name and by content (dates, money, text), so unknown header names still map.
- S3 Ambiguities: "como se pueden resolver ambiguedades o detectare dartos de las columnas" — date order (day/month vs month/day) decided from the column's values, Excel serial dates and month names ("05-nov-2026") read; decimal separator (1.234,56 vs 1,234.56) and parenthesis negatives read; a cargo/abono "Tipo" column sets the sign; total/balance rows skipped instead of errors.
- S4 Ask instead of guess: in the preview, each column has a selector pre-filled with the detection (Fecha, Descripción, Monto, Cargo, Abono, Tipo, Categoría, Ignorar), a date-order choice when the data cannot decide it, and "los gastos vienen en positivo".
- S5 Remember: the choices are saved per header signature, so the next file from the same bank imports with them.
- Assumption: Latin-1 decoding fallback already exists (features.rs read_csv_file); no new encoding work.

## Tasks

- T1 [S1] inline — Rust reads .xlsx/.xls/.ods with `calamine` into rows; file pickers and drop accept them. Risk: item 5 (new dependency). Status: done 40eba0c (cargo test 6/6).
- T2 [S2,S3] inline — csv.ts detection engine: header search, content scoring, date/decimal/sign detection, totals skipped; tests. Status: done (csv.test.ts 16/16, 52/52 total).
- T3 [S4,S5] inline — ImportDialog mapping row, date-order and sign toggles, saved mappings per header signature. Status: done (53/53 unit, QA importar.js 21/21 in the real app).

## Log

- L1 (2026-11-24) User: "como se pueden resolver ambiguedades o detectare dartos de las columnas o qie detecte la mayoria de excwl o cvs que tenga, si por ejemplo qu eponga un ecvel que ellos tengan ya caserom que detecte la mayoria de columnas y haga un uen implementavcion en la apop de cualquiera que se le meta?"
- L2 (2026-11-24) Proposal accepted: "sí, impleméntalo". Route inline for all tasks (no writer trigger: units depend on each other).
- L3 Uncommitted Windows-shortcut fix (keys.ts etc.) rides along in the working tree; not part of this feature's commits.
- L4 T1 done 40eba0c; Windows-shortcut fix committed separately as 883c05f on this branch. T2 done: detect() + parseMoney, typeSign exact single letters.
- L5 T3 done. Auto-flip threshold raised to 3 of 4 positive amounts (a bank file with a few deposits must not flip). README updated.
