use avian3d::{
    dynamics::{
        integrator::Gravity,
        rigid_body::forces::{Forces, WriteRigidBodyForces},
    },
    physics_transform::Position,
};
use bevy::prelude::*;

use crate::SimState;

pub struct GravityPlugin;
impl Plugin for GravityPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(Gravity::ZERO)
            .add_systems(Update, planet_gravity.run_if(in_state(SimState::Running)));
    }
}

pub fn planet_gravity(mut query: Query<(Forces, &Position)>) {
    let planet_center = Vec3::new(0.0, 0.0, 0.0);
    for (mut forces, position) in &mut query {
        let to_center = planet_center - position.0.as_vec3();
        let direction = to_center.normalize_or_zero();
        forces.apply_linear_acceleration(direction * 9.81);
    }
}
