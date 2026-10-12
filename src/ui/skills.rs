//! Skill book: pick one of two skills per slot and a rune for each, backed by the same
//! `SkillLoadout` component that combat and saves use.
use super::localization::{NativeLanguageName, UiLanguage};
use super::*;
#[path = "summon_talents.rs"]
mod summon_talents;
use crate::class::{ClassCatalog, ClassDef, Rune, SelectedClass, SkillLoadout, SkillSlot};

#[derive(Resource, Default, Clone, Copy, PartialEq, Eq)]
pub(super) enum BuildTab {
    #[default]
    Skills,
    Talents,
}
#[derive(Component)]
pub(super) struct BuildPage(pub BuildTab);
#[derive(Component)]
struct TabButton(BuildTab);
/// One of the two skills a slot can hold.
#[derive(Component, Clone, Copy, Debug, PartialEq, Eq)]
struct SkillChoice {
    slot: SkillSlot,
    alt: bool,
}
/// One of the two runes of the skill currently equipped in a slot.
#[derive(Component, Clone, Copy, Debug, PartialEq, Eq)]
struct RuneChoice {
    slot: SkillSlot,
    alt: bool,
}
#[derive(Component, Clone, Copy)]
struct SkillChoiceIcon(SkillChoice);
#[derive(Component)]
enum SkillText {
    Tab(BuildTab),
    Intro,
    SlotHeader(SkillSlot),
    Skill(SkillChoice),
    Rune(RuneChoice),
    Basics,
    TalentSummary(TalentBranch),
    TalentRank(TalentBranch, u32),
}

pub(super) struct SkillPanelPlugin;
impl Plugin for SkillPanelPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(summon_talents::SummonTalentPlugin);
        app.init_resource::<BuildTab>().add_systems(
            Update,
            (
                choose_tab,
                choose_skill,
                choose_rune,
                choose_talent,
                update_page,
            )
                .chain()
                .run_if(in_state(GameState::InGame)),
        );
    }
}
fn label(parent: &mut ChildSpawnerCommands, assets: &AssetServer, marker: SkillText, size: f32) {
    parent.spawn((
        Text::new(""),
        marker,
        NativeLanguageName,
        TextFont {
            font: assets.load(UI_FONT_REGULAR).into(),
            font_size: FontSize::Px(size),
            ..default()
        },
        TextColor(Color::srgb(0.92, 0.85, 0.72)),
        TextLayout::new(Justify::Left, LineBreak::WordOrCharacter),
    ));
}
pub(super) fn spawn_talent_description(
    parent: &mut ChildSpawnerCommands,
    assets: &AssetServer,
    branch: TalentBranch,
) {
    parent
        .spawn(Node {
            width: Val::Percent(100.0),
            min_height: Val::Px(92.0),
            flex_shrink: 0.0,
            ..default()
        })
        .with_children(|body| label(body, assets, SkillText::TalentSummary(branch), 12.0));
}
pub(super) fn spawn_talent_rank(
    parent: &mut ChildSpawnerCommands,
    assets: &AssetServer,
    branch: TalentBranch,
    rank: u32,
) {
    label(parent, assets, SkillText::TalentRank(branch, rank), 11.0);
}

