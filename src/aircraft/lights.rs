use std::{collections::HashMap, fs};

use bevy::{
    ecs::{lifecycle::HookContext, world::DeferredWorld},
    prelude::*,
};
use serde::Deserialize;

use crate::{AppState, MaterialStore};

pub struct LightPlugin;
impl Plugin for LightPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            (
                Light::update_mesh_lights,
                Light::update_lights,
                Lights::load
                    .run_if(not(resource_exists::<Lights>))
                    .run_if(in_state(AppState::InGame)),
            ),
        );
    }
}

/// Add this component to objects in Blender to mark them as lights.
/// Their material will be overwritten.
#[derive(Debug, Component, Deserialize, PartialEq, Reflect, Default)]
#[reflect(Component, Default)]
#[component(on_add = on_add_use_light_material)]
pub struct Light {
    state: LightState,
    name: String,
    colour: LinearRgba,
    strength: f32,
}

impl Light {
    pub fn is_on(&self) -> bool {
        self.state == LightState::On
    }

    /// Returns the identifier of the light
    pub fn name(&self) -> String {
        self.name.clone()
    }

    pub fn toggle(&mut self) {
        self.state.toggle();
    }

    pub fn update_mesh_lights(
        material_handles: Query<(&MeshMaterial3d<StandardMaterial>, &Light)>,
        mut materials: ResMut<Assets<StandardMaterial>>,
    ) {
        for (material_handle, light) in material_handles.iter() {
            if let Some(mut material) = materials.get_mut(material_handle) {
                material.emissive = light.colour;
                material.emissive_exposure_weight = light.strength;
            }
        }
    }

    pub fn update_lights(
        point_light_query: Query<(&mut PointLight, &Light)>,
        spot_light_query: Query<(&mut SpotLight, &Light)>,
    ) {
        for (mut point_light, light) in point_light_query {
            let on = true;

            point_light.color = Color::LinearRgba(light.colour);

            if on {
                point_light.intensity = 10000.0;
            } else {
                point_light.intensity = 0.0
            }
        }
        for (mut spot_light, light) in spot_light_query {
            let on = true;

            spot_light.color = Color::LinearRgba(light.colour);

            if on {
                spot_light.intensity = light.strength;
            } else {
                spot_light.intensity = 0.0
            }
        }
    }
}

fn on_add_use_light_material(mut world: DeferredWorld, HookContext { entity, .. }: HookContext) {
    let Some(store) = world.get_resource::<MaterialStore>() else {
        return;
    };
    let template = store.lights.clone();

    let Some(mut materials) = world.get_resource_mut::<Assets<StandardMaterial>>() else {
        return;
    };
    let handle = materials.add(template);

    world
        .commands()
        .entity(entity)
        .insert(MeshMaterial3d(handle));
}

#[derive(Debug, Clone, Deserialize)]
pub struct LightPattern {
    pub pattern: Vec<LightStep>,
}

// TODO: needs to reset between games
#[derive(Resource)]
pub struct Lights {
    pub patterns: HashMap<String, LightPattern>,
}
impl Lights {
    pub fn load(mut commands: Commands) {
        // TODO: stop hardcoding aircraft
        let path = "addons/aircraft/breeze/lights.toml";
        let Ok(text) = fs::read_to_string(path) else {
            error!("Couldn't find light config: {path}");
            return;
        };

        let config = match toml::from_str::<LightConfig>(&text) {
            Ok(config) => config,
            Err(e) => {
                error!("Failed to parse toml from path {path}: {e}");
                return;
            }
        };
        let lights = Lights {
            patterns: config.lights,
        };
        commands.insert_resource(lights);
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct LightStep {
    pub state: LightState,
    pub secs: f32,
}

#[derive(Debug, Deserialize)]
pub struct LightConfig {
    pub lights: HashMap<String, LightPattern>,
}

#[derive(Debug, Clone, Copy, PartialEq, Deserialize, Reflect, Default)]
#[serde(rename_all = "lowercase")]
pub enum LightState {
    On,
    #[default]
    Off,
}
impl LightState {
    fn toggle(&mut self) {
        *self = match *self {
            Self::On => Self::Off,
            Self::Off => Self::On,
        }
    }
}

pub type LightStates = HashMap<&'static str, bool>;
