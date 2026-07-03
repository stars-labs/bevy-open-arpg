use crate::player::LegendaryPower;
use bevy::prelude::*;
use serde::Deserialize;
use std::{collections::HashMap, fs};

#[derive(Clone, Debug, Deserialize, Resource)]
pub struct PlayerTuning {
    pub move_speed: f32,
    pub max_health: f32,
    pub basic_damage: f32,
    pub basic_range: f32,
    pub dash_damage: f32,
    pub dash_cooldown: f32,
    pub dash_distance: f32,
    pub nova_damage: f32,
    pub nova_cooldown: f32,
    pub nova_radius: f32,
}

#[derive(Clone, Debug, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum EnemyAttackKind {
    Melee,
    Projectile,
    Shockwave,
}

#[derive(Clone, Debug, Deserialize)]
pub struct EnemyTuning {
    pub id: String,
    pub display_name: String,
    pub max_health: f32,
    pub move_speed: f32,
    pub attack_damage: f32,
    pub attack_kind: EnemyAttackKind,
    pub attack_range: f32,
    pub attack_cooldown: f32,
    pub aggro_range: f32,
    pub gold_min: u32,
    pub gold_max: u32,
    pub xp_reward: u32,
    pub model: String,
}

#[derive(Clone, Debug, Deserialize, Resource)]
pub struct EnemyCatalog {
    pub enemies: Vec<EnemyTuning>,
    #[serde(skip)]
    by_id: HashMap<String, usize>,
}

#[derive(Clone, Debug, Deserialize)]
pub struct LootEntry {
    pub name: String,
    pub quality: String,
    pub weight: u32,
    pub damage_bonus: f32,
    pub crit_chance: f32,
    pub health_bonus: f32,
    pub armor_bonus: f32,
    #[serde(default)]
    pub legendary_power: LegendaryPower,
    pub potions: u32,
}

#[derive(Clone, Debug, Deserialize, Resource)]
pub struct LootTable {
    pub weapons: Vec<LootEntry>,
}

impl EnemyCatalog {
    pub fn hydrate(mut self) -> Self {
        self.by_id = self
            .enemies
            .iter()
            .enumerate()
            .map(|(index, enemy)| (enemy.id.clone(), index))
            .collect();
        self
    }

    pub fn get(&self, id: &str) -> Option<&EnemyTuning> {
        self.by_id
            .get(id)
            .and_then(|index| self.enemies.get(*index))
    }
}

pub struct GameDataPlugin;

impl Plugin for GameDataPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(load_ron::<PlayerTuning>("assets/data/player.ron"))
            .insert_resource(load_ron::<EnemyCatalog>("assets/data/enemies.ron").hydrate())
            .insert_resource(load_ron::<LootTable>("assets/data/loot.ron"));
    }
}

