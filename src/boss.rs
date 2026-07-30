use crate::{
    Difficulty, DifficultySettings, GameState,
    assets::GameAssets,
    combat::CombatSet,
    enemy::{BossPhase, BossStagger, Enemy, EnemyEntity, EnemySet, Staggered},
    feedback::{CombatEvent, FloatingCombatTextEvent, ScreenShakeEvent},
    not_paused,
    player::{
        Barrier, Burning, ElixirBuff, Equipment, Evade, Health, Jailed, Player, PlayerHitReact,
        apply_player_damage_with_evade, mitigated_damage, total_armor,
    },
};
use bevy::prelude::*;
use bevy::world_serialization::WorldAsset;

const FIRST_SPECIAL_DELAY_SECS: f32 = 3.8;

pub struct BossAbilitiesPlugin;

impl Plugin for BossAbilitiesPlugin {
    fn build(&self, app: &mut App) {
        app.configure_sets(Update, EnemySet::BossPhase.after(CombatSet::PlayerDamage))
            .add_systems(
                Update,
                (
                    attach_boss_ability_controllers,
                    cast_boss_abilities,
                    resolve_boss_ability_hazards,
                )
                    .chain()
                    .after(EnemySet::BossPhase)
                    .run_if(in_state(GameState::InGame).and_then(not_paused)),
            )
            .add_systems(OnExit(GameState::InGame), cleanup_boss_ability_entities);
    }
}

#[derive(Component, Debug)]
struct BossAbilityController {
    cooldown: Timer,
    sequence: u32,
}

