use crate::{
    BOSS_BREAK_TARGET, DifficultySettings, GameState, RunStats, chapter_completion_essence_reward,
    chapter_completion_reward, chapter_completion_shard_reward,
    data::LootTable,
    enemy::EnemyKilled,
    feedback::CombatEvent,
    loot::soul_sigil_cache_ready_summary,
    lore::LoreJournal,
    not_paused,
    player::{
        DamageBonus, Equipment, GearSlot, Health, Inventory, InventoryItem, LegendaryCodex, Player,
        equip_inventory_item, gear_quality_is_legendary_or_better,
    },
    primal_ember_cache_reward,
};
use bevy::ecs::system::SystemParam;
use bevy::prelude::*;
use serde::{Deserialize, Serialize};

pub const REQUIRED_SEAL_FRAGMENTS: u32 = 3;
pub const CHAPTER_MAIN_QUEST_STEPS: u32 = 5;
const SEAL_FRAGMENT_REWARD_GOLD: u32 = 35;
const SEAL_FRAGMENT_REWARD_SHARDS: u32 = 1;
const FINAL_SEAL_REWARD_ESSENCE: u32 = 1;

#[derive(Debug, Clone, Copy, Eq, PartialEq, Serialize, Deserialize)]
pub enum ChapterPhase {
    Breach,
    Cache,
    Sanctum,
    Ritual,
    Boss,
    Victory,
}

#[derive(Resource)]
pub struct ChapterProgress {
    pub title: String,
    pub phase: ChapterPhase,
    pub objective: String,
    pub enemies_slain: u32,
    pub chest_opened: bool,
    pub altar_activated: bool,
    pub sanctum_wave_spawned: bool,
    pub seal_fragments: u32,
    pub boss_spawned: bool,
    pub boss_slain: bool,
    pub last_kill: String,
    pub interaction_hint: String,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum MainQuestStep {
    BreachReliquary,
    ClaimOuterSeal,
    BreakSealWarden,
    ClaimFinalSeal,
    DefeatKeeper,
}

impl MainQuestStep {
    pub fn label(self) -> &'static str {
        match self {
            Self::BreachReliquary => "Breach",
            Self::ClaimOuterSeal => "Outer Seal",
            Self::BreakSealWarden => "Seal Warden",
            Self::ClaimFinalSeal => "Final Seal",
            Self::DefeatKeeper => "Malrec",
        }
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct MainQuestSnapshot {
    pub completed_steps: u32,
    pub total_steps: u32,
    pub active_step: Option<MainQuestStep>,
    pub active_status: String,
}

impl Default for ChapterProgress {
    fn default() -> Self {
        Self {
            title: "Chapter I: The Ashen Reliquary".to_string(),
            phase: ChapterPhase::Breach,
            objective: "Purge the reliquary guards".to_string(),
            enemies_slain: 0,
            chest_opened: false,
            altar_activated: false,
            sanctum_wave_spawned: false,
            seal_fragments: 0,
            boss_spawned: false,
            boss_slain: false,
            last_kill: "No kills yet".to_string(),
            interaction_hint: "Find the sealed reliquary cache".to_string(),
        }
    }
}

#[derive(Message, Debug, Clone, Copy)]
pub struct SpawnBoss;

#[derive(Message, Debug, Clone, Copy)]
pub struct SpawnSanctumWave;

#[derive(Message, Debug, Clone, Copy, Eq, PartialEq, Serialize, Deserialize)]
pub enum InteractableKind {
    Cache,
    Altar,
    WeaponShrine,
    FortuneShrine,
    StormShrine,
    Merchant,
    LorePage,
    CursedShrine,
    BloodObelisk,
    ReliquaryVault,
    HealingWell,
    EmberRift,
    AshenPylon,
}

#[derive(Message, Debug, Clone, Copy)]
pub struct InteractableUsed {
    pub kind: InteractableKind,
    pub position: Vec3,
}

pub struct ChapterPlugin;

impl Plugin for ChapterPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<ChapterProgress>()
            .add_message::<SpawnBoss>()
            .add_message::<SpawnSanctumWave>()
            .add_message::<InteractableUsed>()
            .add_systems(OnEnter(GameState::InGame), reset_chapter)
            .add_systems(
                Update,
                update_chapter.run_if(in_state(GameState::InGame).and_then(not_paused)),
            );
    }
}

pub fn phase_label(phase: ChapterPhase) -> &'static str {
    match phase {
        ChapterPhase::Breach => "Breach",
        ChapterPhase::Cache => "Cache",
        ChapterPhase::Sanctum => "Sanctum",
        ChapterPhase::Ritual => "Ritual",
        ChapterPhase::Boss => "Keeper",
        ChapterPhase::Victory => "Victory",
    }
}

