//! Readable skill silhouettes, driven by actual releases, not key presses.
use super::{
    CombatSet, SkillCast,
    skill_vfx_art::{arc, mesh, ribbon},
};
use crate::{
    GameState,
    class::{HeroClass, ProjectileArt, Rune, SelectedClass, StrikeShape},
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

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum EvadeVfxPhase {
    Start,
    Trail,
    End,
}

#[derive(Message, Clone, Copy, Debug)]
pub(crate) struct EvadeVisual {
    pub origin: Vec3,
    pub direction: Vec3,
    pub phase: EvadeVfxPhase,
}

pub(super) struct SkillVfxPlugin;
impl Plugin for SkillVfxPlugin {
    fn build(&self, app: &mut App) {
        app.add_message::<SkillVisual>()
            .add_message::<MovementVisual>()
            .add_message::<ProjectileVisual>()
            .add_message::<EvadeVisual>()
            .add_systems(Startup, prepare_art)
            .add_systems(
                Update,
                (
                    show_releases,
                    show_movement,
                    show_evade,
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
    spawn_colored(commands, materials, mesh, tint(rune), 2.5, stroke);
}

fn spawn_colored(
    commands: &mut Commands,
    materials: &mut Assets<StandardMaterial>,
    mesh: Handle<Mesh>,
    color: Color,
    emissive_boost: f32,
    stroke: Stroke,
) {
    let material = materials.add(StandardMaterial {
        base_color: color,
        emissive: LinearRgba::from(color) * emissive_boost,
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

fn evade_palette(class: HeroClass) -> (Color, Color) {
    match class {
        HeroClass::Warden => (Color::srgb(0.95, 0.74, 0.34), Color::srgb(0.92, 0.96, 1.0)),
        HeroClass::Stalker => (Color::srgb(0.24, 0.88, 0.68), Color::srgb(0.75, 1.0, 0.90)),
        HeroClass::Soulbinder => (Color::srgb(0.14, 0.92, 0.76), Color::srgb(0.68, 1.0, 0.95)),
        HeroClass::Embercaller => (Color::srgb(1.0, 0.42, 0.10), Color::srgb(1.0, 0.86, 0.48)),
    }
}
fn show_releases(
    mut commands: Commands,
    art: Res<Art>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut events: MessageReader<SkillVisual>,
    selected: Res<SelectedClass>,
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
            StrikeShape::Arc { range } if selected.0 == HeroClass::Warden => {
                commands.write_message(super::pixelfx::PixelFx::at(
                    super::pixelfx::Kind::Slash,
                    e.origin + Vec3::Y * 0.65,
                    e.facing,
                    range * 1.25,
                ));
            }
            StrikeShape::Radial { radius } if e.cast.rune == Rune::Frost => {
                commands.write_message(super::pixelfx::PixelFx::at(
                    super::pixelfx::Kind::FrostNova,
                    base,
                    e.facing,
                    radius * 2.5,
                ));
            }
            StrikeShape::Projectiles {
                art: ProjectileArt::EmberBolt,
                ..
            } => {}
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
fn show_evade(
    mut commands: Commands,
    art: Res<Art>,
    selected: Res<SelectedClass>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut events: MessageReader<EvadeVisual>,
) {
    let (primary, core) = evade_palette(selected.0);
    for e in events.read() {
        let dir = Vec3::new(e.direction.x, 0.0, e.direction.z).normalize_or_zero();
        let forward = if dir.length_squared() > 0.01 {
            dir
        } else {
            Vec3::Z
        };
        let yaw = forward.x.atan2(forward.z);
        let side = Vec3::new(forward.z, 0.0, -forward.x);
        let base = e.origin + Vec3::Y * 0.08;
        match e.phase {
            EvadeVfxPhase::Start => {
                // Departure phase rift ring + bright inner flash ring
                spawn_colored(
                    &mut commands,
                    &mut materials,
                    art.ring.clone(),
                    primary,
                    2.6,
                    Stroke {
                        pose: Transform::from_translation(base).with_scale(Vec3::splat(0.32)),
                        end_scale: Vec3::splat(1.42),
                        duration: 0.26,
                        velocity: Vec3::ZERO,
                        spin: 2.6,
                    },
                );
                spawn_colored(
                    &mut commands,
                    &mut materials,
                    art.ring.clone(),
                    core,
                    3.2,
                    Stroke {
                        pose: Transform::from_translation(base + Vec3::Y * 0.04)
                            .with_scale(Vec3::splat(0.20)),
                        end_scale: Vec3::splat(0.88),
                        duration: 0.16,
                        velocity: Vec3::ZERO,
                        spin: -3.4,
                    },
                );
                // Swept directional shock crescent launching along the evade vector
                spawn_colored(
                    &mut commands,
                    &mut materials,
                    art.crescent.clone(),
                    core,
                    2.8,
                    Stroke {
                        pose: Transform::from_translation(base + Vec3::Y * 0.48 + forward * 0.12)
                            .with_rotation(Quat::from_rotation_y(yaw))
                            .with_scale(Vec3::new(0.55, 1.0, 0.62)),
                        end_scale: Vec3::new(1.32, 1.0, 1.42),
                        duration: 0.22,
                        velocity: forward * 1.4,
                        spin: 0.0,
                    },
                );
                // Vertical phase-rift ribbons rising at takeoff so departure feels like a blink
                for (offset, color, width, height) in [
                    (0.0, core, 1.55, 1.85),
                    (-0.24, primary, 1.05, 1.45),
                    (0.24, primary, 1.05, 1.45),
                ] {
                    spawn_colored(
                        &mut commands,
                        &mut materials,
                        art.streak.clone(),
                        color,
                        2.8,
                        Stroke {
                            pose: Transform::from_translation(
                                base + side * offset + Vec3::Y * 0.06,
                            )
                            .with_rotation(
                                Quat::from_rotation_y(yaw)
                                    * Quat::from_rotation_x(-std::f32::consts::FRAC_PI_2),
                            )
                            .with_scale(Vec3::new(width, 1.0, height)),
                            end_scale: Vec3::new(0.0, 1.0, height * 1.25),
                            duration: 0.24,
                            velocity: Vec3::Y * 0.55 - forward * 0.35,
                            spin: 0.0,
                        },
                    );
                }
                // Six directional velocity shards kicking backward along -forward
                for i in 0..6 {
                    let spread = (i as f32 - 2.5) * 0.18;
                    let back_yaw = yaw + std::f32::consts::PI + spread;
                    let back_dir = Vec3::new(back_yaw.sin(), 0.0, back_yaw.cos());
                    let lift = 0.26 + (i % 3) as f32 * 0.26;
                    spawn_colored(
                        &mut commands,
                        &mut materials,
                        art.streak.clone(),
                        if i % 2 == 0 { core } else { primary },
                        2.6,
                        Stroke {
                            pose: Transform::from_translation(base + Vec3::Y * lift)
                                .with_rotation(Quat::from_rotation_y(back_yaw))
                                .with_scale(Vec3::new(1.05, 1.0, 0.92)),
                            end_scale: Vec3::new(0.0, 1.0, 1.35),
                            duration: 0.24,
                            velocity: back_dir * (2.2 + (i % 2) as f32 * 0.9) + Vec3::Y * 0.35,
                            spin: 0.0,
                        },
                    );
                }
            }
            EvadeVfxPhase::Trail => {
                // Sleek longitudinal slipstream ribbons streaming straight behind the hero
                let back_yaw = yaw + std::f32::consts::PI;
                for (lat, height, color, width, len) in [
                    (0.0, 0.88, core, 1.15, 1.55),
                    (-0.28, 0.56, primary, 0.85, 1.35),
                    (0.28, 0.56, primary, 0.85, 1.35),
                ] {
                    spawn_colored(
                        &mut commands,
                        &mut materials,
                        art.streak.clone(),
                        color,
                        2.4,
                        Stroke {
                            pose: Transform::from_translation(
                                e.origin + side * lat + Vec3::Y * height - forward * 0.12,
                            )
                            .with_rotation(Quat::from_rotation_y(back_yaw))
                            .with_scale(Vec3::new(width, 1.0, len)),
                            end_scale: Vec3::new(0.0, 1.0, len * 1.25),
                            duration: 0.19,
                            velocity: -forward * 0.65,
                            spin: 0.0,
                        },
                    );
                }
            }
            EvadeVfxPhase::End => {
                // Crisp contracting arrival ring + compact forward snap crescent
                spawn_colored(
                    &mut commands,
                    &mut materials,
                    art.ring.clone(),
                    primary,
                    2.6,
                    Stroke {
                        pose: Transform::from_translation(base).with_scale(Vec3::splat(1.12)),
                        end_scale: Vec3::splat(0.46),
                        duration: 0.18,
                        velocity: Vec3::ZERO,
                        spin: -2.4,
                    },
                );
                spawn_colored(
                    &mut commands,
                    &mut materials,
                    art.crescent.clone(),
                    core,
                    2.8,
                    Stroke {
                        pose: Transform::from_translation(base + Vec3::Y * 0.34 + forward * 0.18)
                            .with_rotation(Quat::from_rotation_y(yaw))
                            .with_scale(Vec3::new(0.48, 1.0, 0.52)),
                        end_scale: Vec3::new(0.98, 1.0, 1.02),
                        duration: 0.16,
                        velocity: forward * 0.75,
                        spin: 0.0,
                    },
                );
            }
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn evade_visual_emits_distinct_start_trail_and_end_particles() {
        let mut app = App::new();
        app.init_resource::<Assets<Mesh>>()
            .init_resource::<Assets<StandardMaterial>>()
            .insert_resource(SelectedClass(HeroClass::Soulbinder))
            .add_message::<EvadeVisual>()
            .add_systems(Startup, prepare_art)
            .add_systems(Update, show_evade);

        let count_particles = |app: &mut App| {
            app.world_mut()
                .query_filtered::<Entity, With<SkillParticle>>()
                .iter(app.world())
                .count()
        };

        app.world_mut().write_message(EvadeVisual {
            origin: Vec3::ZERO,
            direction: Vec3::Z,
            phase: EvadeVfxPhase::Start,
        });
        app.update();
        let after_start = count_particles(&mut app);
        assert!(
            after_start >= 10,
            "start phase should emit departure ring, rift, crescent, and shards"
        );

        app.world_mut().write_message(EvadeVisual {
            origin: Vec3::Z * 1.5,
            direction: Vec3::Z,
            phase: EvadeVfxPhase::Trail,
        });
        app.update();
        let after_trail = count_particles(&mut app);
        assert_eq!(
            after_trail - after_start,
            3,
            "trail phase should emit 3 longitudinal slipstream streaks"
        );

        app.world_mut().write_message(EvadeVisual {
            origin: Vec3::Z * 3.0,
            direction: Vec3::Z,
            phase: EvadeVfxPhase::End,
        });
        app.update();
        let after_end = count_particles(&mut app);
        assert_eq!(
            after_end - after_trail,
            2,
            "end phase should emit contracting arrival ring and snap crescent"
        );
    }
}
