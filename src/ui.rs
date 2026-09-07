use crate::{
    ControlInputs, EARTH_RADIUS, GameState, RunOnceSystemList, Settings,
    aircraft::{Aircraft, AircraftState, AircraftTypes},
    scenery::terrain::Coord,
};
use avian3d::prelude::LinearVelocity;
use bevy::{
    feathers::{FeathersPlugins, containers::*, controls::*, display::label, theme::ThemedText},
    input_focus::InputFocus,
    prelude::*,
    ui::{Checked, Selected},
    ui_widgets::{Activate, ValueChange, listbox_update_selection},
    window::WindowMode,
};
use core::convert::Into;

#[derive(Component)]
pub struct MenuCamera;

#[derive(Component)]
pub enum UIHudComponent {
    Altitude,
    Throttle,
    Velocity,
}

#[derive(Component, Clone, Debug, Default)]
enum AircraftSelector {
    #[default]
    Breeze,
    Helicopter,
    J3Cub,
}

impl AircraftSelector {
    pub const BREEZE: Self = Self::Breeze;
    pub const J3CUB: Self = Self::J3Cub;
    pub const HELICOPTER: Self = Self::Helicopter;
}

#[derive(Component, Clone, Default)]
struct LocationSelector(Coord);

#[derive(Component, Default, Clone)]
pub struct Menu;

fn aircraft_selector() -> impl Scene {
    bsn! {
        Node {
            display: Display::Flex,
            flex_direction: FlexDirection::Column,
            align_items: AlignItems::Stretch,
            justify_content: JustifyContent::Start,
            padding: px(8),
            row_gap: px(8),
            width: percent(30),
            min_width: px(200),
        }
        Children [
            subpane() Children [
                subpane_header() Children [
                    (Text("Aircraft") ThemedText),
                ],
                subpane_body() Children [
                    @FeathersListView {
                        @rows: { bsn_list! [
                            @FeathersListRow
                            AircraftSelector::BREEZE
                            Node {
                                display: Display::Grid,
                                margin: UiRect::bottom(px(10))
                            }
                            Children
                            [
                                (Text("Breeze C F3") ThemedText),
                                (label("An aeroplane inspired by a French twin-engine, canard delta wing, multirole fighter aircraft."))
                            ],
                            @FeathersListRow
                            AircraftSelector::J3CUB
                            Node {
                                display: Display::Grid,
                                margin: UiRect::bottom(px(10))
                            }
                            Children [
                                (Text("J-3 Cub") ThemedText),
                                (label("This aeroplane is a classic light aircraft and probably older than you."))
                            ],
                            @FeathersListRow
                            AircraftSelector::HELICOPTER
                            Node {
                                display: Display::Grid,
                                margin: UiRect::bottom(px(10))
                            }
                            Children [
                                (Text("Helicopter H134") ThemedText),
                                (label("A fictional single-engine civil light utility helicopter."))
                            ],
                        ]}
                    }
                    Node {
                        min_height: px(100)
                    }
                    on(listbox_update_selection)

                ]
            ]
        ]
    }
}

