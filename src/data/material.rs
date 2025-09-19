// Copyright © 2025
// Author: Nocthir <nocthir@proton.me>
// SPDX-License-Identifier: MIT or Apache-2.0

use bevy::prelude::*;

use wow_m2 as m2;

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
