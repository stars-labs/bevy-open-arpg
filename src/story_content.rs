use crate::{chapter::ChapterId, story::StoryBeat};

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub struct StoryDialogueLine {
    pub speaker: &'static str,
    pub text: &'static str,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct StoryScene {
    pub lines: &'static [StoryDialogueLine],
    pub next_action: &'static str,
    pub line_duration_secs: f32,
}

const ASHEN_MAINLINE: [StoryBeat; 9] = [
    StoryBeat::Arrival,
    StoryBeat::CacheRevealed,
    StoryBeat::SealRecovered,
    StoryBeat::SanctumOpened,
    StoryBeat::SealWardenSlain,
    StoryBeat::AltarDemand,
    StoryBeat::KeeperAwakened,
    StoryBeat::KeeperBloodied,
    StoryBeat::Victory,
];

const HOLLOW_MAINLINE: [StoryBeat; 9] = [
    StoryBeat::HollowArrival,
    StoryBeat::ChoirCacheRevealed,
    StoryBeat::ChoirSealRecovered,
    StoryBeat::BellCryptOpened,
    StoryBeat::BellWardenSlain,
    StoryBeat::SoulFontDemand,
    StoryBeat::MatronAwakened,
    StoryBeat::MatronBloodied,
    StoryBeat::BastionVictory,
];

pub fn chapter_story_beats(chapter: ChapterId) -> &'static [StoryBeat] {
    match chapter {
        ChapterId::AshenReliquary => &ASHEN_MAINLINE,
        ChapterId::HollowBastion => &HOLLOW_MAINLINE,
    }
}

pub fn chapter_ending_summary(chapter: ChapterId) -> String {
    let ending = match chapter {
        ChapterId::AshenReliquary => StoryBeat::Victory,
        ChapterId::HollowBastion => StoryBeat::BastionVictory,
    };
    story_scene(ending)
        .lines
        .iter()
        .map(|line| format!("{}: {}", line.speaker, line.text))
        .collect::<Vec<_>>()
        .join("\n")
}

