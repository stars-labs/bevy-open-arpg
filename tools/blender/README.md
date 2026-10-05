# Blender Asset Pipeline

`generate_assets.py` dispatches the canonical character, prop, environment and restrained combat-effect generators. There is no primitive-model fallback.

Run:

```bash
nix develop -c blender --background --factory-startup \
  --python tools/blender/generate_assets.py
nix develop -c blender --background --factory-startup \
  --python tools/blender/postprocess_glb_materials.py
```

`generate_reliquary_enemies.py` owns enemy characters; `generate_enemy_roster.py` owns the four Keeper/Matron skill effects. `generate_reliquary_effects.py` owns all contact, death, aura and motion effects. Health/fury phials, coin purses and broken armour remains come from `generate_reliquary_props.py`.

## Deterministic material post-processing

Always run `postprocess_glb_materials.py` after any generator. It processes
every runtime GLB in sorted order and embeds the 512 px grayscale detail maps
from `assets/textures/materials/`. Stable authored material names select bone,
cloth, metal, or stone detail; any unknown non-emissive name fails validation
until it is classified. The node graph exports as
`baseColorTexture * baseColorFactor`, so each original color remains the tint.
Emissive materials and the explicit `VFX_GLBS` whitelist in the script stay
texture-free.

Closed characters and props enable back-face culling. Only the named
burst/ring/trail, affix, portal, and Boss-skill VFX whitelist remains
double-sided. `keeper_cleaver_wave.glb` is intentionally on that list: its
procedural arc has degenerate UVs, but it is an opaque emissive gameplay
telegraph and never receives a sampled detail texture.

Use Blender 5.1 and verify repeatability after changing the pipeline:

```bash
nix develop -c blender --version
nix develop -c blender --background --factory-startup \
  --python tools/blender/postprocess_glb_materials.py
find assets/models -name '*.glb' -print0 | sort -z | xargs -0 sha256sum \
  > /tmp/glb-material-pass-1.sha256
nix develop -c blender --background --factory-startup \
  --python tools/blender/postprocess_glb_materials.py
find assets/models -name '*.glb' -print0 | sort -z | xargs -0 sha256sum \
  > /tmp/glb-material-pass-2.sha256
diff -u /tmp/glb-material-pass-1.sha256 /tmp/glb-material-pass-2.sha256
```

Asset rules:
- Export `.glb` files to `assets/models/`.
- Keep the character origin at ground center.
- Use one Blender unit as one Bevy world unit.
- Author characters facing Blender `-Y`; glTF export maps this to the game's
  forward direction.
- Export one embedded GLB scene at index zero (`#Scene0`) with transforms
  applied and Y-up conversion enabled.
- Keep materials fully opaque. Use emissive color for glow; do not enable
  alpha blending on exported PBR materials. Run the material post-processor
  to apply the repository's texture and culling policy.
- Use original dark fantasy silhouettes; do not recreate proprietary Diablo assets.
- Character exports include named hit bones plus hidden contact-zone markers for head, chest, weapon, left, and right VFX placement.
- Preserve `hit_bones`, `hit_head`, `hit_chest`, `hit_weapon`, `hit_left`, and
  `hit_right`, together with each matching `*_marker` and `*_contact_zone`.
- Roster exports also include `socket_weapon`, `socket_cast`,
  `socket_head_fx`, `socket_chest_fx`, and `socket_ground_fx`. They are
  metadata hooks and contain no rendered geometry.
- Combat VFX exports include `slash_arc.glb`, `hit_spark.glb`, `bone_shatter.glb`, `bone_impact.glb`, `blood_spray.glb`, `execution_burst.glb`, `arcane_impact.glb`, `holy_impact.glb`, `ember_impact.glb`, `frost_impact.glb`, `void_impact.glb`, `frenzy_impact.glb`, `vampiric_siphon.glb`, `desecrator_burst.glb`, `guard_clash.glb`, `armor_break.glb`, `soul_ward_hit.glb`, `hit_bone_rune.glb`, `hit_bone_lock.glb`, `marrow_flash.glb`, `bone_fracture_echo.glb`, `elite_affix_break.glb`, `shadow_burst.glb`, `headshot_burst.glb`, `crit_bone_crown.glb`, `crit_burst.glb`, `stagger_burst.glb`, `shadow_trail.glb`, and `loot_prism.glb` for Bevy hit feedback, dash trails, stagger windows, affix readability, hit-bone overlays, bone-point locks, fracture echoes, and high-value death rewards.
- If Blender MCP is connected later, keep these output filenames stable and replace only the generator implementation.

## Authored replacement models

`generate_assets.py` routes character and prop generation to the dedicated
`generate_hero.py`, `generate_reliquary_enemies.py` and
`generate_reliquary_props.py` pipelines. `generate_reliquary_environment.py`
produces weathered rocks, branching dead trees, crystal clusters, broken arches
and carved sanctum gate pillars. `generate_reliquary_effects.py` replaces bulky
auras and combat markers with ground inscriptions and curved ribbons. Use
Blender 5.1 for the canonical material pass. Generated previews and verification
reports live under `target/validation/`.

The hero body contains no baked boots or sword: `hero_arm`, `hero_leg` and
`hero_mantle` are authored components driven by the runtime animation. Enemies
expose four named animated limb pivots. Contact markers remain non-rendering
metadata; they must never become visible spheres or discs on character models.