pub(super) fn spawn_tabs(parent: &mut ChildSpawnerCommands, assets: &AssetServer) {
    parent
        .spawn(Node {
            column_gap: Val::Px(10.0),
            flex_shrink: 0.0,
            ..default()
        })
        .with_children(|row| {
            for tab in [BuildTab::Skills, BuildTab::Talents] {
                row.spawn((
                    Button,
                    TabButton(tab),
                    Node {
                        padding: UiRect::axes(Val::Px(20.0), Val::Px(9.0)),
                        border: UiRect::all(Val::Px(1.0)),
                        ..default()
                    },
                    BackgroundColor(Color::srgb(0.06, 0.04, 0.02)),
                    BorderColor::all(Color::srgb(0.6, 0.45, 0.2)),
                ))
                .with_children(|button| label(button, assets, SkillText::Tab(tab), 17.0));
            }
        });
}
fn choice_button(parent: &mut ChildSpawnerCommands, basis: f32) -> Node {
    let _ = parent;
    Node {
        flex_basis: Val::Px(basis),
        flex_grow: 1.0,
        min_width: Val::Px(0.0),
        padding: UiRect::all(Val::Px(10.0)),
        border: UiRect::all(Val::Px(1.0)),
        column_gap: Val::Px(10.0),
        align_items: AlignItems::Center,
        ..default()
    }
}
pub(super) fn spawn_page(parent: &mut ChildSpawnerCommands, assets: &AssetServer) {
    summon_talents::spawn(parent, assets);
    parent
        .spawn((
            BuildPage(BuildTab::Skills),
            Node {
                width: Val::Percent(100.0),
                flex_direction: FlexDirection::Column,
                row_gap: Val::Px(14.0),
                flex_shrink: 0.0,
                ..default()
            },
        ))
        .with_children(|page| {
            label(page, assets, SkillText::Intro, 14.0);
            for slot in [SkillSlot::Finisher, SkillSlot::Mobility, SkillSlot::Area] {
                page.spawn((
                    Node {
                        width: Val::Percent(100.0),
                        padding: UiRect::all(Val::Px(12.0)),
                        flex_direction: FlexDirection::Column,
                        row_gap: Val::Px(10.0),
                        border: UiRect::all(Val::Px(1.0)),
                        flex_shrink: 0.0,
                        ..default()
                    },
                    BackgroundColor(Color::srgb(0.035, 0.028, 0.038)),
                    BorderColor::all(Color::srgb(0.26, 0.22, 0.17)),
                ))
                .with_children(|card| {
                    label(card, assets, SkillText::SlotHeader(slot), 16.0);
                    card.spawn(Node {
                        flex_wrap: FlexWrap::Wrap,
                        column_gap: Val::Px(10.0),
                        row_gap: Val::Px(8.0),
                        ..default()
                    })
                    .with_children(|choices| {
                        for alt in [false, true] {
                            let choice = SkillChoice { slot, alt };
                            let node = choice_button(choices, 280.0);
                            choices
                                .spawn((
                                    Button,
                                    choice,
                                    node,
                                    BackgroundColor(Color::srgb(0.02, 0.02, 0.03)),
                                    BorderColor::all(Color::srgb(0.25, 0.23, 0.22)),
                                ))
                                .with_children(|button| {
                                    button.spawn((
                                        atlas_image_node(
                                            assets,
                                            UI_SKILL_ICONS_IMAGE,
                                            atlas_rect(
                                                UI_SKILL_ICON_CELL,
                                                UI_SKILL_ICON_CELL,
                                                3,
                                                0,
                                            ),
                                        ),
                                        Node {
                                            width: Val::Px(48.0),
                                            height: Val::Px(48.0),
                                            flex_shrink: 0.0,
                                            ..default()
                                        },
                                        SkillChoiceIcon(choice),
                                        Pickable::IGNORE,
                                    ));
                                    label(button, assets, SkillText::Skill(choice), 13.0);
                                });
                        }
                    });
                    card.spawn(Node {
                        flex_wrap: FlexWrap::Wrap,
                        column_gap: Val::Px(10.0),
                        row_gap: Val::Px(8.0),
                        ..default()
                    })
                    .with_children(|runes| {
                        for alt in [false, true] {
                            let choice = RuneChoice { slot, alt };
                            let node = choice_button(runes, 280.0);
                            runes
                                .spawn((
                                    Button,
                                    choice,
                                    node,
                                    BackgroundColor(Color::srgb(0.02, 0.02, 0.03)),
                                    BorderColor::all(Color::srgb(0.25, 0.23, 0.22)),
                                ))
                                .with_children(|button| {
                                    label(button, assets, SkillText::Rune(choice), 13.0)
                                });
                        }
                    });
                });
            }
            label(page, assets, SkillText::Basics, 14.0);
        });
}
fn choose_tab(
    open: Res<BuildOpen>,
    mut tab: ResMut<BuildTab>,
    buttons: Query<(&Interaction, &TabButton), Changed<Interaction>>,
    mut scroll: Query<&mut ScrollPosition, With<BuildPanel>>,
) {
    if !open.open {
        return;
    }
    for (interaction, button) in &buttons {
        if *interaction == Interaction::Pressed {
            *tab = button.0;
            for mut position in &mut scroll {
                *position = ScrollPosition::default();
            }
        }
    }
}
/// Equips an unlocked skill; locked skills stay visible but inert.
fn select_skill(
    loadout: &mut SkillLoadout,
    class: &ClassDef,
    level: u32,
    choice: SkillChoice,
) -> bool {
    if class.skill(choice.slot, choice.alt).unlock_level > level {
        return false;
    }
    loadout.alt[choice.slot.index()] = choice.alt;
    true
}
fn skill_selected(loadout: &SkillLoadout, choice: SkillChoice) -> bool {
    loadout.alt[choice.slot.index()] == choice.alt
}
fn select_rune(loadout: &mut SkillLoadout, choice: RuneChoice) {
    let alt = usize::from(loadout.alt[choice.slot.index()]);
    loadout.rune_alt[choice.slot.index()][alt] = choice.alt;
}
fn rune_selected(loadout: &SkillLoadout, choice: RuneChoice) -> bool {
    let alt = usize::from(loadout.alt[choice.slot.index()]);
    loadout.rune_alt[choice.slot.index()][alt] == choice.alt
}
fn choose_skill(
    open: Res<BuildOpen>,
    tab: Res<BuildTab>,
    catalog: Res<ClassCatalog>,
    selected: Res<SelectedClass>,
    buttons: Query<(&Interaction, &SkillChoice), Changed<Interaction>>,
    mut player: Query<(&mut SkillLoadout, &PlayerLevel), With<Player>>,
    mut events: MessageWriter<CombatEvent>,
) {
    if !open.open || *tab != BuildTab::Skills {
        return;
    }
    let Ok((mut loadout, level)) = player.single_mut() else {
        return;
    };
    let class = catalog.get(selected.0);
    for (interaction, choice) in &buttons {
        if *interaction != Interaction::Pressed {
            continue;
        }
        let skill = class.skill(choice.slot, choice.alt);
        if select_skill(&mut loadout, class, level.level, *choice) {
            events.write(CombatEvent {
                text: format!("{} equipped on {}", skill.name, choice.slot.key_label()),
            });
        } else {
            events.write(CombatEvent {
                text: format!("{} unlocks at level {}", skill.name, skill.unlock_level),
            });
        }
    }
}
fn choose_rune(
    open: Res<BuildOpen>,
    tab: Res<BuildTab>,
    buttons: Query<(&Interaction, &RuneChoice), Changed<Interaction>>,
    mut player: Query<&mut SkillLoadout, With<Player>>,
) {
    if !open.open || *tab != BuildTab::Skills {
        return;
    }
    let Ok(mut loadout) = player.single_mut() else {
        return;
    };
    for (interaction, choice) in &buttons {
        if *interaction == Interaction::Pressed {
            select_rune(&mut loadout, *choice);
        }
    }
}
fn choose_talent(
    open: Res<BuildOpen>,
    tab: Res<BuildTab>,
    selected: Res<SelectedClass>,
    buttons: Query<(&Interaction, &TalentRankNode), Changed<Interaction>>,
    mut player: Query<(&mut Talents, &mut Health), With<Player>>,
    mut events: MessageWriter<CombatEvent>,
) {
    if !open.open || *tab != BuildTab::Talents || selected.0 == crate::class::HeroClass::Soulbinder
    {
        return;
    }
    let Ok((mut talents, mut health)) = player.single_mut() else {
        return;
    };
    for (interaction, node) in &buttons {
        if *interaction != Interaction::Pressed
            || node.rank != node.branch.rank(&talents).saturating_add(1)
        {
            continue;
        }
        for text in crate::player::spend_talent_rank(&mut talents, &mut health, node.branch) {
            events.write(CombatEvent { text });
        }
    }
}

