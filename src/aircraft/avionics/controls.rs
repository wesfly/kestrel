pub mod fly_by_wire;
pub mod interfaces;

use bevy::prelude::*;

use crate::{
    aircraft::{
        Aircraft,
        avionics::controls::interfaces::InterfaceAction,
        engine::Engine,
        landing_gear::{LandingGearCommand, LandingGearCommands, LandingGearStatus},
        lights::Light,
    },
    input::ControlInputs,
    utils::{Sides, ToSides},
};

/// This is  a marker for all aircraft not using fly-by-wire.
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
    }
}

#[derive(Message, Debug, Clone)]
pub struct TriggerAction(pub InterfaceAction);

#[derive(Message, Debug, Clone)]
pub struct ScriptButtonPressed {
    pub name: String,
}

pub fn run_actions(
    mut requests: MessageReader<TriggerAction>,
    mut aircraft: Single<&mut Aircraft>,
    mut engines: Query<&mut Engine>,
    mut lights: Query<&mut Light>,
    mut gear_commands: MessageWriter<LandingGearCommand>,
    mut script_presses: MessageWriter<ScriptButtonPressed>,
) {
    for TriggerAction(action) in requests.read() {
        match action {
            InterfaceAction::Engine => {
                engines.iter_mut().for_each(|mut e| e.toggle());
            }
            InterfaceAction::ParkingBrake => aircraft.systems.parking_brake ^= true,
            InterfaceAction::LandingGear => {
                if matches!(
                    *&aircraft.landing_gear,
                    LandingGearStatus::Deployed | LandingGearStatus::Retracted
                ) {
                    gear_commands.write(LandingGearCommand(LandingGearCommands::Toggle));
                }
            }
            InterfaceAction::Script(name) => {
                let mut split = name.split(".");
                let Some(first) = split.next() else {
                    script_presses.write(ScriptButtonPressed { name: name.clone() });
                    return;
                };

                if first == "lights" {
                    if let Some(name) = split.next() {
                        lights
                            .iter_mut()
                            .find(|l| l.name() == name)
                            .map(|mut l| l.toggle());
                    }
                }
            }
        }
    }
}
