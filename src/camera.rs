use crate::{
    absolute_position,
    aircraft::{self, Aircraft, AircraftTypes},
    bevy_to_aerospace_coords,
    input::Keymap,
};
use bevy::{
    anti_alias::taa::TemporalAntiAliasing,
    camera::{Exposure, Hdr},
    core_pipeline::tonemapping::Tonemapping,
    input::mouse::{AccumulatedMouseMotion, MouseScrollUnit, MouseWheel},
    light::AtmosphereEnvironmentMapLight,
    pbr::{AtmosphereSettings, ScreenSpaceReflections},
    post_process::{bloom::Bloom, motion_blur::MotionBlur},
    prelude::*,
};
use big_space::prelude::*;
use std::f32::consts::{FRAC_PI_2, PI};

#[derive(Component)]
pub struct AircraftCamera;

#[derive(Debug)]
pub enum CameraView {
    Cockpit,
    Follow,
    Tail,
}

#[derive(Debug, Resource)]
pub struct CameraSettings {
    pub orbit_distance: f32,
    pub pitch_speed: f32,
    pub yaw_speed: f32,
    tail_default_position: Vec3,
    pub view: CameraView,
}

impl Default for CameraSettings {
    fn default() -> Self {
        Self {
            orbit_distance: 25.0,
            pitch_speed: 0.1,
            yaw_speed: 0.1,
            tail_default_position: Vec3 {
                x: 0.0,
                y: 4.0,
                z: 7.0,
            },
            view: CameraView::Follow,
        }
    }
}

#[derive(Debug)]
pub struct CameraRotation {
    pub yaw: f32,
    pub pitch: f32,
}

#[derive(Resource, Debug)]
pub struct CameraPosition {
    pub cockpit: Transform,
    pub follow: CameraRotation,
    pub tail: Transform,
}

impl Default for CameraPosition {
    fn default() -> Self {
        CameraPosition {
            cockpit: Transform::default(),
            follow: CameraRotation {
                yaw: std::f32::consts::PI,
                pitch: 0.1,
            },
            tail: Transform::default(),
        }
    }
}

impl AircraftCamera {
    pub fn spawn() -> impl Bundle {
        (
            FloatingOrigin,
            Camera3d::default(),
            AtmosphereSettings {
                rendering_method: bevy::pbr::AtmosphereMode::Raymarched,
                ..default()
            },
            AtmosphereEnvironmentMapLight::default(),
            Exposure { ev100: 13.0 },
            Tonemapping::AcesFitted,
            Bloom::NATURAL,
            Projection::from(PerspectiveProjection {
                fov: 50.0_f32.to_radians(),
                ..default()
            }),
            (
                Msaa::Off,
                TemporalAntiAliasing::default(),
                ScreenSpaceReflections {
                    min_perceptual_roughness: 0.0..0.0,
                    ..default()
                },
                Hdr,
                MotionBlur {
                    shutter_angle: 0.5,
                    samples: 2,
                },
            ),
            crate::camera::AircraftCamera,
        )
    }

