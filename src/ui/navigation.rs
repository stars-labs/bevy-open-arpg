//! Compact, discoverable access to the pack, talents, and journal.

use super::{
    HudUi, HudViewport,
    icons::{UiIcon, ui_icon},
};
use crate::{BuildOpen, GameState, InventoryOpen, JournalOpen, RunStats};
use bevy::prelude::*;

pub(super) struct HudNavigationPlugin;

impl Plugin for HudNavigationPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(GameState::InGame), spawn_navigation)
            .add_systems(
                Update,
                (interact_navigation, update_counters, place_navigation)
                    .run_if(in_state(GameState::InGame)),
            );
    }
}

#[derive(Component, Clone, Copy)]
enum NavigationAction {
    Inventory,
    Talents,
    Journal,
}

impl NavigationAction {
    fn icon(self) -> UiIcon {
        match self {
            Self::Inventory => UiIcon::Inventory,
            Self::Talents => UiIcon::Talents,
            Self::Journal => UiIcon::Journal,
        }
    }

    fn key(self) -> &'static str {
        match self {
            Self::Inventory => "I",
            Self::Talents => "K",
            Self::Journal => "J",
        }
    }

    fn tooltip(self) -> &'static str {
        match self {
            Self::Inventory => "Inventory [I]\nInspect and equip your spoils",
            Self::Talents => "Skills & talents [K]\nChoose runes and spend talent points",
            Self::Journal => "Journal [J]\nQuests, lore and chapter records",
        }
    }
}

#[derive(Component)]
struct NavigationTooltip;

#[derive(Component)]
struct NavigationDock;

pub(super) const DOCK_WIDTH: f32 = 150.0;

/// Desktop and portrait park the dock above the console; a landscape phone has
/// no room there, so it hangs directly under the minimap instead.
fn dock_position(viewport: HudViewport, viewport_width: f32) -> (Val, Val) {
    match viewport {
        HudViewport::Desktop => (Val::Auto, Val::Px(192.0)),
        HudViewport::Landscape => (
            Val::Px(super::HUD_MARGIN + super::MINIMAP_HEIGHT + 4.0),
            Val::Auto,
        ),
        HudViewport::Portrait => (
            Val::Auto,
            Val::Px(super::hud_layout::console_top(viewport, viewport_width) + 6.0),
        ),
    }
}

fn place_navigation(
    viewport: Res<HudViewport>,
    windows: Query<&Window, With<bevy::window::PrimaryWindow>>,
    mut docks: Query<&mut Node, With<NavigationDock>>,
) {
    let Ok(window) = windows.single() else {
        return;
    };
    let (top, bottom) = dock_position(*viewport, window.width());
    for mut node in &mut docks {
        let mut placed = node.clone();
        placed.top = top;
        placed.bottom = bottom;
        node.set_if_neq(placed);
    }
}

#[derive(Component, Clone, Copy)]
enum Counter {
    Gold,
    Kills,
}

impl Counter {
    fn tooltip(self) -> &'static str {
        match self {
            Self::Gold => "Gold\nCollected this run",
            Self::Kills => "Kills\nEnemies slain this run",
        }
    }
}

/// The hoverable icon-and-number pair; the number itself carries [`Counter`].
#[derive(Component)]
struct CounterTip(Counter);

fn spawn_navigation(mut commands: Commands, assets: Res<AssetServer>) {
    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                right: Val::Px(18.0),
                bottom: Val::Px(192.0),
                width: Val::Px(DOCK_WIDTH),
                max_width: Val::Percent(40.0),
                flex_direction: FlexDirection::Column,
                row_gap: Val::Px(4.0),
                padding: UiRect::all(Val::Px(4.0)),
                ..default()
            },
            BackgroundColor(Color::srgba(0.025, 0.021, 0.019, 0.42)),
            GlobalZIndex(20),
            HudUi,
            NavigationDock,
        ))
        .with_children(|panel| {
            panel
                .spawn((Node {
                    width: Val::Percent(100.0),
                    justify_content: JustifyContent::SpaceBetween,
                    column_gap: Val::Px(3.0),
                    ..default()
                },))
                .with_children(|strip| {
                    for action in [
                        NavigationAction::Inventory,
                        NavigationAction::Talents,
                        NavigationAction::Journal,
                    ] {
                        strip
                            .spawn((
                                Button,
                                action,
                                Node {
                                    width: Val::Px(44.0),
                                    height: Val::Px(50.0),
                                    min_width: Val::Px(0.0),
                                    flex_direction: FlexDirection::Column,
                                    align_items: AlignItems::Center,
                                    border: UiRect::bottom(Val::Px(1.0)),
                                    border_radius: BorderRadius::all(Val::Px(4.0)),
                                    ..default()
                                },
                                BorderColor::all(Color::srgba(0.58, 0.41, 0.22, 0.20)),
                                BackgroundColor(Color::srgba(0.08, 0.06, 0.035, 0.24)),
                            ))
                            .with_children(|button| {
                                button.spawn((
                                    ui_icon(&assets, action.icon()),
                                    Node {
                                        width: Val::Px(34.0),
                                        height: Val::Px(34.0),
                                        ..default()
                                    },
                                    Pickable::IGNORE,
                                ));
                                button.spawn((
                                    Text::new(action.key()),
                                    TextFont {
                                        font_size: FontSize::Px(11.0),
                                        ..default()
                                    },
                                    TextColor(Color::srgb(0.91, 0.79, 0.55)),
                                    Pickable::IGNORE,
                                ));
                            });
                    }
                });
            panel
                .spawn((Node {
                    justify_content: JustifyContent::SpaceBetween,
                    column_gap: Val::Px(8.0),
                    ..default()
                },))
                .with_children(|row| {
                    for (icon, counter) in [
                        (UiIcon::Gold, Counter::Gold),
                        (UiIcon::Skull, Counter::Kills),
                    ] {
                        row.spawn((
                            Node {
                                align_items: AlignItems::Center,
                                column_gap: Val::Px(3.0),
                                ..default()
                            },
                            Interaction::default(),
                            CounterTip(counter),
                        ))
                        .with_children(|stat| {
                            stat.spawn((
                                ui_icon(&assets, icon),
                                Node {
                                    width: Val::Px(21.0),
                                    height: Val::Px(21.0),
                                    ..default()
                                },
                                Pickable::IGNORE,
                            ));
                            stat.spawn((
                                Text::new("0"),
                                TextFont {
                                    font_size: FontSize::Px(12.0),
                                    ..default()
                                },
                                TextColor(Color::srgb(0.85, 0.74, 0.55)),
                                counter,
                                Pickable::IGNORE,
                            ));
                        });
                    }
                });
            panel.spawn((
                Node {
                    position_type: PositionType::Absolute,
                    right: Val::Px(0.0),
                    bottom: Val::Percent(100.0),
                    width: Val::Px(224.0),
                    padding: UiRect::all(Val::Px(9.0)),
                    border: UiRect::all(Val::Px(1.0)),
                    display: Display::None,
                    ..default()
                },
                Text::new(""),
                TextFont {
                    font_size: FontSize::Px(14.0),
                    ..default()
                },
                TextColor(Color::srgb(0.93, 0.83, 0.64)),
                BackgroundColor(Color::srgba(0.035, 0.025, 0.019, 0.97)),
                BorderColor::all(Color::srgb(0.58, 0.41, 0.22)),
                NavigationTooltip,
                Pickable::IGNORE,
            ));
        });
}

