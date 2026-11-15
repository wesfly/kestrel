use crate::{AppState, SimState, addons::Addons, aircraft::Aircraft};
use bevy::prelude::*;
use mlua::{
    Lua, LuaSerdeExt,
    prelude::{LuaFunction, LuaResult, LuaValue},
};
use serde::{Deserialize, Serialize};

pub struct FlyByWirePlugin;
impl Plugin for FlyByWirePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            OnEnter(AppState::InGame),
            FlyByWire::load.run_if(not(resource_exists::<FlyByWire>)),
        )
        .add_systems(
            FixedUpdate,
            fly_by_wire
                .run_if(in_state(SimState::Running))
                .run_if(resource_exists::<FlyByWire>),
        );
    }
}

type FbwControlInputs = crate::input::ControlInputs;

#[derive(Serialize)]
struct Input {
    id: u32,
}

#[derive(Deserialize, Debug)]
struct FbwOutput {
    id: u32,
}

#[derive(Resource)]
pub struct FlyByWire {
    lua: Lua,
    process: LuaFunction,
}

impl FlyByWire {
    fn load(mut commands: Commands, aircraft: Single<&Aircraft>, addons: Res<Addons>) {
        let name = &aircraft.name;
        let variant = &aircraft.variant;
        let path = addons
            .get_absolute_path(name, variant)
            .join("scripts/fly_by_wire.lua");

        let lua = Lua::new();
        lua.load(&*path).exec().unwrap_or_else(|e| {
            error!("Failed to load fly_by_wire.lua: {e}. Disabling fly-by-wire.");
            commands.remove_resource::<FlyByWire>();
            return;
        });

        let Ok(process) = lua.globals().get("main") else {
            error!(
                "`main` function not found in `{}`.
                 Consider adding a `main` function to `fly_by_wire.lua`
                ",
                &path.display()
            );
            commands.remove_resource::<FlyByWire>();
            return;
        };

        commands.insert_resource(FlyByWire { lua, process });
    }
}

fn fly_by_wire(fbw: Res<FlyByWire>, input: Res<FbwControlInputs>) {
    info!("calling");
    let arg = fbw.lua.to_value(&input.into_inner()).unwrap();
    let ret: LuaValue = fbw.process.call(arg).unwrap();
    let result: LuaResult<FbwOutput> = fbw.lua.from_value(ret);
    info!("{result:?}");
}
