//! Bottom console: one local coordinate space for its art, resources and actions.
use super::*;
use bevy::ui::Val2;

const CONSOLE_WIDTH: f32 = 1040.0;
const CONSOLE_HEIGHT: f32 = 184.0;
const ORB_SIZE: f32 = 170.0;
const APERTURE_SIZE: f32 = 100.0;
const ORB_INSET: f32 = 12.0;
const SKILL_BAR_WIDTH: f32 = 606.0;
const STRIP_CARD_HEIGHT: f32 = 42.0;
/// Smallest on-screen edge of anything meant to be tapped.
const TOUCH_TARGET: f32 = 46.0;
// Stacked portrait orbs must leave the middle free for the utility strip.
const _: () = assert!(SKILL_BAR_WIDTH - 2.0 * ORB_SIZE >= 208.0);

#[derive(Component)]
pub(super) struct BottomConsole;

#[derive(Component)]
pub(super) struct ConsoleOrb {
    health: bool,
}

#[derive(Component)]
pub(super) struct StatusStrip;

/// Scales the console to the viewport. A portrait screen is too narrow for
/// orb + skills + orb in one row, so the orbs stack above the skill bar there.
pub(super) fn fit_bottom_console(
    viewport: Res<HudViewport>,
    windows: Query<&Window, With<PrimaryWindow>>,
    mut consoles: Query<&mut UiTransform, With<BottomConsole>>,
    mut strips: Query<&mut UiTransform, (With<StatusStrip>, Without<BottomConsole>)>,
    mut orbs: Query<(&ConsoleOrb, &mut Node)>,
) {
    let Ok(window) = windows.single() else {
        return;
    };
    let scale = console_scale(*viewport, window.width());
    for mut transform in &mut consoles {
        transform.scale = Vec2::splat(scale);
        transform.translation = Val2::px(0.0, CONSOLE_HEIGHT * (1.0 - scale) * 0.5);
    }
    let strip = strip_scale(*viewport, scale);
    for mut transform in &mut strips {
        transform.scale = Vec2::splat(strip);
        // Grow upward from the strip's bottom edge.
        transform.translation = Val2::px(0.0, -STRIP_CARD_HEIGHT * (strip - 1.0) * 0.5);
    }
    let (inset, bottom) = orb_placement(*viewport, strip);
    for (orb, mut node) in &mut orbs {
        let mut placed = node.clone();
        placed.bottom = Val::Px(bottom);
        if orb.health {
            placed.left = Val::Px(inset);
        } else {
            placed.right = Val::Px(inset);
        }
        node.set_if_neq(placed);
    }
}

fn console_scale(viewport: HudViewport, viewport_width: f32) -> f32 {
    let (margin, content) = match viewport {
        HudViewport::Portrait => (12.0, SKILL_BAR_WIDTH),
        HudViewport::Desktop | HudViewport::Landscape => (24.0, CONSOLE_WIDTH),
    };
    ((viewport_width - margin).max(1.0) / content).min(1.0)
}

/// Utility cards are small art; on touch layouts they grow to a tappable size.
fn strip_scale(viewport: HudViewport, console_scale: f32) -> f32 {
    if viewport.compact() {
        (TOUCH_TARGET / (STRIP_CARD_HEIGHT * console_scale)).max(1.0)
    } else {
        1.0
    }
}

/// Orb inset from the console edge and height above its bottom, in console units.
fn orb_placement(viewport: HudViewport, strip_scale: f32) -> (f32, f32) {
    match viewport {
        HudViewport::Portrait => (
            (CONSOLE_WIDTH - SKILL_BAR_WIDTH) * 0.5,
            121.0 + STRIP_CARD_HEIGHT * strip_scale + 6.0,
        ),
        HudViewport::Desktop | HudViewport::Landscape => (ORB_INSET, 16.0),
    }
}

/// On-screen height the console occupies above the window's bottom edge.
pub(super) fn console_top(viewport: HudViewport, viewport_width: f32) -> f32 {
    let scale = console_scale(viewport, viewport_width);
    let (_, orb_bottom) = orb_placement(viewport, strip_scale(viewport, scale));
    4.0 + (orb_bottom + ORB_SIZE) * scale
}

