# Contributing

Thanks for your interest in improving **bevy-open-arpg**.

## Quick start

```bash
cargo fmt -- --check
cargo check --locked
cargo clippy --all-features -- -D warnings
cargo test --locked
```

Before opening a PR, run at least:

```bash
cargo fmt --all
cargo test --locked
```

## Runtime checks

When you add gameplay or rendering behavior, verify at least:

- Headed launch: `cargo run`
- No-window smoke: `BEVY_OPEN_ARPG_HEADLESS_SMOKE=1 BEVY_OPEN_ARPG_AUDIO=0 cargo run`
- Web check: `bash scripts/build_web.sh`

## Pull requests

- Keep changes scoped and include clear test notes in the PR description.
- For gameplay balance changes, include rationale and references to tuning files in `assets/data/*.ron`.
- For generated assets (models/audio), regenerate only via the project scripts:
  - `python3 tools/blender/generate_assets.py` (requires Blender)
  - `python3 tools/audio/generate_sfx.py`

## Reporting issues

Use the issue templates with:
- steps to reproduce
- observed output logs
- platform/backend (`linux x11`, `linux wayland`, `windows`, `wasm`)
- expected vs actual behavior
