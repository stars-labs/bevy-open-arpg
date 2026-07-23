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
- GitHub release attachment parity fixed: `release-asset-manifest.csv` is now attached to every GitHub release with web builds, matching the published release manifest and Bevy-style reproducibility expectations.
- GitHub release workflow behavior refined for Bevy-like preview handling:
  - `generate_release_notes` now runs only for versioned tag releases, avoiding noisy/redundant generation on `web-latest` preview runs.
- GitHub release pipeline hardened for publish parity with Bevy-style flow:
  - tag creation now prefers GPG-signed annotated tags when available and falls back to normal annotated tags when no signing key is configured.
  - release-trigger workflow concurrency is now scoped by workflow/ref to prevent unrelated runs from canceling each other.
  - release publishing now fails fast when expected `web-latest` or tagged artifacts are missing, mirroring Bevy's strict attachment contract for reproducible releases.
  - added `actions/configure-pages@v5` in the Pages deploy stage to match GitHub's standard publish flow.
  - publish helper documentation expanded with versioned release and verification workflow.
- Release package contents now include `LICENSE` in native artifacts so legal metadata travels with published binaries.
- `scripts/build_web.sh` was cleaned up to satisfy shellcheck style checks used in CI.
- CI release workflow now supports Merge Queue (`merge_group`) validation for PR merge-path checks and only cancels in-progress CI runs for PR/merge-queue events; checkout steps now use `persist-credentials: false` to align with Bevy security defaults.
- Character/model polish, full combat animation pass, and additional endgame content are planned in future slices.
