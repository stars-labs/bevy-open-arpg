//! Two behavior-changing Soulbinder talent paths with visible progression gates.
use super::{BuildPage, BuildTab};
use crate::class::HeroClass;
use crate::combat::summons::SummonTalents;
use crate::ui::*;

#[derive(Component)]
pub(super) struct SummonTalentPage;
#[derive(Component, Clone, Copy)]
struct TalentNode {
    bone: bool,
    rank: u32,
}
#[derive(Component)]
struct TalentCopy(TalentNode);
#[derive(Component)]
struct TalentPoints;
#[derive(Component)]
struct ResetTalent;
#[derive(Component)]
struct StaticCopy(&'static str, &'static str);

pub(super) struct SummonTalentPlugin;
impl Plugin for SummonTalentPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            (spend, reset, update, localize_static)
                .chain()
                .run_if(in_state(GameState::InGame)),
        );
    }
}
fn copy(parent: &mut ChildSpawnerCommands, assets: &AssetServer, text: &'static str, size: f32) {
    parent.spawn((
        Text::new(text),
        StaticCopy(
            text,
            match text {
                "骨卫统御" => "Bone Dominion",
                "灵契共战" => "Soul Concord",
                "以双卫围攻，以骨墙护主" => {
                    "Twin guards surround foes and shelter their master"
                }
                "普攻引导重劈，技能引发共击" => {
                    "Attacks guide heavy strikes; skills trigger echoes"
                }
                "重置天赋 · 附近十二米无敌人时可用" => {
                    "Reset talents · Available with no living enemy within 12 m"
                }
                _ => "Sequential unlocks · Choose one keystone · Five point limit",
            },
        ),
        localization::NativeLanguageName,
        TextFont {
            font: assets.load(UI_FONT_REGULAR).into(),
            font_size: FontSize::Px(size),
            ..default()
        },
        TextColor(Color::srgb(0.88, 0.90, 0.80)),
        TextLayout::new(Justify::Left, LineBreak::WordOrCharacter),
    ));
}
pub(super) fn spawn(parent: &mut ChildSpawnerCommands, assets: &AssetServer) {
    parent
        .spawn((
            BuildPage(BuildTab::Talents),
            SummonTalentPage,
            Node {
                display: Display::None,
                width: Val::Percent(100.),
                flex_direction: FlexDirection::Column,
                row_gap: Val::Px(8.),
                ..default()
            },
        ))
        .with_children(|page| {
            page.spawn((
                Text::new(""),
                TalentPoints,
                localization::NativeLanguageName,
                TextFont {
                    font: assets.load(UI_FONT_REGULAR).into(),
                    font_size: FontSize::Px(18.),
                    ..default()
                },
                TextColor(Color::srgb(0.63, 0.87, 0.76)),
            ));
            page.spawn((
                Button,
                ResetTalent,
                Node {
                    padding: UiRect::all(Val::Px(10.)),
                    border: UiRect::all(Val::Px(1.)),
                    ..default()
                },
                BackgroundColor(Color::srgb(0.06, 0.07, 0.05)),
                BorderColor::all(Color::srgb(0.45, 0.42, 0.26)),
            ))
            .with_children(|button| copy(button, assets, "重置天赋 · 附近十二米无敌人时可用", 13.));
            copy(
                page,
                assets,
                "每条路线逐级解锁 · 深层天赋二选一 · 最多投入五点",
                13.,
            );
            page.spawn(Node {
                width: Val::Percent(100.),
                column_gap: Val::Px(16.),
                ..default()
            })
            .with_children(|row| {
                for bone in [true, false] {
                    row.spawn(Node {
                        flex_grow: 1.,
                        flex_basis: Val::Px(0.),
                        min_width: Val::Px(0.),
                        flex_direction: FlexDirection::Column,
                        row_gap: Val::Px(8.),
                        ..default()
                    })
                    .with_children(|path| {
                        copy(
                            path,
                            assets,
                            if bone { "骨卫统御" } else { "灵契共战" },
                            22.,
                        );
                        copy(
                            path,
                            assets,
                            if bone {
                                "以双卫围攻，以骨墙护主"
                            } else {
                                "普攻引导重劈，技能引发共击"
                            },
                            13.,
                        );
                        for rank in 1..=3 {
                            let node = TalentNode { bone, rank };
                            path.spawn((
                                Button,
                                node,
                                Node {
                                    min_height: Val::Px(88.),
                                    width: Val::Percent(100.),
                                    padding: UiRect::all(Val::Px(12.)),
                                    border: UiRect::all(Val::Px(1.)),
                                    column_gap: Val::Px(10.),
                                    align_items: AlignItems::Center,
                                    ..default()
                                },
                                BackgroundColor(Color::srgb(0.025, 0.06, 0.05)),
                                BorderColor::all(Color::srgb(0.25, 0.38, 0.32)),
                            ))
                            .with_children(|button| {
                                let column = if bone {
                                    [4, 1, 6][rank as usize - 1]
                                } else {
                                    [0, 3, 5][rank as usize - 1]
                                };
                                button.spawn((
                                    ImageNode {
                                        image: assets
                                            .load("images/generated/ui/soulbinder-skill-icons.png"),
                                        rect: Some(Rect::from_corners(
                                            Vec2::new(column as f32 * 256., 0.),
                                            Vec2::new((column + 1) as f32 * 256., 256.),
                                        )),
                                        ..default()
                                    },
                                    Node {
                                        width: Val::Px(54.),
                                        height: Val::Px(54.),
                                        flex_shrink: 0.,
                                        ..default()
                                    },
                                ));
                                button.spawn((
                                    Text::new(""),
                                    TalentCopy(node),
                                    localization::NativeLanguageName,
                                    TextFont {
                                        font: assets.load(UI_FONT_REGULAR).into(),
                                        font_size: FontSize::Px(14.),
                                        ..default()
                                    },
                                    TextColor(Color::srgb(0.91, 0.88, 0.77)),
                                    TextLayout::new(Justify::Left, LineBreak::WordOrCharacter),
                                    Node {
                                        flex_grow: 1.,
                                        min_width: Val::Px(0.),
                                        ..default()
                                    },
                                ));
                            });
                        }
                    });
                }
            });
        });
}
fn available(node: TalentNode, tree: &SummonTalents, points: u32, level: u32) -> bool {
    let rank = if node.bone { tree.bone } else { tree.spirit };
    let other = if node.bone { tree.spirit } else { tree.bone };
    points > 0
        && node.rank == rank + 1
        && level >= node.rank * 2
        && tree.bone + tree.spirit < 5
        && (node.rank < 3 || other < 3)
}
fn spend(
    open: Res<BuildOpen>,
    tab: Res<BuildTab>,
    selected: Res<SelectedClass>,
    language: Res<localization::UiLanguage>,
    buttons: Query<(&Interaction, &TalentNode), Changed<Interaction>>,
    mut player: Query<(&PlayerLevel, &mut Talents, &mut SummonTalents), With<Player>>,
    mut events: MessageWriter<CombatEvent>,
) {
    if !open.open || *tab != BuildTab::Talents {
        return;
    }
    let Ok((level, mut points, mut tree)) = player.single_mut() else {
        return;
    };
    if selected.0 != HeroClass::Soulbinder {
        return;
    }
    for (interaction, node) in &buttons {
        if *interaction == Interaction::Pressed
            && available(*node, &tree, points.points, level.level)
        {
            if node.bone {
                tree.bone = node.rank;
            } else {
                tree.spirit = node.rank;
            }
            points.points -= 1;
            events.write(CombatEvent {
                text: if *language == localization::UiLanguage::Chinese {
                    format!("天赋已觉醒：{} · 召唤物战斗方式已改变", name(*node))
                } else {
                    format!("Talent awakened: {}", name_en(*node))
                },
            });
        }
    }
}
fn name(node: TalentNode) -> &'static str {
    match (node.bone, node.rank) {
        (true, 1) => "双卫",
        (true, 2) => "护主",
        (true, _) => "骨墙",
        (false, 1) => "共鸣",
        (false, 2) => "共击",
        (false, _) => "灵契",
    }
}
fn detail(node: TalentNode) -> &'static str {
    match (node.bone, node.rank) {
        (true, 1) => "一名魂灵化为两名骨卫，围攻标记目标。",
        (true, 2) => "骨卫拦截逼近主人的敌人，护主反击。",
        (true, _) => "骨卫结成骨墙，以守势保护主人。",
        (false, 1) => "每三次普攻引导魂灵重劈，命中有短暂停顿。",
        (false, 2) => "灵魂齐射触发主人回响，形成协同连击。",
        (false, _) => "灵契爆发时魂灵重劈横扫周围敌人。",
    }
}
#[allow(clippy::too_many_arguments)]
fn update(
    tab: Res<BuildTab>,
    selected: Res<SelectedClass>,
    language: Res<localization::UiLanguage>,
    mut pages: Query<&mut Node, With<SummonTalentPage>>,
    player: Query<(&PlayerLevel, &Talents, &SummonTalents), With<Player>>,
    mut labels: Query<(&TalentCopy, &mut Text), Without<TalentPoints>>,
    mut points_labels: Query<&mut Text, (With<TalentPoints>, Without<TalentCopy>)>,
    mut buttons: Query<(&TalentNode, &mut BorderColor, &mut BackgroundColor)>,
) {
    for mut page in &mut pages {
        page.display = if selected.0 == HeroClass::Soulbinder && *tab == BuildTab::Talents {
            Display::Flex
        } else {
            Display::None
        };
    }
    let Ok((level, points, tree)) = player.single() else {
        return;
    };
    let zh = *language == localization::UiLanguage::Chinese;
    for mut text in &mut points_labels {
        text.0 = if zh {
            format!(
                "缚魂使天赋 · 可用 {} 点 · 已投入 {}/5",
                points.points,
                tree.bone + tree.spirit
            )
        } else {
            format!(
                "Soulbinder talents · {} points available · {}/5 spent",
                points.points,
                tree.bone + tree.spirit
            )
        };
    }
    for (copy, mut text) in &mut labels {
        let node = copy.0;
        let rank = if node.bone { tree.bone } else { tree.spirit };
        let state = if rank >= node.rank {
            "已觉醒"
        } else if available(node, tree, points.points, level.level) {
            "点击觉醒 · 消耗一点"
        } else if node.rank == 3 && (if node.bone { tree.spirit } else { tree.bone }) >= 3 {
            "与另一条深层天赋互斥"
        } else if tree.bone + tree.spirit >= 5 {
            "已达到五点上限"
        } else if level.level < node.rank * 2 {
            "等级尚未满足"
        } else if rank + 1 < node.rank {
            "先觉醒上一节点"
        } else {
            "升级获得天赋点"
        };
        text.0 = if zh {
            format!(
                "{} · 第{}阶\n{}\n{}级解锁 · {}",
                name(node),
                node.rank,
                detail(node),
                node.rank * 2,
                state
            )
        } else {
            let state = match state {
                "已觉醒" => "Awakened",
                "点击觉醒 · 消耗一点" => "Awaken · One point",
                "与另一条深层天赋互斥" => "Other keystone chosen",
                "已达到五点上限" => "Five point limit reached",
                "等级尚未满足" => "Level requirement unmet",
                "先觉醒上一节点" => "Awaken preceding node first",
                _ => "Level up to earn points",
            };
            format!(
                "{} · Tier {}\n{}\nLevel {} · {}",
                name_en(node),
                node.rank,
                detail_en(node),
                node.rank * 2,
                state
            )
        };
    }
    for (node, mut border, mut bg) in &mut buttons {
        let rank = if node.bone { tree.bone } else { tree.spirit };
        let active = rank >= node.rank;
        let ready = available(*node, tree, points.points, level.level);
        *border = BorderColor::all(if active {
            Color::srgb(0.55, 0.83, 0.68)
        } else if ready {
            Color::srgb(0.66, 0.51, 0.27)
        } else {
            Color::srgb(0.20, 0.24, 0.22)
        });
        bg.0 = if active {
            Color::srgb(0.04, 0.13, 0.10)
        } else {
            Color::srgb(0.025, 0.035, 0.03)
        };
    }
}

