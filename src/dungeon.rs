use crate::{
    GameState, RunStats,
    assets::GameAssets,
    chapter::{ChapterPhase, ChapterProgress, InteractableKind, InteractableUsed},
    feedback::CombatEvent,
    not_paused,
    ordeal::ChapterModifier,
    player::{
        Barrier, ElixirBuff, Equipment, Evade, FortuneBuff, Fury, Health, Player, PotionBelt,
        apply_player_damage_with_evade, fortune_gold_reward, mitigated_damage, total_armor,
    },
};
use bevy::prelude::*;

#[derive(Component)]
pub struct DungeonEntity;

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
    #[default]
    ReliquaryCrossing,
}

impl ChapterZone {
    pub fn label(self) -> &'static str {
        match self {
            Self::CacheOssuary => "Cache Ossuary",
            Self::OuterSanctum => "Outer Sanctum",
            Self::GildedVault => "Gilded Vault",
            Self::EmberAltar => "Ember Altar",
            Self::QuartermasterNave => "Quartermaster Nave",
            Self::ReliquaryCrossing => "Reliquary Crossing",
        }
    }

    pub fn tactical_hint(self) -> &'static str {
        match self {
            Self::CacheOssuary => "cache, rift, cursed shrine",
            Self::OuterSanctum => "shrines, lore, first seal route",
            Self::GildedVault => "vault, fortune shrine, breakables",
            Self::EmberAltar => "altar, obelisk, boss gate",
            Self::QuartermasterNave => "vendor, well, pylon, portal return",
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

/// One slab of the sealed gate between the outer hall and the inner sanctum.
#[derive(Component)]
pub struct SanctumGate;

/// Walkable-space state for the two-chamber reliquary.
#[derive(Resource, Default, Debug, Clone, Copy)]
pub struct DungeonLayout {
    pub sanctum_gate_open: bool,
}

/// Clamp a translation to the walkable dungeon: the outer hall, the inner
/// sanctum, and — once the seal parts — the gate corridor joining them.
pub fn clamp_dungeon_translation(layout: &DungeonLayout, mut translation: Vec3) -> Vec3 {
    const OUTER_X: f32 = 11.5;
    const OUTER_Z_MIN: f32 = -7.5;
    const OUTER_Z_MAX: f32 = 7.5;
    const SANCTUM_X: f32 = 9.5;
    const SANCTUM_Z_MIN: f32 = -23.5;
    const SANCTUM_Z_MAX: f32 = -10.5;
    const CORRIDOR_X: f32 = 1.3;

    if translation.z >= OUTER_Z_MIN {
        translation.x = translation.x.clamp(-OUTER_X, OUTER_X);
        translation.z = translation.z.clamp(OUTER_Z_MIN, OUTER_Z_MAX);
    } else if translation.z <= SANCTUM_Z_MAX {
        if layout.sanctum_gate_open {
            translation.x = translation.x.clamp(-SANCTUM_X, SANCTUM_X);
            translation.z = translation.z.clamp(SANCTUM_Z_MIN, SANCTUM_Z_MAX);
        } else {
            translation.x = translation.x.clamp(-OUTER_X, OUTER_X);
            translation.z = OUTER_Z_MIN;
        }
    } else if layout.sanctum_gate_open {
        // Inside the corridor band: stay within the doorway.
        translation.x = translation.x.clamp(-CORRIDOR_X, CORRIDOR_X);
    } else {
        translation.x = translation.x.clamp(-OUTER_X, OUTER_X);
        translation.z = OUTER_Z_MIN;
    }
    translation
}

/// Removes the gate slabs when the chapter cracks the seal.
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
            text: "The seal parts: the inner sanctum lies open to the north".to_string(),
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
    mut zone: ResMut<ChapterZoneState>,
    mut combat_events: MessageWriter<CombatEvent>,
) {
    let Ok(player_transform) = player.single() else {
        return;
    };
    let current = chapter_zone_for_position(player_transform.translation);
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
    match (position.x, position.z) {
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
) {
    let floor_mesh = meshes.add(Cuboid::new(26.0, 0.25, 18.0));
    let floor_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(0.12, 0.105, 0.12),
        perceptual_roughness: 0.92,
        ..default()
    });
    commands.spawn((
        Mesh3d(floor_mesh),
        MeshMaterial3d(floor_mat.clone()),
        Transform::from_xyz(0.0, -0.13, 0.0),
        DungeonEntity,
        Name::new("Reliquary Outer Floor"),
    ));
    // Inner sanctum chamber to the north, joined through the sealed gate.
    let sanctum_floor_mesh = meshes.add(Cuboid::new(22.0, 0.25, 16.0));
    let sanctum_floor_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(0.135, 0.10, 0.105),
        perceptual_roughness: 0.90,
        ..default()
    });
    commands.spawn((
        Mesh3d(sanctum_floor_mesh),
        MeshMaterial3d(sanctum_floor_mat),
        Transform::from_xyz(0.0, -0.13, -17.0),
        DungeonEntity,
        Name::new("Inner Sanctum Floor"),
    ));

    let wall_mesh = meshes.add(Cuboid::new(1.0, 2.4, 1.0));
    let wall_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(0.20, 0.19, 0.22),
        perceptual_roughness: 0.85,
        ..default()
    });

    // Dividing wall between the halls, with a gate opening at |x| <= 1.
    for x in -13..=13 {
        if !(-1..=1).contains(&x) {
            spawn_wall(
                &mut commands,
                &wall_mesh,
                &wall_mat,
                Vec3::new(x as f32, 1.0, -9.0),
            );
        }
        spawn_wall(
            &mut commands,
            &wall_mesh,
            &wall_mat,
            Vec3::new(x as f32, 1.0, 9.0),
        );
    }
    for z in -8..=8 {
        spawn_wall(
            &mut commands,
            &wall_mesh,
            &wall_mat,
            Vec3::new(-13.0, 1.0, z as f32),
        );
        spawn_wall(
            &mut commands,
            &wall_mesh,
            &wall_mat,
            Vec3::new(13.0, 1.0, z as f32),
        );
    }
    // Sanctum shell: side walls and the far northern wall.
    for z in -24..=-10 {
        spawn_wall(
            &mut commands,
            &wall_mesh,
            &wall_mat,
            Vec3::new(-11.0, 1.0, z as f32),
        );
        spawn_wall(
            &mut commands,
            &wall_mesh,
            &wall_mat,
            Vec3::new(11.0, 1.0, z as f32),
        );
    }
    for x in -11..=11 {
        spawn_wall(
            &mut commands,
            &wall_mesh,
            &wall_mat,
            Vec3::new(x as f32, 1.0, -25.0),
        );
    }

    // The sealed gate itself: ember-lit slabs blocking the doorway until the
    // outer objectives crack the seal.
    let gate_mesh = meshes.add(Cuboid::new(1.0, 2.2, 0.9));
    let gate_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(0.32, 0.10, 0.08),
        emissive: Color::srgb(0.9, 0.22, 0.08).into(),
        perceptual_roughness: 0.55,
        ..default()
    });
    for x in -1..=1 {
        commands.spawn((
            Mesh3d(gate_mesh.clone()),
            MeshMaterial3d(gate_mat.clone()),
            Transform::from_xyz(x as f32, 1.0, -9.0),
            DungeonEntity,
            SanctumGate,
            Name::new("Sanctum Seal Gate"),
        ));
    }

    for (scene, position, scale, interaction) in [
        (
            assets.chest.clone(),
            Vec3::new(-8.0, 0.0, -5.5),
            1.0,
            Some(InteractableKind::Cache),
        ),
        (
            assets.altar.clone(),
            Vec3::new(6.0, 0.0, -20.5),
            1.2,
            Some(InteractableKind::Altar),
        ),
        (
            assets.sword.clone(),
            Vec3::new(0.0, 0.0, -6.8),
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
        Transform::from_xyz(-4.8, 0.0, 5.9),
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
        Transform::from_xyz(8.6, 0.0, -5.8),
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
        Transform::from_xyz(-2.8, 0.0, -7.1),
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
        Transform::from_xyz(2.0, 0.0, 6.7),
        Interactable {
            kind: InteractableKind::HealingWell,
            radius: 1.85,
            used: false,
            reusable: false,
        },
        "Renewal Well",
    );

    for (name, position) in [
        ("Steward's Warning", Vec3::new(-10.2, 0.0, 5.8)),
        ("Acolyte's Oath", Vec3::new(3.1, 0.0, -6.3)),
        ("Malrec's Pact", Vec3::new(9.2, 0.0, 5.7)),
    ] {
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
        Transform::from_xyz(-9.0, 0.0, 1.6),
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
        Transform::from_xyz(10.2, 0.0, 2.9),
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
        Transform::from_xyz(11.0, 0.0, -5.6).with_scale(Vec3::splat(1.08)),
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
        Transform::from_xyz(-11.0, 0.0, -4.6).with_scale(Vec3::splat(1.18)),
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
        Transform::from_xyz(4.4, 0.0, 7.0),
        Interactable {
            kind: InteractableKind::AshenPylon,
            radius: 1.95,
            used: false,
            reusable: false,
        },
        "Ashen Pylon",
    );

    for spec in breakable_spawns() {
        spawn_breakable(&mut commands, &assets, spec);
    }

    spawn_hazard(
        &mut commands,
        &assets,
        Vec3::new(-5.6, 0.0, -1.0),
        1.15,
        11.0 * modifier.hazard_damage_multiplier(),
        "Ember vent",
    );
    spawn_hazard(
        &mut commands,
        &assets,
        Vec3::new(1.7, 0.0, -3.2),
        1.05,
        10.0 * modifier.hazard_damage_multiplier(),
        "Ember vent",
    );
    spawn_hazard(
        &mut commands,
        &assets,
        Vec3::new(6.7, 0.0, 1.2),
        1.25,
        13.0 * modifier.hazard_damage_multiplier(),
        "Keeper's flame vent",
    );
}

