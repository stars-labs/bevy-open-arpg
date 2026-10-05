use std::{fs, path::Path};

#[test]
fn chapter_runtime_assets_exist() {
    for path in [
        "assets/data/player.ron",
        "assets/data/enemies.ron",
        "assets/data/loot.ron",
        "assets/audio/ambient_theme.wav",
        "assets/models/affix_arcane_aura.glb",
        "assets/models/affix_blood_aura.glb",
        "assets/models/affix_ember_aura.glb",
        "assets/models/affix_frost_aura.glb",
        "assets/models/affix_ward_aura.glb",
        "assets/models/altar.glb",
        "assets/models/arcane_impact.glb",
        "assets/models/armor_break.glb",
        "assets/models/ashbone_guard.glb",
        "assets/models/ashen_marksman.glb",
        "assets/models/ashen_pylon.glb",
        "assets/models/bell_warden.glb",
        "assets/models/blood_obelisk.glb",
        "assets/models/blood_spray.glb",
        "assets/models/bone_fracture_echo.glb",
        "assets/models/bone_impact.glb",
        "assets/models/bone_shatter.glb",
        "assets/models/bone_stalker.glb",
        "assets/models/boss_summon_portal.glb",
        "assets/models/breakable_coffer.glb",
        "assets/models/breakable_urn.glb",
        "assets/models/butcher.glb",
        "assets/models/chest.glb",
        "assets/models/choir_acolyte.glb",
        "assets/models/choir_marksman.glb",
        "assets/models/crit_bone_crown.glb",
        "assets/models/crit_burst.glb",
        "assets/models/cultist.glb",
        "assets/models/cursed_shrine.glb",
        "assets/models/death_remains.glb",
        "assets/models/desecrator_burst.glb",
        "assets/models/elite_affix_break.glb",
        "assets/models/ember_impact.glb",
        "assets/models/ember_rift_prop.glb",
        "assets/models/ember_vent.glb",
        "assets/models/execution_burst.glb",
        "assets/models/fortune_shrine.glb",
        "assets/models/frenzy_impact.glb",
        "assets/models/frost_impact.glb",
        "assets/models/fury_pickup.glb",
        "assets/models/guard_clash.glb",
        "assets/models/headshot_burst.glb",
        "assets/models/healing_well.glb",
        "assets/models/health_pickup.glb",
        "assets/models/hero.glb",
        "assets/models/hero_arm.glb",
        "assets/models/hero_leg.glb",
        "assets/models/hero_mantle.glb",
        "assets/models/hit_bone_lock.glb",
        "assets/models/hit_bone_rune.glb",
        "assets/models/hit_spark.glb",
        "assets/models/contact_steel.glb",
        "assets/models/contact_expose.glb",
        "assets/models/contact_blood.glb",
        "assets/models/contact_cleanse.glb",
        "assets/models/contact_reap.glb",
        "assets/models/contact_ember.glb",
        "assets/models/contact_frost.glb",
        "assets/models/proc_vigor.glb",
        "assets/models/proc_focus.glb",
        "assets/models/hollow_matron.glb",
        "assets/models/hollow_reaver.glb",
        "assets/models/hollow_stalker.glb",
        "assets/models/hollow_thrall.glb",
        "assets/models/holy_impact.glb",
        "assets/models/keeper.glb",
        "assets/models/keeper_chain_cage.glb",
        "assets/models/keeper_cleaver_wave.glb",
        "assets/models/loot_prism.glb",
        "assets/models/lore_page.glb",
        "assets/models/marrow_flash.glb",
        "assets/models/matron_crescendo.glb",
        "assets/models/matron_soul_lance.glb",
        "assets/models/nemesis.glb",
        "assets/models/objective_sigil.glb",
        "assets/models/quartermaster.glb",
        "assets/models/reliquary_vault.glb",
        "assets/models/sanctum_gate.glb",
        "assets/models/scenery_crystal.glb",
        "assets/models/scenery_dead_tree.glb",
        "assets/models/scenery_rock.glb",
        "assets/models/scenery_ruin.glb",
        "assets/models/seal_warden.glb",
        "assets/models/shadow_burst.glb",
        "assets/models/shadow_trail.glb",
        "assets/models/skeleton.glb",
        "assets/models/slash_arc.glb",
        "assets/models/soul_fissure.glb",
        "assets/models/soul_font.glb",
        "assets/models/soul_ward_hit.glb",
        "assets/models/stagger_burst.glb",
        "assets/models/storm_shrine.glb",
        "assets/models/sword.glb",
        "assets/models/treasure_imp.glb",
        "assets/models/vampiric_siphon.glb",
        "assets/models/void_impact.glb",
        "assets/audio/hit.wav",
        "assets/audio/contact_steel.wav",
        "assets/audio/contact_expose.wav",
        "assets/audio/contact_blood.wav",
        "assets/audio/contact_cleanse.wav",
        "assets/audio/contact_reap.wav",
        "assets/audio/contact_ember.wav",
        "assets/audio/contact_frost.wav",
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
        "assets/data/zh_cn.tsv",
        "assets/data/journal_zh_cn.tsv",
        "assets/data/panels_zh_cn.tsv",
        "assets/fonts/NotoSansSC.ttf",
        "assets/fonts/NotoSansSC-OFL.txt",
        "assets/fonts/Cinzel.ttf",
        "assets/fonts/Cinzel-OFL.txt",
        "assets/fonts/CrimsonPro.ttf",
        "assets/fonts/CrimsonPro-OFL.txt",
        "assets/images/generated/bevy-open-arpg-concept.png",
        "assets/images/generated/ui/gear-icons-reliquary.png",
        "assets/images/generated/ui/main-menu-reliquary-v2.png",
        "assets/images/generated/ui/panel-frame-reliquary.png",
        "assets/images/generated/ui/resource-orb-frame-reliquary.png",
        "assets/images/generated/ui/skill-icons-reliquary.png",
        "assets/images/generated/ui/slot-frame-reliquary.png",
        "assets/images/generated/ui/talent-sigils-reliquary.png",
        "assets/images/generated/ui/utility-icons-reliquary.png",
        "assets/images/generated/world/ashen-ground-albedo.jpg",
        "assets/images/generated/world/ashen-road-albedo.jpg",
        "assets/images/generated/world/hollow-ground-albedo.jpg",
        "assets/images/generated/world/hollow-road-albedo.jpg",
        "assets/textures/materials/bone-detail.jpg",
        "assets/textures/materials/cloth-detail.jpg",
        "assets/textures/materials/metal-detail.jpg",
        "assets/textures/materials/stone-detail.jpg",
    ] {
        assert!(Path::new(path).exists(), "missing runtime asset: {path}");
    }
}

