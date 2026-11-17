pub mod airfoils;
pub mod animations;
pub mod avionics;
pub mod control_surfaces;
pub mod damage;
pub mod engine;
pub mod landing_gear;
pub mod lights;
pub mod rotors;
pub mod springs;

use bevy::prelude::*;
use big_space::grid::cell::CellCoord;

use crate::{
    SimState, absolute_position,
    aircraft::{
        avionics::controls::{fly_by_wire::FlyByWirePlugin, interfaces::InterfacePlugin},
        control_surfaces::ControlSurfacesDeflection,
        damage::DamagePlugin,
        engine::Engine,
        landing_gear::{LandingGear, LandingGearCommand, LandingGearStatus},
        lights::{Light, LightStates},
        rotors::update_rotors,
        springs::SpringsPlugin,
    },
    scenery::terrain::pos_to_coord,
    utils::{Coord, ToSides},
};

pub struct AircraftPlugin;
impl Plugin for AircraftPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((
            DamagePlugin,
            FlyByWirePlugin,
            InterfacePlugin,
            SpringsPlugin,
        ))
        .add_message::<LandingGearCommand>()
        .add_systems(Update, (update_rotors, screens::update_screens))
        .add_systems(
            FixedUpdate,
            (
                (Light::update_mesh_lights, Light::update_lights),
                Engine::update,
                LandingGear::operate_landing_gear.run_if(in_state(SimState::Running)),
                update_aircraft_coord,
            ),
        );
    }
}

pub fn alpha_deg(velocity: &Vec3, transform: &Transform) -> f32 {
    let velocity = velocity.normalize_or_zero();
    let forward = transform.local_x();
    let right = transform.local_y();

    let sin = forward.cross(velocity).dot(right.as_vec3());
    let cos = forward.dot(velocity);

    -sin.atan2(cos).to_degrees()
}

#[derive(Component, Clone, Reflect)]
#[reflect(Component)]
pub struct Aircraft {
    pub name: String,
    pub variant: String,
    pub coord: Coord,
    pub control_surfaces: ControlSurfacesDeflection,
    pub landing_gear: LandingGearStatus,
    pub on_ground: bool,
    pub systems: Systems,
}

#[derive(Clone, Reflect)]
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

fn update_aircraft_coord(mut aircraft: Single<(&CellCoord, &Transform, &mut Aircraft)>) {
    let (cell_coord, transform, aircraft) = &mut *aircraft;
    let reference_pos = absolute_position(cell_coord, transform.translation).as_vec3();

    aircraft.coord = pos_to_coord(reference_pos.normalize());
}
