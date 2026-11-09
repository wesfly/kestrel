use crate::aircraft::avionics::buttons::AircraftButton;
use bevy::{
    picking::{hover::HoverMap, pointer::PointerId},
    prelude::*,
    ui_widgets::Button,
    window::{CursorIcon, PrimaryWindow, SystemCursorIcon},
};

type Clickable = Or<(With<Button>, With<AircraftButton>)>;

pub struct CursorPlugin;
impl Plugin for CursorPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, update_cursor);
    }
}

pub fn update_cursor(
    hover_map: Res<HoverMap>,
    mouse: Res<ButtonInput<MouseButton>>,
    clickable: Query<Has<AircraftButton>, Clickable>,
    parents: Query<&ChildOf>,
    window: Single<(Entity, Option<&CursorIcon>), With<PrimaryWindow>>,
    mut commands: Commands,
) {
    let pressed = mouse.pressed(MouseButton::Left);

    let icon = hover_map
        .get(&PointerId::Mouse)
        .into_iter()
        .flat_map(|hovered| hovered.keys().copied())
        .find_map(|entity| {
            std::iter::once(entity)
                .chain(parents.iter_ancestors(entity))
                .find_map(|e| clickable.get(e).ok())
        })
        .map(|is_cockpit_control| {
            if is_cockpit_control && pressed {
                SystemCursorIcon::Grab
            } else {
                SystemCursorIcon::Pointer
            }
        })
        .unwrap_or(SystemCursorIcon::Default);

    let desired = CursorIcon::System(icon);
    let (window, current) = *window;

    if current != Some(&desired) {
        commands.entity(window).insert(desired);
    }
}
