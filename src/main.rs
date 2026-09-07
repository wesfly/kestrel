/*
I made a little flight simulator here. Check out the README for further information.
If you have fixes or want to contribute, just make a pull request.

Spatial units are in metres
Default for speed is m/s
*/

mod aircraft;
mod camera;
mod input;
mod scenery;
mod sse;
mod ui;

use crate::{
    aircraft::{
        Aircraft, AircraftState,
        animations::{update_control_surfaces, update_rotors},
        breeze::landing_gear::{LandingGear, LandingGearCommand, LandingGearStatus},
        buttons::{Button, ButtonMessages},
        lights::{ACOL_OFF_DURATION, Light, LightsTimers, STROBE_OFF_DURATION},
    },
    camera::{AircraftCamera, CameraPosition, CameraSettings, rotate_sun},
    input::{ControlInputs, Gamepad},
    scenery::{
        osm::buildings::BuildingMaterial,
        terrain::{self, TerrainCacheResource, TerrainMaterial, TerrainSettings},
    },
    sse::Sse,
    ui::{Menu, UI},
};
use avian_fdm::prelude::*;
use avian3d::prelude::*;
use bevy::{
    dev_tools::diagnostics_overlay::DiagnosticsOverlayPlugin,
    diagnostic::FrameTimeDiagnosticsPlugin,
    ecs::system::SystemId,
    feathers::{dark_theme::create_dark_theme, theme::UiTheme},
    math::DVec3,
    pbr::ExtendedMaterial,
    prelude::*,
    render::view::screenshot::{Capturing, Screenshot, save_to_disk},
    window::{CursorIcon, SystemCursorIcon},
};
use bevy_skein::SkeinPlugin;
use big_space::prelude::*;
use core::option::Option::Some;
use once_cell::sync::Lazy;
use reqwest::Client;
use serde::Deserialize;
use std::{
    sync::Arc,
    {collections::HashMap, fs},
};
use tokio::{runtime::Runtime, sync::Semaphore};

#[cfg(debug_assertions)]
use bevy::dev_tools::diagnostics_overlay::DiagnosticsOverlay;

pub const METRES_TO_FEET: f32 = 3.28084;
pub const M_S_TO_KTS: f32 = 1.943844;
pub const EARTH_RADIUS: f32 = 6_360_000.0;

#[derive(Resource)]
pub struct MaterialStore {
    lights: StandardMaterial,
}

pub static TOKIO_RUNTIME: Lazy<Runtime> = Lazy::new(|| {
    #[allow(clippy::expect_used)]
    Runtime::new().expect("runtime to be created without errors. If you see this error, try again.")
});

pub fn bevy_to_aerospace_coords() -> Quat {
    Quat::from_mat3(&Mat3::from_cols(Vec3::Y, -Vec3::Z, -Vec3::X))
}

#[derive(Resource, Deserialize)]
pub struct Settings {
    gamepad: input::Gamepad,
    shadow_distance: f32,
    buildings_enabled: bool,
    terrain: TerrainSettings,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            gamepad: Gamepad::default(),
            shadow_distance: 5000.0,
            buildings_enabled: false, // Buildings are heavy to compute
            terrain: TerrainSettings::default(),
        }
    }
}

impl Settings {
    fn fetch() -> Self {
        let json_data = match fs::read_to_string("settings.json") {
            Ok(data) => data,
            Err(e) => {
                error!(
                    "Couldn't read settings.json ({e}). Try running 'cargo run (--release)' from the project root folder."
                );
                return Self::default();
            }
        };
        let settings: Self = match serde_json::from_str(&json_data) {
            Ok(settings) => settings,
            Err(e) => {
                error!("Invalid settings.json: {e}",);
                Self::default()
            }
        };
        settings
    }
}

#[derive(Resource)]
pub struct GameState {
    pub running: bool,
}

#[derive(Resource)]
pub struct RunOnceSystemList(HashMap<String, SystemId>);

impl FromWorld for RunOnceSystemList {
    fn from_world(world: &mut World) -> Self {
        let mut run_once_systems = RunOnceSystemList(HashMap::new());
        run_once_systems.0.insert(
            "setup_scene".into(),
            world.register_system(scenery::setup_scene),
        );
        run_once_systems.0.insert(
            "setup_breeze".into(),
            world.register_system(aircraft::spawn_breeze),
        );
        run_once_systems.0.insert(
            "setup_j3cub".into(),
            world.register_system(aircraft::spawn_j3cub),
        );
        run_once_systems.0.insert(
            "spawn_helicopter".into(),
            world.register_system(aircraft::spawn_helicopter),
        );
        run_once_systems.0.insert(
            "spawn_ui_hud".into(),
            world.register_system(UI::setup_ui_hud),
        );

        run_once_systems
            .0
            .insert("spawn_menu".into(), world.register_system(Menu::spawn));

        run_once_systems
    }
}

