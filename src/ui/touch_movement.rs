//! A floating movement pad only exists while a world touch is held.
//! HUD taps retain their existing skill/item actions.
use super::*;

#[derive(Resource, Default)]
pub(crate) struct TouchMovement {
    pub axis: Vec3,
    finger: Option<u64>,
    origin: Vec2,
}
#[derive(Component)]
struct Pad;
#[derive(Component)]
struct Thumb;

pub(super) fn install(app: &mut App) {
    app.init_resource::<TouchMovement>()
        .add_systems(Startup, spawn)
        .add_systems(
            PreUpdate,
            update
                .after(super::update_ui_pointer_capture)
                .after(super::hud_layout::press_console_slots),
        );
}
fn spawn(mut commands: Commands) {
    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                width: Val::Px(104.),
                height: Val::Px(104.),
                border: UiRect::all(Val::Px(2.)),
                border_radius: BorderRadius::all(Val::Percent(50.)),
                display: Display::None,
                ..default()
            },
            Pad,
            Pickable::IGNORE,
            GlobalZIndex(80),
            BackgroundColor(Color::srgba(0.04, 0.035, 0.025, 0.42)),
            BorderColor::all(Color::srgba(0.85, 0.67, 0.36, 0.65)),
        ))
        .with_child((
            Node {
                position_type: PositionType::Absolute,
                width: Val::Px(36.),
                height: Val::Px(36.),
                border_radius: BorderRadius::all(Val::Percent(50.)),
                ..default()
            },
            Thumb,
            Pickable::IGNORE,
            BackgroundColor(Color::srgba(0.85, 0.67, 0.36, 0.8)),
        ));
}
fn pad_axis(delta: Vec2) -> Vec2 {
    if delta.length() < 8. {
        Vec2::ZERO
    } else {
        (delta / 44.).clamp_length_max(1.)
    }
}
#[allow(clippy::too_many_arguments)]
fn update(
    touches: Res<Touches>,
    state: Res<State<GameState>>,
    pause: Res<crate::PauseState>,
    settings: Res<super::settings::SettingsPanelState>,
    inventory: Res<InventoryOpen>,
    build: Res<BuildOpen>,
    journal: Res<JournalOpen>,
    capture: Res<UiPointerCapture>,
    viewport: Res<HudViewport>,
    windows: Query<&Window, With<PrimaryWindow>>,
    cameras: Query<&GlobalTransform, With<Camera3d>>,
    mut movement: ResMut<TouchMovement>,
    mut mouse: ResMut<ButtonInput<MouseButton>>,
    mut pads: Query<&mut Node, (With<Pad>, Without<Thumb>)>,
    mut thumbs: Query<&mut Node, (With<Thumb>, Without<Pad>)>,
) {
    let was_active = movement.finger.is_some();
    movement.axis = Vec3::ZERO;
    let Ok(window) = windows.single() else {
        movement.finger = None;
        return;
    };
    let blocked = *state.get() != GameState::InGame
        || pause.paused
        || settings.open
        || inventory.open
        || build.open
        || journal.open
        || !window.focused;
    if blocked {
        movement.finger = None;
    }
    if movement.finger.is_none() && !blocked && !capture.0 {
        let bottom = window.height() - super::hud_layout::console_top(*viewport, window.width());
        if let Some(touch) = touches.iter_just_pressed().find(|touch| {
            let p = touch.position();
            p.x < window.width() * 0.45 && p.y > 80. && p.y < bottom
        }) {
            movement.finger = Some(touch.id());
            movement.origin = touch.position();
        }
    }
    let position = movement
        .finger
        .and_then(|id| touches.get_pressed(id))
        .map(|t| t.position());
    if position.is_none() {
        movement.finger = None;
    }
    let delta = position.map_or(Vec2::ZERO, |p| p - movement.origin);
    let axis = pad_axis(delta);
    if let Ok(camera) = cameras.single() {
        let right = camera.right().as_vec3().with_y(0.).normalize_or_zero();
        let forward = camera.forward().as_vec3().with_y(0.).normalize_or_zero();
        movement.axis = right * axis.x - forward * axis.y;
    }
    if was_active || movement.finger.is_some() {
        // Touch-to-mouse emulation must never turn a movement drag/release into a strike.
        mouse.reset(MouseButton::Left);
        mouse.reset(MouseButton::Right);
    }
    for mut node in &mut pads {
        node.display = if movement.finger.is_some() {
            Display::Flex
        } else {
            Display::None
        };
        node.left = Val::Px(movement.origin.x - 52.);
        node.top = Val::Px(movement.origin.y - 52.);
    }
    let thumb = delta.clamp_length_max(34.);
    for mut node in &mut thumbs {
        node.left = Val::Px(32. + thumb.x);
        node.top = Val::Px(32. + thumb.y);
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn movement_deadzone_and_reach_are_bounded() {
        assert_eq!(pad_axis(Vec2::new(3., 4.)), Vec2::ZERO);
        assert_eq!(pad_axis(Vec2::new(22., 0.)), Vec2::new(0.5, 0.));
        assert!((pad_axis(Vec2::splat(100.)).length() - 1.).abs() < 0.0001);
    }
}
