/*
I made a little flight simulator here. Check out the README for further information.
If you have fixes or want to contribute, just make a pull request.

Spatial units are in metres
Default for speed is m/s
*/

mod addons;
mod aircraft;
mod big_space_compatibility;
mod camera;
mod input;
mod physics;
mod scenery;
mod settings;
mod sse;
mod time;
mod ui;
mod utils;
mod window;

use crate::{
    addons::AddonsPlugin,
    aircraft::{Aircraft, AircraftPlugin, animations::update_control_surfaces},
    big_space_compatibility::BigSpaceCompatibilityPlugin,
    camera::{AircraftCamera, CameraPosition, CameraSettings, rotate_sun},
    input::ControlInputs,
    physics::GravityPlugin,
    scenery::{
        setup_scene,
        terrain::{self, TerrainCacheResource, TerrainMaterial},
    },
    settings::{Settings, SettingsPlugin},
    sse::Sse,
    time::GameTimePlugin,
    ui::UiPlugin,
    utils::acceleration::point_accel,
    window::cursor::CursorPlugin,
};
use avian_fdm::prelude::*;
use avian3d::prelude::*;
use bevy::{
    dev_tools::diagnostics_overlay::DiagnosticsOverlayPlugin,
    diagnostic::FrameTimeDiagnosticsPlugin,
    feathers::{dark_theme::create_dark_theme, theme::UiTheme},
    math::DVec3,
    pbr::ExtendedMaterial,
    prelude::*,
};
use bevy_skein::SkeinPlugin;
use big_space::prelude::*;
use core::option::Option::Some;
use once_cell::sync::Lazy;
use reqwest::Client;
use std::sync::Arc;
use tokio::{runtime::Runtime, sync::Semaphore};

#[cfg(debug_assertions)]
use bevy::dev_tools::diagnostics_overlay::DiagnosticsOverlay;

pub const METRES_TO_FEET: f32 = 3.28084;
pub const M_S_TO_KTS: f32 = 1.943844;
pub const EARTH_RADIUS: f32 = 6_360_000.0;
pub const G: f32 = -9.81;

#[derive(States, Clone, PartialEq, Eq, Hash, Debug, Default)]
pub enum AppState {
    #[default]
    Menu,
    Loading,
    InGame,
}

#[derive(SubStates, Debug, Default, PartialEq, PartialOrd, Ord, Eq, Hash, Clone)]
#[source(AppState = AppState::InGame)]
pub enum SimState {
    #[default]
    Paused,
    Running,
}

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
        AircraftFdmPlugin::default(),
        FrameTimeDiagnosticsPlugin::default(),
        DiagnosticsOverlayPlugin,
        Sse,
        BigSpaceDefaultPlugins,
        bevy::camera_controller::free_camera::FreeCameraPlugin,
        SkeinPlugin::default(),
        bevy::pbr::diagnostic::MaterialAllocatorDiagnosticPlugin::<StandardMaterial>::default(),
        bevy::render::diagnostic::MeshAllocatorDiagnosticPlugin,
        (
            BigSpaceCompatibilityPlugin,
            CursorPlugin,
            UiPlugin,
            AddonsPlugin,
            GameTimePlugin,
            GravityPlugin,
            SettingsPlugin,
            AircraftPlugin,
        ),
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
    .init_state::<SimState>()
    .init_state::<AppState>()
    .insert_resource(PhysicsPickingSettings {
        require_markers: true,
    })
    .insert_resource(Gravity::ZERO)
    .insert_resource(ControlInputs {
        pitch: 0.0,
        yaw: 0.0,
        roll: 0.0,
        throttle: 0.0,
    })
    .insert_resource(CameraSettings::default())
    .insert_resource(input::Keymap::default())
    .insert_resource(terrain::HttpClient(Client::new()))
    .insert_resource(terrain::TileSemaphore(Arc::new(Semaphore::new(64))))
    .init_resource::<terrain::TerrainCacheResource>()
    .init_resource::<terrain::TerrainChunkRegistry>()
    .init_resource::<terrain::TerrainUpdateTracker>()
    .insert_resource(ClearColor(Color::BLACK))
    .insert_resource(TerrainCacheResource::default())
    .insert_resource(UiTheme(create_dark_theme()))
    .insert_resource(CameraPosition::default())
    .insert_resource(avian3d::physics_transform::PhysicsTransformConfig {
        propagate_before_physics: false,
        transform_to_position: false,
        position_to_transform: false,
        transform_to_collider_scale: true,
    })
    // Messages
    // Systems
    .add_systems(Startup, setup)
    .add_systems(OnEnter(AppState::InGame), setup_scene)
    .add_systems(
        Update,
        (
            track_aircraft_terrain,
            input::input_system,
            window::trigger_screenshot,
            AircraftCamera::controller,
            terrain::poll_terrain,
            utils::DespawnAfter::tick_despawn_after_timers,
            window::toggle_fullscreen,
        ),
    )
    .add_systems(
        FixedUpdate,
        (
            physics::planet_gravity,
            update_control_surfaces,
            window::screenshot_saving,
            rotate_sun,
            physics::sync_physics_time,
            point_accel,
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

pub const CELL_SIZE: f64 = 500.0;

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
    terrain_cache: Res<terrain::TerrainCacheResource>,
    client: Res<terrain::HttpClient>,
    semaphore: Res<terrain::TileSemaphore>,
    mut aircraft: Single<(&CellCoord, &Transform), With<Aircraft>>,
) {
    let (cell_coord, transform) = &mut *aircraft;
    let reference_pos = absolute_position(cell_coord, transform.translation).as_vec3();

    settings.terrain = terrain::update_terrain_for_aircraft(
        &mut commands,
        &grid,
        &mut registry,
        &mut tracker,
        &client.0,
        Arc::clone(&semaphore.0),
        terrain_cache.cache.clone(),
        settings.terrain,
        reference_pos,
    );
}
