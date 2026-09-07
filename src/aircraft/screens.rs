use crate::{
    EARTH_RADIUS, M_S_TO_KTS, METRES_TO_FEET,
    aircraft::{Aircraft, alpha_deg},
    input::ControlInputs,
};
use avian3d::{math::PI, physics_transform::Position, prelude::LinearVelocity};
use bevy::{
    asset::RenderAssetUsages,
    camera::RenderTarget,
    color::palettes::css::{BLACK, BLUE, GREEN},
    ecs::{lifecycle::HookContext, world::DeferredWorld},
    prelude::*,
    render::render_resource::{Extent3d, TextureDimension, TextureFormat, TextureUsages},
};
use serde::Deserialize;

#[derive(Deserialize, Debug, Component, Reflect, Clone, Copy)]
#[reflect(Component)]
#[type_path = "skein"]
#[component(on_add = on_add_use_screen_material)]
pub enum Screens {
    Left,
    Right,
    Centre,
    Hud,
}

#[allow(dead_code)]
#[derive(Component)]
pub enum ScreenUiElement {
    Throttle,
    Altitude,
    AirspeedKts,
    SpeedMach,
    Alpha,
    Horizon,
}

pub fn on_add_use_screen_material(
    mut world: DeferredWorld,
    HookContext { entity, .. }: HookContext,
) {
    #[allow(clippy::expect_used)]
    let screens = *world
        .get::<Screens>(entity)
        .expect("on_add_use_screen_material requires a Screens component");

    let asset_server = world.resource::<AssetServer>().clone();

    let size = Extent3d {
        width: 1024,
        height: 1024,
        ..default()
    };
    let mut image = Image::new_fill(
        size,
        TextureDimension::D2,
        &[0, 0, 0, 0],
        TextureFormat::Bgra8UnormSrgb,
        RenderAssetUsages::default(),
    );
    image.texture_descriptor.usage =
        TextureUsages::TEXTURE_BINDING | TextureUsages::COPY_DST | TextureUsages::RENDER_ATTACHMENT;
    let image_handle = world.resource_mut::<Assets<Image>>().add(image);

    let material_handle = {
        let mut materials = world.resource_mut::<Assets<StandardMaterial>>();
        match screens {
            Screens::Hud => {
                let brightness = 200.0;
                materials.add(StandardMaterial {
                    emissive_texture: Some(image_handle.clone()),
                    emissive: LinearRgba::new(brightness, brightness, brightness, 1.0),
                    base_color: Color::linear_rgba(0.3, 0.15, 0.3, 0.4),
                    perceptual_roughness: 0.2,
                    alpha_mode: AlphaMode::Premultiplied,
                    ..default()
                })
            }
            _ => {
                let brightness = 5.0;
                materials.add(StandardMaterial {
                    emissive_texture: Some(image_handle.clone()),
                    emissive: LinearRgba::new(brightness, brightness, brightness, 1.0),
                    base_color: Color::BLACK,
                    perceptual_roughness: 0.2,
                    ..default()
                })
            }
        }
    };

    let mut commands = world.commands();

    let texture_camera = commands
        .spawn((
            Camera2d,
            Camera {
                order: -1,
                ..default()
            },
            RenderTarget::Image(image_handle.into()),
        ))
        .id();

    match screens {
        Screens::Hud => {
            const HUD_COLOUR: Color = Color::Srgba(GREEN);
            let text_bundle = (
                TextFont {
                    font_size: FontSize::Px(32.0),
                    font: asset_server.load("fonts/SourceCodePro-Bold.ttf").into(),
                    ..default()
                },
                TextColor(HUD_COLOUR),
            );
            commands
                .spawn((
                    Node {
                        width: percent(100),
                        height: percent(100),
                        flex_direction: FlexDirection::Column,
                        display: Display::Flex,
                        justify_content: JustifyContent::Center,
                        align_items: AlignItems::Center,
                        ..default()
                    },
                    UiTargetCamera(texture_camera),
                ))
                .with_children(|parent| {
                    parent.spawn((
                        Node {
                            position_type: PositionType::Absolute,
                            left: px(150.0),
                            ..default()
                        },
                        ScreenUiElement::AirspeedKts,
                        Text::new("loading..."),
                        text_bundle.clone(),
                    ));
                    parent.spawn((
                        Node {
                            position_type: PositionType::Absolute,
                            left: px(150.0),
                            bottom: px(400.0),
                            ..default()
                        },
                        ScreenUiElement::Alpha,
                        Text::new("loading..."),
                        text_bundle.clone(),
                    ));
                    parent.spawn((
                        Node {
                            position_type: PositionType::Absolute,
                            right: px(150.0),
                            ..default()
                        },
                        ScreenUiElement::Altitude,
                        Text::new("loading..."),
                        text_bundle.clone(),
                    ));
                    parent
                        .spawn((
                            Node {
                                position_type: PositionType::Absolute,
                                width: percent(50.0),
                                left: percent(25.0),
                                top: percent(50.0),
                                height: px(2.0),
                                ..default()
                            },
                            BackgroundColor(HUD_COLOUR),
                            ScreenUiElement::Horizon,
                        ))
                        .with_children(|parent| {
                            for i in -10..10 {
                                if i != 0 {
                                    let text = format!("{}", i * -10);
                                    let top = 360.0 * i as f32;
                                    parent.spawn((
                                        Node {
                                            position_type: PositionType::Absolute,
                                            right: percent(22.0),
                                            top: px(top - 20.0),
                                            ..default()
                                        },
                                        Text::new(&text),
                                        text_bundle.clone(),
                                    ));
                                    parent.spawn((
                                        Node {
                                            position_type: PositionType::Absolute,
                                            left: percent(12.0),
                                            top: px(top - 20.0),
                                            ..default()
                                        },
                                        text_bundle.clone(),
                                        Text::new(&text),
                                    ));
                                    for j in 0..3 {
                                        parent.spawn((
                                            Node {
                                                position_type: PositionType::Absolute,
                                                width: px(10.0),
                                                left: percent(35.0 - j as f32 * 5.0),
                                                top: px(top),
                                                height: px(2.0),
                                                ..default()
                                            },
                                            BackgroundColor(HUD_COLOUR),
                                        ));
                                        parent.spawn((
                                            Node {
                                                position_type: PositionType::Absolute,
                                                width: px(10.0),
                                                right: percent(35.0 + j as f32 * 5.0),
                                                top: px(top),
                                                height: px(2.0),
                                                ..default()
                                            },
                                            BackgroundColor(HUD_COLOUR),
                                        ));
                                    }
                                }
                            }
                        });
                });
        }
        _ => {
            let label = match screens {
                Screens::Left => ScreenUiElement::Throttle,
                Screens::Right => ScreenUiElement::AirspeedKts,
                Screens::Centre => ScreenUiElement::Altitude,
                Screens::Hud => unreachable!(),
            };
            commands
                .spawn((
                    Node {
                        width: percent(100),
                        height: percent(100),
                        flex_direction: FlexDirection::Column,
                        justify_content: JustifyContent::Center,
                        align_items: AlignItems::Center,
                        ..default()
                    },
                    BackgroundColor(BLACK.into()),
                    UiTargetCamera(texture_camera),
                ))
                .with_children(|parent| {
                    parent
                        .spawn((
                            Node {
                                position_type: PositionType::Absolute,
                                width: Val::Auto,
                                height: Val::Auto,
                                align_items: AlignItems::Center,
                                padding: px(50).all(),
                                border_radius: BorderRadius::all(px(20)),
                                ..default()
                            },
                            BackgroundColor(BLUE.into()),
                        ))
                        .with_children(|parent| {
                            parent.spawn((
                                label,
                                Text::new("loading..."),
                                TextFont {
                                    font_size: FontSize::Px(64.0),
                                    ..default()
                                },
                                TextColor::WHITE,
                            ));
                        });
                });
        }
    }

    commands
        .entity(entity)
        .insert(MeshMaterial3d(material_handle));
}

