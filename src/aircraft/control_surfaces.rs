use crate::aircraft::Sides;
use bevy::prelude::*;
use serde::Deserialize;

#[derive(Debug, Deserialize, Component, Reflect)]
#[reflect(Component)]
pub enum ControlSurfaces {
    Rudder,
    ElevatorLeft,
    ElevatorRight,
    AileronLeft,
    AileronRight,
    Custom(String),
}

#[derive(Debug, Deserialize, Component, Reflect)]
#[reflect(Component)]
pub struct ControlSurface {
    pub name: ControlSurfaces,
    pub speed: f32,
}

/// Angles in radians
#[derive(Default, Resource, Copy, Clone, Reflect)]
pub struct ControlSurfacesDeflection {
    // TODO: Remove custom control surfaces from struct
    pub canards: Sides<f32>,
    pub aileron: Sides<f32>,
    pub elevator: Sides<f32>,
    pub rudder: f32,
}
