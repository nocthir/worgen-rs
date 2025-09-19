// Copyright © 2025
// Author: Nocthir <nocthir@proton.me>
// SPDX-License-Identifier: MIT or Apache-2.0

pub mod archive;
pub mod file;
pub mod model;
pub mod texture;
pub mod world_map;
pub mod world_model;

use std::f32;

use bevy::prelude::*;

use crate::{
    data::{
        archive::{ArchiveInfo, LoadArchiveTasks},
        file::{DataInfo, FileInfo, FileInfoMap},
    },
    ui::FileSelected,
};

pub struct DataPlugin;

impl Plugin for DataPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(FileInfoMap::default())
            .insert_resource(file::LoadFileTask::default())
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

#[derive(Bundle, Clone)]
pub struct ModelBundle {
    pub mesh: Mesh3d,
    pub material: MeshMaterial3d<StandardMaterial>,
    pub transform: Transform,
}

fn load_selected_file(
    mut event_reader: EventReader<FileSelected>,
    current_query: Query<&CurrentFile>,
    entity_query: Query<Entity, With<CurrentFile>>,
    mut file_info_map: ResMut<FileInfoMap>,
    mut load_file_tasks: ResMut<file::LoadFileTask>,
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
            model::start_loading_model(&mut load_file_tasks, file_info);
        }
    }
    Ok(())
}

fn create_mesh_from_selected_file(
    file_info: &FileSelected,
    file_info_map: &FileInfoMap,
    images: &mut Assets<Image>,
    materials: &mut Assets<StandardMaterial>,
    meshes: &mut Assets<Mesh>,
) -> Result<Vec<ModelBundle>> {
    let file_info = file_info_map.get_file_info(&file_info.file_path)?;
    create_mesh_from_file_info(file_info, file_info_map, images, materials, meshes)
}

fn create_mesh_from_file_info(
    file_info: &FileInfo,
    file_info_map: &FileInfoMap,
    images: &mut Assets<Image>,
    materials: &mut Assets<StandardMaterial>,
    meshes: &mut Assets<Mesh>,
) -> Result<Vec<ModelBundle>> {
    match &file_info.data_info {
        Some(DataInfo::Model(model_info)) => model::create_meshes_from_model_info(
            model_info,
            file_info_map,
            images,
            materials,
            meshes,
        ),
        Some(DataInfo::WorldModel(world_model_info)) => {
            world_model::create_meshes_from_world_model_info(
                world_model_info,
                file_info_map,
                images,
                materials,
                meshes,
            )
        }
        Some(DataInfo::WorldMap(world_map_info)) => world_map::create_meshes_from_world_map_info(
            world_map_info,
            file_info_map,
            images,
            materials,
            meshes,
        ),
        _ => Err(format!(
            "Unsupported or missing data info for file: {}",
            file_info.path
        )
        .into()),
    }
}

fn normalize_vec3(v: [f32; 3]) -> [f32; 3] {
    let len = (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt();
    if len > 0.0 {
        [v[0] / len, v[1] / len, v[2] / len]
    } else {
        v
    }
}

pub fn add_bundle<S: Into<String>>(commands: &mut Commands, mut bundle: ModelBundle, path: S) {
    bundle.transform.rotate_local_x(-f32::consts::FRAC_PI_2);
    bundle.transform.rotate_local_z(-f32::consts::FRAC_PI_2);
    commands.spawn((CurrentFile { path: path.into() }, bundle));
}
