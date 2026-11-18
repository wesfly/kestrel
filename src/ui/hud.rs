use avian_fdm::components::FlightState;
use bevy::prelude::*;

use crate::{AppState, input::ControlInputs, utils::acceleration::PointAccel};

#[derive(Component)]
pub enum UiHud {
    Altitude,
    Throttle,
    Velocity,
    GLoad,
}

impl UiHud {
    pub fn spawn(mut commands: Commands) {
        commands.spawn((
            DespawnOnExit(AppState::InGame),
            Node {
                position_type: PositionType::Absolute,
                bottom: px(10.0),
                left: px(10.0),
                ..default()
            },
            Text::new("Loading"),
            UiHud::Altitude,
        ));
        commands.spawn((
            DespawnOnExit(AppState::InGame),
            Node {
                position_type: PositionType::Absolute,
                bottom: px(10.0),
                right: px(10.0),
                ..default()
            },
            Text::new("Loading"),
            UiHud::Throttle,
        ));
        commands.spawn((
            DespawnOnExit(AppState::InGame),
            Node {
                position_type: PositionType::Absolute,
                bottom: px(50.0),
                right: px(10.0),
                ..default()
            },
            Text::new("Loading"),
            UiHud::Velocity,
        ));

        #[cfg(debug_assertions)]
        commands.spawn((
            DespawnOnExit(AppState::InGame),
            Node {
                position_type: PositionType::Absolute,
                bottom: px(50.0),
                left: px(10.0),
                ..default()
            },
            Text::new("Loading"),
            UiHud::GLoad,
        ));
    }

    pub fn update(
        query: Query<(&mut Text, &UiHud)>,
        aircraft: Single<&FlightState>,
        input: Res<ControlInputs>,
        accels: Query<(&PointAccel, &Name)>,
    ) {
        let flight_state = *aircraft;
        for (mut text, ui_hud_component) in query {
            let string = match ui_hud_component {
                UiHud::Altitude => {
                    format!("Altitude: {}m", flight_state.altitude_m.to_string())
                }
                UiHud::Throttle => format!("{:.1}", input.throttle),
                UiHud::GLoad => {
                    let g_text = accels
                        .iter()
                        .find(|(_, name)| name.as_str() == "cockpit")
                        .map(|(p, _)| format!("{:.1}G", p.local.dot(Vec3::NEG_Z) / 9.81))
                        .unwrap_or_else(|| "--".to_string());

                    g_text
                }
                UiHud::Velocity => {
                    format!("{:?} km/h", flight_state.airspeed_ms)
                }
            };
            text.0 = string;
        }
    }
}
