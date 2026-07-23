## Summary

Describe what changed and why.

## Testing

Please list commands run:
- [ ] cargo fmt --check
- [ ] cargo check --locked
- [ ] cargo clippy --all-features -- -D warnings
- [ ] cargo test --locked
- [ ] BEVY_OPEN_ARPG_HEADLESS_SMOKE=1 BEVY_OPEN_ARPG_AUDIO=0 cargo run
- [ ] bash scripts/build_web.sh

## Checklist

- [ ] I updated `CHANGELOG.md` (if user-facing behavior changed).
- [ ] I updated README or docs when adding controls/content.
- [ ] I added or updated asset manifest assertions where needed.
