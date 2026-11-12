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
    ready_elapsed: f32,
    vitality_steps: u8,
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
        ready_elapsed: 0.0,
        vitality_steps: 0,
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
        (With<Player>, Without<summons::Minion>),
    >,
    mut enemies: Query<
        (Entity, &mut Transform, &mut Enemy, &mut Health),
        (With<Enemy>, Without<Player>, Without<summons::Minion>),
    >,
    cameras: Query<(&Camera, &GlobalTransform), With<Camera3d>>,
    catalog: Res<ClassCatalog>,
    selected: Res<SelectedClass>,
    companion_bonds: Query<Entity, With<crate::companion::CompanionBond>>,
    mut summon_talents: Query<&mut summons::SummonTalents, With<Player>>,
    mut minions: Query<
        (&mut summons::SummonVitality, &mut Transform),
        (With<summons::Minion>, Without<Player>),
    >,
) {
    case.elapsed += time.delta_secs();
    // Isolate real player/minion contact from the automatic story companion.
    // Remove its player-side behavior input before fixture enemies are positioned.
    if case.name.starts_with("feel-") || case.name.starts_with("pixelfx-") {
        for entity in &companion_bonds {
            commands
                .entity(entity)
                .remove::<crate::companion::CompanionBond>();
        }
    }
    if case.initialized {
        case.ready_elapsed += time.delta_secs();
        if case.name == "soulbinder-retreat" && case.vitality_steps == 0 {
            let mut count = 0;
            let origin = player
                .single_mut()
                .map(|p| p.0.translation)
                .unwrap_or_default();
            for (mut vitality, mut position) in &mut minions {
                position.translation = origin + Vec3::NEG_Z * 4.0 + Vec3::X * count as f32;
                vitality.current = vitality.max * 0.2;
                count += 1;
            }
            if count > 0 {
                case.vitality_steps = 1;
                info!("DEPTH_FIXTURE retreat health_fraction=0.2");
            }
        }
        if case.name == "soulbinder-rebirth"
            && ((case.vitality_steps == 0 && case.ready_elapsed >= 2.0)
                || (case.vitality_steps == 1 && case.ready_elapsed >= 4.0))
            && let Some((mut vitality, _)) = minions.iter_mut().next()
            && vitality.current > 0.0
        {
            vitality.damage(10000.0);
            case.vitality_steps += 1;
            info!(
                "DEPTH_FIXTURE controller_death step={}",
                case.vitality_steps
            );
        }
        return;
    }
    if case.elapsed < 2.0 {
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
    let feel_case = case.name.starts_with("feel-");
    let pixelfx_case = case.name.starts_with("pixelfx-");
    let soulbinder_case = case.name.starts_with("soulbinder-")
        || case.name == "feel-summon-isolation"
        || case.name == "pixelfx-summon";
    if soulbinder_case {
        if let Ok(mut summon_talents) = summon_talents.single_mut() {
            *summon_talents = summons::SummonTalents::default();
            match case.name.as_str() {
                "soulbinder-aftershock" => summon_talents.spirit = 4,
                "soulbinder-siphon" => summon_talents.spirit = 5,
                "soulbinder-relay" => summon_talents.spirit = 6,
                "soulbinder-retreat" => summon_talents.bone = 4,
                "soulbinder-flank" => summon_talents.bone = 5,
                "soulbinder-rebirth" => summon_talents.bone = 6,
                _ => {}
            }
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
        if matches!(
            case.name.as_str(),
            "soulbinder-sacrifice" | "pixelfx-summon"
        ) {
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
    equipment.crit_chance = if case.name == "feel-critical" {
        1.0
    } else {
        -talents.crit_bonus()
    };
    equipment.legendary_power = LegendaryPower::None;
    damage.0 = 0.0;
    health.current = health.max;
    // Every first-tier skill must be castable; nothing else here reads the level.
    level.level = level.level.max(
        if visual_case || soulbinder_case || feel_case || pixelfx_case {
            6
        } else {
            3
        },
    );
    if matches!(
        case.name.as_str(),
        "soulbinder-tree"
            | "soulbinder-aftershock"
            | "soulbinder-siphon"
            | "soulbinder-relay"
            | "soulbinder-retreat"
            | "soulbinder-flank"
            | "soulbinder-rebirth"
    ) {
        level.level = 20;
        talents.points = if case.name == "soulbinder-tree" {
            10
        } else {
            4
        };
    }
    if pixelfx_case {
        info!(
            "PIXELFX_REVIEW_LOADOUT {} class={:?} level={} area_alt={} finisher_alt={}",
            case.name,
            selected.0,
            level.level,
            loadout.alt[SkillSlot::Area.index()],
            loadout.alt[SkillSlot::Finisher.index()]
        );
    }
    let mut pack_member = 0;
    for (entity, mut transform, mut enemy, mut health) in &mut enemies {
        if matches!(case.name.as_str(), "feel-empty" | "pixelfx-heavy-empty") {
            commands.entity(entity).despawn();
            continue;
        }
        let retained_pack_member = (matches!(case.name.as_str(), "soulbinder-pack" | "feel-multi")
            && pack_member < 2
            || case.name == "soulbinder-relay" && pack_member < 1)
            && entity != target;
        if entity != target && !retained_pack_member {
            commands.entity(entity).despawn();
            continue;
        }
        let movement = catalog
            .get(selected.0)
            .equipped(&loadout, visual_slot)
            .0
            .movement;
        let distance = if matches!(case.name.as_str(), "feel-ranged-hit" | "pixelfx-fireball") {
            5.0
        } else if case.name == "pixelfx-frostnova" {
            2.8
        } else if case.name == "feel-edge-hit" {
            2.1
        } else if soulbinder_case {
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
        health.current = if case.name == "feel-kill" {
            1.0
        } else if case.name == "soulbinder-relay" && entity == target {
            100.0
        } else {
            500.0
        };
        if entity == target
            && let Ok((camera, camera_transform)) = cameras.single()
            && let Ok(point) =
                camera.world_to_viewport(camera_transform, transform.translation + Vec3::Y * 0.6)
        {
            info!(
                "COMBAT_REVIEW_READY {} {:.0} {:.0}",
                case.name, point.x, point.y
            );
        }
    }
    if matches!(case.name.as_str(), "feel-empty" | "pixelfx-heavy-empty") {
        info!("COMBAT_REVIEW_READY {} 1100 130", case.name);
    }
    case.target = Some(target);
    case.initialized = true;
}

#[allow(clippy::type_complexity, clippy::too_many_arguments)]
fn record(
    time: Res<Time<Real>>,
    virtual_time: Res<Time<Virtual>>,
    mut case: ResMut<ReviewCase>,
    mut intent: ResMut<BasicAttackIntent>,
    player: Query<
        (
            &Fury,
            &Barrier,
            &PlayerAnimation,
            &Transform,
            &Health,
            &PlayerLevel,
            &Talents,
            &summons::SummonTalents,
        ),
        With<Player>,
    >,
    pauses: Query<&crate::feedback::ContactPause>,
    cameras: Query<&Transform, (With<Camera3d>, Without<Player>)>,
    heroes: Query<Entity, With<Player>>,
    mut contacts: MessageReader<crate::feedback::HitStopEvent>,
    mut shakes: MessageReader<crate::feedback::ScreenShakeEvent>,
    minions: Query<(Entity, &Transform, &summons::SummonVitality), With<summons::Minion>>,
    enemies: Query<(Entity, &Enemy, &Transform, &Health), (With<Enemy>, Without<Player>)>,
) {
    if !case.initialized || case.recorded >= 1800 {
        return;
    }
    let Some(target) = case.target else {
        return;
    };
    let target_state = enemies.get(target).ok().or_else(|| {
        if case.name.starts_with("feel-") || case.name.starts_with("pixelfx-") {
            None
        } else {
            enemies.iter().next()
        }
    });
    let target_position = target_state.map_or(Vec3::ZERO, |entry| entry.2.translation);
    let target_health = target_state.map_or(0.0, |entry| entry.3.current);
    let Ok((fury, barrier, animation, hero, hero_health, level, talents, tree)) = player.single()
    else {
        return;
    };
    if case.name == "soulbinder-tree" {
        info!(
            "SOULBINDER_TREE level={} points={} bone={} spirit={}",
            level.level, talents.points, tree.bone, tree.spirit
        );
    }
    if target_health < 500.0
        && !case.name.starts_with("feel-")
        && !case.name.starts_with("pixelfx-")
    {
        intent.target = None;
    }
    case.recorded += 1;
    if case.name.starts_with("feel-") || case.name.starts_with("pixelfx-") {
        for event in contacts.read() {
            info!(
                "COMBAT_FEEL_CONTACT {} {:.5} {:?} {} {:.4} {:.4}",
                case.name,
                time.elapsed_secs_f64(),
                event.target,
                event.player_source,
                event.relative_speed,
                event.duration_secs
            );
        }
        for event in shakes.read() {
            info!(
                "COMBAT_FEEL_SHAKE {} {:.5} {:.4} {:.4}",
                case.name,
                time.elapsed_secs_f64(),
                event.intensity,
                event.duration_secs
            );
        }
        let hero_scale = heroes
            .single()
            .ok()
            .and_then(|entity| pauses.get(entity).ok())
            .map_or(1.0, crate::feedback::ContactPause::animation_scale);
        let target_scale = pauses
            .get(target)
            .ok()
            .map_or(1.0, crate::feedback::ContactPause::animation_scale);
        let camera = cameras
            .single()
            .map_or(Vec3::ZERO, |camera| camera.translation);
        for (entity, transform, vitality) in &minions {
            info!(
                "COMBAT_FEEL_MINION {} {:.5} {:?} {:.4} {:.4} {:.3} {:.4}",
                case.name,
                time.elapsed_secs_f64(),
                entity,
                transform.translation.x,
                transform.translation.z,
                vitality.current,
                pauses
                    .get(entity)
                    .map_or(1.0, crate::feedback::ContactPause::animation_scale)
            );
        }
        info!(
            "COMBAT_FEEL_FRAME {} {:.5} {:.5} {:.5} {:.4} {:.4} {:.4} {:.3} {:.3} {:.3} {:.4} {:.4} {:.4} {:.4} {:.4}",
            case.name,
            time.elapsed_secs_f64(),
            virtual_time.relative_speed(),
            animation.attack_progress(),
            hero.translation.x,
            hero.translation.y,
            hero.translation.z,
            hero_health.current,
            target_health,
            fury.current,
            hero_scale,
            target_scale,
            camera.x,
            camera.y,
            camera.z
        );
    }
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
        target_position.x,
        target_position.y,
        target_position.z,
        target_health,
        fury.current,
        barrier.current
    );
}
