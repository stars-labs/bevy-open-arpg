//! Explicit native/dev-only combat comparison fixture. Never enabled in normal play.
use super::*;
use crate::player::{DamageBonus, PlayerLevel};

#[derive(Resource)]
struct ReviewCase {
    name: String,
    elapsed: f32,
    initialized: bool,
    target: Option<Entity>,
    recorded: u32,
    boss_requested: bool,
}

pub(super) fn install(app: &mut App) {
    let Ok(name) = std::env::var("BEVY_OPEN_ARPG_COMBAT_REVIEW") else {
        return;
    };
    app.insert_resource(ReviewCase {
        name,
        elapsed: 0.0,
        initialized: false,
        target: None,
        recorded: 0,
        boss_requested: false,
    })
    .add_systems(
        Update,
        prepare
            .after(CombatSet::PlayerDamage)
            .run_if(in_state(GameState::InGame)),
    )
    .add_systems(PostUpdate, record.run_if(in_state(GameState::InGame)));
}

#[allow(clippy::type_complexity, clippy::too_many_arguments)]
fn prepare(
    mut commands: Commands,
    time: Res<Time<Real>>,
    mut case: ResMut<ReviewCase>,
    mut player: Query<
        (
            &Transform,
            &mut Talents,
            &mut SkillLoadout,
            &mut Fury,
            &mut Barrier,
            &mut Equipment,
            &mut DamageBonus,
            &mut Health,
            &mut PlayerLevel,
        ),
        With<Player>,
    >,
    mut enemies: Query<
        (Entity, &mut Transform, &mut Enemy, &mut Health),
        (With<Enemy>, Without<Player>),
    >,
    cameras: Query<(&Camera, &GlobalTransform), With<Camera3d>>,
    catalog: Res<ClassCatalog>,
    selected: Res<SelectedClass>,
    mut summon_talents: Query<&mut summons::SummonTalents, With<Player>>,
) {
    case.elapsed += time.delta_secs();
    if case.elapsed < 2.0 || case.initialized {
        return;
    }
    let Ok((
        hero,
        mut talents,
        mut loadout,
        mut fury,
        mut barrier,
        mut equipment,
        mut damage,
        mut health,
        mut level,
    )) = player.single_mut()
    else {
        return;
    };
    if case.name == "soulbinder-boss" && !enemies.iter().any(|entry| entry.2.id == "keeper") {
        if !case.boss_requested {
            commands.write_message(crate::chapter::SpawnBoss);
            case.boss_requested = true;
        }
        return;
    }
    let Some(target) = enemies
        .iter()
        .filter(|entry| {
            let wanted = if case.name == "soulbinder-boss" {
                "keeper"
            } else if case.name == "soulbinder-projectile" {
                "cultist"
            } else {
                "skeleton"
            };
            entry.2.id == wanted || (wanted == "skeleton" && entry.2.id == "ashbone_guard")
        })
        .min_by(|a, b| {
            a.1.translation
                .distance_squared(hero.translation)
                .total_cmp(&b.1.translation.distance_squared(hero.translation))
        })
        .map(|entry| entry.0)
    else {
        return;
    };
    *talents = Talents::default();
    *loadout = SkillLoadout::default();
    let (kind, value) = case.name.split_once(':').unwrap_or((&case.name, "0"));
    let rank = value.parse::<u32>().unwrap_or(0).min(8);
    match kind {
        "wrath" => talents.wrath = rank,
        "vigor" => talents.vigor = rank,
        "focus" => talents.focus = rank,
        "blood" => loadout.toggle_rune(SkillSlot::Finisher),
        "reap" => loadout.toggle_rune(SkillSlot::Mobility),
        "frost" => loadout.toggle_rune(SkillSlot::Area),
        _ => {}
    }
    let soulbinder_case = case.name.starts_with("soulbinder-");
    if soulbinder_case {
        if let Ok(mut summon_talents) = summon_talents.single_mut() {
            *summon_talents = summons::SummonTalents::default();
            if case.name == "soulbinder-bone" {
                summon_talents.bone = 3;
            }
            if case.name == "soulbinder-spirit" {
                summon_talents.spirit = 3;
            }
            if case.name == "soulbinder-resonance" {
                summon_talents.spirit = 1;
            }
        }
        if case.name == "soulbinder-exchange" {
            loadout.alt[SkillSlot::Mobility.index()] = true;
        }
        if case.name == "soulbinder-bulwark" {
            loadout.alt[SkillSlot::Area.index()] = true;
        }
        if case.name == "soulbinder-sacrifice" {
            loadout.alt[SkillSlot::Finisher.index()] = true;
        }
    }
    let active_enemy = matches!(
        case.name.as_str(),
        "soulbinder-melee" | "soulbinder-projectile" | "soulbinder-boss" | "soulbinder-pack"
    );
    let visual_case = case.name.starts_with("vfx-");
    let parts: Vec<_> = case.name.split('-').collect();
    let visual_slot = match parts.get(2).copied() {
        Some("q") => SkillSlot::Mobility,
        Some("e") => SkillSlot::Area,
        _ => SkillSlot::Finisher,
    };
    if visual_case {
        loadout.alt[visual_slot.index()] = parts.get(3) == Some(&"b");
        if parts.get(4) == Some(&"alt") {
            loadout.toggle_rune(visual_slot);
        }
    }
    fury.current = if kind == "focus" { 0.0 } else { 100.0 };
    barrier.current = 0.0;
    equipment.crit_chance = -talents.crit_bonus();
    equipment.legendary_power = LegendaryPower::None;
    damage.0 = 0.0;
    health.current = health.max;
    // Every first-tier skill must be castable; nothing else here reads the level.
    level.level = level
        .level
        .max(if visual_case || soulbinder_case { 6 } else { 3 });
    let mut pack_member = 0;
    for (entity, mut transform, mut enemy, mut health) in &mut enemies {
        let retained_pack_member =
            case.name == "soulbinder-pack" && entity != target && pack_member < 2;
        if entity != target && !retained_pack_member {
            commands.entity(entity).despawn();
            continue;
        }
        let movement = catalog
            .get(selected.0)
            .equipped(&loadout, visual_slot)
            .0
            .movement;
        let distance = if soulbinder_case {
            4.0
        } else if matches!(kind, "cleanse" | "reap")
            || (visual_case && matches!(movement, Movement::Toward(_) | Movement::Leap(_)))
        {
            4.1
        } else {
            1.6
        };
        let flank = if retained_pack_member {
            pack_member += 1;
            if pack_member == 1 { -1.0 } else { 1.0 }
        } else {
            0.0
        };
        transform.translation = hero.translation + Vec3::NEG_Z * distance + Vec3::X * flank;
        enemy.affixes.clear();
        if active_enemy {
            enemy.move_speed = 1.0;
            enemy.aggro_range = 12.0;
            enemy.attack_damage = 16.0;
            enemy.attack_timer = Timer::from_seconds(1.0, TimerMode::Once);
        } else {
            enemy.move_speed = 0.0;
            enemy.aggro_range = 0.0;
            enemy.attack_damage = 0.0;
            enemy.attack_timer = Timer::from_seconds(1000.0, TimerMode::Once);
        }
        health.max = 500.0;
        health.current = 500.0;
        if let Ok((camera, camera_transform)) = cameras.single()
            && let Ok(point) =
                camera.world_to_viewport(camera_transform, transform.translation + Vec3::Y * 0.6)
        {
            info!(
                "COMBAT_REVIEW_READY {} {:.0} {:.0}",
                case.name, point.x, point.y
            );
        }
    }
    case.target = Some(target);
    case.initialized = true;
}

