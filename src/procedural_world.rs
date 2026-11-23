use crate::{
    GameState,
    assets::GameAssets,
    chapter::{ActiveChapter, ChapterId},
    dungeon::{DungeonEntity, OutdoorScenery, chapter_world_layout, chapter_world_position},
    enemy::SpawnWildernessEncounter,
    not_paused,
    player::Player,
    terrain::{self, value_noise},
};
use bevy::{
    light::{NotShadowCaster, NotShadowReceiver},
    prelude::*,
};
use std::collections::{BTreeMap, BTreeSet};

/// Width and depth of one streamed world tile, in world units.
pub const CHUNK_SIZE: f32 = 48.0;
pub const CHUNK_LOAD_RADIUS: i64 = 3;
pub const CHUNK_UNLOAD_RADIUS: i64 = 4;
pub const MAX_CHUNKS_PER_FRAME: usize = 4;

const DEFAULT_WORLD_SEED: u64 = 0xA55E_4F52_5047_2026;
/// Height of the ground surface; roads and clearings layer just above it.
const GROUND_Y: f32 = -0.03;
const DECORATION_MARGIN: f32 = 2.25;
const STORY_ROAD_CLEARANCE: f32 = 5.25;

/// Stable seed used to reproduce the same wilderness between play sessions.
///
/// Insert this resource before entering [`GameState::InGame`] to select a
/// different world.
#[derive(Resource, Debug, Clone, Copy, Eq, PartialEq)]
pub struct ProceduralWorldSeed(pub u64);

impl Default for ProceduralWorldSeed {
    fn default() -> Self {
        Self(DEFAULT_WORLD_SEED)
    }
}

/// Root marker and persistent integer address for one generated chunk.
#[derive(Component, Debug, Clone, Copy, Eq, PartialEq, Hash)]
pub struct ProceduralChunk {
    pub x: i64,
    pub z: i64,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq, Ord, PartialOrd, Hash)]
struct ChunkCoord {
    x: i64,
    z: i64,
}

impl ChunkCoord {
    fn from_world(position: Vec3) -> Self {
        Self {
            x: world_axis_to_chunk(position.x),
            z: world_axis_to_chunk(position.z),
        }
    }

    fn center(self) -> Vec3 {
        Vec3::new(
            (self.x as f32 + 0.5) * CHUNK_SIZE,
            0.0,
            (self.z as f32 + 0.5) * CHUNK_SIZE,
        )
    }

    fn chebyshev_distance(self, other: Self) -> i64 {
        self.x
            .abs_diff(other.x)
            .max(self.z.abs_diff(other.z))
            .min(i64::MAX as u64) as i64
    }
}

#[derive(Resource)]
struct LoadedProceduralChunks {
    chapter: ChapterId,
    entities: BTreeMap<ChunkCoord, Entity>,
}

#[derive(Resource)]
struct ProceduralWorldAssets {
    ground_material: Handle<StandardMaterial>,
    ground_palette: [LinearRgba; 4],
    rock: Handle<WorldAsset>,
    dead_tree: Handle<WorldAsset>,
    crystal: Handle<WorldAsset>,
    ruin: Handle<WorldAsset>,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
enum DecorationKind {
    Rock,
    DeadTree,
    Crystal,
    Ruin,
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct DecorationSpec {
    kind: DecorationKind,
    local_position: Vec3,
    scale: Vec3,
    yaw: f32,
}

#[derive(Debug, Clone, PartialEq)]
struct ChunkPlan {
    ground_material: usize,
    decorations: Vec<DecorationSpec>,
}

const AMBIENT_MOTE_COUNT: usize = 28;
const AMBIENT_MOTE_RADIUS: f32 = 15.5;
const AMBIENT_MOTE_MIN_Y: f32 = 0.45;
const AMBIENT_MOTE_MAX_Y: f32 = 5.4;

/// Drifting dark-fantasy ash / ember atmosphere mote orbiting around the player.
#[derive(Component, Debug, Clone, Copy)]
struct AmbientMote {
    offset: Vec3,
    drift_speed: f32,
    sway_phase: f32,
    sway_speed: f32,
    base_scale: f32,
}

#[derive(Resource, Debug, Clone, Copy, Eq, PartialEq)]
struct AmbientAtmosphereChapter(ChapterId);

/// Streams a deterministic wilderness around the player.
pub struct ProceduralWorldPlugin;

impl Plugin for ProceduralWorldPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<ProceduralWorldSeed>()
            .add_systems(
                OnEnter(GameState::InGame),
                (setup_procedural_world, spawn_ambient_motes),
            )
            .add_systems(
                Update,
                (stream_procedural_chunks, update_ambient_motes)
                    .run_if(in_state(GameState::InGame).and_then(not_paused)),
            )
            .add_systems(OnExit(GameState::InGame), teardown_procedural_world);
    }
}