type SkillChoiceStyleQuery<'w, 's> = Query<
    'w,
    's,
    (
        &'static SkillChoice,
        &'static mut BorderColor,
        &'static mut BackgroundColor,
    ),
    (Without<TabButton>, Without<RuneChoice>),
>;

type RuneChoiceStyleQuery<'w, 's> = Query<
    'w,
    's,
    (
        &'static RuneChoice,
        &'static mut BorderColor,
        &'static mut BackgroundColor,
    ),
    (Without<TabButton>, Without<SkillChoice>),
>;

type TabStyleQuery<'w, 's> = Query<
    'w,
    's,
    (
        &'static TabButton,
        &'static mut BorderColor,
        &'static mut BackgroundColor,
    ),
    (Without<RuneChoice>, Without<SkillChoice>),
>;

#[derive(SystemParam)]
struct PageQueries<'w, 's> {
    pages: Query<
        'w,
        's,
        (&'static BuildPage, &'static mut Node),
        Without<summon_talents::SummonTalentPage>,
    >,
    texts: Query<'w, 's, (&'static SkillText, &'static mut Text)>,
    skills: SkillChoiceStyleQuery<'w, 's>,
    runes: RuneChoiceStyleQuery<'w, 's>,
    tabs: TabStyleQuery<'w, 's>,
    icons: Query<'w, 's, (&'static SkillChoiceIcon, &'static mut ImageNode)>,
}

