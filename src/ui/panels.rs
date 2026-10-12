//! Inventory and talent panels share a compact, scrollable presentation.
use super::*;
use bevy::input::mouse::{MouseScrollUnit, MouseWheel};

#[derive(Component)]
pub(super) struct PanelScroll;
#[derive(Component)]
pub(super) struct PanelClose(bool);

pub(super) fn spawn_panels(parent: &mut ChildSpawnerCommands, asset_server: &AssetServer) {
    let regular_font: Handle<Font> = asset_server.load(UI_FONT_REGULAR);
    let bold_font: Handle<Font> = asset_server.load(UI_FONT_BOLD);
    parent
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                left: Val::Px(0.0),
                right: Val::Px(0.0),
                margin: UiRect::horizontal(Val::Auto),
                top: Val::Px(30.0),
                width: Val::Px(760.0),
                max_width: Val::Percent(92.0),
                height: Val::Px(600.0),
                max_height: Val::Percent(88.0),
                padding: UiRect::all(Val::Px(20.0)),
                border: UiRect::all(Val::Px(1.0)),
                display: Display::None,
                flex_direction: FlexDirection::Column,
                row_gap: Val::Px(16.0),
                overflow: Overflow::scroll_y(),
                ..default()
            },
            BorderColor::all(Color::srgba(0.44, 0.60, 0.80, 0.72)),
            BackgroundColor(Color::srgb(0.010, 0.014, 0.020)),
            GlobalZIndex(40),
            InventoryPanel,
            PanelScroll,
            ScrollPosition::default(),
        ))
        .with_children(|panel| {
            panel
                .spawn((Node {
                    width: Val::Percent(100.0),
                    min_height: Val::Px(58.0),
                    flex_shrink: 0.0,
                    flex_wrap: FlexWrap::Wrap,
                    display: Display::Flex,
                    align_items: AlignItems::Center,
                    column_gap: Val::Px(12.0),
                    ..default()
                },))
                .with_children(|header| {
                    header.spawn((
                        atlas_image_node(
                            asset_server,
                            UI_GEAR_ICONS_IMAGE,
                            atlas_rect(UI_GEAR_ICON_CELL, UI_GEAR_ICON_CELL, 4, 7),
                        ),
                        Node {
                            width: Val::Px(52.0),
                            height: Val::Px(52.0),
                            ..default()
                        },
                    ));
                    header
                        .spawn((Node {
                            flex_grow: 1.0,
                            min_width: Val::Px(160.0),
                            flex_direction: FlexDirection::Column,
                            row_gap: Val::Px(2.0),
                            ..default()
                        },))
                        .with_children(|title| {
                            title.spawn((
                                Text::new("RELIQUARY PACK"),
                                TextFont {
                                    font: bold_font.clone().into(),
                                    font_size: FontSize::Px(18.0),
                                    ..default()
                                },
                                TextColor(Color::srgb(0.92, 0.84, 0.70)),
                            ));
                            title.spawn((
                                Text::new("Click an item to equip"),
                                TextFont {
                                    font: regular_font.clone().into(),
                                    font_size: FontSize::Px(11.0),
                                    ..default()
                                },
                                TextColor(Color::srgb(0.50, 0.60, 0.70)),
                            ));
                        });
                    spawn_close(header, false);
                    header.spawn((
                        Text::new(""),
                        TextFont {
                            font: regular_font.clone().into(),
                            font_size: FontSize::Px(12.0),
                            ..default()
                        },
                        TextColor(Color::srgb(0.66, 0.76, 0.86)),
                        TextLayout::new(Justify::Left, LineBreak::WordOrCharacter),
                        Node {
                            width: Val::Percent(100.0),
                            max_height: Val::Px(56.0),
                            overflow: Overflow::clip(),
                            ..default()
                        },
                        InventoryText,
                    ));
                });
            panel
                .spawn((Node {
                    width: Val::Percent(100.0),
                    flex_shrink: 0.0,
                    flex_wrap: FlexWrap::Wrap,
                    row_gap: Val::Px(16.0),
                    display: Display::Flex,
                    column_gap: Val::Px(14.0),
                    min_height: Val::Px(0.0),
                    ..default()
                },))
                .with_children(|body| {
                    body.spawn((Node {
                        width: Val::Px(276.0),
                        max_width: Val::Percent(100.0),
                        flex_shrink: 0.0,
                        display: Display::Flex,
                        flex_direction: FlexDirection::Column,
                        row_gap: Val::Px(6.0),
                        ..default()
                    },))
                        .with_children(|gear| {
                            gear.spawn((
                                Text::new("EQUIPPED"),
                                TextFont {
                                    font: bold_font.clone().into(),
                                    font_size: FontSize::Px(10.0),
                                    ..default()
                                },
                                TextColor(Color::srgb(0.62, 0.66, 0.68)),
                            ));
                            let doll_rows: [&[GearSlot]; 4] = [
                                &[GearSlot::Helm],
                                &[GearSlot::Amulet, GearSlot::Chest, GearSlot::Ring],
                                &[GearSlot::Weapon, GearSlot::Gloves],
                                &[GearSlot::Boots],
                            ];
                            for row in doll_rows {
                                gear.spawn((Node {
                                    width: Val::Percent(100.0),
                                    display: Display::Flex,
                                    justify_content: JustifyContent::Center,
                                    column_gap: Val::Px(6.0),
                                    ..default()
                                },))
                                    .with_children(|doll_row| {
                                        for slot in row {
                                            doll_row
                                                .spawn((
                                                    Node {
                                                        width: Val::Px(82.0),
                                                        height: Val::Px(76.0),
                                                        border: UiRect::all(Val::Px(1.0)),
                                                        overflow: Overflow::clip(),
                                                        ..default()
                                                    },
                                                    BorderColor::all(Color::srgba(
                                                        0.45, 0.40, 0.32, 0.55,
                                                    )),
                                                    BackgroundColor(Color::srgba(
                                                        0.030, 0.026, 0.022, 0.90,
                                                    )),
                                                    slot_frame_image_node(asset_server),
                                                    PaperDollSlot(*slot),
                                                ))
                                                .with_children(|gear_slot| {
                                                    gear_slot.spawn((
                                                        atlas_image_node(
                                                            asset_server,
                                                            UI_GEAR_ICONS_IMAGE,
                                                            atlas_rect(
                                                                UI_GEAR_ICON_CELL,
                                                                UI_GEAR_ICON_CELL,
                                                                4,
                                                                gear_icon_index(*slot),
                                                            ),
                                                        ),
                                                        Node {
                                                            position_type: PositionType::Absolute,
                                                            left: Val::Px(20.0),
                                                            top: Val::Px(5.0),
                                                            width: Val::Px(42.0),
                                                            height: Val::Px(42.0),
                                                            ..default()
                                                        },
                                                        Pickable::IGNORE,
                                                    ));
                                                    gear_slot.spawn((
                                                        Text::new(""),
                                                        TextFont {
                                                            font: regular_font.clone().into(),
                                                            font_size: FontSize::Px(10.0),
                                                            ..default()
                                                        },
                                                        TextColor(Color::srgb(0.82, 0.75, 0.62)),
                                                        TextLayout::new(
                                                            Justify::Center,
                                                            LineBreak::WordOrCharacter,
                                                        ),
                                                        Node {
                                                            position_type: PositionType::Absolute,
                                                            left: Val::Px(4.0),
                                                            right: Val::Px(4.0),
                                                            bottom: Val::Px(4.0),
                                                            height: Val::Px(24.0),
                                                            overflow: Overflow::clip(),
                                                            ..default()
                                                        },
                                                        PaperDollSlotText(*slot),
                                                    ));
                                                });
                                        }
                                    });
                            }
                            gear.spawn((Node {
                                width: Val::Percent(100.0),
                                display: Display::Flex,
                                flex_wrap: FlexWrap::Wrap,
                                column_gap: Val::Px(6.0),
                                row_gap: Val::Px(6.0),
                                ..default()
                            },))
                                .with_children(|summary| {
                                    for slot in [
                                        GearSlotText::Weapon,
                                        GearSlotText::Charm,
                                        GearSlotText::Codex,
                                        GearSlotText::Armory,
                                    ] {
                                        summary.spawn((
                                            Node {
                                                width: Val::Px(132.0),
                                                height: Val::Px(38.0),
                                                padding: UiRect::all(Val::Px(5.0)),
                                                border: UiRect::all(Val::Px(1.0)),
                                                align_items: AlignItems::Center,
                                                overflow: Overflow::clip(),
                                                ..default()
                                            },
                                            BorderColor::all(Color::srgba(0.65, 0.52, 0.34, 0.58)),
                                            BackgroundColor(Color::srgba(
                                                0.035, 0.030, 0.026, 0.90,
                                            )),
                                            Text::new(""),
                                            TextFont {
                                                font: regular_font.clone().into(),
                                                font_size: FontSize::Px(10.0),
                                                ..default()
                                            },
                                            TextColor(Color::srgb(0.86, 0.78, 0.64)),
                                            slot,
                                        ));
                                    }
                                });
                        });
                    body.spawn((Node {
                        flex_grow: 1.0,
                        flex_basis: Val::Px(340.0),
                        min_width: Val::Px(0.0),
                        display: Display::Flex,
                        align_content: AlignContent::FlexStart,
                        flex_wrap: FlexWrap::Wrap,
                        column_gap: Val::Px(7.0),
                        row_gap: Val::Px(7.0),
                        ..default()
                    },))
                        .with_children(|grid| {
                            for index in 0..10 {
                                grid.spawn((
                                    Node {
                                        width: Val::Px(80.0),
                                        height: Val::Px(112.0),
                                        border: UiRect::all(Val::Px(1.0)),
                                        overflow: Overflow::clip(),
                                        ..default()
                                    },
                                    BorderColor::all(Color::srgba(0.36, 0.43, 0.50, 0.58)),
                                    BackgroundColor(Color::srgba(0.020, 0.024, 0.030, 0.92)),
                                    slot_frame_image_node(asset_server),
                                    InventorySlot(index),
                                ))
                                .with_children(|item_slot| {
                                    item_slot.spawn((
                                        atlas_image_node(
                                            asset_server,
                                            UI_GEAR_ICONS_IMAGE,
                                            atlas_rect(UI_GEAR_ICON_CELL, UI_GEAR_ICON_CELL, 4, 0),
                                        ),
                                        Node {
                                            position_type: PositionType::Absolute,
                                            left: Val::Px(15.0),
                                            top: Val::Px(7.0),
                                            width: Val::Px(50.0),
                                            height: Val::Px(50.0),
                                            ..default()
                                        },
                                        Pickable::IGNORE,
                                        Visibility::Hidden,
                                        InventorySlotIcon(index),
                                    ));
                                    item_slot.spawn((
                                        Text::new(""),
                                        TextFont {
                                            font: regular_font.clone().into(),
                                            font_size: FontSize::Px(10.0),
                                            ..default()
                                        },
                                        TextColor(Color::srgb(0.70, 0.82, 0.95)),
                                        TextLayout::new(
                                            Justify::Center,
                                            LineBreak::WordOrCharacter,
                                        ),
                                        Node {
                                            position_type: PositionType::Absolute,
                                            left: Val::Px(4.0),
                                            right: Val::Px(4.0),
                                            bottom: Val::Px(5.0),
                                            height: Val::Px(46.0),
                                            overflow: Overflow::clip(),
                                            ..default()
                                        },
                                        InventorySlotText(index),
                                    ));
                                });
                            }
                        });
                });
        });
    parent
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                left: Val::Px(0.0),
                right: Val::Px(0.0),
                top: Val::Px(30.0),
                width: Val::Px(780.0),
                max_width: Val::Percent(92.0),
                height: Val::Px(600.0),
                max_height: Val::Percent(88.0),
                margin: UiRect::horizontal(Val::Auto),
                padding: UiRect::all(Val::Px(20.0)),
                border: UiRect::all(Val::Px(1.0)),
                display: Display::None,
                flex_direction: FlexDirection::Column,
                row_gap: Val::Px(16.0),
                overflow: Overflow::scroll_y(),
                ..default()
            },
            BorderColor::all(Color::srgba(0.72, 0.48, 0.90, 0.72)),
            BackgroundColor(Color::srgb(0.018, 0.012, 0.026)),
            GlobalZIndex(40),
            BuildPanel,
            PanelScroll,
            ScrollPosition::default(),
        ))
        .with_children(|build| {
            build
                .spawn(Node {
                    width: Val::Percent(100.0),
                    align_items: AlignItems::Center,
                    justify_content: JustifyContent::SpaceBetween,
                    flex_shrink: 0.0,
                    ..default()
                })
                .with_children(|header| {
                    header.spawn((
                        Text::new("Skills & talents"),
                        TextFont {
                            font: bold_font.clone().into(),
                            font_size: FontSize::Px(24.0),
                            ..default()
                        },
                        TextColor(Color::srgb(0.92, 0.84, 0.70)),
                    ));
                    spawn_close(header, true);
                });
            skills::spawn_tabs(build, asset_server);
            skills::spawn_page(build, asset_server);
            build
                .spawn((
                    Node {
                        width: Val::Percent(100.0),
                        flex_direction: FlexDirection::Column,
                        row_gap: Val::Px(16.0),
                        flex_shrink: 0.0,
                        display: Display::None,
                        ..default()
                    },
                    skills::BuildPage(skills::BuildTab::Talents),
                ))
                .with_children(|talents| {
                    talents.spawn((
                        Text::new(""),
                        TextFont {
                            font: bold_font.clone().into(),
                            font_size: FontSize::Px(13.0),
                            ..default()
                        },
                        TextColor(Color::srgb(0.88, 0.82, 0.92)),
                        TextLayout::new(Justify::Left, LineBreak::WordOrCharacter),
                        Node {
                            width: Val::Percent(100.0),
                            min_height: Val::Px(52.0),
                            flex_shrink: 0.0,
                            overflow: Overflow::clip(),
                            ..default()
                        },
                        BuildText,
                    ));
                    talents
                        .spawn((Node {
                            width: Val::Percent(100.0),
                            flex_shrink: 0.0,
                            flex_wrap: FlexWrap::Wrap,
                            row_gap: Val::Px(12.0),
                            display: Display::Flex,
                            column_gap: Val::Px(10.0),
                            min_height: Val::Px(0.0),
                            ..default()
                        },))
                        .with_children(|branches| {
                            for branch in TalentBranch::ALL {
                                branches
                                    .spawn((
                                        Node {
                                            flex_grow: 1.0,
                                            flex_basis: Val::Px(210.0),
                                            max_width: Val::Px(260.0),
                                            min_width: Val::Px(0.0),
                                            min_height: Val::Px(660.0),
                                            padding: UiRect::axes(Val::Px(15.0), Val::Px(13.0)),
                                            border: UiRect::all(Val::Px(1.0)),
                                            flex_direction: FlexDirection::Column,
                                            align_items: AlignItems::Center,
                                            row_gap: Val::Px(5.0),
                                            overflow: Overflow::clip(),
                                            ..default()
                                        },
                                        BorderColor::all(branch.color().with_alpha(0.56)),
                                        BackgroundColor(Color::srgba(0.014, 0.014, 0.020, 0.92)),
                                        slot_frame_image_node(asset_server),
                                    ))
                                    .with_children(|card| {
                                        card.spawn((
                                            atlas_image_node(
                                                asset_server,
                                                UI_TALENT_SIGILS_IMAGE,
                                                atlas_rect(
                                                    UI_TALENT_SIGIL_CELL,
                                                    UI_TALENT_SIGIL_CELL,
                                                    3,
                                                    talent_icon_index(branch),
                                                ),
                                            ),
                                            Node {
                                                width: Val::Px(92.0),
                                                height: Val::Px(92.0),
                                                ..default()
                                            },
                                        ));
                                        card.spawn((
                                            Text::new(branch.label()),
                                            TextFont {
                                                font: bold_font.clone().into(),
                                                font_size: FontSize::Px(16.0),
                                                ..default()
                                            },
                                            TextColor(branch.color()),
                                        ));
                                        card.spawn((
                                            Text::new(branch.role()),
                                            TextFont {
                                                font: regular_font.clone().into(),
                                                font_size: FontSize::Px(11.0),
                                                ..default()
                                            },
                                            TextColor(Color::srgb(0.58, 0.58, 0.62)),
                                        ));
                                        card.spawn((
                                            Text::new("0 / 8"),
                                            TextFont {
                                                font: bold_font.clone().into(),
                                                font_size: FontSize::Px(12.0),
                                                ..default()
                                            },
                                            TextColor(Color::srgb(0.86, 0.84, 0.80)),
                                            TalentRankText(branch),
                                        ));
                                        skills::spawn_talent_description(
                                            card,
                                            asset_server,
                                            branch,
                                        );
                                        card.spawn((Node {
                                            width: Val::Percent(100.0),
                                            flex_grow: 1.0,
                                            flex_direction: FlexDirection::Column,
                                            row_gap: Val::Px(3.0),
                                            ..default()
                                        },))
                                            .with_children(|ladder| {
                                                for rank in 1..=8 {
                                                    ladder
                                                        .spawn((
                                                            Node {
                                                                width: Val::Percent(100.0),
                                                                min_height: Val::Px(42.0),
                                                                flex_shrink: 0.0,
                                                                padding: UiRect::axes(
                                                                    Val::Px(6.0),
                                                                    Val::Px(3.0),
                                                                ),
                                                                border: UiRect::all(Val::Px(1.0)),
                                                                align_items: AlignItems::Center,
                                                                ..default()
                                                            },
                                                            BorderColor::all(Color::srgba(
                                                                0.20, 0.20, 0.24, 0.72,
                                                            )),
                                                            BackgroundColor(Color::srgba(
                                                                0.025, 0.025, 0.032, 0.92,
                                                            )),
                                                            TalentRankNode { branch, rank },
                                                            Button,
                                                        ))
                                                        .with_children(|node| {
                                                            skills::spawn_talent_rank(
                                                                node,
                                                                asset_server,
                                                                branch,
                                                                rank,
                                                            );
                                                        });
                                                }
                                            });
                                    });
                            }
                        });
                    talents.spawn((
                        Text::new("Click the next rank to learn · 1 Wrath · 2 Vigor · 3 Focus"),
                        TextFont {
                            font: regular_font.clone().into(),
                            font_size: FontSize::Px(11.0),
                            ..default()
                        },
                        TextColor(Color::srgb(0.52, 0.50, 0.56)),
                        TextLayout::new(Justify::Center, LineBreak::WordOrCharacter),
                        Node {
                            width: Val::Percent(100.0),
                            ..default()
                        },
                    ));
                });
        });
}

