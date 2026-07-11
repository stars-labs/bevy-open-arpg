use crate::{
    CHAMPION_PACK_REWARD_ESSENCE, CHAMPION_PACK_REWARD_GOLD, CHAMPION_PACK_REWARD_SHARDS,
    CHAMPION_PACK_TARGET, GameState, RunStats,
    assets::GameAssets,
    chapter::{InteractableKind, InteractableUsed},
    data::LootTable,
    enemy::{
        AFFIX_HUNTER_KILLS, ChampionPackMember, CursedAmbusher, Enemy, EnemyAffix, EnemyEntity,
        EnemyKilled, affix_mask, spawn_molten_death_pool,
    },
    feedback::{CombatEvent, FloatingCombatTextEvent, ScreenShakeEvent},
    grant_champion_pack_reward, not_paused,
    ordeal::ChapterModifier,
    player::{
        Charm, DamageBonus, ElixirBelt, Equipment, FortuneBuff, Fury, GemKind, GloryBuff, Health,
        Inventory, InventoryItem, LegendaryCodex, LegendaryPower, Player, PotionBelt, RelicBuff,
        SocketedGem, charm_power, equip_inventory_item, fortune_gold_reward,
        fortune_magic_find_active, gear_quality_is_ancient_or_better,
        gear_quality_is_legendary_or_better, gem_label, grant_glory, grant_reliquary_momentum,
        is_themed_reliquary_resonance, potion_cooldown_secs_for_capacity, socket_or_upgrade_gem,
    },
    register_champion_pack_kill, register_valor_kill, valor_gold_reward, valor_xp_reward,
};
use bevy::ecs::system::SystemParam;
use bevy::prelude::*;
use rand::Rng;

#[derive(Component)]
pub struct LootDrop {
    pub gold: u32,
    pub damage_bonus: f32,
    pub crit_chance: f32,
    pub health_bonus: f32,
    pub armor_bonus: f32,
    pub legendary_power: LegendaryPower,
    pub temper_level: u32,
    pub socketed_gem: Option<SocketedGem>,
    pub bonus_gem: Option<SocketedGem>,
    pub bonus_charm: Option<Charm>,
    pub potions: u32,
    pub elixirs: u32,
    pub quality: String,
    pub label: String,
}

#[derive(Component)]
pub struct LootEntity;

pub const LOOT_PICKUP_RADIUS: f32 = 1.3;
pub(crate) const QUICK_SALVAGE_KEY: KeyCode = KeyCode::KeyU;

#[derive(Resource, Default, Debug, Clone)]
pub struct LootProximityInfo {
    pub visible: bool,
    pub priority: String,
    pub label: String,
    pub compare: String,
    pub rewards: String,
    pub distance: f32,
    pub pickup_ready: bool,
    pub inventory_full: bool,
    pub spare_salvage_available: bool,
    pub quality: String,
}

#[derive(Component)]
struct LootLabel {
    owner: Entity,
}

#[derive(Component)]
struct LootFilterVisual {
    min_rank: u8,
}

#[derive(Component)]
struct LootBeam {
    base_radius: f32,
}

#[derive(Component)]
struct LootGroundRing {
    base_scale: Vec3,
    spin_speed: f32,
    pulse_speed: f32,
    pulse_amount: f32,
}

#[derive(Component, Clone, Copy)]
struct LootPulse {
    base_scale: Vec3,
    spin_speed: f32,
    pulse_speed: f32,
    pulse_amount: f32,
}

#[derive(Component, Clone, Copy)]
struct LootGlow {
    base_intensity: f32,
    pulse_speed: f32,
    pulse_amount: f32,
}

#[derive(Component)]
struct DeathBurstVfx {
    timer: Timer,
    duration_secs: f32,
    base_scale: Vec3,
    base_intensity: f32,
}

#[derive(Component)]
struct DeathRemains {
    timer: Timer,
    duration_secs: f32,
    base_scale: Vec3,
    base_alpha: f32,
}

#[derive(Component)]
pub struct HealthGlobe {
    pub heal_fraction: f32,
}

#[derive(Component)]
pub struct FuryGlobe {
    pub fury_fraction: f32,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum LootClaimMode {
    Ground,
    Direct,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum DeathBurstKind {
    Spark,
    Burst,
    Prism,
    Cataclysm,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum DeathRemainsKind {
    Ash,
    Bone,
    EliteBrand,
    TreasureAsh,
    BossSeal,
}

#[derive(Clone, Copy)]
struct DeathBurstStyle {
    scale: f32,
    lifetime_secs: f32,
    lift: f32,
    light_intensity: f32,
    light_range: f32,
    light_color: Color,
}

#[derive(Clone, Copy)]
struct DeathRemainsStyle {
    radius: f32,
    height: f32,
    lifetime_secs: f32,
    lift: f32,
    alpha: f32,
    color: Color,
    emissive: Color,
}

pub struct LootPlugin;

type LootPickupPlayerQuery<'w, 's> = Query<
    'w,
    's,
    (
        &'static Transform,
        &'static mut DamageBonus,
        &'static mut Charm,
        &'static mut Equipment,
        &'static mut Inventory,
        &'static mut Health,
        &'static mut PotionBelt,
        &'static mut ElixirBelt,
        &'static mut LegendaryCodex,
        &'static FortuneBuff,
    ),
    With<Player>,
>;
type InteractableRewardPlayerQuery<'w, 's> = Query<
    'w,
    's,
    (
        &'static mut Health,
        &'static mut DamageBonus,
        &'static mut Charm,
        &'static mut Equipment,
        &'static mut Inventory,
        &'static mut PotionBelt,
        &'static mut ElixirBelt,
        &'static mut LegendaryCodex,
    ),
    With<Player>,
>;
type DeadEnemyQuery<'w, 's> = Query<
    'w,
    's,
    (
        Entity,
        &'static Transform,
        &'static Enemy,
        &'static crate::player::Health,
        Option<&'static CursedAmbusher>,
        Option<&'static ChampionPackMember>,
    ),
    (With<EnemyEntity>, Without<Player>),
>;
type LootRewardPlayerQuery<'w, 's> = Query<
    'w,
    's,
    (
        &'static mut DamageBonus,
        &'static mut Charm,
        &'static mut Equipment,
        &'static mut Inventory,
        &'static mut Health,
        &'static mut PotionBelt,
        &'static mut ElixirBelt,
        &'static mut LegendaryCodex,
        &'static mut RelicBuff,
        &'static mut Fury,
        &'static FortuneBuff,
    ),
    (With<Player>, Without<EnemyEntity>),
>;
type LootReadabilityCameraQuery<'w, 's> = Query<
    'w,
    's,
    &'static Transform,
    (
        With<Camera3d>,
        Without<LootLabel>,
        Without<LootBeam>,
        Without<LootPulse>,
        Without<LootGroundRing>,
    ),
>;
type LootLabelTransformQuery<'w, 's> = Query<
    'w,
    's,
    &'static mut Transform,
    (
        With<LootLabel>,
        Without<LootBeam>,
        Without<LootPulse>,
        Without<LootGroundRing>,
    ),
>;
type LootBeamTransformQuery<'w, 's> = Query<
    'w,
    's,
    (&'static mut Transform, &'static LootBeam),
    (
        Without<LootLabel>,
        Without<LootPulse>,
        Without<LootGroundRing>,
    ),
>;
type LootPulseTransformQuery<'w, 's> = Query<
    'w,
    's,
    (&'static mut Transform, &'static LootPulse),
    (
        Without<LootLabel>,
        Without<LootBeam>,
        Without<LootGroundRing>,
    ),
>;
type LootGroundRingTransformQuery<'w, 's> = Query<
    'w,
    's,
    (&'static mut Transform, &'static LootGroundRing),
    (Without<LootLabel>, Without<LootBeam>, Without<LootPulse>),
>;
type LootGlowQuery<'w, 's> = Query<'w, 's, (&'static mut PointLight, &'static LootGlow)>;
type LootFilterVisualQuery<'w, 's> =
    Query<'w, 's, (&'static LootFilterVisual, &'static mut Visibility)>;
type LootLabelTextQuery<'w, 's> =
    Query<'w, 's, (&'static LootLabel, &'static mut Text2d), With<LootEntity>>;
type LootComparePlayerQuery<'w, 's> =
    Query<'w, 's, (&'static DamageBonus, &'static Equipment, &'static Inventory), With<Player>>;
type LootProximityPlayerQuery<'w, 's> = Query<
    'w,
    's,
    (
        &'static Transform,
        &'static DamageBonus,
        &'static Equipment,
        &'static Inventory,
    ),
    With<Player>,
>;

#[derive(Resource, Debug, Clone, Copy, Default, Eq, PartialEq)]
enum LootFilterTier {
    #[default]
    All,
    RarePlus,
    LegendaryPlus,
    AncientPlus,
    Primal,
}

impl LootFilterTier {
    fn next(self) -> Self {
        match self {
            Self::All => Self::RarePlus,
            Self::RarePlus => Self::LegendaryPlus,
            Self::LegendaryPlus => Self::AncientPlus,
            Self::AncientPlus => Self::Primal,
            Self::Primal => Self::All,
        }
    }

    fn label(self) -> &'static str {
        match self {
            Self::All => "All",
            Self::RarePlus => "Rare+",
            Self::LegendaryPlus => "Legendary+",
            Self::AncientPlus => "Ancient+",
            Self::Primal => "Primal",
        }
    }

    fn min_rank(self) -> u8 {
        match self {
            Self::All => 0,
            Self::RarePlus => 1,
            Self::LegendaryPlus => 2,
            Self::AncientPlus => 3,
            Self::Primal => 4,
        }
    }
}

#[derive(SystemParam)]
struct InteractableRewardInputs<'w, 's> {
    commands: Commands<'w, 's>,
    render_assets: (
        ResMut<'w, Assets<Mesh>>,
        ResMut<'w, Assets<StandardMaterial>>,
    ),
    loot_table: Res<'w, LootTable>,
    interactions: MessageReader<'w, 's, InteractableUsed>,
    stats: ResMut<'w, RunStats>,
    modifier: Res<'w, ChapterModifier>,
    combat_events: MessageWriter<'w, CombatEvent>,
}

#[derive(SystemParam)]
struct DeadEnemyRewardInputs<'w, 's> {
    commands: Commands<'w, 's>,
    render_assets: (
        ResMut<'w, Assets<Mesh>>,
        ResMut<'w, Assets<StandardMaterial>>,
    ),
    loot_table: Res<'w, LootTable>,
    game_assets: Res<'w, GameAssets>,
    stats: ResMut<'w, RunStats>,
    writers: (
        MessageWriter<'w, EnemyKilled>,
        MessageWriter<'w, CombatEvent>,
        MessageWriter<'w, ScreenShakeEvent>,
        MessageWriter<'w, FloatingCombatTextEvent>,
    ),
    _marker: std::marker::PhantomData<&'s ()>,
}

const MERCHANT_POTION_COST: u32 = 35;
const MERCHANT_ELIXIR_COST: u32 = 55;
const MERCHANT_REFORGE_SHARD_COST: u32 = 6;
const MERCHANT_ENCHANT_GOLD_COST: u32 = 180;
const MERCHANT_ENCHANT_SHARD_COST: u32 = 10;
const MERCHANT_ENCHANT_ESSENCE_COST: u32 = 2;
const SALVAGE_CACHE_ITEM_THRESHOLD: u32 = 4;
const SALVAGE_CACHE_GOLD_REWARD: u32 = 60;
const SALVAGE_CACHE_SHARD_REWARD: u32 = 2;
const SALVAGE_CACHE_ESSENCE_REWARD: u32 = 1;
const MERCHANT_CHARM_POLISH_GOLD_COST: u32 = 150;
const MERCHANT_CHARM_POLISH_SHARD_COST: u32 = 4;
const MERCHANT_MYSTERY_CHARM_GOLD_COST: u32 = 90;
const MERCHANT_MYSTERY_CHARM_SHARD_COST: u32 = 5;
const MERCHANT_MYSTERY_WEAPON_GOLD_COST: u32 = 160;
const MERCHANT_MYSTERY_WEAPON_SHARD_COST: u32 = 8;
const MERCHANT_STASH_GOLD_COST: u32 = 120;
const MERCHANT_STASH_SHARD_COST: u32 = 3;
const MAX_INVENTORY_CAPACITY: usize = 18;
const ALCHEMY_GOLD_TO_SHARDS_COST: u32 = 220;
const ALCHEMY_GOLD_TO_SHARDS_REWARD: u32 = 3;
const ALCHEMY_SHARDS_TO_ESSENCE_COST: u32 = 8;
const ALCHEMY_SHARDS_TO_ESSENCE_REWARD: u32 = 2;
const ALCHEMY_KEYSTONE_SHARD_COST: u32 = 12;
const ALCHEMY_KEYSTONE_ESSENCE_COST: u32 = 6;
const ALCHEMY_KEYSTONE_REWARD: u32 = 1;
pub(crate) const SOUL_SIGIL_CACHE_COST: u32 = 2;
const SOUL_SIGIL_CACHE_GOLD_REWARD: u32 = 120;
const SOUL_SIGIL_CACHE_SHARD_REWARD: u32 = 4;
const SOUL_SIGIL_CACHE_ESSENCE_REWARD: u32 = 3;
const SOUL_SIGIL_CACHE_KEYSTONE_REWARD: u32 = 1;
const MERCHANT_POTION_BELT_GOLD_COST: u32 = 140;
const MERCHANT_POTION_BELT_SHARD_COST: u32 = 4;
const MAX_POTION_CAPACITY: u32 = 7;
const MERCHANT_ELIXIR_BELT_GOLD_COST: u32 = 160;
const MERCHANT_ELIXIR_BELT_SHARD_COST: u32 = 5;
const MAX_ELIXIR_CAPACITY: u32 = 4;
const ANCIENT_AUGMENT_MIN_GEM_RANK: u32 = 3;
const ANCIENT_AUGMENT_BASE_GOLD_COST: u32 = 180;
const ANCIENT_AUGMENT_GOLD_PER_RANK: u32 = 70;
const ANCIENT_AUGMENT_BASE_SHARD_COST: u32 = 8;
const ANCIENT_AUGMENT_SHARDS_PER_RANK: u32 = 2;
const ANCIENT_AUGMENT_ESSENCE_PER_RANK: u32 = 2;
const GEM_EMPOWER_MIN_RANK: u32 = 5;
const GEM_EMPOWER_RANK_GAIN: u32 = 2;
const GEM_EMPOWER_GOLD_COST: u32 = 320;
const GEM_EMPOWER_ESSENCE_COST: u32 = 8;
const GEM_EMPOWER_ECHO_KEYSTONE_COST: u32 = 1;
const ANCIENT_AWAKEN_MIN_GEM_RANK: u32 = 4;
const ANCIENT_AWAKEN_BASE_GOLD_COST: u32 = 320;
const ANCIENT_AWAKEN_GOLD_PER_RANK: u32 = 90;
const ANCIENT_AWAKEN_BASE_SHARD_COST: u32 = 16;
const ANCIENT_AWAKEN_SHARDS_PER_RANK: u32 = 3;
const ANCIENT_AWAKEN_ESSENCE_PER_RANK: u32 = 3;
const PRIMAL_INFUSION_GOLD_COST: u32 = 520;
const PRIMAL_INFUSION_SHARD_COST: u32 = 24;
const PRIMAL_INFUSION_ESSENCE_COST: u32 = 14;
const PRIMAL_INFUSION_ECHO_KEYSTONE_COST: u32 = 1;

#[derive(Clone, Copy)]
struct ItemStats<'a> {
    quality: &'a str,
    damage_bonus: f32,
    crit_chance: f32,
    health_bonus: f32,
    armor_bonus: f32,
    legendary_power: LegendaryPower,
    temper_level: u32,
    socketed_gem: Option<SocketedGem>,
}

struct LootClaimState<'a, 'w> {
    stats: &'a mut RunStats,
    combat_events: &'a mut MessageWriter<'w, CombatEvent>,
    damage_bonus: &'a mut DamageBonus,
    charm: &'a mut Charm,
    equipment: &'a mut Equipment,
    inventory: &'a mut Inventory,
    health: &'a mut Health,
    potions: &'a mut PotionBelt,
    elixirs: &'a mut ElixirBelt,
    codex: &'a mut LegendaryCodex,
}

impl Plugin for LootPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<LootFilterTier>()
            .init_resource::<LootProximityInfo>()
            .add_systems(OnEnter(GameState::InGame), reset_loot_filter)
            .add_systems(
                Update,
                (
                    toggle_loot_filter,
                    drop_loot_for_dead_enemies,
                    reward_interactables,
                    update_death_burst_vfx,
                    update_death_remains,
                    update_loot_readability,
                    update_loot_labels,
                    update_loot_filter_visibility,
                    update_loot_proximity_info,
                    pickup_loot,
                    pickup_health_globes,
                    pickup_fury_globes,
                    quick_salvage_spare_inventory,
                )
                    .run_if(in_state(GameState::InGame).and_then(not_paused)),
            )
            .add_systems(OnExit(GameState::InGame), despawn_loot_runtime_entities);
    }
}

fn reset_loot_filter(mut filter: ResMut<LootFilterTier>) {
    *filter = LootFilterTier::All;
}

fn toggle_loot_filter(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut filter: ResMut<LootFilterTier>,
    mut stats: ResMut<RunStats>,
    mut combat_events: MessageWriter<CombatEvent>,
) {
    if !keyboard.just_pressed(KeyCode::KeyL) {
        return;
    }
    *filter = filter.next();
    stats.loot_filter_cycles = stats.loot_filter_cycles.saturating_add(1);
    combat_events.write(CombatEvent {
        text: format!("Loot filter: {}", filter.label()),
    });
}

fn drop_loot_for_dead_enemies(
    mut inputs: DeadEnemyRewardInputs,
    dead_enemies: DeadEnemyQuery,
    mut player_rewards: LootRewardPlayerQuery,
) {
    let mut rng = rand::rng();
    let Ok((
        mut damage_bonus,
        mut charm,
        mut equipment,
        mut inventory,
        mut player_health,
        mut potions,
        mut elixirs,
        mut codex,
        mut relic,
        mut fury,
        fortune,
    )) = player_rewards.single_mut()
    else {
        return;
    };
    let magic_find = fortune_magic_find_active(fortune);
    for (entity, transform, enemy, health, cursed_ambusher, champion_pack_member) in &dead_enemies {
        if health.current > 0.0 {
            continue;
        }
        inputs.stats.kills += 1;
        if codex.attuned != LegendaryPower::None {
            inputs.stats.codex_attuned_kills = inputs.stats.codex_attuned_kills.saturating_add(1);
        }
        if is_themed_reliquary_resonance(&equipment, &charm) {
            inputs.stats.set_resonance_kills = inputs.stats.set_resonance_kills.saturating_add(1);
        }
        let is_cursed_ambusher = cursed_ambusher.is_some();
        let is_elite = !enemy.affixes.is_empty();
        if let Some(event) = death_screen_shake_event(&enemy.id, is_elite) {
            inputs.writers.2.write(event);
        }
        spawn_death_burst_vfx(
            &mut inputs.commands,
            &inputs.game_assets,
            transform.translation,
            death_burst_kind(&enemy.id, is_elite),
        );
        spawn_death_remains(
            &mut inputs.commands,
            &mut inputs.render_assets.0,
            &mut inputs.render_assets.1,
            transform.translation,
            death_remains_kind(&enemy.id, is_elite),
        );
        let cursed_before = inputs.stats.cursed_ambush_kills;
        if is_cursed_ambusher {
            inputs.stats.cursed_ambush_kills = inputs.stats.cursed_ambush_kills.saturating_add(1);
            if cursed_before < crate::enemy::CURSED_AMBUSH_KILLS
                && inputs.stats.cursed_ambush_kills >= crate::enemy::CURSED_AMBUSH_KILLS
            {
                let reward = grant_reliquary_momentum(&mut inputs.stats, &mut relic, &mut fury, 1);
                inputs.writers.1.write(CombatEvent {
                    text: format!(
                        "Reliquary Momentum {}x: cursed ambush cleared, +{:.0} fury",
                        reward.stacks, reward.fury_gained
                    ),
                });
            }
        }
        let affix_text = if enemy.affixes.is_empty() {
            String::new()
        } else {
            format!(
                " [{}]",
                enemy
                    .affixes
                    .iter()
                    .map(|affix| affix.label())
                    .collect::<Vec<_>>()
                    .join(", ")
            )
        };
        inputs.writers.1.write(CombatEvent {
            text: format!("{} slain{}", enemy.display_name, affix_text),
        });
        inputs.writers.3.write(FloatingCombatTextEvent {
            text: kill_floating_text(&enemy.id, is_elite),
            position: transform.translation + Vec3::Y * 0.25,
            critical: is_elite || enemy.id == "keeper",
        });
        if is_elite || enemy.id == "keeper" || enemy.id == "nemesis" {
            let stacks = register_valor_kill(&mut inputs.stats);
            inputs.writers.1.write(CombatEvent {
                text: format!("Valor {stacks}x: elite rewards empowered"),
            });
        }
        if affix_hunter_kill(enemy.affixes.len()) {
            inputs.stats.elite_affix_kills = inputs.stats.elite_affix_kills.saturating_add(1);
            let break_essence = elite_affix_break_essence_reward(&enemy.id, enemy.affixes.len());
            if break_essence > 0 {
                inputs.stats.affix_essence =
                    inputs.stats.affix_essence.saturating_add(break_essence);
                spawn_elite_affix_break_death_vfx(
                    &mut inputs.commands,
                    &inputs.game_assets,
                    transform.translation,
                    enemy.affixes.len(),
                );
                inputs.writers.1.write(CombatEvent {
                    text: elite_affix_break_reward_text(&enemy.display_name, break_essence),
                });
                inputs.writers.3.write(FloatingCombatTextEvent {
                    text: elite_affix_break_float_text(break_essence),
                    position: transform.translation + Vec3::Y * 0.85,
                    critical: true,
                });
            }
            inputs.writers.1.write(CombatEvent {
                text: format!(
                    "Affix Hunter {}/{}",
                    inputs.stats.elite_affix_kills.min(AFFIX_HUNTER_KILLS),
                    AFFIX_HUNTER_KILLS
                ),
            });
        }
        let slain_affix_mask = affix_mask(&enemy.affixes);
        inputs.stats.affix_codex_mask |= slain_affix_mask;
        inputs.writers.0.write(EnemyKilled {
            enemy_id: enemy.id.clone(),
            display_name: enemy.display_name.clone(),
            position: transform.translation,
            xp_reward: valor_xp_reward(enemy.xp_reward, &inputs.stats),
            affix_count: enemy.affixes.len(),
            affix_mask: slain_affix_mask,
            cursed_ambusher: is_cursed_ambusher,
            champion_pack_member: champion_pack_member.is_some(),
        });
        if champion_pack_member.is_some() {
            let complete = register_champion_pack_kill(&mut inputs.stats);
            inputs.writers.1.write(CombatEvent {
                text: format!(
                    "Champion pack {}/{}",
                    inputs.stats.champion_pack_kills.min(CHAMPION_PACK_TARGET),
                    CHAMPION_PACK_TARGET
                ),
            });
            if complete && grant_champion_pack_reward(&mut inputs.stats) {
                let momentum =
                    grant_reliquary_momentum(&mut inputs.stats, &mut relic, &mut fury, 1);
                let cache_weapon = roll_champion_cache_weapon(&inputs.loot_table, &mut rng);
                spawn_loot_drop(
                    &mut inputs.commands,
                    &mut inputs.render_assets.0,
                    &mut inputs.render_assets.1,
                    transform.translation + Vec3::new(0.7, 0.35, 0.35),
                    CHAMPION_PACK_REWARD_GOLD,
                    cache_weapon,
                    "Champion reliquary cache".to_string(),
                );
                inputs.writers.1.write(CombatEvent {
                    text: format!(
                        "Champion pack broken: +{} gold, +{} shards, +{} essence, Momentum {}x, cache dropped",
                        CHAMPION_PACK_REWARD_GOLD,
                        CHAMPION_PACK_REWARD_SHARDS,
                        CHAMPION_PACK_REWARD_ESSENCE,
                        momentum.stacks
                    ),
                });
            }
        }

        let gold = valor_gold_reward(
            rng.random_range(enemy.gold_min..=enemy.gold_max),
            &inputs.stats,
        );
        let shard_reward = enemy_shard_reward(&enemy.id, is_elite);
        if shard_reward > 0 {
            inputs.stats.ember_shards += shard_reward;
            inputs.writers.1.write(CombatEvent {
                text: format!("Recovered {shard_reward} ember shards"),
            });
        }
        let essence_reward = enemy_affix_essence_reward(&enemy.id, enemy.affixes.len());
        if essence_reward > 0 {
            inputs.stats.affix_essence += essence_reward;
            inputs.writers.1.write(CombatEvent {
                text: format!("Recovered {essence_reward} affix essence"),
            });
        }
        let weapon = roll_enemy_weapon(
            &enemy.id,
            is_elite,
            magic_find,
            &inputs.loot_table,
            &mut rng,
        );
        if enemy_loot_claim_mode(&enemy.id) == LootClaimMode::Direct {
            let drop = loot_drop_from_weapon(gold, weapon);
            let mut claim = LootClaimState {
                stats: &mut inputs.stats,
                combat_events: &mut inputs.writers.1,
                damage_bonus: &mut damage_bonus,
                charm: &mut charm,
                equipment: &mut equipment,
                inventory: &mut inventory,
                health: &mut player_health,
                potions: &mut potions,
                elixirs: &mut elixirs,
                codex: &mut codex,
            };
            claim_loot_drop(&mut claim, fortune, &drop);
            inputs.writers.1.write(CombatEvent {
                text: "Malrec's hoard secured before the reliquary collapses".to_string(),
            });
        } else {
            spawn_loot_drop(
                &mut inputs.commands,
                &mut inputs.render_assets.0,
                &mut inputs.render_assets.1,
                transform.translation + Vec3::new(0.0, 0.25, 0.0),
                gold,
                weapon,
                format!("Loot from {}", enemy.display_name),
            );
        }
        if health_globe_drops(&enemy.id, is_elite, rng.random::<f32>()) {
            spawn_health_globe(
                &mut inputs.commands,
                &mut inputs.render_assets.0,
                &mut inputs.render_assets.1,
                transform.translation + Vec3::new(0.55, 0.32, -0.25),
            );
            inputs.writers.1.write(CombatEvent {
                text: "Health globe dropped".to_string(),
            });
        }
        if fury_globe_drops(&enemy.id, is_elite, rng.random::<f32>()) {
            spawn_fury_globe(
                &mut inputs.commands,
                &mut inputs.render_assets.0,
                &mut inputs.render_assets.1,
                transform.translation + Vec3::new(-0.48, 0.30, 0.28),
            );
            inputs.writers.1.write(CombatEvent {
                text: "Fury globe dropped".to_string(),
            });
        }

        if enemy.affixes.contains(&EnemyAffix::Molten) {
            spawn_molten_death_pool(
                &mut inputs.commands,
                &mut inputs.render_assets.0,
                &mut inputs.render_assets.1,
                transform.translation,
                enemy.attack_damage * 0.85,
            );
            inputs.writers.1.write(CombatEvent {
                text: "Molten death pool erupts".to_string(),
            });
        }

        if let Ok(mut entity_commands) = inputs.commands.get_entity(entity) {
            entity_commands.try_despawn();
        }
    }
}

fn kill_floating_text(enemy_id: &str, is_elite: bool) -> String {
    if enemy_id == "keeper" {
        "BOSS DOWN".to_string()
    } else if is_elite {
        "ELITE SLAY".to_string()
    } else {
        "SLAY".to_string()
    }
}

fn reward_interactables(
    mut inputs: InteractableRewardInputs,
    mut player: InteractableRewardPlayerQuery,
) {
    let mut rng = rand::rng();
    for event in inputs.interactions.read() {
        match event.kind {
            InteractableKind::Cache => {
                let weapon = roll_cache_weapon(&inputs.loot_table, &mut rng);
                spawn_loot_drop(
                    &mut inputs.commands,
                    &mut inputs.render_assets.0,
                    &mut inputs.render_assets.1,
                    event.position + Vec3::new(0.0, 0.35, 0.8),
                    inputs.modifier.scale_reward(45),
                    weapon,
                    "Sealed reliquary cache".to_string(),
                );
                inputs.combat_events.write(CombatEvent {
                    text: "Reliquary cache opened: treasure released".to_string(),
                });
            }
            InteractableKind::Altar => {
                let Ok((mut health, _, _, _, _, mut potions, mut elixirs, _)) = player.single_mut()
                else {
                    continue;
                };
                let restored = (health.max * 0.55).max(1.0);
                health.current = (health.current + restored).min(health.max);
                potions.current = (potions.current + 2).min(potions.max);
                elixirs.current = (elixirs.current + 1).min(elixirs.max);
                inputs.combat_events.write(CombatEvent {
                    text: format!("Ember altar extinguished: restored {restored:.0} health"),
                });
            }
            InteractableKind::WeaponShrine => {}
            InteractableKind::FortuneShrine => {}
            InteractableKind::StormShrine => {}
            InteractableKind::HealingWell => {}
            InteractableKind::LorePage => {}
            InteractableKind::BloodObelisk => {}
            InteractableKind::EmberRift => {}
            InteractableKind::AshenPylon => {}
            InteractableKind::ReliquaryVault => {
                let weapon = roll_boss_weapon(&inputs.loot_table, &mut rng);
                let essence_reward = 3;
                inputs.stats.affix_essence += essence_reward;
                spawn_loot_drop(
                    &mut inputs.commands,
                    &mut inputs.render_assets.0,
                    &mut inputs.render_assets.1,
                    event.position + Vec3::new(0.0, 0.45, 0.95),
                    inputs.modifier.scale_reward(120),
                    weapon,
                    "Resplendent reliquary vault".to_string(),
                );
                inputs.combat_events.write(CombatEvent {
                    text: format!("Reliquary vault opened: +{essence_reward} affix essence"),
                });
            }
            InteractableKind::CursedShrine => {
                let Ok((mut health, _, _, _, _, _, _, _)) = player.single_mut() else {
                    continue;
                };
                let blood_price = cursed_shrine_blood_price(health.max);
                health.current = (health.current - blood_price).max(1.0);
                let weapon = roll_cache_weapon(&inputs.loot_table, &mut rng);
                spawn_loot_drop(
                    &mut inputs.commands,
                    &mut inputs.render_assets.0,
                    &mut inputs.render_assets.1,
                    event.position + Vec3::new(0.0, 0.45, 1.0),
                    inputs.modifier.scale_reward(80),
                    weapon,
                    "Cursed reliquary offering".to_string(),
                );
                inputs.combat_events.write(CombatEvent {
                    text: format!(
                        "Cursed shrine claimed {blood_price:.0} health and released a boon"
                    ),
                });
            }
            InteractableKind::Merchant => {
                let Ok((
                    mut health,
                    mut damage_bonus,
                    mut charm,
                    mut equipment,
                    mut inventory,
                    mut potions,
                    mut elixirs,
                    mut codex,
                )) = player.single_mut()
                else {
                    continue;
                };
                let transaction = trade_with_merchant(MerchantTrade {
                    loot_table: &inputs.loot_table,
                    inventory: &mut inventory,
                    equipment: &mut equipment,
                    damage_bonus: &mut damage_bonus,
                    charm: &mut charm,
                    health: &mut health,
                    potions: &mut potions,
                    elixirs: &mut elixirs,
                    codex: &mut codex,
                    stats: &mut inputs.stats,
                });
                inputs.combat_events.write(CombatEvent {
                    text: transaction.summary(),
                });
            }
        }
    }
}

#[derive(Default)]
struct MerchantTransaction {
    sold_items: usize,
    sale_gold: u32,
    salvaged_shards: u32,
    salvaged_essence: u32,
    salvage_cache_count: u32,
    salvage_cache_gold: u32,
    salvage_cache_shards: u32,
    salvage_cache_essence: u32,
    bought_potions: u32,
    bought_elixirs: u32,
    tempered: bool,
    temper_cost: u32,
    reforged: bool,
    reforge_cost: u32,
    enchanted: bool,
    enchant_gold_cost: u32,
    enchant_shard_cost: u32,
    enchant_essence_cost: u32,
    enchanted_power: LegendaryPower,
    charm_polished: bool,
    charm_polish_gold_cost: u32,
    charm_polish_shard_cost: u32,
    charm_label: String,
    mystery_charm: bool,
    mystery_charm_equipped: bool,
    mystery_charm_gold_cost: u32,
    mystery_charm_shard_cost: u32,
    mystery_charm_label: String,
    mystery_weapon: bool,
    mystery_weapon_equipped: bool,
    mystery_weapon_stored: bool,
    mystery_weapon_codex_unlocked: bool,
    mystery_weapon_gold_cost: u32,
    mystery_weapon_shard_cost: u32,
    mystery_weapon_label: String,
    capacity_upgraded: bool,
    capacity_gold_cost: u32,
    capacity_shard_cost: u32,
    inventory_capacity: usize,
    potion_belt_upgraded: bool,
    potion_belt_gold_cost: u32,
    potion_belt_shard_cost: u32,
    potion_capacity: u32,
    elixir_belt_upgraded: bool,
    elixir_belt_gold_cost: u32,
    elixir_belt_shard_cost: u32,
    elixir_capacity: u32,
    ancient_augmented: bool,
    ancient_augment_gold_cost: u32,
    ancient_augment_shard_cost: u32,
    ancient_augment_essence_cost: u32,
    ancient_augment_label: String,
    ancient_awakened: bool,
    ancient_awaken_gold_cost: u32,
    ancient_awaken_shard_cost: u32,
    ancient_awaken_essence_cost: u32,
    ancient_awaken_label: String,
    primal_infused: bool,
    primal_infusion_gold_cost: u32,
    primal_infusion_shard_cost: u32,
    primal_infusion_essence_cost: u32,
    primal_infusion_echo_cost: u32,
    primal_infusion_label: String,
    gem_empowered: bool,
    gem_empower_gold_cost: u32,
    gem_empower_essence_cost: u32,
    gem_empower_echo_cost: u32,
    gem_empower_label: String,
    gem_upgraded: bool,
    gem_cost: u32,
    gem_label: String,
    alchemy_gold: bool,
    alchemy_gold_cost: u32,
    alchemy_shard_reward: u32,
    alchemy_essence: bool,
    alchemy_shard_cost: u32,
    alchemy_essence_reward: u32,
    alchemy_keystone: bool,
    alchemy_keystone_shard_cost: u32,
    alchemy_keystone_essence_cost: u32,
    alchemy_keystone_reward: u32,
    soul_sigil_cache: bool,
    soul_sigil_cache_cost: u32,
    soul_sigil_cache_gold: u32,
    soul_sigil_cache_shards: u32,
    soul_sigil_cache_essence: u32,
    soul_sigil_cache_keystones: u32,
}

