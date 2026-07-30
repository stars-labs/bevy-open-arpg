use crate::{
    GameState,
    chapter::{ChapterId, ChapterProgress, InteractableKind, InteractableUsed},
    dungeon::chapter_world_layout,
    feedback::CombatEvent,
    not_paused,
};
use bevy::prelude::*;
use serde::{Deserialize, Serialize};

pub const LORE_ENTRY_COUNT: usize = 3;

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum LoreEntryId {
    StewardWarning,
    AcolyteOath,
    MalrecPact,
    LastBell,
    ChoirLitany,
    NyxaraLament,
}

impl LoreEntryId {
    pub fn title(self) -> &'static str {
        match self {
            Self::StewardWarning => "Steward's Warning",
            Self::AcolyteOath => "Acolyte's Oath",
            Self::MalrecPact => "Malrec's Pact",
            Self::LastBell => "The Last Bell",
            Self::ChoirLitany => "Choir Litany",
            Self::NyxaraLament => "Nyxara's Lament",
        }
    }

    pub fn body(self) -> &'static str {
        match self {
            Self::StewardWarning => "The reliquary was sealed when ash began to answer prayer.",
            Self::AcolyteOath => "We trade names for embers and keep watch until Malrec wakes.",
            Self::MalrecPact => "Blood opens the keeper's gate; only a relic blade can close it.",
            Self::LastBell => "When the bastion bell fell silent, every buried oath began to sing.",
            Self::ChoirLitany => {
                "We lend Nyxara our final breath so the Hollow Choir may never end."
            }
            Self::NyxaraLament => {
                "Malrec stole the first seal. I will drown the second in a thousand voices."
            }
        }
    }
}

#[derive(Resource, Debug, Default)]
pub struct LoreJournal {
    pub entries: Vec<LoreEntryId>,
    pub latest: String,
}

impl LoreJournal {
    pub fn discover(&mut self, entry: LoreEntryId) -> bool {
        if self.entries.contains(&entry) {
            return false;
        }
        self.entries.push(entry);
        self.latest = format!("{}: {}", entry.title(), entry.body());
        true
    }

    pub fn summary(&self) -> String {
        if self.entries.is_empty() {
            return "Journal 0/3: no lore recovered".to_string();
        }
        format!(
            "Journal {}/{}: {}",
            self.entries.len(),
            LORE_ENTRY_COUNT,
            self.entries
                .iter()
                .map(|entry| entry.title())
                .collect::<Vec<_>>()
                .join(" | ")
        )
    }

    pub fn open_text(&self) -> String {
        if self.entries.is_empty() {
            return "Journal: no lore recovered".to_string();
        }
        self.entries
            .iter()
            .map(|entry| format!("{} - {}", entry.title(), entry.body()))
            .collect::<Vec<_>>()
            .join(" | ")
    }
}

pub struct LorePlugin;

impl Plugin for LorePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<LoreJournal>()
            .add_systems(OnEnter(GameState::InGame), reset_lore)
            .add_systems(
                Update,
                collect_lore.run_if(in_state(GameState::InGame).and_then(not_paused)),
            );
    }
}

fn reset_lore(mut journal: ResMut<LoreJournal>) {
    *journal = LoreJournal::default();
}

fn collect_lore(
    mut interactions: MessageReader<InteractableUsed>,
    progress: Res<ChapterProgress>,
    mut journal: ResMut<LoreJournal>,
    mut combat_events: MessageWriter<CombatEvent>,
) {
    for event in interactions.read() {
        if event.kind != InteractableKind::LorePage {
            continue;
        }
        if let Some(entry) = lore_entry_for_chapter_position(progress.chapter_id(), event.position)
            && journal.discover(entry)
        {
            combat_events.write(CombatEvent {
                text: format!("Lore recovered: {}", entry.title()),
            });
        }
    }
}

#[cfg(test)]
pub fn lore_entry_for_position(position: Vec3) -> Option<LoreEntryId> {
    lore_entry_for_chapter_position(ChapterId::AshenReliquary, position)
}

pub fn lore_entry_for_chapter_position(chapter: ChapterId, position: Vec3) -> Option<LoreEntryId> {
    let entries = match chapter {
        ChapterId::AshenReliquary => [
            LoreEntryId::StewardWarning,
            LoreEntryId::AcolyteOath,
            LoreEntryId::MalrecPact,
        ],
        ChapterId::HollowBastion => [
            LoreEntryId::LastBell,
            LoreEntryId::ChoirLitany,
            LoreEntryId::NyxaraLament,
        ],
    };
    chapter_world_layout(chapter)
        .lore_pages
        .into_iter()
        .zip(entries)
        .find_map(|(anchor, entry)| (position.distance(anchor) <= 0.2).then_some(entry))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn journal_discovers_each_lore_entry_once() {
        let mut journal = LoreJournal::default();

        assert!(journal.discover(LoreEntryId::StewardWarning));
        assert!(!journal.discover(LoreEntryId::StewardWarning));
        assert_eq!(journal.entries.len(), 1);
        assert!(journal.summary().contains("Steward"));
    }

    #[test]
    fn lore_positions_map_to_entries() {
        let ashen = chapter_world_layout(ChapterId::AshenReliquary);
        let hollow = chapter_world_layout(ChapterId::HollowBastion);

        for (position, expected) in ashen.lore_pages.into_iter().zip([
            LoreEntryId::StewardWarning,
            LoreEntryId::AcolyteOath,
            LoreEntryId::MalrecPact,
        ]) {
            assert_eq!(lore_entry_for_position(position), Some(expected));
        }
        assert_eq!(
            lore_entry_for_chapter_position(ChapterId::HollowBastion, hollow.lore_pages[2]),
            Some(LoreEntryId::NyxaraLament)
        );
        assert_eq!(lore_entry_for_position(ashen.lore_pages[1] + Vec3::X), None);
    }
}