pub fn main_quest_snapshot(progress: &ChapterProgress, stats: &RunStats) -> MainQuestSnapshot {
    let steps = [
        (
            MainQuestStep::BreachReliquary,
            progress.phase != ChapterPhase::Breach || progress.enemies_slain >= 3,
            format!("{} / 3 kills", progress.enemies_slain.min(3)),
        ),
        (
            MainQuestStep::ClaimOuterSeal,
            progress.chest_opened
                || progress.seal_fragments >= 1
                || matches!(
                    progress.phase,
                    ChapterPhase::Sanctum
                        | ChapterPhase::Ritual
                        | ChapterPhase::Boss
                        | ChapterPhase::Victory
                ),
            if progress.chest_opened || progress.seal_fragments >= 1 {
                "claimed".to_string()
            } else {
                "open sealed cache".to_string()
            },
        ),
        (
            MainQuestStep::BreakSealWarden,
            stats.seal_warden_slain
                || matches!(
                    progress.phase,
                    ChapterPhase::Ritual | ChapterPhase::Boss | ChapterPhase::Victory
                ),
            if stats.seal_warden_slain {
                "slain".to_string()
            } else {
                "pending".to_string()
            },
        ),
        (
            MainQuestStep::ClaimFinalSeal,
            (progress.altar_activated && progress.seal_fragments >= REQUIRED_SEAL_FRAGMENTS)
                || matches!(progress.phase, ChapterPhase::Boss | ChapterPhase::Victory),
            format!(
                "{} / {} seals",
                progress.seal_fragments.min(REQUIRED_SEAL_FRAGMENTS),
                REQUIRED_SEAL_FRAGMENTS
            ),
        ),
        (
            MainQuestStep::DefeatKeeper,
            progress.boss_slain || progress.phase == ChapterPhase::Victory,
            if progress.boss_slain || progress.phase == ChapterPhase::Victory {
                "slain".to_string()
            } else {
                "pending".to_string()
            },
        ),
    ];
    let completed_steps = steps.iter().filter(|(_, complete, _)| *complete).count() as u32;
    let active = steps.iter().find(|(_, complete, _)| !*complete);

    MainQuestSnapshot {
        completed_steps,
        total_steps: CHAPTER_MAIN_QUEST_STEPS,
        active_step: active.map(|(step, _, _)| *step),
        active_status: active
            .map(|(_, _, status)| status.clone())
            .unwrap_or_else(|| "chapter complete".to_string()),
    }
}

pub fn main_quest_summary(progress: &ChapterProgress, stats: &RunStats) -> String {
    let snapshot = main_quest_snapshot(progress, stats);
    match snapshot.active_step {
        Some(step) => format!(
            "{}/{} {} {}",
            snapshot.completed_steps,
            snapshot.total_steps,
            step.label(),
            snapshot.active_status
        ),
        None => format!(
            "Complete {}/{}",
            snapshot.completed_steps, snapshot.total_steps
        ),
    }
}

fn reset_chapter(mut progress: ResMut<ChapterProgress>) {
    *progress = ChapterProgress::default();
}

#[derive(SystemParam)]
struct ChapterResources<'w> {
    progress: ResMut<'w, ChapterProgress>,
    stats: ResMut<'w, RunStats>,
    difficulty: Res<'w, DifficultySettings>,
    journal: Res<'w, LoreJournal>,
    loot_table: Res<'w, LootTable>,
    next_state: ResMut<'w, NextState<GameState>>,
}

#[derive(SystemParam)]
struct ChapterEvents<'w, 's> {
    enemy_killed: MessageReader<'w, 's, EnemyKilled>,
    interaction_used: MessageReader<'w, 's, InteractableUsed>,
    sanctum_spawn: MessageWriter<'w, SpawnSanctumWave>,
    boss_spawn: MessageWriter<'w, SpawnBoss>,
    combat_events: MessageWriter<'w, CombatEvent>,
}

type ChapterRewardPlayerQuery<'w, 's> = Query<
    'w,
    's,
    (
        &'static mut DamageBonus,
        &'static mut Equipment,
        &'static mut Inventory,
        &'static mut Health,
        &'static mut LegendaryCodex,
    ),
    With<Player>,
>;

