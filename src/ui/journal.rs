//! A readable journal with a fixed header and independently scrolling entries.
use super::{
    JournalText,
    icons::{UiIcon, ui_icon},
    localization::NativeLanguageName,
    theme::UI_FONT_REGULAR,
};
use crate::{GameState, JournalOpen};
use bevy::{
    input::mouse::{MouseScrollUnit, MouseWheel},
    prelude::*,
    text::LineHeight,
};

pub(super) struct JournalPanelPlugin;
impl Plugin for JournalPanelPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            (
                update_sections.after(super::update_hud),
                close_journal,
                scroll_journal,
            )
                .run_if(in_state(GameState::InGame)),
        );
    }
}

#[derive(Component)]
pub(super) struct JournalPanel;
#[derive(Component)]
struct JournalClose;
#[derive(Component)]
struct JournalScroll;
#[derive(Component)]
struct Section(usize);
#[derive(Component)]
struct SectionText {
    index: usize,
    heading: bool,
}

pub(super) fn spawn_journal(parent: &mut ChildSpawnerCommands, assets: &AssetServer) {
    let font: Handle<Font> = assets.load(UI_FONT_REGULAR);
    parent
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                top: Val::Px(76.0),
                right: Val::Px(16.0),
                bottom: Val::Px(196.0),
                width: Val::Px(480.0),
                max_width: Val::Percent(92.0),
                flex_direction: FlexDirection::Column,
                border: UiRect::all(Val::Px(1.0)),
                padding: UiRect::all(Val::Px(20.0)),
                ..default()
            },
            BorderColor::all(Color::srgb(0.48, 0.36, 0.20)),
            BackgroundColor(Color::srgb(0.027, 0.024, 0.020)),
            Visibility::Hidden,
            GlobalZIndex(30),
            JournalPanel,
            JournalText,
        ))
        .with_children(|panel| {
            // The existing HUD system supplies one source string; only its section
            // views are rendered, so the panel never becomes one enormous text box.
            panel.spawn((
                Text::new(""),
                Node {
                    display: Display::None,
                    ..default()
                },
                JournalText,
                NativeLanguageName,
            ));
            panel
                .spawn(Node {
                    width: Val::Percent(100.0),
                    flex_shrink: 0.0,
                    align_items: AlignItems::Center,
                    column_gap: Val::Px(12.0),
                    padding: UiRect::bottom(Val::Px(16.0)),
                    margin: UiRect::bottom(Val::Px(16.0)),
                    border: UiRect::bottom(Val::Px(1.0)),
                    ..default()
                })
                .insert(BorderColor::all(Color::srgb(0.30, 0.24, 0.15)))
                .with_children(|header| {
                    header.spawn((
                        ui_icon(assets, UiIcon::Journal),
                        Node {
                            width: Val::Px(32.0),
                            height: Val::Px(32.0),
                            ..default()
                        },
                        Pickable::IGNORE,
                    ));
                    header.spawn((
                        Text::new("JOURNEY"),
                        TextFont {
                            font: font.clone().into(),
                            font_size: FontSize::Px(22.0),
                            ..default()
                        },
                        TextColor(Color::srgb(0.94, 0.84, 0.65)),
                        Node {
                            flex_grow: 1.0,
                            ..default()
                        },
                    ));
                    header
                        .spawn((
                            Button,
                            JournalClose,
                            Node {
                                padding: UiRect::axes(Val::Px(10.0), Val::Px(7.0)),
                                ..default()
                            },
                            BackgroundColor(Color::srgb(0.13, 0.105, 0.07)),
                        ))
                        .with_children(|button| {
                            button.spawn((
                                Text::new("Close [J]"),
                                TextFont {
                                    font_size: FontSize::Px(12.0),
                                    ..default()
                                },
                                TextColor(Color::srgb(0.83, 0.76, 0.62)),
                                Pickable::IGNORE,
                            ));
                        });
                });
            panel
                .spawn((
                    Node {
                        width: Val::Percent(100.0),
                        min_height: Val::Px(0.0),
                        flex_grow: 1.0,
                        overflow: Overflow::scroll_y(),
                        flex_direction: FlexDirection::Column,
                        row_gap: Val::Px(20.0),
                        ..default()
                    },
                    ScrollPosition::default(),
                    JournalScroll,
                ))
                .with_children(|body| {
                    for index in 0..6 {
                        body.spawn((
                            Node {
                                width: Val::Percent(100.0),
                                flex_shrink: 0.0,
                                flex_direction: FlexDirection::Column,
                                row_gap: Val::Px(7.0),
                                ..default()
                            },
                            Section(index),
                        ))
                        .with_children(|section| {
                            for heading in [true, false] {
                                section.spawn((
                                    Text::new(""),
                                    SectionText { index, heading },
                                    TextFont {
                                        font: font.clone().into(),
                                        font_size: FontSize::Px(if heading { 13.0 } else { 16.0 }),
                                        ..default()
                                    },
                                    TextColor(if heading {
                                        Color::srgb(0.70, 0.55, 0.32)
                                    } else {
                                        Color::srgb(0.86, 0.84, 0.77)
                                    }),
                                    LineHeight::RelativeToFont(1.55),
                                    TextLayout::new(Justify::Left, LineBreak::WordOrCharacter),
                                    Node {
                                        width: Val::Percent(100.0),
                                        ..default()
                                    },
                                ));
                            }
                        });
                    }
                });
        });
}

