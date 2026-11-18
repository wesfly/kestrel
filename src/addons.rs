//! addons
//! |- aircraft
//!    |- some-aircraft
//!       |- config.toml <- Aircraft config
//!
//! Blender aircraft structure:
//! Empty with
//!  - `Aircraft` (kestrel)
//!  - `AircraftGeometry` (avian_fdm)
//! in the object section

use bevy::{asset::AssetPath, prelude::*};
use serde::Deserialize;
use std::{
    error::Error,
    fs, io,
    path::{Path, PathBuf},
};

pub const AIRCRAFT_DIR: &str = "addons/aircraft";
const CONFIG_FILE: &str = "config.toml";

pub struct AddonsPlugin;
impl Plugin for AddonsPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(Addons::init());
    }
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct AircraftConfig {
    pub general: General,
    pub description: Description,
    #[serde(skip)]
    pub dir: PathBuf,
    #[serde(skip)]
    pub variants: Vec<Variant>,
}

impl AircraftConfig {
    pub fn load(dir: &Path) -> Result<Self, Box<dyn Error>> {
        let contents = fs::read_to_string(dir.join(CONFIG_FILE))?;
        let mut config: Self = toml::from_str(&contents)?;

        config.dir = dir.to_path_buf();
        config.variants = fs::read_dir(config.dir.join("variants"))?
            .filter_map(|entry| {
                let path = entry
                    .inspect_err(|e| error!("Failed to read directory entry: {e}"))
                    .ok()?
                    .path()
                    .join("variant.toml");
                let content = fs::read_to_string(&path)
                    .inspect_err(|e| error!("Failed to find {path:?}: {e}"))
                    .ok()?;
                let variant: Variant = toml::from_str(&content)
                    .inspect_err(|e| error!("Failed to serialize {path:?}: {e}"))
                    .ok()?;

                Some(variant)
            })
            .collect();

        Ok(config)
    }
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct General {
    /// The aircraft name, in lowercase letters. Has to have the same name as
    /// the folder the `config.toml` file sits in.
    pub name: String,
    /// Versioning system for aircraft to check compatibility with current
    /// version of `kestrel`.
    #[allow(unused)]
    pub version: String,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct Variant {
    pub name: String,
    pub controls: Controls,
    pub model: Model,
    #[serde(skip)]
    pub dir: PathBuf,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct Description {
    pub name: String,
    pub description: String,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct Controls {
    pub fly_by_wire: bool,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct Model {
    pub file_ext: ModelFileType,
}

/// The 3D model formats are `.glb` and `.gltf`, other file formats aren't supported.
#[derive(Debug, Clone, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum ModelFileType {
    Glb,
    #[default]
    Gltf,
}

impl ModelFileType {
    pub fn to_str(&self) -> &'static str {
        match &self {
            Self::Glb => "glb",
            Self::Gltf => "gltf",
        }
    }
}

fn load_aircraft_configs(dir: impl AsRef<Path>) -> io::Result<Vec<AircraftConfig>> {
    let configs = fs::read_dir(dir)?
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| path.is_dir())
        .filter_map(|dir| match AircraftConfig::load(&dir) {
            Ok(config) => Some(config),
            Err(err) => {
                warn!("skipping {:?}: {err}", dir);
                None
            }
        })
        .collect();

    Ok(configs)
}

#[derive(Resource, Default, Clone)]
pub struct Addons {
    pub aircraft: Vec<AircraftConfig>,
}

impl Addons {
    fn init() -> Addons {
        let aircraft = load_aircraft_configs(AIRCRAFT_DIR).unwrap();
        Addons { aircraft }
    }

    pub fn get_asset_path(aircraft_name: &str, variant: &str) -> AssetPath<'static> {
        let variant = match variant {
            "" => "default",
            v => v,
        };

        AssetPath::from(format!("aircraft/{aircraft_name}/variants/{variant}/"))
            .with_source("addons")
    }

    pub fn get_model_path(aircraft_name: &str, variant: &str, model: &Model) -> AssetPath<'static> {
        let variant = match variant {
            "" => "default",
            v => v,
        };

        AssetPath::from(format!(
            "aircraft/{aircraft_name}/variants/{variant}/models/model.{}",
            model.file_ext.to_str()
        ))
        .with_source("addons")
    }

    pub fn get_absolute_path(&self, aircraft_name: &str, variant: &str) -> PathBuf {
        format!("addons/aircraft/{aircraft_name}/variants/{variant}/").into()
    }
}