#[test]
fn webgpu_runtime_avoids_transparent_pbr_materials() {
    for entry in fs::read_dir("src").unwrap() {
        let path = entry.unwrap().path();
        if path.extension().and_then(|extension| extension.to_str()) != Some("rs") {
            continue;
        }
        let source = fs::read_to_string(&path).unwrap();
        assert!(
            !source.contains("AlphaMode::Blend"),
            "{} can trigger the unsupported pbr_alpha_blend pipeline",
            path.display()
        );
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
            768,
            512,
        ),
        (
            "assets/images/generated/ui/slot-frame-reliquary.png",
            627,
            627,
        ),
        (
            "assets/images/generated/ui/talent-sigils-reliquary.png",
            1_086,
            362,
        ),
        (
            "assets/images/generated/ui/skill-icons-reliquary.png",
            768,
            512,
        ),
        (
            "assets/images/generated/ui/gear-icons-reliquary.png",
            888,
            444,
        ),
        (
            "assets/images/generated/ui/resource-orb-frame-reliquary.png",
            627,
            627,
        ),
        (
            "assets/images/generated/ui/utility-icons-reliquary.png",
            1_024,
            256,
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
        if !path.ends_with("main-menu-reliquary-v2.png") {
            assert_eq!(color_type, 6, "{path} must preserve its alpha channel");
        }
    }
}

