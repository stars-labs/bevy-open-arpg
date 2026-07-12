use crate::{
    AFFIX_CODEX_TARGET, ASHEN_THREAT_SURGE_TARGET, BOSS_BREAK_TARGET, CARNAGE_MASTER_STREAK,
    CHAMPION_PACK_TARGET, CODEX_ADEPT_KILLS, Difficulty, DifficultySettings, GEM_ADEPT_KINDS,
    GLORY_SEEKER_GLOBES, GameState, LAST_STAND_POTIONS, RunStats, SET_ADEPT_KILLS,
    bestiary::{Bestiary, chapter_bestiary_complete},
    bounty::BountyProgress,
    chapter::{ChapterProgress, InteractableKind, InteractableUsed},
    companion::{CompanionBond, SENTINEL_VETERAN_RANK},
    dungeon::BREAKER_TARGET_BREAKABLES,
    enemy::{AFFIX_HUNTER_KILLS, CURSED_AMBUSH_KILLS, EnemyKilled},
    feedback::CombatEvent,
    lore::{LORE_ENTRY_COUNT, LoreJournal},
    milestone::{ALL_MILESTONES, MilestoneId, Milestones},
    obelisk::{BloodObelisk, ObeliskState},
    player::{
        ARMORY_SLOT_COUNT, ChapterBoon, ChapterBoonChoice, Charm, DeathWard, ElixirBelt,
        EmberParagon, Equipment, Inventory, LegendaryCodex, PYLON_REAPER_KILLS, Player, PotionBelt,
        SURGE_CHAIN_KILLS, Talents, gear_quality_is_ancient_or_better,
        is_themed_reliquary_resonance,
    },
    rift::{EmberRift, RIFT_SWIFT_SEAL_SECS, RiftState},
};
use bevy::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Eq, PartialEq, Serialize, Deserialize)]
pub enum ChallengeId {
    SwiftClear,
    TreasureHunter,
    TreasureVault,
    Massacre,
    CarnageMaster,
    EliteHunter,
    BattleTrance,
    Vaultbreaker,
    Breaker,
    ShrineResonance,
    Shrinekeeper,
    AffixHunter,
    AffixCodex,
    SealWarden,
    AshenThreat,
    ChampionBreaker,
    NemesisHunter,
    CursedPact,
    Ritekeeper,
    Riftwalker,
    Riftbreaker,
    PylonOverload,
    PylonReaper,
    Sealbreaker,
    SetBuilder,
    SetAdept,
    AncientArmory,
    Augmenter,
    GemResonance,
    MasterGemcutter,
    GemAdept,
    ParagonAwakened,
    TalentAdept,
    QuartermasterPatron,
    Alchemist,
    AshenAlchemy,
    LootCurator,
    UntouchedFlask,
    LastStand,
    SurgeMastery,
    RuneWeaver,
    ArmoryAdept,
    PortalWayfarer,
    GlorySeeker,
    SalvageCache,
    SentinelVeteran,
    Boonbound,
    BountyHunter,
    BestiaryScholar,
    Lorekeeper,
    CodexAdept,
    CodexArchivist,
    FlawlessVictory,
    EnrageDenied,
    BossBreaker,
    SoulSigil,
    TrophyCache,
    TormentClear,
    PrimalCache,
    PrimalInfusion,
    ReliquaryConquest,
}

impl ChallengeId {
    pub fn label(self) -> &'static str {
        match self {
            Self::SwiftClear => "Swift Clear",
            Self::TreasureHunter => "Treasure Hunter",
            Self::TreasureVault => "Treasure Vault",
            Self::Massacre => "Massacre Rite",
            Self::CarnageMaster => "Carnage Master",
            Self::EliteHunter => "Elite Hunter",
            Self::BattleTrance => "Battle Trance",
            Self::Vaultbreaker => "Vaultbreaker",
            Self::Breaker => "Breaker",
            Self::ShrineResonance => "Shrine Resonance",
            Self::Shrinekeeper => "Shrinekeeper",
            Self::AffixHunter => "Affix Hunter",
            Self::AffixCodex => "Affix Codex",
            Self::SealWarden => "Seal Warden",
            Self::AshenThreat => "Ashen Threat",
            Self::ChampionBreaker => "Champion Breaker",
            Self::NemesisHunter => "Nemesis Hunter",
            Self::CursedPact => "Cursed Pact",
            Self::Ritekeeper => "Ritekeeper",
            Self::Riftwalker => "Riftwalker",
            Self::Riftbreaker => "Riftbreaker",
            Self::PylonOverload => "Pylon Overload",
            Self::PylonReaper => "Pylon Reaper",
            Self::Sealbreaker => "Sealbreaker",
            Self::SetBuilder => "Set Builder",
            Self::SetAdept => "Set Adept",
            Self::AncientArmory => "Ancient Armory",
            Self::Augmenter => "Augmenter",
            Self::GemResonance => "Gem Resonance",
            Self::MasterGemcutter => "Master Gemcutter",
            Self::GemAdept => "Gem Adept",
            Self::ParagonAwakened => "Paragon Awakened",
            Self::TalentAdept => "Talent Adept",
            Self::QuartermasterPatron => "Quartermaster Patron",
            Self::Alchemist => "Alchemist",
            Self::AshenAlchemy => "Ashen Alchemy",
            Self::LootCurator => "Loot Curator",
            Self::UntouchedFlask => "Untouched Flask",
            Self::LastStand => "Last Stand",
            Self::SurgeMastery => "Surge Mastery",
            Self::RuneWeaver => "Rune Weaver",
            Self::ArmoryAdept => "Armory Adept",
            Self::PortalWayfarer => "Portal Wayfarer",
            Self::GlorySeeker => "Glory Seeker",
            Self::SalvageCache => "Salvage Cache",
            Self::SentinelVeteran => "Sentinel Veteran",
            Self::Boonbound => "Boonbound",
            Self::BountyHunter => "Bounty Hunter",
            Self::BestiaryScholar => "Bestiary Scholar",
            Self::Lorekeeper => "Lorekeeper",
            Self::CodexAdept => "Codex Adept",
            Self::CodexArchivist => "Codex Archivist",
            Self::FlawlessVictory => "Flawless Victory",
            Self::EnrageDenied => "Enrage Denied",
            Self::BossBreaker => "Boss Breaker",
            Self::SoulSigil => "Soul Sigil",
            Self::TrophyCache => "Trophy Cache",
            Self::TormentClear => "Torment Clear",
            Self::PrimalCache => "Primal Cache",
            Self::PrimalInfusion => "Primal Infusion",
            Self::ReliquaryConquest => "Reliquary Conquest",
        }
    }

    pub fn description(self) -> &'static str {
        match self {
            Self::SwiftClear => "cleanse the reliquary within six minutes",
            Self::TreasureHunter => "slay the reliquary treasure imp",
            Self::TreasureVault => "open the treasure imp's reliquary vault",
            Self::Massacre => "reach a five-kill massacre streak",
            Self::CarnageMaster => "reach a ten-kill massacre streak",
            Self::EliteHunter => "reach five Valor stacks",
            Self::BattleTrance => "reach a five-kill massacre and five Valor stacks",
            Self::Vaultbreaker => "open the resplendent reliquary vault",
            Self::Breaker => "smash every reliquary urn and offering box",
            Self::ShrineResonance => "trigger shrine resonance by chaining shrine blessings",
            Self::Shrinekeeper => "activate every reliquary shrine and pylon",
            Self::AffixHunter => "slay three enemies with at least two affixes",
            Self::AffixCodex => "slay elites carrying six distinct affix types",
            Self::SealWarden => "slay Seal Warden Vhal in the inner sanctum",
            Self::AshenThreat => "trigger three Ashen Threat surges",
            Self::ChampionBreaker => "break the roaming champion pack",
            Self::NemesisHunter => "slay an Ashbound Nemesis after a soul ward revival",
            Self::CursedPact => "slay all cursed shrine ambushers",
            Self::Ritekeeper => "complete the blood obelisk kill rite",
            Self::Riftwalker => "seal the Ember Rift invasion",
            Self::Riftbreaker => "seal the Ember Rift with 30 seconds remaining",
            Self::PylonOverload => "activate the Ashen Pylon",
            Self::PylonReaper => "kill three enemies during Ashen Pylon overload",
            Self::Sealbreaker => {
                "open the vault, complete the blood obelisk, and seal the Ember Rift"
            }
            Self::SetBuilder => "equip a themed high-quality weapon and charm set",
            Self::SetAdept => "slay enemies while a themed set resonance is active",
            Self::AncientArmory => "equip an ancient weapon",
            Self::Augmenter => "apply an ancient augment to a weapon",
            Self::GemResonance => "equip a rank 3 socketed gem",
            Self::MasterGemcutter => "equip a rank 5 socketed gem",
            Self::GemAdept => "socket ruby, emerald, amethyst, and topaz gems",
            Self::ParagonAwakened => "awaken Ember Paragon rank 1",
            Self::TalentAdept => "invest in Wrath, Vigor, and Focus",
            Self::QuartermasterPatron => "upgrade stash, potion belt, and elixir belt",
            Self::Alchemist => "use iron, wrath, and haste elixirs in one run",
            Self::AshenAlchemy => "perform gold, essence, and keystone transmutations",
            Self::LootCurator => "cycle the loot filter through three tiers",
            Self::UntouchedFlask => "defeat Malrec without drinking a potion",
            Self::LastStand => "drink three potions while below 30% health",
            Self::SurgeMastery => "kill three enemies during Nephalem Surge",
            Self::RuneWeaver => "hit enemies with Reap dash, Hemorrhage rupture, and Frost nova",
            Self::ArmoryAdept => "save three armory loadouts",
            Self::PortalWayfarer => "return to the fight through the town portal",
            Self::GlorySeeker => "collect three health globes for Glory",
            Self::SalvageCache => "earn a quartermaster cache from spare gear",
            Self::SentinelVeteran => "raise the Reliquary Sentinel to rank 3",
            Self::Boonbound => "claim a Reliquary Boon before Malrec falls",
            Self::BountyHunter => "complete the optional chapter bounty",
            Self::BestiaryScholar => "record every chapter monster in the bestiary",
            Self::Lorekeeper => "recover every lore page",
            Self::CodexAdept => "slay enemies with an attuned codex power",
            Self::CodexArchivist => "unlock every legendary power in the codex",
            Self::FlawlessVictory => "defeat Malrec without triggering the soul ward",
            Self::EnrageDenied => "defeat Malrec before Ashen Enrage triggers",
            Self::BossBreaker => "break Malrec's stagger twice in one run",
            Self::SoulSigil => "recover a Malrec Soul Sigil from the Keeper",
            Self::TrophyCache => "open a Malrec trophy cache at the quartermaster",
            Self::TormentClear => "defeat Malrec on Torment difficulty",
            Self::PrimalCache => "claim a Primal Ember Cache on Torment",
            Self::PrimalInfusion => "primal-infuse an ancient weapon at the quartermaster",
            Self::ReliquaryConquest => "complete every chapter milestone in one run",
        }
    }

    pub fn reward(self) -> ChallengeReward {
        match self {
            Self::SwiftClear => ChallengeReward {
                gold: 80,
                ember_shards: 2,
                affix_essence: 0,
            },
            Self::TreasureHunter => ChallengeReward {
                gold: 110,
                ember_shards: 3,
                affix_essence: 0,
            },
            Self::TreasureVault => ChallengeReward {
                gold: 160,
                ember_shards: 5,
                affix_essence: 4,
            },
            Self::Massacre => ChallengeReward {
                gold: 70,
                ember_shards: 2,
                affix_essence: 0,
            },
            Self::CarnageMaster => ChallengeReward {
                gold: 180,
                ember_shards: 5,
                affix_essence: 5,
            },
            Self::EliteHunter => ChallengeReward {
                gold: 130,
                ember_shards: 2,
                affix_essence: 5,
            },
            Self::BattleTrance => ChallengeReward {
                gold: 160,
                ember_shards: 4,
                affix_essence: 5,
            },
            Self::Vaultbreaker => ChallengeReward {
                gold: 125,
                ember_shards: 3,
                affix_essence: 3,
            },
            Self::Breaker => ChallengeReward {
                gold: 90,
                ember_shards: 2,
                affix_essence: 2,
            },
            Self::ShrineResonance => ChallengeReward {
                gold: 125,
                ember_shards: 3,
                affix_essence: 3,
            },
            Self::Shrinekeeper => ChallengeReward {
                gold: 170,
                ember_shards: 5,
                affix_essence: 5,
            },
            Self::AffixHunter => ChallengeReward {
                gold: 185,
                ember_shards: 5,
                affix_essence: 7,
            },
            Self::AffixCodex => ChallengeReward {
                gold: 210,
                ember_shards: 6,
                affix_essence: 8,
            },
            Self::SealWarden => ChallengeReward {
                gold: 115,
                ember_shards: 3,
                affix_essence: 3,
            },
            Self::AshenThreat => ChallengeReward {
                gold: 150,
                ember_shards: 5,
                affix_essence: 4,
            },
            Self::ChampionBreaker => ChallengeReward {
                gold: 145,
                ember_shards: 4,
                affix_essence: 4,
            },
            Self::NemesisHunter => ChallengeReward {
                gold: 150,
                ember_shards: 4,
                affix_essence: 6,
            },
            Self::CursedPact => ChallengeReward {
                gold: 145,
                ember_shards: 4,
                affix_essence: 5,
            },
            Self::Ritekeeper => ChallengeReward {
                gold: 140,
                ember_shards: 4,
                affix_essence: 4,
            },
            Self::Riftwalker => ChallengeReward {
                gold: 155,
                ember_shards: 5,
                affix_essence: 5,
            },
            Self::Riftbreaker => ChallengeReward {
                gold: 190,
                ember_shards: 6,
                affix_essence: 7,
            },
            Self::PylonOverload => ChallengeReward {
                gold: 135,
                ember_shards: 4,
                affix_essence: 4,
            },
            Self::PylonReaper => ChallengeReward {
                gold: 175,
                ember_shards: 5,
                affix_essence: 6,
            },
            Self::Sealbreaker => ChallengeReward {
                gold: 220,
                ember_shards: 7,
                affix_essence: 8,
            },
            Self::SetBuilder => ChallengeReward {
                gold: 150,
                ember_shards: 4,
                affix_essence: 5,
            },
            Self::SetAdept => ChallengeReward {
                gold: 165,
                ember_shards: 5,
                affix_essence: 5,
            },
            Self::AncientArmory => ChallengeReward {
                gold: 170,
                ember_shards: 5,
                affix_essence: 6,
            },
            Self::Augmenter => ChallengeReward {
                gold: 190,
                ember_shards: 5,
                affix_essence: 7,
            },
            Self::GemResonance => ChallengeReward {
                gold: 150,
                ember_shards: 4,
                affix_essence: 4,
            },
            Self::MasterGemcutter => ChallengeReward {
                gold: 175,
                ember_shards: 5,
                affix_essence: 5,
            },
            Self::GemAdept => ChallengeReward {
                gold: 155,
                ember_shards: 4,
                affix_essence: 5,
            },
            Self::ParagonAwakened => ChallengeReward {
                gold: 210,
                ember_shards: 6,
                affix_essence: 8,
            },
            Self::TalentAdept => ChallengeReward {
                gold: 145,
                ember_shards: 4,
                affix_essence: 4,
            },
            Self::QuartermasterPatron => ChallengeReward {
                gold: 180,
                ember_shards: 5,
                affix_essence: 5,
            },
            Self::Alchemist => ChallengeReward {
                gold: 150,
                ember_shards: 4,
                affix_essence: 4,
            },
            Self::AshenAlchemy => ChallengeReward {
                gold: 170,
                ember_shards: 5,
                affix_essence: 6,
            },
            Self::LootCurator => ChallengeReward {
                gold: 95,
                ember_shards: 3,
                affix_essence: 2,
            },
            Self::UntouchedFlask => ChallengeReward {
                gold: 180,
                ember_shards: 5,
                affix_essence: 5,
            },
            Self::LastStand => ChallengeReward {
                gold: 120,
                ember_shards: 3,
                affix_essence: 4,
            },
            Self::SurgeMastery => ChallengeReward {
                gold: 165,
                ember_shards: 4,
                affix_essence: 5,
            },
            Self::RuneWeaver => ChallengeReward {
                gold: 155,
                ember_shards: 4,
                affix_essence: 5,
            },
            Self::ArmoryAdept => ChallengeReward {
                gold: 150,
                ember_shards: 4,
                affix_essence: 4,
            },
            Self::PortalWayfarer => ChallengeReward {
                gold: 120,
                ember_shards: 3,
                affix_essence: 3,
            },
            Self::GlorySeeker => ChallengeReward {
                gold: 125,
                ember_shards: 3,
                affix_essence: 4,
            },
            Self::SalvageCache => ChallengeReward {
                gold: 140,
                ember_shards: 4,
                affix_essence: 4,
            },
            Self::SentinelVeteran => ChallengeReward {
                gold: 160,
                ember_shards: 4,
                affix_essence: 4,
            },
            Self::Boonbound => ChallengeReward {
                gold: 150,
                ember_shards: 4,
                affix_essence: 4,
            },
            Self::BountyHunter => ChallengeReward {
                gold: 170,
                ember_shards: 5,
                affix_essence: 5,
            },
            Self::BestiaryScholar => ChallengeReward {
                gold: 150,
                ember_shards: 4,
                affix_essence: 4,
            },
            Self::Lorekeeper => ChallengeReward {
                gold: 120,
                ember_shards: 4,
                affix_essence: 3,
            },
            Self::CodexAdept => ChallengeReward {
                gold: 150,
                ember_shards: 4,
                affix_essence: 5,
            },
            Self::CodexArchivist => ChallengeReward {
                gold: 180,
                ember_shards: 5,
                affix_essence: 6,
            },
            Self::FlawlessVictory => ChallengeReward {
                gold: 160,
                ember_shards: 4,
                affix_essence: 4,
            },
            Self::EnrageDenied => ChallengeReward {
                gold: 200,
                ember_shards: 6,
                affix_essence: 7,
            },
            Self::BossBreaker => ChallengeReward {
                gold: 190,
                ember_shards: 6,
                affix_essence: 8,
            },
            Self::SoulSigil => ChallengeReward {
                gold: 180,
                ember_shards: 5,
                affix_essence: 7,
            },
            Self::TrophyCache => ChallengeReward {
                gold: 210,
                ember_shards: 6,
                affix_essence: 8,
            },
            Self::TormentClear => ChallengeReward {
                gold: 320,
                ember_shards: 10,
                affix_essence: 12,
            },
            Self::PrimalCache => ChallengeReward {
                gold: 240,
                ember_shards: 8,
                affix_essence: 10,
            },
            Self::PrimalInfusion => ChallengeReward {
                gold: 260,
                ember_shards: 8,
                affix_essence: 12,
            },
            Self::ReliquaryConquest => ChallengeReward {
                gold: 260,
                ember_shards: 8,
                affix_essence: 10,
            },
        }
    }
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub struct ChallengeReward {
    pub gold: u32,
    pub ember_shards: u32,
    pub affix_essence: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ChallengeProgress {
    pub id: ChallengeId,
    pub completed: bool,
    pub reward_claimed: bool,
}

impl ChallengeProgress {
    pub fn new(id: ChallengeId) -> Self {
        Self {
            id,
            completed: false,
            reward_claimed: false,
        }
    }
}

#[derive(Resource, Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ChapterChallenges {
    pub challenges: Vec<ChallengeProgress>,
    pub latest: String,
    pub reward_gold: u32,
    pub reward_shards: u32,
    pub reward_essence: u32,
}

