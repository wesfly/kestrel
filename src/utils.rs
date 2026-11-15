pub mod acceleration;

use bevy::prelude::*;
use serde::Deserialize;

#[derive(Reflect, Deserialize, Debug, Default, PartialEq)]
pub enum Axis3D {
    X,
    /// Up
    #[default]
    Y,
    Z,
}

#[derive(Copy, Clone, PartialEq, Deserialize, Debug, Default)]
pub struct Coord {
    pub lat: f32,
    pub lon: f32,
}

impl Coord {
    pub fn from(lat: f32, lon: f32) -> Self {
        Self { lat, lon }
    }
}

#[derive(Component)]
pub struct DespawnAfter(pub Timer);

impl DespawnAfter {
    pub(super) fn tick_despawn_after_timers(mut query: Query<&mut Self>, time: Res<Time>) {
        let delta = time.delta();
        query.iter_mut().for_each(|mut t| {
            t.0.tick(delta);
        });
    }
}

#[derive(Default, Clone, Copy, Debug, Reflect)]
pub struct Sides<T> {
    pub left: T,
    pub right: T,
}

pub trait ToSides {
    fn left_right(self) -> Sides<f32>;
}
impl ToSides for f32 {
    fn left_right(self) -> Sides<f32> {
        Sides {
            left: self,
            right: self,
        }
    }
}
