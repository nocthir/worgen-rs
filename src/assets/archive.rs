// Copyright © 2025
// Author: Nocthir <nocthir@proton.me>
// SPDX-License-Identifier: MIT or Apache-2.0

use bevy::asset::io::Reader;
use bevy::asset::io::{
    AssetReader, AssetReaderError, AssetSource, AssetSourceId, PathStream, VecReader,
};
use bevy::prelude::*;
use std::io;
use std::path::Path;

use crate::data::archive;
use crate::data::file::FileMap;

#[derive(Default)]
pub struct ArchiveAssetReader {}

impl ArchiveAssetReader {
    pub fn read_file<P: AsRef<Path>>(&self, file_path: P) -> Result<Vec<u8>> {
        let file_name = file_path.as_ref().to_str().ok_or("Invalid file path")?;
        let file = FileMap::get().get_file(file_name)?;
        let mut archive = archive::get_archive!(&file.archive_path)?;
        Ok(archive.read_file(&file.path)?)
    }

    fn into_error(err: BevyError) -> AssetReaderError {
        AssetReaderError::Io(io::Error::other(err.to_string()).into())
    }
}

impl AssetReader for ArchiveAssetReader {
    async fn read<'a>(&'a self, path: &'a Path) -> Result<impl Reader + 'a, AssetReaderError> {
        Ok(VecReader::new(
            self.read_file(path).map_err(Self::into_error)?,
        ))
    }

    async fn read_meta<'a>(
        &'a self,
        _path: &'a Path,
    ) -> Result<impl Reader + 'a, AssetReaderError> {
        Ok(VecReader::new(Vec::new()))
    }

    async fn read_directory<'a>(
        &'a self,
        _path: &'a Path,
    ) -> Result<Box<PathStream>, AssetReaderError> {
        unimplemented!()
    }
    async fn is_directory<'a>(&'a self, _path: &'a Path) -> Result<bool, AssetReaderError> {
        unimplemented!()
    }
}

pub struct ArchiveAssetReaderPlugin;

impl Plugin for ArchiveAssetReaderPlugin {
    fn build(&self, app: &mut App) {
        app.register_asset_source(
            AssetSourceId::Name("archive".into()),
            AssetSource::build().with_reader(|| Box::new(ArchiveAssetReader {})),
        );
    }
}
