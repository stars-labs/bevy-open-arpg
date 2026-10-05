use crate::{
    GameState, RunStats,
    assets::GameAssets,
    chapter::{
        ActiveChapter, ChapterId, ChapterPhase, ChapterProgress, InteractableKind, InteractableUsed,
    },
    feedback::CombatEvent,
    not_paused,
    ordeal::ChapterModifier,
    player::{
        Barrier, ElixirBuff, Equipment, Evade, FortuneBuff, Fury, Health, Player, PotionBelt,
        apply_player_damage_with_evade, fortune_gold_reward, mitigated_damage, total_armor,
    },
};
use bevy::math::Affine2;
use bevy::prelude::*;

#[derive(Component)]
pub struct DungeonEntity;

/// Programmatic opaque scenery used to dress the outdoor map.
#[derive(Component)]
pub struct OutdoorScenery;

#[derive(Component)]
pub struct Interactable {
    pub kind: InteractableKind,
    pub radius: f32,
    pub used: bool,
    pub reusable: bool,
}

#[derive(Debug, Clone, Copy, Default, Eq, PartialEq)]
pub enum ChapterZone {
    CacheOssuary,
    OuterSanctum,
    GildedVault,
    EmberAltar,
    QuartermasterNave,
    Wilderness,
    #[default]
    ReliquaryCrossing,
}

impl ChapterZone {
    pub fn label(self) -> &'static str {
        match self {
            Self::CacheOssuary => "Ashen Gravefield",
            Self::OuterSanctum => "Northern Expanse",
            Self::GildedVault => "Gilded Ruins",
            Self::EmberAltar => "Ember Highlands",
            Self::QuartermasterNave => "Wayfarer Camp",
            Self::Wilderness => "Uncharted Wilds",
            Self::ReliquaryCrossing => "Reliquary Wilds",
        }
    }

    pub fn tactical_hint(self) -> &'static str {
        match self {
            Self::CacheOssuary => "cache, rift, cursed shrine",
            Self::OuterSanctum => "shrines, lore, first seal route",
            Self::GildedVault => "vault, fortune shrine, breakables",
            Self::EmberAltar => "altar, obelisk, boss gate",
            Self::QuartermasterNave => "vendor, well, pylon, portal return",
            Self::Wilderness => "explore the endless frontier; follow the compass home",
            Self::ReliquaryCrossing => "rotate between main and optional sites",
        }
    }
}

#[derive(Resource, Debug, Clone, Copy, Eq, PartialEq)]
pub struct ChapterZoneState {
    pub current: ChapterZone,
    pub previous: Option<ChapterZone>,
}

impl Default for ChapterZoneState {
    fn default() -> Self {
        Self {
            current: ChapterZone::ReliquaryCrossing,
            previous: None,
        }
    }
}

#[derive(Component)]
struct ObjectiveMarkerAttached;

#[derive(Component)]
struct ObjectiveMarkerVisual {
    owner: Entity,
    role: ObjectiveMarkerRole,
}

#[derive(Component)]
struct ObjectivePromptVisual {
    owner: Entity,
}

#[derive(Clone, Copy)]
enum ObjectiveMarkerRole {
    Ring,
    Beacon,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
enum ObjectiveMarkerState {
    Hidden,
    Optional,
    Primary,
}

#[derive(Component)]
struct DungeonHazard {
    damage: f32,
    radius: f32,
    pulse: Timer,
    label: &'static str,
}

/// One energized monolith or light belonging to the sealed mountain pass.
#[derive(Component)]
pub struct SanctumGate;

/// Converts the compact prototype coordinates into the authored origin-region scale.
pub const WORLD_SCALE: f32 = 3.2;

#[cfg(test)]
const WORLD_MIN_X: f32 = -52.0;
#[cfg(test)]
const WORLD_MAX_X: f32 = 52.0;
#[cfg(test)]
const WORLD_MIN_Z: f32 = -84.0;
#[cfg(test)]
const WORLD_MAX_Z: f32 = 36.0;
#[cfg(test)]
const WORLD_CENTER_Z: f32 = (WORLD_MIN_Z + WORLD_MAX_Z) * 0.5;
const SEALED_NORTH_LIMIT_Z: f32 = -7.5 * WORLD_SCALE;
const SEALED_PASS_HALF_WIDTH: f32 = 5.2 * WORLD_SCALE;
const SEALED_PASS_DEPTH: f32 = 2.6 * WORLD_SCALE;

/// Chapter landmarks shared by spawning, navigation, and the minimap.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ChapterWorldLayout {
    pub player_spawn: Vec3,
    pub camp: Vec3,
    pub cache: Vec3,
    pub altar: Vec3,
    pub weapon_shrine: Vec3,
    pub boss: Vec3,
    pub lore_pages: [Vec3; 3],
}

/// Maps a legacy encounter coordinate into the enlarged outdoor world.
///
/// Hollow Bastion mirrors the route along the x axis so that its landmarks
/// and traversal read differently without changing chapter progression.
pub fn chapter_world_position(chapter: ChapterId, legacy: Vec3) -> Vec3 {
    let mirrored_x = match chapter {
        ChapterId::AshenReliquary => legacy.x,
        ChapterId::HollowBastion => -legacy.x,
    };
    Vec3::new(mirrored_x * WORLD_SCALE, legacy.y, legacy.z * WORLD_SCALE)
}

pub fn chapter_world_layout(chapter: ChapterId) -> ChapterWorldLayout {
    ChapterWorldLayout {
        player_spawn: chapter_world_position(chapter, Vec3::new(0.0, 0.0, 4.0)),
        camp: chapter_world_position(chapter, Vec3::new(-4.8, 0.0, 5.9)),
        cache: chapter_world_position(chapter, Vec3::new(-8.0, 0.0, -5.5)),
        altar: chapter_world_position(chapter, Vec3::new(6.0, 0.0, -20.5)),
        weapon_shrine: chapter_world_position(chapter, Vec3::new(0.0, 0.0, -6.8)),
        boss: chapter_world_position(chapter, Vec3::new(0.0, 0.0, -20.0)),
        lore_pages: [
            chapter_world_position(chapter, Vec3::new(-10.2, 0.0, 5.8)),
            chapter_world_position(chapter, Vec3::new(3.1, 0.0, -6.3)),
            chapter_world_position(chapter, Vec3::new(9.2, 0.0, 5.7)),
        ],
    }
}

/// Compatibility hook for gameplay systems that previously clamped movement.
///
/// The procedural world is unbounded, so positions now pass through unchanged.
pub fn clamp_world_translation(translation: Vec3) -> Vec3 {
    translation
}

/// Walkable-space state for the outdoor reliquary.
#[derive(Resource, Default, Debug, Clone, Copy)]
pub struct DungeonLayout {
    pub sanctum_gate_open: bool,
}

/// Blocks only the authored mountain-pass seal while leaving the procedural
/// wilderness unbounded. Players can explore around the story route.
pub fn clamp_dungeon_translation(layout: &DungeonLayout, translation: Vec3) -> Vec3 {
    let mut clamped = translation;
    let inside_sealed_pass = translation.x.abs() <= SEALED_PASS_HALF_WIDTH
        && translation.z < SEALED_NORTH_LIMIT_Z
        && translation.z >= SEALED_NORTH_LIMIT_Z - SEALED_PASS_DEPTH;
    if !layout.sanctum_gate_open && inside_sealed_pass {
        clamped.z = clamped.z.max(SEALED_NORTH_LIMIT_Z);
    }
    clamped
}

/// Removes the pass seal when the chapter cracks it.
fn open_sanctum_gate(
    mut commands: Commands,
    layout: Res<DungeonLayout>,
    gates: Query<Entity, With<SanctumGate>>,
    mut combat_events: MessageWriter<CombatEvent>,
) {
    if !layout.is_changed() || !layout.sanctum_gate_open {
        return;
    }
    let mut opened = false;
    for gate in &gates {
        if let Ok(mut entity_commands) = commands.get_entity(gate) {
            entity_commands.try_despawn();
            opened = true;
        }
    }
    if opened {
        combat_events.write(CombatEvent {
            text: "The seal parts: the northern wilds lie open".to_string(),
        });
    }
}

#[derive(Component)]
struct DungeonLightPulse {
    base_intensity: f32,
    amplitude: f32,
    speed: f32,
    phase: f32,
}

#[derive(Clone, Copy)]
struct DungeonLightProfile {
    color: Color,
    intensity: f32,
    range: f32,
    pulse_amplitude: f32,
    pulse_speed: f32,
}

type HazardPlayerQuery<'w, 's> = Query<
    'w,
    's,
    (
        &'static Transform,
        &'static mut Health,
        &'static mut Barrier,
        &'static Evade,
        &'static Equipment,
        &'static ElixirBuff,
    ),
    With<Player>,
>;

type PromptPlayerQuery<'w, 's> = Query<
    'w,
    's,
    &'static Transform,
    (
        With<Player>,
        Without<Interactable>,
        Without<ObjectivePromptVisual>,
    ),
>;

type ObjectivePromptQuery<'w, 's> = Query<
    'w,
    's,
    (
        &'static ObjectivePromptVisual,
        &'static mut Visibility,
        &'static mut Transform,
        &'static mut Text2d,
    ),
    (Without<Player>, Without<Interactable>),
>;

#[derive(Component, Clone, Copy)]
pub struct Breakable {
    gold: u32,
    potions: u32,
    fury: f32,
    label: &'static str,
}

pub const BREAKER_TARGET_BREAKABLES: u32 = 4;

#[derive(Clone, Copy)]
struct BreakableSpawn {
    position: Vec3,
    gold: u32,
    potions: u32,
    fury: f32,
    label: &'static str,
}

pub struct DungeonPlugin;

