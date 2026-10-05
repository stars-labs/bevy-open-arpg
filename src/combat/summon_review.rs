//! Opt-in single-spirit prototype. It does not add a shipping class or save schema.
use super::{summon_logic::*, *};
use crate::player::{summon_limb_transform, summon_weapon_transform};
use bevy::light::{NotShadowCaster, NotShadowReceiver};
use std::collections::HashMap;

#[derive(Resource, Default)]
struct Session {
    bond: Bond,
    attack: Option<(Entity, f32, bool)>,
    cooldown: f32,
    phase: f32,
    moving: bool,
    progress: f32,
    heavy: bool,
    tinted: HashMap<AssetId<StandardMaterial>, Handle<StandardMaterial>>,
}
#[derive(Component)]
struct Spirit;
#[derive(Component)]
struct Part(usize);
#[derive(Component)]
struct Marker;
#[derive(Component)]
struct Flash(f32);
#[derive(Component)]
struct Tinted;
#[derive(Component)]
struct ReviewUi;
#[derive(Resource)]
struct Art {
    arc: Handle<Mesh>,
    cyan: Handle<StandardMaterial>,
    gold: Handle<StandardMaterial>,
}

pub(super) fn install(app: &mut App) {
    if std::env::var("BEVY_OPEN_ARPG_SUMMON_REVIEW").is_err() {
        return;
    }
    app.init_resource::<Session>()
        .add_systems(
            Update,
            (
                spawn.run_if(not(resource_exists::<Art>)),
                contacts,
                behavior,
                visuals,
                tint,
            )
                .chain()
                .after(CombatSet::PlayerDamage)
                .run_if(in_state(GameState::InGame).and_then(not_paused)),
        )
        .add_systems(OnExit(GameState::InGame), clear)
        .add_systems(PostUpdate, trace.run_if(in_state(GameState::InGame)));
}

#[allow(clippy::too_many_arguments)]
fn spawn(
    mut commands: Commands,
    assets: Res<GameAssets>,
    server: Res<AssetServer>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut session: ResMut<Session>,
    player: Query<&Transform, With<Player>>,
) {
    *session = Session::default();
    let Ok(hero) = player.single() else { return };
    let cyan = materials.add(StandardMaterial {
        base_color: Color::srgba(0.25, 0.85, 0.92, 0.8),
        emissive: LinearRgba::new(0.15, 0.9, 1.2, 1.0),
        unlit: true,
        alpha_mode: AlphaMode::Add,
        cull_mode: None,
        ..default()
    });
    let gold = materials.add(StandardMaterial {
        base_color: Color::srgba(0.9, 0.78, 0.4, 0.9),
        emissive: LinearRgba::new(1.4, 0.8, 0.2, 1.0),
        unlit: true,
        alpha_mode: AlphaMode::Add,
        cull_mode: None,
        ..default()
    });
    let arc = meshes.add(skill_vfx_art::arc(2.2, 0.028));
    let ring = meshes.add(skill_vfx_art::arc(std::f32::consts::TAU, 0.012));
    let parts = assets.hero_parts(crate::class::HeroClass::Warden);
    commands
        .spawn((
            Spirit,
            Transform::from_translation(hero.translation + Vec3::new(-1.6, 0.0, 0.9))
                .with_scale(Vec3::splat(0.85)),
            Visibility::default(),
            Name::new("Bound Spirit Prototype"),
        ))
        .with_children(|root| {
            root.spawn((
                WorldAssetRoot(parts.body),
                Transform::default(),
                Name::new("Spirit Torso"),
            ));
            for i in 0..5 {
                let model = match i {
                    0 => parts.mantle.clone(),
                    1 | 2 => parts.arm.clone(),
                    _ => parts.leg.clone(),
                };
                root.spawn((
                    WorldAssetRoot(model),
                    summon_limb_transform(i, 0.0, false, 1.0, false),
                    Part(i),
                ));
            }
            root.spawn((
                WorldAssetRoot(parts.weapon),
                summon_weapon_transform(1.0, false),
                Part(5),
            ));
        });
    commands.spawn((
        Marker,
        Mesh3d(ring.clone()),
        MeshMaterial3d(cyan.clone()),
        Transform::default(),
        Visibility::Hidden,
        NotShadowCaster,
        NotShadowReceiver,
    ));
    commands.spawn((
        ReviewUi,
        Text::new("灵契试炼 · 普攻标记，技能命中后灵体重击"),
        TextFont {
            font: server.load("fonts/NotoSansSC.ttf").into(),
            font_size: FontSize::Px(17.0),
            ..default()
        },
        TextColor(Color::srgb(0.65, 0.92, 0.95)),
        Node {
            position_type: PositionType::Absolute,
            top: Val::Px(64.0),
            left: Val::Px(24.0),
            ..default()
        },
    ));
    commands.insert_resource(Art { arc, cyan, gold });
    info!("SUMMON_REVIEW_SPAWN one_spirit");
}

