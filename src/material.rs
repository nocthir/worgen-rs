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
    #[texture(100)]
    #[sampler(101)]
    pub level_texture: Option<Handle<Image>>,
    #[texture(102)]
    #[sampler(103)]
    pub level_alpha: Option<Handle<Image>>,
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
