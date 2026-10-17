//! Two behavior-changing Soulbinder talent paths with visible progression gates.
use super::{BuildPage, BuildTab};
use crate::class::HeroClass;
use crate::combat::summon_progression::{LEVEL_CAP, MAX_POINTS, TIER_LEVELS, can_unlock};
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
                _ => "One point every two levels · Level cap 20 · Ten points total · Final tiers are exclusive",
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
                flex_shrink: 0.,
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
                "每两级获得一点 · 等级上限20 · 共十点 · 两线第六阶互斥",
                13.,
            );
            page.spawn(Node {
                width: Val::Percent(100.),
                column_gap: Val::Px(16.),
                flex_shrink: 0.,
                align_items: AlignItems::Start,
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
                        for rank in 1..=6 {
                            if rank > 1 {
                                path.spawn((
                                    Node {
                                        width: Val::Px(3.),
                                        height: Val::Px(20.),
                                        flex_shrink: 0.,
                                        align_self: AlignSelf::Center,
                                        ..default()
                                    },
                                    BackgroundColor(Color::srgb(0.34, 0.48, 0.39)),
                                ));
                            }
                            let node = TalentNode { bone, rank };
                            path.spawn((
                                Button,
                                node,
                                Node {
                                    min_height: Val::Px(130.),
                                    flex_shrink: 0.,
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
                                    [4, 1, 6, 1, 4, 6][rank as usize - 1]
                                } else {
                                    [0, 3, 5, 0, 3, 5][rank as usize - 1]
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
    can_unlock(tree.bone, tree.spirit, node.bone, node.rank, points, level)
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
        (true, 3) => "骨墙",
        (true, 4) => "退守",
        (true, 5) => "围猎",
        (true, _) => "不屈",
        (false, 1) => "共鸣",
        (false, 2) => "共击",
        (false, 3) => "灵契",
        (false, 4) => "残响",
        (false, 5) => "汲魂",
        (false, _) => "传契",
    }
}
fn detail(node: TalentNode) -> &'static str {
    match (node.bone, node.rank) {
        (true, 1) => "一名魂灵化为两名骨卫，围攻标记目标。",
        (true, 2) => "骨卫拦截逼近主人的敌人，护主反击。",
        (true, 3) => "骨卫结成骨墙，以守势保护主人。",
        (true, 4) => "残血骨卫撤回主人身边治疗，休整期间不攻击。",
        (true, 5) => "骨卫从两侧夹击；齐击接触使目标陷入破绽。",
        (true, _) => "自然阵亡后1.2秒重聚并回复50%生命，每30秒一次；献祭仍需6秒。",
        (false, 1) => "每三次普攻引导魂灵重劈，命中有短暂停顿。",
        (false, 2) => "灵魂齐射触发主人回响，形成协同连击。",
        (false, 3) => "灵契爆发时魂灵重劈横扫周围敌人。",
        (false, 4) => "重劈命中0.22秒后追加35%追击；目标变更、死亡或过期则取消。",
        (false, 5) => "重劈命中回复8魂能，队伍每0.8秒最多触发一次。",
        (false, _) => "狂攻期间标记目标死亡，灵契传给4米内下一名敌人。",
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
                "缚魂使天赋 · 可用 {} 点 · 已投入 {}/{} · 等级上限 {}",
                points.points,
                tree.bone + tree.spirit,
                MAX_POINTS,
                LEVEL_CAP
            )
        } else {
            format!(
                "Soulbinder talents · {} points available · {}/{} spent · Level cap {}",
                points.points,
                tree.bone + tree.spirit,
                MAX_POINTS,
                LEVEL_CAP
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
        } else if node.rank == 6 && (if node.bone { tree.spirit } else { tree.bone }) >= 6 {
            "与另一条第六阶天赋互斥"
        } else if tree.bone + tree.spirit >= MAX_POINTS {
            "已达到十点上限"
        } else if level.level < TIER_LEVELS[node.rank as usize - 1] {
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
                TIER_LEVELS[node.rank as usize - 1],
                state
            )
        } else {
            let state = match state {
                "已觉醒" => "Awakened",
                "点击觉醒 · 消耗一点" => "Awaken · One point",
                "与另一条第六阶天赋互斥" => "Other keystone chosen",
                "已达到十点上限" => "Ten point limit reached",
                "等级尚未满足" => "Level requirement unmet",
                "先觉醒上一节点" => "Awaken preceding node first",
                _ => "Level up to earn points",
            };
            format!(
                "{} · Tier {}\n{}\nLevel {} · {}",
                name_en(node),
                node.rank,
                detail_en(node),
                TIER_LEVELS[node.rank as usize - 1],
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
        (true, 3) => "Bone Wall",
        (true, 4) => "Fallback",
        (true, 5) => "Encirclement",
        (true, _) => "Defiance",
        (false, 1) => "Resonance",
        (false, 2) => "Echo Strike",
        (false, 3) => "Soul Concord",
        (false, 4) => "Aftershock",
        (false, 5) => "Soul Siphon",
        (false, _) => "Passing Pact",
    }
}
fn detail_en(node: TalentNode) -> &'static str {
    match (node.bone, node.rank) {
        (true, 1) => "Replace one spirit with two bone guards surrounding the marked target.",
        (true, 2) => "Guards intercept enemies approaching their master.",
        (true, 3) => "Guards form a defensive bone wall.",
        (true, 4) => "Wounded guards retreat to their master and heal without attacking.",
        (true, 5) => "Guards flank their target; coordinated contact applies Expose.",
        (true, _) => {
            "Natural deaths reform after 1.2s at 50% health, once per 30s. Sacrifice still takes 6s."
        }
        (false, 1) => "Every third basic attack guides a heavy spirit strike with contact hold.",
        (false, 2) => "Soul Volley triggers an additional owner echo.",
        (false, 3) => "Overdrive heavy strikes cleave nearby enemies.",
        (false, 4) => {
            "Heavy hits echo for 35% after 0.22s; target changes, death or expiry cancel it."
        }
        (false, 5) => "Heavy hits restore 8 soul energy, at most once per 0.8s for the team.",
        (false, _) => {
            "During Overdrive, a marked target dying passes the pact to another foe within 4m."
        }
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
    fn interaction_app(level: u32) -> (App, Entity, Entity) {
        let mut app = App::new();
        app.insert_resource(BuildOpen { open: true })
            .insert_resource(BuildTab::Talents)
            .insert_resource(SelectedClass(HeroClass::Soulbinder))
            .insert_resource(localization::UiLanguage::Chinese)
            .add_message::<CombatEvent>()
            .add_systems(Update, (spend, reset).chain());
        let player = app
            .world_mut()
            .spawn((
                Player,
                PlayerLevel {
                    level,
                    xp: 0,
                    next_level_xp: 100,
                },
                Talents {
                    points: MAX_POINTS,
                    ..default()
                },
                SummonTalents::default(),
                Transform::default(),
            ))
            .id();
        let button = app
            .world_mut()
            .spawn((
                TalentNode {
                    bone: true,
                    rank: 1,
                },
                Interaction::None,
            ))
            .id();
        (app, player, button)
    }

    fn press_talent(app: &mut App, button: Entity, bone: bool, rank: u32) {
        app.world_mut()
            .entity_mut(button)
            .insert(TalentNode { bone, rank });
        {
            let mut interaction = app.world_mut().get_mut::<Interaction>(button).unwrap();
            *interaction = Interaction::Pressed;
            interaction.set_changed();
        }
        app.update();
    }

    #[test]
    fn ecs_talent_presses_spend_ten_points_and_reset_refunds_all() {
        let (mut app, player, button) = interaction_app(LEVEL_CAP);
        for rank in 1..=6 {
            press_talent(&mut app, button, true, rank);
            assert_eq!(app.world().get::<SummonTalents>(player).unwrap().bone, rank);
            assert_eq!(
                app.world().get::<Talents>(player).unwrap().points,
                MAX_POINTS - rank
            );
        }
        for rank in 1..=4 {
            press_talent(&mut app, button, false, rank);
            assert_eq!(
                app.world().get::<SummonTalents>(player).unwrap().spirit,
                rank
            );
            assert_eq!(app.world().get::<Talents>(player).unwrap().points, 4 - rank);
        }
        // A further point and the other capstone must not be purchased.
        press_talent(&mut app, button, false, 5);
        press_talent(&mut app, button, false, 6);
        let tree = app.world().get::<SummonTalents>(player).unwrap();
        assert_eq!((tree.bone, tree.spirit), (6, 4));
        assert_eq!(app.world().get::<Talents>(player).unwrap().points, 0);

        app.world_mut()
            .entity_mut(button)
            .remove::<TalentNode>()
            .insert(ResetTalent);
        app.world_mut()
            .get_mut::<Interaction>(button)
            .unwrap()
            .set_changed();
        app.update();
        let tree = app.world().get::<SummonTalents>(player).unwrap();
        assert_eq!((tree.bone, tree.spirit), (0, 0));
        assert_eq!(
            app.world().get::<Talents>(player).unwrap().points,
            MAX_POINTS
        );
    }

    #[test]
    fn ecs_talent_press_respects_level_gate_without_spending() {
        let (mut app, player, button) = interaction_app(1);
        press_talent(&mut app, button, true, 1);
        assert_eq!(app.world().get::<SummonTalents>(player).unwrap().bone, 0);
        assert_eq!(
            app.world().get::<Talents>(player).unwrap().points,
            MAX_POINTS
        );
        app.world_mut()
            .get_mut::<PlayerLevel>(player)
            .unwrap()
            .level = 2;
        press_talent(&mut app, button, true, 1);
        assert_eq!(app.world().get::<SummonTalents>(player).unwrap().bone, 1);
        assert_eq!(
            app.world().get::<Talents>(player).unwrap().points,
            MAX_POINTS - 1
        );
    }

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
    fn talent_copy_covers_both_six_tier_paths() {
        for bone in [true, false] {
            let mut names = std::collections::HashSet::new();
            let mut english_names = std::collections::HashSet::new();
            for rank in 1..=6 {
                let node = TalentNode { bone, rank };
                assert!(names.insert(name(node)));
                assert!(english_names.insert(name_en(node)));
                assert!(!detail(node).is_empty());
                assert!(!detail_en(node).is_empty());
            }
        }
        assert_eq!(TIER_LEVELS, [2, 4, 6, 10, 14, 18]);
    }
}
