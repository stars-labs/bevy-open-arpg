use crate::{
    GameState,
    chapter::{ChapterPhase, ChapterProgress, InteractableKind, InteractableUsed},
    enemy::{BossPhase, Enemy, EnemyKilled},
    feedback::CombatEvent,
    not_paused,
};
use bevy::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub enum StoryBeat {
    Arrival,
    CacheRevealed,
    SealRecovered,
    SanctumOpened,
    SealWardenSlain,
    AltarDemand,
    KeeperAwakened,
    KeeperBloodied,
    Victory,
    CursedShrine,
    BloodObelisk,
    ReliquaryVault,
    EmberRift,
    AshenPylon,
}

impl StoryBeat {
    const TOTAL: usize = 14;

    pub fn title(self) -> &'static str {
        match self {
            Self::Arrival => "Arrival",
            Self::CacheRevealed => "Cache Revealed",
            Self::SealRecovered => "First Seal",
            Self::SanctumOpened => "Sanctum Opened",
            Self::SealWardenSlain => "Seal Warden",
            Self::AltarDemand => "Ember Altar",
            Self::KeeperAwakened => "Keeper Awakened",
            Self::KeeperBloodied => "Keeper Bloodied",
            Self::Victory => "Reliquary Cleansed",
            Self::CursedShrine => "Cursed Shrine",
            Self::BloodObelisk => "Blood Rite",
            Self::ReliquaryVault => "Resplendent Vault",
            Self::EmberRift => "Ember Rift",
            Self::AshenPylon => "Ashen Pylon",
        }
    }

    pub fn line(self) -> &'static str {
        match self {
            Self::Arrival => {
                "The steward's ward flickers: recover the three seals before ash takes the vault."
            }
            Self::CacheRevealed => {
                "The first guards fall. A sealed cache hums beneath the outer reliquary."
            }
            Self::SealRecovered => "A reliquary seal locks into the blade. Malrec's gate weakens.",
            Self::SanctumOpened => "The inner sanctum opens, and acolytes answer the broken seal.",
            Self::SealWardenSlain => {
                "Seal Warden Vhal falls. The second reliquary seal answers the blade."
            }
            Self::AltarDemand => {
                "The final seal burns inside the ember altar. Extinguish it before the keeper wakes."
            }
            Self::KeeperAwakened => {
                "Malrec rises from the ash gate: your relic blade is the last binding oath."
            }
            Self::KeeperBloodied => "Malrec staggers, but the reliquary shakes with borrowed fire.",
            Self::Victory => "The ash oath breaks. The reliquary is yours, for now.",
            Self::CursedShrine => {
                "The shrine drinks blood and buys a cruel bargain from the reliquary dark."
            }
            Self::BloodObelisk => {
                "The obelisk answers with names of the dead. Finish the rite or feed it."
            }
            Self::ReliquaryVault => "Ancient locks turn inside the vault. Greed has a voice here.",
            Self::EmberRift => {
                "An ember rift tears open, spilling hunters from Malrec's failed oaths."
            }
            Self::AshenPylon => {
                "The pylon overloads your relic with ash-fire, bright enough to burn back the dark."
            }
        }
    }
}

#[derive(Resource, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StoryLog {
    pub beats: Vec<StoryBeat>,
    pub latest: String,
}

impl Default for StoryLog {
    fn default() -> Self {
        let mut log = Self {
            beats: Vec::new(),
            latest: String::new(),
        };
        log.record(StoryBeat::Arrival);
        log
    }
}

impl StoryLog {
    pub fn total_beats(&self) -> usize {
        StoryBeat::TOTAL
    }

    pub fn record(&mut self, beat: StoryBeat) -> bool {
        if self.beats.contains(&beat) {
            return false;
        }
        self.beats.push(beat);
        self.latest = format!("{}: {}", beat.title(), beat.line());
        true
    }

    pub fn restore(&mut self, beats: Vec<StoryBeat>) {
        self.beats.clear();
        for beat in beats {
            self.record(beat);
        }
        if self.beats.is_empty() {
            self.record(StoryBeat::Arrival);
        }
    }

    pub fn summary(&self) -> String {
        let latest = if self.latest.is_empty() {
            StoryBeat::Arrival.line()
        } else {
            self.latest.as_str()
        };
        format!("Story {}/{}: {latest}", self.beats.len(), StoryBeat::TOTAL)
    }

    pub fn recap(&self) -> String {
        if self.beats.is_empty() {
            return StoryBeat::Arrival.line().to_string();
        }
        self.beats
            .iter()
            .map(|beat| beat.title())
            .collect::<Vec<_>>()
            .join(" > ")
    }
}

pub struct StoryPlugin;

impl Plugin for StoryPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<StoryLog>()
            .add_systems(OnEnter(GameState::InGame), reset_story)
            .add_systems(
                Update,
                update_story_log.run_if(in_state(GameState::InGame).and_then(not_paused)),
            );
    }
}

fn reset_story(mut story: ResMut<StoryLog>) {
    *story = StoryLog::default();
}

