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
    world_map::{self, WorldMapInfo},
    world_model::{self, WorldModelInfo},
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
    pub data_type: DataType,
    pub data_info: Option<DataInfo>,
}

impl FileInfo {
    pub fn new<S: Into<String>, P: AsRef<Path>>(path: S, archive_path: P) -> Self {
        let path = path.into();
        Self {
            path: path.clone(),
            archive_path: archive_path.as_ref().into(),
            state: FileInfoState::Unloaded,
            data_type: DataType::from(path),
            data_info: None,
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
            data_type: self.data_type.clone(),
            data_info: None,
        }
    }

    pub fn get_model(&self) -> Result<&ModelInfo> {
        if let Some(DataInfo::Model(model_info)) = &self.data_info {
            Ok(model_info)
        } else {
            Err(format!("File {} is not a model", self.path).into())
        }
    }

    pub fn get_world_model(&self) -> Result<&WorldModelInfo> {
        if let Some(DataInfo::WorldModel(world_model_info)) = &self.data_info {
            Ok(world_model_info)
        } else {
            Err(format!("File {} is not a world model", self.path).into())
        }
    }

    pub fn get_world_map(&self) -> Result<&WorldMapInfo> {
        if let Some(DataInfo::WorldMap(world_map_info)) = &self.data_info {
            Ok(world_map_info)
        } else {
            Err(format!("File {} is not a world map", self.path).into())
        }
    }

    pub fn set_texture(&mut self, texture: TextureInfo) {
        self.data_info = Some(DataInfo::Texture(texture));
        self.state = FileInfoState::Loaded;
    }

    pub fn set_model(&mut self, model: ModelInfo) {
        self.data_info = Some(DataInfo::Model(model));
        self.state = FileInfoState::Loaded;
    }

    pub fn set_world_model(&mut self, wmo: WorldModelInfo) {
        self.data_info = Some(DataInfo::WorldModel(wmo));
        self.state = FileInfoState::Loaded;
    }

    pub fn set_world_map(&mut self, world_map: WorldMapInfo) {
        self.data_info = Some(DataInfo::WorldMap(world_map));
        self.state = FileInfoState::Loaded;
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DataType {
    Texture,
    Model,
    WorldModel,
    WorldMap,
    Unknown,
}

impl<S: Into<String>> From<S> for DataType {
    fn from(file_path: S) -> Self {
        let lowercase = file_path.into().to_lowercase();
        if lowercase.ends_with(".blp") {
            DataType::Texture
        } else if lowercase.ends_with(".m2") {
            DataType::Model
        } else if lowercase.ends_with(".wmo") {
            DataType::WorldModel
        } else if lowercase.ends_with(".adt") {
            DataType::WorldMap
        } else {
            DataType::Unknown
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
        Some(DataInfo::WorldModel(world_model_info)) => {
            load_unloaded_textures(
                world_model_info.get_texture_paths(),
                file_info_map,
                new_tasks,
            )?;
            // Put the current task back to be processed later
            load_task.completed.push(file);
        }
        Some(DataInfo::Model(model_info)) => {
            load_unloaded_textures(&model_info.get_texture_paths(), file_info_map, new_tasks)?;
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

/// Checks the file info map for texture files and starts loading them if necessary.
fn load_unloaded_textures(
    texture_paths: &[String],
    file_info_map: &mut FileInfoMap,
    new_tasks: &mut Vec<tasks::Task<Result<FileInfo>>>,
) -> Result<()> {
    for texture_path in texture_paths {
        let texture_file_info = file_info_map.get_file_info_mut(texture_path)?;
        if texture_file_info.state == FileInfoState::Unloaded {
            // Start loading the texture
            texture_file_info.state = FileInfoState::Loading;
            let new_task = texture::loading_texture_task(texture_file_info);
            new_tasks.push(new_task);
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
        match file.data_type {
            DataType::Model => {
                check_loaded_model(
                    file,
                    file_info_map,
                    images,
                    materials,
                    meshes,
                    commands,
                    load_task,
                )?;
            }
            DataType::WorldModel => {
                check_loaded_world_model(
                    file,
                    file_info_map,
                    images,
                    materials,
                    meshes,
                    commands,
                    load_task,
                )?;
            }
            DataType::WorldMap => {
                check_loaded_world_map(
                    file,
                    file_info_map,
                    images,
                    materials,
                    meshes,
                    commands,
                    load_task,
                )?;
            }
            _ => {
                file.state = FileInfoState::Error("No data".to_string());
                file_info_map.insert(file);
            }
        }
    }

    Ok(())
}

fn check_loaded_model(
    mut file: FileInfo,
    file_info_map: &mut FileInfoMap,
    images: &mut Assets<Image>,
    materials: &mut Assets<StandardMaterial>,
    meshes: &mut Assets<Mesh>,
    commands: &mut Commands,
    load_task: &mut LoadFileTask,
) -> Result<()> {
    let model_info = file.get_model()?;

    // At this point we have the model loaded, but textures may not be loaded yet.
    // We need to check the file info map to see whether the loading has completed.
    let textures_state = check_files_state(&model_info.get_texture_paths(), file_info_map);

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

    Ok(())
}

fn check_loaded_world_model(
    mut file: FileInfo,
    file_info_map: &mut FileInfoMap,
    images: &mut Assets<Image>,
    materials: &mut Assets<StandardMaterial>,
    meshes: &mut Assets<Mesh>,
    commands: &mut Commands,
    load_task: &mut LoadFileTask,
) -> Result<()> {
    let world_model_info = file.get_world_model()?;

    // At this point we have the world model loaded, but textures may not be loaded yet.
    // We need to check the file info map to see whether the loading has completed.
    let textures_state = check_files_state(world_model_info.get_texture_paths(), file_info_map);

    match textures_state {
        FileInfoState::Loaded => {
            // All textures are loaded, we can create the meshes
            let bundles = world_model::create_meshes_from_world_model_info(
                world_model_info,
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

    Ok(())
}

fn check_loaded_world_map(
    mut file: FileInfo,
    file_info_map: &mut FileInfoMap,
    images: &mut Assets<Image>,
    materials: &mut Assets<StandardMaterial>,
    meshes: &mut Assets<Mesh>,
    commands: &mut Commands,
    load_task: &mut LoadFileTask,
) -> Result<()> {
    let world_map_info = file.get_world_map()?;

    // At this point we have the world map loaded, but models and textures may not be loaded yet.
    // We need to check the file info map to see whether the loading has completed.
    let models_state = check_files_state(&world_map_info.model_paths, file_info_map);

    match models_state {
        FileInfoState::Loaded => {
            // All models are loaded, we can create the meshes
            let bundles = world_map::create_meshes_from_world_map_info(
                world_map_info,
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
            file.state = models_state.clone();
            file_info_map.insert(file);
        }
        _ => {
            // Put this task back to be processed later
            load_task.completed.push(file);
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
