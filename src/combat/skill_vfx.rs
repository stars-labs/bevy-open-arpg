//! Readable skill silhouettes, driven by actual releases, not key presses.
use super::{
    CombatSet, SkillCast,
    skill_vfx_art::{arc, mesh, ribbon},
};
use crate::{
    GameState,
    class::{Rune, StrikeShape},
    not_paused,
};
use bevy::{
    light::{NotShadowCaster, NotShadowReceiver},
    prelude::*,
};

#[derive(Message, Clone, Copy)]
pub(super) struct SkillVisual {
    pub origin: Vec3,
    pub facing: Vec3,
    pub shape: StrikeShape,
    pub cast: SkillCast,
    pub strength: f32,
}

#[derive(Message, Clone, Copy)]
pub(super) struct MovementVisual {
    pub start: Vec3,
    pub end: Vec3,
    pub rune: Rune,
}

#[derive(Message, Clone, Copy)]
pub(super) struct ProjectileVisual {
    pub entity: Entity,
    pub rune: Rune,
    pub width: f32,
}

pub(super) struct SkillVfxPlugin;
impl Plugin for SkillVfxPlugin {
    fn build(&self, app: &mut App) {
        app.add_message::<SkillVisual>()
            .add_message::<MovementVisual>()
            .add_message::<ProjectileVisual>()
            .add_systems(Startup, prepare_art)
            .add_systems(
                Update,
                (
                    show_releases,
                    show_movement,
                    show_projectile_trails,
                    animate,
                )
                    .chain()
                    .after(CombatSet::PlayerDamage)
                    .run_if(in_state(GameState::InGame).and_then(not_paused)),
            )
            .add_systems(OnExit(GameState::InGame), clear);
    }
}

#[derive(Resource)]
struct Art {
    crescent: Handle<Mesh>,
    ring: Handle<Mesh>,
    fracture: Handle<Mesh>,
    frost: Handle<Mesh>,
    streak: Handle<Mesh>,
}

