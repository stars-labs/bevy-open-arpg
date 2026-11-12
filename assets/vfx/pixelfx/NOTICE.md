# PixelFX attribution

These five generated effect atlases adapt PixelFX by Patch, pinned to commit `044e12a45dc9cc312cec50ee606dfc98afe5adc2`.

Source https://github.com/patch-gg/pixelfx

Copyright (c) 2026 Patch and Hugo Duprez. MIT license, included in LICENSE.

PixelFX's `src/core/sim.js` and the attributed slash, fireball, impact and summon sources adapt Hugo Duprez's Sprite Fusion pixel art VFX, copyright (c) 2026 Hugo Duprez, MIT.

PixelFX's exporter imports acting utilities adapted from Hollow Crown by rehan-remade, copyright (c) 2026 rehan-remade, MIT.

Adaptations by Bevy Open ARPG include dark-fantasy color ramps, release-aligned animation timing, a radial ground-plane frost nova composed from PixelFX crystal and band primitives, and removal of the creature drawing from summon-rise. The source effects are `melee/sword-slash`, `impacts/blunt-hit`, `fire/fireball`, `ice/frost-nova` and `spawn/summon-rise`.

Regenerate with `node tools/vfx/generate_pixelfx.mjs`. The generator fetches the pinned upstream source into `target/pixelfx-upstream` when absent. No npm packages are required.
