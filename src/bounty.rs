use crate::{
    CHAMPION_PACK_TARGET, Difficulty, DifficultySettings, GameState, RunStats,
    chapter::{InteractableKind, InteractableUsed},
    dungeon::BREAKER_TARGET_BREAKABLES,
    enemy::EnemyKilled,
    feedback::CombatEvent,
    not_paused,
    ui::ChapterRecords,
};
use bevy::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Default, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BountyKind {
    #[default]
    SlayEnemies,
    RecoverLorePages,
    SmashBreakables,
    HuntChampionPack,
    OpenTreasureVault,
    SlayAffixedElites,
}

impl BountyKind {
    pub fn objective_label(self) -> &'static str {
        match self {
            Self::SlayEnemies => "Slay enemies",
            Self::RecoverLorePages => "Recover lore pages",
            Self::SmashBreakables => "Smash reliquary vessels",
            Self::HuntChampionPack => "Break the champion pack",
            Self::OpenTreasureVault => "Open the treasure vault",
            Self::SlayAffixedElites => "Slay affixed elites",
        }
    }
}

#[derive(Resource, Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct BountyProgress {
    pub title: String,
    pub kind: BountyKind,
    pub current: u32,
    pub target: u32,
    pub reward_gold: u32,
    pub reward_shards: u32,
    #[serde(default)]
    pub reward_essence: u32,
    pub completed: bool,
    pub reward_claimed: bool,
    pub latest: String,
}

impl Default for BountyProgress {
    fn default() -> Self {
        bounty_for_difficulty(Difficulty::Normal)
    }
}

pub fn bounty_for_difficulty(difficulty: Difficulty) -> BountyProgress {
    bounty_for_run(difficulty, 0)
}

pub fn bounty_for_run(difficulty: Difficulty, clears: u32) -> BountyProgress {
    let kind = bounty_kind_for_run(difficulty, clears);
    bounty_for_kind(difficulty, kind)
}

fn bounty_kind_for_run(difficulty: Difficulty, clears: u32) -> BountyKind {
    let rotation = match difficulty {
        Difficulty::Normal => [
            BountyKind::SlayEnemies,
            BountyKind::SmashBreakables,
            BountyKind::HuntChampionPack,
            BountyKind::RecoverLorePages,
            BountyKind::OpenTreasureVault,
            BountyKind::SlayAffixedElites,
        ],
        Difficulty::Nightmare => [
            BountyKind::RecoverLorePages,
            BountyKind::SlayAffixedElites,
            BountyKind::HuntChampionPack,
            BountyKind::OpenTreasureVault,
            BountyKind::SmashBreakables,
            BountyKind::SlayEnemies,
        ],
        Difficulty::Hell => [
            BountyKind::RecoverLorePages,
            BountyKind::SmashBreakables,
            BountyKind::SlayAffixedElites,
            BountyKind::HuntChampionPack,
            BountyKind::OpenTreasureVault,
            BountyKind::SlayEnemies,
        ],
        Difficulty::Torment => [
            BountyKind::RecoverLorePages,
            BountyKind::SlayAffixedElites,
            BountyKind::HuntChampionPack,
            BountyKind::OpenTreasureVault,
            BountyKind::SmashBreakables,
            BountyKind::SlayEnemies,
        ],
    };
    rotation[(clears as usize) % rotation.len()]
}

