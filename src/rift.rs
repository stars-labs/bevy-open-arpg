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

pub const RIFT_TARGET_KILLS: u32 = 6;
pub const RIFT_SWIFT_SEAL_SECS: f32 = 30.0;
pub const ECHO_KEYSTONE_SWIFT_REWARD: u32 = 1;
const RIFT_DURATION_SECS: f32 = 60.0;
const RIFT_REWARD_GOLD: u32 = 135;
const RIFT_REWARD_SHARDS: u32 = 7;
const RIFT_REWARD_ESSENCE: u32 = 5;
const RIFT_SWIFT_REWARD_GOLD: u32 = 85;
const RIFT_SWIFT_REWARD_SHARDS: u32 = 3;
const RIFT_SWIFT_REWARD_ESSENCE: u32 = 2;

#[derive(Debug, Clone, Copy, Default, Eq, PartialEq, Serialize, Deserialize)]
pub enum RiftState {
    #[default]
    Sealed,
    Active,
    Completed,
    Failed,
}

#[derive(Resource, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EmberRift {
    pub state: RiftState,
    pub kills: u32,
    pub target_kills: u32,
    pub remaining_secs: f32,
    pub reward_claimed: bool,
    #[serde(default)]
    pub best_remaining_secs: f32,
}

impl Default for EmberRift {
    fn default() -> Self {
        Self {
            state: RiftState::Sealed,
            kills: 0,
            target_kills: RIFT_TARGET_KILLS,
            remaining_secs: 0.0,
            reward_claimed: false,
            best_remaining_secs: 0.0,
        }
    }
}

#[derive(Message, Debug, Clone, Copy)]
pub struct SpawnRiftWave {
    pub origin: Vec3,
}

pub struct RiftPlugin;

impl Plugin for RiftPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<EmberRift>()
            .add_message::<SpawnRiftWave>()
            .add_systems(OnEnter(GameState::InGame), reset_rift)
            .add_systems(
                Update,
                update_rift.run_if(in_state(GameState::InGame).and_then(not_paused)),
            );
    }
}

fn reset_rift(mut rift: ResMut<EmberRift>) {
    *rift = EmberRift::default();
}

#[derive(SystemParam)]
struct RiftResources<'w> {
    time: Res<'w, Time>,
    rift: ResMut<'w, EmberRift>,
    modifier: Res<'w, ChapterModifier>,
    stats: ResMut<'w, RunStats>,
}

#[derive(SystemParam)]
struct RiftEvents<'w, 's> {
    interactions: MessageReader<'w, 's, InteractableUsed>,
    enemy_killed: MessageReader<'w, 's, EnemyKilled>,
    wave_events: MessageWriter<'w, SpawnRiftWave>,
    combat_events: MessageWriter<'w, CombatEvent>,
}

fn update_rift(
    mut resources: RiftResources,
    mut events: RiftEvents,
    mut player: Query<(&mut RelicBuff, &mut Fury), With<Player>>,
) {
    for event in events.interactions.read() {
        if event.kind == InteractableKind::EmberRift && activate_rift(&mut resources.rift) {
            events.wave_events.write(SpawnRiftWave {
                origin: event.position,
            });
            events.combat_events.write(CombatEvent {
                text: "Ember rift opened: defeat six invaders before it collapses".to_string(),
            });
        }
    }

    if resources.rift.state != RiftState::Active {
        events.enemy_killed.clear();
        return;
    }

    for _ in events.enemy_killed.read() {
        record_rift_kill(&mut resources.rift);
    }

    let reward = rift_completion_reward(resources.rift.remaining_secs, *resources.modifier);
    if complete_rift_if_ready(
        &mut resources.rift,
        &mut resources.stats,
        *resources.modifier,
    ) {
        let momentum_amount = if reward.swift_bonus { 2 } else { 1 };
        let momentum = player.single_mut().ok().map(|(mut relic, mut fury)| {
            grant_reliquary_momentum(&mut resources.stats, &mut relic, &mut fury, momentum_amount)
        });
        events.combat_events.write(CombatEvent {
            text: format!(
                "Ember rift sealed{}: +{} gold +{} shards +{} essence{}",
                if reward.swift_bonus {
                    " swiftly, Echo Keystone claimed"
                } else {
                    ""
                },
                reward.gold,
                reward.shards,
                reward.essence,
                momentum
                    .map(|reward| format!(", Momentum {}x", reward.stacks))
                    .unwrap_or_default()
            ),
        });
        return;
    }

    tick_rift(&mut resources.rift, resources.time.delta_secs());
    if resources.rift.state == RiftState::Failed {
        events.combat_events.write(CombatEvent {
            text: "Ember rift collapsed before it was sealed".to_string(),
        });
    }
}