#[allow(clippy::type_complexity)]
fn record(
    time: Res<Time<Real>>,
    virtual_time: Res<Time<Virtual>>,
    mut case: ResMut<ReviewCase>,
    mut intent: ResMut<BasicAttackIntent>,
    player: Query<(&Fury, &Barrier, &PlayerAnimation, &Transform, &Health), With<Player>>,
    minions: Query<(Entity, &Transform, &summons::SummonVitality), With<summons::Minion>>,
    enemies: Query<(Entity, &Enemy, &Transform, &Health), (With<Enemy>, Without<Player>)>,
) {
    if !case.initialized || case.recorded >= 1800 {
        return;
    }
    let Some(target) = case.target else {
        return;
    };
    let Ok((_, _, transform, health)) = enemies.get(target) else {
        return;
    };
    let Ok((fury, barrier, animation, hero, hero_health)) = player.single() else {
        return;
    };
    if health.current < 500.0 {
        intent.target = None;
    }
    case.recorded += 1;
    if case.name.starts_with("soulbinder-") {
        for (entity, enemy, _, health) in &enemies {
            info!(
                "SOULBINDER_ENEMY {} {:.5} {:?} {} {:.3}",
                case.name,
                time.elapsed_secs_f64(),
                entity,
                enemy.id,
                health.current
            );
        }
        info!(
            "SOULBINDER_HERO {} {:.5} {:.4} {:.4} {:.3}",
            case.name,
            time.elapsed_secs_f64(),
            hero.translation.x,
            hero.translation.z,
            hero_health.current
        );
        for (entity, transform, vitality) in &minions {
            info!(
                "SOULBINDER_MINION {} {:.5} {:?} {:.4} {:.4} {:.3} {:.3} {:.3}",
                case.name,
                time.elapsed_secs_f64(),
                entity,
                transform.translation.x,
                transform.translation.z,
                vitality.current,
                vitality.max,
                vitality.dead_secs
            );
        }
    }
    info!(
        "COMBAT_REVIEW_FRAME {} {:.5} {:.5} {:.5} {:.4} {:.4} {:.4} {:.3} {:.3} {:.3}",
        case.name,
        time.elapsed_secs_f64(),
        virtual_time.relative_speed(),
        animation.attack_progress(),
        transform.translation.x,
        transform.translation.y,
        transform.translation.z,
        health.current,
        fury.current,
        barrier.current
    );
}
