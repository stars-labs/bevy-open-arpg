//! North-up map markers and objective bearings.
use super::*;

pub(super) fn update_minimap(
    mut commands: Commands,
    progress: Res<ChapterProgress>,
    map: Query<Entity, With<MinimapUi>>,
    dots: Query<Entity, With<MinimapDot>>,
    mut title_text: Query<&mut Text, With<MinimapTitleText>>,
    queries: MinimapQueries,
) {
    let Ok(map_entity) = map.single() else {
        return;
    };
    let (player, enemies, loot, health_globes, fury_globes, interactables) = queries;
    let Ok((player_transform, animation)) = player.single() else {
        return;
    };
    let center = player_transform.translation;

    for mut text in &mut title_text {
        **text = progress.chapter_id().short_title().to_string();
    }
    for entity in &dots {
        queue_safe_despawn(commands.reborrow(), entity);
    }

    for transform in &loot {
        spawn_minimap_dot(
            &mut commands,
            map_entity,
            transform.translation,
            center,
            4.0,
            Color::srgb(0.95, 0.72, 0.18),
        );
    }
    for transform in &health_globes {
        spawn_minimap_dot(
            &mut commands,
            map_entity,
            transform.translation,
            center,
            4.5,
            Color::srgb(1.0, 0.08, 0.12),
        );
    }
    for transform in &fury_globes {
        spawn_minimap_dot(
            &mut commands,
            map_entity,
            transform.translation,
            center,
            4.2,
            Color::srgb(0.18, 0.58, 1.0),
        );
    }
    for (transform, interactable) in &interactables {
        if interactable.used {
            continue;
        }
        let role = minimap_interactable_role(&progress, interactable.kind);
        let (size, color) = minimap_interactable_style(role);
        spawn_minimap_dot(
            &mut commands,
            map_entity,
            transform.translation,
            center,
            size,
            color,
        );
    }
    for (transform, enemy) in &enemies {
        let (size, color) = minimap_enemy_style(enemy);
        spawn_minimap_dot(
            &mut commands,
            map_entity,
            transform.translation,
            center,
            size,
            color,
        );
    }
    commands.entity(map_entity).with_children(|map| {
        let position = minimap_position(center, center);
        spawn_arrow(
            map,
            Node {
                position_type: PositionType::Absolute,
                left: Val::Px(position.x - 8.0),
                top: Val::Px(position.y - 8.0),
                ..default()
            },
            minimap_position(center + animation.facing_direction(), center) - position,
            Color::srgb(0.56, 1.0, 0.72),
            false,
        );
    });
}

pub(super) fn update_objective_compass_hud(
    progress: Res<ChapterProgress>,
    player: Query<&Transform, With<Player>>,
    enemies: Query<(&Transform, &Enemy, &Health)>,
    interactables: Query<(&Transform, &Interactable)>,
    mut panels: Query<&mut Node, With<ObjectiveCompassPanel>>,
    mut texts: Query<(&mut Text, &ObjectiveCompassText)>,
    mut arrows: Query<(&mut UiTransform, &mut Visibility), With<ObjectiveArrow>>,
) {
    let display = player.single().ok().and_then(|transform| {
        objective_compass_display_from_targets(
            &progress,
            transform.translation,
            enemies
                .iter()
                .map(|(transform, enemy, health)| ObjectiveCompassEnemyTarget {
                    id: enemy.id.as_str(),
                    current_health: health.current,
                    position: transform.translation,
                }),
            interactables.iter().map(|(transform, interactable)| {
                ObjectiveCompassInteractableTarget {
                    kind: interactable.kind,
                    used: interactable.used,
                    reusable: interactable.reusable,
                    position: transform.translation,
                }
            }),
        )
    });

    for (mut transform, mut visibility) in &mut arrows {
        let bearing = display
            .as_ref()
            .map_or(Vec2::ZERO, |display| display.bearing);
        transform.rotation = arrow_rotation(bearing);
        *visibility = if bearing.length_squared() < 0.25 {
            Visibility::Hidden
        } else {
            Visibility::Inherited
        };
    }
    let panel_display = if display.is_some() {
        Display::Flex
    } else {
        Display::None
    };
    for mut panel in &mut panels {
        panel.display = panel_display;
    }

    for (mut text, slot) in &mut texts {
        **text = match (display.as_ref(), slot) {
            (Some(display), ObjectiveCompassText::Direction) => display.direction.to_string(),
            (Some(display), ObjectiveCompassText::Detail) => display.detail.clone(),
            (None, _) => String::new(),
        };
    }
}