fn location_menu() -> impl Scene {
    let mut locations: Vec<(&str, Coord)> = vec![
        ("Toulon, FR", Coord::from(43.12694, 5.93071)),
        ("Hobart, AU", Coord::from(-42.88369, 147.3287)),
        ("San Francisco, US", Coord::from(37.7922, -122.4385)),
        ("Cape Town, ZA", Coord::from(-33.9114, 18.5033)),
        ("Vancouver, CA", Coord::from(49.2920, -123.1416)),
        ("London, UK", Coord::from(51.5074, -0.1278)),
        ("Tokyo, JP", Coord::from(35.6762, 139.6503)),
        ("New York, US", Coord::from(40.7128, -74.0060)),
        ("Sydney, AU", Coord::from(-33.8688, 151.2093)),
        ("Paris, FR", Coord::from(48.8566, 2.3522)),
        ("Cairo, EG", Coord::from(30.0444, 31.2357)),
        ("Rio de Janeiro, BR", Coord::from(-22.9068, -43.1729)),
        ("Berlin, DE", Coord::from(52.5200, 13.4050)),
        ("Mumbai, IN", Coord::from(19.0760, 72.8777)),
        ("Zürich, CH", Coord::from(47.37445, 8.541039)),
        ("Lucerne, CH", Coord::from(47.052099, 8.30899)),
        ("Nürnberg, DE", Coord::from(49.45387, 11.0773)),
        ("Guăngzhōu Shì, CN", Coord::from(23.128864, 113.259009)),
        ("Amsterdam, NL", Coord::from(52.37403, 4.88969)),
        ("København, DK", Coord::from(55.676111, 12.568333)),
        ("Bucharest, RO", Coord::from(44.43225, 26.10626)),
        ("Madrid, ES", Coord::from(40.4169, -3.7033)),
        ("Αθήνα, GR", Coord::from(37.984167, 23.728056)),
        (
            "Wellington/Te Whanganui-a-Tara, NZ",
            Coord::from(-41.288889, 174.777222),
        ),
        ("Ottawa, CA", Coord::from(45.424722, -75.695)),
        ("Reykjavík, IS", Coord::from(64.145833, -21.9425)),
        ("Oslo, NO", Coord::from(59.913333, 10.738889)),
        ("Shanghai, CN", Coord::from(31.2325, 121.469167)),
        ("Roma, IT", Coord::from(41.893333, 12.482778)),
        ("Stockholm, SE", Coord::from(59.329444, 18.068611)),
        ("Melbourne/Narrm, AU", Coord::from(-37.814167, 144.963056)),
        (
            "Port Moresby/Pot Mosbi, PG",
            Coord::from(-9.478889, 147.149444),
        ),
        ("Honolulu, US", Coord::from(21.3, -157.85)),
        (
            "Mexico City/Ciudad de México, MX",
            Coord::from(19.433333, -99.133333),
        ),
        ("Hanoi/Hà Nội, VN", Coord::from(21.0, 105.85)),
        ("Seoul, KR", Coord::from(37.56, 126.99)),
        ("Osaka, JP", Coord::from(34.693889, 135.502222)),
        ("Innsbruck, AT", Coord::from(47.268333, 11.393333)),
        ("Prague/Praha, CZ", Coord::from(50.0875, 14.421389)),
        ("Kathmandu , NP", Coord::from(27.71, 85.32)),
        (
            "Dublin/Baile Átha Cliath, IE",
            Coord::from(53.35, -6.260278),
        ),
        ("Belgrade/Београд, RS", Coord::from(44.817778, 20.456944)),
        ("Calvi, FR", Coord::from(42.5686, 8.7569)),
        ("Catania, IT", Coord::from(37.5, 15.090278)),
        ("Nairobi, KE", Coord::from(-1.286389, 36.817222)),
        ("Jakarta, ID", Coord::from(-6.18, 106.83)),
        (
            "Brussels/Bruxelles/Brussel, BE",
            Coord::from(50.846667, 4.3525),
        ),
        ("Bogotá, CO", Coord::from(4.711111, -74.072222)),
    ];

    locations.sort_by_key(|(name, _)| *name);

    let rows: Vec<_> = locations
        .into_iter()
        .map(|(name, coord)| {
            bsn! {
                @FeathersListRow
                LocationSelector(coord)
                Children [(
                    ThemedText
                    Text({name.to_string()})
                )]
            }
        })
        .collect();

    bsn! {
        Node {
            display: Display::Flex,
            flex_direction: FlexDirection::Column,
            align_items: AlignItems::Stretch,
            justify_content: JustifyContent::Start,
            padding: px(8),
            row_gap: px(8),
            width: percent(30),
            min_width: px(200),
            position_type: PositionType::Absolute,
            top: px(10),
            right: px(10),
            max_height: percent(100),
            overflow: Overflow::clip_y(),
        }

        Children[
            subpane() Children [
                subpane_header() Children [
                    (Text("Location") ThemedText),
                ],
                subpane_body() Children [(
                    @FeathersListView {
                        @rows: { Box::new(rows) as Box<dyn SceneList> }
                    }
                    Node {
                        max_height: vh(75)
                    }
                    on(listbox_update_selection)

                ),
                (
                    @FeathersCheckbox {
                        @caption: bsn! { Text("Buildings (experimental)") ThemedText }
                    }
                    Checked
                    AccessibleLabel("Enable Buildings Checkbox")
                    on(
                        |
                            change: On<ValueChange<bool>>,
                            mut commands: Commands,
                            mut settings: ResMut<Settings>
                        | {
                            let mut checkbox = commands.entity(change.source);
                            if change.value {
                                checkbox.insert(Checked);
                                settings.buildings_enabled = true;
                            } else {
                                checkbox.remove::<Checked>();
                                settings.buildings_enabled = false;
                            }
                        }
                    )
                )]
            ],

        ]
    }
}

