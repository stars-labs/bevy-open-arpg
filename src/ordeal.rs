use crate::Difficulty;
use bevy::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Resource, Debug, Clone, Copy, Eq, PartialEq, Serialize, Deserialize)]
pub struct ChapterModifier {
    pub kind: ChapterModifierKind,
    #[serde(default)]
    pub affix: OrdealAffix,
}

impl Default for ChapterModifier {
    fn default() -> Self {
        Self {
            kind: ChapterModifierKind::AshenEchoes,
            affix: OrdealAffix::None,
        }
    }
}

#[derive(Debug, Clone, Copy, Default, Eq, PartialEq, Serialize, Deserialize)]
pub enum ChapterModifierKind {
    #[default]
    AshenEchoes,
    BloodTithe,
    Emberstorm,
    TormentBrand,
}

#[derive(Debug, Clone, Copy, Default, Eq, PartialEq, Serialize, Deserialize)]
pub enum OrdealAffix {
    #[default]
    None,
    AshenHunger,
    GlassRelics,
    TreasureFever,
    CinderVeins,
}

impl ChapterModifierKind {
    pub fn label(self) -> &'static str {
        match self {
            Self::AshenEchoes => "Ashen Echoes",
            Self::BloodTithe => "Blood Tithe",
            Self::Emberstorm => "Emberstorm",
            Self::TormentBrand => "Torment Brand",
        }
    }

    pub fn summary(self) -> &'static str {
        match self {
            Self::AshenEchoes => "standard enemy rules and rewards",
            Self::BloodTithe => "harder enemy hits, richer monster rewards",
            Self::Emberstorm => "deadlier enemies and vents, highest rewards",
            Self::TormentBrand => "brutal enemies, lethal vents, and torment-grade rewards",
        }
    }

    pub fn enemy_health_multiplier(self) -> f32 {
        match self {
            Self::AshenEchoes => 1.0,
            Self::BloodTithe => 1.08,
            Self::Emberstorm => 1.16,
            Self::TormentBrand => 1.28,
        }
    }

    pub fn enemy_damage_multiplier(self) -> f32 {
        match self {
            Self::AshenEchoes => 1.0,
            Self::BloodTithe => 1.12,
            Self::Emberstorm => 1.18,
            Self::TormentBrand => 1.30,
        }
    }

    pub fn hazard_damage_multiplier(self) -> f32 {
        match self {
            Self::AshenEchoes => 1.0,
            Self::BloodTithe => 1.0,
            Self::Emberstorm => 1.30,
            Self::TormentBrand => 1.55,
        }
    }

    pub fn reward_multiplier(self) -> f32 {
        match self {
            Self::AshenEchoes => 1.0,
            Self::BloodTithe => 1.18,
            Self::Emberstorm => 1.32,
            Self::TormentBrand => 1.55,
        }
    }

    #[allow(dead_code)]
    pub fn scale_reward(self, reward: u32) -> u32 {
        ((reward as f32) * self.reward_multiplier())
            .round()
            .max(1.0) as u32
    }
}

impl OrdealAffix {
    pub fn label(self) -> &'static str {
        match self {
            Self::None => "No Affix",
            Self::AshenHunger => "Ashen Hunger",
            Self::GlassRelics => "Glass Relics",
            Self::TreasureFever => "Treasure Fever",
            Self::CinderVeins => "Cinder Veins",
        }
    }

    pub fn summary(self) -> &'static str {
        match self {
            Self::None => "no rotating secondary rule",
            Self::AshenHunger => "enemies hit harder, rewards rise",
            Self::GlassRelics => "enemies endure longer, rewards rise",
            Self::TreasureFever => "bonus rewards without extra combat pressure",
            Self::CinderVeins => "vents and hazards bite harder, rewards rise",
        }
    }

    pub fn enemy_health_multiplier(self) -> f32 {
        match self {
            Self::GlassRelics => 1.10,
            _ => 1.0,
        }
    }

    pub fn enemy_damage_multiplier(self) -> f32 {
        match self {
            Self::AshenHunger => 1.08,
            _ => 1.0,
        }
    }

    pub fn hazard_damage_multiplier(self) -> f32 {
        match self {
            Self::CinderVeins => 1.15,
            _ => 1.0,
        }
    }

    pub fn reward_multiplier(self) -> f32 {
        match self {
            Self::None => 1.0,
            Self::AshenHunger => 1.08,
            Self::GlassRelics => 1.06,
            Self::TreasureFever => 1.15,
            Self::CinderVeins => 1.10,
        }
    }
}

impl ChapterModifier {
    pub fn label(self) -> String {
        if self.affix == OrdealAffix::None {
            self.kind.label().to_string()
        } else {
            format!("{} + {}", self.kind.label(), self.affix.label())
        }
    }

    pub fn summary(self) -> String {
        if self.affix == OrdealAffix::None {
            self.kind.summary().to_string()
        } else {
            format!("{}; {}", self.kind.summary(), self.affix.summary())
        }
    }

