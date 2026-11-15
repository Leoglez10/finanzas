# Release Finanzas 0.4.0
Objective: publish the verified wishes feature as a new release on Leoglez10/finanzas.
Branch: feat/wishlist-reservations; release base: f821174; delivery: single-pr (version-only unit, feature already reviewed); checks: workspace tests/clippy, manifest metadata, GitHub CI and release assets.

## Specs
S1. "subelo". Check: publish the local wishes commits to the existing origin repository, preserving unrelated work.
S2. "con nueva version". Check: bump 0.3.0 to 0.4.0 consistently, publish through the existing release workflow, observe release and installer assets.

## Tasks
T1 | S1,S2 | inline | done | Version commit 393096e; all 206 tests/clippy pass, metadata 0.4.0 on all crates; independent version-only verification PASS; committed assessment medium/under_budget (no native review due).
T2 | S1,S2 | inline remote delivery | in_progress | Push branch, create release PR, observe CI, merge using normal target policy; no force push.
T3 | S2 | inline remote delivery | pending | Observe installer workflow, release tag/assets and cask update; report failures honestly.

## Log
L1. User delivery authorization verbatim: "subelo"; clarification: "con nueva version".
L2. Existing remote https://github.com/Leoglez10/finanzas.git; gh target metadata confirms main is default. Latest published release v0.3.0. origin/main is 318b24a, ancestor of verified feature. Existing release.yml automatically tags/publishes on main when Cargo version has no release, builds macOS/Windows/Linux installers, and updates Homebrew cask. Existing workflow and credentials are not modified. No local installation authorized/requested.
L3. Feature passed 206 tests, clippy, macOS build and screenshots, independent verification, and native approval (review-a1768c5cd1de47f4). Non-blocking category-deletion reservation advisory remains disclosed, no corrective scope in this publication. Preserve unrelated untracked odd/tasks/app-wide-visual-refresh.md. Version 0.4.0 selected for new feature; no dependency updates. Manifest-only change has no meaningful RED; check versions/lockfile and ordinary tests instead. Risk: item 5 (release/delivery); independent verification before remote publication.
