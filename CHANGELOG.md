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
- GitHub release pipeline hardened for publish parity with Bevy-style flow:
  - tag creation can proceed without GPG keys by falling back to unsigned tags.
  - release-trigger workflow concurrency is now scoped by workflow/ref to prevent unrelated runs from canceling each other.
  - publish helper documentation expanded with versioned release and verification workflow.
- Character/model polish, full combat animation pass, and additional endgame content are planned in future slices.
