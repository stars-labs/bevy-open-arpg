mod vfx;
use vfx::{GroundMark, ground_mark, projectile_bolt};

use crate::{
    Difficulty, DifficultySettings, GameState, RunStats,
    assets::GameAssets,
    chapter::{
        ActiveChapter, ChapterId, ChapterPhase, ChapterProgress, InteractableKind,
        InteractableUsed, SpawnBoss, SpawnSanctumWave,
    },
    combat::summons::{Minion, SummonVitality},
    data::{EnemyAttackKind, EnemyCatalog, EnemyTuning},
    dungeon::{chapter_world_position, clamp_world_translation},
    feedback::{CombatEvent, FloatingCombatTextEvent, ScreenShakeEvent},
    not_paused,
    obelisk::SpawnObeliskWave,
    ordeal::ChapterModifier,
    player::{
        Barrier, Burning, DamageTaken, ElixirBuff, Equipment, Evade, Health, Jailed, Player,
        PlayerHitReact, PlayerHitReactKind, apply_player_damage_with_evade, mitigated_damage,
        total_armor,
    },
    procedural_world::{CHUNK_SIZE, CHUNK_UNLOAD_RADIUS},
    rift::SpawnRiftWave,
};
use bevy::ecs::system::SystemParam;
use bevy::prelude::*;
use std::collections::{HashMap, HashSet, VecDeque};

#[derive(Component)]
pub struct Enemy {
    pub id: String,
    pub display_name: String,
    pub affixes: Vec<EnemyAffix>,
    pub attack_damage: f32,
    pub attack_kind: EnemyAttackKind,
    pub attack_range: f32,
    pub attack_timer: Timer,
    pub aggro_range: f32,
    pub move_speed: f32,
    pub gold_min: u32,
    pub gold_max: u32,
    pub xp_reward: u32,
}

pub fn enemy_cursor_pick_radius(enemy: &Enemy) -> f32 {
    if matches!(enemy.id.as_str(), "keeper" | "hollow_matron") {
        1.55
    } else if matches!(enemy.id.as_str(), "seal_warden" | "bell_warden") {
        1.18
    } else if enemy.id == "nemesis" || !enemy.affixes.is_empty() {
        1.05
    } else {
        0.76
    }
}

fn chapter_enemy_id(chapter: ChapterId, base_id: &str) -> &str {
    if chapter == ChapterId::AshenReliquary {
        return base_id;
    }
    match base_id {
        "skeleton" => "hollow_thrall",
        "bone_stalker" => "hollow_stalker",
        "cultist" => "choir_acolyte",
        "ashen_marksman" => "choir_marksman",
        "reliquary_brute" => "hollow_reaver",
        "seal_warden" => "bell_warden",
        "keeper" => "hollow_matron",
        _ => base_id,
    }
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum EnemyCombatRole {
    Vanguard,
    Skirmisher,
    Caster,
    Marksman,
    Juggernaut,
    Warden,
    Treasure,
    Nemesis,
    Boss,
}

pub fn enemy_combat_role(id: &str) -> EnemyCombatRole {
    match id {
        "bone_stalker" | "hollow_stalker" => EnemyCombatRole::Skirmisher,
        "cultist" | "choir_acolyte" => EnemyCombatRole::Caster,
        "ashen_marksman" | "choir_marksman" => EnemyCombatRole::Marksman,
        "reliquary_brute" | "hollow_reaver" => EnemyCombatRole::Juggernaut,
        "seal_warden" | "bell_warden" => EnemyCombatRole::Warden,
        "treasure_imp" => EnemyCombatRole::Treasure,
        "nemesis" => EnemyCombatRole::Nemesis,
        "keeper" | "hollow_matron" => EnemyCombatRole::Boss,
        _ => EnemyCombatRole::Vanguard,
    }
}

fn enemy_scene(assets: &GameAssets, model: &str) -> Handle<WorldAsset> {
    debug_assert!(
        enemy_model_is_supported(model),
        "enemy catalog references unmapped model {model}"
    );
    match model {
        "ashbone_guard.glb" => assets.ashbone_guard.clone(),
        "cultist.glb" => assets.cultist.clone(),
        "butcher.glb" => assets.butcher.clone(),
        "bone_stalker.glb" => assets.bone_stalker.clone(),
        "ashen_marksman.glb" => assets.ashen_marksman.clone(),
        "treasure_imp.glb" => assets.treasure_imp.clone(),
        "seal_warden.glb" => assets.seal_warden.clone(),
        "keeper.glb" => assets.keeper.clone(),
        "nemesis.glb" => assets.nemesis.clone(),
        "hollow_thrall.glb" => assets.hollow_thrall.clone(),
        "hollow_stalker.glb" => assets.hollow_stalker.clone(),
        "choir_acolyte.glb" => assets.choir_acolyte.clone(),
        "choir_marksman.glb" => assets.choir_marksman.clone(),
        "hollow_reaver.glb" => assets.hollow_reaver.clone(),
        "bell_warden.glb" => assets.bell_warden.clone(),
        "hollow_matron.glb" => assets.hollow_matron.clone(),
        _ => assets.skeleton.clone(),
    }
}

fn enemy_model_is_supported(model: &str) -> bool {
    matches!(
        model,
        "skeleton.glb"
            | "ashbone_guard.glb"
            | "cultist.glb"
            | "butcher.glb"
            | "bone_stalker.glb"
            | "ashen_marksman.glb"
            | "treasure_imp.glb"
            | "seal_warden.glb"
            | "keeper.glb"
            | "nemesis.glb"
            | "hollow_thrall.glb"
            | "hollow_stalker.glb"
            | "choir_acolyte.glb"
            | "choir_marksman.glb"
            | "hollow_reaver.glb"
            | "bell_warden.glb"
            | "hollow_matron.glb"
    )
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum EnemyAffix {
    Frenzied,
    Vampiric,
    Molten,
    Shielded,
    Arcane,
    Jailer,
    Frozen,
    Desecrator,
    Reflective,
}

impl EnemyAffix {
    pub fn label(self) -> &'static str {
        match self {
            Self::Frenzied => "Frenzied",
            Self::Vampiric => "Vampiric",
            Self::Molten => "Molten",
            Self::Shielded => "Shielded",
            Self::Arcane => "Arcane",
            Self::Jailer => "Jailer",
            Self::Frozen => "Frozen",
            Self::Desecrator => "Desecrator",
            Self::Reflective => "Reflective",
        }
    }

    pub fn mask(self) -> u16 {
        1 << match self {
            Self::Frenzied => 0,
            Self::Vampiric => 1,
            Self::Molten => 2,
            Self::Shielded => 3,
            Self::Arcane => 4,
            Self::Jailer => 5,
            Self::Frozen => 6,
            Self::Desecrator => 7,
            Self::Reflective => 8,
        }
    }
}

#[derive(Component)]
pub struct EnemyEntity;

#[derive(Component, Debug)]
struct EnemySkillCycle {
    attacks: u32,
    signature_interval: u32,
}

impl EnemySkillCycle {
    fn new(difficulty: Difficulty) -> Self {
        Self {
            attacks: 0,
            signature_interval: difficulty.enemy_signature_interval().max(1),
        }
    }

    fn advance(&mut self) -> bool {
        self.attacks = self.attacks.saturating_add(1);
        self.attacks.is_multiple_of(self.signature_interval)
    }
}

/// Marks an enemy that belongs to a streamed wilderness encounter.
#[derive(Component, Debug, Clone, Copy, Eq, PartialEq)]
pub struct WildernessEnemy {
    pub chapter: ChapterId,
    pub chunk_x: i64,
    pub chunk_z: i64,
}

#[derive(Message, Debug, Clone, Copy)]
pub struct SpawnWildernessEncounter {
    pub chapter: ChapterId,
    pub chunk_x: i64,
    pub chunk_z: i64,
    pub center: Vec3,
    pub seed: u64,
}

#[derive(Resource, Default)]
struct PendingWildernessEncounters {
    events: VecDeque<SpawnWildernessEncounter>,
}

#[derive(Resource)]
struct EnemyRuntimeVfx {
    flat_box: Handle<Mesh>,
    flat_disc: Handle<Mesh>,
    impact_spark: Handle<Mesh>,
    projectile_bolt: Handle<Mesh>,
    warning_materials: [Handle<StandardMaterial>; 3],
    impact_materials: [Handle<StandardMaterial>; 3],
    projectile_materials: HashMap<&'static str, Handle<StandardMaterial>>,
}

#[derive(Component)]
struct EnemyAnimation {
    walk_phase: f32,
    facing_yaw: f32,
    attack_timer: Timer,
    attack_duration_secs: f32,
    base_scale: f32,
    moving: bool,
    warning_progress: f32,
    hit_react_power: f32,
}

impl EnemyAnimation {
    fn new(base_scale: f32) -> Self {
        let mut attack_timer = Timer::from_seconds(0.01, TimerMode::Once);
        attack_timer.set_elapsed(attack_timer.duration());
        Self {
            walk_phase: 0.0,
            facing_yaw: 0.0,
            attack_timer,
            attack_duration_secs: 0.01,
            base_scale,
            moving: false,
            warning_progress: 0.0,
            hit_react_power: 0.0,
        }
    }

    fn trigger_attack(&mut self, duration_secs: f32) {
        self.attack_duration_secs = duration_secs.max(0.08);
        self.attack_timer = Timer::from_seconds(self.attack_duration_secs, TimerMode::Once);
        self.attack_timer.reset();
    }
}

#[derive(Component, Debug, Clone, Copy, Eq, PartialEq)]
enum EnemyLimbKind {
    AttackHand,
    OffHand,
    LeftLeg,
    RightLeg,
}

impl EnemyLimbKind {
    fn from_node_name(name: &str) -> Option<Self> {
        match name {
            "enemy_right_arm" => Some(Self::AttackHand),
            "enemy_left_arm" => Some(Self::OffHand),
            "enemy_left_leg" => Some(Self::LeftLeg),
            "enemy_right_leg" => Some(Self::RightLeg),
            _ => None,
        }
    }

    fn side(self) -> f32 {
        match self {
            Self::AttackHand | Self::RightLeg => 1.0,
            Self::OffHand | Self::LeftLeg => -1.0,
        }
    }
}

#[derive(Component)]
struct EnemyLimb {
    kind: EnemyLimbKind,
    owner: Entity,
    rest: Transform,
}

#[derive(Component)]
pub struct EnemyHitReact {
    timer: Timer,
    duration_secs: f32,
    recoil_direction: Vec3,
    recoil_distance: f32,
}

impl EnemyHitReact {
    pub fn new(duration_secs: f32) -> Self {
        let duration_secs = duration_secs.max(0.06);
        Self {
            timer: Timer::from_seconds(duration_secs, TimerMode::Once),
            duration_secs,
            recoil_direction: Vec3::ZERO,
            recoil_distance: 0.0,
        }
    }

    pub fn with_recoil_from(mut self, source: Vec3, target: Vec3, distance: f32) -> Self {
        self.recoil_direction = flat_recoil_direction(source, target);
        self.recoil_distance = distance.max(0.0);
        self
    }
}

fn flat_recoil_direction(source: Vec3, target: Vec3) -> Vec3 {
    let delta = Vec3::new(target.x - source.x, 0.0, target.z - source.z);
    if delta.length_squared() > 0.0001 {
        delta.normalize()
    } else {
        Vec3::Z
    }
}

fn enemy_hit_recoil_step(hit_react: &EnemyHitReact, delta_secs: f32) -> Vec3 {
    if hit_react.recoil_distance <= 0.0 || hit_react.duration_secs <= 0.0 {
        return Vec3::ZERO;
    }
    let start = (hit_react.timer.elapsed_secs() / hit_react.duration_secs).clamp(0.0, 1.0);
    let end =
        ((hit_react.timer.elapsed_secs() + delta_secs) / hit_react.duration_secs).clamp(0.0, 1.0);
    // Integrate the decaying impulse over this frame. The configured distance is
    // the total displacement, independent of frame rate or hit-stop slowdown.
    let travel = (1.0 - start).powf(2.35) - (1.0 - end).powf(2.35);
    hit_react.recoil_direction * hit_react.recoil_distance * travel
}

fn remove_expired_hit_react(world: &mut World, target: Entity) {
    let Ok(mut entity) = world.get_entity_mut(target) else {
        return;
    };
    // Another strike may have replaced the reaction since cleanup was queued.
    if entity
        .get::<EnemyHitReact>()
        .is_some_and(|reaction| reaction.timer.is_finished())
    {
        entity.remove::<EnemyHitReact>();
    }
}

#[derive(Component)]
pub struct EnemyTargetFocus {
    timer: Timer,
    duration_secs: f32,
    critical: bool,
}

impl EnemyTargetFocus {
    pub fn new(critical: bool) -> Self {
        let duration_secs = enemy_target_focus_duration_secs(critical);
        Self {
            timer: Timer::from_seconds(duration_secs, TimerMode::Once),
            duration_secs,
            critical,
        }
    }

    pub fn tick(&mut self, delta: std::time::Duration) {
        self.timer.tick(delta);
    }

    pub fn is_finished(&self) -> bool {
        self.timer.is_finished()
    }

    pub fn intensity(&self) -> f32 {
        (1.0 - self.timer.elapsed_secs() / self.duration_secs.max(0.001)).clamp(0.0, 1.0)
    }

    pub fn critical(&self) -> bool {
        self.critical
    }
}

fn enemy_target_focus_duration_secs(critical: bool) -> f32 {
    if critical { 1.35 } else { 0.92 }
}

#[derive(Component)]
struct EnemyAttackWarning {
    timer: Timer,
    duration_secs: f32,
    kind: EnemyAttackKind,
}

impl EnemyAttackWarning {
    fn new(kind: EnemyAttackKind, duration_secs: f32) -> Self {
        let duration_secs = duration_secs.max(0.12);
        Self {
            timer: Timer::from_seconds(duration_secs, TimerMode::Once),
            duration_secs,
            kind,
        }
    }

    fn progress(&self) -> f32 {
        (self.timer.elapsed_secs() / self.duration_secs).clamp(0.0, 1.0)
    }
}

#[derive(Component)]
struct EnemyAttackTelegraph {
    timer: Timer,
    duration_secs: f32,
    base_scale: Vec3,
}

struct EnemyAttackWarningSpec<'a> {
    origin: Vec3,
    direction: Vec3,
    kind: &'a EnemyAttackKind,
    attack_range: f32,
    duration_secs: f32,
}

#[derive(Component)]
pub struct CursedAmbusher;

#[derive(Component)]
pub struct ChampionPackMember;

pub const CURSED_AMBUSH_KILLS: u32 = 3;
pub const AFFIX_HUNTER_KILLS: u32 = 3;

#[derive(Component)]
pub struct BossPhase {
    phase_two_started: bool,
    enrage_started: bool,
    enrage_timer: Timer,
}

impl BossPhase {
    #[cfg_attr(not(test), allow(dead_code))]
    pub fn new_phase_two(enrage_remaining_secs: f32) -> Self {
        Self {
            phase_two_started: true,
            enrage_started: false,
            enrage_timer: Timer::from_seconds(enrage_remaining_secs.max(0.0), TimerMode::Once),
        }
    }

    #[cfg_attr(not(test), allow(dead_code))]
    pub fn new_enraged() -> Self {
        Self {
            phase_two_started: true,
            enrage_started: true,
            enrage_timer: Timer::from_seconds(0.0, TimerMode::Once),
        }
    }

    pub fn phase_two_started(&self) -> bool {
        self.phase_two_started
    }

    pub fn enrage_started(&self) -> bool {
        self.enrage_started
    }

    pub fn enrage_remaining_secs(&self) -> f32 {
        self.enrage_timer.remaining_secs()
    }
}

#[derive(Component)]
pub struct BossStagger {
    pub meter: f32,
    pub threshold: f32,
}

impl Default for BossStagger {
    fn default() -> Self {
        Self {
            meter: 0.0,
            threshold: BOSS_STAGGER_THRESHOLD,
        }
    }
}

#[derive(Component)]
pub struct Staggered {
    pub timer: Timer,
    pub damage_multiplier: f32,
}

#[derive(Component, Debug, Clone)]
pub struct SealWardenWard {
    pub current: f32,
    pub max: f32,
    pub broken: bool,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SealWardenWardHit {
    pub health_damage: f32,
    pub ward_damage: f32,
    pub ward_broken: bool,
}

impl Default for SealWardenWard {
    fn default() -> Self {
        Self {
            current: SEAL_WARDEN_WARD_MAX,
            max: SEAL_WARDEN_WARD_MAX,
            broken: false,
        }
    }
}

impl SealWardenWard {
    pub fn active(&self) -> bool {
        !self.broken && self.current > 0.0
    }

    pub fn absorb_damage(&mut self, damage: f32) -> SealWardenWardHit {
        let damage = damage.max(0.0);
        if !self.active() {
            return SealWardenWardHit {
                health_damage: damage,
                ward_damage: 0.0,
                ward_broken: false,
            };
        }

        let ward_damage = damage.min(self.current);
        self.current = (self.current - ward_damage).max(0.0);
        let ward_broken = self.current <= f32::EPSILON;
        if ward_broken {
            self.current = 0.0;
            self.broken = true;
        }

        SealWardenWardHit {
            health_damage: (damage - ward_damage).max(0.0),
            ward_damage,
            ward_broken,
        }
    }
}

const BOSS_ENRAGE_DELAY_SECS: f32 = 22.0;
const BOSS_STAGGER_THRESHOLD: f32 = 125.0;
const BOSS_STAGGER_DURATION_SECS: f32 = 2.8;
const BOSS_STAGGER_DAMAGE_MULTIPLIER: f32 = 1.18;
pub const SEAL_WARDEN_WARD_MAX: f32 = 45.0;
pub const SEAL_WARDEN_WARD_VULNERABLE_SECS: f32 = 3.0;

#[derive(Component)]
struct TreasureFiend {
    escape_timer: Timer,
    alerted: bool,
}

#[derive(Component)]
pub struct Chilled {
    pub timer: Timer,
    pub slow_multiplier: f32,
}

#[derive(Component)]
struct EliteAffixAura {
    base_scale: f32,
    spin_speed: f32,
    pulse_speed: f32,
    pulse_amount: f32,
    elapsed: f32,
}

#[derive(Component)]
struct EliteAffixAuraLight {
    base_intensity: f32,
    pulse_speed: f32,
    pulse_amount: f32,
    elapsed: f32,
}

#[derive(Component)]
struct MainQuestEnemyMarkerAttached;

#[derive(Component)]
struct MainQuestEnemyMarker {
    owner: Entity,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
enum MainQuestEnemyMarkerState {
    Hidden,
    SealWarden,
    Keeper,
}

#[derive(Component)]
struct EnemyProjectile {
    damage: f32,
    velocity: Vec3,
    radius: f32,
    lifetime: Timer,
    arm_timer: Option<Timer>,
    impact: ProjectileImpact,
}

#[derive(Clone, Copy)]
enum ProjectileImpact {
    None,
    Burning,
    Arcane,
    Jailer,
    Frozen,
    Desecrator,
    WardenRune,
}

#[derive(Message, Debug, Clone)]
pub struct EnemyKilled {
    pub enemy_id: String,
    pub display_name: String,
    pub position: Vec3,
    pub xp_reward: u32,
    pub affix_count: usize,
    pub affix_mask: u16,
    pub cursed_ambusher: bool,
    pub champion_pack_member: bool,
}

pub fn affix_mask(affixes: &[EnemyAffix]) -> u16 {
    affixes.iter().fold(0, |mask, affix| mask | affix.mask())
}

#[derive(Message, Debug, Clone, Copy)]
pub struct SpawnNemesis {
    pub checkpoint: Vec3,
}

#[derive(Message, Debug, Clone, Copy)]
pub struct SpawnAshenThreatWave {
    pub origin: Vec3,
    pub surge: u32,
}

pub struct EnemyPlugin;

type PlayerHealthQuery<'w, 's> = Query<
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
    (With<Player>, Without<Enemy>),
>;
type ProjectilePlayerQuery<'w, 's> = Query<
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
    (With<Player>, Without<EnemyProjectile>),
>;
type MinionDamageQuery<'w, 's> = Query<
    'w,
    's,
    (Entity, &'static Transform, &'static mut SummonVitality),
    (
        With<Minion>,
        Without<Player>,
        Without<Enemy>,
        Without<EnemyProjectile>,
    ),
>;

/// Prefer a nearby living guard without pulling enemies from beyond the hero's aggro range.
fn minion_attack_target(
    origin: Vec3,
    owner: Vec3,
    minions: &MinionDamageQuery,
) -> Option<(Entity, Vec3)> {
    let owner_distance = origin.distance(owner);
    minions
        .iter()
        .filter(|(_, pose, vitality)| {
            vitality.current > 0.0 && pose.translation.distance(owner) <= 10.0
        })
        .filter(|(_, pose, _)| pose.translation.distance(origin) + 0.35 < owner_distance)
        .min_by(|(_, a, _), (_, b, _)| {
            a.translation
                .distance_squared(origin)
                .total_cmp(&b.translation.distance_squared(origin))
        })
        .map(|(entity, pose, _)| (entity, pose.translation))
}

type EnemyAiQuery<'w, 's> = Query<
    'w,
    's,
    (
        Entity,
        &'static mut Transform,
        &'static mut Health,
        &'static mut Enemy,
        &'static mut EnemyAnimation,
        Option<&'static Chilled>,
        Option<&'static Staggered>,
        Option<&'static mut TreasureFiend>,
        Option<&'static mut EnemyHitReact>,
        Option<&'static mut EnemyAttackWarning>,
        &'static mut EnemySkillCycle,
    ),
>;
type EnemyLimbQuery<'w, 's> =
    Query<'w, 's, (&'static EnemyLimb, &'static mut Transform), Without<Enemy>>;
type EnemyLimbSourceQuery<'w, 's> =
    Query<'w, 's, (Entity, &'static Enemy, &'static EnemyAnimation)>;

#[derive(SystemParam)]
struct EnemyFeedbackWriters<'w> {
    combat_events: MessageWriter<'w, CombatEvent>,
    floating_text: MessageWriter<'w, FloatingCombatTextEvent>,
    screen_shake: MessageWriter<'w, ScreenShakeEvent>,
}

#[derive(SystemParam)]
struct EnemySpawnResources<'w, 's> {
    commands: Commands<'w, 's>,
    assets: Res<'w, GameAssets>,
    catalog: Res<'w, EnemyCatalog>,
    difficulty: Res<'w, DifficultySettings>,
    modifier: Res<'w, ChapterModifier>,
    active_chapter: Res<'w, ActiveChapter>,
}

#[derive(SystemSet, Debug, Clone, Copy, Eq, PartialEq, Hash)]
pub enum EnemySet {
    BossPhase,
}

impl Plugin for EnemyPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<PendingWildernessEncounters>()
            .add_message::<EnemyKilled>()
            .add_message::<SpawnNemesis>()
            .add_message::<SpawnAshenThreatWave>()
            .add_message::<SpawnWildernessEncounter>()
            .add_systems(Startup, setup_enemy_runtime_vfx)
            .add_systems(
                OnEnter(GameState::InGame),
                (clear_pending_wilderness_encounters, spawn_enemies).chain(),
            )
            .add_systems(
                Update,
                (
                    (bind_enemy_limbs, enemy_ai, animate_enemy_limbs_system).chain(),
                    tick_chilled,
                    tick_staggered,
                    tick_elite_affix_auras,
                    attach_main_quest_enemy_markers,
                    update_main_quest_enemy_markers,
                    tick_enemy_attack_telegraphs,
                    move_enemy_projectiles,
                    spawn_sanctum_wave,
                    spawn_cursed_shrine_wave,
                    spawn_obelisk_wave,
                    spawn_rift_wave,
                    spawn_ashen_threat_wave,
                    spawn_boss,
                    spawn_nemesis,
                )
                    .run_if(in_state(GameState::InGame).and_then(not_paused)),
            )
            .add_systems(
                Update,
                boss_phase_transition
                    .in_set(EnemySet::BossPhase)
                    .run_if(in_state(GameState::InGame).and_then(not_paused)),
            )
            .add_systems(
                Update,
                (
                    despawn_unloaded_wilderness_enemies,
                    spawn_wilderness_encounters,
                )
                    .chain()
                    .run_if(in_state(GameState::InGame).and_then(not_paused)),
            )
            .add_systems(
                OnExit(GameState::InGame),
                (despawn_enemies, clear_pending_wilderness_encounters).chain(),
            );
    }
}

