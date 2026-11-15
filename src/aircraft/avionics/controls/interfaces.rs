use bevy::prelude::*;
use serde::Deserialize;

use crate::{
    aircraft::{
        Aircraft,
        avionics::controls::{ScriptButtonPressed, TriggerAction, run_actions},
        engine::Engine,
        landing_gear::LandingGearStatus,
        lights::Light,
    },
    utils::Axis3D,
};

pub struct InterfacePlugin;
impl Plugin for InterfacePlugin {
    fn build(&self, app: &mut App) {
        app.add_message::<TriggerAction>()
            .add_message::<ScriptButtonPressed>()
            .add_observer(on_interface_operation)
            .add_systems(
                Update,
                (
                    init_interfaces,
                    run_actions,
                    sync_builtin_state,
                    update_interface_visuals,
                )
                    .chain(),
            );
    }
}

/// # Interface Types
///
/// - [`Biased`](https://en.wikipedia.org/wiki/Switch#Biased_switches): A
///   button that is always in the same position unless actively pressed. After
///   that it will return to its default position.
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

/// Rotation the mesh was authored with. Switch poses are computed relative to this.
#[derive(Component, Clone, Copy, Debug)]
struct RestRotation(Quat);

#[derive(Component, Default, Clone, Copy, Debug, PartialEq)]
pub struct InterfaceState(pub bool);

#[derive(Component, Clone, Debug, PartialEq)]
#[require(InterfaceState)]
pub enum InterfaceAction {
    Engine,
    ParkingBrake,
    LandingGear,
    Script(String),
}

impl InterfaceAction {
    pub fn parse(name: &str) -> Self {
        match name {
            "engine" => Self::Engine,
            "park_brk" => Self::ParkingBrake,
            "landing_gear" => Self::LandingGear,
            other => Self::Script(other.to_owned()),
        }
    }

    /// Current state of a built-in action (what the switch should *show*).
    /// `None` for scripted actions, which own their state in `InterfaceState`.
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

/// Resolve the authored string into runtime components, once per interface.
/// (`ButtonState` comes along automatically via `#[require]` on `ButtonAction`.)
fn init_interfaces(
    mut commands: Commands,
    new: Query<(Entity, &Interface, &Transform), Added<Interface>>,
) {
    for (entity, interface, transform) in &new {
        let action = InterfaceAction::parse(&interface.operation);

        if let InterfaceAction::Script(name) = &action {
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

fn on_interface_operation(
    press: On<PointerPress>,
    buttons: Query<&InterfaceAction>,
    mut actions: MessageWriter<TriggerAction>,
) {
    if press.button != PointerButton::Primary {
        return;
    }
    if let Ok(action) = buttons.get(press.entity) {
        actions.write(TriggerAction(action.clone()));
    }
}

/// Visuals: pose is a pure function of `InterfaceState`, so it can never drift.
/// `Changed` also fires on spawn, which sets the correct initial pose.
fn update_interface_visuals(
    mut buttons: Query<
        (&Interface, &InterfaceState, &RestRotation, &mut Transform),
        Changed<InterfaceState>,
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

fn sync_builtin_state(
    cockpit: Single<&Aircraft>,
    engine: Query<&Engine>,
    mut buttons: Query<(&InterfaceAction, &mut InterfaceState)>,
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
            state.set_if_neq(InterfaceState(on));
        }
    }
}
