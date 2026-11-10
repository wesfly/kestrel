pub mod acceleration;
pub mod airfoils;
pub mod animations;
pub mod avionics;
pub mod buttons;
pub mod control_surfaces;
pub mod damage;
pub mod engine;
pub mod landing_gear;
pub mod lights;
pub mod screens;
pub mod springs;

use bevy::prelude::*;
use serde::Deserialize;

use crate::aircraft::landing_gear::LandingGearStatus;

pub fn alpha_deg(velocity: &Vec3, transform: &Transform) -> f32 {
    let velocity = velocity.normalize_or_zero();
    let forward = transform.local_x();
    let right = transform.local_y();

    let sin = forward.cross(velocity).dot(right.as_vec3());
    let cos = forward.dot(velocity);

    -sin.atan2(cos).to_degrees()
}

const SPAWN_ALTITUDE: f64 = 3000.0;

#[derive(Component, Default, Reflect)]
#[reflect(Default, Component)]
#[type_path = "skein"]
pub enum RotorTypes {
    Main(Dir3),
    #[default]
    Rear,
}

#[derive(Resource, Default, Deserialize, PartialEq, Clone, Copy)]
pub enum AircraftTypes {
    Helicopter,
    J3Cub,
    #[default]
    Breeze,
}

#[derive(Resource, Copy, Clone)]
pub struct EngineState {
    on: bool,
}

/// Angles in radians
#[derive(Default, Resource, Copy, Clone)]
pub struct ControlSurfacesDeflection {
    canards: Sides<f32>,
    aileron: Sides<f32>,
    elevator: Sides<f32>,
    rudder: f32,
    ground_brakes: f32, // 0 to 1
}

#[derive(Default, Resource, Copy, Clone)]
pub struct LightState {
    pub pos: bool,
    pub strobe: bool,
    pub form: bool,
    pub landing: bool,
    pub anti_col: bool,
}

#[derive(Component, Clone)]
pub struct Aircraft {
    pub name: String,
    pub variant: String,
    pub control_surfaces: ControlSurfacesDeflection,
    pub lights: LightState,
    pub aircraft_type: AircraftTypes,
    pub landing_gear: LandingGearStatus,
    pub parking_brake: bool,
    pub on_ground: bool,
    pub engine: EngineState,
}

impl Default for Aircraft {
    fn default() -> Self {
        Aircraft {
            name: String::new(),
            variant: String::new(),
            control_surfaces: ControlSurfacesDeflection {
                canards: 0.0_f32.left_right(),
                aileron: 0.0_f32.left_right(),
                elevator: 0.0_f32.left_right(),
                rudder: 0.0,

                ground_brakes: 0.0,
            },
            engine: EngineState { on: false },
            aircraft_type: AircraftTypes::Helicopter,
            lights: LightState::default(),
            landing_gear: LandingGearStatus::Retracted,
            parking_brake: false,
            on_ground: false,
        }
    }
}

#[derive(Default, Clone, Copy, Debug, Reflect)]
pub struct Sides<T> {
    pub left: T,
    pub right: T,
}

pub trait ToSides {
    fn left_right(self) -> Sides<f32>;
}
impl ToSides for f32 {
    fn left_right(self) -> Sides<f32> {
        Sides {
            left: self,
            right: self,
        }
    }
}