impl Plugin for DungeonPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<ChapterZoneState>()
            .init_resource::<DungeonLayout>()
            .add_systems(
                OnEnter(GameState::InGame),
                (reset_chapter_zone, reset_dungeon_layout, spawn_dungeon),
            )
            .add_systems(
                Update,
                (
                    update_chapter_zone,
                    use_interactables,
                    attach_objective_markers,
                    update_objective_markers,
                    update_objective_prompts,
                    update_dungeon_light_pulses,
                    tick_dungeon_hazards,
                    reward_destroyed_breakables,
                    open_sanctum_gate,
                )
                    .run_if(in_state(GameState::InGame).and_then(not_paused)),
            )
            .add_systems(OnExit(GameState::InGame), despawn_dungeon);
    }
}

fn reset_dungeon_layout(mut layout: ResMut<DungeonLayout>) {
    layout.sanctum_gate_open = false;
}

fn reset_chapter_zone(mut zone: ResMut<ChapterZoneState>) {
    *zone = ChapterZoneState::default();
}

fn update_chapter_zone(
    player: Query<&Transform, With<Player>>,
    active_chapter: Res<ActiveChapter>,
    mut zone: ResMut<ChapterZoneState>,
    mut combat_events: MessageWriter<CombatEvent>,
) {
    let Ok(player_transform) = player.single() else {
        return;
    };
    let current = match active_chapter.0 {
        ChapterId::AshenReliquary => chapter_zone_for_position(player_transform.translation),
        ChapterId::HollowBastion => {
            chapter_zone_for_chapter_position(active_chapter.0, player_transform.translation)
        }
    };
    if current == zone.current {
        return;
    }
    zone.previous = Some(zone.current);
    zone.current = current;
    combat_events.write(CombatEvent {
        text: format!("Entered {} | {}", current.label(), current.tactical_hint()),
    });
}

pub fn chapter_zone_for_position(position: Vec3) -> ChapterZone {
    chapter_zone_for_chapter_position(ChapterId::AshenReliquary, position)
}

pub fn chapter_zone_for_chapter_position(chapter: ChapterId, position: Vec3) -> ChapterZone {
    let canonical_x = match chapter {
        ChapterId::AshenReliquary => position.x,
        ChapterId::HollowBastion => -position.x,
    } / WORLD_SCALE;
    let canonical_z = position.z / WORLD_SCALE;
    if canonical_x.abs() > 15.0 || !(-24.0..=10.0).contains(&canonical_z) {
        return ChapterZone::Wilderness;
    }
    match (canonical_x, canonical_z) {
        (x, z) if x <= -6.0 && z <= -1.2 => ChapterZone::CacheOssuary,
        (x, z) if x >= 6.2 && z <= -1.8 => ChapterZone::GildedVault,
        (_, z) if z <= -4.4 => ChapterZone::OuterSanctum,
        (x, z) if x >= 5.2 && z >= 0.8 => ChapterZone::EmberAltar,
        (_, z) if z >= 4.8 => ChapterZone::QuartermasterNave,
        _ => ChapterZone::ReliquaryCrossing,
    }
}

fn spawn_dungeon(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    assets: Res<GameAssets>,
    modifier: Res<ChapterModifier>,
    active_chapter: Res<ActiveChapter>,
) {
    let chapter = active_chapter.0;
    let layout = chapter_world_layout(chapter);
    let (highland_texture, road_texture) = match chapter {
        ChapterId::AshenReliquary => (
            assets.ashen_ground_albedo.clone(),
            assets.ashen_road_albedo.clone(),
        ),
        ChapterId::HollowBastion => (
            assets.hollow_ground_albedo.clone(),
            assets.hollow_road_albedo.clone(),
        ),
    };
    let (highland_color, road_color) = match chapter {
        ChapterId::AshenReliquary => (Color::srgb(0.88, 0.78, 0.68), Color::srgb(0.92, 0.80, 0.66)),
        ChapterId::HollowBastion => (Color::srgb(0.66, 0.84, 0.88), Color::srgb(0.76, 0.88, 0.92)),
    };

    let unit_cube = meshes.add(Cuboid::new(1.0, 1.0, 1.0));
    let clearing_mesh = meshes.add(Cylinder::new(1.0, 1.0));
    let highland_material = materials.add(StandardMaterial {
        base_color: highland_color,
        base_color_texture: Some(highland_texture),
        uv_transform: Affine2::from_scale(Vec2::splat(3.6)),
        perceptual_roughness: 0.94,
        ..default()
    });
    let road_material = materials.add(StandardMaterial {
        base_color: road_color,
        base_color_texture: Some(road_texture),
        uv_transform: Affine2::from_scale(Vec2::new(3.0, 10.0)),
        perceptual_roughness: 0.91,
        ..default()
    });

    for (index, (position, radius)) in [
        (layout.camp, Vec2::new(12.0, 9.0)),
        (layout.cache, Vec2::new(10.0, 8.0)),
        (layout.weapon_shrine, Vec2::new(11.0, 8.0)),
        (layout.altar, Vec2::new(13.0, 10.0)),
        (layout.boss, Vec2::new(16.0, 12.0)),
    ]
    .into_iter()
    .enumerate()
    {
        commands.spawn((
            Mesh3d(clearing_mesh.clone()),
            MeshMaterial3d(highland_material.clone()),
            Transform::from_xyz(position.x, -0.035 - index as f32 * 0.001, position.z)
                .with_scale(Vec3::new(radius.x, 0.08, radius.y)),
            DungeonEntity,
            Name::new("Outdoor Landmark Clearing"),
        ));
    }

    let gate_approach = chapter_world_position(chapter, Vec3::new(0.0, 0.0, -7.5));
    let fortune_position = chapter_world_position(chapter, Vec3::new(8.6, 0.0, -5.8));
    for (index, (from, to, width)) in [
        (layout.player_spawn, layout.camp, 4.8),
        (layout.camp, layout.weapon_shrine, 4.4),
        (layout.weapon_shrine, layout.cache, 3.6),
        (layout.weapon_shrine, fortune_position, 3.6),
        (layout.weapon_shrine, gate_approach, 4.2),
        (gate_approach, layout.boss, 4.8),
        (layout.boss, layout.altar, 4.0),
    ]
    .into_iter()
    .enumerate()
    {
        spawn_outdoor_road(
            &mut commands,
            &unit_cube,
            &road_material,
            from,
            to,
            width,
            index,
        );
    }

    // Carved seal pillars share the gate's lifetime and gameplay boundary.
    let gate_emissive = match chapter {
        ChapterId::AshenReliquary => Color::srgb(0.9, 0.22, 0.08),
        ChapterId::HollowBastion => Color::srgb(0.05, 0.72, 0.90),
    };
    for legacy_x in [-3.6, -1.8, 0.0, 1.8, 3.6] {
        let position = chapter_world_position(chapter, Vec3::new(legacy_x, 0.0, -9.0));
        commands.spawn((
            WorldAssetRoot(assets.sanctum_gate.clone()),
            Transform::from_translation(position)
                .with_rotation(Quat::from_rotation_z(legacy_x * 0.018)),
            DungeonEntity,
            SanctumGate,
            Name::new("Sanctum Pass Seal"),
        ));
    }
    let gate_light = spawn_dungeon_light(
        &mut commands,
        chapter_world_position(chapter, Vec3::new(0.0, 0.0, -9.0)),
        DungeonLightProfile {
            color: gate_emissive,
            intensity: 780.0,
            range: 12.0,
            pulse_amplitude: 260.0,
            pulse_speed: 2.0,
        },
        "Sanctum Pass Light",
    );
    commands.entity(gate_light).insert(SanctumGate);

    for (scene, position, scale, interaction) in [
        (
            assets.chest.clone(),
            layout.cache,
            1.0,
            Some(InteractableKind::Cache),
        ),
        (
            match chapter {
                ChapterId::AshenReliquary => assets.altar.clone(),
                ChapterId::HollowBastion => assets.soul_font.clone(),
            },
            layout.altar,
            1.2,
            Some(InteractableKind::Altar),
        ),
        (
            assets.sword.clone(),
            layout.weapon_shrine,
            1.0,
            Some(InteractableKind::WeaponShrine),
        ),
    ] {
        let mut entity = commands.spawn((
            WorldAssetRoot(scene),
            Transform::from_translation(position).with_scale(Vec3::splat(scale)),
            DungeonEntity,
        ));
        if let Some(kind) = interaction {
            entity.insert(Interactable {
                kind,
                radius: 2.0,
                used: false,
                reusable: false,
            });
            if let Some(profile) = dungeon_light_profile_for_interactable(kind) {
                spawn_dungeon_light(&mut commands, position, profile, "Interactable Light");
            }
        }
    }

    spawn_interactable_scene(
        &mut commands,
        assets.quartermaster.clone(),
        Transform::from_translation(chapter_world_position(chapter, Vec3::new(-4.8, 0.0, 5.9))),
        Interactable {
            kind: InteractableKind::Merchant,
            radius: 2.1,
            used: false,
            reusable: true,
        },
        "Reliquary Quartermaster",
    );
    spawn_interactable_scene(
        &mut commands,
        assets.fortune_shrine.clone(),
        Transform::from_translation(fortune_position),
        Interactable {
            kind: InteractableKind::FortuneShrine,
            radius: 1.85,
            used: false,
            reusable: false,
        },
        "Gilded Fortune Shrine",
    );
    spawn_interactable_scene(
        &mut commands,
        assets.storm_shrine.clone(),
        Transform::from_translation(chapter_world_position(chapter, Vec3::new(-2.8, 0.0, -7.1))),
        Interactable {
            kind: InteractableKind::StormShrine,
            radius: 1.85,
            used: false,
            reusable: false,
        },
        "Storm Conduit Shrine",
    );
    spawn_interactable_scene(
        &mut commands,
        assets.healing_well.clone(),
        Transform::from_translation(chapter_world_position(chapter, Vec3::new(2.0, 0.0, 6.7))),
        Interactable {
            kind: InteractableKind::HealingWell,
            radius: 1.85,
            used: false,
            reusable: false,
        },
        "Renewal Well",
    );

    let lore_page_names = match chapter {
        ChapterId::AshenReliquary => ["Steward's Warning", "Acolyte's Oath", "Malrec's Pact"],
        ChapterId::HollowBastion => ["The Last Bell", "Choir Litany", "Nyxara's Lament"],
    };
    for (name, position) in lore_page_names.into_iter().zip(layout.lore_pages) {
        commands.spawn((
            WorldAssetRoot(assets.lore_page.clone()),
            Transform::from_translation(position),
            DungeonEntity,
            Interactable {
                kind: InteractableKind::LorePage,
                radius: 1.45,
                used: false,
                reusable: false,
            },
            Name::new(name),
        ));
    }

    spawn_interactable_scene(
        &mut commands,
        assets.cursed_shrine.clone(),
        Transform::from_translation(chapter_world_position(chapter, Vec3::new(-9.0, 0.0, 1.6))),
        Interactable {
            kind: InteractableKind::CursedShrine,
            radius: 1.9,
            used: false,
            reusable: false,
        },
        "Cursed Reliquary Shrine",
    );
    spawn_interactable_scene(
        &mut commands,
        assets.blood_obelisk.clone(),
        Transform::from_translation(chapter_world_position(chapter, Vec3::new(10.2, 0.0, 2.9))),
        Interactable {
            kind: InteractableKind::BloodObelisk,
            radius: 1.9,
            used: false,
            reusable: false,
        },
        "Blood Obelisk",
    );
    spawn_interactable_scene(
        &mut commands,
        assets.reliquary_vault.clone(),
        Transform::from_translation(chapter_world_position(chapter, Vec3::new(11.0, 0.0, -5.6)))
            .with_scale(Vec3::splat(1.08)),
        Interactable {
            kind: InteractableKind::ReliquaryVault,
            radius: 2.0,
            used: false,
            reusable: false,
        },
        "Resplendent Reliquary Vault",
    );
    spawn_interactable_scene(
        &mut commands,
        assets.ember_rift_prop.clone(),
        Transform::from_translation(chapter_world_position(chapter, Vec3::new(-11.0, 0.0, -4.6)))
            .with_scale(Vec3::splat(1.18)),
        Interactable {
            kind: InteractableKind::EmberRift,
            radius: 2.05,
            used: false,
            reusable: false,
        },
        "Ember Rift",
    );
    spawn_interactable_scene(
        &mut commands,
        assets.ashen_pylon.clone(),
        Transform::from_translation(chapter_world_position(chapter, Vec3::new(4.4, 0.0, 7.0))),
        Interactable {
            kind: InteractableKind::AshenPylon,
            radius: 1.95,
            used: false,
            reusable: false,
        },
        "Ashen Pylon",
    );

    for spec in breakable_spawns(chapter) {
        spawn_breakable(&mut commands, &assets, spec);
    }

    spawn_hazard(
        &mut commands,
        &assets,
        chapter_world_position(chapter, Vec3::new(-5.6, 0.0, -1.0)),
        1.15,
        11.0 * modifier.hazard_damage_multiplier(),
        match chapter {
            ChapterId::AshenReliquary => "Ember vent",
            ChapterId::HollowBastion => "Soul fissure",
        },
        chapter,
    );
    spawn_hazard(
        &mut commands,
        &assets,
        chapter_world_position(chapter, Vec3::new(1.7, 0.0, -3.2)),
        1.05,
        10.0 * modifier.hazard_damage_multiplier(),
        match chapter {
            ChapterId::AshenReliquary => "Ember vent",
            ChapterId::HollowBastion => "Soul fissure",
        },
        chapter,
    );
    spawn_hazard(
        &mut commands,
        &assets,
        chapter_world_position(chapter, Vec3::new(6.7, 0.0, 1.2)),
        1.25,
        13.0 * modifier.hazard_damage_multiplier(),
        match chapter {
            ChapterId::AshenReliquary => "Keeper's flame vent",
            ChapterId::HollowBastion => "Matron's soul fissure",
        },
        chapter,
    );
}