fn main() {
    let mut app = App::new();
    let default_plugins = DefaultPlugins
        .set(WindowPlugin {
            primary_window: Some(Window {
                title: "Kestrel Flight Simulator".into(),
                ..default()
            }),
            ..default()
        })
        .build()
        .disable::<TransformPlugin>();

    app.add_plugins((
        default_plugins,
        PhysicsPlugins::default(),
        MeshPickingPlugin,
        UI,
        AircraftFdmPlugin::default(),
        FrameTimeDiagnosticsPlugin::default(),
        DiagnosticsOverlayPlugin,
        Sse,
        BigSpaceDefaultPlugins,
        bevy::camera_controller::free_camera::FreeCameraPlugin,
        SkeinPlugin::default(),
        bevy::pbr::diagnostic::MaterialAllocatorDiagnosticPlugin::<StandardMaterial>::default(),
        bevy::render::diagnostic::MeshAllocatorDiagnosticPlugin,
    ))
    .add_plugins(MaterialPlugin::<
        ExtendedMaterial<StandardMaterial, TerrainMaterial>,
    >::default())
    // .insert_gizmo_config(
    //     FdmGizmos {
    //         force_scale: 1.0 / 600.0,
    //         total_force_color: None,
    //         weight_color: None,
    //         ..FdmGizmos::default()
    //     },
    //     GizmoConfig::default(),
    // )
    // .add_plugins(AircraftFdmDebugPlugin)
    // .add_plugins(PhysicsDebugPlugin)
    // Resources
    .init_resource::<BuildingMaterial>()
    .insert_resource(PhysicsPickingSettings {
        require_markers: true,
    })
    .insert_resource(Gravity::ZERO)
    .insert_resource(GameState { running: false })
    .insert_resource(ControlInputs {
        pitch: 0.0,
        yaw: 0.0,
        roll: 0.0,
        throttle: 0.0,
        ground_brakes: 0.0,
    })
    .insert_resource(CameraSettings::default())
    .insert_resource(input::Keymap::default())
    .insert_resource(Settings::fetch())
    .insert_resource(LightsTimers {
        acol: Timer::from_seconds(ACOL_OFF_DURATION, TimerMode::Repeating),
        acol_on_cycle: false,
        strobe: Timer::from_seconds(STROBE_OFF_DURATION, TimerMode::Repeating),
        strobe_on_cycle: false,
    })
    .insert_resource(terrain::HttpClient(Client::new()))
    .insert_resource(terrain::TileSemaphore(Arc::new(Semaphore::new(64))))
    .init_resource::<terrain::TerrainCacheResource>()
    .init_resource::<terrain::TerrainChunkRegistry>()
    .init_resource::<terrain::TerrainUpdateTracker>()
    .init_resource::<RunOnceSystemList>()
    .insert_resource(ClearColor(Color::BLACK))
    .insert_resource(TerrainCacheResource::default())
    .insert_resource(UiTheme(create_dark_theme()))
    .insert_resource(AircraftState::default())
    .insert_resource(LandingGearStatus::Retracted)
    .insert_resource(CameraPosition::default())
    .insert_resource(avian3d::physics_transform::PhysicsTransformConfig {
        propagate_before_physics: false,
        transform_to_position: false,
        position_to_transform: false,
        transform_to_collider_scale: true,
    })
    // Messages
    .add_message::<LandingGearCommand>()
    .add_message::<ButtonMessages>()
    .add_message::<ResetMessage>()
    // Systems
    .add_systems(Startup, setup)
    .add_systems(
        Update,
        (
            Button::press_system,
            track_aircraft_terrain,
            update_rotors,
            input::input_system,
            aircraft::buttons::update_cursor,
            screenshot,
            AircraftCamera::controller,
            terrain::poll_terrain,
            track_resets,
        ),
    )
    .add_systems(
        FixedPostUpdate,
        (
            sync_to_avian.in_set(PhysicsSystems::Prepare),
            sync_from_avian.in_set(TransformSystems::Propagate),
        ),
    )
    .add_systems(
        FixedUpdate,
        (
            planet_gravity,
            update_control_surfaces,
            Light::update_light_cycle,
            (Light::update_mesh_lights, Light::update_lights).after(Light::update_light_cycle),
            LandingGear::operate_landing_gear,
            screenshot_saving,
            aircraft::screens::update_screens,
            aircraft::main,
            Button::listener,
            rotate_sun,
            game_state,
            scenery::osm::buildings::poll,
        ),
    );

    app.run();
}

fn setup(mut commands: Commands) {
    #[cfg(debug_assertions)]
    {
        commands.spawn(DiagnosticsOverlay::fps());
        commands.spawn(DiagnosticsOverlay::mesh_and_standard_material());
    }
    commands.insert_resource(MaterialStore {
        lights: StandardMaterial {
            perceptual_roughness: 0.1,
            specular_transmission: 1.0,
            base_color: Color::LinearRgba(LinearRgba::rgb(0.5, 0.5, 0.5).with_alpha(0.5)),
            ..default()
        },
    });
}

