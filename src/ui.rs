use crate::assets::{AssetLoadingProgress, GameAssets, game_assets_load_summary};
use crate::{
    AFFIX_CODEX_TARGET, ASHEN_THREAT_MAX, ASHEN_THREAT_SURGE_TARGET, AudioSettings,
    BOSS_BREAK_TARGET, BuildOpen, CARNAGE_MASTER_STREAK, CHAMPION_PACK_TARGET, CODEX_ADEPT_KILLS,
    Difficulty, DifficultySettings, GEM_ADEPT_KINDS, GLORY_SEEKER_GLOBES, GameState, InventoryOpen,
    JournalOpen, LAST_STAND_POTIONS, PauseState, RunStats, SET_ADEPT_KILLS,
    bestiary::{Bestiary, bestiary_next_goal, bestiary_summary},
    bounty::{BountyProgress, bounty_summary},
    challenge::{ChapterChallenges, challenge_summary},
    chapter::{
        ChapterPhase, ChapterProgress, InteractableKind, REQUIRED_SEAL_FRAGMENTS,
        main_quest_summary, phase_label, seal_fragment_reward_preview,
    },
    chapter_rating,
    combat::CombatComboState,
    companion::{CompanionBond, CompanionStance, companion_summary},
    dungeon::{BREAKER_TARGET_BREAKABLES, ChapterZoneState, Interactable},
    enemy::{
        AFFIX_HUNTER_KILLS, BossPhase, BossStagger, CURSED_AMBUSH_KILLS, Enemy, EnemyAffix,
        Staggered, boss_pressure_summary, boss_stagger_summary,
    },
    escalated_difficulty_after_clear,
    feedback::{AudioBackendStatus, CombatEvent, CombatLog, TargetInfo},
    format_run_time,
    journey::{
        award_chapter_journey_reward, chapter_journey_snapshot, journey_next_tier_reward_summary,
        journey_summary, journey_tier_for_score,
    },
    loot::{
        FuryGlobe, HealthGlobe, LOOT_PICKUP_RADIUS, LootEntity, LootProximityInfo,
        SOUL_SIGIL_CACHE_COST, loot_pursuit_summary, primal_infusion_pursuit_summary,
        quartermaster_preview_summary, soul_sigil_cache_ready_count,
        soul_sigil_cache_ready_summary,
    },
    lore::LoreJournal,
    massacre_summary,
    mastery::{ChapterMastery, mastery_next_goal_summary, mastery_summary},
    milestone::{ALL_MILESTONES, Milestones, milestone_summary},
    obelisk::{BloodObelisk, ObeliskState, obelisk_summary},
    ordeal::{ChapterModifier, OrdealAffix, modifier_for_difficulty, modifier_for_run},
    player::{
        Armory, Barrier, Burning, CHAPTER_LEVEL_CAP, ChapterBoon, ChapterBoonChoice, Charm,
        ConduitBuff, DamageBonus, DashRune, DeathWard, DeathWardReport, ElixirBelt, ElixirBuff,
        EmberParagon, Equipment, Evade, FortuneBuff, Fury, GearSlot, GloryBuff, Health, Inventory,
        InventoryItem, Jailed, LegendaryCodex, LegendaryPower, NovaRune, PYLON_REAPER_KILLS,
        Player, PlayerLevel, PotionBelt, RELIQUARY_MOMENTUM_MAX, RelicBuff, ReliquarySet,
        RuptureRune, SkillCooldowns, SkillRunes, SocketedGem, SurgeBuff, Talents, TownPortal,
        buff_seconds_remaining, chapter_boon_choice_prompt, conduit_seconds_remaining,
        elixir_seconds_remaining, equip_gear_piece, equip_inventory_item, evade_cooldown_remaining,
        evade_seconds_remaining, fortune_seconds_remaining, gem_label, glory_seconds_remaining,
        inventory_swap_index, legendary_codex_pursuit_summary, potion_seconds_remaining,
        reliquary_boon_momentum_ready, reliquary_momentum_summary, reliquary_resonance,
        surge_seconds_remaining, temper_label, themed_reliquary_set, total_armor,
        total_crit_chance, total_damage_bonus, town_portal_seconds_remaining, unequip_gear_piece,
    },
    rift::{EmberRift, RiftState, rift_summary},
    story::{StoryBeat, StoryLog, story_recap, story_summary},
    valor_summary,
};
use bevy::ecs::system::SystemParam;
use bevy::prelude::*;
use bevy::ui::UiGlobalTransform;
use bevy::window::PrimaryWindow;
use serde::{Deserialize, Serialize};
#[cfg(not(target_arch = "wasm32"))]
use std::fs;
use std::path::Path;

const PROFILE_PATH: &str = "saves/profile.ron";

#[derive(Component)]
struct MenuUi;

#[derive(Component)]
struct LoadingUi;

#[derive(Component)]
struct LoadingTitleText;

#[derive(Component)]
struct LoadingStatusText;

#[derive(Component)]
struct LoadingDetailText;

#[derive(Component)]
struct MenuDifficultyText;

#[derive(Component)]
struct MenuProfileText;

#[derive(Component)]
struct HudUi;

#[derive(Component)]
struct MinimapUi;

#[derive(Component)]
struct MinimapDot;

#[derive(Component)]
struct MinimapObjectiveText;

#[derive(Component)]
struct ObjectiveCompassPanel;

#[derive(Component, Debug, Clone, Copy, Eq, PartialEq)]
enum ObjectiveCompassText {
    Direction,
    Detail,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
enum MinimapInteractableRole {
    Primary,
    Vendor,
    Sustain,
    Shrine,
    Risk,
    Reward,
    Lore,
    Other,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
enum MinimapLegendRole {
    Player,
    Main,
    Loot,
    Life,
    Fury,
    Threat,
}

#[derive(Component)]
struct HealthText;

#[derive(Component, Debug, Clone, Copy, Eq, PartialEq)]
enum ResourceFill {
    Health,
    Barrier,
    Fury,
}

impl ResourceFill {
    fn fills_vertically(self) -> bool {
        matches!(self, Self::Health | Self::Fury)
    }
}

#[derive(Component, Debug, Clone, Copy, Eq, PartialEq)]
enum ResourceOrbFrame {
    Health,
    Fury,
}

#[derive(Component)]
struct CooldownText;

#[derive(Component, Debug, Clone, Copy, Eq, PartialEq)]
struct SkillSlotFrame(SkillSlotText);

#[derive(Component, Debug, Clone, Copy, Eq, PartialEq)]
struct SkillCooldownMask(SkillSlotText);

#[derive(Component, Debug, Clone, Copy, Eq, PartialEq)]
struct SkillSlotLamp(SkillSlotText);

#[derive(Component, Debug, Clone, Copy, Eq, PartialEq)]
enum SkillSlotText {
    Strike,
    Rupture,
    Dash,
    Nova,
    Surge,
    Evade,
}

impl SkillSlotText {
    fn key(self) -> &'static str {
        match self {
            Self::Strike => "LMB",
            Self::Rupture => "RMB",
            Self::Dash => "Q",
            Self::Nova => "E",
            Self::Surge => "Y",
            Self::Evade => "Shift",
        }
    }
}

#[derive(Component)]
struct StatsText;
#[derive(Component)]
struct ObjectiveText;
#[derive(Component)]
struct LogText;
#[derive(Component)]
struct PauseText;
#[derive(Component)]
struct InventoryText;

#[derive(Component)]
struct InventoryPanel;

#[derive(Component)]
struct BuildPanel;

#[derive(Component)]
struct BuildText;

#[derive(Component)]
struct ExperienceFill;

#[derive(Component, Debug, Clone, Copy, Eq, PartialEq)]
struct PotionPip(u32);

#[derive(Component)]
struct LevelUpBannerText;

#[derive(Component)]
struct LootPickupBannerText;

#[derive(Component)]
struct RewardToastText;

#[derive(Component)]
struct RewardToastPanel;

#[derive(Component)]
struct SkillFeedbackText;

#[derive(Component)]
struct ActionBeltText;

#[derive(Component)]
struct LootProximityPanel;

#[derive(Component)]
struct LootProximityText;

#[derive(Component)]
struct InteractProximityPanel;

#[derive(Component)]
struct InteractProximityText;

#[derive(Component)]
struct CombatStreakBannerText;

#[derive(Component)]
struct BuffBarText;

#[derive(Component)]
struct BossHudPanel;

#[derive(Component)]
struct BossHudFill;

#[derive(Component)]
struct BossHudStaggerFill;

#[derive(Component)]
struct BossHudText;

#[derive(Component)]
struct LowHealthVignette;

#[derive(Component)]
struct TargetInfoPanel;

#[derive(Component)]
struct TargetInfoFill;

#[derive(Component)]
struct TargetInfoText;

#[derive(Component, Debug, Clone, Copy, Eq, PartialEq)]
enum GearSlotText {
    Weapon,
    Charm,
    Codex,
    Armory,
}

/// True while the pointer is over (or pressing) an interactive UI node, so
/// world input (click-to-move/attack) ignores clicks meant for panels.
#[derive(Resource, Default, Debug, Clone, Copy)]
pub struct UiPointerCapture(pub bool);

/// One paper-doll gear box in the inventory panel (weapon + armor slots).
#[derive(Component, Debug, Clone, Copy, Eq, PartialEq)]
struct PaperDollSlot(GearSlot);

#[derive(Component, Debug, Clone, Copy, Eq, PartialEq)]
struct InventorySlotText(usize);

#[derive(Component, Debug, Clone, Copy, Eq, PartialEq)]
enum CompactCharacterText {
    Weapon,
    Charm,
    Inventory,
    Codex,
}

#[derive(Component, Debug, Clone, Copy, Eq, PartialEq)]
enum CompactUtilityText {
    Potion,
    Elixir,
    Portal,
    Interact,
}

#[derive(Component)]
struct JournalText;
#[derive(Component)]
struct ChapterBannerText;

#[derive(Component)]
struct StoryCommsText;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
struct ChapterRecord {
    elapsed_secs: f32,
    rating: String,
    kills: u32,
    gold: u32,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq, Serialize, Deserialize)]
enum SeasonDeedId {
    FirstClear,
    NightmareVeteran,
    TormentClear,
    BossBreaker,
    PrimalHunter,
    PrimalArtificer,
    AshboundVendetta,
    VaultSeeker,
    WritHunter,
    SetCollector,
    FlawlessChampion,
    UntouchedAscetic,
    BattleTranceVeteran,
    ChapterClearAdept,
    AscendantLapidary,
    ParagonLapidary,
    AffixArchivist,
    TrophyHoarder,
    OrdealConqueror,
    ReliquaryBanker,
    SoulboundResolve,
    BoonTriad,
    CodexKeeper,
    ConquerorPath,
    SigilEngraver,
}

impl SeasonDeedId {
    fn label(self) -> &'static str {
        match self {
            Self::FirstClear => "First Reliquary Clear",
            Self::NightmareVeteran => "Nightmare Veteran",
            Self::TormentClear => "Torment Proven",
            Self::BossBreaker => "Keeper Breaker",
            Self::PrimalHunter => "Primal Hunter",
            Self::PrimalArtificer => "Primal Artificer",
            Self::AshboundVendetta => "Ashbound Vendetta",
            Self::VaultSeeker => "Vault Seeker",
            Self::WritHunter => "Writ Hunter",
            Self::SetCollector => "Set Collector",
            Self::FlawlessChampion => "Flawless Champion",
            Self::UntouchedAscetic => "Untouched Ascetic",
            Self::BattleTranceVeteran => "Battle Trance Veteran",
            Self::ChapterClearAdept => "Chapter Clear Adept",
            Self::AscendantLapidary => "Ascendant Lapidary",
            Self::ParagonLapidary => "Paragon Lapidary",
            Self::AffixArchivist => "Affix Archivist",
            Self::TrophyHoarder => "Trophy Hoarder",
            Self::OrdealConqueror => "Ordeal Conqueror",
            Self::ReliquaryBanker => "Reliquary Banker",
            Self::SoulboundResolve => "Soulbound Resolve",
            Self::BoonTriad => "Boon Triad",
            Self::CodexKeeper => "Codex Keeper",
            Self::ConquerorPath => "Conqueror Path",
            Self::SigilEngraver => "Sigil Engraver",
        }
    }

    fn objective(self) -> &'static str {
        match self {
            Self::FirstClear => "clear the chapter once",
            Self::NightmareVeteran => "clear Nightmare or higher",
            Self::TormentClear => "clear Torment",
            Self::BossBreaker => "break Malrec ten times across clears",
            Self::PrimalHunter => "claim three Primal Ember Caches",
            Self::PrimalArtificer => "primal-infuse three ancient weapons",
            Self::AshboundVendetta => "slay three Ashbound Nemesis elites",
            Self::VaultSeeker => "open three Treasure Vaults",
            Self::WritHunter => "claim five Bounty Board caches",
            Self::SetCollector => "clear with all four themed set resonances",
            Self::FlawlessChampion => "clear three times without soul ward revival",
            Self::UntouchedAscetic => "clear three times without drinking a potion",
            Self::BattleTranceVeteran => "record a 20-kill massacre and five Valor stacks",
            Self::ChapterClearAdept => "earn two S-grade Chapter Clears",
            Self::AscendantLapidary => "clear twice with an Ascendant socketed gem",
            Self::ParagonLapidary => "clear twice with a Paragon socketed gem",
            Self::AffixArchivist => "complete Affix Codex in three clears",
            Self::TrophyHoarder => "open three Malrec Trophy Caches",
            Self::OrdealConqueror => "clear every rotating ordeal affix",
            Self::ReliquaryBanker => "bank 1000 gold, 20 shards, and 10 essence",
            Self::SoulboundResolve => "recover three Soul Remnants",
            Self::BoonTriad => "clear with all three Reliquary Boons",
            Self::CodexKeeper => "clear twice with a complete legendary codex",
            Self::ConquerorPath => "reach 60 Chapter Journey score",
            Self::SigilEngraver => "complete two perfect Reliquary Sigil inscriptions",
        }
    }

    fn reward(self) -> ProfileStashBonus {
        match self {
            Self::FirstClear => ProfileStashBonus {
                gold: 120,
                shards: 4,
                essence: 1,
            },
            Self::NightmareVeteran => ProfileStashBonus {
                gold: 180,
                shards: 6,
                essence: 2,
            },
            Self::TormentClear => ProfileStashBonus {
                gold: 300,
                shards: 10,
                essence: 5,
            },
            Self::BossBreaker => ProfileStashBonus {
                gold: 220,
                shards: 8,
                essence: 4,
            },
            Self::PrimalHunter => ProfileStashBonus {
                gold: 260,
                shards: 9,
                essence: 5,
            },
            Self::PrimalArtificer => ProfileStashBonus {
                gold: 320,
                shards: 10,
                essence: 8,
            },
            Self::AshboundVendetta => ProfileStashBonus {
                gold: 240,
                shards: 8,
                essence: 5,
            },
            Self::VaultSeeker => ProfileStashBonus {
                gold: 220,
                shards: 7,
                essence: 4,
            },
            Self::WritHunter => ProfileStashBonus {
                gold: 240,
                shards: 8,
                essence: 5,
            },
            Self::SetCollector => ProfileStashBonus {
                gold: 250,
                shards: 8,
                essence: 5,
            },
            Self::FlawlessChampion => ProfileStashBonus {
                gold: 280,
                shards: 9,
                essence: 6,
            },
            Self::UntouchedAscetic => ProfileStashBonus {
                gold: 260,
                shards: 8,
                essence: 6,
            },
            Self::BattleTranceVeteran => ProfileStashBonus {
                gold: 270,
                shards: 9,
                essence: 6,
            },
            Self::ChapterClearAdept => ProfileStashBonus {
                gold: 300,
                shards: 10,
                essence: 6,
            },
            Self::AscendantLapidary => ProfileStashBonus {
                gold: 250,
                shards: 8,
                essence: 5,
            },
            Self::ParagonLapidary => ProfileStashBonus {
                gold: 340,
                shards: 11,
                essence: 8,
            },
            Self::AffixArchivist => ProfileStashBonus {
                gold: 230,
                shards: 8,
                essence: 6,
            },
            Self::TrophyHoarder => ProfileStashBonus {
                gold: 260,
                shards: 9,
                essence: 6,
            },
            Self::OrdealConqueror => ProfileStashBonus {
                gold: 300,
                shards: 10,
                essence: 7,
            },
            Self::ReliquaryBanker => ProfileStashBonus {
                gold: 240,
                shards: 8,
                essence: 6,
            },
            Self::SoulboundResolve => ProfileStashBonus {
                gold: 200,
                shards: 7,
                essence: 3,
            },
            Self::BoonTriad => ProfileStashBonus {
                gold: 260,
                shards: 8,
                essence: 5,
            },
            Self::CodexKeeper => ProfileStashBonus {
                gold: 280,
                shards: 9,
                essence: 6,
            },
            Self::ConquerorPath => ProfileStashBonus {
                gold: 280,
                shards: 9,
                essence: 6,
            },
            Self::SigilEngraver => ProfileStashBonus {
                gold: 420,
                shards: 14,
                essence: 10,
            },
        }
    }
}

const ALL_SEASON_DEEDS: [SeasonDeedId; 25] = [
    SeasonDeedId::FirstClear,
    SeasonDeedId::NightmareVeteran,
    SeasonDeedId::TormentClear,
    SeasonDeedId::BossBreaker,
    SeasonDeedId::PrimalHunter,
    SeasonDeedId::PrimalArtificer,
    SeasonDeedId::AshboundVendetta,
    SeasonDeedId::VaultSeeker,
    SeasonDeedId::WritHunter,
    SeasonDeedId::SetCollector,
    SeasonDeedId::FlawlessChampion,
    SeasonDeedId::UntouchedAscetic,
    SeasonDeedId::BattleTranceVeteran,
    SeasonDeedId::ChapterClearAdept,
    SeasonDeedId::AscendantLapidary,
    SeasonDeedId::ParagonLapidary,
    SeasonDeedId::AffixArchivist,
    SeasonDeedId::TrophyHoarder,
    SeasonDeedId::OrdealConqueror,
    SeasonDeedId::ReliquaryBanker,
    SeasonDeedId::SoulboundResolve,
    SeasonDeedId::BoonTriad,
    SeasonDeedId::CodexKeeper,
    SeasonDeedId::ConquerorPath,
    SeasonDeedId::SigilEngraver,
];

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
enum ReliquaryAltarSealId {
    FirstEmber,
    TormentBrand,
    PrimalSpark,
    ParagonGem,
    CodexVow,
    BoonTriad,
    PerfectSigil,
    SeasonCrown,
}

impl ReliquaryAltarSealId {
    fn label(self) -> &'static str {
        match self {
            Self::FirstEmber => "First Ember",
            Self::TormentBrand => "Torment Brand",
            Self::PrimalSpark => "Primal Spark",
            Self::ParagonGem => "Paragon Gem",
            Self::CodexVow => "Codex Vow",
            Self::BoonTriad => "Boon Triad",
            Self::PerfectSigil => "Perfect Sigil",
            Self::SeasonCrown => "Season Crown",
        }
    }

    fn objective(self) -> &'static str {
        match self {
            Self::FirstEmber => "clear the chapter once",
            Self::TormentBrand => "clear Torment",
            Self::PrimalSpark => "claim a Primal Ember Cache",
            Self::ParagonGem => "clear with a Paragon socketed gem",
            Self::CodexVow => "clear with every legendary codex power unlocked",
            Self::BoonTriad => "clear with all three Reliquary Boons over time",
            Self::PerfectSigil => "engrave a perfect Reliquary Sigil",
            Self::SeasonCrown => "claim every Season Deed",
        }
    }
}

const ALL_RELIQUARY_ALTAR_SEALS: [ReliquaryAltarSealId; 8] = [
    ReliquaryAltarSealId::FirstEmber,
    ReliquaryAltarSealId::TormentBrand,
    ReliquaryAltarSealId::PrimalSpark,
    ReliquaryAltarSealId::ParagonGem,
    ReliquaryAltarSealId::CodexVow,
    ReliquaryAltarSealId::BoonTriad,
    ReliquaryAltarSealId::PerfectSigil,
    ReliquaryAltarSealId::SeasonCrown,
];

#[derive(Resource, Debug, Default, Clone, PartialEq, Serialize, Deserialize)]
pub(crate) struct ChapterRecords {
    normal: Option<ChapterRecord>,
    nightmare: Option<ChapterRecord>,
    hell: Option<ChapterRecord>,
    torment: Option<ChapterRecord>,
    #[serde(default)]
    normal_clears: u32,
    #[serde(default)]
    nightmare_clears: u32,
    #[serde(default)]
    hell_clears: u32,
    #[serde(default)]
    torment_clears: u32,
    #[serde(default)]
    lifetime_kills: u32,
    #[serde(default)]
    lifetime_gold: u32,
    #[serde(default)]
    lifetime_shards: u32,
    #[serde(default)]
    lifetime_essence: u32,
    #[serde(default)]
    best_rating: String,
    #[serde(default)]
    best_journey_score: u32,
    #[serde(default)]
    best_journey_tier: String,
    #[serde(default)]
    lifetime_boss_breaks: u32,
    #[serde(default)]
    lifetime_primal_caches: u32,
    #[serde(default)]
    lifetime_primal_infusions: u32,
    #[serde(default)]
    lifetime_echo_keystones: u32,
    #[serde(default)]
    lifetime_ashen_surges: u32,
    #[serde(default)]
    lifetime_nemesis_kills: u32,
    #[serde(default)]
    lifetime_treasure_vaults: u32,
    #[serde(default)]
    lifetime_bounties_completed: u32,
    #[serde(default)]
    completed_set_mask: u8,
    #[serde(default)]
    lifetime_flawless_clears: u32,
    #[serde(default)]
    lifetime_potionless_clears: u32,
    #[serde(default)]
    lifetime_affix_codexes: u32,
    #[serde(default)]
    best_chapter_clear_grade: String,
    #[serde(default)]
    lifetime_s_chapter_clears: u32,
    #[serde(default)]
    best_socketed_gem_rank: u32,
    #[serde(default)]
    lifetime_ascendant_gem_clears: u32,
    #[serde(default)]
    lifetime_paragon_gem_clears: u32,
    #[serde(default)]
    lifetime_soul_sigils: u32,
    #[serde(default)]
    lifetime_soul_sigil_caches: u32,
    #[serde(default)]
    cleared_ordeal_affix_mask: u8,
    #[serde(default)]
    best_massacre_streak: u32,
    #[serde(default)]
    best_valor_stacks: u32,
    #[serde(default)]
    best_boss_breaks: u32,
    #[serde(default)]
    stash_gold: u32,
    #[serde(default)]
    stash_shards: u32,
    #[serde(default)]
    stash_essence: u32,
    #[serde(default)]
    failed_runs: u32,
    #[serde(default)]
    soul_remnant_gold: u32,
    #[serde(default)]
    soul_remnant_shards: u32,
    #[serde(default)]
    soul_remnant_essence: u32,
    #[serde(default)]
    completed_boon_mask: u8,
    #[serde(default)]
    best_legendary_codex_powers: u32,
    #[serde(default)]
    lifetime_legendary_codex_completions: u32,
    #[serde(default)]
    best_reliquary_sigil_tier: u32,
    #[serde(default)]
    lifetime_reliquary_sigil_clears: u32,
    #[serde(default)]
    lifetime_perfect_sigil_clears: u32,
    #[serde(default)]
    season_deeds_claimed: Vec<SeasonDeedId>,
}

impl ChapterRecords {
    fn get(&self, difficulty: Difficulty) -> Option<ChapterRecord> {
        match difficulty {
            Difficulty::Normal => self.normal.clone(),
            Difficulty::Nightmare => self.nightmare.clone(),
            Difficulty::Hell => self.hell.clone(),
            Difficulty::Torment => self.torment.clone(),
        }
    }

    fn get_mut(&mut self, difficulty: Difficulty) -> &mut Option<ChapterRecord> {
        match difficulty {
            Difficulty::Normal => &mut self.normal,
            Difficulty::Nightmare => &mut self.nightmare,
            Difficulty::Hell => &mut self.hell,
            Difficulty::Torment => &mut self.torment,
        }
    }

    pub(crate) fn clears(&self, difficulty: Difficulty) -> u32 {
        match difficulty {
            Difficulty::Normal => self.normal_clears,
            Difficulty::Nightmare => self.nightmare_clears,
            Difficulty::Hell => self.hell_clears,
            Difficulty::Torment => self.torment_clears,
        }
    }

    fn has_clear(&self, difficulty: Difficulty) -> bool {
        self.clears(difficulty) > 0 || self.get(difficulty).is_some()
    }

    pub(crate) fn add_clear(&mut self, difficulty: Difficulty) {
        let clears = match difficulty {
            Difficulty::Normal => &mut self.normal_clears,
            Difficulty::Nightmare => &mut self.nightmare_clears,
            Difficulty::Hell => &mut self.hell_clears,
            Difficulty::Torment => &mut self.torment_clears,
        };
        *clears = clears.saturating_add(1);
    }

    fn total_clears(&self) -> u32 {
        self.normal_clears
            .saturating_add(self.nightmare_clears)
            .saturating_add(self.hell_clears)
            .saturating_add(self.torment_clears)
    }

    fn highest_cleared_difficulty(&self) -> Option<Difficulty> {
        [
            (Difficulty::Torment, self.torment_clears),
            (Difficulty::Hell, self.hell_clears),
            (Difficulty::Nightmare, self.nightmare_clears),
            (Difficulty::Normal, self.normal_clears),
        ]
        .into_iter()
        .find_map(|(difficulty, clears)| (clears > 0).then_some(difficulty))
    }

    fn best_rating_label(&self) -> &str {
        if self.best_rating.is_empty() {
            "None"
        } else {
            &self.best_rating
        }
    }

    fn register_lifetime_stats(&mut self, stats: &RunStats, rating: &str) {
        self.lifetime_kills = self.lifetime_kills.saturating_add(stats.kills);
        self.lifetime_gold = self.lifetime_gold.saturating_add(stats.gold);
        self.lifetime_shards = self.lifetime_shards.saturating_add(stats.ember_shards);
        self.lifetime_essence = self.lifetime_essence.saturating_add(stats.affix_essence);
        let deposit = profile_stash_deposit(stats);
        self.stash_gold = self.stash_gold.saturating_add(deposit.gold);
        self.stash_shards = self.stash_shards.saturating_add(deposit.shards);
        self.stash_essence = self.stash_essence.saturating_add(deposit.essence);
        self.lifetime_boss_breaks = self
            .lifetime_boss_breaks
            .saturating_add(stats.boss_staggers);
        self.lifetime_primal_caches = self
            .lifetime_primal_caches
            .saturating_add(stats.primal_caches);
        self.lifetime_primal_infusions = self
            .lifetime_primal_infusions
            .saturating_add(stats.primal_infusions);
        self.lifetime_echo_keystones = self
            .lifetime_echo_keystones
            .saturating_add(stats.echo_keystones);
        self.lifetime_ashen_surges = self
            .lifetime_ashen_surges
            .saturating_add(stats.ashen_threat_surges);
        self.lifetime_nemesis_kills = self
            .lifetime_nemesis_kills
            .saturating_add(stats.nemesis_kills);
        self.lifetime_treasure_vaults = self
            .lifetime_treasure_vaults
            .saturating_add(stats.treasure_vaults_opened);
        if affix_codex_complete_for_profile(stats) {
            self.lifetime_affix_codexes = self.lifetime_affix_codexes.saturating_add(1);
        }
        self.lifetime_soul_sigils = self
            .lifetime_soul_sigils
            .saturating_add(stats.malrec_soul_sigils_earned);
        self.lifetime_soul_sigil_caches = self
            .lifetime_soul_sigil_caches
            .saturating_add(stats.soul_sigil_caches);
        self.best_massacre_streak = self.best_massacre_streak.max(stats.best_massacre_streak);
        self.best_valor_stacks = self.best_valor_stacks.max(stats.best_valor_stacks);
        self.best_boss_breaks = self.best_boss_breaks.max(stats.boss_staggers);
        if rating_rank(rating) > rating_rank(self.best_rating_label()) {
            self.best_rating = rating.to_string();
        }
        if stats.journey_score > self.best_journey_score {
            self.best_journey_score = stats.journey_score;
            self.best_journey_tier = journey_tier_for_score(stats.journey_score)
                .label()
                .to_string();
        }
    }

    fn register_ordeal_affix_clear(&mut self, affix: OrdealAffix) {
        self.cleared_ordeal_affix_mask |= ordeal_affix_mask(affix);
    }

    fn register_bounty_cache_claimed(&mut self, claimed: bool) {
        if claimed {
            self.lifetime_bounties_completed = self.lifetime_bounties_completed.saturating_add(1);
        }
    }

    fn register_completed_set(&mut self, set: Option<ReliquarySet>) {
        if let Some(set) = set {
            self.completed_set_mask |= set.mask();
        }
    }

    fn register_flawless_clear(&mut self, flawless: bool) {
        if flawless {
            self.lifetime_flawless_clears = self.lifetime_flawless_clears.saturating_add(1);
        }
    }

    fn register_potionless_clear(&mut self, potionless: bool) {
        if potionless {
            self.lifetime_potionless_clears = self.lifetime_potionless_clears.saturating_add(1);
        }
    }

    fn register_chapter_clear_grade(&mut self, grade: &str) {
        if grade == "S" {
            self.lifetime_s_chapter_clears = self.lifetime_s_chapter_clears.saturating_add(1);
        }
        if clear_grade_rank(grade) > clear_grade_rank(self.best_chapter_clear_grade_label()) {
            self.best_chapter_clear_grade = grade.to_string();
        }
    }

    fn best_chapter_clear_grade_label(&self) -> &str {
        if self.best_chapter_clear_grade.is_empty() {
            "None"
        } else {
            &self.best_chapter_clear_grade
        }
    }

    fn register_socketed_gem_clear(&mut self, gem: Option<SocketedGem>) {
        let Some(gem) = gem else {
            return;
        };
        self.best_socketed_gem_rank = self.best_socketed_gem_rank.max(gem.rank);
        if gem.rank >= 5 {
            self.lifetime_ascendant_gem_clears =
                self.lifetime_ascendant_gem_clears.saturating_add(1);
        }
        if gem.rank >= 7 {
            self.lifetime_paragon_gem_clears = self.lifetime_paragon_gem_clears.saturating_add(1);
        }
    }

    fn register_completed_boon(&mut self, choice: ChapterBoonChoice) {
        self.completed_boon_mask |= chapter_boon_mask(choice);
    }

    fn register_legendary_codex_clear(&mut self, codex: &LegendaryCodex) {
        let unlocked = legendary_codex_power_count(codex);
        self.best_legendary_codex_powers = self.best_legendary_codex_powers.max(unlocked);
        if unlocked == LegendaryPower::ALL.len() as u32 {
            self.lifetime_legendary_codex_completions =
                self.lifetime_legendary_codex_completions.saturating_add(1);
        }
    }

    fn register_reliquary_sigil_clear(
        &mut self,
        difficulty: Difficulty,
        stats: &RunStats,
        clear_grade: &str,
        socketed_gem: Option<SocketedGem>,
        codex: Option<&LegendaryCodex>,
    ) -> Option<u32> {
        let tier = reliquary_sigil_tier(difficulty, stats, clear_grade, socketed_gem, codex);
        if tier == 0 {
            return None;
        }
        self.best_reliquary_sigil_tier = self.best_reliquary_sigil_tier.max(tier);
        self.lifetime_reliquary_sigil_clears =
            self.lifetime_reliquary_sigil_clears.saturating_add(1);
        if tier >= PERFECT_RELIQUARY_SIGIL_TIER {
            self.lifetime_perfect_sigil_clears =
                self.lifetime_perfect_sigil_clears.saturating_add(1);
        }
        Some(tier)
    }

    fn best_journey_label(&self) -> String {
        if self.best_journey_score == 0 {
            "None".to_string()
        } else if self.best_journey_tier.is_empty() {
            format!(
                "{} {}",
                journey_tier_for_score(self.best_journey_score).label(),
                self.best_journey_score
            )
        } else {
            format!("{} {}", self.best_journey_tier, self.best_journey_score)
        }
    }

    fn renown_rank(&self) -> u32 {
        renown_rank_for_score(self.renown_score())
    }

    fn renown_score(&self) -> u32 {
        self.total_clears()
            .saturating_mul(5)
            .saturating_add(self.clears(Difficulty::Torment).saturating_mul(10))
            .saturating_add(self.best_journey_score / 4)
            .saturating_add(self.lifetime_kills / 25)
            .saturating_add(self.lifetime_gold / 1_000)
            .saturating_add(self.lifetime_shards / 20)
            .saturating_add(self.lifetime_essence / 10)
            .saturating_add(self.lifetime_boss_breaks)
            .saturating_add(self.lifetime_primal_caches.saturating_mul(4))
            .saturating_add(self.lifetime_primal_infusions.saturating_mul(3))
            .saturating_add(self.lifetime_echo_keystones.saturating_mul(2))
            .saturating_add(self.lifetime_ashen_surges.saturating_mul(2))
            .saturating_add(self.lifetime_nemesis_kills.saturating_mul(2))
            .saturating_add(self.lifetime_bounties_completed.saturating_mul(2))
            .saturating_add(self.completed_set_mask.count_ones().saturating_mul(3))
            .saturating_add(self.lifetime_flawless_clears.saturating_mul(3))
            .saturating_add(self.lifetime_potionless_clears.saturating_mul(2))
            .saturating_add(self.lifetime_affix_codexes.saturating_mul(2))
            .saturating_add(self.lifetime_s_chapter_clears.saturating_mul(3))
            .saturating_add(clear_grade_rank(self.best_chapter_clear_grade_label()))
            .saturating_add(self.best_socketed_gem_rank)
            .saturating_add(self.lifetime_ascendant_gem_clears.saturating_mul(2))
            .saturating_add(self.lifetime_paragon_gem_clears.saturating_mul(3))
            .saturating_add(self.lifetime_soul_sigils)
            .saturating_add(self.lifetime_soul_sigil_caches.saturating_mul(3))
            .saturating_add(
                self.cleared_ordeal_affix_mask
                    .count_ones()
                    .saturating_mul(2),
            )
            .saturating_add(self.best_massacre_streak / 10)
            .saturating_add(self.best_valor_stacks)
            .saturating_add(self.best_boss_breaks.saturating_mul(2))
            .saturating_add(self.completed_boon_mask.count_ones().saturating_mul(2))
            .saturating_add(self.best_legendary_codex_powers)
            .saturating_add(self.lifetime_legendary_codex_completions.saturating_mul(3))
            .saturating_add(self.best_reliquary_sigil_tier)
            .saturating_add(self.lifetime_reliquary_sigil_clears.saturating_mul(2))
            .saturating_add(self.lifetime_perfect_sigil_clears.saturating_mul(5))
            .saturating_add(self.failed_runs)
            .saturating_add(self.soul_remnant_shards / 3)
            .saturating_add(self.soul_remnant_essence)
            .saturating_add(self.stash_gold / 1_500)
            .saturating_add(self.stash_shards / 15)
            .saturating_add(self.stash_essence / 8)
    }

    fn completed_season_deeds(&self) -> usize {
        ALL_SEASON_DEEDS
            .iter()
            .filter(|deed| season_deed_complete(**deed, self))
            .count()
    }
}

const PERFECT_RELIQUARY_SIGIL_TIER: u32 = 10;

fn reliquary_sigil_tier(
    difficulty: Difficulty,
    stats: &RunStats,
    clear_grade: &str,
    socketed_gem: Option<SocketedGem>,
    codex: Option<&LegendaryCodex>,
) -> u32 {
    if !matches!(difficulty, Difficulty::Torment) {
        return 0;
    }

    let mut tier: u32 = 1;
    tier = tier.saturating_add(match clear_grade {
        "S" => 3,
        "A" => 2,
        "B" => 1,
        _ => 0,
    });
    tier = tier.saturating_add((stats.boss_staggers / 2).min(2));
    tier = tier.saturating_add(u32::from(stats.primal_caches > 0));
    tier = tier.saturating_add(u32::from(stats.ashen_threat_surges > 0));
    tier = tier.saturating_add(u32::from(stats.best_massacre_streak >= 20));
    tier = tier.saturating_add(u32::from(stats.best_valor_stacks >= 5));
    if let Some(gem) = socketed_gem {
        tier = tier.saturating_add(if gem.rank >= 7 {
            2
        } else if gem.rank >= 5 {
            1
        } else {
            0
        });
    }
    if let Some(codex) = codex {
        let powers = legendary_codex_power_count(codex);
        tier = tier.saturating_add(if powers == LegendaryPower::ALL.len() as u32 {
            2
        } else if powers >= 3 {
            1
        } else {
            0
        });
    }
    tier.min(12)
}

pub(crate) fn next_unlocked_difficulty(
    current: Difficulty,
    records: &ChapterRecords,
) -> Difficulty {
    let mut candidate = current.next();
    for _ in 0..4 {
        if difficulty_unlocked(candidate, records) {
            return candidate;
        }
        candidate = candidate.next();
    }
    Difficulty::Normal
}

fn difficulty_unlocked(difficulty: Difficulty, records: &ChapterRecords) -> bool {
    match difficulty {
        Difficulty::Normal => true,
        Difficulty::Nightmare => {
            records.has_clear(Difficulty::Normal)
                || records.has_clear(Difficulty::Nightmare)
                || records.has_clear(Difficulty::Hell)
                || records.has_clear(Difficulty::Torment)
        }
        Difficulty::Hell => {
            records.has_clear(Difficulty::Nightmare)
                || records.has_clear(Difficulty::Hell)
                || records.has_clear(Difficulty::Torment)
        }
        Difficulty::Torment => {
            records.has_clear(Difficulty::Hell) || records.has_clear(Difficulty::Torment)
        }
    }
}

fn difficulty_unlock_summary(difficulty: Difficulty, records: &ChapterRecords) -> String {
    if difficulty_unlocked(difficulty, records) {
        return "Unlocked".to_string();
    }
    let required = match difficulty {
        Difficulty::Normal => "available",
        Difficulty::Nightmare => "clear Normal",
        Difficulty::Hell => "clear Nightmare",
        Difficulty::Torment => "clear Hell",
    };
    format!("Locked: {required}")
}

#[derive(Resource)]
struct ChapterBannerState {
    last_phase: Option<ChapterPhase>,
    timer: Timer,
}

impl Default for ChapterBannerState {
    fn default() -> Self {
        Self {
            last_phase: None,
            timer: Timer::from_seconds(0.0, TimerMode::Once),
        }
    }
}

#[derive(Resource)]
struct LevelUpBannerState {
    observed_level: u32,
    observed_talent_points: u32,
    timer: Timer,
}

impl Default for LevelUpBannerState {
    fn default() -> Self {
        Self {
            observed_level: 0,
            observed_talent_points: 0,
            timer: Timer::from_seconds(0.0, TimerMode::Once),
        }
    }
}

#[derive(Resource)]
struct LootPickupBannerState {
    timer: Timer,
}

impl Default for LootPickupBannerState {
    fn default() -> Self {
        Self {
            timer: Timer::from_seconds(0.0, TimerMode::Once),
        }
    }
}

#[derive(Resource)]
struct RewardToastState {
    timer: Timer,
}

impl Default for RewardToastState {
    fn default() -> Self {
        Self {
            timer: Timer::from_seconds(0.0, TimerMode::Once),
        }
    }
}

#[derive(Resource)]
struct StoryCommsState {
    timer: Timer,
}

impl Default for StoryCommsState {
    fn default() -> Self {
        Self {
            timer: Timer::from_seconds(0.0, TimerMode::Once),
        }
    }
}

#[derive(Resource)]
struct SkillFeedbackState {
    timer: Timer,
    pulse: Option<SkillSlotPulse>,
}

impl Default for SkillFeedbackState {
    fn default() -> Self {
        Self {
            timer: Timer::from_seconds(0.0, TimerMode::Once),
            pulse: None,
        }
    }
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
struct SkillSlotPulse {
    slot: SkillSlotText,
    kind: SkillSlotPulseKind,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
enum SkillSlotPulseKind {
    Cast,
    Blocked,
    Survival,
}

#[derive(Resource)]
struct CombatStreakBannerState {
    timer: Timer,
}

impl Default for CombatStreakBannerState {
    fn default() -> Self {
        Self {
            timer: Timer::from_seconds(0.0, TimerMode::Once),
        }
    }
}

#[derive(Resource, Debug, Clone, Copy, Eq, PartialEq, Default)]
enum HudDensity {
    #[default]
    Clean,
    Tactical,
}

impl HudDensity {
    fn toggle(&mut self) -> Self {
        *self = match *self {
            Self::Clean => Self::Tactical,
            Self::Tactical => Self::Clean,
        };
        *self
    }

    fn label(self) -> &'static str {
        match self {
            Self::Clean => "clean",
            Self::Tactical => "tactical",
        }
    }

    fn is_clean(self) -> bool {
        matches!(self, Self::Clean)
    }
}

type ObjectiveTextQuery<'w, 's> = Query<
    'w,
    's,
    &'static mut Text,
    (
        With<ObjectiveText>,
        Without<HealthText>,
        Without<CooldownText>,
        Without<StatsText>,
        Without<LogText>,
        Without<PauseText>,
        Without<InventoryText>,
        Without<GearSlotText>,
        Without<InventorySlotText>,
        Without<JournalText>,
        Without<BuildText>,
    ),
>;
type HealthTextQuery<'w, 's> = Query<
    'w,
    's,
    &'static mut Text,
    (
        With<HealthText>,
        Without<ObjectiveText>,
        Without<CooldownText>,
        Without<StatsText>,
        Without<LogText>,
        Without<PauseText>,
        Without<InventoryText>,
        Without<GearSlotText>,
        Without<InventorySlotText>,
        Without<JournalText>,
        Without<BuildText>,
    ),
>;
type CooldownTextQuery<'w, 's> = Query<
    'w,
    's,
    (&'static SkillSlotText, &'static mut Text),
    (
        With<CooldownText>,
        Without<ObjectiveText>,
        Without<HealthText>,
        Without<StatsText>,
        Without<LogText>,
        Without<PauseText>,
        Without<InventoryText>,
        Without<GearSlotText>,
        Without<InventorySlotText>,
        Without<JournalText>,
        Without<BuildText>,
    ),
>;
type StatsTextQuery<'w, 's> = Query<
    'w,
    's,
    &'static mut Text,
    (
        With<StatsText>,
        Without<ObjectiveText>,
        Without<HealthText>,
        Without<CooldownText>,
        Without<LogText>,
        Without<PauseText>,
        Without<InventoryText>,
        Without<GearSlotText>,
        Without<InventorySlotText>,
        Without<JournalText>,
        Without<BuildText>,
    ),
>;
type LogTextQuery<'w, 's> = Query<
    'w,
    's,
    &'static mut Text,
    (
        With<LogText>,
        Without<ObjectiveText>,
        Without<HealthText>,
        Without<CooldownText>,
        Without<StatsText>,
        Without<PauseText>,
        Without<InventoryText>,
        Without<GearSlotText>,
        Without<InventorySlotText>,
        Without<JournalText>,
        Without<BuildText>,
    ),
>;
type PauseTextQuery<'w, 's> = Query<
    'w,
    's,
    &'static mut Text,
    (
        With<PauseText>,
        Without<ObjectiveText>,
        Without<HealthText>,
        Without<CooldownText>,
        Without<StatsText>,
        Without<LogText>,
        Without<InventoryText>,
        Without<GearSlotText>,
        Without<InventorySlotText>,
        Without<JournalText>,
        Without<BuildText>,
    ),
>;
type InventoryTextQuery<'w, 's> = Query<
    'w,
    's,
    &'static mut Text,
    (
        With<InventoryText>,
        Without<ObjectiveText>,
        Without<HealthText>,
        Without<CooldownText>,
        Without<StatsText>,
        Without<LogText>,
        Without<PauseText>,
        Without<GearSlotText>,
        Without<InventorySlotText>,
        Without<JournalText>,
        Without<BuildText>,
    ),
>;
type GearSlotTextQuery<'w, 's> = Query<
    'w,
    's,
    (&'static GearSlotText, &'static mut Text),
    (
        Without<ObjectiveText>,
        Without<HealthText>,
        Without<CooldownText>,
        Without<StatsText>,
        Without<LogText>,
        Without<PauseText>,
        Without<InventoryText>,
        Without<InventorySlotText>,
        Without<JournalText>,
        Without<BuildText>,
    ),
>;
type InventorySlotTextQuery<'w, 's> = Query<
    'w,
    's,
    (&'static InventorySlotText, &'static mut Text),
    (
        Without<ObjectiveText>,
        Without<HealthText>,
        Without<CooldownText>,
        Without<StatsText>,
        Without<LogText>,
        Without<PauseText>,
        Without<InventoryText>,
        Without<GearSlotText>,
        Without<JournalText>,
        Without<BuildText>,
    ),
>;
type GearSlotFrameQuery<'w, 's> = Query<
    'w,
    's,
    (
        &'static GearSlotText,
        &'static mut BorderColor,
        &'static mut BackgroundColor,
    ),
    (
        Without<ResourceFill>,
        Without<SkillCooldownMask>,
        Without<InventorySlotText>,
        Without<SkillSlotFrame>,
        Without<PotionPip>,
        Without<ObjectiveText>,
    ),
>;
type InventorySlotFrameQuery<'w, 's> = Query<
    'w,
    's,
    (
        &'static InventorySlotText,
        &'static mut BorderColor,
        &'static mut BackgroundColor,
    ),
    (
        Without<ResourceFill>,
        Without<SkillCooldownMask>,
        Without<GearSlotText>,
        Without<SkillSlotFrame>,
        Without<PotionPip>,
        Without<ObjectiveText>,
    ),
>;
type JournalTextQuery<'w, 's> = Query<
    'w,
    's,
    &'static mut Text,
    (
        With<JournalText>,
        Without<ObjectiveText>,
        Without<HealthText>,
        Without<CooldownText>,
        Without<StatsText>,
        Without<LogText>,
        Without<PauseText>,
        Without<InventoryText>,
        Without<GearSlotText>,
        Without<InventorySlotText>,
        Without<BuildText>,
    ),
>;
type HudTextQueries<'w, 's> = (
    ObjectiveTextQuery<'w, 's>,
    HealthTextQuery<'w, 's>,
    CooldownTextQuery<'w, 's>,
    StatsTextQuery<'w, 's>,
    LogTextQuery<'w, 's>,
    PauseTextQuery<'w, 's>,
    InventoryTextQuery<'w, 's>,
    GearSlotTextQuery<'w, 's>,
    InventorySlotTextQuery<'w, 's>,
    JournalTextQuery<'w, 's>,
    BuildTextQuery<'w, 's>,
);
type ResourceFillQuery<'w, 's> = Query<
    'w,
    's,
    (
        &'static ResourceFill,
        &'static mut Node,
        &'static mut BackgroundColor,
    ),
    (
        Without<SkillSlotFrame>,
        Without<SkillCooldownMask>,
        Without<InventoryPanel>,
        Without<BuildPanel>,
        Without<ExperienceFill>,
        Without<PotionPip>,
        Without<ObjectiveText>,
        Without<GearSlotText>,
        Without<InventorySlotText>,
        Without<LogText>,
        Without<JournalText>,
    ),
>;
type ResourceOrbFrameQuery<'w, 's> = Query<
    'w,
    's,
    (
        &'static ResourceOrbFrame,
        &'static mut BorderColor,
        &'static mut BackgroundColor,
    ),
    (
        Without<ResourceFill>,
        Without<SkillSlotFrame>,
        Without<SkillCooldownMask>,
        Without<SkillSlotLamp>,
        Without<InventoryPanel>,
        Without<BuildPanel>,
        Without<ExperienceFill>,
        Without<PotionPip>,
        Without<ObjectiveText>,
        Without<GearSlotText>,
        Without<InventorySlotText>,
        Without<LogText>,
        Without<JournalText>,
    ),
>;
type SkillSlotFrameQuery<'w, 's> = Query<
    'w,
    's,
    (
        &'static SkillSlotFrame,
        &'static mut BorderColor,
        &'static mut BackgroundColor,
    ),
    (
        Without<ResourceFill>,
        Without<SkillCooldownMask>,
        Without<ExperienceFill>,
        Without<PotionPip>,
        Without<ObjectiveText>,
        Without<InventorySlotText>,
        Without<GearSlotText>,
    ),
>;
type SkillCooldownMaskQuery<'w, 's> = Query<
    'w,
    's,
    (
        &'static SkillCooldownMask,
        &'static mut Node,
        &'static mut BackgroundColor,
    ),
    (
        Without<ResourceFill>,
        Without<InventoryPanel>,
        Without<BuildPanel>,
        Without<ExperienceFill>,
        Without<PotionPip>,
        Without<SkillSlotFrame>,
        Without<ObjectiveText>,
        Without<InventorySlotText>,
        Without<GearSlotText>,
    ),
>;
type SkillSlotLampQuery<'w, 's> = Query<
    'w,
    's,
    (
        &'static SkillSlotLamp,
        &'static mut Node,
        &'static mut BackgroundColor,
    ),
    (
        Without<ResourceFill>,
        Without<InventoryPanel>,
        Without<BuildPanel>,
        Without<ExperienceFill>,
        Without<PotionPip>,
        Without<SkillSlotFrame>,
        Without<SkillCooldownMask>,
        Without<ObjectiveText>,
        Without<InventorySlotText>,
        Without<GearSlotText>,
    ),
>;
type InventoryPanelQuery<'w, 's> = Query<
    'w,
    's,
    &'static mut Node,
    (
        With<InventoryPanel>,
        Without<ResourceFill>,
        Without<BuildPanel>,
        Without<ExperienceFill>,
        Without<PotionPip>,
        Without<ObjectiveText>,
    ),
>;
type BuildPanelQuery<'w, 's> = Query<
    'w,
    's,
    &'static mut Node,
    (
        With<BuildPanel>,
        Without<ResourceFill>,
        Without<InventoryPanel>,
        Without<SkillCooldownMask>,
        Without<ExperienceFill>,
        Without<PotionPip>,
        Without<ObjectiveText>,
    ),
>;
type ExperienceFillQuery<'w, 's> = Query<
    'w,
    's,
    (&'static mut Node, &'static mut BackgroundColor),
    (
        With<ExperienceFill>,
        Without<ResourceFill>,
        Without<SkillCooldownMask>,
        Without<SkillSlotFrame>,
        Without<InventoryPanel>,
        Without<BuildPanel>,
        Without<PotionPip>,
        Without<ObjectiveText>,
        Without<GearSlotText>,
        Without<InventorySlotText>,
    ),
>;
type PotionPipQuery<'w, 's> = Query<
    'w,
    's,
    (
        &'static PotionPip,
        &'static mut BorderColor,
        &'static mut BackgroundColor,
    ),
    (
        Without<ResourceFill>,
        Without<SkillSlotFrame>,
        Without<SkillCooldownMask>,
        Without<ExperienceFill>,
        Without<InventoryPanel>,
        Without<BuildPanel>,
        Without<ObjectiveText>,
        Without<GearSlotText>,
        Without<InventorySlotText>,
    ),
>;
type ObjectiveFrameQuery<'w, 's> = Query<
    'w,
    's,
    (
        &'static mut Node,
        &'static mut BackgroundColor,
        &'static mut BorderColor,
        &'static mut TextFont,
    ),
    With<ObjectiveText>,
>;
type BuildTextQuery<'w, 's> = Query<
    'w,
    's,
    &'static mut Text,
    (
        With<BuildText>,
        Without<ObjectiveText>,
        Without<HealthText>,
        Without<CooldownText>,
        Without<StatsText>,
        Without<LogText>,
        Without<PauseText>,
        Without<InventoryText>,
        Without<GearSlotText>,
        Without<InventorySlotText>,
        Without<JournalText>,
    ),
>;
type LogVisibilityQuery<'w, 's> =
    Query<'w, 's, &'static mut Visibility, (With<LogText>, Without<JournalText>)>;
type JournalVisibilityQuery<'w, 's> =
    Query<'w, 's, &'static mut Visibility, (With<JournalText>, Without<LogText>)>;

#[derive(SystemParam)]
struct HudNodeQueries<'w, 's> {
    objective_frames: ObjectiveFrameQuery<'w, 's>,
    resource_fills: ResourceFillQuery<'w, 's>,
    resource_orb_frames: ResourceOrbFrameQuery<'w, 's>,
    skill_frames: SkillSlotFrameQuery<'w, 's>,
    skill_masks: SkillCooldownMaskQuery<'w, 's>,
    skill_lamps: SkillSlotLampQuery<'w, 's>,
    inventory_panels: InventoryPanelQuery<'w, 's>,
    build_panels: BuildPanelQuery<'w, 's>,
    experience_fills: ExperienceFillQuery<'w, 's>,
    potion_pips: PotionPipQuery<'w, 's>,
    gear_frames: GearSlotFrameQuery<'w, 's>,
    inventory_frames: InventorySlotFrameQuery<'w, 's>,
    log_visibility: LogVisibilityQuery<'w, 's>,
    journal_visibility: JournalVisibilityQuery<'w, 's>,
}

#[derive(SystemParam)]
struct HudResources<'w> {
    stats: Res<'w, RunStats>,
    progress: Res<'w, ChapterProgress>,
    story: Res<'w, StoryLog>,
    combat_log: Res<'w, CombatLog>,
    pause: Res<'w, PauseState>,
    inventory_open: Res<'w, InventoryOpen>,
    build_open: Res<'w, BuildOpen>,
    journal_open: Res<'w, JournalOpen>,
    audio: Res<'w, AudioSettings>,
    audio_backend: Res<'w, AudioBackendStatus>,
    difficulty: Res<'w, DifficultySettings>,
    journal: Res<'w, LoreJournal>,
    milestones: Res<'w, Milestones>,
    bestiary: Res<'w, Bestiary>,
    bounty: Res<'w, BountyProgress>,
    modifier: Res<'w, ChapterModifier>,
    challenges: Res<'w, ChapterChallenges>,
    mastery: Res<'w, ChapterMastery>,
    obelisk: Res<'w, BloodObelisk>,
    rift: Res<'w, EmberRift>,
    records: Res<'w, ChapterRecords>,
    zone: Res<'w, ChapterZoneState>,
    skill_feedback: Res<'w, SkillFeedbackState>,
    target_info: Res<'w, TargetInfo>,
    hud_density: Res<'w, HudDensity>,
}
type PlayerHudVitalsQuery<'w, 's> = Query<
    'w,
    's,
    (
        &'static Health,
        &'static Fury,
        &'static SkillCooldowns,
        &'static Evade,
        &'static SurgeBuff,
        &'static TownPortal,
        &'static PlayerLevel,
        &'static PotionBelt,
        &'static ElixirBelt,
        &'static Transform,
    ),
    With<Player>,
>;
type PlayerHudBuildQuery<'w, 's> = Query<
    'w,
    's,
    (
        &'static DamageBonus,
        &'static Equipment,
        &'static RelicBuff,
        &'static ElixirBuff,
        &'static FortuneBuff,
        &'static Inventory,
        &'static Talents,
        &'static SkillRunes,
        &'static CompanionBond,
    ),
    With<Player>,
>;
type ChapterBoonQuery<'w, 's> = Query<'w, 's, &'static ChapterBoon, With<Player>>;
type ParagonHudQuery<'w, 's> = Query<'w, 's, &'static EmberParagon, With<Player>>;
type CodexHudQuery<'w, 's> = Query<'w, 's, &'static LegendaryCodex, With<Player>>;
type CharmHudQuery<'w, 's> = Query<'w, 's, &'static Charm, With<Player>>;
type ArmoryHudQuery<'w, 's> = Query<'w, 's, &'static Armory, With<Player>>;
type ConduitHudQuery<'w, 's> = Query<'w, 's, &'static ConduitBuff, With<Player>>;
type GloryHudQuery<'w, 's> = Query<'w, 's, &'static GloryBuff, With<Player>>;
type WardHudQuery<'w, 's> = Query<'w, 's, (&'static DeathWard, &'static Barrier), With<Player>>;
type BossHudQuery<'w, 's> = Query<
    'w,
    's,
    (
        &'static Health,
        &'static BossPhase,
        Option<&'static BossStagger>,
        Option<&'static Staggered>,
    ),
    With<Enemy>,
>;
type BossHudStatusQuery<'w, 's> = Query<
    'w,
    's,
    (
        &'static Health,
        &'static BossPhase,
        Option<&'static BossStagger>,
        Option<&'static Staggered>,
        &'static Enemy,
    ),
    With<Enemy>,
>;
type BossHudPanelQuery<'w, 's> = Query<
    'w,
    's,
    &'static mut Node,
    (
        With<BossHudPanel>,
        Without<BossHudFill>,
        Without<BossHudStaggerFill>,
    ),
>;
type BossHudFillQuery<'w, 's> = Query<
    'w,
    's,
    (&'static mut Node, &'static mut BackgroundColor),
    (
        With<BossHudFill>,
        Without<BossHudPanel>,
        Without<BossHudStaggerFill>,
    ),
>;
type BossHudStaggerFillQuery<'w, 's> = Query<
    'w,
    's,
    (&'static mut Node, &'static mut BackgroundColor),
    (
        With<BossHudStaggerFill>,
        Without<BossHudPanel>,
        Without<BossHudFill>,
    ),
>;
type BossHudTextQuery<'w, 's> = Query<'w, 's, &'static mut Text, With<BossHudText>>;
type TargetInfoPanelQuery<'w, 's> = Query<
    'w,
    's,
    &'static mut Node,
    (
        With<TargetInfoPanel>,
        Without<TargetInfoFill>,
        Without<TargetInfoText>,
    ),
>;
type TargetInfoFillQuery<'w, 's> = Query<
    'w,
    's,
    (&'static mut Node, &'static mut BackgroundColor),
    (
        With<TargetInfoFill>,
        Without<TargetInfoPanel>,
        Without<TargetInfoText>,
    ),
>;
type TargetInfoTextQuery<'w, 's> = Query<
    'w,
    's,
    (&'static mut Text, &'static mut TextColor),
    (
        With<TargetInfoText>,
        Without<TargetInfoPanel>,
        Without<TargetInfoFill>,
    ),
>;
type LootProximityPanelQuery<'w, 's> = Query<
    'w,
    's,
    (&'static mut Node, &'static mut BorderColor),
    (With<LootProximityPanel>, Without<LootProximityText>),
>;
type LootProximityTextQuery<'w, 's> = Query<
    'w,
    's,
    (&'static mut Text, &'static mut TextColor),
    (With<LootProximityText>, Without<LootProximityPanel>),
>;
type InteractProximityPlayerQuery<'w, 's> = Query<
    'w,
    's,
    (
        &'static Transform,
        &'static Inventory,
        &'static Equipment,
        &'static PotionBelt,
        &'static ElixirBelt,
        &'static LegendaryCodex,
    ),
    (With<Player>, Without<Interactable>),
>;
type InteractProximityWorldQuery<'w, 's> =
    Query<'w, 's, (&'static Transform, &'static Interactable), Without<Player>>;
type InteractProximityPanelQuery<'w, 's> = Query<
    'w,
    's,
    (&'static mut Node, &'static mut BorderColor),
    (With<InteractProximityPanel>, Without<InteractProximityText>),
>;
type InteractProximityTextQuery<'w, 's> = Query<
    'w,
    's,
    (&'static mut Text, &'static mut TextColor),
    (With<InteractProximityText>, Without<InteractProximityPanel>),
>;
type BuffBarPlayerQuery<'w, 's> = Query<
    'w,
    's,
    (
        &'static RelicBuff,
        &'static ElixirBuff,
        &'static FortuneBuff,
        &'static ConduitBuff,
        &'static GloryBuff,
        &'static SurgeBuff,
        &'static TownPortal,
        &'static Health,
        Option<&'static Burning>,
        Option<&'static Jailed>,
    ),
    With<Player>,
>;

#[derive(SystemParam)]
struct HudQueryParams<'w, 's> {
    vitals: PlayerHudVitalsQuery<'w, 's>,
    build: PlayerHudBuildQuery<'w, 's>,
    boon: ChapterBoonQuery<'w, 's>,
    paragon: ParagonHudQuery<'w, 's>,
    codex: CodexHudQuery<'w, 's>,
    charm: CharmHudQuery<'w, 's>,
    armory: ArmoryHudQuery<'w, 's>,
    conduit: ConduitHudQuery<'w, 's>,
    glory: GloryHudQuery<'w, 's>,
    ward: WardHudQuery<'w, 's>,
    boss: BossHudQuery<'w, 's>,
    texts: HudTextQueries<'w, 's>,
    nodes: HudNodeQueries<'w, 's>,
}

type MinimapPlayerQuery<'w, 's> = Query<'w, 's, &'static Transform, With<Player>>;
type MinimapEnemyQuery<'w, 's> = Query<'w, 's, (&'static Transform, &'static Enemy)>;
type MinimapLootQuery<'w, 's> = Query<'w, 's, &'static Transform, With<LootEntity>>;
type MinimapHealthGlobeQuery<'w, 's> = Query<'w, 's, &'static Transform, With<HealthGlobe>>;
type MinimapFuryGlobeQuery<'w, 's> = Query<'w, 's, &'static Transform, With<FuryGlobe>>;
type MinimapInteractableQuery<'w, 's> = Query<'w, 's, (&'static Transform, &'static Interactable)>;
type MinimapQueries<'w, 's> = (
    MinimapPlayerQuery<'w, 's>,
    MinimapEnemyQuery<'w, 's>,
    MinimapLootQuery<'w, 's>,
    MinimapHealthGlobeQuery<'w, 's>,
    MinimapFuryGlobeQuery<'w, 's>,
    MinimapInteractableQuery<'w, 's>,
);

type VictoryPlayerProfileQuery<'w, 's> = Query<
    'w,
    's,
    (
        &'static Equipment,
        &'static Charm,
        &'static DeathWard,
        &'static ChapterBoon,
        &'static LegendaryCodex,
    ),
    With<Player>,
>;

type CompactCharacterPlayerQuery<'w, 's> = Query<
    'w,
    's,
    (
        &'static DamageBonus,
        &'static Equipment,
        &'static Charm,
        &'static Inventory,
        &'static LegendaryCodex,
        &'static Talents,
        &'static SkillRunes,
    ),
    With<Player>,
>;
type CompactCharacterTextQuery<'w, 's> =
    Query<'w, 's, (&'static CompactCharacterText, &'static mut Text)>;
type CompactUtilityPlayerQuery<'w, 's> = Query<
    'w,
    's,
    (
        &'static PotionBelt,
        &'static ElixirBelt,
        &'static ElixirBuff,
        &'static TownPortal,
    ),
    With<Player>,
>;
type CompactUtilityTextQuery<'w, 's> =
    Query<'w, 's, (&'static CompactUtilityText, &'static mut Text)>;
type ActionBeltPlayerQuery<'w, 's> = Query<
    'w,
    's,
    (
        &'static Health,
        &'static Barrier,
        &'static Fury,
        &'static SkillCooldowns,
        &'static Evade,
        &'static SurgeBuff,
        &'static ChapterBoon,
        &'static PotionBelt,
        &'static Inventory,
        &'static Equipment,
        &'static DamageBonus,
        &'static SkillRunes,
        &'static Talents,
    ),
    With<Player>,
>;

#[derive(SystemParam)]
struct VictoryResources<'w, 's> {
    stats: ResMut<'w, RunStats>,
    progress: Res<'w, ChapterProgress>,
    difficulty: Res<'w, DifficultySettings>,
    story: Res<'w, StoryLog>,
    bounty: Res<'w, BountyProgress>,
    milestones: Res<'w, Milestones>,
    challenges: Res<'w, ChapterChallenges>,
    mastery: Res<'w, ChapterMastery>,
    obelisk: Res<'w, BloodObelisk>,
    rift: Res<'w, EmberRift>,
    modifier: Res<'w, ChapterModifier>,
    records: ResMut<'w, ChapterRecords>,
    player_profile: VictoryPlayerProfileQuery<'w, 's>,
}

const MINIMAP_WIDTH: f32 = 164.0;
const MINIMAP_HEIGHT: f32 = 132.0;
const MINIMAP_MAP_LEFT: f32 = 8.0;
const MINIMAP_MAP_RIGHT: f32 = 8.0;
const MINIMAP_MAP_TOP: f32 = 34.0;
const MINIMAP_MAP_BOTTOM: f32 = 20.0;
const WORLD_MIN_X: f32 = -13.0;
const WORLD_MAX_X: f32 = 13.0;
// Covers both chambers: outer hall (z in -9..9) and inner sanctum (to -25).
const WORLD_MIN_Z: f32 = -25.0;
const WORLD_MAX_Z: f32 = 9.0;
const MAIN_MENU_BACKGROUND_IMAGE: &str = "images/generated/bevy-open-arpg-concept.png";

pub struct HudPlugin;

impl Plugin for HudPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<ChapterBannerState>()
            .init_resource::<LevelUpBannerState>()
            .init_resource::<LootPickupBannerState>()
            .init_resource::<RewardToastState>()
            .init_resource::<StoryCommsState>()
            .init_resource::<SkillFeedbackState>()
            .init_resource::<CombatStreakBannerState>()
            .init_resource::<ChapterRecords>()
            .init_resource::<HudDensity>()
            .add_systems(
                OnEnter(GameState::MainMenu),
                (load_chapter_record_profile, spawn_menu).chain(),
            )
            .add_systems(OnEnter(GameState::Loading), spawn_loading_screen)
            .add_systems(
                Update,
                update_loading_screen.run_if(in_state(GameState::Loading)),
            )
            .add_systems(OnExit(GameState::Loading), despawn_loading_screen)
            .add_systems(
                Update,
                ensure_main_menu_ui.run_if(in_state(GameState::MainMenu)),
            )
            .add_systems(Update, update_menu.run_if(in_state(GameState::MainMenu)))
            .add_systems(OnExit(GameState::MainMenu), despawn_menu)
            .add_systems(
                OnEnter(GameState::InGame),
                (
                    reset_chapter_banner,
                    reset_level_up_banner,
                    reset_loot_pickup_banner,
                    reset_reward_toast,
                    reset_story_comms,
                    reset_skill_feedback,
                    reset_combat_streak_banner,
                    apply_profile_renown_bonus,
                    spawn_hud,
                )
                    .chain(),
            )
            .add_systems(
                Update,
                (
                    toggle_hud_density,
                    update_hud,
                    update_low_health_vignette_hud,
                    update_compact_character_hud,
                    update_compact_utility_hud,
                    update_buff_bar,
                    update_boss_hud,
                    update_target_info_hud,
                    update_loot_proximity_hud,
                    update_interact_proximity_hud,
                    update_minimap,
                    update_objective_compass_hud,
                    update_chapter_banner,
                    update_level_up_banner,
                    update_loot_pickup_banner,
                    update_reward_toast,
                    update_story_comms,
                    update_skill_feedback,
                    update_action_belt_hud,
                    update_combat_streak_banner,
                )
                    .run_if(in_state(GameState::InGame)),
            )
            .init_resource::<UiPointerCapture>()
            .add_systems(
                Update,
                (
                    update_paper_doll_slots,
                    update_ui_pointer_capture,
                    handle_inventory_slot_clicks,
                    handle_paper_doll_clicks,
                )
                    .run_if(in_state(GameState::InGame)),
            )
            .add_systems(OnExit(GameState::InGame), despawn_hud)
            .add_systems(OnEnter(GameState::GameOver), spawn_game_over)
            .add_systems(OnExit(GameState::GameOver), despawn_menu)
            .add_systems(OnEnter(GameState::Victory), spawn_victory)
            .add_systems(OnExit(GameState::Victory), despawn_menu);
    }
}

fn spawn_menu(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    difficulty: Res<DifficultySettings>,
    records: Res<ChapterRecords>,
    audio: Res<AudioSettings>,
    audio_backend: Res<AudioBackendStatus>,
) {
    spawn_main_menu(
        &mut commands,
        &asset_server,
        &difficulty,
        &records,
        &audio,
        &audio_backend,
    );
}

fn spawn_loading_screen(mut commands: Commands) {
    commands
        .spawn((
            Node {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                flex_direction: FlexDirection::Column,
                row_gap: Val::Px(12.0),
                ..default()
            },
            BackgroundColor(Color::srgba(0.02, 0.02, 0.03, 0.95)),
            LoadingUi,
        ))
        .with_children(|parent| {
            parent.spawn((
                Text::new("Shadow Reliquary"),
                TextFont {
                    font_size: 44.0.into(),
                    ..default()
                },
                TextColor(Color::srgb(0.96, 0.88, 0.64)),
                LoadingTitleText,
            ));
            parent
                .spawn((
                    Node {
                        min_width: Val::Px(560.0),
                        max_width: Val::Px(560.0),
                        margin: UiRect::top(Val::Px(6.0)),
                        padding: UiRect::all(Val::Px(12.0)),
                        ..default()
                    },
                    BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.24)),
                    BorderColor::all(Color::srgba(0.8, 0.6, 0.3, 0.65)),
                    // keep only text-level marker for detail updates
                ))
                .with_children(|panel| {
                    panel.spawn((
                        Text::new("Preparing game resources..."),
                        TextFont {
                            font_size: 14.0.into(),
                            ..default()
                        },
                        TextColor(Color::srgb(0.88, 0.84, 0.72)),
                        LoadingStatusText,
                    ));
                    panel.spawn((
                        Text::new("Loading assets..."),
                        TextFont {
                            font_size: 12.0.into(),
                            ..default()
                        },
                        TextColor(Color::srgb(0.76, 0.76, 0.82)),
                        LoadingDetailText,
                    ));
                });
        });
}

fn update_loading_screen(
    asset_server: Res<AssetServer>,
    assets: Option<Res<GameAssets>>,
    progress: Res<AssetLoadingProgress>,
    mut status_query: Query<&mut Text, (With<LoadingStatusText>, Without<LoadingDetailText>)>,
    mut detail_query: Query<&mut Text, (With<LoadingDetailText>, Without<LoadingStatusText>)>,
) {
    let mut status = "Initializing...".to_string();
    let mut detail = "Waiting for asset pipeline...".to_string();

    if let Some(assets) = assets {
        let summary = game_assets_load_summary(&asset_server, &assets);
        let stage = if summary.ready() {
            "Assets loaded. Entering main menu..."
        } else if progress.timed_out {
            "Asset loading timeout; continuing with available resources."
        } else if summary.settled() {
            "Some assets failed, continuing with remaining resources."
        } else if summary.failed > 0 {
            "Some assets failed, waiting for remaining..."
        } else {
            "Loading..."
        };

        status = stage.to_string();
        detail = format!(
            "ready {}/{}  loading {}  failed {}  checks {}  elapsed {:.1}s",
            summary.loaded,
            summary.total,
            summary.loading + summary.not_loaded,
            summary.failed,
            progress.checks(),
            progress.elapsed_secs(),
        );
    } else if progress.timed_out {
        status = "Loading timed out; using fallback assets where available.".to_string();
    }

    for mut text in &mut status_query {
        **text = status.clone();
    }
    for mut text in &mut detail_query {
        **text = detail.clone();
    }
}

fn despawn_loading_screen(mut commands: Commands, query: Query<Entity, With<LoadingUi>>) {
    for entity in &query {
        queue_safe_despawn(commands.reborrow(), entity);
    }
}

fn ensure_main_menu_ui(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    menu_query: Query<Entity, With<MenuUi>>,
    difficulty: Res<DifficultySettings>,
    records: Res<ChapterRecords>,
    audio: Res<AudioSettings>,
    audio_backend: Res<AudioBackendStatus>,
) {
    if !menu_query.is_empty() {
        return;
    }
    spawn_main_menu(
        &mut commands,
        &asset_server,
        &difficulty,
        &records,
        &audio,
        &audio_backend,
    );
}

fn spawn_main_menu(
    commands: &mut Commands,
    asset_server: &AssetServer,
    difficulty: &DifficultySettings,
    records: &ChapterRecords,
    audio: &AudioSettings,
    audio_backend: &AudioBackendStatus,
) {
    commands
        .spawn((
            Node {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                overflow: Overflow::clip(),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                ..default()
            },
            BackgroundColor(Color::srgba(0.01, 0.01, 0.012, 0.90)),
            MenuUi,
        ))
        .with_children(|root| {
            root.spawn((
                ImageNode::new(asset_server.load(MAIN_MENU_BACKGROUND_IMAGE)),
                Node {
                    position_type: PositionType::Absolute,
                    width: Val::Percent(100.0),
                    height: Val::Percent(100.0),
                    ..default()
                },
            ));
            root.spawn((
                Node {
                    position_type: PositionType::Absolute,
                    width: Val::Percent(100.0),
                    height: Val::Percent(100.0),
                    ..default()
                },
                BackgroundColor(main_menu_scrim_color()),
            ));
            root.spawn((Node {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                flex_direction: FlexDirection::Column,
                row_gap: Val::Px(18.0),
                ..default()
            },))
                .with_children(|parent| {
                    parent.spawn((
                        Text::new("Bevy Open ARPG"),
                        TextFont {
                            font_size: FontSize::Px(56.0),
                            ..default()
                        },
                        TextColor(Color::srgb(0.92, 0.82, 0.68)),
                    ));
                    parent.spawn((
                        Text::new(menu_difficulty_line(difficulty.current, records)),
                        TextFont {
                            font_size: FontSize::Px(22.0),
                            ..default()
                        },
                        TextColor(Color::srgb(0.86, 0.72, 0.28)),
                        MenuDifficultyText,
                    ));
                    parent.spawn((
                        Text::new(menu_profile_summary(
                            records,
                            difficulty.current,
                            audio_backend.status_label(audio),
                        )),
                        TextFont {
                            font_size: FontSize::Px(19.0),
                            ..default()
                        },
                        TextColor(Color::srgb(0.76, 0.70, 0.58)),
                        MenuProfileText,
                    ));
                    parent.spawn((
                        Text::new("Press Space or Enter to start | F9 continue slot 1"),
                        TextFont {
                            font_size: FontSize::Px(24.0),
                            ..default()
                        },
                        TextColor(Color::srgb(0.70, 0.68, 0.64)),
                    ));
                });
        });
}

fn main_menu_scrim_color() -> Color {
    Color::srgba(0.012, 0.010, 0.012, 0.62)
}

#[cfg(test)]
fn main_menu_background_image_path() -> &'static str {
    MAIN_MENU_BACKGROUND_IMAGE
}

#[cfg(test)]
fn main_menu_scrim_alpha() -> f32 {
    main_menu_scrim_color().to_srgba().alpha
}

fn update_menu(
    difficulty: Res<DifficultySettings>,
    records: Res<ChapterRecords>,
    audio: Res<AudioSettings>,
    audio_backend: Res<AudioBackendStatus>,
    mut difficulty_text: Query<&mut Text, (With<MenuDifficultyText>, Without<MenuProfileText>)>,
    mut profile_text: Query<&mut Text, (With<MenuProfileText>, Without<MenuDifficultyText>)>,
) {
    if !difficulty.is_changed()
        && !records.is_changed()
        && !audio.is_changed()
        && !audio_backend.is_changed()
    {
        return;
    }
    if let Ok(mut text) = difficulty_text.single_mut() {
        **text = menu_difficulty_line(difficulty.current, &records);
    }
    if let Ok(mut text) = profile_text.single_mut() {
        **text = menu_profile_summary(
            &records,
            difficulty.current,
            audio_backend.status_label(&audio),
        );
    }
}

fn spawn_game_over(
    mut commands: Commands,
    stats: Res<RunStats>,
    progress: Res<ChapterProgress>,
    difficulty: Res<DifficultySettings>,
    mut records: ResMut<ChapterRecords>,
    death_report: Res<DeathWardReport>,
) {
    let remnant_reward = claim_soul_remnant(&mut records, &stats, difficulty.current);
    let deed_reward = claim_completed_season_deeds(&mut records);
    if (!remnant_reward.is_empty() || !deed_reward.is_empty())
        && let Err(err) = write_chapter_record_profile(PROFILE_PATH, &records)
    {
        warn!("failed to write chapter profile after soul remnant claim: {err}");
    }
    let soul_remnant_text = soul_remnant_summary(remnant_reward, deed_reward, &records);
    let death_scorecard_text =
        death_scorecard_summary(&progress, &stats, &death_report, remnant_reward);
    let death_pursuit_text = death_pursuit_summary(
        &progress,
        &stats,
        difficulty.current,
        &death_report,
        remnant_reward,
    );
    commands
        .spawn((
            Node {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                flex_direction: FlexDirection::Column,
                row_gap: Val::Px(18.0),
                ..default()
            },
            BackgroundColor(Color::srgba(0.02, 0.0, 0.0, 0.86)),
            MenuUi,
        ))
        .with_children(|parent| {
            parent.spawn((
                Text::new("You fell in the reliquary"),
                TextFont {
                    font_size: FontSize::Px(44.0),
                    ..default()
                },
                TextColor(Color::srgb(0.95, 0.32, 0.24)),
            ));
            parent.spawn((
                Text::new(game_over_stats_summary(&stats, &difficulty)),
                TextFont {
                    font_size: FontSize::Px(24.0),
                    ..default()
                },
                TextColor(Color::srgb(0.85, 0.80, 0.70)),
            ));
            parent.spawn((
                Text::new(death_report.summary()),
                TextFont {
                    font_size: FontSize::Px(20.0),
                    ..default()
                },
                TextColor(Color::srgb(0.78, 0.62, 0.54)),
            ));
            parent.spawn((
                Text::new(soul_remnant_text),
                TextFont {
                    font_size: FontSize::Px(20.0),
                    ..default()
                },
                TextColor(Color::srgb(0.72, 0.74, 0.92)),
            ));
            parent.spawn((
                Text::new(death_scorecard_text),
                TextFont {
                    font_size: FontSize::Px(20.0),
                    ..default()
                },
                TextColor(Color::srgb(0.74, 0.84, 0.78)),
            ));
            parent.spawn((
                Text::new(death_pursuit_text),
                TextFont {
                    font_size: FontSize::Px(20.0),
                    ..default()
                },
                TextColor(Color::srgb(0.86, 0.72, 0.42)),
            ));
            parent.spawn((
                Text::new("Press R to restart"),
                TextFont {
                    font_size: FontSize::Px(22.0),
                    ..default()
                },
                TextColor(Color::srgb(0.70, 0.68, 0.64)),
            ));
        });
}

fn game_over_stats_summary(stats: &RunStats, difficulty: &DifficultySettings) -> String {
    format!(
        "Difficulty: {}    Time: {}    Kills: {}    Gold: {}",
        difficulty.current.label(),
        format_run_time(stats.elapsed_secs),
        stats.kills,
        stats.gold
    )
}

fn death_scorecard_summary(
    progress: &ChapterProgress,
    stats: &RunStats,
    death_report: &DeathWardReport,
    remnant: ProfileStashBonus,
) -> String {
    let mainline = if progress.phase == ChapterPhase::Victory || progress.boss_slain {
        "Main cleared".to_string()
    } else {
        format!("Main {:.0}%", chapter_mainline_percent(progress))
    };
    let ward = if death_report.gold_available < death_report.gold_penalty {
        format!(
            "Ward short {}g",
            death_report
                .gold_penalty
                .saturating_sub(death_report.gold_available)
        )
    } else if death_report.revives_used >= death_report.max_revives {
        format!(
            "Ward spent {}/{}",
            death_report.revives_used, death_report.max_revives
        )
    } else {
        "Ward failed".to_string()
    };
    let combat = if stats.boss_staggers > 0 {
        format!("Boss breaks {}/{}", stats.boss_staggers, BOSS_BREAK_TARGET)
    } else if stats.best_valor_stacks > 0 {
        format!("Valor best {}x", stats.best_valor_stacks)
    } else if stats.best_massacre_streak > 0 {
        format!("Massacre best {}", stats.best_massacre_streak)
    } else {
        format!("Kills {}", stats.kills)
    };
    let bank = if remnant.is_empty() {
        "No remnant".to_string()
    } else {
        format!(
            "Remnant +{}g +{}s +{}e",
            remnant.gold, remnant.shards, remnant.essence
        )
    };
    let next = death_scorecard_next_step(progress, stats, death_report, remnant);
    format!("Failure Scorecard: {mainline} | {ward} | {combat} | {bank} | {next}")
}

fn death_scorecard_next_step(
    progress: &ChapterProgress,
    stats: &RunStats,
    death_report: &DeathWardReport,
    remnant: ProfileStashBonus,
) -> &'static str {
    if death_report.gold_available < death_report.gold_penalty {
        "Next: bank gold for ward"
    } else if progress.phase == ChapterPhase::Boss && stats.boss_staggers == 0 {
        "Next: break Malrec first"
    } else if progress.phase == ChapterPhase::Boss {
        "Next: save Surge for execute"
    } else if stats.elapsed_secs < 20.0 && stats.kills == 0 {
        "Next: kite and potion early"
    } else if stats.gold >= 120 || remnant.gold > 0 {
        "Next: spend at quartermaster"
    } else {
        "Next: open cache then side rewards"
    }
}

fn death_pursuit_summary(
    progress: &ChapterProgress,
    stats: &RunStats,
    difficulty: Difficulty,
    death_report: &DeathWardReport,
    remnant: ProfileStashBonus,
) -> String {
    let ward_note = if death_report.gold_available < death_report.gold_penalty {
        "ward short on gold"
    } else if death_report.revives_used >= death_report.max_revives {
        "ward charges spent"
    } else {
        "ward failed"
    };
    let route = match progress.phase {
        ChapterPhase::Boss => {
            if stats.boss_staggers > 0 {
                "retry Malrec: repeat break window, Surge during stagger, then kite enrage"
                    .to_string()
            } else {
                "retry Malrec: choose a boon, bank Surge, and break his stagger meter".to_string()
            }
        }
        ChapterPhase::Ritual => {
            "next run: use well/quartermaster before altar, then claim final seal".to_string()
        }
        ChapterPhase::Sanctum => {
            "next run: clear Seal Warden first, then spend cache loot at quartermaster".to_string()
        }
        ChapterPhase::Cache => {
            "next run: open cache early for upgraded gear before optional fights".to_string()
        }
        ChapterPhase::Breach => {
            if stats.elapsed_secs < 20.0 && stats.kills == 0 {
                "next run: survive first room, evade through swings, drink potion before 30%"
                    .to_string()
            } else {
                "next run: finish outer guards, open cache, then hunt safer side rewards"
                    .to_string()
            }
        }
        ChapterPhase::Victory => {
            "next run: push the unlocked difficulty or replay for a cleaner clear".to_string()
        }
    };
    let remnant_note = if remnant.is_empty() {
        "no remnant banked".to_string()
    } else {
        format!(
            "banked +{}g +{}s +{}e",
            remnant.gold, remnant.shards, remnant.essence
        )
    };
    format!(
        "Death Pursuit: {ward_note} | {route} | {remnant_note} | replay {}",
        difficulty.label()
    )
}

fn spawn_victory(mut commands: Commands, resources: VictoryResources) {
    let VictoryResources {
        mut stats,
        progress,
        difficulty,
        story,
        bounty,
        milestones,
        challenges,
        mastery,
        obelisk,
        rift,
        modifier,
        mut records,
        player_profile,
    } = resources;
    award_chapter_journey_reward(&mut stats, &challenges, &milestones, &mastery);
    award_chapter_clear_reward(&mut stats, &progress, &bounty, &obelisk, &rift, &story);
    let clear_grade = chapter_clear_grade(
        &progress,
        optional_objective_score(&stats, &bounty, &obelisk, &rift),
        &story,
    );
    records.register_chapter_clear_grade(clear_grade);
    let new_record =
        register_chapter_clear(&mut records, difficulty.current, &stats, modifier.affix);
    let sigil_tier = if let Ok((equipment, charm, death_ward, boon, codex)) =
        player_profile.single()
    {
        records.register_completed_set(themed_reliquary_set(equipment, charm));
        records.register_socketed_gem_clear(equipment.socketed_gem);
        records.register_flawless_clear(death_ward.revives_used == 0);
        records.register_completed_boon(boon.choice);
        records.register_legendary_codex_clear(codex);
        records.register_reliquary_sigil_clear(
            difficulty.current,
            &stats,
            clear_grade,
            equipment.socketed_gem,
            Some(codex),
        )
    } else {
        records.register_reliquary_sigil_clear(difficulty.current, &stats, clear_grade, None, None)
    };
    records.register_potionless_clear(stats.potions_used == 0);
    records.register_bounty_cache_claimed(bounty.reward_claimed);
    let deed_reward = claim_completed_season_deeds(&mut records);
    if let Err(err) = write_chapter_record_profile(PROFILE_PATH, &records) {
        warn!("failed to write chapter profile: {err}");
    }
    let recap_sections = victory_recap_sections([
        victory_run_summary(&stats, &difficulty),
        victory_record_summary(&records, difficulty.current, new_record),
        hero_legacy_summary(&records),
        profile_stash_summary(&records),
        season_deed_summary(&records, deed_reward),
        victory_reward_summary(&stats, &mastery),
        victory_reliquary_sigil_summary(
            difficulty.current,
            &stats,
            clear_grade,
            sigil_tier,
            &records,
        ),
        chapter_clear_summary(&progress, &stats, &bounty, &obelisk, &rift, &story),
        victory_scorecard_summary(&progress, &stats, &bounty, &obelisk, &rift, &story),
        format!("Story: {}", story_recap(&story)),
        chapter_quest_log_summary(&progress, &stats, &bounty, &obelisk, &rift),
        journey_summary(&challenges, &milestones, &mastery),
        victory_progress_summary(&challenges, &mastery, &milestones),
        victory_latest_summary(&challenges, &mastery, &milestones),
        victory_pursuit_summary(
            &progress,
            &stats,
            &bounty,
            &obelisk,
            &rift,
            &story,
            &challenges,
            &milestones,
            &mastery,
            difficulty.current,
        ),
        next_run_plan_summary(&records, difficulty.current),
        victory_next_difficulty_summary(difficulty.current),
    ]);
    commands
        .spawn((
            Node {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                flex_direction: FlexDirection::Column,
                row_gap: Val::Px(10.0),
                padding: UiRect::axes(Val::Px(28.0), Val::Px(20.0)),
                ..default()
            },
            BackgroundColor(Color::srgba(0.01, 0.014, 0.012, 0.88)),
            MenuUi,
        ))
        .with_children(|parent| {
            parent.spawn((
                Text::new("The Ashen Reliquary is cleansed"),
                TextFont {
                    font_size: FontSize::Px(38.0),
                    ..default()
                },
                TextColor(Color::srgb(0.84, 0.76, 0.52)),
            ));
            spawn_victory_command_panel(
                parent,
                victory_command_panel_summary(&stats, &records, difficulty.current),
            );
            for section in recap_sections {
                spawn_victory_recap_section(parent, section);
            }
            parent.spawn((
                Text::new(victory_action_prompt(difficulty.current)),
                TextFont {
                    font_size: FontSize::Px(18.0),
                    ..default()
                },
                TextColor(Color::srgb(0.70, 0.68, 0.64)),
            ));
        });
}

#[derive(Debug, PartialEq, Eq)]
struct VictoryRecapSection {
    title: &'static str,
    lines: Vec<String>,
}

fn victory_recap_sections(recap_lines: [String; 17]) -> Vec<VictoryRecapSection> {
    let [
        run,
        record,
        hero,
        stash,
        deed,
        reward,
        sigil,
        clear,
        scorecard,
        story,
        quest,
        journey,
        progress,
        latest,
        pursuit,
        plan,
        next,
    ] = recap_lines;

    vec![
        VictoryRecapSection {
            title: "Run Result",
            lines: vec![run, clear, scorecard, reward],
        },
        VictoryRecapSection {
            title: "Chapter Progress",
            lines: vec![story, quest, journey, progress, latest],
        },
        VictoryRecapSection {
            title: "Legacy",
            lines: vec![record, hero, stash, deed, sigil],
        },
        VictoryRecapSection {
            title: "Next Step",
            lines: vec![pursuit, plan, next],
        },
    ]
}

fn spawn_victory_command_panel(parent: &mut ChildSpawnerCommands, summary: String) {
    parent
        .spawn((
            Node {
                width: Val::Px(1060.0),
                max_width: Val::Percent(92.0),
                flex_direction: FlexDirection::Column,
                row_gap: Val::Px(5.0),
                padding: UiRect::axes(Val::Px(16.0), Val::Px(11.0)),
                border: UiRect::all(Val::Px(2.0)),
                ..default()
            },
            BorderColor::all(Color::srgba(0.86, 0.58, 0.22, 0.76)),
            BackgroundColor(Color::srgba(0.095, 0.063, 0.038, 0.88)),
        ))
        .with_children(|panel| {
            for (index, line) in summary.lines().enumerate() {
                panel.spawn((
                    Text::new(line.to_string()),
                    TextFont {
                        font_size: if index == 0 {
                            FontSize::Px(17.0)
                        } else {
                            FontSize::Px(14.0)
                        },
                        ..default()
                    },
                    TextColor(if index == 0 {
                        Color::srgb(0.98, 0.78, 0.38)
                    } else {
                        Color::srgb(0.88, 0.82, 0.72)
                    }),
                ));
            }
        });
}

fn spawn_victory_recap_section(parent: &mut ChildSpawnerCommands, section: VictoryRecapSection) {
    parent
        .spawn((
            Node {
                width: Val::Px(1060.0),
                max_width: Val::Percent(92.0),
                flex_direction: FlexDirection::Column,
                row_gap: Val::Px(4.0),
                padding: UiRect::axes(Val::Px(14.0), Val::Px(9.0)),
                border: UiRect::all(Val::Px(1.0)),
                ..default()
            },
            BorderColor::all(Color::srgba(0.58, 0.46, 0.26, 0.45)),
            BackgroundColor(Color::srgba(0.055, 0.046, 0.038, 0.72)),
        ))
        .with_children(|section_parent| {
            section_parent.spawn((
                Text::new(section.title),
                TextFont {
                    font_size: FontSize::Px(15.0),
                    ..default()
                },
                TextColor(Color::srgb(0.92, 0.77, 0.42)),
            ));
            for line in section.lines {
                section_parent.spawn((
                    Text::new(line),
                    TextFont {
                        font_size: FontSize::Px(13.5),
                        ..default()
                    },
                    TextColor(Color::srgb(0.80, 0.76, 0.68)),
                ));
            }
        });
}

fn victory_run_summary(stats: &RunStats, difficulty: &DifficultySettings) -> String {
    format!(
        "Run: {} | {} | Rating {} | Kills {} | Boss Breaks {} | Best Valor {}x | Best Momentum {}x | Ashen Surges {} | Nemesis {}",
        difficulty.current.label(),
        format_run_time(stats.elapsed_secs),
        chapter_rating(difficulty.current, stats.elapsed_secs, stats.kills),
        stats.kills,
        stats.boss_staggers,
        stats.best_valor_stacks,
        stats.best_reliquary_momentum,
        stats.ashen_threat_surges,
        stats.nemesis_kills
    )
}

fn victory_next_difficulty_summary(current: Difficulty) -> String {
    let next = escalated_difficulty_after_clear(current);
    let ordeal = modifier_for_difficulty(next).label();
    if next == current {
        format!(
            "Next: stay {} ({ordeal}) | {} | R replay current",
            next.label(),
            difficulty_unlock_after_clear_summary(current)
        )
    } else {
        format!(
            "Next: push {} ({ordeal}) | {} | R replay {}",
            next.label(),
            difficulty_unlock_after_clear_summary(current),
            current.label()
        )
    }
}

fn victory_action_prompt(current: Difficulty) -> String {
    let next = escalated_difficulty_after_clear(current);
    if next == current {
        format!(
            "Space/Enter replay {} endgame | R replay current build",
            current.label()
        )
    } else {
        format!(
            "Space/Enter push {} | R replay {}",
            next.label(),
            current.label()
        )
    }
}

fn difficulty_unlock_after_clear_summary(current: Difficulty) -> &'static str {
    match current {
        Difficulty::Normal => "Unlocked Nightmare",
        Difficulty::Nightmare => "Unlocked Hell",
        Difficulty::Hell => "Unlocked Torment",
        Difficulty::Torment => "Torment endgame remains unlocked",
    }
}

fn menu_difficulty_line(difficulty: Difficulty, records: &ChapterRecords) -> String {
    let modifier = modifier_for_run(difficulty, records.clears(difficulty));
    format!(
        "Difficulty: {}  |  Ordeal: {}  |  {}  |  Clears: {}  |  Record: {}  |  Tab unlocked",
        difficulty.label(),
        modifier.label(),
        difficulty_unlock_summary(difficulty, records),
        records.clears(difficulty),
        chapter_record_summary(records, difficulty)
    )
}

#[cfg_attr(not(test), allow(dead_code))]
fn profile_summary(records: &ChapterRecords) -> String {
    let highest = records
        .highest_cleared_difficulty()
        .map(|difficulty| difficulty.label())
        .unwrap_or("None");
    format!(
        "Profile: title {} | badges {} | renown {} | altar {}/{} | deeds {}/{} | {} | {} clears | highest {} | best {} | clear {} (S {}) | gem best {} asc {} parag {} | sigil best {} clears {} perfect {} | boons {}/3 | codex {}/{} ({}) | journey {} | {} kills | {} gold | {} shards | {} essence | Stash {}g/{}s/{}e | Remnants {} runs {}g/{}s/{}e | Boss Breaks {} | Primal Caches {} | Primal Infusions {} | Ashen Surges {} | Nemesis {} | Treasure Vaults {} | Bounties {} | Sets {}/4 | Flawless {} | Potionless {} | Affix Codex {} | Soul Sigils {} | Trophy Caches {} | Ordeals {}/4 | Torment {}",
        legacy_title(records),
        legacy_badges_summary(records),
        records.renown_rank(),
        reliquary_altar_seals_unlocked(records),
        ALL_RELIQUARY_ALTAR_SEALS.len(),
        records.completed_season_deeds(),
        ALL_SEASON_DEEDS.len(),
        season_deed_pursuit_summary(records),
        records.total_clears(),
        highest,
        records.best_rating_label(),
        records.best_chapter_clear_grade_label(),
        records.lifetime_s_chapter_clears,
        records.best_socketed_gem_rank,
        records.lifetime_ascendant_gem_clears,
        records.lifetime_paragon_gem_clears,
        records.best_reliquary_sigil_tier,
        records.lifetime_reliquary_sigil_clears,
        records.lifetime_perfect_sigil_clears,
        records.completed_boon_mask.count_ones(),
        records.best_legendary_codex_powers,
        LegendaryPower::ALL.len(),
        records.lifetime_legendary_codex_completions,
        records.best_journey_label(),
        records.lifetime_kills,
        records.lifetime_gold,
        records.lifetime_shards,
        records.lifetime_essence,
        records.stash_gold,
        records.stash_shards,
        records.stash_essence,
        records.failed_runs,
        records.soul_remnant_gold,
        records.soul_remnant_shards,
        records.soul_remnant_essence,
        records.lifetime_boss_breaks,
        records.lifetime_primal_caches,
        records.lifetime_primal_infusions,
        records.lifetime_ashen_surges,
        records.lifetime_nemesis_kills,
        records.lifetime_treasure_vaults,
        records.lifetime_bounties_completed,
        records.completed_set_mask.count_ones(),
        records.lifetime_flawless_clears,
        records.lifetime_potionless_clears,
        records.lifetime_affix_codexes,
        records.lifetime_soul_sigils,
        records.lifetime_soul_sigil_caches,
        records.cleared_ordeal_affix_mask.count_ones(),
        records.clears(Difficulty::Torment)
    )
}

fn menu_profile_summary(
    records: &ChapterRecords,
    current_difficulty: Difficulty,
    audio_status: &str,
) -> String {
    let highest = records
        .highest_cleared_difficulty()
        .map(|difficulty| difficulty.label())
        .unwrap_or("None");
    let stash = profile_stash_starting_bonus(records);
    let stash_line = if stash.is_empty() {
        "Start: no stash grant yet".to_string()
    } else {
        format!(
            "Start: +{}g +{}s +{}e from shared stash",
            stash.gold, stash.shards, stash.essence
        )
    };
    let altar_line = reliquary_altar_summary(records);
    format!(
        "HERO LOADOUT | {} | clears {} | highest {} | best {} | journey {}\nProfile: stash {}g/{}s/{}e | {} | {audio_status} (M)\n{}\n{}\nControls: Space/Enter start | Tab difficulty | F9 continue | M audio",
        legacy_title(records),
        records.total_clears(),
        highest,
        records.best_rating_label(),
        records.best_journey_label(),
        records.stash_gold,
        records.stash_shards,
        records.stash_essence,
        stash_line,
        compact_label(&altar_line, 92),
        next_run_plan_summary(records, current_difficulty)
    )
}

fn hero_legacy_summary(records: &ChapterRecords) -> String {
    format!(
        "Hero Legacy: Title {} | Badges {} | Boss Breaks {} (best {}) | Primal Caches {} | Primal Infusions {} | Echo Keys {} | Ashen Surges {} | Nemesis {} | Treasure Vaults {} | Clear {} (S {}) | Gem best {} asc {} parag {} | Sigil best {} clears {} perfect {} | Boons {}/3 | Codex {}/{} ({}) | Sets {}/4 | Flawless {} | Potionless {} | Affix Codex {} | Soul Sigils {} | Trophy Caches {} | Soul Remnants {} | Best Massacre {} | Best Valor {}x",
        legacy_title(records),
        legacy_badges_summary(records),
        records.lifetime_boss_breaks,
        records.best_boss_breaks,
        records.lifetime_primal_caches,
        records.lifetime_primal_infusions,
        records.lifetime_echo_keystones,
        records.lifetime_ashen_surges,
        records.lifetime_nemesis_kills,
        records.lifetime_treasure_vaults,
        records.best_chapter_clear_grade_label(),
        records.lifetime_s_chapter_clears,
        records.best_socketed_gem_rank,
        records.lifetime_ascendant_gem_clears,
        records.lifetime_paragon_gem_clears,
        records.best_reliquary_sigil_tier,
        records.lifetime_reliquary_sigil_clears,
        records.lifetime_perfect_sigil_clears,
        records.completed_boon_mask.count_ones(),
        records.best_legendary_codex_powers,
        LegendaryPower::ALL.len(),
        records.lifetime_legendary_codex_completions,
        records.completed_set_mask.count_ones(),
        records.lifetime_flawless_clears,
        records.lifetime_potionless_clears,
        records.lifetime_affix_codexes,
        records.lifetime_soul_sigils,
        records.lifetime_soul_sigil_caches,
        records.failed_runs,
        records.best_massacre_streak,
        records.best_valor_stacks
    )
}

fn legacy_title(records: &ChapterRecords) -> &'static str {
    if all_season_deeds_finished(records) {
        "Season Conqueror"
    } else if records.clears(Difficulty::Torment) >= 3
        && records.lifetime_s_chapter_clears >= 2
        && records.best_journey_score >= 60
    {
        "Primal Paragon"
    } else if records.has_clear(Difficulty::Torment) {
        "Torment Vanquisher"
    } else if records.has_clear(Difficulty::Hell) {
        "Hellforged Champion"
    } else if records.has_clear(Difficulty::Nightmare) {
        "Nightmare Delver"
    } else if records.total_clears() > 0 {
        "Reliquary Seeker"
    } else {
        "Ashen Wanderer"
    }
}

fn legacy_badges_summary(records: &ChapterRecords) -> String {
    let mut badges = Vec::new();
    if records.lifetime_s_chapter_clears > 0 {
        badges.push("S-Clear");
    }
    if records.has_clear(Difficulty::Torment) {
        badges.push("Torment");
    }
    if records.lifetime_primal_caches > 0 || records.lifetime_primal_infusions > 0 {
        badges.push("Primal");
    }
    if records.lifetime_ascendant_gem_clears > 0 || records.best_socketed_gem_rank >= 5 {
        badges.push("Ascendant");
    }
    if records.lifetime_paragon_gem_clears > 0 || records.best_socketed_gem_rank >= 7 {
        badges.push("ParagonGem");
    }
    if records.completed_set_mask.count_ones() == ReliquarySet::ALL.len() as u32 {
        badges.push("Setmaster");
    }
    if records.cleared_ordeal_affix_mask.count_ones() == 4 {
        badges.push("Ordeal");
    }
    if records.best_journey_score >= 60 {
        badges.push("Journey");
    }
    if records.lifetime_reliquary_sigil_clears > 0 || records.best_reliquary_sigil_tier > 0 {
        badges.push("Sigil");
    }
    if records.lifetime_perfect_sigil_clears > 0
        || records.best_reliquary_sigil_tier >= PERFECT_RELIQUARY_SIGIL_TIER
    {
        badges.push("PerfectSigil");
    }
    if all_boons_completed(records) {
        badges.push("Boonmaster");
    }
    if records.lifetime_legendary_codex_completions > 0
        || records.best_legendary_codex_powers == LegendaryPower::ALL.len() as u32
    {
        badges.push("Codex");
    }
    if all_season_deeds_finished(records) {
        badges.push("Season");
    }

    if badges.is_empty() {
        "none".to_string()
    } else {
        badges.join("/")
    }
}

fn all_season_deeds_finished(records: &ChapterRecords) -> bool {
    records.completed_season_deeds() == ALL_SEASON_DEEDS.len()
        || ALL_SEASON_DEEDS
            .iter()
            .all(|deed| records.season_deeds_claimed.contains(deed))
}

fn profile_stash_summary(records: &ChapterRecords) -> String {
    let bonus = profile_stash_starting_bonus(records);
    format!(
        "Shared Stash: {} gold {} shards {} essence banked | next run +{} gold +{} shards +{} essence | {}",
        records.stash_gold,
        records.stash_shards,
        records.stash_essence,
        bonus.gold,
        bonus.shards,
        bonus.essence,
        reliquary_altar_summary(records)
    )
}

fn affix_codex_complete_for_profile(stats: &RunStats) -> bool {
    stats.affix_codex_mask.count_ones() >= AFFIX_CODEX_TARGET
}

fn legendary_codex_power_count(codex: &LegendaryCodex) -> u32 {
    LegendaryPower::ALL
        .iter()
        .filter(|power| codex.unlocked.contains(power))
        .count() as u32
}

fn ordeal_affix_mask(affix: OrdealAffix) -> u8 {
    match affix {
        OrdealAffix::None => 0,
        OrdealAffix::AshenHunger => 1 << 0,
        OrdealAffix::GlassRelics => 1 << 1,
        OrdealAffix::TreasureFever => 1 << 2,
        OrdealAffix::CinderVeins => 1 << 3,
    }
}

fn all_ordeal_affixes_cleared(records: &ChapterRecords) -> bool {
    const ALL_ROTATING_ORDEAL_AFFIXES: u8 = (1 << 4) - 1;
    records.cleared_ordeal_affix_mask & ALL_ROTATING_ORDEAL_AFFIXES == ALL_ROTATING_ORDEAL_AFFIXES
}

fn all_themed_sets_collected(records: &ChapterRecords) -> bool {
    let all_sets = ReliquarySet::ALL
        .into_iter()
        .fold(0, |mask, set| mask | set.mask());
    records.completed_set_mask & all_sets == all_sets
}

fn chapter_boon_mask(choice: ChapterBoonChoice) -> u8 {
    match choice {
        ChapterBoonChoice::None => 0,
        ChapterBoonChoice::CrimsonOath => 1 << 0,
        ChapterBoonChoice::TitanWard => 1 << 1,
        ChapterBoonChoice::ArcaneMomentum => 1 << 2,
    }
}

fn all_boons_completed(records: &ChapterRecords) -> bool {
    const ALL_RELIQUARY_BOONS: u8 = (1 << 3) - 1;
    records.completed_boon_mask & ALL_RELIQUARY_BOONS == ALL_RELIQUARY_BOONS
}

fn reliquary_altar_seal_complete(seal: ReliquaryAltarSealId, records: &ChapterRecords) -> bool {
    match seal {
        ReliquaryAltarSealId::FirstEmber => records.total_clears() >= 1,
        ReliquaryAltarSealId::TormentBrand => records.clears(Difficulty::Torment) >= 1,
        ReliquaryAltarSealId::PrimalSpark => records.lifetime_primal_caches >= 1,
        ReliquaryAltarSealId::ParagonGem => {
            records.lifetime_paragon_gem_clears >= 1 || records.best_socketed_gem_rank >= 7
        }
        ReliquaryAltarSealId::CodexVow => records.lifetime_legendary_codex_completions >= 1,
        ReliquaryAltarSealId::BoonTriad => all_boons_completed(records),
        ReliquaryAltarSealId::PerfectSigil => records.lifetime_perfect_sigil_clears >= 1,
        ReliquaryAltarSealId::SeasonCrown => all_season_deeds_finished(records),
    }
}

fn reliquary_altar_seals_unlocked(records: &ChapterRecords) -> u32 {
    ALL_RELIQUARY_ALTAR_SEALS
        .into_iter()
        .filter(|seal| reliquary_altar_seal_complete(*seal, records))
        .count() as u32
}

fn next_reliquary_altar_seal(records: &ChapterRecords) -> Option<ReliquaryAltarSealId> {
    ALL_RELIQUARY_ALTAR_SEALS
        .into_iter()
        .find(|seal| !reliquary_altar_seal_complete(*seal, records))
}

fn reliquary_altar_starting_bonus(records: &ChapterRecords) -> ProfileStashBonus {
    let seals = reliquary_altar_seals_unlocked(records);
    ProfileStashBonus {
        gold: seals.saturating_mul(25),
        shards: seals / 2,
        essence: seals / 4,
    }
}

fn reliquary_altar_summary(records: &ChapterRecords) -> String {
    let seals = reliquary_altar_seals_unlocked(records);
    let bonus = reliquary_altar_starting_bonus(records);
    let next = next_reliquary_altar_seal(records)
        .map(|seal| format!("next {}: {}", seal.label(), seal.objective()))
        .unwrap_or_else(|| "all seals lit".to_string());
    format!(
        "Altar {}/{} seals | {} | start +{} gold +{} shards +{} essence",
        seals,
        ALL_RELIQUARY_ALTAR_SEALS.len(),
        next,
        bonus.gold,
        bonus.shards,
        bonus.essence
    )
}

fn apply_profile_renown_bonus(mut stats: ResMut<RunStats>, records: Res<ChapterRecords>) {
    apply_profile_starting_bonus_to_stats(&mut stats, &records);
}

fn apply_profile_starting_bonus_to_stats(stats: &mut RunStats, records: &ChapterRecords) -> bool {
    let renown_applied = apply_renown_bonus_to_stats(stats, records);
    let stash_applied = apply_profile_stash_bonus_to_stats(stats, records);
    let altar_applied = apply_reliquary_altar_bonus_to_stats(stats, records);
    renown_applied || stash_applied || altar_applied
}

fn apply_renown_bonus_to_stats(stats: &mut RunStats, records: &ChapterRecords) -> bool {
    if stats.renown_bonus_claimed {
        return false;
    }
    let rank = records.renown_rank();
    if rank == 0 {
        return false;
    }
    let reward = renown_starting_bonus(rank);
    stats.renown_rank = rank;
    stats.renown_bonus_gold = reward.gold;
    stats.renown_bonus_shards = reward.shards;
    stats.renown_bonus_essence = reward.essence;
    stats.gold = stats.gold.saturating_add(reward.gold);
    stats.ember_shards = stats.ember_shards.saturating_add(reward.shards);
    stats.affix_essence = stats.affix_essence.saturating_add(reward.essence);
    stats.renown_bonus_claimed = true;
    true
}

fn apply_profile_stash_bonus_to_stats(stats: &mut RunStats, records: &ChapterRecords) -> bool {
    if stats.stash_bonus_claimed {
        return false;
    }
    let bonus = profile_stash_starting_bonus(records);
    if bonus.is_empty() {
        return false;
    }
    stats.stash_bonus_gold = bonus.gold;
    stats.stash_bonus_shards = bonus.shards;
    stats.stash_bonus_essence = bonus.essence;
    stats.gold = stats.gold.saturating_add(bonus.gold);
    stats.ember_shards = stats.ember_shards.saturating_add(bonus.shards);
    stats.affix_essence = stats.affix_essence.saturating_add(bonus.essence);
    stats.stash_bonus_claimed = true;
    true
}

fn apply_reliquary_altar_bonus_to_stats(stats: &mut RunStats, records: &ChapterRecords) -> bool {
    if stats.altar_bonus_claimed {
        return false;
    }
    let seals = reliquary_altar_seals_unlocked(records);
    if seals == 0 {
        return false;
    }
    let bonus = reliquary_altar_starting_bonus(records);
    stats.altar_seals = seals;
    stats.altar_bonus_gold = bonus.gold;
    stats.altar_bonus_shards = bonus.shards;
    stats.altar_bonus_essence = bonus.essence;
    stats.gold = stats.gold.saturating_add(bonus.gold);
    stats.ember_shards = stats.ember_shards.saturating_add(bonus.shards);
    stats.affix_essence = stats.affix_essence.saturating_add(bonus.essence);
    stats.altar_bonus_claimed = true;
    true
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
struct RenownStartingBonus {
    gold: u32,
    shards: u32,
    essence: u32,
}

fn renown_starting_bonus(rank: u32) -> RenownStartingBonus {
    RenownStartingBonus {
        gold: rank.saturating_mul(35),
        shards: rank / 2,
        essence: rank / 4,
    }
}

fn renown_rank_for_score(score: u32) -> u32 {
    (score / 25).min(10)
}

#[derive(Debug, Clone, Copy, Default, Eq, PartialEq)]
struct ProfileStashBonus {
    gold: u32,
    shards: u32,
    essence: u32,
}

impl ProfileStashBonus {
    fn is_empty(self) -> bool {
        self.gold == 0 && self.shards == 0 && self.essence == 0
    }
}

fn profile_stash_deposit(stats: &RunStats) -> ProfileStashBonus {
    ProfileStashBonus {
        gold: stats.gold / 10,
        shards: stats.ember_shards / 4,
        essence: stats.affix_essence / 3,
    }
}

fn profile_stash_starting_bonus(records: &ChapterRecords) -> ProfileStashBonus {
    ProfileStashBonus {
        gold: (records.stash_gold / 20).min(250),
        shards: (records.stash_shards / 8).min(8),
        essence: (records.stash_essence / 6).min(4),
    }
}

fn claim_soul_remnant(
    records: &mut ChapterRecords,
    stats: &RunStats,
    difficulty: Difficulty,
) -> ProfileStashBonus {
    let reward = soul_remnant_reward(stats, difficulty);
    if reward.is_empty() {
        return reward;
    }

    records.failed_runs = records.failed_runs.saturating_add(1);
    records.soul_remnant_gold = records.soul_remnant_gold.saturating_add(reward.gold);
    records.soul_remnant_shards = records.soul_remnant_shards.saturating_add(reward.shards);
    records.soul_remnant_essence = records.soul_remnant_essence.saturating_add(reward.essence);
    records.stash_gold = records.stash_gold.saturating_add(reward.gold);
    records.stash_shards = records.stash_shards.saturating_add(reward.shards);
    records.stash_essence = records.stash_essence.saturating_add(reward.essence);
    reward
}

fn soul_remnant_reward(stats: &RunStats, difficulty: Difficulty) -> ProfileStashBonus {
    if stats.elapsed_secs < 20.0
        && stats.kills == 0
        && stats.gold == 0
        && stats.ember_shards == 0
        && stats.affix_essence == 0
    {
        return ProfileStashBonus::default();
    }

    let difficulty_gold = match difficulty {
        Difficulty::Normal => 0,
        Difficulty::Nightmare => 12,
        Difficulty::Hell => 24,
        Difficulty::Torment => 40,
    };
    let mut reward = ProfileStashBonus {
        gold: stats
            .kills
            .saturating_mul(3)
            .saturating_add(stats.gold / 12)
            .saturating_add(difficulty_gold)
            .min(180),
        shards: (stats.ember_shards / 5)
            .saturating_add(stats.kills / 12)
            .saturating_add(u32::from(matches!(
                difficulty,
                Difficulty::Hell | Difficulty::Torment
            )))
            .min(5),
        essence: (stats.affix_essence / 6)
            .saturating_add(stats.boss_staggers / 2)
            .saturating_add(u32::from(
                matches!(difficulty, Difficulty::Torment) && stats.kills >= 20,
            ))
            .min(3),
    };

    if reward.is_empty() {
        reward.gold = 15;
    }
    reward
}

fn soul_remnant_summary(
    reward: ProfileStashBonus,
    deed_reward: ProfileStashBonus,
    records: &ChapterRecords,
) -> String {
    if reward.is_empty() {
        if !deed_reward.is_empty() {
            return format!(
                "Soul Remnant: no recoverable echoes | deed +{} gold +{} shards +{} essence | stash {}g/{}s/{}e",
                deed_reward.gold,
                deed_reward.shards,
                deed_reward.essence,
                records.stash_gold,
                records.stash_shards,
                records.stash_essence
            );
        }
        return "Soul Remnant: no recoverable echoes".to_string();
    }

    let deed_text = if deed_reward.is_empty() {
        "no deed reward".to_string()
    } else {
        format!(
            "deed +{} gold +{} shards +{} essence",
            deed_reward.gold, deed_reward.shards, deed_reward.essence
        )
    };
    format!(
        "Soul Remnant: recovered +{} gold +{} shards +{} essence | {} | failures {} | stash {}g/{}s/{}e",
        reward.gold,
        reward.shards,
        reward.essence,
        deed_text,
        records.failed_runs,
        records.stash_gold,
        records.stash_shards,
        records.stash_essence
    )
}

fn season_deed_complete(deed: SeasonDeedId, records: &ChapterRecords) -> bool {
    match deed {
        SeasonDeedId::FirstClear => records.total_clears() >= 1,
        SeasonDeedId::NightmareVeteran => {
            records.clears(Difficulty::Nightmare)
                + records.clears(Difficulty::Hell)
                + records.clears(Difficulty::Torment)
                >= 1
        }
        SeasonDeedId::TormentClear => records.clears(Difficulty::Torment) >= 1,
        SeasonDeedId::BossBreaker => records.lifetime_boss_breaks >= 10,
        SeasonDeedId::PrimalHunter => records.lifetime_primal_caches >= 3,
        SeasonDeedId::PrimalArtificer => records.lifetime_primal_infusions >= 3,
        SeasonDeedId::AshboundVendetta => records.lifetime_nemesis_kills >= 3,
        SeasonDeedId::VaultSeeker => records.lifetime_treasure_vaults >= 3,
        SeasonDeedId::WritHunter => records.lifetime_bounties_completed >= 5,
        SeasonDeedId::SetCollector => all_themed_sets_collected(records),
        SeasonDeedId::FlawlessChampion => records.lifetime_flawless_clears >= 3,
        SeasonDeedId::UntouchedAscetic => records.lifetime_potionless_clears >= 3,
        SeasonDeedId::BattleTranceVeteran => {
            records.best_massacre_streak >= 20 && records.best_valor_stacks >= 5
        }
        SeasonDeedId::ChapterClearAdept => records.lifetime_s_chapter_clears >= 2,
        SeasonDeedId::AscendantLapidary => records.lifetime_ascendant_gem_clears >= 2,
        SeasonDeedId::ParagonLapidary => records.lifetime_paragon_gem_clears >= 2,
        SeasonDeedId::AffixArchivist => records.lifetime_affix_codexes >= 3,
        SeasonDeedId::TrophyHoarder => records.lifetime_soul_sigil_caches >= 3,
        SeasonDeedId::OrdealConqueror => all_ordeal_affixes_cleared(records),
        SeasonDeedId::ReliquaryBanker => {
            records.stash_gold >= 1_000 && records.stash_shards >= 20 && records.stash_essence >= 10
        }
        SeasonDeedId::SoulboundResolve => records.failed_runs >= 3,
        SeasonDeedId::BoonTriad => all_boons_completed(records),
        SeasonDeedId::CodexKeeper => records.lifetime_legendary_codex_completions >= 2,
        SeasonDeedId::ConquerorPath => records.best_journey_score >= 60,
        SeasonDeedId::SigilEngraver => records.lifetime_perfect_sigil_clears >= 2,
    }
}

fn claim_completed_season_deeds(records: &mut ChapterRecords) -> ProfileStashBonus {
    let mut reward = ProfileStashBonus::default();
    for deed in ALL_SEASON_DEEDS {
        if records.season_deeds_claimed.contains(&deed) || !season_deed_complete(deed, records) {
            continue;
        }
        let deed_reward = deed.reward();
        reward.gold = reward.gold.saturating_add(deed_reward.gold);
        reward.shards = reward.shards.saturating_add(deed_reward.shards);
        reward.essence = reward.essence.saturating_add(deed_reward.essence);
        records.season_deeds_claimed.push(deed);
    }
    records.stash_gold = records.stash_gold.saturating_add(reward.gold);
    records.stash_shards = records.stash_shards.saturating_add(reward.shards);
    records.stash_essence = records.stash_essence.saturating_add(reward.essence);
    reward
}

fn next_season_deed(records: &ChapterRecords) -> Option<SeasonDeedId> {
    ALL_SEASON_DEEDS.into_iter().find(|deed| {
        !records.season_deeds_claimed.contains(deed) && !season_deed_complete(*deed, records)
    })
}

fn claimable_season_deed(records: &ChapterRecords) -> Option<SeasonDeedId> {
    ALL_SEASON_DEEDS.into_iter().find(|deed| {
        !records.season_deeds_claimed.contains(deed) && season_deed_complete(*deed, records)
    })
}

fn season_deed_pursuit_summary(records: &ChapterRecords) -> String {
    if let Some(deed) = claimable_season_deed(records) {
        let reward = deed.reward();
        return format!(
            "Deed ready {} +{}g +{}s +{}e",
            deed.label(),
            reward.gold,
            reward.shards,
            reward.essence
        );
    }
    next_season_deed(records)
        .map(|deed| format!("Next deed {}: {}", deed.label(), deed.objective()))
        .unwrap_or_else(|| "All deeds claimed".to_string())
}

fn season_deed_summary(records: &ChapterRecords, reward: ProfileStashBonus) -> String {
    let reward_text = if reward.is_empty() {
        "No new deed reward".to_string()
    } else {
        format!(
            "Claimed +{} gold +{} shards +{} essence",
            reward.gold, reward.shards, reward.essence
        )
    };
    let next = next_season_deed(records)
        .map(|deed| format!("Next {}: {}", deed.label(), deed.objective()))
        .unwrap_or_else(|| "All season deeds claimed".to_string());
    format!(
        "Season Deeds: {}/{} complete | {} | {}",
        records.season_deeds_claimed.len(),
        ALL_SEASON_DEEDS.len(),
        next,
        reward_text
    )
}

fn chapter_record_summary(records: &ChapterRecords, difficulty: Difficulty) -> String {
    match records.get(difficulty) {
        Some(record) => format_chapter_record(record),
        None => "no clear yet".to_string(),
    }
}

fn victory_record_summary(
    records: &ChapterRecords,
    difficulty: Difficulty,
    new_record: bool,
) -> String {
    let prefix = if new_record {
        "Record: new best"
    } else {
        "Record: best"
    };
    format!(
        "{} {} - {}",
        prefix,
        difficulty.label(),
        chapter_record_summary(records, difficulty)
    )
}

fn format_chapter_record(record: ChapterRecord) -> String {
    format!(
        "{} in {}, {} kills, {} gold",
        record.rating,
        format_run_time(record.elapsed_secs),
        record.kills,
        record.gold
    )
}

fn record_chapter_clear(
    records: &mut ChapterRecords,
    difficulty: Difficulty,
    stats: &RunStats,
) -> bool {
    let candidate = ChapterRecord {
        elapsed_secs: stats.elapsed_secs,
        rating: chapter_rating(difficulty, stats.elapsed_secs, stats.kills).to_string(),
        kills: stats.kills,
        gold: stats.gold,
    };
    let slot = records.get_mut(difficulty);
    if slot
        .as_ref()
        .map(|current| chapter_record_is_better(&candidate, current))
        .unwrap_or(true)
    {
        *slot = Some(candidate);
        true
    } else {
        false
    }
}

fn register_chapter_clear(
    records: &mut ChapterRecords,
    difficulty: Difficulty,
    stats: &RunStats,
    affix: OrdealAffix,
) -> bool {
    records.add_clear(difficulty);
    let rating = chapter_rating(difficulty, stats.elapsed_secs, stats.kills);
    records.register_lifetime_stats(stats, rating);
    records.register_ordeal_affix_clear(affix);
    record_chapter_clear(records, difficulty, stats)
}

fn chapter_record_is_better(candidate: &ChapterRecord, current: &ChapterRecord) -> bool {
    const TIME_EPSILON: f32 = 0.05;
    if candidate.elapsed_secs + TIME_EPSILON < current.elapsed_secs {
        return true;
    }
    (candidate.elapsed_secs - current.elapsed_secs).abs() <= TIME_EPSILON
        && (
            rating_rank(&candidate.rating),
            candidate.kills,
            candidate.gold,
        ) > (rating_rank(&current.rating), current.kills, current.gold)
}

fn rating_rank(rating: &str) -> u8 {
    match rating {
        "S" => 4,
        "A" => 3,
        "B" => 2,
        "C" => 1,
        _ => 0,
    }
}

fn load_chapter_record_profile(mut records: ResMut<ChapterRecords>) {
    match read_chapter_record_profile(PROFILE_PATH) {
        Ok(profile) => *records = profile,
        Err(err) if profile_missing(&err) => {}
        Err(err) => warn!("failed to read chapter profile: {err}"),
    }
}

fn profile_missing(err: &str) -> bool {
    err.contains("No such file")
        || err.contains("os error 2")
        || err.contains("not persisted in the web build")
}

#[cfg(not(target_arch = "wasm32"))]
fn write_chapter_record_profile(
    path: impl AsRef<Path>,
    records: &ChapterRecords,
) -> Result<(), String> {
    let path = path.as_ref();
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|err| err.to_string())?;
    }
    let content = ron::ser::to_string_pretty(records, ron::ser::PrettyConfig::default())
        .map_err(|err| err.to_string())?;
    fs::write(path, content).map_err(|err| err.to_string())
}

#[cfg(target_arch = "wasm32")]
fn write_chapter_record_profile(
    _path: impl AsRef<Path>,
    _records: &ChapterRecords,
) -> Result<(), String> {
    Err("chapter records are not persisted in the web build".to_string())
}

#[cfg(not(target_arch = "wasm32"))]
fn read_chapter_record_profile(path: impl AsRef<Path>) -> Result<ChapterRecords, String> {
    let content = fs::read_to_string(path).map_err(|err| err.to_string())?;
    ron::from_str(&content).map_err(|err| err.to_string())
}

#[cfg(target_arch = "wasm32")]
fn read_chapter_record_profile(_path: impl AsRef<Path>) -> Result<ChapterRecords, String> {
    Err("chapter records are not persisted in the web build".to_string())
}

#[derive(Debug, Clone, Copy, Default, Eq, PartialEq)]
struct ChapterClearReward {
    gold: u32,
    shards: u32,
    essence: u32,
}

impl ChapterClearReward {
    fn is_empty(self) -> bool {
        self.gold == 0 && self.shards == 0 && self.essence == 0
    }
}

fn award_chapter_clear_reward(
    stats: &mut RunStats,
    progress: &ChapterProgress,
    bounty: &BountyProgress,
    obelisk: &BloodObelisk,
    rift: &EmberRift,
    story: &StoryLog,
) -> ChapterClearReward {
    if stats.chapter_clear_reward_claimed {
        return ChapterClearReward::default();
    }
    let optional = optional_objective_score(stats, bounty, obelisk, rift);
    let reward = chapter_clear_reward(chapter_clear_grade(progress, optional, story));
    if reward.is_empty() {
        return reward;
    }
    stats.gold += reward.gold;
    stats.ember_shards += reward.shards;
    stats.affix_essence += reward.essence;
    stats.chapter_clear_bonus_gold = reward.gold;
    stats.chapter_clear_bonus_shards = reward.shards;
    stats.chapter_clear_bonus_essence = reward.essence;
    stats.chapter_clear_reward_claimed = true;
    reward
}

fn chapter_clear_reward(grade: &str) -> ChapterClearReward {
    match grade {
        "S" => ChapterClearReward {
            gold: 360,
            shards: 12,
            essence: 6,
        },
        "A" => ChapterClearReward {
            gold: 220,
            shards: 7,
            essence: 3,
        },
        "B" => ChapterClearReward {
            gold: 120,
            shards: 4,
            essence: 1,
        },
        "C" => ChapterClearReward {
            gold: 60,
            shards: 2,
            essence: 0,
        },
        _ => ChapterClearReward::default(),
    }
}

fn victory_reward_summary(stats: &RunStats, mastery: &ChapterMastery) -> String {
    let primal_cache = if stats.primal_caches > 0 {
        format!(
            " | Primal Cache x{} item{} +{} gold +{} shards +{} essence",
            stats.primal_cache_items_claimed,
            if stats.primal_cache_echo_items > 0 {
                " incl Echo"
            } else {
                ""
            },
            stats.primal_cache_bonus_gold,
            stats.primal_cache_bonus_shards,
            stats.primal_cache_bonus_essence
        )
    } else {
        String::new()
    };
    let soul_sigils = if stats.malrec_soul_sigils_earned > 0 {
        format!(
            " | Soul Sigils +{} (held {}, {})",
            stats.malrec_soul_sigils_earned,
            stats.malrec_soul_sigils,
            soul_sigil_cache_status(stats.malrec_soul_sigils)
        )
    } else if stats.malrec_soul_sigils > 0 {
        format!(
            " | Soul Sigils held {} ({})",
            stats.malrec_soul_sigils,
            soul_sigil_cache_status(stats.malrec_soul_sigils)
        )
    } else {
        String::new()
    };
    let chapter_clear = if stats.chapter_clear_reward_claimed {
        format!(
            " | Clear +{} gold +{} shards +{} essence",
            stats.chapter_clear_bonus_gold,
            stats.chapter_clear_bonus_shards,
            stats.chapter_clear_bonus_essence
        )
    } else {
        String::new()
    };
    format!(
        "Rewards: Chapter +{} gold +{} shards +{} essence{}{}{} | Mastery +{} gold +{} shards +{} essence | Journey {} pts +{} gold +{} shards +{} essence | {} | {} | {} | Total {} gold {} shards {} essence",
        stats.completion_bonus_gold,
        stats.completion_bonus_shards,
        stats.completion_bonus_essence,
        primal_cache,
        soul_sigils,
        chapter_clear,
        mastery.reward_gold,
        mastery.reward_shards,
        mastery.reward_essence,
        stats.journey_score,
        stats.journey_bonus_gold,
        stats.journey_bonus_shards,
        stats.journey_bonus_essence,
        renown_run_summary(stats),
        stash_run_summary(stats),
        altar_run_summary(stats),
        stats.gold,
        stats.ember_shards,
        stats.affix_essence
    )
}

fn victory_reliquary_sigil_summary(
    difficulty: Difficulty,
    stats: &RunStats,
    clear_grade: &str,
    sigil_tier: Option<u32>,
    records: &ChapterRecords,
) -> String {
    let record = format!(
        "best {} clears {} perfect {}",
        records.best_reliquary_sigil_tier,
        records.lifetime_reliquary_sigil_clears,
        records.lifetime_perfect_sigil_clears
    );
    if difficulty != Difficulty::Torment {
        return format!("Reliquary Sigil: unlocks in Torment | {record}");
    }

    let Some(tier) = sigil_tier else {
        return format!("Reliquary Sigil: no inscription | {record}");
    };
    let quality = if tier >= PERFECT_RELIQUARY_SIGIL_TIER {
        " perfect"
    } else {
        ""
    };
    format!(
        "Reliquary Sigil: tier {tier}/12{quality} | {record} | {}",
        reliquary_sigil_next_step(stats, clear_grade, tier)
    )
}

fn reliquary_sigil_next_step(stats: &RunStats, clear_grade: &str, tier: u32) -> String {
    if tier >= 12 {
        return "tier cap reached".to_string();
    }
    let mut goals = Vec::new();
    if clear_grade != "S" {
        goals.push("earn S clear".to_string());
    }
    let boss_break_goal = BOSS_BREAK_TARGET.saturating_mul(2);
    if stats.boss_staggers < boss_break_goal {
        goals.push(format!("break Malrec {boss_break_goal}x"));
    }
    if stats.primal_caches == 0 {
        goals.push("claim Primal Cache".to_string());
    }
    if stats.ashen_threat_surges == 0 {
        goals.push("trigger Ashen surge".to_string());
    }
    if stats.best_massacre_streak < 20 {
        goals.push("20-kill massacre".to_string());
    }
    if stats.best_valor_stacks < 5 {
        goals.push("5 Valor".to_string());
    }
    if tier < PERFECT_RELIQUARY_SIGIL_TIER {
        goals.push("Paragon gem/full codex".to_string());
    }
    if goals.is_empty() {
        "push Paragon gem/full codex for tier 12".to_string()
    } else {
        format!(
            "next {}",
            goals.into_iter().take(3).collect::<Vec<_>>().join(", ")
        )
    }
}

fn soul_sigil_cache_status(sigils: u32) -> String {
    if sigils >= SOUL_SIGIL_CACHE_COST {
        "Trophy Cache ready".to_string()
    } else {
        format!("cache {sigils}/{SOUL_SIGIL_CACHE_COST}")
    }
}

fn renown_run_summary(stats: &RunStats) -> String {
    if !stats.renown_bonus_claimed || stats.renown_rank == 0 {
        return "Renown: none".to_string();
    }
    format!(
        "Renown {} start +{} gold +{} shards +{} essence",
        stats.renown_rank,
        stats.renown_bonus_gold,
        stats.renown_bonus_shards,
        stats.renown_bonus_essence
    )
}

fn stash_run_summary(stats: &RunStats) -> String {
    if !stats.stash_bonus_claimed {
        return "Stash: none".to_string();
    }
    format!(
        "Stash start +{} gold +{} shards +{} essence",
        stats.stash_bonus_gold, stats.stash_bonus_shards, stats.stash_bonus_essence
    )
}

fn altar_run_summary(stats: &RunStats) -> String {
    if !stats.altar_bonus_claimed || stats.altar_seals == 0 {
        return "Altar: none".to_string();
    }
    format!(
        "Altar {}/{} start +{} gold +{} shards +{} essence",
        stats.altar_seals,
        ALL_RELIQUARY_ALTAR_SEALS.len(),
        stats.altar_bonus_gold,
        stats.altar_bonus_shards,
        stats.altar_bonus_essence
    )
}

fn victory_progress_summary(
    challenges: &ChapterChallenges,
    mastery: &ChapterMastery,
    milestones: &Milestones,
) -> String {
    let completed_challenges = challenges
        .challenges
        .iter()
        .filter(|challenge| challenge.completed)
        .count();
    format!(
        "Progress: Challenges {}/{} | Milestones {}/{} | {}",
        completed_challenges,
        challenges.challenges.len(),
        milestones.unlocked.len(),
        ALL_MILESTONES.len(),
        mastery_summary(mastery)
    )
}

fn victory_latest_summary(
    challenges: &ChapterChallenges,
    mastery: &ChapterMastery,
    milestones: &Milestones,
) -> String {
    format!(
        "Latest: {} | {} | {}",
        if challenges.latest.is_empty() {
            "no new challenge".to_string()
        } else {
            challenges.latest.clone()
        },
        if milestones.latest.is_empty() {
            "no new milestone".to_string()
        } else {
            milestones.latest.clone()
        },
        if mastery.latest.is_empty() {
            format!("Mastery: {}", mastery.rank.label())
        } else {
            mastery.latest.clone()
        }
    )
}

#[allow(clippy::too_many_arguments)]
fn victory_pursuit_summary(
    progress: &ChapterProgress,
    stats: &RunStats,
    bounty: &BountyProgress,
    obelisk: &BloodObelisk,
    rift: &EmberRift,
    story: &StoryLog,
    challenges: &ChapterChallenges,
    milestones: &Milestones,
    mastery: &ChapterMastery,
    current_difficulty: Difficulty,
) -> String {
    let optional = optional_objective_score(stats, bounty, obelisk, rift);
    let grade = chapter_clear_grade(progress, optional, story);
    let journey = chapter_journey_snapshot(challenges, milestones, mastery);
    let next_difficulty = escalated_difficulty_after_clear(current_difficulty);
    let route = if next_difficulty == current_difficulty {
        format!("replay {}", current_difficulty.label())
    } else {
        format!("push {}", next_difficulty.label())
    };
    let checklist = post_clear_checklist_summary(stats, current_difficulty);

    if grade != "S" {
        let optional_needed = 8_u32.saturating_sub(optional);
        let story_needed = 10_u32.saturating_sub(story.beats.len() as u32);
        let focus = victory_improvement_focus(stats, bounty, obelisk, rift, story, optional);
        let grade_target = if optional_needed > 0 && story_needed > 0 {
            format!("S needs +{optional_needed} optional and +{story_needed} story beats")
        } else if optional_needed > 0 {
            format!(
                "S needs +{optional_needed} optional objective{}",
                plural_s(optional_needed)
            )
        } else if story_needed > 0 {
            format!(
                "S needs +{story_needed} story beat{}",
                plural_s(story_needed)
            )
        } else {
            "S clear is within reach".to_string()
        };
        return format!(
            "Pursuit: {focus} | {grade_target} | {checklist} | Journey {} | {}",
            journey.next_goal, route
        );
    }

    let tier_target = journey_next_tier_reward_summary(&journey);
    format!(
        "Pursuit: S secured | {tier_target} | {checklist} | {} | {}",
        journey.next_goal, route
    )
}

fn post_clear_checklist_summary(stats: &RunStats, current_difficulty: Difficulty) -> String {
    let upgrade = post_clear_upgrade_summary(stats, current_difficulty);
    let next_difficulty = escalated_difficulty_after_clear(current_difficulty);
    let route = if next_difficulty == current_difficulty {
        format!("replay {}", current_difficulty.label())
    } else {
        format!("push {}", next_difficulty.label())
    };
    format!("Checklist: {upgrade}; then {route}; review quest board")
}

fn victory_command_panel_summary(
    stats: &RunStats,
    records: &ChapterRecords,
    current_difficulty: Difficulty,
) -> String {
    let action = victory_action_prompt(current_difficulty);
    let claim = compact_label(&post_clear_upgrade_summary(stats, current_difficulty), 96);
    let next = escalated_difficulty_after_clear(current_difficulty);
    let route = if next == current_difficulty {
        format!(
            "Replay {} for S clear, Primal Cache, and sigil tiers",
            current_difficulty.label()
        )
    } else {
        format!(
            "Push {} after rewards and quartermaster upgrades",
            next.label()
        )
    };
    let plan = compact_label(&next_run_plan_summary(records, current_difficulty), 104);

    format!("POST-CLEAR COMMAND | {action}\nClaim: {claim}\nNext: {route}\n{plan}")
}

fn post_clear_upgrade_summary(stats: &RunStats, current_difficulty: Difficulty) -> String {
    if let Some(summary) = soul_sigil_cache_ready_summary(stats.malrec_soul_sigils) {
        return format!("Post-clear: open {summary}");
    }
    if stats.primal_caches > 0 {
        return format!(
            "Post-clear: inspect Primal Cache x{} item{}",
            stats.primal_cache_items_claimed,
            if stats.primal_cache_echo_items > 0 {
                " incl Echo"
            } else {
                ""
            }
        );
    }
    if stats.echo_keystones > 0 {
        return format!(
            "Post-clear: hold Echo Keystone x{} for primal infusion",
            stats.echo_keystones
        );
    }
    if current_difficulty == Difficulty::Torment {
        return "Post-clear: improve Torment clear for Primal Cache".to_string();
    }
    if stats.gold >= 180 && stats.ember_shards >= 6 {
        return "Post-clear: temper, socket, or expand at quartermaster".to_string();
    }
    "Post-clear: salvage spares and prep next run".to_string()
}

fn next_run_plan_summary(records: &ChapterRecords, current_difficulty: Difficulty) -> String {
    let target = if current_difficulty == Difficulty::Torment
        && difficulty_unlocked(Difficulty::Torment, records)
    {
        Difficulty::Torment
    } else {
        next_unlocked_difficulty(current_difficulty, records)
    };
    let route = if target == current_difficulty {
        format!("replay {}", current_difficulty.label())
    } else {
        format!("push {}", target.label())
    };
    let stash = profile_stash_starting_bonus(records);
    let stash = if stash.is_empty() {
        "no stash boost".to_string()
    } else {
        format!(
            "start +{}g +{}s +{}e",
            stash.gold, stash.shards, stash.essence
        )
    };
    let seal = next_reliquary_altar_seal(records)
        .map(|seal| format!("altar {}", seal.label()))
        .unwrap_or_else(|| "altar complete".to_string());
    let deed = compact_label(&season_deed_pursuit_summary(records), 42);
    let priority = compact_label(
        &next_run_priority_summary(records, current_difficulty, target),
        34,
    );
    let ordeal = next_run_ordeal_preview_summary(records, target);
    format!("Next Run Plan: {route} | Priority {priority} | {ordeal} | {stash} | {seal} | {deed}")
}

fn next_run_priority_summary(
    records: &ChapterRecords,
    current_difficulty: Difficulty,
    target: Difficulty,
) -> String {
    if records.total_clears() == 0 {
        return "first clear".to_string();
    }
    if target != current_difficulty {
        return format!("push {}", target.label());
    }
    if clear_grade_rank(records.best_chapter_clear_grade_label()) < clear_grade_rank("S") {
        return "earn S clear".to_string();
    }
    if records.best_journey_score < 60 {
        return format!("journey {}/60", records.best_journey_score);
    }
    if let Some(deed) = claimable_season_deed(records) {
        return format!("claim {}", deed.label());
    }
    if let Some(deed) = next_season_deed(records) {
        return format!("deed {}", deed.label());
    }
    if records.best_reliquary_sigil_tier < PERFECT_RELIQUARY_SIGIL_TIER {
        return "perfect Reliquary Sigil".to_string();
    }
    "Torment farming".to_string()
}

fn next_run_ordeal_preview_summary(records: &ChapterRecords, target: Difficulty) -> String {
    let modifier = modifier_for_run(target, records.clears(target));
    let tactic = ordeal_tactic(&modifier);
    let tactic = if tactic.is_empty() {
        "learn route"
    } else {
        tactic
    };
    format!(
        "Ordeal {} [{}] x{:.2}: {}",
        modifier.label(),
        ordeal_pressure_label(&modifier),
        modifier.reward_multiplier(),
        tactic
    )
}

fn ordeal_pressure_label(modifier: &ChapterModifier) -> &'static str {
    let pressure = (modifier.enemy_health_multiplier() - 1.0)
        + (modifier.enemy_damage_multiplier() - 1.0)
        + (modifier.hazard_damage_multiplier() - 1.0);
    if pressure >= 0.85 {
        "lethal"
    } else if pressure >= 0.35 {
        "high"
    } else if pressure > 0.0 {
        "rising"
    } else {
        "baseline"
    }
}

fn victory_improvement_focus(
    stats: &RunStats,
    bounty: &BountyProgress,
    obelisk: &BloodObelisk,
    rift: &EmberRift,
    story: &StoryLog,
    optional: u32,
) -> String {
    if optional < 8 {
        let next_side = next_optional_objective_focus(stats, bounty, obelisk, rift);
        return format!("Focus side events {optional}/9 - {next_side}");
    }
    let story_count = story.beats.len() as u32;
    if story_count < 10 {
        return format!("Focus story beats {story_count}/10 - lore and optional sites");
    }
    "Focus clean route - push for S time and kills".to_string()
}

fn next_optional_objective_focus(
    stats: &RunStats,
    bounty: &BountyProgress,
    obelisk: &BloodObelisk,
    rift: &EmberRift,
) -> String {
    if !bounty.reward_claimed {
        "Bounty Board cache".to_string()
    } else if !stats.champion_pack_reward_claimed {
        "roaming champion pack".to_string()
    } else if stats.breakables_smashed < BREAKER_TARGET_BREAKABLES {
        "smash all breakables".to_string()
    } else if stats.pylon_kills < PYLON_REAPER_KILLS {
        "Ashen Pylon kills".to_string()
    } else if stats.cursed_ambush_kills < CURSED_AMBUSH_KILLS {
        "cursed shrine ambush".to_string()
    } else if stats.elite_affix_kills < AFFIX_HUNTER_KILLS {
        "multi-affix elites".to_string()
    } else if let Some(next) = affix_codex_next_label(stats) {
        format!("hunt {next} elite")
    } else if !stats.shrine_resonance_triggered {
        "chain shrine resonance".to_string()
    } else if obelisk.state != ObeliskState::Completed {
        "complete Blood Obelisk".to_string()
    } else if rift.state != RiftState::Completed {
        "seal Ember Rift".to_string()
    } else {
        "optional route cleanup".to_string()
    }
}

fn plural_s(value: u32) -> &'static str {
    if value == 1 { "" } else { "s" }
}

fn pylon_reaper_summary(stats: &RunStats) -> String {
    format!(
        "Pylon Reaper {}/{}",
        stats.pylon_kills.min(PYLON_REAPER_KILLS),
        PYLON_REAPER_KILLS
    )
}

fn cursed_pact_summary(stats: &RunStats) -> String {
    format!(
        "Cursed Pact {}/{}",
        stats.cursed_ambush_kills.min(CURSED_AMBUSH_KILLS),
        CURSED_AMBUSH_KILLS
    )
}

fn breaker_summary(stats: &RunStats) -> String {
    format!(
        "Breaker {}/{}",
        stats.breakables_smashed.min(BREAKER_TARGET_BREAKABLES),
        BREAKER_TARGET_BREAKABLES
    )
}

fn affix_hunter_summary(stats: &RunStats) -> String {
    format!(
        "Affix Hunter {}/{}",
        stats.elite_affix_kills.min(AFFIX_HUNTER_KILLS),
        AFFIX_HUNTER_KILLS
    )
}

const AFFIX_CODEX_DISPLAY_ORDER: [EnemyAffix; 9] = [
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

fn affix_codex_summary(stats: &RunStats) -> String {
    let known = affix_codex_known_count(stats);
    if known >= AFFIX_CODEX_TARGET {
        return format!("Affix Codex {known}/{AFFIX_CODEX_TARGET} complete");
    }
    let next = affix_codex_next_label(stats).unwrap_or("elite affix");
    format!("Affix Codex {known}/{AFFIX_CODEX_TARGET} next {next}")
}

fn affix_codex_known_count(stats: &RunStats) -> u32 {
    stats.affix_codex_mask.count_ones().min(AFFIX_CODEX_TARGET)
}

fn affix_codex_next_label(stats: &RunStats) -> Option<&'static str> {
    if affix_codex_known_count(stats) >= AFFIX_CODEX_TARGET {
        return None;
    }
    AFFIX_CODEX_DISPLAY_ORDER
        .iter()
        .copied()
        .find(|affix| stats.affix_codex_mask & affix.mask() == 0)
        .map(EnemyAffix::label)
}

fn champion_pack_summary(stats: &RunStats) -> String {
    if stats.champion_pack_reward_claimed {
        return "Champion Pack claimed".to_string();
    }
    format!(
        "Champion Pack {}/{}",
        stats.champion_pack_kills.min(CHAMPION_PACK_TARGET),
        CHAMPION_PACK_TARGET
    )
}

fn nemesis_hunter_summary(stats: &RunStats) -> String {
    format!("Nemesis Hunter {}/1", stats.nemesis_kills.min(1))
}

fn treasure_vault_summary(stats: &RunStats) -> &'static str {
    if stats.treasure_vaults_opened > 0 {
        "Treasure Vault opened"
    } else {
        "Treasure Vault pending"
    }
}

fn ashen_threat_summary(stats: &RunStats) -> String {
    format!(
        "Ashen Threat {}/{} surge {}/{}",
        stats.ashen_threat.min(ASHEN_THREAT_MAX),
        ASHEN_THREAT_MAX,
        stats.ashen_threat_surges.min(ASHEN_THREAT_SURGE_TARGET),
        ASHEN_THREAT_SURGE_TARGET
    )
}

fn rune_weaver_summary(stats: &RunStats) -> String {
    let completed = u32::from(stats.reap_dash_hits > 0)
        + u32::from(stats.hemorrhage_rupture_hits > 0)
        + u32::from(stats.frost_nova_hits > 0);
    if let Some(next) = rune_weaver_next_step(stats) {
        return format!("Rune Weaver {completed}/3: {next}");
    }
    format!("Rune Weaver {completed}/3")
}

fn armory_adept_summary(stats: &RunStats) -> String {
    format!("Armory Adept {}/3", stats.armory_loadouts_saved.min(3))
}

fn portal_wayfarer_summary(stats: &RunStats) -> &'static str {
    if stats.town_portal_returns > 0 {
        "Portal Wayfarer"
    } else {
        "Portal Wayfarer pending"
    }
}

fn glory_seeker_summary(stats: &RunStats) -> String {
    format!(
        "Glory Seeker {}/{}",
        stats.health_globes_collected.min(GLORY_SEEKER_GLOBES),
        GLORY_SEEKER_GLOBES
    )
}

fn last_stand_summary(stats: &RunStats) -> String {
    format!(
        "Last Stand {}/{}",
        stats.last_stand_potions.min(LAST_STAND_POTIONS),
        LAST_STAND_POTIONS
    )
}

fn ashen_alchemy_summary(stats: &RunStats) -> String {
    let completed = u32::from(stats.alchemy_gold_transmutes > 0)
        + u32::from(stats.alchemy_essence_transmutes > 0)
        + u32::from(stats.alchemy_keystone_transmutes > 0);
    format!("Ashen Alchemy {completed}/3")
}

fn loot_curator_summary(stats: &RunStats) -> String {
    format!("Loot Curator {}/3", stats.loot_filter_cycles.min(3))
}

fn codex_adept_summary(stats: &RunStats) -> String {
    let kills = stats.codex_attuned_kills.min(CODEX_ADEPT_KILLS);
    if kills >= CODEX_ADEPT_KILLS {
        return format!("Codex Adept {kills}/{CODEX_ADEPT_KILLS}");
    }
    if kills == 0 {
        return format!("Codex Adept 0/{CODEX_ADEPT_KILLS}: B attune power");
    }
    format!("Codex Adept {kills}/{CODEX_ADEPT_KILLS}: kill with B power")
}

fn set_adept_summary(stats: &RunStats) -> String {
    let kills = stats.set_resonance_kills.min(SET_ADEPT_KILLS);
    if kills >= SET_ADEPT_KILLS {
        return format!("Set Adept {kills}/{SET_ADEPT_KILLS}");
    }
    if kills == 0 {
        return format!("Set Adept 0/{SET_ADEPT_KILLS}: match weapon/charm");
    }
    format!("Set Adept {kills}/{SET_ADEPT_KILLS}: keep set kills")
}

fn gem_adept_summary(stats: &RunStats) -> String {
    let socketed = u32::from(stats.ruby_socketed)
        + u32::from(stats.emerald_socketed)
        + u32::from(stats.amethyst_socketed)
        + u32::from(stats.topaz_socketed);
    format!(
        "Gem Adept {}/{}",
        socketed.min(GEM_ADEPT_KINDS),
        GEM_ADEPT_KINDS
    )
}

fn shrine_resonance_summary(stats: &RunStats) -> &'static str {
    if stats.shrine_resonance_triggered {
        "Shrine Resonance"
    } else {
        "Shrine Resonance pending"
    }
}

fn chapter_route_summary(
    progress: &ChapterProgress,
    stats: &RunStats,
    bounty: &BountyProgress,
    obelisk: &BloodObelisk,
    rift: &EmberRift,
) -> String {
    format!(
        "Route [{}] {:.0}% | Optional {}/9 | Threat {} | {}",
        chapter_route_nodes(progress.phase),
        chapter_mainline_percent(progress),
        optional_objective_score(stats, bounty, obelisk, rift),
        threat_tier(stats),
        chapter_next_reward_hint(progress, bounty, obelisk, rift)
    )
}

fn chapter_quest_log_summary(
    progress: &ChapterProgress,
    stats: &RunStats,
    bounty: &BountyProgress,
    obelisk: &BloodObelisk,
    rift: &EmberRift,
) -> String {
    format!(
        "Quest Log: Main {} | Bounty {} | Side Champion {}, Breaker {}, Obelisk {}, Rift {}",
        main_quest_status(progress, stats),
        bounty_quest_status(bounty),
        progress_fraction_or_done(
            stats.champion_pack_kills,
            CHAMPION_PACK_TARGET,
            stats.champion_pack_reward_claimed
        ),
        progress_fraction_or_done(
            stats.breakables_smashed,
            BREAKER_TARGET_BREAKABLES,
            stats.breakables_smashed >= BREAKER_TARGET_BREAKABLES
        ),
        obelisk_quest_status(obelisk),
        rift_quest_status(rift)
    )
}

fn chapter_quest_board_text(
    progress: &ChapterProgress,
    stats: &RunStats,
    bounty: &BountyProgress,
    obelisk: &BloodObelisk,
    rift: &EmberRift,
    story: &StoryLog,
) -> String {
    let optional = optional_objective_score(stats, bounty, obelisk, rift);
    [
        format!(
            "QUEST BOARD | Main {:.0}% | Optional {}/9 | Story {}/{} | Threat {} | Priority {}",
            chapter_mainline_percent(progress),
            optional,
            story.beats.len(),
            story.total_beats(),
            threat_tier(stats),
            compact_label(
                &chapter_quest_priority(progress, stats, bounty, obelisk, rift, story),
                30
            )
        ),
        format!(
            "Main: {} | Route {} | Now {}",
            compact_label(&main_quest_status(progress, stats), 34),
            chapter_route_nodes_compact(progress.phase),
            compact_label(&progress.objective, 34)
        ),
        format!(
            "Next: {} | Reward: {} | Side: {}",
            chapter_next_step(progress),
            chapter_next_reward(progress, bounty, obelisk, rift),
            compact_label(
                &next_optional_objective_focus(stats, bounty, obelisk, rift),
                28
            )
        ),
        format!(
            "Side: Bounty {} | Champion {} | Breaker {}",
            compact_label(&bounty_quest_status(bounty), 32),
            progress_fraction_or_done(
                stats.champion_pack_kills,
                CHAMPION_PACK_TARGET,
                stats.champion_pack_reward_claimed
            ),
            progress_fraction_or_done(
                stats.breakables_smashed,
                BREAKER_TARGET_BREAKABLES,
                stats.breakables_smashed >= BREAKER_TARGET_BREAKABLES
            )
        ),
        format!(
            "Events: Obelisk {} | Rift {} | {}",
            obelisk_quest_status(obelisk),
            rift_quest_status(rift),
            compact_label(&ashen_threat_summary(stats), 34)
        ),
        format!("Lore: {}", compact_label(&story.recap(), 112)),
    ]
    .join("\n")
}

fn chapter_quest_priority(
    progress: &ChapterProgress,
    stats: &RunStats,
    bounty: &BountyProgress,
    obelisk: &BloodObelisk,
    rift: &EmberRift,
    story: &StoryLog,
) -> String {
    if rift.state == RiftState::Active {
        let echo = if rift.remaining_secs >= crate::rift::RIFT_SWIFT_SEAL_SECS {
            " for Echo"
        } else {
            ""
        };
        return format!(
            "seal rift {}/{}{}",
            rift.kills.min(rift.target_kills),
            rift.target_kills,
            echo
        );
    }
    if obelisk.state == ObeliskState::Active {
        return format!(
            "feed obelisk {}/{}",
            obelisk.kills.min(obelisk.target_kills),
            obelisk.target_kills
        );
    }
    if bounty.completed && !bounty.reward_claimed {
        return "claim bounty cache".to_string();
    }
    if stats.ashen_threat >= 75 && stats.ashen_threat_surges < ASHEN_THREAT_SURGE_TARGET {
        return "stabilize threat surge".to_string();
    }
    if progress.phase != ChapterPhase::Victory && !progress.boss_slain {
        return chapter_next_step(progress);
    }
    let optional = optional_objective_score(stats, bounty, obelisk, rift);
    if optional < 8 {
        return format!(
            "side: {}",
            next_optional_objective_focus(stats, bounty, obelisk, rift)
        );
    }
    if story.beats.len() < 10 {
        return "recover lore beats".to_string();
    }
    "push next difficulty".to_string()
}

fn chapter_next_reward_hint(
    progress: &ChapterProgress,
    bounty: &BountyProgress,
    obelisk: &BloodObelisk,
    rift: &EmberRift,
) -> String {
    format!(
        "Next {} => {}",
        compact_label(&chapter_next_step(progress), 22),
        compact_label(&chapter_next_reward(progress, bounty, obelisk, rift), 30)
    )
}

fn chapter_next_step(progress: &ChapterProgress) -> String {
    match progress.phase {
        ChapterPhase::Breach => format!("slay guards {}/3", progress.enemies_slain.min(3)),
        ChapterPhase::Cache => "open sealed cache".to_string(),
        ChapterPhase::Sanctum => "kill Seal Warden Vhal".to_string(),
        ChapterPhase::Ritual => format!(
            "restore final seal {}/{}",
            progress.seal_fragments.min(REQUIRED_SEAL_FRAGMENTS),
            REQUIRED_SEAL_FRAGMENTS
        ),
        ChapterPhase::Boss => "break and kill Malrec".to_string(),
        ChapterPhase::Victory => "start next difficulty".to_string(),
    }
}

fn chapter_next_reward(
    progress: &ChapterProgress,
    bounty: &BountyProgress,
    obelisk: &BloodObelisk,
    rift: &EmberRift,
) -> String {
    if bounty.completed && !bounty.reward_claimed {
        return format!(
            "bounty {}g {} shards",
            bounty.reward_gold, bounty.reward_shards
        );
    }
    if matches!(obelisk.state, ObeliskState::Active) {
        return "obelisk gold, shards, essence".to_string();
    }
    if matches!(rift.state, RiftState::Active) {
        let echo = if rift.remaining_secs >= crate::rift::RIFT_SWIFT_SEAL_SECS {
            " + Echo"
        } else {
            ""
        };
        return format!("rift gold, shards, essence{echo}");
    }
    chapter_phase_payoff(progress)
}

fn chapter_clear_summary(
    progress: &ChapterProgress,
    stats: &RunStats,
    bounty: &BountyProgress,
    obelisk: &BloodObelisk,
    rift: &EmberRift,
    story: &StoryLog,
) -> String {
    let optional = optional_objective_score(stats, bounty, obelisk, rift);
    let grade = chapter_clear_grade_status(progress, optional, story);
    let s_goal = chapter_s_grade_status(progress, optional, story);
    format!(
        "Chapter Clear: {grade} | Main {:.0}% | Optional {optional}/9 | Story {}/{} | Threat {} | {s_goal}",
        chapter_mainline_percent(progress),
        story.beats.len(),
        story.total_beats(),
        threat_tier(stats)
    )
}

fn victory_scorecard_summary(
    progress: &ChapterProgress,
    stats: &RunStats,
    bounty: &BountyProgress,
    obelisk: &BloodObelisk,
    rift: &EmberRift,
    story: &StoryLog,
) -> String {
    let optional = optional_objective_score(stats, bounty, obelisk, rift);
    let grade = chapter_clear_grade(progress, optional, story);
    let main = if progress.phase == ChapterPhase::Victory || progress.boss_slain {
        "Main cleared".to_string()
    } else {
        format!("Main {:.0}%", chapter_mainline_percent(progress))
    };
    let side = if optional >= 9 {
        "Side complete".to_string()
    } else {
        format!(
            "Side {optional}/9 next {}",
            compact_label(
                &next_optional_objective_focus(stats, bounty, obelisk, rift),
                26
            )
        )
    };
    let story_line = if story.beats.len() >= 10 {
        format!(
            "Story S-ready {}/{}",
            story.beats.len(),
            story.total_beats()
        )
    } else {
        format!(
            "Story {}/{} find lore sites",
            story.beats.len(),
            story.total_beats()
        )
    };
    let threat = if stats.ashen_threat_surges >= ASHEN_THREAT_SURGE_TARGET {
        "Threat mastered".to_string()
    } else {
        format!(
            "Threat {}/{} surge",
            stats.ashen_threat_surges, ASHEN_THREAT_SURGE_TARGET
        )
    };
    let grade_goal = if grade == "S" {
        "S route locked".to_string()
    } else {
        chapter_s_grade_gap(optional, story)
    };

    format!("Scorecard: {main} | {side} | {story_line} | {threat} | {grade_goal}")
}

fn chapter_s_grade_gap(optional: u32, story: &StoryLog) -> String {
    let optional_needed = 8_u32.saturating_sub(optional);
    let story_needed = 10_u32.saturating_sub(story.beats.len() as u32);
    if optional_needed > 0 && story_needed > 0 {
        format!("S needs +{optional_needed} side +{story_needed} story")
    } else if optional_needed > 0 {
        format!("S needs +{optional_needed} side")
    } else if story_needed > 0 {
        format!("S needs +{story_needed} story")
    } else {
        "S route ready".to_string()
    }
}

fn chapter_s_grade_short_gap(optional: u32, story: &StoryLog) -> String {
    let optional_needed = 8_u32.saturating_sub(optional);
    let story_needed = 10_u32.saturating_sub(story.beats.len() as u32);
    if optional_needed > 0 && story_needed > 0 {
        format!("S +{optional_needed} side +{story_needed} story")
    } else if optional_needed > 0 {
        format!("S +{optional_needed} side")
    } else if story_needed > 0 {
        format!("S +{story_needed} story")
    } else {
        "S ready".to_string()
    }
}

fn chapter_s_grade_status(progress: &ChapterProgress, optional: u32, story: &StoryLog) -> String {
    if chapter_projected_clear_grade(optional, story) != "S" {
        return chapter_s_grade_short_gap(optional, story);
    }
    if progress.phase == ChapterPhase::Victory || progress.boss_slain {
        "S secured".to_string()
    } else {
        "S ready".to_string()
    }
}

fn chapter_clear_grade_status(
    progress: &ChapterProgress,
    optional: u32,
    story: &StoryLog,
) -> String {
    let grade = chapter_projected_clear_grade(optional, story);
    if progress.phase == ChapterPhase::Victory || progress.boss_slain {
        grade.to_string()
    } else {
        format!("Projected {grade}")
    }
}

fn chapter_clear_grade(
    progress: &ChapterProgress,
    optional: u32,
    story: &StoryLog,
) -> &'static str {
    if progress.phase != ChapterPhase::Victory && !progress.boss_slain {
        return "In Progress";
    }
    chapter_projected_clear_grade(optional, story)
}

fn chapter_projected_clear_grade(optional: u32, story: &StoryLog) -> &'static str {
    if optional >= 8 && story.beats.len() >= 10 {
        "S"
    } else if optional >= 6 && story.beats.len() >= 7 {
        "A"
    } else if optional >= 3 {
        "B"
    } else {
        "C"
    }
}

fn clear_grade_rank(grade: &str) -> u32 {
    match grade {
        "S" => 4,
        "A" => 3,
        "B" => 2,
        "C" => 1,
        _ => 0,
    }
}

struct ActionGuideInputs<'a> {
    progress: &'a ChapterProgress,
    stats: &'a RunStats,
    boon: &'a ChapterBoon,
    bounty: &'a BountyProgress,
    obelisk: &'a BloodObelisk,
    rift: &'a EmberRift,
    health: &'a Health,
    potions: &'a PotionBelt,
    death_ward: &'a DeathWard,
    surge: &'a SurgeBuff,
    level: &'a PlayerLevel,
    talents: &'a Talents,
    equipment: &'a Equipment,
    companion: &'a CompanionBond,
    boss: Option<BossActionGuideState>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct BossActionGuideState {
    phase_two_started: bool,
    enrage_started: bool,
    enrage_remaining_secs: f32,
    stagger_meter: f32,
    stagger_threshold: f32,
    staggered_remaining_secs: f32,
}

fn action_guide_summary(inputs: ActionGuideInputs) -> String {
    format!(
        "Action Guide: {} | {}",
        action_guide_primary(&inputs),
        action_guide_optional(inputs.stats, inputs.bounty, inputs.obelisk, inputs.rift)
    )
}

fn action_guide_with_ordeal(
    base: String,
    modifier: &ChapterModifier,
    progress: &ChapterProgress,
) -> String {
    if progress.phase == ChapterPhase::Victory || ordeal_tactic(modifier).is_empty() {
        return base;
    }
    format!(
        "{} | Ordeal {}",
        compact_label(&base, 112),
        ordeal_tactic(modifier)
    )
}

struct JourneyActionGuideContext<'a> {
    progress: &'a ChapterProgress,
    health: &'a Health,
    target_info: &'a TargetInfo,
    obelisk: &'a BloodObelisk,
    rift: &'a EmberRift,
    surge: &'a SurgeBuff,
    challenges: &'a ChapterChallenges,
    milestones: &'a Milestones,
    mastery: &'a ChapterMastery,
}

fn action_guide_with_journey(base: String, inputs: JourneyActionGuideContext) -> String {
    if !journey_hint_allowed(&inputs) {
        return base;
    }
    let Some(hint) =
        action_guide_journey_hint(inputs.challenges, inputs.milestones, inputs.mastery)
    else {
        return base;
    };
    format!("{} | {}", compact_label(&base, 108), hint)
}

fn journey_hint_allowed(inputs: &JourneyActionGuideContext) -> bool {
    if matches!(
        inputs.progress.phase,
        ChapterPhase::Boss | ChapterPhase::Victory
    ) {
        return false;
    }
    if inputs.health.current <= inputs.health.max * 0.35 {
        return false;
    }
    if inputs.target_info.visible {
        return false;
    }
    if surge_seconds_remaining(inputs.surge) > 0.0 {
        return false;
    }
    if inputs.obelisk.state == ObeliskState::Active || inputs.rift.state == RiftState::Active {
        return false;
    }
    true
}

fn action_guide_journey_hint(
    challenges: &ChapterChallenges,
    milestones: &Milestones,
    mastery: &ChapterMastery,
) -> Option<String> {
    let snapshot = chapter_journey_snapshot(challenges, milestones, mastery);
    if snapshot.next_goal == "all chapter journey goals complete" {
        return None;
    }
    let goal = snapshot
        .next_goal
        .strip_prefix("pursue ")
        .unwrap_or(&snapshot.next_goal);
    if let Some((tier, threshold)) = snapshot.next_tier {
        return Some(format!(
            "Journey {}/{} {}: {}",
            snapshot.score,
            threshold,
            tier.label(),
            compact_label(goal, 48)
        ));
    }
    Some(format!(
        "Journey {}: {}",
        snapshot.tier.label(),
        compact_label(goal, 56)
    ))
}

fn ordeal_tactic(modifier: &ChapterModifier) -> &'static str {
    match modifier.affix {
        OrdealAffix::AshenHunger => "kite hits, save potion",
        OrdealAffix::GlassRelics => "focus elites, break shields",
        OrdealAffix::TreasureFever => "push side rewards",
        OrdealAffix::CinderVeins => "avoid vents and pools",
        OrdealAffix::None => match modifier.kind {
            crate::ordeal::ChapterModifierKind::AshenEchoes => "",
            crate::ordeal::ChapterModifierKind::BloodTithe => "respect heavier hits",
            crate::ordeal::ChapterModifierKind::Emberstorm => "watch ember vents",
            crate::ordeal::ChapterModifierKind::TormentBrand => "kite lethal pressure",
        },
    }
}

fn action_guide_primary(inputs: &ActionGuideInputs) -> String {
    if inputs.health.current <= inputs.health.max * 0.35 {
        return low_health_action_guide(inputs);
    }
    if surge_seconds_remaining(inputs.surge) > 0.0 {
        return "Surge active: chain kills to extend it".to_string();
    }
    if inputs.progress.phase == ChapterPhase::Boss && inputs.boon.choice == ChapterBoonChoice::None
    {
        return chapter_boon_tactical_prompt(
            inputs.stats,
            inputs.health,
            inputs.potions,
            inputs.equipment,
        );
    }
    if inputs.surge.charge >= inputs.surge.max_charge && inputs.progress.phase == ChapterPhase::Boss
    {
        return "Press Y Surge for Malrec burst".to_string();
    }
    if let Some(boss) = action_guide_boss_pressure(inputs.progress, inputs.boss) {
        return boss;
    }
    if inputs.progress.phase == ChapterPhase::Boss && inputs.stats.reliquary_momentum > 0 {
        return format!(
            "Momentum {}x: spend Q/RMB/E burst on Malrec",
            inputs.stats.reliquary_momentum.min(RELIQUARY_MOMENTUM_MAX)
        );
    }
    if let Some(readiness) = action_guide_boss_readiness(inputs) {
        return readiness;
    }
    if inputs.rift.state == RiftState::Active {
        return format!(
            "Seal Ember Rift: {}/{} kills {:.0}s{}",
            inputs.rift.kills.min(inputs.rift.target_kills),
            inputs.rift.target_kills,
            inputs.rift.remaining_secs.ceil(),
            if inputs.rift.remaining_secs >= crate::rift::RIFT_SWIFT_SEAL_SECS {
                " for Echo"
            } else {
                ""
            }
        );
    }
    if inputs.obelisk.state == ObeliskState::Active {
        return format!(
            "Feed Blood Obelisk: {}/{} kills {:.0}s",
            inputs.obelisk.kills.min(inputs.obelisk.target_kills),
            inputs.obelisk.target_kills,
            inputs.obelisk.remaining_secs.ceil()
        );
    }
    if let Some(talent) = action_guide_talent_pick(inputs.level, inputs.talents) {
        return talent;
    }
    if let Some(crafting) = action_guide_crafting(inputs.stats, inputs.equipment) {
        return crafting;
    }
    if let Some(command) = action_guide_sentinel_command(inputs.progress, inputs.companion) {
        return command;
    }
    if let Some(pursuit) = action_guide_progression_pursuit(inputs.stats, inputs.equipment) {
        return pursuit;
    }
    match inputs.progress.phase {
        ChapterPhase::Breach => {
            format!(
                "Slay reliquary guards {}/3",
                inputs.progress.enemies_slain.min(3)
            )
        }
        ChapterPhase::Cache => "Press Space at the sealed cache".to_string(),
        ChapterPhase::Sanctum => format!(
            "Slay Seal Warden Vhal {}",
            if inputs.stats.seal_warden_slain {
                "done"
            } else {
                "pending"
            }
        ),
        ChapterPhase::Ritual => "Press Space at the ember altar".to_string(),
        ChapterPhase::Boss => {
            if inputs.stats.boss_staggers < BOSS_BREAK_TARGET {
                format!(
                    "Break Malrec {}/{}",
                    inputs.stats.boss_staggers, BOSS_BREAK_TARGET
                )
            } else {
                "Finish Malrec".to_string()
            }
        }
        ChapterPhase::Victory => "Press Space/Enter for next difficulty".to_string(),
    }
}

fn action_guide_boss_readiness(inputs: &ActionGuideInputs) -> Option<String> {
    if inputs.progress.phase != ChapterPhase::Boss
        || inputs.boss.is_some()
        || inputs.boon.choice == ChapterBoonChoice::None
    {
        return None;
    }

    let health_ratio = if inputs.health.max <= 0.0 {
        1.0
    } else {
        inputs.health.current / inputs.health.max
    };
    let mut fixes = Vec::new();
    if health_ratio < 0.65 {
        fixes.push("heal to 65%+".to_string());
    }

    let potion_cooldown = potion_seconds_remaining(inputs.potions);
    if inputs.potions.current == 0 {
        fixes.push("restock potion".to_string());
    } else if potion_cooldown > 0.5 {
        fixes.push(format!("wait potion {:.0}s", potion_cooldown.ceil()));
    }

    if inputs.surge.charge < inputs.surge.max_charge {
        fixes.push(format!(
            "charge Surge {}/{}",
            inputs.surge.charge, inputs.surge.max_charge
        ));
    }

    if inputs.equipment.legendary_power == LegendaryPower::None
        && !matches!(
            inputs.equipment.quality.as_str(),
            "legendary" | "ancient" | "primal"
        )
    {
        fixes.push("equip legendary power".to_string());
    }

    if fixes.is_empty() {
        return None;
    }
    Some(format!("Boss prep: {}", fixes.join(", ")))
}

fn low_health_action_guide(inputs: &ActionGuideInputs) -> String {
    let ward = soul_ward_survival_note(inputs.death_ward, inputs.stats.gold);
    let cooldown = potion_seconds_remaining(inputs.potions);
    if inputs.potions.current > 0 && cooldown <= 0.1 {
        return format!(
            "Low health: F potion +{:.0} HP; {ward}",
            inputs.potions.heal_amount
        );
    }
    if inputs.potions.current > 0 {
        return format!("Low health: potion CD {:.0}s; {ward}", cooldown.ceil());
    }
    format!("Low health: potions empty; {ward}")
}

fn soul_ward_survival_note(ward: &DeathWard, gold: u32) -> String {
    let revives = ward.remaining_revives();
    if revives == 0 {
        return "ward spent".to_string();
    }
    if gold < ward.gold_penalty {
        return format!("ward needs {}g", ward.gold_penalty - gold);
    }
    format!("ward ready {revives}")
}

fn chapter_boon_tactical_prompt(
    stats: &RunStats,
    health: &Health,
    potions: &PotionBelt,
    equipment: &Equipment,
) -> String {
    format!(
        "{} | {}",
        chapter_boon_choice_prompt(stats),
        chapter_boon_build_recommendation(stats, health, potions, equipment)
    )
}

fn chapter_boon_build_recommendation(
    stats: &RunStats,
    health: &Health,
    potions: &PotionBelt,
    equipment: &Equipment,
) -> &'static str {
    match chapter_boon_recommended_choice(stats, health, potions, equipment) {
        Some(ChapterBoonChoice::TitanWard) => "Build rec 5 Titan: survive Malrec",
        Some(ChapterBoonChoice::ArcaneMomentum) => "Build rec 6 Arcane: more skill uptime",
        Some(ChapterBoonChoice::CrimsonOath) => "Build rec 4 Crimson: scale crit power",
        _ => "Build rec choose 4 damage, 5 safety, 6 skills",
    }
}

fn chapter_boon_recommended_key(
    stats: &RunStats,
    health: &Health,
    potions: &PotionBelt,
    equipment: &Equipment,
) -> &'static str {
    match chapter_boon_recommended_choice(stats, health, potions, equipment) {
        Some(ChapterBoonChoice::CrimsonOath) => "4 Crimson",
        Some(ChapterBoonChoice::TitanWard) => "5 Titan",
        Some(ChapterBoonChoice::ArcaneMomentum) => "6 Arcane",
        _ => "best fit",
    }
}

fn chapter_boon_recommended_choice(
    stats: &RunStats,
    health: &Health,
    potions: &PotionBelt,
    equipment: &Equipment,
) -> Option<ChapterBoonChoice> {
    let health_ratio = if health.max <= 0.0 {
        1.0
    } else {
        health.current / health.max
    };
    let toughness = equipment.health_bonus + equipment.armor_bonus;
    if health_ratio <= 0.55 || potions.current == 0 || toughness < 18.0 && stats.potions_used > 0 {
        return Some(ChapterBoonChoice::TitanWard);
    }
    if reliquary_boon_momentum_ready(stats)
        || stats.surge_kills >= 2
        || stats.echo_keystones > 0
        || stats.pylon_kills > 0
    {
        return Some(ChapterBoonChoice::ArcaneMomentum);
    }
    match equipment.legendary_power {
        LegendaryPower::Soulreaver | LegendaryPower::Aegisbrand => {
            return Some(ChapterBoonChoice::TitanWard);
        }
        LegendaryPower::Frostbrand => {
            return Some(ChapterBoonChoice::ArcaneMomentum);
        }
        LegendaryPower::Emberbrand | LegendaryPower::Stormbrand => {
            return Some(ChapterBoonChoice::CrimsonOath);
        }
        LegendaryPower::None => {}
    }
    if equipment.crit_chance >= 0.12 {
        return Some(ChapterBoonChoice::CrimsonOath);
    }
    None
}

fn action_guide_talent_pick(level: &PlayerLevel, talents: &Talents) -> Option<String> {
    if talents.points == 0 {
        return None;
    }
    Some(format!(
        "Talent ready: K then {} ({}) | L{}",
        next_talent_key(talents),
        next_talent_pick_summary(talents),
        level.level
    ))
}

fn next_talent_key(talents: &Talents) -> &'static str {
    if talents.wrath <= talents.vigor && talents.wrath <= talents.focus {
        "1 Wrath"
    } else if talents.focus <= talents.vigor {
        "3 Focus"
    } else {
        "2 Vigor"
    }
}

fn action_guide_boss_pressure(
    progress: &ChapterProgress,
    boss: Option<BossActionGuideState>,
) -> Option<String> {
    if progress.phase != ChapterPhase::Boss {
        return None;
    }
    let boss = boss?;
    if boss.staggered_remaining_secs > 0.0 {
        return Some(format!(
            "Execute window: Y/Q/E burst Malrec {:.0}s",
            boss.staggered_remaining_secs.ceil()
        ));
    }
    if boss.enrage_started {
        return Some("Ashen Enrage: kite vents, burst after slam".to_string());
    }
    if boss.phase_two_started && boss.enrage_remaining_secs <= 6.0 {
        return Some(format!(
            "Enrage soon: Q/E break {}/{} before fire",
            boss.stagger_meter.round(),
            boss.stagger_threshold.round()
        ));
    }
    if boss.phase_two_started {
        return Some(format!(
            "Phase two: build stagger {}/{}; hold Y",
            boss.stagger_meter.round(),
            boss.stagger_threshold.round()
        ));
    }
    None
}

fn action_guide_sentinel_command(
    progress: &ChapterProgress,
    companion: &CompanionBond,
) -> Option<String> {
    if companion.command_cooldown > 0.0 {
        return None;
    }
    if !matches!(
        progress.phase,
        ChapterPhase::Breach | ChapterPhase::Sanctum | ChapterPhase::Boss
    ) {
        return None;
    }
    let role = match companion.stance {
        CompanionStance::Vanguard => "burst",
        CompanionStance::Guardian => "barrier",
        CompanionStance::Seeker => "sweep",
    };
    Some(format!(
        "Sentinel command ready: H {} {}",
        companion.stance.label(),
        role
    ))
}

fn action_guide_crafting(stats: &RunStats, equipment: &Equipment) -> Option<String> {
    let ready_caches = soul_sigil_cache_ready_count(stats.malrec_soul_sigils);
    if ready_caches > 0 {
        let cache_plural = if ready_caches == 1 { "" } else { "s" };
        return Some(format!(
            "Trophy cache ready: T then Space at quartermaster ({} cache{} ready)",
            ready_caches, cache_plural
        ));
    }
    if primal_infusion_pursuit_summary(stats, equipment).starts_with("Primal ready") {
        return Some("Primal ready: T then Space at quartermaster".to_string());
    }
    None
}

fn action_guide_progression_pursuit(stats: &RunStats, equipment: &Equipment) -> Option<String> {
    if stats.massacre_timer_secs > 0.0
        && stats.massacre_streak > 0
        && stats.best_massacre_streak < CARNAGE_MASTER_STREAK
    {
        return Some(format!(
            "Pursuit: keep Massacre {}/{} - kill before {:.0}s",
            stats.massacre_streak.min(CARNAGE_MASTER_STREAK),
            CARNAGE_MASTER_STREAK,
            stats.massacre_timer_secs.ceil()
        ));
    }
    if stats.valor_timer_secs > 0.0 && stats.valor_stacks > 0 && stats.best_valor_stacks < 5 {
        return Some(format!(
            "Pursuit: chain elite Valor {}/5 - hunt champion/affix",
            stats.valor_stacks.min(5)
        ));
    }
    let rune_count = rune_weaver_progress(stats);
    if rune_count > 0 && rune_count < 3 {
        return Some(format!(
            "Pursuit: Rune Weaver {rune_count}/3 - {}",
            rune_weaver_next_step(stats).unwrap_or("land missing alt rune hits")
        ));
    }
    if equipment.legendary_power != LegendaryPower::None
        && stats.codex_attuned_kills > 0
        && stats.codex_attuned_kills < CODEX_ADEPT_KILLS
    {
        return Some(format!(
            "Pursuit: Codex Adept {}/{} - B attune, kill with {}",
            stats.codex_attuned_kills,
            CODEX_ADEPT_KILLS,
            equipment.legendary_power.label(),
        ));
    }
    if stats.set_resonance_kills > 0 && stats.set_resonance_kills < SET_ADEPT_KILLS {
        return Some(format!(
            "Pursuit: Set Adept {}/{} - keep resonance kills",
            stats.set_resonance_kills, SET_ADEPT_KILLS
        ));
    }
    None
}

fn rune_weaver_progress(stats: &RunStats) -> u32 {
    u32::from(stats.reap_dash_hits > 0)
        + u32::from(stats.hemorrhage_rupture_hits > 0)
        + u32::from(stats.frost_nova_hits > 0)
}

fn rune_weaver_next_step(stats: &RunStats) -> Option<&'static str> {
    if stats.reap_dash_hits == 0 {
        Some("Z Reap then Q hit")
    } else if stats.hemorrhage_rupture_hits == 0 {
        Some("C Hemorrhage then RMB hit")
    } else if stats.frost_nova_hits == 0 {
        Some("X Frost then E hit")
    } else {
        None
    }
}

fn action_guide_optional(
    stats: &RunStats,
    bounty: &BountyProgress,
    obelisk: &BloodObelisk,
    rift: &EmberRift,
) -> String {
    if bounty.completed && !bounty.reward_claimed {
        return "Optional: bounty cache claimed automatically".to_string();
    }
    if !bounty.completed {
        return format!(
            "Optional: {} {}/{}",
            bounty.kind.objective_label(),
            bounty.current.min(bounty.target),
            bounty.target
        );
    }
    if !stats.champion_pack_reward_claimed {
        return format!(
            "Optional: champion pack {}/{}",
            stats.champion_pack_kills.min(CHAMPION_PACK_TARGET),
            CHAMPION_PACK_TARGET
        );
    }
    if stats.breakables_smashed < BREAKER_TARGET_BREAKABLES {
        return format!(
            "Optional: smash breakables {}/{}",
            stats.breakables_smashed.min(BREAKER_TARGET_BREAKABLES),
            BREAKER_TARGET_BREAKABLES
        );
    }
    if obelisk.state == ObeliskState::Active {
        return format!(
            "Optional: Obelisk {}/{} {:.0}s",
            obelisk.kills.min(obelisk.target_kills),
            obelisk.target_kills,
            obelisk.remaining_secs.ceil()
        );
    }
    if rift.state == RiftState::Active {
        return action_guide_optional_rift(rift);
    }
    if stats.pylon_kills < PYLON_REAPER_KILLS {
        return format!(
            "Optional: Ashen Pylon kills {}/{}",
            stats.pylon_kills.min(PYLON_REAPER_KILLS),
            PYLON_REAPER_KILLS
        );
    }
    if stats.cursed_ambush_kills < CURSED_AMBUSH_KILLS {
        return format!(
            "Optional: cursed shrine ambush {}/{}",
            stats.cursed_ambush_kills.min(CURSED_AMBUSH_KILLS),
            CURSED_AMBUSH_KILLS
        );
    }
    if stats.elite_affix_kills < AFFIX_HUNTER_KILLS {
        return format!(
            "Optional: multi-affix elites {}/{}",
            stats.elite_affix_kills.min(AFFIX_HUNTER_KILLS),
            AFFIX_HUNTER_KILLS
        );
    }
    if let Some(next) = affix_codex_next_label(stats) {
        return format!(
            "Optional: Affix Codex {}/{} hunt {next}",
            affix_codex_known_count(stats),
            AFFIX_CODEX_TARGET
        );
    }
    if !stats.shrine_resonance_triggered {
        return "Optional: chain shrine resonance".to_string();
    }
    if obelisk.state == ObeliskState::Dormant {
        return "Optional: awaken Blood Obelisk".to_string();
    }
    if rift.state == RiftState::Sealed {
        return "Optional: open Ember Rift".to_string();
    }
    if rift.state == RiftState::Completed {
        return action_guide_optional_rift(rift);
    }
    if obelisk.state == ObeliskState::Failed {
        return "Optional: Obelisk failed, push Rift".to_string();
    }
    if rift.state == RiftState::Failed {
        return "Optional: Rift failed, push boss".to_string();
    }
    if stats.reliquary_momentum > 0 {
        return format!(
            "Optional: Momentum {}/{} ready for boss",
            stats.reliquary_momentum.min(RELIQUARY_MOMENTUM_MAX),
            RELIQUARY_MOMENTUM_MAX
        );
    }
    "Optional: side events claimed, push mastery".to_string()
}

fn action_guide_optional_rift(rift: &EmberRift) -> String {
    match rift.state {
        RiftState::Active => format!(
            "Optional: Rift {}/{} {:.0}s{}",
            rift.kills.min(rift.target_kills),
            rift.target_kills,
            rift.remaining_secs.ceil(),
            if rift.remaining_secs >= crate::rift::RIFT_SWIFT_SEAL_SECS {
                " Echo"
            } else {
                ""
            }
        ),
        RiftState::Completed => {
            if rift.best_remaining_secs >= crate::rift::RIFT_SWIFT_SEAL_SECS {
                "Optional: Rift swift sealed".to_string()
            } else {
                "Optional: Rift sealed".to_string()
            }
        }
        RiftState::Failed => "Optional: Rift failed, push boss".to_string(),
        RiftState::Sealed => "Optional: open Ember Rift".to_string(),
    }
}

fn main_quest_status(progress: &ChapterProgress, stats: &RunStats) -> String {
    main_quest_summary(progress, stats)
}

fn bounty_quest_status(bounty: &BountyProgress) -> String {
    if bounty.reward_claimed {
        return "claimed".to_string();
    }
    if bounty.completed {
        return "cache ready".to_string();
    }
    format!(
        "{} {}/{}",
        bounty.kind.objective_label(),
        bounty.current.min(bounty.target),
        bounty.target
    )
}

fn progress_fraction_or_done(current: u32, target: u32, done: bool) -> String {
    if done {
        "done".to_string()
    } else {
        format!("{}/{}", current.min(target), target)
    }
}

fn obelisk_quest_status(obelisk: &BloodObelisk) -> String {
    match obelisk.state {
        ObeliskState::Dormant => "dormant".to_string(),
        ObeliskState::Active => format!(
            "{}/{} kills {:.0}s",
            obelisk.kills.min(obelisk.target_kills),
            obelisk.target_kills,
            obelisk.remaining_secs.max(0.0)
        ),
        ObeliskState::Completed => "done".to_string(),
        ObeliskState::Failed => "failed".to_string(),
    }
}

fn rift_quest_status(rift: &EmberRift) -> String {
    match rift.state {
        RiftState::Sealed => "sealed".to_string(),
        RiftState::Active => format!(
            "{}/{} invaders {:.0}s",
            rift.kills.min(rift.target_kills),
            rift.target_kills,
            rift.remaining_secs.max(0.0)
        ),
        RiftState::Completed => {
            if rift.best_remaining_secs >= crate::rift::RIFT_SWIFT_SEAL_SECS {
                "swift seal".to_string()
            } else {
                "done".to_string()
            }
        }
        RiftState::Failed => "failed".to_string(),
    }
}

fn chapter_route_nodes(phase: ChapterPhase) -> String {
    [
        (ChapterPhase::Breach, "Breach"),
        (ChapterPhase::Cache, "Cache"),
        (ChapterPhase::Sanctum, "Sanctum"),
        (ChapterPhase::Ritual, "Ritual"),
        (ChapterPhase::Boss, "Keeper"),
        (ChapterPhase::Victory, "Cleansed"),
    ]
    .iter()
    .map(|(node_phase, label)| {
        if *node_phase == phase {
            format!(">{label}<")
        } else {
            (*label).to_string()
        }
    })
    .collect::<Vec<_>>()
    .join(" -> ")
}

fn chapter_route_nodes_compact(phase: ChapterPhase) -> String {
    [
        (ChapterPhase::Breach, "B"),
        (ChapterPhase::Cache, "C"),
        (ChapterPhase::Sanctum, "S"),
        (ChapterPhase::Ritual, "R"),
        (ChapterPhase::Boss, "K"),
    ]
    .iter()
    .map(|(node_phase, label)| {
        if *node_phase == phase {
            format!("[{label}]")
        } else {
            (*label).to_string()
        }
    })
    .collect::<Vec<_>>()
    .join(">")
}

fn chapter_mainline_percent(progress: &ChapterProgress) -> f32 {
    let phase_base = match progress.phase {
        ChapterPhase::Breach => 0.0,
        ChapterPhase::Cache => 20.0,
        ChapterPhase::Sanctum => 40.0,
        ChapterPhase::Ritual => 60.0,
        ChapterPhase::Boss => 80.0,
        ChapterPhase::Victory => 100.0,
    };
    let phase_fill = match progress.phase {
        ChapterPhase::Breach => progress.enemies_slain.min(3) as f32 / 3.0,
        ChapterPhase::Cache => {
            if progress.chest_opened {
                1.0
            } else {
                0.0
            }
        }
        ChapterPhase::Sanctum => progress.enemies_slain.saturating_sub(3).min(2) as f32 / 2.0,
        ChapterPhase::Ritual => {
            progress.seal_fragments.min(REQUIRED_SEAL_FRAGMENTS) as f32
                / REQUIRED_SEAL_FRAGMENTS as f32
        }
        ChapterPhase::Boss => {
            if progress.boss_slain {
                1.0
            } else {
                0.0
            }
        }
        ChapterPhase::Victory => 0.0,
    };
    (phase_base + phase_fill * 20.0).clamp(0.0, 100.0)
}

fn optional_objective_score(
    stats: &RunStats,
    bounty: &BountyProgress,
    obelisk: &BloodObelisk,
    rift: &EmberRift,
) -> u32 {
    u32::from(bounty.reward_claimed)
        + u32::from(stats.champion_pack_reward_claimed)
        + u32::from(stats.breakables_smashed >= BREAKER_TARGET_BREAKABLES)
        + u32::from(stats.pylon_kills >= PYLON_REAPER_KILLS)
        + u32::from(stats.cursed_ambush_kills >= CURSED_AMBUSH_KILLS)
        + u32::from(stats.elite_affix_kills >= AFFIX_HUNTER_KILLS)
        + u32::from(stats.shrine_resonance_triggered)
        + u32::from(obelisk.state == ObeliskState::Completed)
        + u32::from(rift.state == RiftState::Completed)
}

fn threat_tier(stats: &RunStats) -> &'static str {
    if stats.ashen_threat_surges >= ASHEN_THREAT_SURGE_TARGET {
        "surge mastered"
    } else if stats.ashen_threat >= 75 {
        "surging"
    } else if stats.ashen_threat >= 40 {
        "rising"
    } else {
        "low"
    }
}

fn spawn_hud(mut commands: Commands) {
    commands
        .spawn((
            Node {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                ..default()
            },
            HudUi,
        ))
        .with_children(|parent| {
            parent.spawn((
                Node {
                    position_type: PositionType::Absolute,
                    left: Val::Px(0.0),
                    right: Val::Px(0.0),
                    top: Val::Px(0.0),
                    bottom: Val::Px(0.0),
                    width: Val::Percent(100.0),
                    height: Val::Percent(100.0),
                    ..default()
                },
                BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.0)),
                Visibility::Hidden,
                LowHealthVignette,
            ));
            spawn_bottom_console_backdrop(parent);
            parent.spawn((
                Node {
                    position_type: PositionType::Absolute,
                    left: Val::Percent(50.0),
                    top: Val::Px(8.0),
                    width: Val::Px(700.0),
                    min_height: Val::Px(24.0),
                    margin: UiRect::left(Val::Px(-350.0)),
                    padding: UiRect::axes(Val::Px(8.0), Val::Px(3.0)),
                    justify_content: JustifyContent::Center,
                    align_items: AlignItems::Center,
                    border: UiRect::all(Val::Px(1.0)),
                    overflow: Overflow::clip(),
                    ..default()
                },
                BorderColor::all(Color::srgba(0.78, 0.62, 0.38, 0.38)),
                BackgroundColor(Color::srgba(0.018, 0.014, 0.018, 0.34)),
                Text::new("Objective"),
                TextFont {
                    font_size: FontSize::Px(9.0),
                    ..default()
                },
                TextColor(Color::srgb(0.92, 0.82, 0.68)),
                ObjectiveText,
            ));
            parent
                .spawn((
                    Node {
                        position_type: PositionType::Absolute,
                        top: Val::Px(38.0),
                        left: Val::Percent(50.0),
                        width: Val::Px(560.0),
                        height: Val::Px(56.0),
                        margin: UiRect::left(Val::Px(-280.0)),
                        padding: UiRect::axes(Val::Px(12.0), Val::Px(6.0)),
                        border: UiRect::all(Val::Px(2.0)),
                        display: Display::None,
                        overflow: Overflow::clip(),
                        justify_content: JustifyContent::Center,
                        align_items: AlignItems::Center,
                        ..default()
                    },
                    BorderColor::all(Color::srgba(0.95, 0.34, 0.08, 0.88)),
                    BackgroundColor(Color::srgba(0.026, 0.008, 0.006, 0.90)),
                    BossHudPanel,
                ))
                .with_children(|boss| {
                    boss.spawn((
                        Node {
                            position_type: PositionType::Absolute,
                            left: Val::Px(12.0),
                            right: Val::Px(12.0),
                            bottom: Val::Px(9.0),
                            width: Val::Percent(100.0),
                            height: Val::Px(10.0),
                            ..default()
                        },
                        BackgroundColor(Color::srgba(0.92, 0.18, 0.05, 0.86)),
                        BossHudFill,
                    ));
                    boss.spawn((
                        Node {
                            position_type: PositionType::Absolute,
                            left: Val::Px(12.0),
                            right: Val::Px(12.0),
                            bottom: Val::Px(4.0),
                            width: Val::Percent(0.0),
                            height: Val::Px(4.0),
                            ..default()
                        },
                        BackgroundColor(Color::srgba(1.0, 0.78, 0.18, 0.92)),
                        BossHudStaggerFill,
                    ));
                    boss.spawn((
                        Text::new(""),
                        TextFont {
                            font_size: FontSize::Px(11.5),
                            ..default()
                        },
                        TextColor(Color::srgb(1.0, 0.78, 0.56)),
                        BossHudText,
                    ));
                });
            parent.spawn((
                Node {
                    position_type: PositionType::Absolute,
                    top: Val::Px(126.0),
                    left: Val::Percent(50.0),
                    width: Val::Px(700.0),
                    margin: UiRect::left(Val::Px(-350.0)),
                    justify_content: JustifyContent::Center,
                    ..default()
                },
                Text::new(""),
                TextFont {
                    font_size: FontSize::Px(13.0),
                    ..default()
                },
                TextColor(Color::srgb(0.98, 0.78, 0.36)),
                BuffBarText,
            ));
            parent
                .spawn((
                    Node {
                        position_type: PositionType::Absolute,
                        left: Val::Percent(50.0),
                        bottom: Val::Px(140.0),
                        width: Val::Px(640.0),
                        height: Val::Px(12.0),
                        margin: UiRect::left(Val::Px(-320.0)),
                        padding: UiRect::all(Val::Px(2.0)),
                        border: UiRect::all(Val::Px(1.0)),
                        overflow: Overflow::clip(),
                        ..default()
                    },
                    BorderColor::all(Color::srgba(0.80, 0.58, 0.30, 0.78)),
                    BackgroundColor(Color::srgba(0.018, 0.014, 0.018, 0.92)),
                ))
                .with_children(|bar| {
                    bar.spawn((
                        Node {
                            width: Val::Percent(0.0),
                            height: Val::Percent(100.0),
                            ..default()
                        },
                        BackgroundColor(Color::srgba(0.86, 0.46, 0.10, 0.92)),
                        ExperienceFill,
                    ));
                });
            parent.spawn((
                Node {
                    position_type: PositionType::Absolute,
                    left: Val::Percent(50.0),
                    bottom: Val::Px(116.0),
                    width: Val::Px(820.0),
                    height: Val::Px(22.0),
                    margin: UiRect::left(Val::Px(-410.0)),
                    padding: UiRect::axes(Val::Px(10.0), Val::Px(3.0)),
                    border: UiRect::all(Val::Px(1.0)),
                    overflow: Overflow::clip(),
                    justify_content: JustifyContent::Center,
                    align_items: AlignItems::Center,
                    ..default()
                },
                BorderColor::all(Color::srgba(0.82, 0.62, 0.34, 0.64)),
                BackgroundColor(Color::srgba(0.018, 0.015, 0.018, 0.86)),
                Text::new(""),
                TextFont {
                    font_size: FontSize::Px(10.5),
                    ..default()
                },
                TextColor(Color::srgb(0.90, 0.82, 0.66)),
                ActionBeltText,
            ));
            parent
                .spawn((Node {
                    position_type: PositionType::Absolute,
                    left: Val::Percent(50.0),
                    bottom: Val::Px(154.0),
                    width: Val::Px(632.0),
                    height: Val::Px(50.0),
                    margin: UiRect::left(Val::Px(-316.0)),
                    display: Display::Flex,
                    column_gap: Val::Px(6.0),
                    justify_content: JustifyContent::Center,
                    align_items: AlignItems::Center,
                    ..default()
                },))
                .with_children(|strip| {
                    for slot in [
                        CompactCharacterText::Weapon,
                        CompactCharacterText::Charm,
                        CompactCharacterText::Inventory,
                        CompactCharacterText::Codex,
                    ] {
                        strip.spawn((
                            Node {
                                width: Val::Px(72.0),
                                height: Val::Px(44.0),
                                padding: UiRect::axes(Val::Px(3.0), Val::Px(3.0)),
                                border: UiRect::all(Val::Px(1.0)),
                                border_radius: BorderRadius::all(Val::Px(5.0)),
                                align_items: AlignItems::Center,
                                justify_content: JustifyContent::Center,
                                overflow: Overflow::clip(),
                                ..default()
                            },
                            BorderColor::all(compact_character_border(slot)),
                            BackgroundColor(Color::srgba(0.018, 0.017, 0.020, 0.82)),
                            Text::new(""),
                            TextFont {
                                font_size: FontSize::Px(8.5),
                                ..default()
                            },
                            TextColor(Color::srgb(0.84, 0.78, 0.66)),
                            slot,
                        ));
                    }
                    for slot in [
                        CompactUtilityText::Potion,
                        CompactUtilityText::Elixir,
                        CompactUtilityText::Portal,
                        CompactUtilityText::Interact,
                    ] {
                        strip.spawn((
                            Node {
                                width: Val::Px(72.0),
                                height: Val::Px(44.0),
                                padding: UiRect::axes(Val::Px(3.0), Val::Px(3.0)),
                                border: UiRect::all(Val::Px(1.0)),
                                border_radius: BorderRadius::all(Val::Px(5.0)),
                                align_items: AlignItems::Center,
                                justify_content: JustifyContent::Center,
                                overflow: Overflow::clip(),
                                ..default()
                            },
                            BorderColor::all(compact_utility_border(slot)),
                            BackgroundColor(Color::srgba(0.018, 0.017, 0.020, 0.82)),
                            Text::new(""),
                            TextFont {
                                font_size: FontSize::Px(8.5),
                                ..default()
                            },
                            TextColor(Color::srgb(0.84, 0.78, 0.66)),
                            slot,
                        ));
                    }
                });
            parent
                .spawn((
                    Node {
                        position_type: PositionType::Absolute,
                        left: Val::Px(28.0),
                        bottom: Val::Px(18.0),
                        width: Val::Px(170.0),
                        height: Val::Px(170.0),
                        padding: UiRect::all(Val::Px(12.0)),
                        border: UiRect::all(Val::Px(3.0)),
                        overflow: Overflow::clip(),
                        border_radius: BorderRadius::MAX,
                        justify_content: JustifyContent::Center,
                        align_items: AlignItems::Center,
                        ..default()
                    },
                    BorderColor::all(Color::srgba(0.75, 0.10, 0.08, 0.82)),
                    BackgroundColor(Color::srgba(0.040, 0.006, 0.008, 0.94)),
                    ResourceOrbFrame::Health,
                ))
                .with_children(|panel| {
                    panel.spawn((
                        Node {
                            position_type: PositionType::Absolute,
                            left: Val::Px(0.0),
                            right: Val::Px(0.0),
                            bottom: Val::Px(0.0),
                            width: Val::Percent(100.0),
                            height: Val::Percent(100.0),
                            ..default()
                        },
                        BackgroundColor(Color::srgba(0.70, 0.015, 0.020, 0.92)),
                        ResourceFill::Health,
                    ));
                    panel.spawn((
                        Node {
                            position_type: PositionType::Absolute,
                            left: Val::Px(28.0),
                            right: Val::Px(28.0),
                            bottom: Val::Px(18.0),
                            width: Val::Percent(70.0),
                            height: Val::Px(8.0),
                            ..default()
                        },
                        BackgroundColor(Color::srgba(0.96, 0.78, 0.24, 0.92)),
                        ResourceFill::Barrier,
                    ));
                    panel.spawn((
                        Node {
                            position_type: PositionType::Absolute,
                            left: Val::Px(16.0),
                            right: Val::Px(16.0),
                            top: Val::Px(26.0),
                            bottom: Val::Px(22.0),
                            justify_content: JustifyContent::Center,
                            align_items: AlignItems::Center,
                            ..default()
                        },
                        Text::new(""),
                        TextFont {
                            font_size: FontSize::Px(15.0),
                            ..default()
                        },
                        TextColor(Color::srgb(1.0, 0.84, 0.72)),
                        HealthText,
                    ));
                    panel
                        .spawn((Node {
                            position_type: PositionType::Absolute,
                            left: Val::Px(22.0),
                            right: Val::Px(22.0),
                            bottom: Val::Px(12.0),
                            height: Val::Px(16.0),
                            column_gap: Val::Px(4.0),
                            justify_content: JustifyContent::Center,
                            align_items: AlignItems::Center,
                            ..default()
                        },))
                        .with_children(|pips| {
                            for index in 0..8 {
                                pips.spawn((
                                    Node {
                                        width: Val::Px(11.0),
                                        height: Val::Px(15.0),
                                        border: UiRect::all(Val::Px(1.0)),
                                        ..default()
                                    },
                                    BorderColor::all(Color::srgba(0.50, 0.12, 0.10, 0.72)),
                                    BackgroundColor(Color::srgba(0.48, 0.02, 0.02, 0.82)),
                                    PotionPip(index),
                                ));
                            }
                        });
                });
            parent
                .spawn((
                    Node {
                        position_type: PositionType::Absolute,
                        left: Val::Percent(50.0),
                        bottom: Val::Px(16.0),
                        width: Val::Px(606.0),
                        height: Val::Px(96.0),
                        margin: UiRect::left(Val::Px(-303.0)),
                        padding: UiRect::all(Val::Px(8.0)),
                        border: UiRect::all(Val::Px(2.0)),
                        display: Display::Flex,
                        column_gap: Val::Px(8.0),
                        justify_content: JustifyContent::Center,
                        align_items: AlignItems::Center,
                        ..default()
                    },
                    BorderColor::all(Color::srgba(0.76, 0.60, 0.36, 0.78)),
                    BackgroundColor(Color::srgba(0.025, 0.022, 0.030, 0.90)),
                ))
                .with_children(|bar| {
                    for slot in [
                        SkillSlotText::Strike,
                        SkillSlotText::Rupture,
                        SkillSlotText::Dash,
                        SkillSlotText::Nova,
                        SkillSlotText::Surge,
                        SkillSlotText::Evade,
                    ] {
                        let theme = skill_slot_theme(slot);
                        bar.spawn((
                            Node {
                                width: Val::Px(90.0),
                                height: Val::Px(78.0),
                                padding: UiRect::all(Val::Px(4.0)),
                                border: UiRect::all(Val::Px(2.0)),
                                border_radius: BorderRadius::all(Val::Px(6.0)),
                                justify_content: JustifyContent::Center,
                                align_items: AlignItems::Center,
                                overflow: Overflow::clip(),
                                ..default()
                            },
                            BorderColor::all(Color::srgba(0.92, 0.74, 0.42, 0.62)),
                            BackgroundColor(Color::srgba(0.055, 0.047, 0.058, 0.92)),
                            SkillSlotFrame(slot),
                        ))
                        .with_children(|slot_node| {
                            slot_node.spawn((
                                Node {
                                    position_type: PositionType::Absolute,
                                    left: Val::Px(0.0),
                                    right: Val::Px(0.0),
                                    bottom: Val::Px(0.0),
                                    height: Val::Percent(0.0),
                                    ..default()
                                },
                                BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.62)),
                                SkillCooldownMask(slot),
                            ));
                            slot_node.spawn((
                                Node {
                                    position_type: PositionType::Absolute,
                                    left: Val::Px(5.0),
                                    top: Val::Px(4.0),
                                    width: Val::Px(25.0),
                                    height: Val::Px(16.0),
                                    border: UiRect::all(Val::Px(1.0)),
                                    justify_content: JustifyContent::Center,
                                    align_items: AlignItems::Center,
                                    ..default()
                                },
                                BorderColor::all(theme.idle_border),
                                BackgroundColor(Color::srgba(0.012, 0.010, 0.012, 0.78)),
                                Text::new(skill_slot_keycap(slot)),
                                TextFont {
                                    font_size: FontSize::Px(skill_slot_keycap_font_size(slot)),
                                    ..default()
                                },
                                TextColor(Color::srgb(0.96, 0.90, 0.76)),
                            ));
                            slot_node.spawn((
                                Node {
                                    position_type: PositionType::Absolute,
                                    right: Val::Px(6.0),
                                    top: Val::Px(7.0),
                                    width: Val::Px(10.0),
                                    height: Val::Px(5.0),
                                    ..default()
                                },
                                BackgroundColor(Color::srgba(0.22, 0.18, 0.12, 0.74)),
                                SkillSlotLamp(slot),
                            ));
                            slot_node.spawn((
                                Node {
                                    position_type: PositionType::Absolute,
                                    left: Val::Px(0.0),
                                    right: Val::Px(0.0),
                                    top: Val::Px(8.0),
                                    height: Val::Px(32.0),
                                    justify_content: JustifyContent::Center,
                                    align_items: AlignItems::Center,
                                    ..default()
                                },
                                Text::new(skill_slot_glyph(slot)),
                                TextFont {
                                    font_size: FontSize::Px(skill_slot_glyph_font_size(slot)),
                                    ..default()
                                },
                                TextColor(skill_slot_glyph_color(slot)),
                            ));
                            slot_node.spawn((
                                Node {
                                    position_type: PositionType::Absolute,
                                    left: Val::Px(3.0),
                                    right: Val::Px(3.0),
                                    bottom: Val::Px(4.0),
                                    justify_content: JustifyContent::Center,
                                    align_items: AlignItems::Center,
                                    ..default()
                                },
                                Text::new(""),
                                TextFont {
                                    font_size: FontSize::Px(8.0),
                                    ..default()
                                },
                                TextColor(Color::srgb(0.92, 0.86, 0.72)),
                                CooldownText,
                                slot,
                            ));
                        });
                    }
                });
            parent
                .spawn((
                    Node {
                        position_type: PositionType::Absolute,
                        right: Val::Px(28.0),
                        bottom: Val::Px(18.0),
                        width: Val::Px(170.0),
                        height: Val::Px(170.0),
                        padding: UiRect::all(Val::Px(12.0)),
                        border: UiRect::all(Val::Px(3.0)),
                        overflow: Overflow::clip(),
                        border_radius: BorderRadius::MAX,
                        justify_content: JustifyContent::Center,
                        align_items: AlignItems::Center,
                        ..default()
                    },
                    BorderColor::all(Color::srgba(0.16, 0.34, 0.86, 0.82)),
                    BackgroundColor(Color::srgba(0.004, 0.014, 0.050, 0.94)),
                    ResourceOrbFrame::Fury,
                ))
                .with_children(|panel| {
                    panel.spawn((
                        Node {
                            position_type: PositionType::Absolute,
                            left: Val::Px(0.0),
                            right: Val::Px(0.0),
                            bottom: Val::Px(0.0),
                            width: Val::Percent(100.0),
                            height: Val::Percent(100.0),
                            ..default()
                        },
                        BackgroundColor(Color::srgba(0.035, 0.20, 0.84, 0.90)),
                        ResourceFill::Fury,
                    ));
                    panel.spawn((
                        Node {
                            position_type: PositionType::Absolute,
                            left: Val::Px(16.0),
                            right: Val::Px(16.0),
                            top: Val::Px(26.0),
                            bottom: Val::Px(22.0),
                            justify_content: JustifyContent::Center,
                            align_items: AlignItems::Center,
                            ..default()
                        },
                        Text::new(""),
                        TextFont {
                            font_size: FontSize::Px(14.0),
                            ..default()
                        },
                        TextColor(Color::srgb(0.76, 0.86, 1.0)),
                        StatsText,
                    ));
                });
            parent.spawn((
                Node {
                    position_type: PositionType::Absolute,
                    left: Val::Px(24.0),
                    bottom: Val::Px(204.0),
                    width: Val::Px(360.0),
                    min_height: Val::Px(30.0),
                    padding: UiRect::axes(Val::Px(10.0), Val::Px(6.0)),
                    border: UiRect::all(Val::Px(1.0)),
                    ..default()
                },
                BorderColor::all(Color::srgba(0.56, 0.48, 0.38, 0.44)),
                BackgroundColor(Color::srgba(0.015, 0.014, 0.016, 0.66)),
                Text::new("Log"),
                TextFont {
                    font_size: FontSize::Px(13.0),
                    ..default()
                },
                TextColor(Color::srgb(0.78, 0.75, 0.66)),
                Visibility::Hidden,
                LogText,
            ));
            parent.spawn((
                Node {
                    position_type: PositionType::Absolute,
                    left: Val::Px(18.0),
                    top: Val::Px(14.0),
                    width: Val::Px(410.0),
                    padding: UiRect::all(Val::Px(10.0)),
                    ..default()
                },
                Text::new(""),
                TextFont {
                    font_size: FontSize::Px(14.0),
                    ..default()
                },
                TextColor(Color::srgb(0.95, 0.78, 0.35)),
                PauseText,
            ));
            parent
                .spawn((
                    Node {
                        position_type: PositionType::Absolute,
                        left: Val::Px(24.0),
                        top: Val::Px(78.0),
                        width: Val::Px(660.0),
                        min_height: Val::Px(340.0),
                        padding: UiRect::all(Val::Px(12.0)),
                        border: UiRect::all(Val::Px(1.0)),
                        display: Display::None,
                        flex_direction: FlexDirection::Column,
                        row_gap: Val::Px(10.0),
                        ..default()
                    },
                    BorderColor::all(Color::srgba(0.44, 0.60, 0.80, 0.72)),
                    BackgroundColor(Color::srgba(0.010, 0.014, 0.020, 0.92)),
                    InventoryPanel,
                ))
                .with_children(|panel| {
                    panel.spawn((
                        Text::new(""),
                        TextFont {
                            font_size: FontSize::Px(14.0),
                            ..default()
                        },
                        TextColor(Color::srgb(0.70, 0.82, 0.95)),
                        InventoryText,
                    ));
                    panel
                        .spawn((Node {
                            width: Val::Percent(100.0),
                            display: Display::Flex,
                            column_gap: Val::Px(10.0),
                            ..default()
                        },))
                        .with_children(|body| {
                            body.spawn((Node {
                                width: Val::Px(250.0),
                                display: Display::Flex,
                                flex_direction: FlexDirection::Column,
                                row_gap: Val::Px(8.0),
                                ..default()
                            },))
                                .with_children(|gear| {
                                    // Paper doll: helm on top, amulet/chest/ring
                                    // across the torso row, weapon/gloves at the
                                    // hands, boots at the feet.
                                    let doll_rows: [&[GearSlot]; 4] = [
                                        &[GearSlot::Helm],
                                        &[GearSlot::Amulet, GearSlot::Chest, GearSlot::Ring],
                                        &[GearSlot::Weapon, GearSlot::Gloves],
                                        &[GearSlot::Boots],
                                    ];
                                    for row in doll_rows {
                                        gear.spawn((Node {
                                            width: Val::Percent(100.0),
                                            display: Display::Flex,
                                            justify_content: JustifyContent::Center,
                                            column_gap: Val::Px(6.0),
                                            ..default()
                                        },))
                                            .with_children(|doll_row| {
                                                for slot in row {
                                                    doll_row.spawn((
                                                        Node {
                                                            width: Val::Px(76.0),
                                                            height: Val::Px(64.0),
                                                            padding: UiRect::all(Val::Px(5.0)),
                                                            border: UiRect::all(Val::Px(1.0)),
                                                            justify_content: JustifyContent::Center,
                                                            align_items: AlignItems::Center,
                                                            ..default()
                                                        },
                                                        BorderColor::all(Color::srgba(
                                                            0.45, 0.40, 0.32, 0.55,
                                                        )),
                                                        BackgroundColor(Color::srgba(
                                                            0.030, 0.026, 0.022, 0.90,
                                                        )),
                                                        Text::new(""),
                                                        TextFont {
                                                            font_size: FontSize::Px(10.0),
                                                            ..default()
                                                        },
                                                        TextColor(Color::srgb(0.82, 0.75, 0.62)),
                                                        PaperDollSlot(*slot),
                                                    ));
                                                }
                                            });
                                    }
                                    for slot in [
                                        GearSlotText::Weapon,
                                        GearSlotText::Charm,
                                        GearSlotText::Codex,
                                        GearSlotText::Armory,
                                    ] {
                                        gear.spawn((
                                            Node {
                                                width: Val::Percent(100.0),
                                                height: Val::Px(58.0),
                                                padding: UiRect::all(Val::Px(8.0)),
                                                border: UiRect::all(Val::Px(1.0)),
                                                align_items: AlignItems::Center,
                                                ..default()
                                            },
                                            BorderColor::all(Color::srgba(0.65, 0.52, 0.34, 0.58)),
                                            BackgroundColor(Color::srgba(
                                                0.035, 0.030, 0.026, 0.90,
                                            )),
                                            Text::new(""),
                                            TextFont {
                                                font_size: FontSize::Px(12.0),
                                                ..default()
                                            },
                                            TextColor(Color::srgb(0.86, 0.78, 0.64)),
                                            slot,
                                        ));
                                    }
                                });
                            body.spawn((Node {
                                width: Val::Px(374.0),
                                display: Display::Flex,
                                flex_wrap: FlexWrap::Wrap,
                                column_gap: Val::Px(8.0),
                                row_gap: Val::Px(8.0),
                                ..default()
                            },))
                                .with_children(|grid| {
                                    for index in 0..10 {
                                        grid.spawn((
                                            Node {
                                                width: Val::Px(68.0),
                                                height: Val::Px(82.0),
                                                padding: UiRect::all(Val::Px(6.0)),
                                                border: UiRect::all(Val::Px(1.0)),
                                                justify_content: JustifyContent::Center,
                                                align_items: AlignItems::Center,
                                                ..default()
                                            },
                                            BorderColor::all(Color::srgba(0.36, 0.43, 0.50, 0.58)),
                                            BackgroundColor(Color::srgba(
                                                0.020, 0.024, 0.030, 0.92,
                                            )),
                                            Text::new(""),
                                            TextFont {
                                                font_size: FontSize::Px(10.5),
                                                ..default()
                                            },
                                            TextColor(Color::srgb(0.70, 0.82, 0.95)),
                                            InventorySlotText(index),
                                        ));
                                    }
                                });
                        });
                });
            parent.spawn((
                Node {
                    position_type: PositionType::Absolute,
                    right: Val::Px(24.0),
                    top: Val::Px(78.0),
                    width: Val::Px(430.0),
                    min_height: Val::Px(280.0),
                    padding: UiRect::all(Val::Px(12.0)),
                    border: UiRect::all(Val::Px(1.0)),
                    display: Display::None,
                    ..default()
                },
                BorderColor::all(Color::srgba(0.72, 0.48, 0.90, 0.72)),
                BackgroundColor(Color::srgba(0.018, 0.012, 0.026, 0.92)),
                Text::new(""),
                TextFont {
                    font_size: FontSize::Px(13.0),
                    ..default()
                },
                TextColor(Color::srgb(0.84, 0.76, 0.96)),
                BuildPanel,
                BuildText,
            ));
            parent.spawn((
                Node {
                    position_type: PositionType::Absolute,
                    right: Val::Px(24.0),
                    top: Val::Px(176.0),
                    width: Val::Px(470.0),
                    padding: UiRect::all(Val::Px(12.0)),
                    border: UiRect::all(Val::Px(1.0)),
                    ..default()
                },
                BorderColor::all(Color::srgba(0.72, 0.58, 0.34, 0.65)),
                BackgroundColor(Color::srgba(0.020, 0.016, 0.012, 0.84)),
                Text::new(""),
                TextFont {
                    font_size: FontSize::Px(13.0),
                    ..default()
                },
                TextColor(Color::srgb(0.86, 0.76, 0.58)),
                Visibility::Hidden,
                JournalText,
            ));
            parent.spawn((
                Node {
                    position_type: PositionType::Absolute,
                    top: Val::Px(116.0),
                    left: Val::Px(0.0),
                    width: Val::Percent(100.0),
                    justify_content: JustifyContent::Center,
                    ..default()
                },
                Text::new(""),
                TextFont {
                    font_size: FontSize::Px(24.0),
                    ..default()
                },
                TextColor(Color::srgb(1.0, 0.76, 0.28)),
                ChapterBannerText,
            ));
            parent.spawn((
                Node {
                    position_type: PositionType::Absolute,
                    left: Val::Px(24.0),
                    top: Val::Px(46.0),
                    width: Val::Px(430.0),
                    min_height: Val::Px(64.0),
                    padding: UiRect::axes(Val::Px(12.0), Val::Px(8.0)),
                    border: UiRect::all(Val::Px(1.0)),
                    overflow: Overflow::clip(),
                    ..default()
                },
                BorderColor::all(Color::srgba(0.78, 0.58, 0.28, 0.70)),
                BackgroundColor(Color::srgba(0.018, 0.014, 0.018, 0.82)),
                Text::new(""),
                TextFont {
                    font_size: FontSize::Px(13.0),
                    ..default()
                },
                TextColor(Color::srgb(0.92, 0.80, 0.62)),
                Visibility::Hidden,
                StoryCommsText,
            ));
            parent.spawn((
                Node {
                    position_type: PositionType::Absolute,
                    left: Val::Percent(50.0),
                    bottom: Val::Px(204.0),
                    width: Val::Px(520.0),
                    margin: UiRect::left(Val::Px(-260.0)),
                    justify_content: JustifyContent::Center,
                    ..default()
                },
                Text::new(""),
                TextFont {
                    font_size: FontSize::Px(24.0),
                    ..default()
                },
                TextColor(Color::srgb(1.0, 0.66, 0.20)),
                LevelUpBannerText,
            ));
            parent.spawn((
                Node {
                    position_type: PositionType::Absolute,
                    left: Val::Percent(50.0),
                    bottom: Val::Px(238.0),
                    width: Val::Px(560.0),
                    margin: UiRect::left(Val::Px(-280.0)),
                    justify_content: JustifyContent::Center,
                    ..default()
                },
                Text::new(""),
                TextFont {
                    font_size: FontSize::Px(18.0),
                    ..default()
                },
                TextColor(Color::srgb(0.92, 0.82, 0.62)),
                LootPickupBannerText,
            ));
            parent
                .spawn((
                    Node {
                        position_type: PositionType::Absolute,
                        left: Val::Percent(50.0),
                        top: Val::Px(164.0),
                        width: Val::Px(700.0),
                        min_height: Val::Px(44.0),
                        margin: UiRect::left(Val::Px(-340.0)),
                        padding: UiRect::axes(Val::Px(10.0), Val::Px(6.0)),
                        border: UiRect::all(Val::Px(1.0)),
                        justify_content: JustifyContent::Center,
                        align_items: AlignItems::Center,
                        overflow: Overflow::clip(),
                        ..default()
                    },
                    Visibility::Hidden,
                    ZIndex(500),
                    BorderColor::all(Color::srgba(0.84, 0.62, 0.38, 0.6)),
                    BackgroundColor(Color::srgba(0.03, 0.015, 0.01, 0.75)),
                    RewardToastPanel,
                ))
                .with_children(|toast_panel| {
                    toast_panel.spawn((
                        Text::new(""),
                        TextFont {
                            font_size: FontSize::Px(18.0),
                            ..default()
                        },
                        TextColor(Color::srgb(1.0, 0.74, 0.28)),
                        RewardToastText,
                    ));
                });
            parent.spawn((
                Node {
                    position_type: PositionType::Absolute,
                    left: Val::Percent(50.0),
                    bottom: Val::Px(198.0),
                    width: Val::Px(540.0),
                    margin: UiRect::left(Val::Px(-270.0)),
                    justify_content: JustifyContent::Center,
                    ..default()
                },
                Text::new(""),
                TextFont {
                    font_size: FontSize::Px(18.0),
                    ..default()
                },
                TextColor(Color::srgb(0.76, 0.90, 1.0)),
                SkillFeedbackText,
            ));
            parent.spawn((
                Node {
                    position_type: PositionType::Absolute,
                    left: Val::Percent(50.0),
                    bottom: Val::Px(280.0),
                    width: Val::Px(620.0),
                    margin: UiRect::left(Val::Px(-310.0)),
                    justify_content: JustifyContent::Center,
                    ..default()
                },
                Text::new(""),
                TextFont {
                    font_size: FontSize::Px(26.0),
                    ..default()
                },
                TextColor(Color::srgb(1.0, 0.78, 0.22)),
                CombatStreakBannerText,
            ));
            parent
                .spawn((
                    Node {
                        position_type: PositionType::Absolute,
                        right: Val::Px(212.0),
                        bottom: Val::Px(204.0),
                        width: Val::Px(390.0),
                        min_height: Val::Px(54.0),
                        padding: UiRect::axes(Val::Px(10.0), Val::Px(7.0)),
                        border: UiRect::all(Val::Px(1.0)),
                        display: Display::None,
                        overflow: Overflow::clip(),
                        justify_content: JustifyContent::Center,
                        align_items: AlignItems::Center,
                        ..default()
                    },
                    BorderColor::all(Color::srgba(0.72, 0.58, 0.34, 0.62)),
                    BackgroundColor(Color::srgba(0.018, 0.016, 0.018, 0.78)),
                    LootProximityPanel,
                ))
                .with_children(|panel| {
                    panel.spawn((
                        Text::new(""),
                        TextFont {
                            font_size: FontSize::Px(12.0),
                            ..default()
                        },
                        TextColor(Color::srgb(0.86, 0.78, 0.62)),
                        LootProximityText,
                    ));
                });
            parent
                .spawn((
                    Node {
                        position_type: PositionType::Absolute,
                        right: Val::Px(18.0),
                        top: Val::Px(18.0),
                        width: Val::Px(MINIMAP_WIDTH),
                        height: Val::Px(MINIMAP_HEIGHT),
                        border: UiRect::all(Val::Px(1.0)),
                        ..default()
                    },
                    BorderColor::all(Color::srgba(0.78, 0.68, 0.48, 0.8)),
                    BackgroundColor(Color::srgba(0.02, 0.018, 0.024, 0.72)),
                    MinimapUi,
                ))
                .with_children(|map| {
                    map.spawn((
                        Node {
                            position_type: PositionType::Absolute,
                            left: Val::Px(8.0),
                            top: Val::Px(6.0),
                            ..default()
                        },
                        Text::new("Reliquary"),
                        TextFont {
                            font_size: FontSize::Px(11.0),
                            ..default()
                        },
                        TextColor(Color::srgba(0.78, 0.74, 0.66, 0.92)),
                    ));
                    map.spawn((
                        Node {
                            position_type: PositionType::Absolute,
                            left: Val::Px(8.0),
                            top: Val::Px(20.0),
                            width: Val::Px(MINIMAP_WIDTH - 16.0),
                            overflow: Overflow::clip(),
                            ..default()
                        },
                        Text::new(""),
                        TextFont {
                            font_size: FontSize::Px(8.5),
                            ..default()
                        },
                        TextColor(Color::srgba(0.72, 0.82, 0.94, 0.88)),
                        MinimapObjectiveText,
                    ));
                    for (index, (role, label)) in minimap_legend_entries().into_iter().enumerate() {
                        let color = minimap_legend_color(role);
                        map.spawn((
                            Node {
                                position_type: PositionType::Absolute,
                                left: Val::Px(8.0 + index as f32 * 31.0),
                                bottom: Val::Px(6.0),
                                width: Val::Px(29.0),
                                height: Val::Px(10.0),
                                align_items: AlignItems::Center,
                                column_gap: Val::Px(3.0),
                                ..default()
                            },
                            Pickable::IGNORE,
                        ))
                        .with_children(|legend| {
                            legend.spawn((
                                Node {
                                    width: Val::Px(5.0),
                                    height: Val::Px(5.0),
                                    border_radius: BorderRadius::MAX,
                                    ..default()
                                },
                                BackgroundColor(color),
                            ));
                            legend.spawn((
                                Text::new(label),
                                TextFont {
                                    font_size: FontSize::Px(7.5),
                                    ..default()
                                },
                                TextColor(Color::srgba(0.76, 0.72, 0.64, 0.86)),
                            ));
                        });
                    }
                });
            parent
                .spawn((
                    Node {
                        position_type: PositionType::Absolute,
                        right: Val::Px(18.0),
                        top: Val::Px(158.0),
                        width: Val::Px(260.0),
                        min_height: Val::Px(42.0),
                        padding: UiRect::axes(Val::Px(8.0), Val::Px(6.0)),
                        border: UiRect::all(Val::Px(1.0)),
                        display: Display::None,
                        overflow: Overflow::clip(),
                        flex_direction: FlexDirection::Row,
                        align_items: AlignItems::Center,
                        column_gap: Val::Px(8.0),
                        ..default()
                    },
                    BorderColor::all(Color::srgba(0.22, 0.68, 1.0, 0.72)),
                    BackgroundColor(Color::srgba(0.010, 0.018, 0.026, 0.82)),
                    Pickable::IGNORE,
                    ObjectiveCompassPanel,
                ))
                .with_children(|compass| {
                    compass.spawn((
                        Node {
                            width: Val::Px(54.0),
                            height: Val::Px(30.0),
                            border: UiRect::all(Val::Px(1.0)),
                            border_radius: BorderRadius::all(Val::Px(4.0)),
                            justify_content: JustifyContent::Center,
                            align_items: AlignItems::Center,
                            ..default()
                        },
                        BorderColor::all(Color::srgba(0.36, 0.84, 1.0, 0.72)),
                        BackgroundColor(Color::srgba(0.018, 0.044, 0.064, 0.86)),
                        Text::new(""),
                        TextFont {
                            font_size: FontSize::Px(15.5),
                            ..default()
                        },
                        TextColor(Color::srgb(0.68, 0.92, 1.0)),
                        ObjectiveCompassText::Direction,
                    ));
                    compass.spawn((
                        Node {
                            flex_grow: 1.0,
                            min_width: Val::Px(0.0),
                            overflow: Overflow::clip(),
                            justify_content: JustifyContent::FlexStart,
                            align_items: AlignItems::Center,
                            ..default()
                        },
                        Text::new(""),
                        TextFont {
                            font_size: FontSize::Px(10.5),
                            ..default()
                        },
                        TextColor(Color::srgb(0.74, 0.84, 0.92)),
                        ObjectiveCompassText::Detail,
                    ));
                });
            parent
                .spawn((
                    Node {
                        position_type: PositionType::Absolute,
                        right: Val::Px(18.0),
                        top: Val::Px(224.0),
                        width: Val::Px(260.0),
                        min_height: Val::Px(84.0),
                        padding: UiRect::axes(Val::Px(10.0), Val::Px(8.0)),
                        border: UiRect::all(Val::Px(1.0)),
                        display: Display::None,
                        overflow: Overflow::clip(),
                        flex_direction: FlexDirection::Column,
                        justify_content: JustifyContent::FlexStart,
                        row_gap: Val::Px(6.0),
                        ..default()
                    },
                    BorderColor::all(Color::srgba(0.78, 0.62, 0.38, 0.66)),
                    BackgroundColor(Color::srgba(0.018, 0.014, 0.018, 0.82)),
                    TargetInfoPanel,
                ))
                .with_children(|target| {
                    target.spawn((
                        Text::new(""),
                        TextFont {
                            font_size: FontSize::Px(12.0),
                            ..default()
                        },
                        TextColor(Color::srgb(0.88, 0.80, 0.66)),
                        TargetInfoText,
                    ));
                    target.spawn((
                        Node {
                            width: Val::Percent(100.0),
                            height: Val::Px(7.0),
                            border_radius: BorderRadius::all(Val::Px(3.0)),
                            ..default()
                        },
                        BackgroundColor(Color::srgba(0.86, 0.08, 0.04, 0.88)),
                        TargetInfoFill,
                    ));
                });
            parent
                .spawn((
                    Node {
                        position_type: PositionType::Absolute,
                        right: Val::Px(18.0),
                        top: Val::Px(316.0),
                        width: Val::Px(286.0),
                        min_height: Val::Px(72.0),
                        padding: UiRect::axes(Val::Px(10.0), Val::Px(8.0)),
                        border: UiRect::all(Val::Px(1.0)),
                        display: Display::None,
                        overflow: Overflow::clip(),
                        justify_content: JustifyContent::Center,
                        align_items: AlignItems::Center,
                        ..default()
                    },
                    BorderColor::all(Color::srgba(0.82, 0.66, 0.38, 0.66)),
                    BackgroundColor(Color::srgba(0.018, 0.015, 0.012, 0.82)),
                    InteractProximityPanel,
                ))
                .with_children(|panel| {
                    panel.spawn((
                        Text::new(""),
                        TextFont {
                            font_size: FontSize::Px(12.0),
                            ..default()
                        },
                        TextColor(Color::srgb(0.90, 0.78, 0.58)),
                        InteractProximityText,
                    ));
                });
        });
}

fn toggle_hud_density(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut hud_density: ResMut<HudDensity>,
    mut combat_events: MessageWriter<CombatEvent>,
) {
    if !keyboard.just_pressed(KeyCode::F2) {
        return;
    }

    let density = hud_density.toggle();
    combat_events.write(CombatEvent {
        text: format!("HUD mode: {}", density.label()),
    });
}

fn spawn_bottom_console_backdrop(parent: &mut ChildSpawnerCommands) {
    parent
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                left: Val::Percent(50.0),
                bottom: Val::Px(4.0),
                width: Val::Px(1180.0),
                height: Val::Px(210.0),
                margin: UiRect::left(Val::Px(-590.0)),
                border: UiRect::all(Val::Px(2.0)),
                border_radius: BorderRadius::all(Val::Px(8.0)),
                overflow: Overflow::clip(),
                ..default()
            },
            BorderColor::all(Color::srgba(0.74, 0.58, 0.34, 0.54)),
            BackgroundColor(Color::srgba(0.012, 0.011, 0.014, 0.78)),
        ))
        .with_children(|console| {
            console.spawn((
                Node {
                    position_type: PositionType::Absolute,
                    left: Val::Px(10.0),
                    right: Val::Px(10.0),
                    top: Val::Px(8.0),
                    height: Val::Px(22.0),
                    border: UiRect {
                        bottom: Val::Px(1.0),
                        ..default()
                    },
                    ..default()
                },
                BorderColor::all(Color::srgba(0.92, 0.72, 0.38, 0.32)),
                BackgroundColor(Color::srgba(0.080, 0.060, 0.038, 0.44)),
            ));
            console.spawn((
                Node {
                    position_type: PositionType::Absolute,
                    left: Val::Px(184.0),
                    right: Val::Px(184.0),
                    bottom: Val::Px(14.0),
                    height: Val::Px(116.0),
                    border: UiRect::all(Val::Px(1.0)),
                    border_radius: BorderRadius::all(Val::Px(6.0)),
                    ..default()
                },
                BorderColor::all(Color::srgba(0.64, 0.50, 0.32, 0.36)),
                BackgroundColor(Color::srgba(0.026, 0.023, 0.030, 0.66)),
            ));
            for left in [18.0_f32, 982.0] {
                console.spawn((
                    Node {
                        position_type: PositionType::Absolute,
                        left: Val::Px(left),
                        bottom: Val::Px(10.0),
                        width: Val::Px(180.0),
                        height: Val::Px(180.0),
                        border: UiRect::all(Val::Px(2.0)),
                        border_radius: BorderRadius::MAX,
                        ..default()
                    },
                    BorderColor::all(Color::srgba(0.88, 0.68, 0.36, 0.34)),
                    BackgroundColor(Color::srgba(0.018, 0.014, 0.016, 0.52)),
                ));
            }
        });
}

fn reset_chapter_banner(mut state: ResMut<ChapterBannerState>) {
    *state = ChapterBannerState::default();
}

fn reset_level_up_banner(mut state: ResMut<LevelUpBannerState>) {
    *state = LevelUpBannerState::default();
}

fn reset_loot_pickup_banner(mut state: ResMut<LootPickupBannerState>) {
    *state = LootPickupBannerState::default();
}

fn reset_reward_toast(mut state: ResMut<RewardToastState>) {
    *state = RewardToastState::default();
}

fn reset_story_comms(mut state: ResMut<StoryCommsState>) {
    *state = StoryCommsState::default();
}

fn reset_skill_feedback(mut state: ResMut<SkillFeedbackState>) {
    *state = SkillFeedbackState::default();
}

fn reset_combat_streak_banner(mut state: ResMut<CombatStreakBannerState>) {
    *state = CombatStreakBannerState::default();
}

fn update_chapter_banner(
    time: Res<Time>,
    progress: Res<ChapterProgress>,
    mut state: ResMut<ChapterBannerState>,
    mut banner: Query<&mut Text, With<ChapterBannerText>>,
) {
    let Ok(mut text) = banner.single_mut() else {
        return;
    };

    if state.last_phase != Some(progress.phase) {
        state.last_phase = Some(progress.phase);
        state.timer = Timer::from_seconds(chapter_banner_duration(progress.phase), TimerMode::Once);
        state.timer.reset();
        **text = chapter_banner_text(progress.phase).to_string();
        return;
    }

    if state.timer.is_finished() {
        if !text.is_empty() {
            **text = String::new();
        }
        return;
    }

    state.timer.tick(time.delta());
    if state.timer.is_finished() {
        **text = String::new();
    }
}

fn update_level_up_banner(
    time: Res<Time>,
    mut state: ResMut<LevelUpBannerState>,
    player: Query<(&PlayerLevel, &Talents), With<Player>>,
    mut banner: Query<&mut Text, With<LevelUpBannerText>>,
) {
    let Ok((level, talents)) = player.single() else {
        return;
    };
    let Ok(mut text) = banner.single_mut() else {
        return;
    };

    if state.observed_level == 0 {
        state.observed_level = level.level;
        state.observed_talent_points = talents.points;
        **text = String::new();
        return;
    }

    if let Some(copy) = level_up_banner_copy(
        level.level,
        talents.points,
        state.observed_level,
        state.observed_talent_points,
    ) {
        state.timer = Timer::from_seconds(2.8, TimerMode::Once);
        state.timer.reset();
        state.observed_level = level.level;
        state.observed_talent_points = talents.points;
        **text = copy;
        return;
    }

    state.observed_level = state.observed_level.max(level.level);
    if talents.points < state.observed_talent_points {
        state.observed_talent_points = talents.points;
    }

    if state.timer.is_finished() {
        if !text.is_empty() {
            **text = String::new();
        }
        return;
    }

    state.timer.tick(time.delta());
    if state.timer.is_finished() {
        **text = String::new();
    }
}

fn level_up_banner_copy(
    current_level: u32,
    current_talent_points: u32,
    observed_level: u32,
    observed_talent_points: u32,
) -> Option<String> {
    if current_level > observed_level {
        let point_line = if current_talent_points > 0 {
            format!(
                "\n{} talent point available - K then 1/2/3",
                current_talent_points
            )
        } else {
            String::new()
        };
        return Some(format!("LEVEL {current_level}{point_line}"));
    }
    if current_talent_points > observed_talent_points && current_talent_points > 0 {
        return Some(format!(
            "{} talent point available - K then 1/2/3",
            current_talent_points
        ));
    }
    None
}

fn update_loot_pickup_banner(
    time: Res<Time>,
    mut state: ResMut<LootPickupBannerState>,
    mut events: MessageReader<CombatEvent>,
    mut banner: Query<(&mut Text, &mut TextColor), With<LootPickupBannerText>>,
) {
    let Ok((mut text, mut color)) = banner.single_mut() else {
        return;
    };

    for event in events.read() {
        let Some(pickup) = loot_pickup_banner_from_event(&event.text) else {
            continue;
        };
        state.timer = Timer::from_seconds(2.4, TimerMode::Once);
        state.timer.reset();
        **text = pickup.copy;
        color.0 = pickup.color;
    }

    if state.timer.is_finished() {
        if !text.is_empty() {
            **text = String::new();
        }
        return;
    }

    state.timer.tick(time.delta());
    if state.timer.is_finished() {
        **text = String::new();
    }
}

#[derive(Debug, Clone)]
struct LootPickupBanner {
    copy: String,
    color: Color,
}

fn loot_pickup_banner_from_event(text: &str) -> Option<LootPickupBanner> {
    let payload = text.strip_prefix("Picked up ")?;
    let quality = payload.split_whitespace().next().unwrap_or_default();
    let item_text = payload
        .strip_prefix(quality)
        .map(str::trim_start)
        .unwrap_or(payload);
    let copy = loot_pickup_banner_copy(quality, item_text);
    Some(LootPickupBanner {
        copy,
        color: quality_color(quality),
    })
}

fn loot_pickup_banner_copy(quality: &str, item_text: &str) -> String {
    let item = compact_label(item_text, 58);
    let (headline, verdict, next_step) = match quality.to_ascii_lowercase().as_str() {
        "primal" => (
            "PRIMAL DROP",
            "Build around",
            "Next: socket, augment, save loadout",
        ),
        "ancient" => (
            "ANCIENT DROP",
            "Temper candidate",
            "Next: compare DPS, augment if fit",
        ),
        "legendary" => (
            "LEGENDARY DROP",
            "Equip or codex-check",
            "Next: test power, extract spare",
        ),
        "rare" => (
            "RARE DROP",
            "Upgrade or craft base",
            "Next: equip gain, salvage spare",
        ),
        "magic" => (
            "MAGIC DROP",
            "Early stat filler",
            "Next: replace or salvage",
        ),
        "common" => (
            "COMMON DROP",
            "Material fodder",
            "Next: salvage when bag fills",
        ),
        other => {
            let headline = if other.is_empty() {
                "DROP".to_string()
            } else {
                format!("{} DROP", other.to_ascii_uppercase())
            };
            return format!("{headline}\n{}\nNext: inspect or salvage", item);
        }
    };
    if verdict.is_empty() {
        format!("{headline}\n{item}\n{next_step}")
    } else {
        format!("{headline}\n{verdict} {item}\n{next_step}")
    }
}

fn update_reward_toast(
    time: Res<Time>,
    mut state: ResMut<RewardToastState>,
    mut events: MessageReader<CombatEvent>,
    mut banner: Query<(&mut Text, &mut TextColor, &mut Visibility), With<RewardToastText>>,
    mut panel: Query<&mut Visibility, (With<RewardToastPanel>, Without<RewardToastText>)>,
) {
    let Ok((mut text, mut color, mut visibility)) = banner.single_mut() else {
        return;
    };
    let Ok(mut panel_visibility) = panel.single_mut() else {
        return;
    };

    for event in events.read() {
        let Some(toast) = reward_toast_from_event(&event.text) else {
            continue;
        };
        state.timer = Timer::from_seconds(toast.duration_secs, TimerMode::Once);
        state.timer.reset();
        **text = toast.copy;
        color.0 = toast.color;
        *visibility = Visibility::Visible;
        *panel_visibility = Visibility::Visible;
    }

    if state.timer.is_finished() {
        if !text.is_empty() {
            **text = String::new();
        }
        *visibility = Visibility::Hidden;
        *panel_visibility = Visibility::Hidden;
        return;
    }

    state.timer.tick(time.delta());
    if state.timer.is_finished() {
        **text = String::new();
        *visibility = Visibility::Hidden;
        *panel_visibility = Visibility::Hidden;
    }
}

fn update_story_comms(
    time: Res<Time>,
    mut state: ResMut<StoryCommsState>,
    mut events: MessageReader<CombatEvent>,
    mut banner: Query<(&mut Text, &mut Visibility), With<StoryCommsText>>,
) {
    let Ok((mut text, mut visibility)) = banner.single_mut() else {
        return;
    };

    for event in events.read() {
        let Some(comms) = story_comms_from_event(&event.text) else {
            continue;
        };
        state.timer = Timer::from_seconds(comms.duration_secs, TimerMode::Once);
        state.timer.reset();
        **text = comms.copy;
        *visibility = Visibility::Visible;
    }

    if state.timer.is_finished() {
        if !text.is_empty() {
            **text = String::new();
        }
        *visibility = Visibility::Hidden;
        return;
    }

    state.timer.tick(time.delta());
    if state.timer.is_finished() {
        **text = String::new();
        *visibility = Visibility::Hidden;
    }
}

#[derive(Debug, Clone)]
struct RewardToast {
    copy: String,
    color: Color,
    duration_secs: f32,
}

#[derive(Debug, Clone, PartialEq)]
struct StoryComms {
    copy: String,
    duration_secs: f32,
}

fn story_comms_from_event(text: &str) -> Option<StoryComms> {
    let beat = story_beat_from_event_text(text)?;
    let speaker = story_beat_speaker(beat);
    let action = story_beat_action_hint(beat);
    let line = text
        .split_once(": ")
        .map(|(_, line)| line)
        .unwrap_or_else(|| beat.line());

    Some(StoryComms {
        copy: format!("{speaker}\n{}\nNext: {action}", compact_label(line, 86)),
        duration_secs: story_comms_duration(beat),
    })
}

fn story_beat_from_event_text(text: &str) -> Option<StoryBeat> {
    [
        StoryBeat::Arrival,
        StoryBeat::CacheRevealed,
        StoryBeat::SealRecovered,
        StoryBeat::SanctumOpened,
        StoryBeat::SealWardenSlain,
        StoryBeat::AltarDemand,
        StoryBeat::KeeperAwakened,
        StoryBeat::KeeperBloodied,
        StoryBeat::Victory,
        StoryBeat::CursedShrine,
        StoryBeat::BloodObelisk,
        StoryBeat::ReliquaryVault,
        StoryBeat::EmberRift,
        StoryBeat::AshenPylon,
    ]
    .into_iter()
    .find(|beat| text.starts_with(&format!("{}: ", beat.title())))
}

fn story_beat_speaker(beat: StoryBeat) -> &'static str {
    match beat {
        StoryBeat::KeeperAwakened | StoryBeat::KeeperBloodied => "Malrec, Keeper of Ash",
        StoryBeat::CursedShrine | StoryBeat::BloodObelisk | StoryBeat::EmberRift => {
            "Reliquary Whisper"
        }
        StoryBeat::ReliquaryVault | StoryBeat::AshenPylon => "Quartermaster Vey",
        StoryBeat::Victory => "Steward Edda",
        _ => "Steward Edda",
    }
}

fn story_beat_action_hint(beat: StoryBeat) -> &'static str {
    match beat {
        StoryBeat::Arrival => "breach the outer guards",
        StoryBeat::CacheRevealed => "open the sealed cache",
        StoryBeat::SealRecovered => "push into the inner sanctum",
        StoryBeat::SanctumOpened => "survive the ambush and find Vhal",
        StoryBeat::SealWardenSlain => "take the second seal to the altar",
        StoryBeat::AltarDemand => "extinguish the final seal",
        StoryBeat::KeeperAwakened => "choose a boon and break Malrec",
        StoryBeat::KeeperBloodied => "save burst for the stagger window",
        StoryBeat::Victory => "claim rewards and plan the next run",
        StoryBeat::CursedShrine => "kill all ambushers for the offering",
        StoryBeat::BloodObelisk => "finish the rite before it turns",
        StoryBeat::ReliquaryVault => "loot the vault and spend at town",
        StoryBeat::EmberRift => "seal the rift before the timer breaks",
        StoryBeat::AshenPylon => "chain kills while the pylon burns",
    }
}

fn story_comms_duration(beat: StoryBeat) -> f32 {
    match beat {
        StoryBeat::KeeperAwakened | StoryBeat::KeeperBloodied | StoryBeat::Victory => 5.2,
        StoryBeat::CursedShrine | StoryBeat::BloodObelisk | StoryBeat::EmberRift => 4.6,
        _ => 4.0,
    }
}

fn reward_toast_from_event(text: &str) -> Option<RewardToast> {
    let (headline, detail, color, duration_secs) = reward_toast_copy(text)?;
    Some(RewardToast {
        copy: format!("{headline}\n{detail}"),
        color,
        duration_secs,
    })
}

fn reward_toast_copy(text: &str) -> Option<(&'static str, String, Color, f32)> {
    let normalized = text.trim().to_ascii_lowercase().replace("  ", " ");

    if text.starts_with("Picked up ")
        || text.starts_with("Massacre ")
        || text.starts_with("Carnage ")
        || text.starts_with("Slaughter ")
        || text.starts_with("Annihilation ")
        || text.starts_with("Valor ")
    {
        return None;
    }

    if normalized.starts_with("main quest complete") {
        let detail = text.split_once(':').map_or("", |(_, detail)| detail).trim();
        return Some((
            "QUEST COMPLETE",
            detail.to_string(),
            Color::srgb(1.0, 0.78, 0.24),
            4.2,
        ));
    }
    if normalized.starts_with("main quest:") {
        let detail = text.split_once(':').map_or("", |(_, detail)| detail).trim();
        return Some((
            "QUEST UPDATED",
            detail.to_string(),
            Color::srgb(1.0, 0.70, 0.28),
            3.4,
        ));
    }
    if normalized.starts_with("relic shrine") {
        let detail = text.split_once(':').map_or("", |(_, detail)| detail).trim();
        return Some((
            "RELIC SHRINE",
            detail.to_string(),
            Color::srgb(0.96, 0.56, 0.23),
            3.4,
        ));
    }
    if normalized.starts_with("fortune shrine") {
        let detail = text.split_once(':').map_or("", |(_, detail)| detail).trim();
        return Some((
            "FORTUNE SHRINE",
            detail.to_string(),
            Color::srgb(0.95, 0.75, 0.34),
            3.4,
        ));
    }
    if normalized.starts_with("storm shrine") {
        let detail = text.split_once(':').map_or("", |(_, detail)| detail).trim();
        return Some((
            "STORM SHRINE",
            detail.to_string(),
            Color::srgb(0.48, 0.84, 0.95),
            3.4,
        ));
    }
    if normalized.starts_with("shrine resonance x") {
        let detail = text.to_string();
        return Some((
            "SHRINE RESONANCE",
            detail,
            Color::srgb(0.95, 0.58, 0.95),
            3.2,
        ));
    }
    if normalized.starts_with("blood obelisk awakened")
        && let Some((_, detail)) = text.split_once(':')
    {
        return Some((
            "BLOOD OBELISK",
            detail.trim().to_string(),
            Color::srgb(0.95, 0.25, 0.18),
            3.8,
        ));
    }
    if normalized.starts_with("ashen pylon") {
        let detail = text.split_once(':').map_or("", |(_, detail)| detail).trim();
        return Some((
            "ASHEN PYLON",
            detail.to_string(),
            Color::srgb(0.88, 0.46, 0.16),
            3.2,
        ));
    }
    if normalized.starts_with("ember rift sealed") {
        let suffix = text
            .strip_prefix("Ember rift sealed")
            .or_else(|| text.strip_prefix("Ember rift sealed:"))
            .unwrap_or("")
            .trim();
        return Some((
            "RIFT SEALED",
            if suffix.is_empty() {
                "rift sealed".to_string()
            } else {
                format!("rift sealed{suffix}")
            },
            Color::srgb(1.0, 0.48, 0.18),
            3.6,
        ));
    }

    if let Some(detail) = text.strip_prefix("MAIN ") {
        return Some((
            "MAIN QUEST",
            chapter_milestone_toast_detail(detail),
            Color::srgb(1.0, 0.76, 0.28),
            3.7,
        ));
    }
    if let Some(detail) = text.strip_prefix("BOSS ") {
        return Some((
            "BOSS OBJECTIVE",
            chapter_milestone_toast_detail(detail),
            Color::srgb(1.0, 0.28, 0.16),
            4.2,
        ));
    }

    if let Some(detail) = text.strip_prefix("Main Quest Complete: ") {
        return Some((
            "QUEST COMPLETE",
            detail.to_string(),
            Color::srgb(1.0, 0.78, 0.24),
            4.2,
        ));
    }
    if let Some(detail) = text.strip_prefix("Main Quest: ") {
        return Some((
            "QUEST UPDATED",
            detail.to_string(),
            Color::srgb(1.0, 0.70, 0.28),
            3.4,
        ));
    }
    if let Some(detail) = text.strip_prefix("Relic shrine: ") {
        return Some((
            "RELIC SHRINE",
            detail.to_string(),
            Color::srgb(0.96, 0.56, 0.23),
            3.4,
        ));
    }
    if let Some(detail) = text.strip_prefix("Fortune shrine: ") {
        return Some((
            "FORTUNE SHRINE",
            detail.to_string(),
            Color::srgb(0.95, 0.75, 0.34),
            3.4,
        ));
    }
    if let Some(detail) = text.strip_prefix("Storm shrine: ") {
        return Some((
            "STORM SHRINE",
            detail.to_string(),
            Color::srgb(0.48, 0.84, 0.95),
            3.4,
        ));
    }
    if let Some(detail) = text.strip_prefix("Renewal well restored ") {
        return Some((
            "HEALING WELL",
            format!("Renewal well restored {detail}"),
            Color::srgb(0.42, 0.94, 0.92),
            3.4,
        ));
    }
    if let Some(detail) = text.strip_prefix("Lore recovered: ") {
        return Some((
            "LORE RECOVERED",
            detail.to_string(),
            Color::srgb(0.72, 0.68, 0.48),
            3.2,
        ));
    }
    if let Some(detail) = text.strip_prefix("Shrine resonance x") {
        return Some((
            "SHRINE RESONANCE",
            format!("Shrine resonance x{detail}"),
            Color::srgb(0.95, 0.58, 0.95),
            3.2,
        ));
    }
    if text.starts_with("Cursed shrine claimed ") {
        return Some((
            "CURSED SHRINE",
            text.to_string(),
            Color::srgb(0.98, 0.25, 0.34),
            3.4,
        ));
    }
    if let Some(detail) = text.strip_prefix("Blood obelisk awakened: ") {
        return Some((
            "BLOOD OBELISK",
            detail.to_string(),
            Color::srgb(0.95, 0.25, 0.18),
            3.8,
        ));
    }
    if let Some(detail) = text.strip_prefix("Blood obelisk completed: ") {
        return Some((
            "RITE COMPLETE",
            detail.to_string(),
            Color::srgb(1.0, 0.30, 0.22),
            3.4,
        ));
    }
    if text == "Blood obelisk faded before it was fed" {
        return Some((
            "BLOOD OBELISK",
            "Blood obelisk faded before it was fed".to_string(),
            Color::srgb(0.85, 0.26, 0.20),
            3.6,
        ));
    }
    if let Some(detail) = text.strip_prefix("Ember rift opened: ") {
        return Some((
            "EMBER RIFT",
            detail.to_string(),
            Color::srgb(1.0, 0.32, 0.12),
            3.4,
        ));
    }
    if text.starts_with("Ember rift sealed") {
        return Some((
            "RIFT SEALED",
            text.strip_prefix("Ember rift sealed").map_or_else(
                || "".to_string(),
                |suffix| {
                    if suffix.is_empty() {
                        "rift sealed".to_string()
                    } else {
                        format!("rift sealed{suffix}")
                    }
                },
            ),
            Color::srgb(1.0, 0.48, 0.18),
            3.6,
        ));
    }
    if text == "Ember rift collapsed before it was sealed" {
        return Some((
            "RIFT COLLAPSED",
            text.to_string(),
            Color::srgb(0.88, 0.33, 0.20),
            3.6,
        ));
    }
    if let Some(detail) = text.strip_prefix("Ashen pylon: ") {
        return Some((
            "ASHEN PYLON",
            detail.to_string(),
            Color::srgb(0.88, 0.46, 0.16),
            3.2,
        ));
    }
    if let Some(detail) = text.strip_prefix("Reliquary cache opened: ") {
        return Some((
            "CACHE OPENED",
            detail.to_string(),
            Color::srgb(0.79, 0.92, 0.57),
            3.4,
        ));
    }
    if let Some(detail) = text.strip_prefix("Reliquary vault opened: ") {
        return Some((
            "VAULT OPENED",
            detail.to_string(),
            Color::srgb(0.89, 0.68, 0.95),
            3.6,
        ));
    }
    if let Some(detail) = text.strip_prefix("Ember altar extinguished: ") {
        return Some((
            "ALTAR EXTINGUISHED",
            detail.to_string(),
            Color::srgb(0.34, 0.86, 0.54),
            3.4,
        ));
    }
    if text.starts_with("Reliquary seal ") {
        return Some((
            "SEAL RECOVERED",
            text.to_string(),
            Color::srgb(0.60, 0.86, 1.0),
            3.4,
        ));
    }
    if let Some(detail) = text.strip_prefix("Boss: ") {
        return Some((
            "BOSS AWAKENED",
            detail.to_string(),
            Color::srgb(1.0, 0.25, 0.16),
            3.8,
        ));
    }
    if let Some(detail) = text.strip_prefix("Milestone: ") {
        return Some((
            "MILESTONE",
            detail.to_string(),
            Color::srgb(0.74, 0.92, 1.0),
            3.0,
        ));
    }
    if let Some(detail) = text.strip_prefix("Bounty complete: ") {
        return Some((
            "BOUNTY COMPLETE",
            detail.to_string(),
            Color::srgb(0.48, 0.94, 0.68),
            3.2,
        ));
    }
    if let Some(detail) = text.strip_prefix("Blood obelisk completed: ") {
        return Some((
            "RITE COMPLETE",
            detail.to_string(),
            Color::srgb(1.0, 0.30, 0.22),
            3.2,
        ));
    }
    if let Some(detail) = text.strip_prefix("Ember rift sealed") {
        return Some((
            "RIFT SEALED",
            format!("Ember rift sealed{detail}"),
            Color::srgb(1.0, 0.48, 0.18),
            3.3,
        ));
    }
    if let Some(detail) = text.strip_prefix("Chapter reward: ") {
        return Some((
            "CHAPTER REWARD",
            detail.to_string(),
            Color::srgb(1.0, 0.76, 0.34),
            3.4,
        ));
    }
    if let Some(detail) = text.strip_prefix("Reliquary boon claimed: ") {
        return Some((
            "RELIQUARY BOON",
            detail.to_string(),
            Color::srgb(0.68, 0.88, 1.0),
            4.2,
        ));
    }
    if let Some(detail) = text.strip_prefix("Challenge rewards: ") {
        return Some((
            "CHALLENGE REWARD",
            detail.to_string(),
            Color::srgb(0.78, 0.95, 0.58),
            3.6,
        ));
    }
    if let Some(detail) = text.strip_prefix("Primal Ember Cache: ") {
        return Some((
            "PRIMAL CACHE",
            detail.to_string(),
            Color::srgb(1.0, 0.46, 0.20),
            3.4,
        ));
    }
    if let Some(detail) = text.strip_prefix("Primal Infusion complete: ") {
        return Some((
            "PRIMAL INFUSION",
            detail.to_string(),
            Color::srgb(1.0, 0.54, 0.24),
            3.6,
        ));
    }
    if let Some(detail) = text.strip_prefix("Ember Paragon ")
        && let Some(rank) = detail.strip_suffix(" awakened")
    {
        return Some((
            "EMBER PARAGON",
            format!("Rank {rank} awakened"),
            Color::srgb(1.0, 0.58, 0.22),
            3.6,
        ));
    }
    if let Some(detail) = text.strip_prefix("Chapter mastery ") {
        return Some((
            "MASTERY REWARD",
            detail.to_string(),
            Color::srgb(0.70, 0.84, 1.0),
            3.4,
        ));
    }
    if let Some(detail) = text.strip_prefix("Challenge complete: ") {
        return Some((
            "CHALLENGE COMPLETE",
            detail.to_string(),
            Color::srgb(0.78, 0.95, 0.58),
            3.0,
        ));
    }
    if text.starts_with("Salvaged ") && text.contains("salvage cache") {
        return Some((
            "SALVAGE CACHE",
            text.to_string(),
            Color::srgb(0.66, 0.92, 0.82),
            3.8,
        ));
    }

    None
}

fn chapter_milestone_toast_detail(detail: &str) -> String {
    let mut title = "";
    let mut next = None;
    let mut reward = None;
    let mut payoff = None;
    for (index, part) in detail.split(" | ").enumerate() {
        if index == 0 {
            title = part;
        } else if let Some(value) = part.strip_prefix("NEXT ") {
            next = Some(value);
        } else if let Some(value) = part.strip_prefix("REWARD ") {
            reward = Some(value);
        } else if let Some(value) = part.strip_prefix("PAYOFF ") {
            payoff = Some(value);
        }
    }

    let mut pieces = vec![title.to_string()];
    if let Some(next) = next {
        pieces.push(format!("Next: {next}"));
    }
    if let Some(reward) = reward {
        pieces.push(format!("Reward: {reward}"));
    } else if let Some(payoff) = payoff {
        pieces.push(format!("Payoff: {payoff}"));
    }
    compact_label(&pieces.join(" | "), 96)
}

fn update_skill_feedback(
    time: Res<Time>,
    mut state: ResMut<SkillFeedbackState>,
    mut events: MessageReader<CombatEvent>,
    mut banner: Query<(&mut Text, &mut TextColor), With<SkillFeedbackText>>,
) {
    let Ok((mut text, mut color)) = banner.single_mut() else {
        return;
    };

    for event in events.read() {
        let Some(feedback) = skill_feedback_from_event(&event.text) else {
            continue;
        };
        state.timer = Timer::from_seconds(feedback.duration_secs, TimerMode::Once);
        state.timer.reset();
        state.pulse = feedback.pulse;
        **text = feedback.copy;
        color.0 = feedback.color;
    }

    if state.timer.is_finished() {
        if !text.is_empty() {
            **text = String::new();
        }
        return;
    }

    state.timer.tick(time.delta());
    if state.timer.is_finished() {
        **text = String::new();
        state.pulse = None;
    }
}

#[derive(Debug, Clone)]
struct SkillFeedback {
    copy: String,
    color: Color,
    duration_secs: f32,
    pulse: Option<SkillSlotPulse>,
}

fn skill_feedback_from_event(text: &str) -> Option<SkillFeedback> {
    if let Some((copy, pulse)) = skill_failure_feedback_copy(text) {
        return Some(SkillFeedback {
            copy,
            color: Color::srgb(1.0, 0.34, 0.24),
            duration_secs: 1.35,
            pulse,
        });
    }
    if let Some((copy, break_event)) = combo_feedback_copy(text) {
        return Some(SkillFeedback {
            copy,
            color: if break_event {
                Color::srgb(1.0, 0.72, 0.18)
            } else {
                Color::srgb(0.66, 0.88, 1.0)
            },
            duration_secs: if break_event { 1.35 } else { 1.0 },
            pulse: skill_slot_pulse_from_event(text),
        });
    }
    if let Some(copy) = legendary_proc_feedback_copy(text) {
        return Some(SkillFeedback {
            copy,
            color: Color::srgb(1.0, 0.62, 0.20),
            duration_secs: 1.05,
            pulse: skill_slot_pulse_from_event(text),
        });
    }
    if let Some(copy) = survival_feedback_copy(text) {
        return Some(SkillFeedback {
            copy,
            color: Color::srgb(0.46, 1.0, 0.58),
            duration_secs: 1.2,
            pulse: skill_slot_pulse_from_event(text),
        });
    }
    if let Some(copy) = skill_success_feedback_copy(text) {
        return Some(SkillFeedback {
            copy,
            color: Color::srgb(0.66, 0.88, 1.0),
            duration_secs: 1.15,
            pulse: skill_slot_pulse_from_event(text),
        });
    }
    None
}

fn combo_feedback_copy(text: &str) -> Option<(String, bool)> {
    if let Some(detail) = text.strip_prefix("Combo Break: ") {
        return Some((combo_break_feedback_copy(detail), true));
    }
    if let Some(detail) = text.strip_prefix("Combo Ready: ") {
        return Some((combo_ready_feedback_copy(detail), false));
    }
    None
}

fn combo_ready_feedback_copy(detail: &str) -> String {
    let finisher = if detail.contains("Dash or Nova") {
        "Q/E FINISH"
    } else if detail.contains("Dash through Nova") {
        "Q DASH FINISH"
    } else {
        "FINISH NOW"
    };
    format!("COMBO READY\n{finisher}")
}

fn combo_break_feedback_copy(detail: &str) -> String {
    let payoff = detail
        .split_once(" hit ")
        .map(|(_, payoff)| {
            payoff
                .split(" | ")
                .next()
                .unwrap_or(payoff)
                .replace(" health", " HP")
                .to_ascii_uppercase()
        })
        .unwrap_or_else(|| "PAYOFF CLAIMED".to_string());
    format!("COMBO BREAK\n{payoff}")
}

fn legendary_proc_feedback_copy(text: &str) -> Option<String> {
    if text == "Emberbrand ignited enemy" {
        Some("EMBERBRAND IGNITE".to_string())
    } else if text == "Frostbrand chilled enemy" {
        Some("FROSTBRAND CHILL".to_string())
    } else if text.starts_with("Stormbrand chained for ") {
        Some("STORMBRAND CHAIN".to_string())
    } else if text.starts_with("Soulreaver restored ") {
        Some("SOULREAVER LEECH".to_string())
    } else if text.starts_with("Aegisbrand granted ") {
        Some("AEGISBRAND WARD".to_string())
    } else {
        None
    }
}

fn survival_feedback_copy(text: &str) -> Option<String> {
    if let Some(restored) = text
        .strip_prefix("Potion restored ")
        .and_then(|suffix| suffix.strip_suffix(" health"))
    {
        return Some(format!("POTION +{restored} HP"));
    }
    if text.starts_with("Health globe restored ") {
        return Some("HEALTH GLOBE".to_string());
    }
    if text.starts_with("Renewal well restored ") {
        return Some("RENEWAL WELL".to_string());
    }
    if text.starts_with("Ember altar extinguished: restored ") {
        return Some("EMBER ALTAR RESTORED".to_string());
    }
    if text.starts_with("Iron elixir: ") {
        return Some("IRON ELIXIR".to_string());
    }
    if text.starts_with("Wrath elixir: ") {
        return Some("WRATH ELIXIR".to_string());
    }
    if text.starts_with("Haste elixir: ") {
        return Some("HASTE ELIXIR".to_string());
    }
    None
}

fn skill_failure_feedback_copy(text: &str) -> Option<(String, Option<SkillSlotPulse>)> {
    if text.starts_with("Need ") && text.contains(" fury for ") {
        return Some((text.to_ascii_uppercase(), skill_slot_pulse_from_event(text)));
    }
    if text.ends_with("not ready 1s")
        || text.contains(" not ready ")
        || text.starts_with("Evade recharging ")
        || text.starts_with("Potion recharging ")
    {
        return Some((text.to_ascii_uppercase(), skill_slot_pulse_from_event(text)));
    }
    if text.starts_with("Nephalem Surge charging ") {
        return Some((text.to_string(), skill_slot_pulse_from_event(text)));
    }
    None
}

fn skill_success_feedback_copy(text: &str) -> Option<String> {
    if skill_cast_event_name(text).is_some() {
        return Some(text.to_ascii_uppercase());
    }
    match text {
        "Nephalem Surge unleashed" | "Nephalem Surge ready" | "Evade" => Some(text.to_string()),
        _ => None,
    }
}

fn skill_cast_event_name(text: &str) -> Option<&'static str> {
    let (rune, skill) = text.split_once(' ')?;
    match (rune, skill) {
        ("Cleanse" | "Reap", "Dash") => Some("Dash"),
        ("Expose" | "Hemorrhage", "Rupture") => Some("Rupture"),
        ("Ember" | "Frost", "Nova") => Some("Nova"),
        _ => None,
    }
}

fn skill_slot_pulse_from_event(text: &str) -> Option<SkillSlotPulse> {
    let (slot, kind) = if skill_cast_event_name(text) == Some("Dash") {
        (SkillSlotText::Dash, SkillSlotPulseKind::Cast)
    } else if skill_cast_event_name(text) == Some("Rupture") {
        (SkillSlotText::Rupture, SkillSlotPulseKind::Cast)
    } else if skill_cast_event_name(text) == Some("Nova") {
        (SkillSlotText::Nova, SkillSlotPulseKind::Cast)
    } else if text == "Nephalem Surge unleashed" || text == "Nephalem Surge ready" {
        (SkillSlotText::Surge, SkillSlotPulseKind::Cast)
    } else if text == "Evade"
        || text.starts_with("Potion restored ")
        || text.starts_with("Potion recharging ")
    {
        (SkillSlotText::Evade, SkillSlotPulseKind::Survival)
    } else if text.starts_with("Evade recharging ") {
        (SkillSlotText::Evade, SkillSlotPulseKind::Blocked)
    } else if text.starts_with("Nephalem Surge charging ") {
        (SkillSlotText::Surge, SkillSlotPulseKind::Blocked)
    } else if text.contains(" fury for dash") || text.starts_with("Dash not ready ") {
        (SkillSlotText::Dash, SkillSlotPulseKind::Blocked)
    } else if text.contains(" fury for rupture") || text.starts_with("Rupture not ready ") {
        (SkillSlotText::Rupture, SkillSlotPulseKind::Blocked)
    } else if text.contains(" fury for nova") || text.starts_with("Nova not ready ") {
        (SkillSlotText::Nova, SkillSlotPulseKind::Blocked)
    } else {
        return None;
    };
    Some(SkillSlotPulse { slot, kind })
}

fn update_combat_streak_banner(
    time: Res<Time>,
    mut state: ResMut<CombatStreakBannerState>,
    mut events: MessageReader<CombatEvent>,
    mut banner: Query<(&mut Text, &mut TextColor), With<CombatStreakBannerText>>,
) {
    let Ok((mut text, mut color)) = banner.single_mut() else {
        return;
    };

    for event in events.read() {
        let Some(streak) = combat_streak_banner_from_event(&event.text) else {
            continue;
        };
        state.timer = Timer::from_seconds(streak.duration_secs, TimerMode::Once);
        state.timer.reset();
        **text = streak.copy;
        color.0 = streak.color;
    }

    if state.timer.is_finished() {
        if !text.is_empty() {
            **text = String::new();
        }
        return;
    }

    state.timer.tick(time.delta());
    if state.timer.is_finished() {
        **text = String::new();
    }
}

#[derive(Debug, Clone)]
struct CombatStreakBanner {
    copy: String,
    color: Color,
    duration_secs: f32,
}

fn combat_streak_banner_from_event(text: &str) -> Option<CombatStreakBanner> {
    if let Some(copy) = massacre_banner_copy(text) {
        return Some(CombatStreakBanner {
            copy,
            color: Color::srgb(1.0, 0.72, 0.18),
            duration_secs: 1.8,
        });
    }
    if let Some(copy) = valor_banner_copy(text) {
        return Some(CombatStreakBanner {
            copy,
            color: Color::srgb(0.34, 0.82, 1.0),
            duration_secs: 1.6,
        });
    }
    None
}

fn massacre_banner_copy(text: &str) -> Option<String> {
    let (title, rest) = [
        ("Massacre", "Massacre "),
        ("Carnage", "Carnage "),
        ("Slaughter", "Slaughter "),
        ("Annihilation", "Annihilation "),
    ]
    .iter()
    .find_map(|(title, prefix)| text.strip_prefix(prefix).map(|rest| (*title, rest)))?;
    let (streak, rewards) = massacre_banner_parts(rest)?;
    let chase = massacre_banner_chase(streak);
    Some(format!(
        "{} {}X\n{} {}",
        title.to_ascii_uppercase(),
        streak,
        chase,
        rewards
    ))
}

fn massacre_banner_parts(rest: &str) -> Option<(u32, &str)> {
    let (streak_text, rewards) = rest.split_once('x')?;
    let streak = streak_text.trim().parse::<u32>().ok()?;
    let rewards = rewards.trim_start_matches(':').trim();
    Some((streak, rewards))
}

fn massacre_banner_chase(streak: u32) -> &'static str {
    if streak >= 20 {
        "Battle Trance pace"
    } else if streak >= 10 {
        "Carnage Master online"
    } else if streak >= 5 {
        "Massacre Rite online"
    } else {
        "Chain kills"
    }
}

fn valor_banner_copy(text: &str) -> Option<String> {
    let rest = text.strip_prefix("Valor ")?;
    let stacks = valor_banner_stacks(rest)?;
    let bonus = stacks.saturating_mul(8);
    Some(format!(
        "VALOR {}X\n{} +{}% elite rewards",
        stacks,
        valor_banner_chase(stacks),
        bonus
    ))
}

fn valor_banner_stacks(rest: &str) -> Option<u32> {
    let (stacks_text, _) = rest.split_once('x')?;
    stacks_text.trim().parse::<u32>().ok()
}

fn valor_banner_chase(stacks: u32) -> &'static str {
    match stacks {
        0 => "Hunt elites",
        1 => "Chain elites",
        2 => "Keep pressure",
        3 => "One pack from mastery",
        4 => "One elite to full Valor",
        _ => "Full Valor, push boss",
    }
}

fn chapter_banner_text(phase: ChapterPhase) -> &'static str {
    match phase {
        ChapterPhase::Breach => {
            "Breach the Ashen Reliquary\nNext: kill guards | Reward: cache path"
        }
        ChapterPhase::Cache => {
            "Seal Fragment: Reliquary Cache\nNext: open cache | Reward: outer seal"
        }
        ChapterPhase::Sanctum => {
            "Sanctum Ambush: Hold the Inner Hall\nNext: break Vhal | Reward: second seal"
        }
        ChapterPhase::Ritual => {
            "Final Seal: Ember Altar\nNext: extinguish altar | Reward: essence + boss gate"
        }
        ChapterPhase::Boss => {
            "Malrec, Keeper of Ash, Has Awakened\nNext: break stagger | Reward: soul sigil cache"
        }
        ChapterPhase::Victory => {
            "The Ashen Reliquary Is Cleansed\nNext: claim cache | Reward: account progress"
        }
    }
}

fn chapter_banner_duration(phase: ChapterPhase) -> f32 {
    match phase {
        ChapterPhase::Boss | ChapterPhase::Victory => 4.0,
        _ => 2.8,
    }
}

fn update_hud(resources: HudResources, queries: HudQueryParams) {
    let HudQueryParams {
        vitals,
        build,
        boon,
        paragon,
        codex,
        charm,
        armory,
        conduit,
        glory,
        ward,
        boss,
        mut texts,
        mut nodes,
    } = queries;
    let HudResources {
        stats,
        progress,
        story,
        combat_log,
        pause,
        inventory_open,
        build_open,
        journal_open,
        audio,
        audio_backend,
        difficulty,
        journal,
        milestones,
        bestiary,
        bounty,
        modifier,
        challenges,
        mastery,
        obelisk,
        rift,
        records,
        zone,
        skill_feedback,
        target_info,
        hud_density,
    } = resources;
    let Ok((health, fury, cooldowns, evade, surge, _portal, level, potions, elixirs, transform)) =
        vitals.single()
    else {
        return;
    };
    let Ok((
        damage_bonus,
        equipment,
        _buff,
        elixir_buff,
        _fortune_buff,
        inventory,
        talents,
        runes,
        companion,
    )) = build.single()
    else {
        return;
    };
    let Ok(boon) = boon.single() else {
        return;
    };
    let Ok(paragon) = paragon.single() else {
        return;
    };
    let Ok(codex) = codex.single() else {
        return;
    };
    let Ok(charm) = charm.single() else {
        return;
    };
    let Ok(armory) = armory.single() else {
        return;
    };
    let Ok(_conduit_buff) = conduit.single() else {
        return;
    };
    let Ok(_glory_buff) = glory.single() else {
        return;
    };
    let Ok((death_ward, barrier)) = ward.single() else {
        return;
    };
    let boss_text = boss
        .single()
        .map(|(health, phase, stagger, staggered)| {
            format!(
                "{} | {}",
                boss_pressure_summary(health, phase),
                boss_stagger_summary(stagger, staggered)
            )
        })
        .unwrap_or_else(|_| {
            if progress.phase == ChapterPhase::Boss {
                "Malrec: entering".to_string()
            } else {
                "Malrec: dormant".to_string()
            }
        });
    let boss_action =
        boss.single()
            .ok()
            .map(|(_, phase, stagger, staggered)| BossActionGuideState {
                phase_two_started: phase.phase_two_started(),
                enrage_started: phase.enrage_started(),
                enrage_remaining_secs: phase.enrage_remaining_secs(),
                stagger_meter: stagger.map(|stagger| stagger.meter).unwrap_or_default(),
                stagger_threshold: stagger.map(|stagger| stagger.threshold).unwrap_or_default(),
                staggered_remaining_secs: staggered
                    .map(|staggered| staggered.timer.remaining_secs())
                    .unwrap_or_default(),
            });
    let journey_text = journey_summary(&challenges, &milestones, &mastery);
    let quest_log_text = chapter_quest_log_summary(&progress, &stats, &bounty, &obelisk, &rift);
    let clear_summary_text =
        chapter_clear_summary(&progress, &stats, &bounty, &obelisk, &rift, &story);
    let bestiary_goal_text = bestiary_next_goal(&bestiary);
    let codex_goal_text = legendary_codex_pursuit_summary(codex, equipment);
    let primal_goal_text = primal_infusion_pursuit_summary(&stats, equipment);
    let mastery_goal_text =
        mastery_next_goal_summary(&stats, &bounty, &challenges, &journal, codex);
    let action_guide_base = action_guide_summary(ActionGuideInputs {
        progress: &progress,
        stats: &stats,
        boon,
        bounty: &bounty,
        obelisk: &obelisk,
        rift: &rift,
        health,
        potions,
        death_ward,
        surge,
        level,
        talents,
        equipment,
        companion,
        boss: boss_action,
    });
    let action_guide_base = action_guide_with_journey(
        action_guide_base,
        JourneyActionGuideContext {
            progress: &progress,
            health,
            target_info: &target_info,
            obelisk: &obelisk,
            rift: &rift,
            surge,
            challenges: &challenges,
            milestones: &milestones,
            mastery: &mastery,
        },
    );
    let action_guide_text = action_guide_with_ordeal(action_guide_base, &modifier, &progress);
    let combat_focus = hud_combat_focus_active(
        &progress,
        health,
        &target_info,
        boss_action,
        &obelisk,
        &rift,
    );
    let resonance_text = reliquary_resonance(equipment, charm)
        .map(|resonance| resonance.summary())
        .unwrap_or_else(|| "Resonance: inactive".to_string());
    let set_tactic_text = reliquary_set_tactic(equipment, charm, &stats);
    let gem_resonance_text = equipment
        .socketed_gem
        .map(|gem| gem.resonance_summary())
        .unwrap_or_else(|| "Gem Resonance: socket gems to awaken power".to_string());
    let detailed_status_text = character_status_summary(CharacterStatusInputs {
        level,
        paragon,
        equipment,
        elixir_buff,
        death_ward,
        resonance: &resonance_text,
        set_tactic: &set_tactic_text,
        gem_resonance: &gem_resonance_text,
        companion,
        boss: &boss_text,
    });
    let systems_status_text = systems_status_summary(SystemsStatusInputs {
        progress: &progress,
        stats: &stats,
        bounty: &bounty,
        challenges: &challenges,
        obelisk: &obelisk,
        rift: &rift,
        modifier: &modifier,
        zone: &zone,
    });
    let live_priority_text = live_objective_priority(&stats, &bounty, &obelisk, &rift);
    let clean_hud = clean_hud_active(
        *hud_density,
        &pause,
        &inventory_open,
        &build_open,
        &journal_open,
    );

    if let Ok(mut text) = texts.0.single_mut() {
        **text = if combat_focus {
            hud_combat_focus_text(&action_guide_text, &target_info, &progress)
        } else if clean_hud {
            hud_clean_objective_text(
                &progress,
                &zone,
                transform.translation,
                live_priority_text.as_deref(),
            )
        } else {
            hud_objective_text(
                &progress,
                &zone,
                transform.translation,
                live_priority_text.as_deref(),
            )
        };
    }
    update_objective_frame(&mut nodes.objective_frames, combat_focus, clean_hud);
    if let Ok(mut text) = texts.1.single_mut() {
        **text = health_orb_text(health, barrier, potions, death_ward);
    }
    for (slot, mut text) in &mut texts.2 {
        **text = skill_slot_summary(*slot, fury, cooldowns, runes, surge, evade);
    }
    update_resource_fills(&mut nodes.resource_fills, health, barrier, fury);
    update_resource_orb_frames(&mut nodes.resource_orb_frames, health, barrier, fury, surge);
    update_potion_pips(&mut nodes.potion_pips, potions);
    update_skill_frames(
        &mut nodes.skill_frames,
        fury,
        cooldowns,
        surge,
        evade,
        &skill_feedback,
    );
    update_skill_cooldown_masks(&mut nodes.skill_masks, fury, cooldowns, surge, evade);
    update_skill_lamps(&mut nodes.skill_lamps, fury, cooldowns, surge, evade);
    update_experience_fills(&mut nodes.experience_fills, level);
    update_inventory_panels(&mut nodes.inventory_panels, inventory_open.open);
    update_build_panels(&mut nodes.build_panels, build_open.open);
    if let Ok(mut text) = texts.3.single_mut() {
        let readiness = combat_readiness_summary(CombatReadinessInputs {
            health,
            potions,
            fury,
            cooldowns,
            surge,
            evade,
            runes,
        });
        **text = fury_orb_text(fury, level, paragon, &stats, &readiness);
    }
    if let Ok(mut text) = texts.4.single_mut() {
        let (log_text, display) =
            combat_log_panel_text(combat_log.lines(), combat_focus || clean_hud);
        **text = log_text;
        for mut visibility in &mut nodes.log_visibility {
            *visibility = visibility_from_display(display);
        }
    }
    if let Ok(mut text) = texts.5.single_mut() {
        **text = if pause.paused {
            pause_help_text(PauseHelpInputs {
                progress: &progress,
                difficulty: &difficulty,
                modifier: &modifier,
                stats: &stats,
                journey: &journey_text,
                clear_summary: &clear_summary_text,
                season_deeds: &season_deed_pursuit_summary(&records),
                bestiary_goal: &bestiary_goal_text,
                codex_goal: &codex_goal_text,
                crafting_goal: &primal_goal_text,
                mastery_goal: &mastery_goal_text,
                story: &story,
                quest_log: &quest_log_text,
                action_guide: &action_guide_text,
                detailed_status: &detailed_status_text,
                systems_status: &systems_status_text,
                inventory_open: inventory_open.open,
                journal_open: journal_open.open,
                audio_status: audio_backend.status_label(&audio),
            })
        } else {
            String::new()
        };
    }
    if let Ok(mut text) = texts.6.single_mut() {
        **text = if inventory_open.open {
            inventory_panel_header(
                &stats,
                inventory,
                equipment,
                damage_bonus,
                potions,
                elixirs,
                codex,
            )
        } else {
            String::new()
        };
    }
    let resonance_text = reliquary_resonance(equipment, charm)
        .map(|resonance| resonance.summary())
        .unwrap_or_else(|| "Resonance: inactive".to_string());
    update_gear_slots(
        &mut texts.7,
        inventory_open.open,
        damage_bonus,
        equipment,
        charm,
        codex,
        armory,
        talents,
        runes,
        &resonance_text,
    );
    update_gear_slot_frames(&mut nodes.gear_frames, inventory_open.open);
    update_inventory_slots(
        &mut texts.8,
        inventory_open.open,
        inventory,
        equipment,
        damage_bonus,
    );
    update_inventory_slot_frames(
        &mut nodes.inventory_frames,
        inventory_open.open,
        inventory,
        equipment,
    );
    if let Ok(mut text) = texts.9.single_mut() {
        let quest_board =
            chapter_quest_board_text(&progress, &stats, &bounty, &obelisk, &rift, &story);
        let pause_summary = format!(
            "{}\nLore: {}\nProgress: {}\nBestiary: {} | {}",
            quest_board,
            compact_label(&journal.summary(), 112),
            compact_label(&milestone_summary(&milestones), 112),
            compact_label(&bestiary_summary(&bestiary), 64),
            compact_label(&bestiary_goal_text, 64)
        );
        let (journal_text, display) = journal_panel_text(
            journal_open.open,
            pause.paused,
            &journal.open_text(),
            &pause_summary,
        );
        **text = journal_text;
        for mut visibility in &mut nodes.journal_visibility {
            *visibility = visibility_from_display(display);
        }
    }
    if let Ok(mut text) = texts.10.single_mut() {
        **text = if build_open.open {
            build_panel_text(BuildPanelInputs {
                health,
                fury,
                cooldowns,
                evade,
                surge,
                level,
                damage_bonus,
                equipment,
                charm,
                talents,
                runes,
                codex,
            })
        } else {
            String::new()
        };
    }
}

fn update_action_belt_hud(
    mut texts: Query<(&mut Text, &mut Node), With<ActionBeltText>>,
    player: ActionBeltPlayerQuery,
    resources: ActionBeltResources,
) {
    let Ok((mut text, mut node)) = texts.single_mut() else {
        return;
    };
    let Ok((
        health,
        barrier,
        fury,
        cooldowns,
        evade,
        surge,
        boon,
        potions,
        inventory,
        equipment,
        damage_bonus,
        runes,
        talents,
    )) = player.single()
    else {
        return;
    };
    let inputs = ActionBeltInputs {
        health,
        barrier,
        fury,
        cooldowns,
        evade,
        surge,
        boon,
        potions,
        inventory,
        equipment,
        damage_bonus,
        runes,
        talents,
        stats: &resources.stats,
        progress: &resources.progress,
        bounty: &resources.bounty,
        obelisk: &resources.obelisk,
        rift: &resources.rift,
        combo: Some(&resources.combo),
        target_info: Some(&resources.target_info),
    };

    node.display = Display::Flex;
    **text = if clean_hud_active(
        *resources.hud_density,
        &resources.pause,
        &resources.inventory_open,
        &resources.build_open,
        &resources.journal_open,
    ) {
        action_belt_clean_summary(inputs)
    } else {
        action_belt_summary(inputs)
    };
}

#[derive(SystemParam)]
struct ActionBeltResources<'w> {
    stats: Res<'w, RunStats>,
    progress: Res<'w, ChapterProgress>,
    bounty: Res<'w, BountyProgress>,
    obelisk: Res<'w, BloodObelisk>,
    rift: Res<'w, EmberRift>,
    combo: Res<'w, CombatComboState>,
    target_info: Res<'w, TargetInfo>,
    hud_density: Res<'w, HudDensity>,
    pause: Res<'w, PauseState>,
    inventory_open: Res<'w, InventoryOpen>,
    build_open: Res<'w, BuildOpen>,
    journal_open: Res<'w, JournalOpen>,
}

#[derive(Clone, Copy)]
struct ActionBeltInputs<'a> {
    health: &'a Health,
    barrier: &'a Barrier,
    fury: &'a Fury,
    cooldowns: &'a SkillCooldowns,
    evade: &'a Evade,
    surge: &'a SurgeBuff,
    boon: &'a ChapterBoon,
    potions: &'a PotionBelt,
    inventory: &'a Inventory,
    equipment: &'a Equipment,
    damage_bonus: &'a DamageBonus,
    runes: &'a SkillRunes,
    talents: &'a Talents,
    stats: &'a RunStats,
    progress: &'a ChapterProgress,
    bounty: &'a BountyProgress,
    obelisk: &'a BloodObelisk,
    rift: &'a EmberRift,
    combo: Option<&'a CombatComboState>,
    target_info: Option<&'a TargetInfo>,
}

fn action_belt_summary(inputs: ActionBeltInputs) -> String {
    let hp = action_belt_health(inputs.health, inputs.barrier);
    let fury = action_belt_fury_state(&inputs);
    let next = action_belt_next_action(&inputs);
    let combo = action_belt_combo_hint(&inputs);
    let bag = compact_label(
        &bag_action_summary(
            inputs.stats,
            inputs.inventory,
            inputs.equipment,
            inputs.damage_bonus,
        ),
        12,
    );
    let priority =
        live_objective_priority(inputs.stats, inputs.bounty, inputs.obelisk, inputs.rift)
            .unwrap_or_else(|| chapter_next_step(inputs.progress));

    format!(
        "{hp} | {fury} | F {} | Next {} | Combo {} | Bag {}/{} {} | {}g | {}",
        action_belt_potion_state(inputs.potions),
        next,
        combo,
        inputs.inventory.items.len(),
        inputs.inventory.capacity,
        bag,
        inputs.stats.gold,
        compact_label(&priority, 22)
    )
}

fn action_belt_clean_summary(inputs: ActionBeltInputs) -> String {
    let hp = compact_label(&action_belt_health(inputs.health, inputs.barrier), 14);
    let skills = compact_label(&action_belt_skill_snapshot(&inputs), 24);
    let next = compact_label(&action_belt_next_action(&inputs), 28);
    let combo = action_belt_clean_combo_hint(&inputs);
    let priority =
        live_objective_priority(inputs.stats, inputs.bounty, inputs.obelisk, inputs.rift)
            .unwrap_or_else(|| chapter_next_step(inputs.progress));
    compact_label(
        &format!(
            "{hp} | Fury {:.0}/{:.0} {skills} | F {} | Next {}{} | Bag {}/{} | {}",
            inputs.fury.current,
            inputs.fury.max,
            action_belt_potion_state(inputs.potions),
            next,
            combo,
            inputs.inventory.items.len(),
            inputs.inventory.capacity,
            compact_label(&priority, 20)
        ),
        118,
    )
}

fn action_belt_clean_combo_hint(inputs: &ActionBeltInputs) -> String {
    action_belt_clean_combo_hint_from(inputs.combo.and_then(CombatComboState::hud_hint))
}

fn action_belt_clean_combo_hint_from(hint: Option<String>) -> String {
    hint.map(|hint| format!(" | Combo {}", compact_label(&hint, 18)))
        .unwrap_or_default()
}

fn action_belt_health(health: &Health, barrier: &Barrier) -> String {
    let hp_label = if health.current <= health.max * 0.35 {
        "HP!"
    } else {
        "HP"
    };
    if barrier.current > 0.5 {
        format!(
            "{hp_label} {:.0}/{:.0} +{:.0}",
            health.current, health.max, barrier.current
        )
    } else {
        format!("{hp_label} {:.0}/{:.0}", health.current, health.max)
    }
}

fn action_belt_fury_state(inputs: &ActionBeltInputs) -> String {
    let min_spender = inputs
        .fury
        .rupture_cost
        .min(inputs.fury.dash_cost)
        .min(inputs.fury.nova_cost);
    let stance = if surge_seconds_remaining(inputs.surge) > 0.0 {
        "surge"
    } else if inputs.fury.current + f32::EPSILON < min_spender {
        "build"
    } else if inputs.fury.current >= inputs.fury.nova_cost
        && inputs.fury.current >= inputs.fury.rupture_cost
    {
        "spend"
    } else {
        "ready"
    };
    let skills = action_belt_skill_snapshot(inputs);
    compact_label(
        &format!(
            "Fury {:.0}/{:.0} {stance} {skills}",
            inputs.fury.current, inputs.fury.max
        ),
        42,
    )
}

fn action_belt_skill_snapshot(inputs: &ActionBeltInputs) -> String {
    let min_spender = inputs
        .fury
        .rupture_cost
        .min(inputs.fury.dash_cost)
        .min(inputs.fury.nova_cost);
    if inputs.fury.current + f32::EPSILON < min_spender {
        return format!(
            "LMB +{:.0}F",
            (min_spender - inputs.fury.current).ceil().max(1.0)
        );
    }

    let mut tokens = vec![
        action_belt_skill_token(SkillSlotText::Rupture, "RMB", inputs),
        action_belt_skill_token(SkillSlotText::Dash, "Q", inputs),
        action_belt_skill_token(SkillSlotText::Nova, "E", inputs),
    ];
    tokens.push(action_belt_surge_token(inputs.surge));
    tokens.join("/")
}

fn action_belt_skill_token(
    slot: SkillSlotText,
    key: &'static str,
    inputs: &ActionBeltInputs,
) -> String {
    if skill_slot_ready(
        slot,
        inputs.fury,
        inputs.cooldowns,
        inputs.surge,
        inputs.evade,
    ) {
        return key.to_string();
    }
    if let Some(required) = skill_slot_required_fury(slot, inputs.fury)
        && inputs.fury.current + f32::EPSILON < required
    {
        return format!(
            "{key}+{:.0}F",
            (required - inputs.fury.current).ceil().max(1.0)
        );
    }

    let cooldown = match slot {
        SkillSlotText::Rupture => inputs.cooldowns.rupture.remaining_secs(),
        SkillSlotText::Dash => inputs.cooldowns.dash.remaining_secs(),
        SkillSlotText::Nova => inputs.cooldowns.nova.remaining_secs(),
        _ => 0.0,
    };
    if cooldown > 0.05 {
        format!("{key}{:.0}s", cooldown.ceil())
    } else {
        key.to_string()
    }
}

fn action_belt_surge_token(surge: &SurgeBuff) -> String {
    let active = surge_seconds_remaining(surge);
    if active > 0.0 {
        format!("Y{:.0}s", active.ceil())
    } else if surge.charge >= surge.max_charge {
        "Y".to_string()
    } else {
        format!("Y{}/{}", surge.charge, surge.max_charge)
    }
}

fn action_belt_potion_state(potions: &PotionBelt) -> String {
    let cooldown = potion_seconds_remaining(potions);
    if cooldown > 0.05 {
        format!("{:.0}s", cooldown.ceil())
    } else if potions.current == 0 {
        "EMPTY".to_string()
    } else {
        format!("{}/{}", potions.current, potions.max)
    }
}

fn action_belt_next_action(inputs: &ActionBeltInputs) -> String {
    if inputs.health.current <= inputs.health.max * 0.35 {
        return survival_readiness_summary(inputs.potions, inputs.evade)
            .replace("Potion heal", "heal")
            .replace("Evade", "evade");
    }
    if let Some(target_prompt) = inputs
        .target_info
        .and_then(|target| action_belt_target_prompt(target, inputs.evade))
    {
        return target_prompt;
    }
    if let Some(boon_prompt) = action_belt_boss_boon_prompt(inputs) {
        return boon_prompt;
    }
    if inputs.progress.phase == ChapterPhase::Boss
        && inputs.boon.choice != ChapterBoonChoice::None
        && inputs.stats.reliquary_momentum > 0
    {
        return format!(
            "Momentum {}x burst",
            inputs.stats.reliquary_momentum.min(RELIQUARY_MOMENTUM_MAX)
        );
    }
    if let Some(talent_prompt) = action_belt_talent_prompt(inputs.talents) {
        return talent_prompt;
    }
    if let Some(inventory_prompt) = action_belt_inventory_prompt(inputs) {
        return inventory_prompt;
    }
    if surge_seconds_remaining(inputs.surge) > 0.0 {
        return action_belt_rotation(
            [
                (SkillSlotText::Rupture, rupture_action_label(inputs.runes)),
                (SkillSlotText::Nova, nova_action_label(inputs.runes)),
                (SkillSlotText::Dash, dash_action_label(inputs.runes)),
            ],
            inputs,
            "LMB keep fury",
        );
    }
    if skill_slot_ready(
        SkillSlotText::Surge,
        inputs.fury,
        inputs.cooldowns,
        inputs.surge,
        inputs.evade,
    ) {
        return "Y burst -> RMB/E".to_string();
    }
    action_belt_rotation(
        [
            (SkillSlotText::Rupture, rupture_action_label(inputs.runes)),
            (SkillSlotText::Nova, nova_action_label(inputs.runes)),
            (SkillSlotText::Dash, dash_action_label(inputs.runes)),
        ],
        inputs,
        "LMB build fury",
    )
}

fn action_belt_boss_boon_prompt(inputs: &ActionBeltInputs) -> Option<String> {
    if inputs.progress.phase != ChapterPhase::Boss || inputs.boon.choice != ChapterBoonChoice::None
    {
        return None;
    }
    Some(format!(
        "4/5/6 boon rec {}",
        chapter_boon_recommended_key(
            inputs.stats,
            inputs.health,
            inputs.potions,
            inputs.equipment
        )
    ))
}

fn action_belt_talent_prompt(talents: &Talents) -> Option<String> {
    if talents.points == 0 {
        return None;
    }
    Some(format!("K {}", next_talent_key(talents)))
}

fn action_belt_inventory_prompt(inputs: &ActionBeltInputs) -> Option<String> {
    if inputs.inventory.items.len() >= inputs.inventory.capacity {
        if let Some(item) =
            best_inventory_upgrade(inputs.inventory, inputs.equipment, inputs.damage_bonus)
        {
            return Some(format!(
                "FULL {}",
                action_belt_inventory_item_prompt(item, inputs.equipment, inputs.damage_bonus)
            ));
        }
        let salvageable =
            salvageable_inventory_count(inputs.inventory, inputs.equipment, inputs.damage_bonus);
        return Some(format!("FULL U salvage {}", salvageable.max(1)));
    }
    if let Some(item) =
        best_inventory_upgrade(inputs.inventory, inputs.equipment, inputs.damage_bonus)
    {
        return Some(action_belt_inventory_item_prompt(
            item,
            inputs.equipment,
            inputs.damage_bonus,
        ));
    }
    if let Some(item) =
        best_inventory_build_piece(inputs.inventory, inputs.equipment, inputs.damage_bonus)
    {
        return Some(action_belt_inventory_item_prompt(
            item,
            inputs.equipment,
            inputs.damage_bonus,
        ));
    }
    let salvageable =
        salvageable_inventory_count(inputs.inventory, inputs.equipment, inputs.damage_bonus);
    if salvageable > 0 {
        Some(format!("U salvage {salvageable}"))
    } else {
        None
    }
}

fn action_belt_inventory_item_prompt(
    item: &InventoryItem,
    equipment: &Equipment,
    damage_bonus: &DamageBonus,
) -> String {
    let gain = compact_label(
        &inventory_item_action_gain(item, equipment, damage_bonus),
        10,
    );
    match inventory_item_action_hint(item, equipment, damage_bonus) {
        "[ equip power" | "[ equip now" => format!("[ equip {gain}"),
        "B codex" => format!("B codex {gain}"),
        "keep craft" => format!("I craft {gain}"),
        "stash swap" => format!("I swap {gain}"),
        "U salvage" => format!("U salvage {gain}"),
        _ => format!("I inspect {gain}"),
    }
}

fn action_belt_combo_hint(inputs: &ActionBeltInputs) -> String {
    if let Some(hint) = inputs.combo.and_then(CombatComboState::hud_hint) {
        return hint;
    }
    if inputs.health.current <= inputs.health.max * 0.35 {
        return survival_combo_hint(inputs.potions, inputs.evade);
    }
    if inputs.fury.current < inputs.fury.rupture_cost.min(inputs.fury.nova_cost) {
        return format!(
            "LMB to {:.0}F",
            inputs.fury.rupture_cost.min(inputs.fury.nova_cost)
        );
    }
    if surge_seconds_remaining(inputs.surge) > 0.0 {
        return skill_combo_sequence(inputs.runes, true);
    }
    if inputs.surge.charge >= inputs.surge.max_charge {
        return format!("Y -> {}", skill_combo_sequence(inputs.runes, true));
    }
    skill_combo_sequence(inputs.runes, false)
}

fn survival_combo_hint(potions: &PotionBelt, evade: &Evade) -> String {
    if potions.current > 0 && potion_seconds_remaining(potions) <= 0.05 {
        "F -> Shift".to_string()
    } else if evade_cooldown_remaining(evade) <= 0.05 {
        "Shift -> LMB".to_string()
    } else {
        "kite -> LMB".to_string()
    }
}

fn skill_combo_sequence(runes: &SkillRunes, burst: bool) -> String {
    let setup = match runes.rupture {
        RuptureRune::Expose => "RMB vuln",
        RuptureRune::Hemorrhage => "RMB bleed",
    };
    let finisher = match (runes.nova, runes.dash, burst) {
        (NovaRune::Frost, _, _) => "E freeze",
        (NovaRune::Ember, DashRune::Reap, true) => "Q reap",
        (NovaRune::Ember, _, _) => "E burn",
    };
    format!("{setup} -> {finisher}")
}

fn action_belt_rotation(
    priorities: [(SkillSlotText, &'static str); 3],
    inputs: &ActionBeltInputs,
    fallback: &'static str,
) -> String {
    for (slot, label) in priorities {
        if skill_slot_ready(
            slot,
            inputs.fury,
            inputs.cooldowns,
            inputs.surge,
            inputs.evade,
        ) {
            return label.to_string();
        }
    }
    if let Some(required) = [
        SkillSlotText::Rupture,
        SkillSlotText::Nova,
        SkillSlotText::Dash,
    ]
    .into_iter()
    .filter_map(|slot| skill_slot_required_fury(slot, inputs.fury))
    .filter(|required| inputs.fury.current < *required)
    .min_by(|a, b| a.total_cmp(b))
    {
        return format!("LMB +{:.0}F", (required - inputs.fury.current).ceil());
    }
    fallback.to_string()
}

fn action_belt_target_prompt(target: &TargetInfo, evade: &Evade) -> Option<String> {
    if !target.visible {
        return None;
    }
    let details = target.details.as_str();
    if details.contains("window: burst now") {
        return Some("Y burst window".to_string());
    }
    if details.contains("break before enrage") {
        return Some("RMB/Q break".to_string());
    }
    if details.contains("kite fire") {
        return Some(evade_or_move_prompt(evade, "kite fire"));
    }
    if let Some(incoming) = details
        .split("incoming: ")
        .nth(1)
        .and_then(|tail| tail.split('|').next())
        .map(str::trim)
    {
        return Some(evade_or_move_prompt(
            evade,
            compact_label(&strip_trailing_seconds(incoming), 15).as_str(),
        ));
    }
    if details.contains("stop into reflect") || details.contains("reflect") {
        return Some("stop reflect".to_string());
    }
    if details.contains("wait shield") || details.contains("shield window") {
        return Some("wait shield".to_string());
    }
    if details.contains("save Shift") || details.contains("root trap") {
        return Some("hold Shift root".to_string());
    }
    if details.contains("leave circle")
        || details.contains("move out")
        || details.contains("step off death pool")
        || details.contains("ground fire")
        || details.contains("death pool")
        || details.contains("seal rune")
    {
        return Some(evade_or_move_prompt(evade, "move out"));
    }
    if details.contains("strafe beam") || details.contains("beam hazard") {
        return Some("strafe beam".to_string());
    }
    if details.contains("flank shield guard") || details.contains("role: guard") {
        return Some("flank guard".to_string());
    }
    if details.contains("dash through fire") || details.contains("role: caster") {
        return Some(evade_or_move_prompt(evade, "dash fire"));
    }
    if details.contains("break stagger") || details.contains("role: boss") {
        return Some("RMB/Q stagger".to_string());
    }
    if details.contains("ward broken") {
        return Some("RMB/E punish".to_string());
    }
    if details.contains("break ward") || details.contains("role: warden") {
        return Some(evade_or_move_prompt(evade, "break warden"));
    }
    if details.contains("role: heavy") {
        return Some(evade_or_move_prompt(evade, "dodge heavy"));
    }
    if details.contains("role: marksman") {
        return Some("close marksman".to_string());
    }
    if details.contains("role: chaser") {
        return Some(evade_or_move_prompt(evade, "kite chaser"));
    }
    if details.contains("role: treasure") {
        return Some("chase treasure".to_string());
    }
    if details.contains("role: nemesis") {
        return Some(evade_or_move_prompt(evade, "kite nemesis"));
    }
    None
}

fn evade_or_move_prompt(evade: &Evade, action: &str) -> String {
    if evade_cooldown_remaining(evade) <= 0.05 {
        format!("Shift {action}")
    } else {
        action.to_string()
    }
}

fn strip_trailing_seconds(text: &str) -> String {
    let mut parts = text.split_whitespace().collect::<Vec<_>>();
    if parts
        .last()
        .is_some_and(|last| last.ends_with('s') && last[..last.len() - 1].parse::<f32>().is_ok())
    {
        parts.pop();
    }
    parts.join(" ")
}

fn rupture_action_label(runes: &SkillRunes) -> &'static str {
    match runes.rupture {
        RuptureRune::Expose => "RMB vuln",
        RuptureRune::Hemorrhage => "RMB bleed",
    }
}

fn nova_action_label(runes: &SkillRunes) -> &'static str {
    match runes.nova {
        NovaRune::Ember => "E burn",
        NovaRune::Frost => "E freeze",
    }
}

fn dash_action_label(runes: &SkillRunes) -> &'static str {
    match runes.dash {
        DashRune::Cleanse => "Q cleanse",
        DashRune::Reap => "Q reap",
    }
}

#[allow(clippy::too_many_arguments)]
fn update_buff_bar(
    mut texts: Query<(&mut Text, &mut Node), With<BuffBarText>>,
    player: BuffBarPlayerQuery,
    progress: Res<ChapterProgress>,
    target_info: Res<TargetInfo>,
    obelisk: Res<BloodObelisk>,
    rift: Res<EmberRift>,
    hud_density: Res<HudDensity>,
    pause: Res<PauseState>,
    inventory_open: Res<InventoryOpen>,
    build_open: Res<BuildOpen>,
    journal_open: Res<JournalOpen>,
) {
    let Ok((relic, elixir, fortune, conduit, glory, surge, portal, health, burning, jailed)) =
        player.single()
    else {
        return;
    };
    let Ok((mut text, mut node)) = texts.single_mut() else {
        return;
    };

    let clean_hud = clean_hud_active(
        *hud_density,
        &pause,
        &inventory_open,
        &build_open,
        &journal_open,
    );
    let combat_focus =
        clean_hud || hud_top_clear_active(&progress, health, &target_info, &obelisk, &rift);
    let buff_text = buff_bar_text(BuffBarInputs {
        relic,
        elixir,
        fortune,
        conduit,
        glory,
        surge,
        portal,
        health,
        burning,
        jailed,
    });
    **text = top_buff_bar_text(&buff_text, combat_focus);
    node.display = if text.is_empty() {
        Display::None
    } else {
        Display::Flex
    };
}

struct BuffBarInputs<'a> {
    relic: &'a RelicBuff,
    elixir: &'a ElixirBuff,
    fortune: &'a FortuneBuff,
    conduit: &'a ConduitBuff,
    glory: &'a GloryBuff,
    surge: &'a SurgeBuff,
    portal: &'a TownPortal,
    health: &'a Health,
    burning: Option<&'a Burning>,
    jailed: Option<&'a Jailed>,
}

fn buff_bar_text(inputs: BuffBarInputs) -> String {
    let mut labels = Vec::new();

    if player_health_danger(inputs.health) {
        labels.push("DANGER low life - heal now".to_string());
    }
    if let Some(burning) = inputs.burning {
        push_timed_buff(
            &mut labels,
            "BURNING - heal or cleanse",
            burning_seconds_remaining(burning),
        );
    }
    if let Some(jailed) = inputs.jailed {
        push_timed_buff(
            &mut labels,
            "ROOTED - break chains",
            jailed_seconds_remaining(jailed),
        );
    }
    push_timed_buff(&mut labels, "Shrine", buff_seconds_remaining(inputs.relic));
    push_timed_buff(
        &mut labels,
        &format!("{} Elixir", inputs.elixir.kind.label()),
        elixir_seconds_remaining(inputs.elixir),
    );
    push_timed_buff(
        &mut labels,
        "Fortune",
        fortune_seconds_remaining(inputs.fortune),
    );
    push_timed_buff(
        &mut labels,
        "Conduit",
        conduit_seconds_remaining(inputs.conduit),
    );

    let glory_seconds = glory_seconds_remaining(inputs.glory);
    if glory_seconds > 0.0 && inputs.glory.stacks > 0 {
        labels.push(format!(
            "Glory {}x {}s",
            inputs.glory.stacks,
            hud_seconds(glory_seconds)
        ));
    }

    push_timed_buff(&mut labels, "Surge", surge_seconds_remaining(inputs.surge));

    let portal_seconds = town_portal_seconds_remaining(inputs.portal);
    if portal_seconds > 0.0 {
        let label = if inputs.portal.return_position.is_some() {
            "Return Portal"
        } else {
            "Town Portal"
        };
        push_timed_buff(&mut labels, label, portal_seconds);
    }

    compact_buff_labels(labels).join("  |  ")
}

fn top_buff_bar_text(buff_text: &str, combat_focus: bool) -> String {
    if combat_focus {
        String::new()
    } else {
        buff_text.to_string()
    }
}

fn hud_top_clear_active(
    progress: &ChapterProgress,
    health: &Health,
    target_info: &TargetInfo,
    obelisk: &BloodObelisk,
    rift: &EmberRift,
) -> bool {
    target_info.visible
        || progress.phase == ChapterPhase::Boss
        || player_health_danger(health)
        || matches!(obelisk.state, ObeliskState::Active)
        || matches!(rift.state, RiftState::Active)
}

fn compact_buff_labels(mut labels: Vec<String>) -> Vec<String> {
    const MAX_VISIBLE_BUFF_LABELS: usize = 4;
    if labels.len() <= MAX_VISIBLE_BUFF_LABELS {
        return labels;
    }
    let hidden = labels.len() - MAX_VISIBLE_BUFF_LABELS;
    labels.truncate(MAX_VISIBLE_BUFF_LABELS);
    labels.push(format!("+{hidden} buffs"));
    labels
}

fn player_health_danger(health: &Health) -> bool {
    health.max > 0.0 && health.current <= health.max * 0.35
}

fn burning_seconds_remaining(burning: &Burning) -> f32 {
    burning.timer.remaining_secs().max(0.0)
}

fn jailed_seconds_remaining(jailed: &Jailed) -> f32 {
    jailed.timer.remaining_secs().max(0.0)
}

fn push_timed_buff(labels: &mut Vec<String>, label: &str, seconds: f32) {
    if seconds > 0.0 {
        labels.push(format!("{} {}s", label, hud_seconds(seconds)));
    }
}

fn hud_seconds(seconds: f32) -> u32 {
    seconds.ceil().max(1.0) as u32
}

fn combat_log_panel_text<'a>(
    entries: impl IntoIterator<Item = &'a str>,
    combat_focus: bool,
) -> (String, Display) {
    if combat_focus {
        return (String::new(), Display::None);
    }
    let entries = entries
        .into_iter()
        .take(1)
        .map(|entry| compact_label(entry, 58))
        .collect::<Vec<_>>();
    if entries.is_empty() {
        (String::new(), Display::None)
    } else {
        (entries.join(""), Display::Flex)
    }
}

#[derive(Clone, Copy)]
struct ObjectiveFrameStyle {
    width: f32,
    top: f32,
    min_height: f32,
    padding_x: f32,
    padding_y: f32,
    font_size: f32,
    border_color: Color,
    background_color: Color,
}

fn objective_frame_style(combat_focus: bool, clean_hud: bool) -> ObjectiveFrameStyle {
    if combat_focus {
        ObjectiveFrameStyle {
            width: 430.0,
            top: 10.0,
            min_height: 20.0,
            padding_x: 7.0,
            padding_y: 2.0,
            font_size: 9.5,
            border_color: Color::srgba(0.95, 0.46, 0.24, 0.42),
            background_color: Color::srgba(0.018, 0.010, 0.012, 0.36),
        }
    } else if clean_hud {
        ObjectiveFrameStyle {
            width: 430.0,
            top: 10.0,
            min_height: 20.0,
            padding_x: 7.0,
            padding_y: 2.0,
            font_size: 9.5,
            border_color: Color::srgba(0.78, 0.62, 0.38, 0.28),
            background_color: Color::srgba(0.018, 0.014, 0.018, 0.24),
        }
    } else {
        ObjectiveFrameStyle {
            width: 700.0,
            top: 8.0,
            min_height: 24.0,
            padding_x: 8.0,
            padding_y: 3.0,
            font_size: 9.0,
            border_color: Color::srgba(0.78, 0.62, 0.38, 0.38),
            background_color: Color::srgba(0.018, 0.014, 0.018, 0.34),
        }
    }
}

fn update_objective_frame(frames: &mut ObjectiveFrameQuery, combat_focus: bool, clean_hud: bool) {
    let style = objective_frame_style(combat_focus, clean_hud);
    for (mut node, mut background, mut border, mut font) in frames {
        node.width = Val::Px(style.width);
        node.top = Val::Px(style.top);
        node.min_height = Val::Px(style.min_height);
        node.margin = UiRect::left(Val::Px(-style.width * 0.5));
        node.padding = UiRect::axes(Val::Px(style.padding_x), Val::Px(style.padding_y));
        *background = BackgroundColor(style.background_color);
        *border = BorderColor::all(style.border_color);
        font.font_size = FontSize::Px(style.font_size);
    }
}

fn journal_panel_text(
    journal_open: bool,
    paused: bool,
    open_text: &str,
    pause_summary: &str,
) -> (String, Display) {
    if journal_open {
        (open_text.to_string(), Display::Flex)
    } else if paused {
        (pause_summary.to_string(), Display::Flex)
    } else {
        (String::new(), Display::None)
    }
}

fn visibility_from_display(display: Display) -> Visibility {
    match display {
        Display::None => Visibility::Hidden,
        _ => Visibility::Visible,
    }
}

fn update_boss_hud(
    boss: BossHudStatusQuery,
    boon: ChapterBoonQuery,
    mut panels: BossHudPanelQuery,
    mut fills: BossHudFillQuery,
    mut stagger_fills: BossHudStaggerFillQuery,
    mut texts: BossHudTextQuery,
) {
    let boss_state = boss
        .iter()
        .find(|(health, _, _, _, _)| health.current > 0.0);
    let Some((health, phase, stagger, staggered, enemy)) = boss_state else {
        for mut panel in &mut panels {
            panel.display = Display::None;
        }
        return;
    };

    for mut panel in &mut panels {
        panel.display = Display::Flex;
    }

    let health_percent = percent(health.current, health.max);
    for (mut fill, mut color) in &mut fills {
        fill.width = Val::Percent(health_percent);
        color.0 = boss_hud_health_color(health_percent, phase);
    }

    let stagger_percent = boss_hud_stagger_percent(stagger, staggered);
    for (mut fill, mut color) in &mut stagger_fills {
        fill.width = Val::Percent(stagger_percent);
        color.0 = if staggered.is_some() {
            Color::srgba(1.0, 0.86, 0.16, 0.96)
        } else {
            Color::srgba(1.0, 0.60, 0.10, 0.82)
        };
    }

    let boon = boon.single().ok();
    for mut text in &mut texts {
        **text = boss_hud_summary(enemy, health, phase, stagger, staggered, boon);
    }
}

fn boss_hud_health_color(health_percent: f32, phase: &BossPhase) -> Color {
    if phase.enrage_started() {
        Color::srgba(1.0, 0.04, 0.02, 0.92)
    } else if phase.phase_two_started() {
        Color::srgba(1.0, 0.32, 0.06, 0.90)
    } else if health_percent <= 25.0 {
        Color::srgba(0.95, 0.08, 0.04, 0.90)
    } else {
        Color::srgba(0.84, 0.13, 0.04, 0.88)
    }
}

fn boss_hud_stagger_percent(stagger: Option<&BossStagger>, staggered: Option<&Staggered>) -> f32 {
    if staggered.is_some() {
        return 100.0;
    }
    let Some(stagger) = stagger else {
        return 0.0;
    };
    percent(stagger.meter, stagger.threshold)
}

fn boss_hud_summary(
    enemy: &Enemy,
    health: &Health,
    phase: &BossPhase,
    stagger: Option<&BossStagger>,
    staggered: Option<&Staggered>,
    boon: Option<&ChapterBoon>,
) -> String {
    let health_percent = percent(health.current, health.max).round();
    let phase_text = boss_hud_phase_text(phase, staggered);
    let tactic_text = boss_hud_tactic(health_percent, phase, stagger, staggered);
    let stagger_text = boss_stagger_summary(stagger, staggered);
    let mechanic_text = boss_hud_mechanic_text(health_percent, phase, stagger, staggered);
    let boon_text = boss_boon_summary(boon);
    format!(
        "{} {:.0}% | {} | {}\n{} | {} | {}",
        enemy.display_name,
        health_percent,
        phase_text,
        stagger_text,
        tactic_text,
        mechanic_text,
        boon_text
    )
}

fn boss_hud_phase_text(phase: &BossPhase, staggered: Option<&Staggered>) -> String {
    if let Some(staggered) = staggered {
        return format!("EXECUTE {:.0}s", staggered.timer.remaining_secs().ceil());
    }
    if phase.enrage_started() {
        return "ENRAGED".to_string();
    }
    if phase.phase_two_started() {
        let remaining = phase.enrage_remaining_secs().ceil();
        return if remaining <= 6.0 {
            format!("DANGER ENRAGE {remaining:.0}s")
        } else {
            format!("Phase II enrage {remaining:.0}s")
        };
    }
    "Phase I".to_string()
}

fn boss_hud_tactic(
    health_percent: f32,
    phase: &BossPhase,
    stagger: Option<&BossStagger>,
    staggered: Option<&Staggered>,
) -> String {
    if staggered.is_some() {
        return "Burst: Y/Q/E now".to_string();
    }
    if phase.enrage_started() {
        return if health_percent <= 18.0 {
            "Kill now, kite fire".to_string()
        } else {
            "Kite fire, punish gaps".to_string()
        };
    }
    if phase.phase_two_started() {
        let percent_to_break = boss_hud_stagger_percent(stagger, staggered);
        return if phase.enrage_remaining_secs() <= 3.0 {
            "DANGER: all burst or kite".to_string()
        } else if phase.enrage_remaining_secs() <= 6.0 {
            "DANGER: break now".to_string()
        } else if percent_to_break >= 72.0 {
            "Break soon: save burst".to_string()
        } else {
            "Build stagger before enrage".to_string()
        };
    }
    if health_percent <= 55.0 {
        "Push phase II with Surge".to_string()
    } else {
        "Build fury and hold potion".to_string()
    }
}

fn boss_hud_mechanic_text(
    health_percent: f32,
    phase: &BossPhase,
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
    if phase.enrage_started() {
        return "ENRAGE floor fire; punish gaps".to_string();
    }
    if phase.phase_two_started() {
        let stagger_percent = boss_hud_stagger_percent(stagger, staggered).round();
        return format!(
            "Stagger {stagger_percent:.0}% before enrage {:.0}s",
            phase.enrage_remaining_secs().ceil()
        );
    }
    if health_percent <= 55.0 {
        "Transition soon: boon, potion, Surge".to_string()
    } else {
        "Read swings; build Fury".to_string()
    }
}

fn boss_boon_summary(boon: Option<&ChapterBoon>) -> String {
    let Some(boon) = boon else {
        return boss_boon_choice_prompt_compact().to_string();
    };
    match boon.choice {
        ChapterBoonChoice::None => boss_boon_choice_prompt_compact().to_string(),
        ChapterBoonChoice::CrimsonOath => boss_boon_label("Crimson", "+dmg/crit", boon.empowered),
        ChapterBoonChoice::TitanWard => boss_boon_label("Titan", "+hp/armor", boon.empowered),
        ChapterBoonChoice::ArcaneMomentum => boss_boon_label_with_momentum(
            "Arcane",
            "+fury/economy",
            boon.empowered,
            boon.momentum_stacks,
        ),
    }
}

fn boss_boon_choice_prompt_compact() -> &'static str {
    "Boon 4 Crimson | 5 Titan | 6 Arcane"
}

fn boss_boon_label(name: &str, effect: &str, empowered: bool) -> String {
    if empowered {
        format!("Empowered Boon {name} {effect}")
    } else {
        format!("Boon {name} {effect}")
    }
}

fn boss_boon_label_with_momentum(
    name: &str,
    effect: &str,
    empowered: bool,
    momentum_stacks: u32,
) -> String {
    let label = boss_boon_label(name, effect, empowered);
    if momentum_stacks > 0 {
        format!(
            "{label} | Momentum {}x",
            momentum_stacks.min(RELIQUARY_MOMENTUM_MAX)
        )
    } else {
        label
    }
}

fn update_target_info_hud(
    target_info: Res<TargetInfo>,
    mut panels: TargetInfoPanelQuery,
    mut fills: TargetInfoFillQuery,
    mut texts: TargetInfoTextQuery,
) {
    for mut panel in &mut panels {
        panel.display = if target_info.visible {
            Display::Flex
        } else {
            Display::None
        };
    }
    if !target_info.visible {
        return;
    }

    for (mut fill, mut color) in &mut fills {
        fill.width = Val::Percent(target_info.health_percent.clamp(0.0, 100.0));
        color.0 = target_info_health_color(&target_info);
    }
    for (mut text, mut color) in &mut texts {
        **text = target_info_summary(&target_info);
        color.0 = target_info.threat_color;
    }
}

fn target_info_health_color(target_info: &TargetInfo) -> Color {
    if target_info.health_percent <= 25.0 {
        Color::srgba(0.96, 0.08, 0.04, 0.92)
    } else if target_info.subtitle.contains("BOSS") {
        Color::srgba(0.95, 0.28, 0.06, 0.92)
    } else if target_info.subtitle.contains("ELITE") || target_info.subtitle.contains("RARE") {
        Color::srgba(0.60, 0.20, 0.92, 0.90)
    } else {
        Color::srgba(0.78, 0.05, 0.04, 0.88)
    }
}

fn target_info_summary(target_info: &TargetInfo) -> String {
    format!(
        "{}\n{} | {}\n{}",
        target_info.name, target_info.subtitle, target_info.health_line, target_info.details
    )
}

fn update_loot_proximity_hud(
    info: Res<LootProximityInfo>,
    mut panels: LootProximityPanelQuery,
    mut texts: LootProximityTextQuery,
) {
    for (mut panel, mut border) in &mut panels {
        panel.display = if info.visible {
            Display::Flex
        } else {
            Display::None
        };
        if info.visible {
            *border = BorderColor::all(loot_proximity_color(&info.quality));
        }
    }
    if !info.visible {
        return;
    }

    let summary = loot_proximity_summary(&info);
    let color = if info.inventory_full {
        Color::srgb(0.92, 0.40, 0.32)
    } else {
        loot_proximity_color(&info.quality)
    };
    for (mut text, mut text_color) in &mut texts {
        **text = summary.clone();
        text_color.0 = color;
    }
}

fn loot_proximity_summary(info: &LootProximityInfo) -> String {
    let auto_equips = loot_compare_auto_equips(&info.compare);
    let state = if info.pickup_ready && auto_equips {
        "AUTO EQUIP"
    } else if !auto_equips && info.inventory_full {
        "INVENTORY FULL"
    } else if info.pickup_ready {
        "AUTO PICKUP"
    } else if auto_equips {
        "MOVE TO EQUIP"
    } else {
        "MOVE CLOSER"
    };
    let distance = (info.distance - LOOT_PICKUP_RADIUS).max(0.0);
    let rewards = if info.rewards.is_empty() {
        "no extra rewards".to_string()
    } else {
        info.rewards.clone()
    };
    let rewards = if info.inventory_full && info.spare_salvage_available {
        format!("{rewards} | U salvage spares")
    } else {
        rewards
    };
    let hint = loot_proximity_decision_hint(info, auto_equips);
    let priority = if info.priority.is_empty() {
        "DROP"
    } else {
        info.priority.as_str()
    };
    format!(
        "{state} {priority} {:.1}m | {hint}\n{}\n{}\n{}",
        distance, info.label, info.compare, rewards
    )
}

fn loot_compare_auto_equips(compare: &str) -> bool {
    compare.starts_with("AUTO-EQUIP")
}

fn loot_proximity_decision_hint(info: &LootProximityInfo, auto_equips: bool) -> &'static str {
    let compare = info.compare.as_str();
    if compare.contains("[CODEX]") || compare.contains("BUILD POWER") {
        "unlock codex power"
    } else if compare.contains("[CRAFT]") {
        "keep for craft"
    } else if compare.contains("[DPS]") || compare.contains("DPS GAIN") {
        "equip for DPS"
    } else if compare.contains("[TANK]") || compare.contains("TOUGHNESS") {
        "keep defensive swap"
    } else if compare.contains("[SALVAGE]") {
        "salvage spare"
    } else if compare.contains("[STASH]") {
        "stash or salvage"
    } else if compare.contains("POWER SPIKE") {
        "equip power spike"
    } else if compare.contains("SOCKET") {
        "socket upgrade"
    } else if compare.contains("TEMPER") {
        "temper upgrade"
    } else if compare.contains("BUILD SWAP") || compare.starts_with("SIDEGRADE") {
        "stash build swap"
    } else if compare.contains("SALVAGE LATER") || compare.starts_with("STASH") {
        "stash or salvage"
    } else if compare.contains("MAKE ROOM") || compare.starts_with("FULL") || info.inventory_full {
        "make room"
    } else if auto_equips {
        "equip upgrade"
    } else {
        "inspect drop"
    }
}

fn loot_proximity_color(quality: &str) -> Color {
    match quality {
        "primal" => Color::srgb(1.0, 0.26, 0.08),
        "ancient" => Color::srgb(1.0, 0.58, 0.16),
        "legendary" => Color::srgb(0.95, 0.68, 0.18),
        "rare" => Color::srgb(0.40, 0.64, 1.0),
        _ => Color::srgb(0.82, 0.78, 0.68),
    }
}

fn update_interact_proximity_hud(
    stats: Res<RunStats>,
    progress: Res<ChapterProgress>,
    player: InteractProximityPlayerQuery,
    interactables: InteractProximityWorldQuery,
    mut panels: InteractProximityPanelQuery,
    mut texts: InteractProximityTextQuery,
) {
    let Ok((player_transform, inventory, equipment, potions, elixirs, codex)) = player.single()
    else {
        hide_interact_proximity_hud(&mut panels);
        return;
    };
    let Some((interactable, distance)) =
        nearest_interactable(player_transform.translation, interactables.iter())
    else {
        hide_interact_proximity_hud(&mut panels);
        return;
    };
    let visible = interact_proximity_visible(interactable, distance);
    for (mut panel, mut border) in &mut panels {
        panel.display = if visible {
            Display::Flex
        } else {
            Display::None
        };
        if visible {
            *border = BorderColor::all(interact_proximity_color(interactable, &progress));
        }
    }
    if !visible {
        return;
    }
    let quartermaster_hint = if interactable.kind == InteractableKind::Merchant {
        Some(quartermaster_preview_summary(
            &stats, inventory, equipment, potions, elixirs, codex,
        ))
    } else {
        None
    };
    let summary = interact_proximity_summary(
        interactable,
        distance,
        &progress,
        quartermaster_hint.as_deref(),
    );
    let color = interact_proximity_color(interactable, &progress);
    for (mut text, mut text_color) in &mut texts {
        **text = summary.clone();
        text_color.0 = color;
    }
}

fn hide_interact_proximity_hud(panels: &mut InteractProximityPanelQuery) {
    for (mut panel, _) in panels {
        panel.display = Display::None;
    }
}

fn nearest_interactable<'a>(
    player_position: Vec3,
    interactables: impl Iterator<Item = (&'a Transform, &'a Interactable)>,
) -> Option<(&'a Interactable, f32)> {
    interactables
        .map(|(transform, interactable)| {
            (
                interactable,
                player_position.distance(transform.translation),
            )
        })
        .min_by(|(_, left), (_, right)| left.total_cmp(right))
}

fn interact_proximity_visible(interactable: &Interactable, distance: f32) -> bool {
    distance <= interactable.radius + 2.4
}

fn interact_proximity_summary(
    interactable: &Interactable,
    distance: f32,
    progress: &ChapterProgress,
    quartermaster_hint: Option<&str>,
) -> String {
    let state = interact_proximity_state(interactable, distance);
    let role = if is_primary_objective_interactable(progress, interactable.kind) {
        "MAIN"
    } else if is_guided_optional_interactable(interactable.kind) {
        "SIDE"
    } else {
        "OPTIONAL"
    };
    let reward = interact_reward_line(interactable.kind, quartermaster_hint);
    format!(
        "{state} | {role} {}\n{}\n{}",
        interact_label(interactable.kind),
        reward,
        interact_phase_hint(interactable.kind, progress)
    )
}

fn interact_proximity_state(interactable: &Interactable, distance: f32) -> String {
    if interactable.used && !interactable.reusable {
        return "USED".to_string();
    }
    if distance <= interactable.radius {
        "SPACE READY".to_string()
    } else {
        format!("MOVE {:.1}m", (distance - interactable.radius).max(0.0))
    }
}

fn interact_label(kind: InteractableKind) -> &'static str {
    match kind {
        InteractableKind::Cache => "Reliquary Cache",
        InteractableKind::Altar => "Ember Altar",
        InteractableKind::WeaponShrine => "Weapon Shrine",
        InteractableKind::FortuneShrine => "Fortune Shrine",
        InteractableKind::StormShrine => "Storm Shrine",
        InteractableKind::HealingWell => "Healing Well",
        InteractableKind::LorePage => "Lore Page",
        InteractableKind::CursedShrine => "Cursed Shrine",
        InteractableKind::BloodObelisk => "Blood Obelisk",
        InteractableKind::ReliquaryVault => "Reliquary Vault",
        InteractableKind::EmberRift => "Ember Rift",
        InteractableKind::AshenPylon => "Ashen Pylon",
        InteractableKind::Merchant => "Quartermaster",
    }
}

fn interact_reward_summary(kind: InteractableKind) -> &'static str {
    match kind {
        InteractableKind::Cache => "REWARD seal + gear cache",
        InteractableKind::Altar => "BOSS TRIGGER final seal",
        InteractableKind::WeaponShrine => "BUFF weapon damage",
        InteractableKind::FortuneShrine => "BUFF gold + magic find",
        InteractableKind::StormShrine => "BUFF lightning pulses",
        InteractableKind::HealingWell => "RECOVER life + potions",
        InteractableKind::LorePage => "LORE journal + challenge",
        InteractableKind::CursedShrine => "RISK blood price, elite ambush",
        InteractableKind::BloodObelisk => "RISK timed kill rite",
        InteractableKind::ReliquaryVault => "REWARD treasure vault",
        InteractableKind::EmberRift => "RISK timed rift cache",
        InteractableKind::AshenPylon => "RISK surge power + ambush",
        InteractableKind::Merchant => "SERVICE craft, temper, salvage",
    }
}

fn interact_reward_line(kind: InteractableKind, quartermaster_hint: Option<&str>) -> String {
    if kind != InteractableKind::Merchant {
        return interact_reward_summary(kind).to_string();
    }
    let Some(hint) = quartermaster_hint else {
        return interact_reward_summary(kind).to_string();
    };
    let detail = hint
        .strip_prefix("Quartermaster next: ")
        .or_else(|| hint.strip_prefix("Quartermaster ready: "))
        .or_else(|| hint.strip_prefix("Primal target: "))
        .unwrap_or(hint);
    format!("SERVICE {}", compact_label(detail, 72))
}

fn interact_phase_hint(kind: InteractableKind, progress: &ChapterProgress) -> String {
    match (progress.phase, kind) {
        (ChapterPhase::Cache, InteractableKind::Cache) => "main path: open now".to_string(),
        (ChapterPhase::Ritual, InteractableKind::Altar) => {
            "main path: claim final seal".to_string()
        }
        (ChapterPhase::Boss, InteractableKind::Altar) => "seal already claimed".to_string(),
        (_, InteractableKind::Merchant) => "upgrade before boss or optional fights".to_string(),
        (_, InteractableKind::HealingWell) => "save for low health or before boss".to_string(),
        _ => format!("chapter: {}", phase_label(progress.phase)),
    }
}

fn interact_proximity_color(interactable: &Interactable, progress: &ChapterProgress) -> Color {
    if interactable.used && !interactable.reusable {
        Color::srgb(0.50, 0.48, 0.44)
    } else if is_objective_interactable(progress, interactable.kind) {
        Color::srgb(1.0, 0.74, 0.26)
    } else {
        Color::srgb(0.64, 0.82, 1.0)
    }
}

fn update_inventory_panels(inventory_panels: &mut InventoryPanelQuery, open: bool) {
    for mut node in inventory_panels {
        node.display = if open { Display::Flex } else { Display::None };
    }
}

fn update_build_panels(build_panels: &mut BuildPanelQuery, open: bool) {
    for mut node in build_panels {
        node.display = if open { Display::Flex } else { Display::None };
    }
}

fn update_experience_fills(experience_fills: &mut ExperienceFillQuery, level: &PlayerLevel) {
    let amount = percent(level.xp as f32, level.next_level_xp as f32);
    for (mut node, mut color) in experience_fills {
        node.width = Val::Percent(amount);
        color.0 = if amount >= 75.0 {
            Color::srgba(1.0, 0.62, 0.16, 0.96)
        } else {
            Color::srgba(0.86, 0.46, 0.10, 0.92)
        };
    }
}

struct BuildPanelInputs<'a> {
    health: &'a Health,
    fury: &'a Fury,
    cooldowns: &'a SkillCooldowns,
    evade: &'a Evade,
    surge: &'a SurgeBuff,
    level: &'a PlayerLevel,
    damage_bonus: &'a DamageBonus,
    equipment: &'a Equipment,
    charm: &'a Charm,
    talents: &'a Talents,
    runes: &'a SkillRunes,
    codex: &'a LegendaryCodex,
}

fn build_panel_text(inputs: BuildPanelInputs) -> String {
    let damage = total_damage_bonus(inputs.damage_bonus, inputs.equipment, inputs.charm);
    let crit = total_crit_chance(inputs.equipment, inputs.charm) * 100.0;
    let archetype = build_archetype_summary(
        inputs.damage_bonus,
        inputs.equipment,
        inputs.charm,
        inputs.talents,
        inputs.runes,
    );
    let score = build_score_summary(
        inputs.damage_bonus,
        inputs.equipment,
        inputs.charm,
        inputs.talents,
        inputs.runes,
    );
    format!(
        "BUILD | K close | I bag | J journal | 1/2/3 talents | Z/X/C runes\n\
         Archetype {} | {}\n\
         Loadout {} | Skills LMB/RMB/Q/E/Y/Shift | Bag [/] U\n\
         Next {}\n\
         Core L{} XP {}/{} | HP {:.0}/{:.0} | Fury {:.0}/{:.0}\n\
         Damage +{:.0} | Crit {:.0}% | Armor {:.0} | Worn +{:.0} tough +{:.0} hp | x{:.2} dmg x{:.2} cd\n\
         Talents {} pts | Wrath {} Vigor {} Focus {} | {}\n\
         Mastery {}\n\
         Runes {} | {} | {}\n\
         Rotation {}\n\
         Cooldowns Strike {:.1}s | Rupture {:.1}s | Dash {:.1}s | Nova {:.1}s\n\
         Surge Y {}/{} {} | Evade Shift {}\n\
         Power {} | Codex {}/{} | {}\n\
        Weapon {} {}{}",
        archetype,
        score,
        build_loadout_label(inputs.equipment, inputs.charm, inputs.runes),
        build_next_step_summary(
            inputs.talents,
            inputs.equipment,
            inputs.charm,
            inputs.runes,
            inputs.codex,
        ),
        inputs.level.level,
        inputs.level.xp,
        inputs.level.next_level_xp,
        inputs.health.current.max(0.0),
        inputs.health.max,
        inputs.fury.current,
        inputs.fury.max,
        damage,
        crit,
        inputs.equipment.armor_bonus + inputs.equipment.worn_armor_bonus(),
        inputs.equipment.worn_armor_bonus(),
        inputs.equipment.worn_health_bonus(),
        inputs.talents.damage_multiplier(),
        inputs.talents.cooldown_multiplier(),
        inputs.talents.points,
        inputs.talents.wrath,
        inputs.talents.vigor,
        inputs.talents.focus,
        next_talent_pick_summary(inputs.talents),
        inputs.talents.mastery_summary(),
        rune_loadout_summary(inputs.runes),
        rune_synergy_summary(inputs.runes),
        set_combo_payoff_summary(inputs.equipment, inputs.charm),
        build_rotation_summary(inputs.fury, inputs.cooldowns, inputs.surge, inputs.runes),
        inputs.cooldowns.basic.remaining_secs(),
        inputs.cooldowns.rupture.remaining_secs(),
        inputs.cooldowns.dash.remaining_secs(),
        inputs.cooldowns.nova.remaining_secs(),
        inputs.surge.charge,
        inputs.surge.max_charge,
        surge_build_state(inputs.surge),
        evade_build_state(inputs.evade),
        inputs.equipment.legendary_power.label(),
        legendary_codex_power_count(inputs.codex),
        LegendaryPower::ALL.len(),
        legendary_codex_pursuit_summary(inputs.codex, inputs.equipment),
        inputs.equipment.quality,
        compact_label(&inputs.equipment.weapon_name, 24),
        inputs
            .equipment
            .socketed_gem
            .map(|gem| format!(" | {}", gem.label()))
            .unwrap_or_default()
    )
}

fn build_next_step_summary(
    talents: &Talents,
    equipment: &Equipment,
    charm: &Charm,
    runes: &SkillRunes,
    codex: &LegendaryCodex,
) -> String {
    if talents.points > 0 {
        return format!(
            "K then {} ({})",
            next_talent_key(talents),
            next_talent_pick_summary(talents)
        );
    }
    if equipment.legendary_power == LegendaryPower::None && !codex.unlocked.is_empty() {
        return compact_label(&legendary_codex_pursuit_summary(codex, equipment), 92);
    }
    if themed_reliquary_set(equipment, charm).is_none() {
        return "find matching weapon/charm theme for combo set".to_string();
    }
    if !codex.is_complete() {
        return compact_label(&legendary_codex_pursuit_summary(codex, equipment), 92);
    }
    format!("Z/X/C tune runes: {}", rune_synergy_summary(runes))
}

fn build_archetype_summary(
    damage_bonus: &DamageBonus,
    equipment: &Equipment,
    charm: &Charm,
    talents: &Talents,
    runes: &SkillRunes,
) -> String {
    let primary = build_archetype_name(equipment, runes);
    let posture = build_posture_label(damage_bonus, equipment, charm, talents);
    let payoff = build_payoff_label(equipment, charm, runes);
    format!("{primary} {posture} | {payoff}")
}

fn build_loadout_label(equipment: &Equipment, charm: &Charm, runes: &SkillRunes) -> String {
    format!(
        "{} / {} / {}",
        compact_label(&equipment.weapon_name, 10),
        compact_label(&charm.name, 10),
        compact_label(&rune_loadout_summary(runes), 24)
    )
}

fn build_archetype_name(equipment: &Equipment, runes: &SkillRunes) -> &'static str {
    match equipment.legendary_power {
        LegendaryPower::Stormbrand => "Storm Reaper",
        LegendaryPower::Soulreaver => "Blood Reaver",
        LegendaryPower::Aegisbrand => "Aegis Ward",
        LegendaryPower::Frostbrand => "Frost Control",
        LegendaryPower::Emberbrand => "Ember Cleave",
        LegendaryPower::None => match (runes.dash, runes.nova, runes.rupture) {
            (DashRune::Reap, NovaRune::Frost, _) => "Reap Control",
            (DashRune::Reap, NovaRune::Ember, _) => "Reap Burn",
            (_, NovaRune::Frost, RuptureRune::Expose) => "Control Setup",
            (_, NovaRune::Ember, RuptureRune::Hemorrhage) => "Bleed Burn",
            _ => "Reliquary Striker",
        },
    }
}

fn build_posture_label(
    damage_bonus: &DamageBonus,
    equipment: &Equipment,
    charm: &Charm,
    talents: &Talents,
) -> &'static str {
    let damage = total_damage_bonus(damage_bonus, equipment, charm);
    let crit = total_crit_chance(equipment, charm);
    let toughness = equipment.health_bonus + equipment.armor_bonus + talents.vigor as f32 * 18.0;
    if toughness >= 70.0 {
        "tank"
    } else if crit >= 0.22 {
        "crit"
    } else if damage >= 26.0 || talents.wrath >= 3 {
        "burst"
    } else if talents.focus >= 3 {
        "tempo"
    } else {
        "balanced"
    }
}

fn build_payoff_label(equipment: &Equipment, charm: &Charm, runes: &SkillRunes) -> &'static str {
    if let Some(label) = legendary_rune_synergy_payoff_label(equipment.legendary_power, runes) {
        return label;
    }
    if let Some(set) = themed_reliquary_set(equipment, charm) {
        return match set {
            ReliquarySet::Storm => "chain packs",
            ReliquarySet::Blood => "steal life",
            ReliquarySet::Ashen => "barrier loop",
            ReliquarySet::Reliquary => "fury engine",
        };
    }
    match (equipment.legendary_power, runes.nova, runes.rupture) {
        (LegendaryPower::Stormbrand, _, _) => "chain packs",
        (LegendaryPower::Soulreaver, _, _) => "sustain hits",
        (LegendaryPower::Aegisbrand, _, _) => "shield trades",
        (_, NovaRune::Frost, RuptureRune::Expose) => "freeze vuln",
        (_, NovaRune::Frost, RuptureRune::Hemorrhage) => "freeze bleed",
        (_, NovaRune::Ember, RuptureRune::Expose) => "burn vuln",
        (_, NovaRune::Ember, RuptureRune::Hemorrhage) => "burn bleed",
    }
}

fn legendary_rune_synergy_payoff_label(
    power: LegendaryPower,
    runes: &SkillRunes,
) -> Option<&'static str> {
    match power {
        LegendaryPower::Emberbrand
            if runes.nova == NovaRune::Ember || runes.rupture == RuptureRune::Hemorrhage =>
        {
            Some("rune ignites")
        }
        LegendaryPower::Frostbrand
            if runes.nova == NovaRune::Frost || runes.rupture == RuptureRune::Expose =>
        {
            Some("rune chills")
        }
        LegendaryPower::Stormbrand
            if runes.dash == DashRune::Reap || runes.nova == NovaRune::Frost =>
        {
            Some("rune chains")
        }
        LegendaryPower::Soulreaver
            if runes.dash == DashRune::Reap || runes.rupture == RuptureRune::Hemorrhage =>
        {
            Some("rune leech")
        }
        LegendaryPower::Aegisbrand
            if runes.dash == DashRune::Cleanse || runes.rupture == RuptureRune::Expose =>
        {
            Some("rune wards")
        }
        _ => None,
    }
}

fn reliquary_set_tactic(equipment: &Equipment, charm: &Charm, stats: &RunStats) -> String {
    let Some(set) = themed_reliquary_set(equipment, charm) else {
        return "Set: match weapon/charm theme".to_string();
    };
    let kills = stats.set_resonance_kills.min(SET_ADEPT_KILLS);
    let progress = if kills >= SET_ADEPT_KILLS {
        "mastered".to_string()
    } else {
        format!("{kills}/{SET_ADEPT_KILLS}")
    };
    format!(
        "Set {} {} {}",
        reliquary_set_short_label(set),
        reliquary_set_playstyle(set),
        progress
    )
}

fn reliquary_set_short_label(set: ReliquarySet) -> &'static str {
    match set {
        ReliquarySet::Storm => "Storm",
        ReliquarySet::Blood => "Blood",
        ReliquarySet::Ashen => "Ashen",
        ReliquarySet::Reliquary => "Reliquary",
    }
}

fn reliquary_set_playstyle(set: ReliquarySet) -> &'static str {
    match set {
        ReliquarySet::Storm => "chain",
        ReliquarySet::Blood => "sustain",
        ReliquarySet::Ashen => "burn",
        ReliquarySet::Reliquary => "balanced clear",
    }
}

fn build_score_summary(
    damage_bonus: &DamageBonus,
    equipment: &Equipment,
    charm: &Charm,
    talents: &Talents,
    runes: &SkillRunes,
) -> String {
    format!(
        "DPS {} | Tough {} | Ctrl {}",
        build_dps_score(damage_bonus, equipment, charm, talents),
        build_toughness_score(equipment, charm, talents),
        build_control_score(equipment, runes, talents)
    )
}

fn build_dps_score(
    damage_bonus: &DamageBonus,
    equipment: &Equipment,
    charm: &Charm,
    talents: &Talents,
) -> u32 {
    let damage = total_damage_bonus(damage_bonus, equipment, charm);
    let crit = total_crit_chance(equipment, charm) * 100.0;
    let power = if equipment.legendary_power == LegendaryPower::None {
        0.0
    } else {
        16.0
    };
    (damage * 2.0 + crit * 1.4 + talents.wrath as f32 * 8.0 + power)
        .round()
        .clamp(0.0, 999.0) as u32
}

fn build_toughness_score(equipment: &Equipment, charm: &Charm, talents: &Talents) -> u32 {
    let resonance = reliquary_resonance(equipment, charm)
        .map(|resonance| resonance.damage_bonus + resonance.crit_bonus * 120.0)
        .unwrap_or_default();
    (equipment.health_bonus * 0.8 + equipment.armor_bonus + talents.vigor as f32 * 18.0 + resonance)
        .round()
        .clamp(0.0, 999.0) as u32
}

fn build_control_score(equipment: &Equipment, runes: &SkillRunes, talents: &Talents) -> u32 {
    let mut score = talents.focus * 10;
    score += match runes.nova {
        NovaRune::Frost => 30,
        NovaRune::Ember => 12,
    };
    score += match runes.rupture {
        RuptureRune::Expose => 18,
        RuptureRune::Hemorrhage => 12,
    };
    score += match runes.dash {
        DashRune::Cleanse => 16,
        DashRune::Reap => 10,
    };
    if matches!(
        equipment.legendary_power,
        LegendaryPower::Frostbrand | LegendaryPower::Aegisbrand
    ) {
        score += 14;
    }
    score
}

fn next_talent_pick_summary(talents: &Talents) -> String {
    if talents.points == 0 {
        return "next point: level up".to_string();
    }
    if talents.wrath <= talents.vigor && talents.wrath <= talents.focus {
        "spend 1 Wrath for damage".to_string()
    } else if talents.focus <= talents.vigor {
        "spend 3 Focus for cooldowns".to_string()
    } else {
        "spend 2 Vigor for survival".to_string()
    }
}

fn rune_loadout_summary(runes: &SkillRunes) -> String {
    format!(
        "Dash {} / Nova {} / Rupture {}",
        runes.dash.label(),
        runes.nova.label(),
        runes.rupture.label()
    )
}

fn rune_synergy_summary(runes: &SkillRunes) -> &'static str {
    match (runes.dash, runes.nova, runes.rupture) {
        (DashRune::Reap, NovaRune::Frost, RuptureRune::Hemorrhage) => {
            "freeze packs, bleed them, reap executes"
        }
        (DashRune::Reap, NovaRune::Ember, RuptureRune::Expose) => {
            "mark vulnerable, burn pack, reap low targets"
        }
        (DashRune::Cleanse, NovaRune::Frost, RuptureRune::Expose) => {
            "control setup with safe cleanse dash"
        }
        (DashRune::Cleanse, NovaRune::Ember, RuptureRune::Hemorrhage) => {
            "bleed and burn while keeping cleanse ready"
        }
        (DashRune::Cleanse, NovaRune::Ember, RuptureRune::Expose) => {
            "balanced starter burst and safe escape"
        }
        (DashRune::Cleanse, NovaRune::Frost, RuptureRune::Hemorrhage) => {
            "defensive control with bleed cleanup"
        }
        (DashRune::Reap, NovaRune::Frost, RuptureRune::Expose) => {
            "freeze then expose for controlled burst"
        }
        (DashRune::Reap, NovaRune::Ember, RuptureRune::Hemorrhage) => {
            "damage over time build with reap finish"
        }
    }
}

fn set_combo_payoff_summary(equipment: &Equipment, charm: &Charm) -> &'static str {
    match themed_reliquary_set(equipment, charm) {
        Some(ReliquarySet::Storm) => "Combo set: shock",
        Some(ReliquarySet::Blood) => "Combo set: heal",
        Some(ReliquarySet::Ashen) => "Combo set: barrier",
        Some(ReliquarySet::Reliquary) => "Combo set: fury",
        None => "Combo set: match theme",
    }
}

fn build_rotation_summary(
    fury: &Fury,
    cooldowns: &SkillCooldowns,
    surge: &SurgeBuff,
    runes: &SkillRunes,
) -> String {
    if surge_seconds_remaining(surge) > 0.0 {
        return "Surge active: spend Fury and chain kills".to_string();
    }
    if surge.charge >= surge.max_charge {
        return "Y Surge before elite/Boss burst".to_string();
    }
    if fury.current < fury.rupture_cost.min(fury.dash_cost).min(fury.nova_cost) {
        return "LMB Strike to build Fury".to_string();
    }
    if cooldowns.rupture.is_finished() && fury.current >= fury.rupture_cost {
        return match runes.rupture {
            RuptureRune::Expose => "RMB Expose starts combo; Q/E finish".to_string(),
            RuptureRune::Hemorrhage => "RMB Bleed starts combo; Q/E finish".to_string(),
        };
    }
    if cooldowns.nova.is_finished() && fury.current >= fury.nova_cost {
        return match runes.nova {
            NovaRune::Ember => "E Ember Nova finishes packs".to_string(),
            NovaRune::Frost => "E Frost Nova controls and finishes".to_string(),
        };
    }
    if cooldowns.dash.is_finished() && fury.current >= fury.dash_cost {
        return match runes.dash {
            DashRune::Cleanse => "Q Cleanse Dash finishes or escapes".to_string(),
            DashRune::Reap => "Q Reap Dash finishes weakened enemies".to_string(),
        };
    }
    "Hold position, Strike, wait for cooldowns".to_string()
}

fn surge_build_state(surge: &SurgeBuff) -> String {
    let active = surge_seconds_remaining(surge);
    if active > 0.0 {
        format!("ACTIVE {:.0}s", active.ceil())
    } else if surge.charge >= surge.max_charge {
        "READY".to_string()
    } else {
        "charging".to_string()
    }
}

fn evade_build_state(evade: &Evade) -> String {
    let active = evade_seconds_remaining(evade);
    if active > 0.0 {
        return format!("I-FRAME {:.1}s", active);
    }
    let cooldown = evade_cooldown_remaining(evade);
    if cooldown > 0.05 {
        format!("CD {:.1}s", cooldown)
    } else {
        "READY".to_string()
    }
}

fn update_compact_character_hud(
    player: CompactCharacterPlayerQuery,
    stats: Res<RunStats>,
    mut texts: CompactCharacterTextQuery,
) {
    let Ok((damage_bonus, equipment, charm, inventory, codex, talents, runes)) = player.single()
    else {
        return;
    };
    let resonance = reliquary_resonance(equipment, charm)
        .map(|resonance| resonance.label.to_string())
        .unwrap_or_else(|| "No resonance".to_string());
    let set_tactic = reliquary_set_tactic(equipment, charm, &stats);
    for (slot, mut text) in &mut texts {
        **text = compact_character_summary(
            *slot,
            damage_bonus,
            equipment,
            charm,
            inventory,
            codex,
            talents,
            runes,
            &resonance,
            &set_tactic,
            &stats,
        );
    }
}

#[allow(clippy::too_many_arguments)]
fn compact_character_summary(
    slot: CompactCharacterText,
    damage_bonus: &DamageBonus,
    equipment: &Equipment,
    charm: &Charm,
    inventory: &Inventory,
    codex: &LegendaryCodex,
    talents: &Talents,
    runes: &SkillRunes,
    resonance: &str,
    set_tactic: &str,
    stats: &RunStats,
) -> String {
    match slot {
        CompactCharacterText::Weapon => format!(
            "GEAR WEAPON\n+{:.0} dmg {:.0}%\n{}",
            damage_bonus.0,
            equipment.crit_chance * 100.0,
            compact_label(
                &format!(
                    "{} {}",
                    compact_label(&equipment.weapon_name, 8),
                    weapon_build_tag(equipment, damage_bonus, equipment.socketed_gem)
                ),
                18
            )
        ),
        CompactCharacterText::Charm => format!(
            "GEAR CHARM\n+{:.0} dmg {:.0}%\n{}",
            charm.damage_bonus,
            charm.crit_chance * 100.0,
            compact_label(
                &format!(
                    "{} {}",
                    compact_label(&charm.name, 8),
                    charm_build_tag(charm)
                ),
                18
            )
        ),
        CompactCharacterText::Inventory => {
            let compare = next_weapon_compare_summary(inventory, equipment, damage_bonus);
            let action = bag_action_summary(stats, inventory, equipment, damage_bonus);
            format!(
                "I BAG {}/{}\n{}\n{}",
                inventory.items.len(),
                inventory.capacity,
                compact_label(&compare, 18),
                compact_label(&action, 20)
            )
        }
        CompactCharacterText::Codex => format!(
            "K BUILD {}\n{}\n{}",
            compact_label(
                &build_archetype_name(equipment, runes).to_ascii_uppercase(),
                10
            ),
            compact_label(
                &format!(
                    "{} | {}",
                    build_score_summary(damage_bonus, equipment, charm, talents, runes),
                    next_talent_pick_summary(talents)
                ),
                20
            ),
            compact_label(
                &format!(
                    "{} {}/{} {}",
                    if themed_reliquary_set(equipment, charm).is_some() {
                        set_tactic
                    } else {
                        build_payoff_label(equipment, charm, runes)
                    },
                    legendary_codex_power_count(codex),
                    LegendaryPower::ALL.len(),
                    if themed_reliquary_set(equipment, charm).is_some() {
                        ""
                    } else {
                        resonance
                    }
                ),
                24
            )
        ),
    }
}

fn bag_action_summary(
    stats: &RunStats,
    inventory: &Inventory,
    equipment: &Equipment,
    damage_bonus: &DamageBonus,
) -> String {
    if inventory.items.len() >= inventory.capacity {
        let salvageable = salvageable_inventory_count(inventory, equipment, damage_bonus);
        if let Some(item) = best_inventory_upgrade(inventory, equipment, damage_bonus) {
            return format!(
                "FULL U salvage {} / {}",
                salvageable.max(1),
                bag_item_action_summary(item, equipment, damage_bonus)
            );
        }
        return format!("FULL U salvage {}", salvageable.max(1));
    }
    if let Some(item) = best_inventory_upgrade(inventory, equipment, damage_bonus) {
        return bag_item_action_summary(item, equipment, damage_bonus);
    }
    if let Some(item) = best_inventory_build_piece(inventory, equipment, damage_bonus) {
        return bag_item_action_summary(item, equipment, damage_bonus);
    }
    let salvageable = salvageable_inventory_count(inventory, equipment, damage_bonus);
    if salvageable > 0 {
        return format!("U salvage {salvageable}");
    }
    loot_pursuit_summary(stats, inventory, equipment)
}

fn inventory_panel_header(
    stats: &RunStats,
    inventory: &Inventory,
    equipment: &Equipment,
    damage_bonus: &DamageBonus,
    potions: &PotionBelt,
    elixirs: &ElixirBelt,
    codex: &LegendaryCodex,
) -> String {
    let salvageable = salvageable_inventory_count(inventory, equipment, damage_bonus);
    let full_marker = if inventory.items.len() >= inventory.capacity {
        " FULL"
    } else {
        ""
    };
    format!(
        "INVENTORY | I close | [/] equip weapon | ; don best gear | U salvage {} | B codex | O/P armory\n\
         Bag {}/{}{} | Action {} | {}\n\
         Gold {} Shards {} Essence {} Echo {} | {} | Elixir V {} G {}/{}",
        salvageable,
        inventory.items.len(),
        inventory.capacity,
        full_marker,
        compact_label(
            &bag_action_summary(stats, inventory, equipment, damage_bonus),
            34
        ),
        compact_label(
            &next_weapon_compare_summary(inventory, equipment, damage_bonus),
            48
        ),
        stats.gold,
        stats.ember_shards,
        stats.affix_essence,
        stats.echo_keystones,
        compact_label(
            &quartermaster_preview_summary(stats, inventory, equipment, potions, elixirs, codex),
            52
        ),
        elixirs.selected.label(),
        elixirs.current,
        elixirs.max
    )
}

fn bag_item_action_summary(
    item: &InventoryItem,
    equipment: &Equipment,
    damage_bonus: &DamageBonus,
) -> String {
    format!(
        "{} {}",
        inventory_item_action_hint(item, equipment, damage_bonus),
        compact_label(
            &inventory_item_action_gain(item, equipment, damage_bonus),
            11
        )
    )
}

fn inventory_item_action_gain(
    item: &InventoryItem,
    equipment: &Equipment,
    damage_bonus: &DamageBonus,
) -> String {
    if item.legendary_power != LegendaryPower::None
        && item.legendary_power != equipment.legendary_power
    {
        return item.legendary_power.label().to_string();
    }
    if let Some(callout) = inventory_item_callouts(item, equipment, damage_bonus)
        .into_iter()
        .find(|callout| {
            callout.starts_with("socket")
                || callout.starts_with("temper")
                || callout.starts_with("weapon")
                || callout.starts_with("crit")
                || callout.starts_with("toughness")
        })
    {
        return callout;
    }
    let delta = inventory_item_power_delta(item, equipment, damage_bonus);
    format!("PWR {delta:+.0}")
}

fn best_inventory_upgrade<'a>(
    inventory: &'a Inventory,
    equipment: &Equipment,
    damage_bonus: &DamageBonus,
) -> Option<&'a InventoryItem> {
    inventory
        .items
        .iter()
        .filter(|item| item.name != equipment.weapon_name)
        .filter(|item| inventory_item_power_delta(item, equipment, damage_bonus) >= 5.0)
        .max_by(|left, right| {
            inventory_item_power_delta(left, equipment, damage_bonus)
                .total_cmp(&inventory_item_power_delta(right, equipment, damage_bonus))
        })
}

fn best_inventory_build_piece<'a>(
    inventory: &'a Inventory,
    equipment: &Equipment,
    damage_bonus: &DamageBonus,
) -> Option<&'a InventoryItem> {
    inventory
        .items
        .iter()
        .filter(|item| item.name != equipment.weapon_name)
        .filter(|item| {
            item.legendary_power != LegendaryPower::None
                && item.legendary_power != equipment.legendary_power
                || item.socketed_gem.is_some()
                || item.temper_level > equipment.temper_level
                || inventory_item_power_delta(item, equipment, damage_bonus) >= -5.0
        })
        .max_by(|left, right| {
            inventory_item_power_delta(left, equipment, damage_bonus)
                .total_cmp(&inventory_item_power_delta(right, equipment, damage_bonus))
        })
}

fn salvageable_inventory_count(
    inventory: &Inventory,
    equipment: &Equipment,
    damage_bonus: &DamageBonus,
) -> usize {
    inventory
        .items
        .iter()
        .filter(|item| item.name != equipment.weapon_name)
        .filter(|item| inventory_item_decision_reason(item, equipment, damage_bonus) == "SALVAGE")
        .count()
}

fn compact_character_border(slot: CompactCharacterText) -> Color {
    match slot {
        CompactCharacterText::Weapon => Color::srgba(0.92, 0.62, 0.24, 0.76),
        CompactCharacterText::Charm => Color::srgba(0.46, 0.72, 0.94, 0.68),
        CompactCharacterText::Inventory => Color::srgba(0.68, 0.62, 0.48, 0.64),
        CompactCharacterText::Codex => Color::srgba(0.62, 0.44, 0.90, 0.70),
    }
}

fn update_compact_utility_hud(
    progress: Res<ChapterProgress>,
    player: CompactUtilityPlayerQuery,
    mut texts: CompactUtilityTextQuery,
) {
    let Ok((potions, elixirs, elixir_buff, portal)) = player.single() else {
        return;
    };
    for (slot, mut text) in &mut texts {
        **text = compact_utility_summary(*slot, potions, elixirs, elixir_buff, portal, &progress);
    }
}

fn compact_utility_summary(
    slot: CompactUtilityText,
    potions: &PotionBelt,
    elixirs: &ElixirBelt,
    elixir_buff: &ElixirBuff,
    portal: &TownPortal,
    progress: &ChapterProgress,
) -> String {
    match slot {
        CompactUtilityText::Potion => format!("F POT\n{}", potion_utility_state(potions)),
        CompactUtilityText::Elixir => format!(
            "G ELIX\n{} {}/{}",
            elixir_utility_state(elixirs, elixir_buff),
            elixirs.current,
            elixirs.max
        ),
        CompactUtilityText::Portal => format!("T PORT\n{}", portal_utility_state(portal)),
        CompactUtilityText::Interact => {
            format!(
                "SPACE\n{}",
                compact_label(&main_interact_hint(progress), 10)
            )
        }
    }
}

fn potion_utility_state(potions: &PotionBelt) -> String {
    let cooldown = potion_seconds_remaining(potions);
    if cooldown > 0.05 {
        format!("CD {:.0}s", cooldown.ceil())
    } else if potions.current == 0 {
        "EMPTY".to_string()
    } else {
        format!("READY {}/{}", potions.current, potions.max)
    }
}

fn elixir_utility_state(elixirs: &ElixirBelt, elixir_buff: &ElixirBuff) -> String {
    let active = elixir_seconds_remaining(elixir_buff);
    if active > 0.05 {
        format!("{} {:.0}s", elixir_buff.kind.label(), active.ceil())
    } else if elixirs.current == 0 {
        format!("{} EMPTY", elixirs.selected.label())
    } else {
        elixirs.selected.label().to_string()
    }
}

fn portal_utility_state(portal: &TownPortal) -> String {
    let cooldown = town_portal_seconds_remaining(portal);
    if cooldown > 0.05 {
        format!("CD {:.0}s", cooldown.ceil())
    } else if portal.return_position.is_some() {
        "RETURN".to_string()
    } else {
        "READY".to_string()
    }
}

fn main_interact_hint(progress: &ChapterProgress) -> String {
    match progress.phase {
        ChapterPhase::Cache => "Cache".to_string(),
        ChapterPhase::Ritual => "Altar".to_string(),
        ChapterPhase::Boss => "Boon/Boss".to_string(),
        ChapterPhase::Victory => "Next Run".to_string(),
        _ => compact_label(&progress.objective, 10),
    }
}

fn compact_utility_border(slot: CompactUtilityText) -> Color {
    match slot {
        CompactUtilityText::Potion => Color::srgba(0.90, 0.16, 0.12, 0.72),
        CompactUtilityText::Elixir => Color::srgba(0.50, 0.82, 0.50, 0.68),
        CompactUtilityText::Portal => Color::srgba(0.42, 0.58, 0.95, 0.70),
        CompactUtilityText::Interact => Color::srgba(0.90, 0.72, 0.32, 0.76),
    }
}

fn update_gear_slot_frames(gear_frames: &mut GearSlotFrameQuery, open: bool) {
    for (slot, mut border, mut background) in gear_frames {
        if !open {
            *border = BorderColor::all(Color::srgba(0.0, 0.0, 0.0, 0.0));
            background.0 = Color::srgba(0.0, 0.0, 0.0, 0.0);
            continue;
        }
        let accent = match slot {
            GearSlotText::Weapon => Color::srgba(0.92, 0.62, 0.24, 0.84),
            GearSlotText::Charm => Color::srgba(0.62, 0.78, 0.94, 0.74),
            GearSlotText::Codex => Color::srgba(0.60, 0.42, 0.92, 0.76),
            GearSlotText::Armory => Color::srgba(0.78, 0.68, 0.44, 0.74),
        };
        *border = BorderColor::all(accent);
        background.0 = Color::srgba(0.035, 0.030, 0.026, 0.92);
    }
}

#[allow(clippy::too_many_arguments)]
fn update_gear_slots(
    gear_slots: &mut GearSlotTextQuery,
    open: bool,
    damage_bonus: &DamageBonus,
    equipment: &Equipment,
    charm: &Charm,
    codex: &LegendaryCodex,
    armory: &Armory,
    talents: &Talents,
    runes: &SkillRunes,
    resonance: &str,
) {
    for (slot, mut text) in gear_slots {
        **text = if open {
            match slot {
                GearSlotText::Weapon => format!(
                    "WEAPON\n{} {}\n+{:.0} dmg  {:.0}% crit{}{}\n{}",
                    equipment.quality,
                    compact_label(&equipment.weapon_name, 20),
                    damage_bonus.0,
                    equipment.crit_chance * 100.0,
                    temper_label(equipment.temper_level),
                    gem_label(equipment.socketed_gem),
                    compact_label(
                        &weapon_build_tag(equipment, damage_bonus, equipment.socketed_gem),
                        30
                    )
                ),
                GearSlotText::Charm => format!(
                    "CHARM\n{} {}\n+{:.0} dmg  {:.0}% crit\n{}",
                    charm.quality,
                    compact_label(&charm.name, 21),
                    charm.damage_bonus,
                    charm.crit_chance * 100.0,
                    compact_label(&charm_build_tag(charm), 30)
                ),
                GearSlotText::Codex => format!(
                    "BUILD\n{}\n{}\n{}",
                    compact_label(&talents.summary(), 30),
                    compact_label(&runes.summary(), 30),
                    compact_label(&codex.summary(), 30)
                ),
                GearSlotText::Armory => format!(
                    "ARMORY\n{}\n{}\n{}",
                    compact_label(&armory.summary(), 30),
                    compact_label(resonance, 30),
                    compact_label(
                        &weapon_build_tag(equipment, damage_bonus, equipment.socketed_gem),
                        30
                    )
                ),
            }
        } else {
            String::new()
        };
    }
}

fn cursor_physical_position(window: &Window) -> Option<Vec2> {
    window
        .cursor_position()
        .map(|cursor| cursor * window.scale_factor())
}

fn node_under_cursor(
    node: &ComputedNode,
    transform: &UiGlobalTransform,
    cursor_physical: Vec2,
) -> bool {
    node.contains_point(*transform, cursor_physical)
}

fn update_ui_pointer_capture(
    mut capture: ResMut<UiPointerCapture>,
    windows: Query<&Window, With<PrimaryWindow>>,
    inventory_open: Res<InventoryOpen>,
    build_open: Res<BuildOpen>,
    panels: Query<(&ComputedNode, &UiGlobalTransform, &ViewVisibility), With<InventoryPanel>>,
    build_panels: Query<(&ComputedNode, &UiGlobalTransform, &ViewVisibility), With<BuildPanel>>,
) {
    let mut over_ui = false;
    if let Ok(window) = windows.single()
        && let Some(cursor) = cursor_physical_position(window)
    {
        if inventory_open.open {
            over_ui |= panels
                .iter()
                .any(|(node, transform, _)| node_under_cursor(node, transform, cursor));
        }
        if build_open.open {
            over_ui |= build_panels
                .iter()
                .any(|(node, transform, _)| node_under_cursor(node, transform, cursor));
        }
    }
    if capture.0 != over_ui {
        capture.0 = over_ui;
    }
}

fn handle_inventory_slot_clicks(
    inventory_open: Res<InventoryOpen>,
    mouse: Res<ButtonInput<MouseButton>>,
    windows: Query<&Window, With<PrimaryWindow>>,
    slots: Query<(&ComputedNode, &UiGlobalTransform, &InventorySlotText)>,
    mut combat_events: MessageWriter<CombatEvent>,
    mut player: Query<
        (
            &mut Inventory,
            &mut Equipment,
            &mut DamageBonus,
            &mut Health,
        ),
        With<Player>,
    >,
) {
    if !inventory_open.open || !mouse.just_pressed(MouseButton::Left) {
        return;
    }
    let Some(cursor) = windows.single().ok().and_then(cursor_physical_position) else {
        return;
    };
    let Ok((mut inventory, mut equipment, mut damage_bonus, mut health)) = player.single_mut()
    else {
        return;
    };
    for (node, transform, slot) in &slots {
        if !node_under_cursor(node, transform, cursor) {
            continue;
        }
        // The grid shows the most recent items first.
        let len = inventory.items.len();
        if slot.0 >= len {
            continue;
        }
        let index = len - 1 - slot.0;
        let item = inventory.items[index].clone();
        if item.slot.is_weapon() {
            if item.name == equipment.weapon_name {
                combat_events.write(CombatEvent {
                    text: format!("Already wielding {}", item.name),
                });
                continue;
            }
            equip_inventory_item(&item, &mut damage_bonus, &mut equipment, &mut health);
            combat_events.write(CombatEvent {
                text: format!("Equipped {}", item.name),
            });
        } else {
            let item = inventory.items.remove(index);
            let label = item.name.clone();
            let slot_label = item.slot.label();
            if let Ok(replaced) = equip_gear_piece(item, &mut equipment, &mut health) {
                if let Some(replaced) = replaced {
                    inventory.items.push(replaced);
                }
                combat_events.write(CombatEvent {
                    text: format!("Donned {label} ({slot_label})"),
                });
            }
        }
    }
}

fn handle_paper_doll_clicks(
    inventory_open: Res<InventoryOpen>,
    mouse: Res<ButtonInput<MouseButton>>,
    windows: Query<&Window, With<PrimaryWindow>>,
    slots: Query<(&ComputedNode, &UiGlobalTransform, &PaperDollSlot)>,
    mut combat_events: MessageWriter<CombatEvent>,
    mut player: Query<(&mut Inventory, &mut Equipment, &mut Health), With<Player>>,
) {
    if !inventory_open.open || !mouse.just_pressed(MouseButton::Left) {
        return;
    }
    let Some(cursor) = windows.single().ok().and_then(cursor_physical_position) else {
        return;
    };
    let Ok((mut inventory, mut equipment, mut health)) = player.single_mut() else {
        return;
    };
    for (node, transform, slot) in &slots {
        if !node_under_cursor(node, transform, cursor) || slot.0.is_weapon() {
            continue;
        }
        if inventory.items.len() >= inventory.capacity {
            combat_events.write(CombatEvent {
                text: "Bag full: salvage before unequipping".to_string(),
            });
            continue;
        }
        if let Some(removed) = unequip_gear_piece(slot.0, &mut equipment, &mut health) {
            combat_events.write(CombatEvent {
                text: format!("Stowed {} ({})", removed.name, slot.0.label()),
            });
            inventory.items.push(removed);
        }
    }
}

fn update_paper_doll_slots(
    inventory_open: Res<InventoryOpen>,
    player: Query<(&Equipment, &DamageBonus), With<Player>>,
    mut slots: Query<(
        &PaperDollSlot,
        &mut Text,
        &mut TextColor,
        &mut BorderColor,
        &mut BackgroundColor,
    )>,
) {
    let Ok((equipment, damage_bonus)) = player.single() else {
        return;
    };
    for (slot, mut text, mut text_color, mut border, mut background) in &mut slots {
        if !inventory_open.open {
            **text = String::new();
            *border = BorderColor::all(Color::srgba(0.0, 0.0, 0.0, 0.0));
            background.0 = Color::srgba(0.0, 0.0, 0.0, 0.0);
            continue;
        }
        let title = slot.0.label().to_uppercase();
        if slot.0.is_weapon() {
            **text = format!(
                "{title}\n{}\n+{:.0} dmg {:.0}%{}",
                compact_label(&equipment.weapon_name, 13),
                damage_bonus.0,
                equipment.crit_chance * 100.0,
                temper_label(equipment.temper_level),
            );
            *border = BorderColor::all(quality_color(&equipment.quality));
            background.0 = quality_background(&equipment.quality);
            text_color.0 = Color::srgb(0.88, 0.82, 0.70);
            continue;
        }
        match equipment.worn_piece(slot.0) {
            Some(item) => {
                **text = format!(
                    "{title}\n{}\n+{:.0} tough +{:.0} hp",
                    compact_label(&item.name, 13),
                    item.armor_bonus,
                    item.health_bonus,
                );
                *border = BorderColor::all(quality_color(&item.quality));
                background.0 = quality_background(&item.quality);
                text_color.0 = Color::srgb(0.88, 0.82, 0.70);
            }
            None => {
                **text = format!("{title}\n—");
                *border = BorderColor::all(Color::srgba(0.45, 0.40, 0.32, 0.55));
                background.0 = Color::srgba(0.030, 0.026, 0.022, 0.90);
                text_color.0 = Color::srgba(0.62, 0.56, 0.46, 0.80);
            }
        }
    }
}

fn update_inventory_slots(
    inventory_slots: &mut InventorySlotTextQuery,
    open: bool,
    inventory: &Inventory,
    equipment: &Equipment,
    damage_bonus: &DamageBonus,
) {
    let recent_items = inventory.items.iter().rev().take(10).collect::<Vec<_>>();
    for (slot, mut text) in inventory_slots {
        **text = if open {
            recent_items
                .get(slot.0)
                .map(|item| inventory_slot_label(item, equipment, damage_bonus))
                .unwrap_or_else(|| "Empty".to_string())
        } else {
            String::new()
        };
    }
}

fn update_inventory_slot_frames(
    inventory_frames: &mut InventorySlotFrameQuery,
    open: bool,
    inventory: &Inventory,
    equipment: &Equipment,
) {
    let recent_items = inventory.items.iter().rev().take(10).collect::<Vec<_>>();
    for (slot, mut border, mut background) in inventory_frames {
        if !open {
            *border = BorderColor::all(Color::srgba(0.0, 0.0, 0.0, 0.0));
            background.0 = Color::srgba(0.0, 0.0, 0.0, 0.0);
            continue;
        }
        let Some(item) = recent_items.get(slot.0) else {
            *border = BorderColor::all(Color::srgba(0.20, 0.22, 0.25, 0.50));
            background.0 = Color::srgba(0.012, 0.014, 0.018, 0.86);
            continue;
        };
        let selected = item.name == equipment.weapon_name;
        *border = BorderColor::all(if selected {
            Color::srgba(1.0, 0.86, 0.36, 1.0)
        } else {
            quality_color(&item.quality)
        });
        background.0 = if selected {
            Color::srgba(0.15, 0.11, 0.035, 0.96)
        } else {
            quality_background(&item.quality)
        };
    }
}

fn inventory_slot_label(
    item: &InventoryItem,
    equipment: &Equipment,
    damage_bonus: &DamageBonus,
) -> String {
    let power = if item.legendary_power == LegendaryPower::None {
        String::new()
    } else {
        format!("\n{}", compact_label(item.legendary_power.summary(), 12))
    };
    format!(
        "{} {} | {} | {}\n{}\n+{:.0} dmg {:.0}% crit\n{}{}{}{}{}",
        inventory_item_decision(item, equipment, damage_bonus),
        item.quality,
        inventory_item_decision_reason(item, equipment, damage_bonus),
        inventory_item_action_hint(item, equipment, damage_bonus),
        compact_label(&item.name, 12),
        item.damage_bonus,
        item.crit_chance * 100.0,
        inventory_item_power_delta_label(item, equipment, damage_bonus),
        inventory_item_callout_line(item, equipment, damage_bonus),
        compact_bonus_line("+hp", item.health_bonus),
        compact_bonus_line("+arm", item.armor_bonus),
        power
    )
}

fn inventory_item_action_hint(
    item: &InventoryItem,
    equipment: &Equipment,
    damage_bonus: &DamageBonus,
) -> &'static str {
    if item.name == equipment.weapon_name {
        return "equipped";
    }
    match inventory_item_decision_reason(item, equipment, damage_bonus) {
        "BUILD POWER" if inventory_item_power_delta(item, equipment, damage_bonus) >= 5.0 => {
            "[ equip power"
        }
        "BUILD POWER" => "B codex",
        "POWER SPIKE" | "DPS GAIN" | "TOUGHNESS" => "[ equip now",
        "SOCKET" | "TEMPER" => "keep craft",
        "SWAP BUILD" => "stash swap",
        "SALVAGE" => "U salvage",
        _ if inventory_item_power_delta(item, equipment, damage_bonus) >= 5.0 => "[ equip now",
        _ => "stash inspect",
    }
}

fn inventory_item_decision(
    item: &InventoryItem,
    equipment: &Equipment,
    damage_bonus: &DamageBonus,
) -> &'static str {
    if item.name == equipment.weapon_name {
        return "EQUIPPED";
    }
    let delta = inventory_item_power_delta(item, equipment, damage_bonus);
    if delta >= 5.0 {
        "UPGRADE"
    } else if delta >= -5.0 {
        "SIDEGRADE"
    } else {
        "STASH"
    }
}

fn inventory_item_decision_reason(
    item: &InventoryItem,
    equipment: &Equipment,
    damage_bonus: &DamageBonus,
) -> &'static str {
    if item.name == equipment.weapon_name {
        return "CURRENT";
    }
    if item.legendary_power != LegendaryPower::None
        && item.legendary_power != equipment.legendary_power
    {
        return "BUILD POWER";
    }
    let delta = inventory_item_power_delta(item, equipment, damage_bonus);
    if delta >= 25.0 {
        return "POWER SPIKE";
    }
    if item.damage_bonus - damage_bonus.0 >= 5.0
        || (item.crit_chance - equipment.crit_chance) * 100.0 >= 5.0
    {
        return "DPS GAIN";
    }
    if item.health_bonus - equipment.health_bonus + item.armor_bonus - equipment.armor_bonus >= 20.0
    {
        return "TOUGHNESS";
    }
    if matches!((item.socketed_gem, equipment.socketed_gem), (Some(_), None))
        || matches!(
            (item.socketed_gem, equipment.socketed_gem),
            (Some(item_gem), Some(equipped_gem)) if item_gem.rank > equipped_gem.rank
        )
    {
        return "SOCKET";
    }
    if item.temper_level > equipment.temper_level {
        return "TEMPER";
    }
    if delta >= -5.0 {
        "SWAP BUILD"
    } else {
        "SALVAGE"
    }
}

fn inventory_item_power_delta_label(
    item: &InventoryItem,
    equipment: &Equipment,
    damage_bonus: &DamageBonus,
) -> String {
    format!(
        "PWR {:+.0} {}",
        inventory_item_power_delta(item, equipment, damage_bonus),
        compare_delta_summary(item, equipment, damage_bonus)
    )
}

fn inventory_item_power_delta(
    item: &InventoryItem,
    equipment: &Equipment,
    damage_bonus: &DamageBonus,
) -> f32 {
    inventory_item_power_score(item) - equipped_power_score(equipment, damage_bonus)
}

fn equipped_power_score(equipment: &Equipment, damage_bonus: &DamageBonus) -> f32 {
    let item = InventoryItem {
        name: equipment.weapon_name.clone(),
        quality: equipment.quality.clone(),
        damage_bonus: damage_bonus.0,
        crit_chance: equipment.crit_chance,
        health_bonus: equipment.health_bonus,
        armor_bonus: equipment.armor_bonus,
        legendary_power: equipment.legendary_power,
        temper_level: equipment.temper_level,
        socketed_gem: equipment.socketed_gem,
        slot: GearSlot::Weapon,
    };
    inventory_item_power_score(&item)
}

fn inventory_item_power_score(item: &InventoryItem) -> f32 {
    let quality_multiplier = match item.quality.as_str() {
        "primal" => 4.8,
        "ancient" => 3.8,
        "legendary" => 3.0,
        "rare" => 2.0,
        _ => 1.0,
    };
    (12.0
        + item.damage_bonus * 2.5
        + item.crit_chance * 120.0
        + item.health_bonus * 0.6
        + item.armor_bonus * 0.8
        + inventory_legendary_power_score(item.legendary_power)
        + item
            .socketed_gem
            .map(|gem| 10.0 + gem.rank as f32 * 6.0)
            .unwrap_or_default()
        + item.temper_level as f32 * 8.0)
        * quality_multiplier
}

fn inventory_item_callout_line(
    item: &InventoryItem,
    equipment: &Equipment,
    damage_bonus: &DamageBonus,
) -> String {
    let callouts = inventory_item_callouts(item, equipment, damage_bonus);
    if callouts.is_empty() {
        String::new()
    } else {
        format!("\n{}", callouts.join(" | "))
    }
}

fn inventory_item_callouts(
    item: &InventoryItem,
    equipment: &Equipment,
    damage_bonus: &DamageBonus,
) -> Vec<String> {
    let mut callouts = Vec::new();
    if item.name == equipment.weapon_name {
        return callouts;
    }
    if item.legendary_power != LegendaryPower::None
        && item.legendary_power != equipment.legendary_power
    {
        let prefix = if equipment.legendary_power == LegendaryPower::None {
            "new power"
        } else {
            "power swap"
        };
        callouts.push(format!("{prefix} {}", item.legendary_power.label()));
        callouts.push(item.legendary_power.build_role().to_string());
    }
    match (item.socketed_gem, equipment.socketed_gem) {
        (Some(item_gem), Some(equipped_gem)) if item_gem.rank > equipped_gem.rank => {
            callouts.push(format!("socket {}", item_gem.label()));
        }
        (Some(item_gem), None) => callouts.push(format!("socket {}", item_gem.label())),
        _ => {}
    }
    if item.temper_level > equipment.temper_level {
        callouts.push(format!(
            "temper +{}",
            item.temper_level - equipment.temper_level
        ));
    }
    let damage_delta = item.damage_bonus - damage_bonus.0;
    if damage_delta >= 5.0 {
        callouts.push(format!("weapon dmg +{damage_delta:.0}"));
    }
    let crit_delta = (item.crit_chance - equipment.crit_chance) * 100.0;
    if crit_delta >= 5.0 {
        callouts.push(format!("crit build +{crit_delta:.0}%"));
    }
    let toughness_delta =
        item.health_bonus - equipment.health_bonus + item.armor_bonus - equipment.armor_bonus;
    if toughness_delta >= 20.0 {
        callouts.push(format!("toughness +{toughness_delta:.0}"));
    }
    callouts
}

fn inventory_legendary_power_score(power: LegendaryPower) -> f32 {
    if power == LegendaryPower::None {
        0.0
    } else {
        30.0
    }
}

fn next_weapon_compare_summary(
    inventory: &Inventory,
    equipment: &Equipment,
    damage_bonus: &DamageBonus,
) -> String {
    let Some(index) = inventory_swap_index(inventory, &equipment.weapon_name, 1) else {
        return "No spare weapon".to_string();
    };
    let Some(item) = inventory.items.get(index) else {
        return "No spare weapon".to_string();
    };
    format!(
        "Next {} {} ({})",
        item.quality,
        compact_label(&item.name, 18),
        compare_delta_summary(item, equipment, damage_bonus)
    )
}

fn compare_delta_summary(
    item: &InventoryItem,
    equipment: &Equipment,
    damage_bonus: &DamageBonus,
) -> String {
    format!(
        "{:+.0} dmg {:+.0}% crit {:+.0} hp {:+.0} arm",
        item.damage_bonus - damage_bonus.0,
        (item.crit_chance - equipment.crit_chance) * 100.0,
        item.health_bonus - equipment.health_bonus,
        item.armor_bonus - equipment.armor_bonus
    )
}

fn quality_color(quality: &str) -> Color {
    match quality.to_ascii_lowercase().as_str() {
        "primal" => Color::srgba(1.0, 0.18, 0.12, 0.96),
        "ancient" => Color::srgba(0.96, 0.34, 0.86, 0.94),
        "legendary" => Color::srgba(1.0, 0.58, 0.18, 0.92),
        "rare" => Color::srgba(0.34, 0.58, 1.0, 0.88),
        "magic" => Color::srgba(0.35, 0.86, 0.92, 0.84),
        "set" => Color::srgba(0.22, 0.92, 0.34, 0.90),
        _ => Color::srgba(0.56, 0.56, 0.58, 0.72),
    }
}

fn quality_background(quality: &str) -> Color {
    match quality.to_ascii_lowercase().as_str() {
        "primal" => Color::srgba(0.16, 0.030, 0.024, 0.92),
        "ancient" => Color::srgba(0.11, 0.035, 0.12, 0.90),
        "legendary" => Color::srgba(0.13, 0.075, 0.030, 0.90),
        "rare" => Color::srgba(0.030, 0.048, 0.105, 0.90),
        "magic" => Color::srgba(0.024, 0.080, 0.090, 0.88),
        "set" => Color::srgba(0.026, 0.096, 0.040, 0.90),
        _ => Color::srgba(0.020, 0.024, 0.030, 0.90),
    }
}

fn compact_bonus_line(label: &str, value: f32) -> String {
    if value <= 0.0 {
        String::new()
    } else {
        format!("\n{} {:.0}", label, value)
    }
}

fn weapon_build_tag(
    equipment: &Equipment,
    damage_bonus: &DamageBonus,
    socketed_gem: Option<SocketedGem>,
) -> String {
    let mut tags = Vec::new();
    if damage_bonus.0 >= 24.0 {
        tags.push("DPS carry".to_string());
    } else if damage_bonus.0 >= 14.0 {
        tags.push("DPS ready".to_string());
    } else {
        tags.push("builder weapon".to_string());
    }
    if equipment.crit_chance >= 0.16 {
        tags.push("crit build".to_string());
    }
    if equipment.health_bonus + equipment.armor_bonus >= 38.0 {
        tags.push("toughness".to_string());
    }
    if equipment.legendary_power != LegendaryPower::None {
        tags.push(equipment.legendary_power.build_role().to_string());
    }
    if let Some(gem) = socketed_gem {
        tags.push(format!("{} socket", gem.label()));
    } else if matches!(
        equipment.quality.as_str(),
        "legendary" | "ancient" | "primal"
    ) {
        tags.push("socket target".to_string());
    }
    if equipment.temper_level > 0 {
        tags.push(format!("temper +{}", equipment.temper_level));
    }
    tags.truncate(3);
    tags.join(" | ")
}

fn charm_build_tag(charm: &Charm) -> String {
    let mut tags = Vec::new();
    if charm.damage_bonus >= 6.0 {
        tags.push("DPS charm");
    }
    if charm.crit_chance >= 0.04 {
        tags.push("crit charm");
    }
    if matches!(charm.quality.as_str(), "legendary" | "ancient" | "primal") {
        tags.push("resonance piece");
    }
    if tags.is_empty() {
        tags.push("stat filler");
    }
    tags.join(" | ")
}

fn compact_label(text: &str, max_chars: usize) -> String {
    let mut out = String::new();
    for ch in text.chars().take(max_chars) {
        out.push(ch);
    }
    if text.chars().count() > max_chars {
        out.push_str("...");
    }
    out
}

fn update_resource_fills(
    resource_fills: &mut ResourceFillQuery,
    health: &Health,
    barrier: &Barrier,
    fury: &Fury,
) {
    for (fill, mut node, mut color) in resource_fills {
        let amount = match fill {
            ResourceFill::Health => percent(health.current, health.max),
            ResourceFill::Barrier => percent(barrier.current, barrier.max),
            ResourceFill::Fury => percent(fury.current, fury.max),
        };
        if fill.fills_vertically() {
            node.width = Val::Percent(100.0);
            node.height = Val::Percent(amount);
        } else {
            node.width = Val::Percent(amount);
            node.height = Val::Px(8.0);
        }
        match fill {
            ResourceFill::Health => {
                color.0 = if health.current <= health.max * 0.30 {
                    Color::srgba(0.95, 0.02, 0.02, 0.92)
                } else {
                    Color::srgba(0.74, 0.03, 0.02, 0.86)
                };
            }
            ResourceFill::Barrier => {
                color.0 = if barrier.current > 0.0 {
                    Color::srgba(0.88, 0.72, 0.30, 0.90)
                } else {
                    Color::srgba(0.18, 0.12, 0.06, 0.42)
                };
            }
            ResourceFill::Fury => {
                color.0 = if fury.current >= fury.nova_cost {
                    Color::srgba(0.08, 0.32, 0.98, 0.88)
                } else {
                    Color::srgba(0.04, 0.16, 0.48, 0.74)
                };
            }
        }
    }
}

fn update_resource_orb_frames(
    frames: &mut ResourceOrbFrameQuery,
    health: &Health,
    barrier: &Barrier,
    fury: &Fury,
    surge: &SurgeBuff,
) {
    for (frame, mut border, mut background) in frames {
        let style = resource_orb_frame_style(*frame, health, barrier, fury, surge);
        *border = BorderColor::all(style.border);
        background.0 = style.background;
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct ResourceOrbFrameStyle {
    border: Color,
    background: Color,
}

fn resource_orb_frame_style(
    frame: ResourceOrbFrame,
    health: &Health,
    barrier: &Barrier,
    fury: &Fury,
    surge: &SurgeBuff,
) -> ResourceOrbFrameStyle {
    match frame {
        ResourceOrbFrame::Health => health_orb_frame_style(health, barrier),
        ResourceOrbFrame::Fury => fury_orb_frame_style(fury, surge),
    }
}

fn health_orb_frame_style(health: &Health, barrier: &Barrier) -> ResourceOrbFrameStyle {
    let health_ratio = if health.max > 0.0 {
        health.current.max(0.0) / health.max
    } else {
        1.0
    };
    if health_ratio <= 0.30 {
        return ResourceOrbFrameStyle {
            border: Color::srgba(1.0, 0.08, 0.04, 0.98),
            background: Color::srgba(0.082, 0.006, 0.006, 0.96),
        };
    }
    if barrier.current > 0.5 {
        return ResourceOrbFrameStyle {
            border: Color::srgba(0.96, 0.70, 0.24, 0.94),
            background: Color::srgba(0.060, 0.032, 0.010, 0.95),
        };
    }
    if health_ratio <= 0.55 {
        return ResourceOrbFrameStyle {
            border: Color::srgba(1.0, 0.34, 0.10, 0.92),
            background: Color::srgba(0.060, 0.014, 0.010, 0.95),
        };
    }
    ResourceOrbFrameStyle {
        border: Color::srgba(0.75, 0.10, 0.08, 0.82),
        background: Color::srgba(0.040, 0.006, 0.008, 0.94),
    }
}

fn fury_orb_frame_style(fury: &Fury, surge: &SurgeBuff) -> ResourceOrbFrameStyle {
    if surge_seconds_remaining(surge) > 0.0 {
        return ResourceOrbFrameStyle {
            border: Color::srgba(0.42, 1.0, 0.86, 0.98),
            background: Color::srgba(0.006, 0.050, 0.055, 0.96),
        };
    }
    if surge.charge >= surge.max_charge {
        return ResourceOrbFrameStyle {
            border: Color::srgba(1.0, 0.70, 0.22, 0.98),
            background: Color::srgba(0.070, 0.038, 0.010, 0.96),
        };
    }
    let min_spender = fury.dash_cost.min(fury.rupture_cost).min(fury.nova_cost);
    if fury.current + f32::EPSILON < min_spender {
        return ResourceOrbFrameStyle {
            border: Color::srgba(0.72, 0.12, 0.12, 0.88),
            background: Color::srgba(0.036, 0.010, 0.018, 0.95),
        };
    }
    if fury.current >= fury.nova_cost {
        return ResourceOrbFrameStyle {
            border: Color::srgba(0.24, 0.58, 1.0, 0.96),
            background: Color::srgba(0.006, 0.026, 0.078, 0.96),
        };
    }
    ResourceOrbFrameStyle {
        border: Color::srgba(0.16, 0.34, 0.86, 0.82),
        background: Color::srgba(0.004, 0.014, 0.050, 0.94),
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct LowHealthVignetteStyle {
    visibility: Visibility,
    color: Color,
}

fn update_low_health_vignette_hud(
    player: Query<(&Health, &Barrier), With<Player>>,
    mut vignettes: Query<(&mut Visibility, &mut BackgroundColor), With<LowHealthVignette>>,
) {
    let Ok((health, barrier)) = player.single() else {
        return;
    };
    let style = low_health_vignette_style(health, barrier);
    for (mut visibility, mut color) in &mut vignettes {
        *visibility = style.visibility;
        color.0 = style.color;
    }
}

fn low_health_vignette_style(health: &Health, barrier: &Barrier) -> LowHealthVignetteStyle {
    let health_percent = if health.max > 0.0 {
        health.current.max(0.0) / health.max
    } else {
        1.0
    };
    if health_percent > 0.55 {
        return LowHealthVignetteStyle {
            visibility: Visibility::Hidden,
            color: Color::srgba(0.0, 0.0, 0.0, 0.0),
        };
    }

    let missing = (1.0 - health_percent).clamp(0.0, 1.0);
    let guarded = barrier.current > 0.5;
    let alpha = if health_percent <= 0.25 {
        0.08 + missing * if guarded { 0.08 } else { 0.15 }
    } else {
        0.035 + missing * if guarded { 0.04 } else { 0.07 }
    }
    .min(if guarded { 0.15 } else { 0.22 });
    let color = if guarded {
        Color::srgba(0.72, 0.18, 0.035, alpha)
    } else {
        Color::srgba(0.84, 0.015, 0.012, alpha)
    };

    LowHealthVignetteStyle {
        visibility: Visibility::Visible,
        color,
    }
}

fn hud_objective_text(
    progress: &ChapterProgress,
    zone: &ChapterZoneState,
    player_position: Vec3,
    live_priority: Option<&str>,
) -> String {
    let priority = live_priority
        .map(|priority| format!(" | {}", compact_label(priority, 18)))
        .unwrap_or_default();
    let step = compact_label(&chapter_next_step(progress), 18);
    let nav = compact_label(
        &chapter_navigation_hint(progress, zone, player_position),
        14,
    );
    let payoff = compact_label(&hud_payoff_label(progress), 18);
    format!(
        "Q{:.0}% {}{} | {} {}/{} | Next {} | {} | {}",
        chapter_mainline_percent(progress),
        chapter_route_nodes_compact(progress.phase),
        priority,
        phase_label(progress.phase),
        progress.seal_fragments,
        REQUIRED_SEAL_FRAGMENTS,
        step,
        nav,
        payoff
    )
}

fn hud_clean_objective_text(
    progress: &ChapterProgress,
    zone: &ChapterZoneState,
    player_position: Vec3,
    live_priority: Option<&str>,
) -> String {
    let primary = live_priority
        .map(|priority| compact_label(priority, 22))
        .unwrap_or_else(|| compact_label(&chapter_next_step(progress), 24));
    let nav = compact_label(
        &chapter_navigation_hint(progress, zone, player_position),
        15,
    );
    format!(
        "{} {}/{} | {} | {}",
        phase_label(progress.phase),
        progress.seal_fragments,
        REQUIRED_SEAL_FRAGMENTS,
        primary,
        nav
    )
}

fn clean_hud_active(
    hud_density: HudDensity,
    pause: &PauseState,
    inventory_open: &InventoryOpen,
    build_open: &BuildOpen,
    journal_open: &JournalOpen,
) -> bool {
    hud_density.is_clean()
        && !pause.paused
        && !inventory_open.open
        && !build_open.open
        && !journal_open.open
}

fn hud_payoff_label(progress: &ChapterProgress) -> String {
    chapter_phase_payoff(progress)
        .trim_start_matches("PAYOFF ")
        .trim_start_matches("NEXT ")
        .to_string()
}

fn hud_combat_focus_active(
    progress: &ChapterProgress,
    health: &Health,
    target_info: &TargetInfo,
    boss: Option<BossActionGuideState>,
    obelisk: &BloodObelisk,
    rift: &EmberRift,
) -> bool {
    target_info.visible
        || boss.is_some()
        || progress.phase == ChapterPhase::Boss
        || player_health_danger(health)
        || matches!(obelisk.state, ObeliskState::Active)
        || matches!(rift.state, RiftState::Active)
}

fn hud_combat_focus_text(
    action_guide: &str,
    target_info: &TargetInfo,
    progress: &ChapterProgress,
) -> String {
    let primary = action_guide_primary_line(action_guide);
    let target = if target_info.visible {
        format!(
            "{} {}",
            compact_label(&target_info.name, 9),
            compact_target_health_line(&target_info.health_line)
        )
    } else if progress.phase == ChapterPhase::Boss {
        "Malrec".to_string()
    } else {
        "nearest threat".to_string()
    };
    format!(
        "NOW {} | {} | {}",
        compact_label(&primary, 20),
        target,
        compact_label(&chapter_next_step(progress), 14)
    )
}

fn compact_target_health_line(health_line: &str) -> String {
    health_line
        .split_whitespace()
        .next()
        .map(|line| compact_label(line, 10))
        .unwrap_or_else(|| "?".to_string())
}

fn action_guide_primary_line(action_guide: &str) -> String {
    action_guide
        .strip_prefix("Action Guide: ")
        .unwrap_or(action_guide)
        .split_once(" | ")
        .map(|(primary, _)| primary)
        .unwrap_or(action_guide)
        .to_string()
}

fn live_objective_priority(
    stats: &RunStats,
    bounty: &BountyProgress,
    obelisk: &BloodObelisk,
    rift: &EmberRift,
) -> Option<String> {
    if rift.state == RiftState::Active {
        let echo = if rift.remaining_secs >= crate::rift::RIFT_SWIFT_SEAL_SECS {
            " Echo"
        } else {
            ""
        };
        return Some(format!(
            "NOW rift {}/{}{}",
            rift.kills.min(rift.target_kills),
            rift.target_kills,
            echo
        ));
    }
    if obelisk.state == ObeliskState::Active {
        return Some(format!(
            "NOW obelisk {}/{}",
            obelisk.kills.min(obelisk.target_kills),
            obelisk.target_kills
        ));
    }
    if bounty.completed && !bounty.reward_claimed {
        return Some("NOW claim bounty".to_string());
    }
    if stats.ashen_threat >= 75 && stats.ashen_threat_surges < ASHEN_THREAT_SURGE_TARGET {
        return Some("NOW threat surge".to_string());
    }
    None
}

fn chapter_navigation_hint(
    progress: &ChapterProgress,
    zone: &ChapterZoneState,
    player_position: Vec3,
) -> String {
    if let Some((label, target)) = mainline_navigation_target(progress) {
        return format!(
            "{} {:.0}m {}",
            label,
            flat_distance(player_position, target),
            compass_direction(player_position, target)
        );
    }
    compact_label(zone.current.tactical_hint(), 28)
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ObjectiveCompassDisplay {
    direction: &'static str,
    detail: String,
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct ObjectiveCompassTarget {
    scope: &'static str,
    label: &'static str,
    position: Vec3,
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct ObjectiveCompassEnemyTarget<'a> {
    id: &'a str,
    current_health: f32,
    position: Vec3,
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct ObjectiveCompassInteractableTarget {
    kind: InteractableKind,
    used: bool,
    reusable: bool,
    position: Vec3,
}

#[cfg(test)]
fn objective_compass_display(
    progress: &ChapterProgress,
    player_position: Vec3,
) -> Option<ObjectiveCompassDisplay> {
    objective_compass_display_from_targets(
        progress,
        player_position,
        std::iter::empty(),
        std::iter::empty(),
    )
}

fn objective_compass_display_from_targets<'a>(
    progress: &ChapterProgress,
    player_position: Vec3,
    enemies: impl IntoIterator<Item = ObjectiveCompassEnemyTarget<'a>>,
    interactables: impl IntoIterator<Item = ObjectiveCompassInteractableTarget>,
) -> Option<ObjectiveCompassDisplay> {
    let target = fixed_mainline_compass_target(progress)
        .or_else(|| mainline_enemy_compass_target(progress, player_position, enemies))
        .or_else(|| boss_fallback_compass_target(progress))
        .or_else(|| optional_interactable_compass_target(player_position, interactables))?;
    Some(objective_compass_display_for_target(
        target,
        player_position,
    ))
}

fn objective_compass_display_for_target(
    target: ObjectiveCompassTarget,
    player_position: Vec3,
) -> ObjectiveCompassDisplay {
    let direction = match compass_direction(player_position, target.position) {
        "here" | "near" => "HERE",
        direction => direction,
    };
    ObjectiveCompassDisplay {
        direction,
        detail: format!(
            "{} {:.0}m {}",
            target.scope,
            flat_distance(player_position, target.position),
            target.label
        ),
    }
}

fn fixed_mainline_compass_target(progress: &ChapterProgress) -> Option<ObjectiveCompassTarget> {
    match progress.phase {
        ChapterPhase::Cache if !progress.chest_opened => Some(ObjectiveCompassTarget {
            scope: "MAIN",
            label: "Cache",
            position: Vec3::new(-8.0, 0.0, -5.5),
        }),
        ChapterPhase::Ritual if !progress.altar_activated => Some(ObjectiveCompassTarget {
            scope: "MAIN",
            label: "Altar",
            position: Vec3::new(6.5, 0.0, 4.8),
        }),
        _ => None,
    }
}

fn mainline_enemy_compass_target<'a>(
    progress: &ChapterProgress,
    player_position: Vec3,
    enemies: impl IntoIterator<Item = ObjectiveCompassEnemyTarget<'a>>,
) -> Option<ObjectiveCompassTarget> {
    let mut best: Option<(ObjectiveCompassTarget, f32)> = None;
    for enemy in enemies {
        if enemy.current_health <= 0.0 {
            continue;
        }
        let Some(label) = mainline_enemy_compass_label(progress, enemy.id) else {
            continue;
        };
        let target = ObjectiveCompassTarget {
            scope: "MAIN",
            label,
            position: enemy.position,
        };
        let distance = flat_distance(player_position, enemy.position);
        if best
            .as_ref()
            .is_none_or(|(_, best_distance)| distance < *best_distance)
        {
            best = Some((target, distance));
        }
    }
    best.map(|(target, _)| target)
}

fn mainline_enemy_compass_label(
    progress: &ChapterProgress,
    enemy_id: &str,
) -> Option<&'static str> {
    match (progress.phase, enemy_id) {
        (ChapterPhase::Sanctum, "seal_warden") => Some("Warden"),
        (ChapterPhase::Boss, "keeper") if !progress.boss_slain => Some("Malrec"),
        _ => None,
    }
}

fn boss_fallback_compass_target(progress: &ChapterProgress) -> Option<ObjectiveCompassTarget> {
    match progress.phase {
        ChapterPhase::Boss if !progress.boss_slain => Some(ObjectiveCompassTarget {
            scope: "MAIN",
            label: "Malrec",
            position: Vec3::new(6.5, 0.0, 4.8),
        }),
        _ => None,
    }
}

fn optional_interactable_compass_target(
    player_position: Vec3,
    interactables: impl IntoIterator<Item = ObjectiveCompassInteractableTarget>,
) -> Option<ObjectiveCompassTarget> {
    let mut best: Option<(ObjectiveCompassTarget, u8, f32)> = None;
    for interactable in interactables {
        if interactable.used && !interactable.reusable {
            continue;
        }
        let Some(priority) = optional_interactable_compass_priority(interactable.kind) else {
            continue;
        };
        let distance = flat_distance(player_position, interactable.position);
        let target = ObjectiveCompassTarget {
            scope: "SIDE",
            label: optional_interactable_compass_label(interactable.kind),
            position: interactable.position,
        };
        let is_better = best
            .as_ref()
            .is_none_or(|(_, best_priority, best_distance)| {
                priority < *best_priority
                    || (priority == *best_priority && distance < *best_distance)
            });
        if is_better {
            best = Some((target, priority, distance));
        }
    }
    best.map(|(target, _, _)| target)
}

fn optional_interactable_compass_priority(kind: InteractableKind) -> Option<u8> {
    match kind {
        InteractableKind::EmberRift => Some(0),
        InteractableKind::ReliquaryVault => Some(1),
        InteractableKind::BloodObelisk
        | InteractableKind::CursedShrine
        | InteractableKind::AshenPylon => Some(2),
        InteractableKind::StormShrine
        | InteractableKind::FortuneShrine
        | InteractableKind::WeaponShrine => Some(3),
        InteractableKind::HealingWell => Some(4),
        InteractableKind::LorePage => Some(5),
        InteractableKind::Merchant => Some(6),
        InteractableKind::Cache | InteractableKind::Altar => None,
    }
}

fn optional_interactable_compass_label(kind: InteractableKind) -> &'static str {
    match kind {
        InteractableKind::EmberRift => "Rift",
        InteractableKind::ReliquaryVault => "Vault",
        InteractableKind::BloodObelisk => "Obelisk",
        InteractableKind::CursedShrine => "Cursed",
        InteractableKind::AshenPylon => "Pylon",
        InteractableKind::StormShrine => "Storm",
        InteractableKind::FortuneShrine => "Fortune",
        InteractableKind::WeaponShrine => "Weapon",
        InteractableKind::HealingWell => "Well",
        InteractableKind::LorePage => "Lore",
        InteractableKind::Merchant => "Craft",
        InteractableKind::Cache => "Cache",
        InteractableKind::Altar => "Altar",
    }
}

fn mainline_navigation_target(progress: &ChapterProgress) -> Option<(&'static str, Vec3)> {
    match progress.phase {
        ChapterPhase::Cache if !progress.chest_opened => {
            Some(("Cache", Vec3::new(-8.0, 0.0, -5.5)))
        }
        ChapterPhase::Ritual if !progress.altar_activated => {
            Some(("Altar", Vec3::new(6.5, 0.0, 4.8)))
        }
        ChapterPhase::Boss if !progress.boss_slain => Some(("Malrec", Vec3::new(6.5, 0.0, 4.8))),
        _ => None,
    }
}

fn flat_distance(from: Vec3, to: Vec3) -> f32 {
    Vec2::new(to.x - from.x, to.z - from.z).length()
}

fn compass_direction(from: Vec3, to: Vec3) -> &'static str {
    let delta = Vec2::new(to.x - from.x, to.z - from.z);
    if delta.length_squared() < 0.25 {
        return "here";
    }
    let east_west = if delta.x.abs() < 0.8 {
        ""
    } else if delta.x > 0.0 {
        "E"
    } else {
        "W"
    };
    let north_south = if delta.y.abs() < 0.8 {
        ""
    } else if delta.y > 0.0 {
        "S"
    } else {
        "N"
    };
    match (north_south, east_west) {
        ("", "") => "near",
        ("", ew) => ew,
        (ns, "") => ns,
        ("N", "E") => "NE",
        ("N", "W") => "NW",
        ("S", "E") => "SE",
        ("S", "W") => "SW",
        _ => "near",
    }
}

fn chapter_phase_payoff(progress: &ChapterProgress) -> String {
    match progress.phase {
        ChapterPhase::Breach => format!(
            "NEXT Cache seal {}",
            seal_fragment_reward_preview(progress.seal_fragments.saturating_add(1))
        ),
        ChapterPhase::Cache => format!(
            "PAYOFF Outer seal {}",
            seal_fragment_reward_preview(progress.seal_fragments.saturating_add(1))
        ),
        ChapterPhase::Sanctum => "PAYOFF Final altar unlock".to_string(),
        ChapterPhase::Ritual => format!(
            "PAYOFF Final seal {}",
            seal_fragment_reward_preview(REQUIRED_SEAL_FRAGMENTS)
        ),
        ChapterPhase::Boss => "PAYOFF Soul Sigil + chapter cache".to_string(),
        ChapterPhase::Victory => "PAYOFF Claimed - start next run".to_string(),
    }
}

fn chapter_zone_summary(zone: &ChapterZoneState) -> String {
    format!(
        "Zone {} - {}",
        zone.current.label(),
        zone.current.tactical_hint()
    )
}

fn health_orb_text(
    health: &Health,
    barrier: &Barrier,
    potions: &PotionBelt,
    death_ward: &DeathWard,
) -> String {
    format!(
        "LIFE {}\n{:.0} / {:.0}\nF POTION {}/{} {}\nWARD {:.0} | SOUL {}",
        health_state_label(health),
        health.current.max(0.0),
        health.max,
        potions.current,
        potions.max,
        cooldown_label(potion_seconds_remaining(potions)).to_uppercase(),
        barrier.current.max(0.0),
        death_ward.remaining_revives()
    )
}

fn health_state_label(health: &Health) -> &'static str {
    if health.max <= 0.0 {
        return "UNKNOWN";
    }
    let percent = health.current / health.max;
    if percent <= 0.25 {
        "CRITICAL"
    } else if percent <= 0.55 {
        "WOUNDED"
    } else {
        "STABLE"
    }
}

fn fury_orb_text(
    fury: &Fury,
    level: &PlayerLevel,
    paragon: &EmberParagon,
    stats: &RunStats,
    readiness: &str,
) -> String {
    format!(
        "FURY\n{:.0} / {:.0}\n{}\n{}\n{}\n{}\n{}g {}s {}e",
        fury.current,
        fury.max,
        fury_spend_line(fury),
        fury_builder_line(fury),
        growth_orb_text(level, paragon),
        compact_label(readiness, 16),
        stats.gold,
        stats.ember_shards,
        stats.affix_essence
    )
}

fn fury_spend_line(fury: &Fury) -> String {
    format!(
        "SPEND RMB{:.0} Q{:.0} E{:.0}",
        fury.rupture_cost, fury.dash_cost, fury.nova_cost
    )
}

fn fury_builder_line(fury: &Fury) -> String {
    format!("BUILD LMB +{:.0}F", fury.basic_gain)
}

struct CombatReadinessInputs<'a> {
    health: &'a Health,
    potions: &'a PotionBelt,
    fury: &'a Fury,
    cooldowns: &'a SkillCooldowns,
    surge: &'a SurgeBuff,
    evade: &'a Evade,
    runes: &'a SkillRunes,
}

fn combat_readiness_summary(inputs: CombatReadinessInputs) -> String {
    if inputs.health.current <= inputs.health.max * 0.35 {
        return survival_readiness_summary(inputs.potions, inputs.evade);
    }
    let surge_active = surge_seconds_remaining(inputs.surge);
    if surge_active > 0.0 {
        return format!("SURGE {:.0}s chain", surge_active.ceil());
    }
    if skill_slot_ready(
        SkillSlotText::Surge,
        inputs.fury,
        inputs.cooldowns,
        inputs.surge,
        inputs.evade,
    ) {
        return "Y Surge burst".to_string();
    }
    for (slot, label) in [
        (
            SkillSlotText::Rupture,
            match inputs.runes.rupture {
                RuptureRune::Expose => "RMB start vuln",
                RuptureRune::Hemorrhage => "RMB start bleed",
            },
        ),
        (
            SkillSlotText::Nova,
            match inputs.runes.nova {
                NovaRune::Ember => "E Nova burn",
                NovaRune::Frost => "E Nova freeze",
            },
        ),
        (
            SkillSlotText::Dash,
            match inputs.runes.dash {
                DashRune::Cleanse => "Q Dash cleanse",
                DashRune::Reap => "Q Dash finish",
            },
        ),
    ] {
        if skill_slot_ready(
            slot,
            inputs.fury,
            inputs.cooldowns,
            inputs.surge,
            inputs.evade,
        ) {
            return label.to_string();
        }
    }
    let missing_nova = (inputs.fury.nova_cost - inputs.fury.current).ceil();
    if missing_nova > 0.0 {
        return format!("LMB +{missing_nova:.0}F");
    }
    "LMB Strike".to_string()
}

fn survival_readiness_summary(potions: &PotionBelt, evade: &Evade) -> String {
    let potion_cd = potion_seconds_remaining(potions);
    if potions.current > 0 && potion_cd <= 0.1 {
        return "F Potion heal".to_string();
    }
    if evade_cooldown_remaining(evade) <= 0.0 {
        return "Shift Evade".to_string();
    }
    if potions.current > 0 {
        return format!("SURVIVE pot {:.0}s", potion_cd.ceil());
    }
    "SURVIVE kite".to_string()
}

fn growth_orb_text(level: &PlayerLevel, paragon: &EmberParagon) -> String {
    if level.level >= CHAPTER_LEVEL_CAP {
        format!(
            "PARA {}  XP {}/{}",
            paragon.rank, paragon.xp, paragon.next_rank_xp
        )
    } else {
        format!(
            "LV {}  XP {}/{}",
            level.level, level.xp, level.next_level_xp
        )
    }
}

fn update_potion_pips(potion_pips: &mut PotionPipQuery, potions: &PotionBelt) {
    for (pip, mut border, mut background) in potion_pips {
        let style = potion_pip_style(pip.0, potions);
        *border = BorderColor::all(style.border);
        background.0 = style.background;
    }
}

#[derive(Debug, Clone, Copy)]
struct PotionPipStyle {
    border: Color,
    background: Color,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
enum PotionPipState {
    Locked,
    Empty,
    Recharging,
    Ready,
}

fn potion_pip_style(index: u32, potions: &PotionBelt) -> PotionPipStyle {
    match potion_pip_state(index, potions) {
        PotionPipState::Locked => PotionPipStyle {
            border: Color::srgba(0.10, 0.08, 0.08, 0.40),
            background: Color::srgba(0.020, 0.016, 0.016, 0.45),
        },
        PotionPipState::Empty => PotionPipStyle {
            border: Color::srgba(0.26, 0.16, 0.14, 0.58),
            background: Color::srgba(0.050, 0.025, 0.024, 0.72),
        },
        PotionPipState::Recharging => PotionPipStyle {
            border: Color::srgba(0.58, 0.30, 0.12, 0.74),
            background: Color::srgba(0.48, 0.10, 0.03, 0.82),
        },
        PotionPipState::Ready => PotionPipStyle {
            border: Color::srgba(0.98, 0.44, 0.28, 0.92),
            background: Color::srgba(0.76, 0.02, 0.03, 0.90),
        },
    }
}

fn potion_pip_state(index: u32, potions: &PotionBelt) -> PotionPipState {
    if index >= potions.max {
        PotionPipState::Locked
    } else if index >= potions.current {
        PotionPipState::Empty
    } else if potion_seconds_remaining(potions) > 0.0 {
        PotionPipState::Recharging
    } else {
        PotionPipState::Ready
    }
}

fn update_skill_frames(
    skill_frames: &mut SkillSlotFrameQuery,
    fury: &Fury,
    cooldowns: &SkillCooldowns,
    surge: &SurgeBuff,
    evade: &Evade,
    feedback: &SkillFeedbackState,
) {
    for (frame, mut border, mut background) in skill_frames {
        let style = skill_slot_frame_style(frame.0, fury, cooldowns, surge, evade, feedback);
        *border = BorderColor::all(style.border);
        background.0 = style.background;
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct SkillSlotFrameStyle {
    border: Color,
    background: Color,
}

fn skill_slot_frame_style(
    slot: SkillSlotText,
    fury: &Fury,
    cooldowns: &SkillCooldowns,
    surge: &SurgeBuff,
    evade: &Evade,
    feedback: &SkillFeedbackState,
) -> SkillSlotFrameStyle {
    if let Some(pulse) = active_skill_slot_pulse(feedback)
        && pulse.slot == slot
    {
        return skill_slot_pulse_style(pulse.kind, slot);
    }

    let ready = skill_slot_ready(slot, fury, cooldowns, surge, evade);
    let blocked = skill_slot_blocked(slot, fury);
    let theme = skill_slot_theme(slot);
    if ready {
        SkillSlotFrameStyle {
            border: theme.ready_border,
            background: theme.ready_background,
        }
    } else if blocked {
        SkillSlotFrameStyle {
            border: Color::srgba(0.42, 0.12, 0.10, 0.70),
            background: Color::srgba(0.060, 0.030, 0.030, 0.92),
        }
    } else {
        SkillSlotFrameStyle {
            border: theme.idle_border,
            background: theme.idle_background,
        }
    }
}

fn active_skill_slot_pulse(feedback: &SkillFeedbackState) -> Option<SkillSlotPulse> {
    if feedback.timer.is_finished() {
        None
    } else {
        feedback.pulse
    }
}

fn skill_slot_pulse_style(kind: SkillSlotPulseKind, slot: SkillSlotText) -> SkillSlotFrameStyle {
    let theme = skill_slot_theme(slot);
    match kind {
        SkillSlotPulseKind::Cast => SkillSlotFrameStyle {
            border: Color::srgba(0.72, 0.94, 1.0, 1.0),
            background: brighten_color(theme.ready_background, 1.75, 0.98),
        },
        SkillSlotPulseKind::Blocked => SkillSlotFrameStyle {
            border: Color::srgba(1.0, 0.25, 0.16, 0.98),
            background: Color::srgba(0.16, 0.018, 0.016, 0.96),
        },
        SkillSlotPulseKind::Survival => SkillSlotFrameStyle {
            border: Color::srgba(0.46, 1.0, 0.58, 0.98),
            background: Color::srgba(0.025, 0.12, 0.044, 0.96),
        },
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct SkillSlotTheme {
    idle_border: Color,
    idle_background: Color,
    ready_border: Color,
    ready_background: Color,
}

fn skill_slot_theme(slot: SkillSlotText) -> SkillSlotTheme {
    let (border, background) = match slot {
        SkillSlotText::Strike => (
            Color::srgba(0.70, 0.56, 0.36, 0.68),
            Color::srgba(0.052, 0.044, 0.034, 0.92),
        ),
        SkillSlotText::Rupture => (
            Color::srgba(0.78, 0.18, 0.16, 0.70),
            Color::srgba(0.072, 0.022, 0.024, 0.92),
        ),
        SkillSlotText::Dash => (
            Color::srgba(0.52, 0.64, 0.86, 0.70),
            Color::srgba(0.024, 0.038, 0.068, 0.92),
        ),
        SkillSlotText::Nova => (
            Color::srgba(0.36, 0.62, 0.92, 0.72),
            Color::srgba(0.018, 0.040, 0.078, 0.92),
        ),
        SkillSlotText::Surge => (
            Color::srgba(0.92, 0.62, 0.20, 0.76),
            Color::srgba(0.075, 0.044, 0.016, 0.92),
        ),
        SkillSlotText::Evade => (
            Color::srgba(0.50, 0.70, 0.54, 0.70),
            Color::srgba(0.026, 0.052, 0.034, 0.92),
        ),
    };

    SkillSlotTheme {
        idle_border: border,
        idle_background: background,
        ready_border: brighten_color(border, 1.28, 0.96),
        ready_background: brighten_color(background, 1.42, 0.96),
    }
}

fn brighten_color(color: Color, factor: f32, alpha: f32) -> Color {
    let srgba = color.to_srgba();
    Color::srgba(
        (srgba.red * factor).min(1.0),
        (srgba.green * factor).min(1.0),
        (srgba.blue * factor).min(1.0),
        alpha,
    )
}

fn percent(current: f32, max: f32) -> f32 {
    if max <= 0.0 {
        0.0
    } else {
        (current / max * 100.0).clamp(0.0, 100.0)
    }
}

fn update_skill_cooldown_masks(
    skill_masks: &mut SkillCooldownMaskQuery,
    fury: &Fury,
    cooldowns: &SkillCooldowns,
    surge: &SurgeBuff,
    evade: &Evade,
) {
    for (mask, mut node, mut color) in skill_masks {
        let blocked = skill_slot_blocked(mask.0, fury);
        let fill = skill_slot_overlay_percent(mask.0, fury, cooldowns, surge, evade);
        node.height = Val::Percent(fill);
        color.0 = if blocked {
            Color::srgba(0.48, 0.02, 0.02, 0.58)
        } else {
            Color::srgba(0.0, 0.0, 0.0, 0.62)
        };
    }
}

fn update_skill_lamps(
    skill_lamps: &mut SkillSlotLampQuery,
    fury: &Fury,
    cooldowns: &SkillCooldowns,
    surge: &SurgeBuff,
    evade: &Evade,
) {
    for (lamp, mut node, mut color) in skill_lamps {
        let style = skill_slot_lamp_style(lamp.0, fury, cooldowns, surge, evade);
        node.width = Val::Px(style.width);
        color.0 = style.color;
    }
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
enum SkillSlotLampState {
    Ready,
    Cooling,
    Starved,
    Charging,
    Active,
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct SkillSlotLampStyle {
    state: SkillSlotLampState,
    width: f32,
    color: Color,
}

fn skill_slot_lamp_style(
    slot: SkillSlotText,
    fury: &Fury,
    cooldowns: &SkillCooldowns,
    surge: &SurgeBuff,
    evade: &Evade,
) -> SkillSlotLampStyle {
    let state = skill_slot_lamp_state(slot, fury, cooldowns, surge, evade);
    let progress = skill_slot_lamp_progress(slot, fury, cooldowns, surge, evade);
    let width = match state {
        SkillSlotLampState::Ready => 30.0,
        SkillSlotLampState::Active => 36.0,
        SkillSlotLampState::Starved => 24.0,
        SkillSlotLampState::Charging => 9.0 + progress * 25.0,
        SkillSlotLampState::Cooling => 8.0 + progress * 20.0,
    };
    let color = match state {
        SkillSlotLampState::Ready => skill_slot_theme(slot).ready_border,
        SkillSlotLampState::Active => Color::srgba(0.46, 1.0, 0.68, 0.98),
        SkillSlotLampState::Starved => Color::srgba(1.0, 0.16, 0.10, 0.92),
        SkillSlotLampState::Charging => Color::srgba(0.96, 0.58, 0.16, 0.88),
        SkillSlotLampState::Cooling => Color::srgba(0.22, 0.28, 0.38, 0.78),
    };

    SkillSlotLampStyle {
        state,
        width,
        color,
    }
}

fn skill_slot_lamp_state(
    slot: SkillSlotText,
    fury: &Fury,
    cooldowns: &SkillCooldowns,
    surge: &SurgeBuff,
    evade: &Evade,
) -> SkillSlotLampState {
    if matches!(slot, SkillSlotText::Surge) && surge_seconds_remaining(surge) > 0.0 {
        return SkillSlotLampState::Active;
    }
    if matches!(slot, SkillSlotText::Evade) && evade_seconds_remaining(evade) > 0.0 {
        return SkillSlotLampState::Active;
    }
    if skill_slot_blocked(slot, fury) {
        return SkillSlotLampState::Starved;
    }
    if skill_slot_ready(slot, fury, cooldowns, surge, evade) {
        return SkillSlotLampState::Ready;
    }
    if matches!(slot, SkillSlotText::Surge) {
        SkillSlotLampState::Charging
    } else {
        SkillSlotLampState::Cooling
    }
}

fn skill_slot_lamp_progress(
    slot: SkillSlotText,
    fury: &Fury,
    cooldowns: &SkillCooldowns,
    surge: &SurgeBuff,
    evade: &Evade,
) -> f32 {
    match skill_slot_lamp_state(slot, fury, cooldowns, surge, evade) {
        SkillSlotLampState::Ready | SkillSlotLampState::Active | SkillSlotLampState::Starved => 1.0,
        SkillSlotLampState::Charging => {
            if surge.max_charge == 0 {
                0.0
            } else {
                (surge.charge as f32 / surge.max_charge as f32).clamp(0.0, 1.0)
            }
        }
        SkillSlotLampState::Cooling => {
            let fill = skill_slot_overlay_percent(slot, fury, cooldowns, surge, evade);
            (1.0 - fill / 100.0).clamp(0.0, 1.0)
        }
    }
}

fn skill_slot_overlay_percent(
    slot: SkillSlotText,
    fury: &Fury,
    cooldowns: &SkillCooldowns,
    surge: &SurgeBuff,
    evade: &Evade,
) -> f32 {
    if skill_slot_blocked(slot, fury) {
        return 100.0;
    }
    match slot {
        SkillSlotText::Strike => timer_percent_remaining(&cooldowns.basic),
        SkillSlotText::Rupture => timer_percent_remaining(&cooldowns.rupture),
        SkillSlotText::Dash => timer_percent_remaining(&cooldowns.dash),
        SkillSlotText::Nova => timer_percent_remaining(&cooldowns.nova),
        SkillSlotText::Surge => {
            if surge.max_charge == 0 || surge.charge >= surge.max_charge {
                0.0
            } else {
                100.0 - (surge.charge as f32 / surge.max_charge as f32 * 100.0)
            }
        }
        SkillSlotText::Evade => {
            if evade.cooldown_secs <= 0.0 {
                0.0
            } else {
                (evade_cooldown_remaining(evade) / evade.cooldown_secs * 100.0).clamp(0.0, 100.0)
            }
        }
    }
}

fn timer_percent_remaining(timer: &Timer) -> f32 {
    let duration = timer.duration().as_secs_f32();
    if duration <= 0.0 || timer.is_finished() {
        0.0
    } else {
        (timer.remaining_secs() / duration * 100.0).clamp(0.0, 100.0)
    }
}

fn skill_slot_ready(
    slot: SkillSlotText,
    fury: &Fury,
    cooldowns: &SkillCooldowns,
    surge: &SurgeBuff,
    evade: &Evade,
) -> bool {
    match slot {
        SkillSlotText::Strike => cooldowns.basic.remaining_secs() <= f32::EPSILON,
        SkillSlotText::Rupture => {
            cooldowns.rupture.remaining_secs() <= f32::EPSILON && fury.current >= fury.rupture_cost
        }
        SkillSlotText::Dash => {
            cooldowns.dash.remaining_secs() <= f32::EPSILON && fury.current >= fury.dash_cost
        }
        SkillSlotText::Nova => {
            cooldowns.nova.remaining_secs() <= f32::EPSILON && fury.current >= fury.nova_cost
        }
        SkillSlotText::Surge => surge.charge >= surge.max_charge,
        SkillSlotText::Evade => evade_cooldown_remaining(evade) <= 0.0,
    }
}

fn skill_slot_blocked(slot: SkillSlotText, fury: &Fury) -> bool {
    match slot {
        SkillSlotText::Rupture => fury.current < fury.rupture_cost,
        SkillSlotText::Dash => fury.current < fury.dash_cost,
        SkillSlotText::Nova => fury.current < fury.nova_cost,
        _ => false,
    }
}

struct CharacterStatusInputs<'a> {
    level: &'a PlayerLevel,
    paragon: &'a EmberParagon,
    equipment: &'a Equipment,
    elixir_buff: &'a ElixirBuff,
    death_ward: &'a DeathWard,
    resonance: &'a str,
    set_tactic: &'a str,
    gem_resonance: &'a str,
    companion: &'a CompanionBond,
    boss: &'a str,
}

fn character_status_summary(inputs: CharacterStatusInputs) -> String {
    let paragon = compact_label(&inputs.paragon.summary().replace("Ember Paragon ", "P"), 12);
    let power_gem = compact_label(
        &format!(
            "{}{}",
            inputs.equipment.legendary_power.label(),
            gem_label(inputs.equipment.socketed_gem)
        ),
        24,
    );
    format!(
        "L{} {} | Arm {:.0} Ward {} | {} | {} | {} | {} | {} | Boss {}",
        inputs.level.level,
        paragon,
        total_armor(inputs.equipment, inputs.elixir_buff),
        inputs.death_ward.remaining_revives(),
        power_gem,
        compact_label(inputs.resonance, 10),
        compact_label(inputs.set_tactic, 20),
        compact_label(inputs.gem_resonance, 10),
        compact_label(&companion_summary(inputs.companion), 14),
        compact_label(inputs.boss, 14),
    )
}

struct SystemsStatusInputs<'a> {
    progress: &'a ChapterProgress,
    stats: &'a RunStats,
    bounty: &'a BountyProgress,
    challenges: &'a ChapterChallenges,
    obelisk: &'a BloodObelisk,
    rift: &'a EmberRift,
    modifier: &'a ChapterModifier,
    zone: &'a ChapterZoneState,
}

fn systems_status_summary(inputs: SystemsStatusInputs) -> String {
    let parts = [
        chapter_route_summary(
            inputs.progress,
            inputs.stats,
            inputs.bounty,
            inputs.obelisk,
            inputs.rift,
        ),
        chapter_zone_summary(inputs.zone),
        modifier_system_summary(inputs.modifier),
        bounty_summary(inputs.bounty),
        challenge_summary(inputs.challenges),
        obelisk_summary(inputs.obelisk),
        rift_summary(inputs.rift),
        massacre_summary(inputs.stats),
        valor_summary(inputs.stats),
        reliquary_momentum_summary(inputs.stats),
        pylon_reaper_summary(inputs.stats),
        breaker_summary(inputs.stats),
        affix_hunter_summary(inputs.stats),
        affix_codex_summary(inputs.stats),
        champion_pack_summary(inputs.stats),
        nemesis_hunter_summary(inputs.stats),
        treasure_vault_summary(inputs.stats).to_string(),
        ashen_threat_summary(inputs.stats),
        rune_weaver_summary(inputs.stats),
        armory_adept_summary(inputs.stats),
        portal_wayfarer_summary(inputs.stats).to_string(),
        glory_seeker_summary(inputs.stats),
        last_stand_summary(inputs.stats),
        cursed_pact_summary(inputs.stats),
        shrine_resonance_summary(inputs.stats).to_string(),
        ashen_alchemy_summary(inputs.stats),
        loot_curator_summary(inputs.stats),
        codex_adept_summary(inputs.stats),
        set_adept_summary(inputs.stats),
        gem_adept_summary(inputs.stats),
    ];
    compact_label(&parts.join(" | "), 112)
}

fn modifier_system_summary(modifier: &ChapterModifier) -> String {
    let tactic = ordeal_tactic(modifier);
    if tactic.is_empty() {
        format!("Ordeal {} ({})", modifier.label(), modifier.summary())
    } else {
        format!(
            "Ordeal {} ({}) - tactic {}",
            modifier.label(),
            modifier.summary(),
            tactic
        )
    }
}

struct PauseHelpInputs<'a> {
    progress: &'a ChapterProgress,
    difficulty: &'a DifficultySettings,
    modifier: &'a ChapterModifier,
    stats: &'a RunStats,
    journey: &'a str,
    clear_summary: &'a str,
    season_deeds: &'a str,
    bestiary_goal: &'a str,
    codex_goal: &'a str,
    crafting_goal: &'a str,
    mastery_goal: &'a str,
    story: &'a StoryLog,
    quest_log: &'a str,
    action_guide: &'a str,
    detailed_status: &'a str,
    systems_status: &'a str,
    inventory_open: bool,
    journal_open: bool,
    audio_status: &'a str,
}

fn pause_help_text(inputs: PauseHelpInputs) -> String {
    pause_help_lines(inputs).join("\n")
}

fn pause_help_lines(inputs: PauseHelpInputs) -> Vec<String> {
    let run_summary = format!(
        "{} {} | {} | {}",
        inputs.difficulty.current.label(),
        chapter_rating(
            inputs.difficulty.current,
            inputs.stats.elapsed_secs,
            inputs.stats.kills,
        ),
        inputs.modifier.label(),
        inputs.clear_summary,
    );
    let goals_summary = format!(
        "{} | {} | {}",
        compact_label(inputs.codex_goal, 38),
        compact_label(inputs.mastery_goal, 38),
        compact_label(inputs.crafting_goal, 38),
    );
    let collection_summary = format!(
        "{} | {} | {}",
        compact_label(inputs.season_deeds, 38),
        compact_label(inputs.bestiary_goal, 38),
        compact_label(inputs.journey, 38),
    );
    let activity_summary = format!(
        "{} | {} | {}",
        compact_label(inputs.systems_status, 62),
        compact_label(&renown_run_summary(inputs.stats), 18),
        compact_label(&stash_run_summary(inputs.stats), 52),
    );

    vec![
        "PAUSED | Esc resume | F5 save | F9 load".to_string(),
        format!(
            "Objective: {} - {} | {} | Seals {}/{}",
            inputs.progress.title,
            phase_label(inputs.progress.phase),
            compact_label(&inputs.progress.objective, 48),
            inputs.progress.seal_fragments,
            REQUIRED_SEAL_FRAGMENTS,
        ),
        "Combat: WASD move | LMB/RMB attack | Q/E skills | Shift evade | Y surge | Space interact"
            .to_string(),
        "Build: K talents | 1/2/3 invest | Z/X/C runes | B codex | [/] equip | U salvage | O/P armory"
            .to_string(),
        format!("Now: {}", compact_label(inputs.action_guide, 112)),
        format!("Quest: {}", compact_label(inputs.quest_log, 112)),
        format!("Character: {}", compact_label(inputs.detailed_status, 112)),
        format!("Run: {}", compact_label(&run_summary, 128)),
        format!("Goals: {}", goals_summary),
        format!("Collection: {}", collection_summary),
        format!("Activity: {}", activity_summary),
        format!(
            "Story: {}",
            compact_label(&story_summary(inputs.story), 112)
        ),
        format!(
            "Panels: Inventory {} | Journal {} | Audio: {} (M) | N/H sentinel | T town portal | F potion",
            open_state_label(inputs.inventory_open),
            open_state_label(inputs.journal_open),
            inputs.audio_status,
        ),
    ]
}

fn open_state_label(open: bool) -> &'static str {
    if open { "open" } else { "closed" }
}

fn skill_slot_summary(
    slot: SkillSlotText,
    fury: &Fury,
    cooldowns: &SkillCooldowns,
    runes: &SkillRunes,
    surge: &SurgeBuff,
    evade: &Evade,
) -> String {
    format!(
        "{} {}\n{}\n{}\n{}",
        skill_slot_keycap(slot),
        skill_slot_name(slot),
        skill_slot_state(slot, fury, cooldowns, surge, evade),
        skill_slot_meter_label(slot, fury, cooldowns, surge, evade),
        skill_slot_footer(slot, fury, runes, surge, evade),
    )
}

fn skill_slot_footer(
    slot: SkillSlotText,
    fury: &Fury,
    runes: &SkillRunes,
    surge: &SurgeBuff,
    evade: &Evade,
) -> String {
    format!(
        "{} {}",
        skill_slot_cost_label(slot, fury),
        skill_slot_role_short(slot, runes, surge, evade)
    )
}

fn skill_slot_cost_label(slot: SkillSlotText, fury: &Fury) -> String {
    match slot {
        SkillSlotText::Strike => format!("+{:.0}F", fury.basic_gain),
        SkillSlotText::Rupture => format!("-{:.0}F", fury.rupture_cost),
        SkillSlotText::Dash => format!("-{:.0}F", fury.dash_cost),
        SkillSlotText::Nova => format!("-{:.0}F", fury.nova_cost),
        SkillSlotText::Surge => "CHARGE".to_string(),
        SkillSlotText::Evade => "IFRAME".to_string(),
    }
}

fn skill_slot_keycap(slot: SkillSlotText) -> &'static str {
    slot.key()
}

fn skill_slot_keycap_font_size(slot: SkillSlotText) -> f32 {
    if matches!(slot, SkillSlotText::Evade) {
        7.0
    } else {
        9.0
    }
}

fn skill_slot_glyph(slot: SkillSlotText) -> &'static str {
    match slot {
        SkillSlotText::Strike => "/",
        SkillSlotText::Rupture => "X",
        SkillSlotText::Dash => ">>",
        SkillSlotText::Nova => "O",
        SkillSlotText::Surge => "!",
        SkillSlotText::Evade => "S",
    }
}

fn skill_slot_glyph_font_size(slot: SkillSlotText) -> f32 {
    match slot {
        SkillSlotText::Dash => 21.0,
        SkillSlotText::Surge => 26.0,
        _ => 24.0,
    }
}

fn skill_slot_glyph_color(slot: SkillSlotText) -> Color {
    match slot {
        SkillSlotText::Strike => Color::srgb(0.94, 0.78, 0.46),
        SkillSlotText::Rupture => Color::srgb(1.0, 0.32, 0.22),
        SkillSlotText::Dash => Color::srgb(0.54, 0.72, 1.0),
        SkillSlotText::Nova => Color::srgb(0.36, 0.82, 1.0),
        SkillSlotText::Surge => Color::srgb(1.0, 0.70, 0.22),
        SkillSlotText::Evade => Color::srgb(0.50, 1.0, 0.62),
    }
}

fn skill_slot_name(slot: SkillSlotText) -> &'static str {
    match slot {
        SkillSlotText::Strike => "STRIKE",
        SkillSlotText::Rupture => "RUPTURE",
        SkillSlotText::Dash => "DASH",
        SkillSlotText::Nova => "NOVA",
        SkillSlotText::Surge => "SURGE",
        SkillSlotText::Evade => "EVADE",
    }
}

fn skill_slot_state(
    slot: SkillSlotText,
    fury: &Fury,
    cooldowns: &SkillCooldowns,
    surge: &SurgeBuff,
    evade: &Evade,
) -> String {
    if skill_slot_blocked(slot, fury) {
        return skill_slot_blocked_label(slot, fury);
    }
    match slot {
        SkillSlotText::Strike => cooldown_state_label(cooldowns.basic.remaining_secs()),
        SkillSlotText::Rupture => cooldown_state_label(cooldowns.rupture.remaining_secs()),
        SkillSlotText::Dash => cooldown_state_label(cooldowns.dash.remaining_secs()),
        SkillSlotText::Nova => cooldown_state_label(cooldowns.nova.remaining_secs()),
        SkillSlotText::Surge => {
            let active = surge_seconds_remaining(surge);
            if active > 0.0 {
                format!("ACTIVE {active:.0}s")
            } else if surge.charge >= surge.max_charge {
                "READY".to_string()
            } else {
                format!("CHARGE {}/{}", surge.charge, surge.max_charge)
            }
        }
        SkillSlotText::Evade => {
            let active = evade_seconds_remaining(evade);
            if active > 0.0 {
                "ACTIVE".to_string()
            } else {
                cooldown_state_label(evade_cooldown_remaining(evade))
            }
        }
    }
}

fn skill_slot_blocked_label(slot: SkillSlotText, fury: &Fury) -> String {
    let missing = match slot {
        SkillSlotText::Rupture => fury.rupture_cost - fury.current,
        SkillSlotText::Dash => fury.dash_cost - fury.current,
        SkillSlotText::Nova => fury.nova_cost - fury.current,
        _ => 0.0,
    };
    format!("NEED {:.0}F", missing.max(1.0).ceil())
}

fn skill_slot_meter_label(
    slot: SkillSlotText,
    fury: &Fury,
    cooldowns: &SkillCooldowns,
    surge: &SurgeBuff,
    evade: &Evade,
) -> String {
    if let Some(required) = skill_slot_required_fury(slot, fury)
        && fury.current < required
    {
        return format!("FURY {}", skill_meter_bar(percent(fury.current, required)));
    }

    match slot {
        SkillSlotText::Strike => {
            let remaining = timer_percent_remaining(&cooldowns.basic);
            let label = if remaining <= 0.0 { "RDY" } else { "CD" };
            format!("{} {}", label, skill_meter_bar(100.0 - remaining))
        }
        SkillSlotText::Rupture => skill_cooldown_meter(&cooldowns.rupture),
        SkillSlotText::Dash => skill_cooldown_meter(&cooldowns.dash),
        SkillSlotText::Nova => skill_cooldown_meter(&cooldowns.nova),
        SkillSlotText::Surge => {
            let active = surge_seconds_remaining(surge);
            if active > 0.0 {
                format!(
                    "BUFF {}",
                    skill_meter_bar(percent(active, surge.duration_secs))
                )
            } else {
                format!(
                    "CHG {}",
                    skill_meter_bar(percent(surge.charge as f32, surge.max_charge as f32))
                )
            }
        }
        SkillSlotText::Evade => {
            let active = evade_seconds_remaining(evade);
            if active > 0.0 {
                format!(
                    "IFRM {}",
                    skill_meter_bar(percent(active, evade.active.duration().as_secs_f32()))
                )
            } else {
                let remaining = skill_slot_overlay_percent(slot, fury, cooldowns, surge, evade);
                let label = if remaining <= 0.0 { "RDY" } else { "CD" };
                format!("{} {}", label, skill_meter_bar(100.0 - remaining))
            }
        }
    }
}

fn skill_slot_required_fury(slot: SkillSlotText, fury: &Fury) -> Option<f32> {
    match slot {
        SkillSlotText::Rupture => Some(fury.rupture_cost),
        SkillSlotText::Dash => Some(fury.dash_cost),
        SkillSlotText::Nova => Some(fury.nova_cost),
        SkillSlotText::Strike | SkillSlotText::Surge | SkillSlotText::Evade => None,
    }
}

fn skill_cooldown_meter(timer: &Timer) -> String {
    let remaining = timer_percent_remaining(timer);
    let label = if remaining <= 0.0 { "RDY" } else { "CD" };
    format!("{} {}", label, skill_meter_bar(100.0 - remaining))
}

fn skill_meter_bar(percent_full: f32) -> String {
    const SEGMENTS: usize = 6;
    let filled = ((percent_full.clamp(0.0, 100.0) / 100.0) * SEGMENTS as f32)
        .round()
        .clamp(0.0, SEGMENTS as f32) as usize;
    let mut bar = String::from("[");
    for index in 0..SEGMENTS {
        bar.push(if index < filled { '#' } else { '-' });
    }
    bar.push(']');
    bar
}

fn cooldown_state_label(seconds: f32) -> String {
    if seconds <= 0.05 {
        "READY".to_string()
    } else {
        format!("CD {seconds:.1}s")
    }
}

#[cfg(test)]
fn skill_slot_tactic(
    slot: SkillSlotText,
    runes: &SkillRunes,
    surge: &SurgeBuff,
    evade: &Evade,
) -> &'static str {
    match slot {
        SkillSlotText::Strike => "FURY BUILDER",
        SkillSlotText::Rupture => match runes.rupture {
            RuptureRune::Expose => "START VULN",
            RuptureRune::Hemorrhage => "START BLEED",
        },
        SkillSlotText::Dash => match runes.dash {
            DashRune::Cleanse => "FINISH CLEANSE",
            DashRune::Reap => "FINISH REAP",
        },
        SkillSlotText::Nova => match runes.nova {
            NovaRune::Ember => "FINISH BURN",
            NovaRune::Frost => "FINISH FREEZE",
        },
        SkillSlotText::Surge => {
            if surge_seconds_remaining(surge) > 0.0 {
                "DAMAGE WINDOW"
            } else if surge.charge >= surge.max_charge {
                "BOSS BURST"
            } else {
                "KILL TO CHARGE"
            }
        }
        SkillSlotText::Evade => {
            if evade_seconds_remaining(evade) > 0.0 {
                "I-FRAME NOW"
            } else {
                "DODGE HITS"
            }
        }
    }
}

fn skill_slot_role_short(
    slot: SkillSlotText,
    runes: &SkillRunes,
    surge: &SurgeBuff,
    evade: &Evade,
) -> &'static str {
    match slot {
        SkillSlotText::Strike => "BUILD FURY",
        SkillSlotText::Rupture => match runes.rupture {
            RuptureRune::Expose => "START VULN",
            RuptureRune::Hemorrhage => "START BLEED",
        },
        SkillSlotText::Dash => match runes.dash {
            DashRune::Cleanse => "FINISH CLEANSE",
            DashRune::Reap => "FINISH REAP",
        },
        SkillSlotText::Nova => match runes.nova {
            NovaRune::Ember => "FINISH BURN",
            NovaRune::Frost => "FINISH FREEZE",
        },
        SkillSlotText::Surge => {
            if surge_seconds_remaining(surge) > 0.0 {
                "DAMAGE WINDOW"
            } else if surge.charge >= surge.max_charge {
                "BOSS BURST"
            } else {
                "KILL CHARGE"
            }
        }
        SkillSlotText::Evade => {
            if evade_seconds_remaining(evade) > 0.0 {
                "I-FRAME NOW"
            } else {
                "DODGE HIT"
            }
        }
    }
}

fn cooldown_label(seconds: f32) -> String {
    if seconds <= 0.05 {
        "ready".to_string()
    } else {
        format!("{seconds:.1}s")
    }
}

fn queue_safe_despawn(mut commands: Commands, entity: Entity) {
    // `try_despawn` is the safest API for one-shot teardown:
    // it is silent when the entity is already invalid.
    commands.entity(entity).try_despawn();
}

fn despawn_menu(mut commands: Commands, query: Query<Entity, (With<MenuUi>, Without<ChildOf>)>) {
    for entity in &query {
        queue_safe_despawn(commands.reborrow(), entity);
    }
}

fn despawn_hud(mut commands: Commands, query: Query<Entity, (With<HudUi>, Without<ChildOf>)>) {
    for entity in &query {
        queue_safe_despawn(commands.reborrow(), entity);
    }
}

fn update_minimap(
    mut commands: Commands,
    progress: Res<ChapterProgress>,
    map: Query<Entity, With<MinimapUi>>,
    dots: Query<Entity, With<MinimapDot>>,
    mut objective_text: Query<&mut Text, With<MinimapObjectiveText>>,
    queries: MinimapQueries,
) {
    let Ok(map_entity) = map.single() else {
        return;
    };
    for mut text in &mut objective_text {
        **text = minimap_objective_line(&progress);
    }
    for entity in &dots {
        queue_safe_despawn(commands.reborrow(), entity);
    }

    let (player, enemies, loot, health_globes, fury_globes, interactables) = queries;
    for transform in &loot {
        spawn_minimap_dot(
            &mut commands,
            map_entity,
            transform.translation,
            4.0,
            Color::srgb(0.95, 0.72, 0.18),
        );
    }
    for transform in &health_globes {
        spawn_minimap_dot(
            &mut commands,
            map_entity,
            transform.translation,
            4.5,
            Color::srgb(1.0, 0.08, 0.12),
        );
    }
    for transform in &fury_globes {
        spawn_minimap_dot(
            &mut commands,
            map_entity,
            transform.translation,
            4.2,
            Color::srgb(0.18, 0.58, 1.0),
        );
    }
    for (transform, interactable) in &interactables {
        if interactable.used {
            continue;
        }
        let role = minimap_interactable_role(&progress, interactable.kind);
        let (size, color) = minimap_interactable_style(role);
        spawn_minimap_dot(
            &mut commands,
            map_entity,
            transform.translation,
            size,
            color,
        );
    }
    for (transform, enemy) in &enemies {
        let (size, color) = minimap_enemy_style(enemy);
        spawn_minimap_dot(
            &mut commands,
            map_entity,
            transform.translation,
            size,
            color,
        );
    }
    if let Ok(transform) = player.single() {
        spawn_minimap_dot(
            &mut commands,
            map_entity,
            transform.translation,
            7.0,
            Color::srgb(0.20, 0.92, 0.46),
        );
    }
}

fn update_objective_compass_hud(
    progress: Res<ChapterProgress>,
    player: Query<&Transform, With<Player>>,
    enemies: Query<(&Transform, &Enemy, &Health)>,
    interactables: Query<(&Transform, &Interactable)>,
    mut panels: Query<&mut Node, With<ObjectiveCompassPanel>>,
    mut texts: Query<(&mut Text, &ObjectiveCompassText)>,
) {
    let display = player.single().ok().and_then(|transform| {
        objective_compass_display_from_targets(
            &progress,
            transform.translation,
            enemies
                .iter()
                .map(|(transform, enemy, health)| ObjectiveCompassEnemyTarget {
                    id: enemy.id.as_str(),
                    current_health: health.current,
                    position: transform.translation,
                }),
            interactables.iter().map(|(transform, interactable)| {
                ObjectiveCompassInteractableTarget {
                    kind: interactable.kind,
                    used: interactable.used,
                    reusable: interactable.reusable,
                    position: transform.translation,
                }
            }),
        )
    });

    let panel_display = if display.is_some() {
        Display::Flex
    } else {
        Display::None
    };
    for mut panel in &mut panels {
        panel.display = panel_display;
    }

    for (mut text, slot) in &mut texts {
        **text = match (display.as_ref(), slot) {
            (Some(display), ObjectiveCompassText::Direction) => display.direction.to_string(),
            (Some(display), ObjectiveCompassText::Detail) => display.detail.clone(),
            (None, _) => String::new(),
        };
    }
}

fn minimap_enemy_style(enemy: &Enemy) -> (f32, Color) {
    if enemy.id == "keeper" {
        return (9.8, Color::srgb(0.86, 0.10, 1.0));
    }
    if enemy.id == "treasure_imp" {
        return (8.0, Color::srgb(1.0, 0.82, 0.18));
    }
    if enemy.affixes.len() >= 2 {
        (7.8, Color::srgb(1.0, 0.36, 0.04))
    } else if !enemy.affixes.is_empty() {
        (6.8, Color::srgb(1.0, 0.52, 0.12))
    } else {
        (5.0, Color::srgb(0.92, 0.16, 0.10))
    }
}

fn minimap_objective_line(progress: &ChapterProgress) -> String {
    format!(
        "{} | {}",
        phase_label(progress.phase),
        compact_label(&progress.objective, 30)
    )
}

fn minimap_legend_entries() -> [(MinimapLegendRole, &'static str); 6] {
    [
        (MinimapLegendRole::Player, "You"),
        (MinimapLegendRole::Main, "Main"),
        (MinimapLegendRole::Loot, "Loot"),
        (MinimapLegendRole::Life, "Life"),
        (MinimapLegendRole::Fury, "Fury"),
        (MinimapLegendRole::Threat, "Threat"),
    ]
}

fn minimap_legend_color(role: MinimapLegendRole) -> Color {
    match role {
        MinimapLegendRole::Player => Color::srgb(0.20, 0.92, 0.46),
        MinimapLegendRole::Main => Color::srgb(0.15, 0.72, 1.0),
        MinimapLegendRole::Loot => Color::srgb(0.95, 0.72, 0.18),
        MinimapLegendRole::Life => Color::srgb(1.0, 0.08, 0.12),
        MinimapLegendRole::Fury => Color::srgb(0.18, 0.58, 1.0),
        MinimapLegendRole::Threat => Color::srgb(1.0, 0.36, 0.04),
    }
}

fn spawn_minimap_dot(
    commands: &mut Commands,
    map_entity: Entity,
    world_position: Vec3,
    size: f32,
    color: Color,
) {
    let position = minimap_position(world_position);
    let Ok(mut map_entity_commands) = commands.get_entity(map_entity) else {
        return;
    };
    map_entity_commands.with_children(|map| {
        map.spawn((
            Node {
                position_type: PositionType::Absolute,
                left: Val::Px(position.x - size * 0.5),
                top: Val::Px(position.y - size * 0.5),
                width: Val::Px(size),
                height: Val::Px(size),
                border_radius: BorderRadius::MAX,
                ..default()
            },
            BackgroundColor(color),
            MinimapDot,
        ));
    });
}

fn minimap_position(world_position: Vec3) -> Vec2 {
    let x = ((world_position.x - WORLD_MIN_X) / (WORLD_MAX_X - WORLD_MIN_X)).clamp(0.0, 1.0);
    let z = ((world_position.z - WORLD_MIN_Z) / (WORLD_MAX_Z - WORLD_MIN_Z)).clamp(0.0, 1.0);
    Vec2::new(
        MINIMAP_MAP_LEFT + x * (MINIMAP_WIDTH - MINIMAP_MAP_LEFT - MINIMAP_MAP_RIGHT),
        MINIMAP_MAP_TOP + z * (MINIMAP_HEIGHT - MINIMAP_MAP_TOP - MINIMAP_MAP_BOTTOM),
    )
}

fn is_objective_interactable(progress: &ChapterProgress, kind: InteractableKind) -> bool {
    is_primary_objective_interactable(progress, kind) || is_guided_optional_interactable(kind)
}

fn is_primary_objective_interactable(progress: &ChapterProgress, kind: InteractableKind) -> bool {
    matches!(
        (progress.phase, kind),
        (ChapterPhase::Cache, InteractableKind::Cache)
            | (ChapterPhase::Ritual, InteractableKind::Altar)
    )
}

fn is_guided_optional_interactable(kind: InteractableKind) -> bool {
    matches!(
        kind,
        InteractableKind::WeaponShrine
            | InteractableKind::FortuneShrine
            | InteractableKind::StormShrine
            | InteractableKind::AshenPylon
            | InteractableKind::HealingWell
            | InteractableKind::LorePage
            | InteractableKind::CursedShrine
            | InteractableKind::BloodObelisk
            | InteractableKind::ReliquaryVault
            | InteractableKind::EmberRift
    )
}

fn minimap_interactable_role(
    progress: &ChapterProgress,
    kind: InteractableKind,
) -> MinimapInteractableRole {
    if is_primary_objective_interactable(progress, kind) {
        return MinimapInteractableRole::Primary;
    }
    match kind {
        InteractableKind::Merchant => MinimapInteractableRole::Vendor,
        InteractableKind::HealingWell => MinimapInteractableRole::Sustain,
        InteractableKind::WeaponShrine
        | InteractableKind::FortuneShrine
        | InteractableKind::StormShrine
        | InteractableKind::AshenPylon => MinimapInteractableRole::Shrine,
        InteractableKind::CursedShrine | InteractableKind::BloodObelisk => {
            MinimapInteractableRole::Risk
        }
        InteractableKind::ReliquaryVault | InteractableKind::EmberRift => {
            MinimapInteractableRole::Reward
        }
        InteractableKind::LorePage => MinimapInteractableRole::Lore,
        _ => MinimapInteractableRole::Other,
    }
}

fn minimap_interactable_style(role: MinimapInteractableRole) -> (f32, Color) {
    match role {
        MinimapInteractableRole::Primary => (9.6, Color::srgb(0.15, 0.72, 1.0)),
        MinimapInteractableRole::Vendor => (6.8, Color::srgb(0.36, 0.90, 0.48)),
        MinimapInteractableRole::Sustain => (6.8, Color::srgb(0.95, 0.16, 0.24)),
        MinimapInteractableRole::Shrine => (6.4, Color::srgb(0.70, 0.86, 1.0)),
        MinimapInteractableRole::Risk => (7.4, Color::srgb(0.88, 0.12, 0.42)),
        MinimapInteractableRole::Reward => (7.2, Color::srgb(1.0, 0.68, 0.18)),
        MinimapInteractableRole::Lore => (5.8, Color::srgb(0.72, 0.58, 0.38)),
        MinimapInteractableRole::Other => (5.0, Color::srgb(0.46, 0.50, 0.58)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    fn test_health(current: f32, max: f32) -> Health {
        Health { current, max }
    }

    fn test_potions(current: u32, cooldown_remaining_secs: f32) -> PotionBelt {
        let mut cooldown = Timer::from_seconds(10.0, TimerMode::Once);
        cooldown.set_elapsed(Duration::from_secs_f32(
            (10.0_f32 - cooldown_remaining_secs).clamp(0.0, 10.0),
        ));
        PotionBelt {
            current,
            max: 5,
            heal_amount: 45.0,
            cooldown,
            cooldown_secs: 10.0,
        }
    }

    fn test_death_ward(revives_used: u32, max_revives: u32, gold_penalty: u32) -> DeathWard {
        DeathWard {
            checkpoint: Vec3::ZERO,
            revives_used,
            max_revives,
            gold_penalty,
        }
    }

    fn test_surge(charge: u32, active_remaining_secs: f32) -> SurgeBuff {
        let mut timer = Timer::from_seconds(8.0, TimerMode::Once);
        timer.set_elapsed(Duration::from_secs_f32(
            (8.0_f32 - active_remaining_secs).clamp(0.0, 8.0),
        ));
        SurgeBuff {
            timer,
            charge,
            max_charge: 9,
            duration_secs: 8.0,
            damage_multiplier: 1.35,
            speed_multiplier: 1.22,
            fury_multiplier: 1.45,
        }
    }

    fn test_level(level: u32) -> PlayerLevel {
        PlayerLevel {
            level,
            xp: 0,
            next_level_xp: 100,
        }
    }

    fn test_talents(points: u32, wrath: u32, vigor: u32, focus: u32) -> Talents {
        Talents {
            points,
            wrath,
            vigor,
            focus,
        }
    }

    fn test_timer(duration_secs: f32, remaining_secs: f32) -> Timer {
        let mut timer = Timer::from_seconds(duration_secs, TimerMode::Once);
        timer.set_elapsed(Duration::from_secs_f32(
            (duration_secs - remaining_secs).clamp(0.0, duration_secs),
        ));
        timer
    }

    fn test_fury(current: f32) -> Fury {
        Fury {
            current,
            max: 100.0,
            basic_gain: 18.0,
            dash_cost: 25.0,
            nova_cost: 45.0,
            rupture_cost: 32.0,
        }
    }

    fn test_cooldowns() -> SkillCooldowns {
        SkillCooldowns {
            dash: test_timer(5.0, 1.2),
            nova: test_timer(8.0, 0.0),
            rupture: test_timer(3.8, 2.4),
            basic: test_timer(0.34, 0.0),
            dash_base: 5.0,
            nova_base: 8.0,
            rupture_base: 3.8,
            basic_base: 0.34,
        }
    }

    fn test_evade(cooldown_remaining_secs: f32, active_remaining_secs: f32) -> Evade {
        Evade {
            active: test_timer(0.42, active_remaining_secs),
            cooldown: test_timer(4.5, cooldown_remaining_secs),
            cooldown_secs: 4.5,
            speed_multiplier: 2.65,
        }
    }

    fn inactive_relic_buff() -> RelicBuff {
        RelicBuff {
            timer: Timer::from_seconds(0.0, TimerMode::Once),
            damage_multiplier: 1.0,
            speed_multiplier: 1.0,
        }
    }

    fn inactive_elixir_buff() -> ElixirBuff {
        ElixirBuff {
            timer: Timer::from_seconds(0.0, TimerMode::Once),
            kind: crate::player::ElixirKind::Iron,
            armor_bonus: 0.0,
            damage_multiplier: 1.0,
            speed_multiplier: 1.0,
        }
    }

    fn inactive_fortune_buff() -> FortuneBuff {
        FortuneBuff {
            timer: Timer::from_seconds(0.0, TimerMode::Once),
            xp_multiplier: 1.25,
            gold_multiplier: 1.50,
        }
    }

    fn inactive_conduit_buff() -> ConduitBuff {
        ConduitBuff {
            timer: Timer::from_seconds(0.0, TimerMode::Once),
            pulse: Timer::from_seconds(0.65, TimerMode::Repeating),
            damage: 18.0,
            radius: 5.4,
        }
    }

    fn inactive_glory_buff() -> GloryBuff {
        GloryBuff {
            timer: Timer::from_seconds(0.0, TimerMode::Once),
            stacks: 0,
            max_stacks: 3,
            damage_per_stack: 0.06,
            speed_per_stack: 0.04,
        }
    }

    fn inactive_portal() -> TownPortal {
        TownPortal {
            cooldown: Timer::from_seconds(0.0, TimerMode::Once),
            destination: Vec3::new(-4.8, 0.0, 4.4),
            return_position: None,
        }
    }

    fn test_equipment(quality: &str) -> Equipment {
        Equipment {
            weapon_name: "Test Blade".to_string(),
            quality: quality.to_string(),
            crit_chance: 0.05,
            health_bonus: 0.0,
            armor_bonus: 0.0,
            legendary_power: crate::player::LegendaryPower::None,
            temper_level: 0,
            socketed_gem: None,
            worn: Equipment::empty_worn(),
        }
    }

    fn test_companion(stance: CompanionStance, command_cooldown: f32) -> CompanionBond {
        CompanionBond {
            stance,
            command_cooldown,
            ..default()
        }
    }

    fn story_with_beats(beats: &[crate::story::StoryBeat]) -> StoryLog {
        let mut story = StoryLog::default();
        for beat in beats {
            story.record(*beat);
        }
        story
    }

    #[test]
    fn level_up_banner_copy_surfaces_level_and_talent_points() {
        assert_eq!(level_up_banner_copy(1, 0, 1, 0), None);
        assert_eq!(
            level_up_banner_copy(2, 1, 1, 0),
            Some("LEVEL 2\n1 talent point available - K then 1/2/3".to_string())
        );
        assert_eq!(
            level_up_banner_copy(2, 2, 2, 1),
            Some("2 talent point available - K then 1/2/3".to_string())
        );
        assert_eq!(level_up_banner_copy(2, 0, 2, 1), None);
    }

    #[test]
    fn potion_pip_state_tracks_charges_cooldown_and_locked_slots() {
        let ready = test_potions(2, 0.0);
        assert_eq!(potion_pip_state(0, &ready), PotionPipState::Ready);
        assert_eq!(potion_pip_state(2, &ready), PotionPipState::Empty);
        assert_eq!(potion_pip_state(6, &ready), PotionPipState::Locked);

        let recharging = test_potions(2, 4.0);
        assert_eq!(potion_pip_state(0, &recharging), PotionPipState::Recharging);
        assert_eq!(potion_pip_state(2, &recharging), PotionPipState::Empty);
    }

    #[test]
    fn hud_objective_text_stays_compact_and_mainline_only() {
        let progress = ChapterProgress {
            phase: ChapterPhase::Ritual,
            objective: "Claim the final reliquary seal at the altar".to_string(),
            seal_fragments: 2,
            ..default()
        };
        let zone = ChapterZoneState {
            current: crate::dungeon::ChapterZone::EmberAltar,
            previous: Some(crate::dungeon::ChapterZone::ReliquaryCrossing),
        };
        let text = hud_objective_text(&progress, &zone, Vec3::ZERO, None);

        assert!(!text.contains(&progress.title));
        assert!(text.starts_with("Q73%"));
        assert!(text.contains("B>C>S>[R]>K"));
        assert!(text.contains("Altar 8m SE"));
        assert!(text.contains("Ritual"));
        assert!(text.contains("2/3"));
        assert!(!text.contains("Claim the final reliquary seal"));
        assert!(!text.contains("Action Guide"));
        assert!(!text.contains("Quest Log"));
        assert!(!text.contains("HP"));
        assert!(!text.contains("gold"));
        assert!(text.contains("Final seal +35g"));
        assert!(!text.contains("PAYOFF"));
        assert!(text.contains("Next restore final seal"));
        assert!(
            text.lines().all(|line| line.chars().count() <= 118),
            "{text}"
        );
        assert_eq!(text.lines().count(), 1);
    }

    #[test]
    fn hud_objective_payoff_tracks_next_mainline_reward() {
        let breach = ChapterProgress::default();
        let boss = ChapterProgress {
            phase: ChapterPhase::Boss,
            seal_fragments: REQUIRED_SEAL_FRAGMENTS,
            boss_spawned: true,
            ..default()
        };
        let victory = ChapterProgress {
            phase: ChapterPhase::Victory,
            seal_fragments: REQUIRED_SEAL_FRAGMENTS,
            boss_slain: true,
            ..default()
        };

        let zone = ChapterZoneState::default();

        assert!(hud_objective_text(&breach, &zone, Vec3::ZERO, None).contains("Cache seal +35g"));
        assert!(!hud_objective_text(&breach, &zone, Vec3::ZERO, None).contains("+1e"));
        assert!(hud_objective_text(&boss, &zone, Vec3::ZERO, None).contains("Soul Sigil"));
        assert!(hud_objective_text(&boss, &zone, Vec3::ZERO, None).contains("Malrec"));
        assert!(hud_objective_text(&victory, &zone, Vec3::ZERO, None).contains("Claimed"));
    }

    #[test]
    fn hud_objective_surfaces_urgent_live_priority_without_new_panel() {
        let progress = ChapterProgress {
            phase: ChapterPhase::Ritual,
            seal_fragments: 2,
            ..default()
        };
        let zone = ChapterZoneState::default();
        let rift = EmberRift {
            state: RiftState::Active,
            kills: 4,
            target_kills: crate::rift::RIFT_TARGET_KILLS,
            remaining_secs: crate::rift::RIFT_SWIFT_SEAL_SECS,
            ..default()
        };
        let priority = live_objective_priority(
            &RunStats::default(),
            &BountyProgress::default(),
            &BloodObelisk::default(),
            &rift,
        )
        .expect("active rift should surface live priority");
        let text = hud_objective_text(&progress, &zone, Vec3::ZERO, Some(&priority));

        assert!(text.contains("NOW rift 4/6 Echo"));
        assert!(text.contains("Final seal"));
        assert!(!text.contains("PAYOFF"));
        assert_eq!(text.lines().count(), 1);
        assert!(
            text.lines().all(|line| line.chars().count() <= 118),
            "{text}"
        );

        let obelisk = BloodObelisk {
            state: ObeliskState::Active,
            kills: 2,
            target_kills: crate::obelisk::OBELISK_TARGET_KILLS,
            ..default()
        };
        assert_eq!(
            live_objective_priority(
                &RunStats::default(),
                &BountyProgress::default(),
                &obelisk,
                &EmberRift::default()
            ),
            Some("NOW obelisk 2/4".to_string())
        );

        let bounty = BountyProgress {
            completed: true,
            reward_claimed: false,
            ..default()
        };
        assert_eq!(
            live_objective_priority(
                &RunStats::default(),
                &bounty,
                &BloodObelisk::default(),
                &EmberRift::default()
            ),
            Some("NOW claim bounty".to_string())
        );

        let threat = RunStats {
            ashen_threat: 90,
            ..default()
        };
        assert_eq!(
            live_objective_priority(
                &threat,
                &BountyProgress::default(),
                &BloodObelisk::default(),
                &EmberRift::default()
            ),
            Some("NOW threat surge".to_string())
        );
    }

    #[test]
    fn hud_objective_avoids_top_screen_text_wall() {
        let progress = ChapterProgress {
            phase: ChapterPhase::Cache,
            objective: "Open the ancient reliquary cache after clearing the bridge guards"
                .to_string(),
            seal_fragments: 1,
            ..default()
        };
        let zone = ChapterZoneState {
            current: crate::dungeon::ChapterZone::ReliquaryCrossing,
            previous: Some(crate::dungeon::ChapterZone::OuterSanctum),
        };

        let text = hud_objective_text(
            &progress,
            &zone,
            Vec3::new(-2.0, 0.0, -3.0),
            Some("NOW claim bounty cache before pushing deeper"),
        );
        let lines = text.lines().collect::<Vec<_>>();

        assert_eq!(lines.len(), 1, "{text}");
        assert!(lines[0].starts_with("Q20%"));
        assert!(lines[0].contains("NOW claim bounty"));
        assert!(lines[0].contains("Next open sealed cache"));
        assert!(lines[0].contains("Cache"));
        assert!(!text.contains(&progress.objective));
        assert!(!text.contains("Quest Log"));
        assert!(!text.contains("Action Guide"));
        assert!(!text.contains("Bounty Slay enemies"));
        assert!(
            lines.iter().all(|line| line.chars().count() <= 118),
            "{text}"
        );
    }

    #[test]
    fn combat_focus_hud_replaces_long_quest_copy_with_action_target_and_mainline() {
        let progress = ChapterProgress {
            phase: ChapterPhase::Boss,
            seal_fragments: REQUIRED_SEAL_FRAGMENTS,
            boss_spawned: true,
            ..default()
        };
        let health = test_health(88.0, 120.0);
        let target = TargetInfo {
            visible: true,
            name: "Malrec, Keeper of Ash".to_string(),
            health_line: "82/240 HP (34%)".to_string(),
            ..default()
        };

        assert!(hud_combat_focus_active(
            &progress,
            &health,
            &target,
            None,
            &BloodObelisk::default(),
            &EmberRift::default()
        ));

        let text = hud_combat_focus_text(
            "Action Guide: Press Y Surge for Malrec burst | Side Champion 2/4",
            &target,
            &progress,
        );

        assert!(text.starts_with("NOW Press Y Surge"));
        assert!(text.contains("Malrec, K"));
        assert!(text.contains("82/240"));
        assert!(text.contains("break and kill"));
        assert!(!text.contains("Side Champion"));
        assert!(!text.contains("Target "));
        assert!(!text.contains("Main "));
        assert!(!text.contains("HP"));
        assert!(!text.contains("QUEST"));
        assert!(text.len() <= 72, "{text}");
        assert_eq!(text.lines().count(), 1);
    }

    #[test]
    fn combat_focus_hud_activates_for_danger_and_timed_events_without_target() {
        let progress = ChapterProgress {
            phase: ChapterPhase::Ritual,
            seal_fragments: 2,
            ..default()
        };
        let low_health = test_health(30.0, 120.0);
        assert!(hud_combat_focus_active(
            &progress,
            &low_health,
            &TargetInfo::default(),
            None,
            &BloodObelisk::default(),
            &EmberRift::default()
        ));

        let rift = EmberRift {
            state: RiftState::Active,
            ..default()
        };
        assert!(hud_combat_focus_active(
            &progress,
            &test_health(100.0, 120.0),
            &TargetInfo::default(),
            None,
            &BloodObelisk::default(),
            &rift
        ));

        assert!(!hud_combat_focus_active(
            &progress,
            &test_health(100.0, 120.0),
            &TargetInfo::default(),
            None,
            &BloodObelisk::default(),
            &EmberRift::default()
        ));
    }

    #[test]
    fn combat_focus_objective_frame_recedes_into_short_tactical_prompt() {
        let exploration = objective_frame_style(false, false);
        let combat = objective_frame_style(true, false);

        assert!(combat.width < exploration.width);
        assert!(combat.min_height <= exploration.min_height);
        assert!(combat.top >= exploration.top);
        assert!(combat.font_size > exploration.font_size);
        assert!(
            combat.background_color.to_srgba().alpha
                > exploration.background_color.to_srgba().alpha
        );
        assert!(combat.width <= 430.0);
    }

    #[test]
    fn clean_hud_objective_frame_removes_top_screen_clutter() {
        let tactical = objective_frame_style(false, false);
        let clean = objective_frame_style(false, true);
        let combat_clean = objective_frame_style(true, true);

        assert!(clean.width < tactical.width);
        assert!(clean.min_height < tactical.min_height);
        assert!(
            clean.background_color.to_srgba().alpha < tactical.background_color.to_srgba().alpha
        );
        assert_eq!(combat_clean.width, objective_frame_style(true, false).width);
    }

    #[test]
    fn clean_hud_objective_text_keeps_only_actionable_route_state() {
        let progress = ChapterProgress {
            phase: ChapterPhase::Ritual,
            objective: "Claim the final reliquary seal at the altar".to_string(),
            seal_fragments: 2,
            ..default()
        };
        let zone = ChapterZoneState {
            current: crate::dungeon::ChapterZone::EmberAltar,
            previous: Some(crate::dungeon::ChapterZone::ReliquaryCrossing),
        };
        let text =
            hud_clean_objective_text(&progress, &zone, Vec3::ZERO, Some("NOW rift 4/8 Echo"));

        assert!(text.starts_with("Ritual 2/3"));
        assert!(text.contains("NOW rift"));
        assert!(text.contains("Altar"));
        assert!(!text.contains("Q73%"));
        assert!(!text.contains("Final seal"));
        assert!(!text.contains("Action Guide"));
        assert!(text.chars().count() <= 64, "{text}");
    }

    #[test]
    fn action_belt_summary_concentrates_core_arpg_state_at_bottom() {
        let health = test_health(34.0, 120.0);
        let barrier = Barrier {
            current: 18.0,
            max: 40.0,
        };
        let fury = test_fury(80.0);
        let cooldowns = SkillCooldowns {
            dash: test_timer(5.0, 0.0),
            nova: test_timer(8.0, 0.0),
            rupture: test_timer(3.8, 0.0),
            basic: test_timer(0.34, 0.0),
            dash_base: 5.0,
            nova_base: 8.0,
            rupture_base: 3.8,
            basic_base: 0.34,
        };
        let surge = test_surge(9, 0.0);
        let evade = test_evade(0.0, 0.0);
        let potions = test_potions(2, 0.0);
        let runes = SkillRunes {
            dash: DashRune::Reap,
            nova: NovaRune::Ember,
            rupture: RuptureRune::Expose,
        };
        let talents = Talents::default();
        let mut equipment = test_equipment("rare");
        equipment.weapon_name = "Iron Fang".to_string();
        let damage_bonus = DamageBonus(10.0);
        let inventory = Inventory {
            items: vec![
                InventoryItem {
                    name: "Cracked Fang".to_string(),
                    quality: "common".to_string(),
                    damage_bonus: 1.0,
                    crit_chance: 0.0,
                    health_bonus: 0.0,
                    armor_bonus: 0.0,
                    legendary_power: LegendaryPower::None,
                    temper_level: 0,
                    socketed_gem: None,
                    slot: GearSlot::Weapon,
                },
                InventoryItem {
                    name: "Stormcall Edge".to_string(),
                    quality: "legendary".to_string(),
                    damage_bonus: 18.0,
                    crit_chance: 0.12,
                    health_bonus: 0.0,
                    armor_bonus: 0.0,
                    legendary_power: LegendaryPower::Stormbrand,
                    temper_level: 0,
                    socketed_gem: None,
                    slot: GearSlot::Weapon,
                },
            ],
            capacity: 2,
        };
        let progress = ChapterProgress {
            phase: ChapterPhase::Sanctum,
            ..default()
        };
        let rift = EmberRift {
            state: RiftState::Active,
            kills: 4,
            target_kills: crate::rift::RIFT_TARGET_KILLS,
            remaining_secs: crate::rift::RIFT_SWIFT_SEAL_SECS,
            ..default()
        };
        let boon = ChapterBoon::default();

        let text = action_belt_summary(ActionBeltInputs {
            health: &health,
            barrier: &barrier,
            fury: &fury,
            cooldowns: &cooldowns,
            evade: &evade,
            surge: &surge,
            boon: &boon,
            potions: &potions,
            inventory: &inventory,
            equipment: &equipment,
            damage_bonus: &damage_bonus,
            runes: &runes,
            talents: &talents,
            stats: &RunStats::default(),
            progress: &progress,
            bounty: &BountyProgress::default(),
            obelisk: &BloodObelisk::default(),
            rift: &rift,
            combo: None,
            target_info: None,
        });

        assert!(text.starts_with("HP! 34/120 +18"));
        assert!(text.contains("Fury 80/100 spend RMB/Q/E/Y"));
        assert!(text.contains("F 2/5"));
        assert!(text.contains("Next F heal"));
        assert!(text.contains("Combo F -> Shift"));
        assert!(text.contains("Bag 2/2 FULL"));
        assert!(text.contains("0g"));
        assert!(text.contains("NOW rift 4/6 Echo"));
        assert_eq!(text.lines().count(), 1);
        assert!(text.len() <= 166, "{text}");
        assert!(!text.contains("BELT"));
        assert!(!text.contains("ward"));
    }

    #[test]
    fn clean_action_belt_keeps_bottom_skills_visible_without_text_wall() {
        let health = test_health(72.0, 120.0);
        let barrier = Barrier {
            current: 0.0,
            max: 40.0,
        };
        let fury = test_fury(64.0);
        let cooldowns = test_cooldowns();
        let evade = test_evade(0.0, 0.0);
        let surge = test_surge(5, 0.0);
        let potions = test_potions(3, 0.0);
        let inventory = Inventory {
            items: vec![InventoryItem {
                name: "Moonforged Cleaver".to_string(),
                quality: "rare".to_string(),
                damage_bonus: 9.0,
                crit_chance: 0.0,
                health_bonus: 0.0,
                armor_bonus: 0.0,
                legendary_power: LegendaryPower::None,
                temper_level: 0,
                socketed_gem: None,
                slot: GearSlot::Weapon,
            }],
            capacity: 12,
        };
        let equipment = test_equipment("common");
        let damage_bonus = DamageBonus(0.0);
        let runes = SkillRunes::default();
        let talents = Talents::default();
        let stats = RunStats {
            gold: 140,
            ..default()
        };
        let progress = ChapterProgress {
            phase: ChapterPhase::Cache,
            seal_fragments: 1,
            ..default()
        };
        let text = action_belt_clean_summary(ActionBeltInputs {
            health: &health,
            barrier: &barrier,
            fury: &fury,
            cooldowns: &cooldowns,
            evade: &evade,
            surge: &surge,
            boon: &ChapterBoon::default(),
            potions: &potions,
            inventory: &inventory,
            equipment: &equipment,
            damage_bonus: &damage_bonus,
            runes: &runes,
            talents: &talents,
            stats: &stats,
            progress: &progress,
            bounty: &BountyProgress::default(),
            obelisk: &BloodObelisk::default(),
            rift: &EmberRift::default(),
            combo: None,
            target_info: None,
        });

        assert!(text.starts_with("HP 72/120 | Fury 64/100"));
        assert!(text.contains("RMB"));
        assert!(text.contains("Q2s"));
        assert!(text.contains("E"));
        assert!(text.contains("Y5/9"));
        assert!(text.contains("F 3/5"));
        assert!(text.contains("Next [ equip"));
        assert!(text.contains("Bag 1/12"));
        assert!(text.contains("open sealed cache"));
        assert!(!text.contains("Combo"));
        assert!(!text.contains("Action Guide"));
        assert!(text.chars().count() <= 118, "{text}");
        assert_eq!(text.lines().count(), 1);
    }

    #[test]
    fn clean_action_belt_can_surface_active_combo_window_compactly() {
        let combo = action_belt_clean_combo_hint_from(Some("Rupture 3s -> Q/E".to_string()));

        assert_eq!(combo, " | Combo Rupture 3s -> Q/E");
        assert!(combo.chars().count() <= 32, "{combo}");

        let long_combo = action_belt_clean_combo_hint_from(Some(
            "Ancient Reliquary Rupture 3s -> Dash or Nova finisher".to_string(),
        ));

        assert!(
            long_combo.starts_with(" | Combo Ancient Reliquary"),
            "{long_combo}"
        );
        assert!(long_combo.chars().count() <= 32, "{long_combo}");
    }

    #[test]
    fn action_belt_next_action_prioritizes_rotation_survival_and_fury_building() {
        let health = test_health(100.0, 120.0);
        let low_health = test_health(30.0, 120.0);
        let barrier = Barrier {
            current: 0.0,
            max: 40.0,
        };
        let mut cooldowns = test_cooldowns();
        cooldowns.rupture = test_timer(3.8, 0.0);
        cooldowns.dash = test_timer(5.0, 0.0);
        let ready_surge = test_surge(9, 0.0);
        let charging_surge = test_surge(4, 0.0);
        let evade = test_evade(0.0, 0.0);
        let potions = test_potions(2, 0.0);
        let inventory = Inventory {
            items: Vec::new(),
            capacity: 12,
        };
        let equipment = test_equipment("common");
        let damage_bonus = DamageBonus(0.0);
        let runes = SkillRunes {
            dash: DashRune::Cleanse,
            nova: NovaRune::Frost,
            rupture: RuptureRune::Hemorrhage,
        };
        let talents = Talents::default();
        let stats = RunStats::default();
        let progress = ChapterProgress::default();
        let bounty = BountyProgress::default();
        let obelisk = BloodObelisk::default();
        let rift = EmberRift::default();
        let fury = test_fury(80.0);
        let boon = ChapterBoon::default();

        let base = ActionBeltInputs {
            health: &health,
            barrier: &barrier,
            fury: &fury,
            cooldowns: &cooldowns,
            evade: &evade,
            surge: &charging_surge,
            boon: &boon,
            potions: &potions,
            inventory: &inventory,
            equipment: &equipment,
            damage_bonus: &damage_bonus,
            runes: &runes,
            talents: &talents,
            stats: &stats,
            progress: &progress,
            bounty: &bounty,
            obelisk: &obelisk,
            rift: &rift,
            combo: None,
            target_info: None,
        };
        assert_eq!(action_belt_next_action(&base), "RMB bleed");
        assert_eq!(action_belt_combo_hint(&base), "RMB bleed -> E freeze");
        assert_eq!(
            action_belt_fury_state(&base),
            "Fury 80/100 spend RMB/Q/E/Y4/9"
        );

        let burst = ActionBeltInputs {
            surge: &ready_surge,
            ..base
        };
        assert_eq!(action_belt_next_action(&burst), "Y burst -> RMB/E");
        assert_eq!(action_belt_combo_hint(&burst), "Y -> RMB bleed -> E freeze");

        let danger = ActionBeltInputs {
            health: &low_health,
            ..burst
        };
        assert_eq!(action_belt_next_action(&danger), "F heal");
        assert_eq!(action_belt_combo_hint(&danger), "F -> Shift");

        let starved_fury = test_fury(8.0);
        let starved = ActionBeltInputs {
            health: &health,
            fury: &starved_fury,
            surge: &charging_surge,
            ..base
        };
        assert_eq!(action_belt_next_action(&starved), "LMB +17F");
        assert_eq!(action_belt_combo_hint(&starved), "LMB to 32F");
        assert_eq!(
            action_belt_fury_state(&starved),
            "Fury 8/100 build LMB +17F"
        );
    }

    #[test]
    fn action_belt_next_action_surfaces_inventory_pressure_out_of_combat() {
        let health = test_health(100.0, 120.0);
        let low_health = test_health(30.0, 120.0);
        let barrier = Barrier {
            current: 0.0,
            max: 40.0,
        };
        let cooldowns = test_cooldowns();
        let surge = test_surge(4, 0.0);
        let evade = test_evade(0.0, 0.0);
        let potions = test_potions(2, 0.0);
        let mut equipment = test_equipment("rare");
        equipment.weapon_name = "Iron Fang".to_string();
        equipment.legendary_power = LegendaryPower::None;
        let damage_bonus = DamageBonus(10.0);
        let runes = SkillRunes::default();
        let talents = Talents::default();
        let stats = RunStats::default();
        let progress = ChapterProgress::default();
        let bounty = BountyProgress::default();
        let obelisk = BloodObelisk::default();
        let rift = EmberRift::default();
        let fury = test_fury(80.0);
        let boon = ChapterBoon::default();
        let upgrade = InventoryItem {
            name: "Stormcall Edge".to_string(),
            quality: "legendary".to_string(),
            damage_bonus: 18.0,
            crit_chance: 0.12,
            health_bonus: 0.0,
            armor_bonus: 0.0,
            legendary_power: LegendaryPower::Stormbrand,
            temper_level: 0,
            socketed_gem: None,
            slot: GearSlot::Weapon,
        };
        let weak = InventoryItem {
            name: "Cracked Fang".to_string(),
            quality: "common".to_string(),
            damage_bonus: 1.0,
            crit_chance: 0.0,
            health_bonus: 0.0,
            armor_bonus: 0.0,
            legendary_power: LegendaryPower::None,
            temper_level: 0,
            socketed_gem: None,
            slot: GearSlot::Weapon,
        };
        let full_inventory = Inventory {
            items: vec![weak.clone(), upgrade.clone()],
            capacity: 2,
        };
        let upgrade_inventory = Inventory {
            items: vec![upgrade],
            capacity: 12,
        };
        let salvage_inventory = Inventory {
            items: vec![weak],
            capacity: 12,
        };
        let target = TargetInfo {
            visible: true,
            details: "Reflective | react: stop into reflect".to_string(),
            ..default()
        };

        let base = ActionBeltInputs {
            health: &health,
            barrier: &barrier,
            fury: &fury,
            cooldowns: &cooldowns,
            evade: &evade,
            surge: &surge,
            boon: &boon,
            potions: &potions,
            inventory: &full_inventory,
            equipment: &equipment,
            damage_bonus: &damage_bonus,
            runes: &runes,
            talents: &talents,
            stats: &stats,
            progress: &progress,
            bounty: &bounty,
            obelisk: &obelisk,
            rift: &rift,
            combo: None,
            target_info: None,
        };

        assert_eq!(action_belt_next_action(&base), "FULL [ equip Stormbrand");
        assert_eq!(
            action_belt_next_action(&ActionBeltInputs {
                inventory: &upgrade_inventory,
                ..base
            }),
            "[ equip Stormbrand"
        );
        assert_eq!(
            action_belt_next_action(&ActionBeltInputs {
                inventory: &salvage_inventory,
                ..base
            }),
            "U salvage 1"
        );
        assert_eq!(
            action_belt_next_action(&ActionBeltInputs {
                health: &low_health,
                ..base
            }),
            "F heal"
        );
        assert_eq!(
            action_belt_next_action(&ActionBeltInputs {
                target_info: Some(&target),
                ..base
            }),
            "stop reflect"
        );
    }

    #[test]
    fn action_belt_next_action_surfaces_unspent_talent_points() {
        let health = test_health(100.0, 120.0);
        let low_health = test_health(30.0, 120.0);
        let barrier = Barrier {
            current: 0.0,
            max: 40.0,
        };
        let cooldowns = test_cooldowns();
        let surge = test_surge(4, 0.0);
        let evade = test_evade(0.0, 0.0);
        let potions = test_potions(2, 0.0);
        let inventory = Inventory {
            items: Vec::new(),
            capacity: 12,
        };
        let equipment = test_equipment("common");
        let damage_bonus = DamageBonus(0.0);
        let runes = SkillRunes::default();
        let talents = test_talents(1, 2, 1, 1);
        let stats = RunStats::default();
        let progress = ChapterProgress::default();
        let bounty = BountyProgress::default();
        let obelisk = BloodObelisk::default();
        let rift = EmberRift::default();
        let fury = test_fury(80.0);
        let boon = ChapterBoon::default();
        let target = TargetInfo {
            visible: true,
            details: "Reflective | threat: reflect | react: stop into reflect".to_string(),
            ..default()
        };

        let base = ActionBeltInputs {
            health: &health,
            barrier: &barrier,
            fury: &fury,
            cooldowns: &cooldowns,
            evade: &evade,
            surge: &surge,
            boon: &boon,
            potions: &potions,
            inventory: &inventory,
            equipment: &equipment,
            damage_bonus: &damage_bonus,
            runes: &runes,
            talents: &talents,
            stats: &stats,
            progress: &progress,
            bounty: &bounty,
            obelisk: &obelisk,
            rift: &rift,
            combo: None,
            target_info: None,
        };

        assert_eq!(action_belt_next_action(&base), "K 3 Focus");
        assert_eq!(
            action_belt_next_action(&ActionBeltInputs {
                target_info: Some(&target),
                ..base
            }),
            "stop reflect"
        );
        assert_eq!(
            action_belt_next_action(&ActionBeltInputs {
                health: &low_health,
                ..base
            }),
            "F heal"
        );
    }

    #[test]
    fn action_belt_next_action_prioritizes_boss_boon_choice_before_build_management() {
        let health = test_health(100.0, 120.0);
        let low_health = test_health(30.0, 120.0);
        let barrier = Barrier {
            current: 0.0,
            max: 40.0,
        };
        let cooldowns = test_cooldowns();
        let surge = test_surge(4, 0.0);
        let evade = test_evade(0.0, 0.0);
        let potions = test_potions(2, 0.0);
        let equipment = test_equipment("common");
        let damage_bonus = DamageBonus(0.0);
        let runes = SkillRunes::default();
        let talents = test_talents(1, 2, 1, 1);
        let stats = RunStats {
            surge_kills: 2,
            ..default()
        };
        let progress = ChapterProgress {
            phase: ChapterPhase::Boss,
            ..default()
        };
        let bounty = BountyProgress::default();
        let obelisk = BloodObelisk::default();
        let rift = EmberRift::default();
        let fury = test_fury(80.0);
        let boon = ChapterBoon::default();
        let chosen_boon = ChapterBoon {
            choice: ChapterBoonChoice::ArcaneMomentum,
            empowered: true,
            momentum_stacks: 2,
        };
        let upgrade = InventoryItem {
            name: "Stormcall Edge".to_string(),
            quality: "legendary".to_string(),
            damage_bonus: 18.0,
            crit_chance: 0.12,
            health_bonus: 0.0,
            armor_bonus: 0.0,
            legendary_power: LegendaryPower::Stormbrand,
            temper_level: 0,
            socketed_gem: None,
            slot: GearSlot::Weapon,
        };
        let inventory = Inventory {
            items: vec![upgrade],
            capacity: 12,
        };
        let target = TargetInfo {
            visible: true,
            details: "Reflective | threat: reflect | react: stop into reflect".to_string(),
            ..default()
        };

        let base = ActionBeltInputs {
            health: &health,
            barrier: &barrier,
            fury: &fury,
            cooldowns: &cooldowns,
            evade: &evade,
            surge: &surge,
            boon: &boon,
            potions: &potions,
            inventory: &inventory,
            equipment: &equipment,
            damage_bonus: &damage_bonus,
            runes: &runes,
            talents: &talents,
            stats: &stats,
            progress: &progress,
            bounty: &bounty,
            obelisk: &obelisk,
            rift: &rift,
            combo: None,
            target_info: None,
        };

        assert_eq!(action_belt_next_action(&base), "4/5/6 boon rec 6 Arcane");
        assert_eq!(
            action_belt_next_action(&ActionBeltInputs {
                target_info: Some(&target),
                ..base
            }),
            "stop reflect"
        );
        assert_eq!(
            action_belt_next_action(&ActionBeltInputs {
                health: &low_health,
                ..base
            }),
            "F heal"
        );
        assert_eq!(
            action_belt_next_action(&ActionBeltInputs {
                boon: &chosen_boon,
                ..base
            }),
            "K 3 Focus"
        );
        let momentum_stats = RunStats {
            reliquary_momentum: 2,
            best_reliquary_momentum: 2,
            ..default()
        };
        assert_eq!(
            action_belt_next_action(&ActionBeltInputs {
                boon: &chosen_boon,
                stats: &momentum_stats,
                ..base
            }),
            "Momentum 2x burst"
        );
    }

    #[test]
    fn action_belt_next_action_surfaces_target_threat_counterplay() {
        let health = test_health(100.0, 120.0);
        let low_health = test_health(30.0, 120.0);
        let barrier = Barrier {
            current: 0.0,
            max: 40.0,
        };
        let cooldowns = test_cooldowns();
        let surge = test_surge(4, 0.0);
        let evade = test_evade(0.0, 0.0);
        let potions = test_potions(2, 0.0);
        let inventory = Inventory {
            items: Vec::new(),
            capacity: 12,
        };
        let equipment = test_equipment("common");
        let damage_bonus = DamageBonus(0.0);
        let runes = SkillRunes::default();
        let talents = Talents::default();
        let stats = RunStats::default();
        let progress = ChapterProgress::default();
        let bounty = BountyProgress::default();
        let obelisk = BloodObelisk::default();
        let rift = EmberRift::default();
        let fury = test_fury(80.0);
        let boon = ChapterBoon::default();
        let reflective = TargetInfo {
            visible: true,
            name: "Reflective Shielded Guard".to_string(),
            details: "Reflective / Shielded | threat: reflect, shield window | react: stop into reflect, wait shield, then burst".to_string(),
            ..default()
        };

        let base = ActionBeltInputs {
            health: &health,
            barrier: &barrier,
            fury: &fury,
            cooldowns: &cooldowns,
            evade: &evade,
            surge: &surge,
            boon: &boon,
            potions: &potions,
            inventory: &inventory,
            equipment: &equipment,
            damage_bonus: &damage_bonus,
            runes: &runes,
            talents: &talents,
            stats: &stats,
            progress: &progress,
            bounty: &bounty,
            obelisk: &obelisk,
            rift: &rift,
            combo: None,
            target_info: None,
        };

        assert_eq!(
            action_belt_next_action(&ActionBeltInputs {
                target_info: Some(&reflective),
                ..base
            }),
            "stop reflect"
        );

        let incoming = TargetInfo {
            visible: true,
            details: "Arcane / Jailer | threat: beam hazard, root trap | react: strafe beam, save Shift | incoming: strafe shot 0.2s".to_string(),
            ..default()
        };
        assert_eq!(
            action_belt_next_action(&ActionBeltInputs {
                target_info: Some(&incoming),
                ..base
            }),
            "Shift strafe shot"
        );

        let ground = TargetInfo {
            visible: true,
            details: "Molten / Desecrator | threat: death pool, ground fire | react: step off death pool, move out".to_string(),
            ..default()
        };
        assert_eq!(
            action_belt_next_action(&ActionBeltInputs {
                target_info: Some(&ground),
                ..base
            }),
            "Shift move out"
        );

        let chaser = TargetInfo {
            visible: true,
            details: "no affixes | role: chaser | tip: kite rush, punish whiffs".to_string(),
            ..default()
        };
        assert_eq!(
            action_belt_next_action(&ActionBeltInputs {
                target_info: Some(&chaser),
                ..base
            }),
            "Shift kite chaser"
        );

        let marksman = TargetInfo {
            visible: true,
            details: "no affixes | role: marksman | tip: close gap, sidestep bolts".to_string(),
            ..default()
        };
        assert_eq!(
            action_belt_next_action(&ActionBeltInputs {
                target_info: Some(&marksman),
                ..base
            }),
            "close marksman"
        );

        let heavy = TargetInfo {
            visible: true,
            details: "no affixes | role: heavy | tip: bait shockwave, punish slam".to_string(),
            ..default()
        };
        assert_eq!(
            action_belt_next_action(&ActionBeltInputs {
                target_info: Some(&heavy),
                ..base
            }),
            "Shift dodge heavy"
        );

        let guard = TargetInfo {
            visible: true,
            details: "no affixes | role: guard | tip: flank shield guard".to_string(),
            ..default()
        };
        assert_eq!(
            action_belt_next_action(&ActionBeltInputs {
                target_info: Some(&guard),
                ..base
            }),
            "flank guard"
        );

        let caster = TargetInfo {
            visible: true,
            details: "no affixes | role: caster | tip: dash through fire".to_string(),
            ..default()
        };
        assert_eq!(
            action_belt_next_action(&ActionBeltInputs {
                target_info: Some(&caster),
                ..base
            }),
            "Shift dash fire"
        );

        let boss = TargetInfo {
            visible: true,
            details: "no affixes | role: boss | tip: break stagger, deny enrage".to_string(),
            ..default()
        };
        assert_eq!(
            action_belt_next_action(&ActionBeltInputs {
                target_info: Some(&boss),
                ..base
            }),
            "RMB/Q stagger"
        );

        let warden = TargetInfo {
            visible: true,
            details: "no affixes | role: warden | tip: break ward, sidestep runes".to_string(),
            ..default()
        };
        assert_eq!(
            action_belt_next_action(&ActionBeltInputs {
                target_info: Some(&warden),
                ..base
            }),
            "Shift break warden"
        );

        let broken_warden = TargetInfo {
            visible: true,
            details: "no affixes | role: warden | ward broken: punish".to_string(),
            ..default()
        };
        assert_eq!(
            action_belt_next_action(&ActionBeltInputs {
                target_info: Some(&broken_warden),
                ..base
            }),
            "RMB/E punish"
        );

        let rune = TargetInfo {
            visible: true,
            details: "no affixes | role: warden | incoming: leave seal rune 0.2s".to_string(),
            ..default()
        };
        assert_eq!(
            action_belt_next_action(&ActionBeltInputs {
                target_info: Some(&rune),
                ..base
            }),
            "Shift leave seal rune"
        );

        let execute = TargetInfo {
            visible: true,
            details: "boss | window: burst now".to_string(),
            ..default()
        };
        assert_eq!(
            action_belt_next_action(&ActionBeltInputs {
                target_info: Some(&execute),
                ..base
            }),
            "Y burst window"
        );

        let danger = ActionBeltInputs {
            health: &low_health,
            target_info: Some(&execute),
            ..base
        };
        assert_eq!(action_belt_next_action(&danger), "F heal");
    }

    #[test]
    fn chapter_navigation_hint_points_to_active_mainline_sites() {
        let zone = ChapterZoneState {
            current: crate::dungeon::ChapterZone::ReliquaryCrossing,
            previous: None,
        };
        let cache = ChapterProgress {
            phase: ChapterPhase::Cache,
            chest_opened: false,
            ..default()
        };
        let altar = ChapterProgress {
            phase: ChapterPhase::Ritual,
            altar_activated: false,
            ..default()
        };
        let sanctum = ChapterProgress {
            phase: ChapterPhase::Sanctum,
            ..default()
        };

        assert_eq!(
            chapter_navigation_hint(&cache, &zone, Vec3::ZERO),
            "Cache 10m NW"
        );
        assert_eq!(
            chapter_navigation_hint(&altar, &zone, Vec3::new(6.5, 0.0, 4.8)),
            "Altar 0m here"
        );
        assert_eq!(
            chapter_navigation_hint(&sanctum, &zone, Vec3::ZERO),
            "rotate between main and opti..."
        );
    }

    #[test]
    fn objective_compass_display_tracks_mainline_target_without_long_text() {
        let cache = ChapterProgress {
            phase: ChapterPhase::Cache,
            chest_opened: false,
            ..default()
        };
        let altar = ChapterProgress {
            phase: ChapterPhase::Ritual,
            altar_activated: false,
            ..default()
        };
        let cleared = ChapterProgress {
            phase: ChapterPhase::Boss,
            boss_slain: true,
            ..default()
        };

        let cache_display = objective_compass_display(&cache, Vec3::ZERO).unwrap();
        assert_eq!(cache_display.direction, "NW");
        assert_eq!(cache_display.detail, "MAIN 10m Cache");
        assert!(cache_display.detail.len() <= 16);

        let altar_display = objective_compass_display(&altar, Vec3::new(6.5, 0.0, 4.8)).unwrap();
        assert_eq!(altar_display.direction, "HERE");
        assert_eq!(altar_display.detail, "MAIN 0m Altar");

        assert!(objective_compass_display(&cleared, Vec3::ZERO).is_none());
    }

    #[test]
    fn objective_compass_prefers_live_mainline_enemies() {
        let sanctum = ChapterProgress {
            phase: ChapterPhase::Sanctum,
            ..default()
        };
        let boss = ChapterProgress {
            phase: ChapterPhase::Boss,
            boss_slain: false,
            ..default()
        };

        let warden_display = objective_compass_display_from_targets(
            &sanctum,
            Vec3::ZERO,
            [
                ObjectiveCompassEnemyTarget {
                    id: "seal_warden",
                    current_health: 25.0,
                    position: Vec3::new(-2.0, 0.0, 3.0),
                },
                ObjectiveCompassEnemyTarget {
                    id: "keeper",
                    current_health: 180.0,
                    position: Vec3::new(8.0, 0.0, 4.0),
                },
            ],
            [ObjectiveCompassInteractableTarget {
                kind: InteractableKind::EmberRift,
                used: false,
                reusable: false,
                position: Vec3::new(1.0, 0.0, 0.0),
            }],
        )
        .unwrap();
        assert_eq!(warden_display.direction, "SW");
        assert_eq!(warden_display.detail, "MAIN 4m Warden");

        let boss_display = objective_compass_display_from_targets(
            &boss,
            Vec3::ZERO,
            [ObjectiveCompassEnemyTarget {
                id: "keeper",
                current_health: 200.0,
                position: Vec3::new(2.0, 0.0, 0.0),
            }],
            std::iter::empty(),
        )
        .unwrap();
        assert_eq!(boss_display.direction, "E");
        assert_eq!(boss_display.detail, "MAIN 2m Malrec");
    }

    #[test]
    fn objective_compass_falls_back_to_high_value_side_targets() {
        let sanctum = ChapterProgress {
            phase: ChapterPhase::Sanctum,
            ..default()
        };

        let side_display = objective_compass_display_from_targets(
            &sanctum,
            Vec3::ZERO,
            [ObjectiveCompassEnemyTarget {
                id: "seal_warden",
                current_health: 0.0,
                position: Vec3::new(-2.0, 0.0, 3.0),
            }],
            [
                ObjectiveCompassInteractableTarget {
                    kind: InteractableKind::Merchant,
                    used: false,
                    reusable: true,
                    position: Vec3::new(1.0, 0.0, 0.0),
                },
                ObjectiveCompassInteractableTarget {
                    kind: InteractableKind::ReliquaryVault,
                    used: false,
                    reusable: false,
                    position: Vec3::new(2.0, 0.0, 0.0),
                },
                ObjectiveCompassInteractableTarget {
                    kind: InteractableKind::EmberRift,
                    used: false,
                    reusable: false,
                    position: Vec3::new(10.0, 0.0, 0.0),
                },
            ],
        )
        .unwrap();
        assert_eq!(side_display.direction, "E");
        assert_eq!(side_display.detail, "SIDE 10m Rift");

        let used_rift_display = objective_compass_display_from_targets(
            &sanctum,
            Vec3::ZERO,
            std::iter::empty(),
            [
                ObjectiveCompassInteractableTarget {
                    kind: InteractableKind::ReliquaryVault,
                    used: false,
                    reusable: false,
                    position: Vec3::new(2.0, 0.0, 0.0),
                },
                ObjectiveCompassInteractableTarget {
                    kind: InteractableKind::EmberRift,
                    used: true,
                    reusable: false,
                    position: Vec3::new(1.0, 0.0, 0.0),
                },
            ],
        )
        .unwrap();
        assert_eq!(used_rift_display.detail, "SIDE 2m Vault");
    }

    #[test]
    fn combat_log_panel_hides_when_empty_and_caps_visible_lines() {
        let (empty_text, empty_display) = combat_log_panel_text([], false);
        assert_eq!(empty_text, "");
        assert_eq!(empty_display, Display::None);

        let (text, display) = combat_log_panel_text(["hit", "loot", "third"], false);
        assert_eq!(text, "hit");
        assert_eq!(display, Display::Flex);

        let (focused_text, focused_display) = combat_log_panel_text(["hit"], true);
        assert_eq!(focused_text, "");
        assert_eq!(focused_display, Display::None);
    }

    #[test]
    fn journal_panel_stays_hidden_during_live_combat() {
        let (closed_text, closed_display) =
            journal_panel_text(false, false, "Journal full", "Pause summary");
        assert_eq!(closed_text, "");
        assert_eq!(closed_display, Display::None);

        let (open_text, open_display) = journal_panel_text(true, false, "Journal full", "Pause");
        assert_eq!(open_text, "Journal full");
        assert_eq!(open_display, Display::Flex);

        let (pause_text, pause_display) =
            journal_panel_text(false, true, "Journal full", "Pause summary");
        assert_eq!(pause_text, "Pause summary");
        assert_eq!(pause_display, Display::Flex);
    }

    #[test]
    fn resource_orbs_surface_core_arpg_resources() {
        let health = test_health(72.0, 120.0);
        let barrier = Barrier {
            current: 18.0,
            max: 40.0,
        };
        let potions = test_potions(3, 0.0);
        let ward = test_death_ward(1, 2, 40);
        let fury = test_fury(64.0);
        let level = PlayerLevel {
            level: 4,
            xp: 88,
            next_level_xp: 140,
        };
        let paragon = EmberParagon::default();
        let stats = RunStats {
            gold: 275,
            ember_shards: 12,
            affix_essence: 3,
            ..default()
        };
        let readiness = combat_readiness_summary(CombatReadinessInputs {
            health: &health,
            potions: &potions,
            fury: &fury,
            cooldowns: &test_cooldowns(),
            surge: &test_surge(9, 0.0),
            evade: &test_evade(0.0, 0.0),
            runes: &SkillRunes::default(),
        });

        let life = health_orb_text(&health, &barrier, &potions, &ward);
        assert!(life.contains("LIFE STABLE"));
        assert!(life.contains("72 / 120"));
        assert!(life.contains("WARD 18 | SOUL 1"));
        assert!(life.contains("F POTION 3/5 READY"));

        let resource = fury_orb_text(&fury, &level, &paragon, &stats, &readiness);
        assert!(resource.contains("FURY"));
        assert!(resource.contains("64 / 100"));
        assert!(resource.contains("SPEND RMB32 Q25 E45"));
        assert!(resource.contains("BUILD LMB +18F"));
        assert!(resource.contains("LV 4"));
        assert!(resource.contains("XP 88/140"));
        assert!(resource.contains("Y Surge burst"));
        assert!(resource.contains("275g 12s 3e"));
    }

    #[test]
    fn resource_orb_frames_make_low_life_barrier_and_burst_state_readable() {
        let safe = test_health(100.0, 120.0);
        let wounded = test_health(54.0, 120.0);
        let critical = test_health(24.0, 120.0);
        let no_barrier = Barrier {
            current: 0.0,
            max: 40.0,
        };
        let guarded = Barrier {
            current: 18.0,
            max: 40.0,
        };
        let full_fury = test_fury(80.0);
        let starved_fury = test_fury(8.0);
        let charging_surge = test_surge(4, 0.0);
        let ready_surge = test_surge(9, 0.0);
        let active_surge = test_surge(9, 4.5);

        let safe_life = resource_orb_frame_style(
            ResourceOrbFrame::Health,
            &safe,
            &no_barrier,
            &full_fury,
            &charging_surge,
        );
        let wounded_life = resource_orb_frame_style(
            ResourceOrbFrame::Health,
            &wounded,
            &no_barrier,
            &full_fury,
            &charging_surge,
        );
        let critical_life = resource_orb_frame_style(
            ResourceOrbFrame::Health,
            &critical,
            &no_barrier,
            &full_fury,
            &charging_surge,
        );
        let guarded_life = resource_orb_frame_style(
            ResourceOrbFrame::Health,
            &wounded,
            &guarded,
            &full_fury,
            &charging_surge,
        );

        assert!(critical_life.border.to_srgba().red >= safe_life.border.to_srgba().red);
        assert!(critical_life.border.to_srgba().green < wounded_life.border.to_srgba().green);
        assert!(guarded_life.border.to_srgba().green > wounded_life.border.to_srgba().green);

        let starved_resource = resource_orb_frame_style(
            ResourceOrbFrame::Fury,
            &safe,
            &no_barrier,
            &starved_fury,
            &charging_surge,
        );
        let spend_resource = resource_orb_frame_style(
            ResourceOrbFrame::Fury,
            &safe,
            &no_barrier,
            &full_fury,
            &charging_surge,
        );
        let ready_resource = resource_orb_frame_style(
            ResourceOrbFrame::Fury,
            &safe,
            &no_barrier,
            &full_fury,
            &ready_surge,
        );
        let active_resource = resource_orb_frame_style(
            ResourceOrbFrame::Fury,
            &safe,
            &no_barrier,
            &full_fury,
            &active_surge,
        );

        assert!(starved_resource.border.to_srgba().red > spend_resource.border.to_srgba().red);
        assert!(spend_resource.border.to_srgba().blue > starved_resource.border.to_srgba().blue);
        assert!(ready_resource.border.to_srgba().red > spend_resource.border.to_srgba().red);
        assert!(active_resource.border.to_srgba().green > ready_resource.border.to_srgba().green);
        assert!(
            active_resource.background.to_srgba().green
                > spend_resource.background.to_srgba().green
        );
    }

    #[test]
    fn health_orb_surfaces_wound_state_and_soul_ward_charges() {
        let barrier = Barrier {
            current: 0.0,
            max: 40.0,
        };
        let potions = test_potions(1, 4.2);

        let wounded = health_orb_text(
            &test_health(50.0, 100.0),
            &barrier,
            &potions,
            &test_death_ward(0, 2, 40),
        );
        assert!(wounded.contains("LIFE WOUNDED"));
        assert!(wounded.contains("SOUL 2"));
        assert!(wounded.contains("F POTION 1/5 4.2S"));

        let critical = health_orb_text(
            &test_health(20.0, 100.0),
            &barrier,
            &test_potions(0, 0.0),
            &test_death_ward(2, 2, 40),
        );
        assert!(critical.contains("LIFE CRITICAL"));
        assert!(critical.contains("SOUL 0"));
        assert!(critical.contains("F POTION 0/5 READY"));
        assert_eq!(health_state_label(&test_health(1.0, 0.0)), "UNKNOWN");
    }

    #[test]
    fn low_health_vignette_escalates_screen_pressure_without_covering_safe_play() {
        let safe = low_health_vignette_style(
            &test_health(80.0, 100.0),
            &Barrier {
                current: 0.0,
                max: 40.0,
            },
        );
        let wounded_guarded = low_health_vignette_style(
            &test_health(45.0, 100.0),
            &Barrier {
                current: 20.0,
                max: 40.0,
            },
        );
        let critical_exposed = low_health_vignette_style(
            &test_health(16.0, 100.0),
            &Barrier {
                current: 0.0,
                max: 40.0,
            },
        );

        assert_eq!(safe.visibility, Visibility::Hidden);
        assert_eq!(wounded_guarded.visibility, Visibility::Visible);
        assert_eq!(critical_exposed.visibility, Visibility::Visible);
        assert!(wounded_guarded.color.to_srgba().alpha > 0.03);
        assert!(
            critical_exposed.color.to_srgba().alpha > wounded_guarded.color.to_srgba().alpha + 0.04
        );
        assert!(critical_exposed.color.to_srgba().red > wounded_guarded.color.to_srgba().red);
        assert!(critical_exposed.color.to_srgba().alpha <= 0.22);
    }

    #[test]
    fn resource_orb_switches_to_paragon_growth_at_chapter_cap() {
        let fury = test_fury(82.0);
        let level = PlayerLevel {
            level: CHAPTER_LEVEL_CAP,
            xp: 0,
            next_level_xp: 999,
        };
        let paragon = EmberParagon {
            rank: 3,
            xp: 42,
            next_rank_xp: 210,
        };
        let stats = RunStats {
            gold: 60,
            ember_shards: 8,
            affix_essence: 2,
            ..default()
        };

        let resource = fury_orb_text(&fury, &level, &paragon, &stats, "NEXT E Nova");

        assert!(resource.contains("FURY"));
        assert!(resource.contains("82 / 100"));
        assert!(resource.contains("SPEND RMB32 Q25 E45"));
        assert!(resource.contains("BUILD LMB +18F"));
        assert!(resource.contains("PARA 3"));
        assert!(resource.contains("XP 42/210"));
        assert!(resource.contains("NEXT E Nova"));
        assert!(!resource.contains("LV 6"));
    }

    #[test]
    fn fury_orb_names_skill_costs_and_builder_gain() {
        let fury = Fury {
            current: 12.0,
            max: 100.0,
            basic_gain: 21.0,
            dash_cost: 24.0,
            nova_cost: 42.0,
            rupture_cost: 30.0,
        };

        assert_eq!(fury_spend_line(&fury), "SPEND RMB30 Q24 E42");
        assert_eq!(fury_builder_line(&fury), "BUILD LMB +21F");
    }

    #[test]
    fn combat_readiness_summary_guides_survival_burst_and_fury_building() {
        let cooldowns = test_cooldowns();
        let ready_evade = test_evade(0.0, 0.0);

        let low_health = combat_readiness_summary(CombatReadinessInputs {
            health: &test_health(22.0, 100.0),
            potions: &test_potions(1, 0.0),
            fury: &test_fury(80.0),
            cooldowns: &cooldowns,
            surge: &test_surge(9, 0.0),
            evade: &ready_evade,
            runes: &SkillRunes::default(),
        });
        assert_eq!(low_health, "F Potion heal");

        let low_health_no_potion = combat_readiness_summary(CombatReadinessInputs {
            health: &test_health(22.0, 100.0),
            potions: &test_potions(0, 0.0),
            fury: &test_fury(80.0),
            cooldowns: &cooldowns,
            surge: &test_surge(9, 0.0),
            evade: &ready_evade,
            runes: &SkillRunes::default(),
        });
        assert_eq!(low_health_no_potion, "Shift Evade");

        let burst = combat_readiness_summary(CombatReadinessInputs {
            health: &test_health(90.0, 100.0),
            potions: &test_potions(1, 0.0),
            fury: &test_fury(30.0),
            cooldowns: &cooldowns,
            surge: &test_surge(9, 0.0),
            evade: &ready_evade,
            runes: &SkillRunes::default(),
        });
        assert_eq!(burst, "Y Surge burst");

        let build = combat_readiness_summary(CombatReadinessInputs {
            health: &test_health(90.0, 100.0),
            potions: &test_potions(1, 0.0),
            fury: &test_fury(12.0),
            cooldowns: &cooldowns,
            surge: &test_surge(0, 0.0),
            evade: &ready_evade,
            runes: &SkillRunes::default(),
        });
        assert_eq!(build, "LMB +33F");

        let mut rupture_ready = test_cooldowns();
        rupture_ready.rupture.tick(rupture_ready.rupture.duration());
        let runes = SkillRunes {
            rupture: RuptureRune::Expose,
            nova: NovaRune::Frost,
            dash: DashRune::Reap,
        };
        let setup = combat_readiness_summary(CombatReadinessInputs {
            health: &test_health(90.0, 100.0),
            potions: &test_potions(1, 0.0),
            fury: &test_fury(80.0),
            cooldowns: &rupture_ready,
            surge: &test_surge(0, 0.0),
            evade: &ready_evade,
            runes: &runes,
        });
        assert_eq!(setup, "RMB start vuln");

        let mut nova_ready = test_cooldowns();
        nova_ready.nova.tick(nova_ready.nova.duration());
        let finish = combat_readiness_summary(CombatReadinessInputs {
            health: &test_health(90.0, 100.0),
            potions: &test_potions(1, 0.0),
            fury: &test_fury(80.0),
            cooldowns: &nova_ready,
            surge: &test_surge(0, 0.0),
            evade: &ready_evade,
            runes: &runes,
        });
        assert_eq!(finish, "E Nova freeze");
    }

    #[test]
    fn build_panel_guides_talents_runes_rotation_and_burst_windows() {
        let health = test_health(88.0, 130.0);
        let fury = test_fury(84.0);
        let cooldowns = test_cooldowns();
        let evade = test_evade(0.0, 0.0);
        let surge = test_surge(9, 0.0);
        let level = PlayerLevel {
            level: 5,
            xp: 120,
            next_level_xp: 180,
        };
        let mut equipment = test_equipment("legendary");
        equipment.weapon_name = "Stormcall Reliquary Brand of the Long Night".to_string();
        equipment.legendary_power = LegendaryPower::Stormbrand;
        equipment.armor_bonus = 24.0;
        let charm = Charm {
            name: "Stormglass Charm".to_string(),
            quality: "legendary".to_string(),
            damage_bonus: 5.0,
            crit_chance: 0.035,
        };
        let talents = Talents {
            points: 1,
            wrath: 1,
            vigor: 2,
            focus: 0,
        };
        let runes = SkillRunes {
            dash: DashRune::Reap,
            nova: NovaRune::Frost,
            rupture: RuptureRune::Hemorrhage,
        };
        let mut codex = LegendaryCodex::default();
        codex.unlock(LegendaryPower::Stormbrand);

        let text = build_panel_text(BuildPanelInputs {
            health: &health,
            fury: &fury,
            cooldowns: &cooldowns,
            evade: &evade,
            surge: &surge,
            level: &level,
            damage_bonus: &DamageBonus(14.0),
            equipment: &equipment,
            charm: &charm,
            talents: &talents,
            runes: &runes,
            codex: &codex,
        });

        assert!(text.contains("BUILD | K close"));
        assert!(text.contains("I bag | J journal"));
        assert!(text.contains("Skills LMB/RMB/Q/E/Y/Shift"));
        assert!(text.contains("Bag [/] U"));
        assert!(text.contains("Archetype Storm Reaper balanced | rune chains"));
        assert!(text.contains("Next K then 3 Focus"));
        assert!(text.contains("DPS "));
        assert!(text.contains("Tough "));
        assert!(text.contains("Ctrl "));
        assert!(text.contains("spend 3 Focus for cooldowns"));
        assert!(text.contains("freeze packs, bleed them, reap executes"));
        assert!(text.contains("Combo set: shock"));
        assert!(text.contains("Y Surge before elite/Boss burst"));
        assert!(text.contains("Surge Y 9/9 READY"));
        assert!(text.contains("Evade Shift READY"));
        assert!(text.contains("Codex 1/5"));
        assert!(text.contains("Stormcall Reliquary"));
        assert!(text.contains("Mastery Wrath"));
        assert!(text.lines().count() <= 14);
        assert!(text.lines().all(|line| line.len() <= 118));
    }

    #[test]
    fn build_archetype_summary_names_role_score_and_payoff() {
        let mut equipment = test_equipment("legendary");
        equipment.weapon_name = "Soulreaver Reliquary Fang".to_string();
        equipment.legendary_power = LegendaryPower::Soulreaver;
        equipment.health_bonus = 44.0;
        equipment.armor_bonus = 34.0;
        let charm = Charm {
            name: "Bloodglass Charm".to_string(),
            quality: "legendary".to_string(),
            damage_bonus: 6.0,
            crit_chance: 0.04,
        };
        let talents = Talents {
            wrath: 1,
            vigor: 2,
            focus: 2,
            points: 0,
        };
        let runes = SkillRunes {
            dash: DashRune::Reap,
            nova: NovaRune::Frost,
            rupture: RuptureRune::Hemorrhage,
        };

        let archetype =
            build_archetype_summary(&DamageBonus(18.0), &equipment, &charm, &talents, &runes);
        let score = build_score_summary(&DamageBonus(18.0), &equipment, &charm, &talents, &runes);

        assert_eq!(archetype, "Blood Reaver tank | rune leech");
        assert!(score.starts_with("DPS "));
        assert!(score.contains(" | Tough "));
        assert!(score.contains(" | Ctrl "));
        assert!(build_dps_score(&DamageBonus(18.0), &equipment, &charm, &talents) > 80);
        assert!(build_toughness_score(&equipment, &charm, &talents) > 90);
        assert!(build_control_score(&equipment, &runes, &talents) >= 60);
    }

    #[test]
    fn legendary_rune_synergy_payoff_labels_match_combat_proc_routes() {
        assert_eq!(
            legendary_rune_synergy_payoff_label(
                LegendaryPower::Emberbrand,
                &SkillRunes {
                    dash: DashRune::Cleanse,
                    nova: NovaRune::Ember,
                    rupture: RuptureRune::Expose,
                }
            ),
            Some("rune ignites")
        );
        assert_eq!(
            legendary_rune_synergy_payoff_label(
                LegendaryPower::Frostbrand,
                &SkillRunes {
                    dash: DashRune::Cleanse,
                    nova: NovaRune::Frost,
                    rupture: RuptureRune::Hemorrhage,
                }
            ),
            Some("rune chills")
        );
        assert_eq!(
            legendary_rune_synergy_payoff_label(
                LegendaryPower::Stormbrand,
                &SkillRunes {
                    dash: DashRune::Reap,
                    nova: NovaRune::Ember,
                    rupture: RuptureRune::Expose,
                }
            ),
            Some("rune chains")
        );
        assert_eq!(
            legendary_rune_synergy_payoff_label(
                LegendaryPower::Soulreaver,
                &SkillRunes {
                    dash: DashRune::Cleanse,
                    nova: NovaRune::Ember,
                    rupture: RuptureRune::Hemorrhage,
                }
            ),
            Some("rune leech")
        );
        assert_eq!(
            legendary_rune_synergy_payoff_label(
                LegendaryPower::Aegisbrand,
                &SkillRunes {
                    dash: DashRune::Cleanse,
                    nova: NovaRune::Ember,
                    rupture: RuptureRune::Hemorrhage,
                }
            ),
            Some("rune wards")
        );
        assert_eq!(
            legendary_rune_synergy_payoff_label(
                LegendaryPower::Stormbrand,
                &SkillRunes {
                    dash: DashRune::Cleanse,
                    nova: NovaRune::Ember,
                    rupture: RuptureRune::Expose,
                }
            ),
            None
        );
    }

    #[test]
    fn build_rotation_summary_prioritizes_builder_setup_and_control() {
        let runes = SkillRunes {
            dash: DashRune::Cleanse,
            nova: NovaRune::Frost,
            rupture: RuptureRune::Expose,
        };
        let starved = test_fury(6.0);
        let ready = test_fury(90.0);
        let cooldowns = test_cooldowns();
        let charging_surge = test_surge(4, 0.0);
        let active_surge = test_surge(9, 3.0);

        assert_eq!(
            build_rotation_summary(&starved, &cooldowns, &charging_surge, &runes),
            "LMB Strike to build Fury"
        );
        assert_eq!(
            build_rotation_summary(&ready, &cooldowns, &active_surge, &runes),
            "Surge active: spend Fury and chain kills"
        );

        let mut ready_cooldowns = test_cooldowns();
        ready_cooldowns
            .rupture
            .tick(ready_cooldowns.rupture.duration());
        assert_eq!(
            build_rotation_summary(&ready, &ready_cooldowns, &charging_surge, &runes),
            "RMB Expose starts combo; Q/E finish"
        );
    }

    #[test]
    fn set_combo_payoff_summary_tracks_themed_weapon_and_charm() {
        let mut equipment = test_equipment("legendary");
        let mut charm = Charm {
            name: "Empty Charm Slot".to_string(),
            quality: "none".to_string(),
            damage_bonus: 0.0,
            crit_chance: 0.0,
        };
        assert_eq!(
            set_combo_payoff_summary(&equipment, &charm),
            "Combo set: match theme"
        );

        equipment.weapon_name = "Soulreaver Reliquary Fang".to_string();
        charm.name = "Bloodglass Charm".to_string();
        charm.quality = "legendary".to_string();
        assert_eq!(
            set_combo_payoff_summary(&equipment, &charm),
            "Combo set: heal"
        );

        equipment.weapon_name = "Ashen Reliquary Reaver".to_string();
        charm.name = "Ashen Saint Charm".to_string();
        assert_eq!(
            set_combo_payoff_summary(&equipment, &charm),
            "Combo set: barrier"
        );
    }

    #[test]
    fn target_info_hud_summarizes_target_and_colors_threats() {
        let normal = TargetInfo {
            visible: true,
            name: "Ashbone Guard".to_string(),
            subtitle: "Melee".to_string(),
            health_line: "80/100 HP  (80%)".to_string(),
            details: "no affixes".to_string(),
            health_percent: 80.0,
            threat_color: Color::srgb(0.86, 0.82, 0.72),
        };
        let elite = TargetInfo {
            visible: true,
            name: "Molten Guard".to_string(),
            subtitle: "ELITE - Melee".to_string(),
            health_line: "20/100 HP  (20%)".to_string(),
            details: "Molten / Shielded".to_string(),
            health_percent: 20.0,
            threat_color: Color::srgb(0.82, 0.38, 1.0),
        };

        let normal_summary = target_info_summary(&normal);
        assert!(normal_summary.contains("Ashbone Guard"));
        assert!(normal_summary.contains("Melee | 80/100 HP"));
        assert!(normal_summary.contains("no affixes"));
        assert_ne!(
            target_info_health_color(&normal),
            target_info_health_color(&elite)
        );
        assert!(target_info_summary(&elite).contains("Molten / Shielded"));
    }

    #[test]
    fn inventory_slots_surface_equipped_upgrade_and_stash_decisions() {
        let equipment = test_equipment("rare");
        let damage_bonus = DamageBonus(10.0);
        let equipped = InventoryItem {
            name: "Test Blade".to_string(),
            quality: "rare".to_string(),
            damage_bonus: 10.0,
            crit_chance: 0.05,
            health_bonus: 0.0,
            armor_bonus: 0.0,
            legendary_power: LegendaryPower::None,
            temper_level: 0,
            socketed_gem: None,
            slot: GearSlot::Weapon,
        };
        let upgrade = InventoryItem {
            name: "Ember-Saint Reaver".to_string(),
            quality: "legendary".to_string(),
            damage_bonus: 18.0,
            crit_chance: 0.12,
            health_bonus: 24.0,
            armor_bonus: 8.0,
            legendary_power: LegendaryPower::Emberbrand,
            temper_level: 1,
            socketed_gem: Some(SocketedGem {
                kind: crate::player::GemKind::Ruby,
                rank: 2,
            }),
            slot: GearSlot::Weapon,
        };
        let stash = InventoryItem {
            name: "Bent Fang".to_string(),
            quality: "common".to_string(),
            damage_bonus: 2.0,
            crit_chance: 0.0,
            health_bonus: 0.0,
            armor_bonus: 0.0,
            legendary_power: LegendaryPower::None,
            temper_level: 0,
            socketed_gem: None,
            slot: GearSlot::Weapon,
        };

        let equipped_label = inventory_slot_label(&equipped, &equipment, &damage_bonus);
        let upgrade_label = inventory_slot_label(&upgrade, &equipment, &damage_bonus);
        let stash_label = inventory_slot_label(&stash, &equipment, &damage_bonus);

        assert!(equipped_label.contains("EQUIPPED rare"));
        assert!(equipped_label.contains("CURRENT | equipped"));
        assert!(equipped_label.contains("PWR +0"));
        assert!(upgrade_label.contains("UPGRADE legendary"));
        assert!(upgrade_label.contains("BUILD POWER | [ equip power"));
        assert!(upgrade_label.contains("PWR +"));
        assert!(upgrade_label.contains("new power Emberbrand"));
        assert!(upgrade_label.contains("ember burn build"));
        assert!(upgrade_label.contains("socket Ruby 2"));
        assert!(upgrade_label.contains("temper +1"));
        assert!(upgrade_label.contains("weapon dmg +8"));
        assert!(upgrade_label.contains("crit build +"));
        assert!(upgrade_label.contains("toughness +"));
        assert!(stash_label.contains("STASH common"));
        assert!(equipped_label.contains("CURRENT"));
        assert!(upgrade_label.contains("BUILD POWER"));
        assert!(stash_label.contains("SALVAGE | U salvage"));
        assert!(stash_label.contains("PWR -"));
    }

    #[test]
    fn inventory_slot_action_hints_use_real_player_commands() {
        let mut equipment = test_equipment("legendary");
        equipment.weapon_name = "Equipped Fang".to_string();
        equipment.legendary_power = LegendaryPower::Stormbrand;
        equipment.socketed_gem = None;
        equipment.temper_level = 0;
        let damage_bonus = DamageBonus(18.0);

        let codex_piece = InventoryItem {
            name: "Soulreaver Archive Knife".to_string(),
            quality: "legendary".to_string(),
            damage_bonus: 4.0,
            crit_chance: 0.0,
            health_bonus: 0.0,
            armor_bonus: 0.0,
            legendary_power: LegendaryPower::Soulreaver,
            temper_level: 0,
            socketed_gem: None,
            slot: GearSlot::Weapon,
        };
        let socket_piece = InventoryItem {
            name: "Ruby Socket Reaver".to_string(),
            quality: "rare".to_string(),
            damage_bonus: 11.0,
            crit_chance: 0.03,
            health_bonus: 0.0,
            armor_bonus: 0.0,
            legendary_power: LegendaryPower::None,
            temper_level: 0,
            socketed_gem: Some(SocketedGem {
                kind: crate::player::GemKind::Ruby,
                rank: 3,
            }),
            slot: GearSlot::Weapon,
        };
        let temper_piece = InventoryItem {
            name: "Tempered Sideblade".to_string(),
            quality: "rare".to_string(),
            damage_bonus: 11.0,
            crit_chance: 0.03,
            health_bonus: 0.0,
            armor_bonus: 0.0,
            legendary_power: LegendaryPower::None,
            temper_level: 2,
            socketed_gem: None,
            slot: GearSlot::Weapon,
        };
        let salvage_piece = InventoryItem {
            name: "Bent Scrap Fang".to_string(),
            quality: "common".to_string(),
            damage_bonus: 1.0,
            crit_chance: 0.0,
            health_bonus: 0.0,
            armor_bonus: 0.0,
            legendary_power: LegendaryPower::None,
            temper_level: 0,
            socketed_gem: None,
            slot: GearSlot::Weapon,
        };

        assert!(inventory_slot_label(&codex_piece, &equipment, &damage_bonus).contains("B codex"));
        assert!(
            inventory_slot_label(&socket_piece, &equipment, &damage_bonus).contains("keep craft")
        );
        assert!(
            inventory_slot_label(&temper_piece, &equipment, &damage_bonus).contains("keep craft")
        );
        assert!(
            inventory_slot_label(&salvage_piece, &equipment, &damage_bonus).contains("U salvage")
        );
    }

    #[test]
    fn gear_build_tags_explain_current_equipment_role() {
        let mut equipment = test_equipment("ancient");
        equipment.weapon_name = "Ashen Reliquary Reaver".to_string();
        equipment.crit_chance = 0.18;
        equipment.health_bonus = 24.0;
        equipment.armor_bonus = 18.0;
        equipment.legendary_power = LegendaryPower::Soulreaver;
        equipment.temper_level = 2;
        equipment.socketed_gem = Some(SocketedGem {
            kind: crate::player::GemKind::Emerald,
            rank: 4,
        });
        let weapon = weapon_build_tag(&equipment, &DamageBonus(28.0), equipment.socketed_gem);
        assert!(weapon.contains("DPS carry"));
        assert!(weapon.contains("crit build"));
        assert!(weapon.contains("toughness") || weapon.contains("life steal build"));

        let charm = Charm {
            name: "Stormglass Charm".to_string(),
            quality: "legendary".to_string(),
            damage_bonus: 6.5,
            crit_chance: 0.045,
        };
        let charm_tag = charm_build_tag(&charm);
        assert!(charm_tag.contains("DPS charm"));
        assert!(charm_tag.contains("crit charm"));
        assert!(charm_tag.contains("resonance piece"));
    }

    #[test]
    fn skill_slot_summary_surfaces_cost_cooldown_and_effect() {
        let fury = test_fury(80.0);
        let cooldowns = test_cooldowns();
        let runes = SkillRunes {
            dash: DashRune::Reap,
            nova: NovaRune::Frost,
            rupture: RuptureRune::Hemorrhage,
        };
        let surge = test_surge(4, 0.0);
        let evade = test_evade(2.0, 0.0);

        let strike = skill_slot_summary(
            SkillSlotText::Strike,
            &fury,
            &cooldowns,
            &runes,
            &surge,
            &evade,
        );
        assert_eq!(strike.lines().next(), Some("LMB STRIKE"));
        assert_eq!(strike.lines().nth(1), Some("READY"));
        assert_eq!(strike.lines().nth(2), Some("RDY [######]"));
        assert_eq!(strike.lines().nth(3), Some("+18F BUILD FURY"));
        assert_eq!(strike.lines().count(), 4);
        assert!(strike.lines().all(|line| line.len() <= 16));

        let rupture = skill_slot_summary(
            SkillSlotText::Rupture,
            &fury,
            &cooldowns,
            &runes,
            &surge,
            &evade,
        );
        assert_eq!(rupture.lines().next(), Some("RMB RUPTURE"));
        assert_eq!(rupture.lines().nth(1), Some("CD 2.4s"));
        assert_eq!(rupture.lines().nth(2), Some("CD [##----]"));
        assert_eq!(rupture.lines().nth(3), Some("-32F START BLEED"));
        assert_eq!(rupture.lines().count(), 4);
        assert!(rupture.lines().all(|line| line.len() <= 18));

        let dash = skill_slot_summary(
            SkillSlotText::Dash,
            &fury,
            &cooldowns,
            &runes,
            &surge,
            &evade,
        );
        assert_eq!(dash.lines().next(), Some("Q DASH"));
        assert_eq!(dash.lines().nth(1), Some("CD 1.2s"));
        assert_eq!(dash.lines().nth(2), Some("CD [#####-]"));
        assert_eq!(dash.lines().nth(3), Some("-25F FINISH REAP"));
        assert_eq!(dash.lines().count(), 4);

        let nova = skill_slot_summary(
            SkillSlotText::Nova,
            &fury,
            &cooldowns,
            &runes,
            &surge,
            &evade,
        );
        assert_eq!(nova.lines().next(), Some("E NOVA"));
        assert_eq!(nova.lines().nth(1), Some("READY"));
        assert_eq!(nova.lines().nth(2), Some("RDY [######]"));
        assert_eq!(nova.lines().nth(3), Some("-45F FINISH FREEZE"));
        assert_eq!(nova.lines().count(), 4);

        let surge_summary = skill_slot_summary(
            SkillSlotText::Surge,
            &fury,
            &cooldowns,
            &runes,
            &surge,
            &evade,
        );
        assert_eq!(surge_summary.lines().next(), Some("Y SURGE"));
        assert_eq!(surge_summary.lines().nth(1), Some("CHARGE 4/9"));
        assert_eq!(surge_summary.lines().nth(2), Some("CHG [###---]"));
        assert_eq!(surge_summary.lines().nth(3), Some("CHARGE KILL CHARGE"));
        assert_eq!(surge_summary.lines().count(), 4);

        let evade_summary = skill_slot_summary(
            SkillSlotText::Evade,
            &fury,
            &cooldowns,
            &runes,
            &surge,
            &evade,
        );
        assert_eq!(evade_summary.lines().next(), Some("Shift EVADE"));
        assert_eq!(evade_summary.lines().nth(1), Some("CD 2.0s"));
        assert_eq!(evade_summary.lines().nth(2), Some("CD [###---]"));
        assert_eq!(evade_summary.lines().nth(3), Some("IFRAME DODGE HIT"));
        assert_eq!(evade_summary.lines().count(), 4);

        let starved = test_fury(8.0);
        let blocked = skill_slot_summary(
            SkillSlotText::Nova,
            &starved,
            &cooldowns,
            &runes,
            &surge,
            &evade,
        );
        assert!(blocked.contains("NEED 37F"));
        assert!(blocked.contains("FURY [#-----]"));
        assert!(blocked.contains("FINISH FREEZE"));
    }

    #[test]
    fn skill_slot_tactics_track_burst_and_survival_windows() {
        let runes = SkillRunes {
            dash: DashRune::Cleanse,
            nova: NovaRune::Ember,
            rupture: RuptureRune::Expose,
        };
        let charging_surge = test_surge(3, 0.0);
        let ready_surge = test_surge(9, 0.0);
        let active_surge = test_surge(9, 4.5);
        let idle_evade = test_evade(0.0, 0.0);
        let active_evade = test_evade(0.0, 0.4);

        assert_eq!(
            skill_slot_tactic(SkillSlotText::Rupture, &runes, &ready_surge, &idle_evade),
            "START VULN"
        );
        assert_eq!(
            skill_slot_tactic(SkillSlotText::Dash, &runes, &ready_surge, &idle_evade),
            "FINISH CLEANSE"
        );
        assert_eq!(
            skill_slot_tactic(SkillSlotText::Nova, &runes, &ready_surge, &idle_evade),
            "FINISH BURN"
        );
        assert_eq!(
            skill_slot_tactic(SkillSlotText::Surge, &runes, &charging_surge, &idle_evade),
            "KILL TO CHARGE"
        );
        assert_eq!(
            skill_slot_tactic(SkillSlotText::Surge, &runes, &ready_surge, &idle_evade),
            "BOSS BURST"
        );
        assert_eq!(
            skill_slot_tactic(SkillSlotText::Surge, &runes, &active_surge, &idle_evade),
            "DAMAGE WINDOW"
        );
        assert_eq!(
            skill_slot_tactic(SkillSlotText::Evade, &runes, &ready_surge, &active_evade),
            "I-FRAME NOW"
        );
    }

    #[test]
    fn skill_slot_blocked_label_reports_missing_fury_amount() {
        let fury = test_fury(18.0);

        assert_eq!(
            skill_slot_blocked_label(SkillSlotText::Dash, &fury),
            "NEED 7F"
        );
        assert_eq!(
            skill_slot_blocked_label(SkillSlotText::Rupture, &fury),
            "NEED 14F"
        );
        assert_eq!(
            skill_slot_blocked_label(SkillSlotText::Nova, &fury),
            "NEED 27F"
        );
    }

    #[test]
    fn skill_slot_themes_give_core_actions_distinct_readable_colors() {
        let strike = skill_slot_theme(SkillSlotText::Strike);
        let rupture = skill_slot_theme(SkillSlotText::Rupture);
        let dash = skill_slot_theme(SkillSlotText::Dash);
        let nova = skill_slot_theme(SkillSlotText::Nova);
        let surge = skill_slot_theme(SkillSlotText::Surge);
        let evade = skill_slot_theme(SkillSlotText::Evade);

        assert_ne!(strike.idle_background, rupture.idle_background);
        assert_ne!(dash.idle_background, nova.idle_background);
        assert_ne!(surge.idle_background, evade.idle_background);
        assert!(color_luminance(strike.ready_background) > color_luminance(strike.idle_background));
        assert!(color_luminance(nova.ready_border) > color_luminance(nova.idle_border));
    }

    #[test]
    fn skill_slot_glyphs_make_action_bar_scannable() {
        let slots = [
            SkillSlotText::Strike,
            SkillSlotText::Rupture,
            SkillSlotText::Dash,
            SkillSlotText::Nova,
            SkillSlotText::Surge,
            SkillSlotText::Evade,
        ];

        for (index, slot) in slots.iter().enumerate() {
            assert!(!skill_slot_glyph(*slot).is_empty());
            assert!(skill_slot_glyph_font_size(*slot) >= 21.0);
            assert!(color_luminance(skill_slot_glyph_color(*slot)) > 0.12);
            for other in slots.iter().skip(index + 1) {
                assert_ne!(skill_slot_glyph(*slot), skill_slot_glyph(*other));
            }
        }
        assert_eq!(skill_slot_glyph(SkillSlotText::Dash), ">>");
        assert!(
            skill_slot_keycap_font_size(SkillSlotText::Evade)
                < skill_slot_keycap_font_size(SkillSlotText::Dash)
        );
    }

    #[test]
    fn skill_slot_frame_style_pulses_casts_and_failures_on_matching_slot() {
        let fury = test_fury(80.0);
        let cooldowns = test_cooldowns();
        let surge = test_surge(9, 0.0);
        let evade = test_evade(0.0, 0.0);
        let idle_feedback = SkillFeedbackState::default();
        let base = skill_slot_frame_style(
            SkillSlotText::Nova,
            &fury,
            &cooldowns,
            &surge,
            &evade,
            &idle_feedback,
        );
        let cast_feedback = SkillFeedbackState {
            timer: test_timer(1.0, 0.6),
            pulse: Some(SkillSlotPulse {
                slot: SkillSlotText::Nova,
                kind: SkillSlotPulseKind::Cast,
            }),
        };
        let cast = skill_slot_frame_style(
            SkillSlotText::Nova,
            &fury,
            &cooldowns,
            &surge,
            &evade,
            &cast_feedback,
        );
        let blocked_feedback = SkillFeedbackState {
            timer: test_timer(1.0, 0.6),
            pulse: Some(SkillSlotPulse {
                slot: SkillSlotText::Nova,
                kind: SkillSlotPulseKind::Blocked,
            }),
        };
        let blocked = skill_slot_frame_style(
            SkillSlotText::Nova,
            &fury,
            &cooldowns,
            &surge,
            &evade,
            &blocked_feedback,
        );
        let untouched_dash = skill_slot_frame_style(
            SkillSlotText::Dash,
            &fury,
            &cooldowns,
            &surge,
            &evade,
            &cast_feedback,
        );

        assert_ne!(cast.background, base.background);
        assert_ne!(blocked.background, base.background);
        assert_eq!(
            untouched_dash,
            skill_slot_frame_style(
                SkillSlotText::Dash,
                &fury,
                &cooldowns,
                &surge,
                &evade,
                &idle_feedback,
            )
        );
        assert!(color_luminance(cast.border) > color_luminance(base.border));
        assert!(blocked.border.to_srgba().red > base.border.to_srgba().red);
    }

    #[test]
    fn skill_slot_lamps_make_action_bar_state_scannable_without_reading_text() {
        let fury = test_fury(80.0);
        let starved_fury = test_fury(8.0);
        let cooldowns = test_cooldowns();
        let ready_surge = test_surge(9, 0.0);
        let charging_surge = test_surge(4, 0.0);
        let active_surge = test_surge(9, 4.5);
        let idle_evade = test_evade(0.0, 0.0);
        let active_evade = test_evade(0.0, 0.4);

        let ready = skill_slot_lamp_style(
            SkillSlotText::Nova,
            &fury,
            &cooldowns,
            &ready_surge,
            &idle_evade,
        );
        let cooling = skill_slot_lamp_style(
            SkillSlotText::Dash,
            &fury,
            &cooldowns,
            &ready_surge,
            &idle_evade,
        );
        let starved = skill_slot_lamp_style(
            SkillSlotText::Nova,
            &starved_fury,
            &cooldowns,
            &ready_surge,
            &idle_evade,
        );
        let charging = skill_slot_lamp_style(
            SkillSlotText::Surge,
            &fury,
            &cooldowns,
            &charging_surge,
            &idle_evade,
        );
        let active = skill_slot_lamp_style(
            SkillSlotText::Surge,
            &fury,
            &cooldowns,
            &active_surge,
            &idle_evade,
        );
        let evading = skill_slot_lamp_style(
            SkillSlotText::Evade,
            &fury,
            &cooldowns,
            &ready_surge,
            &active_evade,
        );

        assert_eq!(ready.state, SkillSlotLampState::Ready);
        assert_eq!(cooling.state, SkillSlotLampState::Cooling);
        assert_eq!(starved.state, SkillSlotLampState::Starved);
        assert_eq!(charging.state, SkillSlotLampState::Charging);
        assert_eq!(active.state, SkillSlotLampState::Active);
        assert_eq!(evading.state, SkillSlotLampState::Active);
        assert!(ready.width > cooling.width);
        assert!(active.width > ready.width);
        assert!(charging.width > 18.0);
        assert!(starved.color.to_srgba().red > ready.color.to_srgba().red);
        assert!(color_luminance(active.color) > color_luminance(cooling.color));
    }

    #[test]
    fn compact_character_hud_surfaces_gear_bag_and_build_without_menu_wall() {
        let damage_bonus = DamageBonus(11.0);
        let mut equipment = test_equipment("legendary");
        equipment.weapon_name = "Stormcall Reliquary Brand".to_string();
        equipment.legendary_power = LegendaryPower::Stormbrand;
        equipment.crit_chance = 0.16;
        let charm = Charm {
            name: "Stormglass Charm".to_string(),
            quality: "legendary".to_string(),
            damage_bonus: 5.0,
            crit_chance: 0.035,
        };
        let inventory = Inventory {
            items: vec![
                InventoryItem {
                    name: equipment.weapon_name.clone(),
                    quality: equipment.quality.clone(),
                    damage_bonus: damage_bonus.0,
                    crit_chance: equipment.crit_chance,
                    health_bonus: 0.0,
                    armor_bonus: 0.0,
                    legendary_power: equipment.legendary_power,
                    temper_level: 0,
                    socketed_gem: None,
                    slot: GearSlot::Weapon,
                },
                InventoryItem {
                    name: "Moonforged Cleaver".to_string(),
                    quality: "rare".to_string(),
                    damage_bonus: 9.0,
                    crit_chance: 0.06,
                    health_bonus: 0.0,
                    armor_bonus: 0.0,
                    legendary_power: LegendaryPower::None,
                    temper_level: 0,
                    socketed_gem: None,
                    slot: GearSlot::Weapon,
                },
            ],
            capacity: 12,
        };
        let mut codex = LegendaryCodex::default();
        codex.unlock(LegendaryPower::Stormbrand);
        let talents = Talents {
            wrath: 2,
            vigor: 1,
            focus: 0,
            points: 0,
        };
        let runes = SkillRunes {
            dash: DashRune::Reap,
            nova: NovaRune::Frost,
            rupture: RuptureRune::Hemorrhage,
        };
        let resonance = reliquary_resonance(&equipment, &charm)
            .map(|resonance| resonance.label.to_string())
            .unwrap();
        let stats = RunStats::default();
        let set_tactic = reliquary_set_tactic(&equipment, &charm, &stats);

        let weapon = compact_character_summary(
            CompactCharacterText::Weapon,
            &damage_bonus,
            &equipment,
            &charm,
            &inventory,
            &codex,
            &talents,
            &runes,
            &resonance,
            &set_tactic,
            &stats,
        );
        assert!(weapon.contains("GEAR WEAPON"));
        assert!(weapon.contains("+11"));
        assert!(weapon.contains("16%"));
        assert!(weapon.lines().count() <= 3);

        let bag = compact_character_summary(
            CompactCharacterText::Inventory,
            &damage_bonus,
            &equipment,
            &charm,
            &inventory,
            &codex,
            &talents,
            &runes,
            &resonance,
            &set_tactic,
            &stats,
        );
        assert!(bag.contains("I BAG 2/12"));
        assert!(bag.contains("Next"));
        assert!(bag.contains("U salvage 1"));
        assert!(bag.lines().count() <= 3);

        let build = compact_character_summary(
            CompactCharacterText::Codex,
            &damage_bonus,
            &equipment,
            &charm,
            &inventory,
            &codex,
            &talents,
            &runes,
            &resonance,
            &set_tactic,
            &stats,
        );
        assert!(build.contains("K BUILD"));
        assert!(build.contains("STORM REAP"));
        assert!(build.contains("DPS"));
        assert!(build.contains("1/"));
        assert!(build.contains("Storm chain"));
        assert!(build.lines().count() <= 3);
    }

    #[test]
    fn bag_action_summary_prioritizes_upgrade_full_build_piece_and_salvage() {
        let stats = RunStats::default();
        let damage_bonus = DamageBonus(10.0);
        let mut equipment = test_equipment("rare");
        equipment.weapon_name = "Iron Fang".to_string();
        equipment.legendary_power = LegendaryPower::None;
        equipment.crit_chance = 0.05;
        let upgrade = InventoryItem {
            name: "Stormcall Edge".to_string(),
            quality: "legendary".to_string(),
            damage_bonus: 18.0,
            crit_chance: 0.12,
            health_bonus: 0.0,
            armor_bonus: 0.0,
            legendary_power: LegendaryPower::Stormbrand,
            temper_level: 0,
            socketed_gem: None,
            slot: GearSlot::Weapon,
        };
        let weak = InventoryItem {
            name: "Cracked Fang".to_string(),
            quality: "common".to_string(),
            damage_bonus: 1.0,
            crit_chance: 0.0,
            health_bonus: 0.0,
            armor_bonus: 0.0,
            legendary_power: LegendaryPower::None,
            temper_level: 0,
            socketed_gem: None,
            slot: GearSlot::Weapon,
        };
        let upgrade_inventory = Inventory {
            items: vec![weak.clone(), upgrade.clone()],
            capacity: 8,
        };
        assert!(
            bag_action_summary(&stats, &upgrade_inventory, &equipment, &damage_bonus)
                .contains("[ equip")
        );
        assert!(
            bag_action_summary(&stats, &upgrade_inventory, &equipment, &damage_bonus)
                .contains("Stormbrand")
        );

        let full_inventory = Inventory {
            items: vec![weak.clone(), upgrade],
            capacity: 2,
        };
        let full_action = bag_action_summary(&stats, &full_inventory, &equipment, &damage_bonus);
        assert!(full_action.contains("FULL U salvage 1"));
        assert!(full_action.contains("[ equip power"));
        assert!(full_action.contains("Stormbrand"));

        let sidegrade = InventoryItem {
            name: "Sidegrade Fang".to_string(),
            quality: "rare".to_string(),
            damage_bonus: 9.5,
            crit_chance: 0.05,
            health_bonus: 0.0,
            armor_bonus: 0.0,
            legendary_power: LegendaryPower::None,
            temper_level: 0,
            socketed_gem: None,
            slot: GearSlot::Weapon,
        };
        let build_inventory = Inventory {
            items: vec![sidegrade],
            capacity: 8,
        };
        assert_eq!(
            bag_action_summary(&stats, &build_inventory, &equipment, &damage_bonus),
            "stash swap PWR -2"
        );

        let socket_piece = InventoryItem {
            name: "Socketed Fang".to_string(),
            quality: "common".to_string(),
            damage_bonus: 1.0,
            crit_chance: 0.05,
            health_bonus: 0.0,
            armor_bonus: 0.0,
            legendary_power: LegendaryPower::None,
            temper_level: 0,
            socketed_gem: Some(SocketedGem {
                kind: crate::player::GemKind::Ruby,
                rank: 2,
            }),
            slot: GearSlot::Weapon,
        };
        let socket_inventory = Inventory {
            items: vec![socket_piece],
            capacity: 8,
        };
        let socket_action =
            bag_action_summary(&stats, &socket_inventory, &equipment, &damage_bonus);
        assert!(socket_action.contains("keep craft"));
        assert!(socket_action.contains("socket"));

        let salvage_inventory = Inventory {
            items: vec![weak],
            capacity: 8,
        };
        assert_eq!(
            bag_action_summary(&stats, &salvage_inventory, &equipment, &damage_bonus),
            "U salvage 1"
        );
    }

    #[test]
    fn inventory_panel_header_surfaces_bag_decisions_resources_and_codex() {
        let stats = RunStats {
            gold: 180,
            ember_shards: 7,
            affix_essence: 3,
            echo_keystones: 1,
            ..default()
        };
        let damage_bonus = DamageBonus(10.0);
        let mut equipment = test_equipment("rare");
        equipment.weapon_name = "Iron Fang".to_string();
        equipment.legendary_power = LegendaryPower::None;
        equipment.crit_chance = 0.05;
        let inventory = Inventory {
            items: vec![
                InventoryItem {
                    name: "Cracked Fang".to_string(),
                    quality: "common".to_string(),
                    damage_bonus: 1.0,
                    crit_chance: 0.0,
                    health_bonus: 0.0,
                    armor_bonus: 0.0,
                    legendary_power: LegendaryPower::None,
                    temper_level: 0,
                    socketed_gem: None,
                    slot: GearSlot::Weapon,
                },
                InventoryItem {
                    name: "Stormcall Edge".to_string(),
                    quality: "legendary".to_string(),
                    damage_bonus: 18.0,
                    crit_chance: 0.12,
                    health_bonus: 0.0,
                    armor_bonus: 0.0,
                    legendary_power: LegendaryPower::Stormbrand,
                    temper_level: 0,
                    socketed_gem: None,
                    slot: GearSlot::Weapon,
                },
            ],
            capacity: 12,
        };
        let potions = test_potions(2, 0.0);
        let elixirs = ElixirBelt {
            current: 1,
            max: 3,
            armor_bonus: 25.0,
            duration_secs: 15.0,
            selected: crate::player::ElixirKind::Wrath,
        };
        let mut codex = LegendaryCodex::default();
        codex.unlock(LegendaryPower::Stormbrand);

        let text = inventory_panel_header(
            &stats,
            &inventory,
            &equipment,
            &damage_bonus,
            &potions,
            &elixirs,
            &codex,
        );

        assert!(text.contains("INVENTORY | I close"));
        assert!(text.contains("[/] equip weapon"));
        assert!(text.contains("U salvage 1"));
        assert!(text.contains("B codex"));
        assert!(text.contains("O/P armory"));
        assert!(text.contains("Bag 2/12"));
        assert!(text.contains("Action [ equip power"));
        assert!(text.contains("Next legendary Stormcall"));
        assert!(text.contains("Gold 180 Shards 7 Essence 3 Echo 1"));
        assert!(text.contains("Elixir V Wrath G 1/3"));
        assert!(text.lines().count() <= 3);
        assert!(text.lines().all(|line| line.len() <= 118));
    }

    #[test]
    fn compact_utility_hud_surfaces_items_portal_and_interact_states() {
        let ready_potions = test_potions(3, 0.0);
        let recharging_potions = test_potions(2, 4.2);
        let empty_potions = test_potions(0, 0.0);
        assert_eq!(potion_utility_state(&ready_potions), "READY 3/5");
        assert_eq!(potion_utility_state(&recharging_potions), "CD 5s");
        assert_eq!(potion_utility_state(&empty_potions), "EMPTY");

        let elixirs = ElixirBelt {
            current: 2,
            max: 3,
            armor_bonus: 35.0,
            duration_secs: 22.0,
            selected: crate::player::ElixirKind::Wrath,
        };
        let mut active_elixir = inactive_elixir_buff();
        active_elixir.kind = crate::player::ElixirKind::Haste;
        active_elixir.timer = test_timer(22.0, 9.1);
        assert_eq!(elixir_utility_state(&elixirs, &active_elixir), "Haste 10s");

        let mut portal = inactive_portal();
        assert_eq!(portal_utility_state(&portal), "READY");
        portal.return_position = Some(Vec3::new(1.0, 0.0, 2.0));
        assert_eq!(portal_utility_state(&portal), "RETURN");
        portal.cooldown = test_timer(18.0, 6.2);
        assert_eq!(portal_utility_state(&portal), "CD 7s");

        let ritual = ChapterProgress {
            phase: ChapterPhase::Ritual,
            objective: "Claim the final reliquary seal".to_string(),
            ..default()
        };
        let potion_slot = compact_utility_summary(
            CompactUtilityText::Potion,
            &ready_potions,
            &elixirs,
            &active_elixir,
            &portal,
            &ritual,
        );
        assert!(potion_slot.contains("F POT"));
        assert!(potion_slot.contains("READY 3/5"));

        let interact_slot = compact_utility_summary(
            CompactUtilityText::Interact,
            &ready_potions,
            &elixirs,
            &active_elixir,
            &portal,
            &ritual,
        );
        assert_eq!(interact_slot, "SPACE\nAltar");
    }

    fn color_luminance(color: Color) -> f32 {
        let srgba = color.to_srgba();
        srgba.red * 0.2126 + srgba.green * 0.7152 + srgba.blue * 0.0722
    }

    #[test]
    fn loot_pickup_banner_parses_quality_and_copy() {
        let banner = loot_pickup_banner_from_event(
            "Picked up legendary Ember-Saint Relic Blade and 42 gold",
        )
        .expect("pickup event should create banner");
        assert_eq!(
            banner.copy,
            "LEGENDARY DROP\nEquip or codex-check Ember-Saint Relic Blade and 42 gold\nNext: test power, extract spare"
        );
        let ancient = loot_pickup_banner_from_event("Picked up ancient Ashen Reaver and 80 gold")
            .expect("ancient pickup");
        assert_eq!(
            ancient.copy,
            "ANCIENT DROP\nTemper candidate Ashen Reaver and 80 gold\nNext: compare DPS, augment if fit"
        );
        let primal = loot_pickup_banner_from_event("Picked up primal Soulreaver Fang and 120 gold")
            .expect("primal pickup");
        assert_eq!(
            primal.copy,
            "PRIMAL DROP\nBuild around Soulreaver Fang and 120 gold\nNext: socket, augment, save loadout"
        );
        let common = loot_pickup_banner_from_event("Picked up common Iron Fang (+4 damage)")
            .expect("common pickup");
        assert_eq!(
            common.copy,
            "COMMON DROP\nMaterial fodder Iron Fang (+4 damage)\nNext: salvage when bag fills"
        );
        assert_eq!(
            loot_pickup_banner_from_event("Potion restored 30 health").map(|banner| banner.copy),
            None
        );
    }

    #[test]
    fn loot_pickup_banner_keeps_item_decision_visible_for_all_qualities() {
        let rare = loot_pickup_banner_copy("rare", "Moonforged Cleaver (+9 damage)");
        assert!(rare.contains("RARE DROP"));
        assert!(rare.contains("Upgrade or craft base"));
        assert!(rare.contains("Next: equip gain, salvage spare"));

        let magic = loot_pickup_banner_copy("magic", "Duskworn Edge (+5 damage)");
        assert!(magic.contains("Next: replace or salvage"));

        let unknown = loot_pickup_banner_copy("set", "Gravebound Hood");
        assert_eq!(
            unknown,
            "SET DROP\nGravebound Hood\nNext: inspect or salvage"
        );
    }

    #[test]
    fn loot_proximity_hud_reports_distance_pickup_and_full_inventory() {
        let move_info = LootProximityInfo {
            visible: true,
            priority: "BEST".to_string(),
            label: "rare Moonforged Cleaver | +9 dmg".to_string(),
            compare: "AUTO-EQUIP DPS GAIN [DPS] | PWR +7 | dmg +5 crit +0% hp +0 armor +0"
                .to_string(),
            rewards: "12 gold".to_string(),
            distance: LOOT_PICKUP_RADIUS + 1.4,
            pickup_ready: false,
            inventory_full: false,
            spare_salvage_available: false,
            quality: "rare".to_string(),
        };
        let ready_info = LootProximityInfo {
            pickup_ready: true,
            distance: LOOT_PICKUP_RADIUS - 0.2,
            ..move_info.clone()
        };
        let full_info = LootProximityInfo {
            inventory_full: true,
            spare_salvage_available: true,
            quality: "legendary".to_string(),
            ..ready_info.clone()
        };

        assert!(loot_proximity_summary(&move_info).contains("MOVE TO EQUIP BEST 1.4m"));
        assert!(loot_proximity_summary(&move_info).contains("equip for DPS"));
        assert!(loot_proximity_summary(&ready_info).contains("AUTO EQUIP BEST 0.0m"));
        assert!(loot_proximity_summary(&full_info).contains("AUTO EQUIP"));
        assert!(loot_proximity_summary(&full_info).contains("U salvage spares"));
        assert!(loot_compare_auto_equips(&ready_info.compare));
        let power_info = LootProximityInfo {
            compare: "AUTO-EQUIP BUILD POWER [CODEX] | PWR +30 | dmg +0 crit +0% hp +0 armor +0 | new power Stormbrand".to_string(),
            ..ready_info.clone()
        };
        assert!(loot_proximity_summary(&power_info).contains("unlock codex power"));

        let craft_info = LootProximityInfo {
            compare:
                "SIDEGRADE SOCKET [CRAFT] | PWR -2 | dmg +0 crit +0% hp +0 armor +0 | socket Ruby 3"
                    .to_string(),
            ..ready_info.clone()
        };
        assert!(loot_proximity_summary(&craft_info).contains("keep for craft"));

        let salvage_info = LootProximityInfo {
            pickup_ready: false,
            inventory_full: false,
            spare_salvage_available: false,
            compare: "STASH SALVAGE LATER [STASH] | PWR -16 | dmg -8 crit +0% hp +0 armor +0"
                .to_string(),
            ..ready_info.clone()
        };
        assert!(loot_proximity_summary(&salvage_info).contains("stash or salvage"));
        let full_salvage_info = LootProximityInfo {
            inventory_full: true,
            compare: "FULL MAKE ROOM [SALVAGE] | PWR -16 | dmg -8 crit +0% hp +0 armor +0"
                .to_string(),
            ..ready_info.clone()
        };
        assert!(loot_proximity_summary(&full_salvage_info).contains("salvage spare"));
        assert_ne!(
            loot_proximity_color("rare"),
            loot_proximity_color("legendary")
        );
    }

    #[test]
    fn interact_proximity_hud_reports_ready_move_used_and_role() {
        let progress = ChapterProgress {
            phase: ChapterPhase::Cache,
            objective: "Open the reliquary cache".to_string(),
            ..default()
        };
        let cache = Interactable {
            kind: InteractableKind::Cache,
            radius: 1.4,
            used: false,
            reusable: false,
        };
        let merchant = Interactable {
            kind: InteractableKind::Merchant,
            radius: 1.6,
            used: false,
            reusable: true,
        };
        let used_shrine = Interactable {
            kind: InteractableKind::FortuneShrine,
            radius: 1.2,
            used: true,
            reusable: false,
        };

        let ready = interact_proximity_summary(&cache, 1.0, &progress, None);
        assert!(ready.contains("SPACE READY"));
        assert!(ready.contains("MAIN Reliquary Cache"));
        assert!(ready.contains("main path: open now"));

        let move_text = interact_proximity_summary(&merchant, 2.1, &progress, None);
        assert!(move_text.contains("MOVE 0.5m"));
        assert!(move_text.contains("OPTIONAL Quartermaster"));
        assert!(move_text.contains("SERVICE craft, temper, salvage"));

        let used = interact_proximity_summary(&used_shrine, 0.8, &progress, None);
        assert!(used.contains("USED"));
        assert!(used.contains("SIDE Fortune Shrine"));
        assert!(used.contains("BUFF gold + magic find"));
        assert_ne!(
            interact_proximity_color(&cache, &progress),
            interact_proximity_color(&used_shrine, &progress)
        );
    }

    #[test]
    fn interact_proximity_hud_surfaces_rewards_risks_and_boss_triggers() {
        let ritual = ChapterProgress {
            phase: ChapterPhase::Ritual,
            objective: "Extinguish the ember altar".to_string(),
            seal_fragments: REQUIRED_SEAL_FRAGMENTS,
            ..default()
        };
        let altar = Interactable {
            kind: InteractableKind::Altar,
            radius: 1.5,
            used: false,
            reusable: false,
        };
        let cursed = Interactable {
            kind: InteractableKind::CursedShrine,
            radius: 1.2,
            used: false,
            reusable: false,
        };
        let well = Interactable {
            kind: InteractableKind::HealingWell,
            radius: 1.4,
            used: false,
            reusable: true,
        };

        let altar_text = interact_proximity_summary(&altar, 1.0, &ritual, None);
        assert!(altar_text.contains("MAIN Ember Altar"));
        assert!(altar_text.contains("BOSS TRIGGER final seal"));
        assert!(altar_text.contains("main path: claim final seal"));

        let cursed_text = interact_proximity_summary(&cursed, 1.0, &ritual, None);
        assert!(cursed_text.contains("SIDE Cursed Shrine"));
        assert!(cursed_text.contains("RISK blood price, elite ambush"));

        let well_text = interact_proximity_summary(&well, 1.0, &ritual, None);
        assert!(well_text.contains("SIDE Healing Well"));
        assert!(well_text.contains("RECOVER life + potions"));
        assert!(well_text.contains("save for low health or before boss"));
    }

    #[test]
    fn interact_proximity_hud_surfaces_live_quartermaster_service() {
        let progress = ChapterProgress {
            phase: ChapterPhase::Ritual,
            objective: "Prepare before Malrec".to_string(),
            ..default()
        };
        let merchant = Interactable {
            kind: InteractableKind::Merchant,
            radius: 1.6,
            used: false,
            reusable: true,
        };

        let potion_text = interact_proximity_summary(
            &merchant,
            1.0,
            &progress,
            Some("Quartermaster next: Space buys 2 potion(s) for 35g each"),
        );
        assert!(potion_text.contains("SPACE READY"));
        assert!(potion_text.contains("OPTIONAL Quartermaster"));
        assert!(potion_text.contains("SERVICE Space buys 2 potion"));
        assert!(potion_text.contains("upgrade before boss"));

        let primal_text = interact_proximity_summary(
            &merchant,
            2.0,
            &progress,
            Some(
                "Quartermaster ready: Space can primal-infuse after auto-upgrades for 520g/24s/14e",
            ),
        );
        assert!(primal_text.contains("MOVE 0.4m"));
        assert!(primal_text.contains("SERVICE Space can primal-infuse"));
        assert!(!primal_text.contains("SERVICE craft, temper, salvage"));
        assert!(primal_text.lines().all(|line| line.len() <= 96));
    }

    #[test]
    fn combat_streak_banner_parses_massacre_and_valor_events() {
        let massacre =
            combat_streak_banner_from_event("Massacre 3x: +5 XP +5 gold").expect("massacre");
        assert_eq!(massacre.copy, "MASSACRE 3X\nChain kills +5 XP +5 gold");
        assert!((massacre.duration_secs - 1.8).abs() < f32::EPSILON);

        let carnage =
            combat_streak_banner_from_event("Carnage 5x +10 XP +10 gold").expect("carnage");
        assert_eq!(
            carnage.copy,
            "CARNAGE 5X\nMassacre Rite online +10 XP +10 gold"
        );

        let annihilation = combat_streak_banner_from_event("Annihilation 20x +15 XP +15 gold")
            .expect("annihilation");
        assert_eq!(
            annihilation.copy,
            "ANNIHILATION 20X\nBattle Trance pace +15 XP +15 gold"
        );

        let valor =
            combat_streak_banner_from_event("Valor 4x: elite rewards empowered").expect("valor");
        assert_eq!(
            valor.copy,
            "VALOR 4X\nOne elite to full Valor +32% elite rewards"
        );
        assert!((valor.duration_secs - 1.6).abs() < f32::EPSILON);

        let first_valor =
            combat_streak_banner_from_event("Valor 1x: elite rewards empowered").expect("valor");
        assert_eq!(first_valor.copy, "VALOR 1X\nChain elites +8% elite rewards");

        let full_valor =
            combat_streak_banner_from_event("Valor 5x: elite rewards empowered").expect("valor");
        assert_eq!(
            full_valor.copy,
            "VALOR 5X\nFull Valor, push boss +40% elite rewards"
        );

        assert!(combat_streak_banner_from_event("Picked up rare Iron Fang").is_none());
    }

    #[test]
    fn skill_feedback_surfaces_casts_and_failures_without_loot_spam() {
        let dash = skill_feedback_from_event("Reap Dash").expect("dash cast");
        assert_eq!(dash.copy, "REAP DASH");
        assert!(dash.duration_secs >= 1.0);
        assert_eq!(
            dash.pulse,
            Some(SkillSlotPulse {
                slot: SkillSlotText::Dash,
                kind: SkillSlotPulseKind::Cast,
            })
        );

        let nova = skill_feedback_from_event("Frost Nova").expect("nova cast");
        assert_eq!(nova.copy, "FROST NOVA");
        assert_eq!(
            nova.pulse.map(|pulse| pulse.slot),
            Some(SkillSlotText::Nova)
        );

        let cooldown = skill_feedback_from_event("Dash not ready 2s").expect("cooldown");
        assert_eq!(cooldown.copy, "DASH NOT READY 2S");
        assert_eq!(
            cooldown.pulse,
            Some(SkillSlotPulse {
                slot: SkillSlotText::Dash,
                kind: SkillSlotPulseKind::Blocked,
            })
        );

        let potion_cooldown =
            skill_feedback_from_event("Potion recharging 6s").expect("potion cooldown");
        assert_eq!(potion_cooldown.copy, "POTION RECHARGING 6S");

        let fury = skill_feedback_from_event("Need 45 fury for nova").expect("fury fail");
        assert_eq!(fury.copy, "NEED 45 FURY FOR NOVA");
        assert_eq!(
            fury.pulse,
            Some(SkillSlotPulse {
                slot: SkillSlotText::Nova,
                kind: SkillSlotPulseKind::Blocked,
            })
        );

        let surge = skill_feedback_from_event("Nephalem Surge unleashed").expect("surge");
        assert_eq!(surge.copy, "Nephalem Surge unleashed");
        assert_eq!(
            surge.pulse.map(|pulse| pulse.slot),
            Some(SkillSlotText::Surge)
        );

        let evade = skill_feedback_from_event("Evade").expect("evade");
        assert_eq!(evade.copy, "Evade");
        assert_eq!(
            evade.pulse,
            Some(SkillSlotPulse {
                slot: SkillSlotText::Evade,
                kind: SkillSlotPulseKind::Survival,
            })
        );

        let ember = skill_feedback_from_event("Emberbrand ignited enemy").expect("emberbrand");
        assert_eq!(ember.copy, "EMBERBRAND IGNITE");
        assert!(ember.duration_secs >= 1.0);

        let frost = skill_feedback_from_event("Frostbrand chilled enemy").expect("frostbrand");
        assert_eq!(frost.copy, "FROSTBRAND CHILL");

        let storm = skill_feedback_from_event("Stormbrand chained for 18").expect("stormbrand");
        assert_eq!(storm.copy, "STORMBRAND CHAIN");

        let soul = skill_feedback_from_event("Soulreaver restored 12 health").expect("soulreaver");
        assert_eq!(soul.copy, "SOULREAVER LEECH");

        let aegis = skill_feedback_from_event("Aegisbrand granted 8 barrier").expect("aegisbrand");
        assert_eq!(aegis.copy, "AEGISBRAND WARD");

        let combo_ready =
            skill_feedback_from_event("Combo Ready: finish Rupture with Dash or Nova")
                .expect("combo ready");
        assert_eq!(combo_ready.copy, "COMBO READY\nQ/E FINISH");

        let nova_combo_ready =
            skill_feedback_from_event("Combo Ready: Dash through Nova-marked enemies")
                .expect("nova combo ready");
        assert_eq!(nova_combo_ready.copy, "COMBO READY\nQ DASH FINISH");

        let combo_break =
            skill_feedback_from_event("Combo Break: Rupture into Nova hit 3 targets +17 fury")
                .expect("combo break");
        assert_eq!(combo_break.copy, "COMBO BREAK\n3 TARGETS +17 FURY");
        assert!(combo_break.duration_secs > combo_ready.duration_secs);

        let rich_combo_break = skill_feedback_from_event(
            "Combo Break: Rupture into Nova hit 4 targets +28 fury +17 health | Blood nova | Storm surge",
        )
        .expect("rich combo break");
        assert_eq!(
            rich_combo_break.copy,
            "COMBO BREAK\n4 TARGETS +28 FURY +17 HP"
        );

        let potion = skill_feedback_from_event("Potion restored 45 health").expect("potion");
        assert_eq!(potion.copy, "POTION +45 HP");
        assert!(potion.duration_secs >= 1.0);

        let globe = skill_feedback_from_event("Health globe restored 28 health; Glory 2x")
            .expect("health globe");
        assert_eq!(globe.copy, "HEALTH GLOBE");

        let well =
            skill_feedback_from_event("Renewal well restored 50 health, 20 barrier, 3 potions")
                .expect("renewal well");
        assert_eq!(well.copy, "RENEWAL WELL");

        let altar = skill_feedback_from_event("Ember altar extinguished: restored 65 health")
            .expect("ember altar");
        assert_eq!(altar.copy, "EMBER ALTAR RESTORED");

        let iron =
            skill_feedback_from_event("Iron elixir: +35 armor and +18 barrier").expect("iron");
        assert_eq!(iron.copy, "IRON ELIXIR");

        let wrath = skill_feedback_from_event("Wrath elixir: +18% damage").expect("wrath");
        assert_eq!(wrath.copy, "WRATH ELIXIR");

        let haste = skill_feedback_from_event("Haste elixir: +20% movement speed").expect("haste");
        assert_eq!(haste.copy, "HASTE ELIXIR");

        assert!(skill_feedback_from_event("Picked up rare Iron Fang").is_none());
        assert!(skill_feedback_from_event("Critical basic hit for 12").is_none());
        assert!(skill_feedback_from_event("Elixir selected: Haste").is_none());
    }

    #[test]
    fn buff_bar_is_empty_without_active_buffs() {
        let relic = inactive_relic_buff();
        let elixir = inactive_elixir_buff();
        let fortune = inactive_fortune_buff();
        let conduit = inactive_conduit_buff();
        let glory = inactive_glory_buff();
        let surge = test_surge(0, 0.0);
        let portal = inactive_portal();
        let health = test_health(100.0, 100.0);

        assert_eq!(
            buff_bar_text(BuffBarInputs {
                relic: &relic,
                elixir: &elixir,
                fortune: &fortune,
                conduit: &conduit,
                glory: &glory,
                surge: &surge,
                portal: &portal,
                health: &health,
                burning: None,
                jailed: None,
            }),
            ""
        );
    }

    #[test]
    fn buff_bar_surfaces_active_buffs_and_timers() {
        let relic = RelicBuff {
            timer: test_timer(10.0, 7.2),
            damage_multiplier: 1.35,
            speed_multiplier: 1.18,
        };
        let elixir = ElixirBuff {
            timer: test_timer(22.0, 12.1),
            kind: crate::player::ElixirKind::Wrath,
            armor_bonus: 0.0,
            damage_multiplier: 1.18,
            speed_multiplier: 1.0,
        };
        let fortune = inactive_fortune_buff();
        let conduit = inactive_conduit_buff();
        let glory = GloryBuff {
            timer: test_timer(16.0, 5.4),
            stacks: 2,
            max_stacks: 3,
            damage_per_stack: 0.06,
            speed_per_stack: 0.04,
        };
        let surge = test_surge(9, 3.6);
        let mut portal = inactive_portal();
        portal.cooldown = test_timer(18.0, 9.2);
        portal.return_position = Some(Vec3::new(1.0, 0.0, 2.0));
        let health = test_health(100.0, 100.0);

        assert_eq!(
            buff_bar_text(BuffBarInputs {
                relic: &relic,
                elixir: &elixir,
                fortune: &fortune,
                conduit: &conduit,
                glory: &glory,
                surge: &surge,
                portal: &portal,
                health: &health,
                burning: None,
                jailed: None,
            }),
            "Shrine 8s  |  Wrath Elixir 13s  |  Glory 2x 6s  |  Surge 4s  |  +1 buffs"
        );
        assert_eq!(
            compact_buff_labels(vec![
                "A".to_string(),
                "B".to_string(),
                "C".to_string(),
                "D".to_string(),
                "E".to_string(),
                "F".to_string(),
            ]),
            vec![
                "A".to_string(),
                "B".to_string(),
                "C".to_string(),
                "D".to_string(),
                "+2 buffs".to_string(),
            ]
        );
    }

    #[test]
    fn buff_bar_surfaces_danger_and_control_states() {
        let relic = inactive_relic_buff();
        let elixir = inactive_elixir_buff();
        let fortune = inactive_fortune_buff();
        let conduit = inactive_conduit_buff();
        let glory = inactive_glory_buff();
        let surge = test_surge(0, 0.0);
        let portal = inactive_portal();
        let health = test_health(28.0, 100.0);
        let burning = Burning {
            dps: 6.0,
            timer: test_timer(3.0, 1.7),
        };
        let jailed = Jailed {
            timer: test_timer(2.4, 0.8),
            speed_multiplier: 0.2,
        };

        assert!(player_health_danger(&health));
        assert_eq!(hud_seconds(burning_seconds_remaining(&burning)), 2);
        assert_eq!(hud_seconds(jailed_seconds_remaining(&jailed)), 1);
        assert_eq!(
            buff_bar_text(BuffBarInputs {
                relic: &relic,
                elixir: &elixir,
                fortune: &fortune,
                conduit: &conduit,
                glory: &glory,
                surge: &surge,
                portal: &portal,
                health: &health,
                burning: Some(&burning),
                jailed: Some(&jailed),
            }),
            "DANGER low life - heal now  |  BURNING - heal or cleanse 2s  |  ROOTED - break chains 1s"
        );
        assert!(
            buff_bar_text(BuffBarInputs {
                relic: &relic,
                elixir: &elixir,
                fortune: &fortune,
                conduit: &conduit,
                glory: &glory,
                surge: &surge,
                portal: &portal,
                health: &health,
                burning: Some(&burning),
                jailed: Some(&jailed),
            })
            .len()
                <= 96
        );
    }

    #[test]
    fn top_buff_bar_clears_during_combat_focus() {
        let buff_text = "Shrine 8s  |  Wrath Elixir 13s";
        assert_eq!(top_buff_bar_text(buff_text, false), buff_text);
        assert_eq!(top_buff_bar_text(buff_text, true), "");

        let safe_health = test_health(100.0, 100.0);
        let low_health = test_health(28.0, 100.0);
        let progress = ChapterProgress::default();
        let mut target = TargetInfo::default();

        assert!(!hud_top_clear_active(
            &progress,
            &safe_health,
            &target,
            &BloodObelisk::default(),
            &EmberRift::default()
        ));

        target.visible = true;
        assert!(hud_top_clear_active(
            &progress,
            &safe_health,
            &target,
            &BloodObelisk::default(),
            &EmberRift::default()
        ));

        target.visible = false;
        assert!(hud_top_clear_active(
            &ChapterProgress {
                phase: ChapterPhase::Boss,
                ..default()
            },
            &safe_health,
            &target,
            &BloodObelisk::default(),
            &EmberRift::default()
        ));
        assert!(hud_top_clear_active(
            &progress,
            &low_health,
            &target,
            &BloodObelisk::default(),
            &EmberRift::default()
        ));
        assert!(hud_top_clear_active(
            &progress,
            &safe_health,
            &target,
            &BloodObelisk {
                state: ObeliskState::Active,
                ..default()
            },
            &EmberRift::default()
        ));
        assert!(hud_top_clear_active(
            &progress,
            &safe_health,
            &target,
            &BloodObelisk::default(),
            &EmberRift {
                state: RiftState::Active,
                ..default()
            }
        ));
    }

    #[test]
    fn minimap_projection_clamps_to_panel_bounds() {
        assert_eq!(
            minimap_position(Vec3::new(WORLD_MIN_X, 0.0, WORLD_MIN_Z)),
            Vec2::new(MINIMAP_MAP_LEFT, MINIMAP_MAP_TOP)
        );
        assert_eq!(
            minimap_position(Vec3::new(WORLD_MAX_X, 0.0, WORLD_MAX_Z)),
            Vec2::new(
                MINIMAP_WIDTH - MINIMAP_MAP_RIGHT,
                MINIMAP_HEIGHT - MINIMAP_MAP_BOTTOM
            )
        );
        assert_eq!(
            minimap_position(Vec3::new(999.0, 0.0, -999.0)),
            Vec2::new(MINIMAP_WIDTH - MINIMAP_MAP_RIGHT, MINIMAP_MAP_TOP)
        );
    }

    #[test]
    fn minimap_objective_line_surfaces_phase_and_compacts_goal() {
        let progress = ChapterProgress {
            phase: ChapterPhase::Ritual,
            objective: "Extinguish the ember altar before Malrec awakens completely".to_string(),
            ..default()
        };

        let line = minimap_objective_line(&progress);

        assert!(line.starts_with("Ritual | "));
        assert!(line.contains("Extinguish the ember altar"));
        assert!(line.ends_with("..."));
        assert!(line.chars().count() <= "Ritual | ".chars().count() + 33);
    }

    #[test]
    fn minimap_legend_entries_cover_core_marker_roles() {
        let entries = minimap_legend_entries();

        assert_eq!(
            entries.map(|(_, label)| label),
            ["You", "Main", "Loot", "Life", "Fury", "Threat"]
        );
        assert_eq!(
            minimap_legend_color(MinimapLegendRole::Player),
            Color::srgb(0.20, 0.92, 0.46)
        );
        assert_eq!(
            minimap_legend_color(MinimapLegendRole::Main),
            minimap_interactable_style(MinimapInteractableRole::Primary).1
        );
        assert_eq!(
            minimap_legend_color(MinimapLegendRole::Loot),
            Color::srgb(0.95, 0.72, 0.18)
        );
        assert_eq!(
            minimap_legend_color(MinimapLegendRole::Life),
            Color::srgb(1.0, 0.08, 0.12)
        );
        assert_eq!(
            minimap_legend_color(MinimapLegendRole::Fury),
            Color::srgb(0.18, 0.58, 1.0)
        );
        assert_eq!(
            minimap_legend_color(MinimapLegendRole::Threat),
            Color::srgb(1.0, 0.36, 0.04)
        );
    }

    #[test]
    fn reward_toast_parses_chapter_and_reward_events() {
        let milestone = reward_toast_from_event(
            "MAIN Breach Secured | NEXT open sealed cache | REWARD outer seal + cache loot",
        )
        .expect("main chapter milestones should produce a toast");
        assert_eq!(
            milestone.copy,
            "MAIN QUEST\nBreach Secured | Next: open sealed cache | Reward: outer seal + cache loot"
        );
        assert!(milestone.duration_secs >= 3.7);

        let boss = reward_toast_from_event(
            "BOSS Malrec Awakened | NEXT break stagger, deny enrage | PAYOFF Soul Sigil + chapter cache",
        )
        .expect("boss chapter milestones should produce a toast");
        assert_eq!(
            boss.copy,
            "BOSS OBJECTIVE\nMalrec Awakened | Next: break stagger, deny enrage | Payoff: Soul Sigil + chapter cache"
        );
        assert!(boss.duration_secs > milestone.duration_secs);

        let quest = reward_toast_from_event("Main Quest Complete: the Ashen Reliquary is cleansed")
            .expect("main quest completion should produce a toast");
        assert_eq!(
            quest.copy,
            "QUEST COMPLETE\nthe Ashen Reliquary is cleansed"
        );
        assert!(quest.duration_secs > 4.0);

        let seal = reward_toast_from_event("Reliquary seal 2/3 recovered: +80 gold +5 shards")
            .expect("seal rewards should produce a toast");
        assert!(seal.copy.starts_with("SEAL RECOVERED\nReliquary seal 2/3"));

        let reward = reward_toast_from_event("Chapter reward: 315 gold, 12 shards, 7 essence")
            .expect("chapter rewards should produce a toast");
        assert_eq!(
            reward.copy,
            "CHAPTER REWARD\n315 gold, 12 shards, 7 essence"
        );

        let boon =
            reward_toast_from_event("Reliquary boon claimed: Empowered Titan Ward: +28 health")
                .expect("boon choices should produce a toast");
        assert_eq!(
            boon.copy,
            "RELIQUARY BOON\nEmpowered Titan Ward: +28 health"
        );

        let challenge = reward_toast_from_event("Challenge rewards: 125 gold, 8 shards, 3 essence")
            .expect("challenge rewards should produce a toast");
        assert_eq!(
            challenge.copy,
            "CHALLENGE REWARD\n125 gold, 8 shards, 3 essence"
        );

        let salvage = reward_toast_from_event(
            "Salvaged 4 spare gear: +60g +12s +5 essence | salvage cache x1: +80g +10s +5 essence",
        )
        .expect("salvage cache rewards should produce a toast");
        assert!(salvage.copy.starts_with("SALVAGE CACHE\nSalvaged 4"));

        let infusion = reward_toast_from_event(
            "Primal Infusion complete: Primal Ashen Reliquary Reaver forged",
        )
        .expect("primal infusion should produce a toast");
        assert_eq!(
            infusion.copy,
            "PRIMAL INFUSION\nPrimal Ashen Reliquary Reaver forged"
        );

        let paragon = reward_toast_from_event("Ember Paragon 3 awakened")
            .expect("paragon rank awakenings should produce a toast");
        assert_eq!(paragon.copy, "EMBER PARAGON\nRank 3 awakened");

        let relic = reward_toast_from_event("Relic shrine: +35% damage and +18% speed")
            .expect("relic shrine activation should produce a toast");
        assert!(relic.copy.starts_with("RELIC SHRINE\n+35%"));

        let fortune =
            reward_toast_from_event("Fortune shrine: +50% gold, +25% XP, and better drops")
                .expect("fortune shrine activation should produce a toast");
        assert!(fortune.copy.starts_with("FORTUNE SHRINE\n+50%"));

        let storm = reward_toast_from_event("Storm shrine: conduit lightning awakened")
            .expect("storm shrine activation should produce a toast");
        assert_eq!(storm.copy, "STORM SHRINE\nconduit lightning awakened");

        let well =
            reward_toast_from_event("Renewal well restored 145 health, 90 barrier, 2 potions")
                .expect("healing well should produce a toast");
        assert_eq!(
            well.copy,
            "HEALING WELL\nRenewal well restored 145 health, 90 barrier, 2 potions"
        );

        let lore = reward_toast_from_event("Lore recovered: Steward warning sigil")
            .expect("lore page reads should produce a toast");
        assert_eq!(lore.copy, "LORE RECOVERED\nSteward warning sigil");

        let cache = reward_toast_from_event("Reliquary cache opened: treasure released")
            .expect("cache opening should produce a toast");
        assert_eq!(cache.copy, "CACHE OPENED\ntreasure released");

        let vault = reward_toast_from_event("Reliquary vault opened: +3 affix essence")
            .expect("vault opening should produce a toast");
        assert_eq!(vault.copy, "VAULT OPENED\n+3 affix essence");

        let cursed = reward_toast_from_event("Cursed shrine claimed 12 health and released a boon")
            .expect("cursed shrine rewards should produce a toast");
        assert_eq!(
            cursed.copy,
            "CURSED SHRINE\nCursed shrine claimed 12 health and released a boon"
        );

        let obelisk = reward_toast_from_event("Blood obelisk awakened: feed it four kills")
            .expect("obelisk activation should produce a toast");
        assert_eq!(obelisk.copy, "BLOOD OBELISK\nfeed it four kills");

        let pylon =
            reward_toast_from_event("Ashen pylon: +55% damage, +24% speed, +42 barrier, +38 fury")
                .expect("pylon activation should produce a toast");
        assert_eq!(
            pylon.copy,
            "ASHEN PYLON\n+55% damage, +24% speed, +42 barrier, +38 fury"
        );

        let rift = reward_toast_from_event(
            "Ember rift sealed: +135 gold +7 shards +5 essence, Momentum 1x",
        )
        .expect("rifts should produce a toast");
        assert!(rift.copy.contains("RIFT SEALED"));
    }

    #[test]
    fn reward_toast_filters_combat_and_pickup_spam() {
        assert!(reward_toast_from_event("Picked up rare Moonforged Cleaver (+9 damage)").is_none());
        assert!(reward_toast_from_event("Massacre 12 enemies: +120 xp").is_none());
        assert!(reward_toast_from_event("Valor 4/5: +8% damage").is_none());
        assert!(reward_toast_from_event("Salvaged 1 spare gear: +3g +1s +0 essence").is_none());
        assert!(reward_toast_from_event("Gained 35 Ember Paragon XP").is_none());
    }

    #[test]
    fn objective_interactable_follows_chapter_phase() {
        let mut progress = ChapterProgress {
            phase: ChapterPhase::Cache,
            ..default()
        };
        assert!(is_primary_objective_interactable(
            &progress,
            InteractableKind::Cache
        ));
        assert!(is_objective_interactable(
            &progress,
            InteractableKind::Cache
        ));
        assert!(!is_objective_interactable(
            &progress,
            InteractableKind::Altar
        ));

        progress.phase = ChapterPhase::Ritual;
        assert!(is_primary_objective_interactable(
            &progress,
            InteractableKind::Altar
        ));
        assert!(is_objective_interactable(
            &progress,
            InteractableKind::Altar
        ));
        assert!(!is_primary_objective_interactable(
            &progress,
            InteractableKind::CursedShrine
        ));
        assert!(is_guided_optional_interactable(
            InteractableKind::CursedShrine
        ));
        assert!(is_objective_interactable(
            &progress,
            InteractableKind::StormShrine
        ));
        assert!(is_objective_interactable(
            &progress,
            InteractableKind::AshenPylon
        ));
        assert!(is_objective_interactable(
            &progress,
            InteractableKind::HealingWell
        ));
        assert!(is_objective_interactable(
            &progress,
            InteractableKind::BloodObelisk
        ));
        assert!(is_objective_interactable(
            &progress,
            InteractableKind::ReliquaryVault
        ));
        assert_ne!(
            minimap_interactable_role(&progress, InteractableKind::Altar),
            minimap_interactable_role(&progress, InteractableKind::CursedShrine)
        );

        let (primary_size, primary_color) = minimap_interactable_style(minimap_interactable_role(
            &progress,
            InteractableKind::Altar,
        ));
        let (risk_size, risk_color) = minimap_interactable_style(minimap_interactable_role(
            &progress,
            InteractableKind::CursedShrine,
        ));
        let (reward_size, reward_color) = minimap_interactable_style(minimap_interactable_role(
            &progress,
            InteractableKind::ReliquaryVault,
        ));
        let (other_size, _) = minimap_interactable_style(MinimapInteractableRole::Other);

        assert!(primary_size > risk_size);
        assert!(risk_size > other_size);
        assert!(reward_size > other_size);
        assert_ne!(primary_color, risk_color);
        assert_ne!(risk_color, reward_color);
    }

    #[test]
    fn minimap_interactable_roles_split_route_reward_and_support() {
        let progress = ChapterProgress {
            phase: ChapterPhase::Cache,
            ..default()
        };

        assert_eq!(
            minimap_interactable_role(&progress, InteractableKind::Cache),
            MinimapInteractableRole::Primary
        );
        assert_eq!(
            minimap_interactable_role(&progress, InteractableKind::Merchant),
            MinimapInteractableRole::Vendor
        );
        assert_eq!(
            minimap_interactable_role(&progress, InteractableKind::HealingWell),
            MinimapInteractableRole::Sustain
        );
        assert_eq!(
            minimap_interactable_role(&progress, InteractableKind::StormShrine),
            MinimapInteractableRole::Shrine
        );
        assert_eq!(
            minimap_interactable_role(&progress, InteractableKind::BloodObelisk),
            MinimapInteractableRole::Risk
        );
        assert_eq!(
            minimap_interactable_role(&progress, InteractableKind::EmberRift),
            MinimapInteractableRole::Reward
        );
        assert_eq!(
            minimap_interactable_role(&progress, InteractableKind::LorePage),
            MinimapInteractableRole::Lore
        );
    }

    #[test]
    fn victory_summaries_surface_challenges_mastery_and_milestones() {
        let stats = RunStats {
            elapsed_secs: 220.0,
            kills: 9,
            completion_bonus_gold: 315,
            completion_bonus_shards: 12,
            completion_bonus_essence: 7,
            chapter_clear_bonus_gold: 360,
            chapter_clear_bonus_shards: 12,
            chapter_clear_bonus_essence: 6,
            chapter_clear_reward_claimed: true,
            primal_caches: 1,
            primal_cache_bonus_gold: 350,
            primal_cache_bonus_shards: 18,
            primal_cache_bonus_essence: 17,
            primal_cache_items_claimed: 2,
            echo_keystones: 1,
            primal_cache_echo_items: 1,
            journey_score: 28,
            journey_bonus_gold: 260,
            journey_bonus_shards: 8,
            journey_bonus_essence: 5,
            journey_reward_claimed: true,
            renown_rank: 3,
            renown_bonus_gold: 105,
            renown_bonus_shards: 1,
            renown_bonus_essence: 0,
            renown_bonus_claimed: true,
            stash_bonus_gold: 45,
            stash_bonus_shards: 2,
            stash_bonus_essence: 1,
            stash_bonus_claimed: true,
            altar_seals: 4,
            altar_bonus_gold: 100,
            altar_bonus_shards: 2,
            altar_bonus_essence: 1,
            altar_bonus_claimed: true,
            gold: 800,
            ember_shards: 22,
            affix_essence: 11,
            best_valor_stacks: 5,
            boss_staggers: 2,
            malrec_soul_sigils: SOUL_SIGIL_CACHE_COST,
            malrec_soul_sigils_earned: SOUL_SIGIL_CACHE_COST,
            nemesis_kills: 2,
            ..default()
        };
        let difficulty = DifficultySettings {
            current: crate::Difficulty::Hell,
        };
        let mut milestones = Milestones::default();
        milestones.unlock(crate::milestone::MilestoneId::BossSlain);
        let mut challenges = ChapterChallenges::new();
        challenges.complete(crate::challenge::ChallengeId::SetBuilder);
        let mastery = ChapterMastery {
            points: 10,
            rank: crate::mastery::MasteryRank::Conqueror,
            reward_claimed: true,
            reward_gold: 420,
            reward_shards: 12,
            reward_essence: 8,
            boss_bonus_points: 2,
            latest: "Chapter mastery: Conqueror".to_string(),
        };

        let run_summary = victory_run_summary(&stats, &difficulty);
        let reward_summary = victory_reward_summary(&stats, &mastery);
        let progress_summary = victory_progress_summary(&challenges, &mastery, &milestones);
        let latest_summary = victory_latest_summary(&challenges, &mastery, &milestones);
        let next_summary = victory_next_difficulty_summary(difficulty.current);

        assert!(run_summary.contains("Run: Hell"));
        assert!(run_summary.contains("Rating S"));
        assert!(run_summary.contains("Boss Breaks 2"));
        assert!(run_summary.contains("Nemesis 2"));
        assert!(reward_summary.contains("Chapter +315 gold"));
        assert!(reward_summary.contains("Clear +360 gold +12 shards +6 essence"));
        assert!(reward_summary.contains("Primal Cache x2 item incl Echo +350 gold"));
        assert!(reward_summary.contains("Soul Sigils +2 (held 2, Trophy Cache ready)"));
        assert!(reward_summary.contains("Mastery +420 gold"));
        assert!(reward_summary.contains("Journey 28 pts +260 gold +8 shards +5 essence"));
        assert!(reward_summary.contains("Renown 3 start +105 gold +1 shards +0 essence"));
        assert!(reward_summary.contains("Stash start +45 gold +2 shards +1 essence"));
        assert!(reward_summary.contains("Altar 4/8 start +100 gold +2 shards +1 essence"));
        assert!(progress_summary.contains(&format!(
            "Challenges 1/{}",
            ChapterChallenges::new().challenges.len()
        )));
        assert!(progress_summary.contains("Conqueror"));
        assert!(progress_summary.contains("boss +2"));
        assert!(progress_summary.contains("+8 essence"));
        assert!(latest_summary.contains("Challenge complete: Set Builder"));
        assert!(latest_summary.contains("Keeper Slain"));
        assert!(latest_summary.contains("Chapter mastery: Conqueror"));
        assert!(next_summary.contains("push Torment"));
        assert!(next_summary.contains("Unlocked Torment"));
        assert!(next_summary.contains("Torment Brand"));
        assert!(
            victory_next_difficulty_summary(crate::Difficulty::Torment).contains("stay Torment")
        );
        assert!(
            victory_next_difficulty_summary(crate::Difficulty::Normal)
                .contains("Unlocked Nightmare")
        );
        assert!(
            victory_next_difficulty_summary(crate::Difficulty::Nightmare).contains("Unlocked Hell")
        );
        assert!(
            victory_next_difficulty_summary(crate::Difficulty::Torment)
                .contains("Torment endgame remains unlocked")
        );
    }

    #[test]
    fn victory_action_prompt_names_exact_buttons_and_route() {
        let normal = victory_action_prompt(crate::Difficulty::Normal);
        assert!(normal.contains("Space/Enter push Nightmare"));
        assert!(normal.contains("R replay Normal"));

        let nightmare = victory_action_prompt(crate::Difficulty::Nightmare);
        assert!(nightmare.contains("Space/Enter push Hell"));
        assert!(nightmare.contains("R replay Nightmare"));

        let torment = victory_action_prompt(crate::Difficulty::Torment);
        assert!(torment.contains("Space/Enter replay Torment endgame"));
        assert!(torment.contains("R replay current build"));
    }

    #[test]
    fn victory_reward_summary_reports_soul_sigil_cache_progress() {
        let stats = RunStats {
            malrec_soul_sigils: 3,
            malrec_soul_sigils_earned: 1,
            ..default()
        };
        let mastery = ChapterMastery::default();

        let reward_summary = victory_reward_summary(&stats, &mastery);

        assert!(reward_summary.contains("Soul Sigils +1 (held 3, Trophy Cache ready)"));
        let carried_summary = victory_reward_summary(
            &RunStats {
                malrec_soul_sigils: 1,
                ..default()
            },
            &mastery,
        );
        assert!(carried_summary.contains("Soul Sigils held 1 (cache 1/2)"));
        assert_eq!(
            soul_sigil_cache_status(SOUL_SIGIL_CACHE_COST),
            "Trophy Cache ready"
        );
        assert_eq!(soul_sigil_cache_status(1), "cache 1/2");
    }

    #[test]
    fn victory_reliquary_sigil_summary_surfaces_current_tier_and_next_steps() {
        let mut records = ChapterRecords::default();
        let normal = victory_reliquary_sigil_summary(
            crate::Difficulty::Hell,
            &RunStats::default(),
            "S",
            None,
            &records,
        );
        assert!(normal.contains("Reliquary Sigil: unlocks in Torment"));

        records.best_reliquary_sigil_tier = 2;
        records.lifetime_reliquary_sigil_clears = 1;
        let weak = victory_reliquary_sigil_summary(
            crate::Difficulty::Torment,
            &RunStats {
                boss_staggers: 1,
                ..default()
            },
            "B",
            Some(2),
            &records,
        );
        assert!(weak.contains("tier 2/12"));
        assert!(weak.contains("best 2 clears 1 perfect 0"));
        assert!(weak.contains("next earn S clear, break Malrec 4x, claim Primal Cache"));

        records.best_reliquary_sigil_tier = PERFECT_RELIQUARY_SIGIL_TIER;
        records.lifetime_perfect_sigil_clears = 1;
        let perfect = victory_reliquary_sigil_summary(
            crate::Difficulty::Torment,
            &RunStats {
                boss_staggers: 4,
                primal_caches: 1,
                ashen_threat_surges: 1,
                best_massacre_streak: 24,
                best_valor_stacks: 5,
                ..default()
            },
            "S",
            Some(PERFECT_RELIQUARY_SIGIL_TIER),
            &records,
        );
        assert!(perfect.contains("tier 10/12 perfect"));
        assert!(perfect.contains("push Paragon gem/full codex for tier 12"));
    }

    #[test]
    fn victory_recap_sections_group_clear_rewards_legacy_and_next_step() {
        let sections = victory_recap_sections([
            "Run: Hell | Rating S".to_string(),
            "Record: new best Hell".to_string(),
            "Legacy: Warden".to_string(),
            "Stash: 120g/6s/4e".to_string(),
            "Season Deeds: +1 claimed".to_string(),
            "Reward: Chapter +315 gold".to_string(),
            "Reliquary Sigil: tier 10/12 perfect".to_string(),
            "Chapter Clear: S | Keeper slain".to_string(),
            "Scorecard: Main cleared | Side complete".to_string(),
            "Story: Act I sealed".to_string(),
            "Quest Log: all rites complete".to_string(),
            "Journey: 28 pts".to_string(),
            "Progress: Conqueror".to_string(),
            "Latest: Challenge complete".to_string(),
            "Pursuit: farm Torment sigils".to_string(),
            "Next Run Plan: push Torment | start +40g +1s +0e".to_string(),
            "Next: push Torment | Unlocked Torment".to_string(),
        ]);

        assert_eq!(sections.len(), 4);
        assert_eq!(sections[0].title, "Run Result");
        assert!(
            sections[0]
                .lines
                .iter()
                .any(|line| line.contains("Run: Hell"))
        );
        assert!(
            sections[0]
                .lines
                .iter()
                .any(|line| line.contains("Chapter Clear"))
        );
        assert!(
            sections[0]
                .lines
                .iter()
                .any(|line| line.contains("Scorecard"))
        );
        assert!(
            sections[0]
                .lines
                .iter()
                .any(|line| line.contains("Reward: Chapter"))
        );
        assert_eq!(sections[1].title, "Chapter Progress");
        assert!(sections[1].lines.iter().any(|line| line.contains("Story:")));
        assert_eq!(sections[2].title, "Legacy");
        assert!(
            sections[2]
                .lines
                .iter()
                .any(|line| line.contains("Record: new best"))
        );
        assert!(
            sections[2]
                .lines
                .iter()
                .any(|line| line.contains("Reliquary Sigil"))
        );
        assert_eq!(sections[3].title, "Next Step");
        assert!(
            sections[3]
                .lines
                .iter()
                .any(|line| line.contains("Next Run Plan"))
        );
        assert!(
            sections[3]
                .lines
                .iter()
                .any(|line| line.contains("Next: push"))
        );
    }

    #[test]
    fn next_run_plan_combines_difficulty_stash_altar_and_deed() {
        let empty = ChapterRecords::default();
        let plan = next_run_plan_summary(&empty, crate::Difficulty::Normal);
        assert!(plan.contains("Next Run Plan: replay Normal"));
        assert!(plan.contains("Priority first clear"));
        assert!(plan.contains("Ordeal Ashen Echoes [baseline] x1.00: learn route"));
        assert!(plan.contains("no stash boost"));
        assert!(plan.contains("altar First Ember"));
        assert!(plan.contains("Next deed First Reliquary Clear"));

        let records = ChapterRecords {
            normal_clears: 1,
            stash_gold: 900,
            stash_shards: 16,
            stash_essence: 12,
            season_deeds_claimed: vec![SeasonDeedId::FirstClear],
            ..default()
        };
        let plan = next_run_plan_summary(&records, crate::Difficulty::Normal);
        assert!(plan.contains("push Nightmare"));
        assert!(plan.contains("Priority push Nightmare"));
        assert!(plan.contains("Ordeal Blood Tithe + Glass Relics [rising] x1.25"));
        assert!(plan.contains("focus elites, break shields"));
        assert!(plan.contains("start +45g +2s +2e"));
        assert!(plan.contains("altar Torment Brand"));
        assert!(plan.contains("Next deed Nightmare Veteran"));

        let s_hunter = ChapterRecords {
            normal_clears: 1,
            nightmare_clears: 1,
            hell_clears: 1,
            torment_clears: 1,
            best_chapter_clear_grade: "A".to_string(),
            best_journey_score: 42,
            season_deeds_claimed: vec![
                SeasonDeedId::FirstClear,
                SeasonDeedId::NightmareVeteran,
                SeasonDeedId::TormentClear,
            ],
            ..default()
        };
        let plan = next_run_plan_summary(&s_hunter, crate::Difficulty::Torment);
        assert!(plan.contains("Priority earn S clear"));
        assert!(plan.contains("Ordeal Torment Brand + Ashen Hunger [lethal] x1.67"));

        let journey_hunter = ChapterRecords {
            best_chapter_clear_grade: "S".to_string(),
            ..s_hunter
        };
        let plan = next_run_plan_summary(&journey_hunter, crate::Difficulty::Torment);
        assert!(plan.contains("Priority journey 42/60"));
    }

    #[test]
    fn next_run_ordeal_preview_surfaces_pressure_reward_and_counterplay() {
        let empty = ChapterRecords::default();
        assert_eq!(
            next_run_ordeal_preview_summary(&empty, crate::Difficulty::Normal),
            "Ordeal Ashen Echoes [baseline] x1.00: learn route"
        );

        let nightmare = ChapterRecords::default();
        let preview = next_run_ordeal_preview_summary(&nightmare, crate::Difficulty::Nightmare);
        assert!(preview.contains("Blood Tithe + Glass Relics"));
        assert!(preview.contains("[rising]"));
        assert!(preview.contains("x1.25"));
        assert!(preview.contains("focus elites, break shields"));

        let torment = ChapterRecords {
            torment_clears: 2,
            ..default()
        };
        let preview = next_run_ordeal_preview_summary(&torment, crate::Difficulty::Torment);
        assert!(preview.contains("Torment Brand + Glass Relics"));
        assert!(preview.contains("[lethal]"));
        assert!(preview.contains("x1.64"));
        assert!(preview.contains("focus elites, break shields"));
        assert_eq!(
            ordeal_pressure_label(&modifier_for_run(crate::Difficulty::Normal, 0)),
            "baseline"
        );
    }

    #[test]
    fn menu_profile_summary_surfaces_next_run_plan() {
        let empty = ChapterRecords::default();
        let summary = menu_profile_summary(&empty, crate::Difficulty::Normal, "audio on");
        assert!(summary.contains("HERO LOADOUT | Ashen Wanderer"));
        assert!(summary.contains("clears 0"));
        assert!(summary.contains("Profile: stash 0g/0s/0e"));
        assert!(summary.contains("Start: no stash grant yet"));
        assert!(summary.contains("audio on (M)"));
        assert!(
            summary
                .contains("Controls: Space/Enter start | Tab difficulty | F9 continue | M audio")
        );
        assert!(summary.contains("Next Run Plan: replay Normal"));
        assert!(summary.contains("Priority first clear"));
        assert!(summary.contains("Ordeal Ashen Echoes"));
        assert!(summary.contains("altar First Ember"));
        assert!(summary.contains("Next deed First Reliquary Clear"));
        let no_device_summary =
            menu_profile_summary(&empty, crate::Difficulty::Normal, "audio no device");
        assert!(no_device_summary.contains("audio no device (M)"));

        let records = ChapterRecords {
            normal_clears: 1,
            best_rating: "A".to_string(),
            stash_gold: 1_000,
            stash_shards: 16,
            stash_essence: 12,
            season_deeds_claimed: vec![SeasonDeedId::FirstClear],
            ..default()
        };
        let summary = menu_profile_summary(&records, crate::Difficulty::Normal, "audio muted");
        assert!(summary.contains("HERO LOADOUT | Reliquary Seeker"));
        assert!(summary.contains("clears 1"));
        assert!(summary.contains("highest Normal"));
        assert!(summary.contains("best A"));
        assert!(summary.contains("audio muted (M)"));
        assert!(summary.contains("Next Run Plan: push Nightmare"));
        assert!(summary.contains("Priority push Nightmare"));
        assert!(summary.contains("Ordeal Blood Tithe + Glass Relics"));
        assert!(summary.contains("start +50g +2s +2e"));
        assert!(summary.contains("Start: +50g +2s +2e from shared stash"));
        assert!(summary.contains("Nightmare Veteran"));

        let torment = ChapterRecords {
            normal_clears: 1,
            nightmare_clears: 1,
            hell_clears: 1,
            torment_clears: 2,
            best_reliquary_sigil_tier: PERFECT_RELIQUARY_SIGIL_TIER,
            lifetime_reliquary_sigil_clears: 2,
            lifetime_perfect_sigil_clears: 1,
            season_deeds_claimed: vec![
                SeasonDeedId::FirstClear,
                SeasonDeedId::NightmareVeteran,
                SeasonDeedId::TormentClear,
            ],
            ..default()
        };
        let summary = menu_profile_summary(&torment, crate::Difficulty::Torment, "audio on");
        assert!(summary.contains("HERO LOADOUT | Torment Vanquisher"));
        assert!(summary.contains("Next Run Plan: replay Torment"));
        assert!(summary.contains("Priority earn S clear"));
        assert!(summary.contains("Ordeal Torment Brand + Glass Relics"));
        assert!(summary.contains("altar Primal Spark"));
        assert!(summary.contains("Keeper Breaker"));
        assert_eq!(summary.lines().count(), 5);
    }

    #[test]
    fn main_menu_uses_concept_art_background_with_readability_scrim() {
        assert_eq!(
            main_menu_background_image_path(),
            "images/generated/bevy-open-arpg-concept.png"
        );
        assert!(main_menu_background_image_path().ends_with(".png"));
        assert!(main_menu_scrim_alpha() >= 0.55);
        assert!(main_menu_scrim_alpha() <= 0.75);
    }

    #[test]
    fn victory_pursuit_guides_next_chapter_clear_goal() {
        let progress = ChapterProgress {
            phase: ChapterPhase::Victory,
            boss_slain: true,
            seal_fragments: REQUIRED_SEAL_FRAGMENTS,
            ..default()
        };
        let challenges = ChapterChallenges::new();
        let milestones = Milestones::default();
        let mastery = ChapterMastery::default();

        let pursuit = victory_pursuit_summary(
            &progress,
            &RunStats::default(),
            &BountyProgress::default(),
            &BloodObelisk::default(),
            &EmberRift::default(),
            &StoryLog::default(),
            &challenges,
            &milestones,
            &mastery,
            crate::Difficulty::Normal,
        );

        assert!(pursuit.contains("S needs +8 optional and +9 story beats"));
        assert!(pursuit.contains("Focus side events 0/9 - Bounty Board cache"));
        assert!(pursuit.contains("Post-clear: salvage spares and prep next run"));
        assert!(pursuit.contains("Checklist:"));
        assert!(pursuit.contains("then push Nightmare"));
        assert!(pursuit.contains("review quest board"));
        assert!(pursuit.contains("Journey pursue Swift Clear"));
        assert!(pursuit.contains("push Nightmare"));

        let story = story_with_beats(&[
            crate::story::StoryBeat::CacheRevealed,
            crate::story::StoryBeat::SealRecovered,
            crate::story::StoryBeat::SanctumOpened,
            crate::story::StoryBeat::SealWardenSlain,
            crate::story::StoryBeat::AltarDemand,
            crate::story::StoryBeat::KeeperAwakened,
            crate::story::StoryBeat::KeeperBloodied,
            crate::story::StoryBeat::Victory,
            crate::story::StoryBeat::BloodObelisk,
            crate::story::StoryBeat::EmberRift,
        ]);
        let pursuit = victory_pursuit_summary(
            &progress,
            &RunStats::default(),
            &BountyProgress::default(),
            &BloodObelisk::default(),
            &EmberRift::default(),
            &story,
            &challenges,
            &milestones,
            &mastery,
            crate::Difficulty::Hell,
        );

        assert!(pursuit.contains("S needs +8 optional objectives"));
        assert!(pursuit.contains("Focus side events 0/9 - Bounty Board cache"));
        assert!(pursuit.contains("push Torment"));

        let partial_stats = RunStats {
            champion_pack_reward_claimed: true,
            breakables_smashed: BREAKER_TARGET_BREAKABLES,
            pylon_kills: PYLON_REAPER_KILLS,
            cursed_ambush_kills: CURSED_AMBUSH_KILLS,
            elite_affix_kills: AFFIX_HUNTER_KILLS,
            shrine_resonance_triggered: true,
            ..default()
        };
        let claimed_bounty = BountyProgress {
            reward_claimed: true,
            ..default()
        };
        let completed_obelisk = BloodObelisk {
            state: ObeliskState::Completed,
            ..default()
        };
        let completed_rift = EmberRift {
            state: RiftState::Completed,
            ..default()
        };
        let thin_story = story_with_beats(&[
            crate::story::StoryBeat::CacheRevealed,
            crate::story::StoryBeat::SealRecovered,
            crate::story::StoryBeat::SanctumOpened,
            crate::story::StoryBeat::SealWardenSlain,
            crate::story::StoryBeat::AltarDemand,
            crate::story::StoryBeat::KeeperAwakened,
            crate::story::StoryBeat::Victory,
        ]);
        let pursuit = victory_pursuit_summary(
            &progress,
            &partial_stats,
            &claimed_bounty,
            &completed_obelisk,
            &completed_rift,
            &thin_story,
            &challenges,
            &milestones,
            &mastery,
            crate::Difficulty::Hell,
        );

        assert!(pursuit.contains("Focus story beats 8/10"));
        assert!(pursuit.contains("S needs +2 story beats"));

        let stats = RunStats {
            champion_pack_reward_claimed: true,
            breakables_smashed: BREAKER_TARGET_BREAKABLES,
            pylon_kills: PYLON_REAPER_KILLS,
            cursed_ambush_kills: CURSED_AMBUSH_KILLS,
            elite_affix_kills: AFFIX_HUNTER_KILLS,
            shrine_resonance_triggered: true,
            ashen_threat_surges: ASHEN_THREAT_SURGE_TARGET,
            ..default()
        };
        let bounty = BountyProgress {
            reward_claimed: true,
            ..default()
        };
        let obelisk = BloodObelisk {
            state: ObeliskState::Completed,
            ..default()
        };
        let rift = EmberRift {
            state: RiftState::Completed,
            ..default()
        };
        let pursuit = victory_pursuit_summary(
            &progress,
            &stats,
            &bounty,
            &obelisk,
            &rift,
            &story,
            &challenges,
            &milestones,
            &mastery,
            crate::Difficulty::Torment,
        );

        assert!(pursuit.contains("S secured"));
        assert!(pursuit.contains("next Sealbearer at 12 (+140g +4s +2e)"));
        assert!(pursuit.contains("Post-clear: improve Torment clear for Primal Cache"));
        assert!(pursuit.contains("then replay Torment"));
        assert!(pursuit.contains("replay Torment"));
    }

    #[test]
    fn post_clear_upgrade_summary_prioritizes_cache_and_crafting_steps() {
        let trophy = RunStats {
            malrec_soul_sigils: SOUL_SIGIL_CACHE_COST,
            ..default()
        };
        assert_eq!(
            post_clear_upgrade_summary(&trophy, crate::Difficulty::Hell),
            "Post-clear: open 1 Trophy Cache (2 Soul Sigils, +120g +4s +3e +1 key)"
        );

        let trophy_hoard = RunStats {
            malrec_soul_sigils: SOUL_SIGIL_CACHE_COST * 2,
            ..default()
        };
        assert_eq!(
            post_clear_upgrade_summary(&trophy_hoard, crate::Difficulty::Torment),
            "Post-clear: open 2 Trophy Caches (4 Soul Sigils, +240g +8s +6e +2 keys)"
        );

        let primal = RunStats {
            primal_caches: 1,
            primal_cache_items_claimed: 2,
            primal_cache_echo_items: 1,
            ..default()
        };
        assert_eq!(
            post_clear_upgrade_summary(&primal, crate::Difficulty::Torment),
            "Post-clear: inspect Primal Cache x2 item incl Echo"
        );

        let echo = RunStats {
            echo_keystones: 1,
            ..default()
        };
        assert!(
            post_clear_upgrade_summary(&echo, crate::Difficulty::Nightmare)
                .contains("Echo Keystone x1")
        );

        let crafting = RunStats {
            gold: 220,
            ember_shards: 8,
            ..default()
        };
        assert_eq!(
            post_clear_upgrade_summary(&crafting, crate::Difficulty::Normal),
            "Post-clear: temper, socket, or expand at quartermaster"
        );
    }

    #[test]
    fn post_clear_checklist_combines_reward_handling_and_next_route() {
        let trophy = RunStats {
            malrec_soul_sigils: SOUL_SIGIL_CACHE_COST,
            ..default()
        };
        let checklist = post_clear_checklist_summary(&trophy, crate::Difficulty::Hell);

        assert!(checklist.contains("open 1 Trophy Cache"));
        assert!(checklist.contains("+120g +4s +3e +1 key"));
        assert!(checklist.contains("then push Torment"));
        assert!(checklist.contains("review quest board"));

        let trophy_hoard = RunStats {
            malrec_soul_sigils: SOUL_SIGIL_CACHE_COST * 2,
            ..default()
        };
        let checklist = post_clear_checklist_summary(&trophy_hoard, crate::Difficulty::Torment);
        assert!(checklist.contains("open 2 Trophy Caches"));
        assert!(checklist.contains("+240g +8s +6e +2 keys"));

        let torment = RunStats::default();
        let checklist = post_clear_checklist_summary(&torment, crate::Difficulty::Torment);

        assert!(checklist.contains("improve Torment clear"));
        assert!(checklist.contains("then replay Torment"));
    }

    #[test]
    fn victory_command_panel_prioritizes_claims_crafting_and_route() {
        let trophy = RunStats {
            malrec_soul_sigils: SOUL_SIGIL_CACHE_COST,
            ..default()
        };
        let records = ChapterRecords {
            normal_clears: 1,
            nightmare_clears: 1,
            hell_clears: 1,
            ..default()
        };
        let summary = victory_command_panel_summary(&trophy, &records, crate::Difficulty::Hell);

        assert!(summary.contains("POST-CLEAR COMMAND"));
        assert!(summary.contains("Space/Enter push Torment"));
        assert!(summary.contains("Claim: Post-clear: open 1 Trophy Cache"));
        assert!(summary.contains("Next: Push Torment"));
        assert!(summary.contains("Next Run Plan: push Torment"));
        assert_eq!(summary.lines().count(), 4);

        let crafting = RunStats {
            gold: 240,
            ember_shards: 8,
            ..default()
        };
        let summary = victory_command_panel_summary(
            &crafting,
            &ChapterRecords::default(),
            crate::Difficulty::Normal,
        );
        assert!(summary.contains("Claim: Post-clear: temper, socket, or expand at quartermaster"));
        assert!(summary.contains("Next: Push Nightmare"));

        let torment = victory_command_panel_summary(
            &RunStats::default(),
            &ChapterRecords {
                normal_clears: 1,
                nightmare_clears: 1,
                hell_clears: 1,
                torment_clears: 1,
                ..default()
            },
            crate::Difficulty::Torment,
        );
        assert!(torment.contains("Space/Enter replay Torment endgame"));
        assert!(torment.contains("Claim: Post-clear: improve Torment clear for Primal Cache"));
        assert!(torment.contains("Next: Replay Torment"));
    }

    #[test]
    fn chapter_records_keep_best_clear_per_difficulty() {
        let mut records = ChapterRecords::default();
        let slow_clear = RunStats {
            elapsed_secs: 360.0,
            kills: 8,
            gold: 320,
            ..default()
        };
        let slower_clear = RunStats {
            elapsed_secs: 420.0,
            kills: 20,
            gold: 900,
            ..default()
        };
        let fast_clear = RunStats {
            elapsed_secs: 240.0,
            kills: 9,
            gold: 450,
            ..default()
        };

        assert!(record_chapter_clear(
            &mut records,
            crate::Difficulty::Nightmare,
            &slow_clear
        ));
        assert!(!record_chapter_clear(
            &mut records,
            crate::Difficulty::Nightmare,
            &slower_clear
        ));
        assert!(record_chapter_clear(
            &mut records,
            crate::Difficulty::Nightmare,
            &fast_clear
        ));

        let record = records.get(crate::Difficulty::Nightmare).unwrap();
        assert_eq!(record.elapsed_secs, 240.0);
        assert_eq!(record.kills, 9);
        assert_eq!(record.gold, 450);
        assert_eq!(records.clears(crate::Difficulty::Nightmare), 0);
    }

    #[test]
    fn registering_chapter_clears_counts_every_victory() {
        let mut records = ChapterRecords::default();
        let fast_clear = RunStats {
            elapsed_secs: 210.0,
            kills: 9,
            gold: 420,
            ember_shards: 12,
            affix_essence: 4,
            journey_score: 14,
            boss_staggers: 1,
            primal_caches: 1,
            echo_keystones: 1,
            ashen_threat_surges: 2,
            treasure_vaults_opened: 1,
            best_massacre_streak: 12,
            best_valor_stacks: 3,
            ..default()
        };
        let slow_clear = RunStats {
            elapsed_secs: 420.0,
            kills: 20,
            gold: 900,
            ember_shards: 18,
            affix_essence: 7,
            journey_score: 48,
            boss_staggers: 2,
            primal_caches: 2,
            echo_keystones: 0,
            ashen_threat_surges: 1,
            treasure_vaults_opened: 1,
            affix_codex_mask: 0b11_1111,
            best_massacre_streak: 20,
            best_valor_stacks: 5,
            ..default()
        };

        assert!(register_chapter_clear(
            &mut records,
            crate::Difficulty::Hell,
            &fast_clear,
            OrdealAffix::AshenHunger
        ));
        assert!(!register_chapter_clear(
            &mut records,
            crate::Difficulty::Hell,
            &slow_clear,
            OrdealAffix::GlassRelics
        ));

        assert_eq!(records.clears(crate::Difficulty::Hell), 2);
        assert_eq!(records.total_clears(), 2);
        assert_eq!(records.lifetime_kills, 29);
        assert_eq!(
            records.cleared_ordeal_affix_mask,
            ordeal_affix_mask(OrdealAffix::AshenHunger)
                | ordeal_affix_mask(OrdealAffix::GlassRelics)
        );
        assert_eq!(records.lifetime_gold, 1320);
        assert_eq!(records.lifetime_shards, 30);
        assert_eq!(records.lifetime_essence, 11);
        assert_eq!(records.stash_gold, 132);
        assert_eq!(records.stash_shards, 7);
        assert_eq!(records.stash_essence, 3);
        assert_eq!(records.lifetime_boss_breaks, 3);
        assert_eq!(records.lifetime_primal_caches, 3);
        assert_eq!(records.lifetime_echo_keystones, 1);
        assert_eq!(records.lifetime_ashen_surges, 3);
        assert_eq!(records.lifetime_treasure_vaults, 2);
        assert_eq!(records.lifetime_affix_codexes, 1);
        assert_eq!(records.best_massacre_streak, 20);
        assert_eq!(records.best_valor_stacks, 5);
        assert_eq!(records.best_boss_breaks, 2);
        assert_eq!(records.best_rating, "S");
        assert_eq!(records.best_journey_score, 48);
        assert_eq!(records.best_journey_tier, "Ashen Champion");
        assert_eq!(
            records.highest_cleared_difficulty(),
            Some(crate::Difficulty::Hell)
        );
        let record = records.get(crate::Difficulty::Hell).unwrap();
        assert_eq!(record.elapsed_secs, 210.0);
    }

    #[test]
    fn lifetime_soul_sigils_count_newly_earned_not_carried_inventory() {
        let mut records = ChapterRecords::default();
        let clear = RunStats {
            malrec_soul_sigils: 5,
            malrec_soul_sigils_earned: 2,
            soul_sigil_caches: 1,
            ..default()
        };

        records.register_lifetime_stats(&clear, "B");
        records.register_lifetime_stats(&clear, "B");

        assert_eq!(records.lifetime_soul_sigils, 4);
        assert_eq!(records.lifetime_soul_sigil_caches, 2);
    }

    #[test]
    fn chapter_records_track_best_clear_grade_and_s_clears() {
        let mut records = ChapterRecords::default();

        records.register_chapter_clear_grade("B");
        records.register_chapter_clear_grade("S");
        records.register_chapter_clear_grade("A");
        records.register_chapter_clear_grade("S");

        assert_eq!(records.best_chapter_clear_grade_label(), "S");
        assert_eq!(records.lifetime_s_chapter_clears, 2);
        assert!(records.renown_score() >= clear_grade_rank("S") + 6);

        let profile = profile_summary(&records);
        let legacy = hero_legacy_summary(&records);

        assert_eq!(legacy_title(&records), "Ashen Wanderer");
        assert_eq!(legacy_badges_summary(&records), "S-Clear");
        assert!(profile.contains("clear S (S 2)"));
        assert!(legacy.contains("Clear S (S 2)"));
    }

    #[test]
    fn chapter_records_track_best_gem_rank_and_ascendant_clears() {
        let mut records = ChapterRecords::default();

        records.register_socketed_gem_clear(Some(SocketedGem {
            kind: crate::player::GemKind::Ruby,
            rank: 3,
        }));
        records.register_socketed_gem_clear(Some(SocketedGem {
            kind: crate::player::GemKind::Topaz,
            rank: 5,
        }));
        records.register_socketed_gem_clear(None);
        records.register_socketed_gem_clear(Some(SocketedGem {
            kind: crate::player::GemKind::Emerald,
            rank: 4,
        }));

        assert_eq!(records.best_socketed_gem_rank, 5);
        assert_eq!(records.lifetime_ascendant_gem_clears, 1);
        assert_eq!(records.lifetime_paragon_gem_clears, 0);
        assert!(records.renown_score() >= 7);

        let profile = profile_summary(&records);
        let legacy = hero_legacy_summary(&records);

        assert_eq!(legacy_badges_summary(&records), "Ascendant");
        assert!(profile.contains("gem best 5 asc 1 parag 0"));
        assert!(legacy.contains("Gem best 5 asc 1 parag 0"));
    }

    #[test]
    fn chapter_records_track_completed_boon_clears() {
        let mut records = ChapterRecords::default();

        records.register_completed_boon(ChapterBoonChoice::None);
        records.register_completed_boon(ChapterBoonChoice::CrimsonOath);
        records.register_completed_boon(ChapterBoonChoice::TitanWard);
        assert_eq!(records.completed_boon_mask.count_ones(), 2);
        assert!(!all_boons_completed(&records));

        records.register_completed_boon(ChapterBoonChoice::ArcaneMomentum);
        assert!(all_boons_completed(&records));
        assert!(records.renown_score() >= 6);

        let profile = profile_summary(&records);
        let legacy = hero_legacy_summary(&records);

        assert_eq!(legacy_badges_summary(&records), "Boonmaster");
        assert!(profile.contains("boons 3/3"));
        assert!(legacy.contains("Boons 3/3"));
    }

    #[test]
    fn chapter_records_track_legendary_codex_completion_clears() {
        let mut records = ChapterRecords::default();
        let mut codex = LegendaryCodex::default();

        codex.unlock(LegendaryPower::Emberbrand);
        codex.unlock(LegendaryPower::Frostbrand);
        records.register_legendary_codex_clear(&codex);
        assert_eq!(records.best_legendary_codex_powers, 2);
        assert_eq!(records.lifetime_legendary_codex_completions, 0);

        for power in LegendaryPower::ALL {
            codex.unlock(power);
        }
        records.register_legendary_codex_clear(&codex);
        records.register_legendary_codex_clear(&codex);

        assert_eq!(
            records.best_legendary_codex_powers,
            LegendaryPower::ALL.len() as u32
        );
        assert_eq!(records.lifetime_legendary_codex_completions, 2);
        assert_eq!(legacy_badges_summary(&records), "Codex");
        assert!(records.renown_score() >= 11);

        let profile = profile_summary(&records);
        let legacy = hero_legacy_summary(&records);

        assert!(profile.contains("codex 5/5 (2)"));
        assert!(legacy.contains("Codex 5/5 (2)"));
    }

    #[test]
    fn chapter_records_track_reliquary_sigil_inscriptions() {
        let mut records = ChapterRecords::default();
        let mut codex = LegendaryCodex::default();
        for power in LegendaryPower::ALL {
            codex.unlock(power);
        }
        let strong_torment = RunStats {
            boss_staggers: 4,
            primal_caches: 1,
            ashen_threat_surges: 1,
            best_massacre_streak: 24,
            best_valor_stacks: 5,
            ..default()
        };

        assert_eq!(
            records.register_reliquary_sigil_clear(
                crate::Difficulty::Hell,
                &strong_torment,
                "S",
                Some(SocketedGem {
                    kind: crate::player::GemKind::Ruby,
                    rank: 7,
                }),
                Some(&codex),
            ),
            None
        );
        assert_eq!(records.best_reliquary_sigil_tier, 0);

        assert_eq!(
            records.register_reliquary_sigil_clear(
                crate::Difficulty::Torment,
                &strong_torment,
                "S",
                Some(SocketedGem {
                    kind: crate::player::GemKind::Ruby,
                    rank: 7,
                }),
                Some(&codex),
            ),
            Some(12)
        );
        assert_eq!(records.best_reliquary_sigil_tier, 12);
        assert_eq!(records.lifetime_reliquary_sigil_clears, 1);
        assert_eq!(records.lifetime_perfect_sigil_clears, 1);
        assert!(records.renown_score() >= 19);

        let weak_torment = RunStats {
            boss_staggers: 1,
            ..default()
        };
        assert_eq!(
            records.register_reliquary_sigil_clear(
                crate::Difficulty::Torment,
                &weak_torment,
                "B",
                None,
                None,
            ),
            Some(2)
        );
        assert_eq!(records.best_reliquary_sigil_tier, 12);
        assert_eq!(records.lifetime_reliquary_sigil_clears, 2);
        assert_eq!(records.lifetime_perfect_sigil_clears, 1);
        assert_eq!(legacy_badges_summary(&records), "Sigil/PerfectSigil");

        let profile = profile_summary(&records);
        let legacy = hero_legacy_summary(&records);
        assert!(profile.contains("sigil best 12 clears 2 perfect 1"));
        assert!(legacy.contains("Sigil best 12 clears 2 perfect 1"));
    }

    #[test]
    fn legacy_titles_and_badges_track_profile_prestige() {
        let mut records = ChapterRecords::default();

        assert_eq!(legacy_title(&records), "Ashen Wanderer");
        assert_eq!(legacy_badges_summary(&records), "none");

        records.normal_clears = 1;
        assert_eq!(legacy_title(&records), "Reliquary Seeker");

        records.nightmare_clears = 1;
        assert_eq!(legacy_title(&records), "Nightmare Delver");

        records.hell_clears = 1;
        assert_eq!(legacy_title(&records), "Hellforged Champion");

        records.torment_clears = 1;
        assert_eq!(legacy_title(&records), "Torment Vanquisher");
        assert_eq!(legacy_badges_summary(&records), "Torment");

        records.torment_clears = 3;
        records.lifetime_s_chapter_clears = 2;
        records.lifetime_primal_caches = 1;
        records.best_socketed_gem_rank = 7;
        records.lifetime_ascendant_gem_clears = 1;
        records.lifetime_paragon_gem_clears = 1;
        records.best_journey_score = 60;
        records.completed_boon_mask = (1 << 3) - 1;
        records.best_legendary_codex_powers = LegendaryPower::ALL.len() as u32;
        records.lifetime_legendary_codex_completions = 1;
        records.completed_set_mask = ReliquarySet::ALL
            .into_iter()
            .fold(0, |mask, set| mask | set.mask());
        records.cleared_ordeal_affix_mask = (1 << 4) - 1;

        assert_eq!(legacy_title(&records), "Primal Paragon");
        assert_eq!(
            legacy_badges_summary(&records),
            "S-Clear/Torment/Primal/Ascendant/ParagonGem/Setmaster/Ordeal/Journey/Boonmaster/Codex"
        );

        records.best_reliquary_sigil_tier = PERFECT_RELIQUARY_SIGIL_TIER;
        records.lifetime_reliquary_sigil_clears = 3;
        records.lifetime_perfect_sigil_clears = 1;
        assert_eq!(
            legacy_badges_summary(&records),
            "S-Clear/Torment/Primal/Ascendant/ParagonGem/Setmaster/Ordeal/Journey/Sigil/PerfectSigil/Boonmaster/Codex"
        );

        records.season_deeds_claimed = ALL_SEASON_DEEDS.to_vec();
        assert_eq!(legacy_title(&records), "Season Conqueror");
        assert_eq!(
            legacy_badges_summary(&records),
            "S-Clear/Torment/Primal/Ascendant/ParagonGem/Setmaster/Ordeal/Journey/Sigil/PerfectSigil/Boonmaster/Codex/Season"
        );
    }

    #[test]
    fn profile_summary_surfaces_total_highest_and_torment_clears() {
        let records = ChapterRecords {
            normal_clears: 2,
            hell_clears: 1,
            torment_clears: 3,
            lifetime_kills: 88,
            lifetime_gold: 4300,
            lifetime_shards: 120,
            lifetime_essence: 44,
            best_rating: "A".to_string(),
            best_journey_score: 72,
            best_journey_tier: "Torment Conqueror".to_string(),
            lifetime_boss_breaks: 7,
            lifetime_primal_caches: 2,
            lifetime_primal_infusions: 1,
            lifetime_ashen_surges: 5,
            lifetime_nemesis_kills: 3,
            lifetime_treasure_vaults: 3,
            lifetime_affix_codexes: 2,
            best_chapter_clear_grade: "S".to_string(),
            lifetime_s_chapter_clears: 2,
            best_socketed_gem_rank: 7,
            lifetime_ascendant_gem_clears: 2,
            lifetime_paragon_gem_clears: 1,
            lifetime_soul_sigils: 3,
            lifetime_soul_sigil_caches: 1,
            stash_gold: 900,
            stash_shards: 24,
            stash_essence: 12,
            failed_runs: 2,
            soul_remnant_gold: 75,
            soul_remnant_shards: 3,
            soul_remnant_essence: 1,
            completed_boon_mask: (1 << 3) - 1,
            best_legendary_codex_powers: LegendaryPower::ALL.len() as u32,
            lifetime_legendary_codex_completions: 2,
            best_reliquary_sigil_tier: PERFECT_RELIQUARY_SIGIL_TIER,
            lifetime_reliquary_sigil_clears: 4,
            lifetime_perfect_sigil_clears: 2,
            ..default()
        };

        let summary = profile_summary(&records);

        assert!(summary.contains("title Primal Paragon"));
        assert!(summary.contains(
            "badges S-Clear/Torment/Primal/Ascendant/ParagonGem/Journey/Sigil/PerfectSigil/Boonmaster/Codex"
        ));
        assert!(summary.contains("6 clears"));
        assert!(summary.contains(&format!("renown {}", records.renown_rank())));
        assert!(summary.contains("altar 7/8"));
        assert!(summary.contains(&format!(
            "deeds {}/{}",
            records.completed_season_deeds(),
            ALL_SEASON_DEEDS.len()
        )));
        assert!(summary.contains("highest Torment"));
        assert!(summary.contains("best A"));
        assert!(summary.contains("clear S (S 2)"));
        assert!(summary.contains("gem best 7 asc 2 parag 1"));
        assert!(summary.contains("sigil best 10 clears 4 perfect 2"));
        assert!(summary.contains("boons 3/3"));
        assert!(summary.contains("codex 5/5 (2)"));
        assert!(summary.contains("journey Torment Conqueror 72"));
        assert!(summary.contains("88 kills"));
        assert!(summary.contains("4300 gold"));
        assert!(summary.contains("120 shards"));
        assert!(summary.contains("44 essence"));
        assert!(summary.contains("Stash 900g/24s/12e"));
        assert!(summary.contains("Remnants 2 runs 75g/3s/1e"));
        assert!(summary.contains("Boss Breaks 7"));
        assert!(summary.contains("Primal Caches 2"));
        assert!(summary.contains("Primal Infusions 1"));
        assert!(summary.contains("Ashen Surges 5"));
        assert!(summary.contains("Nemesis 3"));
        assert!(summary.contains("Treasure Vaults 3"));
        assert!(summary.contains("Bounties 0"));
        assert!(summary.contains("Sets 0/4"));
        assert!(summary.contains("Flawless 0"));
        assert!(summary.contains("Potionless 0"));
        assert!(summary.contains("Affix Codex 2"));
        assert!(summary.contains("Soul Sigils 3"));
        assert!(summary.contains("Trophy Caches 1"));
        assert!(summary.contains("Ordeals 0/4"));
        assert!(summary.contains("Torment 3"));
    }

    #[test]
    fn hero_legacy_summary_surfaces_long_term_combat_records() {
        let records = ChapterRecords {
            lifetime_boss_breaks: 9,
            lifetime_primal_caches: 3,
            lifetime_primal_infusions: 2,
            lifetime_echo_keystones: 2,
            lifetime_ashen_surges: 4,
            lifetime_nemesis_kills: 5,
            lifetime_treasure_vaults: 4,
            lifetime_affix_codexes: 2,
            lifetime_soul_sigils: 4,
            lifetime_soul_sigil_caches: 2,
            best_socketed_gem_rank: 7,
            lifetime_ascendant_gem_clears: 1,
            lifetime_paragon_gem_clears: 1,
            best_massacre_streak: 31,
            best_valor_stacks: 5,
            best_boss_breaks: 3,
            failed_runs: 4,
            stash_gold: 1_200,
            stash_shards: 32,
            stash_essence: 18,
            completed_boon_mask: (1 << 3) - 1,
            best_legendary_codex_powers: LegendaryPower::ALL.len() as u32,
            lifetime_legendary_codex_completions: 1,
            best_reliquary_sigil_tier: 8,
            lifetime_reliquary_sigil_clears: 2,
            ..default()
        };

        let summary = hero_legacy_summary(&records);
        let stash = profile_stash_summary(&records);

        assert!(summary.contains("Title Ashen Wanderer"));
        assert!(summary.contains("Badges Primal/Ascendant/ParagonGem/Sigil/Boonmaster/Codex"));
        assert!(summary.contains("Boss Breaks 9 (best 3)"));
        assert!(summary.contains("Primal Caches 3"));
        assert!(summary.contains("Primal Infusions 2"));
        assert!(summary.contains("Echo Keys 2"));
        assert!(summary.contains("Ashen Surges 4"));
        assert!(summary.contains("Nemesis 5"));
        assert!(summary.contains("Treasure Vaults 4"));
        assert!(summary.contains("Gem best 7 asc 1 parag 1"));
        assert!(summary.contains("Sigil best 8 clears 2 perfect 0"));
        assert!(summary.contains("Boons 3/3"));
        assert!(summary.contains("Codex 5/5 (1)"));
        assert!(summary.contains("Sets 0/4"));
        assert!(summary.contains("Flawless 0"));
        assert!(summary.contains("Potionless 0"));
        assert!(summary.contains("Affix Codex 2"));
        assert!(summary.contains("Soul Sigils 4"));
        assert!(summary.contains("Trophy Caches 2"));
        assert!(summary.contains("Soul Remnants 4"));
        assert!(summary.contains("Best Massacre 31"));
        assert!(summary.contains("Best Valor 5x"));
        assert!(stash.contains("1200 gold 32 shards 18 essence banked"));
        assert!(stash.contains("next run +60 gold +4 shards +3 essence"));
        assert!(stash.contains("Altar 4/8 seals"));
        assert!(stash.contains("start +100 gold +2 shards +1 essence"));
    }

    #[test]
    fn season_deeds_claim_once_and_bank_profile_rewards() {
        let mut records = ChapterRecords {
            normal_clears: 1,
            torment_clears: 1,
            lifetime_boss_breaks: 11,
            lifetime_primal_caches: 3,
            lifetime_primal_infusions: 3,
            lifetime_nemesis_kills: 3,
            lifetime_treasure_vaults: 3,
            lifetime_bounties_completed: 5,
            completed_set_mask: ReliquarySet::ALL
                .into_iter()
                .fold(0, |mask, set| mask | set.mask()),
            lifetime_flawless_clears: 3,
            lifetime_potionless_clears: 3,
            lifetime_s_chapter_clears: 2,
            best_socketed_gem_rank: 7,
            lifetime_ascendant_gem_clears: 2,
            lifetime_paragon_gem_clears: 2,
            lifetime_affix_codexes: 3,
            lifetime_soul_sigil_caches: 3,
            cleared_ordeal_affix_mask: (1 << 4) - 1,
            best_massacre_streak: 20,
            best_valor_stacks: 5,
            best_journey_score: 64,
            failed_runs: 3,
            stash_gold: 1_100,
            stash_shards: 25,
            stash_essence: 12,
            completed_boon_mask: (1 << 3) - 1,
            best_legendary_codex_powers: LegendaryPower::ALL.len() as u32,
            lifetime_legendary_codex_completions: 2,
            best_reliquary_sigil_tier: PERFECT_RELIQUARY_SIGIL_TIER,
            lifetime_reliquary_sigil_clears: 5,
            lifetime_perfect_sigil_clears: 2,
            ..default()
        };

        let reward = claim_completed_season_deeds(&mut records);
        let summary = season_deed_summary(&records, reward);

        assert_eq!(records.season_deeds_claimed.len(), 25);
        assert_eq!(legacy_title(&records), "Season Conqueror");
        assert!(legacy_badges_summary(&records).contains("Season"));
        assert_eq!(
            reward,
            ProfileStashBonus {
                gold: 6_520,
                shards: 215,
                essence: 136
            }
        );
        assert_eq!(records.stash_gold, 7_620);
        assert_eq!(records.stash_shards, 240);
        assert_eq!(records.stash_essence, 148);
        assert!(summary.contains("Season Deeds: 25/25 complete"));
        assert!(summary.contains("Claimed +6520 gold +215 shards +136 essence"));
        assert!(summary.contains("All season deeds claimed"));

        let second_reward = claim_completed_season_deeds(&mut records);
        assert_eq!(second_reward, ProfileStashBonus::default());
        assert_eq!(records.season_deeds_claimed.len(), 25);
    }

    #[test]
    fn season_deed_pursuit_summary_guides_next_profile_goal() {
        let mut records = ChapterRecords::default();

        assert_eq!(
            season_deed_pursuit_summary(&records),
            "Next deed First Reliquary Clear: clear the chapter once"
        );

        records.normal_clears = 1;
        assert!(
            season_deed_pursuit_summary(&records)
                .contains("Deed ready First Reliquary Clear +120g +4s +1e")
        );

        records.season_deeds_claimed = ALL_SEASON_DEEDS.to_vec();
        assert_eq!(season_deed_pursuit_summary(&records), "All deeds claimed");
    }

    #[test]
    fn writ_hunter_deed_tracks_claimed_bounty_caches() {
        let mut records = ChapterRecords {
            season_deeds_claimed: vec![
                SeasonDeedId::FirstClear,
                SeasonDeedId::NightmareVeteran,
                SeasonDeedId::TormentClear,
                SeasonDeedId::BossBreaker,
                SeasonDeedId::PrimalHunter,
                SeasonDeedId::PrimalArtificer,
                SeasonDeedId::AshboundVendetta,
                SeasonDeedId::VaultSeeker,
            ],
            lifetime_bounties_completed: 4,
            ..default()
        };

        assert_eq!(
            season_deed_pursuit_summary(&records),
            "Next deed Writ Hunter: claim five Bounty Board caches"
        );

        records.register_bounty_cache_claimed(true);
        assert!(
            season_deed_pursuit_summary(&records).contains("Deed ready Writ Hunter +240g +8s +5e")
        );

        let reward = claim_completed_season_deeds(&mut records);
        assert_eq!(
            reward,
            ProfileStashBonus {
                gold: 240,
                shards: 8,
                essence: 5
            }
        );
        assert!(
            records
                .season_deeds_claimed
                .contains(&SeasonDeedId::WritHunter)
        );
    }

    #[test]
    fn set_collector_deed_tracks_all_themed_resonance_clears() {
        let mut records = ChapterRecords {
            season_deeds_claimed: vec![
                SeasonDeedId::FirstClear,
                SeasonDeedId::NightmareVeteran,
                SeasonDeedId::TormentClear,
                SeasonDeedId::BossBreaker,
                SeasonDeedId::PrimalHunter,
                SeasonDeedId::PrimalArtificer,
                SeasonDeedId::AshboundVendetta,
                SeasonDeedId::VaultSeeker,
                SeasonDeedId::WritHunter,
            ],
            ..default()
        };

        records.register_completed_set(Some(ReliquarySet::Storm));
        records.register_completed_set(Some(ReliquarySet::Blood));
        records.register_completed_set(Some(ReliquarySet::Ashen));
        assert_eq!(
            season_deed_pursuit_summary(&records),
            "Next deed Set Collector: clear with all four themed set resonances"
        );

        records.register_completed_set(Some(ReliquarySet::Reliquary));
        assert!(
            season_deed_pursuit_summary(&records)
                .contains("Deed ready Set Collector +250g +8s +5e")
        );

        let reward = claim_completed_season_deeds(&mut records);
        assert_eq!(
            reward,
            ProfileStashBonus {
                gold: 250,
                shards: 8,
                essence: 5
            }
        );
        assert!(
            records
                .season_deeds_claimed
                .contains(&SeasonDeedId::SetCollector)
        );
    }

    #[test]
    fn flawless_champion_deed_tracks_deathless_clears() {
        let mut records = ChapterRecords {
            season_deeds_claimed: vec![
                SeasonDeedId::FirstClear,
                SeasonDeedId::NightmareVeteran,
                SeasonDeedId::TormentClear,
                SeasonDeedId::BossBreaker,
                SeasonDeedId::PrimalHunter,
                SeasonDeedId::PrimalArtificer,
                SeasonDeedId::AshboundVendetta,
                SeasonDeedId::VaultSeeker,
                SeasonDeedId::WritHunter,
                SeasonDeedId::SetCollector,
            ],
            lifetime_flawless_clears: 2,
            ..default()
        };

        records.register_flawless_clear(false);
        assert_eq!(records.lifetime_flawless_clears, 2);
        assert_eq!(
            season_deed_pursuit_summary(&records),
            "Next deed Flawless Champion: clear three times without soul ward revival"
        );

        records.register_flawless_clear(true);
        assert!(
            season_deed_pursuit_summary(&records)
                .contains("Deed ready Flawless Champion +280g +9s +6e")
        );

        let reward = claim_completed_season_deeds(&mut records);
        assert_eq!(
            reward,
            ProfileStashBonus {
                gold: 280,
                shards: 9,
                essence: 6
            }
        );
        assert!(
            records
                .season_deeds_claimed
                .contains(&SeasonDeedId::FlawlessChampion)
        );
    }

    #[test]
    fn untouched_ascetic_deed_tracks_potionless_clears() {
        let mut records = ChapterRecords {
            season_deeds_claimed: vec![
                SeasonDeedId::FirstClear,
                SeasonDeedId::NightmareVeteran,
                SeasonDeedId::TormentClear,
                SeasonDeedId::BossBreaker,
                SeasonDeedId::PrimalHunter,
                SeasonDeedId::PrimalArtificer,
                SeasonDeedId::AshboundVendetta,
                SeasonDeedId::VaultSeeker,
                SeasonDeedId::WritHunter,
                SeasonDeedId::SetCollector,
                SeasonDeedId::FlawlessChampion,
            ],
            lifetime_potionless_clears: 2,
            ..default()
        };

        records.register_potionless_clear(false);
        assert_eq!(records.lifetime_potionless_clears, 2);
        assert_eq!(
            season_deed_pursuit_summary(&records),
            "Next deed Untouched Ascetic: clear three times without drinking a potion"
        );

        records.register_potionless_clear(true);
        assert!(
            season_deed_pursuit_summary(&records)
                .contains("Deed ready Untouched Ascetic +260g +8s +6e")
        );

        let reward = claim_completed_season_deeds(&mut records);
        assert_eq!(
            reward,
            ProfileStashBonus {
                gold: 260,
                shards: 8,
                essence: 6
            }
        );
        assert!(
            records
                .season_deeds_claimed
                .contains(&SeasonDeedId::UntouchedAscetic)
        );
    }

    #[test]
    fn battle_trance_veteran_deed_tracks_massacre_and_valor_records() {
        let mut records = ChapterRecords {
            season_deeds_claimed: vec![
                SeasonDeedId::FirstClear,
                SeasonDeedId::NightmareVeteran,
                SeasonDeedId::TormentClear,
                SeasonDeedId::BossBreaker,
                SeasonDeedId::PrimalHunter,
                SeasonDeedId::PrimalArtificer,
                SeasonDeedId::AshboundVendetta,
                SeasonDeedId::VaultSeeker,
                SeasonDeedId::WritHunter,
                SeasonDeedId::SetCollector,
                SeasonDeedId::FlawlessChampion,
                SeasonDeedId::UntouchedAscetic,
            ],
            best_massacre_streak: 19,
            best_valor_stacks: 5,
            ..default()
        };

        assert_eq!(
            season_deed_pursuit_summary(&records),
            "Next deed Battle Trance Veteran: record a 20-kill massacre and five Valor stacks"
        );

        records.best_massacre_streak = 20;
        records.best_valor_stacks = 4;
        assert_eq!(
            season_deed_pursuit_summary(&records),
            "Next deed Battle Trance Veteran: record a 20-kill massacre and five Valor stacks"
        );

        records.best_valor_stacks = 5;
        assert!(
            season_deed_pursuit_summary(&records)
                .contains("Deed ready Battle Trance Veteran +270g +9s +6e")
        );

        let reward = claim_completed_season_deeds(&mut records);
        assert_eq!(
            reward,
            ProfileStashBonus {
                gold: 270,
                shards: 9,
                essence: 6
            }
        );
        assert!(
            records
                .season_deeds_claimed
                .contains(&SeasonDeedId::BattleTranceVeteran)
        );
    }

    #[test]
    fn chapter_clear_adept_deed_tracks_s_grade_clears() {
        let mut records = ChapterRecords {
            season_deeds_claimed: vec![
                SeasonDeedId::FirstClear,
                SeasonDeedId::NightmareVeteran,
                SeasonDeedId::TormentClear,
                SeasonDeedId::BossBreaker,
                SeasonDeedId::PrimalHunter,
                SeasonDeedId::PrimalArtificer,
                SeasonDeedId::AshboundVendetta,
                SeasonDeedId::VaultSeeker,
                SeasonDeedId::WritHunter,
                SeasonDeedId::SetCollector,
                SeasonDeedId::FlawlessChampion,
                SeasonDeedId::UntouchedAscetic,
                SeasonDeedId::BattleTranceVeteran,
            ],
            lifetime_s_chapter_clears: 1,
            ..default()
        };

        assert_eq!(
            season_deed_pursuit_summary(&records),
            "Next deed Chapter Clear Adept: earn two S-grade Chapter Clears"
        );

        records.register_chapter_clear_grade("A");
        assert_eq!(
            season_deed_pursuit_summary(&records),
            "Next deed Chapter Clear Adept: earn two S-grade Chapter Clears"
        );

        records.register_chapter_clear_grade("S");
        assert!(
            season_deed_pursuit_summary(&records)
                .contains("Deed ready Chapter Clear Adept +300g +10s +6e")
        );

        let reward = claim_completed_season_deeds(&mut records);
        assert_eq!(
            reward,
            ProfileStashBonus {
                gold: 300,
                shards: 10,
                essence: 6
            }
        );
        assert!(
            records
                .season_deeds_claimed
                .contains(&SeasonDeedId::ChapterClearAdept)
        );
    }

    #[test]
    fn ascendant_lapidary_deed_tracks_ascendant_gem_clears() {
        let mut records = ChapterRecords {
            season_deeds_claimed: vec![
                SeasonDeedId::FirstClear,
                SeasonDeedId::NightmareVeteran,
                SeasonDeedId::TormentClear,
                SeasonDeedId::BossBreaker,
                SeasonDeedId::PrimalHunter,
                SeasonDeedId::PrimalArtificer,
                SeasonDeedId::AshboundVendetta,
                SeasonDeedId::VaultSeeker,
                SeasonDeedId::WritHunter,
                SeasonDeedId::SetCollector,
                SeasonDeedId::FlawlessChampion,
                SeasonDeedId::UntouchedAscetic,
                SeasonDeedId::BattleTranceVeteran,
                SeasonDeedId::ChapterClearAdept,
            ],
            lifetime_ascendant_gem_clears: 1,
            ..default()
        };

        assert_eq!(
            season_deed_pursuit_summary(&records),
            "Next deed Ascendant Lapidary: clear twice with an Ascendant socketed gem"
        );

        records.register_socketed_gem_clear(Some(SocketedGem {
            kind: crate::player::GemKind::Amethyst,
            rank: 4,
        }));
        assert_eq!(
            season_deed_pursuit_summary(&records),
            "Next deed Ascendant Lapidary: clear twice with an Ascendant socketed gem"
        );

        records.register_socketed_gem_clear(Some(SocketedGem {
            kind: crate::player::GemKind::Amethyst,
            rank: 5,
        }));
        assert!(
            season_deed_pursuit_summary(&records)
                .contains("Deed ready Ascendant Lapidary +250g +8s +5e")
        );

        let reward = claim_completed_season_deeds(&mut records);
        assert_eq!(
            reward,
            ProfileStashBonus {
                gold: 250,
                shards: 8,
                essence: 5
            }
        );
        assert!(
            records
                .season_deeds_claimed
                .contains(&SeasonDeedId::AscendantLapidary)
        );
    }

    #[test]
    fn paragon_lapidary_deed_tracks_paragon_gem_clears() {
        let mut records = ChapterRecords {
            season_deeds_claimed: vec![
                SeasonDeedId::FirstClear,
                SeasonDeedId::NightmareVeteran,
                SeasonDeedId::TormentClear,
                SeasonDeedId::BossBreaker,
                SeasonDeedId::PrimalHunter,
                SeasonDeedId::PrimalArtificer,
                SeasonDeedId::AshboundVendetta,
                SeasonDeedId::VaultSeeker,
                SeasonDeedId::WritHunter,
                SeasonDeedId::SetCollector,
                SeasonDeedId::FlawlessChampion,
                SeasonDeedId::UntouchedAscetic,
                SeasonDeedId::BattleTranceVeteran,
                SeasonDeedId::ChapterClearAdept,
                SeasonDeedId::AscendantLapidary,
            ],
            lifetime_paragon_gem_clears: 1,
            ..default()
        };

        assert_eq!(
            season_deed_pursuit_summary(&records),
            "Next deed Paragon Lapidary: clear twice with a Paragon socketed gem"
        );

        records.register_socketed_gem_clear(Some(SocketedGem {
            kind: crate::player::GemKind::Ruby,
            rank: 6,
        }));
        assert_eq!(
            season_deed_pursuit_summary(&records),
            "Next deed Paragon Lapidary: clear twice with a Paragon socketed gem"
        );

        records.register_socketed_gem_clear(Some(SocketedGem {
            kind: crate::player::GemKind::Ruby,
            rank: 7,
        }));
        assert_eq!(records.lifetime_ascendant_gem_clears, 2);
        assert_eq!(records.lifetime_paragon_gem_clears, 2);
        assert!(
            season_deed_pursuit_summary(&records)
                .contains("Deed ready Paragon Lapidary +340g +11s +8e")
        );

        let reward = claim_completed_season_deeds(&mut records);
        assert_eq!(
            reward,
            ProfileStashBonus {
                gold: 340,
                shards: 11,
                essence: 8
            }
        );
        assert!(
            records
                .season_deeds_claimed
                .contains(&SeasonDeedId::ParagonLapidary)
        );
    }

    #[test]
    fn ordeal_conqueror_deed_tracks_all_rotating_affix_clears() {
        let mut records = ChapterRecords {
            season_deeds_claimed: vec![
                SeasonDeedId::FirstClear,
                SeasonDeedId::NightmareVeteran,
                SeasonDeedId::TormentClear,
                SeasonDeedId::BossBreaker,
                SeasonDeedId::PrimalHunter,
                SeasonDeedId::PrimalArtificer,
                SeasonDeedId::AshboundVendetta,
                SeasonDeedId::VaultSeeker,
                SeasonDeedId::WritHunter,
                SeasonDeedId::SetCollector,
                SeasonDeedId::FlawlessChampion,
                SeasonDeedId::UntouchedAscetic,
                SeasonDeedId::BattleTranceVeteran,
                SeasonDeedId::ChapterClearAdept,
                SeasonDeedId::AscendantLapidary,
                SeasonDeedId::ParagonLapidary,
                SeasonDeedId::AffixArchivist,
                SeasonDeedId::TrophyHoarder,
            ],
            ..default()
        };

        records.register_ordeal_affix_clear(OrdealAffix::AshenHunger);
        records.register_ordeal_affix_clear(OrdealAffix::GlassRelics);
        records.register_ordeal_affix_clear(OrdealAffix::TreasureFever);
        assert_eq!(
            season_deed_pursuit_summary(&records),
            "Next deed Ordeal Conqueror: clear every rotating ordeal affix"
        );

        records.register_ordeal_affix_clear(OrdealAffix::CinderVeins);
        assert!(
            season_deed_pursuit_summary(&records)
                .contains("Deed ready Ordeal Conqueror +300g +10s +7e")
        );

        let reward = claim_completed_season_deeds(&mut records);
        assert_eq!(
            reward,
            ProfileStashBonus {
                gold: 300,
                shards: 10,
                essence: 7
            }
        );
        assert!(
            records
                .season_deeds_claimed
                .contains(&SeasonDeedId::OrdealConqueror)
        );
    }

    #[test]
    fn trophy_hoarder_deed_tracks_malrec_trophy_cache_openings() {
        let mut records = ChapterRecords {
            normal_clears: 1,
            torment_clears: 1,
            lifetime_boss_breaks: 10,
            lifetime_primal_caches: 3,
            lifetime_primal_infusions: 3,
            lifetime_nemesis_kills: 3,
            lifetime_treasure_vaults: 3,
            lifetime_affix_codexes: 3,
            stash_gold: 1_000,
            stash_shards: 20,
            stash_essence: 10,
            failed_runs: 3,
            best_journey_score: 60,
            season_deeds_claimed: vec![
                SeasonDeedId::FirstClear,
                SeasonDeedId::NightmareVeteran,
                SeasonDeedId::TormentClear,
                SeasonDeedId::BossBreaker,
                SeasonDeedId::PrimalHunter,
                SeasonDeedId::PrimalArtificer,
                SeasonDeedId::AshboundVendetta,
                SeasonDeedId::VaultSeeker,
                SeasonDeedId::WritHunter,
                SeasonDeedId::SetCollector,
                SeasonDeedId::FlawlessChampion,
                SeasonDeedId::UntouchedAscetic,
                SeasonDeedId::BattleTranceVeteran,
                SeasonDeedId::ChapterClearAdept,
                SeasonDeedId::AscendantLapidary,
                SeasonDeedId::ParagonLapidary,
                SeasonDeedId::AffixArchivist,
                SeasonDeedId::ReliquaryBanker,
                SeasonDeedId::SoulboundResolve,
                SeasonDeedId::ConquerorPath,
            ],
            ..default()
        };

        assert_eq!(
            season_deed_pursuit_summary(&records),
            "Next deed Trophy Hoarder: open three Malrec Trophy Caches"
        );

        records.lifetime_soul_sigil_caches = 3;
        assert!(
            season_deed_pursuit_summary(&records)
                .contains("Deed ready Trophy Hoarder +260g +9s +6e")
        );

        let reward = claim_completed_season_deeds(&mut records);
        assert_eq!(
            reward,
            ProfileStashBonus {
                gold: 260,
                shards: 9,
                essence: 6
            }
        );
        assert!(
            records
                .season_deeds_claimed
                .contains(&SeasonDeedId::TrophyHoarder)
        );
    }

    #[test]
    fn profile_starting_bonus_grants_renown_and_stash_once() {
        let records = ChapterRecords {
            normal_clears: 2,
            hell_clears: 1,
            torment_clears: 1,
            lifetime_kills: 100,
            lifetime_gold: 5_000,
            lifetime_shards: 40,
            lifetime_essence: 20,
            best_journey_score: 72,
            best_journey_tier: "Torment Conqueror".to_string(),
            stash_gold: 1_000,
            stash_shards: 24,
            stash_essence: 12,
            ..default()
        };
        let mut stats = RunStats::default();

        assert_eq!(records.renown_rank(), 2);
        assert_eq!(
            renown_starting_bonus(records.renown_rank()),
            RenownStartingBonus {
                gold: 70,
                shards: 1,
                essence: 0
            }
        );
        assert_eq!(
            profile_stash_starting_bonus(&records),
            ProfileStashBonus {
                gold: 50,
                shards: 3,
                essence: 2
            }
        );
        assert!(apply_profile_starting_bonus_to_stats(&mut stats, &records));
        assert_eq!(stats.renown_rank, 2);
        assert_eq!(
            renown_run_summary(&stats),
            "Renown 2 start +70 gold +1 shards +0 essence"
        );
        assert_eq!(
            stash_run_summary(&stats),
            "Stash start +50 gold +3 shards +2 essence"
        );
        assert_eq!(
            altar_run_summary(&stats),
            "Altar 2/8 start +50 gold +1 shards +0 essence"
        );
        assert_eq!(stats.gold, 170);
        assert_eq!(stats.ember_shards, 5);
        assert_eq!(stats.affix_essence, 2);
        assert!(stats.renown_bonus_claimed);
        assert!(stats.stash_bonus_claimed);
        assert!(stats.altar_bonus_claimed);
        assert!(!apply_profile_starting_bonus_to_stats(&mut stats, &records));
        assert_eq!(stats.gold, 170);

        let empty_stats = RunStats::default();
        assert_eq!(renown_run_summary(&empty_stats), "Renown: none");
        assert_eq!(stash_run_summary(&empty_stats), "Stash: none");
        assert_eq!(altar_run_summary(&empty_stats), "Altar: none");
    }

    #[test]
    fn reliquary_altar_seals_track_account_milestones() {
        let mut records = ChapterRecords::default();
        assert_eq!(reliquary_altar_seals_unlocked(&records), 0);
        assert_eq!(
            reliquary_altar_summary(&records),
            "Altar 0/8 seals | next First Ember: clear the chapter once | start +0 gold +0 shards +0 essence"
        );

        records.normal_clears = 1;
        records.torment_clears = 1;
        records.lifetime_primal_caches = 1;
        records.best_socketed_gem_rank = 7;
        records.lifetime_legendary_codex_completions = 1;
        records.completed_boon_mask = (1 << 3) - 1;
        records.lifetime_perfect_sigil_clears = 1;
        records.season_deeds_claimed = ALL_SEASON_DEEDS.to_vec();

        assert_eq!(reliquary_altar_seals_unlocked(&records), 8);
        assert_eq!(
            reliquary_altar_starting_bonus(&records),
            ProfileStashBonus {
                gold: 200,
                shards: 4,
                essence: 2
            }
        );
        assert_eq!(
            reliquary_altar_summary(&records),
            "Altar 8/8 seals | all seals lit | start +200 gold +4 shards +2 essence"
        );
    }

    #[test]
    fn chapter_records_break_time_ties_with_rating_kills_and_gold() {
        let mut records = ChapterRecords::default();
        let first_clear = RunStats {
            elapsed_secs: 300.0,
            kills: 6,
            gold: 200,
            ..default()
        };
        let better_tie = RunStats {
            elapsed_secs: 300.02,
            kills: 8,
            gold: 250,
            ..default()
        };

        assert!(record_chapter_clear(
            &mut records,
            crate::Difficulty::Hell,
            &first_clear
        ));
        assert!(record_chapter_clear(
            &mut records,
            crate::Difficulty::Hell,
            &better_tie
        ));

        let record = records.get(crate::Difficulty::Hell).unwrap();
        assert_eq!(record.kills, 8);
        assert_eq!(record.gold, 250);
    }

    #[test]
    fn chapter_record_summaries_surface_menu_and_victory_copy() {
        let mut records = ChapterRecords::default();
        let stats = RunStats {
            elapsed_secs: 185.0,
            kills: 10,
            gold: 640,
            ..default()
        };

        let empty_menu_line = menu_difficulty_line(crate::Difficulty::Torment, &records);
        assert!(empty_menu_line.contains("Locked: clear Hell"));
        assert!(empty_menu_line.contains("Torment Brand + Cinder Veins"));
        assert!(empty_menu_line.contains("Clears: 0"));
        assert!(empty_menu_line.contains("no clear yet"));
        assert!(record_chapter_clear(
            &mut records,
            crate::Difficulty::Torment,
            &stats
        ));

        let menu_line = menu_difficulty_line(crate::Difficulty::Torment, &records);
        let victory_line = victory_record_summary(&records, crate::Difficulty::Torment, true);

        assert!(menu_line.contains("Difficulty: Torment"));
        assert!(menu_line.contains("Torment Brand + Cinder Veins"));
        assert!(menu_line.contains("Unlocked"));
        assert!(menu_line.contains("Clears: 0"));
        assert!(menu_line.contains("Record: S in 3:05"));
        assert!(profile_summary(&records).contains("0 clears"));
        assert!(profile_summary(&records).contains("best None"));
        assert!(profile_summary(&records).contains("journey None"));
        assert!(victory_line.contains("Record: new best Torment"));
        assert!(victory_line.contains("10 kills"));
    }

    #[test]
    fn profile_records_unlock_difficulties_in_order() {
        let mut records = ChapterRecords::default();

        assert_eq!(
            next_unlocked_difficulty(crate::Difficulty::Normal, &records),
            crate::Difficulty::Normal
        );
        assert!(difficulty_unlock_summary(crate::Difficulty::Hell, &records).contains("Nightmare"));

        records.normal_clears = 1;
        assert_eq!(
            next_unlocked_difficulty(crate::Difficulty::Normal, &records),
            crate::Difficulty::Nightmare
        );
        assert!(
            difficulty_unlock_summary(crate::Difficulty::Nightmare, &records).contains("Unlocked")
        );

        records.nightmare_clears = 1;
        assert_eq!(
            next_unlocked_difficulty(crate::Difficulty::Nightmare, &records),
            crate::Difficulty::Hell
        );

        records.hell_clears = 1;
        assert_eq!(
            next_unlocked_difficulty(crate::Difficulty::Hell, &records),
            crate::Difficulty::Torment
        );
    }

    #[test]
    fn chapter_record_profile_round_trips_to_disk() {
        let mut records = ChapterRecords::default();
        let stats = RunStats {
            elapsed_secs: 142.0,
            kills: 12,
            gold: 720,
            boss_staggers: 2,
            primal_caches: 1,
            primal_infusions: 1,
            echo_keystones: 1,
            ashen_threat_surges: 3,
            nemesis_kills: 2,
            treasure_vaults_opened: 1,
            affix_codex_mask: 0b11_1111,
            best_massacre_streak: 18,
            best_valor_stacks: 4,
            ..default()
        };
        assert!(register_chapter_clear(
            &mut records,
            crate::Difficulty::Torment,
            &stats,
            OrdealAffix::TreasureFever
        ));
        records.register_chapter_clear_grade("S");
        let deed_reward = claim_completed_season_deeds(&mut records);
        assert!(!deed_reward.is_empty());
        let path =
            std::env::temp_dir().join(format!("bevy_open_arpg_profile_{}.ron", std::process::id()));

        write_chapter_record_profile(&path, &records).unwrap();
        let restored = read_chapter_record_profile(&path).unwrap();
        let _ = std::fs::remove_file(&path);

        assert_eq!(restored, records);
        assert_eq!(restored.clears(crate::Difficulty::Torment), 1);
        assert_eq!(restored.lifetime_kills, 12);
        assert_eq!(restored.lifetime_gold, 720);
        assert_eq!(restored.stash_gold, 672);
        assert_eq!(restored.stash_shards, 20);
        assert_eq!(restored.stash_essence, 8);
        assert_eq!(restored.failed_runs, 0);
        assert_eq!(restored.soul_remnant_gold, 0);
        assert_eq!(restored.soul_remnant_shards, 0);
        assert_eq!(restored.soul_remnant_essence, 0);
        assert!(
            restored
                .season_deeds_claimed
                .contains(&SeasonDeedId::FirstClear)
        );
        assert!(
            restored
                .season_deeds_claimed
                .contains(&SeasonDeedId::TormentClear)
        );
        assert_eq!(restored.lifetime_boss_breaks, 2);
        assert_eq!(restored.lifetime_primal_caches, 1);
        assert_eq!(restored.lifetime_primal_infusions, 1);
        assert_eq!(restored.lifetime_echo_keystones, 1);
        assert_eq!(restored.lifetime_ashen_surges, 3);
        assert_eq!(restored.lifetime_nemesis_kills, 2);
        assert_eq!(restored.lifetime_treasure_vaults, 1);
        assert_eq!(restored.lifetime_affix_codexes, 1);
        assert_eq!(restored.best_chapter_clear_grade_label(), "S");
        assert_eq!(restored.lifetime_s_chapter_clears, 1);
        assert_eq!(restored.best_massacre_streak, 18);
        assert_eq!(restored.best_valor_stacks, 4);
        assert_eq!(restored.best_boss_breaks, 2);
        assert_eq!(restored.best_rating, "S");
        let record = restored.get(crate::Difficulty::Torment).unwrap();
        assert_eq!(record.rating, "S");
        assert_eq!(record.kills, 12);
    }

    #[test]
    fn pylon_reaper_summary_caps_at_challenge_requirement() {
        let mut stats = RunStats {
            pylon_kills: 2,
            ..default()
        };

        assert_eq!(pylon_reaper_summary(&stats), "Pylon Reaper 2/3");

        stats.pylon_kills = 8;

        assert_eq!(pylon_reaper_summary(&stats), "Pylon Reaper 3/3");
    }

    #[test]
    fn cursed_pact_summary_caps_at_challenge_requirement() {
        let mut stats = RunStats {
            cursed_ambush_kills: 2,
            ..default()
        };

        assert_eq!(cursed_pact_summary(&stats), "Cursed Pact 2/3");

        stats.cursed_ambush_kills = 8;

        assert_eq!(cursed_pact_summary(&stats), "Cursed Pact 3/3");
    }

    #[test]
    fn breaker_summary_caps_at_challenge_requirement() {
        let mut stats = RunStats {
            breakables_smashed: 2,
            ..default()
        };

        assert_eq!(breaker_summary(&stats), "Breaker 2/4");

        stats.breakables_smashed = 8;

        assert_eq!(breaker_summary(&stats), "Breaker 4/4");
    }

    #[test]
    fn affix_hunter_summary_caps_at_challenge_requirement() {
        let mut stats = RunStats {
            elite_affix_kills: 2,
            ..default()
        };

        assert_eq!(affix_hunter_summary(&stats), "Affix Hunter 2/3");

        stats.elite_affix_kills = 8;

        assert_eq!(affix_hunter_summary(&stats), "Affix Hunter 3/3");
    }

    #[test]
    fn affix_codex_summary_caps_distinct_affixes() {
        let mut stats = RunStats {
            affix_codex_mask: 0b1011,
            ..default()
        };

        assert_eq!(affix_codex_summary(&stats), "Affix Codex 3/6 next Molten");

        stats.affix_codex_mask = u16::MAX;

        assert_eq!(affix_codex_summary(&stats), "Affix Codex 6/6 complete");
    }

    #[test]
    fn nemesis_hunter_summary_caps_at_challenge_requirement() {
        let mut stats = RunStats::default();

        assert_eq!(nemesis_hunter_summary(&stats), "Nemesis Hunter 0/1");

        stats.nemesis_kills = 4;

        assert_eq!(nemesis_hunter_summary(&stats), "Nemesis Hunter 1/1");
    }

    #[test]
    fn ashen_threat_summary_caps_at_challenge_requirement() {
        let mut stats = RunStats {
            ashen_threat: 140,
            ashen_threat_surges: 2,
            ..default()
        };

        assert_eq!(
            ashen_threat_summary(&stats),
            "Ashen Threat 100/100 surge 2/3"
        );

        stats.ashen_threat_surges = 8;

        assert_eq!(
            ashen_threat_summary(&stats),
            "Ashen Threat 100/100 surge 3/3"
        );
    }

    #[test]
    fn rune_weaver_summary_counts_distinct_alternate_rune_hits() {
        let mut stats = RunStats {
            reap_dash_hits: 3,
            frost_nova_hits: 1,
            ..default()
        };

        assert_eq!(
            rune_weaver_summary(&stats),
            "Rune Weaver 2/3: C Hemorrhage then RMB hit"
        );

        stats.reap_dash_hits = 0;
        stats.hemorrhage_rupture_hits = 1;
        assert_eq!(
            rune_weaver_summary(&stats),
            "Rune Weaver 2/3: Z Reap then Q hit"
        );

        stats.reap_dash_hits = 3;

        stats.hemorrhage_rupture_hits = 1;

        assert_eq!(rune_weaver_summary(&stats), "Rune Weaver 3/3");
    }

    #[test]
    fn armory_adept_summary_caps_saved_loadouts() {
        let mut stats = RunStats {
            armory_loadouts_saved: 2,
            ..default()
        };

        assert_eq!(armory_adept_summary(&stats), "Armory Adept 2/3");

        stats.armory_loadouts_saved = 9;

        assert_eq!(armory_adept_summary(&stats), "Armory Adept 3/3");
    }

    #[test]
    fn portal_wayfarer_summary_tracks_return_state() {
        let mut stats = RunStats::default();

        assert_eq!(portal_wayfarer_summary(&stats), "Portal Wayfarer pending");

        stats.town_portal_returns = 1;

        assert_eq!(portal_wayfarer_summary(&stats), "Portal Wayfarer");
    }

    #[test]
    fn treasure_vault_summary_tracks_opened_state() {
        let mut stats = RunStats::default();

        assert_eq!(treasure_vault_summary(&stats), "Treasure Vault pending");

        stats.treasure_vaults_opened = 1;

        assert_eq!(treasure_vault_summary(&stats), "Treasure Vault opened");
    }

    #[test]
    fn glory_seeker_summary_caps_health_globes() {
        let mut stats = RunStats {
            health_globes_collected: 2,
            ..default()
        };

        assert_eq!(glory_seeker_summary(&stats), "Glory Seeker 2/3");

        stats.health_globes_collected = 9;

        assert_eq!(glory_seeker_summary(&stats), "Glory Seeker 3/3");
    }

    #[test]
    fn last_stand_summary_caps_clutch_potions() {
        let mut stats = RunStats {
            last_stand_potions: 2,
            ..default()
        };

        assert_eq!(last_stand_summary(&stats), "Last Stand 2/3");

        stats.last_stand_potions = 9;

        assert_eq!(last_stand_summary(&stats), "Last Stand 3/3");
    }

    #[test]
    fn loot_curator_summary_caps_filter_cycles() {
        let mut stats = RunStats {
            loot_filter_cycles: 2,
            ..default()
        };

        assert_eq!(loot_curator_summary(&stats), "Loot Curator 2/3");

        stats.loot_filter_cycles = 9;

        assert_eq!(loot_curator_summary(&stats), "Loot Curator 3/3");
    }

    #[test]
    fn codex_adept_summary_caps_attuned_kills() {
        let mut stats = RunStats::default();

        assert_eq!(
            codex_adept_summary(&stats),
            "Codex Adept 0/5: B attune power"
        );

        stats = RunStats {
            codex_attuned_kills: 3,
            ..default()
        };

        assert_eq!(
            codex_adept_summary(&stats),
            "Codex Adept 3/5: kill with B power"
        );

        stats.codex_attuned_kills = 12;

        assert_eq!(codex_adept_summary(&stats), "Codex Adept 5/5");
    }

    #[test]
    fn set_adept_summary_caps_set_resonance_kills() {
        let mut stats = RunStats::default();

        assert_eq!(
            set_adept_summary(&stats),
            "Set Adept 0/8: match weapon/charm"
        );

        stats = RunStats {
            set_resonance_kills: 6,
            ..default()
        };

        assert_eq!(set_adept_summary(&stats), "Set Adept 6/8: keep set kills");

        stats.set_resonance_kills = 12;

        assert_eq!(set_adept_summary(&stats), "Set Adept 8/8");
    }

    #[test]
    fn reliquary_set_tactic_guides_theme_matching_and_kill_progress() {
        let mut equipment = test_equipment("legendary");
        equipment.weapon_name = "Stormcall Reliquary Brand".to_string();
        let mut charm = Charm {
            name: "Moon Charm".to_string(),
            quality: "legendary".to_string(),
            damage_bonus: 4.0,
            crit_chance: 0.02,
        };
        let stats = RunStats::default();

        assert_eq!(
            reliquary_set_tactic(&equipment, &charm, &stats),
            "Set: match weapon/charm theme"
        );

        charm.name = "Stormglass Charm".to_string();
        assert_eq!(
            reliquary_set_tactic(&equipment, &charm, &stats),
            "Set Storm chain 0/8"
        );

        let progressed = RunStats {
            set_resonance_kills: 5,
            ..default()
        };
        assert_eq!(
            reliquary_set_tactic(&equipment, &charm, &progressed),
            "Set Storm chain 5/8"
        );

        let mastered = RunStats {
            set_resonance_kills: 12,
            ..default()
        };
        assert_eq!(
            reliquary_set_tactic(&equipment, &charm, &mastered),
            "Set Storm chain mastered"
        );
    }

    #[test]
    fn gem_adept_summary_counts_socketed_gem_kinds() {
        let mut stats = RunStats {
            ruby_socketed: true,
            emerald_socketed: true,
            ..default()
        };

        assert_eq!(gem_adept_summary(&stats), "Gem Adept 2/4");

        stats.amethyst_socketed = true;
        stats.topaz_socketed = true;

        assert_eq!(gem_adept_summary(&stats), "Gem Adept 4/4");
    }

    #[test]
    fn shrine_resonance_summary_tracks_triggered_state() {
        let mut stats = RunStats::default();

        assert_eq!(shrine_resonance_summary(&stats), "Shrine Resonance pending");

        stats.shrine_resonance_triggered = true;

        assert_eq!(shrine_resonance_summary(&stats), "Shrine Resonance");
    }

    #[test]
    fn chapter_route_summary_tracks_mainline_optional_and_threat() {
        let progress = ChapterProgress {
            phase: ChapterPhase::Ritual,
            enemies_slain: 5,
            seal_fragments: 2,
            ..default()
        };
        let stats = RunStats {
            ashen_threat: 82,
            champion_pack_reward_claimed: true,
            breakables_smashed: BREAKER_TARGET_BREAKABLES,
            pylon_kills: PYLON_REAPER_KILLS,
            cursed_ambush_kills: CURSED_AMBUSH_KILLS,
            elite_affix_kills: AFFIX_HUNTER_KILLS,
            shrine_resonance_triggered: true,
            ..default()
        };
        let bounty = BountyProgress {
            reward_claimed: true,
            ..default()
        };
        let obelisk = BloodObelisk {
            state: ObeliskState::Completed,
            ..default()
        };
        let rift = EmberRift {
            state: RiftState::Completed,
            ..default()
        };

        let summary = chapter_route_summary(&progress, &stats, &bounty, &obelisk, &rift);

        assert!(summary.contains(">Ritual<"));
        assert!(summary.contains("73%"));
        assert!(summary.contains("Optional 9/9"));
        assert!(summary.contains("Threat surging"));
        assert!(summary.contains("Next restore final seal 2/3"));
        assert!(summary.contains("PAYOFF Final seal"));
    }

    #[test]
    fn chapter_mainline_percent_advances_by_phase() {
        let breach = ChapterProgress {
            enemies_slain: 2,
            ..default()
        };
        let boss = ChapterProgress {
            phase: ChapterPhase::Boss,
            seal_fragments: REQUIRED_SEAL_FRAGMENTS,
            boss_spawned: true,
            ..default()
        };
        let victory = ChapterProgress {
            phase: ChapterPhase::Victory,
            boss_slain: true,
            ..default()
        };

        assert!((chapter_mainline_percent(&breach) - 13.333).abs() < 0.01);
        assert_eq!(chapter_mainline_percent(&boss), 80.0);
        assert_eq!(chapter_mainline_percent(&victory), 100.0);
        assert!(chapter_route_nodes(ChapterPhase::Boss).contains(">Keeper<"));
        assert_eq!(
            chapter_route_nodes_compact(ChapterPhase::Boss),
            "B>C>S>R>[K]"
        );
    }

    #[test]
    fn chapter_quest_log_surfaces_main_bounty_and_side_objectives() {
        let progress = ChapterProgress {
            phase: ChapterPhase::Ritual,
            seal_fragments: 2,
            ..default()
        };
        let stats = RunStats {
            champion_pack_kills: 3,
            breakables_smashed: BREAKER_TARGET_BREAKABLES,
            ..default()
        };
        let bounty = BountyProgress {
            current: 2,
            target: 3,
            ..default()
        };
        let obelisk = BloodObelisk {
            state: ObeliskState::Active,
            kills: 2,
            target_kills: crate::obelisk::OBELISK_TARGET_KILLS,
            remaining_secs: 18.4,
            reward_claimed: false,
        };
        let rift = EmberRift {
            state: RiftState::Completed,
            best_remaining_secs: crate::rift::RIFT_SWIFT_SEAL_SECS,
            ..default()
        };

        let summary = chapter_quest_log_summary(&progress, &stats, &bounty, &obelisk, &rift);

        assert!(summary.contains("Main 3/5 Final Seal 2 / 3 seals"));
        assert!(summary.contains("Bounty Slay enemies 2/3"));
        assert!(summary.contains("Champion 3/4"));
        assert!(summary.contains("Breaker done"));
        assert!(summary.contains("Obelisk 2/4 kills 18s"));
        assert!(summary.contains("Rift swift seal"));
    }

    #[test]
    fn chapter_quest_log_reports_completion_states() {
        let progress = ChapterProgress {
            phase: ChapterPhase::Victory,
            boss_slain: true,
            ..default()
        };
        let stats = RunStats {
            champion_pack_reward_claimed: true,
            breakables_smashed: BREAKER_TARGET_BREAKABLES,
            ..default()
        };
        let bounty = BountyProgress {
            completed: true,
            reward_claimed: true,
            ..default()
        };
        let obelisk = BloodObelisk {
            state: ObeliskState::Completed,
            ..default()
        };
        let rift = EmberRift {
            state: RiftState::Completed,
            ..default()
        };

        let summary = chapter_quest_log_summary(&progress, &stats, &bounty, &obelisk, &rift);

        assert!(summary.contains("Main Complete 5/5"));
        assert!(summary.contains("Bounty claimed"));
        assert!(summary.contains("Champion done"));
        assert!(summary.contains("Obelisk done"));
        assert!(summary.contains("Rift done"));
    }

    #[test]
    fn chapter_quest_board_surfaces_main_side_story_and_threat() {
        let progress = ChapterProgress {
            phase: ChapterPhase::Ritual,
            objective: "Extinguish the ember altar".to_string(),
            seal_fragments: 2,
            ..default()
        };
        let stats = RunStats {
            champion_pack_kills: 3,
            breakables_smashed: BREAKER_TARGET_BREAKABLES,
            pylon_kills: PYLON_REAPER_KILLS,
            cursed_ambush_kills: CURSED_AMBUSH_KILLS,
            ashen_threat: 84,
            ..default()
        };
        let bounty = BountyProgress {
            current: 2,
            target: 3,
            ..default()
        };
        let obelisk = BloodObelisk {
            state: ObeliskState::Active,
            kills: 2,
            target_kills: crate::obelisk::OBELISK_TARGET_KILLS,
            remaining_secs: 18.4,
            reward_claimed: false,
        };
        let rift = EmberRift {
            state: RiftState::Completed,
            best_remaining_secs: crate::rift::RIFT_SWIFT_SEAL_SECS,
            ..default()
        };
        let mut story = StoryLog::default();
        story.record(crate::story::StoryBeat::CursedShrine);
        story.record(crate::story::StoryBeat::BloodObelisk);
        story.record(crate::story::StoryBeat::EmberRift);

        let board = chapter_quest_board_text(&progress, &stats, &bounty, &obelisk, &rift, &story);

        for expected in [
            "QUEST BOARD",
            "Main",
            "Optional",
            "Story",
            "Threat",
            "Priority",
            "Bounty",
            "Champion",
            "Breaker",
            "Obelisk",
            "Rift",
            "Lore",
            "Ember Rift",
            "Next:",
            "Reward:",
        ] {
            assert!(board.contains(expected), "{expected} missing from {board}");
        }
        assert!(board.contains("restore final seal 2/3"));
        assert!(board.contains("Priority feed obelisk 2/4"));
        assert!(board.contains("obelisk gold, shards, essence"));
        assert!(board.lines().count() <= 6);
        for line in board.lines() {
            assert!(line.chars().count() <= 125, "{line}");
        }
    }

    #[test]
    fn chapter_quest_priority_guides_timed_claim_threat_and_cleanup() {
        let progress = ChapterProgress {
            phase: ChapterPhase::Ritual,
            seal_fragments: 2,
            ..default()
        };
        let stats = RunStats::default();
        let bounty = BountyProgress::default();
        let obelisk = BloodObelisk::default();
        let story = StoryLog::default();
        let active_rift = EmberRift {
            state: RiftState::Active,
            kills: 4,
            target_kills: crate::rift::RIFT_TARGET_KILLS,
            remaining_secs: crate::rift::RIFT_SWIFT_SEAL_SECS,
            ..default()
        };

        assert_eq!(
            chapter_quest_priority(&progress, &stats, &bounty, &obelisk, &active_rift, &story),
            "seal rift 4/6 for Echo"
        );

        let claimable_bounty = BountyProgress {
            completed: true,
            reward_claimed: false,
            ..default()
        };
        assert_eq!(
            chapter_quest_priority(
                &progress,
                &stats,
                &claimable_bounty,
                &obelisk,
                &EmberRift::default(),
                &story
            ),
            "claim bounty cache"
        );

        let threat_stats = RunStats {
            ashen_threat: 90,
            ..default()
        };
        assert_eq!(
            chapter_quest_priority(
                &progress,
                &threat_stats,
                &bounty,
                &obelisk,
                &EmberRift::default(),
                &story
            ),
            "stabilize threat surge"
        );

        assert_eq!(
            chapter_quest_priority(
                &progress,
                &stats,
                &bounty,
                &obelisk,
                &EmberRift::default(),
                &story
            ),
            "restore final seal 2/3"
        );

        let victory = ChapterProgress {
            phase: ChapterPhase::Victory,
            boss_slain: true,
            ..default()
        };
        assert!(
            chapter_quest_priority(
                &victory,
                &stats,
                &bounty,
                &obelisk,
                &EmberRift::default(),
                &story
            )
            .contains("side:")
        );
    }

    #[test]
    fn chapter_clear_summary_grades_main_side_and_story_completion() {
        let progress = ChapterProgress {
            phase: ChapterPhase::Ritual,
            seal_fragments: 2,
            ..default()
        };
        let story = StoryLog::default();
        let in_progress = chapter_clear_summary(
            &progress,
            &RunStats::default(),
            &BountyProgress::default(),
            &BloodObelisk::default(),
            &EmberRift::default(),
            &story,
        );

        assert!(in_progress.contains("Chapter Clear: Projected C"));
        assert!(in_progress.contains("Main 73%"));
        assert!(in_progress.contains("Optional 0/9"));
        assert!(in_progress.contains("Story 1/14"));
        assert!(in_progress.contains("S +8 side +9 story"));

        let progress = ChapterProgress {
            phase: ChapterPhase::Victory,
            boss_slain: true,
            seal_fragments: REQUIRED_SEAL_FRAGMENTS,
            ..default()
        };
        let stats = RunStats {
            champion_pack_reward_claimed: true,
            breakables_smashed: BREAKER_TARGET_BREAKABLES,
            pylon_kills: PYLON_REAPER_KILLS,
            cursed_ambush_kills: CURSED_AMBUSH_KILLS,
            elite_affix_kills: AFFIX_HUNTER_KILLS,
            shrine_resonance_triggered: true,
            ashen_threat_surges: ASHEN_THREAT_SURGE_TARGET,
            ..default()
        };
        let bounty = BountyProgress {
            reward_claimed: true,
            ..default()
        };
        let obelisk = BloodObelisk {
            state: ObeliskState::Completed,
            ..default()
        };
        let rift = EmberRift {
            state: RiftState::Completed,
            ..default()
        };
        let mut story = StoryLog::default();
        for beat in [
            crate::story::StoryBeat::CacheRevealed,
            crate::story::StoryBeat::SealRecovered,
            crate::story::StoryBeat::SanctumOpened,
            crate::story::StoryBeat::SealWardenSlain,
            crate::story::StoryBeat::AltarDemand,
            crate::story::StoryBeat::KeeperAwakened,
            crate::story::StoryBeat::KeeperBloodied,
            crate::story::StoryBeat::Victory,
            crate::story::StoryBeat::BloodObelisk,
        ] {
            story.record(beat);
        }

        let summary = chapter_clear_summary(&progress, &stats, &bounty, &obelisk, &rift, &story);

        assert!(summary.contains("Chapter Clear: S"));
        assert!(summary.contains("Main 100%"));
        assert!(summary.contains("Optional 9/9"));
        assert!(summary.contains("Story 10/14"));
        assert!(summary.contains("Threat surge mastered"));
        assert!(summary.contains("S secured"));
    }

    #[test]
    fn victory_scorecard_surfaces_missing_chapter_goals() {
        let progress = ChapterProgress {
            phase: ChapterPhase::Victory,
            boss_slain: true,
            seal_fragments: REQUIRED_SEAL_FRAGMENTS,
            ..default()
        };
        let scorecard = victory_scorecard_summary(
            &progress,
            &RunStats::default(),
            &BountyProgress::default(),
            &BloodObelisk::default(),
            &EmberRift::default(),
            &StoryLog::default(),
        );

        assert!(scorecard.contains("Scorecard: Main cleared"));
        assert!(scorecard.contains("Side 0/9 next Bounty Board cache"));
        assert!(scorecard.contains("Story 1/14 find lore sites"));
        assert!(scorecard.contains("Threat 0/3 surge"));
        assert!(scorecard.contains("S needs +8 side +9 story"));

        let stats = RunStats {
            champion_pack_reward_claimed: true,
            breakables_smashed: BREAKER_TARGET_BREAKABLES,
            pylon_kills: PYLON_REAPER_KILLS,
            cursed_ambush_kills: CURSED_AMBUSH_KILLS,
            elite_affix_kills: AFFIX_HUNTER_KILLS,
            shrine_resonance_triggered: true,
            ashen_threat_surges: ASHEN_THREAT_SURGE_TARGET,
            ..default()
        };
        let bounty = BountyProgress {
            reward_claimed: true,
            ..default()
        };
        let obelisk = BloodObelisk {
            state: ObeliskState::Completed,
            ..default()
        };
        let rift = EmberRift {
            state: RiftState::Completed,
            ..default()
        };
        let story = story_with_beats(&[
            crate::story::StoryBeat::CacheRevealed,
            crate::story::StoryBeat::SealRecovered,
            crate::story::StoryBeat::SanctumOpened,
            crate::story::StoryBeat::SealWardenSlain,
            crate::story::StoryBeat::AltarDemand,
            crate::story::StoryBeat::KeeperAwakened,
            crate::story::StoryBeat::KeeperBloodied,
            crate::story::StoryBeat::Victory,
            crate::story::StoryBeat::BloodObelisk,
            crate::story::StoryBeat::EmberRift,
        ]);

        let scorecard =
            victory_scorecard_summary(&progress, &stats, &bounty, &obelisk, &rift, &story);

        assert!(scorecard.contains("Side complete"));
        assert!(scorecard.contains("Story S-ready 11/14"));
        assert!(scorecard.contains("Threat mastered"));
        assert!(scorecard.contains("S route locked"));
    }

    #[test]
    fn chapter_clear_reward_pays_once_from_clear_grade() {
        let progress = ChapterProgress {
            phase: ChapterPhase::Victory,
            boss_slain: true,
            seal_fragments: REQUIRED_SEAL_FRAGMENTS,
            ..default()
        };
        let mut stats = RunStats {
            gold: 100,
            ember_shards: 5,
            affix_essence: 2,
            champion_pack_reward_claimed: true,
            breakables_smashed: BREAKER_TARGET_BREAKABLES,
            pylon_kills: PYLON_REAPER_KILLS,
            cursed_ambush_kills: CURSED_AMBUSH_KILLS,
            elite_affix_kills: AFFIX_HUNTER_KILLS,
            shrine_resonance_triggered: true,
            ..default()
        };
        let bounty = BountyProgress {
            reward_claimed: true,
            ..default()
        };
        let obelisk = BloodObelisk {
            state: ObeliskState::Completed,
            ..default()
        };
        let rift = EmberRift {
            state: RiftState::Completed,
            ..default()
        };
        let mut story = StoryLog::default();
        for beat in [
            crate::story::StoryBeat::CacheRevealed,
            crate::story::StoryBeat::SealRecovered,
            crate::story::StoryBeat::SanctumOpened,
            crate::story::StoryBeat::SealWardenSlain,
            crate::story::StoryBeat::AltarDemand,
            crate::story::StoryBeat::KeeperAwakened,
            crate::story::StoryBeat::KeeperBloodied,
            crate::story::StoryBeat::Victory,
            crate::story::StoryBeat::BloodObelisk,
        ] {
            story.record(beat);
        }

        let reward =
            award_chapter_clear_reward(&mut stats, &progress, &bounty, &obelisk, &rift, &story);

        assert_eq!(
            reward,
            ChapterClearReward {
                gold: 360,
                shards: 12,
                essence: 6,
            }
        );
        assert_eq!(stats.gold, 460);
        assert_eq!(stats.ember_shards, 17);
        assert_eq!(stats.affix_essence, 8);
        assert!(stats.chapter_clear_reward_claimed);
        assert_eq!(stats.chapter_clear_bonus_gold, 360);

        let second =
            award_chapter_clear_reward(&mut stats, &progress, &bounty, &obelisk, &rift, &story);

        assert_eq!(second, ChapterClearReward::default());
        assert_eq!(stats.gold, 460);
    }

    #[test]
    fn action_guide_prioritizes_survival_when_health_is_low() {
        let guide = action_guide_summary(ActionGuideInputs {
            progress: &ChapterProgress::default(),
            stats: &RunStats::default(),
            boon: &ChapterBoon::default(),
            bounty: &BountyProgress::default(),
            obelisk: &BloodObelisk::default(),
            rift: &EmberRift::default(),
            health: &test_health(25.0, 100.0),
            potions: &test_potions(1, 0.0),
            death_ward: &test_death_ward(0, 2, 40),
            surge: &test_surge(0, 0.0),
            level: &test_level(3),
            talents: &test_talents(0, 1, 1, 1),
            equipment: &test_equipment("common"),
            companion: &test_companion(CompanionStance::Vanguard, 9.0),
            boss: None,
        });

        assert!(guide.contains("Low health: F potion +45 HP; ward needs 40g"));
        assert!(guide.contains("Optional: Slay enemies 0/8"));

        let cooldown_stats = RunStats {
            gold: 80,
            ..default()
        };
        let cooldown_guide = action_guide_summary(ActionGuideInputs {
            progress: &ChapterProgress::default(),
            stats: &cooldown_stats,
            boon: &ChapterBoon::default(),
            bounty: &BountyProgress {
                completed: true,
                reward_claimed: true,
                ..default()
            },
            obelisk: &BloodObelisk::default(),
            rift: &EmberRift::default(),
            health: &test_health(20.0, 100.0),
            potions: &test_potions(1, 5.4),
            death_ward: &test_death_ward(1, 2, 40),
            surge: &test_surge(9, 0.0),
            level: &test_level(3),
            talents: &test_talents(0, 1, 1, 1),
            equipment: &test_equipment("common"),
            companion: &test_companion(CompanionStance::Vanguard, 0.0),
            boss: None,
        });

        assert!(cooldown_guide.contains("Low health: potion CD 6s; ward ready 1"));
        assert!(!cooldown_guide.contains("Press Y Surge"));

        let empty_guide = action_guide_summary(ActionGuideInputs {
            progress: &ChapterProgress::default(),
            stats: &RunStats::default(),
            boon: &ChapterBoon::default(),
            bounty: &BountyProgress {
                completed: true,
                reward_claimed: true,
                ..default()
            },
            obelisk: &BloodObelisk::default(),
            rift: &EmberRift::default(),
            health: &test_health(12.0, 100.0),
            potions: &test_potions(0, 0.0),
            death_ward: &test_death_ward(2, 2, 40),
            surge: &test_surge(0, 0.0),
            level: &test_level(3),
            talents: &test_talents(0, 1, 1, 1),
            equipment: &test_equipment("common"),
            companion: &test_companion(CompanionStance::Vanguard, 0.0),
            boss: None,
        });

        assert!(empty_guide.contains("Low health: potions empty; ward spent"));
    }

    #[test]
    fn action_guide_surfaces_boss_burst_and_active_events() {
        let boss = ChapterProgress {
            phase: ChapterPhase::Boss,
            ..default()
        };
        let boss_guide = action_guide_summary(ActionGuideInputs {
            progress: &boss,
            stats: &RunStats::default(),
            boon: &ChapterBoon::default(),
            bounty: &BountyProgress {
                completed: true,
                reward_claimed: true,
                ..default()
            },
            obelisk: &BloodObelisk::default(),
            rift: &EmberRift::default(),
            health: &test_health(90.0, 100.0),
            potions: &test_potions(1, 0.0),
            death_ward: &test_death_ward(0, 2, 40),
            surge: &test_surge(9, 0.0),
            level: &test_level(3),
            talents: &test_talents(0, 1, 1, 1),
            equipment: &test_equipment("common"),
            companion: &test_companion(CompanionStance::Vanguard, 9.0),
            boss: None,
        });

        assert!(boss_guide.contains("Boon: 4 Crimson base"));
        assert!(boss_guide.contains("Build rec choose 4 damage, 5 safety, 6 skills"));

        let chosen_boon = ChapterBoon {
            choice: ChapterBoonChoice::TitanWard,
            empowered: false,
            momentum_stacks: 0,
        };
        let boss_guide = action_guide_summary(ActionGuideInputs {
            progress: &boss,
            stats: &RunStats::default(),
            boon: &chosen_boon,
            bounty: &BountyProgress {
                completed: true,
                reward_claimed: true,
                ..default()
            },
            obelisk: &BloodObelisk::default(),
            rift: &EmberRift::default(),
            health: &test_health(90.0, 100.0),
            potions: &test_potions(1, 0.0),
            death_ward: &test_death_ward(0, 2, 40),
            surge: &test_surge(9, 0.0),
            level: &test_level(3),
            talents: &test_talents(0, 1, 1, 1),
            equipment: &test_equipment("common"),
            companion: &test_companion(CompanionStance::Guardian, 9.0),
            boss: None,
        });

        assert!(boss_guide.contains("Press Y Surge for Malrec burst"));

        let momentum_stats = RunStats {
            reliquary_momentum: 3,
            best_reliquary_momentum: 3,
            ..default()
        };
        let momentum_guide = action_guide_summary(ActionGuideInputs {
            progress: &boss,
            stats: &momentum_stats,
            boon: &chosen_boon,
            bounty: &BountyProgress {
                completed: true,
                reward_claimed: true,
                ..default()
            },
            obelisk: &BloodObelisk::default(),
            rift: &EmberRift::default(),
            health: &test_health(90.0, 100.0),
            potions: &test_potions(1, 0.0),
            death_ward: &test_death_ward(0, 2, 40),
            surge: &test_surge(0, 0.0),
            level: &test_level(3),
            talents: &test_talents(0, 1, 1, 1),
            equipment: &test_equipment("common"),
            companion: &test_companion(CompanionStance::Guardian, 9.0),
            boss: None,
        });
        assert!(momentum_guide.contains("Momentum 3x: spend Q/RMB/E burst on Malrec"));

        let active_rift = EmberRift {
            state: RiftState::Active,
            kills: 4,
            target_kills: crate::rift::RIFT_TARGET_KILLS,
            remaining_secs: 20.0,
            reward_claimed: false,
            best_remaining_secs: 0.0,
        };
        let rift_guide = action_guide_summary(ActionGuideInputs {
            progress: &ChapterProgress::default(),
            stats: &RunStats::default(),
            boon: &ChapterBoon::default(),
            bounty: &BountyProgress {
                completed: true,
                reward_claimed: true,
                ..default()
            },
            obelisk: &BloodObelisk::default(),
            rift: &active_rift,
            health: &test_health(90.0, 100.0),
            potions: &test_potions(1, 0.0),
            death_ward: &test_death_ward(0, 2, 40),
            surge: &test_surge(0, 0.0),
            level: &test_level(3),
            talents: &test_talents(0, 1, 1, 1),
            equipment: &test_equipment("common"),
            companion: &test_companion(CompanionStance::Seeker, 9.0),
            boss: None,
        });

        assert!(rift_guide.contains("Seal Ember Rift: 4/6 kills 20s"));

        let active_swift_rift = EmberRift {
            remaining_secs: crate::rift::RIFT_SWIFT_SEAL_SECS,
            ..active_rift
        };
        let swift_rift_guide = action_guide_summary(ActionGuideInputs {
            progress: &ChapterProgress::default(),
            stats: &RunStats::default(),
            boon: &ChapterBoon::default(),
            bounty: &BountyProgress {
                completed: true,
                reward_claimed: true,
                ..default()
            },
            obelisk: &BloodObelisk::default(),
            rift: &active_swift_rift,
            health: &test_health(90.0, 100.0),
            potions: &test_potions(1, 0.0),
            death_ward: &test_death_ward(0, 2, 40),
            surge: &test_surge(0, 0.0),
            level: &test_level(3),
            talents: &test_talents(0, 1, 1, 1),
            equipment: &test_equipment("common"),
            companion: &test_companion(CompanionStance::Seeker, 9.0),
            boss: None,
        });
        assert!(swift_rift_guide.contains("Seal Ember Rift: 4/6 kills 30s for Echo"));

        let active_obelisk = BloodObelisk {
            state: ObeliskState::Active,
            kills: 2,
            target_kills: crate::obelisk::OBELISK_TARGET_KILLS,
            remaining_secs: 12.4,
            reward_claimed: false,
        };
        let obelisk_guide = action_guide_summary(ActionGuideInputs {
            progress: &ChapterProgress::default(),
            stats: &RunStats::default(),
            boon: &ChapterBoon::default(),
            bounty: &BountyProgress {
                completed: true,
                reward_claimed: true,
                ..default()
            },
            obelisk: &active_obelisk,
            rift: &EmberRift::default(),
            health: &test_health(90.0, 100.0),
            potions: &test_potions(1, 0.0),
            death_ward: &test_death_ward(0, 2, 40),
            surge: &test_surge(0, 0.0),
            level: &test_level(3),
            talents: &test_talents(0, 1, 1, 1),
            equipment: &test_equipment("common"),
            companion: &test_companion(CompanionStance::Guardian, 9.0),
            boss: None,
        });
        assert!(obelisk_guide.contains("Feed Blood Obelisk: 2/4 kills 13s"));
    }

    #[test]
    fn action_guide_surfaces_malrec_readiness_before_unprepared_boss_pull() {
        let boss = ChapterProgress {
            phase: ChapterPhase::Boss,
            ..default()
        };
        let chosen_boon = ChapterBoon {
            choice: ChapterBoonChoice::TitanWard,
            empowered: false,
            momentum_stacks: 0,
        };

        let guide = action_guide_summary(ActionGuideInputs {
            progress: &boss,
            stats: &RunStats::default(),
            boon: &chosen_boon,
            bounty: &BountyProgress {
                completed: true,
                reward_claimed: true,
                ..default()
            },
            obelisk: &BloodObelisk::default(),
            rift: &EmberRift::default(),
            health: &test_health(52.0, 100.0),
            potions: &test_potions(0, 0.0),
            death_ward: &test_death_ward(0, 2, 40),
            surge: &test_surge(2, 0.0),
            level: &test_level(3),
            talents: &test_talents(0, 1, 1, 1),
            equipment: &test_equipment("rare"),
            companion: &test_companion(CompanionStance::Guardian, 9.0),
            boss: None,
        });

        assert!(guide.contains("Boss prep: heal to 65%+"));
        assert!(guide.contains("restock potion"));
        assert!(guide.contains("charge Surge 2/9"));
        assert!(guide.contains("equip legendary power"));
        assert!(guide.contains("Optional: champion pack 0/4"));
    }

    #[test]
    fn chapter_boon_tactical_prompt_recommends_from_build_and_survival_state() {
        let mut crit_equipment = test_equipment("legendary");
        crit_equipment.legendary_power = LegendaryPower::Stormbrand;
        crit_equipment.crit_chance = 0.16;
        let safe_health = test_health(95.0, 120.0);
        let potions = test_potions(2, 0.0);

        let crit_prompt = chapter_boon_tactical_prompt(
            &RunStats::default(),
            &safe_health,
            &potions,
            &crit_equipment,
        );
        assert!(crit_prompt.contains("Boon: 4 Crimson base"));
        assert!(crit_prompt.contains("Build rec 4 Crimson: scale crit power"));

        let mut sustain_equipment = test_equipment("legendary");
        sustain_equipment.legendary_power = LegendaryPower::Soulreaver;
        sustain_equipment.health_bonus = 26.0;
        assert!(
            chapter_boon_tactical_prompt(
                &RunStats::default(),
                &safe_health,
                &potions,
                &sustain_equipment
            )
            .contains("Build rec 5 Titan: survive Malrec")
        );

        let mut control_equipment = test_equipment("legendary");
        control_equipment.legendary_power = LegendaryPower::Frostbrand;
        control_equipment.health_bonus = 26.0;
        assert!(
            chapter_boon_tactical_prompt(
                &RunStats::default(),
                &safe_health,
                &potions,
                &control_equipment
            )
            .contains("Build rec 6 Arcane: more skill uptime")
        );

        let arcane_stats = RunStats {
            surge_kills: 2,
            ..default()
        };
        let mut plain_equipment = test_equipment("rare");
        plain_equipment.health_bonus = 24.0;
        assert!(
            chapter_boon_tactical_prompt(&arcane_stats, &safe_health, &potions, &plain_equipment)
                .contains("Build rec 6 Arcane: more skill uptime")
        );
        let momentum_stats = RunStats {
            reliquary_momentum: 4,
            best_reliquary_momentum: 4,
            ..default()
        };
        let momentum_prompt =
            chapter_boon_tactical_prompt(&momentum_stats, &safe_health, &potions, &crit_equipment);
        assert!(momentum_prompt.contains("6 Arcane empowered"));
        assert!(momentum_prompt.contains("Build rec 6 Arcane: more skill uptime"));

        let danger = test_health(45.0, 120.0);
        assert!(
            chapter_boon_tactical_prompt(&arcane_stats, &danger, &potions, &crit_equipment)
                .contains("Build rec 5 Titan: survive Malrec")
        );

        let empty_potions = test_potions(0, 0.0);
        assert!(
            chapter_boon_build_recommendation(
                &RunStats::default(),
                &safe_health,
                &empty_potions,
                &crit_equipment,
            )
            .contains("Titan")
        );
    }

    #[test]
    fn action_guide_surfaces_boss_phase_pressure() {
        let progress = ChapterProgress {
            phase: ChapterPhase::Boss,
            ..default()
        };
        let boon = ChapterBoon {
            choice: ChapterBoonChoice::TitanWard,
            empowered: false,
            momentum_stacks: 0,
        };
        let bounty = BountyProgress {
            completed: true,
            reward_claimed: true,
            ..default()
        };
        let stats = RunStats::default();
        let obelisk = BloodObelisk::default();
        let rift = EmberRift::default();
        let health = test_health(90.0, 100.0);
        let potions = test_potions(1, 0.0);
        let death_ward = test_death_ward(0, 2, 40);
        let surge = test_surge(0, 0.0);
        let equipment = test_equipment("common");
        let companion = test_companion(CompanionStance::Guardian, 9.0);

        let guide = action_guide_summary(ActionGuideInputs {
            progress: &progress,
            stats: &stats,
            boon: &boon,
            bounty: &bounty,
            obelisk: &obelisk,
            rift: &rift,
            health: &health,
            potions: &potions,
            death_ward: &death_ward,
            surge: &surge,
            level: &test_level(3),
            talents: &test_talents(0, 1, 1, 1),
            equipment: &equipment,
            companion: &companion,
            boss: Some(BossActionGuideState {
                phase_two_started: true,
                enrage_started: false,
                enrage_remaining_secs: 18.0,
                stagger_meter: 40.0,
                stagger_threshold: 125.0,
                staggered_remaining_secs: 0.0,
            }),
        });
        assert!(guide.contains("Phase two: build stagger 40/125; hold Y"));

        let guide = action_guide_summary(ActionGuideInputs {
            progress: &progress,
            stats: &stats,
            boon: &boon,
            bounty: &bounty,
            obelisk: &obelisk,
            rift: &rift,
            health: &health,
            potions: &potions,
            death_ward: &death_ward,
            surge: &surge,
            level: &test_level(3),
            talents: &test_talents(0, 1, 1, 1),
            equipment: &equipment,
            companion: &companion,
            boss: Some(BossActionGuideState {
                phase_two_started: true,
                enrage_started: false,
                enrage_remaining_secs: 5.2,
                stagger_meter: 92.0,
                stagger_threshold: 125.0,
                staggered_remaining_secs: 0.0,
            }),
        });
        assert!(guide.contains("Enrage soon: Q/E break 92/125 before fire"));

        let guide = action_guide_summary(ActionGuideInputs {
            progress: &progress,
            stats: &stats,
            boon: &boon,
            bounty: &bounty,
            obelisk: &obelisk,
            rift: &rift,
            health: &health,
            potions: &potions,
            death_ward: &death_ward,
            surge: &surge,
            level: &test_level(3),
            talents: &test_talents(0, 1, 1, 1),
            equipment: &equipment,
            companion: &companion,
            boss: Some(BossActionGuideState {
                phase_two_started: true,
                enrage_started: true,
                enrage_remaining_secs: 0.0,
                stagger_meter: 20.0,
                stagger_threshold: 125.0,
                staggered_remaining_secs: 0.0,
            }),
        });
        assert!(guide.contains("Ashen Enrage: kite vents, burst after slam"));

        let guide = action_guide_summary(ActionGuideInputs {
            progress: &progress,
            stats: &stats,
            boon: &boon,
            bounty: &bounty,
            obelisk: &obelisk,
            rift: &rift,
            health: &health,
            potions: &potions,
            death_ward: &death_ward,
            surge: &surge,
            level: &test_level(3),
            talents: &test_talents(0, 1, 1, 1),
            equipment: &equipment,
            companion: &companion,
            boss: Some(BossActionGuideState {
                phase_two_started: true,
                enrage_started: true,
                enrage_remaining_secs: 0.0,
                stagger_meter: 0.0,
                stagger_threshold: 125.0,
                staggered_remaining_secs: 2.4,
            }),
        });
        assert!(guide.contains("Execute window: Y/Q/E burst Malrec 3s"));
    }

    #[test]
    fn action_guide_reports_mainline_when_no_priority_override_exists() {
        let progress = ChapterProgress {
            phase: ChapterPhase::Ritual,
            seal_fragments: 2,
            ..default()
        };
        let stats = RunStats {
            champion_pack_kills: 1,
            ..default()
        };
        let guide = action_guide_summary(ActionGuideInputs {
            progress: &progress,
            stats: &stats,
            boon: &ChapterBoon::default(),
            bounty: &BountyProgress {
                completed: true,
                reward_claimed: true,
                ..default()
            },
            obelisk: &BloodObelisk::default(),
            rift: &EmberRift::default(),
            health: &test_health(90.0, 100.0),
            potions: &test_potions(1, 0.0),
            death_ward: &test_death_ward(0, 2, 40),
            surge: &test_surge(0, 0.0),
            level: &test_level(3),
            talents: &test_talents(0, 1, 1, 1),
            equipment: &test_equipment("common"),
            companion: &test_companion(CompanionStance::Vanguard, 9.0),
            boss: None,
        });

        assert!(guide.contains("Press Space at the ember altar"));
        assert!(guide.contains("Optional: champion pack 1/4"));
    }

    #[test]
    fn action_guide_optional_surfaces_timed_event_state() {
        let stats = RunStats {
            champion_pack_reward_claimed: true,
            breakables_smashed: BREAKER_TARGET_BREAKABLES,
            pylon_kills: PYLON_REAPER_KILLS,
            cursed_ambush_kills: CURSED_AMBUSH_KILLS,
            elite_affix_kills: AFFIX_HUNTER_KILLS,
            affix_codex_mask: u16::MAX,
            shrine_resonance_triggered: true,
            ..default()
        };
        let bounty = BountyProgress {
            completed: true,
            reward_claimed: true,
            ..default()
        };
        let completed_obelisk = BloodObelisk {
            state: ObeliskState::Completed,
            ..default()
        };
        let active_rift = EmberRift {
            state: RiftState::Active,
            kills: 3,
            target_kills: crate::rift::RIFT_TARGET_KILLS,
            remaining_secs: crate::rift::RIFT_SWIFT_SEAL_SECS,
            reward_claimed: false,
            best_remaining_secs: 0.0,
        };
        assert_eq!(
            action_guide_optional(&stats, &bounty, &completed_obelisk, &active_rift),
            "Optional: Rift 3/6 30s Echo"
        );

        let swift_rift = EmberRift {
            state: RiftState::Completed,
            best_remaining_secs: crate::rift::RIFT_SWIFT_SEAL_SECS,
            ..default()
        };
        assert_eq!(
            action_guide_optional(&stats, &bounty, &completed_obelisk, &swift_rift),
            "Optional: Rift swift sealed"
        );

        let failed_obelisk = BloodObelisk {
            state: ObeliskState::Failed,
            ..default()
        };
        assert_eq!(
            action_guide_optional(&stats, &bounty, &failed_obelisk, &EmberRift::default()),
            "Optional: open Ember Rift"
        );

        let failed_rift = EmberRift {
            state: RiftState::Failed,
            ..default()
        };
        assert_eq!(
            action_guide_optional(&stats, &bounty, &completed_obelisk, &failed_rift),
            "Optional: Rift failed, push boss"
        );
    }

    #[test]
    fn action_guide_optional_surfaces_next_affix_codex_target() {
        let stats = RunStats {
            champion_pack_reward_claimed: true,
            breakables_smashed: BREAKER_TARGET_BREAKABLES,
            pylon_kills: PYLON_REAPER_KILLS,
            cursed_ambush_kills: CURSED_AMBUSH_KILLS,
            elite_affix_kills: AFFIX_HUNTER_KILLS,
            affix_codex_mask: EnemyAffix::Frenzied.mask() | EnemyAffix::Vampiric.mask(),
            ..default()
        };
        let bounty = BountyProgress {
            completed: true,
            reward_claimed: true,
            ..default()
        };
        let completed_obelisk = BloodObelisk {
            state: ObeliskState::Completed,
            ..default()
        };
        let completed_rift = EmberRift {
            state: RiftState::Completed,
            ..default()
        };

        assert_eq!(
            next_optional_objective_focus(&stats, &bounty, &completed_obelisk, &completed_rift),
            "hunt Molten elite"
        );
        assert_eq!(
            action_guide_optional(&stats, &bounty, &completed_obelisk, &completed_rift),
            "Optional: Affix Codex 2/6 hunt Molten"
        );
    }

    #[test]
    fn action_guide_journey_hint_surfaces_next_tier_and_goal() {
        let hint = action_guide_journey_hint(
            &ChapterChallenges::default(),
            &Milestones::default(),
            &ChapterMastery::default(),
        )
        .expect("fresh chapter has a journey goal");

        assert!(hint.contains("Journey 0/12 Sealbearer"));
        assert!(hint.contains("Swift Clear"));
    }

    #[test]
    fn action_guide_journey_hint_appends_only_when_not_urgent() {
        let base = "Action Guide: Press Space at the ember altar | Optional: side events claimed, push mastery"
            .to_string();
        let progress = ChapterProgress {
            phase: ChapterPhase::Ritual,
            ..default()
        };
        let safe_health = test_health(90.0, 100.0);
        let low_health = test_health(20.0, 100.0);
        let target_info = TargetInfo::default();
        let obelisk = BloodObelisk {
            state: ObeliskState::Completed,
            ..default()
        };
        let rift = EmberRift {
            state: RiftState::Completed,
            ..default()
        };
        let surge = test_surge(0, 0.0);
        let challenges = ChapterChallenges::default();
        let milestones = Milestones::default();
        let mastery = ChapterMastery::default();

        let with_journey = action_guide_with_journey(
            base.clone(),
            JourneyActionGuideContext {
                progress: &progress,
                health: &safe_health,
                target_info: &target_info,
                obelisk: &obelisk,
                rift: &rift,
                surge: &surge,
                challenges: &challenges,
                milestones: &milestones,
                mastery: &mastery,
            },
        );
        assert!(with_journey.contains("Journey 0/12 Sealbearer"));

        let urgent = action_guide_with_journey(
            base.clone(),
            JourneyActionGuideContext {
                progress: &progress,
                health: &low_health,
                target_info: &target_info,
                obelisk: &obelisk,
                rift: &rift,
                surge: &surge,
                challenges: &challenges,
                milestones: &milestones,
                mastery: &mastery,
            },
        );
        assert_eq!(urgent, base);
    }

    #[test]
    fn action_guide_prioritizes_unspent_talent_points_before_routine_mainline() {
        let progress = ChapterProgress {
            phase: ChapterPhase::Ritual,
            seal_fragments: 2,
            ..default()
        };
        let talents = test_talents(1, 2, 1, 1);
        let guide = action_guide_summary(ActionGuideInputs {
            progress: &progress,
            stats: &RunStats {
                champion_pack_reward_claimed: true,
                breakables_smashed: BREAKER_TARGET_BREAKABLES,
                ..default()
            },
            boon: &ChapterBoon::default(),
            bounty: &BountyProgress {
                completed: true,
                reward_claimed: true,
                ..default()
            },
            obelisk: &BloodObelisk {
                state: ObeliskState::Completed,
                ..default()
            },
            rift: &EmberRift {
                state: RiftState::Completed,
                ..default()
            },
            health: &test_health(90.0, 100.0),
            potions: &test_potions(1, 0.0),
            death_ward: &test_death_ward(0, 2, 40),
            surge: &test_surge(0, 0.0),
            level: &test_level(4),
            talents: &talents,
            equipment: &test_equipment("common"),
            companion: &test_companion(CompanionStance::Vanguard, 9.0),
            boss: None,
        });

        assert!(guide.contains("Talent ready: K then 3 Focus"));
        assert!(guide.contains("spend 3 Focus for cooldowns"));
        assert!(!guide.contains("Press Space at the ember altar"));

        let low_health = action_guide_summary(ActionGuideInputs {
            progress: &progress,
            stats: &RunStats::default(),
            boon: &ChapterBoon::default(),
            bounty: &BountyProgress {
                completed: true,
                reward_claimed: true,
                ..default()
            },
            obelisk: &BloodObelisk::default(),
            rift: &EmberRift::default(),
            health: &test_health(20.0, 100.0),
            potions: &test_potions(1, 0.0),
            death_ward: &test_death_ward(0, 2, 40),
            surge: &test_surge(0, 0.0),
            level: &test_level(4),
            talents: &talents,
            equipment: &test_equipment("common"),
            companion: &test_companion(CompanionStance::Vanguard, 9.0),
            boss: None,
        });

        assert!(low_health.contains("Low health"));
        assert!(!low_health.contains("Talent ready"));
    }

    #[test]
    fn action_guide_surfaces_ready_sentinel_command_during_combat() {
        let progress = ChapterProgress {
            phase: ChapterPhase::Breach,
            enemies_slain: 1,
            ..default()
        };
        let guide = action_guide_summary(ActionGuideInputs {
            progress: &progress,
            stats: &RunStats::default(),
            boon: &ChapterBoon::default(),
            bounty: &BountyProgress {
                completed: true,
                reward_claimed: true,
                ..default()
            },
            obelisk: &BloodObelisk::default(),
            rift: &EmberRift::default(),
            health: &test_health(90.0, 100.0),
            potions: &test_potions(1, 0.0),
            death_ward: &test_death_ward(0, 2, 40),
            surge: &test_surge(0, 0.0),
            level: &test_level(3),
            talents: &test_talents(0, 1, 1, 1),
            equipment: &test_equipment("common"),
            companion: &test_companion(CompanionStance::Seeker, 0.0),
            boss: None,
        });

        assert!(guide.contains("Sentinel command ready: H Seeker sweep"));

        let cooldown_guide = action_guide_summary(ActionGuideInputs {
            progress: &progress,
            stats: &RunStats::default(),
            boon: &ChapterBoon::default(),
            bounty: &BountyProgress {
                completed: true,
                reward_claimed: true,
                ..default()
            },
            obelisk: &BloodObelisk::default(),
            rift: &EmberRift::default(),
            health: &test_health(90.0, 100.0),
            potions: &test_potions(1, 0.0),
            death_ward: &test_death_ward(0, 2, 40),
            surge: &test_surge(0, 0.0),
            level: &test_level(3),
            talents: &test_talents(0, 1, 1, 1),
            equipment: &test_equipment("common"),
            companion: &test_companion(CompanionStance::Seeker, 3.0),
            boss: None,
        });

        assert!(cooldown_guide.contains("Slay reliquary guards 1/3"));
    }

    #[test]
    fn action_guide_surfaces_active_progression_pursuits() {
        let equipment = test_equipment("rare");
        let massacre = RunStats {
            massacre_streak: 4,
            best_massacre_streak: 4,
            massacre_timer_secs: 2.4,
            ..default()
        };
        assert_eq!(
            action_guide_progression_pursuit(&massacre, &equipment),
            Some("Pursuit: keep Massacre 4/10 - kill before 3s".to_string())
        );

        let valor = RunStats {
            valor_stacks: 3,
            best_valor_stacks: 3,
            valor_timer_secs: 25.0,
            ..default()
        };
        assert_eq!(
            action_guide_progression_pursuit(&valor, &equipment),
            Some("Pursuit: chain elite Valor 3/5 - hunt champion/affix".to_string())
        );

        let runes = RunStats {
            reap_dash_hits: 2,
            frost_nova_hits: 1,
            ..default()
        };
        assert_eq!(
            action_guide_progression_pursuit(&runes, &equipment),
            Some("Pursuit: Rune Weaver 2/3 - C Hemorrhage then RMB hit".to_string())
        );

        let mut legendary = test_equipment("legendary");
        legendary.legendary_power = LegendaryPower::Stormbrand;
        let codex = RunStats {
            codex_attuned_kills: 3,
            ..default()
        };
        assert_eq!(
            action_guide_progression_pursuit(&codex, &legendary),
            Some("Pursuit: Codex Adept 3/5 - B attune, kill with Stormbrand".to_string())
        );

        let set = RunStats {
            set_resonance_kills: 4,
            ..default()
        };
        assert_eq!(
            action_guide_progression_pursuit(&set, &equipment),
            Some("Pursuit: Set Adept 4/8 - keep resonance kills".to_string())
        );
    }

    #[test]
    fn action_guide_uses_progression_pursuit_before_routine_mainline() {
        let progress = ChapterProgress {
            phase: ChapterPhase::Breach,
            enemies_slain: 1,
            ..default()
        };
        let stats = RunStats {
            massacre_streak: 3,
            best_massacre_streak: 3,
            massacre_timer_secs: 2.0,
            ..default()
        };
        let guide = action_guide_summary(ActionGuideInputs {
            progress: &progress,
            stats: &stats,
            boon: &ChapterBoon::default(),
            bounty: &BountyProgress {
                completed: true,
                reward_claimed: true,
                ..default()
            },
            obelisk: &BloodObelisk::default(),
            rift: &EmberRift::default(),
            health: &test_health(90.0, 100.0),
            potions: &test_potions(1, 0.0),
            death_ward: &test_death_ward(0, 2, 40),
            surge: &test_surge(0, 0.0),
            level: &test_level(3),
            talents: &test_talents(0, 1, 1, 1),
            equipment: &test_equipment("rare"),
            companion: &test_companion(CompanionStance::Vanguard, 3.0),
            boss: None,
        });

        assert!(guide.contains("Pursuit: keep Massacre 3/10"));
        assert!(!guide.contains("Slay reliquary guards 1/3"));
    }

    #[test]
    fn action_guide_surfaces_ready_quartermaster_crafting() {
        let trophy_stats = RunStats {
            malrec_soul_sigils: SOUL_SIGIL_CACHE_COST,
            champion_pack_reward_claimed: true,
            breakables_smashed: BREAKER_TARGET_BREAKABLES,
            ..default()
        };
        let guide = action_guide_summary(ActionGuideInputs {
            progress: &ChapterProgress::default(),
            stats: &trophy_stats,
            boon: &ChapterBoon::default(),
            bounty: &BountyProgress {
                completed: true,
                reward_claimed: true,
                ..default()
            },
            obelisk: &BloodObelisk {
                state: ObeliskState::Completed,
                ..default()
            },
            rift: &EmberRift {
                state: RiftState::Completed,
                ..default()
            },
            health: &test_health(90.0, 100.0),
            potions: &test_potions(1, 0.0),
            death_ward: &test_death_ward(0, 2, 40),
            surge: &test_surge(0, 0.0),
            level: &test_level(3),
            talents: &test_talents(0, 1, 1, 1),
            equipment: &test_equipment("common"),
            companion: &test_companion(CompanionStance::Vanguard, 9.0),
            boss: None,
        });

        assert!(guide.contains("Trophy cache ready: T then Space at quartermaster"));

        let primal_stats = RunStats {
            gold: 999,
            ember_shards: 999,
            affix_essence: 999,
            echo_keystones: 1,
            champion_pack_reward_claimed: true,
            breakables_smashed: BREAKER_TARGET_BREAKABLES,
            ..default()
        };
        let guide = action_guide_summary(ActionGuideInputs {
            progress: &ChapterProgress::default(),
            stats: &primal_stats,
            boon: &ChapterBoon::default(),
            bounty: &BountyProgress {
                completed: true,
                reward_claimed: true,
                ..default()
            },
            obelisk: &BloodObelisk {
                state: ObeliskState::Completed,
                ..default()
            },
            rift: &EmberRift {
                state: RiftState::Completed,
                ..default()
            },
            health: &test_health(90.0, 100.0),
            potions: &test_potions(1, 0.0),
            death_ward: &test_death_ward(0, 2, 40),
            surge: &test_surge(0, 0.0),
            level: &test_level(3),
            talents: &test_talents(0, 1, 1, 1),
            equipment: &test_equipment("ancient"),
            companion: &test_companion(CompanionStance::Vanguard, 9.0),
            boss: None,
        });

        assert!(guide.contains("Primal ready: T then Space at quartermaster"));
    }

    #[test]
    fn chapter_banner_copy_marks_major_phase_changes() {
        assert!(chapter_banner_text(ChapterPhase::Breach).contains("Breach"));
        assert!(chapter_banner_text(ChapterPhase::Cache).contains("Cache"));
        assert!(chapter_banner_text(ChapterPhase::Sanctum).contains("Ambush"));
        assert!(chapter_banner_text(ChapterPhase::Ritual).contains("Final Seal"));
        assert!(chapter_banner_text(ChapterPhase::Boss).contains("Malrec"));
        assert!(chapter_banner_text(ChapterPhase::Victory).contains("Cleansed"));
        for phase in [
            ChapterPhase::Breach,
            ChapterPhase::Cache,
            ChapterPhase::Sanctum,
            ChapterPhase::Ritual,
            ChapterPhase::Boss,
            ChapterPhase::Victory,
        ] {
            let copy = chapter_banner_text(phase);
            assert_eq!(copy.lines().count(), 2, "{copy}");
            assert!(copy.contains("Next:"), "{copy}");
            assert!(copy.contains("Reward:"), "{copy}");
            assert!(
                copy.lines().all(|line| line.chars().count() <= 74),
                "{copy}"
            );
        }
        assert!(
            chapter_banner_duration(ChapterPhase::Boss)
                > chapter_banner_duration(ChapterPhase::Cache)
        );
        assert!(
            chapter_banner_duration(ChapterPhase::Victory)
                > chapter_banner_duration(ChapterPhase::Ritual)
        );
    }

    #[test]
    fn story_comms_turn_story_beats_into_tactical_dialogue() {
        let cache = story_comms_from_event(&format!(
            "{}: {}",
            crate::story::StoryBeat::CacheRevealed.title(),
            crate::story::StoryBeat::CacheRevealed.line()
        ))
        .expect("story beat should create comms");

        assert!(cache.copy.contains("Steward Edda"));
        assert!(cache.copy.contains("Next: open the sealed cache"));
        assert_eq!(cache.copy.lines().count(), 3);
        assert_eq!(
            story_beat_from_event_text(&cache.copy),
            None,
            "rendered comms should not feed itself back as a story event"
        );

        let boss = story_comms_from_event(&format!(
            "{}: {}",
            crate::story::StoryBeat::KeeperBloodied.title(),
            crate::story::StoryBeat::KeeperBloodied.line()
        ))
        .expect("boss story beat should create comms");
        assert!(boss.copy.contains("Malrec, Keeper of Ash"));
        assert!(
            boss.copy
                .contains("Next: save burst for the stagger window")
        );
        assert!(boss.duration_secs > cache.duration_secs);

        let rift = story_comms_from_event(&format!(
            "{}: {}",
            crate::story::StoryBeat::EmberRift.title(),
            crate::story::StoryBeat::EmberRift.line()
        ))
        .expect("optional story beat should create comms");
        assert!(rift.copy.contains("Reliquary Whisper"));
        assert!(
            rift.copy
                .contains("Next: seal the rift before the timer breaks")
        );

        assert!(story_comms_from_event("Picked up legendary Iron Fang").is_none());
        assert!(
            reward_toast_from_event(&format!(
                "{}: {}",
                crate::story::StoryBeat::Victory.title(),
                crate::story::StoryBeat::Victory.line()
            ))
            .is_none()
        );
    }

    #[test]
    fn threat_tier_reports_pressure_thresholds() {
        let mut stats = RunStats::default();
        assert_eq!(threat_tier(&stats), "low");

        stats.ashen_threat = 40;
        assert_eq!(threat_tier(&stats), "rising");

        stats.ashen_threat = 75;
        assert_eq!(threat_tier(&stats), "surging");

        stats.ashen_threat_surges = ASHEN_THREAT_SURGE_TARGET;
        assert_eq!(threat_tier(&stats), "surge mastered");
    }

    #[test]
    fn game_over_summary_surfaces_soul_ward_failure() {
        let stats = RunStats {
            elapsed_secs: 481.0,
            kills: 6,
            gold: 20,
            ..default()
        };
        let difficulty = DifficultySettings {
            current: crate::Difficulty::Nightmare,
        };
        let report = DeathWardReport {
            revives_used: 2,
            max_revives: 2,
            gold_penalty: 60,
            gold_available: 20,
            reason: "revive charges exhausted".to_string(),
        };

        let stats_summary = game_over_stats_summary(&stats, &difficulty);
        let ward_summary = report.summary();
        let mut records = ChapterRecords::default();
        let remnant = claim_soul_remnant(&mut records, &stats, difficulty.current);
        let deed_reward = claim_completed_season_deeds(&mut records);
        let remnant_summary = soul_remnant_summary(remnant, deed_reward, &records);
        let progress = ChapterProgress {
            phase: ChapterPhase::Ritual,
            seal_fragments: 2,
            ..default()
        };
        let scorecard = death_scorecard_summary(&progress, &stats, &report, remnant);
        let pursuit =
            death_pursuit_summary(&progress, &stats, difficulty.current, &report, remnant);

        assert!(stats_summary.contains("Difficulty: Nightmare"));
        assert!(stats_summary.contains("Time: 8:01"));
        assert!(ward_summary.contains("revive charges exhausted"));
        assert!(ward_summary.contains("revives 2/2"));
        assert!(remnant_summary.contains("Soul Remnant: recovered"));
        assert!(remnant_summary.contains("failures 1"));
        assert!(scorecard.contains("Failure Scorecard"));
        assert!(scorecard.contains("Main 73%"));
        assert!(scorecard.contains("Ward short 40g"));
        assert!(scorecard.contains("Kills 6"));
        assert!(scorecard.contains("Remnant +"));
        assert!(scorecard.contains("Next: bank gold for ward"));
        assert!(pursuit.contains("ward short on gold"));
        assert!(pursuit.contains("use well/quartermaster before altar"));
        assert!(pursuit.contains("banked +"));
        assert!(pursuit.contains("replay Nightmare"));
        assert_eq!(records.failed_runs, 1);
        assert_eq!(records.stash_gold, remnant.gold);
    }

    #[test]
    fn death_pursuit_guides_early_and_boss_failures() {
        let report = DeathWardReport {
            revives_used: 2,
            max_revives: 2,
            gold_penalty: 40,
            gold_available: 80,
            reason: "revive charges exhausted".to_string(),
        };
        let early = death_pursuit_summary(
            &ChapterProgress::default(),
            &RunStats::default(),
            crate::Difficulty::Normal,
            &report,
            ProfileStashBonus::default(),
        );
        assert!(early.contains("ward charges spent"));
        assert!(early.contains("survive first room"));
        assert!(early.contains("no remnant banked"));
        let early_scorecard = death_scorecard_summary(
            &ChapterProgress::default(),
            &RunStats::default(),
            &report,
            ProfileStashBonus::default(),
        );
        assert!(early_scorecard.contains("Main 0%"));
        assert!(early_scorecard.contains("Ward spent 2/2"));
        assert!(early_scorecard.contains("No remnant"));
        assert!(early_scorecard.contains("Next: kite and potion early"));

        let boss_progress = ChapterProgress {
            phase: ChapterPhase::Boss,
            boss_spawned: true,
            seal_fragments: REQUIRED_SEAL_FRAGMENTS,
            ..default()
        };
        let boss_stats = RunStats {
            elapsed_secs: 420.0,
            kills: 34,
            boss_staggers: 1,
            ..default()
        };
        let boss = death_pursuit_summary(
            &boss_progress,
            &boss_stats,
            crate::Difficulty::Hell,
            &report,
            ProfileStashBonus {
                gold: 90,
                shards: 2,
                essence: 1,
            },
        );
        assert!(boss.contains("repeat break window"));
        assert!(boss.contains("banked +90g +2s +1e"));
        assert!(boss.contains("replay Hell"));
        let boss_scorecard = death_scorecard_summary(
            &boss_progress,
            &boss_stats,
            &report,
            ProfileStashBonus {
                gold: 90,
                shards: 2,
                essence: 1,
            },
        );
        assert!(boss_scorecard.contains("Main 80%"));
        assert!(boss_scorecard.contains("Boss breaks 1/2"));
        assert!(boss_scorecard.contains("Remnant +90g +2s +1e"));
        assert!(boss_scorecard.contains("Next: save Surge for execute"));
    }

    #[test]
    fn soul_remnant_deed_claims_on_failed_run_recap() {
        let stats = RunStats {
            elapsed_secs: 240.0,
            kills: 14,
            gold: 120,
            ember_shards: 5,
            affix_essence: 3,
            ..default()
        };
        let mut records = ChapterRecords {
            failed_runs: 2,
            soul_remnant_gold: 70,
            soul_remnant_shards: 2,
            soul_remnant_essence: 1,
            ..default()
        };

        let remnant = claim_soul_remnant(&mut records, &stats, crate::Difficulty::Nightmare);
        let deed_reward = claim_completed_season_deeds(&mut records);
        let summary = soul_remnant_summary(remnant, deed_reward, &records);

        assert_eq!(records.failed_runs, 3);
        assert!(
            records
                .season_deeds_claimed
                .contains(&SeasonDeedId::SoulboundResolve)
        );
        assert_eq!(
            deed_reward,
            ProfileStashBonus {
                gold: 200,
                shards: 7,
                essence: 3,
            }
        );
        assert!(summary.contains("deed +200 gold +7 shards +3 essence"));
        assert!(summary.contains("failures 3"));
    }

    #[test]
    fn boon_triad_deed_tracks_all_reliquary_boon_clears() {
        let mut records = ChapterRecords {
            season_deeds_claimed: ALL_SEASON_DEEDS
                .into_iter()
                .filter(|deed| {
                    !matches!(
                        deed,
                        SeasonDeedId::BoonTriad
                            | SeasonDeedId::CodexKeeper
                            | SeasonDeedId::ConquerorPath
                    )
                })
                .collect(),
            ..default()
        };

        assert_eq!(
            season_deed_pursuit_summary(&records),
            "Next deed Boon Triad: clear with all three Reliquary Boons"
        );

        records.register_completed_boon(ChapterBoonChoice::CrimsonOath);
        records.register_completed_boon(ChapterBoonChoice::TitanWard);
        assert_eq!(
            season_deed_pursuit_summary(&records),
            "Next deed Boon Triad: clear with all three Reliquary Boons"
        );

        records.register_completed_boon(ChapterBoonChoice::ArcaneMomentum);
        assert!(
            season_deed_pursuit_summary(&records).contains("Deed ready Boon Triad +260g +8s +5e")
        );

        let reward = claim_completed_season_deeds(&mut records);
        assert_eq!(
            reward,
            ProfileStashBonus {
                gold: 260,
                shards: 8,
                essence: 5
            }
        );
        assert!(
            records
                .season_deeds_claimed
                .contains(&SeasonDeedId::BoonTriad)
        );
    }

    #[test]
    fn codex_keeper_deed_tracks_complete_legendary_codex_clears() {
        let mut records = ChapterRecords {
            season_deeds_claimed: ALL_SEASON_DEEDS
                .into_iter()
                .filter(|deed| {
                    !matches!(
                        deed,
                        SeasonDeedId::CodexKeeper | SeasonDeedId::ConquerorPath
                    )
                })
                .collect(),
            lifetime_legendary_codex_completions: 1,
            ..default()
        };
        let mut codex = LegendaryCodex::default();

        codex.unlock(LegendaryPower::Emberbrand);
        codex.unlock(LegendaryPower::Frostbrand);
        records.register_legendary_codex_clear(&codex);
        assert_eq!(
            season_deed_pursuit_summary(&records),
            "Next deed Codex Keeper: clear twice with a complete legendary codex"
        );

        for power in LegendaryPower::ALL {
            codex.unlock(power);
        }
        records.register_legendary_codex_clear(&codex);
        assert!(
            season_deed_pursuit_summary(&records).contains("Deed ready Codex Keeper +280g +9s +6e")
        );

        let reward = claim_completed_season_deeds(&mut records);
        assert_eq!(
            reward,
            ProfileStashBonus {
                gold: 280,
                shards: 9,
                essence: 6
            }
        );
        assert!(
            records
                .season_deeds_claimed
                .contains(&SeasonDeedId::CodexKeeper)
        );
    }

    #[test]
    fn sigil_engraver_deed_tracks_perfect_reliquary_sigil_clears() {
        let mut records = ChapterRecords {
            season_deeds_claimed: ALL_SEASON_DEEDS
                .into_iter()
                .filter(|deed| !matches!(deed, SeasonDeedId::SigilEngraver))
                .collect(),
            lifetime_perfect_sigil_clears: 1,
            ..default()
        };

        assert_eq!(
            season_deed_pursuit_summary(&records),
            "Next deed Sigil Engraver: complete two perfect Reliquary Sigil inscriptions"
        );

        records.best_reliquary_sigil_tier = PERFECT_RELIQUARY_SIGIL_TIER;
        records.lifetime_reliquary_sigil_clears = 3;
        records.lifetime_perfect_sigil_clears = 2;
        assert!(
            season_deed_pursuit_summary(&records)
                .contains("Deed ready Sigil Engraver +420g +14s +10e")
        );

        let reward = claim_completed_season_deeds(&mut records);
        assert_eq!(
            reward,
            ProfileStashBonus {
                gold: 420,
                shards: 14,
                essence: 10
            }
        );
        assert!(
            records
                .season_deeds_claimed
                .contains(&SeasonDeedId::SigilEngraver)
        );
    }

    #[test]
    fn soul_remnants_reward_failed_progress_without_instant_death_farming() {
        let empty = soul_remnant_reward(&RunStats::default(), crate::Difficulty::Torment);
        assert!(empty.is_empty());

        let stats = RunStats {
            elapsed_secs: 210.0,
            kills: 18,
            gold: 360,
            ember_shards: 11,
            affix_essence: 7,
            boss_staggers: 2,
            ..default()
        };
        let mut records = ChapterRecords {
            stash_gold: 40,
            stash_shards: 1,
            stash_essence: 0,
            ..default()
        };

        let reward = claim_soul_remnant(&mut records, &stats, crate::Difficulty::Hell);

        assert_eq!(
            reward,
            ProfileStashBonus {
                gold: 108,
                shards: 4,
                essence: 2,
            }
        );
        assert_eq!(records.failed_runs, 1);
        assert_eq!(records.soul_remnant_gold, 108);
        assert_eq!(records.soul_remnant_shards, 4);
        assert_eq!(records.soul_remnant_essence, 2);
        assert_eq!(records.stash_gold, 148);
        assert_eq!(records.stash_shards, 5);
        assert_eq!(records.stash_essence, 2);
    }

    #[test]
    fn minimap_enemy_markers_rank_boss_treasure_elite_and_trash() {
        let mut enemy = Enemy {
            id: "treasure_imp".to_string(),
            display_name: "Reliquary Treasure Imp".to_string(),
            affixes: vec![],
            attack_damage: 0.0,
            attack_kind: crate::data::EnemyAttackKind::Melee,
            attack_range: 0.6,
            attack_timer: Timer::from_seconds(1.0, TimerMode::Once),
            aggro_range: 6.5,
            move_speed: 3.35,
            gold_min: 95,
            gold_max: 145,
            xp_reward: 75,
        };

        let treasure_size = minimap_enemy_style(&enemy).0;
        enemy.id = "keeper".to_string();
        let boss_size = minimap_enemy_style(&enemy).0;
        enemy.id = "skeleton".to_string();
        let trash_size = minimap_enemy_style(&enemy).0;
        enemy.affixes = vec![crate::enemy::EnemyAffix::Molten];
        let elite_size = minimap_enemy_style(&enemy).0;
        enemy.affixes.push(crate::enemy::EnemyAffix::Shielded);
        let champion_size = minimap_enemy_style(&enemy).0;

        assert!(boss_size > treasure_size);
        assert!(treasure_size > champion_size);
        assert!(champion_size > elite_size);
        assert!(elite_size > trash_size);
    }

    #[test]
    fn pause_help_surfaces_objective_progress_and_core_actions() {
        let progress = ChapterProgress {
            phase: ChapterPhase::Ritual,
            objective: "Extinguish the ember altar".to_string(),
            seal_fragments: 2,
            ..default()
        };
        let difficulty = DifficultySettings {
            current: crate::Difficulty::Nightmare,
        };
        let modifier = ChapterModifier {
            kind: crate::ordeal::ChapterModifierKind::BloodTithe,
            affix: crate::ordeal::OrdealAffix::None,
        };
        let stats = RunStats {
            elapsed_secs: 180.0,
            kills: 8,
            champion_pack_kills: 2,
            nemesis_kills: 1,
            ashen_threat: 48,
            stash_bonus_gold: 25,
            stash_bonus_shards: 1,
            stash_bonus_essence: 1,
            stash_bonus_claimed: true,
            altar_seals: 1,
            altar_bonus_gold: 25,
            altar_bonus_shards: 0,
            altar_bonus_essence: 0,
            altar_bonus_claimed: true,
            ..default()
        };
        let challenges = ChapterChallenges::new();
        let milestones = Milestones::default();
        let mastery = ChapterMastery::default();
        let journey = journey_summary(&challenges, &milestones, &mastery);
        let story = StoryLog::default();

        let text = pause_help_text(PauseHelpInputs {
            progress: &progress,
            difficulty: &difficulty,
            modifier: &modifier,
            stats: &stats,
            journey: &journey,
            clear_summary: "Chapter Clear: Projected C | Main 73% | Optional 2/9 | Story 1/14 | Threat low | S +6 side +9 story",
            season_deeds: "Next deed First Reliquary Clear: clear the chapter once",
            bestiary_goal: "Bestiary next: record Ashbone Guard",
            codex_goal: "Codex next: find a legendary weapon power",
            crafting_goal: "Primal next: seal a swift Ember Rift for an Echo Keystone",
            mastery_goal: "Mastery next: Treasure Hunter - slay the reliquary treasure imp",
            story: &story,
            quest_log: "Quest Log: Main Recover seals 2/3",
            action_guide: "Action Guide: Press Space at the ember altar",
            detailed_status: "Character: test details",
            systems_status: "Champion Pack 2/4 | Nemesis Hunter 1/1 | Ashen Threat 48/100",
            inventory_open: true,
            journal_open: false,
            audio_status: "audio muted",
        });

        assert!(text.contains("PAUSED | Esc resume"));
        assert!(text.contains("Extinguish the ember altar"));
        assert!(text.contains("Seals 2/3"));
        assert!(text.contains("Nightmare"));
        assert!(text.contains("Blood Tithe"));
        assert!(text.contains("Story 1/14"));
        assert!(text.contains("Action Guide: Press Space at the ember altar"));
        assert!(text.contains("Quest Log: Main Recover seals 2/3"));
        assert!(text.contains("Chapter Clear: Projected C"));
        assert!(text.contains("Journey: Initiate"));
        assert!(text.contains("Next deed First Reliquary Clear"));
        assert!(text.contains("Bestiary next: record Ashbone Guard"));
        assert!(text.contains("Codex next: find a legendary"));
        assert!(text.contains("Primal next: seal a swift Ember Rift"));
        assert!(text.contains("Mastery next: Treasure Hunter"));
        assert!(text.contains("Q/E skills"));
        assert!(text.contains("B codex"));
        assert!(text.contains("U salvage"));
        assert!(text.contains("F potion"));
        assert!(text.contains("Champion Pack 2/4"));
        assert!(text.contains("Nemesis Hunter 1/1"));
        assert!(text.contains("Ashen Threat 48/100"));
        assert!(text.contains("Renown: none"));
        assert!(text.contains("Stash start +25 gold +1 shards +1 essence"));
        assert!(text.contains("Inventory open"));
        assert!(text.contains("Journal closed"));
        assert!(text.contains("Audio: audio muted (M)"));
        assert!(!text.contains("Boss Breaks"));
        assert!(text.lines().count() <= 13);
        assert!(text.lines().all(|line| line.len() <= 145));
    }

    #[test]
    fn ordeal_tactics_surface_live_rule_counterplay() {
        let base = "Action Guide: Slay reliquary guards 1/3 | Side Champion 0/4".to_string();
        let progress = ChapterProgress {
            phase: ChapterPhase::Breach,
            ..default()
        };
        let hunger = ChapterModifier {
            kind: crate::ordeal::ChapterModifierKind::BloodTithe,
            affix: OrdealAffix::AshenHunger,
        };
        let glass = ChapterModifier {
            kind: crate::ordeal::ChapterModifierKind::Emberstorm,
            affix: OrdealAffix::GlassRelics,
        };
        let treasure = ChapterModifier {
            kind: crate::ordeal::ChapterModifierKind::TormentBrand,
            affix: OrdealAffix::TreasureFever,
        };
        let cinder = ChapterModifier {
            kind: crate::ordeal::ChapterModifierKind::TormentBrand,
            affix: OrdealAffix::CinderVeins,
        };

        assert!(
            action_guide_with_ordeal(base.clone(), &hunger, &progress)
                .contains("Ordeal kite hits, save potion")
        );
        assert_eq!(ordeal_tactic(&glass), "focus elites, break shields");
        assert_eq!(ordeal_tactic(&treasure), "push side rewards");
        assert_eq!(ordeal_tactic(&cinder), "avoid vents and pools");

        let victory = ChapterProgress {
            phase: ChapterPhase::Victory,
            ..default()
        };
        assert_eq!(
            action_guide_with_ordeal(base.clone(), &hunger, &victory),
            base
        );
        assert_eq!(
            action_guide_with_ordeal(
                base.clone(),
                &ChapterModifier {
                    kind: crate::ordeal::ChapterModifierKind::AshenEchoes,
                    affix: OrdealAffix::None,
                },
                &progress,
            ),
            base
        );
    }

    #[test]
    fn modifier_system_summary_includes_reward_rule_and_tactic() {
        let modifier = ChapterModifier {
            kind: crate::ordeal::ChapterModifierKind::TormentBrand,
            affix: OrdealAffix::CinderVeins,
        };
        let summary = modifier_system_summary(&modifier);

        assert!(summary.contains("Ordeal Torment Brand + Cinder Veins"));
        assert!(summary.contains("brutal enemies"));
        assert!(summary.contains("vents and hazards bite harder"));
        assert!(summary.contains("tactic avoid vents and pools"));
        assert!(summary.len() <= 180);
    }

    #[test]
    fn character_status_summary_stays_readable_and_combat_focused() {
        let mut equipment = test_equipment("legendary");
        equipment.legendary_power = LegendaryPower::Stormbrand;
        equipment.socketed_gem = Some(SocketedGem {
            kind: crate::player::GemKind::Emerald,
            rank: 3,
        });
        equipment.armor_bonus = 18.0;
        let level = PlayerLevel {
            level: 7,
            xp: 40,
            next_level_xp: 100,
        };
        let paragon = EmberParagon {
            rank: 2,
            xp: 10,
            next_rank_xp: 80,
        };
        let elixir = inactive_elixir_buff();
        let ward = DeathWard {
            checkpoint: Vec3::ZERO,
            revives_used: 1,
            max_revives: 2,
            gold_penalty: 40,
        };
        let companion = test_companion(CompanionStance::Vanguard, 0.0);

        let summary = character_status_summary(CharacterStatusInputs {
            level: &level,
            paragon: &paragon,
            equipment: &equipment,
            elixir_buff: &elixir,
            death_ward: &ward,
            resonance: "Resonance: legendary storm set active",
            set_tactic: "Set Storm chain 4/8",
            gem_resonance: "Gem Resonance: emerald sharpens critical chance",
            companion: &companion,
            boss: "Malrec phase two | stagger 40%",
        });

        assert!(summary.contains("L7 P2"));
        assert!(summary.contains("Ward 1"));
        assert!(summary.contains("Stormbrand"));
        assert!(summary.contains("Emerald 3"));
        assert!(summary.contains("Set Storm chain 4/8"));
        assert!(summary.contains("Sentinel"));
        assert!(summary.contains("Boss Malrec"));
        assert!(!summary.contains("Boss Breaks"));
        assert!(!summary.contains("Kills"));
        assert!(!summary.contains("Treasure Vault"));
        assert!(summary.len() <= 170);
    }

    #[test]
    fn boss_hud_stagger_fill_tracks_break_state() {
        let stagger = BossStagger {
            meter: 50.0,
            threshold: 125.0,
        };
        let staggered = Staggered {
            timer: Timer::from_seconds(1.0, TimerMode::Once),
            damage_multiplier: 1.18,
        };

        assert_eq!(boss_hud_stagger_percent(Some(&stagger), None), 40.0);
        assert_eq!(
            boss_hud_stagger_percent(Some(&stagger), Some(&staggered)),
            100.0
        );
        assert_eq!(boss_hud_stagger_percent(None, None), 0.0);
    }

    #[test]
    fn boss_hud_summary_prioritizes_execute_window_when_staggered() {
        let enemy = Enemy {
            id: "keeper".to_string(),
            display_name: "Malrec, Keeper of Ash".to_string(),
            affixes: vec![],
            attack_damage: 20.0,
            attack_kind: crate::data::EnemyAttackKind::Melee,
            attack_range: 1.5,
            attack_timer: Timer::from_seconds(2.0, TimerMode::Once),
            aggro_range: 10.0,
            move_speed: 2.0,
            gold_min: 1,
            gold_max: 2,
            xp_reward: 10,
        };
        let health = Health {
            current: 80.0,
            max: 200.0,
        };
        let phase = BossPhase::new_phase_two(12.0);
        let stagger = BossStagger {
            meter: 0.0,
            threshold: 125.0,
        };
        let staggered = Staggered {
            timer: Timer::from_seconds(1.0, TimerMode::Once),
            damage_multiplier: 1.18,
        };

        let boon = ChapterBoon {
            choice: ChapterBoonChoice::TitanWard,
            empowered: true,
            momentum_stacks: 0,
        };
        let summary = boss_hud_summary(
            &enemy,
            &health,
            &phase,
            Some(&stagger),
            Some(&staggered),
            Some(&boon),
        );

        assert!(summary.contains("EXECUTE 1s"));
        assert!(summary.contains("EXECUTE WINDOW 1.0s x1.18"));
        assert!(summary.contains("Burst: Y/Q/E now"));
        assert!(summary.contains("Empowered Boon Titan"));
        assert!(!summary.contains("Phase II"));
        assert!(!summary.contains("Mechanic:"));
        assert!(!summary.contains("Action:"));
        assert_eq!(summary.lines().count(), 2);
        assert!(
            summary.lines().all(|line| line.chars().count() <= 118),
            "{summary}"
        );
        assert!(boss_boon_summary(None).contains("Boon 4 Crimson"));
        assert!(boss_boon_summary(None).contains("5 Titan"));
        assert!(boss_boon_summary(None).contains("6 Arcane"));

        let near_break = BossStagger {
            meter: 96.0,
            threshold: 125.0,
        };
        let phase_two = boss_hud_summary(&enemy, &health, &phase, Some(&near_break), None, None);
        assert!(phase_two.contains("Phase II"));
        assert!(phase_two.contains("Stagger 77% before enrage 12s"));
        assert!(phase_two.contains("Break soon: save burst"));

        let danger_phase = BossPhase::new_phase_two(5.2);
        let danger = boss_hud_summary(
            &enemy,
            &health,
            &danger_phase,
            Some(&near_break),
            None,
            None,
        );
        assert!(danger.contains("DANGER ENRAGE 6s"));
        assert!(danger.contains("Stagger 77% before enrage 6s"));
        assert!(danger.contains("DANGER: break now"));

        let last_call_phase = BossPhase::new_phase_two(2.4);
        let last_call = boss_hud_summary(
            &enemy,
            &health,
            &last_call_phase,
            Some(&near_break),
            None,
            None,
        );
        assert!(last_call.contains("DANGER ENRAGE 3s"));
        assert!(last_call.contains("DANGER: all burst or kite"));

        let enrage = BossPhase::new_enraged();
        let low_health = Health {
            current: 30.0,
            max: 200.0,
        };
        let enraged = boss_hud_summary(&enemy, &low_health, &enrage, Some(&stagger), None, None);
        assert!(enraged.contains("ENRAGED"));
        assert!(enraged.contains("ENRAGE floor fire"));
        assert!(enraged.contains("Kill now, kite fire"));
    }

    #[test]
    fn boss_boon_summary_surfaces_choice_and_empowerment() {
        let unchosen = ChapterBoon {
            choice: ChapterBoonChoice::None,
            empowered: false,
            momentum_stacks: 0,
        };
        let crimson = ChapterBoon {
            choice: ChapterBoonChoice::CrimsonOath,
            empowered: false,
            momentum_stacks: 0,
        };
        let arcane = ChapterBoon {
            choice: ChapterBoonChoice::ArcaneMomentum,
            empowered: true,
            momentum_stacks: 4,
        };

        assert_eq!(
            boss_boon_summary(Some(&unchosen)),
            "Boon 4 Crimson | 5 Titan | 6 Arcane"
        );
        assert_eq!(boss_boon_summary(Some(&crimson)), "Boon Crimson +dmg/crit");
        assert_eq!(
            boss_boon_summary(Some(&arcane)),
            "Empowered Boon Arcane +fury/economy | Momentum 4x"
        );
    }
}
