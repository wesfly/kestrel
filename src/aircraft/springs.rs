use avian3d::{
    dynamics::rigid_body::forces::{Forces, ReadRigidBodyForces, WriteRigidBodyForces},
    physics_transform::Position,
    spatial_query::{SpatialQuery, SpatialQueryFilter},
};
use bevy::prelude::*;
use std::ops::DerefMut;

use crate::{
    G,
    aircraft::{
        Aircraft,
        landing_gear::{LEFT_POS, LandingGearStatus, NOSEWHEEL_POS, RIGHT_POS},
    },
    bevy_to_aerospace_coords,
};

const REST: f32 = 1.2;

const MAIN_STRENGTH: f32 = 400_000.0;
const MAIN_DAMPING: f32 = 59_000.0;

const NOSE_STRENGTH: f32 = 158_000.0;
const NOSE_DAMPING: f32 = 17_500.0;

const MAX_FORCE: f32 = 2_000_000.0; // safety clamp only

/// Tire lateral grip coefficient
const MU_LATERAL: f32 = 0.8;
const MU_BRAKE: f32 = 0.8; // dry max braking friction coefficient
const TIRE_RELAXATION_TIME: f32 = 0.1; // s, how fast lateral slip is killed

#[derive(Component, Reflect)]
#[reflect(Component)]
struct LandingGearLeg {
    strength: f64,
    damping: f64,
    mu_lateral: f64,
    mu_brake: f64,
    tire_relaxation_time: f64,
}

pub fn spring_forces(
    spatial_query: SpatialQuery,
    mut query: Single<(
        &mut Aircraft,
        &Position,
        &Transform,
        Forces,
        Option<&mut avian_fdm::prelude::ControlInputs>,
    )>,
) {
    let (aircraft, position, transform, force, _) = query.deref_mut();

    if !(aircraft.landing_gear == LandingGearStatus::Deployed
        && force.linear_velocity().length() <= 200.0)
    {
        return;
    }

    let landing_gear = [
        bevy_to_aerospace_coords() * LEFT_POS,
        bevy_to_aerospace_coords() * RIGHT_POS,
        bevy_to_aerospace_coords() * NOSEWHEEL_POS,
    ];

    let mut on_ground_vec = [false; 3];

    for (i, gear_pos) in landing_gear.iter().enumerate() {
        let is_nosewheel = i == 2;
        let (strength, damping) = if is_nosewheel {
            (NOSE_STRENGTH, NOSE_DAMPING)
        } else {
            (MAIN_STRENGTH, MAIN_DAMPING)
        };
        let rest = if is_nosewheel { REST - 0.1 } else { REST };

        let filter = SpatialQueryFilter::DEFAULT;

        // World origin
        let origin = position.0 + (transform.rotation * gear_pos).as_dvec3();
        let ray_dir = transform.local_z();

        let Some(hit) = spatial_query.cast_ray(origin, ray_dir, rest, true, &filter) else {
            continue;
        };

        let distance = hit.distance.max(1e-4);
        let spring_dir = -transform.local_z();
        on_ground_vec[i] = true;

        let contact_point = origin + (ray_dir * distance).as_dvec3();

        //============================== springs ==============================
        let spring_vel = spring_dir.dot(force.velocity_at_point(contact_point));
        let spring_scalar = spring(distance, rest, strength, damping, spring_vel);
        let spring_force = (spring_scalar * spring_dir).clamp_length_max(MAX_FORCE);

        force.apply_force_at_point(spring_force, contact_point);

        let normal_load = spring_scalar.max(0.0);

        //============================== steering/anti-drift ==============================
        let steering_dir = if is_nosewheel {
            Quat::from_rotation_y(20.0 * aircraft.control_surfaces.rudder.to_radians())
                * transform.local_y()
        } else {
            transform.local_y()
        };
        let vel_at_contact_point = force.velocity_at_point(contact_point);
        let steering_vel = steering_dir.dot(vel_at_contact_point);

        let effective_mass = normal_load / G as f32;
        let desired_lateral_accel = -steering_vel / TIRE_RELAXATION_TIME;
        let max_lateral_force = normal_load * MU_LATERAL;
        let lateral_force = (effective_mass * desired_lateral_accel * steering_dir)
            .clamp_length_max(max_lateral_force);
        force.apply_force_at_point(lateral_force, contact_point);

        //============================== brakes ==============================
        if !is_nosewheel {
            let tire_speed = transform.local_x().dot(vel_at_contact_point);
            let brake_input = if aircraft.parking_brake {
                1.0
            } else {
                aircraft.control_surfaces.ground_brakes
            };

            let max_brake_force = normal_load * MU_BRAKE * brake_input;
            let braking_force = tire_speed.signum() * max_brake_force * -transform.local_x();

            force.apply_force_at_point(braking_force, contact_point);
        }
    }

    let ldg_gear_on_ground = on_ground_vec.iter().filter(|&&x| x).count();
    aircraft.on_ground = ldg_gear_on_ground >= 2;
}

fn spring(
    distance: f32,
    rest_length: f32,
    strength: f32,
    damping_factor: f32,
    velocity: f32,
) -> f32 {
    let offset = rest_length - distance;
    if offset >= 0.0 {
        let spring = offset * strength;
        let damping = velocity * damping_factor;
        return spring - damping;
    }
    0.0
}