fn clear_pending_wilderness_encounters(mut pending: ResMut<PendingWildernessEncounters>) {
    pending.events.clear();
}

fn setup_enemy_runtime_vfx(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let warning_materials = [
        add_opaque_vfx_material(
            &mut materials,
            enemy_attack_warning_color(&EnemyAttackKind::Melee),
        ),
        add_opaque_vfx_material(
            &mut materials,
            enemy_attack_warning_color(&EnemyAttackKind::Projectile),
        ),
        add_opaque_vfx_material(
            &mut materials,
            enemy_attack_warning_color(&EnemyAttackKind::Shockwave),
        ),
    ];
    let impact_materials = [
        add_opaque_vfx_material(
            &mut materials,
            enemy_attack_impact_style(&EnemyAttackKind::Melee, 1.0).color,
        ),
        add_opaque_vfx_material(
            &mut materials,
            enemy_attack_impact_style(&EnemyAttackKind::Projectile, 1.0).color,
        ),
        add_opaque_vfx_material(
            &mut materials,
            enemy_attack_impact_style(&EnemyAttackKind::Shockwave, 1.0).color,
        ),
    ];

    let choir_shot = add_opaque_vfx_material(&mut materials, Color::srgb(0.10, 0.88, 1.0));
    let reliquary_shot = add_opaque_vfx_material(&mut materials, Color::srgb(1.0, 0.62, 0.10));
    let hollow_spell = add_opaque_vfx_material(&mut materials, Color::srgb(0.18, 0.72, 1.0));
    let cinder_spell = add_opaque_vfx_material(&mut materials, Color::srgb(1.0, 0.22, 0.04));
    let bell_note = add_opaque_vfx_material(&mut materials, Color::srgb(0.46, 0.26, 1.0));
    let seal_orb = add_opaque_vfx_material(&mut materials, Color::srgb(0.66, 0.18, 1.0));
    let projectile_materials = HashMap::from([
        ("Choir Trident Volley", choir_shot.clone()),
        ("Choir Lance", choir_shot),
        ("Reliquary Trident Volley", reliquary_shot.clone()),
        ("Reliquary Bolt", reliquary_shot),
        ("Hollow Duet", hollow_spell.clone()),
        ("Hollow Note", hollow_spell),
        ("Cinder Duet", cinder_spell.clone()),
        ("Cinder Bolt", cinder_spell),
        ("Bell Warden Note", bell_note),
        ("Seal Warden Orb", seal_orb),
    ]);

    commands.insert_resource(EnemyRuntimeVfx {
        flat_box: meshes.add(attack_lane_outline_mesh()),
        flat_disc: meshes.add(ground_mark(1.0, GroundMark::Wave)),
        impact_spark: meshes.add(impact_spark_mesh()),
        projectile_bolt: meshes.add(projectile_bolt()),
        warning_materials,
        impact_materials,
        projectile_materials,
    });
}

fn add_opaque_vfx_material(
    materials: &mut Assets<StandardMaterial>,
    color: Color,
) -> Handle<StandardMaterial> {
    materials.add(StandardMaterial {
        base_color: color,
        emissive: color.into(),
        alpha_mode: AlphaMode::Opaque,
        ..default()
    })
}

fn spawn_enemies(
    mut commands: Commands,
    mut combat_events: MessageWriter<CombatEvent>,
    assets: Res<GameAssets>,
    catalog: Res<EnemyCatalog>,
    difficulty: Res<DifficultySettings>,
    modifier: Res<ChapterModifier>,
    active_chapter: Res<ActiveChapter>,
) {
    // These are authored in chapter-local coordinates, then expanded into the
    // shared world so each chapter keeps the same readable opening encounter.
    let spawns = [
        ("skeleton", Vec3::new(-6.0, 0.0, -2.5), vec![]),
        (
            "bone_stalker",
            Vec3::new(-3.0, 0.0, -5.5),
            vec![EnemyAffix::Frenzied],
        ),
        ("ashen_marksman", Vec3::new(3.2, 0.0, -7.0), vec![]),
        ("skeleton", Vec3::new(7.0, 0.0, -2.0), vec![]),
    ];

    for (base_id, local_position, affixes) in spawns {
        let id = chapter_enemy_id(active_chapter.0, base_id);
        let Some(tuning) = catalog.get(id) else {
            continue;
        };
        let scene = enemy_scene(&assets, &tuning.model);
        spawn_enemy(
            &mut commands,
            &assets,
            EnemySpawnSpec {
                scene,
                tuning,
                position: chapter_world_position(active_chapter.0, local_position),
                difficulty: difficulty.current,
                modifier: *modifier,
                affixes,
            },
        );
    }

    spawn_treasure_fiend(
        &mut commands,
        &assets,
        &catalog,
        difficulty.current,
        *modifier,
        chapter_world_position(active_chapter.0, Vec3::new(-8.5, 0.0, 5.6)),
    );

    spawn_champion_pack(
        &mut commands,
        &assets,
        &catalog,
        difficulty.current,
        *modifier,
        active_chapter.0,
    );
    combat_events.write(CombatEvent {
        text: match active_chapter.0 {
            ChapterId::AshenReliquary => {
                "A reliquary champion pack patrols the ashen wilds".to_string()
            }
            ChapterId::HollowBastion => {
                "A Hollow Choir champion pack patrols the bastion moor".to_string()
            }
        },
    });
}

// Keep streamed wilderness packs bounded for the WebAssembly/WebGPU build.
const MAX_ACTIVE_WILDERNESS_ENEMIES: usize = 32;

fn wilderness_pack_fits(active_count: usize, pack_size: usize) -> bool {
    active_count.saturating_add(pack_size) <= MAX_ACTIVE_WILDERNESS_ENEMIES
}

fn spawn_wilderness_encounters(
    mut resources: EnemySpawnResources,
    mut events: MessageReader<SpawnWildernessEncounter>,
    mut pending: ResMut<PendingWildernessEncounters>,
    active_wilderness: Query<&WildernessEnemy>,
    player: Query<&Transform, With<Player>>,
) {
    let active_homes = active_wilderness
        .iter()
        .map(|enemy| (enemy.chapter, enemy.chunk_x, enemy.chunk_z))
        .collect::<HashSet<_>>();
    for event in events.read().copied() {
        let key = (event.chapter, event.chunk_x, event.chunk_z);
        if !active_homes.contains(&key)
            && !pending
                .events
                .iter()
                .any(|queued| (queued.chapter, queued.chunk_x, queued.chunk_z) == key)
        {
            pending.events.push_back(event);
        }
    }

    let Ok(player_transform) = player.single() else {
        return;
    };
    let player_chunk_x = (player_transform.translation.x / CHUNK_SIZE).floor() as i64;
    let player_chunk_z = (player_transform.translation.z / CHUNK_SIZE).floor() as i64;
    pending.events.retain(|event| {
        event.chapter == resources.active_chapter.0
            && event
                .chunk_x
                .abs_diff(player_chunk_x)
                .max(event.chunk_z.abs_diff(player_chunk_z))
                <= CHUNK_UNLOAD_RADIUS as u64
    });

    let mut active_count = active_wilderness.iter().count();
    while let Some(event) = pending.events.front().copied() {
        let specs = wilderness_encounter_specs(event.chapter, resources.difficulty.current, event);
        if !wilderness_pack_fits(active_count, specs.len()) {
            break;
        }
        pending.events.pop_front();

        for spec in specs {
            let Some(tuning) = resources.catalog.get(spec.id) else {
                continue;
            };
            let mut scaled_tuning = tuning.clone();
            let threat_multiplier =
                wilderness_distance_threat_multiplier(event.chunk_x, event.chunk_z);
            scaled_tuning.max_health *= threat_multiplier;
            scaled_tuning.attack_damage *= threat_multiplier.sqrt();
            scaled_tuning.xp_reward =
                scale_reward(scaled_tuning.xp_reward, threat_multiplier.max(1.0));
            scaled_tuning.gold_min =
                scale_reward(scaled_tuning.gold_min, threat_multiplier.max(1.0));
            scaled_tuning.gold_max =
                scale_reward(scaled_tuning.gold_max, threat_multiplier.max(1.0));

            let entity = spawn_enemy(
                &mut resources.commands,
                &resources.assets,
                EnemySpawnSpec {
                    scene: enemy_scene(&resources.assets, &scaled_tuning.model),
                    tuning: &scaled_tuning,
                    position: event.center + spec.offset,
                    difficulty: resources.difficulty.current,
                    modifier: *resources.modifier,
                    affixes: spec.affixes,
                },
            );
            resources
                .commands
                .entity(entity)
                .try_insert(WildernessEnemy {
                    chapter: event.chapter,
                    chunk_x: event.chunk_x,
                    chunk_z: event.chunk_z,
                });
            active_count += 1;
        }
    }
}

fn despawn_unloaded_wilderness_enemies(
    mut commands: Commands,
    active_chapter: Res<ActiveChapter>,
    player: Query<&Transform, (With<Player>, Without<WildernessEnemy>)>,
    enemies: Query<(Entity, &WildernessEnemy, &Transform)>,
) {
    let Ok(player_transform) = player.single() else {
        return;
    };
    let player_chunk_x = (player_transform.translation.x / CHUNK_SIZE).floor() as i64;
    let player_chunk_z = (player_transform.translation.z / CHUNK_SIZE).floor() as i64;

    for (entity, home, transform) in &enemies {
        let home_distance = home
            .chunk_x
            .abs_diff(player_chunk_x)
            .max(home.chunk_z.abs_diff(player_chunk_z));
        let current_chunk_x = (transform.translation.x / CHUNK_SIZE).floor() as i64;
        let current_chunk_z = (transform.translation.z / CHUNK_SIZE).floor() as i64;
        let current_distance = current_chunk_x
            .abs_diff(player_chunk_x)
            .max(current_chunk_z.abs_diff(player_chunk_z));
        if (home.chapter != active_chapter.0
            || (home_distance > CHUNK_UNLOAD_RADIUS as u64
                && current_distance > CHUNK_UNLOAD_RADIUS as u64))
            && let Ok(mut entity_commands) = commands.get_entity(entity)
        {
            entity_commands.try_despawn();
        }
    }
}

struct WildernessEncounterSpec {
    id: &'static str,
    offset: Vec3,
    affixes: Vec<EnemyAffix>,
}

fn wilderness_encounter_specs(
    chapter: ChapterId,
    difficulty: Difficulty,
    event: SpawnWildernessEncounter,
) -> Vec<WildernessEncounterSpec> {
    let mut rng = EncounterRng::new(event.seed);
    let variant = rng.index(4);
    let base_size = 2 + rng.index(2);
    let size = (base_size + difficulty.wilderness_pack_bonus()).min(5);
    let base_roles: [&str; 5] = match variant {
        0 => [
            "skeleton",
            "bone_stalker",
            "bone_stalker",
            "ashen_marksman",
            "skeleton",
        ],
        1 => [
            "skeleton",
            "cultist",
            "ashen_marksman",
            "bone_stalker",
            "cultist",
        ],
        2 => [
            "reliquary_brute",
            "skeleton",
            "ashen_marksman",
            "skeleton",
            "bone_stalker",
        ],
        _ => [
            "skeleton",
            "skeleton",
            "ashen_marksman",
            "cultist",
            "reliquary_brute",
        ],
    };

    (0..size)
        .map(|index| {
            let angle = rng.range_f32(0.0, std::f32::consts::TAU);
            let radius = rng.range_f32(3.2, 9.4);
            WildernessEncounterSpec {
                id: chapter_enemy_id(chapter, base_roles[index]),
                offset: Vec3::new(angle.cos() * radius, 0.0, angle.sin() * radius),
                affixes: wilderness_enemy_affixes(difficulty, event.seed, index, size),
            }
        })
        .collect()
}

fn wilderness_enemy_affixes(
    difficulty: Difficulty,
    seed: u64,
    index: usize,
    pack_size: usize,
) -> Vec<EnemyAffix> {
    let desired = match difficulty {
        Difficulty::Normal if index == 0 && seed.is_multiple_of(5) => 1,
        Difficulty::Normal => 0,
        Difficulty::Nightmare if index == 0 => 1,
        Difficulty::Nightmare => 0,
        Difficulty::Hell if index == 0 => 2,
        Difficulty::Hell if index + 1 == pack_size => 1,
        Difficulty::Hell => 0,
        Difficulty::Torment if index == 0 => 2,
        Difficulty::Torment if index + 1 == pack_size => 2,
        Difficulty::Torment => 0,
    };
    const POOL: [EnemyAffix; 9] = [
        EnemyAffix::Frenzied,
        EnemyAffix::Vampiric,
        EnemyAffix::Molten,
        EnemyAffix::Shielded,
        EnemyAffix::Arcane,
        EnemyAffix::Jailer,
        EnemyAffix::Frozen,
        EnemyAffix::Desecrator,
        EnemyAffix::Reflective,
    ];
    let start = ((seed.rotate_left((index as u32 * 7) % 64) >> 24) as usize) % POOL.len();
    (0..desired)
        .map(|offset| POOL[(start + offset * 3) % POOL.len()])
        .collect()
}

fn wilderness_distance_threat_multiplier(chunk_x: i64, chunk_z: i64) -> f32 {
    let distance = chunk_x.abs().max(chunk_z.abs()) as f32;
    (1.0 + (distance / 12.0).floor() * 0.08).clamp(1.0, 1.80)
}

struct EncounterRng {
    state: u64,
}

impl EncounterRng {
    fn new(seed: u64) -> Self {
        Self { state: seed }
    }

