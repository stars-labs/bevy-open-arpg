# Blender Asset Pipeline

`generate_assets.py` creates the first-pass `.glb` models and combat VFX used by the Bevy prototype.

Run:

```bash
blender --background --python tools/blender/generate_assets.py
```

Asset rules:
- Export `.glb` files to `assets/models/`.
- Keep the character origin at ground center.
- Use one Blender unit as one Bevy world unit.
- Use original dark fantasy silhouettes; do not recreate proprietary Diablo assets.
- Character exports include named hit bones plus hidden contact-zone markers for head, chest, weapon, left, and right VFX placement.
- Combat VFX exports include `slash_arc.glb`, `hit_spark.glb`, `bone_shatter.glb`, `bone_impact.glb`, `blood_spray.glb`, `execution_burst.glb`, `arcane_impact.glb`, `holy_impact.glb`, `ember_impact.glb`, `frost_impact.glb`, `void_impact.glb`, `frenzy_impact.glb`, `vampiric_siphon.glb`, `desecrator_burst.glb`, `guard_clash.glb`, `armor_break.glb`, `soul_ward_hit.glb`, `hit_bone_rune.glb`, `hit_bone_lock.glb`, `marrow_flash.glb`, `bone_fracture_echo.glb`, `elite_affix_break.glb`, `shadow_burst.glb`, `headshot_burst.glb`, `crit_bone_crown.glb`, `crit_burst.glb`, `stagger_burst.glb`, `shadow_trail.glb`, and `loot_prism.glb` for Bevy hit feedback, dash trails, stagger windows, affix readability, hit-bone overlays, bone-point locks, fracture echoes, and high-value death rewards.
- If Blender MCP is connected later, keep these output filenames stable and replace only the generator implementation.