#[allow(clippy::too_many_arguments)]
fn update_page(
    tab: Res<BuildTab>,
    language: Res<UiLanguage>,
    tuning: Res<crate::data::PlayerTuning>,
    catalog: Res<ClassCatalog>,
    selected: Res<SelectedClass>,
    game_assets: Option<Res<GameAssets>>,
    player: Query<(&SkillLoadout, &Fury, &PlayerLevel, &Talents), With<Player>>,
    mut queries: PageQueries,
) {
    for (page, mut node) in &mut queries.pages {
        node.display = if page.0 == *tab
            && !(page.0 == BuildTab::Talents && selected.0 == crate::class::HeroClass::Soulbinder)
        {
            Display::Flex
        } else {
            Display::None
        };
    }
    for (button, mut border, mut bg) in &mut queries.tabs {
        style(button.0 == *tab, &mut border, &mut bg);
    }
    let Ok((loadout, fury, level, talents)) = player.single() else {
        return;
    };
    let class = catalog.get(selected.0);
    for (choice, mut border, mut bg) in &mut queries.skills {
        let unlocked = class.skill(choice.slot, choice.alt).unlock_level <= level.level;
        style(skill_selected(loadout, *choice), &mut border, &mut bg);
        if !unlocked {
            bg.0 = Color::srgb(0.03, 0.03, 0.035);
            *border = BorderColor::all(Color::srgb(0.18, 0.17, 0.17));
        }
    }
    for (choice, mut border, mut bg) in &mut queries.runes {
        style(rune_selected(loadout, *choice), &mut border, &mut bg);
    }
    if let Some(game_assets) = game_assets.as_deref() {
        for (icon, mut image) in &mut queries.icons {
            let soulbinder = class.class == crate::class::HeroClass::Soulbinder;
            let atlas = if soulbinder {
                &game_assets.ui_soulbinder_skill_icons
            } else {
                &game_assets.ui_class_skill_icons
            };
            let atlas_class = if soulbinder {
                crate::class::HeroClass::Warden
            } else {
                class.class
            };
            let rect = atlas_rect(
                UI_SKILL_ICON_CELL,
                UI_SKILL_ICON_CELL,
                CLASS_SKILL_ATLAS_COLUMNS,
                class_skill_icon_index(atlas_class, Some(icon.0.slot), icon.0.alt),
            );
            if image.rect != Some(rect) || image.image != *atlas {
                image.image = atlas.clone();
                image.rect = Some(rect);
            }
        }
    }
    let zh = *language == UiLanguage::Chinese;
    for (marker, mut text) in &mut queries.texts {
        let value = match marker {
            SkillText::Tab(BuildTab::Skills) => bilingual(zh, "技能", "Skills").into(),
            SkillText::Tab(BuildTab::Talents) => bilingual(zh, "天赋", "Talents").into(),
            SkillText::Intro => intro_text(class, level.level, zh),
            SkillText::SlotHeader(slot) => slot_header(*slot, zh),
            SkillText::Skill(choice) => {
                skill_card_text(class, loadout, fury, talents, level.level, *choice, zh)
            }
            SkillText::Rune(choice) => {
                let (skill, _) = class.equipped(loadout, choice.slot);
                let rune = skill.rune(choice.alt);
                let prefix = if rune_selected(loadout, *choice) {
                    bilingual(zh, "已装备 · ", "Equipped · ")
                } else {
                    bilingual(zh, "选择 · ", "Select · ")
                };
                let name = if zh { rune.label_zh() } else { rune.label() };
                format!("{prefix}{name}\n{}", rune_description(rune, zh))
            }
            SkillText::Basics => basics_text(class, talents, &tuning.growth, zh),
            SkillText::TalentSummary(branch) => {
                talent_description(*branch, talents, &tuning.growth, zh)
            }
            SkillText::TalentRank(branch, rank) => {
                talent_rank_copy(*branch, *rank, &tuning.growth, zh)
            }
        };
        if text.0 != value {
            text.0 = value;
        }
    }
}

fn intro_text(class: &ClassDef, level: u32, zh: bool) -> String {
    let name = if zh { &class.name_zh } else { &class.name };
    let passive = if zh {
        format!("{}：{}", class.passive_name_zh, class.passive_summary_zh)
    } else {
        format!("{}: {}", class.passive_name, class.passive_summary)
    };
    let next = class.unlocked_at(level + 1).next().map(|(_, skill)| {
        if zh {
            skill.name_zh.clone()
        } else {
            skill.name.clone()
        }
    });
    let unlock = match next {
        Some(skill) if zh => format!("{} 级解锁：{skill}", level + 1),
        Some(skill) => format!("Level {}: {skill}", level + 1),
        None => bilingual(zh, "全部技能已习得", "Every skill is learned").to_string(),
    };
    if zh {
        format!(
            "{name} · {passive}\n每个槽位二选一，每个技能两个符文，点击切换，不花费天赋点。{unlock}。"
        )
    } else {
        format!(
            "{name} · {passive}\nTwo skills per slot and two runes per skill; click to switch, no talent points needed. {unlock}."
        )
    }
}

