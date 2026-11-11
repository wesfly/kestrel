// TODO: Make lights scriptable in lua file

use crate::{MaterialStore, aircraft::Aircraft};
use bevy::{
    ecs::{lifecycle::HookContext, world::DeferredWorld},
    prelude::*,
};
use serde::Deserialize;

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
pub struct Light {
    name: String,
    on: bool,
}

impl Light {
    pub fn is_on(&self) -> bool {
        self.on
    }

    /// Returns the identifier of the light
    pub fn name(&self) -> String {
        self.name.clone()
    }

    pub fn toggle(&mut self) {
        self.on ^= true
    }

    pub fn update_mesh_lights(
        aircraft: Single<&Aircraft>,
        material_handles: Query<(&MeshMaterial3d<StandardMaterial>, &Light, Entity)>,
        mut materials: ResMut<Assets<StandardMaterial>>,
    ) {
        for material_handle in material_handles.iter() {
            if let Some(mut material) = materials.get_mut(material_handle.0) {
                let LinearRgba {
                    ref mut red,
                    ref mut green,
                    ref mut blue,
                    alpha: _,
                } = material.emissive;

                *red = 100.0;
            }
        }
    }

    pub fn update_lights(
        aircraft: Single<&Aircraft>,
        point_light_query: Query<(&mut PointLight, &Light)>,
        spot_light_query: Query<(&mut SpotLight, &Light)>,
    ) {
        for (mut point_light, light) in point_light_query {
            let (colour, on) = (Color::linear_rgb(1.0, 0.0, 0.0), true);

            point_light.color = colour;

            if on {
                point_light.intensity = 10000.0;
            } else {
                point_light.intensity = 0.0
            }
        }
        for (mut spot_light, light) in spot_light_query {
            let (colour, on) = (Color::linear_rgb(1.0, 0.0, 0.0), true);

            spot_light.color = colour;

            if on {
                spot_light.intensity = 100000000.0;
            } else {
                spot_light.intensity = 0.0
            }
        }
    }
}
