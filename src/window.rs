use crate::ui::notifications::{GameNotification, GameNotificationUrgency};
use bevy::{
    prelude::*,
    render::view::screenshot::{Capturing, Screenshot, save_to_disk},
    window::{CursorIcon, SystemCursorIcon, WindowMode},
};

pub mod cursor;

pub fn trigger_screenshot(
    mut commands: Commands,
    input: Res<ButtonInput<KeyCode>>,
    mut notifications: MessageWriter<GameNotification>,
) {
    if input.just_pressed(KeyCode::F3) {
        let now = chrono::Local::now();
        let path = format!(".user/screenshots/screenshot-{:?}.png", now);

        let body = format!("Saving to {path}");
        notifications.write(GameNotification {
            urgency: GameNotificationUrgency::Info,
            title: "Screenshot captured".to_string(),
            body,
        });
        commands
            .spawn(Screenshot::primary_window())
            .observe(save_to_disk(path));
    }
}

pub fn screenshot_saving(
    mut commands: Commands,
    screenshot_saving: Query<Entity, With<Capturing>>,
    window: Single<Entity, With<Window>>,
) {
    match screenshot_saving.iter().count() {
        0 => {
            commands.entity(*window).remove::<CursorIcon>();
        }
        x if x > 0 => {
            commands
                .entity(*window)
                .insert(CursorIcon::from(SystemCursorIcon::Progress));
        }
        _ => {}
    }
}

pub fn toggle_fullscreen(input: Res<ButtonInput<KeyCode>>, mut windows: Query<&mut Window>) {
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
