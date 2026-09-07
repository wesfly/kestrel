use super::AircraftState;
use crate::aircraft::breeze::landing_gear::{LandingGearCommand, LandingGearStatus};
use bevy::picking::hover::PickingInteraction;
use bevy::prelude::*;
use bevy::window::PrimaryWindow;
use bevy::{
    picking::{hover::HoverMap, pointer::PointerId},
    window::CursorIcon,
};
use serde::Deserialize;

#[derive(Component, Clone, Debug, Deserialize, PartialEq, Reflect)]
pub enum InterfaceOperation {
    AntiColLt,
    Engine,
    PositionLt,
    StrobeLt,
    FormationLt,
    Apu,
    LdgGear,
    ParkBrk,
}

#[derive(Debug, Component, Deserialize, PartialEq, Reflect)]
pub enum InterfaceType {
    Switch,
    Button,
    Lever,
}

#[derive(Debug, Component, Deserialize, PartialEq, Reflect)]
#[reflect(Component, Default)]
#[type_path = "skein"]
pub struct Button {
    pub interface_type: InterfaceType,
    pub operation: Option<InterfaceOperation>,
    pub inverse: Option<bool>,
}

impl Default for Button {
    fn default() -> Self {
        Self {
            interface_type: InterfaceType::Button,
            operation: None,
            inverse: Some(false),
        }
    }
}

#[derive(Message)]
pub struct ButtonMessages(pub InterfaceOperation);

pub fn update_cursor(
    hover_map: Res<HoverMap>,
    mouse_input: Res<ButtonInput<MouseButton>>,
    window_query: Query<Entity, With<PrimaryWindow>>,
    mut commands: Commands,
    grabbable_query: Query<&Button>,
) {
    let Ok(window_entity) = window_query.single() else {
        return;
    };

    let is_hovering_target = hover_map
        .get(&PointerId::Mouse)
        .map(|hovered_entities| {
            hovered_entities
                .keys()
                .any(|entity| grabbable_query.contains(*entity))
        })
        .unwrap_or(false);

    if is_hovering_target {
        if mouse_input.pressed(MouseButton::Left) {
            commands
                .entity(window_entity)
                .insert(CursorIcon::System(bevy::window::SystemCursorIcon::Grab));
        } else {
            commands
                .entity(window_entity)
                .insert(CursorIcon::System(bevy::window::SystemCursorIcon::Pointer));
        }
    } else {
        commands
            .entity(window_entity)
            .insert(CursorIcon::System(bevy::window::SystemCursorIcon::Default));
    }
}

impl Button {
    pub fn press_system(
        query: Query<(&Button, &PickingInteraction), Changed<PickingInteraction>>,
        mut messages: MessageWriter<ButtonMessages>,
    ) {
        for (button, interaction) in &query {
            if *interaction == PickingInteraction::Pressed {
                if let Some(op) = &button.operation {
                    messages.write(ButtonMessages(op.clone()));
                }
            }
        }
    }

    pub fn listener(
        mut query_tf_button: Query<(&mut Transform, &Button)>,
        mut state: ResMut<AircraftState>,
        mut messages: MessageReader<ButtonMessages>,
        ldg_gear_status: Res<LandingGearStatus>,
        mut landing_gear_messages: MessageWriter<LandingGearCommand>,
    ) {
        for message in messages.read() {
            let interface_op = &message.0;
            let (bool, _) = match interface_op {
                InterfaceOperation::Engine => {
                    (Some(state.engine.on), state.engine.on = !state.engine.on)
                }
                InterfaceOperation::AntiColLt => (
                    Some(state.lights.anti_col),
                    state.lights.anti_col = !state.lights.anti_col,
                ),
                InterfaceOperation::PositionLt => {
                    (Some(state.lights.pos), state.lights.pos = !state.lights.pos)
                }
                InterfaceOperation::StrobeLt => (
                    Some(state.lights.strobe),
                    state.lights.strobe = !state.lights.strobe,
                ),
                InterfaceOperation::FormationLt => (
                    Some(state.lights.form),
                    state.lights.form = !state.lights.form,
                ),
                InterfaceOperation::LdgGear => {
                    let do_not_change_lever_pos;
                    let (a1, _) = match *ldg_gear_status {
                        LandingGearStatus::Deploying => {
                            (Some(true), do_not_change_lever_pos = true)
                        }
                        LandingGearStatus::Deployed => (
                            Some(state.landing_gear_deployed),
                            do_not_change_lever_pos = false,
                        ),
                        LandingGearStatus::Retracting => {
                            (Some(true), do_not_change_lever_pos = true)
                        }
                        LandingGearStatus::Retracted => (
                            Some(state.landing_gear_deployed),
                            do_not_change_lever_pos = false,
                        ),
                    };
                    if do_not_change_lever_pos {
                        continue;
                    }
                    (a1, {
                        landing_gear_messages.write(LandingGearCommand(
                            super::breeze::landing_gear::LandingGearCommands::Toggle,
                        ));
                    })
                }

                InterfaceOperation::ParkBrk => (
                    Some(state.parking_brake),
                    state.parking_brake = !state.parking_brake,
                ),
                _ => (None, ()),
            };

            for (mut transform, button) in &mut query_tf_button {
                const SWITCH_ANGLE_LIMIT: f32 = 70.0;
                if let InterfaceType::Switch = button.interface_type
                    && let Some(mut bool) = bool
                {
                    if let Some(inverse) = button.inverse
                        && inverse
                    {
                        bool = !bool
                    }

                    let angle = match bool {
                        true => -SWITCH_ANGLE_LIMIT,
                        false => SWITCH_ANGLE_LIMIT,
                    };
                    if let Some(op) = &button.operation {
                        if *interface_op == *op {
                            transform.rotate_local_x(angle.to_radians());
                        }
                    }
                }
            }
        }
    }
}
