//! Authored PixelFX flipbooks placed in the depth-tested 3D combat world.
use crate::{GameState, assets::GameAssets, not_paused};
use bevy::{
    asset::RenderAssetUsages,
    light::{NotShadowCaster, NotShadowReceiver},
    math::Affine2,
    mesh::Indices,
    prelude::*,
    render::render_resource::PrimitiveTopology,
    transform::TransformSystems,
};
use serde::Deserialize;
use std::sync::LazyLock;

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq)]
pub(super) enum Kind {
    Slash,
    HeavyImpact,
    Fireball,
    FrostNova,
    Summon,
}
impl Kind {
    fn index(self) -> usize {
        self as usize
    }
    fn name(self) -> &'static str {
        match self {
            Self::Slash => "slash",
            Self::HeavyImpact => "heavy_impact",
            Self::Fireball => "fireball",
            Self::FrostNova => "frost_nova",
            Self::Summon => "summon",
        }
    }
}

#[derive(Deserialize)]
struct Manifest {
    effects: Vec<Sheet>,
}
#[derive(Deserialize)]
struct Sheet {
    kind: Kind,
    size: [f32; 2],
    columns: u32,
    rows: u32,
    pivot: [f32; 2],
    durations: Vec<f32>,
    looping: bool,
}
impl Sheet {
    fn duration(&self) -> f32 {
        self.durations.iter().sum()
    }
    fn frame_at(&self, elapsed: f32) -> Option<usize> {
        let duration = self.duration();
        if !self.looping && elapsed >= duration {
            return None;
        }
        let mut time = if self.looping {
            elapsed.rem_euclid(duration)
        } else {
            elapsed.max(0.0)
        };
        for (index, duration) in self.durations.iter().enumerate() {
            if time < *duration {
                return Some(index);
            }
            time -= duration;
        }
        Some(self.durations.len() - 1)
    }
    fn uv_transform(&self, frame: usize) -> Affine2 {
        let scale = Vec2::new(1.0 / self.columns as f32, 1.0 / self.rows as f32);
        let offset = Vec2::new(
            (frame as u32 % self.columns) as f32,
            (frame as u32 / self.columns) as f32,
        ) * scale;
        Affine2::from_scale_angle_translation(scale, 0.0, offset)
    }
    fn mesh(&self) -> Mesh {
        let width = self.size[0];
        let left = -self.pivot[0] / width;
        let right = (width - self.pivot[0]) / width;
        let top = self.pivot[1] / width;
        let bottom = (self.pivot[1] - self.size[1]) / width;
        Mesh::new(
            PrimitiveTopology::TriangleList,
            RenderAssetUsages::default(),
        )
        .with_inserted_attribute(
            Mesh::ATTRIBUTE_POSITION,
            vec![
                [left, top, 0.0],
                [left, bottom, 0.0],
                [right, bottom, 0.0],
                [right, top, 0.0],
            ],
        )
        .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, vec![[0.0, 0.0, 1.0]; 4])
        .with_inserted_attribute(
            Mesh::ATTRIBUTE_UV_0,
            vec![[0.0, 0.0], [0.0, 1.0], [1.0, 1.0], [1.0, 0.0]],
        )
        .with_inserted_indices(Indices::U32(vec![0, 1, 2, 0, 2, 3]))
    }
}
fn sheets() -> &'static [Sheet] {
    static MANIFEST: LazyLock<Manifest> = LazyLock::new(|| {
        ron::from_str(include_str!("../../assets/vfx/pixelfx/manifest.ron"))
            .expect("valid generated PixelFX manifest")
    });
    &MANIFEST.effects
}
fn sheet(kind: Kind) -> &'static Sheet {
    let sheet = &sheets()[kind.index()];
    debug_assert_eq!(sheet.kind, kind);
    sheet
}

