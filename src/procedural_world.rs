use crate::{
    GameState,
    chapter::{ActiveChapter, ChapterId},
    dungeon::{DungeonEntity, OutdoorScenery, chapter_world_layout, chapter_world_position},
    enemy::SpawnWildernessEncounter,
    not_paused,
    player::Player,
};
use bevy::prelude::*;
use std::collections::{BTreeMap, BTreeSet};

/// Width and depth of one streamed world tile, in world units.
pub const CHUNK_SIZE: f32 = 48.0;
pub const CHUNK_LOAD_RADIUS: i64 = 3;
pub const CHUNK_UNLOAD_RADIUS: i64 = 4;
pub const MAX_CHUNKS_PER_FRAME: usize = 4;

const DEFAULT_WORLD_SEED: u64 = 0xA55E_4F52_5047_2026;
const GROUND_Y: f32 = -0.10;
const GROUND_THICKNESS: f32 = 0.18;
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
    ground_mesh: Handle<Mesh>,
    ground_materials: [Handle<StandardMaterial>; 4],
    rock_mesh: Handle<Mesh>,
    rock_material: Handle<StandardMaterial>,
    dead_tree_mesh: Handle<Mesh>,
    dead_tree_material: Handle<StandardMaterial>,
    crystal_mesh: Handle<Mesh>,
    crystal_material: Handle<StandardMaterial>,
    ruin_mesh: Handle<Mesh>,
    ruin_material: Handle<StandardMaterial>,
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

/// Streams a deterministic wilderness around the player.
pub struct ProceduralWorldPlugin;

impl Plugin for ProceduralWorldPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<ProceduralWorldSeed>()
            .add_systems(OnEnter(GameState::InGame), setup_procedural_world)
            .add_systems(
                Update,
                stream_procedural_chunks.run_if(in_state(GameState::InGame).and_then(not_paused)),
            )
            .add_systems(OnExit(GameState::InGame), teardown_procedural_world);
    }
}

fn setup_procedural_world(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    active_chapter: Res<ActiveChapter>,
) {
    let palette = chapter_palette(active_chapter.0);
    let ground_mesh = meshes.add(Cuboid::new(
        CHUNK_SIZE + 0.08,
        GROUND_THICKNESS,
        CHUNK_SIZE + 0.08,
    ));
    let ground_materials = palette.ground.map(|base_color| {
        materials.add(StandardMaterial {
            base_color,
            alpha_mode: AlphaMode::Opaque,
            perceptual_roughness: 0.97,
            reflectance: 0.18,
            ..default()
        })
    });

    commands.insert_resource(ProceduralWorldAssets {
        ground_mesh,
        ground_materials,
        rock_mesh: meshes.add(Sphere::new(1.0)),
        rock_material: materials.add(StandardMaterial {
            base_color: palette.rock,
            alpha_mode: AlphaMode::Opaque,
            perceptual_roughness: 0.94,
            ..default()
        }),
        dead_tree_mesh: meshes.add(Cylinder::new(0.42, 2.8)),
        dead_tree_material: materials.add(StandardMaterial {
            base_color: palette.dead_tree,
            alpha_mode: AlphaMode::Opaque,
            perceptual_roughness: 0.99,
            ..default()
        }),
        crystal_mesh: meshes.add(Cone::new(0.58, 2.6)),
        crystal_material: materials.add(StandardMaterial {
            base_color: palette.crystal,
            emissive: palette.crystal.with_luminance(0.16).into(),
            alpha_mode: AlphaMode::Opaque,
            metallic: 0.18,
            perceptual_roughness: 0.43,
            ..default()
        }),
        ruin_mesh: meshes.add(Cuboid::new(1.0, 1.0, 1.0)),
        ruin_material: materials.add(StandardMaterial {
            base_color: palette.ruin,
            alpha_mode: AlphaMode::Opaque,
            perceptual_roughness: 0.88,
            ..default()
        }),
    });
    commands.insert_resource(LoadedProceduralChunks {
        chapter: active_chapter.0,
        entities: BTreeMap::new(),
    });
}

