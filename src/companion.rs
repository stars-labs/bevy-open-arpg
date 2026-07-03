use crate::{
    GameState,
    assets::GameAssets,
    bestiary::{Bestiary, bestiary_damage_multiplier},
    enemy::{Chilled, Enemy, EnemyAffix, EnemyKilled},
    feedback::CombatEvent,
    not_paused,
    player::{Barrier, Health, Player},
};
use bevy::ecs::system::SystemParam;
use bevy::prelude::*;
use serde::{Deserialize, Serialize};

pub const SENTINEL_VETERAN_RANK: u32 = 3;
pub(crate) const SENTINEL_STANCE_KEY: KeyCode = KeyCode::KeyN;
const SENTINEL_COMMAND_COOLDOWN_SECS: f32 = 9.0;

#[derive(Debug, Clone, Copy, Default, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CompanionStance {
    #[default]
    Vanguard,
    Guardian,
    Seeker,
}

impl CompanionStance {
    pub fn next(self) -> Self {
        match self {
            Self::Vanguard => Self::Guardian,
            Self::Guardian => Self::Seeker,
            Self::Seeker => Self::Vanguard,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Vanguard => "Vanguard",
            Self::Guardian => "Guardian",
            Self::Seeker => "Seeker",
        }
    }
}

#[derive(Component, Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CompanionBond {
    pub rank: u32,
    pub xp: u32,
    pub next_rank_xp: u32,
    #[serde(default)]
    pub stance: CompanionStance,
    #[serde(default)]
    pub command_cooldown: f32,
}

impl Default for CompanionBond {
    fn default() -> Self {
        Self {
            rank: 1,
            xp: 0,
            next_rank_xp: companion_next_rank_xp(1),
            stance: CompanionStance::Vanguard,
            command_cooldown: 0.0,
        }
    }
}

#[derive(Component)]
struct CompanionEntity;

#[derive(Component)]
struct Companion {
    attack_timer: Timer,
    range: f32,
    follow_distance: f32,
    move_speed: f32,
}

#[derive(Component)]
struct SentinelCommandVfx {
    timer: Timer,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SentinelCommandTuning {
    pub damage: f32,
    pub radius: f32,
    pub barrier: f32,
}

pub struct CompanionPlugin;

impl Plugin for CompanionPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(GameState::InGame), spawn_companion)
            .add_systems(
                Update,
                (
                    cycle_companion_stance,
                    activate_sentinel_command,
                    companion_behavior,
                    award_companion_xp,
                    tick_sentinel_command_vfx,
                )
                    .run_if(in_state(GameState::InGame).and_then(not_paused)),
            )
            .add_systems(
                OnExit(GameState::InGame),
                (despawn_companion, despawn_sentinel_command_vfx),
            );
    }
}

pub fn companion_next_rank_xp(rank: u32) -> u32 {
    40 + rank.saturating_mul(35)
}

pub fn companion_damage(rank: u32, stance: CompanionStance) -> f32 {
    let base_damage = 5.0 + rank.max(1) as f32 * 2.5;
    match stance {
        CompanionStance::Vanguard => base_damage * 1.25,
        CompanionStance::Guardian => base_damage * 0.85,
        CompanionStance::Seeker => base_damage,
    }
}

pub fn companion_xp_reward(enemy: &EnemyKilled, stance: CompanionStance) -> u32 {
    let reward = (enemy.xp_reward / 3).max(5);
    match stance {
        CompanionStance::Seeker => ((reward as f32) * 1.25).round() as u32,
        CompanionStance::Vanguard | CompanionStance::Guardian => reward,
    }
}

pub fn companion_range(base_range: f32, stance: CompanionStance) -> f32 {
    match stance {
        CompanionStance::Seeker => base_range + 1.8,
        CompanionStance::Vanguard | CompanionStance::Guardian => base_range,
    }
}

pub fn companion_summary(bond: &CompanionBond) -> String {
    let command_status = if bond.command_cooldown <= 0.0 {
        "H ready".to_string()
    } else {
        format!("H {:.0}s", bond.command_cooldown.ceil())
    };
    format!(
        "Sentinel {} R{} {}/{} XP {}",
        bond.stance.label(),
        bond.rank,
        bond.xp,
        bond.next_rank_xp,
        command_status
    )
}

