use crate::{GameState, RunStats, enemy::EnemyKilled, feedback::CombatEvent, not_paused};
use bevy::prelude::*;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct BestiaryEntry {
    pub display_name: String,
    pub kills: u32,
    #[serde(default)]
    pub claimed_trophy_kills: u32,
}

#[derive(Resource, Debug, Default, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Bestiary {
    pub entries: BTreeMap<String, BestiaryEntry>,
    pub latest: String,
}

pub const CHAPTER_BESTIARY_ENEMY_IDS: [&str; 8] = [
    "skeleton",
    "bone_stalker",
    "cultist",
    "ashen_marksman",
    "reliquary_brute",
    "treasure_imp",
    "keeper",
    "nemesis",
];

impl Bestiary {
    pub fn record_kill(
        &mut self,
        enemy_id: &str,
        display_name: &str,
    ) -> Option<BestiaryTrophyReward> {
        let entry = self
            .entries
            .entry(enemy_id.to_string())
            .or_insert_with(|| BestiaryEntry {
                display_name: display_name.to_string(),
                kills: 0,
                claimed_trophy_kills: 0,
            });
        entry.display_name = display_name.to_string();
        entry.kills += 1;
        self.latest = format!("{} x{}", entry.display_name, entry.kills);
        claim_bestiary_trophy(entry)
    }

    pub fn restore(&mut self, entries: BTreeMap<String, BestiaryEntry>) {
        self.entries = entries;
        self.latest = self
            .entries
            .values()
            .next_back()
            .map(|entry| format!("{} x{}", entry.display_name, entry.kills))
            .unwrap_or_default();
    }

    pub fn kills_for(&self, enemy_id: &str) -> u32 {
        self.entries
            .get(enemy_id)
            .map(|entry| entry.kills)
            .unwrap_or_default()
    }
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub struct BestiaryTrophyReward {
    pub threshold: u32,
    pub gold: u32,
    pub ember_shards: u32,
    pub affix_essence: u32,
}

pub fn chapter_bestiary_complete(bestiary: &Bestiary) -> bool {
    CHAPTER_BESTIARY_ENEMY_IDS
        .iter()
        .all(|enemy_id| bestiary.entries.contains_key(*enemy_id))
}
pub fn bestiary_damage_multiplier(bestiary: &Bestiary, enemy_id: &str) -> f32 {
    1.0 + bestiary_damage_bonus_for_kills(bestiary.kills_for(enemy_id))
}
pub fn bestiary_damage_bonus_for_kills(kills: u32) -> f32 {
    match kills {
        15.. => 0.12,
        8.. => 0.08,
        3.. => 0.04,
        _ => 0.0,
    }
}

pub fn bestiary_trophy_reward_for_threshold(threshold: u32) -> Option<BestiaryTrophyReward> {
    match threshold {
        3 => Some(BestiaryTrophyReward {
            threshold,
            gold: 25,
            ember_shards: 0,
            affix_essence: 0,
        }),
        8 => Some(BestiaryTrophyReward {
            threshold,
            gold: 60,
            ember_shards: 1,
            affix_essence: 0,
        }),
        15 => Some(BestiaryTrophyReward {
            threshold,
            gold: 120,
            ember_shards: 2,
            affix_essence: 1,
        }),
        _ => None,
    }
}

fn claim_bestiary_trophy(entry: &mut BestiaryEntry) -> Option<BestiaryTrophyReward> {
    for threshold in [3, 8, 15] {
        if entry.kills >= threshold && entry.claimed_trophy_kills < threshold {
            entry.claimed_trophy_kills = threshold;
            return bestiary_trophy_reward_for_threshold(threshold);
        }
    }
    None
}
pub struct BestiaryPlugin;

impl Plugin for BestiaryPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Bestiary>()
            .add_systems(OnEnter(GameState::InGame), reset_bestiary)
            .add_systems(
                Update,
                record_bestiary_kills.run_if(in_state(GameState::InGame).and_then(not_paused)),
            );
    }
}

fn reset_bestiary(mut bestiary: ResMut<Bestiary>) {
    *bestiary = Bestiary::default();
}

