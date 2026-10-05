//! Hero classes and their data-driven skill books (`assets/data/classes.ron`).
use crate::player::PlayerAttackMotion;
use bevy::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HeroClass {
    #[default]
    Warden,
    Stalker,
    Embercaller,
}

impl HeroClass {
    pub const ALL: [Self; 3] = [Self::Warden, Self::Stalker, Self::Embercaller];

    pub fn index(self) -> usize {
        match self {
            Self::Warden => 0,
            Self::Stalker => 1,
            Self::Embercaller => 2,
        }
    }
}

/// The hero's class: chosen on the menu or read from a save, and kept across chapters.
#[derive(Resource, Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct SelectedClass(pub HeroClass);

#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SkillSlot {
    Mobility,
    Area,
    Finisher,
}

impl SkillSlot {
    pub const ALL: [Self; 3] = [Self::Mobility, Self::Area, Self::Finisher];

    pub fn index(self) -> usize {
        match self {
            Self::Mobility => 0,
            Self::Area => 1,
            Self::Finisher => 2,
        }
    }

    pub fn key(self) -> KeyCode {
        match self {
            Self::Mobility => KeyCode::KeyQ,
            Self::Area => KeyCode::KeyE,
            Self::Finisher => KeyCode::KeyR,
        }
    }

    pub fn key_label(self) -> &'static str {
        match self {
            Self::Mobility => "Q",
            Self::Area => "E",
            Self::Finisher => "R",
        }
    }
}

/// Six effects shared by every class; each skill offers two of them.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Rune {
    Expose,
    Hemorrhage,
    Cleanse,
    Reap,
    Ember,
    Frost,
}

/// Health restored per enemy struck by a Reap skill.
pub const REAP_HEAL_PER_HIT: f32 = 8.0;

