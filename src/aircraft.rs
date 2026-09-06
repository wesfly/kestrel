pub mod animations;
pub mod buttons;
pub mod lights;
pub mod screens;

pub mod airfoils;
pub mod breeze;
mod helicopter;
mod j3cub;

use crate::{
    EARTH_RADIUS, GameState, Settings, aircraft,
    camera::AircraftCamera,
    input::ControlInputs,
    scenery::terrain::{coord_to_pos, coord_to_world_pos},
};
use avian3d::prelude::*;
use bevy::{
    math::{DMat3, DQuat, DVec3},
    prelude::*,
};
use big_space::prelude::*;
use core::f64::consts::FRAC_PI_2;
use serde::Deserialize;

pub fn alpha_deg(velocity: &DVec3, transform: &Transform) -> f64 {
    let velocity = velocity.normalize_or_zero();
    let forward = transform.local_x();
    let right = transform.local_y();

    let sin = forward.cross(velocity.as_vec3()).dot(right.as_vec3());
    let cos = forward.dot(velocity.as_vec3());

    -sin.atan2(cos).to_degrees() as f64
}

#[derive(Component)]
pub struct Aircraft;

const SPAWN_ALTITUDE: f64 = 3000.0;

#[derive(Component, Default, Reflect)]
#[reflect(Default, Component)]
#[type_path = "skein"]
pub enum RotorTypes {
    Main(Dir3),
    #[default]
    Rear,
}