impl Default for ChapterChallenges {
    fn default() -> Self {
        Self::new()
    }
}

impl ChapterChallenges {
    pub fn new() -> Self {
        Self {
            challenges: vec![
                ChallengeProgress::new(ChallengeId::SwiftClear),
                ChallengeProgress::new(ChallengeId::TreasureHunter),
                ChallengeProgress::new(ChallengeId::TreasureVault),
                ChallengeProgress::new(ChallengeId::Massacre),
                ChallengeProgress::new(ChallengeId::CarnageMaster),
                ChallengeProgress::new(ChallengeId::EliteHunter),
                ChallengeProgress::new(ChallengeId::BattleTrance),
                ChallengeProgress::new(ChallengeId::Vaultbreaker),
                ChallengeProgress::new(ChallengeId::Breaker),
                ChallengeProgress::new(ChallengeId::ShrineResonance),
                ChallengeProgress::new(ChallengeId::Shrinekeeper),
                ChallengeProgress::new(ChallengeId::AffixHunter),
                ChallengeProgress::new(ChallengeId::AffixCodex),
                ChallengeProgress::new(ChallengeId::SealWarden),
                ChallengeProgress::new(ChallengeId::AshenThreat),
                ChallengeProgress::new(ChallengeId::ChampionBreaker),
                ChallengeProgress::new(ChallengeId::NemesisHunter),
                ChallengeProgress::new(ChallengeId::CursedPact),
                ChallengeProgress::new(ChallengeId::Ritekeeper),
                ChallengeProgress::new(ChallengeId::Riftwalker),
                ChallengeProgress::new(ChallengeId::Riftbreaker),
                ChallengeProgress::new(ChallengeId::PylonOverload),
                ChallengeProgress::new(ChallengeId::PylonReaper),
                ChallengeProgress::new(ChallengeId::Sealbreaker),
                ChallengeProgress::new(ChallengeId::SetBuilder),
                ChallengeProgress::new(ChallengeId::SetAdept),
                ChallengeProgress::new(ChallengeId::AncientArmory),
                ChallengeProgress::new(ChallengeId::Augmenter),
                ChallengeProgress::new(ChallengeId::GemResonance),
                ChallengeProgress::new(ChallengeId::MasterGemcutter),
                ChallengeProgress::new(ChallengeId::GemAdept),
                ChallengeProgress::new(ChallengeId::ParagonAwakened),
                ChallengeProgress::new(ChallengeId::TalentAdept),
                ChallengeProgress::new(ChallengeId::QuartermasterPatron),
                ChallengeProgress::new(ChallengeId::Alchemist),
                ChallengeProgress::new(ChallengeId::AshenAlchemy),
                ChallengeProgress::new(ChallengeId::LootCurator),
                ChallengeProgress::new(ChallengeId::UntouchedFlask),
                ChallengeProgress::new(ChallengeId::LastStand),
                ChallengeProgress::new(ChallengeId::SurgeMastery),
                ChallengeProgress::new(ChallengeId::RuneWeaver),
                ChallengeProgress::new(ChallengeId::ArmoryAdept),
                ChallengeProgress::new(ChallengeId::PortalWayfarer),
                ChallengeProgress::new(ChallengeId::GlorySeeker),
                ChallengeProgress::new(ChallengeId::SalvageCache),
                ChallengeProgress::new(ChallengeId::SentinelVeteran),
                ChallengeProgress::new(ChallengeId::Boonbound),
                ChallengeProgress::new(ChallengeId::BountyHunter),
                ChallengeProgress::new(ChallengeId::BestiaryScholar),
                ChallengeProgress::new(ChallengeId::Lorekeeper),
                ChallengeProgress::new(ChallengeId::CodexAdept),
                ChallengeProgress::new(ChallengeId::CodexArchivist),
                ChallengeProgress::new(ChallengeId::FlawlessVictory),
                ChallengeProgress::new(ChallengeId::EnrageDenied),
                ChallengeProgress::new(ChallengeId::BossBreaker),
                ChallengeProgress::new(ChallengeId::SoulSigil),
                ChallengeProgress::new(ChallengeId::TrophyCache),
                ChallengeProgress::new(ChallengeId::TormentClear),
                ChallengeProgress::new(ChallengeId::PrimalCache),
                ChallengeProgress::new(ChallengeId::PrimalInfusion),
                ChallengeProgress::new(ChallengeId::ReliquaryConquest),
            ],
            latest: String::new(),
            reward_gold: 0,
            reward_shards: 0,
            reward_essence: 0,
        }
    }

    pub fn restore(&mut self, challenges: Vec<ChallengeProgress>) {
        let mut restored = Self::new();
        for challenge in challenges {
            if let Some(existing) = restored
                .challenges
                .iter_mut()
                .find(|existing| existing.id == challenge.id)
            {
                *existing = challenge;
            }
        }
        *self = restored;
    }

    pub fn complete(&mut self, id: ChallengeId) -> bool {
        let Some(challenge) = self
            .challenges
            .iter_mut()
            .find(|challenge| challenge.id == id)
        else {
            return false;
        };
        if challenge.completed {
            return false;
        }
        challenge.completed = true;
        self.latest = format!("Challenge complete: {}", id.label());
        true
    }