/// Tapping a skill or utility slot presses its key, so the console is usable
/// without a keyboard. Runs before gameplay reads input this frame.
///
/// Slots are deliberately not `Button`s: hovering a button sets
/// `UiPointerCapture`, which suspends all combat input, and a cursor resting on
/// the skill bar must not swallow Q/E/R. Instead the click that pressed the
/// slot is withdrawn from the mouse so it cannot also start a basic attack.
pub(super) fn press_console_slots(
    slots: Query<(Ref<Interaction>, &SkillSlotFrame)>,
    utilities: Query<(Ref<Interaction>, &UtilitySlot)>,
    mut keyboard: ResMut<ButtonInput<KeyCode>>,
    mut mouse: ResMut<ButtonInput<MouseButton>>,
    mut held: Local<Vec<KeyCode>>,
) {
    for key in held.drain(..) {
        keyboard.release(key);
    }
    let skills = slots
        .iter()
        .filter(|(interaction, _)| {
            interaction.is_changed() && **interaction == Interaction::Pressed
        })
        .filter_map(|(_, frame)| skill_slot_key(frame.0));
    let items = utilities
        .iter()
        .filter(|(interaction, _)| {
            interaction.is_changed() && **interaction == Interaction::Pressed
        })
        .map(|(_, slot)| utility_slot_key(slot.0));
    for key in skills.chain(items) {
        keyboard.press(key);
        held.push(key);
    }
    if !held.is_empty() {
        mouse.reset(MouseButton::Left);
    }
}

/// The basic strike has no key: it is aimed by clicking the world.
fn skill_slot_key(slot: SkillSlotText) -> Option<KeyCode> {
    match slot {
        SkillSlotText::Strike => None,
        SkillSlotText::Finisher => Some(crate::class::SkillSlot::Finisher.key()),
        SkillSlotText::Mobility => Some(crate::class::SkillSlot::Mobility.key()),
        SkillSlotText::Area => Some(crate::class::SkillSlot::Area.key()),
        SkillSlotText::Surge => Some(KeyCode::KeyV),
        SkillSlotText::Evade => Some(KeyCode::ShiftLeft),
    }
}

fn utility_slot_key(slot: CompactUtilityText) -> KeyCode {
    match slot {
        CompactUtilityText::Potion => KeyCode::KeyF,
        CompactUtilityText::Elixir => KeyCode::KeyG,
        CompactUtilityText::Portal => KeyCode::KeyT,
        CompactUtilityText::Interact => KeyCode::Space,
    }
}

