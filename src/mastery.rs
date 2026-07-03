use crate::{
    BOSS_BREAK_TARGET, GameState, RunStats,
    bounty::BountyProgress,
    challenge::{ChallengeId, ChapterChallenges},
    feedback::CombatEvent,
    lore::{LORE_ENTRY_COUNT, LoreJournal},
    player::{LegendaryCodex, Player},
};
use bevy::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Default, Eq, PartialEq, Serialize, Deserialize)]
pub enum MasteryRank {
    #[default]
    Initiate,
    Adept,
    Master,
    Conqueror,
    Paragon,
}

impl MasteryRank {
    pub fn label(self) -> &'static str {
        match self {
            Self::Initiate => "Initiate",
            Self::Adept => "Adept",
            Self::Master => "Master",
            Self::Conqueror => "Conqueror",
            Self::Paragon => "Paragon",
        }
    }

    pub fn reward(self) -> MasteryReward {
        match self {
            Self::Initiate => MasteryReward {
                gold: 50,
                ember_shards: 1,
                affix_essence: 0,
            },
            Self::Adept => MasteryReward {
                gold: 140,
                ember_shards: 4,
                affix_essence: 2,
            },
            Self::Master => MasteryReward {
                gold: 260,
                ember_shards: 8,
                affix_essence: 5,
            },
            Self::Conqueror => MasteryReward {
                gold: 420,
                ember_shards: 12,
                affix_essence: 8,
            },
            Self::Paragon => MasteryReward {
                gold: 620,
                ember_shards: 18,
                affix_essence: 14,
            },
        }
    }
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub struct MasteryReward {
    pub gold: u32,
    pub ember_shards: u32,
    pub affix_essence: u32,
}

#[derive(Resource, Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct ChapterMastery {
    pub points: u32,
    pub rank: MasteryRank,
    pub reward_claimed: bool,
    pub reward_gold: u32,
    pub reward_shards: u32,
    #[serde(default)]
    pub reward_essence: u32,
    #[serde(default)]
    pub boss_bonus_points: u32,
    pub latest: String,
}

impl ChapterMastery {
    pub fn reset(&mut self) {
        *self = Self::default();
    }
}

pub fn rank_for_points(points: u32) -> MasteryRank {
    match points {
        0..=2 => MasteryRank::Initiate,
        3..=5 => MasteryRank::Adept,
        6..=9 => MasteryRank::Master,
        10..=14 => MasteryRank::Conqueror,
        _ => MasteryRank::Paragon,
    }
}

pub fn mastery_summary(mastery: &ChapterMastery) -> String {
    if mastery.reward_claimed {
        format!(
            "Mastery: {} ({} pts, boss +{}, +{} gold +{} shards +{} essence)",
            mastery.rank.label(),
            mastery.points,
            mastery.boss_bonus_points,
            mastery.reward_gold,
            mastery.reward_shards,
            mastery.reward_essence
        )
    } else {
        format!("Mastery: {} ({} pts)", mastery.rank.label(), mastery.points)
    }
}