fn spawn_outdoor_road(
    commands: &mut Commands,
    mesh: &Handle<Mesh>,
    material: &Handle<StandardMaterial>,
    from: Vec3,
    to: Vec3,
    width: f32,
    index: usize,
) {
    let delta = to - from;
    let length = Vec2::new(delta.x, delta.z).length();
    if length <= f32::EPSILON {
        return;
    }
    let yaw = delta.x.atan2(delta.z);
    commands.spawn((
        Mesh3d(mesh.clone()),
        MeshMaterial3d(material.clone()),
        Transform::from_translation((from + to) * 0.5 + Vec3::Y * 0.015)
            .with_rotation(Quat::from_rotation_y(yaw))
            .with_scale(Vec3::new(width, 0.07, length)),
        DungeonEntity,
        Name::new(format!("Weathered Outdoor Road {index}")),
    ));
}

#[cfg(test)]
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
enum OutdoorDecorationKind {
    RidgeRock,
    DeadTree,
    Crystal,
    BrokenColumn,
    RuinStone,
    Waystone,
    ArchPillar,
    ArchLintel,
}

#[cfg(test)]
#[derive(Debug, Clone, Copy)]
struct OutdoorDecorationSpec {
    kind: OutdoorDecorationKind,
    position: Vec3,
    scale: Vec3,
    yaw: f32,
}

#[cfg(test)]
fn outdoor_decoration_specs(chapter: ChapterId) -> Vec<OutdoorDecorationSpec> {
    let mut specs = Vec::with_capacity(170);
    let (phase, mirror) = match chapter {
        ChapterId::AshenReliquary => (0.0, 1.0),
        ChapterId::HollowBastion => (0.47, -1.0),
    };

    // Discontinuous boulder ridges imply distant terrain without enclosing the
    // player in a continuous wall.
    for index in 0..64 {
        if index % 9 == 4 {
            continue;
        }
        let angle = index as f32 * std::f32::consts::TAU / 64.0 + phase * 0.23;
        let jitter = ((index * 17 % 11) as f32 - 5.0) * 0.28;
        let scale = Vec3::new(
            1.6 + (index % 5) as f32 * 0.22,
            1.2 + (index % 7) as f32 * 0.18,
            1.5 + (index % 3) as f32 * 0.3,
        );
        specs.push(OutdoorDecorationSpec {
            kind: OutdoorDecorationKind::RidgeRock,
            position: Vec3::new(
                mirror * angle.cos() * (45.0 + jitter),
                scale.y * 0.38,
                WORLD_CENTER_Z + angle.sin() * (52.0 + jitter),
            ),
            scale,
            yaw: angle * 1.7,
        });
    }

    // A low-discrepancy spiral fills the valley with sparse silhouettes while
    // leaving broad sightlines between landmark clearings.
    for index in 0..64 {
        let angle = index as f32 * 2.399_963_1 + phase;
        let radius = 20.0 + (index * 17 % 19) as f32 * 1.32;
        let x = mirror * angle.cos() * radius;
        let z = WORLD_CENTER_Z + angle.sin() * radius * 1.12;
        let dead_tree = match chapter {
            ChapterId::AshenReliquary => index % 5 != 0,
            ChapterId::HollowBastion => index % 4 == 0,
        };
        let (kind, scale, y) = if dead_tree {
            let height = 2.8 + (index % 7) as f32 * 0.38;
            (
                OutdoorDecorationKind::DeadTree,
                Vec3::new(
                    0.26 + (index % 3) as f32 * 0.07,
                    height,
                    0.26 + (index % 4) as f32 * 0.045,
                ),
                height * 0.5,
            )
        } else {
            let height = 1.1 + (index % 6) as f32 * 0.22;
            (
                OutdoorDecorationKind::Crystal,
                Vec3::new(
                    0.48 + (index % 3) as f32 * 0.09,
                    height,
                    0.48 + (index % 4) as f32 * 0.065,
                ),
                height,
            )
        };
        specs.push(OutdoorDecorationSpec {
            kind,
            position: Vec3::new(x, y, z),
            scale,
            yaw: angle + index as f32 * 0.11,
        });
    }

    let ruin_centers = [
        Vec2::new(-31.0, -45.0),
        Vec2::new(30.0, -51.0),
        Vec2::new(-29.0, 7.0),
        Vec2::new(27.0, 11.0),
        Vec2::new(4.0, -70.0),
    ];
    for index in 0..30 {
        let center = ruin_centers[index % ruin_centers.len()];
        let angle = index as f32 * 1.73 + phase * 2.0;
        let distance = 2.4 + (index % 6) as f32 * 0.72;
        let kind = match index % 5 {
            0 | 1 => OutdoorDecorationKind::BrokenColumn,
            2 | 3 => OutdoorDecorationKind::RuinStone,
            _ => OutdoorDecorationKind::Waystone,
        };
        let (scale, y) = match kind {
            OutdoorDecorationKind::BrokenColumn => {
                let height = 1.8 + (index % 5) as f32 * 0.55;
                (Vec3::new(0.62, height, 0.62), height * 0.5)
            }
            OutdoorDecorationKind::Waystone => {
                let height = 1.5 + (index % 4) as f32 * 0.3;
                (Vec3::new(0.7, height, 0.5), height * 0.5)
            }
            _ => {
                let height = 0.55 + (index % 3) as f32 * 0.24;
                (
                    Vec3::new(1.4 + (index % 4) as f32 * 0.3, height, 0.75),
                    height * 0.5,
                )
            }
        };
        specs.push(OutdoorDecorationSpec {
            kind,
            position: Vec3::new(
                mirror * (center.x + angle.cos() * distance),
                y,
                center.y + angle.sin() * distance,
            ),
            scale,
            yaw: angle,
        });
    }

    for (center, yaw) in [
        (Vec2::new(-18.0, -38.0), 0.32 + phase),
        (Vec2::new(23.0, -58.0), -0.48 + phase),
        (Vec2::new(-11.0, 17.0), 0.71 + phase),
        (Vec2::new(33.0, -13.0), -0.18 + phase),
    ] {
        let center = Vec2::new(mirror * center.x, center.y);
        let right = Vec2::new(yaw.cos(), -yaw.sin());
        for side in [-1.0, 1.0] {
            let position = center + right * 2.25 * side;
            specs.push(OutdoorDecorationSpec {
                kind: OutdoorDecorationKind::ArchPillar,
                position: Vec3::new(position.x, 2.2, position.y),
                scale: Vec3::new(0.72, 4.4, 0.72),
                yaw,
            });
        }
        specs.push(OutdoorDecorationSpec {
            kind: OutdoorDecorationKind::ArchLintel,
            position: Vec3::new(center.x, 4.55, center.y),
            scale: Vec3::new(5.4, 0.66, 0.82),
            yaw,
        });
    }

    specs
}

