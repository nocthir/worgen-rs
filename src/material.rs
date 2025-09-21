use bevy::{pbr::*, prelude::*, render::render_resource::*};

/// This example uses a shader source file from the assets subdirectory
const SHADER_ASSET_PATH: &str = "shaders/terrain_material.wgsl";

pub struct TerrainMaterialPlugin;

impl Plugin for TerrainMaterialPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(MaterialPlugin::<
            ExtendedMaterial<StandardMaterial, TerrainMaterial>,
        >::default());
    }
}

// This struct defines the data that will be passed to your shader
#[derive(Asset, Default, AsBindGroup, Reflect, Debug, Clone)]
pub struct TerrainMaterial {
    #[uniform(70)]
    pub level_count: u32,

    #[texture(71)]
    #[sampler(72)]
    pub alpha_texture: Handle<Image>,
    #[texture(73)]
    #[sampler(74)]
    pub level1_texture: Option<Handle<Image>>,
    #[texture(75)]
    #[sampler(76)]
    pub level2_texture: Option<Handle<Image>>,
    #[texture(77)]
    #[sampler(78)]
    pub level3_texture: Option<Handle<Image>>,
}

/// The Material trait is very configurable, but comes with sensible defaults for all methods.
/// You only need to implement functions for features that need non-default behavior.
/// See the Material api docs for details!
impl MaterialExtension for TerrainMaterial {
    fn fragment_shader() -> ShaderRef {
        SHADER_ASSET_PATH.into()
    }

    fn deferred_fragment_shader() -> ShaderRef {
        SHADER_ASSET_PATH.into()
    }
}