pub fn sentinel_command_tuning(rank: u32, stance: CompanionStance) -> SentinelCommandTuning {
    let rank = rank.max(1) as f32;
    match stance {
        CompanionStance::Vanguard => SentinelCommandTuning {
            damage: 18.0 + rank * 7.0,
            radius: 3.4,
            barrier: 0.0,
        },
        CompanionStance::Guardian => SentinelCommandTuning {
            damage: 8.0 + rank * 3.0,
            radius: 3.2,
            barrier: 18.0 + rank * 6.0,
        },
        CompanionStance::Seeker => SentinelCommandTuning {
            damage: 12.0 + rank * 4.0,
            radius: 6.8,
            barrier: 0.0,
        },
    }
}

pub fn grant_companion_xp(bond: &mut CompanionBond, amount: u32) -> u32 {
    bond.xp = bond.xp.saturating_add(amount);
    let mut ranks_gained: u32 = 0;
    while bond.xp >= bond.next_rank_xp {
        bond.xp -= bond.next_rank_xp;
        bond.rank = bond.rank.saturating_add(1);
        bond.next_rank_xp = companion_next_rank_xp(bond.rank);
        ranks_gained = ranks_gained.saturating_add(1);
    }
    ranks_gained
}

type CompanionPlayerQuery<'w, 's> = Query<
    'w,
    's,
    (
        &'static Transform,
        &'static CompanionBond,
        &'static mut Barrier,
    ),
    (With<Player>, Without<CompanionEntity>, Without<Enemy>),
>;
type CompanionEntityQuery<'w, 's> = Query<
    'w,
    's,
    (&'static mut Transform, &'static mut Companion),
    (With<CompanionEntity>, Without<Player>, Without<Enemy>),
>;
type CompanionEnemyQuery<'w, 's> = Query<
    'w,
    's,
    (
        &'static Transform,
        &'static Enemy,
        &'static mut Health,
        Option<&'static Chilled>,
    ),
    (With<Enemy>, Without<Player>, Without<CompanionEntity>),
>;

fn cycle_companion_stance(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut combat_events: MessageWriter<CombatEvent>,
    mut player: Query<&mut CompanionBond, With<Player>>,
) {
    if !keyboard.just_pressed(SENTINEL_STANCE_KEY) {
        return;
    }
    let Ok(mut bond) = player.single_mut() else {
        return;
    };
    bond.stance = bond.stance.next();
    combat_events.write(CombatEvent {
        text: format!("Reliquary Sentinel stance: {}", bond.stance.label()),
    });
}

type SentinelCommandPlayerQuery<'w, 's> = Query<
    'w,
    's,
    (
        &'static Transform,
        &'static mut CompanionBond,
        &'static mut Barrier,
    ),
    (With<Player>, Without<CompanionEntity>, Without<Enemy>),
>;
type SentinelCommandCompanionQuery<'w, 's> =
    Query<'w, 's, &'static Transform, (With<CompanionEntity>, Without<Player>, Without<Enemy>)>;
type SentinelCommandEnemyQuery<'w, 's> = Query<
    'w,
    's,
    (
        &'static Transform,
        &'static Enemy,
        &'static mut Health,
        Option<&'static Chilled>,
    ),
    (With<Enemy>, Without<Player>, Without<CompanionEntity>),
>;

#[derive(SystemParam)]
struct SentinelCommandParams<'w, 's> {
    time: Res<'w, Time>,
    keyboard: Res<'w, ButtonInput<KeyCode>>,
    bestiary: Res<'w, Bestiary>,
    assets: Res<'w, GameAssets>,
    commands: Commands<'w, 's>,
    combat_events: MessageWriter<'w, CombatEvent>,
}

