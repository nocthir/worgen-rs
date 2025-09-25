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

pub mod archive;
pub mod image;
pub use archive::*;
pub use image::*;
pub mod model;
pub use model::*;

use bevy::prelude::*;

/// Plugin that registers the experimental model asset + loader.
/// Not added to the main app yet; opt-in during migration.
pub struct ExperimentalModelAssetPlugin;

impl Plugin for ExperimentalModelAssetPlugin {
    fn build(&self, app: &mut App) {
        app.init_asset::<ModelAsset>()
            .init_asset_loader::<ImageLoader>()
            .init_asset_loader::<ModelAssetLoader>();
        // Systems are optional; gated behind a run condition if selection resource exists.
        app.insert_resource(SelectedModelHandle(None))
            .add_systems(Update, model::prepare_model_assets)
            .add_systems(Update, model::spawn_selected_model);
        app.add_systems(Startup, select_test_model);
    }
}

fn select_test_model(mut sel: ResMut<SelectedModelHandle>, asset_server: Res<AssetServer>) {
    sel.0 = Some(asset_server.load("HumanMale.m2")); // adjust path
}

/// Resource to hold a currently selected model asset handle (experimental path).
#[derive(Resource, Default, Deref, DerefMut)]
pub struct SelectedModelHandle(pub Option<Handle<ModelAsset>>);

/// Marker component for entities spawned from a `ModelAsset` via the experimental path.
#[derive(Component)]
pub struct ExperimentalModelInstance;

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
