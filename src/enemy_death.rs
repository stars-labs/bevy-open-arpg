//! Keep the authored enemy visible through its fall instead of replacing it with a primitive.
use crate::{GameState, enemy::Enemy, not_paused, player::Health};
use bevy::prelude::*;

const FALL_SECONDS: f32 = 0.48;
const REST_SECONDS: f32 = 3.0;
const SINK_SECONDS: f32 = 1.2;
const CORPSE_SECONDS: f32 = FALL_SECONDS + REST_SECONDS + SINK_SECONDS;

/// Runtime UI and affix decorations must not remain attached to a fallen body.
#[derive(Component)]
pub(crate) struct DeathDecoration;

#[derive(Component)]
struct FallenEnemy {
    timer: Timer,
    initial: Transform,
}

pub(crate) struct EnemyDeathPlugin;

impl Plugin for EnemyDeathPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            animate_fallen_enemies.run_if(in_state(GameState::InGame).and_then(not_paused)),
        )
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
            });
    });
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

fn cleanup_fallen_enemies(mut commands: Commands, enemies: Query<Entity, With<FallenEnemy>>) {
    for entity in &enemies {
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
        assert_eq!(corpse_pose(initial, 0.0), initial);
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
}
