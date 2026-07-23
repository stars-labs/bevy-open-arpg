use crate::{
    Difficulty, DifficultySettings, GameState, RunStats,
    assets::GameAssets,
    chapter::{
        ChapterPhase, ChapterProgress, InteractableKind, InteractableUsed, SpawnBoss,
        SpawnSanctumWave,
    },
    data::{EnemyAttackKind, EnemyCatalog, EnemyTuning},
    feedback::{CombatEvent, FloatingCombatTextEvent, ScreenShakeEvent},
    not_paused,
    obelisk::SpawnObeliskWave,
    ordeal::ChapterModifier,
    player::{
        Barrier, Burning, DamageTaken, ElixirBuff, Equipment, Evade, Health, Jailed, Player,
        PlayerHitReact, PlayerHitReactKind, apply_player_damage_with_evade, mitigated_damage,
        total_armor,
    },
    rift::SpawnRiftWave,
};
use bevy::ecs::system::SystemParam;
use bevy::prelude::*;

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
    if enemy.id == "keeper" {
        1.55
    } else if enemy.id == "seal_warden" {
        1.18
    } else if enemy.id == "nemesis" || !enemy.affixes.is_empty() {
        1.05
    } else {
        0.76
    }
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
enum EnemyMotionVisualKind {
    AttackHand,
    OffHand,
    LeftStep,
    RightStep,
}

impl EnemyMotionVisualKind {
    fn label(self) -> &'static str {
        match self {
            Self::AttackHand => "Enemy Attack Hand",
            Self::OffHand => "Enemy Off Hand",
            Self::LeftStep => "Enemy Left Step",
            Self::RightStep => "Enemy Right Step",
        }
    }

    fn side(self) -> f32 {
        match self {
            Self::AttackHand | Self::RightStep => 1.0,
            Self::OffHand | Self::LeftStep => -1.0,
        }
    }
}

#[derive(Component)]
struct EnemyMotionVisual {
    kind: EnemyMotionVisualKind,
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
    let elapsed_ratio =
        (hit_react.timer.elapsed_secs() / hit_react.duration_secs.max(0.001)).clamp(0.0, 1.0);
    let remaining_weight = (1.0 - elapsed_ratio).powf(1.35);
    hit_react.recoil_direction
        * hit_react.recoil_distance
        * remaining_weight
        * (delta_secs / hit_react.duration_secs.max(0.001))
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
    ),
>;
type EnemyMotionVisualQuery<'w, 's> = Query<
    'w,
    's,
    (
        &'static ChildOf,
        &'static EnemyMotionVisual,
        &'static mut Transform,
        &'static mut Visibility,
    ),
    (With<EnemyMotionVisual>, Without<Enemy>),
>;
type EnemyMotionSourceQuery<'w, 's> =
    Query<'w, 's, (Entity, &'static Enemy, &'static EnemyAnimation)>;

#[derive(SystemParam)]
struct EnemyFeedbackWriters<'w> {
    combat_events: MessageWriter<'w, CombatEvent>,
    floating_text: MessageWriter<'w, FloatingCombatTextEvent>,
    screen_shake: MessageWriter<'w, ScreenShakeEvent>,
}