pub(super) fn spawn_bottom_hud(parent: &mut ChildSpawnerCommands, asset_server: &AssetServer) {
    let regular_font: Handle<Font> = asset_server.load(UI_FONT_REGULAR);
    let bold_font: Handle<Font> = asset_server.load(UI_FONT_BOLD);
    parent
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                left: Val::Percent(50.0),
                bottom: Val::Px(4.0),
                width: Val::Px(CONSOLE_WIDTH),
                height: Val::Px(CONSOLE_HEIGHT),
                margin: UiRect::left(Val::Px(-CONSOLE_WIDTH * 0.5)),
                ..default()
            },
            BottomConsole,
            UiTransform::default(),
            Pickable::IGNORE,
        ))
        .with_children(|parent| {
            parent
                .spawn((
                    Node {
                        position_type: PositionType::Absolute,
                        left: Val::Percent(50.0),
                        bottom: Val::Px(13.0),
                        width: Val::Px(584.0),
                        height: Val::Px(4.0),
                        margin: UiRect::left(Val::Px(-292.0)),
                        padding: UiRect::ZERO,
                        border: UiRect::all(Val::Px(1.0)),
                        overflow: Overflow::clip(),
                        ..default()
                    },
                    BorderColor::all(Color::srgba(0.80, 0.58, 0.30, 0.78)),
                    BackgroundColor(Color::srgba(0.018, 0.014, 0.018, 0.92)),
                ))
                .with_children(|bar| {
                    bar.spawn((
                        Node {
                            width: Val::Percent(0.0),
                            height: Val::Percent(100.0),
                            ..default()
                        },
                        BackgroundColor(Color::srgba(0.86, 0.46, 0.10, 0.92)),
                        ExperienceFill,
                    ));
                });
            spawn_status_strip(parent, asset_server);
            spawn_orb(parent, asset_server, true);
            spawn_orb(parent, asset_server, false);
            parent
                .spawn((
                    Node {
                        position_type: PositionType::Absolute,
                        left: Val::Percent(50.0),
                        bottom: Val::Px(24.0),
                        width: Val::Px(606.0),
                        height: Val::Px(96.0),
                        margin: UiRect::left(Val::Px(-303.0)),
                        padding: UiRect::all(Val::Px(8.0)),
                        display: Display::Flex,
                        column_gap: Val::Px(8.0),
                        justify_content: JustifyContent::Center,
                        align_items: AlignItems::Center,
                        ..default()
                    },
                    Pickable::IGNORE,
                ))
                .with_children(|bar| {
                    for slot in [
                        SkillSlotText::Strike,
                        SkillSlotText::Finisher,
                        SkillSlotText::Mobility,
                        SkillSlotText::Area,
                        SkillSlotText::Surge,
                        SkillSlotText::Evade,
                    ] {
                        let theme = skill_slot_theme(slot);
                        bar.spawn((
                            Node {
                                width: Val::Px(90.0),
                                height: Val::Px(78.0),
                                padding: UiRect::all(Val::Px(4.0)),
                                border: UiRect::all(Val::Px(2.0)),
                                border_radius: BorderRadius::all(Val::Px(6.0)),
                                justify_content: JustifyContent::Center,
                                align_items: AlignItems::Center,
                                overflow: Overflow::clip(),
                                ..default()
                            },
                            BorderColor::all(Color::srgba(0.92, 0.74, 0.42, 0.62)),
                            BackgroundColor(Color::srgba(0.055, 0.047, 0.058, 0.92)),
                            slot_frame_image_node(asset_server),
                            Interaction::default(),
                            SkillSlotFrame(slot),
                        ))
                        .with_children(|slot_node| {
                            slot_node.spawn((
                                atlas_image_node(
                                    asset_server,
                                    UI_SKILL_ICONS_IMAGE,
                                    atlas_rect(
                                        UI_SKILL_ICON_CELL,
                                        UI_SKILL_ICON_CELL,
                                        3,
                                        skill_icon_index(slot),
                                    ),
                                ),
                                Node {
                                    position_type: PositionType::Absolute,
                                    left: Val::Px(15.0),
                                    right: Val::Px(15.0),
                                    top: Val::Px(7.0),
                                    height: Val::Px(56.0),
                                    ..default()
                                },
                                SkillSlotIcon(slot),
                                Pickable::IGNORE,
                            ));
                            slot_node.spawn((
                                Node {
                                    position_type: PositionType::Absolute,
                                    left: Val::Px(0.0),
                                    right: Val::Px(0.0),
                                    bottom: Val::Px(0.0),
                                    height: Val::Percent(0.0),
                                    ..default()
                                },
                                BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.62)),
                                SkillCooldownMask(slot),
                                Pickable::IGNORE,
                            ));
                            slot_node.spawn((
                                Node {
                                    position_type: PositionType::Absolute,
                                    left: Val::Px(5.0),
                                    top: Val::Px(4.0),
                                    width: Val::Px(25.0),
                                    height: Val::Px(16.0),
                                    border: UiRect::all(Val::Px(1.0)),
                                    justify_content: JustifyContent::Center,
                                    align_items: AlignItems::Center,
                                    ..default()
                                },
                                BorderColor::all(theme.idle_border),
                                BackgroundColor(Color::srgba(0.012, 0.010, 0.012, 0.78)),
                                Pickable::IGNORE,
                                Text::new(skill_slot_keycap(slot)),
                                TextFont {
                                    font: bold_font.clone().into(),
                                    font_size: FontSize::Px(skill_slot_keycap_font_size(slot)),
                                    ..default()
                                },
                                TextColor(Color::srgb(0.96, 0.90, 0.76)),
                            ));
                            slot_node.spawn((
                                Node {
                                    position_type: PositionType::Absolute,
                                    right: Val::Px(6.0),
                                    top: Val::Px(7.0),
                                    width: Val::Px(10.0),
                                    height: Val::Px(5.0),
                                    ..default()
                                },
                                BackgroundColor(Color::srgba(0.22, 0.18, 0.12, 0.74)),
                                SkillSlotLamp(slot),
                                Pickable::IGNORE,
                            ));
                            slot_node.spawn((
                                Node {
                                    position_type: PositionType::Absolute,
                                    left: Val::Px(3.0),
                                    right: Val::Px(3.0),
                                    bottom: Val::Px(4.0),
                                    justify_content: JustifyContent::Center,
                                    align_items: AlignItems::Center,
                                    ..default()
                                },
                                Text::new(""),
                                TextFont {
                                    font: regular_font.clone().into(),
                                    font_size: FontSize::Px(10.0),
                                    ..default()
                                },
                                TextColor(Color::srgb(0.92, 0.86, 0.72)),
                                BackgroundColor(Color::srgba(0.018, 0.014, 0.010, 0.88)),
                                TextLayout::justify(Justify::Center),
                                CooldownText,
                                Pickable::IGNORE,
                                slot,
                            ));
                        });
                    }
                    bar.spawn((
                        SkillTooltip,
                        Node {
                            position_type: PositionType::Absolute,
                            bottom: Val::Px(104.0),
                            width: Val::Px(280.0),
                            padding: UiRect::all(Val::Px(12.0)),
                            display: Display::None,
                            ..default()
                        },
                        Text::new(""),
                        TextFont {
                            font_size: FontSize::Px(14.0),
                            ..default()
                        },
                        TextColor(Color::srgb(0.93, 0.84, 0.66)),
                        BackgroundColor(Color::srgba(0.025, 0.021, 0.019, 0.97)),
                        GlobalZIndex(35),
                        Pickable::IGNORE,
                    ));
                });
        });
}