    fn next_u64(&mut self) -> u64 {
        self.state = self.state.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut value = self.state;
        value = (value ^ (value >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        value = (value ^ (value >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        value ^ (value >> 31)
    }

    fn index(&mut self, upper_bound: usize) -> usize {
        (self.next_u64() % upper_bound as u64) as usize
    }

    fn range_f32(&mut self, min: f32, max: f32) -> f32 {
        let unit = ((self.next_u64() >> 40) as f32) * (1.0 / 16_777_216.0);
        min + (max - min) * unit
    }
}

fn champion_pack_spawns() -> [(String, Vec3, Vec<EnemyAffix>); 5] {
    [
        (
            "ashen_marksman".to_string(),
            Vec3::new(-7.4, 0.0, -1.2),
            vec![
                EnemyAffix::Frenzied,
                EnemyAffix::Shielded,
                EnemyAffix::Arcane,
                EnemyAffix::Reflective,
            ],
        ),
        (
            "bone_stalker".to_string(),
            Vec3::new(-9.1, 0.0, -0.3),
            vec![EnemyAffix::Molten],
        ),
        (
            "reliquary_brute".to_string(),
            Vec3::new(-7.9, 0.0, -3.0),
            vec![EnemyAffix::Jailer],
        ),
        ("cultist".to_string(), Vec3::new(-5.8, 0.0, -1.9), vec![]),
        ("skeleton".to_string(), Vec3::new(-6.6, 0.0, 0.6), vec![]),
    ]
}

fn spawn_champion_pack(
    commands: &mut Commands,
    assets: &GameAssets,
    catalog: &EnemyCatalog,
    difficulty: Difficulty,
    modifier: ChapterModifier,
    chapter: ChapterId,
) {
    for (base_id, local_position, affixes) in champion_pack_spawns() {
        let id = chapter_enemy_id(chapter, &base_id);
        let Some(tuning) = catalog.get(id) else {
            continue;
        };
        let scene = enemy_scene(assets, &tuning.model);
        let entity = spawn_enemy(
            commands,
            assets,
            EnemySpawnSpec {
                scene,
                tuning,
                position: chapter_world_position(chapter, local_position),
                difficulty,
                modifier,
                affixes,
            },
        );
        commands.entity(entity).try_insert(ChampionPackMember);
    }
}

fn spawn_treasure_fiend(
    commands: &mut Commands,
    assets: &GameAssets,
    catalog: &EnemyCatalog,
    difficulty: Difficulty,
    modifier: ChapterModifier,
    position: Vec3,
) {
    let Some(tuning) = catalog.get("treasure_imp") else {
        return;
    };
    let entity = spawn_enemy(
        commands,
        assets,
        EnemySpawnSpec {
            scene: assets.cultist.clone(),
            tuning,
            position,
            difficulty,
            modifier,
            affixes: vec![EnemyAffix::Frenzied],
        },
    );
    commands.entity(entity).try_insert(TreasureFiend {
        escape_timer: Timer::from_seconds(10.0, TimerMode::Once),
        alerted: false,
    });
}

fn spawn_sanctum_wave(
    mut resources: EnemySpawnResources,
    mut wave_events: MessageReader<SpawnSanctumWave>,
    mut combat_events: MessageWriter<CombatEvent>,
) {
    for _ in wave_events.read() {
        for (base_id, local_position, affixes) in [
            ("ashen_marksman", Vec3::new(-4.0, 0.0, -15.0), vec![]),
            (
                "seal_warden",
                Vec3::new(0.0, 0.0, -19.0),
                vec![
                    EnemyAffix::Vampiric,
                    EnemyAffix::Shielded,
                    EnemyAffix::Arcane,
                    EnemyAffix::Jailer,
                    EnemyAffix::Frozen,
                    EnemyAffix::Desecrator,
                    EnemyAffix::Reflective,
                ],
            ),
            (
                "reliquary_brute",
                Vec3::new(4.5, 0.0, -15.5),
                vec![EnemyAffix::Molten],
            ),
        ] {
            let id = chapter_enemy_id(resources.active_chapter.0, base_id);
            let Some(tuning) = resources.catalog.get(id) else {
                continue;
            };
            let scene = enemy_scene(&resources.assets, &tuning.model);
            spawn_enemy(
                &mut resources.commands,
                &resources.assets,
                EnemySpawnSpec {
                    scene,
                    tuning,
                    position: chapter_world_position(resources.active_chapter.0, local_position),
                    difficulty: resources.difficulty.current,
                    modifier: *resources.modifier,
                    affixes,
                },
            );
        }
        combat_events.write(CombatEvent {
            text: format!(
                "{} guards the second chapter seal",
                resources.active_chapter.0.warden_name()
            ),
        });
    }
}

fn cursed_shrine_spawns(origin: Vec3) -> [(String, Vec3, Vec<EnemyAffix>); 4] {
    [
        (
            "bone_stalker".to_string(),
            origin + Vec3::new(1.8, 0.0, -1.2),
            vec![EnemyAffix::Frenzied],
        ),
        (
            "reliquary_brute".to_string(),
            origin + Vec3::new(-1.6, 0.0, -1.5),
            vec![
                EnemyAffix::Vampiric,
                EnemyAffix::Shielded,
                EnemyAffix::Arcane,
                EnemyAffix::Jailer,
                EnemyAffix::Frozen,
                EnemyAffix::Desecrator,
                EnemyAffix::Reflective,
            ],
        ),
        (
            "ashen_marksman".to_string(),
            origin + Vec3::new(0.2, 0.0, 2.0),
            vec![EnemyAffix::Molten],
        ),
        (
            "skeleton".to_string(),
            origin + Vec3::new(-2.3, 0.0, 1.3),
            vec![],
        ),
    ]
}

fn spawn_cursed_shrine_wave(
    mut resources: EnemySpawnResources,
    mut interactions: MessageReader<InteractableUsed>,
    mut combat_events: MessageWriter<CombatEvent>,
) {
    for event in interactions.read() {
        if event.kind != InteractableKind::CursedShrine {
            continue;
        }
        for (base_id, position, affixes) in cursed_shrine_spawns(event.position) {
            let id = chapter_enemy_id(resources.active_chapter.0, &base_id);
            let Some(tuning) = resources.catalog.get(id) else {
                continue;
            };
            let scene = enemy_scene(&resources.assets, &tuning.model);
            let entity = spawn_enemy(
                &mut resources.commands,
                &resources.assets,
                EnemySpawnSpec {
                    scene,
                    tuning,
                    position,
                    difficulty: resources.difficulty.current,
                    modifier: *resources.modifier,
                    affixes,
                },
            );
            resources.commands.entity(entity).try_insert(CursedAmbusher);
        }
        combat_events.write(CombatEvent {
            text: "Cursed shrine awakened an elite ambush".to_string(),
        });
    }
}

fn obelisk_spawns(origin: Vec3) -> [(String, Vec3, Vec<EnemyAffix>); 5] {
    [
        (
            "bone_stalker".to_string(),
            origin + Vec3::new(1.7, 0.0, -1.3),
            vec![],
        ),
        (
            "ashen_marksman".to_string(),
            origin + Vec3::new(-1.8, 0.0, -1.1),
            vec![EnemyAffix::Frenzied],
        ),
        (
            "skeleton".to_string(),
            origin + Vec3::new(1.2, 0.0, 1.7),
            vec![EnemyAffix::Molten],
        ),
        (
            "reliquary_brute".to_string(),
            origin + Vec3::new(-1.4, 0.0, 1.6),
            vec![EnemyAffix::Shielded, EnemyAffix::Arcane],
        ),
        (
            "skeleton".to_string(),
            origin + Vec3::new(0.0, 0.0, 2.35),
            vec![EnemyAffix::Desecrator, EnemyAffix::Reflective],
        ),
    ]
}

fn spawn_obelisk_wave(
    mut resources: EnemySpawnResources,
    mut events: MessageReader<SpawnObeliskWave>,
    mut combat_events: MessageWriter<CombatEvent>,
) {
    for event in events.read() {
        for (base_id, position, affixes) in obelisk_spawns(event.origin) {
            let id = chapter_enemy_id(resources.active_chapter.0, &base_id);
            let Some(tuning) = resources.catalog.get(id) else {
                continue;
            };
            let scene = enemy_scene(&resources.assets, &tuning.model);
            spawn_enemy(
                &mut resources.commands,
                &resources.assets,
                EnemySpawnSpec {
                    scene,
                    tuning,
                    position,
                    difficulty: resources.difficulty.current,
                    modifier: *resources.modifier,
                    affixes,
                },
            );
        }
        combat_events.write(CombatEvent {
            text: "Blood obelisk summoned a kill rite".to_string(),
        });
    }
}

fn rift_spawns(origin: Vec3) -> [(String, Vec3, Vec<EnemyAffix>); 6] {
    [
        (
            "bone_stalker".to_string(),
            origin + Vec3::new(1.8, 0.0, -1.6),
            vec![EnemyAffix::Frenzied],
        ),
        (
            "skeleton".to_string(),
            origin + Vec3::new(-1.8, 0.0, -1.4),
            vec![EnemyAffix::Molten],
        ),
        (
            "ashen_marksman".to_string(),
            origin + Vec3::new(2.4, 0.0, 0.8),
            vec![EnemyAffix::Arcane],
        ),
        (
            "skeleton".to_string(),
            origin + Vec3::new(-2.3, 0.0, 0.9),
            vec![EnemyAffix::Jailer],
        ),
        (
            "reliquary_brute".to_string(),
            origin + Vec3::new(0.2, 0.0, 2.3),
            vec![EnemyAffix::Shielded, EnemyAffix::Desecrator],
        ),
        (
            "skeleton".to_string(),
            origin + Vec3::new(0.0, 0.0, -2.7),
            vec![EnemyAffix::Vampiric, EnemyAffix::Reflective],
        ),
    ]
}

fn spawn_rift_wave(
    mut resources: EnemySpawnResources,
    mut events: MessageReader<SpawnRiftWave>,
    mut combat_events: MessageWriter<CombatEvent>,
) {
    for event in events.read() {
        for (base_id, position, affixes) in rift_spawns(event.origin) {
            let id = chapter_enemy_id(resources.active_chapter.0, &base_id);
            let Some(tuning) = resources.catalog.get(id) else {
                continue;
            };
            let scene = enemy_scene(&resources.assets, &tuning.model);
            spawn_enemy(
                &mut resources.commands,
                &resources.assets,
                EnemySpawnSpec {
                    scene,
                    tuning,
                    position,
                    difficulty: resources.difficulty.current,
                    modifier: *resources.modifier,
                    affixes,
                },
            );
        }
        combat_events.write(CombatEvent {
            text: "Ember rift disgorged an elite invader pack".to_string(),
        });
    }
}

fn ashen_threat_spawns(origin: Vec3, surge: u32) -> [(String, Vec3, Vec<EnemyAffix>); 3] {
    let pressure_affixes = if surge >= 3 {
        vec![EnemyAffix::Arcane, EnemyAffix::Jailer]
    } else {
        vec![EnemyAffix::Frenzied]
    };
    [
        (
            "skeleton".to_string(),
            origin + Vec3::new(1.9, 0.0, -1.5),
            vec![EnemyAffix::Molten],
        ),
        (
            "ashen_marksman".to_string(),
            origin + Vec3::new(-1.7, 0.0, -1.4),
            pressure_affixes,
        ),
        (
            "bone_stalker".to_string(),
            origin + Vec3::new(0.1, 0.0, 2.1),
            vec![EnemyAffix::Shielded],
        ),
    ]
}

fn spawn_ashen_threat_wave(
    mut resources: EnemySpawnResources,
    mut events: MessageReader<SpawnAshenThreatWave>,
    mut combat_events: MessageWriter<CombatEvent>,
) {
    for event in events.read() {
        for (base_id, position, affixes) in ashen_threat_spawns(event.origin, event.surge) {
            let id = chapter_enemy_id(resources.active_chapter.0, &base_id);
            let Some(tuning) = resources.catalog.get(id) else {
                continue;
            };
            let scene = enemy_scene(&resources.assets, &tuning.model);
            spawn_enemy(
                &mut resources.commands,
                &resources.assets,
                EnemySpawnSpec {
                    scene,
                    tuning,
                    position,
                    difficulty: resources.difficulty.current,
                    modifier: *resources.modifier,
                    affixes,
                },
            );
        }
        combat_events.write(CombatEvent {
            text: match resources.active_chapter.0 {
                ChapterId::AshenReliquary => format!(
                    "Ashen threat surge {} opened a retaliation wave",
                    event.surge
                ),
                ChapterId::HollowBastion => format!(
                    "Hollow resonance surge {} opened a retaliation wave",
                    event.surge
                ),
            },
        });
    }
}

fn spawn_boss(
    mut commands: Commands,
    mut boss_events: MessageReader<SpawnBoss>,
    assets: Res<GameAssets>,
    catalog: Res<EnemyCatalog>,
    difficulty: Res<DifficultySettings>,
    modifier: Res<ChapterModifier>,
    active_chapter: Res<ActiveChapter>,
) {
    for _ in boss_events.read() {
        let boss_id = active_chapter.0.boss_id();
        let Some(tuning) = catalog.get(boss_id) else {
            continue;
        };
        let boss_position = chapter_world_position(active_chapter.0, Vec3::new(0.0, 0.0, -20.0));
        commands.spawn((
            WorldAssetRoot(assets.boss_summon_portal.clone()),
            Transform::from_translation(boss_position).with_scale(Vec3::splat(1.18)),
            EnemyEntity,
            Name::new(format!(
                "{} Summoning Seal",
                active_chapter.0.boss_short_name()
            )),
        ));
        let summon_color = match active_chapter.0 {
            ChapterId::AshenReliquary => Color::srgb(1.0, 0.18, 0.04),
            ChapterId::HollowBastion => Color::srgb(0.10, 0.82, 1.0),
        };
        commands.spawn((
            PointLight {
                color: summon_color,
                intensity: 950.0,
                range: 8.0,
                shadow_maps_enabled: false,
                ..default()
            },
            Transform::from_translation(boss_position + Vec3::Y * 1.35),
            EnemyEntity,
            Name::new(format!(
                "{} Summoning Light",
                active_chapter.0.boss_short_name()
            )),
        ));
        let boss = spawn_enemy(
            &mut commands,
            &assets,
            EnemySpawnSpec {
                scene: enemy_scene(&assets, &tuning.model),
                tuning,
                position: boss_position,
                difficulty: difficulty.current,
                modifier: *modifier,
                affixes: boss_starting_affixes(active_chapter.0),
            },
        );
        commands.entity(boss).try_insert(BossPhase {
            phase_two_started: false,
            enrage_started: false,
            enrage_timer: Timer::from_seconds(BOSS_ENRAGE_DELAY_SECS, TimerMode::Once),
        });
        commands.entity(boss).try_insert(BossStagger::default());
    }
}

fn boss_starting_affixes(chapter: ChapterId) -> Vec<EnemyAffix> {
    match chapter {
        ChapterId::AshenReliquary => vec![
            EnemyAffix::Molten,
            EnemyAffix::Vampiric,
            EnemyAffix::Shielded,
        ],
        ChapterId::HollowBastion => {
            vec![EnemyAffix::Arcane, EnemyAffix::Frozen, EnemyAffix::Shielded]
        }
    }
}

fn spawn_nemesis(
    mut resources: EnemySpawnResources,
    mut events: MessageReader<SpawnNemesis>,
    mut combat_events: MessageWriter<CombatEvent>,
) {
    for event in events.read() {
        let Some(tuning) = resources.catalog.get("nemesis") else {
            continue;
        };
        spawn_enemy(
            &mut resources.commands,
            &resources.assets,
            EnemySpawnSpec {
                scene: enemy_scene(&resources.assets, &tuning.model),
                tuning,
                position: nemesis_spawn_position(resources.active_chapter.0, event.checkpoint),
                difficulty: resources.difficulty.current,
                modifier: *resources.modifier,
                affixes: nemesis_affixes(),
            },
        );
        combat_events.write(CombatEvent {
            text: "A soul-bound nemesis clawed out of the ash".to_string(),
        });
    }
}

struct EnemySpawnSpec<'a> {
    scene: Handle<WorldAsset>,
    tuning: &'a EnemyTuning,
    position: Vec3,
    difficulty: Difficulty,
    modifier: ChapterModifier,
    affixes: Vec<EnemyAffix>,
}

fn spawn_enemy(commands: &mut Commands, assets: &GameAssets, spec: EnemySpawnSpec) -> Entity {
    let EnemySpawnSpec {
        scene,
        tuning,
        position,
        difficulty,
        modifier,
        affixes,
    } = spec;
    let is_elite = !affixes.is_empty();
    let affix_count = affixes.len();
    let affix_visual = affix_visual_kind(&affixes);
    let affix_visual_scale = affix_visual_scale(&affixes);
    let max_health = tuning.max_health
        * difficulty.enemy_health_multiplier()
        * modifier.enemy_health_multiplier()
        * if is_elite { 1.55 } else { 1.0 };
    let reward_multiplier = difficulty.reward_multiplier()
        * modifier.reward_multiplier()
        * if is_elite { 1.8 } else { 1.0 };
    let has_frenzied = affixes.contains(&EnemyAffix::Frenzied);
    let display_name = display_name_with_affixes(&tuning.display_name, &affixes);
    let attack_cooldown = tuning.attack_cooldown
        * difficulty.enemy_attack_cooldown_multiplier()
        * if has_frenzied { 0.72 } else { 1.0 };
    let base_scale = if is_elite { 1.12 } else { 1.0 };
    let entity = commands
        .spawn((
            WorldAssetRoot(scene),
            Transform::from_translation(position).with_scale(Vec3::splat(base_scale)),
            Health {
                current: max_health,
                max: max_health,
            },
            Enemy {
                id: tuning.id.clone(),
                display_name: display_name.clone(),
                affixes,
                attack_damage: tuning.attack_damage
                    * difficulty.enemy_damage_multiplier()
                    * modifier.enemy_damage_multiplier()
                    * if is_elite { 1.22 } else { 1.0 },
                attack_kind: tuning.attack_kind.clone(),
                attack_range: tuning.attack_range,
                attack_timer: Timer::from_seconds(attack_cooldown, TimerMode::Once),
                aggro_range: tuning.aggro_range,
                move_speed: tuning.move_speed
                    * difficulty.enemy_speed_multiplier()
                    * if has_frenzied { 1.28 } else { 1.0 },
                gold_min: scale_reward(tuning.gold_min, reward_multiplier),
                gold_max: scale_reward(tuning.gold_max, reward_multiplier),
                xp_reward: scale_reward(tuning.xp_reward, reward_multiplier),
            },
            EnemySkillCycle::new(difficulty),
            EnemyAnimation::new(base_scale),
            EnemyEntity,
            Name::new(display_name),
        ))
        .id();
    if matches!(tuning.id.as_str(), "seal_warden" | "bell_warden") {
        commands
            .entity(entity)
            .try_insert(SealWardenWard::default());
    }
    if let Some(visual) = affix_visual {
        let scene = affix_visual_scene(assets, visual);
        commands.entity(entity).with_children(|parent| {
            parent.spawn((
                WorldAssetRoot(scene),
                Transform::from_translation(Vec3::Y * 0.05)
                    .with_scale(Vec3::splat(affix_visual_scale)),
                EliteAffixAura {
                    base_scale: affix_visual_scale,
                    spin_speed: affix_aura_spin_speed(affix_count),
                    pulse_speed: affix_aura_pulse_speed(affix_count),
                    pulse_amount: affix_aura_pulse_amount(affix_count),
                    elapsed: 0.0,
                },
                crate::enemy_death::DeathDecoration,
                Name::new("Elite Affix Aura"),
            ));
            parent.spawn((
                PointLight {
                    color: affix_visual_light_color(visual),
                    intensity: affix_visual_light_intensity(affix_count),
                    range: affix_visual_light_range(affix_count),
                    shadow_maps_enabled: false,
                    ..default()
                },
                Transform::from_translation(Vec3::Y * 1.05),
                EliteAffixAuraLight {
                    base_intensity: affix_visual_light_intensity(affix_count),
                    pulse_speed: affix_aura_pulse_speed(affix_count),
                    pulse_amount: 0.22,
                    elapsed: 0.0,
                },
                crate::enemy_death::DeathDecoration,
                Name::new("Elite Affix Aura Light"),
            ));
        });
    }
    entity
}

fn attach_main_quest_enemy_markers(
    mut commands: Commands,
    assets: Res<GameAssets>,
    enemies: Query<Entity, (With<Enemy>, Without<MainQuestEnemyMarkerAttached>)>,
) {
    for entity in &enemies {
        let marker = commands
            .spawn((
                WorldAssetRoot(assets.objective_sigil.clone()),
                Transform::from_xyz(
                    0.0,
                    main_quest_enemy_marker_height(MainQuestEnemyMarkerState::SealWarden),
                    0.0,
                )
                .with_scale(Vec3::splat(main_quest_enemy_marker_scale(
                    MainQuestEnemyMarkerState::SealWarden,
                    0.0,
                ))),
                Visibility::Hidden,
                MainQuestEnemyMarker { owner: entity },
                crate::enemy_death::DeathDecoration,
                Name::new("Main Quest Enemy Marker"),
            ))
            .id();
        commands.entity(entity).add_child(marker);
        commands
            .entity(entity)
            .try_insert(MainQuestEnemyMarkerAttached);
    }
}

fn update_main_quest_enemy_markers(
    time: Res<Time>,
    progress: Res<ChapterProgress>,
    enemies: Query<(&Enemy, &Health)>,
    mut markers: Query<(&MainQuestEnemyMarker, &mut Visibility, &mut Transform)>,
) {
    let pulse = (time.elapsed_secs() * 4.2).sin() * 0.5 + 0.5;
    let bob = (time.elapsed_secs() * 2.7).sin() * 0.5 + 0.5;
    for (marker, mut visibility, mut transform) in &mut markers {
        let state = enemies
            .get(marker.owner)
            .map(|(enemy, health)| main_quest_enemy_marker_state(&progress, enemy, health))
            .unwrap_or(MainQuestEnemyMarkerState::Hidden);
        if state == MainQuestEnemyMarkerState::Hidden {
            *visibility = Visibility::Hidden;
            continue;
        }
        *visibility = Visibility::Visible;
        transform.translation.y = main_quest_enemy_marker_height(state) + bob * 0.18;
        transform.scale = Vec3::splat(main_quest_enemy_marker_scale(state, pulse));
    }
}

fn main_quest_enemy_marker_state(
    progress: &ChapterProgress,
    enemy: &Enemy,
    health: &Health,
) -> MainQuestEnemyMarkerState {
    if health.current <= 0.0 {
        return MainQuestEnemyMarkerState::Hidden;
    }
    let chapter = progress.chapter_id();
    if progress.phase == ChapterPhase::Sanctum && enemy.id == chapter.warden_id() {
        MainQuestEnemyMarkerState::SealWarden
    } else if progress.phase == ChapterPhase::Boss
        && enemy.id == chapter.boss_id()
        && !progress.boss_slain
    {
        MainQuestEnemyMarkerState::Keeper
    } else {
        MainQuestEnemyMarkerState::Hidden
    }
}

fn main_quest_enemy_marker_height(state: MainQuestEnemyMarkerState) -> f32 {
    match state {
        MainQuestEnemyMarkerState::Keeper => 3.15,
        MainQuestEnemyMarkerState::SealWarden => 2.45,
        MainQuestEnemyMarkerState::Hidden => 0.0,
    }
}

fn main_quest_enemy_marker_scale(state: MainQuestEnemyMarkerState, pulse: f32) -> f32 {
    let pulse = pulse.clamp(0.0, 1.0);
    match state {
        MainQuestEnemyMarkerState::Keeper => 0.78 + pulse * 0.14,
        MainQuestEnemyMarkerState::SealWarden => 0.58 + pulse * 0.10,
        MainQuestEnemyMarkerState::Hidden => 0.0,
    }
}

fn affix_visual_scene(assets: &GameAssets, visual: EliteAffixVisual) -> Handle<WorldAsset> {
    match visual {
        EliteAffixVisual::Arcane => assets.affix_arcane_aura.clone(),
        EliteAffixVisual::Frost => assets.affix_frost_aura.clone(),
        EliteAffixVisual::Ember => assets.affix_ember_aura.clone(),
        EliteAffixVisual::Blood => assets.affix_blood_aura.clone(),
        EliteAffixVisual::Ward => assets.affix_ward_aura.clone(),
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum EliteAffixVisual {
    Arcane,
    Frost,
    Ember,
    Blood,
    Ward,
}

fn affix_visual_kind(affixes: &[EnemyAffix]) -> Option<EliteAffixVisual> {
    if affixes.is_empty() {
        return None;
    }
    if affixes.contains(&EnemyAffix::Arcane) {
        return Some(EliteAffixVisual::Arcane);
    }
    if affixes.contains(&EnemyAffix::Frozen) || affixes.contains(&EnemyAffix::Jailer) {
        return Some(EliteAffixVisual::Frost);
    }
    if affixes.contains(&EnemyAffix::Molten) || affixes.contains(&EnemyAffix::Desecrator) {
        return Some(EliteAffixVisual::Ember);
    }
    if affixes.contains(&EnemyAffix::Vampiric) {
        return Some(EliteAffixVisual::Blood);
    }
    if affixes.contains(&EnemyAffix::Shielded) || affixes.contains(&EnemyAffix::Reflective) {
        return Some(EliteAffixVisual::Ward);
    }
    Some(EliteAffixVisual::Ember)
}

fn affix_visual_scale(affixes: &[EnemyAffix]) -> f32 {
    match affixes.len() {
        0 => 0.0,
        1 => 0.95,
        2..=3 => 1.08,
        _ => 1.22,
    }
}

fn affix_aura_spin_speed(affix_count: usize) -> f32 {
    0.55 + affix_count.min(6) as f32 * 0.12
}

fn affix_aura_pulse_speed(affix_count: usize) -> f32 {
    2.2 + affix_count.min(6) as f32 * 0.18
}

fn affix_aura_pulse_amount(affix_count: usize) -> f32 {
    0.055 + affix_count.min(6) as f32 * 0.008
}

fn affix_aura_pulse_factor(elapsed: f32, pulse_speed: f32, pulse_amount: f32) -> f32 {
    1.0 + (elapsed * pulse_speed).sin() * pulse_amount
}

fn affix_visual_light_color(visual: EliteAffixVisual) -> Color {
    match visual {
        EliteAffixVisual::Arcane => Color::srgb(0.18, 0.48, 1.0),
        EliteAffixVisual::Frost => Color::srgb(0.42, 0.78, 1.0),
        EliteAffixVisual::Ember => Color::srgb(1.0, 0.24, 0.04),
        EliteAffixVisual::Blood => Color::srgb(0.72, 0.03, 0.02),
        EliteAffixVisual::Ward => Color::srgb(1.0, 0.72, 0.18),
    }
}

fn affix_visual_light_intensity(affix_count: usize) -> f32 {
    130.0 + affix_count.min(8) as f32 * 32.0
}

fn affix_visual_light_range(affix_count: usize) -> f32 {
    2.4 + affix_count.min(8) as f32 * 0.22
}

fn nemesis_affixes() -> Vec<EnemyAffix> {
    vec![
        EnemyAffix::Frenzied,
        EnemyAffix::Vampiric,
        EnemyAffix::Jailer,
        EnemyAffix::Frozen,
        EnemyAffix::Desecrator,
        EnemyAffix::Reflective,
    ]
}

fn nemesis_spawn_position(chapter: ChapterId, checkpoint: Vec3) -> Vec3 {
    clamp_world_translation(checkpoint + chapter_world_position(chapter, Vec3::new(2.6, 0.0, -1.8)))
}

fn display_name_with_affixes(base: &str, affixes: &[EnemyAffix]) -> String {
    if affixes.is_empty() {
        return base.to_string();
    }
    let labels = affixes
        .iter()
        .map(|affix| affix.label())
        .collect::<Vec<_>>()
        .join(" ");
    format!("{labels} {base}")
}

fn scale_reward(value: u32, multiplier: f32) -> u32 {
    ((value as f32) * multiplier).round().max(1.0) as u32
}

#[allow(clippy::too_many_arguments)]
fn enemy_ai(
    time: Res<Time>,
    difficulty: Res<DifficultySettings>,
    mut commands: Commands,
    mut feedback: EnemyFeedbackWriters,
    vfx: Res<EnemyRuntimeVfx>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut enemies: EnemyAiQuery,
    mut player: PlayerHealthQuery,
    mut minions: MinionDamageQuery,
) {
    let Ok((
        player_entity,
        player_transform,
        mut player_health,
        mut barrier,
        evade,
        player_equipment,
        elixir,
    )) = player.single_mut()
    else {
        return;
    };
    let player_armor = total_armor(player_equipment, elixir);

    for (
        entity,
        mut transform,
        mut health,
        mut enemy,
        mut animation,
        chilled,
        staggered,
        treasure_fiend,
        hit_react,
        attack_warning,
        mut skill_cycle,
    ) in &mut enemies
    {
        let mut hit_react_power = 0.0;
        if let Some(mut hit_react) = hit_react {
            let recoil = enemy_hit_recoil_step(&hit_react, time.delta_secs());
            hit_react.timer.tick(time.delta());
            hit_react_power =
                (1.0 - hit_react.timer.elapsed_secs() / hit_react.duration_secs).clamp(0.0, 1.0);
            transform.translation += recoil;
            transform.translation = clamp_world_translation(transform.translation);
            if hit_react.timer.is_finished() {
                commands.queue(move |world: &mut World| remove_expired_hit_react(world, entity));
            }
        }

        let mut attack_warning_active = false;
        let mut attack_warning_pose = None;
        let mut attack_warning_progress = 0.0;
        if let Some(mut attack_warning) = attack_warning {
            attack_warning.timer.tick(time.delta());
            attack_warning_active = !attack_warning.timer.is_finished();
            if attack_warning_active {
                attack_warning_progress = attack_warning.progress();
                attack_warning_pose = Some(enemy_warning_pose(
                    &attack_warning.kind,
                    attack_warning_progress,
                ));
            }
            if attack_warning.timer.is_finished()
                && let Ok(mut entity_commands) = commands.get_entity(entity)
            {
                entity_commands.remove::<EnemyAttackWarning>();
            }
        }

        enemy.attack_timer.tick(time.delta());
        if staggered.is_some() {
            animate_enemy(
                &time,
                &mut transform,
                &mut animation,
                &enemy.attack_kind,
                EnemyAnimateInput {
                    moving: false,
                    hit_react_power,
                    warning_progress: attack_warning_progress,
                    warning_pose: attack_warning_pose,
                },
            );
            continue;
        }
        let pet_target = minion_attack_target(
            transform.translation,
            player_transform.translation,
            &minions,
        );
        let target_position =
            pet_target.map_or(player_transform.translation, |(_, position)| position);
        let to_player = target_position - transform.translation;
        let flat_to_player = Vec3::new(to_player.x, 0.0, to_player.z);
        let distance = flat_to_player.length();

        if let Some(mut treasure_fiend) = treasure_fiend {
            match handle_treasure_fiend_ai(
                &time,
                &mut transform,
                &enemy,
                &mut treasure_fiend,
                flat_to_player,
                distance,
                chilled,
            ) {
                TreasureFiendState::Dormant => {}
                TreasureFiendState::Alerted => {
                    feedback.combat_events.write(CombatEvent {
                        text: "A treasure imp bolts with a reliquary cache".to_string(),
                    });
                    animate_enemy(
                        &time,
                        &mut transform,
                        &mut animation,
                        &enemy.attack_kind,
                        EnemyAnimateInput {
                            moving: true,
                            hit_react_power,
                            warning_progress: attack_warning_progress,
                            warning_pose: attack_warning_pose,
                        },
                    );
                    continue;
                }
                TreasureFiendState::Fleeing => {
                    animate_enemy(
                        &time,
                        &mut transform,
                        &mut animation,
                        &enemy.attack_kind,
                        EnemyAnimateInput {
                            moving: true,
                            hit_react_power,
                            warning_progress: attack_warning_progress,
                            warning_pose: attack_warning_pose,
                        },
                    );
                    continue;
                }
                TreasureFiendState::Escaped => {
                    feedback.combat_events.write(CombatEvent {
                        text: "The treasure imp escaped beyond the valley".to_string(),
                    });
                    if let Ok(mut entity_commands) = commands.get_entity(entity) {
                        entity_commands.try_despawn();
                    }
                    continue;
                }
            }
        }

        if transform.translation.distance(player_transform.translation) > enemy.aggro_range {
            continue;
        }

        let mut moved = false;
        if let Some(plan) =
            enemy_movement_plan(&enemy, transform.translation, flat_to_player, distance)
        {
            transform.translation += plan.movement_direction
                * enemy.move_speed
                * plan.speed_multiplier
                * chilled_slow_multiplier(chilled)
                * time.delta_secs();
            transform.translation = clamp_world_translation(transform.translation);
            if plan.facing_direction.length_squared() > 0.0 {
                animation.facing_yaw = plan.facing_direction.x.atan2(plan.facing_direction.z);
                moved = true;
            }
        } else if enemy.attack_timer.is_finished() {
            if let Ok(mut entity_commands) = commands.get_entity(entity) {
                entity_commands.remove::<EnemyAttackWarning>();
            }
            let role = enemy_combat_role(&enemy.id);
            let signature_attack = skill_cycle.advance();
            animation.trigger_attack(match enemy.attack_kind {
                EnemyAttackKind::Melee => 0.32,
                EnemyAttackKind::Projectile => 0.42,
                EnemyAttackKind::Shockwave => 0.52,
            });
            spawn_enemy_attack_impact(
                &mut commands,
                &vfx,
                transform.translation,
                flat_to_player.normalize_or_zero(),
                &enemy.attack_kind,
                enemy.attack_range,
            );
            match enemy.attack_kind {
                EnemyAttackKind::Melee => {
                    let raw_damage = enemy.attack_damage
                        * signature_attack_damage_multiplier(role, signature_attack);
                    if let Some((pet, position)) = pet_target {
                        if let Ok((_, _, mut vitality)) = minions.get_mut(pet) {
                            let taken = vitality.damage(raw_damage);
                            spawn_player_damage_impact(
                                &mut commands,
                                &mut meshes,
                                &mut materials,
                                transform.translation,
                                position,
                                DamageTaken {
                                    absorbed: 0.0,
                                    health_damage: taken,
                                },
                            );
                        }
                        enemy.attack_timer.reset();
                        continue;
                    }
                    let damage = mitigated_damage(raw_damage, player_armor);
                    let taken = apply_player_damage_with_evade(
                        &mut player_health,
                        &mut barrier,
                        evade,
                        damage,
                    );
                    emit_player_damage_feedback(
                        &mut feedback.combat_events,
                        &mut feedback.floating_text,
                        &mut feedback.screen_shake,
                        PlayerDamageFeedback {
                            source: &enemy.display_name,
                            taken,
                            player_health: &player_health,
                            position: player_transform.translation,
                        },
                    );
                    spawn_player_damage_impact(
                        &mut commands,
                        &mut meshes,
                        &mut materials,
                        transform.translation,
                        player_transform.translation,
                        taken,
                    );
                    trigger_player_hit_react(&mut commands, player_entity, taken);
                    trigger_vampiric_heal(&enemy, &mut health);
                    if signature_attack {
                        let signature = match role {
                            EnemyCombatRole::Vanguard => Some(("Shield Bash", 0.26, 0.68)),
                            EnemyCombatRole::Skirmisher => Some(("Pouncing Rend", 0.42, 0.52)),
                            EnemyCombatRole::Nemesis => Some(("Soul Hunt", 0.48, 0.46)),
                            _ => None,
                        };
                        if let Some((label, duration_secs, speed_multiplier)) = signature {
                            commands.entity(player_entity).try_insert(Jailed {
                                timer: Timer::from_seconds(duration_secs, TimerMode::Once),
                                speed_multiplier,
                            });
                            feedback.combat_events.write(CombatEvent {
                                text: format!("{} used {label}", enemy.display_name),
                            });
                        }
                    }
                }
                EnemyAttackKind::Projectile => spawn_role_projectile_volley(
                    &mut commands,
                    &vfx,
                    transform.translation + Vec3::Y,
                    flat_to_player.normalize_or_zero(),
                    enemy.attack_damage,
                    &enemy,
                    role,
                    signature_attack,
                    difficulty.current,
                ),
                EnemyAttackKind::Shockwave => {
                    let signature_bonus = if signature_attack { 1.10 } else { 0.0 };
                    let blast_radius = enemy.attack_range + 0.85 + signature_bonus;
                    for (_, pose, mut vitality) in &mut minions {
                        if vitality.current > 0.0
                            && pose.translation.distance(transform.translation) <= blast_radius
                        {
                            vitality.damage(
                                enemy.attack_damage
                                    * signature_attack_damage_multiplier(role, signature_attack),
                            );
                        }
                    }
                    if player_transform.translation.distance(transform.translation) <= blast_radius
                    {
                        let raw_damage = enemy.attack_damage
                            * signature_attack_damage_multiplier(role, signature_attack);
                        let damage = mitigated_damage(raw_damage, player_armor);
                        let taken = apply_player_damage_with_evade(
                            &mut player_health,
                            &mut barrier,
                            evade,
                            damage,
                        );
                        emit_player_damage_feedback(
                            &mut feedback.combat_events,
                            &mut feedback.floating_text,
                            &mut feedback.screen_shake,
                            PlayerDamageFeedback {
                                source: &format!("{} shockwave", enemy.display_name),
                                taken,
                                player_health: &player_health,
                                position: player_transform.translation,
                            },
                        );
                        spawn_player_damage_impact(
                            &mut commands,
                            &mut meshes,
                            &mut materials,
                            transform.translation,
                            player_transform.translation,
                            taken,
                        );
                        trigger_player_hit_react(&mut commands, player_entity, taken);
                        trigger_vampiric_heal(&enemy, &mut health);
                    }
                    spawn_shockwave_marker(
                        &mut commands,
                        &mut meshes,
                        &mut materials,
                        transform.translation,
                    );
                    if signature_attack && role == EnemyCombatRole::Juggernaut {
                        spawn_juggernaut_quake_line(
                            &mut commands,
                            &mut meshes,
                            &mut materials,
                            transform.translation,
                            flat_to_player.normalize_or_zero(),
                            enemy.attack_damage * 0.62,
                        );
                        feedback.combat_events.write(CombatEvent {
                            text: format!(
                                "{} split the ground with a quake line",
                                enemy.display_name
                            ),
                        });
                    }
                }
            }
            let release_control_affix = signature_attack && !matches!(role, EnemyCombatRole::Boss);
            if release_control_affix && enemy.affixes.contains(&EnemyAffix::Arcane) {
                spawn_arcane_sentry(
                    &mut commands,
                    &mut meshes,
                    &mut materials,
                    player_transform.translation,
                    enemy.attack_damage * 0.55,
                );
            }
            if release_control_affix && enemy.affixes.contains(&EnemyAffix::Jailer) {
                spawn_jailer_prison(
                    &mut commands,
                    &mut meshes,
                    &mut materials,
                    player_transform.translation,
                    enemy.attack_damage * 0.35,
                );
                feedback.combat_events.write(CombatEvent {
                    text: "Jailer affix: ash chains are forming—move".to_string(),
                });
            }
            if release_control_affix && enemy.affixes.contains(&EnemyAffix::Frozen) {
                spawn_frozen_orb(
                    &mut commands,
                    &mut meshes,
                    &mut materials,
                    player_transform.translation,
                    enemy.attack_damage * 0.42,
                );
                feedback.combat_events.write(CombatEvent {
                    text: "Frozen affix: frost burst forming".to_string(),
                });
            }
            if release_control_affix && enemy.affixes.contains(&EnemyAffix::Desecrator) {
                spawn_desecrator_pool(
                    &mut commands,
                    &mut meshes,
                    &mut materials,
                    player_transform.translation,
                    enemy.attack_damage * 0.48,
                );
                feedback.combat_events.write(CombatEvent {
                    text: "Desecrator affix: corrupted ground opens".to_string(),
                });
            }
            if signature_attack && role == EnemyCombatRole::Warden {
                spawn_seal_warden_rune(
                    &mut commands,
                    &mut meshes,
                    &mut materials,
                    player_transform.translation,
                    enemy.attack_damage * 0.58,
                );
                feedback.combat_events.write(CombatEvent {
                    text: "Seal Warden: violet rune forming underfoot".to_string(),
                });
            }
            enemy.attack_timer.reset();
        } else {
            let remaining = enemy.attack_timer.remaining_secs();
            let warning_window = enemy_attack_warning_window(&enemy.attack_kind)
                * difficulty.current.enemy_warning_window_multiplier();
            if !attack_warning_active && remaining > 0.08 && remaining <= warning_window {
                spawn_enemy_attack_warning(
                    &mut commands,
                    &vfx,
                    EnemyAttackWarningSpec {
                        origin: transform.translation,
                        direction: flat_to_player.normalize_or_zero(),
                        kind: &enemy.attack_kind,
                        attack_range: enemy.attack_range,
                        duration_secs: remaining + 0.1,
                    },
                );
                if let Ok(mut entity_commands) = commands.get_entity(entity) {
                    entity_commands.try_insert(EnemyAttackWarning::new(
                        enemy.attack_kind.clone(),
                        remaining + 0.12,
                    ));
                }
            }
        }
        animate_enemy(
            &time,
            &mut transform,
            &mut animation,
            &enemy.attack_kind,
            EnemyAnimateInput {
                moving: moved,
                hit_react_power,
                warning_progress: attack_warning_progress,
                warning_pose: attack_warning_pose,
            },
        );
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct EnemyMovementPlan {
    movement_direction: Vec3,
    facing_direction: Vec3,
    speed_multiplier: f32,
}

fn enemy_movement_plan(
    enemy: &Enemy,
    enemy_position: Vec3,
    flat_to_player: Vec3,
    distance: f32,
) -> Option<EnemyMovementPlan> {
    let to_player = horizontal_ai_direction(flat_to_player);
    let role = enemy_combat_role(&enemy.id);
    if matches!(
        role,
        EnemyCombatRole::Marksman | EnemyCombatRole::Caster | EnemyCombatRole::Warden
    ) && distance < ranged_min_spacing(enemy, role)
    {
        let away = -to_player;
        let strafe = marksman_strafe_direction(enemy_position, to_player);
        return Some(EnemyMovementPlan {
            movement_direction: (away * 0.82 + strafe * 0.46).normalize_or_zero(),
            facing_direction: to_player,
            speed_multiplier: if role == EnemyCombatRole::Marksman {
                1.16
            } else {
                0.92
            },
        });
    }

    let desired_range = enemy_desired_attack_range(enemy);
    if distance > desired_range {
        let movement_direction = if role == EnemyCombatRole::Skirmisher {
            let strafe = marksman_strafe_direction(enemy_position, to_player);
            (to_player * 0.90 + strafe * 0.34).normalize_or_zero()
        } else {
            to_player
        };
        return Some(EnemyMovementPlan {
            movement_direction,
            facing_direction: to_player,
            speed_multiplier: enemy_approach_speed_multiplier(enemy),
        });
    }

    None
}

fn enemy_desired_attack_range(enemy: &Enemy) -> f32 {
    match enemy_combat_role(&enemy.id) {
        EnemyCombatRole::Skirmisher => enemy.attack_range * 0.88,
        EnemyCombatRole::Marksman => enemy.attack_range * 0.88,
        EnemyCombatRole::Caster | EnemyCombatRole::Warden => enemy.attack_range * 0.82,
        EnemyCombatRole::Juggernaut => enemy.attack_range * 0.92,
        _ => match enemy.attack_kind {
            EnemyAttackKind::Melee | EnemyAttackKind::Shockwave => enemy.attack_range,
            EnemyAttackKind::Projectile => enemy.attack_range * 0.82,
        },
    }
}

#[cfg(test)]
fn marksman_min_spacing(enemy: &Enemy) -> f32 {
    ranged_min_spacing(enemy, EnemyCombatRole::Marksman)
}

fn ranged_min_spacing(enemy: &Enemy, role: EnemyCombatRole) -> f32 {
    enemy.attack_range
        * match role {
            EnemyCombatRole::Marksman => 0.46,
            EnemyCombatRole::Caster => 0.38,
            EnemyCombatRole::Warden => 0.34,
            _ => 0.30,
        }
}

fn enemy_approach_speed_multiplier(enemy: &Enemy) -> f32 {
    match enemy_combat_role(&enemy.id) {
        EnemyCombatRole::Skirmisher => 1.18,
        EnemyCombatRole::Juggernaut => 0.74,
        _ => 1.0,
    }
}

fn marksman_strafe_direction(enemy_position: Vec3, to_player: Vec3) -> Vec3 {
    let right = Vec3::new(to_player.z, 0.0, -to_player.x);
    if (enemy_position.x + enemy_position.z).sin() >= 0.0 {
        right
    } else {
        -right
    }
}

fn horizontal_ai_direction(direction: Vec3) -> Vec3 {
    let direction = Vec3::new(direction.x, 0.0, direction.z).normalize_or_zero();
    if direction.length_squared() > 0.0 {
        direction
    } else {
        Vec3::Z
    }
}

#[derive(Debug, Clone, Copy)]
struct EnemyAnimateInput {
    moving: bool,
    hit_react_power: f32,
    warning_progress: f32,
    warning_pose: Option<EnemyAttackPose>,
}

fn animate_enemy(
    time: &Time,
    transform: &mut Transform,
    animation: &mut EnemyAnimation,
    attack_kind: &EnemyAttackKind,
    input: EnemyAnimateInput,
) {
    animation.moving = input.moving;
    animation.warning_progress = input.warning_progress;
    animation.hit_react_power = input.hit_react_power;
    animation.attack_timer.tick(time.delta());
    if input.moving {
        animation.walk_phase += time.delta_secs() * 6.4;
    } else {
        animation.walk_phase *= 0.86;
    }
    let attack_progress = if animation.attack_timer.is_finished() {
        1.0
    } else {
        (animation.attack_timer.elapsed_secs() / animation.attack_duration_secs).clamp(0.0, 1.0)
    };
    let attack_drive = if animation.attack_timer.is_finished() {
        0.0
    } else {
        (attack_progress * std::f32::consts::PI).sin()
    };
    let attack_pose = enemy_attack_pose(attack_kind, attack_progress, attack_drive);
    let warning_pose = input.warning_pose.unwrap_or_else(EnemyAttackPose::identity);
    let walk_sway = if input.moving {
        animation.walk_phase.sin()
    } else {
        0.0
    };
    let walk_bob = if input.moving {
        animation.walk_phase.sin().abs() * 0.052
    } else {
        0.0
    };
    let hit_snap = (input.hit_react_power * std::f32::consts::PI)
        .sin()
        .max(0.0);
    transform.translation.y = walk_bob + attack_pose.lift + warning_pose.lift + hit_snap * 0.105;
    transform.scale = Vec3::new(
        animation.base_scale * (attack_pose.scale.x * warning_pose.scale.x + hit_snap * 0.16),
        animation.base_scale
            * (attack_pose.scale.y * warning_pose.scale.y + walk_bob * 0.64 - hit_snap * 0.11),
        animation.base_scale * (attack_pose.scale.z * warning_pose.scale.z + hit_snap * 0.10),
    );
    transform.rotation = Quat::from_rotation_y(animation.facing_yaw)
        * Quat::from_rotation_x(attack_pose.pitch + warning_pose.pitch)
        * Quat::from_rotation_z(
            0.085 * walk_sway + attack_pose.roll + warning_pose.roll + 0.24 * hit_snap,
        );
}

// Bind the actual authored mesh pivots after a glTF scene arrives. No proxy
// weapons, duplicate feet, or persistent VFX are spawned for locomotion.
#[allow(clippy::type_complexity)]
fn bind_enemy_limbs(
    mut commands: Commands,
    nodes: Query<(Entity, &Name, &Transform), (Added<Name>, Without<EnemyLimb>)>,
    parents: Query<&ChildOf>,
    enemies: Query<(), With<Enemy>>,
) {
    for (entity, name, rest) in &nodes {
        let Some(kind) = EnemyLimbKind::from_node_name(name.as_str()) else {
            continue;
        };
        let mut current = entity;
        while let Ok(parent) = parents.get(current) {
            current = parent.parent();
            if enemies.contains(current) {
                commands.entity(entity).insert(EnemyLimb {
                    kind,
                    owner: current,
                    rest: *rest,
                });
                break;
            }
        }
    }
}

fn animate_enemy_limbs_system(enemies: EnemyLimbSourceQuery, mut limbs: EnemyLimbQuery) {
    for (limb, mut transform) in &mut limbs {
        let Ok((_, enemy, animation)) = enemies.get(limb.owner) else {
            continue;
        };
        *transform = limb.rest;
        transform.rotation = limb.rest.rotation
            * enemy_limb_rotation(
                limb.kind,
                &enemy.attack_kind,
                animation.moving,
                animation.walk_phase,
                enemy_attack_progress(animation),
                animation.warning_progress,
            );
    }
}

fn enemy_attack_progress(animation: &EnemyAnimation) -> f32 {
    if animation.attack_timer.is_finished() {
        1.0
    } else {
        (animation.attack_timer.elapsed_secs() / animation.attack_duration_secs.max(0.001))
            .clamp(0.0, 1.0)
    }
}

fn enemy_limb_rotation(
    kind: EnemyLimbKind,
    attack_kind: &EnemyAttackKind,
    moving: bool,
    walk_phase: f32,
    attack_progress: f32,
    warning_progress: f32,
) -> Quat {
    let stride = if moving {
        walk_phase.sin() * kind.side()
    } else {
        0.0
    };
    let drive = if attack_progress >= 1.0 {
        0.0
    } else {
        (attack_progress.clamp(0.0, 1.0) * std::f32::consts::PI).sin()
    };
    let warning = warning_progress.clamp(0.0, 1.0);
    let pitch = match kind {
        EnemyLimbKind::LeftLeg | EnemyLimbKind::RightLeg => stride * 0.46,
        EnemyLimbKind::AttackHand => match attack_kind {
            EnemyAttackKind::Melee => -stride * 0.24 - warning * 1.25 + drive * 1.45,
            EnemyAttackKind::Projectile => -stride * 0.18 - warning * 0.85 - drive * 0.40,
            EnemyAttackKind::Shockwave => -stride * 0.22 - warning * 1.5 + drive * 1.1,
        },
        EnemyLimbKind::OffHand => -stride * 0.24 - warning * 0.25 - drive * 0.30,
    };
    Quat::from_rotation_x(pitch)
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct EnemyAttackPose {
    lift: f32,
    pitch: f32,
    roll: f32,
    scale: Vec3,
}

impl EnemyAttackPose {
    fn identity() -> Self {
        Self {
            lift: 0.0,
            pitch: 0.0,
            roll: 0.0,
            scale: Vec3::ONE,
        }
    }
}

fn enemy_attack_pose(
    kind: &EnemyAttackKind,
    attack_progress: f32,
    attack_drive: f32,
) -> EnemyAttackPose {
    if attack_drive <= 0.0 {
        return EnemyAttackPose::identity();
    }
    let impact = (1.0 - (attack_progress - 0.56).abs() / 0.34).clamp(0.0, 1.0);
    match kind {
        EnemyAttackKind::Melee => EnemyAttackPose {
            lift: attack_drive * 0.070,
            pitch: -0.28 * attack_drive - 0.24 * impact,
            roll: 0.10 * attack_drive,
            scale: Vec3::new(
                1.0 + 0.10 * impact,
                1.0 - 0.08 * impact,
                1.0 + 0.24 * impact,
            ),
        },
        EnemyAttackKind::Projectile => EnemyAttackPose {
            lift: attack_drive * 0.125,
            pitch: 0.24 * attack_drive - 0.12 * impact,
            roll: -0.14 * attack_drive,
            scale: Vec3::new(
                1.0 + 0.08 * attack_drive,
                1.0 + 0.10 * attack_drive,
                1.0 - 0.10 * impact,
            ),
        },
        EnemyAttackKind::Shockwave => EnemyAttackPose {
            lift: attack_drive * 0.045,
            pitch: -0.66 * impact,
            roll: 0.0,
            scale: Vec3::new(
                1.0 + 0.25 * attack_drive,
                1.0 - 0.18 * impact,
                1.0 + 0.25 * attack_drive,
            ),
        },
    }
}

fn enemy_warning_pose(kind: &EnemyAttackKind, warning_progress: f32) -> EnemyAttackPose {
    let charge = warning_progress.clamp(0.0, 1.0);
    let pulse = (charge * std::f32::consts::PI).sin() * 0.24;
    match kind {
        EnemyAttackKind::Melee => EnemyAttackPose {
            lift: 0.022 * charge,
            pitch: -0.24 * charge,
            roll: 0.16 * charge,
            scale: Vec3::new(
                1.0 + pulse * 0.55,
                1.0 - charge * 0.080,
                1.0 + charge * 0.25,
            ),
        },
        EnemyAttackKind::Projectile => EnemyAttackPose {
            lift: 0.16 * charge,
            pitch: 0.28 * charge,
            roll: -0.18 * charge,
            scale: Vec3::new(1.0 + charge * 0.08, 1.0 + pulse * 0.55, 1.0 - charge * 0.11),
        },
        EnemyAttackKind::Shockwave => EnemyAttackPose {
            lift: 0.010 * charge,
            pitch: -0.34 * charge,
            roll: 0.0,
            scale: Vec3::new(
                1.0 + charge * 0.30,
                1.0 - charge * 0.18,
                1.0 + charge * 0.30,
            ),
        },
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum TreasureFiendState {
    Dormant,
    Alerted,
    Fleeing,
    Escaped,
}

fn handle_treasure_fiend_ai(
    time: &Time,
    transform: &mut Transform,
    enemy: &Enemy,
    treasure_fiend: &mut TreasureFiend,
    flat_to_player: Vec3,
    distance: f32,
    chilled: Option<&Chilled>,
) -> TreasureFiendState {
    if !treasure_fiend.alerted && distance > enemy.aggro_range {
        return TreasureFiendState::Dormant;
    }
    let alerted_this_frame = !treasure_fiend.alerted;
    if !treasure_fiend.alerted {
        treasure_fiend.alerted = true;
    }
    treasure_fiend.escape_timer.tick(time.delta());
    if treasure_fiend.escape_timer.is_finished() {
        return TreasureFiendState::Escaped;
    }

    let direction = treasure_flee_direction(flat_to_player);
    transform.translation +=
        direction * enemy.move_speed * chilled_slow_multiplier(chilled) * time.delta_secs();
    transform.translation = clamp_world_translation(transform.translation);
    if direction.length_squared() > 0.0 {
        transform.rotation = Quat::from_rotation_y(direction.x.atan2(direction.z));
    }
    if alerted_this_frame {
        TreasureFiendState::Alerted
    } else {
        TreasureFiendState::Fleeing
    }
}

fn treasure_flee_direction(flat_to_player: Vec3) -> Vec3 {
    -flat_to_player.normalize_or_zero()
}

fn trigger_vampiric_heal(enemy: &Enemy, health: &mut Health) {
    if enemy.affixes.contains(&EnemyAffix::Vampiric) {
        health.current = (health.current + enemy.attack_damage * 0.65).min(health.max);
    }
}

fn chilled_slow_multiplier(chilled: Option<&Chilled>) -> f32 {
    chilled
        .map(|chilled| chilled.slow_multiplier)
        .unwrap_or(1.0)
        .clamp(0.25, 1.0)
}

fn tick_chilled(
    time: Res<Time>,
    mut commands: Commands,
    mut chilled: Query<(Entity, &mut Chilled)>,
) {
    for (entity, mut chilled) in &mut chilled {
        chilled.timer.tick(time.delta());
        if chilled.timer.is_finished() {
            commands.entity(entity).try_remove::<Chilled>();
        }
    }
}

fn tick_staggered(
    time: Res<Time>,
    mut commands: Commands,
    mut staggered: Query<(Entity, &mut Staggered)>,
) {
    for (entity, mut staggered) in &mut staggered {
        staggered.timer.tick(time.delta());
        if staggered.timer.is_finished() {
            commands.entity(entity).try_remove::<Staggered>();
        }
    }
}

fn tick_elite_affix_auras(
    time: Res<Time>,
    mut auras: Query<(&mut Transform, &mut EliteAffixAura)>,
    mut lights: Query<(&mut PointLight, &mut EliteAffixAuraLight)>,
) {
    let delta_secs = time.delta_secs();
    for (mut transform, mut aura) in &mut auras {
        aura.elapsed += delta_secs;
        transform.rotate_y(aura.spin_speed * delta_secs);
        let scale = aura.base_scale
            * affix_aura_pulse_factor(aura.elapsed, aura.pulse_speed, aura.pulse_amount);
        transform.scale = Vec3::splat(scale);
    }
    for (mut light, mut aura_light) in &mut lights {
        aura_light.elapsed += delta_secs;
        light.intensity = aura_light.base_intensity
            * affix_aura_pulse_factor(
                aura_light.elapsed,
                aura_light.pulse_speed,
                aura_light.pulse_amount,
            );
    }
}

#[derive(SystemParam)]
struct BossPhaseResources<'w, 's> {
    time: Res<'w, Time>,
    commands: Commands<'w, 's>,
    assets: Res<'w, GameAssets>,
    catalog: Res<'w, EnemyCatalog>,
    difficulty: Res<'w, DifficultySettings>,
    modifier: Res<'w, ChapterModifier>,
    active_chapter: Res<'w, ActiveChapter>,
    stats: ResMut<'w, RunStats>,
}

fn boss_phase_transition(
    mut resources: BossPhaseResources,
    mut combat_events: MessageWriter<CombatEvent>,
    mut bosses: Query<(&Transform, &mut Health, &mut Enemy, &mut BossPhase)>,
) {
    for (transform, mut health, mut enemy, mut phase) in &mut bosses {
        if phase.phase_two_started {
            if !phase.enrage_started {
                phase.enrage_timer.tick(resources.time.delta());
                if boss_enrage_should_trigger(&phase) {
                    apply_boss_enrage(&mut enemy, &mut phase);
                    resources.stats.boss_enraged = true;
                    spawn_boss_reinforcement_wave(
                        &mut resources.commands,
                        BossReinforcementWaveContext {
                            assets: &resources.assets,
                            catalog: &resources.catalog,
                            origin: transform.translation,
                            difficulty: resources.difficulty.current,
                            modifier: *resources.modifier,
                            chapter: resources.active_chapter.0,
                        },
                        boss_enrage_reinforcements(),
                    );
                    combat_events.write(CombatEvent {
                        text: boss_enrage_event_text_for(resources.active_chapter.0),
                    });
                }
            }
            continue;
        }
        if !boss_phase_should_trigger(&health) {
            continue;
        }

        phase.phase_two_started = true;
        phase.enrage_timer = Timer::from_seconds(BOSS_ENRAGE_DELAY_SECS, TimerMode::Once);
        if !enemy.affixes.contains(&EnemyAffix::Frenzied) {
            enemy.affixes.push(EnemyAffix::Frenzied);
            enemy.display_name =
                display_name_with_affixes(resources.active_chapter.0.boss_name(), &enemy.affixes);
        }
        enemy.attack_damage *= 1.18;
        enemy.move_speed *= 1.15;
        health.current = (health.current + health.max * 0.08).min(health.max);

        spawn_boss_reinforcement_wave(
            &mut resources.commands,
            BossReinforcementWaveContext {
                assets: &resources.assets,
                catalog: &resources.catalog,
                origin: transform.translation,
                difficulty: resources.difficulty.current,
                modifier: *resources.modifier,
                chapter: resources.active_chapter.0,
            },
            boss_phase_two_reinforcements(),
        );
        combat_events.write(CombatEvent {
            text: boss_phase_two_event_text_for(resources.active_chapter.0, BOSS_ENRAGE_DELAY_SECS),
        });
    }
}

fn spawn_boss_reinforcement_wave(
    commands: &mut Commands,
    context: BossReinforcementWaveContext<'_>,
    wave: impl IntoIterator<Item = BossReinforcementWaveSpec>,
) {
    for reinforcement in wave {
        spawn_boss_reinforcement(
            commands,
            context.assets,
            context.catalog,
            BossReinforcementSpec {
                difficulty: context.difficulty,
                modifier: context.modifier,
                id: chapter_enemy_id(context.chapter, reinforcement.id),
                position: context.origin + reinforcement.offset,
                affixes: reinforcement.affixes,
            },
        );
    }
}

fn spawn_boss_reinforcement(
    commands: &mut Commands,
    assets: &GameAssets,
    catalog: &EnemyCatalog,
    spec: BossReinforcementSpec,
) {
    let Some(tuning) = catalog.get(spec.id) else {
        return;
    };
    let scene = enemy_scene(assets, &tuning.model);
    spawn_enemy(
        commands,
        assets,
        EnemySpawnSpec {
            scene,
            tuning,
            position: spec.position,
            difficulty: spec.difficulty,
            modifier: spec.modifier,
            affixes: spec.affixes,
        },
    );
}

struct BossReinforcementSpec {
    difficulty: Difficulty,
    modifier: ChapterModifier,
    id: &'static str,
    position: Vec3,
    affixes: Vec<EnemyAffix>,
}

struct BossReinforcementWaveSpec {
    id: &'static str,
    offset: Vec3,
    affixes: Vec<EnemyAffix>,
}

struct BossReinforcementWaveContext<'a> {
    assets: &'a GameAssets,
    catalog: &'a EnemyCatalog,
    origin: Vec3,
    difficulty: Difficulty,
    modifier: ChapterModifier,
    chapter: ChapterId,
}

fn boss_phase_two_reinforcements() -> [BossReinforcementWaveSpec; 3] {
    [
        BossReinforcementWaveSpec {
            id: "bone_stalker",
            offset: Vec3::new(-2.8, 0.0, -1.6),
            affixes: vec![EnemyAffix::Frenzied],
        },
        BossReinforcementWaveSpec {
            id: "ashen_marksman",
            offset: Vec3::new(0.2, 0.0, 2.8),
            affixes: vec![EnemyAffix::Arcane, EnemyAffix::Jailer],
        },
        BossReinforcementWaveSpec {
            id: "reliquary_brute",
            offset: Vec3::new(2.4, 0.0, -1.8),
            affixes: vec![
                EnemyAffix::Molten,
                EnemyAffix::Shielded,
                EnemyAffix::Frozen,
                EnemyAffix::Desecrator,
                EnemyAffix::Reflective,
            ],
        },
    ]
}

fn boss_enrage_reinforcements() -> [BossReinforcementWaveSpec; 2] {
    [
        BossReinforcementWaveSpec {
            id: "ashen_marksman",
            offset: Vec3::new(-2.2, 0.0, 2.8),
            affixes: vec![
                EnemyAffix::Arcane,
                EnemyAffix::Jailer,
                EnemyAffix::Reflective,
            ],
        },
        BossReinforcementWaveSpec {
            id: "reliquary_brute",
            offset: Vec3::new(2.2, 0.0, 2.6),
            affixes: vec![
                EnemyAffix::Molten,
                EnemyAffix::Desecrator,
                EnemyAffix::Frozen,
            ],
        },
    ]
}

fn boss_phase_should_trigger(health: &Health) -> bool {
    health.max > 0.0 && health.current <= health.max * 0.55
}

fn boss_enrage_should_trigger(phase: &BossPhase) -> bool {
    phase.phase_two_started && !phase.enrage_started && phase.enrage_timer.is_finished()
}
#[cfg(test)]
fn boss_phase_two_event_text(enrage_delay_secs: f32) -> String {
    boss_phase_two_event_text_for(ChapterId::AshenReliquary, enrage_delay_secs)
}

fn boss_phase_two_event_text_for(chapter: ChapterId, enrage_delay_secs: f32) -> String {
    format!(
        "Boss Phase II: BREAK {} before enrage {:.0}s | stagger opens EXECUTE WINDOW | kill stalker, marksman, brute",
        chapter.boss_short_name().to_ascii_uppercase(),
        enrage_delay_secs.max(0.0)
    )
}

fn boss_enrage_event_text() -> &'static str {
    "Ashen Enrage: floor burning | Primal cache downgraded | clear marksman/brute Arcane/Jailer/Desecrator pressure"
}

fn boss_enrage_event_text_for(chapter: ChapterId) -> String {
    match chapter {
        ChapterId::AshenReliquary => boss_enrage_event_text().to_string(),
        ChapterId::HollowBastion => {
            "Hollow Crescendo: soul fissures erupt | Primal cache downgraded | clear choir/reaver pressure"
                .to_string()
        }
    }
}

pub fn boss_stagger_summary(
    stagger: Option<&BossStagger>,
    staggered: Option<&Staggered>,
) -> String {
    if let Some(staggered) = staggered {
        return format!(
            "EXECUTE WINDOW {:.1}s x{:.2}",
            staggered.timer.remaining_secs(),
            staggered.damage_multiplier
        );
    }
    let Some(stagger) = stagger else {
        return "stagger unavailable".to_string();
    };
    format!(
        "stagger {}/{}",
        stagger.meter.round(),
        stagger.threshold.round()
    )
}

fn apply_boss_enrage(enemy: &mut Enemy, phase: &mut BossPhase) {
    if phase.enrage_started {
        return;
    }
    phase.enrage_started = true;
    enemy.attack_damage *= 1.22;
    enemy.move_speed *= 1.10;
    let faster_cooldown = (enemy.attack_timer.duration().as_secs_f32() * 0.72).max(0.75);
    enemy
        .attack_timer
        .set_duration(std::time::Duration::from_secs_f32(faster_cooldown));
}

pub fn boss_stagger_damage_multiplier(staggered: Option<&Staggered>) -> f32 {
    staggered
        .map(|staggered| staggered.damage_multiplier)
        .unwrap_or(1.0)
        .max(1.0)
}

pub fn add_boss_stagger(stagger: &mut BossStagger, damage: f32) -> bool {
    stagger.meter = (stagger.meter + damage.max(0.0)).min(stagger.threshold);
    if stagger.meter < stagger.threshold {
        return false;
    }
    stagger.meter = 0.0;
    true
}

pub fn boss_staggered() -> Staggered {
    Staggered {
        timer: Timer::from_seconds(BOSS_STAGGER_DURATION_SECS, TimerMode::Once),
        damage_multiplier: BOSS_STAGGER_DAMAGE_MULTIPLIER,
    }
}

fn despawn_enemies(
    mut commands: Commands,
    query: Query<Entity, (With<EnemyEntity>, Without<ChildOf>)>,
) {
    for entity in &query {
        if let Ok(mut entity_commands) = commands.get_entity(entity) {
            entity_commands.try_despawn();
        }
    }
}

fn enemy_attack_warning_window(kind: &EnemyAttackKind) -> f32 {
    match kind {
        EnemyAttackKind::Melee => 0.48,
        EnemyAttackKind::Projectile => 0.70,
        EnemyAttackKind::Shockwave => 0.90,
    }
}

fn enemy_attack_warning_reach(kind: &EnemyAttackKind, attack_range: f32) -> f32 {
    match kind {
        EnemyAttackKind::Melee => attack_range.max(1.45),
        EnemyAttackKind::Projectile => attack_range.clamp(3.0, 7.2),
        EnemyAttackKind::Shockwave => attack_range + 1.35,
    }
}

fn enemy_attack_warning_width(kind: &EnemyAttackKind) -> f32 {
    match kind {
        EnemyAttackKind::Melee => 1.55,
        EnemyAttackKind::Projectile => 0.48,
        EnemyAttackKind::Shockwave => 0.0,
    }
}

fn enemy_attack_warning_forward_offset(kind: &EnemyAttackKind, reach: f32) -> f32 {
    match kind {
        EnemyAttackKind::Melee => reach * 0.46,
        EnemyAttackKind::Projectile => reach * 0.5,
        EnemyAttackKind::Shockwave => 0.0,
    }
}

fn enemy_attack_kind_index(kind: &EnemyAttackKind) -> usize {
    match kind {
        EnemyAttackKind::Melee => 0,
        EnemyAttackKind::Projectile => 1,
        EnemyAttackKind::Shockwave => 2,
    }
}

fn enemy_attack_telegraph_pulse(progress: f32) -> f32 {
    let progress = progress.clamp(0.0, 1.0);
    1.0 + progress * 0.30 + (progress * std::f32::consts::TAU * 2.0).sin().abs() * 0.10
}

fn enemy_attack_warning_color(kind: &EnemyAttackKind) -> Color {
    match kind {
        EnemyAttackKind::Melee => Color::srgba(1.0, 0.05, 0.02, 0.68),
        EnemyAttackKind::Projectile => Color::srgba(1.0, 0.34, 0.04, 0.72),
        EnemyAttackKind::Shockwave => Color::srgba(1.0, 0.02, 0.02, 0.64),
    }
}

impl EnemyAttackTelegraph {
    fn new(duration_secs: f32, base_scale: Vec3) -> Self {
        let duration_secs = duration_secs.max(0.12);
        Self {
            timer: Timer::from_seconds(duration_secs, TimerMode::Once),
            duration_secs,
            base_scale,
        }
    }

    fn progress(&self) -> f32 {
        (self.timer.elapsed_secs() / self.duration_secs).clamp(0.0, 1.0)
    }
}

// Normalized lane perimeter: retain the full danger footprint without painting
// a solid luminous plank over the ground and the characters' feet.
fn attack_lane_outline_mesh() -> Mesh {
    let mut positions = Vec::with_capacity(24);
    for (x0, x1, z0, z1) in [
        (-0.5, -0.47, -0.5, 0.5),
        (0.47, 0.5, -0.5, 0.5),
        (-0.47, 0.47, -0.5, -0.485),
        (-0.47, 0.47, 0.485, 0.5),
    ] {
        let a = [x0, 0.0, z0];
        let b = [x0, 0.0, z1];
        let c = [x1, 0.0, z1];
        let d = [x1, 0.0, z0];
        positions.extend([a, b, c, a, c, d]);
    }
    let mut mesh = Mesh::new(bevy::mesh::PrimitiveTopology::TriangleList, default());
    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
    mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, vec![[0.0, 1.0, 0.0]; 24]);
    mesh
}

fn spawn_enemy_attack_warning(
    commands: &mut Commands,
    vfx: &EnemyRuntimeVfx,
    spec: EnemyAttackWarningSpec,
) {
    let material = vfx.warning_materials[enemy_attack_kind_index(spec.kind)].clone();
    let direction = spec.direction.normalize_or_zero();
    let yaw = direction.x.atan2(direction.z);
    let duration_secs = spec.duration_secs.max(0.12);
    let reach = enemy_attack_warning_reach(spec.kind, spec.attack_range);
    let width = enemy_attack_warning_width(spec.kind);
    let forward_offset = enemy_attack_warning_forward_offset(spec.kind, reach);

    match spec.kind {
        EnemyAttackKind::Melee => {
            let base_scale = Vec3::new(width, 1.0, reach);
            commands.spawn((
                Mesh3d(vfx.flat_box.clone()),
                MeshMaterial3d(material),
                Transform::from_translation(
                    spec.origin + direction * forward_offset + Vec3::Y * 0.035,
                )
                .with_rotation(Quat::from_rotation_y(yaw))
                .with_scale(base_scale),
                EnemyAttackTelegraph::new(duration_secs, base_scale),
                EnemyEntity,
                Name::new("Melee Attack Telegraph"),
            ));
        }
        EnemyAttackKind::Projectile => {
            let base_scale = Vec3::new(width, 1.0, reach);
            commands.spawn((
                Mesh3d(vfx.flat_box.clone()),
                MeshMaterial3d(material),
                Transform::from_translation(
                    spec.origin + direction * forward_offset + Vec3::Y * 0.045,
                )
                .with_rotation(Quat::from_rotation_y(yaw))
                .with_scale(base_scale),
                EnemyAttackTelegraph::new(duration_secs, base_scale),
                EnemyEntity,
                Name::new("Projectile Attack Telegraph"),
            ));
        }
        EnemyAttackKind::Shockwave => {
            let base_scale = Vec3::new(reach, 1.0, reach);
            commands.spawn((
                Mesh3d(vfx.flat_disc.clone()),
                MeshMaterial3d(material),
                Transform::from_translation(spec.origin + Vec3::Y * 0.04).with_scale(base_scale),
                EnemyAttackTelegraph::new(duration_secs, base_scale),
                EnemyEntity,
                Name::new("Shockwave Attack Telegraph"),
            ));
        }
    }
}

#[derive(Debug, Clone, Copy)]
struct EnemyAttackImpactStyle {
    color: Color,
    scale: Vec3,
    forward_offset: f32,
    y: f32,
    lifetime_secs: f32,
}

fn enemy_attack_impact_style(kind: &EnemyAttackKind, attack_range: f32) -> EnemyAttackImpactStyle {
    match kind {
        EnemyAttackKind::Melee => EnemyAttackImpactStyle {
            color: Color::srgba(1.0, 0.06, 0.025, 0.58),
            scale: Vec3::new(1.52, 1.0, attack_range.max(1.35)),
            forward_offset: attack_range.max(1.35) * 0.48,
            y: 0.11,
            lifetime_secs: 0.18,
        },
        EnemyAttackKind::Projectile => EnemyAttackImpactStyle {
            color: Color::srgba(1.0, 0.36, 0.045, 0.64),
            scale: Vec3::splat(0.44),
            forward_offset: 0.56,
            y: 1.04,
            lifetime_secs: 0.22,
        },
        EnemyAttackKind::Shockwave => EnemyAttackImpactStyle {
            color: Color::srgba(1.0, 0.12, 0.025, 0.46),
            scale: Vec3::new(attack_range + 1.15, 1.0, attack_range + 1.15),
            forward_offset: 0.0,
            y: 0.07,
            lifetime_secs: 0.26,
        },
    }
}

fn enemy_attack_impact_transform(
    origin: Vec3,
    direction: Vec3,
    kind: &EnemyAttackKind,
    attack_range: f32,
) -> Transform {
    let style = enemy_attack_impact_style(kind, attack_range);
    let direction = if direction.length_squared() > 0.0001 {
        direction.normalize()
    } else {
        Vec3::Z
    };
    Transform::from_translation(origin + direction * style.forward_offset + Vec3::Y * style.y)
        .with_rotation(Quat::from_rotation_y(direction.x.atan2(direction.z)))
        .with_scale(style.scale)
}

fn spawn_enemy_attack_impact(
    commands: &mut Commands,
    vfx: &EnemyRuntimeVfx,
    origin: Vec3,
    direction: Vec3,
    kind: &EnemyAttackKind,
    attack_range: f32,
) {
    let style = enemy_attack_impact_style(kind, attack_range);
    let material = vfx.impact_materials[enemy_attack_kind_index(kind)].clone();
    let mesh = match kind {
        EnemyAttackKind::Melee => vfx.flat_box.clone(),
        EnemyAttackKind::Projectile => vfx.impact_spark.clone(),
        EnemyAttackKind::Shockwave => vfx.flat_disc.clone(),
    };
    commands.spawn((
        Mesh3d(mesh),
        MeshMaterial3d(material),
        enemy_attack_impact_transform(origin, direction, kind, attack_range),
        EnemyProjectile {
            damage: 0.0,
            velocity: Vec3::ZERO,
            radius: 0.0,
            lifetime: Timer::from_seconds(style.lifetime_secs, TimerMode::Once),
            arm_timer: None,
            impact: ProjectileImpact::None,
        },
        EnemyEntity,
        Name::new(enemy_attack_impact_name(kind)),
    ));
}

fn enemy_attack_impact_name(kind: &EnemyAttackKind) -> &'static str {
    match kind {
        EnemyAttackKind::Melee => "Enemy Melee Impact",
        EnemyAttackKind::Projectile => "Enemy Cast Impact",
        EnemyAttackKind::Shockwave => "Enemy Shockwave Impact",
    }
}

fn tick_enemy_attack_telegraphs(
    time: Res<Time>,
    mut commands: Commands,
    mut telegraphs: Query<(Entity, &mut EnemyAttackTelegraph, &mut Transform)>,
) {
    for (entity, mut telegraph, mut transform) in &mut telegraphs {
        telegraph.timer.tick(time.delta());
        transform.scale = telegraph.base_scale * enemy_attack_telegraph_pulse(telegraph.progress());
        if telegraph.timer.is_finished()
            && let Ok(mut entity_commands) = commands.get_entity(entity)
        {
            entity_commands.try_despawn();
        }
    }
}

fn signature_attack_damage_multiplier(role: EnemyCombatRole, signature: bool) -> f32 {
    if !signature {
        return 1.0;
    }
    match role {
        EnemyCombatRole::Skirmisher => 1.42,
        EnemyCombatRole::Juggernaut => 1.28,
        EnemyCombatRole::Nemesis => 1.34,
        EnemyCombatRole::Vanguard => 1.16,
        EnemyCombatRole::Caster
        | EnemyCombatRole::Marksman
        | EnemyCombatRole::Warden
        | EnemyCombatRole::Treasure
        | EnemyCombatRole::Boss => 1.0,
    }
}

#[allow(clippy::too_many_arguments)]
fn spawn_role_projectile_volley(
    commands: &mut Commands,
    vfx: &EnemyRuntimeVfx,
    origin: Vec3,
    direction: Vec3,
    damage: f32,
    enemy: &Enemy,
    role: EnemyCombatRole,
    signature: bool,
    difficulty: Difficulty,
) {
    let is_hollow = matches!(
        enemy.id.as_str(),
        "choir_acolyte" | "choir_marksman" | "bell_warden"
    );
    let (angles, damage_multiplier, base_speed, impact, _color, name): (
        &[f32],
        f32,
        f32,
        ProjectileImpact,
        Color,
        &'static str,
    ) = match (role, signature, is_hollow) {
        (EnemyCombatRole::Marksman, true, true) => (
            &[-0.20, 0.0, 0.20],
            0.72,
            8.4,
            ProjectileImpact::None,
            Color::srgb(0.10, 0.88, 1.0),
            "Choir Trident Volley",
        ),
        (EnemyCombatRole::Marksman, true, false) => (
            &[-0.18, 0.0, 0.18],
            0.72,
            8.4,
            ProjectileImpact::None,
            Color::srgb(1.0, 0.62, 0.10),
            "Reliquary Trident Volley",
        ),
        (EnemyCombatRole::Marksman, false, true) => (
            &[0.0],
            1.0,
            8.1,
            ProjectileImpact::None,
            Color::srgb(0.10, 0.88, 1.0),
            "Choir Lance",
        ),
        (EnemyCombatRole::Marksman, false, false) => (
            &[0.0],
            1.0,
            8.1,
            ProjectileImpact::None,
            Color::srgb(1.0, 0.62, 0.10),
            "Reliquary Bolt",
        ),
        (EnemyCombatRole::Caster, true, true) => (
            &[-0.11, 0.11],
            0.78,
            6.7,
            ProjectileImpact::Arcane,
            Color::srgb(0.18, 0.72, 1.0),
            "Hollow Duet",
        ),
        (EnemyCombatRole::Caster, true, false) => (
            &[-0.10, 0.10],
            0.78,
            6.7,
            ProjectileImpact::Burning,
            Color::srgb(1.0, 0.22, 0.04),
            "Cinder Duet",
        ),
        (EnemyCombatRole::Caster, false, true) => (
            &[0.0],
            1.0,
            6.2,
            ProjectileImpact::Arcane,
            Color::srgb(0.18, 0.72, 1.0),
            "Hollow Note",
        ),
        (EnemyCombatRole::Warden, _, true) => (
            &[0.0],
            1.0,
            6.8,
            ProjectileImpact::Arcane,
            Color::srgb(0.46, 0.26, 1.0),
            "Bell Warden Note",
        ),
        (EnemyCombatRole::Warden, _, false) => (
            &[0.0],
            1.0,
            6.8,
            ProjectileImpact::Arcane,
            Color::srgb(0.66, 0.18, 1.0),
            "Seal Warden Orb",
        ),
        _ => (
            &[0.0],
            1.0,
            6.2,
            ProjectileImpact::Burning,
            Color::srgb(1.0, 0.22, 0.04),
            "Cinder Bolt",
        ),
    };

    for angle in angles {
        let shot_direction = Quat::from_rotation_y(*angle)
            .mul_vec3(direction)
            .normalize_or_zero();
        spawn_projectile(
            commands,
            vfx,
            EnemyProjectileSpec {
                origin,
                direction: shot_direction,
                damage: damage * damage_multiplier,
                speed: base_speed * difficulty.enemy_projectile_speed_multiplier(),
                radius: if role == EnemyCombatRole::Marksman {
                    0.44
                } else {
                    0.55
                },
                impact,
                name,
            },
        );
    }
}

struct EnemyProjectileSpec {
    origin: Vec3,
    direction: Vec3,
    damage: f32,
    speed: f32,
    radius: f32,
    impact: ProjectileImpact,
    name: &'static str,
}

fn spawn_projectile(commands: &mut Commands, vfx: &EnemyRuntimeVfx, spec: EnemyProjectileSpec) {
    let material = vfx
        .projectile_materials
        .get(spec.name)
        .expect("projectile VFX material must be cached")
        .clone();
    commands.spawn((
        Mesh3d(vfx.projectile_bolt.clone()),
        MeshMaterial3d(material),
        Transform::from_translation(spec.origin + spec.direction * 0.45).with_rotation(
            Quat::from_rotation_y(spec.direction.x.atan2(spec.direction.z)),
        ),
        EnemyProjectile {
            damage: spec.damage,
            velocity: spec.direction * spec.speed,
            radius: spec.radius,
            lifetime: Timer::from_seconds(2.0, TimerMode::Once),
            arm_timer: None,
            impact: spec.impact,
        },
        EnemyEntity,
        Name::new(spec.name.to_string()),
    ));
}

fn spawn_juggernaut_quake_line(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    origin: Vec3,
    direction: Vec3,
    damage: f32,
) {
    let color = Color::srgb(0.86, 0.20, 0.035);
    let mesh = meshes.add(ground_mark(1.05, GroundMark::Fracture));
    let material = materials.add(StandardMaterial {
        base_color: color,
        emissive: color.with_luminance(0.42).into(),
        alpha_mode: AlphaMode::Opaque,
        perceptual_roughness: 0.64,
        ..default()
    });
    for step in 1..=3 {
        let arm_secs = 0.28 + step as f32 * 0.13;
        commands.spawn((
            Mesh3d(mesh.clone()),
            MeshMaterial3d(material.clone()),
            Transform::from_translation(origin + direction * (step as f32 * 1.35) + Vec3::Y * 0.06),
            EnemyProjectile {
                damage,
                velocity: Vec3::ZERO,
                radius: 1.05,
                lifetime: Timer::from_seconds(arm_secs + 0.72, TimerMode::Once),
                arm_timer: Some(Timer::from_seconds(arm_secs, TimerMode::Once)),
                impact: ProjectileImpact::None,
            },
            EnemyEntity,
            Name::new(format!("Juggernaut Quake {step}")),
        ));
    }
}

fn spawn_shockwave_marker(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    origin: Vec3,
) {
    let color = Color::srgba(1.0, 0.28, 0.04, 0.55);
    commands.spawn((
        Mesh3d(meshes.add(ground_mark(2.6, GroundMark::Wave))),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: color,
            emissive: color.into(),
            alpha_mode: AlphaMode::Opaque,
            ..default()
        })),
        Transform::from_translation(origin + Vec3::new(0.0, 0.04, 0.0)),
        EnemyProjectile {
            damage: 0.0,
            velocity: Vec3::ZERO,
            radius: 0.0,
            lifetime: Timer::from_seconds(0.3, TimerMode::Once),
            arm_timer: None,
            impact: ProjectileImpact::None,
        },
        EnemyEntity,
        Name::new("Ember Shockwave Telegraph"),
    ));
}

pub fn spawn_molten_death_pool(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    origin: Vec3,
    damage: f32,
) {
    let color = Color::srgba(1.0, 0.18, 0.02, 0.72);
    commands.spawn((
        Mesh3d(meshes.add(ground_mark(1.35, GroundMark::Fracture))),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: color,
            emissive: color.into(),
            alpha_mode: AlphaMode::Opaque,
            ..default()
        })),
        Transform::from_translation(origin + Vec3::new(0.0, 0.08, 0.0)),
        EnemyProjectile {
            damage,
            velocity: Vec3::ZERO,
            radius: 1.35,
            lifetime: Timer::from_seconds(1.4, TimerMode::Once),
            arm_timer: projectile_arm_timer(ProjectileImpact::None),
            impact: ProjectileImpact::None,
        },
        EnemyEntity,
        Name::new("Molten Death Pool"),
    ));
}

