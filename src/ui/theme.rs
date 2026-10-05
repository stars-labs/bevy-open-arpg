//! Shared engraved display type and readable book type for the game's UI.
use bevy::{prelude::*, text::FontSource, ui::UiSystems};

pub(super) const UI_FONT_REGULAR: &str = "fonts/CrimsonPro.ttf";
pub(super) const UI_FONT_BOLD: &str = "fonts/Cinzel.ttf";

pub(super) struct ReliquaryThemePlugin;

impl Plugin for ReliquaryThemePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, load_typefaces).add_systems(
            PostUpdate,
            style_ui_text
                .before(super::localization::LocalizeText)
                .before(UiSystems::Content),
        );
    }
}

#[derive(Resource)]
struct Typefaces {
    body: Handle<Font>,
    display: Handle<Font>,
}

fn load_typefaces(mut commands: Commands, assets: Res<AssetServer>) {
    commands.insert_resource(Typefaces {
        body: assets.load(UI_FONT_REGULAR),
        display: assets.load(UI_FONT_BOLD),
    });
}

#[allow(clippy::type_complexity)]
fn style_ui_text(
    mut commands: Commands,
    typefaces: Res<Typefaces>,
    mut text: Query<(Entity, &mut TextFont, Option<&TextShadow>), (With<Text>, Changed<TextFont>)>,
) {
    for (entity, mut font, shadow) in &mut text {
        let default_face = matches!(&font.font, FontSource::Handle(handle) if *handle == Handle::<Font>::default());
        let display_face =
            matches!(&font.font, FontSource::Handle(handle) if *handle == typefaces.display);
        let heading = display_face
            || (default_face && matches!(font.font_size, FontSize::Px(size) if size >= 24.));

        if default_face {
            font.font = if heading {
                typefaces.display.clone().into()
            } else {
                typefaces.body.clone().into()
            };
        }
        // Variable fonts keep fine serifs visible at small HUD sizes. Preserve
        // explicit weights so a title or disabled control can choose its own.
        if font.weight == FontWeight::NORMAL {
            font.weight = if heading {
                FontWeight::SEMIBOLD
            } else {
                FontWeight::MEDIUM
            };
        }
        if shadow.is_none() {
            commands.entity(entity).insert(TextShadow {
                offset: if heading {
                    Vec2::new(0., 2.)
                } else {
                    Vec2::new(0., 1.)
                },
                color: Color::srgba(0.025, 0.012, 0.008, 0.95),
            });
        }
    }
}
