// Copyright © 2025
// Author: Nocthir <nocthir@proton.me>
// SPDX-License-Identifier: MIT or Apache-2.0

use std::{io, path::Path};

use bevy::{prelude::*, tasks};
use wow_adt as adt;
use wow_mpq as mpq;

use crate::data::{archive, file};

#[derive(Clone)]
pub struct WorldMapInfo {
    pub world_map: adt::Adt,
    pub texture_paths: Vec<String>,
    pub model_paths: Vec<String>,
    pub world_model_paths: Vec<String>,
}

impl WorldMapInfo {
    pub fn new<P: AsRef<Path>>(file_path: &str, archive_path: P) -> Result<Self> {
        let archive = archive::get_archive!(archive_path)?;
        let world_map = read_world_map(file_path, &archive)?;
        Ok(Self::from_adt(world_map))
    }

    fn from_adt(mut world_map: adt::Adt) -> Self {
        Self::fix_model_extensions(&mut world_map);
        let texture_paths = Self::get_texture_paths(&world_map);
        let model_paths = Self::get_model_paths(&world_map);
        let world_model_paths = Self::get_world_model_paths(&world_map);
        Self {
            texture_paths,
            world_map,
            model_paths,
            world_model_paths,
        }
    }

    fn fix_model_extensions(world_map: &mut adt::Adt) {
        if let Some(mmdx) = &mut world_map.mmdx {
            for filename in &mut mmdx.filenames {
                if filename.ends_with(".mdx") {
                    filename.replace_range(filename.len() - 4..filename.len(), ".m2");
                }
            }
        }
    }

    fn get_texture_paths(world_map: &adt::Adt) -> Vec<String> {
        let mut textures = Vec::new();
        if let Some(mtex) = &world_map.mtex {
            textures.extend(mtex.filenames.iter().cloned());
        }
        textures
    }

    fn get_model_paths(world_map: &adt::Adt) -> Vec<String> {
        let mut models = Vec::new();
        if let Some(mmdx) = &world_map.mmdx {
            models.extend(
                mmdx.filenames
                    .iter()
                    .filter(|&f| f.ends_with(".m2"))
                    .cloned(),
            );
        }
        models
    }

    pub fn get_world_model_paths(world_map: &adt::Adt) -> Vec<String> {
        let mut world_models = Vec::new();
        if let Some(modf) = &world_map.modf
            && let Some(mwmo) = &world_map.mwmo
        {
            let filenames = &mwmo.filenames;
            for model in &modf.models {
                if let Some(filename) = filenames.get(model.name_id as usize) {
                    world_models.push(filename.clone());
                }
            }
        }
        world_models
    }

    pub fn get_dependencies(&self) -> Vec<String> {
        let mut deps = Vec::new();
        deps.extend(self.texture_paths.iter().cloned());
        deps.extend(self.model_paths.iter().cloned());
        deps.extend(self.world_model_paths.iter().cloned());
        deps
    }
}

pub fn read_world_map(path: &str, archive: &mpq::Archive) -> Result<adt::Adt> {
    let file = archive.read_file(path)?;
    let mut reader = io::Cursor::new(file);
    Ok(adt::Adt::from_reader(&mut reader)?)
}

pub fn is_world_map_extension(filename: &str) -> bool {
    let lower_filename = filename.to_lowercase();
    lower_filename.ends_with(".adt")
}

pub fn loading_world_map_task(task: file::LoadFileTask) -> tasks::Task<file::LoadFileTask> {
    info!("Starting to load world map: {}", task.file.path);
    tasks::IoTaskPool::get().spawn(load_world_map(task))
}

async fn load_world_map(mut task: file::LoadFileTask) -> file::LoadFileTask {
    match WorldMapInfo::new(&task.file.path, &task.file.archive_path) {
        Ok(world_map_info) => {
            task.file.set_world_map(world_map_info);
            info!("Loaded world map: {}", task.file.path);
        }
        Err(e) => {
            error!("Failed to load world map {}: {}", task.file.path, e);
            task.file.state = file::FileInfoState::Error(e.to_string());
        }
    }
    task
}
