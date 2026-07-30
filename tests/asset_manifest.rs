use std::{fs, path::Path};

#[test]
fn chapter_runtime_assets_exist() {
    for path in [
        "assets/data/player.ron",
        "assets/data/enemies.ron",
        "assets/data/loot.ron",
        "assets/audio/ambient_theme.wav",
        "assets/models/hero.glb",
        "assets/models/skeleton.glb",
        "assets/models/cultist.glb",
        "assets/models/butcher.glb",
        "assets/models/sword.glb",
        "assets/models/chest.glb",
        "assets/models/altar.glb",
        "assets/models/quartermaster.glb",
        "assets/models/fortune_shrine.glb",
        "assets/models/storm_shrine.glb",
        "assets/models/healing_well.glb",
        "assets/models/cursed_shrine.glb",
        "assets/models/blood_obelisk.glb",
        "assets/models/reliquary_vault.glb",
        "assets/models/ember_rift_prop.glb",
        "assets/models/ashen_pylon.glb",
        "assets/models/lore_page.glb",
        "assets/models/breakable_urn.glb",
        "assets/models/breakable_coffer.glb",
        "assets/models/slash_arc.glb",
        "assets/models/hit_spark.glb",
        "assets/models/bone_shatter.glb",
        "assets/models/bone_impact.glb",
        "assets/models/blood_spray.glb",
        "assets/models/execution_burst.glb",
        "assets/models/arcane_impact.glb",
        "assets/models/holy_impact.glb",
        "assets/models/ember_impact.glb",
        "assets/models/frost_impact.glb",
        "assets/models/void_impact.glb",
        "assets/models/frenzy_impact.glb",
        "assets/models/vampiric_siphon.glb",
        "assets/models/desecrator_burst.glb",
        "assets/models/guard_clash.glb",
        "assets/models/armor_break.glb",
        "assets/models/soul_ward_hit.glb",
        "assets/models/hit_bone_rune.glb",
        "assets/models/hit_bone_lock.glb",
        "assets/models/marrow_flash.glb",
        "assets/models/bone_fracture_echo.glb",
        "assets/models/elite_affix_break.glb",
        "assets/models/shadow_burst.glb",
        "assets/models/headshot_burst.glb",
        "assets/models/crit_bone_crown.glb",
        "assets/models/crit_burst.glb",
        "assets/models/stagger_burst.glb",
        "assets/models/shadow_trail.glb",
        "assets/models/loot_prism.glb",
        "assets/models/objective_sigil.glb",
        "assets/models/ember_vent.glb",
        "assets/models/boss_summon_portal.glb",
        "assets/models/affix_ember_aura.glb",
        "assets/models/affix_arcane_aura.glb",
        "assets/models/affix_frost_aura.glb",
        "assets/models/affix_blood_aura.glb",
        "assets/models/affix_ward_aura.glb",
        "assets/audio/hit.wav",
        "assets/audio/critical.wav",
        "assets/audio/loot.wav",
        "assets/audio/danger.wav",
        "assets/audio/death.wav",
        "assets/audio/skill.wav",
        "assets/audio/combo.wav",
        "assets/audio/boss.wav",
        "assets/audio/quest.wav",
        "assets/audio/potion.wav",
        "assets/audio/utility.wav",
        "assets/audio/victory.wav",
        "assets/audio/defeat.wav",
        "assets/fonts/FiraSans-Bold.ttf",
        "assets/fonts/FiraSans-Regular.ttf",
        "assets/fonts/FiraSans-License.txt",
        "assets/images/generated/bevy-open-arpg-concept.png",
        "assets/images/generated/ui/gear-icons-reliquary.png",
        "assets/images/generated/ui/main-menu-reliquary-v2.png",
        "assets/images/generated/ui/panel-frame-reliquary.png",
        "assets/images/generated/ui/resource-orb-frame-reliquary.png",
        "assets/images/generated/ui/skill-icons-reliquary.png",
        "assets/images/generated/ui/slot-frame-reliquary.png",
        "assets/images/generated/ui/talent-sigils-reliquary.png",
    ] {
        assert!(Path::new(path).exists(), "missing runtime asset: {path}");
    }
}

