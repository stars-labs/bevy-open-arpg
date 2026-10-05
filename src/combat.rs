mod presentation;
mod ranged;
#[cfg(all(feature = "dev_tools", not(target_arch = "wasm32")))]
mod review;
mod skill_vfx;
mod skill_vfx_art;

use crate::{
    GameState, RunStats,
    assets::GameAssets,
    bestiary::{Bestiary, bestiary_damage_multiplier},
    class::{
        ClassCatalog, Movement, Passive, REAP_HEAL_PER_HIT, ResourceKind, Rune, SelectedClass,
        SkillLoadout, SkillSlot, StrikeShape,
    },
    data::PlayerTuning,
    dungeon::{Breakable, DungeonLayout, clamp_dungeon_translation},
    enemy::{
        BossStagger, Chilled, Enemy, EnemyAffix, EnemyHitReact, EnemyTargetFocus,
        SEAL_WARDEN_WARD_VULNERABLE_SECS, SealWardenWard, Staggered, add_boss_stagger,
        boss_stagger_damage_multiplier, boss_staggered, enemy_cursor_pick_radius,
    },
    feedback::{CombatEvent, FloatingCombatTextEvent, HitStopEvent, ScreenShakeEvent},
    not_paused,
    player::{
        Barrier, Burning, Charm, ConduitBuff, DamageBonus, ElixirBuff, Equipment, Evade, Fury,
        GloryBuff, Health, LegendaryCodex, LegendaryPower, Player, PlayerAnimation,
        PlayerAttackMotion, PlayerLevel, RelicBuff, ReliquarySet, SkillCooldowns, SurgeBuff,
        TalentBranch, Talents, apply_player_damage_with_evade, attack_contact_fraction,
        damage_multiplier, elixir_damage_multiplier, glory_damage_multiplier, mitigated_damage,
        surge_damage_multiplier, surge_fury_multiplier, themed_reliquary_set, total_armor,
        total_crit_chance, total_damage_bonus,
    },
};
use bevy::ecs::system::SystemParam;
use bevy::prelude::*;
use bevy::window::PrimaryWindow;
use presentation::{ContactFeedback, TalentProcFeedback};
use rand::RngExt;

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
        base_distance * impact_scale.clamp(0.0, 1.85),
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
    /// Seconds each slot's key press stays buffered, indexed by `SkillSlot::index`.
    secs: [f32; 3],
}

impl SkillInputBuffer {
    fn tick(&mut self, delta_secs: f32) {
        for secs in &mut self.secs {
            *secs = (*secs - delta_secs).max(0.0);
        }
    }

    fn capture(&mut self, slot: SkillSlot) {
        self.secs[slot.index()] = SKILL_INPUT_BUFFER_SECS;
    }

    fn requested(&self, slot: SkillSlot) -> bool {
        self.secs[slot.index()] > 0.0
    }

    fn clear(&mut self, slot: SkillSlot) {
        self.secs[slot.index()] = 0.0;
    }
}

#[derive(Resource, Debug)]
pub struct CombatComboState {
    /// The slot whose skill last marked enemies for a follow-up.
    primer: Option<SkillSlot>,
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
    fn prime(&mut self, primer: SkillSlot) {
        self.primer = Some(primer);
        self.timer = Timer::from_seconds(COMBO_WINDOW_SECS, TimerMode::Once);
        self.timer.reset();
    }

    fn clear(&mut self) {
        self.primer = None;
        self.timer = Timer::from_seconds(0.0, TimerMode::Once);
    }

    fn active_primer(&self) -> Option<SkillSlot> {
        if self.timer.is_finished() {
            None
        } else {
            self.primer
        }
    }
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

/// Where a blow lands.
#[derive(Debug, Clone, Copy, PartialEq)]
enum HitArea {
    Arc {
        origin: Vec3,
        forward: Vec3,
        range: f32,
    },
    Radial {
        center: Vec3,
        radius: f32,
    },
    /// The one enemy or breakable a missile touched.
    Target(Entity),
}

impl HitArea {
    fn contains(self, entity: Entity, position: Vec3) -> bool {
        match self {
            Self::Arc {
                origin,
                forward,
                range,
            } => arc_target_hit(origin, forward, position, range),
            Self::Radial { center, radius } => center.distance(position) <= radius,
            Self::Target(target) => target == entity,
        }
    }

    /// The point struck enemies recoil away from.
    fn recoil_origin(self, source: Vec3) -> Vec3 {
        match self {
            Self::Arc { origin, .. } => origin,
            Self::Radial { center, .. } => center,
            Self::Target(_) => source,
        }
    }
}

/// Everything about a blow that is fixed the moment it is thrown.
#[derive(Debug, Clone, Copy, PartialEq)]
struct StrikeProfile {
    damage: f32,
    impact_scale: f32,
    crit_chance: f32,
    legendary_powers: LegendaryPowerSet,
    skill: Option<SkillCast>,
    /// Blows at arm's length freeze the swing; missiles and bursts must not stall the game.
    hit_stop: bool,
    /// Long Shot passive: (minimum distance, damage multiplier).
    long_shot: Option<(f32, f32)>,
    /// Ledger id shared by every missile of one cast, so once-per-cast rewards pay once.
    cast: u32,
    /// Focus: bonus resource for a basic attack that connects.
    fury_bonus: f32,
    /// Vigor: barrier for a skill that connects.
    barrier_reward: f32,
}

struct Strike {
    area: HitArea,
    /// Where the hero stood when the blow was thrown.
    source: Vec3,
    profile: StrikeProfile,
}

/// A strike waiting for its animation's contact (or release) frame.
#[derive(Component, Debug)]
struct PendingPlayerStrike {
    timer: Timer,
    facing: Vec3,
    /// Cursor point captured at the key press, for ground-targeted shapes.
    target: Vec3,
    shape: StrikeShape,
    motion: PlayerAttackMotion,
    profile: StrikeProfile,
}

/// Pays rewards that belong to a cast rather than to each enemy or missile it hits.
#[derive(Component, Debug, Default)]
pub(crate) struct CastLedger {
    next: u32,
    barrier_paid: u32,
    focus_paid: u32,
}

impl CastLedger {
    fn begin(&mut self) -> u32 {
        self.next = self.next.wrapping_add(1).max(1);
        self.next
    }

    /// True only the first time `cast` asks for this reward.
    fn claim(paid: &mut u32, cast: u32) -> bool {
        if *paid == cast {
            false
        } else {
            *paid = cast;
            true
        }
    }
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

    #[cfg(test)]
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

/// The slot a skill was cast from and the rune it carried.
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
struct SkillCast {
    slot: SkillSlot,
    rune: Rune,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
struct LegendaryProcTrigger {
    critical: bool,
    skill: Option<SkillCast>,
}

impl LegendaryProcTrigger {
    #[cfg(test)]
    fn basic(critical: bool) -> Self {
        Self {
            critical,
            skill: None,
        }
    }

    #[cfg(test)]
    fn skill(critical: bool, skill: SkillCast) -> Self {
        Self {
            critical,
            skill: Some(skill),
        }
    }

    fn from_cast(critical: bool, skill: Option<SkillCast>) -> Self {
        Self { critical, skill }
    }

    fn has_skill_synergy(self, power: LegendaryPower) -> bool {
        legendary_power_rune_synergy(power, self.skill.map(|cast| cast.rune))
    }

