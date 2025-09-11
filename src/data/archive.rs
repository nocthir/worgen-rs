// Copyright © 2025
// Author: Nocthir <nocthir@proton.me>
// SPDX-License-Identifier: MIT or Apache-2.0

use std::path::Path;
use std::path::PathBuf;

use bevy::prelude::*;
use bevy::tasks;
use wow_mpq as mpq;

use crate::data::model;
use crate::data::model::*;
use crate::data::world_model;
use crate::data::world_model::*;

#[derive(Clone)]
pub struct ArchiveInfo {
    pub path: PathBuf,
    pub model_infos: Vec<ModelInfo>,
    pub wmo_infos: Vec<WmoInfo>,
}

impl ArchiveInfo {
    pub fn new<P: AsRef<Path>>(
        path: P,
        model_infos: Vec<ModelInfo>,
        wmo_infos: Vec<WmoInfo>,
    ) -> Self {
        Self {
            path: path.as_ref().to_path_buf(),
            model_infos,
            wmo_infos,
        }
    }

    pub fn has_models(&self) -> bool {
        !self.model_infos.is_empty() || !self.wmo_infos.is_empty()
    }
}

#[derive(Event)]
pub struct ArchiveLoaded {
    pub archive: ArchiveInfo,
}

#[derive(Resource, Default)]
pub struct LoadArchiveTasks {
    tasks: Vec<tasks::Task<Result<ArchiveInfo>>>,
}

pub fn start_loading(mut commands: Commands) -> Result<()> {
    let game_path = PathBuf::from(std::env::var("GAME_PATH").unwrap_or_else(|_| ".".to_string()));
    let data_path = game_path.join("Data");

    let mut tasks = LoadArchiveTasks::default();

    for file in data_path.read_dir()? {
        let file = file?;
        if file.file_name().to_string_lossy().ends_with(".MPQ") {
            let mpq_path = file.path();
            let task = tasks::IoTaskPool::get().spawn(load_archive(mpq_path));
            tasks.tasks.push(task);
        }
    }

    commands.insert_resource(tasks);

    Ok(())
}

async fn load_archive(archive_path: PathBuf) -> Result<ArchiveInfo> {
    let mut archive = mpq::Archive::open(&archive_path)?;
    let model_infos = model::read_m2s(&mut archive)?;
    let wmo_infos = world_model::read_mwos(&mut archive)?;
    Ok(ArchiveInfo::new(archive_path, model_infos, wmo_infos))
}

pub fn check_archive_loading(
    mut exit: EventWriter<AppExit>,
    mut load_task: ResMut<LoadArchiveTasks>,
    mut event_writer: EventWriter<ArchiveLoaded>,
) {
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
                    info!("Loaded archive: {}", archive.path.display());
                    event_writer.write(ArchiveLoaded { archive });
                }
            }
        } else {
            // Not ready yet, put it back
            load_task.tasks.push(current_task);
        }
    }
}
