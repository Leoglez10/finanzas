# Design rules

The app should look like a tool (OpenCode, Hex), not like a generated landing page,
and look the same on macOS, Windows and Linux. Tokens live at the top of
`src/styles/app.css`. New and touched code uses only these tokens; older literals
are migrated screen by screen (see `odd/tasks/redesign-no-slop.md`).

## Color

- Colors only through named roles: `--canvas`, `--surface`, `--surface-2`, `--sidebar`,
  `--border`, `--ink`, `--muted`, `--brand`, `--brand-ink`, `--brand-soft`,
  `--income`, `--expense`, `--warn` (and their `-soft` variants), `--c-<category>`.
- No hex/rgb literals in components. No gradients.
- `--brand` fill is for at most one main action per screen. Everything else is a
  secondary (bordered or plain) control.

## Shape

- Two radii: `--r-control` (6px) for buttons, inputs, chips, rows; `--r-panel` (10px)
  for panels, dialogs, popovers. Circles (`50%`) only for avatars and dots;
  fully round ends (`99px`) only for switches and progress tracks.
- Spacing on a 4px grid: `--space-1` 4 · `--space-2` 8 · `--space-3` 12 ·
  `--space-4` 16 · `--space-6` 24 · `--space-8` 32.
- Sections are divided by a 1px `--border` line, not by stacking shadowed cards.
- Shadows (`--shadow-float`) only on floating layers: menus, popovers, dialogs, toasts.

## Type

- Four sizes: `--text-sm` 12 (captions), `--text-md` 13 (body, tables),
  `--text-lg` 16 (section titles), `--text-xl` 24 (page title, key numbers).
- Amounts and small numeric captions use `--font-mono` with tabular numbers.
- Sentence case everywhere. No ALL-CAPS tracked labels.

## Banned

- Glass / `backdrop-filter` blur.
- `text-transform: uppercase` labels with letter-spacing.
- Sparkle ✨ icons, marketing taglines under the logo.
- Stat cards with an icon inside a tinted square.
- Pills (`border-radius: 99px`) as the default shape for chips and buttons.
- Decorative shadows, glows and motion. Motion only explains a change
  (open/close, reorder) and stays under 200ms.

## Checking a change

Every redesign PR shows before/after screenshots of the affected screens in light
and dark.
