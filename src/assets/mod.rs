// Copyright © 2025
// Author: Nocthir <nocthir@proton.me>
// SPDX-License-Identifier: MIT or Apache-2.0

//! Experimental asset-based loading pipeline.
//!
//! This module contains early scaffolding for migrating manual IO task based model
//! loading into Bevy's `AssetServer` pipeline. It is intentionally not wired into
//! the existing `DataPlugin` yet; integration will be incremental.
//!
//! Steps planned (not yet all implemented):
//! 1. Define `ModelAsset` as a lightweight, parse-only representation referencing
//!    texture handles (no mesh/material creation yet).
//! 2. Implement `AssetLoader` that parses M2 model bytes and enqueues dependent
//!    texture loads using `LoadContext::load`.
//! 3. Add a post-processing system to create meshes/materials once all textures
//!    are ready (future step).
//! 4. Replace manual model load task path with handle-based selection (future).

use std::io;

use anyhow::Result;
use bevy::asset::{AssetLoader, LoadContext, io::Reader};
use bevy::prelude::*;
use wow_m2 as m2;

/// First pass model asset: parsed data + texture dependency handles.
/// Mesh / material baking will happen in a later preparation system.
#[derive(Asset, TypePath, Debug)]
pub struct ModelAsset {
    /// Raw parsed model (CPU-side). Retained to allow later mesh generation.
    pub model: m2::M2Model,
    /// Original file bytes (kept for potential reprocessing; may be dropped later).
    pub data: Vec<u8>,
    /// Texture paths extracted from the model referencing external images.
    pub texture_paths: Vec<String>,
    /// Texture image handles requested during load (populated by the loader).
    pub texture_handles: Vec<Handle<Image>>,
}

impl ModelAsset {
    fn texture_paths(model: &m2::M2Model) -> Vec<String> {
        model
            .textures
            .iter()
            .filter(|t| t.texture_type == m2::chunks::M2TextureType::Hardcoded)
            .map(|t| t.filename.string.to_string_lossy())
            .collect()
    }
}

#[derive(Default)]
pub struct ModelAssetLoader;

#[derive(Debug)]
pub enum ModelAssetLoaderError {
    Io(io::Error),
    Parse(String),
}

impl core::fmt::Display for ModelAssetLoaderError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::Io(e) => write!(f, "IO error: {}", e),
            Self::Parse(e) => write!(f, "Parse error: {}", e),
        }
    }
}

impl std::error::Error for ModelAssetLoaderError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Io(e) => Some(e),
            Self::Parse(_) => None,
        }
    }
}

impl AssetLoader for ModelAssetLoader {
    type Asset = ModelAsset;
    type Settings = (); // No custom settings yet
    type Error = ModelAssetLoaderError;

    async fn load(
        &self,
        reader: &mut dyn Reader,
        _settings: &Self::Settings,
        load_context: &mut LoadContext<'_>,
    ) -> Result<Self::Asset, Self::Error> {
        let mut bytes = Vec::new();
        reader
            .read_to_end(&mut bytes)
            .await
            .map_err(ModelAssetLoaderError::Io)?;
        let mut cursor = io::Cursor::new(&bytes);
        let model = m2::M2Model::parse(&mut cursor)
            .map_err(|e| ModelAssetLoaderError::Parse(e.to_string()))?;
        let texture_paths = ModelAsset::texture_paths(&model);

        // Queue dependent texture loads.
        let texture_handles: Vec<Handle<Image>> = texture_paths
            .iter()
            .map(|p| load_context.load(p.as_str()))
            .collect();

        Ok(ModelAsset {
            model,
            data: bytes,
            texture_paths,
            texture_handles,
        })
    }

    fn extensions(&self) -> &[&str] {
        // M2 primary extension; MDX/MDL are historical / variant forms that may alias.
        &["m2", "mdx", "mdl"]
    }
}

/// Plugin that registers the experimental model asset + loader.
/// Not added to the main app yet; opt-in during migration.
pub struct ExperimentalModelAssetPlugin;

impl Plugin for ExperimentalModelAssetPlugin {
    fn build(&self, app: &mut App) {
        app.init_asset::<ModelAsset>()
            .register_asset_loader(ModelAssetLoader);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    // No extra imports needed; test ensures plugin builds.

    #[test]
    fn model_asset_loader_registers() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.add_plugins(AssetPlugin::default());
        app.add_plugins(ExperimentalModelAssetPlugin);
        // If we reach here without panic, registration succeeded.
    }
}
