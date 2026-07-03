use crate::{
    GameState, RunStats,
    chapter::{InteractableKind, InteractableUsed},
    enemy::EnemyKilled,
    feedback::CombatEvent,
    not_paused,
    ordeal::ChapterModifier,
    player::{Fury, Player, RelicBuff, grant_reliquary_momentum},
};
use bevy::ecs::system::SystemParam;
use bevy::prelude::*;
use serde::{Deserialize, Serialize};

pub const OBELISK_TARGET_KILLS: u32 = 4;
const OBELISK_DURATION_SECS: f32 = 40.0;
const OBELISK_REWARD_GOLD: u32 = 95;
const OBELISK_REWARD_SHARDS: u32 = 5;
const OBELISK_REWARD_ESSENCE: u32 = 3;

#[derive(Debug, Clone, Copy, Default, Eq, PartialEq, Serialize, Deserialize)]
pub enum ObeliskState {
    #[default]
    Dormant,
    Active,
    Completed,
    Failed,
}

#[derive(Resource, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BloodObelisk {
    pub state: ObeliskState,
    pub kills: u32,
    pub target_kills: u32,
    pub remaining_secs: f32,
    pub reward_claimed: bool,
}

impl Default for BloodObelisk {
    fn default() -> Self {
        Self {
            state: ObeliskState::Dormant,
            kills: 0,
            target_kills: OBELISK_TARGET_KILLS,
            remaining_secs: 0.0,
            reward_claimed: false,
        }
    }
}

#[derive(Message, Debug, Clone, Copy)]
pub struct SpawnObeliskWave {
    pub origin: Vec3,
}

pub struct ObeliskPlugin;

impl Plugin for ObeliskPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<BloodObelisk>()
            .add_message::<SpawnObeliskWave>()
            .add_systems(OnEnter(GameState::InGame), reset_obelisk)
            .add_systems(
                Update,
                update_obelisk.run_if(in_state(GameState::InGame).and_then(not_paused)),
            );
    }
}

fn reset_obelisk(mut obelisk: ResMut<BloodObelisk>) {
    *obelisk = BloodObelisk::default();
}

#[derive(SystemParam)]
struct ObeliskResources<'w> {
    time: Res<'w, Time>,
    obelisk: ResMut<'w, BloodObelisk>,
    modifier: Res<'w, ChapterModifier>,
    stats: ResMut<'w, RunStats>,
}

#[derive(SystemParam)]
struct ObeliskEvents<'w, 's> {
    interactions: MessageReader<'w, 's, InteractableUsed>,
    enemy_killed: MessageReader<'w, 's, EnemyKilled>,
    wave_events: MessageWriter<'w, SpawnObeliskWave>,
    combat_events: MessageWriter<'w, CombatEvent>,
}

fn update_obelisk(
    mut resources: ObeliskResources,
    mut events: ObeliskEvents,
    mut player: Query<(&mut RelicBuff, &mut Fury), With<Player>>,
) {
    for event in events.interactions.read() {
        if event.kind == InteractableKind::BloodObelisk && activate_obelisk(&mut resources.obelisk)
        {
            events.wave_events.write(SpawnObeliskWave {
                origin: event.position,
            });
            events.combat_events.write(CombatEvent {
                text: "Blood obelisk awakened: feed it four kills".to_string(),
            });
        }
    }

    if resources.obelisk.state == ObeliskState::Active {
        for _ in events.enemy_killed.read() {
            record_obelisk_kill(&mut resources.obelisk);
        }
        if complete_obelisk_if_ready(
            &mut resources.obelisk,
            &mut resources.stats,
            *resources.modifier,
        ) {
            let momentum = player.single_mut().ok().map(|(mut relic, mut fury)| {
                grant_reliquary_momentum(&mut resources.stats, &mut relic, &mut fury, 1)
            });
            events.combat_events.write(CombatEvent {
                text: obelisk_completion_text(
                    resources.modifier.scale_reward(OBELISK_REWARD_GOLD),
                    OBELISK_REWARD_SHARDS,
                    OBELISK_REWARD_ESSENCE,
                    momentum.map(|reward| reward.stacks),
                ),
            });
            return;
        }
        let delta_secs = resources.time.delta_secs();
        tick_obelisk(&mut resources.obelisk, delta_secs);
        if resources.obelisk.state == ObeliskState::Failed {
            events.combat_events.write(CombatEvent {
                text: "Blood obelisk faded before it was fed".to_string(),
            });
        }
    }
}

fn obelisk_completion_text(
    gold: u32,
    shards: u32,
    essence: u32,
    momentum_stacks: Option<u32>,
) -> String {
    match momentum_stacks {
        Some(stacks) => format!(
            "Blood obelisk completed: +{gold} gold +{shards} shards +{essence} essence, Momentum {stacks}x"
        ),
        None => {
            format!("Blood obelisk completed: +{gold} gold +{shards} shards +{essence} essence")
        }
    }
}