fn sections(source: &str) -> Vec<(String, String)> {
    let mut sections: Vec<(String, String)> = Vec::new();
    for line in source.lines() {
        if matches!(
            line,
            "CURRENT OBJECTIVE"
                | "BOUNTY"
                | "BLOOD RITE"
                | "EMBER RIFT"
                | "RECENT EVENTS"
                | "RECOVERED WRITING"
        ) {
            sections.push((line.into(), String::new()));
        } else if let Some((_, body)) = sections.last_mut() {
            if !body.is_empty() {
                body.push('\n');
            }
            body.push_str(line);
        }
    }
    for (_, body) in &mut sections {
        *body = body.trim().into();
    }
    sections
}

fn update_sections(
    source: Query<&Text, (With<JournalText>, Without<SectionText>)>,
    mut texts: Query<(&SectionText, &mut Text), Without<JournalText>>,
    mut nodes: Query<(&Section, &mut Node)>,
) {
    let Ok(source) = source.single() else {
        return;
    };
    let entries = sections(&source.0);
    for (section, mut node) in &mut nodes {
        node.display = if section.0 < entries.len() {
            Display::Flex
        } else {
            Display::None
        };
    }
    for (section, mut text) in &mut texts {
        let value = entries
            .get(section.index)
            .map(|entry| if section.heading { &entry.0 } else { &entry.1 })
            .cloned()
            .unwrap_or_default();
        if text.0 != value {
            text.0 = value;
        }
    }
}

fn close_journal(
    buttons: Query<&Interaction, (Changed<Interaction>, With<JournalClose>)>,
    mut journal: ResMut<JournalOpen>,
) {
    if buttons
        .iter()
        .any(|interaction| *interaction == Interaction::Pressed)
    {
        journal.open = false;
    }
}

fn scroll_journal(
    mut wheel: MessageReader<MouseWheel>,
    panels: Query<&Visibility, With<JournalPanel>>,
    mut scrolls: Query<(&mut ScrollPosition, &ComputedNode), With<JournalScroll>>,
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
    if panels
        .iter()
        .all(|visibility| *visibility == Visibility::Hidden)
    {
        return;
    }
    for (mut scroll, node) in &mut scrolls {
        let max = ((node.content_size.y - node.size.y) * node.inverse_scale_factor).max(0.0);
        scroll.0.y = (scroll.0.y - delta).clamp(0.0, max);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn journal_sections_preserve_story_paragraphs_and_drop_document_title() {
        let entries = sections(
            "JOURNEY\n\nCURRENT OBJECTIVE\nOpen the cache\n\nRECENT EVENTS\nFirst event.\n\nSecond event.\n\nRECOVERED WRITING\nA recovered page.",
        );
        assert_eq!(entries.len(), 3);
        assert_eq!(entries[0].1, "Open the cache");
        assert_eq!(entries[1].1, "First event.\n\nSecond event.");
        assert_eq!(entries[2].1, "A recovered page.");
    }
}