fn ui_root() -> impl Scene {
    bsn! {
        Menu
        Node {
            height: percent(100.0),
            width: percent(100.0)
        }
        Visibility
        Children [
            aircraft_selector(),
            fly_button(),
            location_menu(),
        ]
    }
}

fn fly_button() -> impl Scene {
    bsn! {
        Node {
            position_type: PositionType::Absolute,
            bottom: px(10.0),
            right: px(10.0),
        }
        @FeathersButton {
            @caption: bsn! {
                Text("Fly")
                ZIndex(1)
                ThemedText
                TextFont {
                    font_size: FontSize::Px(54.0)
                }
                Node {
                    padding: UiRect::horizontal(px(50)) // TODO: vertical axis doesn't work?
                }
            },
            @variant: ButtonVariant::Primary,
        }
        on(|
            _activate: On<Activate>,
            mut messages: MessageWriter<UIMessage>,
            aircraft_selector: Option<Single<&AircraftSelector, With<Selected>>>,
            location_selector: Option<Single<&LocationSelector, With<Selected>>>,
            mut settings: ResMut<crate::Settings>,
       | {
            if !aircraft_selector.is_some() {
                error!("No aircraft selected");
                return;
            }

            if let Some(loc_sel) = location_selector {
                settings.terrain.coord = loc_sel.0;
            }

            messages.write(UIMessage::SpawnScenery);
            messages.write(UIMessage::DespawnMenu);
            messages.write(UIMessage::SpawnUIHud);
            if let Some(ac_sel) = aircraft_selector {
                match **ac_sel {
                    AircraftSelector::Breeze => {
                        messages.write(UIMessage::SpawnBreeze);
                    }
                    AircraftSelector::J3Cub => {
                        messages.write(UIMessage::SpawnJ3Cub);
                    }
                    AircraftSelector::Helicopter => {
                        messages.write(UIMessage::SpawnHelicopter);
                    }
                }
            }
        })
    }
}

impl Menu {
    fn despawn(commands: &mut Commands, menu_query: Query<Entity, With<Menu>>) {
        for entity in menu_query {
            commands.entity(entity).try_despawn();
        }
    }

    pub fn spawn(
        mut commands: Commands,
        mut game_state: ResMut<GameState>,
        scene_query: Query<Entity, With<GlobalTransform>>, // all 3d objects
        ui_hud_query: Query<Entity, With<UIHudComponent>>,
    ) {
        commands.spawn((Camera2d::default(), MenuCamera, IsDefaultUiCamera));
        game_state.running = false;
        for entity in &scene_query {
            commands.entity(entity).try_despawn();
        }

        for entity in &ui_hud_query {
            commands.entity(entity).try_despawn();
        }

        commands.spawn_scene(ui_root());
    }
}

#[derive(Message)]
pub enum UIMessage {
    SpawnMenu,
    DespawnMenu,
    SpawnUIHud,
    SpawnScenery,
    SpawnHelicopter,
    SpawnBreeze,
    SpawnJ3Cub,
}

