//! Image-backed navigation symbols. No icon depends on a platform font glyph.

use bevy::prelude::*;
use bevy::ui::widget::NodeImageMode;

#[derive(Clone, Copy)]
pub(super) enum UiIcon {
    Begin,
    Continue,
    Difficulty,
    Inventory,
    Talents,
    Audio,
    Journal,
    Gold,
    Skull,
}

impl UiIcon {
    pub(super) fn image(self, asset_server: &AssetServer) -> ImageNode {
        let (path, cell, columns, index) = match self {
            Self::Begin => (super::UI_SKILL_ICONS_IMAGE, 256.0, 3, 0),
            Self::Continue => (super::UI_GEAR_ICONS_IMAGE, 222.0, 4, 4),
            Self::Difficulty => (super::UI_TALENT_SIGILS_IMAGE, 362.0, 3, 0),
            Self::Inventory => (super::UI_GEAR_ICONS_IMAGE, 222.0, 4, 7),
            Self::Talents => (super::UI_TALENT_SIGILS_IMAGE, 362.0, 3, 2),
            Self::Audio => (UTILITY_ICONS, 256.0, 4, 0),
            Self::Journal => (UTILITY_ICONS, 256.0, 4, 1),
            Self::Gold => (UTILITY_ICONS, 256.0, 4, 2),
            Self::Skull => (UTILITY_ICONS, 256.0, 4, 3),
        };
        ImageNode {
            image: asset_server.load(path),
            rect: Some(super::atlas_rect(cell, cell, columns, index)),
            image_mode: NodeImageMode::Stretch,
            ..default()
        }
    }
}

const UTILITY_ICONS: &str = "images/generated/ui/utility-icons-reliquary.png";

pub(super) fn ui_icon(asset_server: &AssetServer, icon: UiIcon) -> ImageNode {
    icon.image(asset_server)
}
