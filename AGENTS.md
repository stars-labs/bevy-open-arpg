# Repository Guidelines

## Project Structure & Module Organization

This is a single Rust 2024 crate built with Bevy 0.19. `src/main.rs` registers plugins and top-level game states; each other `src/*.rs` file owns a gameplay domain. Runtime tuning lives in `assets/data/*.ron`, with generated content under `assets/`. Integration checks are in `tests/`; unit tests sit beside implementations in `#[cfg(test)]` modules. Web files are in `web/`, automation in `scripts/`, and asset generators in `tools/`.

## Build, Test, and Development Commands

- `cargo run` launches the native game.
- `cargo run --features dev_tools` enables debug tooling and asset watching.
- `cargo fmt --all -- --check` verifies Rust formatting.
- `cargo check --locked --all-features` checks every feature using `Cargo.lock`.
- `cargo clippy --locked --all-targets --all-features -- -D warnings` runs the CI lint policy.
- `cargo test --all-features --locked` runs unit and integration tests.
- `cargo run --locked -- --headless-smoke --smoke-chapter=2 --smoke-frames=120` validates headless startup.
- `bash scripts/build_web.sh` builds the WebGPU bundle into `web/`.

## Coding Style & Naming Conventions

Use `rustfmt` output (four-space indentation). Name modules and functions in `snake_case`, types and plugins in `UpperCamelCase`, and constants in `SCREAMING_SNAKE_CASE`. Keep behavior in its owning plugin and communicate through Bevy resources and events. Put balance changes in RON data. Preserve native/wasm gates and do not casually change the audited Bevy features.

## Testing Guidelines

Add focused unit tests beside changed logic, using descriptive `snake_case` names. Update `tests/asset_manifest.rs` when runtime assets change. There is no numeric coverage target; exercise regressions and new behavior directly. Run format, check, Clippy, and tests before submitting. Use the headless smoke test for gameplay changes and the web build for rendering, asset, or platform-sensitive changes.

## Commit & Pull Request Guidelines

Recent history uses short Conventional Commit-style subjects such as `chore: ...` and `ci: ...`; follow the same pattern (`feat:`, `fix:`, `docs:`) with an imperative summary. Keep commits and PRs scoped. PRs should explain what changed and why, list commands run, link relevant issues, and include screenshots for visible UI or rendering changes. Update `CHANGELOG.md` for user-facing behavior, documentation for controls/content, and asset manifest assertions when applicable.

## Asset Generation

Regenerate models and audio through the scripts in `tools/blender/` and `tools/audio/`; do not hand-edit generated binaries. Keep filenames stable and commit generator changes with their outputs.