fn slot_header(slot: SkillSlot, zh: bool) -> String {
    let (zh_name, en_name) = match slot {
        SkillSlot::Finisher => ("终结", "Finisher"),
        SkillSlot::Mobility => ("机动", "Mobility"),
        SkillSlot::Area => ("范围", "Area"),
    };
    format!("[{}] {}", slot.key_label(), bilingual(zh, zh_name, en_name))
}

fn skill_card_text(
    class: &ClassDef,
    loadout: &SkillLoadout,
    fury: &Fury,
    talents: &Talents,
    level: u32,
    choice: SkillChoice,
    zh: bool,
) -> String {
    let skill = class.skill(choice.slot, choice.alt);
    let name = if zh { &skill.name_zh } else { &skill.name };
    let summary = if zh {
        &skill.summary_zh
    } else {
        &skill.summary
    };
    let cost = fury.skill_cost(skill.cost) * talents.fury_cost_multiplier();
    let cooldown = skill.cooldown * talents.cooldown_multiplier();
    let resource = class.resource.kind;
    let state = if skill.unlock_level > level {
        if zh {
            format!("{} 级解锁", skill.unlock_level)
        } else {
            format!("Unlocks at level {}", skill.unlock_level)
        }
    } else if skill_selected(loadout, choice) {
        bilingual(zh, "已装备", "Equipped").to_string()
    } else {
        bilingual(zh, "选择", "Select").to_string()
    };
    if zh {
        format!(
            "{state} · {name}\n消耗 {cost:.0} {} · 冷却 {cooldown:.1} 秒\n{summary}",
            resource_name_zh(resource)
        )
    } else {
        format!(
            "{state} · {name}\n{cost:.0} {} · {cooldown:.1}s cooldown\n{summary}",
            resource.label()
        )
    }
}

pub(super) fn resource_name_zh(kind: crate::class::ResourceKind) -> &'static str {
    match kind {
        crate::class::ResourceKind::Fury => "怒气",
        crate::class::ResourceKind::Malice => "杀意",
        crate::class::ResourceKind::Arcana => "魔能",
        crate::class::ResourceKind::Soul => "魂能",
    }
}

fn basics_text(
    class: &ClassDef,
    talents: &Talents,
    growth: &crate::data::CombatGrowthTuning,
    zh: bool,
) -> String {
    let resource = class.resource.kind;
    let basic = if zh {
        &class.basic.name_zh
    } else {
        &class.basic.name
    };
    let gain = if class.resource.basic_gain > 0.0 {
        if zh {
            format!("命中积攒{}，为技能提供能量。", resource_name_zh(resource))
        } else {
            format!("Hits build {} to fuel skills.", resource.label())
        }
    } else if zh {
        format!("{}随时间自行回复。", resource_name_zh(resource))
    } else {
        format!("{} regenerates over time.", resource.label())
    };
    let mut text = if zh {
        format!(
            "左键 · {basic}\n{gain}\n\nShift · 闪避\n躲避来袭攻击。\n\nY · 奔涌\n击杀蓄能，充满后激活，暂时提高伤害、移速与资源获取。\n\n右键移动，鼠标指向决定攻击方向。"
        )
    } else {
        format!(
            "LMB · {basic}\n{gain}\n\nShift · Evade\nDodge incoming attacks.\n\nY · Surge\nKills charge Surge. Activate when full for increased damage, speed and resource gain.\n\nMove with RMB; aim attacks with the cursor."
        )
    };
    let bonus = talents.basic_fury_bonus(growth);
    if bonus > 0.0 {
        text.push_str(&if zh {
            format!(
                "\n\n专注：每次{basic}命中额外获得 {bonus:.0} {}。",
                resource_name_zh(resource)
            )
        } else {
            format!(
                "\n\nFocus: each {} that connects grants {bonus:.0} extra {}.",
                basic.to_lowercase(),
                resource.label()
            )
        });
    }
    text
}

fn talent_at_rank(branch: TalentBranch, rank: u32) -> Talents {
    let mut talents = Talents::default();
    match branch {
        TalentBranch::Wrath => talents.wrath = rank,
        TalentBranch::Vigor => talents.vigor = rank,
        TalentBranch::Focus => talents.focus = rank,
    };
    talents
}

