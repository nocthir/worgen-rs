// Copyright © 2025
// Author: Nocthir <nocthir@proton.me>
// SPDX-License-Identifier: MIT or Apache-2.0

use bevy::{pbr::*, prelude::*, render::render_resource::*, shader::ShaderRef};

use wow_m2 as m2;
use wow_wmo as wmo;

/// This example uses a shader source file from the assets subdirectory
const SHADER_ASSET_PATH: &str = "shaders/terrain_material.wgsl";

pub type ExtTerrainMaterial = ExtendedMaterial<StandardMaterial, TerrainMaterial>;

// This struct defines the data that will be passed to your shader
#[derive(Asset, Default, AsBindGroup, Reflect, Debug, Clone)]
pub struct TerrainMaterial {
    #[uniform(69)]
    pub level_mask: u32,

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

pub fn alpha_mode_from_model_blend_mode(
    blend_mode: m2::chunks::material::M2BlendMode,
) -> AlphaMode {
    use m2::chunks::material::M2BlendMode as BM;
    if blend_mode.intersects(BM::ALPHA_KEY | BM::NO_ALPHA_ADD) {
        AlphaMode::AlphaToCoverage
    } else if blend_mode.intersects(BM::ADD | BM::BLEND_ADD) {
        AlphaMode::Add
    } else if blend_mode.intersects(BM::MOD | BM::MOD2X) {
        AlphaMode::Multiply
    } else if blend_mode.intersects(BM::ALPHA) {
        AlphaMode::Blend
    } else {
        AlphaMode::Opaque
    }
}

pub fn alpha_mode_from_world_model_blend_mode(blend_mode: u32) -> AlphaMode {
    alpha_mode_from_model_blend_mode(m2::chunks::material::M2BlendMode::from_bits_truncate(
        blend_mode as u16,
    ))
}

pub fn sampler_from_world_model_material_flags(
    flags: wmo::WmoMaterialFlags,
) -> bevy::image::ImageSampler {
    use bevy::image::*;
    let address_mode_u = if flags.contains(wmo::WmoMaterialFlags::CLAMP_S) {
        ImageAddressMode::ClampToEdge
    } else {
        ImageAddressMode::Repeat
    };
    let address_mode_v = if flags.contains(wmo::WmoMaterialFlags::CLAMP_T) {
        ImageAddressMode::ClampToEdge
    } else {
        ImageAddressMode::Repeat
    };
    let descriptor = ImageSamplerDescriptor {
        address_mode_u,
        address_mode_v,
        ..Default::default()
    };
    ImageSampler::Descriptor(descriptor)
}

pub fn color_from_world_model(bgra: [u8; 4]) -> Color {
    let b = bgra[0];
    let g = bgra[1];
    let r = bgra[2];
    let a = bgra[3];
    Color::linear_rgba(
        r as f32 / 255.0,
        g as f32 / 255.0,
        b as f32 / 255.0,
        a as f32 / 255.0,
    )
}

pub fn from_normalized_vec3_u8(v: [u8; 3]) -> [f32; 3] {
    let x = u8::cast_signed(v[0]) as f32 / 127.0;
    let y = u8::cast_signed(v[1]) as f32 / 127.0;
    let z = u8::cast_signed(v[2]) as f32 / 127.0;
    normalize_vec3([x, y, z])
}

pub fn normalize_vec3(v: [f32; 3]) -> [f32; 3] {
    let len = (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt();
    if len > 0.0 {
        [v[0] / len, v[1] / len, v[2] / len]
    } else {
        v
    }
}
