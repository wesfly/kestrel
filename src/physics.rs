use avian_fdm::components::PlanetResource;
use avian3d::{
    dynamics::integrator::Gravity,
    schedule::{Physics, PhysicsTime},
};
use bevy::{math::DVec3, prelude::*};

use crate::{EARTH_RADIUS, SimState};

pub struct GravityPlugin;
impl Plugin for GravityPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(Gravity::ZERO)
            .insert_resource(PlanetResource {
                center: DVec3::ZERO,
                radius_m: EARTH_RADIUS as f64,
                surface_gravity_ms2: crate::G,
            });
    }
}

/// Syncs `PhysicsTime` to `SimState`.
pub fn sync_physics_time(mut physics_time: ResMut<Time<Physics>>, sim_state: Res<State<SimState>>) {
    match **sim_state {
        SimState::Running => physics_time.unpause(),
        SimState::Paused => physics_time.pause(),
    }
}