#[test]
fn generated_world_and_material_textures_are_compact_jpegs() {
    for path in [
        "assets/images/generated/world/ashen-ground-albedo.jpg",
        "assets/images/generated/world/ashen-road-albedo.jpg",
        "assets/images/generated/world/hollow-ground-albedo.jpg",
        "assets/images/generated/world/hollow-road-albedo.jpg",
        "assets/textures/materials/bone-detail.jpg",
        "assets/textures/materials/cloth-detail.jpg",
        "assets/textures/materials/metal-detail.jpg",
        "assets/textures/materials/stone-detail.jpg",
    ] {
        let bytes = fs::read(path).unwrap();
        assert!(bytes.len() > 8_000, "{path} is unexpectedly small");
        assert_eq!(&bytes[..2], b"\xff\xd8", "{path} is not a JPEG");
        assert_eq!(
            &bytes[bytes.len() - 2..],
            b"\xff\xd9",
            "{path} has a truncated JPEG trailer"
        );
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
        "assets/models/ashbone_guard.glb",
        "assets/models/cultist.glb",
        "assets/models/butcher.glb",
        "assets/models/bone_stalker.glb",
        "assets/models/ashen_marksman.glb",
        "assets/models/treasure_imp.glb",
        "assets/models/seal_warden.glb",
        "assets/models/keeper.glb",
        "assets/models/nemesis.glb",
        "assets/models/hollow_thrall.glb",
        "assets/models/hollow_stalker.glb",
        "assets/models/choir_acolyte.glb",
        "assets/models/choir_marksman.glb",
        "assets/models/hollow_reaver.glb",
        "assets/models/bell_warden.glb",
        "assets/models/hollow_matron.glb",
    ] {
        let bytes = fs::read(path).unwrap();
        for bone in [
            "hit_head",
            "hit_chest",
            "hit_weapon",
            "hit_left",
            "hit_right",
        ] {
            assert!(
                contains_ascii(&bytes, bone),
                "{path} is missing Blender hit bone {bone}"
            );
        }
    }
}

#[test]
fn generated_enemy_roster_keeps_contact_zones_and_effect_sockets() {
    for path in [
        "assets/models/ashbone_guard.glb",
        "assets/models/bone_stalker.glb",
        "assets/models/ashen_marksman.glb",
        "assets/models/treasure_imp.glb",
        "assets/models/seal_warden.glb",
        "assets/models/keeper.glb",
        "assets/models/nemesis.glb",
        "assets/models/hollow_stalker.glb",
        "assets/models/choir_marksman.glb",
        "assets/models/bell_warden.glb",
    ] {
        let bytes = fs::read(path).unwrap();
        for marker in [
            "hit_head_marker",
            "hit_chest_marker",
            "hit_weapon_marker",
            "hit_left_marker",
            "hit_right_marker",
            "hit_head_contact_zone",
            "hit_chest_contact_zone",
            "hit_weapon_contact_zone",
            "hit_left_contact_zone",
            "hit_right_contact_zone",
            "socket_weapon",
            "socket_cast",
            "socket_head_fx",
            "socket_chest_fx",
            "socket_ground_fx",
        ] {
            assert!(
                contains_ascii(&bytes, marker),
                "{path} is missing generated combat marker {marker}"
            );
        }
    }
}

#[test]
fn boss_skill_models_keep_readable_effect_nodes() {
    for (path, marker) in [
        (
            "assets/models/keeper_cleaver_wave.glb",
            "keeper cleaver wave crescent",
        ),
        (
            "assets/models/keeper_chain_cage.glb",
            "keeper interlocking chain cage",
        ),
        (
            "assets/models/matron_soul_lance.glb",
            "matron tapered soul spear",
        ),
        (
            "assets/models/matron_crescendo.glb",
            "matron broken resonance wave",
        ),
    ] {
        let bytes = fs::read(path).unwrap();
        assert!(
            contains_ascii(&bytes, marker),
            "{path} is missing expected Boss ability marker {marker}"
        );
    }
}

#[test]
fn core_characters_and_bosses_embed_detail_textures_and_cull_backfaces() {
    for path in [
        "assets/models/hero.glb",
        "assets/models/skeleton.glb",
        "assets/models/ashbone_guard.glb",
        "assets/models/cultist.glb",
        "assets/models/butcher.glb",
        "assets/models/bone_stalker.glb",
        "assets/models/ashen_marksman.glb",
        "assets/models/treasure_imp.glb",
        "assets/models/seal_warden.glb",
        "assets/models/keeper.glb",
        "assets/models/nemesis.glb",
        "assets/models/hollow_thrall.glb",
        "assets/models/hollow_stalker.glb",
        "assets/models/choir_acolyte.glb",
        "assets/models/choir_marksman.glb",
        "assets/models/hollow_reaver.glb",
        "assets/models/bell_warden.glb",
        "assets/models/hollow_matron.glb",
    ] {
        let json = glb_json(path);
        assert!(
            json.contains("\"images\":[{") && json.contains("\"textures\":[{"),
            "{path} must embed at least one detail image and glTF texture"
        );
        assert!(
            json.contains("\"mimeType\":\"image/jpeg\"")
                && json.contains("\"bufferView\":")
                && !json.contains("\"uri\":"),
            "{path} detail images must be embedded JPEG buffer views"
        );
        assert!(
            json.contains("\"baseColorTexture\":"),
            "{path} must connect detail to a PBR baseColorTexture"
        );
        assert!(
            !json.contains("\"doubleSided\":true"),
            "{path} is a closed character and must cull backfaces"
        );
    }
}