pub fn activate_rift(rift: &mut EmberRift) -> bool {
    if rift.state != RiftState::Sealed {
        return false;
    }
    rift.state = RiftState::Active;
    rift.kills = 0;
    rift.target_kills = RIFT_TARGET_KILLS;
    rift.remaining_secs = RIFT_DURATION_SECS;
    rift.reward_claimed = false;
    true
}

pub fn record_rift_kill(rift: &mut EmberRift) {
    if rift.state != RiftState::Active {
        return;
    }
    rift.kills = (rift.kills + 1).min(rift.target_kills);
}

pub fn tick_rift(rift: &mut EmberRift, delta_secs: f32) {
    if rift.state != RiftState::Active {
        return;
    }
    rift.remaining_secs = (rift.remaining_secs - delta_secs.max(0.0)).max(0.0);
    if rift.remaining_secs <= 0.0 && rift.kills < rift.target_kills {
        rift.state = RiftState::Failed;
    }
}

pub fn complete_rift_if_ready(
    rift: &mut EmberRift,
    stats: &mut RunStats,
    modifier: crate::ordeal::ChapterModifier,
) -> bool {
    if rift.state != RiftState::Active || rift.kills < rift.target_kills || rift.reward_claimed {
        return false;
    }
    let reward = rift_completion_reward(rift.remaining_secs, modifier);
    stats.gold += reward.gold;
    stats.ember_shards += reward.shards;
    stats.affix_essence += reward.essence;
    if reward.echo_keystone {
        stats.echo_keystones += ECHO_KEYSTONE_SWIFT_REWARD;
    }
    rift.best_remaining_secs = rift.best_remaining_secs.max(rift.remaining_secs);
    rift.state = RiftState::Completed;
    rift.remaining_secs = 0.0;
    rift.reward_claimed = true;
    true
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RiftCompletionReward {
    pub gold: u32,
    pub shards: u32,
    pub essence: u32,
    pub swift_bonus: bool,
    pub echo_keystone: bool,
}

pub fn rift_completion_reward(
    remaining_secs: f32,
    modifier: crate::ordeal::ChapterModifier,
) -> RiftCompletionReward {
    let swift_bonus = remaining_secs >= RIFT_SWIFT_SEAL_SECS;
    RiftCompletionReward {
        gold: modifier.scale_reward(
            RIFT_REWARD_GOLD
                + if swift_bonus {
                    RIFT_SWIFT_REWARD_GOLD
                } else {
                    0
                },
        ),
        shards: RIFT_REWARD_SHARDS
            + if swift_bonus {
                RIFT_SWIFT_REWARD_SHARDS
            } else {
                0
            },
        essence: RIFT_REWARD_ESSENCE
            + if swift_bonus {
                RIFT_SWIFT_REWARD_ESSENCE
            } else {
                0
            },
        swift_bonus,
        echo_keystone: swift_bonus,
    }
}

pub fn rift_summary(rift: &EmberRift) -> String {
    match rift.state {
        RiftState::Sealed => "Rift: sealed".to_string(),
        RiftState::Active => format!(
            "Rift: {}/{} kills {:.0}s",
            rift.kills, rift.target_kills, rift.remaining_secs
        ),
        RiftState::Completed => format!("Rift: sealed best {:.0}s", rift.best_remaining_secs),
        RiftState::Failed => "Rift: collapsed".to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ordeal::{ChapterModifier, ChapterModifierKind, OrdealAffix};

    #[test]
    fn rift_activates_tracks_kills_and_rewards_once() {
        let mut rift = EmberRift::default();
        let mut stats = RunStats::default();

        assert!(activate_rift(&mut rift));
        assert!(!activate_rift(&mut rift));
        for _ in 0..RIFT_TARGET_KILLS {
            record_rift_kill(&mut rift);
        }

        assert!(complete_rift_if_ready(
            &mut rift,
            &mut stats,
            ChapterModifier {
                kind: ChapterModifierKind::AshenEchoes,
                affix: OrdealAffix::None,
            }
        ));
        assert!(!complete_rift_if_ready(
            &mut rift,
            &mut stats,
            ChapterModifier {
                kind: ChapterModifierKind::AshenEchoes,
                affix: OrdealAffix::None,
            }
        ));
        assert_eq!(rift.state, RiftState::Completed);
        assert_eq!(
            stats.ember_shards,
            RIFT_REWARD_SHARDS + RIFT_SWIFT_REWARD_SHARDS
        );
        assert_eq!(
            stats.affix_essence,
            RIFT_REWARD_ESSENCE + RIFT_SWIFT_REWARD_ESSENCE
        );
        assert_eq!(
            stats.gold,
            ChapterModifierKind::AshenEchoes
                .scale_reward(RIFT_REWARD_GOLD + RIFT_SWIFT_REWARD_GOLD)
        );
        assert_eq!(stats.echo_keystones, ECHO_KEYSTONE_SWIFT_REWARD);
        assert_eq!(rift.best_remaining_secs, RIFT_DURATION_SECS);
        assert!(rift_summary(&rift).contains("best 60s"));
    }

    #[test]
    fn rift_slow_completion_skips_swift_bonus() {
        let mut rift = EmberRift::default();
        let mut stats = RunStats::default();

        activate_rift(&mut rift);
        tick_rift(&mut rift, RIFT_DURATION_SECS - RIFT_SWIFT_SEAL_SECS + 1.0);
        for _ in 0..RIFT_TARGET_KILLS {
            record_rift_kill(&mut rift);
        }

        assert!(complete_rift_if_ready(
            &mut rift,
            &mut stats,
            ChapterModifier {
                kind: ChapterModifierKind::AshenEchoes,
                affix: OrdealAffix::None,
            }
        ));
        assert_eq!(stats.ember_shards, RIFT_REWARD_SHARDS);
        assert_eq!(stats.affix_essence, RIFT_REWARD_ESSENCE);
        assert_eq!(stats.gold, RIFT_REWARD_GOLD);
        assert_eq!(stats.echo_keystones, 0);
        assert!(rift.best_remaining_secs < RIFT_SWIFT_SEAL_SECS);
    }

    #[test]
    fn rift_rewards_use_rotating_ordeal_affix() {
        let modifier = ChapterModifier {
            kind: ChapterModifierKind::Emberstorm,
            affix: OrdealAffix::TreasureFever,
        };
        let reward = rift_completion_reward(RIFT_DURATION_SECS, modifier);

        assert_eq!(
            reward.gold,
            modifier.scale_reward(RIFT_REWARD_GOLD + RIFT_SWIFT_REWARD_GOLD)
        );
        assert!(reward.swift_bonus);
    }

    #[test]
    fn rift_fails_when_timer_expires_before_target() {
        let mut rift = EmberRift::default();

        activate_rift(&mut rift);
        record_rift_kill(&mut rift);
        tick_rift(&mut rift, RIFT_DURATION_SECS + 0.1);

        assert_eq!(rift.state, RiftState::Failed);
        assert!(rift_summary(&rift).contains("collapsed"));
    }
}
