use bevy::prelude::*;

use crate::{aircraft::Aircraft, utils::Axis3D};

#[derive(Component, Default, Reflect)]
#[reflect(Default, Component)]
pub struct Rotor {
    name: String,
    rpm_type: RpmType,
    axis: Axis3D,
}

#[derive(Component, Default, Reflect)]
pub enum RpmType {
    #[default]
    DirectlyControlled,
    Coupled(CoupledToEngine),
}

#[derive(Component, Default, Reflect)]
pub struct CoupledToEngine {
    name: String,
    gear_ratio: f32,
}

pub fn update_rotors(
    query: Query<(&mut Transform, &Rotor)>,
    aircraft: Single<&Aircraft>,
    time: Res<Time>,
) {
    for (mut transform, rotor) in query {}
}