fn setup_procedural_world(
    mut commands: Commands,
    mut materials: ResMut<Assets<StandardMaterial>>,
    assets: Res<GameAssets>,
    active_chapter: Res<ActiveChapter>,
) {
    let palette = chapter_palette(active_chapter.0);
    let ground_texture = match active_chapter.0 {
        ChapterId::AshenReliquary => assets.ashen_ground_albedo.clone(),
        ChapterId::HollowBastion => assets.hollow_ground_albedo.clone(),
    };
    // One white material: biome tint and mottling live in each chunk's vertex
    // colors, so neighbouring chunks blend instead of meeting at a hard seam.
    let ground_material = materials.add(StandardMaterial {
        base_color: Color::WHITE,
        base_color_texture: Some(ground_texture),
        alpha_mode: AlphaMode::Opaque,
        perceptual_roughness: 0.97,
        reflectance: 0.18,
        ..default()
    });

    commands.insert_resource(ProceduralWorldAssets {
        ground_material,
        ground_palette: palette.ground.map(|color| color.to_linear()),
        rock: assets.scenery_rock.clone(),
        dead_tree: assets.scenery_dead_tree.clone(),
        crystal: assets.scenery_crystal.clone(),
        ruin: assets.scenery_ruin.clone(),
    });
    commands.insert_resource(LoadedProceduralChunks {
        chapter: active_chapter.0,
        entities: BTreeMap::new(),
    });
}

#[allow(clippy::too_many_arguments)]
fn stream_procedural_chunks(
    mut commands: Commands,
    player: Query<&Transform, With<Player>>,
    active_chapter: Res<ActiveChapter>,
    world_seed: Res<ProceduralWorldSeed>,
    assets: Res<ProceduralWorldAssets>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut loaded: ResMut<LoadedProceduralChunks>,
    mut encounter_events: MessageWriter<SpawnWildernessEncounter>,
) {
    let Ok(player_transform) = player.single() else {
        return;
    };
    let player_chunk = ChunkCoord::from_world(player_transform.translation);

    if loaded.chapter != active_chapter.0 {
        for entity in loaded.entities.values().copied() {
            commands.entity(entity).try_despawn();
        }
        loaded.entities.clear();
        loaded.chapter = active_chapter.0;
    }

    let stale_chunks = chunks_to_unload(player_chunk, loaded.entities.keys().copied());
    for coord in stale_chunks {
        if let Some(entity) = loaded.entities.remove(&coord) {
            commands.entity(entity).try_despawn();
        }
    }

    let loaded_coords = loaded.entities.keys().copied().collect();
    for coord in select_chunks_to_load(player_chunk, &loaded_coords) {
        let entity = spawn_chunk(
            &mut commands,
            &mut meshes,
            &assets,
            active_chapter.0,
            world_seed.0,
            coord,
        );
        loaded.entities.insert(coord, entity);
        if should_spawn_wilderness_encounter(world_seed.0, active_chapter.0, coord) {
            encounter_events.write(SpawnWildernessEncounter {
                chapter: active_chapter.0,
                chunk_x: coord.x,
                chunk_z: coord.z,
                center: coord.center(),
                seed: chunk_seed(world_seed.0, active_chapter.0, coord),
            });
        }
    }
}

