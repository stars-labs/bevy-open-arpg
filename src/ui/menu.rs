use super::icons::{UiIcon, ui_icon};
use super::*;

pub(super) fn ensure_main_menu_ui(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    menu_query: Query<Entity, With<MenuUi>>,
    difficulty: Res<DifficultySettings>,
    records: Res<ChapterRecords>,
    audio: Res<AudioSettings>,
    audio_backend: Res<AudioBackendStatus>,
) {
    if !menu_query.is_empty() {
        return;
    }
    spawn_main_menu(
        &mut commands,
        &asset_server,
        &difficulty,
        &records,
        &audio,
        &audio_backend,
    );
}

pub(super) fn update_main_menu_backdrop_crop(
    window: Query<&Window, With<PrimaryWindow>>,
    mut images: Query<&mut ImageNode, With<MainMenuBackdropArt>>,
) {
    let Ok(window) = window.single() else {
        return;
    };
    let viewport = Vec2::new(window.resolution.width(), window.resolution.height());
    let crop = main_menu_cover_rect(viewport);
    for mut image in &mut images {
        image.rect = Some(crop);
    }
}

pub(super) fn main_menu_cover_rect(viewport: Vec2) -> Rect {
    if viewport.x <= f32::EPSILON || viewport.y <= f32::EPSILON {
        return Rect::from_corners(Vec2::ZERO, MAIN_MENU_BACKGROUND_SIZE);
    }

    let source_aspect = MAIN_MENU_BACKGROUND_SIZE.x / MAIN_MENU_BACKGROUND_SIZE.y;
    let viewport_aspect = viewport.x / viewport.y;
    if viewport_aspect > source_aspect {
        let visible_height = MAIN_MENU_BACKGROUND_SIZE.x / viewport_aspect;
        let top = (MAIN_MENU_BACKGROUND_SIZE.y - visible_height) * 0.5;
        Rect::from_corners(
            Vec2::new(0.0, top),
            Vec2::new(MAIN_MENU_BACKGROUND_SIZE.x, top + visible_height),
        )
    } else {
        let visible_width = MAIN_MENU_BACKGROUND_SIZE.y * viewport_aspect;
        let left = (MAIN_MENU_BACKGROUND_SIZE.x - visible_width) * 0.5;
        Rect::from_corners(
            Vec2::new(left, 0.0),
            Vec2::new(left + visible_width, MAIN_MENU_BACKGROUND_SIZE.y),
        )
    }
}

#[derive(Component, Clone, Copy, PartialEq, Eq)]
pub(crate) enum MenuAction {
    Begin,
    Continue,
    Difficulty,
    Audio,
}

#[derive(Component)]
pub(super) struct MenuHint(String);
#[derive(Component)]
pub(super) struct MenuHintText;
#[derive(Component)]
pub(super) struct MenuEmber(usize);
#[derive(Component)]
struct MenuTitle;
#[derive(Component)]
pub(super) struct MenuContent;
#[derive(Component)]
pub(super) struct MenuAudioText;

const GOLD: Color = Color::srgb(0.83, 0.67, 0.40);
const IVORY: Color = Color::srgb(0.95, 0.89, 0.76);
const MUTED: Color = Color::srgb(0.65, 0.62, 0.54);

fn lettering(
    asset_server: &AssetServer,
    text: impl Into<String>,
    size: f32,
    heading: bool,
    color: Color,
) -> impl Bundle {
    (
        Text::new(text),
        TextFont {
            font: asset_server
                .load(if heading {
                    UI_FONT_BOLD
                } else {
                    UI_FONT_REGULAR
                })
                .into(),
            font_size: FontSize::Px(size),
            ..default()
        },
        TextColor(color),
        TextShadow {
            offset: Vec2::new(0.0, 2.0),
            color: Color::srgba(0.0, 0.0, 0.0, 0.95),
        },
    )
}