#[test]
fn generated_ui_pngs_have_expected_ihdr() {
    for (path, expected_width, expected_height) in [
        (
            "assets/images/generated/ui/main-menu-reliquary-v2.png",
            1_672,
            941,
        ),
        (
            "assets/images/generated/ui/panel-frame-reliquary.png",
            1_536,
            1_024,
        ),
        (
            "assets/images/generated/ui/slot-frame-reliquary.png",
            1_254,
            1_254,
        ),
        (
            "assets/images/generated/ui/talent-sigils-reliquary.png",
            2_172,
            724,
        ),
        (
            "assets/images/generated/ui/skill-icons-reliquary.png",
            1_536,
            1_024,
        ),
        (
            "assets/images/generated/ui/gear-icons-reliquary.png",
            1_776,
            888,
        ),
        (
            "assets/images/generated/ui/resource-orb-frame-reliquary.png",
            1_254,
            1_254,
        ),
    ] {
        let bytes = fs::read(path).unwrap();
        assert!(
            bytes.len() >= 33,
            "{path} is too short to contain a PNG IHDR"
        );
        assert_eq!(&bytes[..8], b"\x89PNG\r\n\x1a\n", "{path} is not a PNG");
        assert_eq!(
            u32::from_be_bytes(bytes[8..12].try_into().unwrap()),
            13,
            "{path} has an invalid IHDR length"
        );
        assert_eq!(&bytes[12..16], b"IHDR", "{path} does not start with IHDR");

        let width = u32::from_be_bytes(bytes[16..20].try_into().unwrap());
        let height = u32::from_be_bytes(bytes[20..24].try_into().unwrap());
        let bit_depth = bytes[24];
        let color_type = bytes[25];

        assert_eq!(
            (width, height),
            (expected_width, expected_height),
            "{path} has unexpected dimensions"
        );
        assert_eq!(bit_depth, 8, "{path} must use 8-bit color channels");
        assert!(
            matches!(color_type, 2 | 6),
            "{path} must be RGB or RGBA, found PNG color type {color_type}"
        );
        if path.ends_with("resource-orb-frame-reliquary.png") {
            assert_eq!(color_type, 6, "{path} must preserve its alpha channel");
        }
    }
}

#[test]
fn runtime_data_files_are_valid_ron() {
    for path in [
        "assets/data/player.ron",
        "assets/data/enemies.ron",
        "assets/data/loot.ron",
    ] {
        let content = fs::read_to_string(path).unwrap();
        let parsed: ron::Value = ron::from_str(&content).unwrap();
        assert!(matches!(parsed, ron::Value::Map(_)));
    }
}

#[test]
fn readme_controls_cover_core_runtime_shortcuts() {
    let readme = fs::read_to_string("README.md").unwrap();
    let controls = readme
        .split("## Controls")
        .nth(1)
        .and_then(|section| section.split("## Current Chapter").next())
        .expect("README should contain a controls section before the chapter section");

    for shortcut in [
        "`WASD`",
        "`Left Mouse`",
        "`Right Mouse`",
        "`Q`",
        "`E`",
        "`Y`",
        "`N`",
        "`H`",
        "`U`",
        "`T`",
        "`;`",
        "`I`",
        "`K`",
        "`J`",
        "`1` / `2` / `3`",
        "`4` / `5` / `6`",
        "`7` / `8` / `9`",
        "`Space`",
        "`Esc`",
    ] {
        assert!(
            controls.contains(shortcut),
            "README controls are missing shortcut {shortcut}"
        );
    }
    assert!(controls.contains("cycle Reliquary Sentinel stance"));
    assert!(controls.contains("salvage spare inventory gear"));
    assert!(!controls.contains("`N`: salvage"));
}

