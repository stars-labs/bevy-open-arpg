//! Player-facing settings, shared by the title screen and pause menu.
use super::localization::{NativeLanguageName, UiLanguage};
use crate::{GameState, OpenArpgUserSettings, PauseState};
use bevy::prelude::*;
use bevy::settings::SaveSettingsDeferred;
use std::time::Duration;

pub(super) struct SettingsPanelPlugin;

#[derive(Resource, Default)]
pub(crate) struct SettingsPanelState {
    pub(crate) open: bool,
    resume_on_close: bool,
}
impl SettingsPanelState {
    fn show(&mut self, in_game: bool, pause: &mut PauseState) {
        self.resume_on_close = in_game && !pause.paused;
        if in_game {
            pause.paused = true;
        }
        self.open = true;
    }
    fn dismiss(&mut self, pause: &mut PauseState) {
        if self.resume_on_close {
            pause.paused = false;
        }
        self.resume_on_close = false;
        self.open = false;
    }
}

#[derive(Component)]
struct SettingsEntry;
#[derive(Component)]
struct SettingsTooltip;
#[derive(Component)]
struct SettingsModal;
#[derive(Component, Clone, Copy)]
enum SettingsAction {
    Open,
    Close,
    #[cfg(target_arch = "wasm32")]
    Downloads,
    Language(UiLanguage),
}

impl Plugin for SettingsPanelPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<SettingsPanelState>()
            .add_systems(Startup, spawn_settings)
            .add_systems(
                Update,
                (
                    interact_settings.after(crate::toggle_pause),
                    update_settings,
                )
                    .chain(),
            );
    }
}

