// Copyright © 2025
// Author: Nocthir <nocthir@proton.me>
// SPDX-License-Identifier: MIT or Apache-2.0

use bevy::asset::{AssetLoader, LoadContext, io::Reader};
use bevy::asset::{ReadAssetBytesError, RenderAssetUsages};
use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
use std::io;
use thiserror::Error;
use wow_blp as blp;

#[derive(Default)]
pub struct ImageLoader;

#[derive(Debug, Error)]
pub enum ImageLoaderError {
    #[error("IO error: {0}")]
    Io(#[from] io::Error),
    #[error("Parse error: {0}")]
    Parse(#[from] blp::parser::LoadError),
    #[error("Conversion error: {0}")]
    Conversion(#[from] blp::convert::Error),
    #[error("Read error: {0}")]
    Read(#[from] ReadAssetBytesError),
}

impl AssetLoader for ImageLoader {
    type Asset = Image;
    type Settings = (); // No custom settings yet
    type Error = ImageLoaderError;

    async fn load(
        &self,
        reader: &mut dyn Reader,
        _settings: &Self::Settings,
        _load_context: &mut LoadContext<'_>,
    ) -> Result<Self::Asset, Self::Error> {
        let mut bytes = Vec::new();
        reader.read_to_end(&mut bytes).await?;
        Self::load_image_impl(&bytes).await
    }

    fn extensions(&self) -> &[&str] {
        &["blp"]
    }
}

impl ImageLoader {
    pub async fn load_path<S: Into<String>>(
        path: S,
        load_context: &mut LoadContext<'_>,
    ) -> Result<Image, ImageLoaderError> {
        let asset_path = format!("archive://{}", path.into());
        Self::load_image(&asset_path, load_context).await
    }

    pub async fn load_image(
        asset_path: &str,
        load_context: &mut LoadContext<'_>,
    ) -> Result<Image, ImageLoaderError> {
        let bytes = load_context.read_asset_bytes(asset_path).await?;
        Self::load_image_impl(&bytes).await
    }

    async fn load_image_impl(bytes: &[u8]) -> Result<Image, ImageLoaderError> {
        let image = blp::parser::load_blp_from_buf(bytes)?;
        let dyn_image = blp::convert::blp_to_image(&image, 0)?;
        let extent = Extent3d {
            width: dyn_image.width(),
            height: dyn_image.height(),
            depth_or_array_layers: 1,
        };
        let dimension = TextureDimension::D2;
        let data = dyn_image.to_rgba8().into_raw();
        let texture_format = TextureFormat::Rgba8Unorm;
        let usage = RenderAssetUsages::RENDER_WORLD;
        Ok(Image::new(extent, dimension, data, texture_format, usage))
    }
}
