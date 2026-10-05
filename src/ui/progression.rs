//! Earned level rewards and an unobtrusive reminder to spend talent points.
use super::{
    HudUi, UI_FONT_BOLD, UI_FONT_REGULAR,
    localization::{NativeLanguageName, UiLanguage},
    skills::BuildTab,
};
use crate::{
    BuildOpen, GameState, InventoryOpen, JournalOpen,
    player::{Player, PlayerLevelUp, Talents},
};
use bevy::prelude::*;

pub(super) struct ProgressionFeedbackPlugin;

impl Plugin for ProgressionFeedbackPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<RewardCard>()
            .add_systems(OnEnter(GameState::InGame), spawn_progression)
            .add_systems(
                Update,
                (update_progression, open_talents)
                    .chain()
                    .run_if(in_state(GameState::InGame)),
            );
    }
}

#[derive(Resource, Default)]
struct RewardCard {
    reward: Option<PlayerLevelUp>,
    remaining: f32,
}

impl RewardCard {
    fn earn(&mut self, reward: PlayerLevelUp) {
        if let Some(previous) = &mut self.reward {
            previous.level = reward.level;
            previous.paragon_rank = reward.paragon_rank;
            previous.armor_gained += reward.armor_gained;
            previous.crit_gained += reward.crit_gained;
            previous.life_restored |= reward.life_restored;
            previous.health_gained += reward.health_gained;
            previous.damage_gained += reward.damage_gained;
            previous.points_gained += reward.points_gained;
        } else {
            self.reward = Some(reward);
        }
        self.remaining = 4.0;
    }

    fn tick(&mut self, seconds: f32) {
        self.remaining = (self.remaining - seconds).max(0.0);
        if self.remaining == 0.0 {
            self.reward = None;
        }
    }
}

#[derive(Component)]
struct RewardText;
#[derive(Component)]
struct TalentReminder;
#[derive(Component)]
struct ReminderText;

fn spawn_progression(
    mut commands: Commands,
    assets: Res<AssetServer>,
    mut card: ResMut<RewardCard>,
) {
    *card = RewardCard::default();
    commands.spawn((
        Node {
            position_type: PositionType::Absolute,
            top: Val::Px(102.0),
            left: Val::Percent(50.0),
            width: Val::Px(390.0),
            max_width: Val::Percent(88.0),
            padding: UiRect::axes(Val::Px(22.0), Val::Px(14.0)),
            border: UiRect::all(Val::Px(1.0)),
            display: Display::None,
            ..default()
        },
        Text::default(),
        TextFont {
            font: assets.load(UI_FONT_BOLD).into(),
            font_size: FontSize::Px(17.0),
            ..default()
        },
        TextColor(Color::srgb(0.98, 0.86, 0.60)),
        TextLayout::new(Justify::Center, LineBreak::WordOrCharacter),
        bevy::text::LineHeight::RelativeToFont(1.5),
        BackgroundColor(Color::srgba(0.035, 0.028, 0.022, 0.95)),
        BorderColor::all(Color::srgba(0.72, 0.55, 0.27, 0.85)),
        GlobalZIndex(9),
        UiTransform::from_translation(bevy::ui::Val2::new(Val::Percent(-50.0), Val::Px(0.0))),
        RewardText,
        NativeLanguageName,
        HudUi,
    ));
    commands
        .spawn((
            Button,
            Node {
                position_type: PositionType::Absolute,
                bottom: Val::Px(203.0),
                left: Val::Percent(50.0),
                width: Val::Px(260.0),
                max_width: Val::Percent(85.0),
                padding: UiRect::axes(Val::Px(14.0), Val::Px(9.0)),
                justify_content: JustifyContent::Center,
                border: UiRect::all(Val::Px(1.0)),
                display: Display::None,
                ..default()
            },
            BackgroundColor(Color::srgba(0.06, 0.042, 0.024, 0.94)),
            BorderColor::all(Color::srgba(0.66, 0.49, 0.25, 0.9)),
            GlobalZIndex(9),
            UiTransform::from_translation(bevy::ui::Val2::new(Val::Percent(-50.0), Val::Px(0.0))),
            TalentReminder,
            HudUi,
        ))
        .with_children(|parent| {
            parent.spawn((
                Text::default(),
                TextFont {
                    font: assets.load(UI_FONT_REGULAR).into(),
                    font_size: FontSize::Px(14.0),
                    ..default()
                },
                TextColor(Color::srgb(0.95, 0.80, 0.48)),
                ReminderText,
                NativeLanguageName,
            ));
        });
}

