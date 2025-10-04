// Copyright © 2025
// Author: Nocthir <nocthir@proton.me>
// SPDX-License-Identifier: MIT or Apache-2.0

use std::collections::HashMap;
use std::path::Path;
use std::path::PathBuf;

use bevy::prelude::*;
use bevy::tasks;
use wow_mpq as mpq;

use crate::assets::*;

#[derive(Default, Resource, Reflect)]
pub struct ArchiveInfoMap {
    pub map: HashMap<PathBuf, ArchiveInfo>,
}

#[derive(Reflect)]
pub struct ArchiveInfo {
    pub path: PathBuf,
    pub texture_paths: Vec<String>,
    pub model_paths: Vec<String>,
    pub world_model_paths: Vec<String>,
    pub world_map_paths: Vec<String>,
}

impl ArchiveInfo {
    pub fn new<P: AsRef<Path>>(path: P) -> Result<Self> {
        let mut archive = mpq::Archive::open(&path)?;
        let texture_paths = Self::get_texture_paths(&mut archive)?;
        let model_paths = Self::get_model_paths(&mut archive)?;
        let world_model_paths = Self::get_world_model_paths(&mut archive)?;
        let world_map_paths = Self::get_world_map_paths(&mut archive)?;
        Ok(Self {
            path: path.as_ref().into(),
            texture_paths,
            model_paths,
            world_model_paths,
            world_map_paths,
        })
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
            if model::is_model_extension(&file.name) {
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

    fn get_world_map_paths(archive: &mut mpq::Archive) -> Result<Vec<String>> {
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

#[derive(Resource, Default)]
pub struct LoadArchiveTasks {
    tasks: Vec<tasks::Task<Result<ArchiveInfo>>>,
}

pub fn start_loading(mut commands: Commands) -> Result<()> {
    let mut tasks = LoadArchiveTasks::default();
    for archive_path in archive::get_archive_paths()? {
        let task = tasks::IoTaskPool::get().spawn(load_archive(archive_path.clone()));
        tasks.tasks.push(task);
    }
    commands.insert_resource(tasks);
    Ok(())
}

async fn load_archive(archive_path: PathBuf) -> Result<ArchiveInfo> {
    ArchiveInfo::new(archive_path)
}

pub fn check_archive_loading(
    mut exit: MessageWriter<AppExit>,
    mut load_task: ResMut<LoadArchiveTasks>,
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
                Ok(archive) => {
                    info!("Loaded archive info: {}", archive.path.display());
                    // Update the file archive map
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
