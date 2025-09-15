// Copyright © 2025
// Author: Nocthir <nocthir@proton.me>
// SPDX-License-Identifier: MIT or Apache-2.0

use std::path::PathBuf;

use bevy::prelude::*;
use bevy::tasks;
use wow_mpq as mpq;

use crate::data::model;
use crate::data::model::*;
use crate::data::texture;
use crate::data::texture::TextureInfo;
use crate::data::world_map;
use crate::data::world_map::WorldMapInfo;
use crate::data::world_model;
use crate::data::world_model::*;
use crate::settings::Settings;

#[derive(Clone)]
pub struct ArchiveInfo {
    pub path: String,
    pub texture_infos: Vec<TextureInfo>,
    pub model_infos: Vec<ModelInfo>,
    pub wmo_infos: Vec<WmoInfo>,
    pub world_map_infos: Vec<WorldMapInfo>,
}

impl ArchiveInfo {
    pub fn new<S: Into<String>>(
        path: S,
        texture_infos: Vec<TextureInfo>,
        model_infos: Vec<ModelInfo>,
        wmo_infos: Vec<WmoInfo>,
        world_map_infos: Vec<WorldMapInfo>,
    ) -> Self {
        Self {
            path: path.into(),
            texture_infos,
            model_infos,
            wmo_infos,
            world_map_infos,
        }
    }

    pub fn has_stuff(&self) -> bool {
        !self.model_infos.is_empty()
            || !self.wmo_infos.is_empty()
            || !self.texture_infos.is_empty()
            || !self.world_map_infos.is_empty()
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
    let mut archive = mpq::Archive::open(&archive_path)?;
    let texture_infos = texture::read_textures(&mut archive)?;
    let model_infos = model::read_models(&mut archive)?;
    let wmo_infos = world_model::read_mwos(&mut archive)?;
    let world_map_infos = world_map::read_world_maps(&mut archive)?;
    Ok(ArchiveInfo::new(
        archive_path,
        texture_infos,
        model_infos,
        wmo_infos,
        world_map_infos,
    ))
}

pub fn check_archive_loading(
    mut exit: EventWriter<AppExit>,
    mut load_task: ResMut<LoadArchiveTasks>,
    mut event_writer: EventWriter<ArchiveLoaded>,
    mut texture_archive_map: ResMut<texture::TextureArchiveMap>,
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
                    info!("Loaded archive: {}", archive.path);

                    // Update the texture to archive map
                    for texture_info in &archive.texture_infos {
                        texture_archive_map
                            .map
                            .insert(texture_info.texture_path.clone(), archive.path.clone());
                    }

                    event_writer.write(ArchiveLoaded { archive });
                }
            }
        } else {
            // Not ready yet, put it back
            load_task.tasks.push(current_task);
        }
    }
}
