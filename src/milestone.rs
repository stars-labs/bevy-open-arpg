use crate::{
    AFFIX_CODEX_TARGET, ASHEN_THREAT_SURGE_TARGET, BOSS_BREAK_TARGET, CARNAGE_MASTER_STREAK,
    CHAMPION_PACK_TARGET, CODEX_ADEPT_KILLS, GEM_ADEPT_KINDS, GLORY_SEEKER_GLOBES, GameState,
    LAST_STAND_POTIONS, RunStats, SET_ADEPT_KILLS,
    bounty::BountyProgress,
    chapter::{ChapterProgress, InteractableKind, InteractableUsed},
    companion::{CompanionBond, SENTINEL_VETERAN_RANK},
    dungeon::BREAKER_TARGET_BREAKABLES,
    enemy::{AFFIX_HUNTER_KILLS, CURSED_AMBUSH_KILLS, EnemyKilled},
    feedback::CombatEvent,
    lore::{LORE_ENTRY_COUNT, LoreJournal},
    not_paused,
    obelisk::{BloodObelisk, ObeliskState},
    player::{
        ARMORY_SLOT_COUNT, ChapterBoon, ChapterBoonChoice, Charm, ElixirBelt, EmberParagon,
        Equipment, Inventory, LegendaryCodex, LegendaryPower, PYLON_REAPER_KILLS, Player,
        PotionBelt, SURGE_CHAIN_KILLS, Talents, gear_quality_is_ancient_or_better,
        is_themed_reliquary_resonance,
    },
    rift::{EmberRift, RIFT_SWIFT_SEAL_SECS, RiftState},
};
use bevy::ecs::system::SystemParam;
use bevy::prelude::*;
use serde::{Deserialize, Serialize};

pub const ALL_MILESTONES: [MilestoneId; 63] = [
    MilestoneId::CacheOpened,
    MilestoneId::VaultOpened,
    MilestoneId::Breaker,
    MilestoneId::RelicEmpowered,
    MilestoneId::FortuneBlessed,
    MilestoneId::StormConduit,
    MilestoneId::ShrineResonance,
    MilestoneId::Shrinekeeper,
    MilestoneId::AffixHunter,
    MilestoneId::AffixCodex,
    MilestoneId::SealWarden,
    MilestoneId::AshenThreat,
    MilestoneId::ChampionPack,
    MilestoneId::AshenPylon,
    MilestoneId::HealingWell,
    MilestoneId::CursedShrine,
    MilestoneId::CursedPact,
    MilestoneId::BloodObelisk,
    MilestoneId::EmberRift,
    MilestoneId::Riftbreaker,
    MilestoneId::ReliquarySeal,
    MilestoneId::TreasureFiend,
    MilestoneId::TreasureVault,
    MilestoneId::BountyCompleted,
    MilestoneId::Lorekeeper,
    MilestoneId::EmberParagon,
    MilestoneId::TalentAdept,
    MilestoneId::QuartermasterPatron,
    MilestoneId::Alchemist,
    MilestoneId::AshenAlchemy,
    MilestoneId::LootCurator,
    MilestoneId::LastStand,
    MilestoneId::SalvageCache,
    MilestoneId::SentinelVeteran,
    MilestoneId::Boonbound,
    MilestoneId::TemperedSteel,
    MilestoneId::LegendaryFound,
    MilestoneId::AncientWeapon,
    MilestoneId::AncientAugment,
    MilestoneId::PrimalInfusion,
    MilestoneId::GemResonance,
    MilestoneId::MasterGemcutter,
    MilestoneId::GemAdept,
    MilestoneId::SetResonance,
    MilestoneId::SetAdept,
    MilestoneId::CodexAdept,
    MilestoneId::CodexArchivist,
    MilestoneId::Massacre,
    MilestoneId::CarnageMaster,
    MilestoneId::BattleTrance,
    MilestoneId::SurgeMastery,
    MilestoneId::RuneWeaver,
    MilestoneId::ArmoryAdept,
    MilestoneId::PortalWayfarer,
    MilestoneId::GlorySeeker,
    MilestoneId::PylonReaper,
    MilestoneId::BossBreaker,
    MilestoneId::SoulSigil,
    MilestoneId::TrophyCache,
    MilestoneId::PrimalCache,
    MilestoneId::NemesisSlain,
    MilestoneId::AshenDuel,
    MilestoneId::BossSlain,
];

#[derive(Debug, Clone, Copy, Eq, PartialEq, Serialize, Deserialize)]
pub enum MilestoneId {
    CacheOpened,
    VaultOpened,
    Breaker,
    RelicEmpowered,
    FortuneBlessed,
    StormConduit,
    ShrineResonance,
    Shrinekeeper,
    AffixHunter,
    AffixCodex,
    SealWarden,
    AshenThreat,
    ChampionPack,
    AshenPylon,
    HealingWell,
    CursedShrine,
    CursedPact,
    BloodObelisk,
    EmberRift,
    Riftbreaker,
    ReliquarySeal,
    TreasureFiend,
    TreasureVault,
    BountyCompleted,
    Lorekeeper,
    EmberParagon,
    TalentAdept,
    QuartermasterPatron,
    Alchemist,
    AshenAlchemy,
    LootCurator,
    LastStand,
    SalvageCache,
    SentinelVeteran,
    Boonbound,
    TemperedSteel,
    LegendaryFound,
    AncientWeapon,
    AncientAugment,
    PrimalInfusion,
    GemResonance,
    MasterGemcutter,
    GemAdept,
    SetResonance,
    SetAdept,
    CodexAdept,
    CodexArchivist,
    Massacre,
    CarnageMaster,
    BattleTrance,
    SurgeMastery,
    RuneWeaver,
    ArmoryAdept,
    PortalWayfarer,
    GlorySeeker,
    PylonReaper,
    BossBreaker,
    SoulSigil,
    TrophyCache,
    PrimalCache,
    NemesisSlain,
    AshenDuel,
    BossSlain,
}

