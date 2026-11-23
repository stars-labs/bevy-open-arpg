//! Signature combat loops and live HUD gauges for Warden, Stalker, and Embercaller.
use super::{
    skill_vfx::{ClassProcKind, ClassProcVisual},
    *,
};
use crate::class::{HeroClass, Movement, ProjectileArt, Rune, SelectedClass, SkillSlot};

const WARDEN_MAX_CHARGES: u32 = 3;
const STALKER_MAX_CHARGES: u32 = 4;
const EMBER_MAX_CHARGES: u32 = 3;

#[derive(Message, Clone, Copy, Debug)]
pub(super) struct ClassCastSignal {
    pub origin: Vec3,
    pub facing: Vec3,
    pub target: Vec3,
    pub shape: StrikeShape,
    pub skill: Option<SkillCast>,
    pub movement: Movement,
    pub damage: f32,
    pub impact: f32,
    pub high_fury: bool,
    pub low_health: bool,
}

#[derive(Resource, Default, Debug)]
pub(super) struct ClassMasteryState {
    pub charges: u32,
    last_contact_cast: u32,
    banner_secs: f32,
    last_proc: Option<ClassProcKind>,
}

impl ClassMasteryState {
    pub(super) fn max_charges_for(class: HeroClass, talents: &Talents) -> u32 {
        match class {
            HeroClass::Warden => {
                if talents.wrath >= 4 {
                    2
                } else {
                    WARDEN_MAX_CHARGES
                }
            }
            HeroClass::Stalker => {
                if talents.focus >= 2 {
                    3
                } else {
                    STALKER_MAX_CHARGES
                }
            }
            HeroClass::Embercaller => {
                if talents.focus >= 2 {
                    2
                } else {
                    EMBER_MAX_CHARGES
                }
            }
            HeroClass::Soulbinder => 0,
        }
    }

    fn add_charge_for(&mut self, class: HeroClass, talents: &Talents, amount: u32) {
        let cap = Self::max_charges_for(class, talents);
        if cap > 0 {
            self.charges = (self.charges + amount).min(cap);
        }
    }

    #[cfg(test)]
    fn add_charge(&mut self, class: HeroClass, amount: u32) {
        self.add_charge_for(class, &Talents::default(), amount);
    }

    fn ready_for(&self, class: HeroClass, talents: &Talents) -> bool {
        let cap = Self::max_charges_for(class, talents);
        cap > 0 && self.charges >= cap
    }

    #[cfg(test)]
    fn ready(&self, class: HeroClass) -> bool {
        self.ready_for(class, &Talents::default())
    }
}

pub(super) fn install(app: &mut App) {
    app.init_resource::<ClassMasteryState>()
        .add_message::<ClassCastSignal>()
        .add_systems(Startup, spawn_hud)
        .add_systems(
            Update,
            (observe_contacts, resolve_class_mastery)
                .chain()
                .after(CombatSet::PlayerDamage)
                .run_if(in_state(GameState::InGame).and_then(not_paused)),
        )
        .add_systems(Update, update_hud)
        .add_systems(OnExit(GameState::InGame), clear_state);
}

fn clear_state(
    mut state: ResMut<ClassMasteryState>,
    mut panels: Query<&mut Node, With<MasteryHudRoot>>,
) {
    *state = ClassMasteryState::default();
    for mut node in &mut panels {
        node.display = Display::None;
    }
}

fn observe_contacts(
    selected: Res<SelectedClass>,
    mut contacts: MessageReader<ContactFeedback>,
    mut state: ResMut<ClassMasteryState>,
    player_talents: Query<&Talents, With<Player>>,
) {
    let talents = player_talents.single().copied().unwrap_or_default();
    for event in contacts.read() {
        if event.cast == 0 || event.cast == state.last_contact_cast {
            continue;
        }
        match selected.0 {
            HeroClass::Warden if event.skill.is_none() => {
                state.last_contact_cast = event.cast;
                state.add_charge_for(HeroClass::Warden, &talents, 1);
            }
            HeroClass::Stalker => {
                state.last_contact_cast = event.cast;
                state.add_charge_for(HeroClass::Stalker, &talents, 1);
            }
            _ => {}
        }
    }
}

