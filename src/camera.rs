use crate::{absolute_position, aircraft::Aircraft, bevy_to_aerospace_coords, input::Keymap};
use bevy::{
    anti_alias::taa::TemporalAntiAliasing,
    camera::{Exposure, Hdr},
    core_pipeline::tonemapping::Tonemapping,
    ecs::{lifecycle::HookContext, world::DeferredWorld},
    input::mouse::{AccumulatedMouseMotion, MouseScrollUnit, MouseWheel},
    light::AtmosphereEnvironmentMapLight,
    pbr::{AtmosphereSettings, ScreenSpaceReflections},
    post_process::{bloom::Bloom, motion_blur::MotionBlur},
    prelude::*,
};
use big_space::prelude::*;
use std::f32::consts::FRAC_PI_2;

pub struct AircraftCameraPlugin;
impl Plugin for AircraftCameraPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(CurrentCameraView(0))
            .insert_resource(CameraPositions(Vec::new()))
            .add_systems(Update, AircraftCamera::controller);
    }
}

/// The current camera view. `usize` is the index to the entry in `CameraPositions`.
#[derive(Resource, PartialEq, Eq, Debug)]
pub struct CurrentCameraView(usize);

impl CurrentCameraView {
    pub fn next(&mut self, cam_pos_len: usize) {
        if self.0 == cam_pos_len.wrapping_sub(1) {
            self.0 = 0
        } else {
            self.0 = self.0.wrapping_add(1)
        }
    }
    pub fn previous(&mut self, cam_pos_len: usize) {
        if self.0 == 0 {
            self.0 = cam_pos_len.wrapping_sub(1)
        } else {
            self.0 = self.0.wrapping_sub(1)
        }
    }
}

#[derive(Debug, Component)]
pub struct AircraftCamera {
    pub pitch_speed: f32,
    pub yaw_speed: f32,
}

impl Default for AircraftCamera {
    fn default() -> Self {
        Self {
            pitch_speed: 0.1,
            yaw_speed: 0.1,
        }
    }
}

#[derive(Debug, Default, Clone)]
pub struct CameraRotation {
    pub yaw: f32,
    pub pitch: f32,
}

#[derive(Component, Debug, PartialEq, Default, Clone, Reflect)]
#[reflect(Component)]
#[component(on_add = get_camera_positions)]
pub struct CameraPosition {
    name: String,
    view_type: ViewType,
}

#[derive(Debug, PartialEq, Default, Clone, Reflect)]
pub enum ViewType {
    Orbit(f32),
    #[default]
    LookFromPoint,
}

/// The Resource holding the `CameraPosition`, the transform of the camera, and
/// the optional `Vec3`, which is the point the camera orbits around.
#[derive(Resource)]
pub struct CameraPositions(pub Vec<(CameraPosition, CameraRotation, Transform, Vec3)>);

fn get_camera_positions(mut world: DeferredWorld, HookContext { entity, .. }: HookContext) {
    let Some(transform) = world.entity(entity).get::<Transform>().cloned() else {
        return;
    };
    let Some(cam_pos) = world.entity(entity).get::<CameraPosition>().cloned() else {
        return;
    };

    let Some(mut cam_positions) = world.get_resource_mut::<CameraPositions>() else {
        return;
    };

    cam_positions.0.push((
        cam_pos,
        CameraRotation::default(),
        transform,
        transform.translation,
    ));
}

impl AircraftCamera {
    pub fn new() -> impl Bundle {
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
            AircraftCamera {
                pitch_speed: 0.3,
                yaw_speed: 0.3,
            },
        )
    }

    pub fn controller(
        mut camera: Single<
            (
                &AircraftCamera,
                &mut Transform,
                &mut CellCoord,
                &mut Exposure,
            ),
            Without<Aircraft>,
        >,
        aircraft: Single<(&Aircraft, &Transform, &CellCoord)>,
        mouse_buttons: Res<ButtonInput<MouseButton>>,
        mouse_motion: Res<AccumulatedMouseMotion>,
        keyboard_input: Res<'_, ButtonInput<KeyCode>>,
        keymap: Res<Keymap>,
        mut projection: Single<&mut Projection, With<AircraftCamera>>,
        mut scroll_events: MessageReader<MouseWheel>,
        mut camera_positions: ResMut<CameraPositions>,
        time: Res<Time>,
        current_view: Res<CurrentCameraView>,
    ) {
        let Some((cam_pos, cam_rot, _cam_tf, cam_tl)) =
            camera_positions.0.get_mut(current_view.0 as usize)
        else {
            error!("Expected aircraft to have camera positions");
            return;
        };

        let (camera, ref mut cam_tf, ref mut cell_coord, ref mut exposure) = *camera;
        let (_aircraft, ac_tf, ac_cell) = *aircraft;

        **cell_coord = *ac_cell;

        let delta = mouse_motion.delta;
        let delta_pitch;
        let delta_yaw;
        if mouse_buttons.pressed(MouseButton::Right) {
            delta_pitch = -delta.y * camera.pitch_speed * time.delta_secs();
            delta_yaw = -delta.x * camera.yaw_speed * time.delta_secs();
        } else {
            delta_pitch = 0.0;
            delta_yaw = 0.0;
        }

        let surface_up = absolute_position(cell_coord, cam_tf.translation)
            .normalize()
            .as_vec3();

        match cam_pos.view_type {
            ViewType::LookFromPoint => {
                // TODO: Needs work
                cam_tf.translation = ac_tf.translation + ac_tf.rotation * cam_tl.clone();

                cam_tf.rotate_local_y(delta_yaw);
                cam_tf.rotate_local_x(delta_pitch);
            }
            ViewType::Orbit(d) => {
                cam_rot.yaw += delta_yaw;
                cam_rot.pitch =
                    (cam_rot.pitch + delta_pitch).clamp(-(FRAC_PI_2 - 0.01), FRAC_PI_2 - 0.01);

                let orbit_yaw = cam_rot.yaw;
                let orbit_pitch = cam_rot.pitch;

                let aircraft_forward = -(ac_tf.rotation * Vec3::X);

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

                cam_tf.translation = ac_tf.translation + orbit_dir * d;
                cam_tf.look_at(ac_tf.translation, surface_up);
            }
        };

        if keyboard_input.pressed(keymap.reset_camera) {
            **cam_tf = Transform::default();
        }

        if keyboard_input.pressed(keymap.increase_exposure) {
            exposure.ev100 -= 1.1 * time.delta_secs();
        } else if keyboard_input.pressed(keymap.decrease_exposure) {
            exposure.ev100 += 1.1 * time.delta_secs();
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