fn spawn_settings(mut commands: Commands) {
    commands
        .spawn((
            Button,
            SettingsAction::Open,
            SettingsEntry,
            Node {
                position_type: PositionType::Absolute,
                right: Val::Px(20.0),
                top: Val::Px(20.0),
                width: Val::Px(44.0),
                height: Val::Px(44.0),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                border_radius: BorderRadius::all(Val::Px(5.0)),
                display: Display::None,
                ..default()
            },
            BackgroundColor(Color::srgb(0.07, 0.055, 0.035)),
            BorderColor::all(Color::srgb(0.55, 0.40, 0.21)),
            GlobalZIndex(100),
        ))
        .with_children(|button| {
            // Three adjustable rails remain legible at HUD size, without a font glyph.
            button
                .spawn((
                    Node {
                        width: Val::Px(22.0),
                        height: Val::Px(22.0),
                        flex_direction: FlexDirection::Column,
                        justify_content: JustifyContent::SpaceAround,
                        ..default()
                    },
                    Pickable::IGNORE,
                ))
                .with_children(|icon| {
                    for offset in [4.0, 14.0, 7.0] {
                        icon.spawn((
                            Node {
                                width: Val::Percent(100.0),
                                height: Val::Px(2.0),
                                ..default()
                            },
                            BackgroundColor(Color::srgb(0.70, 0.55, 0.33)),
                            Pickable::IGNORE,
                        ))
                        .with_child((
                            Node {
                                position_type: PositionType::Absolute,
                                left: Val::Px(offset),
                                top: Val::Px(-2.0),
                                width: Val::Px(5.0),
                                height: Val::Px(6.0),
                                border_radius: BorderRadius::all(Val::Px(1.0)),
                                ..default()
                            },
                            BackgroundColor(Color::srgb(0.95, 0.83, 0.62)),
                            Pickable::IGNORE,
                        ));
                    }
                });
            button.spawn((
                SettingsTooltip,
                Text::new("Settings"),
                Node {
                    position_type: PositionType::Absolute,
                    top: Val::Px(48.0),
                    right: Val::Px(0.0),
                    padding: UiRect::axes(Val::Px(10.0), Val::Px(6.0)),
                    display: Display::None,
                    ..default()
                },
                TextFont {
                    font_size: 14.0.into(),
                    ..default()
                },
                TextColor(Color::srgb(0.95, 0.83, 0.62)),
                BackgroundColor(Color::srgba(0.025, 0.021, 0.019, 0.97)),
                Pickable::IGNORE,
            ));
        });

    commands
        .spawn((
            Button,
            SettingsModal,
            Node {
                position_type: PositionType::Absolute,
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                display: Display::None,
                ..default()
            },
            BackgroundColor(Color::srgba(0.005, 0.004, 0.008, 0.92)),
            GlobalZIndex(200),
        ))
        .with_children(|overlay| {
            overlay
                .spawn((
                    Node {
                        width: Val::Px(480.0),
                        max_width: Val::Percent(92.0),
                        flex_direction: FlexDirection::Column,
                        row_gap: Val::Px(8.0),
                        padding: UiRect::all(Val::Px(16.0)),
                        border: UiRect::all(Val::Px(1.0)),
                        ..default()
                    },
                    BackgroundColor(Color::srgb(0.055, 0.042, 0.031)),
                    BorderColor::all(Color::srgb(0.59, 0.44, 0.26)),
                ))
                .with_children(|panel| {
                    panel.spawn((
                        Text::new("Settings"),
                        TextFont {
                            font_size: 30.0.into(),
                            ..default()
                        },
                        TextColor(Color::srgb(0.95, 0.83, 0.62)),
                    ));
                    for (language, label) in [
                        (UiLanguage::Chinese, "简体中文"),
                        (UiLanguage::English, "English"),
                    ] {
                        panel
                            .spawn((
                                Button,
                                SettingsAction::Language(language),
                                Node {
                                    width: Val::Percent(100.0),
                                    min_height: Val::Px(44.0),
                                    padding: UiRect::all(Val::Px(8.0)),
                                    border: UiRect::all(Val::Px(2.0)),
                                    ..default()
                                },
                                BackgroundColor(Color::srgb(0.09, 0.07, 0.05)),
                                BorderColor::all(Color::srgb(0.30, 0.25, 0.19)),
                            ))
                            .with_child((
                                Text::new(label),
                                NativeLanguageName,
                                TextFont {
                                    font_size: 20.0.into(),
                                    ..default()
                                },
                            ));
                    }
                    #[cfg(target_arch = "wasm32")]
                    panel
                        .spawn((
                            Button,
                            SettingsAction::Downloads,
                            Node {
                                width: Val::Percent(100.),
                                min_height: Val::Px(44.),
                                align_items: AlignItems::Center,
                                justify_content: JustifyContent::Center,
                                ..default()
                            },
                            BackgroundColor(Color::srgb(0.09, 0.07, 0.05)),
                        ))
                        .with_child((
                            Text::new("Downloads & offline play"),
                            TextFont {
                                font_size: 18.0.into(),
                                ..default()
                            },
                        ));
                    panel
                        .spawn((
                            Button,
                            SettingsAction::Close,
                            Node {
                                width: Val::Percent(100.0),
                                padding: UiRect::all(Val::Px(12.0)),
                                justify_content: JustifyContent::Center,
                                ..default()
                            },
                            BackgroundColor(Color::srgb(0.26, 0.12, 0.045)),
                        ))
                        .with_child((
                            Text::new("Back"),
                            TextFont {
                                font_size: 18.0.into(),
                                ..default()
                            },
                        ));
                });
        });
}

#[allow(clippy::too_many_arguments)]
fn interact_settings(
    mut commands: Commands,
    keys: Res<ButtonInput<KeyCode>>,
    state: Res<State<GameState>>,
    mut pause: ResMut<PauseState>,
    mut panel: ResMut<SettingsPanelState>,
    mut language: ResMut<UiLanguage>,
    mut preferences: ResMut<OpenArpgUserSettings>,
    buttons: Query<(&Interaction, &SettingsAction), Changed<Interaction>>,
) {
    #[cfg(target_arch = "wasm32")]
    if crate::web_runtime::downloads_open() {
        return;
    }
    if panel.open && keys.just_pressed(KeyCode::Escape) {
        panel.dismiss(&mut pause);
        return;
    }
    for (interaction, action) in &buttons {
        if *interaction != Interaction::Pressed {
            continue;
        }
        match action {
            SettingsAction::Open => panel.show(*state.get() == GameState::InGame, &mut pause),
            SettingsAction::Close => panel.dismiss(&mut pause),
            #[cfg(target_arch = "wasm32")]
            SettingsAction::Downloads => crate::web_runtime::open_downloads(),
            SettingsAction::Language(selected) => {
                *language = *selected;
                #[cfg(target_arch = "wasm32")]
                crate::web_runtime::save_language(*selected);
                preferences.language = match selected {
                    UiLanguage::Chinese => "zh-CN",
                    UiLanguage::English => "en",
                }
                .into();
                commands.queue(SaveSettingsDeferred(Duration::from_millis(250)));
            }
        }
    }
}

