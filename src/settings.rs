use std::fs;

use bevy::prelude::*;
use serde::Deserialize;

use crate::scenery::terrain::TerrainSettings;

const SETTINGS_FILE: &str = "settings.toml";

pub struct SettingsPlugin;
impl Plugin for SettingsPlugin {
    fn build(&self, app: &mut App) {
        // TODO: Add hot reloading and GUI
        app.insert_resource(Settings::fetch());
    }
}

#[derive(Resource, Deserialize)]
pub struct Settings {
    pub terrain: TerrainSettings,
    pub graphics: GraphicsSettings,
    pub input: InputSettings,
}

#[derive(Deserialize)]
pub struct GraphicsSettings {
    pub shadow_distance: f32,
    pub screen_resolution: u32,
    pub buildings_enabled: bool,
}

#[derive(Deserialize)]
pub struct InputSettings {
    pub gamepad: crate::input::GamepadSettings,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            terrain: TerrainSettings::default(),
            graphics: GraphicsSettings {
                screen_resolution: 1024,
                shadow_distance: 5000.0,
                buildings_enabled: false, // Buildings are heavy to compute
            },
            input: InputSettings {
                gamepad: crate::input::GamepadSettings::default(),
            },
        }
    }
}

impl Settings {
    fn fetch() -> Self {
        let contents = fs::read_to_string(SETTINGS_FILE).unwrap();
        let settings: Self = toml::from_str(&contents).unwrap();

        settings
    }
}
