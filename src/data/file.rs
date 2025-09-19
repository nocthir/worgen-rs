// Copyright © 2025
// Author: Nocthir <nocthir@proton.me>
// SPDX-License-Identifier: MIT or Apache-2.0

use std::{
    collections::HashMap,
    path::{Path, PathBuf},
};

use bevy::{prelude::*, tasks};

use crate::data::{
    add_bundle,
    archive::ArchiveInfo,
    model::{self, ModelInfo},
    texture::{self, TextureInfo},
    world_map::WorldMapInfo,
    world_model::WorldModelInfo,
};

#[derive(Debug, Clone, PartialEq, Eq)]
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

    /// Creates a shallow clone of the `FileInfo`, without cloning the `data_info`.
    /// This is useful when you want to create a new `FileInfo` with the same `path` and `archive_path`,
    /// but without loading the data again.
    pub fn shallow_clone(&self) -> Self {
        Self {
            path: self.path.clone(),
            archive_path: self.archive_path.clone(),
            state: self.state.clone(),
            data_info: None,
        }
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

    pub fn set_texture(&mut self, texture: TextureInfo) {
        self.data_info = Some(DataInfo::Texture(texture));
        self.state = FileInfoState::Loaded;
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
    map: HashMap<String, FileInfo>,
}

impl FileInfoMap {
    pub fn insert(&mut self, file_info: FileInfo) {
        self.map.insert(file_info.path.to_lowercase(), file_info);
    }

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

#[derive(Resource, Default)]
pub struct LoadFileTask {
    pub tasks: Vec<tasks::Task<Result<FileInfo>>>,
    completed: Vec<FileInfo>,
}

pub fn check_file_loading(
    mut load_task: ResMut<LoadFileTask>,
    mut file_info_map: ResMut<FileInfoMap>,
    mut images: ResMut<Assets<Image>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut commands: Commands,
) -> Result<()> {
    let mut tasks = Vec::new();
    tasks.append(&mut load_task.tasks);

    let mut new_tasks = Vec::new();

    for mut current_task in tasks {
        let poll_result = tasks::block_on(tasks::poll_once(&mut current_task));
        if let Some(result) = poll_result {
            match result {
                Err(err) => {
                    error!("Error loading file: {err}");
                }
                Ok(file) => {
                    if let FileInfoState::Error(_) = &file.state {
                        file_info_map.insert(file);
                        continue;
                    }
                    assert_eq!(file.state, FileInfoState::Loaded);
                    process_loaded_file(file, &mut file_info_map, &mut new_tasks, &mut load_task)?;
                }
            }
        } else {
            // Not ready yet, put it back
            load_task.tasks.push(current_task);
        }
    }

    load_task.tasks.extend(new_tasks);
    process_completed_tasks(
        &mut load_task,
        &mut file_info_map,
        &mut images,
        &mut materials,
        &mut meshes,
        &mut commands,
    )
}

fn process_loaded_file(
    mut file: FileInfo,
    file_info_map: &mut FileInfoMap,
    new_tasks: &mut Vec<tasks::Task<Result<FileInfo>>>,
    load_task: &mut LoadFileTask,
) -> Result<()> {
    match &file.data_info {
        Some(DataInfo::Model(model_info)) => {
            // At this point we have the model loaded, but textures may not be loaded yet.
            // We need to check the file info map for texture files and start loading them if necessary.
            for texture_path in model_info.get_texture_paths() {
                let texture_file_info = file_info_map.get_file_info_mut(&texture_path)?;
                if texture_file_info.state == FileInfoState::Unloaded {
                    // Start loading the texture
                    texture_file_info.state = FileInfoState::Loading;
                    let new_task = texture::loading_texture_task(texture_file_info);
                    new_tasks.push(new_task);
                }
            }

            // Put the current task back to be processed later
            load_task.completed.push(file);
        }
        Some(DataInfo::Texture(_)) => {
            // Texture loaded, update the file info map
            file_info_map.insert(file);
        }
        _ => {
            file.state = FileInfoState::Error("Loaded file type is not supported".to_string());
            file_info_map.insert(file);
        }
    }
    Ok(())
}

fn process_completed_tasks(
    load_task: &mut LoadFileTask,
    file_info_map: &mut FileInfoMap,
    images: &mut Assets<Image>,
    materials: &mut Assets<StandardMaterial>,
    meshes: &mut Assets<Mesh>,
    commands: &mut Commands,
) -> Result<()> {
    let mut completed_tasks = Vec::new();
    completed_tasks.append(&mut load_task.completed);

    for mut file in completed_tasks {
        match &file.data_info {
            Some(DataInfo::Model(model_info)) => {
                // At this point we have the model loaded, but textures may not be loaded yet.
                // We need to check the file info map to see whether the loading has completed.
                let textures_state =
                    check_files_state(&model_info.get_texture_paths(), file_info_map);

                match textures_state {
                    FileInfoState::Loaded => {
                        // All textures are loaded, we can create the meshes
                        let bundles = model::create_meshes_from_model_info(
                            model_info,
                            file_info_map,
                            images,
                            materials,
                            meshes,
                        )?;

                        if bundles.is_empty() {
                            file.state = FileInfoState::Error("No meshes".to_string());
                        } else {
                            for bundle in bundles {
                                add_bundle(commands, bundle, &file.path);
                            }
                            info!("Added meshes from {}", file.path);
                            file.state = FileInfoState::Loaded;
                        }
                        // Update the file archive map
                        file_info_map.insert(file);
                    }
                    FileInfoState::Error(_) => {
                        file.state = textures_state.clone();
                        file_info_map.insert(file);
                    }
                    _ => {
                        // Put this task back to be processed later
                        load_task.completed.push(file);
                    }
                }
            }
            _ => {
                file.state = FileInfoState::Error("No data".to_string());
                file_info_map.insert(file);
            }
        }
    }

    Ok(())
}

/// Returns the overall state of the files in `paths`.
/// If all files are `Loaded`, returns `Loaded`.
/// If any file is `Error`, returns `Error`.
/// If any file is `Loading`, returns `Loading`.
/// If any file is `Unloaded`, returns `Unloaded`.
/// If `paths` is empty, returns `Loaded`.
fn check_files_state(paths: &[String], file_info_map: &FileInfoMap) -> FileInfoState {
    let state = FileInfoState::Loaded;
    for path in paths {
        let file_info = match file_info_map.get_file_info(path) {
            Ok(info) => info,
            Err(e) => {
                return FileInfoState::Error(e.to_string());
            }
        };
        match &file_info.state {
            FileInfoState::Loaded => (), // continue checking
            _ => return file_info.state.clone(),
        }
    }
    state
}