fn bounty_for_kind(difficulty: Difficulty, kind: BountyKind) -> BountyProgress {
    let (reward_gold, reward_shards, reward_essence) = bounty_reward_for_difficulty(difficulty);
    match kind {
        BountyKind::SlayEnemies => BountyProgress {
            title: match difficulty {
                Difficulty::Normal => "Reliquary Hunt",
                Difficulty::Nightmare => "Nightmare Hunt",
                Difficulty::Hell => "Hellborne Hunt",
                Difficulty::Torment => "Torment Hunt",
            }
            .to_string(),
            kind,
            current: 0,
            target: match difficulty {
                Difficulty::Normal => 8,
                Difficulty::Nightmare => 10,
                Difficulty::Hell => 12,
                Difficulty::Torment => 14,
            },
            reward_gold,
            reward_shards,
            reward_essence,
            completed: false,
            reward_claimed: false,
            latest: "Optional bounty: slay enemies for a cache reward".to_string(),
        },
        BountyKind::RecoverLorePages => BountyProgress {
            title: match difficulty {
                Difficulty::Normal => "Steward's Notes",
                Difficulty::Nightmare => "Steward's Ledger",
                Difficulty::Hell => "Ashen Testament",
                Difficulty::Torment => "Torment Testament",
            }
            .to_string(),
            kind,
            current: 0,
            target: match difficulty {
                Difficulty::Normal => 1,
                Difficulty::Nightmare => 2,
                Difficulty::Hell | Difficulty::Torment => 3,
            },
            reward_gold,
            reward_shards,
            reward_essence,
            completed: false,
            reward_claimed: false,
            latest: "Optional bounty: recover lore pages for the quartermaster".to_string(),
        },
        BountyKind::SmashBreakables => BountyProgress {
            title: match difficulty {
                Difficulty::Normal => "Breaker Tithe",
                Difficulty::Nightmare => "Shattered Offering",
                Difficulty::Hell => "Ashen Offering",
                Difficulty::Torment => "Torment Offering",
            }
            .to_string(),
            kind,
            current: 0,
            target: BREAKER_TARGET_BREAKABLES,
            reward_gold,
            reward_shards,
            reward_essence,
            completed: false,
            reward_claimed: false,
            latest: "Optional bounty: smash reliquary vessels for a cache".to_string(),
        },
        BountyKind::HuntChampionPack => BountyProgress {
            title: match difficulty {
                Difficulty::Normal => "Champion Writ",
                Difficulty::Nightmare => "Nightmare Writ",
                Difficulty::Hell => "Hell Champion Writ",
                Difficulty::Torment => "Torment Champion Writ",
            }
            .to_string(),
            kind,
            current: 0,
            target: CHAMPION_PACK_TARGET,
            reward_gold,
            reward_shards,
            reward_essence,
            completed: false,
            reward_claimed: false,
            latest: "Optional bounty: break the roaming champion pack".to_string(),
        },
        BountyKind::OpenTreasureVault => BountyProgress {
            title: match difficulty {
                Difficulty::Normal => "Treasure Writ",
                Difficulty::Nightmare => "Nightmare Vault Writ",
                Difficulty::Hell => "Hell Vault Writ",
                Difficulty::Torment => "Torment Vault Writ",
            }
            .to_string(),
            kind,
            current: 0,
            target: 1,
            reward_gold,
            reward_shards,
            reward_essence,
            completed: false,
            reward_claimed: false,
            latest: "Optional bounty: open the treasure imp's reliquary vault".to_string(),
        },
        BountyKind::SlayAffixedElites => BountyProgress {
            title: match difficulty {
                Difficulty::Normal => "Affix Writ",
                Difficulty::Nightmare => "Nightmare Affix Writ",
                Difficulty::Hell => "Hell Affix Writ",
                Difficulty::Torment => "Torment Affix Writ",
            }
            .to_string(),
            kind,
            current: 0,
            target: match difficulty {
                Difficulty::Normal => 2,
                Difficulty::Nightmare => 3,
                Difficulty::Hell => 4,
                Difficulty::Torment => 5,
            },
            reward_gold,
            reward_shards,
            reward_essence,
            completed: false,
            reward_claimed: false,
            latest: "Optional bounty: slay multi-affix elites for a cache".to_string(),
        },
    }
}

fn bounty_reward_for_difficulty(difficulty: Difficulty) -> (u32, u32, u32) {
    match difficulty {
        Difficulty::Normal => (95, 2, 1),
        Difficulty::Nightmare => (140, 4, 2),
        Difficulty::Hell => (190, 6, 4),
        Difficulty::Torment => (260, 9, 7),
    }
}

pub fn bounty_summary(bounty: &BountyProgress) -> String {
    if bounty.reward_claimed {
        return format!("Bounty complete: {}", bounty.title);
    }
    format!(
        "Bounty: {} - {} {}/{} for {} gold + {} shards + {} essence",
        bounty.title,
        bounty.kind.objective_label(),
        bounty.current.min(bounty.target),
        bounty.target,
        bounty.reward_gold,
        bounty.reward_shards,
        bounty.reward_essence
    )
}

pub fn record_bounty_progress(bounty: &mut BountyProgress, amount: u32) -> bool {
    if bounty.completed || amount == 0 {
        return false;
    }
    bounty.current = bounty.current.saturating_add(amount).min(bounty.target);
    if bounty.current >= bounty.target {
        bounty.completed = true;
        bounty.latest = format!("{} completed", bounty.title);
        return true;
    }
    bounty.latest = format!(
        "{}: {}/{}",
        bounty.kind.objective_label(),
        bounty.current,
        bounty.target
    );
    false
}

pub fn claim_bounty_reward(bounty: &mut BountyProgress, stats: &mut RunStats) -> bool {
    if !bounty.completed || bounty.reward_claimed {
        return false;
    }
    stats.gold += bounty.reward_gold;
    stats.ember_shards += bounty.reward_shards;
    stats.affix_essence += bounty.reward_essence;
    bounty.reward_claimed = true;
    true
}

