//! Class choice overlay on the title screen: four cards, one descent.
use super::localization::{NativeLanguageName, UiLanguage};
use super::menu::MenuAction;
use super::*;
use crate::class::{ClassCatalog, ClassDef, HeroClass, SelectedClass};

pub(super) struct ClassSelectPlugin;

#[derive(Resource, Default)]
pub(crate) struct ClassSelectState {
    pub(crate) open: bool,
    pub(crate) focus: HeroClass,
}

/// The player confirmed a class; the title screen starts the descent.
#[derive(Message, Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct BeginDescent;

#[derive(Component)]
struct ClassSelectModal;
#[derive(Component)]
struct ClassViewport;
#[derive(Component)]
enum ClassLayoutElement {
    Card,
    Portrait,
    Viewport,
    Actions,
    Action,
}
#[derive(Component, Clone, Copy)]
struct ClassCard(HeroClass);
#[derive(Component, Clone, Copy)]
enum ClassSelectAction {
    Confirm,
    Back,
}
#[derive(Component, Clone, Copy)]
enum ClassText {
    Title,
    Subtitle,
    Name(HeroClass),
    Role(HeroClass),
    Summary(HeroClass),
    Passive(HeroClass),
    Difficulty(HeroClass),
    Confirm,
    Back,
}

const GOLD: Color = Color::srgb(0.83, 0.67, 0.40);
const IVORY: Color = Color::srgb(0.95, 0.89, 0.76);
const MUTED: Color = Color::srgb(0.65, 0.62, 0.54);

impl Plugin for ClassSelectPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<ClassSelectState>()
            .add_message::<BeginDescent>()
            .add_systems(OnEnter(GameState::MainMenu), spawn_class_select)
            .add_systems(OnExit(GameState::MainMenu), close_class_select)
            .add_systems(
                Update,
                (
                    interact_class_select,
                    resize_class_select,
                    scroll_class_select,
                    update_class_select,
                )
                    .chain()
                    .run_if(in_state(GameState::MainMenu)),
            );
    }
}

