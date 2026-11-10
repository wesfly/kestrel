use crate::{
    MaterialStore,
    aircraft::{Aircraft, landing_gear::LandingGearStatus},
};
use bevy::{
    ecs::{lifecycle::HookContext, world::DeferredWorld},
    prelude::*,
};
use serde::Deserialize;
use std::time::Duration;

pub const STROBE_OFF_DURATION: f32 = 1.0;
pub const STROBE_ON_DURATION: f32 = 0.1;
pub const ACOL_OFF_DURATION: f32 = 1.2;
pub const ACOL_ON_DURATION: f32 = 0.1;

#[derive(Debug, Component, Deserialize, PartialEq, Reflect, Default)]
#[reflect(Component, Default)]
#[type_path = "skein"]
#[component(on_add = on_add_use_light_material)]
pub struct UseLightMaterial;

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

#[derive(Clone, PartialEq, Deserialize, Debug, Component, Reflect)]
#[reflect(Component)]
#[type_path = "skein"]
pub enum Light {
    CockpitGearInd,
    AntiCol,
    Strobe,
    PositionPort,
    PositionStarboard,
    PositionRear,
    Formation,
    Landing,
    Custom(String),
}

impl Light {
    pub fn is_on(&self) {}
}

impl Light {
    pub fn update_mesh_lights(
        aircraft: Single<&Aircraft>,
        material_handles: Query<(&MeshMaterial3d<StandardMaterial>, &Light, Entity)>,
        mut materials: ResMut<Assets<StandardMaterial>>,
        timer: ResMut<LightsTimers>,
    ) {
        for material_handle in material_handles.iter() {
            if let Some(mut material) = materials.get_mut(material_handle.0) {
                let LinearRgba {
                    ref mut red,
                    ref mut green,
                    ref mut blue,
                    alpha: _,
                } = material.emissive;

                match material_handle.1 {
                    Light::AntiCol => {
                        if aircraft.lights.anti_col && timer.acol_on_cycle {
                            *red = 100.0
                        } else {
                            *red = 0.0
                        }
                    }
                    Light::PositionPort => {
                        if aircraft.lights.pos {
                            *red = 100.
                        } else {
                            *red = 0.0
                        }
                    }
                    Light::PositionStarboard => {
                        if aircraft.lights.pos {
                            *green = 100.
                        } else {
                            *green = 0.0
                        }
                    }
                    Light::PositionRear => {
                        if aircraft.lights.pos {
                            *red = 100.;
                            *green = 100.;
                            *blue = 100.
                        } else {
                            *red = 0.;
                            *green = 0.;
                            *blue = 0.
                        }
                    }
                    Light::Strobe => {
                        if aircraft.lights.strobe && timer.strobe_on_cycle {
                            *red = 100.;
                            *green = 100.;
                            *blue = 100.
                        } else {
                            *red = 0.;
                            *green = 0.;
                            *blue = 0.
                        }
                    }
                    Light::Formation => {
                        if aircraft.lights.form {
                            *red = 10.;
                            *green = 20.;
                            *blue = 0.
                        } else {
                            *red = 0.;
                            *green = 0.;
                            *blue = 0.
                        }
                    }
                    Light::Landing => {
                        if aircraft.lights.landing
                            && aircraft.landing_gear == LandingGearStatus::Deployed
                        {
                            *red = 500.;
                            *green = 500.;
                            *blue = 500.
                        } else {
                            *red = 0.;
                            *green = 0.;
                            *blue = 0.
                        }
                    }
                    Light::CockpitGearInd => {
                        if aircraft.landing_gear == LandingGearStatus::Deployed {
                            *green = 50.0
                        } else {
                            *red = 0.0;
                            *green = 0.0;
                            *blue = 0.0;
                        }
                    }
                }
            }
        }
    }

    pub fn update_lights(
        aircraft: Single<&Aircraft>,
        timer: ResMut<LightsTimers>,
        point_light_query: Query<(&mut PointLight, &Light)>,
        spot_light_query: Query<(&mut SpotLight, &Light)>,
    ) {
        for (mut point_light, light) in point_light_query {
            let (colour, on) = match light {
                Light::PositionPort => (Color::linear_rgb(10.0, 0.0, 0.0), aircraft.lights.pos),
                Light::PositionStarboard => {
                    (Color::linear_rgb(0.0, 10.0, 0.0), aircraft.lights.pos)
                }
                Light::PositionRear => (Color::linear_rgb(1.0, 1.0, 1.0), aircraft.lights.pos),
                Light::AntiCol => (
                    Color::linear_rgb(1.0, 0.0, 0.0),
                    (aircraft.lights.anti_col && timer.acol_on_cycle),
                ),
                Light::Strobe => (
                    Color::linear_rgb(1.0, 1.0, 1.0),
                    (aircraft.lights.strobe && timer.strobe_on_cycle),
                ),

                _ => (Color::linear_rgb(1.0, 1.0, 1.0), false),
            };

            point_light.color = colour;

            if on {
                point_light.intensity = 10000.0;
            } else {
                point_light.intensity = 0.0
            }
        }
        for (mut spot_light, light) in spot_light_query {
            let (colour, on) = match light {
                Light::Landing => (
                    Color::linear_rgb(1.0, 1.0, 1.0),
                    aircraft.lights.landing && aircraft.landing_gear == LandingGearStatus::Deployed,
                ),
                _ => (Color::BLACK, false),
            };

            spot_light.color = colour;

            if on {
                spot_light.intensity = 100000000.0;
            } else {
                spot_light.intensity = 0.0
            }
        }
    }
}
