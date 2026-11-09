use crate::aircraft::Aircraft;
use bevy::prelude::*;
use chrono::{DateTime, Datelike, TimeDelta, Timelike, Utc};
use chrono_tz::Tz;
use lazy_static::lazy_static;
use std::f32::consts::PI;
use tzf_rs::DefaultFinder;

pub struct GameTimePlugin;
impl Plugin for GameTimePlugin {
    fn build(&self, app: &mut App) {
        let now = Utc::now();
        app.insert_resource(GameTime {
            utc: DateTime::from(now),
            time_zone: Tz::UTC,
            local_time: now.with_timezone(&Tz::UTC),
        })
        .insert_resource(TimeZoneFinder(DefaultFinder::default()))
        .add_systems(
            Update,
            (GameTime::update, GameTime::change_manually, rotate_sun),
        );
    }
}

#[derive(Resource)]
pub struct TimeZoneFinder(pub DefaultFinder);

lazy_static! {
    static ref FINDER: DefaultFinder = DefaultFinder::new();
}

pub fn rotate_sun(time: Res<GameTime>, mut suns: Query<&mut Transform, With<DirectionalLight>>) {
    let date_time = &time.utc;

    let hour = date_time.hour() as f32
        + date_time.minute() as f32 / 60.0
        + date_time.second() as f32 / 3600.0;
    let day_of_year = date_time.ordinal() as f32;

    let gamma = (2.0 * PI / 365.0) * (day_of_year - 1.0 + (hour - 12.0) / 24.0);

    let decl = 0.006918 - 0.399912 * gamma.cos() + 0.070257 * gamma.sin()
        - 0.006758 * (2.0 * gamma).cos()
        + 0.000907 * (2.0 * gamma).sin()
        - 0.002697 * (3.0 * gamma).cos()
        + 0.001480 * (3.0 * gamma).sin();

    let hour_angle = (hour - 6.0) / 24.0 * (2.0 * PI);

    let y = decl.sin();
    let xz_radius = decl.cos();

    let x = xz_radius * hour_angle.cos();
    let z = xz_radius * hour_angle.sin();

    let sun_dir = Vec3::new(x, y, z).normalize();

    for mut transform in suns.iter_mut() {
        *transform = Transform::default().looking_to(-sun_dir, Vec3::Y);
    }
}

#[derive(Resource)]
pub struct GameTime {
    pub utc: DateTime<Utc>,
    pub time_zone: Tz,
    pub local_time: DateTime<Tz>,
}

impl GameTime {
    pub fn change_manually(mut time: ResMut<GameTime>, keys: Res<ButtonInput<KeyCode>>) {
        if keys.pressed(KeyCode::KeyH) {
            time.utc = time.utc.checked_sub_signed(TimeDelta::days(1)).unwrap();
        }
        if keys.pressed(KeyCode::KeyJ) {
            time.utc = time.utc.checked_sub_signed(TimeDelta::days(-1)).unwrap();
        }
        if keys.pressed(KeyCode::KeyK) {
            time.utc = time.utc.checked_sub_signed(TimeDelta::minutes(2)).unwrap();
        }
        if keys.pressed(KeyCode::KeyL) {
            time.utc = time.utc.checked_sub_signed(TimeDelta::minutes(-2)).unwrap();
        }
    }

    pub fn update(
        mut time: ResMut<GameTime>,
        delta_time: Res<Time>,
        aircraft: Single<&Aircraft>,
        timezone_finder: Res<TimeZoneFinder>,
    ) {
        let finder = &timezone_finder.0;

        let tz_name = finder.get_tz_name(aircraft.coord.lat as f64, aircraft.coord.long as f64);

        let parsed_tz: Tz = tz_name.parse().unwrap_or(chrono_tz::UTC);

        time.utc = time
            .utc
            .checked_add_signed(TimeDelta::milliseconds(
                delta_time.delta().as_millis() as i64
            ))
            .unwrap();

        time.time_zone = parsed_tz;
        time.local_time = time.utc.with_timezone(&parsed_tz);
    }
}