fn update_chapter(
    mut resources: ChapterResources,
    mut events: ChapterEvents,
    mut player: ChapterRewardPlayerQuery,
    mut layout: ResMut<crate::dungeon::DungeonLayout>,
) {
    for event in events.enemy_killed.read() {
        resources.progress.enemies_slain += 1;
        resources.progress.last_kill = format!(
            "Last slain: {} at {:.0}, {:.0}",
            event.display_name, event.position.x, event.position.z
        );
        if event.enemy_id == "keeper" {
            resources.progress.boss_slain = true;
        }
        if event.enemy_id == "seal_warden" {
            resources.stats.seal_warden_slain = true;
        }
    }

    let interactions = events.interaction_used.read().copied().collect::<Vec<_>>();
    let actions = advance_progress(
        &mut resources.progress,
        &interactions,
        resources.stats.seal_warden_slain,
    );

    if actions.seal_fragments_claimed > 0 {
        grant_seal_fragment_rewards(
            &mut resources.stats,
            actions.seal_fragments_claimed,
            resources.progress.seal_fragments,
            &mut events.combat_events,
        );
    }
    publish_chapter_milestones(&actions.milestones, &mut events.combat_events);
    if actions.spawn_sanctum_wave {
        events.sanctum_spawn.write(SpawnSanctumWave);
        // The sealed gate parts once the sanctum phase begins.
        layout.sanctum_gate_open = true;
    }
    if actions.spawn_boss {
        events.boss_spawn.write(SpawnBoss);
    }
    if actions.victory {
        grant_completion_reward(
            &mut resources.stats,
            &resources.difficulty,
            &resources.journal,
            &resources.loot_table,
            &mut player,
            &mut events.combat_events,
        );
        resources.next_state.set(GameState::Victory);
    }
}

