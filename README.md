# Bevy Open ARPG

Original dark-fantasy ARPG prototype built with Bevy.

## Running

```bash
cargo run
```

Development debug metadata, asset file watching, Bevy camera-controller tooling, and dynamic linking are behind an explicit feature so they do not ship in the default build:

```bash
cargo run --features dev_tools
BEVY_OPEN_ARPG_DEBUG_GIZMOS=1 cargo run --features dev_tools
BEVY_OPEN_ARPG_DIAGNOSTICS=1 cargo run --features dev_tools
cargo run --features dev_tools -- --free-camera
```

## Web build (WebGPU / GitHub Pages)

The game also builds for `wasm32-unknown-unknown` and runs on WebGPU in the browser:

```bash
bash scripts/build_web.sh      # release wasm + wasm-bindgen + wasm-opt → web/
python3 -m http.server -d web  # then open http://localhost:8000 in Chrome/Edge
```

The wasm build uses `--no-default-features --features webgpu`: the `native` feature (on by default) carries the Bevy integrations that only build on native targets (`accesskit_unix`, `basis-universal`/`compressed_image_saver`, `raw_vulkan_init`), while `webgpu` adds Bevy's `web`, `webgpu`, and `web_asset_cache` support. On web, the tuning RON data is embedded in the binary and save slots / chapter-record profiles are not persisted. Combat sound cues play through bevy_audio (the browser's Web Audio API) instead of the native rodio mixer thread; the browser unlocks audio after the first click/keypress per its autoplay policy.

WebGPU also can't run some of the native render stack, so these are `native`-only and the web build renders without them: PCSS soft shadows (its WGSL samples inside non-uniform control flow), the six extra `pbr_*` material textures (24 fragment samplers vs. WebGPU's 16 per-stage cap), and volumetric fog (produces a solid black frame — bisected via headless Chrome). Distance fog, atmosphere scattering, bloom, SMAA, SSAO, contact shadows, depth of field, vignette, and chromatic aberration all stay on.

`.github/workflows/deploy-wasm-pages.yml` builds the bundle on every push and PR, publishes it as the rolling `web-latest` GitHub Release (or a `v*` tag release), and deploys that exact release artifact to GitHub Pages from the `gh-pages` branch.

## GitHub Publishing

The repository ships one publish pipeline (`.github/workflows/deploy-wasm-pages.yml`).

- Pushes to `main` publish a rolling **`web-latest`** prerelease and deploy the bundle to Pages.
- Pushing a tag (`v*`) publishes a versioned release and deploys the same assets to Pages.
- Manually dispatching the workflow is also supported:
  - `release_tag=vX.Y.Z` (must start with `v`) creates a versioned release and also builds
    native bundles.
  - Empty `release_tag` (default) creates/updates `web-latest` web-only preview.
- On versioned releases (`v*`), the workflow also builds and attaches native release
  bundles for:

- `bevy-open-arpg-<tag>-linux-x86_64.tar.gz`
- `bevy-open-arpg-<tag>-windows-x86_64.zip`
- `bevy-open-arpg-<tag>-macos-x86_64.tar.gz`

- Every web release also ships:
  - `web-dist.zip.sha256`
  - `web-release-manifest.csv`
- `bevy-open-arpg-<tag>-linux-x86_64.tar.gz.sha256`
- `bevy-open-arpg-<tag>-windows-x86_64.zip.sha256`
- `bevy-open-arpg-<tag>-macos-x86_64.tar.gz.sha256`

The tarball contains:

`bevy-open-arpg` (Linux/macOS native binary)
- `README.md`
- `Cargo.toml`

Create a normal version release by tagging and pushing:

```bash
git tag -a v0.1.0 -m "Release v0.1.0"
git push origin v0.1.0
```

For a quick web-only smoke release, trigger GitHub Actions manual dispatch with no
`release_tag` (this publishes `web-latest` only).

Or run from CLI:

```bash
gh workflow run deploy-wasm-pages.yml -f release_tag=v0.1.0
```

The default build is audited against Bevy's cargo feature list. It uses Bevy 0.19's `3d`, `ui`, `scene`, `picking`, and `audio-all-formats` feature collections, plus explicit animation/morph support (`bevy_animation`, `gltf_animation`, `morph`, `morph_animation`), UI widget/focus support (`bevy_ui_widgets`, `bevy_input_focus`), mesh/UI picking backends, native platform basics (`accesskit_unix`, `bevy_clipboard`, `default_font`, `multi_threaded`, `webgl2`), asset processor support, broad local image/texture import (`basis-universal`, `bmp`, `dds`, `exr`, `ff`, `gif`, `ico`, `jpeg`, `ktx2`, `pnm`, `qoi`, `tga`, `tiff`, `webp`, `zlib`, `zstd_rust`), PBR material texture support, area-light/DFG/SMAA/tonemapping/blue-noise LUTs, PCSS soft shadows, native Vulkan initialization, shader formats, system font discovery, system clipboard integration, settings/remote support, reflected settings documentation, reflected function helpers, UI debug metadata, and explicit native X11/Wayland/window input/accessibility support. The runtime uses those features for Blender-authored animated glTF/PBR scenes and morph-capable monsters, generated 2D concept/menu art, HDR bloom, SMAA, atmosphere/environment lighting, screen-space ambient occlusion, depth of field, subtle chromatic aberration, dark-fantasy vignette, distance and volumetric fog, rectangular area lights, contact and soft shadows, mesh/UI picking, UI, audio cue decoding, compressed texture readiness, gamepad input, Linux AccessKit accessibility integration, persisted settings, Bevy Remote/tooling schema introspection, and copying a run/debug summary to the OS clipboard. Startup and F6 clipboard diagnostics report this as grouped `profiles`, `animation`, `ui`, `assets`, `render`, `post_process`, `picking`, `platform`, `tools`, `omitted`, `deferred`, and `dev_tools` capability lines.

