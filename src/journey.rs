use crate::{
    RunStats,
    challenge::{ChallengeId, ChapterChallenges},
    mastery::ChapterMastery,
    milestone::{ALL_MILESTONES, MilestoneId, Milestones},
};

const MASTERY_SCORE_CAP: u32 = 15;
const JOURNEY_TIERS: [(ChapterJourneyTier, u32); 5] = [
    (ChapterJourneyTier::Initiate, 0),
    (ChapterJourneyTier::Sealbearer, 12),
    (ChapterJourneyTier::RiftHunter, 28),
    (ChapterJourneyTier::AshenChampion, 48),
    (ChapterJourneyTier::TormentConqueror, 72),
];

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum ChapterJourneyTier {
    Initiate,
    Sealbearer,
    RiftHunter,
    AshenChampion,
    TormentConqueror,
}

impl ChapterJourneyTier {
    pub fn label(self) -> &'static str {
        match self {
            Self::Initiate => "Initiate",
            Self::Sealbearer => "Sealbearer",
            Self::RiftHunter => "Rift Hunter",
            Self::AshenChampion => "Ashen Champion",
            Self::TormentConqueror => "Torment Conqueror",
        }
    }

    pub fn reward(self) -> ChapterJourneyReward {
        match self {
            Self::Initiate => ChapterJourneyReward {
                gold: 40,
                ember_shards: 1,
                affix_essence: 0,
            },
            Self::Sealbearer => ChapterJourneyReward {
                gold: 140,
                ember_shards: 4,
                affix_essence: 2,
            },
            Self::RiftHunter => ChapterJourneyReward {
                gold: 260,
                ember_shards: 8,
                affix_essence: 5,
            },
            Self::AshenChampion => ChapterJourneyReward {
                gold: 430,
                ember_shards: 13,
                affix_essence: 9,
            },
            Self::TormentConqueror => ChapterJourneyReward {
                gold: 700,
                ember_shards: 20,
                affix_essence: 16,
            },
        }
    }
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub struct ChapterJourneyReward {
    pub gold: u32,
    pub ember_shards: u32,
    pub affix_essence: u32,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ChapterJourneySnapshot {
    pub tier: ChapterJourneyTier,
    pub score: u32,
    pub max_score: u32,
    pub percent: u32,
    pub next_tier: Option<(ChapterJourneyTier, u32)>,
    pub next_goal: String,
}

pub fn chapter_journey_snapshot(
    challenges: &ChapterChallenges,
    milestones: &Milestones,
    mastery: &ChapterMastery,
) -> ChapterJourneySnapshot {
    let score = journey_score(challenges, milestones, mastery);
    let max_score =
        challenges.challenges.len() as u32 + ALL_MILESTONES.len() as u32 + MASTERY_SCORE_CAP;
    ChapterJourneySnapshot {
        tier: journey_tier_for_score(score),
        score,
        max_score,
        percent: ((score as f32 / max_score as f32) * 100.0).round() as u32,
        next_tier: next_tier(score),
        next_goal: next_journey_goal(challenges, milestones),
    }
}

pub fn journey_summary(
    challenges: &ChapterChallenges,
    milestones: &Milestones,
    mastery: &ChapterMastery,
) -> String {
    let snapshot = chapter_journey_snapshot(challenges, milestones, mastery);
    let next_tier = journey_next_tier_reward_summary(&snapshot);
    format!(
        "Journey: {} {}/{} ({}%) | {} | {}",
        snapshot.tier.label(),
        snapshot.score,
        snapshot.max_score,
        snapshot.percent,
        next_tier,
        snapshot.next_goal
    )
}

pub fn journey_next_tier_reward_summary(snapshot: &ChapterJourneySnapshot) -> String {
    snapshot
        .next_tier
        .map(|(tier, threshold)| {
            let reward = tier.reward();
            format!(
                "next {} at {} (+{}g +{}s +{}e)",
                tier.label(),
                threshold,
                reward.gold,
                reward.ember_shards,
                reward.affix_essence
            )
        })
        .unwrap_or_else(|| "journey tiers complete".to_string())
}

pub fn award_chapter_journey_reward(
    stats: &mut RunStats,
    challenges: &ChapterChallenges,
    milestones: &Milestones,
    mastery: &ChapterMastery,
) -> ChapterJourneyReward {
    if stats.journey_reward_claimed {
        return ChapterJourneyReward {
            gold: 0,
            ember_shards: 0,
            affix_essence: 0,
        };
    }
    let snapshot = chapter_journey_snapshot(challenges, milestones, mastery);
    let reward = snapshot.tier.reward();
    stats.journey_score = snapshot.score;
    stats.journey_bonus_gold = reward.gold;
    stats.journey_bonus_shards = reward.ember_shards;
    stats.journey_bonus_essence = reward.affix_essence;
    stats.gold += reward.gold;
    stats.ember_shards += reward.ember_shards;
    stats.affix_essence += reward.affix_essence;
    stats.journey_reward_claimed = true;
    reward
}

fn journey_score(
    challenges: &ChapterChallenges,
    milestones: &Milestones,
    mastery: &ChapterMastery,
) -> u32 {
    let completed_challenges = challenges
        .challenges
        .iter()
        .filter(|challenge| challenge.completed)
        .count() as u32;
    completed_challenges + milestones.unlocked.len() as u32 + mastery.points.min(MASTERY_SCORE_CAP)
}

pub fn journey_tier_for_score(score: u32) -> ChapterJourneyTier {
    JOURNEY_TIERS
        .iter()
        .rev()
        .find(|(_, threshold)| score >= *threshold)
        .map(|(tier, _)| *tier)
        .unwrap_or(ChapterJourneyTier::Initiate)
}

fn next_tier(score: u32) -> Option<(ChapterJourneyTier, u32)> {
    JOURNEY_TIERS
        .iter()
        .find(|(_, threshold)| score < *threshold)
        .copied()
}

fn next_journey_goal(challenges: &ChapterChallenges, milestones: &Milestones) -> String {
    let challenge_goal = priority_challenges()
        .into_iter()
        .find(|id| {
            challenges
                .challenges
                .iter()
                .any(|challenge| challenge.id == *id && !challenge.completed)
        })
        .map(|id| format!("pursue {}: {}", id.label(), id.description()));
    if let Some(goal) = challenge_goal {
        return goal;
    }

    priority_milestones()
        .into_iter()
        .find(|id| !milestones.unlocked.contains(id))
        .map(|id| format!("pursue {}: {}", id.title(), id.summary()))
        .unwrap_or_else(|| "all chapter journey goals complete".to_string())
}

fn priority_challenges() -> [ChallengeId; 32] {
    [
        ChallengeId::SwiftClear,
        ChallengeId::TreasureHunter,
        ChallengeId::TreasureVault,
        ChallengeId::CarnageMaster,
        ChallengeId::Vaultbreaker,
        ChallengeId::Ritekeeper,
        ChallengeId::Riftwalker,
        ChallengeId::Sealbreaker,
        ChallengeId::Lorekeeper,
        ChallengeId::CodexAdept,
        ChallengeId::SetAdept,
        ChallengeId::BountyHunter,
        ChallengeId::TalentAdept,
        ChallengeId::AshenAlchemy,
        ChallengeId::LootCurator,
        ChallengeId::LastStand,
        ChallengeId::SealWarden,
        ChallengeId::ChampionBreaker,
        ChallengeId::AffixCodex,
        ChallengeId::NemesisHunter,
        ChallengeId::GemAdept,
        ChallengeId::RuneWeaver,
        ChallengeId::ArmoryAdept,
        ChallengeId::PortalWayfarer,
        ChallengeId::GlorySeeker,
        ChallengeId::BossBreaker,
        ChallengeId::SoulSigil,
        ChallengeId::TrophyCache,
        ChallengeId::EnrageDenied,
        ChallengeId::TormentClear,
        ChallengeId::PrimalCache,
        ChallengeId::ReliquaryConquest,
    ]
}

fn priority_milestones() -> [MilestoneId; 24] {
    [
        MilestoneId::CacheOpened,
        MilestoneId::VaultOpened,
        MilestoneId::BloodObelisk,
        MilestoneId::EmberRift,
        MilestoneId::ReliquarySeal,
        MilestoneId::TreasureFiend,
        MilestoneId::TreasureVault,
        MilestoneId::CarnageMaster,
        MilestoneId::Lorekeeper,
        MilestoneId::BountyCompleted,
        MilestoneId::LastStand,
        MilestoneId::SealWarden,
        MilestoneId::ChampionPack,
        MilestoneId::AffixCodex,
        MilestoneId::NemesisSlain,
        MilestoneId::SetAdept,
        MilestoneId::GemAdept,
        MilestoneId::BossBreaker,
        MilestoneId::SoulSigil,
        MilestoneId::TrophyCache,
        MilestoneId::PrimalCache,
        MilestoneId::AshenDuel,
        MilestoneId::BossSlain,
        MilestoneId::CodexArchivist,
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        challenge::{ChallengeProgress, ChapterChallenges},
        mastery::{ChapterMastery, MasteryRank},
    };

    #[test]
    fn journey_snapshot_combines_challenges_milestones_and_mastery() {
        let mut challenges = ChapterChallenges::new();
        challenges.restore(vec![
            ChallengeProgress {
                id: ChallengeId::SwiftClear,
                completed: true,
                reward_claimed: true,
            },
            ChallengeProgress {
                id: ChallengeId::Vaultbreaker,
                completed: true,
                reward_claimed: false,
            },
        ]);
        let mut milestones = Milestones::default();
        milestones.unlock(MilestoneId::CacheOpened);
        milestones.unlock(MilestoneId::VaultOpened);
        let mastery = ChapterMastery {
            points: 10,
            rank: MasteryRank::Conqueror,
            ..Default::default()
        };

        let snapshot = chapter_journey_snapshot(&challenges, &milestones, &mastery);

        assert_eq!(snapshot.score, 14);
        assert_eq!(snapshot.tier, ChapterJourneyTier::Sealbearer);
        assert_eq!(
            snapshot.next_tier,
            Some((ChapterJourneyTier::RiftHunter, 28))
        );
        assert!(snapshot.next_goal.contains("Treasure Hunter"));
    }

    #[test]
    fn journey_summary_surfaces_next_tier_reward_payoff() {
        let challenges = ChapterChallenges::new();
        let milestones = Milestones::default();
        let mastery = ChapterMastery::default();

        let summary = journey_summary(&challenges, &milestones, &mastery);

        assert!(summary.contains("Journey: Initiate"));
        assert!(summary.contains("next Sealbearer at 12 (+140g +4s +2e)"));
    }

    #[test]
    fn journey_next_goal_surfaces_treasure_fiend_milestone_after_hunt() {
        let mut challenges = ChapterChallenges::new();
        for id in [
            ChallengeId::SwiftClear,
            ChallengeId::TreasureHunter,
            ChallengeId::TreasureVault,
            ChallengeId::CarnageMaster,
            ChallengeId::Vaultbreaker,
            ChallengeId::Ritekeeper,
            ChallengeId::Riftwalker,
            ChallengeId::Sealbreaker,
            ChallengeId::Lorekeeper,
            ChallengeId::CodexAdept,
            ChallengeId::SetAdept,
            ChallengeId::BountyHunter,
            ChallengeId::TalentAdept,
            ChallengeId::AshenAlchemy,
            ChallengeId::LootCurator,
            ChallengeId::LastStand,
            ChallengeId::SealWarden,
            ChallengeId::ChampionBreaker,
            ChallengeId::AffixCodex,
            ChallengeId::NemesisHunter,
            ChallengeId::GemAdept,
            ChallengeId::RuneWeaver,
            ChallengeId::ArmoryAdept,
            ChallengeId::PortalWayfarer,
            ChallengeId::GlorySeeker,
            ChallengeId::BossBreaker,
            ChallengeId::SoulSigil,
            ChallengeId::TrophyCache,
            ChallengeId::EnrageDenied,
            ChallengeId::TormentClear,
            ChallengeId::PrimalCache,
            ChallengeId::ReliquaryConquest,
        ] {
            challenges.complete(id);
        }
        let mut milestones = Milestones::default();
        for id in [
            MilestoneId::CacheOpened,
            MilestoneId::VaultOpened,
            MilestoneId::BloodObelisk,
            MilestoneId::EmberRift,
            MilestoneId::ReliquarySeal,
        ] {
            milestones.unlock(id);
        }
        let mastery = ChapterMastery::default();

        let snapshot = chapter_journey_snapshot(&challenges, &milestones, &mastery);

        assert!(snapshot.next_goal.contains("Treasure Fiend"));
    }

    #[test]
    fn journey_summary_surfaces_final_tier_and_complete_goal() {
        let mut challenges = ChapterChallenges::new();
        for challenge in &mut challenges.challenges {
            challenge.completed = true;
        }
        let mut milestones = Milestones::default();
        milestones.restore(ALL_MILESTONES.to_vec());
        let mastery = ChapterMastery {
            points: 99,
            rank: MasteryRank::Paragon,
            ..Default::default()
        };

        let summary = journey_summary(&challenges, &milestones, &mastery);

        assert!(summary.contains("Torment Conqueror"));
        assert!(summary.contains("journey tiers complete"));
        assert!(summary.contains("all chapter journey goals complete"));
    }

    #[test]
    fn journey_next_tier_reward_summary_names_reward_payoff() {
        let challenges = ChapterChallenges::new();
        let milestones = Milestones::default();
        let mastery = ChapterMastery::default();

        let snapshot = chapter_journey_snapshot(&challenges, &milestones, &mastery);

        assert_eq!(
            journey_next_tier_reward_summary(&snapshot),
            "next Sealbearer at 12 (+140g +4s +2e)"
        );

        let complete_snapshot = ChapterJourneySnapshot {
            tier: ChapterJourneyTier::TormentConqueror,
            score: 99,
            max_score: 99,
            percent: 100,
            next_tier: None,
            next_goal: "all chapter journey goals complete".to_string(),
        };
        assert_eq!(
            journey_next_tier_reward_summary(&complete_snapshot),
            "journey tiers complete"
        );
    }

    #[test]
    fn journey_reward_is_claimed_once_and_scales_by_tier() {
        let mut challenges = ChapterChallenges::new();
        for id in [
            ChallengeId::SwiftClear,
            ChallengeId::Vaultbreaker,
            ChallengeId::Ritekeeper,
            ChallengeId::Riftwalker,
            ChallengeId::Sealbreaker,
            ChallengeId::Lorekeeper,
            ChallengeId::BountyHunter,
        ] {
            challenges.complete(id);
        }
        let mut milestones = Milestones::default();
        for id in [
            MilestoneId::CacheOpened,
            MilestoneId::VaultOpened,
            MilestoneId::BloodObelisk,
            MilestoneId::EmberRift,
            MilestoneId::ReliquarySeal,
            MilestoneId::BossSlain,
        ] {
            milestones.unlock(id);
        }
        let mastery = ChapterMastery {
            points: 20,
            rank: MasteryRank::Paragon,
            ..Default::default()
        };
        let mut stats = RunStats::default();

        let reward = award_chapter_journey_reward(&mut stats, &challenges, &milestones, &mastery);

        assert_eq!(reward, ChapterJourneyTier::RiftHunter.reward());
        assert_eq!(stats.journey_score, 28);
        assert_eq!(stats.gold, reward.gold);
        assert_eq!(stats.ember_shards, reward.ember_shards);
        assert_eq!(stats.affix_essence, reward.affix_essence);
        assert!(stats.journey_reward_claimed);
        assert_eq!(
            award_chapter_journey_reward(&mut stats, &challenges, &milestones, &mastery),
            ChapterJourneyReward {
                gold: 0,
                ember_shards: 0,
                affix_essence: 0
            }
        );
    }
}