fn spawn_chunk(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    assets: &ProceduralWorldAssets,
    chapter: ChapterId,
    world_seed: u64,
    coord: ChunkCoord,
) -> Entity {
    let plan = generate_chunk_plan(world_seed, chapter, coord);
    let center = coord.center();
    let ground = meshes.add(terrain::ground_chunk_mesh(
        Vec2::new(center.x, center.z),
        CHUNK_SIZE,
        |world| {
            terrain::ground_color(
                world_seed,
                &assets.ground_palette,
                biome_value(
                    world_seed,
                    chapter,
                    world.x / CHUNK_SIZE - 0.5,
                    world.y / CHUNK_SIZE - 0.5,
                ),
                world,
            )
        },
    ));
    let root = commands
        .spawn((
            ProceduralChunk {
                x: coord.x,
                z: coord.z,
            },
            DungeonEntity,
            Transform::from_translation(center),
            Visibility::default(),
            Name::new(format!("Procedural Chunk {}, {}", coord.x, coord.z)),
        ))
        .id();

    commands.entity(root).with_children(|parent| {
        parent.spawn((
            Mesh3d(ground),
            MeshMaterial3d(assets.ground_material.clone()),
            Transform::from_xyz(0.0, GROUND_Y, 0.0),
            OutdoorScenery,
            Name::new("Opaque Procedural Ground"),
        ));

        for spec in plan.decorations {
            let (scene, name) = match spec.kind {
                DecorationKind::Rock => (&assets.rock, "Weathered Ridge Rock"),
                DecorationKind::DeadTree => (&assets.dead_tree, "Twisted Dead Tree"),
                DecorationKind::Crystal => (&assets.crystal, "Relic Crystal Cluster"),
                DecorationKind::Ruin => (&assets.ruin, "Broken Gothic Arch"),
            };
            parent.spawn((
                WorldAssetRoot(scene.clone()),
                Transform::from_translation(spec.local_position)
                    .with_rotation(Quat::from_rotation_y(spec.yaw))
                    .with_scale(spec.scale),
                OutdoorScenery,
                Name::new(name),
            ));
        }
    });
    root
}

fn ambient_mote_colors(chapter: ChapterId) -> (Color, Color) {
    match chapter {
        ChapterId::AshenReliquary => (
            Color::srgba(1.0, 0.48, 0.14, 0.72),
            Color::srgba(0.66, 0.56, 0.48, 0.36),
        ),
        ChapterId::HollowBastion => (
            Color::srgba(0.32, 0.90, 0.92, 0.65),
            Color::srgba(0.54, 0.64, 0.74, 0.36),
        ),
    }
}

fn spawn_ambient_motes(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    active_chapter: Res<ActiveChapter>,
) {
    let (primary_color, ash_color) = ambient_mote_colors(active_chapter.0);
    let mote_mesh = meshes.add(Sphere::new(0.055).mesh().ico(1).unwrap());
    let ember_mat = materials.add(StandardMaterial {
        base_color: primary_color,
        emissive: LinearRgba::from(primary_color) * 2.2,
        unlit: true,
        alpha_mode: AlphaMode::Add,
        ..default()
    });
    let ash_mat = materials.add(StandardMaterial {
        base_color: ash_color,
        emissive: LinearRgba::from(ash_color) * 0.9,
        unlit: true,
        alpha_mode: AlphaMode::Add,
        ..default()
    });

    let mut rng = SplitMix64::new(0xE1BE_4A5B_900D_1337);
    for i in 0..AMBIENT_MOTE_COUNT {
        let angle = rng.unit_f32() * std::f32::consts::TAU;
        let dist = rng.unit_f32().sqrt() * AMBIENT_MOTE_RADIUS;
        let y = AMBIENT_MOTE_MIN_Y + rng.unit_f32() * (AMBIENT_MOTE_MAX_Y - AMBIENT_MOTE_MIN_Y);
        let offset = Vec3::new(angle.cos() * dist, y, angle.sin() * dist);
        let is_ember = i % 3 != 0;
        let base_scale = if is_ember {
            0.75 + rng.unit_f32() * 0.65
        } else {
            0.55 + rng.unit_f32() * 0.50
        };
        commands.spawn((
            Mesh3d(mote_mesh.clone()),
            MeshMaterial3d(if is_ember {
                ember_mat.clone()
            } else {
                ash_mat.clone()
            }),
            Transform::from_translation(offset).with_scale(Vec3::splat(base_scale)),
            NotShadowCaster,
            NotShadowReceiver,
            OutdoorScenery,
            AmbientMote {
                offset,
                drift_speed: 0.34 + rng.unit_f32() * 0.55,
                sway_phase: rng.unit_f32() * std::f32::consts::TAU,
                sway_speed: 0.7 + rng.unit_f32() * 1.1,
                base_scale,
            },
            Name::new("Ambient Ash Mote"),
        ));
    }
    commands.insert_resource(AmbientAtmosphereChapter(active_chapter.0));
}