fn talent_effect(
    branch: TalentBranch,
    rank: u32,
    growth: &crate::data::CombatGrowthTuning,
    chinese: bool,
) -> String {
    let talents = talent_at_rank(branch, rank);
    match branch {
        TalentBranch::Wrath => {
            let impact = (talents.combat_impact_multiplier(growth) - 1.0) * 100.0;
            let damage = (talents.damage_multiplier() - 1.0) * 100.0;
            if chinese {
                format!("打击力度 +{impact:.0}% · 伤害 +{damage:.0}%")
            } else {
                format!("Impact +{impact:.0}% · Damage +{damage:.0}%")
            }
        }
        TalentBranch::Vigor => {
            let barrier = talents.skill_barrier_gain(growth);
            if chinese {
                format!("技能命中获得 {barrier:.0} 护盾，每次施法一次")
            } else {
                format!("Skill hits grant {barrier:.0} barrier, once per cast")
            }
        }
        TalentBranch::Focus => {
            let fury = talents.basic_fury_bonus(growth);
            let reduction = (1.0 - talents.cooldown_multiplier()) * 100.0;
            if chinese {
                format!("斩击命中额外 {fury:.0} 怒气 · 冷却 -{reduction:.0}%")
            } else {
                format!("Strike hits: +{fury:.0} Fury · Cooldowns -{reduction:.0}%")
            }
        }
    }
}

fn talent_description(
    branch: TalentBranch,
    talents: &Talents,
    growth: &crate::data::CombatGrowthTuning,
    chinese: bool,
) -> String {
    let rank = branch.rank(talents).min(8);
    let current = talent_effect(branch, rank, growth, chinese);
    if rank == 8 {
        return format!(
            "{}\n{current}",
            bilingual(chinese, "已达最高阶", "Fully mastered")
        );
    }
    let next = talent_effect(branch, rank + 1, growth, chinese);
    if chinese {
        format!("当前：{current}\n下一阶：{next}")
    } else {
        format!("Now: {current}\nNext: {next}")
    }
}

fn talent_rank_copy(
    branch: TalentBranch,
    rank: u32,
    growth: &crate::data::CombatGrowthTuning,
    chinese: bool,
) -> String {
    let talents = talent_at_rank(branch, rank);
    let effect = match branch {
        TalentBranch::Wrath => {
            let amount = (talents.combat_impact_multiplier(growth) - 1.0) * 100.0;
            if chinese {
                format!("打击力度 +{amount:.0}%")
            } else {
                format!("Impact +{amount:.0}%")
            }
        }
        TalentBranch::Vigor => {
            let amount = talents.skill_barrier_gain(growth);
            if chinese {
                format!("命中护盾 {amount:.0}")
            } else {
                format!("Barrier on hit: {amount:.0}")
            }
        }
        TalentBranch::Focus => {
            let amount = talents.basic_fury_bonus(growth);
            if chinese {
                format!("额外怒气 {amount:.0}")
            } else {
                format!("Extra Fury: {amount:.0}")
            }
        }
    };
    let mastery = match (branch, rank) {
        (TalentBranch::Wrath, 4) => bilingual(chinese, "血涌 · 暴击 +6%", "Bloodrush · Crit +6%"),
        (TalentBranch::Wrath, 8) => {
            bilingual(chinese, "屠戮 · 总伤害 +10%", "Carnage · Total damage +10%")
        }
        (TalentBranch::Vigor, 4) => bilingual(
            chinese,
            "复苏 · 药水治疗 +40%",
            "Second Wind · Potions +40%",
        ),
        (TalentBranch::Vigor, 8) => bilingual(
            chinese,
            "钢铁 · 生命上限 +45",
            "Iron Constitution · Life +45",
        ),
        (TalentBranch::Focus, 4) => bilingual(
            chinese,
            "行云流水 · 冷却额外 -8%",
            "Flow · Extra cooldown -8%",
        ),
        (TalentBranch::Focus, 8) => {
            bilingual(chinese, "暴风 · 怒气消耗 -15%", "Tempest · Fury cost -15%")
        }
        (TalentBranch::Wrath, _) => bilingual(chinese, "每阶伤害 +8%", "Damage +8% per rank"),
        (TalentBranch::Vigor, _) => bilingual(chinese, "每阶生命上限 +12", "Life +12 per rank"),
        (TalentBranch::Focus, _) => {
            bilingual(chinese, "冷却随阶数缩短", "Cooldowns fall with rank")
        }
    };
    format!("{rank} · {effect}\n{mastery}")
}

fn bilingual(chinese: bool, zh: &'static str, en: &'static str) -> &'static str {
    if chinese { zh } else { en }
}

