//! A quiet defeat screen: one cause, three run metrics, and an immediate way back.
use super::icons::{UiIcon, ui_icon};
use super::*;

#[derive(Component, Clone, Copy, PartialEq, Eq)]
pub(crate) enum DeathAction {
    Retry,
    MainMenu,
}

const IVORY: Color = Color::srgb(0.94, 0.88, 0.75);
const GOLD: Color = Color::srgb(0.75, 0.57, 0.32);
const MUTED: Color = Color::srgb(0.63, 0.65, 0.65);

fn lettering(text: impl Into<String>, size: f32, color: Color) -> impl Bundle {
    (
        Text::new(text),
        TextFont {
            font_size: FontSize::Px(size),
            ..default()
        },
        TextColor(color),
        TextLayout::justify(Justify::Center),
    )
}

fn defeat_cause(report: &DeathWardReport) -> &'static str {
    if report.max_revives > 0 && report.revives_used >= report.max_revives {
        "Your soul ward has no revives remaining."
    } else if report.gold_available < report.gold_penalty {
        "Not enough gold to revive."
    } else {
        "The reliquary claimed another soul."
    }
}

fn defeat_hint(progress: &ChapterProgress) -> &'static str {
    if progress.phase == ChapterPhase::Boss {
        "Break the boss's guard, then unleash your fury."
    } else {
        "Evade heavy attacks. Drink a potion before your life runs low."
    }
}

