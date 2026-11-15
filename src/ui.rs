use crate::{
    AppState,
    ui::{
        hud::UiHud,
        menu::{ChosenAircraft, Menu},
        notifications::NotificationPlugin,
    },
};
use bevy::{feathers::FeathersPlugins, input_focus::InputFocus, prelude::*};

mod hud;
mod menu;
pub mod notifications;

pub const UI_SCALE: f32 = 1.0;
pub const TEXT_FONT: &'static str = "fonts/Mozilla_Text/MozillaText-VariableFont_wght.ttf";
pub const TITLE_FONT: &'static str =
    "fonts/Mozilla_Headline/MozillaHeadline-VariableFont_wdth,wght.ttf";

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
