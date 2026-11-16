# Finanzas design system

Captured from current repository sources for app-modal prototypes. These are observed tokens, not a replacement production theme. Native app colors/components take precedence over the separate marketing website.

## Dark colors
| Token | Value | Use / source |
|---|---|---|
| `--canvas` | `#151D27` | App background; `crates/finanzas-app/src/main.rs:584` |
| `--card` | `#1E2936` | Dialog surface; same source |
| `--border` | `#354453` | Borders; same source |
| `--petroleum` | `#67C6DF` | Primary action/focus; same source |
| `--emerald` | `#6DDBAD` | Success; same source |
| `--coral` | `#FF9A8F` | Errors; same source |
| `--amber` | `#F5C46B` | Warnings; same source |
| `--ink` | `#F3F7F9` | Primary text; same source |
| `--muted` | `#A9BAC6` | Secondary text; same source |
| `--selected` | `#304657` | Selected/secondary controls; same source |

## Light colors
| Token | Value | Use / source |
|---|---|---|
| `--canvas` | `#F4F7FA` | App background; `crates/finanzas-app/src/main.rs:586` |
| `--card` | `#FFFFFF` | Dialog surface; same source |
| `--border` | `#DEE7ED` | Borders; same source |
| `--petroleum` | `#155E75` | Primary action/focus; same source |
| `--emerald` | `#147D59` | Success; same source |
| `--coral` | `#BE493D` | Errors; same source |
| `--amber` | `#986000` | Warnings; same source |
| `--ink` | `#202F3A` | Primary text; same source |
| `--muted` | `#6C7E8A` | Secondary text; same source |
| `--selected` | `#E2F0F4` | Selected/secondary controls; same source |

## Typography
| Token | Value | Use / source |
|---|---|---|
| `--font` | `system-ui` | Browser prototype mapping from `site/assets/system.css:9-26`; native Rune BODY/TITLE/LABEL exact font metrics were not verified |
| `--body` | `16px` | Readable prototype body from `site/assets/system.css:34-104`, not a claim about native BODY |
| `--headline` | `22px` | Native headline; `crates/finanzas-app/src/theme.rs:5-10` |
| `--headline-weight` | `650` | Same source |
| `--hero` | `38px` | Native hero; same source |
| `--hero-weight` | `700` | Same source |
| `--amount` | `26px` | Native amount; same source |
| `--amount-weight` | `700` | Same source |
| `--card-title` | `13.5px` | Native card title; same source |
| `--card-title-weight` | `600` | Same source |

## Spacing and geometry
| Token | Value | Use / source |
|---|---|---|
| `--space-1` | `4px` | Web spacing scale, `site/assets/system.css:34-104` |
| `--space-2` | `8px` | Same source |
| `--space-3` | `12px` | Same source; native wish-body gap also 12 |
| `--space-4` | `16px` | Same source; native wish-form gap also 16 |
| `--space-6` | `24px` | Same source; native wish-form padding also 24 |
| `--space-8` | `32px` | Same source |
| `--space-12` | `48px` | Same source |
| `--field-gap` | `5px` | `crates/finanzas-app/src/sheet.rs:21-25` |
| `--field-padding` | `9px` | Same source |
| `--control-radius` | `8px` | Same source |
| `--button-padding` | `10px` | Same source |
| `--button-radius` | `8px` | Same source |
| `--primary-radius` | `10px` | `crates/finanzas-app/src/theme.rs:27-35` |
| `--dialog-radius` | `14px` | `crates/finanzas-app/src/wishes.rs:185-190` |
| `--dialog-width` | `580px` | Same source |
| `--overlay` | `rgb(0 0 0 / 0.4)` | Native wish-modal veil, same source |

## Shadows
No shadow is applied to the native wish modal. Use borders/surfaces rather than introducing a new shadow token. The marketing site's shadow at `site/assets/system.css:629` belongs to that website and is not used here.

## Component rules
- Existing: canvas/card surfaces, one-pixel borders, petroleum actions, emerald success, coral errors, amber warnings. Native input padding 9, gap5 and radius8; native modal padding24, gap16, radius14, width580.
- Prototype adaptations: system-ui/16px body, responsive max-width, scrollable body, persistent footer, visible focus ring and minimum 44px controls. Geometry can reflow; these adaptations are not claimed as existing native behavior.
- Proposed UX: label required/optional fields; associate helper/error text; keep submit available to explain validation; accept only unambiguous comma-thousands/period-decimal prices in these prototypes, with explicit interpreted amount. No live budget or bank data is loaded.
- Native HTML dialog provides focus containment; Escape/cancel asks before discarding changes and restores trigger focus. Busy/error/success are demo-only states, never a real save or financial assessment.
- Color and font self-check: artboards may use only the palette above, `currentColor`, transparent, the recorded overlay and system-ui. Light primary text uses recorded white card; dark primary text uses canvas. No external fonts, scripts, images or styles.

## Sources inspected
Native: `crates/finanzas-app/src/main.rs`, `theme.rs`, `sheet.rs`, `wishes.rs`; monetary validation: `crates/finanzas-core/src/domain/money.rs`. Marketing source inventory checked by the explorer: `site/assets/system.css`, `site/assets/backgrounds/backgrounds.css`, `site/assets/icons/icon.css`, `site/assets/layout/layout.css`, `site/assets/motion/motion.css`. Marketing palette/animation are intentionally not imported into the native-app directions.
