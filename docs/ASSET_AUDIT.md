# Runtime visual audit

The audit follows spawn sites as well as GLB files: replacing a character alone does not remove primitives spawned on death, impact or loot.

| Player-visible case | Removed | Current representation | Owner |
| --- | --- | --- | --- |
| Enemy death | Immediate disappearance, filled ground cylinder, normal-death flash | Original model falls, rests, sinks and is removed; small broken armour/bone remains | `enemy_death.rs`, `loot.rs` |
| Dead-enemy decorations | Persistent health/name/focus and elite glow | Decorations removed when combat components are removed | `feedback.rs`, `enemy.rs` |
| Life / fury recovery | Emissive spheres | Red/blue sealed metal-braced phials | `loot.rs`, prop generator |
| Item drop | Floating rotating stack of coins | Grounded leather coin purse and loose coins; fine rarity ring retained | `loot.rs`, prop generator |
| Enemy shots | Undirected coloured sphere | Narrow directional bolt with tip and fletching | `enemy/vfx.rs` |
| Quake, molten death pool, frost, desecration, arcane and binding hazards | Filled cylinders / thick torus | Thin boundary with sparse fracture, frost or binding marks; unchanged gameplay radius and timing | `enemy/vfx.rs` |
| Contact / critical / affix effects | Opaque cores and radial rectangular rods | 22 regenerated sparse tapered-fragment effects; small open arcs where appropriate | effect generator |
| Keeper / Matron skills | Solid fan, columns, cylindrical lance, core and spikes | Narrow crescent, linked chains, tapered spear, broken resonance arcs | boss effect generator |
| Objective marker | Thick cyan tube | Thin boundary ring, authored sigil | `dungeon.rs` |

Intentional runtime geometry remains for textured terrain/roads, world-space health bars, thin target/loot/range outlines and small contact sparks. These communicate gameplay or carry terrain textures; they are not stand-in actor or item models. Authored GLBs may themselves use bevelled or lathed geometry; the audit targets the visible stand-in result, not the modelling primitive used to construct it.

Reproduction: build the native game, then run `python3 tools/ui/capture_ui.py --death-vfx-review --output target/validation/placeholder-audit`. This uses real movement and attacks, captures consecutive kill frames, and waits for corpse cleanup. Asset previews and native captures are complementary: a GLB preview alone does not verify its runtime placement or lifespan.
