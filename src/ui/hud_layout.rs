//! Bottom console: one local coordinate space for its art, resources and actions.
use super::*;
use bevy::ui::Val2;

const CONSOLE_WIDTH: f32 = 1040.0;
const CONSOLE_HEIGHT: f32 = 184.0;
const ORB_SIZE: f32 = 170.0;
const APERTURE_SIZE: f32 = 100.0;
const ORB_INSET: f32 = 12.0;

#[derive(Component)]
pub(super) struct BottomConsole;

pub(super) fn fit_bottom_console(
    windows: Query<&Window, With<PrimaryWindow>>,
    mut consoles: Query<&mut UiTransform, With<BottomConsole>>,
) {
    let Ok(window) = windows.single() else {
        return;
    };
    let scale = console_scale(window.width());
    for mut transform in &mut consoles {
        transform.scale = Vec2::splat(scale);
        transform.translation = Val2::px(0.0, CONSOLE_HEIGHT * (1.0 - scale) * 0.5);
    }
}

fn console_scale(viewport_width: f32) -> f32 {
    ((viewport_width - 24.0).max(1.0) / CONSOLE_WIDTH).min(1.0)
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
                            SkillSlotFrame(slot),
                            Interaction::default(),
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
        .spawn((Node {
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
        },))
        .with_children(|orb| {
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
        .spawn((Node {
            position_type: PositionType::Absolute,
            left: Val::Percent(50.0),
            bottom: Val::Px(121.0),
            width: Val::Px(208.0),
            height: Val::Px(42.0),
            margin: UiRect::left(Val::Px(-104.0)),
            column_gap: Val::Px(12.0),
            justify_content: JustifyContent::Center,
            ..default()
        },))
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
                        Node {
                            width: Val::Px(40.0),
                            height: Val::Px(42.0),
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
            let scale = console_scale(width);
            let left = (width - CONSOLE_WIDTH * scale) * 0.5;
            let right_orb = left + (CONSOLE_WIDTH - ORB_INSET - ORB_SIZE) * scale;
            assert!(left + ORB_INSET * scale >= left);
            assert!(right_orb + ORB_SIZE * scale <= left + CONSOLE_WIDTH * scale);
            // The action strip and both orbs occupy disjoint horizontal regions.
            let skill_left = left + (CONSOLE_WIDTH - 640.0) * 0.5 * scale;
            assert!(left + (ORB_INSET + ORB_SIZE) * scale < skill_left);
            assert_eq!(scale, 1.0);
        }
        assert!(CONSOLE_WIDTH * console_scale(600.0) <= 576.0);
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