#[allow(clippy::too_many_arguments, clippy::type_complexity)]
fn resolve_class_mastery(
    time: Res<Time>,
    selected: Res<SelectedClass>,
    language: Res<crate::ui::localization::UiLanguage>,
    mut state: ResMut<ClassMasteryState>,
    mut signals: MessageReader<ClassCastSignal>,
    mut world: StrikeWorld,
    mut heroes: Query<
        (&mut Health, &mut Barrier, &mut Fury, Option<&Talents>),
        (With<Player>, Without<Enemy>),
    >,
) {
    state.banner_secs = (state.banner_secs - time.delta_secs()).max(0.0);
    if state.banner_secs <= 0.0 {
        state.last_proc = None;
    }
    if selected.0 == HeroClass::Soulbinder {
        signals.clear();
        return;
    }
    let zh = *language == crate::ui::localization::UiLanguage::Chinese;
    let Ok((mut health, mut barrier, mut resource, talents_opt)) = heroes.single_mut() else {
        signals.clear();
        return;
    };
    let talents = talents_opt.copied().unwrap_or_default();

    for signal in signals.read() {
        match selected.0 {
            HeroClass::Warden => {
                // 1. Reliquary Solar Cleave when Valor stacks are ready (3, or 2 at Wrath >= 4).
                if state.ready_for(HeroClass::Warden, &talents) {
                    state.charges = 0;
                    state.last_proc = Some(ClassProcKind::WardenSolarCleave);
                    state.banner_secs = 1.3;
                    let center = signal.origin + signal.facing * 1.1;
                    let radius = if talents.wrath >= 2 { 4.2 } else { 3.2 };
                    let damage_mult = if talents.wrath >= 2 { 0.90 } else { 0.65 };
                    let vfx = combat_vfx_assets(&world.assets);
                    apply_strike(
                        &mut world.commands,
                        &mut AttackFeedback {
                            combat_events: &mut world.combat_events,
                            floating_text: &mut world.floating_text,
                            screen_shake: &mut world.screen_shake,
                            hit_stop: &mut world.hit_stop,
                        },
                        &vfx,
                        Strike {
                            area: HitArea::Radial { center, radius },
                            source: signal.origin,
                            profile: proc_profile(
                                signal.damage * damage_mult,
                                1.35,
                                Some(Rune::Expose),
                            ),
                        },
                        &mut world.enemies,
                        &world.bestiary,
                    );
                    // Wrath >= 2: Solar Shockwave launches a second forward solar eruption!
                    if talents.wrath >= 2 {
                        let forward_center = signal.origin + signal.facing * 3.3;
                        apply_strike(
                            &mut world.commands,
                            &mut AttackFeedback {
                                combat_events: &mut world.combat_events,
                                floating_text: &mut world.floating_text,
                                screen_shake: &mut world.screen_shake,
                                hit_stop: &mut world.hit_stop,
                            },
                            &vfx,
                            Strike {
                                area: HitArea::Radial {
                                    center: forward_center,
                                    radius: 2.5,
                                },
                                source: signal.origin,
                                profile: proc_profile(signal.damage * 0.55, 1.2, Some(Rune::Ember)),
                            },
                            &mut world.enemies,
                            &world.bestiary,
                        );
                    }
                    // Vigor >= 2 doubles Solar Cleave barrier (12 -> 24).
                    barrier.grant(if talents.vigor >= 2 { 24.0 } else { 12.0 });
                    // Focus >= 2 refunds +18 Fury on Solar Cleave!
                    if talents.focus >= 2 {
                        resource.gain(18.0);
                    }
                    world.commands.write_message(ClassProcVisual {
                        kind: ClassProcKind::WardenSolarCleave,
                        origin: signal.origin,
                        facing: signal.facing,
                        radius,
                    });
                    world.floating_text.write(FloatingCombatTextEvent {
                        text: if zh { "圣焰重劈" } else { "Solar Cleave" }.into(),
                        position: signal.origin + Vec3::Y * 1.35,
                        critical: true,
                    });
                }

                // Focus >= 4: Chain Valor grants +1 Valor stack when weaving any skill.
                if signal.skill.is_some() && talents.focus >= 4 {
                    state.add_charge_for(HeroClass::Warden, &talents, 1);
                }

                // 2. Aegis Quake when casting a skill at high fury (>=70), Unbowed (<40% HP),
                // or unconditionally on Area/Mobility casts at Vigor >= 4.
                let unbreakable_quake = talents.vigor >= 4
                    && signal.skill.is_some_and(|cast| {
                        matches!(cast.slot, SkillSlot::Area | SkillSlot::Mobility)
                    });
                if signal.skill.is_some()
                    && (signal.high_fury || signal.low_health || unbreakable_quake)
                {
                    state.last_proc = Some(ClassProcKind::WardenAegisQuake);
                    state.banner_secs = 1.25;
                    let radius = if talents.vigor >= 2 { 4.1 } else { 3.4 };
                    let vfx = combat_vfx_assets(&world.assets);
                    apply_strike(
                        &mut world.commands,
                        &mut AttackFeedback {
                            combat_events: &mut world.combat_events,
                            floating_text: &mut world.floating_text,
                            screen_shake: &mut world.screen_shake,
                            hit_stop: &mut world.hit_stop,
                        },
                        &vfx,
                        Strike {
                            area: HitArea::Radial {
                                center: signal.origin,
                                radius,
                            },
                            source: signal.origin,
                            profile: proc_profile(signal.damage * 0.45, 1.4, Some(Rune::Expose)),
                        },
                        &mut world.enemies,
                        &world.bestiary,
                    );
                    if talents.vigor >= 2 {
                        health.current = (health.current + 10.0).min(health.max);
                        barrier.grant(12.0);
                    }
                    world.commands.write_message(ClassProcVisual {
                        kind: ClassProcKind::WardenAegisQuake,
                        origin: signal.origin,
                        facing: signal.facing,
                        radius,
                    });
                    world.floating_text.write(FloatingCombatTextEvent {
                        text: if zh { "怒火震慑" } else { "Aegis Quake" }.into(),
                        position: signal.origin + Vec3::Y * 1.6,
                        critical: false,
                    });
                }
            }
            HeroClass::Stalker => {
                // Mobility skills immediately prime Predator Focus to full!
                if signal
                    .skill
                    .is_some_and(|cast| cast.slot == SkillSlot::Mobility)
                {
                    state.charges =
                        ClassMasteryState::max_charges_for(HeroClass::Stalker, &talents);
                    // Vigor >= 4: Shadow Mend restores +12 HP and +12 Barrier on Vault / Shadow Lunge.
                    if talents.vigor >= 4 {
                        health.current = (health.current + 12.0).min(health.max);
                        barrier.grant(12.0);
                    }
                }

                // Arrow Rain triggers a secondary staggered scatter volley (or 3 waves at Vigor >= 2!).
                if let StrikeShape::GroundBlast { radius, delay, .. } = signal.shape {
                    let follow_profile = proc_profile(
                        signal.damage * 0.55,
                        signal.impact * 0.9,
                        signal.skill.map(|cast| cast.rune),
                    );
                    ranged::mark_ground_blast(
                        &mut world.commands,
                        signal.origin,
                        signal.target,
                        radius * 0.92,
                        delay + 0.28,
                        follow_profile,
                    );
                    if talents.vigor >= 2 {
                        ranged::mark_ground_blast(
                            &mut world.commands,
                            signal.origin,
                            signal.target,
                            radius * 1.05,
                            delay + 0.54,
                            follow_profile,
                        );
                    }
                }

                // Full Focus unleashes Phantom Volley (2 arrows, or 4 arrows at Wrath >= 2, crit burst at Wrath >= 4).
                if state.ready_for(HeroClass::Stalker, &talents)
                    && matches!(signal.shape, StrikeShape::Projectiles { .. })
                    && signal
                        .skill
                        .is_none_or(|cast| cast.slot != SkillSlot::Mobility)
                {
                    state.charges = 0;
                    state.last_proc = Some(ClassProcKind::StalkerPhantomVolley);
                    state.banner_secs = 1.25;
                    let arrow_count = if talents.wrath >= 2 { 4 } else { 2 };
                    let spread_deg = if talents.wrath >= 2 { 38.0 } else { 26.0 };
                    let dmg_scale = if talents.wrath >= 4 { 0.90 } else { 0.72 };
                    let mut bonus_profile = proc_profile(
                        signal.damage * dmg_scale,
                        1.15,
                        Some(signal.skill.map_or(Rune::Expose, |c| c.rune)),
                    );
                    if talents.wrath >= 4 {
                        bonus_profile.crit_chance = 0.45;
                    }
                    ranged::launch_projectiles(
                        &mut world.commands,
                        &world.assets,
                        signal.origin,
                        signal.facing,
                        StrikeShape::Projectiles {
                            art: ProjectileArt::BoneArrow,
                            count: arrow_count,
                            spread_deg,
                            speed: 32.0,
                            range: 12.0,
                            width: 0.48,
                            pierce: true,
                            blast_radius: 0.0,
                        },
                        bonus_profile,
                    );
                    // Focus >= 4 doubles Malice refund (10 -> 20).
                    resource.gain(if talents.focus >= 4 { 20.0 } else { 10.0 });
                    world.commands.write_message(ClassProcVisual {
                        kind: ClassProcKind::StalkerPhantomVolley,
                        origin: signal.origin,
                        facing: signal.facing,
                        radius: 1.8,
                    });
                    world.floating_text.write(FloatingCombatTextEvent {
                        text: if zh { "追魂连矢" } else { "Phantom Volley" }.into(),
                        position: signal.origin + Vec3::Y * 1.4,
                        critical: true,
                    });
                }
            }
            HeroClass::Embercaller => {
                // Ash Step blink sparks Ember Veil barrier (10, or 22 + flame pulse at Vigor >= 2).
                if matches!(signal.movement, Movement::Blink(_)) {
                    barrier.grant(if talents.vigor >= 2 { 22.0 } else { 10.0 });
                    if talents.vigor >= 2 {
                        let vfx = combat_vfx_assets(&world.assets);
                        apply_strike(
                            &mut world.commands,
                            &mut AttackFeedback {
                                combat_events: &mut world.combat_events,
                                floating_text: &mut world.floating_text,
                                screen_shake: &mut world.screen_shake,
                                hit_stop: &mut world.hit_stop,
                            },
                            &vfx,
                            Strike {
                                area: HitArea::Radial {
                                    center: signal.origin,
                                    radius: 3.0,
                                },
                                source: signal.origin,
                                profile: proc_profile(signal.damage * 0.48, 1.1, Some(Rune::Ember)),
                            },
                            &mut world.enemies,
                            &world.bestiary,
                        );
                    }
                }

                // At max Cinder Charges (3, or 2 at Focus >= 2), casting any skill triggers Cinder Overload!
                if state.ready_for(HeroClass::Embercaller, &talents) && signal.skill.is_some() {
                    state.charges = 0;
                    state.last_proc = Some(ClassProcKind::EmberOverload);
                    state.banner_secs = 1.35;
                    let anchor = match signal.shape {
                        StrikeShape::GroundBlast { .. } => signal.target,
                        _ => signal.origin + signal.facing * 3.8,
                    };
                    let meteor_count = if talents.wrath >= 2 { 5 } else { 3 };
                    let blast_radius = if talents.wrath >= 4 { 2.8 } else { 2.1 };
                    let dmg_scale = if talents.wrath >= 4 { 0.72 } else { 0.52 };
                    let meteor_profile =
                        proc_profile(signal.damage * dmg_scale, 1.2, Some(Rune::Ember));
                    for i in 0..meteor_count {
                        let angle = i as f32 * std::f32::consts::TAU / meteor_count as f32 + 0.4;
                        let ring_dist = if meteor_count > 3 { 2.05 } else { 1.55 };
                        let offset = Vec3::new(angle.sin(), 0.0, angle.cos()) * ring_dist;
                        ranged::mark_ground_blast(
                            &mut world.commands,
                            signal.origin,
                            anchor + offset,
                            blast_radius,
                            0.20 + i as f32 * 0.09,
                            meteor_profile,
                        );
                    }
                    // Focus >= 4 refunds +22 Arcana (vs +10).
                    resource.gain(if talents.focus >= 4 { 22.0 } else { 10.0 });
                    // Vigor >= 4 grants Frost-Ember Ward barrier & healing on Overload.
                    if talents.vigor >= 4 {
                        barrier.grant(18.0);
                        health.current = (health.current + 8.0).min(health.max);
                    }
                    world.commands.write_message(ClassProcVisual {
                        kind: ClassProcKind::EmberOverload,
                        origin: anchor,
                        facing: signal.facing,
                        radius: if talents.wrath >= 4 { 3.4 } else { 2.8 },
                    });
                    world.floating_text.write(FloatingCombatTextEvent {
                        text: if zh {
                            "余烬过载"
                        } else {
                            "Cinder Overload"
                        }
                        .into(),
                        position: signal.origin + Vec3::Y * 1.45,
                        critical: true,
                    });
                } else {
                    state.add_charge_for(HeroClass::Embercaller, &talents, 1);
                }
            }
            HeroClass::Soulbinder => {}
        }
    }
}