fn record_bestiary_kills(
    mut killed: MessageReader<EnemyKilled>,
    mut bestiary: ResMut<Bestiary>,
    mut stats: ResMut<RunStats>,
    mut combat_events: MessageWriter<CombatEvent>,
) {
    for event in killed.read() {
        if let Some(reward) = bestiary.record_kill(&event.enemy_id, &event.display_name) {
            stats.gold = stats.gold.saturating_add(reward.gold);
            stats.ember_shards = stats.ember_shards.saturating_add(reward.ember_shards);
            stats.affix_essence = stats.affix_essence.saturating_add(reward.affix_essence);
            combat_events.write(CombatEvent {
                text: format!(
                    "Bestiary trophy {} x{}: +{} gold +{} shards +{} essence",
                    event.display_name,
                    reward.threshold,
                    reward.gold,
                    reward.ember_shards,
                    reward.affix_essence
                ),
            });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bestiary_records_kills_and_latest_entry() {
        let mut bestiary = Bestiary::default();

        bestiary.record_kill("skeleton", "Ashbone Guard");
        bestiary.record_kill("skeleton", "Ashbone Guard");
        bestiary.record_kill("cultist", "Cinder Acolyte");

        assert_eq!(bestiary.entries["skeleton"].kills, 2);
        assert!(bestiary.latest.contains("Cinder Acolyte"));
    }

    #[test]
    fn bestiary_damage_bonus_scales_by_kill_thresholds() {
        let mut bestiary = Bestiary::default();
        for _ in 0..8 {
            bestiary.record_kill("skeleton", "Ashbone Guard");
        }

        assert_eq!(bestiary_damage_bonus_for_kills(0), 0.0);
        assert_eq!(bestiary_damage_bonus_for_kills(3), 0.04);
        assert_eq!(bestiary_damage_bonus_for_kills(8), 0.08);
        assert_eq!(bestiary_damage_bonus_for_kills(15), 0.12);
        assert_eq!(bestiary_damage_multiplier(&bestiary, "skeleton"), 1.08);
        assert_eq!(bestiary_damage_multiplier(&bestiary, "cultist"), 1.0);
    }

    #[test]
    fn bestiary_trophies_pay_once_at_kill_thresholds() {
        let mut bestiary = Bestiary::default();

        assert_eq!(bestiary.record_kill("skeleton", "Ashbone Guard"), None);
        assert_eq!(bestiary.record_kill("skeleton", "Ashbone Guard"), None);
        assert_eq!(
            bestiary.record_kill("skeleton", "Ashbone Guard"),
            bestiary_trophy_reward_for_threshold(3)
        );
        assert_eq!(bestiary.entries["skeleton"].claimed_trophy_kills, 3);

        for _ in 3..7 {
            assert_eq!(bestiary.record_kill("skeleton", "Ashbone Guard"), None);
        }
        assert_eq!(
            bestiary.record_kill("skeleton", "Ashbone Guard"),
            bestiary_trophy_reward_for_threshold(8)
        );

        for _ in 8..14 {
            assert_eq!(bestiary.record_kill("skeleton", "Ashbone Guard"), None);
        }
        assert_eq!(
            bestiary.record_kill("skeleton", "Ashbone Guard"),
            bestiary_trophy_reward_for_threshold(15)
        );
        assert_eq!(bestiary.record_kill("skeleton", "Ashbone Guard"), None);
    }

    #[test]
    fn bestiary_restore_rebuilds_latest() {
        let mut restored = BTreeMap::new();
        restored.insert(
            "keeper".to_string(),
            BestiaryEntry {
                display_name: "Malrec, Keeper of Ash".to_string(),
                kills: 1,
                claimed_trophy_kills: 0,
            },
        );
        let mut bestiary = Bestiary::default();

        bestiary.restore(restored);

        assert_eq!(bestiary.entries.len(), 1);
        assert!(bestiary.latest.contains("Malrec"));
    }

    #[test]
    fn chapter_bestiary_requires_every_chapter_enemy_type() {
        let mut bestiary = Bestiary::default();

        for enemy_id in CHAPTER_BESTIARY_ENEMY_IDS {
            bestiary.record_kill(enemy_id, enemy_id);
        }

        assert!(chapter_bestiary_complete(&bestiary));
        bestiary.entries.remove("reliquary_brute");
        assert!(!chapter_bestiary_complete(&bestiary));
        bestiary.record_kill("reliquary_brute", "Reliquary Bonebreaker");
        bestiary.entries.remove("nemesis");
        assert!(!chapter_bestiary_complete(&bestiary));
    }
}