fn spawn_close(parent: &mut ChildSpawnerCommands, build: bool) {
    parent
        .spawn((
            Button,
            PanelClose(build),
            Node {
                padding: UiRect::axes(Val::Px(12.0), Val::Px(8.0)),
                flex_shrink: 0.0,
                border: UiRect::all(Val::Px(1.0)),
                ..default()
            },
            BackgroundColor(Color::srgba(0.08, 0.065, 0.045, 1.0)),
            BorderColor::all(Color::srgba(0.56, 0.43, 0.25, 0.8)),
        ))
        .with_children(|button| {
            button.spawn((
                Text::new(if build { "K · Close" } else { "I · Close" }),
                TextFont {
                    font_size: FontSize::Px(12.0),
                    ..default()
                },
                TextColor(Color::srgb(0.92, 0.84, 0.70)),
            ));
        });
}

pub(super) fn close_panels(
    buttons: Query<(&Interaction, &PanelClose), Changed<Interaction>>,
    mut inventory: ResMut<InventoryOpen>,
    mut build: ResMut<BuildOpen>,
) {
    for (interaction, close) in &buttons {
        if *interaction == Interaction::Pressed {
            if close.0 {
                build.open = false;
            } else {
                inventory.open = false;
            }
        }
    }
}

pub(super) fn scroll_panels(
    mut wheel: MessageReader<MouseWheel>,
    windows: Query<&Window, With<PrimaryWindow>>,
    mut panels: Query<
        (
            &Node,
            &ComputedNode,
            &UiGlobalTransform,
            &mut ScrollPosition,
        ),
        With<PanelScroll>,
    >,
) {
    let delta: f32 = wheel
        .read()
        .map(|event| {
            event.y
                * if event.unit == MouseScrollUnit::Line {
                    36.0
                } else {
                    1.0
                }
        })
        .sum();
    let Some(cursor) = windows.single().ok().and_then(cursor_physical_position) else {
        return;
    };
    for (style, node, transform, mut scroll) in &mut panels {
        if style.display == Display::None || !node_under_cursor(node, transform, cursor) {
            continue;
        }
        let max = ((node.content_size.y - node.size.y) * node.inverse_scale_factor).max(0.0);
        scroll.0.y = (scroll.0.y - delta).clamp(0.0, max);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn inventory_header_contains_only_capacity_and_currency() {
        let stats = RunStats {
            gold: 13,
            ember_shards: 2,
            ..default()
        };
        let inventory = Inventory {
            items: Vec::new(),
            capacity: 12,
        };
        assert_eq!(
            inventory_panel_compact_header(&stats, &inventory),
            format!(
                "Inventory slots 0/{}\nGold 13  ·  Ember shards 2",
                inventory.capacity
            )
        );
    }

    #[test]
    fn talent_summary_shows_real_bonuses_without_estimated_build_scores() {
        let summary = talent_panel_text(BuildPanelInputs {
            damage_bonus: &DamageBonus(0.0),
            equipment: &Equipment {
                weapon_name: "Test Blade".into(),
                quality: "common".into(),
                crit_chance: 0.03,
                health_bonus: 0.0,
                armor_bonus: 0.0,
                legendary_power: LegendaryPower::None,
                temper_level: 0,
                socketed_gem: None,
                worn: Equipment::empty_worn(),
            },
            charm: &Charm::default(),
            talents: &Talents::default(),
        });
        assert!(summary.starts_with("Available talent points: 0\n"));
        assert!(summary.contains("Critical chance"));
        assert!(summary.contains("Cooldown reduction"));
        for diagnostic in ["DPS", "UNBOUND", "CONSTELLATION", "LIFE", "FURY", "x1."] {
            assert!(!summary.contains(diagnostic));
        }
    }
}