    fn should_proc(self, power: LegendaryPower) -> bool {
        self.critical || self.has_skill_synergy(power)
    }
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

#[derive(Default)]
struct CombatVfxAssets {
    slash_arc: Handle<WorldAsset>,
    holy_impact: Handle<WorldAsset>,
    ember_impact: Handle<WorldAsset>,
    frost_impact: Handle<WorldAsset>,
    shadow_burst: Handle<WorldAsset>,
    stagger_burst: Handle<WorldAsset>,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
enum HitBone {
    Head,
    Chest,
    Weapon,
    Left,
    Right,
}

#[derive(Debug, Default, Clone, Copy, PartialEq)]
struct AttackOutcome {
    hits: u32,
    damaging_hits: u32,
    /// Enemies that received the rune's status (exposed, bleeding or chilled).
    rune_applied: u32,
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
        &'static mut CastLedger,
        Option<&'static PendingPlayerStrike>,
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
        &'static SkillLoadout,
        &'static Talents,
        &'static LegendaryCodex,
        &'static PlayerLevel,
    ),
    With<Player>,
>;
/// The hero as seen by strikes that land after the cast (contact frames, missiles, bursts).
type StrikeHeroQuery<'w, 's> = Query<
    'w,
    's,
    (
        Entity,
        &'static Transform,
        &'static mut Health,
        &'static mut Barrier,
        &'static mut Fury,
        &'static mut CastLedger,
        &'static mut PlayerAnimation,
        &'static Equipment,
        &'static Charm,
        &'static ElixirBuff,
        &'static Evade,
        &'static SurgeBuff,
        Option<&'static mut PendingPlayerStrike>,
    ),
    With<Player>,
>;

/// Who collects what a landed strike pays back.
struct Hero<'a> {
    position: Vec3,
    health: &'a mut Health,
    barrier: &'a mut Barrier,
    fury: &'a mut Fury,
    ledger: &'a mut CastLedger,
    equipment: &'a Equipment,
    charm: &'a Charm,
    elixir: &'a ElixirBuff,
    evade: &'a Evade,
    surge: &'a SurgeBuff,
}

/// Runs `reward` with the hero, for strikes that resolve away from the cast system.
fn with_hero(heroes: &mut StrikeHeroQuery, reward: impl FnOnce(Hero)) {
    let Ok((
        _,
        transform,
        mut health,
        mut barrier,
        mut fury,
        mut ledger,
        _,
        equipment,
        charm,
        elixir,
        evade,
        surge,
        _,
    )) = heroes.single_mut()
    else {
        return;
    };
    reward(Hero {
        position: transform.translation,
        health: &mut health,
        barrier: &mut barrier,
        fury: &mut fury,
        ledger: &mut ledger,
        equipment,
        charm,
        elixir,
        evade,
        surge,
    });
}

#[derive(SystemParam)]
struct CombatInputs<'w, 's> {
    time: Res<'w, Time>,
    keyboard: Res<'w, ButtonInput<KeyCode>>,
    mouse: Res<'w, ButtonInput<MouseButton>>,
    ui_capture: Res<'w, crate::ui::UiPointerCapture>,
    windows: Query<'w, 's, &'static Window, With<PrimaryWindow>>,
    cameras: Query<'w, 's, (&'static Camera, &'static GlobalTransform), With<Camera3d>>,
    tuning: Res<'w, PlayerTuning>,
    catalog: Res<'w, ClassCatalog>,
    selected: Res<'w, SelectedClass>,
    dungeon_layout: Res<'w, DungeonLayout>,
    input_buffer: ResMut<'w, SkillInputBuffer>,
    basic_attack_intent: ResMut<'w, BasicAttackIntent>,
}

/// Everything a strike touches besides the hero: targets, feedback and run bookkeeping.
#[derive(SystemParam)]
struct StrikeWorld<'w, 's> {
    commands: Commands<'w, 's>,
    assets: Res<'w, GameAssets>,
    bestiary: Res<'w, Bestiary>,
    stats: ResMut<'w, RunStats>,
    combo: ResMut<'w, CombatComboState>,
    combat_events: MessageWriter<'w, CombatEvent>,
    floating_text: MessageWriter<'w, FloatingCombatTextEvent>,
    screen_shake: MessageWriter<'w, ScreenShakeEvent>,
    hit_stop: MessageWriter<'w, HitStopEvent>,
    enemies: EnemyTargets<'w, 's>,
    breakables: BreakableTargets<'w, 's>,
}

/// One strike leaving the hero: its shape decides whether it lands now or later.
struct Release {
    origin: Vec3,
    facing: Vec3,
    target: Vec3,
    shape: StrikeShape,
    profile: StrikeProfile,
}

impl StrikeWorld<'_, '_> {
    /// Hit now (arcs, bursts around the hero) or launch what will hit later.
    /// Returns the outcome of whatever landed immediately.
    fn release(&mut self, hero: Hero, resource: ResourceKind, release: Release) -> AttackOutcome {
        let Release {
            origin,
            facing,
            target,
            shape,
            profile,
        } = release;
        // Present confirmed releases, including empty casts, at their true footprint.
        if let Some(cast) = profile.skill
            && !matches!(shape, StrikeShape::GroundBlast { .. })
        {
            self.commands.write_message(skill_vfx::SkillVisual {
                origin,
                facing,
                shape,
                cast,
                strength: profile.impact_scale,
            });
        }
        match shape {
            StrikeShape::Arc { range } => self.land(
                hero,
                resource,
                HitArea::Arc {
                    origin,
                    forward: facing,
                    range,
                },
                origin,
                profile,
            ),
            StrikeShape::Radial { radius } => self.land(
                hero,
                resource,
                HitArea::Radial {
                    center: origin,
                    radius,
                },
                origin,
                profile,
            ),
            StrikeShape::Projectiles { .. } => {
                ranged::launch_projectiles(
                    &mut self.commands,
                    &self.assets,
                    origin,
                    facing,
                    shape,
                    profile,
                );
                AttackOutcome::default()
            }
            StrikeShape::GroundBlast { radius, delay, .. } => {
                ranged::mark_ground_blast(
                    &mut self.commands,
                    origin,
                    target,
                    radius,
                    delay,
                    profile,
                );
                AttackOutcome::default()
            }
        }
    }

    /// Strike an area, break what is breakable inside it and pay the hero's rewards.
    fn land(
        &mut self,
        hero: Hero,
        resource: ResourceKind,
        area: HitArea,
        source: Vec3,
        profile: StrikeProfile,
    ) -> AttackOutcome {
        let vfx = combat_vfx_assets(&self.assets);
        let outcome = apply_strike(
            &mut self.commands,
            &mut AttackFeedback {
                combat_events: &mut self.combat_events,
                floating_text: &mut self.floating_text,
                screen_shake: &mut self.screen_shake,
                hit_stop: &mut self.hit_stop,
            },
            &vfx,
            Strike {
                area,
                source,
                profile,
            },
            &mut self.enemies,
            &self.bestiary,
        );
        damage_breakables(area, profile.damage, &mut self.breakables);
        self.settle(hero, resource, profile, outcome);
        outcome
    }

