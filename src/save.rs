use crate::{
    Difficulty, DifficultySettings, GameState, RunStats,
    bestiary::{Bestiary, BestiaryEntry},
    bounty::BountyProgress,
    challenge::{ChallengeProgress, ChapterChallenges},
    chapter::{ChapterPhase, ChapterProgress, InteractableKind},
    companion::{CompanionBond, CompanionStance},
    dungeon::{Breakable, Interactable},
    feedback::CombatEvent,
    lore::{LoreEntryId, LoreJournal},
    mastery::ChapterMastery,
    milestone::{MilestoneId, Milestones},
    obelisk::BloodObelisk,
    ordeal::{ChapterModifier, ChapterModifierKind, OrdealAffix},
    player::{
        ARMORY_SLOT_COUNT, Armory, ArmoryLoadout, Barrier, ChapterBoon, ChapterBoonChoice, Charm,
        ConduitBuff, DamageBonus, DashRune, DeathWard, EMBER_PARAGON_BASE_XP, ElixirBelt,
        ElixirBuff, ElixirKind, EmberParagon, Equipment, Evade, FortuneBuff, Fury, GloryBuff,
        Health, Inventory, InventoryItem, LegendaryCodex, LegendaryPower, NovaRune, Player,
        PlayerLevel, PotionBelt, RelicBuff, RuptureRune, SkillRunes, SocketedGem, SurgeBuff,
        Talents, TownPortal, configure_elixir_buff, potion_cooldown_secs_for_capacity,
    },
    rift::EmberRift,
    story::{StoryBeat, StoryLog},
};
use bevy::ecs::system::SystemParam;
use bevy::prelude::*;
use serde::{Deserialize, Serialize};
#[cfg(not(target_arch = "wasm32"))]
use std::fs;
use std::path::Path;

const SAVE_PATH: &str = "saves/slot1.ron";

#[derive(Resource, Debug, Default)]
pub struct PendingLoadGame {
    requested: bool,
}

impl PendingLoadGame {
    pub fn request(&mut self) {
        self.requested = true;
    }

    fn take(&mut self) -> bool {
        let requested = self.requested;
        self.requested = false;
        requested
    }
}

fn default_fury_max() -> f32 {
    100.0
}

fn default_fury_basic_gain() -> f32 {
    18.0
}

fn default_dash_cost() -> f32 {
    25.0
}

fn default_nova_cost() -> f32 {
    45.0
}

fn default_rupture_cost() -> f32 {
    32.0
}

fn default_barrier_max() -> f32 {
    35.0
}

fn default_paragon_next_rank_xp() -> u32 {
    EMBER_PARAGON_BASE_XP
}

fn default_death_checkpoint() -> [f32; 3] {
    [0.0, 0.0, 4.0]
}

fn default_max_revives() -> u32 {
    2
}

fn default_revive_gold_penalty() -> u32 {
    60
}

fn default_companion_rank() -> u32 {
    CompanionBond::default().rank
}

fn default_companion_next_rank_xp() -> u32 {
    CompanionBond::default().next_rank_xp
}

fn default_companion_stance() -> CompanionStance {
    CompanionBond::default().stance
}

fn default_companion_command_cooldown() -> f32 {
    CompanionBond::default().command_cooldown
}

fn default_surge_max_charge() -> u32 {
    9
}

fn default_surge_duration_secs() -> f32 {
    9.0
}

fn default_relic_damage_multiplier() -> f32 {
    1.0
}