#[derive(Component)]
pub(super) struct SkillTooltip;

pub(super) fn update_skill_tooltips(
    language: Res<localization::UiLanguage>,
    catalog: Res<ClassCatalog>,
    selected: Res<SelectedClass>,
    loadouts: Query<(&SkillLoadout, &PlayerLevel), With<Player>>,
    slots: Query<(&Interaction, &SkillSlotFrame)>,
    mut tips: Query<(&mut Node, &mut Text), With<SkillTooltip>>,
) {
    let hovered = slots
        .iter()
        .find_map(|(interaction, frame)| (*interaction != Interaction::None).then_some(frame.0));
    let zh = *language == localization::UiLanguage::Chinese;
    let class = catalog.get(selected.0);
    for (mut node, mut text) in &mut tips {
        node.display = if hovered.is_some() {
            Display::Flex
        } else {
            Display::None
        };
        let Some(slot) = hovered else {
            continue;
        };
        **text = match (slot.class_slot(), loadouts.single().ok()) {
            (Some(class_slot), Some((loadout, level))) => {
                let (skill, rune) = class.equipped(loadout, class_slot);
                skill_tooltip_text(skill, rune, level.level, slot.key(), zh)
            }
            _ => {
                let (name, english, chinese) = match slot {
                    SkillSlotText::Strike => (
                        if zh {
                            class.basic.name_zh.as_str()
                        } else {
                            class.basic.name.as_str()
                        },
                        "Basic attack. Builds your resource when it connects.",
                        "普通攻击，命中时积攒资源。",
                    ),
                    SkillSlotText::Surge => (
                        if zh { "蓄能" } else { "Surge" },
                        "Slay enemies to charge a burst of power.",
                        "击杀敌人积蓄能量，释放力量爆发。",
                    ),
                    _ => (
                        if zh { "闪避" } else { "Evade" },
                        "Dodge incoming attacks.",
                        "闪避来袭攻击。",
                    ),
                };
                format!(
                    "{name} [{}]\n{}",
                    slot.key(),
                    if zh { chinese } else { english }
                )
            }
        };
    }
}

/// What each orb measures; the number inside it carries the current value.
fn orb_tooltip_text(health: bool) -> &'static str {
    if health {
        "Health\nDrink a potion [F] before it runs dry"
    } else {
        "Class resource\nSpent by your Q, E and R skills"
    }
}