fn activate_sentinel_command(
    mut params: SentinelCommandParams,
    mut player: SentinelCommandPlayerQuery,
    companion: SentinelCommandCompanionQuery,
    mut enemies: SentinelCommandEnemyQuery,
) {
    let Ok((player_transform, mut bond, mut barrier)) = player.single_mut() else {
        return;
    };

    bond.command_cooldown = (bond.command_cooldown - params.time.delta_secs()).max(0.0);
    if !params.keyboard.just_pressed(KeyCode::KeyH) {
        return;
    }
    if bond.command_cooldown > 0.0 {
        params.combat_events.write(CombatEvent {
            text: format!(
                "Sentinel command cooling {:.0}s",
                bond.command_cooldown.ceil()
            ),
        });
        return;
    }

    let origin = companion
        .single()
        .map(|transform| transform.translation)
        .unwrap_or(player_transform.translation);
    let tuning = sentinel_command_tuning(bond.rank, bond.stance);
    let mut hits = 0_u32;
    let mut highest_damage: f32 = 0.0;

    for (transform, enemy, mut health, chilled) in enemies.iter_mut() {
        if health.current <= 0.0 || transform.translation.distance(origin) > tuning.radius {
            continue;
        }
        let damage = companion_final_damage(tuning.damage, enemy, chilled, &params.bestiary);
        health.current = (health.current - damage).max(0.0);
        hits = hits.saturating_add(1);
        highest_damage = highest_damage.max(damage);
    }

    let barrier_gained = if tuning.barrier > 0.0 {
        barrier.grant(tuning.barrier)
    } else {
        0.0
    };
    if hits == 0 && barrier_gained <= 0.0 {
        params.combat_events.write(CombatEvent {
            text: "Sentinel command needs a nearby target".to_string(),
        });
        return;
    }

    bond.command_cooldown = SENTINEL_COMMAND_COOLDOWN_SECS;
    spawn_sentinel_command_vfx(&mut params.commands, &params.assets, origin, bond.stance);
    params.combat_events.write(CombatEvent {
        text: sentinel_command_event_text(bond.stance, hits, highest_damage, barrier_gained),
    });
}

fn sentinel_command_event_text(
    stance: CompanionStance,
    hits: u32,
    damage: f32,
    barrier_gained: f32,
) -> String {
    match stance {
        CompanionStance::Vanguard => {
            format!("Sentinel Vanguard command hit {hits} for {damage:.0}")
        }
        CompanionStance::Guardian => {
            format!("Sentinel Guardian command hit {hits} and granted {barrier_gained:.0} barrier")
        }
        CompanionStance::Seeker => format!("Sentinel Seeker command swept {hits} targets"),
    }
}

fn spawn_sentinel_command_vfx(
    commands: &mut Commands,
    assets: &GameAssets,
    origin: Vec3,
    stance: CompanionStance,
) {
    let scene = match stance {
        CompanionStance::Vanguard => assets.ember_impact.clone(),
        CompanionStance::Guardian => assets.holy_impact.clone(),
        CompanionStance::Seeker => assets.arcane_impact.clone(),
    };
    commands.spawn((
        WorldAssetRoot(scene),
        Transform::from_translation(origin + Vec3::Y * 0.18).with_scale(Vec3::splat(1.35)),
        SentinelCommandVfx {
            timer: Timer::from_seconds(0.42, TimerMode::Once),
        },
        Name::new("Sentinel Command VFX"),
    ));
}

fn spawn_companion(
    mut commands: Commands,
    assets: Res<GameAssets>,
    player: Query<&Transform, With<Player>>,
) {
    let Ok(player_transform) = player.single() else {
        return;
    };
    commands.spawn((
        WorldAssetRoot(assets.cultist.clone()),
        Transform::from_translation(player_transform.translation + Vec3::new(-1.2, 0.0, 1.1))
            .with_scale(Vec3::splat(0.72)),
        Companion {
            attack_timer: Timer::from_seconds(0.78, TimerMode::Repeating),
            range: 5.8,
            follow_distance: 1.65,
            move_speed: 5.6,
        },
        CompanionEntity,
        Name::new("Reliquary Sentinel"),
    ));
}

