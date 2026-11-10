use crate::aircraft::Aircraft;
use bevy::prelude::*;
use serde::Deserialize;

#[derive(
    Debug,
    Deserialize,
    Clone,
    Copy,
    PartialEq,
    Eq,
    Reflect,
    Component,
    Hash
)]
#[reflect(Component)]
#[type_path = "skein"]
pub enum LandingGearElement {
    FrontLdgGear,
    FrontLdgGearDoorFront,
    FrontLdgGearDoorAftLeft,
    FrontLdgGearDoorAftRight,

    StarboardLdgGear,
    StarboardLdgGearDoorFront,
    StarboardLdgGearDoorAft,

    PortLdgGear,
    PortLdgGearDoorFront,
    PortLdgGearDoorAft,
}

pub enum LandingGearCommands {
    Toggle,
}

#[derive(Message)]
pub struct LandingGearCommand(pub LandingGearCommands);

#[derive(Deserialize, Debug, Default, Clone, Copy, Eq, PartialEq)]
pub enum LandingGearStatus {
    Deploying,
    Deployed,
    Retracting,
    #[default]
    Retracted,
}

#[derive(Default, Clone, Copy, PartialEq, Eq)]
pub enum LdgGearPhase {
    #[default]
    Phase1,
    Phase2,
    Phase3,
}

// Old transform, needs to be multiplied with bevy_to_aerospace_coords()
pub const LEFT_POS: Vec3 = Vec3::new(-1.3, -0.60, 2.1);
pub const RIGHT_POS: Vec3 = Vec3::new(1.3, -0.60, 2.1);
pub const NOSEWHEEL_POS: Vec3 = Vec3::new(0.0, -0.53, -2.9);

const LDG_GEAR_DEPLOY_SPD: f32 = 40.0_f32.to_radians();
const DOOR_DEPLOY_ANGLE: f32 = 80.0_f32.to_radians();
const WIDE_ANGLE: f32 = DOOR_DEPLOY_ANGLE + 20.0_f32.to_radians();

#[derive(Clone, Copy)]
enum Axis {
    X,
    Z,
}

type GearTarget = (LandingGearElement, Axis, f32);

use LandingGearElement::*;

fn deploy_phase1_targets() -> [GearTarget; 7] {
    [
        (FrontLdgGearDoorFront, Axis::Z, -WIDE_ANGLE),
        (FrontLdgGearDoorAftLeft, Axis::Z, -DOOR_DEPLOY_ANGLE),
        (FrontLdgGearDoorAftRight, Axis::Z, DOOR_DEPLOY_ANGLE),
        (PortLdgGearDoorFront, Axis::Z, WIDE_ANGLE),
        (PortLdgGearDoorAft, Axis::X, -DOOR_DEPLOY_ANGLE),
        (StarboardLdgGearDoorFront, Axis::Z, -WIDE_ANGLE),
        (StarboardLdgGearDoorAft, Axis::X, -DOOR_DEPLOY_ANGLE),
    ]
}

fn deploy_phase2_targets() -> [GearTarget; 3] {
    [
        (FrontLdgGear, Axis::Z, -90.0_f32.to_radians()),
        (StarboardLdgGear, Axis::X, -210.0_f32.to_radians()),
        (PortLdgGear, Axis::X, -210.0_f32.to_radians()),
    ]
}

fn deploy_phase3_targets() -> [GearTarget; 3] {
    [
        (FrontLdgGearDoorFront, Axis::Z, 0.0),
        (PortLdgGearDoorFront, Axis::Z, 0.0),
        (StarboardLdgGearDoorFront, Axis::Z, 0.0),
    ]
}

fn retract_phase1_targets() -> [GearTarget; 3] {
    [
        (FrontLdgGearDoorFront, Axis::Z, -WIDE_ANGLE),
        (PortLdgGearDoorFront, Axis::Z, WIDE_ANGLE),
        (StarboardLdgGearDoorFront, Axis::Z, -WIDE_ANGLE),
    ]
}

fn retract_phase2_targets() -> [GearTarget; 3] {
    [
        (FrontLdgGear, Axis::Z, 0.0),
        (StarboardLdgGear, Axis::X, 0.0),
        (PortLdgGear, Axis::X, 0.0),
    ]
}

fn retract_phase3_targets() -> [GearTarget; 7] {
    [
        (FrontLdgGearDoorFront, Axis::Z, 0.0),
        (FrontLdgGearDoorAftLeft, Axis::Z, 0.0),
        (FrontLdgGearDoorAftRight, Axis::Z, 0.0),
        (PortLdgGearDoorFront, Axis::Z, 0.0),
        (StarboardLdgGearDoorFront, Axis::Z, 0.0),
        (PortLdgGearDoorAft, Axis::X, 0.0),
        (StarboardLdgGearDoorAft, Axis::X, 0.0),
    ]
}

