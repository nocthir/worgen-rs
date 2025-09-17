// Copyright © 2025
// Author: Nocthir <nocthir@proton.me>
// SPDX-License-Identifier: MIT or Apache-2.0

use std::collections::HashMap;
use std::path::Path;
use std::path::PathBuf;

use bevy::prelude::*;
use bevy::tasks;
use wow_mpq as mpq;

use crate::data::ArchivesInfo;
use crate::data::model::*;
use crate::data::texture::TextureInfo;
use crate::data::world_map::WorldMapInfo;
use crate::data::world_map::is_world_map_extension;
use crate::data::world_model::*;
use crate::settings::Settings;

pub struct ArchiveInfo {
    pub path: PathBuf,
    pub archive: mpq::Archive,
    pub texture_paths: Vec<String>,
    pub model_paths: Vec<String>,
    pub world_model_paths: Vec<String>,
    pub world_map_paths: Vec<String>,
}

impl ArchiveInfo {
    pub fn new<P: AsRef<Path>>(path: P) -> Result<Self> {
        let mut archive = mpq::Archive::open(path.as_ref())?;
        let texture_paths = Self::get_texture_paths(&mut archive)?;
        let model_paths = Self::get_model_paths(&mut archive)?;
        let world_model_paths = Self::get_world_model_paths(&mut archive)?;
        let world_map_paths = Self::get_world_map_paths(&mut archive)?;
        Ok(Self {
            path: path.as_ref().into(),
            archive,
            texture_paths,
            model_paths,
            world_model_paths,
            world_map_paths,
        })
    }

    pub fn has_stuff(&mut self) -> bool {
        self.archive.list().is_ok_and(|files| !files.is_empty())
    }

    fn get_texture_paths(archive: &mut mpq::Archive) -> Result<Vec<String>> {
        let mut textures = Vec::new();
        archive.list()?.retain(|file| {
            if file.name.to_lowercase().ends_with(".blp") {
                textures.push(file.name.clone());
                false
            } else {
                true
            }
        });
        Ok(textures)
    }

    fn get_model_paths(archive: &mut mpq::Archive) -> Result<Vec<String>> {
        let mut models = Vec::new();
        archive.list()?.retain(|file| {
            if is_model_extension(&file.name) {
                models.push(file.name.clone());
                false
            } else {
                true
            }
        });
        Ok(models)
    }

    fn get_world_model_paths(archive: &mut mpq::Archive) -> Result<Vec<String>> {
        let mut world_models = Vec::new();
        archive.list()?.retain(|file| {
            if is_world_model_extension(&file.name) {
                world_models.push(file.name.clone());
                false
            } else {
                true
            }
        });
        Ok(world_models)
    }

    fn get_world_map_paths(archive: &mut mpq::Archive) -> Result<Vec<String>> {
        let mut world_maps = Vec::new();
        archive.list()?.retain(|file| {
            if is_world_map_extension(&file.name) {
                world_maps.push(file.name.clone());
                false
            } else {
                true
            }
        });
        Ok(world_maps)
    }
}

pub enum FileInfoState {
    Unloaded,
    Loading,
    Loaded,
    Error(String),
}

pub struct FileInfo {
    pub path: String,
    pub archive_path: PathBuf,
    pub state: FileInfoState,
    pub data_info: Option<DataInfo>,
}

impl FileInfo {
    pub fn new<S: Into<String>, P: AsRef<Path>>(path: S, archive_path: P) -> Self {
        Self {
            path: path.into(),
            archive_path: archive_path.as_ref().into(),
            state: FileInfoState::Unloaded,
            data_info: None,
        }
    }

    pub fn new_with_data<S: Into<String>, P: AsRef<Path>>(
        path: S,
        archive_path: P,
        data_info: DataInfo,
    ) -> Self {
        Self {
            path: path.into(),
            archive_path: archive_path.as_ref().into(),
            state: FileInfoState::Loaded,
            data_info: Some(data_info),
        }
    }

    pub fn is_unloaded(&self) -> bool {
        matches!(self.state, FileInfoState::Unloaded)
    }

    pub fn new_texture<S: Into<String>, P: AsRef<Path>>(
        path: S,
        archive_path: P,
        texture_info: TextureInfo,
    ) -> Self {
        Self::new_with_data(path, archive_path, DataInfo::Texture(texture_info))
    }

    pub fn new_model<S: Into<String>, P: AsRef<Path>>(
        path: S,
        archive_path: P,
        model_info: ModelInfo,
    ) -> Self {
        Self::new_with_data(path, archive_path, DataInfo::Model(model_info))
    }

    pub fn new_world_model<S: Into<String>, P: AsRef<Path>>(
        path: S,
        archive_path: P,
        wmo_info: WorldModelInfo,
    ) -> Self {
        Self::new_with_data(path, archive_path, DataInfo::WorldModel(wmo_info))
    }

    pub fn new_world_map<S: Into<String>, P: AsRef<Path>>(
        path: S,
        archive_path: P,
        world_map_info: WorldMapInfo,
    ) -> Self {
        Self::new_with_data(path, archive_path, DataInfo::WorldMap(world_map_info))
    }

