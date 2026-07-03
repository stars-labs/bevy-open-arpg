use crate::{
    CARNAGE_MASTER_STREAK, GameState, LAST_STAND_HEALTH_RATIO, RunStats, SET_ADEPT_KILLS,
    assets::GameAssets,
    chapter::{ChapterPhase, ChapterProgress, InteractableKind, InteractableUsed},
    companion::CompanionBond,
    data::PlayerTuning,
    enemy::{Enemy, EnemyKilled, SpawnNemesis, enemy_cursor_pick_radius},
    feedback::{CombatEvent, FloatingCombatTextEvent, ScreenShakeEvent},
    not_paused, register_massacre_kill,
};
use bevy::ecs::system::SystemParam;
use bevy::prelude::*;
use bevy::window::PrimaryWindow;
use serde::{Deserialize, Serialize};

pub const CHAPTER_LEVEL_CAP: u32 = 6;
pub const EMBER_PARAGON_BASE_XP: u32 = 140;
pub const SURGE_CHAIN_KILLS: u32 = 3;
pub const PYLON_REAPER_KILLS: u32 = 3;
pub const RELIQUARY_MOMENTUM_MAX: u32 = 5;
pub const RELIQUARY_BOON_MOMENTUM_THRESHOLD: u32 = 3;

#[derive(Component)]
pub struct Player;

#[derive(Component)]
pub struct Health {
    pub current: f32,
    pub max: f32,
}

#[derive(Component, Debug, Clone, Copy, PartialEq)]
pub struct Barrier {
    pub current: f32,
    pub max: f32,
}