fn contacts(
    mut events: MessageReader<ContactFeedback>,
    enemies: Query<&Health, With<Enemy>>,
    mut session: ResMut<Session>,
) {
    for contact in events.read() {
        if contact.cast == 0 || !enemies.get(contact.target).is_ok_and(|h| h.current > 0.0) {
            continue;
        }
        let previous = session.bond.target;
        if session
            .bond
            .observe(contact.cast, contact.target, contact.skill.is_some())
        {
            if previous != session.bond.target {
                session.attack = None;
            }
            info!(
                "SUMMON_COMMAND {} cast={} target={:?}",
                if contact.skill.is_some() {
                    "empower"
                } else {
                    "mark"
                },
                contact.cast,
                contact.target
            );
        }
    }
}

#[derive(SystemParam)]
struct Damage<'w, 's> {
    assets: Res<'w, GameAssets>,
    bestiary: Res<'w, Bestiary>,
    enemies: EnemyTargets<'w, 's>,
    combat: MessageWriter<'w, CombatEvent>,
    floating: MessageWriter<'w, FloatingCombatTextEvent>,
    shake: MessageWriter<'w, ScreenShakeEvent>,
    stop: MessageWriter<'w, HitStopEvent>,
}

#[allow(clippy::type_complexity, clippy::too_many_arguments)]
fn behavior(
    time: Res<Time>,
    mut session: ResMut<Session>,
    player: Query<(&Transform, &Health), With<Player>>,
    mut spirit: Query<&mut Transform, (With<Spirit>, Without<Player>, Without<Enemy>)>,
    mut damage: Damage,
    mut commands: Commands,
    art: Res<Art>,
) {
    let Ok((hero, health)) = player.single() else {
        return;
    };
    let Ok(mut spirit) = spirit.single_mut() else {
        return;
    };
    let target_position = session.bond.target.and_then(|target| {
        damage
            .enemies
            .get(target)
            .ok()
            .filter(|e| e.3.current > 0.0)
            .map(|e| e.1.translation)
    });
    let previous_target = session.bond.target;
    let valid =
        health.current > 0.0 && target_position.is_some_and(|p| within_leash(hero.translation, p));
    session.bond.tick(time.delta_secs(), valid);
    if previous_target.is_some() && session.bond.target.is_none() {
        info!(
            "SUMMON_CLEAR invalid={} hero_alive={}",
            !valid,
            health.current > 0.0
        );
    }
    if session.bond.target.is_none() {
        session.attack = None;
    }
    session.cooldown = (session.cooldown - time.delta_secs()).max(0.0);
    let goal = if session.bond.target.is_some() {
        target_position.unwrap_or(hero.translation)
    } else {
        hero.translation + Vec3::new(-1.6, 0.0, 0.9)
    };
    let distance = Vec2::new(goal.x - spirit.translation.x, goal.z - spirit.translation.z).length();
    let stop = if session.bond.target.is_some() {
        REACH * 0.85
    } else {
        0.15
    };
    session.moving = distance > stop && session.attack.is_none();
    if session.moving {
        spirit.translation = step_toward(
            spirit.translation,
            goal,
            (5.8 * time.delta_secs()).min(distance - stop),
        );
        session.phase += time.delta_secs() * 12.0;
    }
    let facing = Vec3::new(
        goal.x - spirit.translation.x,
        0.0,
        goal.z - spirit.translation.z,
    )
    .normalize_or_zero();
    if facing.length_squared() > 0.0 {
        spirit.rotation = Quat::from_rotation_y(facing.x.atan2(facing.z));
    }
    if session.attack.is_none()
        && session.cooldown == 0.0
        && distance <= REACH
        && let Some(target) = session.bond.target
    {
        session.attack = Some((target, 0.0, session.bond.empowered));
    }
    session.progress = 1.0;
    if let Some((target, before, heavy)) = session.attack {
        session.heavy = heavy;
        let duration = if heavy { HEAVY_WINDUP } else { WINDUP };
        let after = before + time.delta_secs();
        session.progress = (after / duration).min(1.0);
        if contact_crossed(before, after, heavy) {
            session.progress = if heavy { HEAVY_CONTACT } else { CONTACT };
            let still_in_reach = damage.enemies.get(target).is_ok_and(|e| {
                e.3.current > 0.0 && e.1.translation.distance(spirit.translation) <= REACH
            });
            if still_in_reach {
                let empowered = heavy && session.bond.empowered;
                let before_health = damage
                    .enemies
                    .get(target)
                    .map(|e| e.3.current)
                    .unwrap_or(0.0);
                let base_damage = if empowered {
                    session.bond.consume()
                } else {
                    9.0
                };
                let source = spirit.translation;
                let Damage {
                    assets,
                    bestiary,
                    enemies,
                    combat,
                    floating,
                    shake,
                    stop,
                } = &mut damage;
                let result = apply_strike(
                    &mut commands,
                    &mut AttackFeedback {
                        combat_events: combat,
                        floating_text: floating,
                        screen_shake: shake,
                        hit_stop: stop,
                    },
                    &combat_vfx_assets(assets),
                    Strike {
                        area: HitArea::Target(target),
                        source,
                        profile: StrikeProfile {
                            damage: base_damage,
                            impact_scale: if empowered { 1.4 } else { 0.6 },
                            crit_chance: 0.0,
                            legendary_powers: LegendaryPowerSet::new(
                                LegendaryPower::None,
                                LegendaryPower::None,
                            ),
                            skill: None,
                            hit_stop: false,
                            long_shot: None,
                            cast: 0,
                            fury_bonus: 0.0,
                            barrier_reward: 0.0,
                        },
                    },
                    enemies,
                    bestiary,
                );
                let blade = source + Vec3::Y * 0.7 + facing * 0.5;
                commands.spawn((
                    Flash(0.16),
                    Mesh3d(art.arc.clone()),
                    MeshMaterial3d(if empowered {
                        art.gold.clone()
                    } else {
                        art.cyan.clone()
                    }),
                    Transform::from_translation(blade)
                        .with_rotation(spirit.rotation)
                        .with_scale(Vec3::splat(0.9)),
                    NotShadowCaster,
                    NotShadowReceiver,
                ));
                let dealt = before_health
                    - enemies
                        .get(target)
                        .map(|e| e.3.current)
                        .unwrap_or(before_health);
                info!(
                    "SUMMON_HIT target={target:?} empowered={empowered} base={base_damage:.1} dealt={dealt:.3} hits={} contact={after:.4} pose={:.2} x={:.3} z={:.3}",
                    result.damaging_hits, session.progress, source.x, source.z
                );
            } else {
                info!("SUMMON_MISS target={target:?}");
            }
        }
        if after >= duration {
            session.attack = None;
            session.cooldown = 0.65;
        } else {
            session.attack = Some((target, after, heavy));
        }
    }
}