#[test]
fn keeper_cleaver_wave_stays_untextured_double_sided_vfx() {
    let json = glb_json("assets/models/keeper_cleaver_wave.glb");
    assert!(
        !json.contains("\"baseColorTexture\":"),
        "keeper cleaver wave has intentionally degenerate UVs and must stay texture-free"
    );
    assert!(
        json.contains("\"doubleSided\":true"),
        "keeper cleaver wave is a thin gameplay telegraph and must stay double-sided"
    );
}

#[test]
fn combat_vfx_models_keep_readable_effect_nodes() {
    for (path, marker) in [
        ("assets/models/slash_arc.glb", "forged slash crescent"),
        (
            "assets/models/hit_spark.glb",
            "hit_spark tapered contact fragments",
        ),
        (
            "assets/models/bone_shatter.glb",
            "bone_shatter tapered contact fragments",
        ),
        (
            "assets/models/bone_impact.glb",
            "bone_impact tapered contact fragments",
        ),
        (
            "assets/models/blood_spray.glb",
            "blood_spray tapered contact fragments",
        ),
        (
            "assets/models/execution_burst.glb",
            "execution_burst tapered contact fragments",
        ),
        ("assets/models/arcane_impact.glb", "arcane impact sigil"),
        (
            "assets/models/holy_impact.glb",
            "holy_impact tapered contact fragments",
        ),
        (
            "assets/models/ember_impact.glb",
            "ember_impact tapered contact fragments",
        ),
        (
            "assets/models/frost_impact.glb",
            "frost_impact tapered contact fragments",
        ),
        (
            "assets/models/void_impact.glb",
            "void_impact tapered contact fragments",
        ),
        (
            "assets/models/frenzy_impact.glb",
            "frenzy_impact tapered contact fragments",
        ),
        (
            "assets/models/vampiric_siphon.glb",
            "vampiric_siphon tapered contact fragments",
        ),
        (
            "assets/models/desecrator_burst.glb",
            "desecrator_burst tapered contact fragments",
        ),
        (
            "assets/models/guard_clash.glb",
            "guard_clash tapered contact fragments",
        ),
        (
            "assets/models/armor_break.glb",
            "armor_break tapered contact fragments",
        ),
        (
            "assets/models/soul_ward_hit.glb",
            "soul_ward_hit tapered contact fragments",
        ),
        ("assets/models/hit_bone_rune.glb", "bone hit runic arcs"),
        ("assets/models/hit_bone_lock.glb", "bone lock brackets"),
        (
            "assets/models/marrow_flash.glb",
            "marrow_flash tapered contact fragments",
        ),
        (
            "assets/models/bone_fracture_echo.glb",
            "fracture echo crescents",
        ),
        (
            "assets/models/elite_affix_break.glb",
            "elite_affix_break tapered contact fragments",
        ),
        (
            "assets/models/shadow_burst.glb",
            "shadow_burst tapered contact fragments",
        ),
        (
            "assets/models/headshot_burst.glb",
            "headshot_burst tapered contact fragments",
        ),
        (
            "assets/models/crit_bone_crown.glb",
            "crit_bone_crown tapered contact fragments",
        ),
        (
            "assets/models/crit_burst.glb",
            "crit_burst tapered contact fragments",
        ),
        (
            "assets/models/stagger_burst.glb",
            "stagger_burst tapered contact fragments",
        ),
        ("assets/models/shadow_trail.glb", "tapered shadow ribbons"),
        ("assets/models/loot_prism.glb", "loot_prism"),
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
            "ember ground inscriptions",
        ),
        (
            "assets/models/affix_arcane_aura.glb",
            "arcane ground inscriptions",
        ),
        (
            "assets/models/affix_frost_aura.glb",
            "frost ground inscriptions",
        ),
        (
            "assets/models/affix_blood_aura.glb",
            "blood ground inscriptions",
        ),
        (
            "assets/models/affix_ward_aura.glb",
            "ward ground inscriptions",
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

fn glb_json(path: &str) -> String {
    let bytes = fs::read(path).unwrap();
    assert!(bytes.len() >= 20, "{path} is too short to be a GLB");
    assert_eq!(&bytes[..4], b"glTF", "{path} has an invalid GLB magic");
    assert_eq!(
        u32::from_le_bytes(bytes[4..8].try_into().unwrap()),
        2,
        "{path} must use glTF 2.0"
    );
    let declared_length = u32::from_le_bytes(bytes[8..12].try_into().unwrap()) as usize;
    assert_eq!(
        declared_length,
        bytes.len(),
        "{path} has an invalid declared GLB length"
    );
    let json_length = u32::from_le_bytes(bytes[12..16].try_into().unwrap()) as usize;
    assert_eq!(
        &bytes[16..20],
        b"JSON",
        "{path} must start with a JSON chunk"
    );
    assert!(
        20 + json_length <= bytes.len(),
        "{path} JSON chunk exceeds its file length"
    );
    String::from_utf8(bytes[20..20 + json_length].to_vec()).unwrap()
}

#[test]
fn hero_asset_owns_body_only_and_limbs_are_separate() {
    let body = glb_json("assets/models/hero.glb");
    for baked in ["left boot", "right boot", "sword blade", "sword hilt"] {
        assert!(
            !body.contains(baked),
            "baked {baked} duplicates runtime animation"
        );
    }
    for part in ["hero", "hero_arm", "hero_leg", "hero_mantle"] {
        let json = glb_json(&format!("assets/models/{part}.glb"));
        assert!(json.contains(&format!("\"hero_part\":\"{part}\"")));
        assert!(json.contains("\"baseColorTexture\":"));
    }
}

#[test]
fn enemy_assets_expose_actual_articulated_limbs() {
    for name in [
        "skeleton",
        "ashbone_guard",
        "cultist",
        "butcher",
        "bone_stalker",
        "ashen_marksman",
        "treasure_imp",
        "seal_warden",
        "keeper",
        "nemesis",
        "hollow_thrall",
        "hollow_stalker",
        "choir_acolyte",
        "choir_marksman",
        "hollow_reaver",
        "bell_warden",
        "hollow_matron",
    ] {
        let json = glb_json(&format!("assets/models/{name}.glb"));
        for limb in [
            "enemy_left_leg",
            "enemy_right_leg",
            "enemy_left_arm",
            "enemy_right_arm",
        ] {
            assert!(json.contains(limb), "{name} lacks animated {limb}");
        }
    }
}

#[test]
fn combat_contacts_use_opaque_sparse_authored_models() {
    for name in [
        "contact_steel",
        "contact_expose",
        "contact_blood",
        "contact_cleanse",
        "contact_reap",
        "contact_ember",
        "contact_frost",
        "proc_vigor",
        "proc_focus",
    ] {
        let json = glb_json(&format!("assets/models/{name}.glb"));
        assert!(json.contains(&format!("{name} sparse contact artwork")));
        assert!(
            !json.contains("BLEND"),
            "{name} must not depend on transparent PBR"
        );
        assert!(!json.contains("MASK"), "{name} is authored opaque geometry");
    }
}

#[test]
fn contact_sounds_are_short_distinct_unclipped_pcm() {
    let mut previous = Vec::new();
    for name in [
        "steel", "expose", "blood", "cleanse", "reap", "ember", "frost",
    ] {
        let bytes = fs::read(format!("assets/audio/contact_{name}.wav")).unwrap();
        assert_eq!(&bytes[..4], b"RIFF");
        assert_eq!(&bytes[8..12], b"WAVE");
        assert_eq!(u16::from_le_bytes(bytes[22..24].try_into().unwrap()), 1);
        assert_eq!(
            u32::from_le_bytes(bytes[24..28].try_into().unwrap()),
            44_100
        );
        assert_eq!(u16::from_le_bytes(bytes[34..36].try_into().unwrap()), 16);
        let samples: Vec<_> = bytes[44..]
            .as_chunks::<2>()
            .0
            .iter()
            .map(|sample| i16::from_le_bytes([sample[0], sample[1]]))
            .collect();
        assert!(
            samples.len() <= 44_100 / 3,
            "{name} contact tail is too long"
        );
        assert!(samples.iter().any(|sample| sample.unsigned_abs() > 1_000));
        assert!(samples.iter().all(|sample| sample.unsigned_abs() < 32_000));
        assert!(
            !previous.contains(&bytes),
            "{name} duplicates another material sound"
        );
        previous.push(bytes);
    }
}