pub(super) fn spawn_game_over(
    mut commands: Commands,
    assets: Res<AssetServer>,
    stats: Res<RunStats>,
    progress: Res<ChapterProgress>,
    difficulty: Res<DifficultySettings>,
    mut records: ResMut<ChapterRecords>,
    death_report: Res<DeathWardReport>,
) {
    let remnant_reward = claim_soul_remnant(&mut records, &stats, difficulty.current);
    let deed_reward = claim_completed_season_deeds(&mut records);
    if (!remnant_reward.is_empty() || !deed_reward.is_empty())
        && let Err(err) = write_chapter_record_profile(PROFILE_PATH, &records)
    {
        warn!("failed to write chapter profile after soul remnant claim: {err}");
    }
    commands
        .spawn((
            Node {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                overflow: Overflow::clip(),
                ..default()
            },
            BackgroundColor(Color::srgb(0.006, 0.008, 0.012)),
            MenuUi,
        ))
        .with_children(|root| {
            root.spawn((
                ImageNode {
                    image: assets.load(MAIN_MENU_BACKGROUND_IMAGE),
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
                BackgroundColor(Color::srgba(0.003, 0.005, 0.009, 0.74)),
            ));
            root.spawn((
                Node {
                    width: Val::Percent(92.0),
                    max_width: Val::Px(620.0),
                    padding: UiRect::axes(Val::Px(30.0), Val::Px(24.0)),
                    flex_direction: FlexDirection::Column,
                    align_items: AlignItems::Center,
                    row_gap: Val::Px(12.0),
                    ..default()
                },
                panel_image_node(&assets),
                BoxShadow(vec![ShadowStyle {
                    color: Color::srgba(0.0, 0.0, 0.0, 0.8),
                    blur_radius: Val::Px(60.0),
                    ..default()
                }]),
            ))
            .with_children(|panel| {
                panel.spawn((
                    ui_icon(&assets, UiIcon::Skull),
                    Node {
                        width: Val::Px(48.0),
                        height: Val::Px(48.0),
                        ..default()
                    },
                ));
                panel.spawn(lettering("ASHES REMAIN", 13.0, GOLD));
                panel.spawn(lettering("YOU HAVE FALLEN", 36.0, IVORY));
                panel.spawn(lettering(defeat_cause(&death_report), 18.0, MUTED));
                panel.spawn((
                    Node {
                        width: Val::Percent(75.0),
                        height: Val::Px(1.0),
                        margin: UiRect::vertical(Val::Px(4.0)),
                        ..default()
                    },
                    BackgroundColor(Color::srgba(0.64, 0.43, 0.22, 0.55)),
                ));
                panel
                    .spawn((Node {
                        width: Val::Percent(100.0),
                        justify_content: JustifyContent::SpaceEvenly,
                        flex_wrap: FlexWrap::Wrap,
                        column_gap: Val::Px(16.0),
                        row_gap: Val::Px(12.0),
                        ..default()
                    },))
                    .with_children(|metrics| {
                        for (label, value) in [
                            ("Time survived", format_run_time(stats.elapsed_secs)),
                            ("Enemies slain", stats.kills.to_string()),
                            ("Gold gathered", stats.gold.to_string()),
                        ] {
                            metrics
                                .spawn(Node {
                                    flex_direction: FlexDirection::Column,
                                    align_items: AlignItems::Center,
                                    row_gap: Val::Px(5.0),
                                    ..default()
                                })
                                .with_children(|metric| {
                                    metric.spawn(lettering(value, 26.0, IVORY));
                                    metric.spawn(lettering(label, 14.0, GOLD));
                                });
                        }
                    });
                panel.spawn((
                    lettering(defeat_hint(&progress), 16.0, MUTED),
                    Node {
                        margin: UiRect::vertical(Val::Px(8.0)),
                        max_width: Val::Percent(100.0),
                        ..default()
                    },
                ));
                for (action, label) in [
                    (DeathAction::Retry, "Rise again  [R]"),
                    (DeathAction::MainMenu, "Return to menu"),
                ] {
                    panel
                        .spawn((
                            Button,
                            action,
                            Node {
                                width: Val::Percent(100.0),
                                min_height: Val::Px(48.0),
                                padding: UiRect::all(Val::Px(10.0)),
                                border: UiRect::all(Val::Px(1.0)),
                                align_items: AlignItems::Center,
                                justify_content: JustifyContent::Center,
                                ..default()
                            },
                            BackgroundColor(button_color(action, false)),
                            BorderColor::all(if action == DeathAction::Retry {
                                GOLD
                            } else {
                                Color::srgba(0.50, 0.48, 0.42, 0.40)
                            }),
                        ))
                        .with_children(|button| {
                            button.spawn(lettering(label, 18.0, IVORY));
                        });
                }
            });
        });
}

fn button_color(action: DeathAction, hover: bool) -> Color {
    match (action, hover) {
        (DeathAction::Retry, true) => Color::srgb(0.32, 0.13, 0.055),
        (DeathAction::Retry, false) => Color::srgb(0.18, 0.065, 0.028),
        (_, true) => Color::srgb(0.11, 0.12, 0.13),
        (_, false) => Color::srgb(0.025, 0.031, 0.040),
    }
}

pub(super) fn update_death_buttons(
    mut buttons: Query<(&Interaction, &DeathAction, &mut BackgroundColor), Changed<Interaction>>,
) {
    for (interaction, action, mut color) in &mut buttons {
        color.0 = button_color(*action, *interaction != Interaction::None);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defeat_cause_prioritizes_exhausted_revives_and_never_exposes_raw_diagnostics() {
        let mut report = DeathWardReport {
            revives_used: 2,
            max_revives: 2,
            gold_penalty: 60,
            gold_available: 39,
            reason: "raw debug explanation".into(),
        };
        assert_eq!(
            defeat_cause(&report),
            "Your soul ward has no revives remaining."
        );
        report.revives_used = 0;
        assert_eq!(defeat_cause(&report), "Not enough gold to revive.");
        report.gold_available = 60;
        assert_eq!(defeat_cause(&report), "The reliquary claimed another soul.");
    }

    #[test]
    fn defeat_advice_changes_at_the_boss() {
        let mut progress = ChapterProgress::default();
        assert!(defeat_hint(&progress).contains("potion"));
        progress.phase = ChapterPhase::Boss;
        assert!(defeat_hint(&progress).contains("boss"));
    }
}