impl MilestoneId {
    pub fn title(self) -> &'static str {
        match self {
            Self::CacheOpened => "Reliquary Cache",
            Self::VaultOpened => "Resplendent Vault",
            Self::Breaker => "Breaker",
            Self::RelicEmpowered => "Relic Blade",
            Self::FortuneBlessed => "Fortune Shrine",
            Self::StormConduit => "Storm Conduit",
            Self::ShrineResonance => "Shrine Resonance",
            Self::Shrinekeeper => "Shrinekeeper",
            Self::AffixHunter => "Affix Hunter",
            Self::AffixCodex => "Affix Codex",
            Self::SealWarden => "Seal Warden",
            Self::AshenThreat => "Ashen Threat",
            Self::ChampionPack => "Champion Pack",
            Self::AshenPylon => "Ashen Pylon",
            Self::HealingWell => "Renewal Well",
            Self::CursedShrine => "Blood Price",
            Self::CursedPact => "Cursed Pact",
            Self::BloodObelisk => "Blood Obelisk",
            Self::EmberRift => "Ember Rift",
            Self::Riftbreaker => "Riftbreaker",
            Self::ReliquarySeal => "Reliquary Seal",
            Self::TreasureFiend => "Treasure Fiend",
            Self::TreasureVault => "Treasure Vault",
            Self::BountyCompleted => "Reliquary Bounty",
            Self::Lorekeeper => "Lorekeeper",
            Self::EmberParagon => "Ember Paragon",
            Self::TalentAdept => "Talent Adept",
            Self::QuartermasterPatron => "Quartermaster Patron",
            Self::Alchemist => "Alchemist",
            Self::AshenAlchemy => "Ashen Alchemy",
            Self::LootCurator => "Loot Curator",
            Self::LastStand => "Last Stand",
            Self::SalvageCache => "Salvage Cache",
            Self::SentinelVeteran => "Sentinel Veteran",
            Self::Boonbound => "Boonbound",
            Self::TemperedSteel => "Tempered Steel",
            Self::LegendaryFound => "Legendary Power",
            Self::AncientWeapon => "Ancient Weapon",
            Self::AncientAugment => "Ancient Augment",
            Self::PrimalInfusion => "Primal Infusion",
            Self::GemResonance => "Gem Resonance",
            Self::MasterGemcutter => "Master Gemcutter",
            Self::GemAdept => "Gem Adept",
            Self::SetResonance => "Set Resonance",
            Self::SetAdept => "Set Adept",
            Self::CodexAdept => "Codex Adept",
            Self::CodexArchivist => "Codex Archivist",
            Self::Massacre => "Massacre",
            Self::CarnageMaster => "Carnage Master",
            Self::BattleTrance => "Battle Trance",
            Self::SurgeMastery => "Surge Mastery",
            Self::RuneWeaver => "Rune Weaver",
            Self::ArmoryAdept => "Armory Adept",
            Self::PortalWayfarer => "Portal Wayfarer",
            Self::GlorySeeker => "Glory Seeker",
            Self::PylonReaper => "Pylon Reaper",
            Self::BossBreaker => "Boss Breaker",
            Self::SoulSigil => "Malrec Soul Sigil",
            Self::TrophyCache => "Malrec Trophy Cache",
            Self::PrimalCache => "Primal Ember Cache",
            Self::NemesisSlain => "Nemesis Slain",
            Self::AshenDuel => "Ashen Duel",
            Self::BossSlain => "Keeper Slain",
        }
    }

    pub fn summary(self) -> &'static str {
        match self {
            Self::CacheOpened => "Opened the sealed cache",
            Self::VaultOpened => "Opened the resplendent reliquary vault",
            Self::Breaker => "Smashed every reliquary urn and offering box",
            Self::RelicEmpowered => "Claimed the relic shrine buff",
            Self::FortuneBlessed => "Claimed the gilded fortune blessing",
            Self::StormConduit => "Awakened the storm conduit shrine",
            Self::ShrineResonance => "Triggered shrine resonance from overlapping blessings",
            Self::Shrinekeeper => "Activated every reliquary shrine and pylon",
            Self::AffixHunter => "Slew three multi-affix elite enemies",
            Self::AffixCodex => "Catalogued six distinct elite affixes",
            Self::SealWarden => "Defeated Seal Warden Vhal in the inner sanctum",
            Self::AshenThreat => "Triggered three Ashen Threat surges",
            Self::ChampionPack => "Broke the roaming champion pack",
            Self::AshenPylon => "Overloaded the Ashen Pylon",
            Self::HealingWell => "Restored at the renewal well",
            Self::CursedShrine => "Survived the cursed shrine bargain",
            Self::CursedPact => "Slew every cursed shrine ambusher",
            Self::BloodObelisk => "Completed the obelisk kill rite",
            Self::EmberRift => "Sealed the Ember Rift invasion",
            Self::Riftbreaker => "Sealed the Ember Rift with 30 seconds remaining",
            Self::ReliquarySeal => "Completed the vault, blood obelisk, and Ember Rift",
            Self::TreasureFiend => "Caught the reliquary treasure imp before it escaped",
            Self::TreasureVault => "Opened the treasure imp's reliquary vault",
            Self::BountyCompleted => "Completed an optional bounty",
            Self::Lorekeeper => "Recovered every lore page",
            Self::EmberParagon => "Earned an Ember Paragon rank",
            Self::TalentAdept => "Invested in Wrath, Vigor, and Focus",
            Self::QuartermasterPatron => "Upgraded stash, potion belt, and elixir belt",
            Self::Alchemist => "Used iron, wrath, and haste elixirs",
            Self::AshenAlchemy => "Completed gold, essence, and keystone transmutations",
            Self::LootCurator => "Cycled the loot filter through three tiers",
            Self::LastStand => "Drank three potions while below 30% health",
            Self::SalvageCache => "Earned a quartermaster cache from spare gear",
            Self::SentinelVeteran => "Raised the Reliquary Sentinel to rank 3",
            Self::Boonbound => "Claimed a Reliquary Boon",
            Self::TemperedSteel => "Tempered a weapon at the quartermaster",
            Self::LegendaryFound => "Equipped a legendary power",
            Self::AncientWeapon => "Equipped an ancient weapon",
            Self::AncientAugment => "Applied an ancient augment",
            Self::PrimalInfusion => "Primal-infused an ancient weapon",
            Self::GemResonance => "Equipped a rank 3 resonant socketed gem",
            Self::MasterGemcutter => "Equipped a rank 5 socketed gem",
            Self::GemAdept => "Socketed ruby, emerald, amethyst, and topaz gems",
            Self::SetResonance => "Equipped a themed weapon and charm resonance",
            Self::SetAdept => "Slew enemies while a themed set resonance was active",
            Self::CodexAdept => "Slew enemies with an attuned codex power",
            Self::CodexArchivist => "Unlocked every legendary codex power",
            Self::Massacre => "Chained a five-kill massacre",
            Self::CarnageMaster => "Chained a ten-kill massacre",
            Self::BattleTrance => "Reached full Valor during a massacre run",
            Self::SurgeMastery => "Extended Nephalem Surge through three kills",
            Self::RuneWeaver => "Landed Reap, Hemorrhage, and Frost rune hits",
            Self::ArmoryAdept => "Saved three armory loadouts",
            Self::PortalWayfarer => "Returned to the fight through town portal",
            Self::GlorySeeker => "Collected three health globes for Glory",
            Self::PylonReaper => "Slew three enemies during Ashen Pylon overload",
            Self::BossBreaker => "Broke Malrec's stagger twice",
            Self::SoulSigil => "Recovered a Malrec Soul Sigil",
            Self::TrophyCache => "Opened a Malrec trophy cache",
            Self::PrimalCache => "Claimed a Primal Ember Cache on Torment",
            Self::NemesisSlain => "Defeated an ashbound nemesis",
            Self::AshenDuel => "Defeated Malrec before Ashen Enrage",
            Self::BossSlain => "Defeated Malrec, Keeper of Ash",
        }
    }
}

#[derive(Resource, Debug, Default)]
pub struct Milestones {
    pub unlocked: Vec<MilestoneId>,
    pub latest: String,
}

impl Milestones {
    pub fn unlock(&mut self, milestone: MilestoneId) -> bool {
        if self.unlocked.contains(&milestone) {
            return false;
        }
        self.unlocked.push(milestone);
        self.latest = format!("{}: {}", milestone.title(), milestone.summary());
        true
    }

    pub fn restore(&mut self, unlocked: Vec<MilestoneId>) {
        self.unlocked = unlocked;
        self.latest = self
            .unlocked
            .last()
            .map(|milestone| format!("{}: {}", milestone.title(), milestone.summary()))
            .unwrap_or_default();
    }
}

