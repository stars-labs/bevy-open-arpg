# Changelog

## [0.1.0] - 2026-07-24

### Added
- Initial playable vertical slice with chapter system, enemy encounters, loot, run progression, UI, and combat feedback.
- Headless-smoke and native/web publish scripts for CI and GitHub Releases.
- Explicit Bevy 0.19 feature profile with desktop-native and web-native split and startup feature audit.
- Repository-level publishing metadata and docs for GitHub workflows, releases, and contribution flow.

### Changed
- Reworked startup diagnostics and startup-path handling so popup mode is explicit and logged.

### Fixed
- Addressed previous release metadata gaps and tightened `vX.Y.Z` tag validation for release automation.

## Unreleased
- Replaced the enclosed arena with an unbounded, deterministic wilderness that streams 48 m chunks around the player, unloads distant terrain, recreates seeded biome dressing on return, preserves an authored main-quest origin region, and migrates movement, fog, navigation, local-radar UI, actors, objectives, and legacy saves away from fixed world bounds.
- Reworked the main menu and core HUD, inventory, character, and talent interfaces with generated Reliquary artwork and a clearer icon-driven hierarchy.
- Added Chapter II, The Hollow Bastion, with a distinct enemy roster, Bell Warden encounter, Soul Font ritual, Nyxara boss fight, lore, story beats, HUD guidance, and six Blender-generated GLB assets with named hit bones.
- Added campaign chapter transitions that carry the hero build and companion forward, persist active chapter identity in saves, and advance difficulty only after Chapter II.
- Added `--smoke-chapter=1|2` gameplay startup validation and fixed headless runs so saved audio preferences cannot start the native audio backend or leave CI processes hanging.
- GitHub release attachment parity fixed: `release-asset-manifest.csv` is now attached to every GitHub release with web builds, matching the published release manifest and Bevy-style reproducibility expectations.
- GitHub release workflow behavior refined for Bevy-like preview handling:
  - `generate_release_notes` now runs only for versioned tag releases, avoiding noisy/redundant generation on `web-latest` preview runs.
- GitHub release pipeline hardened for publish parity with Bevy-style flow:
  - tag creation now prefers GPG-signed annotated tags when available and falls back to normal annotated tags when no signing key is configured.
  - release-trigger workflow concurrency is now scoped by workflow/ref to prevent unrelated runs from canceling each other.
  - release publishing now fails fast when expected `web-latest` or tagged artifacts are missing, mirroring Bevy's strict attachment contract for reproducible releases.
  - added commit-pinned `actions/configure-pages@v6` in the Pages deploy stage to match GitHub's standard publish flow.
  - publish helper documentation expanded with versioned release and verification workflow.
- Release package contents now include `LICENSE` in native artifacts so legal metadata travels with published binaries.
- Fixed preview publication being skipped when the native matrix is intentionally skipped, and made the rolling `web-latest` tag follow the commit that actually produced the web bundle.
- Native release archives now include the complete `assets/` runtime tree and a platform-named root directory, so downloaded builds can load models, audio, and tuning data.
- Release manifests now use a valid detached SHA-256 instead of attempting to hash themselves, and release payloads receive GitHub build-provenance attestations.
- Web bundle checksum files now store a portable artifact basename and are verified during packaging, so `sha256sum -c web-dist.zip.sha256` works after a normal Release download.
- GitHub Actions now use commit-pinned current action releases and job-level least-privilege permissions.
- GitHub Pages now uses the Actions publishing source with protected `main` and `v*` deployment rules, plus an explicit workflow preflight that rejects legacy branch publishing.
- Pages deployments now finish with cache-busted SHA-256 parity checks for the live HTML, JavaScript, compressed WASM, and a Chapter II model.
- `scripts/build_web.sh` was cleaned up to satisfy shellcheck style checks used in CI.
- CI release workflow now supports Merge Queue (`merge_group`) validation for PR merge-path checks and only cancels in-progress CI runs for PR/merge-queue events; checkout steps now use `persist-credentials: false` to align with Bevy security defaults.