fn prepare_art(mut commands: Commands, mut meshes: ResMut<Assets<Mesh>>) {
    let mut cracks = Vec::new();
    for i in 0..7 {
        let a = i as f32 * std::f32::consts::TAU / 7.0;
        let point = |r: f32, offset: f32| Vec2::new((a + offset).sin(), (a + offset).cos()) * r;
        ribbon(
            &mut cracks,
            point(0.12, 0.0),
            point(0.55, 0.08),
            0.012,
            0.024,
        );
        ribbon(
            &mut cracks,
            point(0.55, 0.08),
            point(0.96, -0.07),
            0.024,
            0.0,
        );
        ribbon(
            &mut cracks,
            point(0.55, 0.08),
            point(0.78, 0.24),
            0.015,
            0.0,
        );
    }
    let mut ice = Vec::new();
    for i in 0..6 {
        let a = i as f32 * std::f32::consts::TAU / 6.0;
        let point = |r: f32, offset: f32| Vec2::new((a + offset).sin(), (a + offset).cos()) * r;
        ribbon(&mut ice, point(0.18, 0.0), point(0.96, 0.0), 0.024, 0.0);
        for offset in [-0.23, 0.23] {
            ribbon(&mut ice, point(0.55, 0.0), point(0.82, offset), 0.018, 0.0);
        }
    }
    let mut streak = Vec::new();
    ribbon(&mut streak, Vec2::ZERO, Vec2::Y * 0.4, 0.0, 0.065);
    ribbon(&mut streak, Vec2::Y * 0.4, Vec2::Y, 0.065, 0.0);
    commands.insert_resource(Art {
        crescent: meshes.add(arc(2.65, 0.065)),
        ring: meshes.add(arc(std::f32::consts::TAU, 0.017)),
        fracture: meshes.add(mesh(cracks)),
        frost: meshes.add(mesh(ice)),
        streak: meshes.add(mesh(streak)),
    });
}
fn tint(rune: Rune) -> Color {
    match rune {
        Rune::Expose => Color::srgb(1.0, 0.76, 0.32),
        Rune::Hemorrhage => Color::srgb(0.95, 0.16, 0.12),
        Rune::Cleanse => Color::srgb(0.60, 0.91, 0.86),
        Rune::Reap => Color::srgb(0.48, 0.79, 0.56),
        Rune::Ember => Color::srgb(1.0, 0.40, 0.08),
        Rune::Frost => Color::srgb(0.36, 0.78, 1.0),
    }
}
#[derive(Component)]
struct SkillParticle {
    age: f32,
    duration: f32,
    start_scale: Vec3,
    end_scale: Vec3,
    velocity: Vec3,
    spin: f32,
    color: Color,
}
struct Stroke {
    pose: Transform,
    end_scale: Vec3,
    duration: f32,
    velocity: Vec3,
    spin: f32,
}
fn spawn(
    commands: &mut Commands,
    materials: &mut Assets<StandardMaterial>,
    mesh: Handle<Mesh>,
    rune: Rune,
    stroke: Stroke,
) {
    let color = tint(rune);
    let material = materials.add(StandardMaterial {
        base_color: color,
        emissive: LinearRgba::from(color) * 2.5,
        alpha_mode: AlphaMode::Add,
        unlit: true,
        cull_mode: None,
        ..default()
    });
    commands.spawn((
        Mesh3d(mesh),
        MeshMaterial3d(material),
        NotShadowCaster,
        NotShadowReceiver,
        stroke.pose,
        SkillParticle {
            age: 0.0,
            duration: stroke.duration,
            start_scale: stroke.pose.scale,
            end_scale: stroke.end_scale,
            velocity: stroke.velocity,
            spin: stroke.spin,
            color,
        },
        Name::new("Skill silhouette"),
    ));
}
fn show_releases(
    mut commands: Commands,
    art: Res<Art>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut events: MessageReader<SkillVisual>,
) {
    for e in events.read() {
        #[cfg(feature = "dev_tools")]
        info!(
            "SKILL_VFX_RELEASE {:?} {:?} {:?}",
            e.cast.slot, e.cast.rune, e.shape
        );
        let yaw = e.facing.x.atan2(e.facing.z);
        // Growth changes weight/height slightly; it never lies about the hitbox.
        let weight = e.strength.clamp(0.8, 1.85);
        let base = e.origin + Vec3::Y * 0.09;
        match e.shape {
            StrikeShape::Command { .. } => {}
            StrikeShape::Arc { range } => {
                let scale = Vec3::new(range * 0.64, 1.0, range * 0.64);
                spawn(
                    &mut commands,
                    &mut materials,
                    art.crescent.clone(),
                    e.cast.rune,
                    Stroke {
                        pose: Transform::from_translation(base + Vec3::Y * 0.48)
                            .with_rotation(Quat::from_rotation_y(yaw - 0.18))
                            .with_scale(scale),
                        end_scale: Vec3::new(range * 0.96, 1.0, range * 0.96),
                        duration: 0.32,
                        velocity: e.facing * 0.25,
                        spin: 1.2,
                    },
                );
                // The heavy cut also traces a short ground fracture in front of the hero.
                spawn(
                    &mut commands,
                    &mut materials,
                    art.streak.clone(),
                    e.cast.rune,
                    Stroke {
                        pose: Transform::from_translation(base)
                            .with_rotation(Quat::from_rotation_y(yaw))
                            .with_scale(Vec3::new(weight * 0.8, 1.0, range)),
                        end_scale: Vec3::new(0.0, 1.0, range),
                        duration: 0.42,
                        velocity: Vec3::ZERO,
                        spin: 0.0,
                    },
                );
            }
            StrikeShape::Radial { radius } => {
                spawn(
                    &mut commands,
                    &mut materials,
                    art.ring.clone(),
                    e.cast.rune,
                    Stroke {
                        pose: Transform::from_translation(base)
                            .with_scale(Vec3::splat(radius * 0.22)),
                        end_scale: Vec3::splat(radius),
                        duration: 0.48,
                        velocity: Vec3::ZERO,
                        spin: 0.0,
                    },
                );
                spawn(
                    &mut commands,
                    &mut materials,
                    if e.cast.rune == Rune::Frost {
                        art.frost.clone()
                    } else {
                        art.fracture.clone()
                    },
                    e.cast.rune,
                    Stroke {
                        pose: Transform::from_translation(base + Vec3::Y * 0.015)
                            .with_rotation(Quat::from_rotation_y(yaw))
                            .with_scale(Vec3::splat(radius * 0.58)),
                        end_scale: Vec3::splat(radius * 0.88),
                        duration: 0.64,
                        velocity: Vec3::ZERO,
                        spin: 0.0,
                    },
                );
                // Sparse pointed embers/ice splinters rise outward, leaving the centre open.
                for i in 0..6 {
                    let angle = i as f32 * std::f32::consts::TAU / 6.0 + yaw;
                    let outward = Vec3::new(angle.sin(), 0.0, angle.cos());
                    spawn(
                        &mut commands,
                        &mut materials,
                        art.streak.clone(),
                        e.cast.rune,
                        Stroke {
                            pose: Transform::from_translation(
                                base + outward * radius * 0.24 + Vec3::Y * 0.14,
                            )
                            .with_rotation(
                                Quat::from_rotation_y(angle) * Quat::from_rotation_x(-0.65),
                            )
                            .with_scale(Vec3::new(
                                0.6,
                                1.0,
                                0.45 + weight * 0.14,
                            )),
                            end_scale: Vec3::ZERO,
                            duration: 0.40 + i as f32 * 0.025,
                            velocity: outward * radius * 1.1 + Vec3::Y * 0.45,
                            spin: 0.0,
                        },
                    );
                }
            }
            StrikeShape::GroundBlast { radius, delay, .. } => {
                // A bright boundary charges at the target; descending streaks arrive
                // when damage resolves. The hero and enemies remain visible inside it.
                spawn(
                    &mut commands,
                    &mut materials,
                    art.ring.clone(),
                    e.cast.rune,
                    Stroke {
                        pose: Transform::from_translation(base).with_scale(Vec3::splat(radius)),
                        end_scale: Vec3::splat(radius * 0.96),
                        duration: delay,
                        velocity: Vec3::ZERO,
                        spin: 0.7,
                    },
                );
                for i in 0..5 {
                    let angle = i as f32 * 2.399;
                    let offset = Vec3::new(angle.sin(), 0.0, angle.cos()) * radius * 0.45;
                    spawn(
                        &mut commands,
                        &mut materials,
                        art.streak.clone(),
                        e.cast.rune,
                        Stroke {
                            pose: Transform::from_translation(base + offset + Vec3::Y * 2.6)
                                .with_rotation(Quat::from_rotation_x(std::f32::consts::FRAC_PI_2))
                                .with_scale(Vec3::new(0.6, 1.0, 0.8)),
                            end_scale: Vec3::new(0.2, 1.0, 1.1),
                            duration: delay,
                            velocity: Vec3::NEG_Y * (2.6 / delay.max(0.05)),
                            spin: 0.0,
                        },
                    );
                }
            }
            StrikeShape::Projectiles { .. } => {
                spawn(
                    &mut commands,
                    &mut materials,
                    art.crescent.clone(),
                    e.cast.rune,
                    Stroke {
                        pose: Transform::from_translation(
                            e.origin + e.facing * 0.65 + Vec3::Y * 0.9,
                        )
                        .with_rotation(Quat::from_rotation_y(yaw))
                        .with_scale(Vec3::splat(0.24)),
                        end_scale: Vec3::splat(0.5),
                        duration: 0.24,
                        velocity: e.facing * 0.4,
                        spin: 0.0,
                    },
                );
            }
        }
    }
}
fn show_movement(
    mut commands: Commands,
    art: Res<Art>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut events: MessageReader<MovementVisual>,
) {
    for e in events.read() {
        let delta = (e.end - e.start).with_y(0.0);
        if delta.length_squared() < 0.01 {
            continue;
        }
        let yaw = delta.x.atan2(delta.z);
        let side = Vec3::new(delta.z, 0.0, -delta.x).normalize_or_zero();
        for offset in [-0.20, 0.20] {
            spawn(
                &mut commands,
                &mut materials,
                art.streak.clone(),
                e.rune,
                Stroke {
                    pose: Transform::from_translation(e.start + side * offset + Vec3::Y * 0.32)
                        .with_rotation(Quat::from_rotation_y(yaw))
                        .with_scale(Vec3::new(0.7, 1.0, delta.length())),
                    end_scale: Vec3::new(0.0, 1.0, delta.length()),
                    duration: 0.38,
                    velocity: Vec3::ZERO,
                    spin: 0.0,
                },
            );
        }
    }
}
fn show_projectile_trails(
    mut commands: Commands,
    art: Res<Art>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut events: MessageReader<ProjectileVisual>,
    projectiles: Query<(), With<super::ranged::PlayerProjectile>>,
) {
    for event in events.read() {
        if projectiles.get(event.entity).is_err() {
            continue;
        }
        let color = tint(event.rune);
        let material = materials.add(StandardMaterial {
            base_color: color.with_alpha(0.65),
            emissive: LinearRgba::from(color) * 1.5,
            alpha_mode: AlphaMode::Add,
            unlit: true,
            cull_mode: None,
            ..default()
        });
        // Attached to the damaging missile, never left behind as persistent clutter.
        commands.entity(event.entity).with_children(|parent| {
            parent.spawn((
                Mesh3d(art.streak.clone()),
                MeshMaterial3d(material),
                NotShadowCaster,
                NotShadowReceiver,
                Transform::from_translation(Vec3::new(0.0, 0.015, -1.0)).with_scale(Vec3::new(
                    event.width.clamp(0.2, 0.7) * 3.0,
                    1.0,
                    1.0,
                )),
                Name::new("Skill missile tapered wake"),
            ));
        });
    }
}
fn animate(
    time: Res<Time>,
    mut commands: Commands,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut particles: Query<(
        Entity,
        &mut Transform,
        &MeshMaterial3d<StandardMaterial>,
        &mut SkillParticle,
    )>,
) {
    for (entity, mut pose, handle, mut p) in &mut particles {
        p.age += time.delta_secs();
        if p.age >= p.duration {
            commands.entity(entity).despawn();
            continue;
        }
        let t = p.age / p.duration;
        let ease = 1.0 - (1.0 - t).powi(3);
        pose.scale = p.start_scale.lerp(p.end_scale, ease);
        pose.translation += p.velocity * time.delta_secs();
        pose.rotate_y(p.spin * time.delta_secs());
        if let Some(mut material) = materials.get_mut(&handle.0) {
            let opacity = (1.0 - t).powf(0.65);
            material.base_color = p.color.with_alpha(opacity);
            material.emissive = LinearRgba::from(p.color) * (2.5 * opacity);
        }
    }
}
fn clear(mut commands: Commands, particles: Query<Entity, With<SkillParticle>>) {
    for entity in &particles {
        commands.entity(entity).despawn();
    }
}