fn style(active: bool, border: &mut BorderColor, bg: &mut BackgroundColor) {
    *border = BorderColor::all(if active {
        Color::srgb(0.86, 0.65, 0.28)
    } else {
        Color::srgb(0.25, 0.23, 0.22)
    });
    bg.0 = if active {
        Color::srgb(0.13, 0.09, 0.035)
    } else {
        Color::srgb(0.02, 0.02, 0.03)
    };
}
fn rune_description(rune: Rune, zh: bool) -> &'static str {
    match (rune, zh) {
        (Rune::Expose, true) => "命中的敌人易伤 22%，持续 4 秒。",
        (Rune::Expose, false) => "Struck enemies take 22% more damage for 4 seconds.",
        (Rune::Hemorrhage, true) => "伤害降至 82%，附加 3.5 秒流血。",
        (Rune::Hemorrhage, false) => "82% damage, plus 3.5 seconds of bleeding.",
        (Rune::Cleanse, true) => "施放时移除自身燃烧。",
        (Rune::Cleanse, false) => "Casting removes your own burning.",
        (Rune::Reap, true) => "伤害 90%、位移 82%；每命中一名敌人恢复 8 点生命。",
        (Rune::Reap, false) => "90% damage and 82% travel; restore 8 health per enemy struck.",
        (Rune::Ember, true) => "全额伤害。",
        (Rune::Ember, false) => "Full damage.",
        (Rune::Frost, true) => "伤害 72%，敌人移速降低 52%，持续 2.8 秒。",
        (Rune::Frost, false) => "72% damage; slows enemies by 52% for 2.8 seconds.",
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn talent_copy_explains_the_next_contact_and_mastery_in_both_languages() {
        let tuning: crate::data::PlayerTuning =
            ron::from_str(include_str!("../../assets/data/player.ron")).unwrap();
        let base = Talents::default();
        assert!(
            talent_description(TalentBranch::Wrath, &base, &tuning.growth, true).contains("+29%")
        );
        assert!(
            talent_description(TalentBranch::Vigor, &base, &tuning.growth, true).contains("6 护盾")
        );
        assert!(
            talent_description(TalentBranch::Focus, &base, &tuning.growth, false)
                .contains("+2 Fury")
        );
        assert!(
            talent_rank_copy(TalentBranch::Wrath, 4, &tuning.growth, true).contains("暴击 +6%")
        );
        assert!(
            talent_rank_copy(TalentBranch::Vigor, 8, &tuning.growth, false).contains("Life +45")
        );
        assert!(
            talent_rank_copy(TalentBranch::Focus, 8, &tuning.growth, true)
                .contains("怒气消耗 -15%")
        );
    }

    #[test]
    fn skill_panel_system_populates_text_and_applies_clicks() {
        use bevy::state::app::StatesPlugin;
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, StatesPlugin));
        app.insert_state(GameState::InGame)
            .insert_resource(BuildOpen { open: true })
            .init_resource::<UiLanguage>()
            .init_resource::<SelectedClass>()
            .insert_resource(crate::class::test_catalog())
            .insert_resource(
                ron::from_str::<crate::data::PlayerTuning>(include_str!(
                    "../../assets/data/player.ron"
                ))
                .unwrap(),
            )
            .add_message::<CombatEvent>()
            .add_plugins(SkillPanelPlugin);
        let player = app
            .world_mut()
            .spawn((
                Player,
                SkillLoadout::default(),
                Fury {
                    current: 60.0,
                    max: 100.0,
                    basic_gain: 18.0,
                    regen_per_sec: 0.0,
                    cost_discount: 0.0,
                },
                PlayerLevel {
                    level: 4,
                    xp: 0,
                    next_level_xp: 100,
                },
                Talents {
                    points: 1,
                    ..default()
                },
                Health {
                    current: 100.0,
                    max: 100.0,
                },
            ))
            .id();
        let title = app
            .world_mut()
            .spawn((SkillText::Tab(BuildTab::Skills), Text::new("")))
            .id();
        let finisher_a = SkillChoice {
            slot: SkillSlot::Finisher,
            alt: false,
        };
        let detail = app
            .world_mut()
            .spawn((SkillText::Skill(finisher_a), Text::new("")))
            .id();
        let rune_choice = RuneChoice {
            slot: SkillSlot::Finisher,
            alt: true,
        };
        let rune_button = app
            .world_mut()
            .spawn((
                rune_choice,
                Interaction::None,
                BorderColor::default(),
                BackgroundColor::default(),
            ))
            .id();
        let rune_text = app
            .world_mut()
            .spawn((SkillText::Rune(rune_choice), Text::new("")))
            .id();
        let locked = SkillChoice {
            slot: SkillSlot::Mobility,
            alt: true,
        };
        let locked_button = app
            .world_mut()
            .spawn((
                locked,
                Interaction::None,
                BorderColor::default(),
                BackgroundColor::default(),
            ))
            .id();
        let unlocked = SkillChoice {
            slot: SkillSlot::Finisher,
            alt: true,
        };
        let unlocked_button = app
            .world_mut()
            .spawn((
                unlocked,
                Interaction::None,
                BorderColor::default(),
                BackgroundColor::default(),
            ))
            .id();
        app.update();
        assert_eq!(app.world().get::<Text>(title).unwrap().0, "技能");
        let detail_text = app.world().get::<Text>(detail).unwrap().0.clone();
        assert!(
            detail_text.contains("已装备") && detail_text.contains("20"),
            "{detail_text}"
        );
        *app.world_mut().get_mut::<Interaction>(rune_button).unwrap() = Interaction::Pressed;
        app.update();
        assert!(app.world().get::<SkillLoadout>(player).unwrap().rune_alt[2][0]);
        assert!(
            app.world()
                .get::<Text>(rune_text)
                .unwrap()
                .0
                .starts_with("已装备 · 血伤")
        );
        // Level 4 cannot take the level-5 mobility skill, but can take the level-4 finisher.
        *app.world_mut()
            .get_mut::<Interaction>(locked_button)
            .unwrap() = Interaction::Pressed;
        *app.world_mut()
            .get_mut::<Interaction>(unlocked_button)
            .unwrap() = Interaction::Pressed;
        app.update();
        let loadout = *app.world().get::<SkillLoadout>(player).unwrap();
        assert!(!loadout.alt[SkillSlot::Mobility.index()]);
        assert!(loadout.alt[SkillSlot::Finisher.index()]);
        *app.world_mut().resource_mut::<UiLanguage>() = UiLanguage::English;
        app.update();
        assert_eq!(app.world().get::<Text>(title).unwrap().0, "Skills");
        assert!(
            app.world()
                .get::<Text>(rune_text)
                .unwrap()
                .0
                .starts_with("Select")
        );
    }
    #[test]
    fn talent_selection_spends_once_and_preserves_vigor_mastery_reward() {
        let mut talents = Talents {
            points: 2,
            vigor: 7,
            ..default()
        };
        let mut health = Health {
            current: 50.0,
            max: 100.0,
        };
        let messages =
            crate::player::spend_talent_rank(&mut talents, &mut health, TalentBranch::Vigor);
        assert_eq!(talents.points, 1);
        assert_eq!(talents.vigor, 8);
        assert_eq!(messages.len(), 2);
        assert!(health.max > 112.0);
        assert_eq!(health.max - health.current, 50.0);
        let max = health.max;
        assert!(
            crate::player::spend_talent_rank(&mut talents, &mut health, TalentBranch::Vigor)
                .is_empty()
        );
        assert_eq!(talents.points, 1);
        assert_eq!(health.max, max);
        talents.points = 0;
        assert!(
            crate::player::spend_talent_rank(&mut talents, &mut health, TalentBranch::Wrath)
                .is_empty()
        );
        assert_eq!(talents.wrath, 0);
    }
    #[test]
    fn rune_choice_follows_the_equipped_skill_of_its_slot() {
        let mut loadout = SkillLoadout::default();
        let frost_on_a = RuneChoice {
            slot: SkillSlot::Area,
            alt: true,
        };
        select_rune(&mut loadout, frost_on_a);
        assert!(rune_selected(&loadout, frost_on_a));
        // Switching the slot's skill brings that skill's own rune choice, untouched so far.
        loadout.alt[SkillSlot::Area.index()] = true;
        assert!(!rune_selected(&loadout, frost_on_a));
        select_rune(&mut loadout, frost_on_a);
        loadout.alt[SkillSlot::Area.index()] = false;
        assert!(rune_selected(&loadout, frost_on_a));
        for slot in [SkillSlot::Finisher, SkillSlot::Mobility] {
            assert!(rune_selected(&loadout, RuneChoice { slot, alt: false }));
        }
    }
    #[test]
    fn every_rune_has_copy_in_both_languages() {
        for rune in [
            Rune::Expose,
            Rune::Hemorrhage,
            Rune::Cleanse,
            Rune::Reap,
            Rune::Ember,
            Rune::Frost,
        ] {
            assert!(!rune_description(rune, true).is_empty());
            assert!(rune_description(rune, false).is_ascii());
        }
    }
}
