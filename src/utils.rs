use bevy::{
    ecs::{
        component::Component,
        system::{Query, Res},
    },
    time::{Time, Timer},
};

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