pub fn mastery_next_goal_summary(
    stats: &RunStats,
    bounty: &BountyProgress,
    challenges: &ChapterChallenges,
    journal: &LoreJournal,
    codex: &LegendaryCodex,
) -> String {
    let points = calculate_mastery_points(stats, bounty, challenges, journal, codex);
    let rank_goal = next_mastery_rank_goal(points)
        .map(|(rank, threshold)| format!("next {} at {} pts", rank.label(), threshold))
        .unwrap_or_else(|| "Paragon rank reached".to_string());

    if let Some(challenge) = first_available_mastery_challenge(challenges, stats.elapsed_secs) {
        return format!(
            "Mastery next: {} - {} | {}",
            challenge.id.label(),
            challenge.id.description(),
            rank_goal
        );
    }
    if !bounty.completed {
        return format!("Mastery next: complete the bounty | {rank_goal}");
    }
    if journal.entries.len() < LORE_ENTRY_COUNT {
        return format!(
            "Mastery next: recover lore pages {}/{} | {}",
            journal.entries.len(),
            LORE_ENTRY_COUNT,
            rank_goal
        );
    }
    if stats.best_valor_stacks < 5 {
        return format!(
            "Mastery next: reach five Valor stacks ({}/5) | {}",
            stats.best_valor_stacks, rank_goal
        );
    }
    if stats.boss_staggers < BOSS_BREAK_TARGET {
        return format!(
            "Mastery next: break Malrec {}/{} times | {}",
            stats.boss_staggers, BOSS_BREAK_TARGET, rank_goal
        );
    }
    if stats.kills == 0 {
        return format!("Mastery next: defeat Malrec before Ashen Enrage | {rank_goal}");
    }
    if stats.boss_enraged {
        return format!("Mastery next: future run defeat Malrec before Ashen Enrage | {rank_goal}");
    }
    if let Some(power) = codex.next_missing_power() {
        return format!(
            "Mastery next: unlock {} in the codex | {}",
            power.label(),
            rank_goal
        );
    }

    "Mastery complete: all chapter mastery point sources secured".to_string()
}

fn next_mastery_rank_goal(points: u32) -> Option<(MasteryRank, u32)> {
    match points {
        0..=2 => Some((MasteryRank::Adept, 3)),
        3..=5 => Some((MasteryRank::Master, 6)),
        6..=9 => Some((MasteryRank::Conqueror, 10)),
        10..=14 => Some((MasteryRank::Paragon, 15)),
        _ => None,
    }
}

fn first_available_mastery_challenge(
    challenges: &ChapterChallenges,
    elapsed_secs: f32,
) -> Option<&crate::challenge::ChallengeProgress> {
    challenges.challenges.iter().find(|challenge| {
        !challenge.completed && (challenge.id != ChallengeId::SwiftClear || elapsed_secs <= 360.0)
    })
}

fn completed_challenge_points(challenges: &ChapterChallenges, elapsed_secs: f32) -> u32 {
    let mut points = challenges
        .challenges
        .iter()
        .filter(|challenge| challenge.completed)
        .count() as u32;
    let swift_already_counted = challenges
        .challenges
        .iter()
        .any(|challenge| challenge.id == ChallengeId::SwiftClear && challenge.completed);
    if elapsed_secs <= 360.0 && !swift_already_counted {
        points += 1;
    }
    points
}

pub fn calculate_mastery_points(
    stats: &RunStats,
    bounty: &BountyProgress,
    challenges: &ChapterChallenges,
    journal: &LoreJournal,
    codex: &LegendaryCodex,
) -> u32 {
    let mut points = completed_challenge_points(challenges, stats.elapsed_secs);
    if bounty.completed {
        points += 1;
    }
    if journal.entries.len() >= 3 {
        points += 1;
    }
    if stats.best_valor_stacks >= 5 {
        points += 1;
    }
    points + boss_mastery_points(stats) + codex.unlocked.len() as u32
}

fn boss_mastery_points(stats: &RunStats) -> u32 {
    u32::from(stats.boss_staggers >= BOSS_BREAK_TARGET)
        + u32::from(stats.kills > 0 && !stats.boss_enraged)
}

pub struct MasteryPlugin;

impl Plugin for MasteryPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<ChapterMastery>()
            .add_systems(OnEnter(GameState::InGame), reset_mastery)
            .add_systems(OnEnter(GameState::Victory), award_mastery);
    }
}

fn reset_mastery(mut mastery: ResMut<ChapterMastery>) {
    mastery.reset();
}