Not every Bevy feature is useful for this desktop ARPG. Remote HTTP/HTTPS asset loading, experimental Solari/DLSS/meshlet rendering, Tracy tracing, hotpatching, and C-backed `zstd_c` are left out of the default build because they add security surface, external SDK/tooling assumptions, or instability without improving the normal playable path. `bevy_feathers`, `clipboard_image`, and `pan_camera` are also left out until there is a concrete in-game screen, image clipboard workflow, or 2D map camera that uses them.

Optional Bevy runtime integrations are deliberately opt-in:

```bash
BEVY_OPEN_ARPG_REMOTE=1 cargo run
BEVY_OPEN_ARPG_ASSET_MODE=processed cargo run --features dev_tools
cargo run --features ci
cargo run -- --remote
cargo run -- --asset-mode=processed
```

`BEVY_OPEN_ARPG_REMOTE=1` starts Bevy Remote HTTP for tooling/integration tests. `BEVY_OPEN_ARPG_ASSET_MODE=processed` switches the Bevy `AssetPlugin` to processed assets and, with `dev_tools`, the app explicitly enables Bevy's `file_watcher` hot reload for imported assets. `--features ci` enables Bevy's CI testing plugin.
`BEVY_OPEN_ARPG_DIAGNOSTICS=1` with `--features dev_tools` enables Bevy FPS, entity-count, system CPU/memory, render diagnostics logging, source-location tracking, schedule data, detailed trace spans, and Chrome trace support.
`bevy_settings` persists player-facing runtime preferences such as audio mute and debug visuals without saving window position. `reflect_documentation` and `reflect_functions` are enabled by default because the startup diagnostic and F6 clipboard summary now audit reflected settings docs and reflected debug helpers; this catches feature drift when the Bevy feature list changes.

For CI or no-window smoke tests:

```bash
BEVY_OPEN_ARPG_HEADLESS_SMOKE=1 BEVY_OPEN_ARPG_AUDIO=0 cargo run
cargo run -- --headless-smoke --no-audio --smoke-frames=30
```

On Linux, Bevy Open ARPG defaults Bevy's renderer to a conservative WGPU compatibility profile to avoid driver-specific Vulkan startup stalls. If you need to override it while debugging, set:

```bash
BEVY_OPEN_ARPG_RENDER_PROFILE=functionality cargo run
cargo run -- --render-profile=functionality
```

There is also a native GL/GLES diagnostic profile. The current 3D/PBR renderer may still reject older GL feature levels, but this is useful for separating "Vulkan hangs" from "no alternate GPU backend is available":

```bash
BEVY_OPEN_ARPG_RENDER_PROFILE=gl cargo run
cargo run -- --render-profile=gl
```

Useful Linux window/backend overrides:

```bash
WINIT_UNIX_BACKEND=x11 cargo run
WINIT_UNIX_BACKEND=wayland cargo run
WGPU_BACKEND=vulkan cargo run
WGPU_SETTINGS_PRIO=compatibility cargo run
cargo run -- --x11 --render-profile=compat
cargo run -- --wayland --render-profile=compat
```

Every desktop run prints a startup line with the selected window mode, X11/Wayland display detection, Winit backend, render profile, audio state, and asset mode. The app now defaults to a popup window, and only enters headless smoke mode when explicitly requested (`--headless-smoke` or `BEVY_OPEN_ARPG_HEADLESS_SMOKE=1`); for CI/no-window runs, keep `--headless-smoke` or `BEVY_OPEN_ARPG_HEADLESS_SMOKE=1`.

## Controls