impl Rune {
    pub fn label(self) -> &'static str {
        match self {
            Self::Expose => "Expose",
            Self::Hemorrhage => "Hemorrhage",
            Self::Cleanse => "Cleanse",
            Self::Reap => "Reap",
            Self::Ember => "Ember",
            Self::Frost => "Frost",
        }
    }

    pub fn label_zh(self) -> &'static str {
        match self {
            Self::Expose => "破绽",
            Self::Hemorrhage => "血伤",
            Self::Cleanse => "净化",
            Self::Reap => "收割",
            Self::Ember => "余烬",
            Self::Frost => "寒霜",
        }
    }

    /// Utility runes pay for their effect with damage.
    pub fn damage_multiplier(self) -> f32 {
        match self {
            Self::Expose | Self::Cleanse | Self::Ember => 1.0,
            Self::Reap => 0.9,
            Self::Hemorrhage => 0.82,
            Self::Frost => 0.72,
        }
    }

    /// Reap shortens repositioning in exchange for healing.
    pub fn movement_multiplier(self) -> f32 {
        if self == Self::Reap { 0.82 } else { 1.0 }
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ResourceKind {
    #[default]
    Fury,
    Malice,
    Arcana,
}

impl ResourceKind {
    pub fn label(self) -> &'static str {
        match self {
            Self::Fury => "fury",
            Self::Malice => "malice",
            Self::Arcana => "arcana",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Movement {
    None,
    /// Dash a fixed distance toward the cursor, striking on arrival.
    Toward(f32),
    /// Leap away from the cursor, striking back toward it.
    Away(f32),
    /// Land on the cursor (clamped to this distance), striking on arrival.
    Leap(f32),
    /// Vanish to the cursor (clamped to this distance), striking where the hero stood.
    Blink(f32),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProjectileArt {
    BoneArrow,
    EmberBolt,
    AshLance,
    SunderWave,
}

#[derive(Clone, Copy, Debug, PartialEq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StrikeShape {
    /// A frontal melee arc reaching `range` from the hero.
    Arc { range: f32 },
    /// Everything within `radius` of the hero.
    Radial { radius: f32 },
    Projectiles {
        art: ProjectileArt,
        count: u32,
        spread_deg: f32,
        speed: f32,
        range: f32,
        /// Half-width of the swept path.
        width: f32,
        pierce: bool,
        /// Zero strikes only the enemy touched; otherwise everything around the impact.
        blast_radius: f32,
    },
    /// A telegraphed burst at the cursor, clamped to `cast_range`.
    GroundBlast {
        cast_range: f32,
        radius: f32,
        delay: f32,
    },
}

impl StrikeShape {
    /// How close the hero must be before this strike can reach a target.
    pub fn reach(self) -> f32 {
        match self {
            Self::Arc { range } => range,
            Self::Radial { radius } => radius,
            Self::Projectiles { range, .. } => range * 0.92,
            Self::GroundBlast { cast_range, .. } => cast_range,
        }
    }

    pub fn is_melee(self) -> bool {
        matches!(self, Self::Arc { .. })
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Passive {
    /// Deal more damage while wounded.
    Unbowed {
        health_fraction: f32,
        damage_multiplier: f32,
    },
    /// Deal more damage to distant enemies.
    LongShot {
        min_distance: f32,
        damage_multiplier: f32,
    },
    /// Rebuild a small barrier after avoiding damage for a while.
    EmberVeil {
        delay_secs: f32,
        barrier_per_sec: f32,
        cap: f32,
    },
}

fn one() -> f32 {
    1.0
}

#[derive(Clone, Debug, Deserialize)]
pub struct ResourceDef {
    pub kind: ResourceKind,
    pub max: f32,
    pub start: f32,
    pub regen_per_sec: f32,
    /// Granted per enemy damaged by a basic attack.
    pub basic_gain: f32,
}

#[derive(Clone, Debug, Deserialize)]
pub struct BasicAttackDef {
    pub name: String,
    pub name_zh: String,
    pub damage: f32,
    /// Seconds for one full swing or shot before haste.
    pub cycle: f32,
    pub motion: PlayerAttackMotion,
    pub strike: StrikeShape,
}

#[derive(Clone, Debug, Deserialize)]
pub struct SkillDef {
    pub name: String,
    pub name_zh: String,
    pub summary: String,
    pub summary_zh: String,
    pub unlock_level: u32,
    pub cost: f32,
    pub cooldown: f32,
    pub damage: f32,
    /// Share of flat gear damage added to `damage`.
    #[serde(default = "one")]
    pub bonus_scale: f32,
    pub impact: f32,
    pub motion: PlayerAttackMotion,
    pub cast_secs: f32,
    /// The cast animation speeds up with cooldown reduction.
    #[serde(default)]
    pub hasted: bool,
    /// Resolve at the animation's contact frame instead of the key press.
    #[serde(default)]
    pub on_contact: bool,
    /// Refuse to cast (and refund nothing) unless the strike would reach something.
    #[serde(default)]
    pub requires_target: bool,
    #[serde(default)]
    pub barrier: f32,
    pub movement: Movement,
    pub strike: StrikeShape,
    pub runes: (Rune, Rune),
}

impl SkillDef {
    pub fn rune(&self, alt: bool) -> Rune {
        if alt { self.runes.1 } else { self.runes.0 }
    }
}

#[derive(Clone, Debug, Deserialize)]
pub struct SlotSkills {
    pub a: SkillDef,
    pub b: SkillDef,
}

impl SlotSkills {
    pub fn get(&self, alt: bool) -> &SkillDef {
        if alt { &self.b } else { &self.a }
    }
}

#[derive(Clone, Debug, Deserialize)]
pub struct ClassDef {
    pub class: HeroClass,
    pub name: String,
    pub name_zh: String,
    pub role: String,
    pub role_zh: String,
    pub summary: String,
    pub summary_zh: String,
    pub passive_name: String,
    pub passive_name_zh: String,
    pub passive_summary: String,
    pub passive_summary_zh: String,
    /// One (easy) to three (demanding), shown on the class card.
    pub difficulty: u32,
    pub resource: ResourceDef,
    pub max_health: f32,
    pub move_speed: f32,
    pub passive: Passive,
    pub starting_weapon: String,
    pub basic: BasicAttackDef,
    pub mobility: SlotSkills,
    pub area: SlotSkills,
    pub finisher: SlotSkills,
}

impl ClassDef {
    pub fn slot(&self, slot: SkillSlot) -> &SlotSkills {
        match slot {
            SkillSlot::Mobility => &self.mobility,
            SkillSlot::Area => &self.area,
            SkillSlot::Finisher => &self.finisher,
        }
    }

    pub fn skill(&self, slot: SkillSlot, alt: bool) -> &SkillDef {
        self.slot(slot).get(alt)
    }

    /// The skill and rune the loadout has in `slot`.
    pub fn equipped(&self, loadout: &SkillLoadout, slot: SkillSlot) -> (&SkillDef, Rune) {
        let alt = loadout.alt[slot.index()];
        let skill = self.skill(slot, alt);
        (skill, loadout.rune(skill, slot, alt))
    }

    /// Skills first usable at exactly `level`, in slot order.
    pub fn unlocked_at(&self, level: u32) -> impl Iterator<Item = (SkillSlot, &SkillDef)> {
        SkillSlot::ALL.into_iter().flat_map(move |slot| {
            [false, true]
                .into_iter()
                .map(move |alt| (slot, self.skill(slot, alt)))
                .filter(move |(_, skill)| skill.unlock_level == level)
        })
    }
}

#[derive(Clone, Debug, Deserialize, Resource)]
pub struct ClassCatalog {
    pub classes: Vec<ClassDef>,
}

impl ClassCatalog {
    pub fn get(&self, class: HeroClass) -> &ClassDef {
        self.classes
            .iter()
            .find(|def| def.class == class)
            .unwrap_or_else(|| panic!("classes.ron has no entry for {class:?}"))
    }
}

/// Which skill sits in each slot and which rune every skill carries.
#[derive(Component, Clone, Copy, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub struct SkillLoadout {
    /// Per slot: the second skill is equipped instead of the first.
    pub alt: [bool; 3],
    /// Per slot and skill: the second rune is chosen instead of the first.
    pub rune_alt: [[bool; 2]; 3],
}

impl SkillLoadout {
    pub fn rune(&self, skill: &SkillDef, slot: SkillSlot, alt: bool) -> Rune {
        skill.rune(self.rune_alt[slot.index()][usize::from(alt)])
    }

    pub fn toggle_rune(&mut self, slot: SkillSlot) {
        let alt = usize::from(self.alt[slot.index()]);
        let chosen = &mut self.rune_alt[slot.index()][alt];
        *chosen = !*chosen;
    }
}

#[cfg(test)]
pub(crate) fn test_catalog() -> ClassCatalog {
    ron::from_str(include_str!("../assets/data/classes.ron")).expect("classes.ron parses")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_class_fills_three_slots_with_two_skills_and_two_distinct_runes() {
        let catalog = test_catalog();
        assert_eq!(catalog.classes.len(), HeroClass::ALL.len());
        for class in HeroClass::ALL {
            let def = catalog.get(class);
            for slot in SkillSlot::ALL {
                for alt in [false, true] {
                    let skill = def.skill(slot, alt);
                    assert_ne!(skill.runes.0, skill.runes.1, "{}", skill.name);
                    assert!(skill.cost > 0.0 && skill.cooldown > 0.0, "{}", skill.name);
                    assert!(skill.cost <= def.resource.max, "{}", skill.name);
                    assert!(!skill.name_zh.is_empty() && !skill.summary_zh.is_empty());
                }
            }
        }
    }

    #[test]
    fn each_level_from_one_to_six_unlocks_exactly_one_skill() {
        let catalog = test_catalog();
        for class in HeroClass::ALL {
            let def = catalog.get(class);
            for level in 1..=crate::player::CHAPTER_LEVEL_CAP {
                assert_eq!(def.unlocked_at(level).count(), 1, "{class:?} level {level}");
            }
            // The first skill of every slot arrives before its alternative.
            for slot in SkillSlot::ALL {
                assert!(def.skill(slot, false).unlock_level < def.skill(slot, true).unlock_level);
            }
        }
    }

    #[test]
    fn every_class_can_open_with_its_first_finisher() {
        let catalog = test_catalog();
        for class in HeroClass::ALL {
            let def = catalog.get(class);
            let opener = def.skill(SkillSlot::Finisher, false);
            assert_eq!(opener.unlock_level, 1);
            assert!(def.resource.start >= opener.cost, "{class:?}");
            assert!(def.resource.start <= def.resource.max);
        }
    }

    #[test]
    fn warden_starting_fury_supports_three_ruptures() {
        let catalog = test_catalog();
        let warden = catalog.get(HeroClass::Warden);
        assert!(warden.resource.start >= warden.skill(SkillSlot::Finisher, false).cost * 3.0);
    }

    #[test]
    fn ranged_classes_trade_sustained_single_target_damage_for_reach() {
        let catalog = test_catalog();
        let dps = |class| {
            let basic = &catalog.get(class).basic;
            basic.damage / basic.cycle
        };
        let warden = dps(HeroClass::Warden);
        for class in [HeroClass::Stalker, HeroClass::Embercaller] {
            let def = catalog.get(class);
            assert!(dps(class) < warden, "{class:?}");
            assert!(dps(class) > warden * 0.5, "{class:?}");
            assert!(def.basic.strike.reach() > catalog.get(HeroClass::Warden).basic.strike.reach());
            assert!(def.max_health < catalog.get(HeroClass::Warden).max_health);
        }
    }

    #[test]
    fn loadout_tracks_a_rune_per_skill_rather_than_per_slot() {
        let catalog = test_catalog();
        let def = catalog.get(HeroClass::Warden);
        let mut loadout = SkillLoadout::default();
        loadout.toggle_rune(SkillSlot::Finisher);
        assert_eq!(
            def.equipped(&loadout, SkillSlot::Finisher).1,
            Rune::Hemorrhage
        );
        loadout.alt[SkillSlot::Finisher.index()] = true;
        let (skill, rune) = def.equipped(&loadout, SkillSlot::Finisher);
        assert_eq!(rune, skill.runes.0);
    }

    #[test]
    fn utility_runes_cost_damage_and_only_reap_shortens_movement() {
        assert_eq!(Rune::Expose.damage_multiplier(), 1.0);
        assert_eq!(Rune::Ember.damage_multiplier(), 1.0);
        assert!(Rune::Hemorrhage.damage_multiplier() < 1.0);
        assert!(Rune::Frost.damage_multiplier() < Rune::Hemorrhage.damage_multiplier());
        assert!(Rune::Reap.movement_multiplier() < 1.0);
        assert_eq!(Rune::Cleanse.movement_multiplier(), 1.0);
    }
}
