# Native two-step wish editor
Objective: implement the chosen artboard in Rust/Rune, fixing grouped target prices and silent validation.
Branch: feat/wish-wizard; base: 2c9f730956b52805e9e7a57ffb50cb1a4b67359e; delivery: single-pr / local work unit only; checks: isolated-target cargo test --workspace, clippy, native scenes and independent verification.

## Specs
S1. "Implement the design artboard design/wish-dialogs/03-guided-steps.html"; "Compra en dos pasos"; "Use the artboard as the visual spec: map its parts to the project's existing components and styles, and do not copy the artboard HTML verbatim." Check: native purchase step, context/review step, back navigation, real save only on final confirmation; preserve name, price, category, URL and multiline notes.
S2. "Take colors, typography, spacing, radius and shadows from design/system.md, the project's token source." Check: existing palette, headline/amount tokens, 24px padding, 580px maximum width, 14px radius, no new theme switch/shadows; scroll body and visible actions at smaller native window size.
S3. "no me deja guardar, como se pueden mejorar esos modales? /gentle:design". Check visual-spec behavior: unambiguous prices 11,000 / 11000 / 11,000.50, explicit interpreted amount, field errors on attempted advance/save, optional data expandable, dirty-dismiss confirmation, busy single-shot guard, backend failure retains inputs. Do not alter shared/core money parser or reservation-input semantics.
S4. "Comment id: radjmca2"; "Element: #theme"; "Excerpt: \"Tema claro\""; "quita ese boton". Check: no theme button; record resolved comment and handoff progress.
S5. "Report progress with the gentle_design tool: action \"handoff\", target \"wish-dialogs/03-guided-steps.html\", status \"in_progress\" when you start; status \"implemented\" when it is done, with previewUrl set to the running app's loopback URL (localhost, 127.0.0.1 or [::1]) when one is running, so the canvas can compare it with the artboard." Check: report real native implementation, no invented web URL for desktop app, native snapshot proof. No publishing, release bump, install or real-user-data access in this request.

## Tasks
T1 | S1-S4 | inline | done | Dedicated-target RED then 7/7 native editor tests passed (job-23); price parsing, dirty close, focus, failure retention and reservation regression verified; commit pending.
T2 | S1-S5 | inline | done | Job-24: 213 tests, workspace clippy and app build passed; job-25: four native captures rendered and inspected (purchase light/dark, compact review, discard); docs/scenes updated; commit pending.
T3 | S1-S5 | gentle-ai-verify + native review when due | in_progress | Independent checks for shared Escape dispatch; handoff/comment closure; commit pending.

