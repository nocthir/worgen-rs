// Copyright © 2025
// Author: Nocthir <nocthir@proton.me>
// SPDX-License-Identifier: MIT or Apache-2.0

use bevy::prelude::*;

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
