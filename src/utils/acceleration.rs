//! # Acceleration probes to measure G-forces
//!
//! This can be useful for structural damage or pilot blood effects that will affect vision.

use avian3d::prelude::*;
use bevy::prelude::*;

#[derive(Component, Debug, Default, Reflect)]
#[reflect(Component)]
#[require(Transform)]
pub struct PointAccel {
    /// When there are more than two `PointAccel`, this helps them distinguish from each other.
    prev_vel: Option<Vec3>,
    pub acceleration: Vec3, // world space, true kinematic acceleration
    pub world: Vec3,        // world space, accelerometer-style
    pub local: Vec3,        // body-local, accelerometer-style
}

pub fn point_accel(
    time: Res<Time<Physics>>,
    gravity: Res<Gravity>,
    bodies: Query<(
        &Rotation,
        &LinearVelocity,
        &AngularVelocity,
        &ComputedCenterOfMass,
    )>,
    mut points: Query<(&ChildOf, &Transform, &mut PointAccel)>,
) {
    let dt = time.delta_secs();
    if dt <= 0.0 {
        return;
    }

    for (child_of, tf, mut p) in &mut points {
        let Ok((rot, lin, ang, com)) = bodies.get(child_of.parent()) else {
            continue;
        };

        let r_world = rot.0 * (tf.translation - com.0);
        let v_point = lin.0 + ang.0.cross(r_world);

        if let Some(prev) = p.prev_vel {
            let a = (v_point - prev) / dt;
            p.acceleration = a;
            p.world = a - gravity.0;
            p.local = rot.0.inverse() * p.world;
        }
        p.prev_vel = Some(v_point);
    }
}
