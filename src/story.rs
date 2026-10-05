use crate::{
    GameState,
    chapter::{
        ActiveChapter, ChapterId, ChapterPhase, ChapterProgress, InteractableKind, InteractableUsed,
    },
    enemy::{BossPhase, Enemy, EnemyKilled},
    feedback::CombatEvent,
    not_paused,
    save::PendingLoadGame,
    story_content::{chapter_story_beats, story_scene},
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
    HollowArrival,
    ChoirCacheRevealed,
    ChoirSealRecovered,
    BellCryptOpened,
    BellWardenSlain,
    SoulFontDemand,
    MatronAwakened,
    MatronBloodied,
    BastionVictory,
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
            Self::HollowArrival => "The Hollow Road",
            Self::ChoirCacheRevealed => "Choir Reliquary",
            Self::ChoirSealRecovered => "Choir Seal",
            Self::BellCryptOpened => "Bell Crypt Opened",
            Self::BellWardenSlain => "The Last Bell",
            Self::SoulFontDemand => "Soul Font",
            Self::MatronAwakened => "Matron Awakened",
            Self::MatronBloodied => "Matron Bloodied",
            Self::BastionVictory => "Bastion Silenced",
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
            Self::HollowArrival => {
                "Malrec's broken seal points north, where a dead bastion still sings."
            }
            Self::ChoirCacheRevealed => {
                "The Hollowbound fall. A choir reliquary answers with a stolen voice."
            }
            Self::ChoirSealRecovered => {
                "The choir seal joins your blade, and the bell crypt shudders open."
            }
            Self::BellCryptOpened => {
                "Vesper tolls the last bell. Every corpse in the crypt turns to listen."
            }
            Self::BellWardenSlain => {
                "Vesper is silenced. The second seal remembers the sound of daylight."
            }
            Self::SoulFontDemand => {
                "Nyxara's soul font holds the final seal. Break the chorus at its source."
            }
            Self::MatronAwakened => {
                "Nyxara rises beneath a crown of voices, carrying the seal in her hollow heart."
            }
            Self::MatronBloodied => {
                "The Matron's chorus fractures. Strike before the Hollow Crescendo peaks."
            }
            Self::BastionVictory => {
                "The last voice fades. Beyond the bastion, something older answers the silence."
            }
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

#[derive(Message, Debug, Clone, Copy, PartialEq)]
pub struct StoryDialogueEvent {
    pub beat: StoryBeat,
    pub speaker: &'static str,
    pub text: &'static str,
    pub next_action: Option<&'static str>,
    pub duration_secs: f32,
}

#[derive(Message, Debug, Clone, Copy, Default, Eq, PartialEq)]
pub struct StoryPresentationReset;

#[derive(Resource, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StoryLog {
    pub beats: Vec<StoryBeat>,
    pub latest: String,
}

impl Default for StoryLog {
    fn default() -> Self {
        Self::for_chapter(ChapterId::default())
    }
}

impl StoryLog {
    pub fn for_chapter(chapter: ChapterId) -> Self {
        let mut log = Self {
            beats: Vec::new(),
            latest: String::new(),
        };
        log.record(match chapter {
            ChapterId::AshenReliquary => StoryBeat::Arrival,
            ChapterId::HollowBastion => StoryBeat::HollowArrival,
        });
        log
    }

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
            .add_message::<StoryDialogueEvent>()
            .add_message::<StoryPresentationReset>()
            .add_systems(OnEnter(GameState::InGame), reset_story)
            .add_systems(
                Update,
                update_story_log.run_if(in_state(GameState::InGame).and_then(not_paused)),
            );
    }
}

fn reset_story(
    active_chapter: Res<ActiveChapter>,
    pending_load: Res<PendingLoadGame>,
    mut story: ResMut<StoryLog>,
    mut combat_events: MessageWriter<CombatEvent>,
    mut dialogue_events: MessageWriter<StoryDialogueEvent>,
) {
    *story = StoryLog::for_chapter(active_chapter.0);
    if pending_load.is_requested() {
        return;
    }
    combat_events.write(CombatEvent {
        text: story.latest.clone(),
    });
    emit_story_scene(
        chapter_story_beats(active_chapter.0)[0],
        &mut dialogue_events,
    );
}

