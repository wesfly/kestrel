use crate::{
    CameraSettings, GameState, Settings,
    aircraft::avionics::buttons::{ButtonAction, TriggerAction},
    camera::CameraView,
    ui::UIMessage,
};
use bevy::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Resource, Serialize)]
pub struct ControlInputs {
    pub pitch: f32,
    pub yaw: f32,
    pub roll: f32,
    pub throttle: f32,
    pub ground_brakes: f32, // 0.0 to 1.0
}

#[derive(Serialize, Deserialize, Clone)]
pub struct GamepadSettings {
    enabled: bool,
    hotas: bool,
}

impl Default for GamepadSettings {
    fn default() -> Self {
        Self {
            enabled: false,
            hotas: false,
        }
    }
}

#[derive(Resource)]
pub struct Keymap {
    pub reset_camera: KeyCode,
    up: KeyCode,
    down: KeyCode,
    rudder_left: KeyCode,
    rudder_right: KeyCode,
    roll_left: KeyCode,
    roll_right: KeyCode,
    throttle_up: KeyCode,
    throttle_down: KeyCode,
    change_camera: KeyCode,
    toggle_gear: KeyCode,
    engine: KeyCode,
    parking_brake: KeyCode,
    ground_brake: KeyCode,

    pos_lights: KeyCode,
    strobe_lights: KeyCode,
    formation_lights: KeyCode,
    anti_col_lights: KeyCode,

    pause: KeyCode,
    menu: KeyCode,
}

impl Default for Keymap {
    fn default() -> Self {
        Self {
            up: KeyCode::KeyW,
            down: KeyCode::KeyS,
            rudder_left: KeyCode::KeyQ,
            rudder_right: KeyCode::KeyE,
            roll_left: KeyCode::KeyA,
            roll_right: KeyCode::KeyD,
            throttle_up: KeyCode::PageUp,
            throttle_down: KeyCode::PageDown,

            change_camera: KeyCode::KeyC,
            reset_camera: KeyCode::KeyR,

            toggle_gear: KeyCode::KeyG,
            engine: KeyCode::KeyM,

            parking_brake: KeyCode::Equal,
            ground_brake: KeyCode::KeyZ,

            pos_lights: KeyCode::Digit1,
            strobe_lights: KeyCode::Digit2,
            formation_lights: KeyCode::Digit3,
            anti_col_lights: KeyCode::Digit4,

            pause: KeyCode::KeyP,
            menu: KeyCode::Escape,
        }
    }
}

pub fn input_system(
    mut input: ResMut<ControlInputs>,
    keys: Res<ButtonInput<KeyCode>>,
    keymap: Res<Keymap>,
    settings: Res<Settings>,
    keyboard_input: Res<'_, ButtonInput<KeyCode>>,
    gp: Option<Single<&bevy::prelude::Gamepad>>,
    mut camera_settings: ResMut<CameraSettings>,
    mut trigger_actions: MessageWriter<TriggerAction>,
    mut game_state: ResMut<GameState>,
    mut messages: MessageWriter<UIMessage>,
) {
    if keyboard_input.just_pressed(keymap.change_camera) {
        match camera_settings.view {
            CameraView::Follow => camera_settings.view = CameraView::Cockpit,
            CameraView::Cockpit => camera_settings.view = CameraView::Tail,
            CameraView::Tail => camera_settings.view = CameraView::Follow,
        }
    }

    if keyboard_input.just_pressed(keymap.menu) {
        messages.write(UIMessage::SpawnMenu);
    }

    // Only read input for flight dynamics when the game is running
    if game_state.running {
        if keyboard_input.just_pressed(keymap.pause) {
            game_state.running = false;
        }

        if keyboard_input.just_pressed(keymap.toggle_gear) {
            trigger_actions.write(TriggerAction(ButtonAction::LandingGear));
        }
        if keyboard_input.just_pressed(keymap.formation_lights) {
            trigger_actions.write(TriggerAction(ButtonAction::Script(String::from(
                "lights.formation",
            ))));
        }
        if keyboard_input.just_pressed(keymap.strobe_lights) {
            trigger_actions.write(TriggerAction(ButtonAction::Script("lights.strobe".into())));
        }
        if keyboard_input.just_pressed(keymap.pos_lights) {
            trigger_actions.write(TriggerAction(ButtonAction::Script(
                "lights.position".into(),
            )));
        }
        if keyboard_input.just_pressed(keymap.anti_col_lights) {
            trigger_actions.write(TriggerAction(ButtonAction::Script(
                "lights.anti_collision".into(),
            )));
        }
        if keyboard_input.just_pressed(keymap.engine) {
            trigger_actions.write(TriggerAction(ButtonAction::Engine));
        }

        if keyboard_input.just_pressed(keymap.parking_brake) {
            trigger_actions.write(TriggerAction(ButtonAction::ParkingBrake));
        }
        if keyboard_input.pressed(keymap.ground_brake) {
            input.ground_brakes = input.ground_brakes.lerp(1.0, 0.1);
        } else {
            input.ground_brakes = input.ground_brakes.lerp(0.0, 0.1);
        };

        if settings.gamepad.enabled {
            if let Some(gamepad) = gp {
                handle_gamepad_input(&mut input, settings, gamepad);
            } else {
                handle_keyboard_input(keys, keymap, input);
            }
        } else {
            handle_keyboard_input(keys, keymap, input);
        };
    } else if keyboard_input.just_pressed(keymap.pause) {
        game_state.running = true;
    }
}