#[derive(Default)]
struct ProgressActions {
    spawn_sanctum_wave: bool,
    spawn_boss: bool,
    victory: bool,
    seal_fragments_claimed: u32,
    milestones: Vec<ChapterMilestone>,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
struct SealFragmentReward {
    gold: u32,
    shards: u32,
    essence: u32,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
enum ChapterMilestone {
    BreachSecured,
    CacheOpened,
    SealWardenBroken,
    FinalSealClaimed,
    KeeperAwakened,
    Victory,
}

fn advance_progress(
    progress: &mut ChapterProgress,
    interactions: &[InteractableUsed],
    seal_warden_slain: bool,
) -> ProgressActions {
    let mut actions = ProgressActions::default();

    for event in interactions {
        match event.kind {
            InteractableKind::Cache => {
                if progress.phase == ChapterPhase::Cache {
                    progress.chest_opened = true;
                    actions.seal_fragments_claimed += u32::from(collect_seal_fragment(progress));
                }
            }
            InteractableKind::Altar => {
                if progress.phase == ChapterPhase::Ritual {
                    progress.altar_activated = true;
                    actions.seal_fragments_claimed += u32::from(collect_seal_fragment(progress));
                }
            }
            InteractableKind::WeaponShrine => {}
            InteractableKind::FortuneShrine => {}
            InteractableKind::StormShrine => {}
            InteractableKind::Merchant => {}
            InteractableKind::LorePage => {}
            InteractableKind::CursedShrine => {}
            InteractableKind::BloodObelisk => {}
            InteractableKind::ReliquaryVault => {}
            InteractableKind::HealingWell => {}
            InteractableKind::EmberRift => {}
            InteractableKind::AshenPylon => {}
        }
    }

    if progress.phase == ChapterPhase::Breach && progress.enemies_slain >= 3 {
        progress.phase = ChapterPhase::Cache;
        progress.objective = seal_objective(progress, "Open the sealed reliquary cache");
        progress.interaction_hint = "Press Space near the cache".to_string();
        actions.milestones.push(ChapterMilestone::BreachSecured);
    }

    if progress.phase == ChapterPhase::Cache && progress.chest_opened {
        progress.phase = ChapterPhase::Sanctum;
        progress.objective = seal_objective(progress, "Survive the cult ambush");
        progress.interaction_hint = "The inner sanctum is open".to_string();
        actions.milestones.push(ChapterMilestone::CacheOpened);
        if !progress.sanctum_wave_spawned {
            progress.sanctum_wave_spawned = true;
            actions.spawn_sanctum_wave = true;
        }
    }

    if progress.phase == ChapterPhase::Sanctum && seal_warden_slain {
        actions.seal_fragments_claimed += u32::from(collect_seal_fragment(progress));
        progress.phase = ChapterPhase::Ritual;
        progress.objective = seal_objective(progress, "Extinguish the ember altar");
        progress.interaction_hint =
            "Press Space near the altar to claim the final seal".to_string();
        actions.milestones.push(ChapterMilestone::SealWardenBroken);
    }

    if progress.phase == ChapterPhase::Ritual
        && progress.altar_activated
        && progress.seal_fragments >= REQUIRED_SEAL_FRAGMENTS
        && !progress.boss_spawned
    {
        progress.phase = ChapterPhase::Boss;
        progress.objective = "Defeat Malrec, Keeper of Ash".to_string();
        progress.interaction_hint = "Malrec has entered the reliquary".to_string();
        progress.boss_spawned = true;
        actions.milestones.push(ChapterMilestone::FinalSealClaimed);
        actions.milestones.push(ChapterMilestone::KeeperAwakened);
        actions.spawn_boss = true;
    }

    if progress.boss_slain {
        progress.phase = ChapterPhase::Victory;
        progress.objective = "The reliquary is cleansed".to_string();
        actions.milestones.push(ChapterMilestone::Victory);
        actions.victory = true;
    }

    actions
}

fn collect_seal_fragment(progress: &mut ChapterProgress) -> bool {
    let previous = progress.seal_fragments;
    progress.seal_fragments = (progress.seal_fragments + 1).min(REQUIRED_SEAL_FRAGMENTS);
    progress.seal_fragments > previous
}

pub fn seal_objective(progress: &ChapterProgress, action: &str) -> String {
    format!(
        "{action} ({}/{REQUIRED_SEAL_FRAGMENTS} seals)",
        progress.seal_fragments
    )
}

fn publish_chapter_milestones(
    milestones: &[ChapterMilestone],
    combat_events: &mut MessageWriter<CombatEvent>,
) {
    for milestone in milestones {
        combat_events.write(CombatEvent {
            text: chapter_milestone_text(*milestone).to_string(),
        });
    }
}

fn chapter_milestone_text(milestone: ChapterMilestone) -> &'static str {
    match milestone {
        ChapterMilestone::BreachSecured => {
            "MAIN Breach Secured | NEXT open sealed cache | REWARD outer seal + cache loot"
        }
        ChapterMilestone::CacheOpened => {
            "MAIN Cache Opened | NEXT survive sanctum ambush | REWARD first seal + upgraded loot"
        }
        ChapterMilestone::SealWardenBroken => {
            "MAIN Seal Warden Broken | NEXT extinguish ember altar | REWARD second seal + altar heal"
        }
        ChapterMilestone::FinalSealClaimed => {
            "MAIN Final Seal Claimed | NEXT face Malrec | REWARD essence + boss gate"
        }
        ChapterMilestone::KeeperAwakened => {
            "BOSS Malrec Awakened | NEXT break stagger, deny enrage | PAYOFF Soul Sigil + chapter cache"
        }
        ChapterMilestone::Victory => {
            "MAIN Complete | NEXT claim cache, review build | REWARD Reliquary cleansed"
        }
    }
}

fn grant_seal_fragment_rewards(
    stats: &mut RunStats,
    claimed: u32,
    total_fragments: u32,
    combat_events: &mut MessageWriter<CombatEvent>,
) {
    let reward = seal_fragment_reward(claimed, total_fragments);
    stats.gold += reward.gold;
    stats.ember_shards += reward.shards;
    stats.affix_essence += reward.essence;
    combat_events.write(CombatEvent {
        text: format!(
            "Reliquary seal {}/{} recovered: +{} gold +{} shards{}",
            total_fragments,
            REQUIRED_SEAL_FRAGMENTS,
            reward.gold,
            reward.shards,
            if reward.essence > 0 {
                " +1 essence"
            } else {
                ""
            }
        ),
    });
}

fn seal_fragment_reward(claimed: u32, total_fragments: u32) -> SealFragmentReward {
    SealFragmentReward {
        gold: SEAL_FRAGMENT_REWARD_GOLD * claimed,
        shards: SEAL_FRAGMENT_REWARD_SHARDS * claimed,
        essence: if claimed > 0 && total_fragments >= REQUIRED_SEAL_FRAGMENTS {
            FINAL_SEAL_REWARD_ESSENCE
        } else {
            0
        },
    }
}

pub fn seal_fragment_reward_preview(total_fragments_after_claim: u32) -> String {
    let reward = seal_fragment_reward(1, total_fragments_after_claim);
    format!(
        "+{}g +{}s{}",
        reward.gold,
        reward.shards,
        if reward.essence > 0 { " +1e" } else { "" }
    )
}

fn grant_completion_reward(
    stats: &mut RunStats,
    difficulty: &DifficultySettings,
    journal: &LoreJournal,
    loot_table: &LootTable,
    player: &mut ChapterRewardPlayerQuery,
    combat_events: &mut MessageWriter<CombatEvent>,
) {
    if stats.completion_reward_claimed {
        return;
    }
    let gold_reward = chapter_completion_reward(
        difficulty.current,
        stats.elapsed_secs,
        stats.kills,
        journal.entries.len(),
    );
    let shard_reward = chapter_completion_shard_reward(
        difficulty.current,
        stats.elapsed_secs,
        stats.kills,
        journal.entries.len(),
    );
    let essence_reward = chapter_completion_essence_reward(
        difficulty.current,
        stats.elapsed_secs,
        stats.kills,
        journal.entries.len(),
    );
    stats.gold += gold_reward;
    stats.ember_shards += shard_reward;
    stats.affix_essence += essence_reward;
    stats.completion_bonus_gold = gold_reward;
    stats.completion_bonus_shards = shard_reward;
    stats.completion_bonus_essence = essence_reward;
    let primal_reward = primal_ember_cache_reward(
        difficulty.current,
        stats.elapsed_secs,
        stats.kills,
        stats.boss_staggers,
        stats.boss_enraged,
        stats.echo_keystones,
    );
    if !primal_reward.is_empty() {
        let echo_items = primal_cache_echo_item_bonus(stats.echo_keystones);
        stats.gold += primal_reward.gold;
        stats.ember_shards += primal_reward.shards;
        stats.affix_essence += primal_reward.essence;
        stats.primal_caches += 1;
        stats.primal_cache_bonus_gold += primal_reward.gold;
        stats.primal_cache_bonus_shards += primal_reward.shards;
        stats.primal_cache_bonus_essence += primal_reward.essence;
        stats.primal_cache_echo_items += echo_items;
        grant_primal_cache_items(stats, loot_table, player, combat_events);
    }
    grant_malrec_soul_sigils(stats, combat_events);
    stats.completion_reward_claimed = true;
    combat_events.write(CombatEvent {
        text: format!(
            "Chapter reward: {gold_reward} gold, {shard_reward} shards, {essence_reward} essence"
        ),
    });
    if !primal_reward.is_empty() {
        combat_events.write(CombatEvent {
            text: format!(
                "Primal Ember Cache: {} gold, {} shards, {} essence",
                primal_reward.gold, primal_reward.shards, primal_reward.essence
            ),
        });
    }
}

fn grant_malrec_soul_sigils(stats: &mut RunStats, combat_events: &mut MessageWriter<CombatEvent>) {
    let sigils = malrec_soul_sigil_reward(stats);
    if sigils == 0 {
        return;
    }
    stats.malrec_soul_sigils = stats.malrec_soul_sigils.saturating_add(sigils);
    stats.malrec_soul_sigils_earned = stats.malrec_soul_sigils_earned.saturating_add(sigils);
    combat_events.write(CombatEvent {
        text: malrec_soul_sigil_reward_text(sigils, stats.malrec_soul_sigils),
    });
}

fn malrec_soul_sigil_reward(stats: &RunStats) -> u32 {
    1 + u32::from(stats.boss_staggers >= BOSS_BREAK_TARGET) + u32::from(!stats.boss_enraged)
}

fn malrec_soul_sigil_reward_text(sigils_gained: u32, total_sigils: u32) -> String {
    let plural = if sigils_gained == 1 { "" } else { "s" };
    match soul_sigil_cache_ready_summary(total_sigils) {
        Some(cache) => format!(
            "Malrec Soul Sigil{} +{}: {} ready at the quartermaster",
            plural, sigils_gained, cache
        ),
        None => format!(
            "Malrec Soul Sigil{} +{}: collect more for a Trophy Cache",
            plural, sigils_gained
        ),
    }
}

fn grant_primal_cache_items(
    stats: &mut RunStats,
    loot_table: &LootTable,
    player: &mut ChapterRewardPlayerQuery,
    combat_events: &mut MessageWriter<CombatEvent>,
) {
    let pending_items = pending_primal_cache_items(stats);
    if pending_items == 0 {
        return;
    }
    let Ok((mut damage_bonus, mut equipment, mut inventory, mut health, mut codex)) =
        player.single_mut()
    else {
        return;
    };
    let Some(item) = primal_cache_item(loot_table) else {
        return;
    };

    for _ in 0..pending_items {
        if codex.unlock(item.legendary_power) {
            combat_events.write(CombatEvent {
                text: format!("Codex unlocked: {}", item.legendary_power.label()),
            });
        }
        let stored = inventory.add(item.clone());
        equip_inventory_item(&item, &mut damage_bonus, &mut equipment, &mut health);
        stats.primal_cache_items_claimed += 1;
        combat_events.write(CombatEvent {
            text: format!(
                "Primal Ember Cache item: {}{}",
                item.name,
                if stored { "" } else { " (inventory full)" }
            ),
        });
    }
}

fn pending_primal_cache_items(stats: &RunStats) -> u32 {
    primal_cache_item_target(stats).saturating_sub(stats.primal_cache_items_claimed)
}

fn primal_cache_item_target(stats: &RunStats) -> u32 {
    stats
        .primal_caches
        .saturating_add(stats.primal_cache_echo_items)
}

fn primal_cache_echo_item_bonus(echo_keystones: u32) -> u32 {
    echo_keystones.min(1)
}

fn primal_cache_item(loot_table: &LootTable) -> Option<InventoryItem> {
    let weapon = loot_table
        .weapons
        .iter()
        .find(|weapon| weapon.quality == "primal")
        .or_else(|| {
            loot_table
                .weapons
                .iter()
                .find(|weapon| gear_quality_is_legendary_or_better(&weapon.quality))
        })
        .or_else(|| loot_table.weapons.first())?;
    Some(InventoryItem {
        name: format!("{} {}", weapon.quality, weapon.name),
        quality: weapon.quality.clone(),
        damage_bonus: weapon.damage_bonus,
        crit_chance: weapon.crit_chance,
        health_bonus: weapon.health_bonus,
        armor_bonus: weapon.armor_bonus,
        legendary_power: weapon.legendary_power,
        temper_level: 0,
        socketed_gem: None,
        slot: GearSlot::Weapon,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn chapter_requires_cache_then_altar_before_boss() {
        let mut progress = ChapterProgress {
            enemies_slain: 3,
            ..Default::default()
        };
        let actions = advance_progress(&mut progress, &[], false);
        assert_eq!(progress.phase, ChapterPhase::Cache);
        assert_eq!(progress.seal_fragments, 0);
        assert_eq!(actions.seal_fragments_claimed, 0);
        assert!(!progress.sanctum_wave_spawned);
        assert!(!actions.spawn_sanctum_wave);
        assert_eq!(actions.milestones, vec![ChapterMilestone::BreachSecured]);

        let actions = advance_progress(
            &mut progress,
            &[InteractableUsed {
                kind: InteractableKind::Cache,
                position: Vec3::ZERO,
            }],
            false,
        );
        assert_eq!(progress.phase, ChapterPhase::Sanctum);
        assert_eq!(progress.seal_fragments, 1);
        assert_eq!(actions.seal_fragments_claimed, 1);
        assert!(progress.sanctum_wave_spawned);
        assert!(actions.spawn_sanctum_wave);
        assert_eq!(actions.milestones, vec![ChapterMilestone::CacheOpened]);

        progress.enemies_slain = 5;
        let actions = advance_progress(&mut progress, &[], true);
        assert_eq!(progress.phase, ChapterPhase::Ritual);
        assert_eq!(progress.seal_fragments, 2);
        assert_eq!(actions.seal_fragments_claimed, 1);
        assert!(!progress.boss_spawned);
        assert!(!actions.spawn_boss);
        assert_eq!(actions.milestones, vec![ChapterMilestone::SealWardenBroken]);

        let actions = advance_progress(
            &mut progress,
            &[InteractableUsed {
                kind: InteractableKind::Altar,
                position: Vec3::ZERO,
            }],
            true,
        );
        assert_eq!(progress.phase, ChapterPhase::Boss);
        assert_eq!(progress.seal_fragments, REQUIRED_SEAL_FRAGMENTS);
        assert_eq!(actions.seal_fragments_claimed, 1);
        assert!(progress.boss_spawned);
        assert!(actions.spawn_boss);
        assert_eq!(
            actions.milestones,
            vec![
                ChapterMilestone::FinalSealClaimed,
                ChapterMilestone::KeeperAwakened
            ]
        );
    }

    #[test]
    fn boss_waits_for_all_reliquary_seals() {
        let mut progress = ChapterProgress {
            phase: ChapterPhase::Ritual,
            altar_activated: false,
            seal_fragments: 1,
            ..Default::default()
        };

        let actions = advance_progress(
            &mut progress,
            &[InteractableUsed {
                kind: InteractableKind::Altar,
                position: Vec3::ZERO,
            }],
            false,
        );

        assert_eq!(progress.phase, ChapterPhase::Ritual);
        assert_eq!(progress.seal_fragments, 2);
        assert_eq!(actions.seal_fragments_claimed, 1);
        assert!(!progress.boss_spawned);
        assert!(!actions.spawn_boss);
        assert!(seal_objective(&progress, "Recover relic seals").contains("2/3"));
    }

    #[test]
    fn reliquary_seal_fragments_pay_mainline_rewards() {
        assert_eq!(
            seal_fragment_reward(1, 1),
            SealFragmentReward {
                gold: 35,
                shards: 1,
                essence: 0,
            }
        );
        assert_eq!(
            seal_fragment_reward(1, REQUIRED_SEAL_FRAGMENTS),
            SealFragmentReward {
                gold: 35,
                shards: 1,
                essence: 1,
            }
        );
        assert_eq!(
            seal_fragment_reward(0, REQUIRED_SEAL_FRAGMENTS),
            SealFragmentReward {
                gold: 0,
                shards: 0,
                essence: 0,
            }
        );
        assert_eq!(seal_fragment_reward_preview(1), "+35g +1s");
        assert_eq!(
            seal_fragment_reward_preview(REQUIRED_SEAL_FRAGMENTS),
            "+35g +1s +1e"
        );
    }

    #[test]
    fn malrec_soul_sigil_reward_scales_with_boss_execution() {
        let enraged_without_breaks = RunStats {
            boss_enraged: true,
            boss_staggers: 0,
            ..Default::default()
        };
        let clean_kill = RunStats {
            boss_enraged: false,
            boss_staggers: BOSS_BREAK_TARGET,
            ..Default::default()
        };
        let break_but_enraged = RunStats {
            boss_enraged: true,
            boss_staggers: BOSS_BREAK_TARGET,
            ..Default::default()
        };

        assert_eq!(malrec_soul_sigil_reward(&enraged_without_breaks), 1);
        assert_eq!(malrec_soul_sigil_reward(&break_but_enraged), 2);
        assert_eq!(malrec_soul_sigil_reward(&clean_kill), 3);

        let partial_text = malrec_soul_sigil_reward_text(1, 1);
        assert!(partial_text.contains("collect more"));

        let cache_ready_text = malrec_soul_sigil_reward_text(2, 2);
        assert!(cache_ready_text.contains("Trophy Cache"));
        assert!(cache_ready_text.contains("quartermaster"));
    }

    #[test]
    fn chapter_milestone_text_surfaces_main_quest_progress() {
        for milestone in [
            ChapterMilestone::BreachSecured,
            ChapterMilestone::CacheOpened,
            ChapterMilestone::SealWardenBroken,
            ChapterMilestone::FinalSealClaimed,
            ChapterMilestone::KeeperAwakened,
            ChapterMilestone::Victory,
        ] {
            let text = chapter_milestone_text(milestone);
            assert!(text.contains("NEXT"), "{text}");
            assert!(text.contains("REWARD") || text.contains("PAYOFF"), "{text}");
            assert!(text.len() <= 96, "{text}");
        }
        assert!(chapter_milestone_text(ChapterMilestone::BreachSecured).contains("sealed cache"));
        assert!(chapter_milestone_text(ChapterMilestone::CacheOpened).contains("ambush"));
        assert!(chapter_milestone_text(ChapterMilestone::SealWardenBroken).contains("altar"));
        assert!(chapter_milestone_text(ChapterMilestone::FinalSealClaimed).contains("Malrec"));
        assert!(chapter_milestone_text(ChapterMilestone::KeeperAwakened).contains("Soul Sigil"));
        assert!(chapter_milestone_text(ChapterMilestone::Victory).contains("build"));
    }

    #[test]
    fn sanctum_waits_for_seal_warden_before_second_seal() {
        let mut progress = ChapterProgress {
            phase: ChapterPhase::Sanctum,
            enemies_slain: 8,
            seal_fragments: 1,
            ..Default::default()
        };

        let actions = advance_progress(&mut progress, &[], false);

        assert_eq!(progress.phase, ChapterPhase::Sanctum);
        assert_eq!(progress.seal_fragments, 1);
        assert_eq!(actions.seal_fragments_claimed, 0);

        let actions = advance_progress(&mut progress, &[], true);

        assert_eq!(progress.phase, ChapterPhase::Ritual);
        assert_eq!(progress.seal_fragments, 2);
        assert_eq!(actions.seal_fragments_claimed, 1);
    }

    #[test]
    fn main_quest_snapshot_tracks_active_chapter_step() {
        let mut progress = ChapterProgress {
            enemies_slain: 2,
            ..Default::default()
        };
        let mut stats = RunStats::default();

        let snapshot = main_quest_snapshot(&progress, &stats);
        assert_eq!(snapshot.completed_steps, 0);
        assert_eq!(snapshot.total_steps, CHAPTER_MAIN_QUEST_STEPS);
        assert_eq!(snapshot.active_step, Some(MainQuestStep::BreachReliquary));
        assert_eq!(snapshot.active_status, "2 / 3 kills");
        assert!(main_quest_summary(&progress, &stats).contains("Breach"));

        progress.phase = ChapterPhase::Cache;
        progress.enemies_slain = 3;
        let snapshot = main_quest_snapshot(&progress, &stats);
        assert_eq!(snapshot.completed_steps, 1);
        assert_eq!(snapshot.active_step, Some(MainQuestStep::ClaimOuterSeal));
        assert_eq!(snapshot.active_status, "open sealed cache");

        progress.phase = ChapterPhase::Sanctum;
        progress.chest_opened = true;
        progress.seal_fragments = 1;
        let snapshot = main_quest_snapshot(&progress, &stats);
        assert_eq!(snapshot.completed_steps, 2);
        assert_eq!(snapshot.active_step, Some(MainQuestStep::BreakSealWarden));
        assert_eq!(snapshot.active_status, "pending");

        progress.phase = ChapterPhase::Ritual;
        progress.seal_fragments = 2;
        stats.seal_warden_slain = true;
        let snapshot = main_quest_snapshot(&progress, &stats);
        assert_eq!(snapshot.completed_steps, 3);
        assert_eq!(snapshot.active_step, Some(MainQuestStep::ClaimFinalSeal));
        assert_eq!(snapshot.active_status, "2 / 3 seals");

        progress.phase = ChapterPhase::Boss;
        progress.altar_activated = true;
        progress.seal_fragments = REQUIRED_SEAL_FRAGMENTS;
        let snapshot = main_quest_snapshot(&progress, &stats);
        assert_eq!(snapshot.completed_steps, 4);
        assert_eq!(snapshot.active_step, Some(MainQuestStep::DefeatKeeper));
        assert_eq!(snapshot.active_status, "pending");

        progress.phase = ChapterPhase::Victory;
        progress.boss_slain = true;
        let snapshot = main_quest_snapshot(&progress, &stats);
        assert_eq!(snapshot.completed_steps, CHAPTER_MAIN_QUEST_STEPS);
        assert_eq!(snapshot.active_step, None);
        assert_eq!(main_quest_summary(&progress, &stats), "Complete 5/5");
    }

    #[test]
    fn completion_reward_is_claimed_once() {
        let mut stats = RunStats {
            kills: 7,
            elapsed_secs: 220.0,
            ..Default::default()
        };
        let difficulty = DifficultySettings {
            current: crate::Difficulty::Hell,
        };
        let mut journal = LoreJournal::default();
        journal.discover(crate::lore::LoreEntryId::StewardWarning);

        let gold_reward = chapter_completion_reward(
            difficulty.current,
            stats.elapsed_secs,
            stats.kills,
            journal.entries.len(),
        );
        let shard_reward = chapter_completion_shard_reward(
            difficulty.current,
            stats.elapsed_secs,
            stats.kills,
            journal.entries.len(),
        );
        let essence_reward = chapter_completion_essence_reward(
            difficulty.current,
            stats.elapsed_secs,
            stats.kills,
            journal.entries.len(),
        );
        stats.gold += gold_reward;
        stats.ember_shards += shard_reward;
        stats.affix_essence += essence_reward;
        stats.completion_bonus_gold = gold_reward;
        stats.completion_bonus_shards = shard_reward;
        stats.completion_bonus_essence = essence_reward;
        stats.completion_reward_claimed = true;

        assert_eq!(stats.completion_bonus_gold, gold_reward);
        assert_eq!(stats.completion_bonus_shards, shard_reward);
        assert_eq!(stats.completion_bonus_essence, essence_reward);
        assert!(stats.ember_shards >= shard_reward);
        assert!(stats.affix_essence >= essence_reward);
        assert!(stats.completion_reward_claimed);
    }

    #[test]
    fn primal_cache_item_prefers_primal_weapon() {
        let table = LootTable {
            weapons: vec![
                test_weapon(
                    "Iron Fang",
                    "common",
                    4.0,
                    crate::player::LegendaryPower::None,
                ),
                test_weapon(
                    "Ember-Saint Relic Blade",
                    "legendary",
                    16.0,
                    crate::player::LegendaryPower::Emberbrand,
                ),
                test_weapon(
                    "Primal Ashen Reliquary Reaver",
                    "primal",
                    28.0,
                    crate::player::LegendaryPower::Stormbrand,
                ),
            ],
        };

        let item = primal_cache_item(&table).unwrap();

        assert_eq!(item.quality, "primal");
        assert_eq!(item.name, "primal Primal Ashen Reliquary Reaver");
        assert_eq!(
            item.legendary_power,
            crate::player::LegendaryPower::Stormbrand
        );
    }

    #[test]
    fn primal_cache_item_falls_back_to_legendary_when_needed() {
        let table = LootTable {
            weapons: vec![
                test_weapon(
                    "Iron Fang",
                    "common",
                    4.0,
                    crate::player::LegendaryPower::None,
                ),
                test_weapon(
                    "Aegis-Saint Reliquary Guard",
                    "legendary",
                    12.0,
                    crate::player::LegendaryPower::Aegisbrand,
                ),
            ],
        };

        let item = primal_cache_item(&table).unwrap();

        assert_eq!(item.quality, "legendary");
        assert_eq!(
            item.legendary_power,
            crate::player::LegendaryPower::Aegisbrand
        );
    }

    #[test]
    fn pending_primal_cache_items_only_counts_unclaimed_items() {
        let mut stats = RunStats {
            primal_caches: 2,
            primal_cache_items_claimed: 1,
            ..default()
        };

        assert_eq!(pending_primal_cache_items(&stats), 1);

        stats.primal_cache_items_claimed = 3;
        assert_eq!(pending_primal_cache_items(&stats), 0);
    }

    #[test]
    fn echo_keystone_adds_one_extra_primal_cache_item() {
        let mut stats = RunStats {
            primal_caches: 1,
            primal_cache_echo_items: primal_cache_echo_item_bonus(3),
            ..default()
        };

        assert_eq!(stats.primal_cache_echo_items, 1);
        assert_eq!(pending_primal_cache_items(&stats), 2);

        stats.primal_cache_items_claimed = 2;
        assert_eq!(pending_primal_cache_items(&stats), 0);
    }

    fn test_weapon(
        name: &str,
        quality: &str,
        damage_bonus: f32,
        legendary_power: crate::player::LegendaryPower,
    ) -> crate::data::LootEntry {
        crate::data::LootEntry {
            name: name.to_string(),
            quality: quality.to_string(),
            weight: 1,
            damage_bonus,
            crit_chance: 0.1,
            health_bonus: 0.0,
            armor_bonus: 0.0,
            legendary_power,
            potions: 0,
            slot: GearSlot::Weapon,
        }
    }
}
