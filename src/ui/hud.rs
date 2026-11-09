use crate::{
    AppState, EARTH_RADIUS,
    aircraft::{Aircraft, acceleration::PointAccel},
    input::ControlInputs,
};
use avian3d::dynamics::rigid_body::LinearVelocity;
use bevy::{math::DVec3, prelude::*};

#[derive(Component)]
pub enum UiHud {
    Altitude,
    Throttle,
    Velocity,
    GLoad,
}

impl UiHud {
    pub fn setup(mut commands: Commands) {
        commands.spawn((
            DespawnOnExit(AppState::InGame),
            Node {
                position_type: PositionType::Absolute,
                bottom: px(10.0),
                left: px(10.0),
                ..default()
            },
            Text::new("Altitude"),
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
            Text::new("Throttle"),
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
            Text::new("Velocity"),
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
            Text::new("G"),
            UiHud::GLoad,
        ));
    }
    pub fn update(
        query: Query<(&mut Text, &UiHud)>,
        aircraft: Single<(&Aircraft, &Transform, &avian3d::prelude::Position)>,
        input: Res<ControlInputs>,
        vel: Single<&LinearVelocity, With<Aircraft>>,
        accels: Query<(&PointAccel, &Name)>,
    ) {
        let (state, transform, position) = *aircraft;
        for (mut text, ui_hud_component) in query {
            let forward = match state.name.as_str() {
                "h134" => -transform.local_z(),
                _ => transform.local_x(),
            };
            let string = match ui_hud_component {
                UiHud::Altitude => {
                    format!(
                        "Altitude: {}m",
                        (position.length() - EARTH_RADIUS as f64)
                            .round()
                            .to_string()
                    )
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
                    format!("Velocity: {:?} km/h", (forward.dot(vel.0) * 3.6) as i32)
                }
            };
            text.0 = string;
        }
    }
}