impl MerchantTransaction {
    fn summary(&self) -> String {
        if self.sold_items == 0
            && self.bought_potions == 0
            && self.bought_elixirs == 0
            && self.salvaged_shards == 0
            && self.salvaged_essence == 0
            && self.salvage_cache_count == 0
            && !self.tempered
            && !self.reforged
            && !self.enchanted
            && !self.charm_polished
            && !self.mystery_charm
            && !self.mystery_weapon
            && !self.capacity_upgraded
            && !self.potion_belt_upgraded
            && !self.elixir_belt_upgraded
            && !self.ancient_augmented
            && !self.ancient_awakened
            && !self.primal_infused
            && !self.gem_empowered
            && !self.gem_upgraded
            && !self.alchemy_gold
            && !self.alchemy_essence
            && !self.alchemy_keystone
            && !self.soul_sigil_cache
        {
            return "Quartermaster: no spare gear to sell and no supplies bought".to_string();
        }

        let mut lines = Vec::new();
        if self.sold_items > 0
            || self.sale_gold > 0
            || self.salvaged_shards > 0
            || self.salvaged_essence > 0
        {
            lines.push(format!(
                "Trade sold {} gear: +{}g +{}s +{} affix essence",
                self.sold_items, self.sale_gold, self.salvaged_shards, self.salvaged_essence
            ));
        }
        if self.bought_potions > 0 || self.bought_elixirs > 0 {
            lines.push(format!(
                "Supplies +{} potion +{} elixir",
                self.bought_potions, self.bought_elixirs
            ));
        }
        let mut gear = Vec::new();
        if self.tempered {
            gear.push(format!("tempered {}g", self.temper_cost));
        }
        if self.reforged {
            gear.push(format!("reforged {}s", self.reforge_cost));
        }
        if self.enchanted {
            gear.push(format!(
                "enchanted {} {}g/{}s/{} essence",
                self.enchanted_power.label(),
                self.enchant_gold_cost,
                self.enchant_shard_cost,
                self.enchant_essence_cost
            ));
        }
        if self.charm_polished {
            gear.push(format!(
                "polished {} {}g/{}s",
                self.charm_label, self.charm_polish_gold_cost, self.charm_polish_shard_cost
            ));
        }
        if !gear.is_empty() {
            lines.push(format!("Gear {}", gear.join(", ")));
        }

        let mut craft = Vec::new();
        if self.ancient_augmented {
            craft.push(format!(
                "ancient-augmented {} {}g/{}s/{}e",
                self.ancient_augment_label,
                self.ancient_augment_gold_cost,
                self.ancient_augment_shard_cost,
                self.ancient_augment_essence_cost
            ));
        }
        if self.ancient_awakened {
            craft.push(format!(
                "awakened {} {}g/{}s/{}e",
                self.ancient_awaken_label,
                self.ancient_awaken_gold_cost,
                self.ancient_awaken_shard_cost,
                self.ancient_awaken_essence_cost
            ));
        }
        if self.primal_infused {
            craft.push(format!(
                "primal-infused {} {}g/{}s/{}e/{}key",
                self.primal_infusion_label,
                self.primal_infusion_gold_cost,
                self.primal_infusion_shard_cost,
                self.primal_infusion_essence_cost,
                self.primal_infusion_echo_cost
            ));
        }
        if self.gem_empowered {
            craft.push(format!(
                "empowered {} {}g/{}e/{}key",
                self.gem_empower_label,
                self.gem_empower_gold_cost,
                self.gem_empower_essence_cost,
                self.gem_empower_echo_cost
            ));
        }
        if self.gem_upgraded {
            craft.push(format!("upgraded {} {}g", self.gem_label, self.gem_cost));
        }
        if !craft.is_empty() {
            lines.push(format!("Craft {}", craft.join(", ")));
        }

        let mut services = Vec::new();
        if self.mystery_charm {
            services.push(format!(
                "gambled {} {}g/{}s{}",
                self.mystery_charm_label,
                self.mystery_charm_gold_cost,
                self.mystery_charm_shard_cost,
                if self.mystery_charm_equipped {
                    " equipped"
                } else {
                    ""
                }
            ));
        }
        if self.mystery_weapon {
            services.push(format!(
                "gambled {} {}g/{}s{}{}{}",
                self.mystery_weapon_label,
                self.mystery_weapon_gold_cost,
                self.mystery_weapon_shard_cost,
                if self.mystery_weapon_equipped {
                    " equipped"
                } else {
                    ""
                },
                if self.mystery_weapon_codex_unlocked {
                    " +codex"
                } else {
                    ""
                },
                if self.mystery_weapon_stored {
                    ""
                } else {
                    " full"
                }
            ));
        }
        if self.capacity_upgraded {
            services.push(format!(
                "bag {} slots {}g/{}s",
                self.inventory_capacity, self.capacity_gold_cost, self.capacity_shard_cost
            ));
        }
        if self.potion_belt_upgraded {
            services.push(format!(
                "potion belt {} {}g/{}s",
                self.potion_capacity, self.potion_belt_gold_cost, self.potion_belt_shard_cost
            ));
        }
        if self.elixir_belt_upgraded {
            services.push(format!(
                "elixir belt {} {}g/{}s",
                self.elixir_capacity, self.elixir_belt_gold_cost, self.elixir_belt_shard_cost
            ));
        }
        if !services.is_empty() {
            lines.push(format!("Services {}", services.join(", ")));
        }

        let mut cache = Vec::new();
        if self.salvage_cache_count > 0 {
            cache.push(format!(
                "salvage cache x{} +{}g +{}s +{}e",
                self.salvage_cache_count,
                self.salvage_cache_gold,
                self.salvage_cache_shards,
                self.salvage_cache_essence
            ));
        }
        if self.alchemy_gold {
            cache.push(format!(
                "alchemy {}g -> {}s",
                self.alchemy_gold_cost, self.alchemy_shard_reward
            ));
        }
        if self.alchemy_essence {
            cache.push(format!(
                "alchemy {}s -> {}e",
                self.alchemy_shard_cost, self.alchemy_essence_reward
            ));
        }
        if self.alchemy_keystone {
            cache.push(format!(
                "alchemy {}s/{}e -> {}key",
                self.alchemy_keystone_shard_cost,
                self.alchemy_keystone_essence_cost,
                self.alchemy_keystone_reward
            ));
        }
        if self.soul_sigil_cache {
            cache.push(format!(
                "Malrec cache {} sigils: +{}g +{}s +{}e +{}key",
                self.soul_sigil_cache_cost,
                self.soul_sigil_cache_gold,
                self.soul_sigil_cache_shards,
                self.soul_sigil_cache_essence,
                self.soul_sigil_cache_keystones
            ));
        }
        if !cache.is_empty() {
            lines.push(format!("Cache {}", cache.join(", ")));
        }

        format!("Quartermaster: {}", lines.join(" | "))
    }
}

struct MerchantTrade<'a> {
    loot_table: &'a LootTable,
    inventory: &'a mut Inventory,
    equipment: &'a mut Equipment,
    damage_bonus: &'a mut DamageBonus,
    charm: &'a mut Charm,
    health: &'a mut Health,
    potions: &'a mut PotionBelt,
    elixirs: &'a mut ElixirBelt,
    codex: &'a mut LegendaryCodex,
    stats: &'a mut RunStats,
}