pub(super) fn minimap_enemy_style(enemy: &Enemy) -> (f32, Color) {
    if enemy.id == "keeper" {
        return (9.8, Color::srgb(0.86, 0.10, 1.0));
    }
    if enemy.id == "treasure_imp" {
        return (8.0, Color::srgb(1.0, 0.82, 0.18));
    }
    if enemy.affixes.len() >= 2 {
        (7.8, Color::srgb(1.0, 0.36, 0.04))
    } else if !enemy.affixes.is_empty() {
        (6.8, Color::srgb(1.0, 0.52, 0.12))
    } else {
        (5.0, Color::srgb(0.92, 0.16, 0.10))
    }
}

pub(super) fn update_minimap_legend(
    maps: Query<&Interaction, With<MinimapUi>>,
    mut legends: Query<&mut Node, With<MinimapLegend>>,
) {
    let hovered = maps
        .iter()
        .any(|interaction| *interaction != Interaction::None);
    for mut legend in &mut legends {
        legend.display = if hovered {
            Display::Flex
        } else {
            Display::None
        };
    }
}

pub(super) fn minimap_legend_entries() -> [(MinimapLegendRole, &'static str); 6] {
    [
        (MinimapLegendRole::Player, "You"),
        (MinimapLegendRole::Main, "Main"),
        (MinimapLegendRole::Loot, "Loot"),
        (MinimapLegendRole::Life, "Life"),
        (MinimapLegendRole::Fury, "Fury"),
        (MinimapLegendRole::Threat, "Threat"),
    ]
}

pub(super) fn minimap_legend_color(role: MinimapLegendRole) -> Color {
    match role {
        MinimapLegendRole::Player => Color::srgb(0.20, 0.92, 0.46),
        MinimapLegendRole::Main => Color::srgb(0.15, 0.72, 1.0),
        MinimapLegendRole::Loot => Color::srgb(0.95, 0.72, 0.18),
        MinimapLegendRole::Life => Color::srgb(1.0, 0.08, 0.12),
        MinimapLegendRole::Fury => Color::srgb(0.18, 0.58, 1.0),
        MinimapLegendRole::Threat => Color::srgb(1.0, 0.36, 0.04),
    }
}

pub(super) fn spawn_minimap_dot(
    commands: &mut Commands,
    map_entity: Entity,
    world_position: Vec3,
    center: Vec3,
    size: f32,
    color: Color,
) {
    let position = minimap_position(world_position, center);
    let Ok(mut map_entity_commands) = commands.get_entity(map_entity) else {
        return;
    };
    map_entity_commands.with_children(|map| {
        map.spawn((
            Node {
                position_type: PositionType::Absolute,
                left: Val::Px(position.x - size * 0.5),
                top: Val::Px(position.y - size * 0.5),
                width: Val::Px(size),
                height: Val::Px(size),
                border_radius: BorderRadius::MAX,
                ..default()
            },
            BackgroundColor(color),
            Pickable::IGNORE,
            MinimapDot,
        ));
    });
}

pub(super) fn minimap_position(world_position: Vec3, center: Vec3) -> Vec2 {
    let x =
        (0.5 + (world_position.x - center.x) / (MINIMAP_VISIBLE_RADIUS_X * 2.0)).clamp(0.0, 1.0);
    let z =
        (0.5 + (world_position.z - center.z) / (MINIMAP_VISIBLE_RADIUS_Z * 2.0)).clamp(0.0, 1.0);
    Vec2::new(
        MINIMAP_MAP_LEFT + x * (MINIMAP_WIDTH - MINIMAP_MAP_LEFT - MINIMAP_MAP_RIGHT),
        MINIMAP_MAP_TOP + z * (MINIMAP_HEIGHT - MINIMAP_MAP_TOP - MINIMAP_MAP_BOTTOM),
    )
}

#[derive(Component)]
pub(super) struct ObjectiveArrow;

#[derive(Component)]
pub(super) struct MapCardinal(&'static str, &'static str);

// UI coordinates increase downward. The unrotated arrow points toward -Z/north.
fn arrow_rotation(bearing: Vec2) -> Rot2 {
    Rot2::radians(bearing.x.atan2(-bearing.y))
}

fn spawn_arrow(
    parent: &mut ChildSpawnerCommands,
    mut node: Node,
    bearing: Vec2,
    color: Color,
    objective: bool,
) {
    node.width = Val::Px(16.0);
    node.height = Val::Px(16.0);
    node.flex_shrink = 0.0;
    let mut arrow = parent.spawn((
        node,
        UiTransform::from_rotation(arrow_rotation(bearing)),
        Pickable::IGNORE,
    ));
    if objective {
        arrow.insert(ObjectiveArrow);
    } else {
        arrow.insert((MinimapDot, ZIndex(5)));
    }
    arrow.with_children(|arrow| {
        // Two metal-like bevels and a shaft form a genuine arrow without a font glyph.
        for (left, top, width, height, angle) in [
            (6.5, 5.0, 3.0, 10.0, 0.0),
            (3.0, 1.0, 3.0, 9.0, 0.70),
            (9.0, 1.0, 3.0, 9.0, -0.70),
        ] {
            arrow.spawn((
                Node {
                    position_type: PositionType::Absolute,
                    left: Val::Px(left),
                    top: Val::Px(top),
                    width: Val::Px(width),
                    height: Val::Px(height),
                    border_radius: BorderRadius::all(Val::Px(1.0)),
                    ..default()
                },
                UiTransform::from_rotation(Rot2::radians(angle)),
                BackgroundColor(color),
                Pickable::IGNORE,
            ));
        }
    });
}

pub(super) fn spawn_objective_arrow(parent: &mut ChildSpawnerCommands) {
    spawn_arrow(
        parent,
        Node::default(),
        Vec2::NEG_Y,
        Color::srgb(0.96, 0.78, 0.38),
        true,
    );
}

pub(super) fn spawn_map_directions(parent: &mut ChildSpawnerCommands) {
    let center = minimap_position(Vec3::ZERO, Vec3::ZERO);
    for (english, chinese, x, y) in [
        ("N", "北", center.x - 7.0, 24.0),
        ("E", "东", MINIMAP_WIDTH - 17.0, center.y - 7.0),
        ("S", "南", center.x - 7.0, MINIMAP_HEIGHT - 17.0),
        ("W", "西", 3.0, center.y - 7.0),
    ] {
        parent.spawn((
            Node {
                position_type: PositionType::Absolute,
                left: Val::Px(x),
                top: Val::Px(y),
                width: Val::Px(14.0),
                height: Val::Px(14.0),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                ..default()
            },
            Text::new(english),
            TextFont {
                font_size: FontSize::Px(10.0),
                ..default()
            },
            TextColor(Color::srgb(0.81, 0.73, 0.55)),
            MapCardinal(english, chinese),
            localization::NativeLanguageName,
            Pickable::IGNORE,
        ));
    }
}

pub(super) fn update_map_cardinals(
    language: Res<localization::UiLanguage>,
    mut labels: Query<(&mut Text, &MapCardinal)>,
) {
    for (mut text, cardinal) in &mut labels {
        let label = match *language {
            localization::UiLanguage::Chinese => cardinal.1,
            localization::UiLanguage::English => cardinal.0,
        };
        if text.0 != label {
            text.0 = label.to_owned();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn arrow_matches_north_up_map_in_all_four_directions() {
        for world in [Vec3::NEG_Z, Vec3::X, Vec3::Z, Vec3::NEG_X] {
            let map_delta = minimap_position(world * 10.0, Vec3::ZERO)
                - minimap_position(Vec3::ZERO, Vec3::ZERO);
            let tip = arrow_rotation(world.xz()) * Vec2::NEG_Y;
            assert!(tip.dot(map_delta.normalize()) > 0.999);
        }
    }

    #[test]
    fn player_arrow_uses_animation_facing_instead_of_unrotated_actor() {
        for direction in [
            Vec3::NEG_Z,
            Vec3::X,
            Vec3::Z,
            Vec3::NEG_X,
            Vec3::new(1.0, 0.0, -1.0).normalize(),
        ] {
            let mut animation = crate::player::PlayerAnimation::default();
            animation.face_direction(direction);
            let projected = minimap_position(animation.facing_direction(), Vec3::ZERO)
                - minimap_position(Vec3::ZERO, Vec3::ZERO);
            let tip = arrow_rotation(projected) * Vec2::NEG_Y;
            let expected = minimap_position(direction * 10.0, Vec3::ZERO)
                - minimap_position(Vec3::ZERO, Vec3::ZERO);
            assert!(tip.dot(expected.normalize()) > 0.999);
        }
    }
}