fn proc_profile(damage: f32, impact_scale: f32, rune: Option<Rune>) -> StrikeProfile {
    StrikeProfile {
        damage,
        impact_scale,
        crit_chance: 0.15,
        legendary_powers: LegendaryPowerSet::new(LegendaryPower::None, LegendaryPower::None),
        skill: rune.map(|rune| SkillCast {
            slot: SkillSlot::Area,
            rune,
        }),
        hit_stop: impact_scale >= 1.3,
        long_shot: None,
        cast: 0,
        fury_bonus: 0.0,
        barrier_reward: 0.0,
    }
}

#[derive(Component)]
struct MasteryHudRoot;
#[derive(Component)]
struct MasteryHudPortrait;
#[derive(Component)]
struct MasteryHudText;
#[derive(Component)]
struct MasteryHudFill;

fn spawn_hud(mut commands: Commands, server: Res<AssetServer>) {
    commands
        .spawn((
            MasteryHudRoot,
            Node {
                position_type: PositionType::Absolute,
                left: Val::Px(18.0),
                top: Val::Px(184.0),
                padding: UiRect::all(Val::Px(8.0)),
                align_items: AlignItems::Center,
                display: Display::None,
                ..default()
            },
            BackgroundColor(Color::srgba(0.03, 0.025, 0.025, 0.80)),
            BorderColor::all(Color::srgba(0.75, 0.56, 0.24, 0.72)),
        ))
        .with_children(|row| {
            row.spawn((
                MasteryHudPortrait,
                ImageNode::new(server.load("images/generated/ui/class-portrait-warden.png")),
                Node {
                    width: Val::Px(30.0),
                    height: Val::Px(42.0),
                    margin: UiRect::right(Val::Px(8.0)),
                    ..default()
                },
            ));
            row.spawn(Node {
                flex_direction: FlexDirection::Column,
                row_gap: Val::Px(5.0),
                ..default()
            })
            .with_children(|col| {
                col.spawn((
                    MasteryHudText,
                    crate::ui::localization::NativeLanguageName,
                    Text::new(""),
                    TextFont {
                        font: server.load("fonts/NotoSansSC.ttf").into(),
                        font_size: FontSize::Px(14.0),
                        ..default()
                    },
                    TextColor(Color::srgb(0.95, 0.88, 0.72)),
                ));
                col.spawn((
                    Node {
                        width: Val::Px(132.0),
                        height: Val::Px(6.0),
                        ..default()
                    },
                    BackgroundColor(Color::srgb(0.10, 0.09, 0.09)),
                ))
                .with_children(|bar| {
                    bar.spawn((
                        MasteryHudFill,
                        Node {
                            width: Val::Percent(0.0),
                            height: Val::Percent(100.0),
                            ..default()
                        },
                        BackgroundColor(Color::srgb(0.92, 0.70, 0.26)),
                    ));
                });
            });
        });
}