pub fn update_screens(
    query: Query<(Option<&mut Text>, &ScreenUiElement, &mut UiTransform)>,
    input_axis: Res<ControlInputs>,
    vel_tf: Single<(&LinearVelocity, &Transform, &Position), With<Aircraft>>,
) {
    let (velocity, transform, position) = *vel_tf;
    for (text, screen, mut ui_transform) in query {
        if let Some(mut text) = text {
            match screen {
                ScreenUiElement::Throttle => {
                    *text = Text::new(format!("Throttle:\n{:.1}%", input_axis.throttle * 100.0))
                }
                ScreenUiElement::AirspeedKts => {
                    *text = Text::new(format!(
                        "{} kts",
                        (transform.local_x().dot(velocity.0.as_vec3()) * M_S_TO_KTS) as i32
                    ))
                }
                ScreenUiElement::Altitude => {
                    *text = Text::new(format!(
                        "{} ft",
                        ((position.length() - EARTH_RADIUS as f64) * METRES_TO_FEET as f64) as i32
                    ))
                }
                ScreenUiElement::Alpha => {
                    let alpha_deg = alpha_deg(velocity, transform);
                    *text = Text::new(format!("AOA {:.2}", alpha_deg))
                }

                _ => *text = Text::new("todo!()"),
            }
        } else if let ScreenUiElement::Horizon = screen {
            let dist_from_center = position.as_vec3().length();
            let local_up = position.as_vec3() / dist_from_center;

            let dip = (EARTH_RADIUS / dist_from_center).clamp(-1.0, 1.0).acos();

            let up = transform.rotation.inverse() * local_up;
            let roll = f32::atan2(-up.z, up.y);
            ui_transform.rotation = Rot2::radians(-roll + (1.0 / 2.0 * PI) as f32);

            let forward = transform.local_x();
            let pitch = forward.dot(local_up).asin();

            let px_per_rad = 3100.0;
            ui_transform.translation.y = px((pitch + dip) * px_per_rad);
            ui_transform.translation.x = px(0.0);
        }
    }
}