fn trade_with_merchant(trade: MerchantTrade) -> MerchantTransaction {
    let MerchantTrade {
        loot_table,
        inventory,
        equipment,
        damage_bonus,
        charm,
        health,
        potions,
        elixirs,
        codex,
        stats,
    } = trade;
    let salvage = salvage_spare_inventory_into_stats(inventory, equipment, stats);
    let missing_potions = potions.max.saturating_sub(potions.current);
    let affordable_potions = (stats.gold / MERCHANT_POTION_COST).min(missing_potions);
    if affordable_potions > 0 {
        stats.gold -= affordable_potions * MERCHANT_POTION_COST;
        potions.current += affordable_potions;
    }
    let missing_elixirs = elixirs.max.saturating_sub(elixirs.current);
    let affordable_elixirs = (stats.gold / MERCHANT_ELIXIR_COST).min(missing_elixirs);
    if affordable_elixirs > 0 {
        stats.gold -= affordable_elixirs * MERCHANT_ELIXIR_COST;
        elixirs.current += affordable_elixirs;
    }
    let temper_cost = temper_cost(equipment.temper_level);
    let tempered = temper_equipped_weapon(inventory, equipment, damage_bonus, &mut stats.gold);
    let reforged =
        reforge_equipped_weapon(inventory, equipment, damage_bonus, &mut stats.ember_shards);
    let enchanted = enchant_equipped_weapon(inventory, equipment, codex, stats);
    let ancient_awaken_gem = equipment.socketed_gem;
    let (ancient_awaken_gold_cost, ancient_awaken_shard_cost, ancient_awaken_essence_cost) =
        ancient_awaken_gem
            .map(ancient_awaken_cost)
            .unwrap_or_default();
    let ancient_awaken_label = equipment.weapon_name.clone();
    let ancient_awakened =
        awaken_legendary_weapon(inventory, equipment, damage_bonus, health, stats);
    let ancient_augment_gem = equipment.socketed_gem;
    let (ancient_augment_gold_cost, ancient_augment_shard_cost) = ancient_augment_gem
        .map(ancient_augment_cost)
        .unwrap_or_default();
    let ancient_augment_essence_cost = ancient_augment_gem
        .map(ancient_augment_essence_cost)
        .unwrap_or_default();
    let ancient_augmented =
        augment_ancient_weapon(inventory, equipment, damage_bonus, health, stats);
    let primal_infusion_label = equipment.weapon_name.clone();
    let primal_infused = infuse_primal_weapon(inventory, equipment, damage_bonus, health, stats);
    let gem_empower_label = equipment
        .socketed_gem
        .map(|gem| gem.label())
        .unwrap_or_else(|| "gem".to_string());
    let gem_empowered = empower_socketed_gem(inventory, equipment, damage_bonus, health, stats);
    let gem_cost = equipment
        .socketed_gem
        .map(|gem| gem_upgrade_cost(gem.rank))
        .unwrap_or_default();
    let gem_upgraded = !gem_empowered
        && upgrade_socketed_gem(inventory, equipment, damage_bonus, health, &mut stats.gold);
    let charm_polished = polish_charm(charm, stats);
    let mut rng = rand::rng();
    let mystery_charm =
        gamble_mystery_charm(charm, stats, rng.random::<f32>(), rng.random_range(0..3));
    let capacity_upgraded = expand_inventory_capacity(inventory, stats);
    let potion_belt_upgraded = upgrade_potion_belt(potions, stats);
    let elixir_belt_upgraded = upgrade_elixir_belt(elixirs, stats);
    let mystery_weapon_roll = roll_cache_weapon(loot_table, &mut rng);
    let mystery_weapon = gamble_mystery_weapon(
        mystery_weapon_roll,
        inventory,
        equipment,
        damage_bonus,
        health,
        codex,
        stats,
    );
    let fallback_trade_available = salvage.items == 0
        && affordable_potions == 0
        && affordable_elixirs == 0
        && !tempered
        && !reforged
        && !enchanted
        && !ancient_awakened
        && !ancient_augmented
        && !primal_infused
        && !gem_empowered
        && !gem_upgraded
        && !charm_polished
        && mystery_charm.is_none()
        && !capacity_upgraded
        && !potion_belt_upgraded
        && !elixir_belt_upgraded
        && mystery_weapon.is_none();
    let soul_sigil_cache = if fallback_trade_available {
        open_soul_sigil_cache(stats)
    } else {
        SoulSigilCacheReward::default()
    };
    let alchemy = if fallback_trade_available && !soul_sigil_cache.opened {
        apply_ashen_alchemy(stats)
    } else {
        AshenAlchemyBatch::default()
    };
    MerchantTransaction {
        sold_items: salvage.items,
        sale_gold: salvage.gold,
        salvaged_shards: salvage.shards,
        salvaged_essence: salvage.essence,
        salvage_cache_count: salvage.cache.caches,
        salvage_cache_gold: salvage.cache.gold,
        salvage_cache_shards: salvage.cache.shards,
        salvage_cache_essence: salvage.cache.essence,
        bought_potions: affordable_potions,
        bought_elixirs: affordable_elixirs,
        tempered,
        temper_cost,
        reforged,
        reforge_cost: MERCHANT_REFORGE_SHARD_COST,
        enchanted,
        enchant_gold_cost: MERCHANT_ENCHANT_GOLD_COST,
        enchant_shard_cost: MERCHANT_ENCHANT_SHARD_COST,
        enchant_essence_cost: MERCHANT_ENCHANT_ESSENCE_COST,
        enchanted_power: equipment.legendary_power,
        charm_polished,
        charm_polish_gold_cost: MERCHANT_CHARM_POLISH_GOLD_COST,
        charm_polish_shard_cost: MERCHANT_CHARM_POLISH_SHARD_COST,
        charm_label: charm.name.clone(),
        mystery_charm: mystery_charm.is_some(),
        mystery_charm_equipped: mystery_charm
            .as_ref()
            .map(|gamble| gamble.equipped)
            .unwrap_or_default(),
        mystery_charm_gold_cost: MERCHANT_MYSTERY_CHARM_GOLD_COST,
        mystery_charm_shard_cost: MERCHANT_MYSTERY_CHARM_SHARD_COST,
        mystery_charm_label: mystery_charm
            .map(|gamble| gamble.charm.summary())
            .unwrap_or_default(),
        mystery_weapon: mystery_weapon.is_some(),
        mystery_weapon_equipped: mystery_weapon
            .as_ref()
            .map(|gamble| gamble.equipped)
            .unwrap_or_default(),
        mystery_weapon_stored: mystery_weapon
            .as_ref()
            .map(|gamble| gamble.stored)
            .unwrap_or_default(),
        mystery_weapon_codex_unlocked: mystery_weapon
            .as_ref()
            .map(|gamble| gamble.codex_unlocked)
            .unwrap_or_default(),
        mystery_weapon_gold_cost: MERCHANT_MYSTERY_WEAPON_GOLD_COST,
        mystery_weapon_shard_cost: MERCHANT_MYSTERY_WEAPON_SHARD_COST,
        mystery_weapon_label: mystery_weapon
            .map(|gamble| gamble.item.name)
            .unwrap_or_default(),
        capacity_upgraded,
        capacity_gold_cost: MERCHANT_STASH_GOLD_COST,
        capacity_shard_cost: MERCHANT_STASH_SHARD_COST,
        inventory_capacity: inventory.capacity,
        potion_belt_upgraded,
        potion_belt_gold_cost: MERCHANT_POTION_BELT_GOLD_COST,
        potion_belt_shard_cost: MERCHANT_POTION_BELT_SHARD_COST,
        potion_capacity: potions.max,
        elixir_belt_upgraded,
        elixir_belt_gold_cost: MERCHANT_ELIXIR_BELT_GOLD_COST,
        elixir_belt_shard_cost: MERCHANT_ELIXIR_BELT_SHARD_COST,
        elixir_capacity: elixirs.max,
        ancient_augmented,
        ancient_augment_gold_cost,
        ancient_augment_shard_cost,
        ancient_augment_essence_cost,
        ancient_augment_label: ancient_augment_gem
            .map(|gem| gem.label())
            .unwrap_or_else(|| "gem".to_string()),
        ancient_awakened,
        ancient_awaken_gold_cost,
        ancient_awaken_shard_cost,
        ancient_awaken_essence_cost,
        ancient_awaken_label,
        primal_infused,
        primal_infusion_gold_cost: PRIMAL_INFUSION_GOLD_COST,
        primal_infusion_shard_cost: PRIMAL_INFUSION_SHARD_COST,
        primal_infusion_essence_cost: PRIMAL_INFUSION_ESSENCE_COST,
        primal_infusion_echo_cost: PRIMAL_INFUSION_ECHO_KEYSTONE_COST,
        primal_infusion_label,
        gem_empowered,
        gem_empower_gold_cost: GEM_EMPOWER_GOLD_COST,
        gem_empower_essence_cost: GEM_EMPOWER_ESSENCE_COST,
        gem_empower_echo_cost: GEM_EMPOWER_ECHO_KEYSTONE_COST,
        gem_empower_label,
        gem_upgraded,
        gem_cost,
        gem_label: equipment
            .socketed_gem
            .map(|gem| gem.label())
            .unwrap_or_else(|| "gem".to_string()),
        alchemy_gold: alchemy.gold_to_shards,
        alchemy_gold_cost: ALCHEMY_GOLD_TO_SHARDS_COST,
        alchemy_shard_reward: ALCHEMY_GOLD_TO_SHARDS_REWARD,
        alchemy_essence: alchemy.shards_to_essence,
        alchemy_shard_cost: ALCHEMY_SHARDS_TO_ESSENCE_COST,
        alchemy_essence_reward: ALCHEMY_SHARDS_TO_ESSENCE_REWARD,
        alchemy_keystone: alchemy.essence_to_keystone,
        alchemy_keystone_shard_cost: ALCHEMY_KEYSTONE_SHARD_COST,
        alchemy_keystone_essence_cost: ALCHEMY_KEYSTONE_ESSENCE_COST,
        alchemy_keystone_reward: ALCHEMY_KEYSTONE_REWARD,
        soul_sigil_cache: soul_sigil_cache.opened,
        soul_sigil_cache_cost: SOUL_SIGIL_CACHE_COST,
        soul_sigil_cache_gold: soul_sigil_cache.gold,
        soul_sigil_cache_shards: soul_sigil_cache.shards,
        soul_sigil_cache_essence: soul_sigil_cache.essence,
        soul_sigil_cache_keystones: soul_sigil_cache.keystones,
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
struct SoulSigilCacheReward {
    opened: bool,
    gold: u32,
    shards: u32,
    essence: u32,
    keystones: u32,
}

fn open_soul_sigil_cache(stats: &mut RunStats) -> SoulSigilCacheReward {
    if stats.malrec_soul_sigils < SOUL_SIGIL_CACHE_COST {
        return SoulSigilCacheReward::default();
    }
    stats.malrec_soul_sigils -= SOUL_SIGIL_CACHE_COST;
    stats.gold += SOUL_SIGIL_CACHE_GOLD_REWARD;
    stats.ember_shards += SOUL_SIGIL_CACHE_SHARD_REWARD;
    stats.affix_essence += SOUL_SIGIL_CACHE_ESSENCE_REWARD;
    stats.echo_keystones += SOUL_SIGIL_CACHE_KEYSTONE_REWARD;
    stats.soul_sigil_caches = stats.soul_sigil_caches.saturating_add(1);
    SoulSigilCacheReward {
        opened: true,
        gold: SOUL_SIGIL_CACHE_GOLD_REWARD,
        shards: SOUL_SIGIL_CACHE_SHARD_REWARD,
        essence: SOUL_SIGIL_CACHE_ESSENCE_REWARD,
        keystones: SOUL_SIGIL_CACHE_KEYSTONE_REWARD,
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
struct AshenAlchemyBatch {
    gold_to_shards: bool,
    shards_to_essence: bool,
    essence_to_keystone: bool,
}

fn apply_ashen_alchemy(stats: &mut RunStats) -> AshenAlchemyBatch {
    let mut batch = AshenAlchemyBatch::default();
    if stats.gold >= ALCHEMY_GOLD_TO_SHARDS_COST {
        stats.gold -= ALCHEMY_GOLD_TO_SHARDS_COST;
        stats.ember_shards += ALCHEMY_GOLD_TO_SHARDS_REWARD;
        stats.alchemy_gold_transmutes = stats.alchemy_gold_transmutes.saturating_add(1);
        batch.gold_to_shards = true;
    }
    if stats.ember_shards >= ALCHEMY_SHARDS_TO_ESSENCE_COST {
        stats.ember_shards -= ALCHEMY_SHARDS_TO_ESSENCE_COST;
        stats.affix_essence += ALCHEMY_SHARDS_TO_ESSENCE_REWARD;
        stats.alchemy_essence_transmutes = stats.alchemy_essence_transmutes.saturating_add(1);
        batch.shards_to_essence = true;
    }
    if stats.ember_shards >= ALCHEMY_KEYSTONE_SHARD_COST
        && stats.affix_essence >= ALCHEMY_KEYSTONE_ESSENCE_COST
    {
        stats.ember_shards -= ALCHEMY_KEYSTONE_SHARD_COST;
        stats.affix_essence -= ALCHEMY_KEYSTONE_ESSENCE_COST;
        stats.echo_keystones += ALCHEMY_KEYSTONE_REWARD;
        stats.alchemy_keystone_transmutes = stats.alchemy_keystone_transmutes.saturating_add(1);
        batch.essence_to_keystone = true;
    }
    batch
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
struct SalvageCacheReward {
    caches: u32,
    gold: u32,
    shards: u32,
    essence: u32,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
struct SpareSalvage {
    items: usize,
    gold: u32,
    shards: u32,
    essence: u32,
    cache: SalvageCacheReward,
}

impl SpareSalvage {
    fn summary(self) -> String {
        if self.items == 0 {
            return "No spare gear to salvage".to_string();
        }
        let mut text = format!(
            "Salvaged {} spare gear: +{}g +{}s +{} essence",
            self.items, self.gold, self.shards, self.essence
        );
        if self.cache.caches > 0 {
            text.push_str(&format!(
                " | salvage cache x{}: +{}g +{}s +{} essence",
                self.cache.caches, self.cache.gold, self.cache.shards, self.cache.essence
            ));
        }
        text
    }
}

fn apply_salvage_cache_rewards(stats: &mut RunStats, sold_items: usize) -> SalvageCacheReward {
    if sold_items == 0 {
        return SalvageCacheReward::default();
    }

    stats.salvage_progress = stats.salvage_progress.saturating_add(sold_items as u32);
    let caches = stats.salvage_progress / SALVAGE_CACHE_ITEM_THRESHOLD;
    if caches == 0 {
        return SalvageCacheReward::default();
    }

    stats.salvage_progress %= SALVAGE_CACHE_ITEM_THRESHOLD;
    stats.salvage_caches = stats.salvage_caches.saturating_add(caches);
    let reward = SalvageCacheReward {
        caches,
        gold: caches * SALVAGE_CACHE_GOLD_REWARD,
        shards: caches * SALVAGE_CACHE_SHARD_REWARD,
        essence: caches * SALVAGE_CACHE_ESSENCE_REWARD,
    };
    stats.gold += reward.gold;
    stats.ember_shards += reward.shards;
    stats.affix_essence += reward.essence;
    reward
}

fn polish_charm(charm: &mut Charm, stats: &mut RunStats) -> bool {
    if charm.quality == "none" {
        return false;
    }
    if stats.gold < MERCHANT_CHARM_POLISH_GOLD_COST
        || stats.ember_shards < MERCHANT_CHARM_POLISH_SHARD_COST
    {
        return false;
    }
    stats.gold -= MERCHANT_CHARM_POLISH_GOLD_COST;
    stats.ember_shards -= MERCHANT_CHARM_POLISH_SHARD_COST;
    charm.damage_bonus += 1.5;
    charm.crit_chance = (charm.crit_chance + 0.005).min(0.08);
    true
}

struct MysteryCharmGamble {
    charm: Charm,
    equipped: bool,
}

struct MysteryWeaponGamble {
    item: InventoryItem,
    equipped: bool,
    stored: bool,
    codex_unlocked: bool,
}

fn mystery_charm_quality(quality_roll: f32) -> &'static str {
    if quality_roll < 0.18 {
        "legendary"
    } else if quality_roll < 0.68 {
        "rare"
    } else {
        "common"
    }
}

fn gamble_mystery_charm(
    current: &mut Charm,
    stats: &mut RunStats,
    quality_roll: f32,
    charm_roll: u32,
) -> Option<MysteryCharmGamble> {
    if stats.gold < MERCHANT_MYSTERY_CHARM_GOLD_COST
        || stats.ember_shards < MERCHANT_MYSTERY_CHARM_SHARD_COST
    {
        return None;
    }
    stats.gold -= MERCHANT_MYSTERY_CHARM_GOLD_COST;
    stats.ember_shards -= MERCHANT_MYSTERY_CHARM_SHARD_COST;
    let charm = charm_for_roll(mystery_charm_quality(quality_roll), charm_roll);
    let equipped = equip_charm_if_better(current, &charm);
    Some(MysteryCharmGamble { charm, equipped })
}

fn gamble_mystery_weapon(
    weapon: &crate::data::LootEntry,
    inventory: &mut Inventory,
    equipment: &mut Equipment,
    damage_bonus: &mut DamageBonus,
    health: &mut Health,
    codex: &mut LegendaryCodex,
    stats: &mut RunStats,
) -> Option<MysteryWeaponGamble> {
    if stats.gold < MERCHANT_MYSTERY_WEAPON_GOLD_COST
        || stats.ember_shards < MERCHANT_MYSTERY_WEAPON_SHARD_COST
    {
        return None;
    }
    stats.gold -= MERCHANT_MYSTERY_WEAPON_GOLD_COST;
    stats.ember_shards -= MERCHANT_MYSTERY_WEAPON_SHARD_COST;
    let item = inventory_item_from_weapon(weapon);
    let codex_unlocked = codex.unlock(item.legendary_power);
    let stored = inventory.add(item.clone());
    let equipped =
        if item_power(ItemStats::from(&item)) >= equipped_item_power(equipment, damage_bonus) {
            equip_inventory_item(&item, damage_bonus, equipment, health);
            true
        } else {
            false
        };
    Some(MysteryWeaponGamble {
        item,
        equipped,
        stored,
        codex_unlocked,
    })
}

fn inventory_item_from_weapon(weapon: &crate::data::LootEntry) -> InventoryItem {
    InventoryItem {
        name: format!("{} {}", weapon.quality, weapon.name),
        quality: weapon.quality.clone(),
        damage_bonus: weapon.damage_bonus,
        crit_chance: weapon.crit_chance,
        health_bonus: weapon.health_bonus,
        armor_bonus: weapon.armor_bonus,
        legendary_power: weapon.legendary_power,
        temper_level: 0,
        socketed_gem: None,
    }
}

fn enchant_equipped_weapon(
    inventory: &mut Inventory,
    equipment: &mut Equipment,
    codex: &LegendaryCodex,
    stats: &mut RunStats,
) -> bool {
    let Some(next_power) = codex.enchant_target(equipment.legendary_power) else {
        return false;
    };
    if stats.gold < MERCHANT_ENCHANT_GOLD_COST
        || stats.ember_shards < MERCHANT_ENCHANT_SHARD_COST
        || stats.affix_essence < MERCHANT_ENCHANT_ESSENCE_COST
    {
        return false;
    }
    stats.gold -= MERCHANT_ENCHANT_GOLD_COST;
    stats.ember_shards -= MERCHANT_ENCHANT_SHARD_COST;
    stats.affix_essence -= MERCHANT_ENCHANT_ESSENCE_COST;
    equipment.legendary_power = next_power;
    equipment.quality = quality_after_legendary_enchant(&equipment.quality).to_string();
    if let Some(item) = inventory
        .items
        .iter_mut()
        .find(|item| item.name == equipment.weapon_name)
    {
        item.legendary_power = equipment.legendary_power;
        item.quality = equipment.quality.clone();
    }
    true
}

fn quality_after_legendary_enchant(quality: &str) -> &'static str {
    match quality {
        "primal" => "primal",
        "ancient" => "ancient",
        _ => "legendary",
    }
}

fn expand_inventory_capacity(inventory: &mut Inventory, stats: &mut RunStats) -> bool {
    if inventory.capacity >= MAX_INVENTORY_CAPACITY {
        return false;
    }
    if stats.gold < MERCHANT_STASH_GOLD_COST || stats.ember_shards < MERCHANT_STASH_SHARD_COST {
        return false;
    }
    stats.gold -= MERCHANT_STASH_GOLD_COST;
    stats.ember_shards -= MERCHANT_STASH_SHARD_COST;
    inventory.capacity = (inventory.capacity + 2).min(MAX_INVENTORY_CAPACITY);
    true
}

fn upgrade_potion_belt(potions: &mut PotionBelt, stats: &mut RunStats) -> bool {
    if potions.max >= MAX_POTION_CAPACITY {
        return false;
    }
    if stats.gold < MERCHANT_POTION_BELT_GOLD_COST
        || stats.ember_shards < MERCHANT_POTION_BELT_SHARD_COST
    {
        return false;
    }
    stats.gold -= MERCHANT_POTION_BELT_GOLD_COST;
    stats.ember_shards -= MERCHANT_POTION_BELT_SHARD_COST;
    potions.max = (potions.max + 1).min(MAX_POTION_CAPACITY);
    potions.cooldown_secs = potion_cooldown_secs_for_capacity(potions.max);
    true
}

fn upgrade_elixir_belt(elixirs: &mut ElixirBelt, stats: &mut RunStats) -> bool {
    if elixirs.max >= MAX_ELIXIR_CAPACITY {
        return false;
    }
    if stats.gold < MERCHANT_ELIXIR_BELT_GOLD_COST
        || stats.ember_shards < MERCHANT_ELIXIR_BELT_SHARD_COST
    {
        return false;
    }
    stats.gold -= MERCHANT_ELIXIR_BELT_GOLD_COST;
    stats.ember_shards -= MERCHANT_ELIXIR_BELT_SHARD_COST;
    elixirs.max = (elixirs.max + 1).min(MAX_ELIXIR_CAPACITY);
    true
}

fn reforge_equipped_weapon(
    inventory: &mut Inventory,
    equipment: &mut Equipment,
    damage_bonus: &mut DamageBonus,
    ember_shards: &mut u32,
) -> bool {
    if *ember_shards < MERCHANT_REFORGE_SHARD_COST {
        return false;
    }
    *ember_shards -= MERCHANT_REFORGE_SHARD_COST;
    damage_bonus.0 += 1.0;
    equipment.armor_bonus += 1.0;
    equipment.crit_chance = (equipment.crit_chance + 0.01).min(0.40);
    if let Some(item) = inventory
        .items
        .iter_mut()
        .find(|item| item.name == equipment.weapon_name)
    {
        item.damage_bonus = damage_bonus.0;
        item.armor_bonus = equipment.armor_bonus;
        item.crit_chance = equipment.crit_chance;
        item.socketed_gem = equipment.socketed_gem;
    }
    true
}

fn temper_cost(current_level: u32) -> u32 {
    90 + current_level * 45
}

fn temper_equipped_weapon(
    inventory: &mut Inventory,
    equipment: &mut Equipment,
    damage_bonus: &mut DamageBonus,
    gold: &mut u32,
) -> bool {
    let cost = temper_cost(equipment.temper_level);
    if *gold < cost {
        return false;
    }
    *gold -= cost;
    equipment.temper_level += 1;
    damage_bonus.0 += 2.0;
    equipment.armor_bonus += 2.0;
    equipment.crit_chance = (equipment.crit_chance + 0.005).min(0.35);
    if let Some(item) = inventory
        .items
        .iter_mut()
        .find(|item| item.name == equipment.weapon_name)
    {
        item.temper_level = equipment.temper_level;
        item.damage_bonus = damage_bonus.0;
        item.armor_bonus = equipment.armor_bonus;
        item.crit_chance = equipment.crit_chance;
        item.socketed_gem = equipment.socketed_gem;
    }
    true
}

fn gem_upgrade_cost(current_rank: u32) -> u32 {
    70 + current_rank * 50
}

fn empower_socketed_gem(
    inventory: &mut Inventory,
    equipment: &mut Equipment,
    damage_bonus: &mut DamageBonus,
    health: &mut Health,
    stats: &mut RunStats,
) -> bool {
    let Some(gem) = equipment.socketed_gem else {
        return false;
    };
    if gem.rank < GEM_EMPOWER_MIN_RANK {
        return false;
    }
    if stats.gold < GEM_EMPOWER_GOLD_COST
        || stats.affix_essence < GEM_EMPOWER_ESSENCE_COST
        || stats.echo_keystones < GEM_EMPOWER_ECHO_KEYSTONE_COST
    {
        return false;
    }
    stats.gold -= GEM_EMPOWER_GOLD_COST;
    stats.affix_essence -= GEM_EMPOWER_ESSENCE_COST;
    stats.echo_keystones -= GEM_EMPOWER_ECHO_KEYSTONE_COST;
    socket_or_upgrade_gem(
        SocketedGem {
            kind: gem.kind,
            rank: GEM_EMPOWER_RANK_GAIN,
        },
        damage_bonus,
        equipment,
        inventory,
        health,
    );
    true
}

fn ancient_augment_cost(gem: SocketedGem) -> (u32, u32) {
    (
        ANCIENT_AUGMENT_BASE_GOLD_COST + gem.rank * ANCIENT_AUGMENT_GOLD_PER_RANK,
        ANCIENT_AUGMENT_BASE_SHARD_COST + gem.rank * ANCIENT_AUGMENT_SHARDS_PER_RANK,
    )
}

fn ancient_augment_essence_cost(gem: SocketedGem) -> u32 {
    gem.rank * ANCIENT_AUGMENT_ESSENCE_PER_RANK
}

fn ancient_awaken_cost(gem: SocketedGem) -> (u32, u32, u32) {
    (
        ANCIENT_AWAKEN_BASE_GOLD_COST + gem.rank * ANCIENT_AWAKEN_GOLD_PER_RANK,
        ANCIENT_AWAKEN_BASE_SHARD_COST + gem.rank * ANCIENT_AWAKEN_SHARDS_PER_RANK,
        gem.rank * ANCIENT_AWAKEN_ESSENCE_PER_RANK,
    )
}

fn awaken_legendary_weapon(
    inventory: &mut Inventory,
    equipment: &mut Equipment,
    damage_bonus: &mut DamageBonus,
    health: &mut Health,
    stats: &mut RunStats,
) -> bool {
    if equipment.quality != "legendary" || equipment.legendary_power == LegendaryPower::None {
        return false;
    }
    let Some(gem) = equipment.socketed_gem else {
        return false;
    };
    if gem.rank < ANCIENT_AWAKEN_MIN_GEM_RANK {
        return false;
    }
    let (gold_cost, shard_cost, essence_cost) = ancient_awaken_cost(gem);
    if stats.gold < gold_cost
        || stats.ember_shards < shard_cost
        || stats.affix_essence < essence_cost
    {
        return false;
    }
    stats.gold -= gold_cost;
    stats.ember_shards -= shard_cost;
    stats.affix_essence -= essence_cost;
    let old_name = equipment.weapon_name.clone();
    equipment.weapon_name = ancient_awakened_name(&equipment.weapon_name);
    equipment.quality = "ancient".to_string();
    match gem.kind {
        GemKind::Ruby => {
            damage_bonus.0 += gem.rank as f32 * 2.0;
        }
        GemKind::Emerald => {
            equipment.crit_chance = (equipment.crit_chance + gem.rank as f32 * 0.005).min(0.40);
        }
        GemKind::Amethyst => {
            let health_bonus = gem.rank as f32 * 5.0;
            equipment.health_bonus += health_bonus;
            health.max += health_bonus;
            health.current = (health.current + health_bonus).min(health.max);
        }
        GemKind::Topaz => {
            equipment.armor_bonus += gem.rank as f32 * 3.0;
        }
    }
    equipment.socketed_gem = None;
    if let Some(item) = inventory
        .items
        .iter_mut()
        .find(|item| item.name == old_name)
    {
        item.name = equipment.weapon_name.clone();
        item.quality = equipment.quality.clone();
        item.damage_bonus = damage_bonus.0;
        item.crit_chance = equipment.crit_chance;
        item.health_bonus = equipment.health_bonus;
        item.armor_bonus = equipment.armor_bonus;
        item.socketed_gem = None;
    }
    true
}

fn ancient_awakened_name(name: &str) -> String {
    if name.starts_with("Ancient ") {
        name.to_string()
    } else {
        format!("Ancient {name}")
    }
}

fn augment_ancient_weapon(
    inventory: &mut Inventory,
    equipment: &mut Equipment,
    damage_bonus: &mut DamageBonus,
    health: &mut Health,
    stats: &mut RunStats,
) -> bool {
    if !gear_quality_is_ancient_or_better(&equipment.quality) {
        return false;
    }
    let Some(gem) = equipment.socketed_gem else {
        return false;
    };
    if gem.rank < ANCIENT_AUGMENT_MIN_GEM_RANK {
        return false;
    }
    let (gold_cost, shard_cost) = ancient_augment_cost(gem);
    let essence_cost = ancient_augment_essence_cost(gem);
    if stats.gold < gold_cost
        || stats.ember_shards < shard_cost
        || stats.affix_essence < essence_cost
    {
        return false;
    }
    stats.gold -= gold_cost;
    stats.ember_shards -= shard_cost;
    stats.affix_essence -= essence_cost;
    match gem.kind {
        GemKind::Ruby => {
            damage_bonus.0 += gem.rank as f32 * 2.5;
        }
        GemKind::Emerald => {
            equipment.crit_chance = (equipment.crit_chance + gem.rank as f32 * 0.006).min(0.40);
        }
        GemKind::Amethyst => {
            let health_bonus = gem.rank as f32 * 6.0;
            equipment.health_bonus += health_bonus;
            health.max += health_bonus;
            health.current = (health.current + health_bonus).min(health.max);
        }
        GemKind::Topaz => {
            equipment.armor_bonus += gem.rank as f32 * 3.5;
        }
    }
    stats.ancient_augments += 1;
    equipment.socketed_gem = None;
    if let Some(item) = inventory
        .items
        .iter_mut()
        .find(|item| item.name == equipment.weapon_name)
    {
        item.damage_bonus = damage_bonus.0;
        item.crit_chance = equipment.crit_chance;
        item.health_bonus = equipment.health_bonus;
        item.armor_bonus = equipment.armor_bonus;
        item.socketed_gem = None;
    }
    true
}

fn infuse_primal_weapon(
    inventory: &mut Inventory,
    equipment: &mut Equipment,
    damage_bonus: &mut DamageBonus,
    health: &mut Health,
    stats: &mut RunStats,
) -> bool {
    if equipment.quality != "ancient" {
        return false;
    }
    if stats.echo_keystones < PRIMAL_INFUSION_ECHO_KEYSTONE_COST
        || stats.gold < PRIMAL_INFUSION_GOLD_COST
        || stats.ember_shards < PRIMAL_INFUSION_SHARD_COST
        || stats.affix_essence < PRIMAL_INFUSION_ESSENCE_COST
    {
        return false;
    }

    stats.echo_keystones -= PRIMAL_INFUSION_ECHO_KEYSTONE_COST;
    stats.gold -= PRIMAL_INFUSION_GOLD_COST;
    stats.ember_shards -= PRIMAL_INFUSION_SHARD_COST;
    stats.affix_essence -= PRIMAL_INFUSION_ESSENCE_COST;
    stats.primal_infusions = stats.primal_infusions.saturating_add(1);

    let old_name = equipment.weapon_name.clone();
    equipment.weapon_name = primal_infused_name(&equipment.weapon_name);
    equipment.quality = "primal".to_string();
    damage_bonus.0 += 8.0;
    equipment.crit_chance = (equipment.crit_chance + 0.025).min(0.45);
    equipment.armor_bonus += 10.0;
    equipment.health_bonus += 24.0;
    health.max += 24.0;
    health.current = (health.current + 24.0).min(health.max);

    if let Some(item) = inventory
        .items
        .iter_mut()
        .find(|item| item.name == old_name)
    {
        item.name = equipment.weapon_name.clone();
        item.quality = equipment.quality.clone();
        item.damage_bonus = damage_bonus.0;
        item.crit_chance = equipment.crit_chance;
        item.health_bonus = equipment.health_bonus;
        item.armor_bonus = equipment.armor_bonus;
        item.socketed_gem = equipment.socketed_gem;
    }
    true
}

fn primal_infused_name(name: &str) -> String {
    if name.starts_with("Primal ") {
        name.to_string()
    } else if let Some(rest) = name.strip_prefix("Ancient ") {
        format!("Primal {rest}")
    } else {
        format!("Primal {name}")
    }
}

pub(crate) fn primal_infusion_pursuit_summary(stats: &RunStats, equipment: &Equipment) -> String {
    if stats.primal_infusions > 0 {
        return format!(
            "Primal Infusion complete: {} forged",
            stats.primal_infusions
        );
    }
    match equipment.quality.as_str() {
        "primal" => "Primal Infusion complete: primal weapon equipped".to_string(),
        "ancient" => {
            if stats.echo_keystones == 0 {
                return "Primal next: seal a swift Ember Rift for an Echo Keystone".to_string();
            }
            let missing_gold = PRIMAL_INFUSION_GOLD_COST.saturating_sub(stats.gold);
            let missing_shards = PRIMAL_INFUSION_SHARD_COST.saturating_sub(stats.ember_shards);
            let missing_essence = PRIMAL_INFUSION_ESSENCE_COST.saturating_sub(stats.affix_essence);
            if missing_gold == 0 && missing_shards == 0 && missing_essence == 0 {
                "Primal ready: Space at quartermaster to infuse ancient weapon".to_string()
            } else {
                format!(
                    "Primal next: gather {} gold, {} shards, {} essence",
                    missing_gold, missing_shards, missing_essence
                )
            }
        }
        "legendary" => {
            "Primal next: awaken legendary into ancient at the quartermaster".to_string()
        }
        _ => "Primal next: find and build a legendary weapon first".to_string(),
    }
}

pub(crate) fn quartermaster_preview_summary(
    stats: &RunStats,
    inventory: &Inventory,
    equipment: &Equipment,
    potions: &PotionBelt,
    elixirs: &ElixirBelt,
    codex: &LegendaryCodex,
) -> String {
    let spare_items = quartermaster_spare_item_count(inventory, equipment);
    if spare_items > 0 {
        return format!(
            "Quartermaster next: U salvage {spare_items} spare gear now, or Space sells/restocks/upgrades"
        );
    }
    if potions.current < potions.max && stats.gold >= MERCHANT_POTION_COST {
        return format!(
            "Quartermaster next: Space buys {} potion(s) for {}g each",
            potions.max - potions.current,
            MERCHANT_POTION_COST
        );
    }
    if elixirs.current < elixirs.max && stats.gold >= MERCHANT_ELIXIR_COST {
        return format!(
            "Quartermaster next: Space buys {} elixir(s) for {}g each",
            elixirs.max - elixirs.current,
            MERCHANT_ELIXIR_COST
        );
    }
    if equipment.quality == "ancient" {
        let (gold, shards, essence, keys) = quartermaster_primal_trade_requirements(equipment);
        if stats.gold >= gold
            && stats.ember_shards >= shards
            && stats.affix_essence >= essence
            && stats.echo_keystones >= keys
        {
            return format!(
                "Quartermaster ready: Space can primal-infuse after auto-upgrades for {}g/{}s/{}e",
                PRIMAL_INFUSION_GOLD_COST, PRIMAL_INFUSION_SHARD_COST, PRIMAL_INFUSION_ESSENCE_COST
            );
        }
        if let Some(summary) = soul_sigil_cache_ready_summary(stats.malrec_soul_sigils) {
            return format!(
                "Quartermaster next: Space opens {summary}; then {}",
                quartermaster_next_need_summary(
                    stats, inventory, equipment, potions, elixirs, codex
                )
            );
        }
        return format!(
            "Quartermaster needs: {} for primal infusion",
            resource_gap_summary(stats, gold, shards, essence, keys)
        );
    }
    if let Some(summary) = soul_sigil_cache_ready_summary(stats.malrec_soul_sigils) {
        return format!(
            "Quartermaster next: Space opens {summary}; then {}",
            quartermaster_next_need_summary(stats, inventory, equipment, potions, elixirs, codex)
        );
    }
    if stats.gold >= temper_cost(equipment.temper_level) {
        return format!(
            "Quartermaster next: Space tempers weapon +2 damage for {}g",
            temper_cost(equipment.temper_level)
        );
    }
    if stats.ember_shards >= MERCHANT_REFORGE_SHARD_COST {
        return format!(
            "Quartermaster next: Space reforges weapon for {} shards",
            MERCHANT_REFORGE_SHARD_COST
        );
    }
    if codex.enchant_target(equipment.legendary_power).is_some()
        && stats.gold >= MERCHANT_ENCHANT_GOLD_COST
        && stats.ember_shards >= MERCHANT_ENCHANT_SHARD_COST
        && stats.affix_essence >= MERCHANT_ENCHANT_ESSENCE_COST
    {
        return format!(
            "Quartermaster next: Space enchants legendary power for {}g/{}s/{}e",
            MERCHANT_ENCHANT_GOLD_COST, MERCHANT_ENCHANT_SHARD_COST, MERCHANT_ENCHANT_ESSENCE_COST
        );
    }
    if let Some(gem) = equipment.socketed_gem {
        if equipment.quality == "legendary"
            && equipment.legendary_power != LegendaryPower::None
            && gem.rank >= ANCIENT_AWAKEN_MIN_GEM_RANK
        {
            let (gold, shards, essence) = ancient_awaken_cost(gem);
            if stats.gold >= gold && stats.ember_shards >= shards && stats.affix_essence >= essence
            {
                return format!(
                    "Quartermaster next: Space awakens legendary to ancient for {gold}g/{shards}s/{essence}e"
                );
            }
        }
        if gear_quality_is_ancient_or_better(&equipment.quality)
            && gem.rank >= ANCIENT_AUGMENT_MIN_GEM_RANK
        {
            let (gold, shards) = ancient_augment_cost(gem);
            let essence = ancient_augment_essence_cost(gem);
            if stats.gold >= gold && stats.ember_shards >= shards && stats.affix_essence >= essence
            {
                return format!(
                    "Quartermaster next: Space consumes {} to ancient-augment gear",
                    gem.label()
                );
            }
        }
        if gem.rank >= GEM_EMPOWER_MIN_RANK
            && stats.gold >= GEM_EMPOWER_GOLD_COST
            && stats.affix_essence >= GEM_EMPOWER_ESSENCE_COST
            && stats.echo_keystones >= GEM_EMPOWER_ECHO_KEYSTONE_COST
        {
            return format!(
                "Quartermaster next: Space empowers {} +{} ranks",
                gem.label(),
                GEM_EMPOWER_RANK_GAIN
            );
        }
        let gem_cost = gem_upgrade_cost(gem.rank);
        if stats.gold >= gem_cost {
            return format!(
                "Quartermaster next: Space upgrades {} for {}g",
                gem.label(),
                gem_cost
            );
        }
    }
    if stats.gold >= MERCHANT_MYSTERY_CHARM_GOLD_COST
        && stats.ember_shards >= MERCHANT_MYSTERY_CHARM_SHARD_COST
    {
        return format!(
            "Quartermaster next: Space gambles mystery charm for {}g/{}s",
            MERCHANT_MYSTERY_CHARM_GOLD_COST, MERCHANT_MYSTERY_CHARM_SHARD_COST
        );
    }
    if inventory.capacity < MAX_INVENTORY_CAPACITY
        && stats.gold >= MERCHANT_STASH_GOLD_COST
        && stats.ember_shards >= MERCHANT_STASH_SHARD_COST
    {
        return format!(
            "Quartermaster next: Space expands bag +2 for {}g/{}s",
            MERCHANT_STASH_GOLD_COST, MERCHANT_STASH_SHARD_COST
        );
    }
    if potions.max < MAX_POTION_CAPACITY
        && stats.gold >= MERCHANT_POTION_BELT_GOLD_COST
        && stats.ember_shards >= MERCHANT_POTION_BELT_SHARD_COST
    {
        return format!(
            "Quartermaster next: Space upgrades potion belt to {} for {}g/{}s",
            potions.max + 1,
            MERCHANT_POTION_BELT_GOLD_COST,
            MERCHANT_POTION_BELT_SHARD_COST
        );
    }
    if elixirs.max < MAX_ELIXIR_CAPACITY
        && stats.gold >= MERCHANT_ELIXIR_BELT_GOLD_COST
        && stats.ember_shards >= MERCHANT_ELIXIR_BELT_SHARD_COST
    {
        return format!(
            "Quartermaster next: Space upgrades elixir belt to {} for {}g/{}s",
            elixirs.max + 1,
            MERCHANT_ELIXIR_BELT_GOLD_COST,
            MERCHANT_ELIXIR_BELT_SHARD_COST
        );
    }
    quartermaster_next_need_summary(stats, inventory, equipment, potions, elixirs, codex)
}

fn soul_sigil_cache_reward_summary_for(caches: u32) -> String {
    let key_plural = if caches == 1 { "" } else { "s" };
    format!(
        "+{}g +{}s +{}e +{} key{}",
        SOUL_SIGIL_CACHE_GOLD_REWARD.saturating_mul(caches),
        SOUL_SIGIL_CACHE_SHARD_REWARD.saturating_mul(caches),
        SOUL_SIGIL_CACHE_ESSENCE_REWARD.saturating_mul(caches),
        SOUL_SIGIL_CACHE_KEYSTONE_REWARD.saturating_mul(caches),
        key_plural
    )
}

pub(crate) fn soul_sigil_cache_ready_count(sigils: u32) -> u32 {
    sigils / SOUL_SIGIL_CACHE_COST
}

pub(crate) fn soul_sigil_cache_ready_summary(sigils: u32) -> Option<String> {
    let caches = soul_sigil_cache_ready_count(sigils);
    if caches == 0 {
        return None;
    }
    let cache_plural = if caches == 1 { "" } else { "s" };
    let spent_sigils = caches.saturating_mul(SOUL_SIGIL_CACHE_COST);
    Some(format!(
        "{} Trophy Cache{} ({} Soul Sigils, {})",
        caches,
        cache_plural,
        spent_sigils,
        soul_sigil_cache_reward_summary_for(caches)
    ))
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub(crate) struct SalvagePreview {
    pub items: usize,
    pub gold: u32,
    pub shards: u32,
    pub essence: u32,
}

pub(crate) fn salvage_preview(inventory: &Inventory, equipment: &Equipment) -> SalvagePreview {
    let mut kept_equipped = false;
    let mut preview = SalvagePreview::default();
    for item in &inventory.items {
        if !kept_equipped && item.name == equipment.weapon_name {
            kept_equipped = true;
            continue;
        }
        preview.items += 1;
        preview.gold += item_sale_value(item);
        preview.shards += item_salvage_shards(item);
        preview.essence += item_salvage_essence(item);
    }
    preview
}

pub(crate) fn loot_pursuit_summary(
    stats: &RunStats,
    inventory: &Inventory,
    equipment: &Equipment,
) -> String {
    let salvage = salvage_preview(inventory, equipment);
    if salvage.items > 0 {
        return format!(
            "U salvage {}: +{}g +{}s +{}e",
            salvage.items, salvage.gold, salvage.shards, salvage.essence
        );
    }
    if inventory.items.len() >= inventory.capacity {
        return "Bag full: equip upgrade or visit quartermaster".to_string();
    }
    if equipment.quality == "ancient" || equipment.quality == "primal" {
        return primal_infusion_pursuit_summary(stats, equipment);
    }
    if gear_quality_is_legendary_or_better(&equipment.quality) {
        return "Loot next: socket gem, awaken ancient, then chase primal".to_string();
    }
    "Loot next: hunt elites for legendary gear and shards".to_string()
}

fn quartermaster_spare_item_count(inventory: &Inventory, equipment: &Equipment) -> usize {
    let mut kept_equipped = false;
    inventory
        .items
        .iter()
        .filter(|item| {
            if !kept_equipped && item.name == equipment.weapon_name {
                kept_equipped = true;
                false
            } else {
                true
            }
        })
        .count()
}

fn quartermaster_primal_trade_requirements(equipment: &Equipment) -> (u32, u32, u32, u32) {
    (
        PRIMAL_INFUSION_GOLD_COST + temper_cost(equipment.temper_level),
        PRIMAL_INFUSION_SHARD_COST + MERCHANT_REFORGE_SHARD_COST,
        PRIMAL_INFUSION_ESSENCE_COST,
        PRIMAL_INFUSION_ECHO_KEYSTONE_COST,
    )
}

fn quartermaster_next_need_summary(
    stats: &RunStats,
    inventory: &Inventory,
    equipment: &Equipment,
    potions: &PotionBelt,
    elixirs: &ElixirBelt,
    codex: &LegendaryCodex,
) -> String {
    if equipment.quality == "ancient" {
        let (gold, shards, essence, keys) = quartermaster_primal_trade_requirements(equipment);
        return format!(
            "Quartermaster needs: {} for primal infusion",
            resource_gap_summary(stats, gold, shards, essence, keys)
        );
    }
    if codex.enchant_target(equipment.legendary_power).is_some() {
        return format!(
            "Quartermaster needs: {} for codex enchant",
            resource_gap_summary(
                stats,
                MERCHANT_ENCHANT_GOLD_COST,
                MERCHANT_ENCHANT_SHARD_COST,
                MERCHANT_ENCHANT_ESSENCE_COST,
                0,
            )
        );
    }
    if equipment.quality == "legendary" && equipment.legendary_power != LegendaryPower::None {
        if let Some(gem) = equipment.socketed_gem
            && gem.rank >= ANCIENT_AWAKEN_MIN_GEM_RANK
        {
            let (gold, shards, essence) = ancient_awaken_cost(gem);
            return format!(
                "Quartermaster needs: {} to awaken {}",
                resource_gap_summary(stats, gold, shards, essence, 0),
                gem.label()
            );
        }
        return "Quartermaster needs: rank 4+ socketed gem to awaken legendary gear".to_string();
    }
    if let Some(gem) = equipment.socketed_gem {
        if gear_quality_is_ancient_or_better(&equipment.quality)
            && gem.rank >= ANCIENT_AUGMENT_MIN_GEM_RANK
        {
            let (gold, shards) = ancient_augment_cost(gem);
            let essence = ancient_augment_essence_cost(gem);
            return format!(
                "Quartermaster needs: {} to ancient-augment {}",
                resource_gap_summary(stats, gold, shards, essence, 0),
                gem.label()
            );
        }
        if gem.rank >= GEM_EMPOWER_MIN_RANK {
            return format!(
                "Quartermaster needs: {} to empower {}",
                resource_gap_summary(
                    stats,
                    GEM_EMPOWER_GOLD_COST,
                    0,
                    GEM_EMPOWER_ESSENCE_COST,
                    GEM_EMPOWER_ECHO_KEYSTONE_COST,
                ),
                gem.label()
            );
        }
        let gem_cost = gem_upgrade_cost(gem.rank);
        return format!(
            "Quartermaster needs: {} to upgrade {}",
            resource_gap_summary(stats, gem_cost, 0, 0, 0),
            gem.label()
        );
    }
    if stats.gold < MERCHANT_MYSTERY_CHARM_GOLD_COST
        || stats.ember_shards < MERCHANT_MYSTERY_CHARM_SHARD_COST
    {
        return format!(
            "Quartermaster needs: {} for mystery charm",
            resource_gap_summary(
                stats,
                MERCHANT_MYSTERY_CHARM_GOLD_COST,
                MERCHANT_MYSTERY_CHARM_SHARD_COST,
                0,
                0,
            )
        );
    }
    if inventory.capacity < MAX_INVENTORY_CAPACITY {
        return format!(
            "Quartermaster needs: {} to expand bag",
            resource_gap_summary(
                stats,
                MERCHANT_STASH_GOLD_COST,
                MERCHANT_STASH_SHARD_COST,
                0,
                0,
            )
        );
    }
    if potions.max < MAX_POTION_CAPACITY {
        return format!(
            "Quartermaster needs: {} to upgrade potion belt",
            resource_gap_summary(
                stats,
                MERCHANT_POTION_BELT_GOLD_COST,
                MERCHANT_POTION_BELT_SHARD_COST,
                0,
                0,
            )
        );
    }
    if elixirs.max < MAX_ELIXIR_CAPACITY {
        return format!(
            "Quartermaster needs: {} to upgrade elixir belt",
            resource_gap_summary(
                stats,
                MERCHANT_ELIXIR_BELT_GOLD_COST,
                MERCHANT_ELIXIR_BELT_SHARD_COST,
                0,
                0,
            )
        );
    }
    "Quartermaster needs: spare gear, shards, or a stronger legendary weapon".to_string()
}

fn resource_gap_summary(
    stats: &RunStats,
    gold: u32,
    shards: u32,
    essence: u32,
    keys: u32,
) -> String {
    let mut missing = Vec::new();
    let gold = gold.saturating_sub(stats.gold);
    if gold > 0 {
        missing.push(format!("{gold}g"));
    }
    let shards = shards.saturating_sub(stats.ember_shards);
    if shards > 0 {
        missing.push(format!("{shards}s"));
    }
    let essence = essence.saturating_sub(stats.affix_essence);
    if essence > 0 {
        missing.push(format!("{essence}e"));
    }
    let keys = keys.saturating_sub(stats.echo_keystones);
    if keys > 0 {
        missing.push(format!("{keys} key{}", if keys == 1 { "" } else { "s" }));
    }
    if missing.is_empty() {
        "ready".to_string()
    } else {
        missing.join(" ")
    }
}

fn upgrade_socketed_gem(
    inventory: &mut Inventory,
    equipment: &mut Equipment,
    damage_bonus: &mut DamageBonus,
    health: &mut Health,
    gold: &mut u32,
) -> bool {
    let Some(gem) = equipment.socketed_gem else {
        return false;
    };
    let cost = gem_upgrade_cost(gem.rank);
    if *gold < cost {
        return false;
    }
    *gold -= cost;
    socket_or_upgrade_gem(
        SocketedGem {
            kind: gem.kind,
            rank: 1,
        },
        damage_bonus,
        equipment,
        inventory,
        health,
    );
    true
}

fn salvage_spare_inventory(inventory: &mut Inventory, equipment: &Equipment) -> SpareSalvage {
    let mut kept_equipped = false;
    let mut salvage = SpareSalvage::default();
    inventory.items.retain(|item| {
        if !kept_equipped && item.name == equipment.weapon_name {
            kept_equipped = true;
            return true;
        }
        salvage.items += 1;
        salvage.gold += item_sale_value(item);
        salvage.shards += item_salvage_shards(item);
        salvage.essence += item_salvage_essence(item);
        false
    });
    salvage
}

fn salvage_spare_inventory_into_stats(
    inventory: &mut Inventory,
    equipment: &Equipment,
    stats: &mut RunStats,
) -> SpareSalvage {
    let mut salvage = salvage_spare_inventory(inventory, equipment);
    stats.gold += salvage.gold;
    stats.ember_shards += salvage.shards;
    stats.affix_essence += salvage.essence;
    salvage.cache = apply_salvage_cache_rewards(stats, salvage.items);
    salvage
}

fn item_sale_value(item: &InventoryItem) -> u32 {
    (item_power(ItemStats::from(item)) * 2.0).round().max(1.0) as u32
}

fn item_salvage_shards(item: &InventoryItem) -> u32 {
    let quality_shards = match item.quality.as_str() {
        "primal" => 24,
        "ancient" => 14,
        "legendary" => 8,
        "rare" => 3,
        _ => 1,
    };
    quality_shards + item.temper_level + item.socketed_gem.map(|gem| gem.rank).unwrap_or_default()
}

fn item_salvage_essence(item: &InventoryItem) -> u32 {
    match item.quality.as_str() {
        "primal" => 6,
        "ancient" => 3,
        "legendary" => 1,
        _ if item.legendary_power != LegendaryPower::None => 1,
        _ => 0,
    }
}

fn cursed_shrine_blood_price(max_health: f32) -> f32 {
    (max_health * 0.20).round().max(1.0)
}

fn item_power(item: ItemStats) -> f32 {
    let quality_multiplier = match item.quality {
        "primal" => 4.8,
        "ancient" => 3.8,
        "legendary" => 3.0,
        "rare" => 2.0,
        _ => 1.0,
    };
    (12.0
        + item.damage_bonus * 2.5
        + item.crit_chance * 120.0
        + item.health_bonus * 0.6
        + item.armor_bonus * 0.8
        + legendary_power_score(item.legendary_power)
        + gem_power_score(item.socketed_gem)
        + item.temper_level as f32 * 8.0)
        * quality_multiplier
}

fn loot_compare_summary(
    label: &str,
    candidate: ItemStats,
    equipped: ItemStats,
    stored: bool,
    auto_equipped: bool,
) -> String {
    let power_delta = item_power(candidate) - item_power(equipped);
    let outcome = if auto_equipped {
        if power_delta > 5.0 {
            "Auto-equipped upgrade"
        } else {
            "Auto-equipped sidegrade"
        }
    } else if stored {
        "Stashed for build swap"
    } else {
        "Inventory full, kept current"
    };
    let role = loot_role_tag(candidate, equipped, power_delta, stored);

    format!(
        "{outcome}: {label} | {} [{role}] | power {:+.0} | dmg {:+.0} crit {:+.0}% hp {:+.0} armor {:+.0}{}",
        loot_decision_reason(candidate, equipped, power_delta, stored),
        power_delta,
        candidate.damage_bonus - equipped.damage_bonus,
        (candidate.crit_chance - equipped.crit_chance) * 100.0,
        candidate.health_bonus - equipped.health_bonus,
        candidate.armor_bonus - equipped.armor_bonus,
        item_affix_callout_suffix(candidate, equipped)
    )
}

fn legendary_power_score(power: LegendaryPower) -> f32 {
    match power {
        LegendaryPower::None => 0.0,
        LegendaryPower::Emberbrand => 30.0,
        LegendaryPower::Frostbrand => 30.0,
        LegendaryPower::Stormbrand => 30.0,
        LegendaryPower::Soulreaver => 30.0,
        LegendaryPower::Aegisbrand => 30.0,
    }
}

fn gem_power_score(gem: Option<SocketedGem>) -> f32 {
    gem.map(|gem| 10.0 + gem.rank as f32 * 6.0)
        .unwrap_or_default()
}

impl<'a> From<&'a InventoryItem> for ItemStats<'a> {
    fn from(item: &'a InventoryItem) -> Self {
        Self {
            quality: &item.quality,
            damage_bonus: item.damage_bonus,
            crit_chance: item.crit_chance,
            health_bonus: item.health_bonus,
            armor_bonus: item.armor_bonus,
            legendary_power: item.legendary_power,
            temper_level: item.temper_level,
            socketed_gem: item.socketed_gem,
        }
    }
}

impl<'a> From<&'a LootDrop> for ItemStats<'a> {
    fn from(drop: &'a LootDrop) -> Self {
        Self {
            quality: &drop.quality,
            damage_bonus: drop.damage_bonus,
            crit_chance: drop.crit_chance,
            health_bonus: drop.health_bonus,
            armor_bonus: drop.armor_bonus,
            legendary_power: drop.legendary_power,
            temper_level: drop.temper_level,
            socketed_gem: drop.socketed_gem,
        }
    }
}

fn equipped_item_power(equipment: &Equipment, damage_bonus: &DamageBonus) -> f32 {
    item_power(equipped_item_stats(equipment, damage_bonus))
}

fn equipped_item_stats<'a>(
    equipment: &'a Equipment,
    damage_bonus: &'a DamageBonus,
) -> ItemStats<'a> {
    ItemStats {
        quality: &equipment.quality,
        damage_bonus: damage_bonus.0,
        crit_chance: equipment.crit_chance,
        health_bonus: equipment.health_bonus,
        armor_bonus: equipment.armor_bonus,
        legendary_power: equipment.legendary_power,
        temper_level: equipment.temper_level,
        socketed_gem: equipment.socketed_gem,
    }
}

fn roll_weapon<'a>(loot_table: &'a LootTable, rng: &mut impl Rng) -> &'a crate::data::LootEntry {
    let entries = loot_table.weapons.iter().collect::<Vec<_>>();
    roll_from_weighted_entries(entries, &loot_table.weapons[0], rng)
}

fn roll_enemy_weapon<'a>(
    enemy_id: &str,
    is_elite: bool,
    magic_find: bool,
    loot_table: &'a LootTable,
    rng: &mut impl Rng,
) -> &'a crate::data::LootEntry {
    if enemy_id == "keeper" {
        return roll_boss_weapon(loot_table, rng);
    }
    if enemy_id == "treasure_imp" {
        return roll_cache_weapon(loot_table, rng);
    }
    if is_elite || magic_find {
        return roll_cache_weapon(loot_table, rng);
    }
    roll_weapon(loot_table, rng)
}

fn roll_boss_weapon<'a>(
    loot_table: &'a LootTable,
    rng: &mut impl Rng,
) -> &'a crate::data::LootEntry {
    let legendary = loot_table
        .weapons
        .iter()
        .filter(|entry| quality_is_legendary_or_better(&entry.quality))
        .collect::<Vec<_>>();
    if legendary.is_empty() {
        return roll_cache_weapon(loot_table, rng);
    }
    roll_from_weighted_entries(legendary, &loot_table.weapons[0], rng)
}

fn quality_is_legendary_or_better(quality: &str) -> bool {
    gear_quality_is_legendary_or_better(quality)
}

fn roll_from_weighted_entries<'a>(
    entries: Vec<&'a crate::data::LootEntry>,
    fallback: &'a crate::data::LootEntry,
    rng: &mut impl Rng,
) -> &'a crate::data::LootEntry {
    let total_weight: u32 = entries.iter().map(|entry| entry.weight).sum();
    let mut roll = rng.random_range(0..total_weight.max(1));
    for entry in entries {
        if roll < entry.weight {
            return entry;
        }
        roll -= entry.weight;
    }
    fallback
}