fn spawn_arcane_sentry(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    target: Vec3,
    damage: f32,
) {
    let color = Color::srgba(0.62, 0.30, 1.0, 0.62);
    commands.spawn((
        Mesh3d(meshes.add(ground_mark(1.05, GroundMark::Arcane))),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: color,
            emissive: Color::srgb(0.20, 0.05, 0.42).into(),
            alpha_mode: AlphaMode::Opaque,
            ..default()
        })),
        Transform::from_translation(target + Vec3::new(0.0, 0.09, 0.0)),
        EnemyProjectile {
            damage,
            velocity: Vec3::ZERO,
            radius: 1.05,
            lifetime: Timer::from_seconds(1.65, TimerMode::Once),
            arm_timer: projectile_arm_timer(ProjectileImpact::Arcane),
            impact: ProjectileImpact::Arcane,
        },
        EnemyEntity,
        Name::new("Arcane Sentry"),
    ));
}

fn spawn_jailer_prison(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    target: Vec3,
    damage: f32,
) {
    let color = Color::srgba(0.78, 0.20, 0.08, 0.58);
    commands.spawn((
        Mesh3d(meshes.add(ground_mark(0.95, GroundMark::Shackles))),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: color,
            emissive: Color::srgb(0.35, 0.04, 0.02).into(),
            alpha_mode: AlphaMode::Opaque,
            ..default()
        })),
        Transform::from_translation(target + Vec3::new(0.0, 0.12, 0.0)),
        EnemyProjectile {
            damage,
            velocity: Vec3::ZERO,
            radius: 0.95,
            lifetime: Timer::from_seconds(1.15, TimerMode::Once),
            arm_timer: projectile_arm_timer(ProjectileImpact::Jailer),
            impact: ProjectileImpact::Jailer,
        },
        EnemyEntity,
        Name::new("Jailer Prison"),
    ));
}