#[allow(clippy::type_complexity)]
fn update_settings(
    state: Res<State<GameState>>,
    mut pause: ResMut<PauseState>,
    mut panel: ResMut<SettingsPanelState>,
    language: Res<UiLanguage>,
    mut nodes: Query<
        (&mut Node, Option<&SettingsEntry>),
        (
            Or<(With<SettingsEntry>, With<SettingsModal>)>,
            Without<SettingsTooltip>,
        ),
    >,
    mut tooltips: Query<&mut Node, With<SettingsTooltip>>,
    mut buttons: Query<(
        &Interaction,
        &SettingsAction,
        &mut BackgroundColor,
        Option<&mut BorderColor>,
    )>,
) {
    let available = *state.get() == GameState::MainMenu || *state.get() == GameState::InGame;
    if !available {
        panel.dismiss(&mut pause);
    }
    for (mut node, entry) in &mut nodes {
        let visible = if entry.is_some() {
            let in_game = *state.get() == GameState::InGame;
            node.top = Val::Px(if in_game { 0.0 } else { 20.0 });
            node.left = if in_game { Val::Px(20.0) } else { Val::Auto };
            node.right = if in_game { Val::Auto } else { Val::Px(20.0) };
            available && !panel.open
        } else {
            panel.open
        };
        node.display = if visible {
            Display::Flex
        } else {
            Display::None
        };
    }
    let mut entry_hovered = false;
    for (interaction, action, mut background, border) in &mut buttons {
        if matches!(action, SettingsAction::Open) && *interaction == Interaction::Hovered {
            entry_hovered = true;
        }
        let selected = matches!(action, SettingsAction::Language(choice) if *choice == *language);
        background.0 = if selected {
            Color::srgb(0.27, 0.14, 0.055)
        } else if *interaction != Interaction::None {
            Color::srgb(0.19, 0.13, 0.075)
        } else {
            Color::srgb(0.09, 0.07, 0.05)
        };
        if let Some(mut border) = border {
            *border = BorderColor::all(if selected {
                Color::srgb(0.95, 0.70, 0.32)
            } else {
                Color::srgb(0.40, 0.31, 0.20)
            });
        }
    }
    for mut node in &mut tooltips {
        node.display = if available && !panel.open && entry_hovered {
            Display::Flex
        } else {
            Display::None
        };
        let in_game = *state.get() == GameState::InGame;
        node.left = if in_game { Val::Px(0.0) } else { Val::Auto };
        node.right = if in_game { Val::Auto } else { Val::Px(0.0) };
    }
}

#[cfg(test)]
mod mobile_pause_tests {
    use super::*;
    #[test]
    fn settings_can_pause_and_resume_without_a_keyboard() {
        let mut panel = SettingsPanelState::default();
        let mut pause = PauseState::default();
        panel.show(true, &mut pause);
        assert!(panel.open && pause.paused);
        panel.dismiss(&mut pause);
        assert!(!panel.open && !pause.paused);
    }
    #[test]
    fn closing_settings_preserves_an_existing_manual_pause() {
        let mut panel = SettingsPanelState::default();
        let mut pause = PauseState { paused: true };
        panel.show(true, &mut pause);
        panel.dismiss(&mut pause);
        assert!(pause.paused);
    }
}
