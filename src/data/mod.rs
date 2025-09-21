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
    data::{archive::*, file::*},
    ui::FileSelected,
};

pub struct DataPlugin;

impl Plugin for DataPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(FileInfoMap::default())
            .insert_resource(file::LoadingFileTasks::default())
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

#[derive(Default, Resource)]
pub struct ArchivesInfo {
    pub archives: Vec<ArchiveInfo>,
}

fn load_selected_file(
    mut event_reader: EventReader<FileSelected>,
    current_query: Query<&CurrentFile>,
    entity_query: Query<Entity, With<CurrentFile>>,
    mut file_info_map: ResMut<FileInfoMap>,
    mut load_file_tasks: ResMut<file::LoadingFileTasks>,
    mut commands: Commands,
) -> Result {
    // Ignore all but the last event
    if let Some(event) = event_reader.read().last() {
        for entity in entity_query.into_iter() {
            let current_file = current_query.get(entity)?;
            if current_file.path == event.file_path {
                return Ok(());
            }
            // Remove the previous model
            file_info_map.get_file_info_mut(&current_file.path)?.state =
                file::FileInfoState::Unloaded;
            commands.entity(entity).despawn();
        }

        let file_info = file_info_map.get_file_info_mut(&event.file_path)?;
        if file_info.state == file::FileInfoState::Unloaded {
            file_info.state = file::FileInfoState::Loading;
            match file_info.data_type {
                file::DataType::Model => {
                    load_file_tasks
                        .tasks
                        .push(model::loading_model_task(file::LoadFileTask::new(
                            file_info, true,
                        )));
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
