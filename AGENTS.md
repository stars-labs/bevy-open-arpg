# Repository Guidelines

This file provides guidance to coding agents when working with code in this repository.

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

## Project

Bevy Open ARPG — an original dark-fantasy ARPG prototype built with Bevy 0.19 (Rust, edition 2024). Single binary crate (`bevy-open-arpg`), no workspace.

## Commands

```bash
# Verification (run all four before considering a change done)
cargo fmt --check
cargo check
cargo clippy --all-targets -- -D warnings
cargo test

# Run a single test
cargo test <test_name>                    # unit tests live in #[cfg(test)] mods inside each src file
cargo test --test asset_manifest          # the one integration test (asset file existence)

# Run the game
cargo run                                 # default: popup window
cargo run --features dev_tools            # debug tooling, file watching, dynamic linking, free camera
BEVY_OPEN_ARPG_HEADLESS_SMOKE=1 BEVY_OPEN_ARPG_AUDIO=0 cargo run   # CI / no-window smoke test
cargo run -- --headless-smoke --no-audio --smoke-frames=30         # same via CLI flags

# Asset regeneration (only after changing generator scripts)
blender --background --python tools/blender/generate_assets.py    # .glb models/VFX → assets/models/
python3 tools/audio/generate_sfx.py                                # audio cues → assets/audio/

# Web (WebGPU) build — deployed to GitHub Pages by .github/workflows/deploy-wasm-pages.yml
bash scripts/build_web.sh                                          # full bundle → web/
RUSTFLAGS='--cfg getrandom_backend="wasm_js"' cargo check --target wasm32-unknown-unknown --no-default-features --features webgpu
```

Nix flake provides the dev shell (`.envrc` + direnv); Linux native deps (ALSA, Vulkan, X11/Wayland, Blender) come from it.

## Runtime configuration

Most optional behavior is toggled by env vars and mirrored CLI flags, parsed in `src/main.rs` into a startup config: `BEVY_OPEN_ARPG_HEADLESS_SMOKE`, `BEVY_OPEN_ARPG_AUDIO`, `BEVY_OPEN_ARPG_REMOTE` (Bevy Remote HTTP), `BEVY_OPEN_ARPG_ASSET_MODE=processed`, `BEVY_OPEN_ARPG_RENDER_PROFILE` (compat default on Linux / `functionality` / `gl`), `BEVY_OPEN_ARPG_DIAGNOSTICS`, `BEVY_OPEN_ARPG_DEBUG_GIZMOS`. The app defaults to a windowed popup and only goes headless when explicitly requested.

## Architecture

Everything is a Bevy plugin registered in `src/main.rs`. Each `src/*.rs` module owns one gameplay domain and exposes a `<Name>Plugin` (e.g. `PlayerPlugin`, `CombatPlugin`, `LootPlugin`, `HudPlugin` in `ui.rs`). `main.rs` also owns the top-level `GameState` (`Loading → MainMenu → InGame → GameOver/Victory`), the `RunStats` resource, difficulty settings, and window/renderer/headless startup handling; most systems are gated with `run_if(in_state(...))`.

Key cross-cutting modules:

- `data.rs` — `GameDataPlugin`: deserializes tuning data from `assets/data/*.ron` (player, enemy catalog, loot tables) into resources. Balance changes go in the RON files, not code.
- `assets.rs` — `GameAssetsPlugin`: loads every glTF model into the `GameAssets` resource during `GameState::Loading`. New models must be added to `GameAssets`, the Blender generator, and `tests/asset_manifest.rs` (which asserts each runtime asset file exists on disk).
- `save.rs` — `SavePlugin`: RON save/load of the full run (player build, progression, world state) to `saves/`; imports state types from nearly every other module, so renaming/adding persisted fields usually touches this file.
- `ui.rs` (~22k lines) — `HudPlugin`: all HUD, menus, inventory/build/journal panels, and difficulty records.
- `feedback.rs` — `CombatEvent` and combat VFX/audio reactions; combat modules emit events that feedback/UI consume.

The remaining modules (enemy, combat, loot, chapter, dungeon, challenge, milestone, mastery, bounty, journey, rift, obelisk, ordeal, companion, bestiary, lore, story) follow the same plugin pattern and mostly communicate through resources and events rather than direct calls.

## Constraints

- Bevy's cargo feature list in `Cargo.toml` is deliberately audited and explicit (no `default` features). Don't add or remove Bevy features casually — the startup diagnostic and F6 clipboard summary audit the feature set, and the README documents the rationale for every inclusion/exclusion. Dev-only features belong under the `dev_tools` cargo feature.
- The crate must keep compiling for `wasm32-unknown-unknown` (webgpu feature). Bevy features that only build natively go in the `native` cargo feature (in `default`), not the base list. Filesystem, threads, and rodio usage must be `#[cfg]`-gated; wasm embeds the RON tuning data and skips save/profile persistence.
- Assets are generated, not hand-authored: models come from `tools/blender/generate_assets.py`, audio from `tools/audio/generate_sfx.py`. Keep output filenames stable; export models to `assets/models/` with ground-center origin, 1 Blender unit = 1 world unit. Original dark-fantasy designs only — do not recreate proprietary Diablo assets.