fn companion_behavior(
    time: Res<Time>,
    bestiary: Res<Bestiary>,
    mut combat_events: MessageWriter<CombatEvent>,
    mut player: CompanionPlayerQuery,
    mut companion: CompanionEntityQuery,
    mut enemies: CompanionEnemyQuery,
) {
    let Ok((player_transform, bond, mut barrier)) = player.single_mut() else {
        return;
    };
    let Ok((mut companion_transform, mut companion)) = companion.single_mut() else {
        return;
    };

    let offset = Vec3::new(-1.35, 0.0, 1.15);
    let desired_position = player_transform.translation + offset;
    let to_desired = desired_position - companion_transform.translation;
    let flat_to_desired = Vec3::new(to_desired.x, 0.0, to_desired.z);
    if flat_to_desired.length() > companion.follow_distance {
        companion_transform.translation +=
            flat_to_desired.normalize_or_zero() * companion.move_speed * time.delta_secs();
    }

    let Some((target_position, enemy_name, mut target_health, chilled)) = nearest_companion_target(
        companion_transform.translation,
        companion_range(companion.range, bond.stance),
        &mut enemies,
    ) else {
        companion.attack_timer.tick(time.delta());
        return;
    };

    let to_target = target_position - companion_transform.translation;
    let flat_to_target = Vec3::new(to_target.x, 0.0, to_target.z);
    if flat_to_target.length_squared() > 0.0 {
        companion_transform.rotation =
            Quat::from_rotation_y(flat_to_target.normalize_or_zero().x.atan2(flat_to_target.z));
    }

    companion.attack_timer.tick(time.delta());
    if !companion.attack_timer.just_finished() {
        return;
    }

    let damage = companion_final_damage(
        companion_damage(bond.rank, bond.stance),
        enemy_name.1,
        chilled,
        &bestiary,
    );
    target_health.current = (target_health.current - damage).max(0.0);
    if bond.stance == CompanionStance::Guardian {
        let gained = barrier.grant(3.0 + bond.rank as f32 * 1.5);
        if gained > 0.0 {
            combat_events.write(CombatEvent {
                text: format!("Reliquary Sentinel granted {gained:.0} barrier"),
            });
        }
    }
    combat_events.write(CombatEvent {
        text: format!("Reliquary Sentinel hit {} for {damage:.0}", enemy_name.0),
    });
}

type CompanionTarget<'a> = (
    Vec3,
    (&'a str, &'a Enemy),
    Mut<'a, Health>,
    Option<&'a Chilled>,
);

fn nearest_companion_target<'a>(
    origin: Vec3,
    range: f32,
    enemies: &'a mut CompanionEnemyQuery,
) -> Option<CompanionTarget<'a>> {
    enemies
        .iter_mut()
        .filter(|(_, _, health, _)| health.current > 0.0)
        .filter_map(|(transform, enemy, health, chilled)| {
            let distance = transform.translation.distance(origin);
            (distance <= range).then_some((distance, transform.translation, enemy, health, chilled))
        })
        .min_by(|a, b| a.0.total_cmp(&b.0))
        .map(|(_, position, enemy, health, chilled)| {
            (
                position,
                (enemy.display_name.as_str(), enemy),
                health,
                chilled,
            )
        })
}

fn companion_final_damage(
    base_damage: f32,
    enemy: &Enemy,
    chilled: Option<&Chilled>,
    bestiary: &Bestiary,
) -> f32 {
    let mut damage = base_damage * bestiary_damage_multiplier(bestiary, &enemy.id);
    if enemy.affixes.contains(&EnemyAffix::Shielded) && chilled.is_none() {
        damage *= 0.62;
    }
    damage
}

fn award_companion_xp(
    mut kills: MessageReader<EnemyKilled>,
    mut combat_events: MessageWriter<CombatEvent>,
    mut player: Query<&mut CompanionBond, With<Player>>,
) {
    let Ok(mut bond) = player.single_mut() else {
        return;
    };
    for kill in kills.read() {
        let gained = companion_xp_reward(kill, bond.stance);
        let ranks = grant_companion_xp(&mut bond, gained);
        if ranks > 0 {
            combat_events.write(CombatEvent {
                text: format!("Reliquary Sentinel reached rank {}", bond.rank),
            });
        }
    }
}