fn roll_cache_weapon<'a>(
    loot_table: &'a LootTable,
    rng: &mut impl Rng,
) -> &'a crate::data::LootEntry {
    let upgraded = loot_table
        .weapons
        .iter()
        .filter(|entry| entry.quality != "common")
        .collect::<Vec<_>>();
    if upgraded.is_empty() {
        return roll_weapon(loot_table, rng);
    }

    roll_from_weighted_entries(upgraded, &loot_table.weapons[0], rng)
}

fn roll_champion_cache_weapon<'a>(
    loot_table: &'a LootTable,
    rng: &mut impl Rng,
) -> &'a crate::data::LootEntry {
    roll_cache_weapon(loot_table, rng)
}

fn enemy_loot_claim_mode(enemy_id: &str) -> LootClaimMode {
    if enemy_id == "keeper" {
        LootClaimMode::Direct
    } else {
        LootClaimMode::Ground
    }
}

fn death_burst_kind(enemy_id: &str, is_elite: bool) -> DeathBurstKind {
    if enemy_id == "keeper" {
        DeathBurstKind::Cataclysm
    } else if enemy_id == "treasure_imp" || enemy_id == "nemesis" {
        DeathBurstKind::Prism
    } else if is_elite {
        DeathBurstKind::Burst
    } else {
        DeathBurstKind::Spark
    }
}

fn death_remains_kind(enemy_id: &str, is_elite: bool) -> DeathRemainsKind {
    if enemy_id == "keeper" {
        DeathRemainsKind::BossSeal
    } else if enemy_id == "treasure_imp" {
        DeathRemainsKind::TreasureAsh
    } else if enemy_id == "skeleton" || enemy_id == "bone_stalker" {
        DeathRemainsKind::Bone
    } else if enemy_id == "nemesis" || enemy_id == "reliquary_brute" || is_elite {
        DeathRemainsKind::EliteBrand
    } else {
        DeathRemainsKind::Ash
    }
}

fn death_screen_shake_event(enemy_id: &str, is_elite: bool) -> Option<ScreenShakeEvent> {
    if enemy_id == "keeper" {
        Some(ScreenShakeEvent::new(0.18, 0.36))
    } else if enemy_id == "treasure_imp" || enemy_id == "nemesis" || is_elite {
        Some(ScreenShakeEvent::new(0.09, 0.2))
    } else {
        None
    }
}

fn death_remains_style(kind: DeathRemainsKind) -> DeathRemainsStyle {
    match kind {
        DeathRemainsKind::Ash => DeathRemainsStyle {
            radius: 0.58,
            height: 0.035,
            lifetime_secs: 3.2,
            lift: 0.018,
            alpha: 0.42,
            color: Color::srgb(0.13, 0.018, 0.012),
            emissive: Color::srgb(0.02, 0.004, 0.002),
        },
        DeathRemainsKind::Bone => DeathRemainsStyle {
            radius: 0.64,
            height: 0.035,
            lifetime_secs: 3.8,
            lift: 0.02,
            alpha: 0.48,
            color: Color::srgb(0.36, 0.32, 0.25),
            emissive: Color::srgb(0.035, 0.025, 0.018),
        },
        DeathRemainsKind::EliteBrand => DeathRemainsStyle {
            radius: 0.9,
            height: 0.045,
            lifetime_secs: 5.2,
            lift: 0.024,
            alpha: 0.56,
            color: Color::srgb(0.42, 0.025, 0.012),
            emissive: Color::srgb(0.16, 0.018, 0.006),
        },
        DeathRemainsKind::TreasureAsh => DeathRemainsStyle {
            radius: 0.82,
            height: 0.04,
            lifetime_secs: 4.6,
            lift: 0.022,
            alpha: 0.54,
            color: Color::srgb(0.72, 0.42, 0.08),
            emissive: Color::srgb(0.18, 0.09, 0.018),
        },
        DeathRemainsKind::BossSeal => DeathRemainsStyle {
            radius: 1.35,
            height: 0.055,
            lifetime_secs: 7.4,
            lift: 0.03,
            alpha: 0.64,
            color: Color::srgb(0.62, 0.055, 0.018),
            emissive: Color::srgb(0.32, 0.036, 0.012),
        },
    }
}

fn death_burst_style(kind: DeathBurstKind) -> DeathBurstStyle {
    match kind {
        DeathBurstKind::Spark => DeathBurstStyle {
            scale: 0.9,
            lifetime_secs: 0.32,
            lift: 0.82,
            light_intensity: 260.0,
            light_range: 2.4,
            light_color: Color::srgb(1.0, 0.38, 0.18),
        },
        DeathBurstKind::Burst => DeathBurstStyle {
            scale: 1.55,
            lifetime_secs: 0.55,
            lift: 1.02,
            light_intensity: 560.0,
            light_range: 3.6,
            light_color: Color::srgb(1.0, 0.22, 0.08),
        },
        DeathBurstKind::Prism => DeathBurstStyle {
            scale: 1.35,
            lifetime_secs: 0.72,
            lift: 1.12,
            light_intensity: 740.0,
            light_range: 4.4,
            light_color: Color::srgb(1.0, 0.78, 0.28),
        },
        DeathBurstKind::Cataclysm => DeathBurstStyle {
            scale: 2.05,
            lifetime_secs: 0.92,
            lift: 1.24,
            light_intensity: 980.0,
            light_range: 5.8,
            light_color: Color::srgb(1.0, 0.36, 0.12),
        },
    }
}

fn death_burst_scale_factor(progress: f32) -> f32 {
    let progress = progress.clamp(0.0, 1.0);
    1.0 + (1.0 - progress) * 0.34 + (progress * std::f32::consts::PI).sin() * 0.18
}

fn death_burst_light_factor(progress: f32) -> f32 {
    let progress = progress.clamp(0.0, 1.0);
    (1.0 - progress).powf(1.7)
}

fn death_remains_scale_factor(progress: f32) -> f32 {
    let progress = progress.clamp(0.0, 1.0);
    1.0 + progress * 0.16
}

fn death_remains_alpha_factor(progress: f32) -> f32 {
    let progress = progress.clamp(0.0, 1.0);
    (1.0 - progress).powf(1.35)
}

fn spawn_death_burst_vfx(
    commands: &mut Commands,
    assets: &GameAssets,
    position: Vec3,
    kind: DeathBurstKind,
) {
    let scene = match kind {
        DeathBurstKind::Spark => assets.hit_spark.clone(),
        DeathBurstKind::Burst => assets.crit_burst.clone(),
        DeathBurstKind::Prism => assets.loot_prism.clone(),
        DeathBurstKind::Cataclysm => assets.boss_summon_portal.clone(),
    };
    let style = death_burst_style(kind);
    let base_scale = Vec3::splat(style.scale);
    commands.spawn((
        WorldAssetRoot(scene),
        Transform::from_translation(position + Vec3::Y * style.lift)
            .with_scale(base_scale * death_burst_scale_factor(0.0)),
        PointLight {
            intensity: style.light_intensity,
            range: style.light_range,
            color: style.light_color,
            shadow_maps_enabled: false,
            ..default()
        },
        DeathBurstVfx {
            timer: Timer::from_seconds(style.lifetime_secs, TimerMode::Once),
            duration_secs: style.lifetime_secs,
            base_scale,
            base_intensity: style.light_intensity,
        },
        Name::new("Death Burst VFX"),
    ));
}

fn spawn_elite_affix_break_death_vfx(
    commands: &mut Commands,
    assets: &GameAssets,
    position: Vec3,
    affix_count: usize,
) {
    let style = elite_affix_break_death_style(affix_count);
    let base_scale = Vec3::splat(style.scale);
    commands.spawn((
        WorldAssetRoot(assets.elite_affix_break.clone()),
        Transform::from_translation(position + Vec3::Y * style.lift).with_scale(base_scale),
        PointLight {
            intensity: style.light_intensity,
            range: style.light_range,
            color: style.light_color,
            shadow_maps_enabled: false,
            ..default()
        },
        DeathBurstVfx {
            timer: Timer::from_seconds(style.lifetime_secs, TimerMode::Once),
            duration_secs: style.lifetime_secs,
            base_scale,
            base_intensity: style.light_intensity,
        },
        Name::new("Elite Affix Break VFX"),
    ));
}

fn elite_affix_break_death_style(affix_count: usize) -> DeathBurstStyle {
    let pressure = affix_count.saturating_sub(2).min(4) as f32;
    DeathBurstStyle {
        scale: 1.08 + pressure * 0.08,
        lift: 0.44 + pressure * 0.03,
        light_intensity: 780.0 + pressure * 90.0,
        light_range: 4.6 + pressure * 0.35,
        light_color: Color::srgb(0.70, 0.38, 1.0),
        lifetime_secs: 0.58 + pressure * 0.04,
    }
}

fn spawn_death_remains(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    position: Vec3,
    kind: DeathRemainsKind,
) {
    let style = death_remains_style(kind);
    let base_scale = Vec3::new(1.0, 1.0, 1.0);
    let material = materials.add(StandardMaterial {
        base_color: style.color.with_alpha(style.alpha),
        emissive: style.emissive.into(),
        alpha_mode: AlphaMode::Blend,
        perceptual_roughness: 0.92,
        ..default()
    });
    commands.spawn((
        Mesh3d(meshes.add(Cylinder::new(style.radius, style.height))),
        MeshMaterial3d(material),
        Transform::from_translation(position + Vec3::Y * style.lift)
            .with_scale(base_scale * death_remains_scale_factor(0.0)),
        DeathRemains {
            timer: Timer::from_seconds(style.lifetime_secs, TimerMode::Once),
            duration_secs: style.lifetime_secs,
            base_scale,
            base_alpha: style.alpha,
        },
        Name::new("Death Remains"),
    ));
}

fn loot_drop_from_weapon(gold: u32, weapon: &crate::data::LootEntry) -> LootDrop {
    LootDrop {
        gold,
        damage_bonus: weapon.damage_bonus,
        crit_chance: weapon.crit_chance,
        health_bonus: weapon.health_bonus,
        armor_bonus: weapon.armor_bonus,
        legendary_power: weapon.legendary_power,
        temper_level: 0,
        socketed_gem: None,
        bonus_gem: roll_gem_for_quality(&weapon.quality),
        bonus_charm: roll_charm_for_quality(&weapon.quality),
        potions: weapon.potions,
        elixirs: elixirs_for_quality(&weapon.quality),
        quality: weapon.quality.clone(),
        label: format!("{} {}", weapon.quality, weapon.name),
    }
}

fn spawn_loot_drop(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    position: Vec3,
    gold: u32,
    weapon: &crate::data::LootEntry,
    source_name: String,
) {
    let drop = loot_drop_from_weapon(gold, weapon);
    let color = loot_quality_color(&weapon.quality);
    let beam_height = loot_beam_height(&weapon.quality);
    let pulse = loot_pulse_for_quality(&weapon.quality);
    let ground_ring = loot_ground_ring_for_quality(&weapon.quality);
    let label_text = loot_label_text(&drop);

    let loot_entity = commands
        .spawn((
            Mesh3d(meshes.add(Cuboid::new(0.45, 0.18, 0.45))),
            MeshMaterial3d(materials.add(StandardMaterial {
                base_color: color,
                emissive: color.into(),
                ..default()
            })),
            Transform::from_translation(position),
            pulse,
            drop,
            LootEntity,
            Name::new(source_name),
        ))
        .id();

    let label_entity = commands
        .spawn((
            Text2d::new(label_text),
            TextFont {
                font_size: FontSize::Px(loot_label_font_size(&weapon.quality)),
                ..default()
            },
            TextColor(color),
            Transform::from_xyz(0.0, 1.04, 0.0).with_scale(Vec3::splat(0.012)),
            LootLabel { owner: loot_entity },
            LootFilterVisual {
                min_rank: loot_quality_rank(&weapon.quality),
            },
            LootEntity,
            Name::new("Loot Label"),
        ))
        .id();
    commands.entity(loot_entity).add_child(label_entity);

    let ring_entity = commands
        .spawn((
            Mesh3d(meshes.add(Cylinder::new(
                loot_ground_ring_radius(&weapon.quality),
                0.035,
            ))),
            MeshMaterial3d(materials.add(StandardMaterial {
                base_color: color.with_alpha(loot_ground_ring_alpha(&weapon.quality)),
                emissive: color.into(),
                alpha_mode: AlphaMode::Blend,
                ..default()
            })),
            Transform::from_xyz(0.0, 0.025, 0.0).with_scale(ground_ring.base_scale),
            ground_ring,
            LootFilterVisual {
                min_rank: loot_quality_rank(&weapon.quality),
            },
            LootEntity,
            Name::new("Loot Ground Ring"),
        ))
        .id();
    commands.entity(loot_entity).add_child(ring_entity);

    if beam_height > 0.0 {
        let radius = loot_beam_radius(&weapon.quality);
        let beam_entity = commands
            .spawn((
                Mesh3d(meshes.add(Cylinder::new(radius, beam_height))),
                MeshMaterial3d(materials.add(StandardMaterial {
                    base_color: color.with_alpha(0.34),
                    emissive: color.into(),
                    alpha_mode: AlphaMode::Blend,
                    ..default()
                })),
                Transform::from_xyz(0.0, beam_height * 0.5 + 0.16, 0.0),
                LootBeam {
                    base_radius: radius,
                },
                LootFilterVisual {
                    min_rank: loot_quality_rank(&weapon.quality),
                },
                LootEntity,
                Name::new("Loot Rarity Beam"),
            ))
            .id();
        commands.entity(loot_entity).add_child(beam_entity);

        let glow_entity = commands
            .spawn((
                PointLight {
                    color,
                    intensity: loot_glow_intensity(&weapon.quality),
                    range: loot_glow_range(&weapon.quality),
                    shadow_maps_enabled: false,
                    ..default()
                },
                Transform::from_xyz(0.0, beam_height.min(2.2) * 0.55 + 0.42, 0.0),
                LootGlow {
                    base_intensity: loot_glow_intensity(&weapon.quality),
                    pulse_speed: pulse.pulse_speed,
                    pulse_amount: 0.18,
                },
                LootFilterVisual {
                    min_rank: loot_quality_rank(&weapon.quality),
                },
                LootEntity,
                Name::new("Loot Rarity Glow"),
            ))
            .id();
        commands.entity(loot_entity).add_child(glow_entity);
    }
}

fn loot_quality_color(quality: &str) -> Color {
    match quality {
        "rare" => Color::srgb(0.20, 0.42, 1.0),
        "legendary" => Color::srgb(1.0, 0.45, 0.08),
        "ancient" => Color::srgb(0.95, 0.18, 0.08),
        "primal" => Color::srgb(1.0, 0.88, 0.38),
        _ => Color::srgb(0.75, 0.75, 0.75),
    }
}

fn loot_quality_rank(quality: &str) -> u8 {
    match quality {
        "primal" => 4,
        "ancient" => 3,
        "legendary" => 2,
        "rare" => 1,
        _ => 0,
    }
}

fn loot_label_text(drop: &LootDrop) -> String {
    let primary = loot_label_primary_text(drop);
    let rewards = loot_label_reward_summary(drop);
    let header = loot_label_rarity_header(&drop.quality);
    let mut lines = Vec::new();
    if !header.is_empty() {
        lines.push(header.to_string());
    }
    lines.push(primary);
    if rewards.is_empty() {
        lines.join("\n")
    } else {
        lines.push(rewards);
        lines.join("\n")
    }
}

fn loot_label_text_with_compare(
    drop: &LootDrop,
    equipped: ItemStats,
    inventory_has_room: bool,
) -> String {
    let mut lines = Vec::new();
    let header = loot_label_rarity_header(&drop.quality);
    if !header.is_empty() {
        lines.push(header.to_string());
    }
    lines.push(loot_label_primary_text(drop));
    lines.push(loot_ground_compare_summary(
        drop,
        equipped,
        inventory_has_room,
    ));
    let rewards = loot_label_reward_summary(drop);
    if !rewards.is_empty() {
        lines.push(rewards);
    }
    lines.join("\n")
}

fn loot_label_rarity_header(quality: &str) -> &'static str {
    match quality {
        "primal" => "PRIMAL DROP",
        "ancient" => "ANCIENT DROP",
        "legendary" => "LEGENDARY DROP",
        _ => "",
    }
}

fn loot_label_primary_text(drop: &LootDrop) -> String {
    let mut primary = vec![drop.label.clone(), format!("+{:.0} dmg", drop.damage_bonus)];
    if drop.crit_chance > 0.0 {
        primary.push(format!("+{:.0}% crit", drop.crit_chance * 100.0));
    }
    if drop.health_bonus > 0.0 {
        primary.push(format!("+{:.0} hp", drop.health_bonus));
    }
    if drop.armor_bonus > 0.0 {
        primary.push(format!("+{:.0} armor", drop.armor_bonus));
    }
    if !matches!(drop.legendary_power, LegendaryPower::None) {
        primary.push(drop.legendary_power.label().to_string());
    }
    primary.join(" | ")
}

fn loot_ground_compare_summary(
    drop: &LootDrop,
    equipped: ItemStats,
    inventory_has_room: bool,
) -> String {
    let candidate = ItemStats::from(drop);
    let power_delta = item_power(candidate) - item_power(equipped);
    let decision = loot_ground_decision(power_delta, inventory_has_room);
    let role = loot_role_tag(candidate, equipped, power_delta, inventory_has_room);

    format!(
        "{decision} {} [{role}] | PWR {:+.0} | dmg {:+.0} crit {:+.0}% hp {:+.0} armor {:+.0}{}",
        loot_decision_reason(candidate, equipped, power_delta, inventory_has_room),
        power_delta,
        candidate.damage_bonus - equipped.damage_bonus,
        (candidate.crit_chance - equipped.crit_chance) * 100.0,
        candidate.health_bonus - equipped.health_bonus,
        candidate.armor_bonus - equipped.armor_bonus,
        item_affix_callout_suffix(candidate, equipped)
    )
}

fn loot_ground_decision(power_delta: f32, inventory_has_room: bool) -> &'static str {
    if power_delta >= 0.0 {
        "AUTO-EQUIP"
    } else if power_delta >= -5.0 {
        "SIDEGRADE"
    } else if inventory_has_room {
        "STASH"
    } else {
        "FULL"
    }
}

fn loot_focus_score(
    drop: &LootDrop,
    equipped: ItemStats,
    inventory_has_room: bool,
    distance: f32,
) -> f32 {
    let candidate = ItemStats::from(drop);
    let power_delta = item_power(candidate) - item_power(equipped);
    let mut score = loot_quality_rank(&drop.quality) as f32 * 36.0 + power_delta.max(-20.0);
    if candidate.legendary_power != LegendaryPower::None
        && candidate.legendary_power != equipped.legendary_power
    {
        score += 90.0;
    }
    if power_delta >= 0.0 {
        score += 72.0;
    } else if power_delta >= -5.0 {
        score += 28.0;
    }
    if drop.socketed_gem.is_some() || drop.bonus_gem.is_some() {
        score += 18.0;
    }
    if drop.bonus_charm.is_some() {
        score += 22.0;
    }
    score += drop.gold as f32 * 0.03 + drop.potions as f32 * 4.0 + drop.elixirs as f32 * 6.0;
    if !inventory_has_room && power_delta < 0.0 {
        score -= 45.0;
    }
    score - distance * 5.0
}

fn loot_priority_label(
    drop: &LootDrop,
    equipped: ItemStats,
    inventory_has_room: bool,
) -> &'static str {
    let candidate = ItemStats::from(drop);
    let power_delta = item_power(candidate) - item_power(equipped);
    if (candidate.legendary_power != LegendaryPower::None
        && candidate.legendary_power != equipped.legendary_power)
        || power_delta >= 0.0
    {
        "BEST"
    } else if power_delta >= -5.0
        || loot_quality_rank(&drop.quality) >= loot_quality_rank("legendary")
        || drop.socketed_gem.is_some()
        || drop.bonus_gem.is_some()
        || drop.bonus_charm.is_some()
    {
        "NEXT"
    } else if inventory_has_room {
        "LOW"
    } else {
        "SALVAGE"
    }
}

fn loot_role_tag(
    candidate: ItemStats,
    equipped: ItemStats,
    power_delta: f32,
    inventory_has_room: bool,
) -> &'static str {
    if candidate.legendary_power != LegendaryPower::None
        && candidate.legendary_power != equipped.legendary_power
    {
        return "CODEX";
    }
    if gear_quality_is_ancient_or_better(candidate.quality)
        || candidate.socketed_gem.is_some()
        || candidate.temper_level > equipped.temper_level
    {
        return "CRAFT";
    }
    if candidate.damage_bonus - equipped.damage_bonus >= 5.0
        || (candidate.crit_chance - equipped.crit_chance) * 100.0 >= 5.0
    {
        return "DPS";
    }
    if candidate.health_bonus - equipped.health_bonus + candidate.armor_bonus - equipped.armor_bonus
        >= 20.0
    {
        return "TANK";
    }
    if power_delta >= -5.0 {
        return "SIDE";
    }
    if inventory_has_room {
        "STASH"
    } else {
        "SALVAGE"
    }
}

fn loot_decision_reason(
    candidate: ItemStats,
    equipped: ItemStats,
    power_delta: f32,
    inventory_has_room: bool,
) -> &'static str {
    if candidate.legendary_power != LegendaryPower::None
        && candidate.legendary_power != equipped.legendary_power
    {
        return "BUILD POWER";
    }
    if power_delta >= 25.0 {
        return "POWER SPIKE";
    }
    if candidate.damage_bonus - equipped.damage_bonus >= 5.0
        || (candidate.crit_chance - equipped.crit_chance) * 100.0 >= 5.0
    {
        return "DPS GAIN";
    }
    if candidate.health_bonus - equipped.health_bonus + candidate.armor_bonus - equipped.armor_bonus
        >= 20.0
    {
        return "TOUGHNESS";
    }
    if matches!(
        (candidate.socketed_gem, equipped.socketed_gem),
        (Some(_), None)
    ) || matches!(
        (candidate.socketed_gem, equipped.socketed_gem),
        (Some(candidate_gem), Some(equipped_gem)) if candidate_gem.rank > equipped_gem.rank
    ) {
        return "SOCKET";
    }
    if candidate.temper_level > equipped.temper_level {
        return "TEMPER";
    }
    if power_delta >= -5.0 {
        return "BUILD SWAP";
    }
    if inventory_has_room {
        "SALVAGE LATER"
    } else {
        "MAKE ROOM"
    }
}

fn item_affix_callout_suffix(candidate: ItemStats, equipped: ItemStats) -> String {
    let callouts = item_affix_callouts(candidate, equipped);
    if callouts.is_empty() {
        String::new()
    } else {
        format!(" | {}", callouts.join(" | "))
    }
}

fn item_affix_callouts(candidate: ItemStats, equipped: ItemStats) -> Vec<String> {
    let mut callouts = Vec::new();
    if candidate.legendary_power != LegendaryPower::None
        && candidate.legendary_power != equipped.legendary_power
    {
        let prefix = if equipped.legendary_power == LegendaryPower::None {
            "new power"
        } else {
            "power swap"
        };
        callouts.push(format!("{prefix} {}", candidate.legendary_power.label()));
        callouts.push(candidate.legendary_power.build_role().to_string());
    }

    match (candidate.socketed_gem, equipped.socketed_gem) {
        (Some(candidate_gem), Some(equipped_gem)) if candidate_gem.rank > equipped_gem.rank => {
            callouts.push(format!("socket {}", candidate_gem.label()));
        }
        (Some(candidate_gem), None) => callouts.push(format!("socket {}", candidate_gem.label())),
        _ => {}
    }

    if candidate.temper_level > equipped.temper_level {
        callouts.push(format!(
            "temper +{}",
            candidate.temper_level - equipped.temper_level
        ));
    }

    let damage_delta = candidate.damage_bonus - equipped.damage_bonus;
    if damage_delta >= 5.0 {
        callouts.push(format!("weapon dmg +{damage_delta:.0}"));
    }

    let crit_delta = (candidate.crit_chance - equipped.crit_chance) * 100.0;
    if crit_delta >= 5.0 {
        callouts.push(format!("crit build +{crit_delta:.0}%"));
    }

    let toughness_delta = candidate.health_bonus - equipped.health_bonus + candidate.armor_bonus
        - equipped.armor_bonus;
    if toughness_delta >= 20.0 {
        callouts.push(format!("toughness +{toughness_delta:.0}"));
    }

    callouts
}

fn loot_label_reward_summary(drop: &LootDrop) -> String {
    let mut rewards = Vec::new();
    if drop.gold > 0 {
        rewards.push(format!("{} gold", drop.gold));
    }
    if drop.bonus_gem.is_some() {
        rewards.push("gem".to_string());
    }
    if drop.bonus_charm.is_some() {
        rewards.push("charm".to_string());
    }
    if drop.potions > 0 {
        rewards.push(format!("{} potion", drop.potions));
    }
    if drop.elixirs > 0 {
        rewards.push(format!("{} elixir", drop.elixirs));
    }
    rewards.join(" | ")
}

fn loot_label_font_size(quality: &str) -> f32 {
    match quality {
        "primal" => 24.0,
        "ancient" => 22.0,
        "legendary" => 21.0,
        "rare" => 18.0,
        _ => 16.0,
    }
}

fn loot_ground_ring_radius(quality: &str) -> f32 {
    match quality {
        "primal" => 0.72,
        "ancient" => 0.64,
        "legendary" => 0.58,
        "rare" => 0.50,
        _ => 0.42,
    }
}

fn loot_ground_ring_alpha(quality: &str) -> f32 {
    match quality {
        "primal" => 0.46,
        "ancient" => 0.40,
        "legendary" => 0.34,
        "rare" => 0.28,
        _ => 0.20,
    }
}

fn loot_beam_height(quality: &str) -> f32 {
    match quality {
        "primal" => 1.9,
        "ancient" => 1.6,
        "legendary" => 1.35,
        "rare" => 0.9,
        _ => 0.0,
    }
}

fn loot_beam_radius(quality: &str) -> f32 {
    match quality {
        "primal" => 0.11,
        "ancient" => 0.1,
        "legendary" => 0.09,
        "rare" => 0.075,
        _ => 0.0,
    }
}

fn loot_pulse_for_quality(quality: &str) -> LootPulse {
    LootPulse {
        base_scale: Vec3::ONE,
        spin_speed: loot_spin_speed(quality),
        pulse_speed: loot_pulse_speed(quality),
        pulse_amount: loot_pulse_amount(quality),
    }
}

fn loot_ground_ring_for_quality(quality: &str) -> LootGroundRing {
    LootGroundRing {
        base_scale: Vec3::new(1.0, 1.0, 1.0),
        spin_speed: loot_spin_speed(quality) * 0.45,
        pulse_speed: loot_pulse_speed(quality) * 0.82,
        pulse_amount: loot_pulse_amount(quality) * 1.35,
    }
}

fn loot_spin_speed(quality: &str) -> f32 {
    match quality {
        "primal" => 1.35,
        "ancient" => 1.05,
        "legendary" => 0.82,
        "rare" => 0.52,
        _ => 0.22,
    }
}

fn loot_pulse_speed(quality: &str) -> f32 {
    match quality {
        "primal" => 3.6,
        "ancient" => 3.1,
        "legendary" => 2.6,
        "rare" => 2.1,
        _ => 1.5,
    }
}

fn loot_pulse_amount(quality: &str) -> f32 {
    match quality {
        "primal" => 0.12,
        "ancient" => 0.10,
        "legendary" => 0.08,
        "rare" => 0.05,
        _ => 0.025,
    }
}

fn loot_pulse_factor(elapsed_secs: f32, pulse_speed: f32, pulse_amount: f32) -> f32 {
    1.0 + (elapsed_secs * pulse_speed).sin() * pulse_amount
}

fn loot_glow_intensity(quality: &str) -> f32 {
    match quality {
        "primal" => 620.0,
        "ancient" => 460.0,
        "legendary" => 330.0,
        "rare" => 180.0,
        _ => 0.0,
    }
}

fn loot_glow_range(quality: &str) -> f32 {
    match quality {
        "primal" => 5.2,
        "ancient" => 4.4,
        "legendary" => 3.6,
        "rare" => 2.6,
        _ => 0.0,
    }
}

fn roll_gem_for_quality(quality: &str) -> Option<SocketedGem> {
    let mut rng = rand::rng();
    let chance = match quality {
        "primal" => 1.0,
        "ancient" => 1.0,
        "legendary" => 1.0,
        "rare" => 0.55,
        _ => 0.18,
    };
    if rng.random::<f32>() >= chance {
        return None;
    }
    let kind = match rng.random_range(0..4) {
        0 => GemKind::Ruby,
        1 => GemKind::Emerald,
        2 => GemKind::Amethyst,
        _ => GemKind::Topaz,
    };
    Some(SocketedGem::new(kind))
}

fn roll_charm_for_quality(quality: &str) -> Option<Charm> {
    let mut rng = rand::rng();
    let chance = match quality {
        "primal" => 1.0,
        "ancient" => 0.95,
        "legendary" => 0.85,
        "rare" => 0.35,
        _ => 0.08,
    };
    if rng.random::<f32>() >= chance {
        return None;
    }
    Some(charm_for_roll(quality, rng.random_range(0..3)))
}

fn charm_for_roll(quality: &str, roll: u32) -> Charm {
    let (name, damage_bonus, crit_chance) = match (quality, roll) {
        ("primal", 0) => ("Primal Stormglass Charm", 11.0, 0.065),
        ("primal", 1) => ("Primal Bloodstar Charm", 13.0, 0.045),
        ("primal", _) => ("Primal Ashen Saint Charm", 9.0, 0.075),
        ("ancient", 0) => ("Ancient Stormglass Charm", 8.0, 0.050),
        ("ancient", 1) => ("Ancient Bloodstar Charm", 10.0, 0.030),
        ("ancient", _) => ("Ancient Ashen Saint Charm", 6.0, 0.060),
        ("legendary", 0) => ("Stormglass Charm", 5.0, 0.035),
        ("legendary", 1) => ("Bloodstar Charm", 7.0, 0.018),
        ("legendary", _) => ("Ashen Saint Charm", 4.0, 0.045),
        ("rare", 0) => ("Gilded Fang Charm", 3.0, 0.018),
        ("rare", 1) => ("Cinder Eye Charm", 2.0, 0.025),
        ("rare", _) => ("Reliquary Bone Charm", 4.0, 0.010),
        (_, 0) => ("Cracked Fang Charm", 1.0, 0.006),
        (_, 1) => ("Dull Ember Charm", 0.0, 0.012),
        _ => ("Iron Chip Charm", 2.0, 0.0),
    };
    Charm {
        name: name.to_string(),
        quality: quality.to_string(),
        damage_bonus,
        crit_chance,
    }
}

fn equip_charm_if_better(current: &mut Charm, candidate: &Charm) -> bool {
    if charm_power(candidate) < charm_power(current) {
        return false;
    }
    *current = candidate.clone();
    true
}

fn elixirs_for_quality(quality: &str) -> u32 {
    match quality {
        "primal" => 3,
        "ancient" => 2,
        "legendary" => 1,
        "rare" => 1,
        _ => 0,
    }
}

fn health_globe_drop_chance(enemy_id: &str, is_elite: bool) -> f32 {
    if enemy_id == "keeper" {
        1.0
    } else if enemy_id == "treasure_imp" {
        0.0
    } else if is_elite {
        0.65
    } else {
        0.18
    }
}

fn health_globe_drops(enemy_id: &str, is_elite: bool, roll: f32) -> bool {
    roll < health_globe_drop_chance(enemy_id, is_elite)
}

fn fury_globe_drop_chance(enemy_id: &str, is_elite: bool) -> f32 {
    if enemy_id == "keeper" {
        1.0
    } else if enemy_id == "seal_warden" {
        0.85
    } else if enemy_id == "treasure_imp" {
        0.35
    } else if is_elite {
        0.42
    } else {
        0.10
    }
}

fn fury_globe_drops(enemy_id: &str, is_elite: bool, roll: f32) -> bool {
    roll < fury_globe_drop_chance(enemy_id, is_elite)
}

fn enemy_shard_reward(enemy_id: &str, is_elite: bool) -> u32 {
    if enemy_id == "keeper" {
        6
    } else if enemy_id == "treasure_imp" {
        10
    } else if enemy_id == "nemesis" {
        6
    } else if is_elite {
        2
    } else {
        0
    }
}

fn enemy_affix_essence_reward(enemy_id: &str, affix_count: usize) -> u32 {
    if affix_count == 0 {
        return 0;
    }
    let base = affix_count as u32;
    if enemy_id == "keeper" || enemy_id == "nemesis" {
        base + 2
    } else if enemy_id == "treasure_imp" {
        base + 1
    } else {
        base
    }
}

fn elite_affix_break_essence_reward(enemy_id: &str, affix_count: usize) -> u32 {
    if affix_count < 2 || enemy_id == "keeper" {
        return 0;
    }
    let base = affix_count.saturating_sub(1) as u32;
    if enemy_id == "nemesis" {
        base + 2
    } else {
        base
    }
}

fn elite_affix_break_reward_text(enemy_name: &str, essence: u32) -> String {
    format!("Affix Break: {enemy_name} shattered, +{essence} essence")
}