fn screenshot(mut commands: Commands, input: Res<ButtonInput<KeyCode>>) {
    if input.just_pressed(KeyCode::F3) {
        let now = chrono::Local::now();
        let path = format!(".user/screenshots/screenshot-{:?}.png", now);
        commands
            .spawn(Screenshot::primary_window())
            .observe(save_to_disk(path));
    }
}

fn screenshot_saving(
    mut commands: Commands,
    screenshot_saving: Query<Entity, With<Capturing>>,
    window: Single<Entity, With<Window>>,
) {
    match screenshot_saving.iter().count() {
        0 => {
            commands.entity(*window).remove::<CursorIcon>();
        }
        x if x > 0 => {
            commands
                .entity(*window)
                .insert(CursorIcon::from(SystemCursorIcon::Progress));
        }
        _ => {}
    }
}

fn game_state(mut physics_time: ResMut<Time<Physics>>, game_state: Res<GameState>) {
    match game_state.running {
        false => physics_time.pause(),
        true => physics_time.unpause(),
    }
}

pub const CELL_SIZE: f64 = 500.0;

// big_space compatibility systems

fn sync_to_avian(mut bodies: Query<(&CellCoord, &Transform, &mut Position, &mut Rotation)>) {
    for (cell, transform, mut pos, mut rot) in &mut bodies {
        let cell_diff = DVec3::new((cell.x) as f64, (cell.y) as f64, (cell.z) as f64);
        let position = transform.translation.as_dvec3() + cell_diff * CELL_SIZE;
        let rotation = transform.rotation.as_dquat().normalize();

        pos.0 = position;
        rot.0 = rotation;
    }
}

fn sync_from_avian(mut bodies: Query<(&Position, &mut CellCoord, &mut Transform, &Rotation)>) {
    for (pos, mut cell, mut transform, rot) in &mut bodies {
        let relative_pos = pos.0;

        let cell_offset_x = (relative_pos.x / CELL_SIZE).round() as i64;
        let cell_offset_y = (relative_pos.y / CELL_SIZE).round() as i64;
        let cell_offset_z = (relative_pos.z / CELL_SIZE).round() as i64;

        cell.x = cell_offset_x;
        cell.y = cell_offset_y;
        cell.z = cell_offset_z;
        transform.translation = (relative_pos
            - DVec3::new(
                cell_offset_x as f64 * CELL_SIZE,
                cell_offset_y as f64 * CELL_SIZE,
                cell_offset_z as f64 * CELL_SIZE,
            ))
        .as_vec3();
        transform.rotation = rot.as_quat().normalize();
    }
}

fn planet_gravity(mut query: Query<(Forces, &Position)>, game_state: Res<GameState>) {
    if game_state.running {
        let planet_center = DVec3::new(0.0, 0.0, 0.0); // your planet's position
        for (mut forces, position) in &mut query {
            let to_center = planet_center - position.as_ivec3().as_dvec3();
            let direction = to_center.normalize_or_zero();
            forces.apply_linear_acceleration(direction * 9.81);
        }
    }
}

pub fn absolute_position(cell: &CellCoord, local_translation: Vec3) -> DVec3 {
    DVec3::new(cell.x as f64, cell.y as f64, cell.z as f64) * CELL_SIZE as f64
        + local_translation.as_dvec3()
}

fn track_aircraft_terrain(
    mut commands: Commands,
    grid: Single<&Grid, With<BigSpace>>,
    mut registry: ResMut<terrain::TerrainChunkRegistry>,
    mut tracker: ResMut<terrain::TerrainUpdateTracker>,
    mut settings: ResMut<Settings>,
    cache: Res<terrain::TerrainCacheResource>,
    client: Res<terrain::HttpClient>,
    semaphore: Res<terrain::TileSemaphore>,
    aircraft: Single<(&CellCoord, &Transform), With<Aircraft>>,
    reset_messages: MessageReader<ResetMessage>,
) {
    let override_update = !reset_messages.is_empty();
    let (cell_coord, transform) = *aircraft;
    let reference_pos = absolute_position(cell_coord, transform.translation).as_vec3();

    settings.terrain = terrain::update_terrain_for_aircraft(
        &mut commands,
        &grid,
        &mut registry,
        &mut tracker,
        &client.0,
        Arc::clone(&semaphore.0),
        cache.cache.clone(),
        settings.terrain,
        reference_pos,
        override_update,
    );
}

#[derive(Message)]
pub struct ResetMessage;

fn track_resets(
    mut message_writer: MessageWriter<ResetMessage>,
    game_state: Res<GameState>,
    mut prev_running: Local<bool>,
) {
    if *prev_running == false && game_state.running {
        message_writer.write(ResetMessage);
    }
    *prev_running = game_state.running;
}