/// Name, key, cost line and the equipped rune for one class skill.
pub(super) fn skill_tooltip_text(
    skill: &crate::class::SkillDef,
    rune: crate::class::Rune,
    level: u32,
    key: &str,
    zh: bool,
) -> String {
    let name = if zh { &skill.name_zh } else { &skill.name };
    let summary = if zh {
        &skill.summary_zh
    } else {
        &skill.summary
    };
    let rune_name = if zh { rune.label_zh() } else { rune.label() };
    if level < skill.unlock_level {
        return if zh {
            format!("{name} [{key}]\n{} 级解锁", skill.unlock_level)
        } else {
            format!("{name} [{key}]\nUnlocks at level {}", skill.unlock_level)
        };
    }
    format!("{name} [{key}] · {rune_name}\n{summary}")
}

fn spawn_orb(parent: &mut ChildSpawnerCommands, assets: &AssetServer, health: bool) {
    let frame = if health {
        ResourceOrbFrame::Health
    } else {
        ResourceOrbFrame::Fury
    };
    parent
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                left: if health {
                    Val::Px(ORB_INSET)
                } else {
                    Val::Auto
                },
                right: if health {
                    Val::Auto
                } else {
                    Val::Px(ORB_INSET)
                },
                bottom: Val::Px(16.0),
                width: Val::Px(ORB_SIZE),
                height: Val::Px(ORB_SIZE),
                ..default()
            },
            ConsoleOrb { health },
            Interaction::default(),
            UtilityHover,
        ))
        .with_children(|orb| {
            orb.spawn((
                Text::new(orb_tooltip_text(health)),
                TextFont {
                    font: assets.load(UI_FONT_REGULAR).into(),
                    font_size: FontSize::Px(14.0),
                    ..default()
                },
                TextColor(Color::srgb(0.93, 0.85, 0.71)),
                BackgroundColor(Color::srgba(0.025, 0.021, 0.019, 0.97)),
                BorderColor::all(Color::srgba(0.61, 0.46, 0.27, 0.9)),
                Node {
                    position_type: PositionType::Absolute,
                    bottom: Val::Px(ORB_SIZE - 6.0),
                    left: if health { Val::Px(0.0) } else { Val::Auto },
                    right: if health { Val::Auto } else { Val::Px(0.0) },
                    width: Val::Px(220.0),
                    padding: UiRect::all(Val::Px(10.0)),
                    border: UiRect::all(Val::Px(1.0)),
                    display: Display::None,
                    ..default()
                },
                ZIndex(30),
                UtilityTooltip,
                Pickable::IGNORE,
            ));
            orb.spawn((
                Node {
                    position_type: PositionType::Absolute,
                    left: Val::Px((ORB_SIZE - APERTURE_SIZE) * 0.5),
                    top: Val::Px((ORB_SIZE - APERTURE_SIZE) * 0.5),
                    width: Val::Px(APERTURE_SIZE),
                    height: Val::Px(APERTURE_SIZE),
                    border_radius: BorderRadius::MAX,
                    ..default()
                },
                BackgroundColor(Color::srgb(0.025, 0.015, 0.018)),
                BorderColor::all(Color::NONE),
                frame,
            ))
            .with_children(|aperture| {
                // Clip the bottom of a FIXED circle. Scaling the circle's height
                // would turn partially filled resources into detached ellipses.
                aperture
                    .spawn((
                        resource_mask_node(100.0),
                        BackgroundColor(Color::NONE),
                        if health {
                            ResourceFill::Health
                        } else {
                            ResourceFill::Fury
                        },
                    ))
                    .with_children(|mask| {
                        if health {
                            mask.spawn((
                                resource_liquid_node(),
                                BackgroundColor(Color::srgb(0.63, 0.025, 0.035)),
                            ));
                        } else {
                            mask.spawn((
                                resource_liquid_node(),
                                BackgroundColor(Color::srgb(0.025, 0.21, 0.65)),
                                ResourceLiquid,
                            ));
                        }
                    });
            });
            orb.spawn((
                ImageNode {
                    image: assets.load(UI_RESOURCE_ORB_FRAME_IMAGE),
                    color: if health {
                        Color::WHITE
                    } else {
                        Color::srgb(0.72, 0.82, 1.0)
                    },
                    ..default()
                },
                Node {
                    position_type: PositionType::Absolute,
                    width: Val::Px(ORB_SIZE),
                    height: Val::Px(ORB_SIZE),
                    ..default()
                },
                ResourceOrbArt,
                Pickable::IGNORE,
            ));
            // A flex wrapper centers the text block; Text on the wrapper itself
            // only aligns glyphs horizontally and had pinned labels to the top.
            orb.spawn((
                Node {
                    position_type: PositionType::Absolute,
                    width: Val::Px(ORB_SIZE),
                    height: Val::Px(ORB_SIZE),
                    justify_content: JustifyContent::Center,
                    align_items: AlignItems::Center,
                    ..default()
                },
                ZIndex(2),
                Pickable::IGNORE,
            ))
            .with_children(|label| {
                let mut text = label.spawn((
                    Text::new(""),
                    TextFont {
                        font: assets.load(UI_FONT_BOLD).into(),
                        font_size: FontSize::Px(13.0),
                        ..default()
                    },
                    TextLayout::justify(Justify::Center),
                    TextColor(if health {
                        Color::srgb(1.0, 0.86, 0.76)
                    } else {
                        Color::srgb(0.82, 0.90, 1.0)
                    }),
                    Pickable::IGNORE,
                ));
                if health {
                    text.insert(HealthText);
                } else {
                    text.insert(StatsText);
                }
            });
            if health {
                orb.spawn((Node {
                    position_type: PositionType::Absolute,
                    left: Val::Px(35.0),
                    bottom: Val::Px(12.0),
                    width: Val::Px(100.0),
                    height: Val::Px(4.0),
                    overflow: Overflow::clip(),
                    ..default()
                },))
                    .with_children(|ward| {
                        ward.spawn((
                            Node {
                                width: Val::Percent(0.0),
                                height: Val::Px(4.0),
                                ..default()
                            },
                            BackgroundColor(Color::NONE),
                            ResourceFill::Barrier,
                        ));
                    });
                orb.spawn((Node {
                    position_type: PositionType::Absolute,
                    left: Val::Px(0.0),
                    bottom: Val::Px(-3.0),
                    width: Val::Percent(100.0),
                    height: Val::Px(9.0),
                    column_gap: Val::Px(3.0),
                    justify_content: JustifyContent::Center,
                    ..default()
                },))
                    .with_children(|pips| {
                        for index in 0..8 {
                            pips.spawn((
                                Node {
                                    width: Val::Px(8.0),
                                    height: Val::Px(8.0),
                                    border: UiRect::all(Val::Px(1.0)),
                                    ..default()
                                },
                                BorderColor::all(Color::NONE),
                                BackgroundColor(Color::NONE),
                                PotionPip(index),
                            ));
                        }
                    });
            }
        });
}

