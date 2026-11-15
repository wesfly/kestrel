pub mod fly_by_wire;

use crate::aircraft::engine::Engine;
use crate::aircraft::landing_gear::{LandingGearCommand, LandingGearCommands, LandingGearStatus};
use crate::aircraft::lights::Light;
use crate::utils::Axis3D;
use crate::{
    aircraft::{Aircraft, Sides, ToSides},
    input::ControlInputs,
};
use bevy::picking::pointer::PointerButton;
use bevy::prelude::*;
use serde::Deserialize;

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
    }
}

/// # Control Types
///
/// - [`Biased`](https://en.wikipedia.org/wiki/Switch#Biased_switches): A switch
///   that is always in the same position unless actively pressed. After that it
///   will return to its default position.
/// - `Binary`: A normal toggle switch.
/// - `MultiPosition`: Like a toggle switch, but with more than 2 positions.
///   ```rust,no_run
///   InterfaceType::MultiPosition(number_of_positions)
///   ```
/// - `Rotary`: Like a multi-position switch, but rotary.
///   ```rust,no_run
///   InterfaceType::Rotary(number_of_positions)
///   ```
/// - `Continuous`: A continuous interface, like thrust levers or
///   potentiometers in electronics.
#[derive(Debug, Component, Deserialize, PartialEq, Reflect)]
pub enum InterfaceType {
    Biased,
    Binary,
    MultiPosition(u8),
    Rotary(u8),
    Continuous(Continuous),
}

/// # Continuous Interface Types
///
/// - `Linear`: Continuous interfaces that move along an axis.
/// - `Rotary`: Continuous interfaces that rotate around the object origin.
#[derive(Debug, Deserialize, PartialEq, Reflect)]
pub enum Continuous {
    Linear,
    Rotary,
}

#[derive(Debug, Component, Deserialize, PartialEq, Reflect)]
#[reflect(Component, Default)]
pub struct Interface {
    pub interface_type: InterfaceType,
    pub axis: Axis3D,
    pub operation: String,
    pub inverse: Option<bool>,
}

impl Default for Interface {
    fn default() -> Self {
        Self {
            interface_type: InterfaceType::Biased,
            axis: Axis3D::X,
            operation: String::new(),
            inverse: Some(false),
        }
    }
}

#[derive(Component, Clone, Debug, PartialEq)]
#[require(ButtonState)]
pub enum ButtonAction {
    Engine,
    ParkingBrake,
    LandingGear,
    Script(String),
}

impl ButtonAction {
    pub fn parse(name: &str) -> Self {
        match name {
            "engine" => Self::Engine,
            "park_brk" => Self::ParkingBrake,
            "landing_gear" => Self::LandingGear,
            other => Self::Script(other.to_owned()),
        }
    }

    /// Current state of a built-in action (what the switch should *show*).
    /// `None` for scripted actions, which own their state in `ButtonState`.
    ///
    /// Takes the components that back the switches. When another part of
    /// `Aircraft` becomes its own component it becomes one more parameter here
    /// and in `run_actions`, and the compiler points at both.
    fn is_on(&self, aircraft: &Aircraft, engine: &Engine, lights: &Vec<&Light>) -> Option<bool> {
        Some(match self {
            Self::Engine => engine.is_on(),
            Self::ParkingBrake => aircraft.systems.parking_brake,
            Self::LandingGear => aircraft.landing_gear == LandingGearStatus::Deployed,
            Self::Script(n) => {
                let mut split = n.split(".");
                let Some(first) = split.next() else {
                    return None;
                };

                if first == "lights" {
                    if let Some(name) = split.next() {
                        return lights.iter().find(|l| l.name() == name).map(|l| l.is_on());
                    }
                }
                return None;
            }
        })
    }
}

#[derive(Component, Default, Clone, Copy, Debug, PartialEq)]
pub struct ButtonState(pub bool);