#[allow(clippy::type_complexity, clippy::too_many_arguments)]
fn visuals(
    time: Res<Time>,
    session: Res<Session>,
    enemies: Query<&Transform, With<Enemy>>,
    art: Res<Art>,
    mut parts: Query<(&Part, &mut Transform), (Without<Enemy>, Without<Marker>, Without<Flash>)>,
    mut marker: Query<
        (
            &mut Transform,
            &mut Visibility,
            &mut MeshMaterial3d<StandardMaterial>,
        ),
        (With<Marker>, Without<Enemy>, Without<Part>, Without<Flash>),
    >,
    mut flashes: Query<
        (Entity, &mut Flash, &mut Transform),
        (Without<Part>, Without<Enemy>, Without<Marker>),
    >,
    mut commands: Commands,
) {
    for (part, mut transform) in &mut parts {
        *transform = if part.0 == 5 {
            summon_weapon_transform(session.progress, session.heavy)
        } else {
            summon_limb_transform(
                part.0,
                session.phase,
                session.moving,
                session.progress,
                session.heavy,
            )
        };
    }
    for (mut transform, mut visibility, mut material) in &mut marker {
        if let Some(target) = session.bond.target
            && let Ok(target) = enemies.get(target)
        {
            *visibility = Visibility::Visible;
            transform.translation = target.translation + Vec3::Y * 0.035;
            transform.scale = Vec3::splat(if session.bond.empowered { 0.62 } else { 0.46 });
            material.0 = if session.bond.empowered {
                art.gold.clone()
            } else {
                art.cyan.clone()
            };
        } else {
            *visibility = Visibility::Hidden;
        }
    }
    for (entity, mut flash, mut transform) in &mut flashes {
        flash.0 -= time.delta_secs();
        transform.scale *= (1.0 - time.delta_secs() * 4.0).max(0.0);
        if flash.0 <= 0.0 {
            commands.entity(entity).despawn();
        }
    }
}