fn resource_mask_node(percent: f32) -> Node {
    Node {
        position_type: PositionType::Absolute,
        left: Val::Px(0.0),
        bottom: Val::Px(0.0),
        width: Val::Percent(100.0),
        height: Val::Percent(percent),
        overflow: Overflow::clip(),
        ..default()
    }
}

fn resource_liquid_node() -> Node {
    Node {
        position_type: PositionType::Absolute,
        left: Val::Px(0.0),
        bottom: Val::Px(0.0),
        width: Val::Px(APERTURE_SIZE),
        height: Val::Px(APERTURE_SIZE),
        border_radius: BorderRadius::MAX,
        ..default()
    }
}

#[derive(Component)]
pub(super) struct UtilityTooltip;

#[derive(Component)]
pub(super) struct UtilityHover;

#[derive(Component)]
pub(super) struct UtilitySlot(CompactUtilityText);

pub(super) fn update_utility_tooltips(
    buttons: Query<(&Interaction, &Children), With<UtilityHover>>,
    mut tips: Query<&mut Node, With<UtilityTooltip>>,
) {
    for (interaction, children) in &buttons {
        for child in children.iter() {
            if let Ok(mut node) = tips.get_mut(child) {
                node.display = if *interaction == Interaction::None {
                    Display::None
                } else {
                    Display::Flex
                };
            }
        }
    }
}

