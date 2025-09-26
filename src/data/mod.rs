// Copyright © 2025
// Author: Nocthir <nocthir@proton.me>
// SPDX-License-Identifier: MIT or Apache-2.0

pub mod archive;
pub mod file;

use bevy::prelude::*;

use crate::{assets::ModelAssetLabel, data::archive::*, ui};

pub struct DataPlugin;

impl Plugin for DataPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(archive::ArchiveInfoMap::default())
            .insert_resource(file::FileInfoMap::new().expect("Failed to create FileInfoMap"))
            .add_systems(Startup, (archive::start_loading, ui::select_default_model))
            .add_systems(
                Update,
                archive::check_archive_loading.run_if(resource_exists::<LoadArchiveTasks>),
            )
            .add_systems(Update, load_selected_file);
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
    mut event_reader: EventReader<ui::FileSelected>,
    current_query: Query<&CurrentFile>,
    entity_query: Query<Entity, With<CurrentFile>>,
    mut commands: Commands,
    mut asset_server: ResMut<AssetServer>,
    mut file_map: ResMut<file::FileInfoMap>,
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

            // Unload the previous file's asset
            file_map.get_file_mut(&current_file.path)?.unload();
        }

        file_map
            .get_file_mut(&event.file_path)?
            .load(&mut asset_server);
        let model = asset_server.load(ModelAssetLabel::Root.from_asset(event.get_asset_path()));
        commands.spawn((CurrentFile::new(event.file_path.clone()), SceneRoot(model)));
    }
    Ok(())
}