    pub fn enemy_health_multiplier(self) -> f32 {
        self.kind.enemy_health_multiplier() * self.affix.enemy_health_multiplier()
    }

    pub fn enemy_damage_multiplier(self) -> f32 {
        self.kind.enemy_damage_multiplier() * self.affix.enemy_damage_multiplier()
    }

    pub fn hazard_damage_multiplier(self) -> f32 {
        self.kind.hazard_damage_multiplier() * self.affix.hazard_damage_multiplier()
    }

    pub fn reward_multiplier(self) -> f32 {
        self.kind.reward_multiplier() * self.affix.reward_multiplier()
    }

    pub fn scale_reward(self, reward: u32) -> u32 {
        ((reward as f32) * self.reward_multiplier())
            .round()
            .max(1.0) as u32
    }
}

pub fn modifier_for_difficulty(difficulty: Difficulty) -> ChapterModifierKind {
    match difficulty {
        Difficulty::Normal => ChapterModifierKind::AshenEchoes,
        Difficulty::Nightmare => ChapterModifierKind::BloodTithe,
        Difficulty::Hell => ChapterModifierKind::Emberstorm,
        Difficulty::Torment => ChapterModifierKind::TormentBrand,
    }
}

pub fn modifier_for_run(difficulty: Difficulty, clear_count: u32) -> ChapterModifier {
    ChapterModifier {
        kind: modifier_for_difficulty(difficulty),
        affix: affix_for_run(difficulty, clear_count),
    }
}

pub fn affix_for_run(difficulty: Difficulty, clear_count: u32) -> OrdealAffix {
    if difficulty == Difficulty::Normal && clear_count == 0 {
        return OrdealAffix::None;
    }
    let difficulty_offset = match difficulty {
        Difficulty::Normal => 0,
        Difficulty::Nightmare => 1,
        Difficulty::Hell => 2,
        Difficulty::Torment => 3,
    };
    match (clear_count + difficulty_offset) % 4 {
        0 => OrdealAffix::AshenHunger,
        1 => OrdealAffix::GlassRelics,
        2 => OrdealAffix::TreasureFever,
        _ => OrdealAffix::CinderVeins,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn difficulty_selects_escalating_chapter_modifiers() {
        assert_eq!(
            modifier_for_difficulty(Difficulty::Normal),
            ChapterModifierKind::AshenEchoes
        );
        assert_eq!(
            modifier_for_difficulty(Difficulty::Nightmare),
            ChapterModifierKind::BloodTithe
        );
        assert_eq!(
            modifier_for_difficulty(Difficulty::Hell),
            ChapterModifierKind::Emberstorm
        );
        assert_eq!(
            modifier_for_difficulty(Difficulty::Torment),
            ChapterModifierKind::TormentBrand
        );
        assert!(
            ChapterModifierKind::TormentBrand.enemy_damage_multiplier()
                > ChapterModifierKind::Emberstorm.enemy_damage_multiplier()
        );
        assert!(
            ChapterModifierKind::TormentBrand
                .summary()
                .contains("torment")
        );
    }

    #[test]
    fn reward_scaling_rounds_and_never_drops_below_one() {
        assert_eq!(
            ChapterModifier {
                kind: ChapterModifierKind::BloodTithe,
                affix: OrdealAffix::None,
            }
            .scale_reward(100),
            118
        );
        assert_eq!(
            ChapterModifier {
                kind: ChapterModifierKind::TormentBrand,
                affix: OrdealAffix::None,
            }
            .scale_reward(100),
            155
        );
        assert_eq!(
            ChapterModifier {
                kind: ChapterModifierKind::Emberstorm,
                affix: OrdealAffix::None,
            }
            .scale_reward(0),
            1
        );
    }

    #[test]
    fn run_modifiers_rotate_secondary_affixes_after_first_normal_run() {
        assert_eq!(
            modifier_for_run(Difficulty::Normal, 0).affix,
            OrdealAffix::None
        );
        assert_eq!(
            modifier_for_run(Difficulty::Normal, 1).affix,
            OrdealAffix::GlassRelics
        );
        assert_eq!(
            modifier_for_run(Difficulty::Nightmare, 0).affix,
            OrdealAffix::GlassRelics
        );
        assert_eq!(
            modifier_for_run(Difficulty::Torment, 1).affix,
            OrdealAffix::AshenHunger
        );
    }

    #[test]
    fn combined_modifier_scales_pressure_and_rewards() {
        let modifier = ChapterModifier {
            kind: ChapterModifierKind::TormentBrand,
            affix: OrdealAffix::CinderVeins,
        };

        assert!(modifier.label().contains("Cinder Veins"));
        assert!(modifier.summary().contains("hazards"));
        assert!(
            modifier.hazard_damage_multiplier()
                > ChapterModifierKind::TormentBrand.hazard_damage_multiplier()
        );
        assert_eq!(modifier.scale_reward(100), 171);
    }
}