#[allow(clippy::type_complexity)]
fn interact_navigation(
    mut buttons: Query<(
        Ref<Interaction>,
        &NavigationAction,
        &mut BorderColor,
        &mut BackgroundColor,
    )>,
    counters: Query<(&Interaction, &CounterTip)>,
    mut tooltip: Query<(&mut Node, &mut Text), With<NavigationTooltip>>,
    mut inventory: ResMut<InventoryOpen>,
    mut build: ResMut<BuildOpen>,
    mut journal: ResMut<JournalOpen>,
) {
    let mut hovered = None;
    for (interaction, action, mut border, mut background) in &mut buttons {
        let active = match *action {
            NavigationAction::Inventory => inventory.open,
            NavigationAction::Talents => build.open,
            NavigationAction::Journal => journal.open,
        };
        if *interaction != Interaction::None {
            hovered = Some(action.tooltip());
        }
        *border = BorderColor::all(if active || *interaction != Interaction::None {
            Color::srgb(0.95, 0.72, 0.36)
        } else {
            Color::srgba(0.58, 0.41, 0.22, 0.20)
        });
        background.0 = if active {
            Color::srgba(0.30, 0.14, 0.045, 0.8)
        } else {
            Color::srgba(0.08, 0.06, 0.035, 0.24)
        };
        if *interaction == Interaction::Pressed && interaction.is_changed() {
            match *action {
                NavigationAction::Inventory => {
                    (inventory.open, build.open) =
                        crate::toggled_exclusive_panel(inventory.open, build.open);
                }
                NavigationAction::Talents => {
                    (build.open, inventory.open) =
                        crate::toggled_exclusive_panel(build.open, inventory.open);
                }
                NavigationAction::Journal => journal.open = !journal.open,
            }
        }
    }
    for (interaction, tip) in &counters {
        if *interaction != Interaction::None {
            hovered = Some(tip.0.tooltip());
        }
    }
    for (mut node, mut text) in &mut tooltip {
        node.display = if hovered.is_some() {
            Display::Flex
        } else {
            Display::None
        };
        if let Some(label) = hovered {
            **text = label.to_string();
        }
    }
}

fn update_counters(stats: Res<RunStats>, mut counters: Query<(&Counter, &mut Text)>) {
    for (counter, mut text) in &mut counters {
        let count = match counter {
            Counter::Gold => stats.gold,
            Counter::Kills => stats.kills,
        };
        let label = if count >= 10_000 {
            format!("{:.1}k", count as f32 / 1000.0)
        } else {
            count.to_string()
        };
        if text.0 != label {
            **text = label;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn landscape_dock_sits_between_the_minimap_and_the_console() {
        let (width, height) = (844.0, 390.0);
        let viewport = HudViewport::classify(width, height);
        let (top, bottom) = dock_position(viewport, width);
        assert_eq!(bottom, Val::Auto);
        let Val::Px(top) = top else {
            panic!("landscape dock is anchored from the top");
        };
        assert!(top >= super::super::HUD_MARGIN + super::super::MINIMAP_HEIGHT);
        // 4 padding + 50 button + 4 gap + 21 counters + 4 padding.
        let dock_bottom = top + 83.0;
        assert!(dock_bottom <= height - super::super::hud_layout::console_top(viewport, width));
    }
}
