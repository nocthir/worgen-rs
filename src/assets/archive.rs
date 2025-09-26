// Copyright © 2025
// Author: Nocthir <nocthir@proton.me>
// SPDX-License-Identifier: MIT or Apache-2.0

use std::collections::HashMap;
use std::io;
use std::path::{Path, PathBuf};
use std::ptr::addr_of;
use std::sync::Once;

use bevy::asset::io::Reader;
use bevy::asset::io::{
    AssetReader, AssetReaderError, AssetSource, AssetSourceId, PathStream, VecReader,
};
use bevy::prelude::*;
use wow_mpq as mpq;

use crate::settings;

pub static mut FILE_ARCHIVE_MAP: FileArchiveMap = FileArchiveMap::new();
static FILE_ARCHIVE_MAP_ONCE: Once = Once::new();

pub fn get_archive_paths() -> Result<Vec<PathBuf>> {
    let game_path = PathBuf::from(&settings::Settings::get().game_path);
    let data_path = game_path.join("Data");

    let mut ret = Vec::new();

    for file in data_path.read_dir()? {
        let file = file?;
        let file_path = file.path();
        if is_archive_extension(&file_path) {
            ret.push(file_path);
        }
    }

    Ok(ret)
}

pub fn is_archive_extension<P: AsRef<Path>>(path: P) -> bool {
    path.as_ref()
        .extension()
        .is_some_and(|ext| ext.to_string_lossy().eq_ignore_ascii_case("mpq"))
}

#[derive(Default)]
pub struct FileArchiveMap {
    pub map: Option<HashMap<String, PathBuf>>,
}

impl FileArchiveMap {
    const fn new() -> Self {
        Self { map: None }
    }

    pub fn get() -> &'static Self {
        debug_assert!(FILE_ARCHIVE_MAP_ONCE.is_completed());
        // SAFETY: no mut references exist at this point
        unsafe { &*addr_of!(FILE_ARCHIVE_MAP) }
    }

    pub fn get_archive_path(&self, file_path: &str) -> Result<&PathBuf> {
        let lowercase_name = file_path.to_lowercase();
        self.map
            .as_ref()
            .unwrap()
            .get(&lowercase_name)
            .ok_or(format!("File `{}` not found in file archive map", file_path).into())
    }

    fn fill(&mut self) -> Result<()> {
        let mut map = HashMap::new();
        for archive_path in get_archive_paths()? {
            let mut archive = mpq::Archive::open(&archive_path)?;
            for file_path in archive.list()? {
                map.insert(file_path.name.to_lowercase(), archive_path.clone());
            }
        }
        self.map.replace(map);
        Ok(())
    }

    pub fn init() {
        // SAFETY: no concurrent static mut access due to std::Once
        #[allow(static_mut_refs)]
        FILE_ARCHIVE_MAP_ONCE.call_once(|| unsafe {
            FILE_ARCHIVE_MAP
                .fill()
                .expect("Failed to fill file archive map");
        });
    }
}

#[derive(Default)]
pub struct ArchiveAssetReader {}

impl ArchiveAssetReader {
    pub fn read_file<P: AsRef<Path>>(&self, file_path: P) -> Result<Vec<u8>> {
        let file_name = file_path.as_ref().to_str().ok_or("Invalid file path")?;
        let archive_path = FileArchiveMap::get().get_archive_path(file_name)?;
        let mut archive = mpq::Archive::open(archive_path)?;
        Ok(archive.read_file(file_name)?)
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