pub fn milestone_summary(milestones: &Milestones) -> String {
    if milestones.latest.is_empty() {
        return format!(
            "Milestones {}/{}",
            milestones.unlocked.len(),
            ALL_MILESTONES.len()
        );
    }
    format!(
        "Milestones {}/{}: {}",
        milestones.unlocked.len(),
        ALL_MILESTONES.len(),
        milestones.latest
    )
}

pub struct MilestonePlugin;

impl Plugin for MilestonePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Milestones>()
            .add_systems(OnEnter(GameState::InGame), reset_milestones)
            .add_systems(
                Update,
                update_milestones.run_if(in_state(GameState::InGame).and_then(not_paused)),
            );
    }
}

fn reset_milestones(mut milestones: ResMut<Milestones>) {
    *milestones = Milestones::default();
}

#[derive(SystemParam)]
struct MilestoneResources<'w> {
    milestones: ResMut<'w, Milestones>,
    stats: Res<'w, RunStats>,
    progress: Res<'w, ChapterProgress>,
    bounty: Res<'w, BountyProgress>,
    journal: Res<'w, LoreJournal>,
    obelisk: Res<'w, BloodObelisk>,
    rift: Res<'w, EmberRift>,
}

type MilestonePlayerQuery<'w, 's> = Query<
    'w,
    's,
    (
        &'static Equipment,
        &'static Charm,
        &'static LegendaryCodex,
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

fn update_milestones(
    mut interactions: MessageReader<InteractableUsed>,
    mut enemy_killed: MessageReader<EnemyKilled>,
    mut resources: MilestoneResources,
    player: MilestonePlayerQuery,
    mut combat_events: MessageWriter<CombatEvent>,
) {
    for event in interactions.read() {
        match event.kind {
            InteractableKind::Cache => {
                unlock_and_log(
                    &mut resources.milestones,
                    MilestoneId::CacheOpened,
                    &mut combat_events,
                );
            }
            InteractableKind::WeaponShrine => {
                unlock_and_log(
                    &mut resources.milestones,
                    MilestoneId::RelicEmpowered,
                    &mut combat_events,
                );
            }
            InteractableKind::FortuneShrine => {
                unlock_and_log(
                    &mut resources.milestones,
                    MilestoneId::FortuneBlessed,
                    &mut combat_events,
                );
            }
            InteractableKind::StormShrine => {
                unlock_and_log(
                    &mut resources.milestones,
                    MilestoneId::StormConduit,
                    &mut combat_events,
                );
            }
            InteractableKind::AshenPylon => {
                unlock_and_log(
                    &mut resources.milestones,
                    MilestoneId::AshenPylon,
                    &mut combat_events,
                );
            }
            InteractableKind::HealingWell => {
                unlock_and_log(
                    &mut resources.milestones,
                    MilestoneId::HealingWell,
                    &mut combat_events,
                );
            }
            InteractableKind::CursedShrine => {
                unlock_and_log(
                    &mut resources.milestones,
                    MilestoneId::CursedShrine,
                    &mut combat_events,
                );
            }
            InteractableKind::ReliquaryVault => {
                unlock_and_log(
                    &mut resources.milestones,
                    MilestoneId::VaultOpened,
                    &mut combat_events,
                );
            }
            InteractableKind::Altar
            | InteractableKind::Merchant
            | InteractableKind::LorePage
            | InteractableKind::BloodObelisk
            | InteractableKind::EmberRift => {}
        }
    }

    for event in enemy_killed.read() {
        if event.enemy_id == "treasure_imp" {
            unlock_and_log(
                &mut resources.milestones,
                MilestoneId::TreasureFiend,
                &mut combat_events,
            );
        }
        if resources.stats.treasure_vaults_opened > 0 {
            unlock_and_log(
                &mut resources.milestones,
                MilestoneId::TreasureVault,
                &mut combat_events,
            );
        }
        if event.enemy_id == "nemesis" {
            unlock_and_log(
                &mut resources.milestones,
                MilestoneId::NemesisSlain,
                &mut combat_events,
            );
        }
        if event.enemy_id == "seal_warden" {
            unlock_and_log(
                &mut resources.milestones,
                MilestoneId::SealWarden,
                &mut combat_events,
            );
        }
    }

    if resources.journal.entries.len() >= LORE_ENTRY_COUNT {
        unlock_and_log(
            &mut resources.milestones,
            MilestoneId::Lorekeeper,
            &mut combat_events,
        );
    }
    if codex_adept_complete(&resources.stats) {
        unlock_and_log(
            &mut resources.milestones,
            MilestoneId::CodexAdept,
            &mut combat_events,
        );
    }
    if set_adept_complete(&resources.stats) {
        unlock_and_log(
            &mut resources.milestones,
            MilestoneId::SetAdept,
            &mut combat_events,
        );
    }
    if gem_adept_complete(&resources.stats) {
        unlock_and_log(
            &mut resources.milestones,
            MilestoneId::GemAdept,
            &mut combat_events,
        );
    }
    if resources.stats.best_massacre_streak >= 5 {
        unlock_and_log(
            &mut resources.milestones,
            MilestoneId::Massacre,
            &mut combat_events,
        );
    }
    if carnage_master_complete(&resources.stats) {
        unlock_and_log(
            &mut resources.milestones,
            MilestoneId::CarnageMaster,
            &mut combat_events,
        );
    }
    if battle_trance_complete(&resources.stats) {
        unlock_and_log(
            &mut resources.milestones,
            MilestoneId::BattleTrance,
            &mut combat_events,
        );
    }
    if surge_mastery_complete(&resources.stats) {
        unlock_and_log(
            &mut resources.milestones,
            MilestoneId::SurgeMastery,
            &mut combat_events,
        );
    }
    if rune_weaver_complete(&resources.stats) {
        unlock_and_log(
            &mut resources.milestones,
            MilestoneId::RuneWeaver,
            &mut combat_events,
        );
    }
    if armory_adept_complete(&resources.stats) {
        unlock_and_log(
            &mut resources.milestones,
            MilestoneId::ArmoryAdept,
            &mut combat_events,
        );
    }
    if portal_wayfarer_complete(&resources.stats) {
        unlock_and_log(
            &mut resources.milestones,
            MilestoneId::PortalWayfarer,
            &mut combat_events,
        );
    }
    if glory_seeker_complete(&resources.stats) {
        unlock_and_log(
            &mut resources.milestones,
            MilestoneId::GlorySeeker,
            &mut combat_events,
        );
    }
    if pylon_reaper_complete(&resources.stats) {
        unlock_and_log(
            &mut resources.milestones,
            MilestoneId::PylonReaper,
            &mut combat_events,
        );
    }
    if boss_breaker_complete(&resources.stats) {
        unlock_and_log(
            &mut resources.milestones,
            MilestoneId::BossBreaker,
            &mut combat_events,
        );
    }
    if soul_sigil_complete(&resources.stats) {
        unlock_and_log(
            &mut resources.milestones,
            MilestoneId::SoulSigil,
            &mut combat_events,
        );
    }
    if trophy_cache_complete(&resources.stats) {
        unlock_and_log(
            &mut resources.milestones,
            MilestoneId::TrophyCache,
            &mut combat_events,
        );
    }
    if primal_cache_complete(&resources.stats) {
        unlock_and_log(
            &mut resources.milestones,
            MilestoneId::PrimalCache,
            &mut combat_events,
        );
    }
    if breaker_complete(&resources.stats) {
        unlock_and_log(
            &mut resources.milestones,
            MilestoneId::Breaker,
            &mut combat_events,
        );
    }
    if shrine_resonance_complete(&resources.stats) {
        unlock_and_log(
            &mut resources.milestones,
            MilestoneId::ShrineResonance,
            &mut combat_events,
        );
    }
    if shrinekeeper_complete(&resources.milestones) {
        unlock_and_log(
            &mut resources.milestones,
            MilestoneId::Shrinekeeper,
            &mut combat_events,
        );
    }
    if affix_hunter_complete(&resources.stats) {
        unlock_and_log(
            &mut resources.milestones,
            MilestoneId::AffixHunter,
            &mut combat_events,
        );
    }
    if affix_codex_complete(&resources.stats) {
        unlock_and_log(
            &mut resources.milestones,
            MilestoneId::AffixCodex,
            &mut combat_events,
        );
    }
    if seal_warden_complete(&resources.stats) {
        unlock_and_log(
            &mut resources.milestones,
            MilestoneId::SealWarden,
            &mut combat_events,
        );
    }
    if ashen_threat_complete(&resources.stats) {
        unlock_and_log(
            &mut resources.milestones,
            MilestoneId::AshenThreat,
            &mut combat_events,
        );
    }
    if champion_pack_complete(&resources.stats) {
        unlock_and_log(
            &mut resources.milestones,
            MilestoneId::ChampionPack,
            &mut combat_events,
        );
    }
    if cursed_pact_complete(&resources.stats) {
        unlock_and_log(
            &mut resources.milestones,
            MilestoneId::CursedPact,
            &mut combat_events,
        );
    }
    if resources.stats.ancient_augments > 0 {
        unlock_and_log(
            &mut resources.milestones,
            MilestoneId::AncientAugment,
            &mut combat_events,
        );
    }
    if primal_infusion_complete(&resources.stats) {
        unlock_and_log(
            &mut resources.milestones,
            MilestoneId::PrimalInfusion,
            &mut combat_events,
        );
    }
    if alchemist_complete(&resources.stats) {
        unlock_and_log(
            &mut resources.milestones,
            MilestoneId::Alchemist,
            &mut combat_events,
        );
    }
    if ashen_alchemy_complete(&resources.stats) {
        unlock_and_log(
            &mut resources.milestones,
            MilestoneId::AshenAlchemy,
            &mut combat_events,
        );
    }
    if loot_curator_complete(&resources.stats) {
        unlock_and_log(
            &mut resources.milestones,
            MilestoneId::LootCurator,
            &mut combat_events,
        );
    }
    if last_stand_complete(&resources.stats) {
        unlock_and_log(
            &mut resources.milestones,
            MilestoneId::LastStand,
            &mut combat_events,
        );
    }
    if salvage_cache_complete(&resources.stats) {
        unlock_and_log(
            &mut resources.milestones,
            MilestoneId::SalvageCache,
            &mut combat_events,
        );
    }
    if resources.progress.boss_slain {
        if ashen_duel_complete(&resources.progress, &resources.stats) {
            unlock_and_log(
                &mut resources.milestones,
                MilestoneId::AshenDuel,
                &mut combat_events,
            );
        }
        unlock_and_log(
            &mut resources.milestones,
            MilestoneId::BossSlain,
            &mut combat_events,
        );
    }
    if resources.bounty.reward_claimed {
        unlock_and_log(
            &mut resources.milestones,
            MilestoneId::BountyCompleted,
            &mut combat_events,
        );
    }
    if resources.obelisk.state == ObeliskState::Completed {
        unlock_and_log(
            &mut resources.milestones,
            MilestoneId::BloodObelisk,
            &mut combat_events,
        );
    }
    if resources.rift.state == RiftState::Completed {
        unlock_and_log(
            &mut resources.milestones,
            MilestoneId::EmberRift,
            &mut combat_events,
        );
    }
    if riftbreaker_complete(&resources.rift) {
        unlock_and_log(
            &mut resources.milestones,
            MilestoneId::Riftbreaker,
            &mut combat_events,
        );
    }
    if reliquary_seal_complete(&resources.milestones) {
        unlock_and_log(
            &mut resources.milestones,
            MilestoneId::ReliquarySeal,
            &mut combat_events,
        );
    }

    let Ok((
        equipment,
        charm,
        codex,
        paragon,
        inventory,
        potions,
        elixirs,
        companion,
        boon,
        talents,
    )) = player.single()
    else {
        return;
    };
    if paragon.rank > 0 {
        unlock_and_log(
            &mut resources.milestones,
            MilestoneId::EmberParagon,
            &mut combat_events,
        );
    }
    if talent_adept_complete(talents) {
        unlock_and_log(
            &mut resources.milestones,
            MilestoneId::TalentAdept,
            &mut combat_events,
        );
    }
    if quartermaster_patron_complete(inventory, potions, elixirs) {
        unlock_and_log(
            &mut resources.milestones,
            MilestoneId::QuartermasterPatron,
            &mut combat_events,
        );
    }
    if sentinel_veteran_complete(companion) {
        unlock_and_log(
            &mut resources.milestones,
            MilestoneId::SentinelVeteran,
            &mut combat_events,
        );
    }
    if boonbound_complete(boon) {
        unlock_and_log(
            &mut resources.milestones,
            MilestoneId::Boonbound,
            &mut combat_events,
        );
    }
    if equipment.temper_level > 0 {
        unlock_and_log(
            &mut resources.milestones,
            MilestoneId::TemperedSteel,
            &mut combat_events,
        );
    }
    if equipment.legendary_power != LegendaryPower::None {
        unlock_and_log(
            &mut resources.milestones,
            MilestoneId::LegendaryFound,
            &mut combat_events,
        );
    }
    if gear_quality_is_ancient_or_better(&equipment.quality) {
        unlock_and_log(
            &mut resources.milestones,
            MilestoneId::AncientWeapon,
            &mut combat_events,
        );
    }
    if gem_resonance_complete(equipment) {
        unlock_and_log(
            &mut resources.milestones,
            MilestoneId::GemResonance,
            &mut combat_events,
        );
    }
    if master_gemcutter_complete(equipment) {
        unlock_and_log(
            &mut resources.milestones,
            MilestoneId::MasterGemcutter,
            &mut combat_events,
        );
    }
    if is_themed_reliquary_resonance(equipment, charm) {
        unlock_and_log(
            &mut resources.milestones,
            MilestoneId::SetResonance,
            &mut combat_events,
        );
    }
    if codex.is_complete() {
        unlock_and_log(
            &mut resources.milestones,
            MilestoneId::CodexArchivist,
            &mut combat_events,
        );
    }
}

fn quartermaster_patron_complete(
    inventory: &Inventory,
    potions: &PotionBelt,
    elixirs: &ElixirBelt,
) -> bool {
    inventory.capacity > 12 && potions.max > 5 && elixirs.max > 3
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

fn salvage_cache_complete(stats: &RunStats) -> bool {
    stats.salvage_caches > 0
}

fn codex_adept_complete(stats: &RunStats) -> bool {
    stats.codex_attuned_kills >= CODEX_ADEPT_KILLS
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

fn talent_adept_complete(talents: &Talents) -> bool {
    talents.wrath > 0 && talents.vigor > 0 && talents.focus > 0
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

fn champion_pack_complete(stats: &RunStats) -> bool {
    stats.champion_pack_reward_claimed && stats.champion_pack_kills >= CHAMPION_PACK_TARGET
}

fn cursed_pact_complete(stats: &RunStats) -> bool {
    stats.cursed_ambush_kills >= CURSED_AMBUSH_KILLS
}

fn ashen_duel_complete(progress: &ChapterProgress, stats: &RunStats) -> bool {
    progress.boss_slain && !stats.boss_enraged
}

fn riftbreaker_complete(rift: &EmberRift) -> bool {
    rift.best_remaining_secs >= RIFT_SWIFT_SEAL_SECS
}

fn reliquary_seal_complete(milestones: &Milestones) -> bool {
    milestones.unlocked.contains(&MilestoneId::VaultOpened)
        && milestones.unlocked.contains(&MilestoneId::BloodObelisk)
        && milestones.unlocked.contains(&MilestoneId::EmberRift)
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

fn unlock_and_log(
    milestones: &mut Milestones,
    milestone: MilestoneId,
    combat_events: &mut MessageWriter<CombatEvent>,
) {
    if milestones.unlock(milestone) {
        combat_events.write(CombatEvent {
            text: format!("Milestone: {}", milestone.title()),
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::player::{GemKind, SocketedGem};

    #[test]
    fn milestones_unlock_once_and_track_latest() {
        let mut milestones = Milestones::default();

        assert!(milestones.unlock(MilestoneId::CacheOpened));
        assert!(!milestones.unlock(MilestoneId::CacheOpened));
        assert_eq!(milestones.unlocked, vec![MilestoneId::CacheOpened]);
        assert!(milestones.latest.contains("Reliquary Cache"));
    }

    #[test]
    fn vault_milestone_has_distinct_title_and_summary() {
        let mut milestones = Milestones::default();

        assert!(milestones.unlock(MilestoneId::VaultOpened));

        assert!(milestones.latest.contains("Resplendent Vault"));
        assert!(milestone_summary(&milestones).contains("resplendent reliquary vault"));
        assert!(ALL_MILESTONES.contains(&MilestoneId::VaultOpened));
    }

    #[test]
    fn treasure_fiend_milestone_has_distinct_title_and_summary() {
        let mut milestones = Milestones::default();

        assert!(milestones.unlock(MilestoneId::TreasureFiend));

        assert!(milestones.latest.contains("Treasure Fiend"));
        assert!(milestone_summary(&milestones).contains("treasure imp"));
        assert!(ALL_MILESTONES.contains(&MilestoneId::TreasureFiend));
    }

    #[test]
    fn healing_well_milestone_has_distinct_title_and_summary() {
        let mut milestones = Milestones::default();

        assert!(milestones.unlock(MilestoneId::HealingWell));

        assert!(milestones.latest.contains("Renewal Well"));
        assert!(milestone_summary(&milestones).contains("renewal well"));
        assert!(ALL_MILESTONES.contains(&MilestoneId::HealingWell));
    }

    #[test]
    fn ashen_pylon_milestone_has_distinct_title_and_summary() {
        let mut milestones = Milestones::default();

        assert!(milestones.unlock(MilestoneId::AshenPylon));

        assert!(milestones.latest.contains("Ashen Pylon"));
        assert!(milestone_summary(&milestones).contains("Overloaded"));
        assert!(ALL_MILESTONES.contains(&MilestoneId::AshenPylon));
    }

    #[test]
    fn codex_archivist_milestone_has_distinct_title_and_summary() {
        let mut milestones = Milestones::default();

        assert!(milestones.unlock(MilestoneId::CodexArchivist));

        assert!(milestones.latest.contains("Codex Archivist"));
        assert!(milestone_summary(&milestones).contains("legendary codex power"));
        assert!(ALL_MILESTONES.contains(&MilestoneId::CodexArchivist));
    }

    #[test]
    fn codex_adept_milestone_has_distinct_title_and_summary() {
        let mut milestones = Milestones::default();

        assert!(milestones.unlock(MilestoneId::CodexAdept));

        assert!(milestones.latest.contains("Codex Adept"));
        assert!(milestone_summary(&milestones).contains("attuned codex power"));
        assert!(ALL_MILESTONES.contains(&MilestoneId::CodexAdept));
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
    fn set_resonance_milestone_has_distinct_title_and_summary() {
        let mut milestones = Milestones::default();

        assert!(milestones.unlock(MilestoneId::SetResonance));

        assert!(milestones.latest.contains("Set Resonance"));
        assert!(milestone_summary(&milestones).contains("themed weapon and charm"));
        assert!(ALL_MILESTONES.contains(&MilestoneId::SetResonance));
    }

    #[test]
    fn set_adept_milestone_has_distinct_title_and_summary() {
        let mut milestones = Milestones::default();

        assert!(milestones.unlock(MilestoneId::SetAdept));

        assert!(milestones.latest.contains("Set Adept"));
        assert!(milestone_summary(&milestones).contains("themed set resonance"));
        assert!(ALL_MILESTONES.contains(&MilestoneId::SetAdept));
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
    fn ancient_weapon_milestone_has_distinct_title_and_summary() {
        let mut milestones = Milestones::default();

        assert!(milestones.unlock(MilestoneId::AncientWeapon));

        assert!(milestones.latest.contains("Ancient Weapon"));
        assert!(milestone_summary(&milestones).contains("ancient weapon"));
        assert!(ALL_MILESTONES.contains(&MilestoneId::AncientWeapon));
    }

    #[test]
    fn ancient_augment_milestone_has_distinct_title_and_summary() {
        let mut milestones = Milestones::default();

        assert!(milestones.unlock(MilestoneId::AncientAugment));

        assert!(milestones.latest.contains("Ancient Augment"));
        assert!(milestone_summary(&milestones).contains("ancient augment"));
        assert!(ALL_MILESTONES.contains(&MilestoneId::AncientAugment));
    }

    #[test]
    fn primal_infusion_milestone_has_distinct_title_and_summary() {
        let mut milestones = Milestones::default();

        assert!(milestones.unlock(MilestoneId::PrimalInfusion));

        assert!(milestones.latest.contains("Primal Infusion"));
        assert!(milestone_summary(&milestones).contains("Primal-infused"));
        assert!(ALL_MILESTONES.contains(&MilestoneId::PrimalInfusion));
    }

    #[test]
    fn master_gemcutter_milestone_has_distinct_title_and_summary() {
        let mut milestones = Milestones::default();

        assert!(milestones.unlock(MilestoneId::MasterGemcutter));

        assert!(milestones.latest.contains("Master Gemcutter"));
        assert!(milestone_summary(&milestones).contains("rank 5 socketed gem"));
        assert!(ALL_MILESTONES.contains(&MilestoneId::MasterGemcutter));
    }

    #[test]
    fn gem_resonance_milestone_has_distinct_title_and_summary() {
        let mut milestones = Milestones::default();

        assert!(milestones.unlock(MilestoneId::GemResonance));

        assert!(milestones.latest.contains("Gem Resonance"));
        assert!(milestone_summary(&milestones).contains("rank 3 resonant"));
        assert!(ALL_MILESTONES.contains(&MilestoneId::GemResonance));
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
                kind: GemKind::Topaz,
                rank: 2,
            }),
        };

        assert!(!gem_resonance_complete(&equipment));

        equipment.socketed_gem = Some(SocketedGem {
            kind: GemKind::Topaz,
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
                kind: GemKind::Topaz,
                rank: 4,
            }),
        };

        assert!(!master_gemcutter_complete(&equipment));

        equipment.socketed_gem = Some(SocketedGem {
            kind: GemKind::Topaz,
            rank: 5,
        });
        assert!(master_gemcutter_complete(&equipment));
    }

    #[test]
    fn gem_adept_milestone_has_distinct_title_and_summary() {
        let mut milestones = Milestones::default();

        assert!(milestones.unlock(MilestoneId::GemAdept));

        assert!(milestones.latest.contains("Gem Adept"));
        assert!(milestone_summary(&milestones).contains("ruby, emerald, amethyst, and topaz"));
        assert!(ALL_MILESTONES.contains(&MilestoneId::GemAdept));
    }

    #[test]
    fn gem_adept_requires_all_four_socketed_gem_kinds() {
        let mut stats = RunStats {
            ruby_socketed: true,
            emerald_socketed: true,
            amethyst_socketed: true,
            ..default()
        };

        assert_eq!(gem_adept_count(&stats), 3);
        assert!(!gem_adept_complete(&stats));

        stats.topaz_socketed = true;
        assert_eq!(gem_adept_count(&stats), GEM_ADEPT_KINDS);
        assert!(gem_adept_complete(&stats));
    }

    #[test]
    fn battle_trance_milestone_has_distinct_title_and_summary() {
        let mut milestones = Milestones::default();

        assert!(milestones.unlock(MilestoneId::BattleTrance));

        assert!(milestones.latest.contains("Battle Trance"));
        assert!(milestone_summary(&milestones).contains("full Valor"));
        assert!(ALL_MILESTONES.contains(&MilestoneId::BattleTrance));
    }

    #[test]
    fn treasure_vault_milestone_has_distinct_title_and_summary() {
        let mut milestones = Milestones::default();

        assert!(milestones.unlock(MilestoneId::TreasureVault));

        assert!(milestones.latest.contains("Treasure Vault"));
        assert!(milestone_summary(&milestones).contains("reliquary vault"));
        assert!(ALL_MILESTONES.contains(&MilestoneId::TreasureVault));
    }

    #[test]
    fn carnage_master_milestone_has_distinct_title_and_summary() {
        let mut milestones = Milestones::default();

        assert!(milestones.unlock(MilestoneId::CarnageMaster));

        assert!(milestones.latest.contains("Carnage Master"));
        assert!(milestone_summary(&milestones).contains("ten-kill massacre"));
        assert!(ALL_MILESTONES.contains(&MilestoneId::CarnageMaster));
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
    fn battle_trance_requires_massacre_and_full_valor() {
        let mut stats = RunStats {
            best_massacre_streak: 4,
            best_valor_stacks: 5,
            ..default()
        };

        assert!(!battle_trance_complete(&stats));

        stats.best_massacre_streak = 5;
        assert!(battle_trance_complete(&stats));
    }

    #[test]
    fn surge_mastery_milestone_has_distinct_title_and_summary() {
        let mut milestones = Milestones::default();

        assert!(milestones.unlock(MilestoneId::SurgeMastery));

        assert!(milestones.latest.contains("Surge Mastery"));
        assert!(milestone_summary(&milestones).contains("Nephalem Surge"));
        assert!(ALL_MILESTONES.contains(&MilestoneId::SurgeMastery));
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
    fn rune_weaver_milestone_has_distinct_title_and_summary() {
        let mut milestones = Milestones::default();

        assert!(milestones.unlock(MilestoneId::RuneWeaver));

        assert!(milestones.latest.contains("Rune Weaver"));
        assert!(milestone_summary(&milestones).contains("Reap"));
        assert!(ALL_MILESTONES.contains(&MilestoneId::RuneWeaver));
    }

    #[test]
    fn rune_weaver_requires_all_three_alternate_rune_hits() {
        let mut stats = RunStats {
            reap_dash_hits: 1,
            frost_nova_hits: 1,
            ..default()
        };

        assert!(!rune_weaver_complete(&stats));

        stats.hemorrhage_rupture_hits = 1;
        assert!(rune_weaver_complete(&stats));
    }

    #[test]
    fn armory_adept_milestone_has_distinct_title_and_summary() {
        let mut milestones = Milestones::default();

        assert!(milestones.unlock(MilestoneId::ArmoryAdept));

        assert!(milestones.latest.contains("Armory Adept"));
        assert!(milestone_summary(&milestones).contains("three armory loadouts"));
        assert!(ALL_MILESTONES.contains(&MilestoneId::ArmoryAdept));
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
    fn portal_wayfarer_milestone_has_distinct_title_and_summary() {
        let mut milestones = Milestones::default();

        assert!(milestones.unlock(MilestoneId::PortalWayfarer));

        assert!(milestones.latest.contains("Portal Wayfarer"));
        assert!(milestone_summary(&milestones).contains("town portal"));
        assert!(ALL_MILESTONES.contains(&MilestoneId::PortalWayfarer));
    }

    #[test]
    fn portal_wayfarer_requires_a_town_portal_return() {
        let mut stats = RunStats::default();

        assert!(!portal_wayfarer_complete(&stats));

        stats.town_portal_returns = 1;
        assert!(portal_wayfarer_complete(&stats));
    }

    #[test]
    fn glory_seeker_milestone_has_distinct_title_and_summary() {
        let mut milestones = Milestones::default();

        assert!(milestones.unlock(MilestoneId::GlorySeeker));

        assert!(milestones.latest.contains("Glory Seeker"));
        assert!(milestone_summary(&milestones).contains("health globes"));
        assert!(ALL_MILESTONES.contains(&MilestoneId::GlorySeeker));
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
    fn pylon_reaper_milestone_has_distinct_title_and_summary() {
        let mut milestones = Milestones::default();

        assert!(milestones.unlock(MilestoneId::PylonReaper));

        assert!(milestones.latest.contains("Pylon Reaper"));
        assert!(milestone_summary(&milestones).contains("Ashen Pylon"));
        assert!(ALL_MILESTONES.contains(&MilestoneId::PylonReaper));
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
    fn boss_breaker_milestone_has_distinct_title_and_summary() {
        let mut milestones = Milestones::default();

        assert!(milestones.unlock(MilestoneId::BossBreaker));

        assert!(milestones.latest.contains("Boss Breaker"));
        assert!(milestone_summary(&milestones).contains("Malrec"));
        assert!(ALL_MILESTONES.contains(&MilestoneId::BossBreaker));
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
    fn soul_sigil_milestone_has_distinct_title_and_summary() {
        let mut milestones = Milestones::default();

        assert!(milestones.unlock(MilestoneId::SoulSigil));

        assert!(milestones.latest.contains("Malrec Soul Sigil"));
        assert!(milestone_summary(&milestones).contains("Soul Sigil"));
        assert!(ALL_MILESTONES.contains(&MilestoneId::SoulSigil));
    }

    #[test]
    fn soul_sigil_requires_boss_trophy_material() {
        let mut stats = RunStats::default();

        assert!(!soul_sigil_complete(&stats));

        stats.malrec_soul_sigils = 1;
        assert!(soul_sigil_complete(&stats));
    }

    #[test]
    fn trophy_cache_milestone_has_distinct_title_and_summary() {
        let mut milestones = Milestones::default();

        assert!(milestones.unlock(MilestoneId::TrophyCache));

        assert!(milestones.latest.contains("Malrec Trophy Cache"));
        assert!(milestone_summary(&milestones).contains("Trophy Cache"));
        assert!(ALL_MILESTONES.contains(&MilestoneId::TrophyCache));
    }

    #[test]
    fn trophy_cache_requires_opened_soul_sigil_cache() {
        let mut stats = RunStats::default();

        assert!(!trophy_cache_complete(&stats));

        stats.soul_sigil_caches = 1;
        assert!(trophy_cache_complete(&stats));
    }

    #[test]
    fn primal_cache_milestone_has_distinct_title_and_summary() {
        let mut milestones = Milestones::default();

        assert!(milestones.unlock(MilestoneId::PrimalCache));

        assert!(milestones.latest.contains("Primal Ember Cache"));
        assert!(milestone_summary(&milestones).contains("Primal Ember Cache"));
        assert!(ALL_MILESTONES.contains(&MilestoneId::PrimalCache));
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
    fn breaker_milestone_has_distinct_title_and_summary() {
        let mut milestones = Milestones::default();

        assert!(milestones.unlock(MilestoneId::Breaker));

        assert!(milestones.latest.contains("Breaker"));
        assert!(milestone_summary(&milestones).contains("reliquary urn"));
        assert!(ALL_MILESTONES.contains(&MilestoneId::Breaker));
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
    fn shrine_resonance_milestone_has_distinct_title_and_summary() {
        let mut milestones = Milestones::default();

        assert!(milestones.unlock(MilestoneId::ShrineResonance));

        assert!(milestones.latest.contains("Shrine Resonance"));
        assert!(milestone_summary(&milestones).contains("overlapping blessings"));
        assert!(ALL_MILESTONES.contains(&MilestoneId::ShrineResonance));
    }

    #[test]
    fn shrine_resonance_requires_triggered_resonance() {
        let mut stats = RunStats::default();

        assert!(!shrine_resonance_complete(&stats));

        stats.shrine_resonance_triggered = true;
        assert!(shrine_resonance_complete(&stats));
    }

    #[test]
    fn shrinekeeper_milestone_has_distinct_title_and_summary() {
        let mut milestones = Milestones::default();

        assert!(milestones.unlock(MilestoneId::Shrinekeeper));

        assert!(milestones.latest.contains("Shrinekeeper"));
        assert!(milestone_summary(&milestones).contains("every reliquary shrine"));
        assert!(ALL_MILESTONES.contains(&MilestoneId::Shrinekeeper));
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
    fn affix_hunter_milestone_has_distinct_title_and_summary() {
        let mut milestones = Milestones::default();

        assert!(milestones.unlock(MilestoneId::AffixHunter));

        assert!(milestones.latest.contains("Affix Hunter"));
        assert!(milestone_summary(&milestones).contains("multi-affix elite"));
        assert!(ALL_MILESTONES.contains(&MilestoneId::AffixHunter));
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
    fn affix_codex_milestone_has_distinct_title_and_summary() {
        let mut milestones = Milestones::default();

        assert!(milestones.unlock(MilestoneId::AffixCodex));

        assert!(milestones.latest.contains("Affix Codex"));
        assert!(milestone_summary(&milestones).contains("six distinct elite affixes"));
        assert!(ALL_MILESTONES.contains(&MilestoneId::AffixCodex));
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
    fn seal_warden_milestone_has_distinct_title_and_summary() {
        let mut milestones = Milestones::default();

        assert!(milestones.unlock(MilestoneId::SealWarden));

        assert!(milestones.latest.contains("Seal Warden"));
        assert!(milestone_summary(&milestones).contains("inner sanctum"));
        assert!(ALL_MILESTONES.contains(&MilestoneId::SealWarden));
    }

    #[test]
    fn seal_warden_requires_named_sanctum_elite() {
        let mut stats = RunStats::default();

        assert!(!seal_warden_complete(&stats));

        stats.seal_warden_slain = true;
        assert!(seal_warden_complete(&stats));
    }

    #[test]
    fn ashen_threat_milestone_has_distinct_title_and_summary() {
        let mut milestones = Milestones::default();

        assert!(milestones.unlock(MilestoneId::AshenThreat));

        assert!(milestones.latest.contains("Ashen Threat"));
        assert!(milestone_summary(&milestones).contains("three Ashen Threat surges"));
        assert!(ALL_MILESTONES.contains(&MilestoneId::AshenThreat));
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
    fn champion_pack_milestone_has_distinct_title_and_summary() {
        let mut milestones = Milestones::default();

        assert!(milestones.unlock(MilestoneId::ChampionPack));

        assert!(milestones.latest.contains("Champion Pack"));
        assert!(milestone_summary(&milestones).contains("roaming champion pack"));
        assert!(ALL_MILESTONES.contains(&MilestoneId::ChampionPack));
    }

    #[test]
    fn champion_pack_requires_pack_reward_claimed() {
        let mut stats = RunStats {
            champion_pack_kills: CHAMPION_PACK_TARGET,
            ..default()
        };

        assert!(!champion_pack_complete(&stats));

        stats.champion_pack_reward_claimed = true;
        assert!(champion_pack_complete(&stats));
    }

    #[test]
    fn cursed_pact_milestone_has_distinct_title_and_summary() {
        let mut milestones = Milestones::default();

        assert!(milestones.unlock(MilestoneId::CursedPact));

        assert!(milestones.latest.contains("Cursed Pact"));
        assert!(milestone_summary(&milestones).contains("cursed shrine ambusher"));
        assert!(ALL_MILESTONES.contains(&MilestoneId::CursedPact));
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
    fn ember_paragon_milestone_has_distinct_title_and_summary() {
        let mut milestones = Milestones::default();

        assert!(milestones.unlock(MilestoneId::EmberParagon));

        assert!(milestones.latest.contains("Ember Paragon"));
        assert!(milestone_summary(&milestones).contains("Ember Paragon rank"));
        assert!(ALL_MILESTONES.contains(&MilestoneId::EmberParagon));
    }

    #[test]
    fn talent_adept_milestone_has_distinct_title_and_summary() {
        let mut milestones = Milestones::default();

        assert!(milestones.unlock(MilestoneId::TalentAdept));

        assert!(milestones.latest.contains("Talent Adept"));
        assert!(milestone_summary(&milestones).contains("Wrath"));
        assert!(ALL_MILESTONES.contains(&MilestoneId::TalentAdept));
    }

    #[test]
    fn talent_adept_requires_wrath_vigor_and_focus() {
        let mut talents = Talents {
            wrath: 1,
            vigor: 0,
            focus: 1,
            points: 0,
        };

        assert!(!talent_adept_complete(&talents));

        talents.vigor = 1;
        assert!(talent_adept_complete(&talents));
    }

    #[test]
    fn quartermaster_patron_milestone_has_distinct_title_and_summary() {
        let mut milestones = Milestones::default();

        assert!(milestones.unlock(MilestoneId::QuartermasterPatron));

        assert!(milestones.latest.contains("Quartermaster Patron"));
        assert!(milestone_summary(&milestones).contains("potion belt"));
        assert!(ALL_MILESTONES.contains(&MilestoneId::QuartermasterPatron));
    }

    #[test]
    fn alchemist_milestone_has_distinct_title_and_summary() {
        let mut milestones = Milestones::default();

        assert!(milestones.unlock(MilestoneId::Alchemist));

        assert!(milestones.latest.contains("Alchemist"));
        assert!(milestone_summary(&milestones).contains("iron, wrath, and haste elixirs"));
        assert!(ALL_MILESTONES.contains(&MilestoneId::Alchemist));
    }

    #[test]
    fn ashen_alchemy_milestone_has_distinct_title_and_summary() {
        let mut milestones = Milestones::default();

        assert!(milestones.unlock(MilestoneId::AshenAlchemy));

        assert!(milestones.latest.contains("Ashen Alchemy"));
        assert!(milestone_summary(&milestones).contains("keystone transmutations"));
        assert!(ALL_MILESTONES.contains(&MilestoneId::AshenAlchemy));
    }

    #[test]
    fn ashen_alchemy_requires_all_three_recipe_types() {
        let mut stats = RunStats {
            alchemy_gold_transmutes: 1,
            alchemy_keystone_transmutes: 1,
            ..RunStats::default()
        };

        assert!(!ashen_alchemy_complete(&stats));

        stats.alchemy_essence_transmutes = 1;
        assert!(ashen_alchemy_complete(&stats));
    }

    #[test]
    fn loot_curator_milestone_has_distinct_title_and_summary() {
        let mut milestones = Milestones::default();

        assert!(milestones.unlock(MilestoneId::LootCurator));

        assert!(milestones.latest.contains("Loot Curator"));
        assert!(milestone_summary(&milestones).contains("loot filter"));
        assert!(ALL_MILESTONES.contains(&MilestoneId::LootCurator));
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
    fn last_stand_milestone_has_distinct_title_and_summary() {
        let mut milestones = Milestones::default();

        assert!(milestones.unlock(MilestoneId::LastStand));

        assert!(milestones.latest.contains("Last Stand"));
        assert!(milestone_summary(&milestones).contains("30% health"));
        assert!(ALL_MILESTONES.contains(&MilestoneId::LastStand));
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
    fn salvage_cache_milestone_has_distinct_title_and_summary() {
        let mut milestones = Milestones::default();

        assert!(milestones.unlock(MilestoneId::SalvageCache));

        assert!(milestones.latest.contains("Salvage Cache"));
        assert!(milestone_summary(&milestones).contains("quartermaster cache"));
        assert!(ALL_MILESTONES.contains(&MilestoneId::SalvageCache));
    }

    #[test]
    fn salvage_cache_requires_a_claimed_cache() {
        let mut stats = RunStats::default();

        assert!(!salvage_cache_complete(&stats));

        stats.salvage_caches = 1;
        assert!(salvage_cache_complete(&stats));
    }

    #[test]
    fn ember_rift_milestone_has_distinct_title_and_summary() {
        let mut milestones = Milestones::default();

        assert!(milestones.unlock(MilestoneId::EmberRift));

        assert!(milestones.latest.contains("Ember Rift"));
        assert!(milestone_summary(&milestones).contains("Ember Rift invasion"));
        assert!(ALL_MILESTONES.contains(&MilestoneId::EmberRift));
    }

    #[test]
    fn riftbreaker_milestone_has_distinct_title_and_summary() {
        let mut milestones = Milestones::default();

        assert!(milestones.unlock(MilestoneId::Riftbreaker));

        assert!(milestones.latest.contains("Riftbreaker"));
        assert!(milestone_summary(&milestones).contains("30 seconds"));
        assert!(ALL_MILESTONES.contains(&MilestoneId::Riftbreaker));
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
    fn reliquary_seal_milestone_has_distinct_title_and_summary() {
        let mut milestones = Milestones::default();

        assert!(milestones.unlock(MilestoneId::ReliquarySeal));

        assert!(milestones.latest.contains("Reliquary Seal"));
        assert!(milestone_summary(&milestones).contains("vault, blood obelisk, and Ember Rift"));
        assert!(ALL_MILESTONES.contains(&MilestoneId::ReliquarySeal));
    }

    #[test]
    fn reliquary_seal_requires_vault_obelisk_and_rift_milestones() {
        let mut milestones = Milestones::default();

        milestones.unlock(MilestoneId::VaultOpened);
        milestones.unlock(MilestoneId::EmberRift);
        assert!(!reliquary_seal_complete(&milestones));

        milestones.unlock(MilestoneId::BloodObelisk);
        assert!(reliquary_seal_complete(&milestones));
    }

    #[test]
    fn ashen_duel_milestone_has_distinct_title_and_summary() {
        let mut milestones = Milestones::default();

        assert!(milestones.unlock(MilestoneId::AshenDuel));

        assert!(milestones.latest.contains("Ashen Duel"));
        assert!(milestone_summary(&milestones).contains("before Ashen Enrage"));
        assert!(ALL_MILESTONES.contains(&MilestoneId::AshenDuel));
    }

    #[test]
    fn ashen_duel_requires_boss_slain_before_enrage() {
        let mut progress = ChapterProgress {
            boss_slain: false,
            ..default()
        };
        let mut stats = RunStats {
            boss_enraged: true,
            ..default()
        };

        assert!(!ashen_duel_complete(&progress, &stats));

        progress.boss_slain = true;
        assert!(!ashen_duel_complete(&progress, &stats));

        stats.boss_enraged = false;
        assert!(ashen_duel_complete(&progress, &stats));
    }

    #[test]
    fn sentinel_veteran_milestone_has_distinct_title_and_summary() {
        let mut milestones = Milestones::default();

        assert!(milestones.unlock(MilestoneId::SentinelVeteran));

        assert!(milestones.latest.contains("Sentinel Veteran"));
        assert!(milestone_summary(&milestones).contains("Reliquary Sentinel"));
        assert!(ALL_MILESTONES.contains(&MilestoneId::SentinelVeteran));
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
    fn boonbound_milestone_has_distinct_title_and_summary() {
        let mut milestones = Milestones::default();

        assert!(milestones.unlock(MilestoneId::Boonbound));

        assert!(milestones.latest.contains("Boonbound"));
        assert!(milestone_summary(&milestones).contains("Reliquary Boon"));
        assert!(ALL_MILESTONES.contains(&MilestoneId::Boonbound));
    }

    #[test]
    fn boonbound_requires_a_chosen_reliquary_boon() {
        let mut boon = ChapterBoon {
            choice: ChapterBoonChoice::None,
            empowered: false,
            momentum_stacks: 0,
        };

        assert!(!boonbound_complete(&boon));

        boon.choice = ChapterBoonChoice::TitanWard;
        assert!(boonbound_complete(&boon));
    }

    #[test]
    fn alchemist_requires_all_elixir_types() {
        let mut stats = RunStats {
            iron_elixir_used: true,
            haste_elixir_used: true,
            ..default()
        };

        assert!(!alchemist_complete(&stats));

        stats.wrath_elixir_used = true;
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
    fn restored_milestones_rebuild_latest_text() {
        let mut milestones = Milestones::default();

        milestones.restore(vec![MilestoneId::CacheOpened, MilestoneId::BossSlain]);

        assert_eq!(milestones.unlocked.len(), 2);
        assert!(milestone_summary(&milestones).contains("Keeper Slain"));
    }
}
