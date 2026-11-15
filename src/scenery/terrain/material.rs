use bevy::{
    asset::{Asset, Handle},
    image::Image,
    pbr::MaterialExtension,
    prelude::Vec3,
    reflect::TypePath,
    render::render_resource::AsBindGroup,
    shader::ShaderRef,
};

const SHADER_ASSET_PATH: &str = "shaders/terrain.wesl";

#[derive(Asset, TypePath, AsBindGroup, Debug, Clone)]
pub struct TerrainMaterial {
    // The normal map image.
    // Note that, like all normal maps, this must not be loaded as sRGB.
    #[texture(103)]
    #[sampler(104)]
    pub normals: Handle<Image>,
    #[uniform(105)]
    pub chunk_normal: Vec3,
}

impl MaterialExtension for TerrainMaterial {
    fn deferred_fragment_shader() -> ShaderRef {
        SHADER_ASSET_PATH.into()
    }
}