#[allow(clippy::too_many_arguments)]
fn update_ambient_motes(
    mut commands: Commands,
    time: Res<Time>,
    active_chapter: Res<ActiveChapter>,
    atmosphere_chapter: Option<Res<AmbientAtmosphereChapter>>,
    player: Query<&Transform, (With<Player>, Without<AmbientMote>)>,
    mut motes: Query<(Entity, &mut Transform, &mut AmbientMote), Without<Player>>,
    meshes: ResMut<Assets<Mesh>>,
    materials: ResMut<Assets<StandardMaterial>>,
) {
    if atmosphere_chapter.is_some_and(|ch| ch.0 != active_chapter.0) {
        for (entity, ..) in &motes {
            commands.entity(entity).try_despawn();
        }
        spawn_ambient_motes(commands, meshes, materials, active_chapter);
        return;
    }
    let Ok(player_transform) = player.single() else {
        return;
    };
    let dt = time.delta_secs();
    let elapsed = time.elapsed_secs();
    let span = AMBIENT_MOTE_MAX_Y - AMBIENT_MOTE_MIN_Y;
    let wind = Vec3::new(0.42, 0.0, -0.26);

    for (_, mut transform, mut mote) in &mut motes {
        mote.offset.y += mote.drift_speed * dt;
        mote.offset.x += wind.x * dt;
        mote.offset.z += wind.z * dt;
        if mote.offset.y > AMBIENT_MOTE_MAX_Y {
            mote.offset.y -= span;
        }
        // Wrap gently around player horizon so motes stay uniformly distributed
        if mote.offset.x > AMBIENT_MOTE_RADIUS {
            mote.offset.x -= AMBIENT_MOTE_RADIUS * 2.0;
        } else if mote.offset.x < -AMBIENT_MOTE_RADIUS {
            mote.offset.x += AMBIENT_MOTE_RADIUS * 2.0;
        }
        if mote.offset.z > AMBIENT_MOTE_RADIUS {
            mote.offset.z -= AMBIENT_MOTE_RADIUS * 2.0;
        } else if mote.offset.z < -AMBIENT_MOTE_RADIUS {
            mote.offset.z += AMBIENT_MOTE_RADIUS * 2.0;
        }

        let sway = (elapsed * mote.sway_speed + mote.sway_phase).sin() * 0.55;
        let pulse = 0.72
            + 0.38
                * (elapsed * (mote.sway_speed * 1.4) + mote.sway_phase)
                    .cos()
                    .abs();
        let height_fade = ((mote.offset.y - AMBIENT_MOTE_MIN_Y) / 0.9).min(1.0)
            * ((AMBIENT_MOTE_MAX_Y - mote.offset.y) / 1.1).min(1.0);
        let scale = (mote.base_scale * pulse * height_fade.max(0.08)).max(0.01);

        transform.translation = Vec3::new(
            player_transform.translation.x + mote.offset.x + sway,
            mote.offset.y,
            player_transform.translation.z + mote.offset.z + sway * 0.6,
        );
        transform.scale = Vec3::new(scale, scale * 1.35, scale);
    }
}