#[allow(clippy::too_many_arguments)]
fn menu_button(
    parent: &mut ChildSpawnerCommands,
    assets: &AssetServer,
    action: MenuAction,
    icon: UiIcon,
    label: &str,
    key: &str,
    hint: String,
) {
    let primary = action == MenuAction::Begin;
    parent
        .spawn((
            Button,
            action,
            MenuHint(hint),
            Node {
                width: Val::Percent(100.0),
                min_height: Val::Px(if primary { 66.0 } else { 48.0 }),
                padding: UiRect::axes(Val::Px(16.0), Val::Px(7.0)),
                border: UiRect::all(Val::Px(1.0)),
                align_items: AlignItems::Center,
                column_gap: Val::Px(12.0),
                flex_shrink: 0.0,
                ..default()
            },
            BackgroundColor(if primary {
                Color::srgba(0.23, 0.075, 0.025, 0.90)
            } else {
                Color::srgba(0.018, 0.022, 0.026, 0.85)
            }),
            BorderColor::all(if primary {
                GOLD
            } else {
                Color::srgba(0.53, 0.43, 0.28, 0.45)
            }),
            BoxShadow(vec![ShadowStyle {
                color: Color::srgba(0.65, 0.20, 0.025, if primary { 0.20 } else { 0.0 }),
                x_offset: Val::ZERO,
                y_offset: Val::ZERO,
                spread_radius: Val::ZERO,
                blur_radius: Val::Px(20.0),
            }]),
        ))
        .with_children(|button| {
            button.spawn((
                ui_icon(assets, icon),
                Node {
                    width: Val::Px(38.0),
                    height: Val::Px(38.0),
                    flex_shrink: 0.0,
                    ..default()
                },
            ));
            let mut text = button.spawn((
                lettering(
                    assets,
                    label,
                    if primary { 21.0 } else { 16.0 },
                    true,
                    IVORY,
                ),
                Node {
                    flex_grow: 1.0,
                    ..default()
                },
            ));
            if action == MenuAction::Difficulty {
                text.insert(MenuDifficultyText);
            }
            if action == MenuAction::Audio {
                text.insert(MenuAudioText);
            }
            button.spawn((
                lettering(assets, key, 12.0, true, GOLD),
                Node {
                    padding: UiRect::axes(Val::Px(6.0), Val::Px(3.0)),
                    border: UiRect::all(Val::Px(1.0)),
                    ..default()
                },
                BorderColor::all(Color::srgba(0.66, 0.52, 0.32, 0.35)),
            ));
        });
}

