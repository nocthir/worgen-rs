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
    data::{
        archive::{ArchiveInfo, ArchiveLoaded, LoadArchiveTasks},
        texture::FileArchiveMap,
    },
    ui::FileSelected,
};

pub struct DataPlugin;

impl Plugin for DataPlugin {
    fn build(&self, app: &mut App) {
        app.add_event::<ArchiveLoaded>()
            .insert_resource(texture::FileArchiveMap::default())
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
pub struct DataInfo {
    pub archives: Vec<ArchiveInfo>,
}
fn load_selected_model(
    mut event_reader: EventReader<FileSelected>,
    query: Query<Entity, With<CurrentModel>>,
    mut commands: Commands,
    file_archive_map: Res<FileArchiveMap>,
    mut images: ResMut<Assets<Image>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut standard_materials: ResMut<Assets<StandardMaterial>>,
) -> Result {
    // Ignore all but the last event
    if let Some(event) = event_reader.read().last() {
        match create_mesh_from_selected_file(
            event,
            &file_archive_map,
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

                for (mesh, material, transform) in bundles {
                    add_bundle(&mut commands, mesh, material, transform);
                }

                info!("Loaded model from {}", event.file_path);
            }
            Err(err) => {
                error!(
                    "Error loading model {} from archive {}: {err}",
                    event.file_path, event.archive_path
                );
            }
        }
    }
    Ok(())
}

fn create_mesh_from_selected_file(
    file_info: &FileSelected,
    file_archive_map: &FileArchiveMap,
    images: &mut Assets<Image>,
    materials: &mut Assets<StandardMaterial>,
    meshes: &mut Assets<Mesh>,
) -> Result<Vec<(Handle<Mesh>, Handle<StandardMaterial>, Transform)>> {
    create_mesh_from_file_path(
        &file_info.file_path,
        file_archive_map,
        images,
        materials,
        meshes,
    )
}

fn create_mesh_from_file_path(
    file_path: &str,
    file_archive_map: &FileArchiveMap,
    images: &mut Assets<Image>,
    materials: &mut Assets<StandardMaterial>,
    meshes: &mut Assets<Mesh>,
) -> Result<Vec<(Handle<Mesh>, Handle<StandardMaterial>, Transform)>> {
    if model::is_model_extension(file_path) {
        model::create_meshes_from_model_path(file_path, file_archive_map, images, materials, meshes)
    } else if world_model::is_world_model_extension(file_path) {
        world_model::create_meshes_from_world_model_path(
            file_path,
            file_archive_map,
            images,
            materials,
            meshes,
        )
    } else if world_map::is_world_map_extension(file_path) {
        world_map::create_meshes_from_world_map_path(
            file_path,
            file_archive_map,
            images,
            materials,
            meshes,
        )
    } else {
        Err(format!("Unsupported model format: {}", file_path).into())
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

fn add_bundle(
    commands: &mut Commands,
    mesh: Handle<Mesh>,
    material: Handle<StandardMaterial>,
    mut transform: Transform,
) {
    transform.rotate_local_x(-f32::consts::FRAC_PI_2);
    transform.rotate_local_z(-f32::consts::FRAC_PI_2);

    commands.spawn((
        CurrentModel,
        Mesh3d(mesh),
        MeshMaterial3d(material),
        transform,
    ));
}