    pub fn claim_completed_rewards(&mut self) -> ChallengeReward {
        let mut reward = ChallengeReward {
            gold: 0,
            ember_shards: 0,
            affix_essence: 0,
        };
        for challenge in &mut self.challenges {
            if !challenge.completed || challenge.reward_claimed {
                continue;
            }
            let challenge_reward = challenge.id.reward();
            reward.gold += challenge_reward.gold;
            reward.ember_shards += challenge_reward.ember_shards;
            reward.affix_essence += challenge_reward.affix_essence;
            challenge.reward_claimed = true;
        }
        self.reward_gold += reward.gold;
        self.reward_shards += reward.ember_shards;
        self.reward_essence += reward.affix_essence;
        reward
    }
}

pub fn challenge_summary(challenges: &ChapterChallenges) -> String {
    let completed = challenges
        .challenges
        .iter()
        .filter(|challenge| challenge.completed)
        .count();
    let total = challenges.challenges.len();
    let next = challenges
        .challenges
        .iter()
        .find(|challenge| !challenge.completed)
        .map(|challenge| format!("{}: {}", challenge.id.label(), challenge.id.description()))
        .unwrap_or_else(|| "all challenges complete".to_string());
    format!("Challenges {completed}/{total} - {next}")
}

pub struct ChallengePlugin;

impl Plugin for ChallengePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<ChapterChallenges>()
            .add_systems(OnEnter(GameState::InGame), reset_challenges)
            .add_systems(Update, track_challenges.run_if(in_state(GameState::InGame)))
            .add_systems(OnEnter(GameState::Victory), award_challenge_rewards);
    }
}

fn reset_challenges(mut challenges: ResMut<ChapterChallenges>) {
    *challenges = ChapterChallenges::new();
}

type ChallengePlayerQuery<'w, 's> = Query<
    'w,
    's,
    (
        &'static LegendaryCodex,
        &'static Equipment,
        &'static Charm,
        &'static EmberParagon,
        &'static Inventory,
        &'static PotionBelt,
        &'static ElixirBelt,
        &'static CompanionBond,
        &'static ChapterBoon,
        &'static Talents,
    ),
    With<Player>,
>;
type ChallengeResources<'w> = (
    Res<'w, RunStats>,
    Res<'w, BloodObelisk>,
    Res<'w, EmberRift>,
    Res<'w, Bestiary>,
    Res<'w, LoreJournal>,
    Res<'w, BountyProgress>,
    Res<'w, Milestones>,
);

fn track_challenges(
    mut kills: MessageReader<EnemyKilled>,
    mut interactions: MessageReader<InteractableUsed>,
    resources: ChallengeResources,
    player: ChallengePlayerQuery,
    mut challenges: ResMut<ChapterChallenges>,
    mut events: MessageWriter<CombatEvent>,
) {
    let (stats, obelisk, rift, bestiary, journal, bounty, milestones) = resources;
    for kill in kills.read() {
        if kill.enemy_id == "treasure_imp" && challenges.complete(ChallengeId::TreasureHunter) {
            events.write(CombatEvent {
                text: "Challenge complete: Treasure Hunter".to_string(),
            });
        }
        if treasure_vault_complete(&stats) && challenges.complete(ChallengeId::TreasureVault) {
            events.write(CombatEvent {
                text: "Challenge complete: Treasure Vault".to_string(),
            });
        }
        if kill.enemy_id == "nemesis" && challenges.complete(ChallengeId::NemesisHunter) {
            events.write(CombatEvent {
                text: "Challenge complete: Nemesis Hunter".to_string(),
            });
        }
        if kill.cursed_ambusher
            && cursed_pact_complete(&stats)
            && challenges.complete(ChallengeId::CursedPact)
        {
            events.write(CombatEvent {
                text: "Challenge complete: Cursed Pact".to_string(),
            });
        }
        if kill.affix_count >= 2
            && affix_hunter_complete(&stats)
            && challenges.complete(ChallengeId::AffixHunter)
        {
            events.write(CombatEvent {
                text: "Challenge complete: Affix Hunter".to_string(),
            });
        }
        if kill.enemy_id == "seal_warden" && challenges.complete(ChallengeId::SealWarden) {
            events.write(CombatEvent {
                text: "Challenge complete: Seal Warden".to_string(),
            });
        }
    }
    for interaction in interactions.read() {
        if interaction.kind == InteractableKind::ReliquaryVault
            && challenges.complete(ChallengeId::Vaultbreaker)
        {
            events.write(CombatEvent {
                text: "Challenge complete: Vaultbreaker".to_string(),
            });
        }
        if interaction.kind == InteractableKind::AshenPylon
            && challenges.complete(ChallengeId::PylonOverload)
        {
            events.write(CombatEvent {
                text: "Challenge complete: Pylon Overload".to_string(),
            });
        }
    }
    if stats.best_massacre_streak >= 5 && challenges.complete(ChallengeId::Massacre) {
        events.write(CombatEvent {
            text: "Challenge complete: Massacre Rite".to_string(),
        });
    }
    if carnage_master_complete(&stats) && challenges.complete(ChallengeId::CarnageMaster) {
        events.write(CombatEvent {
            text: "Challenge complete: Carnage Master".to_string(),
        });
    }
    if stats.best_valor_stacks >= 5 && challenges.complete(ChallengeId::EliteHunter) {
        events.write(CombatEvent {
            text: "Challenge complete: Elite Hunter".to_string(),
        });
    }
    if battle_trance_complete(&stats) && challenges.complete(ChallengeId::BattleTrance) {
        events.write(CombatEvent {
            text: "Challenge complete: Battle Trance".to_string(),
        });
    }
    if stats.ancient_augments > 0 && challenges.complete(ChallengeId::Augmenter) {
        events.write(CombatEvent {
            text: "Challenge complete: Augmenter".to_string(),
        });
    }
    if alchemist_complete(&stats) && challenges.complete(ChallengeId::Alchemist) {
        events.write(CombatEvent {
            text: "Challenge complete: Alchemist".to_string(),
        });
    }
    if ashen_alchemy_complete(&stats) && challenges.complete(ChallengeId::AshenAlchemy) {
        events.write(CombatEvent {
            text: "Challenge complete: Ashen Alchemy".to_string(),
        });
    }
    if loot_curator_complete(&stats) && challenges.complete(ChallengeId::LootCurator) {
        events.write(CombatEvent {
            text: "Challenge complete: Loot Curator".to_string(),
        });
    }
    if last_stand_complete(&stats) && challenges.complete(ChallengeId::LastStand) {
        events.write(CombatEvent {
            text: "Challenge complete: Last Stand".to_string(),
        });
    }
    if surge_mastery_complete(&stats) && challenges.complete(ChallengeId::SurgeMastery) {
        events.write(CombatEvent {
            text: "Challenge complete: Surge Mastery".to_string(),
        });
    }
    if rune_weaver_complete(&stats) && challenges.complete(ChallengeId::RuneWeaver) {
        events.write(CombatEvent {
            text: "Challenge complete: Rune Weaver".to_string(),
        });
    }
    if armory_adept_complete(&stats) && challenges.complete(ChallengeId::ArmoryAdept) {
        events.write(CombatEvent {
            text: "Challenge complete: Armory Adept".to_string(),
        });
    }
    if portal_wayfarer_complete(&stats) && challenges.complete(ChallengeId::PortalWayfarer) {
        events.write(CombatEvent {
            text: "Challenge complete: Portal Wayfarer".to_string(),
        });
    }
    if glory_seeker_complete(&stats) && challenges.complete(ChallengeId::GlorySeeker) {
        events.write(CombatEvent {
            text: "Challenge complete: Glory Seeker".to_string(),
        });
    }
    if pylon_reaper_complete(&stats) && challenges.complete(ChallengeId::PylonReaper) {
        events.write(CombatEvent {
            text: "Challenge complete: Pylon Reaper".to_string(),
        });
    }
    if boss_breaker_complete(&stats) && challenges.complete(ChallengeId::BossBreaker) {
        events.write(CombatEvent {
            text: "Challenge complete: Boss Breaker".to_string(),
        });
    }
    if soul_sigil_complete(&stats) && challenges.complete(ChallengeId::SoulSigil) {
        events.write(CombatEvent {
            text: "Challenge complete: Soul Sigil".to_string(),
        });
    }
    if trophy_cache_complete(&stats) && challenges.complete(ChallengeId::TrophyCache) {
        events.write(CombatEvent {
            text: "Challenge complete: Trophy Cache".to_string(),
        });
    }
    if primal_cache_complete(&stats) && challenges.complete(ChallengeId::PrimalCache) {
        events.write(CombatEvent {
            text: "Challenge complete: Primal Cache".to_string(),
        });
    }
    if primal_infusion_complete(&stats) && challenges.complete(ChallengeId::PrimalInfusion) {
        events.write(CombatEvent {
            text: "Challenge complete: Primal Infusion".to_string(),
        });
    }
    if breaker_complete(&stats) && challenges.complete(ChallengeId::Breaker) {
        events.write(CombatEvent {
            text: "Challenge complete: Breaker".to_string(),
        });
    }
    if shrine_resonance_complete(&stats) && challenges.complete(ChallengeId::ShrineResonance) {
        events.write(CombatEvent {
            text: "Challenge complete: Shrine Resonance".to_string(),
        });
    }
    if shrinekeeper_complete(&milestones) && challenges.complete(ChallengeId::Shrinekeeper) {
        events.write(CombatEvent {
            text: "Challenge complete: Shrinekeeper".to_string(),
        });
    }
    if affix_hunter_complete(&stats) && challenges.complete(ChallengeId::AffixHunter) {
        events.write(CombatEvent {
            text: "Challenge complete: Affix Hunter".to_string(),
        });
    }
    if affix_codex_complete(&stats) && challenges.complete(ChallengeId::AffixCodex) {
        events.write(CombatEvent {
            text: "Challenge complete: Affix Codex".to_string(),
        });
    }
    if seal_warden_complete(&stats) && challenges.complete(ChallengeId::SealWarden) {
        events.write(CombatEvent {
            text: "Challenge complete: Seal Warden".to_string(),
        });
    }
    if ashen_threat_complete(&stats) && challenges.complete(ChallengeId::AshenThreat) {
        events.write(CombatEvent {
            text: "Challenge complete: Ashen Threat".to_string(),
        });
    }
    if champion_breaker_complete(&stats) && challenges.complete(ChallengeId::ChampionBreaker) {
        events.write(CombatEvent {
            text: "Challenge complete: Champion Breaker".to_string(),
        });
    }
    if cursed_pact_complete(&stats) && challenges.complete(ChallengeId::CursedPact) {
        events.write(CombatEvent {
            text: "Challenge complete: Cursed Pact".to_string(),
        });
    }
    if sealbreaker_complete(&milestones) && challenges.complete(ChallengeId::Sealbreaker) {
        events.write(CombatEvent {
            text: "Challenge complete: Sealbreaker".to_string(),
        });
    }
    if salvage_cache_complete(&stats) && challenges.complete(ChallengeId::SalvageCache) {
        events.write(CombatEvent {
            text: "Challenge complete: Salvage Cache".to_string(),
        });
    }
    if obelisk.state == ObeliskState::Completed && challenges.complete(ChallengeId::Ritekeeper) {
        events.write(CombatEvent {
            text: "Challenge complete: Ritekeeper".to_string(),
        });
    }
    if rift.state == RiftState::Completed && challenges.complete(ChallengeId::Riftwalker) {
        events.write(CombatEvent {
            text: "Challenge complete: Riftwalker".to_string(),
        });
    }
    if riftbreaker_complete(&rift) && challenges.complete(ChallengeId::Riftbreaker) {
        events.write(CombatEvent {
            text: "Challenge complete: Riftbreaker".to_string(),
        });
    }
    if chapter_bestiary_complete(&bestiary) && challenges.complete(ChallengeId::BestiaryScholar) {
        events.write(CombatEvent {
            text: "Challenge complete: Bestiary Scholar".to_string(),
        });
    }
    if journal.entries.len() >= LORE_ENTRY_COUNT && challenges.complete(ChallengeId::Lorekeeper) {
        events.write(CombatEvent {
            text: "Challenge complete: Lorekeeper".to_string(),
        });
    }
    if bounty.reward_claimed && challenges.complete(ChallengeId::BountyHunter) {
        events.write(CombatEvent {
            text: "Challenge complete: Bounty Hunter".to_string(),
        });
    }
    if codex_adept_complete(&stats) && challenges.complete(ChallengeId::CodexAdept) {
        events.write(CombatEvent {
            text: "Challenge complete: Codex Adept".to_string(),
        });
    }
    if gem_adept_complete(&stats) && challenges.complete(ChallengeId::GemAdept) {
        events.write(CombatEvent {
            text: "Challenge complete: Gem Adept".to_string(),
        });
    }
    if let Ok((
        codex,
        equipment,
        charm,
        paragon,
        inventory,
        potions,
        elixirs,
        companion,
        boon,
        talents,
    )) = player.single()
    {
        if is_themed_reliquary_resonance(equipment, charm)
            && challenges.complete(ChallengeId::SetBuilder)
        {
            events.write(CombatEvent {
                text: "Challenge complete: Set Builder".to_string(),
            });
        }
        if set_adept_complete(&stats) && challenges.complete(ChallengeId::SetAdept) {
            events.write(CombatEvent {
                text: "Challenge complete: Set Adept".to_string(),
            });
        }
        if gear_quality_is_ancient_or_better(&equipment.quality)
            && challenges.complete(ChallengeId::AncientArmory)
        {
            events.write(CombatEvent {
                text: "Challenge complete: Ancient Armory".to_string(),
            });
        }
        if gem_resonance_complete(equipment) && challenges.complete(ChallengeId::GemResonance) {
            events.write(CombatEvent {
                text: "Challenge complete: Gem Resonance".to_string(),
            });
        }
        if master_gemcutter_complete(equipment) && challenges.complete(ChallengeId::MasterGemcutter)
        {
            events.write(CombatEvent {
                text: "Challenge complete: Master Gemcutter".to_string(),
            });
        }
        if codex.is_complete() && challenges.complete(ChallengeId::CodexArchivist) {
            events.write(CombatEvent {
                text: "Challenge complete: Codex Archivist".to_string(),
            });
        }
        if paragon.rank > 0 && challenges.complete(ChallengeId::ParagonAwakened) {
            events.write(CombatEvent {
                text: "Challenge complete: Paragon Awakened".to_string(),
            });
        }
        if talent_adept_complete(talents) && challenges.complete(ChallengeId::TalentAdept) {
            events.write(CombatEvent {
                text: "Challenge complete: Talent Adept".to_string(),
            });
        }
        if quartermaster_patron_complete(inventory, potions, elixirs)
            && challenges.complete(ChallengeId::QuartermasterPatron)
        {
            events.write(CombatEvent {
                text: "Challenge complete: Quartermaster Patron".to_string(),
            });
        }
        if sentinel_veteran_complete(companion) && challenges.complete(ChallengeId::SentinelVeteran)
        {
            events.write(CombatEvent {
                text: "Challenge complete: Sentinel Veteran".to_string(),
            });
        }
        if boonbound_complete(boon) && challenges.complete(ChallengeId::Boonbound) {
            events.write(CombatEvent {
                text: "Challenge complete: Boonbound".to_string(),
            });
        }
    }
}