fn spawn_frozen_orb(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    target: Vec3,
    damage: f32,
) {
    let color = Color::srgba(0.34, 0.74, 1.0, 0.62);
    commands.spawn((
        Mesh3d(meshes.add(ground_mark(1.2, GroundMark::Frost))),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: color,
            emissive: Color::srgb(0.04, 0.18, 0.42).into(),
            alpha_mode: AlphaMode::Opaque,
            ..default()
        })),
        Transform::from_translation(target + Vec3::new(0.0, 0.1, 0.0)),
        EnemyProjectile {
            damage,
            velocity: Vec3::ZERO,
            radius: 1.2,
            lifetime: Timer::from_seconds(1.25, TimerMode::Once),
            arm_timer: projectile_arm_timer(ProjectileImpact::Frozen),
            impact: ProjectileImpact::Frozen,
        },
        EnemyEntity,
        Name::new("Frozen Orb"),
    ));
}

fn spawn_desecrator_pool(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    target: Vec3,
    damage: f32,
) {
    let color = Color::srgba(0.68, 0.03, 0.09, 0.66);
    commands.spawn((
        Mesh3d(meshes.add(ground_mark(1.45, GroundMark::Blight))),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: color,
            emissive: Color::srgb(0.42, 0.0, 0.03).into(),
            alpha_mode: AlphaMode::Opaque,
            ..default()
        })),
        Transform::from_translation(target + Vec3::new(0.0, 0.095, 0.0)),
        EnemyProjectile {
            damage,
            velocity: Vec3::ZERO,
            radius: 1.45,
            lifetime: Timer::from_seconds(1.9, TimerMode::Once),
            arm_timer: projectile_arm_timer(ProjectileImpact::Desecrator),
            impact: ProjectileImpact::Desecrator,
        },
        EnemyEntity,
        Name::new("Desecrator Pool"),
    ));
}