fn teardown_procedural_world(
    mut commands: Commands,
    chunks: Query<Entity, With<ProceduralChunk>>,
    motes: Query<Entity, With<AmbientMote>>,
) {
    for entity in &chunks {
        commands.entity(entity).try_despawn();
    }
    for entity in &motes {
        commands.entity(entity).try_despawn();
    }
    commands.remove_resource::<LoadedProceduralChunks>();
    commands.remove_resource::<ProceduralWorldAssets>();
    commands.remove_resource::<AmbientAtmosphereChapter>();
}

fn world_axis_to_chunk(value: f32) -> i64 {
    (value / CHUNK_SIZE).floor() as i64
}

fn desired_chunk_coords(center: ChunkCoord) -> Vec<ChunkCoord> {
    let diameter = CHUNK_LOAD_RADIUS * 2 + 1;
    let mut coords = Vec::with_capacity((diameter * diameter) as usize);
    for z in (center.z - CHUNK_LOAD_RADIUS)..=(center.z + CHUNK_LOAD_RADIUS) {
        for x in (center.x - CHUNK_LOAD_RADIUS)..=(center.x + CHUNK_LOAD_RADIUS) {
            coords.push(ChunkCoord { x, z });
        }
    }
    coords.sort_unstable_by_key(|coord| {
        let x = coord.x - center.x;
        let z = coord.z - center.z;
        (x * x + z * z, z, x)
    });
    coords
}

fn select_chunks_to_load(center: ChunkCoord, loaded: &BTreeSet<ChunkCoord>) -> Vec<ChunkCoord> {
    desired_chunk_coords(center)
        .into_iter()
        .filter(|coord| !loaded.contains(coord))
        .take(MAX_CHUNKS_PER_FRAME)
        .collect()
}

fn chunks_to_unload(
    center: ChunkCoord,
    loaded: impl IntoIterator<Item = ChunkCoord>,
) -> Vec<ChunkCoord> {
    loaded
        .into_iter()
        .filter(|coord| coord.chebyshev_distance(center) > CHUNK_UNLOAD_RADIUS)
        .collect()
}

fn should_spawn_wilderness_encounter(
    world_seed: u64,
    chapter: ChapterId,
    coord: ChunkCoord,
) -> bool {
    let center = coord.center();
    let layout = chapter_world_layout(chapter);
    if center.distance(layout.player_spawn) < CHUNK_SIZE * 1.35
        || is_story_clearance(chapter, xz(center))
    {
        return false;
    }
    chunk_seed(world_seed, chapter, coord) % 100 < 46
}

fn generate_chunk_plan(world_seed: u64, chapter: ChapterId, coord: ChunkCoord) -> ChunkPlan {
    let mut rng = SplitMix64::new(chunk_seed(world_seed, chapter, coord));
    let ground_material = biome_index(world_seed, chapter, coord);
    let target_count = 6 + rng.index(5);
    let mut decorations = Vec::with_capacity(target_count);
    let center = coord.center();

    for _ in 0..(target_count * 12) {
        if decorations.len() == target_count {
            break;
        }
        let local_x = rng.range_f32(
            -CHUNK_SIZE * 0.5 + DECORATION_MARGIN,
            CHUNK_SIZE * 0.5 - DECORATION_MARGIN,
        );
        let local_z = rng.range_f32(
            -CHUNK_SIZE * 0.5 + DECORATION_MARGIN,
            CHUNK_SIZE * 0.5 - DECORATION_MARGIN,
        );
        let world_position = Vec2::new(center.x + local_x, center.z + local_z);
        if is_story_clearance(chapter, world_position) {
            continue;
        }
        let local_position = Vec3::new(local_x, 0.0, local_z);
        if decorations.iter().any(|existing: &DecorationSpec| {
            let delta = existing.local_position - local_position;
            Vec2::new(delta.x, delta.z).length_squared() < 4.5_f32.powi(2)
        }) {
            continue;
        }

        let kind = decoration_kind_for_biome(ground_material, rng.index(100));
        let scale = match kind {
            DecorationKind::Rock => Vec3::new(
                rng.range_f32(0.72, 1.65),
                rng.range_f32(0.42, 0.88),
                rng.range_f32(0.68, 1.42),
            ),
            DecorationKind::DeadTree => Vec3::new(
                rng.range_f32(0.72, 1.08),
                rng.range_f32(0.72, 1.46),
                rng.range_f32(0.72, 1.08),
            ),
            DecorationKind::Crystal => Vec3::new(
                rng.range_f32(0.54, 0.96),
                rng.range_f32(0.78, 1.58),
                rng.range_f32(0.54, 0.96),
            ),
            DecorationKind::Ruin => Vec3::new(
                rng.range_f32(0.82, 2.25),
                rng.range_f32(0.46, 1.38),
                rng.range_f32(0.56, 1.62),
            ),
        };
        decorations.push(DecorationSpec {
            kind,
            local_position,
            scale,
            yaw: rng.range_f32(0.0, std::f32::consts::TAU),
        });
    }

    ChunkPlan {
        ground_material,
        decorations,
    }
}