fn default_relic_speed_multiplier() -> f32 {
    1.0
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
struct SaveGame {
    version: u32,
    stats: SaveStats,
    #[serde(default)]
    difficulty: Difficulty,
    #[serde(default)]
    modifier: ChapterModifierKind,
    #[serde(default)]
    modifier_affix: OrdealAffix,
    #[serde(default)]
    chapter: SaveChapter,
    #[serde(default)]
    bounty: BountyProgress,
    #[serde(default)]
    challenges: Vec<ChallengeProgress>,
    #[serde(default)]
    mastery: ChapterMastery,
    #[serde(default)]
    interactables: Vec<SaveInteractable>,
    #[serde(default)]
    breakables: Option<Vec<SaveBreakable>>,
    #[serde(default)]
    journal: Vec<LoreEntryId>,
    #[serde(default)]
    milestones: Vec<MilestoneId>,
    #[serde(default)]
    bestiary: std::collections::BTreeMap<String, BestiaryEntry>,
    #[serde(default)]
    obelisk: BloodObelisk,
    #[serde(default)]
    rift: EmberRift,
    #[serde(default)]
    story: Vec<StoryBeat>,
    player: SavePlayer,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
struct SaveStats {
    kills: u32,
    gold: u32,
    #[serde(default)]
    ember_shards: u32,
    #[serde(default)]
    affix_essence: u32,
    #[serde(default)]
    valor_stacks: u32,
    #[serde(default)]
    best_valor_stacks: u32,
    #[serde(default)]
    valor_timer_secs: f32,
    #[serde(default)]
    elapsed_secs: f32,
    #[serde(default)]
    completion_bonus_gold: u32,
    #[serde(default)]
    completion_bonus_shards: u32,
    #[serde(default)]
    completion_bonus_essence: u32,
    #[serde(default)]
    chapter_clear_bonus_gold: u32,
    #[serde(default)]
    chapter_clear_bonus_shards: u32,
    #[serde(default)]
    chapter_clear_bonus_essence: u32,
    #[serde(default)]
    chapter_clear_reward_claimed: bool,
    #[serde(default)]
    journey_score: u32,
    #[serde(default)]
    journey_bonus_gold: u32,
    #[serde(default)]
    journey_bonus_shards: u32,
    #[serde(default)]
    journey_bonus_essence: u32,
    #[serde(default)]
    journey_reward_claimed: bool,
    #[serde(default)]
    renown_rank: u32,
    #[serde(default)]
    renown_bonus_gold: u32,
    #[serde(default)]
    renown_bonus_shards: u32,
    #[serde(default)]
    renown_bonus_essence: u32,
    #[serde(default)]
    renown_bonus_claimed: bool,
    #[serde(default)]
    stash_bonus_gold: u32,
    #[serde(default)]
    stash_bonus_shards: u32,
    #[serde(default)]
    stash_bonus_essence: u32,
    #[serde(default)]
    stash_bonus_claimed: bool,
    #[serde(default)]
    altar_seals: u32,
    #[serde(default)]
    altar_bonus_gold: u32,
    #[serde(default)]
    altar_bonus_shards: u32,
    #[serde(default)]
    altar_bonus_essence: u32,
    #[serde(default)]
    altar_bonus_claimed: bool,
    #[serde(default)]
    primal_caches: u32,
    #[serde(default)]
    primal_cache_bonus_gold: u32,
    #[serde(default)]
    primal_cache_bonus_shards: u32,
    #[serde(default)]
    primal_cache_bonus_essence: u32,
    #[serde(default)]
    primal_cache_items_claimed: u32,
    #[serde(default)]
    echo_keystones: u32,
    #[serde(default)]
    primal_cache_echo_items: u32,
    #[serde(default)]
    malrec_soul_sigils: u32,
    #[serde(default)]
    malrec_soul_sigils_earned: u32,
    #[serde(default)]
    soul_sigil_caches: u32,
    #[serde(default)]
    completion_reward_claimed: bool,
    #[serde(default)]
    massacre_streak: u32,
    #[serde(default)]
    best_massacre_streak: u32,
    #[serde(default)]
    massacre_timer_secs: f32,
    #[serde(default)]
    massacre_bonus_gold: u32,
    #[serde(default)]
    ancient_augments: u32,
    #[serde(default)]
    primal_infusions: u32,
    #[serde(default)]
    potions_used: u32,
    #[serde(default)]
    last_stand_potions: u32,
    #[serde(default)]
    boss_enraged: bool,
    #[serde(default)]
    boss_staggers: u32,
    #[serde(default)]
    shrine_resonance_triggered: bool,
    #[serde(default)]
    elite_affix_kills: u32,
    #[serde(default)]
    affix_codex_mask: u16,
    #[serde(default)]
    seal_warden_slain: bool,
    #[serde(default)]
    cursed_ambush_kills: u32,
    #[serde(default)]
    champion_pack_kills: u32,
    #[serde(default)]
    champion_pack_reward_claimed: bool,
    #[serde(default)]
    nemesis_kills: u32,
    #[serde(default)]
    treasure_vaults_opened: u32,
    #[serde(default)]
    breakables_smashed: u32,
    #[serde(default)]
    health_globes_collected: u32,
    #[serde(default)]
    surge_kills: u32,
    #[serde(default)]
    reap_dash_hits: u32,
    #[serde(default)]
    hemorrhage_rupture_hits: u32,
    #[serde(default)]
    frost_nova_hits: u32,
    #[serde(default)]
    armory_loadouts_saved: u32,
    #[serde(default)]
    town_portal_returns: u32,
    #[serde(default)]
    pylon_kills: u32,
    #[serde(default)]
    salvage_progress: u32,
    #[serde(default)]
    salvage_caches: u32,
    #[serde(default)]
    alchemy_gold_transmutes: u32,
    #[serde(default)]
    alchemy_essence_transmutes: u32,
    #[serde(default)]
    alchemy_keystone_transmutes: u32,
    #[serde(default)]
    loot_filter_cycles: u32,
    #[serde(default)]
    codex_attuned_kills: u32,
    #[serde(default)]
    set_resonance_kills: u32,
    #[serde(default)]
    ruby_socketed: bool,
    #[serde(default)]
    emerald_socketed: bool,
    #[serde(default)]
    amethyst_socketed: bool,
    #[serde(default)]
    topaz_socketed: bool,
    #[serde(default)]
    iron_elixir_used: bool,
    #[serde(default)]
    wrath_elixir_used: bool,
    #[serde(default)]
    haste_elixir_used: bool,
    #[serde(default)]
    reliquary_momentum: u32,
    #[serde(default)]
    best_reliquary_momentum: u32,
    #[serde(default)]
    ashen_threat: u32,
    #[serde(default)]
    ashen_threat_surges: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
struct SaveChapter {
    phase: ChapterPhase,
    objective: String,
    enemies_slain: u32,
    chest_opened: bool,
    altar_activated: bool,
    sanctum_wave_spawned: bool,
    #[serde(default)]
    seal_fragments: u32,
    boss_spawned: bool,
    boss_slain: bool,
    last_kill: String,
    interaction_hint: String,
}

impl Default for SaveChapter {
    fn default() -> Self {
        Self::from(&ChapterProgress::default())
    }
}

impl From<&ChapterProgress> for SaveChapter {
    fn from(progress: &ChapterProgress) -> Self {
        Self {
            phase: progress.phase,
            objective: progress.objective.clone(),
            enemies_slain: progress.enemies_slain,
            chest_opened: progress.chest_opened,
            altar_activated: progress.altar_activated,
            sanctum_wave_spawned: progress.sanctum_wave_spawned,
            seal_fragments: progress.seal_fragments,
            boss_spawned: progress.boss_spawned,
            boss_slain: progress.boss_slain,
            last_kill: progress.last_kill.clone(),
            interaction_hint: progress.interaction_hint.clone(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
struct SaveInteractable {
    kind: InteractableKind,
    position: [f32; 3],
    used: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
struct SaveBreakable {
    position: [f32; 3],
    health_current: f32,
    health_max: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
struct SavePlayer {
    position: [f32; 3],
    health_current: f32,
    health_max: f32,
    #[serde(default)]
    barrier_current: f32,
    #[serde(default = "default_barrier_max")]
    barrier_max: f32,
    level: u32,
    xp: u32,
    next_level_xp: u32,
    #[serde(default)]
    paragon_rank: u32,
    #[serde(default)]
    paragon_xp: u32,
    #[serde(default = "default_paragon_next_rank_xp")]
    paragon_next_rank_xp: u32,
    damage_bonus: f32,
    #[serde(default)]
    fury_current: f32,
    #[serde(default = "default_fury_max")]
    fury_max: f32,
    #[serde(default = "default_fury_basic_gain")]
    fury_basic_gain: f32,
    #[serde(default = "default_dash_cost")]
    dash_cost: f32,
    #[serde(default = "default_nova_cost")]
    nova_cost: f32,
    #[serde(default = "default_rupture_cost")]
    rupture_cost: f32,
    equipment: SaveEquipment,
    #[serde(default)]
    charm: Charm,
    inventory: Vec<SaveInventoryItem>,
    inventory_capacity: usize,
    #[serde(default)]
    armory_loadout: Option<ArmoryLoadout>,
    #[serde(default)]
    armory_loadouts: Vec<Option<ArmoryLoadout>>,
    #[serde(default)]
    armory_active_slot: usize,
    #[serde(default)]
    legendary_codex: Vec<LegendaryPower>,
    #[serde(default)]
    legendary_codex_attuned: LegendaryPower,
    #[serde(default = "default_death_checkpoint")]
    death_checkpoint: [f32; 3],
    #[serde(default)]
    revives_used: u32,
    #[serde(default = "default_max_revives")]
    max_revives: u32,
    #[serde(default = "default_revive_gold_penalty")]
    revive_gold_penalty: u32,
    potions: u32,
    max_potions: u32,
    #[serde(default)]
    potion_cooldown_secs: f32,
    #[serde(default)]
    potion_cooldown_remaining_secs: f32,
    #[serde(default)]
    elixirs: u32,
    #[serde(default)]
    max_elixirs: u32,
    #[serde(default)]
    selected_elixir: ElixirKind,
    #[serde(default)]
    active_elixir: ElixirKind,
    #[serde(default)]
    elixir_buff_secs: f32,
    #[serde(default)]
    relic_buff_secs: f32,
    #[serde(default = "default_relic_damage_multiplier")]
    relic_damage_multiplier: f32,
    #[serde(default = "default_relic_speed_multiplier")]
    relic_speed_multiplier: f32,
    #[serde(default)]
    fortune_buff_secs: f32,
    #[serde(default)]
    conduit_buff_secs: f32,
    #[serde(default)]
    glory_buff_secs: f32,
    #[serde(default)]
    glory_stacks: u32,
    #[serde(default)]
    surge_buff_secs: f32,
    #[serde(default)]
    surge_charge: u32,
    #[serde(default = "default_surge_max_charge")]
    surge_max_charge: u32,
    #[serde(default = "default_surge_duration_secs")]
    surge_duration_secs: f32,
    #[serde(default)]
    town_portal_cooldown_secs: f32,
    #[serde(default)]
    town_portal_return_position: Option<[f32; 3]>,
    #[serde(default)]
    evade_active_secs: f32,
    #[serde(default)]
    evade_cooldown_secs: f32,
    #[serde(default)]
    evade_cooldown_remaining_secs: f32,
    #[serde(default = "default_companion_rank")]
    companion_rank: u32,
    #[serde(default)]
    companion_xp: u32,
    #[serde(default = "default_companion_next_rank_xp")]
    companion_next_rank_xp: u32,
    #[serde(default = "default_companion_stance")]
    companion_stance: CompanionStance,
    #[serde(default = "default_companion_command_cooldown")]
    companion_command_cooldown_secs: f32,
    #[serde(default)]
    chapter_boon: ChapterBoonChoice,
    #[serde(default)]
    chapter_boon_empowered: bool,
    #[serde(default)]
    chapter_boon_momentum_stacks: u32,
    #[serde(default)]
    talent_points: u32,
    #[serde(default)]
    talent_wrath: u32,
    #[serde(default)]
    talent_vigor: u32,
    #[serde(default)]
    talent_focus: u32,
    #[serde(default)]
    dash_rune: DashRune,
    #[serde(default)]
    nova_rune: NovaRune,
    #[serde(default)]
    rupture_rune: RuptureRune,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
struct SaveEquipment {
    weapon_name: String,
    quality: String,
    crit_chance: f32,
    health_bonus: f32,
    #[serde(default)]
    armor_bonus: f32,
    #[serde(default)]
    legendary_power: LegendaryPower,
    #[serde(default)]
    temper_level: u32,
    #[serde(default)]
    socketed_gem: Option<SocketedGem>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
struct SaveInventoryItem {
    name: String,
    quality: String,
    damage_bonus: f32,
    crit_chance: f32,
    health_bonus: f32,
    #[serde(default)]
    armor_bonus: f32,
    #[serde(default)]
    legendary_power: LegendaryPower,
    #[serde(default)]
    temper_level: u32,
    #[serde(default)]
    socketed_gem: Option<SocketedGem>,
}

pub struct SavePlugin;

type SavePlayerCoreQuery<'w, 's> = Query<
    'w,
    's,
    (
        &'static Transform,
        &'static Health,
        &'static PlayerLevel,
        &'static DamageBonus,
        &'static Fury,
        &'static Equipment,
        &'static Inventory,
        &'static ChapterBoon,
        &'static Talents,
    ),
    With<Player>,
>;

type SavePlayerStateQuery<'w, 's> = Query<
    'w,
    's,
    (
        &'static PotionBelt,
        &'static ElixirBelt,
        &'static ElixirBuff,
        &'static RelicBuff,
        &'static FortuneBuff,
        &'static TownPortal,
        &'static Evade,
    ),
    With<Player>,
>;

type LoadPlayerCoreQuery<'w, 's> = Query<
    'w,
    's,
    (
        &'static mut Transform,
        &'static mut Health,
        &'static mut PlayerLevel,
        &'static mut DamageBonus,
        &'static mut Fury,
        &'static mut Equipment,
        &'static mut Inventory,
        &'static mut ChapterBoon,
        &'static mut Talents,
    ),
    With<Player>,
>;

type LoadPlayerStateQuery<'w, 's> = Query<
    'w,
    's,
    (
        &'static mut PotionBelt,
        &'static mut ElixirBelt,
        &'static mut ElixirBuff,
        &'static mut RelicBuff,
        &'static mut FortuneBuff,
        &'static mut TownPortal,
        &'static mut Evade,
    ),
    With<Player>,
>;

type SaveRunesQuery<'w, 's> = Query<'w, 's, &'static SkillRunes, With<Player>>;
type LoadRunesQuery<'w, 's> = Query<'w, 's, &'static mut SkillRunes, With<Player>>;
type SaveCompanionQuery<'w, 's> = Query<'w, 's, &'static CompanionBond, With<Player>>;
type LoadCompanionQuery<'w, 's> = Query<'w, 's, &'static mut CompanionBond, With<Player>>;
type SaveArmoryQuery<'w, 's> = Query<'w, 's, &'static Armory, With<Player>>;
type LoadArmoryQuery<'w, 's> = Query<'w, 's, &'static mut Armory, With<Player>>;
type SaveCodexQuery<'w, 's> = Query<'w, 's, &'static LegendaryCodex, With<Player>>;
type LoadCodexQuery<'w, 's> = Query<'w, 's, &'static mut LegendaryCodex, With<Player>>;
type SaveParagonQuery<'w, 's> = Query<'w, 's, &'static EmberParagon, With<Player>>;
type LoadParagonQuery<'w, 's> = Query<'w, 's, &'static mut EmberParagon, With<Player>>;
type SaveCharmQuery<'w, 's> = Query<'w, 's, &'static Charm, With<Player>>;
type LoadCharmQuery<'w, 's> = Query<'w, 's, &'static mut Charm, With<Player>>;
type SaveDeathWardQuery<'w, 's> = Query<'w, 's, &'static DeathWard, With<Player>>;
type LoadDeathWardQuery<'w, 's> = Query<'w, 's, &'static mut DeathWard, With<Player>>;
type SaveBarrierQuery<'w, 's> = Query<'w, 's, &'static Barrier, With<Player>>;
type LoadBarrierQuery<'w, 's> = Query<'w, 's, &'static mut Barrier, With<Player>>;
type SaveConduitQuery<'w, 's> = Query<'w, 's, &'static ConduitBuff, With<Player>>;
type LoadConduitQuery<'w, 's> = Query<'w, 's, &'static mut ConduitBuff, With<Player>>;
type SaveGloryQuery<'w, 's> = Query<'w, 's, &'static GloryBuff, With<Player>>;
type LoadGloryQuery<'w, 's> = Query<'w, 's, &'static mut GloryBuff, With<Player>>;
type SaveSurgeQuery<'w, 's> = Query<'w, 's, &'static SurgeBuff, With<Player>>;
type LoadSurgeQuery<'w, 's> = Query<'w, 's, &'static mut SurgeBuff, With<Player>>;
type SaveInteractableQuery<'w, 's> =
    Query<'w, 's, (&'static Transform, &'static Interactable), Without<Player>>;
type LoadInteractableQuery<'w, 's> =
    Query<'w, 's, (&'static Transform, &'static mut Interactable), Without<Player>>;
type SaveBreakableQuery<'w, 's> =
    Query<'w, 's, (&'static Transform, &'static Health), (With<Breakable>, Without<Player>)>;
type LoadBreakableQuery<'w, 's> = Query<
    'w,
    's,
    (Entity, &'static Transform, &'static mut Health),
    (With<Breakable>, Without<Player>),
>;
type SaveResources<'w> = (
    Res<'w, RunStats>,
    Res<'w, DifficultySettings>,
    Res<'w, ChapterModifier>,
    Res<'w, ChapterProgress>,
    Res<'w, BountyProgress>,
    Res<'w, ChapterChallenges>,
    Res<'w, ChapterMastery>,
    Res<'w, LoreJournal>,
    Res<'w, Milestones>,
    Res<'w, Bestiary>,
    Res<'w, BloodObelisk>,
    Res<'w, EmberRift>,
    Res<'w, StoryLog>,
);
type LoadResources<'w> = (
    ResMut<'w, RunStats>,
    ResMut<'w, DifficultySettings>,
    ResMut<'w, ChapterModifier>,
    ResMut<'w, ChapterProgress>,
    ResMut<'w, BountyProgress>,
    ResMut<'w, ChapterChallenges>,
    ResMut<'w, ChapterMastery>,
    ResMut<'w, LoreJournal>,
    ResMut<'w, Milestones>,
    ResMut<'w, Bestiary>,
    ResMut<'w, BloodObelisk>,
    ResMut<'w, EmberRift>,
    ResMut<'w, StoryLog>,
);

#[derive(SystemParam)]
struct LoadGameInputs<'w, 's> {
    commands: Commands<'w, 's>,
    resources: LoadResources<'w>,
    player_core: LoadPlayerCoreQuery<'w, 's>,
    player_state: LoadPlayerStateQuery<'w, 's>,
    runes: LoadRunesQuery<'w, 's>,
    companion: LoadCompanionQuery<'w, 's>,
    armory: LoadArmoryQuery<'w, 's>,
    paragon: LoadParagonQuery<'w, 's>,
    codex: LoadCodexQuery<'w, 's>,
    charm: LoadCharmQuery<'w, 's>,
    death_ward: LoadDeathWardQuery<'w, 's>,
    barrier: LoadBarrierQuery<'w, 's>,
    conduit: LoadConduitQuery<'w, 's>,
    glory: LoadGloryQuery<'w, 's>,
    surge: LoadSurgeQuery<'w, 's>,
    interactables: LoadInteractableQuery<'w, 's>,
    breakables: LoadBreakableQuery<'w, 's>,
    events: MessageWriter<'w, CombatEvent>,
}

#[derive(SystemParam)]
struct SaveGameInputs<'w, 's> {
    resources: SaveResources<'w>,
    player_core: SavePlayerCoreQuery<'w, 's>,
    player_state: SavePlayerStateQuery<'w, 's>,
    runes: SaveRunesQuery<'w, 's>,
    companion: SaveCompanionQuery<'w, 's>,
    armory: SaveArmoryQuery<'w, 's>,
    paragon: SaveParagonQuery<'w, 's>,
    codex: SaveCodexQuery<'w, 's>,
    charm: SaveCharmQuery<'w, 's>,
    death_ward: SaveDeathWardQuery<'w, 's>,
    barrier: SaveBarrierQuery<'w, 's>,
    conduit: SaveConduitQuery<'w, 's>,
    glory: SaveGloryQuery<'w, 's>,
    surge: SaveSurgeQuery<'w, 's>,
    interactables: SaveInteractableQuery<'w, 's>,
    breakables: SaveBreakableQuery<'w, 's>,
    events: MessageWriter<'w, CombatEvent>,
}

impl Plugin for SavePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<PendingLoadGame>()
            .add_systems(Update, save_game.run_if(in_state(GameState::InGame)))
            .add_systems(Update, load_game.run_if(in_state(GameState::InGame)));
    }
}

fn save_game(keyboard: Res<ButtonInput<KeyCode>>, mut inputs: SaveGameInputs) {
    if !keyboard.just_pressed(KeyCode::F5) {
        return;
    }
    let (
        stats,
        difficulty,
        modifier,
        chapter,
        bounty,
        challenges,
        mastery,
        journal,
        milestones,
        bestiary,
        obelisk,
        rift,
        story,
    ) = inputs.resources;
    let Ok(snapshot) = snapshot_from_world(SnapshotInputs {
        stats: &stats,
        difficulty: &difficulty,
        modifier: &modifier,
        chapter: &chapter,
        bounty: &bounty,
        challenges: &challenges,
        mastery: &mastery,
        journal: &journal,
        milestones: &milestones,
        bestiary: &bestiary,
        obelisk: &obelisk,
        rift: &rift,
        story: &story,
        player_core: &inputs.player_core,
        player_state: &inputs.player_state,
        runes: &inputs.runes,
        companion: &inputs.companion,
        armory: &inputs.armory,
        paragon: &inputs.paragon,
        codex: &inputs.codex,
        charm: &inputs.charm,
        death_ward: &inputs.death_ward,
        barrier: &inputs.barrier,
        conduit: &inputs.conduit,
        glory: &inputs.glory,
        surge: &inputs.surge,
        interactables: &inputs.interactables,
        breakables: &inputs.breakables,
    }) else {
        inputs.events.write(CombatEvent {
            text: "Save failed: player not ready".to_string(),
        });
        return;
    };

    match write_save(SAVE_PATH, &snapshot) {
        Ok(()) => inputs.events.write(CombatEvent {
            text: "Saved slot 1".to_string(),
        }),
        Err(err) => inputs.events.write(CombatEvent {
            text: format!("Save failed: {err}"),
        }),
    };
}

fn load_game(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut pending: ResMut<PendingLoadGame>,
    mut inputs: LoadGameInputs,
) {
    if !keyboard.just_pressed(KeyCode::F9) && !pending.take() {
        return;
    }
    let (
        mut stats,
        mut difficulty,
        mut modifier,
        mut chapter,
        mut bounty,
        mut challenges,
        mut mastery,
        mut journal,
        mut milestones,
        mut bestiary,
        mut obelisk,
        mut rift,
        mut story,
    ) = inputs.resources;
    match read_save(SAVE_PATH).and_then(|save| {
        apply_save(
            &save,
            ApplyResources {
                stats: &mut stats,
                difficulty: &mut difficulty,
                modifier: &mut modifier,
                chapter: &mut chapter,
                bounty: &mut bounty,
                challenges: &mut challenges,
                mastery: &mut mastery,
                journal: &mut journal,
                milestones: &mut milestones,
                bestiary: &mut bestiary,
                obelisk: &mut obelisk,
                rift: &mut rift,
                story: &mut story,
            },
            ApplyQueries {
                player_core: &mut inputs.player_core,
                player_state: &mut inputs.player_state,
                runes: &mut inputs.runes,
                companion: &mut inputs.companion,
                armory: &mut inputs.armory,
                paragon: &mut inputs.paragon,
                codex: &mut inputs.codex,
                charm: &mut inputs.charm,
                death_ward: &mut inputs.death_ward,
                barrier: &mut inputs.barrier,
                conduit: &mut inputs.conduit,
                glory: &mut inputs.glory,
                surge: &mut inputs.surge,
                interactables: &mut inputs.interactables,
                breakables: &mut inputs.breakables,
                commands: &mut inputs.commands,
            },
        )
    }) {
        Ok(()) => inputs.events.write(CombatEvent {
            text: "Loaded slot 1".to_string(),
        }),
        Err(err) => inputs.events.write(CombatEvent {
            text: format!("Load failed: {err}"),
        }),
    };
}

struct SnapshotInputs<'a, 'w, 's> {
    stats: &'a RunStats,
    difficulty: &'a DifficultySettings,
    modifier: &'a ChapterModifier,
    chapter: &'a ChapterProgress,
    bounty: &'a BountyProgress,
    challenges: &'a ChapterChallenges,
    mastery: &'a ChapterMastery,
    journal: &'a LoreJournal,
    milestones: &'a Milestones,
    bestiary: &'a Bestiary,
    obelisk: &'a BloodObelisk,
    rift: &'a EmberRift,
    story: &'a StoryLog,
    player_core: &'a SavePlayerCoreQuery<'w, 's>,
    player_state: &'a SavePlayerStateQuery<'w, 's>,
    runes: &'a SaveRunesQuery<'w, 's>,
    companion: &'a SaveCompanionQuery<'w, 's>,
    armory: &'a SaveArmoryQuery<'w, 's>,
    paragon: &'a SaveParagonQuery<'w, 's>,
    codex: &'a SaveCodexQuery<'w, 's>,
    charm: &'a SaveCharmQuery<'w, 's>,
    death_ward: &'a SaveDeathWardQuery<'w, 's>,
    barrier: &'a SaveBarrierQuery<'w, 's>,
    conduit: &'a SaveConduitQuery<'w, 's>,
    glory: &'a SaveGloryQuery<'w, 's>,
    surge: &'a SaveSurgeQuery<'w, 's>,
    interactables: &'a SaveInteractableQuery<'w, 's>,
    breakables: &'a SaveBreakableQuery<'w, 's>,
}

fn snapshot_from_world(inputs: SnapshotInputs) -> Result<SaveGame, String> {
    let (transform, health, level, damage, fury, equipment, inventory, chapter_boon, talents) =
        inputs
            .player_core
            .single()
            .map_err(|_| "expected exactly one player".to_string())?;
    let (potions, elixirs, elixir_buff, relic_buff, fortune_buff, town_portal, evade) = inputs
        .player_state
        .single()
        .map_err(|_| "expected exactly one player state".to_string())?;
    let runes = inputs
        .runes
        .single()
        .map_err(|_| "expected exactly one player runes".to_string())?;
    let companion = inputs
        .companion
        .single()
        .map_err(|_| "expected exactly one player companion".to_string())?;
    let armory = inputs
        .armory
        .single()
        .map_err(|_| "expected exactly one player armory".to_string())?;
    let paragon = inputs
        .paragon
        .single()
        .map_err(|_| "expected exactly one player paragon".to_string())?;
    let codex = inputs
        .codex
        .single()
        .map_err(|_| "expected exactly one player codex".to_string())?;
    let charm = inputs
        .charm
        .single()
        .map_err(|_| "expected exactly one player charm".to_string())?;
    let death_ward = inputs
        .death_ward
        .single()
        .map_err(|_| "expected exactly one player death ward".to_string())?;
    let barrier = inputs
        .barrier
        .single()
        .map_err(|_| "expected exactly one player barrier".to_string())?;
    let conduit_buff = inputs
        .conduit
        .single()
        .map_err(|_| "expected exactly one player conduit".to_string())?;
    let glory_buff = inputs
        .glory
        .single()
        .map_err(|_| "expected exactly one player glory".to_string())?;
    let surge_buff = inputs
        .surge
        .single()
        .map_err(|_| "expected exactly one player surge".to_string())?;
    Ok(SaveGame {
        version: 1,
        stats: SaveStats {
            kills: inputs.stats.kills,
            gold: inputs.stats.gold,
            ember_shards: inputs.stats.ember_shards,
            affix_essence: inputs.stats.affix_essence,
            valor_stacks: inputs.stats.valor_stacks,
            best_valor_stacks: inputs.stats.best_valor_stacks,
            valor_timer_secs: inputs.stats.valor_timer_secs,
            elapsed_secs: inputs.stats.elapsed_secs,
            completion_bonus_gold: inputs.stats.completion_bonus_gold,
            completion_bonus_shards: inputs.stats.completion_bonus_shards,
            completion_bonus_essence: inputs.stats.completion_bonus_essence,
            chapter_clear_bonus_gold: inputs.stats.chapter_clear_bonus_gold,
            chapter_clear_bonus_shards: inputs.stats.chapter_clear_bonus_shards,
            chapter_clear_bonus_essence: inputs.stats.chapter_clear_bonus_essence,
            chapter_clear_reward_claimed: inputs.stats.chapter_clear_reward_claimed,
            journey_score: inputs.stats.journey_score,
            journey_bonus_gold: inputs.stats.journey_bonus_gold,
            journey_bonus_shards: inputs.stats.journey_bonus_shards,
            journey_bonus_essence: inputs.stats.journey_bonus_essence,
            journey_reward_claimed: inputs.stats.journey_reward_claimed,
            renown_rank: inputs.stats.renown_rank,
            renown_bonus_gold: inputs.stats.renown_bonus_gold,
            renown_bonus_shards: inputs.stats.renown_bonus_shards,
            renown_bonus_essence: inputs.stats.renown_bonus_essence,
            renown_bonus_claimed: inputs.stats.renown_bonus_claimed,
            stash_bonus_gold: inputs.stats.stash_bonus_gold,
            stash_bonus_shards: inputs.stats.stash_bonus_shards,
            stash_bonus_essence: inputs.stats.stash_bonus_essence,
            stash_bonus_claimed: inputs.stats.stash_bonus_claimed,
            altar_seals: inputs.stats.altar_seals,
            altar_bonus_gold: inputs.stats.altar_bonus_gold,
            altar_bonus_shards: inputs.stats.altar_bonus_shards,
            altar_bonus_essence: inputs.stats.altar_bonus_essence,
            altar_bonus_claimed: inputs.stats.altar_bonus_claimed,
            primal_caches: inputs.stats.primal_caches,
            primal_cache_bonus_gold: inputs.stats.primal_cache_bonus_gold,
            primal_cache_bonus_shards: inputs.stats.primal_cache_bonus_shards,
            primal_cache_bonus_essence: inputs.stats.primal_cache_bonus_essence,
            primal_cache_items_claimed: inputs.stats.primal_cache_items_claimed,
            echo_keystones: inputs.stats.echo_keystones,
            primal_cache_echo_items: inputs.stats.primal_cache_echo_items,
            malrec_soul_sigils: inputs.stats.malrec_soul_sigils,
            malrec_soul_sigils_earned: inputs.stats.malrec_soul_sigils_earned,
            soul_sigil_caches: inputs.stats.soul_sigil_caches,
            completion_reward_claimed: inputs.stats.completion_reward_claimed,
            massacre_streak: inputs.stats.massacre_streak,
            best_massacre_streak: inputs.stats.best_massacre_streak,
            massacre_timer_secs: inputs.stats.massacre_timer_secs,
            massacre_bonus_gold: inputs.stats.massacre_bonus_gold,
            ancient_augments: inputs.stats.ancient_augments,
            primal_infusions: inputs.stats.primal_infusions,
            potions_used: inputs.stats.potions_used,
            last_stand_potions: inputs.stats.last_stand_potions,
            boss_enraged: inputs.stats.boss_enraged,
            boss_staggers: inputs.stats.boss_staggers,
            shrine_resonance_triggered: inputs.stats.shrine_resonance_triggered,
            elite_affix_kills: inputs.stats.elite_affix_kills,
            affix_codex_mask: inputs.stats.affix_codex_mask,
            seal_warden_slain: inputs.stats.seal_warden_slain,
            cursed_ambush_kills: inputs.stats.cursed_ambush_kills,
            champion_pack_kills: inputs.stats.champion_pack_kills,
            champion_pack_reward_claimed: inputs.stats.champion_pack_reward_claimed,
            nemesis_kills: inputs.stats.nemesis_kills,
            treasure_vaults_opened: inputs.stats.treasure_vaults_opened,
            breakables_smashed: inputs.stats.breakables_smashed,
            health_globes_collected: inputs.stats.health_globes_collected,
            surge_kills: inputs.stats.surge_kills,
            reap_dash_hits: inputs.stats.reap_dash_hits,
            hemorrhage_rupture_hits: inputs.stats.hemorrhage_rupture_hits,
            frost_nova_hits: inputs.stats.frost_nova_hits,
            armory_loadouts_saved: inputs.stats.armory_loadouts_saved,
            town_portal_returns: inputs.stats.town_portal_returns,
            pylon_kills: inputs.stats.pylon_kills,
            salvage_progress: inputs.stats.salvage_progress,
            salvage_caches: inputs.stats.salvage_caches,
            alchemy_gold_transmutes: inputs.stats.alchemy_gold_transmutes,
            alchemy_essence_transmutes: inputs.stats.alchemy_essence_transmutes,
            alchemy_keystone_transmutes: inputs.stats.alchemy_keystone_transmutes,
            loot_filter_cycles: inputs.stats.loot_filter_cycles,
            codex_attuned_kills: inputs.stats.codex_attuned_kills,
            set_resonance_kills: inputs.stats.set_resonance_kills,
            ruby_socketed: inputs.stats.ruby_socketed,
            emerald_socketed: inputs.stats.emerald_socketed,
            amethyst_socketed: inputs.stats.amethyst_socketed,
            topaz_socketed: inputs.stats.topaz_socketed,
            iron_elixir_used: inputs.stats.iron_elixir_used,
            wrath_elixir_used: inputs.stats.wrath_elixir_used,
            haste_elixir_used: inputs.stats.haste_elixir_used,
            reliquary_momentum: inputs.stats.reliquary_momentum,
            best_reliquary_momentum: inputs.stats.best_reliquary_momentum,
            ashen_threat: inputs.stats.ashen_threat,
            ashen_threat_surges: inputs.stats.ashen_threat_surges,
        },
        difficulty: inputs.difficulty.current,
        modifier: inputs.modifier.kind,
        modifier_affix: inputs.modifier.affix,
        chapter: SaveChapter::from(inputs.chapter),
        bounty: inputs.bounty.clone(),
        challenges: inputs.challenges.challenges.clone(),
        mastery: inputs.mastery.clone(),
        interactables: inputs
            .interactables
            .iter()
            .filter(|(_, interactable)| !interactable.reusable)
            .map(|(transform, interactable)| SaveInteractable {
                kind: interactable.kind,
                position: transform.translation.to_array(),
                used: interactable.used,
            })
            .collect(),
        breakables: Some(
            inputs
                .breakables
                .iter()
                .filter(|(_, health)| health.current > 0.0)
                .map(|(transform, health)| SaveBreakable {
                    position: transform.translation.to_array(),
                    health_current: health.current,
                    health_max: health.max,
                })
                .collect(),
        ),
        journal: inputs.journal.entries.clone(),
        milestones: inputs.milestones.unlocked.clone(),
        bestiary: inputs.bestiary.entries.clone(),
        obelisk: inputs.obelisk.clone(),
        rift: inputs.rift.clone(),
        story: inputs.story.beats.clone(),
        player: SavePlayer {
            position: transform.translation.to_array(),
            health_current: health.current,
            health_max: health.max,
            barrier_current: barrier.current,
            barrier_max: barrier.max,
            level: level.level,
            xp: level.xp,
            next_level_xp: level.next_level_xp,
            paragon_rank: paragon.rank,
            paragon_xp: paragon.xp,
            paragon_next_rank_xp: paragon.next_rank_xp,
            damage_bonus: damage.0,
            fury_current: fury.current,
            fury_max: fury.max,
            fury_basic_gain: fury.basic_gain,
            dash_cost: fury.dash_cost,
            nova_cost: fury.nova_cost,
            rupture_cost: fury.rupture_cost,
            equipment: SaveEquipment {
                weapon_name: equipment.weapon_name.clone(),
                quality: equipment.quality.clone(),
                crit_chance: equipment.crit_chance,
                health_bonus: equipment.health_bonus,
                armor_bonus: equipment.armor_bonus,
                legendary_power: equipment.legendary_power,
                temper_level: equipment.temper_level,
                socketed_gem: equipment.socketed_gem,
            },
            charm: charm.clone(),
            inventory: inventory
                .items
                .iter()
                .map(SaveInventoryItem::from)
                .collect(),
            inventory_capacity: inventory.capacity,
            armory_loadout: armory.legacy_loadout(),
            armory_loadouts: normalized_armory_loadouts(armory),
            armory_active_slot: armory.active_slot.min(ARMORY_SLOT_COUNT - 1),
            legendary_codex: codex.unlocked.clone(),
            legendary_codex_attuned: codex.attuned,
            death_checkpoint: death_ward.checkpoint.to_array(),
            revives_used: death_ward.revives_used,
            max_revives: death_ward.max_revives,
            revive_gold_penalty: death_ward.gold_penalty,
            potions: potions.current,
            max_potions: potions.max,
            potion_cooldown_secs: potions.cooldown_secs,
            potion_cooldown_remaining_secs: if potions.cooldown.is_finished() {
                0.0
            } else {
                potions.cooldown.remaining_secs()
            },
            elixirs: elixirs.current,
            max_elixirs: elixirs.max,
            selected_elixir: elixirs.selected,
            active_elixir: elixir_buff.kind,
            elixir_buff_secs: if elixir_buff.timer.is_finished() {
                0.0
            } else {
                elixir_buff.timer.remaining_secs()
            },
            relic_buff_secs: if relic_buff.timer.is_finished() {
                0.0
            } else {
                relic_buff.timer.remaining_secs()
            },
            relic_damage_multiplier: relic_buff.damage_multiplier,
            relic_speed_multiplier: relic_buff.speed_multiplier,
            fortune_buff_secs: if fortune_buff.timer.is_finished() {
                0.0
            } else {
                fortune_buff.timer.remaining_secs()
            },
            conduit_buff_secs: if conduit_buff.timer.is_finished() {
                0.0
            } else {
                conduit_buff.timer.remaining_secs()
            },
            glory_buff_secs: if glory_buff.timer.is_finished() {
                0.0
            } else {
                glory_buff.timer.remaining_secs()
            },
            glory_stacks: glory_buff.stacks,
            surge_buff_secs: if surge_buff.timer.is_finished() {
                0.0
            } else {
                surge_buff.timer.remaining_secs()
            },
            surge_charge: surge_buff.charge,
            surge_max_charge: surge_buff.max_charge,
            surge_duration_secs: surge_buff.duration_secs,
            town_portal_cooldown_secs: if town_portal.cooldown.is_finished() {
                0.0
            } else {
                town_portal.cooldown.remaining_secs()
            },
            town_portal_return_position: town_portal
                .return_position
                .map(|position| position.to_array()),
            evade_active_secs: if evade.active.is_finished() {
                0.0
            } else {
                evade.active.remaining_secs()
            },
            evade_cooldown_secs: evade.cooldown_secs,
            evade_cooldown_remaining_secs: if evade.cooldown.is_finished() {
                0.0
            } else {
                evade.cooldown.remaining_secs()
            },
            companion_rank: companion.rank,
            companion_xp: companion.xp,
            companion_next_rank_xp: companion.next_rank_xp,
            companion_stance: companion.stance,
            companion_command_cooldown_secs: companion.command_cooldown,
            chapter_boon: chapter_boon.choice,
            chapter_boon_empowered: chapter_boon.empowered,
            chapter_boon_momentum_stacks: chapter_boon.momentum_stacks,
            talent_points: talents.points,
            talent_wrath: talents.wrath,
            talent_vigor: talents.vigor,
            talent_focus: talents.focus,
            dash_rune: runes.dash,
            nova_rune: runes.nova,
            rupture_rune: runes.rupture,
        },
    })
}

struct ApplyResources<'a> {
    stats: &'a mut RunStats,
    difficulty: &'a mut DifficultySettings,
    modifier: &'a mut ChapterModifier,
    chapter: &'a mut ChapterProgress,
    bounty: &'a mut BountyProgress,
    challenges: &'a mut ChapterChallenges,
    mastery: &'a mut ChapterMastery,
    journal: &'a mut LoreJournal,
    milestones: &'a mut Milestones,
    bestiary: &'a mut Bestiary,
    obelisk: &'a mut BloodObelisk,
    rift: &'a mut EmberRift,
    story: &'a mut StoryLog,
}

struct ApplyQueries<'a, 'w, 's> {
    player_core: &'a mut LoadPlayerCoreQuery<'w, 's>,
    player_state: &'a mut LoadPlayerStateQuery<'w, 's>,
    runes: &'a mut LoadRunesQuery<'w, 's>,
    companion: &'a mut LoadCompanionQuery<'w, 's>,
    armory: &'a mut LoadArmoryQuery<'w, 's>,
    paragon: &'a mut LoadParagonQuery<'w, 's>,
    codex: &'a mut LoadCodexQuery<'w, 's>,
    charm: &'a mut LoadCharmQuery<'w, 's>,
    death_ward: &'a mut LoadDeathWardQuery<'w, 's>,
    barrier: &'a mut LoadBarrierQuery<'w, 's>,
    conduit: &'a mut LoadConduitQuery<'w, 's>,
    glory: &'a mut LoadGloryQuery<'w, 's>,
    surge: &'a mut LoadSurgeQuery<'w, 's>,
    interactables: &'a mut LoadInteractableQuery<'w, 's>,
    breakables: &'a mut LoadBreakableQuery<'w, 's>,
    commands: &'a mut Commands<'w, 's>,
}

fn apply_save(
    save: &SaveGame,
    resources: ApplyResources,
    queries: ApplyQueries,
) -> Result<(), String> {
    if save.version != 1 {
        return Err(format!("unsupported save version {}", save.version));
    }
    let (
        mut transform,
        mut health,
        mut level,
        mut damage,
        mut fury,
        mut equipment,
        mut inventory,
        mut chapter_boon,
        mut talents,
    ) = queries
        .player_core
        .single_mut()
        .map_err(|_| "expected exactly one player".to_string())?;
    let (
        mut potions,
        mut elixirs,
        mut elixir_buff,
        mut relic_buff,
        mut fortune_buff,
        mut town_portal,
        mut evade,
    ) = queries
        .player_state
        .single_mut()
        .map_err(|_| "expected exactly one player state".to_string())?;
    let mut runes = queries
        .runes
        .single_mut()
        .map_err(|_| "expected exactly one player runes".to_string())?;
    let mut companion = queries
        .companion
        .single_mut()
        .map_err(|_| "expected exactly one player companion".to_string())?;
    let mut armory = queries
        .armory
        .single_mut()
        .map_err(|_| "expected exactly one player armory".to_string())?;
    let mut paragon = queries
        .paragon
        .single_mut()
        .map_err(|_| "expected exactly one player paragon".to_string())?;
    let mut codex = queries
        .codex
        .single_mut()
        .map_err(|_| "expected exactly one player codex".to_string())?;
    let mut charm = queries
        .charm
        .single_mut()
        .map_err(|_| "expected exactly one player charm".to_string())?;
    let mut death_ward = queries
        .death_ward
        .single_mut()
        .map_err(|_| "expected exactly one player death ward".to_string())?;
    let mut barrier = queries
        .barrier
        .single_mut()
        .map_err(|_| "expected exactly one player barrier".to_string())?;
    let mut conduit_buff = queries
        .conduit
        .single_mut()
        .map_err(|_| "expected exactly one player conduit".to_string())?;
    let mut glory_buff = queries
        .glory
        .single_mut()
        .map_err(|_| "expected exactly one player glory".to_string())?;
    let mut surge_buff = queries
        .surge
        .single_mut()
        .map_err(|_| "expected exactly one player surge".to_string())?;

    resources.stats.kills = save.stats.kills;
    resources.stats.gold = save.stats.gold;
    resources.stats.ember_shards = save.stats.ember_shards;
    resources.stats.affix_essence = save.stats.affix_essence;
    resources.stats.valor_stacks = save.stats.valor_stacks.min(5);
    resources.stats.best_valor_stacks = save.stats.best_valor_stacks.min(5);
    resources.stats.valor_timer_secs = if resources.stats.valor_stacks == 0 {
        0.0
    } else {
        save.stats.valor_timer_secs.max(0.0)
    };
    resources.stats.elapsed_secs = save.stats.elapsed_secs;
    resources.stats.completion_bonus_gold = save.stats.completion_bonus_gold;
    resources.stats.completion_bonus_shards = save.stats.completion_bonus_shards;
    resources.stats.completion_bonus_essence = save.stats.completion_bonus_essence;
    resources.stats.chapter_clear_bonus_gold = save.stats.chapter_clear_bonus_gold;
    resources.stats.chapter_clear_bonus_shards = save.stats.chapter_clear_bonus_shards;
    resources.stats.chapter_clear_bonus_essence = save.stats.chapter_clear_bonus_essence;
    resources.stats.chapter_clear_reward_claimed = save.stats.chapter_clear_reward_claimed;
    resources.stats.journey_score = save.stats.journey_score;
    resources.stats.journey_bonus_gold = save.stats.journey_bonus_gold;
    resources.stats.journey_bonus_shards = save.stats.journey_bonus_shards;
    resources.stats.journey_bonus_essence = save.stats.journey_bonus_essence;
    resources.stats.journey_reward_claimed = save.stats.journey_reward_claimed;
    resources.stats.renown_rank = save.stats.renown_rank;
    resources.stats.renown_bonus_gold = save.stats.renown_bonus_gold;
    resources.stats.renown_bonus_shards = save.stats.renown_bonus_shards;
    resources.stats.renown_bonus_essence = save.stats.renown_bonus_essence;
    resources.stats.renown_bonus_claimed = save.stats.renown_bonus_claimed;
    resources.stats.stash_bonus_gold = save.stats.stash_bonus_gold;
    resources.stats.stash_bonus_shards = save.stats.stash_bonus_shards;
    resources.stats.stash_bonus_essence = save.stats.stash_bonus_essence;
    resources.stats.stash_bonus_claimed = save.stats.stash_bonus_claimed;
    resources.stats.altar_seals = save.stats.altar_seals;
    resources.stats.altar_bonus_gold = save.stats.altar_bonus_gold;
    resources.stats.altar_bonus_shards = save.stats.altar_bonus_shards;
    resources.stats.altar_bonus_essence = save.stats.altar_bonus_essence;
    resources.stats.altar_bonus_claimed = save.stats.altar_bonus_claimed;
    resources.stats.primal_caches = save.stats.primal_caches;
    resources.stats.primal_cache_bonus_gold = save.stats.primal_cache_bonus_gold;
    resources.stats.primal_cache_bonus_shards = save.stats.primal_cache_bonus_shards;
    resources.stats.primal_cache_bonus_essence = save.stats.primal_cache_bonus_essence;
    resources.stats.primal_cache_items_claimed = save.stats.primal_cache_items_claimed;
    resources.stats.echo_keystones = save.stats.echo_keystones;
    resources.stats.primal_cache_echo_items = save.stats.primal_cache_echo_items;
    resources.stats.malrec_soul_sigils = save.stats.malrec_soul_sigils;
    resources.stats.malrec_soul_sigils_earned = save.stats.malrec_soul_sigils_earned;
    resources.stats.soul_sigil_caches = save.stats.soul_sigil_caches;
    resources.stats.completion_reward_claimed = save.stats.completion_reward_claimed;
    resources.stats.massacre_streak = save.stats.massacre_streak;
    resources.stats.best_massacre_streak = save.stats.best_massacre_streak;
    resources.stats.massacre_timer_secs = save.stats.massacre_timer_secs.max(0.0);
    resources.stats.massacre_bonus_gold = save.stats.massacre_bonus_gold;
    resources.stats.ancient_augments = save.stats.ancient_augments;
    resources.stats.primal_infusions = save.stats.primal_infusions;
    resources.stats.potions_used = save.stats.potions_used;
    resources.stats.last_stand_potions = save.stats.last_stand_potions;
    resources.stats.boss_enraged = save.stats.boss_enraged;
    resources.stats.boss_staggers = save.stats.boss_staggers;
    resources.stats.shrine_resonance_triggered = save.stats.shrine_resonance_triggered;
    resources.stats.elite_affix_kills = save.stats.elite_affix_kills;
    resources.stats.affix_codex_mask = save.stats.affix_codex_mask;
    resources.stats.seal_warden_slain = save.stats.seal_warden_slain;
    resources.stats.cursed_ambush_kills = save.stats.cursed_ambush_kills;
    resources.stats.champion_pack_kills = save.stats.champion_pack_kills;
    resources.stats.champion_pack_reward_claimed = save.stats.champion_pack_reward_claimed;
    resources.stats.nemesis_kills = save.stats.nemesis_kills;
    resources.stats.treasure_vaults_opened = save.stats.treasure_vaults_opened;
    resources.stats.breakables_smashed = save.stats.breakables_smashed;
    resources.stats.health_globes_collected = save.stats.health_globes_collected;
    resources.stats.surge_kills = save.stats.surge_kills;
    resources.stats.reap_dash_hits = save.stats.reap_dash_hits;
    resources.stats.hemorrhage_rupture_hits = save.stats.hemorrhage_rupture_hits;
    resources.stats.frost_nova_hits = save.stats.frost_nova_hits;
    resources.stats.armory_loadouts_saved = save.stats.armory_loadouts_saved;
    resources.stats.town_portal_returns = save.stats.town_portal_returns;
    resources.stats.pylon_kills = save.stats.pylon_kills;
    resources.stats.salvage_progress = save.stats.salvage_progress;
    resources.stats.salvage_caches = save.stats.salvage_caches;
    resources.stats.alchemy_gold_transmutes = save.stats.alchemy_gold_transmutes;
    resources.stats.alchemy_essence_transmutes = save.stats.alchemy_essence_transmutes;
    resources.stats.alchemy_keystone_transmutes = save.stats.alchemy_keystone_transmutes;
    resources.stats.loot_filter_cycles = save.stats.loot_filter_cycles;
    resources.stats.codex_attuned_kills = save.stats.codex_attuned_kills;
    resources.stats.set_resonance_kills = save.stats.set_resonance_kills;
    resources.stats.ruby_socketed = save.stats.ruby_socketed;
    resources.stats.emerald_socketed = save.stats.emerald_socketed;
    resources.stats.amethyst_socketed = save.stats.amethyst_socketed;
    resources.stats.topaz_socketed = save.stats.topaz_socketed;
    resources.stats.iron_elixir_used = save.stats.iron_elixir_used;
    resources.stats.wrath_elixir_used = save.stats.wrath_elixir_used;
    resources.stats.haste_elixir_used = save.stats.haste_elixir_used;
    resources.stats.reliquary_momentum = save.stats.reliquary_momentum;
    resources.stats.best_reliquary_momentum = save.stats.best_reliquary_momentum;
    resources.stats.ashen_threat = save.stats.ashen_threat;
    resources.stats.ashen_threat_surges = save.stats.ashen_threat_surges;
    resources.difficulty.current = save.difficulty;
    resources.modifier.kind = save.modifier;
    resources.modifier.affix = save.modifier_affix;
    apply_chapter_save(&save.chapter, resources.chapter);
    *resources.bounty = save.bounty.clone();
    resources.challenges.restore(save.challenges.clone());
    *resources.mastery = save.mastery.clone();
    apply_journal_save(&save.journal, resources.journal);
    resources.milestones.restore(save.milestones.clone());
    resources.bestiary.restore(save.bestiary.clone());
    *resources.obelisk = save.obelisk.clone();
    *resources.rift = save.rift.clone();
    resources.story.restore(save.story.clone());
    apply_interactable_save(&save.interactables, queries.interactables);
    apply_breakable_save(&save.breakables, queries.breakables, queries.commands);
    transform.translation = Vec3::from_array(save.player.position);
    health.current = save
        .player
        .health_current
        .min(save.player.health_max)
        .max(0.0);
    health.max = save.player.health_max.max(1.0);
    barrier.max = save.player.barrier_max.max(1.0);
    barrier.current = save.player.barrier_current.clamp(0.0, barrier.max);
    level.level = save.player.level.max(1);
    level.xp = save.player.xp;
    level.next_level_xp = save.player.next_level_xp.max(1);
    paragon.rank = save.player.paragon_rank;
    paragon.xp = save.player.paragon_xp;
    paragon.next_rank_xp = save.player.paragon_next_rank_xp.max(1);
    damage.0 = save.player.damage_bonus;
    fury.max = save.player.fury_max.max(1.0);
    fury.current = save.player.fury_current.clamp(0.0, fury.max);
    fury.basic_gain = save.player.fury_basic_gain.max(0.0);
    fury.dash_cost = save.player.dash_cost.max(0.0);
    fury.nova_cost = save.player.nova_cost.max(0.0);
    fury.rupture_cost = save.player.rupture_cost.max(0.0);
    equipment.weapon_name = save.player.equipment.weapon_name.clone();
    equipment.quality = save.player.equipment.quality.clone();
    equipment.crit_chance = save.player.equipment.crit_chance;
    equipment.health_bonus = save.player.equipment.health_bonus;
    equipment.armor_bonus = save.player.equipment.armor_bonus;
    equipment.legendary_power = save.player.equipment.legendary_power;
    equipment.temper_level = save.player.equipment.temper_level;
    equipment.socketed_gem = save.player.equipment.socketed_gem;
    *charm = save.player.charm.clone();
    inventory.items = save
        .player
        .inventory
        .iter()
        .map(InventoryItem::from)
        .collect();
    inventory.capacity = save.player.inventory_capacity.max(inventory.items.len());
    *armory = restored_armory(&save.player);
    codex.restore(save.player.legendary_codex.clone());
    backfill_codex_from_items(&mut codex, &equipment, &inventory);
    codex.restore_attunement(save.player.legendary_codex_attuned);
    death_ward.checkpoint = Vec3::from_array(save.player.death_checkpoint);
    death_ward.max_revives = save.player.max_revives.max(1);
    death_ward.revives_used = save.player.revives_used.min(death_ward.max_revives);
    death_ward.gold_penalty = save.player.revive_gold_penalty;
    potions.current = save.player.potions.min(save.player.max_potions);
    potions.max = save.player.max_potions.max(1);
    potions.cooldown_secs = if save.player.potion_cooldown_secs > 0.0 {
        save.player.potion_cooldown_secs
    } else {
        potion_cooldown_secs_for_capacity(potions.max)
    };
    potions.cooldown = Timer::from_seconds(
        save.player.potion_cooldown_remaining_secs.max(0.0),
        TimerMode::Once,
    );
    elixirs.current = save.player.elixirs.min(save.player.max_elixirs.max(1));
    elixirs.max = save.player.max_elixirs.max(1);
    elixirs.selected = save.player.selected_elixir;
    elixir_buff.timer = Timer::from_seconds(save.player.elixir_buff_secs.max(0.0), TimerMode::Once);
    if save.player.elixir_buff_secs > 0.0 {
        configure_elixir_buff(
            save.player.active_elixir,
            elixirs.armor_bonus,
            &mut elixir_buff,
        );
    } else {
        configure_elixir_buff(ElixirKind::Iron, elixirs.armor_bonus, &mut elixir_buff);
    }
    relic_buff.timer = Timer::from_seconds(save.player.relic_buff_secs.max(0.0), TimerMode::Once);
    relic_buff.damage_multiplier = save.player.relic_damage_multiplier.max(1.0);
    relic_buff.speed_multiplier = save.player.relic_speed_multiplier.max(1.0);
    fortune_buff.timer =
        Timer::from_seconds(save.player.fortune_buff_secs.max(0.0), TimerMode::Once);
    conduit_buff.timer =
        Timer::from_seconds(save.player.conduit_buff_secs.max(0.0), TimerMode::Once);
    conduit_buff.pulse = Timer::from_seconds(0.65, TimerMode::Repeating);
    conduit_buff.damage = 18.0;
    conduit_buff.radius = 5.4;
    glory_buff.timer = Timer::from_seconds(save.player.glory_buff_secs.max(0.0), TimerMode::Once);
    glory_buff.stacks = save.player.glory_stacks.min(glory_buff.max_stacks);
    if save.player.glory_buff_secs <= 0.0 {
        glory_buff.stacks = 0;
    }
    surge_buff.timer = Timer::from_seconds(save.player.surge_buff_secs.max(0.0), TimerMode::Once);
    surge_buff.max_charge = save.player.surge_max_charge.max(1);
    surge_buff.charge = save.player.surge_charge.min(surge_buff.max_charge);
    if save.player.surge_duration_secs > 0.0 {
        surge_buff.duration_secs = save.player.surge_duration_secs;
    }
    town_portal.cooldown = Timer::from_seconds(
        save.player.town_portal_cooldown_secs.max(0.0),
        TimerMode::Once,
    );
    town_portal.return_position = save
        .player
        .town_portal_return_position
        .map(Vec3::from_array);
    if save.player.evade_cooldown_secs > 0.0 {
        evade.cooldown_secs = save.player.evade_cooldown_secs;
    }
    evade.active = Timer::from_seconds(save.player.evade_active_secs.max(0.0), TimerMode::Once);
    evade.cooldown = Timer::from_seconds(
        save.player.evade_cooldown_remaining_secs.max(0.0),
        TimerMode::Once,
    );
    companion.rank = save.player.companion_rank.max(1);
    companion.xp = save.player.companion_xp;
    companion.next_rank_xp = save.player.companion_next_rank_xp.max(1);
    companion.stance = save.player.companion_stance;
    companion.command_cooldown = save.player.companion_command_cooldown_secs.max(0.0);
    chapter_boon.choice = save.player.chapter_boon;
    chapter_boon.empowered = save.player.chapter_boon_empowered;
    chapter_boon.momentum_stacks = save.player.chapter_boon_momentum_stacks;
    talents.points = save.player.talent_points;
    talents.wrath = save.player.talent_wrath;
    talents.vigor = save.player.talent_vigor;
    talents.focus = save.player.talent_focus;
    runes.dash = save.player.dash_rune;
    runes.nova = save.player.nova_rune;
    runes.rupture = save.player.rupture_rune;
    Ok(())
}

fn apply_chapter_save(save: &SaveChapter, chapter: &mut ChapterProgress) {
    chapter.phase = save.phase;
    chapter.objective = save.objective.clone();
    chapter.enemies_slain = save.enemies_slain;
    chapter.chest_opened = save.chest_opened;
    chapter.altar_activated = save.altar_activated;
    chapter.sanctum_wave_spawned = save.sanctum_wave_spawned;
    chapter.seal_fragments = save
        .seal_fragments
        .max(inferred_seal_fragments_from_legacy_chapter(save));
    chapter.boss_spawned = save.boss_spawned;
    chapter.boss_slain = save.boss_slain;
    chapter.last_kill = save.last_kill.clone();
    chapter.interaction_hint = save.interaction_hint.clone();
}

fn inferred_seal_fragments_from_legacy_chapter(save: &SaveChapter) -> u32 {
    if save.boss_spawned || save.boss_slain || save.phase == ChapterPhase::Boss {
        return crate::chapter::REQUIRED_SEAL_FRAGMENTS;
    }
    u32::from(save.chest_opened)
        + u32::from(save.sanctum_wave_spawned || save.phase == ChapterPhase::Ritual)
        + u32::from(save.altar_activated)
}

fn normalized_armory_loadouts(armory: &Armory) -> Vec<Option<ArmoryLoadout>> {
    let mut loadouts = armory.loadouts.clone();
    if loadouts.len() < ARMORY_SLOT_COUNT {
        loadouts.resize_with(ARMORY_SLOT_COUNT, || None);
    } else if loadouts.len() > ARMORY_SLOT_COUNT {
        loadouts.truncate(ARMORY_SLOT_COUNT);
    }
    loadouts
}

fn restored_armory(player: &SavePlayer) -> Armory {
    let mut armory = Armory {
        loadouts: if player.armory_loadouts.is_empty() {
            let mut loadouts = vec![None; ARMORY_SLOT_COUNT];
            loadouts[0] = player.armory_loadout.clone();
            loadouts
        } else {
            player.armory_loadouts.clone()
        },
        active_slot: player.armory_active_slot,
    };
    armory.normalize();
    if armory.loadouts[0].is_none() {
        armory.loadouts[0] = player.armory_loadout.clone();
    }
    armory
}

fn backfill_codex_from_items(
    codex: &mut LegendaryCodex,
    equipment: &Equipment,
    inventory: &Inventory,
) {
    codex.unlock(equipment.legendary_power);
    for item in &inventory.items {
        codex.unlock(item.legendary_power);
    }
}

fn apply_journal_save(entries: &[LoreEntryId], journal: &mut LoreJournal) {
    journal.entries = entries.to_vec();
    journal.latest = entries
        .last()
        .map(|entry| format!("{}: {}", entry.title(), entry.body()))
        .unwrap_or_default();
}

fn apply_interactable_save(saves: &[SaveInteractable], interactables: &mut LoadInteractableQuery) {
    for (transform, mut interactable) in interactables.iter_mut() {
        if interactable.reusable {
            continue;
        }
        if let Some(save) = saves.iter().find(|save| {
            save.kind == interactable.kind
                && Vec3::from_array(save.position).distance(transform.translation) <= 0.1
        }) {
            interactable.used = save.used;
        }
    }
}

fn apply_breakable_save(
    saves: &Option<Vec<SaveBreakable>>,
    breakables: &mut LoadBreakableQuery,
    commands: &mut Commands,
) {
    let Some(saves) = saves else {
        return;
    };
    for (entity, transform, mut health) in breakables.iter_mut() {
        if let Some(save) = saves
            .iter()
            .find(|save| Vec3::from_array(save.position).distance(transform.translation) <= 0.1)
        {
            health.max = save.health_max.max(1.0);
            health.current = save.health_current.clamp(0.0, health.max);
        } else {
            if let Ok(mut entity_commands) = commands.get_entity(entity) {
                entity_commands.try_despawn();
            }
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn write_save(path: impl AsRef<Path>, save: &SaveGame) -> Result<(), String> {
    let path = path.as_ref();
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|err| err.to_string())?;
    }
    let content = ron::ser::to_string_pretty(save, ron::ser::PrettyConfig::default())
        .map_err(|err| err.to_string())?;
    fs::write(path, content).map_err(|err| err.to_string())
}

#[cfg(target_arch = "wasm32")]
fn write_save(_path: impl AsRef<Path>, _save: &SaveGame) -> Result<(), String> {
    Err("save slots are not persisted in the web build".to_string())
}

#[cfg(not(target_arch = "wasm32"))]
fn read_save(path: impl AsRef<Path>) -> Result<SaveGame, String> {
    let content = fs::read_to_string(path).map_err(|err| err.to_string())?;
    ron::from_str(&content).map_err(|err| err.to_string())
}

#[cfg(target_arch = "wasm32")]
fn read_save(_path: impl AsRef<Path>) -> Result<SaveGame, String> {
    Err("save slots are not persisted in the web build".to_string())
}

impl From<&InventoryItem> for SaveInventoryItem {
    fn from(item: &InventoryItem) -> Self {
        Self {
            name: item.name.clone(),
            quality: item.quality.clone(),
            damage_bonus: item.damage_bonus,
            crit_chance: item.crit_chance,
            health_bonus: item.health_bonus,
            armor_bonus: item.armor_bonus,
            legendary_power: item.legendary_power,
            temper_level: item.temper_level,
            socketed_gem: item.socketed_gem,
        }
    }
}

impl From<&SaveInventoryItem> for InventoryItem {
    fn from(item: &SaveInventoryItem) -> Self {
        Self {
            name: item.name.clone(),
            quality: item.quality.clone(),
            damage_bonus: item.damage_bonus,
            crit_chance: item.crit_chance,
            health_bonus: item.health_bonus,
            armor_bonus: item.armor_bonus,
            legendary_power: item.legendary_power,
            temper_level: item.temper_level,
            socketed_gem: item.socketed_gem,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pending_load_game_consumes_one_request() {
        let mut pending = PendingLoadGame::default();

        assert!(!pending.take());
        pending.request();
        assert!(pending.take());
        assert!(!pending.take());
    }

    #[test]
    fn save_game_round_trips_through_ron() {
        let save = SaveGame {
            version: 1,
            stats: SaveStats {
                kills: 7,
                gold: 42,
                ember_shards: 9,
                affix_essence: 5,
                valor_stacks: 3,
                best_valor_stacks: 5,
                valor_timer_secs: 42.0,
                elapsed_secs: 215.0,
                completion_bonus_gold: 275,
                completion_bonus_shards: 11,
                completion_bonus_essence: 6,
                chapter_clear_bonus_gold: 220,
                chapter_clear_bonus_shards: 7,
                chapter_clear_bonus_essence: 3,
                chapter_clear_reward_claimed: true,
                journey_score: 32,
                journey_bonus_gold: 260,
                journey_bonus_shards: 8,
                journey_bonus_essence: 5,
                journey_reward_claimed: true,
                renown_rank: 3,
                renown_bonus_gold: 105,
                renown_bonus_shards: 1,
                renown_bonus_essence: 0,
                renown_bonus_claimed: true,
                stash_bonus_gold: 80,
                stash_bonus_shards: 3,
                stash_bonus_essence: 2,
                stash_bonus_claimed: true,
                altar_seals: 5,
                altar_bonus_gold: 125,
                altar_bonus_shards: 2,
                altar_bonus_essence: 1,
                altar_bonus_claimed: true,
                primal_caches: 1,
                primal_cache_bonus_gold: 350,
                primal_cache_bonus_shards: 18,
                primal_cache_bonus_essence: 17,
                primal_cache_items_claimed: 1,
                echo_keystones: 1,
                primal_cache_echo_items: 1,
                malrec_soul_sigils: 2,
                malrec_soul_sigils_earned: 1,
                soul_sigil_caches: 1,
                completion_reward_claimed: true,
                massacre_streak: 5,
                best_massacre_streak: 7,
                massacre_timer_secs: 2.25,
                massacre_bonus_gold: 35,
                ancient_augments: 1,
                primal_infusions: 1,
                potions_used: 2,
                last_stand_potions: 3,
                boss_enraged: true,
                boss_staggers: 2,
                shrine_resonance_triggered: true,
                elite_affix_kills: 3,
                affix_codex_mask: 0b11_1111,
                seal_warden_slain: true,
                cursed_ambush_kills: 3,
                champion_pack_kills: 4,
                champion_pack_reward_claimed: true,
                nemesis_kills: 2,
                treasure_vaults_opened: 1,
                breakables_smashed: 4,
                health_globes_collected: 3,
                surge_kills: 3,
                reap_dash_hits: 1,
                hemorrhage_rupture_hits: 2,
                frost_nova_hits: 3,
                armory_loadouts_saved: 3,
                town_portal_returns: 1,
                pylon_kills: 3,
                salvage_progress: 2,
                salvage_caches: 1,
                alchemy_gold_transmutes: 1,
                alchemy_essence_transmutes: 2,
                alchemy_keystone_transmutes: 1,
                loot_filter_cycles: 3,
                codex_attuned_kills: 5,
                set_resonance_kills: 8,
                ruby_socketed: true,
                emerald_socketed: true,
                amethyst_socketed: true,
                topaz_socketed: false,
                iron_elixir_used: true,
                wrath_elixir_used: true,
                haste_elixir_used: false,
                reliquary_momentum: 4,
                best_reliquary_momentum: 5,
                ashen_threat: 72,
                ashen_threat_surges: 2,
            },
            difficulty: Difficulty::Nightmare,
            modifier: ChapterModifierKind::BloodTithe,
            modifier_affix: OrdealAffix::AshenHunger,
            chapter: SaveChapter {
                phase: ChapterPhase::Boss,
                objective: "Defeat Malrec, Keeper of Ash".to_string(),
                enemies_slain: 6,
                chest_opened: true,
                altar_activated: true,
                sanctum_wave_spawned: true,
                seal_fragments: 3,
                boss_spawned: true,
                boss_slain: false,
                last_kill: "Last slain: Cinder Acolyte at 7, -1".to_string(),
                interaction_hint: "Malrec has entered the reliquary".to_string(),
            },
            bounty: BountyProgress {
                current: 8,
                completed: true,
                reward_claimed: true,
                latest: "Reliquary Hunt completed".to_string(),
                ..default()
            },
            challenges: vec![ChallengeProgress {
                id: crate::challenge::ChallengeId::TreasureHunter,
                completed: true,
                reward_claimed: true,
            }],
            mastery: ChapterMastery {
                points: 6,
                rank: crate::mastery::MasteryRank::Master,
                reward_claimed: true,
                reward_gold: 260,
                reward_shards: 8,
                reward_essence: 5,
                boss_bonus_points: 2,
                latest: "Chapter mastery: Master".to_string(),
            },
            interactables: vec![
                SaveInteractable {
                    kind: InteractableKind::Cache,
                    position: [-8.0, 0.0, -5.5],
                    used: true,
                },
                SaveInteractable {
                    kind: InteractableKind::Altar,
                    position: [6.5, 0.0, 4.8],
                    used: true,
                },
            ],
            breakables: Some(vec![SaveBreakable {
                position: [-7.4, 0.36, 1.8],
                health_current: 8.0,
                health_max: 18.0,
            }]),
            journal: vec![LoreEntryId::StewardWarning, LoreEntryId::MalrecPact],
            milestones: vec![
                MilestoneId::CacheOpened,
                MilestoneId::FortuneBlessed,
                MilestoneId::TemperedSteel,
            ],
            bestiary: {
                let mut entries = std::collections::BTreeMap::new();
                entries.insert(
                    "skeleton".to_string(),
                    BestiaryEntry {
                        display_name: "Ashbone Guard".to_string(),
                        kills: 4,
                        claimed_trophy_kills: 3,
                    },
                );
                entries
            },
            obelisk: BloodObelisk {
                state: crate::obelisk::ObeliskState::Active,
                kills: 2,
                target_kills: crate::obelisk::OBELISK_TARGET_KILLS,
                remaining_secs: 21.0,
                reward_claimed: false,
            },
            rift: EmberRift {
                state: crate::rift::RiftState::Active,
                kills: 3,
                target_kills: crate::rift::RIFT_TARGET_KILLS,
                remaining_secs: 38.0,
                reward_claimed: false,
                best_remaining_secs: 0.0,
            },
            story: vec![
                StoryBeat::Arrival,
                StoryBeat::CacheRevealed,
                StoryBeat::KeeperAwakened,
            ],
            player: SavePlayer {
                position: [1.0, 0.0, -2.0],
                health_current: 88.0,
                health_max: 120.0,
                barrier_current: 18.0,
                barrier_max: 42.0,
                level: 3,
                xp: 12,
                next_level_xp: 126,
                paragon_rank: 2,
                paragon_xp: 45,
                paragon_next_rank_xp: 237,
                damage_bonus: 9.0,
                fury_current: 64.0,
                fury_max: 100.0,
                fury_basic_gain: 22.0,
                dash_cost: 20.0,
                nova_cost: 37.0,
                rupture_cost: 26.0,
                equipment: SaveEquipment {
                    weapon_name: "rare Moonforged Cleaver".to_string(),
                    quality: "rare".to_string(),
                    crit_chance: 0.08,
                    health_bonus: 10.0,
                    armor_bonus: 8.0,
                    legendary_power: LegendaryPower::Emberbrand,
                    temper_level: 2,
                    socketed_gem: Some(SocketedGem {
                        kind: crate::player::GemKind::Ruby,
                        rank: 2,
                    }),
                },
                charm: Charm {
                    name: "Stormglass Charm".to_string(),
                    quality: "legendary".to_string(),
                    damage_bonus: 5.0,
                    crit_chance: 0.035,
                },
                inventory: vec![SaveInventoryItem {
                    name: "rare Moonforged Cleaver".to_string(),
                    quality: "rare".to_string(),
                    damage_bonus: 9.0,
                    crit_chance: 0.08,
                    health_bonus: 10.0,
                    armor_bonus: 8.0,
                    legendary_power: LegendaryPower::Emberbrand,
                    temper_level: 2,
                    socketed_gem: Some(SocketedGem {
                        kind: crate::player::GemKind::Ruby,
                        rank: 2,
                    }),
                }],
                inventory_capacity: 12,
                armory_loadout: Some(ArmoryLoadout {
                    weapon: InventoryItem {
                        name: "rare Moonforged Cleaver".to_string(),
                        quality: "rare".to_string(),
                        damage_bonus: 9.0,
                        crit_chance: 0.08,
                        health_bonus: 10.0,
                        armor_bonus: 8.0,
                        legendary_power: LegendaryPower::Emberbrand,
                        temper_level: 2,
                        socketed_gem: Some(SocketedGem {
                            kind: crate::player::GemKind::Ruby,
                            rank: 2,
                        }),
                    },
                    charm: Charm {
                        name: "Stormglass Charm".to_string(),
                        quality: "legendary".to_string(),
                        damage_bonus: 5.0,
                        crit_chance: 0.035,
                    },
                    dash_rune: DashRune::Reap,
                    nova_rune: NovaRune::Frost,
                    rupture_rune: RuptureRune::Hemorrhage,
                    codex_attunement: LegendaryPower::Frostbrand,
                }),
                armory_loadouts: vec![],
                armory_active_slot: 0,
                legendary_codex: vec![LegendaryPower::Emberbrand, LegendaryPower::Frostbrand],
                legendary_codex_attuned: LegendaryPower::Frostbrand,
                death_checkpoint: [6.5, 0.0, 5.5],
                revives_used: 1,
                max_revives: 2,
                revive_gold_penalty: 60,
                potions: 3,
                max_potions: 5,
                potion_cooldown_secs: 12.0,
                potion_cooldown_remaining_secs: 4.5,
                elixirs: 2,
                max_elixirs: 3,
                selected_elixir: ElixirKind::Wrath,
                active_elixir: ElixirKind::Haste,
                elixir_buff_secs: 12.5,
                relic_buff_secs: 14.0,
                relic_damage_multiplier: 1.55,
                relic_speed_multiplier: 1.24,
                fortune_buff_secs: 18.0,
                conduit_buff_secs: 9.0,
                glory_buff_secs: 11.0,
                glory_stacks: 2,
                surge_buff_secs: 6.5,
                surge_charge: 4,
                surge_max_charge: 9,
                surge_duration_secs: 9.0,
                town_portal_cooldown_secs: 7.0,
                town_portal_return_position: Some([6.0, 0.0, -3.0]),
                evade_active_secs: 0.24,
                evade_cooldown_secs: 4.5,
                evade_cooldown_remaining_secs: 3.75,
                companion_rank: 4,
                companion_xp: 22,
                companion_next_rank_xp: 180,
                companion_stance: CompanionStance::Seeker,
                companion_command_cooldown_secs: 4.25,
                chapter_boon: ChapterBoonChoice::CrimsonOath,
                chapter_boon_empowered: true,
                chapter_boon_momentum_stacks: 3,
                talent_points: 1,
                talent_wrath: 2,
                talent_vigor: 1,
                talent_focus: 3,
                dash_rune: DashRune::Reap,
                nova_rune: NovaRune::Frost,
                rupture_rune: RuptureRune::Hemorrhage,
            },
        };

        let content = ron::ser::to_string(&save).unwrap();
        let parsed: SaveGame = ron::from_str(&content).unwrap();
        assert_eq!(parsed, save);
        assert_eq!(parsed.modifier_affix, OrdealAffix::AshenHunger);
        assert_eq!(parsed.stats.journey_score, 32);
        assert_eq!(parsed.stats.chapter_clear_bonus_gold, 220);
        assert_eq!(parsed.stats.chapter_clear_bonus_shards, 7);
        assert_eq!(parsed.stats.chapter_clear_bonus_essence, 3);
        assert!(parsed.stats.chapter_clear_reward_claimed);
        assert_eq!(parsed.stats.journey_bonus_gold, 260);
        assert_eq!(parsed.stats.journey_bonus_shards, 8);
        assert_eq!(parsed.stats.journey_bonus_essence, 5);
        assert!(parsed.stats.journey_reward_claimed);
        assert_eq!(parsed.stats.renown_rank, 3);
        assert_eq!(parsed.stats.renown_bonus_gold, 105);
        assert_eq!(parsed.stats.renown_bonus_shards, 1);
        assert_eq!(parsed.stats.renown_bonus_essence, 0);
        assert!(parsed.stats.renown_bonus_claimed);
        assert_eq!(parsed.stats.stash_bonus_gold, 80);
        assert_eq!(parsed.stats.stash_bonus_shards, 3);
        assert_eq!(parsed.stats.stash_bonus_essence, 2);
        assert!(parsed.stats.stash_bonus_claimed);
        assert_eq!(parsed.stats.altar_seals, 5);
        assert_eq!(parsed.stats.altar_bonus_gold, 125);
        assert_eq!(parsed.stats.altar_bonus_shards, 2);
        assert_eq!(parsed.stats.altar_bonus_essence, 1);
        assert!(parsed.stats.altar_bonus_claimed);
        assert_eq!(parsed.stats.treasure_vaults_opened, 1);
        assert_eq!(parsed.stats.affix_codex_mask, 0b11_1111);
        assert!(parsed.stats.seal_warden_slain);
        assert_eq!(parsed.stats.malrec_soul_sigils, 2);
        assert_eq!(parsed.stats.malrec_soul_sigils_earned, 1);
        assert_eq!(parsed.stats.soul_sigil_caches, 1);
        assert_eq!(parsed.stats.armory_loadouts_saved, 3);
        assert_eq!(parsed.stats.town_portal_returns, 1);
        assert_eq!(parsed.stats.health_globes_collected, 3);
        assert_eq!(parsed.stats.last_stand_potions, 3);
        assert_eq!(parsed.stats.reliquary_momentum, 4);
        assert_eq!(parsed.stats.best_reliquary_momentum, 5);
        assert_eq!(parsed.bestiary["skeleton"].claimed_trophy_kills, 3);
        assert_eq!(parsed.player.chapter_boon, ChapterBoonChoice::CrimsonOath);
        assert!(parsed.player.chapter_boon_empowered);
        assert_eq!(parsed.player.chapter_boon_momentum_stacks, 3);
        assert!(parsed.player.armory_loadouts.is_empty());
        assert_eq!(parsed.player.armory_active_slot, 0);
    }

    #[test]
    fn old_saves_without_breakables_remain_valid() {
        let content = r#"(
            version: 1,
            stats: (
                kills: 0,
                gold: 0,
            ),
            player: (
                position: (0.0, 0.0, 0.0),
                health_current: 100.0,
                health_max: 100.0,
                level: 1,
                xp: 0,
                next_level_xp: 60,
                damage_bonus: 0.0,
                equipment: (
                    weapon_name: "Initiate Blade",
                    quality: "common",
                    crit_chance: 0.03,
                    health_bonus: 0.0,
                ),
                inventory: [],
                inventory_capacity: 12,
                potions: 2,
                max_potions: 5,
            ),
        )"#;

        let parsed: SaveGame = ron::from_str(content).unwrap();

        assert!(parsed.breakables.is_none());
        assert_eq!(parsed.modifier, ChapterModifierKind::AshenEchoes);
        assert_eq!(parsed.modifier_affix, OrdealAffix::None);
        assert!(parsed.player.armory_loadout.is_none());
        assert!(parsed.player.armory_loadouts.is_empty());
        assert_eq!(parsed.player.armory_active_slot, 0);
        assert!(parsed.player.legendary_codex.is_empty());
        assert_eq!(parsed.player.legendary_codex_attuned, LegendaryPower::None);
        assert_eq!(parsed.bounty, BountyProgress::default());
        assert!(parsed.challenges.is_empty());
        assert_eq!(parsed.mastery, ChapterMastery::default());
        assert!(parsed.bestiary.is_empty());
        assert_eq!(parsed.obelisk, BloodObelisk::default());
        assert_eq!(parsed.rift, EmberRift::default());
        assert!(parsed.story.is_empty());
        assert_eq!(parsed.stats.ember_shards, 0);
        assert_eq!(parsed.stats.affix_essence, 0);
        assert_eq!(parsed.stats.valor_stacks, 0);
        assert_eq!(parsed.stats.best_valor_stacks, 0);
        assert_eq!(parsed.stats.valor_timer_secs, 0.0);
        assert_eq!(parsed.stats.completion_bonus_shards, 0);
        assert_eq!(parsed.stats.completion_bonus_essence, 0);
        assert_eq!(parsed.stats.chapter_clear_bonus_gold, 0);
        assert_eq!(parsed.stats.chapter_clear_bonus_shards, 0);
        assert_eq!(parsed.stats.chapter_clear_bonus_essence, 0);
        assert!(!parsed.stats.chapter_clear_reward_claimed);
        assert_eq!(parsed.stats.journey_score, 0);
        assert_eq!(parsed.stats.journey_bonus_gold, 0);
        assert_eq!(parsed.stats.journey_bonus_shards, 0);
        assert_eq!(parsed.stats.journey_bonus_essence, 0);
        assert!(!parsed.stats.journey_reward_claimed);
        assert_eq!(parsed.stats.renown_rank, 0);
        assert_eq!(parsed.stats.renown_bonus_gold, 0);
        assert_eq!(parsed.stats.renown_bonus_shards, 0);
        assert_eq!(parsed.stats.renown_bonus_essence, 0);
        assert!(!parsed.stats.renown_bonus_claimed);
        assert_eq!(parsed.stats.stash_bonus_gold, 0);
        assert_eq!(parsed.stats.stash_bonus_shards, 0);
        assert_eq!(parsed.stats.stash_bonus_essence, 0);
        assert!(!parsed.stats.stash_bonus_claimed);
        assert_eq!(parsed.stats.altar_seals, 0);
        assert_eq!(parsed.stats.altar_bonus_gold, 0);
        assert_eq!(parsed.stats.altar_bonus_shards, 0);
        assert_eq!(parsed.stats.altar_bonus_essence, 0);
        assert!(!parsed.stats.altar_bonus_claimed);
        assert_eq!(parsed.stats.primal_caches, 0);
        assert_eq!(parsed.stats.primal_cache_bonus_gold, 0);
        assert_eq!(parsed.stats.primal_cache_bonus_shards, 0);
        assert_eq!(parsed.stats.primal_cache_bonus_essence, 0);
        assert_eq!(parsed.stats.primal_cache_items_claimed, 0);
        assert_eq!(parsed.stats.echo_keystones, 0);
        assert_eq!(parsed.stats.primal_cache_echo_items, 0);
        assert_eq!(parsed.stats.malrec_soul_sigils, 0);
        assert_eq!(parsed.stats.malrec_soul_sigils_earned, 0);
        assert_eq!(parsed.stats.soul_sigil_caches, 0);
        assert_eq!(parsed.stats.ancient_augments, 0);
        assert_eq!(parsed.stats.primal_infusions, 0);
        assert_eq!(parsed.stats.potions_used, 0);
        assert!(!parsed.stats.boss_enraged);
        assert_eq!(parsed.stats.boss_staggers, 0);
        assert!(!parsed.stats.shrine_resonance_triggered);
        assert_eq!(parsed.stats.elite_affix_kills, 0);
        assert_eq!(parsed.stats.affix_codex_mask, 0);
        assert!(!parsed.stats.seal_warden_slain);
        assert_eq!(parsed.stats.cursed_ambush_kills, 0);
        assert_eq!(parsed.stats.champion_pack_kills, 0);
        assert!(!parsed.stats.champion_pack_reward_claimed);
        assert_eq!(parsed.stats.treasure_vaults_opened, 0);
        assert_eq!(parsed.stats.health_globes_collected, 0);
        assert_eq!(parsed.stats.reap_dash_hits, 0);
        assert_eq!(parsed.stats.hemorrhage_rupture_hits, 0);
        assert_eq!(parsed.stats.frost_nova_hits, 0);
        assert_eq!(parsed.stats.armory_loadouts_saved, 0);
        assert_eq!(parsed.stats.town_portal_returns, 0);
        assert_eq!(parsed.stats.breakables_smashed, 0);
        assert_eq!(parsed.stats.surge_kills, 0);
        assert_eq!(parsed.stats.pylon_kills, 0);
        assert_eq!(parsed.stats.salvage_progress, 0);
        assert_eq!(parsed.stats.salvage_caches, 0);
        assert!(!parsed.stats.iron_elixir_used);
        assert!(!parsed.stats.wrath_elixir_used);
        assert!(!parsed.stats.haste_elixir_used);
        assert_eq!(parsed.stats.ashen_threat, 0);
        assert_eq!(parsed.stats.ashen_threat_surges, 0);
        assert_eq!(parsed.player.relic_buff_secs, 0.0);
        assert_eq!(
            parsed.player.relic_damage_multiplier,
            default_relic_damage_multiplier()
        );
        assert_eq!(
            parsed.player.relic_speed_multiplier,
            default_relic_speed_multiplier()
        );
        assert_eq!(parsed.player.fury_max, default_fury_max());
        assert_eq!(parsed.player.fury_basic_gain, default_fury_basic_gain());
        assert_eq!(parsed.player.dash_cost, default_dash_cost());
        assert_eq!(parsed.player.nova_cost, default_nova_cost());
        assert_eq!(parsed.player.rupture_cost, default_rupture_cost());
        assert_eq!(parsed.player.barrier_current, 0.0);
        assert_eq!(parsed.player.barrier_max, default_barrier_max());
        assert_eq!(parsed.player.paragon_rank, 0);
        assert_eq!(parsed.player.paragon_xp, 0);
        assert_eq!(
            parsed.player.paragon_next_rank_xp,
            default_paragon_next_rank_xp()
        );
        assert_eq!(parsed.player.conduit_buff_secs, 0.0);
        assert_eq!(parsed.player.glory_buff_secs, 0.0);
        assert_eq!(parsed.player.glory_stacks, 0);
        assert_eq!(parsed.player.surge_buff_secs, 0.0);
        assert_eq!(parsed.player.surge_charge, 0);
        assert_eq!(parsed.player.surge_max_charge, default_surge_max_charge());
        assert_eq!(
            parsed.player.surge_duration_secs,
            default_surge_duration_secs()
        );
        assert_eq!(parsed.player.potion_cooldown_secs, 0.0);
        assert_eq!(parsed.player.potion_cooldown_remaining_secs, 0.0);
        assert_eq!(parsed.player.selected_elixir, ElixirKind::Iron);
        assert_eq!(parsed.player.active_elixir, ElixirKind::Iron);
        assert_eq!(parsed.player.death_checkpoint, default_death_checkpoint());
        assert_eq!(parsed.player.revives_used, 0);
        assert_eq!(parsed.player.max_revives, default_max_revives());
        assert_eq!(
            parsed.player.revive_gold_penalty,
            default_revive_gold_penalty()
        );
        assert_eq!(parsed.player.town_portal_cooldown_secs, 0.0);
        assert_eq!(parsed.player.town_portal_return_position, None);
        assert_eq!(parsed.player.evade_active_secs, 0.0);
        assert_eq!(parsed.player.evade_cooldown_secs, 0.0);
        assert_eq!(parsed.player.evade_cooldown_remaining_secs, 0.0);
        assert_eq!(parsed.player.companion_rank, default_companion_rank());
        assert_eq!(parsed.player.companion_xp, 0);
        assert_eq!(
            parsed.player.companion_next_rank_xp,
            default_companion_next_rank_xp()
        );
        assert_eq!(parsed.player.companion_stance, default_companion_stance());
        assert_eq!(
            parsed.player.companion_command_cooldown_secs,
            default_companion_command_cooldown()
        );
        assert_eq!(parsed.player.chapter_boon, ChapterBoonChoice::None);
        assert!(!parsed.player.chapter_boon_empowered);
        assert_eq!(parsed.player.chapter_boon_momentum_stacks, 0);
        assert_eq!(parsed.player.rupture_rune, RuptureRune::Expose);
        assert!(parsed.player.armory_loadout.is_none());
    }

    fn save_player_with_armory(
        legacy_loadout: Option<ArmoryLoadout>,
        loadouts: Vec<Option<ArmoryLoadout>>,
        active_slot: usize,
    ) -> SavePlayer {
        SavePlayer {
            position: [0.0, 0.0, 0.0],
            health_current: 100.0,
            health_max: 100.0,
            barrier_current: 0.0,
            barrier_max: default_barrier_max(),
            level: 1,
            xp: 0,
            next_level_xp: 60,
            paragon_rank: 0,
            paragon_xp: 0,
            paragon_next_rank_xp: default_paragon_next_rank_xp(),
            damage_bonus: 0.0,
            fury_current: 0.0,
            fury_max: default_fury_max(),
            fury_basic_gain: default_fury_basic_gain(),
            dash_cost: default_dash_cost(),
            nova_cost: default_nova_cost(),
            rupture_cost: default_rupture_cost(),
            equipment: SaveEquipment {
                weapon_name: "Initiate Blade".to_string(),
                quality: "common".to_string(),
                crit_chance: 0.03,
                health_bonus: 0.0,
                armor_bonus: 0.0,
                legendary_power: LegendaryPower::None,
                temper_level: 0,
                socketed_gem: None,
            },
            charm: Charm::default(),
            inventory: vec![],
            inventory_capacity: 12,
            armory_loadout: legacy_loadout,
            armory_loadouts: loadouts,
            armory_active_slot: active_slot,
            legendary_codex: vec![],
            legendary_codex_attuned: LegendaryPower::None,
            death_checkpoint: default_death_checkpoint(),
            revives_used: 0,
            max_revives: default_max_revives(),
            revive_gold_penalty: default_revive_gold_penalty(),
            potions: 2,
            max_potions: 5,
            potion_cooldown_secs: 0.0,
            potion_cooldown_remaining_secs: 0.0,
            elixirs: 0,
            max_elixirs: 0,
            selected_elixir: ElixirKind::Iron,
            active_elixir: ElixirKind::Iron,
            elixir_buff_secs: 0.0,
            relic_buff_secs: 0.0,
            relic_damage_multiplier: default_relic_damage_multiplier(),
            relic_speed_multiplier: default_relic_speed_multiplier(),
            fortune_buff_secs: 0.0,
            conduit_buff_secs: 0.0,
            glory_buff_secs: 0.0,
            glory_stacks: 0,
            surge_buff_secs: 0.0,
            surge_charge: 0,
            surge_max_charge: default_surge_max_charge(),
            surge_duration_secs: default_surge_duration_secs(),
            town_portal_cooldown_secs: 0.0,
            town_portal_return_position: None,
            evade_active_secs: 0.0,
            evade_cooldown_secs: 0.0,
            evade_cooldown_remaining_secs: 0.0,
            companion_rank: default_companion_rank(),
            companion_xp: 0,
            companion_next_rank_xp: default_companion_next_rank_xp(),
            companion_stance: default_companion_stance(),
            companion_command_cooldown_secs: default_companion_command_cooldown(),
            chapter_boon: ChapterBoonChoice::None,
            chapter_boon_empowered: false,
            chapter_boon_momentum_stacks: 0,
            talent_points: 0,
            talent_wrath: 0,
            talent_vigor: 0,
            talent_focus: 0,
            dash_rune: DashRune::Cleanse,
            nova_rune: NovaRune::Ember,
            rupture_rune: RuptureRune::Expose,
        }
    }

    #[test]
    fn armory_save_migrates_legacy_loadout_and_preserves_multiple_slots() {
        let legacy_loadout = ArmoryLoadout {
            weapon: InventoryItem {
                name: "Legacy Edge".to_string(),
                quality: "legendary".to_string(),
                damage_bonus: 11.0,
                crit_chance: 0.1,
                health_bonus: 4.0,
                armor_bonus: 2.0,
                legendary_power: LegendaryPower::Emberbrand,
                temper_level: 1,
                socketed_gem: None,
            },
            charm: Charm::default(),
            dash_rune: DashRune::Cleanse,
            nova_rune: NovaRune::Ember,
            rupture_rune: RuptureRune::Expose,
            codex_attunement: LegendaryPower::Emberbrand,
        };
        let legacy_player = save_player_with_armory(Some(legacy_loadout.clone()), vec![], 0);

        let migrated = restored_armory(&legacy_player);

        assert_eq!(migrated.loadouts.len(), ARMORY_SLOT_COUNT);
        assert_eq!(migrated.loadouts[0], Some(legacy_loadout.clone()));
        assert_eq!(migrated.active_slot, 0);

        let multi_player = save_player_with_armory(
            Some(legacy_loadout.clone()),
            vec![None, Some(legacy_loadout.clone()), None],
            1,
        );

        let restored = restored_armory(&multi_player);

        assert_eq!(restored.loadouts.len(), ARMORY_SLOT_COUNT);
        assert_eq!(restored.active_slot, 1);
        assert_eq!(restored.loadouts[0], Some(legacy_loadout.clone()));
        assert_eq!(restored.loadouts[1], Some(legacy_loadout));
    }

    #[test]
    fn legacy_chapter_saves_infer_reliquary_seal_progress() {
        let boss_save = SaveChapter {
            phase: ChapterPhase::Boss,
            objective: "Defeat Malrec, Keeper of Ash".to_string(),
            enemies_slain: 6,
            chest_opened: true,
            altar_activated: true,
            sanctum_wave_spawned: true,
            seal_fragments: 0,
            boss_spawned: true,
            boss_slain: false,
            last_kill: "Last slain: Cinder Acolyte at 7, -1".to_string(),
            interaction_hint: "Malrec has entered the reliquary".to_string(),
        };
        let mut chapter = ChapterProgress::default();

        apply_chapter_save(&boss_save, &mut chapter);

        assert_eq!(
            chapter.seal_fragments,
            crate::chapter::REQUIRED_SEAL_FRAGMENTS
        );

        let ritual_save = SaveChapter {
            phase: ChapterPhase::Ritual,
            boss_spawned: false,
            boss_slain: false,
            ..boss_save
        };
        let mut chapter = ChapterProgress::default();

        apply_chapter_save(&ritual_save, &mut chapter);

        assert_eq!(chapter.seal_fragments, 3);
    }

    #[test]
    fn codex_backfill_restores_powers_from_equipment_and_inventory() {
        let equipment = Equipment {
            weapon_name: "legendary Moonforged Cleaver".to_string(),
            quality: "legendary".to_string(),
            crit_chance: 0.08,
            health_bonus: 10.0,
            armor_bonus: 8.0,
            legendary_power: LegendaryPower::Emberbrand,
            temper_level: 2,
            socketed_gem: None,
        };
        let inventory = Inventory {
            items: vec![InventoryItem {
                name: "legendary Frostwake Edge".to_string(),
                quality: "legendary".to_string(),
                damage_bonus: 12.0,
                crit_chance: 0.10,
                health_bonus: 12.0,
                armor_bonus: 9.0,
                legendary_power: LegendaryPower::Frostbrand,
                temper_level: 0,
                socketed_gem: None,
            }],
            capacity: 12,
        };
        let mut codex = LegendaryCodex::default();

        backfill_codex_from_items(&mut codex, &equipment, &inventory);

        assert_eq!(
            codex.unlocked,
            vec![LegendaryPower::Emberbrand, LegendaryPower::Frostbrand]
        );
    }
}