fn stream_procedural_chunks(
    mut commands: Commands,
    player: Query<&Transform, With<Player>>,
    active_chapter: Res<ActiveChapter>,
    world_seed: Res<ProceduralWorldSeed>,
    assets: Res<ProceduralWorldAssets>,
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
    assets: &ProceduralWorldAssets,
    chapter: ChapterId,
    world_seed: u64,
    coord: ChunkCoord,
) -> Entity {
    let plan = generate_chunk_plan(world_seed, chapter, coord);
    let center = coord.center();
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
            Mesh3d(assets.ground_mesh.clone()),
            MeshMaterial3d(assets.ground_materials[plan.ground_material].clone()),
            Transform::from_xyz(0.0, GROUND_Y, 0.0),
            OutdoorScenery,
            Name::new("Opaque Procedural Ground"),
        ));

        for spec in plan.decorations {
            let (mesh, material, name, height) = match spec.kind {
                DecorationKind::Rock => (
                    &assets.rock_mesh,
                    &assets.rock_material,
                    "Procedural Ridge Rock",
                    spec.scale.y,
                ),
                DecorationKind::DeadTree => (
                    &assets.dead_tree_mesh,
                    &assets.dead_tree_material,
                    "Procedural Dead Tree",
                    1.4 * spec.scale.y,
                ),
                DecorationKind::Crystal => (
                    &assets.crystal_mesh,
                    &assets.crystal_material,
                    "Procedural Crystal",
                    1.3 * spec.scale.y,
                ),
                DecorationKind::Ruin => (
                    &assets.ruin_mesh,
                    &assets.ruin_material,
                    "Procedural Ruin",
                    0.5 * spec.scale.y,
                ),
            };
            parent.spawn((
                Mesh3d(mesh.clone()),
                MeshMaterial3d(material.clone()),
                Transform::from_translation(spec.local_position + Vec3::Y * height)
                    .with_rotation(Quat::from_rotation_y(spec.yaw))
                    .with_scale(spec.scale),
                OutdoorScenery,
                Name::new(name),
            ));
        }
    });
    root
}

fn teardown_procedural_world(mut commands: Commands, chunks: Query<Entity, With<ProceduralChunk>>) {
    for entity in &chunks {
        commands.entity(entity).try_despawn();
    }
    commands.remove_resource::<LoadedProceduralChunks>();
    commands.remove_resource::<ProceduralWorldAssets>();
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
    let chapter_salt: u64 = match chapter {
        ChapterId::AshenReliquary => 0x4153_4845_4E5F_4249,
        ChapterId::HollowBastion => 0x484F_4C4C_4F57_4249,
    };
    let x = coord.x as f32;
    let z = coord.z as f32;
    let broad = value_noise(world_seed ^ chapter_salt, x / 4.8, z / 4.8);
    let detail = value_noise(
        world_seed ^ chapter_salt.rotate_left(23),
        (x + 19.0) / 2.35,
        (z - 31.0) / 2.35,
    );
    ((broad * 0.72 + detail * 0.28) * 4.0)
        .floor()
        .clamp(0.0, 3.0) as usize
}

fn value_noise(seed: u64, x: f32, z: f32) -> f32 {
    let x_floor = x.floor();
    let z_floor = z.floor();
    let x0 = x_floor as i64;
    let z0 = z_floor as i64;
    let tx = smoothstep(x - x_floor);
    let tz = smoothstep(z - z_floor);
    let n00 = lattice_value(seed, x0, z0);
    let n10 = lattice_value(seed, x0 + 1, z0);
    let n01 = lattice_value(seed, x0, z0 + 1);
    let n11 = lattice_value(seed, x0 + 1, z0 + 1);
    let north = n00 + (n10 - n00) * tx;
    let south = n01 + (n11 - n01) * tx;
    north + (south - north) * tz
}

fn lattice_value(seed: u64, x: i64, z: i64) -> f32 {
    let hash = SplitMix64::mix(
        seed ^ (x as u64).wrapping_mul(0xD6E8_FEB8_6659_FD93)
            ^ (z as u64)
                .rotate_left(32)
                .wrapping_mul(0xA5A3_564E_27F8_864D),
    );
    ((hash >> 40) as f32) * (1.0 / 16_777_216.0)
}

fn smoothstep(value: f32) -> f32 {
    value * value * (3.0 - 2.0 * value)
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

    fn mix(mut value: u64) -> u64 {
        value = (value ^ (value >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        value = (value ^ (value >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        value ^ (value >> 31)
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
    rock: Color,
    dead_tree: Color,
    crystal: Color,
    ruin: Color,
}

fn chapter_palette(chapter: ChapterId) -> ChapterPalette {
    match chapter {
        ChapterId::AshenReliquary => ChapterPalette {
            ground: [
                Color::srgb(0.090, 0.072, 0.060),
                Color::srgb(0.115, 0.086, 0.065),
                Color::srgb(0.080, 0.078, 0.071),
                Color::srgb(0.135, 0.088, 0.060),
            ],
            rock: Color::srgb(0.185, 0.165, 0.145),
            dead_tree: Color::srgb(0.115, 0.074, 0.050),
            crystal: Color::srgb(0.62, 0.105, 0.025),
            ruin: Color::srgb(0.215, 0.185, 0.155),
        },
        ChapterId::HollowBastion => ChapterPalette {
            ground: [
                Color::srgb(0.035, 0.060, 0.066),
                Color::srgb(0.043, 0.078, 0.082),
                Color::srgb(0.055, 0.071, 0.075),
                Color::srgb(0.035, 0.088, 0.094),
            ],
            rock: Color::srgb(0.095, 0.145, 0.150),
            dead_tree: Color::srgb(0.050, 0.095, 0.098),
            crystal: Color::srgb(0.025, 0.48, 0.68),
            ruin: Color::srgb(0.110, 0.165, 0.170),
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