fn despawn_companion(
    mut commands: Commands,
    query: Query<Entity, (With<CompanionEntity>, Without<ChildOf>)>,
) {
    for entity in &query {
        if let Ok(mut entity_commands) = commands.get_entity(entity) {
            entity_commands.try_despawn();
        }
    }
}

fn tick_sentinel_command_vfx(
    time: Res<Time>,
    mut commands: Commands,
    mut query: Query<(Entity, &mut SentinelCommandVfx)>,
) {
    for (entity, mut vfx) in &mut query {
        vfx.timer.tick(time.delta());
        if vfx.timer.is_finished()
            && let Ok(mut entity_commands) = commands.get_entity(entity)
        {
            entity_commands.try_despawn();
        }
    }
}

fn despawn_sentinel_command_vfx(
    mut commands: Commands,
    query: Query<Entity, (With<SentinelCommandVfx>, Without<ChildOf>)>,
) {
    for entity in &query {
        if let Ok(mut entity_commands) = commands.get_entity(entity) {
            entity_commands.try_despawn();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn companion_rank_damage_scales_from_rank_one() {
        assert_eq!(
            companion_damage(0, CompanionStance::Vanguard),
            companion_damage(1, CompanionStance::Vanguard)
        );
        assert!(
            companion_damage(4, CompanionStance::Vanguard)
                > companion_damage(1, CompanionStance::Vanguard)
        );
        assert!(
            companion_damage(2, CompanionStance::Vanguard)
                > companion_damage(2, CompanionStance::Guardian)
        );
    }

    #[test]
    fn companion_stances_cycle_and_modify_growth_or_range() {
        let enemy = EnemyKilled {
            enemy_id: "skeleton".to_string(),
            display_name: "Ashbone Guard".to_string(),
            position: Vec3::ZERO,
            xp_reward: 30,
            affix_count: 0,
            affix_mask: 0,
            cursed_ambusher: false,
            champion_pack_member: false,
        };

        assert_eq!(CompanionStance::Vanguard.next(), CompanionStance::Guardian);
        assert_eq!(CompanionStance::Guardian.next(), CompanionStance::Seeker);
        assert!(companion_range(5.8, CompanionStance::Seeker) > 5.8);
        assert!(
            companion_xp_reward(&enemy, CompanionStance::Seeker)
                > companion_xp_reward(&enemy, CompanionStance::Vanguard)
        );
    }

    #[test]
    fn companion_xp_can_gain_multiple_ranks() {
        let mut bond = CompanionBond::default();
        let ranks = grant_companion_xp(&mut bond, 200);

        assert_eq!(ranks, 2);
        assert_eq!(bond.rank, 3);
        assert!(bond.xp < bond.next_rank_xp);
    }

    #[test]
    fn companion_summary_reports_rank_and_progress() {
        let bond = CompanionBond {
            rank: 3,
            xp: 12,
            next_rank_xp: 145,
            stance: CompanionStance::Guardian,
            command_cooldown: 0.0,
        };

        assert_eq!(
            companion_summary(&bond),
            "Sentinel Guardian R3 12/145 XP H ready"
        );
    }

    #[test]
    fn sentinel_command_tuning_gives_each_stance_a_distinct_role() {
        let vanguard = sentinel_command_tuning(3, CompanionStance::Vanguard);
        let guardian = sentinel_command_tuning(3, CompanionStance::Guardian);
        let seeker = sentinel_command_tuning(3, CompanionStance::Seeker);

        assert!(vanguard.damage > guardian.damage);
        assert!(guardian.barrier > 0.0);
        assert!(seeker.radius > vanguard.radius);
        assert!(companion_summary(&CompanionBond::default()).contains("H ready"));
    }

    #[test]
    fn companion_summary_reports_sentinel_command_cooldown() {
        let bond = CompanionBond {
            command_cooldown: 3.2,
            ..CompanionBond::default()
        };

        assert!(companion_summary(&bond).contains("H 4s"));
    }
}