/// Rotation the mesh was authored with. Switch poses are computed relative to this.
#[derive(Component, Clone, Copy, Debug)]
struct RestRotation(Quat);

#[derive(Message, Debug, Clone)]
pub struct TriggerAction(pub ButtonAction);

#[derive(Message, Debug, Clone)]
pub struct ScriptButtonPressed {
    pub name: String,
}

pub struct InterfacePlugin;

impl Plugin for InterfacePlugin {
    fn build(&self, app: &mut App) {
        app.add_message::<TriggerAction>()
            .add_message::<ScriptButtonPressed>()
            .add_observer(on_button_press)
            .add_systems(
                Update,
                (
                    init_buttons,
                    run_actions,
                    sync_builtin_state,
                    update_switch_visuals,
                )
                    .chain(),
            );
    }
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
            ButtonAction::Engine => {
                engines.iter_mut().for_each(|mut e| e.toggle());
            }
            ButtonAction::ParkingBrake => aircraft.systems.parking_brake ^= true,
            ButtonAction::LandingGear => {
                if matches!(
                    *&aircraft.landing_gear,
                    LandingGearStatus::Deployed | LandingGearStatus::Retracted
                ) {
                    gear_commands.write(LandingGearCommand(LandingGearCommands::Toggle));
                }
            }
            ButtonAction::Script(name) => {
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

/// Resolve the authored string into runtime components, once per button.
/// (`ButtonState` comes along automatically via `#[require]` on `ButtonAction`.)
fn init_buttons(
    mut commands: Commands,
    new: Query<(Entity, &Interface, &Transform), Added<Interface>>,
) {
    for (entity, button, transform) in &new {
        let action = ButtonAction::parse(&button.operation);

        if let ButtonAction::Script(name) = &action {
            if name.is_empty() {
                warn!("button {entity} has no operation set");
            } else {
                debug!("button `{name}` is not built in, presses go to scripts");
            }
        }

        commands
            .entity(entity)
            .insert((action, RestRotation(transform.rotation)));
    }
}

fn on_button_press(
    press: On<PointerPress>,
    buttons: Query<&ButtonAction>,
    mut actions: MessageWriter<TriggerAction>,
) {
    if press.button != PointerButton::Primary {
        return;
    }
    if let Ok(action) = buttons.get(press.entity) {
        actions.write(TriggerAction(action.clone()));
    }
}

fn sync_builtin_state(
    cockpit: Single<&Aircraft>,
    engine: Query<&Engine>,
    mut buttons: Query<(&ButtonAction, &mut ButtonState)>,
    lights: Query<&Light>,
) {
    let aircraft = *cockpit;

    for (action, mut state) in &mut buttons {
        let lights: &Vec<&Light> = &lights.iter().collect();
        if let Some(on) = action.is_on(
            aircraft,
            engine.iter().nth(0).unwrap_or(&Engine::default()),
            lights,
        ) {
            state.set_if_neq(ButtonState(on));
        }
    }
}

/// Visuals: pose is a pure function of `ButtonState`, so it can never drift.
/// `Changed` also fires on spawn, which sets the correct initial pose.
///
/// Convention: the mesh is authored in the "off" pose and "on" is +70 degrees
/// about local X. Set `inverse` in Blender for switches authored the other way.
fn update_switch_visuals(
    mut buttons: Query<
        (&Interface, &ButtonState, &RestRotation, &mut Transform),
        Changed<ButtonState>,
    >,
) {
    const SWITCH_TRAVEL_DEG: f32 = -40.0;

    for (button, state, rest, mut transform) in &mut buttons {
        if button.interface_type != InterfaceType::Binary {
            continue; // TODO: levers / momentary buttons
        }
        let on = state.0 ^ button.inverse.unwrap_or(false);
        let angle = if on {
            SWITCH_TRAVEL_DEG.to_radians()
        } else {
            0.0
        };
        transform.rotation = rest.0 * Quat::from_rotation_x(angle);
    }
}