fn text(
    parent: &mut ChildSpawnerCommands,
    assets: &AssetServer,
    marker: ClassText,
    size: f32,
    heading: bool,
    color: Color,
) {
    parent.spawn((
        Text::new(""),
        marker,
        NativeLanguageName,
        TextFont {
            font: assets
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
        TextLayout::new(Justify::Left, LineBreak::WordOrCharacter),
    ));
}

fn spawn_class_select(
    mut commands: Commands,
    assets: Res<AssetServer>,
    game_assets: Res<GameAssets>,
) {
    commands
        .spawn((
            ClassSelectModal,
            MenuUi,
            Node {
                position_type: PositionType::Absolute,
                left: Val::Px(0.0),
                right: Val::Px(0.0),
                top: Val::Px(0.0),
                bottom: Val::Px(0.0),
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                row_gap: Val::Px(12.0),
                padding: UiRect::all(Val::Px(24.0)),
                display: Display::None,
                ..default()
            },
            BackgroundColor(Color::srgba(0.012, 0.010, 0.012, 0.94)),
            GlobalZIndex(90),
        ))
        .with_children(|modal| {
            text(modal, &assets, ClassText::Title, 34.0, true, IVORY);
            text(modal, &assets, ClassText::Subtitle, 15.0, false, MUTED);
            modal
                .spawn((
                    ClassViewport,
                    ClassLayoutElement::Viewport,
                    ScrollPosition::default(),
                    Node {
                        overflow: Overflow::scroll_y(),
                        max_height: Val::Px(500.0),
                        flex_shrink: 0.0,
                        align_content: AlignContent::FlexStart,
                        column_gap: Val::Px(12.0),
                        row_gap: Val::Px(14.0),
                        flex_wrap: FlexWrap::Wrap,
                        justify_content: JustifyContent::Center,
                        max_width: Val::Percent(100.0),
                        ..default()
                    },
                ))
                .with_children(|row| {
                    for class in HeroClass::ALL {
                        row.spawn((
                            Button,
                            ClassCard(class),
                            ClassLayoutElement::Card,
                            Node {
                                flex_shrink: 0.0,
                                width: Val::Px(272.0),
                                flex_direction: FlexDirection::Column,
                                row_gap: Val::Px(8.0),
                                padding: UiRect::all(Val::Px(12.0)),
                                border: UiRect::all(Val::Px(2.0)),
                                ..default()
                            },
                            BackgroundColor(Color::srgba(0.03, 0.026, 0.03, 0.96)),
                            BorderColor::all(Color::srgba(0.45, 0.36, 0.22, 0.5)),
                        ))
                        .with_children(|card| {
                            card.spawn((
                                ClassLayoutElement::Portrait,
                                ImageNode::new(
                                    game_assets.ui_class_portraits[class.index()].clone(),
                                ),
                                Node {
                                    width: Val::Percent(100.0),
                                    height: Val::Px(230.0),
                                    ..default()
                                },
                                Pickable::IGNORE,
                            ));
                            text(card, &assets, ClassText::Name(class), 22.0, true, IVORY);
                            text(card, &assets, ClassText::Role(class), 13.0, true, GOLD);
                            text(
                                card,
                                &assets,
                                ClassText::Difficulty(class),
                                12.0,
                                false,
                                MUTED,
                            );
                            text(card, &assets, ClassText::Summary(class), 13.0, false, IVORY);
                            text(card, &assets, ClassText::Passive(class), 12.0, false, GOLD);
                        });
                    }
                });
            modal
                .spawn((
                    ClassLayoutElement::Actions,
                    Node {
                        column_gap: Val::Px(14.0),
                        row_gap: Val::Px(8.0),
                        max_width: Val::Percent(100.0),
                        ..default()
                    },
                ))
                .with_children(|buttons| {
                    for (action, marker, primary) in [
                        (ClassSelectAction::Confirm, ClassText::Confirm, true),
                        (ClassSelectAction::Back, ClassText::Back, false),
                    ] {
                        buttons
                            .spawn((
                                Button,
                                action,
                                ClassLayoutElement::Action,
                                Node {
                                    padding: UiRect::axes(Val::Px(26.0), Val::Px(10.0)),
                                    border: UiRect::all(Val::Px(1.0)),
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
                            ))
                            .with_children(|button| {
                                text(button, &assets, marker, 17.0, true, IVORY);
                            });
                    }
                });
        });
}

fn close_class_select(mut state: ResMut<ClassSelectState>) {
    state.open = false;
}

fn next_class(class: HeroClass, step: i32) -> HeroClass {
    let count = HeroClass::ALL.len() as i32;
    let index = (class.index() as i32 + step).rem_euclid(count) as usize;
    HeroClass::ALL[index]
}

#[allow(clippy::too_many_arguments)]
fn interact_class_select(
    mut state: ResMut<ClassSelectState>,
    mut selected: ResMut<SelectedClass>,
    settings: Res<settings::SettingsPanelState>,
    keyboard: Res<ButtonInput<KeyCode>>,
    gamepads: Query<&Gamepad>,
    menu_buttons: Query<(&Interaction, &MenuAction), Changed<Interaction>>,
    cards: Query<(&Interaction, &ClassCard), Changed<Interaction>>,
    actions: Query<(&Interaction, &ClassSelectAction), Changed<Interaction>>,
    mut begin: MessageWriter<BeginDescent>,
) {
    if settings.open {
        return;
    }
    let pressed = |buttons: &[GamepadButton]| {
        gamepads
            .iter()
            .any(|gamepad| buttons.iter().any(|button| gamepad.just_pressed(*button)))
    };
    if !state.open {
        let begin_clicked = menu_buttons.iter().any(|(interaction, action)| {
            *interaction == Interaction::Pressed && *action == MenuAction::Begin
        });
        if begin_clicked
            || keyboard.just_pressed(KeyCode::Space)
            || keyboard.just_pressed(KeyCode::Enter)
            || pressed(&[GamepadButton::South, GamepadButton::Start])
        {
            state.open = true;
            state.focus = selected.0;
        }
        return;
    }
    for (interaction, card) in &cards {
        if *interaction == Interaction::Pressed {
            // A card chooses the preview only. Starting on press would launch a
            // descent when a finger begins scrolling the already focused card.
            state.focus = card.0;
        }
    }
    if keyboard.just_pressed(KeyCode::ArrowLeft)
        || keyboard.just_pressed(KeyCode::KeyA)
        || pressed(&[GamepadButton::DPadLeft])
    {
        state.focus = next_class(state.focus, -1);
    }
    if keyboard.just_pressed(KeyCode::ArrowRight)
        || keyboard.just_pressed(KeyCode::KeyD)
        || pressed(&[GamepadButton::DPadRight])
    {
        state.focus = next_class(state.focus, 1);
    }
    let confirm = actions.iter().any(|(interaction, action)| {
        *interaction == Interaction::Pressed && matches!(action, ClassSelectAction::Confirm)
    }) || keyboard.just_pressed(KeyCode::Enter)
        || keyboard.just_pressed(KeyCode::Space)
        || pressed(&[GamepadButton::South, GamepadButton::Start]);
    let back = actions.iter().any(|(interaction, action)| {
        *interaction == Interaction::Pressed && matches!(action, ClassSelectAction::Back)
    }) || keyboard.just_pressed(KeyCode::Escape)
        || pressed(&[GamepadButton::East]);
    if confirm {
        selected.0 = state.focus;
        begin.write(BeginDescent);
        state.open = false;
    } else if back {
        state.open = false;
    }
}

fn difficulty_pips(difficulty: u32) -> String {
    let filled = difficulty.clamp(1, 3) as usize;
    format!("{}{}", "◆".repeat(filled), "◇".repeat(3 - filled))
}

fn card_text(class: &ClassDef, marker: ClassText, zh: bool) -> Option<String> {
    Some(match marker {
        ClassText::Name(_) => {
            if zh {
                class.name_zh.clone()
            } else {
                class.name.clone()
            }
        }
        ClassText::Role(_) => {
            if zh {
                class.role_zh.clone()
            } else {
                class.role.clone()
            }
        }
        ClassText::Summary(_) => {
            if zh {
                class.summary_zh.clone()
            } else {
                class.summary.clone()
            }
        }
        ClassText::Passive(_) => {
            if zh {
                format!("{}：{}", class.passive_name_zh, class.passive_summary_zh)
            } else {
                format!("{}: {}", class.passive_name, class.passive_summary)
            }
        }
        ClassText::Difficulty(_) => {
            let resource = if zh {
                super::skills::resource_name_zh(class.resource.kind).to_string()
            } else {
                class.resource.kind.label().to_string()
            };
            if zh {
                format!(
                    "上手 {} · 资源：{resource}",
                    difficulty_pips(class.difficulty)
                )
            } else {
                format!(
                    "Difficulty {} · Resource: {resource}",
                    difficulty_pips(class.difficulty)
                )
            }
        }
        _ => return None,
    })
}

#[allow(clippy::type_complexity)]
fn update_class_select(
    state: Res<ClassSelectState>,
    language: Res<UiLanguage>,
    catalog: Res<ClassCatalog>,
    windows: Query<&Window, With<PrimaryWindow>>,
    mut modals: Query<&mut Node, With<ClassSelectModal>>,
    mut cards: Query<(&ClassCard, &mut BorderColor, &mut BackgroundColor)>,
    mut texts: Query<(&ClassText, &mut Text)>,
) {
    for mut node in &mut modals {
        node.display = if state.open {
            Display::Flex
        } else {
            Display::None
        };
    }
    if !state.open {
        return;
    }
    for (card, mut border, mut background) in &mut cards {
        let focused = card.0 == state.focus;
        *border = BorderColor::all(if focused {
            GOLD
        } else {
            Color::srgba(0.45, 0.36, 0.22, 0.5)
        });
        background.0 = if focused {
            Color::srgba(0.13, 0.06, 0.03, 0.97)
        } else {
            Color::srgba(0.03, 0.026, 0.03, 0.96)
        };
    }
    let zh = *language == UiLanguage::Chinese;
    for (marker, mut text) in &mut texts {
        let value = match marker {
            ClassText::Title => {
                if zh {
                    "选择你的道路".to_string()
                } else {
                    "Choose Your Path".to_string()
                }
            }
            ClassText::Subtitle => {
                if zh {
                    if windows
                        .single()
                        .is_ok_and(|w| w.width() < 1200.0 || w.height() < 700.0)
                    {
                        "点选职业 · 滑动浏览 · 下方确认"
                    } else {
                        "← → 切换 · 回车确认 · Esc 返回"
                    }
                    .to_string()
                } else {
                    if windows
                        .single()
                        .is_ok_and(|w| w.width() < 1200.0 || w.height() < 700.0)
                    {
                        "Tap a class · Swipe to browse · Confirm below"
                    } else {
                        "← → switch · Enter to confirm · Esc to go back"
                    }
                    .to_string()
                }
            }
            ClassText::Confirm => {
                let class = catalog.get(state.focus);
                if zh {
                    format!("以{}之身踏入深渊", class.name_zh)
                } else {
                    format!("Descend as the {}", class.name)
                }
            }
            ClassText::Back => {
                if zh {
                    "返回".to_string()
                } else {
                    "Back".to_string()
                }
            }
            ClassText::Name(class)
            | ClassText::Role(class)
            | ClassText::Summary(class)
            | ClassText::Passive(class)
            | ClassText::Difficulty(class) => {
                let Some(value) = card_text(catalog.get(*class), *marker, zh) else {
                    continue;
                };
                value
            }
        };
        if text.0 != value {
            text.0 = value;
        }
    }
}

fn resize_class_select(
    windows: Query<&Window, With<PrimaryWindow>>,
    mut elements: Query<(&ClassLayoutElement, &mut Node)>,
    mut fonts: Query<(&ClassText, &mut TextFont)>,
) {
    let Ok(window) = windows.single() else { return };
    let narrow = window.width() < 600.0;
    let width = if narrow {
        (window.width() - 60.0).max(160.0)
    } else {
        ((window.width() - 84.0) / 4.0).clamp(180.0, 272.0)
    };
    for (marker, mut font) in &mut fonts {
        if matches!(marker, ClassText::Title) {
            font.font_size = FontSize::Px(if narrow { 24.0 } else { 34.0 });
        }
    }
    for (element, mut node) in &mut elements {
        match element {
            ClassLayoutElement::Card => node.width = Val::Px(width),
            ClassLayoutElement::Portrait => {
                node.height = Val::Px((window.height() * 0.24).clamp(120.0, 230.0))
            }
            ClassLayoutElement::Viewport => {
                node.max_height =
                    Val::Px((window.height() - if narrow { 260.0 } else { 200.0 }).max(160.0))
            }
            ClassLayoutElement::Actions => {
                node.flex_direction = if narrow {
                    FlexDirection::Column
                } else {
                    FlexDirection::Row
                };
                node.width = if narrow {
                    Val::Percent(100.0)
                } else {
                    Val::Auto
                };
            }
            ClassLayoutElement::Action => {
                node.width = if narrow {
                    Val::Percent(100.0)
                } else {
                    Val::Auto
                };
                node.min_height = Val::Px(44.0);
                node.justify_content = JustifyContent::Center;
                node.padding =
                    UiRect::axes(Val::Px(if narrow { 8.0 } else { 26.0 }), Val::Px(10.0));
            }
        }
    }
}
fn scroll_class_select(
    state: Res<ClassSelectState>,
    mut wheel: MessageReader<bevy::input::mouse::MouseWheel>,
    touches: Res<Touches>,
    windows: Query<&Window, With<PrimaryWindow>>,
    mut drag: Local<Option<(u64, Vec2)>>,
    mut viewports: Query<
        (&ComputedNode, &UiGlobalTransform, &mut ScrollPosition),
        With<ClassViewport>,
    >,
) {
    let delta: f32 = wheel
        .read()
        .map(|event| {
            event.y
                * if event.unit == bevy::input::mouse::MouseScrollUnit::Line {
                    36.0
                } else {
                    1.0
                }
        })
        .sum();
    if !state.open {
        *drag = None;
        return;
    }
    let Ok(window) = windows.single() else {
        *drag = None;
        return;
    };
    if drag.is_none() {
        for touch in touches.iter_just_pressed() {
            if viewports.iter().any(|(node, transform, _)| {
                node_under_cursor(node, transform, touch.position() * window.scale_factor())
            }) {
                *drag = Some((touch.id(), touch.position()));
                break;
            }
        }
    }
    let movement = drag.and_then(|(id, last)| {
        touches
            .get_pressed(id)
            .map(|touch| (id, touch.position(), touch.position().y - last.y))
    });
    *drag = movement.map(|(id, p, _)| (id, p));
    let touch_delta = movement.map_or(0.0, |(_, _, dy)| dy);
    for (node, _, mut scroll) in &mut viewports {
        let max = ((node.content_size.y - node.size.y) * node.inverse_scale_factor).max(0.0);
        scroll.0.y = (scroll.0.y - delta - touch_delta).clamp(0.0, max);
    }
}
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pressing_focused_card_does_not_launch_while_starting_a_scroll() {
        let mut app = App::new();
        app.insert_resource(ClassSelectState {
            open: true,
            focus: HeroClass::Warden,
        })
        .init_resource::<SelectedClass>()
        .init_resource::<settings::SettingsPanelState>()
        .init_resource::<ButtonInput<KeyCode>>()
        .add_message::<BeginDescent>()
        .add_systems(Update, interact_class_select);
        app.world_mut()
            .spawn((Interaction::Pressed, ClassCard(HeroClass::Warden)));
        app.update();
        assert!(app.world().resource::<ClassSelectState>().open);
        assert_eq!(app.world().resource::<Messages<BeginDescent>>().len(), 0);
        app.world_mut()
            .spawn((Interaction::Pressed, ClassSelectAction::Confirm));
        app.update();
        assert!(!app.world().resource::<ClassSelectState>().open);
        assert_eq!(app.world().resource::<Messages<BeginDescent>>().len(), 1);
    }

    #[test]
    fn focus_wraps_around_the_four_classes() {
        assert_eq!(next_class(HeroClass::Warden, -1), HeroClass::Soulbinder);
        assert_eq!(next_class(HeroClass::Soulbinder, 1), HeroClass::Warden);
        assert_eq!(next_class(HeroClass::Warden, 1), HeroClass::Stalker);
    }

    #[test]
    fn card_copy_is_fully_chinese_when_chinese_is_selected() {
        let catalog = crate::class::test_catalog();
        for class in HeroClass::ALL {
            let def = catalog.get(class);
            for marker in [
                ClassText::Name(class),
                ClassText::Role(class),
                ClassText::Summary(class),
                ClassText::Passive(class),
                ClassText::Difficulty(class),
            ] {
                let value = card_text(def, marker, true).unwrap();
                assert!(
                    !value.chars().any(|c| c.is_ascii_alphabetic()),
                    "{class:?}: {value}"
                );
            }
        }
        assert_eq!(difficulty_pips(2), "◆◆◇");
    }
}