#[derive(Message, Clone, Copy)]
pub(super) struct PixelFx {
    pub kind: Kind,
    pub position: Vec3,
    pub facing: Vec3,
    /// Width of the full source frame, in world metres.
    pub size: f32,
    pub follow: Option<Entity>,
}
impl PixelFx {
    pub fn at(kind: Kind, position: Vec3, facing: Vec3, size: f32) -> Self {
        Self {
            kind,
            position,
            facing,
            size,
            follow: None,
        }
    }
}
#[derive(Component)]
struct Playing {
    kind: Kind,
    elapsed: f32,
    facing: Vec3,
    follow: Option<Entity>,
}
#[derive(Resource)]
struct Art {
    meshes: Vec<Handle<Mesh>>,
}

pub(super) struct PixelFxPlugin;
impl Plugin for PixelFxPlugin {
    fn build(&self, app: &mut App) {
        app.add_message::<PixelFx>()
            .add_systems(Startup, prepare)
            .add_systems(
                PostUpdate,
                (spawn_effects, animate)
                    .chain()
                    .before(TransformSystems::Propagate)
                    .run_if(in_state(GameState::InGame).and_then(not_paused)),
            )
            .add_systems(OnExit(GameState::InGame), clear);
    }
}
fn prepare(mut commands: Commands, mut meshes: ResMut<Assets<Mesh>>) {
    commands.insert_resource(Art {
        meshes: sheets()
            .iter()
            .map(|sheet| meshes.add(sheet.mesh()))
            .collect(),
    });
}
fn spawn_effects(
    mut commands: Commands,
    assets: Res<GameAssets>,
    art: Res<Art>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut requests: MessageReader<PixelFx>,
) {
    for request in requests.read() {
        let material = materials.add(StandardMaterial {
            base_color_texture: Some(assets.pixelfx_sheets[request.kind.index()].clone()),
            alpha_mode: AlphaMode::Add,
            unlit: true,
            cull_mode: None,
            uv_transform: sheet(request.kind).uv_transform(0),
            ..default()
        });
        let entity = commands
            .spawn((
                Mesh3d(art.meshes[request.kind.index()].clone()),
                MeshMaterial3d(material),
                Transform::from_translation(request.position).with_scale(Vec3::splat(request.size)),
                Playing {
                    kind: request.kind,
                    elapsed: 0.0,
                    facing: request.facing,
                    follow: request.follow,
                },
                NotShadowCaster,
                NotShadowReceiver,
                Name::new(format!("PixelFX {}", request.kind.name())),
            ))
            .id();
        #[cfg(feature = "dev_tools")]
        info!(
            "PIXELFX_SPAWN {} entity={:?} position={:?}",
            request.kind.name(),
            entity,
            request.position
        );
        #[cfg(not(feature = "dev_tools"))]
        let _ = entity;
    }
}
fn effect_rotation(kind: Kind, facing: Vec3, camera: Quat) -> Quat {
    if kind == Kind::Slash {
        let yaw = facing.x.atan2(facing.z);
        Quat::from_rotation_y(yaw - std::f32::consts::FRAC_PI_2)
            * Quat::from_rotation_x(-std::f32::consts::FRAC_PI_2)
    } else if kind == Kind::FrostNova {
        Quat::from_rotation_x(-std::f32::consts::FRAC_PI_2)
    } else if kind == Kind::Fireball {
        let right = camera * Vec3::X;
        let up = camera * Vec3::Y;
        let angle = facing.dot(up).atan2(facing.dot(right));
        camera * Quat::from_rotation_z(angle)
    } else {
        camera
    }
}
fn animate(
    time: Res<Time>,
    mut commands: Commands,
    cameras: Query<&Transform, (With<Camera3d>, Without<Playing>)>,
    targets: Query<&Transform, (Without<Playing>, Without<Camera3d>)>,
    mut effects: Query<(
        Entity,
        &mut Playing,
        &mut Transform,
        &MeshMaterial3d<StandardMaterial>,
    )>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let camera = cameras
        .single()
        .map_or(Quat::IDENTITY, |camera| camera.rotation);
    for (entity, mut playing, mut transform, material) in &mut effects {
        if let Some(owner) = playing.follow {
            let Ok(target) = targets.get(owner) else {
                materials.remove(material.0.id());
                commands.entity(entity).despawn();
                continue;
            };
            transform.translation = target.translation;
        }
        let Some(frame) = sheet(playing.kind).frame_at(playing.elapsed) else {
            materials.remove(material.0.id());
            commands.entity(entity).despawn();
            continue;
        };
        transform.rotation = effect_rotation(playing.kind, playing.facing, camera);
        if let Some(mut material) = materials.get_mut(material.0.id()) {
            material.uv_transform = sheet(playing.kind).uv_transform(frame);
        }
        playing.elapsed += time.delta_secs();
    }
}
fn clear(
    mut commands: Commands,
    effects: Query<(Entity, &MeshMaterial3d<StandardMaterial>), With<Playing>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut requests: ResMut<Messages<PixelFx>>,
) {
    for (entity, material) in &effects {
        materials.remove(material.0.id());
        commands.entity(entity).despawn();
    }
    requests.clear();
}
pub(super) fn clear_transient(world: &mut World) {
    let effects: Vec<_> = world
        .query_filtered::<(Entity, &MeshMaterial3d<StandardMaterial>), With<Playing>>()
        .iter(world)
        .map(|(entity, material)| (entity, material.0.id()))
        .collect();
    for (entity, material) in effects {
        if let Some(mut materials) = world.get_resource_mut::<Assets<StandardMaterial>>() {
            materials.remove(material);
        }
        world.entity_mut(entity).despawn();
    }
    if let Some(mut messages) = world.get_resource_mut::<Messages<PixelFx>>() {
        messages.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn generated_sheets_have_complete_timing_and_grid_metadata() {
        assert_eq!(sheets().len(), 5);
        for (index, sheet) in sheets().iter().enumerate() {
            assert_eq!(sheet.kind.index(), index);
            assert!(sheet.size.iter().all(|size| *size > 0.0));
            assert!(sheet.durations.len() >= 8);
            assert!(sheet.durations.iter().all(|duration| *duration > 0.0));
            assert!(sheet.columns * sheet.rows >= sheet.durations.len() as u32);
            assert!(sheet.duration() > 0.1);
            assert!(sheet.frame_at(0.0).is_some());
            assert_eq!(sheet.frame_at(sheet.duration()).is_some(), sheet.looping);
            let last = sheet.uv_transform(sheet.durations.len() - 1);
            let corner = last.transform_point2(Vec2::ONE);
            assert!(corner.x <= 1.0 && corner.y <= 1.0);
        }
    }
    #[test]
    fn atlas_timing_crosses_frames_once_and_honours_individual_durations() {
        let sheet = Sheet {
            kind: Kind::Slash,
            size: [16.0; 2],
            columns: 2,
            rows: 2,
            pivot: [8.0; 2],
            durations: vec![0.03, 0.07, 0.04],
            looping: false,
        };
        assert_eq!(sheet.frame_at(0.029), Some(0));
        assert_eq!(sheet.frame_at(0.03), Some(1));
        assert_eq!(sheet.frame_at(0.101), Some(2));
        assert_eq!(sheet.frame_at(0.15), None);
        assert_eq!(
            sheet.uv_transform(2).transform_point2(Vec2::ZERO),
            Vec2::new(0.0, 0.5)
        );
    }
    #[test]
    fn directional_sheets_point_towards_actual_attack_in_all_quadrants() {
        for facing in [Vec3::X, Vec3::NEG_X, Vec3::Z, Vec3::NEG_Z] {
            let rotation = effect_rotation(Kind::Slash, facing, Quat::IDENTITY);
            assert!((rotation * Vec3::X).abs_diff_eq(facing, 0.00001));
            let frost = effect_rotation(Kind::FrostNova, facing, Quat::IDENTITY);
            assert!((frost * Vec3::Z).abs_diff_eq(Vec3::Y, 0.00001));
        }
    }
}