fn update_story_log(
    progress: Res<ChapterProgress>,
    mut interactions: MessageReader<InteractableUsed>,
    mut kills: MessageReader<EnemyKilled>,
    boss_phases: Query<(&Enemy, &BossPhase)>,
    mut story: ResMut<StoryLog>,
    mut combat_events: MessageWriter<CombatEvent>,
    mut dialogue_events: MessageWriter<StoryDialogueEvent>,
) {
    let chapter = progress.chapter_id();

    for interaction in interactions.read() {
        match interaction.kind {
            InteractableKind::Cache => {
                record_story_beat(
                    match chapter {
                        ChapterId::AshenReliquary => StoryBeat::SealRecovered,
                        ChapterId::HollowBastion => StoryBeat::ChoirSealRecovered,
                    },
                    &mut story,
                    &mut combat_events,
                    &mut dialogue_events,
                );
            }
            InteractableKind::Altar => {
                record_story_beat(
                    match chapter {
                        ChapterId::AshenReliquary => StoryBeat::KeeperAwakened,
                        ChapterId::HollowBastion => StoryBeat::MatronAwakened,
                    },
                    &mut story,
                    &mut combat_events,
                    &mut dialogue_events,
                );
            }
            InteractableKind::CursedShrine => {
                record_story_beat(
                    StoryBeat::CursedShrine,
                    &mut story,
                    &mut combat_events,
                    &mut dialogue_events,
                );
            }
            InteractableKind::BloodObelisk => {
                record_story_beat(
                    StoryBeat::BloodObelisk,
                    &mut story,
                    &mut combat_events,
                    &mut dialogue_events,
                );
            }
            InteractableKind::ReliquaryVault => {
                record_story_beat(
                    StoryBeat::ReliquaryVault,
                    &mut story,
                    &mut combat_events,
                    &mut dialogue_events,
                );
            }
            InteractableKind::EmberRift => {
                record_story_beat(
                    StoryBeat::EmberRift,
                    &mut story,
                    &mut combat_events,
                    &mut dialogue_events,
                );
            }
            InteractableKind::AshenPylon => {
                record_story_beat(
                    StoryBeat::AshenPylon,
                    &mut story,
                    &mut combat_events,
                    &mut dialogue_events,
                );
            }
            _ => {}
        }
    }

    for kill in kills.read() {
        if kill.enemy_id == chapter.warden_id() {
            record_story_beat(
                match chapter {
                    ChapterId::AshenReliquary => StoryBeat::SealWardenSlain,
                    ChapterId::HollowBastion => StoryBeat::BellWardenSlain,
                },
                &mut story,
                &mut combat_events,
                &mut dialogue_events,
            );
        }
        if kill.enemy_id == chapter.boss_id() {
            record_story_beat(
                match chapter {
                    ChapterId::AshenReliquary => StoryBeat::Victory,
                    ChapterId::HollowBastion => StoryBeat::BastionVictory,
                },
                &mut story,
                &mut combat_events,
                &mut dialogue_events,
            );
        }
    }

    // Gameplay messages are handled first so a cache, warden, altar, or boss
    // exchange is queued before the phase narration it unlocks.
    record_progress_beats(
        &progress,
        &mut story,
        &mut combat_events,
        &mut dialogue_events,
    );
    record_boss_phase_beats(
        chapter,
        &boss_phases,
        &mut story,
        &mut combat_events,
        &mut dialogue_events,
    );
}

fn record_boss_phase_beats(
    chapter: ChapterId,
    boss_phases: &Query<(&Enemy, &BossPhase)>,
    story: &mut StoryLog,
    combat_events: &mut MessageWriter<CombatEvent>,
    dialogue_events: &mut MessageWriter<StoryDialogueEvent>,
) {
    for (enemy, phase) in boss_phases.iter() {
        if enemy.id != chapter.boss_id() {
            continue;
        }
        if let Some(beat) = boss_phase_story_beat_for(chapter, phase.phase_two_started()) {
            record_story_beat(beat, story, combat_events, dialogue_events);
        }
    }
}

#[cfg(test)]
fn boss_phase_story_beat(phase_two_started: bool) -> Option<StoryBeat> {
    boss_phase_story_beat_for(ChapterId::AshenReliquary, phase_two_started)
}