fn decoration_kind_for_biome(biome: usize, roll: usize) -> DecorationKind {
    match biome {
        // Ash scrub: silhouettes and stone dominate.
        0 => match roll {
            0..=37 => DecorationKind::DeadTree,
            38..=72 => DecorationKind::Rock,
            73..=92 => DecorationKind::Ruin,
            _ => DecorationKind::Crystal,
        },
        // Weathered badlands: open rock fields with occasional relics.
        1 => match roll {
            0..=44 => DecorationKind::Rock,
            45..=69 => DecorationKind::Ruin,
            70..=88 => DecorationKind::DeadTree,
            _ => DecorationKind::Crystal,
        },
        // Crystal reaches: emissive spires create a distinct navigational read.
        2 => match roll {
            0..=43 => DecorationKind::Crystal,
            44..=70 => DecorationKind::Rock,
            71..=87 => DecorationKind::Ruin,
            _ => DecorationKind::DeadTree,
        },
        // Ruin belts: denser remnants imply vanished settlements.
        _ => match roll {
            0..=43 => DecorationKind::Ruin,
            44..=69 => DecorationKind::DeadTree,
            70..=90 => DecorationKind::Rock,
            _ => DecorationKind::Crystal,
        },
    }
}

fn biome_index(world_seed: u64, chapter: ChapterId, coord: ChunkCoord) -> usize {
    biome_value(world_seed, chapter, coord.x as f32, coord.z as f32)
        .floor()
        .clamp(0.0, 3.0) as usize
}

/// Continuous biome field in chunk units (`0..4`); integer chunk coordinates
/// give the value that picks that chunk's decorations.
fn biome_value(world_seed: u64, chapter: ChapterId, x: f32, z: f32) -> f32 {
    let chapter_salt: u64 = match chapter {
        ChapterId::AshenReliquary => 0x4153_4845_4E5F_4249,
        ChapterId::HollowBastion => 0x484F_4C4C_4F57_4249,
    };
    let broad = value_noise(world_seed ^ chapter_salt, x / 4.8, z / 4.8);
    let detail = value_noise(
        world_seed ^ chapter_salt.rotate_left(23),
        (x + 19.0) / 2.35,
        (z - 31.0) / 2.35,
    );
    (broad * 0.72 + detail * 0.28) * 4.0
}

