//! Shipping Soulbinder controller: mark, command, contact and minion survival.
use super::{summon_logic::*, *};
use crate::class::{HeroClass, Rune, SelectedClass, SummonCommand};
use crate::player::{summon_limb_transform, summon_weapon_transform};
use bevy::light::{NotShadowCaster, NotShadowReceiver};
use serde::{Deserialize, Serialize};

#[derive(Component, Default, Clone, Copy, Debug, Serialize, Deserialize, PartialEq)]
pub(crate) struct SummonTalents {
    pub bone: u32,
    pub spirit: u32,
}
#[derive(Component)]
pub(crate) struct Minion;
#[derive(Component, Debug)]
pub(crate) struct SummonVitality {
    pub current: f32,
    pub max: f32,
    pub dead_secs: f32,
    pub guard_secs: f32,
    pub guard_multiplier: f32,
}
#[derive(Message, Clone, Copy, Debug)]
pub(crate) struct SummonOrder {
    pub command: SummonCommand,
    pub rune: Rune,
    pub damage: f32,
    pub impact: f32,
}
#[derive(Resource, Deserialize)]
struct Tuning {
    spirit_health: f32,
    bone_health: f32,
    spirit_damage: f32,
    bone_damage: f32,
    attack_cooldown: f32,
    speed: f32,
    revive_secs: f32,
    overdrive_secs: f32,
    sacrifice_radius: f32,
    recall_heal_fraction: f32,
    recall_reap_heal: f32,
    bulwark_barrier: f32,
    bulwark_heal: f32,
    bulwark_secs: f32,
    guard_damage_multiplier: f32,
    overdrive_damage_multiplier: f32,
    overdrive_attack_speed: f32,
    overdrive_extra_secs_per_rank: f32,
    wall_barrier: f32,
    wall_cooldown: f32,
    wall_guard_secs: f32,
    intercept_radius: f32,
    intercept_mark_secs: f32,
    echo_damage_fraction: f32,
    heavy_damage_multiplier: f32,
    overdrive_cleave_radius: f32,
    volley_radius: f32,
    bone_regen_per_sec: f32,
    level_damage_growth: f32,
    level_health_growth: f32,
    gear_damage_share: f32,
    heavy_impact: f32,
    basic_impact: f32,
    echo_impact: f32,
}
#[derive(Resource, Default)]
pub(crate) struct SummonStatus {
    pub marked: Option<Entity>,
    bond: Bond,
    overdrive: f32,
    guard: f32,
    rune: Option<Rune>,
    basics: u32,
    wall_cooldown: f32,
    transition_health: Option<f32>,
    transition_dead_secs: f32,
}
#[derive(Component)]
struct State {
    slot: usize,
    bone: bool,
    attack: Option<(Entity, f32, bool)>,
    cooldown: f32,
    phase: f32,
    moving: bool,
    progress: f32,
    heavy: bool,
    volley: f32,
    rune: Rune,
}
#[derive(Component)]
struct Part(usize);
#[derive(Component)]
struct Marker;
#[derive(Component)]
struct Aura;
#[derive(Component)]
struct Flash(f32);
#[derive(Resource)]
struct Art {
    arc: Handle<Mesh>,
    ring: Handle<Mesh>,
    cyan: Handle<StandardMaterial>,
    gold: Handle<StandardMaterial>,
}