fn quartermaster_patron_complete(
    inventory: &Inventory,
    potions: &PotionBelt,
    elixirs: &ElixirBelt,
) -> bool {
    inventory.capacity > 12 && potions.max > 5 && elixirs.max > 3
}

fn talent_adept_complete(talents: &Talents) -> bool {
    talents.wrath > 0 && talents.vigor > 0 && talents.focus > 0
}

fn alchemist_complete(stats: &RunStats) -> bool {
    stats.iron_elixir_used && stats.wrath_elixir_used && stats.haste_elixir_used
}

fn ashen_alchemy_complete(stats: &RunStats) -> bool {
    stats.alchemy_gold_transmutes > 0
        && stats.alchemy_essence_transmutes > 0
        && stats.alchemy_keystone_transmutes > 0
}

fn loot_curator_complete(stats: &RunStats) -> bool {
    stats.loot_filter_cycles >= 3
}

fn last_stand_complete(stats: &RunStats) -> bool {
    stats.last_stand_potions >= LAST_STAND_POTIONS
}

fn battle_trance_complete(stats: &RunStats) -> bool {
    stats.best_massacre_streak >= 5 && stats.best_valor_stacks >= 5
}

fn carnage_master_complete(stats: &RunStats) -> bool {
    stats.best_massacre_streak >= CARNAGE_MASTER_STREAK
}

fn surge_mastery_complete(stats: &RunStats) -> bool {
    stats.surge_kills >= SURGE_CHAIN_KILLS
}

fn rune_weaver_complete(stats: &RunStats) -> bool {
    stats.reap_dash_hits > 0 && stats.hemorrhage_rupture_hits > 0 && stats.frost_nova_hits > 0
}

fn armory_adept_complete(stats: &RunStats) -> bool {
    stats.armory_loadouts_saved >= ARMORY_SLOT_COUNT as u32
}

fn portal_wayfarer_complete(stats: &RunStats) -> bool {
    stats.town_portal_returns > 0
}

fn glory_seeker_complete(stats: &RunStats) -> bool {
    stats.health_globes_collected >= GLORY_SEEKER_GLOBES
}

fn pylon_reaper_complete(stats: &RunStats) -> bool {
    stats.pylon_kills >= PYLON_REAPER_KILLS
}

fn boss_breaker_complete(stats: &RunStats) -> bool {
    stats.boss_staggers >= BOSS_BREAK_TARGET
}

fn soul_sigil_complete(stats: &RunStats) -> bool {
    stats.malrec_soul_sigils > 0
}

fn trophy_cache_complete(stats: &RunStats) -> bool {
    stats.soul_sigil_caches > 0
}

fn primal_cache_complete(stats: &RunStats) -> bool {
    stats.primal_caches > 0
}

fn primal_infusion_complete(stats: &RunStats) -> bool {
    stats.primal_infusions > 0
}

fn breaker_complete(stats: &RunStats) -> bool {
    stats.breakables_smashed >= BREAKER_TARGET_BREAKABLES
}

fn shrine_resonance_complete(stats: &RunStats) -> bool {
    stats.shrine_resonance_triggered
}

fn shrinekeeper_complete(milestones: &Milestones) -> bool {
    milestones.unlocked.contains(&MilestoneId::RelicEmpowered)
        && milestones.unlocked.contains(&MilestoneId::FortuneBlessed)
        && milestones.unlocked.contains(&MilestoneId::StormConduit)
        && milestones.unlocked.contains(&MilestoneId::AshenPylon)
}

fn affix_hunter_complete(stats: &RunStats) -> bool {
    stats.elite_affix_kills >= AFFIX_HUNTER_KILLS
}

fn affix_codex_count(stats: &RunStats) -> u32 {
    stats.affix_codex_mask.count_ones()
}

fn affix_codex_complete(stats: &RunStats) -> bool {
    affix_codex_count(stats) >= AFFIX_CODEX_TARGET
}

fn seal_warden_complete(stats: &RunStats) -> bool {
    stats.seal_warden_slain
}

fn ashen_threat_complete(stats: &RunStats) -> bool {
    stats.ashen_threat_surges >= ASHEN_THREAT_SURGE_TARGET
}

fn cursed_pact_complete(stats: &RunStats) -> bool {
    stats.cursed_ambush_kills >= CURSED_AMBUSH_KILLS
}

fn sealbreaker_complete(milestones: &Milestones) -> bool {
    milestones.unlocked.contains(&MilestoneId::VaultOpened)
        && milestones.unlocked.contains(&MilestoneId::BloodObelisk)
        && milestones.unlocked.contains(&MilestoneId::EmberRift)
}

fn salvage_cache_complete(stats: &RunStats) -> bool {
    stats.salvage_caches > 0
}

fn champion_breaker_complete(stats: &RunStats) -> bool {
    stats.champion_pack_reward_claimed && stats.champion_pack_kills >= CHAMPION_PACK_TARGET
}

fn codex_adept_complete(stats: &RunStats) -> bool {
    stats.codex_attuned_kills >= CODEX_ADEPT_KILLS
}

fn treasure_vault_complete(stats: &RunStats) -> bool {
    stats.treasure_vaults_opened > 0
}

fn set_adept_complete(stats: &RunStats) -> bool {
    stats.set_resonance_kills >= SET_ADEPT_KILLS
}

fn gem_adept_complete(stats: &RunStats) -> bool {
    gem_adept_count(stats) >= GEM_ADEPT_KINDS
}

fn gem_adept_count(stats: &RunStats) -> u32 {
    u32::from(stats.ruby_socketed)
        + u32::from(stats.emerald_socketed)
        + u32::from(stats.amethyst_socketed)
        + u32::from(stats.topaz_socketed)
}

fn riftbreaker_complete(rift: &EmberRift) -> bool {
    rift.best_remaining_secs >= RIFT_SWIFT_SEAL_SECS
}

fn sentinel_veteran_complete(companion: &CompanionBond) -> bool {
    companion.rank >= SENTINEL_VETERAN_RANK
}

fn boonbound_complete(boon: &ChapterBoon) -> bool {
    boon.choice != ChapterBoonChoice::None
}

fn gem_resonance_complete(equipment: &Equipment) -> bool {
    equipment.socketed_gem.is_some_and(|gem| gem.rank >= 3)
}

fn master_gemcutter_complete(equipment: &Equipment) -> bool {
    equipment.socketed_gem.is_some_and(|gem| gem.rank >= 5)
}

fn award_challenge_rewards(
    mut stats: ResMut<RunStats>,
    mut challenges: ResMut<ChapterChallenges>,
    difficulty: Res<DifficultySettings>,
    progress: Res<ChapterProgress>,
    milestones: Res<Milestones>,
    ward: Query<&DeathWard, With<Player>>,
    mut events: MessageWriter<CombatEvent>,
) {
    if stats.elapsed_secs <= 360.0 {
        challenges.complete(ChallengeId::SwiftClear);
    }
    if let Ok(ward) = ward.single()
        && ward.revives_used == 0
    {
        challenges.complete(ChallengeId::FlawlessVictory);
    }
    if stats.potions_used == 0 {
        challenges.complete(ChallengeId::UntouchedFlask);
    }
    if enrage_denied_complete(&progress, &stats) {
        challenges.complete(ChallengeId::EnrageDenied);
    }
    if difficulty.current == Difficulty::Torment {
        challenges.complete(ChallengeId::TormentClear);
    }
    if primal_cache_complete(&stats) {
        challenges.complete(ChallengeId::PrimalCache);
    }
    if reliquary_conquest_complete(
        &milestones,
        progress.boss_slain,
        enrage_denied_complete(&progress, &stats),
    ) {
        challenges.complete(ChallengeId::ReliquaryConquest);
    }
    let reward = challenges.claim_completed_rewards();
    if reward.gold == 0 && reward.ember_shards == 0 && reward.affix_essence == 0 {
        return;
    }
    stats.gold += reward.gold;
    stats.ember_shards += reward.ember_shards;
    stats.affix_essence += reward.affix_essence;
    events.write(CombatEvent {
        text: format!(
            "Challenge rewards: {} gold, {} shards, {} essence",
            reward.gold, reward.ember_shards, reward.affix_essence
        ),
    });
}

fn reliquary_conquest_complete(
    milestones: &Milestones,
    boss_slain: bool,
    ashen_duel_complete: bool,
) -> bool {
    ALL_MILESTONES.iter().all(|milestone| {
        milestones.unlocked.contains(milestone)
            || (*milestone == MilestoneId::BossSlain && boss_slain)
            || (*milestone == MilestoneId::AshenDuel && ashen_duel_complete)
    })
}

