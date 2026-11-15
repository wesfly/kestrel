use bevy::prelude::*;

use crate::{aircraft::engine::Engine, utils::Axis3D};

#[derive(Component, Default, Reflect, Debug)]
#[reflect(Default, Component)]
pub struct Rotor {
    name: String,
    rpm_type: RpmType,
    axis: Axis3D,
}

#[derive(Component, Default, Reflect, Debug)]
pub enum RpmType {
    #[default]
    DirectlyControlled,
    Coupled(CoupledToEngine),
}

#[derive(Component, Default, Reflect, Debug)]
pub struct CoupledToEngine {
    name: String,
    gear_ratio: f32,
}

pub fn update_rotors(
    query: Query<(&mut Transform, &Rotor)>,
    engines: Query<&Engine>,
    time: Res<Time>,
) {
    for (mut transform, rotor) in query {
        let rpm = match &rotor.rpm_type {
            RpmType::DirectlyControlled => 60.0,
            RpmType::Coupled(coupled_to_engine) => {
                let Some(engine) = engines
                    .iter_inner()
                    .find(|e| e.name() == coupled_to_engine.name)
                else {
                    error!("Engine specified in {:?} does not exist.", rotor);
                    continue;
                };

                engine.get_rpm()
            }
        };

        let axis = match rotor.axis {
            Axis3D::X => Dir3::X,
            Axis3D::Y => Dir3::Y,
            Axis3D::Z => Dir3::Z,
        };

        transform.rotate_axis(axis, (rpm / 60.0) * time.delta_secs());
    }
}