fn elite_affix_break_float_text(essence: u32) -> String {
    format!("AFFIX BREAK +{essence}E")
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum CodexDropClaim {
    None,
    Unlocked,
    DuplicateEssence(u32),
}

fn claim_codex_drop_power(
    codex: &mut LegendaryCodex,
    stats: &mut RunStats,
    power: LegendaryPower,
    quality: &str,
) -> CodexDropClaim {
    if power == LegendaryPower::None {
        return CodexDropClaim::None;
    }
    if codex.unlock(power) {
        return CodexDropClaim::Unlocked;
    }
    let essence = duplicate_codex_essence_reward(quality);
    if essence == 0 {
        return CodexDropClaim::None;
    }
    stats.affix_essence = stats.affix_essence.saturating_add(essence);
    CodexDropClaim::DuplicateEssence(essence)
}

fn duplicate_codex_essence_reward(quality: &str) -> u32 {
    match quality {
        "primal" => 4,
        "ancient" => 3,
        "legendary" => 2,
        _ => 0,
    }
}

fn affix_hunter_kill(affix_count: usize) -> bool {
    affix_count >= 2
}

fn health_globe_heal_amount(max_health: f32, heal_fraction: f32) -> f32 {
    (max_health * heal_fraction).round().max(1.0)
}

fn fury_globe_amount(max_fury: f32, fury_fraction: f32) -> f32 {
    (max_fury * fury_fraction).round().max(1.0)
}

fn spawn_health_globe(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    position: Vec3,
) {
    let color = Color::srgb(0.95, 0.05, 0.09);
    commands.spawn((
        Mesh3d(meshes.add(Sphere::new(0.24))),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: color,
            emissive: color.into(),
            ..default()
        })),
        Transform::from_translation(position),
        HealthGlobe {
            heal_fraction: 0.30,
        },
        Name::new("Health Globe"),
    ));
}

fn spawn_fury_globe(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    position: Vec3,
) {
    let color = Color::srgb(0.18, 0.58, 1.0);
    commands.spawn((
        Mesh3d(meshes.add(Sphere::new(0.21))),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: color,
            emissive: color.into(),
            ..default()
        })),
        Transform::from_translation(position),
        FuryGlobe {
            fury_fraction: 0.38,
        },
        PointLight {
            color,
            intensity: 160.0,
            range: 2.2,
            shadow_maps_enabled: false,
            ..default()
        },
        Name::new("Fury Globe"),
    ));
}

fn update_loot_readability(
    time: Res<Time>,
    camera: LootReadabilityCameraQuery,
    mut labels: LootLabelTransformQuery,
    mut beams: LootBeamTransformQuery,
    mut pulses: LootPulseTransformQuery,
    mut rings: LootGroundRingTransformQuery,
    mut glows: LootGlowQuery,
) {
    let camera_rotation = camera.single().ok().map(|transform| transform.rotation);
    let elapsed_secs = time.elapsed_secs();
    let pulse = elapsed_secs.sin() * 0.08 + 1.0;
    let label_bob = (time.elapsed_secs() * 2.6).sin() * 0.04;

    for mut transform in &mut labels {
        transform.translation.y = 1.04 + label_bob;
        if let Some(rotation) = camera_rotation {
            transform.rotation = rotation;
        }
    }

    for (mut transform, beam) in &mut beams {
        transform.scale.x = pulse;
        transform.scale.z = pulse;
        let radius_weight = (beam.base_radius * 8.0).clamp(0.6, 1.0);
        transform.scale.y = 1.0 + (pulse - 1.0) * 0.2 * radius_weight;
    }

    for (mut transform, loot_pulse) in &mut pulses {
        transform.rotate_y(loot_pulse.spin_speed * time.delta_secs());
        let scale = loot_pulse.base_scale
            * loot_pulse_factor(
                elapsed_secs,
                loot_pulse.pulse_speed,
                loot_pulse.pulse_amount,
            );
        transform.scale = scale;
    }

    for (mut transform, ring) in &mut rings {
        transform.rotate_y(ring.spin_speed * time.delta_secs());
        transform.scale =
            ring.base_scale * loot_pulse_factor(elapsed_secs, ring.pulse_speed, ring.pulse_amount);
    }

    for (mut light, glow) in &mut glows {
        light.intensity = glow.base_intensity
            * loot_pulse_factor(elapsed_secs, glow.pulse_speed, glow.pulse_amount);
    }
}

fn update_loot_filter_visibility(filter: Res<LootFilterTier>, mut visuals: LootFilterVisualQuery) {
    for (visual, mut visibility) in &mut visuals {
        *visibility = if visual.min_rank >= filter.min_rank() {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
    }
}

fn update_loot_labels(
    player: LootComparePlayerQuery,
    drops: Query<&LootDrop, With<LootEntity>>,
    mut labels: LootLabelTextQuery,
) {
    let Ok((damage_bonus, equipment, inventory)) = player.single() else {
        return;
    };
    let equipped = equipped_item_stats(equipment, damage_bonus);
    let inventory_has_room = inventory.items.len() < inventory.capacity;

    for (label, mut text) in &mut labels {
        let Ok(drop) = drops.get(label.owner) else {
            continue;
        };
        *text = Text2d::new(loot_label_text_with_compare(
            drop,
            equipped,
            inventory_has_room,
        ));
    }
}

fn update_loot_proximity_info(
    player: LootProximityPlayerQuery,
    loot: Query<(&Transform, &LootDrop), With<LootEntity>>,
    mut info: ResMut<LootProximityInfo>,
) {
    let Ok((player_transform, damage_bonus, equipment, inventory)) = player.single() else {
        *info = LootProximityInfo::default();
        return;
    };
    let equipped = equipped_item_stats(equipment, damage_bonus);
    let inventory_has_room = inventory.items.len() < inventory.capacity;
    let Some((drop, distance)) = focused_loot_drop(
        player_transform.translation,
        &loot,
        equipped,
        inventory_has_room,
    ) else {
        *info = LootProximityInfo::default();
        return;
    };

    *info = loot_proximity_info(
        drop,
        distance,
        equipped,
        inventory_has_room,
        salvage_preview(inventory, equipment).items > 0,
    );
}

fn focused_loot_drop<'a>(
    player_position: Vec3,
    loot: &'a Query<(&Transform, &LootDrop), With<LootEntity>>,
    equipped: ItemStats,
    inventory_has_room: bool,
) -> Option<(&'a LootDrop, f32)> {
    focused_loot_drop_from_iter(
        loot.iter()
            .map(|(transform, drop)| (drop, player_position.distance(transform.translation))),
        equipped,
        inventory_has_room,
    )
}

fn focused_loot_drop_from_iter<'a>(
    drops: impl Iterator<Item = (&'a LootDrop, f32)>,
    equipped: ItemStats,
    inventory_has_room: bool,
) -> Option<(&'a LootDrop, f32)> {
    const FOCUS_RADIUS: f32 = LOOT_PICKUP_RADIUS + 3.5;
    let drops = drops.collect::<Vec<_>>();
    drops
        .iter()
        .copied()
        .filter(|(_, distance)| *distance <= FOCUS_RADIUS)
        .max_by(|(left_drop, left_distance), (right_drop, right_distance)| {
            let left = loot_focus_score(left_drop, equipped, inventory_has_room, *left_distance);
            let right = loot_focus_score(right_drop, equipped, inventory_has_room, *right_distance);
            left.total_cmp(&right)
                .then_with(|| right_distance.total_cmp(left_distance))
        })
        .or_else(|| {
            drops
                .into_iter()
                .min_by(|(_, left), (_, right)| left.total_cmp(right))
        })
}

fn loot_proximity_info(
    drop: &LootDrop,
    distance: f32,
    equipped: ItemStats,
    inventory_has_room: bool,
    spare_salvage_available: bool,
) -> LootProximityInfo {
    LootProximityInfo {
        visible: true,
        priority: loot_priority_label(drop, equipped, inventory_has_room).to_string(),
        label: loot_label_primary_text(drop),
        compare: loot_ground_compare_summary(drop, equipped, inventory_has_room),
        rewards: loot_label_reward_summary(drop),
        distance,
        pickup_ready: distance <= LOOT_PICKUP_RADIUS,
        inventory_full: !inventory_has_room,
        spare_salvage_available,
        quality: drop.quality.clone(),
    }
}

fn update_death_burst_vfx(
    time: Res<Time>,
    mut commands: Commands,
    mut bursts: Query<(
        Entity,
        &mut DeathBurstVfx,
        &mut Transform,
        Option<&mut PointLight>,
    )>,
) {
    for (entity, mut burst, mut transform, light) in &mut bursts {
        burst.timer.tick(time.delta());
        let progress = (burst.timer.elapsed_secs() / burst.duration_secs).clamp(0.0, 1.0);
        transform.scale = burst.base_scale * death_burst_scale_factor(progress);
        if let Some(mut light) = light {
            light.intensity = burst.base_intensity * death_burst_light_factor(progress);
        }
        if burst.timer.is_finished()
            && let Ok(mut entity_commands) = commands.get_entity(entity)
        {
            entity_commands.try_despawn();
        }
    }
}

fn update_death_remains(
    time: Res<Time>,
    mut commands: Commands,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut remains: Query<(
        Entity,
        &mut DeathRemains,
        &mut Transform,
        &MeshMaterial3d<StandardMaterial>,
    )>,
) {
    for (entity, mut remains, mut transform, material) in &mut remains {
        remains.timer.tick(time.delta());
        let progress = (remains.timer.elapsed_secs() / remains.duration_secs).clamp(0.0, 1.0);
        transform.scale = remains.base_scale * death_remains_scale_factor(progress);
        if let Some(mut material) = materials.get_mut(&material.0) {
            material.base_color = material
                .base_color
                .with_alpha(remains.base_alpha * death_remains_alpha_factor(progress));
        }
        if remains.timer.is_finished()
            && let Ok(mut entity_commands) = commands.get_entity(entity)
        {
            entity_commands.try_despawn();
        }
    }
}

fn claim_loot_drop(claim: &mut LootClaimState, fortune: &FortuneBuff, drop: &LootDrop) {
    let gold = fortune_gold_reward(drop.gold, fortune);
    claim.stats.gold += gold;
    let candidate_stats = ItemStats::from(drop);
    let equipped_quality = claim.equipment.quality.clone();
    let equipped_stats = ItemStats {
        quality: &equipped_quality,
        damage_bonus: claim.damage_bonus.0,
        crit_chance: claim.equipment.crit_chance,
        health_bonus: claim.equipment.health_bonus,
        armor_bonus: claim.equipment.armor_bonus,
        legendary_power: claim.equipment.legendary_power,
        temper_level: claim.equipment.temper_level,
        socketed_gem: claim.equipment.socketed_gem,
    };
    let item = InventoryItem {
        name: drop.label.clone(),
        quality: drop.quality.clone(),
        damage_bonus: drop.damage_bonus,
        crit_chance: drop.crit_chance,
        health_bonus: drop.health_bonus,
        armor_bonus: drop.armor_bonus,
        legendary_power: drop.legendary_power,
        temper_level: drop.temper_level,
        socketed_gem: drop.socketed_gem,
    };
    match claim_codex_drop_power(
        claim.codex,
        claim.stats,
        item.legendary_power,
        &item.quality,
    ) {
        CodexDropClaim::Unlocked => {
            claim.combat_events.write(CombatEvent {
                text: format!("Codex unlocked: {}", item.legendary_power.label()),
            });
        }
        CodexDropClaim::DuplicateEssence(essence) => {
            claim.combat_events.write(CombatEvent {
                text: format!(
                    "Codex duplicate: {} refined into {} essence",
                    item.legendary_power.label(),
                    essence
                ),
            });
        }
        CodexDropClaim::None => {}
    }
    let stored = claim.inventory.add(item.clone());
    let auto_equipped = item_power(candidate_stats) >= item_power(equipped_stats);
    if auto_equipped {
        equip_inventory_item(&item, claim.damage_bonus, claim.equipment, claim.health);
        if claim.equipment.legendary_power != LegendaryPower::None {
            claim.combat_events.write(CombatEvent {
                text: format!(
                    "Legendary power: {}",
                    claim.equipment.legendary_power.label()
                ),
            });
        }
    }
    claim.combat_events.write(CombatEvent {
        text: loot_compare_summary(
            &drop.label,
            candidate_stats,
            equipped_stats,
            stored,
            auto_equipped,
        ),
    });
    if let Some(gem) = drop.bonus_gem {
        let socketed = socket_or_upgrade_gem(
            gem,
            claim.damage_bonus,
            claim.equipment,
            claim.inventory,
            claim.health,
        );
        register_gem_adept_kind(claim.stats, socketed.kind);
        claim.combat_events.write(CombatEvent {
            text: format!("Socketed {}", socketed.label()),
        });
    }
    if let Some(drop_charm) = &drop.bonus_charm {
        let equipped = equip_charm_if_better(claim.charm, drop_charm);
        claim.combat_events.write(CombatEvent {
            text: if equipped {
                format!("Equipped {}", claim.charm.summary())
            } else {
                format!("Kept current charm over {}", drop_charm.summary())
            },
        });
    }
    claim.potions.current = (claim.potions.current + drop.potions).min(claim.potions.max);
    claim.elixirs.current = (claim.elixirs.current + drop.elixirs).min(claim.elixirs.max);
    claim.combat_events.write(CombatEvent {
        text: format!(
            "Picked up {}{} and {} gold{}{}",
            drop.label,
            gem_label(drop.bonus_gem),
            gold,
            if drop.elixirs > 0 { " + elixir" } else { "" },
            if stored { "" } else { " (inventory full)" }
        ),
    });
    info!("Picked up {} (+{} damage)", drop.label, drop.damage_bonus);
}

fn register_gem_adept_kind(stats: &mut RunStats, kind: GemKind) {
    match kind {
        GemKind::Ruby => stats.ruby_socketed = true,
        GemKind::Emerald => stats.emerald_socketed = true,
        GemKind::Amethyst => stats.amethyst_socketed = true,
        GemKind::Topaz => stats.topaz_socketed = true,
    }
}

fn pickup_loot(
    mut commands: Commands,
    mut stats: ResMut<RunStats>,
    mut combat_events: MessageWriter<CombatEvent>,
    mut player: LootPickupPlayerQuery,
    loot: Query<(Entity, &Transform, &LootDrop), With<LootEntity>>,
) {
    let Ok((
        player_transform,
        mut damage_bonus,
        mut charm,
        mut equipment,
        mut inventory,
        mut health,
        mut potions,
        mut elixirs,
        mut codex,
        fortune,
    )) = player.single_mut()
    else {
        return;
    };

    for (entity, transform, drop) in &loot {
        if player_transform.translation.distance(transform.translation) < LOOT_PICKUP_RADIUS {
            let mut claim = LootClaimState {
                stats: &mut stats,
                combat_events: &mut combat_events,
                damage_bonus: &mut damage_bonus,
                charm: &mut charm,
                equipment: &mut equipment,
                inventory: &mut inventory,
                health: &mut health,
                potions: &mut potions,
                elixirs: &mut elixirs,
                codex: &mut codex,
            };
            claim_loot_drop(&mut claim, fortune, drop);
            if let Ok(mut entity_commands) = commands.get_entity(entity) {
                entity_commands.try_despawn();
            }
        }
    }
}

fn pickup_health_globes(
    mut commands: Commands,
    mut combat_events: MessageWriter<CombatEvent>,
    mut stats: ResMut<RunStats>,
    mut player: Query<(&Transform, &mut Health, &mut GloryBuff), With<Player>>,
    globes: Query<(Entity, &Transform, &HealthGlobe)>,
) {
    let Ok((player_transform, mut health, mut glory)) = player.single_mut() else {
        return;
    };

    for (entity, transform, globe) in &globes {
        if player_transform.translation.distance(transform.translation) < 1.15 {
            let healed = health_globe_heal_amount(health.max, globe.heal_fraction);
            health.current = (health.current + healed).min(health.max);
            let stacks = grant_glory(&mut glory);
            stats.health_globes_collected = stats.health_globes_collected.saturating_add(1);
            combat_events.write(CombatEvent {
                text: format!("Health globe restored {healed:.0} health; Glory {stacks}x"),
            });
            if let Ok(mut entity_commands) = commands.get_entity(entity) {
                entity_commands.try_despawn();
            }
        }
    }
}

fn pickup_fury_globes(
    mut commands: Commands,
    mut combat_events: MessageWriter<CombatEvent>,
    mut player: Query<(&Transform, &mut Fury), With<Player>>,
    globes: Query<(Entity, &Transform, &FuryGlobe)>,
) {
    let Ok((player_transform, mut fury)) = player.single_mut() else {
        return;
    };

    for (entity, transform, globe) in &globes {
        if player_transform.translation.distance(transform.translation) < 1.15 {
            let max_fury = fury.max;
            let gained = fury.gain(fury_globe_amount(max_fury, globe.fury_fraction));
            combat_events.write(CombatEvent {
                text: format!("Fury globe restored {gained:.0} fury"),
            });
            if let Ok(mut entity_commands) = commands.get_entity(entity) {
                entity_commands.try_despawn();
            }
        }
    }
}

fn quick_salvage_spare_inventory(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut stats: ResMut<RunStats>,
    mut combat_events: MessageWriter<CombatEvent>,
    mut player: Query<(&mut Inventory, &Equipment), With<Player>>,
) {
    if !keyboard.just_pressed(QUICK_SALVAGE_KEY) {
        return;
    }
    let Ok((mut inventory, equipment)) = player.single_mut() else {
        return;
    };
    let salvage = salvage_spare_inventory_into_stats(&mut inventory, equipment, &mut stats);
    combat_events.write(CombatEvent {
        text: salvage.summary(),
    });
}

type LootRuntimeCleanupQuery<'w, 's> = Query<
    'w,
    's,
    Entity,
    (
        Without<ChildOf>,
        Or<(
            With<LootEntity>,
            With<HealthGlobe>,
            With<FuryGlobe>,
            With<DeathBurstVfx>,
            With<DeathRemains>,
        )>,
    ),
>;

