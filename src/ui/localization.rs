//! Presentation localization. Gameplay retains stable English identifiers and saves.
use bevy::{
    prelude::*,
    text::{FontSource, Text2dUpdateSystems},
    ui::UiSystems,
};
use std::collections::HashMap;

pub(super) const UI_FONT_CHINESE: &str = "fonts/NotoSansSC.ttf";

#[derive(Resource, Default, Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum UiLanguage {
    #[default]
    Chinese,
    English,
}

#[derive(Component)]
pub(crate) struct NativeLanguageName;

#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub(super) struct LocalizeText;

pub(super) struct LocalizationPlugin;

impl Plugin for LocalizationPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<UiLanguage>()
            .init_resource::<Catalog>()
            .add_systems(Startup, load_chinese_font)
            .add_systems(
                PostUpdate,
                localize_text
                    .in_set(LocalizeText)
                    .before(UiSystems::Content)
                    .before(Text2dUpdateSystems),
            );
    }
}

#[derive(Resource)]
struct ChineseFont(Handle<Font>);

fn load_chinese_font(mut commands: Commands, assets: Res<AssetServer>) {
    commands.insert_resource(ChineseFont(assets.load(UI_FONT_CHINESE)));
}

#[derive(Resource)]
struct Catalog {
    // Longest matching phrase wins; indexing by initial byte avoids scanning
    // the full catalog for every glyph in frequently updated HUD strings.
    entries: HashMap<u8, Vec<(String, &'static str)>>,
}

impl Default for Catalog {
    fn default() -> Self {
        let mut entries: HashMap<u8, Vec<(String, &'static str)>> = HashMap::new();
        for line in include_str!("../../assets/data/zh_cn.tsv")
            .lines()
            .chain(include_str!("../../assets/data/journal_zh_cn.tsv").lines())
            .chain(include_str!("../../assets/data/panels_zh_cn.tsv").lines())
        {
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            let (english, chinese) = line
                .split_once('\t')
                .expect("translation must contain a tab");
            assert!(
                !english.is_empty() && !chinese.is_empty(),
                "empty translation"
            );
            let english = english.to_ascii_lowercase();
            entries
                .entry(english.as_bytes()[0])
                .or_default()
                .push((english, chinese));
        }
        for phrases in entries.values_mut() {
            phrases.sort_by_key(|(english, _)| std::cmp::Reverse(english.len()));
        }
        Self { entries }
    }
}

fn word_byte(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || byte == b'_' || byte == b'\''
}

impl Catalog {
    fn translate(&self, source: &str) -> String {
        let lower = source.to_ascii_lowercase();
        // Existing narrow widgets shorten source prose before rendering. Only
        // resolve a long truncated prefix when all matching phrases agree.
        if let Some(prefix) = lower
            .strip_suffix("...")
            .or_else(|| lower.strip_suffix('…'))
            && prefix.len() >= 12
        {
            let mut candidates = self
                .entries
                .values()
                .flatten()
                .filter(|(english, _)| english.starts_with(prefix.trim_end()))
                .map(|(_, chinese)| *chinese);
            if let Some(first) = candidates.next()
                && candidates.all(|candidate| candidate == first)
            {
                return first.to_owned();
            }
        }
        let bytes = lower.as_bytes();
        let mut translated = String::with_capacity(source.len());
        let mut offset = 0;
        while offset < source.len() {
            // Compact HUD lines concatenate independently shortened labels.
            // Recover each unambiguous phrase even when numbers follow it.
            if (offset == 0 || !word_byte(bytes[offset - 1]))
                && let Some(end) = lower[offset..].find("...")
                && (8..=80).contains(&end)
            {
                let prefix = lower[offset..offset + end].trim_end();
                let mut candidates = self
                    .entries
                    .values()
                    .flatten()
                    .filter(|(english, _)| english.starts_with(prefix))
                    .map(|(_, chinese)| *chinese);
                if let Some(first) = candidates.next()
                    && candidates.all(|candidate| candidate == first)
                {
                    translated.push_str(first);
                    offset += end + 3;
                    continue;
                }
            }
            let matched = self.entries.get(&bytes[offset]).and_then(|phrases| {
                phrases.iter().find(|(english, _)| {
                    let end = offset + english.len();
                    lower[offset..].starts_with(english)
                        // "on" is a state label only when it is the whole text;
                        // never turn the English preposition into "开启".
                        && (!matches!(english.as_str(), "on" | "off")
                            || (offset == 0 && end == bytes.len()))
                        && (offset == 0
                            || !word_byte(bytes[offset - 1])
                            || !word_byte(bytes[offset]))
                        && (end == bytes.len()
                            || !word_byte(bytes[end])
                            || !word_byte(bytes[end - 1]))
                })
            });
            if let Some((english, chinese)) = matched {
                translated.push_str(chinese);
                offset += english.len();
            } else {
                let character = source[offset..].chars().next().unwrap();
                translated.push(character);
                offset += character.len_utf8();
            }
        }
        translated
    }
}

struct CachedText {
    source: String,
    rendered: String,
    language: UiLanguage,
    english_font: Option<FontSource>,
}

impl CachedText {
    fn new(source: &str, font: Option<&TextFont>) -> Self {
        Self {
            source: source.to_owned(),
            rendered: source.to_owned(),
            language: UiLanguage::English,
            english_font: font.map(|font| font.font.clone()),
        }
    }