impl Plugin for EnemyPlugin {
    fn build(&self, app: &mut App) {
        app.add_message::<EnemyKilled>()
            .add_message::<SpawnNemesis>()
            .add_message::<SpawnAshenThreatWave>()
            .add_systems(OnEnter(GameState::InGame), spawn_enemies)
            .add_systems(
                Update,
                (
                    (enemy_ai, animate_enemy_motion_visuals_system).chain(),
                    tick_chilled,
                    tick_staggered,
                    tick_elite_affix_auras,
                    attach_main_quest_enemy_markers,
                    update_main_quest_enemy_markers,
                    boss_phase_transition,
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
            .add_systems(OnExit(GameState::InGame), despawn_enemies);
    }
}

fn spawn_enemies(
    mut commands: Commands,
    mut combat_events: MessageWriter<CombatEvent>,
    assets: Res<GameAssets>,
    catalog: Res<EnemyCatalog>,
    difficulty: Res<DifficultySettings>,
    modifier: Res<ChapterModifier>,
) {
    // Keep the guard wave ahead of the player spawn (0, 0, 4): an ARPG opener
    // should give a few seconds to read the room before first contact, not
    // spawn a marksman on top of the player.
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

    for (id, position, affixes) in spawns {
        let Some(tuning) = catalog.get(id) else {
            continue;
        };
        let scene = match tuning.model.as_str() {
            "skeleton.glb" => assets.skeleton.clone(),
            "cultist.glb" => assets.cultist.clone(),
            "butcher.glb" => assets.butcher.clone(),
            _ => assets.skeleton.clone(),
        };
        spawn_enemy(
            &mut commands,
            &assets,
            EnemySpawnSpec {
                scene,
                tuning,
                position,
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
        Vec3::new(-8.5, 0.0, 5.6),
    );

    spawn_champion_pack(
        &mut commands,
        &assets,
        &catalog,
        difficulty.current,
        *modifier,
    );
    combat_events.write(CombatEvent {
        text: "A reliquary champion pack patrols the outer halls".to_string(),
    });
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
) {
    for (id, position, affixes) in champion_pack_spawns() {
        let Some(tuning) = catalog.get(&id) else {
            continue;
        };
        let scene = match tuning.model.as_str() {
            "cultist.glb" => assets.cultist.clone(),
            "butcher.glb" => assets.butcher.clone(),
            _ => assets.skeleton.clone(),
        };
        let entity = spawn_enemy(
            commands,
            assets,
            EnemySpawnSpec {
                scene,
                tuning,
                position,
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
    mut commands: Commands,
    mut wave_events: MessageReader<SpawnSanctumWave>,
    mut combat_events: MessageWriter<CombatEvent>,
    assets: Res<GameAssets>,
    catalog: Res<EnemyCatalog>,
    difficulty: Res<DifficultySettings>,
    modifier: Res<ChapterModifier>,
) {
    for _ in wave_events.read() {
        for (id, position, affixes) in [
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
            let Some(tuning) = catalog.get(id) else {
                continue;
            };
            let scene = match tuning.model.as_str() {
                "cultist.glb" => assets.cultist.clone(),
                "butcher.glb" => assets.butcher.clone(),
                _ => assets.skeleton.clone(),
            };
            spawn_enemy(
                &mut commands,
                &assets,
                EnemySpawnSpec {
                    scene,
                    tuning,
                    position,
                    difficulty: difficulty.current,
                    modifier: *modifier,
                    affixes,
                },
            );
        }
        combat_events.write(CombatEvent {
            text: "Seal Warden Vhal guards the second reliquary seal".to_string(),
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
    mut commands: Commands,
    mut interactions: MessageReader<InteractableUsed>,
    mut combat_events: MessageWriter<CombatEvent>,
    assets: Res<GameAssets>,
    catalog: Res<EnemyCatalog>,
    difficulty: Res<DifficultySettings>,
    modifier: Res<ChapterModifier>,
) {
    for event in interactions.read() {
        if event.kind != InteractableKind::CursedShrine {
            continue;
        }
        for (id, position, affixes) in cursed_shrine_spawns(event.position) {
            let Some(tuning) = catalog.get(&id) else {
                continue;
            };
            let scene = match tuning.model.as_str() {
                "cultist.glb" => assets.cultist.clone(),
                "butcher.glb" => assets.butcher.clone(),
                _ => assets.skeleton.clone(),
            };
            let entity = spawn_enemy(
                &mut commands,
                &assets,
                EnemySpawnSpec {
                    scene,
                    tuning,
                    position,
                    difficulty: difficulty.current,
                    modifier: *modifier,
                    affixes,
                },
            );
            commands.entity(entity).try_insert(CursedAmbusher);
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
    mut commands: Commands,
    mut events: MessageReader<SpawnObeliskWave>,
    mut combat_events: MessageWriter<CombatEvent>,
    assets: Res<GameAssets>,
    catalog: Res<EnemyCatalog>,
    difficulty: Res<DifficultySettings>,
    modifier: Res<ChapterModifier>,
) {
    for event in events.read() {
        for (id, position, affixes) in obelisk_spawns(event.origin) {
            let Some(tuning) = catalog.get(&id) else {
                continue;
            };
            let scene = match tuning.model.as_str() {
                "cultist.glb" => assets.cultist.clone(),
                "butcher.glb" => assets.butcher.clone(),
                _ => assets.skeleton.clone(),
            };
            spawn_enemy(
                &mut commands,
                &assets,
                EnemySpawnSpec {
                    scene,
                    tuning,
                    position,
                    difficulty: difficulty.current,
                    modifier: *modifier,
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
    mut commands: Commands,
    mut events: MessageReader<SpawnRiftWave>,
    mut combat_events: MessageWriter<CombatEvent>,
    assets: Res<GameAssets>,
    catalog: Res<EnemyCatalog>,
    difficulty: Res<DifficultySettings>,
    modifier: Res<ChapterModifier>,
) {
    for event in events.read() {
        for (id, position, affixes) in rift_spawns(event.origin) {
            let Some(tuning) = catalog.get(&id) else {
                continue;
            };
            let scene = match tuning.model.as_str() {
                "cultist.glb" => assets.cultist.clone(),
                "butcher.glb" => assets.butcher.clone(),
                _ => assets.skeleton.clone(),
            };
            spawn_enemy(
                &mut commands,
                &assets,
                EnemySpawnSpec {
                    scene,
                    tuning,
                    position,
                    difficulty: difficulty.current,
                    modifier: *modifier,
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
    mut commands: Commands,
    mut events: MessageReader<SpawnAshenThreatWave>,
    mut combat_events: MessageWriter<CombatEvent>,
    assets: Res<GameAssets>,
    catalog: Res<EnemyCatalog>,
    difficulty: Res<DifficultySettings>,
    modifier: Res<ChapterModifier>,
) {
    for event in events.read() {
        for (id, position, affixes) in ashen_threat_spawns(event.origin, event.surge) {
            let Some(tuning) = catalog.get(&id) else {
                continue;
            };
            let scene = match tuning.model.as_str() {
                "cultist.glb" => assets.cultist.clone(),
                "butcher.glb" => assets.butcher.clone(),
                _ => assets.skeleton.clone(),
            };
            spawn_enemy(
                &mut commands,
                &assets,
                EnemySpawnSpec {
                    scene,
                    tuning,
                    position,
                    difficulty: difficulty.current,
                    modifier: *modifier,
                    affixes,
                },
            );
        }
        combat_events.write(CombatEvent {
            text: format!(
                "Ashen threat surge {} opened a retaliation wave",
                event.surge
            ),
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
) {
    for _ in boss_events.read() {
        let Some(tuning) = catalog.get("keeper") else {
            continue;
        };
        let boss_position = Vec3::new(0.0, 0.0, -20.0);
        commands.spawn((
            WorldAssetRoot(assets.boss_summon_portal.clone()),
            Transform::from_translation(boss_position).with_scale(Vec3::splat(1.18)),
            EnemyEntity,
            Name::new("Malrec Summoning Seal"),
        ));
        commands.spawn((
            PointLight {
                color: Color::srgb(1.0, 0.18, 0.04),
                intensity: 950.0,
                range: 8.0,
                shadow_maps_enabled: false,
                ..default()
            },
            Transform::from_translation(boss_position + Vec3::Y * 1.35),
            EnemyEntity,
            Name::new("Malrec Summoning Light"),
        ));
        let boss = spawn_enemy(
            &mut commands,
            &assets,
            EnemySpawnSpec {
                scene: assets.butcher.clone(),
                tuning,
                position: boss_position,
                difficulty: difficulty.current,
                modifier: *modifier,
                affixes: vec![
                    EnemyAffix::Molten,
                    EnemyAffix::Vampiric,
                    EnemyAffix::Shielded,
                    EnemyAffix::Arcane,
                    EnemyAffix::Jailer,
                    EnemyAffix::Frozen,
                    EnemyAffix::Desecrator,
                    EnemyAffix::Reflective,
                ],
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

fn spawn_nemesis(
    mut commands: Commands,
    mut events: MessageReader<SpawnNemesis>,
    mut combat_events: MessageWriter<CombatEvent>,
    assets: Res<GameAssets>,
    catalog: Res<EnemyCatalog>,
    difficulty: Res<DifficultySettings>,
    modifier: Res<ChapterModifier>,
) {
    for event in events.read() {
        let Some(tuning) = catalog.get("nemesis") else {
            continue;
        };
        spawn_enemy(
            &mut commands,
            &assets,
            EnemySpawnSpec {
                scene: assets.skeleton.clone(),
                tuning,
                position: nemesis_spawn_position(event.checkpoint),
                difficulty: difficulty.current,
                modifier: *modifier,
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
    let attack_cooldown = tuning.attack_cooldown * if has_frenzied { 0.72 } else { 1.0 };
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
                move_speed: tuning.move_speed * if has_frenzied { 1.28 } else { 1.0 },
                gold_min: scale_reward(tuning.gold_min, reward_multiplier),
                gold_max: scale_reward(tuning.gold_max, reward_multiplier),
                xp_reward: scale_reward(tuning.xp_reward, reward_multiplier),
            },
            EnemyAnimation::new(base_scale),
            EnemyEntity,
            Name::new(display_name),
        ))
        .id();
    if tuning.id == "seal_warden" {
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
                Name::new("Elite Affix Aura Light"),
            ));
        });
    }
    commands.entity(entity).with_children(|parent| {
        for kind in [
            EnemyMotionVisualKind::AttackHand,
            EnemyMotionVisualKind::OffHand,
            EnemyMotionVisualKind::LeftStep,
            EnemyMotionVisualKind::RightStep,
        ] {
            let scene = enemy_motion_visual_scene(assets, kind, &tuning.attack_kind);
            let pose = enemy_motion_visual_pose(
                kind,
                EnemyMotionVisualContext {
                    attack_kind: &tuning.attack_kind,
                    moving: false,
                    walk_phase: 0.0,
                    attack_progress: 1.0,
                    warning_progress: 0.0,
                    hit_react_power: 0.0,
                    base_scale,
                },
            );
            parent.spawn((
                WorldAssetRoot(scene),
                Transform::from_translation(pose.translation)
                    .with_rotation(pose.rotation)
                    .with_scale(pose.scale),
                pose.visibility,
                EnemyMotionVisual { kind },
                Name::new(kind.label()),
            ));
        }
    });
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
    match (progress.phase, enemy.id.as_str()) {
        (ChapterPhase::Sanctum, "seal_warden") => MainQuestEnemyMarkerState::SealWarden,
        (ChapterPhase::Boss, "keeper") if !progress.boss_slain => MainQuestEnemyMarkerState::Keeper,
        _ => MainQuestEnemyMarkerState::Hidden,
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

fn enemy_motion_visual_scene(
    assets: &GameAssets,
    kind: EnemyMotionVisualKind,
    attack_kind: &EnemyAttackKind,
) -> Handle<WorldAsset> {
    match kind {
        EnemyMotionVisualKind::AttackHand => match attack_kind {
            EnemyAttackKind::Melee => assets.sword.clone(),
            EnemyAttackKind::Projectile => assets.ember_impact.clone(),
            EnemyAttackKind::Shockwave => assets.void_impact.clone(),
        },
        EnemyMotionVisualKind::OffHand => assets.hit_bone_rune.clone(),
        EnemyMotionVisualKind::LeftStep | EnemyMotionVisualKind::RightStep => {
            assets.shadow_trail.clone()
        }
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

fn nemesis_spawn_position(checkpoint: Vec3) -> Vec3 {
    let mut position = checkpoint + Vec3::new(2.6, 0.0, -1.8);
    position.x = position.x.clamp(-10.8, 10.8);
    position.z = position.z.clamp(-6.8, 6.8);
    position
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
    mut commands: Commands,
    mut feedback: EnemyFeedbackWriters,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut enemies: EnemyAiQuery,
    mut player: PlayerHealthQuery,
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
    ) in &mut enemies
    {
        let mut hit_react_power = 0.0;
        if let Some(mut hit_react) = hit_react {
            hit_react.timer.tick(time.delta());
            hit_react_power =
                (1.0 - hit_react.timer.elapsed_secs() / hit_react.duration_secs).clamp(0.0, 1.0);
            transform.translation += enemy_hit_recoil_step(&hit_react, time.delta_secs());
            transform.translation.x = transform.translation.x.clamp(-11.5, 11.5);
            transform.translation.z = transform.translation.z.clamp(-7.5, 7.5);
            if hit_react.timer.is_finished()
                && let Ok(mut entity_commands) = commands.get_entity(entity)
            {
                entity_commands.remove::<EnemyHitReact>();
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
        let to_player = player_transform.translation - transform.translation;
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
                        text: "The treasure imp escaped into the ash vents".to_string(),
                    });
                    if let Ok(mut entity_commands) = commands.get_entity(entity) {
                        entity_commands.try_despawn();
                    }
                    continue;
                }
            }
        }

        if distance > enemy.aggro_range {
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
            transform.translation.x = transform.translation.x.clamp(-11.5, 11.5);
            transform.translation.z = transform.translation.z.clamp(-7.5, 7.5);
            if plan.facing_direction.length_squared() > 0.0 {
                animation.facing_yaw = plan.facing_direction.x.atan2(plan.facing_direction.z);
                moved = true;
            }
        } else if enemy.attack_timer.is_finished() {
            if let Ok(mut entity_commands) = commands.get_entity(entity) {
                entity_commands.remove::<EnemyAttackWarning>();
            }
            animation.trigger_attack(match enemy.attack_kind {
                EnemyAttackKind::Melee => 0.32,
                EnemyAttackKind::Projectile => 0.42,
                EnemyAttackKind::Shockwave => 0.52,
            });
            spawn_enemy_attack_impact(
                &mut commands,
                &mut meshes,
                &mut materials,
                transform.translation,
                flat_to_player.normalize_or_zero(),
                &enemy.attack_kind,
                enemy.attack_range,
            );
            match enemy.attack_kind {
                EnemyAttackKind::Melee => {
                    let damage = mitigated_damage(enemy.attack_damage, player_armor);
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
                }
                EnemyAttackKind::Projectile => spawn_projectile(
                    &mut commands,
                    &mut meshes,
                    &mut materials,
                    transform.translation + Vec3::Y,
                    flat_to_player.normalize_or_zero(),
                    enemy.attack_damage,
                ),
                EnemyAttackKind::Shockwave => {
                    if distance <= enemy.attack_range + 0.85 {
                        let damage = mitigated_damage(enemy.attack_damage, player_armor);
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
                }
            }
            if enemy.affixes.contains(&EnemyAffix::Arcane) {
                spawn_arcane_sentry(
                    &mut commands,
                    &mut meshes,
                    &mut materials,
                    player_transform.translation,
                    enemy.attack_damage * 0.55,
                );
            }
            if enemy.affixes.contains(&EnemyAffix::Jailer) {
                spawn_jailer_prison(
                    &mut commands,
                    &mut meshes,
                    &mut materials,
                    player_transform.translation,
                    enemy.attack_damage * 0.35,
                );
                commands.entity(player_entity).try_insert(Jailed {
                    timer: Timer::from_seconds(1.15, TimerMode::Once),
                    speed_multiplier: 0.18,
                });
                feedback.combat_events.write(CombatEvent {
                    text: "Jailer affix: ash chains bind your movement".to_string(),
                });
            }
            if enemy.affixes.contains(&EnemyAffix::Frozen) {
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
            if enemy.affixes.contains(&EnemyAffix::Desecrator) {
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
            if enemy.id == "seal_warden" {
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
            let warning_window = enemy_attack_warning_window(&enemy.attack_kind);
            if !attack_warning_active && remaining > 0.08 && remaining <= warning_window {
                spawn_enemy_attack_warning(
                    &mut commands,
                    &mut meshes,
                    &mut materials,
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
    if enemy.id == "ashen_marksman" && distance < marksman_min_spacing(enemy) {
        let away = -to_player;
        let strafe = marksman_strafe_direction(enemy_position, to_player);
        return Some(EnemyMovementPlan {
            movement_direction: (away * 0.82 + strafe * 0.46).normalize_or_zero(),
            facing_direction: to_player,
            speed_multiplier: 1.16,
        });
    }

    let desired_range = enemy_desired_attack_range(enemy);
    if distance > desired_range {
        return Some(EnemyMovementPlan {
            movement_direction: to_player,
            facing_direction: to_player,
            speed_multiplier: enemy_approach_speed_multiplier(enemy),
        });
    }

    None
}

fn enemy_desired_attack_range(enemy: &Enemy) -> f32 {
    match enemy.id.as_str() {
        "bone_stalker" => enemy.attack_range * 0.88,
        "ashen_marksman" => enemy.attack_range * 0.88,
        "reliquary_brute" => enemy.attack_range * 0.92,
        _ => match enemy.attack_kind {
            EnemyAttackKind::Melee | EnemyAttackKind::Shockwave => enemy.attack_range,
            EnemyAttackKind::Projectile => enemy.attack_range * 0.82,
        },
    }
}

fn marksman_min_spacing(enemy: &Enemy) -> f32 {
    enemy.attack_range * 0.46
}

fn enemy_approach_speed_multiplier(enemy: &Enemy) -> f32 {
    match enemy.id.as_str() {
        "bone_stalker" => 1.18,
        "reliquary_brute" => 0.74,
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

fn animate_enemy_motion_visuals_system(
    enemies: EnemyMotionSourceQuery,
    mut visuals: EnemyMotionVisualQuery,
) {
    for (child_of, visual, mut transform, mut visibility) in visuals.iter_mut() {
        let Ok((_, enemy, animation)) = enemies.get(child_of.parent()) else {
            continue;
        };
        let pose = enemy_motion_visual_pose(
            visual.kind,
            EnemyMotionVisualContext {
                attack_kind: &enemy.attack_kind,
                moving: animation.moving,
                walk_phase: animation.walk_phase,
                attack_progress: enemy_attack_progress(animation),
                warning_progress: animation.warning_progress,
                hit_react_power: animation.hit_react_power,
                base_scale: animation.base_scale,
            },
        );
        transform.translation = pose.translation;
        transform.rotation = pose.rotation;
        transform.scale = pose.scale;
        *visibility = pose.visibility;
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct EnemyMotionVisualPose {
    translation: Vec3,
    rotation: Quat,
    scale: Vec3,
    visibility: Visibility,
}

#[derive(Debug, Clone, Copy)]
struct EnemyMotionVisualContext<'a> {
    attack_kind: &'a EnemyAttackKind,
    moving: bool,
    walk_phase: f32,
    attack_progress: f32,
    warning_progress: f32,
    hit_react_power: f32,
    base_scale: f32,
}

fn enemy_attack_progress(animation: &EnemyAnimation) -> f32 {
    if animation.attack_timer.is_finished() {
        1.0
    } else {
        (animation.attack_timer.elapsed_secs() / animation.attack_duration_secs.max(0.001))
            .clamp(0.0, 1.0)
    }
}

fn enemy_motion_visual_pose(
    kind: EnemyMotionVisualKind,
    context: EnemyMotionVisualContext,
) -> EnemyMotionVisualPose {
    let side = kind.side();
    let stride = if context.moving {
        (context.walk_phase + side * std::f32::consts::FRAC_PI_2).sin()
    } else {
        0.0
    };
    let attack_drive = if context.attack_progress >= 1.0 {
        0.0
    } else {
        (context.attack_progress.clamp(0.0, 1.0) * std::f32::consts::PI).sin()
    };
    let impact = if context.attack_progress >= 1.0 {
        0.0
    } else {
        (1.0 - (context.attack_progress - 0.56).abs() / 0.28).clamp(0.0, 1.0)
    };
    let warning = context.warning_progress.clamp(0.0, 1.0);
    let hit_snap = (context.hit_react_power * std::f32::consts::PI)
        .sin()
        .max(0.0);
    let scale = context.base_scale.max(0.2);

    match kind {
        EnemyMotionVisualKind::AttackHand => {
            let (translation, rotation, local_scale) = match context.attack_kind {
                EnemyAttackKind::Melee => (
                    Vec3::new(
                        0.40 + 0.16 * attack_drive,
                        0.78 + 0.14 * warning + 0.07 * hit_snap,
                        0.26 + 0.64 * impact - 0.18 * warning,
                    ),
                    Quat::from_rotation_x(-0.72 - 1.26 * impact + 0.26 * warning)
                        * Quat::from_rotation_y(0.26)
                        * Quat::from_rotation_z(-0.96 - 0.48 * attack_drive),
                    Vec3::splat(0.50 + 0.20 * impact),
                ),
                EnemyAttackKind::Projectile => (
                    Vec3::new(0.26, 1.02 + 0.38 * warning + 0.26 * attack_drive, 0.40),
                    Quat::from_rotation_x(-0.16 + 0.46 * warning)
                        * Quat::from_rotation_y(-0.28 * attack_drive)
                        * Quat::from_rotation_z(-0.34),
                    Vec3::splat(0.34 + 0.28 * warning + 0.16 * impact),
                ),
                EnemyAttackKind::Shockwave => (
                    Vec3::new(0.36 + 0.16 * attack_drive, 0.46 - 0.14 * impact, 0.54),
                    Quat::from_rotation_x(-1.14 - 0.48 * impact) * Quat::from_rotation_z(-0.44),
                    Vec3::splat(0.44 + 0.26 * attack_drive + 0.12 * warning),
                ),
            };
            EnemyMotionVisualPose {
                translation: translation * scale,
                rotation,
                scale: local_scale * scale,
                visibility: Visibility::Visible,
            }
        }
        EnemyMotionVisualKind::OffHand => {
            let lift = match context.attack_kind {
                EnemyAttackKind::Projectile => 0.28 * warning + 0.20 * attack_drive,
                EnemyAttackKind::Shockwave => -0.07 * impact,
                EnemyAttackKind::Melee => 0.07 * attack_drive,
            };
            EnemyMotionVisualPose {
                translation: Vec3::new(-0.34, 0.70 + lift, 0.18 - 0.16 * impact) * scale,
                rotation: Quat::from_rotation_x(-0.26 - 0.58 * impact)
                    * Quat::from_rotation_z(0.48 + 0.24 * warning),
                scale: Vec3::splat((0.24 + 0.09 * warning + 0.06 * attack_drive) * scale),
                visibility: Visibility::Visible,
            }
        }
        EnemyMotionVisualKind::LeftStep | EnemyMotionVisualKind::RightStep => {
            let visible = context.moving || attack_drive > 0.05 || hit_snap > 0.05;
            let press = if context.moving { stride.max(0.0) } else { 0.0 };
            EnemyMotionVisualPose {
                translation: Vec3::new(side * 0.32, 0.045, -0.22 - press * 0.20) * scale,
                rotation: Quat::from_rotation_y(side * 0.30 * stride),
                scale: Vec3::new(0.30 + press * 0.16, 0.12, 0.20 + press * 0.13) * scale,
                visibility: if visible {
                    Visibility::Visible
                } else {
                    Visibility::Hidden
                },
            }
        }
    }
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
    transform.translation.x = transform.translation.x.clamp(-11.5, 11.5);
    transform.translation.z = transform.translation.z.clamp(-7.5, 7.5);
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
                        &resources.assets,
                        &resources.catalog,
                        transform.translation,
                        resources.difficulty.current,
                        *resources.modifier,
                        boss_enrage_reinforcements(),
                    );
                    combat_events.write(CombatEvent {
                        text: boss_enrage_event_text().to_string(),
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
            enemy.display_name = display_name_with_affixes("Malrec, Keeper of Ash", &enemy.affixes);
        }
        enemy.attack_damage *= 1.18;
        enemy.move_speed *= 1.15;
        health.current = (health.current + health.max * 0.08).min(health.max);

        spawn_boss_reinforcement_wave(
            &mut resources.commands,
            &resources.assets,
            &resources.catalog,
            transform.translation,
            resources.difficulty.current,
            *resources.modifier,
            boss_phase_two_reinforcements(),
        );
        combat_events.write(CombatEvent {
            text: boss_phase_two_event_text(BOSS_ENRAGE_DELAY_SECS),
        });
    }
}

fn spawn_boss_reinforcement_wave(
    commands: &mut Commands,
    assets: &GameAssets,
    catalog: &EnemyCatalog,
    origin: Vec3,
    difficulty: Difficulty,
    modifier: ChapterModifier,
    wave: impl IntoIterator<Item = BossReinforcementWaveSpec>,
) {
    for reinforcement in wave {
        spawn_boss_reinforcement(
            commands,
            assets,
            catalog,
            BossReinforcementSpec {
                difficulty,
                modifier,
                id: reinforcement.id,
                position: origin + reinforcement.offset,
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
    let scene = match tuning.model.as_str() {
        "cultist.glb" => assets.cultist.clone(),
        "butcher.glb" => assets.butcher.clone(),
        _ => assets.skeleton.clone(),
    };
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

pub fn boss_pressure_summary(health: &Health, phase: &BossPhase) -> String {
    let health_percent = if health.max > 0.0 {
        (health.current.max(0.0) / health.max * 100.0).round()
    } else {
        0.0
    };
    if phase.enrage_started {
        return format!(
            "Malrec: Ashen Enrage {:.0}% | kite burning floor",
            health_percent
        );
    }
    if phase.phase_two_started {
        return format!(
            "Malrec: phase two {:.0}% | break before enrage {:.0}s",
            health_percent,
            phase.enrage_timer.remaining_secs()
        );
    }
    format!("Malrec: phase one {:.0}%", health_percent)
}

fn boss_phase_two_event_text(enrage_delay_secs: f32) -> String {
    format!(
        "Boss Phase II: BREAK MALREC before enrage {:.0}s | stagger opens EXECUTE WINDOW | kill stalker, marksman, brute",
        enrage_delay_secs.max(0.0)
    )
}

fn boss_enrage_event_text() -> &'static str {
    "Ashen Enrage: floor burning | Primal cache downgraded | clear marksman/brute Arcane/Jailer/Desecrator pressure"
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
    fn new(duration_secs: f32) -> Self {
        let duration_secs = duration_secs.max(0.12);
        Self {
            timer: Timer::from_seconds(duration_secs, TimerMode::Once),
            duration_secs,
            base_scale: Vec3::ONE,
        }
    }

    fn progress(&self) -> f32 {
        (self.timer.elapsed_secs() / self.duration_secs).clamp(0.0, 1.0)
    }
}

fn spawn_enemy_attack_warning(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    spec: EnemyAttackWarningSpec,
) {
    let color = enemy_attack_warning_color(spec.kind);
    let material = materials.add(StandardMaterial {
        base_color: color,
        emissive: color.into(),
        alpha_mode: AlphaMode::Blend,
        ..default()
    });
    let direction = spec.direction.normalize_or_zero();
    let yaw = direction.x.atan2(direction.z);
    let duration_secs = spec.duration_secs.max(0.12);
    let reach = enemy_attack_warning_reach(spec.kind, spec.attack_range);
    let width = enemy_attack_warning_width(spec.kind);
    let forward_offset = enemy_attack_warning_forward_offset(spec.kind, reach);

    match spec.kind {
        EnemyAttackKind::Melee => {
            commands.spawn((
                Mesh3d(meshes.add(Cuboid::new(width, 0.04, reach))),
                MeshMaterial3d(material),
                Transform::from_translation(
                    spec.origin + direction * forward_offset + Vec3::Y * 0.035,
                )
                .with_rotation(Quat::from_rotation_y(yaw)),
                EnemyAttackTelegraph::new(duration_secs),
                EnemyEntity,
                Name::new("Melee Attack Telegraph"),
            ));
        }
        EnemyAttackKind::Projectile => {
            commands.spawn((
                Mesh3d(meshes.add(Cuboid::new(width, 0.04, reach))),
                MeshMaterial3d(material),
                Transform::from_translation(
                    spec.origin + direction * forward_offset + Vec3::Y * 0.045,
                )
                .with_rotation(Quat::from_rotation_y(yaw)),
                EnemyAttackTelegraph::new(duration_secs),
                EnemyEntity,
                Name::new("Projectile Attack Telegraph"),
            ));
        }
        EnemyAttackKind::Shockwave => {
            commands.spawn((
                Mesh3d(meshes.add(Cylinder::new(reach, 0.045))),
                MeshMaterial3d(material),
                Transform::from_translation(spec.origin + Vec3::Y * 0.04),
                EnemyAttackTelegraph::new(duration_secs),
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
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    origin: Vec3,
    direction: Vec3,
    kind: &EnemyAttackKind,
    attack_range: f32,
) {
    let style = enemy_attack_impact_style(kind, attack_range);
    let material = materials.add(StandardMaterial {
        base_color: style.color,
        emissive: style.color.into(),
        alpha_mode: AlphaMode::Blend,
        ..default()
    });
    let mesh = match kind {
        EnemyAttackKind::Melee => meshes.add(Cuboid::new(1.0, 0.04, 1.0)),
        EnemyAttackKind::Projectile => meshes.add(Sphere::new(1.0)),
        EnemyAttackKind::Shockwave => meshes.add(Cylinder::new(1.0, 0.045)),
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

fn spawn_projectile(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    origin: Vec3,
    direction: Vec3,
    damage: f32,
) {
    let color = Color::srgb(1.0, 0.22, 0.04);
    commands.spawn((
        Mesh3d(meshes.add(Sphere::new(0.16))),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: color,
            emissive: color.into(),
            ..default()
        })),
        Transform::from_translation(origin + direction * 0.45),
        EnemyProjectile {
            damage,
            velocity: direction * 6.2,
            radius: 0.55,
            lifetime: Timer::from_seconds(2.0, TimerMode::Once),
            arm_timer: None,
            impact: ProjectileImpact::Burning,
        },
        EnemyEntity,
        Name::new("Cinder Bolt"),
    ));
}

fn spawn_shockwave_marker(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    origin: Vec3,
) {
    let color = Color::srgba(1.0, 0.28, 0.04, 0.55);
    commands.spawn((
        Mesh3d(meshes.add(Cylinder::new(2.6, 0.04))),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: color,
            emissive: color.into(),
            alpha_mode: AlphaMode::Blend,
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
        Mesh3d(meshes.add(Cylinder::new(1.45, 0.06))),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: color,
            emissive: color.into(),
            alpha_mode: AlphaMode::Blend,
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
        Mesh3d(meshes.add(Cylinder::new(1.1, 0.08))),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: color,
            emissive: Color::srgb(0.20, 0.05, 0.42).into(),
            alpha_mode: AlphaMode::Blend,
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
        Mesh3d(meshes.add(Torus::new(0.72, 0.09))),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: color,
            emissive: Color::srgb(0.35, 0.04, 0.02).into(),
            alpha_mode: AlphaMode::Blend,
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
        Mesh3d(meshes.add(Cylinder::new(1.25, 0.08))),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: color,
            emissive: Color::srgb(0.04, 0.18, 0.42).into(),
            alpha_mode: AlphaMode::Blend,
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
        Mesh3d(meshes.add(Cylinder::new(1.55, 0.07))),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: color,
            emissive: Color::srgb(0.42, 0.0, 0.03).into(),
            alpha_mode: AlphaMode::Blend,
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
        Mesh3d(meshes.add(Cylinder::new(1.32, 0.075))),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: color,
            emissive: Color::srgb(0.44, 0.08, 0.58).into(),
            alpha_mode: AlphaMode::Blend,
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
        transform.translation += projectile.velocity * time.delta_secs();
        if projectile_is_armed(&projectile)
            && projectile.damage > 0.0
            && transform
                .translation
                .distance(player_transform.translation + Vec3::Y * 0.8)
                <= projectile.radius
        {
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
            let force = (taken.health_damage * 0.018).clamp(0.0, 0.55);
            PlayerDamageImpactStyle {
                color: Color::srgba(1.0, 0.035, 0.02, 0.66),
                y: 0.86,
                scale: Vec3::splat(0.58 + force),
                lifetime_secs: 0.30,
            }
        }
        PlayerDamageImpactKind::Ward => {
            let force = (taken.absorbed * 0.014).clamp(0.0, 0.34);
            PlayerDamageImpactStyle {
                color: Color::srgba(0.98, 0.72, 0.20, 0.58),
                y: 0.92,
                scale: Vec3::new(0.86 + force, 0.18, 0.86 + force),
                lifetime_secs: 0.25,
            }
        }
        PlayerDamageImpactKind::Evade => PlayerDamageImpactStyle {
            color: Color::srgba(0.28, 0.78, 1.0, 0.52),
            y: 0.58,
            scale: Vec3::new(0.72, 0.10, 0.34),
            lifetime_secs: 0.22,
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
        PlayerDamageImpactKind::Ward => Vec3::ZERO,
        PlayerDamageImpactKind::Health => direction * -0.05,
    };
    Transform::from_translation(target + Vec3::Y * style.y + lateral)
        .with_rotation(Quat::from_rotation_y(direction.x.atan2(direction.z)))
        .with_scale(style.scale)
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
        alpha_mode: AlphaMode::Blend,
        ..default()
    });
    let mesh = match kind {
        PlayerDamageImpactKind::Health => meshes.add(Sphere::new(1.0)),
        PlayerDamageImpactKind::Ward => meshes.add(Torus::new(1.0, 0.06)),
        PlayerDamageImpactKind::Evade => meshes.add(Cuboid::new(1.0, 0.05, 1.0)),
    };
    commands.spawn((
        Mesh3d(mesh),
        MeshMaterial3d(material),
        player_damage_impact_transform(source, target, taken),
        EnemyProjectile {
            damage: 0.0,
            velocity: Vec3::ZERO,
            radius: 0.0,
            lifetime: Timer::from_seconds(style.lifetime_secs, TimerMode::Once),
            arm_timer: None,
            impact: ProjectileImpact::None,
        },
        EnemyEntity,
        Name::new(player_damage_impact_name(kind)),
    ));
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
    fn boss_pressure_summary_reports_phase_enrage_timer_and_enrage() {
        let health = Health {
            current: 54.0,
            max: 120.0,
        };
        let mut phase = BossPhase {
            phase_two_started: false,
            enrage_started: false,
            enrage_timer: Timer::from_seconds(12.0, TimerMode::Once),
        };

        assert_eq!(
            boss_pressure_summary(&health, &phase),
            "Malrec: phase one 45%"
        );

        phase.phase_two_started = true;
        assert_eq!(
            boss_pressure_summary(&health, &phase),
            "Malrec: phase two 45% | break before enrage 12s"
        );

        phase.enrage_started = true;
        assert_eq!(
            boss_pressure_summary(&health, &phase),
            "Malrec: Ashen Enrage 45% | kite burning floor"
        );
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
    fn enemy_motion_visual_pose_gives_walk_cycle_readable_steps() {
        let idle_left = enemy_motion_visual_pose(
            EnemyMotionVisualKind::LeftStep,
            EnemyMotionVisualContext {
                attack_kind: &EnemyAttackKind::Melee,
                moving: false,
                walk_phase: 0.0,
                attack_progress: 1.0,
                warning_progress: 0.0,
                hit_react_power: 0.0,
                base_scale: 1.0,
            },
        );
        let moving_left = enemy_motion_visual_pose(
            EnemyMotionVisualKind::LeftStep,
            EnemyMotionVisualContext {
                attack_kind: &EnemyAttackKind::Melee,
                moving: true,
                walk_phase: 0.0,
                attack_progress: 1.0,
                warning_progress: 0.0,
                hit_react_power: 0.0,
                base_scale: 1.0,
            },
        );
        let moving_right = enemy_motion_visual_pose(
            EnemyMotionVisualKind::RightStep,
            EnemyMotionVisualContext {
                attack_kind: &EnemyAttackKind::Melee,
                moving: true,
                walk_phase: 0.0,
                attack_progress: 1.0,
                warning_progress: 0.0,
                hit_react_power: 0.0,
                base_scale: 1.0,
            },
        );

        assert_eq!(idle_left.visibility, Visibility::Hidden);
        assert_eq!(moving_left.visibility, Visibility::Visible);
        assert_eq!(moving_right.visibility, Visibility::Visible);
        assert!(moving_left.translation.x < 0.0);
        assert!(moving_right.translation.x > 0.0);
        assert!(moving_right.translation.z < moving_left.translation.z - 0.16);
        assert!(moving_right.scale.x > moving_left.scale.x + 0.12);
        assert!(moving_right.scale.z > moving_left.scale.z + 0.10);
        assert_ne!(moving_left.scale, moving_right.scale);
    }

    #[test]
    fn enemy_motion_visual_pose_makes_melee_attacks_read_as_weapon_swings() {
        let idle = enemy_motion_visual_pose(
            EnemyMotionVisualKind::AttackHand,
            EnemyMotionVisualContext {
                attack_kind: &EnemyAttackKind::Melee,
                moving: false,
                walk_phase: 0.0,
                attack_progress: 1.0,
                warning_progress: 0.0,
                hit_react_power: 0.0,
                base_scale: 1.0,
            },
        );
        let windup = enemy_motion_visual_pose(
            EnemyMotionVisualKind::AttackHand,
            EnemyMotionVisualContext {
                attack_kind: &EnemyAttackKind::Melee,
                moving: false,
                walk_phase: 0.0,
                attack_progress: 0.28,
                warning_progress: 0.6,
                hit_react_power: 0.0,
                base_scale: 1.0,
            },
        );
        let impact = enemy_motion_visual_pose(
            EnemyMotionVisualKind::AttackHand,
            EnemyMotionVisualContext {
                attack_kind: &EnemyAttackKind::Melee,
                moving: false,
                walk_phase: 0.0,
                attack_progress: 0.56,
                warning_progress: 0.0,
                hit_react_power: 0.0,
                base_scale: 1.0,
            },
        );

        assert!(windup.translation.y > idle.translation.y + 0.08);
        assert!(windup.translation.z < idle.translation.z);
        assert!(impact.translation.z > idle.translation.z + 0.55);
        assert!(impact.scale.x > idle.scale.x + 0.18);
        assert_ne!(impact.rotation, idle.rotation);
    }

    #[test]
    fn enemy_motion_visual_pose_separates_caster_and_shockwave_silhouettes() {
        let projectile = enemy_motion_visual_pose(
            EnemyMotionVisualKind::AttackHand,
            EnemyMotionVisualContext {
                attack_kind: &EnemyAttackKind::Projectile,
                moving: false,
                walk_phase: 0.0,
                attack_progress: 0.42,
                warning_progress: 0.85,
                hit_react_power: 0.0,
                base_scale: 1.0,
            },
        );
        let shockwave = enemy_motion_visual_pose(
            EnemyMotionVisualKind::AttackHand,
            EnemyMotionVisualContext {
                attack_kind: &EnemyAttackKind::Shockwave,
                moving: false,
                walk_phase: 0.0,
                attack_progress: 0.42,
                warning_progress: 0.85,
                hit_react_power: 0.0,
                base_scale: 1.0,
            },
        );
        let caster_offhand = enemy_motion_visual_pose(
            EnemyMotionVisualKind::OffHand,
            EnemyMotionVisualContext {
                attack_kind: &EnemyAttackKind::Projectile,
                moving: false,
                walk_phase: 0.0,
                attack_progress: 0.42,
                warning_progress: 0.85,
                hit_react_power: 0.0,
                base_scale: 1.0,
            },
        );

        assert!(projectile.translation.y > shockwave.translation.y + 0.65);
        assert!(projectile.scale.x > shockwave.scale.x * 0.70);
        assert!(shockwave.translation.z >= projectile.translation.z);
        assert!(caster_offhand.translation.y > 1.05);
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
        let position = nemesis_spawn_position(checkpoint);
        let affixes = nemesis_affixes();

        assert!(position.x <= 10.8);
        assert!(position.z >= -6.8);
        assert!(affixes.contains(&EnemyAffix::Frenzied));
        assert!(affixes.contains(&EnemyAffix::Vampiric));
        assert!(affixes.contains(&EnemyAffix::Jailer));
        assert!(affixes.contains(&EnemyAffix::Frozen));
        assert!(affixes.contains(&EnemyAffix::Desecrator));
        assert!(affixes.contains(&EnemyAffix::Reflective));
    }
}