fn award_mastery(
    mut stats: ResMut<RunStats>,
    bounty: Res<BountyProgress>,
    challenges: Res<ChapterChallenges>,
    journal: Res<LoreJournal>,
    codex: Query<&LegendaryCodex, With<Player>>,
    mut mastery: ResMut<ChapterMastery>,
    mut events: MessageWriter<CombatEvent>,
) {
    if mastery.reward_claimed {
        return;
    }
    let Ok(codex) = codex.single() else {
        return;
    };
    let points = calculate_mastery_points(&stats, &bounty, &challenges, &journal, codex);
    let rank = rank_for_points(points);
    let reward = rank.reward();

    mastery.points = points;
    mastery.rank = rank;
    mastery.reward_claimed = true;
    mastery.reward_gold = reward.gold;
    mastery.reward_shards = reward.ember_shards;
    mastery.reward_essence = reward.affix_essence;
    mastery.boss_bonus_points = boss_mastery_points(&stats);
    mastery.latest = format!(
        "Chapter mastery: {} (boss +{})",
        rank.label(),
        mastery.boss_bonus_points
    );
    stats.gold += reward.gold;
    stats.ember_shards += reward.ember_shards;
    stats.affix_essence += reward.affix_essence;
    events.write(CombatEvent {
        text: format!(
            "Chapter mastery {}: {} gold, {} shards, {} essence",
            rank.label(),
            reward.gold,
            reward.ember_shards,
            reward.affix_essence
        ),
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        bounty::{BountyKind, BountyProgress},
        challenge::ChallengeProgress,
        lore::LoreEntryId,
        player::LegendaryPower,
    };

    #[test]
    fn mastery_rank_scales_with_points() {
        assert_eq!(rank_for_points(0), MasteryRank::Initiate);
        assert_eq!(rank_for_points(3), MasteryRank::Adept);
        assert_eq!(rank_for_points(6), MasteryRank::Master);
        assert_eq!(rank_for_points(10), MasteryRank::Conqueror);
        assert_eq!(rank_for_points(15), MasteryRank::Paragon);
    }

    #[test]
    fn mastery_rewards_include_endgame_essence() {
        assert_eq!(MasteryRank::Initiate.reward().affix_essence, 0);
        assert_eq!(MasteryRank::Adept.reward().affix_essence, 2);
        assert_eq!(MasteryRank::Master.reward().affix_essence, 5);
        assert_eq!(MasteryRank::Conqueror.reward().affix_essence, 8);
        assert_eq!(MasteryRank::Paragon.reward().affix_essence, 14);

        let mastery = ChapterMastery {
            points: 15,
            rank: MasteryRank::Paragon,
            reward_claimed: true,
            reward_gold: 620,
            reward_shards: 18,
            reward_essence: 14,
            boss_bonus_points: 2,
            latest: "Chapter mastery: Paragon".to_string(),
        };

        assert!(mastery_summary(&mastery).contains("Paragon"));
        assert!(mastery_summary(&mastery).contains("boss +2"));
        assert!(mastery_summary(&mastery).contains("+14 essence"));
    }

    #[test]
    fn mastery_points_count_challenges_bounty_lore_and_codex() {
        let stats = RunStats {
            elapsed_secs: 320.0,
            best_valor_stacks: 5,
            ..RunStats::default()
        };
        let bounty = BountyProgress {
            kind: BountyKind::RecoverLorePages,
            completed: true,
            ..BountyProgress::default()
        };
        let mut challenges = ChapterChallenges::new();
        challenges.restore(vec![ChallengeProgress {
            id: ChallengeId::TreasureHunter,
            completed: true,
            reward_claimed: false,
        }]);
        let journal = LoreJournal {
            entries: vec![
                LoreEntryId::StewardWarning,
                LoreEntryId::AcolyteOath,
                LoreEntryId::MalrecPact,
            ],
            latest: String::new(),
        };
        let mut codex = LegendaryCodex::default();
        codex.unlock(LegendaryPower::Emberbrand);
        codex.unlock(LegendaryPower::Frostbrand);

        assert_eq!(
            calculate_mastery_points(&stats, &bounty, &challenges, &journal, &codex),
            7
        );
    }

    #[test]
    fn mastery_requires_full_valor_stack_for_bonus_point() {
        let low_valor = RunStats {
            elapsed_secs: 500.0,
            best_valor_stacks: 4,
            ..RunStats::default()
        };
        let full_valor = RunStats {
            elapsed_secs: 500.0,
            best_valor_stacks: 5,
            ..RunStats::default()
        };
        let bounty = BountyProgress::default();
        let challenges = ChapterChallenges::new();
        let journal = LoreJournal::default();
        let codex = LegendaryCodex::default();

        assert_eq!(
            calculate_mastery_points(&low_valor, &bounty, &challenges, &journal, &codex),
            0
        );
        assert_eq!(
            calculate_mastery_points(&full_valor, &bounty, &challenges, &journal, &codex),
            1
        );
    }

    #[test]
    fn mastery_awards_boss_performance_bonus_points() {
        let empty_stats = RunStats::default();
        let clean_boss = RunStats {
            kills: 9,
            boss_staggers: BOSS_BREAK_TARGET,
            boss_enraged: false,
            ..RunStats::default()
        };
        let enraged_boss = RunStats {
            kills: 9,
            boss_staggers: BOSS_BREAK_TARGET,
            boss_enraged: true,
            ..RunStats::default()
        };

        assert_eq!(boss_mastery_points(&empty_stats), 0);
        assert_eq!(boss_mastery_points(&clean_boss), 2);
        assert_eq!(boss_mastery_points(&enraged_boss), 1);
    }

    #[test]
    fn mastery_next_goal_guides_available_point_sources() {
        let stats = RunStats {
            elapsed_secs: 420.0,
            ..RunStats::default()
        };
        let bounty = BountyProgress::default();
        let challenges = ChapterChallenges::new();
        let journal = LoreJournal::default();
        let codex = LegendaryCodex::default();

        let summary = mastery_next_goal_summary(&stats, &bounty, &challenges, &journal, &codex);

        assert!(summary.contains("Treasure Hunter"));
        assert!(summary.contains("next Adept at 3 pts"));
        assert!(!summary.contains("Swift Clear"));
    }

    #[test]
    fn mastery_next_goal_falls_through_to_run_and_codex_goals() {
        let mut challenges = ChapterChallenges::new();
        for challenge in &mut challenges.challenges {
            challenge.completed = true;
        }
        let bounty = BountyProgress {
            completed: true,
            ..BountyProgress::default()
        };
        let mut stats = RunStats {
            elapsed_secs: 420.0,
            ..RunStats::default()
        };
        let mut journal = LoreJournal::default();
        let mut codex = LegendaryCodex::default();

        assert!(
            mastery_next_goal_summary(&stats, &bounty, &challenges, &journal, &codex)
                .contains("recover lore pages 0/3")
        );

        journal.entries = vec![
            LoreEntryId::StewardWarning,
            LoreEntryId::AcolyteOath,
            LoreEntryId::MalrecPact,
        ];
        assert!(
            mastery_next_goal_summary(&stats, &bounty, &challenges, &journal, &codex)
                .contains("five Valor stacks")
        );

        stats.best_valor_stacks = 5;
        assert!(
            mastery_next_goal_summary(&stats, &bounty, &challenges, &journal, &codex)
                .contains("break Malrec 0/2")
        );

        stats.boss_staggers = BOSS_BREAK_TARGET;
        assert!(
            mastery_next_goal_summary(&stats, &bounty, &challenges, &journal, &codex)
                .contains("defeat Malrec before Ashen Enrage")
        );

        stats.kills = 12;
        for power in [
            LegendaryPower::Emberbrand,
            LegendaryPower::Frostbrand,
            LegendaryPower::Stormbrand,
            LegendaryPower::Soulreaver,
        ] {
            codex.unlock(power);
        }
        assert!(
            mastery_next_goal_summary(&stats, &bounty, &challenges, &journal, &codex)
                .contains("unlock Aegisbrand")
        );

        codex.unlock(LegendaryPower::Aegisbrand);
        assert_eq!(
            mastery_next_goal_summary(&stats, &bounty, &challenges, &journal, &codex),
            "Mastery complete: all chapter mastery point sources secured"
        );
    }
}
