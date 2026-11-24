Objective: accounts get a preset icon and color, or the user's own image, settable in the app and through the MCP server.
Branch: feat/account-style · Delivery: ask-on-risk · Runner: `cargo test --workspace`; `pnpm --dir desktop check` + `pnpm --dir desktop test`; QA `desktop/qa/cuentas.js`

## Specs

- S1 Preset icon + color: "Or if it's a bank, you always put a logo of the bank or a SVG predetermined ... Que se pueda poner una imagen o un icono así predeterminado." User chose "Iconos + color".
- S2 Own image: user chose "Imagen propia": the user picks an image; it is stored with their data, never shipped in the app.
- S3 Agent: "si te digo que agregues un banco o si te digo algo, pues que tú ya sepas cómo ponerla y cómo configurar esa imagen y el banco que te pida y qué ponerla." MCP can set icon, color and image (from a local file the agent saved) when creating or editing an account.
- Decision: no bank logos bundled in the public repo (trademarks); logos only come from the user or the agent at the user's request, stored locally.
- Assumption: images are PNG, JPEG, WebP or SVG up to 512 KB, shown through <img> (an SVG cannot run scripts there).

## Tasks

- T1 [S1,S2] inline — schema v9 `account_style`; core service (icon/color lists, set style, set image with type sniffing and size limit). Risk: item 1 (schema migration, additive).
- T2 [S1,S2] inline — Tauri commands, AccountBadge in sidebar/Resumen/Deudas/account page, style picker when creating and editing, image picker.
- T3 [S3] inline — MCP set_account_style; create_account and list_accounts carry icon/color/image; tool descriptions tell agents how to add a bank logo.

## Log

- L1 (2026-11-23) User: "Or if it's a bank, you always put a logo of the bank or a SVG predetermined. You know, for the applicattion Estaría increíble. Que se pueda poner una imagen o un icono así predeterminado."
- L2 Choices: "Iconos + color (Recomendado), Imagen propia, \"Y la opción o una indicación de un agente de los MSP es que: si te digo que agregues un banco o si te digo algo, pues que tú ya sepas cómo ponerla y cómo configurar esa imagen y el banco que te pida y qué ponerla.\""
