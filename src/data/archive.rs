// Copyright © 2025
// Author: Nocthir <nocthir@proton.me>
// SPDX-License-Identifier: MIT or Apache-2.0

use std::path::Path;
use std::path::PathBuf;

use bevy::prelude::*;
use bevy::tasks;
use wow_mpq as mpq;

use crate::data::DataInfo;
use crate::data::LoadArchivesTask;
use crate::data::model;
use crate::data::model::*;
use crate::data::world_model;
use crate::data::world_model::*;
use crate::state::GameState;

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

pub fn load_mpqs(
    mut exit: EventWriter<AppExit>,
    mut commands: Commands,
    mut task: ResMut<LoadArchivesTask>,
    mut next: ResMut<NextState<GameState>>,
) {
    if let Some(result) = tasks::block_on(tasks::poll_once(&mut task.task)) {
        match result {
            Err(err) => {
                error!("Error loading MPQs: {err}");
                exit.write(AppExit::error());
            }
            Ok(data_info) => {
                commands.insert_resource(data_info);
                // Once MPQs are loaded, transition from Loading to Main state
                next.set(GameState::Main);
            }
        }
        commands.remove_resource::<LoadArchivesTask>();
    }
}

pub async fn load_mpqs_impl() -> Result<DataInfo> {
    let mut data_info = DataInfo::default();

    let game_path = PathBuf::from(std::env::var("GAME_PATH").unwrap_or_else(|_| ".".to_string()));
    let data_path = game_path.join("Data");

    for file in data_path.read_dir()? {
        let file = file?;
        if file.file_name().to_string_lossy().ends_with(".MPQ") {
            let mpq_path = file.path();
            let mut archive = mpq::Archive::open(&mpq_path)?;
            let model_infos = model::read_m2s(&mut archive)?;
            let wmo_infos = world_model::read_mwos(&mut archive)?;
            let archive_info = ArchiveInfo::new(mpq_path, model_infos, wmo_infos);
            data_info.archives.push(archive_info);
        }
    }

    Ok(data_info)
}
