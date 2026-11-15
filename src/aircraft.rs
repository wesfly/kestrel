pub mod airfoils;
pub mod animations;
pub mod avionics;
pub mod control_surfaces;
pub mod damage;
pub mod engine;
pub mod landing_gear;
pub mod lights;
pub mod rotors;
pub mod screens;
pub mod springs;

use bevy::prelude::*;
use big_space::grid::cell::CellCoord;

use crate::{
    absolute_position,
    scenery::terrain::{Coord, pos_to_coord},
    aircraft::{
        control_surfaces::ControlSurfacesDeflection, landing_gear::LandingGearStatus,
        lights::LightStates,
    },
};

pub fn alpha_deg(velocity: &Vec3, transform: &Transform) -> f32 {
    let velocity = velocity.normalize_or_zero();
    let forward = transform.local_x();
    let right = transform.local_y();

    let sin = forward.cross(velocity).dot(right.as_vec3());
    let cos = forward.dot(velocity);

    -sin.atan2(cos).to_degrees()
}

#[derive(Component, Clone)]
pub struct Aircraft {
    pub name: String,
    pub variant: String,
    pub coord: Coord,
    pub control_surfaces: ControlSurfacesDeflection,
    pub landing_gear: LandingGearStatus,
    pub on_ground: bool,
    pub systems: Systems,
}

#[derive(Clone)]
pub struct Systems {
    pub parking_brake: bool,
    /// 0.0 to 1.0
    pub wheel_brake: f32,
    pub lights: LightStates,
}

impl Default for Aircraft {
    fn default() -> Self {
        Aircraft {
            name: String::new(),
            variant: String::new(),
            coord: Coord::default(),
            control_surfaces: ControlSurfacesDeflection {
                canards: 0.0_f32.left_right(),
                aileron: 0.0_f32.left_right(),
                elevator: 0.0_f32.left_right(),
                rudder: 0.0,
            },
            landing_gear: LandingGearStatus::Retracted,
            on_ground: false,
            systems: Systems {
                parking_brake: false,
                wheel_brake: 0.0,
                lights: LightStates::default(),
            },
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

fn update_aircraft_coord(mut aircraft: Single<(&CellCoord, &Transform, &mut Aircraft)>) {
    let (cell_coord, transform, aircraft) = &mut *aircraft;
    let reference_pos = absolute_position(cell_coord, transform.translation).as_vec3();

    aircraft.coord = pos_to_coord(reference_pos.normalize());
}