fn handle_gamepad_input(
    input: &mut ResMut<'_, ControlInputs>,
    settings: Res<'_, Settings>,
    gamepad: Single<'_, '_, &bevy::input::gamepad::Gamepad>,
) {
    let mut gamepad_input = ControlInputs {
        pitch: 0.0,
        roll: 0.0,
        yaw: 0.0,
        throttle: 0.0,
        ground_brakes: 0.0,
    };

    if settings.gamepad.hotas {
        if let (Some(x), Some(y)) = (
            gamepad.get(GamepadAxis::LeftStickX),
            gamepad.get(GamepadAxis::LeftStickY),
        ) {
            gamepad_input.pitch = -y;
            gamepad_input.roll = -x;
        }

        if gamepad.just_pressed(GamepadButton::DPadDown) {
            gamepad_input.throttle = -0.1;
        }
        if gamepad.just_pressed(GamepadButton::DPadUp) {
            gamepad_input.throttle = 0.1;
        }

        if gamepad.pressed(GamepadButton::DPadLeft) {
            gamepad_input.yaw = 1.0;
        }
        if gamepad.pressed(GamepadButton::DPadRight) {
            gamepad_input.yaw = -1.0;
        }
    } else {
        if let (Some(x), Some(y)) = (
            gamepad.get(GamepadAxis::RightStickX),
            gamepad.get(GamepadAxis::RightStickY),
        ) {
            gamepad_input.pitch = -y;
            gamepad_input.roll = -x;
        }
        if let (Some(x), Some(y)) = (
            gamepad.get(GamepadAxis::LeftStickX),
            gamepad.get(GamepadAxis::LeftStickY),
        ) {
            gamepad_input.throttle += y * 0.01;
            gamepad_input.yaw = -x;
        }
    }
    input.pitch = gamepad_input.pitch;
    input.roll = gamepad_input.roll;
    input.yaw = gamepad_input.yaw;
    input.throttle += gamepad_input.throttle;
    input.throttle = input.throttle.clamp(0., 1.);
}

fn handle_keyboard_input(
    keys: Res<ButtonInput<KeyCode>>,
    keymap: Res<Keymap>,
    mut input: ResMut<ControlInputs>,
) {
    let mut button_input = ControlInputs {
        pitch: 0.0,
        roll: 0.0,
        yaw: 0.0,
        throttle: 0.0,
        ground_brakes: 0.0,
    };
    if keys.pressed(keymap.up) {
        button_input.pitch = -1.0
    }
    if keys.pressed(keymap.down) {
        button_input.pitch = 1.0
    }
    if keys.pressed(keymap.roll_left) {
        button_input.roll = 1.0
    }
    if keys.pressed(keymap.roll_right) {
        button_input.roll = -1.0
    }

    if keys.pressed(keymap.rudder_left) {
        button_input.yaw = 1.0
    }
    if keys.pressed(keymap.rudder_right) {
        button_input.yaw = -1.0
    }

    if keys.just_pressed(keymap.throttle_up) {
        button_input.throttle = 0.1
    }
    if keys.just_pressed(keymap.throttle_down) {
        button_input.throttle = -0.1
    }

    input.pitch = button_input.pitch;
    input.roll = button_input.roll;
    input.yaw = button_input.yaw;
    input.throttle += button_input.throttle;
    input.throttle = input.throttle.clamp(0.0, 1.0);
}
