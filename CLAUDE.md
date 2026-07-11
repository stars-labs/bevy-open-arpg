# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

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