fn reward_copy(reward: PlayerLevelUp, language: UiLanguage) -> String {
    let chinese = language == UiLanguage::Chinese;
    let title = match (chinese, reward.paragon_rank) {
        (true, 0) => format!("晋升至 {} 级", reward.level),
        (false, 0) => format!("LEVEL {} REACHED", reward.level),
        (true, rank) => format!("余烬巅峰 · 第 {rank} 阶"),
        (false, rank) => format!("EMBER PARAGON · RANK {rank}"),
    };
    let mut rewards = Vec::new();
    for (amount, chinese_label, english_label) in [
        (reward.health_gained, "生命上限", "Maximum life"),
        (reward.damage_gained, "伤害", "Damage"),
        (reward.armor_gained, "护甲", "Armor"),
    ] {
        if amount > 0.0 {
            let label = if chinese {
                chinese_label
            } else {
                english_label
            };
            rewards.push(format!("{label} +{amount}"));
        }
    }
    if reward.crit_gained > 0.0 {
        let label = if chinese {
            "暴击率"
        } else {
            "Critical chance"
        };
        rewards.push(format!("{label} +{:.1}%", reward.crit_gained * 100.0));
    }
    let points = if chinese {
        format!("获得 {} 点天赋", reward.points_gained)
    } else {
        format!("{} talent points gained", reward.points_gained)
    };
    rewards.push(points);
    if reward.life_restored {
        rewards.push(
            if chinese {
                "生命已恢复"
            } else {
                "Life restored"
            }
            .into(),
        );
    }
    format!("{title}\n{}", rewards.join(" · "))
}

pub(super) fn experience_progress(
    level: &crate::player::PlayerLevel,
    paragon: &crate::player::EmberParagon,
) -> f32 {
    let (xp, next) = if level.level >= crate::player::CHAPTER_LEVEL_CAP {
        (paragon.xp, paragon.next_rank_xp)
    } else {
        (level.xp, level.next_level_xp)
    };
    super::percent(xp as f32, next as f32)
}

fn reminder_visible(points: u32, build_open: bool) -> bool {
    points > 0 && !build_open
}

#[allow(clippy::type_complexity, clippy::too_many_arguments)]
fn update_progression(
    time: Res<Time>,
    language: Res<UiLanguage>,
    build: Res<BuildOpen>,
    mut earned: MessageReader<PlayerLevelUp>,
    mut card: ResMut<RewardCard>,
    player: Query<&Talents, With<Player>>,
    mut rewards: Query<
        (&mut Text, &mut Node),
        (
            With<RewardText>,
            Without<ReminderText>,
            Without<TalentReminder>,
        ),
    >,
    mut reminders: Query<&mut Node, (With<TalentReminder>, Without<RewardText>)>,
    mut labels: Query<&mut Text, (With<ReminderText>, Without<RewardText>)>,
) {
    card.tick(time.delta_secs());
    for event in earned.read() {
        card.earn(*event);
    }
    for (mut text, mut node) in &mut rewards {
        node.display = if card.reward.is_some() {
            Display::Flex
        } else {
            Display::None
        };
        if let Some(reward) = card.reward {
            **text = reward_copy(reward, *language);
        }
    }
    let points = player.single().map_or(0, |talents| talents.points);
    for mut node in &mut reminders {
        node.display = if reminder_visible(points, build.open) {
            Display::Flex
        } else {
            Display::None
        };
    }
    for mut text in &mut labels {
        **text = match *language {
            UiLanguage::Chinese => format!("可用天赋点：{points}    分配 [K]"),
            UiLanguage::English => format!("{points} talent points    Spend [K]"),
        };
    }
}