#[test]
fn character_models_keep_named_hit_bones() {
    for path in [
        "assets/models/hero.glb",
        "assets/models/skeleton.glb",
        "assets/models/cultist.glb",
        "assets/models/butcher.glb",
    ] {
        let bytes = fs::read(path).unwrap();
        for bone in [
            "hit_head",
            "hit_chest",
            "hit_weapon",
            "hit_left",
            "hit_right",
            "hit_head_contact_zone",
            "hit_chest_contact_zone",
            "hit_weapon_contact_zone",
        ] {
            assert!(
                contains_ascii(&bytes, bone),
                "{path} is missing Blender hit bone {bone}"
            );
        }
    }
}

#[test]
fn combat_vfx_models_keep_readable_effect_nodes() {
    for (path, marker) in [
        ("assets/models/slash_arc.glb", "slash crescent core"),
        ("assets/models/hit_spark.glb", "hit spark center"),
        ("assets/models/bone_shatter.glb", "bone shatter core"),
        ("assets/models/bone_impact.glb", "bone impact hit bone ring"),
        ("assets/models/blood_spray.glb", "blood spray heart"),
        ("assets/models/execution_burst.glb", "execution blood seal"),
        ("assets/models/arcane_impact.glb", "arcane impact ring"),
        ("assets/models/holy_impact.glb", "holy impact core"),
        (
            "assets/models/ember_impact.glb",
            "ember impact scorched hit ring",
        ),
        (
            "assets/models/frost_impact.glb",
            "frost impact fractured hit ring",
        ),
        ("assets/models/void_impact.glb", "void impact siphon seal"),
        (
            "assets/models/frenzy_impact.glb",
            "frenzy impact torn hit ring",
        ),
        (
            "assets/models/vampiric_siphon.glb",
            "vampiric siphon blood halo",
        ),
        (
            "assets/models/desecrator_burst.glb",
            "desecrator burst corrupted ground seal",
        ),
        ("assets/models/guard_clash.glb", "guard clash hit bone disc"),
        ("assets/models/armor_break.glb", "armor break cracked plate"),
        (
            "assets/models/soul_ward_hit.glb",
            "soul ward reflected halo",
        ),
        (
            "assets/models/hit_bone_rune.glb",
            "hit bone rune contact ring",
        ),
        (
            "assets/models/hit_bone_lock.glb",
            "hit bone lock targeting ring",
        ),
        ("assets/models/marrow_flash.glb", "marrow flash core"),
        (
            "assets/models/bone_fracture_echo.glb",
            "bone fracture echo cracked halo",
        ),
        (
            "assets/models/elite_affix_break.glb",
            "elite affix break outer seal",
        ),
        ("assets/models/shadow_burst.glb", "shadow burst ember"),
        (
            "assets/models/headshot_burst.glb",
            "headshot hit bone crown",
        ),
        ("assets/models/crit_bone_crown.glb", "crit bone crown halo"),
        ("assets/models/crit_burst.glb", "crit burst core"),
        ("assets/models/stagger_burst.glb", "stagger shock ring"),
        ("assets/models/shadow_trail.glb", "shadow dash trail"),
        ("assets/models/loot_prism.glb", "loot prism beam"),
    ] {
        let bytes = fs::read(path).unwrap();
        assert!(
            contains_ascii(&bytes, marker),
            "{path} is missing expected Blender node {marker}"
        );
    }
}

#[test]
fn elite_affix_aura_models_keep_readable_effect_nodes() {
    for (path, marker) in [
        (
            "assets/models/affix_ember_aura.glb",
            "affix ember aura ground rune",
        ),
        (
            "assets/models/affix_arcane_aura.glb",
            "affix arcane aura rotating sigil",
        ),
        (
            "assets/models/affix_frost_aura.glb",
            "affix frost aura control ring",
        ),
        (
            "assets/models/affix_blood_aura.glb",
            "affix blood aura siphon ring",
        ),
        (
            "assets/models/affix_ward_aura.glb",
            "affix ward aura shield ring",
        ),
    ] {
        let bytes = fs::read(path).unwrap();
        assert!(
            contains_ascii(&bytes, marker),
            "{path} is missing expected elite affix marker {marker}"
        );
    }
}

fn contains_ascii(bytes: &[u8], needle: &str) -> bool {
    bytes
        .windows(needle.len())
        .any(|window| window == needle.as_bytes())
}