fn affixed_elite_bounty_credit(affix_count: usize) -> u32 {
    u32::from(affix_count >= 2)
}

pub struct BountyPlugin;

impl Plugin for BountyPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<BountyProgress>()
            .add_systems(OnEnter(GameState::InGame), reset_bounty)
            .add_systems(
                Update,
                update_bounty.run_if(in_state(GameState::InGame).and_then(not_paused)),
            );
    }
}

fn reset_bounty(
    mut bounty: ResMut<BountyProgress>,
    difficulty: Res<DifficultySettings>,
    records: Res<ChapterRecords>,
) {
    *bounty = bounty_for_run(difficulty.current, records.clears(difficulty.current));
}

fn update_bounty(
    mut enemy_killed: MessageReader<EnemyKilled>,
    mut interactions: MessageReader<InteractableUsed>,
    mut bounty: ResMut<BountyProgress>,
    mut stats: ResMut<RunStats>,
    mut combat_events: MessageWriter<CombatEvent>,
) {
    match bounty.kind {
        BountyKind::SlayEnemies => {
            for _ in enemy_killed.read() {
                if record_bounty_progress(&mut bounty, 1) {
                    combat_events.write(CombatEvent {
                        text: format!("Bounty complete: {}", bounty.title),
                    });
                }
            }
            interactions.clear();
        }
        BountyKind::RecoverLorePages => {
            enemy_killed.clear();
            for event in interactions.read() {
                if event.kind == InteractableKind::LorePage
                    && record_bounty_progress(&mut bounty, 1)
                {
                    combat_events.write(CombatEvent {
                        text: format!("Bounty complete: {}", bounty.title),
                    });
                }
            }
        }
        BountyKind::SmashBreakables => {
            enemy_killed.clear();
            interactions.clear();
            update_bounty_absolute_progress(
                &mut bounty,
                stats.breakables_smashed,
                &mut combat_events,
            );
        }
        BountyKind::HuntChampionPack => {
            enemy_killed.clear();
            interactions.clear();
            update_bounty_absolute_progress(
                &mut bounty,
                stats.champion_pack_kills,
                &mut combat_events,
            );
        }
        BountyKind::OpenTreasureVault => {
            enemy_killed.clear();
            interactions.clear();
            update_bounty_absolute_progress(
                &mut bounty,
                stats.treasure_vaults_opened,
                &mut combat_events,
            );
        }
        BountyKind::SlayAffixedElites => {
            for event in enemy_killed.read() {
                if record_bounty_progress(
                    &mut bounty,
                    affixed_elite_bounty_credit(event.affix_count),
                ) {
                    combat_events.write(CombatEvent {
                        text: format!("Bounty complete: {}", bounty.title),
                    });
                }
            }
            interactions.clear();
        }
    }

    if claim_bounty_reward(&mut bounty, &mut stats) {
        combat_events.write(CombatEvent {
            text: format!(
                "Bounty cache: {} gold, {} shards, {} essence",
                bounty.reward_gold, bounty.reward_shards, bounty.reward_essence
            ),
        });
    }
}

