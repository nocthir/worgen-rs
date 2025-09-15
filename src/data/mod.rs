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

use wow_mpq as mpq;

use crate::{
    data::{
        archive::{ArchiveInfo, ArchiveLoaded, LoadArchiveTasks},
        texture::TextureArchiveMap,
    },
    ui::ModelSelected,
};

pub struct DataPlugin;

impl Plugin for DataPlugin {
    fn build(&self, app: &mut App) {
        app.add_event::<ArchiveLoaded>()
            .insert_resource(texture::TextureArchiveMap::default())
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
    mut event_reader: EventReader<ModelSelected>,
    query: Query<Entity, With<CurrentModel>>,
    mut commands: Commands,
    texture_archive_map: Res<TextureArchiveMap>,
    mut images: ResMut<Assets<Image>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut standard_materials: ResMut<Assets<StandardMaterial>>,
) -> Result {
    // Ignore all but the last event
    if let Some(event) = event_reader.read().last() {
        match create_mesh_from_selected_model(
            event,
            &texture_archive_map,
            &mut images,
            &mut standard_materials,
            &mut meshes,
        ) {
            Ok(bundles) => {
                if bundles.is_empty() {
                    error!("No meshes loaded for model: {}", event.model_path);
                    return Ok(());
                }

                // Remove the previous model
                query.into_iter().for_each(|entity| {
                    commands.entity(entity).despawn();
                });

                for (mesh, material) in bundles {
                    add_bundle(&mut commands, mesh, material);
                }

                info!("Loaded model from {}", event.model_path);
            }
            Err(err) => {
                error!(
                    "Error loading model {} from archive {}: {err}",
                    event.model_path, event.archive_path
                );
            }
        }
    }
    Ok(())
}

fn create_mesh_from_selected_model(
    model_info: &ModelSelected,
    texture_archive_map: &TextureArchiveMap,
    images: &mut Assets<Image>,
    materials: &mut Assets<StandardMaterial>,
    meshes: &mut Assets<Mesh>,
) -> Result<Vec<(Handle<Mesh>, Handle<StandardMaterial>)>> {
    let mpq_path = &model_info.archive_path;
    let mut archive = mpq::Archive::open(mpq_path)?;
    info!("Loaded archive {}", mpq_path);
    create_mesh_from_path_archive(
        &model_info.model_path,
        &mut archive,
        texture_archive_map,
        images,
        materials,
        meshes,
    )
}

fn create_mesh_from_path_archive(
    file_path: &str,
    archive: &mut mpq::Archive,
    texture_archive_map: &TextureArchiveMap,
    images: &mut Assets<Image>,
    materials: &mut Assets<StandardMaterial>,
    meshes: &mut Assets<Mesh>,
) -> Result<Vec<(Handle<Mesh>, Handle<StandardMaterial>)>> {
    if model::is_model_extension(file_path) {
        model::create_meshes_from_model_path(
            archive,
            file_path,
            texture_archive_map,
            images,
            materials,
            meshes,
        )
    } else if world_model::is_world_model_extension(file_path) {
        world_model::create_meshes_from_wmo_path(
            archive,
            file_path,
            texture_archive_map,
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

fn add_bundle(commands: &mut Commands, mesh: Handle<Mesh>, material: Handle<StandardMaterial>) {
    let mut transform = Transform::from_xyz(0.0, 0.0, 0.0);
    transform.rotate_local_x(-f32::consts::FRAC_PI_2);
    transform.rotate_local_z(-f32::consts::FRAC_PI_2);

    commands.spawn((
        CurrentModel,
        Mesh3d(mesh),
        MeshMaterial3d(material),
        transform,
    ));
}