fn spawn_seal_warden_rune(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    target: Vec3,
    damage: f32,
) {
    let color = Color::srgba(0.92, 0.34, 1.0, 0.62);
    commands.spawn((
        Mesh3d(meshes.add(ground_mark(1.28, GroundMark::Seal))),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: color,
            emissive: Color::srgb(0.44, 0.08, 0.58).into(),
            alpha_mode: AlphaMode::Opaque,
            ..default()
        })),
        Transform::from_translation(target + Vec3::new(0.0, 0.105, 0.0)),
        EnemyProjectile {
            damage,
            velocity: Vec3::ZERO,
            radius: 1.28,
            lifetime: Timer::from_seconds(1.35, TimerMode::Once),
            arm_timer: projectile_arm_timer(ProjectileImpact::WardenRune),
            impact: ProjectileImpact::WardenRune,
        },
        EnemyEntity,
        Name::new("Seal Warden Rune"),
    ));
}

fn projectile_arm_timer(impact: ProjectileImpact) -> Option<Timer> {
    let seconds = match impact {
        ProjectileImpact::Arcane => 0.55,
        ProjectileImpact::Jailer => 0.30,
        ProjectileImpact::Frozen => 0.70,
        ProjectileImpact::Desecrator => 0.45,
        ProjectileImpact::WardenRune => 0.62,
        ProjectileImpact::None => 0.25,
        ProjectileImpact::Burning => return None,
    };
    Some(Timer::from_seconds(seconds, TimerMode::Once))
}

fn projectile_is_armed(projectile: &EnemyProjectile) -> bool {
    projectile.arm_timer.as_ref().is_none_or(Timer::is_finished)
}

/// Swept collision keeps fast bolts from stepping through a defending minion.
fn projectile_segment_hits(start: Vec3, segment: Vec3, center: Vec3, radius: f32) -> bool {
    let t = if segment.length_squared() > 0.0001 {
        ((center - start).dot(segment) / segment.length_squared()).clamp(0.0, 1.0)
    } else {
        0.0
    };
    center.distance(start + segment * t) <= radius.max(0.0)
}

#[allow(clippy::too_many_arguments)]
fn move_enemy_projectiles(
    time: Res<Time>,
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut combat_events: MessageWriter<CombatEvent>,
    mut floating_text: MessageWriter<FloatingCombatTextEvent>,
    mut screen_shake: MessageWriter<ScreenShakeEvent>,
    mut projectiles: Query<(Entity, &mut Transform, &mut EnemyProjectile), Without<Player>>,
    mut player: ProjectilePlayerQuery,
    mut minions: MinionDamageQuery,
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

    for (entity, mut transform, mut projectile) in &mut projectiles {
        projectile.lifetime.tick(time.delta());
        if let Some(arm_timer) = projectile.arm_timer.as_mut() {
            arm_timer.tick(time.delta());
        }
        let previous = transform.translation;
        transform.translation += projectile.velocity * time.delta_secs();
        let segment = transform.translation - previous;
        let owner_center = player_transform.translation + Vec3::Y * 0.8;
        let owner_hit = projectile_segment_hits(previous, segment, owner_center, projectile.radius);
        if projectile_is_armed(&projectile) && projectile.damage > 0.0 {
            let owner_distance = if owner_hit {
                previous.distance_squared(owner_center)
            } else {
                f32::INFINITY
            };
            let first_pet = minions
                .iter()
                .filter(|(_, pose, vitality)| {
                    vitality.current > 0.0
                        && previous.distance_squared(pose.translation + Vec3::Y * 0.8)
                            < owner_distance
                        && projectile_segment_hits(
                            previous,
                            segment,
                            pose.translation + Vec3::Y * 0.8,
                            projectile.radius,
                        )
                })
                .min_by(|(_, a, _), (_, b, _)| {
                    previous
                        .distance_squared(a.translation)
                        .total_cmp(&previous.distance_squared(b.translation))
                })
                .map(|(entity, _, _)| entity);
            if let Some(pet) = first_pet
                && let Ok((_, pose, mut vitality)) = minions.get_mut(pet)
            {
                let taken = vitality.damage(projectile.damage);
                spawn_player_damage_impact(
                    &mut commands,
                    &mut meshes,
                    &mut materials,
                    previous,
                    pose.translation,
                    DamageTaken {
                        absorbed: 0.0,
                        health_damage: taken,
                    },
                );
                commands.entity(entity).try_despawn();
                continue;
            }
        }
        if projectile_is_armed(&projectile) && projectile.damage > 0.0 && owner_hit {
            let damage = mitigated_damage(projectile.damage, player_armor);
            let taken =
                apply_player_damage_with_evade(&mut player_health, &mut barrier, evade, damage);
            match projectile.impact {
                ProjectileImpact::Burning => {
                    combat_events.write(CombatEvent {
                        text: format!("Cinder bolt hit for {damage:.0}; burning"),
                    });
                    commands.entity(player_entity).try_insert(Burning {
                        dps: 3.5,
                        timer: Timer::from_seconds(3.0, TimerMode::Once),
                    });
                }
                ProjectileImpact::Arcane => {
                    combat_events.write(CombatEvent {
                        text: format!("Arcane sentry hit for {damage:.0}"),
                    });
                }
                ProjectileImpact::Jailer => {
                    combat_events.write(CombatEvent {
                        text: format!("Jailer prison hit for {damage:.0}"),
                    });
                }
                ProjectileImpact::Frozen => {
                    combat_events.write(CombatEvent {
                        text: format!("Frozen burst hit for {damage:.0}; immobilized"),
                    });
                    commands
                        .entity(player_entity)
                        .try_insert(frozen_immobilize());
                }
                ProjectileImpact::Desecrator => {
                    combat_events.write(CombatEvent {
                        text: format!("Desecrator pool hit for {damage:.0}; burning"),
                    });
                    commands.entity(player_entity).try_insert(Burning {
                        dps: 4.0,
                        timer: Timer::from_seconds(2.5, TimerMode::Once),
                    });
                }
                ProjectileImpact::WardenRune => {
                    combat_events.write(CombatEvent {
                        text: format!("Seal rune erupted for {damage:.0}; movement suppressed"),
                    });
                    commands.entity(player_entity).try_insert(Jailed {
                        timer: Timer::from_seconds(0.85, TimerMode::Once),
                        speed_multiplier: 0.42,
                    });
                }
                ProjectileImpact::None => {
                    combat_events.write(CombatEvent {
                        text: format!("Hazard hit for {damage:.0}"),
                    });
                }
            }
            emit_player_damage_feedback(
                &mut combat_events,
                &mut floating_text,
                &mut screen_shake,
                PlayerDamageFeedback {
                    source: projectile_damage_source(projectile.impact),
                    taken,
                    player_health: &player_health,
                    position: player_transform.translation,
                },
            );
            spawn_player_damage_impact(
                &mut commands,
                &mut meshes,
                &mut materials,
                transform.translation,
                player_transform.translation,
                taken,
            );
            trigger_player_hit_react(&mut commands, player_entity, taken);
            if let Ok(mut entity_commands) = commands.get_entity(entity) {
                entity_commands.try_despawn();
            }
            continue;
        }
        if projectile.lifetime.is_finished()
            && let Ok(mut entity_commands) = commands.get_entity(entity)
        {
            entity_commands.try_despawn();
        }
    }
}

fn frozen_immobilize() -> Jailed {
    Jailed {
        timer: Timer::from_seconds(1.35, TimerMode::Once),
        speed_multiplier: 0.0,
    }
}

struct PlayerDamageFeedback<'a> {
    source: &'a str,
    taken: DamageTaken,
    player_health: &'a Health,
    position: Vec3,
}

fn emit_player_damage_feedback(
    combat_events: &mut MessageWriter<CombatEvent>,
    floating_text: &mut MessageWriter<FloatingCombatTextEvent>,
    screen_shake: &mut MessageWriter<ScreenShakeEvent>,
    feedback: PlayerDamageFeedback,
) {
    let text = player_damage_feedback_text(feedback.source, feedback.taken, feedback.player_health);
    combat_events.write(CombatEvent { text });
    floating_text.write(FloatingCombatTextEvent {
        text: player_damage_floating_text(feedback.taken),
        position: feedback.position + Vec3::Y * 0.65,
        critical: false,
    });
    if let Some(shake) = player_damage_screen_shake(feedback.taken) {
        screen_shake.write(shake);
    }
}

fn trigger_player_hit_react(commands: &mut Commands, player_entity: Entity, taken: DamageTaken) {
    commands
        .entity(player_entity)
        .try_insert(PlayerHitReact::from_damage(taken));
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
enum PlayerDamageImpactKind {
    Health,
    Ward,
    Evade,
}

impl From<PlayerHitReactKind> for PlayerDamageImpactKind {
    fn from(kind: PlayerHitReactKind) -> Self {
        match kind {
            PlayerHitReactKind::Health => Self::Health,
            PlayerHitReactKind::Ward => Self::Ward,
            PlayerHitReactKind::Evade => Self::Evade,
        }
    }
}

#[derive(Debug, Clone, Copy)]
struct PlayerDamageImpactStyle {
    color: Color,
    y: f32,
    scale: Vec3,
    lifetime_secs: f32,
}

fn player_damage_impact_kind(taken: DamageTaken) -> PlayerDamageImpactKind {
    PlayerHitReact::from_damage(taken).kind.into()
}

fn player_damage_impact_style(taken: DamageTaken) -> PlayerDamageImpactStyle {
    match player_damage_impact_kind(taken) {
        PlayerDamageImpactKind::Health => {
            let force = (taken.health_damage * 0.0015).clamp(0.0, 0.06);
            PlayerDamageImpactStyle {
                color: Color::srgb(1.0, 0.22, 0.12),
                y: 0.86,
                scale: Vec3::splat(0.12 + force),
                lifetime_secs: 0.18,
            }
        }
        PlayerDamageImpactKind::Ward => {
            let force = (taken.absorbed * 0.0015).clamp(0.0, 0.05);
            PlayerDamageImpactStyle {
                color: Color::srgb(0.98, 0.72, 0.20),
                y: 0.92,
                scale: Vec3::splat(0.12 + force),
                lifetime_secs: 0.16,
            }
        }
        PlayerDamageImpactKind::Evade => PlayerDamageImpactStyle {
            color: Color::srgb(0.28, 0.78, 1.0),
            y: 0.58,
            scale: Vec3::splat(0.10),
            lifetime_secs: 0.12,
        },
    }
}

fn player_damage_impact_transform(source: Vec3, target: Vec3, taken: DamageTaken) -> Transform {
    let style = player_damage_impact_style(taken);
    let direction = flat_recoil_direction(source, target);
    let right = Vec3::new(direction.z, 0.0, -direction.x);
    let kind = player_damage_impact_kind(taken);
    let lateral = match kind {
        PlayerDamageImpactKind::Evade => right * 0.34 - direction * 0.18,
        PlayerDamageImpactKind::Ward => direction * -0.32,
        PlayerDamageImpactKind::Health => direction * -0.32,
    };
    Transform::from_translation(target + Vec3::Y * style.y + lateral)
        .with_rotation(Quat::from_rotation_y(direction.x.atan2(direction.z)))
        .with_scale(style.scale)
}

// Three narrow, tapered rays leave nearly all of the silhouette unobscured.
// Opaque geometry also works on render profiles without alpha-blended PBR.
fn impact_spark_mesh() -> Mesh {
    let mut positions = Vec::with_capacity(36);
    for (axis, side) in [(Vec3::X, Vec3::Y), (Vec3::Y, Vec3::Z), (Vec3::Z, Vec3::X)] {
        let a = -axis * 0.5;
        let b = side * 0.025;
        let c = axis * 0.5;
        let d = -side * 0.025;
        for point in [a, b, c, a, c, d, c, b, a, d, c, a] {
            positions.push(point.to_array());
        }
    }
    let mut mesh = Mesh::new(bevy::mesh::PrimitiveTopology::TriangleList, default());
    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
    mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, vec![[0.0, 1.0, 0.0]; 36]);
    mesh
}

fn spawn_player_damage_impact(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    source: Vec3,
    target: Vec3,
    taken: DamageTaken,
) {
    let kind = player_damage_impact_kind(taken);
    let style = player_damage_impact_style(taken);
    let material = materials.add(StandardMaterial {
        base_color: style.color,
        emissive: style.color.into(),
        unlit: true,
        alpha_mode: AlphaMode::Opaque,
        ..default()
    });
    let mesh = meshes.add(impact_spark_mesh());
    let count = match kind {
        PlayerDamageImpactKind::Health => 5,
        PlayerDamageImpactKind::Ward => 7,
        PlayerDamageImpactKind::Evade => 3,
    };
    let origin = player_damage_impact_transform(source, target, taken);
    for index in 0..count {
        let angle = -1.0 + 2.0 * index as f32 / (count - 1) as f32;
        let local_direction = Vec3::new(angle.sin(), 0.35 + 0.08 * index as f32, -angle.cos());
        let velocity = origin.rotation * local_direction.normalize() * 1.2;
        let mut transform = origin;
        transform.translation += velocity * 0.035;
        transform.rotation *= Quat::from_rotation_z(angle);
        commands.spawn((
            Mesh3d(mesh.clone()),
            MeshMaterial3d(material.clone()),
            transform,
            EnemyProjectile {
                damage: 0.0,
                velocity,
                radius: 0.0,
                lifetime: Timer::from_seconds(style.lifetime_secs, TimerMode::Once),
                arm_timer: None,
                impact: ProjectileImpact::None,
            },
            EnemyEntity,
            Name::new(player_damage_impact_name(kind)),
        ));
    }
}

fn player_damage_impact_name(kind: PlayerDamageImpactKind) -> &'static str {
    match kind {
        PlayerDamageImpactKind::Health => "Player Health Impact",
        PlayerDamageImpactKind::Ward => "Player Ward Impact",
        PlayerDamageImpactKind::Evade => "Player Evade Afterimage",
    }
}

fn projectile_damage_source(impact: ProjectileImpact) -> &'static str {
    match impact {
        ProjectileImpact::Burning => "Cinder bolt",
        ProjectileImpact::Arcane => "Arcane sentry",
        ProjectileImpact::Jailer => "Jailer prison",
        ProjectileImpact::Frozen => "Frozen burst",
        ProjectileImpact::Desecrator => "Desecrator pool",
        ProjectileImpact::WardenRune => "Seal rune",
        ProjectileImpact::None => "Hazard",
    }
}

fn player_damage_feedback_text(source: &str, taken: DamageTaken, health: &Health) -> String {
    let core = if taken.health_damage <= 0.0 && taken.absorbed <= 0.0 {
        format!("{source} evaded")
    } else if taken.health_damage <= 0.0 {
        format!("{source} absorbed by ward for {:.0}", taken.absorbed)
    } else if taken.absorbed > 0.0 {
        format!(
            "{source} hit for {:.0} ({:.0} ward)",
            taken.health_damage, taken.absorbed
        )
    } else {
        format!("{source} hit for {:.0}", taken.health_damage)
    };
    if player_health_danger_after_hit(health, taken) {
        format!("Low life: {core}")
    } else {
        core
    }
}

