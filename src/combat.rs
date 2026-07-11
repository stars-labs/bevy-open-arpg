use crate::{
    GameState, RunStats,
    assets::GameAssets,
    bestiary::{Bestiary, bestiary_damage_multiplier},
    data::PlayerTuning,
    dungeon::Breakable,
    enemy::{
        BossStagger, Chilled, Enemy, EnemyAffix, EnemyHitReact, EnemyTargetFocus,
        SEAL_WARDEN_WARD_VULNERABLE_SECS, SealWardenWard, Staggered, add_boss_stagger,
        boss_stagger_damage_multiplier, boss_staggered, enemy_cursor_pick_radius,
    },
    feedback::{CombatEvent, FloatingCombatTextEvent, HitStopEvent, ScreenShakeEvent},
    not_paused,
    player::{
        Barrier, Burning, Charm, ConduitBuff, DamageBonus, DashRune, ElixirBuff, Equipment, Evade,
        Fury, GloryBuff, Health, LegendaryCodex, LegendaryPower, NovaRune, Player, PlayerAnimation,
        PlayerAttackMotion, RelicBuff, ReliquarySet, RuptureRune, SkillCooldowns, SkillRunes,
        SurgeBuff, Talents, apply_player_damage_with_evade, damage_multiplier,
        elixir_damage_multiplier, glory_damage_multiplier, mitigated_damage,
        surge_damage_multiplier, surge_fury_multiplier, themed_reliquary_set, total_armor,
        total_crit_chance, total_damage_bonus,
    },
};
use bevy::ecs::system::SystemParam;
use bevy::prelude::*;
use bevy::window::PrimaryWindow;
use rand::Rng;

#[derive(Component)]
struct HitFlash {
    timer: Timer,
}

#[derive(Component)]
struct CombatVfx {
    timer: Timer,
}

fn hit_react_for_damage(critical: bool) -> EnemyHitReact {
    EnemyHitReact::new(if critical { 0.2 } else { 0.14 })
}

fn hit_react_for_contact(
    critical: bool,
    source: Vec3,
    target: Vec3,
    impact_scale: f32,
) -> EnemyHitReact {
    let base_distance = if critical { 0.52 } else { 0.34 };
    hit_react_for_damage(critical).with_recoil_from(
        source,
        target,
        base_distance * impact_scale.clamp(0.55, 1.85),
    )
}

#[derive(Component)]
struct Vulnerable {
    timer: Timer,
    damage_multiplier: f32,
}

const COMBO_WINDOW_SECS: f32 = 3.0;
const SKILL_INPUT_BUFFER_SECS: f32 = 0.18;

#[derive(Resource, Debug, Default)]
struct SkillInputBuffer {
    dash_secs: f32,
    rupture_secs: f32,
    nova_secs: f32,
}

impl SkillInputBuffer {
    fn tick(&mut self, delta_secs: f32) {
        self.dash_secs = (self.dash_secs - delta_secs).max(0.0);
        self.rupture_secs = (self.rupture_secs - delta_secs).max(0.0);
        self.nova_secs = (self.nova_secs - delta_secs).max(0.0);
    }

    fn capture(&mut self, dash: bool, rupture: bool, nova: bool) {
        if dash {
            self.dash_secs = SKILL_INPUT_BUFFER_SECS;
        }
        if rupture {
            self.rupture_secs = SKILL_INPUT_BUFFER_SECS;
        }
        if nova {
            self.nova_secs = SKILL_INPUT_BUFFER_SECS;
        }
    }

    fn dash_requested(&self) -> bool {
        self.dash_secs > 0.0
    }

    fn rupture_requested(&self) -> bool {
        self.rupture_secs > 0.0
    }

    fn nova_requested(&self) -> bool {
        self.nova_secs > 0.0
    }

    fn clear_dash(&mut self) {
        self.dash_secs = 0.0;
    }

    fn clear_rupture(&mut self) {
        self.rupture_secs = 0.0;
    }

    fn clear_nova(&mut self) {
        self.nova_secs = 0.0;
    }
}

#[derive(Resource, Debug)]
pub struct CombatComboState {
    primer: Option<ComboPrimer>,
    timer: Timer,
}

impl Default for CombatComboState {
    fn default() -> Self {
        Self {
            primer: None,
            timer: Timer::from_seconds(0.0, TimerMode::Once),
        }
    }
}

impl CombatComboState {
    fn prime(&mut self, primer: ComboPrimer) {
        self.primer = Some(primer);
        self.timer = Timer::from_seconds(COMBO_WINDOW_SECS, TimerMode::Once);
        self.timer.reset();
    }

    fn clear(&mut self) {
        self.primer = None;
        self.timer = Timer::from_seconds(0.0, TimerMode::Once);
    }

    fn active_primer(&self) -> Option<ComboPrimer> {
        if self.timer.is_finished() {
            None
        } else {
            self.primer
        }
    }

    pub fn hud_hint(&self) -> Option<String> {
        self.active_primer().map(|primer| {
            format!(
                "{} {:.0}s -> {}",
                combo_primer_label(primer),
                self.timer.remaining_secs().ceil().max(1.0),
                combo_hud_finisher_label(primer)
            )
        })
    }
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
enum ComboPrimer {
    Rupture,
    Nova,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
enum ComboFinisher {
    Dash,
    Nova,
}

#[derive(Debug, Default, Clone, Copy, PartialEq)]
struct ComboSetBonus {
    fury: f32,
    healing: f32,
    barrier: f32,
    shake: f32,
    label: Option<&'static str>,
}

#[derive(Debug, Clone, Copy, Default)]
struct ComboChainBonus {
    fury: f32,
    healing: f32,
    barrier: f32,
    shake: f32,
    label: Option<&'static str>,
}

struct ArcAttack {
    origin: Vec3,
    facing: Vec3,
    range: f32,
    damage: f32,
    impact_scale: f32,
    crit_chance: f32,
    legendary_powers: LegendaryPowerSet,
    skill: Option<SkillCast>,
}

#[derive(Component, Debug)]
struct PendingPlayerArcAttack {
    timer: Timer,
    facing: Vec3,
    range: f32,
    damage: f32,
    impact_scale: f32,
    crit_chance: f32,
    legendary_powers: LegendaryPowerSet,
    skill: Option<SkillCast>,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
struct LegendaryPowerSet {
    weapon: LegendaryPower,
    attuned: LegendaryPower,
}

impl LegendaryPowerSet {
    fn new(weapon: LegendaryPower, attuned: LegendaryPower) -> Self {
        Self { weapon, attuned }
    }

    fn none() -> Self {
        Self::new(LegendaryPower::None, LegendaryPower::None)
    }

    fn contains(self, power: LegendaryPower) -> bool {
        power != LegendaryPower::None && (self.weapon == power || self.attuned == power)
    }
}

struct StormChain {
    source: Entity,
    origin: Vec3,
    damage: f32,
}

struct BossStaggerTrigger {
    entity: Entity,
    target: Vec3,
    damage: f32,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
enum SkillCast {
    Dash(DashRune),
    Rupture(RuptureRune),
    Nova(NovaRune),
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
struct LegendaryProcTrigger {
    critical: bool,
    skill: Option<SkillCast>,
}

impl LegendaryProcTrigger {
    fn basic(critical: bool) -> Self {
        Self {
            critical,
            skill: None,
        }
    }

    fn skill(critical: bool, skill: SkillCast) -> Self {
        Self {
            critical,
            skill: Some(skill),
        }
    }

    fn from_attack(critical: bool, attack: &ArcAttack) -> Self {
        if let Some(skill) = attack.skill {
            Self::skill(critical, skill)
        } else {
            Self::basic(critical)
        }
    }

    fn has_skill_synergy(self, power: LegendaryPower) -> bool {
        legendary_power_skill_synergy(power, self.skill)
    }

