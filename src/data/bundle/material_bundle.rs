// Copyright © 2025
// Author: Nocthir <nocthir@proton.me>
// SPDX-License-Identifier: MIT or Apache-2.0

use bevy::{prelude::*, render::render_resource::Face};

use wow_m2 as m2;
use wow_wmo as wmo;

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

pub fn create_materials_from_world_model(
    wmo: &wmo::WmoRoot,
    images: &[Handle<Image>],
    image_assets: &mut Assets<Image>,
) -> Vec<StandardMaterial> {
    let mut materials = Vec::new();

    for material in &wmo.materials {
        let base_color = create_color_from_world_model(material.diffuse_color);
        let emissive = create_color_from_world_model(material.emissive_color).to_linear();

        let texture_index = material.get_texture1_index(&wmo.texture_offset_index_map);
        let image_handle = images[texture_index as usize].clone();
        let image = image_assets.get_mut(&image_handle).unwrap();
        image.sampler = sampler_from_world_model_material_flags(material.flags);

        let unlit = material.flags.intersects(
            wmo::WmoMaterialFlags::UNLIT
                | wmo::WmoMaterialFlags::EXTERIOR_LIGHT
                | wmo::WmoMaterialFlags::WINDOW_LIGHT,
        );
        let cull_mode = if material.flags.contains(wmo::WmoMaterialFlags::TWO_SIDED) {
            None
        } else {
            Some(Face::Back)
        };

        let alpha_mode = alpha_mode_from_world_model_blend_mode(material.blend_mode);

        let material = StandardMaterial {
            base_color,
            emissive,
            perceptual_roughness: 1.0,
            base_color_texture: Some(image_handle),
            unlit,
            cull_mode,
            alpha_mode,
            ..Default::default()
        };
        materials.push(material);
    }
    materials
}

fn sampler_from_world_model_material_flags(
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

fn create_color_from_world_model(color: wmo::types::Color) -> Color {
    Color::linear_rgba(
        color.r as f32 / 255.0,
        color.g as f32 / 255.0,
        color.b as f32 / 255.0,
        color.a as f32 / 255.0,
    )
}
