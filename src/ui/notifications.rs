use crate::DespawnAfter;
use bevy::prelude::*;

pub struct NotificationPlugin;
impl Plugin for NotificationPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, GameNotification::spawn_notifications)
            .add_systems(Startup, GameNotification::setup_container)
            .add_message::<GameNotification>();
    }
}

#[derive(Debug)]
pub enum GameNotificationUrgency {
    Error,
    Warning,
    Info,
}

impl GameNotificationUrgency {
    fn background(&self) -> Color {
        match self {
            Self::Error => Color::srgb(0.65, 0.12, 0.12),
            Self::Warning => Color::srgb(0.85, 0.60, 0.08),
            Self::Info => Color::srgb(0.12, 0.35, 0.65),
        }
    }

    fn text_color(&self) -> Color {
        match self {
            Self::Warning => Color::srgb(0.08, 0.08, 0.08),
            _ => Color::WHITE,
        }
    }
}

#[derive(Message, Debug, Component)]
pub struct GameNotification {
    pub urgency: GameNotificationUrgency,
    pub title: String,
    pub body: String,
}

#[derive(Component)]
struct NotificationContainer;

impl GameNotification {
    fn setup_container(mut commands: Commands) {
        commands.spawn((
            NotificationContainer,
            Node {
                position_type: PositionType::Absolute,
                bottom: px(10.0),
                left: px(10.0),
                max_width: percent(50),
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::FlexStart,
                row_gap: px(8.0),
                ..default()
            },
        ));
    }

    fn spawn_notifications(
        mut commands: Commands,
        mut messages: MessageReader<GameNotification>,
        container: Single<Entity, With<NotificationContainer>>,
    ) {
        for notification in messages.read() {
            commands.spawn((
                ChildOf(*container),
                DespawnAfter(Timer::from_seconds(10.0, TimerMode::Once)),
                Node {
                    padding: px(10.0).all(),
                    flex_direction: FlexDirection::Column,
                    align_items: AlignItems::Start,
                    ..default()
                },
                BackgroundColor(notification.urgency.background()),
                children![
                    (
                        Text::new(format!("{}", notification.title)),
                        TextColor(notification.urgency.text_color()),
                        TextFont {
                            font_size: FontSize::Px(24.0),
                            ..default()
                        },
                    ),
                    (
                        Text::new(format!("{}", notification.body)),
                        TextColor(notification.urgency.text_color()),
                        TextFont {
                            font_size: FontSize::Px(16.0),
                            ..default()
                        },
                    )
                ],
            ));
        }
    }
}