    fn should_proc(self, power: LegendaryPower) -> bool {
        self.critical || self.has_skill_synergy(power)
    }
}

#[derive(Debug, Clone, Copy)]
struct HitContact {
    target: Vec3,
    hit_bone: HitBone,
    effect: HitEffect,
    critical: bool,
    scale: f32,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
enum HitImpactTier {
    Normal,
    Elite,
    Boss,
    BossBreak,
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct HitImpactProfile {
    critical: bool,
    impact_scale: f32,
    tier: HitImpactTier,
}

struct CombatVfxAssets {
    slash_arc: Handle<WorldAsset>,
    hit_spark: Handle<WorldAsset>,
    bone_shatter: Handle<WorldAsset>,
    bone_impact: Handle<WorldAsset>,
    blood_spray: Handle<WorldAsset>,
    execution_burst: Handle<WorldAsset>,
    arcane_impact: Handle<WorldAsset>,
    holy_impact: Handle<WorldAsset>,
    ember_impact: Handle<WorldAsset>,
    frost_impact: Handle<WorldAsset>,
    void_impact: Handle<WorldAsset>,
    frenzy_impact: Handle<WorldAsset>,
    vampiric_siphon: Handle<WorldAsset>,
    desecrator_burst: Handle<WorldAsset>,
    guard_clash: Handle<WorldAsset>,
    armor_break: Handle<WorldAsset>,
    soul_ward_hit: Handle<WorldAsset>,
    hit_bone_rune: Handle<WorldAsset>,
    hit_bone_lock: Handle<WorldAsset>,
    marrow_flash: Handle<WorldAsset>,
    bone_fracture_echo: Handle<WorldAsset>,
    elite_affix_break: Handle<WorldAsset>,
    shadow_burst: Handle<WorldAsset>,
    headshot_burst: Handle<WorldAsset>,
    crit_bone_crown: Handle<WorldAsset>,
    crit_burst: Handle<WorldAsset>,
    stagger_burst: Handle<WorldAsset>,
    shadow_trail: Handle<WorldAsset>,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
enum HitBone {
    Head,
    Chest,
    Weapon,
    Left,
    Right,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
enum HitEffect {
    Spark,
    Bone,
    BoneImpact,
    Blood,
    Execution,
    Arcane,
    Holy,
    Ember,
    Frost,
    Void,
    Frenzy,
    Vampiric,
    Desecrator,
    GuardClash,
    ArmorBreak,
    SoulWard,
    Shadow,
    Headshot,
    Critical,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
enum HitOverlay {
    Rune,
    Marrow,
    Critical,
}

#[derive(Debug, Default, Clone, Copy, PartialEq)]
struct AttackOutcome {
    hits: u32,
    boss_staggers: u32,
    life_stolen: f32,
    barrier_gained: f32,
    reflected_damage: f32,
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct PrimaryAttackTarget {
    entity: Entity,
    position: Vec3,
    in_range: bool,
}

#[derive(Resource, Debug, Default, Clone, Copy, PartialEq, Eq)]
pub(crate) struct BasicAttackIntent {
    pub(crate) target: Option<Entity>,
}

struct AttackFeedback<'a, 'w> {
    combat_events: &'a mut MessageWriter<'w, CombatEvent>,
    floating_text: &'a mut MessageWriter<'w, FloatingCombatTextEvent>,
    screen_shake: &'a mut MessageWriter<'w, ScreenShakeEvent>,
    hit_stop: &'a mut MessageWriter<'w, HitStopEvent>,
}

struct ComboFinisherPlayer<'a> {
    fury: &'a mut Fury,
    health: &'a mut Health,
    barrier: &'a mut Barrier,
    equipment: &'a Equipment,
    charm: &'a Charm,
}

struct ComboFinisherFeedback<'a, 'w> {
    combat_events: &'a mut MessageWriter<'w, CombatEvent>,
    screen_shake: &'a mut MessageWriter<'w, ScreenShakeEvent>,
}

type EnemyTargets<'w, 's> = Query<
    'w,
    's,
    (
        Entity,
        &'static Transform,
        &'static Enemy,
        &'static mut Health,
        Option<&'static mut SealWardenWard>,
        Option<&'static Chilled>,
        Option<&'static Vulnerable>,
        Option<&'static mut BossStagger>,
        Option<&'static Staggered>,
    ),
    (With<Enemy>, Without<Player>),
>;
type BreakableTargets<'w, 's> = Query<
    'w,
    's,
    (Entity, &'static Transform, &'static mut Health),
    (With<Breakable>, Without<Player>, Without<Enemy>),
>;
type CombatPlayerCoreQuery<'w, 's> = Query<
    'w,
    's,
    (
        Entity,
        &'static mut Transform,
        &'static mut Health,
        &'static mut Barrier,
        &'static mut Fury,
        &'static mut SkillCooldowns,
        &'static mut PlayerAnimation,
    ),
    With<Player>,
>;
type CombatPlayerStatsQuery<'w, 's> = Query<
    'w,
    's,
    (
        &'static DamageBonus,
        &'static Charm,
        &'static Equipment,
        &'static RelicBuff,
        &'static GloryBuff,
        &'static ElixirBuff,
        &'static SurgeBuff,
        &'static Evade,
        &'static SkillRunes,
        &'static Talents,
        &'static LegendaryCodex,
    ),
    With<Player>,
>;
type PendingPlayerArcAttackQuery<'w, 's> = Query<
    'w,
    's,
    (
        Entity,
        &'static Transform,
        &'static mut Health,
        &'static mut Barrier,
        &'static mut Fury,
        &'static Equipment,
        &'static ElixirBuff,
        &'static Evade,
        &'static SurgeBuff,
        &'static mut PendingPlayerArcAttack,
    ),
    With<Player>,
>;

#[derive(SystemParam)]
struct CombatInputs<'w, 's> {
    time: Res<'w, Time>,
    keyboard: Res<'w, ButtonInput<KeyCode>>,
    mouse: Res<'w, ButtonInput<MouseButton>>,
    windows: Query<'w, 's, &'static Window, With<PrimaryWindow>>,
    cameras: Query<'w, 's, (&'static Camera, &'static GlobalTransform), With<Camera3d>>,
    tuning: Res<'w, PlayerTuning>,
    bestiary: Res<'w, Bestiary>,
    assets: Res<'w, GameAssets>,
    stats: ResMut<'w, RunStats>,
    combo: ResMut<'w, CombatComboState>,
    input_buffer: ResMut<'w, SkillInputBuffer>,
    basic_attack_intent: ResMut<'w, BasicAttackIntent>,
    commands: Commands<'w, 's>,
    combat_events: MessageWriter<'w, CombatEvent>,
    floating_text: MessageWriter<'w, FloatingCombatTextEvent>,
    screen_shake: MessageWriter<'w, ScreenShakeEvent>,
    hit_stop: MessageWriter<'w, HitStopEvent>,
}

#[derive(SystemParam)]
struct PendingAttackInputs<'w, 's> {
    time: Res<'w, Time>,
    assets: Res<'w, GameAssets>,
    bestiary: Res<'w, Bestiary>,
    stats: ResMut<'w, RunStats>,
    commands: Commands<'w, 's>,
    combat_events: MessageWriter<'w, CombatEvent>,
    floating_text: MessageWriter<'w, FloatingCombatTextEvent>,
    screen_shake: MessageWriter<'w, ScreenShakeEvent>,
    hit_stop: MessageWriter<'w, HitStopEvent>,
}

pub struct CombatPlugin;

impl Plugin for CombatPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<CombatComboState>()
            .init_resource::<SkillInputBuffer>()
            .init_resource::<BasicAttackIntent>()
            .add_systems(
                OnEnter(GameState::InGame),
                (reset_combat_combo, reset_basic_attack_intent),
            )
            .add_systems(
                Update,
                (
                    (player_attacks, resolve_pending_player_arc_attacks).chain(),
                    tick_conduit_lightning,
                    tick_enemy_burning,
                    tick_hit_flash,
                    tick_combat_vfx,
                    tick_vulnerable,
                    tick_combat_combo,
                )
                    .run_if(in_state(GameState::InGame).and_then(not_paused)),
            )
            .add_systems(OnExit(GameState::InGame), despawn_combat_vfx);
    }
}

fn reset_combat_combo(mut combo: ResMut<CombatComboState>) {
    *combo = CombatComboState::default();
}

fn reset_basic_attack_intent(mut intent: ResMut<BasicAttackIntent>) {
    *intent = BasicAttackIntent::default();
}

fn tick_combat_combo(time: Res<Time>, mut combo: ResMut<CombatComboState>) {
    combo.timer.tick(time.delta());
    if combo.timer.is_finished() {
        combo.primer = None;
    }
}

fn player_attacks(
    mut inputs: CombatInputs,
    mut player_core: CombatPlayerCoreQuery,
    player_stats: CombatPlayerStatsQuery,
    mut enemies: EnemyTargets,
    mut breakables: BreakableTargets,
) {
    let Ok((
        player_entity,
        mut player_transform,
        mut player_health,
        mut player_barrier,
        mut fury,
        mut cooldowns,
        mut animation,
    )) = player_core.single_mut()
    else {
        return;
    };
    let Ok((bonus, charm, equipment, buff, glory, elixir, surge, evade, runes, talents, codex)) =
        player_stats.single()
    else {
        return;
    };
    let total_multiplier = damage_multiplier(buff)
        * elixir_damage_multiplier(elixir)
        * glory_damage_multiplier(glory)
        * surge_damage_multiplier(surge)
        * talents.damage_multiplier();
    let damage_bonus = total_damage_bonus(bonus, equipment, charm);
    let crit_chance = total_crit_chance(equipment, charm);
    let legendary_powers = LegendaryPowerSet::new(equipment.legendary_power, codex.attuned);
    let vfx = combat_vfx_assets(&inputs.assets);
    let cursor_ground = cursor_ground_point(&inputs.windows, &inputs.cameras);
    let cursor_primary_attack_target = cursor_ground_primary_attack_target(
        cursor_ground,
        player_transform.translation,
        inputs.tuning.basic_range,
        &enemies,
    );
    update_basic_attack_intent(
        &mut inputs.basic_attack_intent,
        &inputs.mouse,
        cursor_ground,
        cursor_primary_attack_target,
    );
    let primary_attack_target = intended_primary_attack_target(
        &mut inputs.basic_attack_intent,
        cursor_primary_attack_target,
        player_transform.translation,
        inputs.tuning.basic_range,
        &enemies,
    );
    let dash_pressed = inputs.keyboard.just_pressed(KeyCode::KeyQ);
    let rupture_pressed = inputs.mouse.just_pressed(MouseButton::Right);
    let nova_pressed = inputs.keyboard.just_pressed(KeyCode::KeyE);
    inputs.input_buffer.tick(inputs.time.delta_secs());
    inputs
        .input_buffer
        .capture(dash_pressed, rupture_pressed, nova_pressed);

    if basic_attack_requested(
        &inputs.mouse,
        &cooldowns,
        &inputs.basic_attack_intent,
        primary_attack_target,
    ) {
        let facing = attack_facing_from_cursor(
            player_transform.translation,
            player_transform.forward().as_vec3(),
            primary_attack_target
                .map(|target| target.position)
                .or(cursor_ground),
        );
        animation.face_direction(facing);
        animation.trigger_attack(PlayerAttackMotion::Basic, 0.42);
        spawn_slash_vfx(
            &mut inputs.commands,
            &vfx,
            player_transform.translation,
            facing,
            0.78,
        );
        inputs
            .commands
            .entity(player_entity)
            .try_insert(queued_basic_arc_attack(
                facing,
                inputs.tuning.basic_range,
                (inputs.tuning.basic_damage + damage_bonus) * total_multiplier,
                crit_chance,
                legendary_powers,
            ));
        let basic_base = cooldowns.basic_base;
        reset_scaled(
            &mut cooldowns.basic,
            basic_base,
            talents.cooldown_multiplier(),
        );
    }

    if inputs.input_buffer.dash_requested() && !cooldowns.dash.is_finished() && dash_pressed {
        inputs.combat_events.write(CombatEvent {
            text: skill_unavailable_text("Dash", cooldowns.dash.remaining_secs()),
        });
    }

    if inputs.input_buffer.dash_requested() && cooldowns.dash.is_finished() {
        let dash_cost = fury.dash_cost;
        if !fury.spend(dash_cost) {
            inputs.input_buffer.clear_dash();
            if dash_pressed {
                inputs.combat_events.write(CombatEvent {
                    text: format!("Need {dash_cost:.0} fury for dash"),
                });
            }
            return;
        }
        inputs.input_buffer.clear_dash();
        inputs.combat_events.write(CombatEvent {
            text: skill_cast_text(SkillCast::Dash(runes.dash)),
        });
        inputs
            .screen_shake
            .write(skill_cast_screen_shake(SkillCast::Dash(runes.dash)));
        animation.trigger_attack(PlayerAttackMotion::Dash, 0.38);
        let direction = player_transform.forward().as_vec3().normalize_or_zero();
        let dash_start = player_transform.translation;
        let dash_distance = match runes.dash {
            DashRune::Cleanse => inputs.tuning.dash_distance,
            DashRune::Reap => inputs.tuning.dash_distance * 0.82,
        };
        player_transform.translation += direction * dash_distance;
        player_transform.translation.x = player_transform.translation.x.clamp(-11.5, 11.5);
        player_transform.translation.z = player_transform.translation.z.clamp(-7.5, 7.5);
        spawn_slash_vfx(
            &mut inputs.commands,
            &vfx,
            player_transform.translation,
            direction,
            1.05,
        );
        spawn_shadow_trail_vfx(
            &mut inputs.commands,
            &vfx,
            player_transform.translation,
            direction,
            1.2,
        );
        spawn_dash_cast_vfx(
            &mut inputs.commands,
            &vfx,
            dash_start,
            player_transform.translation,
            direction,
            runes.dash,
        );
        let outcome = apply_arc_damage(
            &mut inputs.commands,
            &mut AttackFeedback {
                combat_events: &mut inputs.combat_events,
                floating_text: &mut inputs.floating_text,
                screen_shake: &mut inputs.screen_shake,
                hit_stop: &mut inputs.hit_stop,
            },
            &vfx,
            ArcAttack {
                origin: player_transform.translation,
                facing: direction,
                range: inputs.tuning.basic_range + 1.2,
                damage: (inputs.tuning.dash_damage + damage_bonus)
                    * total_multiplier
                    * dash_damage_multiplier(runes.dash),
                impact_scale: 1.12,
                crit_chance,
                legendary_powers,
                skill: Some(SkillCast::Dash(runes.dash)),
            },
            &mut enemies,
            &inputs.bestiary,
        );
        apply_arc_breakable_damage(
            &mut inputs.commands,
            ArcAttack {
                origin: player_transform.translation,
                facing: direction,
                range: inputs.tuning.basic_range + 1.2,
                damage: (inputs.tuning.dash_damage + damage_bonus)
                    * total_multiplier
                    * dash_damage_multiplier(runes.dash),
                impact_scale: 0.95,
                crit_chance: 0.0,
                legendary_powers: LegendaryPowerSet::none(),
                skill: None,
            },
            &mut breakables,
        );
        if runes.dash == DashRune::Reap && outcome.hits > 0 {
            inputs.stats.reap_dash_hits = inputs.stats.reap_dash_hits.saturating_add(outcome.hits);
            let restored = 8.0 * outcome.hits as f32;
            player_health.current = (player_health.current + restored).min(player_health.max);
            inputs.combat_events.write(CombatEvent {
                text: format!("Reap restored {restored:.0} health"),
            });
        }
        apply_combo_finisher(
            &mut inputs.combo,
            ComboFinisherPlayer {
                fury: &mut fury,
                health: &mut player_health,
                barrier: &mut player_barrier,
                equipment,
                charm,
            },
            ComboFinisher::Dash,
            outcome.hits,
            ComboFinisherFeedback {
                combat_events: &mut inputs.combat_events,
                screen_shake: &mut inputs.screen_shake,
            },
        );
        apply_boss_break_reward(
            &mut fury,
            &mut player_barrier,
            outcome.boss_staggers,
            &mut inputs.stats,
            &mut inputs.combat_events,
        );
        apply_life_steal(
            &mut player_health,
            outcome.life_stolen,
            &mut inputs.combat_events,
        );
        apply_legendary_barrier(
            &mut player_barrier,
            outcome.barrier_gained,
            &mut inputs.combat_events,
        );
        apply_reflected_damage(
            &mut player_health,
            &mut player_barrier,
            outcome.reflected_damage,
            total_armor(equipment, elixir),
            evade,
            &mut inputs.combat_events,
        );
        let dash_base = cooldowns.dash_base;
        reset_scaled(
            &mut cooldowns.dash,
            dash_base,
            talents.cooldown_multiplier(),
        );
        if runes.dash == DashRune::Cleanse {
            inputs
                .commands
                .entity(player_entity)
                .try_remove::<Burning>();
        }
    }

    if inputs.input_buffer.rupture_requested()
        && !cooldowns.rupture.is_finished()
        && rupture_pressed
    {
        inputs.combat_events.write(CombatEvent {
            text: skill_unavailable_text("Rupture", cooldowns.rupture.remaining_secs()),
        });
    }

    if inputs.input_buffer.rupture_requested() && cooldowns.rupture.is_finished() {
        let facing = attack_facing_from_cursor(
            player_transform.translation,
            player_transform.forward().as_vec3(),
            cursor_ground,
        );
        animation.face_direction(facing);
        let rupture_cost = fury.rupture_cost;
        if !fury.spend(rupture_cost) {
            inputs.input_buffer.clear_rupture();
            if rupture_pressed {
                inputs.combat_events.write(CombatEvent {
                    text: format!("Need {rupture_cost:.0} fury for rupture"),
                });
            }
            return;
        }
        inputs.input_buffer.clear_rupture();
        inputs.combat_events.write(CombatEvent {
            text: skill_cast_text(SkillCast::Rupture(runes.rupture)),
        });
        inputs
            .screen_shake
            .write(skill_cast_screen_shake(SkillCast::Rupture(runes.rupture)));
        animation.trigger_attack(PlayerAttackMotion::Rupture, 0.58);
        let rupture_range = inputs.tuning.basic_range + 0.9;
        let rupture_damage = (inputs.tuning.dash_damage * 0.82 + damage_bonus * 0.75)
            * total_multiplier
            * rupture_damage_multiplier(runes.rupture);
        spawn_slash_vfx(
            &mut inputs.commands,
            &vfx,
            player_transform.translation,
            facing,
            1.25,
        );
        spawn_rupture_cast_vfx(
            &mut inputs.commands,
            &vfx,
            player_transform.translation,
            facing,
            rupture_range,
            runes.rupture,
        );
        let outcome = apply_arc_damage(
            &mut inputs.commands,
            &mut AttackFeedback {
                combat_events: &mut inputs.combat_events,
                floating_text: &mut inputs.floating_text,
                screen_shake: &mut inputs.screen_shake,
                hit_stop: &mut inputs.hit_stop,
            },
            &vfx,
            ArcAttack {
                origin: player_transform.translation,
                facing,
                range: rupture_range,
                damage: rupture_damage,
                impact_scale: 1.28,
                crit_chance,
                legendary_powers,
                skill: Some(SkillCast::Rupture(runes.rupture)),
            },
            &mut enemies,
            &inputs.bestiary,
        );
        let affected = match runes.rupture {
            RuptureRune::Expose => apply_rupture_vulnerable(
                &mut inputs.commands,
                player_transform.translation,
                facing,
                rupture_range,
                &mut enemies,
            ),
            RuptureRune::Hemorrhage => apply_rupture_bleed(
                &mut inputs.commands,
                player_transform.translation,
                facing,
                rupture_range,
                rupture_damage,
                &mut enemies,
                &inputs.bestiary,
            ),
        };
        if runes.rupture == RuptureRune::Hemorrhage && affected > 0 {
            inputs.stats.hemorrhage_rupture_hits = inputs
                .stats
                .hemorrhage_rupture_hits
                .saturating_add(affected);
        }
        if outcome.hits > 0 || affected > 0 {
            inputs.combo.prime(ComboPrimer::Rupture);
            inputs.combat_events.write(CombatEvent {
                text: "Combo Ready: finish Rupture with Dash or Nova".to_string(),
            });
        }
        apply_boss_break_reward(
            &mut fury,
            &mut player_barrier,
            outcome.boss_staggers,
            &mut inputs.stats,
            &mut inputs.combat_events,
        );
        apply_arc_breakable_damage(
            &mut inputs.commands,
            ArcAttack {
                origin: player_transform.translation,
                facing,
                range: rupture_range,
                damage: rupture_damage,
                impact_scale: 1.05,
                crit_chance: 0.0,
                legendary_powers: LegendaryPowerSet::none(),
                skill: None,
            },
            &mut breakables,
        );
        apply_life_steal(
            &mut player_health,
            outcome.life_stolen,
            &mut inputs.combat_events,
        );
        apply_legendary_barrier(
            &mut player_barrier,
            outcome.barrier_gained,
            &mut inputs.combat_events,
        );
        apply_reflected_damage(
            &mut player_health,
            &mut player_barrier,
            outcome.reflected_damage,
            total_armor(equipment, elixir),
            evade,
            &mut inputs.combat_events,
        );
        if affected > 0 {
            inputs.combat_events.write(CombatEvent {
                text: rupture_effect_text(runes.rupture, affected),
            });
        }
        let rupture_base = cooldowns.rupture_base;
        reset_scaled(
            &mut cooldowns.rupture,
            rupture_base,
            talents.cooldown_multiplier(),
        );
    }

    if inputs.input_buffer.nova_requested() && !cooldowns.nova.is_finished() && nova_pressed {
        inputs.combat_events.write(CombatEvent {
            text: skill_unavailable_text("Nova", cooldowns.nova.remaining_secs()),
        });
    }

    if inputs.input_buffer.nova_requested() && cooldowns.nova.is_finished() {
        let nova_cost = fury.nova_cost;
        if !fury.spend(nova_cost) {
            inputs.input_buffer.clear_nova();
            if nova_pressed {
                inputs.combat_events.write(CombatEvent {
                    text: format!("Need {nova_cost:.0} fury for nova"),
                });
            }
            return;
        }
        inputs.input_buffer.clear_nova();
        inputs.combat_events.write(CombatEvent {
            text: skill_cast_text(SkillCast::Nova(runes.nova)),
        });
        inputs
            .screen_shake
            .write(skill_cast_screen_shake(SkillCast::Nova(runes.nova)));
        animation.trigger_attack(PlayerAttackMotion::Nova, 0.56);
        spawn_nova_cast_vfx(
            &mut inputs.commands,
            &vfx,
            player_transform.translation,
            runes.nova,
            inputs.tuning.nova_radius,
        );
        let mut storm_chains = Vec::new();
        let mut life_stolen = 0.0;
        let mut barrier_gained = 0.0;
        let mut reflected_damage = 0.0;
        let mut boss_staggers = 0;
        let mut nova_hits = 0;
        for (
            entity,
            enemy_transform,
            enemy,
            mut health,
            ward,
            chilled,
            vulnerable,
            stagger,
            staggered,
        ) in &mut enemies
        {
            let distance = player_transform
                .translation
                .distance(enemy_transform.translation);
            if distance <= inputs.tuning.nova_radius {
                nova_hits += 1;
                let damage = (inputs.tuning.nova_damage + damage_bonus)
                    * total_multiplier
                    * nova_damage_multiplier(runes.nova);
                let critical = roll_critical(crit_chance);
                let damage = bestiary_adjusted_damage(damage, enemy, &inputs.bestiary);
                let final_damage = apply_enemy_affix_damage(
                    apply_critical_damage(damage, critical),
                    enemy,
                    chilled,
                    vulnerable,
                    staggered,
                );
                let ward_result = apply_seal_warden_ward(
                    &mut inputs.commands,
                    &mut inputs.combat_events,
                    &vfx,
                    WardHitContext {
                        entity,
                        target: enemy_transform.translation,
                        enemy,
                        incoming_damage: final_damage,
                        ward,
                    },
                );
                let final_damage = ward_result.health_damage;
                health.current -= final_damage;
                let boss_break = trigger_boss_stagger(
                    &mut inputs.commands,
                    &mut inputs.combat_events,
                    &vfx,
                    enemy,
                    BossStaggerTrigger {
                        entity,
                        target: enemy_transform.translation,
                        damage: final_damage,
                    },
                    stagger,
                    staggered,
                );
                if boss_break {
                    boss_staggers += 1;
                    inputs.hit_stop.write(boss_break_hit_stop_event());
                }
                let impact_profile = hit_impact_profile(enemy, critical, 1.35, boss_break);
                spawn_hit_vfx(
                    &mut inputs.commands,
                    &vfx,
                    enemy_transform.translation,
                    enemy,
                    critical,
                    hit_impact_vfx_scale(impact_profile),
                );
                if final_damage > 0.0 {
                    inputs.combat_events.write(CombatEvent {
                        text: damage_text(
                            &format!("{} nova", runes.nova.label()),
                            final_damage,
                            critical,
                        ),
                    });
                }
                inputs.floating_text.write(FloatingCombatTextEvent {
                    text: floating_damage_text_for_ward_result(
                        final_damage,
                        ward_result,
                        impact_profile,
                    ),
                    position: hit_bone_world_position(enemy_transform.translation, enemy, critical),
                    critical,
                });
                trigger_hit_impact_screen_shake(&mut inputs.screen_shake, impact_profile);
                trigger_hit_impact_hit_stop(&mut inputs.hit_stop, impact_profile);
                if runes.nova == NovaRune::Frost {
                    inputs.commands.entity(entity).try_insert(Chilled {
                        timer: Timer::from_seconds(2.8, TimerMode::Once),
                        slow_multiplier: 0.48,
                    });
                }
                let proc_trigger =
                    LegendaryProcTrigger::skill(critical, SkillCast::Nova(runes.nova));
                if let Some(burning) =
                    legendary_burning(legendary_powers, proc_trigger, final_damage)
                {
                    inputs.commands.entity(entity).try_insert(burning);
                    inputs.combat_events.write(CombatEvent {
                        text: "Emberbrand ignited enemy".to_string(),
                    });
                }
                if let Some(chilled) = legendary_chilled(legendary_powers, proc_trigger) {
                    inputs.commands.entity(entity).try_insert(chilled);
                    inputs.combat_events.write(CombatEvent {
                        text: "Frostbrand chilled enemy".to_string(),
                    });
                }
                if let Some(damage) =
                    stormbrand_chain_damage(legendary_powers, proc_trigger, final_damage)
                {
                    storm_chains.push(StormChain {
                        source: entity,
                        origin: enemy_transform.translation,
                        damage,
                    });
                }
                life_stolen += legendary_life_steal(legendary_powers, proc_trigger, final_damage);
                barrier_gained += legendary_barrier(legendary_powers, proc_trigger, final_damage);
                reflected_damage += reflective_damage(enemy, final_damage);
                inputs.commands.entity(entity).try_insert((
                    HitFlash {
                        timer: Timer::from_seconds(0.18, TimerMode::Once),
                    },
                    hit_react_for_contact(
                        critical,
                        player_transform.translation,
                        enemy_transform.translation,
                        1.35,
                    ),
                    EnemyTargetFocus::new(critical),
                ));
            }
        }
        if runes.nova == NovaRune::Frost && nova_hits > 0 {
            inputs.stats.frost_nova_hits = inputs.stats.frost_nova_hits.saturating_add(nova_hits);
        }
        apply_combo_finisher(
            &mut inputs.combo,
            ComboFinisherPlayer {
                fury: &mut fury,
                health: &mut player_health,
                barrier: &mut player_barrier,
                equipment,
                charm,
            },
            ComboFinisher::Nova,
            nova_hits,
            ComboFinisherFeedback {
                combat_events: &mut inputs.combat_events,
                screen_shake: &mut inputs.screen_shake,
            },
        );
        if nova_hits > 0 {
            inputs.combo.prime(ComboPrimer::Nova);
            inputs.combat_events.write(CombatEvent {
                text: "Combo Ready: Dash through Nova-marked enemies".to_string(),
            });
        }
        apply_stormbrand_chains(
            &mut inputs.commands,
            &mut inputs.combat_events,
            &vfx,
            storm_chains,
            &mut enemies,
            &inputs.bestiary,
        );
        apply_boss_break_reward(
            &mut fury,
            &mut player_barrier,
            boss_staggers,
            &mut inputs.stats,
            &mut inputs.combat_events,
        );
        apply_life_steal(&mut player_health, life_stolen, &mut inputs.combat_events);
        apply_legendary_barrier(
            &mut player_barrier,
            barrier_gained,
            &mut inputs.combat_events,
        );
        apply_reflected_damage(
            &mut player_health,
            &mut player_barrier,
            reflected_damage,
            total_armor(equipment, elixir),
            evade,
            &mut inputs.combat_events,
        );
        apply_nova_breakable_damage(
            &mut inputs.commands,
            player_transform.translation,
            inputs.tuning.nova_radius,
            (inputs.tuning.nova_damage + damage_bonus)
                * total_multiplier
                * nova_damage_multiplier(runes.nova),
            &mut breakables,
        );
        let nova_base = cooldowns.nova_base;
        reset_scaled(
            &mut cooldowns.nova,
            nova_base,
            talents.cooldown_multiplier(),
        );
    }
}

fn queued_basic_arc_attack(
    facing: Vec3,
    range: f32,
    damage: f32,
    crit_chance: f32,
    legendary_powers: LegendaryPowerSet,
) -> PendingPlayerArcAttack {
    PendingPlayerArcAttack {
        timer: Timer::from_seconds(basic_attack_impact_delay_secs(), TimerMode::Once),
        facing,
        range,
        damage,
        impact_scale: 1.0,
        crit_chance,
        legendary_powers,
        skill: None,
    }
}

fn basic_attack_impact_delay_secs() -> f32 {
    0.14
}

fn resolve_pending_player_arc_attacks(
    mut inputs: PendingAttackInputs,
    mut players: PendingPlayerArcAttackQuery,
    mut enemies: EnemyTargets,
    mut breakables: BreakableTargets,
) {
    let vfx = combat_vfx_assets(&inputs.assets);
    for (
        player_entity,
        player_transform,
        mut player_health,
        mut player_barrier,
        mut fury,
        equipment,
        elixir,
        evade,
        surge,
        mut pending,
    ) in &mut players
    {
        pending.timer.tick(inputs.time.delta());
        if !pending.timer.is_finished() {
            continue;
        }

        let origin = player_transform.translation;
        let attack = ArcAttack {
            origin,
            facing: pending.facing,
            range: pending.range,
            damage: pending.damage,
            impact_scale: pending.impact_scale,
            crit_chance: pending.crit_chance,
            legendary_powers: pending.legendary_powers,
            skill: pending.skill,
        };
        let outcome = apply_arc_damage(
            &mut inputs.commands,
            &mut AttackFeedback {
                combat_events: &mut inputs.combat_events,
                floating_text: &mut inputs.floating_text,
                screen_shake: &mut inputs.screen_shake,
                hit_stop: &mut inputs.hit_stop,
            },
            &vfx,
            attack,
            &mut enemies,
            &inputs.bestiary,
        );
        apply_arc_breakable_damage(
            &mut inputs.commands,
            ArcAttack {
                origin,
                facing: pending.facing,
                range: pending.range,
                damage: pending.damage,
                impact_scale: 0.85,
                crit_chance: 0.0,
                legendary_powers: LegendaryPowerSet::none(),
                skill: None,
            },
            &mut breakables,
        );
        if outcome.hits > 0 {
            let basic_gain = fury.basic_gain;
            let gained = fury.gain(basic_gain * outcome.hits as f32 * surge_fury_multiplier(surge));
            if gained > 0.0 {
                inputs.combat_events.write(CombatEvent {
                    text: format!("Generated {gained:.0} fury"),
                });
            }
        }
        apply_boss_break_reward(
            &mut fury,
            &mut player_barrier,
            outcome.boss_staggers,
            &mut inputs.stats,
            &mut inputs.combat_events,
        );
        apply_life_steal(
            &mut player_health,
            outcome.life_stolen,
            &mut inputs.combat_events,
        );
        apply_legendary_barrier(
            &mut player_barrier,
            outcome.barrier_gained,
            &mut inputs.combat_events,
        );
        apply_reflected_damage(
            &mut player_health,
            &mut player_barrier,
            outcome.reflected_damage,
            total_armor(equipment, elixir),
            evade,
            &mut inputs.combat_events,
        );
        inputs
            .commands
            .entity(player_entity)
            .remove::<PendingPlayerArcAttack>();
    }
}

fn basic_attack_requested(
    mouse: &ButtonInput<MouseButton>,
    cooldowns: &SkillCooldowns,
    intent: &BasicAttackIntent,
    target: Option<PrimaryAttackTarget>,
) -> bool {
    let Some(target) = target else {
        return false;
    };
    let wants_attack = mouse.pressed(MouseButton::Left) || intent.target == Some(target.entity);
    wants_attack && target.in_range && basic_attack_ready(cooldowns)
}

fn basic_attack_ready(cooldowns: &SkillCooldowns) -> bool {
    cooldowns.basic.is_finished() || cooldowns.basic.remaining_secs() <= 0.05
}

fn update_basic_attack_intent(
    intent: &mut BasicAttackIntent,
    mouse: &ButtonInput<MouseButton>,
    cursor_ground: Option<Vec3>,
    cursor_target: Option<PrimaryAttackTarget>,
) {
    if !mouse.just_pressed(MouseButton::Left) {
        return;
    }
    if let Some(target) = cursor_target {
        intent.target = Some(target.entity);
    } else if cursor_ground.is_some() {
        intent.target = None;
    }
}

fn intended_primary_attack_target(
    intent: &mut BasicAttackIntent,
    cursor_target: Option<PrimaryAttackTarget>,
    player_position: Vec3,
    attack_range: f32,
    enemies: &EnemyTargets,
) -> Option<PrimaryAttackTarget> {
    if let Some(target) = cursor_target {
        return Some(target);
    }
    let entity = intent.target?;
    let Ok((_, transform, enemy, health, ..)) = enemies.get(entity) else {
        intent.target = None;
        return None;
    };
    if health.current <= 0.0 {
        intent.target = None;
        return None;
    }
    Some(PrimaryAttackTarget {
        entity,
        position: transform.translation,
        in_range: attack_target_in_range(
            player_position,
            transform.translation,
            attack_range,
            enemy,
        ),
    })
}

fn skill_unavailable_text(skill: &str, remaining_secs: f32) -> String {
    format!("{skill} not ready {:.0}s", remaining_secs.ceil().max(1.0))
}

fn cursor_ground_point(
    windows: &Query<&Window, With<PrimaryWindow>>,
    cameras: &Query<(&Camera, &GlobalTransform), With<Camera3d>>,
) -> Option<Vec3> {
    let window = windows.single().ok()?;
    let cursor_position = window.cursor_position()?;
    let (camera, camera_transform) = cameras.single().ok()?;
    let ray = camera
        .viewport_to_world(camera_transform, cursor_position)
        .ok()?;
    let distance = ray.intersect_plane(Vec3::ZERO, InfinitePlane3d::new(Vec3::Y))?;
    let mut point = ray.get_point(distance);
    point.y = 0.0;
    Some(point)
}

fn attack_facing_from_cursor(
    origin: Vec3,
    current_forward: Vec3,
    cursor_ground: Option<Vec3>,
) -> Vec3 {
    let Some(cursor_ground) = cursor_ground else {
        return horizontal_direction_or_forward(current_forward);
    };
    let direction = Vec3::new(cursor_ground.x - origin.x, 0.0, cursor_ground.z - origin.z);
    if direction.length_squared() <= 0.0025 {
        horizontal_direction_or_forward(current_forward)
    } else {
        direction.normalize_or_zero()
    }
}

fn horizontal_direction_or_forward(direction: Vec3) -> Vec3 {
    let direction = Vec3::new(direction.x, 0.0, direction.z).normalize_or_zero();
    if direction.length_squared() > 0.0 {
        direction
    } else {
        Vec3::Z
    }
}

fn cursor_ground_primary_attack_target(
    cursor_ground: Option<Vec3>,
    player_position: Vec3,
    attack_range: f32,
    enemies: &EnemyTargets,
) -> Option<PrimaryAttackTarget> {
    let cursor_ground = cursor_ground?;
    enemies
        .iter()
        .filter(|(_, transform, enemy, health, ..)| {
            health.current > 0.0
                && cursor_targets_enemy(cursor_ground, transform.translation, enemy)
        })
        .min_by(|(_, left_transform, ..), (_, right_transform, ..)| {
            flat_distance(cursor_ground, left_transform.translation)
                .total_cmp(&flat_distance(cursor_ground, right_transform.translation))
        })
        .map(|(entity, transform, enemy, ..)| PrimaryAttackTarget {
            entity,
            position: transform.translation,
            in_range: attack_target_in_range(
                player_position,
                transform.translation,
                attack_range,
                enemy,
            ),
        })
}

fn cursor_targets_enemy(cursor_ground: Vec3, enemy_position: Vec3, enemy: &Enemy) -> bool {
    flat_distance(cursor_ground, enemy_position) <= enemy_cursor_pick_radius(enemy)
}

fn attack_target_in_range(
    player_position: Vec3,
    enemy_position: Vec3,
    attack_range: f32,
    enemy: &Enemy,
) -> bool {
    let forgiveness = enemy_cursor_pick_radius(enemy).min(0.42) * 0.35;
    flat_distance(player_position, enemy_position) <= attack_range + forgiveness
}

fn flat_distance(a: Vec3, b: Vec3) -> f32 {
    Vec2::new(a.x - b.x, a.z - b.z).length()
}

fn dash_damage_multiplier(rune: DashRune) -> f32 {
    match rune {
        DashRune::Cleanse => 1.0,
        DashRune::Reap => 0.9,
    }
}

fn nova_damage_multiplier(rune: NovaRune) -> f32 {
    match rune {
        NovaRune::Ember => 1.0,
        NovaRune::Frost => 0.72,
    }
}

fn rupture_damage_multiplier(rune: RuptureRune) -> f32 {
    match rune {
        RuptureRune::Expose => 1.0,
        RuptureRune::Hemorrhage => 0.82,
    }
}

fn rupture_effect_text(rune: RuptureRune, affected: u32) -> String {
    match rune {
        RuptureRune::Expose => format!("Rupture exposed {affected} enemies"),
        RuptureRune::Hemorrhage => format!("Rupture bleeding {affected} enemies"),
    }
}

fn skill_cast_text(cast: SkillCast) -> String {
    match cast {
        SkillCast::Dash(rune) => format!("{} Dash", rune.label()),
        SkillCast::Rupture(rune) => format!("{} Rupture", rune.label()),
        SkillCast::Nova(rune) => format!("{} Nova", rune.label()),
    }
}

fn skill_cast_screen_shake(cast: SkillCast) -> ScreenShakeEvent {
    let (intensity, duration_secs) = match cast {
        SkillCast::Dash(_) => (0.026, 0.10),
        SkillCast::Rupture(_) => (0.036, 0.13),
        SkillCast::Nova(_) => (0.046, 0.16),
    };
    ScreenShakeEvent::new(intensity, duration_secs)
}

fn roll_critical(crit_chance: f32) -> bool {
    let mut rng = rand::rng();
    is_critical_hit(rng.random::<f32>(), crit_chance)
}

fn is_critical_hit(roll: f32, crit_chance: f32) -> bool {
    roll < crit_chance.clamp(0.0, 1.0)
}

fn apply_critical_damage(base_damage: f32, critical: bool) -> f32 {
    if critical {
        base_damage * critical_damage_multiplier()
    } else {
        base_damage
    }
}

fn legendary_power_skill_synergy(power: LegendaryPower, skill: Option<SkillCast>) -> bool {
    let Some(skill) = skill else {
        return false;
    };
    matches!(
        (power, skill),
        (LegendaryPower::Emberbrand, SkillCast::Nova(NovaRune::Ember))
            | (
                LegendaryPower::Emberbrand,
                SkillCast::Rupture(RuptureRune::Hemorrhage)
            )
            | (LegendaryPower::Frostbrand, SkillCast::Nova(NovaRune::Frost))
            | (
                LegendaryPower::Frostbrand,
                SkillCast::Rupture(RuptureRune::Expose)
            )
            | (LegendaryPower::Stormbrand, SkillCast::Dash(DashRune::Reap))
            | (LegendaryPower::Stormbrand, SkillCast::Nova(NovaRune::Frost))
            | (LegendaryPower::Soulreaver, SkillCast::Dash(DashRune::Reap))
            | (
                LegendaryPower::Soulreaver,
                SkillCast::Rupture(RuptureRune::Hemorrhage)
            )
            | (
                LegendaryPower::Aegisbrand,
                SkillCast::Dash(DashRune::Cleanse)
            )
            | (
                LegendaryPower::Aegisbrand,
                SkillCast::Rupture(RuptureRune::Expose)
            )
    )
}

fn legendary_proc_ratio(
    trigger: LegendaryProcTrigger,
    power: LegendaryPower,
    critical_ratio: f32,
    skill_ratio: f32,
    combined_ratio: f32,
) -> Option<f32> {
    match (trigger.critical, trigger.has_skill_synergy(power)) {
        (true, true) => Some(combined_ratio),
        (true, false) => Some(critical_ratio),
        (false, true) => Some(skill_ratio),
        (false, false) => None,
    }
}

fn legendary_burning(
    powers: LegendaryPowerSet,
    trigger: LegendaryProcTrigger,
    triggering_damage: f32,
) -> Option<Burning> {
    if !powers.contains(LegendaryPower::Emberbrand)
        || !trigger.should_proc(LegendaryPower::Emberbrand)
    {
        return None;
    }
    let ratio = legendary_proc_ratio(trigger, LegendaryPower::Emberbrand, 0.22, 0.16, 0.28)?;
    let duration_secs = if trigger.has_skill_synergy(LegendaryPower::Emberbrand) {
        3.6
    } else {
        3.0
    };
    Some(Burning {
        dps: (triggering_damage * ratio).max(4.0),
        timer: Timer::from_seconds(duration_secs, TimerMode::Once),
    })
}

fn legendary_chilled(powers: LegendaryPowerSet, trigger: LegendaryProcTrigger) -> Option<Chilled> {
    if !powers.contains(LegendaryPower::Frostbrand)
        || !trigger.should_proc(LegendaryPower::Frostbrand)
    {
        return None;
    }
    let synergized = trigger.has_skill_synergy(LegendaryPower::Frostbrand);
    Some(Chilled {
        timer: Timer::from_seconds(
            if trigger.critical && synergized {
                3.1
            } else if synergized {
                2.7
            } else {
                2.4
            },
            TimerMode::Once,
        ),
        slow_multiplier: if trigger.critical && synergized {
            0.44
        } else if synergized {
            0.48
        } else {
            0.52
        },
    })
}

fn stormbrand_chain_damage(
    powers: LegendaryPowerSet,
    trigger: LegendaryProcTrigger,
    triggering_damage: f32,
) -> Option<f32> {
    if !powers.contains(LegendaryPower::Stormbrand)
        || !trigger.should_proc(LegendaryPower::Stormbrand)
    {
        return None;
    }
    let ratio = legendary_proc_ratio(trigger, LegendaryPower::Stormbrand, 0.42, 0.30, 0.50)?;
    Some((triggering_damage * ratio).max(6.0))
}

fn legendary_life_steal(
    powers: LegendaryPowerSet,
    trigger: LegendaryProcTrigger,
    triggering_damage: f32,
) -> f32 {
    if !powers.contains(LegendaryPower::Soulreaver)
        || !trigger.should_proc(LegendaryPower::Soulreaver)
    {
        return 0.0;
    }
    let Some(ratio) = legendary_proc_ratio(trigger, LegendaryPower::Soulreaver, 0.18, 0.12, 0.23)
    else {
        return 0.0;
    };
    (triggering_damage * ratio).max(3.0)
}

fn legendary_barrier(
    powers: LegendaryPowerSet,
    trigger: LegendaryProcTrigger,
    triggering_damage: f32,
) -> f32 {
    if !powers.contains(LegendaryPower::Aegisbrand)
        || !trigger.should_proc(LegendaryPower::Aegisbrand)
    {
        return 0.0;
    }
    let Some(ratio) = legendary_proc_ratio(trigger, LegendaryPower::Aegisbrand, 0.20, 0.14, 0.26)
    else {
        return 0.0;
    };
    (triggering_damage * ratio).max(5.0)
}

fn reflective_damage(enemy: &Enemy, triggering_damage: f32) -> f32 {
    if !enemy.affixes.contains(&EnemyAffix::Reflective) {
        return 0.0;
    }
    (triggering_damage * 0.08).clamp(2.0, 18.0)
}

fn apply_life_steal(
    health: &mut Health,
    amount: f32,
    combat_events: &mut MessageWriter<CombatEvent>,
) {
    if amount <= 0.0 || health.current >= health.max {
        return;
    }
    let before = health.current;
    health.current = (health.current + amount).min(health.max);
    let restored = health.current - before;
    if restored > 0.0 {
        combat_events.write(CombatEvent {
            text: format!("Soulreaver restored {restored:.0} health"),
        });
    }
}

fn apply_legendary_barrier(
    barrier: &mut Barrier,
    amount: f32,
    combat_events: &mut MessageWriter<CombatEvent>,
) {
    let gained = barrier.grant(amount);
    if gained > 0.0 {
        combat_events.write(CombatEvent {
            text: format!("Aegisbrand granted {gained:.0} barrier"),
        });
    }
}

fn apply_reflected_damage(
    health: &mut Health,
    barrier: &mut Barrier,
    amount: f32,
    armor: f32,
    evade: &Evade,
    combat_events: &mut MessageWriter<CombatEvent>,
) {
    if amount <= 0.0 {
        return;
    }
    let damage = mitigated_damage(amount, armor);
    let taken = apply_player_damage_with_evade(health, barrier, evade, damage);
    let total = taken.absorbed + taken.health_damage;
    if total > 0.0 {
        combat_events.write(CombatEvent {
            text: format!("Reflective affix returned {total:.0} damage"),
        });
    }
}

fn stormbrand_chain_radius() -> f32 {
    4.2
}

fn critical_damage_multiplier() -> f32 {
    1.75
}

fn damage_text(source: &str, damage: f32, critical: bool) -> String {
    if critical {
        format!("Critical {source} hit for {damage:.0}")
    } else {
        format!("{source} hit for {damage:.0}")
    }
}

fn hit_impact_profile(
    enemy: &Enemy,
    critical: bool,
    impact_scale: f32,
    boss_break: bool,
) -> HitImpactProfile {
    let tier = if boss_break {
        HitImpactTier::BossBreak
    } else if enemy.id == "keeper" {
        HitImpactTier::Boss
    } else if enemy.id == "seal_warden"
        || enemy.id == "nemesis"
        || enemy.id == "butcher"
        || !enemy.affixes.is_empty()
    {
        HitImpactTier::Elite
    } else {
        HitImpactTier::Normal
    };
    HitImpactProfile {
        critical,
        impact_scale: impact_scale.clamp(0.5, 1.8),
        tier,
    }
}

fn floating_damage_text(damage: f32, critical: bool) -> String {
    if critical {
        format!("CRIT {:.0}", damage)
    } else {
        format!("{:.0}", damage)
    }
}

fn floating_damage_text_for_profile(damage: f32, profile: HitImpactProfile) -> String {
    match (profile.tier, profile.critical) {
        (HitImpactTier::BossBreak, _) => format!("BREAK {:.0}", damage),
        (HitImpactTier::Boss, true) => format!("BOSS CRIT {:.0}", damage),
        (HitImpactTier::Boss, false) => format!("BOSS {:.0}", damage),
        (HitImpactTier::Elite, true) => format!("ELITE CRIT {:.0}", damage),
        (HitImpactTier::Elite, false) => format!("ELITE {:.0}", damage),
        (HitImpactTier::Normal, critical) => floating_damage_text(damage, critical),
    }
}

fn floating_damage_text_for_ward_result(
    health_damage: f32,
    ward: WardAdjustedDamage,
    profile: HitImpactProfile,
) -> String {
    if ward.ward_broken {
        return if health_damage > 0.0 {
            format!("WARD BREAK {:.0}", health_damage)
        } else {
            "WARD BREAK".to_string()
        };
    }
    if ward.ward_damage > 0.0 && health_damage <= 0.0 {
        return format!("WARD {:.0}", ward.ward_damage);
    }
    floating_damage_text_for_profile(health_damage, profile)
}

fn hit_impact_tier_multiplier(tier: HitImpactTier) -> f32 {
    match tier {
        HitImpactTier::Normal => 1.0,
        HitImpactTier::Elite => 1.18,
        HitImpactTier::Boss => 1.34,
        HitImpactTier::BossBreak => 1.68,
    }
}

fn hit_impact_vfx_scale(profile: HitImpactProfile) -> f32 {
    let critical_weight = if profile.critical { 1.14 } else { 1.0 };
    (profile.impact_scale * hit_impact_tier_multiplier(profile.tier) * critical_weight)
        .clamp(0.68, 2.35)
}

fn hit_impact_screen_shake_event(critical: bool, impact_scale: f32) -> ScreenShakeEvent {
    let impact_scale = impact_scale.clamp(0.5, 1.8);
    let base_intensity = if critical { 0.055 } else { 0.022 };
    let base_duration = if critical { 0.14 } else { 0.09 };
    ScreenShakeEvent::new(
        (base_intensity * impact_scale).clamp(0.014, 0.095),
        (base_duration + (impact_scale - 1.0).max(0.0) * 0.04).clamp(0.08, 0.20),
    )
}

fn hit_impact_screen_shake_event_for_profile(profile: HitImpactProfile) -> ScreenShakeEvent {
    let tier_multiplier = hit_impact_tier_multiplier(profile.tier);
    hit_impact_screen_shake_event(profile.critical, profile.impact_scale * tier_multiplier)
}

fn hit_impact_hit_stop_event(critical: bool, impact_scale: f32) -> HitStopEvent {
    let impact_scale = impact_scale.clamp(0.5, 1.8);
    let extra_weight = (impact_scale - 1.0).max(0.0);
    let duration_secs = (if critical { 0.056 } else { 0.024 }) + extra_weight * 0.026;
    let relative_speed = (if critical { 0.30 } else { 0.58 }) - extra_weight * 0.10;
    HitStopEvent::new(relative_speed, duration_secs)
}

fn hit_impact_hit_stop_event_for_profile(profile: HitImpactProfile) -> HitStopEvent {
    let tier_multiplier = hit_impact_tier_multiplier(profile.tier);
    hit_impact_hit_stop_event(profile.critical, profile.impact_scale * tier_multiplier)
}

fn trigger_hit_impact_screen_shake(
    screen_shake: &mut MessageWriter<ScreenShakeEvent>,
    profile: HitImpactProfile,
) {
    screen_shake.write(hit_impact_screen_shake_event_for_profile(profile));
}

fn trigger_hit_impact_hit_stop(
    hit_stop: &mut MessageWriter<HitStopEvent>,
    profile: HitImpactProfile,
) {
    hit_stop.write(hit_impact_hit_stop_event_for_profile(profile));
}

fn boss_break_hit_stop_event() -> HitStopEvent {
    HitStopEvent::new(0.20, 0.105)
}

fn trigger_boss_stagger(
    commands: &mut Commands,
    combat_events: &mut MessageWriter<CombatEvent>,
    vfx: &CombatVfxAssets,
    enemy: &Enemy,
    hit: BossStaggerTrigger,
    stagger: Option<Mut<BossStagger>>,
    staggered: Option<&Staggered>,
) -> bool {
    if enemy.id != "keeper" || staggered.is_some() {
        return false;
    }
    let Some(mut stagger) = stagger else {
        return false;
    };
    if add_boss_stagger(&mut stagger, hit.damage) {
        commands.entity(hit.entity).try_insert(boss_staggered());
        spawn_stagger_vfx(commands, vfx, hit.target, enemy);
        combat_events.write(CombatEvent {
            text: "Malrec staggered: punish the exposed keeper".to_string(),
        });
        return true;
    }
    false
}

fn apply_boss_break_reward(
    fury: &mut Fury,
    barrier: &mut Barrier,
    boss_staggers: u32,
    stats: &mut RunStats,
    combat_events: &mut MessageWriter<CombatEvent>,
) {
    if boss_staggers == 0 {
        return;
    }
    let fury_gained = fury.gain(boss_break_fury_reward() * boss_staggers as f32);
    let barrier_gained = barrier.grant(boss_break_barrier_reward() * boss_staggers as f32);
    stats.boss_staggers += boss_staggers;
    combat_events.write(CombatEvent {
        text: boss_break_reward_text(boss_staggers, fury_gained, barrier_gained),
    });
}

fn boss_break_fury_reward() -> f32 {
    18.0
}

fn boss_break_barrier_reward() -> f32 {
    22.0
}

fn boss_break_reward_text(boss_staggers: u32, fury_gained: f32, barrier_gained: f32) -> String {
    format!(
        "Boss Break x{}: EXECUTE WINDOW - Surge now, +{:.0} fury +{:.0} barrier",
        boss_staggers, fury_gained, barrier_gained
    )
}

fn apply_combo_finisher(
    combo: &mut CombatComboState,
    player: ComboFinisherPlayer,
    finisher: ComboFinisher,
    hits: u32,
    feedback: ComboFinisherFeedback,
) {
    let Some(primer) = combo.active_primer() else {
        return;
    };
    if !combo_finisher_allowed(primer, finisher) || hits == 0 {
        return;
    }
    let chain_bonus = combo_chain_bonus(primer, finisher, hits);
    let set_bonus = combo_set_bonus(themed_reliquary_set(player.equipment, player.charm), hits);
    let bonus_label = combo_bonus_label(chain_bonus.label, set_bonus.label);
    let fury_gained = player
        .fury
        .gain(combo_fury_reward(hits) + chain_bonus.fury + set_bonus.fury);
    let healing_done = combo_heal(player.health, chain_bonus.healing + set_bonus.healing);
    let barrier_gained = player
        .barrier
        .grant(chain_bonus.barrier + set_bonus.barrier);
    feedback.combat_events.write(CombatEvent {
        text: combo_break_text(
            primer,
            finisher,
            hits,
            fury_gained,
            healing_done,
            barrier_gained,
            bonus_label.as_deref(),
        ),
    });
    feedback.screen_shake.write(ScreenShakeEvent::new(
        combo_screen_shake(hits) + chain_bonus.shake + set_bonus.shake,
        0.12,
    ));
    combo.clear();
}

fn combo_finisher_allowed(primer: ComboPrimer, finisher: ComboFinisher) -> bool {
    matches!(
        (primer, finisher),
        (ComboPrimer::Rupture, ComboFinisher::Dash)
            | (ComboPrimer::Rupture, ComboFinisher::Nova)
            | (ComboPrimer::Nova, ComboFinisher::Dash)
    )
}

fn combo_fury_reward(hits: u32) -> f32 {
    8.0 + hits.min(5) as f32 * 3.0
}

fn combo_screen_shake(hits: u32) -> f32 {
    0.045 + hits.min(5) as f32 * 0.008
}

fn combo_chain_bonus(primer: ComboPrimer, finisher: ComboFinisher, hits: u32) -> ComboChainBonus {
    if !combo_finisher_allowed(primer, finisher) || hits == 0 {
        return ComboChainBonus::default();
    }

    let hits = hits.min(5) as f32;
    let multi_target_fury = if hits >= 3.0 { hits * 2.0 } else { 0.0 };
    match (primer, finisher) {
        (ComboPrimer::Rupture, ComboFinisher::Dash) => ComboChainBonus {
            fury: multi_target_fury,
            barrier: 5.0 + hits * 3.0,
            shake: 0.015,
            label: Some("Cull dash"),
            ..default()
        },
        (ComboPrimer::Rupture, ComboFinisher::Nova) => ComboChainBonus {
            fury: multi_target_fury,
            healing: 5.0 + hits * 3.0,
            shake: 0.02,
            label: Some("Blood nova"),
            ..default()
        },
        (ComboPrimer::Nova, ComboFinisher::Dash) => ComboChainBonus {
            fury: 2.0 + multi_target_fury,
            barrier: 6.0 + hits * 4.0,
            shake: 0.02,
            label: Some("Frostbreak dash"),
            ..default()
        },
        _ => ComboChainBonus::default(),
    }
}

fn combo_set_bonus(set: Option<ReliquarySet>, hits: u32) -> ComboSetBonus {
    let hits = hits.min(5) as f32;
    match set {
        Some(ReliquarySet::Storm) => ComboSetBonus {
            shake: 0.035,
            label: Some("Storm surge"),
            ..default()
        },
        Some(ReliquarySet::Blood) => ComboSetBonus {
            healing: 6.0 + hits * 4.0,
            label: Some("Blood pact"),
            ..default()
        },
        Some(ReliquarySet::Ashen) => ComboSetBonus {
            barrier: 8.0 + hits * 5.0,
            label: Some("Ashen ward"),
            ..default()
        },
        Some(ReliquarySet::Reliquary) => ComboSetBonus {
            fury: 5.0 + hits * 3.0,
            label: Some("Reliquary overflow"),
            ..default()
        },
        None => ComboSetBonus::default(),
    }
}

fn combo_heal(health: &mut Health, amount: f32) -> f32 {
    let before = health.current;
    health.current = (health.current + amount.max(0.0)).min(health.max);
    health.current - before
}

fn combo_bonus_label(chain_label: Option<&str>, set_label: Option<&str>) -> Option<String> {
    match (chain_label, set_label) {
        (Some(chain), Some(set)) => Some(format!("{chain} | {set}")),
        (Some(chain), None) => Some(chain.to_string()),
        (None, Some(set)) => Some(set.to_string()),
        (None, None) => None,
    }
}

fn combo_break_text(
    primer: ComboPrimer,
    finisher: ComboFinisher,
    hits: u32,
    fury_gained: f32,
    healing_done: f32,
    barrier_gained: f32,
    set_label: Option<&str>,
) -> String {
    let mut text = format!(
        "Combo Break: {} into {} hit {} target{} +{:.0} fury",
        combo_primer_label(primer),
        combo_finisher_label(finisher),
        hits,
        if hits == 1 { "" } else { "s" },
        fury_gained
    );
    if healing_done > 0.0 {
        text.push_str(&format!(" +{healing_done:.0} health"));
    }
    if barrier_gained > 0.0 {
        text.push_str(&format!(" +{barrier_gained:.0} barrier"));
    }
    if let Some(label) = set_label {
        text.push_str(&format!(" | {label}"));
    }
    text
}

fn combo_primer_label(primer: ComboPrimer) -> &'static str {
    match primer {
        ComboPrimer::Rupture => "Rupture",
        ComboPrimer::Nova => "Nova",
    }
}

fn combo_finisher_label(finisher: ComboFinisher) -> &'static str {
    match finisher {
        ComboFinisher::Dash => "Dash",
        ComboFinisher::Nova => "Nova",
    }
}

fn combo_hud_finisher_label(primer: ComboPrimer) -> &'static str {
    match primer {
        ComboPrimer::Rupture => "Q/E",
        ComboPrimer::Nova => "Q",
    }
}

fn reset_scaled(timer: &mut Timer, base_seconds: f32, multiplier: f32) {
    timer.set_duration(std::time::Duration::from_secs_f32(
        base_seconds * multiplier,
    ));
    timer.reset();
}

fn combat_vfx_assets(assets: &GameAssets) -> CombatVfxAssets {
    CombatVfxAssets {
        slash_arc: assets.slash_arc.clone(),
        hit_spark: assets.hit_spark.clone(),
        bone_shatter: assets.bone_shatter.clone(),
        bone_impact: assets.bone_impact.clone(),
        blood_spray: assets.blood_spray.clone(),
        execution_burst: assets.execution_burst.clone(),
        arcane_impact: assets.arcane_impact.clone(),
        holy_impact: assets.holy_impact.clone(),
        ember_impact: assets.ember_impact.clone(),
        frost_impact: assets.frost_impact.clone(),
        void_impact: assets.void_impact.clone(),
        frenzy_impact: assets.frenzy_impact.clone(),
        vampiric_siphon: assets.vampiric_siphon.clone(),
        desecrator_burst: assets.desecrator_burst.clone(),
        guard_clash: assets.guard_clash.clone(),
        armor_break: assets.armor_break.clone(),
        soul_ward_hit: assets.soul_ward_hit.clone(),
        hit_bone_rune: assets.hit_bone_rune.clone(),
        hit_bone_lock: assets.hit_bone_lock.clone(),
        marrow_flash: assets.marrow_flash.clone(),
        bone_fracture_echo: assets.bone_fracture_echo.clone(),
        elite_affix_break: assets.elite_affix_break.clone(),
        shadow_burst: assets.shadow_burst.clone(),
        headshot_burst: assets.headshot_burst.clone(),
        crit_bone_crown: assets.crit_bone_crown.clone(),
        crit_burst: assets.crit_burst.clone(),
        stagger_burst: assets.stagger_burst.clone(),
        shadow_trail: assets.shadow_trail.clone(),
    }
}

fn spawn_slash_vfx(
    commands: &mut Commands,
    assets: &CombatVfxAssets,
    origin: Vec3,
    facing: Vec3,
    scale: f32,
) {
    let forward = Vec3::new(facing.x, 0.0, facing.z).normalize_or_zero();
    let yaw = forward.x.atan2(forward.z);
    spawn_combat_vfx(
        commands,
        assets.slash_arc.clone(),
        Transform::from_translation(origin + forward * 1.05 + Vec3::Y * 0.42)
            .with_rotation(Quat::from_rotation_y(yaw))
            .with_scale(Vec3::splat(scale)),
        0.24,
    );
}

fn spawn_nova_cast_vfx(
    commands: &mut Commands,
    assets: &CombatVfxAssets,
    origin: Vec3,
    rune: NovaRune,
    radius: f32,
) {
    let scene = match rune {
        NovaRune::Ember => assets.ember_impact.clone(),
        NovaRune::Frost => assets.frost_impact.clone(),
    };
    spawn_combat_vfx(
        commands,
        scene,
        nova_cast_transform(origin, radius),
        nova_cast_lifetime(rune),
    );
    spawn_combat_vfx(
        commands,
        assets.slash_arc.clone(),
        nova_outer_ring_transform(origin, radius),
        nova_outer_ring_lifetime(rune),
    );
}

fn nova_cast_transform(origin: Vec3, radius: f32) -> Transform {
    Transform::from_translation(origin + Vec3::Y * 0.18)
        .with_scale(Vec3::splat(nova_cast_scale(radius)))
}

fn nova_outer_ring_transform(origin: Vec3, radius: f32) -> Transform {
    Transform::from_translation(origin + Vec3::Y * 0.10)
        .with_rotation(Quat::from_rotation_x(std::f32::consts::FRAC_PI_2))
        .with_scale(Vec3::splat((radius / 1.65).clamp(1.45, 4.10)))
}

fn nova_cast_scale(radius: f32) -> f32 {
    (radius / 2.75).clamp(1.10, 2.85)
}

fn nova_cast_lifetime(rune: NovaRune) -> f32 {
    match rune {
        NovaRune::Ember => 0.40,
        NovaRune::Frost => 0.48,
    }
}

fn nova_outer_ring_lifetime(rune: NovaRune) -> f32 {
    nova_cast_lifetime(rune) + 0.10
}

fn spawn_dash_cast_vfx(
    commands: &mut Commands,
    assets: &CombatVfxAssets,
    start: Vec3,
    end: Vec3,
    facing: Vec3,
    rune: DashRune,
) {
    for (progress, scale) in [(0.24, 0.74), (0.52, 0.96), (0.80, 1.14)] {
        spawn_combat_vfx(
            commands,
            assets.shadow_trail.clone(),
            dash_afterimage_transform(start, end, facing, progress, scale),
            0.20 + progress * 0.12,
        );
    }
    let scene = match rune {
        DashRune::Cleanse => assets.holy_impact.clone(),
        DashRune::Reap => assets.shadow_burst.clone(),
    };
    spawn_combat_vfx(
        commands,
        scene,
        dash_arrival_transform(end, facing, rune),
        dash_arrival_lifetime(rune),
    );
}

fn dash_afterimage_transform(
    start: Vec3,
    end: Vec3,
    facing: Vec3,
    progress: f32,
    scale: f32,
) -> Transform {
    let progress = progress.clamp(0.0, 1.0);
    let forward = Vec3::new(facing.x, 0.0, facing.z).normalize_or_zero();
    let yaw = forward.x.atan2(forward.z);
    Transform::from_translation(start.lerp(end, progress) - forward * 0.18 + Vec3::Y * 0.16)
        .with_rotation(Quat::from_rotation_y(yaw))
        .with_scale(Vec3::new(scale * 0.82, scale * 0.72, scale * 1.30))
}

fn dash_arrival_transform(end: Vec3, facing: Vec3, rune: DashRune) -> Transform {
    let forward = Vec3::new(facing.x, 0.0, facing.z).normalize_or_zero();
    let yaw = forward.x.atan2(forward.z);
    let scale = match rune {
        DashRune::Cleanse => 0.98,
        DashRune::Reap => 1.16,
    };
    Transform::from_translation(end + forward * 0.48 + Vec3::Y * 0.22)
        .with_rotation(Quat::from_rotation_y(yaw))
        .with_scale(Vec3::splat(scale))
}

fn dash_arrival_lifetime(rune: DashRune) -> f32 {
    match rune {
        DashRune::Cleanse => 0.28,
        DashRune::Reap => 0.34,
    }
}

fn spawn_rupture_cast_vfx(
    commands: &mut Commands,
    assets: &CombatVfxAssets,
    origin: Vec3,
    facing: Vec3,
    range: f32,
    rune: RuptureRune,
) {
    for side in [-1.0, 0.0, 1.0] {
        spawn_combat_vfx(
            commands,
            assets.slash_arc.clone(),
            rupture_cone_transform(origin, facing, range, side),
            0.32,
        );
    }
    let scene = match rune {
        RuptureRune::Expose => assets.armor_break.clone(),
        RuptureRune::Hemorrhage => assets.blood_spray.clone(),
    };
    spawn_combat_vfx(
        commands,
        scene,
        rupture_tip_transform(origin, facing, range, rune),
        rupture_tip_lifetime(rune),
    );
}

fn rupture_cone_transform(origin: Vec3, facing: Vec3, range: f32, side: f32) -> Transform {
    let forward = Vec3::new(facing.x, 0.0, facing.z).normalize_or_zero();
    let right = Vec3::new(forward.z, 0.0, -forward.x);
    let side = side.clamp(-1.0, 1.0);
    let yaw = forward.x.atan2(forward.z) + side * 0.38;
    let reach = range.max(0.5);
    Transform::from_translation(
        origin + forward * (reach * 0.56) + right * side * (reach * 0.28) + Vec3::Y * 0.30,
    )
    .with_rotation(Quat::from_rotation_y(yaw))
    .with_scale(Vec3::new(1.08 + side.abs() * 0.18, 0.58, reach * 0.58))
}

fn rupture_tip_transform(origin: Vec3, facing: Vec3, range: f32, rune: RuptureRune) -> Transform {
    let forward = Vec3::new(facing.x, 0.0, facing.z).normalize_or_zero();
    let yaw = forward.x.atan2(forward.z);
    let scale = match rune {
        RuptureRune::Expose => 0.95,
        RuptureRune::Hemorrhage => 1.08,
    };
    Transform::from_translation(origin + forward * range.max(0.5) + Vec3::Y * 0.24)
        .with_rotation(Quat::from_rotation_y(yaw))
        .with_scale(Vec3::splat(scale))
}

fn rupture_tip_lifetime(rune: RuptureRune) -> f32 {
    match rune {
        RuptureRune::Expose => 0.36,
        RuptureRune::Hemorrhage => 0.42,
    }
}

fn spawn_hit_vfx(
    commands: &mut Commands,
    assets: &CombatVfxAssets,
    target: Vec3,
    enemy: &Enemy,
    critical: bool,
    scale: f32,
) {
    let hit_bone = select_hit_bone(enemy, critical);
    let effect = select_hit_effect(enemy, hit_bone, critical);
    let scene = hit_effect_scene(assets, effect);
    spawn_combat_vfx(
        commands,
        scene,
        hit_effect_transform(target, enemy, hit_bone, effect, scale),
        hit_effect_lifetime(effect, critical),
    );
    let contact = HitContact {
        target,
        hit_bone,
        effect,
        critical,
        scale,
    };
    spawn_hit_bone_lock_vfx(commands, assets, enemy, contact);
    spawn_hit_bone_overlay_vfx(commands, assets, enemy, contact);
    spawn_fracture_echo_vfx(commands, assets, enemy, contact);
    spawn_affix_break_overlay_vfx(commands, assets, enemy, contact);
}

fn spawn_stagger_vfx(
    commands: &mut Commands,
    assets: &CombatVfxAssets,
    target: Vec3,
    enemy: &Enemy,
) {
    spawn_combat_vfx(
        commands,
        assets.stagger_burst.clone(),
        Transform::from_translation(target + hit_bone_offset(HitBone::Chest, enemy))
            .with_scale(Vec3::splat(1.55)),
        0.5,
    );
    spawn_combat_vfx(
        commands,
        assets.holy_impact.clone(),
        Transform::from_translation(target + hit_bone_offset(HitBone::Head, enemy))
            .with_scale(Vec3::splat(1.1)),
        0.42,
    );
}

fn spawn_shadow_trail_vfx(
    commands: &mut Commands,
    assets: &CombatVfxAssets,
    origin: Vec3,
    facing: Vec3,
    scale: f32,
) {
    let forward = Vec3::new(facing.x, 0.0, facing.z).normalize_or_zero();
    let yaw = forward.x.atan2(forward.z);
    spawn_combat_vfx(
        commands,
        assets.shadow_trail.clone(),
        Transform::from_translation(origin - forward * 0.45 + Vec3::Y * 0.18)
            .with_rotation(Quat::from_rotation_y(yaw))
            .with_scale(Vec3::splat(scale)),
        0.32,
    );
}

fn select_hit_bone(enemy: &Enemy, critical: bool) -> HitBone {
    if critical {
        HitBone::Head
    } else if matches!(enemy.id.as_str(), "keeper" | "butcher" | "seal_warden") {
        HitBone::Chest
    } else if enemy
        .affixes
        .iter()
        .any(|affix| matches!(affix, EnemyAffix::Shielded | EnemyAffix::Reflective))
    {
        HitBone::Weapon
    } else if enemy.id == "cultist" {
        HitBone::Right
    } else {
        HitBone::Left
    }
}

fn select_hit_effect(enemy: &Enemy, hit_bone: HitBone, critical: bool) -> HitEffect {
    if critical {
        return if hit_bone == HitBone::Head {
            HitEffect::Headshot
        } else {
            HitEffect::Critical
        };
    }
    if enemy.affixes.contains(&EnemyAffix::Arcane) {
        return HitEffect::Arcane;
    }
    if enemy.affixes.contains(&EnemyAffix::Molten) {
        return HitEffect::Ember;
    }
    if enemy.affixes.contains(&EnemyAffix::Frenzied) {
        return HitEffect::Frenzy;
    }
    let shielded = enemy.affixes.contains(&EnemyAffix::Shielded);
    let reflective = enemy.affixes.contains(&EnemyAffix::Reflective);
    if shielded && reflective {
        return HitEffect::GuardClash;
    }
    if reflective {
        return HitEffect::SoulWard;
    }
    if shielded {
        return HitEffect::ArmorBreak;
    }
    if enemy
        .affixes
        .iter()
        .any(|affix| matches!(affix, EnemyAffix::Frozen))
    {
        return HitEffect::Frost;
    }
    if enemy.affixes.contains(&EnemyAffix::Jailer) {
        return HitEffect::Holy;
    }
    let vampiric = enemy.affixes.contains(&EnemyAffix::Vampiric);
    let desecrator = enemy.affixes.contains(&EnemyAffix::Desecrator);
    if vampiric && desecrator {
        return HitEffect::Void;
    }
    if vampiric {
        return HitEffect::Vampiric;
    }
    if desecrator {
        return HitEffect::Desecrator;
    }
    if matches!(enemy.id.as_str(), "skeleton" | "nemesis") {
        return if matches!(hit_bone, HitBone::Head | HitBone::Chest) {
            HitEffect::BoneImpact
        } else {
            HitEffect::Bone
        };
    }
    if matches!(enemy.id.as_str(), "keeper" | "butcher") {
        return if matches!(hit_bone, HitBone::Chest) {
            HitEffect::Execution
        } else {
            HitEffect::Blood
        };
    }
    if matches!(enemy.id.as_str(), "cultist" | "seal_warden") {
        return HitEffect::Shadow;
    }
    HitEffect::Spark
}

fn hit_effect_scene(assets: &CombatVfxAssets, effect: HitEffect) -> Handle<WorldAsset> {
    match effect {
        HitEffect::Spark => assets.hit_spark.clone(),
        HitEffect::Bone => assets.bone_shatter.clone(),
        HitEffect::BoneImpact => assets.bone_impact.clone(),
        HitEffect::Blood => assets.blood_spray.clone(),
        HitEffect::Execution => assets.execution_burst.clone(),
        HitEffect::Arcane => assets.arcane_impact.clone(),
        HitEffect::Holy => assets.holy_impact.clone(),
        HitEffect::Ember => assets.ember_impact.clone(),
        HitEffect::Frost => assets.frost_impact.clone(),
        HitEffect::Void => assets.void_impact.clone(),
        HitEffect::Frenzy => assets.frenzy_impact.clone(),
        HitEffect::Vampiric => assets.vampiric_siphon.clone(),
        HitEffect::Desecrator => assets.desecrator_burst.clone(),
        HitEffect::GuardClash => assets.guard_clash.clone(),
        HitEffect::ArmorBreak => assets.armor_break.clone(),
        HitEffect::SoulWard => assets.soul_ward_hit.clone(),
        HitEffect::Shadow => assets.shadow_burst.clone(),
        HitEffect::Headshot => assets.headshot_burst.clone(),
        HitEffect::Critical => assets.crit_burst.clone(),
    }
}

fn spawn_hit_bone_overlay_vfx(
    commands: &mut Commands,
    assets: &CombatVfxAssets,
    enemy: &Enemy,
    contact: HitContact,
) {
    let overlay_scene = hit_bone_overlay_scene(
        assets,
        enemy,
        contact.hit_bone,
        contact.effect,
        contact.critical,
    );
    spawn_combat_vfx(
        commands,
        overlay_scene,
        hit_bone_overlay_transform(
            contact.target,
            enemy,
            contact.hit_bone,
            contact.effect,
            contact.scale,
        ),
        hit_bone_overlay_lifetime(contact.hit_bone, contact.critical),
    );
}

fn spawn_affix_break_overlay_vfx(
    commands: &mut Commands,
    assets: &CombatVfxAssets,
    enemy: &Enemy,
    contact: HitContact,
) {
    if enemy.affixes.len() < 2 {
        return;
    }
    spawn_combat_vfx(
        commands,
        assets.elite_affix_break.clone(),
        hit_bone_overlay_transform(
            contact.target,
            enemy,
            contact.hit_bone,
            HitEffect::Critical,
            contact.scale * 1.08,
        ),
        0.34,
    );
}

fn spawn_hit_bone_lock_vfx(
    commands: &mut Commands,
    assets: &CombatVfxAssets,
    enemy: &Enemy,
    contact: HitContact,
) {
    spawn_combat_vfx(
        commands,
        assets.hit_bone_lock.clone(),
        hit_bone_lock_transform(
            contact.target,
            enemy,
            contact.hit_bone,
            contact.effect,
            contact.scale,
        ),
        hit_bone_lock_lifetime(contact.hit_bone, contact.critical),
    );
}

fn spawn_fracture_echo_vfx(
    commands: &mut Commands,
    assets: &CombatVfxAssets,
    enemy: &Enemy,
    contact: HitContact,
) {
    if !should_spawn_fracture_echo(enemy, contact.hit_bone, contact.effect, contact.critical) {
        return;
    }
    let scene = if contact.critical && contact.hit_bone == HitBone::Head {
        assets.crit_bone_crown.clone()
    } else {
        assets.bone_fracture_echo.clone()
    };
    spawn_combat_vfx(
        commands,
        scene,
        fracture_echo_transform(
            contact.target,
            enemy,
            contact.hit_bone,
            contact.effect,
            contact.scale,
        ),
        fracture_echo_lifetime(contact.critical),
    );
}

fn hit_bone_overlay_scene(
    assets: &CombatVfxAssets,
    enemy: &Enemy,
    hit_bone: HitBone,
    effect: HitEffect,
    critical: bool,
) -> Handle<WorldAsset> {
    match select_hit_overlay(enemy, hit_bone, effect, critical) {
        HitOverlay::Rune => assets.hit_bone_rune.clone(),
        HitOverlay::Marrow => assets.marrow_flash.clone(),
        HitOverlay::Critical => assets.crit_burst.clone(),
    }
}

fn select_hit_overlay(
    enemy: &Enemy,
    hit_bone: HitBone,
    effect: HitEffect,
    critical: bool,
) -> HitOverlay {
    if matches!(enemy.id.as_str(), "skeleton" | "nemesis")
        && matches!(hit_bone, HitBone::Head | HitBone::Chest)
    {
        HitOverlay::Marrow
    } else if critical || matches!(effect, HitEffect::Headshot | HitEffect::Critical) {
        HitOverlay::Critical
    } else {
        HitOverlay::Rune
    }
}

fn hit_bone_overlay_transform(
    target: Vec3,
    enemy: &Enemy,
    hit_bone: HitBone,
    effect: HitEffect,
    scale: f32,
) -> Transform {
    let offset = hit_bone_offset(hit_bone, enemy);
    let lift = match hit_bone {
        HitBone::Head => Vec3::new(0.0, 0.08, -0.01),
        HitBone::Chest => Vec3::new(0.0, 0.04, 0.0),
        HitBone::Weapon => Vec3::new(0.06, 0.02, -0.02),
        HitBone::Left => Vec3::new(-0.04, 0.02, 0.0),
        HitBone::Right => Vec3::new(0.04, 0.02, 0.0),
    };
    let overlay_scale = scale * hit_bone_scale(hit_bone) * 0.82;
    Transform::from_translation(target + offset + lift)
        .with_rotation(Quat::from_rotation_y(hit_bone_yaw(hit_bone, effect) + 0.22))
        .with_scale(Vec3::splat(overlay_scale))
}

fn hit_bone_overlay_lifetime(hit_bone: HitBone, critical: bool) -> f32 {
    match (hit_bone, critical) {
        (HitBone::Head, true) => 0.44,
        (_, true) => 0.38,
        (HitBone::Weapon, false) => 0.30,
        _ => 0.26,
    }
}

fn hit_bone_lock_transform(
    target: Vec3,
    enemy: &Enemy,
    hit_bone: HitBone,
    effect: HitEffect,
    scale: f32,
) -> Transform {
    let offset = hit_bone_offset(hit_bone, enemy);
    let lock_scale = scale * hit_bone_scale(hit_bone) * 0.64;
    Transform::from_translation(target + offset + Vec3::new(0.0, 0.012, -0.018))
        .with_rotation(Quat::from_rotation_y(hit_bone_yaw(hit_bone, effect) - 0.16))
        .with_scale(Vec3::splat(lock_scale))
}

fn hit_bone_lock_lifetime(hit_bone: HitBone, critical: bool) -> f32 {
    match (hit_bone, critical) {
        (HitBone::Head, true) => 0.34,
        (_, true) => 0.30,
        (HitBone::Weapon, false) => 0.24,
        _ => 0.20,
    }
}

fn should_spawn_fracture_echo(
    enemy: &Enemy,
    hit_bone: HitBone,
    effect: HitEffect,
    critical: bool,
) -> bool {
    critical
        || matches!(hit_bone, HitBone::Head | HitBone::Chest)
        || matches!(
            effect,
            HitEffect::BoneImpact
                | HitEffect::Execution
                | HitEffect::GuardClash
                | HitEffect::ArmorBreak
                | HitEffect::SoulWard
        )
        || enemy.affixes.len() >= 2
}

fn fracture_echo_transform(
    target: Vec3,
    enemy: &Enemy,
    hit_bone: HitBone,
    effect: HitEffect,
    scale: f32,
) -> Transform {
    let offset = hit_bone_offset(hit_bone, enemy);
    let lift = if matches!(hit_bone, HitBone::Head) {
        Vec3::new(0.0, 0.11, -0.025)
    } else {
        Vec3::new(0.0, 0.055, -0.015)
    };
    let echo_scale = scale * hit_bone_scale(hit_bone) * 0.98;
    Transform::from_translation(target + offset + lift)
        .with_rotation(Quat::from_rotation_y(hit_bone_yaw(hit_bone, effect) + 0.38))
        .with_scale(Vec3::splat(echo_scale))
}

fn fracture_echo_lifetime(critical: bool) -> f32 {
    if critical { 0.40 } else { 0.28 }
}

fn hit_effect_transform(
    target: Vec3,
    enemy: &Enemy,
    hit_bone: HitBone,
    effect: HitEffect,
    scale: f32,
) -> Transform {
    let offset = hit_bone_offset(hit_bone, enemy);
    let yaw = hit_bone_yaw(hit_bone, effect);
    let effect_scale = scale * hit_bone_scale(hit_bone) * hit_effect_scale(effect);
    Transform::from_translation(target + offset)
        .with_rotation(Quat::from_rotation_y(yaw))
        .with_scale(Vec3::splat(effect_scale))
}

fn hit_bone_yaw(hit_bone: HitBone, effect: HitEffect) -> f32 {
    let base = match hit_bone {
        HitBone::Head | HitBone::Chest => 0.0,
        HitBone::Weapon | HitBone::Right => -0.45,
        HitBone::Left => 0.45,
    };
    base + match effect {
        HitEffect::GuardClash | HitEffect::ArmorBreak => 0.25,
        HitEffect::SoulWard => -0.30,
        HitEffect::Ember => -0.18,
        HitEffect::Frost => 0.18,
        HitEffect::Void | HitEffect::Shadow => 0.34,
        HitEffect::Frenzy => -0.26,
        HitEffect::Vampiric => 0.31,
        HitEffect::Desecrator => -0.38,
        HitEffect::Headshot => -0.12,
        _ => 0.0,
    }
}

fn hit_effect_scale(effect: HitEffect) -> f32 {
    match effect {
        HitEffect::Headshot | HitEffect::Critical => 1.22,
        HitEffect::Execution | HitEffect::GuardClash | HitEffect::ArmorBreak => 1.12,
        HitEffect::SoulWard => 1.16,
        HitEffect::Ember | HitEffect::Frost | HitEffect::Void => 1.05,
        HitEffect::Frenzy => 1.08,
        HitEffect::Vampiric | HitEffect::Desecrator => 1.14,
        HitEffect::Bone => 0.94,
        _ => 1.0,
    }
}

fn hit_effect_lifetime(effect: HitEffect, critical: bool) -> f32 {
    match effect {
        HitEffect::Headshot | HitEffect::Critical => 0.42,
        HitEffect::Execution | HitEffect::Void => 0.36,
        HitEffect::Frenzy => 0.28,
        HitEffect::Vampiric | HitEffect::Desecrator => 0.36,
        HitEffect::SoulWard => 0.34,
        HitEffect::Ember | HitEffect::Frost | HitEffect::Arcane => 0.32,
        HitEffect::GuardClash | HitEffect::ArmorBreak | HitEffect::BoneImpact => 0.30,
        _ if critical => 0.38,
        _ => 0.24,
    }
}

fn hit_bone_world_position(target: Vec3, enemy: &Enemy, critical: bool) -> Vec3 {
    target + hit_bone_offset(select_hit_bone(enemy, critical), enemy)
}

fn hit_bone_offset(hit_bone: HitBone, enemy: &Enemy) -> Vec3 {
    let scale = enemy_hit_bone_scale(enemy);
    match hit_bone {
        HitBone::Head => Vec3::new(0.0, 1.72 * scale, -0.05),
        HitBone::Chest => Vec3::new(0.0, 1.05 * scale, 0.0),
        HitBone::Weapon => Vec3::new(0.72 * scale, 0.94 * scale, -0.16 * scale),
        HitBone::Left => Vec3::new(-0.46 * scale, 1.02 * scale, 0.0),
        HitBone::Right => Vec3::new(0.46 * scale, 1.02 * scale, 0.0),
    }
}

fn hit_bone_scale(hit_bone: HitBone) -> f32 {
    match hit_bone {
        HitBone::Head => 1.18,
        HitBone::Chest => 1.0,
        HitBone::Weapon => 0.92,
        HitBone::Left | HitBone::Right => 0.86,
    }
}

fn enemy_hit_bone_scale(enemy: &Enemy) -> f32 {
    match enemy.id.as_str() {
        "keeper" => 1.18,
        "cultist" | "treasure_imp" => 0.92,
        "seal_warden" => 1.06,
        "skeleton" | "nemesis" => 0.86,
        _ => 1.0,
    }
}

fn spawn_combat_vfx(
    commands: &mut Commands,
    scene: Handle<WorldAsset>,
    transform: Transform,
    secs: f32,
) {
    commands.spawn((
        WorldAssetRoot(scene),
        transform,
        CombatVfx {
            timer: Timer::from_seconds(secs, TimerMode::Once),
        },
    ));
}

fn apply_arc_damage(
    commands: &mut Commands,
    feedback: &mut AttackFeedback,
    vfx: &CombatVfxAssets,
    attack: ArcAttack,
    enemies: &mut EnemyTargets,
    bestiary: &Bestiary,
) -> AttackOutcome {
    let forward = Vec3::new(attack.facing.x, 0.0, attack.facing.z).normalize_or_zero();
    let mut outcome = AttackOutcome::default();
    let mut storm_chains = Vec::new();
    for (entity, transform, enemy, mut health, ward, chilled, vulnerable, stagger, staggered) in
        enemies.iter_mut()
    {
        if arc_target_hit(attack.origin, forward, transform.translation, attack.range) {
            let critical = roll_critical(attack.crit_chance);
            let damage = bestiary_adjusted_damage(attack.damage, enemy, bestiary);
            let final_damage = apply_enemy_affix_damage(
                apply_critical_damage(damage, critical),
                enemy,
                chilled,
                vulnerable,
                staggered,
            );
            let ward_result = apply_seal_warden_ward(
                commands,
                feedback.combat_events,
                vfx,
                WardHitContext {
                    entity,
                    target: transform.translation,
                    enemy,
                    incoming_damage: final_damage,
                    ward,
                },
            );
            let final_damage = ward_result.health_damage;
            health.current -= final_damage;
            let boss_break = trigger_boss_stagger(
                commands,
                feedback.combat_events,
                vfx,
                enemy,
                BossStaggerTrigger {
                    entity,
                    target: transform.translation,
                    damage: final_damage,
                },
                stagger,
                staggered,
            );
            if boss_break {
                outcome.boss_staggers += 1;
                feedback.hit_stop.write(boss_break_hit_stop_event());
            }
            let impact_profile =
                hit_impact_profile(enemy, critical, attack.impact_scale, boss_break);
            if final_damage > 0.0 {
                feedback.combat_events.write(CombatEvent {
                    text: damage_text("Strike", final_damage, critical),
                });
            }
            feedback.floating_text.write(FloatingCombatTextEvent {
                text: floating_damage_text_for_ward_result(
                    final_damage,
                    ward_result,
                    impact_profile,
                ),
                position: hit_bone_world_position(transform.translation, enemy, critical),
                critical,
            });
            trigger_hit_impact_screen_shake(feedback.screen_shake, impact_profile);
            trigger_hit_impact_hit_stop(feedback.hit_stop, impact_profile);
            spawn_hit_vfx(
                commands,
                vfx,
                transform.translation,
                enemy,
                critical,
                hit_impact_vfx_scale(impact_profile),
            );
            commands.entity(entity).try_insert((
                HitFlash {
                    timer: Timer::from_seconds(0.18, TimerMode::Once),
                },
                hit_react_for_contact(
                    critical,
                    attack.origin,
                    transform.translation,
                    attack.impact_scale,
                ),
                EnemyTargetFocus::new(critical),
            ));
            let proc_trigger = LegendaryProcTrigger::from_attack(critical, &attack);
            if let Some(burning) =
                legendary_burning(attack.legendary_powers, proc_trigger, final_damage)
            {
                commands.entity(entity).try_insert(burning);
                feedback.combat_events.write(CombatEvent {
                    text: "Emberbrand ignited enemy".to_string(),
                });
            }
            if let Some(chilled) = legendary_chilled(attack.legendary_powers, proc_trigger) {
                commands.entity(entity).try_insert(chilled);
                feedback.combat_events.write(CombatEvent {
                    text: "Frostbrand chilled enemy".to_string(),
                });
            }
            if let Some(damage) =
                stormbrand_chain_damage(attack.legendary_powers, proc_trigger, final_damage)
            {
                storm_chains.push(StormChain {
                    source: entity,
                    origin: transform.translation,
                    damage,
                });
            }
            outcome.life_stolen +=
                legendary_life_steal(attack.legendary_powers, proc_trigger, final_damage);
            outcome.barrier_gained +=
                legendary_barrier(attack.legendary_powers, proc_trigger, final_damage);
            outcome.reflected_damage += reflective_damage(enemy, final_damage);
            outcome.hits += 1;
        }
    }
    apply_stormbrand_chains(
        commands,
        feedback.combat_events,
        vfx,
        storm_chains,
        enemies,
        bestiary,
    );
    outcome
}

struct WardHitContext<'a> {
    entity: Entity,
    target: Vec3,
    enemy: &'a Enemy,
    incoming_damage: f32,
    ward: Option<Mut<'a, SealWardenWard>>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct WardAdjustedDamage {
    health_damage: f32,
    ward_damage: f32,
    ward_broken: bool,
}

fn apply_seal_warden_ward(
    commands: &mut Commands,
    combat_events: &mut MessageWriter<CombatEvent>,
    vfx: &CombatVfxAssets,
    context: WardHitContext,
) -> WardAdjustedDamage {
    if context.enemy.id != "seal_warden" {
        return WardAdjustedDamage {
            health_damage: context.incoming_damage,
            ward_damage: 0.0,
            ward_broken: false,
        };
    }
    let Some(mut ward) = context.ward else {
        return WardAdjustedDamage {
            health_damage: context.incoming_damage,
            ward_damage: 0.0,
            ward_broken: false,
        };
    };

    let hit = ward.absorb_damage(context.incoming_damage);
    if hit.ward_damage > 0.0 {
        combat_events.write(CombatEvent {
            text: format!("Seal Warden ward absorbed {:.0}", hit.ward_damage),
        });
    }
    if hit.ward_broken {
        commands.entity(context.entity).try_insert(Vulnerable {
            timer: Timer::from_seconds(SEAL_WARDEN_WARD_VULNERABLE_SECS, TimerMode::Once),
            damage_multiplier: 1.28,
        });
        spawn_stagger_vfx(commands, vfx, context.target, context.enemy);
        combat_events.write(CombatEvent {
            text: "Seal Warden ward shattered: punish Vhal".to_string(),
        });
    }

    WardAdjustedDamage {
        health_damage: hit.health_damage,
        ward_damage: hit.ward_damage,
        ward_broken: hit.ward_broken,
    }
}

fn apply_stormbrand_chains(
    commands: &mut Commands,
    combat_events: &mut MessageWriter<CombatEvent>,
    vfx: &CombatVfxAssets,
    chains: Vec<StormChain>,
    enemies: &mut EnemyTargets,
    bestiary: &Bestiary,
) -> u32 {
    let mut chained_hits = 0;
    for chain in chains {
        if let Some((entity, damage)) =
            first_stormbrand_chain_target(commands, combat_events, vfx, &chain, enemies, bestiary)
        {
            commands.entity(entity).try_insert((
                HitFlash {
                    timer: Timer::from_seconds(0.18, TimerMode::Once),
                },
                hit_react_for_damage(false),
                EnemyTargetFocus::new(false),
            ));
            combat_events.write(CombatEvent {
                text: format!("Stormbrand chained for {damage:.0}"),
            });
            chained_hits += 1;
        }
    }
    chained_hits
}

fn first_stormbrand_chain_target(
    commands: &mut Commands,
    combat_events: &mut MessageWriter<CombatEvent>,
    vfx: &CombatVfxAssets,
    chain: &StormChain,
    enemies: &mut EnemyTargets,
    bestiary: &Bestiary,
) -> Option<(Entity, f32)> {
    for (entity, transform, enemy, mut health, ward, chilled, vulnerable, _, staggered) in
        enemies.iter_mut()
    {
        if entity == chain.source {
            continue;
        }
        if transform.translation.distance(chain.origin) <= stormbrand_chain_radius() {
            let damage = apply_enemy_affix_damage(
                bestiary_adjusted_damage(chain.damage, enemy, bestiary),
                enemy,
                chilled,
                vulnerable,
                staggered,
            );
            let ward_result = apply_seal_warden_ward(
                commands,
                combat_events,
                vfx,
                WardHitContext {
                    entity,
                    target: transform.translation,
                    enemy,
                    incoming_damage: damage,
                    ward,
                },
            );
            health.current -= ward_result.health_damage;
            return Some((
                entity,
                ward_result.health_damage.max(ward_result.ward_damage),
            ));
        }
    }
    None
}

fn apply_enemy_affix_damage(
    damage: f32,
    enemy: &Enemy,
    chilled: Option<&Chilled>,
    vulnerable: Option<&Vulnerable>,
    staggered: Option<&Staggered>,
) -> f32 {
    let shielded_damage = if enemy.affixes.contains(&EnemyAffix::Shielded) && chilled.is_none() {
        damage * shielded_damage_multiplier()
    } else {
        damage
    };
    shielded_damage
        * vulnerable_damage_multiplier(vulnerable)
        * boss_stagger_damage_multiplier(staggered)
}

fn bestiary_adjusted_damage(base_damage: f32, enemy: &Enemy, bestiary: &Bestiary) -> f32 {
    base_damage * bestiary_damage_multiplier(bestiary, &enemy.id)
}

fn shielded_damage_multiplier() -> f32 {
    0.62
}

fn vulnerable_damage_multiplier(vulnerable: Option<&Vulnerable>) -> f32 {
    vulnerable
        .map(|vulnerable| vulnerable.damage_multiplier)
        .unwrap_or(1.0)
        .max(1.0)
}

fn apply_rupture_vulnerable(
    commands: &mut Commands,
    origin: Vec3,
    facing: Vec3,
    range: f32,
    enemies: &mut EnemyTargets,
) -> u32 {
    let forward = Vec3::new(facing.x, 0.0, facing.z).normalize_or_zero();
    let mut applied = 0;
    for (entity, transform, _, _, _, _, _, _, _) in enemies.iter_mut() {
        if arc_target_hit(origin, forward, transform.translation, range) {
            commands.entity(entity).try_insert(Vulnerable {
                timer: Timer::from_seconds(4.0, TimerMode::Once),
                damage_multiplier: 1.22,
            });
            applied += 1;
        }
    }
    applied
}

fn apply_rupture_bleed(
    commands: &mut Commands,
    origin: Vec3,
    facing: Vec3,
    range: f32,
    rupture_damage: f32,
    enemies: &mut EnemyTargets,
    bestiary: &Bestiary,
) -> u32 {
    let forward = Vec3::new(facing.x, 0.0, facing.z).normalize_or_zero();
    let mut applied = 0;
    for (entity, transform, enemy, _, _, _, _, _, _) in enemies.iter_mut() {
        if arc_target_hit(origin, forward, transform.translation, range) {
            commands.entity(entity).try_insert(Burning {
                dps: (bestiary_adjusted_damage(rupture_damage, enemy, bestiary) * 0.20).max(4.0),
                timer: Timer::from_seconds(3.5, TimerMode::Once),
            });
            applied += 1;
        }
    }
    applied
}

fn apply_arc_breakable_damage(
    commands: &mut Commands,
    attack: ArcAttack,
    breakables: &mut BreakableTargets,
) {
    let forward = Vec3::new(attack.facing.x, 0.0, attack.facing.z).normalize_or_zero();
    for (entity, transform, mut health) in breakables.iter_mut() {
        if arc_target_hit(attack.origin, forward, transform.translation, attack.range) {
            health.current -= attack.damage;
            commands.entity(entity).try_insert(HitFlash {
                timer: Timer::from_seconds(0.18, TimerMode::Once),
            });
        }
    }
}

fn apply_nova_breakable_damage(
    commands: &mut Commands,
    origin: Vec3,
    radius: f32,
    damage: f32,
    breakables: &mut BreakableTargets,
) {
    for (entity, transform, mut health) in breakables.iter_mut() {
        if origin.distance(transform.translation) <= radius {
            health.current -= damage;
            commands.entity(entity).try_insert(HitFlash {
                timer: Timer::from_seconds(0.18, TimerMode::Once),
            });
        }
    }
}

fn tick_conduit_lightning(
    time: Res<Time>,
    mut commands: Commands,
    mut combat_events: MessageWriter<CombatEvent>,
    assets: Res<GameAssets>,
    bestiary: Res<Bestiary>,
    mut player: Query<(&Transform, &mut ConduitBuff), With<Player>>,
    mut enemies: EnemyTargets,
) {
    let Ok((player_transform, mut conduit)) = player.single_mut() else {
        return;
    };
    conduit.timer.tick(time.delta());
    if conduit.timer.is_finished() {
        return;
    }
    conduit.pulse.tick(time.delta());
    if !conduit.pulse.just_finished() {
        return;
    }
    let vfx = combat_vfx_assets(&assets);
    let hits = apply_conduit_lightning(
        &mut commands,
        &mut combat_events,
        &vfx,
        player_transform.translation,
        conduit.damage,
        conduit.radius,
        &mut enemies,
        &bestiary,
    );
    if hits > 0 {
        combat_events.write(CombatEvent {
            text: format!("Conduit lightning hit {hits} enemies"),
        });
    }
}

#[allow(clippy::too_many_arguments)]
fn apply_conduit_lightning(
    commands: &mut Commands,
    combat_events: &mut MessageWriter<CombatEvent>,
    vfx: &CombatVfxAssets,
    origin: Vec3,
    damage: f32,
    radius: f32,
    enemies: &mut EnemyTargets,
    bestiary: &Bestiary,
) -> u32 {
    let mut hits = 0;
    for (entity, transform, enemy, mut health, ward, chilled, vulnerable, _, staggered) in
        enemies.iter_mut()
    {
        if transform.translation.distance(origin) > radius {
            continue;
        }
        let final_damage = apply_enemy_affix_damage(
            bestiary_adjusted_damage(damage, enemy, bestiary),
            enemy,
            chilled,
            vulnerable,
            staggered,
        );
        let ward_result = apply_seal_warden_ward(
            commands,
            combat_events,
            vfx,
            WardHitContext {
                entity,
                target: transform.translation,
                enemy,
                incoming_damage: final_damage,
                ward,
            },
        );
        let final_damage = ward_result.health_damage;
        health.current -= final_damage;
        commands.entity(entity).try_insert((
            HitFlash {
                timer: Timer::from_seconds(0.18, TimerMode::Once),
            },
            hit_react_for_contact(false, origin, transform.translation, 0.72),
            EnemyTargetFocus::new(false),
        ));
        if final_damage > 0.0 {
            combat_events.write(CombatEvent {
                text: format!(
                    "Conduit struck {} for {final_damage:.0}",
                    enemy.display_name
                ),
            });
        }
        hits += 1;
    }
    hits
}

fn arc_target_hit(origin: Vec3, forward: Vec3, target: Vec3, range: f32) -> bool {
    let to_target = target - origin;
    let flat = Vec3::new(to_target.x, 0.0, to_target.z);
    let distance = flat.length();
    let alignment = forward.dot(flat.normalize_or_zero());
    distance <= range && alignment > 0.1
}

fn tick_enemy_burning(
    time: Res<Time>,
    mut commands: Commands,
    mut enemies: Query<(Entity, &mut Health, &mut Burning), With<Enemy>>,
) {
    for (entity, mut health, mut burning) in &mut enemies {
        burning.timer.tick(time.delta());
        health.current = (health.current - burning.dps * time.delta_secs()).max(0.0);
        if burning.timer.is_finished() {
            commands.entity(entity).try_remove::<Burning>();
        }
    }
}

fn tick_vulnerable(
    time: Res<Time>,
    mut commands: Commands,
    mut query: Query<(Entity, &mut Vulnerable)>,
) {
    for (entity, mut vulnerable) in &mut query {
        vulnerable.timer.tick(time.delta());
        if vulnerable.timer.is_finished() {
            commands.entity(entity).try_remove::<Vulnerable>();
        }
    }
}

fn tick_hit_flash(
    time: Res<Time>,
    mut commands: Commands,
    mut query: Query<(Entity, &mut HitFlash)>,
) {
    for (entity, mut flash) in &mut query {
        flash.timer.tick(time.delta());
        if flash.timer.is_finished() {
            commands.entity(entity).try_remove::<HitFlash>();
        }
    }
}

fn tick_combat_vfx(
    time: Res<Time>,
    mut commands: Commands,
    mut query: Query<(Entity, &mut CombatVfx)>,
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

fn despawn_combat_vfx(
    mut commands: Commands,
    query: Query<Entity, (With<CombatVfx>, Without<ChildOf>)>,
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
    use crate::data::EnemyAttackKind;

    fn power_set(power: LegendaryPower) -> LegendaryPowerSet {
        LegendaryPowerSet::new(power, LegendaryPower::None)
    }

    fn attuned_power_set(power: LegendaryPower) -> LegendaryPowerSet {
        LegendaryPowerSet::new(LegendaryPower::None, power)
    }

    fn test_enemy(affixes: Vec<EnemyAffix>) -> Enemy {
        Enemy {
            id: "test".to_string(),
            display_name: "Test Enemy".to_string(),
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

    fn test_skill_cooldowns(basic_remaining_secs: f32) -> SkillCooldowns {
        let mut basic = Timer::from_seconds(0.34, TimerMode::Once);
        basic.set_elapsed(std::time::Duration::from_secs_f32(
            (0.34_f32 - basic_remaining_secs).clamp(0.0, 0.34),
        ));
        SkillCooldowns {
            dash: Timer::from_seconds(1.0, TimerMode::Once),
            nova: Timer::from_seconds(1.0, TimerMode::Once),
            rupture: Timer::from_seconds(1.0, TimerMode::Once),
            basic,
            dash_base: 1.0,
            nova_base: 1.0,
            rupture_base: 1.0,
            basic_base: 0.34,
        }
    }

    #[test]
    fn queued_basic_attack_resolves_inside_visible_weapon_swing() {
        let facing = Vec3::new(0.7, 0.0, 0.7).normalize();
        let pending = queued_basic_arc_attack(
            facing,
            2.4,
            18.0,
            0.25,
            power_set(LegendaryPower::Stormbrand),
        );
        let delay = basic_attack_impact_delay_secs();
        let progress = delay / 0.34;

        assert!((0.10..=0.18).contains(&delay));
        assert!((0.08..=0.90).contains(&progress));
        assert_eq!(pending.facing, facing);
        assert_eq!(pending.range, 2.4);
        assert_eq!(pending.damage, 18.0);
        assert_eq!(pending.crit_chance, 0.25);
        assert!(
            pending
                .legendary_powers
                .contains(LegendaryPower::Stormbrand)
        );
    }

    #[test]
    fn basic_attack_repeats_while_left_mouse_is_held_and_ready() {
        let mut mouse = ButtonInput::<MouseButton>::default();
        let ready = test_skill_cooldowns(0.0);
        let cooling_down = test_skill_cooldowns(0.12);
        let intent = BasicAttackIntent::default();
        let target = PrimaryAttackTarget {
            entity: Entity::from_raw_u32(1).unwrap(),
            position: Vec3::X,
            in_range: true,
        };
        let out_of_range = PrimaryAttackTarget {
            in_range: false,
            ..target
        };

        assert!(!basic_attack_requested(
            &mouse,
            &ready,
            &intent,
            Some(target)
        ));

        mouse.press(MouseButton::Left);
        assert!(basic_attack_requested(
            &mouse,
            &ready,
            &intent,
            Some(target)
        ));
        assert!(!basic_attack_requested(
            &mouse,
            &ready,
            &intent,
            Some(out_of_range)
        ));
        assert!(!basic_attack_requested(
            &mouse,
            &cooling_down,
            &intent,
            Some(target)
        ));

        mouse.release(MouseButton::Left);
        assert!(!basic_attack_requested(
            &mouse,
            &ready,
            &intent,
            Some(target)
        ));
    }

    #[test]
    fn basic_attack_intent_chases_single_clicked_enemy_until_in_range() {
        let mut mouse = ButtonInput::<MouseButton>::default();
        let mut intent = BasicAttackIntent::default();
        let target = PrimaryAttackTarget {
            entity: Entity::from_raw_u32(7).unwrap(),
            position: Vec3::new(3.0, 0.0, 0.0),
            in_range: false,
        };

        mouse.press(MouseButton::Left);
        update_basic_attack_intent(&mut intent, &mouse, Some(target.position), Some(target));
        assert_eq!(intent.target, Some(target.entity));
        assert!(!basic_attack_requested(
            &mouse,
            &test_skill_cooldowns(0.0),
            &intent,
            Some(target)
        ));

        mouse.release(MouseButton::Left);
        let reached = PrimaryAttackTarget {
            in_range: true,
            ..target
        };
        assert!(basic_attack_requested(
            &mouse,
            &test_skill_cooldowns(0.0),
            &intent,
            Some(reached)
        ));

        mouse.press(MouseButton::Left);
        update_basic_attack_intent(&mut intent, &mouse, Some(Vec3::new(8.0, 0.0, 0.0)), None);
        assert_eq!(intent.target, None);
        assert!(!basic_attack_requested(
            &mouse,
            &test_skill_cooldowns(0.0),
            &intent,
            None
        ));
    }

    #[test]
    fn cursor_enemy_targeting_uses_flat_pick_radius() {
        let enemy = test_enemy(vec![]);
        let elite = test_enemy(vec![EnemyAffix::Arcane]);
        let mut boss = test_enemy(vec![]);
        boss.id = "keeper".to_string();

        assert!(cursor_targets_enemy(
            Vec3::new(0.70, 7.0, 0.0),
            Vec3::ZERO,
            &enemy
        ));
        assert!(!cursor_targets_enemy(
            Vec3::new(0.90, 0.0, 0.0),
            Vec3::ZERO,
            &enemy
        ));
        assert!(cursor_targets_enemy(
            Vec3::new(1.0, 0.0, 0.0),
            Vec3::ZERO,
            &elite
        ));
        assert!(cursor_targets_enemy(
            Vec3::new(1.45, 0.0, 0.0),
            Vec3::ZERO,
            &boss
        ));
    }

    #[test]
    fn primary_attack_waits_until_target_is_in_range() {
        let enemy = test_enemy(vec![]);
        let player = Vec3::ZERO;
        let close = Vec3::new(2.18, 0.0, 0.0);
        let far = Vec3::new(2.95, 0.0, 0.0);

        assert!(attack_target_in_range(player, close, 2.2, &enemy));
        assert!(!attack_target_in_range(player, far, 2.2, &enemy));
    }

    #[test]
    fn attack_facing_prefers_cursor_ground_point() {
        let facing = attack_facing_from_cursor(
            Vec3::new(1.0, 0.0, 1.0),
            Vec3::Z,
            Some(Vec3::new(4.0, 0.0, 5.0)),
        );

        assert!((facing.length() - 1.0).abs() < 0.001);
        assert!(facing.x > 0.59);
        assert!(facing.z > 0.79);
    }

    #[test]
    fn attack_facing_keeps_current_forward_when_cursor_is_missing_or_too_close() {
        let current = Vec3::new(-2.0, 8.0, 0.0);
        let missing = attack_facing_from_cursor(Vec3::ZERO, current, None);
        let too_close =
            attack_facing_from_cursor(Vec3::ZERO, current, Some(Vec3::new(0.01, 0.0, 0.01)));
        let zero_fallback = attack_facing_from_cursor(Vec3::ZERO, Vec3::ZERO, None);

        assert_eq!(missing, Vec3::NEG_X);
        assert_eq!(too_close, Vec3::NEG_X);
        assert_eq!(zero_fallback, Vec3::Z);
    }

    #[test]
    fn skill_unavailable_text_reports_rounded_cooldown_seconds() {
        assert_eq!(skill_unavailable_text("Dash", 1.2), "Dash not ready 2s");
        assert_eq!(
            skill_unavailable_text("Rupture", 0.1),
            "Rupture not ready 1s"
        );
        assert_eq!(skill_unavailable_text("Nova", 0.0), "Nova not ready 1s");
    }

    #[test]
    fn skill_input_buffer_preserves_recent_skill_presses_briefly() {
        let mut buffer = SkillInputBuffer::default();

        buffer.capture(true, false, true);
        assert!(buffer.dash_requested());
        assert!(!buffer.rupture_requested());
        assert!(buffer.nova_requested());

        buffer.tick(SKILL_INPUT_BUFFER_SECS * 0.5);
        assert!(buffer.dash_requested());
        assert!(buffer.nova_requested());

        buffer.clear_dash();
        assert!(!buffer.dash_requested());
        assert!(buffer.nova_requested());

        buffer.tick(SKILL_INPUT_BUFFER_SECS);
        assert!(!buffer.nova_requested());
    }

    #[test]
    fn skill_input_buffer_refreshes_and_clears_each_skill_independently() {
        let mut buffer = SkillInputBuffer::default();

        buffer.capture(false, true, false);
        buffer.tick(SKILL_INPUT_BUFFER_SECS * 0.75);
        assert!(buffer.rupture_requested());

        buffer.capture(false, true, false);
        buffer.tick(SKILL_INPUT_BUFFER_SECS * 0.75);
        assert!(buffer.rupture_requested());

        buffer.clear_rupture();
        assert!(!buffer.rupture_requested());

        buffer.capture(true, false, false);
        buffer.tick(SKILL_INPUT_BUFFER_SECS + 0.01);
        assert!(!buffer.dash_requested());
    }

    #[test]
    fn rune_damage_multipliers_match_tradeoffs() {
        assert_eq!(dash_damage_multiplier(DashRune::Cleanse), 1.0);
        assert!(dash_damage_multiplier(DashRune::Reap) < 1.0);
        assert_eq!(nova_damage_multiplier(NovaRune::Ember), 1.0);
        assert!(nova_damage_multiplier(NovaRune::Frost) < 1.0);
        assert_eq!(rupture_damage_multiplier(RuptureRune::Expose), 1.0);
        assert!(rupture_damage_multiplier(RuptureRune::Hemorrhage) < 1.0);
    }

    #[test]
    fn hit_bone_selection_targets_readable_contact_points() {
        let normal = test_enemy(vec![]);
        let guarded = test_enemy(vec![EnemyAffix::Shielded]);
        let mut cultist = test_enemy(vec![]);
        cultist.id = "cultist".to_string();
        let mut warden = test_enemy(vec![]);
        warden.id = "seal_warden".to_string();
        let mut keeper = test_enemy(vec![]);
        keeper.id = "keeper".to_string();

        assert_eq!(select_hit_bone(&normal, false), HitBone::Left);
        assert_eq!(select_hit_bone(&guarded, false), HitBone::Weapon);
        assert_eq!(select_hit_bone(&cultist, false), HitBone::Right);
        assert_eq!(select_hit_bone(&warden, false), HitBone::Chest);
        assert_eq!(select_hit_bone(&keeper, false), HitBone::Chest);
        assert_eq!(select_hit_bone(&normal, true), HitBone::Head);
    }

    #[test]
    fn hit_bone_offsets_scale_by_enemy_silhouette() {
        let mut skeleton = test_enemy(vec![]);
        skeleton.id = "skeleton".to_string();
        let mut keeper = test_enemy(vec![]);
        keeper.id = "keeper".to_string();

        assert!(
            hit_bone_offset(HitBone::Head, &keeper).y > hit_bone_offset(HitBone::Head, &skeleton).y
        );
        assert!(hit_bone_scale(HitBone::Head) > hit_bone_scale(HitBone::Left));
    }

    #[test]
    fn hit_effect_selection_uses_bone_and_affix_language() {
        let mut skeleton = test_enemy(vec![]);
        skeleton.id = "skeleton".to_string();
        let mut keeper = test_enemy(vec![]);
        keeper.id = "keeper".to_string();

        assert_eq!(
            select_hit_effect(&skeleton, HitBone::Head, false),
            HitEffect::BoneImpact
        );
        assert_eq!(
            select_hit_effect(&skeleton, HitBone::Left, false),
            HitEffect::Bone
        );
        assert_eq!(
            select_hit_effect(&test_enemy(vec![EnemyAffix::Arcane]), HitBone::Left, false),
            HitEffect::Arcane
        );
        assert_eq!(
            select_hit_effect(
                &test_enemy(vec![EnemyAffix::Shielded]),
                HitBone::Weapon,
                false
            ),
            HitEffect::ArmorBreak
        );
        assert_eq!(
            select_hit_effect(
                &test_enemy(vec![EnemyAffix::Reflective]),
                HitBone::Weapon,
                false
            ),
            HitEffect::SoulWard
        );
        assert_eq!(
            select_hit_effect(
                &test_enemy(vec![EnemyAffix::Shielded, EnemyAffix::Reflective]),
                HitBone::Weapon,
                false
            ),
            HitEffect::GuardClash
        );
        assert_eq!(
            select_hit_effect(&test_enemy(vec![EnemyAffix::Frozen]), HitBone::Left, false),
            HitEffect::Frost
        );
        assert_eq!(
            select_hit_effect(&test_enemy(vec![EnemyAffix::Jailer]), HitBone::Left, false),
            HitEffect::Holy
        );
        assert_eq!(
            select_hit_effect(&test_enemy(vec![EnemyAffix::Molten]), HitBone::Left, false),
            HitEffect::Ember
        );
        assert_eq!(
            select_hit_effect(
                &test_enemy(vec![EnemyAffix::Frenzied]),
                HitBone::Left,
                false
            ),
            HitEffect::Frenzy
        );
        assert_eq!(
            select_hit_effect(
                &test_enemy(vec![EnemyAffix::Vampiric]),
                HitBone::Left,
                false
            ),
            HitEffect::Vampiric
        );
        assert_eq!(
            select_hit_effect(
                &test_enemy(vec![EnemyAffix::Desecrator]),
                HitBone::Left,
                false
            ),
            HitEffect::Desecrator
        );
        assert_eq!(
            select_hit_effect(
                &test_enemy(vec![EnemyAffix::Vampiric, EnemyAffix::Desecrator]),
                HitBone::Left,
                false
            ),
            HitEffect::Void
        );
        let mut cultist = test_enemy(vec![]);
        cultist.id = "cultist".to_string();
        let mut warden = test_enemy(vec![]);
        warden.id = "seal_warden".to_string();
        assert_eq!(
            select_hit_effect(&cultist, HitBone::Right, false),
            HitEffect::Shadow
        );
        assert_eq!(
            select_hit_effect(&warden, HitBone::Chest, false),
            HitEffect::Shadow
        );
        assert_eq!(
            select_hit_effect(&keeper, HitBone::Chest, false),
            HitEffect::Execution
        );
        assert_eq!(
            select_hit_effect(&keeper, HitBone::Weapon, false),
            HitEffect::Blood
        );
        assert_eq!(
            select_hit_effect(&keeper, HitBone::Chest, true),
            HitEffect::Critical
        );
        assert_eq!(
            select_hit_effect(&keeper, HitBone::Head, true),
            HitEffect::Headshot
        );
    }

    #[test]
    fn floating_damage_uses_selected_hit_bone_position() {
        let mut cultist = test_enemy(vec![]);
        cultist.id = "cultist".to_string();
        let mut warden = test_enemy(vec![]);
        warden.id = "seal_warden".to_string();
        let target = Vec3::new(2.0, 0.0, 3.0);

        assert_eq!(
            hit_bone_world_position(target, &cultist, false),
            target + hit_bone_offset(HitBone::Right, &cultist)
        );
        assert_eq!(
            hit_bone_world_position(target, &warden, false),
            target + hit_bone_offset(HitBone::Chest, &warden)
        );
        assert_eq!(
            hit_bone_world_position(target, &cultist, true),
            target + hit_bone_offset(HitBone::Head, &cultist)
        );
    }

    #[test]
    fn hit_effect_transform_scales_and_orients_contact_vfx() {
        let guarded = test_enemy(vec![EnemyAffix::Shielded]);
        let ember = test_enemy(vec![EnemyAffix::Molten]);
        let target = Vec3::new(1.0, 0.0, -2.0);

        let guard_transform = hit_effect_transform(
            target,
            &guarded,
            HitBone::Weapon,
            HitEffect::ArmorBreak,
            1.0,
        );
        let ember_transform =
            hit_effect_transform(target, &ember, HitBone::Chest, HitEffect::Ember, 1.0);
        let weapon_spark =
            hit_effect_transform(target, &guarded, HitBone::Weapon, HitEffect::Spark, 1.0);

        assert_eq!(
            guard_transform.translation,
            target + hit_bone_offset(HitBone::Weapon, &guarded)
        );
        assert!(guard_transform.scale.x > weapon_spark.scale.x);
        assert!(ember_transform.scale.x > weapon_spark.scale.x);
        assert_ne!(guard_transform.rotation, ember_transform.rotation);
        assert!(
            hit_effect_lifetime(HitEffect::Ember, false)
                > hit_effect_lifetime(HitEffect::Spark, false)
        );
        assert!(
            hit_effect_lifetime(HitEffect::Headshot, true)
                > hit_effect_lifetime(HitEffect::ArmorBreak, false)
        );
    }

    #[test]
    fn hit_bone_overlays_lift_and_scale_to_contact_points() {
        let guarded = test_enemy(vec![EnemyAffix::Shielded]);
        let target = Vec3::new(1.0, 0.0, -2.0);

        let overlay = hit_bone_overlay_transform(
            target,
            &guarded,
            HitBone::Weapon,
            HitEffect::ArmorBreak,
            1.0,
        );

        assert!(overlay.translation.y > hit_bone_offset(HitBone::Weapon, &guarded).y);
        assert!(
            overlay.scale.x
                < hit_effect_transform(
                    target,
                    &guarded,
                    HitBone::Weapon,
                    HitEffect::ArmorBreak,
                    1.0
                )
                .scale
                .x
        );
        assert!(
            hit_bone_overlay_lifetime(HitBone::Head, true)
                > hit_bone_overlay_lifetime(HitBone::Chest, false)
        );
    }

    #[test]
    fn hit_bone_lock_tracks_contact_without_overpowering_main_effect() {
        let guarded = test_enemy(vec![EnemyAffix::Shielded]);
        let target = Vec3::new(1.0, 0.0, -2.0);

        let lock = hit_bone_lock_transform(
            target,
            &guarded,
            HitBone::Weapon,
            HitEffect::ArmorBreak,
            1.0,
        );
        let main = hit_effect_transform(
            target,
            &guarded,
            HitBone::Weapon,
            HitEffect::ArmorBreak,
            1.0,
        );

        assert_eq!(
            lock.translation,
            target + hit_bone_offset(HitBone::Weapon, &guarded) + Vec3::new(0.0, 0.012, -0.018)
        );
        assert!(lock.scale.x < main.scale.x);
        assert!(
            hit_bone_lock_lifetime(HitBone::Head, true)
                > hit_bone_lock_lifetime(HitBone::Left, false)
        );
    }

    #[test]
    fn fracture_echo_only_spawns_for_readable_heavy_contacts() {
        let normal = test_enemy(vec![]);
        let elite = test_enemy(vec![EnemyAffix::Shielded, EnemyAffix::Reflective]);
        let target = Vec3::new(1.0, 0.0, -2.0);

        assert!(!should_spawn_fracture_echo(
            &normal,
            HitBone::Left,
            HitEffect::Spark,
            false
        ));
        assert!(should_spawn_fracture_echo(
            &normal,
            HitBone::Chest,
            HitEffect::Spark,
            false
        ));
        assert!(should_spawn_fracture_echo(
            &elite,
            HitBone::Weapon,
            HitEffect::GuardClash,
            false
        ));
        assert!(should_spawn_fracture_echo(
            &normal,
            HitBone::Head,
            HitEffect::Headshot,
            true
        ));

        let head_echo =
            fracture_echo_transform(target, &normal, HitBone::Head, HitEffect::Headshot, 1.0);
        let chest_echo =
            fracture_echo_transform(target, &normal, HitBone::Chest, HitEffect::Spark, 1.0);
        assert!(head_echo.translation.y > chest_echo.translation.y);
        assert!(fracture_echo_lifetime(true) > fracture_echo_lifetime(false));
    }

    #[test]
    fn bone_enemies_use_marrow_overlay_on_vital_hit_bones() {
        let mut skeleton = test_enemy(vec![]);
        skeleton.id = "skeleton".to_string();
        let normal = test_enemy(vec![]);

        assert_eq!(
            select_hit_overlay(&skeleton, HitBone::Head, HitEffect::BoneImpact, false),
            HitOverlay::Marrow
        );
        assert_eq!(
            select_hit_overlay(&normal, HitBone::Chest, HitEffect::Spark, false),
            HitOverlay::Rune
        );
        assert_eq!(
            select_hit_overlay(&normal, HitBone::Head, HitEffect::Headshot, true),
            HitOverlay::Critical
        );
    }

    #[test]
    fn rupture_runes_report_distinct_effects() {
        assert_eq!(
            rupture_effect_text(RuptureRune::Expose, 2),
            "Rupture exposed 2 enemies"
        );
        assert_eq!(
            rupture_effect_text(RuptureRune::Hemorrhage, 2),
            "Rupture bleeding 2 enemies"
        );
    }

    #[test]
    fn skill_cast_feedback_names_runes_and_scales_impact() {
        assert_eq!(
            skill_cast_text(SkillCast::Dash(DashRune::Reap)),
            "Reap Dash"
        );
        assert_eq!(
            skill_cast_text(SkillCast::Rupture(RuptureRune::Hemorrhage)),
            "Hemorrhage Rupture"
        );
        assert_eq!(
            skill_cast_text(SkillCast::Nova(NovaRune::Frost)),
            "Frost Nova"
        );

        let dash = skill_cast_screen_shake(SkillCast::Dash(DashRune::Cleanse));
        let rupture = skill_cast_screen_shake(SkillCast::Rupture(RuptureRune::Expose));
        let nova = skill_cast_screen_shake(SkillCast::Nova(NovaRune::Ember));
        assert!(dash.intensity < rupture.intensity);
        assert!(rupture.intensity < nova.intensity);
        assert!(dash.duration_secs < nova.duration_secs);
    }

    #[test]
    fn nova_cast_vfx_scales_with_radius_and_lingers_for_frost() {
        let small = nova_cast_scale(2.0);
        let large = nova_cast_scale(7.0);
        assert!(large > small);
        assert_eq!(nova_cast_scale(0.0), 1.10);
        assert_eq!(nova_cast_scale(99.0), 2.85);
        assert!(nova_cast_lifetime(NovaRune::Frost) > nova_cast_lifetime(NovaRune::Ember));

        let transform = nova_cast_transform(Vec3::new(1.0, 0.0, -2.0), 5.5);
        assert_eq!(transform.translation, Vec3::new(1.0, 0.18, -2.0));
        assert!(transform.scale.x > 1.9);

        let ring = nova_outer_ring_transform(Vec3::new(1.0, 0.0, -2.0), 5.5);
        assert_eq!(ring.translation, Vec3::new(1.0, 0.10, -2.0));
        assert!(ring.scale.x > transform.scale.x);
        assert!(nova_outer_ring_lifetime(NovaRune::Frost) > nova_cast_lifetime(NovaRune::Frost));
    }

    #[test]
    fn dash_cast_vfx_marks_path_and_rune_arrival() {
        let start = Vec3::new(0.0, 0.0, 0.0);
        let end = Vec3::new(0.0, 0.0, 4.0);
        let facing = Vec3::Z;

        let early = dash_afterimage_transform(start, end, facing, 0.24, 0.74);
        let late = dash_afterimage_transform(start, end, facing, 0.80, 1.14);
        assert!(late.translation.z > early.translation.z + 2.0);
        assert!(late.scale.z > early.scale.z);
        assert!(late.scale.z > late.scale.x);

        let cleanse = dash_arrival_transform(end, facing, DashRune::Cleanse);
        let reap = dash_arrival_transform(end, facing, DashRune::Reap);
        assert_eq!(cleanse.translation, reap.translation);
        assert!(reap.scale.x > cleanse.scale.x);
        assert!(dash_arrival_lifetime(DashRune::Reap) > dash_arrival_lifetime(DashRune::Cleanse));
    }

    #[test]
    fn rupture_cast_vfx_draws_wide_cone_and_tip_impact() {
        let origin = Vec3::new(1.0, 0.0, 2.0);
        let facing = Vec3::Z;
        let range = 3.2;

        let left = rupture_cone_transform(origin, facing, range, -1.0);
        let center = rupture_cone_transform(origin, facing, range, 0.0);
        let right = rupture_cone_transform(origin, facing, range, 1.0);
        assert!(left.translation.x < center.translation.x);
        assert!(right.translation.x > center.translation.x);
        assert_eq!(left.translation.z, right.translation.z);
        assert!(left.scale.x > center.scale.x);
        assert!(center.scale.z > 1.7);

        let expose = rupture_tip_transform(origin, facing, range, RuptureRune::Expose);
        let bleed = rupture_tip_transform(origin, facing, range, RuptureRune::Hemorrhage);
        assert!(expose.translation.z > center.translation.z);
        assert_eq!(expose.translation, bleed.translation);
        assert!(bleed.scale.x > expose.scale.x);
        assert!(
            rupture_tip_lifetime(RuptureRune::Hemorrhage)
                > rupture_tip_lifetime(RuptureRune::Expose)
        );
    }

    #[test]
    fn critical_hits_clamp_chance_and_scale_damage() {
        assert!(is_critical_hit(0.2, 2.0));
        assert!(!is_critical_hit(0.0, -1.0));
        assert_eq!(
            apply_critical_damage(40.0, true),
            40.0 * critical_damage_multiplier()
        );
        assert_eq!(apply_critical_damage(40.0, false), 40.0);
    }

    #[test]
    fn damage_text_marks_critical_hits() {
        assert_eq!(damage_text("Strike", 20.0, false), "Strike hit for 20");
        assert_eq!(
            damage_text("Strike", 35.0, true),
            "Critical Strike hit for 35"
        );
        assert_eq!(floating_damage_text(20.0, false), "20");
        assert_eq!(floating_damage_text(35.0, true), "CRIT 35");
    }

    #[test]
    fn hit_impact_profiles_promote_elites_bosses_and_breaks() {
        let normal = test_enemy(vec![]);
        let elite = test_enemy(vec![EnemyAffix::Shielded]);
        let mut nemesis = test_enemy(vec![]);
        nemesis.id = "nemesis".to_string();
        let mut boss = test_enemy(vec![]);
        boss.id = "keeper".to_string();

        assert_eq!(
            hit_impact_profile(&normal, false, 1.0, false).tier,
            HitImpactTier::Normal
        );
        assert_eq!(
            hit_impact_profile(&elite, false, 1.0, false).tier,
            HitImpactTier::Elite
        );
        assert_eq!(
            hit_impact_profile(&nemesis, false, 1.0, false).tier,
            HitImpactTier::Elite
        );
        assert_eq!(
            hit_impact_profile(&boss, true, 1.0, false),
            HitImpactProfile {
                critical: true,
                impact_scale: 1.0,
                tier: HitImpactTier::Boss,
            }
        );
        assert_eq!(
            hit_impact_profile(&boss, false, 1.0, true).tier,
            HitImpactTier::BossBreak
        );
    }

    #[test]
    fn floating_damage_text_calls_out_elites_bosses_and_breaks() {
        let normal = HitImpactProfile {
            critical: false,
            impact_scale: 1.0,
            tier: HitImpactTier::Normal,
        };
        let elite_crit = HitImpactProfile {
            critical: true,
            impact_scale: 1.0,
            tier: HitImpactTier::Elite,
        };
        let boss = HitImpactProfile {
            critical: false,
            impact_scale: 1.0,
            tier: HitImpactTier::Boss,
        };
        let boss_break = HitImpactProfile {
            critical: true,
            impact_scale: 1.0,
            tier: HitImpactTier::BossBreak,
        };

        assert_eq!(floating_damage_text_for_profile(20.0, normal), "20");
        assert_eq!(
            floating_damage_text_for_profile(35.0, elite_crit),
            "ELITE CRIT 35"
        );
        assert_eq!(floating_damage_text_for_profile(48.0, boss), "BOSS 48");
        assert_eq!(
            floating_damage_text_for_profile(91.0, boss_break),
            "BREAK 91"
        );
    }

    #[test]
    fn hit_impact_vfx_scale_makes_elites_bosses_and_breaks_visually_heavier() {
        let normal = HitImpactProfile {
            critical: false,
            impact_scale: 1.0,
            tier: HitImpactTier::Normal,
        };
        let elite = HitImpactProfile {
            critical: false,
            impact_scale: 1.0,
            tier: HitImpactTier::Elite,
        };
        let boss_crit = HitImpactProfile {
            critical: true,
            impact_scale: 1.0,
            tier: HitImpactTier::Boss,
        };
        let boss_break = HitImpactProfile {
            critical: false,
            impact_scale: 1.8,
            tier: HitImpactTier::BossBreak,
        };

        assert_eq!(hit_impact_vfx_scale(normal), 1.0);
        assert!(hit_impact_vfx_scale(elite) > hit_impact_vfx_scale(normal));
        assert!(hit_impact_vfx_scale(boss_crit) > hit_impact_vfx_scale(elite));
        assert_eq!(hit_impact_vfx_scale(boss_break), 2.35);
    }

    #[test]
    fn hit_impact_screen_shake_scales_by_skill_weight_and_critical() {
        let basic = hit_impact_screen_shake_event(false, 1.0);
        let rupture = hit_impact_screen_shake_event(false, 1.28);
        let nova = hit_impact_screen_shake_event(false, 1.35);
        let critical = hit_impact_screen_shake_event(true, 1.0);
        let elite = hit_impact_screen_shake_event_for_profile(HitImpactProfile {
            critical: false,
            impact_scale: 1.0,
            tier: HitImpactTier::Elite,
        });
        let boss = hit_impact_screen_shake_event_for_profile(HitImpactProfile {
            critical: false,
            impact_scale: 1.0,
            tier: HitImpactTier::Boss,
        });
        let boss_break = hit_impact_screen_shake_event_for_profile(HitImpactProfile {
            critical: false,
            impact_scale: 1.0,
            tier: HitImpactTier::BossBreak,
        });

        assert!(basic.intensity > 0.0);
        assert!(rupture.intensity > basic.intensity);
        assert!(nova.intensity > rupture.intensity);
        assert!(critical.intensity > basic.intensity * 2.0);
        assert!(rupture.duration_secs > basic.duration_secs);
        assert!(elite.intensity > basic.intensity);
        assert!(boss.intensity > elite.intensity);
        assert!(boss_break.intensity > boss.intensity);
    }

    #[test]
    fn hit_stop_scales_by_skill_weight_critical_and_boss_break() {
        let basic = hit_impact_hit_stop_event(false, 1.0);
        let rupture = hit_impact_hit_stop_event(false, 1.28);
        let nova = hit_impact_hit_stop_event(false, 1.35);
        let critical = hit_impact_hit_stop_event(true, 1.0);
        let boss_break = boss_break_hit_stop_event();
        let elite_profile = hit_impact_hit_stop_event_for_profile(HitImpactProfile {
            critical: false,
            impact_scale: 1.0,
            tier: HitImpactTier::Elite,
        });
        let boss_profile = hit_impact_hit_stop_event_for_profile(HitImpactProfile {
            critical: false,
            impact_scale: 1.0,
            tier: HitImpactTier::Boss,
        });

        assert!(basic.duration_secs > 0.018);
        assert!(rupture.duration_secs > basic.duration_secs);
        assert!(nova.duration_secs > rupture.duration_secs);
        assert!(critical.duration_secs > basic.duration_secs * 2.0);
        assert!(critical.relative_speed < basic.relative_speed);
        assert!(elite_profile.duration_secs > basic.duration_secs);
        assert!(boss_profile.duration_secs > elite_profile.duration_secs);
        assert!(boss_profile.relative_speed < elite_profile.relative_speed);
        assert!(boss_break.duration_secs > critical.duration_secs);
        assert!(boss_break.relative_speed < critical.relative_speed);
    }

    #[test]
    fn boss_break_reward_text_reports_resource_window() {
        assert_eq!(boss_break_fury_reward(), 18.0);
        assert_eq!(boss_break_barrier_reward(), 22.0);
        assert_eq!(
            boss_break_reward_text(2, 36.0, 44.0),
            "Boss Break x2: EXECUTE WINDOW - Surge now, +36 fury +44 barrier"
        );
    }

    #[test]
    fn shielded_affix_reduces_damage_until_chilled() {
        let shielded = test_enemy(vec![EnemyAffix::Shielded]);
        let chilled = Chilled {
            timer: Timer::from_seconds(2.0, TimerMode::Once),
            slow_multiplier: 0.5,
        };

        assert_eq!(
            apply_enemy_affix_damage(100.0, &shielded, None, None, None),
            100.0 * shielded_damage_multiplier()
        );
        assert_eq!(
            apply_enemy_affix_damage(100.0, &shielded, Some(&chilled), None, None),
            100.0
        );
        assert_eq!(
            apply_enemy_affix_damage(100.0, &test_enemy(vec![]), None, None, None),
            100.0
        );
    }

    #[test]
    fn vulnerable_multiplier_increases_post_rupture_damage() {
        let vulnerable = Vulnerable {
            timer: Timer::from_seconds(4.0, TimerMode::Once),
            damage_multiplier: 1.22,
        };

        assert_eq!(vulnerable_damage_multiplier(Some(&vulnerable)), 1.22);
        assert_eq!(
            apply_enemy_affix_damage(100.0, &test_enemy(vec![]), None, Some(&vulnerable), None),
            122.0
        );
        assert_eq!(
            apply_enemy_affix_damage(
                100.0,
                &test_enemy(vec![EnemyAffix::Shielded]),
                None,
                Some(&vulnerable),
                None
            ),
            100.0 * shielded_damage_multiplier() * 1.22
        );
    }

    #[test]
    fn staggered_boss_takes_bonus_damage() {
        let staggered = Staggered {
            timer: Timer::from_seconds(2.0, TimerMode::Once),
            damage_multiplier: 1.18,
        };

        let damage =
            apply_enemy_affix_damage(100.0, &test_enemy(vec![]), None, None, Some(&staggered));
        assert!((damage - 118.0).abs() < 0.001);
    }

    #[test]
    fn combo_state_tracks_short_skill_chain_windows() {
        let mut combo = CombatComboState::default();
        assert_eq!(combo.active_primer(), None);

        combo.prime(ComboPrimer::Rupture);
        assert_eq!(combo.active_primer(), Some(ComboPrimer::Rupture));
        assert_eq!(combo.hud_hint(), Some("Rupture 3s -> Q/E".to_string()));
        assert!(combo_finisher_allowed(
            ComboPrimer::Rupture,
            ComboFinisher::Dash
        ));
        assert!(combo_finisher_allowed(
            ComboPrimer::Rupture,
            ComboFinisher::Nova
        ));
        assert!(!combo_finisher_allowed(
            ComboPrimer::Nova,
            ComboFinisher::Nova
        ));

        combo
            .timer
            .tick(std::time::Duration::from_secs_f32(COMBO_WINDOW_SECS + 0.1));
        assert_eq!(combo.active_primer(), None);
        assert_eq!(combo.hud_hint(), None);

        combo.prime(ComboPrimer::Nova);
        assert_eq!(combo.hud_hint(), Some("Nova 3s -> Q".to_string()));
    }

    #[test]
    fn combo_break_text_and_rewards_scale_with_hits() {
        assert_eq!(combo_fury_reward(1), 11.0);
        assert_eq!(combo_fury_reward(9), 23.0);
        assert!(combo_screen_shake(4) > combo_screen_shake(1));

        let label = combo_bonus_label(Some("Blood nova"), Some("Storm surge")).unwrap();
        let text = combo_break_text(
            ComboPrimer::Rupture,
            ComboFinisher::Nova,
            3,
            17.0,
            14.0,
            0.0,
            Some(&label),
        );
        assert!(text.contains("Combo Break"));
        assert!(text.contains("Rupture into Nova"));
        assert!(text.contains("3 targets"));
        assert!(text.contains("+17 fury"));
        assert!(text.contains("+14 health"));
        assert!(text.contains("Blood nova | Storm surge"));
    }

    #[test]
    fn combo_chain_bonus_adds_route_specific_finisher_payoffs() {
        let cull = combo_chain_bonus(ComboPrimer::Rupture, ComboFinisher::Dash, 1);
        let blood = combo_chain_bonus(ComboPrimer::Rupture, ComboFinisher::Nova, 4);
        let frostbreak = combo_chain_bonus(ComboPrimer::Nova, ComboFinisher::Dash, 4);
        let invalid = combo_chain_bonus(ComboPrimer::Nova, ComboFinisher::Nova, 4);

        assert_eq!(cull.barrier, 8.0);
        assert_eq!(cull.label, Some("Cull dash"));
        assert_eq!(blood.fury, 8.0);
        assert_eq!(blood.healing, 17.0);
        assert_eq!(blood.label, Some("Blood nova"));
        assert_eq!(frostbreak.fury, 10.0);
        assert_eq!(frostbreak.barrier, 22.0);
        assert_eq!(frostbreak.label, Some("Frostbreak dash"));
        assert_eq!(invalid.fury, 0.0);
        assert_eq!(invalid.healing, 0.0);
        assert_eq!(invalid.barrier, 0.0);
    }

    #[test]
    fn themed_reliquary_sets_add_distinct_combo_break_payoffs() {
        let storm = combo_set_bonus(Some(ReliquarySet::Storm), 4);
        let blood = combo_set_bonus(Some(ReliquarySet::Blood), 4);
        let ashen = combo_set_bonus(Some(ReliquarySet::Ashen), 4);
        let reliquary = combo_set_bonus(Some(ReliquarySet::Reliquary), 4);

        assert!(storm.shake > ComboSetBonus::default().shake);
        assert_eq!(storm.label, Some("Storm surge"));
        assert!(blood.healing > 0.0);
        assert_eq!(blood.label, Some("Blood pact"));
        assert!(ashen.barrier > 0.0);
        assert_eq!(ashen.label, Some("Ashen ward"));
        assert!(reliquary.fury > 0.0);
        assert_eq!(reliquary.label, Some("Reliquary overflow"));

        let mut health = Health {
            current: 60.0,
            max: 100.0,
        };
        let mut barrier = Barrier {
            current: 5.0,
            max: 35.0,
        };
        assert_eq!(combo_heal(&mut health, blood.healing), blood.healing);
        assert_eq!(barrier.grant(ashen.barrier), ashen.barrier);

        let text = combo_break_text(
            ComboPrimer::Nova,
            ComboFinisher::Dash,
            4,
            combo_fury_reward(4) + reliquary.fury,
            blood.healing,
            ashen.barrier,
            reliquary.label,
        );
        assert!(text.contains("+22 health"));
        assert!(text.contains("+28 barrier"));
        assert!(text.contains("Reliquary overflow"));
    }

    #[test]
    fn bestiary_progress_increases_damage_against_known_enemies() {
        let enemy = test_enemy(vec![]);
        let mut bestiary = Bestiary::default();
        for _ in 0..8 {
            bestiary.record_kill(&enemy.id, &enemy.display_name);
        }

        assert!((bestiary_adjusted_damage(100.0, &enemy, &bestiary) - 108.0).abs() < 0.001);
        assert_eq!(
            bestiary_adjusted_damage(
                100.0,
                &test_enemy(vec![EnemyAffix::Molten]),
                &Bestiary::default()
            ),
            100.0
        );
    }

    #[test]
    fn legendary_power_skill_synergy_maps_each_power_to_matching_runes() {
        assert!(legendary_power_skill_synergy(
            LegendaryPower::Emberbrand,
            Some(SkillCast::Nova(NovaRune::Ember))
        ));
        assert!(legendary_power_skill_synergy(
            LegendaryPower::Emberbrand,
            Some(SkillCast::Rupture(RuptureRune::Hemorrhage))
        ));
        assert!(legendary_power_skill_synergy(
            LegendaryPower::Frostbrand,
            Some(SkillCast::Nova(NovaRune::Frost))
        ));
        assert!(legendary_power_skill_synergy(
            LegendaryPower::Stormbrand,
            Some(SkillCast::Dash(DashRune::Reap))
        ));
        assert!(legendary_power_skill_synergy(
            LegendaryPower::Soulreaver,
            Some(SkillCast::Rupture(RuptureRune::Hemorrhage))
        ));
        assert!(legendary_power_skill_synergy(
            LegendaryPower::Aegisbrand,
            Some(SkillCast::Dash(DashRune::Cleanse))
        ));
        assert!(!legendary_power_skill_synergy(
            LegendaryPower::Frostbrand,
            Some(SkillCast::Nova(NovaRune::Ember))
        ));
        assert!(!legendary_power_skill_synergy(
            LegendaryPower::Stormbrand,
            None
        ));
    }

    #[test]
    fn emberbrand_ignites_on_critical_or_matching_ember_runes() {
        assert!(
            legendary_burning(
                power_set(LegendaryPower::None),
                LegendaryProcTrigger::basic(true),
                40.0
            )
            .is_none()
        );
        assert!(
            legendary_burning(
                power_set(LegendaryPower::Emberbrand),
                LegendaryProcTrigger::basic(false),
                40.0
            )
            .is_none()
        );

        let burning = legendary_burning(
            power_set(LegendaryPower::Emberbrand),
            LegendaryProcTrigger::basic(true),
            40.0,
        )
        .unwrap();
        assert_eq!(burning.dps, 8.8);
        assert!(!burning.timer.is_finished());

        let rune_burning = legendary_burning(
            power_set(LegendaryPower::Emberbrand),
            LegendaryProcTrigger::skill(false, SkillCast::Nova(NovaRune::Ember)),
            40.0,
        )
        .unwrap();
        assert!((rune_burning.dps - 6.4).abs() < 0.001);
        assert!(
            rune_burning.timer.duration().as_secs_f32() > burning.timer.duration().as_secs_f32()
        );

        let combined = legendary_burning(
            attuned_power_set(LegendaryPower::Emberbrand),
            LegendaryProcTrigger::skill(true, SkillCast::Rupture(RuptureRune::Hemorrhage)),
            40.0,
        )
        .unwrap();
        assert!((combined.dps - 11.2).abs() < 0.001);
    }

    #[test]
    fn frostbrand_chills_on_critical_or_frost_control_runes() {
        assert!(
            legendary_chilled(
                power_set(LegendaryPower::None),
                LegendaryProcTrigger::basic(true)
            )
            .is_none()
        );
        assert!(
            legendary_chilled(
                power_set(LegendaryPower::Frostbrand),
                LegendaryProcTrigger::basic(false)
            )
            .is_none()
        );

        let chilled = legendary_chilled(
            power_set(LegendaryPower::Frostbrand),
            LegendaryProcTrigger::basic(true),
        )
        .unwrap();
        assert_eq!(chilled.slow_multiplier, 0.52);
        assert!(!chilled.timer.is_finished());

        let rune_chilled = legendary_chilled(
            power_set(LegendaryPower::Frostbrand),
            LegendaryProcTrigger::skill(false, SkillCast::Nova(NovaRune::Frost)),
        )
        .unwrap();
        assert_eq!(rune_chilled.slow_multiplier, 0.48);
        assert!(
            rune_chilled.timer.duration().as_secs_f32() > chilled.timer.duration().as_secs_f32()
        );

        assert!(
            legendary_chilled(
                attuned_power_set(LegendaryPower::Frostbrand),
                LegendaryProcTrigger::skill(true, SkillCast::Rupture(RuptureRune::Expose))
            )
            .is_some()
        );
    }

    #[test]
    fn stormbrand_chains_on_critical_or_reap_control_runes() {
        assert!(
            stormbrand_chain_damage(
                power_set(LegendaryPower::None),
                LegendaryProcTrigger::basic(true),
                40.0
            )
            .is_none()
        );
        assert!(
            stormbrand_chain_damage(
                power_set(LegendaryPower::Stormbrand),
                LegendaryProcTrigger::basic(false),
                40.0
            )
            .is_none()
        );

        assert_eq!(
            stormbrand_chain_damage(
                power_set(LegendaryPower::Stormbrand),
                LegendaryProcTrigger::basic(true),
                40.0
            ),
            Some(16.8)
        );
        assert_eq!(
            stormbrand_chain_damage(
                power_set(LegendaryPower::Stormbrand),
                LegendaryProcTrigger::basic(true),
                5.0
            ),
            Some(6.0)
        );
        assert_eq!(
            stormbrand_chain_damage(
                power_set(LegendaryPower::Stormbrand),
                LegendaryProcTrigger::skill(false, SkillCast::Dash(DashRune::Reap)),
                40.0
            ),
            Some(12.0)
        );
        assert_eq!(
            stormbrand_chain_damage(
                attuned_power_set(LegendaryPower::Stormbrand),
                LegendaryProcTrigger::skill(true, SkillCast::Nova(NovaRune::Frost)),
                40.0
            ),
            Some(20.0)
        );
        assert_eq!(stormbrand_chain_radius(), 4.2);
    }

    #[test]
    fn soulreaver_steals_life_on_critical_or_reap_bleed_runes() {
        assert_eq!(
            legendary_life_steal(
                power_set(LegendaryPower::None),
                LegendaryProcTrigger::basic(true),
                40.0
            ),
            0.0
        );
        assert_eq!(
            legendary_life_steal(
                power_set(LegendaryPower::Soulreaver),
                LegendaryProcTrigger::basic(false),
                40.0
            ),
            0.0
        );
        assert!(
            (legendary_life_steal(
                power_set(LegendaryPower::Soulreaver),
                LegendaryProcTrigger::basic(true),
                40.0
            ) - 7.2)
                .abs()
                < 0.001
        );
        assert_eq!(
            legendary_life_steal(
                power_set(LegendaryPower::Soulreaver),
                LegendaryProcTrigger::basic(true),
                5.0
            ),
            3.0
        );
        assert!(
            (legendary_life_steal(
                power_set(LegendaryPower::Soulreaver),
                LegendaryProcTrigger::skill(false, SkillCast::Rupture(RuptureRune::Hemorrhage)),
                40.0
            ) - 4.8)
                .abs()
                < 0.001
        );
        assert!(
            (legendary_life_steal(
                attuned_power_set(LegendaryPower::Soulreaver),
                LegendaryProcTrigger::skill(true, SkillCast::Dash(DashRune::Reap)),
                40.0
            ) - 9.2)
                .abs()
                < 0.001
        );
    }

    #[test]
    fn aegisbrand_grants_barrier_on_critical_or_guarded_runes() {
        assert_eq!(
            legendary_barrier(
                power_set(LegendaryPower::None),
                LegendaryProcTrigger::basic(true),
                40.0
            ),
            0.0
        );
        assert_eq!(
            legendary_barrier(
                power_set(LegendaryPower::Aegisbrand),
                LegendaryProcTrigger::basic(false),
                40.0
            ),
            0.0
        );
        assert_eq!(
            legendary_barrier(
                power_set(LegendaryPower::Aegisbrand),
                LegendaryProcTrigger::basic(true),
                40.0
            ),
            8.0
        );
        assert_eq!(
            legendary_barrier(
                power_set(LegendaryPower::Aegisbrand),
                LegendaryProcTrigger::basic(true),
                5.0
            ),
            5.0
        );
        assert_eq!(
            legendary_barrier(
                power_set(LegendaryPower::Aegisbrand),
                LegendaryProcTrigger::skill(false, SkillCast::Rupture(RuptureRune::Expose)),
                40.0
            ),
            5.6
        );
        assert_eq!(
            legendary_barrier(
                attuned_power_set(LegendaryPower::Aegisbrand),
                LegendaryProcTrigger::skill(true, SkillCast::Dash(DashRune::Cleanse)),
                40.0
            ),
            10.4
        );
    }

    #[test]
    fn aegisbrand_barrier_gain_respects_capacity() {
        let mut barrier = Barrier {
            current: 18.0,
            max: 20.0,
        };

        assert_eq!(
            barrier.grant(legendary_barrier(
                power_set(LegendaryPower::Aegisbrand),
                LegendaryProcTrigger::basic(true),
                40.0
            )),
            2.0
        );
        assert_eq!(barrier.current, 20.0);
    }

    #[test]
    fn reflective_affix_returns_bounded_direct_damage() {
        assert_eq!(reflective_damage(&test_enemy(vec![]), 100.0), 0.0);
        assert_eq!(
            reflective_damage(&test_enemy(vec![EnemyAffix::Reflective]), 10.0),
            2.0
        );
        assert_eq!(
            reflective_damage(&test_enemy(vec![EnemyAffix::Reflective]), 100.0),
            8.0
        );
        assert_eq!(
            reflective_damage(&test_enemy(vec![EnemyAffix::Reflective]), 400.0),
            18.0
        );
    }

    #[test]
    fn arc_target_hit_uses_flat_forward_alignment() {
        assert!(arc_target_hit(
            Vec3::ZERO,
            Vec3::Z,
            Vec3::new(0.2, 4.0, 1.0),
            1.2
        ));
        assert!(!arc_target_hit(
            Vec3::ZERO,
            Vec3::Z,
            Vec3::new(0.0, 0.0, -1.0),
            1.2
        ));
        assert!(!arc_target_hit(
            Vec3::ZERO,
            Vec3::Z,
            Vec3::new(0.0, 0.0, 2.0),
            1.2
        ));
    }
}
