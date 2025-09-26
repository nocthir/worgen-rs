// Copyright © 2025
// Author: Nocthir <nocthir@proton.me>
// SPDX-License-Identifier: MIT or Apache-2.0

use std::{
    collections::HashMap,
    path::{Path, PathBuf},
    ptr::addr_of,
    sync::Once,
};

use bevy::{ecs::system::SystemParam, pbr::ExtendedMaterial, prelude::*};

use crate::data::*;
use crate::material::TerrainMaterial;

pub static mut FILE_ARCHIVE_MAP: FileArchiveMap = FileArchiveMap::new();
static FILE_ARCHIVE_MAP_ONCE: Once = Once::new();

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
            .ok_or(format!("File `{}` not found in archive map", file_path).into())
    }

    fn fill(&mut self) -> Result<()> {
        let mut map = HashMap::new();
        for archive_path in archive::ArchiveMap::get().get_archive_paths() {
            let mut archive = archive::get_archive!(archive_path)?;
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

pub struct FileInfo {
    pub path: String,
    pub archive_path: PathBuf,
}

impl FileInfo {
    pub fn new<S: Into<String>, P: AsRef<Path>>(path: S, archive_path: P) -> Self {
        let path = path.into();
        Self {
            path: path.clone(),
            archive_path: archive_path.as_ref().into(),
        }
    }

    pub fn get_asset_path(&self) -> String {
        format!("archive://{}", self.path)
    }
}

#[derive(Resource, Default)]
pub struct FileInfoMap {
    map: HashMap<String, FileInfo>,
}

impl FileInfoMap {
    // Actually used in tests
    #[allow(unused)]
    pub fn get_file_infos(&self) -> impl Iterator<Item = &FileInfo> {
        self.map.values()
    }

    pub fn insert(&mut self, file_info: FileInfo) {
        self.map.insert(file_info.path.to_lowercase(), file_info);
    }

    pub fn get_file_info(&self, file_path: &str) -> Result<&FileInfo> {
        let lowercase_name = file_path.to_lowercase();
        self.map
            .get(&lowercase_name)
            .ok_or(format!("File `{}` not found", file_path).into())
    }

    pub fn get_file_info_mut(&mut self, file_path: &str) -> Result<&mut FileInfo> {
        let lowercase_name = file_path.to_lowercase();
        self.map
            .get_mut(&lowercase_name)
            .ok_or(format!("File `{}` not found", file_path).into())
    }

    // Actually used in tests
    #[allow(unused)]
    pub fn fill(&mut self, archive_info: &mut ArchiveInfo) -> Result<()> {
        let mut archive = archive::get_archive!(&archive_info.path)?;
        for file_path in archive.list()? {
            let file_path = file_path.name;
            let texture_info = FileInfo::new(file_path.clone(), &archive_info.path);
            self.map.insert(file_path.to_lowercase(), texture_info);
        }

        Ok(())
    }
}
#[derive(SystemParam)]
pub struct SceneAssets<'w> {
    pub images: ResMut<'w, Assets<Image>>,
    pub meshes: ResMut<'w, Assets<Mesh>>,
    pub terrain_materials: ResMut<'w, Assets<ExtendedMaterial<StandardMaterial, TerrainMaterial>>>,
    pub materials: ResMut<'w, Assets<StandardMaterial>>,
}

impl<'w> SceneAssets<'w> {
    // Intentionally no manual constructor: acquire via SystemState in systems/tests.
}
#[cfg(test)]
pub mod test {
    use std::{fs, path::Path};

    use super::*;
    use crate::{data::archive, settings};

    pub fn default_file_info_map(settings: &settings::TestSettings) -> Result<FileInfoMap> {
        let mut file_info_map = FileInfoMap::default();
        let data_dir = Path::new(&settings.game_path).join("Data");
        for entry in fs::read_dir(&data_dir)? {
            let entry = entry?;
            let path = entry.path();
            if !archive::is_archive_extension(&path) {
                continue;
            }
            let mut archive_info = archive::ArchiveInfo::new(&path)?;
            file_info_map.fill(&mut archive_info)?;
        }
        assert!(!file_info_map.map.is_empty());
        Ok(file_info_map)
    }
}
