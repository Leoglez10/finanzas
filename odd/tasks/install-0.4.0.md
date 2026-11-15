# Install Finanzas 0.4.0
Objective: upgrade the Homebrew-managed macOS application without touching user databases.
Target: /Applications/Finanzas.app, existing cask finanzas 0.3.0; recovery: validated app-bundle backup in target/; no production launch.

## Specs
S1. "instalala". Check: Homebrew reports installed 0.4.0, bundle version agrees, executable/signature and isolated headless scene validate.

## Tasks
T1 | S1 | parent | done | Backup target/install-backup-0.3.0-20261115-120944/Finanzas.app; both executable SHA256 match originals; codesign --verify --deep --strict passed; no database reads.
T2 | S1 | parent | done | brew update && brew upgrade --cask finanzas exit 0; Homebrew and bundle metadata report 0.4.0; MCP symlink correct; codesign strict passed.
T3 | S1 | independent gentle-ai-verify | done | All six independent installation/backup checks exit 0; parent isolated installed-runtime render exit 0; no blockers. Old MCP clients require restart.

## Log
L1. User authorization verbatim: "instalala".
L2. Observed Homebrew cask finanzas 0.3.0 and app CFBundleShortVersionString 0.3.0. Two installed finanzas-mcp processes active (not the GUI); do not kill unrelated clients. Upgrade installs app bundle only; production database migration deferred until user opens app/starts new MCP, so no user database backup/read/mutation performed. Before first production launch, advise database backup as release notes require. Risk: item 5 (environment/installation). Explicit user upgrade authorization covers replacing Homebrew-managed app, with backup/recovery; no unrelated rollback or app launch.
L3. Homebrew upgrade job-16 exit 0, 0.3.0 → 0.4.0. Warning: signer changed, macOS may prompt on next GUI launch. Parent strict deep signature verification exit 0. Installed executable headless empty scene rendered under FINANZAS_DB=target/installed-0.4.0-proof/isolated.db, exit 0; no production GUI launch. Active MCP PIDs unchanged; clients must restart to use new binary. Recovery backup remains in project target, independent of Homebrew old-version cleanup.
L4. Independent verifier mwi5hu95-4-5s7c: Homebrew version0.4.0, installed bundle version0.4.0 and strict signature, correct MCP symlink, backup version0.3.0 and strict signature all passed (six commands, exit0). Did not repeat optional runtime; parent already observed it. No installer/user-data changes by verifier. Installation complete; no GUI production launch, no database migration, no local client restarts. macOS signer-change prompt may appear at first launch. Risk: item5, verified via backup/signatures/package metadata/isolated runtime and independent check.