#[allow(clippy::too_many_arguments, clippy::type_complexity)]
fn update_hud(
    selected: Res<SelectedClass>,
    language: Res<crate::ui::localization::UiLanguage>,
    game_state: Res<State<GameState>>,
    mastery: Res<ClassMasteryState>,
    server: Res<AssetServer>,
    windows: Query<&Window, With<bevy::window::PrimaryWindow>>,
    player_talents: Query<&Talents, With<Player>>,
    mut root_q: Query<(&mut Node, &mut BorderColor), With<MasteryHudRoot>>,
    mut portrait_q: Query<&mut ImageNode, With<MasteryHudPortrait>>,
    mut text_q: Query<(&mut Text, &mut TextColor), With<MasteryHudText>>,
    mut fill_q: Query<
        (&mut Node, &mut BackgroundColor),
        (With<MasteryHudFill>, Without<MasteryHudRoot>),
    >,
) {
    let visible = selected.0 != HeroClass::Soulbinder && *game_state.get() == GameState::InGame;
    let talents = player_talents.single().copied().unwrap_or_default();
    let zh = *language == crate::ui::localization::UiLanguage::Chinese;
    let (accent, border, portrait_path, title, ready_text) = match selected.0 {
        HeroClass::Warden => (
            Color::srgb(0.95, 0.74, 0.30),
            Color::srgba(0.85, 0.62, 0.22, 0.75),
            "images/generated/ui/class-portrait-warden.png",
            if zh {
                "圣匣战意"
            } else {
                "Reliquary Valor"
            },
            if zh {
                "圣焰重劈就绪!"
            } else {
                "Solar Cleave Ready!"
            },
        ),
        HeroClass::Stalker => (
            Color::srgb(0.24, 0.88, 0.68),
            Color::srgba(0.22, 0.76, 0.60, 0.75),
            "images/generated/ui/class-portrait-stalker.png",
            if zh { "猎手专注" } else { "Predator Focus" },
            if zh {
                "追魂连矢就绪!"
            } else {
                "Phantom Volley Ready!"
            },
        ),
        HeroClass::Embercaller => (
            Color::srgb(1.0, 0.46, 0.14),
            Color::srgba(0.92, 0.40, 0.16, 0.75),
            "images/generated/ui/class-portrait-embercaller.png",
            if zh { "余烬电荷" } else { "Cinder Charge" },
            if zh {
                "余烬过载就绪!"
            } else {
                "Overload Ready!"
            },
        ),
        HeroClass::Soulbinder => (
            Color::WHITE,
            Color::NONE,
            "images/generated/ui/class-portrait-soulbinder.png",
            "",
            "",
        ),
    };
    let compact = windows.single().is_ok_and(|w| w.width() < 760.0);
    for (mut node, mut border_color) in &mut root_q {
        node.display = if visible {
            Display::Flex
        } else {
            Display::None
        };
        node.left = Val::Px(if compact { 10.0 } else { 18.0 });
        node.top = Val::Px(if compact { 96.0 } else { 184.0 });
        *border_color = BorderColor::all(border);
    }
    if !visible {
        return;
    }
    for mut portrait in &mut portrait_q {
        portrait.image = server.load(portrait_path);
    }
    let max = ClassMasteryState::max_charges_for(selected.0, &talents).max(1);
    let is_ready = mastery.ready_for(selected.0, &talents);
    let pct = (mastery.charges as f32 / max as f32 * 100.0).clamp(0.0, 100.0);
    let proc_line = mastery.last_proc.map(|kind| match (kind, zh) {
        (ClassProcKind::WardenSolarCleave, true) => "触发 · 圣焰重劈",
        (ClassProcKind::WardenAegisQuake, true) => "触发 · 怒火震慑",
        (ClassProcKind::StalkerPhantomVolley, true) => "触发 · 追魂连矢",
        (ClassProcKind::EmberOverload, true) => "触发 · 余烬过载",
        (ClassProcKind::WardenSolarCleave, false) => "Proc · Solar Cleave",
        (ClassProcKind::WardenAegisQuake, false) => "Proc · Aegis Quake",
        (ClassProcKind::StalkerPhantomVolley, false) => "Proc · Phantom Volley",
        (ClassProcKind::EmberOverload, false) => "Proc · Cinder Overload",
    });
    let body = if is_ready {
        format!("{title} · {ready_text}")
    } else if let Some(proc_text) = proc_line {
        format!("{title} {}/{max}\n{proc_text}", mastery.charges)
    } else {
        format!("{title} · {}/{max}", mastery.charges)
    };
    for (mut text, mut color) in &mut text_q {
        text.0 = body.clone();
        color.0 = if is_ready {
            accent
        } else {
            Color::srgb(0.92, 0.88, 0.80)
        };
    }
    for (mut fill_node, mut bg) in &mut fill_q {
        fill_node.width = Val::Percent(pct);
        bg.0 = accent;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mastery_charges_clamp_per_class_cap() {
        let mut state = ClassMasteryState::default();
        state.add_charge(HeroClass::Warden, 2);
        assert!(!state.ready(HeroClass::Warden));
        state.add_charge(HeroClass::Warden, 5);
        assert_eq!(state.charges, WARDEN_MAX_CHARGES);
        assert!(state.ready(HeroClass::Warden));

        let mut stalker = ClassMasteryState::default();
        stalker.add_charge(HeroClass::Stalker, 3);
        assert!(!stalker.ready(HeroClass::Stalker));
        stalker.add_charge(HeroClass::Stalker, 1);
        assert!(stalker.ready(HeroClass::Stalker));

        let mut ember = ClassMasteryState::default();
        ember.add_charge(HeroClass::Embercaller, 3);
        assert!(ember.ready(HeroClass::Embercaller));
        assert!(!ember.ready(HeroClass::Soulbinder));
    }

    fn mastery_app(class: HeroClass) -> (App, Entity) {
        let mut app = App::new();
        app.insert_resource(Time::<()>::default())
            .insert_resource(SelectedClass(class))
            .insert_resource(crate::ui::localization::UiLanguage::Chinese)
            .init_resource::<GameAssets>()
            .init_resource::<Bestiary>()
            .init_resource::<RunStats>()
            .init_resource::<CombatComboState>()
            .init_resource::<ClassMasteryState>()
            .add_message::<ContactFeedback>()
            .add_message::<ClassCastSignal>()
            .add_message::<ClassProcVisual>()
            .add_message::<skill_vfx::SkillVisual>()
            .add_message::<skill_vfx::ProjectileVisual>()
            .add_message::<CombatEvent>()
            .add_message::<FloatingCombatTextEvent>()
            .add_message::<ScreenShakeEvent>()
            .add_message::<HitStopEvent>()
            .add_systems(Update, (observe_contacts, resolve_class_mastery).chain());
        let hero = app
            .world_mut()
            .spawn((
                Player,
                Transform::default(),
                Health {
                    current: 80.0,
                    max: 120.0,
                },
                Barrier {
                    current: 0.0,
                    max: 100.0,
                },
                Fury {
                    current: 50.0,
                    max: 100.0,
                    basic_gain: 10.0,
                    regen_per_sec: 0.0,
                    cost_discount: 0.0,
                },
                Talents::default(),
            ))
            .id();
        (app, hero)
    }

    #[test]
    fn warden_valor_unleashes_solar_cleave_and_grants_barrier() {
        let (mut app, hero) = mastery_app(HeroClass::Warden);
        let foe = app
            .world_mut()
            .spawn((
                crate::combat::tests::test_enemy(vec![]),
                Transform::from_translation(Vec3::Z * 1.5),
                Health {
                    current: 1000.0,
                    max: 1000.0,
                },
            ))
            .id();
        app.world_mut().resource_mut::<ClassMasteryState>().charges = WARDEN_MAX_CHARGES;
        app.world_mut().write_message(ClassCastSignal {
            origin: Vec3::ZERO,
            facing: Vec3::Z,
            target: Vec3::Z * 1.5,
            shape: StrikeShape::Arc { range: 2.2 },
            skill: None,
            movement: Movement::None,
            damage: 40.0,
            impact: 1.0,
            high_fury: false,
            low_health: false,
        });
        app.update();
        assert!(app.world().get::<Health>(foe).unwrap().current < 1000.0);
        assert!(app.world().get::<Barrier>(hero).unwrap().current >= 12.0);
        assert_eq!(app.world().resource::<ClassMasteryState>().charges, 0);
    }

    #[test]
    fn stalker_mobility_primes_focus_and_next_shot_launches_phantom_volley() {
        let (mut app, hero) = mastery_app(HeroClass::Stalker);
        app.world_mut().write_message(ClassCastSignal {
            origin: Vec3::ZERO,
            facing: Vec3::Z,
            target: Vec3::Z * 5.0,
            shape: StrikeShape::Projectiles {
                art: ProjectileArt::BoneArrow,
                count: 3,
                spread_deg: 26.0,
                speed: 26.0,
                range: 8.0,
                width: 0.4,
                pierce: false,
                blast_radius: 0.0,
            },
            skill: Some(SkillCast {
                slot: SkillSlot::Mobility,
                rune: Rune::Cleanse,
            }),
            movement: Movement::Away(3.4),
            damage: 20.0,
            impact: 0.9,
            high_fury: false,
            low_health: false,
        });
        app.update();
        assert_eq!(
            app.world().resource::<ClassMasteryState>().charges,
            STALKER_MAX_CHARGES
        );

        app.world_mut().write_message(ClassCastSignal {
            origin: Vec3::ZERO,
            facing: Vec3::Z,
            target: Vec3::Z * 6.0,
            shape: StrikeShape::Projectiles {
                art: ProjectileArt::BoneArrow,
                count: 1,
                spread_deg: 0.0,
                speed: 26.0,
                range: 10.0,
                width: 0.4,
                pierce: false,
                blast_radius: 0.0,
            },
            skill: None,
            movement: Movement::None,
            damage: 25.0,
            impact: 1.0,
            high_fury: false,
            low_health: false,
        });
        app.update();
        let spawned_arrows = app
            .world_mut()
            .query_filtered::<Entity, With<ranged::PlayerProjectile>>()
            .iter(app.world())
            .count();
        assert_eq!(spawned_arrows, 2);
        assert_eq!(app.world().resource::<ClassMasteryState>().charges, 0);
        assert_eq!(app.world().get::<Fury>(hero).unwrap().current, 60.0);
    }

    #[test]
    fn embercaller_overload_calls_three_meteors_and_refunds_arcana() {
        let (mut app, hero) = mastery_app(HeroClass::Embercaller);
        app.world_mut().resource_mut::<ClassMasteryState>().charges = EMBER_MAX_CHARGES;
        app.world_mut().write_message(ClassCastSignal {
            origin: Vec3::ZERO,
            facing: Vec3::Z,
            target: Vec3::Z * 5.0,
            shape: StrikeShape::GroundBlast {
                cast_range: 9.5,
                radius: 2.8,
                delay: 0.6,
            },
            skill: Some(SkillCast {
                slot: SkillSlot::Finisher,
                rune: Rune::Ember,
            }),
            movement: Movement::None,
            damage: 54.0,
            impact: 1.4,
            high_fury: false,
            low_health: false,
        });
        app.update();
        let meteors = app
            .world_mut()
            .query_filtered::<Entity, With<ranged::PlayerGroundBlast>>()
            .iter(app.world())
            .count();
        assert_eq!(meteors, 3);
        assert_eq!(app.world().resource::<ClassMasteryState>().charges, 0);
        assert_eq!(app.world().get::<Fury>(hero).unwrap().current, 60.0);
    }

    #[test]
    fn talent_synergies_upgrade_warden_stalker_and_embercaller_masteries() {
        // Warden Wrath >= 4 reduces cap to 2; Vigor >= 2 doubles barrier; Focus >= 2 refunds +18 Fury.
        let (mut w_app, w_hero) = mastery_app(HeroClass::Warden);
        *w_app.world_mut().get_mut::<Talents>(w_hero).unwrap() = Talents {
            points: 0,
            wrath: 4,
            vigor: 2,
            focus: 2,
        };
        assert_eq!(
            ClassMasteryState::max_charges_for(
                HeroClass::Warden,
                w_app.world().get::<Talents>(w_hero).unwrap()
            ),
            2
        );
        w_app
            .world_mut()
            .resource_mut::<ClassMasteryState>()
            .charges = 2;
        w_app.world_mut().write_message(ClassCastSignal {
            origin: Vec3::ZERO,
            facing: Vec3::Z,
            target: Vec3::Z * 1.5,
            shape: StrikeShape::Arc { range: 2.2 },
            skill: None,
            movement: Movement::None,
            damage: 40.0,
            impact: 1.0,
            high_fury: false,
            low_health: false,
        });
        w_app.update();
        assert_eq!(w_app.world().get::<Barrier>(w_hero).unwrap().current, 24.0);
        assert_eq!(w_app.world().get::<Fury>(w_hero).unwrap().current, 68.0);

        // Stalker Wrath >= 2 launches 4 arrows, Focus >= 2 reduces cap to 3, Focus >= 4 refunds +20 Malice.
        let (mut s_app, s_hero) = mastery_app(HeroClass::Stalker);
        *s_app.world_mut().get_mut::<Talents>(s_hero).unwrap() = Talents {
            points: 0,
            wrath: 2,
            vigor: 2,
            focus: 4,
        };
        assert_eq!(
            ClassMasteryState::max_charges_for(
                HeroClass::Stalker,
                s_app.world().get::<Talents>(s_hero).unwrap()
            ),
            3
        );
        s_app
            .world_mut()
            .resource_mut::<ClassMasteryState>()
            .charges = 3;
        s_app.world_mut().write_message(ClassCastSignal {
            origin: Vec3::ZERO,
            facing: Vec3::Z,
            target: Vec3::Z * 6.0,
            shape: StrikeShape::Projectiles {
                art: ProjectileArt::BoneArrow,
                count: 1,
                spread_deg: 0.0,
                speed: 26.0,
                range: 10.0,
                width: 0.4,
                pierce: false,
                blast_radius: 0.0,
            },
            skill: None,
            movement: Movement::None,
            damage: 25.0,
            impact: 1.0,
            high_fury: false,
            low_health: false,
        });
        s_app.update();
        let quad_arrows = s_app
            .world_mut()
            .query_filtered::<Entity, With<ranged::PlayerProjectile>>()
            .iter(s_app.world())
            .count();
        assert_eq!(quad_arrows, 4);
        assert_eq!(s_app.world().get::<Fury>(s_hero).unwrap().current, 70.0);

        // Embercaller Wrath >= 2 summons 5 meteors, Focus >= 2 drops cap to 2, Focus >= 4 refunds +22 Arcana, Vigor >= 4 grants barrier + heal.
        let (mut e_app, e_hero) = mastery_app(HeroClass::Embercaller);
        *e_app.world_mut().get_mut::<Talents>(e_hero).unwrap() = Talents {
            points: 0,
            wrath: 4,
            vigor: 4,
            focus: 4,
        };
        assert_eq!(
            ClassMasteryState::max_charges_for(
                HeroClass::Embercaller,
                e_app.world().get::<Talents>(e_hero).unwrap()
            ),
            2
        );
        e_app
            .world_mut()
            .resource_mut::<ClassMasteryState>()
            .charges = 2;
        e_app.world_mut().write_message(ClassCastSignal {
            origin: Vec3::ZERO,
            facing: Vec3::Z,
            target: Vec3::Z * 5.0,
            shape: StrikeShape::GroundBlast {
                cast_range: 9.5,
                radius: 2.8,
                delay: 0.6,
            },
            skill: Some(SkillCast {
                slot: SkillSlot::Finisher,
                rune: Rune::Ember,
            }),
            movement: Movement::None,
            damage: 54.0,
            impact: 1.4,
            high_fury: false,
            low_health: false,
        });
        e_app.update();
        let penta_meteors = e_app
            .world_mut()
            .query_filtered::<Entity, With<ranged::PlayerGroundBlast>>()
            .iter(e_app.world())
            .count();
        assert_eq!(penta_meteors, 5);
        assert_eq!(e_app.world().get::<Fury>(e_hero).unwrap().current, 72.0);
        assert_eq!(e_app.world().get::<Barrier>(e_hero).unwrap().current, 18.0);
        assert_eq!(e_app.world().get::<Health>(e_hero).unwrap().current, 88.0);
    }
}