## Log
L1. Original report verbatim: [Image#1] no me deja guardar, como se pueden mejorar esos modales? /gentle:design
L2. Implementation handoff verbatim:
Implement the design artboard design/wish-dialogs/03-guided-steps.html

Artboard: design/wish-dialogs/03-guided-steps.html
Title: Compra en dos pasos
Tradeoff: Separa captura y revisión del importe; añade un paso para una compra sencilla.

Implement it in the project's real stack. Use the artboard as the visual spec: map its parts to the project's existing components and styles, and do not copy the artboard HTML verbatim.
Take colors, typography, spacing, radius and shadows from design/system.md, the project's token source.

Open comments on this artboard (1); apply each one in the implementation:
1. Comment id: radjmca2
   Element: #theme
   Excerpt: "Tema claro"
   Comment:
   quita ese boton
When they are applied, resolve them with the gentle_design_comments tool (action "resolve", slug "wish-dialogs", ids ["radjmca2"], note "<one line on what you changed>").

Report progress with the gentle_design tool: action "handoff", target "wish-dialogs/03-guided-steps.html", status "in_progress" when you start; status "implemented" when it is done, with previewUrl set to the running app's loopback URL (localhost, 127.0.0.1 or [::1]) when one is running, so the canvas can compare it with the artboard.
This design is not linked to an ODD feature document: size it under the project's ODD rules as usual.
This handoff itself commits nothing.
L3. Selected visual specification: design/wish-dialogs/03-guided-steps.html; alternatives01/02 remain alternatives. Artboard already reflects resolved radjmca2. Native stack Rust/Rune; original source accepted plain digits but rejected commas. Isolated Runtime reproduction confirmed 11,000 stores nothing and 11000 stores1100000 cents. No production edits in design-only phase.
L4. Runtime preparation incident: first RED reused the earlier diagnostic executable from a shared Cargo target and ran wrong tests. Verified real workspace/manifest, switched to dedicated target/wish-wizard-build without cleaning anything. Correct RED (job-20): six requested-rule tests failed, existing reservation regression passed. New tests were written before shared Escape impact classification; durable tracking now created before implementation edits. Risk: item3 (shared on_key Escape dispatch used by other modals). All other modal/command behavior must remain unchanged.
L5. Forecast ~650 authored changed lines including tests/docs; one coherent native-editor behavior. Single local work unit keeps state/render/tests together; line budget is advisory, no code golf. No source writer delegation: one dependent behavior unit, implemented inline. Independent verification required for item3. Existing untracked design/ and odd/tasks/app-wide-visual-refresh.md preserved; the latter is unrelated and not part of this work. No production app, clients, database, environment or dependency mutation.
L6. A shell-based edit script was denied before execution by the sensitive-path policy, apparently interpreting Rune method syntax as a sensitive file path. No alternative mutation was attempted. Safer proposed plan: exact editor-tool replacements scoped to crates/finanzas-app/src/wishes.rs and crates/finanzas-app/src/main.rs, with no credential/key files or out-of-project access. User selected authorize_scoped_native_edits through the consent UI. Resume only scoped editor-tool changes for those two implementation files; no shell-based source mutation. Verification tasks follow implementation.
L7. Implementation uses scoped editor tools only. Price parser accepts unambiguous grouping, separate from unchanged reservation parser. Native stepper, review, optional multiline details, field descriptions/invalid states, guarded dismissal with focus return and busy/success/error feedback map the chosen artboard. GREEN job-23 passed7/7 Runtime tests. Initial test preparation issues corrected: text event with empty string does not clear selection (use Backspace), clipped inputs need real wheel scrolling, and concurrent double-save probe holds the worker lock to keep the pending state deterministic. Rune ElementInfo.visible is a clipped rectangle, not a boolean; exact pinned API inspected before fixing helper. Added compact-window/keyboard and exact grouped-decimal/ledger-preservation probes for final suite. Native scenes cover purchase, review and discard; no localhost app URL exists for this desktop binary.
L8. Final self-verification job-24 passed213 tests (90 app,90 core,7 core-wishes,25 MCP,1 stdio), clippy --workspace --all-targets -- -D warnings, and app build using dedicated CARGO_TARGET_DIR=target/wish-wizard-build. Native captures under target/wish-wizard-proof/ rendered with isolated FINANZAS_DB (job-25), visually inspected: purchase-light.png, purchase-dark.png, review-small.png at800x600, discard.png. Footer actions remain visible; context body scrolls to optional fields. Runtime probes verify grouped decimals, unchanged ledger, unchanged reservation parsing, focus on validation/navigation/return, compact focus trap, dirty-dismiss and pending/error retry. Earlier failed checks were fixed test setup issues described in L7; no failed functional checks remain. CLI parser/options are unchanged; only scene registry additions and wish-specific Escape branch changed, so a differential CLI parser check has no applicable changed shared input. Existing sibling modal tests all reran in the workspace suite. No Windows/Linux runtime check or installed-app update in this request.
L9. Visual spec remains design/wish-dialogs/03-guided-steps.html; system source design/system.md. Manifest now links this feature; alternative artboards untouched. Comment radjmca2 was already resolved and implementation adds no theme button. All design artifacts remain existing untracked task assets, excluded from code commit; native review request uses this specification and independent verifier reads selected artboard/system directly. One coherent code unit is ~484 authored changed lines plus tracker; retaining behavior/tests/docs together avoids artificial splitting.