impl Default for BossAbilityController {
    fn default() -> Self {
        Self {
            cooldown: Timer::from_seconds(FIRST_SPECIAL_DELAY_SECS, TimerMode::Once),
            sequence: 0,
        }
    }
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
enum BossAbilityKind {
    AshenGuillotine,
    CrucibleRings,
    KeeperChainCage,
    SoulFissureChoir,
    MatronSoulLance,
    HollowCrescendo,
}

impl BossAbilityKind {
    fn label(self) -> &'static str {
        match self {
            Self::AshenGuillotine => "ASHEN GUILLOTINE — cross the fan",
            Self::CrucibleRings => "CRUCIBLE RINGS — swap inner and outer ground",
            Self::KeeperChainCage => "KEEPER'S CAGE — leave the marked circle",
            Self::SoulFissureChoir => "SOUL FISSURE CHOIR — step between eruptions",
            Self::MatronSoulLance => "SOUL LANCE — cross the narrow lanes",
            Self::HollowCrescendo => "HOLLOW CRESCENDO — BREAK NYXARA TO INTERRUPT",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum BossHazardShape {
    Circle {
        radius: f32,
    },
    Ring {
        inner_radius: f32,
        outer_radius: f32,
    },
    Line {
        half_width: f32,
        length: f32,
    },
    Fan {
        radius: f32,
        half_angle: f32,
    },
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
enum BossHazardStatus {
    None,
    Burning,
    Jailed,
}

#[derive(Component)]
struct BossAbilityHazard {
    owner: Entity,
    shape: BossHazardShape,
    forward: Vec3,
    arm_timer: Timer,
    active_timer: Timer,
    damage: f32,
    status: BossHazardStatus,
    label: &'static str,
    base_scale: Vec3,
    hit: bool,
}

#[derive(Component)]
struct BossAbilityEntity;

struct BossHazardSpec {
    owner: Entity,
    scene: Handle<WorldAsset>,
    position: Vec3,
    forward: Vec3,
    scale: Vec3,
    shape: BossHazardShape,
    arm_secs: f32,
    active_secs: f32,
    damage: f32,
    status: BossHazardStatus,
    label: &'static str,
}

type BossPlayerQuery<'w, 's> = Query<
    'w,
    's,
    (
        Entity,
        &'static Transform,
        &'static mut Health,
        &'static mut Barrier,
        &'static Evade,
        &'static Equipment,
        &'static ElixirBuff,
    ),
    (With<Player>, Without<BossAbilityHazard>),
>;

type NewBossQuery<'w, 's> =
    Query<'w, 's, (Entity, &'static Enemy), (With<BossPhase>, Without<BossAbilityController>)>;

type BossCastQuery<'w, 's> = Query<
    'w,
    's,
    (
        Entity,
        &'static Transform,
        &'static Health,
        &'static Enemy,
        &'static BossPhase,
        Option<&'static BossStagger>,
        Option<&'static Staggered>,
        &'static mut BossAbilityController,
    ),
>;

type BossOwnerStateQuery<'w, 's> = Query<
    'w,
    's,
    (
        &'static Health,
        Option<&'static Staggered>,
        Option<&'static BossStagger>,
    ),
    (With<BossPhase>, Without<Player>),
>;

fn attach_boss_ability_controllers(mut commands: Commands, bosses: NewBossQuery) {
    for (entity, enemy) in &bosses {
        if matches!(enemy.id.as_str(), "keeper" | "hollow_matron") {
            commands
                .entity(entity)
                .try_insert(BossAbilityController::default());
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn cast_boss_abilities(
    time: Res<Time>,
    difficulty: Res<DifficultySettings>,
    assets: Res<GameAssets>,
    mut commands: Commands,
    mut combat_events: MessageWriter<CombatEvent>,
    player: Query<&Transform, (With<Player>, Without<Enemy>)>,
    mut bosses: BossCastQuery,
    hazards: Query<(Entity, &BossAbilityHazard)>,
) {
    let Ok(player_transform) = player.single() else {
        return;
    };

    for (entity, transform, health, enemy, phase, stagger, staggered, mut controller) in &mut bosses
    {
        if health.current <= 0.0 {
            continue;
        }
        if boss_is_interrupted(stagger, staggered) {
            controller.cooldown.reset();
            for (hazard_entity, hazard) in &hazards {
                if hazard.owner == entity {
                    commands.entity(hazard_entity).try_despawn();
                }
            }
            continue;
        }

        controller.cooldown.tick(time.delta());
        if !controller.cooldown.is_finished() {
            continue;
        }

        let ability = select_boss_ability(
            &enemy.id,
            phase.phase_two_started(),
            difficulty.current,
            controller.sequence,
        );
        controller.sequence = controller.sequence.saturating_add(1);
        controller.cooldown = Timer::from_seconds(
            boss_special_cooldown_secs(difficulty.current, phase),
            TimerMode::Once,
        );

        let target = player_transform.translation;
        let forward = flat_direction(target - transform.translation);
        spawn_boss_ability(
            &mut commands,
            &assets,
            BossCastContext {
                owner: entity,
                origin: transform.translation,
                target,
                forward,
                base_damage: enemy.attack_damage,
                difficulty: difficulty.current,
            },
            ability,
        );
        combat_events.write(CombatEvent {
            text: format!("{}: {}", enemy.display_name, ability.label()),
        });
    }
}

struct BossCastContext {
    owner: Entity,
    origin: Vec3,
    target: Vec3,
    forward: Vec3,
    base_damage: f32,
    difficulty: Difficulty,
}

fn boss_is_interrupted(stagger: Option<&BossStagger>, staggered: Option<&Staggered>) -> bool {
    staggered.is_some()
        || stagger
            .is_some_and(|stagger| stagger.meter >= stagger.threshold && stagger.threshold > 0.0)
}

fn select_boss_ability(
    boss_id: &str,
    phase_two: bool,
    difficulty: Difficulty,
    sequence: u32,
) -> BossAbilityKind {
    let unlock_third = phase_two || matches!(difficulty, Difficulty::Hell | Difficulty::Torment);
    let pool_size = if unlock_third { 3 } else { 2 };
    let slot = sequence as usize % pool_size;
    match boss_id {
        "hollow_matron" => [
            BossAbilityKind::SoulFissureChoir,
            BossAbilityKind::MatronSoulLance,
            BossAbilityKind::HollowCrescendo,
        ][slot],
        _ => [
            BossAbilityKind::AshenGuillotine,
            BossAbilityKind::CrucibleRings,
            BossAbilityKind::KeeperChainCage,
        ][slot],
    }
}

fn boss_special_cooldown_secs(difficulty: Difficulty, phase: &BossPhase) -> f32 {
    let difficulty_multiplier = match difficulty {
        Difficulty::Normal => 1.0,
        Difficulty::Nightmare => 0.90,
        Difficulty::Hell => 0.80,
        Difficulty::Torment => 0.70,
    };
    let phase_multiplier = if phase.enrage_started() {
        0.68
    } else if phase.phase_two_started() {
        0.84
    } else {
        1.0
    };
    (7.4_f32 * difficulty_multiplier * phase_multiplier).max(3.4)
}

fn boss_hazard_arm_multiplier(difficulty: Difficulty) -> f32 {
    match difficulty {
        Difficulty::Normal => 1.0,
        Difficulty::Nightmare => 0.96,
        Difficulty::Hell => 0.92,
        Difficulty::Torment => 0.88,
    }
}

fn spawn_boss_ability(
    commands: &mut Commands,
    assets: &GameAssets,
    context: BossCastContext,
    ability: BossAbilityKind,
) {
    let arm = boss_hazard_arm_multiplier(context.difficulty);
    match ability {
        BossAbilityKind::AshenGuillotine => {
            for (index, angle) in [-0.30_f32, 0.0, 0.30].into_iter().enumerate() {
                let forward = rotate_flat(context.forward, angle);
                spawn_boss_hazard(
                    commands,
                    BossHazardSpec {
                        owner: context.owner,
                        scene: assets.keeper_cleaver_wave.clone(),
                        position: context.origin + Vec3::Y * 0.06,
                        forward,
                        scale: Vec3::splat(1.35),
                        shape: BossHazardShape::Fan {
                            radius: 3.5,
                            half_angle: 1.08,
                        },
                        arm_secs: (0.82 + index as f32 * 0.16) * arm,
                        active_secs: 0.26,
                        damage: context.base_damage * 0.72,
                        status: BossHazardStatus::Burning,
                        label: "Ashen Guillotine",
                    },
                );
            }
        }
        BossAbilityKind::CrucibleRings => {
            spawn_boss_hazard(
                commands,
                BossHazardSpec {
                    owner: context.owner,
                    scene: assets.keeper_chain_cage.clone(),
                    position: context.origin + Vec3::Y * 0.05,
                    forward: context.forward,
                    scale: Vec3::splat(1.45),
                    shape: BossHazardShape::Circle { radius: 2.25 },
                    arm_secs: 0.78 * arm,
                    active_secs: 0.24,
                    damage: context.base_damage * 0.78,
                    status: BossHazardStatus::Burning,
                    label: "Crucible Inner Ring",
                },
            );
            spawn_boss_hazard(
                commands,
                BossHazardSpec {
                    owner: context.owner,
                    scene: assets.keeper_chain_cage.clone(),
                    position: context.origin + Vec3::Y * 0.05,
                    forward: context.forward,
                    scale: Vec3::splat(3.02),
                    shape: BossHazardShape::Ring {
                        inner_radius: 2.55,
                        outer_radius: 4.70,
                    },
                    arm_secs: 1.34 * arm,
                    active_secs: 0.28,
                    damage: context.base_damage * 0.86,
                    status: BossHazardStatus::Burning,
                    label: "Crucible Outer Ring",
                },
            );
        }
        BossAbilityKind::KeeperChainCage => {
            spawn_boss_hazard(
                commands,
                BossHazardSpec {
                    owner: context.owner,
                    scene: assets.keeper_chain_cage.clone(),
                    position: context.target + Vec3::Y * 0.05,
                    forward: context.forward,
                    scale: Vec3::splat(1.16),
                    shape: BossHazardShape::Circle { radius: 1.75 },
                    arm_secs: 1.02 * arm,
                    active_secs: 0.34,
                    damage: context.base_damage * 0.58,
                    status: BossHazardStatus::Jailed,
                    label: "Keeper Chain Cage",
                },
            );
        }
        BossAbilityKind::SoulFissureChoir => {
            let perpendicular = Vec3::new(context.forward.z, 0.0, -context.forward.x);
            for (index, offset) in [-2.0_f32, -1.0, 0.0, 1.0, 2.0].into_iter().enumerate() {
                let lane_offset = perpendicular * offset * 1.25;
                let advance = context.forward * (index as f32 * 0.72 - 1.4);
                spawn_boss_hazard(
                    commands,
                    BossHazardSpec {
                        owner: context.owner,
                        scene: assets.soul_fissure.clone(),
                        position: context.target + lane_offset + advance + Vec3::Y * 0.04,
                        forward: context.forward,
                        scale: Vec3::splat(1.10 + index as f32 * 0.04),
                        shape: BossHazardShape::Circle { radius: 0.72 },
                        arm_secs: (0.58 + index as f32 * 0.14) * arm,
                        active_secs: 0.32,
                        damage: context.base_damage * 0.54,
                        status: BossHazardStatus::Jailed,
                        label: "Soul Fissure",
                    },
                );
            }
        }
        BossAbilityKind::MatronSoulLance => {
            for (index, angle) in [-0.18_f32, 0.0, 0.18].into_iter().enumerate() {
                let forward = rotate_flat(context.forward, angle);
                spawn_boss_hazard(
                    commands,
                    BossHazardSpec {
                        owner: context.owner,
                        scene: assets.matron_soul_lance.clone(),
                        position: context.origin + forward * 0.25 + Vec3::Y * 0.30,
                        forward,
                        scale: Vec3::new(1.15, 1.15, 1.22),
                        shape: BossHazardShape::Line {
                            half_width: 0.50,
                            length: 4.75,
                        },
                        arm_secs: (0.70 + index as f32 * 0.11) * arm,
                        active_secs: 0.24,
                        damage: context.base_damage * 0.68,
                        status: BossHazardStatus::None,
                        label: "Matron Soul Lance",
                    },
                );
            }
        }
        BossAbilityKind::HollowCrescendo => {
            spawn_boss_hazard(
                commands,
                BossHazardSpec {
                    owner: context.owner,
                    scene: assets.matron_crescendo.clone(),
                    position: context.origin + Vec3::Y * 0.08,
                    forward: context.forward,
                    scale: Vec3::splat(3.0),
                    shape: BossHazardShape::Circle { radius: 6.1 },
                    arm_secs: 1.82 * arm,
                    active_secs: 0.42,
                    damage: context.base_damage * 1.05,
                    status: BossHazardStatus::Jailed,
                    label: "Hollow Crescendo",
                },
            );
        }
    }
}

fn spawn_boss_hazard(commands: &mut Commands, spec: BossHazardSpec) {
    let yaw = spec.forward.x.atan2(spec.forward.z);
    commands.spawn((
        WorldAssetRoot(spec.scene),
        Transform::from_translation(spec.position)
            .with_rotation(Quat::from_rotation_y(yaw))
            .with_scale(spec.scale * 0.82),
        BossAbilityHazard {
            owner: spec.owner,
            shape: spec.shape,
            forward: flat_direction(spec.forward),
            arm_timer: Timer::from_seconds(spec.arm_secs.max(0.18), TimerMode::Once),
            active_timer: Timer::from_seconds(spec.active_secs.max(0.10), TimerMode::Once),
            damage: spec.damage.max(0.0),
            status: spec.status,
            label: spec.label,
            base_scale: spec.scale,
            hit: false,
        },
        BossAbilityEntity,
        EnemyEntity,
        Name::new(format!("Boss Ability: {}", spec.label)),
    ));
}

#[allow(clippy::too_many_arguments)]
fn resolve_boss_ability_hazards(
    time: Res<Time>,
    mut commands: Commands,
    mut combat_events: MessageWriter<CombatEvent>,
    mut floating_text: MessageWriter<FloatingCombatTextEvent>,
    mut screen_shake: MessageWriter<ScreenShakeEvent>,
    bosses: BossOwnerStateQuery,
    mut hazards: Query<(Entity, &mut Transform, &mut BossAbilityHazard)>,
    mut player: BossPlayerQuery,
) {
    let Ok((
        player_entity,
        player_transform,
        mut player_health,
        mut barrier,
        evade,
        equipment,
        elixir,
    )) = player.single_mut()
    else {
        return;
    };
    let player_armor = total_armor(equipment, elixir);

    for (entity, mut transform, mut hazard) in &mut hazards {
        let owner_can_attack =
            bosses
                .get(hazard.owner)
                .is_ok_and(|(health, staggered, stagger)| {
                    health.current > 0.0 && !boss_is_interrupted(stagger, staggered)
                });
        if !owner_can_attack {
            commands.entity(entity).try_despawn();
            continue;
        }

        if !hazard.arm_timer.is_finished() {
            hazard.arm_timer.tick(time.delta());
            let progress = hazard.arm_timer.fraction();
            let pulse = 0.82 + (progress * std::f32::consts::PI * 5.0).sin().abs() * 0.18;
            transform.scale = hazard.base_scale * pulse;
            if !hazard.arm_timer.is_finished() {
                continue;
            }
            transform.scale = hazard.base_scale * 1.08;
        }

        hazard.active_timer.tick(time.delta());
        if !hazard.hit
            && boss_hazard_contains(
                hazard.shape,
                transform.translation,
                hazard.forward,
                player_transform.translation,
            )
        {
            let damage = mitigated_damage(hazard.damage, player_armor);
            let taken =
                apply_player_damage_with_evade(&mut player_health, &mut barrier, evade, damage);
            let total_taken = taken.absorbed + taken.health_damage;
            if total_taken > 0.0 {
                match hazard.status {
                    BossHazardStatus::Burning => {
                        commands.entity(player_entity).try_insert(Burning {
                            dps: 5.0,
                            timer: Timer::from_seconds(3.2, TimerMode::Once),
                        });
                    }
                    BossHazardStatus::Jailed => {
                        commands.entity(player_entity).try_insert(Jailed {
                            timer: Timer::from_seconds(0.86, TimerMode::Once),
                            speed_multiplier: 0.38,
                        });
                    }
                    BossHazardStatus::None => {}
                }
                commands
                    .entity(player_entity)
                    .try_insert(PlayerHitReact::from_damage(taken));
                combat_events.write(CombatEvent {
                    text: format!("{} hit for {total_taken:.0}", hazard.label),
                });
                floating_text.write(FloatingCombatTextEvent {
                    text: format!("-{total_taken:.0} {}", hazard.label),
                    position: player_transform.translation + Vec3::Y * 1.7,
                    critical: false,
                });
                screen_shake.write(ScreenShakeEvent::new(
                    (0.030 + total_taken * 0.0012).clamp(0.035, 0.085),
                    0.15,
                ));
            }
            hazard.hit = true;
        }

        if hazard.active_timer.is_finished() {
            commands.entity(entity).try_despawn();
        }
    }
}

fn boss_hazard_contains(shape: BossHazardShape, origin: Vec3, forward: Vec3, point: Vec3) -> bool {
    let offset = Vec3::new(point.x - origin.x, 0.0, point.z - origin.z);
    match shape {
        BossHazardShape::Circle { radius } => offset.length() <= radius,
        BossHazardShape::Ring {
            inner_radius,
            outer_radius,
        } => {
            let distance = offset.length();
            distance >= inner_radius && distance <= outer_radius
        }
        BossHazardShape::Line { half_width, length } => {
            let forward = flat_direction(forward);
            let right = Vec3::new(forward.z, 0.0, -forward.x);
            let along = offset.dot(forward);
            (0.0..=length).contains(&along) && offset.dot(right).abs() <= half_width
        }
        BossHazardShape::Fan { radius, half_angle } => {
            let distance = offset.length();
            distance <= radius
                && (distance <= f32::EPSILON
                    || offset.normalize().dot(flat_direction(forward)) >= half_angle.cos())
        }
    }
}

fn flat_direction(direction: Vec3) -> Vec3 {
    let flat = Vec3::new(direction.x, 0.0, direction.z).normalize_or_zero();
    if flat.length_squared() > 0.0 {
        flat
    } else {
        Vec3::Z
    }
}

fn rotate_flat(direction: Vec3, angle: f32) -> Vec3 {
    Quat::from_rotation_y(angle)
        .mul_vec3(flat_direction(direction))
        .normalize_or_zero()
}

fn cleanup_boss_ability_entities(
    mut commands: Commands,
    entities: Query<Entity, With<BossAbilityEntity>>,
) {
    for entity in &entities {
        commands.entity(entity).try_despawn();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn boss_ability_rosters_are_distinct_and_unlock_third_skill() {
        assert_eq!(
            select_boss_ability("keeper", false, Difficulty::Normal, 0),
            BossAbilityKind::AshenGuillotine
        );
        assert_eq!(
            select_boss_ability("keeper", false, Difficulty::Normal, 2),
            BossAbilityKind::AshenGuillotine
        );
        assert_eq!(
            select_boss_ability("keeper", true, Difficulty::Normal, 2),
            BossAbilityKind::KeeperChainCage
        );
        assert_eq!(
            select_boss_ability("hollow_matron", false, Difficulty::Normal, 0),
            BossAbilityKind::SoulFissureChoir
        );
        assert_eq!(
            select_boss_ability("hollow_matron", true, Difficulty::Normal, 2),
            BossAbilityKind::HollowCrescendo
        );
        assert_eq!(
            select_boss_ability("hollow_matron", false, Difficulty::Hell, 2),
            BossAbilityKind::HollowCrescendo
        );
    }

    #[test]
    fn higher_difficulty_shortens_boss_special_cooldowns() {
        let phase_one = BossPhase::new_phase_two(20.0);
        let normal = boss_special_cooldown_secs(Difficulty::Normal, &phase_one);
        let torment = boss_special_cooldown_secs(Difficulty::Torment, &phase_one);
        assert!(torment < normal);
        assert!(torment >= 3.4);
    }

    #[test]
    fn circle_ring_line_and_fan_hit_tests_respect_safe_space() {
        let origin = Vec3::ZERO;
        assert!(boss_hazard_contains(
            BossHazardShape::Circle { radius: 2.0 },
            origin,
            Vec3::Z,
            Vec3::new(1.5, 0.0, 0.0)
        ));
        assert!(!boss_hazard_contains(
            BossHazardShape::Circle { radius: 2.0 },
            origin,
            Vec3::Z,
            Vec3::new(2.5, 0.0, 0.0)
        ));

        let ring = BossHazardShape::Ring {
            inner_radius: 2.0,
            outer_radius: 4.0,
        };
        assert!(!boss_hazard_contains(ring, origin, Vec3::Z, Vec3::X));
        assert!(boss_hazard_contains(ring, origin, Vec3::Z, Vec3::X * 3.0));
        assert!(!boss_hazard_contains(ring, origin, Vec3::Z, Vec3::X * 5.0));

        let line = BossHazardShape::Line {
            half_width: 0.5,
            length: 4.0,
        };
        assert!(boss_hazard_contains(
            line,
            origin,
            Vec3::Z,
            Vec3::new(0.4, 0.0, 3.5)
        ));
        assert!(!boss_hazard_contains(
            line,
            origin,
            Vec3::Z,
            Vec3::new(0.8, 0.0, 3.5)
        ));
        assert!(!boss_hazard_contains(
            line,
            origin,
            Vec3::Z,
            Vec3::new(0.0, 0.0, -1.0)
        ));

        let fan = BossHazardShape::Fan {
            radius: 3.5,
            half_angle: 0.6,
        };
        assert!(boss_hazard_contains(
            fan,
            origin,
            Vec3::Z,
            Vec3::new(1.0, 0.0, 2.4)
        ));
        assert!(!boss_hazard_contains(
            fan,
            origin,
            Vec3::Z,
            Vec3::new(2.4, 0.0, 1.0)
        ));
    }

    #[test]
    fn full_stagger_meter_interrupts_before_deferred_marker_is_visible() {
        let stagger = BossStagger {
            meter: 100.0,
            threshold: 100.0,
        };
        assert!(boss_is_interrupted(Some(&stagger), None));
    }

    #[test]
    fn warning_time_remains_readable_on_torment() {
        assert!(boss_hazard_arm_multiplier(Difficulty::Torment) >= 0.85);
        assert!(
            boss_hazard_arm_multiplier(Difficulty::Torment)
                < boss_hazard_arm_multiplier(Difficulty::Normal)
        );
    }
}