fn breakable_spawns(chapter: ChapterId) -> [BreakableSpawn; BREAKER_TARGET_BREAKABLES as usize] {
    let mut spawns = [
        BreakableSpawn {
            position: Vec3::new(-7.4, 0.0, 1.8),
            gold: 9,
            potions: 0,
            fury: 14.0,
            label: "Ashen Bone Urn",
        },
        BreakableSpawn {
            position: Vec3::new(-1.8, 0.0, 6.5),
            gold: 12,
            potions: 1,
            fury: 10.0,
            label: "Reliquary Offering Box",
        },
        BreakableSpawn {
            position: Vec3::new(4.8, 0.0, -6.6),
            gold: 8,
            potions: 0,
            fury: 16.0,
            label: "Cracked Ossuary Jar",
        },
        BreakableSpawn {
            position: Vec3::new(10.1, 0.0, -2.8),
            gold: 14,
            potions: 1,
            fury: 12.0,
            label: "Sealed Grave Coffer",
        },
    ];
    for spawn in &mut spawns {
        spawn.position = chapter_world_position(chapter, spawn.position);
    }
    spawns
}

fn spawn_interactable_scene(
    commands: &mut Commands,
    scene: Handle<WorldAsset>,
    transform: Transform,
    interactable: Interactable,
    name: &'static str,
) {
    let light_position = transform.translation;
    let light_profile = dungeon_light_profile_for_interactable(interactable.kind);
    commands.spawn((
        WorldAssetRoot(scene),
        transform,
        DungeonEntity,
        interactable,
        Name::new(name),
    ));
    if let Some(profile) = light_profile {
        spawn_dungeon_light(commands, light_position, profile, "Interactable Light");
    }
}

fn spawn_breakable(commands: &mut Commands, assets: &GameAssets, spec: BreakableSpawn) {
    commands.spawn((
        WorldAssetRoot(breakable_scene(assets, spec.label)),
        Transform::from_translation(spec.position),
        Health {
            current: 18.0,
            max: 18.0,
        },
        Breakable {
            gold: spec.gold,
            potions: spec.potions,
            fury: spec.fury,
            label: spec.label,
        },
        DungeonEntity,
        Name::new(spec.label),
    ));
}

fn breakable_scene(assets: &GameAssets, label: &str) -> Handle<WorldAsset> {
    if label.contains("Box") || label.contains("Coffer") {
        assets.breakable_coffer.clone()
    } else {
        assets.breakable_urn.clone()
    }
}

fn spawn_hazard(
    commands: &mut Commands,
    assets: &GameAssets,
    position: Vec3,
    radius: f32,
    damage: f32,
    label: &'static str,
    chapter: ChapterId,
) {
    commands.spawn((
        WorldAssetRoot(match chapter {
            ChapterId::AshenReliquary => assets.ember_vent.clone(),
            ChapterId::HollowBastion => assets.soul_fissure.clone(),
        }),
        Transform::from_translation(position).with_scale(hazard_visual_scale(radius)),
        DungeonEntity,
        DungeonHazard {
            damage,
            radius,
            pulse: Timer::from_seconds(0.85, TimerMode::Repeating),
            label,
        },
        Name::new(label),
    ));
    spawn_dungeon_light(
        commands,
        position,
        dungeon_hazard_light_profile(radius),
        "Ember Vent Light",
    );
}

fn hazard_visual_scale(radius: f32) -> Vec3 {
    Vec3::splat(radius.max(0.1))
}

fn spawn_dungeon_light(
    commands: &mut Commands,
    position: Vec3,
    profile: DungeonLightProfile,
    name: &'static str,
) -> Entity {
    commands
        .spawn((
            PointLight {
                color: profile.color,
                intensity: profile.intensity,
                range: profile.range,
                shadow_maps_enabled: false,
                ..default()
            },
            Transform::from_translation(position + Vec3::Y * 1.15),
            DungeonLightPulse {
                base_intensity: profile.intensity,
                amplitude: profile.pulse_amplitude,
                speed: profile.pulse_speed,
                phase: position.x * 0.37 + position.z * 0.19,
            },
            DungeonEntity,
            Name::new(name),
        ))
        .id()
}

fn update_dungeon_light_pulses(
    time: Res<Time>,
    mut lights: Query<(&mut PointLight, &DungeonLightPulse)>,
) {
    let elapsed = time.elapsed_secs();
    for (mut light, pulse) in &mut lights {
        light.intensity = dungeon_light_intensity_at(
            pulse.base_intensity,
            pulse.amplitude,
            pulse.speed,
            pulse.phase,
            elapsed,
        );
    }
}

fn dungeon_light_intensity_at(
    base_intensity: f32,
    amplitude: f32,
    speed: f32,
    phase: f32,
    elapsed_secs: f32,
) -> f32 {
    let wave = (elapsed_secs * speed + phase).sin() * 0.5 + 0.5;
    base_intensity + wave * amplitude.max(0.0)
}

fn dungeon_light_profile_for_interactable(kind: InteractableKind) -> Option<DungeonLightProfile> {
    let profile = match kind {
        InteractableKind::Cache => DungeonLightProfile {
            color: Color::srgb(1.0, 0.72, 0.28),
            intensity: 280.0,
            range: 5.0,
            pulse_amplitude: 90.0,
            pulse_speed: 1.6,
        },
        InteractableKind::Altar => DungeonLightProfile {
            color: Color::srgb(1.0, 0.22, 0.06),
            intensity: 520.0,
            range: 6.5,
            pulse_amplitude: 210.0,
            pulse_speed: 2.4,
        },
        InteractableKind::WeaponShrine => DungeonLightProfile {
            color: Color::srgb(1.0, 0.86, 0.32),
            intensity: 360.0,
            range: 5.2,
            pulse_amplitude: 120.0,
            pulse_speed: 1.8,
        },
        InteractableKind::Merchant => DungeonLightProfile {
            color: Color::srgb(0.40, 0.72, 1.0),
            intensity: 300.0,
            range: 5.4,
            pulse_amplitude: 70.0,
            pulse_speed: 1.2,
        },
        InteractableKind::FortuneShrine => DungeonLightProfile {
            color: Color::srgb(1.0, 0.76, 0.18),
            intensity: 500.0,
            range: 6.0,
            pulse_amplitude: 180.0,
            pulse_speed: 1.9,
        },
        InteractableKind::StormShrine => DungeonLightProfile {
            color: Color::srgb(0.22, 0.62, 1.0),
            intensity: 560.0,
            range: 6.2,
            pulse_amplitude: 260.0,
            pulse_speed: 4.0,
        },
        InteractableKind::HealingWell => DungeonLightProfile {
            color: Color::srgb(0.28, 0.94, 0.86),
            intensity: 430.0,
            range: 5.8,
            pulse_amplitude: 130.0,
            pulse_speed: 1.4,
        },
        InteractableKind::CursedShrine => DungeonLightProfile {
            color: Color::srgb(0.78, 0.08, 0.92),
            intensity: 460.0,
            range: 5.8,
            pulse_amplitude: 190.0,
            pulse_speed: 2.2,
        },
        InteractableKind::BloodObelisk => DungeonLightProfile {
            color: Color::srgb(0.95, 0.04, 0.08),
            intensity: 520.0,
            range: 6.4,
            pulse_amplitude: 230.0,
            pulse_speed: 2.7,
        },
        InteractableKind::ReliquaryVault => DungeonLightProfile {
            color: Color::srgb(1.0, 0.78, 0.30),
            intensity: 420.0,
            range: 5.6,
            pulse_amplitude: 100.0,
            pulse_speed: 1.1,
        },
        InteractableKind::EmberRift => DungeonLightProfile {
            color: Color::srgb(1.0, 0.20, 0.04),
            intensity: 680.0,
            range: 7.2,
            pulse_amplitude: 280.0,
            pulse_speed: 3.1,
        },
        InteractableKind::AshenPylon => DungeonLightProfile {
            color: Color::srgb(1.0, 0.38, 0.08),
            intensity: 620.0,
            range: 6.6,
            pulse_amplitude: 240.0,
            pulse_speed: 2.5,
        },
        InteractableKind::LorePage => return None,
    };
    Some(profile)
}