fn boss_phase_story_beat_for(chapter: ChapterId, phase_two_started: bool) -> Option<StoryBeat> {
    phase_two_started.then_some(match chapter {
        ChapterId::AshenReliquary => StoryBeat::KeeperBloodied,
        ChapterId::HollowBastion => StoryBeat::MatronBloodied,
    })
}

fn record_progress_beats(
    progress: &ChapterProgress,
    story: &mut StoryLog,
    combat_events: &mut MessageWriter<CombatEvent>,
    dialogue_events: &mut MessageWriter<StoryDialogueEvent>,
) {
    let beat = match (progress.chapter_id(), progress.phase) {
        (_, ChapterPhase::Breach) => None,
        (ChapterId::AshenReliquary, ChapterPhase::Cache) => Some(StoryBeat::CacheRevealed),
        (ChapterId::AshenReliquary, ChapterPhase::Sanctum) => Some(StoryBeat::SanctumOpened),
        (ChapterId::AshenReliquary, ChapterPhase::Ritual) => Some(StoryBeat::AltarDemand),
        (ChapterId::AshenReliquary, ChapterPhase::Boss) => Some(StoryBeat::KeeperAwakened),
        (ChapterId::AshenReliquary, ChapterPhase::Victory) => Some(StoryBeat::Victory),
        (ChapterId::HollowBastion, ChapterPhase::Cache) => Some(StoryBeat::ChoirCacheRevealed),
        (ChapterId::HollowBastion, ChapterPhase::Sanctum) => Some(StoryBeat::BellCryptOpened),
        (ChapterId::HollowBastion, ChapterPhase::Ritual) => Some(StoryBeat::SoulFontDemand),
        (ChapterId::HollowBastion, ChapterPhase::Boss) => Some(StoryBeat::MatronAwakened),
        (ChapterId::HollowBastion, ChapterPhase::Victory) => Some(StoryBeat::BastionVictory),
    };
    if let Some(beat) = beat {
        record_story_beat(beat, story, combat_events, dialogue_events);
    }
}

fn record_story_beat(
    beat: StoryBeat,
    story: &mut StoryLog,
    combat_events: &mut MessageWriter<CombatEvent>,
    dialogue_events: &mut MessageWriter<StoryDialogueEvent>,
) {
    if story.record(beat) {
        combat_events.write(CombatEvent {
            text: story.latest.clone(),
        });
        emit_story_scene(beat, dialogue_events);
    }
}

fn emit_story_scene(beat: StoryBeat, dialogue_events: &mut MessageWriter<StoryDialogueEvent>) {
    for event in dialogue_events_for(beat) {
        dialogue_events.write(event);
    }
}

pub fn dialogue_events_for(beat: StoryBeat) -> Vec<StoryDialogueEvent> {
    let scene = story_scene(beat);
    let last_line = scene.lines.len().saturating_sub(1);
    scene
        .lines
        .iter()
        .enumerate()
        .map(|(index, line)| StoryDialogueEvent {
            beat,
            speaker: line.speaker,
            text: line.text,
            next_action: (index == last_line).then_some(scene.next_action),
            duration_secs: scene.line_duration_secs,
        })
        .collect()
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
        assert!(story.beats.contains(&StoryBeat::CacheRevealed));
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
    }

    #[test]
    fn boss_phase_story_beat_records_keeper_bloodied_once_phase_two_starts() {
        assert_eq!(boss_phase_story_beat(false), None);
        assert_eq!(boss_phase_story_beat(true), Some(StoryBeat::KeeperBloodied));
        assert_eq!(
            boss_phase_story_beat_for(ChapterId::HollowBastion, true),
            Some(StoryBeat::MatronBloodied)
        );
    }

    #[test]
    fn dialogue_events_preserve_scene_order_and_end_with_the_objective() {
        for chapter in [ChapterId::AshenReliquary, ChapterId::HollowBastion] {
            for beat in crate::story_content::chapter_story_beats(chapter) {
                let events = dialogue_events_for(*beat);
                assert!(!events.is_empty(), "{beat:?} should publish dialogue");
                assert!(
                    events[..events.len() - 1]
                        .iter()
                        .all(|event| event.next_action.is_none()),
                    "{beat:?} should not show the objective before its final line"
                );
                assert!(
                    events
                        .last()
                        .is_some_and(|event| event.next_action.is_some()),
                    "{beat:?} should end with an actionable objective"
                );
                assert!(events.iter().all(|event| event.beat == *beat));
            }
        }
    }
}