pub(super) fn spawn_main_menu(
    commands: &mut Commands,
    asset_server: &AssetServer,
    difficulty: &DifficultySettings,
    records: &ChapterRecords,
    audio: &AudioSettings,
    audio_backend: &AudioBackendStatus,
) {
    commands
        .spawn((
            Node {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                overflow: Overflow::clip(),
                ..default()
            },
            BackgroundColor(Color::srgb(0.006, 0.007, 0.010)),
            MenuUi,
        ))
        .with_children(|root| {
            root.spawn((
                ImageNode {
                    image: asset_server.load(MAIN_MENU_BACKGROUND_IMAGE),
                    image_mode: NodeImageMode::Stretch,
                    ..default()
                },
                Node {
                    position_type: PositionType::Absolute,
                    width: Val::Percent(100.0),
                    height: Val::Percent(100.0),
                    ..default()
                },
                MainMenuBackdropArt,
            ));
            root.spawn((
                Node {
                    position_type: PositionType::Absolute,
                    width: Val::Percent(100.0),
                    height: Val::Percent(100.0),
                    ..default()
                },
                BackgroundGradient::from(LinearGradient::new(
                    LinearGradient::TO_RIGHT,
                    vec![
                        ColorStop::percent(main_menu_scrim_color(), 0.0),
                        ColorStop::percent(Color::srgba(0.008, 0.009, 0.012, 0.80), 32.0),
                        ColorStop::percent(Color::srgba(0.008, 0.009, 0.012, 0.25), 62.0),
                        ColorStop::percent(Color::NONE, 100.0),
                    ],
                )),
            ));
            // Small rising embers give the title screen life without hiding its artwork.
            for index in 0..24 {
                root.spawn((
                    MenuEmber(index),
                    Node {
                        position_type: PositionType::Absolute,
                        width: Val::Px(if index % 3 == 0 { 3.0 } else { 2.0 }),
                        height: Val::Px(2.0),
                        ..default()
                    },
                    BackgroundColor(Color::srgba(1.0, 0.38, 0.07, 0.4)),
                ));
            }
            root.spawn((
                MenuContent,
                Node {
                    position_type: PositionType::Absolute,
                    left: Val::Percent(7.0),
                    top: Val::Percent(8.0),
                    bottom: Val::Percent(7.0),
                    width: Val::Px(440.0),
                    max_width: Val::Percent(43.0),
                    flex_direction: FlexDirection::Column,
                    justify_content: JustifyContent::Center,
                    row_gap: Val::Px(12.0),
                    ..default()
                },
            ))
            .with_children(|panel| {
                panel
                    .spawn((Node {
                        align_items: AlignItems::Center,
                        column_gap: Val::Px(12.0),
                        height: Val::Px(40.0),
                        flex_shrink: 0.0,
                        ..default()
                    },))
                    .with_children(|brand| {
                        brand.spawn((
                            ui_icon(asset_server, UiIcon::Difficulty),
                            Node {
                                width: Val::Px(42.0),
                                height: Val::Px(42.0),
                                ..default()
                            },
                        ));
                        brand.spawn(lettering(
                            asset_server,
                            "A   D E S C E N T   I N T O   A S H",
                            11.0,
                            true,
                            GOLD,
                        ));
                    });
                panel.spawn((
                    lettering(asset_server, "SHADOW", 50.0, true, IVORY),
                    MenuTitle,
                    Node {
                        margin: UiRect::bottom(Val::Px(-16.0)),
                        ..default()
                    },
                ));
                panel.spawn((
                    lettering(asset_server, "RELIQUARY", 42.0, true, GOLD),
                    MenuTitle,
                ));
                panel.spawn((
                    Node {
                        width: Val::Percent(88.0),
                        height: Val::Px(1.0),
                        margin: UiRect::vertical(Val::Px(10.0)),
                        flex_shrink: 0.0,
                        ..default()
                    },
                    BackgroundGradient::from(LinearGradient::new(
                        LinearGradient::TO_RIGHT,
                        vec![
                            ColorStop::auto(GOLD),
                            ColorStop::auto(Color::srgba(0.83, 0.67, 0.40, 0.0)),
                        ],
                    )),
                ));
                panel.spawn(lettering(
                    asset_server,
                    "Beyond the gates, the embers remember.",
                    21.0,
                    false,
                    IVORY,
                ));
                panel.spawn((
                    lettering(
                        asset_server,
                        "CHAPTER I  /  THE ASHEN RELIQUARY",
                        11.0,
                        true,
                        MUTED,
                    ),
                    Node {
                        margin: UiRect::bottom(Val::Px(12.0)),
                        ..default()
                    },
                ));
                menu_button(
                    panel,
                    asset_server,
                    MenuAction::Begin,
                    UiIcon::Begin,
                    "Begin Descent",
                    "ENTER",
                    "Begin a new journey into the Ashen Reliquary. Space or Enter to descend."
                        .into(),
                );
                menu_button(
                    panel,
                    asset_server,
                    MenuAction::Continue,
                    UiIcon::Continue,
                    "Continue Journey",
                    "F9",
                    "Resume your saved journey. If no save exists, a new journey begins.".into(),
                );
                menu_button(
                    panel,
                    asset_server,
                    MenuAction::Difficulty,
                    UiIcon::Difficulty,
                    difficulty.current.label(),
                    "TAB",
                    menu_difficulty_line(difficulty.current, records),
                );
                menu_button(
                    panel,
                    asset_server,
                    MenuAction::Audio,
                    UiIcon::Audio,
                    if audio.enabled {
                        "Sound On"
                    } else {
                        "Sound Off"
                    },
                    "M",
                    format!(
                        "Toggle music and combat sounds. {}",
                        audio_backend.status_label(audio)
                    ),
                );
                panel.spawn((
                    lettering(asset_server, legacy_title(records), 16.0, false, MUTED),
                    MenuProfileText,
                    Node {
                        margin: UiRect::top(Val::Px(8.0)),
                        ..default()
                    },
                ));
                panel.spawn((
                    lettering(
                        asset_server,
                        "Choose your path. The reliquary awaits.",
                        16.0,
                        false,
                        MUTED,
                    ),
                    MenuHintText,
                    Node {
                        min_height: Val::Px(50.0),
                        width: Val::Percent(100.0),
                        ..default()
                    },
                ));
            });
        });
}

pub(super) fn main_menu_scrim_color() -> Color {
    Color::srgba(0.008, 0.009, 0.012, 0.94)
}

#[cfg(test)]
pub(super) fn main_menu_background_image_path() -> &'static str {
    MAIN_MENU_BACKGROUND_IMAGE
}
#[cfg(test)]
pub(super) fn main_menu_scrim_alpha() -> f32 {
    main_menu_scrim_color().to_srgba().alpha
}