fn name_en(node: TalentNode) -> &'static str {
    match (node.bone, node.rank) {
        (true, 1) => "Twin Guard",
        (true, 2) => "Interception",
        (true, _) => "Bone Wall",
        (false, 1) => "Resonance",
        (false, 2) => "Echo Strike",
        (false, _) => "Soul Concord",
    }
}
fn detail_en(node: TalentNode) -> &'static str {
    match (node.bone, node.rank) {
        (true, 1) => "Replace one spirit with two bone guards surrounding the marked target.",
        (true, 2) => "Guards intercept enemies approaching their master.",
        (true, _) => "Guards form a defensive bone wall.",
        (false, 1) => "Every third basic attack guides a heavy spirit strike with contact hold.",
        (false, 2) => "Soul Volley triggers an additional owner echo.",
        (false, _) => "Overdrive heavy strikes cleave nearby enemies.",
    }
}
fn localize_static(
    language: Res<localization::UiLanguage>,
    mut texts: Query<(&StaticCopy, &mut Text)>,
) {
    for (copy, mut text) in &mut texts {
        text.0 = if *language == localization::UiLanguage::Chinese {
            copy.0
        } else {
            copy.1
        }
        .into();
    }
}
#[allow(clippy::too_many_arguments)]
#[allow(clippy::type_complexity)]
fn reset(
    open: Res<BuildOpen>,
    tab: Res<BuildTab>,
    selected: Res<SelectedClass>,
    language: Res<localization::UiLanguage>,
    buttons: Query<&Interaction, (With<ResetTalent>, Changed<Interaction>)>,
    mut player: Query<(&Transform, &mut Talents, &mut SummonTalents), With<Player>>,
    enemies: Query<(&Transform, &Health), (With<Enemy>, Without<Player>)>,
    mut events: MessageWriter<CombatEvent>,
) {
    if !open.open
        || *tab != BuildTab::Talents
        || selected.0 != HeroClass::Soulbinder
        || !buttons.iter().any(|i| *i == Interaction::Pressed)
    {
        return;
    }
    let Ok((pose, mut points, mut tree)) = player.single_mut() else {
        return;
    };
    let combat = enemies.iter().any(|(enemy, health)| {
        health.current > 0. && enemy.translation.distance_squared(pose.translation) < 144.
    });
    let zh = *language == localization::UiLanguage::Chinese;
    if combat {
        events.write(CombatEvent {
            text: if zh {
                "附近仍有敌人，无法重置天赋"
            } else {
                "Cannot reset while enemies are nearby"
            }
            .into(),
        });
        return;
    }
    points.points += tree.bone + tree.spirit;
    *tree = SummonTalents::default();
    events.write(CombatEvent {
        text: if zh {
            "天赋已重置，投入点数全部返还"
        } else {
            "Talents reset; all invested points refunded"
        }
        .into(),
    });
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn talent_gates_require_levels_and_predecessors() {
        let tree = SummonTalents::default();
        let first = TalentNode {
            bone: true,
            rank: 1,
        };
        assert!(!available(first, &tree, 1, 1));
        assert!(available(first, &tree, 1, 2));
        assert!(!available(
            TalentNode {
                bone: true,
                rank: 2
            },
            &tree,
            2,
            6
        ));
        assert!(!available(first, &tree, 0, 6));
    }
    #[test]
    fn talent_keystones_are_mutually_exclusive_and_capped() {
        let tree = SummonTalents { bone: 2, spirit: 3 };
        assert!(!available(
            TalentNode {
                bone: true,
                rank: 3
            },
            &tree,
            1,
            6
        ));
        let tree = SummonTalents { bone: 2, spirit: 2 };
        assert!(available(
            TalentNode {
                bone: true,
                rank: 3
            },
            &tree,
            1,
            6
        ));
        assert!(!available(
            TalentNode {
                bone: true,
                rank: 3
            },
            &tree,
            1,
            5
        ));
    }
}