fn player_damage_floating_text(taken: DamageTaken) -> String {
    if taken.health_damage <= 0.0 && taken.absorbed <= 0.0 {
        "EVADE".to_string()
    } else if taken.health_damage <= 0.0 {
        format!("WARD -{:.0}", taken.absorbed)
    } else if taken.absorbed > 0.0 {
        format!("-{:.0}  WARD -{:.0}", taken.health_damage, taken.absorbed)
    } else {
        format!("-{:.0}", taken.health_damage)
    }
}

fn player_damage_screen_shake(taken: DamageTaken) -> Option<ScreenShakeEvent> {
    if taken.health_damage <= 0.0 && taken.absorbed <= 0.0 {
        return None;
    }
    let intensity = if taken.health_damage > 0.0 {
        (0.055 + taken.health_damage * 0.0025).clamp(0.055, 0.16)
    } else {
        (0.025 + taken.absorbed * 0.0015).clamp(0.025, 0.07)
    };
    Some(ScreenShakeEvent::new(intensity, 0.13))
}

fn player_health_danger_after_hit(health: &Health, taken: DamageTaken) -> bool {
    taken.health_damage > 0.0 && health.max > 0.0 && health.current <= health.max * 0.35
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_enemy(id: &str, affixes: Vec<EnemyAffix>) -> Enemy {
        Enemy {
            id: id.to_string(),
            display_name: id.to_string(),
            affixes,
            attack_damage: 10.0,
            attack_kind: EnemyAttackKind::Melee,
            attack_range: 1.0,
            attack_timer: Timer::from_seconds(1.0, TimerMode::Once),
            aggro_range: 8.0,
            move_speed: 1.0,
            gold_min: 1,
            gold_max: 2,
            xp_reward: 3,
        }
    }

    #[test]
    fn affix_display_names_prefix_base_name() {
        let name = display_name_with_affixes(
            "Ashbone Guard",
            &[
                EnemyAffix::Frenzied,
                EnemyAffix::Molten,
                EnemyAffix::Shielded,
                EnemyAffix::Arcane,
                EnemyAffix::Jailer,
                EnemyAffix::Frozen,
                EnemyAffix::Desecrator,
                EnemyAffix::Reflective,
            ],
        );

        assert_eq!(
            name,
            "Frenzied Molten Shielded Arcane Jailer Frozen Desecrator Reflective Ashbone Guard"
        );
    }

    #[test]
    fn affix_masks_track_distinct_elite_traits() {
        let mask = affix_mask(&[EnemyAffix::Frenzied, EnemyAffix::Arcane, EnemyAffix::Arcane]);

        assert_eq!(mask.count_ones(), 2);
        assert_ne!(EnemyAffix::Frenzied.mask(), EnemyAffix::Arcane.mask());
    }

    #[test]
    fn enemy_target_focus_lingers_longer_for_critical_hits() {
        let mut normal = EnemyTargetFocus::new(false);
        let mut critical = EnemyTargetFocus::new(true);

        normal.tick(std::time::Duration::from_secs_f32(1.0));
        critical.tick(std::time::Duration::from_secs_f32(1.0));

        assert!(normal.is_finished());
        assert!(!critical.is_finished());
        assert!(critical.intensity() > 0.0);
        assert!(critical.critical());
    }

    #[test]
    fn reward_scaling_never_drops_below_one() {
        assert_eq!(scale_reward(0, 1.8), 1);
        assert_eq!(scale_reward(10, 1.8), 18);
    }

    #[test]
    fn boss_phase_triggers_once_below_threshold() {
        let healthy = Health {
            current: 56.0,
            max: 100.0,
        };
        let wounded = Health {
            current: 55.0,
            max: 100.0,
        };

        assert!(!boss_phase_should_trigger(&healthy));
        assert!(boss_phase_should_trigger(&wounded));
    }

    #[test]
    fn boss_enrage_triggers_after_phase_two_timer() {
        let mut phase = BossPhase {
            phase_two_started: true,
            enrage_started: false,
            enrage_timer: Timer::from_seconds(1.0, TimerMode::Once),
        };

        assert!(!boss_enrage_should_trigger(&phase));
        phase
            .enrage_timer
            .tick(std::time::Duration::from_secs_f32(1.0));

        assert!(boss_enrage_should_trigger(&phase));
    }

    #[test]
    fn boss_phase_events_tell_player_how_to_finish_the_fight() {
        let phase_two = boss_phase_two_event_text(22.0);
        assert!(phase_two.contains("Boss Phase II"));
        assert!(phase_two.contains("BREAK MALREC"));
        assert!(phase_two.contains("EXECUTE WINDOW"));
        assert!(phase_two.contains("stalker"));
        assert!(phase_two.contains("marksman"));
        assert!(phase_two.contains("brute"));

        let enrage = boss_enrage_event_text();
        assert!(enrage.contains("Ashen Enrage"));
        assert!(enrage.contains("floor burning"));
        assert!(enrage.contains("Primal cache downgraded"));
        assert!(enrage.contains("Arcane/Jailer/Desecrator"));
    }

    #[test]
    fn boss_reinforcement_waves_use_distinct_chapter_roles() {
        let phase_two = boss_phase_two_reinforcements();
        let enrage = boss_enrage_reinforcements();

        assert_eq!(phase_two.len(), 3);
        assert!(phase_two.iter().any(|spawn| spawn.id == "bone_stalker"));
        assert!(phase_two.iter().any(|spawn| spawn.id == "ashen_marksman"));
        assert!(phase_two.iter().any(|spawn| spawn.id == "reliquary_brute"));
        assert!(
            phase_two
                .iter()
                .any(|spawn| spawn.affixes.contains(&EnemyAffix::Jailer))
        );
        assert!(
            phase_two
                .iter()
                .any(|spawn| spawn.affixes.contains(&EnemyAffix::Reflective))
        );

        assert_eq!(enrage.len(), 2);
        assert!(enrage.iter().any(|spawn| spawn.id == "ashen_marksman"));
        assert!(enrage.iter().any(|spawn| spawn.id == "reliquary_brute"));
        assert!(
            enrage
                .iter()
                .any(|spawn| spawn.affixes.contains(&EnemyAffix::Desecrator))
        );
    }

    #[test]
    fn boss_stagger_accumulates_resets_and_reports_summary() {
        let mut stagger = BossStagger {
            meter: 0.0,
            threshold: 50.0,
        };

        assert!(!add_boss_stagger(&mut stagger, 24.0));
        assert_eq!(boss_stagger_summary(Some(&stagger), None), "stagger 24/50");
        assert!(add_boss_stagger(&mut stagger, 30.0));
        assert_eq!(stagger.meter, 0.0);

        let staggered = boss_staggered();
        assert!(boss_stagger_damage_multiplier(Some(&staggered)) > 1.0);
        assert!(boss_stagger_summary(Some(&stagger), Some(&staggered)).contains("EXECUTE WINDOW"));
    }

    #[test]
    fn boss_enrage_applies_pressure_once() {
        let mut enemy = Enemy {
            id: "keeper".to_string(),
            display_name: "Malrec, Keeper of Ash".to_string(),
            affixes: vec![EnemyAffix::Molten],
            attack_damage: 20.0,
            attack_kind: EnemyAttackKind::Melee,
            attack_range: 1.5,
            attack_timer: Timer::from_seconds(2.0, TimerMode::Once),
            aggro_range: 10.0,
            move_speed: 2.0,
            gold_min: 1,
            gold_max: 2,
            xp_reward: 10,
        };
        let mut phase = BossPhase {
            phase_two_started: true,
            enrage_started: false,
            enrage_timer: Timer::from_seconds(0.0, TimerMode::Once),
        };

        apply_boss_enrage(&mut enemy, &mut phase);
        let damage_after_first = enemy.attack_damage;

        assert!(phase.enrage_started);
        assert!(enemy.attack_damage > 20.0);
        assert!(enemy.move_speed > 2.0);
        assert!(enemy.attack_timer.duration().as_secs_f32() < 2.0);

        apply_boss_enrage(&mut enemy, &mut phase);

        assert_eq!(enemy.attack_damage, damage_after_first);
    }

    #[test]
    fn chilled_multiplier_is_clamped() {
        let chilled = Chilled {
            timer: Timer::from_seconds(1.0, TimerMode::Once),
            slow_multiplier: 0.1,
        };

        assert_eq!(chilled_slow_multiplier(Some(&chilled)), 0.25);
        assert_eq!(chilled_slow_multiplier(None), 1.0);
    }

    #[test]
    fn seal_warden_ward_absorbs_then_opens_damage_window() {
        let mut ward = SealWardenWard {
            current: 20.0,
            max: 45.0,
            broken: false,
        };

        let blocked = ward.absorb_damage(12.0);
        assert_eq!(
            blocked,
            SealWardenWardHit {
                health_damage: 0.0,
                ward_damage: 12.0,
                ward_broken: false,
            }
        );
        assert_eq!(ward.current, 8.0);
        assert!(ward.active());

        let broken = ward.absorb_damage(15.0);
        assert_eq!(
            broken,
            SealWardenWardHit {
                health_damage: 7.0,
                ward_damage: 8.0,
                ward_broken: true,
            }
        );
        assert!(!ward.active());

        let punish = ward.absorb_damage(10.0);
        assert_eq!(punish.health_damage, 10.0);
        assert_eq!(punish.ward_damage, 0.0);
        assert!(!punish.ward_broken);
    }

    #[test]
    fn cursed_shrine_spawns_affixed_ambush_with_distinct_roles() {
        let spawns = cursed_shrine_spawns(Vec3::ZERO);

        assert_eq!(spawns.len(), 4);
        assert!(
            spawns
                .iter()
                .filter(|(_, _, affixes)| !affixes.is_empty())
                .count()
                >= CURSED_AMBUSH_KILLS as usize
        );
        assert!(spawns.iter().any(|(id, _, _)| id == "bone_stalker"));
        assert!(spawns.iter().any(|(id, _, _)| id == "ashen_marksman"));
        assert!(spawns.iter().any(|(id, _, _)| id == "reliquary_brute"));
        assert!(
            spawns
                .iter()
                .any(|(_, _, affixes)| affixes.contains(&EnemyAffix::Arcane))
        );
        assert!(
            spawns
                .iter()
                .any(|(_, _, affixes)| affixes.contains(&EnemyAffix::Jailer))
        );
        assert!(
            spawns
                .iter()
                .any(|(_, _, affixes)| affixes.contains(&EnemyAffix::Frozen))
        );
        assert!(
            spawns
                .iter()
                .any(|(_, _, affixes)| affixes.contains(&EnemyAffix::Desecrator))
        );
        assert!(
            spawns
                .iter()
                .any(|(_, _, affixes)| affixes.contains(&EnemyAffix::Reflective))
        );
    }

    #[test]
    fn obelisk_spawns_five_rite_enemies_with_elite_pressure() {
        let spawns = obelisk_spawns(Vec3::new(3.0, 0.0, -2.0));

        assert_eq!(spawns.len(), 5);
        assert!(spawns.iter().any(|(id, _, _)| id == "bone_stalker"));
        assert!(spawns.iter().any(|(id, _, _)| id == "ashen_marksman"));
        assert!(spawns.iter().any(|(id, _, _)| id == "reliquary_brute"));
        assert!(
            spawns
                .iter()
                .any(|(_, _, affixes)| affixes.contains(&EnemyAffix::Arcane))
        );
        assert!(
            spawns
                .iter()
                .any(|(_, _, affixes)| affixes.contains(&EnemyAffix::Desecrator))
        );
        assert!(
            spawns
                .iter()
                .any(|(_, _, affixes)| affixes.contains(&EnemyAffix::Reflective))
        );
        assert!(
            spawns
                .iter()
                .filter(|(_, _, affixes)| !affixes.is_empty())
                .count()
                >= 3
        );
    }

    #[test]
    fn rift_spawns_mix_chasers_marksmen_and_heavy_pressure() {
        let spawns = rift_spawns(Vec3::ZERO);

        assert_eq!(spawns.len(), 6);
        assert!(spawns.iter().any(|(id, _, _)| id == "bone_stalker"));
        assert!(spawns.iter().any(|(id, _, _)| id == "ashen_marksman"));
        assert!(spawns.iter().any(|(id, _, _)| id == "reliquary_brute"));
        assert!(
            spawns
                .iter()
                .filter(|(_, _, affixes)| !affixes.is_empty())
                .count()
                >= 5
        );
    }

    #[test]
    fn champion_pack_has_leader_and_elite_guards() {
        let spawns = champion_pack_spawns();

        assert_eq!(spawns.len(), 5);
        assert!(
            spawns
                .iter()
                .any(|(id, _, affixes)| id == "ashen_marksman" && affixes.len() >= 4)
        );
        assert!(spawns.iter().any(|(id, _, _)| id == "bone_stalker"));
        assert!(spawns.iter().any(|(id, _, _)| id == "reliquary_brute"));
        assert!(
            spawns
                .iter()
                .filter(|(_, _, affixes)| !affixes.is_empty())
                .count()
                >= 3
        );
        assert!(
            spawns
                .iter()
                .any(|(_, _, affixes)| affixes.contains(&EnemyAffix::Reflective))
        );
    }

    #[test]
    fn chapter_two_preserves_skirmisher_marksman_and_heavy_roles() {
        assert_eq!(
            chapter_enemy_id(ChapterId::HollowBastion, "skeleton"),
            "hollow_thrall"
        );
        assert_eq!(
            chapter_enemy_id(ChapterId::HollowBastion, "bone_stalker"),
            "hollow_stalker"
        );
        assert_eq!(
            chapter_enemy_id(ChapterId::HollowBastion, "cultist"),
            "choir_acolyte"
        );
        assert_eq!(
            chapter_enemy_id(ChapterId::HollowBastion, "ashen_marksman"),
            "choir_marksman"
        );
        assert_eq!(
            chapter_enemy_id(ChapterId::HollowBastion, "reliquary_brute"),
            "hollow_reaver"
        );
        assert_eq!(
            enemy_combat_role("hollow_stalker"),
            EnemyCombatRole::Skirmisher
        );
        assert_eq!(
            enemy_combat_role("choir_marksman"),
            EnemyCombatRole::Marksman
        );
        assert_eq!(
            enemy_combat_role("hollow_reaver"),
            EnemyCombatRole::Juggernaut
        );
    }

    #[test]
    fn every_enemy_catalog_model_has_an_explicit_runtime_mapping() {
        let catalog: EnemyCatalog =
            ron::from_str(include_str!("../assets/data/enemies.ron")).unwrap();
        for enemy in catalog.enemies {
            assert!(
                enemy_model_is_supported(&enemy.model),
                "{} uses unmapped model {}",
                enemy.id,
                enemy.model
            );
        }
    }

    #[test]
    fn difficulty_controls_signature_attack_frequency() {
        let mut normal = EnemySkillCycle::new(Difficulty::Normal);
        let normal_pattern = (0..4).map(|_| normal.advance()).collect::<Vec<_>>();
        let mut torment = EnemySkillCycle::new(Difficulty::Torment);
        let torment_pattern = (0..4).map(|_| torment.advance()).collect::<Vec<_>>();

        assert_eq!(normal_pattern, vec![false, false, false, true]);
        assert_eq!(torment_pattern, vec![false, true, false, true]);
        assert!(
            signature_attack_damage_multiplier(EnemyCombatRole::Skirmisher, true)
                > signature_attack_damage_multiplier(EnemyCombatRole::Vanguard, true)
        );
    }

    #[test]
    fn wilderness_encounters_are_reproducible_and_scale_pack_pressure() {
        let event = SpawnWildernessEncounter {
            chapter: ChapterId::AshenReliquary,
            chunk_x: 12,
            chunk_z: -8,
            center: Vec3::new(600.0, 0.0, -360.0),
            seed: 0xA55E_1234,
        };
        let first = wilderness_encounter_specs(ChapterId::AshenReliquary, Difficulty::Hell, event);
        let second = wilderness_encounter_specs(ChapterId::AshenReliquary, Difficulty::Hell, event);
        let normal =
            wilderness_encounter_specs(ChapterId::AshenReliquary, Difficulty::Normal, event);

        assert_eq!(first.len(), second.len());
        assert!(
            first
                .iter()
                .zip(second.iter())
                .all(|(left, right)| left.id == right.id
                    && left.offset == right.offset
                    && left.affixes == right.affixes)
        );
        assert!(first.len() > normal.len());
        assert!(first.len() <= 5);
        assert!(first.iter().any(|spec| !spec.affixes.is_empty()));
        assert!(wilderness_pack_fits(30, 2));
        assert!(!wilderness_pack_fits(31, 2));
        assert!(
            wilderness_distance_threat_multiplier(120, -4)
                > wilderness_distance_threat_multiplier(2, -4)
        );
        assert!(wilderness_distance_threat_multiplier(10_000, 0) <= 1.80);
    }

    #[test]
    fn wilderness_chapter_two_uses_its_own_roster() {
        let event = SpawnWildernessEncounter {
            chapter: ChapterId::HollowBastion,
            chunk_x: 8,
            chunk_z: 8,
            center: Vec3::ZERO,
            seed: 17,
        };
        let specs =
            wilderness_encounter_specs(ChapterId::HollowBastion, Difficulty::Torment, event);
        assert!(specs.iter().all(|spec| matches!(
            spec.id,
            "hollow_thrall"
                | "hollow_stalker"
                | "choir_acolyte"
                | "choir_marksman"
                | "hollow_reaver"
        )));
    }

    #[test]
    fn bosses_use_small_chapter_specific_affix_sets() {
        let malrec = boss_starting_affixes(ChapterId::AshenReliquary);
        let nyxara = boss_starting_affixes(ChapterId::HollowBastion);

        assert_eq!(malrec.len(), 3);
        assert_eq!(nyxara.len(), 3);
        assert!(malrec.contains(&EnemyAffix::Molten));
        assert!(nyxara.contains(&EnemyAffix::Arcane));
        assert_ne!(malrec, nyxara);
    }

    #[test]
    fn role_movement_makes_marksmen_backpedal_when_rushed() {
        let mut marksman = test_enemy("ashen_marksman", vec![]);
        marksman.attack_kind = EnemyAttackKind::Projectile;
        marksman.attack_range = 5.6;

        let close = enemy_movement_plan(
            &marksman,
            Vec3::ZERO,
            Vec3::X,
            marksman_min_spacing(&marksman) - 0.2,
        )
        .expect("marksman should retreat when rushed");
        let comfortable = enemy_movement_plan(&marksman, Vec3::ZERO, Vec3::X * 4.0, 4.0);

        assert!(close.movement_direction.x < -0.70);
        assert!(close.movement_direction.z < -0.30);
        assert!(close.facing_direction.x > 0.95);
        assert!(close.speed_multiplier > 1.0);
        assert!(comfortable.is_none());
    }

    #[test]
    fn role_movement_separates_stalker_chase_and_brute_weight() {
        let mut stalker = test_enemy("bone_stalker", vec![]);
        stalker.attack_kind = EnemyAttackKind::Melee;
        stalker.attack_range = 1.15;
        let mut brute = test_enemy("reliquary_brute", vec![]);
        brute.attack_kind = EnemyAttackKind::Shockwave;
        brute.attack_range = 1.9;

        let stalker_plan = enemy_movement_plan(&stalker, Vec3::ZERO, Vec3::Z * 5.0, 5.0).unwrap();
        let brute_plan = enemy_movement_plan(&brute, Vec3::ZERO, Vec3::Z * 5.0, 5.0).unwrap();

        assert!(enemy_desired_attack_range(&stalker) < stalker.attack_range);
        assert!(enemy_desired_attack_range(&brute) < brute.attack_range);
        assert!(stalker_plan.speed_multiplier > 1.0);
        assert!(brute_plan.speed_multiplier < 0.8);
        assert_eq!(stalker_plan.facing_direction, Vec3::Z);
        assert_eq!(brute_plan.movement_direction, Vec3::Z);
    }

    #[test]
    fn main_quest_enemy_markers_track_seal_warden_and_keeper() {
        let mut progress = ChapterProgress {
            phase: ChapterPhase::Sanctum,
            ..Default::default()
        };
        let alive = Health {
            current: 10.0,
            max: 10.0,
        };
        let dead = Health {
            current: 0.0,
            max: 10.0,
        };
        let mut seal_warden = test_enemy("seal_warden", vec![EnemyAffix::Arcane]);
        seal_warden.display_name = "Seal Warden Vhal".to_string();
        let keeper = test_enemy("keeper", vec![EnemyAffix::Molten]);

        assert_eq!(
            main_quest_enemy_marker_state(&progress, &seal_warden, &alive),
            MainQuestEnemyMarkerState::SealWarden
        );
        assert_eq!(
            main_quest_enemy_marker_state(&progress, &keeper, &alive),
            MainQuestEnemyMarkerState::Hidden
        );
        assert_eq!(
            main_quest_enemy_marker_state(&progress, &seal_warden, &dead),
            MainQuestEnemyMarkerState::Hidden
        );

        progress.phase = ChapterPhase::Boss;
        assert_eq!(
            main_quest_enemy_marker_state(&progress, &keeper, &alive),
            MainQuestEnemyMarkerState::Keeper
        );
        progress.boss_slain = true;
        assert_eq!(
            main_quest_enemy_marker_state(&progress, &keeper, &alive),
            MainQuestEnemyMarkerState::Hidden
        );

        assert!(
            main_quest_enemy_marker_height(MainQuestEnemyMarkerState::Keeper)
                > main_quest_enemy_marker_height(MainQuestEnemyMarkerState::SealWarden)
        );
        assert!(
            main_quest_enemy_marker_scale(MainQuestEnemyMarkerState::Keeper, 1.0)
                > main_quest_enemy_marker_scale(MainQuestEnemyMarkerState::SealWarden, 1.0)
        );
    }

    #[test]
    fn ashen_threat_spawns_three_retaliators_and_escalates_late_surges() {
        let early = ashen_threat_spawns(Vec3::ZERO, 1);
        let late = ashen_threat_spawns(Vec3::ZERO, 3);

        assert_eq!(early.len(), 3);
        assert!(early.iter().all(|(_, _, affixes)| !affixes.is_empty()));
        assert!(early.iter().any(|(id, _, _)| id == "ashen_marksman"));
        assert!(early.iter().any(|(id, _, _)| id == "bone_stalker"));
        assert_eq!(early[1].2, vec![EnemyAffix::Frenzied]);
        assert_eq!(late[1].2, vec![EnemyAffix::Arcane, EnemyAffix::Jailer]);
    }

    #[test]
    fn control_affix_labels_are_visible() {
        assert_eq!(EnemyAffix::Arcane.label(), "Arcane");
        assert_eq!(EnemyAffix::Frozen.label(), "Frozen");
        assert_eq!(EnemyAffix::Desecrator.label(), "Desecrator");
        assert_eq!(EnemyAffix::Reflective.label(), "Reflective");
    }

    #[test]
    fn elite_affix_visuals_prioritize_readable_threats() {
        assert_eq!(affix_visual_kind(&[]), None);
        assert_eq!(
            affix_visual_kind(&[EnemyAffix::Arcane, EnemyAffix::Molten]),
            Some(EliteAffixVisual::Arcane)
        );
        assert_eq!(
            affix_visual_kind(&[EnemyAffix::Frozen, EnemyAffix::Desecrator]),
            Some(EliteAffixVisual::Frost)
        );
        assert_eq!(
            affix_visual_kind(&[EnemyAffix::Molten]),
            Some(EliteAffixVisual::Ember)
        );
        assert_eq!(
            affix_visual_kind(&[EnemyAffix::Vampiric, EnemyAffix::Shielded]),
            Some(EliteAffixVisual::Blood)
        );
        assert_eq!(
            affix_visual_kind(&[EnemyAffix::Reflective]),
            Some(EliteAffixVisual::Ward)
        );
        assert!(affix_visual_scale(&nemesis_affixes()) > affix_visual_scale(&[EnemyAffix::Molten]));
        assert!(affix_aura_spin_speed(4) > affix_aura_spin_speed(1));
        assert!(affix_visual_light_intensity(4) > affix_visual_light_intensity(1));
        assert!(affix_visual_light_range(4) > affix_visual_light_range(1));
        assert!(
            affix_aura_pulse_factor(std::f32::consts::FRAC_PI_2, 1.0, 0.08)
                > affix_aura_pulse_factor(0.0, 1.0, 0.08)
        );
    }

    #[test]
    fn frozen_immobilize_stops_movement_briefly() {
        let frozen = frozen_immobilize();

        assert_eq!(frozen.speed_multiplier, 0.0);
        assert!(frozen.timer.remaining_secs() > 1.0);
    }

    #[test]
    fn affix_projectiles_arm_after_visible_warning() {
        let mut arcane = EnemyProjectile {
            damage: 12.0,
            velocity: Vec3::ZERO,
            radius: 1.0,
            lifetime: Timer::from_seconds(1.65, TimerMode::Once),
            arm_timer: projectile_arm_timer(ProjectileImpact::Arcane),
            impact: ProjectileImpact::Arcane,
        };
        let burning = EnemyProjectile {
            damage: 8.0,
            velocity: Vec3::X,
            radius: 0.5,
            lifetime: Timer::from_seconds(2.0, TimerMode::Once),
            arm_timer: projectile_arm_timer(ProjectileImpact::Burning),
            impact: ProjectileImpact::Burning,
        };

        assert!(!projectile_is_armed(&arcane));
        assert!(projectile_is_armed(&burning));

        arcane
            .arm_timer
            .as_mut()
            .unwrap()
            .tick(std::time::Duration::from_secs_f32(0.55));

        assert!(projectile_is_armed(&arcane));
    }

    #[test]
    fn affix_warning_windows_match_threat_readability() {
        let frozen = projectile_arm_timer(ProjectileImpact::Frozen).unwrap();
        let jailer = projectile_arm_timer(ProjectileImpact::Jailer).unwrap();
        let desecrator = projectile_arm_timer(ProjectileImpact::Desecrator).unwrap();
        let warden = projectile_arm_timer(ProjectileImpact::WardenRune).unwrap();

        assert!(frozen.duration().as_secs_f32() > jailer.duration().as_secs_f32());
        assert!(desecrator.duration().as_secs_f32() > 0.3);
        assert!(warden.duration().as_secs_f32() > jailer.duration().as_secs_f32());
        assert_eq!(
            projectile_damage_source(ProjectileImpact::WardenRune),
            "Seal rune"
        );
        assert!(projectile_arm_timer(ProjectileImpact::Burning).is_none());
    }

    #[test]
    fn enemy_attack_warning_windows_give_readable_windup() {
        assert!(
            enemy_attack_warning_window(&EnemyAttackKind::Shockwave)
                > enemy_attack_warning_window(&EnemyAttackKind::Projectile)
        );
        assert!(
            enemy_attack_warning_window(&EnemyAttackKind::Projectile)
                > enemy_attack_warning_window(&EnemyAttackKind::Melee)
        );
        assert!(enemy_attack_warning_window(&EnemyAttackKind::Melee) >= 0.45);
        assert!(enemy_attack_warning_window(&EnemyAttackKind::Shockwave) >= 0.85);
    }

    #[test]
    fn enemy_attack_telegraph_shapes_read_as_different_threats() {
        let melee_reach = enemy_attack_warning_reach(&EnemyAttackKind::Melee, 1.0);
        let projectile_reach = enemy_attack_warning_reach(&EnemyAttackKind::Projectile, 5.5);
        let shockwave_reach = enemy_attack_warning_reach(&EnemyAttackKind::Shockwave, 2.0);

        assert!(projectile_reach > melee_reach * 3.0);
        assert!(shockwave_reach > 3.0);
        assert!(
            enemy_attack_warning_width(&EnemyAttackKind::Melee)
                > enemy_attack_warning_width(&EnemyAttackKind::Projectile)
        );
        assert!(enemy_attack_warning_width(&EnemyAttackKind::Projectile) >= 0.45);
        assert_eq!(
            enemy_attack_warning_forward_offset(&EnemyAttackKind::Shockwave, shockwave_reach),
            0.0
        );
    }

    #[test]
    fn enemy_attack_telegraph_pulses_toward_impact() {
        let early = enemy_attack_telegraph_pulse(0.05);
        let late = enemy_attack_telegraph_pulse(0.95);

        assert!(early >= 1.0);
        assert!(late > early);
        assert!(late > 1.25);
        assert!(enemy_attack_telegraph_pulse(2.0) <= 1.40);
    }

    #[test]
    fn enemy_attack_impacts_make_contact_frame_readable_by_attack_type() {
        let melee = enemy_attack_impact_style(&EnemyAttackKind::Melee, 1.4);
        let projectile = enemy_attack_impact_style(&EnemyAttackKind::Projectile, 5.6);
        let shockwave = enemy_attack_impact_style(&EnemyAttackKind::Shockwave, 2.2);

        assert!(melee.scale.z > melee.scale.x * 0.88);
        assert!(projectile.y > melee.y + 0.80);
        assert!(shockwave.scale.x > melee.scale.x + 1.6);
        assert!(shockwave.scale.z > melee.scale.z + 1.6);
        assert!(melee.lifetime_secs < shockwave.lifetime_secs);
        assert!(projectile.color.to_srgba().alpha > shockwave.color.to_srgba().alpha);
    }

    #[test]
    fn enemy_attack_impact_transform_follows_facing_and_has_safe_default() {
        let origin = Vec3::new(2.0, 0.0, -1.0);
        let melee = enemy_attack_impact_transform(origin, Vec3::X, &EnemyAttackKind::Melee, 1.6);
        let fallback =
            enemy_attack_impact_transform(origin, Vec3::ZERO, &EnemyAttackKind::Melee, 1.6);
        let cast =
            enemy_attack_impact_transform(origin, Vec3::Z, &EnemyAttackKind::Projectile, 5.6);

        assert!(melee.translation.x > origin.x + 0.7);
        assert!((melee.translation.z - origin.z).abs() < 0.01);
        assert!(fallback.translation.z > origin.z + 0.7);
        assert!(cast.translation.y > melee.translation.y + 0.8);
    }

    #[test]
    fn enemy_attack_warning_colors_are_high_readability() {
        let melee = enemy_attack_warning_color(&EnemyAttackKind::Melee).to_srgba();
        let projectile = enemy_attack_warning_color(&EnemyAttackKind::Projectile).to_srgba();
        let shockwave = enemy_attack_warning_color(&EnemyAttackKind::Shockwave).to_srgba();

        assert!(melee.alpha >= 0.65);
        assert!(projectile.alpha > melee.alpha);
        assert!(shockwave.red > 0.95);
        assert!(shockwave.alpha >= 0.60);
    }

    #[test]
    fn enemy_attack_poses_make_attack_types_read_differently() {
        let melee = enemy_attack_pose(&EnemyAttackKind::Melee, 0.56, 1.0);
        let projectile = enemy_attack_pose(&EnemyAttackKind::Projectile, 0.56, 1.0);
        let shockwave = enemy_attack_pose(&EnemyAttackKind::Shockwave, 0.56, 1.0);
        let idle = enemy_attack_pose(&EnemyAttackKind::Melee, 1.0, 0.0);

        assert_eq!(idle.scale, Vec3::ONE);
        assert!(melee.pitch < -0.48);
        assert!(melee.scale.z > melee.scale.y + 0.30);
        assert!(projectile.lift > melee.lift + 0.04);
        assert!(projectile.roll < -0.12);
        assert!(shockwave.pitch < melee.pitch - 0.04);
        assert!(shockwave.scale.x > melee.scale.x + 0.12);
        assert!(shockwave.scale.z > melee.scale.z);
        assert!(shockwave.scale.y < projectile.scale.y - 0.20);
    }

    #[test]
    fn expired_reaction_cleanup_preserves_a_new_hit_in_the_same_frame() {
        let mut world = World::new();
        let mut old = EnemyHitReact::new(0.14);
        old.timer.tick(std::time::Duration::from_secs_f32(0.20));
        let target = world.spawn(old).id();
        world
            .commands()
            .entity(target)
            .insert(EnemyHitReact::new(0.18).with_recoil_from(Vec3::ZERO, Vec3::Z, 0.4352));
        world
            .commands()
            .queue(move |world: &mut World| remove_expired_hit_react(world, target));
        world.flush();
        let reaction = world.get::<EnemyHitReact>(target).unwrap();
        assert_eq!(reaction.recoil_distance, 0.4352);
        assert!(!reaction.timer.is_finished());
        world
            .get_mut::<EnemyHitReact>(target)
            .unwrap()
            .timer
            .tick(std::time::Duration::from_secs_f32(0.20));
        remove_expired_hit_react(&mut world, target);
        assert!(world.get::<EnemyHitReact>(target).is_none());
    }

    #[test]
    fn recoil_covers_same_distance_at_low_and_high_frame_rates() {
        for fps in [15, 30, 60, 144] {
            let mut react = EnemyHitReact::new(0.14).with_recoil_from(Vec3::ZERO, Vec3::Z, 0.4386);
            let delta = 1.0 / fps as f32;
            let mut movement = Vec3::ZERO;
            while !react.timer.is_finished() {
                movement += enemy_hit_recoil_step(&react, delta);
                react.timer.tick(std::time::Duration::from_secs_f32(delta));
            }
            assert!(
                (movement.z - 0.4386).abs() < 0.00001,
                "{fps}: {}",
                movement.z
            );
        }
    }

    #[test]
    fn enemy_hit_react_recoils_away_from_damage_source() {
        let mut react = EnemyHitReact::new(0.20).with_recoil_from(
            Vec3::new(-1.0, 2.0, 0.0),
            Vec3::new(1.0, 0.0, 0.0),
            0.50,
        );
        let first_step = enemy_hit_recoil_step(&react, 0.04);

        react.timer.tick(std::time::Duration::from_secs_f32(0.14));
        let late_step = enemy_hit_recoil_step(&react, 0.04);

        assert!(first_step.x > 0.08);
        assert_eq!(first_step.y, 0.0);
        assert!(first_step.z.abs() < 0.001);
        assert!(late_step.x > 0.0);
        assert!(late_step.x < first_step.x * 0.45);
    }

    #[test]
    fn enemy_limb_binding_animates_nested_authored_mesh_without_spawning_copies() {
        let mut app = App::new();
        app.add_systems(
            Update,
            (bind_enemy_limbs, animate_enemy_limbs_system).chain(),
        );
        let mut animation = EnemyAnimation::new(1.7);
        animation.moving = true;
        animation.walk_phase = std::f32::consts::FRAC_PI_2;
        let owner = app
            .world_mut()
            .spawn((test_enemy("skeleton", vec![]), animation))
            .id();
        let scene_root = app.world_mut().spawn(ChildOf(owner)).id();
        let rest = Transform::from_xyz(-0.15, 0.83, 0.0).with_scale(Vec3::splat(0.9));
        let leg = app
            .world_mut()
            .spawn((Name::new("enemy_left_leg"), rest, ChildOf(scene_root)))
            .id();
        let count = app.world().entities().len();
        app.update();
        let bound = app.world().get::<EnemyLimb>(leg).unwrap();
        assert_eq!(bound.owner, owner);
        let first = *app.world().get::<Transform>(leg).unwrap();
        assert_eq!(first.translation, rest.translation);
        assert_eq!(first.scale, rest.scale);
        assert!(first.rotation.angle_between(rest.rotation) > 0.4);
        app.update();
        assert_eq!(*app.world().get::<Transform>(leg).unwrap(), first);
        assert_eq!(app.world().entities().len(), count);
    }

    #[test]
    fn authored_enemy_legs_swing_oppositely_and_rest_at_idle() {
        let pose = |kind, moving| {
            enemy_limb_rotation(
                kind,
                &EnemyAttackKind::Melee,
                moving,
                std::f32::consts::FRAC_PI_2,
                1.0,
                0.0,
            )
        };
        assert_eq!(pose(EnemyLimbKind::LeftLeg, false), Quat::IDENTITY);
        let left = pose(EnemyLimbKind::LeftLeg, true);
        let right = pose(EnemyLimbKind::RightLeg, true);
        assert!((left * right).angle_between(Quat::IDENTITY) < 0.001);
        assert!(left.angle_between(Quat::IDENTITY) > 0.4);
    }

    #[test]
    fn authored_enemy_weapon_arm_has_distinct_windup_and_swing() {
        let pose = |progress, warning| {
            enemy_limb_rotation(
                EnemyLimbKind::AttackHand,
                &EnemyAttackKind::Melee,
                false,
                0.0,
                progress,
                warning,
            )
        };
        let idle = pose(1.0, 0.0);
        let windup = pose(1.0, 0.9);
        let impact = pose(0.5, 0.0);
        assert_eq!(idle, Quat::IDENTITY);
        assert!(windup.angle_between(impact) > 2.0);
    }

    #[test]
    fn enemy_limb_contract_ignores_effects_and_body_meshes() {
        assert_eq!(EnemyLimbKind::from_node_name("enemy_body"), None);
        assert_eq!(EnemyLimbKind::from_node_name("shadow trail"), None);
        assert_eq!(
            EnemyLimbKind::from_node_name("enemy_left_leg"),
            Some(EnemyLimbKind::LeftLeg)
        );
        assert_eq!(
            EnemyLimbKind::from_node_name("enemy_right_arm"),
            Some(EnemyLimbKind::AttackHand)
        );
    }

    #[test]
    fn enemy_warning_pose_charges_before_impact_by_attack_type() {
        let early = enemy_warning_pose(&EnemyAttackKind::Melee, 0.10);
        let melee = enemy_warning_pose(&EnemyAttackKind::Melee, 0.85);
        let projectile = enemy_warning_pose(&EnemyAttackKind::Projectile, 0.85);
        let shockwave = enemy_warning_pose(&EnemyAttackKind::Shockwave, 0.85);

        assert!(melee.pitch < early.pitch - 0.17);
        assert!(melee.scale.z > early.scale.z + 0.18);
        assert!(projectile.lift > melee.lift + 0.11);
        assert!(projectile.pitch > 0.22);
        assert!(shockwave.pitch < melee.pitch - 0.08);
        assert!(shockwave.scale.x > melee.scale.x + 0.08);
        assert!(shockwave.scale.z > melee.scale.z);
        assert!(shockwave.scale.y < projectile.scale.y - 0.14);
    }

    #[test]
    fn player_damage_feedback_reports_evade_ward_health_and_danger() {
        let healthy = Health {
            current: 80.0,
            max: 100.0,
        };
        let low = Health {
            current: 28.0,
            max: 100.0,
        };

        let evaded = DamageTaken {
            absorbed: 0.0,
            health_damage: 0.0,
        };
        let warded = DamageTaken {
            absorbed: 14.0,
            health_damage: 0.0,
        };
        let mixed = DamageTaken {
            absorbed: 9.0,
            health_damage: 11.0,
        };

        assert_eq!(
            player_damage_feedback_text("Ashbone Guard", evaded, &healthy),
            "Ashbone Guard evaded"
        );
        assert_eq!(player_damage_floating_text(evaded), "EVADE");
        assert!(player_damage_screen_shake(evaded).is_none());

        assert_eq!(
            player_damage_feedback_text("Ashbone Guard", warded, &healthy),
            "Ashbone Guard absorbed by ward for 14"
        );
        assert_eq!(player_damage_floating_text(warded), "WARD -14");
        assert!(player_damage_screen_shake(warded).unwrap().intensity < 0.06);

        assert_eq!(
            player_damage_feedback_text("Ashbone Guard", mixed, &low),
            "Low life: Ashbone Guard hit for 11 (9 ward)"
        );
        assert_eq!(player_damage_floating_text(mixed), "-11  WARD -9");
        assert!(player_damage_screen_shake(mixed).unwrap().intensity > 0.07);
    }

    #[test]
    fn player_damage_impacts_distinguish_health_ward_and_evade() {
        let evaded = DamageTaken {
            absorbed: 0.0,
            health_damage: 0.0,
        };
        let warded = DamageTaken {
            absorbed: 18.0,
            health_damage: 0.0,
        };
        let health = DamageTaken {
            absorbed: 4.0,
            health_damage: 22.0,
        };

        assert_eq!(
            player_damage_impact_kind(evaded),
            PlayerDamageImpactKind::Evade
        );
        assert_eq!(
            player_damage_impact_kind(warded),
            PlayerDamageImpactKind::Ward
        );
        assert_eq!(
            player_damage_impact_kind(health),
            PlayerDamageImpactKind::Health
        );

        let evade_style = player_damage_impact_style(evaded);
        let ward_style = player_damage_impact_style(warded);
        let health_style = player_damage_impact_style(health);

        assert!(health_style.scale.x > evade_style.scale.x);
        assert!(ward_style.scale.x > evade_style.scale.x);
        assert!(health_style.lifetime_secs > evade_style.lifetime_secs);
        assert!(ward_style.color.to_srgba().green > health_style.color.to_srgba().green);
    }

    #[test]
    fn player_health_impact_spawns_only_small_short_lived_non_damaging_sparks() {
        let mut app = App::new();
        app.init_resource::<Assets<Mesh>>()
            .init_resource::<Assets<StandardMaterial>>()
            .add_systems(
                Startup,
                |mut commands: Commands,
                 mut meshes: ResMut<Assets<Mesh>>,
                 mut materials: ResMut<Assets<StandardMaterial>>| {
                    spawn_player_damage_impact(
                        &mut commands,
                        &mut meshes,
                        &mut materials,
                        -Vec3::Z,
                        Vec3::ZERO,
                        DamageTaken {
                            absorbed: 0.0,
                            health_damage: 10_000.0,
                        },
                    );
                },
            );
        app.update();
        let mut query = app
            .world_mut()
            .query::<(&Mesh3d, &Transform, &EnemyProjectile)>();
        let sparks: Vec<_> = query.iter(app.world()).collect();
        assert_eq!(sparks.len(), 5);
        let meshes = app.world().resource::<Assets<Mesh>>();
        for (handle, transform, projectile) in sparks {
            let mesh = meshes.get(&handle.0).unwrap();
            let bevy::mesh::VertexAttributeValues::Float32x3(vertices) =
                mesh.attribute(Mesh::ATTRIBUTE_POSITION).unwrap()
            else {
                panic!("missing spark vertices")
            };
            assert_eq!(vertices.len(), 36);
            for point in vertices {
                let local = Vec3::from_array(*point) * transform.scale;
                assert!(
                    local.length() <= 0.091,
                    "hit mesh must not obscure the body"
                );
            }
            assert_eq!(projectile.damage, 0.0);
            assert_eq!(projectile.radius, 0.0);
            assert!(projectile.velocity.length() > 1.0);
            assert!(projectile.lifetime.duration().as_secs_f32() <= 0.18);
            let mut lifetime = projectile.lifetime.clone();
            lifetime.tick(std::time::Duration::from_millis(200));
            assert!(lifetime.is_finished());
        }
    }

    #[test]
    fn player_damage_impact_transform_tracks_attack_direction() {
        let source = Vec3::new(-1.0, 0.0, 0.0);
        let target = Vec3::new(1.0, 0.0, 0.0);
        let health = DamageTaken {
            absorbed: 0.0,
            health_damage: 20.0,
        };
        let evade = DamageTaken {
            absorbed: 0.0,
            health_damage: 0.0,
        };

        let hit = player_damage_impact_transform(source, target, health);
        let dodge = player_damage_impact_transform(source, target, evade);

        assert!(hit.translation.x < target.x);
        assert!(hit.translation.y > target.y + 0.8);
        assert!((hit.translation.z - target.z).abs() < 0.01);
        assert!(dodge.translation.z < target.z - 0.30);
        assert!(dodge.translation.y < hit.translation.y);
    }

    #[test]
    fn treasure_fiend_runs_away_from_player() {
        let direction = treasure_flee_direction(Vec3::new(2.0, 0.0, 0.0));

        assert_eq!(direction, Vec3::new(-1.0, -0.0, -0.0));
    }

    #[test]
    fn nemesis_spawns_near_checkpoint_with_elite_affixes() {
        let checkpoint = Vec3::new(10.4, 0.0, -6.4);
        let offset = chapter_world_position(ChapterId::AshenReliquary, Vec3::new(2.6, 0.0, -1.8));
        let position = nemesis_spawn_position(ChapterId::AshenReliquary, checkpoint);
        let affixes = nemesis_affixes();

        assert_eq!(position, checkpoint + offset);
        assert!(
            nemesis_spawn_position(ChapterId::HollowBastion, checkpoint).x < checkpoint.x,
            "the mirrored chapter should spawn the nemesis on the opposite flank"
        );
        let distant_checkpoint = Vec3::new(10_000.0, 0.0, -10_000.0);
        assert_eq!(
            nemesis_spawn_position(ChapterId::AshenReliquary, distant_checkpoint),
            clamp_world_translation(distant_checkpoint + offset)
        );
        assert!(affixes.contains(&EnemyAffix::Frenzied));
        assert!(affixes.contains(&EnemyAffix::Vampiric));
        assert!(affixes.contains(&EnemyAffix::Jailer));
        assert!(affixes.contains(&EnemyAffix::Frozen));
        assert!(affixes.contains(&EnemyAffix::Desecrator));
        assert!(affixes.contains(&EnemyAffix::Reflective));
    }
}

