# Finanzas 0.4.1: wish editor polish, site section, release and install
Objective: fix the three pending wish-editor items, add a Deseos section to the GitHub Pages site, publish 0.4.1 and install it.
Branch: feat/wish-wizard (on top of 35bdda6/aa51244); delivery: single-pr; checks: unique-target cargo test --workspace, clippy, native scenes, CI, release assets, pages deploy, brew install.

## Specs
S1. "resuelve" — the three pending items listed to the user: «El aviso «Deseo guardado.» sigue visible después de otras acciones en la pantalla.»; «Un precio pegado con `$` delante, como `$11,000`, da error.»; «El botón principal tiene esquinas algo menos redondeadas que en el diseño.» Check: status clears after other wish actions/route change; `$11,000` and `$ 11,000.50` accepted as target price (reservation input unchanged); primary buttons radius 10 (design/system.md `--primary-radius`).
S2. "agrega en la pagina de gh pages eso de los deseos." Check: site/index.html gets a Deseos section (and nav entry) in the existing kit style with a real app capture; site deploys via pages.yml.
S3. "despues sube" Check: version 0.4.1, push, PR to main, CI green, merge, release v0.4.1 with installers, cask updated.
S4. "y instala." Check: brew upgrade to 0.4.1 with validated backup of the current app bundle; no user database access.

## Tasks
T1 | S1 | inline | done | RED job-26 (3 expected failures) then GREEN job-27 11/11; commit f2088dc. Full suite runs in T3.
T2 | S2 | inline | done | #deseos section + nav link + square capture site/img/deseos-panel{,-700}.webp; verified with real headless Chrome via CDP scroll (1440x900, 390x844, no overflow); commit pending below.
T3 | S3 | inline + gentle-ai-verify (item5) | pending | Bump 0.4.1, full checks, independent verify, assess; commit pending.
T4 | S3 | inline remote | pending | Push, PR, CI, merge, release, cask, pages.
T5 | S4 | inline + verify | pending | Backup, brew upgrade, verify installed version/signature/isolated run.

## Log
L1. User request verbatim: resuelve, despues sube y instala. y agrega en la pagina de gh pages eso de los deseos.
L2. Context: previous reply listed exactly three pending items (stale "Deseo guardado.", `$` prefix rejected, primary radius). Other reviewer advisories (leading-zero groups, stricter URL, stale-guard error, key-branching) stay out of scope. Site: site/index.html (taste-skill kit), sections pagos/funciones/privacidad/mcp; pages.yml deploys when site/ changes on main (memory: may need `gh workflow run pages.yml --ref main` if merge does not trigger it). Release flow: bump Cargo.toml + lock, PR, merge; release.yml tags, publishes, bumps cask. Risk: item 5 (release/install), item 3 already covered for on_key. Only the wish branch of shared code touched; no CLI/flags change.
L3. T1: RED job-26 (radius 8!=10, saved status stale, $11,000 rejected), GREEN job-27 11/11, commit f2088dc. T2: capture from deseos-dark scene (isolated FINANZAS_DB), first wide crop left empty frame space so re-cropped square like proximos-panel. ego-browser CLI no longer on PATH; plain headless screenshots render blank (scroll-reveal), so verified with one-off CDP script target/site-shots/cdp-shot.mjs (untracked). One capture showed a broken image from a load race; rerun rendered correctly.
