use avian3d::{
    physics_transform::{Position, Rotation},
    schedule::PhysicsSystems,
};
use bevy::{math::DVec3, prelude::*};
use big_space::grid::cell::CellCoord;

pub const CELL_SIZE: f64 = 100.0;

pub struct BigSpaceCompatibilityPlugin;
impl Plugin for BigSpaceCompatibilityPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            FixedPostUpdate,
            (
                sync_to_avian.in_set(PhysicsSystems::Prepare),
                sync_from_avian.in_set(TransformSystems::Propagate),
            ),
        );
    }
}

pub fn sync_to_avian(mut bodies: Query<(&CellCoord, &Transform, &mut Position, &mut Rotation)>) {
    for (cell, transform, mut pos, mut rot) in &mut bodies {
        let cell_diff = DVec3::new((cell.x) as f64, (cell.y) as f64, (cell.z) as f64);
        let position = transform.translation.as_dvec3() + cell_diff * CELL_SIZE;
        let rotation = transform.rotation.normalize();

        pos.0 = position;
        rot.0 = rotation;
    }
}

pub fn sync_from_avian(mut bodies: Query<(&Position, &mut CellCoord, &mut Transform, &Rotation)>) {
    for (pos, mut cell, mut transform, rot) in &mut bodies {
        let relative_pos = pos.0;

        let cell_offset_x = (relative_pos.x / CELL_SIZE).round() as i64;
        let cell_offset_y = (relative_pos.y / CELL_SIZE).round() as i64;
        let cell_offset_z = (relative_pos.z / CELL_SIZE).round() as i64;

        cell.x = cell_offset_x;
        cell.y = cell_offset_y;
        cell.z = cell_offset_z;
        transform.translation = (relative_pos
            - DVec3::new(
                cell_offset_x as f64 * CELL_SIZE,
                cell_offset_y as f64 * CELL_SIZE,
                cell_offset_z as f64 * CELL_SIZE,
            ))
        .as_vec3();
        transform.rotation = rot.normalize();
    }
}