    pub fn get_model(&self) -> Result<&ModelInfo> {
        if let Some(DataInfo::Model(model_info)) = &self.data_info {
            Ok(model_info)
        } else {
            Err(format!("File {} is not a model", self.path).into())
        }
    }
}

pub enum DataInfo {
    Texture(TextureInfo),
    Model(ModelInfo),
    WorldModel(WorldModelInfo),
    WorldMap(WorldMapInfo),
}

#[derive(Resource, Default)]
pub struct FileInfoMap {
    pub map: HashMap<String, FileInfo>,
}

impl FileInfoMap {
    pub fn get_file_info(&self, file_path: &str) -> Result<&FileInfo> {
        let lowercase_name = file_path.to_lowercase();
        self.map
            .get(&lowercase_name)
            .ok_or(format!("File {} not found", file_path).into())
    }

    pub fn get_file_info_mut(&mut self, file_path: &str) -> Result<&mut FileInfo> {
        let lowercase_name = file_path.to_lowercase();
        self.map
            .get_mut(&lowercase_name)
            .ok_or(format!("File {} not found", file_path).into())
    }

    pub fn get_texture_info(&self, file_path: &str) -> Result<&TextureInfo> {
        let file_info = self.get_file_info(file_path)?;
        if let Some(DataInfo::Texture(texture_info)) = &file_info.data_info {
            Ok(texture_info)
        } else {
            Err(format!("Texture {} not found", file_path).into())
        }
    }

    pub fn get_model_info(&self, file_path: &str) -> Result<&ModelInfo> {
        let file_info = self.get_file_info(file_path)?;
        if let Some(DataInfo::Model(model_info)) = &file_info.data_info {
            Ok(model_info)
        } else {
            Err(format!("Model {} not found", file_path).into())
        }
    }

    pub fn get_world_model_info(&self, file_path: &str) -> Result<&WorldModelInfo> {
        let file_info = self.get_file_info(file_path)?;
        if let Some(DataInfo::WorldModel(wmo_info)) = &file_info.data_info {
            Ok(wmo_info)
        } else {
            Err(format!("World model {} not found", file_path).into())
        }
    }

    pub fn get_world_map_info(&self, file_path: &str) -> Result<&WorldMapInfo> {
        let file_info = self.get_file_info(file_path)?;
        if let Some(DataInfo::WorldMap(world_map_info)) = &file_info.data_info {
            Ok(world_map_info)
        } else {
            Err(format!("World map {} not found", file_path).into())
        }
    }

    // Actually used in tests
    #[allow(unused)]
    pub fn fill(&mut self, archive_info: &mut ArchiveInfo) -> Result<()> {
        for file_path in archive_info.archive.list()? {
            let file_path = file_path.name;
            let texture_info = FileInfo::new(file_path.clone(), &archive_info.path);
            self.map.insert(file_path.to_lowercase(), texture_info);
        }

        Ok(())
    }
}

#[derive(Event)]
pub struct ArchiveLoaded {
    pub archive: Option<ArchiveInfo>,
}

#[derive(Resource, Default)]
pub struct LoadArchiveTasks {
    tasks: Vec<tasks::Task<Result<ArchiveInfo>>>,
}

pub fn start_loading(mut commands: Commands, settings: Res<Settings>) -> Result<()> {
    let game_path = PathBuf::from(&settings.game_path);
    let data_path = game_path.join("Data");

    let mut tasks = LoadArchiveTasks::default();

    for file in data_path.read_dir()? {
        let file = file?;
        if file.file_name().to_string_lossy().ends_with(".MPQ") {
            let mpq_path = file.path().to_string_lossy().to_string();
            let task = tasks::IoTaskPool::get().spawn(load_archive(mpq_path));
            tasks.tasks.push(task);
        }
    }

    commands.insert_resource(tasks);

    Ok(())
}

async fn load_archive(archive_path: String) -> Result<ArchiveInfo> {
    ArchiveInfo::new(archive_path)
}

pub fn open_archive<P: AsRef<Path>>(archive_path: P) -> Result<mpq::Archive> {
    mpq::Archive::open(archive_path.as_ref()).map_err(|e| {
        format!(
            "Failed to open archive {}: {}",
            archive_path.as_ref().display(),
            e
        )
        .into()
    })
}

pub fn check_archive_loading(
    mut exit: EventWriter<AppExit>,
    mut load_task: ResMut<LoadArchiveTasks>,
    mut file_info_map: ResMut<FileInfoMap>,
    mut archives_info: ResMut<ArchivesInfo>,
) -> Result<()> {
    let mut tasks = Vec::new();
    tasks.append(&mut load_task.tasks);

    for mut current_task in tasks {
        let poll_result = tasks::block_on(tasks::poll_once(&mut current_task));
        if let Some(result) = poll_result {
            match result {
                Err(err) => {
                    error!("Error loading archive: {err}");
                    exit.write(AppExit::error());
                }
                Ok(mut archive) => {
                    info!("Loaded archive info: {}", archive.path.display());

                    // Update the file archive map
                    file_info_map.fill(&mut archive)?;
                    archives_info.archives.push(archive);
                }
            }
        } else {
            // Not ready yet, put it back
            load_task.tasks.push(current_task);
        }
    }
    Ok(())
}
