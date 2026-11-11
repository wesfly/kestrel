use crate::aircraft::{Aircraft, RotorTypes, Sides, control_surfaces::ControlSurfaces};
use bevy::prelude::*;

pub fn update_rotors(
    query: Query<(&mut Transform, &RotorTypes)>,
    aircraft: Single<&Aircraft>,
    time: Res<Time>,
) {
    // TODO include Engine component
    if aircraft.engine.on {
        for (mut rotor, rotor_type) in query {
            match rotor_type {
                RotorTypes::Main(a) => rotor.rotate_local_axis(*a, 100.0 * time.delta_secs()),
                RotorTypes::Rear => rotor.rotate_local_z(100.0 * time.delta_secs()),
            }
        }
    }
}

#[derive(Component)]
pub struct RestRotation(Quat);

pub fn update_control_surfaces(
    mut commands: Commands,
    ctrl_surfaces: Query<(
        Entity,
        &mut Transform,
        &ControlSurfaces,
        Option<&RestRotation>,
    )>,
    state: Single<&Aircraft>,
) {
    let cs = state.control_surfaces;
    let aileron_angle = Sides {
        left: (cs.aileron.left * 30.0).to_radians(),
        right: (cs.aileron.right * 30.0).to_radians(),
    };
    let elevator_angle = Sides {
        left: (cs.elevator.left * 30.0).to_radians(),
        right: (cs.elevator.right * 30.0).to_radians(),
    };
    let lerp_speed = 0.05;

    for (entity, mut transform, ctrl_surface, rest) in ctrl_surfaces {
        // Insert RestRotation on first run
        let rest_rotation = match rest {
            Some(r) => r.0,
            None => {
                let r = transform.rotation;
                commands.entity(entity).insert(RestRotation(r));
                r
            }
        };

        let delta = match ctrl_surface {
            ControlSurfaces::Custom(s) => match &s.as_str() {
                &"canard_left" => Quat::from_rotation_y(cs.canards.left),
                &"canard_right" => Quat::from_rotation_y(cs.canards.right),
                s => {
                    warn!("Control surface {s} not implemented.");
                    Quat::default()
                }
            },
            ControlSurfaces::Rudder => Quat::from_rotation_y((-cs.rudder * 30.0).to_radians()),
            ControlSurfaces::ElevatorLeft => Quat::from_rotation_x(elevator_angle.left),
            ControlSurfaces::ElevatorRight => Quat::from_rotation_x(elevator_angle.right),
            ControlSurfaces::AileronLeft => Quat::from_rotation_x(aileron_angle.left),
            ControlSurfaces::AileronRight => Quat::from_rotation_x(aileron_angle.right),
        };

        let target = rest_rotation * delta;
        transform.rotation = transform.rotation.lerp(target, lerp_speed);
    }
}
