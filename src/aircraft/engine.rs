use bevy::prelude::*;

#[derive(Default, Reflect)]
enum EngineRunningMode {
    #[default]
    Off,
    Idle,
    On,
}

#[derive(Component, Default, Reflect)]
#[reflect(Component, Default)]
pub struct Engine {
    mode: EngineRunningMode,
    rpm: f32,
    full_rpm: f32,
    spin_up_rate: f32,
}

impl Engine {
    pub fn on() -> Self {
        Self {
            mode: EngineRunningMode::On,
            rpm: 0.0,
            full_rpm: 14_000.0,
            spin_up_rate: 500.0,
        }
    }
    pub fn get_rpm(&self) -> f32 {
        self.rpm
    }

    pub fn is_on(&self) -> bool {
        match self.mode {
            EngineRunningMode::Off => false,
            _ => true,
        }
    }

    /// Temporary function for the time `EngineRunningMode::Idle` is not implemented
    // TODO
    pub fn toggle(&mut self) {
        if self.is_on() {
            self.mode = EngineRunningMode::Off;
        } else {
            self.mode = EngineRunningMode::On;
        }
    }

    pub fn update(query: Query<&mut Engine>, time: Res<Time>) {
        for mut engine in query {
            if engine.is_on() && engine.rpm <= engine.full_rpm {
                engine.rpm = engine
                    .rpm
                    .lerp(engine.full_rpm, engine.spin_up_rate * time.delta_secs());
            } else {
                engine.rpm = 0.0;
            }
        }
    }
}
