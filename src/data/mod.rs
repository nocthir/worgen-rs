// Copyright © 2025
// Author: Nocthir <nocthir@proton.me>
// SPDX-License-Identifier: MIT or Apache-2.0

pub mod archive;
pub mod bundle;
pub mod file;
pub mod model;
pub mod texture;
pub mod world_map;
pub mod world_model;

use std::f32;

use bevy::prelude::*;

use crate::{
    assets::ModelAssetLabel,
    data::{archive::*, file::*},
    settings,
    ui::{self, FileSelected},
};

pub struct DataPlugin;

impl Plugin for DataPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(archive::ArchiveInfoMap::default())
            .insert_resource(file::FileInfoMap::default())
            .insert_resource(file::LoadingFileTasks::default())
            .add_systems(
                PreStartup,
                (
                    settings::Settings::init,
                    archive::ArchiveMap::init,
                    file::FileArchiveMap::init,
                    ui::select_default_model,
                )
                    .chain(),
            )
            .add_systems(Startup, archive::start_loading)
            .add_systems(
                Update,
                archive::check_archive_loading.run_if(resource_exists::<LoadArchiveTasks>),
            )
            .add_systems(Update, (load_selected_file, file::check_file_loading));
    }
}

#[derive(Component)]
pub struct CurrentFile {
    path: String,
}

impl CurrentFile {
    pub fn new(path: String) -> Self {
        info!("Current file: {}", path);
        Self { path }
    }

    pub fn get_asset_path(&self) -> String {
        format!("archive://{}", self.path)
    }
}

fn load_selected_file(
    mut event_reader: EventReader<FileSelected>,
    current_query: Query<&CurrentFile>,
    entity_query: Query<Entity, With<CurrentFile>>,
    mut file_info_map: ResMut<FileInfoMap>,
    mut load_file_tasks: ResMut<file::LoadingFileTasks>,
    mut commands: Commands,
    asset_server: ResMut<AssetServer>,
) -> Result {
    // Ignore all but the last event
    if let Some(event) = event_reader.read().last() {
        for entity in entity_query.into_iter() {
            let current_file = current_query.get(entity)?;
            if current_file.path == event.file_path {
                return Ok(());
            }
            // Remove the previous model
            commands.entity(entity).despawn();
        }

        let model = asset_server.load(ModelAssetLabel::Model.from_asset(event.get_asset_path()));
        commands.spawn((CurrentFile::new(event.file_path.clone()), SceneRoot(model)));
        return Ok(());

        let file_info = file_info_map.get_file_info_mut(&event.file_path)?;
        if file_info.state == file::FileInfoState::Unloaded {
            file_info.state = file::FileInfoState::Loading;
            match file_info.data_type {
                file::DataType::Model => {
                    let model = asset_server.load(file_info.get_asset_path());
                    commands.spawn(SceneRoot(model));
                }
                file::DataType::WorldModel => {
                    load_file_tasks
                        .tasks
                        .push(world_model::loading_world_model_task(
                            file::LoadFileTask::new(file_info, true),
                        ));
                }
                file::DataType::WorldMap => {
                    load_file_tasks
                        .tasks
                        .push(world_map::loading_world_map_task(file::LoadFileTask::new(
                            file_info, true,
                        )));
                }
                file::DataType::Texture => {
                    // Textures are loaded as part of model/world model/world map loading
                }
                file::DataType::Unknown => {
                    return Err(format!("Unsupported file type: {}", file_info.path).into());
                }
            }
        }
    }
    Ok(())
}