pub struct UI;
impl Plugin for UI {
    fn build(&self, app: &mut App) {
        app.init_resource::<InputFocus>()
            .add_message::<UIMessage>()
            .add_plugins(FeathersPlugins)
            .add_systems(Startup, Menu::spawn)
            .add_systems(Update, (Self::update_ui_hud, Self::ui_main_loop))
            .add_systems(FixedUpdate, toggle_fullscreen);
    }
}

impl UI {
    fn ui_main_loop(
        mut commands: Commands,
        mut messages: MessageReader<UIMessage>,
        menu_query: Query<Entity, With<Menu>>,
        systems: Res<RunOnceSystemList>,
    ) {
        for message in messages.read() {
            match message {
                UIMessage::DespawnMenu => Menu::despawn(&mut commands, menu_query),
                UIMessage::SpawnMenu => {
                    commands.run_system(systems.0["spawn_menu"]);
                }
                UIMessage::SpawnUIHud => commands.run_system(systems.0["spawn_ui_hud"]),
                UIMessage::SpawnScenery => {
                    commands.run_system(systems.0["setup_scene"]);
                }
                UIMessage::SpawnHelicopter => {
                    commands.run_system(systems.0["spawn_helicopter"]);
                }
                UIMessage::SpawnBreeze => {
                    commands.run_system(systems.0["setup_breeze"]);
                }
                UIMessage::SpawnJ3Cub => {
                    commands.run_system(systems.0["setup_j3cub"]);
                }
            }
        }
    }

    pub fn setup_ui_hud(mut commands: Commands) {
        commands.spawn((
            Node {
                position_type: PositionType::Absolute,
                bottom: px(10.0),
                left: px(10.0),
                ..default()
            },
            Text::new("Altitude"),
            UIHudComponent::Altitude,
        ));
        commands.spawn((
            Node {
                position_type: PositionType::Absolute,
                bottom: px(10.0),
                right: px(10.0),
                ..default()
            },
            Text::new("Throttle"),
            UIHudComponent::Throttle,
        ));

        commands.spawn((
            Node {
                position_type: PositionType::Absolute,
                bottom: px(50.0),
                right: px(10.0),
                ..default()
            },
            Text::new("Velocity"),
            UIHudComponent::Velocity,
        ));
    }

    fn update_ui_hud(
        query: Query<(&mut Text, &UIHudComponent)>,
        state: Res<AircraftState>,
        aircraft: Single<(&Transform, &avian3d::prelude::Position), With<Aircraft>>,
        input: Res<ControlInputs>,
        vel: Single<&LinearVelocity, With<Aircraft>>,
    ) {
        let (transform, position) = *aircraft;
        for (mut text, ui_hud_component) in query {
            let forward = match state.aircraft_type {
                AircraftTypes::Helicopter => -transform.local_z(),
                _ => transform.local_x(),
            };
            let string = match ui_hud_component {
                UIHudComponent::Altitude => {
                    format!(
                        "Altitude: {}m",
                        (position.length() - EARTH_RADIUS as f64)
                            .round()
                            .to_string()
                    )
                }
                UIHudComponent::Throttle => {
                    format!("Throttle: {}%", (input.throttle * 100.0).round())
                }
                UIHudComponent::Velocity => {
                    format!(
                        "Velocity: {:?} km/h",
                        (forward.dot(vel.0.as_vec3()) * 3.6) as i32
                    )
                }
            };
            text.0 = string;
        }
    }
}

fn toggle_fullscreen(input: Res<ButtonInput<KeyCode>>, mut windows: Query<&mut Window>) {
    if input.just_pressed(KeyCode::F11) {
        #[allow(clippy::expect_used)]
        let mut window = windows
            .single_mut()
            .expect("window to exist if keyboard inputs are picked up.");

        window.mode = match window.mode {
            WindowMode::Windowed => WindowMode::BorderlessFullscreen(MonitorSelection::Current),
            _ => WindowMode::Windowed,
        };
    }
}
