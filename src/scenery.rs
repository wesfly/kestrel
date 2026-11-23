use crate::{
    CELL_SIZE, Settings,
    addons::Addons,
    aircraft::{Aircraft, control_surfaces::ControlSurfacesDeflection},
    camera::AircraftCamera,
    scenery::terrain::{TerrainCacheResource, TerrainChunkRegistry},
    ui::ChosenAircraft,
    utils::{Coord, TransformLogger},
};
use avian_fdm::components::{
    AeroZone, AircraftGeometry, AtmosphereState, ControlInputs, EngineZone, FlightState,
    InducedDrag, PlanetGravity,
};
use avian3d::{dynamics::rigid_body::RigidBody, physics_transform::Position};
use bevy::{
    // camera_controller::free_camera::FreeCamera,
    light::{
        Atmosphere, CascadeShadowConfigBuilder,
        atmosphere::{Falloff, ScatteringMedium, ScatteringTerm},
        light_consts::lux,
    },
    math::DVec3,
    prelude::*,
};
use big_space::prelude::*;

pub mod terrain;

pub fn setup_scene(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    settings: Res<Settings>,
    mut scattering_mediums: ResMut<Assets<ScatteringMedium>>,
    mut terrain_tile_cache: ResMut<TerrainCacheResource>,
    mut terrain_registry: ResMut<TerrainChunkRegistry>,
    chosen_aircraft: Res<ChosenAircraft>,
) {
    // Init/Reset tile cache
    *terrain_tile_cache = TerrainCacheResource::default();
    *terrain_registry = TerrainChunkRegistry::default();

    info!("Spawning scene");

    commands.spawn_big_space(Grid::new(CELL_SIZE as f32, 10.0), |root| {
        root.insert((Visibility::default(), DespawnOnExit(AppState::InGame)));
        let mut earth_medium = ScatteringMedium::earth(256, 256);
        let atmosphere_height = 60_000.0;

        let ground_haze_phase = earth_medium.terms[1].phase.clone(); // reuse Mie's phase function
        earth_medium.terms.push(ScatteringTerm {
            scattering: Vec3::splat(3.0e-5),
            absorption: Vec3::ZERO,
            falloff: Falloff::Exponential {
                scale: 400.0 / atmosphere_height,
            },
            phase: ground_haze_phase,
        });

        let earth_medium = scattering_mediums.add(earth_medium);
        let earth_atmosphere = Atmosphere::earth(earth_medium);
        root.spawn_spatial((
            earth_atmosphere.clone(),
            Position::new(DVec3::ZERO)
        ));

        let translation = DVec3::new(0.0, 6_361_018.0, 0.0);
        let (cell, tl) = root.grid().translation_to_grid(translation);

        root.spawn((
            AircraftCamera::new(),
            Position(translation),
            Transform::from_translation(tl),
            cell,
        ));

        let aircraft = chosen_aircraft.0
            .clone()
            .expect("Aircraft should exist because this function is only triggered when there is a selected aircraft.");

        let model_path =
            Addons::get_model_path(
                &aircraft.config.general.name,
                &aircraft.variant.name,
                &aircraft.variant.model
        );

        let atmosphere_temperature_c = 10.0;

        root.spawn((
            // TransformLogger,
            WorldAssetRoot(asset_server.load(GltfAssetLabel::Scene(0).from_asset(model_path))),
            Position(translation),
            FlightState::default(),
            RigidBody::Dynamic,
            cell,
            Transform::from_translation(tl),
            ControlInputs::default(),
            PlanetGravity,
            Aircraft {
                name: aircraft.config.general.name,
                variant: aircraft.variant.name,
                coord: Coord::default(),
                control_surfaces: ControlSurfacesDeflection::default(),
                landing_gear: crate::aircraft::landing_gear::LandingGearStatus::Retracted,
                on_ground: false,
                systems: super::aircraft::Systems::default(),
            },
            AtmosphereState {
                temperature_k: 273.15 + atmosphere_temperature_c,
                ..default()
            }
        ));
    });

    let cascade = CascadeShadowConfigBuilder {
        maximum_distance: settings.graphics.shadow_distance.clamp(1.0, 100_000.0),
        ..Default::default()
    }
    .build();

    let sun_position = Vec3::new(1.0, 0.0, 0.0).normalize();
    let shadows_enabled = settings.graphics.shadow_distance != 0.0;
    if !shadows_enabled {
        info!("Shadows disabled")
    }
    commands.spawn((
        DirectionalLight {
            shadow_maps_enabled: shadows_enabled,
            contact_shadows_enabled: shadows_enabled,
            illuminance: lux::RAW_SUNLIGHT,
            ..default()
        },
        DespawnOnExit(AppState::InGame),
        Transform::from_translation(sun_position).looking_at(Vec3::ZERO, Vec3::Y),
        cascade,
    ));
}

pub fn promote_aircraft_body(
    mut commands: Commands,
    newly_added_aircraft: Query<
        (Entity, &Aircraft, &AircraftGeometry, Option<&InducedDrag>),
        Added<Aircraft>,
    >,
    // ChildOf stores the parents
    parents: Query<&ChildOf>,
    children: Query<&Children>,
    world_asset_roots: Query<(), With<WorldAssetRoot>>,
    zone_marks: Query<(), Or<(With<AeroZone>, With<EngineZone>)>>,
) {
    for (entity, aircraft, geometry, induced_drag) in &newly_added_aircraft {
        // Find parents with `WorldAssetRoot`
        let Some(root) = parents
            .iter_ancestors(entity)
            .find(|e| world_asset_roots.contains(*e))
        else {
            continue;
        };

        // Clone `Aircraft` and `AircraftGeometry` onto the root
        commands
            .entity(root)
            .insert((aircraft.clone(), geometry.clone()));
        if let Some(i) = induced_drag {
            commands.entity(root).insert(i.clone());
        }

        let zones: Vec<Entity> = children
            .get(entity)
            .map(|c| c.iter().filter(|e| zone_marks.contains(*e)).collect())
            .unwrap_or_default();

        for zone in zones {
            commands.entity(zone).insert(ChildOf(root));
        }

        commands
            .entity(entity)
            .remove::<(Aircraft, AircraftGeometry, InducedDrag)>();
    }
}