fn step_axis_towards(
    current: &mut f32,
    transform: &mut Transform,
    axis: Axis,
    target: f32,
    max_step: f32,
) -> bool {
    let diff = target - *current;
    let reached = diff.abs() <= max_step;
    let step = if reached {
        diff
    } else {
        max_step.copysign(diff)
    };

    match axis {
        Axis::X => transform.rotate_local_x(step),
        Axis::Z => transform.rotate_local_z(step),
    }
    *current += step;
    reached
}

fn step_phase(
    query: &mut Query<(&mut Transform, &LandingGearElement)>,
    angles: &mut std::collections::HashMap<LandingGearElement, f32>,
    targets: &[GearTarget],
    max_step: f32,
) -> bool {
    let mut all_reached = true;
    for (mut transform, element) in query.iter_mut() {
        let Some(&(_, axis, target)) = targets.iter().find(|(e, ..)| e == element) else {
            continue;
        };
        let current = angles.entry(*element).or_insert(0.0);
        if !step_axis_towards(current, &mut transform, axis, target, max_step) {
            all_reached = false;
        }
    }
    all_reached
}

#[derive(Component)]
pub struct LandingGear;
impl LandingGear {
    pub fn operate_landing_gear(
        mut landing_gear_messages: MessageReader<LandingGearCommand>,
        mut query: Query<(&mut Transform, &LandingGearElement)>,
        mut aircraft: Single<&mut Aircraft>,
        time: Res<Time>,
        mut phase: Local<LdgGearPhase>,
        mut hash_ldg: Local<std::collections::HashMap<LandingGearElement, f32>>,
    ) {
        if landing_gear_messages.read().last().is_some() {
            *phase = LdgGearPhase::Phase1;
            match *&aircraft.landing_gear {
                LandingGearStatus::Deployed => {
                    *&mut aircraft.landing_gear = LandingGearStatus::Retracting;
                }
                LandingGearStatus::Retracted => {
                    *&mut aircraft.landing_gear = LandingGearStatus::Deploying;
                }
                _ => {}
            };
        }

        match *&aircraft.landing_gear {
            LandingGearStatus::Deploying => {
                Self::deploy_landing_gear(
                    &mut query,
                    &mut aircraft,
                    &time,
                    &mut phase,
                    &mut hash_ldg,
                );
            }
            LandingGearStatus::Retracting => {
                Self::retract_landing_gear(
                    &mut query,
                    &mut aircraft.landing_gear,
                    &time,
                    &mut phase,
                    &mut hash_ldg,
                );
            }
            _ => {}
        }
    }

    pub fn deploy_landing_gear(
        query: &mut Query<(&mut Transform, &LandingGearElement)>,
        aircraft: &mut Aircraft,
        time: &Time,
        phase: &mut LdgGearPhase,
        mut hash_ldg: &mut std::collections::HashMap<LandingGearElement, f32>,
    ) {
        let max_step = LDG_GEAR_DEPLOY_SPD * time.delta_secs();

        let phase_done = match *phase {
            LdgGearPhase::Phase1 => {
                step_phase(query, &mut hash_ldg, &deploy_phase1_targets(), max_step)
            }
            LdgGearPhase::Phase2 => {
                step_phase(query, &mut hash_ldg, &deploy_phase2_targets(), max_step)
            }
            LdgGearPhase::Phase3 => {
                step_phase(query, &mut hash_ldg, &deploy_phase3_targets(), max_step)
            }
        };

        if !phase_done {
            return;
        }

        match *phase {
            LdgGearPhase::Phase1 => *phase = LdgGearPhase::Phase2,
            LdgGearPhase::Phase2 => *phase = LdgGearPhase::Phase3,
            LdgGearPhase::Phase3 => {
                aircraft.landing_gear = LandingGearStatus::Deployed;
            }
        }
    }

    pub fn retract_landing_gear(
        query: &mut Query<(&mut Transform, &LandingGearElement)>,
        status: &mut LandingGearStatus,
        time: &Time,
        phase: &mut LdgGearPhase,
        mut hash_ldg: &mut std::collections::HashMap<LandingGearElement, f32>,
    ) {
        let max_step = LDG_GEAR_DEPLOY_SPD * time.delta_secs();

        let phase_done = match *phase {
            LdgGearPhase::Phase1 => {
                step_phase(query, &mut hash_ldg, &retract_phase1_targets(), max_step)
            }
            LdgGearPhase::Phase2 => {
                step_phase(query, &mut hash_ldg, &retract_phase2_targets(), max_step)
            }
            LdgGearPhase::Phase3 => {
                step_phase(query, &mut hash_ldg, &retract_phase3_targets(), max_step)
            }
        };

        if !phase_done {
            return;
        }

        match *phase {
            LdgGearPhase::Phase1 => *phase = LdgGearPhase::Phase2,
            LdgGearPhase::Phase2 => *phase = LdgGearPhase::Phase3,
            LdgGearPhase::Phase3 => *status = LandingGearStatus::Retracted,
        }
    }
}