fn open_talents(
    interactions: Query<&Interaction, (Changed<Interaction>, With<TalentReminder>)>,
    mut build: ResMut<BuildOpen>,
    mut tab: ResMut<BuildTab>,
    mut inventory: ResMut<InventoryOpen>,
    mut journal: ResMut<JournalOpen>,
) {
    if interactions
        .iter()
        .any(|interaction| *interaction == Interaction::Pressed)
    {
        build.open = true;
        *tab = BuildTab::Talents;
        inventory.open = false;
        journal.open = false;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn consecutive_levels_show_actual_combined_rewards_for_four_seconds() {
        let mut card = RewardCard::default();
        for level in [2, 3] {
            card.earn(PlayerLevelUp {
                level,
                health_gained: 18.0,
                damage_gained: 2.0,
                points_gained: 1,
                life_restored: true,
                ..default()
            });
        }
        let reward = card.reward.unwrap();
        assert_eq!(
            (
                reward.level,
                reward.health_gained,
                reward.damage_gained,
                reward.points_gained
            ),
            (3, 36.0, 4.0, 2)
        );
        assert!(reward_copy(reward, UiLanguage::Chinese).contains("生命上限 +36"));
        card.tick(3.99);
        assert!(card.reward.is_some());
        card.tick(0.02);
        assert!(card.reward.is_none());
    }

    #[test]
    fn paragon_card_shows_only_earned_stats_and_never_claims_full_healing() {
        let mut card = RewardCard::default();
        card.earn(PlayerLevelUp {
            level: 6,
            paragon_rank: 1,
            damage_gained: 1.5,
            points_gained: 1,
            ..default()
        });
        card.earn(PlayerLevelUp {
            level: 6,
            paragon_rank: 2,
            health_gained: 12.0,
            points_gained: 1,
            ..default()
        });
        card.earn(PlayerLevelUp {
            level: 6,
            paragon_rank: 3,
            crit_gained: 0.005,
            points_gained: 1,
            ..default()
        });
        card.earn(PlayerLevelUp {
            level: 6,
            paragon_rank: 4,
            armor_gained: 4.0,
            points_gained: 1,
            ..default()
        });
        let reward = card.reward.unwrap();
        let chinese = reward_copy(reward, UiLanguage::Chinese);
        assert!(chinese.contains("第 4 阶"));
        assert!(chinese.contains("伤害 +1.5"));
        assert!(chinese.contains("暴击率 +0.5%"));
        assert!(chinese.contains("护甲 +4"));
        assert!(chinese.contains("获得 4 点天赋"));
        assert!(!chinese.contains("生命已恢复"));
        let english = reward_copy(reward, UiLanguage::English);
        assert!(english.contains("RANK 4"));
        assert!(!english.contains("Life restored"));
    }

    #[test]
    fn experience_bar_continues_filling_after_the_level_cap() {
        let mut level = crate::player::PlayerLevel {
            level: 5,
            xp: 50,
            next_level_xp: 100,
        };
        let paragon = crate::player::EmberParagon {
            rank: 2,
            xp: 35,
            next_rank_xp: 140,
        };
        assert_eq!(experience_progress(&level, &paragon), 50.0);
        level.level = crate::player::CHAPTER_LEVEL_CAP;
        level.xp = 0;
        assert_eq!(experience_progress(&level, &paragon), 25.0);
    }

    #[test]
    fn loading_a_hero_has_no_reward_but_unspent_points_remain_discoverable() {
        let card = RewardCard::default();
        assert!(card.reward.is_none());
        assert!(reminder_visible(4, false));
        assert!(!reminder_visible(0, false));
        assert!(!reminder_visible(4, true));
    }
}