impl Barrier {
    pub fn grant(&mut self, amount: f32) -> f32 {
        let before = self.current;
        self.current = (self.current + amount.max(0.0)).min(self.max);
        self.current - before
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DamageTaken {
    pub absorbed: f32,
    pub health_damage: f32,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum PlayerHitReactKind {
    Health,
    Ward,
    Evade,
}

#[derive(Component)]
pub struct PlayerHitReact {
    pub kind: PlayerHitReactKind,
    timer: Timer,
}

impl PlayerHitReact {
    pub fn from_damage(taken: DamageTaken) -> Self {
        let kind = player_hit_react_kind(taken);
        let duration_secs = match kind {
            PlayerHitReactKind::Health => 0.24,
            PlayerHitReactKind::Ward => 0.20,
            PlayerHitReactKind::Evade => 0.18,
        };
        Self {
            kind,
            timer: Timer::from_seconds(duration_secs, TimerMode::Once),
        }
    }

    fn progress(&self) -> f32 {
        let duration = self.timer.duration().as_secs_f32();
        if duration <= 0.0 {
            return 1.0;
        }
        (self.timer.elapsed_secs() / duration).clamp(0.0, 1.0)
    }
}

fn player_hit_react_kind(taken: DamageTaken) -> PlayerHitReactKind {
    if taken.health_damage > 0.0 {
        PlayerHitReactKind::Health
    } else if taken.absorbed > 0.0 {
        PlayerHitReactKind::Ward
    } else {
        PlayerHitReactKind::Evade
    }
}

pub fn apply_player_damage(health: &mut Health, barrier: &mut Barrier, damage: f32) -> DamageTaken {
    let damage = damage.max(0.0);
    let absorbed = damage.min(barrier.current);
    barrier.current -= absorbed;
    let health_damage = damage - absorbed;
    health.current = (health.current - health_damage).max(0.0);
    DamageTaken {
        absorbed,
        health_damage,
    }
}

#[derive(Component, Debug)]
pub struct Evade {
    pub active: Timer,
    pub cooldown: Timer,
    pub cooldown_secs: f32,
    pub speed_multiplier: f32,
}

pub fn evade_seconds_remaining(evade: &Evade) -> f32 {
    if buff_timer_active(&evade.active) {
        evade.active.remaining_secs()
    } else {
        0.0
    }
}

pub fn evade_cooldown_remaining(evade: &Evade) -> f32 {
    if buff_timer_active(&evade.cooldown) {
        evade.cooldown.remaining_secs()
    } else {
        0.0
    }
}

pub fn evade_speed_multiplier(evade: &Evade) -> f32 {
    if buff_timer_active(&evade.active) {
        evade.speed_multiplier
    } else {
        1.0
    }
}

pub fn apply_player_damage_with_evade(
    health: &mut Health,
    barrier: &mut Barrier,
    evade: &Evade,
    damage: f32,
) -> DamageTaken {
    if evade_seconds_remaining(evade) > 0.0 {
        return DamageTaken {
            absorbed: 0.0,
            health_damage: 0.0,
        };
    }
    apply_player_damage(health, barrier, damage)
}

#[derive(Component)]
pub struct Velocity(pub Vec3);

#[derive(Resource, Default)]
struct ClickMoveTarget {
    position: Option<Vec3>,
    stop_radius: f32,
}

const CLICK_MOVE_STOP_RADIUS: f32 = 0.22;
const CLICK_MOVE_MARKER_Y: f32 = 0.045;
const CLICK_MOVE_MARKER_FADE_SECS: f32 = 0.28;
const GAMEPAD_MOVE_DEAD_ZONE: f32 = 0.18;

#[derive(Component)]
pub struct PlayerWeaponVisual;

#[derive(Debug, Clone, Copy, Component, Eq, PartialEq)]
enum PlayerLimbKind {
    ShoulderMantle,
    LeftArm,
    RightArm,
    LeftLeg,
    RightLeg,
}

impl PlayerLimbKind {
    fn side(self) -> f32 {
        match self {
            Self::LeftArm | Self::LeftLeg => -1.0,
            Self::RightArm | Self::RightLeg => 1.0,
            Self::ShoulderMantle => 0.0,
        }
    }

    fn label(self) -> &'static str {
        match self {
            Self::ShoulderMantle => "Animated Shoulder Mantle",
            Self::LeftArm => "Animated Left Arm",
            Self::RightArm => "Animated Weapon Arm",
            Self::LeftLeg => "Animated Left Leg",
            Self::RightLeg => "Animated Right Leg",
        }
    }
}

#[derive(Component)]
struct PlayerLimbVisual {
    kind: PlayerLimbKind,
}

#[derive(Component)]
struct PlayerFootVisual {
    side: f32,
}

#[derive(Component)]
struct PlayerAttackArcVisual;

#[derive(Component)]
struct PlayerAttackCommitVisual;

#[derive(Component)]
struct ClickMoveMarker {
    fade: Timer,
    last_position: Option<Vec3>,
}

#[derive(Debug, Clone, Copy, Default, Eq, PartialEq)]
pub enum PlayerAttackMotion {
    #[default]
    Basic,
    Dash,
    Rupture,
    Nova,
}

#[derive(Component)]
pub struct PlayerAnimation {
    walk_phase: f32,
    facing_yaw: f32,
    turn_lean: f32,
    attack_timer: Timer,
    attack_duration_secs: f32,
    attack_motion: PlayerAttackMotion,
    motion_trail_timer: Timer,
    attack_trail_timer: Timer,
    footstep_timer: Timer,
    next_footstep_side: f32,
}

impl Default for PlayerAnimation {
    fn default() -> Self {
        let mut attack_timer = Timer::from_seconds(0.01, TimerMode::Once);
        attack_timer.set_elapsed(attack_timer.duration());
        let mut motion_trail_timer = Timer::from_seconds(0.01, TimerMode::Once);
        motion_trail_timer.set_elapsed(motion_trail_timer.duration());
        let mut attack_trail_timer = Timer::from_seconds(0.01, TimerMode::Once);
        attack_trail_timer.set_elapsed(attack_trail_timer.duration());
        let mut footstep_timer = Timer::from_seconds(0.01, TimerMode::Once);
        footstep_timer.set_elapsed(footstep_timer.duration());
        Self {
            walk_phase: 0.0,
            facing_yaw: 0.0,
            turn_lean: 0.0,
            attack_timer,
            attack_duration_secs: 0.01,
            attack_motion: PlayerAttackMotion::Basic,
            motion_trail_timer,
            attack_trail_timer,
            footstep_timer,
            next_footstep_side: -1.0,
        }
    }
}

impl PlayerAnimation {
    pub fn trigger_attack(&mut self, motion: PlayerAttackMotion, duration_secs: f32) {
        self.attack_duration_secs = duration_secs.max(0.05);
        self.attack_motion = motion;
        self.attack_timer = Timer::from_seconds(self.attack_duration_secs, TimerMode::Once);
        self.attack_timer.reset();
    }

    pub fn face_direction(&mut self, direction: Vec3) {
        let direction = Vec3::new(direction.x, 0.0, direction.z).normalize_or_zero();
        if direction.length_squared() > 0.0 {
            self.facing_yaw = direction.x.atan2(direction.z);
            self.turn_lean = 0.0;
        }
    }
}

#[derive(Component)]
struct PlayerMotionTrail {
    timer: Timer,
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct PlayerVisualPose {
    body_y: f32,
    body_yaw: f32,
    body_pitch: f32,
    body_roll: f32,
    body_scale: Vec3,
    weapon_translation: Vec3,
    weapon_scale: Vec3,
    weapon_pitch: f32,
    weapon_yaw: f32,
    weapon_roll: f32,
}

#[derive(Component)]
pub struct SkillCooldowns {
    pub dash: Timer,
    pub nova: Timer,
    pub rupture: Timer,
    pub basic: Timer,
    pub dash_base: f32,
    pub nova_base: f32,
    pub rupture_base: f32,
    pub basic_base: f32,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub enum DashRune {
    #[default]
    Cleanse,
    Reap,
}

impl DashRune {
    pub fn next(self) -> Self {
        match self {
            Self::Cleanse => Self::Reap,
            Self::Reap => Self::Cleanse,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Cleanse => "Cleanse",
            Self::Reap => "Reap",
        }
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub enum NovaRune {
    #[default]
    Ember,
    Frost,
}

impl NovaRune {
    pub fn next(self) -> Self {
        match self {
            Self::Ember => Self::Frost,
            Self::Frost => Self::Ember,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Ember => "Ember",
            Self::Frost => "Frost",
        }
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub enum RuptureRune {
    #[default]
    Expose,
    Hemorrhage,
}

impl RuptureRune {
    pub fn next(self) -> Self {
        match self {
            Self::Expose => Self::Hemorrhage,
            Self::Hemorrhage => Self::Expose,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Expose => "Expose",
            Self::Hemorrhage => "Hemorrhage",
        }
    }
}

#[derive(Component, Debug, Default)]
pub struct SkillRunes {
    pub dash: DashRune,
    pub nova: NovaRune,
    pub rupture: RuptureRune,
}

impl SkillRunes {
    #[cfg_attr(not(test), allow(dead_code))]
    pub fn summary(&self) -> String {
        format!(
            "Runes: Z Dash {} | X Nova {} | C Rupture {}",
            self.dash.label(),
            self.nova.label(),
            self.rupture.label()
        )
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ChapterBoonChoice {
    #[default]
    None,
    CrimsonOath,
    TitanWard,
    ArcaneMomentum,
}

impl ChapterBoonChoice {
    pub fn label(self) -> &'static str {
        match self {
            Self::None => "No boon",
            Self::CrimsonOath => "Crimson Oath",
            Self::TitanWard => "Titan Ward",
            Self::ArcaneMomentum => "Arcane Momentum",
        }
    }

    pub fn summary(self) -> &'static str {
        match self {
            Self::None => "Boss phase boon: 4 Crimson dmg | 5 Titan defense | 6 Arcane fury",
            Self::CrimsonOath => "Crimson Oath: +6 damage and +3% crit",
            Self::TitanWard => "Titan Ward: +28 health and +10 armor",
            Self::ArcaneMomentum => "Arcane Momentum: +20 fury and cheaper skills",
        }
    }
}

#[derive(Component, Debug, Default)]
pub struct ChapterBoon {
    pub choice: ChapterBoonChoice,
    pub empowered: bool,
    pub momentum_stacks: u32,
}

impl ChapterBoon {
    pub fn summary(&self) -> String {
        if self.choice == ChapterBoonChoice::None {
            return ChapterBoonChoice::None.summary().to_string();
        }
        let summary = if self.empowered {
            format!("Empowered {}", self.choice.summary())
        } else {
            self.choice.summary().to_string()
        };
        if self.momentum_stacks > 0 {
            format!(
                "{summary} | Momentum {}x infused",
                self.momentum_stacks.min(RELIQUARY_MOMENTUM_MAX)
            )
        } else {
            summary
        }
    }
}

#[derive(Component)]
pub struct DamageBonus(pub f32);

#[derive(Component, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Charm {
    pub name: String,
    pub quality: String,
    pub damage_bonus: f32,
    pub crit_chance: f32,
}

impl Default for Charm {
    fn default() -> Self {
        Self {
            name: "Empty Charm Slot".to_string(),
            quality: "none".to_string(),
            damage_bonus: 0.0,
            crit_chance: 0.0,
        }
    }
}

impl Charm {
    pub fn summary(&self) -> String {
        if self.quality == "none" {
            return "Charm: empty".to_string();
        }
        format!(
            "Charm: {} {} +{:.0} dmg {:.0}% crit",
            self.quality,
            self.name,
            self.damage_bonus,
            self.crit_chance * 100.0
        )
    }
}

#[derive(Component, Debug, Clone, Copy, PartialEq)]
pub struct Fury {
    pub current: f32,
    pub max: f32,
    pub basic_gain: f32,
    pub dash_cost: f32,
    pub nova_cost: f32,
    pub rupture_cost: f32,
}

impl Fury {
    pub fn gain(&mut self, amount: f32) -> f32 {
        let before = self.current;
        self.current = (self.current + amount.max(0.0)).min(self.max);
        self.current - before
    }

    pub fn spend(&mut self, amount: f32) -> bool {
        if self.current < amount {
            return false;
        }
        self.current -= amount;
        true
    }
}

#[derive(Component)]
pub struct Equipment {
    pub weapon_name: String,
    pub quality: String,
    pub crit_chance: f32,
    pub health_bonus: f32,
    pub armor_bonus: f32,
    pub legendary_power: LegendaryPower,
    pub temper_level: u32,
    pub socketed_gem: Option<SocketedGem>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GemKind {
    Ruby,
    Emerald,
    Amethyst,
    Topaz,
}

impl GemKind {
    pub fn label(self) -> &'static str {
        match self {
            Self::Ruby => "Ruby",
            Self::Emerald => "Emerald",
            Self::Amethyst => "Amethyst",
            Self::Topaz => "Topaz",
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct SocketedGem {
    pub kind: GemKind,
    pub rank: u32,
}

impl SocketedGem {
    pub fn new(kind: GemKind) -> Self {
        Self { kind, rank: 1 }
    }

    pub fn label(self) -> String {
        format!("{} {}", self.kind.label(), self.rank)
    }

    fn resonance_tier(self) -> u32 {
        if self.rank >= 7 {
            3
        } else if self.rank >= 5 {
            2
        } else if self.rank >= 3 {
            1
        } else {
            0
        }
    }

    fn resonance_damage_bonus(self) -> f32 {
        match self.resonance_tier() {
            3 => 18.0,
            2 => 10.0,
            1 => 4.0,
            _ => 0.0,
        }
    }

    fn resonance_crit_bonus(self) -> f32 {
        match self.resonance_tier() {
            3 => 0.055,
            2 => 0.035,
            1 => 0.015,
            _ => 0.0,
        }
    }

    fn resonance_health_bonus(self) -> f32 {
        match self.resonance_tier() {
            3 => 60.0,
            2 => 36.0,
            1 => 16.0,
            _ => 0.0,
        }
    }

    fn resonance_armor_bonus(self) -> f32 {
        match self.resonance_tier() {
            3 => 34.0,
            2 => 20.0,
            1 => 8.0,
            _ => 0.0,
        }
    }

    pub fn resonance_label(self) -> &'static str {
        match self.resonance_tier() {
            3 => "Paragon",
            2 => "Ascendant",
            1 => "Resonant",
            _ => "Dormant",
        }
    }

    pub fn resonance_summary(self) -> String {
        if self.resonance_tier() == 0 {
            return "Gem Resonance: rank 3 unlocks Resonant power".to_string();
        }
        format!(
            "Gem Resonance: {} {}",
            self.resonance_label(),
            match self.kind {
                GemKind::Ruby => "ruby amplifies weapon damage",
                GemKind::Emerald => "emerald sharpens critical chance",
                GemKind::Amethyst => "amethyst hardens maximum health",
                GemKind::Topaz => "topaz reinforces armor",
            }
        )
    }

    pub fn damage_bonus(self) -> f32 {
        match self.kind {
            GemKind::Ruby => self.rank as f32 * 3.0 + self.resonance_damage_bonus(),
            _ => 0.0,
        }
    }

    pub fn crit_bonus(self) -> f32 {
        match self.kind {
            GemKind::Emerald => self.rank as f32 * 0.015 + self.resonance_crit_bonus(),
            _ => 0.0,
        }
    }

    pub fn health_bonus(self) -> f32 {
        match self.kind {
            GemKind::Amethyst => self.rank as f32 * 12.0 + self.resonance_health_bonus(),
            _ => 0.0,
        }
    }

    pub fn armor_bonus(self) -> f32 {
        match self.kind {
            GemKind::Topaz => self.rank as f32 * 6.0 + self.resonance_armor_bonus(),
            _ => 0.0,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LegendaryPower {
    #[default]
    None,
    Emberbrand,
    Frostbrand,
    Stormbrand,
    Soulreaver,
    Aegisbrand,
}

impl LegendaryPower {
    pub const ALL: [Self; 5] = [
        Self::Emberbrand,
        Self::Frostbrand,
        Self::Stormbrand,
        Self::Soulreaver,
        Self::Aegisbrand,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Self::None => "No power",
            Self::Emberbrand => "Emberbrand",
            Self::Frostbrand => "Frostbrand",
            Self::Stormbrand => "Stormbrand",
            Self::Soulreaver => "Soulreaver",
            Self::Aegisbrand => "Aegisbrand",
        }
    }

    pub fn summary(self) -> &'static str {
        match self {
            Self::None => "",
            Self::Emberbrand => "Emberbrand: crits or Ember/Hemorrhage runes ignite enemies",
            Self::Frostbrand => "Frostbrand: crits or Frost/Expose runes chill enemies",
            Self::Stormbrand => "Stormbrand: crits or Reap/Frost runes chain lightning",
            Self::Soulreaver => "Soulreaver: crits or Reap/Hemorrhage runes steal health",
            Self::Aegisbrand => "Aegisbrand: crits or Cleanse/Expose runes grant barrier",
        }
    }

    pub fn build_role(self) -> &'static str {
        match self {
            Self::None => "",
            Self::Emberbrand => "ember burn build",
            Self::Frostbrand => "control build",
            Self::Stormbrand => "chain clear build",
            Self::Soulreaver => "sustain build",
            Self::Aegisbrand => "ward tank build",
        }
    }

    pub fn boss_boon_key(self) -> &'static str {
        match self {
            Self::None => "best fit",
            Self::Emberbrand | Self::Stormbrand => "4 Crimson",
            Self::Frostbrand => "6 Arcane",
            Self::Soulreaver | Self::Aegisbrand => "5 Titan",
        }
    }

    pub fn boss_boon_route(self) -> &'static str {
        match self {
            Self::None => "choose the boon that fits current pressure",
            Self::Emberbrand | Self::Stormbrand => "pair with Crimson burst",
            Self::Frostbrand => "pair with Arcane uptime",
            Self::Soulreaver | Self::Aegisbrand => "pair with Titan safety",
        }
    }
}

#[derive(Component, Debug, Default, Clone, PartialEq, Serialize, Deserialize)]
pub struct LegendaryCodex {
    pub unlocked: Vec<LegendaryPower>,
    pub attuned: LegendaryPower,
}

impl LegendaryCodex {
    pub fn unlock(&mut self, power: LegendaryPower) -> bool {
        if power == LegendaryPower::None || self.unlocked.contains(&power) {
            return false;
        }
        self.unlocked.push(power);
        if self.attuned == LegendaryPower::None {
            self.attuned = power;
        }
        true
    }

    pub fn restore(&mut self, powers: Vec<LegendaryPower>) {
        self.unlocked.clear();
        self.attuned = LegendaryPower::None;
        for power in powers {
            self.unlock(power);
        }
    }

    pub fn restore_attunement(&mut self, power: LegendaryPower) {
        self.attuned = if self.unlocked.contains(&power) {
            power
        } else {
            self.unlocked
                .first()
                .copied()
                .unwrap_or(LegendaryPower::None)
        };
    }

    pub fn next_power_after(&self, current: LegendaryPower) -> Option<LegendaryPower> {
        if self.unlocked.is_empty() {
            return None;
        }
        let next_index = self
            .unlocked
            .iter()
            .position(|power| *power == current)
            .map(|index| (index + 1) % self.unlocked.len())
            .unwrap_or(0);
        self.unlocked.get(next_index).copied()
    }

    pub fn next_missing_power(&self) -> Option<LegendaryPower> {
        LegendaryPower::ALL
            .iter()
            .find(|power| !self.unlocked.contains(power))
            .copied()
    }

    pub fn enchant_target(&self, current: LegendaryPower) -> Option<LegendaryPower> {
        if self.attuned != LegendaryPower::None && self.attuned != current {
            return Some(self.attuned);
        }
        self.next_power_after(current)
            .filter(|power| *power != current || self.unlocked.len() > 1)
    }

    pub fn is_complete(&self) -> bool {
        LegendaryPower::ALL
            .iter()
            .all(|power| self.unlocked.contains(power))
    }

    pub fn cycle_attunement(&mut self) -> Option<LegendaryPower> {
        let next = self.next_power_after(self.attuned)?;
        self.attuned = next;
        Some(next)
    }

    pub fn summary(&self) -> String {
        if self.unlocked.is_empty() {
            return "Codex: no legendary powers".to_string();
        }
        format!(
            "Codex: {} | Attuned {}",
            self.unlocked
                .iter()
                .map(|power| power.label())
                .collect::<Vec<_>>()
                .join(", "),
            self.attuned.label()
        )
    }
}

pub fn legendary_codex_pursuit_summary(codex: &LegendaryCodex, equipment: &Equipment) -> String {
    if codex.unlocked.is_empty() {
        return "Codex next: find a legendary weapon power".to_string();
    }
    if let Some(power) = codex.next_missing_power() {
        return format!(
            "Codex next: unlock {} from legendary drops ({})",
            power.label(),
            power.build_role()
        );
    }
    if equipment.legendary_power == LegendaryPower::None
        && let Some(power) = codex.enchant_target(equipment.legendary_power)
    {
        return format!(
            "Codex next: enchant {} at quartermaster -> {} ({})",
            power.label(),
            power.boss_boon_key(),
            power.boss_boon_route()
        );
    }
    if codex.attuned != LegendaryPower::None && codex.attuned != equipment.legendary_power {
        return format!(
            "Codex next: enchant attuned {} at quartermaster -> {} ({})",
            codex.attuned.label(),
            codex.attuned.boss_boon_key(),
            codex.attuned.boss_boon_route()
        );
    }
    if let Some(power) = codex.enchant_target(equipment.legendary_power) {
        return format!(
            "Codex ready: B attune or enchant next {} -> {} ({})",
            power.label(),
            power.boss_boon_key(),
            power.boss_boon_route()
        );
    }
    "Codex complete: all powers mastered".to_string()
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct InventoryItem {
    pub name: String,
    pub quality: String,
    pub damage_bonus: f32,
    pub crit_chance: f32,
    pub health_bonus: f32,
    pub armor_bonus: f32,
    pub legendary_power: LegendaryPower,
    pub temper_level: u32,
    pub socketed_gem: Option<SocketedGem>,
}

#[derive(Component, Debug)]
pub struct Inventory {
    pub items: Vec<InventoryItem>,
    pub capacity: usize,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ArmoryLoadout {
    pub weapon: InventoryItem,
    pub charm: Charm,
    pub dash_rune: DashRune,
    pub nova_rune: NovaRune,
    pub rupture_rune: RuptureRune,
    pub codex_attunement: LegendaryPower,
}

pub const ARMORY_SLOT_COUNT: usize = 3;

#[derive(Component, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Armory {
    pub loadouts: Vec<Option<ArmoryLoadout>>,
    pub active_slot: usize,
}

impl Default for Armory {
    fn default() -> Self {
        Self::new()
    }
}

impl Armory {
    pub fn new() -> Self {
        Self {
            loadouts: vec![None; ARMORY_SLOT_COUNT],
            active_slot: 0,
        }
    }

    pub fn normalize(&mut self) {
        if self.loadouts.len() < ARMORY_SLOT_COUNT {
            self.loadouts.resize_with(ARMORY_SLOT_COUNT, || None);
        } else if self.loadouts.len() > ARMORY_SLOT_COUNT {
            self.loadouts.truncate(ARMORY_SLOT_COUNT);
        }
        self.active_slot = self.active_slot.min(ARMORY_SLOT_COUNT - 1);
    }

    pub fn select_slot(&mut self, slot: usize) {
        self.normalize();
        self.active_slot = slot.min(ARMORY_SLOT_COUNT - 1);
    }

    pub fn save_active(&mut self, loadout: ArmoryLoadout) {
        self.normalize();
        self.loadouts[self.active_slot] = Some(loadout);
    }

    pub fn active_loadout(&self) -> Option<&ArmoryLoadout> {
        self.loadouts
            .get(self.active_slot.min(ARMORY_SLOT_COUNT - 1))
            .and_then(Option::as_ref)
    }

    pub fn legacy_loadout(&self) -> Option<ArmoryLoadout> {
        self.loadouts.first().and_then(Clone::clone)
    }

    pub fn summary(&self) -> String {
        let slot = self.active_slot.min(ARMORY_SLOT_COUNT - 1);
        let filled = armory_filled_slots(self);
        match self.loadouts.get(slot).and_then(Option::as_ref) {
            Some(loadout) => format!(
                "Armory {}: {} {} | {} charm | {} codex | {}/{} slots",
                slot + 1,
                loadout.weapon.quality,
                loadout.weapon.name,
                loadout.charm.quality,
                loadout.codex_attunement.label(),
                filled,
                ARMORY_SLOT_COUNT
            ),
            None => format!(
                "Armory {}: empty | {}/{} slots",
                slot + 1,
                filled,
                ARMORY_SLOT_COUNT
            ),
        }
    }
}

pub fn armory_filled_slots(armory: &Armory) -> usize {
    armory
        .loadouts
        .iter()
        .take(ARMORY_SLOT_COUNT)
        .filter(|loadout| loadout.is_some())
        .count()
}

impl Inventory {
    pub fn add(&mut self, item: InventoryItem) -> bool {
        if self.items.len() >= self.capacity {
            return false;
        }
        self.items.push(item);
        true
    }

    #[cfg_attr(not(test), allow(dead_code))]
    pub fn summary(&self) -> String {
        if self.items.is_empty() {
            return "Inventory empty".to_string();
        }
        self.items
            .iter()
            .rev()
            .take(6)
            .map(|item| {
                let power = item.legendary_power.summary();
                let suffix = if power.is_empty() {
                    String::new()
                } else {
                    format!(" | {power}")
                };
                let temper = temper_label(item.temper_level);
                let gem = gem_label(item.socketed_gem);
                format!(
                    "{} {}{}{} +{:.0} dmg {:.0}% crit +{:.0} hp +{:.0} armor",
                    item.quality,
                    item.name,
                    temper,
                    gem,
                    item.damage_bonus,
                    item.crit_chance * 100.0,
                    item.health_bonus,
                    item.armor_bonus
                ) + &suffix
            })
            .collect::<Vec<_>>()
            .join(" | ")
    }
}

pub fn inventory_swap_index(
    inventory: &Inventory,
    current_weapon_name: &str,
    offset: i32,
) -> Option<usize> {
    let len = inventory.items.len();
    if len == 0 {
        return None;
    }
    let current = inventory
        .items
        .iter()
        .position(|item| item.name == current_weapon_name)
        .unwrap_or(0);
    Some((current as i32 + offset).rem_euclid(len as i32) as usize)
}

pub fn equip_inventory_item(
    item: &InventoryItem,
    damage_bonus: &mut DamageBonus,
    equipment: &mut Equipment,
    health: &mut Health,
) {
    let old_health_bonus = equipment.health_bonus;
    damage_bonus.0 = item.damage_bonus;
    equipment.weapon_name = item.name.clone();
    equipment.quality = item.quality.clone();
    equipment.crit_chance = item.crit_chance;
    equipment.health_bonus = item.health_bonus;
    equipment.armor_bonus = item.armor_bonus;
    equipment.legendary_power = item.legendary_power;
    equipment.temper_level = item.temper_level;
    equipment.socketed_gem = item.socketed_gem;
    apply_equipment_health_delta(health, old_health_bonus, item.health_bonus);
}

pub fn manual_equip_summary(
    item: &InventoryItem,
    damage_bonus: &DamageBonus,
    equipment: &Equipment,
) -> String {
    let current = EquipCompareStats::from_equipment(damage_bonus, equipment).score();
    let next = EquipCompareStats::from_inventory_item(item).score();
    let delta = next - current;
    let decision = if delta >= 8.0 {
        "UPGRADE"
    } else if item.legendary_power != equipment.legendary_power
        && item.legendary_power != LegendaryPower::None
    {
        "BUILD SWAP"
    } else if delta <= -8.0 {
        "DOWNGRADE"
    } else {
        "SIDEGRADE"
    };
    let power = if item.legendary_power != equipment.legendary_power {
        if item.legendary_power == LegendaryPower::None {
            "power lost".to_string()
        } else {
            format!(
                "power {} {}",
                item.legendary_power.label(),
                item.legendary_power.build_role()
            )
        }
    } else if item.legendary_power == LegendaryPower::None {
        "no legendary power".to_string()
    } else {
        format!("power kept {}", item.legendary_power.label())
    };
    let socket = gem_label(item.socketed_gem);
    let socket = if socket.is_empty() {
        String::new()
    } else {
        format!(" | socket{}", socket)
    };
    let temper = temper_label(item.temper_level);
    let temper = if temper.is_empty() {
        String::new()
    } else {
        format!(" | temper{}", temper)
    };
    format!(
        "Equipped {decision}: {} {} | PWR {:+.0} | dmg {:+.0} crit {:+.0}% tough {:+.0} | {power}{socket}{temper}",
        item.quality,
        item.name,
        delta,
        item.damage_bonus - damage_bonus.0,
        (item.crit_chance - equipment.crit_chance) * 100.0,
        item_toughness(item.health_bonus, item.armor_bonus)
            - item_toughness(equipment.health_bonus, equipment.armor_bonus),
    )
}

#[derive(Debug, Clone, Copy)]
struct EquipCompareStats<'a> {
    quality: &'a str,
    damage_bonus: f32,
    crit_chance: f32,
    health_bonus: f32,
    armor_bonus: f32,
    legendary_power: LegendaryPower,
    temper_level: u32,
    socketed_gem: Option<SocketedGem>,
}

impl<'a> EquipCompareStats<'a> {
    fn from_equipment(damage_bonus: &DamageBonus, equipment: &'a Equipment) -> Self {
        Self {
            quality: &equipment.quality,
            damage_bonus: damage_bonus.0,
            crit_chance: equipment.crit_chance,
            health_bonus: equipment.health_bonus,
            armor_bonus: equipment.armor_bonus,
            legendary_power: equipment.legendary_power,
            temper_level: equipment.temper_level,
            socketed_gem: equipment.socketed_gem,
        }
    }

    fn from_inventory_item(item: &'a InventoryItem) -> Self {
        Self {
            quality: &item.quality,
            damage_bonus: item.damage_bonus,
            crit_chance: item.crit_chance,
            health_bonus: item.health_bonus,
            armor_bonus: item.armor_bonus,
            legendary_power: item.legendary_power,
            temper_level: item.temper_level,
            socketed_gem: item.socketed_gem,
        }
    }

    fn score(self) -> f32 {
        self.damage_bonus
            + self.crit_chance * 90.0
            + item_toughness(self.health_bonus, self.armor_bonus)
            + quality_compare_bonus(self.quality)
            + if self.legendary_power == LegendaryPower::None {
                0.0
            } else {
                14.0
            }
            + self.temper_level as f32 * 1.6
            + self
                .socketed_gem
                .map(socketed_gem_compare_bonus)
                .unwrap_or_default()
    }
}

fn item_toughness(health_bonus: f32, armor_bonus: f32) -> f32 {
    health_bonus * 0.16 + armor_bonus * 0.46
}

fn quality_compare_bonus(quality: &str) -> f32 {
    match quality {
        "primal" => 34.0,
        "ancient" => 25.0,
        "legendary" => 16.0,
        "rare" => 6.0,
        _ => 0.0,
    }
}

fn socketed_gem_compare_bonus(gem: SocketedGem) -> f32 {
    gem.rank as f32 * 2.2
        + match gem.kind {
            GemKind::Ruby => gem.damage_bonus() * 0.35,
            GemKind::Emerald => gem.crit_bonus() * 140.0,
            GemKind::Amethyst => gem.health_bonus() * 0.10,
            GemKind::Topaz => gem.armor_bonus() * 0.28,
        }
}

pub fn equipment_as_inventory_item(
    damage_bonus: &DamageBonus,
    equipment: &Equipment,
) -> InventoryItem {
    InventoryItem {
        name: equipment.weapon_name.clone(),
        quality: equipment.quality.clone(),
        damage_bonus: damage_bonus.0,
        crit_chance: equipment.crit_chance,
        health_bonus: equipment.health_bonus,
        armor_bonus: equipment.armor_bonus,
        legendary_power: equipment.legendary_power,
        temper_level: equipment.temper_level,
        socketed_gem: equipment.socketed_gem,
    }
}

pub fn capture_armory_loadout(
    damage_bonus: &DamageBonus,
    equipment: &Equipment,
    inventory: &Inventory,
    charm: &Charm,
    runes: &SkillRunes,
    codex: &LegendaryCodex,
) -> ArmoryLoadout {
    let weapon = inventory
        .items
        .iter()
        .find(|item| item.name == equipment.weapon_name)
        .cloned()
        .unwrap_or_else(|| equipment_as_inventory_item(damage_bonus, equipment));
    ArmoryLoadout {
        weapon,
        charm: charm.clone(),
        dash_rune: runes.dash,
        nova_rune: runes.nova,
        rupture_rune: runes.rupture,
        codex_attunement: codex.attuned,
    }
}

pub struct ArmoryRestoreTargets<'a> {
    pub damage_bonus: &'a mut DamageBonus,
    pub equipment: &'a mut Equipment,
    pub inventory: &'a mut Inventory,
    pub health: &'a mut Health,
    pub charm: &'a mut Charm,
    pub runes: &'a mut SkillRunes,
    pub codex: &'a mut LegendaryCodex,
}

pub fn restore_armory_loadout(loadout: &ArmoryLoadout, targets: ArmoryRestoreTargets) {
    if !targets
        .inventory
        .items
        .iter()
        .any(|item| item.name == loadout.weapon.name)
    {
        let _ = targets.inventory.add(loadout.weapon.clone());
    }
    equip_inventory_item(
        &loadout.weapon,
        targets.damage_bonus,
        targets.equipment,
        targets.health,
    );
    *targets.charm = loadout.charm.clone();
    targets.runes.dash = loadout.dash_rune;
    targets.runes.nova = loadout.nova_rune;
    targets.runes.rupture = loadout.rupture_rune;
    targets.codex.restore_attunement(loadout.codex_attunement);
}

pub fn charm_power(charm: &Charm) -> f32 {
    let quality_multiplier = match charm.quality.as_str() {
        "primal" => 4.8,
        "ancient" => 3.8,
        "legendary" => 3.0,
        "rare" => 2.0,
        "none" => 0.0,
        _ => 1.0,
    };
    (charm.damage_bonus * 2.4 + charm.crit_chance * 120.0 + 6.0) * quality_multiplier
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ReliquaryResonance {
    pub label: &'static str,
    pub damage_bonus: f32,
    pub crit_bonus: f32,
}

impl ReliquaryResonance {
    pub fn summary(self) -> String {
        format!(
            "Resonance: {} +{:.0} dmg +{:.0}% crit",
            self.label,
            self.damage_bonus,
            self.crit_bonus * 100.0
        )
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReliquarySet {
    Storm,
    Blood,
    Ashen,
    Reliquary,
}

impl ReliquarySet {
    pub const ALL: [Self; 4] = [Self::Storm, Self::Blood, Self::Ashen, Self::Reliquary];

    pub fn label(self) -> &'static str {
        match self {
            Self::Storm => "Storm Covenant",
            Self::Blood => "Blood Pact",
            Self::Ashen => "Ashen Saint",
            Self::Reliquary => "Reliquary Set",
        }
    }

    pub fn mask(self) -> u8 {
        match self {
            Self::Storm => 1 << 0,
            Self::Blood => 1 << 1,
            Self::Ashen => 1 << 2,
            Self::Reliquary => 1 << 3,
        }
    }
}

pub fn reliquary_resonance(equipment: &Equipment, charm: &Charm) -> Option<ReliquaryResonance> {
    if let Some(resonance) = themed_reliquary_resonance(equipment, charm) {
        return Some(resonance);
    }
    match (equipment.quality.as_str(), charm.quality.as_str()) {
        ("primal", "primal") => Some(ReliquaryResonance {
            label: "Primal Concord",
            damage_bonus: 10.0,
            crit_bonus: 0.065,
        }),
        ("primal", "ancient")
        | ("ancient", "primal")
        | ("primal", "legendary")
        | ("legendary", "primal")
        | ("primal", "rare")
        | ("rare", "primal") => Some(ReliquaryResonance {
            label: "Primal Accord",
            damage_bonus: 8.5,
            crit_bonus: 0.055,
        }),
        ("ancient", "ancient") => Some(ReliquaryResonance {
            label: "Ancient Concord",
            damage_bonus: 7.5,
            crit_bonus: 0.050,
        }),
        ("ancient", "legendary")
        | ("legendary", "ancient")
        | ("ancient", "rare")
        | ("rare", "ancient") => Some(ReliquaryResonance {
            label: "Ancient Accord",
            damage_bonus: 6.0,
            crit_bonus: 0.040,
        }),
        ("legendary", "legendary") => Some(ReliquaryResonance {
            label: "Ashen Harmony",
            damage_bonus: 4.0,
            crit_bonus: 0.030,
        }),
        ("legendary", "rare") | ("rare", "legendary") => Some(ReliquaryResonance {
            label: "Relic Accord",
            damage_bonus: 2.5,
            crit_bonus: 0.018,
        }),
        ("rare", "rare") => Some(ReliquaryResonance {
            label: "Gilded Concord",
            damage_bonus: 1.5,
            crit_bonus: 0.012,
        }),
        _ => None,
    }
}

pub fn is_themed_reliquary_resonance(equipment: &Equipment, charm: &Charm) -> bool {
    themed_reliquary_set(equipment, charm).is_some()
}

pub fn themed_reliquary_set(equipment: &Equipment, charm: &Charm) -> Option<ReliquarySet> {
    if !gear_quality_can_resonate(&equipment.quality) || !gear_quality_can_resonate(&charm.quality)
    {
        return None;
    }
    let weapon_set = gear_affinity(&equipment.weapon_name)?;
    if weapon_set != gear_affinity(&charm.name)? {
        return None;
    }
    Some(weapon_set)
}

pub fn gear_quality_is_ancient_or_better(quality: &str) -> bool {
    matches!(quality, "ancient" | "primal")
}

pub fn gear_quality_is_legendary_or_better(quality: &str) -> bool {
    matches!(quality, "legendary" | "ancient" | "primal")
}

fn themed_reliquary_resonance(equipment: &Equipment, charm: &Charm) -> Option<ReliquaryResonance> {
    Some(match themed_reliquary_set(equipment, charm)? {
        ReliquarySet::Storm => ReliquaryResonance {
            label: ReliquarySet::Storm.label(),
            damage_bonus: 5.5,
            crit_bonus: 0.040,
        },
        ReliquarySet::Blood => ReliquaryResonance {
            label: ReliquarySet::Blood.label(),
            damage_bonus: 6.0,
            crit_bonus: 0.030,
        },
        ReliquarySet::Ashen => ReliquaryResonance {
            label: ReliquarySet::Ashen.label(),
            damage_bonus: 4.5,
            crit_bonus: 0.045,
        },
        ReliquarySet::Reliquary => ReliquaryResonance {
            label: ReliquarySet::Reliquary.label(),
            damage_bonus: 5.0,
            crit_bonus: 0.035,
        },
    })
}

fn gear_quality_can_resonate(quality: &str) -> bool {
    matches!(quality, "rare" | "legendary" | "ancient" | "primal")
}

fn gear_affinity(name: &str) -> Option<ReliquarySet> {
    let lower = name.to_ascii_lowercase();
    if lower.contains("storm") {
        Some(ReliquarySet::Storm)
    } else if lower.contains("blood") || lower.contains("soulreaver") {
        Some(ReliquarySet::Blood)
    } else if lower.contains("ashen") || lower.contains("ember-saint") {
        Some(ReliquarySet::Ashen)
    } else if lower.contains("reliquary") {
        Some(ReliquarySet::Reliquary)
    } else {
        None
    }
}

pub fn total_damage_bonus(damage_bonus: &DamageBonus, equipment: &Equipment, charm: &Charm) -> f32 {
    damage_bonus.0
        + charm.damage_bonus
        + reliquary_resonance(equipment, charm)
            .map(|resonance| resonance.damage_bonus)
            .unwrap_or_default()
}

pub fn total_crit_chance(equipment: &Equipment, charm: &Charm) -> f32 {
    (equipment.crit_chance
        + charm.crit_chance
        + reliquary_resonance(equipment, charm)
            .map(|resonance| resonance.crit_bonus)
            .unwrap_or_default())
    .min(0.50)
}

pub fn gem_label(gem: Option<SocketedGem>) -> String {
    gem.map(|gem| {
        if gem.resonance_tier() > 0 {
            format!(" [{} {}]", gem.label(), gem.resonance_label())
        } else {
            format!(" [{}]", gem.label())
        }
    })
    .unwrap_or_default()
}

pub fn temper_label(level: u32) -> String {
    if level == 0 {
        String::new()
    } else {
        format!(" +{level}")
    }
}

fn apply_equipment_health_delta(health: &mut Health, old_bonus: f32, new_bonus: f32) {
    let delta = new_bonus - old_bonus;
    health.max = (health.max + delta).max(1.0);
    if delta > 0.0 {
        health.current = (health.current + delta).min(health.max);
    } else {
        health.current = health.current.min(health.max);
    }
}

#[derive(Component)]
pub struct PlayerLevel {
    pub level: u32,
    pub xp: u32,
    pub next_level_xp: u32,
}

#[derive(Component, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EmberParagon {
    pub rank: u32,
    pub xp: u32,
    pub next_rank_xp: u32,
}

impl Default for EmberParagon {
    fn default() -> Self {
        Self {
            rank: 0,
            xp: 0,
            next_rank_xp: EMBER_PARAGON_BASE_XP,
        }
    }
}

impl EmberParagon {
    pub fn summary(&self) -> String {
        format!(
            "Ember Paragon {} ({}/{})",
            self.rank, self.xp, self.next_rank_xp
        )
    }
}

#[derive(Component, Debug, Default)]
pub struct Talents {
    pub points: u32,
    pub wrath: u32,
    pub vigor: u32,
    pub focus: u32,
}

impl Talents {
    pub fn damage_multiplier(&self) -> f32 {
        1.0 + self.wrath as f32 * 0.08
    }

    pub fn cooldown_multiplier(&self) -> f32 {
        (1.0 - self.focus as f32 * 0.06).max(0.7)
    }

    pub fn summary(&self) -> String {
        format!(
            "Talent points: {} | 1 Wrath {} (+dmg) | 2 Vigor {} (+hp) | 3 Focus {} (-cd)",
            self.points, self.wrath, self.vigor, self.focus
        )
    }
}

#[derive(Component)]
pub struct PotionBelt {
    pub current: u32,
    pub max: u32,
    pub heal_amount: f32,
    pub cooldown: Timer,
    pub cooldown_secs: f32,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq, Serialize, Deserialize, Default)]
pub enum ElixirKind {
    #[default]
    Iron,
    Wrath,
    Haste,
}

impl ElixirKind {
    pub fn next(self) -> Self {
        match self {
            Self::Iron => Self::Wrath,
            Self::Wrath => Self::Haste,
            Self::Haste => Self::Iron,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Iron => "Iron",
            Self::Wrath => "Wrath",
            Self::Haste => "Haste",
        }
    }
}

#[derive(Component)]
pub struct ElixirBelt {
    pub current: u32,
    pub max: u32,
    pub armor_bonus: f32,
    pub duration_secs: f32,
    pub selected: ElixirKind,
}

#[derive(Component)]
pub struct RelicBuff {
    pub timer: Timer,
    pub damage_multiplier: f32,
    pub speed_multiplier: f32,
}

#[derive(Component)]
pub struct ElixirBuff {
    pub timer: Timer,
    pub kind: ElixirKind,
    pub armor_bonus: f32,
    pub damage_multiplier: f32,
    pub speed_multiplier: f32,
}

#[derive(Component)]
pub struct FortuneBuff {
    pub timer: Timer,
    pub xp_multiplier: f32,
    pub gold_multiplier: f32,
}

#[derive(Component)]
pub struct ConduitBuff {
    pub timer: Timer,
    pub pulse: Timer,
    pub damage: f32,
    pub radius: f32,
}

#[derive(Component)]
pub struct GloryBuff {
    pub timer: Timer,
    pub stacks: u32,
    pub max_stacks: u32,
    pub damage_per_stack: f32,
    pub speed_per_stack: f32,
}

#[derive(Component)]
pub struct SurgeBuff {
    pub timer: Timer,
    pub charge: u32,
    pub max_charge: u32,
    pub duration_secs: f32,
    pub damage_multiplier: f32,
    pub speed_multiplier: f32,
    pub fury_multiplier: f32,
}

#[derive(Component)]
pub struct TownPortal {
    pub cooldown: Timer,
    pub destination: Vec3,
    pub return_position: Option<Vec3>,
}

#[derive(Component, Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct DeathWard {
    pub checkpoint: Vec3,
    pub revives_used: u32,
    pub max_revives: u32,
    pub gold_penalty: u32,
}

impl DeathWard {
    pub fn remaining_revives(&self) -> u32 {
        self.max_revives.saturating_sub(self.revives_used)
    }

    pub fn can_revive(&self, gold: u32) -> bool {
        self.remaining_revives() > 0 && gold >= self.gold_penalty
    }
}

#[derive(Resource, Debug, Default, Clone, PartialEq, Eq)]
pub struct DeathWardReport {
    pub revives_used: u32,
    pub max_revives: u32,
    pub gold_penalty: u32,
    pub gold_available: u32,
    pub reason: String,
}

impl DeathWardReport {
    pub fn summary(&self) -> String {
        if self.reason.is_empty() {
            return "Soul ward ready".to_string();
        }
        format!(
            "Soul ward failed: {} (revives {}/{}, gold {}/{})",
            self.reason,
            self.revives_used,
            self.max_revives,
            self.gold_available,
            self.gold_penalty
        )
    }
}

pub fn death_ward_failure_reason(ward: &DeathWard, gold: u32) -> &'static str {
    if ward.remaining_revives() == 0 {
        "revive charges exhausted"
    } else if gold < ward.gold_penalty {
        "not enough gold for the revive tithe"
    } else {
        "lethal damage overwhelmed the ward"
    }
}

#[derive(Component)]
pub struct Burning {
    pub dps: f32,
    pub timer: Timer,
}

#[derive(Component)]
pub struct Jailed {
    pub timer: Timer,
    pub speed_multiplier: f32,
}

pub fn mitigated_damage(raw_damage: f32, armor: f32) -> f32 {
    let armor = armor.max(0.0);
    (raw_damage * 100.0 / (100.0 + armor)).max(raw_damage * 0.25)
}

pub fn active_elixir_armor(buff: &ElixirBuff) -> f32 {
    if buff_timer_active(&buff.timer) {
        buff.armor_bonus
    } else {
        0.0
    }
}

pub fn total_armor(equipment: &Equipment, elixir: &ElixirBuff) -> f32 {
    equipment.armor_bonus + active_elixir_armor(elixir)
}

pub fn elixir_damage_multiplier(buff: &ElixirBuff) -> f32 {
    if buff_timer_active(&buff.timer) {
        buff.damage_multiplier
    } else {
        1.0
    }
}

pub fn elixir_speed_multiplier(buff: &ElixirBuff) -> f32 {
    if buff_timer_active(&buff.timer) {
        buff.speed_multiplier
    } else {
        1.0
    }
}

pub fn elixir_seconds_remaining(buff: &ElixirBuff) -> f32 {
    if buff_timer_active(&buff.timer) {
        buff.timer.remaining_secs()
    } else {
        0.0
    }
}

pub fn configure_elixir_buff(kind: ElixirKind, armor_bonus: f32, buff: &mut ElixirBuff) {
    buff.kind = kind;
    buff.armor_bonus = 0.0;
    buff.damage_multiplier = 1.0;
    buff.speed_multiplier = 1.0;
    match kind {
        ElixirKind::Iron => buff.armor_bonus = armor_bonus,
        ElixirKind::Wrath => buff.damage_multiplier = 1.18,
        ElixirKind::Haste => buff.speed_multiplier = 1.20,
    }
}

pub fn fortune_seconds_remaining(buff: &FortuneBuff) -> f32 {
    if buff_timer_active(&buff.timer) {
        buff.timer.remaining_secs()
    } else {
        0.0
    }
}

pub fn fortune_xp_reward(base_xp: u32, buff: &FortuneBuff) -> u32 {
    if buff_timer_active(&buff.timer) {
        ((base_xp as f32) * buff.xp_multiplier).round() as u32
    } else {
        base_xp
    }
}

pub fn fortune_gold_reward(base_gold: u32, buff: &FortuneBuff) -> u32 {
    if buff_timer_active(&buff.timer) {
        ((base_gold as f32) * buff.gold_multiplier).round() as u32
    } else {
        base_gold
    }
}

pub fn fortune_magic_find_active(buff: &FortuneBuff) -> bool {
    buff_timer_active(&buff.timer)
}

pub fn grant_glory(buff: &mut GloryBuff) -> u32 {
    buff.stacks = (buff.stacks + 1).min(buff.max_stacks);
    buff.timer = Timer::from_seconds(16.0, TimerMode::Once);
    buff.stacks
}

pub fn glory_damage_multiplier(buff: &GloryBuff) -> f32 {
    if buff_timer_active(&buff.timer) {
        1.0 + buff.damage_per_stack * buff.stacks as f32
    } else {
        1.0
    }
}

pub fn glory_speed_multiplier(buff: &GloryBuff) -> f32 {
    if buff_timer_active(&buff.timer) {
        1.0 + buff.speed_per_stack * buff.stacks as f32
    } else {
        1.0
    }
}

pub fn glory_seconds_remaining(buff: &GloryBuff) -> f32 {
    if buff_timer_active(&buff.timer) {
        buff.timer.remaining_secs()
    } else {
        0.0
    }
}

pub fn surge_damage_multiplier(buff: &SurgeBuff) -> f32 {
    if buff_timer_active(&buff.timer) {
        buff.damage_multiplier
    } else {
        1.0
    }
}

pub fn surge_speed_multiplier(buff: &SurgeBuff) -> f32 {
    if buff_timer_active(&buff.timer) {
        buff.speed_multiplier
    } else {
        1.0
    }
}

pub fn surge_fury_multiplier(buff: &SurgeBuff) -> f32 {
    if buff_timer_active(&buff.timer) {
        buff.fury_multiplier
    } else {
        1.0
    }
}

pub fn surge_seconds_remaining(buff: &SurgeBuff) -> f32 {
    if buff_timer_active(&buff.timer) {
        buff.timer.remaining_secs()
    } else {
        0.0
    }
}

pub fn grant_surge_charge(buff: &mut SurgeBuff, amount: u32) -> u32 {
    let before = buff.charge;
    buff.charge = buff.charge.saturating_add(amount).min(buff.max_charge);
    buff.charge - before
}

pub fn activate_surge(buff: &mut SurgeBuff) -> bool {
    if buff.charge < buff.max_charge {
        return false;
    }
    buff.charge = 0;
    buff.timer = Timer::from_seconds(buff.duration_secs, TimerMode::Once);
    true
}

pub fn extend_active_surge(buff: &mut SurgeBuff, kill_weight: u32) -> Option<f32> {
    if kill_weight == 0 || !buff_timer_active(&buff.timer) {
        return None;
    }

    let before = buff.timer.remaining_secs();
    let extension = kill_weight as f32 * 1.25;
    let cap = buff.duration_secs + 6.0;
    let after = (before + extension).min(cap);
    buff.timer = Timer::from_seconds(after, TimerMode::Once);

    let gained = after - before;
    (gained > 0.0).then_some(gained)
}

pub fn ashen_pylon_active(relic: &RelicBuff) -> bool {
    buff_timer_active(&relic.timer) && relic.damage_multiplier >= 1.55
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ReliquaryMomentumReward {
    pub stacks: u32,
    pub fury_gained: f32,
    pub damage_multiplier: f32,
    pub speed_multiplier: f32,
    pub duration_secs: f32,
}

pub fn reliquary_momentum_damage_multiplier(stacks: u32) -> f32 {
    1.0 + stacks.min(RELIQUARY_MOMENTUM_MAX) as f32 * 0.08
}

pub fn reliquary_momentum_speed_multiplier(stacks: u32) -> f32 {
    1.0 + stacks.min(RELIQUARY_MOMENTUM_MAX) as f32 * 0.04
}

pub fn reliquary_momentum_duration_secs(stacks: u32) -> f32 {
    18.0 + stacks.min(RELIQUARY_MOMENTUM_MAX) as f32 * 3.0
}

pub fn reliquary_boon_momentum_stacks(stats: &RunStats) -> u32 {
    stats.reliquary_momentum.min(RELIQUARY_MOMENTUM_MAX)
}

pub fn reliquary_boon_momentum_ready(stats: &RunStats) -> bool {
    reliquary_boon_momentum_stacks(stats) >= RELIQUARY_BOON_MOMENTUM_THRESHOLD
}

pub fn grant_reliquary_momentum(
    stats: &mut RunStats,
    relic: &mut RelicBuff,
    fury: &mut Fury,
    amount: u32,
) -> ReliquaryMomentumReward {
    stats.reliquary_momentum = stats
        .reliquary_momentum
        .saturating_add(amount.max(1))
        .min(RELIQUARY_MOMENTUM_MAX);
    stats.best_reliquary_momentum = stats.best_reliquary_momentum.max(stats.reliquary_momentum);

    let stacks = stats.reliquary_momentum;
    let damage_multiplier = reliquary_momentum_damage_multiplier(stacks);
    let speed_multiplier = reliquary_momentum_speed_multiplier(stacks);
    let duration_secs = reliquary_momentum_duration_secs(stacks);
    let active_remaining = if buff_timer_active(&relic.timer) {
        relic.timer.remaining_secs()
    } else {
        0.0
    };

    relic.timer = Timer::from_seconds(duration_secs.max(active_remaining), TimerMode::Once);
    relic.damage_multiplier = relic.damage_multiplier.max(damage_multiplier);
    relic.speed_multiplier = relic.speed_multiplier.max(speed_multiplier);

    let fury_gained = fury.gain(10.0 + amount.max(1) as f32 * 8.0 + stacks as f32 * 3.0);
    ReliquaryMomentumReward {
        stacks,
        fury_gained,
        damage_multiplier,
        speed_multiplier,
        duration_secs,
    }
}

pub fn reliquary_momentum_summary(stats: &RunStats) -> String {
    if stats.reliquary_momentum == 0 {
        return "Momentum: none".to_string();
    }
    format!(
        "Momentum: {}/{} +{:.0}% dmg +{:.0}% speed",
        stats.reliquary_momentum,
        RELIQUARY_MOMENTUM_MAX,
        (reliquary_momentum_damage_multiplier(stats.reliquary_momentum) - 1.0) * 100.0,
        (reliquary_momentum_speed_multiplier(stats.reliquary_momentum) - 1.0) * 100.0
    )
}

pub fn town_portal_seconds_remaining(portal: &TownPortal) -> f32 {
    if buff_timer_active(&portal.cooldown) {
        portal.cooldown.remaining_secs()
    } else {
        0.0
    }
}

pub fn potion_cooldown_secs_for_capacity(max_potions: u32) -> f32 {
    (12.0 - max_potions.saturating_sub(5) as f32 * 1.5).max(8.0)
}

pub fn potion_seconds_remaining(potions: &PotionBelt) -> f32 {
    if buff_timer_active(&potions.cooldown) {
        potions.cooldown.remaining_secs()
    } else {
        0.0
    }
}

pub fn town_portal_target(portal: &TownPortal, current_position: Vec3) -> (Vec3, Option<Vec3>) {
    if let Some(return_position) = portal.return_position
        && current_position.distance(portal.destination) <= 2.6
    {
        return (return_position, None);
    }
    (portal.destination, Some(current_position))
}

pub fn apply_chapter_boon(
    choice: ChapterBoonChoice,
    empowered: bool,
    momentum_stacks: u32,
    damage_bonus: &mut DamageBonus,
    equipment: &mut Equipment,
    health: &mut Health,
    fury: &mut Fury,
) {
    let momentum_stacks = momentum_stacks.min(RELIQUARY_MOMENTUM_MAX);
    match choice {
        ChapterBoonChoice::None => {}
        ChapterBoonChoice::CrimsonOath => {
            damage_bonus.0 += if empowered { 10.0 } else { 6.0 };
            equipment.crit_chance =
                (equipment.crit_chance + if empowered { 0.05 } else { 0.03 }).min(0.45);
        }
        ChapterBoonChoice::TitanWard => {
            let health_gain = if empowered { 42.0 } else { 28.0 };
            equipment.health_bonus += health_gain;
            equipment.armor_bonus += if empowered { 16.0 } else { 10.0 };
            health.max += health_gain;
            health.current = (health.current + health_gain).min(health.max);
        }
        ChapterBoonChoice::ArcaneMomentum => {
            let momentum_bonus = momentum_stacks as f32;
            let fury_gain = if empowered { 30.0 } else { 20.0 } + momentum_bonus * 4.0;
            fury.max += fury_gain;
            fury.current = (fury.current + fury_gain).min(fury.max);
            fury.basic_gain += if empowered { 6.0 } else { 4.0 } + momentum_bonus;
            fury.dash_cost =
                (fury.dash_cost - (if empowered { 8.0 } else { 5.0 } + momentum_bonus)).max(5.0);
            fury.nova_cost = (fury.nova_cost
                - (if empowered { 12.0 } else { 8.0 } + momentum_bonus * 2.0))
                .max(10.0);
            fury.rupture_cost =
                (fury.rupture_cost - (if empowered { 9.0 } else { 6.0 } + momentum_bonus)).max(8.0);
        }
    }
}

pub fn chapter_boon_is_empowered(choice: ChapterBoonChoice, stats: &RunStats) -> bool {
    match choice {
        ChapterBoonChoice::None => false,
        ChapterBoonChoice::CrimsonOath => stats.best_massacre_streak >= 8 || stats.kills >= 18,
        ChapterBoonChoice::TitanWard => {
            stats.breakables_smashed >= 4 || stats.shrine_resonance_triggered
        }
        ChapterBoonChoice::ArcaneMomentum => {
            stats.surge_kills >= SURGE_CHAIN_KILLS
                || stats.pylon_kills >= PYLON_REAPER_KILLS
                || stats.echo_keystones > 0
                || reliquary_boon_momentum_ready(stats)
        }
    }
}

pub fn chapter_boon_choice_prompt(stats: &RunStats) -> String {
    let state = |choice| {
        if chapter_boon_is_empowered(choice, stats) {
            "empowered"
        } else {
            "base"
        }
    };
    format!(
        "Boon: 4 Crimson {} | 5 Titan {} | 6 Arcane {} | {}",
        state(ChapterBoonChoice::CrimsonOath),
        state(ChapterBoonChoice::TitanWard),
        state(ChapterBoonChoice::ArcaneMomentum),
        chapter_boon_route_recommendation(stats)
    )
}

fn chapter_boon_route_recommendation(stats: &RunStats) -> &'static str {
    if reliquary_boon_momentum_ready(stats) {
        "Pick 6 fury/skill route"
    } else if stats.best_massacre_streak >= 8
        || stats.kills >= 18
        || stats.set_resonance_kills >= SET_ADEPT_KILLS / 2
    {
        "Pick 4 burst/DPS route"
    } else if stats.breakables_smashed >= 4
        || stats.shrine_resonance_triggered
        || stats.potions_used >= 2
        || stats.last_stand_potions > 0
    {
        "Pick 5 survival route"
    } else if stats.surge_kills > 0
        || stats.pylon_kills > 0
        || stats.echo_keystones > 0
        || stats.best_massacre_streak < CARNAGE_MASTER_STREAK / 2
    {
        "Pick 6 fury/skill route"
    } else {
        "Pick 4 damage, 5 safety, 6 skills"
    }
}

pub fn socket_or_upgrade_gem(
    gem: SocketedGem,
    damage_bonus: &mut DamageBonus,
    equipment: &mut Equipment,
    inventory: &mut Inventory,
    health: &mut Health,
) -> SocketedGem {
    let old_health_bonus = equipment.health_bonus;
    let upgraded = match equipment.socketed_gem {
        Some(current) if current.kind == gem.kind => SocketedGem {
            kind: current.kind,
            rank: current.rank + gem.rank,
        },
        _ => gem,
    };
    if let Some(current) = equipment.socketed_gem {
        damage_bonus.0 -= current.damage_bonus();
        equipment.crit_chance -= current.crit_bonus();
        equipment.health_bonus -= current.health_bonus();
        equipment.armor_bonus -= current.armor_bonus();
    }
    damage_bonus.0 += upgraded.damage_bonus();
    equipment.crit_chance = (equipment.crit_chance + upgraded.crit_bonus()).min(0.40);
    equipment.health_bonus += upgraded.health_bonus();
    equipment.armor_bonus += upgraded.armor_bonus();
    equipment.socketed_gem = Some(upgraded);
    apply_equipment_health_delta(health, old_health_bonus, equipment.health_bonus);
    if let Some(item) = inventory
        .items
        .iter_mut()
        .find(|item| item.name == equipment.weapon_name)
    {
        item.damage_bonus = damage_bonus.0;
        item.crit_chance = equipment.crit_chance;
        item.health_bonus = equipment.health_bonus;
        item.armor_bonus = equipment.armor_bonus;
        item.socketed_gem = equipment.socketed_gem;
    }
    upgraded
}

pub struct PlayerPlugin;

type DeathCheckPlayerQuery<'w, 's> = Query<
    'w,
    's,
    (
        Entity,
        &'static mut Transform,
        &'static mut Health,
        &'static mut Barrier,
        &'static mut Fury,
        &'static mut DeathWard,
    ),
    With<Player>,
>;
type ClickMoveEnemyQuery<'w, 's> = Query<
    'w,
    's,
    (&'static Transform, &'static Enemy),
    (With<Enemy>, Without<Player>, Without<Camera3d>),
>;

impl Plugin for PlayerPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<DeathWardReport>()
            .init_resource::<ClickMoveTarget>()
            .add_systems(
                OnEnter(GameState::InGame),
                (reset_death_report, spawn_player, spawn_click_move_marker),
            )
            .add_systems(
                Update,
                (
                    (update_click_move_target, player_movement).chain(),
                    update_click_move_marker,
                    animate_player_visuals,
                    tick_player_motion_trails,
                    tick_cooldowns,
                    tick_relic_buff,
                    tick_elixir_buff,
                    tick_fortune_buff,
                    tick_glory_buff,
                    tick_surge_buff,
                    tick_evade,
                    tick_town_portal,
                    tick_potion_cooldown,
                    tick_burning,
                    tick_jailed,
                )
                    .run_if(in_state(GameState::InGame).and_then(not_paused)),
            )
            .add_systems(
                Update,
                (
                    activate_relic_shrine,
                    activate_ashen_pylon,
                    activate_fortune_shrine,
                    activate_storm_shrine,
                    activate_healing_well,
                    update_death_checkpoint,
                    charge_surge_from_kills,
                    extend_surge_from_kills,
                    record_pylon_kills,
                    use_surge,
                    use_evade,
                    use_town_portal,
                    use_armory_loadout,
                    cycle_codex_attunement,
                    cycle_skill_runes,
                    cycle_equipped_weapon,
                    use_potion,
                    use_elixir,
                )
                    .run_if(in_state(GameState::InGame).and_then(not_paused)),
            )
            .add_systems(
                Update,
                (
                    spend_talent_points,
                    choose_chapter_boon,
                    gain_xp,
                    camera_follow,
                    check_player_death,
                )
                    .run_if(in_state(GameState::InGame).and_then(not_paused)),
            )
            .add_systems(
                OnExit(GameState::InGame),
                (
                    despawn_player,
                    despawn_player_motion_trails,
                    despawn_click_move_marker,
                ),
            );
    }
}

fn reset_death_report(mut report: ResMut<DeathWardReport>) {
    *report = DeathWardReport::default();
}

fn spawn_click_move_marker(mut commands: Commands, assets: Res<GameAssets>) {
    commands.spawn((
        WorldAssetRoot(assets.objective_sigil.clone()),
        Transform::from_xyz(0.0, CLICK_MOVE_MARKER_Y, 0.0).with_scale(Vec3::splat(0.32)),
        Visibility::Hidden,
        ClickMoveMarker {
            fade: Timer::from_seconds(CLICK_MOVE_MARKER_FADE_SECS, TimerMode::Once),
            last_position: None,
        },
        Name::new("Click Move Marker"),
    ));
}

fn spawn_player(
    mut commands: Commands,
    assets: Res<GameAssets>,
    tuning: Res<PlayerTuning>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let mut cooldowns = SkillCooldowns {
        dash: Timer::from_seconds(tuning.dash_cooldown, TimerMode::Once),
        nova: Timer::from_seconds(tuning.nova_cooldown, TimerMode::Once),
        rupture: Timer::from_seconds(3.8, TimerMode::Once),
        basic: Timer::from_seconds(0.34, TimerMode::Once),
        dash_base: tuning.dash_cooldown,
        nova_base: tuning.nova_cooldown,
        rupture_base: 3.8,
        basic_base: 0.34,
    };
    cooldowns.dash.set_elapsed(cooldowns.dash.duration());
    cooldowns.nova.set_elapsed(cooldowns.nova.duration());
    cooldowns.rupture.set_elapsed(cooldowns.rupture.duration());
    cooldowns.basic.set_elapsed(cooldowns.basic.duration());
    let limb_mesh = meshes.add(Cuboid::new(1.0, 1.0, 1.0));
    let shoulder_material = materials.add(StandardMaterial {
        base_color: Color::srgb(0.42, 0.33, 0.20),
        emissive: Color::srgb(0.08, 0.045, 0.015).into(),
        perceptual_roughness: 0.58,
        metallic: 0.38,
        ..default()
    });
    let limb_material = materials.add(StandardMaterial {
        base_color: Color::srgb(0.24, 0.20, 0.17),
        emissive: Color::srgb(0.025, 0.018, 0.012).into(),
        perceptual_roughness: 0.72,
        metallic: 0.12,
        ..default()
    });

    commands
        .spawn((
            WorldAssetRoot(assets.hero.clone()),
            Transform::from_xyz(0.0, 0.0, 4.0).with_scale(Vec3::splat(1.0)),
            Player,
            Health {
                current: tuning.max_health,
                max: tuning.max_health,
            },
            Barrier {
                current: 0.0,
                max: (tuning.max_health * 0.35).round(),
            },
            Velocity(Vec3::ZERO),
            PlayerAnimation::default(),
            DamageBonus(0.0),
            Fury {
                current: 35.0,
                max: 100.0,
                basic_gain: 18.0,
                dash_cost: 25.0,
                nova_cost: 45.0,
                rupture_cost: 32.0,
            },
            Equipment {
                weapon_name: "Initiate Blade".to_string(),
                quality: "common".to_string(),
                crit_chance: 0.03,
                health_bonus: 0.0,
                armor_bonus: 0.0,
                legendary_power: LegendaryPower::None,
                temper_level: 0,
                socketed_gem: None,
            },
            Inventory {
                items: vec![InventoryItem {
                    name: "Initiate Blade".to_string(),
                    quality: "common".to_string(),
                    damage_bonus: 0.0,
                    crit_chance: 0.03,
                    health_bonus: 0.0,
                    armor_bonus: 0.0,
                    legendary_power: LegendaryPower::None,
                    temper_level: 0,
                    socketed_gem: None,
                }],
                capacity: 12,
            },
            Charm::default(),
            PlayerLevel {
                level: 1,
                xp: 0,
                next_level_xp: 60,
            },
        ))
        .insert((
            ChapterBoon::default(),
            Talents::default(),
            SkillRunes::default(),
            PotionBelt {
                current: 2,
                max: 5,
                heal_amount: 45.0,
                cooldown: Timer::from_seconds(0.0, TimerMode::Once),
                cooldown_secs: potion_cooldown_secs_for_capacity(5),
            },
            ElixirBelt {
                current: 1,
                max: 3,
                armor_bonus: 35.0,
                duration_secs: 22.0,
                selected: ElixirKind::Iron,
            },
            RelicBuff {
                timer: Timer::from_seconds(0.0, TimerMode::Once),
                damage_multiplier: 1.0,
                speed_multiplier: 1.0,
            },
            ElixirBuff {
                timer: Timer::from_seconds(0.0, TimerMode::Once),
                kind: ElixirKind::Iron,
                armor_bonus: 0.0,
                damage_multiplier: 1.0,
                speed_multiplier: 1.0,
            },
            FortuneBuff {
                timer: Timer::from_seconds(0.0, TimerMode::Once),
                xp_multiplier: 1.25,
                gold_multiplier: 1.50,
            },
        ))
        .insert((
            ConduitBuff {
                timer: Timer::from_seconds(0.0, TimerMode::Once),
                pulse: Timer::from_seconds(0.65, TimerMode::Repeating),
                damage: 18.0,
                radius: 5.4,
            },
            GloryBuff {
                timer: Timer::from_seconds(0.0, TimerMode::Once),
                stacks: 0,
                max_stacks: 3,
                damage_per_stack: 0.06,
                speed_per_stack: 0.04,
            },
            SurgeBuff {
                timer: Timer::from_seconds(0.0, TimerMode::Once),
                charge: 0,
                max_charge: 9,
                duration_secs: 9.0,
                damage_multiplier: 1.24,
                speed_multiplier: 1.16,
                fury_multiplier: 1.35,
            },
            Evade {
                active: Timer::from_seconds(0.0, TimerMode::Once),
                cooldown: Timer::from_seconds(0.0, TimerMode::Once),
                cooldown_secs: 4.5,
                speed_multiplier: 2.65,
            },
            TownPortal {
                cooldown: Timer::from_seconds(0.0, TimerMode::Once),
                destination: Vec3::new(-4.8, 0.0, 4.4),
                return_position: None,
            },
            DeathWard {
                checkpoint: Vec3::new(0.0, 0.0, 4.0),
                revives_used: 0,
                max_revives: 2,
                gold_penalty: 60,
            },
            CompanionBond::default(),
            cooldowns,
            LegendaryCodex::default(),
            EmberParagon::default(),
            Armory::default(),
            Name::new("Reliquary Knight"),
        ))
        .with_children(|parent| {
            for kind in [
                PlayerLimbKind::ShoulderMantle,
                PlayerLimbKind::LeftArm,
                PlayerLimbKind::RightArm,
                PlayerLimbKind::LeftLeg,
                PlayerLimbKind::RightLeg,
            ] {
                let material = if matches!(kind, PlayerLimbKind::ShoulderMantle) {
                    shoulder_material.clone()
                } else {
                    limb_material.clone()
                };
                let pose =
                    player_limb_pose(kind, false, false, 0.0, PlayerAttackMotion::Basic, 1.0);
                parent.spawn((
                    Mesh3d(limb_mesh.clone()),
                    MeshMaterial3d(material),
                    Transform::from_translation(pose.translation)
                        .with_rotation(pose.rotation)
                        .with_scale(pose.scale),
                    pose.visibility,
                    PlayerLimbVisual { kind },
                    Name::new(kind.label()),
                ));
            }
            for side in [-1.0, 1.0] {
                parent.spawn((
                    WorldAssetRoot(assets.shadow_trail.clone()),
                    Transform::from_xyz(side * 0.28, 0.035, -0.18)
                        .with_scale(Vec3::new(0.28, 0.16, 0.20)),
                    Visibility::Hidden,
                    PlayerFootVisual { side },
                    Name::new(if side < 0.0 {
                        "Left Step Shadow"
                    } else {
                        "Right Step Shadow"
                    }),
                ));
            }
            parent.spawn((
                WorldAssetRoot(assets.sword.clone()),
                Transform::from_xyz(0.42, 0.92, 0.22)
                    .with_rotation(
                        Quat::from_rotation_x(-0.55)
                            * Quat::from_rotation_y(0.2)
                            * Quat::from_rotation_z(-0.85),
                    )
                    .with_scale(Vec3::splat(0.72)),
                PlayerWeaponVisual,
                Name::new("Equipped Sword"),
            ));
            parent.spawn((
                WorldAssetRoot(assets.slash_arc.clone()),
                Transform::from_xyz(0.0, 0.74, 0.82).with_scale(Vec3::splat(0.72)),
                Visibility::Hidden,
                PlayerAttackArcVisual,
                Name::new("Player Slash Arc"),
            ));
            parent.spawn((
                WorldAssetRoot(assets.hit_bone_lock.clone()),
                Transform::from_xyz(0.0, 0.055, 0.42).with_scale(Vec3::splat(0.0)),
                Visibility::Hidden,
                PlayerAttackCommitVisual,
                Name::new("Player Attack Commitment Ring"),
            ));
        });
}

fn use_potion(
    keyboard: Res<ButtonInput<KeyCode>>,
    gamepads: Query<&Gamepad>,
    mut stats: ResMut<RunStats>,
    mut combat_events: MessageWriter<CombatEvent>,
    mut player: Query<(&mut Health, &mut PotionBelt), With<Player>>,
) {
    if !keyboard.just_pressed(KeyCode::KeyF)
        && !gamepad_button_just_pressed(&gamepads, &[GamepadButton::West])
    {
        return;
    }
    let Ok((mut health, mut potions)) = player.single_mut() else {
        return;
    };
    let health_before = health.current;
    match try_drink_potion(&mut health, &mut potions) {
        Some(restored) => {
            stats.potions_used += 1;
            if last_stand_potion_eligible(health_before, health.max) {
                stats.last_stand_potions += 1;
            }
            combat_events.write(CombatEvent {
                text: format!("Potion restored {restored:.0} health"),
            });
        }
        None if potion_seconds_remaining(&potions) > 0.0 => {
            combat_events.write(CombatEvent {
                text: format!(
                    "Potion recharging {:.0}s",
                    potion_seconds_remaining(&potions)
                ),
            });
        }
        None => {}
    }
}

pub fn try_drink_potion(health: &mut Health, potions: &mut PotionBelt) -> Option<f32> {
    if potions.current == 0
        || health.current >= health.max
        || potion_seconds_remaining(potions) > 0.0
    {
        return None;
    }
    let before = health.current;
    potions.current -= 1;
    health.current = (health.current + potions.heal_amount).min(health.max);
    potions.cooldown = Timer::from_seconds(potions.cooldown_secs, TimerMode::Once);
    Some(health.current - before)
}

fn last_stand_potion_eligible(current: f32, max: f32) -> bool {
    max > 0.0 && current <= max * LAST_STAND_HEALTH_RATIO
}

fn tick_potion_cooldown(time: Res<Time>, mut query: Query<&mut PotionBelt, With<Player>>) {
    let Ok(mut potions) = query.single_mut() else {
        return;
    };
    potions.cooldown.tick(time.delta());
}

fn use_elixir(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut stats: ResMut<RunStats>,
    mut combat_events: MessageWriter<CombatEvent>,
    mut player: Query<(&Health, &mut Barrier, &mut ElixirBelt, &mut ElixirBuff), With<Player>>,
) {
    if keyboard.just_pressed(KeyCode::KeyV) {
        let Ok((_, _, mut belt, _)) = player.single_mut() else {
            return;
        };
        belt.selected = belt.selected.next();
        combat_events.write(CombatEvent {
            text: format!("Elixir selected: {}", belt.selected.label()),
        });
        return;
    }
    if !keyboard.just_pressed(KeyCode::KeyG) {
        return;
    }
    let Ok((health, mut barrier, mut belt, mut buff)) = player.single_mut() else {
        return;
    };
    if belt.current == 0 {
        return;
    }
    belt.current -= 1;
    buff.timer = Timer::from_seconds(belt.duration_secs, TimerMode::Once);
    configure_elixir_buff(belt.selected, belt.armor_bonus, &mut buff);
    match belt.selected {
        ElixirKind::Iron => {
            stats.iron_elixir_used = true;
            barrier.max = (health.max * 0.35).round().max(1.0);
            let granted = barrier.grant((health.max * 0.18).round().max(1.0));
            combat_events.write(CombatEvent {
                text: format!(
                    "Iron elixir: +{:.0} armor and +{granted:.0} barrier",
                    belt.armor_bonus
                ),
            });
        }
        ElixirKind::Wrath => {
            stats.wrath_elixir_used = true;
            buff.damage_multiplier = 1.18;
            combat_events.write(CombatEvent {
                text: "Wrath elixir: +18% damage".to_string(),
            });
        }
        ElixirKind::Haste => {
            stats.haste_elixir_used = true;
            buff.speed_multiplier = 1.20;
            combat_events.write(CombatEvent {
                text: "Haste elixir: +20% movement speed".to_string(),
            });
        }
    }
}

type XpPlayerQuery<'w, 's> = Query<
    'w,
    's,
    (
        &'static mut PlayerLevel,
        &'static mut EmberParagon,
        &'static mut Health,
        &'static mut DamageBonus,
        &'static mut Equipment,
        &'static mut Talents,
        &'static FortuneBuff,
    ),
    With<Player>,
>;

fn gain_xp(
    mut killed: MessageReader<EnemyKilled>,
    mut combat_events: MessageWriter<CombatEvent>,
    mut floating_text: MessageWriter<FloatingCombatTextEvent>,
    mut screen_shake: MessageWriter<ScreenShakeEvent>,
    mut stats: ResMut<RunStats>,
    mut player: XpPlayerQuery,
) {
    let Ok((
        mut level,
        mut paragon,
        mut health,
        mut damage_bonus,
        mut equipment,
        mut talents,
        fortune,
    )) = player.single_mut()
    else {
        return;
    };
    for event in killed.read() {
        let base_xp = fortune_xp_reward(event.xp_reward, fortune);
        let (bonus_xp, bonus_gold) = register_massacre_kill(&mut stats, base_xp);
        let total_xp = base_xp + bonus_xp;
        if bonus_xp > 0 || bonus_gold > 0 {
            combat_events.write(CombatEvent {
                text: format!(
                    "Massacre {}x: +{} XP +{} gold",
                    stats.massacre_streak, bonus_xp, bonus_gold
                ),
            });
            if let Some(feedback) =
                massacre_feedback_event(stats.massacre_streak, bonus_xp, bonus_gold)
            {
                floating_text.write(FloatingCombatTextEvent {
                    text: feedback.text,
                    position: event.position,
                    critical: true,
                });
                screen_shake.write(ScreenShakeEvent::new(feedback.shake_intensity, 0.14));
            }
        }
        grant_player_xp(
            total_xp,
            XpProgression {
                level: &mut level,
                paragon: &mut paragon,
                health: &mut health,
                damage_bonus: &mut damage_bonus,
                equipment: &mut equipment,
                talents: &mut talents,
                combat_events: &mut combat_events,
            },
        );
    }
}

#[derive(Debug, Clone, PartialEq)]
struct MassacreFeedback {
    text: String,
    shake_intensity: f32,
}

fn massacre_feedback_event(
    streak: u32,
    bonus_xp: u32,
    bonus_gold: u32,
) -> Option<MassacreFeedback> {
    if bonus_xp == 0 && bonus_gold == 0 {
        return None;
    }
    let spotlight = matches!(streak, 3 | 5 | 7) || (streak >= 10 && streak.is_multiple_of(5));
    if !spotlight {
        return None;
    }
    let title = match streak {
        3..=4 => "Massacre",
        5..=6 => "Carnage",
        7..=14 => "Slaughter",
        _ => "Annihilation",
    };
    Some(MassacreFeedback {
        text: format!("{title} {streak}x +{bonus_xp} XP +{bonus_gold} gold"),
        shake_intensity: (0.035 + streak.min(20) as f32 * 0.002).min(0.075),
    })
}

struct XpProgression<'a, 'w> {
    level: &'a mut PlayerLevel,
    paragon: &'a mut EmberParagon,
    health: &'a mut Health,
    damage_bonus: &'a mut DamageBonus,
    equipment: &'a mut Equipment,
    talents: &'a mut Talents,
    combat_events: &'a mut MessageWriter<'w, CombatEvent>,
}

fn grant_player_xp(xp: u32, progression: XpProgression) {
    let XpProgression {
        level,
        paragon,
        health,
        damage_bonus,
        equipment,
        talents,
        combat_events,
    } = progression;
    if level.level < CHAPTER_LEVEL_CAP {
        level.xp += xp;
        combat_events.write(CombatEvent {
            text: format!("Gained {xp} XP"),
        });
        while level.level < CHAPTER_LEVEL_CAP && level.xp >= level.next_level_xp {
            level.xp -= level.next_level_xp;
            level.level += 1;
            level.next_level_xp = (level.next_level_xp as f32 * 1.45).round() as u32;
            health.max += 18.0;
            health.current = health.max;
            damage_bonus.0 += 2.0;
            talents.points += 1;
            combat_events.write(CombatEvent {
                text: format!("Level {} reached; gained 1 talent point", level.level),
            });
            info!("Level up to {}", level.level);
        }
        if level.level < CHAPTER_LEVEL_CAP {
            return;
        }
        let overflow = level.xp;
        level.xp = 0;
        if overflow == 0 {
            return;
        }
        grant_ember_paragon_xp(
            overflow,
            paragon,
            health,
            damage_bonus,
            equipment,
            combat_events,
        );
        return;
    }
    grant_ember_paragon_xp(xp, paragon, health, damage_bonus, equipment, combat_events);
}

fn grant_ember_paragon_xp(
    xp: u32,
    paragon: &mut EmberParagon,
    health: &mut Health,
    damage_bonus: &mut DamageBonus,
    equipment: &mut Equipment,
    combat_events: &mut MessageWriter<CombatEvent>,
) {
    paragon.xp += xp;
    combat_events.write(CombatEvent {
        text: format!("Gained {xp} Ember Paragon XP"),
    });
    while paragon.xp >= paragon.next_rank_xp {
        paragon.xp -= paragon.next_rank_xp;
        paragon.rank += 1;
        apply_ember_paragon_rank(paragon.rank, health, damage_bonus, equipment);
        paragon.next_rank_xp = next_ember_paragon_xp(paragon.next_rank_xp);
        combat_events.write(CombatEvent {
            text: format!("Ember Paragon {} awakened", paragon.rank),
        });
    }
}

fn next_ember_paragon_xp(current: u32) -> u32 {
    ((current as f32) * 1.30).round() as u32
}

fn apply_ember_paragon_rank(
    rank: u32,
    health: &mut Health,
    damage_bonus: &mut DamageBonus,
    equipment: &mut Equipment,
) {
    match (rank - 1) % 4 {
        0 => damage_bonus.0 += 1.5,
        1 => {
            health.max += 12.0;
            health.current = (health.current + 12.0).min(health.max);
        }
        2 => equipment.crit_chance = (equipment.crit_chance + 0.005).min(0.50),
        _ => equipment.armor_bonus += 4.0,
    }
}

fn spend_talent_points(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut combat_events: MessageWriter<CombatEvent>,
    mut player: Query<(&mut Talents, &mut Health), With<Player>>,
) {
    let Ok((mut talents, mut health)) = player.single_mut() else {
        return;
    };
    if talents.points == 0 {
        return;
    }

    let spent = if keyboard.just_pressed(KeyCode::Digit1) {
        talents.wrath += 1;
        Some("Wrath")
    } else if keyboard.just_pressed(KeyCode::Digit2) {
        talents.vigor += 1;
        health.max += 12.0;
        health.current = (health.current + 12.0).min(health.max);
        Some("Vigor")
    } else if keyboard.just_pressed(KeyCode::Digit3) {
        talents.focus += 1;
        Some("Focus")
    } else {
        None
    };

    if let Some(name) = spent {
        talents.points -= 1;
        combat_events.write(CombatEvent {
            text: format!("Talent learned: {name}"),
        });
    }
}

fn cycle_codex_attunement(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut combat_events: MessageWriter<CombatEvent>,
    mut player: Query<&mut LegendaryCodex, With<Player>>,
) {
    if !keyboard.just_pressed(KeyCode::KeyB) {
        return;
    }
    let Ok(mut codex) = player.single_mut() else {
        return;
    };
    match codex.cycle_attunement() {
        Some(power) => combat_events.write(CombatEvent {
            text: format!("Codex attuned: {}", power.label()),
        }),
        None => combat_events.write(CombatEvent {
            text: "Codex attunement requires a legendary power".to_string(),
        }),
    };
}

type ArmoryPlayerQuery<'w, 's> = Query<
    'w,
    's,
    (
        &'static mut Armory,
        &'static mut DamageBonus,
        &'static mut Equipment,
        &'static mut Inventory,
        &'static mut Health,
        &'static mut Charm,
        &'static mut SkillRunes,
        &'static mut LegendaryCodex,
    ),
    With<Player>,
>;

fn use_armory_loadout(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut stats: ResMut<RunStats>,
    mut combat_events: MessageWriter<CombatEvent>,
    mut player: ArmoryPlayerQuery,
) {
    if !keyboard.just_pressed(KeyCode::KeyO)
        && !keyboard.just_pressed(KeyCode::KeyP)
        && !keyboard.just_pressed(KeyCode::Digit7)
        && !keyboard.just_pressed(KeyCode::Digit8)
        && !keyboard.just_pressed(KeyCode::Digit9)
    {
        return;
    }
    let Ok((
        mut armory,
        mut damage_bonus,
        mut equipment,
        mut inventory,
        mut health,
        mut charm,
        mut runes,
        mut codex,
    )) = player.single_mut()
    else {
        return;
    };
    if keyboard.just_pressed(KeyCode::Digit7) {
        armory.select_slot(0);
        combat_events.write(CombatEvent {
            text: armory.summary(),
        });
        return;
    }
    if keyboard.just_pressed(KeyCode::Digit8) {
        armory.select_slot(1);
        combat_events.write(CombatEvent {
            text: armory.summary(),
        });
        return;
    }
    if keyboard.just_pressed(KeyCode::Digit9) {
        armory.select_slot(2);
        combat_events.write(CombatEvent {
            text: armory.summary(),
        });
        return;
    }
    if keyboard.just_pressed(KeyCode::KeyO) {
        let slot = armory.active_slot.min(ARMORY_SLOT_COUNT - 1);
        armory.save_active(capture_armory_loadout(
            &damage_bonus,
            &equipment,
            &inventory,
            &charm,
            &runes,
            &codex,
        ));
        stats.armory_loadouts_saved = stats
            .armory_loadouts_saved
            .max(armory_filled_slots(&armory) as u32);
        combat_events.write(CombatEvent {
            text: format!("Armory {} saved: {}", slot + 1, equipment.weapon_name),
        });
        return;
    }
    let slot = armory.active_slot.min(ARMORY_SLOT_COUNT - 1);
    let Some(loadout) = armory.active_loadout().cloned() else {
        combat_events.write(CombatEvent {
            text: format!("Armory {} is empty", slot + 1),
        });
        return;
    };
    restore_armory_loadout(
        &loadout,
        ArmoryRestoreTargets {
            damage_bonus: &mut damage_bonus,
            equipment: &mut equipment,
            inventory: &mut inventory,
            health: &mut health,
            charm: &mut charm,
            runes: &mut runes,
            codex: &mut codex,
        },
    );
    combat_events.write(CombatEvent {
        text: format!("Armory {} restored: {}", slot + 1, equipment.weapon_name),
    });
}

fn choose_chapter_boon(
    keyboard: Res<ButtonInput<KeyCode>>,
    progress: Res<ChapterProgress>,
    stats: Res<RunStats>,
    mut combat_events: MessageWriter<CombatEvent>,
    mut player: Query<
        (
            &mut ChapterBoon,
            &mut DamageBonus,
            &mut Equipment,
            &mut Health,
            &mut Fury,
        ),
        With<Player>,
    >,
) {
    if progress.phase != ChapterPhase::Boss {
        return;
    }
    let choice = if keyboard.just_pressed(KeyCode::Digit4) {
        ChapterBoonChoice::CrimsonOath
    } else if keyboard.just_pressed(KeyCode::Digit5) {
        ChapterBoonChoice::TitanWard
    } else if keyboard.just_pressed(KeyCode::Digit6) {
        ChapterBoonChoice::ArcaneMomentum
    } else {
        return;
    };
    let Ok((mut boon, mut damage, mut equipment, mut health, mut fury)) = player.single_mut()
    else {
        return;
    };
    if boon.choice != ChapterBoonChoice::None {
        combat_events.write(CombatEvent {
            text: format!("Reliquary boon already chosen: {}", boon.choice.label()),
        });
        return;
    }
    let empowered = chapter_boon_is_empowered(choice, &stats);
    let momentum_stacks = if choice == ChapterBoonChoice::ArcaneMomentum {
        reliquary_boon_momentum_stacks(&stats)
    } else {
        0
    };
    apply_chapter_boon(
        choice,
        empowered,
        momentum_stacks,
        &mut damage,
        &mut equipment,
        &mut health,
        &mut fury,
    );
    boon.choice = choice;
    boon.empowered = empowered;
    boon.momentum_stacks = momentum_stacks;
    combat_events.write(CombatEvent {
        text: format!("Reliquary boon claimed: {}", boon.summary()),
    });
}

fn cycle_skill_runes(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut combat_events: MessageWriter<CombatEvent>,
    mut player: Query<&mut SkillRunes, With<Player>>,
) {
    let Ok(mut runes) = player.single_mut() else {
        return;
    };

    if keyboard.just_pressed(KeyCode::KeyZ) {
        runes.dash = runes.dash.next();
        combat_events.write(CombatEvent {
            text: format!("Dash rune: {}", runes.dash.label()),
        });
    }
    if keyboard.just_pressed(KeyCode::KeyX) {
        runes.nova = runes.nova.next();
        combat_events.write(CombatEvent {
            text: format!("Nova rune: {}", runes.nova.label()),
        });
    }
    if keyboard.just_pressed(KeyCode::KeyC) {
        runes.rupture = runes.rupture.next();
        combat_events.write(CombatEvent {
            text: format!("Rupture rune: {}", runes.rupture.label()),
        });
    }
}

fn cycle_equipped_weapon(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut combat_events: MessageWriter<CombatEvent>,
    mut player: Query<(&mut DamageBonus, &mut Equipment, &mut Health, &Inventory), With<Player>>,
) {
    let offset = if keyboard.just_pressed(KeyCode::BracketRight) {
        1
    } else if keyboard.just_pressed(KeyCode::BracketLeft) {
        -1
    } else {
        return;
    };
    let Ok((mut damage_bonus, mut equipment, mut health, inventory)) = player.single_mut() else {
        return;
    };
    let Some(index) = inventory_swap_index(inventory, &equipment.weapon_name, offset) else {
        combat_events.write(CombatEvent {
            text: "No weapons in inventory".to_string(),
        });
        return;
    };
    let item = inventory.items[index].clone();
    let equip_summary = manual_equip_summary(&item, &damage_bonus, &equipment);
    equip_inventory_item(&item, &mut damage_bonus, &mut equipment, &mut health);
    combat_events.write(CombatEvent {
        text: equip_summary,
    });
    if equipment.legendary_power != LegendaryPower::None {
        combat_events.write(CombatEvent {
            text: format!("Legendary power: {}", equipment.legendary_power.label()),
        });
    }
}

type PlayerMovementQuery<'w, 's> = Query<
    'w,
    's,
    (
        &'static mut Transform,
        &'static mut Velocity,
        &'static mut PlayerAnimation,
        &'static RelicBuff,
        &'static ElixirBuff,
        &'static GloryBuff,
        &'static SurgeBuff,
        &'static Evade,
        Option<&'static Jailed>,
    ),
    With<Player>,
>;

type PlayerFootVisualQuery<'w, 's> = Query<
    'w,
    's,
    (
        &'static PlayerFootVisual,
        &'static mut Transform,
        &'static mut Visibility,
    ),
    (
        Without<Player>,
        Without<PlayerWeaponVisual>,
        Without<PlayerAttackArcVisual>,
        Without<PlayerAttackCommitVisual>,
    ),
>;

type PlayerLimbVisualQuery<'w, 's> = Query<
    'w,
    's,
    (
        &'static PlayerLimbVisual,
        &'static mut Transform,
        &'static mut Visibility,
    ),
    (
        Without<Player>,
        Without<PlayerWeaponVisual>,
        Without<PlayerFootVisual>,
        Without<PlayerAttackArcVisual>,
        Without<PlayerAttackCommitVisual>,
    ),
>;

type PlayerAttackArcVisualQuery<'w, 's> = Query<
    'w,
    's,
    (
        &'static mut WorldAssetRoot,
        &'static mut Transform,
        &'static mut Visibility,
    ),
    (
        With<PlayerAttackArcVisual>,
        Without<Player>,
        Without<PlayerWeaponVisual>,
        Without<PlayerFootVisual>,
        Without<PlayerAttackCommitVisual>,
    ),
>;
type PlayerAttackCommitVisualQuery<'w, 's> = Query<
    'w,
    's,
    (
        &'static mut WorldAssetRoot,
        &'static mut Transform,
        &'static mut Visibility,
    ),
    (
        With<PlayerAttackCommitVisual>,
        Without<Player>,
        Without<PlayerWeaponVisual>,
        Without<PlayerFootVisual>,
        Without<PlayerAttackArcVisual>,
    ),
>;
type PlayerVisualPlayerQuery<'w, 's> = Query<
    'w,
    's,
    (
        Entity,
        &'static Velocity,
        &'static Evade,
        Option<&'static mut PlayerHitReact>,
        &'static mut PlayerAnimation,
        &'static mut Transform,
    ),
    With<Player>,
>;

#[derive(SystemParam)]
struct PlayerVisualQueries<'w, 's> {
    player: PlayerVisualPlayerQuery<'w, 's>,
    weapons: Query<'w, 's, &'static mut Transform, (With<PlayerWeaponVisual>, Without<Player>)>,
    limbs: PlayerLimbVisualQuery<'w, 's>,
    feet: PlayerFootVisualQuery<'w, 's>,
    attack_arcs: PlayerAttackArcVisualQuery<'w, 's>,
    attack_commit_rings: PlayerAttackCommitVisualQuery<'w, 's>,
}

fn player_movement(
    keyboard: Res<ButtonInput<KeyCode>>,
    gamepads: Query<&Gamepad>,
    time: Res<Time>,
    tuning: Res<PlayerTuning>,
    mut click_move: ResMut<ClickMoveTarget>,
    mut query: PlayerMovementQuery,
) {
    let Ok((mut transform, mut velocity, mut animation, buff, elixir, glory, surge, evade, jailed)) =
        query.single_mut()
    else {
        return;
    };

    let mut axis = Vec3::ZERO;
    if keyboard.pressed(KeyCode::KeyW) {
        axis.z -= 1.0;
    }
    if keyboard.pressed(KeyCode::KeyS) {
        axis.z += 1.0;
    }
    if keyboard.pressed(KeyCode::KeyA) {
        axis.x -= 1.0;
    }
    if keyboard.pressed(KeyCode::KeyD) {
        axis.x += 1.0;
    }
    axis += gamepad_move_axis(&gamepads);

    let (direction, reached_click_target) = movement_direction(
        axis,
        transform.translation,
        click_move.position,
        click_move.stop_radius,
    );
    if axis.length_squared() > 0.0 || reached_click_target {
        click_move.position = None;
        click_move.stop_radius = CLICK_MOVE_STOP_RADIUS;
    }
    velocity.0 = direction
        * tuning.move_speed
        * active_speed_multiplier(buff)
        * elixir_speed_multiplier(elixir)
        * glory_speed_multiplier(glory)
        * surge_speed_multiplier(surge)
        * evade_speed_multiplier(evade)
        * jailed_speed_multiplier(jailed);
    transform.translation += velocity.0 * time.delta_secs();
    transform.translation = clamp_player_translation(transform.translation);

    if direction.length_squared() > 0.0 {
        let response = player_turn_response(
            animation.facing_yaw,
            direction.x.atan2(direction.z),
            velocity.0.length(),
            time.delta_secs(),
        );
        animation.facing_yaw = response.facing_yaw;
        animation.turn_lean = response.turn_lean;
    } else {
        animation.turn_lean = decay_player_turn_lean(animation.turn_lean, time.delta_secs());
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct PlayerTurnResponse {
    facing_yaw: f32,
    turn_lean: f32,
}

fn player_turn_response(
    current_yaw: f32,
    target_yaw: f32,
    speed: f32,
    delta_secs: f32,
) -> PlayerTurnResponse {
    let delta = shortest_yaw_delta(current_yaw, target_yaw);
    if delta.abs() < 0.001 || delta_secs <= 0.0 {
        return PlayerTurnResponse {
            facing_yaw: target_yaw,
            turn_lean: 0.0,
        };
    }

    let turn_rate = 8.5 + speed * 0.36;
    let max_step = turn_rate * delta_secs;
    let yaw_step = delta.clamp(-max_step, max_step);
    PlayerTurnResponse {
        facing_yaw: current_yaw + yaw_step,
        turn_lean: (yaw_step / delta_secs * 0.045).clamp(-0.46, 0.46),
    }
}

fn decay_player_turn_lean(turn_lean: f32, delta_secs: f32) -> f32 {
    if delta_secs <= 0.0 {
        return turn_lean;
    }
    let decay = (1.0 - delta_secs * 10.0).clamp(0.0, 1.0);
    let leaned = turn_lean * decay;
    if leaned.abs() < 0.001 { 0.0 } else { leaned }
}

fn shortest_yaw_delta(current_yaw: f32, target_yaw: f32) -> f32 {
    (target_yaw - current_yaw + std::f32::consts::PI).rem_euclid(std::f32::consts::TAU)
        - std::f32::consts::PI
}

fn gamepad_move_axis(gamepads: &Query<&Gamepad>) -> Vec3 {
    gamepads
        .iter()
        .map(|gamepad| {
            gamepad_move_axis_from_values(
                gamepad.get(GamepadAxis::LeftStickX).unwrap_or_default(),
                gamepad.get(GamepadAxis::LeftStickY).unwrap_or_default(),
            )
        })
        .find(|axis| axis.length_squared() > 0.0)
        .unwrap_or(Vec3::ZERO)
}

fn gamepad_move_axis_from_values(left_x: f32, left_y: f32) -> Vec3 {
    let stick = Vec2::new(left_x, left_y);
    if stick.length() < GAMEPAD_MOVE_DEAD_ZONE {
        Vec3::ZERO
    } else {
        Vec3::new(left_x, 0.0, -left_y)
    }
}

fn gamepad_button_just_pressed(gamepads: &Query<&Gamepad>, buttons: &[GamepadButton]) -> bool {
    gamepads
        .iter()
        .any(|gamepad| buttons.iter().any(|button| gamepad.just_pressed(*button)))
}

fn update_click_move_target(
    mouse: Res<ButtonInput<MouseButton>>,
    windows: Query<&Window, With<PrimaryWindow>>,
    cameras: Query<(&Camera, &GlobalTransform), With<Camera3d>>,
    tuning: Res<PlayerTuning>,
    enemies: ClickMoveEnemyQuery,
    mut click_move: ResMut<ClickMoveTarget>,
) {
    if !mouse.pressed(MouseButton::Left) {
        return;
    }
    let Ok(window) = windows.single() else {
        return;
    };
    let Some(cursor_position) = window.cursor_position() else {
        return;
    };
    let Ok((camera, camera_transform)) = cameras.single() else {
        return;
    };
    let Ok(ray) = camera.viewport_to_world(camera_transform, cursor_position) else {
        return;
    };
    let Some(distance) = ray.intersect_plane(Vec3::ZERO, InfinitePlane3d::new(Vec3::Y)) else {
        return;
    };
    let mut point = ray.get_point(distance);
    point.y = 0.0;
    if let Some(enemy_position) = cursor_target_enemy_position(point, &enemies) {
        click_move.position = Some(clamp_player_translation(enemy_position));
        click_move.stop_radius = click_attack_stop_radius(tuning.basic_range);
        return;
    }
    click_move.position = Some(clamp_player_translation(point));
    click_move.stop_radius = CLICK_MOVE_STOP_RADIUS;
}

fn cursor_target_enemy_position(
    cursor_ground: Vec3,
    enemies: &ClickMoveEnemyQuery,
) -> Option<Vec3> {
    enemies
        .iter()
        .filter(|(transform, enemy)| {
            cursor_targets_enemy(cursor_ground, transform.translation, enemy)
        })
        .min_by(|(left_transform, _), (right_transform, _)| {
            flat_distance(cursor_ground, left_transform.translation)
                .total_cmp(&flat_distance(cursor_ground, right_transform.translation))
        })
        .map(|(transform, _)| transform.translation)
}

fn cursor_targets_enemy(cursor_ground: Vec3, enemy_position: Vec3, enemy: &Enemy) -> bool {
    flat_distance(cursor_ground, enemy_position) <= enemy_cursor_pick_radius(enemy)
}

fn flat_distance(a: Vec3, b: Vec3) -> f32 {
    Vec2::new(a.x - b.x, a.z - b.z).length()
}

fn click_attack_stop_radius(basic_range: f32) -> f32 {
    (basic_range * 0.86).max(CLICK_MOVE_STOP_RADIUS)
}

fn update_click_move_marker(
    time: Res<Time>,
    click_move: Res<ClickMoveTarget>,
    mut markers: Query<(&mut Transform, &mut Visibility, &mut ClickMoveMarker)>,
) {
    let Ok((mut transform, mut visibility, mut marker)) = markers.single_mut() else {
        return;
    };
    if let Some(target) = click_move.position {
        marker.last_position = Some(target);
        marker.fade = Timer::from_seconds(CLICK_MOVE_MARKER_FADE_SECS, TimerMode::Once);
        marker.fade.reset();
        let pose = click_move_marker_pose(target, 1.0);
        transform.translation = pose.translation;
        transform.scale = pose.scale;
        *visibility = pose.visibility;
        return;
    }

    let Some(last_position) = marker.last_position else {
        *visibility = Visibility::Hidden;
        return;
    };
    marker.fade.tick(time.delta());
    if marker.fade.is_finished() {
        marker.last_position = None;
        *visibility = Visibility::Hidden;
        return;
    }

    let pose = click_move_marker_pose(last_position, click_move_marker_fade_alpha(&marker.fade));
    transform.translation = pose.translation;
    transform.scale = pose.scale;
    *visibility = pose.visibility;
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct ClickMoveMarkerPose {
    translation: Vec3,
    scale: Vec3,
    visibility: Visibility,
}

fn click_move_marker_pose(target: Vec3, alpha: f32) -> ClickMoveMarkerPose {
    let alpha = alpha.clamp(0.0, 1.0);
    ClickMoveMarkerPose {
        translation: Vec3::new(target.x, CLICK_MOVE_MARKER_Y, target.z),
        scale: Vec3::splat(0.18 + 0.16 * alpha),
        visibility: if alpha > 0.0 {
            Visibility::Visible
        } else {
            Visibility::Hidden
        },
    }
}

fn click_move_marker_fade_alpha(timer: &Timer) -> f32 {
    let duration = timer.duration().as_secs_f32();
    if duration <= 0.0 {
        return 0.0;
    }
    (1.0 - timer.elapsed_secs() / duration).clamp(0.0, 1.0)
}

fn movement_direction(
    axis: Vec3,
    current: Vec3,
    click_target: Option<Vec3>,
    stop_radius: f32,
) -> (Vec3, bool) {
    if axis.length_squared() > 0.0 {
        return (axis.normalize_or_zero(), false);
    }
    let Some(target) = click_target else {
        return (Vec3::ZERO, false);
    };
    let delta = Vec3::new(target.x - current.x, 0.0, target.z - current.z);
    let distance = delta.length();
    if distance <= stop_radius.max(CLICK_MOVE_STOP_RADIUS) {
        (Vec3::ZERO, true)
    } else {
        (delta / distance, false)
    }
}

fn clamp_player_translation(mut translation: Vec3) -> Vec3 {
    translation.x = translation.x.clamp(-11.5, 11.5);
    translation.z = translation.z.clamp(-7.5, 7.5);
    translation
}

fn animate_player_visuals(
    time: Res<Time>,
    assets: Res<GameAssets>,
    mut commands: Commands,
    mut visuals: PlayerVisualQueries,
) {
    let Ok((player_entity, velocity, evade, hit_react, mut animation, mut transform)) =
        visuals.player.single_mut()
    else {
        return;
    };

    animation.attack_timer.tick(time.delta());
    animation.motion_trail_timer.tick(time.delta());
    animation.attack_trail_timer.tick(time.delta());
    animation.footstep_timer.tick(time.delta());
    let hit_react_pose = update_player_hit_react(player_entity, hit_react, &time, &mut commands);
    let moving = velocity.0.length_squared() > 0.01;
    let speed = velocity.0.length();
    let evading = evade_seconds_remaining(evade) > 0.0;
    if moving {
        animation.walk_phase += time.delta_secs() * (9.0 + speed * 0.85);
    } else {
        animation.walk_phase *= 0.76;
    }

    let attack_progress = if animation.attack_timer.is_finished() {
        1.0
    } else {
        (animation.attack_timer.elapsed_secs() / animation.attack_duration_secs).clamp(0.0, 1.0)
    };
    let walk_sway = if moving {
        animation.walk_phase.sin()
    } else {
        0.0
    };
    let walk_step = if moving {
        animation.walk_phase.cos()
    } else {
        0.0
    };
    let mut pose = player_visual_pose_with_hit_react(
        moving,
        evading,
        walk_sway,
        walk_step,
        animation.attack_motion,
        attack_progress,
        hit_react_pose,
    );
    pose = player_visual_pose_with_turn_lean(pose, animation.turn_lean, evading);
    let attack_arc_pose = player_attack_arc_pose(animation.attack_motion, attack_progress);

    transform.translation.y = pose.body_y;
    transform.rotation = Quat::from_rotation_y(animation.facing_yaw + pose.body_yaw)
        * Quat::from_rotation_x(pose.body_pitch)
        * Quat::from_rotation_z(pose.body_roll);
    transform.scale = pose.body_scale;

    if let Some(interval_secs) = motion_trail_interval(speed, evading)
        && animation.motion_trail_timer.is_finished()
    {
        spawn_player_motion_trail(
            &mut commands,
            assets.shadow_trail.clone(),
            transform.translation,
            animation.facing_yaw,
            motion_trail_scale(speed, evading),
        );
        animation
            .motion_trail_timer
            .set_duration(std::time::Duration::from_secs_f32(interval_secs));
        animation.motion_trail_timer.reset();
    }
    if let Some(interval_secs) = footstep_interval(speed, evading)
        && animation.footstep_timer.is_finished()
    {
        spawn_player_footstep_mark(
            &mut commands,
            assets.shadow_trail.clone(),
            transform.translation,
            animation.facing_yaw,
            animation.next_footstep_side,
            footstep_scale(speed, evading),
        );
        animation.next_footstep_side *= -1.0;
        animation
            .footstep_timer
            .set_duration(std::time::Duration::from_secs_f32(interval_secs));
        animation.footstep_timer.reset();
    }
    if let (Some(arc_pose), Some(interval_secs)) = (
        attack_arc_pose,
        attack_trail_interval(animation.attack_motion, attack_progress),
    ) && animation.attack_trail_timer.is_finished()
    {
        spawn_player_attack_trail(
            &mut commands,
            player_attack_vfx_handle(&assets, player_attack_arc_vfx(animation.attack_motion)),
            transform.translation,
            animation.facing_yaw,
            animation.attack_motion,
            arc_pose,
        );
        animation
            .attack_trail_timer
            .set_duration(std::time::Duration::from_secs_f32(interval_secs));
        animation.attack_trail_timer.reset();
    }

    let Ok(mut weapon) = visuals.weapons.single_mut() else {
        return;
    };
    weapon.translation = pose.weapon_translation;
    weapon.rotation = Quat::from_rotation_x(pose.weapon_pitch)
        * Quat::from_rotation_y(pose.weapon_yaw)
        * Quat::from_rotation_z(pose.weapon_roll);
    weapon.scale = pose.weapon_scale;

    for (limb, mut limb_transform, mut visibility) in &mut visuals.limbs {
        let limb_pose = player_limb_pose(
            limb.kind,
            moving,
            evading,
            animation.walk_phase,
            animation.attack_motion,
            attack_progress,
        );
        limb_transform.translation = limb_pose.translation;
        limb_transform.rotation = limb_pose.rotation;
        limb_transform.scale = limb_pose.scale;
        *visibility = limb_pose.visibility;
    }

    for (foot, mut foot_transform, mut visibility) in &mut visuals.feet {
        let foot_pose = player_foot_pose(foot.side, moving, evading, animation.walk_phase);
        foot_transform.translation = foot_pose.translation;
        foot_transform.scale = foot_pose.scale;
        *visibility = foot_pose.visibility;
    }

    if let Ok((mut arc_scene, mut arc_transform, mut visibility)) = visuals.attack_arcs.single_mut()
    {
        if let Some(arc_pose) = attack_arc_pose {
            *arc_scene = WorldAssetRoot(player_attack_vfx_handle(
                &assets,
                player_attack_arc_vfx(animation.attack_motion),
            ));
            arc_transform.translation = arc_pose.translation;
            arc_transform.rotation = arc_pose.rotation;
            arc_transform.scale = arc_pose.scale;
            *visibility = Visibility::Visible;
        } else {
            *visibility = Visibility::Hidden;
        }
    }
    if let Ok((mut commit_scene, mut commit_transform, mut visibility)) =
        visuals.attack_commit_rings.single_mut()
    {
        if let Some(commit_pose) =
            player_attack_commit_pose(animation.attack_motion, attack_progress)
        {
            *commit_scene = WorldAssetRoot(player_attack_vfx_handle(
                &assets,
                player_attack_commit_vfx(animation.attack_motion),
            ));
            commit_transform.translation = commit_pose.translation;
            commit_transform.rotation = commit_pose.rotation;
            commit_transform.scale = commit_pose.scale;
            *visibility = Visibility::Visible;
        } else {
            *visibility = Visibility::Hidden;
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct PlayerFootPose {
    translation: Vec3,
    scale: Vec3,
    visibility: Visibility,
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct PlayerLimbPose {
    translation: Vec3,
    rotation: Quat,
    scale: Vec3,
    visibility: Visibility,
}

fn player_limb_pose(
    kind: PlayerLimbKind,
    moving: bool,
    evading: bool,
    walk_phase: f32,
    attack_motion: PlayerAttackMotion,
    attack_progress: f32,
) -> PlayerLimbPose {
    let side = kind.side();
    let attacking = attack_progress < 1.0;
    let locomotion = if moving { 1.0 } else { 0.0 };
    let evade_drive = if evading { 1.0 } else { 0.0 };
    let stride = if moving {
        (walk_phase + side * std::f32::consts::FRAC_PI_2).sin()
    } else {
        0.0
    };
    let counter_stride = if moving { walk_phase.sin() } else { 0.0 };
    let (attack_arc, attack_drive) = if attacking {
        player_attack_motion_weights(attack_motion, attack_progress)
    } else {
        (0.0, 0.0)
    };
    let profile = player_attack_motion_profile(attack_motion, attack_progress);
    let windup = if attacking {
        triangle_peak(attack_progress, 0.16, 0.16)
    } else {
        0.0
    };
    let impact_snap = if attacking {
        triangle_peak(attack_progress, profile.impact_center, profile.impact_width)
    } else {
        0.0
    };

    match kind {
        PlayerLimbKind::ShoulderMantle => {
            let shoulder_twist = attack_arc * profile.body_yaw * 0.75
                + attack_drive * profile.body_drive_yaw * 0.45
                + locomotion * counter_stride * 0.14;
            PlayerLimbPose {
                translation: Vec3::new(0.0, 1.09 + attack_arc * 0.08, 0.03),
                rotation: Quat::from_rotation_y(shoulder_twist)
                    * Quat::from_rotation_z(-locomotion * counter_stride * 0.14),
                scale: Vec3::new(1.02 + attack_drive * 0.12, 0.15, 0.34),
                visibility: Visibility::Visible,
            }
        }
        PlayerLimbKind::RightArm => PlayerLimbPose {
            translation: Vec3::new(
                0.58 + attack_arc * profile.weapon_x * 0.82 - windup * 0.26 + evade_drive * 0.11,
                0.84 + attack_arc * profile.weapon_y * 0.46 + impact_snap * 0.18,
                0.26 + attack_drive * profile.weapon_z * 0.76 - windup * 0.46
                    + locomotion * stride * 0.34
                    + evade_drive * 0.24,
            ),
            rotation: Quat::from_rotation_x(
                -0.34 - attack_drive * (1.10 + profile.weapon_pitch.abs() * 0.38)
                    + windup * 0.58
                    + locomotion * stride * 0.54,
            ) * Quat::from_rotation_y(0.24 + attack_drive * profile.weapon_yaw * 0.66)
                * Quat::from_rotation_z(-0.42 - attack_arc * 0.46 + impact_snap * 0.62),
            scale: Vec3::new(
                0.20,
                0.62 + attack_drive * 0.24
                    + if matches!(attack_motion, PlayerAttackMotion::Rupture) {
                        0.12 * attack_arc
                    } else {
                        0.0
                    },
                0.18,
            ),
            visibility: Visibility::Visible,
        },
        PlayerLimbKind::LeftArm => PlayerLimbPose {
            translation: Vec3::new(
                -0.54 - attack_arc * 0.18,
                0.80 + attack_arc * 0.07,
                0.06 - attack_drive * 0.22 - locomotion * stride * 0.36 + evade_drive * 0.14,
            ),
            rotation: Quat::from_rotation_x(
                -0.18 + attack_drive * 0.62 - locomotion * stride * 0.78,
            ) * Quat::from_rotation_y(-0.28 - attack_arc * 0.28)
                * Quat::from_rotation_z(0.34 + attack_arc * 0.38),
            scale: Vec3::new(0.18, 0.54 + attack_arc * 0.08, 0.16),
            visibility: Visibility::Visible,
        },
        PlayerLimbKind::LeftLeg | PlayerLimbKind::RightLeg => {
            let plant = stride.max(0.0);
            let lift = (-stride).max(0.0);
            PlayerLimbPose {
                translation: Vec3::new(
                    side * (0.30 + plant * 0.08 + evade_drive * 0.06),
                    0.42 + lift * 0.28 + evade_drive * 0.04,
                    -0.05 + stride * 0.96 + evade_drive * 0.20,
                ),
                rotation: Quat::from_rotation_x(stride * 1.48 - evade_drive * 0.24)
                    * Quat::from_rotation_z(side * (0.08 + plant * 0.28)),
                scale: Vec3::new(
                    0.20 + evade_drive * 0.03,
                    0.66 + lift * 0.26,
                    0.22 + plant * 0.14,
                ),
                visibility: Visibility::Visible,
            }
        }
    }
}

fn player_foot_pose(side: f32, moving: bool, evading: bool, walk_phase: f32) -> PlayerFootPose {
    let stride = if moving {
        (walk_phase + side * std::f32::consts::FRAC_PI_2).sin()
    } else {
        0.0
    };
    let plant = if moving { stride.max(0.0) } else { 0.0 };
    let lift = if moving { (-stride).max(0.0) } else { 0.0 };
    let evade_drive = if evading { 1.0 } else { 0.0 };
    PlayerFootPose {
        translation: Vec3::new(
            side * (0.58 + plant * 0.34 + lift * 0.16 + evade_drive * 0.22),
            0.032 + lift * 0.48 + evade_drive * 0.04,
            -0.12 + stride * 1.82 + evade_drive * 0.52,
        ),
        scale: Vec3::new(
            0.48 + plant * 0.68 + lift * 0.18 + evade_drive * 0.26,
            0.13,
            0.44 + plant * 1.52 + lift * 0.42 + evade_drive * 1.62,
        ),
        visibility: if moving || evading {
            Visibility::Visible
        } else {
            Visibility::Hidden
        },
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct PlayerAttackArcPose {
    translation: Vec3,
    rotation: Quat,
    scale: Vec3,
}

fn player_attack_arc_pose(
    motion: PlayerAttackMotion,
    attack_progress: f32,
) -> Option<PlayerAttackArcPose> {
    if matches!(motion, PlayerAttackMotion::Basic) && !(0.04..=0.96).contains(&attack_progress) {
        return None;
    }
    let (arc_weight, impact_weight) = player_attack_motion_weights(motion, attack_progress);
    if arc_weight <= 0.05 {
        return None;
    }
    let peak = impact_weight.max(arc_weight * 0.72);
    let profile = player_attack_motion_profile(motion, attack_progress);
    let yaw = match motion {
        PlayerAttackMotion::Basic => -0.72 + attack_progress * 1.42,
        PlayerAttackMotion::Dash => -0.42 + attack_progress * 1.06,
        PlayerAttackMotion::Rupture => -1.02 + attack_progress * 1.80,
        PlayerAttackMotion::Nova => attack_progress * std::f32::consts::TAU,
    };
    let roll = match motion {
        PlayerAttackMotion::Nova => std::f32::consts::FRAC_PI_2,
        _ => -0.28 + attack_progress * 0.62,
    };
    let scale = match motion {
        PlayerAttackMotion::Basic => {
            Vec3::new(1.78 + peak * 3.34, 0.56 + peak * 0.74, 1.72 + peak * 2.64)
        }
        PlayerAttackMotion::Dash => {
            Vec3::new(1.08 + peak * 1.36, 0.38 + peak * 0.34, 1.46 + peak * 1.40)
        }
        PlayerAttackMotion::Rupture => {
            Vec3::new(1.86 + peak * 3.10, 0.54 + peak * 0.58, 1.52 + peak * 1.74)
        }
        PlayerAttackMotion::Nova => Vec3::splat(1.02 + peak * 1.16),
    };
    Some(PlayerAttackArcPose {
        translation: Vec3::new(
            profile.weapon_x * 0.82,
            0.86 + profile.weapon_y * 0.82,
            0.88 + profile.weapon_z * 0.92,
        ),
        rotation: Quat::from_rotation_y(yaw) * Quat::from_rotation_z(roll),
        scale,
    })
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct PlayerAttackCommitPose {
    translation: Vec3,
    rotation: Quat,
    scale: Vec3,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
enum PlayerAttackVfxAsset {
    SlashArc,
    ShadowTrail,
    BoneFractureEcho,
    ArcaneImpact,
    HitBoneLock,
    HitBoneRune,
}

fn player_attack_arc_vfx(motion: PlayerAttackMotion) -> PlayerAttackVfxAsset {
    match motion {
        PlayerAttackMotion::Basic | PlayerAttackMotion::Dash => PlayerAttackVfxAsset::SlashArc,
        PlayerAttackMotion::Rupture => PlayerAttackVfxAsset::BoneFractureEcho,
        PlayerAttackMotion::Nova => PlayerAttackVfxAsset::ArcaneImpact,
    }
}

fn player_attack_commit_vfx(motion: PlayerAttackMotion) -> PlayerAttackVfxAsset {
    match motion {
        PlayerAttackMotion::Basic => PlayerAttackVfxAsset::HitBoneLock,
        PlayerAttackMotion::Dash => PlayerAttackVfxAsset::ShadowTrail,
        PlayerAttackMotion::Rupture => PlayerAttackVfxAsset::HitBoneRune,
        PlayerAttackMotion::Nova => PlayerAttackVfxAsset::ArcaneImpact,
    }
}

fn player_attack_vfx_handle(
    assets: &GameAssets,
    asset: PlayerAttackVfxAsset,
) -> Handle<WorldAsset> {
    match asset {
        PlayerAttackVfxAsset::SlashArc => assets.slash_arc.clone(),
        PlayerAttackVfxAsset::ShadowTrail => assets.shadow_trail.clone(),
        PlayerAttackVfxAsset::BoneFractureEcho => assets.bone_fracture_echo.clone(),
        PlayerAttackVfxAsset::ArcaneImpact => assets.arcane_impact.clone(),
        PlayerAttackVfxAsset::HitBoneLock => assets.hit_bone_lock.clone(),
        PlayerAttackVfxAsset::HitBoneRune => assets.hit_bone_rune.clone(),
    }
}

fn player_attack_commit_pose(
    motion: PlayerAttackMotion,
    attack_progress: f32,
) -> Option<PlayerAttackCommitPose> {
    if attack_progress >= 0.98 {
        return None;
    }
    let progress = attack_progress.clamp(0.0, 1.0);
    let windup = triangle_peak(progress, 0.16, 0.18);
    let impact = player_attack_motion_weights(motion, progress).1;
    let intensity = windup.max(impact);
    if intensity <= 0.08 {
        return None;
    }

    let (forward, width, length, lift) = match motion {
        PlayerAttackMotion::Basic => (0.64, 0.72, 0.92, 0.055),
        PlayerAttackMotion::Dash => (0.88, 0.58, 1.18, 0.060),
        PlayerAttackMotion::Rupture => (0.78, 1.06, 1.34, 0.065),
        PlayerAttackMotion::Nova => (0.36, 1.20, 1.20, 0.070),
    };
    let pulse = 0.62 + intensity * 0.62 + impact * 0.38;
    let yaw = match motion {
        PlayerAttackMotion::Basic => progress * 0.38,
        PlayerAttackMotion::Dash => 0.12,
        PlayerAttackMotion::Rupture => -0.22 + progress * 0.56,
        PlayerAttackMotion::Nova => progress * std::f32::consts::TAU,
    };

    Some(PlayerAttackCommitPose {
        translation: Vec3::new(0.0, lift, forward + impact * 0.16),
        rotation: Quat::from_rotation_y(yaw),
        scale: Vec3::new(width * pulse, 0.36 + impact * 0.18, length * pulse),
    })
}

fn player_visual_pose(
    moving: bool,
    evading: bool,
    walk_sway: f32,
    walk_step: f32,
    attack_motion: PlayerAttackMotion,
    attack_progress: f32,
) -> PlayerVisualPose {
    let attacking = attack_progress < 1.0;
    let (attack_arc, attack_drive) = if attacking {
        player_attack_motion_weights(attack_motion, attack_progress)
    } else {
        (0.0, 0.0)
    };
    let locomotion = if moving { 1.0 } else { 0.0 };
    let evade_drive = if evading { 1.0 } else { 0.0 };
    let profile = player_attack_motion_profile(attack_motion, attack_progress);
    let bob = locomotion * walk_sway.abs() * 0.64 + evade_drive * 0.10;
    let stride_pitch = locomotion * (-0.66 - evade_drive * 0.20);
    let stride_roll = locomotion * walk_step * 0.96;
    let attack_pitch = profile.body_pitch * attack_arc + profile.body_drive_pitch * attack_drive;
    let attack_yaw = profile.body_yaw * attack_arc + profile.body_drive_yaw * attack_drive;
    let attack_roll = profile.body_roll * attack_arc;
    let sweep = if attacking {
        profile.weapon_roll_start + attack_progress * profile.weapon_roll_sweep
    } else {
        0.0
    };
    let body_scale = Vec3::new(
        1.0 + profile.scale_x * attack_arc,
        1.0 + profile.scale_y * attack_drive,
        1.0 + profile.scale_z * attack_drive,
    );
    let windup = if attacking {
        triangle_peak(attack_progress, 0.16, 0.16)
    } else {
        0.0
    };
    let impact_snap = if attacking {
        triangle_peak(attack_progress, profile.impact_center, profile.impact_width)
    } else {
        0.0
    };

    PlayerVisualPose {
        body_y: bob + attack_arc * profile.body_lift,
        body_yaw: locomotion * walk_sway * 0.16 + attack_yaw,
        body_pitch: stride_pitch + attack_pitch,
        body_roll: stride_roll + attack_roll,
        body_scale,
        weapon_translation: Vec3::new(
            0.50 + attack_arc * profile.weapon_x - windup * profile.weapon_windup_x
                + locomotion * walk_step * 0.26
                + evade_drive * 0.18,
            0.94 + attack_arc * profile.weapon_y
                + locomotion * walk_sway.abs() * 0.30
                + locomotion * walk_step.abs() * 0.08,
            0.23 + attack_drive * profile.weapon_z - windup * profile.weapon_windup_z
                + impact_snap * profile.weapon_impact_z
                + locomotion * walk_sway * 0.46
                + evade_drive * 0.26,
        ),
        weapon_scale: Vec3::splat(0.92 + attack_drive * 0.54 + evade_drive * 0.12),
        weapon_pitch: -0.58 + attack_arc * profile.weapon_pitch,
        weapon_yaw: 0.22 + attack_drive * profile.weapon_yaw + locomotion * walk_step * 0.34,
        weapon_roll: -0.95 + sweep + locomotion * walk_step * 0.62,
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct PlayerHitReactPose {
    body_y: f32,
    body_yaw: f32,
    body_pitch: f32,
    body_roll: f32,
    body_scale_delta: Vec3,
    weapon_translation: Vec3,
    weapon_pitch: f32,
    weapon_roll: f32,
}

fn update_player_hit_react(
    player_entity: Entity,
    hit_react: Option<Mut<PlayerHitReact>>,
    time: &Time,
    commands: &mut Commands,
) -> Option<PlayerHitReactPose> {
    let mut hit_react = hit_react?;
    hit_react.timer.tick(time.delta());
    let pose = player_hit_react_pose(hit_react.kind, hit_react.progress());
    if hit_react.timer.is_finished() {
        commands.entity(player_entity).remove::<PlayerHitReact>();
    }
    Some(pose)
}

fn player_visual_pose_with_hit_react(
    moving: bool,
    evading: bool,
    walk_sway: f32,
    walk_step: f32,
    attack_motion: PlayerAttackMotion,
    attack_progress: f32,
    hit_react: Option<PlayerHitReactPose>,
) -> PlayerVisualPose {
    let mut pose = player_visual_pose(
        moving,
        evading,
        walk_sway,
        walk_step,
        attack_motion,
        attack_progress,
    );
    if let Some(hit_react) = hit_react {
        pose.body_y += hit_react.body_y;
        pose.body_yaw += hit_react.body_yaw;
        pose.body_pitch += hit_react.body_pitch;
        pose.body_roll += hit_react.body_roll;
        pose.body_scale += hit_react.body_scale_delta;
        pose.weapon_translation += hit_react.weapon_translation;
        pose.weapon_pitch += hit_react.weapon_pitch;
        pose.weapon_roll += hit_react.weapon_roll;
    }
    pose
}

fn player_visual_pose_with_turn_lean(
    mut pose: PlayerVisualPose,
    turn_lean: f32,
    evading: bool,
) -> PlayerVisualPose {
    let evade_weight = if evading { 1.25 } else { 1.0 };
    let lean = turn_lean.clamp(-0.50, 0.50) * evade_weight;
    pose.body_yaw += -lean * 0.18;
    pose.body_roll += lean;
    pose.body_pitch += -lean.abs() * 0.12;
    pose.weapon_translation.x += lean * 0.22;
    pose.weapon_translation.z += lean.abs() * 0.10;
    pose.weapon_roll += lean * 0.82;
    pose
}

fn player_hit_react_pose(kind: PlayerHitReactKind, progress: f32) -> PlayerHitReactPose {
    let power = triangle_peak(progress.clamp(0.0, 1.0), 0.28, 0.36);
    match kind {
        PlayerHitReactKind::Health => PlayerHitReactPose {
            body_y: -0.10 * power,
            body_yaw: -0.18 * power,
            body_pitch: 0.42 * power,
            body_roll: -0.24 * power,
            body_scale_delta: Vec3::new(0.05 * power, -0.06 * power, 0.09 * power),
            weapon_translation: Vec3::new(-0.18 * power, -0.10 * power, -0.28 * power),
            weapon_pitch: 0.36 * power,
            weapon_roll: -0.52 * power,
        },
        PlayerHitReactKind::Ward => PlayerHitReactPose {
            body_y: 0.05 * power,
            body_yaw: 0.08 * power,
            body_pitch: -0.16 * power,
            body_roll: 0.10 * power,
            body_scale_delta: Vec3::new(0.10 * power, 0.04 * power, 0.10 * power),
            weapon_translation: Vec3::new(0.06 * power, 0.16 * power, 0.10 * power),
            weapon_pitch: -0.26 * power,
            weapon_roll: 0.34 * power,
        },
        PlayerHitReactKind::Evade => PlayerHitReactPose {
            body_y: 0.07 * power,
            body_yaw: 0.34 * power,
            body_pitch: -0.26 * power,
            body_roll: 0.52 * power,
            body_scale_delta: Vec3::new(-0.04 * power, 0.02 * power, 0.12 * power),
            weapon_translation: Vec3::new(0.34 * power, 0.04 * power, 0.22 * power),
            weapon_pitch: -0.18 * power,
            weapon_roll: 0.78 * power,
        },
    }
}

fn player_attack_motion_weights(motion: PlayerAttackMotion, attack_progress: f32) -> (f32, f32) {
    let progress = attack_progress.clamp(0.0, 1.0);
    let (arc_center, arc_width, impact_center, impact_width) = match motion {
        PlayerAttackMotion::Basic => (0.44, 0.53, 0.46, 0.32),
        PlayerAttackMotion::Dash => (0.30, 0.30, 0.30, 0.20),
        PlayerAttackMotion::Rupture => (0.56, 0.54, 0.62, 0.30),
        PlayerAttackMotion::Nova => (0.50, 0.54, 0.50, 0.46),
    };

    (
        triangle_peak(progress, arc_center, arc_width),
        triangle_peak(progress, impact_center, impact_width),
    )
}

fn triangle_peak(value: f32, center: f32, half_width: f32) -> f32 {
    if half_width <= 0.0 {
        return 0.0;
    }
    (1.0 - (value - center).abs() / half_width).clamp(0.0, 1.0)
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct PlayerAttackMotionProfile {
    body_lift: f32,
    body_yaw: f32,
    body_drive_yaw: f32,
    body_pitch: f32,
    body_drive_pitch: f32,
    body_roll: f32,
    scale_x: f32,
    scale_y: f32,
    scale_z: f32,
    weapon_x: f32,
    weapon_y: f32,
    weapon_z: f32,
    weapon_windup_x: f32,
    weapon_windup_z: f32,
    weapon_impact_z: f32,
    weapon_pitch: f32,
    weapon_yaw: f32,
    weapon_roll_start: f32,
    weapon_roll_sweep: f32,
    impact_center: f32,
    impact_width: f32,
}

fn player_attack_motion_profile(
    motion: PlayerAttackMotion,
    attack_progress: f32,
) -> PlayerAttackMotionProfile {
    let (_, impact_drive) = player_attack_motion_weights(motion, attack_progress);
    match motion {
        PlayerAttackMotion::Basic => PlayerAttackMotionProfile {
            body_lift: 0.13,
            body_yaw: -0.38,
            body_drive_yaw: 1.04,
            body_pitch: -0.46,
            body_drive_pitch: -0.38,
            body_roll: -0.22,
            scale_x: 0.01,
            scale_y: -0.02,
            scale_z: 0.12 * impact_drive,
            weapon_x: 1.06,
            weapon_y: 0.34,
            weapon_z: 1.14,
            weapon_windup_x: 0.52,
            weapon_windup_z: 0.70,
            weapon_impact_z: 0.94,
            weapon_pitch: -0.92,
            weapon_yaw: 1.34,
            weapon_roll_start: -2.35,
            weapon_roll_sweep: 5.35,
            impact_center: 0.44,
            impact_width: 0.20,
        },
        PlayerAttackMotion::Dash => PlayerAttackMotionProfile {
            body_lift: 0.04,
            body_yaw: -0.10,
            body_drive_yaw: 0.26,
            body_pitch: -0.68,
            body_drive_pitch: -0.46,
            body_roll: -0.12,
            scale_x: -0.02,
            scale_y: -0.04,
            scale_z: 0.16 * impact_drive,
            weapon_x: 0.42,
            weapon_y: 0.08,
            weapon_z: 1.58,
            weapon_windup_x: 0.10,
            weapon_windup_z: 0.12,
            weapon_impact_z: 0.92,
            weapon_pitch: -0.34,
            weapon_yaw: 0.72,
            weapon_roll_start: -1.10,
            weapon_roll_sweep: 3.25,
            impact_center: 0.30,
            impact_width: 0.16,
        },
        PlayerAttackMotion::Rupture => PlayerAttackMotionProfile {
            body_lift: 0.17,
            body_yaw: -0.30,
            body_drive_yaw: 0.48,
            body_pitch: -0.64,
            body_drive_pitch: -0.44,
            body_roll: -0.24,
            scale_x: 0.08,
            scale_y: -0.07,
            scale_z: 0.14 * impact_drive,
            weapon_x: 1.24,
            weapon_y: 0.56,
            weapon_z: 0.58,
            weapon_windup_x: 0.30,
            weapon_windup_z: 0.32,
            weapon_impact_z: 0.34,
            weapon_pitch: -0.88,
            weapon_yaw: 0.50,
            weapon_roll_start: -2.05,
            weapon_roll_sweep: 4.05,
            impact_center: 0.60,
            impact_width: 0.20,
        },
        PlayerAttackMotion::Nova => PlayerAttackMotionProfile {
            body_lift: 0.025,
            body_yaw: 0.26,
            body_drive_yaw: -0.30,
            body_pitch: -0.12,
            body_drive_pitch: 0.20,
            body_roll: 0.0,
            scale_x: 0.10 * impact_drive,
            scale_y: -0.10,
            scale_z: 0.10 * impact_drive,
            weapon_x: 0.12,
            weapon_y: -0.24,
            weapon_z: 0.20,
            weapon_windup_x: 0.06,
            weapon_windup_z: 0.08,
            weapon_impact_z: 0.12,
            weapon_pitch: 0.34,
            weapon_yaw: 0.12,
            weapon_roll_start: -0.35,
            weapon_roll_sweep: 1.10,
            impact_center: 0.50,
            impact_width: 0.34,
        },
    }
}

fn motion_trail_interval(speed: f32, evading: bool) -> Option<f32> {
    if !evading && speed < 0.25 {
        return None;
    }
    Some(if evading {
        0.045
    } else {
        (0.16 - speed * 0.012).clamp(0.075, 0.16)
    })
}

fn motion_trail_scale(speed: f32, evading: bool) -> f32 {
    let base = 0.74 + (speed * 0.05).clamp(0.0, 0.36);
    if evading { base * 1.24 } else { base }
}

fn footstep_interval(speed: f32, evading: bool) -> Option<f32> {
    if evading {
        return Some(0.075);
    }
    if speed < 0.35 {
        return None;
    }
    Some((0.24 - speed * 0.018).clamp(0.12, 0.24))
}

fn footstep_scale(speed: f32, evading: bool) -> Vec3 {
    let drive = (speed * 0.055).clamp(0.0, 0.28);
    let evade = if evading { 0.30 } else { 0.0 };
    Vec3::new(0.26 + drive + evade, 0.07, 0.44 + drive * 1.8 + evade)
}

fn attack_trail_interval(motion: PlayerAttackMotion, attack_progress: f32) -> Option<f32> {
    let (arc_weight, impact_weight) = player_attack_motion_weights(motion, attack_progress);
    if arc_weight < 0.34 && impact_weight < 0.26 {
        return None;
    }
    Some(match motion {
        PlayerAttackMotion::Basic => 0.060,
        PlayerAttackMotion::Dash => 0.042,
        PlayerAttackMotion::Rupture => 0.052,
        PlayerAttackMotion::Nova => 0.070,
    })
}

fn attack_trail_lifetime(motion: PlayerAttackMotion) -> f32 {
    match motion {
        PlayerAttackMotion::Basic => 0.18,
        PlayerAttackMotion::Dash => 0.14,
        PlayerAttackMotion::Rupture => 0.22,
        PlayerAttackMotion::Nova => 0.20,
    }
}

fn player_attack_trail_transform(
    origin: Vec3,
    facing_yaw: f32,
    motion: PlayerAttackMotion,
    arc_pose: PlayerAttackArcPose,
) -> Transform {
    let forward = Vec3::new(facing_yaw.sin(), 0.0, facing_yaw.cos());
    let right = Vec3::new(forward.z, 0.0, -forward.x);
    let world_translation = origin
        + right * arc_pose.translation.x
        + Vec3::Y * arc_pose.translation.y
        + forward * arc_pose.translation.z;
    let motion_scale = match motion {
        PlayerAttackMotion::Basic => Vec3::new(0.92, 0.82, 0.96),
        PlayerAttackMotion::Dash => Vec3::new(0.84, 0.72, 1.18),
        PlayerAttackMotion::Rupture => Vec3::new(1.08, 0.92, 1.06),
        PlayerAttackMotion::Nova => Vec3::splat(0.96),
    };
    Transform::from_translation(world_translation)
        .with_rotation(Quat::from_rotation_y(facing_yaw) * arc_pose.rotation)
        .with_scale(arc_pose.scale * motion_scale)
}

fn spawn_player_attack_trail(
    commands: &mut Commands,
    scene: Handle<WorldAsset>,
    origin: Vec3,
    facing_yaw: f32,
    motion: PlayerAttackMotion,
    arc_pose: PlayerAttackArcPose,
) {
    commands.spawn((
        WorldAssetRoot(scene),
        player_attack_trail_transform(origin, facing_yaw, motion, arc_pose),
        PlayerMotionTrail {
            timer: Timer::from_seconds(attack_trail_lifetime(motion), TimerMode::Once),
        },
        Name::new("Player Weapon Afterimage"),
    ));
}

fn spawn_player_motion_trail(
    commands: &mut Commands,
    scene: Handle<WorldAsset>,
    origin: Vec3,
    facing_yaw: f32,
    scale: f32,
) {
    let forward = Vec3::new(facing_yaw.sin(), 0.0, facing_yaw.cos());
    commands.spawn((
        WorldAssetRoot(scene),
        Transform::from_translation(origin - forward * 0.34 + Vec3::Y * 0.10)
            .with_rotation(Quat::from_rotation_y(facing_yaw))
            .with_scale(Vec3::splat(scale)),
        PlayerMotionTrail {
            timer: Timer::from_seconds(0.22, TimerMode::Once),
        },
        Name::new("Player Motion Trail"),
    ));
}

fn player_footstep_transform(origin: Vec3, facing_yaw: f32, side: f32, scale: Vec3) -> Transform {
    let forward = Vec3::new(facing_yaw.sin(), 0.0, facing_yaw.cos());
    let right = Vec3::new(forward.z, 0.0, -forward.x);
    Transform::from_translation(
        origin - forward * 0.14 + right * side.signum() * 0.28 + Vec3::Y * 0.038,
    )
    .with_rotation(Quat::from_rotation_y(facing_yaw + side.signum() * 0.10))
    .with_scale(scale)
}

fn spawn_player_footstep_mark(
    commands: &mut Commands,
    scene: Handle<WorldAsset>,
    origin: Vec3,
    facing_yaw: f32,
    side: f32,
    scale: Vec3,
) {
    commands.spawn((
        WorldAssetRoot(scene),
        player_footstep_transform(origin, facing_yaw, side, scale),
        PlayerMotionTrail {
            timer: Timer::from_seconds(0.32, TimerMode::Once),
        },
        Name::new(if side < 0.0 {
            "Left Footstep Mark"
        } else {
            "Right Footstep Mark"
        }),
    ));
}

fn spawn_surge_activation_vfx(
    commands: &mut Commands,
    assets: &GameAssets,
    player_entity: Entity,
    origin: Vec3,
) {
    commands.spawn((
        WorldAssetRoot(assets.crit_burst.clone()),
        surge_burst_transform(origin),
        PlayerMotionTrail {
            timer: Timer::from_seconds(0.52, TimerMode::Once),
        },
        Name::new("Nephalem Surge Burst"),
    ));
    commands.spawn((
        WorldAssetRoot(assets.holy_impact.clone()),
        surge_ground_ring_transform(origin),
        PlayerMotionTrail {
            timer: Timer::from_seconds(0.66, TimerMode::Once),
        },
        Name::new("Nephalem Surge Ground Ring"),
    ));
    if let Ok(mut entity_commands) = commands.get_entity(player_entity) {
        entity_commands.with_children(|parent| {
            parent.spawn((
                WorldAssetRoot(assets.shadow_burst.clone()),
                surge_aura_transform(),
                PlayerMotionTrail {
                    timer: Timer::from_seconds(1.10, TimerMode::Once),
                },
                Name::new("Nephalem Surge Aura"),
            ));
        });
    }
}

fn surge_burst_transform(origin: Vec3) -> Transform {
    Transform::from_translation(origin + Vec3::Y * 0.82)
        .with_rotation(Quat::from_rotation_y(0.34))
        .with_scale(Vec3::splat(1.42))
}

fn surge_ground_ring_transform(origin: Vec3) -> Transform {
    Transform::from_translation(origin + Vec3::Y * 0.08)
        .with_rotation(Quat::from_rotation_x(std::f32::consts::FRAC_PI_2))
        .with_scale(Vec3::new(2.35, 2.35, 1.0))
}

fn surge_aura_transform() -> Transform {
    Transform::from_xyz(0.0, 0.78, 0.0)
        .with_rotation(Quat::from_rotation_y(-0.22))
        .with_scale(Vec3::splat(1.05))
}

fn active_damage_multiplier(buff: &RelicBuff) -> f32 {
    if buff_timer_active(&buff.timer) {
        buff.damage_multiplier
    } else {
        1.0
    }
}

fn active_speed_multiplier(buff: &RelicBuff) -> f32 {
    if buff_timer_active(&buff.timer) {
        buff.speed_multiplier
    } else {
        1.0
    }
}

pub fn jailed_speed_multiplier(jailed: Option<&Jailed>) -> f32 {
    jailed
        .filter(|jailed| buff_timer_active(&jailed.timer))
        .map(|jailed| jailed.speed_multiplier)
        .unwrap_or(1.0)
        .clamp(0.0, 1.0)
}

pub fn damage_multiplier(buff: &RelicBuff) -> f32 {
    active_damage_multiplier(buff)
}

pub fn buff_seconds_remaining(buff: &RelicBuff) -> f32 {
    if buff_timer_active(&buff.timer) {
        buff.timer.remaining_secs()
    } else {
        0.0
    }
}

pub fn conduit_seconds_remaining(buff: &ConduitBuff) -> f32 {
    if buff_timer_active(&buff.timer) {
        buff.timer.remaining_secs()
    } else {
        0.0
    }
}

fn buff_timer_active(timer: &Timer) -> bool {
    !timer.is_finished() && timer.remaining_secs() > 0.0
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct ShrineResonanceResult {
    active_shrines: u32,
    barrier_gained: f32,
    fury_gained: f32,
}

fn active_shrine_count(relic: &RelicBuff, fortune: &FortuneBuff, conduit: &ConduitBuff) -> u32 {
    u32::from(buff_timer_active(&relic.timer))
        + u32::from(buff_timer_active(&fortune.timer))
        + u32::from(buff_timer_active(&conduit.timer))
}

fn apply_shrine_resonance(
    relic: &RelicBuff,
    fortune: &FortuneBuff,
    conduit: &ConduitBuff,
    barrier: &mut Barrier,
    fury: &mut Fury,
) -> Option<ShrineResonanceResult> {
    let active_shrines = active_shrine_count(relic, fortune, conduit);
    if active_shrines < 2 {
        return None;
    }

    Some(ShrineResonanceResult {
        active_shrines,
        barrier_gained: barrier.grant(8.0 * active_shrines as f32),
        fury_gained: fury.gain(7.0 * active_shrines as f32),
    })
}

fn tick_cooldowns(time: Res<Time>, mut query: Query<&mut SkillCooldowns>) {
    for mut cooldowns in &mut query {
        cooldowns.basic.tick(time.delta());
        cooldowns.dash.tick(time.delta());
        cooldowns.nova.tick(time.delta());
        cooldowns.rupture.tick(time.delta());
    }
}

fn tick_player_motion_trails(
    time: Res<Time>,
    mut commands: Commands,
    mut trails: Query<(Entity, &mut PlayerMotionTrail)>,
) {
    for (entity, mut trail) in &mut trails {
        trail.timer.tick(time.delta());
        if trail.timer.is_finished()
            && let Ok(mut entity_commands) = commands.get_entity(entity)
        {
            entity_commands.try_despawn();
        }
    }
}

fn tick_relic_buff(time: Res<Time>, mut query: Query<&mut RelicBuff, With<Player>>) {
    let Ok(mut buff) = query.single_mut() else {
        return;
    };
    buff.timer.tick(time.delta());
}

fn tick_elixir_buff(time: Res<Time>, mut query: Query<&mut ElixirBuff, With<Player>>) {
    let Ok(mut buff) = query.single_mut() else {
        return;
    };
    buff.timer.tick(time.delta());
}

fn tick_fortune_buff(time: Res<Time>, mut query: Query<&mut FortuneBuff, With<Player>>) {
    let Ok(mut buff) = query.single_mut() else {
        return;
    };
    buff.timer.tick(time.delta());
}

fn tick_glory_buff(time: Res<Time>, mut query: Query<&mut GloryBuff, With<Player>>) {
    let Ok(mut buff) = query.single_mut() else {
        return;
    };
    buff.timer.tick(time.delta());
    if buff.timer.is_finished() {
        buff.stacks = 0;
    }
}

fn tick_surge_buff(time: Res<Time>, mut query: Query<&mut SurgeBuff, With<Player>>) {
    let Ok(mut buff) = query.single_mut() else {
        return;
    };
    buff.timer.tick(time.delta());
}

fn tick_town_portal(time: Res<Time>, mut query: Query<&mut TownPortal, With<Player>>) {
    let Ok(mut portal) = query.single_mut() else {
        return;
    };
    portal.cooldown.tick(time.delta());
}

fn tick_jailed(time: Res<Time>, mut commands: Commands, mut query: Query<(Entity, &mut Jailed)>) {
    for (entity, mut jailed) in &mut query {
        jailed.timer.tick(time.delta());
        if jailed.timer.is_finished() {
            commands.entity(entity).try_remove::<Jailed>();
        }
    }
}

fn tick_evade(time: Res<Time>, mut query: Query<&mut Evade, With<Player>>) {
    let Ok(mut evade) = query.single_mut() else {
        return;
    };
    evade.active.tick(time.delta());
    evade.cooldown.tick(time.delta());
}

fn charge_surge_from_kills(
    mut killed: MessageReader<EnemyKilled>,
    mut combat_events: MessageWriter<CombatEvent>,
    mut player: Query<&mut SurgeBuff, With<Player>>,
) {
    let Ok(mut surge) = player.single_mut() else {
        return;
    };
    let mut gained = 0;
    for kill in killed.read() {
        gained += empowered_kill_weight(&kill.enemy_id);
    }
    if gained == 0 {
        return;
    }
    let actual = grant_surge_charge(&mut surge, gained);
    if actual > 0 && surge.charge == surge.max_charge {
        combat_events.write(CombatEvent {
            text: "Nephalem Surge ready".to_string(),
        });
    }
}

fn extend_surge_from_kills(
    mut killed: MessageReader<EnemyKilled>,
    mut combat_events: MessageWriter<CombatEvent>,
    mut stats: ResMut<RunStats>,
    mut player: Query<&mut SurgeBuff, With<Player>>,
) {
    let Ok(mut surge) = player.single_mut() else {
        return;
    };
    let mut kill_weight = 0;
    for kill in killed.read() {
        kill_weight += empowered_kill_weight(&kill.enemy_id);
    }
    let Some(gained) = extend_active_surge(&mut surge, kill_weight) else {
        return;
    };
    stats.surge_kills = stats.surge_kills.saturating_add(kill_weight);
    combat_events.write(CombatEvent {
        text: format!("Nephalem Surge extended +{gained:.1}s"),
    });
}

fn empowered_kill_weight(enemy_id: &str) -> u32 {
    if enemy_id == "keeper" || enemy_id == "nemesis" {
        3
    } else {
        1
    }
}

fn record_pylon_kills(
    mut killed: MessageReader<EnemyKilled>,
    mut combat_events: MessageWriter<CombatEvent>,
    mut stats: ResMut<RunStats>,
    player: Query<&RelicBuff, With<Player>>,
) {
    let mut kill_weight = 0;
    for kill in killed.read() {
        kill_weight += empowered_kill_weight(&kill.enemy_id);
    }
    if kill_weight == 0 {
        return;
    }
    let Ok(relic) = player.single() else {
        return;
    };
    if !ashen_pylon_active(relic) {
        return;
    }
    stats.pylon_kills = stats.pylon_kills.saturating_add(kill_weight);
    combat_events.write(CombatEvent {
        text: format!(
            "Ashen pylon reaping {}/{}",
            stats.pylon_kills, PYLON_REAPER_KILLS
        ),
    });
}

fn use_surge(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut commands: Commands,
    assets: Res<GameAssets>,
    mut combat_events: MessageWriter<CombatEvent>,
    mut player: Query<(Entity, &mut SurgeBuff, &Transform), With<Player>>,
) {
    if !keyboard.just_pressed(KeyCode::KeyY) {
        return;
    }
    let Ok((entity, mut surge, transform)) = player.single_mut() else {
        return;
    };
    if activate_surge(&mut surge) {
        spawn_surge_activation_vfx(&mut commands, &assets, entity, transform.translation);
        combat_events.write(CombatEvent {
            text: "Nephalem Surge unleashed".to_string(),
        });
    } else {
        combat_events.write(CombatEvent {
            text: format!(
                "Nephalem Surge charging {}/{}",
                surge.charge, surge.max_charge
            ),
        });
    }
}

fn use_town_portal(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut stats: ResMut<RunStats>,
    mut combat_events: MessageWriter<CombatEvent>,
    mut player: Query<(&mut Transform, &mut TownPortal), With<Player>>,
) {
    if !keyboard.just_pressed(KeyCode::KeyT) {
        return;
    }
    let Ok((mut transform, mut portal)) = player.single_mut() else {
        return;
    };
    if buff_timer_active(&portal.cooldown) {
        combat_events.write(CombatEvent {
            text: format!(
                "Town portal recharging {:.0}s",
                portal.cooldown.remaining_secs()
            ),
        });
        return;
    }
    let (target, return_position) = town_portal_target(&portal, transform.translation);
    let returned_to_fight = return_position.is_none();
    transform.translation = target;
    portal.return_position = return_position;
    portal.cooldown = Timer::from_seconds(18.0, TimerMode::Once);
    if returned_to_fight {
        stats.town_portal_returns = stats.town_portal_returns.saturating_add(1);
    }
    combat_events.write(CombatEvent {
        text: if portal.return_position.is_some() {
            "Town portal: returned to the quartermaster".to_string()
        } else {
            "Town portal: returned to the fight".to_string()
        },
    });
}

fn use_evade(
    keyboard: Res<ButtonInput<KeyCode>>,
    gamepads: Query<&Gamepad>,
    mut combat_events: MessageWriter<CombatEvent>,
    mut player: Query<&mut Evade, With<Player>>,
) {
    if !keyboard.just_pressed(KeyCode::ShiftLeft)
        && !gamepad_button_just_pressed(
            &gamepads,
            &[GamepadButton::East, GamepadButton::RightTrigger2],
        )
    {
        return;
    }
    let Ok(mut evade) = player.single_mut() else {
        return;
    };
    if evade_cooldown_remaining(&evade) > 0.0 {
        combat_events.write(CombatEvent {
            text: format!("Evade recharging {:.0}s", evade_cooldown_remaining(&evade)),
        });
        return;
    }
    evade.active = Timer::from_seconds(0.42, TimerMode::Once);
    evade.cooldown = Timer::from_seconds(evade.cooldown_secs, TimerMode::Once);
    combat_events.write(CombatEvent {
        text: "Evade".to_string(),
    });
}

fn activate_relic_shrine(
    mut interactions: MessageReader<InteractableUsed>,
    mut combat_events: MessageWriter<CombatEvent>,
    mut stats: ResMut<RunStats>,
    mut player: Query<
        (
            &mut RelicBuff,
            &FortuneBuff,
            &ConduitBuff,
            &mut Barrier,
            &mut Fury,
        ),
        With<Player>,
    >,
) {
    let Ok((mut relic, fortune, conduit, mut barrier, mut fury)) = player.single_mut() else {
        return;
    };
    for event in interactions.read() {
        if event.kind == InteractableKind::WeaponShrine {
            relic.timer = Timer::from_seconds(28.0, TimerMode::Once);
            relic.damage_multiplier = 1.35;
            relic.speed_multiplier = 1.18;
            combat_events.write(CombatEvent {
                text: "Relic shrine: +35% damage and +18% speed".to_string(),
            });
            if let Some(result) =
                apply_shrine_resonance(&relic, fortune, conduit, &mut barrier, &mut fury)
            {
                stats.shrine_resonance_triggered = true;
                combat_events.write(CombatEvent {
                    text: format!(
                        "Shrine resonance x{}: +{:.0} barrier, +{:.0} fury",
                        result.active_shrines, result.barrier_gained, result.fury_gained
                    ),
                });
            }
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AshenPylonResult {
    pub barrier_gained: f32,
    pub fury_gained: f32,
}

pub fn apply_ashen_pylon(
    relic: &mut RelicBuff,
    barrier: &mut Barrier,
    fury: &mut Fury,
) -> AshenPylonResult {
    relic.timer = Timer::from_seconds(20.0, TimerMode::Once);
    relic.damage_multiplier = 1.55;
    relic.speed_multiplier = 1.24;
    let barrier_before = barrier.current;
    barrier.current = (barrier.current + 42.0).min(barrier.max);
    let fury_gained = fury.gain(38.0);
    AshenPylonResult {
        barrier_gained: barrier.current - barrier_before,
        fury_gained,
    }
}

fn activate_ashen_pylon(
    mut interactions: MessageReader<InteractableUsed>,
    mut combat_events: MessageWriter<CombatEvent>,
    mut player: Query<(&mut RelicBuff, &mut Barrier, &mut Fury), With<Player>>,
) {
    let Ok((mut relic, mut barrier, mut fury)) = player.single_mut() else {
        return;
    };
    for event in interactions.read() {
        if event.kind == InteractableKind::AshenPylon {
            let result = apply_ashen_pylon(&mut relic, &mut barrier, &mut fury);
            combat_events.write(CombatEvent {
                text: format!(
                    "Ashen pylon: +55% damage, +24% speed, +{:.0} barrier, +{:.0} fury",
                    result.barrier_gained, result.fury_gained
                ),
            });
        }
    }
}

fn activate_fortune_shrine(
    mut interactions: MessageReader<InteractableUsed>,
    mut stats: ResMut<RunStats>,
    mut player: Query<
        (
            &RelicBuff,
            &mut FortuneBuff,
            &ConduitBuff,
            &mut Barrier,
            &mut Fury,
        ),
        With<Player>,
    >,
    mut combat_events: MessageWriter<CombatEvent>,
) {
    let Ok((relic, mut fortune, conduit, mut barrier, mut fury)) = player.single_mut() else {
        return;
    };
    for event in interactions.read() {
        if event.kind == InteractableKind::FortuneShrine {
            fortune.timer = Timer::from_seconds(28.0, TimerMode::Once);
            combat_events.write(CombatEvent {
                text: "Fortune shrine: +50% gold, +25% XP, and better drops".to_string(),
            });
            if let Some(result) =
                apply_shrine_resonance(relic, &fortune, conduit, &mut barrier, &mut fury)
            {
                stats.shrine_resonance_triggered = true;
                combat_events.write(CombatEvent {
                    text: format!(
                        "Shrine resonance x{}: +{:.0} barrier, +{:.0} fury",
                        result.active_shrines, result.barrier_gained, result.fury_gained
                    ),
                });
            }
        }
    }
}

fn activate_storm_shrine(
    mut interactions: MessageReader<InteractableUsed>,
    mut stats: ResMut<RunStats>,
    mut player: Query<
        (
            &RelicBuff,
            &FortuneBuff,
            &mut ConduitBuff,
            &mut Barrier,
            &mut Fury,
        ),
        With<Player>,
    >,
    mut combat_events: MessageWriter<CombatEvent>,
) {
    let Ok((relic, fortune, mut conduit, mut barrier, mut fury)) = player.single_mut() else {
        return;
    };
    for event in interactions.read() {
        if event.kind == InteractableKind::StormShrine {
            conduit.timer = Timer::from_seconds(18.0, TimerMode::Once);
            conduit.pulse = Timer::from_seconds(0.65, TimerMode::Repeating);
            conduit.damage = 18.0;
            conduit.radius = 5.4;
            combat_events.write(CombatEvent {
                text: "Storm shrine: conduit lightning awakened".to_string(),
            });
            if let Some(result) =
                apply_shrine_resonance(relic, fortune, &conduit, &mut barrier, &mut fury)
            {
                stats.shrine_resonance_triggered = true;
                combat_events.write(CombatEvent {
                    text: format!(
                        "Shrine resonance x{}: +{:.0} barrier, +{:.0} fury",
                        result.active_shrines, result.barrier_gained, result.fury_gained
                    ),
                });
            }
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct HealingWellResult {
    health_restored: f32,
    barrier_gained: f32,
    potions_restored: u32,
}

fn apply_healing_well(
    health: &mut Health,
    barrier: &mut Barrier,
    potions: &mut PotionBelt,
) -> HealingWellResult {
    let health_before = health.current;
    health.current = health.max;
    let health_restored = health.current - health_before;

    let barrier_target = (barrier.max * 0.5).round();
    let barrier_before = barrier.current;
    if barrier.current < barrier_target {
        barrier.current = barrier_target.min(barrier.max);
    }
    let barrier_gained = barrier.current - barrier_before;

    let potions_before = potions.current;
    potions.current = potions.max;
    potions.cooldown = Timer::from_seconds(0.0, TimerMode::Once);

    HealingWellResult {
        health_restored,
        barrier_gained,
        potions_restored: potions.current.saturating_sub(potions_before),
    }
}

fn activate_healing_well(
    mut commands: Commands,
    mut interactions: MessageReader<InteractableUsed>,
    mut combat_events: MessageWriter<CombatEvent>,
    mut player: Query<(Entity, &mut Health, &mut Barrier, &mut PotionBelt), With<Player>>,
) {
    let Ok((entity, mut health, mut barrier, mut potions)) = player.single_mut() else {
        return;
    };
    for event in interactions.read() {
        if event.kind == InteractableKind::HealingWell {
            let result = apply_healing_well(&mut health, &mut barrier, &mut potions);
            commands.entity(entity).try_remove::<Burning>();
            commands.entity(entity).try_remove::<Jailed>();
            combat_events.write(CombatEvent {
                text: format!(
                    "Renewal well restored {:.0} health, {:.0} barrier, {} potions",
                    result.health_restored, result.barrier_gained, result.potions_restored
                ),
            });
        }
    }
}

fn camera_follow(
    player: Query<&Transform, (With<Player>, Without<Camera3d>)>,
    mut camera: Query<&mut Transform, (With<Camera3d>, Without<Player>)>,
) {
    let Ok(player_transform) = player.single() else {
        return;
    };
    let Ok(mut camera_transform) = camera.single_mut() else {
        return;
    };
    let target = player_transform.translation + Vec3::new(-9.0, 12.0, 13.0);
    camera_transform.translation = camera_transform.translation.lerp(target, 0.08);
    camera_transform.look_at(player_transform.translation, Vec3::Y);
}

fn check_player_death(
    mut commands: Commands,
    mut player: DeathCheckPlayerQuery,
    mut stats: ResMut<RunStats>,
    mut death_report: ResMut<DeathWardReport>,
    mut combat_events: MessageWriter<CombatEvent>,
    mut nemesis_events: MessageWriter<SpawnNemesis>,
    mut next_state: ResMut<NextState<GameState>>,
) {
    let Ok((entity, mut transform, mut health, mut barrier, mut fury, mut ward)) =
        player.single_mut()
    else {
        return;
    };
    if health.current > 0.0 {
        return;
    }
    if ward.can_revive(stats.gold) {
        stats.gold = stats.gold.saturating_sub(ward.gold_penalty);
        ward.revives_used += 1;
        transform.translation = ward.checkpoint;
        health.current = (health.max * 0.45).round().max(1.0);
        barrier.current = (barrier.max * 0.50).round();
        fury.current = (fury.max * 0.35).round();
        commands.entity(entity).try_remove::<Burning>();
        commands.entity(entity).try_remove::<Jailed>();
        nemesis_events.write(SpawnNemesis {
            checkpoint: ward.checkpoint,
        });
        combat_events.write(CombatEvent {
            text: format!(
                "Soul ward revived you at the checkpoint for {} gold",
                ward.gold_penalty
            ),
        });
    } else {
        *death_report = DeathWardReport {
            revives_used: ward.revives_used,
            max_revives: ward.max_revives,
            gold_penalty: ward.gold_penalty,
            gold_available: stats.gold,
            reason: death_ward_failure_reason(&ward, stats.gold).to_string(),
        };
        next_state.set(GameState::GameOver);
    }
}

fn update_death_checkpoint(
    mut interactions: MessageReader<InteractableUsed>,
    mut player: Query<&mut DeathWard, With<Player>>,
    mut combat_events: MessageWriter<CombatEvent>,
) {
    let Ok(mut ward) = player.single_mut() else {
        return;
    };
    for event in interactions.read() {
        if matches!(
            event.kind,
            InteractableKind::Altar | InteractableKind::Merchant | InteractableKind::HealingWell
        ) {
            ward.checkpoint = event.position + Vec3::new(0.0, 0.0, 0.7);
            combat_events.write(CombatEvent {
                text: "Soul ward checkpoint attuned".to_string(),
            });
        }
    }
}

fn despawn_player(mut commands: Commands, query: Query<Entity, With<Player>>) {
    for entity in &query {
        if let Ok(mut entity_commands) = commands.get_entity(entity) {
            entity_commands.try_despawn();
        }
    }
}

fn despawn_player_motion_trails(
    mut commands: Commands,
    query: Query<Entity, (With<PlayerMotionTrail>, Without<ChildOf>)>,
) {
    for entity in &query {
        if let Ok(mut entity_commands) = commands.get_entity(entity) {
            entity_commands.try_despawn();
        }
    }
}

fn despawn_click_move_marker(mut commands: Commands, query: Query<Entity, With<ClickMoveMarker>>) {
    for entity in &query {
        if let Ok(mut entity_commands) = commands.get_entity(entity) {
            entity_commands.try_despawn();
        }
    }
}

fn tick_burning(
    time: Res<Time>,
    mut commands: Commands,
    mut player: Query<(Entity, &mut Health, &mut Barrier, &Evade, &mut Burning), With<Player>>,
) {
    let Ok((entity, mut health, mut barrier, evade, mut burning)) = player.single_mut() else {
        return;
    };
    burning.timer.tick(time.delta());
    apply_player_damage_with_evade(
        &mut health,
        &mut barrier,
        evade,
        burning.dps * time.delta_secs(),
    );
    if burning.timer.is_finished() {
        commands.entity(entity).try_remove::<Burning>();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn relic_buff_multiplier_expires() {
        let mut buff = RelicBuff {
            timer: Timer::from_seconds(1.0, TimerMode::Once),
            damage_multiplier: 1.35,
            speed_multiplier: 1.18,
        };
        assert_eq!(damage_multiplier(&buff), 1.35);
        buff.timer.tick(std::time::Duration::from_secs_f32(1.1));
        assert_eq!(damage_multiplier(&buff), 1.0);
        assert_eq!(buff_seconds_remaining(&buff), 0.0);
    }

    #[test]
    fn ashen_pylon_grants_combat_burst_and_clamps_resources() {
        let mut relic = RelicBuff {
            timer: Timer::from_seconds(0.0, TimerMode::Once),
            damage_multiplier: 1.0,
            speed_multiplier: 1.0,
        };
        let mut barrier = Barrier {
            current: 20.0,
            max: 50.0,
        };
        let mut fury = Fury {
            current: 80.0,
            max: 100.0,
            basic_gain: 18.0,
            dash_cost: 25.0,
            nova_cost: 45.0,
            rupture_cost: 32.0,
        };

        let result = apply_ashen_pylon(&mut relic, &mut barrier, &mut fury);

        assert!((damage_multiplier(&relic) - 1.55).abs() < 0.001);
        assert!((active_speed_multiplier(&relic) - 1.24).abs() < 0.001);
        assert_eq!(barrier.current, 50.0);
        assert_eq!(fury.current, 100.0);
        assert_eq!(
            result,
            AshenPylonResult {
                barrier_gained: 30.0,
                fury_gained: 20.0,
            }
        );
    }

    #[test]
    fn reliquary_momentum_stacks_buff_fury_and_caps() {
        let mut stats = RunStats::default();
        let mut relic = RelicBuff {
            timer: Timer::from_seconds(0.0, TimerMode::Once),
            damage_multiplier: 1.0,
            speed_multiplier: 1.0,
        };
        let mut fury = Fury {
            current: 12.0,
            max: 80.0,
            basic_gain: 18.0,
            dash_cost: 25.0,
            nova_cost: 45.0,
            rupture_cost: 32.0,
        };

        let first = grant_reliquary_momentum(&mut stats, &mut relic, &mut fury, 1);
        assert_eq!(first.stacks, 1);
        assert_eq!(stats.reliquary_momentum, 1);
        assert_eq!(stats.best_reliquary_momentum, 1);
        assert!(first.fury_gained > 0.0);
        assert!(damage_multiplier(&relic) > 1.0);
        assert!(active_speed_multiplier(&relic) > 1.0);

        relic.damage_multiplier = 1.55;
        relic.speed_multiplier = 1.24;
        let capped = grant_reliquary_momentum(&mut stats, &mut relic, &mut fury, 9);
        assert_eq!(capped.stacks, RELIQUARY_MOMENTUM_MAX);
        assert_eq!(stats.best_reliquary_momentum, RELIQUARY_MOMENTUM_MAX);
        assert_eq!(damage_multiplier(&relic), 1.55);
        assert_eq!(active_speed_multiplier(&relic), 1.24);
        assert_eq!(
            reliquary_momentum_summary(&stats),
            "Momentum: 5/5 +40% dmg +20% speed"
        );
    }

    #[test]
    fn ashen_pylon_active_requires_pylon_strength_relic_buff() {
        let mut relic = RelicBuff {
            timer: Timer::from_seconds(10.0, TimerMode::Once),
            damage_multiplier: 1.35,
            speed_multiplier: 1.18,
        };
        assert!(!ashen_pylon_active(&relic));

        let mut barrier = Barrier {
            current: 0.0,
            max: 80.0,
        };
        let mut fury = Fury {
            current: 0.0,
            max: 100.0,
            basic_gain: 18.0,
            dash_cost: 25.0,
            nova_cost: 45.0,
            rupture_cost: 32.0,
        };

        apply_ashen_pylon(&mut relic, &mut barrier, &mut fury);
        assert!(ashen_pylon_active(&relic));

        relic.timer.tick(std::time::Duration::from_secs_f32(20.1));
        assert!(!ashen_pylon_active(&relic));
    }

    #[test]
    fn conduit_buff_reports_remaining_time_only_while_active() {
        let mut buff = ConduitBuff {
            timer: Timer::from_seconds(1.0, TimerMode::Once),
            pulse: Timer::from_seconds(0.65, TimerMode::Repeating),
            damage: 18.0,
            radius: 5.4,
        };

        assert!(conduit_seconds_remaining(&buff) > 0.0);
        buff.timer.tick(std::time::Duration::from_secs_f32(1.1));
        assert_eq!(conduit_seconds_remaining(&buff), 0.0);
    }

    #[test]
    fn jailed_speed_multiplier_clamps_and_expires() {
        let mut jailed = Jailed {
            timer: Timer::from_seconds(1.0, TimerMode::Once),
            speed_multiplier: -0.4,
        };

        assert_eq!(jailed_speed_multiplier(Some(&jailed)), 0.0);
        jailed.timer.tick(std::time::Duration::from_secs_f32(1.1));
        assert_eq!(jailed_speed_multiplier(Some(&jailed)), 1.0);
        assert_eq!(jailed_speed_multiplier(None), 1.0);
    }

    #[test]
    fn death_ward_tracks_revive_budget_and_gold_cost() {
        let ward = DeathWard {
            checkpoint: Vec3::new(1.0, 0.0, 2.0),
            revives_used: 1,
            max_revives: 2,
            gold_penalty: 60,
        };

        assert_eq!(ward.remaining_revives(), 1);
        assert!(ward.can_revive(60));
        assert!(!ward.can_revive(59));

        let exhausted = DeathWard {
            revives_used: 2,
            ..ward
        };
        assert_eq!(exhausted.remaining_revives(), 0);
        assert!(!exhausted.can_revive(1_000));
        assert_eq!(
            death_ward_failure_reason(&exhausted, 1_000),
            "revive charges exhausted"
        );
        assert_eq!(
            death_ward_failure_reason(&ward, 59),
            "not enough gold for the revive tithe"
        );
        assert!(
            DeathWardReport {
                revives_used: 2,
                max_revives: 2,
                gold_penalty: 60,
                gold_available: 20,
                reason: "not enough gold for the revive tithe".to_string(),
            }
            .summary()
            .contains("gold 20/60")
        );
    }

    #[test]
    fn barrier_absorbs_damage_before_health() {
        let mut health = Health {
            current: 50.0,
            max: 100.0,
        };
        let mut barrier = Barrier {
            current: 12.0,
            max: 35.0,
        };

        let taken = apply_player_damage(&mut health, &mut barrier, 20.0);

        assert_eq!(
            taken,
            DamageTaken {
                absorbed: 12.0,
                health_damage: 8.0,
            }
        );
        assert_eq!(barrier.current, 0.0);
        assert_eq!(health.current, 42.0);
    }

    #[test]
    fn evade_prevents_player_damage_while_active() {
        let mut health = Health {
            current: 50.0,
            max: 100.0,
        };
        let mut barrier = Barrier {
            current: 12.0,
            max: 35.0,
        };
        let evade = Evade {
            active: Timer::from_seconds(0.3, TimerMode::Once),
            cooldown: Timer::from_seconds(4.5, TimerMode::Once),
            cooldown_secs: 4.5,
            speed_multiplier: 2.65,
        };

        let taken = apply_player_damage_with_evade(&mut health, &mut barrier, &evade, 20.0);

        assert_eq!(
            taken,
            DamageTaken {
                absorbed: 0.0,
                health_damage: 0.0,
            }
        );
        assert_eq!(barrier.current, 12.0);
        assert_eq!(health.current, 50.0);
    }

    #[test]
    fn evade_speed_multiplier_and_remaining_time_expire() {
        let mut evade = Evade {
            active: Timer::from_seconds(0.3, TimerMode::Once),
            cooldown: Timer::from_seconds(4.5, TimerMode::Once),
            cooldown_secs: 4.5,
            speed_multiplier: 2.65,
        };

        assert_eq!(evade_speed_multiplier(&evade), 2.65);
        assert!(evade_seconds_remaining(&evade) > 0.0);
        assert!(evade_cooldown_remaining(&evade) > 0.0);

        evade.active.tick(std::time::Duration::from_secs_f32(0.31));
        evade.cooldown.tick(std::time::Duration::from_secs_f32(4.6));

        assert_eq!(evade_speed_multiplier(&evade), 1.0);
        assert_eq!(evade_seconds_remaining(&evade), 0.0);
        assert_eq!(evade_cooldown_remaining(&evade), 0.0);
    }

    #[test]
    fn barrier_grant_clamps_to_capacity() {
        let mut barrier = Barrier {
            current: 30.0,
            max: 35.0,
        };

        assert_eq!(barrier.grant(12.0), 5.0);
        assert_eq!(barrier.current, 35.0);
    }

    #[test]
    fn inventory_respects_capacity_and_summarizes_recent_items() {
        let mut inventory = Inventory {
            items: Vec::new(),
            capacity: 2,
        };
        assert!(inventory.add(InventoryItem {
            name: "Iron Fang".to_string(),
            quality: "common".to_string(),
            damage_bonus: 4.0,
            crit_chance: 0.04,
            health_bonus: 0.0,
            armor_bonus: 2.0,
            legendary_power: LegendaryPower::None,
            temper_level: 0,
            socketed_gem: None,
        }));
        assert!(inventory.add(InventoryItem {
            name: "Moonforged Cleaver".to_string(),
            quality: "rare".to_string(),
            damage_bonus: 9.0,
            crit_chance: 0.08,
            health_bonus: 10.0,
            armor_bonus: 6.0,
            legendary_power: LegendaryPower::None,
            temper_level: 0,
            socketed_gem: None,
        }));
        assert!(!inventory.add(InventoryItem {
            name: "Overflow".to_string(),
            quality: "rare".to_string(),
            damage_bonus: 1.0,
            crit_chance: 0.0,
            health_bonus: 0.0,
            armor_bonus: 0.0,
            legendary_power: LegendaryPower::None,
            temper_level: 0,
            socketed_gem: None,
        }));
        assert!(inventory.summary().contains("Moonforged Cleaver"));
    }

    #[test]
    fn inventory_swap_wraps_from_current_equipment() {
        let inventory = Inventory {
            items: vec![
                InventoryItem {
                    name: "A".to_string(),
                    quality: "common".to_string(),
                    damage_bonus: 1.0,
                    crit_chance: 0.01,
                    health_bonus: 0.0,
                    armor_bonus: 0.0,
                    legendary_power: LegendaryPower::None,
                    temper_level: 0,
                    socketed_gem: None,
                },
                InventoryItem {
                    name: "B".to_string(),
                    quality: "rare".to_string(),
                    damage_bonus: 5.0,
                    crit_chance: 0.08,
                    health_bonus: 10.0,
                    armor_bonus: 4.0,
                    legendary_power: LegendaryPower::None,
                    temper_level: 0,
                    socketed_gem: None,
                },
            ],
            capacity: 4,
        };

        assert_eq!(inventory_swap_index(&inventory, "A", 1), Some(1));
        assert_eq!(inventory_swap_index(&inventory, "A", -1), Some(1));
        assert_eq!(inventory_swap_index(&inventory, "Missing", 1), Some(1));
    }

    #[test]
    fn equipping_inventory_item_updates_stats_and_health_delta() {
        let item = InventoryItem {
            name: "B".to_string(),
            quality: "rare".to_string(),
            damage_bonus: 8.0,
            crit_chance: 0.10,
            health_bonus: 18.0,
            armor_bonus: 6.0,
            legendary_power: LegendaryPower::Emberbrand,
            temper_level: 2,
            socketed_gem: None,
        };
        let mut damage_bonus = DamageBonus(2.0);
        let mut equipment = Equipment {
            weapon_name: "A".to_string(),
            quality: "common".to_string(),
            crit_chance: 0.02,
            health_bonus: 5.0,
            armor_bonus: 1.0,
            legendary_power: LegendaryPower::None,
            temper_level: 0,
            socketed_gem: None,
        };
        let mut health = Health {
            current: 50.0,
            max: 105.0,
        };

        equip_inventory_item(&item, &mut damage_bonus, &mut equipment, &mut health);

        assert_eq!(damage_bonus.0, 8.0);
        assert_eq!(equipment.weapon_name, "B");
        assert_eq!(equipment.legendary_power, LegendaryPower::Emberbrand);
        assert_eq!(equipment.temper_level, 2);
        assert_eq!(health.max, 118.0);
        assert_eq!(health.current, 63.0);
    }

    #[test]
    fn manual_equip_summary_surfaces_power_delta_and_build_role() {
        let current_damage = DamageBonus(10.0);
        let current = Equipment {
            weapon_name: "Iron Fang".to_string(),
            quality: "rare".to_string(),
            crit_chance: 0.08,
            health_bonus: 8.0,
            armor_bonus: 8.0,
            legendary_power: LegendaryPower::None,
            temper_level: 0,
            socketed_gem: None,
        };
        let upgrade = InventoryItem {
            name: "Stormglass Reaver".to_string(),
            quality: "legendary".to_string(),
            damage_bonus: 17.0,
            crit_chance: 0.15,
            health_bonus: 24.0,
            armor_bonus: 18.0,
            legendary_power: LegendaryPower::Stormbrand,
            temper_level: 2,
            socketed_gem: Some(SocketedGem {
                kind: GemKind::Emerald,
                rank: 3,
            }),
        };

        let summary = manual_equip_summary(&upgrade, &current_damage, &current);

        assert!(summary.contains("Equipped UPGRADE"));
        assert!(summary.contains("PWR +"));
        assert!(summary.contains("dmg +7"));
        assert!(summary.contains("crit +7%"));
        assert!(summary.contains("tough +"));
        assert!(summary.contains("power Stormbrand chain clear build"));
        assert!(summary.contains("socket [Emerald 3 Resonant]"));
        assert!(summary.contains("temper +2"));
    }

    #[test]
    fn manual_equip_summary_marks_sidegrades_and_downgrades() {
        let current_damage = DamageBonus(15.0);
        let current = Equipment {
            weapon_name: "Ember Fang".to_string(),
            quality: "legendary".to_string(),
            crit_chance: 0.13,
            health_bonus: 10.0,
            armor_bonus: 8.0,
            legendary_power: LegendaryPower::Emberbrand,
            temper_level: 1,
            socketed_gem: None,
        };
        let sidegrade = InventoryItem {
            name: "Frost Wake".to_string(),
            quality: "legendary".to_string(),
            damage_bonus: 14.5,
            crit_chance: 0.12,
            health_bonus: 11.0,
            armor_bonus: 8.0,
            legendary_power: LegendaryPower::Frostbrand,
            temper_level: 1,
            socketed_gem: None,
        };
        let downgrade = InventoryItem {
            name: "Bent Fang".to_string(),
            quality: "common".to_string(),
            damage_bonus: 5.0,
            crit_chance: 0.02,
            health_bonus: 0.0,
            armor_bonus: 1.0,
            legendary_power: LegendaryPower::None,
            temper_level: 0,
            socketed_gem: None,
        };

        let sidegrade_summary = manual_equip_summary(&sidegrade, &current_damage, &current);
        let downgrade_summary = manual_equip_summary(&downgrade, &current_damage, &current);

        assert!(sidegrade_summary.contains("Equipped BUILD SWAP"));
        assert!(sidegrade_summary.contains("power Frostbrand control build"));
        assert!(downgrade_summary.contains("Equipped DOWNGRADE"));
        assert!(downgrade_summary.contains("power lost"));
        assert!(downgrade_summary.contains("dmg -10"));
    }

    #[test]
    fn armory_loadout_captures_and_restores_build_choices() {
        let saved_weapon = InventoryItem {
            name: "Storm Brand".to_string(),
            quality: "legendary".to_string(),
            damage_bonus: 13.0,
            crit_chance: 0.14,
            health_bonus: 8.0,
            armor_bonus: 4.0,
            legendary_power: LegendaryPower::Stormbrand,
            temper_level: 3,
            socketed_gem: Some(SocketedGem {
                kind: GemKind::Emerald,
                rank: 2,
            }),
        };
        let mut inventory = Inventory {
            items: vec![saved_weapon.clone()],
            capacity: 4,
        };
        let mut damage_bonus = DamageBonus(13.0);
        let mut equipment = Equipment {
            weapon_name: saved_weapon.name.clone(),
            quality: saved_weapon.quality.clone(),
            crit_chance: saved_weapon.crit_chance,
            health_bonus: saved_weapon.health_bonus,
            armor_bonus: saved_weapon.armor_bonus,
            legendary_power: saved_weapon.legendary_power,
            temper_level: saved_weapon.temper_level,
            socketed_gem: saved_weapon.socketed_gem,
        };
        let saved_charm = Charm {
            name: "Stormglass Charm".to_string(),
            quality: "legendary".to_string(),
            damage_bonus: 5.0,
            crit_chance: 0.035,
        };
        let mut charm = saved_charm.clone();
        let mut runes = SkillRunes {
            dash: DashRune::Reap,
            nova: NovaRune::Frost,
            rupture: RuptureRune::Hemorrhage,
        };
        let mut codex = LegendaryCodex::default();
        codex.unlock(LegendaryPower::Stormbrand);
        codex.unlock(LegendaryPower::Aegisbrand);
        codex.restore_attunement(LegendaryPower::Aegisbrand);

        let loadout = capture_armory_loadout(
            &damage_bonus,
            &equipment,
            &inventory,
            &charm,
            &runes,
            &codex,
        );

        let other_weapon = InventoryItem {
            name: "Iron Blade".to_string(),
            quality: "common".to_string(),
            damage_bonus: 2.0,
            crit_chance: 0.01,
            health_bonus: 0.0,
            armor_bonus: 0.0,
            legendary_power: LegendaryPower::None,
            temper_level: 0,
            socketed_gem: None,
        };
        let mut swap_health = Health {
            current: 80.0,
            max: 108.0,
        };
        equip_inventory_item(
            &other_weapon,
            &mut damage_bonus,
            &mut equipment,
            &mut swap_health,
        );
        charm = Charm::default();
        runes = SkillRunes::default();
        codex.restore_attunement(LegendaryPower::Stormbrand);
        let mut health = Health {
            current: 80.0,
            max: 108.0,
        };

        restore_armory_loadout(
            &loadout,
            ArmoryRestoreTargets {
                damage_bonus: &mut damage_bonus,
                equipment: &mut equipment,
                inventory: &mut inventory,
                health: &mut health,
                charm: &mut charm,
                runes: &mut runes,
                codex: &mut codex,
            },
        );

        assert_eq!(equipment.weapon_name, "Storm Brand");
        assert_eq!(damage_bonus.0, 13.0);
        assert_eq!(charm, saved_charm);
        assert_eq!(runes.dash, DashRune::Reap);
        assert_eq!(runes.nova, NovaRune::Frost);
        assert_eq!(runes.rupture, RuptureRune::Hemorrhage);
        assert_eq!(codex.attuned, LegendaryPower::Aegisbrand);
        let mut armory = Armory::default();
        armory.save_active(loadout);
        assert!(armory.summary().contains("Armory 1: legendary Storm Brand"));
    }

    #[test]
    fn armory_tracks_three_selectable_loadout_slots() {
        let weapon = InventoryItem {
            name: "Frost Edge".to_string(),
            quality: "rare".to_string(),
            damage_bonus: 7.0,
            crit_chance: 0.04,
            health_bonus: 0.0,
            armor_bonus: 1.0,
            legendary_power: LegendaryPower::None,
            temper_level: 0,
            socketed_gem: None,
        };
        let loadout = ArmoryLoadout {
            weapon,
            charm: Charm::default(),
            dash_rune: DashRune::Cleanse,
            nova_rune: NovaRune::Ember,
            rupture_rune: RuptureRune::Expose,
            codex_attunement: LegendaryPower::None,
        };
        let mut armory = Armory::default();

        armory.select_slot(1);
        armory.save_active(loadout);

        assert!(armory.summary().contains("Armory 2: rare Frost Edge"));
        assert!(armory.summary().contains("1/3 slots"));
        assert_eq!(armory_filled_slots(&armory), 1);
        assert!(armory.active_loadout().is_some());

        armory.select_slot(2);
        assert!(armory.summary().contains("Armory 3: empty"));
        assert!(armory.active_loadout().is_none());
    }

    #[test]
    fn socketing_gems_updates_stats_health_and_inventory() {
        let mut inventory = Inventory {
            items: vec![InventoryItem {
                name: "A".to_string(),
                quality: "rare".to_string(),
                damage_bonus: 8.0,
                crit_chance: 0.08,
                health_bonus: 5.0,
                armor_bonus: 3.0,
                legendary_power: LegendaryPower::None,
                temper_level: 0,
                socketed_gem: None,
            }],
            capacity: 4,
        };
        let mut equipment = Equipment {
            weapon_name: "A".to_string(),
            quality: "rare".to_string(),
            crit_chance: 0.08,
            health_bonus: 5.0,
            armor_bonus: 3.0,
            legendary_power: LegendaryPower::None,
            temper_level: 0,
            socketed_gem: None,
        };
        let mut damage_bonus = DamageBonus(8.0);
        let mut health = Health {
            current: 105.0,
            max: 105.0,
        };

        let gem = socket_or_upgrade_gem(
            SocketedGem::new(GemKind::Amethyst),
            &mut damage_bonus,
            &mut equipment,
            &mut inventory,
            &mut health,
        );

        assert_eq!(gem.rank, 1);
        assert_eq!(equipment.health_bonus, 17.0);
        assert_eq!(health.max, 117.0);
        assert_eq!(health.current, 117.0);
        assert_eq!(inventory.items[0].socketed_gem, Some(gem));

        let gem = socket_or_upgrade_gem(
            SocketedGem::new(GemKind::Amethyst),
            &mut damage_bonus,
            &mut equipment,
            &mut inventory,
            &mut health,
        );

        assert_eq!(gem.rank, 2);
        assert_eq!(equipment.health_bonus, 29.0);
        assert_eq!(health.max, 129.0);
    }

    #[test]
    fn socketed_gem_resonance_unlocks_rank_three_five_and_seven_power() {
        let dormant = SocketedGem {
            kind: GemKind::Ruby,
            rank: 2,
        };
        let resonant = SocketedGem {
            kind: GemKind::Ruby,
            rank: 3,
        };
        let ascendant = SocketedGem {
            kind: GemKind::Ruby,
            rank: 5,
        };
        let paragon = SocketedGem {
            kind: GemKind::Ruby,
            rank: 7,
        };

        assert_eq!(dormant.damage_bonus(), 6.0);
        assert_eq!(dormant.resonance_label(), "Dormant");
        assert_eq!(resonant.damage_bonus(), 13.0);
        assert_eq!(resonant.resonance_label(), "Resonant");
        assert_eq!(ascendant.damage_bonus(), 25.0);
        assert_eq!(ascendant.resonance_label(), "Ascendant");
        assert_eq!(paragon.damage_bonus(), 39.0);
        assert_eq!(paragon.resonance_label(), "Paragon");
        assert!(ascendant.resonance_summary().contains("weapon damage"));
        assert!(paragon.resonance_summary().contains("weapon damage"));
        assert_eq!(gem_label(Some(ascendant)), " [Ruby 5 Ascendant]");
        assert_eq!(gem_label(Some(paragon)), " [Ruby 7 Paragon]");
    }

    #[test]
    fn socketing_rank_three_gem_applies_resonance_delta_once() {
        let mut inventory = Inventory {
            items: vec![InventoryItem {
                name: "A".to_string(),
                quality: "rare".to_string(),
                damage_bonus: 8.0,
                crit_chance: 0.08,
                health_bonus: 5.0,
                armor_bonus: 3.0,
                legendary_power: LegendaryPower::None,
                temper_level: 0,
                socketed_gem: Some(SocketedGem {
                    kind: GemKind::Topaz,
                    rank: 2,
                }),
            }],
            capacity: 4,
        };
        let mut equipment = Equipment {
            weapon_name: "A".to_string(),
            quality: "rare".to_string(),
            crit_chance: 0.08,
            health_bonus: 5.0,
            armor_bonus: 15.0,
            legendary_power: LegendaryPower::None,
            temper_level: 0,
            socketed_gem: Some(SocketedGem {
                kind: GemKind::Topaz,
                rank: 2,
            }),
        };
        let mut damage_bonus = DamageBonus(8.0);
        let mut health = Health {
            current: 105.0,
            max: 105.0,
        };

        let gem = socket_or_upgrade_gem(
            SocketedGem::new(GemKind::Topaz),
            &mut damage_bonus,
            &mut equipment,
            &mut inventory,
            &mut health,
        );

        assert_eq!(gem.rank, 3);
        assert_eq!(equipment.armor_bonus, 29.0);
        assert_eq!(inventory.items[0].armor_bonus, 29.0);
    }

    #[test]
    fn legendary_power_summary_describes_active_power() {
        assert_eq!(LegendaryPower::None.summary(), "");
        assert_eq!(LegendaryPower::None.build_role(), "");
        assert!(LegendaryPower::Emberbrand.summary().contains("ignite"));
        assert!(
            LegendaryPower::Emberbrand
                .summary()
                .contains("Ember/Hemorrhage")
        );
        assert_eq!(LegendaryPower::Emberbrand.build_role(), "ember burn build");
        assert_eq!(LegendaryPower::Emberbrand.boss_boon_key(), "4 Crimson");
        assert_eq!(
            LegendaryPower::Emberbrand.boss_boon_route(),
            "pair with Crimson burst"
        );
        assert!(LegendaryPower::Frostbrand.summary().contains("chill"));
        assert!(
            LegendaryPower::Frostbrand
                .summary()
                .contains("Frost/Expose")
        );
        assert_eq!(LegendaryPower::Frostbrand.build_role(), "control build");
        assert_eq!(LegendaryPower::Frostbrand.boss_boon_key(), "6 Arcane");
        assert_eq!(
            LegendaryPower::Frostbrand.boss_boon_route(),
            "pair with Arcane uptime"
        );
        assert!(LegendaryPower::Stormbrand.summary().contains("chain"));
        assert!(LegendaryPower::Stormbrand.summary().contains("Reap/Frost"));
        assert_eq!(LegendaryPower::Stormbrand.build_role(), "chain clear build");
        assert_eq!(LegendaryPower::Stormbrand.boss_boon_key(), "4 Crimson");
        assert!(LegendaryPower::Soulreaver.summary().contains("steal"));
        assert!(
            LegendaryPower::Soulreaver
                .summary()
                .contains("Reap/Hemorrhage")
        );
        assert_eq!(LegendaryPower::Soulreaver.build_role(), "sustain build");
        assert_eq!(LegendaryPower::Soulreaver.boss_boon_key(), "5 Titan");
        assert_eq!(
            LegendaryPower::Soulreaver.boss_boon_route(),
            "pair with Titan safety"
        );
        assert!(LegendaryPower::Aegisbrand.summary().contains("barrier"));
        assert!(
            LegendaryPower::Aegisbrand
                .summary()
                .contains("Cleanse/Expose")
        );
        assert_eq!(LegendaryPower::Aegisbrand.build_role(), "ward tank build");
        assert_eq!(LegendaryPower::Aegisbrand.boss_boon_key(), "5 Titan");
    }

    #[test]
    fn legendary_codex_unlocks_unique_powers_and_cycles() {
        let mut codex = LegendaryCodex::default();

        assert!(!codex.is_complete());
        assert!(!codex.unlock(LegendaryPower::None));
        for power in LegendaryPower::ALL {
            assert!(codex.unlock(power));
        }
        assert_eq!(codex.attuned, LegendaryPower::Emberbrand);
        assert!(!codex.unlock(LegendaryPower::Emberbrand));
        assert!(codex.is_complete());

        assert_eq!(
            codex.next_power_after(LegendaryPower::None),
            Some(LegendaryPower::Emberbrand)
        );
        assert_eq!(
            codex.next_power_after(LegendaryPower::Emberbrand),
            Some(LegendaryPower::Frostbrand)
        );
        assert_eq!(
            codex.next_power_after(LegendaryPower::Frostbrand),
            Some(LegendaryPower::Stormbrand)
        );
        assert_eq!(
            codex.next_power_after(LegendaryPower::Stormbrand),
            Some(LegendaryPower::Soulreaver)
        );
        assert_eq!(
            codex.next_power_after(LegendaryPower::Soulreaver),
            Some(LegendaryPower::Aegisbrand)
        );
        assert_eq!(
            codex.next_power_after(LegendaryPower::Aegisbrand),
            Some(LegendaryPower::Emberbrand)
        );
        assert_eq!(codex.next_missing_power(), None);
        assert_eq!(
            codex.enchant_target(LegendaryPower::Emberbrand),
            Some(LegendaryPower::Frostbrand)
        );
        assert_eq!(codex.cycle_attunement(), Some(LegendaryPower::Frostbrand));
        assert_eq!(codex.attuned, LegendaryPower::Frostbrand);
        assert_eq!(
            codex.enchant_target(LegendaryPower::Emberbrand),
            Some(LegendaryPower::Frostbrand)
        );
        codex.restore_attunement(LegendaryPower::Soulreaver);
        assert_eq!(codex.attuned, LegendaryPower::Soulreaver);
        codex.restore_attunement(LegendaryPower::None);
        assert_eq!(codex.attuned, LegendaryPower::Emberbrand);
        assert!(codex.summary().contains("Aegisbrand"));
        assert!(codex.summary().contains("Attuned Emberbrand"));
    }

    #[test]
    fn legendary_codex_pursuit_guides_unlocks_and_attuned_enchants() {
        let mut codex = LegendaryCodex::default();
        let mut equipment = Equipment {
            weapon_name: "Keeper Blade".to_string(),
            quality: "rare".to_string(),
            crit_chance: 0.08,
            health_bonus: 0.0,
            armor_bonus: 0.0,
            legendary_power: LegendaryPower::None,
            temper_level: 0,
            socketed_gem: None,
        };

        assert_eq!(
            legendary_codex_pursuit_summary(&codex, &equipment),
            "Codex next: find a legendary weapon power"
        );

        codex.unlock(LegendaryPower::Emberbrand);
        assert_eq!(codex.next_missing_power(), Some(LegendaryPower::Frostbrand));
        assert_eq!(
            legendary_codex_pursuit_summary(&codex, &equipment),
            "Codex next: unlock Frostbrand from legendary drops (control build)"
        );

        for power in [
            LegendaryPower::Frostbrand,
            LegendaryPower::Stormbrand,
            LegendaryPower::Soulreaver,
            LegendaryPower::Aegisbrand,
        ] {
            codex.unlock(power);
        }
        assert_eq!(
            legendary_codex_pursuit_summary(&codex, &equipment),
            "Codex next: enchant Emberbrand at quartermaster -> 4 Crimson (pair with Crimson burst)"
        );

        equipment.legendary_power = LegendaryPower::Emberbrand;
        codex.restore_attunement(LegendaryPower::Soulreaver);
        assert_eq!(
            legendary_codex_pursuit_summary(&codex, &equipment),
            "Codex next: enchant attuned Soulreaver at quartermaster -> 5 Titan (pair with Titan safety)"
        );

        codex.restore_attunement(LegendaryPower::Emberbrand);
        assert_eq!(
            legendary_codex_pursuit_summary(&codex, &equipment),
            "Codex ready: B attune or enchant next Frostbrand -> 6 Arcane (pair with Arcane uptime)"
        );
    }

    #[test]
    fn charms_add_damage_and_crit_as_separate_slot() {
        let damage = DamageBonus(10.0);
        let equipment = Equipment {
            weapon_name: "Blade".to_string(),
            quality: "rare".to_string(),
            crit_chance: 0.12,
            health_bonus: 0.0,
            armor_bonus: 0.0,
            legendary_power: LegendaryPower::None,
            temper_level: 0,
            socketed_gem: None,
        };
        let charm = Charm {
            name: "Stormglass Charm".to_string(),
            quality: "legendary".to_string(),
            damage_bonus: 5.0,
            crit_chance: 0.035,
        };

        assert_eq!(total_damage_bonus(&damage, &equipment, &charm), 17.5);
        assert!((total_crit_chance(&equipment, &charm) - 0.173).abs() < 0.001);
        assert!(charm_power(&charm) > charm_power(&Charm::default()));
        assert!(charm.summary().contains("Stormglass Charm"));
    }

    #[test]
    fn reliquary_resonance_rewards_matching_high_quality_gear() {
        let mut equipment = Equipment {
            weapon_name: "Blade".to_string(),
            quality: "common".to_string(),
            crit_chance: 0.04,
            health_bonus: 0.0,
            armor_bonus: 0.0,
            legendary_power: LegendaryPower::None,
            temper_level: 0,
            socketed_gem: None,
        };
        let mut charm = Charm {
            name: "Gilded Fang Charm".to_string(),
            quality: "rare".to_string(),
            damage_bonus: 3.0,
            crit_chance: 0.018,
        };

        assert_eq!(reliquary_resonance(&equipment, &charm), None);

        equipment.quality = "rare".to_string();
        let rare_resonance = reliquary_resonance(&equipment, &charm).unwrap();
        assert_eq!(rare_resonance.label, "Gilded Concord");
        assert_eq!(rare_resonance.damage_bonus, 1.5);
        assert_eq!(rare_resonance.crit_bonus, 0.012);

        equipment.quality = "legendary".to_string();
        charm.quality = "legendary".to_string();
        let legendary_resonance = reliquary_resonance(&equipment, &charm).unwrap();
        assert_eq!(legendary_resonance.label, "Ashen Harmony");
        assert!(legendary_resonance.damage_bonus > rare_resonance.damage_bonus);
        assert!(legendary_resonance.crit_bonus > rare_resonance.crit_bonus);
        assert!(legendary_resonance.summary().contains("Ashen Harmony"));

        equipment.quality = "ancient".to_string();
        charm.quality = "ancient".to_string();
        let ancient_resonance = reliquary_resonance(&equipment, &charm).unwrap();
        assert_eq!(ancient_resonance.label, "Ancient Concord");
        assert!(ancient_resonance.damage_bonus > legendary_resonance.damage_bonus);
        assert!(ancient_resonance.crit_bonus > legendary_resonance.crit_bonus);

        equipment.quality = "primal".to_string();
        charm.quality = "primal".to_string();
        charm.damage_bonus = 11.0;
        charm.crit_chance = 0.065;
        let primal_resonance = reliquary_resonance(&equipment, &charm).unwrap();
        assert_eq!(primal_resonance.label, "Primal Concord");
        assert!(primal_resonance.damage_bonus > ancient_resonance.damage_bonus);
        assert!(primal_resonance.crit_bonus > ancient_resonance.crit_bonus);
        assert!(charm_power(&charm) > charm_power(&Charm::default()));
        assert!(gear_quality_is_ancient_or_better(&equipment.quality));
        assert!(gear_quality_is_legendary_or_better(&equipment.quality));
    }

    #[test]
    fn themed_reliquary_resonance_overrides_quality_fallback() {
        let equipment = Equipment {
            weapon_name: "Stormcall Reliquary Brand".to_string(),
            quality: "legendary".to_string(),
            crit_chance: 0.15,
            health_bonus: 16.0,
            armor_bonus: 20.0,
            legendary_power: LegendaryPower::Stormbrand,
            temper_level: 0,
            socketed_gem: None,
        };
        let charm = Charm {
            name: "Stormglass Charm".to_string(),
            quality: "legendary".to_string(),
            damage_bonus: 5.0,
            crit_chance: 0.035,
        };

        let resonance = reliquary_resonance(&equipment, &charm).unwrap();

        assert_eq!(
            gear_affinity(&equipment.weapon_name),
            Some(ReliquarySet::Storm)
        );
        assert_eq!(gear_affinity(&charm.name), Some(ReliquarySet::Storm));
        assert_eq!(
            themed_reliquary_set(&equipment, &charm),
            Some(ReliquarySet::Storm)
        );
        assert_eq!(resonance.label, "Storm Covenant");
        assert_eq!(resonance.damage_bonus, 5.5);
        assert_eq!(resonance.crit_bonus, 0.040);
        assert!(resonance.summary().contains("Storm Covenant"));
        assert!(is_themed_reliquary_resonance(&equipment, &charm));
        assert!((total_crit_chance(&equipment, &charm) - 0.225).abs() < 0.001);
    }

    #[test]
    fn different_themes_use_quality_resonance() {
        let equipment = Equipment {
            weapon_name: "Soulreaver Reliquary Fang".to_string(),
            quality: "legendary".to_string(),
            crit_chance: 0.18,
            health_bonus: 26.0,
            armor_bonus: 16.0,
            legendary_power: LegendaryPower::Soulreaver,
            temper_level: 0,
            socketed_gem: None,
        };
        let charm = Charm {
            name: "Stormglass Charm".to_string(),
            quality: "legendary".to_string(),
            damage_bonus: 5.0,
            crit_chance: 0.035,
        };

        let resonance = reliquary_resonance(&equipment, &charm).unwrap();

        assert_eq!(
            gear_affinity(&equipment.weapon_name),
            Some(ReliquarySet::Blood)
        );
        assert_eq!(gear_affinity(&charm.name), Some(ReliquarySet::Storm));
        assert_eq!(themed_reliquary_set(&equipment, &charm), None);
        assert_eq!(resonance.label, "Ashen Harmony");
        assert!(!is_themed_reliquary_resonance(&equipment, &charm));
    }

    #[test]
    fn temper_label_only_shows_upgraded_items() {
        assert_eq!(temper_label(0), "");
        assert_eq!(temper_label(3), " +3");
    }

    #[test]
    fn motion_trail_tuning_marks_moving_and_evading_states() {
        assert_eq!(motion_trail_interval(0.0, false), None);

        let walk_interval = motion_trail_interval(4.0, false).expect("walking trail");
        let sprint_interval = motion_trail_interval(8.0, false).expect("fast trail");
        let evade_interval = motion_trail_interval(0.0, true).expect("evade trail");
        assert!(sprint_interval < walk_interval);
        assert!(evade_interval < sprint_interval);

        let walk_scale = motion_trail_scale(4.0, false);
        let evade_scale = motion_trail_scale(4.0, true);
        assert!(evade_scale > walk_scale);
        assert!(walk_scale > 0.7);
    }

    #[test]
    fn click_move_direction_runs_to_target_and_stops_nearby() {
        let current = Vec3::new(0.0, 0.0, 0.0);
        let target = Some(Vec3::new(3.0, 0.0, 4.0));
        let (direction, reached) =
            movement_direction(Vec3::ZERO, current, target, CLICK_MOVE_STOP_RADIUS);

        assert!(!reached);
        assert!((direction.length() - 1.0).abs() < 0.001);
        assert!((direction.x - 0.6).abs() < 0.001);
        assert!((direction.z - 0.8).abs() < 0.001);

        let (near_direction, near_reached) = movement_direction(
            Vec3::ZERO,
            current,
            Some(Vec3::new(0.10, 0.0, 0.05)),
            CLICK_MOVE_STOP_RADIUS,
        );
        assert_eq!(near_direction, Vec3::ZERO);
        assert!(near_reached);
    }

    #[test]
    fn keyboard_movement_overrides_click_target() {
        let (direction, reached) = movement_direction(
            Vec3::new(-1.0, 0.0, 0.0),
            Vec3::ZERO,
            Some(Vec3::new(4.0, 0.0, 4.0)),
            CLICK_MOVE_STOP_RADIUS,
        );

        assert!(!reached);
        assert_eq!(direction, Vec3::new(-1.0, 0.0, 0.0));
    }

    #[test]
    fn gamepad_left_stick_maps_to_arpg_ground_axis_with_dead_zone() {
        assert_eq!(gamepad_move_axis_from_values(0.08, 0.03), Vec3::ZERO);
        assert_eq!(gamepad_move_axis_from_values(1.0, 0.0), Vec3::X);
        assert_eq!(gamepad_move_axis_from_values(0.0, 1.0), -Vec3::Z);
        assert_eq!(
            gamepad_move_axis_from_values(-0.5, -0.5),
            Vec3::new(-0.5, 0.0, 0.5)
        );
    }

    #[test]
    fn click_move_attack_target_stops_before_enemy_center() {
        let stop_radius = click_attack_stop_radius(2.2);
        let current = Vec3::ZERO;
        let enemy_center = Some(Vec3::new(1.75, 0.0, 0.0));
        let far_enemy_center = Some(Vec3::new(2.35, 0.0, 0.0));

        let (close_direction, close_reached) =
            movement_direction(Vec3::ZERO, current, enemy_center, stop_radius);
        let (far_direction, far_reached) =
            movement_direction(Vec3::ZERO, current, far_enemy_center, stop_radius);

        assert_eq!(close_direction, Vec3::ZERO);
        assert!(close_reached);
        assert_eq!(far_direction, Vec3::X);
        assert!(!far_reached);
        assert!(stop_radius > 1.8);
        assert!(stop_radius < 2.2);
    }

    #[test]
    fn click_move_targets_clamp_to_playable_bounds() {
        let clamped = clamp_player_translation(Vec3::new(40.0, 3.0, -40.0));

        assert_eq!(clamped.x, 11.5);
        assert_eq!(clamped.y, 3.0);
        assert_eq!(clamped.z, -7.5);
    }

    #[test]
    fn click_move_enemy_targeting_uses_flat_cursor_radius() {
        let enemy = Enemy {
            id: "skeleton".to_string(),
            display_name: "Skeleton".to_string(),
            affixes: Vec::new(),
            attack_damage: 8.0,
            attack_kind: crate::data::EnemyAttackKind::Melee,
            attack_range: 1.0,
            attack_timer: Timer::from_seconds(1.0, TimerMode::Once),
            aggro_range: 8.0,
            move_speed: 1.0,
            gold_min: 1,
            gold_max: 2,
            xp_reward: 3,
        };

        assert!(cursor_targets_enemy(
            Vec3::new(0.70, 99.0, 0.0),
            Vec3::ZERO,
            &enemy
        ));
        assert!(!cursor_targets_enemy(
            Vec3::new(0.92, 0.0, 0.0),
            Vec3::ZERO,
            &enemy
        ));
    }

    #[test]
    fn click_move_marker_pose_tracks_ground_target_visibly() {
        let pose = click_move_marker_pose(Vec3::new(2.4, 9.0, -3.2), 1.0);
        let fading = click_move_marker_pose(Vec3::new(2.4, 9.0, -3.2), 0.35);
        let hidden = click_move_marker_pose(Vec3::new(2.4, 9.0, -3.2), 0.0);

        assert_eq!(pose.translation, Vec3::new(2.4, CLICK_MOVE_MARKER_Y, -3.2));
        assert_eq!(pose.visibility, Visibility::Visible);
        assert!(pose.scale.x > 0.3);
        assert_eq!(pose.scale.x, pose.scale.z);
        assert_eq!(fading.visibility, Visibility::Visible);
        assert!(fading.scale.x < pose.scale.x);
        assert_eq!(hidden.visibility, Visibility::Hidden);
    }

    #[test]
    fn click_move_marker_fade_alpha_counts_down() {
        let mut timer = Timer::from_seconds(CLICK_MOVE_MARKER_FADE_SECS, TimerMode::Once);
        assert_eq!(click_move_marker_fade_alpha(&timer), 1.0);

        timer.tick(std::time::Duration::from_secs_f32(
            CLICK_MOVE_MARKER_FADE_SECS * 0.5,
        ));
        let halfway = click_move_marker_fade_alpha(&timer);
        assert!(halfway > 0.45 && halfway < 0.55);

        timer.tick(std::time::Duration::from_secs_f32(
            CLICK_MOVE_MARKER_FADE_SECS,
        ));
        assert_eq!(click_move_marker_fade_alpha(&timer), 0.0);
    }

    #[test]
    fn player_visual_pose_makes_walk_and_evade_readable() {
        let idle = player_visual_pose(false, false, 0.0, 0.0, PlayerAttackMotion::Basic, 1.0);
        let walking = player_visual_pose(true, false, 1.0, 0.5, PlayerAttackMotion::Basic, 1.0);
        let evading = player_visual_pose(true, true, 1.0, 0.5, PlayerAttackMotion::Basic, 1.0);

        assert!(walking.body_y > idle.body_y + 0.14);
        assert!(walking.body_yaw.abs() > idle.body_yaw.abs() + 0.12);
        assert!(walking.body_pitch < idle.body_pitch - 0.30);
        assert!(walking.body_roll.abs() > idle.body_roll.abs() + 0.20);
        assert!(walking.weapon_translation.z > idle.weapon_translation.z + 0.12);
        assert!(evading.body_pitch < walking.body_pitch);
        assert!(evading.weapon_translation.z > walking.weapon_translation.z);
        assert!(evading.weapon_scale.x > walking.weapon_scale.x);
    }

    #[test]
    fn player_hit_react_pose_separates_health_ward_and_evade_feedback() {
        let health = DamageTaken {
            absorbed: 4.0,
            health_damage: 12.0,
        };
        let ward = DamageTaken {
            absorbed: 18.0,
            health_damage: 0.0,
        };
        let evade = DamageTaken {
            absorbed: 0.0,
            health_damage: 0.0,
        };

        assert_eq!(player_hit_react_kind(health), PlayerHitReactKind::Health);
        assert_eq!(player_hit_react_kind(ward), PlayerHitReactKind::Ward);
        assert_eq!(player_hit_react_kind(evade), PlayerHitReactKind::Evade);

        let base = player_visual_pose(false, false, 0.0, 0.0, PlayerAttackMotion::Basic, 1.0);
        let hurt_pose = player_hit_react_pose(PlayerHitReactKind::Health, 0.28);
        let ward_pose = player_hit_react_pose(PlayerHitReactKind::Ward, 0.28);
        let evade_pose = player_hit_react_pose(PlayerHitReactKind::Evade, 0.28);
        let hurt = player_visual_pose_with_hit_react(
            false,
            false,
            0.0,
            0.0,
            PlayerAttackMotion::Basic,
            1.0,
            Some(hurt_pose),
        );
        let blocked = player_visual_pose_with_hit_react(
            false,
            false,
            0.0,
            0.0,
            PlayerAttackMotion::Basic,
            1.0,
            Some(ward_pose),
        );
        let dodged = player_visual_pose_with_hit_react(
            false,
            false,
            0.0,
            0.0,
            PlayerAttackMotion::Basic,
            1.0,
            Some(evade_pose),
        );

        assert!(hurt.body_pitch > base.body_pitch + 0.35);
        assert!(hurt.body_y < base.body_y - 0.08);
        assert!(hurt.weapon_translation.z < base.weapon_translation.z - 0.24);
        assert!(blocked.body_scale.x > base.body_scale.x + 0.08);
        assert!(blocked.weapon_translation.y > base.weapon_translation.y + 0.14);
        assert!(dodged.body_roll > base.body_roll + 0.45);
        assert!(dodged.weapon_translation.x > base.weapon_translation.x + 0.30);
    }

    #[test]
    fn player_walk_cycle_has_planted_steps_not_only_body_drift() {
        let planted = player_foot_pose(1.0, true, false, 0.0);
        let lifted = player_foot_pose(-1.0, true, false, 0.0);
        let right_leg = player_limb_pose(
            PlayerLimbKind::RightLeg,
            true,
            false,
            0.0,
            PlayerAttackMotion::Basic,
            1.0,
        );
        let left_leg = player_limb_pose(
            PlayerLimbKind::LeftLeg,
            true,
            false,
            0.0,
            PlayerAttackMotion::Basic,
            1.0,
        );
        let stride_pose = player_visual_pose(true, false, 1.0, 1.0, PlayerAttackMotion::Basic, 1.0);

        assert!(planted.scale.z > 1.62);
        assert!(planted.translation.x > 0.88);
        assert!(lifted.translation.y > planted.translation.y + 0.30);
        assert!(right_leg.translation.z > left_leg.translation.z + 1.25);
        assert!(stride_pose.body_roll > 0.78);
        assert!(stride_pose.body_pitch < -0.55);
    }

    #[test]
    fn player_animation_faces_horizontal_attack_direction() {
        let mut animation = PlayerAnimation::default();
        animation.face_direction(Vec3::new(1.0, 4.0, 0.0));
        assert!((animation.facing_yaw - std::f32::consts::FRAC_PI_2).abs() < 0.001);
        assert_eq!(animation.turn_lean, 0.0);

        animation.face_direction(Vec3::ZERO);
        assert!((animation.facing_yaw - std::f32::consts::FRAC_PI_2).abs() < 0.001);
    }

    #[test]
    fn player_turn_response_rotates_toward_target_with_body_lean() {
        let response = player_turn_response(0.0, std::f32::consts::FRAC_PI_2, 5.0, 0.016);

        assert!(response.facing_yaw > 0.0);
        assert!(response.facing_yaw < std::f32::consts::FRAC_PI_2);
        assert!(response.turn_lean > 0.0);

        let wrapped = player_turn_response(3.10, -3.10, 5.0, 0.016);
        assert!(wrapped.facing_yaw > 3.10);

        let decayed = decay_player_turn_lean(0.40, 0.05);
        assert!(decayed > 0.0);
        assert!(decayed < 0.40);
    }

    #[test]
    fn player_visual_pose_turn_lean_adds_readable_weight_shift() {
        let base = player_visual_pose(true, false, 0.5, 0.0, PlayerAttackMotion::Basic, 1.0);
        let leaning = player_visual_pose_with_turn_lean(base, 0.38, false);
        let evading = player_visual_pose_with_turn_lean(base, 0.38, true);

        assert!(leaning.body_roll > base.body_roll + 0.34);
        assert!(leaning.body_pitch < base.body_pitch - 0.04);
        assert!(leaning.weapon_translation.x > base.weapon_translation.x + 0.07);
        assert!(evading.body_roll > leaning.body_roll);
        assert!(evading.weapon_roll > leaning.weapon_roll);
    }

    #[test]
    fn player_foot_pose_alternates_visible_stride_marks() {
        let idle = player_foot_pose(-1.0, false, false, 0.0);
        assert_eq!(idle.visibility, Visibility::Hidden);

        let left = player_foot_pose(-1.0, true, false, 0.0);
        let right = player_foot_pose(1.0, true, false, 0.0);
        assert_eq!(left.visibility, Visibility::Visible);
        assert_eq!(right.visibility, Visibility::Visible);
        assert!(left.translation.z < right.translation.z);
        assert!(left.translation.y > right.translation.y + 0.14);
        assert!(right.translation.x > left.translation.x + 0.70);
        assert!(right.scale.z > left.scale.z);
        assert!(right.scale.z > 1.10);

        let evade = player_foot_pose(1.0, false, true, 0.0);
        assert_eq!(evade.visibility, Visibility::Visible);
        assert!(evade.translation.z > idle.translation.z);
        assert!(evade.translation.x > idle.translation.x);
        assert!(evade.scale.z > right.scale.z);
    }

    #[test]
    fn player_limb_pose_gives_visible_walk_cycle_and_weapon_arm() {
        let idle_right_leg = player_limb_pose(
            PlayerLimbKind::RightLeg,
            false,
            false,
            0.0,
            PlayerAttackMotion::Basic,
            1.0,
        );
        let walking_right_leg = player_limb_pose(
            PlayerLimbKind::RightLeg,
            true,
            false,
            0.0,
            PlayerAttackMotion::Basic,
            1.0,
        );
        let walking_left_leg = player_limb_pose(
            PlayerLimbKind::LeftLeg,
            true,
            false,
            0.0,
            PlayerAttackMotion::Basic,
            1.0,
        );
        let idle_weapon_arm = player_limb_pose(
            PlayerLimbKind::RightArm,
            false,
            false,
            0.0,
            PlayerAttackMotion::Basic,
            1.0,
        );

        assert_eq!(idle_right_leg.visibility, Visibility::Visible);
        assert!(walking_right_leg.translation.z > idle_right_leg.translation.z + 0.38);
        assert!(walking_left_leg.translation.z < idle_right_leg.translation.z - 0.38);
        assert!(walking_right_leg.translation.z > walking_left_leg.translation.z + 0.95);
        assert!(walking_right_leg.rotation != walking_left_leg.rotation);
        assert!(idle_weapon_arm.translation.x > 0.52);
        assert!(idle_weapon_arm.scale.y > idle_weapon_arm.scale.x * 2.5);
    }

    #[test]
    fn player_limb_pose_makes_attacks_read_as_arm_and_shoulder_motion() {
        let idle_arm = player_limb_pose(
            PlayerLimbKind::RightArm,
            false,
            false,
            0.0,
            PlayerAttackMotion::Basic,
            1.0,
        );
        let windup_arm = player_limb_pose(
            PlayerLimbKind::RightArm,
            false,
            false,
            0.0,
            PlayerAttackMotion::Basic,
            0.15,
        );
        let impact_arm = player_limb_pose(
            PlayerLimbKind::RightArm,
            false,
            false,
            0.0,
            PlayerAttackMotion::Basic,
            0.45,
        );
        let idle_shoulder = player_limb_pose(
            PlayerLimbKind::ShoulderMantle,
            false,
            false,
            0.0,
            PlayerAttackMotion::Basic,
            1.0,
        );
        let impact_shoulder = player_limb_pose(
            PlayerLimbKind::ShoulderMantle,
            false,
            false,
            0.0,
            PlayerAttackMotion::Basic,
            0.45,
        );
        let rupture_arm = player_limb_pose(
            PlayerLimbKind::RightArm,
            false,
            false,
            0.0,
            PlayerAttackMotion::Rupture,
            0.60,
        );

        assert!(windup_arm.translation.z < idle_arm.translation.z - 0.18);
        assert!(impact_arm.translation.x > idle_arm.translation.x + 0.22);
        assert!(impact_arm.translation.z > windup_arm.translation.z + 0.46);
        assert!(impact_arm.scale.y > idle_arm.scale.y + 0.08);
        assert!(impact_shoulder.translation.y > idle_shoulder.translation.y + 0.04);
        assert!(impact_shoulder.scale.x > idle_shoulder.scale.x);
        assert!(rupture_arm.translation.y > impact_arm.translation.y);
        assert!(rupture_arm.scale.y > impact_arm.scale.y);
    }

    #[test]
    fn player_footstep_marks_are_gait_timed_and_side_offset() {
        assert_eq!(footstep_interval(0.0, false), None);
        let walk_interval = footstep_interval(4.0, false).unwrap();
        let evade_interval = footstep_interval(4.0, true).unwrap();
        assert!(evade_interval < walk_interval);

        let walk_scale = footstep_scale(4.0, false);
        let evade_scale = footstep_scale(4.0, true);
        assert!(evade_scale.x > walk_scale.x);
        assert!(evade_scale.z > walk_scale.z);

        let left = player_footstep_transform(Vec3::ZERO, 0.0, -1.0, walk_scale);
        let right = player_footstep_transform(Vec3::ZERO, 0.0, 1.0, walk_scale);
        assert!(left.translation.x < 0.0);
        assert!(right.translation.x > 0.0);
        assert_eq!(left.translation.z, right.translation.z);
        assert_eq!(left.scale, walk_scale);
    }

    #[test]
    fn player_visual_pose_swings_weapon_through_attacks() {
        let idle = player_visual_pose(false, false, 0.0, 0.0, PlayerAttackMotion::Basic, 1.0);
        let windup = player_visual_pose(false, false, 0.0, 0.0, PlayerAttackMotion::Basic, 0.15);
        let impact = player_visual_pose(false, false, 0.0, 0.0, PlayerAttackMotion::Basic, 0.45);
        let follow_through =
            player_visual_pose(false, false, 0.0, 0.0, PlayerAttackMotion::Basic, 0.85);

        assert!(impact.body_y > idle.body_y + 0.08);
        assert!(impact.body_yaw > windup.body_yaw + 0.48);
        assert!(impact.body_pitch < idle.body_pitch - 0.20);
        assert!(windup.weapon_translation.z < idle.weapon_translation.z - 0.28);
        assert!(impact.weapon_translation.x > idle.weapon_translation.x + 0.48);
        assert!(impact.weapon_translation.z > idle.weapon_translation.z + 1.02);
        assert!(impact.weapon_translation.z > windup.weapon_translation.z + 1.30);
        assert!(impact.weapon_scale.x > idle.weapon_scale.x + 0.24);
        assert!(impact.weapon_yaw > idle.weapon_yaw + 0.80);
        assert!(follow_through.weapon_roll > windup.weapon_roll + 2.7);
    }

    #[test]
    fn basic_attack_reads_as_heavy_weapon_swing_with_broad_arc() {
        let windup = player_visual_pose(false, false, 0.0, 0.0, PlayerAttackMotion::Basic, 0.14);
        let impact = player_visual_pose(false, false, 0.0, 0.0, PlayerAttackMotion::Basic, 0.44);
        let arc = player_attack_arc_pose(PlayerAttackMotion::Basic, 0.44).unwrap();

        assert!(impact.weapon_translation.x > windup.weapon_translation.x + 0.82);
        assert!(impact.weapon_translation.z > windup.weapon_translation.z + 1.70);
        assert!(impact.weapon_roll > windup.weapon_roll + 1.35);
        assert!(impact.body_yaw > windup.body_yaw + 0.45);
        assert!(arc.scale.x > 3.20);
        assert!(arc.scale.z > 2.65);
        assert!(player_attack_arc_pose(PlayerAttackMotion::Basic, 0.88).is_some());
    }

    #[test]
    fn player_attack_arc_pose_only_appears_during_weapon_swing() {
        assert!(player_attack_arc_pose(PlayerAttackMotion::Basic, 0.03).is_none());
        assert!(player_attack_arc_pose(PlayerAttackMotion::Basic, 0.05).is_some());
        assert!(player_attack_arc_pose(PlayerAttackMotion::Basic, 0.97).is_none());
        assert!(player_attack_arc_pose(PlayerAttackMotion::Basic, 0.94).is_some());
        assert!(player_attack_arc_pose(PlayerAttackMotion::Basic, 0.90).is_some());

        let basic = player_attack_arc_pose(PlayerAttackMotion::Basic, 0.42).unwrap();
        let rupture = player_attack_arc_pose(PlayerAttackMotion::Rupture, 0.42).unwrap();
        let rupture_impact = player_attack_arc_pose(PlayerAttackMotion::Rupture, 0.60).unwrap();
        let nova = player_attack_arc_pose(PlayerAttackMotion::Nova, 0.42).unwrap();

        assert!(basic.translation.z > 1.10);
        assert!(basic.scale.x > 2.40);
        assert!(basic.scale.x > basic.scale.y * 2.6);
        assert!(rupture.translation.x > basic.translation.x);
        assert!(rupture_impact.scale.x > basic.scale.x);
        assert!(rupture_impact.scale.x > 2.55);
        assert_ne!(nova.rotation, basic.rotation);
    }

    #[test]
    fn player_attack_commit_pose_marks_windup_and_impact_on_the_ground() {
        assert!(player_attack_commit_pose(PlayerAttackMotion::Basic, 0.01).is_some());
        assert!(player_attack_commit_pose(PlayerAttackMotion::Basic, 0.99).is_none());
        assert!(player_attack_commit_pose(PlayerAttackMotion::Basic, 1.00).is_none());

        let windup = player_attack_commit_pose(PlayerAttackMotion::Basic, 0.16).unwrap();
        let impact = player_attack_commit_pose(PlayerAttackMotion::Basic, 0.46).unwrap();
        let rupture = player_attack_commit_pose(PlayerAttackMotion::Rupture, 0.62).unwrap();
        let nova = player_attack_commit_pose(PlayerAttackMotion::Nova, 0.50).unwrap();

        assert!(windup.translation.y <= 0.08);
        assert!(impact.translation.z > windup.translation.z);
        assert!(impact.scale.x > windup.scale.x);
        assert!(impact.scale.z > windup.scale.z);
        assert!(rupture.scale.x > impact.scale.x);
        assert!(rupture.scale.z > impact.scale.z);
        assert!((nova.scale.x - nova.scale.z).abs() < 0.001);
        assert_ne!(nova.rotation, impact.rotation);
    }

    #[test]
    fn player_attack_vfx_assets_make_skills_visually_distinct() {
        assert_eq!(
            player_attack_arc_vfx(PlayerAttackMotion::Basic),
            PlayerAttackVfxAsset::SlashArc
        );
        assert_eq!(
            player_attack_arc_vfx(PlayerAttackMotion::Dash),
            PlayerAttackVfxAsset::SlashArc
        );
        assert_eq!(
            player_attack_arc_vfx(PlayerAttackMotion::Rupture),
            PlayerAttackVfxAsset::BoneFractureEcho
        );
        assert_eq!(
            player_attack_arc_vfx(PlayerAttackMotion::Nova),
            PlayerAttackVfxAsset::ArcaneImpact
        );
        assert_eq!(
            player_attack_commit_vfx(PlayerAttackMotion::Basic),
            PlayerAttackVfxAsset::HitBoneLock
        );
        assert_eq!(
            player_attack_commit_vfx(PlayerAttackMotion::Dash),
            PlayerAttackVfxAsset::ShadowTrail
        );
        assert_eq!(
            player_attack_commit_vfx(PlayerAttackMotion::Rupture),
            PlayerAttackVfxAsset::HitBoneRune
        );
        assert_eq!(
            player_attack_commit_vfx(PlayerAttackMotion::Nova),
            PlayerAttackVfxAsset::ArcaneImpact
        );
    }

    #[test]
    fn player_attack_trails_follow_weapon_swing_and_skill_weight() {
        assert_eq!(attack_trail_interval(PlayerAttackMotion::Basic, 0.02), None);
        let basic_interval =
            attack_trail_interval(PlayerAttackMotion::Basic, 0.42).expect("basic trail");
        let dash_interval =
            attack_trail_interval(PlayerAttackMotion::Dash, 0.30).expect("dash trail");
        let rupture_interval =
            attack_trail_interval(PlayerAttackMotion::Rupture, 0.60).expect("rupture trail");
        let nova_interval =
            attack_trail_interval(PlayerAttackMotion::Nova, 0.50).expect("nova trail");

        assert!(dash_interval < basic_interval);
        assert!(rupture_interval < basic_interval);
        assert!(nova_interval > dash_interval);
        assert!(attack_trail_lifetime(PlayerAttackMotion::Rupture) > basic_interval);
        assert!(
            attack_trail_lifetime(PlayerAttackMotion::Rupture)
                > attack_trail_lifetime(PlayerAttackMotion::Dash)
        );

        let arc = player_attack_arc_pose(PlayerAttackMotion::Rupture, 0.60).unwrap();
        let transform = player_attack_trail_transform(
            Vec3::new(1.0, 0.2, -2.0),
            std::f32::consts::FRAC_PI_2,
            PlayerAttackMotion::Rupture,
            arc,
        );
        assert!(transform.translation.x > 1.9);
        assert!(transform.translation.y > 1.0);
        assert!(transform.translation.z < -2.2);
        assert!(transform.scale.x > arc.scale.x);
        assert!(transform.scale.z > arc.scale.z);

        let nova_arc = player_attack_arc_pose(PlayerAttackMotion::Nova, 0.50).unwrap();
        let nova =
            player_attack_trail_transform(Vec3::ZERO, 0.0, PlayerAttackMotion::Nova, nova_arc);
        assert!((nova.scale.x - nova.scale.z).abs() < 0.001);
    }

    #[test]
    fn surge_activation_vfx_uses_burst_ring_and_player_aura_poses() {
        let origin = Vec3::new(2.0, 0.0, -3.0);
        let burst = surge_burst_transform(origin);
        let ring = surge_ground_ring_transform(origin);
        let aura = surge_aura_transform();

        assert_eq!(burst.translation, origin + Vec3::Y * 0.82);
        assert!(burst.scale.x > aura.scale.x);
        assert_eq!(ring.translation, origin + Vec3::Y * 0.08);
        assert!(ring.scale.x > burst.scale.x);
        assert!(ring.scale.y > burst.scale.y);
        assert_eq!(aura.translation, Vec3::new(0.0, 0.78, 0.0));
        assert!(aura.scale.x > 1.0);
    }

    #[test]
    fn player_attack_motion_weights_give_skills_distinct_timing() {
        let (_, dash_early) = player_attack_motion_weights(PlayerAttackMotion::Dash, 0.30);
        let (_, rupture_early) = player_attack_motion_weights(PlayerAttackMotion::Rupture, 0.30);
        let (_, dash_late) = player_attack_motion_weights(PlayerAttackMotion::Dash, 0.60);
        let (_, rupture_late) = player_attack_motion_weights(PlayerAttackMotion::Rupture, 0.60);
        let (basic_late_arc, _) = player_attack_motion_weights(PlayerAttackMotion::Basic, 0.82);
        let (nova_late_arc, _) = player_attack_motion_weights(PlayerAttackMotion::Nova, 0.82);

        assert!(dash_early > rupture_early + 0.45);
        assert!(rupture_late > dash_late + 0.90);
        assert!(nova_late_arc > basic_late_arc + 0.12);
    }

    #[test]
    fn player_attack_motions_read_as_different_skill_animations() {
        let basic = player_visual_pose(false, false, 0.0, 0.0, PlayerAttackMotion::Basic, 0.45);
        let dash = player_visual_pose(false, false, 0.0, 0.0, PlayerAttackMotion::Dash, 0.30);
        let rupture = player_visual_pose(false, false, 0.0, 0.0, PlayerAttackMotion::Rupture, 0.60);
        let nova = player_visual_pose(false, false, 0.0, 0.0, PlayerAttackMotion::Nova, 0.50);

        assert!(dash.weapon_translation.z > basic.weapon_translation.z + 0.18);
        assert!(dash.body_pitch < basic.body_pitch - 0.10);
        assert!(rupture.weapon_translation.y > basic.weapon_translation.y + 0.12);
        assert!(rupture.body_pitch < basic.body_pitch - 0.16);
        assert!(rupture.body_y > basic.body_y + 0.02);
        assert!(nova.weapon_translation.y < basic.weapon_translation.y - 0.30);
        assert!(nova.body_scale.x > basic.body_scale.x + 0.04);
    }

    #[test]
    fn fury_gain_clamps_and_spend_requires_enough_resource() {
        let mut fury = Fury {
            current: 20.0,
            max: 50.0,
            basic_gain: 12.0,
            dash_cost: 25.0,
            nova_cost: 45.0,
            rupture_cost: 32.0,
        };
        assert_eq!(fury.gain(40.0), 30.0);
        assert_eq!(fury.current, 50.0);
        assert!(!fury.spend(60.0));
        assert!(fury.spend(25.0));
        assert_eq!(fury.current, 25.0);
    }

    #[test]
    fn potion_drinking_heals_and_respects_cooldown() {
        let mut health = Health {
            current: 40.0,
            max: 100.0,
        };
        let mut potions = PotionBelt {
            current: 2,
            max: 5,
            heal_amount: 45.0,
            cooldown: Timer::from_seconds(0.0, TimerMode::Once),
            cooldown_secs: potion_cooldown_secs_for_capacity(5),
        };

        assert_eq!(try_drink_potion(&mut health, &mut potions), Some(45.0));
        assert_eq!(health.current, 85.0);
        assert_eq!(potions.current, 1);
        assert!(potion_seconds_remaining(&potions) > 0.0);
        assert_eq!(try_drink_potion(&mut health, &mut potions), None);

        potions
            .cooldown
            .tick(std::time::Duration::from_secs_f32(potions.cooldown_secs));

        assert_eq!(try_drink_potion(&mut health, &mut potions), Some(15.0));
        assert_eq!(health.current, 100.0);
        assert_eq!(potions.current, 0);
    }

    #[test]
    fn last_stand_potion_threshold_uses_pre_heal_health() {
        assert!(last_stand_potion_eligible(30.0, 100.0));
        assert!(last_stand_potion_eligible(29.9, 100.0));
        assert!(!last_stand_potion_eligible(31.0, 100.0));
        assert!(!last_stand_potion_eligible(1.0, 0.0));
    }

    #[test]
    fn healing_well_restores_health_barrier_potions_and_cooldown() {
        let mut health = Health {
            current: 35.0,
            max: 120.0,
        };
        let mut barrier = Barrier {
            current: 4.0,
            max: 30.0,
        };
        let mut potions = PotionBelt {
            current: 1,
            max: 5,
            heal_amount: 45.0,
            cooldown: Timer::from_seconds(9.0, TimerMode::Once),
            cooldown_secs: potion_cooldown_secs_for_capacity(5),
        };
        potions.cooldown.reset();

        let result = apply_healing_well(&mut health, &mut barrier, &mut potions);

        assert_eq!(result.health_restored, 85.0);
        assert_eq!(result.barrier_gained, 11.0);
        assert_eq!(result.potions_restored, 4);
        assert_eq!(health.current, 120.0);
        assert_eq!(barrier.current, 15.0);
        assert_eq!(potions.current, 5);
        assert_eq!(potion_seconds_remaining(&potions), 0.0);
    }

    #[test]
    fn potion_cooldown_improves_with_belt_capacity() {
        assert!(potion_cooldown_secs_for_capacity(7) < potion_cooldown_secs_for_capacity(5));
        assert_eq!(potion_cooldown_secs_for_capacity(99), 8.0);
    }

    #[test]
    fn massacre_feedback_spotlights_reward_tiers_without_spamming() {
        assert_eq!(massacre_feedback_event(2, 0, 0), None);
        assert_eq!(massacre_feedback_event(4, 5, 5), None);

        let first_tier = massacre_feedback_event(3, 5, 5).unwrap();
        assert_eq!(first_tier.text, "Massacre 3x +5 XP +5 gold");
        assert!(first_tier.shake_intensity > 0.0);

        let carnage = massacre_feedback_event(5, 10, 10).unwrap();
        assert_eq!(carnage.text, "Carnage 5x +10 XP +10 gold");
        assert!(carnage.shake_intensity > first_tier.shake_intensity);

        assert!(massacre_feedback_event(9, 15, 15).is_none());
        let annihilation = massacre_feedback_event(20, 15, 15).unwrap();
        assert_eq!(annihilation.text, "Annihilation 20x +15 XP +15 gold");
        assert!(annihilation.shake_intensity <= 0.075);
    }

    #[test]
    fn fortune_buff_scales_rewards_only_while_active() {
        let mut fortune = FortuneBuff {
            timer: Timer::from_seconds(8.0, TimerMode::Once),
            xp_multiplier: 1.25,
            gold_multiplier: 1.50,
        };
        fortune.timer.reset();

        assert_eq!(fortune_xp_reward(20, &fortune), 25);
        assert_eq!(fortune_gold_reward(10, &fortune), 15);
        assert!(fortune_magic_find_active(&fortune));

        fortune.timer.tick(std::time::Duration::from_secs_f32(8.1));
        assert_eq!(fortune_xp_reward(20, &fortune), 20);
        assert_eq!(fortune_gold_reward(10, &fortune), 10);
        assert!(!fortune_magic_find_active(&fortune));
        assert_eq!(fortune_seconds_remaining(&fortune), 0.0);
    }

    #[test]
    fn shrine_resonance_requires_multiple_active_shrine_buffs() {
        let active_relic = RelicBuff {
            timer: Timer::from_seconds(8.0, TimerMode::Once),
            damage_multiplier: 1.35,
            speed_multiplier: 1.18,
        };
        let active_fortune = FortuneBuff {
            timer: Timer::from_seconds(8.0, TimerMode::Once),
            xp_multiplier: 1.25,
            gold_multiplier: 1.50,
        };
        let active_conduit = ConduitBuff {
            timer: Timer::from_seconds(8.0, TimerMode::Once),
            pulse: Timer::from_seconds(0.65, TimerMode::Repeating),
            damage: 18.0,
            radius: 5.4,
        };
        let inactive_fortune = FortuneBuff {
            timer: Timer::from_seconds(0.0, TimerMode::Once),
            xp_multiplier: 1.25,
            gold_multiplier: 1.50,
        };
        let inactive_conduit = ConduitBuff {
            timer: Timer::from_seconds(0.0, TimerMode::Once),
            pulse: Timer::from_seconds(0.65, TimerMode::Repeating),
            damage: 18.0,
            radius: 5.4,
        };

        let mut barrier = Barrier {
            current: 5.0,
            max: 60.0,
        };
        let mut fury = Fury {
            current: 10.0,
            max: 60.0,
            basic_gain: 12.0,
            dash_cost: 25.0,
            nova_cost: 45.0,
            rupture_cost: 32.0,
        };

        assert_eq!(
            apply_shrine_resonance(
                &active_relic,
                &inactive_fortune,
                &inactive_conduit,
                &mut barrier,
                &mut fury,
            ),
            None
        );
        assert_eq!(barrier.current, 5.0);
        assert_eq!(fury.current, 10.0);

        let result = apply_shrine_resonance(
            &active_relic,
            &active_fortune,
            &inactive_conduit,
            &mut barrier,
            &mut fury,
        )
        .unwrap();

        assert_eq!(
            result,
            ShrineResonanceResult {
                active_shrines: 2,
                barrier_gained: 16.0,
                fury_gained: 14.0,
            }
        );

        let result = apply_shrine_resonance(
            &active_relic,
            &active_fortune,
            &active_conduit,
            &mut barrier,
            &mut fury,
        )
        .unwrap();

        assert_eq!(result.active_shrines, 3);
        assert_eq!(result.barrier_gained, 24.0);
        assert_eq!(result.fury_gained, 21.0);
        assert_eq!(barrier.current, 45.0);
        assert_eq!(fury.current, 45.0);
    }

    #[test]
    fn town_portal_reports_remaining_cooldown() {
        let mut portal = TownPortal {
            cooldown: Timer::from_seconds(18.0, TimerMode::Once),
            destination: Vec3::new(-4.8, 0.0, 4.4),
            return_position: None,
        };
        portal.cooldown.reset();

        assert!(town_portal_seconds_remaining(&portal) > 17.0);
        portal
            .cooldown
            .tick(std::time::Duration::from_secs_f32(18.1));
        assert_eq!(town_portal_seconds_remaining(&portal), 0.0);
    }

    #[test]
    fn town_portal_toggles_between_field_and_town() {
        let mut portal = TownPortal {
            cooldown: Timer::from_seconds(0.0, TimerMode::Once),
            destination: Vec3::new(-4.8, 0.0, 4.4),
            return_position: None,
        };
        let field = Vec3::new(6.0, 0.0, -3.0);
        let (target, return_position) = town_portal_target(&portal, field);
        assert_eq!(target, portal.destination);
        assert_eq!(return_position, Some(field));

        portal.return_position = return_position;
        let (target, return_position) = town_portal_target(&portal, portal.destination);
        assert_eq!(target, field);
        assert_eq!(return_position, None);
    }

    #[test]
    fn chapter_boons_apply_distinct_permanent_bonuses() {
        let mut damage = DamageBonus(4.0);
        let mut equipment = Equipment {
            weapon_name: "A".to_string(),
            quality: "common".to_string(),
            crit_chance: 0.04,
            health_bonus: 0.0,
            armor_bonus: 0.0,
            legendary_power: LegendaryPower::None,
            temper_level: 0,
            socketed_gem: None,
        };
        let mut health = Health {
            current: 70.0,
            max: 100.0,
        };
        let mut fury = Fury {
            current: 20.0,
            max: 100.0,
            basic_gain: 18.0,
            dash_cost: 25.0,
            nova_cost: 45.0,
            rupture_cost: 32.0,
        };

        apply_chapter_boon(
            ChapterBoonChoice::CrimsonOath,
            false,
            0,
            &mut damage,
            &mut equipment,
            &mut health,
            &mut fury,
        );
        assert_eq!(damage.0, 10.0);
        assert!((equipment.crit_chance - 0.07).abs() < 0.001);

        apply_chapter_boon(
            ChapterBoonChoice::ArcaneMomentum,
            false,
            0,
            &mut damage,
            &mut equipment,
            &mut health,
            &mut fury,
        );
        assert_eq!(fury.max, 120.0);
        assert_eq!(fury.current, 40.0);
        assert_eq!(fury.dash_cost, 20.0);
        assert_eq!(fury.nova_cost, 37.0);
        assert_eq!(fury.rupture_cost, 26.0);
    }

    #[test]
    fn reliquary_momentum_empowers_arcane_boon_economy() {
        let mut damage = DamageBonus(4.0);
        let mut equipment = Equipment {
            weapon_name: "A".to_string(),
            quality: "common".to_string(),
            crit_chance: 0.04,
            health_bonus: 0.0,
            armor_bonus: 0.0,
            legendary_power: LegendaryPower::None,
            temper_level: 0,
            socketed_gem: None,
        };
        let mut health = Health {
            current: 70.0,
            max: 100.0,
        };
        let mut fury = Fury {
            current: 20.0,
            max: 100.0,
            basic_gain: 18.0,
            dash_cost: 25.0,
            nova_cost: 45.0,
            rupture_cost: 32.0,
        };
        let stats = RunStats {
            reliquary_momentum: 4,
            best_reliquary_momentum: 4,
            best_massacre_streak: 9,
            ..default()
        };

        assert!(reliquary_boon_momentum_ready(&stats));
        assert!(chapter_boon_is_empowered(
            ChapterBoonChoice::ArcaneMomentum,
            &stats
        ));
        assert!(chapter_boon_choice_prompt(&stats).contains("Arcane empowered"));
        assert!(chapter_boon_choice_prompt(&stats).contains("Pick 6 fury/skill route"));

        apply_chapter_boon(
            ChapterBoonChoice::ArcaneMomentum,
            true,
            reliquary_boon_momentum_stacks(&stats),
            &mut damage,
            &mut equipment,
            &mut health,
            &mut fury,
        );

        assert_eq!(fury.max, 146.0);
        assert_eq!(fury.current, 66.0);
        assert_eq!(fury.basic_gain, 28.0);
        assert_eq!(fury.dash_cost, 13.0);
        assert_eq!(fury.nova_cost, 25.0);
        assert_eq!(fury.rupture_cost, 19.0);

        let boon = ChapterBoon {
            choice: ChapterBoonChoice::ArcaneMomentum,
            empowered: true,
            momentum_stacks: 4,
        };
        assert!(boon.summary().contains("Momentum 4x infused"));
    }

    #[test]
    fn empowered_chapter_boons_scale_from_run_performance() {
        let mut damage = DamageBonus(4.0);
        let mut equipment = Equipment {
            weapon_name: "A".to_string(),
            quality: "common".to_string(),
            crit_chance: 0.04,
            health_bonus: 0.0,
            armor_bonus: 0.0,
            legendary_power: LegendaryPower::None,
            temper_level: 0,
            socketed_gem: None,
        };
        let mut health = Health {
            current: 70.0,
            max: 100.0,
        };
        let mut fury = Fury {
            current: 20.0,
            max: 100.0,
            basic_gain: 18.0,
            dash_cost: 25.0,
            nova_cost: 45.0,
            rupture_cost: 32.0,
        };
        let stats = RunStats {
            best_massacre_streak: 8,
            shrine_resonance_triggered: true,
            surge_kills: SURGE_CHAIN_KILLS,
            ..default()
        };

        assert!(chapter_boon_is_empowered(
            ChapterBoonChoice::CrimsonOath,
            &stats
        ));
        assert!(chapter_boon_is_empowered(
            ChapterBoonChoice::TitanWard,
            &stats
        ));
        assert!(chapter_boon_is_empowered(
            ChapterBoonChoice::ArcaneMomentum,
            &stats
        ));
        assert!(chapter_boon_choice_prompt(&stats).contains("Crimson empowered"));
        assert!(chapter_boon_choice_prompt(&stats).contains("Pick 4 burst/DPS route"));

        let titan_stats = RunStats {
            potions_used: 2,
            ..default()
        };
        assert!(chapter_boon_choice_prompt(&titan_stats).contains("Pick 5 survival route"));

        let arcane_stats = RunStats {
            surge_kills: 1,
            ..default()
        };
        assert!(chapter_boon_choice_prompt(&arcane_stats).contains("Pick 6 fury/skill route"));

        let neutral_stats = RunStats {
            best_massacre_streak: 5,
            ..default()
        };
        assert!(
            chapter_boon_choice_prompt(&neutral_stats)
                .contains("Pick 4 damage, 5 safety, 6 skills")
        );

        apply_chapter_boon(
            ChapterBoonChoice::TitanWard,
            true,
            0,
            &mut damage,
            &mut equipment,
            &mut health,
            &mut fury,
        );
        assert_eq!(equipment.health_bonus, 42.0);
        assert_eq!(equipment.armor_bonus, 16.0);
        assert_eq!(health.max, 142.0);
        assert_eq!(health.current, 112.0);
    }

    #[test]
    fn talents_scale_damage_and_cooldowns() {
        let talents = Talents {
            points: 0,
            wrath: 2,
            vigor: 1,
            focus: 3,
        };
        assert!((talents.damage_multiplier() - 1.16).abs() < 0.001);
        assert!((talents.cooldown_multiplier() - 0.82).abs() < 0.001);
        assert!(talents.summary().contains("Wrath 2"));
    }

    #[test]
    fn ember_paragon_defaults_and_rank_rewards_rotate() {
        let paragon = EmberParagon::default();
        assert_eq!(paragon.rank, 0);
        assert_eq!(paragon.next_rank_xp, EMBER_PARAGON_BASE_XP);
        assert!(paragon.summary().contains("Ember Paragon 0"));
        assert_eq!(next_ember_paragon_xp(100), 130);

        let mut health = Health {
            current: 50.0,
            max: 100.0,
        };
        let mut damage = DamageBonus(4.0);
        let mut equipment = Equipment {
            weapon_name: "Initiate Blade".to_string(),
            quality: "common".to_string(),
            crit_chance: 0.03,
            health_bonus: 0.0,
            armor_bonus: 0.0,
            legendary_power: LegendaryPower::None,
            temper_level: 0,
            socketed_gem: None,
        };

        apply_ember_paragon_rank(1, &mut health, &mut damage, &mut equipment);
        apply_ember_paragon_rank(2, &mut health, &mut damage, &mut equipment);
        apply_ember_paragon_rank(3, &mut health, &mut damage, &mut equipment);
        apply_ember_paragon_rank(4, &mut health, &mut damage, &mut equipment);

        assert!((damage.0 - 5.5).abs() < 0.001);
        assert_eq!(health.max, 112.0);
        assert_eq!(health.current, 62.0);
        assert!((equipment.crit_chance - 0.035).abs() < 0.001);
        assert_eq!(equipment.armor_bonus, 4.0);
    }

    #[test]
    fn skill_runes_cycle_and_summarize() {
        let mut runes = SkillRunes::default();
        assert_eq!(runes.dash, DashRune::Cleanse);
        assert_eq!(runes.nova, NovaRune::Ember);
        assert_eq!(runes.rupture, RuptureRune::Expose);

        runes.dash = runes.dash.next();
        runes.nova = runes.nova.next();
        runes.rupture = runes.rupture.next();

        assert_eq!(runes.dash, DashRune::Reap);
        assert_eq!(runes.nova, NovaRune::Frost);
        assert_eq!(runes.rupture, RuptureRune::Hemorrhage);
        assert!(runes.summary().contains("Dash Reap"));
        assert!(runes.summary().contains("Rupture Hemorrhage"));
    }

    #[test]
    fn armor_mitigates_damage_with_floor() {
        assert_eq!(mitigated_damage(20.0, 100.0), 10.0);
    }

    #[test]
    fn active_elixir_armor_expires() {
        let mut buff = ElixirBuff {
            timer: Timer::from_seconds(1.0, TimerMode::Once),
            kind: ElixirKind::Iron,
            armor_bonus: 35.0,
            damage_multiplier: 1.0,
            speed_multiplier: 1.0,
        };
        assert_eq!(active_elixir_armor(&buff), 35.0);
        buff.timer.tick(std::time::Duration::from_secs_f32(1.1));
        assert_eq!(active_elixir_armor(&buff), 0.0);
    }

    #[test]
    fn elixir_kinds_cycle_and_configure_distinct_buffs() {
        assert_eq!(ElixirKind::Iron.next(), ElixirKind::Wrath);
        assert_eq!(ElixirKind::Wrath.next(), ElixirKind::Haste);
        assert_eq!(ElixirKind::Haste.next(), ElixirKind::Iron);

        let mut buff = ElixirBuff {
            timer: Timer::from_seconds(1.0, TimerMode::Once),
            kind: ElixirKind::Iron,
            armor_bonus: 0.0,
            damage_multiplier: 1.0,
            speed_multiplier: 1.0,
        };

        configure_elixir_buff(ElixirKind::Iron, 35.0, &mut buff);
        assert_eq!(buff.kind, ElixirKind::Iron);
        assert_eq!(active_elixir_armor(&buff), 35.0);
        assert_eq!(elixir_damage_multiplier(&buff), 1.0);
        assert_eq!(elixir_speed_multiplier(&buff), 1.0);

        configure_elixir_buff(ElixirKind::Wrath, 35.0, &mut buff);
        assert_eq!(buff.kind, ElixirKind::Wrath);
        assert_eq!(active_elixir_armor(&buff), 0.0);
        assert!((elixir_damage_multiplier(&buff) - 1.18).abs() < 0.001);
        assert_eq!(elixir_speed_multiplier(&buff), 1.0);

        configure_elixir_buff(ElixirKind::Haste, 35.0, &mut buff);
        assert_eq!(buff.kind, ElixirKind::Haste);
        assert_eq!(active_elixir_armor(&buff), 0.0);
        assert_eq!(elixir_damage_multiplier(&buff), 1.0);
        assert!((elixir_speed_multiplier(&buff) - 1.20).abs() < 0.001);
    }

    #[test]
    fn glory_buff_stacks_and_expires() {
        let mut buff = GloryBuff {
            timer: Timer::from_seconds(0.0, TimerMode::Once),
            stacks: 0,
            max_stacks: 3,
            damage_per_stack: 0.06,
            speed_per_stack: 0.04,
        };

        assert_eq!(grant_glory(&mut buff), 1);
        assert_eq!(grant_glory(&mut buff), 2);
        assert_eq!(grant_glory(&mut buff), 3);
        assert_eq!(grant_glory(&mut buff), 3);
        assert!((glory_damage_multiplier(&buff) - 1.18).abs() < 0.001);
        assert!((glory_speed_multiplier(&buff) - 1.12).abs() < 0.001);
        buff.timer.tick(std::time::Duration::from_secs_f32(16.1));
        assert_eq!(glory_damage_multiplier(&buff), 1.0);
        assert_eq!(glory_speed_multiplier(&buff), 1.0);
    }

    #[test]
    fn surge_charges_activates_and_expires() {
        let mut buff = SurgeBuff {
            timer: Timer::from_seconds(0.0, TimerMode::Once),
            charge: 0,
            max_charge: 9,
            duration_secs: 9.0,
            damage_multiplier: 1.24,
            speed_multiplier: 1.16,
            fury_multiplier: 1.35,
        };

        assert_eq!(grant_surge_charge(&mut buff, 4), 4);
        assert_eq!(buff.charge, 4);
        assert!(!activate_surge(&mut buff));
        assert_eq!(grant_surge_charge(&mut buff, 99), 5);
        assert_eq!(buff.charge, 9);
        assert!(activate_surge(&mut buff));
        assert_eq!(buff.charge, 0);
        assert!((surge_damage_multiplier(&buff) - 1.24).abs() < 0.001);
        assert!((surge_speed_multiplier(&buff) - 1.16).abs() < 0.001);
        assert!((surge_fury_multiplier(&buff) - 1.35).abs() < 0.001);

        buff.timer.tick(std::time::Duration::from_secs_f32(9.1));

        assert_eq!(surge_damage_multiplier(&buff), 1.0);
        assert_eq!(surge_speed_multiplier(&buff), 1.0);
        assert_eq!(surge_fury_multiplier(&buff), 1.0);
        assert_eq!(surge_seconds_remaining(&buff), 0.0);
    }

    #[test]
    fn active_surge_extends_from_kills_with_duration_cap() {
        let mut buff = SurgeBuff {
            timer: Timer::from_seconds(0.0, TimerMode::Once),
            charge: 9,
            max_charge: 9,
            duration_secs: 9.0,
            damage_multiplier: 1.24,
            speed_multiplier: 1.16,
            fury_multiplier: 1.35,
        };

        assert_eq!(extend_active_surge(&mut buff, 3), None);
        assert!(activate_surge(&mut buff));
        buff.timer.tick(std::time::Duration::from_secs_f32(3.0));

        let gained = extend_active_surge(&mut buff, 2).unwrap();

        assert!((gained - 2.5).abs() < 0.001);
        assert!((surge_seconds_remaining(&buff) - 8.5).abs() < 0.001);

        let gained = extend_active_surge(&mut buff, 99).unwrap();

        assert!((gained - 6.5).abs() < 0.001);
        assert!((surge_seconds_remaining(&buff) - 15.0).abs() < 0.001);
    }
}