fn enrage_denied_complete(progress: &ChapterProgress, stats: &RunStats) -> bool {
    progress.boss_slain && !stats.boss_enraged
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::player::{GemKind, LegendaryPower, SocketedGem};

    #[test]
    fn challenges_complete_once_and_claim_rewards_once() {
        let mut challenges = ChapterChallenges::new();

        assert!(challenges.complete(ChallengeId::TreasureHunter));
        assert!(!challenges.complete(ChallengeId::TreasureHunter));

        let reward = challenges.claim_completed_rewards();
        assert_eq!(reward.gold, 110);
        assert_eq!(reward.ember_shards, 3);
        assert_eq!(reward.affix_essence, 0);
        assert_eq!(
            challenges.claim_completed_rewards(),
            ChallengeReward {
                gold: 0,
                ember_shards: 0,
                affix_essence: 0
            }
        );
    }

    #[test]
    fn elite_hunter_rewards_affix_essence() {
        let mut challenges = ChapterChallenges::new();

        assert!(challenges.complete(ChallengeId::EliteHunter));
        let reward = challenges.claim_completed_rewards();

        assert_eq!(reward.gold, 130);
        assert_eq!(reward.ember_shards, 2);
        assert_eq!(reward.affix_essence, 5);
        assert_eq!(challenges.reward_essence, 5);
    }

    #[test]
    fn treasure_vault_rewards_opening_imp_vault() {
        let mut challenges = ChapterChallenges::new();

        assert!(challenges.complete(ChallengeId::TreasureVault));
        let reward = challenges.claim_completed_rewards();

        assert_eq!(reward.gold, 160);
        assert_eq!(reward.ember_shards, 5);
        assert_eq!(reward.affix_essence, 4);
        assert_eq!(challenges.reward_essence, 4);
    }

    #[test]
    fn treasure_vault_requires_opened_vault() {
        let mut stats = RunStats::default();

        assert!(!treasure_vault_complete(&stats));

        stats.treasure_vaults_opened = 1;
        assert!(treasure_vault_complete(&stats));
    }

    #[test]
    fn carnage_master_rewards_ten_kill_massacre() {
        let mut challenges = ChapterChallenges::new();

        assert!(challenges.complete(ChallengeId::CarnageMaster));
        let reward = challenges.claim_completed_rewards();

        assert_eq!(reward.gold, 180);
        assert_eq!(reward.ember_shards, 5);
        assert_eq!(reward.affix_essence, 5);
        assert_eq!(challenges.reward_essence, 5);
    }

    #[test]
    fn carnage_master_requires_ten_kill_massacre() {
        let mut stats = RunStats {
            best_massacre_streak: CARNAGE_MASTER_STREAK - 1,
            ..default()
        };

        assert!(!carnage_master_complete(&stats));

        stats.best_massacre_streak = CARNAGE_MASTER_STREAK;
        assert!(carnage_master_complete(&stats));
    }

    #[test]
    fn battle_trance_rewards_massacre_and_full_valor() {
        let mut challenges = ChapterChallenges::new();

        assert!(challenges.complete(ChallengeId::BattleTrance));
        let reward = challenges.claim_completed_rewards();

        assert_eq!(reward.gold, 160);
        assert_eq!(reward.ember_shards, 4);
        assert_eq!(reward.affix_essence, 5);
        assert_eq!(challenges.reward_essence, 5);
    }

    #[test]
    fn battle_trance_requires_massacre_and_full_valor() {
        let mut stats = RunStats {
            best_massacre_streak: 5,
            best_valor_stacks: 4,
            ..default()
        };

        assert!(!battle_trance_complete(&stats));

        stats.best_valor_stacks = 5;
        assert!(battle_trance_complete(&stats));
    }

    #[test]
    fn vaultbreaker_rewards_opening_the_resplendent_vault() {
        let mut challenges = ChapterChallenges::new();

        assert!(challenges.complete(ChallengeId::Vaultbreaker));
        let reward = challenges.claim_completed_rewards();

        assert_eq!(reward.gold, 125);
        assert_eq!(reward.ember_shards, 3);
        assert_eq!(reward.affix_essence, 3);
        assert_eq!(challenges.reward_essence, 3);
    }

    #[test]
    fn ritekeeper_rewards_completing_the_blood_obelisk() {
        let mut challenges = ChapterChallenges::new();

        assert!(challenges.complete(ChallengeId::Ritekeeper));
        let reward = challenges.claim_completed_rewards();

        assert_eq!(reward.gold, 140);
        assert_eq!(reward.ember_shards, 4);
        assert_eq!(reward.affix_essence, 4);
        assert_eq!(challenges.reward_essence, 4);
    }

    #[test]
    fn riftwalker_rewards_sealing_the_ember_rift() {
        let mut challenges = ChapterChallenges::new();

        assert!(challenges.complete(ChallengeId::Riftwalker));
        let reward = challenges.claim_completed_rewards();

        assert_eq!(reward.gold, 155);
        assert_eq!(reward.ember_shards, 5);
        assert_eq!(reward.affix_essence, 5);
        assert_eq!(challenges.reward_essence, 5);
    }

    #[test]
    fn riftbreaker_rewards_fast_ember_rift_seals() {
        let mut challenges = ChapterChallenges::new();

        assert!(challenges.complete(ChallengeId::Riftbreaker));
        let reward = challenges.claim_completed_rewards();

        assert_eq!(reward.gold, 190);
        assert_eq!(reward.ember_shards, 6);
        assert_eq!(reward.affix_essence, 7);
        assert_eq!(challenges.reward_essence, 7);
    }

    #[test]
    fn riftbreaker_requires_swift_rift_best_time() {
        let mut rift = EmberRift {
            best_remaining_secs: RIFT_SWIFT_SEAL_SECS - 0.1,
            ..EmberRift::default()
        };

        assert!(!riftbreaker_complete(&rift));

        rift.best_remaining_secs = RIFT_SWIFT_SEAL_SECS;
        assert!(riftbreaker_complete(&rift));
    }

    #[test]
    fn pylon_overload_rewards_activating_the_ashen_pylon() {
        let mut challenges = ChapterChallenges::new();

        assert!(challenges.complete(ChallengeId::PylonOverload));
        let reward = challenges.claim_completed_rewards();

        assert_eq!(reward.gold, 135);
        assert_eq!(reward.ember_shards, 4);
        assert_eq!(reward.affix_essence, 4);
        assert_eq!(challenges.reward_essence, 4);
    }

    #[test]
    fn pylon_reaper_rewards_ashen_pylon_kills() {
        let mut challenges = ChapterChallenges::new();

        assert!(challenges.complete(ChallengeId::PylonReaper));
        let reward = challenges.claim_completed_rewards();

        assert_eq!(reward.gold, 175);
        assert_eq!(reward.ember_shards, 5);
        assert_eq!(reward.affix_essence, 6);
        assert_eq!(challenges.reward_essence, 6);
    }

    #[test]
    fn pylon_reaper_requires_three_pylon_kills() {
        let mut stats = RunStats {
            pylon_kills: PYLON_REAPER_KILLS - 1,
            ..default()
        };

        assert!(!pylon_reaper_complete(&stats));

        stats.pylon_kills = PYLON_REAPER_KILLS;
        assert!(pylon_reaper_complete(&stats));
    }

    #[test]
    fn soul_sigil_rewards_recovering_malrec_sigil() {
        let mut challenges = ChapterChallenges::new();

        assert!(challenges.complete(ChallengeId::SoulSigil));
        let reward = challenges.claim_completed_rewards();

        assert_eq!(reward.gold, 180);
        assert_eq!(reward.ember_shards, 5);
        assert_eq!(reward.affix_essence, 7);
        assert_eq!(challenges.reward_essence, 7);
    }

    #[test]
    fn soul_sigil_requires_boss_trophy_material() {
        let mut stats = RunStats::default();

        assert!(!soul_sigil_complete(&stats));

        stats.malrec_soul_sigils = 1;
        assert!(soul_sigil_complete(&stats));
    }

    #[test]
    fn trophy_cache_rewards_opening_malrec_trophy_cache() {
        let mut challenges = ChapterChallenges::new();

        assert!(challenges.complete(ChallengeId::TrophyCache));
        let reward = challenges.claim_completed_rewards();

        assert_eq!(reward.gold, 210);
        assert_eq!(reward.ember_shards, 6);
        assert_eq!(reward.affix_essence, 8);
        assert_eq!(challenges.reward_essence, 8);
    }

    #[test]
    fn trophy_cache_requires_opened_soul_sigil_cache() {
        let mut stats = RunStats::default();

        assert!(!trophy_cache_complete(&stats));

        stats.soul_sigil_caches = 1;
        assert!(trophy_cache_complete(&stats));
    }

    #[test]
    fn boss_breaker_rewards_repeated_malrec_staggers() {
        let mut challenges = ChapterChallenges::new();

        assert!(challenges.complete(ChallengeId::BossBreaker));
        let reward = challenges.claim_completed_rewards();

        assert_eq!(reward.gold, 190);
        assert_eq!(reward.ember_shards, 6);
        assert_eq!(reward.affix_essence, 8);
        assert_eq!(challenges.reward_essence, 8);
    }

    #[test]
    fn boss_breaker_requires_two_boss_breaks() {
        let mut stats = RunStats {
            boss_staggers: BOSS_BREAK_TARGET - 1,
            ..default()
        };

        assert!(!boss_breaker_complete(&stats));

        stats.boss_staggers = BOSS_BREAK_TARGET;
        assert!(boss_breaker_complete(&stats));
    }

    #[test]
    fn breaker_rewards_smashing_all_breakables() {
        let mut challenges = ChapterChallenges::new();

        assert!(challenges.complete(ChallengeId::Breaker));
        let reward = challenges.claim_completed_rewards();

        assert_eq!(reward.gold, 90);
        assert_eq!(reward.ember_shards, 2);
        assert_eq!(reward.affix_essence, 2);
        assert_eq!(challenges.reward_essence, 2);
    }

    #[test]
    fn breaker_requires_all_breakables_smashed() {
        let mut stats = RunStats {
            breakables_smashed: BREAKER_TARGET_BREAKABLES - 1,
            ..default()
        };

        assert!(!breaker_complete(&stats));

        stats.breakables_smashed = BREAKER_TARGET_BREAKABLES;
        assert!(breaker_complete(&stats));
    }

    #[test]
    fn shrine_resonance_rewards_chaining_shrine_blessings() {
        let mut challenges = ChapterChallenges::new();

        assert!(challenges.complete(ChallengeId::ShrineResonance));
        let reward = challenges.claim_completed_rewards();

        assert_eq!(reward.gold, 125);
        assert_eq!(reward.ember_shards, 3);
        assert_eq!(reward.affix_essence, 3);
        assert_eq!(challenges.reward_essence, 3);
    }

    #[test]
    fn shrine_resonance_requires_triggered_resonance() {
        let mut stats = RunStats::default();

        assert!(!shrine_resonance_complete(&stats));

        stats.shrine_resonance_triggered = true;
        assert!(shrine_resonance_complete(&stats));
    }

    #[test]
    fn shrinekeeper_rewards_activating_every_shrine_route() {
        let mut challenges = ChapterChallenges::new();

        assert!(challenges.complete(ChallengeId::Shrinekeeper));
        let reward = challenges.claim_completed_rewards();

        assert_eq!(reward.gold, 170);
        assert_eq!(reward.ember_shards, 5);
        assert_eq!(reward.affix_essence, 5);
        assert_eq!(challenges.reward_essence, 5);
    }

    #[test]
    fn shrinekeeper_requires_all_reliquary_shrine_milestones() {
        let mut milestones = Milestones::default();

        milestones.unlock(MilestoneId::RelicEmpowered);
        milestones.unlock(MilestoneId::FortuneBlessed);
        milestones.unlock(MilestoneId::StormConduit);
        assert!(!shrinekeeper_complete(&milestones));

        milestones.unlock(MilestoneId::AshenPylon);
        assert!(shrinekeeper_complete(&milestones));
    }

    #[test]
    fn affix_hunter_rewards_multi_affix_elite_kills() {
        let mut challenges = ChapterChallenges::new();

        assert!(challenges.complete(ChallengeId::AffixHunter));
        let reward = challenges.claim_completed_rewards();

        assert_eq!(reward.gold, 185);
        assert_eq!(reward.ember_shards, 5);
        assert_eq!(reward.affix_essence, 7);
        assert_eq!(challenges.reward_essence, 7);
    }

    #[test]
    fn affix_hunter_requires_three_multi_affix_elite_kills() {
        let mut stats = RunStats {
            elite_affix_kills: AFFIX_HUNTER_KILLS - 1,
            ..default()
        };

        assert!(!affix_hunter_complete(&stats));

        stats.elite_affix_kills = AFFIX_HUNTER_KILLS;
        assert!(affix_hunter_complete(&stats));
    }

    #[test]
    fn affix_codex_rewards_distinct_elite_affixes() {
        let mut challenges = ChapterChallenges::new();

        assert!(challenges.complete(ChallengeId::AffixCodex));
        let reward = challenges.claim_completed_rewards();

        assert_eq!(reward.gold, 210);
        assert_eq!(reward.ember_shards, 6);
        assert_eq!(reward.affix_essence, 8);
        assert_eq!(challenges.reward_essence, 8);
    }

    #[test]
    fn affix_codex_requires_six_distinct_affixes() {
        let mut stats = RunStats {
            affix_codex_mask: 0b1_1111,
            ..default()
        };

        assert_eq!(affix_codex_count(&stats), 5);
        assert!(!affix_codex_complete(&stats));

        stats.affix_codex_mask |= 1 << 5;

        assert_eq!(affix_codex_count(&stats), AFFIX_CODEX_TARGET);
        assert!(affix_codex_complete(&stats));
    }

    #[test]
    fn seal_warden_rewards_inner_sanctum_elite_kill() {
        let mut challenges = ChapterChallenges::new();

        assert!(challenges.complete(ChallengeId::SealWarden));
        let reward = challenges.claim_completed_rewards();

        assert_eq!(reward.gold, 115);
        assert_eq!(reward.ember_shards, 3);
        assert_eq!(reward.affix_essence, 3);
        assert_eq!(challenges.reward_essence, 3);
    }

    #[test]
    fn seal_warden_requires_named_sanctum_elite() {
        let mut stats = RunStats::default();

        assert!(!seal_warden_complete(&stats));

        stats.seal_warden_slain = true;
        assert!(seal_warden_complete(&stats));
    }

    #[test]
    fn ashen_threat_rewards_three_threat_surges() {
        let mut challenges = ChapterChallenges::new();

        assert!(challenges.complete(ChallengeId::AshenThreat));
        let reward = challenges.claim_completed_rewards();

        assert_eq!(reward.gold, 150);
        assert_eq!(reward.ember_shards, 5);
        assert_eq!(reward.affix_essence, 4);
        assert_eq!(challenges.reward_essence, 4);
    }

    #[test]
    fn ashen_threat_requires_three_surges() {
        let mut stats = RunStats {
            ashen_threat_surges: ASHEN_THREAT_SURGE_TARGET - 1,
            ..default()
        };

        assert!(!ashen_threat_complete(&stats));

        stats.ashen_threat_surges = ASHEN_THREAT_SURGE_TARGET;
        assert!(ashen_threat_complete(&stats));
    }

    #[test]
    fn champion_breaker_rewards_roaming_champion_pack() {
        let mut challenges = ChapterChallenges::new();

        assert!(challenges.complete(ChallengeId::ChampionBreaker));
        let reward = challenges.claim_completed_rewards();

        assert_eq!(reward.gold, 145);
        assert_eq!(reward.ember_shards, 4);
        assert_eq!(reward.affix_essence, 4);
        assert_eq!(challenges.reward_essence, 4);
    }

    #[test]
    fn champion_breaker_requires_pack_reward_claimed() {
        let mut stats = RunStats {
            champion_pack_kills: CHAMPION_PACK_TARGET,
            ..default()
        };

        assert!(!champion_breaker_complete(&stats));

        stats.champion_pack_reward_claimed = true;
        assert!(champion_breaker_complete(&stats));
    }

    #[test]
    fn nemesis_hunter_rewards_slaying_soul_bound_elite() {
        let mut challenges = ChapterChallenges::new();

        assert!(challenges.complete(ChallengeId::NemesisHunter));
        let reward = challenges.claim_completed_rewards();

        assert_eq!(reward.gold, 150);
        assert_eq!(reward.ember_shards, 4);
        assert_eq!(reward.affix_essence, 6);
        assert_eq!(challenges.reward_essence, 6);
    }

    #[test]
    fn sealbreaker_rewards_core_optional_objectives() {
        let mut challenges = ChapterChallenges::new();

        assert!(challenges.complete(ChallengeId::Sealbreaker));
        let reward = challenges.claim_completed_rewards();

        assert_eq!(reward.gold, 220);
        assert_eq!(reward.ember_shards, 7);
        assert_eq!(reward.affix_essence, 8);
        assert_eq!(challenges.reward_essence, 8);
    }

    #[test]
    fn sealbreaker_requires_vault_obelisk_and_rift_milestones() {
        let mut milestones = Milestones::default();

        milestones.unlock(MilestoneId::VaultOpened);
        milestones.unlock(MilestoneId::BloodObelisk);
        assert!(!sealbreaker_complete(&milestones));

        milestones.unlock(MilestoneId::EmberRift);
        assert!(sealbreaker_complete(&milestones));
    }

    #[test]
    fn cursed_pact_rewards_cursed_shrine_ambush_kills() {
        let mut challenges = ChapterChallenges::new();

        assert!(challenges.complete(ChallengeId::CursedPact));
        let reward = challenges.claim_completed_rewards();

        assert_eq!(reward.gold, 145);
        assert_eq!(reward.ember_shards, 4);
        assert_eq!(reward.affix_essence, 5);
        assert_eq!(challenges.reward_essence, 5);
    }

    #[test]
    fn cursed_pact_requires_all_cursed_ambushers() {
        let mut stats = RunStats {
            cursed_ambush_kills: CURSED_AMBUSH_KILLS - 1,
            ..default()
        };

        assert!(!cursed_pact_complete(&stats));

        stats.cursed_ambush_kills = CURSED_AMBUSH_KILLS;
        assert!(cursed_pact_complete(&stats));
    }

    #[test]
    fn codex_archivist_rewards_completing_legendary_collection() {
        let mut challenges = ChapterChallenges::new();

        assert!(challenges.complete(ChallengeId::CodexArchivist));
        let reward = challenges.claim_completed_rewards();

        assert_eq!(reward.gold, 180);
        assert_eq!(reward.ember_shards, 5);
        assert_eq!(reward.affix_essence, 6);
        assert_eq!(challenges.reward_essence, 6);
    }

    #[test]
    fn codex_adept_rewards_attuned_power_kills() {
        let mut challenges = ChapterChallenges::new();

        assert!(challenges.complete(ChallengeId::CodexAdept));
        let reward = challenges.claim_completed_rewards();

        assert_eq!(reward.gold, 150);
        assert_eq!(reward.ember_shards, 4);
        assert_eq!(reward.affix_essence, 5);
        assert_eq!(challenges.reward_essence, 5);
    }

    #[test]
    fn codex_adept_requires_attuned_power_kill_target() {
        let mut stats = RunStats {
            codex_attuned_kills: CODEX_ADEPT_KILLS - 1,
            ..RunStats::default()
        };

        assert!(!codex_adept_complete(&stats));

        stats.codex_attuned_kills = CODEX_ADEPT_KILLS;
        assert!(codex_adept_complete(&stats));
    }

    #[test]
    fn set_builder_rewards_themed_resonance_builds() {
        let mut challenges = ChapterChallenges::new();

        assert!(challenges.complete(ChallengeId::SetBuilder));
        let reward = challenges.claim_completed_rewards();

        assert_eq!(reward.gold, 150);
        assert_eq!(reward.ember_shards, 4);
        assert_eq!(reward.affix_essence, 5);
        assert_eq!(challenges.reward_essence, 5);
    }

    #[test]
    fn set_adept_rewards_set_resonance_kills() {
        let mut challenges = ChapterChallenges::new();

        assert!(challenges.complete(ChallengeId::SetAdept));
        let reward = challenges.claim_completed_rewards();

        assert_eq!(reward.gold, 165);
        assert_eq!(reward.ember_shards, 5);
        assert_eq!(reward.affix_essence, 5);
        assert_eq!(challenges.reward_essence, 5);
    }

    #[test]
    fn set_adept_requires_set_resonance_kill_target() {
        let mut stats = RunStats {
            set_resonance_kills: SET_ADEPT_KILLS - 1,
            ..RunStats::default()
        };

        assert!(!set_adept_complete(&stats));

        stats.set_resonance_kills = SET_ADEPT_KILLS;
        assert!(set_adept_complete(&stats));
    }

    #[test]
    fn ancient_armory_rewards_equipping_ancient_weapon() {
        let mut challenges = ChapterChallenges::new();

        assert!(challenges.complete(ChallengeId::AncientArmory));
        let reward = challenges.claim_completed_rewards();

        assert_eq!(reward.gold, 170);
        assert_eq!(reward.ember_shards, 5);
        assert_eq!(reward.affix_essence, 6);
        assert_eq!(challenges.reward_essence, 6);
    }

    #[test]
    fn master_gemcutter_rewards_rank_five_socketed_gems() {
        let mut challenges = ChapterChallenges::new();

        assert!(challenges.complete(ChallengeId::MasterGemcutter));
        let reward = challenges.claim_completed_rewards();

        assert_eq!(reward.gold, 175);
        assert_eq!(reward.ember_shards, 5);
        assert_eq!(reward.affix_essence, 5);
        assert_eq!(challenges.reward_essence, 5);
    }

    #[test]
    fn gem_resonance_rewards_rank_three_socketed_gems() {
        let mut challenges = ChapterChallenges::new();

        assert!(challenges.complete(ChallengeId::GemResonance));
        let reward = challenges.claim_completed_rewards();

        assert_eq!(reward.gold, 150);
        assert_eq!(reward.ember_shards, 4);
        assert_eq!(reward.affix_essence, 4);
        assert_eq!(challenges.reward_essence, 4);
    }

    #[test]
    fn gem_resonance_requires_rank_three_socketed_gem() {
        let mut equipment = Equipment {
            weapon_name: "Gem Test".to_string(),
            quality: "rare".to_string(),
            crit_chance: 0.05,
            health_bonus: 0.0,
            armor_bonus: 0.0,
            legendary_power: LegendaryPower::None,
            temper_level: 0,
            socketed_gem: Some(SocketedGem {
                kind: GemKind::Ruby,
                rank: 2,
            }),
            worn: Equipment::empty_worn(),
        };

        assert!(!gem_resonance_complete(&equipment));

        equipment.socketed_gem = Some(SocketedGem {
            kind: GemKind::Ruby,
            rank: 3,
        });
        assert!(gem_resonance_complete(&equipment));
        assert!(!master_gemcutter_complete(&equipment));
    }

    #[test]
    fn master_gemcutter_requires_rank_five_socketed_gem() {
        let mut equipment = Equipment {
            weapon_name: "Gem Test".to_string(),
            quality: "rare".to_string(),
            crit_chance: 0.05,
            health_bonus: 0.0,
            armor_bonus: 0.0,
            legendary_power: LegendaryPower::None,
            temper_level: 0,
            socketed_gem: Some(SocketedGem {
                kind: GemKind::Ruby,
                rank: 4,
            }),
            worn: Equipment::empty_worn(),
        };

        assert!(!master_gemcutter_complete(&equipment));

        equipment.socketed_gem = Some(SocketedGem {
            kind: GemKind::Ruby,
            rank: 5,
        });
        assert!(master_gemcutter_complete(&equipment));
    }

    #[test]
    fn gem_adept_rewards_socketing_every_gem_kind() {
        let mut challenges = ChapterChallenges::new();

        assert!(challenges.complete(ChallengeId::GemAdept));
        let reward = challenges.claim_completed_rewards();

        assert_eq!(reward.gold, 155);
        assert_eq!(reward.ember_shards, 4);
        assert_eq!(reward.affix_essence, 5);
        assert_eq!(challenges.reward_essence, 5);
    }

    #[test]
    fn gem_adept_requires_all_four_socketed_gem_kinds() {
        let mut stats = RunStats {
            ruby_socketed: true,
            emerald_socketed: true,
            amethyst_socketed: true,
            ..RunStats::default()
        };

        assert_eq!(gem_adept_count(&stats), 3);
        assert!(!gem_adept_complete(&stats));

        stats.topaz_socketed = true;
        assert_eq!(gem_adept_count(&stats), GEM_ADEPT_KINDS);
        assert!(gem_adept_complete(&stats));
    }

    #[test]
    fn augmenter_rewards_applying_an_ancient_augment() {
        let mut challenges = ChapterChallenges::new();

        assert!(challenges.complete(ChallengeId::Augmenter));
        let reward = challenges.claim_completed_rewards();

        assert_eq!(reward.gold, 190);
        assert_eq!(reward.ember_shards, 5);
        assert_eq!(reward.affix_essence, 7);
        assert_eq!(challenges.reward_essence, 7);
    }

    #[test]
    fn paragon_awakened_rewards_first_ember_paragon_rank() {
        let mut challenges = ChapterChallenges::new();

        assert!(challenges.complete(ChallengeId::ParagonAwakened));
        let reward = challenges.claim_completed_rewards();

        assert_eq!(reward.gold, 210);
        assert_eq!(reward.ember_shards, 6);
        assert_eq!(reward.affix_essence, 8);
        assert_eq!(challenges.reward_essence, 8);
    }

    #[test]
    fn talent_adept_rewards_investing_in_each_tree() {
        let mut challenges = ChapterChallenges::new();

        assert!(challenges.complete(ChallengeId::TalentAdept));
        let reward = challenges.claim_completed_rewards();

        assert_eq!(reward.gold, 145);
        assert_eq!(reward.ember_shards, 4);
        assert_eq!(reward.affix_essence, 4);
        assert_eq!(challenges.reward_essence, 4);
    }

    #[test]
    fn talent_adept_requires_wrath_vigor_and_focus() {
        let mut talents = Talents {
            wrath: 1,
            vigor: 1,
            focus: 0,
            points: 0,
        };

        assert!(!talent_adept_complete(&talents));

        talents.focus = 1;
        assert!(talent_adept_complete(&talents));
    }

    #[test]
    fn quartermaster_patron_rewards_full_logistics_upgrades() {
        let mut challenges = ChapterChallenges::new();

        assert!(challenges.complete(ChallengeId::QuartermasterPatron));
        let reward = challenges.claim_completed_rewards();

        assert_eq!(reward.gold, 180);
        assert_eq!(reward.ember_shards, 5);
        assert_eq!(reward.affix_essence, 5);
        assert_eq!(challenges.reward_essence, 5);
    }

    #[test]
    fn alchemist_rewards_using_every_elixir_type() {
        let mut challenges = ChapterChallenges::new();

        assert!(challenges.complete(ChallengeId::Alchemist));
        let reward = challenges.claim_completed_rewards();

        assert_eq!(reward.gold, 150);
        assert_eq!(reward.ember_shards, 4);
        assert_eq!(reward.affix_essence, 4);
        assert_eq!(challenges.reward_essence, 4);
    }

    #[test]
    fn ashen_alchemy_rewards_completing_all_transmutations() {
        let mut challenges = ChapterChallenges::new();

        assert!(challenges.complete(ChallengeId::AshenAlchemy));
        let reward = challenges.claim_completed_rewards();

        assert_eq!(reward.gold, 170);
        assert_eq!(reward.ember_shards, 5);
        assert_eq!(reward.affix_essence, 6);
        assert_eq!(challenges.reward_essence, 6);
    }

    #[test]
    fn ashen_alchemy_requires_gold_essence_and_keystone_recipes() {
        let mut stats = RunStats {
            alchemy_gold_transmutes: 1,
            alchemy_essence_transmutes: 1,
            ..RunStats::default()
        };

        assert!(!ashen_alchemy_complete(&stats));

        stats.alchemy_keystone_transmutes = 1;
        assert!(ashen_alchemy_complete(&stats));
    }

    #[test]
    fn loot_curator_rewards_filter_use() {
        let mut challenges = ChapterChallenges::new();

        assert!(challenges.complete(ChallengeId::LootCurator));
        let reward = challenges.claim_completed_rewards();

        assert_eq!(reward.gold, 95);
        assert_eq!(reward.ember_shards, 3);
        assert_eq!(reward.affix_essence, 2);
        assert_eq!(challenges.reward_essence, 2);
    }

    #[test]
    fn loot_curator_requires_three_filter_cycles() {
        let mut stats = RunStats {
            loot_filter_cycles: 2,
            ..RunStats::default()
        };

        assert!(!loot_curator_complete(&stats));

        stats.loot_filter_cycles = 3;
        assert!(loot_curator_complete(&stats));
    }

    #[test]
    fn untouched_flask_rewards_potionless_victory() {
        let mut challenges = ChapterChallenges::new();

        assert!(challenges.complete(ChallengeId::UntouchedFlask));
        let reward = challenges.claim_completed_rewards();

        assert_eq!(reward.gold, 180);
        assert_eq!(reward.ember_shards, 5);
        assert_eq!(reward.affix_essence, 5);
        assert_eq!(challenges.reward_essence, 5);
    }

    #[test]
    fn last_stand_rewards_clutch_potion_use() {
        let mut challenges = ChapterChallenges::new();

        assert!(challenges.complete(ChallengeId::LastStand));
        let reward = challenges.claim_completed_rewards();

        assert_eq!(reward.gold, 120);
        assert_eq!(reward.ember_shards, 3);
        assert_eq!(reward.affix_essence, 4);
        assert_eq!(challenges.reward_essence, 4);
    }

    #[test]
    fn last_stand_requires_three_clutch_potions() {
        let mut stats = RunStats {
            last_stand_potions: LAST_STAND_POTIONS - 1,
            ..RunStats::default()
        };

        assert!(!last_stand_complete(&stats));

        stats.last_stand_potions = LAST_STAND_POTIONS;
        assert!(last_stand_complete(&stats));
    }

    #[test]
    fn surge_mastery_rewards_surge_window_kills() {
        let mut challenges = ChapterChallenges::new();

        assert!(challenges.complete(ChallengeId::SurgeMastery));
        let reward = challenges.claim_completed_rewards();

        assert_eq!(reward.gold, 165);
        assert_eq!(reward.ember_shards, 4);
        assert_eq!(reward.affix_essence, 5);
        assert_eq!(challenges.reward_essence, 5);
    }

    #[test]
    fn surge_mastery_requires_three_surge_kills() {
        let mut stats = RunStats {
            surge_kills: SURGE_CHAIN_KILLS - 1,
            ..default()
        };

        assert!(!surge_mastery_complete(&stats));

        stats.surge_kills = SURGE_CHAIN_KILLS;
        assert!(surge_mastery_complete(&stats));
    }

    #[test]
    fn rune_weaver_rewards_using_alternate_skill_runes() {
        let mut challenges = ChapterChallenges::new();

        assert!(challenges.complete(ChallengeId::RuneWeaver));
        let reward = challenges.claim_completed_rewards();

        assert_eq!(reward.gold, 155);
        assert_eq!(reward.ember_shards, 4);
        assert_eq!(reward.affix_essence, 5);
        assert_eq!(challenges.reward_essence, 5);
    }

    #[test]
    fn rune_weaver_requires_all_three_alternate_rune_hits() {
        let mut stats = RunStats {
            reap_dash_hits: 1,
            hemorrhage_rupture_hits: 1,
            ..default()
        };

        assert!(!rune_weaver_complete(&stats));

        stats.frost_nova_hits = 1;
        assert!(rune_weaver_complete(&stats));
    }

    #[test]
    fn armory_adept_rewards_saved_loadout_slots() {
        let mut challenges = ChapterChallenges::new();

        assert!(challenges.complete(ChallengeId::ArmoryAdept));
        let reward = challenges.claim_completed_rewards();

        assert_eq!(reward.gold, 150);
        assert_eq!(reward.ember_shards, 4);
        assert_eq!(reward.affix_essence, 4);
        assert_eq!(challenges.reward_essence, 4);
    }

    #[test]
    fn armory_adept_requires_three_saved_loadouts() {
        let mut stats = RunStats {
            armory_loadouts_saved: ARMORY_SLOT_COUNT as u32 - 1,
            ..default()
        };

        assert!(!armory_adept_complete(&stats));

        stats.armory_loadouts_saved = ARMORY_SLOT_COUNT as u32;
        assert!(armory_adept_complete(&stats));
    }

    #[test]
    fn portal_wayfarer_rewards_returning_to_the_fight() {
        let mut challenges = ChapterChallenges::new();

        assert!(challenges.complete(ChallengeId::PortalWayfarer));
        let reward = challenges.claim_completed_rewards();

        assert_eq!(reward.gold, 120);
        assert_eq!(reward.ember_shards, 3);
        assert_eq!(reward.affix_essence, 3);
        assert_eq!(challenges.reward_essence, 3);
    }

    #[test]
    fn portal_wayfarer_requires_a_town_portal_return() {
        let mut stats = RunStats::default();

        assert!(!portal_wayfarer_complete(&stats));

        stats.town_portal_returns = 1;
        assert!(portal_wayfarer_complete(&stats));
    }

    #[test]
    fn glory_seeker_rewards_health_globe_pickups() {
        let mut challenges = ChapterChallenges::new();

        assert!(challenges.complete(ChallengeId::GlorySeeker));
        let reward = challenges.claim_completed_rewards();

        assert_eq!(reward.gold, 125);
        assert_eq!(reward.ember_shards, 3);
        assert_eq!(reward.affix_essence, 4);
        assert_eq!(challenges.reward_essence, 4);
    }

    #[test]
    fn glory_seeker_requires_three_health_globes() {
        let mut stats = RunStats {
            health_globes_collected: GLORY_SEEKER_GLOBES - 1,
            ..default()
        };

        assert!(!glory_seeker_complete(&stats));

        stats.health_globes_collected = GLORY_SEEKER_GLOBES;
        assert!(glory_seeker_complete(&stats));
    }

    #[test]
    fn salvage_cache_rewards_spare_gear_caches() {
        let mut challenges = ChapterChallenges::new();

        assert!(challenges.complete(ChallengeId::SalvageCache));
        let reward = challenges.claim_completed_rewards();

        assert_eq!(reward.gold, 140);
        assert_eq!(reward.ember_shards, 4);
        assert_eq!(reward.affix_essence, 4);
        assert_eq!(challenges.reward_essence, 4);
    }

    #[test]
    fn salvage_cache_requires_a_claimed_cache() {
        let mut stats = RunStats::default();

        assert!(!salvage_cache_complete(&stats));

        stats.salvage_caches = 1;
        assert!(salvage_cache_complete(&stats));
    }

    #[test]
    fn sentinel_veteran_rewards_companion_progression() {
        let mut challenges = ChapterChallenges::new();

        assert!(challenges.complete(ChallengeId::SentinelVeteran));
        let reward = challenges.claim_completed_rewards();

        assert_eq!(reward.gold, 160);
        assert_eq!(reward.ember_shards, 4);
        assert_eq!(reward.affix_essence, 4);
        assert_eq!(challenges.reward_essence, 4);
    }

    #[test]
    fn sentinel_veteran_requires_rank_three_companion() {
        let mut companion = CompanionBond {
            rank: 2,
            xp: 0,
            next_rank_xp: 145,
            ..CompanionBond::default()
        };

        assert!(!sentinel_veteran_complete(&companion));

        companion.rank = 3;
        assert!(sentinel_veteran_complete(&companion));
    }

    #[test]
    fn boonbound_rewards_claiming_a_reliquary_boon() {
        let mut challenges = ChapterChallenges::new();

        assert!(challenges.complete(ChallengeId::Boonbound));
        let reward = challenges.claim_completed_rewards();

        assert_eq!(reward.gold, 150);
        assert_eq!(reward.ember_shards, 4);
        assert_eq!(reward.affix_essence, 4);
        assert_eq!(challenges.reward_essence, 4);
    }

    #[test]
    fn boonbound_requires_a_chosen_reliquary_boon() {
        let mut boon = ChapterBoon {
            choice: ChapterBoonChoice::None,
            empowered: false,
            momentum_stacks: 0,
        };

        assert!(!boonbound_complete(&boon));

        boon.choice = ChapterBoonChoice::CrimsonOath;
        assert!(boonbound_complete(&boon));
    }

    #[test]
    fn bounty_hunter_rewards_optional_bounty_completion() {
        let mut challenges = ChapterChallenges::new();

        assert!(challenges.complete(ChallengeId::BountyHunter));
        let reward = challenges.claim_completed_rewards();

        assert_eq!(reward.gold, 170);
        assert_eq!(reward.ember_shards, 5);
        assert_eq!(reward.affix_essence, 5);
        assert_eq!(challenges.reward_essence, 5);
    }

    #[test]
    fn alchemist_requires_all_elixir_types() {
        let mut stats = RunStats {
            iron_elixir_used: true,
            wrath_elixir_used: true,
            ..default()
        };

        assert!(!alchemist_complete(&stats));

        stats.haste_elixir_used = true;
        assert!(alchemist_complete(&stats));
    }

    #[test]
    fn quartermaster_patron_requires_all_logistics_upgrades() {
        let inventory = Inventory {
            items: vec![],
            capacity: 14,
        };
        let potions = PotionBelt {
            current: 5,
            max: 6,
            heal_amount: 45.0,
            cooldown: Timer::from_seconds(0.0, TimerMode::Once),
            cooldown_secs: 12.0,
        };
        let elixirs = ElixirBelt {
            current: 3,
            max: 3,
            armor_bonus: 35.0,
            duration_secs: 22.0,
            selected: default(),
        };

        assert!(!quartermaster_patron_complete(
            &inventory, &potions, &elixirs
        ));

        let upgraded_elixirs = ElixirBelt { max: 4, ..elixirs };
        assert!(quartermaster_patron_complete(
            &inventory,
            &potions,
            &upgraded_elixirs
        ));
    }

    #[test]
    fn bestiary_scholar_rewards_recording_every_monster_type() {
        let mut challenges = ChapterChallenges::new();

        assert!(challenges.complete(ChallengeId::BestiaryScholar));
        let reward = challenges.claim_completed_rewards();

        assert_eq!(reward.gold, 150);
        assert_eq!(reward.ember_shards, 4);
        assert_eq!(reward.affix_essence, 4);
        assert_eq!(challenges.reward_essence, 4);
    }

    #[test]
    fn lorekeeper_rewards_recovering_every_lore_page() {
        let mut challenges = ChapterChallenges::new();

        assert!(challenges.complete(ChallengeId::Lorekeeper));
        let reward = challenges.claim_completed_rewards();

        assert_eq!(reward.gold, 120);
        assert_eq!(reward.ember_shards, 4);
        assert_eq!(reward.affix_essence, 3);
        assert_eq!(challenges.reward_essence, 3);
    }

    #[test]
    fn flawless_victory_rewards_surviving_without_ward_revive() {
        let mut challenges = ChapterChallenges::new();

        assert!(challenges.complete(ChallengeId::FlawlessVictory));
        let reward = challenges.claim_completed_rewards();

        assert_eq!(reward.gold, 160);
        assert_eq!(reward.ember_shards, 4);
        assert_eq!(reward.affix_essence, 4);
        assert_eq!(challenges.reward_gold, 160);
        assert_eq!(challenges.reward_shards, 4);
        assert_eq!(challenges.reward_essence, 4);
    }

    #[test]
    fn enrage_denied_rewards_defeating_malrec_before_ashen_enrage() {
        let mut challenges = ChapterChallenges::new();

        assert!(challenges.complete(ChallengeId::EnrageDenied));
        let reward = challenges.claim_completed_rewards();

        assert_eq!(reward.gold, 200);
        assert_eq!(reward.ember_shards, 6);
        assert_eq!(reward.affix_essence, 7);
        assert_eq!(challenges.reward_essence, 7);
    }

    #[test]
    fn enrage_denied_requires_boss_slain_before_enrage() {
        let mut progress = ChapterProgress {
            boss_slain: false,
            ..default()
        };
        let mut stats = RunStats {
            boss_enraged: false,
            ..default()
        };

        assert!(!enrage_denied_complete(&progress, &stats));

        progress.boss_slain = true;
        stats.boss_enraged = true;
        assert!(!enrage_denied_complete(&progress, &stats));

        stats.boss_enraged = false;
        assert!(enrage_denied_complete(&progress, &stats));
    }

    #[test]
    fn torment_clear_rewards_highest_difficulty_completion() {
        let mut challenges = ChapterChallenges::new();

        assert!(challenges.complete(ChallengeId::TormentClear));
        let reward = challenges.claim_completed_rewards();

        assert_eq!(reward.gold, 320);
        assert_eq!(reward.ember_shards, 10);
        assert_eq!(reward.affix_essence, 12);
        assert_eq!(challenges.reward_essence, 12);
    }

    #[test]
    fn primal_cache_rewards_torment_endgame_cache_claims() {
        let mut challenges = ChapterChallenges::new();

        assert!(challenges.complete(ChallengeId::PrimalCache));
        let reward = challenges.claim_completed_rewards();

        assert_eq!(reward.gold, 240);
        assert_eq!(reward.ember_shards, 8);
        assert_eq!(reward.affix_essence, 10);
        assert_eq!(challenges.reward_essence, 10);
    }

    #[test]
    fn primal_infusion_rewards_endgame_crafting() {
        let mut challenges = ChapterChallenges::new();

        assert!(challenges.complete(ChallengeId::PrimalInfusion));
        let reward = challenges.claim_completed_rewards();

        assert_eq!(reward.gold, 260);
        assert_eq!(reward.ember_shards, 8);
        assert_eq!(reward.affix_essence, 12);
        assert_eq!(challenges.reward_essence, 12);
    }

    #[test]
    fn primal_infusion_requires_a_successful_infusion() {
        let mut stats = RunStats::default();

        assert!(!primal_infusion_complete(&stats));

        stats.primal_infusions = 1;
        assert!(primal_infusion_complete(&stats));
    }

    #[test]
    fn primal_cache_requires_a_claimed_cache() {
        let mut stats = RunStats::default();

        assert!(!primal_cache_complete(&stats));

        stats.primal_caches = 1;
        assert!(primal_cache_complete(&stats));
    }

    #[test]
    fn reliquary_conquest_rewards_full_milestone_completion() {
        let mut challenges = ChapterChallenges::new();

        assert!(challenges.complete(ChallengeId::ReliquaryConquest));
        let reward = challenges.claim_completed_rewards();

        assert_eq!(reward.gold, 260);
        assert_eq!(reward.ember_shards, 8);
        assert_eq!(reward.affix_essence, 10);
        assert_eq!(challenges.reward_essence, 10);
        assert!(
            challenge_summary(&challenges)
                .contains(&format!("1/{}", ChapterChallenges::new().challenges.len()))
        );
    }

    #[test]
    fn reliquary_conquest_accepts_pending_boss_milestone_on_victory() {
        let mut milestones = Milestones::default();
        for milestone in ALL_MILESTONES {
            if milestone != MilestoneId::BossSlain && milestone != MilestoneId::AshenDuel {
                milestones.unlock(milestone);
            }
        }

        assert!(reliquary_conquest_complete(&milestones, true, true));
        assert!(!reliquary_conquest_complete(&milestones, true, false));
        assert!(!reliquary_conquest_complete(&milestones, false, true));
    }

    #[test]
    fn restore_keeps_known_challenge_states() {
        let mut challenges = ChapterChallenges::new();
        challenges.restore(vec![ChallengeProgress {
            id: ChallengeId::Massacre,
            completed: true,
            reward_claimed: true,
        }]);

        let massacre = challenges
            .challenges
            .iter()
            .find(|challenge| challenge.id == ChallengeId::Massacre)
            .unwrap();

        assert!(massacre.completed);
        assert!(massacre.reward_claimed);
        assert!(
            challenge_summary(&challenges)
                .contains(&format!("1/{}", ChapterChallenges::new().challenges.len()))
        );
        assert!(
            challenges
                .challenges
                .iter()
                .any(|challenge| challenge.id == ChallengeId::EliteHunter)
        );
        assert!(
            challenges
                .challenges
                .iter()
                .any(|challenge| challenge.id == ChallengeId::TreasureVault)
        );
        assert!(
            challenges
                .challenges
                .iter()
                .any(|challenge| challenge.id == ChallengeId::CarnageMaster)
        );
        assert!(
            challenges
                .challenges
                .iter()
                .any(|challenge| challenge.id == ChallengeId::BattleTrance)
        );
        assert!(
            challenges
                .challenges
                .iter()
                .any(|challenge| challenge.id == ChallengeId::Vaultbreaker)
        );
        assert!(
            challenges
                .challenges
                .iter()
                .any(|challenge| challenge.id == ChallengeId::Breaker)
        );
        assert!(
            challenges
                .challenges
                .iter()
                .any(|challenge| challenge.id == ChallengeId::ShrineResonance)
        );
        assert!(
            challenges
                .challenges
                .iter()
                .any(|challenge| challenge.id == ChallengeId::Shrinekeeper)
        );
        assert!(
            challenges
                .challenges
                .iter()
                .any(|challenge| challenge.id == ChallengeId::AffixHunter)
        );
        assert!(
            challenges
                .challenges
                .iter()
                .any(|challenge| challenge.id == ChallengeId::AffixCodex)
        );
        assert!(
            challenges
                .challenges
                .iter()
                .any(|challenge| challenge.id == ChallengeId::SealWarden)
        );
        assert!(
            challenges
                .challenges
                .iter()
                .any(|challenge| challenge.id == ChallengeId::ChampionBreaker)
        );
        assert!(
            challenges
                .challenges
                .iter()
                .any(|challenge| challenge.id == ChallengeId::NemesisHunter)
        );
        assert!(
            challenges
                .challenges
                .iter()
                .any(|challenge| challenge.id == ChallengeId::CursedPact)
        );
        assert!(
            challenges
                .challenges
                .iter()
                .any(|challenge| challenge.id == ChallengeId::Ritekeeper)
        );
        assert!(
            challenges
                .challenges
                .iter()
                .any(|challenge| challenge.id == ChallengeId::Riftwalker)
        );
        assert!(
            challenges
                .challenges
                .iter()
                .any(|challenge| challenge.id == ChallengeId::Riftbreaker)
        );
        assert!(
            challenges
                .challenges
                .iter()
                .any(|challenge| challenge.id == ChallengeId::PylonOverload)
        );
        assert!(
            challenges
                .challenges
                .iter()
                .any(|challenge| challenge.id == ChallengeId::PylonReaper)
        );
        assert!(
            challenges
                .challenges
                .iter()
                .any(|challenge| challenge.id == ChallengeId::Sealbreaker)
        );
        assert!(
            challenges
                .challenges
                .iter()
                .any(|challenge| challenge.id == ChallengeId::SetBuilder)
        );
        assert!(
            challenges
                .challenges
                .iter()
                .any(|challenge| challenge.id == ChallengeId::SetAdept)
        );
        assert!(
            challenges
                .challenges
                .iter()
                .any(|challenge| challenge.id == ChallengeId::AncientArmory)
        );
        assert!(
            challenges
                .challenges
                .iter()
                .any(|challenge| challenge.id == ChallengeId::Augmenter)
        );
        assert!(
            challenges
                .challenges
                .iter()
                .any(|challenge| challenge.id == ChallengeId::GemResonance)
        );
        assert!(
            challenges
                .challenges
                .iter()
                .any(|challenge| challenge.id == ChallengeId::MasterGemcutter)
        );
        assert!(
            challenges
                .challenges
                .iter()
                .any(|challenge| challenge.id == ChallengeId::GemAdept)
        );
        assert!(
            challenges
                .challenges
                .iter()
                .any(|challenge| challenge.id == ChallengeId::ParagonAwakened)
        );
        assert!(
            challenges
                .challenges
                .iter()
                .any(|challenge| challenge.id == ChallengeId::TalentAdept)
        );
        assert!(
            challenges
                .challenges
                .iter()
                .any(|challenge| challenge.id == ChallengeId::QuartermasterPatron)
        );
        assert!(
            challenges
                .challenges
                .iter()
                .any(|challenge| challenge.id == ChallengeId::Alchemist)
        );
        assert!(
            challenges
                .challenges
                .iter()
                .any(|challenge| challenge.id == ChallengeId::AshenAlchemy)
        );
        assert!(
            challenges
                .challenges
                .iter()
                .any(|challenge| challenge.id == ChallengeId::LastStand)
        );
        assert!(
            challenges
                .challenges
                .iter()
                .any(|challenge| challenge.id == ChallengeId::UntouchedFlask)
        );
        assert!(
            challenges
                .challenges
                .iter()
                .any(|challenge| challenge.id == ChallengeId::EnrageDenied)
        );
        assert!(
            challenges
                .challenges
                .iter()
                .any(|challenge| challenge.id == ChallengeId::SoulSigil)
        );
        assert!(
            challenges
                .challenges
                .iter()
                .any(|challenge| challenge.id == ChallengeId::TrophyCache)
        );
        assert!(
            challenges
                .challenges
                .iter()
                .any(|challenge| challenge.id == ChallengeId::SentinelVeteran)
        );
        assert!(
            challenges
                .challenges
                .iter()
                .any(|challenge| challenge.id == ChallengeId::Boonbound)
        );
        assert!(
            challenges
                .challenges
                .iter()
                .any(|challenge| challenge.id == ChallengeId::BountyHunter)
        );
        assert!(
            challenges
                .challenges
                .iter()
                .any(|challenge| challenge.id == ChallengeId::BestiaryScholar)
        );
        assert!(
            challenges
                .challenges
                .iter()
                .any(|challenge| challenge.id == ChallengeId::Lorekeeper)
        );
        assert!(
            challenges
                .challenges
                .iter()
                .any(|challenge| challenge.id == ChallengeId::CodexAdept)
        );
        assert!(
            challenges
                .challenges
                .iter()
                .any(|challenge| challenge.id == ChallengeId::CodexArchivist)
        );
        assert!(
            challenges
                .challenges
                .iter()
                .any(|challenge| challenge.id == ChallengeId::FlawlessVictory)
        );
        assert!(
            challenges
                .challenges
                .iter()
                .any(|challenge| challenge.id == ChallengeId::EnrageDenied)
        );
        assert!(
            challenges
                .challenges
                .iter()
                .any(|challenge| challenge.id == ChallengeId::RuneWeaver)
        );
        assert!(
            challenges
                .challenges
                .iter()
                .any(|challenge| challenge.id == ChallengeId::ArmoryAdept)
        );
        assert!(
            challenges
                .challenges
                .iter()
                .any(|challenge| challenge.id == ChallengeId::PortalWayfarer)
        );
        assert!(
            challenges
                .challenges
                .iter()
                .any(|challenge| challenge.id == ChallengeId::GlorySeeker)
        );
        assert!(
            challenges
                .challenges
                .iter()
                .any(|challenge| challenge.id == ChallengeId::TormentClear)
        );
        assert!(
            challenges
                .challenges
                .iter()
                .any(|challenge| challenge.id == ChallengeId::PrimalCache)
        );
        assert!(
            challenges
                .challenges
                .iter()
                .any(|challenge| challenge.id == ChallengeId::PrimalInfusion)
        );
        assert!(
            challenges
                .challenges
                .iter()
                .any(|challenge| challenge.id == ChallengeId::ReliquaryConquest)
        );
    }
}