fn breakable_spawns() -> [BreakableSpawn; BREAKER_TARGET_BREAKABLES as usize] {
    [
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
    ]
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
) {
    commands.spawn((
        WorldAssetRoot(assets.ember_vent.clone()),
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
) {
    commands.spawn((
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
    ));
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

fn spawn_wall(
    commands: &mut Commands,
    mesh: &Handle<Mesh>,
    material: &Handle<StandardMaterial>,
    position: Vec3,
) {
    commands.spawn((
        Mesh3d(mesh.clone()),
        MeshMaterial3d(material.clone()),
        Transform::from_translation(position),
        DungeonEntity,
        Name::new("Reliquary Wall"),
    ));
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
        if !interactable_available(&interactable) {
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
        && interactable_available(&interactable)
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

fn interactable_available(interactable: &Interactable) -> bool {
    !interactable.used || interactable.reusable
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

    let ring_mesh = meshes.add(Torus::new(0.72, 0.86));
    let ring_material = materials.add(StandardMaterial {
        base_color: Color::srgba(0.16, 0.74, 1.0, 0.42),
        emissive: Color::srgb(0.04, 0.28, 0.52).into(),
        alpha_mode: AlphaMode::Blend,
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
        assert_eq!(
            chapter_zone_for_position(Vec3::new(-8.0, 0.0, -5.5)),
            ChapterZone::CacheOssuary
        );
        assert_eq!(
            chapter_zone_for_position(Vec3::new(0.0, 0.0, -6.8)),
            ChapterZone::OuterSanctum
        );
        assert_eq!(
            chapter_zone_for_position(Vec3::new(10.0, 0.0, -5.0)),
            ChapterZone::GildedVault
        );
        assert_eq!(
            chapter_zone_for_position(Vec3::new(7.0, 0.0, 4.8)),
            ChapterZone::EmberAltar
        );
        assert_eq!(
            chapter_zone_for_position(Vec3::new(0.0, 0.0, 6.7)),
            ChapterZone::QuartermasterNave
        );
        assert_eq!(
            chapter_zone_for_position(Vec3::ZERO),
            ChapterZone::ReliquaryCrossing
        );
    }

    #[test]
    fn chapter_zone_labels_surface_tactical_route_roles() {
        let zone = ChapterZone::QuartermasterNave;

        assert_eq!(zone.label(), "Quartermaster Nave");
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
        assert_eq!(breakable_spawns().len(), BREAKER_TARGET_BREAKABLES as usize);
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
        assert!(interactable_available(&well));
        assert!(!interactable_available(&Interactable {
            used: true,
            ..shrine
        }));
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
    fn sealed_gate_confines_the_player_to_the_outer_hall() {
        let closed = DungeonLayout {
            sanctum_gate_open: false,
        };
        // Pushing north against the seal stops at the outer wall.
        let stopped = clamp_dungeon_translation(&closed, Vec3::new(0.0, 0.0, -12.0));
        assert_eq!(stopped.z, -7.5);
        // The corridor band is also blocked while sealed.
        let corridor = clamp_dungeon_translation(&closed, Vec3::new(0.5, 0.0, -8.4));
        assert_eq!(corridor.z, -7.5);
        // Ordinary outer-hall movement is untouched.
        let free = clamp_dungeon_translation(&closed, Vec3::new(3.0, 0.0, 2.0));
        assert_eq!(free, Vec3::new(3.0, 0.0, 2.0));
    }

    #[test]
    fn open_gate_admits_the_sanctum_through_the_doorway() {
        let open = DungeonLayout {
            sanctum_gate_open: true,
        };
        // The corridor funnels through the doorway gap.
        let corridor = clamp_dungeon_translation(&open, Vec3::new(4.0, 0.0, -8.4));
        assert!(corridor.x.abs() <= 1.3);
        assert_eq!(corridor.z, -8.4);
        // Sanctum interior is walkable and bounded.
        let sanctum = clamp_dungeon_translation(&open, Vec3::new(-30.0, 0.0, -40.0));
        assert_eq!(sanctum, Vec3::new(-9.5, 0.0, -23.5));
        let mid = clamp_dungeon_translation(&open, Vec3::new(5.0, 0.0, -18.0));
        assert_eq!(mid, Vec3::new(5.0, 0.0, -18.0));
    }
}
