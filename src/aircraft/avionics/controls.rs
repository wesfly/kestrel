pub mod fly_by_wire;

use crate::{
    aircraft::{Aircraft, Sides, ToSides},
    input::ControlInputs,
};
use bevy::prelude::*;

/// This is for all aircraft not using fly-by-wire.
#[derive(Component)]
pub struct DirectControls;

impl DirectControls {
    pub fn update(
        mut aircraft: Single<&mut Aircraft, With<DirectControls>>,
        input: Res<ControlInputs>,
    ) {
        info_once!("Direct Controls Detected");
        let ref mut ctrl_sfc = &mut aircraft.control_surfaces;

        ctrl_sfc.canards = (0.0_f32.clamp(-1.0, 1.0)).left_right();
        ctrl_sfc.aileron = Sides {
            left: -input.roll.clamp(-1.0, 1.0),
            right: input.roll.clamp(-1.0, 1.0),
        };
        ctrl_sfc.elevator = (-input.pitch.clamp(-1.0, 1.0)).left_right();
        ctrl_sfc.rudder = input.yaw;
        ctrl_sfc.ground_brakes = input.ground_brakes;
    }
}
