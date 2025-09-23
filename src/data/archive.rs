// Copyright © 2025
// Author: Nocthir <nocthir@proton.me>
// SPDX-License-Identifier: MIT or Apache-2.0

use std::collections::HashMap;
use std::path::Path;
use std::path::PathBuf;
use std::sync::OnceLock;

use bevy::prelude::*;
use bevy::tasks;
use wow_mpq as mpq;

use crate::data::archive;
use crate::data::file::FileInfoMap;
use crate::data::model;
use crate::data::world_map;
use crate::data::world_model;
use crate::settings;

pub static ARCHIVE_MAP: OnceLock<ArchiveMap> = OnceLock::new();

macro_rules! get_archive {
    ($path:expr) => {
        $crate::data::archive::ARCHIVE_MAP
            .get()
            .unwrap()
            .get_archive($path)
    };
}
pub(crate) use get_archive;

#[derive(Default, Resource)]
pub struct ArchiveMap {
    pub map: HashMap<PathBuf, mpq::Archive>,
}

impl ArchiveMap {
    pub fn get_archive<P: AsRef<Path>>(&self, path: P) -> Result<&mpq::Archive> {
        self.map
            .get(path.as_ref())
            .ok_or_else(|| format!("Archive not found in map: {}", path.as_ref().display()).into())
    }
}

#[derive(Default, Resource)]
pub struct ArchiveInfoMap {
    pub map: HashMap<PathBuf, ArchiveInfo>,
}

pub struct ArchiveInfo {
    pub path: PathBuf,
    pub texture_paths: Vec<String>,
    pub model_paths: Vec<String>,
    pub world_model_paths: Vec<String>,
    pub world_map_paths: Vec<String>,
}

impl ArchiveInfo {
    pub fn new<P: AsRef<Path>>(path: P) -> Result<Self> {
        let archive = archive::get_archive!(&path)?;
        let texture_paths = Self::get_texture_paths(archive)?;
        let model_paths = Self::get_model_paths(archive)?;
        let world_model_paths = Self::get_world_model_paths(archive)?;
        let world_map_paths = Self::get_world_map_paths(archive)?;
        Ok(Self {
            path: path.as_ref().into(),
            texture_paths,
            model_paths,
            world_model_paths,
            world_map_paths,
        })
    }

    fn get_texture_paths(archive: &mpq::Archive) -> Result<Vec<String>> {
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

    fn get_model_paths(archive: &mpq::Archive) -> Result<Vec<String>> {
        let mut models = Vec::new();
        archive.list()?.retain(|file| {
            if model::is_model_extension(&file.name) {
                models.push(file.name.clone());
                false
            } else {
                true
            }
        });
        Ok(models)
    }

    fn get_world_model_paths(archive: &mpq::Archive) -> Result<Vec<String>> {
        let mut world_models = Vec::new();
        archive.list()?.retain(|file| {
            // We only want the root .wmo files, not the group files
            if world_model::is_world_model_root_path(&file.name) {
                world_models.push(file.name.clone());
                false
            } else {
                true
            }
        });
        Ok(world_models)
    }

    fn get_world_map_paths(archive: &mpq::Archive) -> Result<Vec<String>> {
        let mut world_maps = Vec::new();
        archive.list()?.retain(|file| {
            if world_map::is_world_map_extension(&file.name) {
                world_maps.push(file.name.clone());
                false
            } else {
                true
            }
        });
        Ok(world_maps)
    }
}

pub fn init_archive_map() -> Result<()> {
    let mut value = ArchiveMap::default();
    let game_path = PathBuf::from(&settings::Settings::get().game_path);
    let data_path = game_path.join("Data");

    for file in data_path.read_dir()? {
        let file = file?;
        let file_path = file.path();
        if is_archive_extension(&file_path) {
            let archive = mpq::Archive::open(&file_path)?;
            value.map.insert(file_path, archive);
        }
    }

    ARCHIVE_MAP
        .set(value)
        .map_err(|_| "Failed to initialize ARCHIVE_MAP")?;

    Ok(())
}

#[derive(Resource, Default)]
pub struct LoadArchiveTasks {
    tasks: Vec<tasks::Task<Result<ArchiveInfo>>>,
}

pub fn start_loading(mut commands: Commands) {
    let mut tasks = LoadArchiveTasks::default();
    for archive_path in ARCHIVE_MAP.get().unwrap().map.keys() {
        let task = tasks::IoTaskPool::get().spawn(load_archive(archive_path.clone()));
        tasks.tasks.push(task);
    }
    commands.insert_resource(tasks);
}

pub fn is_archive_extension<P: AsRef<Path>>(path: P) -> bool {
    path.as_ref()
        .extension()
        .is_some_and(|ext| ext.to_string_lossy().eq_ignore_ascii_case("mpq"))
}

async fn load_archive(archive_path: PathBuf) -> Result<ArchiveInfo> {
    ArchiveInfo::new(archive_path)
}

pub fn check_archive_loading(
    mut exit: EventWriter<AppExit>,
    mut load_task: ResMut<LoadArchiveTasks>,
    mut file_info_map: ResMut<FileInfoMap>,
    mut archive_info_map: ResMut<ArchiveInfoMap>,
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
                    archive_info_map.map.insert(archive.path.clone(), archive);
                }
            }
        } else {
            // Not ready yet, put it back
            load_task.tasks.push(current_task);
        }
    }
    Ok(())
}
