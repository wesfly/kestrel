use std::{collections::HashMap, fs};

use bevy::{
    ecs::{lifecycle::HookContext, world::DeferredWorld},
    prelude::*,
};
use serde::Deserialize;

use crate::MaterialStore;

use super::Aircraft;

/// This constant reduces the number users have to put into the fields in
/// Blender because they can get high really fast.
const LIGHT_INTENSITY_FACTOR: f32 = 1e9;

pub struct LightPlugin;
impl Plugin for LightPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, (Light::update_mesh_lights, Light::update_lights))
            .add_systems(Update, Lights::load);
    }
}

/// Add this component to objects in Blender to mark them as lights.
/// Their material will be overwritten by `on_add_use_light_material`.
#[derive(Debug, Component, Deserialize, PartialEq, Reflect, Default)]
#[reflect(Component, Default)]
#[component(on_add = on_add_use_light_material)]
pub struct Light {
    state: LightState,
    name: String,
    colour: LinearRgba,
    intensity: f32,
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
        light_materials: Query<(&MeshMaterial3d<StandardMaterial>, &Light)>,
        lights: Single<&Lights>,
        mut materials: ResMut<Assets<StandardMaterial>>,
        time: Res<Time>,
    ) {
        // TODO: Make lights actually flash
        for (material_handle, light) in light_materials.iter() {
            let Some(pattern) = &lights.patterns.get(&light.name) else {
                error!("Light {light:?} not registered in lights.toml");
                return;
            };

            let strength = match pattern {
                PatternType::Steady => 1.0,
                PatternType::Pattern(p) => {
                    let total = p.get_total_secs();
                    let cycle_time = time.elapsed_secs() % total;

                    let active = p
                        .pattern
                        .iter()
                        .scan(0.0_f32, |end, step| {
                            *end += step.secs;
                            Some((*end, step))
                        })
                        .find(|(end, _)| *end > cycle_time)
                        .map(|(_, step)| step)
                        .or(p.pattern.last());

                    active.map_or(0.0, |step| match step.state {
                        LightState::On => 1.0,
                        LightState::Off => 0.0,
                    })
                }
            };

            if let Some(mut material) = materials.get_mut(material_handle) {
                material.emissive =
                    light.colour * LIGHT_INTENSITY_FACTOR * light.intensity * strength;
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
                point_light.intensity = light.intensity;
            } else {
                point_light.intensity = 0.0
            }
        }
        for (mut spot_light, light) in spot_light_query {
            let on = true;

            spot_light.color = Color::LinearRgba(light.colour);

            if on {
                spot_light.intensity = light.intensity;
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

/// The light pattern, describing the flashing of a light. It's not defined
/// per-component because changing the pattern would be a headache. Instead
/// it's defined in the `lights.toml` file in every aircraft folder.
///
/// ## Example
///
/// ```toml
/// [lights.strobe]
/// pattern = [
///   { state = "on", secs = 0.1 },
///   { state = "off", secs = 1.0 },
///   { state = "on", secs = 0.1 },
///   { state = "off", secs = 0.1 },
/// ]
/// ```
#[derive(Debug, Clone, Deserialize)]
pub struct LightPattern {
    pub pattern: Vec<LightStep>,
}

impl LightPattern {
    /// Yields the total duration of a light pattern cycle.
    fn get_total_secs(&self) -> f32 {
        self.pattern.iter().map(|f| f.secs).sum()
    }
}

/// This is the struct to parse `LightSteps` from `lights.toml`. It has an
/// optional field which makes it possible to create `Steady` lights that
/// don't flash. If the light should be steady, just reference it like this:
/// ```toml
/// [lights.strobe]
/// ```
#[derive(Debug, Clone, Deserialize)]
pub struct LightEntry {
    pub pattern: Option<Vec<LightStep>>,
}

#[derive(Debug, Clone, Deserialize)]
pub enum PatternType {
    Steady,
    Pattern(LightPattern),
}

/// The per-aircraft component holding all lights, their names and their patterns.
#[derive(Component)]
pub struct Lights {
    pub patterns: HashMap<String, PatternType>,
}
impl Lights {
    pub fn load(mut commands: Commands, aircraft: Single<(Entity, &Aircraft), Added<Aircraft>>) {
        let (entity, aircraft) = *aircraft;

        let path = format!("addons/aircraft/{}/lights.toml", aircraft.name);
        let Ok(text) = fs::read_to_string(&path) else {
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
            patterns: config
                .lights
                .into_iter()
                .map(|(name, entry)| {
                    let pattern = match entry.pattern {
                        Some(steps) => PatternType::Pattern(LightPattern { pattern: steps }),
                        None => PatternType::Steady,
                    };
                    (name, pattern)
                })
                .collect(),
        };
        info!("inserting lights");
        commands.entity(entity).insert(lights);
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct LightStep {
    pub state: LightState,
    pub secs: f32,
}

#[derive(Debug, Deserialize)]
pub struct LightConfig {
    pub lights: HashMap<String, LightEntry>,
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
