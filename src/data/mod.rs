// Copyright © 2025
// Author: Nocthir <nocthir@proton.me>
// SPDX-License-Identifier: MIT or Apache-2.0

pub mod archive;
pub mod file;
pub mod material;
pub mod model;
pub mod texture;
pub mod world_map;
pub mod world_model;

use std::f32;

use bevy::{prelude::*, render::mesh::VertexAttributeValues};

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

// Actually used in tests
#[allow(unused)]
fn create_mesh_from_file_path(
    file_path: &str,
    file_info_map: &FileInfoMap,
    images: &mut Assets<Image>,
    materials: &mut Assets<StandardMaterial>,
    meshes: &mut Assets<Mesh>,
) -> Result<Vec<ModelBundle>> {
    let file_info = file_info_map.get_file_info(file_path)?;
    create_mesh_from_file_info(file_info, file_info_map, images, materials, meshes)
}

// Actually used in tests
#[allow(unused)]
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

#[derive(Debug, Clone, Copy)]
pub struct BoundingSphere {
    pub center: Vec3,
    pub radius: f32,
}

/// Compute a combined world-space bounding sphere (center, radius) for the given bundles.
/// Uses mesh positions and applies the same reorientation as `add_bundle` to match spawned transforms.
pub fn compute_bounding_sphere_from_bundles(
    bundles: &[ModelBundle],
    meshes: &Assets<Mesh>,
) -> Option<BoundingSphere> {
    let mut min = Vec3::splat(f32::INFINITY);
    let mut max = Vec3::splat(f32::NEG_INFINITY);

    for b in bundles {
        let Some(mesh) = meshes.get(&b.mesh.0) else {
            continue;
        };

        // Prepare final transform: user-provided transform + reorientation applied at spawn
        let mut final_transform = b.transform;
        final_transform.rotate_local_x(-f32::consts::FRAC_PI_2);
        final_transform.rotate_local_z(-f32::consts::FRAC_PI_2);

        // Extract positions
        if let Some(VertexAttributeValues::Float32x3(positions)) =
            mesh.attribute(Mesh::ATTRIBUTE_POSITION)
        {
            // Compute local AABB
            let mut local_min = Vec3::splat(f32::INFINITY);
            let mut local_max = Vec3::splat(f32::NEG_INFINITY);
            for p in positions {
                let v = Vec3::new(p[0], p[1], p[2]);
                local_min = local_min.min(v);
                local_max = local_max.max(v);
            }

            // Local center and radius (AABB-based sphere)
            let local_center = (local_min + local_max) * 0.5;
            let local_radius = (local_max - local_center).length();

            // Transform sphere to world: position by transform, scale by max scale component
            let world_center = final_transform.transform_point(local_center);
            let s = final_transform.scale.abs();
            let max_scale = s.x.max(s.y).max(s.z).max(1e-6);
            let world_radius = local_radius * max_scale;

            // Expand global AABB by the sphere
            min = min.min(world_center - Vec3::splat(world_radius));
            max = max.max(world_center + Vec3::splat(world_radius));
        }
    }

    if min.x.is_finite() && max.x.is_finite() {
        let center = (min + max) * 0.5;
        let radius = (max - center).length();
        Some(BoundingSphere { center, radius })
    } else {
        None
    }
}