fn is_story_clearance(chapter: ChapterId, point: Vec2) -> bool {
    let layout = chapter_world_layout(chapter);
    let fortune = chapter_world_position(chapter, Vec3::new(8.6, 0.0, -5.8));
    let gate = chapter_world_position(chapter, Vec3::new(0.0, 0.0, -7.5));
    let landmarks = [
        (layout.player_spawn, 9.0),
        (layout.camp, 10.0),
        (layout.cache, 10.0),
        (layout.altar, 11.0),
        (layout.weapon_shrine, 10.0),
        (layout.boss, 13.0),
        (layout.lore_pages[0], 5.5),
        (layout.lore_pages[1], 5.5),
        (layout.lore_pages[2], 5.5),
        (fortune, 6.5),
        (gate, 7.0),
    ];
    if landmarks
        .into_iter()
        .any(|(landmark, radius)| point.distance(xz(landmark)) < radius)
    {
        return true;
    }

    [
        (layout.player_spawn, layout.camp),
        (layout.camp, layout.weapon_shrine),
        (layout.weapon_shrine, layout.cache),
        (layout.weapon_shrine, fortune),
        (layout.weapon_shrine, gate),
        (gate, layout.boss),
        (layout.boss, layout.altar),
    ]
    .into_iter()
    .any(|(start, end)| point_segment_distance(point, xz(start), xz(end)) < STORY_ROAD_CLEARANCE)
}

fn xz(position: Vec3) -> Vec2 {
    Vec2::new(position.x, position.z)
}

fn point_segment_distance(point: Vec2, start: Vec2, end: Vec2) -> f32 {
    let segment = end - start;
    let length_squared = segment.length_squared();
    if length_squared <= f32::EPSILON {
        return point.distance(start);
    }
    let t = ((point - start).dot(segment) / length_squared).clamp(0.0, 1.0);
    point.distance(start + segment * t)
}

fn chunk_seed(world_seed: u64, chapter: ChapterId, coord: ChunkCoord) -> u64 {
    let chapter_key = match chapter {
        ChapterId::AshenReliquary => 0xA511_EE11_0000_0001,
        ChapterId::HollowBastion => 0xB457_10A0_0000_0002,
    };
    SplitMix64::mix(
        world_seed
            ^ chapter_key
            ^ (coord.x as u64).wrapping_mul(0xD6E8_FEB8_6659_FD93)
            ^ (coord.z as u64)
                .rotate_left(32)
                .wrapping_mul(0xA5A3_564E_27F8_864D),
    )
}

struct SplitMix64 {
    state: u64,
}

impl SplitMix64 {
    fn new(seed: u64) -> Self {
        Self { state: seed }
    }

    fn mix(value: u64) -> u64 {
        terrain::mix(value)
    }

    fn next_u64(&mut self) -> u64 {
        self.state = self.state.wrapping_add(0x9E37_79B9_7F4A_7C15);
        Self::mix(self.state)
    }

    fn unit_f32(&mut self) -> f32 {
        ((self.next_u64() >> 40) as f32) * (1.0 / 16_777_216.0)
    }

    fn range_f32(&mut self, min: f32, max: f32) -> f32 {
        min + (max - min) * self.unit_f32()
    }

    fn index(&mut self, upper_bound: usize) -> usize {
        (self.next_u64() % upper_bound as u64) as usize
    }
}

struct ChapterPalette {
    ground: [Color; 4],
}

