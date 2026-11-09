use super::DamageCriticality;
use crate::{
    aircraft::{acceleration::PointAccel, damage::DamageMessage},
    ui::notifications::{GameNotification, GameNotificationUrgency},
};
use bevy::prelude::*;

#[derive(Component)]
pub struct Overload {
    critical: bool,
}

pub fn detect_overload(
    mut commands: Commands,
    aircraft: Query<(Entity, &PointAccel, &Name, Has<Overload>)>,
) {
    let g = aircraft
        .iter()
        .find(|(_, _, name, _)| name.as_str() == "cockpit")
        .map(|(_, p, _, _)| p.local.dot(Vec3::NEG_Z) / 9.81)
        .unwrap_or_default();

    for (entity, _, _name, is_overloaded) in &aircraft {
        if !is_overloaded && g > 12.0 {
            commands
                .entity(entity)
                .insert(Overload { critical: g > 13.0 });
        } else if is_overloaded && g < 11.0 {
            commands.entity(entity).remove::<Overload>();
        }
    }
}

pub fn warn_on_overload(
    trigger: On<Add<Overload>>,
    query: Query<&Overload>,
    mut notifications: MessageWriter<GameNotification>,
    mut damage_messages: MessageWriter<DamageMessage>,
) {
    if let Ok(overload) = query.get(trigger.entity) {
        if overload.critical {
            damage_messages.write(DamageMessage {
                cause: "Overload",
                description: Some(
                    "You overloaded the aircraft, causing severe structural damgage.",
                ),
                criticality: DamageCriticality::Catastrophic,
            });
        }

        notifications.write(GameNotification {
            urgency: GameNotificationUrgency::Warning,
            title: "Overload".to_string(),
            body: "The aircraft suffers from G-overload. This will cause structural damage. Reduce the turn rate.".to_string(),
        });
    }
}
