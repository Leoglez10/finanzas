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
T3 | S3 | inline + gentle-ai-verify (item5) | done | Commit d405403 (0.4.1); job-30: 215 tests, clippy, build pass; assess base 35bdda6 reviewDue=false under_budget; verifier mwi894a3 S1-S3 PASS, no blockers (advisories: stale local tags, design/ untracked).
T4 | S3 | inline remote | done | PR #12 CI green; merged f9f344e; Release run 43753277587 success (version, 3 packages, publish, cask); Website run 43753277674 success.
T5 | S4 | inline + verify | done | Backup target/install-backup-0.4.0-20261115-134317 (hashes + strict signature OK); app quit gracefully, brew upgrade 0.4.0->0.4.1 exit 0; brew/bundle 0.4.1, codesign strict OK, MCP symlink OK, isolated scene OK; app relaunched (PID 83417); independent verifier mwi8tpq6 PASS (brew/bundle 0.4.1, strict signature incl. nested MCP, symlink, backup 0.4.0 valid, release v0.4.1 public with 4 assets, isolated scene shows new editor). Not proven: dmg checksum vs asset, Gatekeeper assess, MCP runtime.

## Log
L1. User request verbatim: resuelve, despues sube y instala. y agrega en la pagina de gh pages eso de los deseos.
L2. Context: previous reply listed exactly three pending items (stale "Deseo guardado.", `$` prefix rejected, primary radius). Other reviewer advisories (leading-zero groups, stricter URL, stale-guard error, key-branching) stay out of scope. Site: site/index.html (taste-skill kit), sections pagos/funciones/privacidad/mcp; pages.yml deploys when site/ changes on main (memory: may need `gh workflow run pages.yml --ref main` if merge does not trigger it). Release flow: bump Cargo.toml + lock, PR, merge; release.yml tags, publishes, bumps cask. Risk: item 5 (release/install), item 3 already covered for on_key. Only the wish branch of shared code touched; no CLI/flags change.
L3. T1: RED job-26 (radius 8!=10, saved status stale, $11,000 rejected), GREEN job-27 11/11, commit f2088dc. T2: capture from deseos-dark scene (isolated FINANZAS_DB), first wide crop left empty frame space so re-cropped square like proximos-panel. ego-browser CLI no longer on PATH; plain headless screenshots render blank (scroll-reveal), so verified with one-off CDP script target/site-shots/cdp-shot.mjs (untracked). One capture showed a broken image from a load race; rerun rendered correctly.