    /// Resource, healing, barrier, combo and run-stat consequences of a landed strike.
    fn settle(
        &mut self,
        hero: Hero,
        resource: ResourceKind,
        profile: StrikeProfile,
        outcome: AttackOutcome,
    ) {
        let Hero {
            position,
            health,
            barrier,
            fury,
            ledger,
            equipment,
            charm,
            elixir,
            evade,
            surge,
        } = hero;
        match profile.skill {
            None => {
                if outcome.damaging_hits > 0 {
                    let base_gain = fury.basic_gain
                        * outcome.damaging_hits as f32
                        * surge_fury_multiplier(surge);
                    let gained = fury.gain(base_gain);
                    let bonus = if CastLedger::claim(&mut ledger.focus_paid, profile.cast) {
                        fury.gain(profile.fury_bonus)
                    } else {
                        0.0
                    };
                    if bonus > 0.0 {
                        self.commands.write_message(TalentProcFeedback {
                            position,
                            branch: TalentBranch::Focus,
                            amount: bonus,
                        });
                    }
                    if gained + bonus > 0.0 {
                        self.combat_events.write(CombatEvent {
                            text: format!("Generated {:.0} {}", gained + bonus, resource.label()),
                        });
                    }
                }
            }
            Some(cast) => {
                match cast.rune {
                    Rune::Reap if outcome.hits > 0 => {
                        self.stats.reap_hits = self.stats.reap_hits.saturating_add(outcome.hits);
                        let restored = REAP_HEAL_PER_HIT * outcome.hits as f32;
                        health.current = (health.current + restored).min(health.max);
                        self.combat_events.write(CombatEvent {
                            text: format!("Reap restored {restored:.0} health"),
                        });
                    }
                    Rune::Frost if outcome.hits > 0 => {
                        self.stats.frost_hits = self.stats.frost_hits.saturating_add(outcome.hits);
                    }
                    Rune::Hemorrhage => {
                        self.stats.hemorrhage_hits = self
                            .stats
                            .hemorrhage_hits
                            .saturating_add(outcome.rune_applied);
                    }
                    _ => {}
                }
                if outcome.damaging_hits > 0
                    && CastLedger::claim(&mut ledger.barrier_paid, profile.cast)
                {
                    grant_vigor_barrier(
                        &mut self.commands,
                        barrier,
                        position,
                        profile.barrier_reward,
                    );
                }
                apply_combo_finisher(
                    &mut self.combo,
                    ComboFinisherPlayer {
                        fury,
                        health,
                        barrier,
                        equipment,
                        charm,
                    },
                    cast.slot,
                    outcome.hits,
                    resource,
                    ComboFinisherFeedback {
                        combat_events: &mut self.combat_events,
                        screen_shake: &mut self.screen_shake,
                    },
                );
                if outcome.hits > 0 && cast.slot != SkillSlot::Mobility {
                    self.combo.prime(cast.slot);
                    if cast.slot == SkillSlot::Area {
                        self.combat_events.write(CombatEvent {
                            text: "Combo Ready: rush the marked enemies".to_string(),
                        });
                    }
                }
                if let Some(text) = rune_effect_text(cast.rune, outcome.rune_applied) {
                    self.combat_events.write(CombatEvent { text });
                }
            }
        }
        apply_boss_break_reward(
            fury,
            barrier,
            outcome.boss_staggers,
            &mut self.stats,
            &mut self.combat_events,
        );
        apply_life_steal(health, outcome.life_stolen, &mut self.combat_events);
        apply_legendary_barrier(barrier, outcome.barrier_gained, &mut self.combat_events);
        apply_reflected_damage(
            health,
            barrier,
            outcome.reflected_damage,
            total_armor(equipment, elixir),
            evade,
            &mut self.combat_events,
        );
    }
}

pub struct CombatPlugin;

#[derive(SystemSet, Debug, Clone, Copy, Eq, PartialEq, Hash)]
pub enum CombatSet {
    PlayerDamage,
}

impl Plugin for CombatPlugin {
    fn build(&self, app: &mut App) {
        #[cfg(all(feature = "dev_tools", not(target_arch = "wasm32")))]
        review::install(app);
        app.add_plugins((
            presentation::CombatPresentationPlugin,
            skill_vfx::SkillVfxPlugin,
        ))
        .init_resource::<CombatComboState>()
        .init_resource::<SkillInputBuffer>()
        .init_resource::<BasicAttackIntent>()
        .add_systems(
            OnEnter(GameState::InGame),
            (reset_combat_combo, reset_basic_attack_intent),
        )
        .add_systems(
            Update,
            (
                (
                    player_attacks,
                    resolve_pending_player_strikes,
                    ranged::fly_player_projectiles,
                    ranged::burst_ground_blasts,
                    tick_conduit_lightning,
                    tick_enemy_burning,
                )
                    .chain()
                    .in_set(CombatSet::PlayerDamage),
                tick_combat_vfx,
                tick_vulnerable,
                tick_combat_combo,
            )
                .run_if(in_state(GameState::InGame).and_then(not_paused)),
        )
        .add_systems(
            OnExit(GameState::InGame),
            (despawn_combat_vfx, ranged::despawn_ranged_strikes),
        );
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
    mut world: StrikeWorld,
    mut player_core: CombatPlayerCoreQuery,
    player_stats: CombatPlayerStatsQuery,
) {
    let Ok((
        player_entity,
        mut player_transform,
        mut player_health,
        mut player_barrier,
        mut fury,
        mut cooldowns,
        mut animation,
        mut ledger,
        pending_strike,
    )) = player_core.single_mut()
    else {
        return;
    };
    let Ok((
        bonus,
        charm,
        equipment,
        buff,
        glory,
        elixir,
        surge,
        evade,
        loadout,
        talents,
        codex,
        level,
    )) = player_stats.single()
    else {
        return;
    };
    let class = inputs.catalog.get(inputs.selected.0);
    let resource = class.resource.kind;
    let growth = &inputs.tuning.growth;
    let total_multiplier = damage_multiplier(buff)
        * elixir_damage_multiplier(elixir)
        * glory_damage_multiplier(glory)
        * surge_damage_multiplier(surge)
        * talents.damage_multiplier()
        * unbowed_multiplier(class.passive, &player_health);
    let damage_bonus = total_damage_bonus(bonus, equipment, charm);
    let crit_chance = (total_crit_chance(equipment, charm) + talents.crit_bonus()).min(0.60);
    let legendary_powers = LegendaryPowerSet::new(equipment.legendary_power, codex.attuned);
    let long_shot = long_shot_passive(class.passive);
    let basic_reach = class.basic.strike.reach();
    // A click on a UI panel is panel input, not a strike command.
    let cursor_ground = if inputs.ui_capture.0 {
        None
    } else {
        cursor_ground_point(&inputs.windows, &inputs.cameras)
    };
    let cursor_primary_attack_target = cursor_ground_primary_attack_target(
        cursor_ground,
        player_transform.translation,
        basic_reach,
        &world.enemies,
    );
    if !inputs.ui_capture.0 {
        update_basic_attack_intent(
            &mut inputs.basic_attack_intent,
            &inputs.mouse,
            cursor_ground,
            cursor_primary_attack_target,
        );
    }
    let primary_attack_target = intended_primary_attack_target(
        &mut inputs.basic_attack_intent,
        cursor_primary_attack_target,
        player_transform.translation,
        basic_reach,
        &world.enemies,
    );
    let mut pressed = [false; 3];
    for slot in SkillSlot::ALL {
        pressed[slot.index()] = inputs.keyboard.just_pressed(slot.key())
            && (slot != SkillSlot::Finisher || !inputs.ui_capture.0);
    }
    inputs.input_buffer.tick(inputs.time.delta_secs());
    for slot in SkillSlot::ALL {
        if pressed[slot.index()] {
            inputs.input_buffer.capture(slot);
        }
    }
    if pending_strike.is_some() {
        return;
    }

    if !inputs.ui_capture.0
        && cursor_ground.is_some()
        && animation.attack_progress() >= 1.0
        && basic_attack_requested(
            &inputs.mouse,
            &cooldowns,
            &inputs.basic_attack_intent,
            primary_attack_target,
        )
    {
        let aim = primary_attack_target
            .map(|target| target.position)
            .or(cursor_ground);
        let facing = attack_facing_from_cursor(
            player_transform.translation,
            player_transform.forward().as_vec3(),
            aim,
        );
        animation.face_direction(facing);
        let cycle = cooldowns.basic_base * talents.cooldown_multiplier();
        animation.trigger_attack(class.basic.motion, cycle);
        let profile = StrikeProfile {
            damage: (class.basic.damage + damage_bonus) * total_multiplier,
            impact_scale: talents.combat_impact_multiplier(growth),
            crit_chance,
            legendary_powers,
            skill: None,
            hit_stop: stalls_on_hit(class.basic.strike),
            long_shot,
            cast: ledger.begin(),
            fury_bonus: talents.basic_fury_bonus(growth),
            barrier_reward: 0.0,
        };
        world
            .commands
            .entity(player_entity)
            .try_insert(queued_strike(
                cycle,
                facing,
                aim.unwrap_or(player_transform.translation + facing),
                class.basic.strike,
                class.basic.motion,
                profile,
            ));
        let basic_base = cooldowns.basic_base;
        reset_scaled(
            &mut cooldowns.basic,
            basic_base,
            talents.cooldown_multiplier(),
        );
    }

    for slot in [SkillSlot::Mobility, SkillSlot::Finisher, SkillSlot::Area] {
        if !inputs.input_buffer.requested(slot) {
            continue;
        }
        let pressed = pressed[slot.index()];
        let (skill, rune) = class.equipped(loadout, slot);
        if level.level < skill.unlock_level {
            inputs.input_buffer.clear(slot);
            if pressed {
                world.combat_events.write(CombatEvent {
                    text: format!("{} unlocks at level {}", skill.name, skill.unlock_level),
                });
            }
            continue;
        }
        if !cooldowns.slot(slot).is_finished() {
            if pressed {
                world.combat_events.write(CombatEvent {
                    text: skill_unavailable_text(
                        &skill.name,
                        cooldowns.slot(slot).remaining_secs(),
                    ),
                });
            }
            continue;
        }
        let facing = attack_facing_from_cursor(
            player_transform.translation,
            player_transform.forward().as_vec3(),
            cursor_ground,
        );
        if skill.requires_target
            && !strike_has_target(
                player_transform.translation,
                facing,
                skill.strike,
                world
                    .enemies
                    .iter()
                    .map(|(_, transform, _, health, ..)| (transform.translation, health.current))
                    .chain(
                        world
                            .breakables
                            .iter()
                            .map(|(_, transform, health)| (transform.translation, health.current)),
                    ),
            )
        {
            inputs.input_buffer.clear(slot);
            if pressed {
                world.combat_events.write(CombatEvent {
                    text: format!("{}: move closer and face an enemy", skill.name),
                });
            }
            return;
        }
        animation.face_direction(facing);
        let cost = fury.skill_cost(skill.cost) * talents.fury_cost_multiplier();
        if !fury.spend(cost) {
            inputs.input_buffer.clear(slot);
            if pressed {
                world.combat_events.write(CombatEvent {
                    text: format!(
                        "Need {cost:.0} {} for {}",
                        resource.label(),
                        skill.name.to_lowercase()
                    ),
                });
            }
            return;
        }
        inputs.input_buffer.clear(slot);
        let cast = SkillCast { slot, rune };
        world.combat_events.write(CombatEvent {
            text: skill_cast_text(&skill.name, rune),
        });
        world.screen_shake.write(skill_cast_screen_shake(slot));
        let cast_secs = if skill.hasted {
            skill.cast_secs * talents.cooldown_multiplier()
        } else {
            skill.cast_secs
        };
        animation.trigger_attack(skill.motion, cast_secs);

        let start = player_transform.translation;
        let destination = clamp_dungeon_translation(
            &inputs.dungeon_layout,
            skill_destination(start, facing, cursor_ground, skill.movement, rune),
        );
        if destination != start {
            player_transform.translation = destination;
            world.commands.write_message(skill_vfx::MovementVisual {
                start,
                end: destination,
                rune,
            });
        }
        // A blink strikes where the hero stood; every other movement strikes on arrival.
        let origin = if matches!(skill.movement, Movement::Blink(_)) {
            start
        } else {
            destination
        };
        if rune == Rune::Cleanse {
            world.commands.entity(player_entity).try_remove::<Burning>();
        }
        if skill.barrier > 0.0 {
            let granted = player_barrier.grant(skill.barrier);
            if granted > 0.0 {
                world.combat_events.write(CombatEvent {
                    text: format!("{} raised {granted:.0} barrier", skill.name),
                });
            }
        }
        let profile = StrikeProfile {
            damage: (skill.damage + damage_bonus * skill.bonus_scale)
                * total_multiplier
                * rune.damage_multiplier(),
            impact_scale: skill.impact * talents.combat_impact_multiplier(growth),
            crit_chance,
            legendary_powers,
            skill: Some(cast),
            hit_stop: stalls_on_hit(skill.strike),
            long_shot,
            cast: ledger.begin(),
            fury_bonus: 0.0,
            barrier_reward: talents.skill_barrier_gain(growth),
        };
        let target = ground_target(origin, facing, cursor_ground, skill.strike);
        if skill.on_contact {
            world
                .commands
                .entity(player_entity)
                .try_insert(queued_strike(
                    cast_secs,
                    facing,
                    target,
                    skill.strike,
                    skill.motion,
                    profile,
                ));
            // The basic attack waits for this swing like any other.
            reset_scaled(&mut cooldowns.basic, cast_secs, 1.0);
        } else {
            world.release(
                Hero {
                    position: origin,
                    health: &mut player_health,
                    barrier: &mut player_barrier,
                    fury: &mut fury,
                    ledger: &mut ledger,
                    equipment,
                    charm,
                    elixir,
                    evade,
                    surge,
                },
                resource,
                Release {
                    origin,
                    facing,
                    target,
                    shape: skill.strike,
                    profile,
                },
            );
        }
        let index = slot.index();
        reset_scaled(
            &mut cooldowns.slots[index],
            skill.cooldown,
            talents.cooldown_multiplier(),
        );
    }
}

/// Unbowed passive: a wounded hero hits harder.
fn unbowed_multiplier(passive: Passive, health: &Health) -> f32 {
    match passive {
        Passive::Unbowed {
            health_fraction,
            damage_multiplier,
        } if health.max > 0.0 && health.current / health.max < health_fraction => damage_multiplier,
        _ => 1.0,
    }
}

fn long_shot_passive(passive: Passive) -> Option<(f32, f32)> {
    match passive {
        Passive::LongShot {
            min_distance,
            damage_multiplier,
        } => Some((min_distance, damage_multiplier)),
        _ => None,
    }
}

fn long_shot_multiplier(long_shot: Option<(f32, f32)>, source: Vec3, target: Vec3) -> f32 {
    match long_shot {
        Some((min_distance, multiplier)) if flat_distance(source, target) >= min_distance => {
            multiplier
        }
        _ => 1.0,
    }
}

/// Blows thrown from the hero's own position freeze the swing on contact.
fn stalls_on_hit(shape: StrikeShape) -> bool {
    matches!(shape, StrikeShape::Arc { .. } | StrikeShape::Radial { .. })
}

/// Where a skill's movement puts the hero, before dungeon clamping.
fn skill_destination(
    origin: Vec3,
    facing: Vec3,
    cursor_ground: Option<Vec3>,
    movement: Movement,
    rune: Rune,
) -> Vec3 {
    let scale = rune.movement_multiplier();
    match movement {
        Movement::None => origin,
        Movement::Toward(distance) => origin + facing * distance * scale,
        Movement::Away(distance) => origin - facing * distance * scale,
        Movement::Leap(max_distance) | Movement::Blink(max_distance) => {
            let max_distance = max_distance * scale;
            let wanted = cursor_ground
                .map(|point| Vec3::new(point.x - origin.x, 0.0, point.z - origin.z))
                .unwrap_or(facing * max_distance);
            origin + wanted.clamp_length_max(max_distance)
        }
    }
}

/// The ground point a cursor-targeted strike lands on, kept within its cast range.
fn ground_target(
    origin: Vec3,
    facing: Vec3,
    cursor_ground: Option<Vec3>,
    shape: StrikeShape,
) -> Vec3 {
    let StrikeShape::GroundBlast { cast_range, .. } = shape else {
        return origin + facing;
    };
    let wanted = cursor_ground
        .map(|point| Vec3::new(point.x - origin.x, 0.0, point.z - origin.z))
        .unwrap_or(facing * cast_range * 0.6);
    let target = origin + wanted.clamp_length_max(cast_range);
    Vec3::new(target.x, 0.0, target.z)
}

fn grant_vigor_barrier(
    commands: &mut Commands,
    barrier: &mut Barrier,
    position: Vec3,
    reward: f32,
) {
    let amount = barrier.grant(reward);
    if amount > 0.0 {
        commands.write_message(TalentProcFeedback {
            position,
            branch: TalentBranch::Vigor,
            amount,
        });
    }
}

fn recoil_scale_for_enemy(enemy: &Enemy, impact: f32) -> f32 {
    if is_boss_enemy(enemy) {
        impact.min(0.15)
    } else {
        impact
    }
}

/// A strike that resolves at the contact frame of a `cycle`-second animation.
fn queued_strike(
    cycle: f32,
    facing: Vec3,
    target: Vec3,
    shape: StrikeShape,
    motion: PlayerAttackMotion,
    profile: StrikeProfile,
) -> PendingPlayerStrike {
    PendingPlayerStrike {
        timer: Timer::from_seconds(cycle * attack_contact_fraction(motion), TimerMode::Once),
        facing,
        target,
        shape,
        motion,
        profile,
    }
}

fn resolve_pending_player_strikes(
    time: Res<Time>,
    catalog: Res<ClassCatalog>,
    selected: Res<SelectedClass>,
    mut world: StrikeWorld,
    mut heroes: StrikeHeroQuery,
) {
    let resource = catalog.get(selected.0).resource.kind;
    for (
        player_entity,
        player_transform,
        mut player_health,
        mut player_barrier,
        mut fury,
        mut ledger,
        mut animation,
        equipment,
        charm,
        elixir,
        evade,
        surge,
        pending,
    ) in &mut heroes
    {
        let Some(mut pending) = pending else {
            continue;
        };
        pending.timer.tick(time.delta());
        if !pending.timer.is_finished() {
            continue;
        }
        let origin = player_transform.translation;
        let outcome = world.release(
            Hero {
                position: origin,
                health: &mut player_health,
                barrier: &mut player_barrier,
                fury: &mut fury,
                ledger: &mut ledger,
                equipment,
                charm,
                elixir,
                evade,
                surge,
            },
            resource,
            Release {
                origin,
                facing: pending.facing,
                target: pending.target,
                shape: pending.shape,
                profile: pending.profile,
            },
        );
        if pending.shape.is_melee() && outcome.damaging_hits > 0 {
            animation.confirm_melee_contact(pending.motion);
        }
        world
            .commands
            .entity(player_entity)
            .remove::<PendingPlayerStrike>();
    }
}

fn basic_attack_requested(
    mouse: &ButtonInput<MouseButton>,
    cooldowns: &SkillCooldowns,
    intent: &BasicAttackIntent,
    target: Option<PrimaryAttackTarget>,
) -> bool {
    if mouse.pressed(MouseButton::Right) || mouse.just_pressed(MouseButton::Right) {
        return false;
    }
    let held_attack = mouse.pressed(MouseButton::Left) || mouse.just_pressed(MouseButton::Left);
    let wants_attack =
        held_attack || target.is_some_and(|target| intent.target == Some(target.entity));
    wants_attack && target.is_none_or(|target| target.in_range) && basic_attack_ready(cooldowns)
}

fn basic_attack_ready(cooldowns: &SkillCooldowns) -> bool {
    cooldowns.basic.remaining_secs() <= f32::EPSILON
}

fn update_basic_attack_intent(
    intent: &mut BasicAttackIntent,
    mouse: &ButtonInput<MouseButton>,
    cursor_ground: Option<Vec3>,
    cursor_target: Option<PrimaryAttackTarget>,
) {
    if mouse.pressed(MouseButton::Right) || mouse.just_pressed(MouseButton::Right) {
        intent.target = None;
        return;
    }
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

fn rune_effect_text(rune: Rune, affected: u32) -> Option<String> {
    if affected == 0 {
        return None;
    }
    match rune {
        Rune::Expose => Some(format!("Exposed {affected} enemies")),
        Rune::Hemorrhage => Some(format!("Bleeding {affected} enemies")),
        Rune::Frost => Some(format!("Chilled {affected} enemies")),
        _ => None,
    }
}

fn skill_cast_text(name: &str, rune: Rune) -> String {
    format!("{} {name}", rune.label())
}

fn skill_cast_screen_shake(slot: SkillSlot) -> ScreenShakeEvent {
    let (intensity, duration_secs) = match slot {
        SkillSlot::Mobility => (0.026, 0.10),
        SkillSlot::Finisher => (0.036, 0.13),
        SkillSlot::Area => (0.046, 0.16),
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

/// Each legendary power resonates with two runes, whatever skill carries them.
fn legendary_power_rune_synergy(power: LegendaryPower, rune: Option<Rune>) -> bool {
    let Some(rune) = rune else {
        return false;
    };
    matches!(
        (power, rune),
        (LegendaryPower::Emberbrand, Rune::Ember | Rune::Hemorrhage)
            | (LegendaryPower::Frostbrand, Rune::Frost | Rune::Expose)
            | (LegendaryPower::Stormbrand, Rune::Reap | Rune::Frost)
            | (LegendaryPower::Soulreaver, Rune::Reap | Rune::Hemorrhage)
            | (LegendaryPower::Aegisbrand, Rune::Cleanse | Rune::Expose)
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

fn is_boss_enemy(enemy: &Enemy) -> bool {
    matches!(enemy.id.as_str(), "keeper" | "hollow_matron")
}

fn hit_impact_profile(
    enemy: &Enemy,
    critical: bool,
    impact_scale: f32,
    boss_break: bool,
) -> HitImpactProfile {
    let tier = if boss_break {
        HitImpactTier::BossBreak
    } else if is_boss_enemy(enemy) {
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
    let duration_secs = (if critical { 0.095 } else { 0.070 }) + extra_weight * 0.025;
    let relative_speed = (if critical { 0.055 } else { 0.09 }) - extra_weight * 0.025;
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
    HitStopEvent::new(0.04, 0.12)
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
    if !can_trigger_boss_stagger(enemy, staggered) {
        return false;
    }
    let Some(mut stagger) = stagger else {
        return false;
    };
    if add_boss_stagger(&mut stagger, hit.damage) {
        commands.entity(hit.entity).try_insert(boss_staggered());
        spawn_stagger_vfx(commands, vfx, hit.target, enemy);
        combat_events.write(CombatEvent {
            text: boss_stagger_event_text(enemy),
        });
        return true;
    }
    false
}

fn can_trigger_boss_stagger(enemy: &Enemy, staggered: Option<&Staggered>) -> bool {
    is_boss_enemy(enemy) && staggered.is_none()
}

fn boss_stagger_event_text(enemy: &Enemy) -> String {
    format!("{} staggered: punish the opening", enemy.display_name)
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
    finisher: SkillSlot,
    hits: u32,
    resource: ResourceKind,
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
            resource,
        ),
    });
    feedback.screen_shake.write(ScreenShakeEvent::new(
        combo_screen_shake(hits) + chain_bonus.shake + set_bonus.shake,
        0.12,
    ));
    combo.clear();
}

/// Finishers mark enemies for area or mobility skills; area skills mark them for mobility skills.
fn combo_finisher_allowed(primer: SkillSlot, finisher: SkillSlot) -> bool {
    matches!(
        (primer, finisher),
        (SkillSlot::Finisher, SkillSlot::Mobility)
            | (SkillSlot::Finisher, SkillSlot::Area)
            | (SkillSlot::Area, SkillSlot::Mobility)
    )
}

fn combo_fury_reward(hits: u32) -> f32 {
    8.0 + hits.min(5) as f32 * 3.0
}

fn combo_screen_shake(hits: u32) -> f32 {
    0.045 + hits.min(5) as f32 * 0.008
}

fn combo_chain_bonus(primer: SkillSlot, finisher: SkillSlot, hits: u32) -> ComboChainBonus {
    if !combo_finisher_allowed(primer, finisher) || hits == 0 {
        return ComboChainBonus::default();
    }

    let hits = hits.min(5) as f32;
    let multi_target_fury = if hits >= 3.0 { hits * 2.0 } else { 0.0 };
    match (primer, finisher) {
        (SkillSlot::Finisher, SkillSlot::Mobility) => ComboChainBonus {
            fury: multi_target_fury,
            barrier: 5.0 + hits * 3.0,
            shake: 0.015,
            label: Some("Cull"),
            ..default()
        },
        (SkillSlot::Finisher, SkillSlot::Area) => ComboChainBonus {
            fury: multi_target_fury,
            healing: 5.0 + hits * 3.0,
            shake: 0.02,
            label: Some("Bloodburst"),
            ..default()
        },
        (SkillSlot::Area, SkillSlot::Mobility) => ComboChainBonus {
            fury: 2.0 + multi_target_fury,
            barrier: 6.0 + hits * 4.0,
            shake: 0.02,
            label: Some("Breakthrough"),
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

#[allow(clippy::too_many_arguments)]
fn combo_break_text(
    primer: SkillSlot,
    finisher: SkillSlot,
    hits: u32,
    fury_gained: f32,
    healing_done: f32,
    barrier_gained: f32,
    set_label: Option<&str>,
    resource: ResourceKind,
) -> String {
    let mut text = format!(
        "Combo Break: {} into {} hit {} target{} +{:.0} {}",
        combo_slot_label(primer),
        combo_slot_label(finisher),
        hits,
        if hits == 1 { "" } else { "s" },
        fury_gained,
        resource.label()
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

fn combo_slot_label(slot: SkillSlot) -> &'static str {
    match slot {
        SkillSlot::Finisher => "finisher",
        SkillSlot::Area => "area skill",
        SkillSlot::Mobility => "mobility skill",
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
        holy_impact: assets.holy_impact.clone(),
        ember_impact: assets.ember_impact.clone(),
        frost_impact: assets.frost_impact.clone(),
        shadow_burst: assets.shadow_burst.clone(),
        stagger_burst: assets.stagger_burst.clone(),
    }
}

/// The burst artwork a rune leaves on the ground; basic attacks and Ember share the ember flash.
fn rune_burst_scene(assets: &CombatVfxAssets, rune: Option<Rune>) -> Handle<WorldAsset> {
    match rune {
        Some(Rune::Frost) => assets.frost_impact.clone(),
        Some(Rune::Cleanse | Rune::Expose) => assets.holy_impact.clone(),
        Some(Rune::Reap | Rune::Hemorrhage) => assets.shadow_burst.clone(),
        Some(Rune::Ember) | None => assets.ember_impact.clone(),
    }
}

fn spawn_burst_vfx(
    commands: &mut Commands,
    assets: &CombatVfxAssets,
    origin: Vec3,
    rune: Option<Rune>,
    radius: f32,
) {
    spawn_combat_vfx(
        commands,
        rune_burst_scene(assets, rune),
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

fn nova_cast_lifetime(rune: Option<Rune>) -> f32 {
    if rune == Some(Rune::Frost) {
        0.48
    } else {
        0.40
    }
}

fn nova_outer_ring_lifetime(rune: Option<Rune>) -> f32 {
    nova_cast_lifetime(rune) + 0.10
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

fn select_hit_bone(enemy: &Enemy, critical: bool) -> HitBone {
    if critical {
        HitBone::Head
    } else if is_boss_enemy(enemy) || matches!(enemy.id.as_str(), "butcher" | "seal_warden") {
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

fn enemy_hit_bone_scale(enemy: &Enemy) -> f32 {
    if is_boss_enemy(enemy) {
        return 1.18;
    }
    match enemy.id.as_str() {
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

fn apply_strike(
    commands: &mut Commands,
    feedback: &mut AttackFeedback,
    vfx: &CombatVfxAssets,
    strike: Strike,
    enemies: &mut EnemyTargets,
    bestiary: &Bestiary,
) -> AttackOutcome {
    let profile = strike.profile;
    let rune = profile.skill.map(|cast| cast.rune);
    let recoil_origin = strike.area.recoil_origin(strike.source);
    let source_label = match rune {
        None => "Strike".to_string(),
        Some(rune) => format!("{} skill", rune.label()),
    };
    let mut outcome = AttackOutcome::default();
    let mut storm_chains = Vec::new();
    for (entity, transform, enemy, mut health, ward, chilled, vulnerable, stagger, staggered) in
        enemies.iter_mut()
    {
        if health.current <= 0.0 || !strike.area.contains(entity, transform.translation) {
            continue;
        }
        let critical = roll_critical(profile.crit_chance);
        let reach_bonus =
            long_shot_multiplier(profile.long_shot, strike.source, transform.translation);
        let damage = bestiary_adjusted_damage(profile.damage * reach_bonus, enemy, bestiary);
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
        let impact_profile = hit_impact_profile(enemy, critical, profile.impact_scale, boss_break);
        if final_damage > 0.0 {
            feedback.combat_events.write(CombatEvent {
                text: damage_text(&source_label, final_damage, critical),
            });
        }
        feedback.floating_text.write(FloatingCombatTextEvent {
            text: floating_damage_text_for_ward_result(final_damage, ward_result, impact_profile),
            position: hit_bone_world_position(transform.translation, enemy, critical),
            critical,
        });
        trigger_hit_impact_screen_shake(feedback.screen_shake, impact_profile);
        if profile.hit_stop {
            trigger_hit_impact_hit_stop(feedback.hit_stop, impact_profile);
        }
        if final_damage > 0.0 {
            outcome.damaging_hits += 1;
            commands.write_message(ContactFeedback {
                target: entity,
                position: hit_bone_world_position(transform.translation, enemy, critical),
                source: strike.source,
                skill: profile.skill,
                strength: profile.impact_scale * if critical { 1.2 } else { 1.0 },
            });
        }
        commands.entity(entity).try_insert((
            hit_react_for_contact(
                critical,
                recoil_origin,
                transform.translation,
                recoil_scale_for_enemy(enemy, profile.impact_scale),
            ),
            EnemyTargetFocus::new(critical),
        ));
        if rune == Some(Rune::Frost) {
            commands.entity(entity).try_insert(Chilled {
                timer: Timer::from_seconds(2.8, TimerMode::Once),
                slow_multiplier: 0.48,
            });
            outcome.rune_applied += 1;
        }
        let proc_trigger = LegendaryProcTrigger::from_cast(critical, profile.skill);
        if let Some(burning) =
            legendary_burning(profile.legendary_powers, proc_trigger, final_damage)
        {
            commands.entity(entity).try_insert(burning);
            feedback.combat_events.write(CombatEvent {
                text: "Emberbrand ignited enemy".to_string(),
            });
        }
        if let Some(chilled) = legendary_chilled(profile.legendary_powers, proc_trigger) {
            commands.entity(entity).try_insert(chilled);
            feedback.combat_events.write(CombatEvent {
                text: "Frostbrand chilled enemy".to_string(),
            });
        }
        if let Some(damage) =
            stormbrand_chain_damage(profile.legendary_powers, proc_trigger, final_damage)
        {
            storm_chains.push(StormChain {
                source: entity,
                origin: transform.translation,
                damage,
            });
        }
        outcome.life_stolen +=
            legendary_life_steal(profile.legendary_powers, proc_trigger, final_damage);
        outcome.barrier_gained +=
            legendary_barrier(profile.legendary_powers, proc_trigger, final_damage);
        outcome.reflected_damage += reflective_damage(enemy, final_damage);
        match rune {
            Some(Rune::Expose) => {
                commands.entity(entity).try_insert(Vulnerable {
                    timer: Timer::from_seconds(4.0, TimerMode::Once),
                    damage_multiplier: 1.22,
                });
                outcome.rune_applied += 1;
            }
            Some(Rune::Hemorrhage) => {
                commands.entity(entity).try_insert(Burning {
                    dps: (bestiary_adjusted_damage(profile.damage, enemy, bestiary) * 0.20)
                        .max(4.0),
                    timer: Timer::from_seconds(3.5, TimerMode::Once),
                });
                outcome.rune_applied += 1;
            }
            _ => {}
        }
        outcome.hits += 1;
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
            commands
                .entity(entity)
                .try_insert((hit_react_for_damage(false), EnemyTargetFocus::new(false)));
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

fn damage_breakables(area: HitArea, damage: f32, breakables: &mut BreakableTargets) {
    for (entity, transform, mut health) in breakables.iter_mut() {
        if area.contains(entity, transform.translation) {
            health.current -= damage;
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

/// Whether a skill that refuses to swing at air would reach something alive.
fn strike_has_target(
    origin: Vec3,
    forward: Vec3,
    shape: StrikeShape,
    targets: impl Iterator<Item = (Vec3, f32)>,
) -> bool {
    let reach = shape.reach();
    targets.into_iter().any(|(position, health)| {
        health > 0.0
            && match shape {
                StrikeShape::Arc { .. } => arc_target_hit(origin, forward, position, reach),
                _ => flat_distance(origin, position) <= reach,
            }
    })
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
    fn vigor_shield_is_once_per_successful_cast_and_never_for_a_miss() {
        fn awarded(hits: u32) -> f32 {
            let mut world = World::new();
            world.init_resource::<Messages<TalentProcFeedback>>();
            let mut barrier = Barrier {
                current: 0.0,
                max: 50.0,
            };
            grant_vigor_barrier(&mut world.commands(), &mut barrier, Vec3::ZERO, 6.0, hits);
            world.flush();
            assert_eq!(
                world.resource::<Messages<TalentProcFeedback>>().len(),
                usize::from(hits > 0)
            );
            barrier.current
        }
        assert_eq!(awarded(0), 0.0);
        assert_eq!(awarded(1), 6.0);
        assert_eq!(awarded(5), 6.0);
    }

    #[test]
    fn focus_scales_contact_and_complete_swing_together() {
        for rank in [0, 1, 4, 8] {
            let talents = Talents {
                focus: rank,
                ..default()
            };
            let cycle = 0.34 * talents.cooldown_multiplier();
            let contact = attack_contact_delay(cycle);
            assert!(
                (contact / cycle
                    - crate::player::attack_contact_fraction(PlayerAttackMotion::Basic))
                .abs()
                    < 0.0001
            );
            assert!(contact < cycle);
        }
        assert!(!basic_attack_ready(&test_skill_cooldowns(0.03)));
    }

    #[test]
    fn ordinary_contact_has_weight_without_pushing_bosses_out_of_the_arena() {
        let normal = test_enemy(vec![]);
        let mut boss = test_enemy(vec![]);
        boss.id = "keeper".to_owned();
        assert_eq!(recoil_scale_for_enemy(&normal, 1.57), 1.57);
        assert!(recoil_scale_for_enemy(&boss, 1.57) <= 0.15);
        let light = hit_impact_hit_stop_event(false, 1.0);
        let heavy = hit_impact_hit_stop_event(false, 1.57);
        assert!(light.duration_secs >= 0.07);
        assert!(heavy.duration_secs > light.duration_secs);
        assert!(heavy.relative_speed < light.relative_speed);
    }

    #[test]
    fn only_live_in_range_targets_emit_contact_and_hitstop() {
        #[derive(SystemParam)]
        struct TestFeedback<'w> {
            combat_events: MessageWriter<'w, CombatEvent>,
            floating_text: MessageWriter<'w, FloatingCombatTextEvent>,
            screen_shake: MessageWriter<'w, ScreenShakeEvent>,
            hit_stop: MessageWriter<'w, HitStopEvent>,
        }
        fn strike(mut commands: Commands, mut enemies: EnemyTargets, mut feedback: TestFeedback) {
            apply_arc_damage(
                &mut commands,
                &mut AttackFeedback {
                    combat_events: &mut feedback.combat_events,
                    floating_text: &mut feedback.floating_text,
                    screen_shake: &mut feedback.screen_shake,
                    hit_stop: &mut feedback.hit_stop,
                },
                &CombatVfxAssets::default(),
                ArcAttack {
                    origin: Vec3::ZERO,
                    facing: Vec3::Z,
                    range: 2.2,
                    damage: 10.0,
                    impact_scale: 1.0,
                    crit_chance: 0.0,
                    legendary_powers: LegendaryPowerSet::none(),
                    skill: None,
                },
                &mut enemies,
                &Bestiary::default(),
            );
        }
        let mut app = App::new();
        app.add_message::<CombatEvent>()
            .add_message::<FloatingCombatTextEvent>()
            .add_message::<ScreenShakeEvent>()
            .add_message::<HitStopEvent>()
            .add_message::<ContactFeedback>()
            .add_systems(Update, strike);
        let live = app
            .world_mut()
            .spawn((
                test_enemy(vec![]),
                Transform::from_xyz(0.0, 0.0, 1.0),
                Health {
                    current: 100.0,
                    max: 100.0,
                },
            ))
            .id();
        app.world_mut().spawn((
            test_enemy(vec![]),
            Transform::from_xyz(0.0, 0.0, 1.0),
            Health {
                current: 0.0,
                max: 100.0,
            },
        ));
        app.world_mut().spawn((
            test_enemy(vec![]),
            Transform::from_xyz(0.0, 0.0, 8.0),
            Health {
                current: 100.0,
                max: 100.0,
            },
        ));
        app.update();
        assert_eq!(app.world().get::<Health>(live).unwrap().current, 90.0);
        assert_eq!(app.world().resource::<Messages<ContactFeedback>>().len(), 1);
        assert_eq!(app.world().resource::<Messages<HitStopEvent>>().len(), 1);
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
        let delay = attack_contact_delay(0.34);
        let progress = delay / 0.34;

        assert!((0.10..=0.18).contains(&delay));
        assert!((0.27..=0.53).contains(&progress));
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
        mouse.clear_just_pressed(MouseButton::Left);
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
        mouse.clear_just_pressed(MouseButton::Left);
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
        assert!(basic_attack_requested(
            &mouse,
            &test_skill_cooldowns(0.0),
            &intent,
            None
        ));
    }

    #[test]
    fn right_move_cancels_attack_intent_even_when_left_is_held() {
        let target = PrimaryAttackTarget {
            entity: Entity::from_raw_u32(7).unwrap(),
            position: Vec3::X,
            in_range: true,
        };
        let mut intent = BasicAttackIntent {
            target: Some(target.entity),
        };
        let mut mouse = ButtonInput::default();
        mouse.press(MouseButton::Left);
        mouse.press(MouseButton::Right);
        update_basic_attack_intent(&mut intent, &mouse, Some(Vec3::Z), Some(target));
        assert_eq!(intent.target, None);
        assert!(!basic_attack_requested(
            &mouse,
            &test_skill_cooldowns(0.0),
            &intent,
            Some(target)
        ));
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
        let mut hollow_matron = test_enemy(vec![]);
        hollow_matron.id = "hollow_matron".to_string();

        assert_eq!(select_hit_bone(&normal, false), HitBone::Left);
        assert_eq!(select_hit_bone(&guarded, false), HitBone::Weapon);
        assert_eq!(select_hit_bone(&cultist, false), HitBone::Right);
        assert_eq!(select_hit_bone(&warden, false), HitBone::Chest);
        assert_eq!(select_hit_bone(&keeper, false), HitBone::Chest);
        assert_eq!(
            select_hit_bone(&hollow_matron, false),
            select_hit_bone(&keeper, false)
        );
        assert_eq!(select_hit_bone(&normal, true), HitBone::Head);
    }

    #[test]
    fn hit_bone_offsets_scale_by_enemy_silhouette() {
        let mut skeleton = test_enemy(vec![]);
        skeleton.id = "skeleton".to_string();
        let mut keeper = test_enemy(vec![]);
        keeper.id = "keeper".to_string();
        let mut hollow_matron = test_enemy(vec![]);
        hollow_matron.id = "hollow_matron".to_string();

        assert!(
            hit_bone_offset(HitBone::Head, &keeper).y > hit_bone_offset(HitBone::Head, &skeleton).y
        );
        assert_eq!(
            hit_bone_offset(HitBone::Head, &hollow_matron),
            hit_bone_offset(HitBone::Head, &keeper)
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
    fn chapter_bosses_share_stagger_rules_and_use_their_display_names() {
        let mut malrec = test_enemy(vec![]);
        malrec.id = "keeper".to_string();
        malrec.display_name = "Malrec, Keeper of Ash".to_string();
        let mut nyxara = test_enemy(vec![]);
        nyxara.id = "hollow_matron".to_string();
        nyxara.display_name = "Nyxara, Hollow Matron".to_string();
        let normal = test_enemy(vec![]);
        let staggered = boss_staggered();

        assert!(can_trigger_boss_stagger(&malrec, None));
        assert!(can_trigger_boss_stagger(&nyxara, None));
        assert!(!can_trigger_boss_stagger(&normal, None));
        assert!(!can_trigger_boss_stagger(&malrec, Some(&staggered)));
        assert!(!can_trigger_boss_stagger(&nyxara, Some(&staggered)));
        assert_eq!(
            boss_stagger_event_text(&malrec),
            "Malrec, Keeper of Ash staggered: punish the opening"
        );
        assert_eq!(
            boss_stagger_event_text(&nyxara),
            "Nyxara, Hollow Matron staggered: punish the opening"
        );
    }

    #[test]
    fn hit_impact_profiles_promote_elites_bosses_and_breaks() {
        let normal = test_enemy(vec![]);
        let elite = test_enemy(vec![EnemyAffix::Shielded]);
        let mut nemesis = test_enemy(vec![]);
        nemesis.id = "nemesis".to_string();
        let mut boss = test_enemy(vec![]);
        boss.id = "keeper".to_string();
        let mut hollow_matron = test_enemy(vec![]);
        hollow_matron.id = "hollow_matron".to_string();

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
            hit_impact_profile(&hollow_matron, true, 1.0, false),
            hit_impact_profile(&boss, true, 1.0, false)
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
        assert!(critical.duration_secs > basic.duration_secs);
        assert!(critical.duration_secs <= 0.12);
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

        combo.prime(ComboPrimer::Nova);
        assert_eq!(combo.active_primer(), Some(ComboPrimer::Nova));
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
    fn rupture_does_not_authorize_a_cast_on_empty_dead_or_unreachable_targets() {
        let target = |position, health| {
            rupture_has_target(Vec3::ZERO, Vec3::Z, 3.1, [(position, health)].into_iter())
        };
        assert!(!rupture_has_target(
            Vec3::ZERO,
            Vec3::Z,
            3.1,
            std::iter::empty()
        ));
        assert!(!target(Vec3::Z * 2.0, 0.0));
        assert!(!target(Vec3::Z * 4.0, 100.0));
        assert!(!target(-Vec3::Z, 100.0));
        assert!(target(Vec3::Z * 2.0, 100.0));
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
