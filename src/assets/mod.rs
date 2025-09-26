// Copyright © 2025
// Author: Nocthir <nocthir@proton.me>
// SPDX-License-Identifier: MIT or Apache-2.0

pub mod archive;
pub mod image;
pub mod material;
pub mod mesh;
pub mod model;
pub mod root_aabb;
pub mod world_map;
pub mod world_model;

use image::*;
use material::*;
use mesh::*;
use model::*;
use root_aabb::*;
use world_map::*;
use world_model::*;

use bevy::prelude::*;

pub struct AssetPlugin;

impl Plugin for AssetPlugin {
    fn build(&self, app: &mut App) {
        app.register_type::<RootAabb>()
            .init_asset::<ModelAsset>()
            .init_asset::<WorldModelAsset>()
            .init_asset::<WorldMapAsset>()
            .init_asset_loader::<ImageLoader>()
            .init_asset_loader::<ModelAssetLoader>()
            .init_asset_loader::<WorldModelAssetLoader>()
            .init_asset_loader::<WorldMapAssetLoader>()
            .add_systems(PreStartup, archive::FileArchiveMap::init);
    }
}
