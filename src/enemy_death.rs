//! Keep the authored enemy visible through its fall instead of replacing it with a primitive.
use crate::{
    GameState,
    chapter::{ActiveChapter, ChapterId},
    enemy::Enemy,
    not_paused,
    player::Health,
};
use bevy::prelude::*;

const FALL_SECONDS: f32 = 0.48;
const REST_SECONDS: f32 = 3.0;
const SINK_SECONDS: f32 = 1.2;
const CORPSE_SECONDS: f32 = FALL_SECONDS + REST_SECONDS + SINK_SECONDS;
/// Motes released as the body drops, then again as the ground reclaims it.
const FALL_MOTES: u32 = 6;
const SINK_MOTES: u32 = 12;
const GOLDEN_ANGLE: f32 = 2.399_963;

/// Runtime UI and affix decorations must not remain attached to a fallen body.
#[derive(Component)]
pub(crate) struct DeathDecoration;

#[derive(Component)]
struct FallenEnemy {
    timer: Timer,
    initial: Transform,
    /// How many of the two mote bursts have been released.
    bursts: u8,
}

/// A small glowing ember that rises from a body and shrinks to nothing.
#[derive(Component, Debug, Clone, Copy)]
struct DeathMote {
    origin: Vec3,
    drift: Vec3,
    age: f32,
    life: f32,
    size: f32,
}

#[derive(Resource)]
struct DeathMoteAssets {
    mesh: Handle<Mesh>,
    material: Handle<StandardMaterial>,
}

pub(crate) struct EnemyDeathPlugin;

impl Plugin for EnemyDeathPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            (
                animate_fallen_enemies,
                release_death_motes,
                animate_death_motes,
            )
                .chain()
                .run_if(in_state(GameState::InGame).and_then(not_paused)),
        )
        .add_systems(OnEnter(GameState::InGame), setup_death_motes)
        .add_systems(OnExit(GameState::InGame), cleanup_fallen_enemies);
    }
}

/// Rewards are issued by loot before this runs. Removing combat components at
/// the same command boundary makes the retained mesh inert and prevents a
/// second reward while preserving its existing, already loaded model hierarchy.
pub(crate) fn begin_enemy_death(commands: &mut Commands, entity: Entity, initial: Transform) {
    commands.queue(move |world: &mut World| {
        if world.get_entity(entity).is_err() || world.get::<FallenEnemy>(entity).is_some() {
            return;
        }
        let mut descendants = vec![entity];
        let mut decorations = Vec::new();
        while let Some(parent) = descendants.pop() {
            if let Some(children) = world.get::<Children>(parent) {
                descendants.extend(children.iter());
            }
            if world.get::<DeathDecoration>(parent).is_some() {
                decorations.push(parent);
            }
        }
        for decoration in decorations {
            if let Ok(entity) = world.get_entity_mut(decoration) {
                entity.despawn();
            }
        }
        world
            .entity_mut(entity)
            .remove::<(Enemy, Health)>()
            .insert(FallenEnemy {
                timer: Timer::from_seconds(CORPSE_SECONDS, TimerMode::Once),
                initial,
                bursts: 0,
            });
    });
}

fn setup_death_motes(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    chapter: Res<ActiveChapter>,
) {
    // Emissive well above 1.0 so the bloom pass turns each mote into a spark.
    let glow = match chapter.0 {
        ChapterId::AshenReliquary => LinearRgba::rgb(7.0, 2.2, 0.5),
        ChapterId::HollowBastion => LinearRgba::rgb(0.7, 4.2, 6.0),
    };
    commands.insert_resource(DeathMoteAssets {
        mesh: meshes.add(Sphere::new(0.5).mesh().ico(1).expect("valid icosphere")),
        material: materials.add(StandardMaterial {
            base_color: Color::BLACK,
            emissive: glow,
            perceptual_roughness: 1.0,
            ..default()
        }),
    });
}

/// Burst `burst` (0 = falling, 1 = sinking) for a body at `origin`. Positions
/// come from the body's own coordinates, so a replay shows the same embers.
fn death_motes(origin: Vec3, burst: u8) -> impl Iterator<Item = DeathMote> {
    let count = if burst == 0 { FALL_MOTES } else { SINK_MOTES };
    let phase = origin.x * 1.7 + origin.z * 2.3 + burst as f32;
    (0..count).map(move |index| {
        let step = index as f32 / count as f32;
        let angle = phase + index as f32 * GOLDEN_ANGLE;
        let ring = Vec3::new(angle.cos(), 0.0, angle.sin());
        let (spread, rise, life) = if burst == 0 {
            (0.35 + 0.35 * step, 0.9 + 0.5 * step, 0.55 + 0.25 * step)
        } else {
            (0.25 + 0.55 * step, 1.3 + 0.9 * step, 0.9 + 0.5 * step)
        };
        DeathMote {
            origin: origin + ring * spread * 0.6 + Vec3::Y * 0.25,
            drift: ring * spread * 0.5 + Vec3::Y * rise,
            // Staggered starts read as a stream rather than a single pop.
            age: -step * if burst == 0 { 0.12 } else { 0.7 },
            life,
            size: 0.07 + 0.05 * ((index * 7) % 5) as f32 / 4.0,
        }
    })
}