fn spawn_status_strip(parent: &mut ChildSpawnerCommands, assets: &AssetServer) {
    parent
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                left: Val::Percent(50.0),
                bottom: Val::Px(121.0),
                width: Val::Px(208.0),
                height: Val::Px(STRIP_CARD_HEIGHT),
                margin: UiRect::left(Val::Px(-104.0)),
                column_gap: Val::Px(12.0),
                justify_content: JustifyContent::Center,
                ..default()
            },
            StatusStrip,
            UiTransform::default(),
        ))
        .with_children(|strip| {
            for (slot, key) in [
                (CompactUtilityText::Potion, "F"),
                (CompactUtilityText::Elixir, "G"),
                (CompactUtilityText::Portal, "T"),
                (CompactUtilityText::Interact, "Space"),
            ] {
                strip
                    .spawn((
                        Interaction::default(),
                        UtilityHover,
                        UtilitySlot(slot),
                        Node {
                            width: Val::Px(40.0),
                            height: Val::Px(STRIP_CARD_HEIGHT),
                            flex_shrink: 0.0,
                            justify_content: JustifyContent::Center,
                            align_items: AlignItems::Center,
                            ..default()
                        },
                        slot_frame_image_node(assets),
                    ))
                    .with_children(|card| {
                        if matches!(
                            slot,
                            CompactUtilityText::Potion | CompactUtilityText::Elixir
                        ) {
                            spawn_flask_icon(card, slot == CompactUtilityText::Potion);
                        } else {
                            card.spawn((
                                icons::ui_icon(
                                    assets,
                                    if slot == CompactUtilityText::Portal {
                                        icons::UiIcon::Talents
                                    } else {
                                        icons::UiIcon::Journal
                                    },
                                ),
                                Node {
                                    width: Val::Px(30.0),
                                    height: Val::Px(30.0),
                                    margin: UiRect::bottom(Val::Px(6.0)),
                                    ..default()
                                },
                                Pickable::IGNORE,
                            ));
                        }
                        card.spawn((
                            Text::new(key),
                            TextFont {
                                font: assets.load(UI_FONT_BOLD).into(),
                                font_size: FontSize::Px(if key == "Space" { 8.0 } else { 10.0 }),
                                ..default()
                            },
                            TextColor(Color::srgb(0.94, 0.85, 0.68)),
                            Node {
                                position_type: PositionType::Absolute,
                                bottom: Val::Px(0.0),
                                ..default()
                            },
                            Pickable::IGNORE,
                        ));
                        card.spawn((
                            Text::new(""),
                            TextFont {
                                font: assets.load(UI_FONT_REGULAR).into(),
                                font_size: FontSize::Px(13.0),
                                ..default()
                            },
                            TextColor(Color::srgb(0.93, 0.85, 0.71)),
                            BackgroundColor(Color::srgba(0.025, 0.021, 0.019, 0.97)),
                            BorderColor::all(Color::srgba(0.61, 0.46, 0.27, 0.9)),
                            Node {
                                position_type: PositionType::Absolute,
                                bottom: Val::Px(49.0),
                                width: Val::Px(184.0),
                                padding: UiRect::all(Val::Px(10.0)),
                                border: UiRect::all(Val::Px(1.0)),
                                display: Display::None,
                                ..default()
                            },
                            ZIndex(30),
                            UtilityTooltip,
                            slot,
                            Pickable::IGNORE,
                        ));
                    });
            }
        });
}

