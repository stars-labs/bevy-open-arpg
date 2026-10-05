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
    let Some(target) = enemies
        .iter()
        .filter(|entry| matches!(entry.2.id.as_str(), "skeleton" | "ashbone_guard"))
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
    level.level = level.level.max(if visual_case { 6 } else { 3 });
    for (entity, mut transform, mut enemy, mut health) in &mut enemies {
        if entity != target {
            commands.entity(entity).despawn();
            continue;
        }
        let movement = catalog
            .get(selected.0)
            .equipped(&loadout, visual_slot)
            .0
            .movement;
        let distance = if matches!(kind, "cleanse" | "reap")
            || (visual_case && matches!(movement, Movement::Toward(_) | Movement::Leap(_)))
        {
            4.1
        } else {
            1.6
        };
        transform.translation = hero.translation + Vec3::NEG_Z * distance;
        enemy.affixes.clear();
        enemy.move_speed = 0.0;
        enemy.aggro_range = 0.0;
        enemy.attack_damage = 0.0;
        enemy.attack_timer = Timer::from_seconds(1000.0, TimerMode::Once);
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
    player: Query<(&Fury, &Barrier, &PlayerAnimation), With<Player>>,
    enemies: Query<(&Transform, &Health), (With<Enemy>, Without<Player>)>,
) {
    if !case.initialized || case.recorded >= 1800 {
        return;
    }
    let Some(target) = case.target else {
        return;
    };
    let Ok((transform, health)) = enemies.get(target) else {
        return;
    };
    let Ok((fury, barrier, animation)) = player.single() else {
        return;
    };
    if health.current < 500.0 {
        intent.target = None;
    }
    case.recorded += 1;
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
