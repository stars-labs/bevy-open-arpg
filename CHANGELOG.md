# Changelog

- Promote directed summons into the selectable Soulbinder class, with six command skills, two talent paths, authored hero and minion assets, minion survival, save persistence, and class-aware UI.

- Add three selectable hero classes with distinct basic attacks, passives, resources, models, six active skills each, rune loadouts, level unlocks, and native save persistence.

- Add readable skill release silhouettes: directional heavy cuts, expanding shockwaves, tapered movement trails, and charged ground impacts, with rune palettes and real strike timing.

- Fix HUD startup query conflicts for class resource liquids and skill icons; remove unused UI helpers and keep test-only combat constructors out of runtime builds.

- Connected talent investment to combat: Wrath increases impact recoil, Vigor grants a shield once per successful skill cast, and Focus adds Fury on a successful strike. Paragon ranks award talent points and show their real rewards.
- Aligned melee damage with weapon contact and attack speed; added a brief contact hold, drag and release, with bounded boss recoil and no stacked pause for multiple targets.
- Added distinct animated contact art and material sounds for all six skill runes; removed redundant slash overlays and obsolete hit-flash code.

- Added a Skills & Talents panel with clickable rune choices, their combat effects and current costs/cooldowns; talent ranks can be purchased with the mouse.
- Level-ups now show actual rewards with a short visual/audio cue, restore life, and keep an actionable reminder until talent points are spent. Loading a save does not replay level-up rewards.

- Refined inventory and talent panels with readable localized headings, unclipped item names, responsive scrolling and close buttons; added north-up map directions, a player-facing arrow and a live objective-bearing arrow.

- Rebuilt the combat HUD around resource orbs and icons; removed diagnostic strips and distant loot comparisons, added a readable Chinese journal with scrolling and a close button, and moved secondary details into hover tips.

- Corrected the sword grip rotation so the blade leads forward during melee contact; moved the short strike trail from the hilt to the blade.

- Planted the normal sword swing: removed whole-body backward tilt, jump/stretch and blade enlargement; constrained the hand to arm reach.

- Changed mouse controls to right-click movement and left-click attacks; Rupture uses R, with updated HUD hints and move-to-cancel targeting.

- Replaced instant enemy disappearance with an inert falling corpse that settles and clears; removed solid death discs and normal-enemy death flashes.
- Replaced restorative spheres and floating coin stacks with authored phials, coin purses and small broken-armour remains.
- Rebuilt contact and boss effects without opaque cores or radial blocks; enemy projectiles and hazards use directional bolts and sparse ground inscriptions.

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
- Moved language selection into a persistent Settings panel available from the title screen and pause menu; removed the F8 language shortcut.
- Replaced the verbose defeat report with a framed results panel, concise advice, and clickable retry/menu actions.
- Reduced basic attacks to one weapon-synchronized slash and one small contact spark, removing stacked trails and ground glyphs.
- Added default Simplified Chinese UI with a bundled CJK font and an English language switch.
- Reworked Rupture into a planted melee slash; reduced its base Fury cost to 20, raised starting Fury to 60, and stopped empty casts consuming Fury.
- Corrected enemy focus rings and jailer circles, and replaced solid attack-warning lanes with hollow outlines.
- Kept animated thighs anchored inside the hip sockets through walking and evasion; replaced body-obscuring hit spheres and block afterimages with brief directional sparks.
- Replaced primitive character, enemy, interactable and wilderness stand-ins with authored textured models; animated the actual limbs and removed duplicate boots, proxy extremities and solid loot beams.
- Anchored resource orbs and skills to a shared bottom HUD, replaced flattened resource fills with circular level masks, and shortened status cards to prevent text overlap.
- Stopped ordinary walking from spawning dash-trail geometry and removed the reused dash-model footprints that looked like dropped matchsticks.
- Simplified resource-orb and skill-card labels, corrected their artwork layering, and kept pause summaries behind inventory and talent panels.
- Removed unused GLSL shader imports from native builds to avoid an upstream debug shader-validation panic on WGSL texture binding arrays.
- Restyled interface typography with local Cinzel and Crimson Pro fonts, rebuilt the title screen around brass-and-ember controls with mouse interaction and contextual hints, and replaced font-glyph icons with image-backed navigation symbols.
- Preserved Bevy's default GPU features and device priority so native startup supports HDR bloom and atmospheric scattering without texture validation errors.
- Fixed native startup compilation after the rand 0.10 and rodio 0.22 upgrades by updating random-number imports and audio playback APIs and enabling Rodio playback.
- Added a data-driven two-chapter narrative with 18 mainline scenes, ordered multi-character dialogue, pre-Boss, half-health, and post-fight exchanges, and a final victory resolution. The `J` journal now combines tasks, story history, and lore; loading clears transient dialogue state and restores recorded progress without stale lines. Mainline caches and altars are phase-gated so early exploration cannot consume an objective or scramble its story beats.
- Added four repeat-sampled imagegen terrain textures for chapter-specific ground and roads under `assets/images/generated/world/`.
- Post-processed all 18 core character and Boss GLBs with embedded bone, cloth, metal, or stone detail textures and back-face culling, plus a deterministic Blender 5.1 rebuild command and explicit double-sided VFX exceptions.
- Converted the six generated core UI atlases to transparent half-resolution RGBA, changed the main-menu artwork to aspect-preserving cover cropping, and added the generated UI, terrain, material-detail, model, font, and VFX assets to the windowed visual `Loading` gate. Headless smoke mode now keeps its marker resource, registers a CPU image loader for textured GLBs, starts gameplay independently of asynchronous GPU readiness, and propagates failed smoke validation as a non-zero process exit.
- Replaced the enclosed arena with an unbounded, deterministic wilderness that streams 48 m chunks around the player, unloads distant terrain, recreates seeded biome dressing on return, preserves an authored main-quest origin region, and migrates movement, fog, navigation, local-radar UI, actors, objectives, and legacy saves away from fixed world bounds.
- Added deterministic wilderness encounters that recreate chapter-specific formations from the world seed and chunk coordinate, scale pack size, affixes, and distance pressure with difficulty, and unload with their home chunks.
- Reworked both chapter rosters around distinct vanguard, skirmisher, caster, marksman, juggernaut, warden, treasure, nemesis, and boss roles with role-specific movement and signature attacks. Difficulty now also scales movement speed, attack cadence, projectile speed, warning windows, signature frequency, and wilderness pack pressure instead of only health and damage.
- Added three signature skills per boss: Malrec uses Ashen Guillotine, Crucible Rings, and Keeper's Cage; Nyxara uses Soul Fissure Choir, Soul Lance, and Hollow Crescendo. Higher difficulties accelerate their skill cycles, and stagger interrupts active boss hazards.
- Added `tools/blender/generate_enemy_roster.py` as a reproducible Blender pipeline for the expanded enemy silhouettes and boss-skill VFX.
- Replaced runtime transparent PBR effects with opaque WebGPU-safe materials, aligned boss hazard hit areas with their Blender telegraphs, and queued capped wilderness encounters for retry instead of dropping them.
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