fn load_ron<T>(path: &str) -> T
where
    T: for<'de> Deserialize<'de>,
{
    let content =
        fs::read_to_string(path).unwrap_or_else(|err| panic!("failed to read {path}: {err}"));
    ron::from_str(&content).unwrap_or_else(|err| panic!("failed to parse {path}: {err}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn enemy_catalog_indexes_by_id() {
        let catalog = EnemyCatalog {
            enemies: vec![EnemyTuning {
                id: "keeper".to_string(),
                display_name: "Malrec".to_string(),
                max_health: 260.0,
                move_speed: 1.5,
                attack_damage: 25.0,
                attack_kind: EnemyAttackKind::Shockwave,
                attack_range: 1.75,
                attack_cooldown: 1.6,
                aggro_range: 13.0,
                gold_min: 55,
                gold_max: 90,
                xp_reward: 120,
                model: "butcher.glb".to_string(),
            }],
            by_id: HashMap::default(),
        }
        .hydrate();

        assert_eq!(catalog.get("keeper").unwrap().display_name, "Malrec");
        assert!(catalog.get("missing").is_none());
    }

    #[test]
    fn runtime_enemy_roster_covers_fast_ranged_and_heavy_roles() {
        let catalog = load_ron::<EnemyCatalog>("assets/data/enemies.ron").hydrate();

        let guard = catalog.get("skeleton").expect("baseline guard");
        let stalker = catalog.get("bone_stalker").expect("fast stalker");
        let marksman = catalog.get("ashen_marksman").expect("ranged marksman");
        let brute = catalog.get("reliquary_brute").expect("heavy brute");

        assert_eq!(stalker.attack_kind, EnemyAttackKind::Melee);
        assert!(stalker.move_speed > guard.move_speed + 0.6);
        assert!(stalker.attack_cooldown < guard.attack_cooldown);

        assert_eq!(marksman.attack_kind, EnemyAttackKind::Projectile);
        assert!(marksman.attack_range > guard.attack_range * 3.0);
        assert!(marksman.attack_damage > guard.attack_damage);

        assert_eq!(brute.attack_kind, EnemyAttackKind::Shockwave);
        assert!(brute.max_health > guard.max_health * 1.8);
        assert!(brute.attack_damage > marksman.attack_damage);
    }

    #[test]
    fn loot_entries_include_potion_rewards() {
        let table: LootTable = ron::from_str(
            r#"(
                weapons: [
                    (
                        name: "Moonforged Cleaver",
                        quality: "rare",
                        weight: 24,
                        damage_bonus: 9.0,
                        crit_chance: 0.08,
                        health_bonus: 10.0,
                        armor_bonus: 6.0,
                        legendary_power: emberbrand,
                        potions: 1,
                    ),
                    (
                        name: "Frostwake Reliquary Edge",
                        quality: "legendary",
                        weight: 5,
                        damage_bonus: 14.0,
                        crit_chance: 0.16,
                        health_bonus: 18.0,
                        armor_bonus: 22.0,
                        legendary_power: frostbrand,
                        potions: 2,
                    ),
                    (
                        name: "Stormcall Reliquary Brand",
                        quality: "legendary",
                        weight: 5,
                        damage_bonus: 15.0,
                        crit_chance: 0.15,
                        health_bonus: 16.0,
                        armor_bonus: 20.0,
                        legendary_power: stormbrand,
                        potions: 2,
                    ),
                    (
                        name: "Soulreaver Reliquary Fang",
                        quality: "legendary",
                        weight: 5,
                        damage_bonus: 13.0,
                        crit_chance: 0.18,
                        health_bonus: 26.0,
                        armor_bonus: 16.0,
                        legendary_power: soulreaver,
                        potions: 2,
                    ),
                    (
                        name: "Aegis-Saint Reliquary Guard",
                        quality: "legendary",
                        weight: 5,
                        damage_bonus: 12.0,
                        crit_chance: 0.17,
                        health_bonus: 32.0,
                        armor_bonus: 26.0,
                        legendary_power: aegisbrand,
                        potions: 2,
                    ),
                    (
                        name: "Ancient Ashen Reliquary Reaver",
                        quality: "ancient",
                        weight: 2,
                        damage_bonus: 22.0,
                        crit_chance: 0.20,
                        health_bonus: 34.0,
                        armor_bonus: 28.0,
                        legendary_power: stormbrand,
                        potions: 3,
                    ),
                    (
                        name: "Ancient Aegis-Saint Reliquary Bulwark",
                        quality: "ancient",
                        weight: 2,
                        damage_bonus: 18.0,
                        crit_chance: 0.22,
                        health_bonus: 48.0,
                        armor_bonus: 38.0,
                        legendary_power: aegisbrand,
                        potions: 3,
                    ),
                ],
            )"#,
        )
        .unwrap();

        assert_eq!(table.weapons[0].potions, 1);
        assert_eq!(table.weapons[0].damage_bonus, 9.0);
        assert_eq!(table.weapons[0].armor_bonus, 6.0);
        assert_eq!(table.weapons[0].legendary_power, LegendaryPower::Emberbrand);
        assert_eq!(table.weapons[1].legendary_power, LegendaryPower::Frostbrand);
        assert_eq!(table.weapons[2].legendary_power, LegendaryPower::Stormbrand);
        assert_eq!(table.weapons[3].legendary_power, LegendaryPower::Soulreaver);
        assert_eq!(table.weapons[4].legendary_power, LegendaryPower::Aegisbrand);
        assert_eq!(table.weapons[5].quality, "ancient");
        assert_eq!(table.weapons[5].potions, 3);
        assert_eq!(table.weapons[5].legendary_power, LegendaryPower::Stormbrand);
        assert_eq!(table.weapons[6].quality, "ancient");
        assert_eq!(table.weapons[6].potions, 3);
        assert_eq!(table.weapons[6].legendary_power, LegendaryPower::Aegisbrand);
    }

    #[test]
    fn enemy_attack_kind_parses_from_ron() {
        let enemy: EnemyTuning = ron::from_str(
            r#"(
                id: "cultist",
                display_name: "Cinder Acolyte",
                max_health: 34.0,
                move_speed: 1.7,
                attack_damage: 13.0,
                attack_kind: projectile,
                attack_range: 4.2,
                attack_cooldown: 1.8,
                aggro_range: 10.5,
                gold_min: 5,
                gold_max: 12,
                xp_reward: 18,
                model: "cultist.glb",
            )"#,
        )
        .unwrap();

        assert_eq!(enemy.attack_kind, EnemyAttackKind::Projectile);
    }

    #[test]
    fn treasure_imp_tuning_parses_from_ron() {
        let catalog: EnemyCatalog =
            ron::from_str(include_str!("../assets/data/enemies.ron")).unwrap();
        let catalog = catalog.hydrate();
        let treasure_imp = catalog.get("treasure_imp").unwrap();

        assert_eq!(treasure_imp.display_name, "Reliquary Treasure Imp");
        assert!(treasure_imp.gold_min >= 90);
        assert!(treasure_imp.move_speed > 3.0);
    }

    #[test]
    fn nemesis_tuning_parses_from_ron() {
        let catalog: EnemyCatalog =
            ron::from_str(include_str!("../assets/data/enemies.ron")).unwrap();
        let catalog = catalog.hydrate();
        let nemesis = catalog.get("nemesis").unwrap();

        assert_eq!(nemesis.display_name, "Ashbound Nemesis");
        assert_eq!(nemesis.attack_kind, EnemyAttackKind::Melee);
        assert!(nemesis.xp_reward >= 70);
    }

    #[test]
    fn seal_warden_is_a_data_backed_mainline_miniboss() {
        let catalog: EnemyCatalog =
            ron::from_str(include_str!("../assets/data/enemies.ron")).unwrap();
        let catalog = catalog.hydrate();
        let cultist = catalog.get("cultist").unwrap();
        let warden = catalog.get("seal_warden").unwrap();

        assert_eq!(warden.display_name, "Seal Warden Vhal");
        assert_eq!(warden.attack_kind, EnemyAttackKind::Projectile);
        assert_eq!(warden.model, "cultist.glb");
        assert!(warden.max_health > cultist.max_health * 2.0);
        assert!(warden.gold_min > cultist.gold_min);
        assert!(warden.xp_reward > cultist.xp_reward);
    }
}
