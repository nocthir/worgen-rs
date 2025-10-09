// Copyright © 2025
// Author: Nocthir <nocthir@proton.me>
// SPDX-License-Identifier: MIT or Apache-2.0

use std::io;

use bevy::{
    asset::{io::Reader, *},
    prelude::*,
};
use thiserror::Error;

use wow_cdbc as dbc;

#[derive(Asset, Debug, TypePath)]
pub struct DataBaseAsset {
    pub record_count: u32,
}

pub fn is_data_base_extension(filename: &str) -> bool {
    filename.to_lowercase().ends_with(".dbc")
}

#[derive(Default)]
pub struct DataBaseAssetLoader;

impl AssetLoader for DataBaseAssetLoader {
    type Asset = DataBaseAsset;
    type Settings = ();
    type Error = DataBaseAssetLoaderError;

    async fn load(
        &self,
        reader: &mut dyn Reader,
        _settings: &Self::Settings,
        load_context: &mut LoadContext<'_>,
    ) -> Result<Self::Asset, Self::Error> {
        let mut bytes = Vec::new();
        reader.read_to_end(&mut bytes).await?;
        let data_base_path = load_context.path().to_string_lossy().into_owned();
        Self::load_data_base(&data_base_path, bytes, load_context).await
    }

    fn extensions(&self) -> &[&str] {
        &["dbc"]
    }
}

impl DataBaseAssetLoader {
    async fn load_data_base(
        data_base_path: &str,
        bytes: Vec<u8>,
        _load_context: &mut LoadContext<'_>,
    ) -> Result<DataBaseAsset, DataBaseAssetLoaderError> {
        // Here you would parse the DBC file and create the DataBaseAsset accordingly.
        // For now, we just log the loading and return an empty DataBaseAsset.
        let parser = dbc::DbcParser::parse_bytes(bytes.as_slice())?;
        let header = parser.header();
        let db = DataBaseAsset {
            record_count: header.record_count,
        };
        info!("Loaded DataBase from path: {}", data_base_path);
        Ok(db)
    }
}

#[derive(Debug, Error)]
pub enum DataBaseAssetLoaderError {
    #[error("IO error: {0}")]
    Io(#[from] io::Error),
    #[error("Parse error: {0}")]
    Parse(#[from] wow_cdbc::Error),
    #[error("Read error: {0}")]
    Read(#[from] ReadAssetBytesError),
    #[error("Other error: {0}")]
    Other(#[from] anyhow::Error),
}