fn dungeon_hazard_light_profile(radius: f32) -> DungeonLightProfile {
    DungeonLightProfile {
        color: Color::srgb(1.0, 0.24, 0.04),
        intensity: 360.0 * radius.max(0.5),
        range: 4.2 * radius.max(0.5),
        pulse_amplitude: 190.0 * radius.max(0.5),
        pulse_speed: 3.6,
    }
}

fn despawn_dungeon(
    mut commands: Commands,
    query: Query<Entity, (With<DungeonEntity>, Without<ChildOf>)>,
) {
    for entity in &query {
        if let Ok(mut entity_commands) = commands.get_entity(entity) {
            entity_commands.try_despawn();
        }
    }
}

fn use_interactables(
    keyboard: Res<ButtonInput<KeyCode>>,
    player: Query<&Transform, With<Player>>,
    progress: Res<ChapterProgress>,
    mut interactables: Query<(Entity, &Transform, &mut Interactable)>,
    mut used_events: MessageWriter<InteractableUsed>,
) {
    if !keyboard.just_pressed(KeyCode::Space) {
        return;
    }
    let Ok(player_transform) = player.single() else {
        return;
    };

    let mut selected = None;
    for (entity, transform, interactable) in &mut interactables {
        if !interactable_available(&progress, &interactable) {
            continue;
        }
        let distance = player_transform.translation.distance(transform.translation);
        if distance <= interactable.radius {
            selected = select_better_interactable_activation(
                selected,
                InteractableActivation {
                    entity,
                    kind: interactable.kind,
                    position: transform.translation,
                    distance,
                    rank: interactable_activation_rank(&progress, &interactable),
                },
            );
        }
    }

    let Some(selected) = selected else {
        return;
    };

    if let Ok((_, _, mut interactable)) = interactables.get_mut(selected.entity)
        && interactable_available(&progress, &interactable)
    {
        if !interactable.reusable {
            interactable.used = true;
        }
        used_events.write(InteractableUsed {
            kind: selected.kind,
            position: selected.position,
        });
    }
}

#[derive(Debug, Clone, Copy)]
struct InteractableActivation {
    entity: Entity,
    kind: InteractableKind,
    position: Vec3,
    distance: f32,
    rank: u8,
}

fn interactable_available(progress: &ChapterProgress, interactable: &Interactable) -> bool {
    if interactable.used && !interactable.reusable {
        return false;
    }

    match interactable.kind {
        InteractableKind::Cache => progress.phase == ChapterPhase::Cache,
        InteractableKind::Altar => progress.phase == ChapterPhase::Ritual,
        _ => true,
    }
}

fn interactable_activation_rank(progress: &ChapterProgress, interactable: &Interactable) -> u8 {
    match objective_marker_state(progress, interactable) {
        ObjectiveMarkerState::Primary => 0,
        ObjectiveMarkerState::Optional => 1,
        ObjectiveMarkerState::Hidden => 2,
    }
}

fn select_better_interactable_activation(
    current: Option<InteractableActivation>,
    candidate: InteractableActivation,
) -> Option<InteractableActivation> {
    match current {
        None => Some(candidate),
        Some(best)
            if candidate.rank < best.rank
                || (candidate.rank == best.rank && candidate.distance < best.distance) =>
        {
            Some(candidate)
        }
        Some(best) => Some(best),
    }
}

fn attach_objective_markers(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    assets: Res<GameAssets>,
    interactables: Query<Entity, (With<Interactable>, Without<ObjectiveMarkerAttached>)>,
) {
    if interactables.is_empty() {
        return;
    }

    let ring_mesh = meshes.add(Torus {
        major_radius: 0.79,
        minor_radius: 0.014,
    });
    let ring_material = materials.add(StandardMaterial {
        base_color: Color::srgba(0.16, 0.74, 1.0, 0.42),
        emissive: Color::srgb(0.04, 0.28, 0.52).into(),
        alpha_mode: AlphaMode::Opaque,
        ..default()
    });

    for entity in &interactables {
        let ring = commands
            .spawn((
                Mesh3d(ring_mesh.clone()),
                MeshMaterial3d(ring_material.clone()),
                Transform::from_xyz(0.0, 0.08, 0.0),
                Visibility::Hidden,
                ObjectiveMarkerVisual {
                    owner: entity,
                    role: ObjectiveMarkerRole::Ring,
                },
                DungeonEntity,
                Name::new("Objective Ring"),
            ))
            .id();
        let beacon = commands
            .spawn((
                WorldAssetRoot(assets.objective_sigil.clone()),
                Transform::from_xyz(
                    0.0,
                    objective_marker_beacon_height(ObjectiveMarkerState::Optional),
                    0.0,
                ),
                Visibility::Hidden,
                ObjectiveMarkerVisual {
                    owner: entity,
                    role: ObjectiveMarkerRole::Beacon,
                },
                DungeonEntity,
                Name::new("Objective Beacon"),
            ))
            .id();
        let prompt = commands
            .spawn((
                Text2d::new(""),
                TextFont {
                    font_size: FontSize::Px(24.0),
                    ..default()
                },
                TextColor(Color::srgb(0.94, 0.90, 0.78)),
                Transform::from_xyz(
                    0.0,
                    objective_prompt_height(ObjectiveMarkerState::Optional),
                    0.0,
                )
                .with_scale(Vec3::splat(0.011)),
                Visibility::Hidden,
                ObjectivePromptVisual { owner: entity },
                DungeonEntity,
                Name::new("Interaction Prompt"),
            ))
            .id();
        commands.entity(entity).add_child(ring);
        commands.entity(entity).add_child(beacon);
        commands.entity(entity).add_child(prompt);
        commands.entity(entity).try_insert(ObjectiveMarkerAttached);
    }
}

fn update_objective_markers(
    time: Res<Time>,
    progress: Res<ChapterProgress>,
    interactables: Query<&Interactable>,
    mut markers: Query<(&ObjectiveMarkerVisual, &mut Visibility, &mut Transform)>,
) {
    let pulse = (time.elapsed_secs() * 3.4).sin();
    let bob = (time.elapsed_secs() * 2.2).sin();

    for (marker, mut visibility, mut transform) in &mut markers {
        let state = interactables
            .get(marker.owner)
            .map(|interactable| objective_marker_state(&progress, interactable))
            .unwrap_or(ObjectiveMarkerState::Hidden);

        if state == ObjectiveMarkerState::Hidden {
            *visibility = Visibility::Hidden;
            continue;
        }
        *visibility = Visibility::Visible;

        match marker.role {
            ObjectiveMarkerRole::Ring => {
                let scale = objective_marker_ring_scale(state) + pulse * 0.045;
                transform.scale = Vec3::new(scale, scale, scale);
                transform.translation.y = 0.08;
            }
            ObjectiveMarkerRole::Beacon => {
                let scale = objective_marker_beacon_scale(state) + pulse * 0.025;
                transform.scale = Vec3::splat(scale);
                transform.translation.y = objective_marker_beacon_height(state) + bob * 0.12;
            }
        }
    }
}

fn update_objective_prompts(
    time: Res<Time>,
    progress: Res<ChapterProgress>,
    player: PromptPlayerQuery,
    interactables: Query<(&Interactable, &Transform), Without<ObjectivePromptVisual>>,
    mut prompts: ObjectivePromptQuery,
) {
    let Ok(player_transform) = player.single() else {
        for (_, mut visibility, _, _) in &mut prompts {
            *visibility = Visibility::Hidden;
        }
        return;
    };
    let bob = (time.elapsed_secs() * 2.6).sin();

    for (prompt, mut visibility, mut transform, mut text) in &mut prompts {
        let Ok((interactable, interactable_transform)) = interactables.get(prompt.owner) else {
            *visibility = Visibility::Hidden;
            continue;
        };
        let state = objective_marker_state(&progress, interactable);
        let distance = player_transform
            .translation
            .distance(interactable_transform.translation);

        if !interaction_prompt_visible(distance, interactable.radius, state) {
            *visibility = Visibility::Hidden;
            continue;
        }

        *visibility = Visibility::Visible;
        *text = Text2d::new(interaction_prompt_text(interactable.kind, state));
        transform.translation.y = objective_prompt_height(state) + bob * 0.08;
        transform.scale = Vec3::splat(objective_prompt_scale(state));
    }
}

fn objective_marker_state(
    progress: &ChapterProgress,
    interactable: &Interactable,
) -> ObjectiveMarkerState {
    if interactable.used && !interactable.reusable {
        return ObjectiveMarkerState::Hidden;
    }
    if is_primary_objective_interactable(progress.phase, interactable.kind) {
        return ObjectiveMarkerState::Primary;
    }
    if is_optional_guided_interactable(interactable.kind) {
        return ObjectiveMarkerState::Optional;
    }
    ObjectiveMarkerState::Hidden
}

fn is_primary_objective_interactable(phase: ChapterPhase, kind: InteractableKind) -> bool {
    matches!(
        (phase, kind),
        (ChapterPhase::Cache, InteractableKind::Cache)
            | (ChapterPhase::Ritual, InteractableKind::Altar)
    )
}

fn is_optional_guided_interactable(kind: InteractableKind) -> bool {
    matches!(
        kind,
        InteractableKind::Merchant
            | InteractableKind::WeaponShrine
            | InteractableKind::FortuneShrine
            | InteractableKind::StormShrine
            | InteractableKind::HealingWell
            | InteractableKind::LorePage
            | InteractableKind::CursedShrine
            | InteractableKind::BloodObelisk
            | InteractableKind::ReliquaryVault
            | InteractableKind::EmberRift
            | InteractableKind::AshenPylon
    )
}

fn objective_marker_ring_scale(state: ObjectiveMarkerState) -> f32 {
    match state {
        ObjectiveMarkerState::Primary => 1.25,
        ObjectiveMarkerState::Optional => 0.82,
        ObjectiveMarkerState::Hidden => 0.0,
    }
}