/// Position and scale over a mote's life: it eases upward and swells briefly
/// before shrinking away. Opaque geometry, so nothing ever flashes the screen.
fn mote_pose(mote: &DeathMote) -> (Vec3, f32) {
    let progress = (mote.age / mote.life).clamp(0.0, 1.0);
    let eased = 1.0 - (1.0 - progress) * (1.0 - progress);
    let envelope = (progress * 6.0).min(1.0) * (1.0 - progress);
    (mote.origin + mote.drift * eased, mote.size * envelope)
}

fn release_death_motes(
    mut commands: Commands,
    assets: Option<Res<DeathMoteAssets>>,
    mut enemies: Query<&mut FallenEnemy>,
) {
    let Some(assets) = assets else {
        return;
    };
    for mut corpse in &mut enemies {
        let elapsed = corpse.timer.elapsed_secs();
        let due = if elapsed >= FALL_SECONDS + REST_SECONDS {
            2
        } else {
            1
        };
        while corpse.bursts < due {
            for mote in death_motes(corpse.initial.translation, corpse.bursts) {
                commands.spawn((
                    Mesh3d(assets.mesh.clone()),
                    MeshMaterial3d(assets.material.clone()),
                    Transform::from_translation(mote.origin).with_scale(Vec3::ZERO),
                    bevy::light::NotShadowCaster,
                    mote,
                    Name::new("Death Ember"),
                ));
            }
            corpse.bursts += 1;
        }
    }
}

fn animate_death_motes(
    time: Res<Time>,
    mut commands: Commands,
    mut motes: Query<(Entity, &mut DeathMote, &mut Transform)>,
) {
    for (entity, mut mote, mut transform) in &mut motes {
        mote.age += time.delta_secs();
        if mote.age >= mote.life {
            commands.entity(entity).try_despawn();
            continue;
        }
        let (position, scale) = mote_pose(&mote);
        transform.translation = position;
        transform.scale = Vec3::splat(scale);
    }
}

fn corpse_pose(initial: Transform, elapsed: f32) -> Transform {
    let fall = (elapsed / FALL_SECONDS).clamp(0.0, 1.0);
    let fall = 1.0 - (1.0 - fall).powi(3);
    let sink = ((elapsed - FALL_SECONDS - REST_SECONDS) / SINK_SECONDS).clamp(0.0, 1.0);
    let yaw = initial.rotation.to_euler(EulerRot::YXZ).0;
    let prone = Quat::from_rotation_y(yaw)
        * Quat::from_rotation_x(std::f32::consts::FRAC_PI_2)
        * Quat::from_rotation_z(0.10);
    Transform {
        translation: initial.translation + Vec3::Y * (0.20 * fall - 1.1 * sink * sink),
        rotation: initial.rotation.slerp(prone, fall),
        scale: initial.scale,
    }
}

fn animate_fallen_enemies(
    time: Res<Time>,
    mut commands: Commands,
    mut enemies: Query<(Entity, &mut FallenEnemy, &mut Transform)>,
) {
    for (entity, mut corpse, mut transform) in &mut enemies {
        corpse.timer.tick(time.delta());
        *transform = corpse_pose(corpse.initial, corpse.timer.elapsed_secs());
        if corpse.timer.is_finished() {
            commands.entity(entity).try_despawn();
        }
    }
}