- `WASD`: move
- `Left Shift`: evade with a short burst of speed and invulnerability
- Gamepad left stick: move
- Gamepad east / right trigger: evade
- `Left Mouse`: strike
- `Right Mouse`: rupture cleave and expose enemies
- `Q`: dash slash
- `E`: ember nova
- `Y`: unleash Nephalem Surge when fully charged
- `N`: cycle Reliquary Sentinel stance
- `H`: trigger the Reliquary Sentinel command pulse
- `C`: cycle rupture rune
- `Z`: cycle dash rune
- `X`: cycle nova rune
- `B`: cycle the attuned legendary codex power
- `7` / `8` / `9`: select an armory loadout slot
- `O`: save the current weapon, charm, rune, and codex attunement to the active armory slot
- `P`: restore the active armory loadout
- `F`: drink potion when it is off cooldown
- Gamepad west: drink potion when it is off cooldown
- `V`: cycle elixir type
- `G`: drink the selected elixir
- `L`: cycle ground-loot labels and beams through All, Rare+, Legendary+, Ancient+, and Primal filters
- `U`: salvage spare inventory gear into gold, ember shards, affix essence, and salvage-cache progress
- `T`: town portal to the quartermaster, then back to the saved return point
- `[` / `]`: cycle equipped weapon from inventory
- `;`: don the strongest bagged piece for every paper-doll armor slot (helm, chest, gloves, boots, amulet, ring)
- `Space`: interact with chapter objectives and the quartermaster
- `I`: open inventory (click a bag slot to equip it; click a worn paper-doll piece to stow it)
- `K`: open build and talent panel
- `J`: open lore journal
- `M`: mute or restore generated combat audio
- Gamepad north: mute or restore generated combat audio
- `F2`: toggle clean or tactical HUD density
- `F4` with `--features dev_tools`: cycle Bevy picking debug overlay
- `F8` with `--features dev_tools -- --free-camera`: grab/release debug free-camera cursor
- Arrow keys / PageUp / PageDown with `--features dev_tools -- --free-camera`: move the debug free camera
- `F3`: toggle debug combat-area and player-radius gizmos
- `F7`: toggle Bevy UI debug outlines for checking HUD/menu overlap
- `F5`: save slot 1
- `F6`: copy current run/debug summary to the system clipboard
- `F9`: load slot 1 in game or continue slot 1 from the main menu
- `1` / `2` / `3`: spend talent points on Wrath, Vigor, or Focus; each line unlocks masteries at 4 and 8 points (Bloodrush/Carnage, Second Wind/Iron Constitution, Flow/Tempest)
- `4` / `5` / `6`: choose a Reliquary Boon during the boss phase
- `Tab`: change between unlocked difficulties on the main menu
- Gamepad D-pad right: change between unlocked difficulties on the main menu
- `Space` / `Enter`: start, or push the next difficulty after victory
- Gamepad south / start: start, continue after victory, or restart after death
- `Esc`: pause and show the tactical status guide
- Gamepad start: pause and show the tactical status guide
- `R`: replay the current difficulty after death or victory
- Gamepad west: replay the current difficulty after victory

## Current Chapter

Chapter I, "The Ashen Reliquary", is a playable vertical slice:

- Breach the reliquary and defeat the first guard wave.
- Recover three reliquary seal fragments by opening the sealed cache, slaying Seal Warden Vhal in the inner sanctum, and extinguishing the ember altar before Malrec can be summoned; each recovered seal pays gold and ember shards, with the final seal also granting affix essence.
- Track the Chapter I main quest as a five-step chain through breach, outer seal, Seal Warden, final seal, and Malrec in the HUD quest log.
- Open the sealed reliquary cache to unlock the inner sanctum and release a guaranteed upgraded reward.
- Slay Seal Warden Vhal during the cult ambush, then extinguish the ember altar to restore health and potions before Malrec appears.
- Activate the relic blade shrine for a temporary damage and movement buff.
- Activate the gilded fortune shrine for temporary XP, gold, and magic-find drop boosts.
- Activate the storm conduit shrine for temporary pulsing lightning.
- Overload the Ashen Pylon for a stronger short burst of damage, movement speed, barrier, and fury; kills during the overload complete Pylon Reaper.
- Chain two or more shrine blessings to trigger shrine resonance for an immediate barrier and fury burst, completing the Shrine Resonance challenge and milestone.
- Activate the relic shrine, fortune shrine, storm shrine, and Ashen Pylon in one run to complete Shrinekeeper.
- Restore at the renewal well to refill health, barrier, potions, and cleanse burning or jailer control.
- Trigger the cursed shrine for a risky health sacrifice, elite ambush, and upgraded offering; killing all ambushers completes Cursed Pact.
- Awaken the blood obelisk and complete its timed kill rite for gold, ember shards, affix essence, and milestone progress.
- Open the Ember Rift for a timed elite-pack invasion that pays out gold, ember shards, and affix essence when sealed; fast seals with at least 30 seconds remaining earn a bonus cache, an Echo Keystone, and complete Riftbreaker.
- Open the resplendent reliquary vault for a Boss-grade loot roll, gold, and affix essence.
- Open the vault, complete the blood obelisk, and seal the Ember Rift in one run to complete Sealbreaker and unlock the Reliquary Seal milestone.
- Attune the soul ward at altars and the quartermaster; lethal damage revives you at the latest checkpoint up to twice if you can pay the gold penalty, then awakens an Ashbound Nemesis elite.
- Review the soul ward failure reason on the death screen when revive charges or gold run out.
- Clear the chapter without triggering the soul ward to earn the Flawless Victory challenge.
- Clear the chapter without drinking a potion to earn the Untouched Flask challenge.
- Drink three potions while below 30% health to complete Last Stand and prove the potion belt under pressure.
- Defeat Malrec before Ashen Enrage triggers to earn Enrage Denied and the Ashen Duel milestone.
- Defeat Malrec, Keeper of Ash.
- Complete the optional Bounty Board objective for an extra gold, ember-shard, and affix-essence cache; repeated clears rotate each difficulty through enemy hunts, lore recovery, breakable sweeps, champion-pack writs, Treasure Vault writs, and multi-affix elite contracts, and claiming the cache completes Bounty Hunter.
- Complete chapter challenges such as Swift Clear, Treasure Hunter, Treasure Vault, Massacre Rite, Carnage Master, Elite Hunter, Battle Trance, Vaultbreaker, Breaker, Shrine Resonance, Shrinekeeper, Affix Hunter, Affix Codex, Seal Warden, Ashen Threat, Champion Breaker, Nemesis Hunter, Cursed Pact, Ritekeeper, Riftwalker, Riftbreaker, Pylon Overload, Pylon Reaper, Boss Breaker, Soul Sigil, Trophy Cache, Sealbreaker, Set Builder, Set Adept, Ancient Armory, Augmenter, Gem Resonance, Master Gemcutter, Gem Adept, Paragon Awakened, Talent Adept, Quartermaster Patron, Alchemist, Ashen Alchemy, Loot Curator, Untouched Flask, Last Stand, Surge Mastery, Rune Weaver, Armory Adept, Portal Wayfarer, Glory Seeker, Salvage Cache, Sentinel Veteran, Boonbound, Bounty Hunter, Bestiary Scholar, Lorekeeper, Codex Adept, Codex Archivist, Flawless Victory, Enrage Denied, Torment Clear, Primal Cache, Primal Infusion, and Reliquary Conquest for extra victory rewards.
- Earn Chapter Mastery on victory from challenges, bounty completion, recovered lore, full Valor stacks, Boss Break performance, Ashen Enrage denial, and legendary codex unlocks for gold, ember shards, and endgame affix essence; high-completion runs can reach Conqueror, while near-complete endgame clears can reach Paragon.
- Gather gold, weapon drops, potions, XP, level upgrades, Ember Paragon ranks after the chapter level cap, and item affixes.
- Fight alongside a Reliquary Sentinel companion that follows you, automatically strikes nearby enemies, earns XP from kills, and gains ranks for higher damage; cycle Vanguard, Guardian, and Seeker stances for damage, barrier support, or wider hunting and faster companion growth; press `H` to fire a stance-specific command pulse: Vanguard bursts nearby enemies, Guardian grants barrier while striking close threats, and Seeker sweeps a wider hunting radius; rank 3 completes the Sentinel Veteran challenge and milestone.
- Hunt a roaming reliquary champion pack with a multi-affix leader and elite guards in the outer halls; breaking the full pack grants gold, ember shards, affix essence, a visible upgraded gear cache, the Champion Breaker challenge, and the Champion Pack milestone.
- Smash every reliquary urn and offering box for small gold, potion, and fury rewards; clearing them all completes Breaker.
- Earn better loot from elites and a guaranteed legendary-or-better reward from Malrec when the table contains one; Malrec's final hoard is claimed immediately before the victory transition, and his death grants difficulty-scaled Malrec Soul Sigils that can be spent at the quartermaster on Trophy Caches.
- Chase down the reliquary treasure imp before it escapes to earn a large gold, ember-shard, upgraded loot cache, Treasure Hunter challenge credit, an opened Treasure Vault material cache, and the Treasure Fiend and Treasure Vault milestones.
- Defeat soul-bound nemesis elites after ward revivals for ember shards, XP, Nemesis Hunter challenge rewards, and the Nemesis Slain milestone; HUD, pause, victory, and profile summaries track run and lifetime Nemesis kills.
- Build Valor stacks by killing elites, nemesis enemies, and Malrec; the timed stack increases gold and XP rewards while it lasts.
- Build Ashen Threat from sustained kills; elites, nemesis enemies, treasure imps, and Malrec push the meter harder, and each full surge opens a retaliation wave while paying out gold and ember shards. Triggering three surges completes Ashen Threat.
- Pick up health globes from slain enemies for burst healing and stacking Glory, a short combat buff to damage and movement speed; elite enemies and Malrec are more likely to drop them, and collecting three completes Glory Seeker.
- Build fury with basic attacks, then spend it on rupture cleave, dash slash, and ember nova.
- Break Malrec's stagger meter to open a punish window, gain burst fury and barrier, and track Boss Breaks through the HUD and victory recap.
- Charge Nephalem Surge from enemy kills, then unleash it for a timed boost to damage, movement speed, and basic-attack fury generation; kills during the active window extend the Surge, and chaining three completes Surge Mastery.
- Evade through enemy swings, projectiles, reflected affix damage, burns, and hazard pulses with a short speed burst and cooldown.
- Cycle rupture runes to expose enemies for stronger follow-up hits or hemorrhage them with damage over time.
- Equip common, rare, legendary, ancient, and primal weapons with damage, crit, health, and armor affixes; crit can spike skill damage and armor mitigates enemy and hazard damage.
- Read ground loot at a glance through rarity-colored item labels, rotating item pulses, glow lights, and beams for rare, legendary, ancient, and primal drops; cycle the loot filter to hide lower-rarity labels, beams, and glow lights, completing Loot Curator after three filter tiers.
- Find legendary powers such as Emberbrand, Frostbrand, Stormbrand, Soulreaver, and Aegisbrand, which ignite, chill, chain lightning, steal health, or grant barrier on critical hits and on matching skill runes such as Ember Nova, Frost Nova, Reap Dash, Hemorrhage Rupture, Cleanse Dash, and Expose Rupture; ancient and primal drops include offensive and defensive endgame chase items.
- Find socket gems from loot drops; ruby, emerald, amethyst, and topaz gems add damage, crit, health, or armor to the equipped weapon, rank 3 gems awaken Resonant bonus power and complete Gem Resonance, rank 5 gems become Ascendant, rank 7 gems become Paragon-style legendary gems, and socketing all four kinds in one run completes Gem Adept.
- Legendary weapons can consume a rank 4+ socketed gem, gold, ember shards, and affix essence at the quartermaster to awaken into ancient weapons; ancient weapons can consume an Echo Keystone, gold, ember shards, and affix essence to primal-infuse into a stronger primal weapon; ancient and primal weapons can consume a rank 3+ socketed gem for a permanent Ancient Augment to damage, crit, health, or armor, completing the Augmenter challenge; equipping a rank 5 socketed gem completes Master Gemcutter.
- The quartermaster can perform Ashen Alchemy recipes that transmute gold into ember shards, refine shards into affix essence, and condense shards plus essence into Echo Keystones; opening a Malrec Trophy Cache consumes two Soul Sigils for gold, shards, essence, and an Echo Keystone; completing all three recipes in one run completes Ashen Alchemy.
- Find and auto-equip charms as a separate gear slot for extra damage and critical chance.
- Trigger Reliquary Resonance bonuses when high-quality weapons and charms are equipped together; matching Storm, Blood, Ashen, or Reliquary themes unlock stronger named set-style resonances, slaying enemies while the themed resonance is active completes Set Adept, and clearing with each themed resonance fills the profile Set Collector deed.
- Keep found weapons in a limited inventory while higher-scoring gear auto-equips based on damage, crit, health, armor, and quality.
- Manually cycle equipped weapons from the inventory to trade damage, armor, crit, health, and legendary powers.
- Save and load player progress, Ember Paragon rank and XP, companion rank, XP, stance, and command cooldown, fury, barrier, evade state, Nephalem Surge charge, active window, Surge kill progress, Ashen Pylon kill progress, Ashen Threat meter and surge progress, boss break count, Primal Ember Cache rewards, Echo Keystones, claimed primal-cache items, Malrec Soul Sigils, opened Soul Sigil Trophy Caches, primal-infusion count, Treasure Vault openings, Nemesis kills, champion pack progress, cursed ambush progress, breakable smash progress, health-globe pickup progress, Last Stand potion progress, shrine resonance progress, Ashen Alchemy recipe progress, loot filter progress, Codex Adept kill progress, Set Adept kill progress, Gem Adept socket progress, Affix Hunter progress, Affix Codex mask progress, Reliquary Altar start bonuses, and whether Malrec enraged, soul ward checkpoint and revive count, shrine and Ashen Pylon buffs, selected and active elixir type, used elixir types, potion-use count, attuned codex power, three armory loadout slots, Glory stacks, Valor stacks, talents, inventory, legendary equipment, socketed gems, charms, gold, ember shards, affix essence, ancient augment count, salvage cache progress, kills, chapter state, blood obelisk and Ember Rift states, bounty progress, challenge progress, mastery rank, difficulty, active ordeal and rotating ordeal affix, lore journal, milestones, bestiary, used objectives including the renewal well, and smashed breakables with a local RON save file; best-clear records, per-difficulty clear counts, lifetime kills, lifetime gold, best rating, hero legacy combat records, failed-run Soul Remnant totals, and shared stash material banks are stored separately in `saves/profile.ron`.
- Choose Normal, Nightmare, Hell, or Torment before starting; higher difficulties scale enemy health, enemy damage, XP, and gold.
- Difficulties unlock from persistent profile clears: Normal is open by default, then Nightmare after a Normal clear, Hell after a Nightmare clear, and Torment after a Hell clear.
- After victory, replay the current difficulty with `R` or push to the next difficulty with `Space` / `Enter`; the victory recap calls out the newly unlocked next difficulty, and Torment clears stay on Torment for repeated endgame runs.
- Each difficulty activates a base chapter ordeal: Ashen Echoes, Blood Tithe, Emberstorm, or Torment Brand modify enemy toughness, incoming damage, hazard damage, and rewards.
- Repeat runs add a visible rotating ordeal affix such as Ashen Hunger, Glass Relics, Treasure Fever, or Cinder Veins; affixes alter enemy pressure, hazard damage, or reward scaling so farming the same difficulty does not feel identical.
- Defeat Malrec on Torment difficulty to complete the Torment Clear challenge.
- Clear Torment to claim a Primal Ember Cache with a guaranteed primal weapon reward; S/A ratings, repeated Boss Breaks, denying Ashen Enrage, and carrying an Echo Keystone from a swift Ember Rift increase the payout, with the Keystone adding a second primal-cache item.
- Clear Torment to engrave a Reliquary Sigil tier from 1-12 based on clear grade, Boss Breaks, Primal Cache performance, Ashen Threat surges, massacre and Valor pressure, Paragon socketed gems, and legendary codex completion; tier 10+ counts as a perfect inscription for long-tail Season Deeds.
- Earn a chapter rating on victory based on difficulty, clear time, and kills.
- Track persistent best chapter clears, clear counts, total clears, lifetime kills, lifetime gold, lifetime ember shards, lifetime affix essence, best rating, best Chapter Journey score/tier, highest cleared difficulty, Torment clear count, lifetime Boss Breaks, Primal Ember Caches, Primal Infusions, Echo Keystones, Ashen Threat surges, Ashbound Nemesis kills, Treasure Vault openings, Bounty Board cache claims, themed set clears, flawless clears, potionless clears, best socketed gem rank, Ascendant-gem clears, Paragon-gem clears, Reliquary Sigil best tier, Sigil clear count, perfect Sigil count, Affix Codex completions, best legendary codex power count, full legendary-codex clears, Malrec Soul Sigils, Trophy Cache openings, cleared rotating ordeal affixes, best massacre streak, best Valor stack, best boss-break count, failed-run Soul Remnants, and shared stash material banks from the main menu and victory recap with a local profile file.
- Bank a share of each victory's gold, ember shards, and affix essence into the shared profile stash; future runs draw a capped starting grant from that stash, surfaced in the pause guide and victory recap.
- Recover a small Soul Remnant on meaningful failed runs based on time survived, kills, difficulty, and collected materials; the death screen banks it into the shared profile stash so failed attempts still nudge the next run forward.
- Complete profile-level Season Deeds for first clear, Nightmare-or-higher clear, Torment clear, repeated Boss Breaks, Primal Ember Cache claims, repeated Primal Infusions, Ashbound Nemesis hunts, Treasure Vault hunts, repeated Bounty Board cache claims, clearing every themed set resonance, repeated flawless clears without soul ward revival, repeated potionless clears, a 20-kill massacre paired with full Valor, repeated S-grade Chapter Clears, repeated Ascendant-gem clears, repeated Paragon-gem clears, repeated Affix Codex completions, repeated Malrec Trophy Cache openings, clearing every rotating ordeal affix, shared-stash banking, repeated Soul Remnant recovery, clearing with all three Reliquary Boons, repeated full legendary-codex clears, Chapter Journey score, and repeated perfect Reliquary Sigil inscriptions; completed deeds automatically pay extra shared-stash materials in the victory recap or death recap.
- Convert persistent profile progress into Reliquary Renown ranks; new runs start with small gold, ember-shard, and affix-essence grants based on the account profile, with the active run's Renown grant surfaced in the HUD, pause guide, and victory recap.
- Light Reliquary Altar account seals from core milestones such as first clear, Torment, Primal Cache, Paragon gem, complete codex, all three boons, perfect Sigil, and full Season Deeds; lit seals add an extra gold, shard, and essence start bonus to every new run and are shown in profile, shared stash, pause, and victory summaries.
- Earn profile Legacy Titles such as Reliquary Seeker, Torment Vanquisher, Primal Paragon, and Season Conqueror, with compact badges for S-grade clears, Torment, Primal crafting, Ascendant gems, Paragon gems, Reliquary Sigils, perfect Sigils, set mastery, rotating ordeals, Chapter Journey, full Reliquary Boon coverage, complete legendary codex clears, and full Season Deeds shown in profile and hero legacy summaries.
- Track a Chapter Journey tier from completed challenges, unlocked milestones, and mastery points, with HUD, pause, and victory recap summaries showing the current tier, score, next tier, next high-value pursuit, and a once-per-clear journey reward; HUD and pause guidance also call out the next Primal Infusion step from legendary, ancient, Echo Keystone, and material states.
- Review a multi-line victory recap with run stats, best-clear records, chapter and mastery rewards, a Chapter Clear grade that pays extra gold, shards, and essence, challenge progress, latest milestone, and mastery rank.
- Track best Chapter Clear grade, lifetime S-grade clears, best socketed gem rank, lifetime Ascendant-gem clears, lifetime Paragon-gem clears, Reliquary Sigil best tier, Sigil clear count, perfect Sigil count, the three Reliquary Boons cleared with, best legendary codex power count, and full legendary-codex clear count in the account profile and hero legacy summary.
- Pause into a tactical status guide that summarizes the current objective, difficulty, ordeal, combat kit, progression systems, and active chapter pursuits.
- Follow a dynamic action guide in the HUD and pause screen that prioritizes low-health potion prompts, active Surge windows, timed rift or obelisk events, main quest actions, and optional pursuits.
- See center-screen chapter banners when the route advances through breach, cache, sanctum ambush, final seal, Malrec, and victory beats.
- Unlock chapter milestones for cache, resplendent vault, Breaker, relic shrine, fortune shrine, storm conduit shrine, Shrine Resonance, Shrinekeeper, Affix Hunter, Affix Codex, Seal Warden, Ashen Threat, Champion Pack, Ashen Pylon, Pylon Reaper, Boss Breaker, Soul Sigil, Trophy Cache, Primal Ember Cache, renewal well, cursed shrine, Cursed Pact, blood obelisk completion, Ember Rift completion, Riftbreaker, Reliquary Seal, Treasure Fiend, Treasure Vault, bounty completion, lore, Ember Paragon ranks, talent investment, quartermaster logistics, using all elixir types, Ashen Alchemy, Loot Curator, Last Stand, Salvage Cache, Sentinel Veteran companion growth, claiming a Reliquary Boon, tempering, legendary gear, ancient-or-better weapons, ancient augments, primal infusion, gem resonance, master gemcutting, Gem Adept, themed set resonance, Set Adept, Codex Adept, legendary codex completion, massacre streak, Carnage Master, Battle Trance, Surge Mastery, Rune Weaver, Armory Adept, Portal Wayfarer, Glory Seeker, nemesis kills, Ashen Duel, and Malrec; completing every milestone in one run earns the Reliquary Conquest challenge.
- Claim a one-time chapter completion reward of gold, ember shards, and affix essence scaled by rating, difficulty, and recovered lore.
- Chain kills within the massacre window to earn bonus XP, bonus gold, best-streak tracking, Massacre Rite at five kills, and Carnage Master at ten kills.
- Maintain elite Valor stacks to increase gold and XP rewards during sustained high-pressure fights; reaching full Valor also adds a Chapter Mastery point, and pairing full Valor with a five-kill massacre completes Battle Trance.
- Use fortune shrine magic find to push normal enemy drops toward upgraded non-common loot while the blessing lasts.
- Follow world-space objective rings and floating beacons for current chapter objectives, reusable services, and optional events.
- Track player position, enemies, loot, and active objectives on the in-game minimap.
- Read the HUD chapter route tracker for Breach, Cache, Sanctum, Ritual, Keeper, and Cleansed progress, including mainline percentage, optional objective completion count, and Ashen Threat pressure.
- Read the chapter quest log for a compact main quest, bounty, champion pack, Breaker, blood obelisk, and Ember Rift checklist in the HUD, pause guide, and victory recap.
- Recover three lore pages and review them in the journal; finding every page completes the Lorekeeper challenge.
- Follow a chapter story log that records arrival, seal recovery, sanctum breach, Seal Warden's fall, altar demand, optional cursed shrine, blood obelisk, vault, Ember Rift, Ashen Pylon echoes, Malrec's awakening, and the final cleanse in the HUD, journal strip, pause guide, save file, and victory recap.
- Track defeated monster types and kill counts in the bestiary summary; next-goal hints, target panels, and bottom action prompts teach counterplay for shields, rushers, cultist fire, marksmen, Bonebreaker shockwaves, treasure imps, Malrec stagger/enrage, and nemesis affixes, repeated kills unlock small damage bonuses and one-time trophy rewards at 3, 8, and 15 kills for each known enemy type, and recording all eight chapter monster roles completes the Bestiary Scholar challenge.
- Trade with the reliquary quartermaster to sell spare gear, salvage ember shards and legendary affix essence from high-quality spares, earn quartermaster caches after enough spare gear is broken down, open Malrec Trophy Caches with Soul Sigils, restock potions, upgrade potion and elixir belts, shorten potion cooldowns, expand inventory capacity, reforge stats, polish equipped charms, gamble mystery charms and weapons, awaken legendary weapons into ancient weapons, primal-infuse ancient weapons with Echo Keystones, ancient-augment socketed weapons with affix essence, empower Ascendant socketed gems into Paragon legendary gems with Echo Keystones, and enchant the equipped weapon with a legendary power unlocked in the codex; mystery weapons can also unlock missing codex powers, and upgrading stash, potion belt, and elixir belt completes the Quartermaster Patron challenge.
- Unlock legendary powers in the codex by finding equipment with those powers, refine duplicate legendary-or-better codex drops into affix essence, attune one unlocked codex power as an extra passive legendary effect, follow its suggested Crimson/Titan/Arcane Boss Boon route, slay enemies with it to complete Codex Adept, then spend gold, ember shards, and affix essence to reuse unlocked powers through quartermaster enchanting; non-legendary weapons become legendary while ancient and primal weapons keep their quality, and repeated clears with every codex power unlocked complete the profile Codex Keeper deed.
- Save and restore three armory loadout slots to quickly swap weapon, charm, skill rune, and codex-attunement builds during a run; saving all three slots completes Armory Adept.
- Use the town portal stone to return to the quartermaster on a cooldown, then return to the saved combat position to complete Portal Wayfarer.
- Choose one Reliquary Boon when Malrec appears: damage and crit, health and armor, or fury economy; strong run performance can empower each boon before the boss, Reliquary Momentum at 3+ stacks pushes the Arcane route and infuses extra Fury economy into that boon, claiming a boon completes Boonbound, and clearing over time with all three boons completes the profile Boon Triad deed.
- Temper the equipped weapon at the quartermaster for escalating gold costs, adding damage, armor, crit, and a visible upgrade level.
- Upgrade socketed gems at the quartermaster for escalating gold costs, then empower rank 5+ gems with gold, affix essence, and Echo Keystones to jump into rank 7 Paragon gem bonuses.
- Activate the cursed reliquary shrine for a blood-price reward and an elite ambush.
- Use the renewal well once per run to reset potion pressure and cleanse dangerous control effects.
- Crack the resplendent reliquary vault for a one-time high-value reward cache.
- Drink potions under cooldown pressure, with low-health drinks tracked toward Last Stand, then upgrade the potion belt for more charges and faster recovery.
- Use Iron, Wrath, and Haste elixirs for temporary armor and barrier, damage, or movement-speed pressure tools.
- Spend level-up talent points on damage, survivability, or cooldown reduction; investing in Wrath, Vigor, and Focus completes Talent Adept.
- Keep earning Ember Paragon XP after level 6; each Paragon rank rotates permanent damage, health, critical chance, and armor gains for long-tail progression, and rank 1 completes the Paragon Awakened challenge and Ember Paragon milestone.
- Cycle skill runes for dash cleanse/reap, rupture expose/hemorrhage, and nova ember/frost variants; landing hits with Reap dash, Hemorrhage rupture, and Frost nova in one run completes Rune Weaver.
- Chapter mobs now cover distinct combat roles: Ashbone Guards hold the line, Ashbone Stalkers rush and punish greed, Ashen Reliquary Marksmen pressure movement with long-range projectiles, Reliquary Bonebreakers telegraph heavy shockwaves, Cinder Acolytes ignite the player, and dash slash clears burning.
- Combat, chapter objectives, and interactable sites use Blender-generated hero and enemy silhouettes, named hit bones, shrines, wells, rifts, pylons, lore pages, breakables, slash arcs, hit sparks, bone shatters, bone-impact bursts, blood sprays, execution blood seals, arcane impacts, holy impacts, ember impacts, frost impacts, void impacts, guard-clash sparks, armor-break shards, soul-ward hits, hit-bone runes, hit-bone locks, marrow flashes, fracture echoes, shadow bursts, headshot bursts, critical bone crowns, critical bursts, dash shadow trails, stagger bursts, objective sigils, ember vents, boss summoning seals, pulsing elite affix aura rings with colored threat lights, and high-value loot prisms for readable hit, movement, hazard, mainline, boss-break, exploration, elite-pressure, and death-reward feedback.
- Key reliquary sites, shrines, rifts, ember vents, and the boss summoning seal cast colored pulsing point lights so threats and rewards read clearly in the isometric dungeon.
- Combat events play generated, gain-balanced audio cues for hits, criticals, loot, danger warnings, enemy deaths, victory, and defeat, with boss and clear cues mixed above common hit spam.
- Hit feedback is placed at named hit-bone contact zones such as head, chest, weapon, left, and right so criticals, skeletons, guarded enemies, arcane elites, jailer/frozen enemies, vampiric or desecrator enemies, cultists, Seal Warden Vhal, and bosses read differently in motion.
- Critical hits and important elite, treasure, nemesis, and boss kills trigger short camera shake for heavier combat impact.
- Malrec uses a close-range ember shockwave with a visible ground marker.
- Build Malrec's stagger meter with direct skill hits, then punish the exposed Keeper during a short bonus-damage window.
- Malrec enters a second phase below half health, summons an affixed stalker, marksman, and Bonebreaker wave, then escalates into Ashen Enrage with marksman/brute pressure if the fight drags on; the HUD tracks phase, health percentage, stagger, and enrage countdown.
- Ember vents create timed dungeon hazards that punish poor positioning.
- Elite enemies appear with Frenzied, Vampiric, Molten, Shielded, Arcane, Jailer, Frozen, Desecrator, and Reflective affixes that change speed, sustain, death hazards, damage resistance, space control, movement control, corrupted ground pressure, reflected damage pressure, and affix essence rewards; slaying three multi-affix elites completes Affix Hunter, and cataloguing six distinct elite affix types completes Affix Codex.
- Reflective elites return a bounded portion of direct player hit damage after armor mitigation, making barrier, healing wells, Soulreaver, and Aegisbrand matter under burst damage.
- Arcane, Frozen, Desecrator, Jailer, and Molten elite hazards show a visible warning window before becoming damaging, rewarding fast movement, evade timing, and spatial awareness.
- The minimap marks loot, health globes, objectives, normal enemies, and elite enemies with different colors and sizes.
- Enemy health bars, world-space nameplates, and a combat log surface damage, loot, XP, status effects, elite affixes, and objective feedback.

## Assets

Run the Blender generator after changing model scripts:

```bash
blender --background --python tools/blender/generate_assets.py
```

Regenerate local audio cues after changing sound scripts:

```bash
python3 tools/audio/generate_sfx.py
```

The concept image was generated with the prompt in `docs/prompts/bevy-open-arpg-concept.md` and saved at `assets/images/generated/bevy-open-arpg-concept.png`.

## Verification

```bash
cargo fmt --check
cargo check
cargo clippy --all-targets -- -D warnings
cargo test
```