fn tint(
    mut commands: Commands,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut session: ResMut<Session>,
    children: Query<&ChildOf>,
    spirits: Query<(), With<Spirit>>,
    meshes: Query<(Entity, &MeshMaterial3d<StandardMaterial>), Without<Tinted>>,
) {
    for (entity, material) in &meshes {
        let mut ancestor = entity;
        let mut is_spirit = false;
        for _ in 0..24 {
            if spirits.contains(ancestor) {
                is_spirit = true;
                break;
            }
            let Ok(parent) = children.get(ancestor) else {
                break;
            };
            ancestor = parent.parent();
        }
        if !is_spirit {
            continue;
        }
        let id = material.0.id();
        let handle = if let Some(handle) = session.tinted.get(&id) {
            handle.clone()
        } else if let Some(original) = materials.get(&material.0) {
            let mut copy = original.clone();
            copy.base_color = Color::srgba(0.25, 0.75, 0.85, 0.58);
            copy.base_color_texture = None;
            copy.emissive = LinearRgba::new(0.06, 0.3, 0.4, 1.0);
            copy.alpha_mode = AlphaMode::Blend;
            let handle = materials.add(copy);
            session.tinted.insert(id, handle.clone());
            handle
        } else {
            continue;
        };
        commands.entity(entity).insert((
            MeshMaterial3d(handle),
            Tinted,
            NotShadowCaster,
            NotShadowReceiver,
        ));
    }
}
#[allow(clippy::type_complexity)]
fn clear(
    mut commands: Commands,
    roots: Query<Entity, Or<(With<Spirit>, With<Marker>, With<Flash>, With<ReviewUi>)>>,
) {
    for entity in &roots {
        commands.entity(entity).despawn();
    }
    commands.remove_resource::<Art>();
}

fn trace(
    time: Res<Time<Real>>,
    virtual_time: Res<Time<Virtual>>,
    session: Res<Session>,
    spirit: Query<&Transform, With<Spirit>>,
) {
    let Ok(spirit) = spirit.single() else { return };
    info!(
        "SUMMON_FRAME {:.3} {:.3} {:.4} {:.3} {:.3} {:.3} {}",
        time.elapsed_secs(),
        virtual_time.elapsed_secs(),
        session.progress,
        spirit.translation.x,
        spirit.translation.z,
        session.bond.remaining,
        virtual_time.is_paused()
    );
}