fn despawn_loot_runtime_entities(mut commands: Commands, query: LootRuntimeCleanupQuery) {
    for entity in &query {
        if let Ok(mut entity_commands) = commands.get_entity(entity) {
            entity_commands.try_despawn();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::companion::SENTINEL_STANCE_KEY;
    use crate::data::LootEntry;
    use rand::SeedableRng;

    #[test]
    fn loot_exit_cleanup_handles_entities_with_multiple_runtime_markers() {
        let mut app = App::new();
        let entity = app
            .world_mut()
            .spawn((
                LootEntity,
                HealthGlobe {
                    heal_fraction: 0.35,
                },
                FuryGlobe {
                    fury_fraction: 0.38,
                },
                DeathBurstVfx {
                    timer: Timer::from_seconds(0.1, TimerMode::Once),
                    duration_secs: 0.1,
                    base_scale: Vec3::ONE,
                    base_intensity: 1.0,
                },
                DeathRemains {
                    timer: Timer::from_seconds(0.1, TimerMode::Once),
                    duration_secs: 0.1,
                    base_scale: Vec3::ONE,
                    base_alpha: 1.0,
                },
            ))
            .id();

        app.add_systems(Update, despawn_loot_runtime_entities);
        app.update();

        assert!(app.world().get_entity(entity).is_err());
    }

    fn merchant_test_loot_table() -> LootTable {
        LootTable {
            weapons: vec![
                LootEntry {
                    name: "Iron Fang".to_string(),
                    quality: "common".to_string(),
                    weight: 100,
                    damage_bonus: 4.0,
                    crit_chance: 0.04,
                    health_bonus: 0.0,
                    armor_bonus: 2.0,
                    legendary_power: LegendaryPower::None,
                    potions: 0,
                },
                LootEntry {
                    name: "Stormglass Reaver".to_string(),
                    quality: "legendary".to_string(),
                    weight: 1,
                    damage_bonus: 16.0,
                    crit_chance: 0.18,
                    health_bonus: 10.0,
                    armor_bonus: 8.0,
                    legendary_power: LegendaryPower::Stormbrand,
                    potions: 1,
                },
            ],
        }
    }

    #[test]
    fn quick_salvage_key_does_not_conflict_with_sentinel_stance() {
        assert_eq!(QUICK_SALVAGE_KEY, KeyCode::KeyU);
        assert_eq!(SENTINEL_STANCE_KEY, KeyCode::KeyN);
        assert_ne!(QUICK_SALVAGE_KEY, SENTINEL_STANCE_KEY);
    }

    #[test]
    fn cache_rewards_skip_common_items_when_upgrades_exist() {
        let table = LootTable {
            weapons: vec![
                LootEntry {
                    name: "Iron Fang".to_string(),
                    quality: "common".to_string(),
                    weight: 100,
                    damage_bonus: 4.0,
                    crit_chance: 0.04,
                    health_bonus: 0.0,
                    armor_bonus: 2.0,
                    legendary_power: LegendaryPower::None,
                    potions: 0,
                },
                LootEntry {
                    name: "Moonforged Cleaver".to_string(),
                    quality: "rare".to_string(),
                    weight: 1,
                    damage_bonus: 9.0,
                    crit_chance: 0.08,
                    health_bonus: 10.0,
                    armor_bonus: 8.0,
                    legendary_power: LegendaryPower::None,
                    potions: 1,
                },
            ],
        };
        let mut rng = rand::rngs::StdRng::seed_from_u64(7);

        let reward = roll_cache_weapon(&table, &mut rng);

        assert_eq!(reward.quality, "rare");
    }

    #[test]
    fn malrec_loot_is_claimed_directly_before_victory_cleanup() {
        assert_eq!(enemy_loot_claim_mode("keeper"), LootClaimMode::Direct);
        assert_eq!(enemy_loot_claim_mode("skeleton"), LootClaimMode::Ground);
        assert_eq!(enemy_loot_claim_mode("treasure_imp"), LootClaimMode::Ground);
    }

    #[test]
    fn death_bursts_escalate_for_elites_and_bosses() {
        assert_eq!(death_burst_kind("skeleton", false), DeathBurstKind::Spark);
        assert_eq!(death_burst_kind("skeleton", true), DeathBurstKind::Burst);
        assert_eq!(death_burst_kind("keeper", false), DeathBurstKind::Cataclysm);
        assert_eq!(
            death_burst_kind("treasure_imp", false),
            DeathBurstKind::Prism
        );
        assert_eq!(death_burst_kind("nemesis", false), DeathBurstKind::Prism);
    }

    #[test]
    fn death_remains_mark_enemy_archetypes() {
        assert_eq!(death_remains_kind("cultist", false), DeathRemainsKind::Ash);
        assert_eq!(
            death_remains_kind("skeleton", false),
            DeathRemainsKind::Bone
        );
        assert_eq!(
            death_remains_kind("bone_stalker", false),
            DeathRemainsKind::Bone
        );
        assert_eq!(
            death_remains_kind("cultist", true),
            DeathRemainsKind::EliteBrand
        );
        assert_eq!(
            death_remains_kind("reliquary_brute", false),
            DeathRemainsKind::EliteBrand
        );
        assert_eq!(
            death_remains_kind("nemesis", false),
            DeathRemainsKind::EliteBrand
        );
        assert_eq!(
            death_remains_kind("treasure_imp", false),
            DeathRemainsKind::TreasureAsh
        );
        assert_eq!(
            death_remains_kind("keeper", false),
            DeathRemainsKind::BossSeal
        );
    }

    #[test]
    fn kill_floating_text_marks_elites_and_bosses() {
        assert_eq!(kill_floating_text("skeleton", false), "SLAY");
        assert_eq!(kill_floating_text("skeleton", true), "ELITE SLAY");
        assert_eq!(kill_floating_text("keeper", false), "BOSS DOWN");
    }

    #[test]
    fn death_burst_styles_escalate_light_and_lifetime() {
        let spark = death_burst_style(DeathBurstKind::Spark);
        let elite = death_burst_style(DeathBurstKind::Burst);
        let prism = death_burst_style(DeathBurstKind::Prism);
        let cataclysm = death_burst_style(DeathBurstKind::Cataclysm);

        assert!(elite.scale > spark.scale);
        assert!(prism.light_intensity > elite.light_intensity);
        assert!(prism.light_range > elite.light_range);
        assert!(prism.lifetime_secs > elite.lifetime_secs);
        assert!(elite.lifetime_secs > spark.lifetime_secs);
        assert!(cataclysm.scale > prism.scale);
        assert!(cataclysm.light_intensity > prism.light_intensity);
        assert!(cataclysm.lifetime_secs > prism.lifetime_secs);
    }

    #[test]
    fn death_burst_animation_expands_while_light_fades() {
        assert!(death_burst_scale_factor(0.0) > 1.0);
        assert!(death_burst_scale_factor(0.5) > death_burst_scale_factor(1.0));
        assert_eq!(death_burst_light_factor(0.0), 1.0);
        assert!(death_burst_light_factor(0.5) < death_burst_light_factor(0.1));
        assert_eq!(death_burst_light_factor(1.0), 0.0);
    }

    #[test]
    fn death_remains_styles_make_elites_and_bosses_read_heavier() {
        let ash = death_remains_style(DeathRemainsKind::Ash);
        let bone = death_remains_style(DeathRemainsKind::Bone);
        let elite = death_remains_style(DeathRemainsKind::EliteBrand);
        let treasure = death_remains_style(DeathRemainsKind::TreasureAsh);
        let boss = death_remains_style(DeathRemainsKind::BossSeal);

        assert!(bone.radius >= ash.radius);
        assert!(elite.radius > bone.radius);
        assert!(elite.lifetime_secs > bone.lifetime_secs);
        assert!(treasure.radius > ash.radius);
        assert!(boss.radius > elite.radius);
        assert!(boss.lifetime_secs > elite.lifetime_secs);
        assert!(boss.alpha > ash.alpha);
    }

    #[test]
    fn death_remains_animation_expands_as_it_fades() {
        assert_eq!(death_remains_scale_factor(0.0), 1.0);
        assert!(death_remains_scale_factor(0.8) > death_remains_scale_factor(0.2));
        assert_eq!(death_remains_alpha_factor(0.0), 1.0);
        assert!(death_remains_alpha_factor(0.5) < death_remains_alpha_factor(0.1));
        assert_eq!(death_remains_alpha_factor(1.0), 0.0);
    }

    #[test]
    fn death_screen_shake_escalates_for_elites_and_bosses() {
        assert!(death_screen_shake_event("skeleton", false).is_none());
        let elite = death_screen_shake_event("skeleton", true).unwrap();
        let boss = death_screen_shake_event("keeper", false).unwrap();

        assert!(boss.intensity > elite.intensity);
        assert!(boss.duration_secs > elite.duration_secs);
    }

    #[test]
    fn champion_cache_rewards_skip_common_items() {
        let table = LootTable {
            weapons: vec![
                LootEntry {
                    name: "Iron Fang".to_string(),
                    quality: "common".to_string(),
                    weight: 100,
                    damage_bonus: 4.0,
                    crit_chance: 0.04,
                    health_bonus: 0.0,
                    armor_bonus: 2.0,
                    legendary_power: LegendaryPower::None,
                    potions: 0,
                },
                LootEntry {
                    name: "Moonforged Cleaver".to_string(),
                    quality: "rare".to_string(),
                    weight: 1,
                    damage_bonus: 9.0,
                    crit_chance: 0.08,
                    health_bonus: 10.0,
                    armor_bonus: 8.0,
                    legendary_power: LegendaryPower::None,
                    potions: 1,
                },
            ],
        };
        let mut rng = rand::rngs::StdRng::seed_from_u64(19);

        let reward = roll_champion_cache_weapon(&table, &mut rng);

        assert_eq!(reward.quality, "rare");
    }

    #[test]
    fn elite_enemy_rewards_skip_common_items() {
        let table = LootTable {
            weapons: vec![
                LootEntry {
                    name: "Iron Fang".to_string(),
                    quality: "common".to_string(),
                    weight: 100,
                    damage_bonus: 4.0,
                    crit_chance: 0.04,
                    health_bonus: 0.0,
                    armor_bonus: 2.0,
                    legendary_power: LegendaryPower::None,
                    potions: 0,
                },
                LootEntry {
                    name: "Moonforged Cleaver".to_string(),
                    quality: "rare".to_string(),
                    weight: 1,
                    damage_bonus: 9.0,
                    crit_chance: 0.08,
                    health_bonus: 10.0,
                    armor_bonus: 8.0,
                    legendary_power: LegendaryPower::None,
                    potions: 1,
                },
            ],
        };
        let mut rng = rand::rngs::StdRng::seed_from_u64(13);

        let reward = roll_enemy_weapon("skeleton", true, false, &table, &mut rng);

        assert_eq!(reward.quality, "rare");
    }

    #[test]
    fn fortune_magic_find_uses_upgraded_pool_for_normal_enemies() {
        let table = LootTable {
            weapons: vec![
                LootEntry {
                    name: "Iron Fang".to_string(),
                    quality: "common".to_string(),
                    weight: 100,
                    damage_bonus: 4.0,
                    crit_chance: 0.04,
                    health_bonus: 0.0,
                    armor_bonus: 2.0,
                    legendary_power: LegendaryPower::None,
                    potions: 0,
                },
                LootEntry {
                    name: "Moonforged Cleaver".to_string(),
                    quality: "rare".to_string(),
                    weight: 1,
                    damage_bonus: 9.0,
                    crit_chance: 0.08,
                    health_bonus: 10.0,
                    armor_bonus: 8.0,
                    legendary_power: LegendaryPower::None,
                    potions: 1,
                },
            ],
        };
        let mut rng = rand::rngs::StdRng::seed_from_u64(13);

        let reward = roll_enemy_weapon("skeleton", false, true, &table, &mut rng);

        assert_eq!(reward.quality, "rare");
    }

    #[test]
    fn boss_rewards_prefer_legendary_or_ancient_items() {
        let table = LootTable {
            weapons: vec![
                LootEntry {
                    name: "Moonforged Cleaver".to_string(),
                    quality: "rare".to_string(),
                    weight: 100,
                    damage_bonus: 9.0,
                    crit_chance: 0.08,
                    health_bonus: 10.0,
                    armor_bonus: 8.0,
                    legendary_power: LegendaryPower::None,
                    potions: 1,
                },
                LootEntry {
                    name: "Ember-Saint Relic Blade".to_string(),
                    quality: "legendary".to_string(),
                    weight: 1,
                    damage_bonus: 16.0,
                    crit_chance: 0.14,
                    health_bonus: 24.0,
                    armor_bonus: 18.0,
                    legendary_power: LegendaryPower::Emberbrand,
                    potions: 2,
                },
            ],
        };
        let mut rng = rand::rngs::StdRng::seed_from_u64(21);

        let reward = roll_enemy_weapon("keeper", false, false, &table, &mut rng);

        assert_eq!(reward.quality, "legendary");
        assert_eq!(reward.legendary_power, LegendaryPower::Emberbrand);
    }

    #[test]
    fn boss_rewards_can_roll_ancient_items() {
        let table = LootTable {
            weapons: vec![
                LootEntry {
                    name: "Moonforged Cleaver".to_string(),
                    quality: "rare".to_string(),
                    weight: 100,
                    damage_bonus: 9.0,
                    crit_chance: 0.08,
                    health_bonus: 10.0,
                    armor_bonus: 8.0,
                    legendary_power: LegendaryPower::None,
                    potions: 1,
                },
                LootEntry {
                    name: "Ancient Ashen Reliquary Reaver".to_string(),
                    quality: "ancient".to_string(),
                    weight: 1,
                    damage_bonus: 22.0,
                    crit_chance: 0.20,
                    health_bonus: 34.0,
                    armor_bonus: 28.0,
                    legendary_power: LegendaryPower::Stormbrand,
                    potions: 3,
                },
            ],
        };
        let mut rng = rand::rngs::StdRng::seed_from_u64(21);

        let reward = roll_enemy_weapon("keeper", false, false, &table, &mut rng);

        assert_eq!(reward.quality, "ancient");
        assert_eq!(reward.legendary_power, LegendaryPower::Stormbrand);
    }

    #[test]
    fn boss_rewards_can_roll_ancient_aegisbrand_items() {
        let table = LootTable {
            weapons: vec![
                LootEntry {
                    name: "Moonforged Cleaver".to_string(),
                    quality: "rare".to_string(),
                    weight: 100,
                    damage_bonus: 9.0,
                    crit_chance: 0.08,
                    health_bonus: 10.0,
                    armor_bonus: 8.0,
                    legendary_power: LegendaryPower::None,
                    potions: 1,
                },
                LootEntry {
                    name: "Ancient Aegis-Saint Reliquary Bulwark".to_string(),
                    quality: "ancient".to_string(),
                    weight: 1,
                    damage_bonus: 18.0,
                    crit_chance: 0.22,
                    health_bonus: 48.0,
                    armor_bonus: 38.0,
                    legendary_power: LegendaryPower::Aegisbrand,
                    potions: 3,
                },
            ],
        };
        let mut rng = rand::rngs::StdRng::seed_from_u64(21);

        let reward = roll_enemy_weapon("keeper", false, false, &table, &mut rng);

        assert_eq!(reward.quality, "ancient");
        assert_eq!(reward.legendary_power, LegendaryPower::Aegisbrand);
    }

    #[test]
    fn boss_rewards_fallback_to_upgraded_items_without_legendary() {
        let table = LootTable {
            weapons: vec![
                LootEntry {
                    name: "Iron Fang".to_string(),
                    quality: "common".to_string(),
                    weight: 100,
                    damage_bonus: 4.0,
                    crit_chance: 0.04,
                    health_bonus: 0.0,
                    armor_bonus: 2.0,
                    legendary_power: LegendaryPower::None,
                    potions: 0,
                },
                LootEntry {
                    name: "Moonforged Cleaver".to_string(),
                    quality: "rare".to_string(),
                    weight: 1,
                    damage_bonus: 9.0,
                    crit_chance: 0.08,
                    health_bonus: 10.0,
                    armor_bonus: 8.0,
                    legendary_power: LegendaryPower::None,
                    potions: 1,
                },
            ],
        };
        let mut rng = rand::rngs::StdRng::seed_from_u64(34);

        let reward = roll_enemy_weapon("keeper", false, false, &table, &mut rng);

        assert_eq!(reward.quality, "rare");
    }

    #[test]
    fn merchant_sells_spares_and_restocks_potions() {
        let mut inventory = Inventory {
            items: vec![
                InventoryItem {
                    name: "rare Keeper Blade".to_string(),
                    quality: "rare".to_string(),
                    damage_bonus: 12.0,
                    crit_chance: 0.08,
                    health_bonus: 8.0,
                    armor_bonus: 9.0,
                    legendary_power: LegendaryPower::None,
                    temper_level: 0,
                    socketed_gem: None,
                },
                InventoryItem {
                    name: "common Rusted Fang".to_string(),
                    quality: "common".to_string(),
                    damage_bonus: 3.0,
                    crit_chance: 0.02,
                    health_bonus: 0.0,
                    armor_bonus: 1.0,
                    legendary_power: LegendaryPower::None,
                    temper_level: 0,
                    socketed_gem: None,
                },
            ],
            capacity: 12,
        };
        let mut equipment = Equipment {
            weapon_name: "rare Keeper Blade".to_string(),
            quality: "rare".to_string(),
            crit_chance: 0.08,
            health_bonus: 8.0,
            armor_bonus: 9.0,
            legendary_power: LegendaryPower::None,
            temper_level: 0,
            socketed_gem: None,
        };
        let mut potions = PotionBelt {
            current: 1,
            max: 3,
            heal_amount: 45.0,
            cooldown: Timer::from_seconds(0.0, TimerMode::Once),
            cooldown_secs: potion_cooldown_secs_for_capacity(3),
        };
        let mut elixirs = ElixirBelt {
            current: 0,
            max: 2,
            armor_bonus: 35.0,
            duration_secs: 22.0,
            selected: default(),
        };
        let mut damage_bonus = DamageBonus(12.0);
        let mut health = Health {
            current: 100.0,
            max: 100.0,
        };
        let mut stats = RunStats {
            gold: 320,
            ..RunStats::default()
        };
        let mut codex = LegendaryCodex::default();
        let mut charm = Charm::default();
        let loot_table = merchant_test_loot_table();

        let transaction = trade_with_merchant(MerchantTrade {
            loot_table: &loot_table,
            inventory: &mut inventory,
            equipment: &mut equipment,
            damage_bonus: &mut damage_bonus,
            charm: &mut charm,
            health: &mut health,
            potions: &mut potions,
            elixirs: &mut elixirs,
            codex: &mut codex,
            stats: &mut stats,
        });

        assert_eq!(transaction.sold_items, 1);
        assert_eq!(transaction.salvaged_shards, 1);
        assert_eq!(transaction.salvaged_essence, 0);
        assert_eq!(inventory.items.len(), 1);
        assert_eq!(inventory.items[0].name, "rare Keeper Blade");
        assert_eq!(potions.current, 3);
        assert!(elixirs.current > 0);
        assert!(transaction.tempered);
        assert_eq!(equipment.temper_level, 1);
        assert_eq!(damage_bonus.0, 14.0);
        assert!(stats.gold < 320 + transaction.sale_gold);
        assert_eq!(stats.ember_shards, 1);
        assert_eq!(stats.affix_essence, 0);
    }

    #[test]
    fn merchant_reforges_equipped_weapon_with_salvaged_shards() {
        let mut inventory = Inventory {
            items: vec![
                InventoryItem {
                    name: "rare Keeper Blade".to_string(),
                    quality: "rare".to_string(),
                    damage_bonus: 12.0,
                    crit_chance: 0.08,
                    health_bonus: 8.0,
                    armor_bonus: 9.0,
                    legendary_power: LegendaryPower::None,
                    temper_level: 0,
                    socketed_gem: None,
                },
                InventoryItem {
                    name: "legendary Spare Relic".to_string(),
                    quality: "legendary".to_string(),
                    damage_bonus: 14.0,
                    crit_chance: 0.10,
                    health_bonus: 16.0,
                    armor_bonus: 12.0,
                    legendary_power: LegendaryPower::Emberbrand,
                    temper_level: 0,
                    socketed_gem: None,
                },
            ],
            capacity: 12,
        };
        let mut equipment = Equipment {
            weapon_name: "rare Keeper Blade".to_string(),
            quality: "rare".to_string(),
            crit_chance: 0.08,
            health_bonus: 8.0,
            armor_bonus: 9.0,
            legendary_power: LegendaryPower::None,
            temper_level: 0,
            socketed_gem: None,
        };
        let mut damage_bonus = DamageBonus(12.0);
        let mut health = Health {
            current: 100.0,
            max: 100.0,
        };
        let mut potions = PotionBelt {
            current: 3,
            max: 3,
            heal_amount: 45.0,
            cooldown: Timer::from_seconds(0.0, TimerMode::Once),
            cooldown_secs: potion_cooldown_secs_for_capacity(3),
        };
        let mut elixirs = ElixirBelt {
            current: 2,
            max: 2,
            armor_bonus: 35.0,
            duration_secs: 22.0,
            selected: default(),
        };
        let mut stats = RunStats::default();
        let mut codex = LegendaryCodex::default();
        let mut charm = Charm::default();
        let loot_table = merchant_test_loot_table();

        let transaction = trade_with_merchant(MerchantTrade {
            loot_table: &loot_table,
            inventory: &mut inventory,
            equipment: &mut equipment,
            damage_bonus: &mut damage_bonus,
            charm: &mut charm,
            health: &mut health,
            potions: &mut potions,
            elixirs: &mut elixirs,
            codex: &mut codex,
            stats: &mut stats,
        });

        assert_eq!(transaction.salvaged_shards, 8);
        assert_eq!(transaction.salvaged_essence, 1);
        assert!(transaction.summary().contains("1 affix essence"));
        assert!(transaction.tempered);
        assert!(transaction.reforged);
        assert_eq!(stats.ember_shards, 2);
        assert_eq!(stats.affix_essence, 1);
        assert_eq!(damage_bonus.0, 15.0);
        assert_eq!(equipment.armor_bonus, 12.0);
        assert!((equipment.crit_chance - 0.095).abs() < 0.001);
        assert_eq!(inventory.items[0].damage_bonus, 15.0);
    }

    #[test]
    fn merchant_salvages_ancient_spares_for_extra_affix_essence() {
        let mut inventory = Inventory {
            items: vec![
                InventoryItem {
                    name: "rare Keeper Blade".to_string(),
                    quality: "rare".to_string(),
                    damage_bonus: 12.0,
                    crit_chance: 0.08,
                    health_bonus: 8.0,
                    armor_bonus: 9.0,
                    legendary_power: LegendaryPower::None,
                    temper_level: 0,
                    socketed_gem: None,
                },
                InventoryItem {
                    name: "Ancient Aegis-Saint Reliquary Bulwark".to_string(),
                    quality: "ancient".to_string(),
                    damage_bonus: 18.0,
                    crit_chance: 0.22,
                    health_bonus: 48.0,
                    armor_bonus: 38.0,
                    legendary_power: LegendaryPower::Aegisbrand,
                    temper_level: 0,
                    socketed_gem: None,
                },
            ],
            capacity: 12,
        };
        let equipment = Equipment {
            weapon_name: "rare Keeper Blade".to_string(),
            quality: "rare".to_string(),
            crit_chance: 0.08,
            health_bonus: 8.0,
            armor_bonus: 9.0,
            legendary_power: LegendaryPower::None,
            temper_level: 0,
            socketed_gem: None,
        };

        let salvage = salvage_spare_inventory(&mut inventory, &equipment);

        assert_eq!(salvage.items, 1);
        assert!(salvage.gold > 0);
        assert_eq!(salvage.shards, 14);
        assert_eq!(salvage.essence, 3);
        assert_eq!(inventory.items.len(), 1);
        assert_eq!(inventory.items[0].name, "rare Keeper Blade");
    }

    #[test]
    fn ashen_alchemy_can_chain_all_three_transmutations() {
        let mut stats = RunStats {
            gold: ALCHEMY_GOLD_TO_SHARDS_COST,
            ember_shards: ALCHEMY_SHARDS_TO_ESSENCE_COST + ALCHEMY_KEYSTONE_SHARD_COST
                - ALCHEMY_GOLD_TO_SHARDS_REWARD,
            affix_essence: ALCHEMY_KEYSTONE_ESSENCE_COST - ALCHEMY_SHARDS_TO_ESSENCE_REWARD,
            ..RunStats::default()
        };

        let batch = apply_ashen_alchemy(&mut stats);

        assert!(batch.gold_to_shards);
        assert!(batch.shards_to_essence);
        assert!(batch.essence_to_keystone);
        assert_eq!(stats.gold, 0);
        assert_eq!(stats.ember_shards, 0);
        assert_eq!(stats.affix_essence, 0);
        assert_eq!(stats.echo_keystones, ALCHEMY_KEYSTONE_REWARD);
        assert_eq!(stats.alchemy_gold_transmutes, 1);
        assert_eq!(stats.alchemy_essence_transmutes, 1);
        assert_eq!(stats.alchemy_keystone_transmutes, 1);
    }

    #[test]
    fn ashen_alchemy_does_not_create_keystone_without_full_cost() {
        let mut stats = RunStats {
            ember_shards: ALCHEMY_KEYSTONE_SHARD_COST,
            affix_essence: ALCHEMY_KEYSTONE_ESSENCE_COST - 1,
            ..RunStats::default()
        };

        let batch = apply_ashen_alchemy(&mut stats);

        assert!(batch.shards_to_essence);
        assert!(!batch.essence_to_keystone);
        assert_eq!(
            stats.ember_shards,
            ALCHEMY_KEYSTONE_SHARD_COST - ALCHEMY_SHARDS_TO_ESSENCE_COST
        );
        assert_eq!(
            stats.affix_essence,
            ALCHEMY_KEYSTONE_ESSENCE_COST - 1 + ALCHEMY_SHARDS_TO_ESSENCE_REWARD
        );
        assert_eq!(stats.echo_keystones, 0);
    }

    #[test]
    fn soul_sigil_cache_consumes_boss_trophies_for_endgame_materials() {
        let mut stats = RunStats {
            malrec_soul_sigils: SOUL_SIGIL_CACHE_COST,
            ..RunStats::default()
        };

        let reward = open_soul_sigil_cache(&mut stats);

        assert!(reward.opened);
        assert_eq!(stats.malrec_soul_sigils, 0);
        assert_eq!(stats.gold, SOUL_SIGIL_CACHE_GOLD_REWARD);
        assert_eq!(stats.ember_shards, SOUL_SIGIL_CACHE_SHARD_REWARD);
        assert_eq!(stats.affix_essence, SOUL_SIGIL_CACHE_ESSENCE_REWARD);
        assert_eq!(stats.echo_keystones, SOUL_SIGIL_CACHE_KEYSTONE_REWARD);
        assert_eq!(stats.soul_sigil_caches, 1);
    }

    #[test]
    fn soul_sigil_cache_requires_two_boss_trophies() {
        let mut stats = RunStats {
            malrec_soul_sigils: SOUL_SIGIL_CACHE_COST - 1,
            ..RunStats::default()
        };

        let reward = open_soul_sigil_cache(&mut stats);

        assert!(!reward.opened);
        assert_eq!(stats.malrec_soul_sigils, SOUL_SIGIL_CACHE_COST - 1);
        assert_eq!(stats.soul_sigil_caches, 0);
        assert_eq!(stats.echo_keystones, 0);
    }

    #[test]
    fn soul_sigil_cache_ready_summary_scales_multiple_boss_trophies() {
        assert_eq!(soul_sigil_cache_ready_count(1), 0);
        assert_eq!(
            soul_sigil_cache_ready_summary(1),
            None,
            "a single sigil should read as progress, not a ready cache"
        );
        assert_eq!(
            soul_sigil_cache_ready_summary(SOUL_SIGIL_CACHE_COST),
            Some("1 Trophy Cache (2 Soul Sigils, +120g +4s +3e +1 key)".to_string())
        );
        assert_eq!(
            soul_sigil_cache_ready_summary(SOUL_SIGIL_CACHE_COST * 2),
            Some("2 Trophy Caches (4 Soul Sigils, +240g +8s +6e +2 keys)".to_string())
        );
    }

    #[test]
    fn salvage_cache_rewards_accumulate_across_merchant_visits() {
        let mut stats = RunStats {
            salvage_progress: 3,
            ..RunStats::default()
        };

        let reward = apply_salvage_cache_rewards(&mut stats, 1);

        assert_eq!(
            reward,
            SalvageCacheReward {
                caches: 1,
                gold: SALVAGE_CACHE_GOLD_REWARD,
                shards: SALVAGE_CACHE_SHARD_REWARD,
                essence: SALVAGE_CACHE_ESSENCE_REWARD,
            }
        );
        assert_eq!(stats.salvage_progress, 0);
        assert_eq!(stats.salvage_caches, 1);
        assert_eq!(stats.gold, SALVAGE_CACHE_GOLD_REWARD);
        assert_eq!(stats.ember_shards, SALVAGE_CACHE_SHARD_REWARD);
        assert_eq!(stats.affix_essence, SALVAGE_CACHE_ESSENCE_REWARD);

        let reward = apply_salvage_cache_rewards(&mut stats, 9);

        assert_eq!(reward.caches, 2);
        assert_eq!(stats.salvage_progress, 1);
        assert_eq!(stats.salvage_caches, 3);
    }

    #[test]
    fn merchant_enchants_equipped_weapon_with_legendary_power() {
        let mut inventory = Inventory {
            items: vec![InventoryItem {
                name: "rare Keeper Blade".to_string(),
                quality: "rare".to_string(),
                damage_bonus: 12.0,
                crit_chance: 0.08,
                health_bonus: 8.0,
                armor_bonus: 9.0,
                legendary_power: LegendaryPower::None,
                temper_level: 2,
                socketed_gem: None,
            }],
            capacity: MAX_INVENTORY_CAPACITY,
        };
        let mut equipment = Equipment {
            weapon_name: "rare Keeper Blade".to_string(),
            quality: "rare".to_string(),
            crit_chance: 0.08,
            health_bonus: 8.0,
            armor_bonus: 9.0,
            legendary_power: LegendaryPower::None,
            temper_level: 2,
            socketed_gem: None,
        };
        let mut stats = RunStats {
            gold: MERCHANT_ENCHANT_GOLD_COST,
            ember_shards: MERCHANT_ENCHANT_SHARD_COST,
            affix_essence: MERCHANT_ENCHANT_ESSENCE_COST,
            ..RunStats::default()
        };
        let mut codex = LegendaryCodex::default();
        codex.unlock(LegendaryPower::Emberbrand);

        assert!(enchant_equipped_weapon(
            &mut inventory,
            &mut equipment,
            &codex,
            &mut stats
        ));

        assert_eq!(equipment.legendary_power, LegendaryPower::Emberbrand);
        assert_eq!(equipment.quality, "legendary");
        assert_eq!(
            inventory.items[0].legendary_power,
            LegendaryPower::Emberbrand
        );
        assert_eq!(inventory.items[0].quality, "legendary");
        assert!(
            MerchantTransaction {
                enchanted: true,
                enchant_gold_cost: MERCHANT_ENCHANT_GOLD_COST,
                enchant_shard_cost: MERCHANT_ENCHANT_SHARD_COST,
                enchant_essence_cost: MERCHANT_ENCHANT_ESSENCE_COST,
                enchanted_power: equipment.legendary_power,
                ..MerchantTransaction::default()
            }
            .summary()
            .contains("2 essence")
        );
        assert_eq!(stats.gold, 0);
        assert_eq!(stats.ember_shards, 0);
        assert_eq!(stats.affix_essence, 0);
    }

    #[test]
    fn merchant_summary_groups_trade_supplies_gear_and_cache_feedback() {
        let summary = MerchantTransaction {
            sold_items: 2,
            sale_gold: 96,
            salvaged_shards: 11,
            salvaged_essence: 1,
            bought_potions: 2,
            bought_elixirs: 1,
            tempered: true,
            temper_cost: 55,
            enchanted: true,
            enchant_gold_cost: MERCHANT_ENCHANT_GOLD_COST,
            enchant_shard_cost: MERCHANT_ENCHANT_SHARD_COST,
            enchant_essence_cost: MERCHANT_ENCHANT_ESSENCE_COST,
            enchanted_power: LegendaryPower::Stormbrand,
            salvage_cache_count: 1,
            salvage_cache_gold: SALVAGE_CACHE_GOLD_REWARD,
            salvage_cache_shards: SALVAGE_CACHE_SHARD_REWARD,
            salvage_cache_essence: SALVAGE_CACHE_ESSENCE_REWARD,
            ..MerchantTransaction::default()
        }
        .summary();

        assert!(summary.starts_with("Quartermaster: Trade sold 2 gear"));
        assert!(summary.contains(" | Supplies +2 potion +1 elixir"));
        assert!(summary.contains(" | Gear tempered 55g, enchanted Stormbrand"));
        assert!(summary.contains("2 essence"));
        assert!(summary.contains(" | Cache salvage cache x1"));
        assert!(!summary.contains("; bought"));
        assert!(summary.len() < 240);
    }

    #[test]
    fn merchant_enchant_preserves_ancient_quality() {
        let mut inventory = Inventory {
            items: vec![InventoryItem {
                name: "Ancient Ashen Reliquary Reaver".to_string(),
                quality: "ancient".to_string(),
                damage_bonus: 22.0,
                crit_chance: 0.20,
                health_bonus: 34.0,
                armor_bonus: 28.0,
                legendary_power: LegendaryPower::Stormbrand,
                temper_level: 1,
                socketed_gem: None,
            }],
            capacity: MAX_INVENTORY_CAPACITY,
        };
        let mut equipment = Equipment {
            weapon_name: "Ancient Ashen Reliquary Reaver".to_string(),
            quality: "ancient".to_string(),
            crit_chance: 0.20,
            health_bonus: 34.0,
            armor_bonus: 28.0,
            legendary_power: LegendaryPower::Stormbrand,
            temper_level: 1,
            socketed_gem: None,
        };
        let mut stats = RunStats {
            gold: MERCHANT_ENCHANT_GOLD_COST,
            ember_shards: MERCHANT_ENCHANT_SHARD_COST,
            affix_essence: MERCHANT_ENCHANT_ESSENCE_COST,
            ..RunStats::default()
        };
        let mut codex = LegendaryCodex::default();
        codex.unlock(LegendaryPower::Stormbrand);
        codex.unlock(LegendaryPower::Aegisbrand);

        assert!(enchant_equipped_weapon(
            &mut inventory,
            &mut equipment,
            &codex,
            &mut stats
        ));

        assert_eq!(equipment.legendary_power, LegendaryPower::Aegisbrand);
        assert_eq!(equipment.quality, "ancient");
        assert_eq!(inventory.items[0].quality, "ancient");
        assert_eq!(
            inventory.items[0].legendary_power,
            LegendaryPower::Aegisbrand
        );
    }

    #[test]
    fn merchant_enchant_requires_unlocked_codex_power_and_essence() {
        let mut inventory = Inventory {
            items: vec![InventoryItem {
                name: "rare Keeper Blade".to_string(),
                quality: "rare".to_string(),
                damage_bonus: 12.0,
                crit_chance: 0.08,
                health_bonus: 8.0,
                armor_bonus: 9.0,
                legendary_power: LegendaryPower::None,
                temper_level: 2,
                socketed_gem: None,
            }],
            capacity: MAX_INVENTORY_CAPACITY,
        };
        let mut equipment = Equipment {
            weapon_name: "rare Keeper Blade".to_string(),
            quality: "rare".to_string(),
            crit_chance: 0.08,
            health_bonus: 8.0,
            armor_bonus: 9.0,
            legendary_power: LegendaryPower::None,
            temper_level: 2,
            socketed_gem: None,
        };
        let mut stats = RunStats {
            gold: MERCHANT_ENCHANT_GOLD_COST,
            ember_shards: MERCHANT_ENCHANT_SHARD_COST,
            affix_essence: MERCHANT_ENCHANT_ESSENCE_COST,
            ..RunStats::default()
        };
        let codex = LegendaryCodex::default();

        assert!(!enchant_equipped_weapon(
            &mut inventory,
            &mut equipment,
            &codex,
            &mut stats
        ));

        assert_eq!(equipment.legendary_power, LegendaryPower::None);
        assert_eq!(stats.gold, MERCHANT_ENCHANT_GOLD_COST);
        assert_eq!(stats.ember_shards, MERCHANT_ENCHANT_SHARD_COST);
        assert_eq!(stats.affix_essence, MERCHANT_ENCHANT_ESSENCE_COST);

        let mut codex = LegendaryCodex::default();
        codex.unlock(LegendaryPower::Emberbrand);
        stats.affix_essence = MERCHANT_ENCHANT_ESSENCE_COST - 1;

        assert!(!enchant_equipped_weapon(
            &mut inventory,
            &mut equipment,
            &codex,
            &mut stats
        ));

        assert_eq!(equipment.legendary_power, LegendaryPower::None);
        assert_eq!(stats.gold, MERCHANT_ENCHANT_GOLD_COST);
        assert_eq!(stats.ember_shards, MERCHANT_ENCHANT_SHARD_COST);
        assert_eq!(stats.affix_essence, MERCHANT_ENCHANT_ESSENCE_COST - 1);
    }

    #[test]
    fn merchant_enchant_cycles_existing_legendary_power() {
        let mut inventory = Inventory {
            items: vec![InventoryItem {
                name: "legendary Keeper Blade".to_string(),
                quality: "legendary".to_string(),
                damage_bonus: 18.0,
                crit_chance: 0.12,
                health_bonus: 14.0,
                armor_bonus: 12.0,
                legendary_power: LegendaryPower::Emberbrand,
                temper_level: 3,
                socketed_gem: None,
            }],
            capacity: MAX_INVENTORY_CAPACITY,
        };
        let mut equipment = Equipment {
            weapon_name: "legendary Keeper Blade".to_string(),
            quality: "legendary".to_string(),
            crit_chance: 0.12,
            health_bonus: 14.0,
            armor_bonus: 12.0,
            legendary_power: LegendaryPower::Emberbrand,
            temper_level: 3,
            socketed_gem: None,
        };
        let mut stats = RunStats {
            gold: MERCHANT_ENCHANT_GOLD_COST * 2,
            ember_shards: MERCHANT_ENCHANT_SHARD_COST * 2,
            affix_essence: MERCHANT_ENCHANT_ESSENCE_COST * 2,
            ..RunStats::default()
        };
        let mut codex = LegendaryCodex::default();
        codex.unlock(LegendaryPower::Emberbrand);
        codex.unlock(LegendaryPower::Frostbrand);

        assert!(enchant_equipped_weapon(
            &mut inventory,
            &mut equipment,
            &codex,
            &mut stats
        ));
        assert_eq!(equipment.legendary_power, LegendaryPower::Frostbrand);

        assert!(enchant_equipped_weapon(
            &mut inventory,
            &mut equipment,
            &codex,
            &mut stats
        ));
        assert_eq!(equipment.legendary_power, LegendaryPower::Emberbrand);
        assert_eq!(
            inventory.items[0].legendary_power,
            LegendaryPower::Emberbrand
        );
        assert_eq!(stats.affix_essence, 0);
    }

    #[test]
    fn merchant_enchant_prefers_codex_attunement() {
        let mut inventory = Inventory {
            items: vec![InventoryItem {
                name: "legendary Keeper Blade".to_string(),
                quality: "legendary".to_string(),
                damage_bonus: 18.0,
                crit_chance: 0.12,
                health_bonus: 14.0,
                armor_bonus: 12.0,
                legendary_power: LegendaryPower::Emberbrand,
                temper_level: 3,
                socketed_gem: None,
            }],
            capacity: MAX_INVENTORY_CAPACITY,
        };
        let mut equipment = Equipment {
            weapon_name: "legendary Keeper Blade".to_string(),
            quality: "legendary".to_string(),
            crit_chance: 0.12,
            health_bonus: 14.0,
            armor_bonus: 12.0,
            legendary_power: LegendaryPower::Emberbrand,
            temper_level: 3,
            socketed_gem: None,
        };
        let mut stats = RunStats {
            gold: MERCHANT_ENCHANT_GOLD_COST,
            ember_shards: MERCHANT_ENCHANT_SHARD_COST,
            affix_essence: MERCHANT_ENCHANT_ESSENCE_COST,
            ..RunStats::default()
        };
        let mut codex = LegendaryCodex::default();
        codex.unlock(LegendaryPower::Emberbrand);
        codex.unlock(LegendaryPower::Frostbrand);
        codex.unlock(LegendaryPower::Soulreaver);
        codex.restore_attunement(LegendaryPower::Soulreaver);

        assert!(enchant_equipped_weapon(
            &mut inventory,
            &mut equipment,
            &codex,
            &mut stats
        ));

        assert_eq!(equipment.legendary_power, LegendaryPower::Soulreaver);
        assert_eq!(
            inventory.items[0].legendary_power,
            LegendaryPower::Soulreaver
        );
        assert_eq!(stats.gold, 0);
        assert_eq!(stats.ember_shards, 0);
        assert_eq!(stats.affix_essence, 0);
    }

    #[test]
    fn merchant_expands_inventory_capacity_with_gold_and_shards() {
        let mut inventory = Inventory {
            items: vec![InventoryItem {
                name: "rare Keeper Blade".to_string(),
                quality: "rare".to_string(),
                damage_bonus: 12.0,
                crit_chance: 0.08,
                health_bonus: 8.0,
                armor_bonus: 9.0,
                legendary_power: LegendaryPower::None,
                temper_level: 2,
                socketed_gem: None,
            }],
            capacity: 12,
        };
        let mut equipment = Equipment {
            weapon_name: "rare Keeper Blade".to_string(),
            quality: "rare".to_string(),
            crit_chance: 0.08,
            health_bonus: 8.0,
            armor_bonus: 9.0,
            legendary_power: LegendaryPower::None,
            temper_level: 2,
            socketed_gem: None,
        };
        let mut damage_bonus = DamageBonus(12.0);
        let mut health = Health {
            current: 100.0,
            max: 100.0,
        };
        let mut potions = PotionBelt {
            current: 3,
            max: 3,
            heal_amount: 45.0,
            cooldown: Timer::from_seconds(0.0, TimerMode::Once),
            cooldown_secs: potion_cooldown_secs_for_capacity(3),
        };
        let mut elixirs = ElixirBelt {
            current: 2,
            max: 2,
            armor_bonus: 35.0,
            duration_secs: 22.0,
            selected: default(),
        };
        let mut stats = RunStats {
            gold: MERCHANT_STASH_GOLD_COST,
            ember_shards: MERCHANT_STASH_SHARD_COST,
            ..RunStats::default()
        };
        let mut codex = LegendaryCodex::default();
        let mut charm = Charm::default();
        let loot_table = merchant_test_loot_table();

        let transaction = trade_with_merchant(MerchantTrade {
            loot_table: &loot_table,
            inventory: &mut inventory,
            equipment: &mut equipment,
            damage_bonus: &mut damage_bonus,
            charm: &mut charm,
            health: &mut health,
            potions: &mut potions,
            elixirs: &mut elixirs,
            codex: &mut codex,
            stats: &mut stats,
        });

        assert!(transaction.capacity_upgraded);
        assert_eq!(inventory.capacity, 14);
        assert_eq!(transaction.inventory_capacity, 14);
        assert_eq!(stats.gold, 0);
        assert_eq!(stats.ember_shards, 0);
        assert!(!transaction.tempered);
        assert!(!transaction.reforged);
    }

    #[test]
    fn merchant_upgrades_potion_belt_capacity_with_gold_and_shards() {
        let mut inventory = Inventory {
            items: vec![InventoryItem {
                name: "rare Keeper Blade".to_string(),
                quality: "rare".to_string(),
                damage_bonus: 12.0,
                crit_chance: 0.08,
                health_bonus: 8.0,
                armor_bonus: 9.0,
                legendary_power: LegendaryPower::None,
                temper_level: 2,
                socketed_gem: None,
            }],
            capacity: MAX_INVENTORY_CAPACITY,
        };
        let mut equipment = Equipment {
            weapon_name: "rare Keeper Blade".to_string(),
            quality: "rare".to_string(),
            crit_chance: 0.08,
            health_bonus: 8.0,
            armor_bonus: 9.0,
            legendary_power: LegendaryPower::None,
            temper_level: 2,
            socketed_gem: None,
        };
        let mut damage_bonus = DamageBonus(12.0);
        let mut health = Health {
            current: 100.0,
            max: 100.0,
        };
        let mut potions = PotionBelt {
            current: 5,
            max: 5,
            heal_amount: 45.0,
            cooldown: Timer::from_seconds(0.0, TimerMode::Once),
            cooldown_secs: potion_cooldown_secs_for_capacity(5),
        };
        let mut elixirs = ElixirBelt {
            current: MAX_ELIXIR_CAPACITY,
            max: MAX_ELIXIR_CAPACITY,
            armor_bonus: 35.0,
            duration_secs: 22.0,
            selected: default(),
        };
        let mut stats = RunStats {
            gold: MERCHANT_POTION_BELT_GOLD_COST,
            ember_shards: MERCHANT_POTION_BELT_SHARD_COST,
            ..RunStats::default()
        };
        let mut codex = LegendaryCodex::default();
        let mut charm = Charm::default();
        let old_cooldown_secs = potions.cooldown_secs;
        let loot_table = merchant_test_loot_table();

        let transaction = trade_with_merchant(MerchantTrade {
            loot_table: &loot_table,
            inventory: &mut inventory,
            equipment: &mut equipment,
            damage_bonus: &mut damage_bonus,
            charm: &mut charm,
            health: &mut health,
            potions: &mut potions,
            elixirs: &mut elixirs,
            codex: &mut codex,
            stats: &mut stats,
        });

        assert!(transaction.potion_belt_upgraded);
        assert_eq!(potions.max, 6);
        assert!(potions.cooldown_secs < old_cooldown_secs);
        assert_eq!(transaction.potion_capacity, 6);
        assert_eq!(stats.gold, 0);
        assert_eq!(stats.ember_shards, 0);
        assert!(!transaction.tempered);
        assert!(!transaction.reforged);
        assert!(!transaction.capacity_upgraded);
    }

    #[test]
    fn tempering_equipped_weapon_spends_gold_and_syncs_inventory() {
        let mut inventory = Inventory {
            items: vec![InventoryItem {
                name: "rare Keeper Blade".to_string(),
                quality: "rare".to_string(),
                damage_bonus: 12.0,
                crit_chance: 0.08,
                health_bonus: 8.0,
                armor_bonus: 9.0,
                legendary_power: LegendaryPower::None,
                temper_level: 0,
                socketed_gem: None,
            }],
            capacity: 12,
        };
        let mut equipment = Equipment {
            weapon_name: "rare Keeper Blade".to_string(),
            quality: "rare".to_string(),
            crit_chance: 0.08,
            health_bonus: 8.0,
            armor_bonus: 9.0,
            legendary_power: LegendaryPower::None,
            temper_level: 0,
            socketed_gem: None,
        };
        let mut damage_bonus = DamageBonus(12.0);
        let mut gold = 90;

        assert!(temper_equipped_weapon(
            &mut inventory,
            &mut equipment,
            &mut damage_bonus,
            &mut gold
        ));
        assert_eq!(gold, 0);
        assert_eq!(equipment.temper_level, 1);
        assert_eq!(inventory.items[0].temper_level, 1);
        assert_eq!(damage_bonus.0, 14.0);
        assert_eq!(equipment.armor_bonus, 11.0);
        assert!((equipment.crit_chance - 0.085).abs() < 0.001);
    }

    #[test]
    fn tempering_requires_enough_gold() {
        let mut inventory = Inventory {
            items: Vec::new(),
            capacity: 12,
        };
        let mut equipment = Equipment {
            weapon_name: "rare Keeper Blade".to_string(),
            quality: "rare".to_string(),
            crit_chance: 0.08,
            health_bonus: 8.0,
            armor_bonus: 9.0,
            legendary_power: LegendaryPower::None,
            temper_level: 1,
            socketed_gem: None,
        };
        let mut damage_bonus = DamageBonus(12.0);
        let mut gold = temper_cost(equipment.temper_level) - 1;

        assert!(!temper_equipped_weapon(
            &mut inventory,
            &mut equipment,
            &mut damage_bonus,
            &mut gold
        ));
        assert_eq!(equipment.temper_level, 1);
        assert_eq!(damage_bonus.0, 12.0);
    }

    #[test]
    fn merchant_upgrades_socketed_gem_when_affordable() {
        let mut inventory = Inventory {
            items: vec![InventoryItem {
                name: "rare Keeper Blade".to_string(),
                quality: "rare".to_string(),
                damage_bonus: 15.0,
                crit_chance: 0.08,
                health_bonus: 8.0,
                armor_bonus: 9.0,
                legendary_power: LegendaryPower::None,
                temper_level: 0,
                socketed_gem: Some(SocketedGem {
                    kind: GemKind::Ruby,
                    rank: 1,
                }),
            }],
            capacity: 12,
        };
        let mut equipment = Equipment {
            weapon_name: "rare Keeper Blade".to_string(),
            quality: "rare".to_string(),
            crit_chance: 0.08,
            health_bonus: 8.0,
            armor_bonus: 9.0,
            legendary_power: LegendaryPower::None,
            temper_level: 0,
            socketed_gem: Some(SocketedGem {
                kind: GemKind::Ruby,
                rank: 1,
            }),
        };
        let mut damage_bonus = DamageBonus(15.0);
        let mut health = Health {
            current: 108.0,
            max: 108.0,
        };
        let mut gold = gem_upgrade_cost(1);

        assert!(upgrade_socketed_gem(
            &mut inventory,
            &mut equipment,
            &mut damage_bonus,
            &mut health,
            &mut gold
        ));

        assert_eq!(gold, 0);
        assert_eq!(equipment.socketed_gem.unwrap().rank, 2);
        assert_eq!(inventory.items[0].socketed_gem.unwrap().rank, 2);
        assert_eq!(damage_bonus.0, 18.0);
    }

    #[test]
    fn merchant_empowers_ascendant_gem_with_echo_keystone_and_essence() {
        let ascendant_ruby = SocketedGem {
            kind: GemKind::Ruby,
            rank: GEM_EMPOWER_MIN_RANK,
        };
        let mut inventory = Inventory {
            items: vec![InventoryItem {
                name: "ancient Reliquary Fang".to_string(),
                quality: "ancient".to_string(),
                damage_bonus: 40.0,
                crit_chance: 0.08,
                health_bonus: 8.0,
                armor_bonus: 9.0,
                legendary_power: LegendaryPower::Emberbrand,
                temper_level: 0,
                socketed_gem: Some(ascendant_ruby),
            }],
            capacity: 12,
        };
        let mut equipment = Equipment {
            weapon_name: "ancient Reliquary Fang".to_string(),
            quality: "ancient".to_string(),
            crit_chance: 0.08,
            health_bonus: 8.0,
            armor_bonus: 9.0,
            legendary_power: LegendaryPower::Emberbrand,
            temper_level: 0,
            socketed_gem: Some(ascendant_ruby),
        };
        let mut damage_bonus = DamageBonus(40.0);
        let mut health = Health {
            current: 108.0,
            max: 108.0,
        };
        let mut stats = RunStats {
            gold: GEM_EMPOWER_GOLD_COST,
            affix_essence: GEM_EMPOWER_ESSENCE_COST,
            echo_keystones: GEM_EMPOWER_ECHO_KEYSTONE_COST,
            ..RunStats::default()
        };

        assert!(empower_socketed_gem(
            &mut inventory,
            &mut equipment,
            &mut damage_bonus,
            &mut health,
            &mut stats
        ));

        let empowered = equipment.socketed_gem.unwrap();
        assert_eq!(empowered.rank, GEM_EMPOWER_MIN_RANK + GEM_EMPOWER_RANK_GAIN);
        assert_eq!(empowered.resonance_label(), "Paragon");
        assert_eq!(inventory.items[0].socketed_gem, Some(empowered));
        assert_eq!(stats.gold, 0);
        assert_eq!(stats.affix_essence, 0);
        assert_eq!(stats.echo_keystones, 0);
        assert_eq!(damage_bonus.0, 54.0);
    }

    #[test]
    fn ancient_awakening_promotes_legendary_weapon_with_high_rank_gem() {
        let mut inventory = Inventory {
            items: vec![InventoryItem {
                name: "Stormcall Reliquary Brand".to_string(),
                quality: "legendary".to_string(),
                damage_bonus: 15.0,
                crit_chance: 0.15,
                health_bonus: 16.0,
                armor_bonus: 20.0,
                legendary_power: LegendaryPower::Stormbrand,
                temper_level: 2,
                socketed_gem: Some(SocketedGem {
                    kind: GemKind::Topaz,
                    rank: 4,
                }),
            }],
            capacity: 12,
        };
        let mut equipment = Equipment {
            weapon_name: "Stormcall Reliquary Brand".to_string(),
            quality: "legendary".to_string(),
            crit_chance: 0.15,
            health_bonus: 16.0,
            armor_bonus: 20.0,
            legendary_power: LegendaryPower::Stormbrand,
            temper_level: 2,
            socketed_gem: Some(SocketedGem {
                kind: GemKind::Topaz,
                rank: 4,
            }),
        };
        let mut damage_bonus = DamageBonus(15.0);
        let mut health = Health {
            current: 116.0,
            max: 116.0,
        };
        let (gold_cost, shard_cost, essence_cost) =
            ancient_awaken_cost(equipment.socketed_gem.unwrap());
        let mut stats = RunStats {
            gold: gold_cost,
            ember_shards: shard_cost,
            affix_essence: essence_cost,
            ..default()
        };

        assert!(awaken_legendary_weapon(
            &mut inventory,
            &mut equipment,
            &mut damage_bonus,
            &mut health,
            &mut stats
        ));

        assert_eq!(stats.gold, 0);
        assert_eq!(stats.ember_shards, 0);
        assert_eq!(stats.affix_essence, 0);
        assert_eq!(equipment.weapon_name, "Ancient Stormcall Reliquary Brand");
        assert_eq!(equipment.quality, "ancient");
        assert_eq!(equipment.armor_bonus, 32.0);
        assert_eq!(equipment.socketed_gem, None);
        assert_eq!(inventory.items[0].name, equipment.weapon_name);
        assert_eq!(inventory.items[0].quality, "ancient");
        assert_eq!(inventory.items[0].armor_bonus, equipment.armor_bonus);
        assert_eq!(inventory.items[0].socketed_gem, None);
    }

    #[test]
    fn ancient_awakening_requires_legendary_power_high_rank_gem_and_essence() {
        let mut inventory = Inventory {
            items: vec![InventoryItem {
                name: "Moonforged Cleaver".to_string(),
                quality: "legendary".to_string(),
                damage_bonus: 12.0,
                crit_chance: 0.10,
                health_bonus: 10.0,
                armor_bonus: 10.0,
                legendary_power: LegendaryPower::None,
                temper_level: 0,
                socketed_gem: Some(SocketedGem {
                    kind: GemKind::Ruby,
                    rank: 4,
                }),
            }],
            capacity: 12,
        };
        let mut equipment = Equipment {
            weapon_name: "Moonforged Cleaver".to_string(),
            quality: "legendary".to_string(),
            crit_chance: 0.10,
            health_bonus: 10.0,
            armor_bonus: 10.0,
            legendary_power: LegendaryPower::None,
            temper_level: 0,
            socketed_gem: Some(SocketedGem {
                kind: GemKind::Ruby,
                rank: 4,
            }),
        };
        let mut damage_bonus = DamageBonus(12.0);
        let mut health = Health {
            current: 110.0,
            max: 110.0,
        };
        let mut stats = RunStats {
            gold: 999,
            ember_shards: 99,
            affix_essence: 99,
            ..default()
        };

        assert!(!awaken_legendary_weapon(
            &mut inventory,
            &mut equipment,
            &mut damage_bonus,
            &mut health,
            &mut stats
        ));
        equipment.legendary_power = LegendaryPower::Emberbrand;
        inventory.items[0].legendary_power = LegendaryPower::Emberbrand;
        equipment.socketed_gem = Some(SocketedGem {
            kind: GemKind::Ruby,
            rank: 3,
        });
        inventory.items[0].socketed_gem = equipment.socketed_gem;

        assert!(!awaken_legendary_weapon(
            &mut inventory,
            &mut equipment,
            &mut damage_bonus,
            &mut health,
            &mut stats
        ));
        equipment.socketed_gem = Some(SocketedGem {
            kind: GemKind::Ruby,
            rank: 4,
        });
        inventory.items[0].socketed_gem = equipment.socketed_gem;
        let (_, _, essence_cost) = ancient_awaken_cost(equipment.socketed_gem.unwrap());
        stats.affix_essence = essence_cost - 1;

        assert!(!awaken_legendary_weapon(
            &mut inventory,
            &mut equipment,
            &mut damage_bonus,
            &mut health,
            &mut stats
        ));

        assert_eq!(equipment.quality, "legendary");
        assert_eq!(equipment.weapon_name, "Moonforged Cleaver");
        assert_eq!(stats.affix_essence, essence_cost - 1);
    }

    #[test]
    fn ancient_augment_consumes_high_rank_ruby_and_essence_for_permanent_damage() {
        let mut inventory = Inventory {
            items: vec![InventoryItem {
                name: "Ancient Ashen Reliquary Reaver".to_string(),
                quality: "ancient".to_string(),
                damage_bonus: 22.0,
                crit_chance: 0.10,
                health_bonus: 12.0,
                armor_bonus: 12.0,
                legendary_power: LegendaryPower::Emberbrand,
                temper_level: 1,
                socketed_gem: Some(SocketedGem {
                    kind: GemKind::Ruby,
                    rank: 3,
                }),
            }],
            capacity: 12,
        };
        let mut equipment = Equipment {
            weapon_name: "Ancient Ashen Reliquary Reaver".to_string(),
            quality: "ancient".to_string(),
            crit_chance: 0.10,
            health_bonus: 12.0,
            armor_bonus: 12.0,
            legendary_power: LegendaryPower::Emberbrand,
            temper_level: 1,
            socketed_gem: Some(SocketedGem {
                kind: GemKind::Ruby,
                rank: 3,
            }),
        };
        let mut damage_bonus = DamageBonus(22.0);
        let mut health = Health {
            current: 112.0,
            max: 112.0,
        };
        let (gold_cost, shard_cost) = ancient_augment_cost(equipment.socketed_gem.unwrap());
        let essence_cost = ancient_augment_essence_cost(equipment.socketed_gem.unwrap());
        let mut stats = RunStats {
            gold: gold_cost,
            ember_shards: shard_cost,
            affix_essence: essence_cost,
            ..default()
        };

        assert!(augment_ancient_weapon(
            &mut inventory,
            &mut equipment,
            &mut damage_bonus,
            &mut health,
            &mut stats
        ));

        assert_eq!(stats.gold, 0);
        assert_eq!(stats.ember_shards, 0);
        assert_eq!(stats.affix_essence, 0);
        assert_eq!(stats.ancient_augments, 1);
        assert_eq!(equipment.socketed_gem, None);
        assert_eq!(inventory.items[0].socketed_gem, None);
        assert_eq!(damage_bonus.0, 29.5);
        assert_eq!(inventory.items[0].damage_bonus, 29.5);
    }

    #[test]
    fn ancient_augment_amethyst_updates_health_and_rejects_low_rank_gems() {
        let mut inventory = Inventory {
            items: vec![InventoryItem {
                name: "Ancient Bloodstar Blade".to_string(),
                quality: "ancient".to_string(),
                damage_bonus: 18.0,
                crit_chance: 0.08,
                health_bonus: 10.0,
                armor_bonus: 10.0,
                legendary_power: LegendaryPower::Soulreaver,
                temper_level: 0,
                socketed_gem: Some(SocketedGem {
                    kind: GemKind::Amethyst,
                    rank: 2,
                }),
            }],
            capacity: 12,
        };
        let mut equipment = Equipment {
            weapon_name: "Ancient Bloodstar Blade".to_string(),
            quality: "ancient".to_string(),
            crit_chance: 0.08,
            health_bonus: 10.0,
            armor_bonus: 10.0,
            legendary_power: LegendaryPower::Soulreaver,
            temper_level: 0,
            socketed_gem: Some(SocketedGem {
                kind: GemKind::Amethyst,
                rank: 2,
            }),
        };
        let mut damage_bonus = DamageBonus(18.0);
        let mut health = Health {
            current: 90.0,
            max: 110.0,
        };
        let mut stats = RunStats {
            gold: 999,
            ember_shards: 99,
            ..default()
        };

        assert!(!augment_ancient_weapon(
            &mut inventory,
            &mut equipment,
            &mut damage_bonus,
            &mut health,
            &mut stats
        ));
        equipment.socketed_gem = Some(SocketedGem {
            kind: GemKind::Amethyst,
            rank: 4,
        });
        inventory.items[0].socketed_gem = equipment.socketed_gem;
        let (gold_cost, shard_cost) = ancient_augment_cost(equipment.socketed_gem.unwrap());
        let essence_cost = ancient_augment_essence_cost(equipment.socketed_gem.unwrap());

        assert!(!augment_ancient_weapon(
            &mut inventory,
            &mut equipment,
            &mut damage_bonus,
            &mut health,
            &mut stats
        ));
        stats.affix_essence = essence_cost;

        assert!(augment_ancient_weapon(
            &mut inventory,
            &mut equipment,
            &mut damage_bonus,
            &mut health,
            &mut stats
        ));

        assert_eq!(equipment.socketed_gem, None);
        assert_eq!(equipment.health_bonus, 34.0);
        assert_eq!(inventory.items[0].health_bonus, 34.0);
        assert_eq!(health.max, 134.0);
        assert_eq!(health.current, 114.0);
        assert_eq!(stats.gold, 999 - gold_cost);
        assert_eq!(stats.ember_shards, 99 - shard_cost);
        assert_eq!(stats.affix_essence, 0);
    }

    #[test]
    fn primal_infusion_promotes_ancient_weapon_with_echo_keystone() {
        let mut inventory = Inventory {
            items: vec![InventoryItem {
                name: "Ancient Ashen Reliquary Reaver".to_string(),
                quality: "ancient".to_string(),
                damage_bonus: 30.0,
                crit_chance: 0.18,
                health_bonus: 18.0,
                armor_bonus: 24.0,
                legendary_power: LegendaryPower::Emberbrand,
                temper_level: 2,
                socketed_gem: None,
            }],
            capacity: 12,
        };
        let mut equipment = Equipment {
            weapon_name: "Ancient Ashen Reliquary Reaver".to_string(),
            quality: "ancient".to_string(),
            crit_chance: 0.18,
            health_bonus: 18.0,
            armor_bonus: 24.0,
            legendary_power: LegendaryPower::Emberbrand,
            temper_level: 2,
            socketed_gem: None,
        };
        let mut damage_bonus = DamageBonus(30.0);
        let mut health = Health {
            current: 118.0,
            max: 118.0,
        };
        let mut stats = RunStats {
            gold: PRIMAL_INFUSION_GOLD_COST,
            ember_shards: PRIMAL_INFUSION_SHARD_COST,
            affix_essence: PRIMAL_INFUSION_ESSENCE_COST,
            echo_keystones: PRIMAL_INFUSION_ECHO_KEYSTONE_COST,
            ..default()
        };

        assert!(infuse_primal_weapon(
            &mut inventory,
            &mut equipment,
            &mut damage_bonus,
            &mut health,
            &mut stats
        ));

        assert_eq!(stats.gold, 0);
        assert_eq!(stats.ember_shards, 0);
        assert_eq!(stats.affix_essence, 0);
        assert_eq!(stats.echo_keystones, 0);
        assert_eq!(stats.primal_infusions, 1);
        assert_eq!(equipment.weapon_name, "Primal Ashen Reliquary Reaver");
        assert_eq!(equipment.quality, "primal");
        assert_eq!(damage_bonus.0, 38.0);
        assert!((equipment.crit_chance - 0.205).abs() < 0.001);
        assert_eq!(equipment.health_bonus, 42.0);
        assert_eq!(equipment.armor_bonus, 34.0);
        assert_eq!(health.max, 142.0);
        assert_eq!(health.current, 142.0);
        assert_eq!(inventory.items[0].name, equipment.weapon_name);
        assert_eq!(inventory.items[0].quality, "primal");
        assert_eq!(inventory.items[0].damage_bonus, damage_bonus.0);
        assert_eq!(inventory.items[0].health_bonus, equipment.health_bonus);
    }

    #[test]
    fn primal_infusion_requires_ancient_weapon_and_echo_keystone() {
        let mut inventory = Inventory {
            items: vec![InventoryItem {
                name: "Stormcall Reliquary Brand".to_string(),
                quality: "legendary".to_string(),
                damage_bonus: 22.0,
                crit_chance: 0.12,
                health_bonus: 12.0,
                armor_bonus: 16.0,
                legendary_power: LegendaryPower::Stormbrand,
                temper_level: 1,
                socketed_gem: None,
            }],
            capacity: 12,
        };
        let mut equipment = Equipment {
            weapon_name: "Stormcall Reliquary Brand".to_string(),
            quality: "legendary".to_string(),
            crit_chance: 0.12,
            health_bonus: 12.0,
            armor_bonus: 16.0,
            legendary_power: LegendaryPower::Stormbrand,
            temper_level: 1,
            socketed_gem: None,
        };
        let mut damage_bonus = DamageBonus(22.0);
        let mut health = Health {
            current: 112.0,
            max: 112.0,
        };
        let mut stats = RunStats {
            gold: 999,
            ember_shards: 99,
            affix_essence: 99,
            echo_keystones: 1,
            ..default()
        };

        assert!(!infuse_primal_weapon(
            &mut inventory,
            &mut equipment,
            &mut damage_bonus,
            &mut health,
            &mut stats
        ));
        equipment.weapon_name = "Ancient Stormcall Reliquary Brand".to_string();
        equipment.quality = "ancient".to_string();
        inventory.items[0].name = equipment.weapon_name.clone();
        inventory.items[0].quality = equipment.quality.clone();
        stats.echo_keystones = 0;

        assert!(!infuse_primal_weapon(
            &mut inventory,
            &mut equipment,
            &mut damage_bonus,
            &mut health,
            &mut stats
        ));
        assert_eq!(equipment.quality, "ancient");
        assert_eq!(equipment.weapon_name, "Ancient Stormcall Reliquary Brand");
        assert_eq!(stats.gold, 999);
        assert_eq!(stats.ember_shards, 99);
        assert_eq!(stats.affix_essence, 99);
    }

    #[test]
    fn primal_infusion_pursuit_guides_gear_materials_and_completion() {
        let mut equipment = Equipment {
            weapon_name: "Iron Fang".to_string(),
            quality: "rare".to_string(),
            crit_chance: 0.04,
            health_bonus: 0.0,
            armor_bonus: 0.0,
            legendary_power: LegendaryPower::None,
            temper_level: 0,
            socketed_gem: None,
        };
        let mut stats = RunStats::default();

        assert!(primal_infusion_pursuit_summary(&stats, &equipment).contains("legendary weapon"));

        equipment.quality = "legendary".to_string();
        equipment.legendary_power = LegendaryPower::Emberbrand;
        assert!(primal_infusion_pursuit_summary(&stats, &equipment).contains("awaken legendary"));

        equipment.quality = "ancient".to_string();
        assert!(primal_infusion_pursuit_summary(&stats, &equipment).contains("Echo Keystone"));

        stats.echo_keystones = 1;
        stats.gold = PRIMAL_INFUSION_GOLD_COST - 5;
        stats.ember_shards = PRIMAL_INFUSION_SHARD_COST - 2;
        stats.affix_essence = PRIMAL_INFUSION_ESSENCE_COST - 1;
        let pursuit = primal_infusion_pursuit_summary(&stats, &equipment);
        assert!(pursuit.contains("5 gold"));
        assert!(pursuit.contains("2 shards"));
        assert!(pursuit.contains("1 essence"));

        stats.gold = PRIMAL_INFUSION_GOLD_COST;
        stats.ember_shards = PRIMAL_INFUSION_SHARD_COST;
        stats.affix_essence = PRIMAL_INFUSION_ESSENCE_COST;
        assert!(primal_infusion_pursuit_summary(&stats, &equipment).contains("Primal ready"));

        stats.primal_infusions = 1;
        assert!(primal_infusion_pursuit_summary(&stats, &equipment).contains("complete"));
    }

    #[test]
    fn quartermaster_preview_surfaces_next_trade_action() {
        let potions = PotionBelt {
            current: 3,
            max: 3,
            heal_amount: 45.0,
            cooldown: Timer::from_seconds(0.0, TimerMode::Once),
            cooldown_secs: potion_cooldown_secs_for_capacity(3),
        };
        let elixirs = ElixirBelt {
            current: 2,
            max: 2,
            armor_bonus: 35.0,
            duration_secs: 22.0,
            selected: default(),
        };
        let codex = LegendaryCodex::default();
        let mut equipment = Equipment {
            weapon_name: "Test Blade".to_string(),
            quality: "rare".to_string(),
            crit_chance: 0.05,
            health_bonus: 0.0,
            armor_bonus: 0.0,
            legendary_power: LegendaryPower::None,
            temper_level: 0,
            socketed_gem: None,
        };
        let inventory = Inventory {
            items: vec![
                InventoryItem {
                    name: "Test Blade".to_string(),
                    quality: "rare".to_string(),
                    damage_bonus: 10.0,
                    crit_chance: 0.05,
                    health_bonus: 0.0,
                    armor_bonus: 0.0,
                    legendary_power: LegendaryPower::None,
                    temper_level: 0,
                    socketed_gem: None,
                },
                InventoryItem {
                    name: "Spare Fang".to_string(),
                    quality: "common".to_string(),
                    damage_bonus: 3.0,
                    crit_chance: 0.02,
                    health_bonus: 0.0,
                    armor_bonus: 1.0,
                    legendary_power: LegendaryPower::None,
                    temper_level: 0,
                    socketed_gem: None,
                },
            ],
            capacity: 12,
        };
        let mut stats = RunStats::default();

        let preview = quartermaster_preview_summary(
            &stats, &inventory, &equipment, &potions, &elixirs, &codex,
        );
        assert!(preview.contains("U salvage 1 spare gear"));

        let empty_inventory = Inventory {
            items: vec![],
            capacity: 12,
        };
        equipment.quality = "ancient".to_string();
        stats.gold = PRIMAL_INFUSION_GOLD_COST + temper_cost(equipment.temper_level);
        stats.ember_shards = PRIMAL_INFUSION_SHARD_COST + MERCHANT_REFORGE_SHARD_COST;
        stats.affix_essence = PRIMAL_INFUSION_ESSENCE_COST;
        stats.echo_keystones = PRIMAL_INFUSION_ECHO_KEYSTONE_COST;
        let preview = quartermaster_preview_summary(
            &stats,
            &empty_inventory,
            &equipment,
            &potions,
            &elixirs,
            &codex,
        );
        assert!(preview.contains("can primal-infuse"));

        stats.gold = PRIMAL_INFUSION_GOLD_COST + temper_cost(equipment.temper_level) - 7;
        stats.ember_shards = PRIMAL_INFUSION_SHARD_COST + MERCHANT_REFORGE_SHARD_COST - 2;
        stats.affix_essence = PRIMAL_INFUSION_ESSENCE_COST - 1;
        stats.echo_keystones = 0;
        let preview = quartermaster_preview_summary(
            &stats,
            &empty_inventory,
            &equipment,
            &potions,
            &elixirs,
            &codex,
        );
        assert!(preview.contains("7g 2s 1e 1 key"));

        stats.gold = 0;
        stats.ember_shards = 0;
        stats.affix_essence = 0;
        stats.echo_keystones = 0;
        stats.malrec_soul_sigils = SOUL_SIGIL_CACHE_COST;
        let preview = quartermaster_preview_summary(
            &stats,
            &empty_inventory,
            &equipment,
            &potions,
            &elixirs,
            &codex,
        );
        assert!(preview.contains("opens 1 Trophy Cache"));
        assert!(preview.contains("+120g +4s +3e +1 key"));
        assert!(preview.contains("then Quartermaster needs:"));
        assert!(preview.contains("for primal infusion"));

        stats.malrec_soul_sigils = SOUL_SIGIL_CACHE_COST * 2;
        let preview = quartermaster_preview_summary(
            &stats,
            &empty_inventory,
            &equipment,
            &potions,
            &elixirs,
            &codex,
        );
        assert!(preview.contains("opens 2 Trophy Caches"));
        assert!(preview.contains("+240g +8s +6e +2 keys"));
    }

    #[test]
    fn quartermaster_preview_reports_crafting_material_gaps() {
        let potions = PotionBelt {
            current: 3,
            max: 3,
            heal_amount: 45.0,
            cooldown: Timer::from_seconds(0.0, TimerMode::Once),
            cooldown_secs: potion_cooldown_secs_for_capacity(3),
        };
        let elixirs = ElixirBelt {
            current: 2,
            max: 2,
            armor_bonus: 35.0,
            duration_secs: 22.0,
            selected: default(),
        };
        let inventory = Inventory {
            items: Vec::new(),
            capacity: MAX_INVENTORY_CAPACITY,
        };
        let codex = LegendaryCodex::default();
        let mut equipment = Equipment {
            weapon_name: "Soulreaver Fang".to_string(),
            quality: "legendary".to_string(),
            crit_chance: 0.05,
            health_bonus: 0.0,
            armor_bonus: 0.0,
            legendary_power: LegendaryPower::Soulreaver,
            temper_level: 8,
            socketed_gem: Some(SocketedGem {
                kind: GemKind::Ruby,
                rank: 4,
            }),
        };
        let mut stats = RunStats::default();

        let preview = quartermaster_preview_summary(
            &stats, &inventory, &equipment, &potions, &elixirs, &codex,
        );
        assert!(preview.contains("680g 28s 12e"));
        assert!(preview.contains("to awaken Ruby 4"));

        equipment.quality = "ancient".to_string();
        equipment.socketed_gem = Some(SocketedGem {
            kind: GemKind::Topaz,
            rank: 3,
        });
        stats.gold = 200;
        stats.ember_shards = 9;
        stats.affix_essence = 1;
        let preview = quartermaster_preview_summary(
            &stats, &inventory, &equipment, &potions, &elixirs, &codex,
        );
        assert!(preview.contains("Quartermaster needs:"));
        assert!(preview.contains("for primal infusion"));
        assert!(preview.contains("key"));
    }

    #[test]
    fn salvage_preview_preserves_equipped_item_and_prices_spares() {
        let equipment = Equipment {
            weapon_name: "Stormcall Reliquary Brand".to_string(),
            quality: "legendary".to_string(),
            crit_chance: 0.16,
            health_bonus: 0.0,
            armor_bonus: 0.0,
            legendary_power: LegendaryPower::Stormbrand,
            temper_level: 0,
            socketed_gem: None,
        };
        let inventory = Inventory {
            items: vec![
                InventoryItem {
                    name: equipment.weapon_name.clone(),
                    quality: equipment.quality.clone(),
                    damage_bonus: 11.0,
                    crit_chance: equipment.crit_chance,
                    health_bonus: 0.0,
                    armor_bonus: 0.0,
                    legendary_power: equipment.legendary_power,
                    temper_level: 0,
                    socketed_gem: None,
                },
                InventoryItem {
                    name: "Moonforged Cleaver".to_string(),
                    quality: "rare".to_string(),
                    damage_bonus: 9.0,
                    crit_chance: 0.06,
                    health_bonus: 0.0,
                    armor_bonus: 0.0,
                    legendary_power: LegendaryPower::None,
                    temper_level: 0,
                    socketed_gem: None,
                },
                InventoryItem {
                    name: "Aegis-Saint Reliquary Guard".to_string(),
                    quality: "legendary".to_string(),
                    damage_bonus: 12.0,
                    crit_chance: 0.09,
                    health_bonus: 20.0,
                    armor_bonus: 10.0,
                    legendary_power: LegendaryPower::Aegisbrand,
                    temper_level: 1,
                    socketed_gem: Some(SocketedGem {
                        kind: GemKind::Ruby,
                        rank: 2,
                    }),
                },
            ],
            capacity: 12,
        };

        let preview = salvage_preview(&inventory, &equipment);

        assert_eq!(preview.items, 2);
        assert!(preview.gold > 0);
        assert!(preview.shards >= 14);
        assert!(preview.essence >= 1);
    }

    #[test]
    fn quick_salvage_preserves_equipped_item_and_grants_cache_rewards() {
        let equipment = Equipment {
            weapon_name: "Stormcall Reliquary Brand".to_string(),
            quality: "legendary".to_string(),
            crit_chance: 0.16,
            health_bonus: 0.0,
            armor_bonus: 0.0,
            legendary_power: LegendaryPower::Stormbrand,
            temper_level: 0,
            socketed_gem: None,
        };
        let mut inventory = Inventory {
            items: vec![
                InventoryItem {
                    name: equipment.weapon_name.clone(),
                    quality: equipment.quality.clone(),
                    damage_bonus: 11.0,
                    crit_chance: equipment.crit_chance,
                    health_bonus: 0.0,
                    armor_bonus: 0.0,
                    legendary_power: equipment.legendary_power,
                    temper_level: 0,
                    socketed_gem: None,
                },
                InventoryItem {
                    name: "Spare Moon Fang".to_string(),
                    quality: "rare".to_string(),
                    damage_bonus: 7.0,
                    crit_chance: 0.04,
                    health_bonus: 0.0,
                    armor_bonus: 0.0,
                    legendary_power: LegendaryPower::None,
                    temper_level: 0,
                    socketed_gem: None,
                },
                InventoryItem {
                    name: "Spare Ember Fang".to_string(),
                    quality: "rare".to_string(),
                    damage_bonus: 8.0,
                    crit_chance: 0.05,
                    health_bonus: 0.0,
                    armor_bonus: 0.0,
                    legendary_power: LegendaryPower::None,
                    temper_level: 0,
                    socketed_gem: None,
                },
                InventoryItem {
                    name: "Spare Saint Fang".to_string(),
                    quality: "legendary".to_string(),
                    damage_bonus: 12.0,
                    crit_chance: 0.09,
                    health_bonus: 20.0,
                    armor_bonus: 10.0,
                    legendary_power: LegendaryPower::Aegisbrand,
                    temper_level: 0,
                    socketed_gem: None,
                },
                InventoryItem {
                    name: "Spare Ancient Fang".to_string(),
                    quality: "ancient".to_string(),
                    damage_bonus: 18.0,
                    crit_chance: 0.16,
                    health_bonus: 24.0,
                    armor_bonus: 14.0,
                    legendary_power: LegendaryPower::Emberbrand,
                    temper_level: 0,
                    socketed_gem: None,
                },
            ],
            capacity: 12,
        };
        let preview = salvage_preview(&inventory, &equipment);
        let mut stats = RunStats::default();

        let salvage = salvage_spare_inventory_into_stats(&mut inventory, &equipment, &mut stats);

        assert_eq!(salvage.items, 4);
        assert_eq!(inventory.items.len(), 1);
        assert_eq!(inventory.items[0].name, equipment.weapon_name);
        assert_eq!(stats.gold, preview.gold + SALVAGE_CACHE_GOLD_REWARD);
        assert_eq!(
            stats.ember_shards,
            preview.shards + SALVAGE_CACHE_SHARD_REWARD
        );
        assert_eq!(
            stats.affix_essence,
            preview.essence + SALVAGE_CACHE_ESSENCE_REWARD
        );
        assert_eq!(stats.salvage_caches, 1);
        assert!(salvage.summary().contains("salvage cache x1"));
    }

    #[test]
    fn loot_pursuit_summary_points_to_salvage_or_next_chase() {
        let mut equipment = Equipment {
            weapon_name: "Ancient Ashen Reliquary Reaver".to_string(),
            quality: "ancient".to_string(),
            crit_chance: 0.16,
            health_bonus: 0.0,
            armor_bonus: 0.0,
            legendary_power: LegendaryPower::Emberbrand,
            temper_level: 0,
            socketed_gem: None,
        };
        let mut stats = RunStats::default();
        let mut inventory = Inventory {
            items: vec![InventoryItem {
                name: "Spare Fang".to_string(),
                quality: "common".to_string(),
                damage_bonus: 3.0,
                crit_chance: 0.02,
                health_bonus: 0.0,
                armor_bonus: 1.0,
                legendary_power: LegendaryPower::None,
                temper_level: 0,
                socketed_gem: None,
            }],
            capacity: 12,
        };

        let summary = loot_pursuit_summary(&stats, &inventory, &equipment);
        assert!(summary.contains("U salvage 1"));
        assert!(summary.contains("+"));

        inventory.items.clear();
        stats.echo_keystones = 0;
        let summary = loot_pursuit_summary(&stats, &inventory, &equipment);
        assert!(summary.contains("Echo Keystone"));

        equipment.quality = "rare".to_string();
        let summary = loot_pursuit_summary(&stats, &inventory, &equipment);
        assert!(summary.contains("legendary gear"));
    }

    #[test]
    fn merchant_polishes_equipped_charm_with_gold_and_shards() {
        let mut inventory = Inventory {
            items: vec![InventoryItem {
                name: "rare Keeper Blade".to_string(),
                quality: "rare".to_string(),
                damage_bonus: 12.0,
                crit_chance: 0.08,
                health_bonus: 8.0,
                armor_bonus: 9.0,
                legendary_power: LegendaryPower::None,
                temper_level: 2,
                socketed_gem: None,
            }],
            capacity: MAX_INVENTORY_CAPACITY,
        };
        let mut equipment = Equipment {
            weapon_name: "rare Keeper Blade".to_string(),
            quality: "rare".to_string(),
            crit_chance: 0.08,
            health_bonus: 8.0,
            armor_bonus: 9.0,
            legendary_power: LegendaryPower::None,
            temper_level: 2,
            socketed_gem: None,
        };
        let mut damage_bonus = DamageBonus(12.0);
        let mut health = Health {
            current: 100.0,
            max: 100.0,
        };
        let mut potions = PotionBelt {
            current: MAX_POTION_CAPACITY,
            max: MAX_POTION_CAPACITY,
            heal_amount: 45.0,
            cooldown: Timer::from_seconds(0.0, TimerMode::Once),
            cooldown_secs: potion_cooldown_secs_for_capacity(MAX_POTION_CAPACITY),
        };
        let mut elixirs = ElixirBelt {
            current: MAX_ELIXIR_CAPACITY,
            max: MAX_ELIXIR_CAPACITY,
            armor_bonus: 35.0,
            duration_secs: 22.0,
            selected: default(),
        };
        let mut charm = charm_for_roll("rare", 0);
        let mut stats = RunStats {
            gold: MERCHANT_CHARM_POLISH_GOLD_COST,
            ember_shards: MERCHANT_CHARM_POLISH_SHARD_COST,
            ..RunStats::default()
        };
        let mut codex = LegendaryCodex::default();
        let loot_table = merchant_test_loot_table();

        let transaction = trade_with_merchant(MerchantTrade {
            loot_table: &loot_table,
            inventory: &mut inventory,
            equipment: &mut equipment,
            damage_bonus: &mut damage_bonus,
            charm: &mut charm,
            health: &mut health,
            potions: &mut potions,
            elixirs: &mut elixirs,
            codex: &mut codex,
            stats: &mut stats,
        });

        assert!(transaction.charm_polished);
        assert_eq!(stats.gold, 0);
        assert_eq!(stats.ember_shards, 0);
        assert_eq!(charm.damage_bonus, 4.5);
        assert!((charm.crit_chance - 0.023).abs() < 0.001);
        assert!(transaction.summary().contains("polished Gilded Fang Charm"));
        assert!(!transaction.tempered);
        assert!(!transaction.reforged);
    }

    #[test]
    fn mystery_charm_gamble_can_upgrade_charm_with_gold_and_shards() {
        let mut charm = Charm::default();
        let mut stats = RunStats {
            gold: MERCHANT_MYSTERY_CHARM_GOLD_COST,
            ember_shards: MERCHANT_MYSTERY_CHARM_SHARD_COST,
            ..RunStats::default()
        };

        let gamble = gamble_mystery_charm(&mut charm, &mut stats, 0.10, 0).unwrap();

        assert!(gamble.equipped);
        assert_eq!(gamble.charm.quality, "legendary");
        assert_eq!(charm.name, "Stormglass Charm");
        assert_eq!(stats.gold, 0);
        assert_eq!(stats.ember_shards, 0);
    }

    #[test]
    fn mystery_charm_gamble_keeps_better_current_charm() {
        let mut charm = charm_for_roll("legendary", 0);
        let mut stats = RunStats {
            gold: MERCHANT_MYSTERY_CHARM_GOLD_COST,
            ember_shards: MERCHANT_MYSTERY_CHARM_SHARD_COST,
            ..RunStats::default()
        };

        let gamble = gamble_mystery_charm(&mut charm, &mut stats, 0.90, 1).unwrap();

        assert!(!gamble.equipped);
        assert_eq!(gamble.charm.quality, "common");
        assert_eq!(charm.name, "Stormglass Charm");
        assert_eq!(stats.gold, 0);
        assert_eq!(stats.ember_shards, 0);
    }

    #[test]
    fn mystery_weapon_gamble_stores_equips_and_unlocks_codex() {
        let loot_table = merchant_test_loot_table();
        let weapon = &loot_table.weapons[1];
        let mut inventory = Inventory {
            items: Vec::new(),
            capacity: 4,
        };
        let mut equipment = Equipment {
            weapon_name: "common Training Sword".to_string(),
            quality: "common".to_string(),
            crit_chance: 0.02,
            health_bonus: 0.0,
            armor_bonus: 0.0,
            legendary_power: LegendaryPower::None,
            temper_level: 0,
            socketed_gem: None,
        };
        let mut damage_bonus = DamageBonus(4.0);
        let mut health = Health {
            current: 100.0,
            max: 100.0,
        };
        let mut codex = LegendaryCodex::default();
        let mut stats = RunStats {
            gold: MERCHANT_MYSTERY_WEAPON_GOLD_COST,
            ember_shards: MERCHANT_MYSTERY_WEAPON_SHARD_COST,
            ..RunStats::default()
        };

        let gamble = gamble_mystery_weapon(
            weapon,
            &mut inventory,
            &mut equipment,
            &mut damage_bonus,
            &mut health,
            &mut codex,
            &mut stats,
        )
        .unwrap();

        assert!(gamble.stored);
        assert!(gamble.equipped);
        assert!(gamble.codex_unlocked);
        assert_eq!(gamble.item.name, "legendary Stormglass Reaver");
        assert_eq!(equipment.weapon_name, "legendary Stormglass Reaver");
        assert_eq!(equipment.legendary_power, LegendaryPower::Stormbrand);
        assert!(codex.unlocked.contains(&LegendaryPower::Stormbrand));
        assert_eq!(codex.attuned, LegendaryPower::Stormbrand);
        assert_eq!(inventory.items.len(), 1);
        assert_eq!(stats.gold, 0);
        assert_eq!(stats.ember_shards, 0);
    }

    #[test]
    fn upgraded_loot_can_include_elixirs() {
        assert_eq!(elixirs_for_quality("common"), 0);
        assert_eq!(elixirs_for_quality("rare"), 1);
        assert_eq!(elixirs_for_quality("legendary"), 1);
        assert_eq!(elixirs_for_quality("ancient"), 2);
        assert_eq!(elixirs_for_quality("primal"), 3);
    }

    #[test]
    fn better_charm_replaces_current_charm() {
        let mut current = Charm::default();
        let rare = charm_for_roll("rare", 0);
        let legendary = charm_for_roll("legendary", 0);
        let ancient = charm_for_roll("ancient", 0);
        let primal = charm_for_roll("primal", 0);

        assert!(equip_charm_if_better(&mut current, &rare));
        assert_eq!(current.name, rare.name);
        assert!(equip_charm_if_better(&mut current, &legendary));
        assert_eq!(current.name, legendary.name);
        assert!(equip_charm_if_better(&mut current, &ancient));
        assert_eq!(current.name, ancient.name);
        assert!(equip_charm_if_better(&mut current, &primal));
        assert_eq!(current.name, primal.name);
        assert!(!equip_charm_if_better(&mut current, &rare));
        assert_eq!(current.name, primal.name);
    }

    #[test]
    fn loot_filter_tiers_cycle_and_map_to_quality_ranks() {
        let mut tier = LootFilterTier::All;

        assert_eq!(tier.min_rank(), loot_quality_rank("common"));
        tier = tier.next();
        assert_eq!(tier, LootFilterTier::RarePlus);
        assert_eq!(tier.min_rank(), loot_quality_rank("rare"));
        tier = tier.next();
        assert_eq!(tier, LootFilterTier::LegendaryPlus);
        assert_eq!(tier.min_rank(), loot_quality_rank("legendary"));
        tier = tier.next();
        assert_eq!(tier, LootFilterTier::AncientPlus);
        assert_eq!(tier.min_rank(), loot_quality_rank("ancient"));
        tier = tier.next();
        assert_eq!(tier, LootFilterTier::Primal);
        assert_eq!(tier.min_rank(), loot_quality_rank("primal"));
        assert_eq!(tier.next(), LootFilterTier::All);
    }

    #[test]
    fn health_globe_drop_rules_reward_elites_and_bosses() {
        assert!(health_globe_drops("keeper", false, 0.99));
        assert!(!health_globe_drops("treasure_imp", true, 0.0));
        assert!(health_globe_drops("skeleton", true, 0.64));
        assert!(!health_globe_drops("skeleton", true, 0.65));
        assert!(health_globe_drops("skeleton", false, 0.17));
        assert!(!health_globe_drops("skeleton", false, 0.18));
    }

    #[test]
    fn fury_globe_drop_rules_reward_resource_pressure_fights() {
        assert!(fury_globe_drops("keeper", false, 0.99));
        assert!(fury_globe_drops("seal_warden", false, 0.84));
        assert!(!fury_globe_drops("seal_warden", false, 0.85));
        assert!(fury_globe_drops("treasure_imp", true, 0.34));
        assert!(!fury_globe_drops("treasure_imp", true, 0.35));
        assert!(fury_globe_drops("skeleton", true, 0.41));
        assert!(!fury_globe_drops("skeleton", true, 0.42));
        assert!(fury_globe_drops("skeleton", false, 0.09));
        assert!(!fury_globe_drops("skeleton", false, 0.10));
    }

    #[test]
    fn fury_globe_amount_scales_from_max_fury() {
        assert_eq!(fury_globe_amount(80.0, 0.38), 30.0);
        assert_eq!(fury_globe_amount(0.0, 0.38), 1.0);
    }

    #[test]
    fn elite_and_boss_kills_reward_ember_shards() {
        assert_eq!(enemy_shard_reward("skeleton", false), 0);
        assert_eq!(enemy_shard_reward("skeleton", true), 2);
        assert_eq!(enemy_shard_reward("keeper", false), 6);
        assert_eq!(enemy_shard_reward("nemesis", true), 6);
        assert_eq!(enemy_shard_reward("treasure_imp", true), 10);
    }

    #[test]
    fn affixed_enemies_reward_essence_by_affix_count() {
        assert_eq!(enemy_affix_essence_reward("skeleton", 0), 0);
        assert_eq!(enemy_affix_essence_reward("skeleton", 1), 1);
        assert_eq!(enemy_affix_essence_reward("cultist", 3), 3);
        assert_eq!(enemy_affix_essence_reward("keeper", 2), 4);
        assert_eq!(enemy_affix_essence_reward("nemesis", 5), 7);
        assert_eq!(enemy_affix_essence_reward("treasure_imp", 1), 2);
    }

    #[test]
    fn elite_affix_break_rewards_multi_affix_elite_pressure() {
        assert_eq!(elite_affix_break_essence_reward("skeleton", 0), 0);
        assert_eq!(elite_affix_break_essence_reward("skeleton", 1), 0);
        assert_eq!(elite_affix_break_essence_reward("skeleton", 2), 1);
        assert_eq!(elite_affix_break_essence_reward("cultist", 4), 3);
        assert_eq!(elite_affix_break_essence_reward("nemesis", 5), 6);
        assert_eq!(elite_affix_break_essence_reward("keeper", 4), 0);

        assert_eq!(
            elite_affix_break_reward_text("Arcane Molten Guard", 2),
            "Affix Break: Arcane Molten Guard shattered, +2 essence"
        );
        assert_eq!(elite_affix_break_float_text(3), "AFFIX BREAK +3E");
    }

    #[test]
    fn elite_affix_break_death_vfx_scales_with_affix_pressure() {
        let two_affix = elite_affix_break_death_style(2);
        let many_affix = elite_affix_break_death_style(6);

        assert!(many_affix.scale > two_affix.scale);
        assert!(many_affix.light_intensity > two_affix.light_intensity);
        assert!(many_affix.light_range > two_affix.light_range);
        assert!(many_affix.lifetime_secs > two_affix.lifetime_secs);
        assert_eq!(two_affix.light_color, Color::srgb(0.70, 0.38, 1.0));
    }

    #[test]
    fn duplicate_codex_drops_refine_into_affix_essence() {
        let mut codex = LegendaryCodex::default();
        let mut stats = RunStats::default();

        assert_eq!(
            claim_codex_drop_power(&mut codex, &mut stats, LegendaryPower::None, "legendary"),
            CodexDropClaim::None
        );
        assert_eq!(stats.affix_essence, 0);

        assert_eq!(
            claim_codex_drop_power(
                &mut codex,
                &mut stats,
                LegendaryPower::Stormbrand,
                "legendary"
            ),
            CodexDropClaim::Unlocked
        );
        assert_eq!(stats.affix_essence, 0);

        assert_eq!(
            claim_codex_drop_power(
                &mut codex,
                &mut stats,
                LegendaryPower::Stormbrand,
                "legendary"
            ),
            CodexDropClaim::DuplicateEssence(2)
        );
        assert_eq!(stats.affix_essence, 2);

        assert_eq!(
            claim_codex_drop_power(
                &mut codex,
                &mut stats,
                LegendaryPower::Stormbrand,
                "ancient"
            ),
            CodexDropClaim::DuplicateEssence(3)
        );
        assert_eq!(stats.affix_essence, 5);

        assert_eq!(
            claim_codex_drop_power(&mut codex, &mut stats, LegendaryPower::Stormbrand, "primal"),
            CodexDropClaim::DuplicateEssence(4)
        );
        assert_eq!(stats.affix_essence, 9);
        assert_eq!(duplicate_codex_essence_reward("rare"), 0);
    }

    #[test]
    fn health_globe_heal_amount_scales_from_max_health() {
        assert_eq!(health_globe_heal_amount(120.0, 0.30), 36.0);
        assert_eq!(health_globe_heal_amount(0.0, 0.30), 1.0);
    }

    #[test]
    fn loot_readability_scales_with_quality() {
        assert_eq!(loot_beam_height("common"), 0.0);
        assert_eq!(loot_beam_radius("common"), 0.0);
        assert!(loot_ground_ring_radius("primal") > loot_ground_ring_radius("rare"));
        assert!(loot_ground_ring_alpha("legendary") > loot_ground_ring_alpha("common"));
        assert!(loot_label_font_size("primal") > loot_label_font_size("ancient"));
        assert!(loot_label_font_size("ancient") > loot_label_font_size("legendary"));
        assert!(loot_label_font_size("legendary") > loot_label_font_size("rare"));
        assert!(loot_beam_height("legendary") > loot_beam_height("rare"));
        assert!(loot_beam_height("ancient") > loot_beam_height("legendary"));
        assert!(loot_beam_height("primal") > loot_beam_height("ancient"));
        assert_ne!(loot_quality_color("rare"), loot_quality_color("legendary"));
        assert!(loot_spin_speed("primal") > loot_spin_speed("legendary"));
        assert!(loot_pulse_amount("ancient") > loot_pulse_amount("rare"));
        assert!(loot_glow_intensity("legendary") > loot_glow_intensity("rare"));
        assert!(loot_glow_range("primal") > loot_glow_range("ancient"));
        assert!(
            loot_pulse_factor(std::f32::consts::FRAC_PI_2, 1.0, 0.1)
                > loot_pulse_factor(0.0, 1.0, 0.1)
        );
    }

    #[test]
    fn loot_label_text_surfaces_actual_drop_value() {
        let drop = LootDrop {
            gold: 42,
            damage_bonus: 12.0,
            crit_chance: 0.05,
            health_bonus: 18.0,
            armor_bonus: 7.0,
            legendary_power: LegendaryPower::Stormbrand,
            temper_level: 0,
            socketed_gem: None,
            bonus_gem: Some(SocketedGem::new(GemKind::Ruby)),
            bonus_charm: Some(charm_for_roll("legendary", 0)),
            potions: 1,
            elixirs: 1,
            quality: "legendary".to_string(),
            label: "legendary Storm Fang".to_string(),
        };

        let text = loot_label_text(&drop);
        let lines = text.lines().collect::<Vec<_>>();

        assert!(text.contains("legendary Storm Fang"));
        assert!(text.contains("+12 dmg"));
        assert!(text.contains("+5% crit"));
        assert!(text.contains("+18 hp"));
        assert!(text.contains("+7 armor"));
        assert!(text.contains("Stormbrand"));
        assert!(text.contains("gem"));
        assert!(text.contains("charm"));
        assert!(text.contains("1 potion"));
        assert!(text.contains("1 elixir"));
        assert!(text.contains("42 gold"));
        assert_eq!(lines.len(), 3);
        assert_eq!(lines[0], "LEGENDARY DROP");
        assert!(lines[1].contains("Stormbrand"));
        assert!(lines[2].contains("42 gold"));
    }

    #[test]
    fn loot_label_reward_summary_stays_empty_without_extra_rewards() {
        let drop = LootDrop {
            gold: 0,
            damage_bonus: 4.0,
            crit_chance: 0.0,
            health_bonus: 0.0,
            armor_bonus: 0.0,
            legendary_power: LegendaryPower::None,
            temper_level: 0,
            socketed_gem: None,
            bonus_gem: None,
            bonus_charm: None,
            potions: 0,
            elixirs: 0,
            quality: "common".to_string(),
            label: "common Iron Fang".to_string(),
        };

        assert_eq!(loot_label_reward_summary(&drop), "");
        assert_eq!(loot_label_text(&drop).lines().count(), 1);
        assert_eq!(loot_label_rarity_header(&drop.quality), "");
    }

    #[test]
    fn loot_label_compare_marks_ground_upgrade_and_rewards() {
        let equipped = ItemStats {
            quality: "rare",
            damage_bonus: 10.0,
            crit_chance: 0.06,
            health_bonus: 8.0,
            armor_bonus: 6.0,
            legendary_power: LegendaryPower::None,
            temper_level: 0,
            socketed_gem: None,
        };
        let drop = LootDrop {
            gold: 42,
            damage_bonus: 18.0,
            crit_chance: 0.12,
            health_bonus: 20.0,
            armor_bonus: 16.0,
            legendary_power: LegendaryPower::Stormbrand,
            temper_level: 0,
            socketed_gem: None,
            bonus_gem: Some(SocketedGem::new(GemKind::Ruby)),
            bonus_charm: None,
            potions: 1,
            elixirs: 0,
            quality: "legendary".to_string(),
            label: "legendary Storm Fang".to_string(),
        };

        let text = loot_label_text_with_compare(&drop, equipped, true);
        let lines = text.lines().collect::<Vec<_>>();

        assert_eq!(lines.len(), 4);
        assert_eq!(lines[0], "LEGENDARY DROP");
        assert!(lines[1].contains("legendary Storm Fang"));
        assert!(lines[2].contains("AUTO-EQUIP"));
        assert!(lines[2].contains("BUILD POWER"));
        assert!(lines[2].contains("PWR +"));
        assert!(lines[2].contains("dmg +8"));
        assert!(lines[2].contains("new power Stormbrand"));
        assert!(lines[2].contains("weapon dmg +8"));
        assert!(lines[2].contains("crit build +6%"));
        assert!(lines[2].contains("toughness +22"));
        assert!(lines[3].contains("42 gold"));
        assert!(lines[3].contains("gem"));
    }

    #[test]
    fn loot_ground_compare_marks_sidegrade_stash_and_full() {
        let equipped = ItemStats {
            quality: "rare",
            damage_bonus: 10.0,
            crit_chance: 0.06,
            health_bonus: 8.0,
            armor_bonus: 6.0,
            legendary_power: LegendaryPower::None,
            temper_level: 0,
            socketed_gem: None,
        };
        let sidegrade = LootDrop {
            gold: 0,
            damage_bonus: 9.0,
            crit_chance: 0.06,
            health_bonus: 8.0,
            armor_bonus: 6.0,
            legendary_power: LegendaryPower::None,
            temper_level: 0,
            socketed_gem: None,
            bonus_gem: None,
            bonus_charm: None,
            potions: 0,
            elixirs: 0,
            quality: "rare".to_string(),
            label: "rare Mirror Fang".to_string(),
        };
        let weaker = LootDrop {
            gold: 0,
            damage_bonus: 2.0,
            crit_chance: 0.01,
            health_bonus: 0.0,
            armor_bonus: 0.0,
            legendary_power: LegendaryPower::None,
            temper_level: 0,
            socketed_gem: None,
            bonus_gem: None,
            bonus_charm: None,
            potions: 0,
            elixirs: 0,
            quality: "common".to_string(),
            label: "common Bent Fang".to_string(),
        };

        assert!(loot_ground_compare_summary(&sidegrade, equipped, true).contains("SIDEGRADE"));
        assert!(loot_ground_compare_summary(&sidegrade, equipped, true).contains("BUILD SWAP"));
        assert!(loot_ground_compare_summary(&weaker, equipped, true).contains("STASH"));
        assert!(loot_ground_compare_summary(&weaker, equipped, true).contains("SALVAGE LATER"));
        assert!(loot_ground_compare_summary(&weaker, equipped, false).contains("FULL"));
        assert!(loot_ground_compare_summary(&weaker, equipped, false).contains("MAKE ROOM"));
        assert!(loot_ground_decision(0.0, false).contains("AUTO-EQUIP"));
    }

    #[test]
    fn loot_proximity_info_reports_pickup_readiness_and_inventory_state() {
        let equipped = ItemStats {
            quality: "rare",
            damage_bonus: 8.0,
            crit_chance: 0.04,
            health_bonus: 0.0,
            armor_bonus: 0.0,
            legendary_power: LegendaryPower::None,
            temper_level: 0,
            socketed_gem: None,
        };
        let drop = LootDrop {
            gold: 17,
            damage_bonus: 18.0,
            crit_chance: 0.08,
            health_bonus: 25.0,
            armor_bonus: 4.0,
            legendary_power: LegendaryPower::Emberbrand,
            temper_level: 1,
            socketed_gem: None,
            bonus_gem: None,
            bonus_charm: None,
            potions: 0,
            elixirs: 1,
            quality: "legendary".to_string(),
            label: "Ember-Saint Relic Blade".to_string(),
        };

        let ready = loot_proximity_info(&drop, LOOT_PICKUP_RADIUS - 0.1, equipped, true, false);
        assert!(ready.visible);
        assert!(ready.pickup_ready);
        assert!(!ready.inventory_full);
        assert_eq!(ready.quality, "legendary");
        assert!(ready.label.contains("Ember-Saint Relic Blade"));
        assert!(ready.compare.contains("AUTO-EQUIP"));
        assert!(ready.compare.contains("BUILD POWER"));
        assert!(ready.compare.contains("PWR +"));
        assert!(ready.compare.contains("new power Emberbrand"));
        assert!(ready.compare.contains("temper +1"));
        assert!(ready.compare.contains("weapon dmg +10"));
        assert_eq!(ready.priority, "BEST");
        assert!(ready.rewards.contains("17 gold"));
        assert!(ready.rewards.contains("elixir"));

        let full = loot_proximity_info(&drop, LOOT_PICKUP_RADIUS + 2.0, equipped, false, true);
        assert!(!full.pickup_ready);
        assert!(full.inventory_full);
        assert!(full.spare_salvage_available);
        assert_eq!(full.priority, "BEST");
        assert!(full.compare.contains("AUTO-EQUIP"));
    }

    #[test]
    fn focused_loot_drop_prioritizes_nearby_upgrade_over_closest_junk() {
        let equipped = ItemStats {
            quality: "rare",
            damage_bonus: 11.0,
            crit_chance: 0.04,
            health_bonus: 8.0,
            armor_bonus: 5.0,
            legendary_power: LegendaryPower::None,
            temper_level: 0,
            socketed_gem: None,
        };
        let junk = LootDrop {
            gold: 3,
            damage_bonus: 2.0,
            crit_chance: 0.0,
            health_bonus: 0.0,
            armor_bonus: 0.0,
            legendary_power: LegendaryPower::None,
            temper_level: 0,
            socketed_gem: None,
            bonus_gem: None,
            bonus_charm: None,
            potions: 0,
            elixirs: 0,
            quality: "common".to_string(),
            label: "Cracked Fang".to_string(),
        };
        let upgrade = LootDrop {
            gold: 18,
            damage_bonus: 17.0,
            crit_chance: 0.07,
            health_bonus: 14.0,
            armor_bonus: 7.0,
            legendary_power: LegendaryPower::Emberbrand,
            temper_level: 1,
            socketed_gem: None,
            bonus_gem: None,
            bonus_charm: None,
            potions: 0,
            elixirs: 1,
            quality: "legendary".to_string(),
            label: "Ember-Saint Relic Blade".to_string(),
        };

        let (focused, distance) = focused_loot_drop_from_iter(
            vec![(&junk, 0.2), (&upgrade, LOOT_PICKUP_RADIUS + 1.6)].into_iter(),
            equipped,
            true,
        )
        .expect("focused drop");

        assert_eq!(focused.label, "Ember-Saint Relic Blade");
        assert_eq!(distance, LOOT_PICKUP_RADIUS + 1.6);
        assert_eq!(loot_priority_label(focused, equipped, true), "BEST");

        let (fallback, distance) = focused_loot_drop_from_iter(
            vec![(&junk, 0.2), (&upgrade, LOOT_PICKUP_RADIUS + 6.0)].into_iter(),
            equipped,
            true,
        )
        .expect("fallback drop");

        assert_eq!(fallback.label, "Cracked Fang");
        assert_eq!(distance, 0.2);
    }

    #[test]
    fn loot_priority_label_separates_build_pickups_from_salvage() {
        let equipped = ItemStats {
            quality: "rare",
            damage_bonus: 12.0,
            crit_chance: 0.04,
            health_bonus: 10.0,
            armor_bonus: 8.0,
            legendary_power: LegendaryPower::None,
            temper_level: 0,
            socketed_gem: None,
        };
        let socket_sidegrade = LootDrop {
            gold: 0,
            damage_bonus: 0.0,
            crit_chance: 0.0,
            health_bonus: 0.0,
            armor_bonus: 0.0,
            legendary_power: LegendaryPower::None,
            temper_level: 0,
            socketed_gem: Some(SocketedGem::new(GemKind::Ruby)),
            bonus_gem: None,
            bonus_charm: None,
            potions: 0,
            elixirs: 0,
            quality: "rare".to_string(),
            label: "Socketed Moonforged Cleaver".to_string(),
        };
        let weak = LootDrop {
            gold: 2,
            damage_bonus: 1.0,
            crit_chance: 0.0,
            health_bonus: 0.0,
            armor_bonus: 0.0,
            legendary_power: LegendaryPower::None,
            temper_level: 0,
            socketed_gem: None,
            bonus_gem: None,
            bonus_charm: None,
            potions: 0,
            elixirs: 0,
            quality: "common".to_string(),
            label: "Bent Fang".to_string(),
        };

        assert_eq!(
            loot_priority_label(&socket_sidegrade, equipped, true),
            "NEXT"
        );
        assert_eq!(loot_priority_label(&weak, equipped, true), "LOW");
        assert_eq!(loot_priority_label(&weak, equipped, false), "SALVAGE");
    }

    #[test]
    fn cursed_shrine_blood_price_scales_from_max_health() {
        assert_eq!(cursed_shrine_blood_price(120.0), 24.0);
        assert_eq!(cursed_shrine_blood_price(0.0), 1.0);
    }

    #[test]
    fn item_power_values_defensive_and_critical_affixes() {
        let glass_cannon = ItemStats {
            quality: "rare",
            damage_bonus: 10.0,
            crit_chance: 0.02,
            health_bonus: 0.0,
            armor_bonus: 0.0,
            legendary_power: LegendaryPower::None,
            temper_level: 0,
            socketed_gem: None,
        };
        let balanced = ItemStats {
            quality: "rare",
            damage_bonus: 8.0,
            crit_chance: 0.12,
            health_bonus: 20.0,
            armor_bonus: 12.0,
            legendary_power: LegendaryPower::None,
            temper_level: 0,
            socketed_gem: None,
        };

        assert!(item_power(balanced) > item_power(glass_cannon));
    }

    #[test]
    fn loot_compare_summary_surfaces_upgrade_stash_and_full_inventory_decisions() {
        let equipped = ItemStats {
            quality: "rare",
            damage_bonus: 10.0,
            crit_chance: 0.08,
            health_bonus: 8.0,
            armor_bonus: 8.0,
            legendary_power: LegendaryPower::None,
            temper_level: 0,
            socketed_gem: None,
        };
        let upgrade = ItemStats {
            quality: "legendary",
            damage_bonus: 16.0,
            crit_chance: 0.14,
            health_bonus: 24.0,
            armor_bonus: 18.0,
            legendary_power: LegendaryPower::Stormbrand,
            temper_level: 2,
            socketed_gem: Some(SocketedGem {
                kind: GemKind::Emerald,
                rank: 3,
            }),
        };
        let spare = ItemStats {
            quality: "common",
            damage_bonus: 4.0,
            crit_chance: 0.02,
            health_bonus: 0.0,
            armor_bonus: 2.0,
            legendary_power: LegendaryPower::None,
            temper_level: 0,
            socketed_gem: None,
        };

        let upgrade_summary =
            loot_compare_summary("legendary Stormglass Reaver", upgrade, equipped, true, true);
        assert!(upgrade_summary.contains("Auto-equipped upgrade"));
        assert!(upgrade_summary.contains("BUILD POWER"));
        assert!(upgrade_summary.contains("[CODEX]"));
        assert!(upgrade_summary.contains("power +"));
        assert!(upgrade_summary.contains("dmg +6"));
        assert!(upgrade_summary.contains("crit +6%"));
        assert!(upgrade_summary.contains("new power Stormbrand"));
        assert!(upgrade_summary.contains("chain clear build"));
        assert!(upgrade_summary.contains("socket Emerald 3"));
        assert!(upgrade_summary.contains("temper +2"));
        assert!(upgrade_summary.contains("toughness +26"));

        let spare_summary = loot_compare_summary("common Iron Fang", spare, equipped, true, false);
        assert!(spare_summary.contains("Stashed for build swap"));
        assert!(spare_summary.contains("SALVAGE LATER"));
        assert!(spare_summary.contains("[STASH]"));
        assert!(spare_summary.contains("power -"));
        assert!(spare_summary.contains("dmg -6"));

        let full_summary = loot_compare_summary("common Iron Fang", spare, equipped, false, false);
        assert!(full_summary.contains("Inventory full, kept current"));
        assert!(full_summary.contains("MAKE ROOM"));
        assert!(full_summary.contains("[SALVAGE]"));
        assert!(full_summary.contains("armor -6"));
    }

    #[test]
    fn equipped_item_power_uses_damage_bonus_resource() {
        let equipment = Equipment {
            weapon_name: "rare Keeper Blade".to_string(),
            quality: "rare".to_string(),
            crit_chance: 0.08,
            health_bonus: 8.0,
            armor_bonus: 9.0,
            legendary_power: LegendaryPower::None,
            temper_level: 0,
            socketed_gem: None,
        };
        let damage_bonus = DamageBonus(12.0);

        assert!(equipped_item_power(&equipment, &damage_bonus) > 0.0);
    }

    #[test]
    fn legendary_power_adds_item_value() {
        let mundane = ItemStats {
            quality: "legendary",
            damage_bonus: 16.0,
            crit_chance: 0.14,
            health_bonus: 24.0,
            armor_bonus: 18.0,
            legendary_power: LegendaryPower::None,
            temper_level: 0,
            socketed_gem: None,
        };
        let powered = ItemStats {
            legendary_power: LegendaryPower::Emberbrand,
            ..mundane
        };
        let frost_powered = ItemStats {
            legendary_power: LegendaryPower::Frostbrand,
            ..mundane
        };
        let aegis_powered = ItemStats {
            legendary_power: LegendaryPower::Aegisbrand,
            ..mundane
        };

        assert!(item_power(powered) > item_power(mundane));
        assert!(item_power(frost_powered) > item_power(mundane));
        assert!(item_power(aegis_powered) > item_power(mundane));
    }

    #[test]
    fn ancient_quality_outscores_and_salvages_above_legendary() {
        let legendary = ItemStats {
            quality: "legendary",
            damage_bonus: 16.0,
            crit_chance: 0.14,
            health_bonus: 24.0,
            armor_bonus: 18.0,
            legendary_power: LegendaryPower::Stormbrand,
            temper_level: 0,
            socketed_gem: None,
        };
        let ancient = ItemStats {
            quality: "ancient",
            ..legendary
        };
        let primal = ItemStats {
            quality: "primal",
            ..legendary
        };
        let ancient_item = InventoryItem {
            name: "Ancient Ashen Reliquary Reaver".to_string(),
            quality: "ancient".to_string(),
            damage_bonus: 22.0,
            crit_chance: 0.20,
            health_bonus: 34.0,
            armor_bonus: 28.0,
            legendary_power: LegendaryPower::Stormbrand,
            temper_level: 1,
            socketed_gem: Some(SocketedGem {
                kind: GemKind::Ruby,
                rank: 2,
            }),
        };
        let primal_item = InventoryItem {
            name: "Primal Ashen Reliquary Reaver".to_string(),
            quality: "primal".to_string(),
            damage_bonus: 28.0,
            crit_chance: 0.24,
            health_bonus: 58.0,
            armor_bonus: 46.0,
            legendary_power: LegendaryPower::Stormbrand,
            temper_level: 1,
            socketed_gem: Some(SocketedGem {
                kind: GemKind::Ruby,
                rank: 2,
            }),
        };

        assert!(item_power(ancient) > item_power(legendary));
        assert!(item_power(primal) > item_power(ancient));
        assert!(item_salvage_shards(&ancient_item) > 14);
        assert_eq!(item_salvage_essence(&ancient_item), 3);
        assert!(item_salvage_shards(&primal_item) > item_salvage_shards(&ancient_item));
        assert_eq!(item_salvage_essence(&primal_item), 6);
        assert!(item_sale_value(&ancient_item) > 0);
        assert!(item_sale_value(&primal_item) > item_sale_value(&ancient_item));
    }

    #[test]
    fn socketed_gems_increase_item_value() {
        let plain = ItemStats {
            quality: "rare",
            damage_bonus: 8.0,
            crit_chance: 0.08,
            health_bonus: 8.0,
            armor_bonus: 8.0,
            legendary_power: LegendaryPower::None,
            temper_level: 0,
            socketed_gem: None,
        };
        let jeweled = ItemStats {
            socketed_gem: Some(SocketedGem {
                kind: GemKind::Topaz,
                rank: 2,
            }),
            ..plain
        };

        assert!(item_power(jeweled) > item_power(plain));
    }
}