fn objective_marker_beacon_scale(state: ObjectiveMarkerState) -> f32 {
    match state {
        ObjectiveMarkerState::Primary => 1.0,
        ObjectiveMarkerState::Optional => 0.62,
        ObjectiveMarkerState::Hidden => 0.0,
    }
}

fn objective_marker_beacon_height(state: ObjectiveMarkerState) -> f32 {
    match state {
        ObjectiveMarkerState::Primary => 2.25,
        ObjectiveMarkerState::Optional => 1.72,
        ObjectiveMarkerState::Hidden => 0.0,
    }
}

fn objective_prompt_height(state: ObjectiveMarkerState) -> f32 {
    match state {
        ObjectiveMarkerState::Primary => 2.72,
        ObjectiveMarkerState::Optional => 2.08,
        ObjectiveMarkerState::Hidden => 0.0,
    }
}

fn objective_prompt_scale(state: ObjectiveMarkerState) -> f32 {
    match state {
        ObjectiveMarkerState::Primary => 0.013,
        ObjectiveMarkerState::Optional => 0.011,
        ObjectiveMarkerState::Hidden => 0.0,
    }
}

fn interaction_prompt_visible(distance: f32, radius: f32, state: ObjectiveMarkerState) -> bool {
    state != ObjectiveMarkerState::Hidden && distance <= radius + 0.75
}

fn interaction_prompt_text(kind: InteractableKind, state: ObjectiveMarkerState) -> &'static str {
    match (kind, state) {
        (InteractableKind::Cache, ObjectiveMarkerState::Primary) => "Space - Open cache | gear",
        (InteractableKind::Altar, ObjectiveMarkerState::Primary) => {
            "Space - Claim final seal | boss"
        }
        (InteractableKind::Merchant, _) => "Space - Quartermaster | craft",
        (InteractableKind::WeaponShrine, _) => "Space - Weapon shrine | damage",
        (InteractableKind::FortuneShrine, _) => "Space - Fortune shrine | loot",
        (InteractableKind::StormShrine, _) => "Space - Storm shrine | burst",
        (InteractableKind::HealingWell, _) => "Space - Healing well | recover",
        (InteractableKind::LorePage, _) => "Space - Read lore | story",
        (InteractableKind::CursedShrine, _) => "Space - Cursed shrine | elite risk",
        (InteractableKind::BloodObelisk, _) => "Space - Blood obelisk | timed",
        (InteractableKind::ReliquaryVault, _) => "Space - Open vault | treasure",
        (InteractableKind::EmberRift, _) => "Space - Enter rift | Echo",
        (InteractableKind::AshenPylon, _) => "Space - Ashen pylon | surge",
        _ => "Space - Interact",
    }
}

fn tick_dungeon_hazards(
    time: Res<Time>,
    mut hazards: Query<(&Transform, &mut DungeonHazard)>,
    mut player: HazardPlayerQuery,
    mut combat_events: MessageWriter<CombatEvent>,
) {
    let Ok((player_transform, mut health, mut barrier, evade, equipment, elixir)) =
        player.single_mut()
    else {
        return;
    };
    let player_armor = total_armor(equipment, elixir);

    for (transform, mut hazard) in &mut hazards {
        hazard.pulse.tick(time.delta());
        if !hazard.pulse.just_finished()
            || !hazard_hits_player(
                transform.translation,
                player_transform.translation,
                hazard.radius,
            )
        {
            continue;
        }
        let damage = mitigated_damage(hazard.damage, player_armor);
        apply_hazard_damage(&mut health, &mut barrier, evade, damage);
        combat_events.write(CombatEvent {
            text: format!("{} scorched you for {damage:.0}", hazard.label),
        });
    }
}

fn hazard_hits_player(hazard_position: Vec3, player_position: Vec3, radius: f32) -> bool {
    let hazard_flat = Vec2::new(hazard_position.x, hazard_position.z);
    let player_flat = Vec2::new(player_position.x, player_position.z);
    hazard_flat.distance(player_flat) <= radius
}

fn apply_hazard_damage(health: &mut Health, barrier: &mut Barrier, evade: &Evade, damage: f32) {
    apply_player_damage_with_evade(health, barrier, evade, damage);
}

fn reward_destroyed_breakables(
    mut commands: Commands,
    mut stats: ResMut<RunStats>,
    modifier: Res<ChapterModifier>,
    mut player: Query<(&mut PotionBelt, &mut Fury, &FortuneBuff), With<Player>>,
    breakables: Query<(Entity, &Breakable, &Health)>,
    mut combat_events: MessageWriter<CombatEvent>,
) {
    let Ok((mut potions, mut fury, fortune)) = player.single_mut() else {
        return;
    };

    for (entity, breakable, health) in &breakables {
        if health.current > 0.0 {
            continue;
        }
        let reward = apply_breakable_reward(
            breakable,
            &mut stats,
            &mut potions,
            &mut fury,
            fortune,
            *modifier,
        );
        combat_events.write(CombatEvent {
            text: format!(
                "{} shattered: +{} gold{}{}",
                breakable.label,
                reward.gold,
                if reward.potions > 0 { " +potion" } else { "" },
                if reward.fury > 0.0 { " +fury" } else { "" }
            ),
        });
        if let Ok(mut entity_commands) = commands.get_entity(entity) {
            entity_commands.try_despawn();
        }
    }
}

#[derive(Debug, PartialEq)]
struct AppliedBreakableReward {
    gold: u32,
    potions: u32,
    fury: f32,
}