fn update_bounty_absolute_progress(
    bounty: &mut BountyProgress,
    observed: u32,
    combat_events: &mut MessageWriter<CombatEvent>,
) {
    if bounty.completed {
        return;
    }
    let progress = observed.saturating_sub(bounty.current);
    if progress > 0 && record_bounty_progress(bounty, progress) {
        combat_events.write(CombatEvent {
            text: format!("Bounty complete: {}", bounty.title),
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bounty_progress_completes_at_target_once() {
        let mut bounty = BountyProgress {
            target: 3,
            ..default()
        };

        assert!(!record_bounty_progress(&mut bounty, 2));
        assert_eq!(bounty.current, 2);
        assert!(record_bounty_progress(&mut bounty, 2));
        assert_eq!(bounty.current, 3);
        assert!(bounty.completed);
        assert!(!record_bounty_progress(&mut bounty, 1));
        assert_eq!(bounty.current, 3);
    }

    #[test]
    fn bounty_reward_is_claimed_once() {
        let mut bounty = BountyProgress {
            current: 3,
            target: 3,
            reward_gold: 95,
            reward_shards: 2,
            reward_essence: 1,
            completed: true,
            ..default()
        };
        let mut stats = RunStats::default();

        assert!(claim_bounty_reward(&mut bounty, &mut stats));
        assert!(!claim_bounty_reward(&mut bounty, &mut stats));
        assert_eq!(stats.gold, 95);
        assert_eq!(stats.ember_shards, 2);
        assert_eq!(stats.affix_essence, 1);
    }

    #[test]
    fn bounty_summary_reports_claimed_state() {
        let mut bounty = BountyProgress::default();
        assert!(bounty_summary(&bounty).contains("8"));
        assert!(bounty_summary(&bounty).contains("essence"));
        bounty.completed = true;
        bounty.reward_claimed = true;
        assert!(bounty_summary(&bounty).contains("complete"));
    }

    #[test]
    fn bounty_config_scales_objective_by_difficulty() {
        let normal = bounty_for_difficulty(Difficulty::Normal);
        let nightmare = bounty_for_difficulty(Difficulty::Nightmare);
        let hell = bounty_for_difficulty(Difficulty::Hell);
        let torment = bounty_for_difficulty(Difficulty::Torment);

        assert_eq!(normal.kind, BountyKind::SlayEnemies);
        assert_eq!(nightmare.kind, BountyKind::RecoverLorePages);
        assert_eq!(hell.target, 3);
        assert_eq!(torment.kind, BountyKind::RecoverLorePages);
        assert_eq!(torment.target, 3);
        assert!(hell.reward_shards > nightmare.reward_shards);
        assert!(hell.reward_essence > nightmare.reward_essence);
        assert!(torment.reward_gold > hell.reward_gold);
        assert!(torment.reward_essence > hell.reward_essence);
        assert!(nightmare.reward_gold > normal.reward_gold);
    }

    #[test]
    fn bounty_board_rotates_objectives_after_repeated_clears() {
        let first_normal = bounty_for_run(Difficulty::Normal, 0);
        let second_normal = bounty_for_run(Difficulty::Normal, 1);
        let third_normal = bounty_for_run(Difficulty::Normal, 2);
        let fourth_normal = bounty_for_run(Difficulty::Normal, 3);
        let fifth_normal = bounty_for_run(Difficulty::Normal, 4);
        let sixth_normal = bounty_for_run(Difficulty::Normal, 5);
        let wrapped_normal = bounty_for_run(Difficulty::Normal, 6);

        assert_eq!(first_normal.kind, BountyKind::SlayEnemies);
        assert_eq!(second_normal.kind, BountyKind::SmashBreakables);
        assert_eq!(second_normal.target, BREAKER_TARGET_BREAKABLES);
        assert_eq!(third_normal.kind, BountyKind::HuntChampionPack);
        assert_eq!(third_normal.target, CHAMPION_PACK_TARGET);
        assert_eq!(fourth_normal.kind, BountyKind::RecoverLorePages);
        assert_eq!(fifth_normal.kind, BountyKind::OpenTreasureVault);
        assert_eq!(fifth_normal.target, 1);
        assert_eq!(sixth_normal.kind, BountyKind::SlayAffixedElites);
        assert_eq!(sixth_normal.target, 2);
        assert_eq!(wrapped_normal.kind, BountyKind::SlayEnemies);

        let torment_followup = bounty_for_run(Difficulty::Torment, 1);
        assert_eq!(torment_followup.kind, BountyKind::SlayAffixedElites);
        assert!(torment_followup.reward_essence > third_normal.reward_essence);

        let torment_champion = bounty_for_run(Difficulty::Torment, 2);
        assert_eq!(torment_champion.kind, BountyKind::HuntChampionPack);

        let torment_vault = bounty_for_run(Difficulty::Torment, 3);
        assert_eq!(torment_vault.kind, BountyKind::OpenTreasureVault);
        assert_eq!(torment_vault.target, 1);
        assert!(torment_vault.reward_gold > fifth_normal.reward_gold);
    }

    #[test]
    fn affixed_elite_bounty_scales_targets_and_summary_by_difficulty() {
        let normal = bounty_for_kind(Difficulty::Normal, BountyKind::SlayAffixedElites);
        let torment = bounty_for_kind(Difficulty::Torment, BountyKind::SlayAffixedElites);

        assert_eq!(normal.target, 2);
        assert_eq!(torment.target, 5);
        assert_eq!(normal.kind.objective_label(), "Slay affixed elites");
        assert!(torment.reward_gold > normal.reward_gold);
        assert!(torment.reward_essence > normal.reward_essence);
        assert!(bounty_summary(&torment).contains("Slay affixed elites"));
    }

    #[test]
    fn affixed_elite_bounty_counts_only_multi_affix_kills() {
        assert_eq!(affixed_elite_bounty_credit(0), 0);
        assert_eq!(affixed_elite_bounty_credit(1), 0);
        assert_eq!(affixed_elite_bounty_credit(2), 1);
        assert_eq!(affixed_elite_bounty_credit(3), 1);
    }
}
