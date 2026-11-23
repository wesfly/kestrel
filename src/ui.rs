use crate::{
    AppState,
    ui::{hud::UiHud, menu::Menu, notifications::NotificationPlugin},
};
use bevy::{feathers::FeathersPlugins, input_focus::InputFocus, prelude::*};

mod fonts;
mod hud;
mod menu;
pub mod notifications;

pub use menu::{ChosenAircraft, LocationSelector};

pub const UI_SCALE: f32 = 1.0;

pub struct UiPlugin;
impl Plugin for UiPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<InputFocus>()
            .insert_resource(ChosenAircraft(None))
            .add_plugins(FeathersPlugins)
            .add_plugins(NotificationPlugin)
            .add_systems(OnEnter(AppState::Menu), Menu::spawn)
            .add_systems(OnEnter(AppState::InGame), UiHud::spawn)
            .add_systems(Update, UiHud::update.run_if(in_state(AppState::InGame)));
    }
}