pub fn story_scene(beat: StoryBeat) -> StoryScene {
    match beat {
        StoryBeat::Arrival => StoryScene {
            lines: &[
                StoryDialogueLine {
                    speaker: "Steward Edda",
                    text: "Sentinel, the Reliquary is falling. Three stolen seals bind Malrec below the ash gate.",
                },
                StoryDialogueLine {
                    speaker: "Quartermaster Vey",
                    text: "I can hold the road, not the vault. Cut down the outer guard and I will mark the first cache.",
                },
            ],
            next_action: "breach the outer guards",
            line_duration_secs: 3.8,
        },
        StoryBeat::CacheRevealed => StoryScene {
            lines: &[
                StoryDialogueLine {
                    speaker: "Steward Edda",
                    text: "The outer ward is clear. The first seal is inside that cache.",
                },
                StoryDialogueLine {
                    speaker: "Quartermaster Vey",
                    text: "Open it quickly; its lock is feeding the inner sanctum.",
                },
            ],
            next_action: "open the sealed cache",
            line_duration_secs: 3.2,
        },
        StoryBeat::SealRecovered => StoryScene {
            lines: &[
                StoryDialogueLine {
                    speaker: "Reliquary Sentinel",
                    text: "One seal answers the blade.",
                },
                StoryDialogueLine {
                    speaker: "Steward Edda",
                    text: "Good. Carry it north. Vhal will spend every acolyte he has to keep the second.",
                },
            ],
            next_action: "push into the inner sanctum",
            line_duration_secs: 3.4,
        },
        StoryBeat::SanctumOpened => StoryScene {
            lines: &[
                StoryDialogueLine {
                    speaker: "Seal Warden Vhal",
                    text: "The oathless blade comes at last. Kneel, and your name may yet be kept.",
                },
                StoryDialogueLine {
                    speaker: "Steward Edda",
                    text: "Do not answer him. Break the ambush, then break the Warden.",
                },
            ],
            next_action: "survive the ambush and find Vhal",
            line_duration_secs: 3.6,
        },
        StoryBeat::SealWardenSlain => StoryScene {
            lines: &[
                StoryDialogueLine {
                    speaker: "Seal Warden Vhal",
                    text: "No... the second seal was never yours.",
                },
                StoryDialogueLine {
                    speaker: "Reliquary Sentinel",
                    text: "It is now.",
                },
                StoryDialogueLine {
                    speaker: "Steward Edda",
                    text: "Two seals restored. The ember altar holds the last.",
                },
            ],
            next_action: "take the second seal to the altar",
            line_duration_secs: 3.2,
        },
        StoryBeat::AltarDemand => StoryScene {
            lines: &[
                StoryDialogueLine {
                    speaker: "Malrec, Keeper of Ash",
                    text: "Bring the blade closer. Every seal you reclaim opens my gate.",
                },
                StoryDialogueLine {
                    speaker: "Steward Edda",
                    text: "He is baiting you. Extinguish the altar and be ready when the binding fails.",
                },
            ],
            next_action: "extinguish the final seal",
            line_duration_secs: 3.8,
        },
        StoryBeat::KeeperAwakened => StoryScene {
            lines: &[
                StoryDialogueLine {
                    speaker: "Malrec, Keeper of Ash",
                    text: "Three seals, one blade, and a mortal hand. I will burn the oath from all of them.",
                },
                StoryDialogueLine {
                    speaker: "Steward Edda",
                    text: "Take the reliquary boon. Break his guard and drive the seals home.",
                },
            ],
            next_action: "choose a boon and break Malrec",
            line_duration_secs: 4.1,
        },
        StoryBeat::KeeperBloodied => StoryScene {
            lines: &[
                StoryDialogueLine {
                    speaker: "Malrec, Keeper of Ash",
                    text: "The vault lends me every fire you thought extinguished.",
                },
                StoryDialogueLine {
                    speaker: "Steward Edda",
                    text: "His guard is cracking. Survive the rings; punish the next break.",
                },
            ],
            next_action: "save burst for the stagger window",
            line_duration_secs: 3.8,
        },
        StoryBeat::Victory => StoryScene {
            lines: &[
                StoryDialogueLine {
                    speaker: "Malrec, Keeper of Ash",
                    text: "Ash remembers me...",
                },
                StoryDialogueLine {
                    speaker: "Steward Edda",
                    text: "And the living will remember who ended your oath.",
                },
                StoryDialogueLine {
                    speaker: "Quartermaster Vey",
                    text: "The broken seal points north. The Hollow Road is open, and the Bastion is answering.",
                },
            ],
            next_action: "continue along the Hollow Road",
            line_duration_secs: 4.0,
        },
        StoryBeat::HollowArrival => StoryScene {
            lines: &[
                StoryDialogueLine {
                    speaker: "Steward Edda",
                    text: "The Hollow Road ends here. Nyxara has woven Malrec's stolen seal into a choir of the dead.",
                },
                StoryDialogueLine {
                    speaker: "Quartermaster Vey",
                    text: "The vanguard stands between us and the reliquary. Make them quiet enough for the ward to speak.",
                },
            ],
            next_action: "break the Hollowbound vanguard",
            line_duration_secs: 3.9,
        },
        StoryBeat::ChoirCacheRevealed => StoryScene {
            lines: &[
                StoryDialogueLine {
                    speaker: "Steward Edda",
                    text: "The vanguard breaks. That choir reliquary carries the first counter-seal.",
                },
                StoryDialogueLine {
                    speaker: "The Hollow Choir",
                    text: "Give us your name, Sentinel. We will sing you home.",
                },
            ],
            next_action: "open the choir reliquary",
            line_duration_secs: 3.4,
        },
        StoryBeat::ChoirSealRecovered => StoryScene {
            lines: &[
                StoryDialogueLine {
                    speaker: "Reliquary Sentinel",
                    text: "The seal is cold. It is trying to speak.",
                },
                StoryDialogueLine {
                    speaker: "Steward Edda",
                    text: "Do not listen. Its echo has opened the bell crypt; Vesper guards the second.",
                },
            ],
            next_action: "enter the bell crypt",
            line_duration_secs: 3.5,
        },
        StoryBeat::BellCryptOpened => StoryScene {
            lines: &[
                StoryDialogueLine {
                    speaker: "Vesper, Bell Warden",
                    text: "One toll for Malrec. One for Nyxara. The third is yours.",
                },
                StoryDialogueLine {
                    speaker: "Steward Edda",
                    text: "Cross the crypt before he rings it. Silence Vesper and his choir.",
                },
            ],
            next_action: "silence Vesper and the choir",
            line_duration_secs: 3.7,
        },
        StoryBeat::BellWardenSlain => StoryScene {
            lines: &[
                StoryDialogueLine {
                    speaker: "Vesper, Bell Warden",
                    text: "The bell... still rings below...",
                },
                StoryDialogueLine {
                    speaker: "Steward Edda",
                    text: "Because Nyxara's soul font is the clapper. Take his seal to it.",
                },
            ],
            next_action: "carry the second seal to the soul font",
            line_duration_secs: 3.5,
        },
        StoryBeat::SoulFontDemand => StoryScene {
            lines: &[
                StoryDialogueLine {
                    speaker: "Nyxara, Hollow Matron",
                    text: "Every soul you freed has come to me. Break the font, and you only give them teeth.",
                },
                StoryDialogueLine {
                    speaker: "Steward Edda",
                    text: "The final seal is inside that chorus. Shatter it and face what wakes.",
                },
            ],
            next_action: "shatter the soul font",
            line_duration_secs: 3.9,
        },
        StoryBeat::MatronAwakened => StoryScene {
            lines: &[
                StoryDialogueLine {
                    speaker: "Nyxara, Hollow Matron",
                    text: "Malrec wanted dominion. I want no voice to die alone.",
                },
                StoryDialogueLine {
                    speaker: "Steward Edda",
                    text: "Her crown carries the seal. Claim the boon, interrupt the Crescendo, and tear it free.",
                },
            ],
            next_action: "choose a boon and break Nyxara",
            line_duration_secs: 4.1,
        },
        StoryBeat::MatronBloodied => StoryScene {
            lines: &[
                StoryDialogueLine {
                    speaker: "Nyxara, Hollow Matron",
                    text: "Choir, rise! Drown the blade in every final breath.",
                },
                StoryDialogueLine {
                    speaker: "Steward Edda",
                    text: "The chorus is fractured. Break her before it reforms.",
                },
            ],
            next_action: "interrupt the Hollow Crescendo",
            line_duration_secs: 3.8,
        },
        StoryBeat::BastionVictory => StoryScene {
            lines: &[
                StoryDialogueLine {
                    speaker: "Nyxara, Hollow Matron",
                    text: "Silence... was never empty.",
                },
                StoryDialogueLine {
                    speaker: "Reliquary Sentinel",
                    text: "No. It belongs to the living.",
                },
                StoryDialogueLine {
                    speaker: "Steward Edda",
                    text: "Both seals answer. The Bastion is quiet, and the Reliquary campaign is complete.",
                },
            ],
            next_action: "claim the campaign cache",
            line_duration_secs: 4.0,
        },
        StoryBeat::CursedShrine => StoryScene {
            lines: &[StoryDialogueLine {
                speaker: "Reliquary Whisper",
                text: "Blood for passage. Courage for debt.",
            }],
            next_action: "kill all ambushers for the offering",
            line_duration_secs: 4.2,
        },
        StoryBeat::BloodObelisk => StoryScene {
            lines: &[StoryDialogueLine {
                speaker: "Reliquary Whisper",
                text: "The obelisk knows the names you buried. Feed it enemies, or join them.",
            }],
            next_action: "finish the rite before it turns",
            line_duration_secs: 4.2,
        },
        StoryBeat::ReliquaryVault => StoryScene {
            lines: &[StoryDialogueLine {
                speaker: "Quartermaster Vey",
                text: "Ancient locks, fresh greed. Take what helps the oath and leave before the vault chooses a keeper.",
            }],
            next_action: "loot the vault and spend at town",
            line_duration_secs: 4.2,
        },
        StoryBeat::EmberRift => StoryScene {
            lines: &[StoryDialogueLine {
                speaker: "Reliquary Whisper",
                text: "A failed oath has torn the road open. Its hunters already know your scent.",
            }],
            next_action: "seal the rift before the timer breaks",
            line_duration_secs: 4.2,
        },
        StoryBeat::AshenPylon => StoryScene {
            lines: &[StoryDialogueLine {
                speaker: "Quartermaster Vey",
                text: "That pylon is overloading the relic. Spend its fire now, before it spends you.",
            }],
            next_action: "chain kills while the pylon burns",
            line_duration_secs: 4.2,
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn both_chapters_define_complete_mainline_scenes() {
        for chapter in [ChapterId::AshenReliquary, ChapterId::HollowBastion] {
            let beats = chapter_story_beats(chapter);
            assert_eq!(beats.len(), 9);
            assert_eq!(
                beats.first().copied(),
                Some(match chapter {
                    ChapterId::AshenReliquary => StoryBeat::Arrival,
                    ChapterId::HollowBastion => StoryBeat::HollowArrival,
                })
            );
            assert_eq!(
                beats.last().copied(),
                Some(match chapter {
                    ChapterId::AshenReliquary => StoryBeat::Victory,
                    ChapterId::HollowBastion => StoryBeat::BastionVictory,
                })
            );
            for beat in beats {
                let scene = story_scene(*beat);
                assert!(
                    scene.lines.len() >= 2,
                    "{beat:?} should contain an exchange"
                );
                assert!(!scene.next_action.is_empty());
                assert!(scene.line_duration_secs >= 3.0);
            }
        }
    }

    #[test]
    fn boss_arcs_include_pre_fight_mid_fight_and_aftermath_dialogue() {
        for beats in [
            [
                StoryBeat::KeeperAwakened,
                StoryBeat::KeeperBloodied,
                StoryBeat::Victory,
            ],
            [
                StoryBeat::MatronAwakened,
                StoryBeat::MatronBloodied,
                StoryBeat::BastionVictory,
            ],
        ] {
            for beat in beats {
                let scene = story_scene(beat);
                assert!(
                    scene.lines.iter().any(|line| {
                        line.speaker.contains("Malrec") || line.speaker.contains("Nyxara")
                    }),
                    "{beat:?} should include the chapter boss"
                );
            }
        }
    }

    #[test]
    fn chapter_endings_bridge_act_one_and_close_act_two() {
        assert!(chapter_ending_summary(ChapterId::AshenReliquary).contains("Hollow Road"));
        assert!(chapter_ending_summary(ChapterId::HollowBastion).contains("campaign is complete"));
    }
}
