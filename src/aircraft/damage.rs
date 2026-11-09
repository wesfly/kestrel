use crate::{
    SimState,
    aircraft::damage::overload::{detect_overload, warn_on_overload},
};
use bevy::prelude::*;

mod overload;

pub enum DamageCriticality {
    NonCritical,
    Major,
    Catastrophic,
}

#[derive(Message)]
pub struct DamageMessage {
    cause: &'static str,
    description: Option<&'static str>,
    criticality: DamageCriticality,
}

pub struct DamagePlugin;
impl Plugin for DamagePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            (
                detect_overload.run_if(in_state(SimState::Running)),
                check_damage.run_if(in_state(SimState::Running)),
            ),
        )
        .add_message::<DamageMessage>()
        .add_observer(warn_on_overload);
    }
}

fn check_damage(
    mut commands: Commands,
    mut damage_messages: MessageReader<DamageMessage>,
    mut next_sim_state: ResMut<NextState<SimState>>,
) {
    for damage in damage_messages.read() {
        match damage.criticality {
            DamageCriticality::Catastrophic => {
                next_sim_state.set(SimState::Paused);
                commands.spawn(());
            }
            _ => {}
        }
    }
}
