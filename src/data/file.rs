// Copyright © 2025
// Author: Nocthir <nocthir@proton.me>
// SPDX-License-Identifier: MIT or Apache-2.0

use std::{
    collections::HashMap,
    path::{Path, PathBuf},
};

use bevy::{
    asset::RecursiveDependencyLoadState, ecs::system::SystemParam, pbr::ExtendedMaterial,
    prelude::*,
};
use wow_mpq as mpq;

use crate::assets::{ModelAsset, WorldMapAsset, WorldModelAsset};
use crate::{assets, material::TerrainMaterial};

pub struct FileInfo {
    pub path: String,
    pub archive_path: PathBuf,
    pub data_type: DataType,
}

impl FileInfo {
    pub fn new<S: Into<String>, P: AsRef<Path>>(path: S, archive_path: P) -> Self {
        let path = path.into();
        Self {
            path: path.clone(),
            archive_path: archive_path.as_ref().into(),
            data_type: DataType::from(path),
        }
    }

    pub fn get_asset_path(&self) -> String {
        format!("archive://{}", self.path)
    }

    pub fn load(&mut self, asset_server: &mut AssetServer) {
        self.data_type.load(self.get_asset_path(), asset_server);
    }

    pub fn unload(&mut self) {
        self.data_type.unload();
    }

    pub fn get_load_state(&self, asset_server: &AssetServer) -> RecursiveDependencyLoadState {
        self.data_type.state(asset_server)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DataType {
    Texture(Handle<Image>),
    Model(Handle<ModelAsset>),
    WorldModel(Handle<WorldModelAsset>),
    WorldMap(Handle<WorldMapAsset>),
    Unknown,
}

impl DataType {
    pub fn set_handle<H: Into<UntypedHandle>>(&mut self, handle: H) {
        let handle = handle.into();
        match self {
            DataType::Texture(h) => *h = handle.typed(),
            DataType::Model(h) => *h = handle.typed(),
            DataType::WorldModel(h) => *h = handle.typed(),
            DataType::WorldMap(h) => *h = handle.typed(),
            DataType::Unknown => {}
        }
    }

    pub fn load(&mut self, path: String, asset_server: &mut AssetServer) {
        info!("Loading file: {}", path);
        match self {
            DataType::Texture(handle) => *handle = asset_server.load(path),
            DataType::Model(handle) => *handle = asset_server.load(path),
            DataType::WorldModel(handle) => *handle = asset_server.load(path),
            DataType::WorldMap(handle) => *handle = asset_server.load(path),
            DataType::Unknown => (),
        };
    }

    pub fn unload(&mut self) {
        match self {
            DataType::Texture(handle) => *handle = Handle::default(),
            DataType::Model(handle) => *handle = Handle::default(),
            DataType::WorldModel(handle) => *handle = Handle::default(),
            DataType::WorldMap(handle) => *handle = Handle::default(),
            DataType::Unknown => (),
        };
    }

    pub fn state(&self, asset_server: &AssetServer) -> RecursiveDependencyLoadState {
        let ret = match self {
            DataType::Texture(handle) => asset_server.get_recursive_dependency_load_state(handle),
            DataType::Model(handle) => asset_server.get_recursive_dependency_load_state(handle),
            DataType::WorldModel(handle) => {
                asset_server.get_recursive_dependency_load_state(handle)
            }
            DataType::WorldMap(handle) => asset_server.get_recursive_dependency_load_state(handle),
            DataType::Unknown => None,
        };
        ret.unwrap_or(RecursiveDependencyLoadState::NotLoaded)
    }
}

impl<S: Into<String>> From<S> for DataType {
    fn from(file_path: S) -> Self {
        let lowercase = file_path.into().to_lowercase();
        if lowercase.ends_with(".blp") {
            DataType::Texture(Handle::default())
        } else if lowercase.ends_with(".m2") {
            DataType::Model(Handle::default())
        } else if lowercase.ends_with(".wmo") {
            DataType::WorldModel(Handle::default())
        } else if lowercase.ends_with(".adt") {
            DataType::WorldMap(Handle::default())
        } else {
            DataType::Unknown
        }
    }
}

#[derive(Resource)]
pub struct FileInfoMap {
    map: HashMap<String, FileInfo>,
}

impl FileInfoMap {
    pub fn new() -> Result<Self> {
        let mut map = HashMap::new();
        for archive_path in assets::get_archive_paths()? {
            let mut archive = mpq::Archive::open(&archive_path)?;
            for file_path in archive.list()? {
                let info = FileInfo::new(file_path.name.clone(), &archive_path);
                map.insert(file_path.name.to_lowercase(), info);
            }
        }
        Ok(Self { map })
    }

    pub fn get_file(&self, file_path: &str) -> Result<&FileInfo> {
        let lowercase_name = file_path.to_lowercase();
        self.map
            .get(&lowercase_name)
            .ok_or(format!("File `{}` not found in file map", file_path).into())
    }

    // Actually used in tests
    #[allow(unused)]
    pub fn get_files(&self) -> impl Iterator<Item = &FileInfo> {
        self.map.values()
    }

    pub fn insert(&mut self, file_info: FileInfo) {
        self.map.insert(file_info.path.to_lowercase(), file_info);
    }

    pub fn get_file_mut(&mut self, file_path: &str) -> Result<&mut FileInfo> {
        let lowercase_name = file_path.to_lowercase();
        self.map
            .get_mut(&lowercase_name)
            .ok_or(format!("File `{}` not found", file_path).into())
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