    pub fn controller(
        mut camera: Single<
            (&mut Transform, &mut CellCoord),
            (With<AircraftCamera>, Without<Aircraft>),
        >,
        camera_settings: Res<CameraSettings>,
        aircraft: Single<(&Aircraft, &Transform, &CellCoord)>,
        mouse_buttons: Res<ButtonInput<MouseButton>>,
        mouse_motion: Res<AccumulatedMouseMotion>,
        keyboard_input: Res<'_, ButtonInput<KeyCode>>,
        keymap: Res<Keymap>,
        mut projection: Single<&mut Projection, With<AircraftCamera>>,
        mut scroll_events: MessageReader<MouseWheel>,
        mut camera_pos: ResMut<CameraPosition>,
        time: Res<Time>,
    ) {
        let (ref mut cam_tf, ref mut cell_coord) = *camera;
        let (aircraft, ac_tf, ac_cell) = *aircraft;

        **cell_coord = *ac_cell;

        let cockpit_default_position = match aircraft.aircraft_type {
            aircraft::AircraftTypes::Helicopter => Vec3 {
                x: 0.38,
                y: 1.2,
                z: -2.6,
            },
            aircraft::AircraftTypes::J3Cub => Vec3 {
                x: 0.2,
                y: 0.0,
                z: -0.5,
            },
            _ => Vec3 {
                x: 2.8,
                y: 0.0,
                z: -1.2,
            },
        };

        let delta = mouse_motion.delta;
        let delta_pitch;
        let delta_yaw;
        if mouse_buttons.pressed(MouseButton::Right) {
            delta_pitch = -delta.y * camera_settings.pitch_speed * time.delta_secs();
            delta_yaw = -delta.x * camera_settings.yaw_speed * time.delta_secs();
        } else {
            delta_pitch = 0.0;
            delta_yaw = 0.0;
        }

        let surface_up = absolute_position(cell_coord, cam_tf.translation)
            .normalize()
            .as_vec3();

        match camera_settings.view {
            CameraView::Cockpit => {
                let (yaw, pitch, roll) = camera_pos.cockpit.rotation.to_euler(EulerRot::YXZ);

                camera_pos.cockpit.rotation = Quat::from_euler(
                    EulerRot::YXZ,
                    yaw + delta_yaw,
                    (pitch + delta_pitch).clamp(-(FRAC_PI_2 - 0.01), FRAC_PI_2 - 0.01),
                    roll,
                );
                cam_tf.translation = ac_tf.translation + ac_tf.rotation * cockpit_default_position;
                cam_tf.rotation = match aircraft.aircraft_type {
                    AircraftTypes::Helicopter => ac_tf.rotation * camera_pos.cockpit.rotation,
                    _ => ac_tf.rotation * bevy_to_aerospace_coords() * camera_pos.cockpit.rotation,
                };
            }
            CameraView::Follow => {
                camera_pos.follow.yaw += delta_yaw;
                camera_pos.follow.pitch = (camera_pos.follow.pitch + delta_pitch)
                    .clamp(-(FRAC_PI_2 - 0.01), FRAC_PI_2 - 0.01);

                let orbit_yaw = camera_pos.follow.yaw;
                let orbit_pitch = camera_pos.follow.pitch;

                let aircraft_forward = match aircraft.aircraft_type {
                    AircraftTypes::Helicopter => ac_tf.rotation * Vec3::Z,
                    _ => -(ac_tf.rotation * Vec3::X),
                };

                let flat_forward = {
                    let f = aircraft_forward - surface_up * aircraft_forward.dot(surface_up);
                    if f.length_squared() < 1e-6 {
                        surface_up.any_orthonormal_vector()
                    } else {
                        f.normalize()
                    }
                };

                let yaw_rot = Quat::from_axis_angle(surface_up, orbit_yaw);
                let base_dir = yaw_rot * (-flat_forward);

                let pitch_axis = base_dir.cross(surface_up);
                let orbit_dir = if pitch_axis.length_squared() < 1e-6 {
                    base_dir
                } else {
                    (Quat::from_axis_angle(pitch_axis.normalize(), orbit_pitch) * base_dir)
                        .normalize()
                };

                cam_tf.translation = ac_tf.translation + orbit_dir * camera_settings.orbit_distance;
                cam_tf.look_at(ac_tf.translation, surface_up);
            }
            CameraView::Tail => {
                let (yaw, pitch, roll) = camera_pos.tail.rotation.to_euler(EulerRot::YXZ);

                camera_pos.tail.rotation = Quat::from_euler(
                    EulerRot::YXZ,
                    yaw + delta_yaw,
                    (pitch + delta_pitch).clamp(-(FRAC_PI_2 - 0.01), FRAC_PI_2 - 0.01),
                    roll,
                );
                cam_tf.translation = match aircraft.aircraft_type {
                    AircraftTypes::Helicopter => {
                        ac_tf.translation + ac_tf.rotation * camera_settings.tail_default_position
                    }
                    _ => {
                        ac_tf.translation
                            + ac_tf.rotation
                                * bevy_to_aerospace_coords()
                                * camera_settings.tail_default_position
                    }
                };

                cam_tf.rotation = match aircraft.aircraft_type {
                    AircraftTypes::Helicopter => ac_tf.rotation * camera_pos.tail.rotation,
                    _ => ac_tf.rotation * bevy_to_aerospace_coords() * camera_pos.tail.rotation,
                };
            }
        };

        if keyboard_input.pressed(keymap.reset_camera) {
            *camera_pos = CameraPosition::default();
        }

        let Projection::Perspective(perspective) = projection.as_mut() else {
            return;
        };

        for event in scroll_events.read() {
            match event.unit {
                MouseScrollUnit::Line => perspective.fov -= event.y * 0.05,
                MouseScrollUnit::Pixel => {}
            }
        }

        perspective.fov = perspective.fov.clamp(0.1, std::f32::consts::FRAC_PI_2);
    }
}

// https://github.com/evroon/bevy-open-world/blob/master/crates/bevy-terrain/src/camera.rs
pub fn rotate_sun(
    mut suns: Query<&mut Transform, With<DirectionalLight>>,
    time: Res<Time>,
    keys: Res<ButtonInput<KeyCode>>,
) {
    let mut sun_vert_rot_factor = 0.0;
    let mut sun_hor_rot_factor = 0.0;

    if keys.pressed(KeyCode::KeyH) {
        sun_vert_rot_factor -= 0.1;
    }
    if keys.pressed(KeyCode::KeyJ) {
        sun_vert_rot_factor += 0.1;
    }
    if keys.pressed(KeyCode::KeyK) {
        sun_hor_rot_factor -= 0.2;
    }
    if keys.pressed(KeyCode::KeyL) {
        sun_hor_rot_factor += 0.2;
    }

    suns.iter_mut().for_each(|mut tf| {
        tf.rotate_x(time.delta_secs() * PI * sun_vert_rot_factor);
        tf.rotate_y(time.delta_secs() * PI * sun_hor_rot_factor)
    });
}
