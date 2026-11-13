use crate::{
    AppState, Settings,
    addons::Addons,
    scenery::terrain::Coord,
    ui::{
        TEXT_FONT, TITLE_FONT, UI_SCALE,
        notifications::{GameNotification, GameNotificationUrgency},
    },
};
use bevy::{
    feathers::{
        containers::{group, group_body, group_header, subpane, subpane_body, subpane_header},
        controls::{
            FeathersCheckbox, FeathersListRow, FeathersListView, FeathersNumberInput, HardLimit,
            NumberInputPrecision,
        },
        display::label,
        theme::ThemedText,
    },
    prelude::*,
    text::FontSourceTemplate,
    ui::{Checked, Selected},
    ui_widgets::{ValueChange, listbox_update_selection},
};

const BORDER: UiRect = UiRect::all(Val::Px(2.0));
const BACKGROUND_COLOUR: Color = Color::WHITE;

#[derive(Component, Clone, Default)]
struct LocationSelector(Coord);

#[derive(Component, Default, Clone)]
pub struct Menu;

#[derive(Component, Clone, Debug, Default)]
pub struct AircraftSelector {
    pub name: String,
    pub variant: String,
}

#[derive(Resource)]
pub struct ChosenAircraft(pub Option<AircraftSelector>);

fn aircraft_selector(addons: Res<Addons>) -> impl Scene {
    let rows: Vec<_> = addons
        .aircraft
        .clone()
        .iter()
        .map(|a| {
            let aircraft_name = format!("{}", a.general.name);

            // TODO make multiple variants possible
            let variant = a
                .get_variants()
                .first()
                .cloned()
                .unwrap_or("default".to_string());

            bsn! {
                @FeathersListRow
                AircraftSelector {
                    name: aircraft_name,
                    variant
                }
                Node {
                    display: Display::Grid,
                    padding: UiRect::all(px(8)),
                    border: BORDER,
                }
                Children [
                    Text(format!("{}", a.description.name))
                    TextFont {
                        font: FontSourceTemplate::Handle(TITLE_FONT),
                        font_size: FontSize::Px(24.0),
                        weight: FontWeight(600)
                    }
                    --
                    Text(format!("{}", a.description.description))
                    TextFont {
                        font: FontSourceTemplate::Handle(TEXT_FONT),
                        font_size: FontSize::Px(14.0)
                    }
                ]
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
        }
        Children [
            @subpane()
            Children [
                @subpane_header() Children [
                    Text("Aircraft") ThemedText
                ]
                @subpane_body()
                Children [
                    @FeathersListView {
                        @rows: { Box::new(rows) as Box<dyn SceneList> }
                    }
                    Node {
                        min_height: px(100)
                    }
                    on(listbox_update_selection)
                    on(update_chosen_aircraft)
                ]
            ]
        ]
    }
}

fn update_chosen_aircraft(
    change: On<ValueChange<Entity>>,
    selectors: Query<&AircraftSelector>,
    mut chosen: ResMut<ChosenAircraft>,
) {
    if let Ok(selector) = selectors.get(change.value) {
        *chosen = ChosenAircraft(Some(selector.clone()));
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
        ("Athens, GR", Coord::from(37.984167, 23.728056)),
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
        ("Belgrade, RS", Coord::from(44.817778, 20.456944)),
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
                Children [
                    TextFont {
                        font: FontSourceTemplate::Handle(TEXT_FONT),
                        font_size: FontSize::Px(14.0)
                    }
                    Text({name.to_string()})
                ]
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

        Children [
            @subpane()
            Children [
                @subpane_header()
                Children [
                    Text("Scene")
                    TextFont {
                        font: FontSourceTemplate::Handle(TITLE_FONT),
                        font_size: FontSize::Px(20.0),
                        weight: FontWeight(600)
                    }
                ]
                @subpane_body() Children [
                    Node {
                        width: percent(100)
                    }
                    @group()
                    Children [
                        @group_header()
                        Children [
                            Text("Locations")
                            Node {
                                margin: UiRect::all(px(8))
                            }
                            TextFont {
                                font: FontSourceTemplate::Handle(TITLE_FONT),
                                font_size: FontSize::Px(18.0),
                                weight: FontWeight(600)
                            }
                        ]
                        --
                        @group_body()
                        Children [
                            @FeathersListView {
                                @rows: { Box::new(rows) as Box<dyn SceneList> }
                            }
                            Node {
                                max_height: vh(50),
                                width: percent(100)
                            }
                            on(listbox_update_selection)
                        ]
                    ]
                    --
                    @group()
                    Node {
                        width: percent(100)
                    }
                    Children [
                        @group_header()
                        Children [
                            Text("Settings")
                            Node {
                                margin: UiRect::all(px(8))
                            }
                            TextFont {
                                font: FontSourceTemplate::Handle(TITLE_FONT),
                                font_size: FontSize::Px(18.0),
                                weight: FontWeight(600)
                            }
                        ]
                        --
                        @group_body()
                        Children [
                            @FeathersCheckbox {
                                @caption: bsn! {
                                    Text("Buildings (experimental)")
                                    TextFont {
                                        font: FontSourceTemplate::Handle(TEXT_FONT),
                                        font_size: FontSize::Px(14.0)
                                    }
                                }
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
                            --
                            @label("Altitude (m) (INOP)") // TODO fix inoperable field
                            --
                            @FeathersNumberInput
                            NumberInputPrecision(2)
                            HardLimit::f32(0.0..=100.0)
                            Node {
                                flex_grow: 1.0,
                                max_width: px(100),
                            }
                        ]
                    ]
                ]
            ]
        ]
    }
}

fn ui_root(addons: Res<Addons>) -> impl Scene {
    bsn! {
        DespawnOnExit<AppState>(AppState::Menu)
        Menu
        Node {
            height: percent(100.0),
            width: percent(100.0)
        }
        Visibility
        Children [
            @aircraft_selector(addons)
            --
            @fly_button()
            --
            @location_menu()
        ]
    }
}

fn fly_button() -> impl Scene {
    bsn! {
        Node {
            position_type: PositionType::Absolute,
            display: Display::Flex,
            bottom: px(16),
            right: px(16),
            width: px(128. * UI_SCALE),
            height: px(128. * UI_SCALE),
            padding: UiRect::all(px(16))
            border: BORDER,
        }
        ZIndex(1)
        BorderColor::all(Color::BLACK)
        BackgroundColor(BACKGROUND_COLOUR)
        on(|
            _trigger: On<PointerClick>,
            aircraft_selector: Option<Single<&AircraftSelector, With<Selected>>>,
            location_selector: Option<Single<&LocationSelector, With<Selected>>>,
            mut settings: ResMut<crate::Settings>,
            mut next_app_state: ResMut<NextState<AppState>>,
            mut notifications: MessageWriter<GameNotification>,
        | {
            if !aircraft_selector.is_some() {
                notifications.write(
                    GameNotification {
                        urgency: GameNotificationUrgency::Error,
                        title: String::from("No aircraft selected"),
                        body: String::from("Select an aircraft from the aircraft selector on the left side"),
                    });
                return;
            }

            if let Some(loc_sel) = location_selector {
                settings.terrain.coord = loc_sel.0;
            }

            next_app_state.set(AppState::InGame);
        })
        Children [
            Node {
                position_type: PositionType::Relative,
                bottom: px(0),
                left: px(0)
            }
            Text("Fly")
            TextFont {
                font_size: FontSize::Px(42.0),
                font: FontSourceTemplate::Handle(TITLE_FONT),
                weight: FontWeight(700)
            }
            TextColor(Color::BLACK)
        ]
    }
}

impl Menu {
    pub fn spawn(mut commands: Commands, addons: Res<Addons>) {
        commands.spawn_scene(ui_root(addons));

        // Spawn menu camera
        commands.spawn((Camera2d::default(), DespawnOnExit(AppState::Menu)));
    }
}
