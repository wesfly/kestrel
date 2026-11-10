use crate::{
    CELL_SIZE, Settings,
    scenery::terrain::{TerrainCacheResource, TerrainChunkRegistry},
    ui::MenuCamera,
};
use bevy::{
    light::{
        Atmosphere, CascadeShadowConfigBuilder,
        atmosphere::{Falloff, ScatteringMedium, ScatteringTerm},
        light_consts::lux,
    },
    prelude::*,
};
use big_space::prelude::*;

pub mod terrain;

pub fn setup_scene(
    mut commands: Commands,
    settings: Res<Settings>,
    camera: Single<Entity, With<MenuCamera>>,
    mut scattering_mediums: ResMut<Assets<ScatteringMedium>>,
    mut terrain_tile_cache: ResMut<TerrainCacheResource>,
    mut terrain_registry: ResMut<TerrainChunkRegistry>,
) {
    // Init/Reset tile cache
    *terrain_tile_cache = TerrainCacheResource::default();
    *terrain_registry = TerrainChunkRegistry::default();

    commands.spawn_big_space(Grid::new(CELL_SIZE as f32, 10.0), |root| {
        root.insert(Visibility::default());
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
        root.spawn_spatial(earth_atmosphere.clone());
    });

    commands.entity(*camera).despawn();

    let cascade = CascadeShadowConfigBuilder {
        maximum_distance: settings.shadow_distance.clamp(1.0, 100_000.0),
        ..Default::default()
    }
    .build();

    let sun_position = Vec3::new(1.0, 0.1, 0.0).normalize();
    let shadows_enabled = settings.shadow_distance != 0.0;
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
        Transform::from_translation(sun_position).looking_at(Vec3::ZERO, Vec3::Y),
        cascade,
    ));
}