pub(crate) fn install(app: &mut App) {
    app.insert_resource(
        ron::from_str::<Tuning>(include_str!("../../assets/data/summons.ron"))
            .expect("summon tuning"),
    )
    .init_resource::<SummonStatus>()
    .add_message::<SummonOrder>()
    .add_systems(Startup, (art, spawn_hud))
    .add_systems(
        Update,
        (reconcile, contacts, orders, behavior, visuals, auras)
            .chain()
            .after(CombatSet::PlayerDamage)
            .run_if(in_state(GameState::InGame).and_then(not_paused)),
    )
    .add_systems(Update, update_hud)
    .add_systems(OnExit(GameState::InGame), clear);
}
fn art(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let material = |color, emissive| StandardMaterial {
        base_color: color,
        emissive,
        unlit: true,
        alpha_mode: AlphaMode::Add,
        cull_mode: None,
        ..default()
    };
    commands.insert_resource(Art {
        arc: meshes.add(skill_vfx_art::arc(2.2, 0.035)),
        ring: meshes.add(skill_vfx_art::arc(std::f32::consts::TAU, 0.018)),
        cyan: materials.add(material(
            Color::srgba(0.2, 0.85, 0.95, 0.8),
            LinearRgba::new(0.1, 0.8, 1.1, 1.0),
        )),
        gold: materials.add(material(
            Color::srgba(0.9, 0.7, 0.3, 0.9),
            LinearRgba::new(1.2, 0.7, 0.15, 1.0),
        )),
    });
}
#[allow(clippy::too_many_arguments)]
fn reconcile(
    mut commands: Commands,
    selected: Res<SelectedClass>,
    assets: Res<GameAssets>,
    tuning: Res<Tuning>,
    player: Query<(&Transform, &SummonTalents), With<Player>>,
    minions: Query<(Entity, &State, &SummonVitality), With<Minion>>,
    markers: Query<Entity, With<Marker>>,
    art: Res<Art>,
    mut session: ResMut<SummonStatus>,
) {
    if selected.0 != HeroClass::Soulbinder {
        for (entity, _, _) in &minions {
            commands.entity(entity).despawn();
        }
        for entity in &markers {
            commands.entity(entity).despawn();
        }
        *session = SummonStatus::default();
        return;
    }
    let Ok((hero, talents)) = player.single() else {
        return;
    };
    let bone = talents.bone > 0;
    let count = if bone { 2 } else { 1 };
    if minions.iter().any(|(_, state, _)| state.bone != bone) {
        let fractions: Vec<_> = minions
            .iter()
            .map(|(_, _, health)| (health.current / health.max).clamp(0.0, 1.0))
            .collect();
        session.transition_health =
            Some(fractions.iter().sum::<f32>() / fractions.len().max(1) as f32);
        session.transition_dead_secs = minions
            .iter()
            .map(|(_, _, v)| v.dead_secs)
            .fold(0.0, f32::max);
        for (entity, _, _) in &minions {
            commands.entity(entity).despawn();
        }
        return;
    }
    for slot in 0..count {
        if minions.iter().any(|(_, state, _)| state.slot == slot) {
            continue;
        }
        let parts = if bone {
            assets.bone_guard_parts()
        } else {
            assets.spirit_parts()
        };
        let health = if bone {
            tuning.bone_health
        } else {
            tuning.spirit_health
        };
        commands
            .spawn((
                Minion,
                State {
                    slot,
                    bone,
                    attack: None,
                    cooldown: 0.0,
                    phase: 0.0,
                    moving: false,
                    progress: 1.0,
                    heavy: false,
                    volley: 0.0,
                    rune: Rune::Expose,
                },
                SummonVitality {
                    current: health * session.transition_health.unwrap_or(1.0),
                    max: health,
                    dead_secs: session.transition_dead_secs,
                    guard_secs: 0.0,
                    guard_multiplier: tuning.guard_damage_multiplier,
                },
                Transform::from_translation(hero.translation + formation(slot, bone)),
                Visibility::default(),
                Name::new(if bone { "契灵骨卫" } else { "缚魂契灵" }),
            ))
            .with_children(|root| {
                root.spawn((WorldAssetRoot(parts.body), Transform::default()));
                root.spawn((
                    Aura,
                    Mesh3d(art.ring.clone()),
                    MeshMaterial3d(art.cyan.clone()),
                    Transform::from_xyz(0.0, 0.04, 0.0).with_scale(Vec3::splat(0.7)),
                    Visibility::Hidden,
                    NotShadowCaster,
                    NotShadowReceiver,
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
    }
    if markers.is_empty() {
        commands.spawn((
            Marker,
            Mesh3d(art.ring.clone()),
            MeshMaterial3d(art.cyan.clone()),
            Transform::default(),
            Visibility::Hidden,
            NotShadowCaster,
            NotShadowReceiver,
        ));
    }
    session.transition_health = None;
    session.transition_dead_secs = 0.0;
}
fn formation(slot: usize, bone: bool) -> Vec3 {
    Vec3::new(
        if bone {
            if slot == 0 { -1.25 } else { 1.25 }
        } else {
            -1.5
        },
        0.0,
        0.85,
    )
}
fn contacts(
    mut events: MessageReader<ContactFeedback>,
    enemies: Query<&Health, With<Enemy>>,
    selected: Res<SelectedClass>,
    player: Query<&SummonTalents, With<Player>>,
    mut session: ResMut<SummonStatus>,
) {
    for event in events.read() {
        if selected.0 == HeroClass::Soulbinder
            && event.cast != 0
            && enemies.get(event.target).is_ok_and(|h| h.current > 0.0)
            && session
                .bond
                .observe(event.cast, event.target, event.skill.is_some())
            && event.skill.is_none()
        {
            session.basics += 1;
            if session.basics.is_multiple_of(3) && player.single().is_ok_and(|t| t.spirit >= 1) {
                session.bond.empowered = true;
                info!("SUMMON_RESONANCE basics={}", session.basics);
            }
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
impl Damage<'_, '_> {
    fn strike(
        &mut self,
        commands: &mut Commands,
        source: Vec3,
        area: HitArea,
        amount: f32,
        impact: f32,
        rune: Option<Rune>,
    ) {
        apply_strike(
            commands,
            &mut AttackFeedback {
                combat_events: &mut self.combat,
                floating_text: &mut self.floating,
                screen_shake: &mut self.shake,
                hit_stop: &mut self.stop,
            },
            &combat_vfx_assets(&self.assets),
            Strike {
                area,
                source,
                profile: StrikeProfile {
                    damage: amount,
                    impact_scale: impact,
                    crit_chance: 0.0,
                    legendary_powers: LegendaryPowerSet::new(
                        LegendaryPower::None,
                        LegendaryPower::None,
                    ),
                    skill: rune.map(|rune| SkillCast {
                        slot: crate::class::SkillSlot::Area,
                        rune,
                    }),
                    hit_stop: impact >= 1.2,
                    long_shot: None,
                    cast: 0,
                    fury_bonus: 0.0,
                    barrier_reward: 0.0,
                },
            },
            &mut self.enemies,
            &self.bestiary,
        );
    }
}
#[allow(clippy::too_many_arguments, clippy::type_complexity)]
fn orders(
    layout: Res<crate::dungeon::DungeonLayout>,
    mut events: MessageReader<SummonOrder>,
    selected: Res<SelectedClass>,
    mut session: ResMut<SummonStatus>,
    tuning: Res<Tuning>,
    mut player: Query<
        (&mut Transform, &mut Health, &mut Barrier, &SummonTalents),
        (With<Player>, Without<Minion>, Without<Enemy>),
    >,
    mut minions: Query<
        (&mut Transform, &mut SummonVitality, &mut State),
        (With<Minion>, Without<Player>, Without<Enemy>),
    >,
    mut damage: Damage,
    mut commands: Commands,
    art: Res<Art>,
) {
    for order in events.read() {
        if selected.0 != HeroClass::Soulbinder {
            continue;
        }
        let Ok((mut hero, mut health, mut barrier, talents)) = player.single_mut() else {
            continue;
        };
        if health.current <= 0.0 {
            continue;
        }
        session.rune = Some(order.rune);
        match order.command {
            SummonCommand::Recall => {
                if order.rune == Rune::Reap {
                    health.current = (health.current + tuning.recall_reap_heal).min(health.max);
                }
                session.bond = Bond::default();
                for (mut position, mut vitality, mut state) in &mut minions {
                    if vitality.current <= 0.0 {
                        continue;
                    }
                    let previous = position.translation;
                    flash(&mut commands, &art, previous, 0.65, false);
                    position.translation = hero.translation + formation(state.slot, state.bone);
                    vitality.current = (vitality.current
                        + vitality.max * tuning.recall_heal_fraction)
                        .min(vitality.max);
                    state.attack = None;
                    state.volley = 0.0;
                    flash(&mut commands, &art, position.translation, 1.1, false);
                }
            }
            SummonCommand::Exchange => {
                if let Some((mut position, _, mut state)) =
                    minions.iter_mut().find(|(_, v, _)| v.current > 0.0)
                {
                    commands.queue(|world: &mut World| {
                        crate::player::cancel_pointer_movement(world);
                        if let Some(mut intent) = world.get_resource_mut::<BasicAttackIntent>() {
                            intent.target = None;
                        }
                    });
                    let old = hero.translation;
                    hero.translation =
                        crate::player::clamp_player_translation(&layout, position.translation);
                    position.translation = crate::player::clamp_player_translation(&layout, old);
                    flash(&mut commands, &art, old, 0.7, true);
                    state.attack = None;
                    flash(&mut commands, &art, hero.translation, 1.2, false);
                }
            }
            SummonCommand::Volley => {
                for (_, vitality, mut state) in &mut minions {
                    if vitality.current > 0.0 {
                        state.volley = order.damage;
                        state.rune = order.rune;
                        state.cooldown = 0.0;
                        state.attack = None;
                    }
                }
                session.bond.empowered = true;
            }
            SummonCommand::Bulwark => {
                session.guard = tuning.bulwark_secs;
                barrier.current = (barrier.current + tuning.bulwark_barrier).min(barrier.max);
                session.bond = Bond::default();
                for (mut position, mut vitality, mut state) in &mut minions {
                    if vitality.current > 0.0 {
                        position.translation = hero.translation + formation(state.slot, state.bone);
                        vitality.current =
                            (vitality.current + tuning.bulwark_heal).min(vitality.max);
                        vitality.guard_secs = tuning.bulwark_secs;
                        state.attack = None;
                    }
                }
                flash(&mut commands, &art, hero.translation, 2.0, false);
            }
            SummonCommand::Overdrive => {
                session.overdrive = tuning.overdrive_secs
                    + talents.spirit.min(3) as f32 * tuning.overdrive_extra_secs_per_rank;
                for (position, vitality, mut state) in &mut minions {
                    if vitality.current > 0.0 {
                        state.cooldown = 0.0;
                        flash(&mut commands, &art, position.translation, 1.4, true);
                    }
                }
            }
            SummonCommand::Sacrifice => {
                for (position, mut vitality, mut state) in &mut minions {
                    if vitality.current <= 0.0 {
                        continue;
                    }
                    damage.strike(
                        &mut commands,
                        position.translation,
                        HitArea::Radial {
                            center: position.translation,
                            radius: tuning.sacrifice_radius,
                        },
                        order.damage,
                        order.impact,
                        Some(order.rune),
                    );
                    flash(
                        &mut commands,
                        &art,
                        position.translation,
                        tuning.sacrifice_radius,
                        true,
                    );
                    vitality.current = 0.0;
                    vitality.dead_secs = tuning.revive_secs;
                    state.attack = None;
                    break;
                }
                session.bond = Bond::default();
            }
        }
        info!(
            "SUMMON_ORDER {:?} bone={} spirit={}",
            order.command, talents.bone, talents.spirit
        );
    }
}
// Exchanges remain on the existing navigable positions of the two actors.
#[allow(clippy::too_many_arguments, clippy::type_complexity)]
fn behavior(
    layout: Res<crate::dungeon::DungeonLayout>,
    time: Res<Time>,
    mut session: ResMut<SummonStatus>,
    tuning: Res<Tuning>,
    mut player: Query<
        (
            &Transform,
            &Health,
            &SummonTalents,
            &DamageBonus,
            &PlayerLevel,
            &mut Barrier,
        ),
        (With<Player>, Without<Minion>),
    >,
    mut minions: Query<
        (
            &mut Transform,
            &mut SummonVitality,
            &mut State,
            &mut Visibility,
        ),
        (With<Minion>, Without<Player>, Without<Enemy>),
    >,
    mut damage: Damage,
    mut commands: Commands,
    art: Res<Art>,
) {
    let Ok((hero, health, talents, bonus, level, mut barrier)) = player.single_mut() else {
        return;
    };
    let dt = time.delta_secs();
    session.overdrive = (session.overdrive - dt).max(0.0);
    session.guard = (session.guard - dt).max(0.0);
    session.wall_cooldown = (session.wall_cooldown - dt).max(0.0);
    if talents.bone >= 2
        && session.bond.target.is_none()
        && let Some((target, ..)) = damage
            .enemies
            .iter()
            .filter(|e| {
                e.3.current > 0.0
                    && e.1.translation.distance(hero.translation) < tuning.intercept_radius
            })
            .min_by(|a, b| {
                a.1.translation
                    .distance_squared(hero.translation)
                    .total_cmp(&b.1.translation.distance_squared(hero.translation))
            })
    {
        session.bond.target = Some(target);
        session.bond.remaining = tuning.intercept_mark_secs;
    }
    if talents.bone >= 3
        && session.wall_cooldown == 0.0
        && damage.enemies.iter().any(|e| {
            e.3.current > 0.0
                && e.1.translation.distance(hero.translation) < tuning.intercept_radius
        })
    {
        session.wall_cooldown = tuning.wall_cooldown;
        barrier.current = (barrier.current + tuning.wall_barrier).min(barrier.max);
        flash(&mut commands, &art, hero.translation, 1.6, false);
        for (_, mut v, _, _) in &mut minions {
            if v.current > 0.0 {
                v.guard_secs = tuning.wall_guard_secs;
            }
        }
    }
    let target_position = session.bond.target.and_then(|target| {
        damage
            .enemies
            .get(target)
            .ok()
            .filter(|e| e.3.current > 0.0)
            .map(|e| e.1.translation)
    });
    session.bond.tick(
        dt,
        health.current > 0.0
            && target_position.is_some_and(|position| within_leash(hero.translation, position)),
    );
    session.marked = session.bond.target;
    for (mut position, mut vitality, mut state, mut visible) in &mut minions {
        let new_max = (if state.bone {
            tuning.bone_health
        } else {
            tuning.spirit_health
        }) * (1.0
            + level.level.saturating_sub(1) as f32 * tuning.level_health_growth);
        if (new_max - vitality.max).abs() > 0.01 {
            let fraction = (vitality.current / vitality.max).clamp(0.0, 1.0);
            vitality.max = new_max;
            vitality.current = new_max * fraction;
        }
        vitality.guard_secs = (vitality.guard_secs - dt).max(0.0);
        if health.current <= 0.0 {
            *visible = Visibility::Hidden;
            state.attack = None;
            continue;
        }
        if vitality.current <= 0.0 {
            if vitality.dead_secs <= 0.0 {
                vitality.dead_secs = tuning.revive_secs;
                info!("SUMMON_DEATH slot={}", state.slot);
            }
            vitality.dead_secs = (vitality.dead_secs - dt).max(0.0);
            *visible = if vitality.dead_secs > tuning.revive_secs - 0.4 {
                position.rotation =
                    Quat::from_rotation_z((tuning.revive_secs - vitality.dead_secs) * 3.5);
                Visibility::Visible
            } else {
                Visibility::Hidden
            };
            state.attack = None;
            state.moving = false;
            state.progress = 1.0;
            if vitality.dead_secs <= 0.0 {
                vitality.current = vitality.max;
                position.translation = hero.translation + formation(state.slot, state.bone);
                flash(&mut commands, &art, position.translation, 1.2, false);
                info!("SUMMON_REVIVE slot={}", state.slot);
            } else {
                continue;
            }
        }
        *visible = Visibility::Visible;
        if talents.bone >= 3 && position.translation.distance(hero.translation) < 3.0 {
            vitality.current =
                (vitality.current + dt * tuning.bone_regen_per_sec).min(vitality.max);
        }
        if state
            .attack
            .is_some_and(|(target, _, _)| Some(target) != session.bond.target)
        {
            state.attack = None;
        }
        state.cooldown = (state.cooldown - dt).max(0.0);
        let goal = if session.guard > 0.0 {
            hero.translation + formation(state.slot, state.bone)
        } else {
            target_position
                .filter(|_| session.bond.target.is_some())
                .unwrap_or(hero.translation + formation(state.slot, state.bone))
        };
        if position.translation.distance(hero.translation) > LEASH * 1.5 {
            position.translation = hero.translation + formation(state.slot, state.bone);
            state.attack = None;
        }
        let delta = Vec3::new(
            goal.x - position.translation.x,
            0.0,
            goal.z - position.translation.z,
        );
        let distance = delta.length();
        let facing = delta.normalize_or_zero();
        let stop = if session.bond.target.is_some() && session.guard == 0.0 {
            REACH * 0.8
        } else {
            0.15
        };
        state.moving = distance > stop && state.attack.is_none();
        if state.moving {
            position.translation = crate::player::clamp_player_translation(
                &layout,
                step_toward(
                    position.translation,
                    goal,
                    (tuning.speed * dt).min(distance - stop),
                ),
            );
            state.phase += dt * 12.0;
        }
        if facing.length_squared() > 0.0 {
            position.rotation = Quat::from_rotation_y(facing.x.atan2(facing.z));
        }
        if state.attack.is_none()
            && state.cooldown == 0.0
            && distance <= REACH
            && session.guard == 0.0
            && let Some(target) = session.bond.target
        {
            state.attack = Some((
                target,
                0.0,
                state.volley > 0.0 || session.overdrive > 0.0 || session.bond.empowered,
            ));
        }
        state.progress = 1.0;
        if let Some((target, before, heavy)) = state.attack {
            let duration = if heavy { HEAVY_WINDUP } else { WINDUP };
            let speed = if session.overdrive > 0.0 {
                tuning.overdrive_attack_speed
            } else {
                1.0
            };
            let after = before + dt * speed;
            state.progress = (after / duration).min(1.0);
            state.heavy = heavy;
            if contact_crossed(before, after, heavy)
                && damage.enemies.get(target).is_ok_and(|e| {
                    e.3.current > 0.0 && e.1.translation.distance(position.translation) <= REACH
                })
            {
                let volley = std::mem::take(&mut state.volley);
                let base = if state.bone {
                    tuning.bone_damage
                } else {
                    tuning.spirit_damage
                };
                let amount = if volley > 0.0 {
                    volley
                } else {
                    scaled_damage(base, bonus.0, level.level, &tuning)
                        * if session.overdrive > 0.0 {
                            tuning.overdrive_damage_multiplier
                        } else if session.bond.empowered {
                            tuning.heavy_damage_multiplier
                        } else {
                            1.0
                        }
                };
                let radius = if talents.spirit >= 3 && session.overdrive > 0.0 {
                    tuning.overdrive_cleave_radius
                } else if volley > 0.0 {
                    tuning.volley_radius
                } else {
                    0.0
                };
                damage.strike(
                    &mut commands,
                    position.translation,
                    if radius > 0.0 {
                        HitArea::Radial {
                            center: position.translation,
                            radius,
                        }
                    } else {
                        HitArea::Target(target)
                    },
                    amount,
                    if heavy {
                        tuning.heavy_impact
                    } else {
                        tuning.basic_impact
                    },
                    if volley > 0.0 {
                        Some(state.rune)
                    } else if session.overdrive > 0.0 {
                        session.rune
                    } else {
                        None
                    },
                );
                if heavy {
                    session.bond.take_empowered();
                }
                if volley > 0.0 && talents.spirit >= 2 {
                    damage.strike(
                        &mut commands,
                        hero.translation,
                        HitArea::Target(target),
                        volley * tuning.echo_damage_fraction,
                        tuning.echo_impact,
                        Some(state.rune),
                    );
                    flash(
                        &mut commands,
                        &art,
                        hero.translation + Vec3::Y * 0.7,
                        0.7,
                        true,
                    );
                }
                slash(
                    &mut commands,
                    &art,
                    position.translation + Vec3::Y * 0.65,
                    position.rotation,
                    heavy,
                );
                info!(
                    "SUMMON_HIT target={target:?} heavy={heavy} amount={amount:.1} slot={}",
                    state.slot
                );
            }
            if after >= duration {
                state.attack = None;
                state.cooldown = tuning.attack_cooldown;
            } else {
                state.attack = Some((target, after, heavy));
            }
        }
    }
}
fn slash(commands: &mut Commands, art: &Art, position: Vec3, rotation: Quat, heavy: bool) {
    commands.spawn((
        Flash(0.18),
        Mesh3d(art.arc.clone()),
        MeshMaterial3d(if heavy {
            art.gold.clone()
        } else {
            art.cyan.clone()
        }),
        Transform::from_translation(position)
            .with_rotation(rotation)
            .with_scale(Vec3::splat(0.9)),
        NotShadowCaster,
        NotShadowReceiver,
    ));
}
fn flash(commands: &mut Commands, art: &Art, position: Vec3, size: f32, heavy: bool) {
    commands.spawn((
        Flash(0.24),
        Mesh3d(if position.y > 0.3 {
            art.arc.clone()
        } else {
            art.ring.clone()
        }),
        MeshMaterial3d(if heavy {
            art.gold.clone()
        } else {
            art.cyan.clone()
        }),
        Transform::from_translation(position + Vec3::Y * 0.035).with_scale(Vec3::splat(size)),
        NotShadowCaster,
        NotShadowReceiver,
    ));
}
#[allow(clippy::type_complexity, clippy::too_many_arguments)]
fn visuals(
    time: Res<Time>,
    session: Res<SummonStatus>,
    states: Query<&State>,
    enemies: Query<&Transform, With<Enemy>>,
    mut parts: Query<
        (&Part, &ChildOf, &mut Transform),
        (
            Without<Enemy>,
            Without<Marker>,
            Without<Flash>,
            Without<Minion>,
        ),
    >,
    mut marker: Query<
        (&mut Transform, &mut Visibility),
        (
            With<Marker>,
            Without<Enemy>,
            Without<Part>,
            Without<Flash>,
            Without<Minion>,
        ),
    >,
    mut flashes: Query<
        (Entity, &mut Flash, &mut Transform),
        (
            Without<Part>,
            Without<Enemy>,
            Without<Marker>,
            Without<Minion>,
        ),
    >,
    mut commands: Commands,
) {
    for (part, parent, mut transform) in &mut parts {
        let Ok(state) = states.get(parent.parent()) else {
            continue;
        };
        *transform = if part.0 == 5 {
            summon_weapon_transform(state.progress, state.heavy)
        } else {
            summon_limb_transform(
                part.0,
                state.phase,
                state.moving,
                state.progress,
                state.heavy,
            )
        };
    }
    for (mut transform, mut visibility) in &mut marker {
        if let Some(target) = session.bond.target
            && let Ok(target) = enemies.get(target)
        {
            *visibility = Visibility::Visible;
            transform.translation = target.translation + Vec3::Y * 0.035;
            transform.scale = Vec3::splat(0.52);
        } else {
            *visibility = Visibility::Hidden;
        }
    }
    for (entity, mut life, mut transform) in &mut flashes {
        life.0 -= time.delta_secs();
        transform.scale *= 1.0 + time.delta_secs() * 1.4;
        if life.0 <= 0.0 {
            commands.entity(entity).despawn();
        }
    }
}
#[allow(clippy::type_complexity)]
fn clear(
    mut commands: Commands,
    roots: Query<Entity, Or<(With<Minion>, With<Marker>, With<Flash>)>>,
    mut panels: Query<&mut Node, With<MinionHud>>,
    mut session: ResMut<SummonStatus>,
) {
    for entity in &roots {
        commands.entity(entity).despawn();
    }
    for mut panel in &mut panels {
        panel.display = Display::None;
    }
    *session = SummonStatus::default();
}

impl SummonVitality {
    pub(crate) fn damage(&mut self, amount: f32) -> f32 {
        if self.current <= 0.0 {
            return 0.0;
        }
        let dealt = amount.max(0.0)
            * if self.guard_secs > 0.0 {
                self.guard_multiplier
            } else {
                1.0
            };
        let before = self.current;
        self.current = (self.current - dealt).max(0.0);
        before - self.current
    }
}
pub(crate) fn can_command(command: SummonCommand, living: usize, marked: bool) -> bool {
    living > 0 && (command != SummonCommand::Volley || marked)
}

#[derive(Component)]
struct MinionHud;
#[derive(Component)]
struct MinionHudText;
#[derive(Component)]
enum HudNode {
    Root,
    Bar(usize),
    Fill(usize),
}
fn spawn_hud(mut commands: Commands, server: Res<AssetServer>) {
    commands
        .spawn((
            MinionHud,
            HudNode::Root,
            Node {
                position_type: PositionType::Absolute,
                left: Val::Px(18.0),
                top: Val::Px(184.0),
                padding: UiRect::all(Val::Px(8.0)),
                display: Display::None,
                ..default()
            },
            BackgroundColor(Color::srgba(0.02, 0.03, 0.035, 0.78)),
            BorderColor::all(Color::srgba(0.25, 0.58, 0.58, 0.65)),
        ))
        .with_children(|row| {
            row.spawn((
                ImageNode::new(server.load("images/generated/ui/class-portrait-soulbinder.png")),
                Node {
                    width: Val::Px(30.0),
                    height: Val::Px(42.0),
                    margin: UiRect::right(Val::Px(8.0)),
                    ..default()
                },
            ));
            row.spawn((
                MinionHudText,
                crate::ui::localization::NativeLanguageName,
                Text::new(""),
                TextFont {
                    font: server.load("fonts/NotoSansSC.ttf").into(),
                    font_size: FontSize::Px(14.0),
                    ..default()
                },
                TextColor(Color::srgb(0.7, 0.9, 0.88)),
            ));
            row.spawn(Node {
                flex_direction: FlexDirection::Column,
                row_gap: Val::Px(5.0),
                margin: UiRect::left(Val::Px(8.0)),
                ..default()
            })
            .with_children(|bars| {
                for slot in 0..2 {
                    bars.spawn((
                        HudNode::Bar(slot),
                        Node {
                            width: Val::Px(94.0),
                            height: Val::Px(5.0),
                            ..default()
                        },
                        BackgroundColor(Color::srgb(0.08, 0.12, 0.13)),
                    ))
                    .with_children(|bar| {
                        bar.spawn((
                            HudNode::Fill(slot),
                            Node {
                                width: Val::Percent(100.0),
                                height: Val::Percent(100.0),
                                ..default()
                            },
                            BackgroundColor(Color::srgb(0.24, 0.65, 0.58)),
                        ));
                    });
                }
            });
        });
}
#[allow(clippy::too_many_arguments)]
fn update_hud(
    selected: Res<SelectedClass>,
    language: Res<crate::ui::localization::UiLanguage>,
    state: Res<bevy::prelude::State<GameState>>,
    session: Res<SummonStatus>,
    minions: Query<(&State, &SummonVitality), With<Minion>>,
    player: Query<&SummonTalents, With<Player>>,
    mut nodes: Query<(&HudNode, &mut Node)>,
    mut labels: Query<&mut Text, With<MinionHudText>>,
) {
    let visible = selected.0 == HeroClass::Soulbinder && *state.get() == GameState::InGame;
    let zh = *language == crate::ui::localization::UiLanguage::Chinese;
    let mut rows: Vec<_> = minions.iter().collect();
    rows.sort_by_key(|(s, _)| s.slot);
    for (part, mut node) in &mut nodes {
        match part {
            HudNode::Root => {
                node.display = if visible {
                    Display::Flex
                } else {
                    Display::None
                }
            }
            HudNode::Bar(slot) => {
                node.display = if rows.iter().any(|(s, _)| s.slot == *slot) {
                    Display::Flex
                } else {
                    Display::None
                }
            }
            HudNode::Fill(slot) => {
                node.width = Val::Percent(
                    rows.iter()
                        .find(|(s, _)| s.slot == *slot)
                        .map(|(_, v)| (v.current / v.max * 100.0).clamp(0.0, 100.0))
                        .unwrap_or(0.0),
                )
            }
        }
    }
    let text = rows
        .into_iter()
        .map(|(s, v)| {
            let name = match (s.bone, zh) {
                (true, true) => "骨卫",
                (false, true) => "契灵",
                (true, false) => "Bone guard",
                (false, false) => "Bound spirit",
            };
            if v.current <= 0.0 {
                format!(
                    "{name} · {} {:.0} {}",
                    if zh { "重聚" } else { "Reforming" },
                    v.dead_secs.ceil(),
                    if zh { "秒" } else { "s" }
                )
            } else {
                format!("{name} · {:.0}/{:.0}", v.current, v.max)
            }
        })
        .collect::<Vec<_>>()
        .join("\n");
    let status = if session.overdrive > 0.0 {
        format!(
            "{} {:.0} {}",
            if zh { "狂攻" } else { "Unbound" },
            session.overdrive.ceil(),
            if zh { "秒" } else { "s" }
        )
    } else if session.guard > 0.0 {
        if zh { "护阵" } else { "Guarding" }.to_string()
    } else if session.marked.is_some() {
        if zh { "追击" } else { "Pursuing" }.to_string()
    } else {
        if zh { "待命" } else { "Following" }.to_string()
    };
    let resonance = if player.single().is_ok_and(|tree| tree.spirit >= 1) {
        if session.bond.empowered {
            if zh {
                " · 共鸣就绪"
            } else {
                " · Resonance ready"
            }
            .to_string()
        } else {
            format!(
                " · {} {}/3",
                if zh { "共鸣" } else { "Resonance" },
                session.basics % 3
            )
        }
    } else {
        String::new()
    };
    for mut label in &mut labels {
        label.0 = format!("{text}\n{status}{resonance}");
    }
}

fn scaled_damage(base: f32, gear: f32, level: u32, tuning: &Tuning) -> f32 {
    (base + gear.max(0.0) * tuning.gear_damage_share)
        * (1.0 + level.saturating_sub(1) as f32 * tuning.level_damage_growth)
}

#[allow(clippy::type_complexity)]
fn auras(
    session: Res<SummonStatus>,
    art: Res<Art>,
    states: Query<&SummonVitality, With<Minion>>,
    mut auras: Query<
        (
            &ChildOf,
            &mut Visibility,
            &mut Transform,
            &mut MeshMaterial3d<StandardMaterial>,
        ),
        With<Aura>,
    >,
) {
    for (parent, mut visible, mut transform, mut material) in &mut auras {
        let alive = states.get(parent.parent()).is_ok_and(|v| v.current > 0.0);
        *visible = if alive && (session.overdrive > 0.0 || session.guard > 0.0) {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
        material.0 = if session.overdrive > 0.0 {
            art.gold.clone()
        } else {
            art.cyan.clone()
        };
        transform.scale = Vec3::splat(if session.overdrive > 0.0 { 0.7 } else { 0.9 });
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn volley_requires_a_living_minion_and_mark() {
        assert!(!can_command(SummonCommand::Volley, 1, false));
        assert!(!can_command(SummonCommand::Volley, 0, true));
        assert!(can_command(SummonCommand::Volley, 1, true));
        assert!(can_command(SummonCommand::Recall, 1, false));
    }
    #[test]
    fn guard_reduces_incoming_damage_and_death_cannot_take_more() {
        let mut health = SummonVitality {
            current: 20.0,
            max: 20.0,
            dead_secs: 0.0,
            guard_secs: 5.0,
            guard_multiplier: 0.45,
        };
        assert_eq!(health.damage(20.0), 9.0);
        health.guard_secs = 0.0;
        assert_eq!(health.damage(30.0), 11.0);
        assert_eq!(health.damage(100.0), 0.0);
    }
    #[test]
    fn level_and_weapon_damage_strengthen_real_minion_strikes() {
        let tuning: Tuning = ron::from_str(include_str!("../../assets/data/summons.ron")).unwrap();
        assert!(scaled_damage(12.0, 20.0, 6, &tuning) > scaled_damage(12.0, 0.0, 1, &tuning) * 1.8);
        assert_eq!(scaled_damage(12.0, -10.0, 1, &tuning), 12.0);
    }
    #[test]
    fn bone_formation_has_two_separate_flanks() {
        assert!(formation(0, true).x < 0.0);
        assert!(formation(1, true).x > 0.0);
    }
}