#[allow(clippy::too_many_arguments, clippy::type_complexity)]
pub(super) fn update_menu(
    difficulty: Res<DifficultySettings>,
    records: Res<ChapterRecords>,
    audio: Res<AudioSettings>,
    audio_backend: Res<AudioBackendStatus>,
    mut difficulty_text: Query<
        &mut Text,
        (
            With<MenuDifficultyText>,
            Without<MenuProfileText>,
            Without<MenuAudioText>,
        ),
    >,
    mut profile_text: Query<
        &mut Text,
        (
            With<MenuProfileText>,
            Without<MenuDifficultyText>,
            Without<MenuAudioText>,
        ),
    >,
    mut audio_text: Query<
        &mut Text,
        (
            With<MenuAudioText>,
            Without<MenuDifficultyText>,
            Without<MenuProfileText>,
        ),
    >,
    mut hints: Query<(&MenuAction, &mut MenuHint)>,
) {
    if !difficulty.is_changed()
        && !records.is_changed()
        && !audio.is_changed()
        && !audio_backend.is_changed()
    {
        return;
    }
    for mut text in &mut difficulty_text {
        **text = difficulty.current.label().into();
    }
    for mut text in &mut profile_text {
        **text = legacy_title(&records).into();
    }
    for mut text in &mut audio_text {
        **text = if audio.enabled {
            "Sound On"
        } else {
            "Sound Off"
        }
        .into();
    }
    for (action, mut hint) in &mut hints {
        hint.0 = match action {
            MenuAction::Difficulty => menu_difficulty_line(difficulty.current, &records),
            MenuAction::Continue => menu_profile_summary(
                &records,
                difficulty.current,
                audio_backend.status_label(&audio),
            ),
            MenuAction::Audio => format!(
                "Toggle music and combat sounds. {}",
                audio_backend.status_label(&audio)
            ),
            MenuAction::Begin => continue,
        };
    }
}

pub(super) fn menu_button_feedback(
    mut buttons: Query<(
        &Interaction,
        &MenuAction,
        &MenuHint,
        &mut BackgroundColor,
        &mut BorderColor,
    )>,
    mut hint_text: Query<&mut Text, With<MenuHintText>>,
) {
    let mut hint = "Choose your path. The reliquary awaits.";
    for (interaction, action, tooltip, mut background, mut border) in &mut buttons {
        let active = *interaction != Interaction::None;
        let primary = *action == MenuAction::Begin;
        background.0 = if active {
            Color::srgba(0.30, 0.14, 0.045, 0.96)
        } else if primary {
            Color::srgba(0.23, 0.075, 0.025, 0.90)
        } else {
            Color::srgba(0.018, 0.022, 0.026, 0.85)
        };
        *border = BorderColor::all(if active {
            IVORY
        } else if primary {
            GOLD
        } else {
            Color::srgba(0.53, 0.43, 0.28, 0.45)
        });
        if active {
            hint = &tooltip.0;
        }
        // The action itself is handled with keyboard and gamepad input in main_menu_input.
    }
    for mut text in &mut hint_text {
        if text.0 != hint {
            **text = hint.to_string();
        }
    }
}

pub(super) fn animate_menu(
    time: Res<Time>,
    windows: Query<&Window, With<PrimaryWindow>>,
    mut embers: Query<(&MenuEmber, &mut Node, &mut BackgroundColor), Without<MenuContent>>,
    mut content: Query<&mut Node, (With<MenuContent>, Without<MenuEmber>)>,
) {
    let t = time.elapsed_secs();
    for (ember, mut node, mut color) in &mut embers {
        let seed = ember.0 as f32;
        let phase = (t * (0.025 + seed * 0.0007) + seed * 0.137).fract();
        node.left = Val::Percent(5.0 + (seed * 7.13) % 87.0 + (t * 0.4 + seed).sin() * 1.2);
        node.top = Val::Percent(100.0 - phase * 100.0);
        color.0 = Color::srgba(
            1.0,
            0.38 + seed * 0.005,
            0.07,
            (phase * std::f32::consts::PI).sin() * 0.5,
        );
    }
    if let Ok(window) = windows.single() {
        let narrow = window.width() < 760.0;
        for mut node in &mut content {
            node.left = Val::Percent(7.0);
            node.max_width = Val::Percent(if narrow { 86.0 } else { 43.0 });
            node.top = Val::Px(if window.height() < 720.0 {
                16.0
            } else {
                window.height() * 0.08
            });
            node.bottom = Val::Px(16.0);
            node.row_gap = Val::Px(if window.height() < 720.0 { 7.0 } else { 12.0 });
        }
    }
}