#[cfg(test)]
mod summon_target_tests {
    use super::*;
    use bevy::ecs::system::RunSystemOnce;

    #[test]
    fn nearby_living_guard_intercepts_but_dead_guard_does_not() {
        let mut world = World::new();
        let pet = world
            .spawn((
                Minion,
                Transform::from_xyz(0.0, 0.0, 2.0),
                SummonVitality {
                    current: 20.0,
                    max: 20.0,
                    dead_secs: 0.0,
                    guard_secs: 0.0,
                    guard_multiplier: 0.45,
                },
            ))
            .id();
        let target = world
            .run_system_once(|pets: MinionDamageQuery| {
                minion_attack_target(Vec3::new(0.0, 0.0, 4.0), Vec3::ZERO, &pets)
            })
            .unwrap();
        assert_eq!(target.map(|(entity, _)| entity), Some(pet));
        world.get_mut::<SummonVitality>(pet).unwrap().current = 0.0;
        assert!(
            world
                .run_system_once(|pets: MinionDamageQuery| {
                    minion_attack_target(Vec3::new(0.0, 0.0, 4.0), Vec3::ZERO, &pets)
                })
                .unwrap()
                .is_none()
        );
    }

    #[test]
    fn fast_projectile_hits_guard_between_frame_endpoints() {
        let start = Vec3::new(-3.0, 0.8, 0.0);
        assert!(projectile_segment_hits(
            start,
            Vec3::X * 6.0,
            Vec3::Y * 0.8,
            0.3
        ));
        assert!(!projectile_segment_hits(
            start,
            Vec3::X * 6.0,
            Vec3::new(0.0, 0.8, 1.0),
            0.3
        ));
        assert!(!projectile_segment_hits(start, Vec3::X, Vec3::Y * 0.8, 0.3));
    }
}