    fn refresh(&mut self, current: &str, language: UiLanguage, catalog: &Catalog) -> bool {
        let source_changed = current != self.rendered && current != self.source;
        if source_changed {
            self.source = current.to_owned();
        }
        if source_changed || language != self.language {
            self.rendered = match language {
                UiLanguage::Chinese => catalog.translate(&self.source),
                UiLanguage::English => self.source.clone(),
            };
            self.language = language;
        }
        current != self.rendered
    }
}

#[allow(clippy::type_complexity)]
fn localize_text(
    language: Res<UiLanguage>,
    catalog: Res<Catalog>,
    chinese: Res<ChineseFont>,
    mut cache: Local<HashMap<Entity, CachedText>>,
    mut texts: Query<
        (
            Entity,
            Option<&mut Text>,
            Option<&mut TextSpan>,
            Option<&mut Text2d>,
            Option<&mut TextFont>,
            Option<&NativeLanguageName>,
            Option<&super::ObjectiveCompassText>,
        ),
        Or<(With<Text>, With<TextSpan>, With<Text2d>)>,
    >,
) {
    for (entity, text, span, world_text, mut font, language_hint, compass) in &mut texts {
        let mut value = if let Some(text) = text {
            text.map_unchanged(|text| &mut text.0)
        } else if let Some(span) = span {
            span.map_unchanged(|span| &mut span.0)
        } else if let Some(text) = world_text {
            text.map_unchanged(|text| &mut text.0)
        } else {
            continue;
        };
        let cached = cache
            .entry(entity)
            .or_insert_with(|| CachedText::new(&value, font.as_deref()));
        if language_hint.is_none() {
            cached.refresh(&value, *language, &catalog);
            if *language == UiLanguage::Chinese
                && compass == Some(&super::ObjectiveCompassText::Direction)
                && let Some(direction) = compass_direction(&cached.source)
            {
                cached.rendered = direction.to_owned();
            }
            if value.as_str() != cached.rendered {
                *value = cached.rendered.clone();
            }
        }
        if let Some(ref mut font) = font {
            if cached.english_font.is_none() {
                cached.english_font = Some(font.font.clone());
            }
            let target = if *language == UiLanguage::Chinese || language_hint.is_some() {
                FontSource::Handle(chinese.0.clone())
            } else {
                cached.english_font.clone().unwrap_or_default()
            };
            if font.font != target {
                font.font = target;
            }
        }
    }
    cache.retain(|entity, _| texts.contains(*entity));
}

fn compass_direction(source: &str) -> Option<&'static str> {
    match source {
        "N" => Some("北"),
        "E" => Some("东"),
        "S" => Some("南"),
        "W" => Some("西"),
        "NE" => Some("东北"),
        "NW" => Some("西北"),
        "SE" => Some("东南"),
        "SW" => Some("西南"),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn journal_sentences_are_fully_chinese_and_restore_english() {
        let catalog = Catalog::default();
        for line in include_str!("../../assets/data/journal_zh_cn.tsv").lines() {
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            let (english, _) = line
                .split_once('\t')
                .expect("translation must contain a tab");
            let mut cached = CachedText::new(english, None);
            cached.refresh(english, UiLanguage::Chinese, &catalog);
            assert!(
                !cached
                    .rendered
                    .replace("[J]", "")
                    .chars()
                    .any(|c| c.is_ascii_alphabetic()),
                "{}: {}",
                english,
                cached.rendered
            );
            let translated = cached.rendered.clone();
            cached.refresh(&translated, UiLanguage::English, &catalog);
            assert_eq!(cached.rendered, english);
        }
    }

    #[test]
    fn inventory_weapon_names_are_fully_localized_before_layout() {
        let catalog = Catalog::default();
        let table: crate::data::LootTable =
            ron::from_str(include_str!("../../assets/data/loot.ron")).unwrap();
        let mut names = vec!["Initiate Blade".to_string()];
        names.extend(table.weapons.into_iter().map(|weapon| weapon.name));
        for name in names {
            for source in [name.clone(), format!("rare Tempest {name} of Embers")] {
                let translated = catalog.translate(&source);
                assert!(
                    !translated.chars().any(|c| c.is_ascii_alphabetic()),
                    "{source} left untranslated: {translated}"
                );
            }
        }
    }

    #[test]
    fn panel_details_translate_without_losing_numbers_or_english_source() {
        let catalog = Catalog::default();
        for (source, expected) in [
            ("Talent paths", "天赋之路"),
            ("Available talent points: 3", "可用天赋点数: 3"),
            ("Passive bonuses", "被动加成"),
            (
                "Damage bonus +12 · Critical chance 8% · Cooldown reduction 15%",
                "伤害加成 +12 · 暴击率 8% · 冷却缩减 15%",
            ),
            (
                "Inventory slots 1/12 · Gold 13 · Ember shards 0",
                "背包容量 1/12 · 金币 13 · 余烬碎片 0",
            ),
            (
                "Spend points: 1 Wrath · 2 Vigor · 3 Focus",
                "投入天赋：1 愤怒 · 2 活力 · 3 专注",
            ),
            ("K · Close", "K · 关闭"),
        ] {
            let mut text = CachedText::new(source, None);
            text.refresh(source, UiLanguage::Chinese, &catalog);
            assert_eq!(text.rendered, expected);
            let translated = text.rendered.clone();
            text.refresh(&translated, UiLanguage::English, &catalog);
            assert_eq!(text.rendered, source);
        }
    }

    #[test]
    fn chinese_is_default_and_dynamic_values_are_preserved() {
        assert_eq!(UiLanguage::default(), UiLanguage::Chinese);
        let catalog = Catalog::default();
        assert_eq!(catalog.translate("LIFE 130 / 130"), "生命 130 / 130");
        assert_eq!(catalog.translate("FURY 35 / 100"), "怒气 35 / 100");
        assert_eq!(catalog.translate("BEGIN DESCENT"), "踏入深渊");
    }

    #[test]
    fn language_toggle_restores_source_and_accepts_gameplay_updates() {
        let catalog = Catalog::default();
        let mut text = CachedText::new("LIFE 130 / 130", None);
        assert!(text.refresh("LIFE 130 / 130", UiLanguage::Chinese, &catalog));
        assert_eq!(text.rendered, "生命 130 / 130");
        assert!(!text.refresh("生命 130 / 130", UiLanguage::Chinese, &catalog));
        assert!(text.refresh("LIFE 96 / 130", UiLanguage::Chinese, &catalog));
        assert_eq!(text.rendered, "生命 96 / 130");
        assert!(text.refresh("生命 96 / 130", UiLanguage::English, &catalog));
        assert_eq!(text.rendered, "LIFE 96 / 130");
        assert!(text.refresh("LIFE 96 / 130", UiLanguage::Chinese, &catalog));
        assert_eq!(text.rendered, "生命 96 / 130");
    }

    #[test]
    fn actual_menu_hud_enemy_and_skill_prompts_have_chinese_coverage() {
        let catalog = Catalog::default();
        for (source, expected) in [
            ("SHADOW\nRELIQUARY", "暗影\n圣匣"),
            ("Difficulty: Normal", "难度: 普通"),
            ("Ashbone Guard", "灰骨卫兵"),
            ("Click to equip", "点击装备"),
            (
                "Rupture: move closer and face an enemy",
                "裂地斩：靠近并面向敌人",
            ),
            ("LMB / RMB", "左键 / 右键"),
            (
                "next run: survive first room, evade through swings, drink potion before 30%",
                "下次征途：稳过首战，闪避挥击，生命降至三成前喝下药水",
            ),
            (
                "Next: continue to Chapter II on Normal",
                "下一步：继续前往 第二章 ／普通难度",
            ),
        ] {
            assert_eq!(
                catalog.translate(source),
                expected,
                "missing localization for {source}"
            );
        }
    }

    #[test]
    fn rendered_enemy_tooltip_and_shortened_route_are_localized() {
        let catalog = Catalog::default();
        assert_eq!(
            catalog.translate("Purge the reliquary guards"),
            "肃清圣匣卫兵"
        );
        assert_eq!(
            catalog.translate("rotate between main and opti..."),
            "往返主线与支线地点"
        );
        let compact = catalog
            .translate("NOW Sentinel command rea... | Ashbone G... 16/46 | rotate between ...");
        assert!(
            !compact
                .chars()
                .any(|character| character.is_ascii_alphabetic()),
            "{compact}"
        );
        assert!(compact.contains("16/46"));
        let tooltip = catalog.translate("Ash Vanguard - Melee | 16/46 HP (36%)\nno affixes | role: ash vanguard | tip: sidestep Shield Bash, punish guard");
        assert!(
            !tooltip
                .chars()
                .any(|character| character.is_ascii_alphabetic()),
            "{tooltip}"
        );
        assert!(tooltip.contains("16/46"));
        assert_eq!(catalog.translate("E"), "E");
        assert_eq!(compass_direction("E"), Some("东"));
    }

    #[test]
    fn phrase_matching_preserves_unknown_names_controls_and_word_boundaries() {
        let catalog = Catalog::default();
        assert_eq!(catalog.translate("F8 / 12.5%"), "F8 / 12.5%");
        assert_eq!(
            catalog.translate("Lifeform_Custom_42"),
            "Lifeform_Custom_42"
        );
        assert_eq!(catalog.translate("生命\n130"), "生命\n130");
    }
    #[test]
    fn every_story_scene_renders_complete_chinese_copy() {
        use crate::story::StoryBeat;
        let catalog = Catalog::default();
        let beats = [
            StoryBeat::Arrival,
            StoryBeat::CacheRevealed,
            StoryBeat::SealRecovered,
            StoryBeat::SanctumOpened,
            StoryBeat::SealWardenSlain,
            StoryBeat::AltarDemand,
            StoryBeat::KeeperAwakened,
            StoryBeat::KeeperBloodied,
            StoryBeat::Victory,
            StoryBeat::HollowArrival,
            StoryBeat::ChoirCacheRevealed,
            StoryBeat::ChoirSealRecovered,
            StoryBeat::BellCryptOpened,
            StoryBeat::BellWardenSlain,
            StoryBeat::SoulFontDemand,
            StoryBeat::MatronAwakened,
            StoryBeat::MatronBloodied,
            StoryBeat::BastionVictory,
            StoryBeat::CursedShrine,
            StoryBeat::BloodObelisk,
            StoryBeat::ReliquaryVault,
            StoryBeat::EmberRift,
            StoryBeat::AshenPylon,
        ];
        for beat in beats {
            let scene = crate::story_content::story_scene(beat);
            let mut human_copy = vec![beat.title(), beat.line(), scene.next_action];
            for line in scene.lines {
                human_copy.extend([line.speaker, line.text]);
            }
            for source in human_copy {
                let localized = catalog.translate(source);
                assert!(
                    !localized.chars().any(|c| c.is_ascii_alphabetic()),
                    "{beat:?}: {source} => {localized}"
                );
            }
        }
        assert_eq!(catalog.translate("Steward Edda"), "执事艾达");
        assert_eq!(catalog.translate("Quartermaster Vey"), "军需官维伊");
        assert_eq!(catalog.translate("Sentinel, the Reliquary is falling. Three stolen seals bind Malrec below the ash gate."),"守卫，圣匣正走向覆灭。被夺走的三道封印，将马尔雷克束缚在灰烬之门下。");
    }
}