#[derive(Debug, Deserialize, Component, Reflect)]
#[reflect(Component)]
#[type_path = "skein"]
pub enum ControlSurfaces {
    CanardPort,
    CanardStarboard,
    Rudder,
    ElevatorPort,
    ElevatorStarboard,
    FlapPort,
    FlapStarboard,
    AileronPort,
    AileronStarboard,
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
    canards: BothSides<f32>,
    aileron: BothSides<f32>,
    elevator: BothSides<f32>,
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

#[derive(Resource, Copy, Clone)]
pub struct AircraftState {
    pub control_surfaces: ControlSurfacesDeflection,
    pub lights: LightState,
    pub aircraft_type: AircraftTypes,
    pub landing_gear_deployed: bool,
    pub parking_brake: bool,
    pub on_ground: bool,
    pub engine: EngineState,
}

impl Default for AircraftState {
    fn default() -> Self {
        AircraftState {
            control_surfaces: ControlSurfacesDeflection {
                canards: 0.0_f32.both_sides(),
                aileron: 0.0_f32.both_sides(),
                elevator: 0.0_f32.both_sides(),
                rudder: 0.0,

                ground_brakes: 0.0,
            },
            engine: EngineState { on: false },
            aircraft_type: AircraftTypes::Helicopter,
            lights: LightState::default(),
            landing_gear_deployed: false,
            parking_brake: false,
            on_ground: false,
        }
    }
}

pub fn main(
    input: Res<ControlInputs>,
    mut state: ResMut<AircraftState>,
    gizmos: Gizmos,
    mut aircraft: Single<
        (
            &Position,
            &Transform,
            Forces,
            Option<&mut avian_fdm::prelude::ControlInputs>,
        ),
        With<Aircraft>,
    >,
    spatial_query: SpatialQuery,
    game_state: Res<GameState>,
) {
    if !game_state.running {
        return;
    }

    match state.aircraft_type {
        AircraftTypes::Helicopter => {
            helicopter::mechanics(input, state, gizmos, &mut aircraft);
        }
        AircraftTypes::J3Cub => {
            // TODO
            aircraft::breeze::mechanics::mechanics(input, &mut state, &mut aircraft);
        }
        AircraftTypes::Breeze => {
            aircraft::breeze::mechanics::mechanics(input, &mut state, &mut aircraft);
            aircraft::breeze::landing_gear::spring_forces(spatial_query, aircraft, state);
        }
    }
}

pub fn spawn_breeze(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut state: ResMut<AircraftState>,
    root_grid: Single<(Entity, &Grid), With<BigSpace>>,
    settings: Res<Settings>,
) {
    let (root_grid_id, grid) = *root_grid;

    *state = AircraftState::default(); // Reset aircraft state
    state.aircraft_type = AircraftTypes::Breeze;
    state.engine.on = true;
    state.lights.landing = true;

    let normalized_pos = coord_to_pos(settings.terrain.coord);

    let altitude = SPAWN_ALTITUDE;
    let translation =
        coord_to_world_pos(settings.terrain.coord) + normalized_pos.as_dvec3() * altitude;
    let (object_cell, object_pos) = grid.translation_to_grid(translation);
    let surface_up = translation.normalize();
    let level = DQuat::from_rotation_arc(DVec3::Y, surface_up) * DQuat::from_rotation_x(FRAC_PI_2);

    let up = surface_up;
    let reference_forward = DVec3::NEG_Z;
    let forward = (reference_forward - up * reference_forward.dot(up)).normalize();
    let right = forward.cross(up).normalize();
    let up = right.cross(forward);

    let rotation = DQuat::from_mat3(&DMat3::from_cols(right, up, -forward));
    commands.spawn((
        AircraftCamera::spawn(),
        ChildOf(root_grid_id),
        object_cell,
        Transform::from_translation(object_pos).with_rotation(rotation.as_quat()),
    ));

    breeze::spawn(
        &mut commands,
        Transform::from_translation(object_pos).with_rotation(level.as_quat()),
        asset_server,
        Rotation(level),
        100.0,
        object_cell,
        root_grid_id,
        translation,
    );
}

pub fn spawn_j3cub(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut state: ResMut<AircraftState>,
    root_grid: Single<(Entity, &Grid), With<BigSpace>>,
    settings: Res<Settings>,
) {
    *state = AircraftState::default(); // Reset aircraft state
    state.aircraft_type = AircraftTypes::J3Cub;
    state.engine.on = true;

    let (root_grid_id, grid) = *root_grid;

    let normalized_pos = coord_to_pos(settings.terrain.coord);
    let altitude = SPAWN_ALTITUDE;
    let translation =
        normalized_pos.as_dvec3() * EARTH_RADIUS as f64 + normalized_pos.as_dvec3() * altitude;
    let (object_cell, object_pos) = grid.translation_to_grid(translation);
    let surface_up = translation.normalize();
    let level = DQuat::from_rotation_arc(DVec3::Y, surface_up) * DQuat::from_rotation_x(FRAC_PI_2);

    let up = surface_up;
    let reference_forward = DVec3::NEG_Z;
    let forward = (reference_forward - up * reference_forward.dot(up)).normalize();
    let right = forward.cross(up).normalize();
    let up = right.cross(forward);

    let rotation = DQuat::from_mat3(&DMat3::from_cols(right, up, -forward));
    commands.spawn((
        AircraftCamera::spawn(),
        ChildOf(root_grid_id),
        object_cell,
        Transform::from_translation(object_pos).with_rotation(rotation.as_quat()),
    ));
    j3cub::spawn(
        &mut commands,
        Transform::from_translation(object_pos).with_rotation(level.as_quat()),
        asset_server,
        translation,
        30.0,
        object_cell,
        root_grid_id,
        Rotation(level),
    );
}

pub fn spawn_helicopter(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut state: ResMut<AircraftState>,
    root_grid: Single<(Entity, &Grid), With<BigSpace>>,
    settings: Res<Settings>,
) {
    *state = AircraftState::default(); // Reset aircraft state
    state.aircraft_type = AircraftTypes::Helicopter;
    let (root_grid_id, grid) = *root_grid;

    let normalized_pos = coord_to_pos(settings.terrain.coord);
    let altitude = SPAWN_ALTITUDE;
    let translation =
        normalized_pos.as_dvec3() * EARTH_RADIUS as f64 + normalized_pos.as_dvec3() * altitude;
    let (object_cell, object_pos) = grid.translation_to_grid(translation);
    let surface_up = translation.normalize();

    let up = surface_up;
    let reference_forward = DVec3::NEG_Z;
    let forward = (reference_forward - up * reference_forward.dot(up)).normalize();
    let right = forward.cross(up).normalize();
    let up = right.cross(forward);

    let level = DQuat::from_rotation_arc(DVec3::Y, up) * DQuat::from_rotation_y(-FRAC_PI_2);
    commands.spawn((
        AircraftCamera::spawn(),
        ChildOf(root_grid_id),
        object_cell,
        Transform::from_translation(object_pos),
    ));
    helicopter::spawn(
        &mut commands,
        &asset_server,
        level,
        object_pos,
        object_cell,
        root_grid_id,
    );
}

#[derive(Default, Clone, Copy, Debug)]
pub struct BothSides<T> {
    pub port: T,
    pub starboard: T,
}

pub trait BothSidesF32 {
    fn both_sides(self) -> BothSides<f32>;
}
impl BothSidesF32 for f32 {
    fn both_sides(self) -> BothSides<f32> {
        BothSides {
            port: self,
            starboard: self,
        }
    }
}