fn chapter_palette(chapter: ChapterId) -> ChapterPalette {
    match chapter {
        ChapterId::AshenReliquary => ChapterPalette {
            ground: [
                Color::srgb(0.86, 0.78, 0.70),
                Color::srgb(0.96, 0.78, 0.63),
                Color::srgb(0.76, 0.76, 0.72),
                Color::srgb(1.00, 0.72, 0.55),
            ],
        },
        ChapterId::HollowBastion => ChapterPalette {
            ground: [
                Color::srgb(0.66, 0.82, 0.86),
                Color::srgb(0.58, 0.90, 0.92),
                Color::srgb(0.76, 0.82, 0.84),
                Color::srgb(0.52, 0.94, 0.98),
            ],
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn negative_world_positions_use_floor_chunk_mapping() {
        assert_eq!(world_axis_to_chunk(0.0), 0);
        assert_eq!(world_axis_to_chunk(47.999), 0);
        assert_eq!(world_axis_to_chunk(48.0), 1);
        assert_eq!(world_axis_to_chunk(-0.001), -1);
        assert_eq!(world_axis_to_chunk(-48.0), -1);
        assert_eq!(world_axis_to_chunk(-48.001), -2);
    }

    #[test]
    fn identical_seed_chapter_and_coordinate_are_reproducible() {
        let coord = ChunkCoord { x: 12, z: -9 };
        let first = generate_chunk_plan(91_337, ChapterId::AshenReliquary, coord);
        let second = generate_chunk_plan(91_337, ChapterId::AshenReliquary, coord);
        assert_eq!(first, second);
        assert!((6..=10).contains(&first.decorations.len()));
    }

    #[test]
    fn different_chunk_coordinates_produce_different_content() {
        let first = generate_chunk_plan(
            91_337,
            ChapterId::HollowBastion,
            ChunkCoord { x: 20, z: 20 },
        );
        let second = generate_chunk_plan(
            91_337,
            ChapterId::HollowBastion,
            ChunkCoord { x: 21, z: 20 },
        );
        assert_ne!(first, second);
    }

    #[test]
    fn desired_set_and_per_frame_order_are_bounded() {
        let center = ChunkCoord { x: -7, z: 13 };
        let desired = desired_chunk_coords(center);
        assert_eq!(desired.len(), ((CHUNK_LOAD_RADIUS * 2 + 1).pow(2)) as usize);
        assert_eq!(desired[0], center);
        assert!(
            desired
                .iter()
                .all(|coord| coord.chebyshev_distance(center) <= CHUNK_LOAD_RADIUS)
        );
        assert!(desired.windows(2).all(|pair| {
            let squared_distance = |coord: ChunkCoord| {
                let x = coord.x - center.x;
                let z = coord.z - center.z;
                x * x + z * z
            };
            squared_distance(pair[0]) <= squared_distance(pair[1])
        }));

        let selected = select_chunks_to_load(center, &BTreeSet::new());
        assert_eq!(selected.len(), MAX_CHUNKS_PER_FRAME);
        assert_eq!(selected, desired[..MAX_CHUNKS_PER_FRAME]);
    }

    #[test]
    fn unload_hysteresis_keeps_only_the_bounded_neighborhood() {
        let old_center = ChunkCoord { x: 0, z: 0 };
        let new_center = ChunkCoord { x: 7, z: -5 };
        let loaded = desired_chunk_coords(old_center)
            .into_iter()
            .chain(desired_chunk_coords(new_center))
            .collect::<BTreeSet<_>>();
        let stale = chunks_to_unload(new_center, loaded.iter().copied())
            .into_iter()
            .collect::<BTreeSet<_>>();
        let retained = loaded.difference(&stale).copied().collect::<Vec<_>>();

        assert!(
            retained
                .iter()
                .all(|coord| coord.chebyshev_distance(new_center) <= CHUNK_UNLOAD_RADIUS)
        );
        assert!(
            retained.len()
                <= ((CHUNK_UNLOAD_RADIUS * 2 + 1) * (CHUNK_UNLOAD_RADIUS * 2 + 1)) as usize
        );
    }

    #[test]
    fn wilderness_encounter_selection_is_stable_and_keeps_story_origin_safe() {
        let seed = 0xA55E_1234_5678_9ABC;
        let chapter = ChapterId::AshenReliquary;
        let origin = ChunkCoord::from_world(chapter_world_layout(chapter).player_spawn);
        assert!(!should_spawn_wilderness_encounter(seed, chapter, origin));

        let decisions = (-12..=12)
            .flat_map(|z| {
                (-12..=12).map(move |x| {
                    should_spawn_wilderness_encounter(seed, chapter, ChunkCoord { x, z })
                })
            })
            .collect::<Vec<_>>();
        let repeated = (-12..=12)
            .flat_map(|z| {
                (-12..=12).map(move |x| {
                    should_spawn_wilderness_encounter(seed, chapter, ChunkCoord { x, z })
                })
            })
            .collect::<Vec<_>>();

        assert_eq!(decisions, repeated);
        let encounter_count = decisions.iter().filter(|spawn| **spawn).count();
        assert!(encounter_count > 120);
        assert!(encounter_count < 360);
    }
}