fn update_story_log(
    progress: Res<ChapterProgress>,
    mut interactions: MessageReader<InteractableUsed>,
    mut kills: MessageReader<EnemyKilled>,
    boss_phases: Query<(&Enemy, &BossPhase)>,
    mut story: ResMut<StoryLog>,
    mut combat_events: MessageWriter<CombatEvent>,
) {
    record_progress_beats(&progress, &mut story, &mut combat_events);
    record_boss_phase_beats(&boss_phases, &mut story, &mut combat_events);

    for interaction in interactions.read() {
        match interaction.kind {
            InteractableKind::Cache => {
                record_story_beat(StoryBeat::SealRecovered, &mut story, &mut combat_events);
            }
            InteractableKind::Altar => {
                record_story_beat(StoryBeat::KeeperAwakened, &mut story, &mut combat_events);
            }
            InteractableKind::CursedShrine => {
                record_story_beat(StoryBeat::CursedShrine, &mut story, &mut combat_events);
            }
            InteractableKind::BloodObelisk => {
                record_story_beat(StoryBeat::BloodObelisk, &mut story, &mut combat_events);
            }
            InteractableKind::ReliquaryVault => {
                record_story_beat(StoryBeat::ReliquaryVault, &mut story, &mut combat_events);
            }
            InteractableKind::EmberRift => {
                record_story_beat(StoryBeat::EmberRift, &mut story, &mut combat_events);
            }
            InteractableKind::AshenPylon => {
                record_story_beat(StoryBeat::AshenPylon, &mut story, &mut combat_events);
            }
            _ => {}
        }
    }

    for kill in kills.read() {
        if kill.enemy_id == "seal_warden" {
            record_story_beat(StoryBeat::SealWardenSlain, &mut story, &mut combat_events);
        }
        if kill.enemy_id == "keeper" {
            record_story_beat(StoryBeat::Victory, &mut story, &mut combat_events);
        }
    }
}

fn record_boss_phase_beats(
    boss_phases: &Query<(&Enemy, &BossPhase)>,
    story: &mut StoryLog,
    combat_events: &mut MessageWriter<CombatEvent>,
) {
    for (enemy, phase) in boss_phases.iter() {
        if enemy.id != "keeper" {
            continue;
        }
        if let Some(beat) = boss_phase_story_beat(phase.phase_two_started()) {
            record_story_beat(beat, story, combat_events);
        }
    }
}

fn boss_phase_story_beat(phase_two_started: bool) -> Option<StoryBeat> {
    phase_two_started.then_some(StoryBeat::KeeperBloodied)
}

fn record_progress_beats(
    progress: &ChapterProgress,
    story: &mut StoryLog,
    combat_events: &mut MessageWriter<CombatEvent>,
) {
    let beat = match progress.phase {
        ChapterPhase::Breach => None,
        ChapterPhase::Cache => Some(StoryBeat::CacheRevealed),
        ChapterPhase::Sanctum => Some(StoryBeat::SanctumOpened),
        ChapterPhase::Ritual => Some(StoryBeat::AltarDemand),
        ChapterPhase::Boss => Some(StoryBeat::KeeperAwakened),
        ChapterPhase::Victory => Some(StoryBeat::Victory),
    };
    if let Some(beat) = beat {
        record_story_beat(beat, story, combat_events);
    }
}

fn record_story_beat(
    beat: StoryBeat,
    story: &mut StoryLog,
    combat_events: &mut MessageWriter<CombatEvent>,
) {
    if story.record(beat) {
        combat_events.write(CombatEvent {
            text: story.latest.clone(),
        });
    }
}

pub fn story_summary(story: &StoryLog) -> String {
    story.summary()
}

pub fn story_recap(story: &StoryLog) -> String {
    story.recap()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn story_log_records_each_beat_once() {
        let mut story = StoryLog::default();

        assert!(story.record(StoryBeat::CacheRevealed));
        assert!(!story.record(StoryBeat::CacheRevealed));

        assert_eq!(story.beats.len(), 2);
        assert!(story.summary().contains("Cache Revealed"));
        assert!(story.summary().contains("Story 2/14"));
        assert!(story.recap().contains("Arrival > Cache Revealed"));
    }

    #[test]
    fn story_restore_keeps_latest_line_and_backfills_arrival() {
        let mut story = StoryLog {
            beats: Vec::new(),
            latest: String::new(),
        };

        story.restore(vec![StoryBeat::KeeperAwakened, StoryBeat::Victory]);
        assert_eq!(
            story.beats,
            vec![StoryBeat::KeeperAwakened, StoryBeat::Victory]
        );
        assert!(story.latest.contains("Reliquary Cleansed"));

        story.restore(Vec::new());
        assert_eq!(story.beats, vec![StoryBeat::Arrival]);
    }

    #[test]
    fn optional_reliquary_sites_add_story_beats() {
        let mut story = StoryLog::default();
        for beat in [
            StoryBeat::CursedShrine,
            StoryBeat::BloodObelisk,
            StoryBeat::ReliquaryVault,
            StoryBeat::EmberRift,
            StoryBeat::AshenPylon,
        ] {
            assert!(story.record(beat));
        }

        assert!(story.recap().contains("Cursed Shrine"));
        assert!(story.recap().contains("Blood Rite"));
        assert!(story.recap().contains("Resplendent Vault"));
        assert!(story.recap().contains("Ember Rift"));
        assert!(story.latest.contains("Ashen Pylon"));
        assert!(story.summary().contains("Story 6/14"));
    }

    #[test]
    fn boss_phase_story_beat_records_keeper_bloodied_once_phase_two_starts() {
        assert_eq!(boss_phase_story_beat(false), None);
        assert_eq!(boss_phase_story_beat(true), Some(StoryBeat::KeeperBloodied));
    }
}