fn apply_breakable_reward(
    breakable: &Breakable,
    stats: &mut RunStats,
    potions: &mut PotionBelt,
    fury: &mut Fury,
    fortune: &FortuneBuff,
    modifier: crate::ordeal::ChapterModifier,
) -> AppliedBreakableReward {
    let gold = modifier.scale_reward(fortune_gold_reward(breakable.gold, fortune));
    stats.gold += gold;
    stats.breakables_smashed = stats.breakables_smashed.saturating_add(1);
    let potion_before = potions.current;
    potions.current = (potions.current + breakable.potions).min(potions.max);
    let fury_gained = fury.gain(breakable.fury);
    AppliedBreakableReward {
        gold,
        potions: potions.current - potion_before,
        fury: fury_gained,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn chapter_zone_for_position_names_major_route_areas() {
        let world = |legacy| chapter_world_position(ChapterId::AshenReliquary, legacy);
        assert_eq!(
            chapter_zone_for_position(world(Vec3::new(-8.0, 0.0, -5.5))),
            ChapterZone::CacheOssuary
        );
        assert_eq!(
            chapter_zone_for_position(world(Vec3::new(0.0, 0.0, -6.8))),
            ChapterZone::OuterSanctum
        );
        assert_eq!(
            chapter_zone_for_position(world(Vec3::new(10.0, 0.0, -5.0))),
            ChapterZone::GildedVault
        );
        assert_eq!(
            chapter_zone_for_position(world(Vec3::new(7.0, 0.0, 4.8))),
            ChapterZone::EmberAltar
        );
        assert_eq!(
            chapter_zone_for_position(world(Vec3::new(0.0, 0.0, 6.7))),
            ChapterZone::QuartermasterNave
        );
        assert_eq!(
            chapter_zone_for_position(Vec3::ZERO),
            ChapterZone::ReliquaryCrossing
        );
        assert_eq!(
            chapter_zone_for_position(Vec3::new(640.0, 0.0, -960.0)),
            ChapterZone::Wilderness
        );

        let hollow_cache =
            chapter_world_position(ChapterId::HollowBastion, Vec3::new(-8.0, 0.0, -5.5));
        assert_eq!(
            chapter_zone_for_chapter_position(ChapterId::HollowBastion, hollow_cache),
            ChapterZone::CacheOssuary
        );
    }

    #[test]
    fn chapter_zone_labels_surface_tactical_route_roles() {
        let zone = ChapterZone::QuartermasterNave;

        assert_eq!(zone.label(), "Wayfarer Camp");
        assert!(zone.tactical_hint().contains("vendor"));
        assert!(zone.tactical_hint().contains("portal"));
    }

    #[test]
    fn hazard_hit_test_uses_flat_radius() {
        assert!(hazard_hits_player(
            Vec3::new(1.0, 0.0, 1.0),
            Vec3::new(1.5, 2.0, 1.5),
            1.0
        ));
        assert!(!hazard_hits_player(
            Vec3::new(1.0, 0.0, 1.0),
            Vec3::new(3.0, 0.0, 3.0),
            1.0
        ));
    }

    #[test]
    fn hazard_damage_never_drops_below_zero() {
        let mut health = Health {
            current: 4.0,
            max: 20.0,
        };
        let mut barrier = Barrier {
            current: 0.0,
            max: 10.0,
        };
        let evade = Evade {
            active: Timer::from_seconds(0.0, TimerMode::Once),
            cooldown: Timer::from_seconds(0.0, TimerMode::Once),
            cooldown_secs: 4.5,
            speed_multiplier: 2.65,
        };
        apply_hazard_damage(&mut health, &mut barrier, &evade, 9.0);
        assert_eq!(health.current, 0.0);
    }

    #[test]
    fn hazard_art_scales_to_damage_radius() {
        assert_eq!(hazard_visual_scale(1.25), Vec3::splat(1.25));
        assert_eq!(hazard_visual_scale(0.0), Vec3::splat(0.1));
    }

    #[test]
    fn breakable_rewards_clamp_supplies_and_resource() {
        let breakable = Breakable {
            gold: 12,
            potions: 2,
            fury: 20.0,
            label: "Test Urn",
        };
        let mut stats = RunStats::default();
        let mut potions = PotionBelt {
            current: 4,
            max: 5,
            heal_amount: 45.0,
            cooldown: Timer::from_seconds(0.0, TimerMode::Once),
            cooldown_secs: crate::player::potion_cooldown_secs_for_capacity(5),
        };
        let mut fury = Fury {
            current: 90.0,
            max: 100.0,
            basic_gain: 18.0,
            dash_cost: 25.0,
            nova_cost: 45.0,
            rupture_cost: 32.0,
        };
        let fortune = FortuneBuff {
            timer: Timer::from_seconds(0.0, TimerMode::Once),
            xp_multiplier: 1.25,
            gold_multiplier: 1.50,
        };

        let reward = apply_breakable_reward(
            &breakable,
            &mut stats,
            &mut potions,
            &mut fury,
            &fortune,
            crate::ordeal::ChapterModifier {
                kind: crate::ordeal::ChapterModifierKind::AshenEchoes,
                affix: crate::ordeal::OrdealAffix::None,
            },
        );

        assert_eq!(
            reward,
            AppliedBreakableReward {
                gold: 12,
                potions: 1,
                fury: 10.0,
            }
        );
        assert_eq!(stats.gold, 12);
        assert_eq!(stats.breakables_smashed, 1);
        assert_eq!(potions.current, 5);
        assert_eq!(fury.current, 100.0);
    }

    #[test]
    fn breaker_target_matches_spawned_breakables() {
        assert_eq!(
            breakable_spawns(ChapterId::AshenReliquary).len(),
            BREAKER_TARGET_BREAKABLES as usize
        );
    }

    #[test]
    fn objective_markers_track_mainline_phase_and_usage() {
        let mut progress = ChapterProgress {
            phase: ChapterPhase::Cache,
            ..default()
        };
        let cache = Interactable {
            kind: InteractableKind::Cache,
            radius: 2.0,
            used: false,
            reusable: false,
        };
        let altar = Interactable {
            kind: InteractableKind::Altar,
            radius: 2.0,
            used: false,
            reusable: false,
        };
        let used_cache = Interactable {
            used: true,
            ..cache
        };

        assert_eq!(
            objective_marker_state(&progress, &cache),
            ObjectiveMarkerState::Primary
        );
        assert_eq!(
            objective_marker_state(&progress, &altar),
            ObjectiveMarkerState::Hidden
        );
        assert_eq!(
            objective_marker_state(&progress, &used_cache),
            ObjectiveMarkerState::Hidden
        );

        progress.phase = ChapterPhase::Ritual;
        assert_eq!(
            objective_marker_state(&progress, &altar),
            ObjectiveMarkerState::Primary
        );
    }

    #[test]
    fn objective_markers_keep_optional_events_visible() {
        let progress = ChapterProgress::default();
        let merchant = Interactable {
            kind: InteractableKind::Merchant,
            radius: 2.0,
            used: true,
            reusable: true,
        };
        let shrine = Interactable {
            kind: InteractableKind::StormShrine,
            radius: 2.0,
            used: false,
            reusable: false,
        };

        assert_eq!(
            objective_marker_state(&progress, &merchant),
            ObjectiveMarkerState::Optional
        );
        assert_eq!(
            objective_marker_state(&progress, &shrine),
            ObjectiveMarkerState::Optional
        );
        assert!(
            objective_marker_ring_scale(ObjectiveMarkerState::Primary)
                > objective_marker_ring_scale(ObjectiveMarkerState::Optional)
        );
        assert!(
            objective_marker_beacon_height(ObjectiveMarkerState::Primary)
                > objective_marker_beacon_height(ObjectiveMarkerState::Optional)
        );
    }

    #[test]
    fn interaction_selection_prefers_primary_objective_over_nearer_optional() {
        let progress = ChapterProgress {
            phase: ChapterPhase::Cache,
            ..default()
        };
        let cache = Interactable {
            kind: InteractableKind::Cache,
            radius: 2.0,
            used: false,
            reusable: false,
        };
        let merchant = Interactable {
            kind: InteractableKind::Merchant,
            radius: 2.0,
            used: false,
            reusable: true,
        };
        let cache_entity = Entity::from_raw_u32(1).unwrap();
        let merchant_entity = Entity::from_raw_u32(2).unwrap();

        let selected = select_better_interactable_activation(
            select_better_interactable_activation(
                None,
                InteractableActivation {
                    entity: merchant_entity,
                    kind: merchant.kind,
                    position: Vec3::new(0.4, 0.0, 0.0),
                    distance: 0.4,
                    rank: interactable_activation_rank(&progress, &merchant),
                },
            ),
            InteractableActivation {
                entity: cache_entity,
                kind: cache.kind,
                position: Vec3::new(1.4, 0.0, 0.0),
                distance: 1.4,
                rank: interactable_activation_rank(&progress, &cache),
            },
        )
        .unwrap();

        assert_eq!(selected.entity, cache_entity);
        assert_eq!(selected.kind, InteractableKind::Cache);
        assert_eq!(interactable_activation_rank(&progress, &cache), 0);
        assert_eq!(interactable_activation_rank(&progress, &merchant), 1);
    }

    fn run_mainline_interaction(
        phase: ChapterPhase,
        kind: InteractableKind,
    ) -> (bool, Vec<InteractableKind>) {
        let mut app = App::new();
        app.insert_resource(ButtonInput::<KeyCode>::default())
            .insert_resource(ChapterProgress { phase, ..default() })
            .add_message::<InteractableUsed>()
            .add_systems(Update, use_interactables);

        app.world_mut().spawn((Player, Transform::default()));
        let interactable_entity = app
            .world_mut()
            .spawn((
                Interactable {
                    kind,
                    radius: 2.0,
                    used: false,
                    reusable: false,
                },
                Transform::default(),
            ))
            .id();
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .press(KeyCode::Space);

        app.update();

        let used = app
            .world()
            .get::<Interactable>(interactable_entity)
            .unwrap()
            .used;
        let messages = app.world().resource::<Messages<InteractableUsed>>();
        let mut cursor = messages.get_cursor();
        let emitted = cursor.read(messages).map(|event| event.kind).collect();
        (used, emitted)
    }

    #[test]
    fn early_mainline_interactions_do_not_consume_or_emit() {
        let phases = [
            ChapterPhase::Breach,
            ChapterPhase::Cache,
            ChapterPhase::Sanctum,
            ChapterPhase::Ritual,
            ChapterPhase::Boss,
            ChapterPhase::Victory,
        ];
        for (kind, required_phase) in [
            (InteractableKind::Cache, ChapterPhase::Cache),
            (InteractableKind::Altar, ChapterPhase::Ritual),
        ] {
            for phase in phases {
                if phase == required_phase {
                    continue;
                }
                let (used, emitted) = run_mainline_interaction(phase, kind);
                assert!(!used, "{kind:?} was consumed during {phase:?}");
                assert!(
                    emitted.is_empty(),
                    "{kind:?} emitted InteractableUsed during {phase:?}"
                );
            }
        }
    }

    #[test]
    fn mainline_interactions_consume_and_emit_in_required_phase() {
        for (phase, kind) in [
            (ChapterPhase::Cache, InteractableKind::Cache),
            (ChapterPhase::Ritual, InteractableKind::Altar),
        ] {
            let (used, emitted) = run_mainline_interaction(phase, kind);
            assert!(used, "{kind:?} was not consumed during {phase:?}");
            assert_eq!(emitted, vec![kind]);
        }
    }

    #[test]
    fn interaction_selection_uses_nearest_target_with_same_priority() {
        let progress = ChapterProgress::default();
        let shrine = Interactable {
            kind: InteractableKind::StormShrine,
            radius: 2.0,
            used: false,
            reusable: false,
        };
        let well = Interactable {
            kind: InteractableKind::HealingWell,
            radius: 2.0,
            used: false,
            reusable: false,
        };
        let far_entity = Entity::from_raw_u32(3).unwrap();
        let near_entity = Entity::from_raw_u32(4).unwrap();

        let selected = select_better_interactable_activation(
            select_better_interactable_activation(
                None,
                InteractableActivation {
                    entity: far_entity,
                    kind: shrine.kind,
                    position: Vec3::new(1.5, 0.0, 0.0),
                    distance: 1.5,
                    rank: interactable_activation_rank(&progress, &shrine),
                },
            ),
            InteractableActivation {
                entity: near_entity,
                kind: well.kind,
                position: Vec3::new(0.7, 0.0, 0.0),
                distance: 0.7,
                rank: interactable_activation_rank(&progress, &well),
            },
        )
        .unwrap();

        assert_eq!(selected.entity, near_entity);
        assert_eq!(selected.kind, InteractableKind::HealingWell);
        assert!(interactable_available(&progress, &well));
        assert!(!interactable_available(
            &progress,
            &Interactable {
                used: true,
                ..shrine
            }
        ));
    }

    #[test]
    fn interaction_prompts_only_show_for_near_visible_objectives() {
        assert!(interaction_prompt_visible(
            2.7,
            2.0,
            ObjectiveMarkerState::Primary
        ));
        assert!(interaction_prompt_visible(
            2.7,
            2.0,
            ObjectiveMarkerState::Optional
        ));
        assert!(!interaction_prompt_visible(
            2.8,
            2.0,
            ObjectiveMarkerState::Optional
        ));
        assert!(!interaction_prompt_visible(
            1.0,
            2.0,
            ObjectiveMarkerState::Hidden
        ));
    }

    #[test]
    fn interaction_prompts_name_the_available_action() {
        assert_eq!(
            interaction_prompt_text(InteractableKind::Cache, ObjectiveMarkerState::Primary),
            "Space - Open cache | gear"
        );
        assert_eq!(
            interaction_prompt_text(InteractableKind::Altar, ObjectiveMarkerState::Primary),
            "Space - Claim final seal | boss"
        );
        assert_eq!(
            interaction_prompt_text(InteractableKind::Merchant, ObjectiveMarkerState::Optional),
            "Space - Quartermaster | craft"
        );
        assert_eq!(
            interaction_prompt_text(InteractableKind::LorePage, ObjectiveMarkerState::Optional),
            "Space - Read lore | story"
        );
        assert_eq!(
            interaction_prompt_text(
                InteractableKind::CursedShrine,
                ObjectiveMarkerState::Optional
            ),
            "Space - Cursed shrine | elite risk"
        );
    }

    #[test]
    fn interaction_prompt_role_labels_stay_short() {
        let labeled_prompts = [
            InteractableKind::Cache,
            InteractableKind::Altar,
            InteractableKind::Merchant,
            InteractableKind::WeaponShrine,
            InteractableKind::FortuneShrine,
            InteractableKind::StormShrine,
            InteractableKind::HealingWell,
            InteractableKind::LorePage,
            InteractableKind::CursedShrine,
            InteractableKind::BloodObelisk,
            InteractableKind::ReliquaryVault,
            InteractableKind::EmberRift,
            InteractableKind::AshenPylon,
        ];

        for kind in labeled_prompts {
            let text = interaction_prompt_text(kind, ObjectiveMarkerState::Optional);
            assert!(
                text.len() <= 38,
                "{text} is too long for a world-space prompt"
            );
        }
    }

    #[test]
    fn primary_interaction_prompts_are_more_prominent() {
        assert!(
            objective_prompt_height(ObjectiveMarkerState::Primary)
                > objective_prompt_height(ObjectiveMarkerState::Optional)
        );
        assert!(
            objective_prompt_scale(ObjectiveMarkerState::Primary)
                > objective_prompt_scale(ObjectiveMarkerState::Optional)
        );
    }

    #[test]
    fn dungeon_light_profiles_match_encounter_readability() {
        let altar = dungeon_light_profile_for_interactable(InteractableKind::Altar).unwrap();
        let merchant = dungeon_light_profile_for_interactable(InteractableKind::Merchant).unwrap();
        let rift = dungeon_light_profile_for_interactable(InteractableKind::EmberRift).unwrap();
        let lore = dungeon_light_profile_for_interactable(InteractableKind::LorePage);
        let hazard = dungeon_hazard_light_profile(1.25);

        assert!(altar.intensity > merchant.intensity);
        assert!(rift.range > altar.range);
        assert!(hazard.intensity > merchant.intensity);
        assert!(lore.is_none());
    }

    #[test]
    fn dungeon_light_pulse_stays_within_profile_bounds() {
        let profile =
            dungeon_light_profile_for_interactable(InteractableKind::StormShrine).unwrap();
        let low = dungeon_light_intensity_at(
            profile.intensity,
            profile.pulse_amplitude,
            profile.pulse_speed,
            0.0,
            std::f32::consts::PI * 1.5 / profile.pulse_speed,
        );
        let high = dungeon_light_intensity_at(
            profile.intensity,
            profile.pulse_amplitude,
            profile.pulse_speed,
            0.0,
            std::f32::consts::FRAC_PI_2 / profile.pulse_speed,
        );

        assert!((low - profile.intensity).abs() < 0.01);
        assert!((high - (profile.intensity + profile.pulse_amplitude)).abs() < 0.01);
    }

    #[test]
    fn fortune_buff_multiplies_breakable_gold() {
        let breakable = Breakable {
            gold: 10,
            potions: 0,
            fury: 0.0,
            label: "Test Coffer",
        };
        let mut stats = RunStats::default();
        let mut potions = PotionBelt {
            current: 0,
            max: 5,
            heal_amount: 45.0,
            cooldown: Timer::from_seconds(0.0, TimerMode::Once),
            cooldown_secs: crate::player::potion_cooldown_secs_for_capacity(5),
        };
        let mut fury = Fury {
            current: 0.0,
            max: 100.0,
            basic_gain: 18.0,
            dash_cost: 25.0,
            nova_cost: 45.0,
            rupture_cost: 32.0,
        };
        let mut fortune = FortuneBuff {
            timer: Timer::from_seconds(10.0, TimerMode::Once),
            xp_multiplier: 1.25,
            gold_multiplier: 1.50,
        };
        fortune.timer.reset();

        let reward = apply_breakable_reward(
            &breakable,
            &mut stats,
            &mut potions,
            &mut fury,
            &fortune,
            crate::ordeal::ChapterModifier {
                kind: crate::ordeal::ChapterModifierKind::AshenEchoes,
                affix: crate::ordeal::OrdealAffix::None,
            },
        );

        assert_eq!(reward.gold, 15);
        assert_eq!(stats.gold, 15);
    }

    #[test]
    fn chapter_modifier_multiplies_breakable_gold() {
        let breakable = Breakable {
            gold: 100,
            potions: 0,
            fury: 0.0,
            label: "Test Coffer",
        };
        let mut stats = RunStats::default();
        let mut potions = PotionBelt {
            current: 0,
            max: 5,
            heal_amount: 45.0,
            cooldown: Timer::from_seconds(0.0, TimerMode::Once),
            cooldown_secs: crate::player::potion_cooldown_secs_for_capacity(5),
        };
        let mut fury = Fury {
            current: 0.0,
            max: 100.0,
            basic_gain: 18.0,
            dash_cost: 25.0,
            nova_cost: 45.0,
            rupture_cost: 32.0,
        };
        let fortune = FortuneBuff {
            timer: Timer::from_seconds(0.0, TimerMode::Once),
            xp_multiplier: 1.25,
            gold_multiplier: 1.50,
        };

        let reward = apply_breakable_reward(
            &breakable,
            &mut stats,
            &mut potions,
            &mut fury,
            &fortune,
            crate::ordeal::ChapterModifier {
                kind: crate::ordeal::ChapterModifierKind::Emberstorm,
                affix: crate::ordeal::OrdealAffix::None,
            },
        );

        assert_eq!(reward.gold, 132);
        assert_eq!(stats.gold, 132);
    }

    #[test]
    fn sealed_gate_blocks_only_the_local_story_pass() {
        let closed = DungeonLayout {
            sanctum_gate_open: false,
        };
        let stopped = clamp_dungeon_translation(&closed, Vec3::new(8.0, 3.0, -28.0));
        assert_eq!(stopped.z, SEALED_NORTH_LIMIT_Z);
        assert_eq!(stopped.y, 3.0);

        let around_pass = Vec3::new(32.0, 0.0, -28.0);
        assert_eq!(clamp_dungeon_translation(&closed, around_pass), around_pass);

        let deep_wilderness = Vec3::new(0.0, 0.0, -300.0);
        assert_eq!(
            clamp_dungeon_translation(&closed, deep_wilderness),
            deep_wilderness
        );
    }

    #[test]
    fn open_gate_admits_the_full_outdoor_world() {
        let open = DungeonLayout {
            sanctum_gate_open: true,
        };
        let northern_basin = Vec3::new(28.0, 0.0, -58.0);
        assert_eq!(
            clamp_dungeon_translation(&open, northern_basin),
            northern_basin
        );

        let beyond_edge = Vec3::new(110.0, 7.0, 95.0);
        assert_eq!(clamp_dungeon_translation(&open, beyond_edge), beyond_edge);
    }

    #[test]
    fn authored_region_is_not_a_world_boundary() {
        assert_eq!(WORLD_MAX_X - WORLD_MIN_X, 104.0);
        assert_eq!(WORLD_MAX_Z - WORLD_MIN_Z, 120.0);
        assert_eq!(WORLD_SCALE, 3.2);

        let far_wilderness = Vec3::new(12_000.0, 9.0, -9_000.0);
        assert_eq!(clamp_world_translation(far_wilderness), far_wilderness);
    }

    #[test]
    fn chapter_landmarks_are_mirrored_and_inside_the_world() {
        let ashen = chapter_world_layout(ChapterId::AshenReliquary);
        let hollow = chapter_world_layout(ChapterId::HollowBastion);
        assert_eq!(ashen.cache.x, -hollow.cache.x);
        assert_eq!(ashen.altar.x, -hollow.altar.x);
        assert_eq!(
            ashen.camp,
            chapter_world_position(ChapterId::AshenReliquary, Vec3::new(-4.8, 0.0, 5.9))
        );

        for layout in [ashen, hollow] {
            for landmark in [
                layout.player_spawn,
                layout.camp,
                layout.cache,
                layout.altar,
                layout.weapon_shrine,
                layout.boss,
                layout.lore_pages[0],
                layout.lore_pages[1],
                layout.lore_pages[2],
            ] {
                assert_eq!(clamp_world_translation(landmark), landmark);
            }
        }
    }

    #[test]
    fn authored_scenery_blueprint_stays_varied_for_procgen_reference() {
        for chapter in [ChapterId::AshenReliquary, ChapterId::HollowBastion] {
            let specs = outdoor_decoration_specs(chapter);
            assert!((100..=220).contains(&specs.len()));
            assert!(
                specs
                    .iter()
                    .any(|spec| spec.kind == OutdoorDecorationKind::DeadTree)
            );
            assert!(
                specs
                    .iter()
                    .any(|spec| spec.kind == OutdoorDecorationKind::Crystal)
            );
            assert!(
                specs
                    .iter()
                    .any(|spec| spec.kind == OutdoorDecorationKind::ArchLintel)
            );
            assert!(specs.iter().all(|spec| spec.position.is_finite()));
            assert!(specs.iter().all(|spec| spec.scale.is_finite()));
            assert!(specs.iter().all(|spec| spec.yaw.is_finite()));
        }
    }
}