fn spawn_flask_icon(parent: &mut ChildSpawnerCommands, health: bool) {
    parent
        .spawn((
            Node {
                width: Val::Px(18.0),
                height: Val::Px(21.0),
                margin: UiRect::bottom(Val::Px(3.0)),
                border: UiRect::all(Val::Px(2.0)),
                border_radius: BorderRadius::all(Val::Px(7.0)),
                ..default()
            },
            BorderColor::all(Color::srgb(0.67, 0.54, 0.34)),
            BackgroundColor(if health {
                Color::srgb(0.48, 0.045, 0.06)
            } else {
                Color::srgb(0.12, 0.36, 0.22)
            }),
            Pickable::IGNORE,
        ))
        .with_children(|flask| {
            flask.spawn((
                Node {
                    position_type: PositionType::Absolute,
                    left: Val::Px(3.0),
                    top: Val::Px(-6.0),
                    width: Val::Px(8.0),
                    height: Val::Px(7.0),
                    border: UiRect::all(Val::Px(1.0)),
                    ..default()
                },
                BackgroundColor(Color::srgb(0.26, 0.19, 0.12)),
                BorderColor::all(Color::srgb(0.74, 0.61, 0.39)),
                Pickable::IGNORE,
            ));
            flask.spawn((
                Node {
                    position_type: PositionType::Absolute,
                    left: Val::Px(3.0),
                    top: Val::Px(3.0),
                    width: Val::Px(3.0),
                    height: Val::Px(8.0),
                    border_radius: BorderRadius::MAX,
                    ..default()
                },
                BackgroundColor(Color::srgba(1.0, 0.91, 0.76, 0.34)),
                Pickable::IGNORE,
            ));
        });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn console_keeps_orbs_in_shared_coordinates_at_both_desktop_sizes() {
        for width in [1280.0, 2048.0] {
            let scale = console_scale(HudViewport::Desktop, width);
            let left = (width - CONSOLE_WIDTH * scale) * 0.5;
            let right_orb = left + (CONSOLE_WIDTH - ORB_INSET - ORB_SIZE) * scale;
            assert!(left + ORB_INSET * scale >= left);
            assert!(right_orb + ORB_SIZE * scale <= left + CONSOLE_WIDTH * scale);
            // The action strip and both orbs occupy disjoint horizontal regions.
            let skill_left = left + (CONSOLE_WIDTH - 640.0) * 0.5 * scale;
            assert!(left + (ORB_INSET + ORB_SIZE) * scale < skill_left);
            assert_eq!(scale, 1.0);
        }
        assert!(CONSOLE_WIDTH * console_scale(HudViewport::Desktop, 600.0) <= 576.0);
    }

    #[test]
    fn compact_consoles_keep_every_tappable_slot_touch_sized() {
        for (width, height) in [
            (844.0, 390.0),
            (667.0, 375.0),
            (390.0, 844.0),
            (360.0, 740.0),
        ] {
            let viewport = HudViewport::classify(width, height);
            assert!(viewport.compact());
            let scale = console_scale(viewport, width);
            // Skill slots are 90x78 console units.
            assert!(
                78.0 * scale >= 44.0,
                "{width}x{height}: skill slot too small"
            );
            let strip = strip_scale(viewport, scale);
            assert!(STRIP_CARD_HEIGHT * strip * scale >= TOUCH_TARGET - 0.01);
            // The enlarged strip still fits between or below the orbs.
            assert!(208.0 * strip <= SKILL_BAR_WIDTH);
            assert!(console_top(viewport, width) < height * 0.5);
        }
    }

    #[test]
    fn portrait_console_stacks_orbs_above_the_skill_bar_inside_the_viewport() {
        let width = 390.0;
        let viewport = HudViewport::classify(width, 844.0);
        assert_eq!(viewport, HudViewport::Portrait);
        let scale = console_scale(viewport, width);
        let strip = strip_scale(viewport, scale);
        let (inset, bottom) = orb_placement(viewport, strip);
        let console_left = (width - CONSOLE_WIDTH * scale) * 0.5;
        assert!(console_left + inset * scale >= 0.0);
        assert!(console_left + (CONSOLE_WIDTH - inset) * scale <= width);
        assert!(bottom >= 121.0 + STRIP_CARD_HEIGHT * strip);
    }

    #[test]
    fn only_keyed_slots_inject_a_key() {
        assert_eq!(skill_slot_key(SkillSlotText::Strike), None);
        assert_eq!(skill_slot_key(SkillSlotText::Mobility), Some(KeyCode::KeyQ));
        assert_eq!(skill_slot_key(SkillSlotText::Finisher), Some(KeyCode::KeyR));
        assert_eq!(utility_slot_key(CompactUtilityText::Potion), KeyCode::KeyF);
    }

    #[test]
    fn partial_liquid_masks_do_not_resize_the_circular_surface() {
        let liquid = resource_liquid_node();
        assert_eq!(liquid.width, Val::Px(APERTURE_SIZE));
        assert_eq!(liquid.height, Val::Px(APERTURE_SIZE));
        assert_eq!(liquid.bottom, Val::Px(0.0));
        for amount in [0.0, 35.0, 100.0] {
            let mask = resource_mask_node(amount);
            assert_eq!(mask.height, Val::Percent(amount));
            assert_eq!(mask.overflow, Overflow::clip());
            assert_eq!(mask.bottom, liquid.bottom);
        }
    }
}