pub fn activate_obelisk(obelisk: &mut BloodObelisk) -> bool {
    if obelisk.state != ObeliskState::Dormant {
        return false;
    }
    obelisk.state = ObeliskState::Active;
    obelisk.kills = 0;
    obelisk.target_kills = OBELISK_TARGET_KILLS;
    obelisk.remaining_secs = OBELISK_DURATION_SECS;
    obelisk.reward_claimed = false;
    true
}

pub fn record_obelisk_kill(obelisk: &mut BloodObelisk) {
    if obelisk.state != ObeliskState::Active {
        return;
    }
    obelisk.kills = (obelisk.kills + 1).min(obelisk.target_kills);
}

pub fn tick_obelisk(obelisk: &mut BloodObelisk, delta_secs: f32) {
    if obelisk.state != ObeliskState::Active {
        return;
    }
    obelisk.remaining_secs = (obelisk.remaining_secs - delta_secs.max(0.0)).max(0.0);
    if obelisk.remaining_secs <= 0.0 && obelisk.kills < obelisk.target_kills {
        obelisk.state = ObeliskState::Failed;
    }
}

pub fn complete_obelisk_if_ready(
    obelisk: &mut BloodObelisk,
    stats: &mut RunStats,
    modifier: crate::ordeal::ChapterModifier,
) -> bool {
    if obelisk.state != ObeliskState::Active
        || obelisk.kills < obelisk.target_kills
        || obelisk.reward_claimed
    {
        return false;
    }
    stats.gold += modifier.scale_reward(OBELISK_REWARD_GOLD);
    stats.ember_shards += OBELISK_REWARD_SHARDS;
    stats.affix_essence += OBELISK_REWARD_ESSENCE;
    obelisk.state = ObeliskState::Completed;
    obelisk.remaining_secs = 0.0;
    obelisk.reward_claimed = true;
    true
}

pub fn obelisk_summary(obelisk: &BloodObelisk) -> String {
    match obelisk.state {
        ObeliskState::Dormant => "Obelisk: dormant".to_string(),
        ObeliskState::Active => format!(
            "Obelisk: {}/{} kills {:.0}s",
            obelisk.kills, obelisk.target_kills, obelisk.remaining_secs
        ),
        ObeliskState::Completed => "Obelisk: completed".to_string(),
        ObeliskState::Failed => "Obelisk: failed".to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ordeal::{ChapterModifier, ChapterModifierKind, OrdealAffix};

    #[test]
    fn obelisk_activates_tracks_kills_and_rewards_once() {
        let mut obelisk = BloodObelisk::default();
        let mut stats = RunStats::default();

        assert!(activate_obelisk(&mut obelisk));
        assert!(!activate_obelisk(&mut obelisk));
        for _ in 0..OBELISK_TARGET_KILLS {
            record_obelisk_kill(&mut obelisk);
        }

        assert!(complete_obelisk_if_ready(
            &mut obelisk,
            &mut stats,
            ChapterModifier {
                kind: ChapterModifierKind::AshenEchoes,
                affix: OrdealAffix::None,
            }
        ));
        assert!(!complete_obelisk_if_ready(
            &mut obelisk,
            &mut stats,
            ChapterModifier {
                kind: ChapterModifierKind::AshenEchoes,
                affix: OrdealAffix::None,
            }
        ));
        assert_eq!(obelisk.state, ObeliskState::Completed);
        assert_eq!(stats.ember_shards, OBELISK_REWARD_SHARDS);
        assert_eq!(stats.affix_essence, OBELISK_REWARD_ESSENCE);
        assert_eq!(stats.gold, OBELISK_REWARD_GOLD);
    }

    #[test]
    fn obelisk_rewards_use_rotating_ordeal_affix() {
        let mut obelisk = BloodObelisk::default();
        let mut stats = RunStats::default();

        assert!(activate_obelisk(&mut obelisk));
        for _ in 0..OBELISK_TARGET_KILLS {
            record_obelisk_kill(&mut obelisk);
        }

        let modifier = ChapterModifier {
            kind: ChapterModifierKind::BloodTithe,
            affix: OrdealAffix::TreasureFever,
        };
        assert!(complete_obelisk_if_ready(
            &mut obelisk,
            &mut stats,
            modifier
        ));
        assert_eq!(stats.gold, modifier.scale_reward(OBELISK_REWARD_GOLD));
    }

    #[test]
    fn obelisk_fails_when_timer_expires_before_target() {
        let mut obelisk = BloodObelisk::default();

        activate_obelisk(&mut obelisk);
        record_obelisk_kill(&mut obelisk);
        tick_obelisk(&mut obelisk, OBELISK_DURATION_SECS + 0.1);

        assert_eq!(obelisk.state, ObeliskState::Failed);
        assert!(obelisk_summary(&obelisk).contains("failed"));
    }
}