fn cleanup_fallen_enemies(
    mut commands: Commands,
    enemies: Query<Entity, With<FallenEnemy>>,
    motes: Query<Entity, With<DeathMote>>,
) {
    for entity in enemies.iter().chain(&motes) {
        commands.entity(entity).try_despawn();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data::EnemyAttackKind;
    use std::time::Duration;

    #[test]
    fn fallen_enemy_keeps_its_model_but_loses_combat_and_decorations() {
        let mut world = World::new();
        let enemy = world
            .spawn((
                Transform::IDENTITY,
                Health {
                    current: 0.0,
                    max: 40.0,
                },
                Enemy {
                    id: "skeleton".into(),
                    display_name: "Skeleton".into(),
                    affixes: vec![],
                    attack_damage: 5.0,
                    attack_kind: EnemyAttackKind::Melee,
                    attack_range: 1.0,
                    attack_timer: Timer::from_seconds(1.0, TimerMode::Once),
                    aggro_range: 8.0,
                    move_speed: 2.0,
                    gold_min: 1,
                    gold_max: 2,
                    xp_reward: 5,
                },
            ))
            .id();
        let model = world.spawn((Transform::IDENTITY, ChildOf(enemy))).id();
        let mesh = world.spawn((Transform::IDENTITY, ChildOf(model))).id();
        let decoration = world.spawn((DeathDecoration, ChildOf(enemy))).id();
        let nested_decoration = world.spawn((DeathDecoration, ChildOf(model))).id();
        begin_enemy_death(&mut world.commands(), enemy, Transform::IDENTITY);
        world.flush();
        assert!(world.get::<Enemy>(enemy).is_none());
        assert!(world.get::<Health>(enemy).is_none());
        assert!(world.get::<FallenEnemy>(enemy).is_some());
        assert_eq!(world.get::<ChildOf>(model).unwrap().parent(), enemy);
        assert_eq!(world.get::<ChildOf>(mesh).unwrap().parent(), model);
        assert!(world.get_entity(decoration).is_err());
        assert!(world.get_entity(nested_decoration).is_err());
    }

    #[test]
    fn corpse_falls_at_original_size_then_sinks_without_expanding() {
        let initial = Transform::from_xyz(4.0, 0.0, 8.0)
            .with_scale(Vec3::splat(1.12))
            .with_rotation(Quat::from_rotation_y(0.7));
        // Slerp at t = 0 is only equal up to float rounding, and the rounding
        // differs between x86_64 and aarch64, so compare rotation with a tolerance.
        let start = corpse_pose(initial, 0.0);
        assert_eq!(start.translation, initial.translation);
        assert_eq!(start.scale, initial.scale);
        assert!(start.rotation.abs_diff_eq(initial.rotation, 1e-6));
        let down = corpse_pose(initial, FALL_SECONDS);
        assert!((down.rotation * Vec3::Y).y.abs() < 0.02);
        assert_eq!(down, corpse_pose(initial, FALL_SECONDS + REST_SECONDS));
        for step in 0..=100 {
            let pose = corpse_pose(initial, CORPSE_SECONDS * step as f32 / 100.0);
            assert_eq!(pose.scale, initial.scale);
            assert_eq!(pose.translation.x, initial.translation.x);
            assert_eq!(pose.translation.z, initial.translation.z);
        }
        assert!(corpse_pose(initial, CORPSE_SECONDS).translation.y < -0.8);
    }

    #[test]
    fn corpse_lifetime_cleans_up_the_retained_model_hierarchy() {
        let mut app = App::new();
        app.init_resource::<Time>()
            .add_systems(Update, animate_fallen_enemies);
        let corpse = app
            .world_mut()
            .spawn((
                Transform::IDENTITY,
                FallenEnemy {
                    timer: Timer::from_seconds(CORPSE_SECONDS, TimerMode::Once),
                    initial: Transform::IDENTITY,
                    bursts: 0,
                },
            ))
            .id();
        let child = app.world_mut().spawn(ChildOf(corpse)).id();
        app.world_mut()
            .resource_mut::<Time>()
            .advance_by(Duration::from_secs_f32(CORPSE_SECONDS + 0.1));
        app.update();
        assert!(app.world().get_entity(corpse).is_err());
        assert!(app.world().get_entity(child).is_err());
    }

    #[test]
    fn death_motes_rise_from_the_body_and_end_at_zero_size() {
        let origin = Vec3::new(3.0, 0.0, -2.0);
        for burst in [0, 1] {
            let motes: Vec<_> = death_motes(origin, burst).collect();
            assert_eq!(
                motes.len() as u32,
                if burst == 0 { FALL_MOTES } else { SINK_MOTES }
            );
            for mut mote in motes {
                assert!(mote.origin.distance(origin) < 1.0);
                mote.age = 0.0;
                assert_eq!(mote_pose(&mote).1, 0.0);
                mote.age = mote.life * 0.4;
                let (mid, size) = mote_pose(&mote);
                assert!(size > 0.0 && size <= mote.size);
                mote.age = mote.life;
                let (end, size) = mote_pose(&mote);
                assert_eq!(size, 0.0);
                assert!(end.y > mid.y && mid.y > mote.origin.y);
            }
        }
        // The same body always sheds the same embers.
        let first: Vec<_> = death_motes(origin, 1).map(|mote| mote.drift).collect();
        let second: Vec<_> = death_motes(origin, 1).map(|mote| mote.drift).collect();
        assert_eq!(first, second);
    }

    #[test]
    fn each_body_releases_one_burst_falling_and_one_sinking() {
        let mut app = App::new();
        app.init_resource::<Time>()
            .insert_resource(DeathMoteAssets {
                mesh: Handle::default(),
                material: Handle::default(),
            })
            .add_systems(
                Update,
                (animate_fallen_enemies, release_death_motes).chain(),
            );
        app.world_mut().spawn((
            Transform::IDENTITY,
            FallenEnemy {
                timer: Timer::from_seconds(CORPSE_SECONDS, TimerMode::Once),
                initial: Transform::IDENTITY,
                bursts: 0,
            },
        ));
        let count = |app: &mut App| {
            app.world_mut()
                .query::<&DeathMote>()
                .iter(app.world())
                .count() as u32
        };
        app.update();
        app.update();
        assert_eq!(count(&mut app), FALL_MOTES);
        app.world_mut()
            .resource_mut::<Time>()
            .advance_by(Duration::from_secs_f32(FALL_SECONDS + REST_SECONDS + 0.05));
        app.update();
        app.update();
        assert_eq!(count(&mut app), FALL_MOTES + SINK_MOTES);
    }
}
