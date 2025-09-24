// Copyright © 2025
// Author: Nocthir <nocthir@proton.me>
// SPDX-License-Identifier: MIT or Apache-2.0

use std::{
    collections::HashMap,
    path::{Path, PathBuf},
    ptr::addr_of,
    sync::Once,
};

use bevy::{pbr::ExtendedMaterial, prelude::*, tasks};

use crate::data::*;
use crate::{camera::FocusCamera, material::TerrainMaterial};

pub static mut FILE_ARCHIVE_MAP: FileArchiveMap = FileArchiveMap::new();
static FILE_ARCHIVE_MAP_ONCE: Once = Once::new();

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
            data_type: self.data_type,
            data_info: None,
        }
    }

    pub fn get_model(&self) -> Result<&model::ModelInfo> {
        if let Some(DataInfo::Model(model_info)) = &self.data_info {
            Ok(model_info)
        } else {
            Err(format!("File {} is not a model", self.path).into())
        }
    }

    pub fn get_world_model(&self) -> Result<&world_model::WorldModelInfo> {
        if let Some(DataInfo::WorldModel(world_model_info)) = &self.data_info {
            Ok(world_model_info)
        } else {
            Err(format!("File {} is not a world model", self.path).into())
        }
    }

    pub fn get_world_map(&self) -> Result<&world_map::WorldMapInfo> {
        if let Some(DataInfo::WorldMap(world_map_info)) = &self.data_info {
            Ok(world_map_info)
        } else {
            Err(format!("File {} is not a world map", self.path).into())
        }
    }

    pub fn set_texture(&mut self, texture: texture::TextureInfo) {
        self.data_info = Some(DataInfo::Texture(Box::new(texture)));
        self.state = FileInfoState::Loaded;
    }

    pub fn set_model(&mut self, model: model::ModelInfo) {
        self.data_info = Some(DataInfo::Model(Box::new(model)));
        self.state = FileInfoState::Loaded;
    }

    pub fn set_world_model(&mut self, wmo: world_model::WorldModelInfo) {
        self.data_info = Some(DataInfo::WorldModel(Box::new(wmo)));
        self.state = FileInfoState::Loaded;
    }

    pub fn set_world_map(&mut self, world_map: world_map::WorldMapInfo) {
        self.data_info = Some(DataInfo::WorldMap(Box::new(world_map)));
        self.state = FileInfoState::Loaded;
    }

    #[allow(unused)]
    pub fn get_dependencies(&self) -> Vec<String> {
        if let Some(data_info) = &self.data_info {
            data_info.get_dependencies()
        } else {
            Vec::new()
        }
    }

    #[allow(unused)]
    pub fn load(&mut self) -> Result<()> {
        if self.state == FileInfoState::Unloaded {
            self.state = FileInfoState::Loading;
            match self.data_type {
                DataType::Texture => {
                    self.set_texture(texture::TextureInfo::new(self)?);
                }
                DataType::Model => {
                    self.set_model(model::ModelInfo::new(&self.path, &self.archive_path)?)
                }
                DataType::WorldModel => self.set_world_model(world_model::WorldModelInfo::new(
                    &self.path,
                    &self.archive_path,
                )?),
                DataType::WorldMap => self.set_world_map(world_map::WorldMapInfo::new(
                    &self.path,
                    &self.archive_path,
                )?),
                DataType::Unknown => unreachable!("Cannot load unknown file type"),
            }
            self.state = FileInfoState::Loaded;
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
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
    Texture(Box<texture::TextureInfo>),
    Model(Box<model::ModelInfo>),
    WorldModel(Box<world_model::WorldModelInfo>),
    WorldMap(Box<world_map::WorldMapInfo>),
}

impl DataInfo {
    pub fn get_dependencies(&self) -> Vec<String> {
        match self {
            DataInfo::Texture(texture) => texture.get_dependencies(),
            DataInfo::Model(model_info) => model_info.get_texture_paths(),
            DataInfo::WorldModel(world_model_info) => world_model_info.get_texture_paths(),
            DataInfo::WorldMap(world_map_info) => world_map_info.get_dependencies(),
        }
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

    pub fn get_texture_info(&self, file_path: &str) -> Result<&texture::TextureInfo> {
        let file_info = self.get_file_info(file_path)?;
        if let Some(DataInfo::Texture(texture_info)) = &file_info.data_info {
            Ok(texture_info)
        } else {
            Err(format!("Texture `{}` not found", file_path).into())
        }
    }

    pub fn get_model_info(&self, file_path: &str) -> Result<&model::ModelInfo> {
        let file_info = self.get_file_info(file_path)?;
        if let Some(DataInfo::Model(model_info)) = &file_info.data_info {
            Ok(model_info)
        } else {
            Err(format!("Model `{}` not found", file_path).into())
        }
    }

    pub fn get_world_model_info(&self, file_path: &str) -> Result<&world_model::WorldModelInfo> {
        let file_info = self.get_file_info(file_path)?;
        if let Some(DataInfo::WorldModel(wmo_info)) = &file_info.data_info {
            Ok(wmo_info)
        } else {
            Err(format!("World model `{}` not found", file_path).into())
        }
    }

    pub fn get_world_map_info(&self, file_path: &str) -> Result<&world_map::WorldMapInfo> {
        let file_info = self.get_file_info(file_path)?;
        if let Some(DataInfo::WorldMap(world_map_info)) = &file_info.data_info {
            Ok(world_map_info)
        } else {
            Err(format!("World map `{}` not found", file_path).into())
        }
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

    #[allow(unused)]
    pub fn load_file_and_dependencies(&mut self, file_path: &str) -> Result<()> {
        let file_info = self.get_file_info_mut(file_path)?;
        if file_info.state == FileInfoState::Unloaded {
            // Start loading the file
            file_info.load()?;
            assert_eq!(file_info.state, FileInfoState::Loaded);
            assert!(file_info.data_info.is_some());
            // Load dependencies
            let dependencies = file_info.get_dependencies();
            for dep in dependencies {
                self.load_file_and_dependencies(&dep)?;
            }
        }
        Ok(())
    }
}

pub struct LoadFileTask {
    pub file: FileInfo,
    /// Whether to instantiate the loaded file into the scene.
    instantiate: bool,
}

impl LoadFileTask {
    pub fn new(file: &FileInfo, instantiate: bool) -> Self {
        Self {
            file: file.shallow_clone(),
            instantiate,
        }
    }
}

#[derive(Resource, Default)]
pub struct LoadingFileTasks {
    pub tasks: Vec<tasks::Task<LoadFileTask>>,
    completed: Vec<LoadFileTask>,
}

pub fn check_file_loading(
    mut load_task: ResMut<LoadingFileTasks>,
    mut file_info_map: ResMut<FileInfoMap>,
    mut images: ResMut<Assets<Image>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut terrain_materials: ResMut<Assets<ExtendedMaterial<StandardMaterial, TerrainMaterial>>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut commands: Commands,
    mut focus_writer: EventWriter<FocusCamera>,
) -> Result<()> {
    let mut tasks = Vec::new();
    tasks.append(&mut load_task.tasks);

    let mut new_tasks = Vec::new();

    for mut current_task in tasks {
        let poll_result = tasks::block_on(tasks::poll_once(&mut current_task));
        if let Some(file_task) = poll_result {
            if let FileInfoState::Error(_) = &file_task.file.state {
                file_info_map.insert(file_task.file);
                continue;
            }
            assert_eq!(file_task.file.state, FileInfoState::Loaded);
            process_loaded_file(
                file_task,
                &mut file_info_map,
                &mut new_tasks,
                &mut load_task,
            )?;
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
        &mut terrain_materials,
        &mut materials,
        &mut meshes,
        &mut commands,
        &mut focus_writer,
    )
}

fn process_loaded_file(
    mut file_task: LoadFileTask,
    file_info_map: &mut FileInfoMap,
    new_tasks: &mut Vec<tasks::Task<LoadFileTask>>,
    load_task: &mut LoadingFileTasks,
) -> Result<()> {
    match &file_task.file.data_info {
        Some(DataInfo::WorldMap(world_map_info)) => {
            load_unloaded_textures(&world_map_info.texture_paths, file_info_map, new_tasks)?;
            load_unloaded_models(&world_map_info.model_paths, file_info_map, new_tasks)?;
            load_unloaded_world_models(
                &world_map_info.world_model_paths,
                file_info_map,
                new_tasks,
            )?;
            // Put the current task back to be processed later
            load_task.completed.push(file_task);
        }
        Some(DataInfo::WorldModel(world_model_info)) => {
            load_unloaded_textures(
                &world_model_info.get_texture_paths(),
                file_info_map,
                new_tasks,
            )?;
            // Put the current task back to be processed later
            load_task.completed.push(file_task);
        }
        Some(DataInfo::Model(model_info)) => {
            load_unloaded_textures(&model_info.get_texture_paths(), file_info_map, new_tasks)?;
            // Put the current task back to be processed later
            load_task.completed.push(file_task);
        }
        Some(DataInfo::Texture(_)) => {
            // Texture loaded, update the file info map
            file_info_map.insert(file_task.file);
        }
        _ => {
            file_task.file.state =
                FileInfoState::Error("Loaded file type is not supported".to_string());
            file_info_map.insert(file_task.file);
        }
    }
    Ok(())
}

/// Checks the file info map for texture files and starts loading them if necessary.
fn load_unloaded_textures(
    texture_paths: &[String],
    file_info_map: &mut FileInfoMap,
    new_tasks: &mut Vec<tasks::Task<LoadFileTask>>,
) -> Result<()> {
    for texture_path in texture_paths {
        let texture_file_info = file_info_map.get_file_info_mut(texture_path)?;
        if texture_file_info.state == FileInfoState::Unloaded {
            // Start loading the texture
            texture_file_info.state = FileInfoState::Loading;
            let new_task =
                texture::loading_texture_task(LoadFileTask::new(texture_file_info, false));
            new_tasks.push(new_task);
        }
    }
    Ok(())
}

/// Checks the file info map for model files and starts loading them if necessary.
fn load_unloaded_models(
    model_paths: &[String],
    file_info_map: &mut FileInfoMap,
    new_tasks: &mut Vec<tasks::Task<LoadFileTask>>,
) -> Result<()> {
    for model_path in model_paths {
        let model_file_info = file_info_map.get_file_info_mut(model_path)?;
        if model_file_info.state == FileInfoState::Unloaded {
            // Start loading the model
            model_file_info.state = FileInfoState::Loading;
            let new_task = model::loading_model_task(LoadFileTask::new(model_file_info, false));
            new_tasks.push(new_task);
        }
    }
    Ok(())
}

/// Checks the file info map for world model files and starts loading them if necessary.
fn load_unloaded_world_models(
    world_model_paths: &[String],
    file_info_map: &mut FileInfoMap,
    new_tasks: &mut Vec<tasks::Task<LoadFileTask>>,
) -> Result<()> {
    for world_model_path in world_model_paths {
        let world_model_file_info = file_info_map.get_file_info_mut(world_model_path)?;
        if world_model_file_info.state == FileInfoState::Unloaded {
            // Start loading the world model
            world_model_file_info.state = FileInfoState::Loading;
            let new_task = world_model::loading_world_model_task(LoadFileTask::new(
                world_model_file_info,
                false,
            ));
            new_tasks.push(new_task);
        }
    }
    Ok(())
}

fn process_completed_tasks(
    load_task: &mut LoadingFileTasks,
    file_info_map: &mut FileInfoMap,
    images: &mut Assets<Image>,
    terrain_materials: &mut Assets<ExtendedMaterial<StandardMaterial, TerrainMaterial>>,
    materials: &mut Assets<StandardMaterial>,
    meshes: &mut Assets<Mesh>,
    commands: &mut Commands,
    focus_writer: &mut EventWriter<FocusCamera>,
) -> Result<()> {
    let mut completed_tasks = Vec::new();
    completed_tasks.append(&mut load_task.completed);

    for mut task in completed_tasks {
        match task.file.data_type {
            DataType::Model => {
                check_loaded_model(
                    task,
                    file_info_map,
                    images,
                    materials,
                    meshes,
                    commands,
                    load_task,
                    focus_writer,
                )?;
            }
            DataType::WorldModel => {
                check_loaded_world_model(
                    task,
                    file_info_map,
                    images,
                    materials,
                    meshes,
                    commands,
                    load_task,
                    focus_writer,
                )?;
            }
            DataType::WorldMap => {
                check_loaded_world_map(
                    task,
                    file_info_map,
                    images,
                    terrain_materials,
                    materials,
                    meshes,
                    commands,
                    load_task,
                    focus_writer,
                )?;
            }
            _ => {
                task.file.state = FileInfoState::Error("No data".to_string());
                file_info_map.insert(task.file);
            }
        }
    }

    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn check_loaded_model(
    mut task: LoadFileTask,
    file_info_map: &mut FileInfoMap,
    images: &mut Assets<Image>,
    materials: &mut Assets<StandardMaterial>,
    meshes: &mut Assets<Mesh>,
    commands: &mut Commands,
    load_task: &mut LoadingFileTasks,
    focus_writer: &mut EventWriter<FocusCamera>,
) -> Result<()> {
    let model_info = task.file.get_model()?;

    // At this point we have the model loaded, but textures may not be loaded yet.
    // We need to check the file info map to see whether the loading has completed.
    let textures_state = check_files_state(model_info.get_texture_paths().iter(), file_info_map);

    match textures_state {
        FileInfoState::Loaded => {
            if task.instantiate {
                // All textures are loaded, we can create the meshes
                let bundles = bundle::create_meshes_from_model_info(
                    model_info,
                    file_info_map,
                    images,
                    materials,
                    meshes,
                )?;

                if bundles.is_empty() {
                    task.file.state = FileInfoState::Error("No meshes".to_string());
                } else {
                    // Compute bounds before spawning (bundles carry the final local transform)
                    if let Some(bounding_sphere) =
                        bundle::compute_bounding_sphere_from_bundles(&bundles, meshes)
                    {
                        focus_writer.write(FocusCamera { bounding_sphere });
                    }

                    for bundle in bundles {
                        bundle::add_bundle(commands, bundle, &task.file.path);
                    }
                    task.file.state = FileInfoState::Loaded;
                    info!("Added meshes from {}", task.file.path);
                }
            } else {
                task.file.state = FileInfoState::Loaded;
            }
            // Update the file archive map
            file_info_map.insert(task.file);
        }
        FileInfoState::Error(_) => {
            task.file.state = textures_state.clone();
            file_info_map.insert(task.file);
        }
        _ => {
            // Put this task back to be processed later
            load_task.completed.push(task);
        }
    }

    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn check_loaded_world_model(
    mut task: LoadFileTask,
    file_info_map: &mut FileInfoMap,
    images: &mut Assets<Image>,
    materials: &mut Assets<StandardMaterial>,
    meshes: &mut Assets<Mesh>,
    commands: &mut Commands,
    load_task: &mut LoadingFileTasks,
    focus_writer: &mut EventWriter<FocusCamera>,
) -> Result<()> {
    let world_model_info = task.file.get_world_model()?;

    // At this point we have the world model loaded, but textures may not be loaded yet.
    // We need to check the file info map to see whether the loading has completed.
    let textures_state =
        check_files_state(world_model_info.get_texture_paths().iter(), file_info_map);

    match textures_state {
        FileInfoState::Loaded => {
            if task.instantiate {
                // All textures are loaded, we can create the meshes
                let bundles = bundle::create_meshes_from_world_model_info(
                    world_model_info,
                    file_info_map,
                    images,
                    materials,
                    meshes,
                )?;

                if bundles.is_empty() {
                    task.file.state = FileInfoState::Error("No meshes".to_string());
                } else {
                    if let Some(bounding_sphere) =
                        bundle::compute_bounding_sphere_from_bundles(&bundles, meshes)
                    {
                        focus_writer.write(FocusCamera { bounding_sphere });
                    }

                    for bundle in bundles {
                        bundle::add_bundle(commands, bundle, &task.file.path);
                    }
                    info!("Added meshes from {}", task.file.path);
                    task.file.state = FileInfoState::Loaded;
                }
            } else {
                task.file.state = FileInfoState::Loaded;
            }
            // Update the file archive map
            file_info_map.insert(task.file);
        }
        FileInfoState::Error(_) => {
            task.file.state = textures_state.clone();
            file_info_map.insert(task.file);
        }
        _ => {
            // Put this task back to be processed later
            load_task.completed.push(task);
        }
    }

    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn check_loaded_world_map(
    mut task: LoadFileTask,
    file_info_map: &mut FileInfoMap,
    images: &mut Assets<Image>,
    terrain_materials: &mut Assets<ExtendedMaterial<StandardMaterial, TerrainMaterial>>,
    materials: &mut Assets<StandardMaterial>,
    meshes: &mut Assets<Mesh>,
    commands: &mut Commands,
    load_task: &mut LoadingFileTasks,
    focus_writer: &mut EventWriter<FocusCamera>,
) -> Result<()> {
    let world_map_info = task.file.get_world_map()?;

    // At this point we have the world map loaded, but models and textures may not be loaded yet.
    // We need to check the file info map to see whether the loading has completed.
    let dependency_paths = world_map_info.get_dependencies();

    let dependencies_state = check_files_state(dependency_paths.iter(), file_info_map);

    match dependencies_state {
        FileInfoState::Loaded => {
            if task.instantiate {
                // All models are loaded, we can create the meshes
                let bundles_result = bundle::create_meshes_from_world_map_info(
                    world_map_info,
                    file_info_map,
                    images,
                    terrain_materials,
                    materials,
                    meshes,
                );

                let (terrain_bundles, model_bundles) = match bundles_result {
                    Ok(bundles) => bundles,
                    Err(e) => {
                        task.file.state = FileInfoState::Error(e.to_string());
                        file_info_map.insert(task.file);
                        return Ok(());
                    }
                };

                if terrain_bundles.is_empty() && model_bundles.is_empty() {
                    task.file.state = FileInfoState::Error("No meshes".to_string());
                } else {
                    if let Some(bounding_sphere) =
                        bundle::compute_bounding_sphere_from_bundles(&model_bundles, meshes)
                    {
                        focus_writer.write(FocusCamera { bounding_sphere });
                    }

                    if model_bundles.is_empty()
                        && let Some(bounding_sphere) =
                            bundle::compute_bounding_sphere_from_bundles(&terrain_bundles, meshes)
                    {
                        focus_writer.write(FocusCamera { bounding_sphere });
                    }

                    for bundle in terrain_bundles {
                        bundle::add_bundle(commands, bundle, &task.file.path);
                    }
                    for bundle in model_bundles {
                        bundle::add_bundle(commands, bundle, &task.file.path);
                    }
                    info!("Added meshes from {}", task.file.path);
                    task.file.state = FileInfoState::Loaded;
                }
            } else {
                task.file.state = FileInfoState::Loaded;
            }
            // Update the file archive map
            file_info_map.insert(task.file);
        }
        FileInfoState::Error(_) => {
            task.file.state = dependencies_state.clone();
            file_info_map.insert(task.file);
        }
        _ => {
            // Put this task back to be processed later
            load_task.completed.push(task);
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
fn check_files_state<'s, I: Iterator<Item = &'s String>>(
    paths: I,
    file_info_map: &FileInfoMap,
) -> FileInfoState {
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
