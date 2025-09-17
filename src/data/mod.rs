// Copyright © 2025
// Author: Nocthir <nocthir@proton.me>
// SPDX-License-Identifier: MIT or Apache-2.0

pub mod archive;
pub mod model;
pub mod texture;
pub mod world_map;
pub mod world_model;

use std::f32;

use bevy::prelude::*;

use crate::{
    data::archive::{ArchiveInfo, ArchiveLoaded, DataInfo, FileInfo, LoadArchiveTasks},
    ui::FileSelected,
};

pub struct DataPlugin;

impl Plugin for DataPlugin {
    fn build(&self, app: &mut App) {
        app.add_event::<ArchiveLoaded>()
            .insert_resource(archive::FileInfoMap::default())
            .add_systems(Startup, archive::start_loading)
            .add_systems(
                Update,
                archive::check_archive_loading.run_if(resource_exists::<LoadArchiveTasks>),
            )
            .add_systems(Update, load_selected_model);
    }
}

#[derive(Component)]
pub struct CurrentModel;

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

fn load_selected_model(
    mut event_reader: EventReader<FileSelected>,
    query: Query<Entity, With<CurrentModel>>,
    mut commands: Commands,
    file_info_map: Res<archive::FileInfoMap>,
    mut images: ResMut<Assets<Image>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut standard_materials: ResMut<Assets<StandardMaterial>>,
) -> Result {
    // Ignore all but the last event
    if let Some(event) = event_reader.read().last() {
        match create_mesh_from_selected_file(
            event,
            &file_info_map,
            &mut images,
            &mut standard_materials,
            &mut meshes,
        ) {
            Ok(bundles) => {
                if bundles.is_empty() {
                    error!("No meshes loaded for file: {}", event.file_path);
                    return Ok(());
                }

                // Remove the previous model
                query.into_iter().for_each(|entity| {
                    commands.entity(entity).despawn();
                });

                for bundle in bundles {
                    add_bundle(&mut commands, bundle);
                }

                info!("Loaded model from {}", event.file_path);
            }
            Err(err) => {
                error!(
                    "Error loading model {} from archive {}: {err}",
                    event.file_path,
                    event.archive_path.display()
                );
            }
        }
    }
    Ok(())
}

fn create_mesh_from_selected_file(
    file_info: &FileSelected,
    file_info_map: &archive::FileInfoMap,
    images: &mut Assets<Image>,
    materials: &mut Assets<StandardMaterial>,
    meshes: &mut Assets<Mesh>,
) -> Result<Vec<ModelBundle>> {
    let file_info = file_info_map.get_file_info(&file_info.file_path)?;
    create_mesh_from_file_info(file_info, file_info_map, images, materials, meshes)
}

fn create_mesh_from_file_info(
    file_info: &FileInfo,
    file_info_map: &archive::FileInfoMap,
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

fn add_bundle(commands: &mut Commands, mut bundle: ModelBundle) {
    bundle.transform.rotate_local_x(-f32::consts::FRAC_PI_2);
    bundle.transform.rotate_local_z(-f32::consts::FRAC_PI_2);
    commands.spawn((CurrentModel, bundle));
}
